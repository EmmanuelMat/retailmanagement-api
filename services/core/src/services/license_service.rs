//! Licencia de prueba (trial) y bloqueo post-vencimiento, sin depender de un
//! servidor central: todo vive en la fila del tenant en Postgres local.
//!
//! El campo `license_sig` es un HMAC-SHA256 sobre los demás campos, firmado
//! con `LICENSE_SECRET` (no vive en la fila ni en el binario en texto plano).
//! Editar la fila a mano por `psql` no produce una firma válida, así que
//! cualquier alteración directa se trata como intento de manipulación y se
//! fuerza el estado a `expired`. `last_seen_at` es un "trinquete": si el reloj
//! del sistema retrocede respecto al último valor visto, también se trata
//! como manipulación (reset de fecha para alargar la prueba).
//!
//! No es a prueba de un atacante determinado con acceso físico al binario -
//! sube el costo lo suficiente para frenar el caso realista (un negocio que
//! simplemente no paga), el mismo estándar de cualquier software de escritorio
//! on-prem.

use anyhow::{Context, Result};
use base64::Engine as _;
use chrono::{DateTime, Utc};
use openssl::hash::MessageDigest;
use openssl::pkey::PKey;
use openssl::sign::Signer;
use serde::Serialize;
use sqlx::PgPool;

pub struct LicenseService {
    pool: PgPool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EstadoLicencia {
    Trial,
    Active,
    Expired,
}

#[derive(Debug, Serialize)]
pub struct LicenseState {
    pub status: EstadoLicencia,
    pub dias_restantes: i64,
}

#[derive(Debug, Clone, sqlx::FromRow)]
struct TenantLicenseRow {
    rnc: String,
    license_status: String,
    trial_started_at: DateTime<Utc>,
    trial_days: i32,
    license_activated_at: Option<DateTime<Utc>>,
    last_seen_at: DateTime<Utc>,
    license_sig: String,
}

/// Clave HMAC. Requerida - sin ella, cualquiera podría forjar una fila de
/// licencia "activa" a mano en la base de datos. Sin fallback de desarrollo,
/// igual que `JWT_SECRET`/`VENDOR_ADMIN_SECRET`/`CERT_ENCRYPTION_KEY`.
fn license_secret() -> Vec<u8> {
    std::env::var("LICENSE_SECRET")
        .expect("LICENSE_SECRET debe estar configurado (ver .env.example)")
        .into_bytes()
}

fn compute_sig(row: &TenantLicenseRow) -> Result<String> {
    let payload = format!(
        "{}|{}|{}|{}|{}",
        row.rnc,
        row.trial_started_at.to_rfc3339(),
        row.trial_days,
        row.license_status,
        row.license_activated_at.map(|d| d.to_rfc3339()).unwrap_or_default(),
    );
    let key = PKey::hmac(&license_secret()).context("Clave de licencia inválida")?;
    let mut signer = Signer::new(MessageDigest::sha256(), &key).context("No se pudo inicializar la firma de licencia")?;
    signer.update(payload.as_bytes()).context("Fallo al firmar la licencia")?;
    let sig = signer.sign_to_vec().context("Fallo al firmar la licencia")?;
    Ok(base64::engine::general_purpose::STANDARD.encode(sig))
}

fn dias_restantes(row: &TenantLicenseRow, now: DateTime<Utc>) -> i64 {
    let transcurridos = now.signed_duration_since(row.trial_started_at).num_days();
    (row.trial_days as i64 - transcurridos).max(0)
}

impl LicenseService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn fetch(&self, tenant_id: &str) -> Result<TenantLicenseRow> {
        let row = sqlx::query_as::<_, TenantLicenseRow>(
            "SELECT rnc, license_status, trial_started_at, trial_days, license_activated_at, last_seen_at, license_sig
             FROM tenants WHERE rnc = $1",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Se llama en cada request autenticado (ver `license_guard` en main.rs).
    /// Barato: una lectura + una escritura de una sola fila.
    pub async fn check_and_update(&self, tenant_id: &str) -> Result<LicenseState> {
        let mut row = self.fetch(tenant_id).await?;
        let now = Utc::now();

        // Fila recién creada (DEFAULT '' de la migración) todavía no tiene
        // firma propia - no es manipulación, solo falta la primera firma.
        let expected_sig = compute_sig(&row)?;
        let ya_firmada = !row.license_sig.is_empty();
        let mut manipulada = ya_firmada && row.license_sig != expected_sig;

        if !manipulada && row.license_status != "active" && now < row.last_seen_at {
            manipulada = true; // reloj del sistema retrocedido
        }

        if manipulada {
            row.license_status = "expired".to_string();
        } else if row.license_status != "active" && dias_restantes(&row, now) <= 0 {
            row.license_status = "expired".to_string();
        }

        if row.last_seen_at < now {
            row.last_seen_at = now;
        }
        row.license_sig = compute_sig(&row)?;

        sqlx::query("UPDATE tenants SET license_status = $2, last_seen_at = $3, license_sig = $4 WHERE rnc = $1")
            .bind(&row.rnc)
            .bind(&row.license_status)
            .bind(row.last_seen_at)
            .bind(&row.license_sig)
            .execute(&self.pool)
            .await?;

        let status = match row.license_status.as_str() {
            "active" => EstadoLicencia::Active,
            "expired" => EstadoLicencia::Expired,
            _ => EstadoLicencia::Trial,
        };
        Ok(LicenseState { status, dias_restantes: dias_restantes(&row, now) })
    }

    /// El vendedor confirma un pago y activa la licencia manualmente. No es
    /// alcanzable con el JWT del ADMIN del propio tenant - ver el chequeo
    /// `X-Vendor-Secret` en `http_activar_licencia` (main.rs).
    pub async fn activate(&self, tenant_id: &str) -> Result<()> {
        let mut row = self.fetch(tenant_id).await?;
        let now = Utc::now();
        row.license_status = "active".to_string();
        row.license_activated_at = Some(now);
        if row.last_seen_at < now {
            row.last_seen_at = now;
        }
        let sig = compute_sig(&row)?;

        sqlx::query(
            "UPDATE tenants SET license_status = 'active', license_activated_at = $2, last_seen_at = $3, license_sig = $4 WHERE rnc = $1",
        )
        .bind(tenant_id)
        .bind(now)
        .bind(row.last_seen_at)
        .bind(&sig)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

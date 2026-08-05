//! Configuración DGII y Empresa - Módulo 11
//! Datos de empresa (sobre tenants, ya existente), Usuarios (CRUD sobre
//! usuarios, ya existente), Secuencias e-NCF (CRUD), Certificado P12
//! (subida cifrada AES-256-GCM en reposo) e Impresora (config + test TCP).

use anyhow::{Context, Result};
use argon2::{
    password_hash::{rand_core::OsRng as ArgonOsRng, PasswordHasher, SaltString},
    Argon2,
};
use base64::Engine as _;
use chrono::{DateTime, NaiveDate, Utc};
use openssl::symm::{decrypt_aead, encrypt_aead, Cipher};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Duration;
use uuid::Uuid;
use zeroize::Zeroize;

pub struct ConfigService {
    pool: PgPool,
}

// ------------------ Empresa ------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Empresa {
    pub rnc: String,
    pub razon_social: String,
    pub nombre_comercial: Option<String>,
    pub direccion: String,
    pub telefono: Option<String>,
    pub correo: Option<String>,
    pub logo_url: Option<String>,
    pub ambiente_dgii: String,
    pub factura_electronica_activa: bool,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmpresaRequest {
    pub razon_social: String,
    pub nombre_comercial: Option<String>,
    pub direccion: String,
    pub telefono: Option<String>,
    pub correo: Option<String>,
    pub logo_url: Option<String>,
    pub ambiente_dgii: String,
    pub factura_electronica_activa: bool,
}

// ------------------ Usuarios ------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Usuario {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre: String,
    pub email: String,
    pub rol: String,
    pub descuento_maximo_sin_aprobacion: rust_decimal::Decimal,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUsuarioRequest {
    pub nombre: String,
    pub email: String,
    pub password: String,
    pub rol: String,
    pub descuento_maximo_sin_aprobacion: Option<rust_decimal::Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUsuarioRequest {
    pub nombre: String,
    pub rol: String,
    pub activo: bool,
    pub descuento_maximo_sin_aprobacion: Option<rust_decimal::Decimal>,
    /// Si viene presente, el ADMIN está restableciendo la contraseña de este
    /// usuario (p.ej. un cajero que la olvidó) - se re-hashea igual que en
    /// `create_usuario`, nunca se guarda en texto plano.
    pub password: Option<String>,
}

// ------------------ Secuencias e-NCF ------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SecuenciaNcf {
    pub id: Uuid,
    pub tenant_id: String,
    pub tipo_ecf: i32,
    pub prefijo: String,
    pub desde: i64,
    pub hasta: i64,
    pub proximo: i64,
    pub fecha_vencimiento: NaiveDate,
    pub estado: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateSecuenciaRequest {
    pub tipo_ecf: i32,
    pub prefijo: String,
    pub desde: i64,
    pub hasta: i64,
    pub fecha_vencimiento: NaiveDate,
}

// ------------------ Certificado P12 ------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CertificadoStatus {
    pub id: Uuid,
    pub nombre_archivo: String,
    pub activo: bool,
    pub uploaded_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UploadCertificadoRequest {
    pub nombre_archivo: String,
    #[serde(rename = "p12Base64")]
    pub p12_base64: String,
    pub password: String,
}

// ------------------ Impresora ------------------

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ImpresoraConfig {
    pub tenant_id: String,
    pub ip: Option<String>,
    pub puerto: i32,
    pub ancho_mm: i32,
    pub copias: i32,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateImpresoraRequest {
    pub ip: Option<String>,
    pub puerto: i32,
    pub ancho_mm: i32,
    pub copias: i32,
}

#[derive(Debug, Serialize)]
pub struct TestImpresoraResult {
    pub alcanzable: bool,
    pub mensaje: String,
}

/// Clave simétrica para cifrar el P12/contraseña en reposo. Requerida - sin
/// fallback de desarrollo, ver `.env.example`. Sin esto cualquiera con acceso
/// a la base de datos podría intentar descifrar certificados DGII guardados
/// si además tuviera esta clave fija y pública en el código fuente.
fn encryption_key() -> Vec<u8> {
    let b64 = std::env::var("CERT_ENCRYPTION_KEY")
        .expect("CERT_ENCRYPTION_KEY debe estar configurado (ver .env.example) - 32 bytes en base64");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .expect("CERT_ENCRYPTION_KEY inválido - debe ser base64");
    assert_eq!(bytes.len(), 32, "CERT_ENCRYPTION_KEY debe decodificar a exactamente 32 bytes");
    bytes
}

fn aead_encrypt(plaintext: &[u8]) -> Result<(Vec<u8>, Vec<u8>)> {
    let mut key = encryption_key();
    let cipher = Cipher::aes_256_gcm();
    let mut nonce = vec![0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce);
    let mut tag = [0u8; 16];
    let mut ciphertext = encrypt_aead(cipher, &key, Some(&nonce), b"", plaintext, &mut tag)
        .context("Fallo al cifrar")?;
    ciphertext.extend_from_slice(&tag);
    key.zeroize();
    Ok((ciphertext, nonce))
}

fn aead_decrypt(ciphertext_and_tag: &[u8], nonce: &[u8]) -> Result<Vec<u8>> {
    let mut key = encryption_key();
    let cipher = Cipher::aes_256_gcm();
    if ciphertext_and_tag.len() < 16 {
        anyhow::bail!("Certificado cifrado corrupto");
    }
    let (ciphertext, tag) = ciphertext_and_tag.split_at(ciphertext_and_tag.len() - 16);
    let plaintext = decrypt_aead(cipher, &key, Some(nonce), b"", ciphertext, tag)
        .context("Fallo al descifrar - clave incorrecta o dato corrupto")?;
    key.zeroize();
    Ok(plaintext)
}

impl ConfigService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ---- Empresa ----

    pub async fn get_empresa(&self, tenant_id: &str) -> Result<Empresa> {
        let row = sqlx::query_as::<_, Empresa>(
            "SELECT rnc, razon_social, nombre_comercial, direccion, telefono, correo, logo_url, ambiente_dgii, factura_electronica_activa
             FROM tenants WHERE rnc = $1",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_empresa(&self, tenant_id: &str, req: UpdateEmpresaRequest) -> Result<Empresa> {
        let row = sqlx::query_as::<_, Empresa>(
            "UPDATE tenants SET razon_social=$2, nombre_comercial=$3, direccion=$4, telefono=$5,
                correo=$6, logo_url=$7, ambiente_dgii=$8, factura_electronica_activa=$9
             WHERE rnc=$1
             RETURNING rnc, razon_social, nombre_comercial, direccion, telefono, correo, logo_url, ambiente_dgii, factura_electronica_activa",
        )
        .bind(tenant_id)
        .bind(req.razon_social)
        .bind(req.nombre_comercial)
        .bind(req.direccion)
        .bind(req.telefono)
        .bind(req.correo)
        .bind(req.logo_url)
        .bind(req.ambiente_dgii)
        .bind(req.factura_electronica_activa)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    // ---- Usuarios ----

    const USUARIOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("nombre", "nombre"),
        ("rol", "rol"),
        ("created_at", "created_at"),
    ];

    pub async fn list_usuarios(
        &self,
        tenant_id: &str,
        search: Option<String>,
        rol: Option<String>,
        activo: Option<bool>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> Result<(Vec<Usuario>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::text IS NULL OR rol = $2)
               AND ($3::bool IS NULL OR activo = $3)
               AND ($4::text IS NULL OR LOWER(nombre) LIKE $4 OR LOWER(email) LIKE $4)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM usuarios {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(&rol)
            .bind(activo)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::USUARIOS_SORTABLE, "created_at ASC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tenant_id, nombre, email, rol, descuento_maximo_sin_aprobacion, activo, created_at
               FROM usuarios
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $5 OFFSET $6"#
        );
        let rows = sqlx::query_as::<_, Usuario>(&query)
            .bind(tenant_id)
            .bind(&rol)
            .bind(activo)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn create_usuario(&self, tenant_id: &str, req: CreateUsuarioRequest) -> Result<Usuario> {
        let salt = SaltString::generate(&mut ArgonOsRng);
        let hash = Argon2::default()
            .hash_password(req.password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Error al procesar contraseña: {}", e))?
            .to_string();

        let row = sqlx::query_as::<_, Usuario>(
            "INSERT INTO usuarios (tenant_id, nombre, email, password_hash, rol, descuento_maximo_sin_aprobacion)
             VALUES ($1, $2, $3, $4, $5, $6)
             RETURNING id, tenant_id, nombre, email, rol, descuento_maximo_sin_aprobacion, activo, created_at",
        )
        .bind(tenant_id)
        .bind(req.nombre)
        .bind(req.email)
        .bind(hash)
        .bind(req.rol)
        .bind(req.descuento_maximo_sin_aprobacion.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_usuario(&self, tenant_id: &str, id: Uuid, req: UpdateUsuarioRequest) -> Result<Usuario> {
        let existing = sqlx::query_as::<_, Usuario>(
            "SELECT id, tenant_id, nombre, email, rol, descuento_maximo_sin_aprobacion, activo, created_at
             FROM usuarios WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        let nuevo_hash = req.password.map(|p| {
            let salt = SaltString::generate(&mut ArgonOsRng);
            Argon2::default()
                .hash_password(p.as_bytes(), &salt)
                .map(|h| h.to_string())
                .map_err(|e| anyhow::anyhow!("Error al procesar contraseña: {}", e))
        }).transpose()?;

        let row = sqlx::query_as::<_, Usuario>(
            "UPDATE usuarios SET nombre=$3, rol=$4, activo=$5, descuento_maximo_sin_aprobacion=$6,
                password_hash = COALESCE($7, password_hash)
             WHERE id=$1 AND tenant_id=$2
             RETURNING id, tenant_id, nombre, email, rol, descuento_maximo_sin_aprobacion, activo, created_at",
        )
        .bind(id)
        .bind(tenant_id)
        .bind(req.nombre)
        .bind(req.rol)
        .bind(req.activo)
        .bind(req.descuento_maximo_sin_aprobacion.unwrap_or(existing.descuento_maximo_sin_aprobacion))
        .bind(nuevo_hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn deactivate_usuario(&self, tenant_id: &str, id: Uuid) -> Result<()> {
        sqlx::query("UPDATE usuarios SET activo = false WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Secuencias e-NCF ----

    pub async fn list_secuencias(&self, tenant_id: &str) -> Result<Vec<SecuenciaNcf>> {
        let rows = sqlx::query_as::<_, SecuenciaNcf>(
            "SELECT id, tenant_id, tipo_ecf, prefijo, desde, hasta, proximo, fecha_vencimiento, estado, created_at
             FROM secuencias_ncf WHERE tenant_id = $1 ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn create_secuencia(&self, tenant_id: &str, req: CreateSecuenciaRequest) -> Result<SecuenciaNcf> {
        if req.hasta < req.desde {
            anyhow::bail!("El rango 'hasta' debe ser mayor o igual a 'desde'");
        }
        let row = sqlx::query_as::<_, SecuenciaNcf>(
            "INSERT INTO secuencias_ncf (tenant_id, tipo_ecf, prefijo, desde, hasta, proximo, fecha_vencimiento, estado)
             VALUES ($1, $2, $3, $4, $5, $4, $6, 'ACTIVA')
             RETURNING id, tenant_id, tipo_ecf, prefijo, desde, hasta, proximo, fecha_vencimiento, estado, created_at",
        )
        .bind(tenant_id)
        .bind(req.tipo_ecf)
        .bind(req.prefijo)
        .bind(req.desde)
        .bind(req.hasta)
        .bind(req.fecha_vencimiento)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn set_secuencia_estado(&self, tenant_id: &str, id: Uuid, estado: &str) -> Result<SecuenciaNcf> {
        let row = sqlx::query_as::<_, SecuenciaNcf>(
            "UPDATE secuencias_ncf SET estado = $3 WHERE id = $1 AND tenant_id = $2
             RETURNING id, tenant_id, tipo_ecf, prefijo, desde, hasta, proximo, fecha_vencimiento, estado, created_at",
        )
        .bind(id)
        .bind(tenant_id)
        .bind(estado)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    // ---- Certificado P12 ----

    pub async fn upload_certificado(&self, tenant_id: &str, req: UploadCertificadoRequest) -> Result<CertificadoStatus> {
        let mut p12_der = base64::engine::general_purpose::STANDARD
            .decode(req.p12_base64.trim())
            .context("p12Base64 inválido")?;

        // Valida que el P12 y contraseña realmente abran antes de guardarlo.
        crate::services::ecfl_service::load_p12(&p12_der, &req.password)
            .context("El certificado o la contraseña no son válidos")?;

        let (cert_encrypted, cert_nonce) = aead_encrypt(&p12_der)?;
        p12_der.zeroize();
        let mut password_bytes = req.password.clone().into_bytes();
        let (password_encrypted, password_nonce) = aead_encrypt(&password_bytes)?;
        password_bytes.zeroize();

        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE certificados_dgii SET activo = false WHERE tenant_id = $1")
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;

        let row = sqlx::query_as::<_, CertificadoStatus>(
            "INSERT INTO certificados_dgii (tenant_id, nombre_archivo, cert_encrypted, cert_nonce, password_encrypted, password_nonce, activo)
             VALUES ($1, $2, $3, $4, $5, $6, true)
             RETURNING id, nombre_archivo, activo, uploaded_at",
        )
        .bind(tenant_id)
        .bind(req.nombre_archivo)
        .bind(cert_encrypted)
        .bind(cert_nonce)
        .bind(password_encrypted)
        .bind(password_nonce)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(row)
    }

    pub async fn certificado_status(&self, tenant_id: &str) -> Result<Option<CertificadoStatus>> {
        let row = sqlx::query_as::<_, CertificadoStatus>(
            "SELECT id, nombre_archivo, activo, uploaded_at FROM certificados_dgii
             WHERE tenant_id = $1 AND activo = true ORDER BY uploaded_at DESC LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Descifra y devuelve el P12 activo del tenant, usado internamente por el
    /// flujo de emisión de e-CF cuando el cliente no adjunta un P12 propio.
    pub async fn get_certificado_activo(&self, tenant_id: &str) -> Result<Option<(Vec<u8>, String)>> {
        let row = sqlx::query_as::<_, (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>)>(
            "SELECT cert_encrypted, cert_nonce, password_encrypted, password_nonce
             FROM certificados_dgii WHERE tenant_id = $1 AND activo = true ORDER BY uploaded_at DESC LIMIT 1",
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;

        let Some((cert_encrypted, cert_nonce, password_encrypted, password_nonce)) = row else {
            return Ok(None);
        };

        let p12_der = aead_decrypt(&cert_encrypted, &cert_nonce)?;
        let password_bytes = aead_decrypt(&password_encrypted, &password_nonce)?;
        let password = String::from_utf8(password_bytes).context("Contraseña de certificado corrupta")?;
        Ok(Some((p12_der, password)))
    }

    // ---- Impresora ----

    pub async fn get_impresora(&self, tenant_id: &str) -> Result<ImpresoraConfig> {
        let row = sqlx::query_as::<_, ImpresoraConfig>(
            "INSERT INTO impresora_config (tenant_id) VALUES ($1)
             ON CONFLICT (tenant_id) DO UPDATE SET tenant_id = EXCLUDED.tenant_id
             RETURNING tenant_id, ip, puerto, ancho_mm, copias, updated_at",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    pub async fn update_impresora(&self, tenant_id: &str, req: UpdateImpresoraRequest) -> Result<ImpresoraConfig> {
        let row = sqlx::query_as::<_, ImpresoraConfig>(
            "INSERT INTO impresora_config (tenant_id, ip, puerto, ancho_mm, copias, updated_at)
             VALUES ($1, $2, $3, $4, $5, NOW())
             ON CONFLICT (tenant_id) DO UPDATE SET ip=$2, puerto=$3, ancho_mm=$4, copias=$5, updated_at=NOW()
             RETURNING tenant_id, ip, puerto, ancho_mm, copias, updated_at",
        )
        .bind(tenant_id)
        .bind(req.ip)
        .bind(req.puerto)
        .bind(req.ancho_mm)
        .bind(req.copias)
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }

    /// Envía bytes crudos (ESC/POS) a la impresora de red configurada del tenant,
    /// vía TCP directo al puerto de impresión (el protocolo estándar "raw 9100").
    pub async fn imprimir_bytes(&self, tenant_id: &str, bytes: &[u8]) -> Result<()> {
        use tokio::io::AsyncWriteExt;
        let config = self.get_impresora(tenant_id).await?;
        let ip = config.ip.filter(|s| !s.trim().is_empty())
            .context("No hay impresora configurada en Configuración → Impresora")?;
        let addr = format!("{}:{}", ip, config.puerto);
        let mut stream = tokio::time::timeout(Duration::from_secs(5), tokio::net::TcpStream::connect(&addr))
            .await
            .context("Tiempo de espera agotado conectando a la impresora")?
            .with_context(|| format!("No se pudo conectar a la impresora en {}", addr))?;
        stream.write_all(bytes).await.context("Fallo al enviar datos a la impresora")?;
        stream.flush().await.context("Fallo al vaciar el buffer hacia la impresora")?;
        Ok(())
    }

    pub async fn test_impresora(&self, tenant_id: &str) -> Result<TestImpresoraResult> {
        let config = self.get_impresora(tenant_id).await?;
        let Some(ip) = config.ip.filter(|s| !s.trim().is_empty()) else {
            return Ok(TestImpresoraResult { alcanzable: false, mensaje: "No hay IP configurada".to_string() });
        };
        let addr = format!("{}:{}", ip, config.puerto);
        let result = tokio::time::timeout(Duration::from_secs(3), tokio::net::TcpStream::connect(&addr)).await;
        match result {
            Ok(Ok(_)) => Ok(TestImpresoraResult { alcanzable: true, mensaje: format!("Conexión exitosa a {}", addr) }),
            Ok(Err(e)) => Ok(TestImpresoraResult { alcanzable: false, mensaje: format!("No se pudo conectar a {}: {}", addr, e) }),
            Err(_) => Ok(TestImpresoraResult { alcanzable: false, mensaje: format!("Tiempo de espera agotado conectando a {}", addr) }),
        }
    }
}

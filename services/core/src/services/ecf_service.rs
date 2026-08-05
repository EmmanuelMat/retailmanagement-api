//! Migración NCF (papel/bloques locales) -> e-CF real - Módulo 5/11 conjunto.
//!
//! Antes: el e-NCF se fabricaba con un timestamp local y no existía relación
//! con las secuencias autorizadas por DGII, ni almacenamiento del XML firmado.
//!
//! Ahora:
//! - `allocar_siguiente_ncf` asigna el próximo número de una secuencia ACTIVA
//!   (`secuencias_ncf`, ya registrada vía Oficina Virtual/Configuración DGII)
//!   de forma atómica (SELECT ... FOR UPDATE), nunca reutiliza ni permite
//!   override manual, y marca la secuencia VENCIDA/AGOTADA cuando corresponde.
//! - `registrar_documento` guarda el XML firmado completo + respuesta DGII en
//!   `ecf_documentos`, cumpliendo la retención de 10 años.
//! - Modo contingencia: si el envío en tiempo real a DGII falla (sin
//!   conexión, DGII caído), el documento se guarda como
//!   `CONTINGENCIA_PENDIENTE` y puede reintentarse después - la venta nunca
//!   se bloquea por falta de conectividad, pero tampoco se marca como
//!   ACEPTADA hasta que DGII realmente responda.

use anyhow::{Context, Result};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

/// Umbral legal: una Factura de Consumo (Tipo 32) sin RNC/Cédula del
/// comprador solo es válida por debajo de este monto.
pub const UMBRAL_RNC_OBLIGATORIO: Decimal = Decimal::from_parts(250_000, 0, 0, false, 0);

pub struct EcfService {
    pool: PgPool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct EcfDocumento {
    pub id: Uuid,
    pub referencia_tipo: String,
    pub referencia_id: Uuid,
    pub tipo_ecf: i32,
    pub e_ncf: String,
    pub estado_dgii: String,
    pub track_id: Option<String>,
    pub codigo_seguridad: Option<String>,
    pub qr_url: Option<String>,
    pub mensaje_dgii: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct NuevoDocumento<'a> {
    pub tenant_id: &'a str,
    pub referencia_tipo: &'a str, // VENTA | NOTA_CREDITO
    pub referencia_id: Uuid,
    pub tipo_ecf: i32,
    pub e_ncf: &'a str,
    pub xml_firmado: &'a str,
    pub estado_dgii: &'a str,
    pub track_id: Option<&'a str>,
    pub codigo_seguridad: &'a str,
    pub qr_url: &'a str,
    pub mensaje_dgii: Option<&'a str>,
}

/// Determina si, dado el tipo de e-CF y el total, el comprador debe tener
/// RNC/Cédula obligatoriamente (punto 2 de la migración: umbral RD$250,000
/// para Consumo; Crédito Fiscal siempre requiere RNC + dirección completa).
/// Función libre (no depende de la base de datos) para poder validar tanto
/// desde `ventas_service::create_venta` como desde el handler de emisión.
pub fn requiere_identificacion(tipo_ecf: i32, total: Decimal, rnc_cedula: Option<&str>, direccion: Option<&str>) -> Result<()> {
    let tiene_id = rnc_cedula.map(|r| !r.trim().is_empty() && r.trim() != "000000000").unwrap_or(false);
    if tipo_ecf == 31 {
        if !tiene_id {
            anyhow::bail!("La Factura de Crédito Fiscal (Tipo 31) requiere RNC del comprador");
        }
        let tiene_direccion = direccion.map(|d| !d.trim().is_empty()).unwrap_or(false);
        if !tiene_direccion {
            anyhow::bail!("La Factura de Crédito Fiscal (Tipo 31) requiere la dirección completa del comprador");
        }
    }
    if tipo_ecf == 32 && total >= UMBRAL_RNC_OBLIGATORIO && !tiene_id {
        anyhow::bail!(
            "Ventas de Consumo desde RD$250,000 requieren RNC o Cédula del comprador (venta actual: RD${})",
            total
        );
    }
    Ok(())
}

impl EcfService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Asigna atómicamente el siguiente e-NCF de la secuencia ACTIVA para
    /// `tipo_ecf`. Nunca reutiliza ni permite fijar el número manualmente -
    /// si la secuencia venció o se agotó, la marca y devuelve error para que
    /// el negocio registre un rango nuevo vía Configuración DGII.
    pub async fn allocar_siguiente_ncf(&self, tenant_id: &str, tipo_ecf: i32) -> Result<(String, NaiveDate)> {
        let mut tx = self.pool.begin().await?;

        let row: Option<(Uuid, String, i64, i64, NaiveDate)> = sqlx::query_as(
            "SELECT id, prefijo, proximo, hasta, fecha_vencimiento FROM secuencias_ncf
             WHERE tenant_id = $1 AND tipo_ecf = $2 AND estado = 'ACTIVA'
             ORDER BY fecha_vencimiento ASC LIMIT 1 FOR UPDATE",
        )
        .bind(tenant_id)
        .bind(tipo_ecf)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((id, prefijo, proximo, hasta, fecha_vencimiento)) = row else {
            anyhow::bail!(
                "No hay una secuencia ACTIVA para el e-CF tipo {} — regístrala en Configuración → DGII",
                tipo_ecf
            );
        };

        if fecha_vencimiento < chrono::Local::now().date_naive() {
            sqlx::query("UPDATE secuencias_ncf SET estado = 'VENCIDA' WHERE id = $1").bind(id).execute(&mut *tx).await?;
            tx.commit().await?;
            anyhow::bail!(
                "La secuencia del e-CF tipo {} venció el {} — registra un rango nuevo en Configuración → DGII",
                tipo_ecf, fecha_vencimiento
            );
        }
        if proximo > hasta {
            sqlx::query("UPDATE secuencias_ncf SET estado = 'AGOTADA' WHERE id = $1").bind(id).execute(&mut *tx).await?;
            tx.commit().await?;
            anyhow::bail!(
                "La secuencia del e-CF tipo {} está agotada — registra un rango nuevo en Configuración → DGII",
                tipo_ecf
            );
        }

        sqlx::query("UPDATE secuencias_ncf SET proximo = proximo + 1 WHERE id = $1").bind(id).execute(&mut *tx).await?;
        tx.commit().await?;

        let e_ncf = format!("{}{:010}", prefijo, proximo);
        Ok((e_ncf, fecha_vencimiento))
    }

    /// Guarda el XML firmado + respuesta DGII de un e-CF (Venta o Nota de
    /// Crédito) para cumplir la retención de 10 años exigida por DGII.
    pub async fn registrar_documento(&self, doc: NuevoDocumento<'_>) -> Result<Uuid> {
        let id: Uuid = sqlx::query_scalar(
            "INSERT INTO ecf_documentos (tenant_id, referencia_tipo, referencia_id, tipo_ecf, e_ncf, xml_firmado, estado_dgii, track_id, codigo_seguridad, qr_url, mensaje_dgii)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
             RETURNING id",
        )
        .bind(doc.tenant_id)
        .bind(doc.referencia_tipo)
        .bind(doc.referencia_id)
        .bind(doc.tipo_ecf)
        .bind(doc.e_ncf)
        .bind(doc.xml_firmado)
        .bind(doc.estado_dgii)
        .bind(doc.track_id)
        .bind(doc.codigo_seguridad)
        .bind(doc.qr_url)
        .bind(doc.mensaje_dgii)
        .fetch_one(&self.pool)
        .await
        .context("No se pudo registrar el documento e-CF")?;
        Ok(id)
    }

    pub async fn actualizar_estado(&self, id: Uuid, estado_dgii: &str, track_id: Option<&str>, mensaje_dgii: Option<&str>) -> Result<()> {
        sqlx::query("UPDATE ecf_documentos SET estado_dgii = $2, track_id = $3, mensaje_dgii = $4 WHERE id = $1")
            .bind(id)
            .bind(estado_dgii)
            .bind(track_id)
            .bind(mensaje_dgii)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    const DOCUMENTOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "created_at"),
        ("tipo_ecf", "tipo_ecf"),
    ];

    pub async fn list_documentos(
        &self,
        tenant_id: &str,
        estado_dgii: Option<String>,
        tipo_ecf: Option<i32>,
        referencia_tipo: Option<String>,
        search: Option<String>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> Result<(Vec<EcfDocumento>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::text IS NULL OR estado_dgii = $2)
               AND ($3::int IS NULL OR tipo_ecf = $3)
               AND ($4::text IS NULL OR referencia_tipo = $4)
               AND ($5::text IS NULL OR LOWER(e_ncf) LIKE $5)
               AND ($6::date IS NULL OR created_at::date >= $6)
               AND ($7::date IS NULL OR created_at::date <= $7)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM ecf_documentos {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(&estado_dgii)
            .bind(tipo_ecf)
            .bind(&referencia_tipo)
            .bind(&pattern)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::DOCUMENTOS_SORTABLE, "created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, referencia_tipo, referencia_id, tipo_ecf, e_ncf, estado_dgii, track_id, codigo_seguridad, qr_url, mensaje_dgii, created_at
               FROM ecf_documentos
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $8 OFFSET $9"#
        );
        let rows = sqlx::query_as::<_, EcfDocumento>(&query)
            .bind(tenant_id)
            .bind(&estado_dgii)
            .bind(tipo_ecf)
            .bind(&referencia_tipo)
            .bind(&pattern)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn list_pendientes_contingencia(&self, tenant_id: &str) -> Result<Vec<EcfDocumento>> {
        let rows = sqlx::query_as::<_, EcfDocumento>(
            "SELECT id, referencia_tipo, referencia_id, tipo_ecf, e_ncf, estado_dgii, track_id, codigo_seguridad, qr_url, mensaje_dgii, created_at
             FROM ecf_documentos WHERE tenant_id = $1 AND estado_dgii = 'CONTINGENCIA_PENDIENTE' ORDER BY created_at ASC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_xml(&self, tenant_id: &str, id: Uuid) -> Result<Option<String>> {
        let xml: Option<String> = sqlx::query_scalar("SELECT xml_firmado FROM ecf_documentos WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(xml)
    }
}

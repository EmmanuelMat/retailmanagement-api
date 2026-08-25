//! Bitácora de auditoría: quién hizo qué. Solo se registran las mutaciones
//! de mayor impacto (ventas/descuentos, inventario, usuarios, nómina,
//! licencia) desde sus respectivos servicios - no cada lectura ni cada
//! mutación rutinaria, para no ahogar el log en ruido.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

pub struct AuditService {
    pool: PgPool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AuditoriaEntry {
    pub id: Uuid,
    pub usuario_id: Option<Uuid>,
    pub accion: String,
    pub entidad: String,
    pub entidad_id: Option<Uuid>,
    pub detalle: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

impl AuditService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Best-effort: un fallo al auditar no debe tumbar la operación de
    /// negocio que la disparó, solo se registra en logs.
    pub async fn log(
        &self,
        tenant_id: &str,
        usuario_id: Option<Uuid>,
        accion: &str,
        entidad: &str,
        entidad_id: Option<Uuid>,
        detalle: serde_json::Value,
    ) {
        let result = sqlx::query(
            "INSERT INTO auditoria (tenant_id, usuario_id, accion, entidad, entidad_id, detalle)
             VALUES ($1, $2, $3, $4, $5, $6)",
        )
        .bind(tenant_id)
        .bind(usuario_id)
        .bind(accion)
        .bind(entidad)
        .bind(entidad_id)
        .bind(detalle)
        .execute(&self.pool)
        .await;

        if let Err(e) = result {
            tracing::warn!("No se pudo registrar auditoría ({} {}): {}", accion, entidad, e);
        }
    }

    const AUDITORIA_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "created_at"),
        ("accion", "accion"),
        ("entidad", "entidad"),
    ];

    #[allow(clippy::too_many_arguments)]
    pub async fn list(
        &self,
        tenant_id: &str,
        usuario_id: Option<Uuid>,
        accion: Option<String>,
        entidad: Option<String>,
        entidad_id: Option<Uuid>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> Result<(Vec<AuditoriaEntry>, i64)> {
        let accion_pattern = accion
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", s));

        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::uuid IS NULL OR usuario_id = $2)
               AND ($3::text IS NULL OR LOWER(accion) LIKE $3)
               AND ($4::text IS NULL OR entidad = $4)
               AND ($5::date IS NULL OR created_at::date >= $5)
               AND ($6::date IS NULL OR created_at::date <= $6)
               AND ($7::uuid IS NULL OR entidad_id = $7)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM auditoria {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(usuario_id)
            .bind(&accion_pattern)
            .bind(&entidad)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(entidad_id)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::AUDITORIA_SORTABLE, "created_at DESC");
        let limit = page.limit(10);
        let query = format!(
            "SELECT id, usuario_id, accion, entidad, entidad_id, detalle, created_at
             FROM auditoria {WHERE_CLAUSE} ORDER BY {order_by} LIMIT $8 OFFSET $9"
        );
        let rows = sqlx::query_as::<_, AuditoriaEntry>(&query)
            .bind(tenant_id)
            .bind(usuario_id)
            .bind(&accion_pattern)
            .bind(&entidad)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(entidad_id)
            .bind(limit)
            .bind(page.offset(10))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }
}

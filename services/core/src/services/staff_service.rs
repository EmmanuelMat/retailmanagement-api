//! Staff service - Módulo 15: panel interno del equipo de ventas.
//! Todo lo que expone se alcanza solo vía rutas gateadas por `X-Vendor-Secret`
//! (ver `staff_guard` en main.rs) - nunca con el JWT de un tenant. Un ADMIN
//! de tenant no puede llegar a nada de aquí con su propio login.
//!
//! El catálogo de módulos (`modulos_catalogo`) es editable desde el sitio de
//! staff sin necesitar una migración - pero conectar un módulo nuevo a rutas
//! reales todavía requiere un cambio de código en `required_modulo`
//! (main.rs), igual que cualquier ruta nueva con `required_roles`.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TenantResumen {
    pub rnc: String,
    pub razon_social: String,
    pub correo: Option<String>,
    pub telefono: Option<String>,
    pub license_status: String,
    pub trial_started_at: DateTime<Utc>,
    pub trial_days: i32,
    pub license_activated_at: Option<DateTime<Utc>>,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ModuloAsignado {
    pub codigo: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub activo: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ModuloCatalogo {
    pub codigo: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub orden: i32,
    pub activo: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateModuloRequest {
    pub codigo: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub orden: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct SetModulosRequest {
    pub codigos: Vec<String>,
}

pub struct StaffService {
    pool: PgPool,
}

impl StaffService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_tenants(&self) -> Result<Vec<TenantResumen>> {
        let rows = sqlx::query_as::<_, TenantResumen>(
            r#"SELECT rnc, razon_social, correo, telefono, license_status, trial_started_at, trial_days,
                      license_activated_at, activo, created_at
               FROM tenants ORDER BY created_at DESC"#,
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_tenant(&self, rnc: &str) -> Result<Option<TenantResumen>> {
        let row = sqlx::query_as::<_, TenantResumen>(
            r#"SELECT rnc, razon_social, correo, telefono, license_status, trial_started_at, trial_days,
                      license_activated_at, activo, created_at
               FROM tenants WHERE rnc = $1"#,
        )
        .bind(rnc)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }

    /// Todos los módulos del catálogo (activos en el catálogo), marcando
    /// cuáles ya tiene este tenant - una sola fila por módulo, no dos listas
    /// separadas, para que la UI de staff pinte checkboxes directamente.
    pub async fn list_modulos_tenant(&self, rnc: &str) -> Result<Vec<ModuloAsignado>> {
        let rows: Vec<(String, String, Option<String>, bool)> = sqlx::query_as(
            r#"SELECT mc.codigo, mc.nombre, mc.descripcion, (tm.tenant_id IS NOT NULL) AS activo
               FROM modulos_catalogo mc
               LEFT JOIN tenant_modulos tm ON tm.modulo_codigo = mc.codigo AND tm.tenant_id = $1
               WHERE mc.activo = true
               ORDER BY mc.orden"#,
        )
        .bind(rnc)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(codigo, nombre, descripcion, activo)| ModuloAsignado { codigo, nombre, descripcion, activo })
            .collect())
    }

    /// Reemplaza el set completo de módulos asignados al tenant por `codigos`
    /// - refleja exactamente lo que el sales rep marcó en el formulario, no
    /// un merge incremental.
    pub async fn set_modulos_tenant(&self, rnc: &str, codigos: &[String], activado_por: Option<&str>) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM tenant_modulos WHERE tenant_id = $1").bind(rnc).execute(&mut *tx).await?;
        for codigo in codigos {
            sqlx::query(
                "INSERT INTO tenant_modulos (tenant_id, modulo_codigo, activado_por) VALUES ($1, $2, $3)",
            )
            .bind(rnc)
            .bind(codigo)
            .bind(activado_por)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn list_catalogo(&self) -> Result<Vec<ModuloCatalogo>> {
        let rows = sqlx::query_as::<_, ModuloCatalogo>(
            "SELECT codigo, nombre, descripcion, orden, activo FROM modulos_catalogo ORDER BY orden",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Agrega un módulo nuevo al catálogo - visible de inmediato en el
    /// formulario de asignación de cualquier tenant. No conecta rutas reales
    /// por sí solo (ver el comentario de módulo arriba).
    pub async fn create_modulo_catalogo(&self, req: CreateModuloRequest) -> Result<ModuloCatalogo> {
        let codigo = req.codigo.trim().to_uppercase().replace(' ', "_");
        if codigo.is_empty() || req.nombre.trim().is_empty() {
            anyhow::bail!("Código y nombre son requeridos");
        }
        let row = sqlx::query_as::<_, ModuloCatalogo>(
            r#"INSERT INTO modulos_catalogo (codigo, nombre, descripcion, orden)
               VALUES ($1, $2, $3, $4)
               RETURNING codigo, nombre, descripcion, orden, activo"#,
        )
        .bind(&codigo)
        .bind(req.nombre.trim())
        .bind(req.descripcion.as_deref().map(|s| s.trim()))
        .bind(req.orden.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?;
        Ok(row)
    }
}

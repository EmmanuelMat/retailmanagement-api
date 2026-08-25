//! Catálogo de permisos granular (roles / permisos_catalogo / role_permisos)
//! - Módulo 15d. Estrictamente ADITIVO: no reemplaza `role_guard`/
//! `required_roles` (main.rs) para ninguna ruta existente - `rol` (el string
//! en `usuarios`) sigue siendo la fuente de verdad para todo lo de siempre.
//! Este servicio solo respalda el nuevo `permiso_guard`/`required_permiso`,
//! que gobierna exclusivamente las rutas nuevas de Órdenes de Servicio /
//! Órdenes de Compra. Ver Contexto en el plan de este módulo.

use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Role {
    pub id: Uuid,
    pub codigo: String,
    pub nombre: String,
    pub es_admin: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PermisoCatalogo {
    pub codigo: String,
    pub nombre: String,
    pub orden: i32,
    pub activo: bool,
}

pub struct RolesService {
    pool: PgPool,
}

impl RolesService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Usado por `permiso_guard`. `es_admin` siempre pasa (mismo escape
    /// hatch que `role_guard` ya usa para el string "ADMIN"). Un usuario sin
    /// `rol_id` todavía asignado (backfill pendiente o rol legado) no tiene
    /// ningún permiso nuevo - solo afecta rutas nuevas, nunca las existentes.
    pub async fn tiene_permiso(&self, rol_id: Option<Uuid>, permiso_codigo: &str) -> anyhow::Result<bool> {
        let Some(rol_id) = rol_id else { return Ok(false) };
        let es_admin: Option<bool> = sqlx::query_scalar("SELECT es_admin FROM roles WHERE id = $1")
            .bind(rol_id)
            .fetch_optional(&self.pool)
            .await?;
        if es_admin == Some(true) {
            return Ok(true);
        }
        let tiene: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM role_permisos WHERE role_id = $1 AND permiso_codigo = $2",
        )
        .bind(rol_id)
        .bind(permiso_codigo)
        .fetch_optional(&self.pool)
        .await?;
        Ok(tiene.is_some())
    }

    pub async fn list_roles(&self) -> anyhow::Result<Vec<Role>> {
        let rows = sqlx::query_as::<_, Role>("SELECT id, codigo, nombre, es_admin FROM roles ORDER BY codigo").fetch_all(&self.pool).await?;
        Ok(rows)
    }

    pub async fn list_permisos(&self) -> anyhow::Result<Vec<PermisoCatalogo>> {
        let rows = sqlx::query_as::<_, PermisoCatalogo>(
            "SELECT codigo, nombre, orden, activo FROM permisos_catalogo WHERE activo = true ORDER BY orden",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn permisos_de_rol(&self, role_id: Uuid) -> anyhow::Result<Vec<String>> {
        let rows: Vec<(String,)> = sqlx::query_as("SELECT permiso_codigo FROM role_permisos WHERE role_id = $1")
            .bind(role_id)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|(c,)| c).collect())
    }

    /// Reemplazo total (igual que `staff_service::set_modulos_tenant`): borra
    /// y re-inserta el set completo, no un merge incremental.
    pub async fn set_permisos_de_rol(&self, role_id: Uuid, codigos: &[String]) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM role_permisos WHERE role_id = $1").bind(role_id).execute(&mut *tx).await?;
        for codigo in codigos {
            sqlx::query("INSERT INTO role_permisos (role_id, permiso_codigo) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(role_id)
                .bind(codigo)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

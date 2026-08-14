//! Permisos y roles - Módulo 17: reemplaza `usuarios.rol` (4 valores fijos,
//! chequeados por un match hardcodeado en main.rs) como fuente real de
//! autorización. `usuarios.rol` se conserva como etiqueta de display y como
//! fallback de migración - ver `permission_guard` en main.rs.
//!
//! Catálogo GLOBAL (sin tenant_id), igual que `modulos_catalogo`: todos los
//! tenants comparten el mismo catálogo de roles, editable desde el sitio de
//! staff. Conectar un permiso nuevo a una ruta real todavía requiere un
//! cambio de código en `required_permiso` (main.rs), igual que cualquier
//! ruta nueva con `required_modulo`.

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct PermisoCatalogo {
    pub codigo: String,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub orden: i32,
    pub activo: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Rol {
    pub id: Uuid,
    pub codigo: String,
    pub nombre: String,
    pub es_admin: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RolConPermisos {
    #[serde(flatten)]
    pub rol: Rol,
    pub permisos: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRolRequest {
    pub codigo: String,
    pub nombre: String,
    pub permisos: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct SetPermisosRolRequest {
    pub permisos: Vec<String>,
}

pub struct RolesService {
    pool: PgPool,
}

impl RolesService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn list_permisos_catalogo(&self) -> Result<Vec<PermisoCatalogo>> {
        let rows = sqlx::query_as::<_, PermisoCatalogo>(
            "SELECT codigo, nombre, descripcion, orden, activo FROM permisos_catalogo WHERE activo = true ORDER BY orden",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn list_roles(&self) -> Result<Vec<RolConPermisos>> {
        let roles = sqlx::query_as::<_, Rol>("SELECT id, codigo, nombre, es_admin, created_at FROM roles ORDER BY codigo")
            .fetch_all(&self.pool)
            .await?;
        let mut resultado = Vec::with_capacity(roles.len());
        for rol in roles {
            let permisos: Vec<String> = sqlx::query_scalar("SELECT permiso_codigo FROM role_permisos WHERE role_id = $1")
                .bind(rol.id)
                .fetch_all(&self.pool)
                .await?;
            resultado.push(RolConPermisos { rol, permisos });
        }
        Ok(resultado)
    }

    /// Roles creados desde el sitio de staff son siempre `es_admin = false`
    /// - el bypass total queda reservado al rol ADMIN sembrado por la
    /// migración, no algo que se pueda otorgar desde la UI de roles.
    pub async fn create_rol(&self, req: CreateRolRequest) -> Result<RolConPermisos> {
        let codigo = req.codigo.trim().to_uppercase().replace(' ', "_");
        if codigo.is_empty() || req.nombre.trim().is_empty() {
            anyhow::bail!("Código y nombre son requeridos");
        }
        let mut tx = self.pool.begin().await?;
        let rol = sqlx::query_as::<_, Rol>(
            "INSERT INTO roles (codigo, nombre, es_admin) VALUES ($1, $2, false)
             RETURNING id, codigo, nombre, es_admin, created_at",
        )
        .bind(&codigo)
        .bind(req.nombre.trim())
        .fetch_one(&mut *tx)
        .await?;
        for permiso in &req.permisos {
            sqlx::query("INSERT INTO role_permisos (role_id, permiso_codigo) VALUES ($1, $2) ON CONFLICT DO NOTHING")
                .bind(rol.id)
                .bind(permiso)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(RolConPermisos { rol, permisos: req.permisos })
    }

    /// Reemplaza el set completo de permisos del rol por `permisos` - refleja
    /// exactamente lo que el staff marcó, no un merge incremental (mismo
    /// principio que `StaffService::set_modulos_tenant`).
    pub async fn set_permisos_rol(&self, rol_id: Uuid, permisos: &[String]) -> Result<RolConPermisos> {
        let mut tx = self.pool.begin().await?;
        let rol = sqlx::query_as::<_, Rol>("SELECT id, codigo, nombre, es_admin, created_at FROM roles WHERE id = $1")
            .bind(rol_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Rol no encontrado"))?;
        sqlx::query("DELETE FROM role_permisos WHERE role_id = $1").bind(rol_id).execute(&mut *tx).await?;
        for permiso in permisos {
            sqlx::query("INSERT INTO role_permisos (role_id, permiso_codigo) VALUES ($1, $2)")
                .bind(rol_id)
                .bind(permiso)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(RolConPermisos { rol, permisos: permisos.to_vec() })
    }

    /// Hot path: llamado en cada request autenticado por `permission_guard`.
    /// Un solo round trip, indexado por `usuarios.rol_id`.
    pub async fn usuario_tiene_permiso(&self, usuario_id: Uuid, permiso: &str) -> Result<bool> {
        let concedido: bool = sqlx::query_scalar(
            r#"SELECT EXISTS (
                 SELECT 1 FROM usuarios u
                 JOIN roles r ON r.id = u.rol_id
                 LEFT JOIN role_permisos rp ON rp.role_id = r.id AND rp.permiso_codigo = $2
                 WHERE u.id = $1 AND (r.es_admin OR rp.role_id IS NOT NULL)
               )"#,
        )
        .bind(usuario_id)
        .bind(permiso)
        .fetch_one(&self.pool)
        .await?;
        Ok(concedido)
    }
}

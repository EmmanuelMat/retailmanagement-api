//! Adjuntos genéricos - Módulo 15c
//! No existía ninguna abstracción de storage reusable (solo el patrón de
//! disco de image_service.rs para fotos de producto, y el patrón
//! BYTEA-cifrado de certificados_dgii, específico a un archivo pequeño/
//! singular/sensible). Se generaliza el patrón de disco de image_service:
//! archivo tal cual (sin reescalar) bajo
//! {UPLOADS_DIR}/{tenant_id}/adjuntos/{entidad_tipo}/{entidad_id}/{uuid}-{nombre},
//! servido por el mismo ServeDir de /uploads ya montado en main.rs.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::path::PathBuf;
use uuid::Uuid;

const MAX_UPLOAD_BYTES: usize = 20 * 1024 * 1024;
pub const ENTIDADES_VALIDAS: &[&str] = &["ORDEN_SERVICIO", "COTIZACION", "VENTA", "CLIENTE", "ORDEN_COMPRA"];

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Adjunto {
    pub id: Uuid,
    pub tenant_id: String,
    pub entidad_tipo: String,
    pub entidad_id: Uuid,
    pub nombre_archivo: String,
    pub storage_path: String,
    pub mime_type: Option<String>,
    pub tamano: Option<i64>,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

/// Quita cualquier separador de ruta o `..` del nombre original - nunca se
/// usa el nombre del cliente para construir la ruta en disco sin sanear.
fn sanitize_filename(nombre: &str) -> String {
    let base = nombre.rsplit(['/', '\\']).next().unwrap_or(nombre);
    let cleaned: String = base.chars().filter(|c| c.is_alphanumeric() || matches!(c, '.' | '-' | '_' | ' ')).collect();
    let trimmed = cleaned.trim();
    if trimmed.is_empty() { "archivo".to_string() } else { trimmed.to_string() }
}

pub struct AdjuntoService {
    pool: PgPool,
    dir: PathBuf,
}

impl AdjuntoService {
    pub fn new(pool: PgPool, uploads_dir: impl Into<PathBuf>) -> Self {
        Self { pool, dir: uploads_dir.into() }
    }

    pub async fn guardar(
        &self,
        tenant_id: &str,
        entidad_tipo: &str,
        entidad_id: Uuid,
        usuario_id: Option<Uuid>,
        nombre_archivo: &str,
        mime_type: Option<String>,
        bytes: Vec<u8>,
    ) -> anyhow::Result<Adjunto> {
        if !ENTIDADES_VALIDAS.contains(&entidad_tipo) {
            anyhow::bail!("entidad_tipo inválido: {}", entidad_tipo);
        }
        if bytes.is_empty() {
            anyhow::bail!("El archivo está vacío");
        }
        if bytes.len() > MAX_UPLOAD_BYTES {
            anyhow::bail!("El archivo supera el tamaño máximo permitido (20MB)");
        }

        let nombre_limpio = sanitize_filename(nombre_archivo);
        let rel_dir = format!("adjuntos/{}/{}", entidad_tipo, entidad_id);
        let tenant_dir = self.dir.join(tenant_id).join(&rel_dir);
        tokio::fs::create_dir_all(&tenant_dir).await?;

        let filename = format!("{}-{}", Uuid::new_v4(), nombre_limpio);
        let dest = tenant_dir.join(&filename);
        let tamano = bytes.len() as i64;
        tokio::fs::write(&dest, &bytes).await?;

        let storage_path = format!("/uploads/{}/{}/{}", tenant_id, rel_dir, filename);

        let adjunto = sqlx::query_as::<_, Adjunto>(
            r#"INSERT INTO adjuntos (tenant_id, entidad_tipo, entidad_id, nombre_archivo, storage_path, mime_type, tamano, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id, tenant_id, entidad_tipo, entidad_id, nombre_archivo, storage_path, mime_type, tamano, usuario_id, created_at"#,
        )
        .bind(tenant_id)
        .bind(entidad_tipo)
        .bind(entidad_id)
        .bind(nombre_archivo)
        .bind(&storage_path)
        .bind(&mime_type)
        .bind(tamano)
        .bind(usuario_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(adjunto)
    }

    pub async fn listar(&self, tenant_id: &str, entidad_tipo: &str, entidad_id: Uuid) -> anyhow::Result<Vec<Adjunto>> {
        let rows = sqlx::query_as::<_, Adjunto>(
            "SELECT id, tenant_id, entidad_tipo, entidad_id, nombre_archivo, storage_path, mime_type, tamano, usuario_id, created_at \
             FROM adjuntos WHERE tenant_id = $1 AND entidad_tipo = $2 AND entidad_id = $3 ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .bind(entidad_tipo)
        .bind(entidad_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn eliminar(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<()> {
        let row: Option<(String,)> = sqlx::query_as("DELETE FROM adjuntos WHERE id = $1 AND tenant_id = $2 RETURNING storage_path")
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        let (storage_path,) = row.ok_or_else(|| anyhow::anyhow!("Adjunto no encontrado"))?;
        // storage_path es "/uploads/<resto>" - le quitamos el prefijo del
        // ServeDir para reconstruir la ruta real en disco.
        if let Some(rel) = storage_path.strip_prefix("/uploads/") {
            let _ = tokio::fs::remove_file(self.dir.join(rel)).await;
        }
        Ok(())
    }
}

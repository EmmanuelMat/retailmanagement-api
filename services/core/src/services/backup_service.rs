//! Respaldo local: la arquitectura es un Postgres por negocio en su propia
//! computadora, sin nube de por medio - un disco dañado sin esto es pérdida
//! total del historial del negocio. Se apoya en `pg_dump` (ya viene con la
//! imagen de Postgres del docker-compose local; el instalador debe incluir
//! las herramientas cliente de Postgres junto al binario) en vez de
//! reinventar un exportador por tabla - restaura directo con `pg_restore`.

use anyhow::{Context, Result};
use chrono::Utc;
use std::path::PathBuf;
use tokio::process::Command;

const RETENCION: usize = 14;

pub struct BackupService {
    database_url: String,
    backup_dir: PathBuf,
}

impl BackupService {
    pub fn new(database_url: String) -> Self {
        Self { database_url, backup_dir: PathBuf::from("backups") }
    }

    /// Corre un `pg_dump` fresco y poda respaldos viejos. Usado tanto por el
    /// job programado cada 24h como por el botón "Descargar respaldo ahora".
    pub async fn run_backup(&self) -> Result<PathBuf> {
        tokio::fs::create_dir_all(&self.backup_dir)
            .await
            .context("No se pudo crear la carpeta de respaldos")?;

        let filename = format!("backup-{}.dump", Utc::now().format("%Y%m%d-%H%M%S"));
        let path = self.backup_dir.join(&filename);

        let status = Command::new("pg_dump")
            .arg(&self.database_url)
            .arg("-Fc")
            .arg("-f")
            .arg(&path)
            .status()
            .await
            .context("No se pudo ejecutar pg_dump - ¿están instaladas las herramientas cliente de Postgres?")?;

        if !status.success() {
            anyhow::bail!("pg_dump terminó con error (código {:?})", status.code());
        }

        if let Err(e) = self.prune_old(RETENCION).await {
            tracing::warn!("No se pudieron podar respaldos viejos: {}", e);
        }

        Ok(path)
    }

    async fn prune_old(&self, keep: usize) -> Result<()> {
        let mut entries = tokio::fs::read_dir(&self.backup_dir).await?;
        let mut files = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("dump") {
                let modified = entry.metadata().await?.modified()?;
                files.push((modified, entry.path()));
            }
        }
        files.sort_by_key(|(modified, _)| *modified);
        if files.len() > keep {
            for (_, path) in &files[..files.len() - keep] {
                let _ = tokio::fs::remove_file(path).await;
            }
        }
        Ok(())
    }
}

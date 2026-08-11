// Miniaturas de fotos de producto. Guarda solo un JPEG reescalado en disco;
// nunca el binario original ni en la fila de Postgres. La ruta pública
// devuelta se sirve vía ServeDir montado en main.rs (/uploads).

use image::imageops::FilterType;
use std::path::PathBuf;
use uuid::Uuid;

const MAX_DIMENSION: u32 = 240;
const MAX_UPLOAD_BYTES: usize = 8 * 1024 * 1024;
const JPEG_QUALITY: u8 = 82;

pub struct ImageService {
    dir: PathBuf,
}

impl ImageService {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub async fn save_thumbnail(
        &self,
        tenant_id: &str,
        producto_id: Uuid,
        bytes: Vec<u8>,
    ) -> anyhow::Result<String> {
        if bytes.is_empty() {
            anyhow::bail!("El archivo de imagen está vacío");
        }
        if bytes.len() > MAX_UPLOAD_BYTES {
            anyhow::bail!("La imagen supera el tamaño máximo permitido (8MB)");
        }

        let tenant_dir = self.dir.join(tenant_id);
        let filename = format!("{}.jpg", producto_id);
        let dest = tenant_dir.join(&filename);

        tokio::fs::create_dir_all(&tenant_dir).await?;

        let encoded = tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<u8>> {
            let img = image::load_from_memory(&bytes)
                .map_err(|_| anyhow::anyhow!("Formato de imagen no soportado"))?;
            let resized = img.resize(MAX_DIMENSION, MAX_DIMENSION, FilterType::Triangle);
            let mut out = Vec::new();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
            encoder.encode_image(&resized)?;
            Ok(out)
        })
        .await??;

        tokio::fs::write(&dest, encoded).await?;

        Ok(format!("/uploads/{}/{}", tenant_id, filename))
    }
}

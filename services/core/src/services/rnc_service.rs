//! Consulta de RNC/Cédula contra el padrón de contribuyentes DGII, importado
//! localmente (ver `bin/import_rnc.rs`) desde el archivo público que publica
//! DGII en https://dgii.gov.do/app/WebApps/Consultas/RNC/DGII_RNC.zip.
//! Es dato nacional compartido, no tenant-scoped.

use anyhow::Result;
use chrono::NaiveDate;
use serde::Serialize;
use sqlx::PgPool;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct RncRecord {
    pub rnc: String,
    pub nombre: String,
    pub nombre_comercial: Option<String>,
    pub actividad_economica: Option<String>,
    pub fecha_inicio: Option<NaiveDate>,
    pub estado: Option<String>,
    pub tipo_contribuyente: Option<String>,
}

pub struct RncService {
    pool: PgPool,
}

impl RncService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn lookup(&self, rnc_o_cedula: &str) -> Result<Option<RncRecord>> {
        let clean: String = rnc_o_cedula.chars().filter(|c| c.is_ascii_digit()).collect();
        if clean.is_empty() {
            return Ok(None);
        }
        let row = sqlx::query_as::<_, RncRecord>(
            "SELECT rnc, nombre, nombre_comercial, actividad_economica, fecha_inicio, estado, tipo_contribuyente
             FROM rnc_padron WHERE rnc = $1",
        )
        .bind(&clean)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row)
    }
}

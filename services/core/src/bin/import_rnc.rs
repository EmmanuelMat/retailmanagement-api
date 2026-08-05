//! Descarga e importa el padrón de contribuyentes DGII (RNC/Cédula) al
//! Postgres local, para poder hacer búsquedas instantáneas por RNC/Cédula
//! desde la app sin depender de una consulta en vivo a dgii.gov.do (que no
//! tiene una API pública documentada y bloquea tráfico no-navegador).
//!
//! Fuente oficial: https://dgii.gov.do/app/WebApps/Consultas/RNC/DGII_RNC.zip
//! (Servicios → RNC Contribuyentes → Descarga Listado de todos los RNC).
//! Formato: TXT delimitado por "|", codificado en ISO-8859-1, ~780k filas:
//!   RNC|Nombre|NombreComercial|ActividadEconomica| | | | |FechaInicio|Estado|TipoContribuyente
//!
//! Uso: cargo run --bin import_rnc
//! (puede tardar unos minutos; re-ejecutar periódicamente para refrescar)

use chrono::NaiveDate;
use sqlx::{PgPool, QueryBuilder};
use std::io::Read;

const DGII_RNC_URL: &str = "https://dgii.gov.do/app/WebApps/Consultas/RNC/DGII_RNC.zip";
const BATCH_SIZE: usize = 2000;

struct Contribuyente {
    rnc: String,
    nombre: String,
    nombre_comercial: Option<String>,
    actividad_economica: Option<String>,
    fecha_inicio: Option<NaiveDate>,
    estado: Option<String>,
    tipo_contribuyente: Option<String>,
}

/// El archivo DGII viene en ISO-8859-1 (Latin-1): cada byte mapea 1:1 al
/// mismo code point Unicode, así que no hace falta una crate de encoding.
fn decode_latin1(bytes: &[u8]) -> String {
    bytes.iter().map(|&b| b as char).collect()
}

fn none_if_empty(s: &str) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

fn parse_row(line: &str) -> Option<Contribuyente> {
    let fields: Vec<&str> = line.trim_end_matches(['\r', '\n']).split('|').collect();
    if fields.len() < 11 {
        return None;
    }
    let rnc: String = fields[0].chars().filter(|c| c.is_ascii_digit()).collect();
    if rnc.is_empty() {
        return None;
    }
    let nombre = fields[1].trim().to_string();
    if nombre.is_empty() {
        return None;
    }
    let fecha_inicio = NaiveDate::parse_from_str(fields[8].trim(), "%d/%m/%Y").ok();

    Some(Contribuyente {
        rnc,
        nombre,
        nombre_comercial: none_if_empty(fields[2]),
        actividad_economica: none_if_empty(fields[3]),
        fecha_inicio,
        estado: none_if_empty(fields[9]),
        tipo_contribuyente: none_if_empty(fields[10]),
    })
}

async fn insert_batch(pool: &PgPool, batch: &[Contribuyente]) -> anyhow::Result<()> {
    if batch.is_empty() {
        return Ok(());
    }
    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        "INSERT INTO rnc_padron (rnc, nombre, nombre_comercial, actividad_economica, fecha_inicio, estado, tipo_contribuyente, updated_at) ",
    );
    qb.push_values(batch, |mut b, c| {
        b.push_bind(&c.rnc)
            .push_bind(&c.nombre)
            .push_bind(&c.nombre_comercial)
            .push_bind(&c.actividad_economica)
            .push_bind(c.fecha_inicio)
            .push_bind(&c.estado)
            .push_bind(&c.tipo_contribuyente)
            .push("NOW()");
    });
    qb.push(
        " ON CONFLICT (rnc) DO UPDATE SET nombre = EXCLUDED.nombre, nombre_comercial = EXCLUDED.nombre_comercial,
           actividad_economica = EXCLUDED.actividad_economica, fecha_inicio = EXCLUDED.fecha_inicio,
           estado = EXCLUDED.estado, tipo_contribuyente = EXCLUDED.tipo_contribuyente, updated_at = NOW()",
    );
    qb.build().execute(pool).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fiscal_core".to_string());
    let pool = PgPool::connect(&db_url).await?;

    println!("Descargando padrón DGII ({DGII_RNC_URL})...");
    let client = reqwest::Client::new();
    let zip_bytes = client
        .get(DGII_RNC_URL)
        .header("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36")
        .header("Referer", "https://dgii.gov.do/")
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    println!("Descargado: {:.1} MB", zip_bytes.len() as f64 / 1_048_576.0);

    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip_bytes))?;
    let mut txt_bytes = Vec::new();
    {
        let mut found = false;
        for i in 0..archive.len() {
            let mut entry = archive.by_index(i)?;
            if entry.name().to_uppercase().ends_with(".TXT") {
                entry.read_to_end(&mut txt_bytes)?;
                found = true;
                break;
            }
        }
        if !found {
            anyhow::bail!("No se encontró un .TXT dentro del ZIP descargado");
        }
    }
    println!("Descomprimido: {:.1} MB, decodificando ISO-8859-1...", txt_bytes.len() as f64 / 1_048_576.0);

    let text = decode_latin1(&txt_bytes);
    drop(txt_bytes);

    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut total = 0usize;
    let mut skipped = 0usize;

    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match parse_row(line) {
            Some(c) => batch.push(c),
            None => {
                skipped += 1;
                continue;
            }
        }
        if batch.len() >= BATCH_SIZE {
            insert_batch(&pool, &batch).await?;
            total += batch.len();
            batch.clear();
            if total % 50_000 == 0 {
                println!("  {total} contribuyentes importados...");
            }
        }
    }
    insert_batch(&pool, &batch).await?;
    total += batch.len();

    println!("Listo: {total} contribuyentes importados/actualizados, {skipped} filas omitidas (formato inválido).");
    Ok(())
}

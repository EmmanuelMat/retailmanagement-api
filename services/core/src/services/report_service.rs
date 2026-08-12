//! Reportes DGII (606/607) y resumen del Dashboard - Módulo 10
//! Reescrito sobre datos reales de compras/ventas (antes era un stub con TXT
//! fijo, sin conexión a ninguna tabla — confirmado muerto por `cargo build`).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;

pub struct ReportService {
    pool: PgPool,
}

#[derive(Debug, Serialize)]
pub struct DashboardResumen {
    pub ventas_hoy_total: Decimal,
    pub ventas_hoy_cantidad: i64,
    pub productos_bajo_minimo: i64,
    pub valor_inventario: Decimal,
    pub caja_abierta: bool,
}

/// Envuelve en comillas y escapa comillas internas solo si el campo lo
/// necesita (coma, comilla o salto de línea) — evita ensuciar el CSV con
/// comillas innecesarias en el caso común (RNC/NCF sin caracteres especiales).
fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// Tipo Id (casilla 2 del 606, casilla 2 del 607): RNC dominicano son 9
/// dígitos, Cédula son 11. No hay campo separado que lo indique en el
/// schema, así que se deriva del largo del identificador (después de
/// quitar guiones/espacios), como hace la propia herramienta de la DGII.
fn tipo_id_dgii(identificador: &str) -> u8 {
    let digitos: String = identificador.chars().filter(|c| c.is_ascii_digit()).collect();
    match digitos.len() {
        11 => 2, // Cédula
        _ => 1,  // RNC (o Pasaporte/ID tributaria en 607, no distinguible aquí)
    }
}

/// Forma de Pago (casilla 23 del 606) según el instructivo oficial DGII.
/// `compras.metodo_pago` solo tiene 4 valores (no incluye Permuta/Notas de
/// crédito/Mixto), así que el mapeo es total.
fn forma_pago_606(metodo_pago: &str) -> u8 {
    match metodo_pago {
        "EFECTIVO" => 1,
        "TRANSFERENCIA" => 2, // Cheques/Transferencias/Depósito
        "TARJETA" => 3,
        "FIADO" => 4, // Compra a crédito
        _ => 1,
    }
}

/// Fila cruda de `compras` con todos los campos que exige el Formato 606.
#[derive(sqlx::FromRow)]
struct Compra606Row {
    rnc: Option<String>,
    ncf: Option<String>,
    ncf_modificado: Option<String>,
    tipo_bienes_servicios: Option<i16>,
    fecha: DateTime<Utc>,
    fecha_pago: Option<DateTime<Utc>>,
    monto_servicios: Decimal,
    monto_bienes: Decimal,
    itbis_facturado: Decimal,
    itbis_retenido: Decimal,
    itbis_proporcionalidad: Decimal,
    itbis_costo: Decimal,
    tipo_retencion_isr: Option<i16>,
    monto_retencion_renta: Decimal,
    isc: Decimal,
    otros_impuestos: Decimal,
    propina_legal: Decimal,
    metodo_pago: String,
}

const COMPRA_606_SELECT: &str = r#"SELECT p.rnc, c.ncf_proveedor AS ncf, c.ncf_modificado, c.tipo_bienes_servicios,
              c.created_at AS fecha, c.fecha_pago, c.monto_facturado_servicios AS monto_servicios,
              c.monto_facturado_bienes AS monto_bienes, c.itbis_total AS itbis_facturado,
              c.itbis_retenido, c.itbis_proporcionalidad, c.itbis_costo, c.tipo_retencion_isr,
              c.monto_retencion_renta, c.isc, c.otros_impuestos, c.propina_legal, c.metodo_pago
       FROM compras c LEFT JOIN proveedores p ON p.id = c.proveedor_id
       WHERE c.tenant_id = $1 AND c.created_at >= $2 AND c.created_at < $3 AND c.estado = 'COMPLETADA'
       ORDER BY c.created_at"#;

/// Línea pipe-delimitada de 23 columnas por fila, en el orden exacto del
/// instructivo oficial DGII "Llenado y Remisión del Formato 606" (rev.
/// febrero 2026). Compras sin `tipo_bienes_servicios` (campo requerido por
/// DGII, casilla 3) se reportan con el default 9 "Compras y gastos que
/// formarán parte del costo de venta" — el caso típico de un retail que
/// compra mercancía — en vez de excluirlas silenciosamente del envío.
fn compra_606_line(r: &Compra606Row) -> String {
    let tipo_id = tipo_id_dgii(r.rnc.as_deref().unwrap_or_default());
    let tipo_bienes = r.tipo_bienes_servicios.unwrap_or(9);
    let total_facturado = r.monto_servicios + r.monto_bienes;
    let itbis_por_adelantar = r.itbis_facturado - r.itbis_costo;
    let forma_pago = forma_pago_606(&r.metodo_pago);
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
        r.rnc.clone().unwrap_or_default(),
        tipo_id,
        tipo_bienes,
        r.ncf.clone().unwrap_or_default(),
        r.ncf_modificado.clone().unwrap_or_default(),
        r.fecha.format("%Y%m%d"),
        r.fecha_pago.map(|f| f.format("%Y%m%d").to_string()).unwrap_or_default(),
        r.monto_servicios,
        r.monto_bienes,
        total_facturado,
        r.itbis_facturado,
        r.itbis_retenido,
        r.itbis_proporcionalidad,
        r.itbis_costo,
        itbis_por_adelantar,
        Decimal::ZERO, // 16: ITBIS percibido en compras — no habilitado por DGII aún
        r.tipo_retencion_isr.map(|t| t.to_string()).unwrap_or_default(),
        r.monto_retencion_renta,
        Decimal::ZERO, // 19: ISR Percibido en compras — no habilitado por DGII aún
        r.isc,
        r.otros_impuestos,
        r.propina_legal,
        forma_pago,
    )
}

impl ReportService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    fn date_range_bounds(fecha_desde: chrono::NaiveDate, fecha_hasta: chrono::NaiveDate) -> anyhow::Result<(DateTime<Utc>, DateTime<Utc>)> {
        if fecha_hasta < fecha_desde {
            anyhow::bail!("La fecha final no puede ser anterior a la fecha inicial");
        }
        let desde = fecha_desde.and_hms_opt(0, 0, 0).unwrap().and_utc();
        // Exclusivo: el día final completo queda incluido sin duplicar el
        // primer instante del día siguiente.
        let hasta = (fecha_hasta + chrono::Duration::days(1)).and_hms_opt(0, 0, 0).unwrap().and_utc();
        Ok((desde, hasta))
    }

    fn period_bounds(period: &str) -> anyhow::Result<(DateTime<Utc>, DateTime<Utc>)> {
        // period: "YYYYMM"
        if period.len() != 6 {
            anyhow::bail!("Período inválido, use formato YYYYMM");
        }
        let year: i32 = period[0..4].parse()?;
        let month: u32 = period[4..6].parse()?;
        let desde = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .ok_or_else(|| anyhow::anyhow!("Período inválido"))?
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();
        let hasta = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .ok_or_else(|| anyhow::anyhow!("Período inválido"))?
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();
        Ok((desde, hasta))
    }

    /// 606 - Compras de bienes y servicios del período (Norma General
    /// 07-2018), formato completo de 23 columnas per el instructivo oficial
    /// DGII vigente (rev. febrero 2026). A diferencia del 607, el 606 SÍ
    /// incluye comprobantes e-CF/e-NCF de proveedores: DGII exige reportar
    /// aquí "los Comprobantes Electrónicos adquiridos de los facturadores
    /// electrónicos autorizados" — no hay duplicación porque este reporte
    /// documenta *el gasto/costo deducible del comprador*, no la venta del
    /// proveedor (esa ya la recibió DGII directamente de él). Por eso
    /// `ncf_proveedor` — sea e-NCF (prefijo E) o NCF tradicional (prefijo
    /// B) — se incluye siempre sin distinción de tipo.
    ///
    /// Compras ANULADAS se excluyen (ver `ComprasService::anular_compra`);
    /// si una compra ya reportada en un período anterior necesita
    /// corregirse, la vía correcta es una fila NOTA_CREDITO/NOTA_DEBITO con
    /// `ncf_modificado` apuntando al NCF original, no editar el histórico.
    pub async fn generate_606(&self, tenant_id: &str, period: &str) -> anyhow::Result<String> {
        let (desde, hasta) = Self::period_bounds(period)?;
        let rows: Vec<Compra606Row> = sqlx::query_as(COMPRA_606_SELECT)
            .bind(tenant_id)
            .bind(desde)
            .bind(hasta)
            .fetch_all(&self.pool)
            .await?;

        let mut txt = format!("{}|{}|{}\n", tenant_id, period, rows.len());
        for r in &rows {
            txt.push_str(&compra_606_line(r));
        }
        Ok(txt)
    }

    /// 607 complementario (Norma General 07-2018, Art. 4 Párrafo III):
    /// SOLO ventas explícitamente autorizadas por la DGII a facturarse
    /// fuera de la Solución Fiscal (e-CF) — p.ej. NCF físico/Serie B con
    /// autorización especial, o rectificación de ventas históricas previas
    /// a la migración a e-CF. Si el contribuyente no tiene operaciones
    /// fuera de la Solución Fiscal en el período, queda EXIMIDO de remitir
    /// este formato (se presenta en cero).
    ///
    /// Las ventas con e-CF (v.e_ncf IS NOT NULL) NUNCA se incluyen aquí: la
    /// DGII las recibe directamente por el canal e-CF al momento de la
    /// emisión, así que reportarlas también en el 607 duplicaría la
    /// transacción ante la DGII. Ver docs/02-DGII-COMPLIANCE.md y el
    /// instructivo oficial "Llenado y Remisión del Formato 607" (DGII,
    /// dic. 2025): "Los contribuyentes obligados a remitir el Libro de
    /// Venta Mensual, deberán enviar el Formato de Envío 607 complementario,
    /// con las ventas que hayan sido autorizadas por Impuestos Internos a
    /// facturarse fuera de las Soluciones Fiscales."
    ///
    /// `v.ncf_no_electronico` es el único campo que puede poblar esta
    /// excepción; para un tenant 100% e-CF (el caso normal de este sistema)
    /// siempre estará vacío y el reporte devuelve 0 filas — comportamiento
    /// correcto, no un bug.
    pub async fn generate_607(&self, tenant_id: &str, period: &str) -> anyhow::Result<String> {
        let (desde, hasta) = Self::period_bounds(period)?;
        let rows: Vec<(Option<String>, Option<String>, DateTime<Utc>, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT cl.rnc_cedula, v.ncf_no_electronico, v.created_at, v.subtotal, v.itbis_total
               FROM ventas v LEFT JOIN clientes cl ON cl.id = v.cliente_id
               WHERE v.tenant_id = $1 AND v.created_at >= $2 AND v.created_at < $3
                 AND v.e_ncf IS NULL AND v.ncf_no_electronico IS NOT NULL AND v.estado = 'COMPLETADA'
               ORDER BY v.created_at"#,
        )
        .bind(tenant_id)
        .bind(desde)
        .bind(hasta)
        .fetch_all(&self.pool)
        .await?;

        let mut txt = format!("{}|{}|{}\n", tenant_id, period, rows.len());
        for (rnc, ncf, fecha, monto, itbis) in rows {
            txt.push_str(&format!(
                "{}|{}|{}|{}|{}|01\n",
                rnc.unwrap_or_else(|| "000000000".to_string()),
                ncf.unwrap_or_default(),
                fecha.format("%d-%m-%Y"),
                monto,
                itbis
            ));
        }
        Ok(txt)
    }

    /// 606 en CSV para un rango de fechas arbitrario — pensado para que
    /// contabilidad lo abra en Excel; el TXT de `generate_606` (mes
    /// calendario completo, formato pipe) sigue siendo el que se presenta
    /// a la DGII tal cual.
    pub async fn generate_606_csv(&self, tenant_id: &str, fecha_desde: chrono::NaiveDate, fecha_hasta: chrono::NaiveDate) -> anyhow::Result<String> {
        let (desde, hasta) = Self::date_range_bounds(fecha_desde, fecha_hasta)?;
        let rows: Vec<Compra606Row> = sqlx::query_as(COMPRA_606_SELECT)
            .bind(tenant_id)
            .bind(desde)
            .bind(hasta)
            .fetch_all(&self.pool)
            .await?;

        let mut csv = String::from(
            "RNC Proveedor,Tipo Id,Tipo Bienes/Servicios,NCF,NCF Modificado,Fecha Comprobante,Fecha Pago,\
             Monto Facturado Servicios,Monto Facturado Bienes,Total Monto Facturado,ITBIS Facturado,\
             ITBIS Retenido,ITBIS Proporcionalidad,ITBIS Costo,ITBIS por Adelantar,Tipo Retención ISR,\
             Monto Retención Renta,ISC,Otros Impuestos,Propina Legal,Forma de Pago\n",
        );
        for r in &rows {
            let tipo_id = tipo_id_dgii(r.rnc.as_deref().unwrap_or_default());
            let tipo_bienes = r.tipo_bienes_servicios.unwrap_or(9);
            let total_facturado = r.monto_servicios + r.monto_bienes;
            let itbis_por_adelantar = r.itbis_facturado - r.itbis_costo;
            let forma_pago = forma_pago_606(&r.metodo_pago);
            csv.push_str(&format!(
                "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
                csv_escape(r.rnc.as_deref().unwrap_or_default()),
                tipo_id,
                tipo_bienes,
                csv_escape(r.ncf.as_deref().unwrap_or_default()),
                csv_escape(r.ncf_modificado.as_deref().unwrap_or_default()),
                r.fecha.format("%d-%m-%Y"),
                r.fecha_pago.map(|f| f.format("%d-%m-%Y").to_string()).unwrap_or_default(),
                r.monto_servicios,
                r.monto_bienes,
                total_facturado,
                r.itbis_facturado,
                r.itbis_retenido,
                r.itbis_proporcionalidad,
                r.itbis_costo,
                itbis_por_adelantar,
                r.tipo_retencion_isr.map(|t| t.to_string()).unwrap_or_default(),
                r.monto_retencion_renta,
                r.isc,
                r.otros_impuestos,
                r.propina_legal,
                forma_pago,
            ));
        }
        Ok(csv)
    }

    /// 607 en CSV para un rango de fechas arbitrario — misma relación con
    /// `generate_607` que `generate_606_csv` tiene con `generate_606`, y la
    /// misma exclusión de ventas e-CF (ver comentario de `generate_607`).
    pub async fn generate_607_csv(&self, tenant_id: &str, fecha_desde: chrono::NaiveDate, fecha_hasta: chrono::NaiveDate) -> anyhow::Result<String> {
        let (desde, hasta) = Self::date_range_bounds(fecha_desde, fecha_hasta)?;
        let rows: Vec<(Option<String>, Option<String>, DateTime<Utc>, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT cl.rnc_cedula, v.ncf_no_electronico, v.created_at, v.subtotal, v.itbis_total
               FROM ventas v LEFT JOIN clientes cl ON cl.id = v.cliente_id
               WHERE v.tenant_id = $1 AND v.created_at >= $2 AND v.created_at < $3
                 AND v.e_ncf IS NULL AND v.ncf_no_electronico IS NOT NULL AND v.estado = 'COMPLETADA'
               ORDER BY v.created_at"#,
        )
        .bind(tenant_id)
        .bind(desde)
        .bind(hasta)
        .fetch_all(&self.pool)
        .await?;

        let mut csv = String::from("RNC/Cédula Cliente,NCF (no electrónico),Fecha,Monto Facturado,ITBIS Facturado\n");
        for (rnc, ncf, fecha, monto, itbis) in rows {
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                csv_escape(&rnc.unwrap_or_else(|| "000000000".to_string())),
                csv_escape(&ncf.unwrap_or_default()),
                fecha.format("%d-%m-%Y"),
                monto,
                itbis
            ));
        }
        Ok(csv)
    }

    pub async fn dashboard_resumen(&self, tenant_id: &str) -> anyhow::Result<DashboardResumen> {
        let hoy_row: (Option<Decimal>, i64) = sqlx::query_as(
            "SELECT SUM(total), COUNT(*) FROM ventas WHERE tenant_id = $1 AND created_at::date = CURRENT_DATE",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        let inv_row: (Option<Decimal>, i64) = sqlx::query_as(
            "SELECT SUM(stock_actual * costo), COUNT(*) FILTER (WHERE stock_actual <= stock_minimo) FROM productos WHERE tenant_id = $1 AND activo = true",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        let caja_abierta: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM caja_sesiones WHERE tenant_id = $1 AND estado = 'ABIERTA')",
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;

        Ok(DashboardResumen {
            ventas_hoy_total: hoy_row.0.unwrap_or_default(),
            ventas_hoy_cantidad: hoy_row.1,
            productos_bajo_minimo: inv_row.1,
            valor_inventario: inv_row.0.unwrap_or_default(),
            caja_abierta,
        })
    }
}

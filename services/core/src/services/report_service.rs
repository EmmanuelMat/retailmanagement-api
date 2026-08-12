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

    /// 606 - Compras del período, por RNC del proveedor.
    ///
    /// A diferencia del 607, aquí no hay riesgo de duplicar un e-CF: `compras`
    /// registra facturas *recibidas* de proveedores (propio e-NCF, e-CF
    /// autogenerado tipo 43/47, o NCF tradicional según el régimen del
    /// proveedor), no documentos que este tenant transmite a la DGII. Por eso
    /// se incluyen todas las compras del período sin filtrar por `tipo_ecf`
    /// del proveedor - el 606 es la vía de reporte para todas ellas.
    pub async fn generate_606(&self, tenant_id: &str, period: &str) -> anyhow::Result<String> {
        let (desde, hasta) = Self::period_bounds(period)?;
        let rows: Vec<(Option<String>, Option<String>, DateTime<Utc>, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT p.rnc, c.ncf_proveedor, c.created_at, c.subtotal, c.itbis_total
               FROM compras c LEFT JOIN proveedores p ON p.id = c.proveedor_id
               WHERE c.tenant_id = $1 AND c.created_at >= $2 AND c.created_at < $3
               ORDER BY c.created_at"#,
        )
        .bind(tenant_id)
        .bind(desde)
        .bind(hasta)
        .fetch_all(&self.pool)
        .await?;

        let mut txt = format!("{}|{}|{}\n", tenant_id, period, rows.len());
        for (rnc, ncf, fecha, monto, itbis) in rows {
            txt.push_str(&format!(
                "{}|{}|{}|{}|{}|01|01\n",
                rnc.unwrap_or_else(|| "".to_string()),
                ncf.unwrap_or_else(|| "".to_string()),
                fecha.format("%d-%m-%Y"),
                monto,
                itbis
            ));
        }
        Ok(txt)
    }

    /// 607 - Ventas NO electrónicas del período (tenant con e-CF desactivado).
    ///
    /// Las ventas con e-CF ya fueron transmitidas a la DGII directamente por
    /// `http_emitir_ecf_venta` (ver ecf_service/dgii_client) - reportarlas
    /// también en el 607 duplicaría esa transacción ante la DGII. El único
    /// origen legítimo de datos para el 607 son ventas de un tenant sin
    /// `factura_electronica_activa` (para el cual `emitir-ecf` está bloqueado,
    /// ver `main.rs::http_emitir_ecf_venta`), y que por lo tanto nunca
    /// recibieron un e-NCF. `v.e_ncf IS NULL` se mantiene como chequeo
    /// adicional por si el tenant desactivó e-CF después de tener ventas ya
    /// facturadas electrónicamente - esas ventas viejas deben seguir excluidas.
    /// Las anuladas tampoco son un hecho generador de ITBIS vigente.
    pub async fn generate_607(&self, tenant_id: &str, period: &str) -> anyhow::Result<String> {
        let (desde, hasta) = Self::period_bounds(period)?;
        let rows: Vec<(Option<String>, Option<String>, DateTime<Utc>, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT cl.rnc_cedula, v.e_ncf, v.created_at, v.subtotal, v.itbis_total
               FROM ventas v
               LEFT JOIN clientes cl ON cl.id = v.cliente_id
               JOIN tenants t ON t.rnc = v.tenant_id
               WHERE v.tenant_id = $1 AND v.created_at >= $2 AND v.created_at < $3
                 AND v.estado != 'ANULADA'
                 AND v.e_ncf IS NULL
                 AND t.factura_electronica_activa = false
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
        let rows: Vec<(Option<String>, Option<String>, DateTime<Utc>, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT p.rnc, c.ncf_proveedor, c.created_at, c.subtotal, c.itbis_total
               FROM compras c LEFT JOIN proveedores p ON p.id = c.proveedor_id
               WHERE c.tenant_id = $1 AND c.created_at >= $2 AND c.created_at < $3
               ORDER BY c.created_at"#,
        )
        .bind(tenant_id)
        .bind(desde)
        .bind(hasta)
        .fetch_all(&self.pool)
        .await?;

        let mut csv = String::from("RNC Proveedor,NCF,Fecha,Monto Facturado,ITBIS Facturado\n");
        for (rnc, ncf, fecha, monto, itbis) in rows {
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                csv_escape(&rnc.unwrap_or_default()),
                csv_escape(&ncf.unwrap_or_default()),
                fecha.format("%d-%m-%Y"),
                monto,
                itbis
            ));
        }
        Ok(csv)
    }

    /// 607 en CSV para un rango de fechas arbitrario — misma relación con
    /// `generate_607` que `generate_606_csv` tiene con `generate_606`, y
    /// misma exclusión de ventas ya reportadas vía e-CF (ver comentario en
    /// `generate_607`).
    pub async fn generate_607_csv(&self, tenant_id: &str, fecha_desde: chrono::NaiveDate, fecha_hasta: chrono::NaiveDate) -> anyhow::Result<String> {
        let (desde, hasta) = Self::date_range_bounds(fecha_desde, fecha_hasta)?;
        let rows: Vec<(Option<String>, Option<String>, DateTime<Utc>, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT cl.rnc_cedula, v.e_ncf, v.created_at, v.subtotal, v.itbis_total
               FROM ventas v
               LEFT JOIN clientes cl ON cl.id = v.cliente_id
               JOIN tenants t ON t.rnc = v.tenant_id
               WHERE v.tenant_id = $1 AND v.created_at >= $2 AND v.created_at < $3
                 AND v.estado != 'ANULADA'
                 AND v.e_ncf IS NULL
                 AND t.factura_electronica_activa = false
               ORDER BY v.created_at"#,
        )
        .bind(tenant_id)
        .bind(desde)
        .bind(hasta)
        .fetch_all(&self.pool)
        .await?;

        let mut csv = String::from("RNC/Cédula Cliente,e-NCF,Fecha,Monto Facturado,ITBIS Facturado\n");
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

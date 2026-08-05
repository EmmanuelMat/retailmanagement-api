//! Cotizaciones - Módulo 5b
//! Una propuesta sin compromiso: no toca stock, caja, ni el límite de
//! descuento sin aprobación (esas reglas son de una venta real). Convertir
//! una cotización en Venta pasa por `ventas_service::create_venta` tal cual,
//! orquestado en main.rs - este servicio nunca inserta en `ventas`.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

fn itbis_rate(tipo: &str) -> Decimal {
    match tipo {
        "GRAVADO_18" => dec!(0.18),
        "GRAVADO_16" => dec!(0.16),
        _ => dec!(0),
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Cotizacion {
    pub id: Uuid,
    pub tenant_id: String,
    pub cliente_id: Option<Uuid>,
    pub usuario_id: Option<Uuid>,
    pub subtotal: Decimal,
    pub itbis_total: Decimal,
    pub total: Decimal,
    pub estado: String,
    pub fecha_vencimiento: Option<NaiveDate>,
    pub venta_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CotizacionItem {
    pub id: Uuid,
    pub cotizacion_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub descuento: Decimal,
    pub itbis_tipo: String,
    pub itbis_monto: Decimal,
    pub subtotal: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CotizacionConCliente {
    pub id: Uuid,
    pub cliente_nombre: Option<String>,
    pub total: Decimal,
    pub estado: String,
    pub fecha_vencimiento: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCotizacionItemRequest {
    pub producto_id: Uuid,
    pub cantidad: Decimal,
    pub descuento: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCotizacionRequest {
    pub cliente_id: Option<Uuid>,
    pub items: Vec<CreateCotizacionItemRequest>,
    pub fecha_vencimiento: Option<NaiveDate>,
}

pub struct CotizacionCompleta {
    pub cotizacion: Cotizacion,
    pub items: Vec<CotizacionItem>,
}

pub struct CotizacionService {
    pool: PgPool,
}

impl CotizacionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_cotizacion(&self, tenant_id: &str, usuario_id: Uuid, req: CreateCotizacionRequest) -> anyhow::Result<CotizacionCompleta> {
        if req.items.is_empty() {
            anyhow::bail!("La cotización necesita al menos un producto");
        }

        let mut tx = self.pool.begin().await?;

        let mut subtotal_total = Decimal::ZERO;
        let mut itbis_total = Decimal::ZERO;
        let mut lineas: Vec<(Uuid, String, String, Decimal, Decimal, Decimal, String, Decimal, Decimal)> = Vec::new();

        // A diferencia de create_venta, aquí NO se bloquea la fila con FOR
        // UPDATE ni se descuenta stock - una cotización no reserva inventario.
        for item in &req.items {
            if item.cantidad <= Decimal::ZERO {
                anyhow::bail!("La cantidad debe ser mayor a cero");
            }
            let row: Option<(String, String, Decimal, String)> = sqlx::query_as(
                "SELECT sku, nombre, precio_venta, itbis_tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
            )
            .bind(item.producto_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (sku, nombre, precio_venta, itbis_tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;

            let descuento = item.descuento.unwrap_or_default();
            let line_bruto = precio_venta * item.cantidad;
            if descuento < Decimal::ZERO || descuento > line_bruto {
                anyhow::bail!("Descuento inválido para {}", nombre);
            }
            let line_subtotal = line_bruto - descuento;
            let line_itbis = line_subtotal * itbis_rate(&itbis_tipo);
            subtotal_total += line_subtotal;
            itbis_total += line_itbis;

            lineas.push((item.producto_id, sku, nombre, item.cantidad, precio_venta, descuento, itbis_tipo, line_itbis, line_subtotal));
        }

        let total = subtotal_total + itbis_total;

        let cotizacion = sqlx::query_as::<_, Cotizacion>(
            r#"INSERT INTO cotizaciones (tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, fecha_vencimiento)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id, tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, estado, fecha_vencimiento, venta_id, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.cliente_id)
        .bind(usuario_id)
        .bind(subtotal_total)
        .bind(itbis_total)
        .bind(total)
        .bind(req.fecha_vencimiento)
        .fetch_one(&mut *tx)
        .await?;

        let mut items = Vec::new();
        for (producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, line_subtotal) in lineas {
            let ci = sqlx::query_as::<_, CotizacionItem>(
                r#"INSERT INTO cotizacion_items (cotizacion_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                   RETURNING id, cotizacion_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal"#,
            )
            .bind(cotizacion.id)
            .bind(producto_id)
            .bind(&sku)
            .bind(&nombre)
            .bind(cantidad)
            .bind(precio_unitario)
            .bind(descuento)
            .bind(&itbis_tipo)
            .bind(itbis_monto)
            .bind(line_subtotal)
            .fetch_one(&mut *tx)
            .await?;
            items.push(ci);
        }

        tx.commit().await?;
        Ok(CotizacionCompleta { cotizacion, items })
    }

    const COTIZACIONES_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "c.created_at"),
        ("total", "c.total"),
        ("fecha_vencimiento", "c.fecha_vencimiento"),
        ("estado", "c.estado"),
    ];

    pub async fn list_cotizaciones(
        &self,
        tenant_id: &str,
        estado: Option<String>,
        cliente_id: Option<Uuid>,
        search: Option<String>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<CotizacionConCliente>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE c.tenant_id = $1
               AND ($2::text IS NULL OR c.estado = $2)
               AND ($3::uuid IS NULL OR c.cliente_id = $3)
               AND ($4::text IS NULL OR LOWER(cl.nombre) LIKE $4)
               AND ($5::date IS NULL OR c.created_at::date >= $5)
               AND ($6::date IS NULL OR c.created_at::date <= $6)";

        let total: i64 = sqlx::query_scalar(&format!(
            r#"SELECT COUNT(*) FROM cotizaciones c LEFT JOIN clientes cl ON cl.id = c.cliente_id {WHERE_CLAUSE}"#
        ))
        .bind(tenant_id)
        .bind(&estado)
        .bind(cliente_id)
        .bind(&pattern)
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::COTIZACIONES_SORTABLE, "c.created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT c.id, cl.nombre AS cliente_nombre, c.total, c.estado, c.fecha_vencimiento, c.created_at
               FROM cotizaciones c
               LEFT JOIN clientes cl ON cl.id = c.cliente_id
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $7 OFFSET $8"#
        );
        let rows = sqlx::query_as::<_, CotizacionConCliente>(&query)
            .bind(tenant_id)
            .bind(&estado)
            .bind(cliente_id)
            .bind(&pattern)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn get_cotizacion(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<CotizacionCompleta> {
        let cotizacion = sqlx::query_as::<_, Cotizacion>(
            r#"SELECT id, tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, estado, fecha_vencimiento, venta_id, created_at
               FROM cotizaciones WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cotización no encontrada"))?;

        let items = sqlx::query_as::<_, CotizacionItem>(
            "SELECT id, cotizacion_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal FROM cotizacion_items WHERE cotizacion_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(CotizacionCompleta { cotizacion, items })
    }

    pub async fn marcar_estado(&self, tenant_id: &str, id: Uuid, estado: &str) -> anyhow::Result<Cotizacion> {
        let cotizacion = sqlx::query_as::<_, Cotizacion>(
            r#"UPDATE cotizaciones SET estado = $1 WHERE id = $2 AND tenant_id = $3
               RETURNING id, tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, estado, fecha_vencimiento, venta_id, created_at"#,
        )
        .bind(estado)
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cotización no encontrada"))?;
        Ok(cotizacion)
    }

    /// Usado por el handler de conversión en main.rs para armar el
    /// `CreateVentaRequest` con los mismos items - la cotización misma no
    /// crea la venta.
    pub async fn marcar_convertida(&self, tenant_id: &str, id: Uuid, venta_id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE cotizaciones SET estado = 'CONVERTIDA', venta_id = $1 WHERE id = $2 AND tenant_id = $3")
            .bind(venta_id)
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

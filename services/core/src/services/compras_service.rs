//! Compras y Gastos Service - Módulo 6
//! Contraparte de Ventas: cada compra aumenta stock, recalcula el costo
//! promedio ponderado del producto, y registra un egreso de caja. Gastos es
//! un registro simple sin efecto en inventario (alquiler, servicios, etc).

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Compra {
    pub id: Uuid,
    pub tenant_id: String,
    pub proveedor_id: Option<Uuid>,
    pub usuario_id: Option<Uuid>,
    pub ncf_proveedor: Option<String>,
    pub subtotal: Decimal,
    pub itbis_total: Decimal,
    pub total: Decimal,
    pub metodo_pago: String,
    /// Requerido en la app (no en DB) cuando metodo_pago = FIADO - ver
    /// create_compra. Alimenta la rama Cuentas por Pagar de
    /// contabilidad_service::sincronizar.
    pub fecha_vencimiento: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CompraItem {
    pub id: Uuid,
    pub compra_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub cantidad: Decimal,
    pub costo_unitario: Decimal,
    pub itbis_monto: Decimal,
    pub subtotal: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CompraConProveedor {
    pub id: Uuid,
    pub proveedor_nombre: Option<String>,
    pub total: Decimal,
    pub metodo_pago: String,
    pub created_at: DateTime<Utc>,
}

pub struct CompraCompleta {
    pub compra: Compra,
    pub items: Vec<CompraItem>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCompraItemRequest {
    pub producto_id: Uuid,
    pub cantidad: Decimal,
    pub costo_unitario: Decimal,
    pub itbis_tipo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCompraRequest {
    pub proveedor_id: Option<Uuid>,
    pub ncf_proveedor: Option<String>,
    pub metodo_pago: Option<String>,
    pub fecha_vencimiento: Option<NaiveDate>,
    pub items: Vec<CreateCompraItemRequest>,
}

fn itbis_rate(tipo: &str) -> Decimal {
    match tipo {
        "GRAVADO_18" => Decimal::new(18, 2),
        "GRAVADO_16" => Decimal::new(16, 2),
        _ => Decimal::ZERO,
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Gasto {
    pub id: Uuid,
    pub tenant_id: String,
    pub concepto: String,
    pub categoria: String,
    pub monto: Decimal,
    pub proveedor_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateGastoRequest {
    pub concepto: String,
    pub categoria: Option<String>,
    pub monto: Decimal,
    pub proveedor_id: Option<Uuid>,
}

pub struct ComprasService {
    pool: PgPool,
}

impl ComprasService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_compra(&self, tenant_id: &str, usuario_id: Uuid, req: CreateCompraRequest) -> anyhow::Result<CompraCompleta> {
        if req.items.is_empty() {
            anyhow::bail!("La compra necesita al menos un producto");
        }
        let metodo_pago = req.metodo_pago.unwrap_or_else(|| "EFECTIVO".to_string());
        if metodo_pago == "FIADO" && req.fecha_vencimiento.is_none() {
            anyhow::bail!("Una compra fiada necesita una fecha de vencimiento");
        }

        let mut tx = self.pool.begin().await?;

        let mut subtotal_total = Decimal::ZERO;
        let mut itbis_total = Decimal::ZERO;
        let mut lineas: Vec<(Uuid, String, String, Decimal, Decimal, Decimal, Decimal)> = Vec::new();

        for item in &req.items {
            if item.cantidad <= Decimal::ZERO {
                anyhow::bail!("La cantidad debe ser mayor a cero");
            }
            let row: Option<(String, String, Decimal, Decimal, String)> = sqlx::query_as(
                "SELECT sku, nombre, costo, stock_actual, tipo FROM productos WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
            )
            .bind(item.producto_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (sku, nombre, costo_actual, stock_actual, tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
            if tipo == "SERVICIO" {
                anyhow::bail!("No se pueden comprar servicios — usa Gastos para este concepto");
            }

            let line_subtotal = item.costo_unitario * item.cantidad;
            let itbis_tipo = item.itbis_tipo.clone().unwrap_or_else(|| "EXENTO".to_string());
            let line_itbis = line_subtotal * itbis_rate(&itbis_tipo);
            subtotal_total += line_subtotal;
            itbis_total += line_itbis;

            // Costo promedio ponderado
            let nuevo_stock = stock_actual + item.cantidad;
            let nuevo_costo = if nuevo_stock > Decimal::ZERO {
                (costo_actual * stock_actual + item.costo_unitario * item.cantidad) / nuevo_stock
            } else {
                item.costo_unitario
            };

            sqlx::query("UPDATE productos SET stock_actual = $1, costo = $2, updated_at = NOW() WHERE id = $3")
                .bind(nuevo_stock)
                .bind(nuevo_costo)
                .bind(item.producto_id)
                .execute(&mut *tx)
                .await?;

            lineas.push((item.producto_id, sku, nombre, item.cantidad, item.costo_unitario, line_itbis, line_subtotal));
        }

        let total = subtotal_total + itbis_total;

        let compra = sqlx::query_as::<_, Compra>(
            r#"INSERT INTO compras (tenant_id, proveedor_id, usuario_id, ncf_proveedor, subtotal, itbis_total, total, metodo_pago, fecha_vencimiento)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING id, tenant_id, proveedor_id, usuario_id, ncf_proveedor, subtotal, itbis_total, total, metodo_pago, fecha_vencimiento, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.proveedor_id)
        .bind(usuario_id)
        .bind(&req.ncf_proveedor)
        .bind(subtotal_total)
        .bind(itbis_total)
        .bind(total)
        .bind(&metodo_pago)
        .bind(req.fecha_vencimiento)
        .fetch_one(&mut *tx)
        .await?;

        let mut items = Vec::new();
        for (producto_id, sku, nombre, cantidad, costo_unitario, itbis_monto, line_subtotal) in lineas {
            let ci = sqlx::query_as::<_, CompraItem>(
                r#"INSERT INTO compra_items (compra_id, producto_id, sku, nombre, cantidad, costo_unitario, itbis_monto, subtotal)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                   RETURNING id, compra_id, producto_id, sku, nombre, cantidad, costo_unitario, itbis_monto, subtotal"#,
            )
            .bind(compra.id)
            .bind(producto_id)
            .bind(&sku)
            .bind(&nombre)
            .bind(cantidad)
            .bind(costo_unitario)
            .bind(itbis_monto)
            .bind(line_subtotal)
            .fetch_one(&mut *tx)
            .await?;
            items.push(ci);

            crate::services::inventario_service::InventarioService::insert_movimiento_tx(
                &mut tx,
                tenant_id,
                Some(usuario_id),
                producto_id,
                "ENTRADA",
                cantidad,
                Some(costo_unitario),
                Some("Compra a proveedor".to_string()),
                Some("COMPRA"),
                Some(compra.id),
            )
            .await?;
        }

        // Una compra FIADO no mueve caja - se acredita 2110 Cuentas por
        // Pagar en su lugar (ver contabilidad_service::sincronizar), igual
        // que ventas_service salta el ingreso de caja para una venta FIADO.
        if metodo_pago != "FIADO" {
            sqlx::query(
                r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, metodo_pago, referencia_tipo, referencia_id, usuario_id)
                   VALUES ($1, 'EGRESO', 'Compra a proveedor', $2, $3, 'COMPRA', $4, $5)"#,
            )
            .bind(tenant_id)
            .bind(total)
            .bind(&metodo_pago)
            .bind(compra.id)
            .bind(usuario_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(CompraCompleta { compra, items })
    }

    const COMPRAS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "c.created_at"),
        ("total", "c.total"),
    ];

    pub async fn list_compras(
        &self,
        tenant_id: &str,
        proveedor_id: Option<Uuid>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<CompraConProveedor>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE c.tenant_id = $1
               AND ($2::uuid IS NULL OR c.proveedor_id = $2)
               AND ($3::date IS NULL OR c.created_at::date >= $3)
               AND ($4::date IS NULL OR c.created_at::date <= $4)";

        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM compras c {WHERE_CLAUSE}"
        ))
        .bind(tenant_id)
        .bind(proveedor_id)
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::COMPRAS_SORTABLE, "c.created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT c.id, p.nombre AS proveedor_nombre, c.total, c.metodo_pago, c.created_at
               FROM compras c
               LEFT JOIN proveedores p ON p.id = c.proveedor_id
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $5 OFFSET $6"#
        );
        let rows = sqlx::query_as::<_, CompraConProveedor>(&query)
            .bind(tenant_id)
            .bind(proveedor_id)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn get_compra(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<CompraCompleta> {
        let compra = sqlx::query_as::<_, Compra>(
            r#"SELECT id, tenant_id, proveedor_id, usuario_id, ncf_proveedor, subtotal, itbis_total, total, metodo_pago, fecha_vencimiento, created_at
               FROM compras WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Compra no encontrada"))?;

        let items = sqlx::query_as::<_, CompraItem>(
            "SELECT id, compra_id, producto_id, sku, nombre, cantidad, costo_unitario, itbis_monto, subtotal FROM compra_items WHERE compra_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(CompraCompleta { compra, items })
    }

    // ---- Gastos ----

    const GASTOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "created_at"),
        ("monto", "monto"),
    ];

    pub async fn list_gastos(
        &self,
        tenant_id: &str,
        categoria: Option<String>,
        proveedor_id: Option<Uuid>,
        search: Option<String>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Gasto>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::text IS NULL OR categoria = $2)
               AND ($3::uuid IS NULL OR proveedor_id = $3)
               AND ($4::text IS NULL OR LOWER(concepto) LIKE $4)
               AND ($5::date IS NULL OR created_at::date >= $5)
               AND ($6::date IS NULL OR created_at::date <= $6)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM gastos {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(&categoria)
            .bind(proveedor_id)
            .bind(&pattern)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::GASTOS_SORTABLE, "created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tenant_id, concepto, categoria, monto, proveedor_id, created_at
               FROM gastos
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $7 OFFSET $8"#
        );
        let rows = sqlx::query_as::<_, Gasto>(&query)
            .bind(tenant_id)
            .bind(&categoria)
            .bind(proveedor_id)
            .bind(&pattern)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn create_gasto(&self, tenant_id: &str, usuario_id: Uuid, req: CreateGastoRequest) -> anyhow::Result<Gasto> {
        if req.concepto.trim().is_empty() {
            anyhow::bail!("El concepto del gasto es requerido");
        }
        if req.monto <= Decimal::ZERO {
            anyhow::bail!("El monto debe ser mayor a cero");
        }
        let categoria = req.categoria.unwrap_or_else(|| "OTROS".to_string());

        let mut tx = self.pool.begin().await?;
        let gasto = sqlx::query_as::<_, Gasto>(
            r#"INSERT INTO gastos (tenant_id, concepto, categoria, monto, proveedor_id, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, tenant_id, concepto, categoria, monto, proveedor_id, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.concepto.trim())
        .bind(&categoria)
        .bind(req.monto)
        .bind(req.proveedor_id)
        .bind(usuario_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, referencia_tipo, referencia_id, usuario_id)
               VALUES ($1, 'EGRESO', $2, $3, 'GASTO', $4, $5)"#,
        )
        .bind(tenant_id)
        .bind(&gasto.concepto)
        .bind(req.monto)
        .bind(gasto.id)
        .bind(usuario_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(gasto)
    }
}

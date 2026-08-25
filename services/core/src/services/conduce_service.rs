//! Conduces (guías de despacho / "Órdenes de Servicio" para tenants
//! SERVICIOS) - Módulo 5c
//! Un conduce ya NO es un documento con precio propio: la Venta (factura) se
//! crea PRIMERO, por la cantidad total acordada; el conduce solo registra
//! que una parte de esa cantidad salió físicamente del negocio - sin precio,
//! sin ITBIS, solo cantidad. El stock se descuenta aquí, en el momento de
//! la entrega, no en `ventas_service::create_venta` (que para una venta con
//! `entrega_diferida` deja el stock intacto - ver ese archivo).
//!
//! Un conduce también puede crearse standalone (`venta_id` ausente): sin una
//! Venta previa que le dé cliente/líneas, `cliente_id` identifica al cliente
//! y cada línea trae su propio `producto_id`/`descripcion`/`cantidad` en vez
//! de copiarlos de un `venta_item`. Un producto tipo SERVICIO (ver
//! catalog_service::Producto) nunca mueve stock en ninguno de los dos casos.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Conduce {
    pub id: Uuid,
    pub tenant_id: String,
    pub venta_id: Option<Uuid>,
    pub cliente_id: Option<Uuid>,
    pub usuario_id: Option<Uuid>,
    pub direccion_entrega: Option<String>,
    pub orden_compra: Option<String>,
    pub vehiculo_placa: Option<String>,
    pub conductor: Option<String>,
    pub notas: Option<String>,
    pub entregado_por: Option<String>,
    pub recibido_por: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ConduceItem {
    pub id: Uuid,
    pub conduce_id: Uuid,
    pub venta_item_id: Option<Uuid>,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub cantidad: Decimal,
    pub unidad: Option<String>,
    pub observaciones: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ConduceConVenta {
    pub id: Uuid,
    pub venta_id: Option<Uuid>,
    pub cliente_nombre: Option<String>,
    pub direccion_entrega: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConduceItemRequest {
    /// Presente solo en el flujo ligado a una Venta (entrega parcial de una
    /// línea ya facturada).
    pub venta_item_id: Option<Uuid>,
    /// Requerido en el flujo standalone - ignorado (se toma del venta_item)
    /// cuando venta_item_id está presente.
    pub producto_id: Option<Uuid>,
    /// Etiqueta libre para una línea standalone (p.ej. "Instalación de
    /// tubería - baño 2"). Si se omite, se usa el nombre del producto.
    pub descripcion: Option<String>,
    pub cantidad: Decimal,
    pub observaciones: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateConduceRequest {
    /// Ausente = conduce standalone (ver cliente_id en su lugar).
    pub venta_id: Option<Uuid>,
    /// Solo tiene efecto cuando venta_id está ausente - el flujo ligado a
    /// Venta toma el cliente de `ventas.cliente_id`.
    pub cliente_id: Option<Uuid>,
    pub direccion_entrega: Option<String>,
    pub orden_compra: Option<String>,
    pub vehiculo_placa: Option<String>,
    pub conductor: Option<String>,
    pub notas: Option<String>,
    pub entregado_por: Option<String>,
    pub recibido_por: Option<String>,
    pub items: Vec<CreateConduceItemRequest>,
}

pub struct ConduceCompleta {
    pub conduce: Conduce,
    pub items: Vec<ConduceItem>,
}

pub struct ConduceService {
    pool: PgPool,
}

impl ConduceService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_conduce(&self, tenant_id: &str, usuario_id: Uuid, req: CreateConduceRequest) -> anyhow::Result<ConduceCompleta> {
        if req.items.is_empty() {
            anyhow::bail!("El conduce necesita al menos un producto");
        }

        let mut tx = self.pool.begin().await?;

        // lineas: (venta_item_id, producto_id, sku, nombre, cantidad, unidad, observaciones, mueve_stock)
        let mut lineas: Vec<(Option<Uuid>, Uuid, String, String, Decimal, Option<String>, Option<String>, bool)> = Vec::new();

        if let Some(venta_id) = req.venta_id {
            // ---- Flujo ligado a una Venta: entrega parcial de líneas ya facturadas ----
            let venta_estado: Option<(String, bool)> = sqlx::query_as(
                "SELECT estado, entrega_diferida FROM ventas WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
            )
            .bind(venta_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (venta_estado, entrega_diferida) = venta_estado.ok_or_else(|| anyhow::anyhow!("Venta no encontrada"))?;
            if venta_estado == "ANULADA" {
                anyhow::bail!("Esta venta está anulada - no se pueden registrar más entregas");
            }
            if !entrega_diferida {
                anyhow::bail!("Esta venta no es de entrega diferida - la mercancía ya salió completa al facturarse");
            }

            for item in &req.items {
                if item.cantidad <= Decimal::ZERO {
                    anyhow::bail!("La cantidad debe ser mayor a cero");
                }
                let venta_item_id = item.venta_item_id.ok_or_else(|| anyhow::anyhow!("Falta venta_item_id"))?;

                let venta_item: Option<(Uuid, String, String, Decimal, Decimal)> = sqlx::query_as(
                    "SELECT producto_id, sku, nombre, cantidad, cantidad_entregada FROM venta_items WHERE id = $1 AND venta_id = $2 FOR UPDATE",
                )
                .bind(venta_item_id)
                .bind(venta_id)
                .fetch_optional(&mut *tx)
                .await?;
                let (producto_id, sku, nombre, cantidad, cantidad_entregada) =
                    venta_item.ok_or_else(|| anyhow::anyhow!("Línea de venta no encontrada"))?;

                let pendiente = cantidad - cantidad_entregada;
                if item.cantidad > pendiente {
                    anyhow::bail!("Para {} solo quedan {} pendientes de entregar (solicitado {})", nombre, pendiente, item.cantidad);
                }

                let producto_row: Option<(Decimal, String)> = sqlx::query_as(
                    "SELECT stock_actual, unidad_medida FROM productos WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
                )
                .bind(producto_id)
                .bind(tenant_id)
                .fetch_optional(&mut *tx)
                .await?;
                let (stock_actual, unidad_medida) = producto_row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
                if stock_actual < item.cantidad {
                    anyhow::bail!("Stock insuficiente para {}: disponible {}, solicitado {}", nombre, stock_actual, item.cantidad);
                }

                sqlx::query("UPDATE productos SET stock_actual = stock_actual - $1, updated_at = NOW() WHERE id = $2")
                    .bind(item.cantidad)
                    .bind(producto_id)
                    .execute(&mut *tx)
                    .await?;

                sqlx::query("UPDATE venta_items SET cantidad_entregada = cantidad_entregada + $1 WHERE id = $2")
                    .bind(item.cantidad)
                    .bind(venta_item_id)
                    .execute(&mut *tx)
                    .await?;

                lineas.push((Some(venta_item_id), producto_id, sku, nombre, item.cantidad, Some(unidad_medida), item.observaciones.clone(), true));
            }
        } else {
            // ---- Flujo standalone: sin Venta previa, cada línea trae su propio producto ----
            for item in &req.items {
                if item.cantidad <= Decimal::ZERO {
                    anyhow::bail!("La cantidad debe ser mayor a cero");
                }
                let producto_id = item.producto_id.ok_or_else(|| anyhow::anyhow!("Falta producto_id"))?;

                let producto_row: Option<(String, String, Decimal, String, String)> = sqlx::query_as(
                    "SELECT sku, nombre, stock_actual, unidad_medida, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true FOR UPDATE",
                )
                .bind(producto_id)
                .bind(tenant_id)
                .fetch_optional(&mut *tx)
                .await?;
                let (sku, nombre_producto, stock_actual, unidad_medida, tipo) =
                    producto_row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
                let nombre = item.descripcion.clone().unwrap_or_else(|| nombre_producto.clone());

                let mueve_stock = tipo != "SERVICIO";
                if mueve_stock {
                    if stock_actual < item.cantidad {
                        anyhow::bail!("Stock insuficiente para {}: disponible {}, solicitado {}", nombre_producto, stock_actual, item.cantidad);
                    }
                    sqlx::query("UPDATE productos SET stock_actual = stock_actual - $1, updated_at = NOW() WHERE id = $2")
                        .bind(item.cantidad)
                        .bind(producto_id)
                        .execute(&mut *tx)
                        .await?;
                }

                lineas.push((None, producto_id, sku, nombre, item.cantidad, Some(unidad_medida), item.observaciones.clone(), mueve_stock));
            }
        }

        let conduce = sqlx::query_as::<_, Conduce>(
            r#"INSERT INTO conduces (tenant_id, venta_id, cliente_id, usuario_id, direccion_entrega, orden_compra, vehiculo_placa, conductor, notas, entregado_por, recibido_por)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
               RETURNING id, tenant_id, venta_id, cliente_id, usuario_id, direccion_entrega, orden_compra, vehiculo_placa, conductor, notas, entregado_por, recibido_por, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.venta_id)
        .bind(if req.venta_id.is_none() { req.cliente_id } else { None })
        .bind(usuario_id)
        .bind(&req.direccion_entrega)
        .bind(&req.orden_compra)
        .bind(&req.vehiculo_placa)
        .bind(&req.conductor)
        .bind(&req.notas)
        .bind(&req.entregado_por)
        .bind(&req.recibido_por)
        .fetch_one(&mut *tx)
        .await?;

        let mut items = Vec::new();
        for (venta_item_id, producto_id, sku, nombre, cantidad, unidad, observaciones, mueve_stock) in lineas {
            let ci = sqlx::query_as::<_, ConduceItem>(
                r#"INSERT INTO conduce_items (conduce_id, venta_item_id, producto_id, sku, nombre, cantidad, unidad, observaciones)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                   RETURNING id, conduce_id, venta_item_id, producto_id, sku, nombre, cantidad, unidad, observaciones"#,
            )
            .bind(conduce.id)
            .bind(venta_item_id)
            .bind(producto_id)
            .bind(&sku)
            .bind(&nombre)
            .bind(cantidad)
            .bind(&unidad)
            .bind(&observaciones)
            .fetch_one(&mut *tx)
            .await?;
            items.push(ci);

            if mueve_stock {
                sqlx::query(
                    r#"INSERT INTO movimientos_inventario (tenant_id, producto_id, tipo, cantidad, costo_unitario, motivo, referencia_tipo, referencia_id, usuario_id)
                       VALUES ($1, $2, 'SALIDA', $3, NULL, 'Conduce', 'CONDUCE', $4, $5)"#,
                )
                .bind(tenant_id)
                .bind(producto_id)
                .bind(-cantidad)
                .bind(conduce.id)
                .bind(usuario_id)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(ConduceCompleta { conduce, items })
    }

    const CONDUCES_SORTABLE: &'static [(&'static str, &'static str)] = &[("created_at", "c.created_at")];

    pub async fn list_conduces(
        &self,
        tenant_id: &str,
        venta_id: Option<Uuid>,
        search: Option<String>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<ConduceConVenta>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        // LEFT JOIN en vez de JOIN: un conduce standalone no tiene venta_id.
        // cliente_nombre sale de conduces.cliente_id (standalone) o, si no,
        // de ventas.cliente_id (flujo ligado) - COALESCE entre ambos.
        const WHERE_CLAUSE: &str = "WHERE c.tenant_id = $1
               AND ($2::uuid IS NULL OR c.venta_id = $2)
               AND ($3::text IS NULL OR LOWER(COALESCE(cl.nombre, cl_v.nombre)) LIKE $3 OR LOWER(c.direccion_entrega) LIKE $3)
               AND ($4::date IS NULL OR c.created_at::date >= $4)
               AND ($5::date IS NULL OR c.created_at::date <= $5)";

        let total: i64 = sqlx::query_scalar(&format!(
            r#"SELECT COUNT(*) FROM conduces c
               LEFT JOIN ventas v ON v.id = c.venta_id
               LEFT JOIN clientes cl ON cl.id = c.cliente_id
               LEFT JOIN clientes cl_v ON cl_v.id = v.cliente_id
               {WHERE_CLAUSE}"#
        ))
        .bind(tenant_id)
        .bind(venta_id)
        .bind(&pattern)
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::CONDUCES_SORTABLE, "c.created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT c.id, c.venta_id, COALESCE(cl.nombre, cl_v.nombre) AS cliente_nombre, c.direccion_entrega, c.created_at
               FROM conduces c
               LEFT JOIN ventas v ON v.id = c.venta_id
               LEFT JOIN clientes cl ON cl.id = c.cliente_id
               LEFT JOIN clientes cl_v ON cl_v.id = v.cliente_id
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $6 OFFSET $7"#
        );
        let rows = sqlx::query_as::<_, ConduceConVenta>(&query)
            .bind(tenant_id)
            .bind(venta_id)
            .bind(&pattern)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn get_conduce(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<ConduceCompleta> {
        let conduce = sqlx::query_as::<_, Conduce>(
            r#"SELECT id, tenant_id, venta_id, cliente_id, usuario_id, direccion_entrega, orden_compra, vehiculo_placa, conductor, notas, entregado_por, recibido_por, created_at
               FROM conduces WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Conduce no encontrado"))?;

        let items = sqlx::query_as::<_, ConduceItem>(
            "SELECT id, conduce_id, venta_item_id, producto_id, sku, nombre, cantidad, unidad, observaciones FROM conduce_items WHERE conduce_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(ConduceCompleta { conduce, items })
    }
}

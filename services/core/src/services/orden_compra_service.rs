//! Órdenes de Compra (Purchase Orders) - Módulo 15b
//! `compras` sigue representando exclusivamente una compra YA recibida (ver
//! compras_service::create_compra, sin estado propio) - esta tabla captura
//! la INTENCIÓN previa a recibir. Recibir una orden de compra (total o
//! parcialmente) es lo que efectivamente crea la fila en `compras` vía el
//! servicio existente; esta tabla nunca toca inventario directamente,
//! orquestado en main.rs igual que la conversión de cotización a venta.
//! El proveedor vive por línea (opcional) y no en la orden - una orden
//! puede mezclar productos de varios proveedores o de ninguno.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

const ORDEN_COLUMNS: &str = "id, tenant_id, proveedor_id, orden_servicio_id, estado, subtotal, itbis_total, total, fecha, fecha_esperada, notas, usuario_id, created_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenCompra {
    pub id: Uuid,
    pub tenant_id: String,
    pub proveedor_id: Option<Uuid>,
    pub orden_servicio_id: Option<Uuid>,
    pub estado: String,
    pub subtotal: Decimal,
    pub itbis_total: Decimal,
    pub total: Decimal,
    pub fecha: NaiveDate,
    pub fecha_esperada: Option<NaiveDate>,
    pub notas: Option<String>,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenCompraItem {
    pub id: Uuid,
    pub orden_compra_id: Uuid,
    pub producto_id: Uuid,
    pub proveedor_id: Option<Uuid>,
    pub sku: String,
    pub nombre: String,
    pub cantidad_solicitada: Decimal,
    pub cantidad_recibida: Decimal,
    pub costo_unitario: Decimal,
}

const ITEM_COLUMNS: &str = "id, orden_compra_id, producto_id, proveedor_id, sku, nombre, cantidad_solicitada, cantidad_recibida, costo_unitario";

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenCompraConProveedor {
    pub id: Uuid,
    pub proveedor_nombre: Option<String>,
    pub estado: String,
    pub total: Decimal,
    pub fecha_esperada: Option<NaiveDate>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrdenCompraItemRequest {
    pub producto_id: Uuid,
    pub proveedor_id: Option<Uuid>,
    pub cantidad_solicitada: Decimal,
    pub costo_unitario: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrdenCompraRequest {
    pub orden_servicio_id: Option<Uuid>,
    pub fecha_esperada: Option<NaiveDate>,
    pub notas: Option<String>,
    pub items: Vec<CreateOrdenCompraItemRequest>,
}

/// Una línea recibida en esta llamada: puede ser menos que lo solicitado
/// (recepción parcial) - lo ya recibido en llamadas anteriores se acumula.
#[derive(Debug, Deserialize)]
pub struct RecibirItemRequest {
    pub item_id: Uuid,
    pub cantidad: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct RecibirOrdenCompraRequest {
    pub items: Vec<RecibirItemRequest>,
    pub ncf_proveedor: Option<String>,
    pub metodo_pago: Option<String>,
    pub fecha_vencimiento: Option<NaiveDate>,
}

pub struct OrdenCompraCompleta {
    pub orden: OrdenCompra,
    pub items: Vec<OrdenCompraItem>,
}

/// Resultado de recibir(): las cantidades ya validadas y agrupadas por
/// producto, listas para pasarle a compras_service::create_compra en
/// main.rs (este servicio no depende de compras_service directamente para
/// no crear un ciclo entre módulos de servicio).
pub struct RecepcionLista {
    pub orden: OrdenCompra,
    pub lineas: Vec<(Uuid, Decimal, Decimal, Option<Uuid>)>, // producto_id, cantidad_recibida_ahora, costo_unitario, proveedor_id
}

pub struct OrdenCompraService {
    pool: PgPool,
}

impl OrdenCompraService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_orden_compra(&self, tenant_id: &str, usuario_id: Uuid, req: CreateOrdenCompraRequest) -> anyhow::Result<OrdenCompraCompleta> {
        if req.items.is_empty() {
            anyhow::bail!("La orden de compra necesita al menos un producto");
        }
        let mut tx = self.pool.begin().await?;

        let mut subtotal_total = Decimal::ZERO;
        let mut lineas: Vec<(Uuid, Option<Uuid>, String, String, Decimal, Decimal)> = Vec::new();
        for item in &req.items {
            if item.cantidad_solicitada <= Decimal::ZERO || item.costo_unitario < Decimal::ZERO {
                anyhow::bail!("Cantidad o costo inválido");
            }
            let row: Option<(String, String, String)> = sqlx::query_as(
                "SELECT sku, nombre, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
            )
            .bind(item.producto_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (sku, nombre, tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
            if tipo == "SERVICIO" {
                anyhow::bail!("No se pueden ordenar servicios en una orden de compra - usa Gastos para este concepto");
            }
            subtotal_total += item.costo_unitario * item.cantidad_solicitada;
            lineas.push((item.producto_id, item.proveedor_id, sku, nombre, item.cantidad_solicitada, item.costo_unitario));
        }
        // El ITBIS real de una compra se fija al recibir (puede variar por
        // línea vía itbis_tipo, ver compras_service) - aquí el total es un
        // estimado de referencia para el usuario antes de enviar la orden.
        let total = subtotal_total;

        let orden = sqlx::query_as::<_, OrdenCompra>(&format!(
            r#"INSERT INTO ordenes_compra (tenant_id, orden_servicio_id, subtotal, total, fecha_esperada, notas, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING {ORDEN_COLUMNS}"#
        ))
        .bind(tenant_id)
        .bind(req.orden_servicio_id)
        .bind(subtotal_total)
        .bind(total)
        .bind(req.fecha_esperada)
        .bind(&req.notas)
        .bind(usuario_id)
        .fetch_one(&mut *tx)
        .await?;

        let mut items = Vec::new();
        for (producto_id, proveedor_id, sku, nombre, cantidad, costo_unitario) in lineas {
            let it = sqlx::query_as::<_, OrdenCompraItem>(&format!(
                r#"INSERT INTO orden_compra_items (orden_compra_id, producto_id, proveedor_id, sku, nombre, cantidad_solicitada, costo_unitario)
                   VALUES ($1, $2, $3, $4, $5, $6, $7)
                   RETURNING {ITEM_COLUMNS}"#
            ))
            .bind(orden.id)
            .bind(producto_id)
            .bind(proveedor_id)
            .bind(&sku)
            .bind(&nombre)
            .bind(cantidad)
            .bind(costo_unitario)
            .fetch_one(&mut *tx)
            .await?;
            items.push(it);
        }

        tx.commit().await?;
        Ok(OrdenCompraCompleta { orden, items })
    }

    pub async fn get_orden_compra(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<OrdenCompraCompleta> {
        let orden = sqlx::query_as::<_, OrdenCompra>(&format!("SELECT {ORDEN_COLUMNS} FROM ordenes_compra WHERE id = $1 AND tenant_id = $2"))
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Orden de compra no encontrada"))?;
        let items = sqlx::query_as::<_, OrdenCompraItem>(&format!(
            "SELECT {ITEM_COLUMNS} FROM orden_compra_items WHERE orden_compra_id = $1"
        ))
        .bind(id)
        .fetch_all(&self.pool)
        .await?;
        Ok(OrdenCompraCompleta { orden, items })
    }

    const ORDENES_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "o.created_at"),
        ("total", "o.total"),
        ("fecha_esperada", "o.fecha_esperada"),
        ("estado", "o.estado"),
    ];

    pub async fn list_ordenes_compra(
        &self,
        tenant_id: &str,
        estado: Option<String>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<OrdenCompraConProveedor>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE o.tenant_id = $1 AND ($2::text IS NULL OR o.estado = $2)";
        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM ordenes_compra o {WHERE_CLAUSE}"
        ))
        .bind(tenant_id)
        .bind(&estado)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::ORDENES_SORTABLE, "o.created_at DESC");
        let limit = page.limit(20);
        // El proveedor ahora vive por línea (orden_compra_items.proveedor_id,
        // opcional) - se agrega aquí para el listado: sin proveedor en
        // ninguna línea -> NULL, un único proveedor distinto -> su nombre,
        // más de uno -> "Varios proveedores".
        let query = format!(
            r#"SELECT o.id,
                      (SELECT CASE
                          WHEN COUNT(DISTINCT oi.proveedor_id) = 0 THEN NULL
                          WHEN COUNT(DISTINCT oi.proveedor_id) = 1 THEN MAX(p.nombre)
                          ELSE 'Varios proveedores'
                       END
                       FROM orden_compra_items oi
                       LEFT JOIN proveedores p ON p.id = oi.proveedor_id
                       WHERE oi.orden_compra_id = o.id AND oi.proveedor_id IS NOT NULL) AS proveedor_nombre,
                      o.estado, o.total, o.fecha_esperada, o.created_at
               FROM ordenes_compra o
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $3 OFFSET $4"#
        );
        let rows = sqlx::query_as::<_, OrdenCompraConProveedor>(&query)
            .bind(tenant_id)
            .bind(&estado)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    /// Valida y acumula la recepción, pero NO crea la compra - eso lo hace
    /// el handler en main.rs llamando a compras_service::create_compra con
    /// `RecepcionLista::lineas`, para no acoplar este servicio a compras_service.
    pub async fn preparar_recepcion(&self, tenant_id: &str, orden_id: Uuid, req: &RecibirOrdenCompraRequest) -> anyhow::Result<RecepcionLista> {
        if req.items.is_empty() {
            anyhow::bail!("Debes indicar al menos una línea a recibir");
        }
        let mut tx = self.pool.begin().await?;
        let orden: OrdenCompra = sqlx::query_as(&format!("SELECT {ORDEN_COLUMNS} FROM ordenes_compra WHERE id = $1 AND tenant_id = $2 FOR UPDATE"))
            .bind(orden_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Orden de compra no encontrada"))?;
        if orden.estado == "RECIBIDA" || orden.estado == "CANCELADA" {
            anyhow::bail!("Esta orden de compra ya está {}", orden.estado.to_lowercase());
        }

        let mut lineas = Vec::new();
        for r in &req.items {
            if r.cantidad <= Decimal::ZERO {
                anyhow::bail!("La cantidad a recibir debe ser mayor a cero");
            }
            let item: Option<(Uuid, Decimal, Decimal, Decimal, Option<Uuid>)> = sqlx::query_as(
                "SELECT producto_id, cantidad_solicitada, cantidad_recibida, costo_unitario, proveedor_id FROM orden_compra_items WHERE id = $1 AND orden_compra_id = $2 FOR UPDATE",
            )
            .bind(r.item_id)
            .bind(orden_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (producto_id, cantidad_solicitada, cantidad_recibida_previa, costo_unitario, proveedor_id) =
                item.ok_or_else(|| anyhow::anyhow!("Línea de orden de compra no encontrada"))?;
            let nueva_recibida = cantidad_recibida_previa + r.cantidad;
            if nueva_recibida > cantidad_solicitada {
                anyhow::bail!("No puedes recibir más de lo solicitado para este producto");
            }
            sqlx::query("UPDATE orden_compra_items SET cantidad_recibida = $1 WHERE id = $2")
                .bind(nueva_recibida)
                .bind(r.item_id)
                .execute(&mut *tx)
                .await?;
            lineas.push((producto_id, r.cantidad, costo_unitario, proveedor_id));
        }

        let pendientes: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM orden_compra_items WHERE orden_compra_id = $1 AND cantidad_recibida < cantidad_solicitada",
        )
        .bind(orden_id)
        .fetch_one(&mut *tx)
        .await?;
        let nuevo_estado = if pendientes == 0 { "RECIBIDA" } else { "RECIBIDA_PARCIAL" };

        let orden_actualizada = sqlx::query_as::<_, OrdenCompra>(&format!(
            "UPDATE ordenes_compra SET estado = $1 WHERE id = $2 RETURNING {ORDEN_COLUMNS}"
        ))
        .bind(nuevo_estado)
        .bind(orden_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(RecepcionLista { orden: orden_actualizada, lineas })
    }

    pub async fn cancelar(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<OrdenCompra> {
        let orden = sqlx::query_as::<_, OrdenCompra>(&format!(
            "UPDATE ordenes_compra SET estado = 'CANCELADA' WHERE id = $1 AND tenant_id = $2 AND estado NOT IN ('RECIBIDA', 'CANCELADA') RETURNING {ORDEN_COLUMNS}"
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("No se puede cancelar esta orden de compra"))?;
        Ok(orden)
    }
}

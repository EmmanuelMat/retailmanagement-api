//! Inventario Service - kardex de movimientos, ajusta productos.stock_actual
//! Plain Postgres: cada movimiento es una fila + un UPDATE atómico del stock
//! del producto dentro de la misma transacción (con FOR UPDATE para evitar
//! carreras entre dos ventas concurrentes del mismo producto).

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Movimiento {
    pub id: Uuid,
    pub tenant_id: String,
    pub producto_id: Uuid,
    pub tipo: String,
    pub cantidad: Decimal,
    pub costo_unitario: Option<Decimal>,
    pub motivo: Option<String>,
    pub referencia_tipo: Option<String>,
    pub referencia_id: Option<Uuid>,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MovimientoConProducto {
    pub id: Uuid,
    pub producto_id: Uuid,
    pub producto_nombre: String,
    pub producto_sku: String,
    pub tipo: String,
    pub cantidad: Decimal,
    pub costo_unitario: Option<Decimal>,
    pub motivo: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMovimientoRequest {
    pub producto_id: Uuid,
    pub tipo: String,
    /// ENTRADA/SALIDA: siempre positiva (la dirección la decide `tipo`).
    /// AJUSTE: puede ser positiva o negativa.
    pub cantidad: Decimal,
    pub costo_unitario: Option<Decimal>,
    pub motivo: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ResumenInventario {
    pub total_productos: i64,
    pub valor_total: Decimal,
    pub bajo_minimo: i64,
}

pub struct InventarioService {
    pool: PgPool,
}

impl InventarioService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const MOVIMIENTOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "m.created_at"),
        ("cantidad", "m.cantidad"),
    ];

    pub async fn list_movimientos(
        &self,
        tenant_id: &str,
        producto_id: Option<Uuid>,
        tipo: Option<String>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<MovimientoConProducto>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE m.tenant_id = $1
               AND ($2::uuid IS NULL OR m.producto_id = $2)
               AND ($3::text IS NULL OR m.tipo = $3)
               AND ($4::date IS NULL OR m.created_at::date >= $4)
               AND ($5::date IS NULL OR m.created_at::date <= $5)";

        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM movimientos_inventario m {WHERE_CLAUSE}"
        ))
        .bind(tenant_id)
        .bind(producto_id)
        .bind(&tipo)
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::MOVIMIENTOS_SORTABLE, "m.created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT m.id, m.producto_id, p.nombre AS producto_nombre, p.sku AS producto_sku,
                      m.tipo, m.cantidad, m.costo_unitario, m.motivo, m.created_at
               FROM movimientos_inventario m
               JOIN productos p ON p.id = m.producto_id
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $6 OFFSET $7"#
        );
        let rows = sqlx::query_as::<_, MovimientoConProducto>(&query)
            .bind(tenant_id)
            .bind(producto_id)
            .bind(&tipo)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    /// Manual movement from the UI — solo AJUSTE. ENTRADA/SALIDA nunca se
    /// crean a mano: salen automáticamente de ventas/compras/notas de crédito.
    pub async fn create_movimiento(&self, tenant_id: &str, usuario_id: Uuid, req: CreateMovimientoRequest) -> anyhow::Result<Movimiento> {
        if req.tipo != "AJUSTE" {
            anyhow::bail!(
                "Solo se permiten ajustes manuales de inventario; ENTRADA/SALIDA se generan automáticamente por ventas, compras y devoluciones"
            );
        }
        if req.cantidad == Decimal::ZERO {
            anyhow::bail!("La cantidad no puede ser cero");
        }
        let tipo_producto: String = sqlx::query_scalar("SELECT tipo FROM productos WHERE id = $1 AND tenant_id = $2")
            .bind(req.producto_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
        if tipo_producto == "SERVICIO" {
            anyhow::bail!("Un Servicio no tiene stock que ajustar");
        }
        let mut tx = self.pool.begin().await?;
        let mov = Self::apply_movimiento_tx(
            &mut tx,
            tenant_id,
            Some(usuario_id),
            req.producto_id,
            &req.tipo,
            req.cantidad, // AJUSTE: el usuario controla el signo
            req.costo_unitario,
            req.motivo,
            None,
            None,
        )
        .await?;
        tx.commit().await?;
        Ok(mov)
    }

    /// FOR UPDATE + chequeo de stock insuficiente + UPDATE de
    /// `productos.stock_actual`. `delta` ya viene con signo (positivo
    /// aumenta stock, negativo lo descuenta). No abre ni cierra transacción:
    /// corre dentro de la del caller.
    pub async fn adjust_stock_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: &str,
        producto_id: Uuid,
        delta: Decimal,
    ) -> anyhow::Result<()> {
        let current: Option<(Decimal,)> = sqlx::query_as(
            "SELECT stock_actual FROM productos WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
        )
        .bind(producto_id)
        .bind(tenant_id)
        .fetch_optional(&mut **tx)
        .await?;
        let current_stock = current.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?.0;
        let new_stock = current_stock + delta;
        if new_stock < Decimal::ZERO {
            anyhow::bail!(
                "Stock insuficiente: disponible {}, se intenta descontar {}",
                current_stock,
                delta.abs()
            );
        }

        sqlx::query("UPDATE productos SET stock_actual = $1, updated_at = NOW() WHERE id = $2")
            .bind(new_stock)
            .bind(producto_id)
            .execute(&mut **tx)
            .await?;
        Ok(())
    }

    /// Únicamente el INSERT en `movimientos_inventario` — no toca stock.
    /// Compartido por ventas/compras/ajustes para no duplicar el shape del
    /// INSERT en cada service. No abre ni cierra transacción.
    #[allow(clippy::too_many_arguments)]
    pub async fn insert_movimiento_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: &str,
        usuario_id: Option<Uuid>,
        producto_id: Uuid,
        tipo: &str,
        cantidad: Decimal,
        costo_unitario: Option<Decimal>,
        motivo: Option<String>,
        referencia_tipo: Option<&str>,
        referencia_id: Option<Uuid>,
    ) -> anyhow::Result<Movimiento> {
        let mov = sqlx::query_as::<_, Movimiento>(
            r#"INSERT INTO movimientos_inventario (tenant_id, producto_id, tipo, cantidad, costo_unitario, motivo, referencia_tipo, referencia_id, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING id, tenant_id, producto_id, tipo, cantidad, costo_unitario, motivo, referencia_tipo, referencia_id, usuario_id, created_at"#,
        )
        .bind(tenant_id)
        .bind(producto_id)
        .bind(tipo)
        .bind(cantidad)
        .bind(costo_unitario)
        .bind(&motivo)
        .bind(referencia_tipo)
        .bind(referencia_id)
        .bind(usuario_id)
        .fetch_one(&mut **tx)
        .await?;
        Ok(mov)
    }

    /// Compone `adjust_stock_tx` + `insert_movimiento_tx` para el caso simple
    /// de un solo paso (ajuste manual, nota de crédito): mueve stock e
    /// inserta el movimiento en la misma llamada, dentro de la tx del caller.
    /// Ventas/Compras NO usan esto — su UPDATE de stock está fusionado con
    /// otra lectura/cálculo (precio+lock, costo promedio) y ocurre antes de
    /// que exista el id de referencia, así que llaman los dos helpers por
    /// separado en pasos distintos.
    #[allow(clippy::too_many_arguments)]
    pub async fn apply_movimiento_tx(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        tenant_id: &str,
        usuario_id: Option<Uuid>,
        producto_id: Uuid,
        tipo: &str,
        cantidad: Decimal,
        costo_unitario: Option<Decimal>,
        motivo: Option<String>,
        referencia_tipo: Option<&str>,
        referencia_id: Option<Uuid>,
    ) -> anyhow::Result<Movimiento> {
        Self::adjust_stock_tx(tx, tenant_id, producto_id, cantidad).await?;
        Self::insert_movimiento_tx(
            tx,
            tenant_id,
            usuario_id,
            producto_id,
            tipo,
            cantidad,
            costo_unitario,
            motivo,
            referencia_tipo,
            referencia_id,
        )
        .await
    }

    pub async fn resumen(&self, tenant_id: &str) -> anyhow::Result<ResumenInventario> {
        let row: (i64, Option<Decimal>, i64) = sqlx::query_as(
            r#"SELECT COUNT(*)::bigint,
                      SUM(stock_actual * costo),
                      COUNT(*) FILTER (WHERE stock_actual <= stock_minimo)::bigint
               FROM productos WHERE tenant_id = $1 AND activo = true"#,
        )
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(ResumenInventario {
            total_productos: row.0,
            valor_total: row.1.unwrap_or_default(),
            bajo_minimo: row.2,
        })
    }
}

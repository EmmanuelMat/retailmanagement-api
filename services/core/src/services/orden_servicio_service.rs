//! Órdenes de Servicio (Work Orders) - Módulo 15
//! Entidad separada de `conduces` (que vuelve a ser solo la guía de despacho
//! para todo tipo de tenant). Ciclo de vida: BORRADOR -> PROGRAMADA ->
//! EN_PROCESO <-> PAUSADA -> COMPLETADA, o CANCELADA desde cualquier estado
//! activo. Facturar (crear la Venta) se orquesta en main.rs reutilizando
//! `ventas_service::create_venta` tal cual - este servicio nunca inserta en
//! `ventas`, igual que `cotizacion_service` no lo hace hoy.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::inventario_service::InventarioService;

fn itbis_rate(tipo: &str) -> Decimal {
    match tipo {
        "GRAVADO_18" => dec!(0.18),
        "GRAVADO_16" => dec!(0.16),
        _ => dec!(0),
    }
}

const PRIORIDADES: &[&str] = &["BAJA", "NORMAL", "ALTA", "URGENTE"];
const ESTADOS_CANCELABLES: &[&str] = &["BORRADOR", "PROGRAMADA", "EN_PROCESO", "PAUSADA"];

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenServicio {
    pub id: Uuid,
    pub tenant_id: String,
    pub cliente_id: Option<Uuid>,
    pub cotizacion_id: Option<Uuid>,
    pub venta_id: Option<Uuid>,
    pub condicion_id: Option<Uuid>,
    pub estado: String,
    pub prioridad: String,
    pub fecha: NaiveDate,
    pub fecha_programada: Option<NaiveDate>,
    pub hora_inicio: Option<NaiveTime>,
    pub hora_fin: Option<NaiveTime>,
    pub direccion: Option<String>,
    pub descripcion: Option<String>,
    pub subtotal: Decimal,
    pub descuento: Decimal,
    pub itbis_total: Decimal,
    pub total: Decimal,
    pub notas: Option<String>,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

const ORDEN_COLUMNS: &str = "id, tenant_id, cliente_id, cotizacion_id, venta_id, condicion_id, estado, prioridad, \
     fecha, fecha_programada, hora_inicio, hora_fin, direccion, descripcion, subtotal, descuento, itbis_total, total, \
     notas, usuario_id, created_at, updated_at";

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenServicioItem {
    pub id: Uuid,
    pub orden_servicio_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub tipo: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub descuento: Decimal,
    pub itbis_tipo: String,
    pub itbis_monto: Decimal,
    pub subtotal: Decimal,
    pub tecnico_id: Option<Uuid>,
    pub observaciones: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenServicioTecnico {
    pub id: Uuid,
    pub orden_servicio_id: Uuid,
    pub empleado_id: Uuid,
    pub rol: String,
    pub fecha_asignacion: DateTime<Utc>,
    pub fecha_inicio: Option<DateTime<Utc>>,
    pub fecha_fin: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenServicioMaterial {
    pub id: Uuid,
    pub orden_servicio_id: Uuid,
    pub producto_id: Uuid,
    pub cantidad_planificada: Decimal,
    pub cantidad_utilizada: Decimal,
    pub costo_unitario: Option<Decimal>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenServicioNota {
    pub id: Uuid,
    pub orden_servicio_id: Uuid,
    pub tipo: String,
    pub contenido: String,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenServicioConCliente {
    pub id: Uuid,
    pub cliente_nombre: Option<String>,
    pub estado: String,
    pub prioridad: String,
    pub condicion_nombre: Option<String>,
    pub fecha_programada: Option<NaiveDate>,
    pub total: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrdenServicioItemRequest {
    pub producto_id: Uuid,
    pub cantidad: Decimal,
    pub descuento: Option<Decimal>,
    /// Requerido cuando el producto es tipo SERVICIO - ver el mismo campo en
    /// cotizacion_service::CreateCotizacionItemRequest.
    pub precio_unitario: Option<Decimal>,
    pub tecnico_id: Option<Uuid>,
    pub observaciones: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrdenServicioRequest {
    pub cliente_id: Option<Uuid>,
    pub cotizacion_id: Option<Uuid>,
    pub condicion_id: Option<Uuid>,
    pub prioridad: Option<String>,
    pub fecha_programada: Option<NaiveDate>,
    pub direccion: Option<String>,
    pub descripcion: Option<String>,
    pub notas: Option<String>,
    pub items: Vec<CreateOrdenServicioItemRequest>,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateOrdenServicioRequest {
    pub condicion_id: Option<Uuid>,
    pub prioridad: Option<String>,
    pub fecha_programada: Option<NaiveDate>,
    pub hora_inicio: Option<NaiveTime>,
    pub hora_fin: Option<NaiveTime>,
    pub direccion: Option<String>,
    pub descripcion: Option<String>,
    pub notas: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AsignarTecnicoRequest {
    pub empleado_id: Uuid,
    pub rol: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AgregarMaterialRequest {
    pub producto_id: Uuid,
    pub cantidad_planificada: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct ConsumirMaterialRequest {
    pub cantidad: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct AgregarNotaRequest {
    pub tipo: Option<String>,
    pub contenido: String,
}

pub struct OrdenServicioCompleta {
    pub orden: OrdenServicio,
    pub items: Vec<OrdenServicioItem>,
    pub tecnicos: Vec<OrdenServicioTecnico>,
    pub materiales: Vec<OrdenServicioMaterial>,
    pub notas: Vec<OrdenServicioNota>,
}

pub struct OrdenServicioService {
    pool: PgPool,
}

impl OrdenServicioService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_orden(&self, tenant_id: &str, usuario_id: Uuid, req: CreateOrdenServicioRequest) -> anyhow::Result<OrdenServicioCompleta> {
        if req.items.is_empty() {
            anyhow::bail!("La orden de servicio necesita al menos un producto o servicio");
        }
        let prioridad = req.prioridad.unwrap_or_else(|| "NORMAL".to_string());
        if !PRIORIDADES.contains(&prioridad.as_str()) {
            anyhow::bail!("Prioridad inválida: {}", prioridad);
        }

        let mut tx = self.pool.begin().await?;

        let mut subtotal_total = Decimal::ZERO;
        let mut itbis_total = Decimal::ZERO;
        let mut descuento_total = Decimal::ZERO;
        #[allow(clippy::type_complexity)]
        let mut lineas: Vec<(Uuid, String, String, String, Decimal, Decimal, Decimal, String, Decimal, Decimal, Option<Uuid>, Option<String>)> = Vec::new();

        for item in &req.items {
            if item.cantidad <= Decimal::ZERO {
                anyhow::bail!("La cantidad debe ser mayor a cero");
            }
            let row: Option<(String, String, Option<Decimal>, String, String)> = sqlx::query_as(
                "SELECT sku, nombre, precio_venta, itbis_tipo, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
            )
            .bind(item.producto_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (sku, nombre, precio_venta_catalogo, itbis_tipo, tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;

            let precio_unitario = if tipo == "SERVICIO" {
                let precio = item.precio_unitario.ok_or_else(|| anyhow::anyhow!("{} es un servicio: falta precio_unitario para esta línea", nombre))?;
                if precio <= Decimal::ZERO {
                    anyhow::bail!("precio_unitario inválido para {}", nombre);
                }
                precio
            } else {
                precio_venta_catalogo.unwrap_or_default()
            };

            let descuento = item.descuento.unwrap_or_default();
            let line_bruto = precio_unitario * item.cantidad;
            if descuento < Decimal::ZERO || descuento > line_bruto {
                anyhow::bail!("Descuento inválido para {}", nombre);
            }
            let line_subtotal = line_bruto - descuento;
            let line_itbis = line_subtotal * itbis_rate(&itbis_tipo);
            subtotal_total += line_subtotal;
            itbis_total += line_itbis;
            descuento_total += descuento;

            lineas.push((
                item.producto_id, sku, nombre, tipo, item.cantidad, precio_unitario, descuento,
                itbis_tipo, line_itbis, line_subtotal, item.tecnico_id, item.observaciones.clone(),
            ));
        }

        let total = subtotal_total + itbis_total;

        let orden = sqlx::query_as::<_, OrdenServicio>(&format!(
            r#"INSERT INTO ordenes_servicio
                   (tenant_id, cliente_id, cotizacion_id, condicion_id, prioridad, fecha_programada,
                    direccion, descripcion, subtotal, descuento, itbis_total, total, notas, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
               RETURNING {ORDEN_COLUMNS}"#
        ))
        .bind(tenant_id)
        .bind(req.cliente_id)
        .bind(req.cotizacion_id)
        .bind(req.condicion_id)
        .bind(&prioridad)
        .bind(req.fecha_programada)
        .bind(&req.direccion)
        .bind(&req.descripcion)
        .bind(subtotal_total)
        .bind(descuento_total)
        .bind(itbis_total)
        .bind(total)
        .bind(&req.notas)
        .bind(usuario_id)
        .fetch_one(&mut *tx)
        .await?;

        let mut items = Vec::new();
        for (producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, line_subtotal, tecnico_id, observaciones) in lineas {
            let it = sqlx::query_as::<_, OrdenServicioItem>(
                r#"INSERT INTO orden_servicio_items
                       (orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, tecnico_id, observaciones)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                   RETURNING id, orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, tecnico_id, observaciones"#,
            )
            .bind(orden.id)
            .bind(producto_id)
            .bind(&sku)
            .bind(&nombre)
            .bind(&tipo)
            .bind(cantidad)
            .bind(precio_unitario)
            .bind(descuento)
            .bind(&itbis_tipo)
            .bind(itbis_monto)
            .bind(line_subtotal)
            .bind(tecnico_id)
            .bind(&observaciones)
            .fetch_one(&mut *tx)
            .await?;
            items.push(it);
        }

        tx.commit().await?;
        Ok(OrdenServicioCompleta { orden, items, tecnicos: Vec::new(), materiales: Vec::new(), notas: Vec::new() })
    }

    pub async fn get_orden(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<OrdenServicioCompleta> {
        let orden = sqlx::query_as::<_, OrdenServicio>(&format!("SELECT {ORDEN_COLUMNS} FROM ordenes_servicio WHERE id = $1 AND tenant_id = $2"))
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Orden de servicio no encontrada"))?;

        let items = sqlx::query_as::<_, OrdenServicioItem>(
            "SELECT id, orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, tecnico_id, observaciones \
             FROM orden_servicio_items WHERE orden_servicio_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let tecnicos = sqlx::query_as::<_, OrdenServicioTecnico>(
            "SELECT id, orden_servicio_id, empleado_id, rol, fecha_asignacion, fecha_inicio, fecha_fin FROM orden_servicio_tecnicos WHERE orden_servicio_id = $1 ORDER BY fecha_asignacion",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let materiales = sqlx::query_as::<_, OrdenServicioMaterial>(
            "SELECT id, orden_servicio_id, producto_id, cantidad_planificada, cantidad_utilizada, costo_unitario FROM orden_servicio_materiales WHERE orden_servicio_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let notas = sqlx::query_as::<_, OrdenServicioNota>(
            "SELECT id, orden_servicio_id, tipo, contenido, usuario_id, created_at FROM orden_servicio_notas WHERE orden_servicio_id = $1 ORDER BY created_at DESC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(OrdenServicioCompleta { orden, items, tecnicos, materiales, notas })
    }

    const ORDENES_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "o.created_at"),
        ("total", "o.total"),
        ("fecha_programada", "o.fecha_programada"),
        ("estado", "o.estado"),
        ("prioridad", "o.prioridad"),
    ];

    #[allow(clippy::too_many_arguments)]
    pub async fn list_ordenes(
        &self,
        tenant_id: &str,
        estado: Option<String>,
        cliente_id: Option<Uuid>,
        search: Option<String>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<OrdenServicioConCliente>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE o.tenant_id = $1
               AND ($2::text IS NULL OR o.estado = $2)
               AND ($3::uuid IS NULL OR o.cliente_id = $3)
               AND ($4::text IS NULL OR LOWER(cl.nombre) LIKE $4)";

        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM ordenes_servicio o LEFT JOIN clientes cl ON cl.id = o.cliente_id {WHERE_CLAUSE}"
        ))
        .bind(tenant_id)
        .bind(&estado)
        .bind(cliente_id)
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::ORDENES_SORTABLE, "o.created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT o.id, cl.nombre AS cliente_nombre, o.estado, o.prioridad, co.nombre AS condicion_nombre, o.fecha_programada, o.total, o.created_at
               FROM ordenes_servicio o
               LEFT JOIN clientes cl ON cl.id = o.cliente_id
               LEFT JOIN condiciones_orden co ON co.id = o.condicion_id
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $5 OFFSET $6"#
        );
        let rows = sqlx::query_as::<_, OrdenServicioConCliente>(&query)
            .bind(tenant_id)
            .bind(&estado)
            .bind(cliente_id)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    async fn estado_actual(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, tenant_id: &str, id: Uuid) -> anyhow::Result<String> {
        sqlx::query_scalar("SELECT estado FROM ordenes_servicio WHERE id = $1 AND tenant_id = $2 FOR UPDATE")
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&mut **tx)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Orden de servicio no encontrada"))
    }

    pub async fn update_orden(&self, tenant_id: &str, id: Uuid, req: UpdateOrdenServicioRequest) -> anyhow::Result<OrdenServicio> {
        let mut tx = self.pool.begin().await?;
        let estado = self.estado_actual(&mut tx, tenant_id, id).await?;
        if estado == "COMPLETADA" || estado == "CANCELADA" {
            anyhow::bail!("No se puede editar una orden {}", estado.to_lowercase());
        }
        if let Some(p) = &req.prioridad {
            if !PRIORIDADES.contains(&p.as_str()) {
                anyhow::bail!("Prioridad inválida: {}", p);
            }
        }
        // Fijar una fecha_programada sobre una orden en BORRADOR la agenda.
        let nuevo_estado = if estado == "BORRADOR" && req.fecha_programada.is_some() { "PROGRAMADA" } else { estado.as_str() };

        let orden = sqlx::query_as::<_, OrdenServicio>(&format!(
            r#"UPDATE ordenes_servicio SET
                   estado = COALESCE($1, estado),
                   condicion_id = COALESCE($2, condicion_id),
                   prioridad = COALESCE($3, prioridad),
                   fecha_programada = COALESCE($4, fecha_programada),
                   hora_inicio = COALESCE($5, hora_inicio),
                   hora_fin = COALESCE($6, hora_fin),
                   direccion = COALESCE($7, direccion),
                   descripcion = COALESCE($8, descripcion),
                   notas = COALESCE($9, notas),
                   updated_at = NOW()
               WHERE id = $10 AND tenant_id = $11
               RETURNING {ORDEN_COLUMNS}"#
        ))
        .bind(nuevo_estado)
        .bind(req.condicion_id)
        .bind(&req.prioridad)
        .bind(req.fecha_programada)
        .bind(req.hora_inicio)
        .bind(req.hora_fin)
        .bind(&req.direccion)
        .bind(&req.descripcion)
        .bind(&req.notas)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(orden)
    }

    async fn recalcular_totales(&self, tx: &mut sqlx::Transaction<'_, sqlx::Postgres>, orden_id: Uuid) -> anyhow::Result<()> {
        sqlx::query(
            r#"UPDATE ordenes_servicio o SET
                   subtotal = agg.subtotal, descuento = agg.descuento, itbis_total = agg.itbis, total = agg.subtotal + agg.itbis, updated_at = NOW()
               FROM (
                   SELECT COALESCE(SUM(subtotal), 0) AS subtotal, COALESCE(SUM(descuento), 0) AS descuento, COALESCE(SUM(itbis_monto), 0) AS itbis
                   FROM orden_servicio_items WHERE orden_servicio_id = $1
               ) agg
               WHERE o.id = $1"#,
        )
        .bind(orden_id)
        .execute(&mut **tx)
        .await?;
        Ok(())
    }

    pub async fn add_item(&self, tenant_id: &str, orden_id: Uuid, item: CreateOrdenServicioItemRequest) -> anyhow::Result<OrdenServicioItem> {
        if item.cantidad <= Decimal::ZERO {
            anyhow::bail!("La cantidad debe ser mayor a cero");
        }
        let mut tx = self.pool.begin().await?;
        let estado = self.estado_actual(&mut tx, tenant_id, orden_id).await?;
        if estado == "COMPLETADA" || estado == "CANCELADA" {
            anyhow::bail!("No se puede editar una orden {}", estado.to_lowercase());
        }

        let row: Option<(String, String, Option<Decimal>, String, String)> = sqlx::query_as(
            "SELECT sku, nombre, precio_venta, itbis_tipo, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
        )
        .bind(item.producto_id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (sku, nombre, precio_venta_catalogo, itbis_tipo, tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;

        // Evita el doble descuento de stock: facturar este renglón (vía
        // ventas_service::create_venta, reusado sin cambios en crear_factura)
        // ya mueve inventario para un PRODUCTO. Si el mismo producto también
        // está anotado en orden_servicio_materiales para esta orden, el
        // consumo real ya se registró por ese otro camino - un producto por
        // orden usa un solo camino, nunca los dos.
        if tipo == "PRODUCTO" {
            let ya_es_material: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM orden_servicio_materiales WHERE orden_servicio_id = $1 AND producto_id = $2",
            )
            .bind(orden_id)
            .bind(item.producto_id)
            .fetch_optional(&mut *tx)
            .await?;
            if ya_es_material.is_some() {
                anyhow::bail!(
                    "{} ya está registrado como material consumido en esta orden - no se puede facturar también como línea (evita descontar el stock dos veces)",
                    nombre
                );
            }
        }

        let precio_unitario = if tipo == "SERVICIO" {
            item.precio_unitario.filter(|p| *p > Decimal::ZERO).ok_or_else(|| anyhow::anyhow!("{} es un servicio: falta precio_unitario para esta línea", nombre))?
        } else {
            precio_venta_catalogo.unwrap_or_default()
        };
        let descuento = item.descuento.unwrap_or_default();
        let line_bruto = precio_unitario * item.cantidad;
        if descuento < Decimal::ZERO || descuento > line_bruto {
            anyhow::bail!("Descuento inválido para {}", nombre);
        }
        let line_subtotal = line_bruto - descuento;
        let line_itbis = line_subtotal * itbis_rate(&itbis_tipo);

        let it = sqlx::query_as::<_, OrdenServicioItem>(
            r#"INSERT INTO orden_servicio_items
                   (orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, tecnico_id, observaciones)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
               RETURNING id, orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, tecnico_id, observaciones"#,
        )
        .bind(orden_id)
        .bind(item.producto_id)
        .bind(&sku)
        .bind(&nombre)
        .bind(&tipo)
        .bind(item.cantidad)
        .bind(precio_unitario)
        .bind(descuento)
        .bind(&itbis_tipo)
        .bind(line_itbis)
        .bind(line_subtotal)
        .bind(item.tecnico_id)
        .bind(&item.observaciones)
        .fetch_one(&mut *tx)
        .await?;

        self.recalcular_totales(&mut tx, orden_id).await?;
        tx.commit().await?;
        Ok(it)
    }

    pub async fn remove_item(&self, tenant_id: &str, orden_id: Uuid, item_id: Uuid) -> anyhow::Result<()> {
        let mut tx = self.pool.begin().await?;
        let estado = self.estado_actual(&mut tx, tenant_id, orden_id).await?;
        if estado == "COMPLETADA" || estado == "CANCELADA" {
            anyhow::bail!("No se puede editar una orden {}", estado.to_lowercase());
        }
        let deleted = sqlx::query("DELETE FROM orden_servicio_items WHERE id = $1 AND orden_servicio_id = $2")
            .bind(item_id)
            .bind(orden_id)
            .execute(&mut *tx)
            .await?;
        if deleted.rows_affected() == 0 {
            anyhow::bail!("Línea no encontrada");
        }
        self.recalcular_totales(&mut tx, orden_id).await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn asignar_tecnico(&self, tenant_id: &str, orden_id: Uuid, req: AsignarTecnicoRequest) -> anyhow::Result<OrdenServicioTecnico> {
        let rol = req.rol.unwrap_or_else(|| "TECNICO_PRINCIPAL".to_string());
        if rol != "TECNICO_PRINCIPAL" && rol != "ASISTENTE" {
            anyhow::bail!("Rol de técnico inválido: {}", rol);
        }
        // Verifica que la orden y el empleado pertenezcan al mismo tenant -
        // nunca confiar en un empleado_id sin validar contra tenant_id.
        let orden_existe: Option<i32> = sqlx::query_scalar("SELECT 1 FROM ordenes_servicio WHERE id = $1 AND tenant_id = $2")
            .bind(orden_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        orden_existe.ok_or_else(|| anyhow::anyhow!("Orden de servicio no encontrada"))?;
        let empleado_existe: Option<i32> = sqlx::query_scalar("SELECT 1 FROM empleados WHERE id = $1 AND tenant_id = $2 AND activo = true")
            .bind(req.empleado_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        empleado_existe.ok_or_else(|| anyhow::anyhow!("Empleado no encontrado"))?;

        let tec = sqlx::query_as::<_, OrdenServicioTecnico>(
            r#"INSERT INTO orden_servicio_tecnicos (orden_servicio_id, empleado_id, rol)
               VALUES ($1, $2, $3)
               RETURNING id, orden_servicio_id, empleado_id, rol, fecha_asignacion, fecha_inicio, fecha_fin"#,
        )
        .bind(orden_id)
        .bind(req.empleado_id)
        .bind(&rol)
        .fetch_one(&self.pool)
        .await?;
        Ok(tec)
    }

    pub async fn quitar_tecnico(&self, tenant_id: &str, orden_id: Uuid, asignacion_id: Uuid) -> anyhow::Result<()> {
        let deleted = sqlx::query(
            "DELETE FROM orden_servicio_tecnicos t USING ordenes_servicio o \
             WHERE t.id = $1 AND t.orden_servicio_id = $2 AND o.id = t.orden_servicio_id AND o.tenant_id = $3",
        )
        .bind(asignacion_id)
        .bind(orden_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;
        if deleted.rows_affected() == 0 {
            anyhow::bail!("Asignación no encontrada");
        }
        Ok(())
    }

    pub async fn agregar_material(&self, tenant_id: &str, orden_id: Uuid, req: AgregarMaterialRequest) -> anyhow::Result<OrdenServicioMaterial> {
        if req.cantidad_planificada < Decimal::ZERO {
            anyhow::bail!("Cantidad planificada inválida");
        }
        let row: Option<(String, Decimal)> = sqlx::query_as(
            "SELECT tipo, costo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
        )
        .bind(req.producto_id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        let (tipo, costo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
        if tipo == "SERVICIO" {
            anyhow::bail!("Un Servicio no es un material consumible de inventario");
        }
        let orden_existe: Option<i32> = sqlx::query_scalar("SELECT 1 FROM ordenes_servicio WHERE id = $1 AND tenant_id = $2")
            .bind(orden_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        orden_existe.ok_or_else(|| anyhow::anyhow!("Orden de servicio no encontrada"))?;
        // Espejo del chequeo en add_item: si este producto ya es una línea
        // facturable de la orden, su stock ya se moverá al facturar (vía
        // ventas_service) - no se anota también como material para no
        // descontar el stock dos veces.
        let ya_es_item: Option<i32> = sqlx::query_scalar(
            "SELECT 1 FROM orden_servicio_items WHERE orden_servicio_id = $1 AND producto_id = $2",
        )
        .bind(orden_id)
        .bind(req.producto_id)
        .fetch_optional(&self.pool)
        .await?;
        if ya_es_item.is_some() {
            anyhow::bail!("Este producto ya está facturado como línea en esta orden - no se puede registrar también como material (evita descontar el stock dos veces)");
        }

        let mat = sqlx::query_as::<_, OrdenServicioMaterial>(
            r#"INSERT INTO orden_servicio_materiales (orden_servicio_id, producto_id, cantidad_planificada, costo_unitario)
               VALUES ($1, $2, $3, $4)
               RETURNING id, orden_servicio_id, producto_id, cantidad_planificada, cantidad_utilizada, costo_unitario"#,
        )
        .bind(orden_id)
        .bind(req.producto_id)
        .bind(req.cantidad_planificada)
        .bind(costo)
        .fetch_one(&self.pool)
        .await?;
        Ok(mat)
    }

    /// Registra consumo REAL de un material (distinto de lo planificado) y
    /// mueve inventario de verdad - a diferencia de agregar_material, que
    /// solo anota intención y no toca stock. Ver InventarioService::apply_movimiento_tx.
    pub async fn consumir_material(
        &self,
        tenant_id: &str,
        orden_id: Uuid,
        material_id: Uuid,
        usuario_id: Uuid,
        req: ConsumirMaterialRequest,
    ) -> anyhow::Result<OrdenServicioMaterial> {
        if req.cantidad <= Decimal::ZERO {
            anyhow::bail!("La cantidad a consumir debe ser mayor a cero");
        }
        let mut tx = self.pool.begin().await?;
        let estado = self.estado_actual(&mut tx, tenant_id, orden_id).await?;
        if estado == "COMPLETADA" || estado == "CANCELADA" {
            anyhow::bail!("No se puede consumir material en una orden {}", estado.to_lowercase());
        }

        let material: Option<(Uuid, Decimal, Option<Decimal>)> = sqlx::query_as(
            "SELECT producto_id, cantidad_utilizada, costo_unitario FROM orden_servicio_materiales WHERE id = $1 AND orden_servicio_id = $2 FOR UPDATE",
        )
        .bind(material_id)
        .bind(orden_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (producto_id, cantidad_previa, costo_unitario) = material.ok_or_else(|| anyhow::anyhow!("Material no encontrado"))?;

        let mat = sqlx::query_as::<_, OrdenServicioMaterial>(
            r#"UPDATE orden_servicio_materiales SET cantidad_utilizada = $1 WHERE id = $2
               RETURNING id, orden_servicio_id, producto_id, cantidad_planificada, cantidad_utilizada, costo_unitario"#,
        )
        .bind(cantidad_previa + req.cantidad)
        .bind(material_id)
        .fetch_one(&mut *tx)
        .await?;

        InventarioService::apply_movimiento_tx(
            &mut tx,
            tenant_id,
            Some(usuario_id),
            producto_id,
            "SALIDA",
            -req.cantidad,
            costo_unitario,
            Some("Consumo en orden de servicio".to_string()),
            Some("ORDEN_SERVICIO"),
            Some(orden_id),
        )
        .await?;

        tx.commit().await?;
        Ok(mat)
    }

    pub async fn agregar_nota(&self, tenant_id: &str, orden_id: Uuid, usuario_id: Option<Uuid>, req: AgregarNotaRequest) -> anyhow::Result<OrdenServicioNota> {
        let tipo = req.tipo.unwrap_or_else(|| "INTERNA".to_string());
        if !["INTERNA", "TECNICO", "CLIENTE", "SISTEMA"].contains(&tipo.as_str()) {
            anyhow::bail!("Tipo de nota inválido: {}", tipo);
        }
        if req.contenido.trim().is_empty() {
            anyhow::bail!("La nota no puede estar vacía");
        }
        let orden_existe: Option<i32> = sqlx::query_scalar("SELECT 1 FROM ordenes_servicio WHERE id = $1 AND tenant_id = $2")
            .bind(orden_id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?;
        orden_existe.ok_or_else(|| anyhow::anyhow!("Orden de servicio no encontrada"))?;

        let nota = sqlx::query_as::<_, OrdenServicioNota>(
            r#"INSERT INTO orden_servicio_notas (orden_servicio_id, tipo, contenido, usuario_id)
               VALUES ($1, $2, $3, $4)
               RETURNING id, orden_servicio_id, tipo, contenido, usuario_id, created_at"#,
        )
        .bind(orden_id)
        .bind(&tipo)
        .bind(&req.contenido)
        .bind(usuario_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(nota)
    }

    async fn transicionar(&self, tenant_id: &str, id: Uuid, desde_permitidos: &[&str], hacia: &str) -> anyhow::Result<OrdenServicio> {
        let mut tx = self.pool.begin().await?;
        let estado = self.estado_actual(&mut tx, tenant_id, id).await?;
        if !desde_permitidos.contains(&estado.as_str()) {
            anyhow::bail!("No se puede pasar de {} a {}", estado, hacia);
        }
        let orden = sqlx::query_as::<_, OrdenServicio>(&format!(
            "UPDATE ordenes_servicio SET estado = $1, updated_at = NOW() WHERE id = $2 AND tenant_id = $3 RETURNING {ORDEN_COLUMNS}"
        ))
        .bind(hacia)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(orden)
    }

    pub async fn iniciar(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<OrdenServicio> {
        self.transicionar(tenant_id, id, &["BORRADOR", "PROGRAMADA", "PAUSADA"], "EN_PROCESO").await
    }

    pub async fn pausar(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<OrdenServicio> {
        self.transicionar(tenant_id, id, &["EN_PROCESO"], "PAUSADA").await
    }

    pub async fn completar(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<OrdenServicio> {
        self.transicionar(tenant_id, id, &["EN_PROCESO", "PAUSADA"], "COMPLETADA").await
    }

    pub async fn cancelar(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<OrdenServicio> {
        self.transicionar(tenant_id, id, ESTADOS_CANCELABLES, "CANCELADA").await
    }

    /// Usado por el handler de facturación en main.rs, igual que
    /// `cotizacion_service::marcar_convertida` - este servicio nunca inserta
    /// en `ventas`. Solo se permite facturar una orden ya COMPLETADA.
    pub async fn marcar_facturada(&self, tenant_id: &str, id: Uuid, venta_id: Uuid) -> anyhow::Result<()> {
        let updated = sqlx::query("UPDATE ordenes_servicio SET venta_id = $1, updated_at = NOW() WHERE id = $2 AND tenant_id = $3 AND estado = 'COMPLETADA'")
            .bind(venta_id)
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        if updated.rows_affected() == 0 {
            anyhow::bail!("Solo una orden completada puede facturarse");
        }
        Ok(())
    }
}

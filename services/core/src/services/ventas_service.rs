//! Ventas Service - Módulo 5 (Punto de Venta)
//! Crea la venta + descuenta inventario + registra ingreso de caja en una
//! sola transacción Postgres. La emisión e-CF es un paso aparte
//! (`emitir_ecf`) que reutiliza el pipeline real de firma/envío DGII ya
//! existente (ecf_builder + ecfl_service + dgii_client) — nada de eso se
//! duplica aquí.

use crate::services::ecf_service::requiere_identificacion;
use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, RoundingStrategy};
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

/// Un CAJERO pidió más descuento del que su cuenta permite sin autorización.
/// El prefijo fijo del mensaje es lo que el frontend usa para distinguir este
/// caso de cualquier otro error de venta y mostrar el popup de aprobación en
/// vez de un error genérico (ver `pos/page.tsx`).
#[derive(Debug, thiserror::Error)]
#[error("DESCUENTO_REQUIERE_APROBACION: descuento solicitado {descuento_solicitado}, tu límite sin aprobación es {limite}")]
pub struct DescuentoRequiereAprobacion {
    pub descuento_solicitado: Decimal,
    pub limite: Decimal,
}

fn itbis_rate(tipo: &str) -> Decimal {
    match tipo {
        "GRAVADO_18" => dec!(0.18),
        "GRAVADO_16" => dec!(0.16),
        _ => dec!(0),
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Venta {
    pub id: Uuid,
    pub tenant_id: String,
    pub cliente_id: Option<Uuid>,
    pub usuario_id: Option<Uuid>,
    pub subtotal: Decimal,
    pub itbis_total: Decimal,
    pub total: Decimal,
    pub metodo_pago: String,
    pub estado: String,
    pub e_ncf: Option<String>,
    pub tipo_ecf: Option<i32>,
    pub estado_dgii: Option<String>,
    pub track_id: Option<String>,
    pub codigo_seguridad: Option<String>,
    pub qr_url: Option<String>,
    /// Si es true, la mercancía no sale toda al facturar - sale en lotes vía
    /// conduces (ver conduce_service.rs). Si es false (default), el stock se
    /// descuenta de inmediato como cualquier venta normal.
    pub entrega_diferida: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VentaItem {
    pub id: Uuid,
    pub venta_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub descuento: Decimal,
    pub itbis_tipo: String,
    pub itbis_monto: Decimal,
    pub subtotal: Decimal,
    /// Cuánto de `cantidad` ha salido realmente del inventario. Para una
    /// venta normal queda igual a `cantidad` desde el momento de la venta;
    /// para una venta con `entrega_diferida` empieza en 0 y sube con cada
    /// conduce (ver conduce_service::create_conduce).
    pub cantidad_entregada: Decimal,
    /// Costo del producto (promedio ponderado) al momento de la venta -
    /// usado por contabilidad_service::sincronizar para el asiento de Costo
    /// de Ventas. NULL en filas de antes de esta columna existir.
    pub costo_unitario: Option<Decimal>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct VentaConCliente {
    pub id: Uuid,
    pub cliente_nombre: Option<String>,
    pub total: Decimal,
    pub metodo_pago: String,
    pub estado: String,
    pub e_ncf: Option<String>,
    pub estado_dgii: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVentaItemRequest {
    pub producto_id: Uuid,
    pub cantidad: Decimal,
    /// Descuento en RD$ para esta línea (no porcentaje) - el frontend calcula
    /// el monto antes de enviar. Se resta del subtotal de la línea antes del ITBIS.
    pub descuento: Option<Decimal>,
    /// Requerido cuando el producto es tipo SERVICIO (sin precio fijo en el
    /// catálogo - ver catalog_service::Producto). Ignorado para un producto
    /// normal: el precio siempre viene de `productos.precio_venta`, nunca del
    /// cliente, para no permitir manipular el precio de un ítem con precio fijo.
    pub precio_unitario: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVentaRequest {
    pub cliente_id: Option<Uuid>,
    pub items: Vec<CreateVentaItemRequest>,
    pub metodo_pago: Option<String>,
    /// 31 = Crédito Fiscal (requiere RNC + dirección del comprador), 32 =
    /// Consumo (default). Se valida contra el total y los datos del cliente
    /// antes de completar la venta - ver `ecf_service::requiere_identificacion`.
    pub tipo_ecf: Option<i32>,
    /// Ver `Venta::entrega_diferida`. Ausente/false = comportamiento normal
    /// (descuenta stock ya mismo) - byte-idéntico a antes de este campo.
    pub entrega_diferida: Option<bool>,
}

pub struct VentaCompleta {
    pub venta: Venta,
    pub items: Vec<VentaItem>,
}

pub struct VentasService {
    pool: PgPool,
}

impl VentasService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_venta(
        &self,
        tenant_id: &str,
        usuario_id: Uuid,
        rol: &str,
        req: CreateVentaRequest,
        aprobado_por: Option<Uuid>,
    ) -> anyhow::Result<VentaCompleta> {
        if req.items.is_empty() {
            anyhow::bail!("La venta necesita al menos un producto");
        }
        let metodo_pago = req.metodo_pago.unwrap_or_else(|| "EFECTIVO".to_string());
        let es_fiado = metodo_pago == "FIADO";
        if es_fiado && req.cliente_id.is_none() {
            anyhow::bail!("Una venta fiada necesita un cliente seleccionado");
        }
        let entrega_diferida = req.entrega_diferida.unwrap_or(false);

        let mut tx = self.pool.begin().await?;

        // La caja representa el turno activo - una venta sin sesión abierta
        // no tiene contra qué reconciliarse en `caja_service::resumen`/`cerrar`.
        // Aplica a todo rol y método de pago (incluye FIADO: no mueve caja,
        // pero igual pertenece a un turno) - salvo un tenant SERVICIOS, que no
        // opera con caja registradora (ver Tenant::tipo_negocio).
        let tipo_negocio: String = sqlx::query_scalar("SELECT tipo_negocio FROM tenants WHERE rnc = $1")
            .bind(tenant_id)
            .fetch_one(&mut *tx)
            .await?;
        if tipo_negocio != "SERVICIOS" {
            let hay_caja_abierta: Option<i32> = sqlx::query_scalar(
                "SELECT 1 FROM caja_sesiones WHERE tenant_id = $1 AND estado = 'ABIERTA' LIMIT 1",
            )
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
            if hay_caja_abierta.is_none() {
                anyhow::bail!("CAJA_NO_ABIERTA: Debes abrir la caja antes de registrar ventas");
            }
        }

        let mut subtotal_total = Decimal::ZERO;
        let mut itbis_total = Decimal::ZERO;
        let mut descuento_total = Decimal::ZERO;
        let mut lineas: Vec<(Uuid, String, String, Decimal, Decimal, Decimal, String, Decimal, Decimal, Decimal, String)> = Vec::new();

        for item in &req.items {
            if item.cantidad <= Decimal::ZERO {
                anyhow::bail!("La cantidad debe ser mayor a cero");
            }
            let row: Option<(String, String, Option<Decimal>, String, Decimal, Decimal, String)> = sqlx::query_as(
                "SELECT sku, nombre, precio_venta, itbis_tipo, stock_actual, costo, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true FOR UPDATE",
            )
            .bind(item.producto_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
            let (sku, nombre, precio_venta_catalogo, itbis_tipo, stock_actual, costo, tipo) =
                row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;

            // Un Servicio no tiene precio fijo en el catálogo (puede variar
            // por volumen/alcance) - el precio se captura por línea en el
            // momento de la venta, y no tiene stock que chequear/descontar.
            let precio_venta = if tipo == "SERVICIO" {
                let precio = item.precio_unitario.ok_or_else(|| anyhow::anyhow!("{} es un servicio: falta precio_unitario para esta línea", nombre))?;
                if precio <= Decimal::ZERO {
                    anyhow::bail!("precio_unitario inválido para {}", nombre);
                }
                precio
            } else {
                if stock_actual < item.cantidad {
                    anyhow::bail!("Stock insuficiente para {}: disponible {}, solicitado {}", nombre, stock_actual, item.cantidad);
                }
                precio_venta_catalogo.unwrap_or_default()
            };

            let descuento = item.descuento.unwrap_or_default();
            let line_bruto = precio_venta * item.cantidad;
            if descuento < Decimal::ZERO || descuento > line_bruto {
                anyhow::bail!("Descuento inválido para {}", nombre);
            }
            let line_subtotal = line_bruto - descuento;
            let line_itbis = line_subtotal * itbis_rate(&itbis_tipo);
            subtotal_total += line_subtotal;
            itbis_total += line_itbis;
            descuento_total += descuento;

            // Si la entrega es diferida, la mercancía no sale todavía - el
            // stock se descuenta después, en lotes, vía conduces (ver
            // conduce_service::create_conduce). El chequeo de disponibilidad
            // de arriba sí se mantiene: no se factura más de lo que hay.
            // Un Servicio nunca mueve stock, sin importar entrega_diferida.
            if tipo != "SERVICIO" && !entrega_diferida {
                sqlx::query("UPDATE productos SET stock_actual = stock_actual - $1, updated_at = NOW() WHERE id = $2")
                    .bind(item.cantidad)
                    .bind(item.producto_id)
                    .execute(&mut *tx)
                    .await?;
            }

            lineas.push((item.producto_id, sku, nombre, item.cantidad, precio_venta, descuento, itbis_tipo, line_itbis, line_subtotal, costo, tipo));
        }

        // ADMIN siempre puede descontar lo que sea. Cualquier otro rol (en la
        // práctica solo CAJERO llega aquí, ver required_roles en main.rs) está
        // limitado a su `descuento_maximo_sin_aprobacion` salvo que un ADMIN
        // ya haya autorizado esta venta puntual (`aprobado_por`).
        if rol != "ADMIN" && descuento_total > Decimal::ZERO && aprobado_por.is_none() {
            let limite: Decimal = sqlx::query_scalar(
                "SELECT descuento_maximo_sin_aprobacion FROM usuarios WHERE id = $1 AND tenant_id = $2",
            )
            .bind(usuario_id)
            .bind(tenant_id)
            .fetch_one(&mut *tx)
            .await?;
            if descuento_total > limite {
                return Err(DescuentoRequiereAprobacion { descuento_solicitado: descuento_total, limite }.into());
            }
        }

        let total = subtotal_total + itbis_total;
        let tipo_ecf = req.tipo_ecf.unwrap_or(32);

        let cliente_info: Option<(String, Option<String>, Decimal, Decimal)> = match req.cliente_id {
            Some(cid) => sqlx::query_as(
                "SELECT rnc_cedula, direccion, saldo_pendiente, limite_credito FROM clientes WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
            )
            .bind(cid)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?
            .map(|(rnc, dir, saldo, limite): (Option<String>, Option<String>, Decimal, Decimal)| (rnc.unwrap_or_default(), dir, saldo, limite)),
            None => None,
        };
        let (rnc_cedula, direccion, saldo_pendiente, limite_credito) =
            cliente_info.unwrap_or((String::new(), None, Decimal::ZERO, Decimal::ZERO));
        requiere_identificacion(tipo_ecf, total, Some(&rnc_cedula), direccion.as_deref())?;

        if es_fiado && saldo_pendiente + total > limite_credito {
            anyhow::bail!(
                "Límite de crédito insuficiente: saldo actual {}, límite {}, esta venta lo excedería",
                saldo_pendiente,
                limite_credito
            );
        }

        let venta = sqlx::query_as::<_, Venta>(
            r#"INSERT INTO ventas (tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, metodo_pago, tipo_ecf, entrega_diferida)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               RETURNING id, tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, metodo_pago, estado,
                         e_ncf, tipo_ecf, estado_dgii, track_id, codigo_seguridad, qr_url, entrega_diferida, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.cliente_id)
        .bind(usuario_id)
        .bind(subtotal_total)
        .bind(itbis_total)
        .bind(total)
        .bind(&metodo_pago)
        .bind(tipo_ecf)
        .bind(entrega_diferida)
        .fetch_one(&mut *tx)
        .await?;

        // Cuánto de cada línea sale de inmediato: todo, salvo que la entrega
        // sea diferida (ahí arranca en 0 y sube con cada conduce). Un
        // Servicio no tiene inventario que "entregar" - siempre cuenta como
        // completo, incluso en una venta entrega_diferida.
        let cantidad_entregada_inicial = |cantidad: Decimal, tipo: &str| {
            if tipo == "SERVICIO" || !entrega_diferida { cantidad } else { Decimal::ZERO }
        };

        let mut items = Vec::new();
        for (producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, line_subtotal, costo, tipo) in lineas {
            let vi = sqlx::query_as::<_, VentaItem>(
                r#"INSERT INTO venta_items (venta_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, cantidad_entregada, costo_unitario)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                   RETURNING id, venta_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, cantidad_entregada, costo_unitario"#,
            )
            .bind(venta.id)
            .bind(producto_id)
            .bind(&sku)
            .bind(&nombre)
            .bind(cantidad)
            .bind(precio_unitario)
            .bind(descuento)
            .bind(&itbis_tipo)
            .bind(itbis_monto)
            .bind(line_subtotal)
            .bind(cantidad_entregada_inicial(cantidad, &tipo))
            .bind(costo)
            .fetch_one(&mut *tx)
            .await?;
            items.push(vi);

            if tipo != "SERVICIO" && !entrega_diferida {
                crate::services::inventario_service::InventarioService::insert_movimiento_tx(
                    &mut tx,
                    tenant_id,
                    Some(usuario_id),
                    producto_id,
                    "SALIDA",
                    -cantidad,
                    None,
                    Some("Venta POS".to_string()),
                    Some("VENTA"),
                    Some(venta.id),
                )
                .await?;
            }
        }

        if es_fiado {
            // No entra efectivo todavía - se registra como saldo del cliente
            // en vez de un ingreso de caja. El abono posterior (partner_service)
            // es lo que sí toca caja_movimientos.
            sqlx::query("UPDATE clientes SET saldo_pendiente = saldo_pendiente + $1 WHERE id = $2 AND tenant_id = $3")
                .bind(total)
                .bind(req.cliente_id)
                .bind(tenant_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, metodo_pago, referencia_tipo, referencia_id, usuario_id)
                   VALUES ($1, 'INGRESO', 'Venta POS', $2, $3, 'VENTA', $4, $5)"#,
            )
            .bind(tenant_id)
            .bind(total)
            .bind(&metodo_pago)
            .bind(venta.id)
            .bind(usuario_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(VentaCompleta { venta, items })
    }

    const VENTAS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "v.created_at"),
        ("total", "v.total"),
        ("estado", "v.estado"),
    ];

    pub async fn list_ventas(
        &self,
        tenant_id: &str,
        cliente_id: Option<Uuid>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        search: Option<String>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<VentaConCliente>, i64)> {
        let search_pattern = search
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .map(|s| format!("{}%", s));

        const WHERE_CLAUSE: &str = "WHERE v.tenant_id = $1
               AND ($2::uuid IS NULL OR v.cliente_id = $2)
               AND ($3::date IS NULL OR v.created_at::date >= $3)
               AND ($4::date IS NULL OR v.created_at::date <= $4)
               AND ($5::text IS NULL OR v.id::text LIKE $5)";

        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(*) FROM ventas v {WHERE_CLAUSE}"
        ))
        .bind(tenant_id)
        .bind(cliente_id)
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .bind(&search_pattern)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::VENTAS_SORTABLE, "v.created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT v.id, c.nombre AS cliente_nombre, v.total, v.metodo_pago, v.estado, v.e_ncf, v.estado_dgii, v.created_at
               FROM ventas v
               LEFT JOIN clientes c ON c.id = v.cliente_id
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $6 OFFSET $7"#
        );
        let rows = sqlx::query_as::<_, VentaConCliente>(&query)
            .bind(tenant_id)
            .bind(cliente_id)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(&search_pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn get_venta(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<VentaCompleta> {
        let venta = sqlx::query_as::<_, Venta>(
            r#"SELECT id, tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, metodo_pago, estado,
                      e_ncf, tipo_ecf, estado_dgii, track_id, codigo_seguridad, qr_url, entrega_diferida, created_at
               FROM ventas WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Venta no encontrada"))?;

        let items = sqlx::query_as::<_, VentaItem>(
            "SELECT id, venta_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, cantidad_entregada, costo_unitario FROM venta_items WHERE venta_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok(VentaCompleta { venta, items })
    }

    /// After DGII signing/send, persist the fiscal result on the sale.
    pub async fn set_ecf_result(
        &self,
        tenant_id: &str,
        id: Uuid,
        e_ncf: &str,
        tipo_ecf: i32,
        estado_dgii: &str,
        track_id: Option<&str>,
        codigo_seguridad: Option<&str>,
        qr_url: Option<&str>,
    ) -> anyhow::Result<Venta> {
        let venta = sqlx::query_as::<_, Venta>(
            r#"UPDATE ventas SET e_ncf = $1, tipo_ecf = $2, estado_dgii = $3, track_id = $4, codigo_seguridad = $5, qr_url = $6
               WHERE id = $7 AND tenant_id = $8
               RETURNING id, tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, metodo_pago, estado,
                         e_ncf, tipo_ecf, estado_dgii, track_id, codigo_seguridad, qr_url, entrega_diferida, created_at"#,
        )
        .bind(e_ncf)
        .bind(tipo_ecf)
        .bind(estado_dgii)
        .bind(track_id)
        .bind(codigo_seguridad)
        .bind(qr_url)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(venta)
    }

    /// Emite una Nota de Crédito (e-CF Tipo 34) para una venta ya COMPLETADA,
    /// referenciándola en vez de editarla/anularla en sitio. El e-NCF de la
    /// venta original permanece intacto en el historial - la Nota de Crédito
    /// es el documento que corrige.
    ///
    /// `items` ausente = devolución TOTAL, idéntica a la de siempre: revierte
    /// el 100% de lo entregado y del dinero y la venta queda ANULADA.
    /// `items` presente = devolución PARCIAL por línea: solo se acredita,
    /// reingresa al inventario y se devuelve de caja lo que trae cada línea, y
    /// la venta sigue COMPLETADA hasta que no quede nada por devolver.
    ///
    /// Los montos acreditados salen SIEMPRE de la línea vendida
    /// (`venta_items.precio_unitario/descuento/itbis_monto`), nunca del precio
    /// actual del producto: devolver mañana un producto que subió de precio no
    /// puede devolver más dinero del que entró.
    pub async fn create_nota_credito(
        &self,
        tenant_id: &str,
        usuario_id: Uuid,
        venta_id: Uuid,
        motivo: &str,
        items_req: Option<Vec<DevolucionItemRequest>>,
    ) -> anyhow::Result<(NotaCredito, Vec<NotaCreditoItem>)> {
        let mut tx = self.pool.begin().await?;

        let venta: Venta = sqlx::query_as(
            r#"SELECT id, tenant_id, cliente_id, usuario_id, subtotal, itbis_total, total, metodo_pago, estado,
                      e_ncf, tipo_ecf, estado_dgii, track_id, codigo_seguridad, qr_url, entrega_diferida, created_at
               FROM ventas WHERE id = $1 AND tenant_id = $2 FOR UPDATE"#,
        )
        .bind(venta_id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Venta no encontrada"))?;

        if venta.estado == "ANULADA" {
            anyhow::bail!("Esta venta ya fue devuelta por completo");
        }

        let items: Vec<VentaItem> = sqlx::query_as(
            "SELECT id, venta_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, cantidad_entregada, costo_unitario FROM venta_items WHERE venta_id = $1 ORDER BY id",
        )
        .bind(venta_id)
        .fetch_all(&mut *tx)
        .await?;

        // (venta_item, cantidad a devolver ahora). `None` = todo lo que quede
        // por devolver de esa línea, que es lo que significa una petición sin
        // `items`: devolución total.
        let solicitado: Vec<(&VentaItem, Option<Decimal>)> = match &items_req {
            None => items.iter().map(|it| (it, None)).collect(),
            Some(reqs) => {
                if reqs.is_empty() {
                    anyhow::bail!("Selecciona al menos un producto para devolver");
                }
                let mut vistos = std::collections::HashSet::new();
                let mut out = Vec::new();
                for r in reqs {
                    if !vistos.insert(r.venta_item_id) {
                        anyhow::bail!("Una misma línea aparece dos veces en la devolución");
                    }
                    if r.cantidad <= Decimal::ZERO {
                        anyhow::bail!("La cantidad a devolver debe ser mayor a cero");
                    }
                    let item = items
                        .iter()
                        .find(|it| it.id == r.venta_item_id)
                        .ok_or_else(|| anyhow::anyhow!("Esa línea no pertenece a esta venta"))?;
                    out.push((item, Some(r.cantidad)));
                }
                out
            }
        };

        let mut lineas: Vec<NuevaLineaDevolucion> = Vec::new();
        let mut subtotal_nota = Decimal::ZERO;
        let mut itbis_nota = Decimal::ZERO;
        // Una devolución es "total y primera" (el caso que existía antes de
        // las devoluciones parciales) solo si nadie devolvió nada antes y esta
        // se lleva cada línea completa: ahí los montos de la nota se copian
        // tal cual de la venta, sin prorrateo ni redondeo de por medio.
        let mut es_total_y_primera = solicitado.len() == items.len();

        for (item, cantidad_pedida) in &solicitado {
            // El FOR UPDATE sobre la línea es lo que hace segura la carrera
            // "dos cajeros devuelven la misma línea a la vez": el segundo
            // espera aquí y recién entonces lee el acumulado real.
            let bloqueada: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM venta_items WHERE id = $1 AND venta_id = $2 FOR UPDATE")
                .bind(item.id)
                .bind(venta_id)
                .fetch_optional(&mut *tx)
                .await?;
            if bloqueada.is_none() {
                anyhow::bail!("Esa línea no pertenece a esta venta");
            }

            let (ya_devuelto, ya_subtotal, ya_itbis): (Decimal, Decimal, Decimal) = sqlx::query_as(
                "SELECT COALESCE(SUM(cantidad), 0), COALESCE(SUM(subtotal), 0), COALESCE(SUM(itbis_monto), 0)
                 FROM nota_credito_items WHERE venta_item_id = $1",
            )
            .bind(item.id)
            .fetch_one(&mut *tx)
            .await?;

            if ya_devuelto > Decimal::ZERO {
                es_total_y_primera = false;
            }
            let pendiente = item.cantidad - ya_devuelto;
            let cantidad = match cantidad_pedida {
                Some(c) => *c,
                // Devolución total sobre una venta que ya tuvo devoluciones
                // parciales: se acredita el resto, no la línea completa.
                None => pendiente,
            };
            if cantidad > pendiente {
                anyhow::bail!(
                    "De {} solo quedan {} por devolver (solicitaste {})",
                    item.nombre,
                    pendiente,
                    cantidad
                );
            }
            if cantidad <= Decimal::ZERO {
                continue;
            }
            if cantidad < item.cantidad {
                es_total_y_primera = false;
            }

            // Prorrateo con el resto asignado a la ÚLTIMA devolución: mientras
            // quede algo pendiente se prorratea y se redondea al centavo; la
            // devolución que agota la línea se lleva exactamente lo que falta
            // para completarla. Así la suma de las devoluciones de una línea
            // es igual al monto original de la línea, al centavo, siempre.
            let agota_linea = ya_devuelto + cantidad == item.cantidad;
            let (subtotal_linea, itbis_linea) = if agota_linea {
                (item.subtotal - ya_subtotal, item.itbis_monto - ya_itbis)
            } else {
                (
                    redondear_centavos(item.subtotal * cantidad / item.cantidad),
                    redondear_centavos(item.itbis_monto * cantidad / item.cantidad),
                )
            };
            // El descuento acreditado es lo que sobra entre el precio de lista
            // devuelto y el subtotal acreditado, de modo que el renglón del
            // e-CF cumpla MontoItem = Cantidad * Precio - Descuento exacto.
            let descuento_linea = (item.precio_unitario * cantidad - subtotal_linea).max(Decimal::ZERO);

            subtotal_nota += subtotal_linea;
            itbis_nota += itbis_linea;

            // Solo vuelve al inventario lo que realmente salió del negocio:
            // en una venta de entrega diferida el stock baja conduce a
            // conduce (ver conduce_service), así que lo no entregado nunca se
            // descontó y no hay nada que reingresar. En una venta normal
            // cantidad_entregada == cantidad y esto es la cantidad devuelta.
            let entregada = item.cantidad_entregada;
            let restock = (ya_devuelto + cantidad).min(entregada) - ya_devuelto.min(entregada);

            lineas.push(NuevaLineaDevolucion {
                venta_item_id: item.id,
                producto_id: item.producto_id,
                sku: item.sku.clone(),
                nombre: item.nombre.clone(),
                cantidad,
                precio_unitario: item.precio_unitario,
                descuento: descuento_linea,
                itbis_tipo: item.itbis_tipo.clone(),
                itbis_monto: itbis_linea,
                subtotal: subtotal_linea,
                costo_unitario: item.costo_unitario,
                restock,
            });
        }

        let (subtotal_nota, itbis_nota, total_nota) = if es_total_y_primera {
            (venta.subtotal, venta.itbis_total, venta.total)
        } else {
            (subtotal_nota, itbis_nota, subtotal_nota + itbis_nota)
        };
        if total_nota <= Decimal::ZERO {
            anyhow::bail!("La devolución no acredita ningún monto");
        }

        for linea in &lineas {
            if linea.restock <= Decimal::ZERO {
                continue;
            }
            // Un ítem SERVICIO queda con cantidad_entregada == cantidad (ver
            // create_venta) pero nunca movió stock - no hay nada que revertir.
            let tipo: String = sqlx::query_scalar("SELECT tipo FROM productos WHERE id = $1")
                .bind(linea.producto_id)
                .fetch_one(&mut *tx)
                .await?;
            if tipo == "SERVICIO" {
                continue;
            }
            crate::services::inventario_service::InventarioService::apply_movimiento_tx(
                &mut tx,
                tenant_id,
                Some(usuario_id),
                linea.producto_id,
                "ENTRADA",
                linea.restock,
                linea.costo_unitario,
                Some("Nota de Crédito".to_string()),
                Some("NOTA_CREDITO"),
                Some(venta_id),
            )
            .await?;
        }

        // Una venta FIADO nunca ingresó efectivo (ver create_venta): devolverlo
        // por caja sacaría dinero que jamás entró y dejaría intacta la deuda
        // del cliente. Se baja el saldo pendiente, que es lo que el asiento de
        // la nota ya venía acreditando contra 1110 Cuentas por Cobrar.
        if venta.metodo_pago == "FIADO" && venta.cliente_id.is_some() {
            sqlx::query("UPDATE clientes SET saldo_pendiente = saldo_pendiente - $1 WHERE id = $2 AND tenant_id = $3")
                .bind(total_nota)
                .bind(venta.cliente_id)
                .bind(tenant_id)
                .execute(&mut *tx)
                .await?;
        } else {
            sqlx::query(
                r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, metodo_pago, referencia_tipo, referencia_id, usuario_id)
                   VALUES ($1, 'EGRESO', 'Nota de Crédito', $2, $3, 'NOTA_CREDITO', $4, $5)"#,
            )
            .bind(tenant_id)
            .bind(total_nota)
            .bind(&venta.metodo_pago)
            .bind(venta_id)
            .bind(usuario_id)
            .execute(&mut *tx)
            .await?;
        }

        let nota = sqlx::query_as::<_, NotaCredito>(
            r#"INSERT INTO notas_credito (tenant_id, venta_id, usuario_id, motivo, subtotal, itbis_total, total, es_parcial)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
               RETURNING id, tenant_id, venta_id, motivo, subtotal, itbis_total, total, es_parcial, e_ncf, estado_dgii, codigo_seguridad, qr_url, created_at"#,
        )
        .bind(tenant_id)
        .bind(venta_id)
        .bind(usuario_id)
        .bind(motivo)
        .bind(subtotal_nota)
        .bind(itbis_nota)
        .bind(total_nota)
        .bind(!es_total_y_primera)
        .fetch_one(&mut *tx)
        .await?;

        let mut nota_items = Vec::new();
        for linea in lineas {
            let ni = sqlx::query_as::<_, NotaCreditoItem>(
                r#"INSERT INTO nota_credito_items (nota_credito_id, venta_item_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, costo_unitario)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
                   RETURNING id, nota_credito_id, venta_item_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, costo_unitario"#,
            )
            .bind(nota.id)
            .bind(linea.venta_item_id)
            .bind(linea.producto_id)
            .bind(&linea.sku)
            .bind(&linea.nombre)
            .bind(linea.cantidad)
            .bind(linea.precio_unitario)
            .bind(linea.descuento)
            .bind(&linea.itbis_tipo)
            .bind(linea.itbis_monto)
            .bind(linea.subtotal)
            .bind(linea.costo_unitario)
            .fetch_one(&mut *tx)
            .await?;
            nota_items.push(ni);
        }

        // La venta solo se ANULA cuando ya no queda nada por devolver; una
        // devolución parcial la deja COMPLETADA (y con su e-NCF y su estado
        // DGII sin tocar, que es lo que la nota de crédito viene a corregir).
        let pendiente_total: Decimal = sqlx::query_scalar(
            r#"SELECT COALESCE(SUM(vi.cantidad), 0) - COALESCE((
                   SELECT SUM(nci.cantidad) FROM nota_credito_items nci
                   JOIN venta_items vi2 ON vi2.id = nci.venta_item_id
                   WHERE vi2.venta_id = $1
               ), 0)
               FROM venta_items vi WHERE vi.venta_id = $1"#,
        )
        .bind(venta_id)
        .fetch_one(&mut *tx)
        .await?;
        if pendiente_total <= Decimal::ZERO {
            sqlx::query("UPDATE ventas SET estado = 'ANULADA' WHERE id = $1").bind(venta_id).execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok((nota, nota_items))
    }

    /// Notas de crédito de una venta + cuánto se ha devuelto de cada línea.
    /// Endpoint propio en vez de un campo nuevo en `VentaItem` para no tocar
    /// el camino caliente de `create_venta`/`get_venta`.
    pub async fn devoluciones_de_venta(
        &self,
        tenant_id: &str,
        venta_id: Uuid,
    ) -> anyhow::Result<(Vec<NotaCredito>, Vec<LineaDevuelta>)> {
        let notas = sqlx::query_as::<_, NotaCredito>(
            r#"SELECT id, tenant_id, venta_id, motivo, subtotal, itbis_total, total, es_parcial, e_ncf, estado_dgii, codigo_seguridad, qr_url, created_at
               FROM notas_credito WHERE tenant_id = $1 AND venta_id = $2 ORDER BY created_at"#,
        )
        .bind(tenant_id)
        .bind(venta_id)
        .fetch_all(&self.pool)
        .await?;

        let lineas = sqlx::query_as::<_, LineaDevuelta>(
            r#"SELECT vi.id AS venta_item_id,
                      COALESCE(SUM(nci.cantidad), 0) AS cantidad_devuelta
               FROM venta_items vi
               LEFT JOIN nota_credito_items nci ON nci.venta_item_id = vi.id
               WHERE vi.venta_id = $1
               GROUP BY vi.id"#,
        )
        .bind(venta_id)
        .fetch_all(&self.pool)
        .await?;

        Ok((notas, lineas))
    }

    const NOTAS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "nc.created_at"),
        ("total", "nc.total"),
    ];

    pub async fn list_notas_credito(
        &self,
        tenant_id: &str,
        venta_id: Option<Uuid>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<NotaCreditoConVenta>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE nc.tenant_id = $1
               AND ($2::uuid IS NULL OR nc.venta_id = $2)
               AND ($3::date IS NULL OR nc.created_at::date >= $3)
               AND ($4::date IS NULL OR nc.created_at::date <= $4)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM notas_credito nc {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(venta_id)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::NOTAS_SORTABLE, "nc.created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT nc.id, nc.venta_id, v.e_ncf AS venta_e_ncf, c.nombre AS cliente_nombre,
                      nc.motivo, nc.total, nc.es_parcial, nc.e_ncf, nc.estado_dgii, nc.created_at
               FROM notas_credito nc
               JOIN ventas v ON v.id = nc.venta_id
               LEFT JOIN clientes c ON c.id = v.cliente_id
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $5 OFFSET $6"#
        );
        let rows = sqlx::query_as::<_, NotaCreditoConVenta>(&query)
            .bind(tenant_id)
            .bind(venta_id)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn set_nota_credito_ecf_result(
        &self,
        tenant_id: &str,
        id: Uuid,
        e_ncf: &str,
        estado_dgii: &str,
        codigo_seguridad: Option<&str>,
        qr_url: Option<&str>,
    ) -> anyhow::Result<NotaCredito> {
        let nota = sqlx::query_as::<_, NotaCredito>(
            r#"UPDATE notas_credito SET e_ncf = $1, estado_dgii = $2, codigo_seguridad = $3, qr_url = $4
               WHERE id = $5 AND tenant_id = $6
               RETURNING id, tenant_id, venta_id, motivo, subtotal, itbis_total, total, es_parcial, e_ncf, estado_dgii, codigo_seguridad, qr_url, created_at"#,
        )
        .bind(e_ncf)
        .bind(estado_dgii)
        .bind(codigo_seguridad)
        .bind(qr_url)
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(nota)
    }

    pub async fn get_nota_credito(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<NotaCredito> {
        let nota = sqlx::query_as::<_, NotaCredito>(
            r#"SELECT id, tenant_id, venta_id, motivo, subtotal, itbis_total, total, es_parcial, e_ncf, estado_dgii, codigo_seguridad, qr_url, created_at
               FROM notas_credito WHERE id = $1 AND tenant_id = $2"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Nota de Crédito no encontrada"))?;
        Ok(nota)
    }

    pub async fn get_nota_credito_items(&self, nota_id: Uuid) -> anyhow::Result<Vec<NotaCreditoItem>> {
        let items = sqlx::query_as::<_, NotaCreditoItem>(
            r#"SELECT id, nota_credito_id, venta_item_id, producto_id, sku, nombre, cantidad, precio_unitario,
                      descuento, itbis_tipo, itbis_monto, subtotal, costo_unitario
               FROM nota_credito_items WHERE nota_credito_id = $1 ORDER BY created_at"#,
        )
        .bind(nota_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(items)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NotaCredito {
    pub id: Uuid,
    pub tenant_id: String,
    pub venta_id: Uuid,
    pub motivo: String,
    pub subtotal: Decimal,
    pub itbis_total: Decimal,
    pub total: Decimal,
    /// false = devolución total (y primera) de la venta: la nota acredita los
    /// montos de la venta tal cual. Es lo que queda en toda fila anterior a
    /// las devoluciones parciales, y lo que `contabilidad_service::sincronizar`
    /// usa para decidir entre el espejo del asiento original y el asiento
    /// proporcional construido desde las líneas de la nota.
    pub es_parcial: bool,
    pub e_ncf: Option<String>,
    pub estado_dgii: Option<String>,
    pub codigo_seguridad: Option<String>,
    pub qr_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Una línea de la Nota de Crédito: qué se devolvió y por cuánto se acreditó.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NotaCreditoItem {
    pub id: Uuid,
    pub nota_credito_id: Uuid,
    pub venta_item_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub descuento: Decimal,
    pub itbis_tipo: String,
    pub itbis_monto: Decimal,
    pub subtotal: Decimal,
    pub costo_unitario: Option<Decimal>,
}

/// Una línea a devolver, tal como la pide el cliente HTTP.
#[derive(Debug, Deserialize)]
pub struct DevolucionItemRequest {
    pub venta_item_id: Uuid,
    pub cantidad: Decimal,
}

/// Cuánto se ha devuelto de una línea de venta, sumando todas sus notas.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LineaDevuelta {
    pub venta_item_id: Uuid,
    pub cantidad_devuelta: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NotaCreditoConVenta {
    pub id: Uuid,
    pub venta_id: Uuid,
    pub venta_e_ncf: Option<String>,
    pub cliente_nombre: Option<String>,
    pub motivo: String,
    pub total: Decimal,
    pub es_parcial: bool,
    pub e_ncf: Option<String>,
    pub estado_dgii: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Línea ya calculada, lista para insertarse y para armar el e-CF.
struct NuevaLineaDevolucion {
    venta_item_id: Uuid,
    producto_id: Uuid,
    sku: String,
    nombre: String,
    cantidad: Decimal,
    precio_unitario: Decimal,
    descuento: Decimal,
    itbis_tipo: String,
    itbis_monto: Decimal,
    subtotal: Decimal,
    costo_unitario: Option<Decimal>,
    /// Cuánto de `cantidad` vuelve al inventario (≤ cantidad: lo no entregado
    /// de una venta de entrega diferida nunca se descontó).
    restock: Decimal,
}

/// Redondeo comercial al centavo (medio centavo se aleja del cero), no el
/// "banker's rounding" que `Decimal::round_dp` usa por defecto: un colmado
/// espera que 0.125 se cobre como 0.13, no como 0.12.
fn redondear_centavos(v: Decimal) -> Decimal {
    v.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

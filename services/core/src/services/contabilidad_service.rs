//! Contabilidad Service - Módulo 7 (Libro Diario / Libro Mayor)
//! Doble entrada plana en Postgres: cada línea es debe o haber sobre una
//! cuenta. `sincronizar` genera asientos simples para Ventas/Compras/Nómina/
//! Abonos/Notas de Crédito/Ajustes de Inventario/Movimientos Bancarios que
//! todavía no tienen uno (reutiliza los datos ya reales de esos módulos en
//! vez de duplicar lógica de negocio).
//!
//! Diseño completo: docs/12-LIBRO-DIARIO-LIBRO-MAYOR-PLAN.md

use chrono::{DateTime, Datelike, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Asiento {
    pub id: Uuid,
    pub fecha: NaiveDate,
    pub cuenta: String,
    pub descripcion: String,
    pub debe: Decimal,
    pub haber: Decimal,
    pub referencia_tipo: Option<String>,
    pub referencia_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    /// Agrupa esta línea con sus hermanas bajo una cabecera `asientos` - es
    /// lo que un cliente necesita para, por ejemplo,
    /// `POST /v1/contabilidad/asientos/:asiento_id/reversar`.
    pub asiento_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAsientoLinea {
    pub cuenta: String,
    pub debe: Decimal,
    pub haber: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CreateAsientoRequest {
    pub descripcion: String,
    pub fecha: Option<NaiveDate>,
    pub lineas: Vec<CreateAsientoLinea>,
}

#[derive(Debug, Serialize)]
pub struct CuentaResumen {
    pub cuenta: String,
    pub debe: Decimal,
    pub haber: Decimal,
    pub saldo: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CuentaContable {
    pub id: Uuid,
    pub codigo: String,
    pub nombre: String,
    pub tipo: String,
    pub naturaleza: String,
    pub activo: bool,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Periodo {
    pub id: Uuid,
    pub anio: i32,
    pub mes: i32,
    pub estado: String,
    pub cerrado_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AsientoHeader {
    pub id: Uuid,
    pub fecha: NaiveDate,
    pub descripcion: String,
    pub origen: String,
    pub referencia_tipo: Option<String>,
    pub referencia_id: Option<Uuid>,
    pub reversa_de: Option<Uuid>,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct LineaSimple {
    pub cuenta: String,
    pub debe: Decimal,
    pub haber: Decimal,
}

#[derive(Debug, Serialize)]
pub struct AsientoConLineas {
    pub cabecera: AsientoHeader,
    pub lineas: Vec<LineaSimple>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct MovimientoCuenta {
    pub fecha: NaiveDate,
    pub descripcion: String,
    pub debe: Decimal,
    pub haber: Decimal,
    pub saldo: Decimal,
}

#[derive(Debug, Serialize)]
pub struct SincronizarResultado {
    pub ventas_procesadas: i64,
    pub compras_procesadas: i64,
    pub nomina_procesada: i64,
    pub gastos_procesados: i64,
    pub adelantos_procesados: i64,
    pub abonos_procesados: i64,
    pub notas_credito_procesadas: i64,
    pub ajustes_procesados: i64,
    pub banco_procesados: i64,
}

pub struct ContabilidadService {
    pool: PgPool,
}

impl ContabilidadService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const ASIENTOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("fecha", "fecha"),
        ("cuenta", "cuenta"),
        ("created_at", "created_at"),
    ];

    pub async fn list_asientos(
        &self,
        tenant_id: &str,
        cuenta: Option<String>,
        referencia_tipo: Option<String>,
        fecha_desde: Option<NaiveDate>,
        fecha_hasta: Option<NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Asiento>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::text IS NULL OR cuenta = $2)
               AND ($3::text IS NULL OR referencia_tipo = $3)
               AND ($4::date IS NULL OR fecha >= $4)
               AND ($5::date IS NULL OR fecha <= $5)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM asientos_contables {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(&cuenta)
            .bind(&referencia_tipo)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::ASIENTOS_SORTABLE, "fecha DESC, created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, fecha, cuenta, descripcion, debe, haber, referencia_tipo, referencia_id, created_at, asiento_id
               FROM asientos_contables
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $6 OFFSET $7"#
        );
        let rows = sqlx::query_as::<_, Asiento>(&query)
            .bind(tenant_id)
            .bind(&cuenta)
            .bind(&referencia_tipo)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    const LIBRO_MAYOR_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("cuenta", "cuenta"),
        ("debe", "debe"),
        ("haber", "haber"),
    ];

    pub async fn libro_mayor(
        &self,
        tenant_id: &str,
        search: Option<String>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<CuentaResumen>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1 AND ($2::text IS NULL OR LOWER(cuenta) LIKE $2)";

        let total: i64 = sqlx::query_scalar(&format!(
            "SELECT COUNT(DISTINCT cuenta) FROM asientos_contables {WHERE_CLAUSE}"
        ))
        .bind(tenant_id)
        .bind(&pattern)
        .fetch_one(&self.pool)
        .await?;

        let order_by = sort.resolve(Self::LIBRO_MAYOR_SORTABLE, "cuenta ASC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT cuenta, COALESCE(SUM(debe),0) AS debe, COALESCE(SUM(haber),0) AS haber
               FROM asientos_contables
               {WHERE_CLAUSE}
               GROUP BY cuenta
               ORDER BY {order_by}
               LIMIT $3 OFFSET $4"#
        );
        let rows: Vec<(String, Decimal, Decimal)> = sqlx::query_as(&query)
            .bind(tenant_id)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((
            rows.into_iter().map(|(cuenta, debe, haber)| CuentaResumen { saldo: debe - haber, cuenta, debe, haber }).collect(),
            total,
        ))
    }

    /// Cuenta por cuenta, en orden cronológico, con saldo corriente
    /// (`SUM(debe - haber)` acumulado). Responde "muéstrame cada movimiento
    /// que afecta esta cuenta" y "cuál es el saldo actual" a la vez.
    pub async fn libro_mayor_detalle(
        &self,
        tenant_id: &str,
        cuenta: &str,
        fecha_desde: Option<NaiveDate>,
        fecha_hasta: Option<NaiveDate>,
        page: &crate::pagination::PageParams,
    ) -> anyhow::Result<(Vec<MovimientoCuenta>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1 AND cuenta = $2
               AND ($3::date IS NULL OR fecha >= $3)
               AND ($4::date IS NULL OR fecha <= $4)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM asientos_contables {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(cuenta)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .fetch_one(&self.pool)
            .await?;

        let limit = page.limit(20);
        // La ventana corre sobre TODAS las filas que matchean el WHERE antes
        // de aplicar LIMIT/OFFSET (Postgres evalúa LIMIT al final), así que
        // el saldo sigue siendo correcto aunque se pagine.
        let query = format!(
            r#"SELECT fecha, descripcion, debe, haber,
                      SUM(debe - haber) OVER (ORDER BY fecha, created_at ROWS UNBOUNDED PRECEDING) AS saldo
               FROM asientos_contables
               {WHERE_CLAUSE}
               ORDER BY fecha, created_at
               LIMIT $5 OFFSET $6"#
        );
        let rows = sqlx::query_as::<_, MovimientoCuenta>(&query)
            .bind(tenant_id)
            .bind(cuenta)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn list_cuentas(&self, tenant_id: &str) -> anyhow::Result<Vec<CuentaContable>> {
        let rows = sqlx::query_as::<_, CuentaContable>(
            "SELECT id, codigo, nombre, tipo, naturaleza, activo FROM cuentas_contables WHERE tenant_id = $1 ORDER BY codigo",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    const DIARIO_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("fecha", "fecha"),
        ("created_at", "created_at"),
    ];

    /// Cronológico, todo tipo de transacción junto, cada asiento con sus
    /// líneas anidadas (a diferencia de `list_asientos`, que devuelve líneas
    /// sueltas).
    pub async fn libro_diario(
        &self,
        tenant_id: &str,
        fecha_desde: Option<NaiveDate>,
        fecha_hasta: Option<NaiveDate>,
        origen: Option<String>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<AsientoConLineas>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::date IS NULL OR fecha >= $2)
               AND ($3::date IS NULL OR fecha <= $3)
               AND ($4::text IS NULL OR origen = $4)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM asientos {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(&origen)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::DIARIO_SORTABLE, "fecha DESC, created_at DESC");
        let limit = page.limit(20);
        let headers = sqlx::query_as::<_, AsientoHeader>(&format!(
            r#"SELECT id, fecha, descripcion, origen, referencia_tipo, referencia_id, reversa_de, usuario_id, created_at
               FROM asientos
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $5 OFFSET $6"#
        ))
        .bind(tenant_id)
        .bind(fecha_desde)
        .bind(fecha_hasta)
        .bind(&origen)
        .bind(limit)
        .bind(page.offset(20))
        .fetch_all(&self.pool)
        .await?;

        let ids: Vec<Uuid> = headers.iter().map(|h| h.id).collect();
        let lineas: Vec<(Uuid, String, Decimal, Decimal)> = sqlx::query_as(
            "SELECT asiento_id, cuenta, debe, haber FROM asientos_contables WHERE asiento_id = ANY($1) ORDER BY created_at",
        )
        .bind(&ids)
        .fetch_all(&self.pool)
        .await?;

        let resultado = headers
            .into_iter()
            .map(|h| {
                let lineas_h = lineas
                    .iter()
                    .filter(|(aid, ..)| *aid == h.id)
                    .map(|(_, cuenta, debe, haber)| LineaSimple { cuenta: cuenta.clone(), debe: *debe, haber: *haber })
                    .collect();
                AsientoConLineas { cabecera: h, lineas: lineas_h }
            })
            .collect();
        Ok((resultado, total))
    }

    pub async fn list_periodos(&self, tenant_id: &str) -> anyhow::Result<Vec<Periodo>> {
        let rows = sqlx::query_as::<_, Periodo>(
            "SELECT id, anio, mes, estado, cerrado_at FROM periodos_contables WHERE tenant_id = $1 ORDER BY anio DESC, mes DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Cierra (o crea ya cerrado) el período anio-mes. Upsert: funciona
    /// tanto si el período ya tiene asientos como si todavía no existe fila
    /// (bloqueo preventivo de un mes futuro).
    pub async fn cerrar_periodo(&self, tenant_id: &str, anio: i32, mes: i32, usuario_id: Uuid) -> anyhow::Result<Periodo> {
        let periodo = sqlx::query_as::<_, Periodo>(
            r#"INSERT INTO periodos_contables (tenant_id, anio, mes, estado, cerrado_at, cerrado_por)
               VALUES ($1, $2, $3, 'CERRADO', NOW(), $4)
               ON CONFLICT (tenant_id, anio, mes) DO UPDATE SET estado = 'CERRADO', cerrado_at = NOW(), cerrado_por = $4
               RETURNING id, anio, mes, estado, cerrado_at"#,
        )
        .bind(tenant_id)
        .bind(anio)
        .bind(mes)
        .bind(usuario_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(periodo)
    }

    /// Núcleo compartido de todo insert de asiento (manual, automático o
    /// reversión): valida balance, respeta períodos cerrados, y es
    /// idempotente vía `UNIQUE(tenant_id, referencia_tipo, referencia_id)`
    /// en `asientos` con `ON CONFLICT DO NOTHING` - una garantía real a
    /// nivel de DB, no un chequeo `NOT EXISTS` de aplicación con condición
    /// de carrera. `Ok(None)` significa "ya existía, no se hizo nada".
    #[allow(clippy::too_many_arguments)]
    async fn create_entry(
        &self,
        tx: &mut sqlx::PgConnection,
        tenant_id: &str,
        fecha: NaiveDate,
        descripcion: &str,
        origen: &str,
        referencia_tipo: &str,
        referencia_id: Option<Uuid>,
        reversa_de: Option<Uuid>,
        usuario_id: Option<Uuid>,
        lineas: &[(String, Decimal, Decimal)],
    ) -> anyhow::Result<Option<Vec<Asiento>>> {
        if lineas.len() < 2 {
            anyhow::bail!("Un asiento necesita al menos dos líneas (debe y haber)");
        }
        let total_debe: Decimal = lineas.iter().map(|(_, d, _)| *d).sum();
        let total_haber: Decimal = lineas.iter().map(|(_, _, h)| *h).sum();
        if total_debe != total_haber {
            anyhow::bail!("El asiento no cuadra: debe {} vs haber {}", total_debe, total_haber);
        }
        if total_debe == Decimal::ZERO {
            anyhow::bail!("El asiento no puede estar vacío");
        }

        let anio = fecha.year();
        let mes = fecha.month() as i32;
        let periodo: (Uuid, String) = sqlx::query_as(
            r#"INSERT INTO periodos_contables (tenant_id, anio, mes) VALUES ($1, $2, $3)
               ON CONFLICT (tenant_id, anio, mes) DO UPDATE SET tenant_id = EXCLUDED.tenant_id
               RETURNING id, estado"#,
        )
        .bind(tenant_id)
        .bind(anio)
        .bind(mes)
        .fetch_one(&mut *tx)
        .await?;
        if periodo.1 == "CERRADO" {
            anyhow::bail!("El período {}-{:02} está cerrado; no se pueden registrar asientos en esa fecha", anio, mes);
        }
        let periodo_id = periodo.0;

        let header: Option<(Uuid,)> = sqlx::query_as(
            r#"INSERT INTO asientos (tenant_id, fecha, descripcion, origen, referencia_tipo, referencia_id, reversa_de, periodo_id, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
               ON CONFLICT (tenant_id, referencia_tipo, referencia_id) DO NOTHING
               RETURNING id"#,
        )
        .bind(tenant_id)
        .bind(fecha)
        .bind(descripcion)
        .bind(origen)
        .bind(referencia_tipo)
        .bind(referencia_id)
        .bind(reversa_de)
        .bind(periodo_id)
        .bind(usuario_id)
        .fetch_optional(&mut *tx)
        .await?;

        let Some((asiento_id,)) = header else {
            return Ok(None);
        };

        let mut asientos = Vec::with_capacity(lineas.len());
        for (cuenta, debe, haber) in lineas {
            let a = sqlx::query_as::<_, Asiento>(
                r#"INSERT INTO asientos_contables (tenant_id, fecha, cuenta, descripcion, debe, haber, referencia_tipo, referencia_id, usuario_id, asiento_id)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                   RETURNING id, fecha, cuenta, descripcion, debe, haber, referencia_tipo, referencia_id, created_at, asiento_id"#,
            )
            .bind(tenant_id)
            .bind(fecha)
            .bind(cuenta)
            .bind(descripcion)
            .bind(debe)
            .bind(haber)
            .bind(referencia_tipo)
            .bind(referencia_id)
            .bind(usuario_id)
            .bind(asiento_id)
            .fetch_one(&mut *tx)
            .await?;
            asientos.push(a);
        }
        Ok(Some(asientos))
    }

    /// Asiento manual: la suma de debe debe igualar la suma de haber.
    pub async fn create_asiento_manual(&self, tenant_id: &str, usuario_id: Uuid, req: CreateAsientoRequest) -> anyhow::Result<Vec<Asiento>> {
        let fecha = req.fecha.unwrap_or_else(|| Utc::now().date_naive());
        let lineas: Vec<(String, Decimal, Decimal)> = req.lineas.into_iter().map(|l| (l.cuenta, l.debe, l.haber)).collect();

        let mut tx = self.pool.begin().await?;
        let resultado = self
            .create_entry(&mut tx, tenant_id, fecha, &req.descripcion, "MANUAL", "MANUAL", None, None, Some(usuario_id), &lineas)
            .await?;
        tx.commit().await?;
        // referencia_id es siempre NULL para MANUAL, y Postgres trata cada
        // NULL como distinto en el UNIQUE - un asiento manual nunca choca
        // con otro, así que esto no debería ser None en la práctica.
        resultado.ok_or_else(|| anyhow::anyhow!("No se pudo crear el asiento"))
    }

    /// Revierte un asiento ya posteado: crea uno nuevo con las mismas
    /// cuentas y debe/haber invertidos, fechado hoy (nunca en el período
    /// original, aunque siga abierto - corregir un período cerrado se hace
    /// con un asiento nuevo, nunca tocando el original). Rechaza revertir
    /// dos veces o revertir una reversión.
    pub async fn reversar_asiento(&self, tenant_id: &str, usuario_id: Uuid, asiento_id: Uuid, motivo: Option<String>) -> anyhow::Result<Vec<Asiento>> {
        let mut tx = self.pool.begin().await?;

        let original: Option<(Uuid, String, String)> = sqlx::query_as(
            "SELECT id, origen, descripcion FROM asientos WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
        )
        .bind(asiento_id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?;
        let (orig_id, orig_origen, orig_descripcion) = original.ok_or_else(|| anyhow::anyhow!("Asiento no encontrado"))?;
        if orig_origen == "REVERSION" {
            anyhow::bail!("No se puede reversar una reversión");
        }
        let ya_reversado: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM asientos WHERE reversa_de = $1").bind(orig_id).fetch_optional(&mut *tx).await?;
        if ya_reversado.is_some() {
            anyhow::bail!("Este asiento ya fue reversado");
        }

        let lineas_originales: Vec<(String, Decimal, Decimal)> =
            sqlx::query_as("SELECT cuenta, debe, haber FROM asientos_contables WHERE asiento_id = $1")
                .bind(orig_id)
                .fetch_all(&mut *tx)
                .await?;
        if lineas_originales.is_empty() {
            anyhow::bail!("El asiento no tiene líneas");
        }
        let lineas_reversa: Vec<(String, Decimal, Decimal)> = lineas_originales.into_iter().map(|(cuenta, debe, haber)| (cuenta, haber, debe)).collect();

        let descripcion = match motivo {
            Some(m) if !m.trim().is_empty() => format!("Reversión: {} - {}", orig_descripcion, m.trim()),
            _ => format!("Reversión: {}", orig_descripcion),
        };
        let fecha = Utc::now().date_naive();

        let resultado = self
            .create_entry(&mut tx, tenant_id, fecha, &descripcion, "REVERSION", "REVERSION", Some(orig_id), Some(orig_id), Some(usuario_id), &lineas_reversa)
            .await?;
        tx.commit().await?;
        resultado.ok_or_else(|| anyhow::anyhow!("Este asiento ya fue reversado"))
    }

    /// Genera asientos para Ventas/Compras/Nómina/Abonos/Notas de
    /// Crédito/Ajustes de Inventario/Movimientos Bancarios que aún no
    /// tienen uno. Ver la tabla de reglas de negocio en
    /// docs/12-LIBRO-DIARIO-LIBRO-MAYOR-PLAN.md §4.
    pub async fn sincronizar(&self, tenant_id: &str) -> anyhow::Result<SincronizarResultado> {
        let mut tx = self.pool.begin().await?;

        // --- Ventas: Caja o Cuentas por Cobrar según metodo_pago (FIADO no
        // mueve caja - ver ventas_service::create_venta), + Costo de Ventas
        // usando el costo capturado en venta_items al momento de la venta.
        let ventas: Vec<(Uuid, Decimal, Decimal, Decimal, String, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT v.id, v.subtotal, v.itbis_total, v.total, v.metodo_pago, v.created_at
               FROM ventas v
               WHERE v.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = v.tenant_id AND a.referencia_tipo = 'VENTA' AND a.referencia_id = v.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut ventas_count = 0i64;
        for (id, subtotal, itbis, total, metodo_pago, created_at) in ventas {
            let fecha = created_at.date_naive();
            let cogs: Decimal = sqlx::query_scalar(
                "SELECT COALESCE(SUM(cantidad * costo_unitario), 0) FROM venta_items WHERE venta_id = $1 AND costo_unitario IS NOT NULL",
            )
            .bind(id)
            .fetch_one(&mut *tx)
            .await?;

            let mut lineas = Vec::new();
            let cuenta_pago = if metodo_pago == "FIADO" { "1110 Cuentas por Cobrar" } else { "1100 Caja y Bancos" };
            lineas.push((cuenta_pago.to_string(), total, Decimal::ZERO));
            lineas.push(("4100 Ingresos por Ventas".to_string(), Decimal::ZERO, subtotal));
            if itbis > Decimal::ZERO {
                lineas.push(("2100 ITBIS por Pagar".to_string(), Decimal::ZERO, itbis));
            }
            if cogs > Decimal::ZERO {
                lineas.push(("5050 Costo de Ventas".to_string(), cogs, Decimal::ZERO));
                lineas.push(("1200 Inventario".to_string(), Decimal::ZERO, cogs));
            }
            if self.create_entry(&mut tx, tenant_id, fecha, "Venta POS", "AUTOMATICO", "VENTA", Some(id), None, None, &lineas).await?.is_some() {
                ventas_count += 1;
            }
        }

        // --- Compras: siempre de contado hoy (compras_service no tiene
        // concepto de crédito a proveedor) - Inventario + ITBIS Adelantado
        // contra Caja.
        let compras: Vec<(Uuid, Decimal, Decimal, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT c.id, c.subtotal, c.itbis_total, c.total, c.created_at
               FROM compras c
               WHERE c.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = c.tenant_id AND a.referencia_tipo = 'COMPRA' AND a.referencia_id = c.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut compras_count = 0i64;
        for (id, subtotal, itbis, total, created_at) in compras {
            let fecha = created_at.date_naive();
            let mut lineas = vec![("1200 Inventario".to_string(), subtotal, Decimal::ZERO)];
            if itbis > Decimal::ZERO {
                lineas.push(("1150 ITBIS Adelantado".to_string(), itbis, Decimal::ZERO));
            }
            lineas.push(("1100 Caja y Bancos".to_string(), Decimal::ZERO, total));
            if self.create_entry(&mut tx, tenant_id, fecha, "Compra a proveedor", "AUTOMATICO", "COMPRA", Some(id), None, None, &lineas).await?.is_some() {
                compras_count += 1;
            }
        }

        // Adelantos ya APROBADOs (o ya DESCONTADOs en una corrida de
        // nómina): el efectivo salió de caja en el momento de la
        // aprobación, así que el asiento se genera ahí, no cuando se
        // descuenta - lo que se le adelantó al empleado es una cuenta por
        // cobrar (Anticipos), no un gasto todavía.
        let adelantos: Vec<(Uuid, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT ad.id, ad.monto, ad.created_at
               FROM nomina_adelantos ad
               WHERE ad.tenant_id = $1 AND ad.estado IN ('APROBADO', 'DESCONTADO')
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = ad.tenant_id AND a.referencia_tipo = 'ADELANTO' AND a.referencia_id = ad.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut adelantos_count = 0i64;
        for (id, monto, created_at) in adelantos {
            let fecha = created_at.date_naive();
            let lineas = vec![
                ("1300 Anticipos a Empleados".to_string(), monto, Decimal::ZERO),
                ("1100 Caja y Bancos".to_string(), Decimal::ZERO, monto),
            ];
            if self.create_entry(&mut tx, tenant_id, fecha, "Adelanto de nómina", "AUTOMATICO", "ADELANTO", Some(id), None, None, &lineas).await?.is_some() {
                adelantos_count += 1;
            }
        }

        // Gastos operativos: cada uno ya generó su salida de caja al
        // crearse (ver compras_service::create_gasto) - aquí solo falta el
        // asiento.
        let gastos: Vec<(Uuid, String, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT g.id, g.categoria, g.monto, g.created_at
               FROM gastos g
               WHERE g.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = g.tenant_id AND a.referencia_tipo = 'GASTO' AND a.referencia_id = g.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut gastos_count = 0i64;
        for (id, categoria, monto, created_at) in gastos {
            let fecha = created_at.date_naive();
            let cuenta_gasto = match categoria.as_str() {
                "ALQUILER" => "5210 Gasto de Alquiler",
                "SERVICIOS" => "5220 Gasto de Servicios",
                "TRANSPORTE" => "5230 Gasto de Transporte",
                _ => "5290 Otros Gastos Operativos",
            };
            let lineas = vec![(cuenta_gasto.to_string(), monto, Decimal::ZERO), ("1100 Caja y Bancos".to_string(), Decimal::ZERO, monto)];
            if self.create_entry(&mut tx, tenant_id, fecha, "Gasto operativo", "AUTOMATICO", "GASTO", Some(id), None, None, &lineas).await?.is_some() {
                gastos_count += 1;
            }
        }

        let periodos: Vec<(Uuid, Decimal, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT p.id, p.total_bruto, p.total_neto, p.created_at
               FROM nomina_periodos p
               WHERE p.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = p.tenant_id AND a.referencia_tipo = 'NOMINA' AND a.referencia_id = p.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut nomina_count = 0i64;
        for (id, bruto, neto, created_at) in periodos {
            let fecha = created_at.date_naive();
            // adelantos_descuento ya se registró como Anticipos a Empleados
            // arriba (al aprobarse) - aquí solo se liquida esa cuenta, el
            // resto de bruto-neto (TSS/ISR) sí es una retención nueva.
            let adelantos_descuento: Decimal = sqlx::query_scalar("SELECT COALESCE(SUM(adelantos_descuento), 0) FROM nomina_detalles WHERE periodo_id = $1")
                .bind(id)
                .fetch_one(&mut *tx)
                .await?;
            let retenciones = bruto - neto - adelantos_descuento;
            let mut lineas = vec![("5100 Gasto de Nómina".to_string(), bruto, Decimal::ZERO), ("1100 Caja y Bancos".to_string(), Decimal::ZERO, neto)];
            if retenciones > Decimal::ZERO {
                lineas.push(("2200 Retenciones y Descuentos".to_string(), Decimal::ZERO, retenciones));
            }
            if adelantos_descuento > Decimal::ZERO {
                lineas.push(("1300 Anticipos a Empleados".to_string(), Decimal::ZERO, adelantos_descuento));
            }
            if self.create_entry(&mut tx, tenant_id, fecha, "Nómina", "AUTOMATICO", "NOMINA", Some(id), None, None, &lineas).await?.is_some() {
                nomina_count += 1;
            }
        }

        // --- Abonos de clientes: paga Cuentas por Cobrar con Caja. Antes no
        // se sincronizaban en absoluto.
        let abonos: Vec<(Uuid, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT ca.id, ca.monto, ca.created_at
               FROM cliente_abonos ca
               WHERE ca.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = ca.tenant_id AND a.referencia_tipo = 'ABONO_CLIENTE' AND a.referencia_id = ca.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut abonos_count = 0i64;
        for (id, monto, created_at) in abonos {
            let fecha = created_at.date_naive();
            let lineas = vec![
                ("1100 Caja y Bancos".to_string(), monto, Decimal::ZERO),
                ("1110 Cuentas por Cobrar".to_string(), Decimal::ZERO, monto),
            ];
            if self.create_entry(&mut tx, tenant_id, fecha, "Abono de cliente", "AUTOMATICO", "ABONO_CLIENTE", Some(id), None, None, &lineas).await?.is_some() {
                abonos_count += 1;
            }
        }

        // --- Notas de Crédito: reversión real del asiento de la venta
        // original (mismas cuentas, debe/haber invertidos) - no una nueva
        // regla hecha a mano, así hereda automáticamente si la venta
        // original fue FIADO y si tuvo Costo de Ventas. Si la venta
        // original todavía no tiene asiento, se salta y se reintenta en el
        // próximo sincronizar.
        let notas: Vec<(Uuid, Uuid, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT nc.id, nc.venta_id, nc.created_at
               FROM notas_credito nc
               WHERE nc.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = nc.tenant_id AND a.referencia_tipo = 'NOTA_CREDITO' AND a.referencia_id = nc.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut notas_count = 0i64;
        for (id, venta_id, created_at) in notas {
            let original: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM asientos WHERE tenant_id = $1 AND referencia_tipo = 'VENTA' AND referencia_id = $2")
                .bind(tenant_id)
                .bind(venta_id)
                .fetch_optional(&mut *tx)
                .await?;
            let Some((orig_asiento_id,)) = original else { continue };
            let lineas_originales: Vec<(String, Decimal, Decimal)> =
                sqlx::query_as("SELECT cuenta, debe, haber FROM asientos_contables WHERE asiento_id = $1")
                    .bind(orig_asiento_id)
                    .fetch_all(&mut *tx)
                    .await?;
            let lineas_reversa: Vec<(String, Decimal, Decimal)> = lineas_originales.into_iter().map(|(cuenta, debe, haber)| (cuenta, haber, debe)).collect();
            let fecha = created_at.date_naive();
            if self
                .create_entry(&mut tx, tenant_id, fecha, "Nota de Crédito", "REVERSION", "NOTA_CREDITO", Some(id), Some(orig_asiento_id), None, &lineas_reversa)
                .await?
                .is_some()
            {
                notas_count += 1;
            }
        }

        // --- Ajustes de inventario manuales (mermas o sobrantes). El resto
        // de movimientos_inventario (ENTRADA/SALIDA) ya está cubierto por
        // sus asientos de origen (COMPRA/VENTA), así que solo se procesa
        // AJUSTE aquí.
        let ajustes: Vec<(Uuid, Decimal, Option<Decimal>, Uuid, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT mi.id, mi.cantidad, mi.costo_unitario, mi.producto_id, mi.created_at
               FROM movimientos_inventario mi
               WHERE mi.tenant_id = $1 AND mi.tipo = 'AJUSTE'
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = mi.tenant_id AND a.referencia_tipo = 'AJUSTE_INVENTARIO' AND a.referencia_id = mi.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut ajustes_count = 0i64;
        for (id, cantidad, costo_unitario, producto_id, created_at) in ajustes {
            let fecha = created_at.date_naive();
            let costo = match costo_unitario {
                Some(c) => c,
                None => sqlx::query_scalar("SELECT costo FROM productos WHERE id = $1").bind(producto_id).fetch_one(&mut *tx).await?,
            };
            let valor = cantidad.abs() * costo;
            if valor == Decimal::ZERO {
                continue;
            }
            let lineas = if cantidad > Decimal::ZERO {
                vec![("1200 Inventario".to_string(), valor, Decimal::ZERO), ("4200 Otros Ingresos".to_string(), Decimal::ZERO, valor)]
            } else {
                vec![
                    ("5295 Ajuste de Inventario (Merma)".to_string(), valor, Decimal::ZERO),
                    ("1200 Inventario".to_string(), Decimal::ZERO, valor),
                ]
            };
            if self
                .create_entry(&mut tx, tenant_id, fecha, "Ajuste de inventario", "AUTOMATICO", "AJUSTE_INVENTARIO", Some(id), None, None, &lineas)
                .await?
                .is_some()
            {
                ajustes_count += 1;
            }
        }

        // --- Movimientos bancarios: se asume que un depósito mueve efectivo
        // de la caja al banco y un retiro lo contrario (supuesto razonable
        // para un colmado; si algún depósito viene de fuera de la caja,
        // ajustar aquí).
        let bancos_movs: Vec<(Uuid, String, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT bm.id, bm.tipo, bm.monto, bm.created_at
               FROM banco_movimientos bm
               WHERE bm.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = bm.tenant_id AND a.referencia_tipo = 'BANCO' AND a.referencia_id = bm.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut banco_count = 0i64;
        for (id, tipo, monto, created_at) in bancos_movs {
            let fecha = created_at.date_naive();
            let lineas = if tipo == "DEPOSITO" {
                vec![
                    ("1160 Depósitos Bancarios".to_string(), monto, Decimal::ZERO),
                    ("1100 Caja y Bancos".to_string(), Decimal::ZERO, monto),
                ]
            } else {
                vec![
                    ("1100 Caja y Bancos".to_string(), monto, Decimal::ZERO),
                    ("1160 Depósitos Bancarios".to_string(), Decimal::ZERO, monto),
                ]
            };
            if self.create_entry(&mut tx, tenant_id, fecha, "Movimiento bancario", "AUTOMATICO", "BANCO", Some(id), None, None, &lineas).await?.is_some() {
                banco_count += 1;
            }
        }

        tx.commit().await?;
        Ok(SincronizarResultado {
            ventas_procesadas: ventas_count,
            compras_procesadas: compras_count,
            nomina_procesada: nomina_count,
            gastos_procesados: gastos_count,
            adelantos_procesados: adelantos_count,
            abonos_procesados: abonos_count,
            notas_credito_procesadas: notas_count,
            ajustes_procesados: ajustes_count,
            banco_procesados: banco_count,
        })
    }
}

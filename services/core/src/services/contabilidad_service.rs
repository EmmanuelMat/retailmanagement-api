//! Contabilidad Service - Módulo 7 (Libro Mayor)
//! Doble entrada plana en Postgres: cada línea es debe o haber sobre una
//! cuenta, sin reinventar TigerBeetle. `sincronizar` genera asientos simples
//! para Ventas/Compras/Nómina que todavía no tienen uno (reutiliza los datos
//! ya reales de esos módulos en vez de duplicar lógica de negocio).

use chrono::{DateTime, NaiveDate, Utc};
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

#[derive(Debug, Serialize)]
pub struct SincronizarResultado {
    pub ventas_procesadas: i64,
    pub compras_procesadas: i64,
    pub nomina_procesada: i64,
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
            r#"SELECT id, fecha, cuenta, descripcion, debe, haber, referencia_tipo, referencia_id, created_at
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

    /// Asiento manual: la suma de debe debe igualar la suma de haber.
    pub async fn create_asiento_manual(&self, tenant_id: &str, usuario_id: Uuid, req: CreateAsientoRequest) -> anyhow::Result<Vec<Asiento>> {
        if req.lineas.len() < 2 {
            anyhow::bail!("Un asiento necesita al menos dos líneas (debe y haber)");
        }
        let total_debe: Decimal = req.lineas.iter().map(|l| l.debe).sum();
        let total_haber: Decimal = req.lineas.iter().map(|l| l.haber).sum();
        if total_debe != total_haber {
            anyhow::bail!("El asiento no cuadra: debe {} vs haber {}", total_debe, total_haber);
        }
        if total_debe == Decimal::ZERO {
            anyhow::bail!("El asiento no puede estar vacío");
        }

        let fecha = req.fecha.unwrap_or_else(|| Utc::now().date_naive());
        let mut tx = self.pool.begin().await?;
        let mut asientos = Vec::new();
        for linea in req.lineas {
            let a = sqlx::query_as::<_, Asiento>(
                r#"INSERT INTO asientos_contables (tenant_id, fecha, cuenta, descripcion, debe, haber, referencia_tipo, usuario_id)
                   VALUES ($1, $2, $3, $4, $5, $6, 'MANUAL', $7)
                   RETURNING id, fecha, cuenta, descripcion, debe, haber, referencia_tipo, referencia_id, created_at"#,
            )
            .bind(tenant_id)
            .bind(fecha)
            .bind(&linea.cuenta)
            .bind(&req.descripcion)
            .bind(linea.debe)
            .bind(linea.haber)
            .bind(usuario_id)
            .fetch_one(&mut *tx)
            .await?;
            asientos.push(a);
        }
        tx.commit().await?;
        Ok(asientos)
    }

    async fn insert_linea(&self, tx: &mut sqlx::PgConnection, tenant_id: &str, fecha: NaiveDate, cuenta: &str, descripcion: &str, debe: Decimal, haber: Decimal, referencia_tipo: &str, referencia_id: Uuid) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO asientos_contables (tenant_id, fecha, cuenta, descripcion, debe, haber, referencia_tipo, referencia_id)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8)"#,
        )
        .bind(tenant_id)
        .bind(fecha)
        .bind(cuenta)
        .bind(descripcion)
        .bind(debe)
        .bind(haber)
        .bind(referencia_tipo)
        .bind(referencia_id)
        .execute(tx)
        .await?;
        Ok(())
    }

    /// Genera asientos simples para Ventas/Compras/Nómina que aún no tienen uno.
    pub async fn sincronizar(&self, tenant_id: &str) -> anyhow::Result<SincronizarResultado> {
        let mut tx = self.pool.begin().await?;

        let ventas: Vec<(Uuid, Decimal, Decimal, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT v.id, v.subtotal, v.itbis_total, v.total, v.created_at
               FROM ventas v
               WHERE v.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos_contables a WHERE a.referencia_tipo = 'VENTA' AND a.referencia_id = v.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let ventas_count = ventas.len() as i64;
        for (id, subtotal, itbis, total, created_at) in ventas {
            let fecha = created_at.date_naive();
            self.insert_linea(&mut tx, tenant_id, fecha, "1100 Caja y Bancos", "Venta POS", total, Decimal::ZERO, "VENTA", id).await?;
            self.insert_linea(&mut tx, tenant_id, fecha, "4100 Ingresos por Ventas", "Venta POS", Decimal::ZERO, subtotal, "VENTA", id).await?;
            if itbis > Decimal::ZERO {
                self.insert_linea(&mut tx, tenant_id, fecha, "2100 ITBIS por Pagar", "Venta POS", Decimal::ZERO, itbis, "VENTA", id).await?;
            }
        }

        let compras: Vec<(Uuid, Decimal, Decimal, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT c.id, c.subtotal, c.itbis_total, c.total, c.created_at
               FROM compras c
               WHERE c.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos_contables a WHERE a.referencia_tipo = 'COMPRA' AND a.referencia_id = c.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let compras_count = compras.len() as i64;
        for (id, subtotal, itbis, total, created_at) in compras {
            let fecha = created_at.date_naive();
            self.insert_linea(&mut tx, tenant_id, fecha, "1200 Inventario", "Compra a proveedor", subtotal, Decimal::ZERO, "COMPRA", id).await?;
            if itbis > Decimal::ZERO {
                self.insert_linea(&mut tx, tenant_id, fecha, "1150 ITBIS Adelantado", "Compra a proveedor", itbis, Decimal::ZERO, "COMPRA", id).await?;
            }
            self.insert_linea(&mut tx, tenant_id, fecha, "1100 Caja y Bancos", "Compra a proveedor", Decimal::ZERO, total, "COMPRA", id).await?;
        }

        let periodos: Vec<(Uuid, Decimal, Decimal, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT p.id, p.total_bruto, p.total_neto, p.created_at
               FROM nomina_periodos p
               WHERE p.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos_contables a WHERE a.referencia_tipo = 'NOMINA' AND a.referencia_id = p.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let nomina_count = periodos.len() as i64;
        for (id, bruto, neto, created_at) in periodos {
            let fecha = created_at.date_naive();
            let retenciones = bruto - neto;
            self.insert_linea(&mut tx, tenant_id, fecha, "5100 Gasto de Nómina", "Nómina", bruto, Decimal::ZERO, "NOMINA", id).await?;
            self.insert_linea(&mut tx, tenant_id, fecha, "1100 Caja y Bancos", "Nómina", Decimal::ZERO, neto, "NOMINA", id).await?;
            if retenciones > Decimal::ZERO {
                self.insert_linea(&mut tx, tenant_id, fecha, "2200 Retenciones y Descuentos", "Nómina", Decimal::ZERO, retenciones, "NOMINA", id).await?;
            }
        }

        tx.commit().await?;
        Ok(SincronizarResultado { ventas_procesadas: ventas_count, compras_procesadas: compras_count, nomina_procesada: nomina_count })
    }
}

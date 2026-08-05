//! Nómina y Adelantos Service - Módulo 8
//! Reconstruido plano (Postgres, sin event sourcing/TigerBeetle). La regla
//! del 50% que antes vivía en el `aggregates/employee.rs` eliminado ahora es
//! una función simple sobre datos reales.

use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Empleado {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre: String,
    pub cedula: Option<String>,
    pub puesto: Option<String>,
    pub salario_mensual: Decimal,
    pub fecha_ingreso: Option<NaiveDate>,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEmpleadoRequest {
    pub nombre: String,
    pub cedula: Option<String>,
    pub puesto: Option<String>,
    pub salario_mensual: Decimal,
    pub fecha_ingreso: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEmpleadoRequest {
    pub nombre: Option<String>,
    pub cedula: Option<String>,
    pub puesto: Option<String>,
    pub salario_mensual: Option<Decimal>,
    pub activo: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct EmpleadoConDisponible {
    #[serde(flatten)]
    pub empleado: Empleado,
    pub disponible_adelanto: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Adelanto {
    pub id: Uuid,
    pub empleado_id: Uuid,
    pub monto: Decimal,
    pub motivo: Option<String>,
    pub estado: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct AdelantoConEmpleado {
    pub id: Uuid,
    pub empleado_id: Uuid,
    pub empleado_nombre: String,
    pub monto: Decimal,
    pub motivo: Option<String>,
    pub estado: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAdelantoRequest {
    pub empleado_id: Uuid,
    pub monto: Decimal,
    pub motivo: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NominaPeriodo {
    pub id: Uuid,
    pub periodo: String,
    pub fecha_pago: Option<NaiveDate>,
    pub total_bruto: Decimal,
    pub total_neto: Decimal,
    pub estado: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct NominaDetalle {
    pub id: Uuid,
    pub periodo_id: Uuid,
    pub empleado_id: Uuid,
    pub empleado_nombre: String,
    pub salario_bruto: Decimal,
    pub tss: Decimal,
    pub isr: Decimal,
    pub adelantos_descuento: Decimal,
    pub neto: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct RunPayrollRequest {
    pub periodo: String,
    pub fecha_pago: Option<NaiveDate>,
}

const TSS_RATE: Decimal = dec!(0.0591);

pub struct NominaService {
    pool: PgPool,
}

impl NominaService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ---- Empleados ----

    pub async fn list_empleados(&self, tenant_id: &str) -> anyhow::Result<Vec<EmpleadoConDisponible>> {
        let empleados = sqlx::query_as::<_, Empleado>(
            "SELECT id, tenant_id, nombre, cedula, puesto, salario_mensual, fecha_ingreso, activo, created_at FROM empleados WHERE tenant_id = $1 AND activo = true ORDER BY nombre",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;

        let mut out = Vec::with_capacity(empleados.len());
        for empleado in empleados {
            let disponible = self.disponible_adelanto(&empleado).await?;
            out.push(EmpleadoConDisponible { empleado, disponible_adelanto: disponible });
        }
        Ok(out)
    }

    pub async fn get_empleado(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<Empleado> {
        sqlx::query_as::<_, Empleado>(
            "SELECT id, tenant_id, nombre, cedula, puesto, salario_mensual, fecha_ingreso, activo, created_at FROM empleados WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Empleado no encontrado"))
    }

    pub async fn create_empleado(&self, tenant_id: &str, req: CreateEmpleadoRequest) -> anyhow::Result<Empleado> {
        if req.nombre.trim().is_empty() {
            anyhow::bail!("El nombre del empleado es requerido");
        }
        if req.salario_mensual <= Decimal::ZERO {
            anyhow::bail!("El salario debe ser mayor a cero");
        }
        let empleado = sqlx::query_as::<_, Empleado>(
            r#"INSERT INTO empleados (tenant_id, nombre, cedula, puesto, salario_mensual, fecha_ingreso)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, tenant_id, nombre, cedula, puesto, salario_mensual, fecha_ingreso, activo, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.nombre.trim())
        .bind(&req.cedula)
        .bind(&req.puesto)
        .bind(req.salario_mensual)
        .bind(req.fecha_ingreso)
        .fetch_one(&self.pool)
        .await?;
        Ok(empleado)
    }

    pub async fn update_empleado(&self, tenant_id: &str, id: Uuid, req: UpdateEmpleadoRequest) -> anyhow::Result<Empleado> {
        let existing = self.get_empleado(tenant_id, id).await?;
        let empleado = sqlx::query_as::<_, Empleado>(
            r#"UPDATE empleados SET nombre = $1, cedula = $2, puesto = $3, salario_mensual = $4, activo = $5
               WHERE id = $6 AND tenant_id = $7
               RETURNING id, tenant_id, nombre, cedula, puesto, salario_mensual, fecha_ingreso, activo, created_at"#,
        )
        .bind(req.nombre.unwrap_or(existing.nombre))
        .bind(req.cedula.or(existing.cedula))
        .bind(req.puesto.or(existing.puesto))
        .bind(req.salario_mensual.unwrap_or(existing.salario_mensual))
        .bind(req.activo.unwrap_or(existing.activo))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(empleado)
    }

    pub async fn delete_empleado(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE empleados SET activo = false WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Adelantos: regla del 50% del sueldo mensual, sin intereses ----

    async fn disponible_adelanto(&self, empleado: &Empleado) -> anyhow::Result<Decimal> {
        let tomado: Option<Decimal> = sqlx::query_scalar(
            "SELECT COALESCE(SUM(monto), 0) FROM nomina_adelantos WHERE empleado_id = $1 AND estado IN ('PENDIENTE', 'APROBADO')",
        )
        .bind(empleado.id)
        .fetch_one(&self.pool)
        .await?;
        let limite = empleado.salario_mensual * dec!(0.5);
        let disponible = limite - tomado.unwrap_or_default();
        Ok(if disponible < Decimal::ZERO { Decimal::ZERO } else { disponible })
    }

    pub async fn list_adelantos(&self, tenant_id: &str) -> anyhow::Result<Vec<AdelantoConEmpleado>> {
        let rows = sqlx::query_as::<_, AdelantoConEmpleado>(
            r#"SELECT a.id, a.empleado_id, e.nombre AS empleado_nombre, a.monto, a.motivo, a.estado, a.created_at
               FROM nomina_adelantos a JOIN empleados e ON e.id = a.empleado_id
               WHERE a.tenant_id = $1 ORDER BY a.created_at DESC LIMIT 200"#,
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn request_adelanto(&self, tenant_id: &str, usuario_id: Uuid, req: CreateAdelantoRequest) -> anyhow::Result<Adelanto> {
        if req.monto <= Decimal::ZERO {
            anyhow::bail!("El monto debe ser mayor a cero");
        }
        let empleado = self.get_empleado(tenant_id, req.empleado_id).await?;
        let disponible = self.disponible_adelanto(&empleado).await?;
        if req.monto > disponible {
            anyhow::bail!("Excede el disponible para adelanto: máximo RD$ {} (50% del sueldo mensual)", disponible);
        }

        let adelanto = sqlx::query_as::<_, Adelanto>(
            r#"INSERT INTO nomina_adelantos (tenant_id, empleado_id, monto, motivo, usuario_id)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, empleado_id, monto, motivo, estado, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.empleado_id)
        .bind(req.monto)
        .bind(&req.motivo)
        .bind(usuario_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(adelanto)
    }

    pub async fn approve_adelanto(&self, tenant_id: &str, id: Uuid, usuario_id: Uuid) -> anyhow::Result<Adelanto> {
        let mut tx = self.pool.begin().await?;

        let adelanto = sqlx::query_as::<_, Adelanto>(
            r#"UPDATE nomina_adelantos SET estado = 'APROBADO' WHERE id = $1 AND tenant_id = $2 AND estado = 'PENDIENTE'
               RETURNING id, empleado_id, monto, motivo, estado, created_at"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Adelanto no encontrado o ya procesado"))?;

        sqlx::query(
            r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, referencia_tipo, referencia_id, usuario_id)
               VALUES ($1, 'EGRESO', 'Adelanto de nómina', $2, 'ADELANTO', $3, $4)"#,
        )
        .bind(tenant_id)
        .bind(adelanto.monto)
        .bind(adelanto.id)
        .bind(usuario_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(adelanto)
    }

    pub async fn reject_adelanto(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<Adelanto> {
        sqlx::query_as::<_, Adelanto>(
            r#"UPDATE nomina_adelantos SET estado = 'RECHAZADO' WHERE id = $1 AND tenant_id = $2 AND estado = 'PENDIENTE'
               RETURNING id, empleado_id, monto, motivo, estado, created_at"#,
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Adelanto no encontrado o ya procesado"))
    }

    // ---- Nómina (corridas de pago) ----

    pub async fn list_periodos(&self, tenant_id: &str) -> anyhow::Result<Vec<NominaPeriodo>> {
        let rows = sqlx::query_as::<_, NominaPeriodo>(
            "SELECT id, periodo, fecha_pago, total_bruto, total_neto, estado, created_at FROM nomina_periodos WHERE tenant_id = $1 ORDER BY created_at DESC",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_periodo(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<(NominaPeriodo, Vec<NominaDetalle>)> {
        let periodo = sqlx::query_as::<_, NominaPeriodo>(
            "SELECT id, periodo, fecha_pago, total_bruto, total_neto, estado, created_at FROM nomina_periodos WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Período no encontrado"))?;

        let detalles = sqlx::query_as::<_, NominaDetalle>(
            "SELECT id, periodo_id, empleado_id, empleado_nombre, salario_bruto, tss, isr, adelantos_descuento, neto FROM nomina_detalles WHERE periodo_id = $1",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        Ok((periodo, detalles))
    }

    /// Corre nómina para todos los empleados activos: TSS 5.91%, ISR simplificado
    /// a 0 (fuera de alcance de este módulo), descuenta adelantos APROBADOs
    /// pendientes de descuento y los marca DESCONTADO.
    pub async fn run_payroll(&self, tenant_id: &str, usuario_id: Uuid, req: RunPayrollRequest) -> anyhow::Result<(NominaPeriodo, Vec<NominaDetalle>)> {
        let empleados = sqlx::query_as::<_, Empleado>(
            "SELECT id, tenant_id, nombre, cedula, puesto, salario_mensual, fecha_ingreso, activo, created_at FROM empleados WHERE tenant_id = $1 AND activo = true ORDER BY nombre",
        )
        .bind(tenant_id)
        .fetch_all(&self.pool)
        .await?;
        if empleados.is_empty() {
            anyhow::bail!("No hay empleados activos para procesar nómina");
        }

        let mut tx = self.pool.begin().await?;

        let periodo = sqlx::query_as::<_, NominaPeriodo>(
            r#"INSERT INTO nomina_periodos (tenant_id, periodo, fecha_pago)
               VALUES ($1, $2, $3)
               RETURNING id, periodo, fecha_pago, total_bruto, total_neto, estado, created_at"#,
        )
        .bind(tenant_id)
        .bind(&req.periodo)
        .bind(req.fecha_pago)
        .fetch_one(&mut *tx)
        .await?;

        let mut total_bruto = Decimal::ZERO;
        let mut total_neto = Decimal::ZERO;
        let mut detalles = Vec::new();

        for empleado in &empleados {
            let bruto = empleado.salario_mensual;
            let tss = bruto * TSS_RATE;
            let isr = Decimal::ZERO;

            let adelantos: Vec<(Uuid, Decimal)> = sqlx::query_as(
                "SELECT id, monto FROM nomina_adelantos WHERE empleado_id = $1 AND estado = 'APROBADO' FOR UPDATE",
            )
            .bind(empleado.id)
            .fetch_all(&mut *tx)
            .await?;
            let descuento_adelantos: Decimal = adelantos.iter().map(|(_, m)| *m).sum();

            let neto = bruto - tss - isr - descuento_adelantos;

            let detalle = sqlx::query_as::<_, NominaDetalle>(
                r#"INSERT INTO nomina_detalles (periodo_id, empleado_id, empleado_nombre, salario_bruto, tss, isr, adelantos_descuento, neto)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                   RETURNING id, periodo_id, empleado_id, empleado_nombre, salario_bruto, tss, isr, adelantos_descuento, neto"#,
            )
            .bind(periodo.id)
            .bind(empleado.id)
            .bind(&empleado.nombre)
            .bind(bruto)
            .bind(tss)
            .bind(isr)
            .bind(descuento_adelantos)
            .bind(neto)
            .fetch_one(&mut *tx)
            .await?;

            for (adelanto_id, _) in adelantos {
                sqlx::query("UPDATE nomina_adelantos SET estado = 'DESCONTADO' WHERE id = $1")
                    .bind(adelanto_id)
                    .execute(&mut *tx)
                    .await?;
            }

            total_bruto += bruto;
            total_neto += neto;
            detalles.push(detalle);
        }

        let periodo_actualizado = sqlx::query_as::<_, NominaPeriodo>(
            r#"UPDATE nomina_periodos SET total_bruto = $1, total_neto = $2 WHERE id = $3
               RETURNING id, periodo, fecha_pago, total_bruto, total_neto, estado, created_at"#,
        )
        .bind(total_bruto)
        .bind(total_neto)
        .bind(periodo.id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, referencia_tipo, referencia_id, usuario_id)
               VALUES ($1, 'EGRESO', 'Nómina', $2, 'NOMINA', $3, $4)"#,
        )
        .bind(tenant_id)
        .bind(total_neto)
        .bind(periodo.id)
        .bind(usuario_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok((periodo_actualizado, detalles))
    }
}

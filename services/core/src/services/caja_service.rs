//! Caja y Bancos Service - Módulo 9
//! Caja: apertura/cierre de turno sobre caja_movimientos (ya alimentado por
//! Ventas/Compras/Gastos). Bancos: cuentas + depósitos/retiros manuales.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CajaSesion {
    pub id: Uuid,
    pub tenant_id: String,
    pub usuario_id: Option<Uuid>,
    pub monto_inicial: Decimal,
    pub monto_final: Option<Decimal>,
    pub monto_esperado: Option<Decimal>,
    pub diferencia: Option<Decimal>,
    pub estado: String,
    pub abierta_at: DateTime<Utc>,
    pub cerrada_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CajaMovimiento {
    pub id: Uuid,
    pub tipo: String,
    pub concepto: String,
    pub monto: Decimal,
    pub metodo_pago: Option<String>,
    pub referencia_tipo: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CajaResumen {
    pub sesion: Option<CajaSesion>,
    pub ingresos: Decimal,
    pub egresos: Decimal,
    pub saldo_actual: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct AbrirCajaRequest {
    pub monto_inicial: Decimal,
}

#[derive(Debug, Deserialize)]
pub struct CerrarCajaRequest {
    pub monto_final: Decimal,
}

pub struct CajaService {
    pool: PgPool,
}

impl CajaService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn sesion_abierta(&self, tenant_id: &str) -> anyhow::Result<Option<CajaSesion>> {
        let sesion = sqlx::query_as::<_, CajaSesion>(
            r#"SELECT id, tenant_id, usuario_id, monto_inicial, monto_final, monto_esperado, diferencia, estado, abierta_at, cerrada_at
               FROM caja_sesiones WHERE tenant_id = $1 AND estado = 'ABIERTA' ORDER BY abierta_at DESC LIMIT 1"#,
        )
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(sesion)
    }

    pub async fn resumen(&self, tenant_id: &str) -> anyhow::Result<CajaResumen> {
        let sesion = self.sesion_abierta(tenant_id).await?;

        // Sin sesión abierta no hay "turno actual" que resumir.
        let Some(sesion) = sesion else {
            return Ok(CajaResumen { sesion: None, ingresos: Decimal::ZERO, egresos: Decimal::ZERO, saldo_actual: Decimal::ZERO });
        };

        let row: (Option<Decimal>, Option<Decimal>) = sqlx::query_as(
            r#"SELECT
                 COALESCE(SUM(monto) FILTER (WHERE tipo = 'INGRESO'), 0),
                 COALESCE(SUM(monto) FILTER (WHERE tipo = 'EGRESO'), 0)
               FROM caja_movimientos WHERE tenant_id = $1 AND created_at >= $2"#,
        )
        .bind(tenant_id)
        .bind(sesion.abierta_at)
        .fetch_one(&self.pool)
        .await?;

        let ingresos = row.0.unwrap_or_default();
        let egresos = row.1.unwrap_or_default();
        let inicial = sesion.monto_inicial;

        Ok(CajaResumen { sesion: Some(sesion), ingresos, egresos, saldo_actual: inicial + ingresos - egresos })
    }

    pub async fn abrir(&self, tenant_id: &str, usuario_id: Uuid, req: AbrirCajaRequest) -> anyhow::Result<CajaSesion> {
        if self.sesion_abierta(tenant_id).await?.is_some() {
            anyhow::bail!("Ya hay una sesión de caja abierta");
        }
        let sesion = sqlx::query_as::<_, CajaSesion>(
            r#"INSERT INTO caja_sesiones (tenant_id, usuario_id, monto_inicial)
               VALUES ($1, $2, $3)
               RETURNING id, tenant_id, usuario_id, monto_inicial, monto_final, monto_esperado, diferencia, estado, abierta_at, cerrada_at"#,
        )
        .bind(tenant_id)
        .bind(usuario_id)
        .bind(req.monto_inicial)
        .fetch_one(&self.pool)
        .await?;
        Ok(sesion)
    }

    pub async fn cerrar(&self, tenant_id: &str, req: CerrarCajaRequest) -> anyhow::Result<CajaSesion> {
        let sesion = self.sesion_abierta(tenant_id).await?.ok_or_else(|| anyhow::anyhow!("No hay sesión de caja abierta"))?;
        let resumen = self.resumen(tenant_id).await?;
        let esperado = resumen.saldo_actual;
        let diferencia = req.monto_final - esperado;

        let actualizado = sqlx::query_as::<_, CajaSesion>(
            r#"UPDATE caja_sesiones SET monto_final = $1, monto_esperado = $2, diferencia = $3, estado = 'CERRADA', cerrada_at = NOW()
               WHERE id = $4 AND tenant_id = $5
               RETURNING id, tenant_id, usuario_id, monto_inicial, monto_final, monto_esperado, diferencia, estado, abierta_at, cerrada_at"#,
        )
        .bind(req.monto_final)
        .bind(esperado)
        .bind(diferencia)
        .bind(sesion.id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(actualizado)
    }

    const MOVIMIENTOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "created_at"),
        ("monto", "monto"),
    ];

    pub async fn list_movimientos(
        &self,
        tenant_id: &str,
        tipo: Option<String>,
        referencia_tipo: Option<String>,
        fecha_desde: Option<chrono::NaiveDate>,
        fecha_hasta: Option<chrono::NaiveDate>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<CajaMovimiento>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::text IS NULL OR tipo = $2)
               AND ($3::text IS NULL OR referencia_tipo = $3)
               AND ($4::date IS NULL OR created_at::date >= $4)
               AND ($5::date IS NULL OR created_at::date <= $5)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM caja_movimientos {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(&tipo)
            .bind(&referencia_tipo)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::MOVIMIENTOS_SORTABLE, "created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tipo, concepto, monto, metodo_pago, referencia_tipo, created_at
               FROM caja_movimientos
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $6 OFFSET $7"#
        );
        let rows = sqlx::query_as::<_, CajaMovimiento>(&query)
            .bind(tenant_id)
            .bind(&tipo)
            .bind(&referencia_tipo)
            .bind(fecha_desde)
            .bind(fecha_hasta)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    const SESIONES_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("abierta_at", "abierta_at"),
        ("diferencia", "diferencia"),
    ];

    pub async fn list_sesiones(
        &self,
        tenant_id: &str,
        estado: Option<String>,
        usuario_id: Option<Uuid>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<CajaSesion>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::text IS NULL OR estado = $2)
               AND ($3::uuid IS NULL OR usuario_id = $3)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM caja_sesiones {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(&estado)
            .bind(usuario_id)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::SESIONES_SORTABLE, "abierta_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tenant_id, usuario_id, monto_inicial, monto_final, monto_esperado, diferencia, estado, abierta_at, cerrada_at
               FROM caja_sesiones
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $4 OFFSET $5"#
        );
        let rows = sqlx::query_as::<_, CajaSesion>(&query)
            .bind(tenant_id)
            .bind(&estado)
            .bind(usuario_id)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }
}

// ---- Bancos ----

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Banco {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre_banco: String,
    pub numero_cuenta: Option<String>,
    pub tipo_cuenta: String,
    pub saldo: Decimal,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBancoRequest {
    pub nombre_banco: String,
    pub numero_cuenta: Option<String>,
    pub tipo_cuenta: Option<String>,
    pub saldo: Option<Decimal>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct BancoMovimiento {
    pub id: Uuid,
    pub banco_id: Uuid,
    pub tipo: String,
    pub concepto: Option<String>,
    pub monto: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBancoMovimientoRequest {
    pub tipo: String, // DEPOSITO | RETIRO
    pub concepto: Option<String>,
    pub monto: Decimal,
}

pub struct BancosService {
    pool: PgPool,
}

impl BancosService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    const BANCOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("nombre_banco", "nombre_banco"),
        ("saldo", "saldo"),
        ("created_at", "created_at"),
    ];

    pub async fn list(
        &self,
        tenant_id: &str,
        search: Option<String>,
        activo: Option<bool>,
        tipo_cuenta: Option<String>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Banco>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::bool IS NULL OR activo = $2)
               AND ($3::text IS NULL OR tipo_cuenta = $3)
               AND ($4::text IS NULL OR LOWER(nombre_banco) LIKE $4 OR numero_cuenta LIKE $4)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM bancos {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(activo)
            .bind(&tipo_cuenta)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::BANCOS_SORTABLE, "nombre_banco ASC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tenant_id, nombre_banco, numero_cuenta, tipo_cuenta, saldo, activo, created_at
               FROM bancos
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $5 OFFSET $6"#
        );
        let rows = sqlx::query_as::<_, Banco>(&query)
            .bind(tenant_id)
            .bind(activo)
            .bind(&tipo_cuenta)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn create(&self, tenant_id: &str, req: CreateBancoRequest) -> anyhow::Result<Banco> {
        if req.nombre_banco.trim().is_empty() {
            anyhow::bail!("El nombre del banco es requerido");
        }
        let banco = sqlx::query_as::<_, Banco>(
            r#"INSERT INTO bancos (tenant_id, nombre_banco, numero_cuenta, tipo_cuenta, saldo)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, tenant_id, nombre_banco, numero_cuenta, tipo_cuenta, saldo, activo, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.nombre_banco.trim())
        .bind(&req.numero_cuenta)
        .bind(req.tipo_cuenta.unwrap_or_else(|| "CORRIENTE".to_string()))
        .bind(req.saldo.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?;
        Ok(banco)
    }

    const BANCO_MOVIMIENTOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("created_at", "created_at"),
        ("monto", "monto"),
    ];

    pub async fn list_movimientos(
        &self,
        tenant_id: &str,
        banco_id: Uuid,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<BancoMovimiento>, i64)> {
        const WHERE_CLAUSE: &str = "WHERE banco_id = $1 AND tenant_id = $2";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM banco_movimientos {WHERE_CLAUSE}"))
            .bind(banco_id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::BANCO_MOVIMIENTOS_SORTABLE, "created_at DESC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, banco_id, tipo, concepto, monto, created_at
               FROM banco_movimientos
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $3 OFFSET $4"#
        );
        let rows = sqlx::query_as::<_, BancoMovimiento>(&query)
            .bind(banco_id)
            .bind(tenant_id)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn create_movimiento(&self, tenant_id: &str, banco_id: Uuid, usuario_id: Uuid, req: CreateBancoMovimientoRequest) -> anyhow::Result<BancoMovimiento> {
        if !["DEPOSITO", "RETIRO"].contains(&req.tipo.as_str()) {
            anyhow::bail!("Tipo inválido: debe ser DEPOSITO o RETIRO");
        }
        if req.monto <= Decimal::ZERO {
            anyhow::bail!("El monto debe ser mayor a cero");
        }

        let mut tx = self.pool.begin().await?;

        let saldo: Option<(Decimal,)> = sqlx::query_as("SELECT saldo FROM bancos WHERE id = $1 AND tenant_id = $2 FOR UPDATE")
            .bind(banco_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
        let saldo_actual = saldo.ok_or_else(|| anyhow::anyhow!("Cuenta bancaria no encontrada"))?.0;

        let delta = if req.tipo == "DEPOSITO" { req.monto } else { -req.monto };
        let nuevo_saldo = saldo_actual + delta;
        if nuevo_saldo < Decimal::ZERO {
            anyhow::bail!("Saldo insuficiente en la cuenta");
        }

        sqlx::query("UPDATE bancos SET saldo = $1 WHERE id = $2")
            .bind(nuevo_saldo)
            .bind(banco_id)
            .execute(&mut *tx)
            .await?;

        let mov = sqlx::query_as::<_, BancoMovimiento>(
            r#"INSERT INTO banco_movimientos (tenant_id, banco_id, tipo, concepto, monto, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, banco_id, tipo, concepto, monto, created_at"#,
        )
        .bind(tenant_id)
        .bind(banco_id)
        .bind(&req.tipo)
        .bind(&req.concepto)
        .bind(req.monto)
        .bind(usuario_id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(mov)
    }
}

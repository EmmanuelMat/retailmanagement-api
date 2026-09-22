//! Partner Service - Clientes y Proveedores (Módulo 4)
//! Same shape as catalog_service.rs; bundled in one file since both entities
//! are near-identical plain CRUD and the module groups them together.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Cliente {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre: String,
    pub rnc_cedula: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub saldo_pendiente: Decimal,
    pub limite_credito: Decimal,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateClienteRequest {
    pub nombre: String,
    pub rnc_cedula: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub limite_credito: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateClienteRequest {
    pub nombre: Option<String>,
    pub rnc_cedula: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub limite_credito: Option<Decimal>,
    pub activo: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ClienteAbono {
    pub id: Uuid,
    pub cliente_id: Uuid,
    pub monto: Decimal,
    pub metodo_pago: String,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAbonoRequest {
    pub monto: Decimal,
    pub metodo_pago: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Proveedor {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre: String,
    pub rnc: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub contacto: Option<String>,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProveedorRequest {
    pub nombre: String,
    pub rnc: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub contacto: Option<String>,
}

/// Un proveedor con su saldo por pagar **derivado** (ver
/// `PartnerService::saldo_proveedor_sql`). No hay columna
/// `proveedores.saldo_pendiente`: el saldo se calcula para que no pueda
/// quedar desfasado del histórico de compras.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProveedorConSaldo {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre: String,
    pub rnc: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub contacto: Option<String>,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub saldo_pendiente: Decimal,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ProveedorAbono {
    pub id: Uuid,
    pub proveedor_id: Uuid,
    pub monto: Decimal,
    pub metodo_pago: String,
    pub nota: Option<String>,
    pub usuario_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProveedorAbonoRequest {
    pub monto: Decimal,
    pub metodo_pago: Option<String>,
    pub nota: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProveedorRequest {
    pub nombre: Option<String>,
    pub rnc: Option<String>,
    pub telefono: Option<String>,
    pub email: Option<String>,
    pub direccion: Option<String>,
    pub contacto: Option<String>,
    pub activo: Option<bool>,
}

pub struct PartnerService {
    pool: PgPool,
}

impl PartnerService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ---- Clientes ----

    const CLIENTES_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("nombre", "nombre"),
        ("saldo_pendiente", "saldo_pendiente"),
        ("created_at", "created_at"),
    ];

    pub async fn list_clientes(
        &self,
        tenant_id: &str,
        search: Option<String>,
        activo: Option<bool>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Cliente>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::bool IS NULL OR activo = $2)
               AND ($3::text IS NULL OR LOWER(nombre) LIKE $3 OR rnc_cedula LIKE $3)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM clientes {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::CLIENTES_SORTABLE, "nombre ASC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tenant_id, nombre, rnc_cedula, telefono, email, direccion, saldo_pendiente, limite_credito, activo, created_at
               FROM clientes
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $4 OFFSET $5"#
        );
        let rows = sqlx::query_as::<_, Cliente>(&query)
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn get_cliente(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<Cliente> {
        sqlx::query_as::<_, Cliente>(
            "SELECT id, tenant_id, nombre, rnc_cedula, telefono, email, direccion, saldo_pendiente, limite_credito, activo, created_at FROM clientes WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cliente no encontrado"))
    }

    pub async fn create_cliente(&self, tenant_id: &str, req: CreateClienteRequest) -> anyhow::Result<Cliente> {
        if req.nombre.trim().is_empty() {
            anyhow::bail!("El nombre del cliente es requerido");
        }
        let cliente = sqlx::query_as::<_, Cliente>(
            r#"INSERT INTO clientes (tenant_id, nombre, rnc_cedula, telefono, email, direccion, limite_credito)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id, tenant_id, nombre, rnc_cedula, telefono, email, direccion, saldo_pendiente, limite_credito, activo, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.nombre.trim())
        .bind(&req.rnc_cedula)
        .bind(&req.telefono)
        .bind(&req.email)
        .bind(&req.direccion)
        .bind(req.limite_credito.unwrap_or_default())
        .fetch_one(&self.pool)
        .await?;
        Ok(cliente)
    }

    pub async fn update_cliente(&self, tenant_id: &str, id: Uuid, req: UpdateClienteRequest) -> anyhow::Result<Cliente> {
        let existing = self.get_cliente(tenant_id, id).await?;
        let cliente = sqlx::query_as::<_, Cliente>(
            r#"UPDATE clientes SET nombre = $1, rnc_cedula = $2, telefono = $3, email = $4, direccion = $5, limite_credito = $6, activo = $7
               WHERE id = $8 AND tenant_id = $9
               RETURNING id, tenant_id, nombre, rnc_cedula, telefono, email, direccion, saldo_pendiente, limite_credito, activo, created_at"#,
        )
        .bind(req.nombre.unwrap_or(existing.nombre))
        .bind(req.rnc_cedula.or(existing.rnc_cedula))
        .bind(req.telefono.or(existing.telefono))
        .bind(req.email.or(existing.email))
        .bind(req.direccion.or(existing.direccion))
        .bind(req.limite_credito.unwrap_or(existing.limite_credito))
        .bind(req.activo.unwrap_or(existing.activo))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(cliente)
    }

    pub async fn delete_cliente(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE clientes SET activo = false WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Abono contra el saldo fiado de un cliente: baja `saldo_pendiente` y
    /// registra el ingreso en caja (a diferencia de una venta FIADO, aquí sí
    /// entra efectivo/transferencia real en este momento).
    pub async fn registrar_abono(
        &self,
        tenant_id: &str,
        cliente_id: Uuid,
        usuario_id: Uuid,
        req: CreateAbonoRequest,
    ) -> anyhow::Result<ClienteAbono> {
        if req.monto <= Decimal::ZERO {
            anyhow::bail!("El monto del abono debe ser mayor a cero");
        }
        let metodo_pago = req.metodo_pago.unwrap_or_else(|| "EFECTIVO".to_string());

        let mut tx = self.pool.begin().await?;

        let cliente = sqlx::query_as::<_, Cliente>(
            "SELECT id, tenant_id, nombre, rnc_cedula, telefono, email, direccion, saldo_pendiente, limite_credito, activo, created_at
             FROM clientes WHERE id = $1 AND tenant_id = $2 FOR UPDATE",
        )
        .bind(cliente_id)
        .bind(tenant_id)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Cliente no encontrado"))?;

        let nuevo_saldo = (cliente.saldo_pendiente - req.monto).max(Decimal::ZERO);
        sqlx::query("UPDATE clientes SET saldo_pendiente = $1 WHERE id = $2 AND tenant_id = $3")
            .bind(nuevo_saldo)
            .bind(cliente_id)
            .bind(tenant_id)
            .execute(&mut *tx)
            .await?;

        let abono = sqlx::query_as::<_, ClienteAbono>(
            r#"INSERT INTO cliente_abonos (tenant_id, cliente_id, monto, metodo_pago, usuario_id)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, cliente_id, monto, metodo_pago, usuario_id, created_at"#,
        )
        .bind(tenant_id)
        .bind(cliente_id)
        .bind(req.monto)
        .bind(&metodo_pago)
        .bind(usuario_id)
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query(
            r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, metodo_pago, referencia_tipo, referencia_id, usuario_id)
               VALUES ($1, 'INGRESO', 'Abono a cuenta de cliente', $2, $3, 'ABONO_CLIENTE', $4, $5)"#,
        )
        .bind(tenant_id)
        .bind(req.monto)
        .bind(&metodo_pago)
        .bind(abono.id)
        .bind(usuario_id)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(abono)
    }

    // ---- Proveedores ----

    const PROVEEDORES_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("nombre", "nombre"),
        ("created_at", "created_at"),
    ];

    pub async fn get_proveedor(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<Proveedor> {
        sqlx::query_as::<_, Proveedor>(
            "SELECT id, tenant_id, nombre, rnc, telefono, email, direccion, contacto, activo, created_at FROM proveedores WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Proveedor no encontrado"))
    }

    pub async fn create_proveedor(&self, tenant_id: &str, req: CreateProveedorRequest) -> anyhow::Result<Proveedor> {
        if req.nombre.trim().is_empty() {
            anyhow::bail!("El nombre del proveedor es requerido");
        }
        let proveedor = sqlx::query_as::<_, Proveedor>(
            r#"INSERT INTO proveedores (tenant_id, nombre, rnc, telefono, email, direccion, contacto)
               VALUES ($1, $2, $3, $4, $5, $6, $7)
               RETURNING id, tenant_id, nombre, rnc, telefono, email, direccion, contacto, activo, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.nombre.trim())
        .bind(&req.rnc)
        .bind(&req.telefono)
        .bind(&req.email)
        .bind(&req.direccion)
        .bind(&req.contacto)
        .fetch_one(&self.pool)
        .await?;
        Ok(proveedor)
    }

    pub async fn update_proveedor(&self, tenant_id: &str, id: Uuid, req: UpdateProveedorRequest) -> anyhow::Result<Proveedor> {
        let existing = self.get_proveedor(tenant_id, id).await?;
        let proveedor = sqlx::query_as::<_, Proveedor>(
            r#"UPDATE proveedores SET nombre = $1, rnc = $2, telefono = $3, email = $4, direccion = $5, contacto = $6, activo = $7
               WHERE id = $8 AND tenant_id = $9
               RETURNING id, tenant_id, nombre, rnc, telefono, email, direccion, contacto, activo, created_at"#,
        )
        .bind(req.nombre.unwrap_or(existing.nombre))
        .bind(req.rnc.or(existing.rnc))
        .bind(req.telefono.or(existing.telefono))
        .bind(req.email.or(existing.email))
        .bind(req.direccion.or(existing.direccion))
        .bind(req.contacto.or(existing.contacto))
        .bind(req.activo.unwrap_or(existing.activo))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(proveedor)
    }

    pub async fn delete_proveedor(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE proveedores SET activo = false WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ---- Abonos a proveedor (Fase F5) ----

    /// Saldo por pagar de un proveedor, **derivado** en vez de guardado en
    /// una columna: compras a crédito no anuladas, menos las notas de
    /// crédito de compra (devoluciones, que bajan la deuda), menos los
    /// abonos ya registrados.
    ///
    /// Solo cuentan las compras con `metodo_pago = 'FIADO'`: una compra de
    /// contado ya salió de caja al registrarse y nunca creó una cuenta por
    /// pagar (ver `contabilidad_service::sincronizar`, rama COMPRA).
    ///
    /// Se deriva a propósito: una columna `saldo_pendiente` sería una
    /// tercera fuente de verdad capaz de desfasarse del histórico de
    /// compras, que es justo el problema que docs/14 §4 señala con los tres
    /// saldos de caja/banco sin conciliar. El costo es que este subselect
    /// corre por fila del listado.
    const SALDO_PROVEEDOR_SQL: &'static str = r#"
        COALESCE((
            SELECT SUM(CASE WHEN c.tipo_documento = 'NOTA_CREDITO' THEN -c.total ELSE c.total END)
            FROM compras c
            WHERE c.proveedor_id = p.id AND c.tenant_id = p.tenant_id
              AND c.metodo_pago = 'FIADO' AND c.estado <> 'ANULADA'
        ), 0)
        - COALESCE((
            SELECT SUM(pa.monto) FROM proveedor_abonos pa
            WHERE pa.proveedor_id = p.id AND pa.tenant_id = p.tenant_id
        ), 0)
    "#;

    pub async fn list_proveedores_con_saldo(
        &self,
        tenant_id: &str,
        search: Option<String>,
        activo: Option<bool>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<ProveedorConSaldo>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE p.tenant_id = $1
               AND ($2::bool IS NULL OR p.activo = $2)
               AND ($3::text IS NULL OR LOWER(p.nombre) LIKE $3 OR p.rnc LIKE $3)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM proveedores p {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?;

        // `saldo_pendiente` no es ordenable: es una expresión derivada y el
        // allowlist de `SortParams` solo conoce columnas reales.
        let order_by = sort.resolve(Self::PROVEEDORES_SORTABLE, "nombre ASC");
        let order_by = order_by.replace("nombre", "p.nombre").replace("created_at", "p.created_at");
        let saldo = Self::SALDO_PROVEEDOR_SQL;
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT p.id, p.tenant_id, p.nombre, p.rnc, p.telefono, p.email, p.direccion, p.contacto, p.activo, p.created_at,
                      ({saldo}) AS saldo_pendiente
               FROM proveedores p
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $4 OFFSET $5"#
        );
        let rows = sqlx::query_as::<_, ProveedorConSaldo>(&query)
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn get_proveedor_con_saldo(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<ProveedorConSaldo> {
        let saldo = Self::SALDO_PROVEEDOR_SQL;
        sqlx::query_as::<_, ProveedorConSaldo>(&format!(
            r#"SELECT p.id, p.tenant_id, p.nombre, p.rnc, p.telefono, p.email, p.direccion, p.contacto, p.activo, p.created_at,
                      ({saldo}) AS saldo_pendiente
               FROM proveedores p WHERE p.id = $1 AND p.tenant_id = $2"#
        ))
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Proveedor no encontrado"))
    }

    pub async fn list_abonos_proveedor(&self, tenant_id: &str, proveedor_id: Uuid) -> anyhow::Result<Vec<ProveedorAbono>> {
        let rows = sqlx::query_as::<_, ProveedorAbono>(
            r#"SELECT id, proveedor_id, monto, metodo_pago, nota, usuario_id, created_at
               FROM proveedor_abonos WHERE tenant_id = $1 AND proveedor_id = $2
               ORDER BY created_at DESC"#,
        )
        .bind(tenant_id)
        .bind(proveedor_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    /// Pago a un proveedor contra su saldo de compras a crédito. Espejo de
    /// `registrar_abono` (lado cliente), en dirección contraria: aquí sale
    /// dinero.
    ///
    /// **Efectivo vs. transferencia/cheque**: solo el efectivo escribe un
    /// `caja_movimientos` EGRESO, porque la caja del colmado es dinero
    /// físico y una transferencia no la toca. El asiento contable es el
    /// mismo en ambos casos (`Dr 2110 / Cr 1100`) porque el plan de cuentas
    /// tiene una sola cuenta "1100 Caja y Bancos"; separar banco de caja en
    /// el mayor es un cambio de plan de cuentas fuera de esta fase.
    ///
    /// No se permite pagar más de lo que se debe: sin partidas abiertas por
    /// factura, un sobrepago dejaría al proveedor con saldo negativo y sin
    /// forma de explicarlo.
    pub async fn registrar_abono_proveedor(
        &self,
        tenant_id: &str,
        proveedor_id: Uuid,
        usuario_id: Uuid,
        req: CreateProveedorAbonoRequest,
    ) -> anyhow::Result<ProveedorAbono> {
        if req.monto <= Decimal::ZERO {
            anyhow::bail!("El monto del abono debe ser mayor a cero");
        }
        let metodo_pago = req.metodo_pago.unwrap_or_else(|| "EFECTIVO".to_string());
        if !["EFECTIVO", "TRANSFERENCIA", "CHEQUE"].contains(&metodo_pago.as_str()) {
            anyhow::bail!("Método de pago inválido: usa EFECTIVO, TRANSFERENCIA o CHEQUE");
        }

        let mut tx = self.pool.begin().await?;

        // FOR UPDATE sobre el proveedor: serializa dos abonos simultáneos al
        // mismo proveedor, de modo que el chequeo de saldo de abajo no pueda
        // leer un saldo obsoleto.
        let existe: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM proveedores WHERE id = $1 AND tenant_id = $2 FOR UPDATE")
            .bind(proveedor_id)
            .bind(tenant_id)
            .fetch_optional(&mut *tx)
            .await?;
        existe.ok_or_else(|| anyhow::anyhow!("Proveedor no encontrado"))?;

        let saldo = Self::SALDO_PROVEEDOR_SQL;
        let saldo_actual: Decimal = sqlx::query_scalar(&format!(
            "SELECT ({saldo}) FROM proveedores p WHERE p.id = $1 AND p.tenant_id = $2"
        ))
        .bind(proveedor_id)
        .bind(tenant_id)
        .fetch_one(&mut *tx)
        .await?;

        if saldo_actual <= Decimal::ZERO {
            anyhow::bail!("Este proveedor no tiene saldo pendiente por pagar");
        }
        if req.monto > saldo_actual {
            anyhow::bail!("El abono ({}) supera el saldo pendiente del proveedor ({})", req.monto, saldo_actual);
        }

        let abono = sqlx::query_as::<_, ProveedorAbono>(
            r#"INSERT INTO proveedor_abonos (tenant_id, proveedor_id, monto, metodo_pago, nota, usuario_id)
               VALUES ($1, $2, $3, $4, $5, $6)
               RETURNING id, proveedor_id, monto, metodo_pago, nota, usuario_id, created_at"#,
        )
        .bind(tenant_id)
        .bind(proveedor_id)
        .bind(req.monto)
        .bind(&metodo_pago)
        .bind(&req.nota)
        .bind(usuario_id)
        .fetch_one(&mut *tx)
        .await?;

        if metodo_pago == "EFECTIVO" {
            sqlx::query(
                r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, metodo_pago, referencia_tipo, referencia_id, usuario_id)
                   VALUES ($1, 'EGRESO', 'Abono a proveedor', $2, $3, 'ABONO_PROVEEDOR', $4, $5)"#,
            )
            .bind(tenant_id)
            .bind(req.monto)
            .bind(&metodo_pago)
            .bind(abono.id)
            .bind(usuario_id)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(abono)
    }
}

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

    pub async fn list_proveedores(
        &self,
        tenant_id: &str,
        search: Option<String>,
        activo: Option<bool>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Proveedor>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::bool IS NULL OR activo = $2)
               AND ($3::text IS NULL OR LOWER(nombre) LIKE $3 OR rnc LIKE $3)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM proveedores {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::PROVEEDORES_SORTABLE, "nombre ASC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tenant_id, nombre, rnc, telefono, email, direccion, contacto, activo, created_at
               FROM proveedores
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $4 OFFSET $5"#
        );
        let rows = sqlx::query_as::<_, Proveedor>(&query)
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

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
}

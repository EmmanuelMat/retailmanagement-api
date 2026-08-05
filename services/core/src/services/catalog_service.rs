//! Catalog Service - Categorías y Productos
//! Plain Postgres CRUD, tenant-scoped by RNC. No event sourcing / ledger -
//! stock_actual here is a simple counter; a real kardex is a future module.

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

const ITBIS_TIPOS: [&str; 3] = ["GRAVADO_18", "GRAVADO_16", "EXENTO"];

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Categoria {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre: String,
    pub color: Option<String>,
    pub icono: Option<String>,
    pub orden: i32,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateCategoriaRequest {
    pub nombre: String,
    pub color: Option<String>,
    pub icono: Option<String>,
    pub orden: Option<i32>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateCategoriaRequest {
    pub nombre: Option<String>,
    pub color: Option<String>,
    pub icono: Option<String>,
    pub orden: Option<i32>,
    pub activo: Option<bool>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct Producto {
    pub id: Uuid,
    pub tenant_id: String,
    pub categoria_id: Option<Uuid>,
    pub sku: String,
    pub codigo_barras: Option<String>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub unidad_medida: String,
    pub itbis_tipo: String,
    pub costo: Decimal,
    pub precio_venta: Decimal,
    pub stock_actual: Decimal,
    pub stock_minimo: Decimal,
    pub activo: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductoRequest {
    pub categoria_id: Option<Uuid>,
    pub sku: String,
    pub codigo_barras: Option<String>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub unidad_medida: Option<String>,
    pub itbis_tipo: Option<String>,
    pub costo: Option<Decimal>,
    pub precio_venta: Option<Decimal>,
    pub stock_actual: Option<Decimal>,
    pub stock_minimo: Option<Decimal>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductoRequest {
    pub categoria_id: Option<Uuid>,
    pub sku: Option<String>,
    pub codigo_barras: Option<String>,
    pub nombre: Option<String>,
    pub descripcion: Option<String>,
    pub unidad_medida: Option<String>,
    pub itbis_tipo: Option<String>,
    pub costo: Option<Decimal>,
    pub precio_venta: Option<Decimal>,
    pub stock_actual: Option<Decimal>,
    pub stock_minimo: Option<Decimal>,
    pub activo: Option<bool>,
}

pub struct CatalogService {
    pool: PgPool,
}

impl CatalogService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // ---- Categorías ----

    const CATEGORIAS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("nombre", "nombre"),
        ("orden", "orden"),
        ("created_at", "created_at"),
    ];

    pub async fn list_categorias(
        &self,
        tenant_id: &str,
        search: Option<String>,
        activo: Option<bool>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Categoria>, i64)> {
        let pattern = search.map(|s| s.trim().to_lowercase()).filter(|s| !s.is_empty()).map(|s| format!("%{}%", s));
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::bool IS NULL OR activo = $2)
               AND ($3::text IS NULL OR LOWER(nombre) LIKE $3)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM categorias {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::CATEGORIAS_SORTABLE, "orden ASC, nombre ASC");
        let limit = page.limit(20);
        let query = format!(
            r#"SELECT id, tenant_id, nombre, color, icono, orden, activo, created_at
               FROM categorias
               {WHERE_CLAUSE}
               ORDER BY {order_by}
               LIMIT $4 OFFSET $5"#
        );
        let rows = sqlx::query_as::<_, Categoria>(&query)
            .bind(tenant_id)
            .bind(activo)
            .bind(&pattern)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn create_categoria(&self, tenant_id: &str, req: CreateCategoriaRequest) -> anyhow::Result<Categoria> {
        if req.nombre.trim().is_empty() {
            anyhow::bail!("El nombre de la categoría es requerido");
        }
        let categoria = sqlx::query_as::<_, Categoria>(
            r#"INSERT INTO categorias (tenant_id, nombre, color, icono, orden)
               VALUES ($1, $2, $3, $4, $5)
               RETURNING id, tenant_id, nombre, color, icono, orden, activo, created_at"#,
        )
        .bind(tenant_id)
        .bind(req.nombre.trim())
        .bind(&req.color)
        .bind(&req.icono)
        .bind(req.orden.unwrap_or(0))
        .fetch_one(&self.pool)
        .await?;
        Ok(categoria)
    }

    pub async fn update_categoria(&self, tenant_id: &str, id: Uuid, req: UpdateCategoriaRequest) -> anyhow::Result<Categoria> {
        let existing = self.get_categoria(tenant_id, id).await?;
        let categoria = sqlx::query_as::<_, Categoria>(
            r#"UPDATE categorias SET nombre = $1, color = $2, icono = $3, orden = $4, activo = $5
               WHERE id = $6 AND tenant_id = $7
               RETURNING id, tenant_id, nombre, color, icono, orden, activo, created_at"#,
        )
        .bind(req.nombre.unwrap_or(existing.nombre))
        .bind(req.color.or(existing.color))
        .bind(req.icono.or(existing.icono))
        .bind(req.orden.unwrap_or(existing.orden))
        .bind(req.activo.unwrap_or(existing.activo))
        .bind(id)
        .bind(tenant_id)
        .fetch_one(&self.pool)
        .await?;
        Ok(categoria)
    }

    pub async fn delete_categoria(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE categorias SET activo = false WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn get_categoria(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<Categoria> {
        sqlx::query_as::<_, Categoria>(
            "SELECT id, tenant_id, nombre, color, icono, orden, activo, created_at
             FROM categorias WHERE id = $1 AND tenant_id = $2",
        )
        .bind(id)
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Categoría no encontrada"))
    }

    // ---- Productos ----

    const PRODUCTO_COLUMNS: &'static str = "id, tenant_id, categoria_id, sku, codigo_barras, nombre, descripcion, unidad_medida, itbis_tipo, costo, precio_venta, stock_actual, stock_minimo, activo, created_at, updated_at";

    const PRODUCTOS_SORTABLE: &'static [(&'static str, &'static str)] = &[
        ("nombre", "nombre"),
        ("sku", "sku"),
        ("precio_venta", "precio_venta"),
        ("stock_actual", "stock_actual"),
        ("created_at", "created_at"),
    ];

    pub async fn list_productos(
        &self,
        tenant_id: &str,
        categoria_id: Option<Uuid>,
        search: Option<String>,
        unidad_medida: Option<String>,
        activo: Option<bool>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Producto>, i64)> {
        let search_pattern = search
            .map(|s| s.trim().to_lowercase())
            .filter(|s| !s.is_empty())
            .map(|s| format!("%{}%", s));
        let unidad_medida = unidad_medida.filter(|s| !s.trim().is_empty());

        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::uuid IS NULL OR categoria_id = $2)
               AND ($3::text IS NULL OR LOWER(nombre) LIKE $3 OR LOWER(sku) LIKE $3)
               AND ($4::text IS NULL OR unidad_medida = $4)
               AND ($5::bool IS NULL OR activo = $5)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM productos {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(categoria_id)
            .bind(&search_pattern)
            .bind(&unidad_medida)
            .bind(activo)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::PRODUCTOS_SORTABLE, "nombre ASC");
        let limit = page.limit(20);
        let query = format!(
            "SELECT {} FROM productos {WHERE_CLAUSE} ORDER BY {order_by} LIMIT $6 OFFSET $7",
            Self::PRODUCTO_COLUMNS
        );
        let rows = sqlx::query_as::<_, Producto>(&query)
            .bind(tenant_id)
            .bind(categoria_id)
            .bind(&search_pattern)
            .bind(&unidad_medida)
            .bind(activo)
            .bind(limit)
            .bind(page.offset(20))
            .fetch_all(&self.pool)
            .await?;
        Ok((rows, total))
    }

    pub async fn get_producto(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<Producto> {
        let query = format!("SELECT {} FROM productos WHERE id = $1 AND tenant_id = $2", Self::PRODUCTO_COLUMNS);
        sqlx::query_as::<_, Producto>(&query)
            .bind(id)
            .bind(tenant_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))
    }

    pub async fn create_producto(&self, tenant_id: &str, req: CreateProductoRequest) -> anyhow::Result<Producto> {
        if req.sku.trim().is_empty() {
            anyhow::bail!("El SKU es requerido");
        }
        if req.nombre.trim().is_empty() {
            anyhow::bail!("El nombre del producto es requerido");
        }
        let itbis_tipo = req.itbis_tipo.unwrap_or_else(|| "GRAVADO_18".to_string());
        if !ITBIS_TIPOS.contains(&itbis_tipo.as_str()) {
            anyhow::bail!("itbisTipo inválido: debe ser GRAVADO_18, GRAVADO_16 o EXENTO");
        }
        let existing: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM productos WHERE tenant_id = $1 AND sku = $2")
            .bind(tenant_id)
            .bind(req.sku.trim())
            .fetch_optional(&self.pool)
            .await?;
        if existing.is_some() {
            anyhow::bail!("Ya existe un producto con SKU: {}", req.sku);
        }
        let query = format!(
            "INSERT INTO productos (tenant_id, categoria_id, sku, codigo_barras, nombre, descripcion, unidad_medida, itbis_tipo, costo, precio_venta, stock_actual, stock_minimo)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
             RETURNING {}",
            Self::PRODUCTO_COLUMNS
        );
        let producto = sqlx::query_as::<_, Producto>(&query)
            .bind(tenant_id)
            .bind(req.categoria_id)
            .bind(req.sku.trim())
            .bind(&req.codigo_barras)
            .bind(req.nombre.trim())
            .bind(&req.descripcion)
            .bind(req.unidad_medida.unwrap_or_else(|| "43".to_string()))
            .bind(itbis_tipo)
            .bind(req.costo.unwrap_or_default())
            .bind(req.precio_venta.unwrap_or_default())
            .bind(req.stock_actual.unwrap_or_default())
            .bind(req.stock_minimo.unwrap_or_default())
            .fetch_one(&self.pool)
            .await?;
        Ok(producto)
    }

    pub async fn update_producto(&self, tenant_id: &str, id: Uuid, req: UpdateProductoRequest) -> anyhow::Result<Producto> {
        let existing = self.get_producto(tenant_id, id).await?;
        let itbis_tipo = req.itbis_tipo.unwrap_or(existing.itbis_tipo);
        if !ITBIS_TIPOS.contains(&itbis_tipo.as_str()) {
            anyhow::bail!("itbisTipo inválido: debe ser GRAVADO_18, GRAVADO_16 o EXENTO");
        }
        let query = format!(
            "UPDATE productos SET categoria_id = $1, sku = $2, codigo_barras = $3, nombre = $4, descripcion = $5,
                 unidad_medida = $6, itbis_tipo = $7, costo = $8, precio_venta = $9, stock_actual = $10, stock_minimo = $11, activo = $12, updated_at = NOW()
             WHERE id = $13 AND tenant_id = $14
             RETURNING {}",
            Self::PRODUCTO_COLUMNS
        );
        let producto = sqlx::query_as::<_, Producto>(&query)
            // categoria_id is authoritative (not merged): the edit form always sends it,
            // either a uuid or null, so null must actually clear the category.
            .bind(req.categoria_id)
            .bind(req.sku.unwrap_or(existing.sku))
            .bind(req.codigo_barras.or(existing.codigo_barras))
            .bind(req.nombre.unwrap_or(existing.nombre))
            .bind(req.descripcion.or(existing.descripcion))
            .bind(req.unidad_medida.unwrap_or(existing.unidad_medida))
            .bind(itbis_tipo)
            .bind(req.costo.unwrap_or(existing.costo))
            .bind(req.precio_venta.unwrap_or(existing.precio_venta))
            .bind(req.stock_actual.unwrap_or(existing.stock_actual))
            .bind(req.stock_minimo.unwrap_or(existing.stock_minimo))
            .bind(req.activo.unwrap_or(existing.activo))
            .bind(id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(producto)
    }

    pub async fn delete_producto(&self, tenant_id: &str, id: Uuid) -> anyhow::Result<()> {
        sqlx::query("UPDATE productos SET activo = false, updated_at = NOW() WHERE id = $1 AND tenant_id = $2")
            .bind(id)
            .bind(tenant_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

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
    pub proveedor_id: Option<Uuid>,
    pub sku: String,
    pub codigo_barras: Option<String>,
    pub nombre: String,
    pub descripcion: Option<String>,
    pub unidad_medida: String,
    pub itbis_tipo: String,
    pub costo: Decimal,
    /// NULL solo cuando tipo = SERVICIO (sin precio fijo, se captura por
    /// línea al cotizar/facturar - ver ventas_service/cotizacion_service).
    pub precio_venta: Option<Decimal>,
    pub stock_actual: Decimal,
    pub stock_minimo: Decimal,
    pub activo: bool,
    pub imagen_url: Option<String>,
    /// PRODUCTO (default) | SERVICIO. SERVICIO no tiene stock ni precio fijo
    /// - ver create_producto/update_producto para la validación cruzada.
    pub tipo: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateProductoRequest {
    pub categoria_id: Option<Uuid>,
    pub proveedor_id: Option<Uuid>,
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
    pub tipo: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProductoRequest {
    pub categoria_id: Option<Uuid>,
    pub proveedor_id: Option<Uuid>,
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
    pub tipo: Option<String>,
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

    const PRODUCTO_COLUMNS: &'static str = "id, tenant_id, categoria_id, proveedor_id, sku, codigo_barras, nombre, descripcion, unidad_medida, itbis_tipo, costo, precio_venta, stock_actual, stock_minimo, activo, imagen_url, tipo, created_at, updated_at";

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
        tipo: Option<String>,
        page: &crate::pagination::PageParams,
        sort: &crate::pagination::SortParams,
    ) -> anyhow::Result<(Vec<Producto>, i64)> {
        let search_raw = search.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
        let search_pattern = search_raw.as_ref().map(|s| format!("%{}%", s.to_lowercase()));
        let unidad_medida = unidad_medida.filter(|s| !s.trim().is_empty());
        let tipo = tipo.filter(|s| !s.trim().is_empty());

        // Nombre/SKU: coincidencia parcial (LIKE). Código de barras: exacto —
        // así el flujo de escáner (que manda el código completo) no depende
        // de coincidencias parciales accidentales entre dos códigos.
        const WHERE_CLAUSE: &str = "WHERE tenant_id = $1
               AND ($2::uuid IS NULL OR categoria_id = $2)
               AND ($3::text IS NULL OR LOWER(nombre) LIKE $3 OR LOWER(sku) LIKE $3 OR codigo_barras = $6)
               AND ($4::text IS NULL OR unidad_medida = $4)
               AND ($5::bool IS NULL OR activo = $5)
               AND ($7::text IS NULL OR tipo = $7)";

        let total: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM productos {WHERE_CLAUSE}"))
            .bind(tenant_id)
            .bind(categoria_id)
            .bind(&search_pattern)
            .bind(&unidad_medida)
            .bind(activo)
            .bind(&search_raw)
            .bind(&tipo)
            .fetch_one(&self.pool)
            .await?;

        let order_by = sort.resolve(Self::PRODUCTOS_SORTABLE, "nombre ASC");
        let limit = page.limit(20);
        let query = format!(
            "SELECT {} FROM productos {WHERE_CLAUSE} ORDER BY {order_by} LIMIT $8 OFFSET $9",
            Self::PRODUCTO_COLUMNS
        );
        let rows = sqlx::query_as::<_, Producto>(&query)
            .bind(tenant_id)
            .bind(categoria_id)
            .bind(&search_pattern)
            .bind(&unidad_medida)
            .bind(activo)
            .bind(&search_raw)
            .bind(&tipo)
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
        let tipo = req.tipo.unwrap_or_else(|| "PRODUCTO".to_string());
        if tipo != "PRODUCTO" && tipo != "SERVICIO" {
            anyhow::bail!("tipo inválido: debe ser PRODUCTO o SERVICIO");
        }
        // Un Servicio nunca tiene precio fijo ni stock, sin importar lo que
        // mande el cliente - el precio se captura por línea al cotizar o
        // facturar (ver ventas_service::create_venta).
        let (precio_venta, stock_actual, stock_minimo): (Option<Decimal>, Decimal, Decimal) = if tipo == "SERVICIO" {
            (None, Decimal::ZERO, Decimal::ZERO)
        } else {
            (Some(req.precio_venta.unwrap_or_default()), req.stock_actual.unwrap_or_default(), req.stock_minimo.unwrap_or_default())
        };
        let existing: Option<(Uuid,)> = sqlx::query_as("SELECT id FROM productos WHERE tenant_id = $1 AND sku = $2")
            .bind(tenant_id)
            .bind(req.sku.trim())
            .fetch_optional(&self.pool)
            .await?;
        if existing.is_some() {
            anyhow::bail!("Ya existe un producto con SKU: {}", req.sku);
        }
        let query = format!(
            "INSERT INTO productos (tenant_id, categoria_id, proveedor_id, sku, codigo_barras, nombre, descripcion, unidad_medida, itbis_tipo, costo, precio_venta, stock_actual, stock_minimo, tipo)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
             RETURNING {}",
            Self::PRODUCTO_COLUMNS
        );
        let producto = sqlx::query_as::<_, Producto>(&query)
            .bind(tenant_id)
            .bind(req.categoria_id)
            .bind(req.proveedor_id)
            .bind(req.sku.trim())
            .bind(&req.codigo_barras)
            .bind(req.nombre.trim())
            .bind(&req.descripcion)
            .bind(req.unidad_medida.unwrap_or_else(|| "43".to_string()))
            .bind(itbis_tipo)
            .bind(req.costo.unwrap_or_default())
            .bind(precio_venta)
            .bind(stock_actual)
            .bind(stock_minimo)
            .bind(tipo)
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
        let tipo = req.tipo.unwrap_or(existing.tipo);
        if tipo != "PRODUCTO" && tipo != "SERVICIO" {
            anyhow::bail!("tipo inválido: debe ser PRODUCTO o SERVICIO");
        }
        let (precio_venta, stock_actual, stock_minimo): (Option<Decimal>, Decimal, Decimal) = if tipo == "SERVICIO" {
            (None, Decimal::ZERO, Decimal::ZERO)
        } else {
            (
                Some(req.precio_venta.unwrap_or_else(|| existing.precio_venta.unwrap_or_default())),
                req.stock_actual.unwrap_or(existing.stock_actual),
                req.stock_minimo.unwrap_or(existing.stock_minimo),
            )
        };
        let query = format!(
            "UPDATE productos SET categoria_id = $1, proveedor_id = $2, sku = $3, codigo_barras = $4, nombre = $5, descripcion = $6,
                 unidad_medida = $7, itbis_tipo = $8, costo = $9, precio_venta = $10, stock_actual = $11, stock_minimo = $12, activo = $13, tipo = $14, updated_at = NOW()
             WHERE id = $15 AND tenant_id = $16
             RETURNING {}",
            Self::PRODUCTO_COLUMNS
        );
        let producto = sqlx::query_as::<_, Producto>(&query)
            // categoria_id/proveedor_id are authoritative (not merged): the edit
            // form always sends them, either a uuid or null, so null must
            // actually clear the category/proveedor.
            .bind(req.categoria_id)
            .bind(req.proveedor_id)
            .bind(req.sku.unwrap_or(existing.sku))
            .bind(req.codigo_barras.or(existing.codigo_barras))
            .bind(req.nombre.unwrap_or(existing.nombre))
            .bind(req.descripcion.or(existing.descripcion))
            .bind(req.unidad_medida.unwrap_or(existing.unidad_medida))
            .bind(itbis_tipo)
            .bind(req.costo.unwrap_or(existing.costo))
            .bind(precio_venta)
            .bind(stock_actual)
            .bind(stock_minimo)
            .bind(req.activo.unwrap_or(existing.activo))
            .bind(tipo)
            .bind(id)
            .bind(tenant_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(producto)
    }

    pub async fn set_imagen(&self, tenant_id: &str, id: Uuid, imagen_url: &str) -> anyhow::Result<Producto> {
        let query = format!(
            "UPDATE productos SET imagen_url = $1, updated_at = NOW() WHERE id = $2 AND tenant_id = $3 RETURNING {}",
            Self::PRODUCTO_COLUMNS
        );
        let producto = sqlx::query_as::<_, Producto>(&query)
            .bind(imagen_url)
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

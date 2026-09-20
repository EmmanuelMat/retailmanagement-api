# Cotización / Orden de Servicio Pricing Rework Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every cotización line manually priced (not silently defaulted from the catalog), auto-generate product/service codes from their name, and move all pricing on órdenes de servicio to a single point — the moment a completed order is billed.

**Architecture:** Full-stack change across the Rust core service (`services/core`, Postgres-backed, plain CRUD-style services) and the Next.js frontend (`apps/web`). No new services or tables — additive/nullable-widening migration, three existing Rust services edited (`catalog_service.rs`, `cotizacion_service.rs`, `orden_servicio_service.rs`), two `main.rs` HTTP handlers reworked, and a new shared React picker component reused across three existing pages. POS (`pos/page.tsx`, `ventas_service.rs`) is explicitly untouched.

**Tech Stack:** Rust (axum, sqlx/Postgres, rust_decimal), Next.js App Router + TypeScript, Tailwind (`@repo/ui` components).

**Spec:** `docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md`

## Global Constraints

- No float for money anywhere — Rust uses `rust_decimal::Decimal`, TS sends/receives price fields as strings.
- POS (`pos/page.tsx`) and `ventas_service::create_venta`'s pricing branch are explicitly out of scope — do not modify their pricing behavior.
- `orden_servicio_materiales` and its `consumir` endpoint are explicitly out of scope — do not modify, only add a regression check.
- Money/DB migration changes must be additive or nullable-widening only — no destructive column drops, no backfill of historical rows.
- Every Rust service function that inserts/updates money fields keeps using `Decimal`, never string math.
- Follow existing file conventions exactly: one Rust service per business area, plain `sqlx::query_as` calls (no query builder/ORM), Next.js API routes are thin proxies to `CORE_HTTP_URL` (never re-implement business logic in a route.ts file).

---

## Task 1: Database migration — nullable pricing columns + line descripción

**Files:**
- Modify: `services/core/src/bin/migrate.rs:266` (after the `venta_items` block)
- Modify: `services/core/src/bin/migrate.rs:586` (after the `cotizacion_items` block)
- Modify: `services/core/src/bin/migrate.rs:891` (after the `orden_servicio_items` block)

**Interfaces:**
- Produces: `venta_items.descripcion` (nullable TEXT), `cotizacion_items.descripcion` (nullable TEXT), `orden_servicio_items.precio_unitario/itbis_tipo/itbis_monto/subtotal` (all now nullable) — every later task in this plan that touches these tables assumes these columns already exist/are nullable.

- [ ] **Step 1: Add `descripcion` to `venta_items`**

In `services/core/src/bin/migrate.rs`, immediately after the existing line:
```sql
ALTER TABLE venta_items ADD COLUMN IF NOT EXISTS descuento DECIMAL(12,2) NOT NULL DEFAULT 0;
```
add:
```sql
-- Nota de línea libre (ver docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md)
-- - viaja desde cotizacion_items.descripcion cuando una cotización se
-- convierte en venta; ausente/NULL para una venta directa (POS).
ALTER TABLE venta_items ADD COLUMN IF NOT EXISTS descripcion TEXT;
```

- [ ] **Step 2: Add `descripcion` to `cotizacion_items`**

Immediately after the line:
```sql
CREATE INDEX IF NOT EXISTS idx_cotizacion_items_cotizacion ON cotizacion_items(cotizacion_id);
```
add:
```sql
-- Nota de línea libre, p.ej. "2 habitaciones, tratamiento inicial" - ver
-- docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md.
ALTER TABLE cotizacion_items ADD COLUMN IF NOT EXISTS descripcion TEXT;
```

- [ ] **Step 3: Make orden de servicio item pricing columns nullable**

Immediately after the line:
```sql
CREATE INDEX IF NOT EXISTS idx_orden_servicio_items_orden ON orden_servicio_items(orden_servicio_id);
```
add:
```sql
-- El precio ya no vive en la orden de servicio - se captura una sola vez,
-- al facturar (ver http_facturar_orden en main.rs). Las columnas se dejan
-- nullable en vez de eliminarse: las órdenes ya facturadas antes de este
-- cambio conservan su precio histórico. Ver
-- docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md.
ALTER TABLE orden_servicio_items ALTER COLUMN precio_unitario DROP NOT NULL;
ALTER TABLE orden_servicio_items ALTER COLUMN itbis_tipo DROP NOT NULL;
ALTER TABLE orden_servicio_items ALTER COLUMN itbis_monto DROP NOT NULL;
ALTER TABLE orden_servicio_items ALTER COLUMN subtotal DROP NOT NULL;
```

- [ ] **Step 4: Apply and verify**

Run (from `services/core`, with the dev Postgres up — `docker-compose up -d` from the repo root first if it isn't already):
```bash
cargo run --bin migrate
```
Expected: `Running migrations...` followed by a clean exit, no SQL errors.

Verify the columns directly:
```bash
psql "$DATABASE_URL" -c "\d orden_servicio_items" -c "\d cotizacion_items" -c "\d venta_items"
```
Expected: `orden_servicio_items.precio_unitario/itbis_tipo/itbis_monto/subtotal` show no `not null`; `cotizacion_items` and `venta_items` each show a new `descripcion | text |` row.

- [ ] **Step 5: Commit**

```bash
git add services/core/src/bin/migrate.rs
git commit -m "feat(db): add line-level descripcion, make orden de servicio item pricing nullable"
```

---

## Task 2: Cotización — always-manual price + line descripción (+ carries into venta)

**Files:**
- Modify: `services/core/src/services/cotizacion_service.rs`
- Modify: `services/core/src/services/ventas_service.rs`
- Modify: `services/core/src/main.rs:2713-2726` (`http_convertir_cotizacion`)
- Create: `services/core/tests/cotizaciones.rs`

**Interfaces:**
- Consumes: `services/core/tests/common/mod.rs`'s `register_tenant()`, `TenantSession::post/post_expect/get`, `create_producto(session, precio_venta, stock_actual)`, `create_servicio(session)`, `abrir_caja(session, monto)`, `decimal_field`, `assert_decimal_eq` (all exist today, unchanged).
- Produces: `CotizacionItem.descripcion: Option<String>`, `CreateCotizacionItemRequest.descripcion: Option<String>`, `CreateVentaItemRequest.descripcion: Option<String>`, `VentaItem.descripcion: Option<String>` — used by Task 4 (`http_facturar_orden` sets `descripcion: None` explicitly) and any later frontend task that reads these shapes.

- [ ] **Step 1: Write the failing tests**

Create `services/core/tests/cotizaciones.rs`:
```rust
//! Cotizaciones - Módulo 5b. Cubre el requisito de precio manual en toda
//! línea (PRODUCTO incluido, no solo SERVICIO - ver
//! docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md)
//! y que la descripción de línea sobrevive a la conversión en venta.

mod common;
use common::*;
use rust_decimal_macros::dec;
use serde_json::json;

#[tokio::test]
async fn cotizacion_requiere_precio_unitario_para_un_producto_normal() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;

    let (status, body) = session
        .post_expect(
            "/v1/cotizaciones",
            json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }] }),
        )
        .await;
    assert_eq!(status, 400, "un PRODUCTO sin precio_unitario manual debe rechazarse: {body}");
}

#[tokio::test]
async fn cotizacion_usa_precio_manual_para_producto_y_servicio_ignorando_el_catalogo() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;
    let servicio = create_servicio(&session).await;

    let cotizacion = session
        .post(
            "/v1/cotizaciones",
            json!({ "items": [
                { "producto_id": producto["id"], "cantidad": "2", "precio_unitario": "300.00", "descripcion": "2 habitaciones" },
                { "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "1500.00" }
            ] }),
        )
        .await;

    // Precio manual (300, no los 200 del catálogo) * 2 = 600; + servicio 1500 => 2100 subtotal, 18% = 378 itbis.
    assert_decimal_eq(decimal_field(&cotizacion, "subtotal"), dec!(2100.00), "subtotal usa el precio manual, no el de catálogo");
    assert_decimal_eq(decimal_field(&cotizacion, "itbis_total"), dec!(378.00), "itbis");
    let items = cotizacion["items"].as_array().unwrap();
    let linea_producto = items.iter().find(|i| i["producto_id"] == producto["id"]).unwrap();
    assert_eq!(linea_producto["descripcion"], "2 habitaciones");
}

#[tokio::test]
async fn convertir_cotizacion_a_venta_conserva_la_descripcion_de_linea() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;

    let cotizacion = session
        .post(
            "/v1/cotizaciones",
            json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1", "precio_unitario": "250.00", "descripcion": "Urgente, entregar hoy" }] }),
        )
        .await;
    let cot_id = cotizacion["id"].as_str().unwrap();

    let venta = session.post(&format!("/v1/cotizaciones/{cot_id}/convertir"), json!({})).await;
    let items = venta["items"].as_array().unwrap();
    assert_eq!(items[0]["descripcion"], "Urgente, entregar hoy", "la descripción de la cotización debe viajar a la venta: {venta}");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

From `services/core`:
```bash
cargo test --test cotizaciones
```
Expected: compile error or `400`/field-mismatch failures — `descripcion` doesn't exist yet on any response, and `PRODUCTO` without `precio_unitario` currently succeeds (falls back to catalog price).

- [ ] **Step 3: Update `cotizacion_service.rs` structs**

In `services/core/src/services/cotizacion_service.rs`, change:
```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CotizacionItem {
    pub id: Uuid,
    pub cotizacion_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub descuento: Decimal,
    pub itbis_tipo: String,
    pub itbis_monto: Decimal,
    pub subtotal: Decimal,
}
```
to:
```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CotizacionItem {
    pub id: Uuid,
    pub cotizacion_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub cantidad: Decimal,
    pub precio_unitario: Decimal,
    pub descuento: Decimal,
    pub itbis_tipo: String,
    pub itbis_monto: Decimal,
    pub subtotal: Decimal,
    pub descripcion: Option<String>,
}
```
and change:
```rust
#[derive(Debug, Deserialize)]
pub struct CreateCotizacionItemRequest {
    pub producto_id: Uuid,
    pub cantidad: Decimal,
    pub descuento: Option<Decimal>,
    /// Requerido cuando el producto es tipo SERVICIO - ver el mismo campo en
    /// ventas_service::CreateVentaItemRequest.
    pub precio_unitario: Option<Decimal>,
}
```
to:
```rust
#[derive(Debug, Deserialize)]
pub struct CreateCotizacionItemRequest {
    pub producto_id: Uuid,
    pub cantidad: Decimal,
    pub descuento: Option<Decimal>,
    /// Siempre requerido - a diferencia de ventas_service::CreateVentaItemRequest,
    /// una cotización nunca usa el precio de catálogo automáticamente (ver
    /// docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md).
    pub precio_unitario: Option<Decimal>,
    /// Nota de línea libre, opcional.
    pub descripcion: Option<String>,
}
```

- [ ] **Step 4: Update `create_cotizacion`**

Replace the item loop (currently `let row: Option<(String, String, Option<Decimal>, String, String)> = ...` through `lineas.push(...)`) with:
```rust
for item in &req.items {
    if item.cantidad <= Decimal::ZERO {
        anyhow::bail!("La cantidad debe ser mayor a cero");
    }
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT sku, nombre, itbis_tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
    )
    .bind(item.producto_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (sku, nombre, itbis_tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;

    let precio_venta = item.precio_unitario.ok_or_else(|| anyhow::anyhow!("Falta precio_unitario para {}", nombre))?;
    if precio_venta <= Decimal::ZERO {
        anyhow::bail!("precio_unitario inválido para {}", nombre);
    }

    let descuento = item.descuento.unwrap_or_default();
    let line_bruto = precio_venta * item.cantidad;
    if descuento < Decimal::ZERO || descuento > line_bruto {
        anyhow::bail!("Descuento inválido para {}", nombre);
    }
    let line_subtotal = line_bruto - descuento;
    let line_itbis = line_subtotal * itbis_rate(&itbis_tipo);
    subtotal_total += line_subtotal;
    itbis_total += line_itbis;

    lineas.push((item.producto_id, sku, nombre, item.cantidad, precio_venta, descuento, itbis_tipo, line_itbis, line_subtotal, item.descripcion.clone()));
}
```
Update the `lineas` declaration just above the loop from:
```rust
let mut lineas: Vec<(Uuid, String, String, Decimal, Decimal, Decimal, String, Decimal, Decimal)> = Vec::new();
```
to:
```rust
let mut lineas: Vec<(Uuid, String, String, Decimal, Decimal, Decimal, String, Decimal, Decimal, Option<String>)> = Vec::new();
```
Then update the insert loop right after `tx.commit()`'s preceding block:
```rust
let mut items = Vec::new();
for (producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, line_subtotal, descripcion) in lineas {
    let ci = sqlx::query_as::<_, CotizacionItem>(
        r#"INSERT INTO cotizacion_items (cotizacion_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, descripcion)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
           RETURNING id, cotizacion_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, descripcion"#,
    )
    .bind(cotizacion.id)
    .bind(producto_id)
    .bind(&sku)
    .bind(&nombre)
    .bind(cantidad)
    .bind(precio_unitario)
    .bind(descuento)
    .bind(&itbis_tipo)
    .bind(itbis_monto)
    .bind(line_subtotal)
    .bind(&descripcion)
    .fetch_one(&mut *tx)
    .await?;
    items.push(ci);
}
```

- [ ] **Step 5: Update `add_item`**

Apply the identical transformation to `add_item` (same file, the `pub async fn add_item` below `recalcular_totales`): drop the `tipo`-based branch, drop `precio_venta`/`tipo` from the `SELECT`/destructure, require `item.precio_unitario` unconditionally, and add `descripcion` to both the `INSERT` column list and its `RETURNING` clause, binding `&item.descripcion` as the 11th bind parameter.

- [ ] **Step 6: Update `get_cotizacion`'s items SELECT**

Change:
```rust
let items = sqlx::query_as::<_, CotizacionItem>(
    "SELECT id, cotizacion_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal FROM cotizacion_items WHERE cotizacion_id = $1",
)
```
to:
```rust
let items = sqlx::query_as::<_, CotizacionItem>(
    "SELECT id, cotizacion_id, producto_id, sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, descripcion FROM cotizacion_items WHERE cotizacion_id = $1",
)
```

- [ ] **Step 7: Add `descripcion` to `ventas_service.rs`**

In `services/core/src/services/ventas_service.rs`, add to `VentaItem` (after `pub subtotal: Decimal,`):
```rust
    /// Nota de línea libre - viaja desde cotizacion_items.descripcion cuando
    /// la venta viene de convertir una cotización; NULL para una venta
    /// directa (POS).
    pub descripcion: Option<String>,
```
And add to `CreateVentaItemRequest` (after `pub precio_unitario: Option<Decimal>,`):
```rust
    pub descripcion: Option<String>,
```
Find the `INSERT INTO venta_items` statement inside `create_venta` and add `descripcion` to its column list, its `VALUES` placeholder list (one more `$N`), its `RETURNING` clause, and bind `&item.descripcion` alongside the other per-item binds. (There is exactly one `INSERT INTO venta_items` in this file — locate it with `grep -n "INSERT INTO venta_items" services/core/src/services/ventas_service.rs`.)

- [ ] **Step 8: Wire `descripcion` through `http_convertir_cotizacion`**

In `services/core/src/main.rs`, in `http_convertir_cotizacion`'s `venta_req` construction (~line 2713-2726), add `descripcion: it.descripcion.clone(),` to the `CreateVentaItemRequest { ... }` literal.

- [ ] **Step 9: Run migrate, then the tests**

```bash
cargo run --bin migrate
cargo test --test cotizaciones
```
Expected: all 3 tests pass.

- [ ] **Step 10: Run the full existing suite to confirm no regressions**

```bash
cargo test --tests
```
Expected: all tests pass, including `servicio_producto.rs` (untouched — exercises only `/v1/ventas`, `/v1/productos`, `/v1/compras`) and `ordenes_servicio.rs`'s `cotizacion_se_convierte_en_orden_de_servicio_reusando_los_precios_ya_fijados` (still passes — it posts `precio_unitario` on the cotización item, which is unaffected by this task; Task 5 rewrites this specific test's assertions once Task 3 lands).

- [ ] **Step 11: Commit**

```bash
git add services/core/src/services/cotizacion_service.rs services/core/src/services/ventas_service.rs services/core/src/main.rs services/core/tests/cotizaciones.rs
git commit -m "feat(cotizaciones): always require manual precio_unitario, add line descripcion"
```

---

## Task 3: Orden de servicio — items lose pricing entirely

**Files:**
- Modify: `services/core/src/services/orden_servicio_service.rs`

**Interfaces:**
- Consumes: nothing new.
- Produces: `OrdenServicioItem.precio_unitario/itbis_tipo/itbis_monto/subtotal: Option<...>`, `CreateOrdenServicioItemRequest` with only `producto_id, cantidad, tecnico_id, observaciones` — Task 4's `http_facturar_orden` and `http_convertir_cotizacion_a_orden` consume this exact shape; Task 5's test rewrite asserts against it.

- [ ] **Step 1: Update `OrdenServicioItem` and `CreateOrdenServicioItemRequest`**

Change:
```rust
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
```
to:
```rust
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct OrdenServicioItem {
    pub id: Uuid,
    pub orden_servicio_id: Uuid,
    pub producto_id: Uuid,
    pub sku: String,
    pub nombre: String,
    pub tipo: String,
    pub cantidad: Decimal,
    /// El precio ya no vive aquí - se captura una sola vez, al facturar (ver
    /// http_facturar_orden en main.rs). NULL para toda orden creada desde
    /// este cambio en adelante; una orden vieja conserva su precio histórico.
    pub precio_unitario: Option<Decimal>,
    pub descuento: Decimal,
    pub itbis_tipo: Option<String>,
    pub itbis_monto: Option<Decimal>,
    pub subtotal: Option<Decimal>,
    pub tecnico_id: Option<Uuid>,
    pub observaciones: Option<String>,
}
```
Change:
```rust
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
```
to:
```rust
#[derive(Debug, Deserialize)]
pub struct CreateOrdenServicioItemRequest {
    pub producto_id: Uuid,
    pub cantidad: Decimal,
    pub tecnico_id: Option<Uuid>,
    /// Nota de línea libre - esta es la "descripción" que el usuario ve; no
    /// hay columna de precio en esta tabla (ver
    /// docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md).
    pub observaciones: Option<String>,
}
```

- [ ] **Step 2: Simplify `create_orden`**

Replace the whole item-processing block — from `let mut subtotal_total = Decimal::ZERO;` through the end of the `for item in &req.items` loop — with:
```rust
let mut lineas: Vec<(Uuid, String, String, String, Decimal, Option<Uuid>, Option<String>)> = Vec::new();

for item in &req.items {
    if item.cantidad <= Decimal::ZERO {
        anyhow::bail!("La cantidad debe ser mayor a cero");
    }
    let row: Option<(String, String, String)> = sqlx::query_as(
        "SELECT sku, nombre, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
    )
    .bind(item.producto_id)
    .bind(tenant_id)
    .fetch_optional(&mut *tx)
    .await?;
    let (sku, nombre, tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;

    lineas.push((item.producto_id, sku, nombre, tipo, item.cantidad, item.tecnico_id, item.observaciones.clone()));
}
```
Then replace the `ordenes_servicio` insert (which currently binds `subtotal_total, descuento_total, itbis_total, total`) with one that drops those four columns entirely (they keep their `DEFAULT 0`):
```rust
let orden = sqlx::query_as::<_, OrdenServicio>(&format!(
    r#"INSERT INTO ordenes_servicio
           (tenant_id, cliente_id, cotizacion_id, condicion_id, prioridad, fecha_programada,
            direccion, descripcion, notas, usuario_id)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
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
.bind(&req.notas)
.bind(usuario_id)
.fetch_one(&mut *tx)
.await?;
```
Then replace the item-insert loop with:
```rust
let mut items = Vec::new();
for (producto_id, sku, nombre, tipo, cantidad, tecnico_id, observaciones) in lineas {
    let it = sqlx::query_as::<_, OrdenServicioItem>(
        r#"INSERT INTO orden_servicio_items
               (orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, tecnico_id, observaciones)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           RETURNING id, orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, tecnico_id, observaciones"#,
    )
    .bind(orden.id)
    .bind(producto_id)
    .bind(&sku)
    .bind(&nombre)
    .bind(&tipo)
    .bind(cantidad)
    .bind(tecnico_id)
    .bind(&observaciones)
    .fetch_one(&mut *tx)
    .await?;
    items.push(it);
}
```
(`descuento` is omitted from the insert column list too — it keeps its `DEFAULT 0`.)

- [ ] **Step 3: Simplify `add_item`**

In the same file's `add_item`, replace:
```rust
let row: Option<(String, String, Option<Decimal>, String, String)> = sqlx::query_as(
    "SELECT sku, nombre, precio_venta, itbis_tipo, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
)
.bind(item.producto_id)
.bind(tenant_id)
.fetch_optional(&mut *tx)
.await?;
let (sku, nombre, precio_venta_catalogo, itbis_tipo, tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
```
with:
```rust
let row: Option<(String, String, String)> = sqlx::query_as(
    "SELECT sku, nombre, tipo FROM productos WHERE id = $1 AND tenant_id = $2 AND activo = true",
)
.bind(item.producto_id)
.bind(tenant_id)
.fetch_optional(&mut *tx)
.await?;
let (sku, nombre, tipo) = row.ok_or_else(|| anyhow::anyhow!("Producto no encontrado"))?;
```
Keep the "evita el doble descuento de stock" `if tipo == "PRODUCTO"` guard block exactly as-is (it doesn't touch price). Then replace everything from `let precio_unitario = if tipo == "SERVICIO" {` through the `INSERT` with:
```rust
let it = sqlx::query_as::<_, OrdenServicioItem>(
    r#"INSERT INTO orden_servicio_items
           (orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, tecnico_id, observaciones)
       VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
       RETURNING id, orden_servicio_id, producto_id, sku, nombre, tipo, cantidad, precio_unitario, descuento, itbis_tipo, itbis_monto, subtotal, tecnico_id, observaciones"#,
)
.bind(orden_id)
.bind(item.producto_id)
.bind(&sku)
.bind(&nombre)
.bind(&tipo)
.bind(item.cantidad)
.bind(item.tecnico_id)
.bind(&item.observaciones)
.fetch_one(&mut *tx)
.await?;

self.recalcular_totales(&mut tx, orden_id).await?;
tx.commit().await?;
Ok(it)
```
(Leave `recalcular_totales` itself and its call from `remove_item` unchanged — with every item's price columns null, its `COALESCE(SUM(...), 0)` aggregation already yields `0` correctly, so it stays a harmless no-op rather than something that needs deleting.)

- [ ] **Step 4: Build to catch any remaining type errors**

```bash
cargo build
```
Expected: compiles clean. Fix any leftover reference to the removed `precio_unitario`/`descuento` fields on `CreateOrdenServicioItemRequest` (there should be none outside `main.rs`, which Task 4 updates next).

- [ ] **Step 5: Commit**

```bash
git add services/core/src/services/orden_servicio_service.rs
git commit -m "feat(ordenes-servicio): items carry no price - moved to facturación step"
```

---

## Task 4: Facturación — price entered at billing, not before

**Files:**
- Modify: `services/core/src/main.rs:3191-3254` (`FacturarOrdenRequest`, `http_facturar_orden`)
- Modify: `services/core/src/main.rs:3276-3293` (`http_convertir_cotizacion_a_orden`)

**Interfaces:**
- Consumes: `CreateOrdenServicioItemRequest { producto_id, cantidad, tecnico_id, observaciones }` (Task 3), `services::ventas_service::CreateVentaItemRequest { producto_id, cantidad, descuento, precio_unitario, descripcion }` (Task 2).
- Produces: `POST /v1/ordenes-servicio/:id/crear-factura` now accepts `{ metodo_pago?, tipo_ecf?, aprobacion_admin?, items?: [{ producto_id, precio_unitario?, descuento? }] }` — Task 5's test rewrite and the frontend Facturación tab (Task 12) both target this exact shape.

- [ ] **Step 1: Add price-at-billing to `FacturarOrdenRequest`**

Change:
```rust
struct FacturarOrdenRequest {
    metodo_pago: Option<String>,
    tipo_ecf: Option<i32>,
    aprobacion_admin: Option<AprobacionAdmin>,
}
```
to:
```rust
#[derive(Debug, Deserialize)]
struct ItemPrecioFactura {
    producto_id: Uuid,
    precio_unitario: Option<rust_decimal::Decimal>,
    descuento: Option<rust_decimal::Decimal>,
}

struct FacturarOrdenRequest {
    metodo_pago: Option<String>,
    tipo_ecf: Option<i32>,
    aprobacion_admin: Option<AprobacionAdmin>,
    /// Precio por línea, capturado una sola vez aquí - la orden ya no lo
    /// guarda (ver docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md).
    /// Solo hace falta una entrada por cada item de tipo SERVICIO; un
    /// PRODUCTO en la orden usa su precio de catálogo sin importar lo que
    /// venga aquí (ver ventas_service::create_venta, que ignora
    /// precio_unitario para tipo PRODUCTO).
    #[serde(default)]
    items: Vec<ItemPrecioFactura>,
}
```
(`FacturarOrdenRequest` already derives `#[derive(Debug, Deserialize)]` immediately above the struct — leave that line as-is, just add the new `items` field and the new `ItemPrecioFactura` struct above it.)

- [ ] **Step 2: Source price from the request in `http_facturar_orden`**

Change:
```rust
    let venta_req = services::ventas_service::CreateVentaRequest {
        cliente_id: orden_completa.orden.cliente_id,
        items: orden_completa.items.iter().map(|it| services::ventas_service::CreateVentaItemRequest {
            producto_id: it.producto_id,
            cantidad: it.cantidad,
            descuento: Some(it.descuento),
            precio_unitario: Some(it.precio_unitario),
        }).collect(),
        metodo_pago: req.metodo_pago,
        tipo_ecf: req.tipo_ecf,
        entrega_diferida: None,
    };
```
to:
```rust
    let venta_req = services::ventas_service::CreateVentaRequest {
        cliente_id: orden_completa.orden.cliente_id,
        items: orden_completa.items.iter().map(|it| {
            let precio = req.items.iter().find(|p| p.producto_id == it.producto_id);
            services::ventas_service::CreateVentaItemRequest {
                producto_id: it.producto_id,
                cantidad: it.cantidad,
                descuento: precio.and_then(|p| p.descuento),
                precio_unitario: precio.and_then(|p| p.precio_unitario),
                descripcion: it.observaciones.clone(),
            }
        }).collect(),
        metodo_pago: req.metodo_pago,
        tipo_ecf: req.tipo_ecf,
        entrega_diferida: None,
    };
```

- [ ] **Step 3: Stop forwarding price in `http_convertir_cotizacion_a_orden`**

Change:
```rust
        items: cotizacion_completa.items.iter().map(|it| services::orden_servicio_service::CreateOrdenServicioItemRequest {
            producto_id: it.producto_id,
            cantidad: it.cantidad,
            descuento: Some(it.descuento),
            precio_unitario: Some(it.precio_unitario),
            tecnico_id: None,
            observaciones: None,
        }).collect(),
```
to:
```rust
        items: cotizacion_completa.items.iter().map(|it| services::orden_servicio_service::CreateOrdenServicioItemRequest {
            producto_id: it.producto_id,
            cantidad: it.cantidad,
            tecnico_id: None,
            observaciones: it.descripcion.clone(),
        }).collect(),
```

- [ ] **Step 4: Build**

```bash
cargo build
```
Expected: compiles clean.

- [ ] **Step 5: Commit**

```bash
git add services/core/src/main.rs
git commit -m "feat(ordenes-servicio): capture price at facturación, not at conversion"
```

---

## Task 5: Rewrite `ordenes_servicio.rs` integration tests for the de-priced order

**Files:**
- Modify: `services/core/tests/ordenes_servicio.rs`

**Interfaces:**
- Consumes: the Task 3/4 contract — `POST /v1/ordenes-servicio` items no longer accept/require `precio_unitario`; `POST .../crear-factura` now takes `{ items: [{ producto_id, precio_unitario }] }`.

- [ ] **Step 1: Strip `precio_unitario` from every orden-creation call in this file**

In `services/core/tests/ordenes_servicio.rs`, every `session.post("/v1/ordenes-servicio", json!({ "items": [...] }))` call currently includes `"precio_unitario": "..."` on the servicio line. Remove that key from the JSON in these five tests (leave `producto_id`/`cantidad` as-is): `transiciones_de_estado_invalidas_son_rechazadas`, `asignar_tecnico_y_agregar_nota`, `consumir_material_mueve_inventario_real_una_sola_vez`, `una_orden_de_servicio_no_es_visible_para_otro_tenant`, `rol_sin_el_permiso_ordenes_servicio_gestionar_es_rechazado`. Also strip it from the two-item creation calls in `no_se_puede_facturar_y_consumir_material_el_mismo_producto_en_la_misma_orden` and `orden_completada_se_factura_como_venta_real_y_no_se_puede_facturar_dos_veces`.

- [ ] **Step 2: Rewrite `crear_orden_con_servicio_y_producto_calcula_totales_correctamente`**

Replace the whole test with:
```rust
#[tokio::test]
async fn crear_orden_no_registra_precio_en_los_items() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;

    let orden = session
        .post(
            "/v1/ordenes-servicio",
            json!({
                "items": [
                    { "producto_id": servicio["id"], "cantidad": "1" },
                    { "producto_id": producto["id"], "cantidad": "2" }
                ]
            }),
        )
        .await;

    assert_eq!(orden["estado"], "BORRADOR");
    assert_decimal_eq(decimal_field(&orden, "subtotal"), dec!(0), "una orden nunca lleva precio");
    assert_decimal_eq(decimal_field(&orden, "itbis_total"), dec!(0), "una orden nunca lleva ITBIS");
    assert_decimal_eq(decimal_field(&orden, "total"), dec!(0), "una orden nunca lleva total");
    let items = orden["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    assert!(items[0]["precio_unitario"].is_null(), "un item de orden no guarda precio: {items:?}");
}
```

- [ ] **Step 3: Rewrite `servicio_sin_precio_unitario_es_rechazado`**

Replace it with a test of the new invariant — price is required at facturación, not at creation:
```rust
#[tokio::test]
async fn facturar_sin_precio_de_servicio_es_rechazado() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let orden = session
        .post("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1" }] }))
        .await;
    let orden_id = orden["id"].as_str().unwrap();

    session.post(&format!("/v1/ordenes-servicio/{orden_id}/iniciar"), json!({})).await;
    session.post(&format!("/v1/ordenes-servicio/{orden_id}/completar"), json!({})).await;
    abrir_caja(&session, dec!(1000)).await;

    // Sin "items" en el body de crear-factura, el SERVICIO no tiene precio.
    let (status, body) = session.post_expect(&format!("/v1/ordenes-servicio/{orden_id}/crear-factura"), json!({})).await;
    assert_eq!(status, 400, "facturar un SERVICIO sin precio debe rechazarse: {body}");
}
```

- [ ] **Step 4: Update `orden_completada_se_factura_como_venta_real_y_no_se_puede_facturar_dos_veces`**

Change the `crear-factura` call from:
```rust
    let venta = session.post(&format!("/v1/ordenes-servicio/{orden_id}/crear-factura"), json!({})).await;
```
to:
```rust
    let venta = session
        .post(
            &format!("/v1/ordenes-servicio/{orden_id}/crear-factura"),
            json!({ "items": [{ "producto_id": servicio["id"], "precio_unitario": "1500" }] }),
        )
        .await;
```
Leave every other assertion in this test unchanged — `producto` (qty 2 @ catalog price 200 = 400) still auto-prices from the catalog since it isn't listed in `items`, so the expected total stays `dec!(2242.00)` (1500 + 400 = 1900 subtotal, 18% = 342 itbis).

- [ ] **Step 5: Update `cotizacion_se_convierte_en_orden_de_servicio_reusando_los_precios_ya_fijados`**

Rename it (the orden no longer reuses any price) and fix its assertion:
```rust
#[tokio::test]
async fn cotizacion_se_convierte_en_orden_de_servicio_sin_llevar_precio() {
    let session = register_tenant().await;
    let cliente = create_cliente(&session).await;
    let servicio = create_servicio(&session).await;

    let cotizacion = session
        .post(
            "/v1/cotizaciones",
            json!({ "cliente_id": cliente["id"], "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "2000" }] }),
        )
        .await;
    let cot_id = cotizacion["id"].as_str().unwrap();

    let orden = session.post(&format!("/v1/cotizaciones/{cot_id}/convertir-a-orden"), json!({})).await;
    assert_eq!(orden["cliente_id"], cliente["id"]);
    assert_eq!(orden["cotizacion_id"], cot_id);
    assert_decimal_eq(decimal_field(&orden, "subtotal"), dec!(0), "la orden no lleva el precio de la cotización - se vuelve a pedir al facturar");

    let cotizacion_actualizada = session.get(&format!("/v1/cotizaciones/{cot_id}")).await;
    assert_eq!(cotizacion_actualizada["estado"], "CONVERTIDA");
    assert!(cotizacion_actualizada["venta_id"].is_null(), "no hay venta todavía - solo una orden de servicio");

    // Una cotización ya convertida no puede volver a convertirse.
    let (status, _) = session.post_expect(&format!("/v1/cotizaciones/{cot_id}/convertir-a-orden"), json!({})).await;
    assert_eq!(status, 400);
}
```

- [ ] **Step 6: Run the file's tests**

```bash
cargo test --test ordenes_servicio
```
Expected: all tests pass.

- [ ] **Step 7: Run the full suite**

```bash
cargo test --tests
```
Expected: all tests across all files pass.

- [ ] **Step 8: Commit**

```bash
git add services/core/tests/ordenes_servicio.rs
git commit -m "test(ordenes-servicio): update for de-priced items and facturación-time pricing"
```

---

## Task 6: Auto-generated product/service code endpoint

**Files:**
- Modify: `services/core/src/services/catalog_service.rs`
- Modify: `services/core/src/main.rs` (new route + handler, near the existing `/v1/productos` routes)
- Create: `services/core/tests/productos.rs`

**Interfaces:**
- Produces: `GET /v1/productos/sugerir-codigo?nombre=...` → `{ "sku": "FUM-0001" }` — consumed by Task 7's `producto-form.tsx`.

- [ ] **Step 1: Write the failing tests**

Create `services/core/tests/productos.rs`:
```rust
//! Catálogo de productos/servicios - sugerencia de código (ver
//! docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md).

mod common;
use common::*;

#[tokio::test]
async fn sugerir_codigo_deriva_prefijo_de_las_primeras_3_letras_sin_acentos() {
    let session = register_tenant().await;
    let sugerido = session.get("/v1/productos/sugerir-codigo?nombre=Fumigaci%C3%B3n").await;
    assert_eq!(sugerido["sku"], "FUM-0001");
}

#[tokio::test]
async fn sugerir_codigo_avanza_la_secuencia_por_prefijo() {
    let session = register_tenant().await;
    session
        .post(
            "/v1/productos",
            serde_json::json!({ "sku": "FUM-0001", "nombre": "Fumigación básica", "itbis_tipo": "GRAVADO_18", "tipo": "SERVICIO" }),
        )
        .await;

    let sugerido = session.get("/v1/productos/sugerir-codigo?nombre=Fumigaci%C3%B3n%20Residencial").await;
    assert_eq!(sugerido["sku"], "FUM-0002", "debe continuar la secuencia existente para el mismo prefijo");
}

#[tokio::test]
async fn sugerir_codigo_rechaza_un_nombre_sin_suficientes_letras() {
    let session = register_tenant().await;
    let (status, body) = session.get_expect("/v1/productos/sugerir-codigo?nombre=A1").await;
    assert_eq!(status, 400, "{body}");
}
```

`TenantSession` doesn't have a `get_expect` helper yet (only `get`, which panics on non-2xx) — add one to `services/core/tests/common/mod.rs`, modeled directly on the existing `post_expect`:
```rust
pub async fn get_expect(&self, path: &str) -> (reqwest::StatusCode, Value) {
    let resp = self.client.get(format!("{}{}", base_url(), path)).bearer_auth(&self.token).send().await.expect("request failed");
    let status = resp.status();
    let text = resp.text().await.expect("response body unreadable");
    let payload: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, payload)
}
```

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test --test productos
```
Expected: compile error (`get_expect` doesn't exist until you add it in Step 1's second half) then, once that's added, 404s — the route doesn't exist yet.

- [ ] **Step 3: Implement `sugerir_codigo` in `catalog_service.rs`**

Add near the bottom of the `// ---- Productos ----` section (after `create_producto`, before `update_producto`):
```rust
    /// Deriva un código a partir del nombre: primeras 3 letras (sin acentos,
    /// sin espacios/puntuación), en mayúsculas, seguidas de la siguiente
    /// secuencia de 4 dígitos para ese prefijo en este tenant. No hay tabla
    /// de contadores - la secuencia se calcula de los SKUs existentes, así
    /// que se autocorrige si se borra un producto. Ver
    /// docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md.
    pub async fn sugerir_codigo(&self, tenant_id: &str, nombre: &str) -> anyhow::Result<String> {
        let prefijo = Self::prefijo_desde_nombre(nombre)?;
        let like_pattern = format!("{}-%", prefijo);
        let skus: Vec<(String,)> = sqlx::query_as("SELECT sku FROM productos WHERE tenant_id = $1 AND sku LIKE $2")
            .bind(tenant_id)
            .bind(&like_pattern)
            .fetch_all(&self.pool)
            .await?;
        let siguiente = skus
            .iter()
            .filter_map(|(sku,)| sku.rsplit('-').next())
            .filter_map(|n| n.parse::<u32>().ok())
            .max()
            .unwrap_or(0)
            + 1;
        Ok(format!("{}-{:04}", prefijo, siguiente))
    }

    fn prefijo_desde_nombre(nombre: &str) -> anyhow::Result<String> {
        fn quitar_acentos(c: char) -> char {
            match c {
                'á' | 'Á' => 'a',
                'é' | 'É' => 'e',
                'í' | 'Í' => 'i',
                'ó' | 'Ó' => 'o',
                'ú' | 'Ú' | 'ü' | 'Ü' => 'u',
                'ñ' | 'Ñ' => 'n',
                other => other,
            }
        }
        let letras: String = nombre.chars().filter(|c| c.is_alphabetic()).map(quitar_acentos).collect();
        let prefijo: String = letras.chars().take(3).collect::<String>().to_uppercase();
        if prefijo.chars().count() < 3 {
            anyhow::bail!("El nombre necesita al menos 3 letras para generar un código");
        }
        Ok(prefijo)
    }
```

- [ ] **Step 4: Register the route and handler in `main.rs`**

Add the route in the same `.route(...)` chain as the other `/v1/productos` routes (`main.rs:558-560`):
```rust
        .route("/v1/productos/sugerir-codigo", get(http_sugerir_codigo_producto))
```
Add the handler next to `http_list_productos` (`main.rs:1850`, just above `struct ListProductosParams`):
```rust
#[derive(Debug, Deserialize)]
struct SugerirCodigoParams {
    nombre: String,
}

async fn http_sugerir_codigo_producto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<SugerirCodigoParams>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let sku = state.catalog_service.sugerir_codigo(&claims.tenant_id, &params.nombre).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(serde_json::json!({ "sku": sku })))
}
```

- [ ] **Step 5: Run the tests**

```bash
cargo test --test productos
```
Expected: all 3 pass.

- [ ] **Step 6: Run the full suite**

```bash
cargo test --tests
```
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add services/core/src/services/catalog_service.rs services/core/src/main.rs services/core/tests/productos.rs services/core/tests/common/mod.rs
git commit -m "feat(productos): auto-suggest product/service code from name"
```

---

## Task 7: Frontend — auto-suggested code on the product/service form

**Files:**
- Modify: `apps/web/app/(customer)/(dashboard)/inventario/productos/producto-form.tsx`
- Modify: `apps/web/lib/api.ts` (add a thin fetch helper — check first whether one already exists before adding)
- Create: `apps/web/app/api/productos/sugerir-codigo/route.ts`

**Interfaces:**
- Consumes: `GET /v1/productos/sugerir-codigo?nombre=...` (Task 6).

- [ ] **Step 1: Add the Next.js proxy route**

Create `apps/web/app/api/productos/sugerir-codigo/route.ts`, matching the exact shape of `apps/web/app/api/productos/route.ts`'s `GET`:
```typescript
import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function GET(req: NextRequest) {
  const qs = req.nextUrl.search;
  const res = await fetch(`${CORE_HTTP}/v1/productos/sugerir-codigo${qs}`, {
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}
```

- [ ] **Step 2: Wire the suggestion into `producto-form.tsx`**

In `apps/web/app/(customer)/(dashboard)/inventario/productos/producto-form.tsx`, add a debounced effect that fetches the suggestion whenever `nombre` changes and the user hasn't already typed a SKU by hand for this session. Add near the other `useState` calls (after `const [imagenError, setImagenError] = useState("");`):
```typescript
  const [skuTocado, setSkuTocado] = useState(!!initial?.sku);
```
Add a new `useEffect` (after the existing categorias/proveedores-loading effect):
```typescript
  useEffect(() => {
    if (skuTocado || !values.nombre.trim()) return;
    const handle = setTimeout(() => {
      apiFetch<{ sku: string }>(`/api/productos/sugerir-codigo?nombre=${encodeURIComponent(values.nombre.trim())}`)
        .then((d) => set("sku", d.sku))
        .catch(() => {});
    }, 400);
    return () => clearTimeout(handle);
  }, [values.nombre, skuTocado]);
```
Change the SKU input's `onChange` from:
```tsx
<Input id="sku" required value={values.sku} onChange={(e) => set("sku", e.target.value)} placeholder="ARR-001" />
```
to:
```tsx
<Input
  id="sku"
  required
  value={values.sku}
  onChange={(e) => {
    setSkuTocado(true);
    set("sku", e.target.value);
  }}
  placeholder="Se sugiere automáticamente al escribir el nombre"
/>
```

- [ ] **Step 3: Resolve the spec's open rate-limiting question**

The design spec flagged whether `sugerir-codigo` needs tenant-scoped rate limiting since it's called on every keystroke. The 400ms debounce added in Step 2 means at most ~2-3 requests/second per active user typing a name, well within what a single Postgres query per tenant can absorb — no rate limiting needed. Record this resolution nowhere else; this step just confirms the debounce is the answer, so no additional code follows from it.

- [ ] **Step 4: Manual verification**

Run the dev server:
```bash
pnpm --filter web dev
```
Open `/inventario/productos/nuevo`, type "Fumigación" in Nombre. Expected: after ~400ms, SKU auto-fills with `FUM-0001` (or the next available sequence if a `FUM-*` product already exists in that dev tenant). Manually edit the SKU field — expected: the auto-fill stops updating from further name changes (since `skuTocado` is now `true`). Create the product — expected: it saves successfully, and the new code appears in `/inventario/productos`.

- [ ] **Step 5: Typecheck**

```bash
pnpm --filter web run type-check
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add apps/web/app/api/productos/sugerir-codigo/route.ts apps/web/app/\(customer\)/\(dashboard\)/inventario/productos/producto-form.tsx
git commit -m "feat(productos): auto-suggest code from name on the product/service form"
```

---

## Task 8: Shared search picker + cotizaciones/nueva rewiring

**Files:**
- Create: `apps/web/app/(customer)/(dashboard)/producto-picker.tsx`
- Modify: `apps/web/app/(customer)/(dashboard)/cotizaciones/nueva/page.tsx`

**Interfaces:**
- Produces: `ProductoPicker({ productos, value, onChange }: { productos: Producto[]; value: string; onChange: (id: string) => void })` — a search input + filtered list, modeled on `pos/page.tsx`'s existing client-side filter (`pos/page.tsx:129-133`). Consumed by this task and Tasks 9-11.

- [ ] **Step 1: Create the shared picker component**

Create `apps/web/app/(customer)/(dashboard)/producto-picker.tsx`:
```tsx
"use client";

import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { Input } from "@repo/ui";

export interface ProductoPickerItem {
  id: string;
  sku: string;
  nombre: string;
  tipo: "PRODUCTO" | "SERVICIO";
  precio_venta: string | null;
}

/**
 * Buscador de producto/servicio por nombre o código, compartido entre
 * cotizaciones y órdenes de servicio (ver
 * docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md).
 * Filtro client-side sobre la lista ya cargada, igual que pos/page.tsx -
 * un catálogo de miles de SKUs no justifica un roundtrip por cada tecla.
 */
export function ProductoPicker({
  productos,
  value,
  onChange,
  placeholder = "Buscar por nombre o código…",
}: {
  productos: ProductoPickerItem[];
  value: string;
  onChange: (id: string) => void;
  placeholder?: string;
}) {
  const [query, setQuery] = useState("");
  const [abierto, setAbierto] = useState(false);

  const seleccionado = productos.find((p) => p.id === value);

  const filtrados = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return productos.slice(0, 20);
    return productos.filter((p) => p.nombre.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q)).slice(0, 20);
  }, [query, productos]);

  return (
    <div className="relative">
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
        <Input
          className="pl-8"
          value={abierto ? query : seleccionado ? `${seleccionado.sku} · ${seleccionado.nombre}` : ""}
          placeholder={placeholder}
          onFocus={() => {
            setAbierto(true);
            setQuery("");
          }}
          onChange={(e) => setQuery(e.target.value)}
          onBlur={() => setTimeout(() => setAbierto(false), 150)}
        />
      </div>
      {abierto && (
        <div className="absolute z-10 mt-1 w-full max-h-64 overflow-y-auto rounded-md border border-border bg-surface shadow-md">
          {filtrados.length === 0 && <p className="p-2.5 text-xs text-muted-foreground">Sin resultados</p>}
          {filtrados.map((p) => (
            <button
              key={p.id}
              type="button"
              className="flex w-full items-center justify-between gap-2 px-2.5 py-2 text-left text-sm hover:bg-muted transition-colors"
              onMouseDown={(e) => {
                e.preventDefault();
                onChange(p.id);
                setAbierto(false);
              }}
            >
              <span>
                <span className="font-mono text-xs text-muted-foreground mr-2">{p.sku}</span>
                {p.nombre}
              </span>
              <span className="text-xs text-muted-foreground shrink-0">
                {p.tipo === "SERVICIO" ? "Servicio" : `RD$ ${Number(p.precio_venta || 0).toLocaleString("es-DO", { minimumFractionDigits: 2 })}`}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Rewire `cotizaciones/nueva/page.tsx`**

In `apps/web/app/(customer)/(dashboard)/cotizaciones/nueva/page.tsx`:

Add the import:
```typescript
import { ProductoPicker } from "../../producto-picker";
```

Add `descripcion` to the `Linea` interface:
```typescript
interface Linea {
  productoId: string;
  cantidad: string;
  descuento: string;
  precioUnitario: string;
  descripcion: string;
}
```
Update the two places that construct an empty `Linea` (in `useState` and in `addLinea`) to include `descripcion: ""`.

Replace `precioLinea` (it always uses the manual price now — Task 2 made every line manual):
```typescript
function precioLinea(l: Linea): number {
  return Number(l.precioUnitario) || 0;
}
```
Update its one call site (inside the `total` reducer) accordingly: `precioLinea(l, prod)` → `precioLinea(l)`.

In `handleSubmit`, remove the `servicioSinPrecio` check block entirely (every line already requires a price via the always-visible input added below) and change the item-mapping:
```typescript
items: items.map((l) => ({
  producto_id: l.productoId,
  cantidad: l.cantidad,
  descuento: l.descuento || undefined,
  precio_unitario: l.precioUnitario,
  descripcion: l.descripcion || undefined,
})),
```

Replace the line-rendering block:
```tsx
{lineas.map((l, i) => {
  const prod = productos.find((p) => p.id === l.productoId);
  const esServicio = prod?.tipo === "SERVICIO";
  return (
    <div key={i} className={`grid gap-2 items-end ${esServicio ? "grid-cols-[1fr_90px_110px_110px_32px]" : "grid-cols-[1fr_90px_110px_32px]"}`}>
      <Select value={l.productoId} onChange={(e) => updateLinea(i, { productoId: e.target.value })}>
        <option value="">Producto…</option>
        {productos.map((p) => (
          <option key={p.id} value={p.id}>
            {p.sku} · {p.nombre} {p.tipo === "SERVICIO" ? "(servicio)" : `(${formatDOP(p.precio_venta || "0")})`}
          </option>
        ))}
      </Select>
      <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
      {esServicio && (
        <Input type="number" step="0.01" placeholder="Precio c/u" value={l.precioUnitario} onChange={(e) => updateLinea(i, { precioUnitario: e.target.value })} />
      )}
      <Input type="number" step="0.01" placeholder="Descuento RD$" value={l.descuento} onChange={(e) => updateLinea(i, { descuento: e.target.value })} />
      <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
        <Trash2 className="h-4 w-4" />
      </Button>
    </div>
  );
})}
```
with:
```tsx
{lineas.map((l, i) => {
  const prod = productos.find((p) => p.id === l.productoId);
  return (
    <div key={i} className="grid gap-2 items-end grid-cols-[1fr_90px_110px_110px_1fr_32px]">
      <ProductoPicker
        productos={productos}
        value={l.productoId}
        onChange={(id) => {
          const nuevo = productos.find((p) => p.id === id);
          updateLinea(i, { productoId: id, precioUnitario: nuevo?.tipo === "PRODUCTO" ? (nuevo.precio_venta || "") : "" });
        }}
      />
      <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
      <Input type="number" step="0.01" placeholder="Precio c/u" value={l.precioUnitario} onChange={(e) => updateLinea(i, { precioUnitario: e.target.value })} />
      <Input type="number" step="0.01" placeholder="Descuento RD$" value={l.descuento} onChange={(e) => updateLinea(i, { descuento: e.target.value })} />
      <Input placeholder="Descripción (opcional)" value={l.descripcion} onChange={(e) => updateLinea(i, { descripcion: e.target.value })} />
      <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
        <Trash2 className="h-4 w-4" />
      </Button>
    </div>
  );
})}
```
(`prod` is kept as a local for a future line-level display need, but is otherwise unused now that the price pre-fill happens in `onChange` — if the linter flags it as unused, remove the `const prod = ...` line; it isn't required by any other part of this block.)

- [ ] **Step 3: Manual verification**

```bash
pnpm --filter web dev
```
Open `/cotizaciones/nueva`. Expected: the product/service field is now a search box — typing a partial name or code filters the dropdown. Selecting a `PRODUCTO` pre-fills its price (editable); selecting a `SERVICIO` leaves price blank (required). Enter a description on one line. Submit — expected: cotización is created, and its detail page shows the correct total (Task 9 will show the description there).

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter web run type-check
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add apps/web/app/\(customer\)/\(dashboard\)/producto-picker.tsx apps/web/app/\(customer\)/\(dashboard\)/cotizaciones/nueva/page.tsx
git commit -m "feat(cotizaciones): search picker, always-manual price, line descripción"
```

---

## Task 9: Cotización detail — ResumenTab add-item form rewiring

**Files:**
- Modify: `apps/web/app/(customer)/(dashboard)/cotizaciones/[id]/page.tsx`

**Interfaces:**
- Consumes: `ProductoPicker` (Task 8).

- [ ] **Step 1: Add `descripcion` to `CotizacionItem` and display it**

Add to the `CotizacionItem` interface (`page.tsx:11-20`):
```typescript
  descripcion: string | null;
```
In `ResumenTab`'s table, add a column showing it — change the header row:
```tsx
<TableHead>SKU</TableHead>
<TableHead>Producto</TableHead>
```
to add one more header after "Producto":
```tsx
<TableHead>SKU</TableHead>
<TableHead>Producto</TableHead>
<TableHead>Descripción</TableHead>
```
and in the row mapping, add a matching cell right after the `nombre` cell:
```tsx
<TableCell className="font-medium">{it.nombre}</TableCell>
<TableCell className="text-muted-foreground text-xs">{it.descripcion || "—"}</TableCell>
```

- [ ] **Step 2: Rewire the add-item mini-form**

Add state for descripcion (next to the existing `useState` calls in `ResumenTab`):
```typescript
  const [descripcionLinea, setDescripcionLinea] = useState("");
```
Replace `handleAdd`'s body:
```typescript
  async function handleAdd() {
    if (!productoId || !cantidad) return;
    if (esServicio && !(Number(precioUnitario) > 0)) {
      setError("Escribe el precio de este servicio.");
      return;
    }
    setAgregando(true);
    setError("");
    try {
      await apiFetch(`/api/cotizaciones/${cotizacion.id}/items`, {
        method: "POST",
        body: JSON.stringify({
          producto_id: productoId,
          cantidad,
          descuento: descuento || undefined,
          precio_unitario: esServicio ? precioUnitario : undefined,
        }),
      });
      setProductoId("");
      setCantidad("");
      setDescuento("");
      setPrecioUnitario("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setAgregando(false);
    }
  }
```
with:
```typescript
  async function handleAdd() {
    if (!productoId || !cantidad) return;
    if (!(Number(precioUnitario) > 0)) {
      setError("Escribe el precio de esta línea.");
      return;
    }
    setAgregando(true);
    setError("");
    try {
      await apiFetch(`/api/cotizaciones/${cotizacion.id}/items`, {
        method: "POST",
        body: JSON.stringify({
          producto_id: productoId,
          cantidad,
          descuento: descuento || undefined,
          precio_unitario: precioUnitario,
          descripcion: descripcionLinea || undefined,
        }),
      });
      setProductoId("");
      setCantidad("");
      setDescuento("");
      setPrecioUnitario("");
      setDescripcionLinea("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setAgregando(false);
    }
  }
```
Replace the add-line JSX:
```tsx
{puedeEditar && (
  <div className={`grid gap-2 items-end ${esServicio ? "grid-cols-[1fr_90px_110px_110px_auto]" : "grid-cols-[1fr_90px_110px_auto]"}`}>
    <Select value={productoId} onChange={(e) => setProductoId(e.target.value)}>
      <option value="">Producto…</option>
      {productos.map((p) => (
        <option key={p.id} value={p.id}>
          {p.sku} · {p.nombre} {p.tipo === "SERVICIO" ? "(servicio)" : `(${formatDOP(p.precio_venta || "0")})`}
        </option>
      ))}
    </Select>
    <Input type="number" step="0.01" placeholder="Cant." value={cantidad} onChange={(e) => setCantidad(e.target.value)} />
    {esServicio && (
      <Input type="number" step="0.01" placeholder="Precio c/u" value={precioUnitario} onChange={(e) => setPrecioUnitario(e.target.value)} />
    )}
    <Input type="number" step="0.01" placeholder="Descuento RD$" value={descuento} onChange={(e) => setDescuento(e.target.value)} />
    <Button type="button" size="sm" disabled={agregando || !productoId || !cantidad} onClick={handleAdd}>
      <Plus className="h-4 w-4" />{agregando ? "..." : "Agregar"}
    </Button>
  </div>
)}
```
with:
```tsx
{puedeEditar && (
  <div className="grid gap-2 items-end grid-cols-[1fr_90px_110px_110px_1fr_auto]">
    <ProductoPicker
      productos={productos}
      value={productoId}
      onChange={(id) => {
        setProductoId(id);
        const nuevo = productos.find((p) => p.id === id);
        setPrecioUnitario(nuevo?.tipo === "PRODUCTO" ? nuevo.precio_venta || "" : "");
      }}
    />
    <Input type="number" step="0.01" placeholder="Cant." value={cantidad} onChange={(e) => setCantidad(e.target.value)} />
    <Input type="number" step="0.01" placeholder="Precio c/u" value={precioUnitario} onChange={(e) => setPrecioUnitario(e.target.value)} />
    <Input type="number" step="0.01" placeholder="Descuento RD$" value={descuento} onChange={(e) => setDescuento(e.target.value)} />
    <Input placeholder="Descripción (opcional)" value={descripcionLinea} onChange={(e) => setDescripcionLinea(e.target.value)} />
    <Button type="button" size="sm" disabled={agregando || !productoId || !cantidad} onClick={handleAdd}>
      <Plus className="h-4 w-4" />{agregando ? "..." : "Agregar"}
    </Button>
  </div>
)}
```
Add the import at the top of the file:
```typescript
import { ProductoPicker } from "../../producto-picker";
```
The `esServicio`/`prodSel` locals become unused by this rewrite — remove `const prodSel = productos.find((p) => p.id === productoId);` and `const esServicio = prodSel?.tipo === "SERVICIO";` from `ResumenTab`.

- [ ] **Step 3: Manual verification**

Open an existing (non-CONVERTIDA/RECHAZADA) cotización's detail page. Expected: adding a `PRODUCTO` line pre-fills its price (editable, required); adding a `SERVICIO` line requires typing one; the new "Descripción" column shows what was typed.

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter web run type-check
```
Expected: no errors, no unused-variable warnings for `prodSel`/`esServicio`.

- [ ] **Step 5: Commit**

```bash
git add "apps/web/app/(customer)/(dashboard)/cotizaciones/[id]/page.tsx"
git commit -m "feat(cotizaciones): search picker + descripción on the detail page's add-item form"
```

---

## Task 10: Orden de servicio — creation form drops pricing

**Files:**
- Modify: `apps/web/app/(customer)/(dashboard)/ordenes-servicio/nueva/page.tsx`

**Interfaces:**
- Consumes: `ProductoPicker` (Task 8).

- [ ] **Step 1: Drop pricing from the `Linea` shape and totals**

Change the `Linea` interface:
```typescript
interface Linea {
  productoId: string;
  cantidad: string;
  descripcion: string;
}
```
Remove the `ITBIS_RATE` constant, the `precioLinea` function, and the `total` reducer entirely (an orden never shows a total now).

Update the two places constructing an empty `Linea` (`useState` initial value and `addLinea`) to `{ productoId: "", cantidad: "", descripcion: "" }`.

- [ ] **Step 2: Simplify `handleSubmit`**

Replace the `servicioSinPrecio` validation block and the item-mapping:
```typescript
    const servicioSinPrecio = items.some((l) => {
      const prod = productos.find((p) => p.id === l.productoId);
      return prod?.tipo === "SERVICIO" && !(Number(l.precioUnitario) > 0);
    });
    if (servicioSinPrecio) {
      setError("Escribe el precio de cada servicio agregado.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      const orden = await apiFetch<{ id: string }>("/api/ordenes-servicio", {
        method: "POST",
        body: JSON.stringify({
          cliente_id: clienteId || undefined,
          condicion_id: condicionId || undefined,
          prioridad,
          fecha_programada: fechaProgramada || undefined,
          direccion: direccion || undefined,
          descripcion: descripcion || undefined,
          items: items.map((l) => {
            const prod = productos.find((p) => p.id === l.productoId);
            return {
              producto_id: l.productoId,
              cantidad: l.cantidad,
              descuento: l.descuento || undefined,
              precio_unitario: prod?.tipo === "SERVICIO" ? l.precioUnitario : undefined,
            };
          }),
        }),
      });
```
with:
```typescript
    setSaving(true);
    setError("");
    try {
      const orden = await apiFetch<{ id: string }>("/api/ordenes-servicio", {
        method: "POST",
        body: JSON.stringify({
          cliente_id: clienteId || undefined,
          condicion_id: condicionId || undefined,
          prioridad,
          fecha_programada: fechaProgramada || undefined,
          direccion: direccion || undefined,
          descripcion: descripcion || undefined,
          items: items.map((l) => ({
            producto_id: l.productoId,
            cantidad: l.cantidad,
            observaciones: l.descripcion || undefined,
          })),
        }),
      });
```

- [ ] **Step 3: Rewire the line UI and drop the total display**

Replace the line-rendering block:
```tsx
{lineas.map((l, i) => {
  const prod = productos.find((p) => p.id === l.productoId);
  const esServicio = prod?.tipo === "SERVICIO";
  return (
    <div key={i} className={`grid gap-2 items-end ${esServicio ? "grid-cols-[1fr_90px_110px_110px_32px]" : "grid-cols-[1fr_90px_110px_32px]"}`}>
      <Select value={l.productoId} onChange={(e) => updateLinea(i, { productoId: e.target.value })}>
        <option value="">Producto o servicio…</option>
        {productos.map((p) => (
          <option key={p.id} value={p.id}>
            {p.sku} · {p.nombre} {p.tipo === "SERVICIO" ? "(servicio)" : `(${formatDOP(p.precio_venta || "0")})`}
          </option>
        ))}
      </Select>
      <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
      {esServicio && (
        <Input type="number" step="0.01" placeholder="Precio c/u" value={l.precioUnitario} onChange={(e) => updateLinea(i, { precioUnitario: e.target.value })} />
      )}
      <Input type="number" step="0.01" placeholder="Descuento RD$" value={l.descuento} onChange={(e) => updateLinea(i, { descuento: e.target.value })} />
      <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
        <Trash2 className="h-4 w-4" />
      </Button>
    </div>
  );
})}
```
with:
```tsx
{lineas.map((l, i) => (
  <div key={i} className="grid gap-2 items-end grid-cols-[1fr_90px_1fr_32px]">
    <ProductoPicker productos={productos} value={l.productoId} onChange={(id) => updateLinea(i, { productoId: id })} placeholder="Servicio…" />
    <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
    <Input placeholder="Descripción (opcional)" value={l.descripcion} onChange={(e) => updateLinea(i, { descripcion: e.target.value })} />
    <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
      <Trash2 className="h-4 w-4" />
    </Button>
  </div>
))}
```
Remove the totals block entirely:
```tsx
<div className="flex items-center justify-end">
  <p className="text-lg font-bold">Total: {formatDOP(total)}</p>
</div>
```
Add the import:
```typescript
import { ProductoPicker } from "../../producto-picker";
```
`formatDOP` becomes unused in this file's import list — remove it from the `@repo/ui` import if nothing else in the file uses it (check with `grep -n formatDOP` in this file before removing).

- [ ] **Step 4: Manual verification**

Open `/ordenes-servicio/nueva`. Expected: no price fields or total anywhere on the form; searching and picking a service works; submitting creates the order.

- [ ] **Step 5: Typecheck**

```bash
pnpm --filter web run type-check
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add "apps/web/app/(customer)/(dashboard)/ordenes-servicio/nueva/page.tsx"
git commit -m "feat(ordenes-servicio): drop all pricing from the creation form"
```

---

## Task 11: Orden de servicio detail — ResumenTab + ItemsTab drop pricing

**Files:**
- Modify: `apps/web/app/(customer)/(dashboard)/ordenes-servicio/[id]/page.tsx`

**Interfaces:**
- Consumes: `ProductoPicker` (Task 8).

- [ ] **Step 1: Update the `OrdenItem` interface**

Change:
```typescript
interface OrdenItem {
  id: string;
  producto_id: string;
  sku: string;
  nombre: string;
  tipo: "PRODUCTO" | "SERVICIO";
  cantidad: string;
  precio_unitario: string;
  descuento: string;
  itbis_monto: string;
  subtotal: string;
}
```
to:
```typescript
interface OrdenItem {
  id: string;
  producto_id: string;
  sku: string;
  nombre: string;
  tipo: "PRODUCTO" | "SERVICIO";
  cantidad: string;
  observaciones: string | null;
}
```

- [ ] **Step 2: Simplify `TotalsBox`/`ResumenTab`**

Delete the `TotalsBox` function entirely (`page.tsx:235-244`) — nothing shows a total anymore on this page's Resumen/Items tabs.

Replace `ResumenTab`:
```tsx
function ResumenTab({ orden }: { orden: OrdenDetalle }) {
  return (
    <div className="space-y-4">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Servicio</TableHead>
            <TableHead className="text-right">Cant.</TableHead>
            <TableHead>Descripción</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {orden.items.map((it) => (
            <TableRow key={it.id}>
              <TableCell>
                <span className="font-medium">{it.nombre}</span>
                {it.tipo === "SERVICIO" && <Badge variant="accent" className="ml-2">Servicio</Badge>}
              </TableCell>
              <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
              <TableCell className="text-muted-foreground text-xs">{it.observaciones || "—"}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      {orden.descripcion && (
        <div className="text-sm text-muted-foreground bg-muted/40 rounded-md p-3">{orden.descripcion}</div>
      )}
    </div>
  );
}
```

- [ ] **Step 3: Rewire `ItemsTab`**

Replace the whole `ItemsTab` function:
```tsx
function ItemsTab({ orden, productos, onChanged }: { orden: OrdenDetalle; productos: Producto[]; onChanged: () => void }) {
  const [productoId, setProductoId] = useState("");
  const [cantidad, setCantidad] = useState("");
  const [observaciones, setObservaciones] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const soloLectura = ["COMPLETADA", "CANCELADA"].includes(orden.estado);

  async function handleAdd() {
    if (!productoId || !cantidad) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/items`, {
        method: "POST",
        body: JSON.stringify({ producto_id: productoId, cantidad, observaciones: observaciones || undefined }),
      });
      setProductoId(""); setCantidad(""); setObservaciones("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleRemove(itemId: string) {
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/items/${itemId}`, { method: "DELETE" });
      onChanged();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="space-y-4">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Servicio</TableHead>
            <TableHead className="text-right">Cant.</TableHead>
            <TableHead>Descripción</TableHead>
            {!soloLectura && <TableHead className="w-10"></TableHead>}
          </TableRow>
        </TableHeader>
        <TableBody>
          {orden.items.map((it) => (
            <TableRow key={it.id}>
              <TableCell className="font-medium">{it.nombre}</TableCell>
              <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
              <TableCell className="text-muted-foreground text-xs">{it.observaciones || "—"}</TableCell>
              {!soloLectura && (
                <TableCell className="text-right">
                  <Button size="icon" variant="ghost" onClick={() => handleRemove(it.id)}><Trash2 className="h-3.5 w-3.5" /></Button>
                </TableCell>
              )}
            </TableRow>
          ))}
        </TableBody>
      </Table>

      {!soloLectura && (
        <div className="grid gap-2 items-end grid-cols-[1fr_90px_1fr_auto]">
          <ProductoPicker productos={productos} value={productoId} onChange={setProductoId} placeholder="Servicio…" />
          <Input type="number" step="0.01" placeholder="Cant." value={cantidad} onChange={(e) => setCantidad(e.target.value)} />
          <Input placeholder="Descripción (opcional)" value={observaciones} onChange={(e) => setObservaciones(e.target.value)} />
          <Button type="button" size="sm" disabled={saving || !productoId || !cantidad} onClick={handleAdd}><Plus className="h-4 w-4" />Agregar</Button>
        </div>
      )}
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
    </div>
  );
}
```
Add the import:
```typescript
import { ProductoPicker } from "../../producto-picker";
```

- [ ] **Step 4: Manual verification**

Open an existing orden's detail page. Expected: Resumen and Items tabs show no price/ITBIS/total columns anywhere; adding an item via Items tab works and shows the description.

- [ ] **Step 5: Typecheck**

```bash
pnpm --filter web run type-check
```
Expected: no errors. `formatDOP` may become unused in this file if `FacturacionTab` (Task 12) is the only remaining consumer — leave the import as-is here since Task 12 still needs it.

- [ ] **Step 6: Commit**

```bash
git add "apps/web/app/(customer)/(dashboard)/ordenes-servicio/[id]/page.tsx"
git commit -m "feat(ordenes-servicio): drop pricing from Resumen/Items tabs"
```

---

## Task 12: Orden de servicio detail — Facturación tab redesign

**Files:**
- Modify: `apps/web/app/(customer)/(dashboard)/ordenes-servicio/[id]/page.tsx`

**Interfaces:**
- Produces: `POST /api/ordenes-servicio/:id/crear-factura` body now includes `items: [{ producto_id, precio_unitario? }]` — matches Task 4's `FacturarOrdenRequest`.

- [ ] **Step 1: Rewrite `FacturacionTab`**

Replace the whole `FacturacionTab` function:
```tsx
function FacturacionTab({ orden, productos, onChanged }: { orden: OrdenDetalle; productos: Producto[]; onChanged: () => void }) {
  const [metodoPago, setMetodoPago] = useState("EFECTIVO");
  const [precios, setPrecios] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  async function handleFacturar() {
    const faltante = orden.items.find((it) => {
      const prod = productos.find((p) => p.id === it.producto_id);
      return prod?.tipo === "SERVICIO" && !(Number(precios[it.id]) > 0);
    });
    if (faltante) {
      setError(`Escribe el precio de "${faltante.nombre}".`);
      return;
    }
    setSaving(true);
    setError("");
    try {
      const venta = await apiFetch<{ id: string }>(`/api/ordenes-servicio/${orden.id}/crear-factura`, {
        method: "POST",
        body: JSON.stringify({
          metodo_pago: metodoPago,
          items: orden.items.map((it) => ({ producto_id: it.producto_id, precio_unitario: precios[it.id] || undefined })),
        }),
      });
      onChanged();
      window.location.href = `/ventas/${venta.id}`;
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  if (orden.venta_id) {
    return (
      <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">
        Facturada — <Link href={`/ventas/${orden.venta_id}` as any} className="underline font-medium">ver venta</Link>.
      </div>
    );
  }

  if (orden.estado !== "COMPLETADA") {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <p className="text-sm">Esta orden todavía no se ha facturado.</p>
        <p className="text-xs mt-1">Completa el trabajo para habilitar la facturación.</p>
      </div>
    );
  }

  return (
    <div className="max-w-md space-y-3">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Servicio</TableHead>
            <TableHead className="text-right">Cant.</TableHead>
            <TableHead className="text-right">Precio c/u</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {orden.items.map((it) => {
            const prod = productos.find((p) => p.id === it.producto_id);
            const esServicio = prod?.tipo === "SERVICIO";
            return (
              <TableRow key={it.id}>
                <TableCell className="font-medium">{it.nombre}</TableCell>
                <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
                <TableCell className="text-right">
                  {esServicio ? (
                    <Input
                      type="number"
                      step="0.01"
                      className="text-right"
                      value={precios[it.id] || ""}
                      onChange={(e) => setPrecios((p) => ({ ...p, [it.id]: e.target.value }))}
                    />
                  ) : (
                    <span className="tabular-nums text-muted-foreground">{formatDOP(prod?.precio_venta || "0")}</span>
                  )}
                </TableCell>
              </TableRow>
            );
          })}
        </TableBody>
      </Table>
      <div className="space-y-1.5">
        <Label htmlFor="metodo">Método de pago</Label>
        <Select id="metodo" value={metodoPago} onChange={(e) => setMetodoPago(e.target.value)}>
          <option value="EFECTIVO">Efectivo</option>
          <option value="TARJETA">Tarjeta</option>
          <option value="TRANSFERENCIA">Transferencia</option>
          <option value="FIADO">Fiado</option>
        </Select>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
      <Button className="w-full" disabled={saving} onClick={handleFacturar}>{saving ? "Procesando..." : "Crear factura"}</Button>
    </div>
  );
}
```

- [ ] **Step 2: Pass `productos` into `FacturacionTab`**

Find where the page renders `<FacturacionTab orden={orden} onChanged={load} />` (near `{tab === "facturacion" && ...}`) and change it to:
```tsx
{tab === "facturacion" && <FacturacionTab orden={orden} productos={productos} onChanged={load} />}
```

- [ ] **Step 3: Manual verification**

Create an orden with one `SERVICIO` line, `iniciar` → `completar` it, open the Facturación tab. Expected: a price input appears for the service (required — attempting "Crear factura" without it shows the inline error); entering a price and submitting creates the venta and redirects to it with the correct total.

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter web run type-check
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add "apps/web/app/(customer)/(dashboard)/ordenes-servicio/[id]/page.tsx"
git commit -m "feat(ordenes-servicio): capture price per item at the facturación step"
```

---

## Task 13: Printed orden de servicio drops items/price

**Files:**
- Modify: `apps/web/app/(customer)/imprimir/orden-servicio/[id]/page.tsx`

- [ ] **Step 1: Update the `OrdenDetalle`/`OrdenItem` interfaces**

Change:
```typescript
interface OrdenItem {
  id: string;
  sku: string;
  nombre: string;
  tipo: "PRODUCTO" | "SERVICIO";
  cantidad: string;
  precio_unitario: string;
  subtotal: string;
}

interface OrdenDetalle {
  id: string;
  cliente_id: string | null;
  condicion_id: string | null;
  estado: string;
  prioridad: string;
  fecha_programada: string | null;
  direccion: string | null;
  descripcion: string | null;
  subtotal: string;
  itbis_total: string;
  total: string;
  notas: string | null;
  created_at: string;
  items: OrdenItem[];
}
```
to:
```typescript
interface OrdenDetalle {
  id: string;
  cliente_id: string | null;
  condicion_id: string | null;
  estado: string;
  prioridad: string;
  fecha_programada: string | null;
  direccion: string | null;
  descripcion: string | null;
  notas: string | null;
  created_at: string;
}
```

- [ ] **Step 2: Remove the items table and totals block**

Delete this entire block:
```tsx
        <table className="w-full mt-3 text-[11.5px] border-collapse">
          <thead>
            <tr className="text-white" style={{ backgroundColor: "#8a5a1f" }}>
              <th className="text-left font-bold py-1.5 px-2 w-16">Cant.</th>
              <th className="text-left font-bold py-1.5 px-2">Descripción</th>
              <th className="text-right font-bold py-1.5 px-2 w-28">Precio</th>
              <th className="text-right font-bold py-1.5 px-2 w-28">Subtotal</th>
            </tr>
          </thead>
          <tbody>
            {orden.items.map((it) => (
              <tr key={it.id} className="border-b border-gray-200">
                <td className="py-1.5 px-2 tabular-nums">{it.cantidad}</td>
                <td className="py-1.5 px-2">{it.nombre}{it.tipo === "SERVICIO" ? " (servicio)" : ""}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">{Number(it.precio_unitario).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">{Number(it.subtotal).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <div className="flex justify-end mt-3">
          <div className="w-56 text-[11.5px] space-y-1">
            <div className="flex justify-between"><span>Subtotal:</span><span className="tabular-nums">RD$ {Number(orden.subtotal).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</span></div>
            <div className="flex justify-between"><span>ITBIS:</span><span className="tabular-nums">RD$ {Number(orden.itbis_total).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</span></div>
            <div className="flex justify-between font-bold text-[13px] pt-1 border-t border-gray-300"><span>TOTAL:</span><span className="tabular-nums">RD$ {Number(orden.total).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</span></div>
          </div>
        </div>

```
leaving the surrounding "CLIENTE" section and "DESCRIPCIÓN DEL TRABAJO" section directly adjacent to each other.

- [ ] **Step 3: Manual verification**

Open `/imprimir/orden-servicio/[id]` for an existing order. Expected: shows client info, "Descripción del trabajo," and the signature lines — no product/service list, no price, no ITBIS, no total anywhere on the page.

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter web run type-check
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add "apps/web/app/(customer)/imprimir/orden-servicio/[id]/page.tsx"
git commit -m "feat(ordenes-servicio): printed copy shows no items, products, or price"
```

---

## Task 14: Full-stack verification pass

**Files:** none (verification only).

- [ ] **Step 1: Full Rust test suite**

From `services/core`, with Postgres up and migrations applied:
```bash
cargo run --bin migrate
cargo test --tests
```
Expected: every test across `servicio_producto.rs`, `ordenes_servicio.rs`, `cotizaciones.rs`, `productos.rs`, and every other existing test file passes.

- [ ] **Step 2: Frontend typecheck, lint, build**

```bash
pnpm --filter web run type-check
pnpm --filter web run lint
pnpm --filter web run build
```
Expected: all three succeed with no errors.

- [ ] **Step 3: End-to-end manual walkthrough**

With the dev server and core service running, as one continuous flow:
1. Create a new `SERVICIO` product named "Fumigación" on `/inventario/productos/nuevo` — confirm the code auto-suggests `FUM-000N`.
2. Create a cotización on `/cotizaciones/nueva` using the search picker for that service plus one `PRODUCTO`, with a manual price on both lines and a description on one — confirm ITBIS/total match manual price × quantity math.
3. Convert that cotización to an orden de servicio — confirm the resulting order shows no price anywhere (Resumen/Items tabs).
4. Add a material to the order's Materiales tab and consume some of it — confirm `/inventario/productos` shows the reduced stock (regression check per spec Section E).
5. `iniciar` → `completar` the order, open Facturación — confirm a price input appears for the service line, submitting without it shows the inline error, and submitting with it creates a venta with the correct total.
6. Open `/imprimir/orden-servicio/[id]` for that order — confirm no items/price/products appear, only client info + descripción del trabajo + signatures.

- [ ] **Step 4: Report**

No commit for this task — it's verification-only. If any step fails, return to the relevant earlier task, fix, and re-run this task's checks before considering the plan complete.

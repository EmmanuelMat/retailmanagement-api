# Phase A Critical Fixes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the three highest-value, lowest-risk correctness/compliance gaps found by the 2026-09-18 investigation audit — the purchase-side Nota de Crédito ledger/inventory bug, the missing equity accounts in the chart of accounts, and the compliance/marketing overclaims in the README and landing page — without touching any other subsystem.

**Architecture:** Each of the three fixes is a small, targeted change to existing code paths — no new subsystems, no schema redesign beyond adding two chart-of-accounts rows. Fixes are additive/branching (`if tipo_documento == "NOTA_CREDITO"`) so the existing FACTURA/NOTA_DEBITO behavior is byte-for-byte unchanged. Tested via the repo's existing integration-test harness (`services/core/tests/`, real Postgres, one fresh tenant per test via `register_tenant()`).

**Tech Stack:** Rust (axum + sqlx + rust_decimal) for the core service; the two doc/marketing fixes are plain Markdown/TSX edits with no build step beyond `pnpm build` sanity.

**Spec:** No separate spec file — this is a bounded fix-bundle brainstormed and approved inline in conversation on 2026-09-18, grounded in `docs/14-COMPLIANCE-WORKFLOW-UIUX-AUDIT.md` (see cross-cutting findings 1, 2, 4, and the send_to_dgii correction note).

## Global Constraints

- Never bypass `contabilidad_service::create_entry` — every ledger-affecting change must still produce entries where `SUM(debe) == SUM(haber)`.
- Any change to the `cuentas_contables` seed list must be made **identically** in both `services/core/src/bin/migrate.rs` and `services/core/src/services/auth_service.rs::register_tenant` (an existing code comment in both files already mandates this pairing).
- `NOTA_DEBITO` purchases keep today's behavior exactly (ENTRADA inventory movement, same ledger direction as FACTURA) — per explicit product decision, only `NOTA_CREDITO` gets special-cased. Do not touch `NOTA_DEBITO` code paths.
- Marketing/doc copy must never claim a report or certification status that isn't actually implemented/true in the current code.
- Every new Rust test lives under `services/core/tests/` and follows the existing `mod common; use common::*;` + `register_tenant()` pattern already used throughout that directory — don't invent a new test harness.

---

### Task 1: Fix purchase-side Nota de Crédito reversal (inventory, caja, ledger)

**Files:**
- Modify: `services/core/src/services/compras_service.rs:150-329` (`ComprasService::create_compra`)
- Modify: `services/core/src/services/contabilidad_service.rs:592-613` (the COMPRAS loop inside `sincronizar`)
- Test: `services/core/tests/compras_gastos_ledger.rs`

**Interfaces:**
- Consumes: existing `CreateCompraRequest.tipo_documento: Option<String>` (already validated to be one of `FACTURA | NOTA_CREDITO | NOTA_DEBITO` earlier in `create_compra`), existing `InventarioService::insert_movimiento_tx(tx, tenant_id, usuario_id, producto_id, tipo: &str, cantidad: Decimal, costo_unitario: Option<Decimal>, motivo: Option<String>, referencia_tipo: Option<&str>, referencia_id: Option<Uuid>)`.
- Produces: no new public functions — this task only changes the internal behavior of `create_compra` and `sincronizar` for the `NOTA_CREDITO` case. No other task depends on new signatures from this one.

- [ ] **Step 1: Write the failing integration test**

Add to `services/core/tests/compras_gastos_ledger.rs` (append at the end of the file, before the final closing — it's a flat list of `#[tokio::test]` functions):

```rust
#[tokio::test]
async fn nota_credito_de_compra_reversa_stock_caja_y_ledger() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0.00)).await;

    // Starts with 20 units in stock.
    let producto = create_producto(&session, dec!(0), dec!(20)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();

    // Original FACTURA: cantidad 5, costo_unitario 40.00, GRAVADO_18
    // => subtotal 200.00, itbis 36.00, total 236.00. Stock 20 -> 25.
    session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{
                    "producto_id": producto_id,
                    "cantidad": "5",
                    "costo_unitario": "40.00",
                    "itbis_tipo": "GRAVADO_18",
                }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;

    // Return 2 of those 5 units via a Nota de Crédito.
    // subtotal 80.00, itbis 14.40, total 94.40. Stock should go 25 -> 23.
    let nota = session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{
                    "producto_id": producto_id,
                    "cantidad": "2",
                    "costo_unitario": "40.00",
                    "itbis_tipo": "GRAVADO_18",
                }],
                "metodo_pago": "EFECTIVO",
                "tipo_documento": "NOTA_CREDITO",
                "ncf_modificado": "B0100000001",
            }),
        )
        .await;
    assert_decimal_eq(decimal_field(&nota, "total"), dec!(94.40), "nota_credito.total");

    let producto_actualizado = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&producto_actualizado, "stock_actual"), dec!(23), "stock after NOTA_CREDITO must decrease, not increase");

    // Caja: the FACTURA paid out 236.00; the NOTA_CREDITO gets 94.40 back in cash.
    let resumen = caja_resumen(&session).await;
    assert_decimal_eq(decimal_field(&resumen, "egresos"), dec!(236.00), "caja egresos: only the original FACTURA");
    assert_decimal_eq(decimal_field(&resumen, "ingresos"), dec!(94.40), "caja ingresos: the NOTA_CREDITO cash refund");

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let asientos = session.get("/v1/contabilidad/asientos?referenciaTipo=COMPRA&pageSize=50").await;
    let lineas = asientos["items"].as_array().expect("asientos.items");
    assert_eq!(lineas.len(), 6, "expected 3 ledger lines each for the FACTURA and its NOTA_CREDITO reversal: {lineas:?}");

    let mut por_cuenta = std::collections::HashMap::<String, (rust_decimal::Decimal, rust_decimal::Decimal)>::new();
    for linea in lineas {
        let cuenta = linea["cuenta"].as_str().unwrap().to_string();
        let entry = por_cuenta.entry(cuenta).or_insert((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO));
        entry.0 += decimal_field(linea, "debe");
        entry.1 += decimal_field(linea, "haber");
    }
    let inventario = por_cuenta.get("1200 Inventario").expect("1200 Inventario line");
    assert_decimal_eq(inventario.0 - inventario.1, dec!(120.00), "net 1200 Inventario debe - haber (200.00 - 80.00)");
    let itbis = por_cuenta.get("1150 ITBIS Adelantado").expect("1150 ITBIS Adelantado line");
    assert_decimal_eq(itbis.0 - itbis.1, dec!(21.60), "net 1150 ITBIS Adelantado debe - haber (36.00 - 14.40)");
    let caja = por_cuenta.get("1100 Caja y Bancos").expect("1100 Caja y Bancos line");
    assert_decimal_eq(caja.1 - caja.0, dec!(141.60), "net 1100 Caja y Bancos haber - debe (236.00 out - 94.40 back)");

    assert_ledger_balanced(&session).await;
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd services/core && cargo test --test compras_gastos_ledger nota_credito_de_compra_reversa_stock_caja_y_ledger -- --nocapture`
Expected: FAIL — either the `stock_actual` assertion fails (stock will be 27, not 23, because today's code always adds), or the `egresos`/`ingresos` assertion fails (today's code always logs an EGRESO), or the net-account assertions fail (today's ledger always debits Inventario/ITBIS and credits Caja regardless of `tipo_documento`).

- [ ] **Step 3: Fix the stock/costo-promedio and caja_movimientos handling in `create_compra`**

In `services/core/src/services/compras_service.rs`, inside `create_compra`, right after the `tipo_documento` validation block (after line 183, before `let mut tx = self.pool.begin().await?;`), add:

```rust
let es_nota_credito = tipo_documento == "NOTA_CREDITO";
```

Replace the item loop's stock-update block (currently):

```rust
            // Costo promedio ponderado
            let nuevo_stock = stock_actual + item.cantidad;
            let nuevo_costo = if nuevo_stock > Decimal::ZERO {
                (costo_actual * stock_actual + item.costo_unitario * item.cantidad) / nuevo_stock
            } else {
                item.costo_unitario
            };

            sqlx::query("UPDATE productos SET stock_actual = $1, costo = $2, updated_at = NOW() WHERE id = $3")
                .bind(nuevo_stock)
                .bind(nuevo_costo)
                .bind(item.producto_id)
                .execute(&mut *tx)
                .await?;
```

with:

```rust
            if es_nota_credito {
                // Devolución al proveedor: el stock sale, no entra. El costo
                // promedio ponderado no se recalcula en una salida.
                if stock_actual < item.cantidad {
                    anyhow::bail!("Stock insuficiente para devolver {}: disponible {}, se intenta devolver {}", nombre, stock_actual, item.cantidad);
                }
                let nuevo_stock = stock_actual - item.cantidad;
                sqlx::query("UPDATE productos SET stock_actual = $1, updated_at = NOW() WHERE id = $2")
                    .bind(nuevo_stock)
                    .bind(item.producto_id)
                    .execute(&mut *tx)
                    .await?;
            } else {
                // Costo promedio ponderado
                let nuevo_stock = stock_actual + item.cantidad;
                let nuevo_costo = if nuevo_stock > Decimal::ZERO {
                    (costo_actual * stock_actual + item.costo_unitario * item.cantidad) / nuevo_stock
                } else {
                    item.costo_unitario
                };

                sqlx::query("UPDATE productos SET stock_actual = $1, costo = $2, updated_at = NOW() WHERE id = $3")
                    .bind(nuevo_stock)
                    .bind(nuevo_costo)
                    .bind(item.producto_id)
                    .execute(&mut *tx)
                    .await?;
            }
```

Replace the movement-logging block (currently):

```rust
            crate::services::inventario_service::InventarioService::insert_movimiento_tx(
                &mut tx,
                tenant_id,
                Some(usuario_id),
                producto_id,
                "ENTRADA",
                cantidad,
                Some(costo_unitario),
                Some("Compra a proveedor".to_string()),
                Some("COMPRA"),
                Some(compra.id),
            )
            .await?;
```

with:

```rust
            let (tipo_movimiento, cantidad_movimiento, costo_movimiento, motivo_movimiento) = if es_nota_credito {
                ("SALIDA", -cantidad, None, "Devolución a proveedor".to_string())
            } else {
                ("ENTRADA", cantidad, Some(costo_unitario), "Compra a proveedor".to_string())
            };

            crate::services::inventario_service::InventarioService::insert_movimiento_tx(
                &mut tx,
                tenant_id,
                Some(usuario_id),
                producto_id,
                tipo_movimiento,
                cantidad_movimiento,
                costo_movimiento,
                Some(motivo_movimiento),
                Some("COMPRA"),
                Some(compra.id),
            )
            .await?;
```

Replace the caja_movimientos block (currently):

```rust
        if metodo_pago != "FIADO" {
            sqlx::query(
                r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, metodo_pago, referencia_tipo, referencia_id, usuario_id)
                   VALUES ($1, 'EGRESO', 'Compra a proveedor', $2, $3, 'COMPRA', $4, $5)"#,
            )
            .bind(tenant_id)
            .bind(total)
            .bind(&metodo_pago)
            .bind(compra.id)
            .bind(usuario_id)
            .execute(&mut *tx)
            .await?;
        }
```

with:

```rust
        if metodo_pago != "FIADO" {
            let (tipo_caja, concepto_caja) = if es_nota_credito {
                ("INGRESO", "Devolución de compra a proveedor")
            } else {
                ("EGRESO", "Compra a proveedor")
            };
            sqlx::query(
                r#"INSERT INTO caja_movimientos (tenant_id, tipo, concepto, monto, metodo_pago, referencia_tipo, referencia_id, usuario_id)
                   VALUES ($1, $2, $3, $4, $5, 'COMPRA', $6, $7)"#,
            )
            .bind(tenant_id)
            .bind(tipo_caja)
            .bind(concepto_caja)
            .bind(total)
            .bind(&metodo_pago)
            .bind(compra.id)
            .bind(usuario_id)
            .execute(&mut *tx)
            .await?;
        }
```

- [ ] **Step 4: Fix the ledger direction in `contabilidad_service::sincronizar`**

In `services/core/src/services/contabilidad_service.rs`, replace the COMPRAS block (lines ~592-613):

```rust
        let compras: Vec<(Uuid, Decimal, Decimal, Decimal, String, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT c.id, c.subtotal, c.itbis_total, c.total, c.metodo_pago, c.created_at
               FROM compras c
               WHERE c.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = c.tenant_id AND a.referencia_tipo = 'COMPRA' AND a.referencia_id = c.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut compras_count = 0i64;
        for (id, subtotal, itbis, total, metodo_pago, created_at) in compras {
            let fecha = created_at.date_naive();
            let mut lineas = vec![("1200 Inventario".to_string(), subtotal, Decimal::ZERO)];
            if itbis > Decimal::ZERO {
                lineas.push(("1150 ITBIS Adelantado".to_string(), itbis, Decimal::ZERO));
            }
            let cuenta_pago = if metodo_pago == "FIADO" { "2110 Cuentas por Pagar" } else { "1100 Caja y Bancos" };
            lineas.push((cuenta_pago.to_string(), Decimal::ZERO, total));
            if self.create_entry(&mut tx, tenant_id, fecha, "Compra a proveedor", "AUTOMATICO", "COMPRA", Some(id), None, None, &lineas).await?.is_some() {
                compras_count += 1;
            }
        }
```

with:

```rust
        let compras: Vec<(Uuid, Decimal, Decimal, Decimal, String, String, DateTime<Utc>)> = sqlx::query_as(
            r#"SELECT c.id, c.subtotal, c.itbis_total, c.total, c.metodo_pago, c.tipo_documento, c.created_at
               FROM compras c
               WHERE c.tenant_id = $1
                 AND NOT EXISTS (SELECT 1 FROM asientos a WHERE a.tenant_id = c.tenant_id AND a.referencia_tipo = 'COMPRA' AND a.referencia_id = c.id)"#,
        )
        .bind(tenant_id)
        .fetch_all(&mut *tx)
        .await?;
        let mut compras_count = 0i64;
        for (id, subtotal, itbis, total, metodo_pago, tipo_documento, created_at) in compras {
            let fecha = created_at.date_naive();
            let cuenta_pago = if metodo_pago == "FIADO" { "2110 Cuentas por Pagar" } else { "1100 Caja y Bancos" };
            let (lineas, concepto) = if tipo_documento == "NOTA_CREDITO" {
                // Devolución a proveedor: mismas cuentas que una compra
                // normal, debe/haber invertidos, usando los montos propios
                // de esta nota (sus compra_items ya traen lo devuelto).
                let mut l = vec![("1200 Inventario".to_string(), Decimal::ZERO, subtotal)];
                if itbis > Decimal::ZERO {
                    l.push(("1150 ITBIS Adelantado".to_string(), Decimal::ZERO, itbis));
                }
                l.push((cuenta_pago.to_string(), total, Decimal::ZERO));
                (l, "Devolución a proveedor")
            } else {
                let mut l = vec![("1200 Inventario".to_string(), subtotal, Decimal::ZERO)];
                if itbis > Decimal::ZERO {
                    l.push(("1150 ITBIS Adelantado".to_string(), itbis, Decimal::ZERO));
                }
                l.push((cuenta_pago.to_string(), Decimal::ZERO, total));
                (l, "Compra a proveedor")
            };
            if self.create_entry(&mut tx, tenant_id, fecha, concepto, "AUTOMATICO", "COMPRA", Some(id), None, None, &lineas).await?.is_some() {
                compras_count += 1;
            }
        }
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd services/core && cargo test --test compras_gastos_ledger nota_credito_de_compra_reversa_stock_caja_y_ledger -- --nocapture`
Expected: PASS

- [ ] **Step 6: Run the full existing test suite to check for regressions**

Run: `cd services/core && cargo test`
Expected: PASS — in particular `compras_gastos_ledger.rs`'s existing `purchase_total_matches_hand_computed_math_and_balances_the_ledger` and `fiado_purchase_credits_cuentas_por_pagar_instead_of_caja` (plain FACTURA cases) must still pass unchanged, and `ledger_invariant.rs` must still pass.

- [ ] **Step 7: Commit**

```bash
git add services/core/src/services/compras_service.rs services/core/src/services/contabilidad_service.rs services/core/tests/compras_gastos_ledger.rs
git commit -m "fix: correctly reverse inventory, caja, and ledger for purchase-side Nota de Crédito"
```

---

### Task 2: Add Patrimonio (equity) accounts to the chart of accounts

**Files:**
- Modify: `services/core/src/bin/migrate.rs:717-738` (backfill seed for existing tenants)
- Modify: `services/core/src/services/auth_service.rs:305-324` (seed for newly-registered tenants)
- Test: `services/core/tests/chart_of_accounts.rs` (new file)

**Interfaces:**
- Consumes: existing `cuentas_contables` table (`tenant_id, codigo, nombre, tipo, naturaleza, activo`), existing `GET /v1/contabilidad/cuentas` route returning `Vec<CuentaContable { id, codigo, nombre, tipo, naturaleza, activo }>`.
- Produces: two new rows per tenant, codes `3100` and `3200`, `tipo = "PATRIMONIO"`. No other task depends on these codes existing yet (Phase F's P&L/Balance Sheet work, not in this plan, will consume them later).

- [ ] **Step 1: Write the failing test**

Create `services/core/tests/chart_of_accounts.rs`:

```rust
//! Chart-of-accounts seeding: every newly-registered tenant must get the
//! full plan de cuentas, including the Patrimonio (equity) accounts needed
//! for a balance sheet to ever balance (Activo = Pasivo + Patrimonio).

mod common;

use common::*;

#[tokio::test]
async fn new_tenant_is_seeded_with_patrimonio_accounts() {
    let session = register_tenant().await;

    let cuentas = session.get("/v1/contabilidad/cuentas").await;
    let cuentas = cuentas.as_array().expect("GET /v1/contabilidad/cuentas must return an array");

    let capital = cuentas.iter().find(|c| c["codigo"] == "3100").expect("3100 Capital Social must exist");
    assert_eq!(capital["tipo"], "PATRIMONIO", "3100 must be tipo PATRIMONIO");
    assert_eq!(capital["naturaleza"], "ACREEDORA", "3100 must be naturaleza ACREEDORA");

    let resultados = cuentas.iter().find(|c| c["codigo"] == "3200").expect("3200 Resultados del Ejercicio must exist");
    assert_eq!(resultados["tipo"], "PATRIMONIO", "3200 must be tipo PATRIMONIO");
    assert_eq!(resultados["naturaleza"], "ACREEDORA", "3200 must be naturaleza ACREEDORA");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cd services/core && cargo test --test chart_of_accounts -- --nocapture`
Expected: FAIL with `3100 Capital Social must exist` (the account doesn't exist yet).

- [ ] **Step 3: Add the two accounts to `auth_service.rs`'s seed list**

In `services/core/src/services/auth_service.rs`, inside the `PLAN_DE_CUENTAS` const array (~lines 305-324), insert two new rows right after the PASIVO block (`("2200", ...)`) and before the INGRESO block (`("4100", ...)`):

```rust
            ("2200", "Retenciones y Descuentos", "PASIVO", "ACREEDORA"),
            ("3100", "Capital Social", "PATRIMONIO", "ACREEDORA"),
            ("3200", "Resultados del Ejercicio", "PATRIMONIO", "ACREEDORA"),
            ("4100", "Ingresos por Ventas", "INGRESO", "ACREEDORA"),
```

- [ ] **Step 4: Add the same two accounts to `migrate.rs`'s backfill seed**

In `services/core/src/bin/migrate.rs`, inside the `VALUES` list at ~lines 717-738, make the identical insertion:

```rust
            ('2200', 'Retenciones y Descuentos', 'PASIVO', 'ACREEDORA'),
            ('3100', 'Capital Social', 'PATRIMONIO', 'ACREEDORA'),
            ('3200', 'Resultados del Ejercicio', 'PATRIMONIO', 'ACREEDORA'),
            ('4100', 'Ingresos por Ventas', 'INGRESO', 'ACREEDORA'),
```

- [ ] **Step 5: Run the test to verify it passes**

Run: `cd services/core && cargo test --test chart_of_accounts -- --nocapture`
Expected: PASS (the test registers a *new* tenant via `register_tenant()`, so it exercises `auth_service.rs`'s seed path from Step 3 — this is sufficient to make the test pass).

- [ ] **Step 6: Backfill existing tenants by re-running the migration binary**

Run: `cd services/core && cargo run --bin migrate`
Expected: exits successfully; the `INSERT ... ON CONFLICT` in `migrate.rs` is idempotent (`UNIQUE(tenant_id, codigo)`), so this adds the two new rows to every tenant that existed before this change, and does nothing to tenants that already have them (including the one the test just created).

- [ ] **Step 7: Run the full existing test suite to check for regressions**

Run: `cd services/core && cargo test`
Expected: PASS

- [ ] **Step 8: Commit**

```bash
git add services/core/src/services/auth_service.rs services/core/src/bin/migrate.rs services/core/tests/chart_of_accounts.rs
git commit -m "feat: seed Patrimonio (equity) accounts in the chart of accounts"
```

---

### Task 3: Correct compliance/marketing overclaims and the audit doc

**Files:**
- Modify: `README.md`
- Modify: `apps/web/app/(customer)/page.tsx`
- Modify: `docs/14-COMPLIANCE-WORKFLOW-UIUX-AUDIT.md`

**Interfaces:** None — this task changes only prose/copy, no code behavior, no test dependencies from or to other tasks.

- [ ] **Step 1: Fix `README.md`**

Replace line 5:
```
> No es un POS CRUD. Es un núcleo bancario con contabilidad de doble entrada, nómina con adelantos 50% y firma XAdES-BES real.
```
with:
```
> No es un POS CRUD. Es un núcleo bancario con contabilidad de doble entrada, nómina con adelantos 50% y firma XML-DSig real según especificación e-CF de la DGII.
```

Replace line 20:
```
                • Firma XAdES-BES real per DGII spec (C14N, RSA-SHA256, DigestValue, QR)
```
with:
```
                • Firma XML-DSig real per DGII e-CF spec (C14N, RSA-SHA256, DigestValue, QR)
```

Replace line 35:
```
- Firma: XAdES-BES real implementada en Rust
```
with:
```
- Firma: XML-DSig (enveloped signature) real implementada en Rust, según especificación "Firmado de e-CF" de la DGII
```

Replace line 40:
```
- Reportes: 606 Compras, 607 Ventas, 608 Anulados, 609 Pagos exterior, IT-1
```
with:
```
- Reportes: 606 Compras, IT-1 (607 y 608 no aplican — un emisor 100% electrónico está exento por Norma 07-2018 Art. 4/8; 609 Pagos al Exterior aún no implementado)
```

Replace line 88 heading:
```
## 🔐 Firma XAdES-BES Real en Rust
```
with:
```
## 🔐 Firma XML-DSig Real en Rust (especificación e-CF DGII)
```

Replace line 141:
```
Hecho en 🇩🇴 Santo Domingo, listo para certificación PSFE DGII.
```
with:
```
Hecho en 🇩🇴 Santo Domingo. En preparación para certificación PSFE DGII (aún no certificado).
```

- [ ] **Step 2: Fix `apps/web/app/(customer)/page.tsx`**

Replace line 53:
```tsx
    body: "606, 608, IT-1 generados automáticamente. Súbelos y ya — sin hojas de cálculo a última hora.",
```
with:
```tsx
    body: "606 e IT-1 generados automáticamente. Súbelos y ya — sin hojas de cálculo a última hora.",
```

Replace line 114:
```tsx
    a: "Sí. Firmamos cada comprobante con XAdES-BES real (C14N, RSA-SHA256) siguiendo el estándar de la DGII, y generamos el QR de verificación en el momento del cobro. Los reportes 606, 608 e IT-1 salen del mismo dato, así que nunca hay descuadre.",
```
with:
```tsx
    a: "Sí. Firmamos cada comprobante con XML-DSig real (C14N, RSA-SHA256) siguiendo la especificación de Firmado de e-CF de la DGII, y generamos el QR de verificación en el momento del cobro. El reporte 606 e IT-1 salen del mismo dato, así que nunca hay descuadre.",
```

Replace line 267:
```tsx
          <span className="flex items-center gap-2"><ShieldCheck className="h-4 w-4" /> Firma XAdES-BES real</span>
```
with:
```tsx
          <span className="flex items-center gap-2"><ShieldCheck className="h-4 w-4" /> Firma XML-DSig real (spec e-CF DGII)</span>
```

Replace line 270:
```tsx
          <span className="flex items-center gap-2"><FileBarChart className="h-4 w-4" /> Reportes 606 · 608 · IT-1</span>
```
with:
```tsx
          <span className="flex items-center gap-2"><FileBarChart className="h-4 w-4" /> Reportes 606 · IT-1</span>
```

- [ ] **Step 3: Verify no stray references remain**

Run: `grep -n "XAdES-BES\|608\|609\|PSFE" README.md "apps/web/app/(customer)/page.tsx"`
Expected: no output referring to a claimed-but-unbuilt 608/609 report or an uncertified "listo para certificación PSFE"; any remaining `609` hits should only be the honest "aún no implementado" note from Step 1, and any remaining `608` hits should only be the "no aplica" explanation from Step 1.

- [ ] **Step 4: Update the audit doc's send_to_dgii finding**

In `docs/14-COMPLIANCE-WORKFLOW-UIUX-AUDIT.md`, find cross-cutting critical finding #3 (currently starting with `**Verify whether \`send_to_dgii()\` is still mocked in the live sale path.**`) and replace its full text with:

```markdown
3. ~~Verify whether `send_to_dgii()` is still mocked in the live sale path.~~ **RESOLVED — confirmed not a bug (2026-09-18).** The live venta e-CF handler in `services/core/src/main.rs` (the e-CF section of the venta-creation flow, ~line 2370-2400) already calls the real `DGIIClient::authenticate` + `send_with_polling`, with a legitimate `CONTINGENCIA_PENDIENTE` fallback if DGII is unreachable. The fabricated-TrackID mock in `ecfl_service.rs`/`ecf_service.rs` is only reachable via the separate `/v1/test/sign-demo` endpoint, which is clearly a demo/test path and not used by real sales.
```

Then find the same item in the "Prioritized punch list" section (item 3, `Confirm whether the mocked send_to_dgii() path is reachable from the live sale flow...`) and replace it with:

```markdown
3. ~~Confirm whether the mocked `send_to_dgii()` path is reachable from the live sale flow~~ — **RESOLVED, not a bug.** See the corrected cross-cutting finding #3 above.
```

- [ ] **Step 5: Sanity-build the web app**

Run: `cd apps/web && pnpm build`
Expected: build succeeds (this catches any accidental TSX syntax error introduced by the string edits in Step 2).

- [ ] **Step 6: Commit**

```bash
git add README.md "apps/web/app/(customer)/page.tsx" docs/14-COMPLIANCE-WORKFLOW-UIUX-AUDIT.md
git commit -m "docs: correct DGII compliance/marketing overclaims (608/609/PSFE/XAdES-BES) and close out the send_to_dgii audit finding"
```

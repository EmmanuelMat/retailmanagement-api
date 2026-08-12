//! Accounting-integrity tests for docs/12-LIBRO-DIARIO-LIBRO-MAYOR-PLAN.md.
//! Complements ledger_invariant.rs (which covers the balance invariant
//! across the original flows) with tests specific to what this plan added:
//! the FIADO/COGS fix, the four new sincronizar loops, reversal, and
//! accounting periods.

mod common;

use common::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Fetches every asientos_contables line for one `referencia_tipo` in this
/// tenant (each test uses its own fresh tenant, so this is unambiguous even
/// without filtering by referencia_id, which the endpoint doesn't expose).
async fn lineas_por_referencia(session: &TenantSession, referencia_tipo: &str) -> Vec<serde_json::Value> {
    let page = session.get(&format!("/v1/contabilidad/asientos?referenciaTipo={referencia_tipo}&pageSize=200")).await;
    page["items"].as_array().cloned().unwrap_or_default()
}

fn suma_cuenta(lineas: &[serde_json::Value], cuenta: &str, campo: &str) -> Decimal {
    lineas.iter().filter(|l| l["cuenta"] == cuenta).map(|l| decimal_field(l, campo)).sum()
}

#[tokio::test]
async fn fiado_sale_debits_receivable_not_cash() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let cliente = session
        .post("/v1/clientes", serde_json::json!({ "nombre": "Cliente Fiado", "limite_credito": "10000.00" }))
        .await;
    let producto = create_producto(&session, dec!(100.00), dec!(5)).await;
    session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": producto["id"], "cantidad": "1" }],
                "metodo_pago": "FIADO",
                "cliente_id": cliente["id"],
            }),
        )
        .await;

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let lineas = lineas_por_referencia(&session, "VENTA").await;
    assert_decimal_eq(suma_cuenta(&lineas, "1110 Cuentas por Cobrar", "debe"), dec!(118.00), "FIADO sale must debit Cuentas por Cobrar");
    assert_decimal_eq(suma_cuenta(&lineas, "1100 Caja y Bancos", "debe"), dec!(0), "FIADO sale must NOT debit Caja y Bancos");
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn cash_sale_still_debits_cash_not_receivable() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(5)).await;
    session
        .post(
            "/v1/ventas",
            serde_json::json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }], "metodo_pago": "EFECTIVO" }),
        )
        .await;

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let lineas = lineas_por_referencia(&session, "VENTA").await;
    assert_decimal_eq(suma_cuenta(&lineas, "1100 Caja y Bancos", "debe"), dec!(118.00), "cash sale must debit Caja y Bancos");
    assert_decimal_eq(suma_cuenta(&lineas, "1110 Cuentas por Cobrar", "debe"), dec!(0), "cash sale must NOT touch Cuentas por Cobrar");
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn sale_posts_cost_of_goods_sold_against_inventory() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(0)).await;
    // A purchase sets productos.costo (weighted average) and is what
    // venta_items.costo_unitario captures at sale time.
    session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto["id"], "cantidad": "10", "costo_unitario": "40.00" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    session
        .post("/v1/ventas", serde_json::json!({ "items": [{ "producto_id": producto["id"], "cantidad": "3" }], "metodo_pago": "EFECTIVO" }))
        .await;

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let lineas = lineas_por_referencia(&session, "VENTA").await;
    assert_decimal_eq(suma_cuenta(&lineas, "5050 Costo de Ventas", "debe"), dec!(120.00), "COGS = 3 units * 40.00 cost");
    assert_decimal_eq(suma_cuenta(&lineas, "1200 Inventario", "haber"), dec!(120.00), "Inventario must be credited for the same COGS amount");
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn customer_payment_reduces_receivable_and_increases_cash() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let cliente = session
        .post("/v1/clientes", serde_json::json!({ "nombre": "Cliente Abono", "limite_credito": "10000.00" }))
        .await;
    let producto = create_producto(&session, dec!(100.00), dec!(5)).await;
    session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": producto["id"], "cantidad": "1" }],
                "metodo_pago": "FIADO",
                "cliente_id": cliente["id"],
            }),
        )
        .await;
    session.post(&format!("/v1/clientes/{}/abonos", cliente["id"].as_str().unwrap()), serde_json::json!({ "monto": "50.00" })).await;

    let sync = session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    assert_eq!(sync["abonos_procesados"], 1);

    let lineas = lineas_por_referencia(&session, "ABONO_CLIENTE").await;
    assert_decimal_eq(suma_cuenta(&lineas, "1100 Caja y Bancos", "debe"), dec!(50.00), "abono must debit Caja");
    assert_decimal_eq(suma_cuenta(&lineas, "1110 Cuentas por Cobrar", "haber"), dec!(50.00), "abono must credit Cuentas por Cobrar");
    assert_ledger_balanced(&session).await;
}

// NOTE: there is intentionally no black-box test here for
// `sincronizar`'s notas_credito loop (docs/12-...md §4/§5 step 4). Creating
// a nota de crédito over real HTTP (`POST /v1/ventas/:id/nota-credito`)
// requires the original sale to already have a real emitted e-CF
// (`http_crear_nota_credito` in main.rs), which requires a signed P12
// certificate - the same DGII e-CF signing pipeline that
// scripts/e2e/run-local.sh already excludes from this suite's scope
// ("DGII e-CF is explicitly excluded from this suite's scope"). The
// reversal mechanism itself (mirror lines, swap debe/haber, link via
// reversa_de, reject double-reversal) is exercised directly by
// `reversing_a_manual_entry_nets_to_zero_and_cannot_be_reversed_twice`
// below - the notas_credito loop calls the exact same `create_entry` path.

#[tokio::test]
async fn inventory_shrinkage_adjustment_posts_merma_against_inventory() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(50.00), dec!(10)).await;
    session
        .post(
            "/v1/inventario/movimientos",
            serde_json::json!({ "producto_id": producto["id"], "tipo": "AJUSTE", "cantidad": "-2", "costo_unitario": "30.00", "motivo": "Merma e2e" }),
        )
        .await;

    let sync = session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    assert_eq!(sync["ajustes_procesados"], 1);

    let lineas = lineas_por_referencia(&session, "AJUSTE_INVENTARIO").await;
    assert_decimal_eq(suma_cuenta(&lineas, "5295 Ajuste de Inventario (Merma)", "debe"), dec!(60.00), "shrinkage = 2 units * 30.00");
    assert_decimal_eq(suma_cuenta(&lineas, "1200 Inventario", "haber"), dec!(60.00), "Inventario credited for the loss");
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn bank_deposit_moves_value_from_cash_to_bank_account() {
    let session = register_tenant().await;
    let banco = session.post("/v1/bancos", serde_json::json!({ "nombre_banco": "Banco Test e2e" })).await;
    session.post(&format!("/v1/bancos/{}/movimientos", banco["id"].as_str().unwrap()), serde_json::json!({ "tipo": "DEPOSITO", "monto": "500.00" })).await;

    let sync = session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    assert_eq!(sync["banco_procesados"], 1);

    let lineas = lineas_por_referencia(&session, "BANCO").await;
    assert_decimal_eq(suma_cuenta(&lineas, "1160 Depósitos Bancarios", "debe"), dec!(500.00), "deposit debits Depósitos Bancarios");
    assert_decimal_eq(suma_cuenta(&lineas, "1100 Caja y Bancos", "haber"), dec!(500.00), "deposit credits Caja y Bancos");
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn reversing_a_manual_entry_nets_to_zero_and_cannot_be_reversed_twice() {
    let session = register_tenant().await;
    let asiento = session
        .post(
            "/v1/contabilidad/asientos",
            serde_json::json!({
                "descripcion": "Asiento manual e2e",
                "lineas": [
                    { "cuenta": "1100 Caja y Bancos", "debe": "200.00", "haber": "0" },
                    { "cuenta": "4200 Otros Ingresos", "debe": "0", "haber": "200.00" },
                ],
            }),
        )
        .await;
    let asiento_id = asiento["asientos"][0]["asiento_id"].as_str().unwrap().to_string();

    session.post(&format!("/v1/contabilidad/asientos/{asiento_id}/reversar"), serde_json::json!({ "motivo": "test" })).await;
    assert_ledger_balanced(&session).await;

    let (status, body) = session.post_expect(&format!("/v1/contabilidad/asientos/{asiento_id}/reversar"), serde_json::json!({})).await;
    assert!(!status.is_success(), "reversing the same entry twice must be rejected, got {status}: {body}");

    // Reversing a REVERSION must also be rejected.
    let lineas = session.get("/v1/contabilidad/libro-diario?pageSize=50").await;
    let reversion_id = lineas["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["cabecera"]["origen"] == "REVERSION")
        .expect("reversal entry should exist")["cabecera"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let (status2, body2) = session.post_expect(&format!("/v1/contabilidad/asientos/{reversion_id}/reversar"), serde_json::json!({})).await;
    assert!(!status2.is_success(), "reversing a REVERSION must be rejected, got {status2}: {body2}");
}

#[tokio::test]
async fn closed_period_rejects_new_manual_entries() {
    let session = register_tenant().await;
    let hoy = chrono::Utc::now().date_naive();
    session.post(&format!("/v1/contabilidad/periodos/{}/{}/cerrar", hoy.format("%Y"), hoy.format("%-m")), serde_json::json!({})).await;

    let (status, body) = session
        .post_expect(
            "/v1/contabilidad/asientos",
            serde_json::json!({
                "descripcion": "No debería poder crearse",
                "lineas": [
                    { "cuenta": "1100 Caja y Bancos", "debe": "10.00", "haber": "0" },
                    { "cuenta": "4200 Otros Ingresos", "debe": "0", "haber": "10.00" },
                ],
            }),
        )
        .await;
    assert!(!status.is_success(), "posting into a closed period must be rejected, got {status}: {body}");
}

#[tokio::test]
async fn concurrent_sincronizar_calls_never_double_post_the_same_sale() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(5)).await;
    session
        .post("/v1/ventas", serde_json::json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }], "metodo_pago": "EFECTIVO" }))
        .await;

    let (a, b) = tokio::join!(
        session.post("/v1/contabilidad/sincronizar", serde_json::json!({})),
        session.post("/v1/contabilidad/sincronizar", serde_json::json!({})),
    );
    let total_procesadas = a["ventas_procesadas"].as_i64().unwrap() + b["ventas_procesadas"].as_i64().unwrap();
    assert_eq!(total_procesadas, 1, "exactly one of the two concurrent calls must have posted the sale, got a={a} b={b}");

    let lineas = lineas_por_referencia(&session, "VENTA").await;
    assert_decimal_eq(suma_cuenta(&lineas, "4100 Ingresos por Ventas", "haber"), dec!(100.00), "revenue must be posted exactly once, not doubled");
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn every_ledger_line_traces_back_to_its_journal_entry_and_source_sale() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(5)).await;
    let venta = session
        .post("/v1/ventas", serde_json::json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }], "metodo_pago": "EFECTIVO" }))
        .await;
    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let diario = session.get("/v1/contabilidad/libro-diario?pageSize=50").await;
    let entrada = diario["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["cabecera"]["referencia_tipo"] == "VENTA" && a["cabecera"]["referencia_id"] == venta["id"])
        .expect("the sale's journal entry must be findable via the diario");
    let lineas = entrada["lineas"].as_array().unwrap();
    assert!(lineas.len() >= 2, "journal entry must group all of the sale's lines together");
    let total_debe: Decimal = lineas.iter().map(|l| decimal_field(l, "debe")).sum();
    let total_haber: Decimal = lineas.iter().map(|l| decimal_field(l, "haber")).sum();
    assert_decimal_eq(total_debe, total_haber, "the sale's own journal entry must itself balance");
}

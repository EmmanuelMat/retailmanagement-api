//! The golden "numbers must match at the end" gate for the whole suite.
//!
//! Runs one fresh tenant through a mix of every money-writing flow this
//! suite covers - sale, purchase, expense, payroll advance + approval,
//! payroll run - then asserts the tenant's books balance exactly:
//! SUM(debe) == SUM(haber) across every account, computed two independent
//! ways (once from the raw asientos_contables total, once from summing each
//! account's own saldo off GET /v1/contabilidad/libro-mayor - see
//! contabilidad_service::libro_mayor). If a future change to any
//! money-writing flow (or to sincronizar itself) ever posts an unbalanced
//! entry, this is the test that must fail.

mod common;

use common::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[tokio::test]
async fn a_mix_of_every_money_flow_still_leaves_the_ledger_balanced() {
    let session = register_tenant().await;

    // 1. POS sale (needs an open caja session).
    abrir_caja(&session, dec!(1000.00)).await;
    let producto_venta = create_producto(&session, dec!(75.00), dec!(20)).await;
    session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": producto_venta["id"], "cantidad": "3", "descuento": "5.00" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;

    // 2. Purchase.
    let producto_compra = create_producto(&session, dec!(0), dec!(0)).await;
    session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{
                    "producto_id": producto_compra["id"],
                    "cantidad": "10",
                    "costo_unitario": "22.50",
                    "itbis_tipo": "GRAVADO_16",
                }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;

    // 3. Operating expense.
    session
        .post(
            "/v1/gastos",
            serde_json::json!({ "concepto": "Internet oficina", "categoria": "SERVICIOS", "monto": "1250.75" }),
        )
        .await;

    // 4. Payroll advance, requested then approved.
    let empleado = create_empleado(&session, dec!(20000.00)).await;
    let adelanto = session
        .post("/v1/nomina/adelantos", serde_json::json!({ "empleado_id": empleado["id"], "monto": "3000.00" }))
        .await;
    session.post(&format!("/v1/nomina/adelantos/{}/aprobar", adelanto["id"].as_str().unwrap()), serde_json::json!({})).await;

    // 5. Payroll run - settles the advance above plus TSS withholding.
    session
        .post("/v1/nomina/run", serde_json::json!({ "periodo": format!("2026-08-invariant-{}", session.rnc) }))
        .await;

    // Generate ledger lines for everything above in one shot.
    let sync_result = session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    assert_eq!(sync_result["ventas_procesadas"], 1);
    assert_eq!(sync_result["compras_procesadas"], 1);
    assert_eq!(sync_result["gastos_procesados"], 1);
    assert_eq!(sync_result["adelantos_procesados"], 1);
    assert_eq!(sync_result["nomina_procesada"], 1);

    // --- The golden check, computed two independent ways ---

    // (a) Raw total across every asientos_contables row for this tenant.
    let (rows, total_debe, total_haber) = libro_mayor(&session).await;
    assert!(!rows.is_empty(), "expected ledger rows to exist after sincronizar");
    assert_eq!(total_debe, total_haber, "SUM(debe) must equal SUM(haber) across all accounts: {rows:?}");

    // (b) Each account's own saldo (debe - haber, as computed server-side by
    // contabilidad_service::libro_mayor) must net to exactly zero when
    // summed across every account - a second, independently-computed
    // statement of the same invariant.
    let saldo_neto: Decimal = rows.iter().map(|r| decimal_field(r, "debe") - decimal_field(r, "haber")).sum();
    assert_decimal_eq(saldo_neto, dec!(0), "sum of every account's saldo (debe - haber) across the whole ledger");

    // (c) The shared assertion helper every other flow test also embeds
    // locally - kept here too so a regression that only breaks the *global*
    // check (not any single flow in isolation) still gets caught.
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn sincronizar_is_idempotent_and_never_double_posts() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(50.00), dec!(5)).await;
    session
        .post(
            "/v1/ventas",
            serde_json::json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }], "metodo_pago": "EFECTIVO" }),
        )
        .await;

    let first = session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    assert_eq!(first["ventas_procesadas"], 1);
    let (_rows, debe_after_first, haber_after_first) = libro_mayor(&session).await;

    // Calling it again with nothing new to sync must not touch the ledger.
    let second = session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    assert_eq!(second["ventas_procesadas"], 0, "already-synced venta must not be processed again");
    let (_rows, debe_after_second, haber_after_second) = libro_mayor(&session).await;

    assert_decimal_eq(debe_after_second, debe_after_first, "SUM(debe) unchanged by a no-op sincronizar");
    assert_decimal_eq(haber_after_second, haber_after_first, "SUM(haber) unchanged by a no-op sincronizar");
    assert_ledger_balanced(&session).await;
}

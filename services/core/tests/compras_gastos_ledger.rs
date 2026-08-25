//! Purchases and operating-expense money-correctness: totals math and the
//! resulting general-ledger lines (contabilidad_service::sincronizar's
//! "COMPRA"/"GASTO" branches).

mod common;

use common::*;
use rust_decimal_macros::dec;

#[tokio::test]
async fn purchase_total_matches_hand_computed_math_and_balances_the_ledger() {
    let session = register_tenant().await;

    // costo_unitario 40.00, cantidad 5, GRAVADO_18 => subtotal 200.00, itbis 36.00, total 236.00
    let producto = create_producto(&session, dec!(0), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap();

    let compra = session
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

    assert_decimal_eq(decimal_field(&compra, "subtotal"), dec!(200.00), "compra.subtotal");
    assert_decimal_eq(decimal_field(&compra, "itbis_total"), dec!(36.00), "compra.itbis_total");
    assert_decimal_eq(decimal_field(&compra, "total"), dec!(236.00), "compra.total");

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let asientos = session.get("/v1/contabilidad/asientos?referenciaTipo=COMPRA&pageSize=50").await;
    let lineas = asientos["items"].as_array().expect("asientos.items");
    assert_eq!(lineas.len(), 3, "expected 3 ledger lines for one COMPRA (inventario/itbis/caja): {lineas:?}");

    let mut por_cuenta = std::collections::HashMap::<String, (rust_decimal::Decimal, rust_decimal::Decimal)>::new();
    for linea in lineas {
        let cuenta = linea["cuenta"].as_str().unwrap().to_string();
        let entry = por_cuenta.entry(cuenta).or_insert((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO));
        entry.0 += decimal_field(linea, "debe");
        entry.1 += decimal_field(linea, "haber");
    }
    assert_decimal_eq(por_cuenta["1200 Inventario"].0, dec!(200.00), "1200 Inventario debe");
    assert_decimal_eq(por_cuenta["1150 ITBIS Adelantado"].0, dec!(36.00), "1150 ITBIS Adelantado debe");
    assert_decimal_eq(por_cuenta["1100 Caja y Bancos"].1, dec!(236.00), "1100 Caja y Bancos haber");

    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn fiado_purchase_credits_cuentas_por_pagar_instead_of_caja() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(0), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap();

    let compra = session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{
                    "producto_id": producto_id,
                    "cantidad": "5",
                    "costo_unitario": "40.00",
                    "itbis_tipo": "GRAVADO_18",
                }],
                "metodo_pago": "FIADO",
                "fecha_vencimiento": "2026-12-31",
            }),
        )
        .await;
    assert_decimal_eq(decimal_field(&compra, "total"), dec!(236.00), "compra.total");

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let asientos = session.get("/v1/contabilidad/asientos?referenciaTipo=COMPRA&pageSize=50").await;
    let lineas = asientos["items"].as_array().expect("asientos.items");
    assert_eq!(lineas.len(), 3, "expected 3 ledger lines for one FIADO COMPRA (inventario/itbis/cxp): {lineas:?}");
    assert!(
        !lineas.iter().any(|l| l["cuenta"] == "1100 Caja y Bancos"),
        "a FIADO purchase must not move Caja y Bancos: {lineas:?}"
    );
    let cxp = lineas.iter().find(|l| l["cuenta"] == "2110 Cuentas por Pagar").expect("2110 Cuentas por Pagar line");
    assert_decimal_eq(decimal_field(cxp, "haber"), dec!(236.00), "2110 Cuentas por Pagar haber");

    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn fiado_purchase_without_fecha_vencimiento_is_rejected() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(0), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap();

    let (status, body) = session
        .post_expect(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "1", "costo_unitario": "10.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "FIADO",
            }),
        )
        .await;
    assert!(status.is_client_error(), "FIADO purchase without fecha_vencimiento should be rejected, got {status}: {body}");
}

#[tokio::test]
async fn expense_amount_matches_and_balances_the_ledger() {
    let session = register_tenant().await;

    let gasto = session
        .post(
            "/v1/gastos",
            serde_json::json!({
                "concepto": "Alquiler local agosto",
                "categoria": "ALQUILER",
                "monto": "15000.00",
            }),
        )
        .await;
    assert_decimal_eq(decimal_field(&gasto, "monto"), dec!(15000.00), "gasto.monto");

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let asientos = session.get("/v1/contabilidad/asientos?referenciaTipo=GASTO&pageSize=50").await;
    let lineas = asientos["items"].as_array().expect("asientos.items");
    assert_eq!(lineas.len(), 2, "expected 2 ledger lines for one GASTO (gasto/caja): {lineas:?}");

    let gasto_linea = lineas.iter().find(|l| l["cuenta"] == "5210 Gasto de Alquiler").expect("5210 Gasto de Alquiler line");
    assert_decimal_eq(decimal_field(gasto_linea, "debe"), dec!(15000.00), "5210 Gasto de Alquiler debe");
    let caja_linea = lineas.iter().find(|l| l["cuenta"] == "1100 Caja y Bancos").expect("1100 Caja y Bancos line");
    assert_decimal_eq(decimal_field(caja_linea, "haber"), dec!(15000.00), "1100 Caja y Bancos haber");

    assert_ledger_balanced(&session).await;
}

//! POS sale money-correctness: totals math and the resulting general-ledger
//! lines (see contabilidad_service::sincronizar's "VENTA" branch).

mod common;

use common::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

#[tokio::test]
async fn sale_total_matches_hand_computed_math_and_balances_the_ledger() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;

    // precio_venta 100.00, cantidad 2, GRAVADO_18 (18% ITBIS), no descuento:
    //   subtotal = 200.00, itbis = 36.00, total = 236.00
    let producto = create_producto(&session, dec!(100.00), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap();

    let venta = session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "2" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;

    assert_decimal_eq(decimal_field(&venta, "subtotal"), dec!(200.00), "venta.subtotal");
    assert_decimal_eq(decimal_field(&venta, "itbis_total"), dec!(36.00), "venta.itbis_total");
    assert_decimal_eq(decimal_field(&venta, "total"), dec!(236.00), "venta.total");
    // subtotal + itbis must equal total exactly - the core money invariant
    // of a single sale, independent of any hand-computed expectation above.
    assert_decimal_eq(
        decimal_field(&venta, "subtotal") + decimal_field(&venta, "itbis_total"),
        decimal_field(&venta, "total"),
        "venta.subtotal + venta.itbis_total vs venta.total",
    );

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let asientos = session.get("/v1/contabilidad/asientos?referenciaTipo=VENTA&pageSize=50").await;
    let lineas = asientos["items"].as_array().expect("asientos.items");
    assert_eq!(lineas.len(), 3, "expected 3 ledger lines for one VENTA (caja/ingresos/itbis): {lineas:?}");

    let mut por_cuenta = std::collections::HashMap::<String, (Decimal, Decimal)>::new();
    for linea in lineas {
        let cuenta = linea["cuenta"].as_str().unwrap().to_string();
        let entry = por_cuenta.entry(cuenta).or_insert((Decimal::ZERO, Decimal::ZERO));
        entry.0 += decimal_field(linea, "debe");
        entry.1 += decimal_field(linea, "haber");
    }
    assert_decimal_eq(por_cuenta["1100 Caja y Bancos"].0, dec!(236.00), "1100 Caja y Bancos debe");
    assert_decimal_eq(por_cuenta["4100 Ingresos por Ventas"].1, dec!(200.00), "4100 Ingresos por Ventas haber");
    assert_decimal_eq(por_cuenta["2100 ITBIS por Pagar"].1, dec!(36.00), "2100 ITBIS por Pagar haber");

    let total_debe: Decimal = lineas.iter().map(|l| decimal_field(l, "debe")).sum();
    let total_haber: Decimal = lineas.iter().map(|l| decimal_field(l, "haber")).sum();
    assert_decimal_eq(total_debe, total_haber, "VENTA ledger lines debe vs haber");

    // Golden invariant: this tenant's whole ledger balances, not just the
    // lines from this one sale.
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn sale_with_discount_reduces_subtotal_and_itbis_proportionally() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;

    // precio_venta 50.00, cantidad 4 => bruto 200.00, descuento 20.00:
    //   subtotal = 180.00, itbis = 32.40, total = 212.40
    let producto = create_producto(&session, dec!(50.00), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap();

    let venta = session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "4", "descuento": "20.00" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;

    assert_decimal_eq(decimal_field(&venta, "subtotal"), dec!(180.00), "venta.subtotal");
    assert_decimal_eq(decimal_field(&venta, "itbis_total"), dec!(32.40), "venta.itbis_total");
    assert_decimal_eq(decimal_field(&venta, "total"), dec!(212.40), "venta.total");

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    assert_ledger_balanced(&session).await;
}

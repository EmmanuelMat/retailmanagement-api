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

#[tokio::test]
async fn it1_acreditable_itbis_subtracts_a_purchase_nota_credito() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(0), dec!(20)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();

    // FACTURA: 5 @ 40.00 GRAVADO_18 => itbis 36.00.
    let factura = session
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

    // Partial NOTA_CREDITO: 2 @ 40.00 GRAVADO_18 => itbis 14.40.
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

    // Derive the IT-1 period ("YYYYMM") from the purchases' own timestamps
    // instead of the wall clock, so the test can't miss its own rows.
    let period_of = |compra: &serde_json::Value| -> String {
        let created_at = compra["created_at"].as_str().expect("compra.created_at");
        format!("{}{}", &created_at[0..4], &created_at[5..7])
    };
    let period = period_of(&factura);
    assert_eq!(period, period_of(&nota), "FACTURA and NOTA_CREDITO must fall in the same IT-1 period");

    let it1 = session.get(&format!("/v1/reports/it1?period={period}")).await;
    assert_decimal_eq(
        decimal_field(&it1, "itbis_acreditable"),
        dec!(21.60),
        "IT-1 itbis_acreditable: FACTURA 36.00 minus NOTA_CREDITO 14.40, not plus",
    );
}

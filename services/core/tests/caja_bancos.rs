//! Cash register (caja) and bank account money-correctness. Note: unlike
//! ventas/compras/gastos/nómina, caja and bancos are NOT part of the general
//! ledger (contabilidad_service::sincronizar never reads caja_movimientos or
//! bancos) - they're verified against their own resumen/saldo math instead.

mod common;

use common::*;
use rust_decimal_macros::dec;

#[tokio::test]
async fn caja_session_balances_across_a_cash_sale_and_close() {
    let session = register_tenant().await;

    let sesion = abrir_caja(&session, dec!(500.00)).await;
    assert_decimal_eq(decimal_field(&sesion, "monto_inicial"), dec!(500.00), "sesion.monto_inicial");
    assert_eq!(sesion["estado"], "ABIERTA");

    // precio_venta 100.00 x 3, GRAVADO_18 => subtotal 300.00, itbis 54.00, total 354.00
    let producto = create_producto(&session, dec!(100.00), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap();
    let venta = session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "3" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    let venta_total = decimal_field(&venta, "total");
    assert_decimal_eq(venta_total, dec!(354.00), "venta.total");

    let resumen = caja_resumen(&session).await;
    assert_decimal_eq(decimal_field(&resumen, "ingresos"), venta_total, "resumen.ingresos after cash sale");
    assert_decimal_eq(decimal_field(&resumen, "egresos"), dec!(0), "resumen.egresos with no expenses yet");
    // saldo_actual = monto_inicial + ingresos - egresos
    assert_decimal_eq(
        decimal_field(&resumen, "saldo_actual"),
        dec!(500.00) + venta_total,
        "resumen.saldo_actual = inicial + ingresos - egresos",
    );

    let esperado = decimal_field(&resumen, "saldo_actual");

    // Close with a counted amount RD$5 short of what's expected.
    let contado = esperado - dec!(5.00);
    let cerrada = cerrar_caja(&session, contado).await;
    assert_eq!(cerrada["estado"], "CERRADA");
    assert_decimal_eq(decimal_field(&cerrada, "monto_esperado"), esperado, "sesion.monto_esperado");
    assert_decimal_eq(decimal_field(&cerrada, "monto_final"), contado, "sesion.monto_final");
    // diferencia = monto_final - esperado (negative = short)
    assert_decimal_eq(decimal_field(&cerrada, "diferencia"), -dec!(5.00), "sesion.diferencia = monto_final - esperado");
}

#[tokio::test]
async fn caja_closes_exactly_when_counted_matches_expected() {
    let session = register_tenant().await;
    let sesion = abrir_caja(&session, dec!(200.00)).await;
    let resumen = caja_resumen(&session).await;
    assert_eq!(sesion["id"], resumen["sesion"]["id"], "resumen should reflect the just-opened session");

    let esperado = decimal_field(&resumen, "saldo_actual");
    let cerrada = cerrar_caja(&session, esperado).await;
    assert_decimal_eq(decimal_field(&cerrada, "diferencia"), dec!(0), "exact count => zero diferencia");
}

#[tokio::test]
async fn bank_account_balance_reflects_deposits_and_withdrawals_exactly() {
    let session = register_tenant().await;

    let banco = session
        .post(
            "/v1/bancos",
            serde_json::json!({ "nombre_banco": "Banco de Prueba", "saldo": "1000.00" }),
        )
        .await;
    let banco_id = banco["id"].as_str().unwrap().to_string();
    assert_decimal_eq(decimal_field(&banco, "saldo"), dec!(1000.00), "banco.saldo at creation");

    session
        .post(&format!("/v1/bancos/{banco_id}/movimientos"), serde_json::json!({ "tipo": "DEPOSITO", "monto": "250.50" }))
        .await;
    session
        .post(&format!("/v1/bancos/{banco_id}/movimientos"), serde_json::json!({ "tipo": "RETIRO", "monto": "300.00" }))
        .await;

    // 1000.00 + 250.50 - 300.00 = 950.50
    let bancos = session.get("/v1/bancos?pageSize=50").await;
    let items = bancos["items"].as_array().expect("bancos.items");
    let actualizado = items.iter().find(|b| b["id"] == banco_id).expect("bank account should still be listed");
    assert_decimal_eq(decimal_field(actualizado, "saldo"), dec!(950.50), "banco.saldo after deposit+withdrawal");
}

#[tokio::test]
async fn bank_withdrawal_exceeding_balance_is_rejected_and_leaves_balance_unchanged() {
    let session = register_tenant().await;
    let banco = session
        .post("/v1/bancos", serde_json::json!({ "nombre_banco": "Banco de Prueba", "saldo": "100.00" }))
        .await;
    let banco_id = banco["id"].as_str().unwrap().to_string();

    let (status, _body) = session
        .post_expect(&format!("/v1/bancos/{banco_id}/movimientos"), serde_json::json!({ "tipo": "RETIRO", "monto": "500.00" }))
        .await;
    assert!(status.is_client_error(), "overdrawing withdrawal should be rejected, got {status}");

    let bancos = session.get("/v1/bancos?pageSize=50").await;
    let items = bancos["items"].as_array().expect("bancos.items");
    let unchanged = items.iter().find(|b| b["id"] == banco_id).expect("bank account should still be listed");
    assert_decimal_eq(decimal_field(unchanged, "saldo"), dec!(100.00), "banco.saldo unchanged after rejected withdrawal");
}

//! Payroll run settlement: net-pay math (bruto - TSS - ISR - adelantos) and
//! that it correctly liquidates an approved advance, then that
//! contabilidad_service::sincronizar's "NOMINA" branch balances the ledger.

mod common;

use common::*;
use rust_decimal_macros::dec;

#[tokio::test]
async fn payroll_run_settles_net_pay_and_liquidates_the_approved_advance() {
    let session = register_tenant().await;

    // salario_mensual 10000.00 => TSS 5.91% = 591.00 exactly (no rounding
    // ambiguity), 50% advance cap = 5000.00.
    let empleado = create_empleado(&session, dec!(10000.00)).await;
    let empleado_id = empleado["id"].as_str().unwrap().to_string();

    let adelanto = session
        .post("/v1/nomina/adelantos", serde_json::json!({ "empleado_id": empleado_id, "monto": "2000.00" }))
        .await;
    let adelanto_id = adelanto["id"].as_str().unwrap().to_string();
    session.post(&format!("/v1/nomina/adelantos/{adelanto_id}/aprobar"), serde_json::json!({})).await;

    let periodo_nombre = format!("2026-08-e2e-{}", session.rnc);
    let corrida = session
        .post("/v1/nomina/run", serde_json::json!({ "periodo": periodo_nombre }))
        .await;

    let detalles = corrida["detalles"].as_array().expect("detalles");
    assert_eq!(detalles.len(), 1, "one active empleado => one detalle: {detalles:?}");
    let detalle = &detalles[0];
    assert_eq!(detalle["empleado_id"], empleado_id);
    assert_decimal_eq(decimal_field(detalle, "salario_bruto"), dec!(10000.00), "detalle.salario_bruto");
    assert_decimal_eq(decimal_field(detalle, "tss"), dec!(591.00), "detalle.tss (5.91% of 10000.00)");
    assert_decimal_eq(decimal_field(detalle, "isr"), dec!(0), "detalle.isr (out of scope, always 0)");
    assert_decimal_eq(decimal_field(detalle, "adelantos_descuento"), dec!(2000.00), "detalle.adelantos_descuento");
    // neto = bruto - tss - isr - adelantos_descuento = 10000.00 - 591.00 - 0 - 2000.00
    assert_decimal_eq(decimal_field(detalle, "neto"), dec!(7409.00), "detalle.neto");
    assert_decimal_eq(
        decimal_field(detalle, "salario_bruto")
            - decimal_field(detalle, "tss")
            - decimal_field(detalle, "isr")
            - decimal_field(detalle, "adelantos_descuento"),
        decimal_field(detalle, "neto"),
        "bruto - tss - isr - adelantos == neto",
    );

    assert_decimal_eq(decimal_field(&corrida, "total_bruto"), dec!(10000.00), "periodo.total_bruto");
    assert_decimal_eq(decimal_field(&corrida, "total_neto"), dec!(7409.00), "periodo.total_neto");

    let adelanto_liquidado = session.get(&format!("/v1/nomina/adelantos?empleadoId={empleado_id}&pageSize=10")).await;
    let items = adelanto_liquidado["items"].as_array().expect("items");
    let liquidado = items.iter().find(|a| a["id"] == adelanto_id).expect("advance should still be listed");
    assert_eq!(liquidado["estado"], "DESCONTADO", "an advance settled in a payroll run moves to DESCONTADO");

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;

    let asientos = session.get("/v1/contabilidad/asientos?referenciaTipo=NOMINA&pageSize=50").await;
    let lineas = asientos["items"].as_array().expect("asientos.items");
    let gasto_nomina = lineas.iter().find(|l| l["cuenta"] == "5100 Gasto de Nómina").expect("5100 Gasto de Nómina line");
    assert_decimal_eq(decimal_field(gasto_nomina, "debe"), dec!(10000.00), "5100 Gasto de Nómina debe == bruto");
    let retenciones = lineas.iter().find(|l| l["cuenta"] == "2200 Retenciones y Descuentos").expect("2200 Retenciones y Descuentos line");
    // retenciones = bruto - neto - adelantos_descuento = 10000.00 - 7409.00 - 2000.00 = 591.00 (== TSS, as expected)
    assert_decimal_eq(decimal_field(retenciones, "haber"), dec!(591.00), "2200 Retenciones y Descuentos haber");

    // The anticipo opened when the advance was approved (ADELANTO sync) and
    // closed when it was liquidated in this run (NOMINA sync) should net to
    // exactly zero on "1300 Anticipos a Empleados" across both.
    let anticipos = session.get("/v1/contabilidad/asientos?cuenta=1300%20Anticipos%20a%20Empleados&pageSize=50").await;
    let anticipos_lineas = anticipos["items"].as_array().expect("items");
    let debe: rust_decimal::Decimal = anticipos_lineas.iter().map(|l| decimal_field(l, "debe")).sum();
    let haber: rust_decimal::Decimal = anticipos_lineas.iter().map(|l| decimal_field(l, "haber")).sum();
    assert_decimal_eq(debe, dec!(2000.00), "1300 Anticipos a Empleados total debe (opened at approval)");
    assert_decimal_eq(haber, dec!(2000.00), "1300 Anticipos a Empleados total haber (closed at settlement)");

    assert_ledger_balanced(&session).await;
}

//! Fase F3: Estado de Resultados (P&L) y Balance General, calculados desde
//! `asientos_contables` unidos a `cuentas_contables.tipo`/`naturaleza`.
//!
//! El cheque duro es la identidad contable sobre datos realmente posteados
//! (venta, compra, gasto, nómina): `Activo + Sin clasificar = Pasivo +
//! Patrimonio + Resultado acumulado`. Como todo asiento balancea, esa
//! identidad es una consecuencia aritmética de `create_entry`, no una
//! coincidencia -- si se rompe, o hay un asiento desbalanceado o la
//! clasificación por tipo está mal.
//!
//! Nota sobre el período: no existe asiento de cierre (fuera de alcance de
//! esta fase), así que `3200 Resultados del Ejercicio` está siempre en cero
//! y el resultado del ejercicio vive como una línea calculada del balance.

mod common;

use common::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::{json, Value};
use std::str::FromStr;

fn money(value: &Value, field: &str) -> Decimal {
    decimal_field(value, field)
}

/// Un tenant con una venta, una compra, un gasto, un adelanto y una corrida
/// de nómina - el mismo mix que `ledger_invariant.rs`, ya sincronizado.
async fn tenant_con_actividad_real() -> TenantSession {
    let session = register_tenant().await;

    abrir_caja(&session, dec!(1000.00)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(50)).await;
    // Venta de contado: 2 x 100.00 => subtotal 200.00, ITBIS 36.00, total 236.00
    session
        .post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "2" }], "metodo_pago": "EFECTIVO" }))
        .await;

    let producto_compra = create_producto(&session, dec!(0), dec!(0)).await;
    session
        .post(
            "/v1/compras",
            json!({ "items": [{ "producto_id": producto_compra["id"], "cantidad": "10", "costo_unitario": "30.00", "itbis_tipo": "GRAVADO_18" }], "metodo_pago": "EFECTIVO" }),
        )
        .await;

    session.post("/v1/gastos", json!({ "concepto": "Alquiler del local", "categoria": "ALQUILER", "monto": "8000.00" })).await;

    let empleado = create_empleado(&session, dec!(18000.00)).await;
    let adelanto = session.post("/v1/nomina/adelantos", json!({ "empleado_id": empleado["id"], "monto": "2000.00" })).await;
    session.post(&format!("/v1/nomina/adelantos/{}/aprobar", adelanto["id"].as_str().unwrap()), json!({})).await;
    session.post("/v1/nomina/run", json!({ "periodo": format!("2026-09-estados-{}", session.rnc) })).await;

    session.post("/v1/contabilidad/sincronizar", json!({})).await;
    session
}

#[tokio::test]
async fn balance_general_cuadra_sobre_actividad_real() {
    let session = tenant_con_actividad_real().await;
    let hoy = chrono::Utc::now().date_naive();

    let balance = session.get(&format!("/v1/contabilidad/balance-general?al={hoy}")).await;

    let activo = money(&balance, "activo");
    let pasivo = money(&balance, "pasivo");
    let patrimonio = money(&balance, "patrimonio");
    let resultado = money(&balance, "resultado_acumulado");
    let sin_clasificar = money(&balance, "sin_clasificar");

    // El activo puede quedar negativo con estos datos (la nómina de un mes
    // sale de una caja que solo recibió una venta): lo que importa es que el
    // balance tenga movimiento real y que la identidad se cumpla.
    assert!(activo != Decimal::ZERO, "el balance debe tener movimiento real: {balance}");
    assert!(
        balance["detalle_activo"].as_array().is_some_and(|d| d.len() >= 3),
        "se esperan al menos Caja, ITBIS Adelantado e Inventario en el activo: {balance}"
    );
    assert_decimal_eq(sin_clasificar, dec!(0), "ninguna línea debería quedar sin cuenta del plan");
    assert_decimal_eq(
        activo + sin_clasificar,
        pasivo + patrimonio + resultado,
        "Activo + Sin clasificar = Pasivo + Patrimonio + Resultado acumulado",
    );
    assert_eq!(balance["cuadra"], true, "el propio endpoint debe reportar que cuadra: {balance}");

    // El detalle debe sumar exactamente el total de su bloque (si no, la
    // página muestra filas que no explican el número de arriba).
    for (bloque, total) in [("detalle_activo", activo), ("detalle_pasivo", pasivo), ("detalle_patrimonio", patrimonio)] {
        let suma: Decimal = balance[bloque].as_array().expect(bloque).iter().map(|l| money(l, "monto")).sum();
        assert_decimal_eq(suma, total, &format!("{bloque} debe sumar su total"));
    }

    // 3200 Resultados del Ejercicio sigue en cero: no hay asiento de cierre.
    let patrimonio_detalle = balance["detalle_patrimonio"].as_array().unwrap();
    let cerrado = patrimonio_detalle.iter().find(|l| l["codigo"] == "3200");
    assert!(
        cerrado.is_none() || money(cerrado.unwrap(), "monto") == Decimal::ZERO,
        "sin cierre de ejercicio, 3200 no debe tener saldo: {balance}"
    );
}

#[tokio::test]
async fn estado_resultados_separa_ingresos_costo_de_ventas_y_gastos() {
    let session = tenant_con_actividad_real().await;
    let hoy = chrono::Utc::now().date_naive();
    let desde = hoy - chrono::Duration::days(1);

    let pl = session.get(&format!("/v1/contabilidad/estado-resultados?desde={desde}&hasta={hoy}")).await;

    let ingresos = money(&pl, "ingresos");
    let costo_ventas = money(&pl, "costo_ventas");
    let gastos = money(&pl, "gastos");
    let resultado = money(&pl, "resultado");

    // Venta de 2 unidades a 100.00 (el ITBIS no es ingreso).
    assert_decimal_eq(ingresos, dec!(200.00), "ingresos = subtotal de la venta, sin ITBIS");
    // El producto se creó sin costo, así que no hay línea 5050 todavía.
    assert_decimal_eq(costo_ventas, dec!(0), "sin costo capturado no hay costo de ventas");
    // Alquiler 8,000.00 + el gasto de nómina de la corrida.
    assert!(gastos >= dec!(8000.00), "los gastos deben incluir al menos el alquiler: {pl}");
    assert_decimal_eq(ingresos - costo_ventas - gastos, resultado, "resultado = ingresos - costo de ventas - gastos");
    assert_decimal_eq(ingresos - costo_ventas, money(&pl, "utilidad_bruta"), "utilidad bruta = ingresos - costo de ventas");

    // El alquiler aparece en el detalle de gastos por su cuenta del plan.
    let alquiler = pl["detalle_gastos"]
        .as_array()
        .expect("detalle_gastos")
        .iter()
        .find(|l| l["codigo"] == "5210")
        .unwrap_or_else(|| panic!("5210 Gasto de Alquiler debe estar en el detalle: {pl}"));
    assert_decimal_eq(money(alquiler, "monto"), dec!(8000.00), "5210 Gasto de Alquiler");
}

/// El costo de ventas sí aparece cuando el producto tiene costo capturado
/// (`venta_items.costo_unitario`), que es lo que alimenta la cuenta 5050.
#[tokio::test]
async fn costo_de_ventas_sale_del_costo_capturado_en_la_venta() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(0)).await;

    // Compra primero: fija el costo del producto en 40.00.
    session
        .post(
            "/v1/compras",
            json!({ "items": [{ "producto_id": producto["id"], "cantidad": "10", "costo_unitario": "40.00" }], "metodo_pago": "EFECTIVO" }),
        )
        .await;
    session.post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "3" }], "metodo_pago": "EFECTIVO" })).await;
    session.post("/v1/contabilidad/sincronizar", json!({})).await;

    let hoy = chrono::Utc::now().date_naive();
    let pl = session.get(&format!("/v1/contabilidad/estado-resultados?desde={hoy}&hasta={hoy}")).await;
    assert_decimal_eq(money(&pl, "ingresos"), dec!(300.00), "3 x 100.00");
    assert_decimal_eq(money(&pl, "costo_ventas"), dec!(120.00), "3 x 40.00 de costo");
    assert_decimal_eq(money(&pl, "utilidad_bruta"), dec!(180.00), "300.00 - 120.00");

    let balance = session.get(&format!("/v1/contabilidad/balance-general?al={hoy}")).await;
    assert_eq!(balance["cuadra"], true, "el balance debe cuadrar también con costo de ventas: {balance}");
}

/// F4: los estados se leen "al día" sin que nadie haya pulsado Sincronizar.
#[tokio::test]
async fn los_estados_reflejan_actividad_aun_sin_sincronizar_a_mano() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(250.00), dec!(5)).await;
    session.post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }], "metodo_pago": "EFECTIVO" })).await;

    // Sin POST /v1/contabilidad/sincronizar de por medio.
    let hoy = chrono::Utc::now().date_naive();
    let pl = session.get(&format!("/v1/contabilidad/estado-resultados?desde={hoy}&hasta={hoy}")).await;
    assert_decimal_eq(money(&pl, "ingresos"), dec!(250.00), "la venta debe estar ya contabilizada al leer el estado");

    let balance = session.get(&format!("/v1/contabilidad/balance-general?al={hoy}")).await;
    assert_eq!(balance["cuadra"], true, "balance al día: {balance}");
}

/// Las fechas acotan de verdad: una venta de hoy no entra en un P&L de ayer.
#[tokio::test]
async fn el_rango_de_fechas_acota_el_estado_de_resultados() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(90.00), dec!(5)).await;
    session.post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }], "metodo_pago": "EFECTIVO" })).await;
    session.post("/v1/contabilidad/sincronizar", json!({})).await;

    let hoy = chrono::Utc::now().date_naive();
    let ayer = hoy - chrono::Duration::days(1);
    let anteayer = hoy - chrono::Duration::days(2);

    let pasado = session.get(&format!("/v1/contabilidad/estado-resultados?desde={anteayer}&hasta={ayer}")).await;
    assert_decimal_eq(money(&pasado, "ingresos"), dec!(0), "una venta de hoy no pertenece a un período anterior");

    let hoy_pl = session.get(&format!("/v1/contabilidad/estado-resultados?desde={hoy}&hasta={hoy}")).await;
    assert_decimal_eq(money(&hoy_pl, "ingresos"), dec!(90.00), "la venta de hoy sí");
}

/// Las cifras del estado de resultados y del balance salen del mismo mayor:
/// el resultado acumulado del balance (desde siempre hasta hoy) debe
/// coincidir con un P&L abierto que cubra todo el histórico.
#[tokio::test]
async fn resultado_del_balance_coincide_con_el_estado_de_resultados_historico() {
    let session = tenant_con_actividad_real().await;
    let hoy = chrono::Utc::now().date_naive();
    let hace_mucho = hoy - chrono::Duration::days(3650);

    let pl = session.get(&format!("/v1/contabilidad/estado-resultados?desde={hace_mucho}&hasta={hoy}")).await;
    let balance = session.get(&format!("/v1/contabilidad/balance-general?al={hoy}")).await;

    assert_decimal_eq(
        Decimal::from_str(balance["resultado_acumulado"].as_str().unwrap()).unwrap(),
        money(&pl, "resultado"),
        "resultado acumulado del balance vs. P&L de todo el histórico",
    );
}

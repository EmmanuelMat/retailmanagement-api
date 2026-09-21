//! Fase F1: toda acción que mueve dinero deja rastro en `auditoria`.
//!
//! El audit (docs/14 §4) encontró que las vías de escritura de mayor riesgo
//! -- asiento manual, compra, gasto, movimiento bancario y la propia
//! sincronización del mayor -- no escribían ninguna fila de auditoría,
//! mientras que ventas/nómina/reversiones sí. Estos tests leen el mismo
//! `GET /v1/auditoria` que usa la UI, no la tabla.

mod common;

use common::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::{json, Value};
use std::str::FromStr;

/// Devuelve la entrada de auditoría más reciente con esa `accion`, o falla
/// con el listado completo para que el diagnóstico no requiera la DB.
async fn entrada_auditoria(session: &TenantSession, accion: &str) -> Value {
    let page = session.get(&format!("/v1/auditoria?accion={accion}&pageSize=50")).await;
    let items = page["items"].as_array().cloned().unwrap_or_default();
    items
        .into_iter()
        .find(|e| e["accion"] == accion)
        .unwrap_or_else(|| panic!("no se registró ninguna auditoría {accion}; listado: {}", page))
}

#[tokio::test]
async fn manual_journal_entry_is_audited_with_its_lines() {
    let session = register_tenant().await;
    session
        .post(
            "/v1/contabilidad/asientos",
            json!({
                "descripcion": "Aporte del socio e2e",
                "lineas": [
                    { "cuenta": "1100 Caja y Bancos", "debe": "1500.00", "haber": "0" },
                    { "cuenta": "3100 Capital Social", "debe": "0", "haber": "1500.00" },
                ],
            }),
        )
        .await;

    let entrada = entrada_auditoria(&session, "ASIENTO_MANUAL_CREADO").await;
    assert_eq!(entrada["entidad"], "asiento");
    assert!(entrada["entidad_id"].is_string(), "el asiento auditado debe traer su id: {entrada}");
    assert!(entrada["usuario_id"].is_string(), "debe quedar registrado quién lo creó: {entrada}");
    let lineas = entrada["detalle"]["lineas"].as_array().expect("el detalle debe traer las líneas");
    assert_eq!(lineas.len(), 2, "se auditan las dos líneas del asiento: {entrada}");
    let total_debe: Decimal = lineas.iter().map(|l| Decimal::from_str(l["debe"].as_str().unwrap()).unwrap()).sum();
    assert_decimal_eq(total_debe, dec!(1500.00), "el detalle auditado conserva los montos exactos");
}

#[tokio::test]
async fn compra_gasto_y_movimiento_bancario_quedan_auditados() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(0)).await;

    session
        .post(
            "/v1/compras",
            json!({
                "items": [{ "producto_id": producto["id"], "cantidad": "3", "costo_unitario": "40.00" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    let compra = entrada_auditoria(&session, "COMPRA_REGISTRADA").await;
    assert_eq!(compra["entidad"], "compra");
    assert_eq!(compra["detalle"]["metodo_pago"], "EFECTIVO");
    // Sin `itbis_tipo` en la línea, una compra no lleva ITBIS (ver
    // compras_service::create_compra): 3 x 40.00 = 120.00 exactos.
    assert_decimal_eq(
        Decimal::from_str(compra["detalle"]["total"].as_str().expect("total auditado")).unwrap(),
        dec!(120.00),
        "el detalle auditado conserva el total exacto de la compra",
    );

    session.post("/v1/gastos", json!({ "concepto": "Luz del local", "categoria": "SERVICIOS", "monto": "850.00" })).await;
    let gasto = entrada_auditoria(&session, "GASTO_REGISTRADO").await;
    assert_eq!(gasto["entidad"], "gasto");
    assert_eq!(gasto["detalle"]["categoria"], "SERVICIOS");

    let banco = session.post("/v1/bancos", json!({ "nombre_banco": "Banco de prueba e2e" })).await;
    session
        .post(&format!("/v1/bancos/{}/movimientos", banco["id"].as_str().unwrap()), json!({ "tipo": "DEPOSITO", "monto": "300.00" }))
        .await;
    let movimiento = entrada_auditoria(&session, "BANCO_MOVIMIENTO_REGISTRADO").await;
    assert_eq!(movimiento["entidad"], "banco");
    assert_eq!(movimiento["detalle"]["tipo"], "DEPOSITO");
}

#[tokio::test]
async fn sincronizar_registra_quien_la_corrio_y_cuantos_asientos_genero() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(75.00), dec!(4)).await;
    session
        .post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }], "metodo_pago": "EFECTIVO" }))
        .await;

    session.post("/v1/contabilidad/sincronizar", json!({})).await;

    let entrada = entrada_auditoria(&session, "CONTABILIDAD_SINCRONIZADA").await;
    assert_eq!(entrada["entidad"], "contabilidad");
    assert!(entrada["usuario_id"].is_string(), "debe quedar registrado quién sincronizó: {entrada}");
    assert_eq!(entrada["detalle"]["ventas_procesadas"], 1, "los conteos por tipo van en el detalle: {entrada}");
}

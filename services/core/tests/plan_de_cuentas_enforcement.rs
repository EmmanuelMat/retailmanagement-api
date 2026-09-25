//! Fase F2: `create_entry` valida el código de cuenta contra
//! `cuentas_contables` del tenant.
//!
//! Antes, un error de tipeo en un asiento manual creaba una "cuenta
//! fantasma" permanente en el libro mayor (docs/14 §4). La validación
//! matchea **solo por el código** (el primer token del string
//! `"<codigo> <nombre>"`), nunca por el nombre: renombrar una cuenta del
//! plan no debe invalidar asientos históricos ni obligar a que las cadenas
//! hardcodeadas de `sincronizar` sigan la semilla carácter por carácter.

mod common;

use common::*;
use rust_decimal_macros::dec;
use serde_json::json;

#[tokio::test]
async fn ghost_account_code_is_rejected_with_a_spanish_error() {
    let session = register_tenant().await;

    let (status, body) = session
        .post_expect(
            "/v1/contabilidad/asientos",
            json!({
                "descripcion": "Asiento con cuenta inventada",
                "lineas": [
                    { "cuenta": "9999 Cuenta Fantasma", "debe": "100.00", "haber": "0" },
                    { "cuenta": "1100 Caja y Bancos", "debe": "0", "haber": "100.00" },
                ],
            }),
        )
        .await;

    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "una cuenta inexistente debe dar 400, dio {status}: {body}");
    let mensaje = body.as_str().unwrap_or_default();
    assert!(mensaje.contains("9999"), "el error debe nombrar el código rechazado: {mensaje:?}");
    assert!(
        mensaje.to_lowercase().contains("cuenta"),
        "el error debe estar en español y hablar de la cuenta: {mensaje:?}"
    );

    // Y nada se posteó: la cuenta fantasma no puede quedar en el mayor.
    let mayor = session.get("/v1/contabilidad/libro-mayor?pageSize=200").await;
    let hay_fantasma = mayor["items"].as_array().unwrap().iter().any(|c| c["cuenta"].as_str().unwrap_or_default().starts_with("9999"));
    assert!(!hay_fantasma, "el asiento rechazado no debe dejar rastro en el libro mayor: {}", mayor);
}

#[tokio::test]
async fn line_without_a_leading_account_code_is_rejected() {
    let session = register_tenant().await;

    let (status, body) = session
        .post_expect(
            "/v1/contabilidad/asientos",
            json!({
                "descripcion": "Asiento sin código de cuenta",
                "lineas": [
                    { "cuenta": "Caja y Bancos", "debe": "50.00", "haber": "0" },
                    { "cuenta": "4200 Otros Ingresos", "debe": "0", "haber": "50.00" },
                ],
            }),
        )
        .await;

    assert_eq!(status, reqwest::StatusCode::BAD_REQUEST, "una línea sin código debe dar 400, dio {status}: {body}");
}

/// El nombre que acompaña al código es informativo: lo que manda es el
/// código. Un asiento con el código correcto y el nombre escrito distinto
/// se acepta (si no, renombrar una cuenta rompería el histórico).
#[tokio::test]
async fn account_code_matches_even_when_the_name_differs() {
    let session = register_tenant().await;

    let asiento = session
        .post(
            "/v1/contabilidad/asientos",
            json!({
                "descripcion": "Nombre alterno, código válido",
                "lineas": [
                    { "cuenta": "1100 Efectivo", "debe": "25.00", "haber": "0" },
                    { "cuenta": "4200 Otros Ingresos", "debe": "0", "haber": "25.00" },
                ],
            }),
        )
        .await;
    assert_eq!(asiento["asientos"].as_array().map(|l| l.len()), Some(2));
}

/// La red de seguridad de F2: ninguna contabilización automática usa una
/// cuenta que no esté en el plan sembrado. Si alguien agrega un loop nuevo
/// a `sincronizar` con una cuenta no sembrada, `sincronizar` fallará aquí.
#[tokio::test]
async fn every_automatic_posting_uses_a_seeded_account() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;

    let producto = create_producto(&session, dec!(120.00), dec!(10)).await;
    session.post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "2" }], "metodo_pago": "EFECTIVO" })).await;
    session
        .post("/v1/compras", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "4", "costo_unitario": "60.00" }], "metodo_pago": "EFECTIVO" }))
        .await;
    session.post("/v1/gastos", json!({ "concepto": "Transporte e2e", "categoria": "TRANSPORTE", "monto": "500.00" })).await;
    session
        .post("/v1/inventario/movimientos", json!({ "producto_id": producto["id"], "tipo": "AJUSTE", "cantidad": "-1", "costo_unitario": "60.00", "motivo": "Merma e2e" }))
        .await;
    let banco = session.post("/v1/bancos", json!({ "nombre_banco": "Banco e2e" })).await;
    session.post(&format!("/v1/bancos/{}/movimientos", banco["id"].as_str().unwrap()), json!({ "tipo": "DEPOSITO", "monto": "200.00" })).await;

    let sync = session.post("/v1/contabilidad/sincronizar", json!({})).await;
    assert_eq!(sync["ventas_procesadas"], 1);
    assert_eq!(sync["compras_procesadas"], 1);
    assert_eq!(sync["gastos_procesados"], 1);
    assert_eq!(sync["ajustes_procesados"], 1);
    assert_eq!(sync["banco_procesados"], 1);

    // Toda cuenta del mayor debe existir en el plan de cuentas del tenant.
    let cuentas = session.get("/v1/contabilidad/cuentas").await;
    let codigos: Vec<String> = cuentas.as_array().unwrap().iter().map(|c| c["codigo"].as_str().unwrap().to_string()).collect();
    let mayor = session.get("/v1/contabilidad/libro-mayor?pageSize=200").await;
    for fila in mayor["items"].as_array().unwrap() {
        let cuenta = fila["cuenta"].as_str().unwrap();
        let codigo = cuenta.split_whitespace().next().unwrap_or_default();
        assert!(codigos.iter().any(|c| c == codigo), "la cuenta automática {cuenta:?} no está en el plan de cuentas {codigos:?}");
    }
    assert_ledger_balanced(&session).await;
}

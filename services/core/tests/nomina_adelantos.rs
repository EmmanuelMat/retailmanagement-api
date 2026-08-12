//! Payroll advance (adelanto) rule enforcement: 50%-of-salary cap, approve
//! posts a caja egreso, reject restores available capacity.

mod common;

use common::*;
use rust_decimal_macros::dec;

async fn request_adelanto(session: &TenantSession, empleado_id: &str, monto: &str) -> (reqwest::StatusCode, serde_json::Value) {
    session
        .post_expect(
            "/v1/nomina/adelantos",
            serde_json::json!({ "empleado_id": empleado_id, "monto": monto }),
        )
        .await
}

#[tokio::test]
async fn advance_at_exactly_fifty_percent_succeeds_and_a_cent_more_is_rejected() {
    let session = register_tenant().await;
    // salario_mensual 10000.00 => 50% cap = 5000.00
    let empleado = create_empleado(&session, dec!(10000.00)).await;
    let empleado_id = empleado["id"].as_str().unwrap();

    let (status, body) = request_adelanto(&session, empleado_id, "5000.00").await;
    assert!(status.is_success(), "advance at exactly 50% should succeed, got {status}: {body}");
    assert_decimal_eq(decimal_field(&body, "monto"), dec!(5000.00), "adelanto.monto");
    assert_eq!(body["estado"], "PENDIENTE");

    // Disponible is now 5000.00 - 5000.00 (still PENDIENTE, counts toward the
    // cap) = 0.00 - any further request must be rejected.
    let (status, body) = request_adelanto(&session, empleado_id, "0.01").await;
    assert!(status.is_client_error(), "advance exceeding cumulative cap should be rejected, got {status}: {body}");
    let message = body.as_str().unwrap_or_default();
    assert!(
        message.contains("Excede el disponible"),
        "expected the specific over-cap rejection message, got: {message}"
    );
}

#[tokio::test]
async fn approving_an_advance_posts_a_caja_egreso_for_the_exact_amount() {
    let session = register_tenant().await;
    let empleado = create_empleado(&session, dec!(10000.00)).await;
    let empleado_id = empleado["id"].as_str().unwrap();

    let (status, adelanto) = request_adelanto(&session, empleado_id, "2000.00").await;
    assert!(status.is_success(), "advance within cap should succeed: {adelanto}");
    let adelanto_id = adelanto["id"].as_str().unwrap().to_string();

    let aprobado = session.post(&format!("/v1/nomina/adelantos/{adelanto_id}/aprobar"), serde_json::json!({})).await;
    assert_eq!(aprobado["estado"], "APROBADO");
    assert_decimal_eq(decimal_field(&aprobado, "monto"), dec!(2000.00), "adelanto.monto unchanged by approval");

    // CajaMovimiento (services/core/src/services/caja_service.rs) has no
    // referencia_id field to match on - only referencia_tipo - so for a
    // fresh tenant with exactly one approved advance, the single ADELANTO
    // movement returned here has to be the one this test just caused.
    let movimientos = session.get("/v1/caja/movimientos?referenciaTipo=ADELANTO&pageSize=50").await;
    let items = movimientos["items"].as_array().expect("movimientos.items");
    assert_eq!(items.len(), 1, "expected exactly one ADELANTO caja_movimiento for a fresh tenant: {items:?}");
    let movimiento = &items[0];
    assert_eq!(movimiento["tipo"], "EGRESO");
    assert_decimal_eq(decimal_field(movimiento, "monto"), dec!(2000.00), "caja_movimiento.monto == adelanto.monto");
}

#[tokio::test]
async fn rejecting_an_advance_restores_available_capacity() {
    let session = register_tenant().await;
    let empleado = create_empleado(&session, dec!(10000.00)).await;
    let empleado_id = empleado["id"].as_str().unwrap();

    let (status, adelanto_a) = request_adelanto(&session, empleado_id, "5000.00").await;
    assert!(status.is_success(), "first advance at full cap should succeed: {adelanto_a}");
    let adelanto_a_id = adelanto_a["id"].as_str().unwrap().to_string();

    let (status, _body) = request_adelanto(&session, empleado_id, "100.00").await;
    assert!(status.is_client_error(), "a second advance while the first is still pending must be rejected");

    let rechazado = session.post(&format!("/v1/nomina/adelantos/{adelanto_a_id}/rechazar"), serde_json::json!({})).await;
    assert_eq!(rechazado["estado"], "RECHAZADO");

    // A RECHAZADO advance no longer counts toward the 50% cap, so the full
    // amount should be available again.
    let (status, adelanto_b) = request_adelanto(&session, empleado_id, "5000.00").await;
    assert!(status.is_success(), "advance should succeed again once the rejected one is excluded from the cap: {adelanto_b}");
}

//! Conduces ("Órdenes de Servicio" para tenants SERVICIOS) - ver
//! conduce_service.rs. Cubre ambos flujos: standalone (nuevo en esta
//! migración, sin Venta previa) y el flujo ligado a una Venta con
//! entrega_diferida (ya existía, pero no tenía cobertura de tests hasta
//! ahora).

mod common;

use common::*;
use rust_decimal_macros::dec;

async fn create_cliente(session: &TenantSession) -> serde_json::Value {
    session
        .post("/v1/clientes", serde_json::json!({ "nombre": "Cliente de prueba e2e" }))
        .await
}

#[tokio::test]
async fn standalone_conduce_moves_stock_for_producto_but_not_for_servicio() {
    let session = register_tenant().await;
    let cliente = create_cliente(&session).await;
    let cliente_id = cliente["id"].as_str().unwrap();

    let producto = create_producto(&session, dec!(0), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap();
    let servicio = create_servicio(&session).await;
    let servicio_id = servicio["id"].as_str().unwrap();

    let conduce = session
        .post(
            "/v1/conduces",
            serde_json::json!({
                "cliente_id": cliente_id,
                "items": [
                    { "producto_id": producto_id, "cantidad": "3" },
                    { "producto_id": servicio_id, "cantidad": "1", "descripcion": "Instalación" },
                ],
            }),
        )
        .await;
    assert!(conduce["venta_id"].is_null(), "a standalone conduce should have no venta_id: {conduce}");

    let producto_actualizado = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&producto_actualizado, "stock_actual"), dec!(7), "producto.stock_actual after standalone conduce (10 - 3)");

    let movimientos = session.get(&format!("/v1/inventario/movimientos?productoId={servicio_id}&pageSize=50")).await;
    let items = movimientos["items"].as_array().expect("movimientos.items");
    assert!(items.is_empty(), "a SERVICIO line on a standalone conduce must not create a movimientos_inventario row: {items:?}");
}

#[tokio::test]
async fn venta_linked_conduce_flow_is_unchanged() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(100.00), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap();

    let venta = session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "5" }],
                "metodo_pago": "EFECTIVO",
                "entrega_diferida": true,
            }),
        )
        .await;
    let venta_id = venta["id"].as_str().unwrap();

    // entrega_diferida: la venta no descontó stock todavía.
    let producto_tras_venta = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&producto_tras_venta, "stock_actual"), dec!(10), "stock unchanged right after an entrega_diferida sale");

    let venta_item_id = venta["items"][0]["id"].as_str().expect("venta.items[0].id").to_string();

    let conduce = session
        .post(
            "/v1/conduces",
            serde_json::json!({
                "venta_id": venta_id,
                "items": [{ "venta_item_id": venta_item_id, "cantidad": "5" }],
            }),
        )
        .await;
    assert_eq!(conduce["venta_id"], venta_id, "a venta-linked conduce should carry the venta_id");

    let producto_tras_conduce = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&producto_tras_conduce, "stock_actual"), dec!(5), "stock decrements when the conduce is registered, not at sale time (10 - 5)");
}

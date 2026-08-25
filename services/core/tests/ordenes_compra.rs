//! Órdenes de Compra (purchase orders) - Módulo 15b. `compras` sigue
//! representando únicamente una compra ya recibida (compras_service::create_compra,
//! sin estado propio) - estas pruebas cubren la intención previa a recibir:
//! crear, recibir total/parcial (que sí crea una `compras` real vía el
//! servicio existente y mueve inventario), y cancelar. Ver
//! tests/common/mod.rs para el harness compartido.

mod common;
use common::*;
use rust_decimal_macros::dec;
use serde_json::json;

async fn create_proveedor(session: &TenantSession) -> serde_json::Value {
    session.post("/v1/proveedores", json!({ "nombre": "Proveedor de prueba e2e" })).await
}

#[tokio::test]
async fn crear_orden_de_compra_calcula_subtotal_estimado() {
    let session = register_tenant().await;
    let proveedor = create_proveedor(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(10)).await;

    let orden = session
        .post(
            "/v1/ordenes-compra",
            json!({ "proveedor_id": proveedor["id"], "items": [{ "producto_id": producto["id"], "cantidad_solicitada": "10", "costo_unitario": "80" }] }),
        )
        .await;

    assert_eq!(orden["estado"], "BORRADOR");
    assert_decimal_eq(decimal_field(&orden, "subtotal"), dec!(800.00), "subtotal estimado");
    assert_eq!(orden["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn no_se_puede_ordenar_un_servicio_en_una_orden_de_compra() {
    let session = register_tenant().await;
    let proveedor = create_proveedor(&session).await;
    let servicio = create_servicio(&session).await;
    let (status, body) = session
        .post_expect(
            "/v1/ordenes-compra",
            json!({ "proveedor_id": proveedor["id"], "items": [{ "producto_id": servicio["id"], "cantidad_solicitada": "1", "costo_unitario": "100" }] }),
        )
        .await;
    assert_eq!(status, 400, "{body}");
}

#[tokio::test]
async fn recibir_completo_crea_una_compra_real_y_mueve_inventario() {
    let session = register_tenant().await;
    let proveedor = create_proveedor(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();

    let orden = session
        .post(
            "/v1/ordenes-compra",
            json!({ "proveedor_id": proveedor["id"], "items": [{ "producto_id": producto_id, "cantidad_solicitada": "5", "costo_unitario": "80" }] }),
        )
        .await;
    let orden_id = orden["id"].as_str().unwrap();
    let item_id = orden["items"][0]["id"].as_str().unwrap();

    let compra = session
        .post(
            &format!("/v1/ordenes-compra/{orden_id}/recibir"),
            json!({ "items": [{ "item_id": item_id, "cantidad": "5" }] }),
        )
        .await;
    assert_decimal_eq(decimal_field(&compra, "subtotal"), dec!(400.00), "subtotal de la compra real (5 x 80)");

    let orden_actualizada = session.get(&format!("/v1/ordenes-compra/{orden_id}")).await;
    assert_eq!(orden_actualizada["estado"], "RECIBIDA");

    let producto_actualizado = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&producto_actualizado, "stock_actual"), dec!(15.00), "stock inicial 10 + 5 recibidas");
}

#[tokio::test]
async fn recibir_parcialmente_dos_veces_completa_la_orden_sin_exceder_lo_solicitado() {
    let session = register_tenant().await;
    let proveedor = create_proveedor(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();

    let orden = session
        .post(
            "/v1/ordenes-compra",
            json!({ "proveedor_id": proveedor["id"], "items": [{ "producto_id": producto_id, "cantidad_solicitada": "10", "costo_unitario": "50" }] }),
        )
        .await;
    let orden_id = orden["id"].as_str().unwrap();
    let item_id = orden["items"][0]["id"].as_str().unwrap();

    session
        .post(&format!("/v1/ordenes-compra/{orden_id}/recibir"), json!({ "items": [{ "item_id": item_id, "cantidad": "6" }] }))
        .await;
    let orden_parcial = session.get(&format!("/v1/ordenes-compra/{orden_id}")).await;
    assert_eq!(orden_parcial["estado"], "RECIBIDA_PARCIAL");

    // Intentar recibir más de lo que queda pendiente (6 de 10 ya recibidas,
    // quedan 4) debe rechazarse.
    let (status, _) = session
        .post_expect(&format!("/v1/ordenes-compra/{orden_id}/recibir"), json!({ "items": [{ "item_id": item_id, "cantidad": "5" }] }))
        .await;
    assert_eq!(status, 400);

    session
        .post(&format!("/v1/ordenes-compra/{orden_id}/recibir"), json!({ "items": [{ "item_id": item_id, "cantidad": "4" }] }))
        .await;
    let orden_completa = session.get(&format!("/v1/ordenes-compra/{orden_id}")).await;
    assert_eq!(orden_completa["estado"], "RECIBIDA");

    let producto_actualizado = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&producto_actualizado, "stock_actual"), dec!(10.00), "0 inicial + 6 + 4 recibidas");
}

#[tokio::test]
async fn cancelar_una_orden_ya_recibida_es_rechazado() {
    let session = register_tenant().await;
    let proveedor = create_proveedor(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(0)).await;

    let orden = session
        .post(
            "/v1/ordenes-compra",
            json!({ "proveedor_id": proveedor["id"], "items": [{ "producto_id": producto["id"], "cantidad_solicitada": "1", "costo_unitario": "50" }] }),
        )
        .await;
    let orden_id = orden["id"].as_str().unwrap();
    let item_id = orden["items"][0]["id"].as_str().unwrap();

    session
        .post(&format!("/v1/ordenes-compra/{orden_id}/recibir"), json!({ "items": [{ "item_id": item_id, "cantidad": "1" }] }))
        .await;

    let (status, _) = session.post_expect(&format!("/v1/ordenes-compra/{orden_id}/cancelar"), json!({})).await;
    assert_eq!(status, 400, "una orden ya RECIBIDA no debería poder cancelarse");
}

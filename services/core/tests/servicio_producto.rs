//! Catálogo tipo SERVICIO (sin precio fijo, sin stock) - ver
//! catalog_service::Producto::tipo. No es tenant-gated: cualquier tenant
//! puede tener productos y servicios mezclados en el mismo catálogo.

mod common;

use common::*;
use rust_decimal_macros::dec;

#[tokio::test]
async fn servicio_producto_has_no_price_and_rejects_an_explicit_one() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    assert_eq!(servicio["tipo"], "SERVICIO");
    assert!(servicio["precio_venta"].is_null(), "a SERVICIO producto should have no precio_venta: {servicio}");

    let (status, body) = session
        .post_expect(
            "/v1/productos",
            serde_json::json!({
                "sku": format!("SRV-{}", uuid::Uuid::new_v4()),
                "nombre": "Servicio con precio inválido",
                "itbis_tipo": "GRAVADO_18",
                "tipo": "SERVICIO",
                "precio_venta": "100.00",
            }),
        )
        .await;
    assert!(status.is_success(), "creating a SERVICIO with a precio_venta should still succeed (server ignores it), got {status}: {body}");
    assert!(body["precio_venta"].is_null(), "server must null out precio_venta for SERVICIO regardless of client input: {body}");
}

#[tokio::test]
async fn venta_with_servicio_line_uses_manual_price_and_moves_no_stock() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;

    let producto = create_producto(&session, dec!(50.00), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap();
    let servicio = create_servicio(&session).await;
    let servicio_id = servicio["id"].as_str().unwrap();

    let venta = session
        .post(
            "/v1/ventas",
            serde_json::json!({
                "items": [
                    { "producto_id": producto_id, "cantidad": "2" },
                    { "producto_id": servicio_id, "cantidad": "1", "precio_unitario": "1000.00" },
                ],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    // producto: 50*2=100 + 18% = 118.00; servicio: 1000 + 18% = 1180.00 => total 1298.00
    assert_decimal_eq(decimal_field(&venta, "total"), dec!(1298.00), "venta.total (producto + servicio)");

    // El producto normal sí movió stock (10 -> 8); el servicio no dejó kardex.
    let productos_actualizado = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&productos_actualizado, "stock_actual"), dec!(8), "producto.stock_actual after sale");

    let movimientos = session.get(&format!("/v1/inventario/movimientos?productoId={servicio_id}&pageSize=50")).await;
    let items = movimientos["items"].as_array().expect("movimientos.items");
    assert!(items.is_empty(), "a SERVICIO line must never create a movimientos_inventario row: {items:?}");
}

#[tokio::test]
async fn venta_with_servicio_line_and_no_precio_unitario_is_rejected() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let servicio = create_servicio(&session).await;
    let servicio_id = servicio["id"].as_str().unwrap();

    let (status, body) = session
        .post_expect(
            "/v1/ventas",
            serde_json::json!({
                "items": [{ "producto_id": servicio_id, "cantidad": "1" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    assert!(status.is_client_error(), "a SERVICIO line without precio_unitario should be rejected, got {status}: {body}");
}

#[tokio::test]
async fn compra_referencing_a_servicio_producto_is_rejected() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let servicio_id = servicio["id"].as_str().unwrap();

    let (status, body) = session
        .post_expect(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": servicio_id, "cantidad": "1", "costo_unitario": "10.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    assert!(status.is_client_error(), "buying a SERVICIO producto should be rejected, got {status}: {body}");
}

//! Cotizaciones - Módulo 5b. Cubre el requisito de precio manual en toda
//! línea (PRODUCTO incluido, no solo SERVICIO - ver
//! docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md)
//! y que la descripción de línea sobrevive a la conversión en venta.

mod common;
use common::*;
use rust_decimal_macros::dec;
use serde_json::json;

#[tokio::test]
async fn cotizacion_requiere_precio_unitario_para_un_producto_normal() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;

    let (status, body) = session
        .post_expect(
            "/v1/cotizaciones",
            json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1" }] }),
        )
        .await;
    assert_eq!(status, 400, "un PRODUCTO sin precio_unitario manual debe rechazarse: {body}");
}

#[tokio::test]
async fn cotizacion_usa_precio_manual_para_producto_y_servicio_ignorando_el_catalogo() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;
    let servicio = create_servicio(&session).await;

    let cotizacion = session
        .post(
            "/v1/cotizaciones",
            json!({ "items": [
                { "producto_id": producto["id"], "cantidad": "2", "precio_unitario": "300.00", "descripcion": "2 habitaciones" },
                { "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "1500.00" }
            ] }),
        )
        .await;

    // Precio manual (300, no los 200 del catálogo) * 2 = 600; + servicio 1500 => 2100 subtotal, 18% = 378 itbis.
    assert_decimal_eq(decimal_field(&cotizacion, "subtotal"), dec!(2100.00), "subtotal usa el precio manual, no el de catálogo");
    assert_decimal_eq(decimal_field(&cotizacion, "itbis_total"), dec!(378.00), "itbis");
    let items = cotizacion["items"].as_array().unwrap();
    let linea_producto = items.iter().find(|i| i["producto_id"] == producto["id"]).unwrap();
    assert_eq!(linea_producto["descripcion"], "2 habitaciones");
}

#[tokio::test]
async fn convertir_cotizacion_a_venta_conserva_la_descripcion_de_linea() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;

    let cotizacion = session
        .post(
            "/v1/cotizaciones",
            json!({ "items": [{ "producto_id": producto["id"], "cantidad": "1", "precio_unitario": "250.00", "descripcion": "Urgente, entregar hoy" }] }),
        )
        .await;
    let cot_id = cotizacion["id"].as_str().unwrap();

    let venta = session.post(&format!("/v1/cotizaciones/{cot_id}/convertir"), json!({})).await;
    let items = venta["items"].as_array().unwrap();
    assert_eq!(items[0]["descripcion"], "Urgente, entregar hoy", "la descripción de la cotización debe viajar a la venta: {venta}");
}

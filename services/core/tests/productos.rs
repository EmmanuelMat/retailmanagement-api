//! Catálogo de productos/servicios - sugerencia de código (ver
//! docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md).

mod common;
use common::*;

#[tokio::test]
async fn sugerir_codigo_deriva_prefijo_de_las_primeras_3_letras_sin_acentos() {
    let session = register_tenant().await;
    let sugerido = session.get("/v1/productos/sugerir-codigo?nombre=Fumigaci%C3%B3n").await;
    assert_eq!(sugerido["sku"], "FUM-0001");
}

#[tokio::test]
async fn sugerir_codigo_avanza_la_secuencia_por_prefijo() {
    let session = register_tenant().await;
    session
        .post(
            "/v1/productos",
            serde_json::json!({ "sku": "FUM-0001", "nombre": "Fumigación básica", "itbis_tipo": "GRAVADO_18", "tipo": "SERVICIO" }),
        )
        .await;

    let sugerido = session.get("/v1/productos/sugerir-codigo?nombre=Fumigaci%C3%B3n%20Residencial").await;
    assert_eq!(sugerido["sku"], "FUM-0002", "debe continuar la secuencia existente para el mismo prefijo");
}

#[tokio::test]
async fn sugerir_codigo_rechaza_un_nombre_sin_suficientes_letras() {
    let session = register_tenant().await;
    let (status, body) = session.get_expect("/v1/productos/sugerir-codigo?nombre=A1").await;
    assert_eq!(status, 400, "{body}");
}

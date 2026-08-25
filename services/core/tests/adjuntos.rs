//! Adjuntos genéricos - Módulo 15c. Sube/lista/borra un archivo contra una
//! Orden de Servicio real, usando multipart (no hay helper JSON para esto en
//! tests/common/mod.rs porque ningún otro endpoint del suite necesita
//! subir archivos). Ver adjunto_service.rs.

mod common;
use common::*;
use serde_json::json;

#[tokio::test]
async fn subir_listar_y_borrar_un_adjunto_de_una_orden_de_servicio() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let orden = session
        .post("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }] }))
        .await;
    let orden_id = orden["id"].as_str().unwrap().to_string();

    let form = reqwest::multipart::Form::new()
        .text("entidad_tipo", "ORDEN_SERVICIO")
        .text("entidad_id", orden_id.clone())
        .part("archivo", reqwest::multipart::Part::bytes(b"contenido de prueba".to_vec()).file_name("foto-antes.txt").mime_str("text/plain").unwrap());

    let resp = session
        .client
        .post(format!("{}/v1/adjuntos", base_url()))
        .bearer_auth(&session.token)
        .multipart(form)
        .send()
        .await
        .expect("upload failed");
    assert!(resp.status().is_success(), "{}", resp.text().await.unwrap_or_default());
    let adjunto: serde_json::Value = resp.json().await.expect("upload response not JSON");
    assert_eq!(adjunto["entidad_tipo"], "ORDEN_SERVICIO");
    assert_eq!(adjunto["entidad_id"], orden_id);
    assert_eq!(adjunto["nombre_archivo"], "foto-antes.txt");
    let adjunto_id = adjunto["id"].as_str().unwrap().to_string();

    let listado = session.get(&format!("/v1/adjuntos?entidadTipo=ORDEN_SERVICIO&entidadId={orden_id}")).await;
    assert_eq!(listado.as_array().unwrap().len(), 1);

    // Descarga real del archivo servido vía /uploads (ServeDir montado en main.rs).
    let storage_path = adjunto["storage_path"].as_str().unwrap();
    let descarga = session.client.get(format!("{}{}", base_url(), storage_path)).send().await.expect("download failed");
    assert!(descarga.status().is_success());
    let bytes = descarga.bytes().await.unwrap();
    assert_eq!(bytes.as_ref(), b"contenido de prueba");

    let resp = session
        .client
        .delete(format!("{}/v1/adjuntos/{adjunto_id}", base_url()))
        .bearer_auth(&session.token)
        .send()
        .await
        .expect("delete failed");
    assert!(resp.status().is_success());

    let listado_tras_borrar = session.get(&format!("/v1/adjuntos?entidadTipo=ORDEN_SERVICIO&entidadId={orden_id}")).await;
    assert_eq!(listado_tras_borrar.as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn entidad_tipo_invalido_es_rechazado() {
    let session = register_tenant().await;
    let form = reqwest::multipart::Form::new()
        .text("entidad_tipo", "NO_EXISTE")
        .text("entidad_id", uuid::Uuid::new_v4().to_string())
        .part("archivo", reqwest::multipart::Part::bytes(b"x".to_vec()).file_name("a.txt"));
    let resp = session
        .client
        .post(format!("{}/v1/adjuntos", base_url()))
        .bearer_auth(&session.token)
        .multipart(form)
        .send()
        .await
        .expect("upload failed");
    assert_eq!(resp.status(), 400);
}


//! Órdenes de Servicio (work orders) - Módulo 15. Cubre el ciclo de vida
//! completo (crear -> asignar técnico -> materiales -> iniciar/pausar/
//! completar -> facturar), la conversión desde Cotización, el guard
//! anti-doble-descuento entre líneas facturadas y materiales consumidos,
//! aislamiento de tenant, y el permiso_guard aditivo del módulo nuevo. Ver
//! tests/common/mod.rs para el harness compartido.

mod common;
use common::*;
use rust_decimal_macros::dec;
use serde_json::json;

async fn create_cliente(session: &TenantSession) -> serde_json::Value {
    session.post("/v1/clientes", json!({ "nombre": "Cliente de prueba e2e" })).await
}

async fn create_empleado_named(session: &TenantSession, nombre: &str) -> serde_json::Value {
    session.post("/v1/empleados", json!({ "nombre": nombre, "salario_mensual": "30000" })).await
}

#[tokio::test]
async fn crear_orden_con_servicio_y_producto_calcula_totales_correctamente() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;

    let orden = session
        .post(
            "/v1/ordenes-servicio",
            json!({
                "items": [
                    { "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "1500" },
                    { "producto_id": producto["id"], "cantidad": "2" }
                ]
            }),
        )
        .await;

    assert_eq!(orden["estado"], "BORRADOR");
    assert_decimal_eq(decimal_field(&orden, "subtotal"), dec!(1900.00), "subtotal");
    assert_decimal_eq(decimal_field(&orden, "itbis_total"), dec!(342.00), "itbis");
    assert_decimal_eq(decimal_field(&orden, "total"), dec!(2242.00), "total");
    assert_eq!(orden["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn servicio_sin_precio_unitario_es_rechazado() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let (status, body) = session
        .post_expect("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1" }] }))
        .await;
    assert_eq!(status, 400, "{body}");
}

#[tokio::test]
async fn transiciones_de_estado_invalidas_son_rechazadas() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let orden = session
        .post("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }] }))
        .await;
    let id = orden["id"].as_str().unwrap();

    // BORRADOR -> COMPLETADA directo: inválido.
    let (status, _) = session.post_expect(&format!("/v1/ordenes-servicio/{id}/completar"), json!({})).await;
    assert_eq!(status, 400);

    // BORRADOR -> PAUSADA directo: inválido (solo desde EN_PROCESO).
    let (status, _) = session.post_expect(&format!("/v1/ordenes-servicio/{id}/pausar"), json!({})).await;
    assert_eq!(status, 400);

    let iniciada = session.post(&format!("/v1/ordenes-servicio/{id}/iniciar"), json!({})).await;
    assert_eq!(iniciada["estado"], "EN_PROCESO");

    let completada = session.post(&format!("/v1/ordenes-servicio/{id}/completar"), json!({})).await;
    assert_eq!(completada["estado"], "COMPLETADA");

    // COMPLETADA -> cualquier transición: inválido, la orden es terminal.
    let (status, _) = session.post_expect(&format!("/v1/ordenes-servicio/{id}/iniciar"), json!({})).await;
    assert_eq!(status, 400);
    let (status, _) = session.post_expect(&format!("/v1/ordenes-servicio/{id}/cancelar"), json!({})).await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn asignar_tecnico_y_agregar_nota() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let empleado = create_empleado_named(&session, "Carlos Técnico").await;
    let orden = session
        .post("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }] }))
        .await;
    let id = orden["id"].as_str().unwrap();

    let asignacion = session
        .post(&format!("/v1/ordenes-servicio/{id}/tecnicos"), json!({ "empleado_id": empleado["id"], "rol": "TECNICO_PRINCIPAL" }))
        .await;
    assert_eq!(asignacion["empleado_id"], empleado["id"]);
    assert_eq!(asignacion["rol"], "TECNICO_PRINCIPAL");

    let nota = session
        .post(&format!("/v1/ordenes-servicio/{id}/notas"), json!({ "tipo": "TECNICO", "contenido": "Revisar antes de iniciar" }))
        .await;
    assert_eq!(nota["tipo"], "TECNICO");

    let completa = session.get(&format!("/v1/ordenes-servicio/{id}")).await;
    assert_eq!(completa["tecnicos"].as_array().unwrap().len(), 1);
    assert_eq!(completa["notas_registro"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn consumir_material_mueve_inventario_real_una_sola_vez() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let material = create_producto(&session, dec!(40), dec!(30)).await;
    let orden = session
        .post("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }] }))
        .await;
    let orden_id = orden["id"].as_str().unwrap();

    let mat = session
        .post(&format!("/v1/ordenes-servicio/{orden_id}/materiales"), json!({ "producto_id": material["id"], "cantidad_planificada": "3" }))
        .await;
    let material_id = mat["id"].as_str().unwrap();
    assert_decimal_eq(decimal_field(&mat, "cantidad_planificada"), dec!(3.00), "planificada");
    assert_decimal_eq(decimal_field(&mat, "cantidad_utilizada"), dec!(0), "utilizada inicial");

    session.post(&format!("/v1/ordenes-servicio/{orden_id}/iniciar"), json!({})).await;
    let consumido = session
        .post(&format!("/v1/ordenes-servicio/{orden_id}/materiales/{material_id}/consumir"), json!({ "cantidad": "2" }))
        .await;
    assert_decimal_eq(decimal_field(&consumido, "cantidad_utilizada"), dec!(2.00), "utilizada tras consumo");

    let producto_actualizado = session.get(&format!("/v1/productos/{}", material["id"].as_str().unwrap())).await;
    assert_decimal_eq(decimal_field(&producto_actualizado, "stock_actual"), dec!(28.00), "stock tras un único consumo de 2");
}

#[tokio::test]
async fn no_se_puede_facturar_y_consumir_material_el_mismo_producto_en_la_misma_orden() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;
    let orden = session
        .post(
            "/v1/ordenes-servicio",
            json!({ "items": [
                { "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" },
                { "producto_id": producto["id"], "cantidad": "1" }
            ] }),
        )
        .await;
    let orden_id = orden["id"].as_str().unwrap();

    // El producto ya está facturado como línea - registrarlo también como
    // material debe rechazarse (evita descontar el stock dos veces).
    let (status, body) = session
        .post_expect(&format!("/v1/ordenes-servicio/{orden_id}/materiales"), json!({ "producto_id": producto["id"], "cantidad_planificada": "1" }))
        .await;
    assert_eq!(status, 400, "{body}");

    // Y en la otra dirección: un producto ya anotado como material no puede
    // agregarse también como línea facturable.
    let material_libre = create_producto(&session, dec!(80), dec!(20)).await;
    session
        .post(&format!("/v1/ordenes-servicio/{orden_id}/materiales"), json!({ "producto_id": material_libre["id"], "cantidad_planificada": "1" }))
        .await;
    let (status, body) = session
        .post_expect(&format!("/v1/ordenes-servicio/{orden_id}/items"), json!({ "producto_id": material_libre["id"], "cantidad": "1" }))
        .await;
    assert_eq!(status, 400, "{body}");
}

#[tokio::test]
async fn orden_completada_se_factura_como_venta_real_y_no_se_puede_facturar_dos_veces() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let producto = create_producto(&session, dec!(200), dec!(50)).await;
    let orden = session
        .post(
            "/v1/ordenes-servicio",
            json!({ "items": [
                { "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "1500" },
                { "producto_id": producto["id"], "cantidad": "2" }
            ] }),
        )
        .await;
    let orden_id = orden["id"].as_str().unwrap();

    // No se puede facturar antes de completar.
    let (status, _) = session.post_expect(&format!("/v1/ordenes-servicio/{orden_id}/crear-factura"), json!({})).await;
    assert_eq!(status, 400);

    session.post(&format!("/v1/ordenes-servicio/{orden_id}/iniciar"), json!({})).await;
    session.post(&format!("/v1/ordenes-servicio/{orden_id}/completar"), json!({})).await;
    abrir_caja(&session, dec!(1000)).await;

    let venta = session.post(&format!("/v1/ordenes-servicio/{orden_id}/crear-factura"), json!({})).await;
    assert_decimal_eq(decimal_field(&venta, "total"), dec!(2242.00), "total de la venta generada");
    assert_eq!(venta["items"].as_array().unwrap().len(), 2);

    let orden_actualizada = session.get(&format!("/v1/ordenes-servicio/{orden_id}")).await;
    assert_eq!(orden_actualizada["venta_id"], venta["id"]);

    // Ya facturada: un segundo intento debe rechazarse.
    let (status, body) = session.post_expect(&format!("/v1/ordenes-servicio/{orden_id}/crear-factura"), json!({})).await;
    assert_eq!(status, 400, "{body}");
}

#[tokio::test]
async fn cotizacion_se_convierte_en_orden_de_servicio_reusando_los_precios_ya_fijados() {
    let session = register_tenant().await;
    let cliente = create_cliente(&session).await;
    let servicio = create_servicio(&session).await;

    let cotizacion = session
        .post(
            "/v1/cotizaciones",
            json!({ "cliente_id": cliente["id"], "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "2000" }] }),
        )
        .await;
    let cot_id = cotizacion["id"].as_str().unwrap();

    let orden = session.post(&format!("/v1/cotizaciones/{cot_id}/convertir-a-orden"), json!({})).await;
    assert_eq!(orden["cliente_id"], cliente["id"]);
    assert_eq!(orden["cotizacion_id"], cot_id);
    assert_decimal_eq(decimal_field(&orden, "subtotal"), dec!(2000.00), "subtotal reusa el precio de la cotización");

    let cotizacion_actualizada = session.get(&format!("/v1/cotizaciones/{cot_id}")).await;
    assert_eq!(cotizacion_actualizada["estado"], "CONVERTIDA");
    assert!(cotizacion_actualizada["venta_id"].is_null(), "no hay venta todavía - solo una orden de servicio");

    // Una cotización ya convertida no puede volver a convertirse.
    let (status, _) = session.post_expect(&format!("/v1/cotizaciones/{cot_id}/convertir-a-orden"), json!({})).await;
    assert_eq!(status, 400);
}

#[tokio::test]
async fn una_orden_de_servicio_no_es_visible_para_otro_tenant() {
    let session_a = register_tenant().await;
    let session_b = register_tenant().await;
    let servicio = create_servicio(&session_a).await;
    let orden = session_a
        .post("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }] }))
        .await;
    let orden_id = orden["id"].as_str().unwrap();

    let (status, _) = session_b.post_expect(&format!("/v1/ordenes-servicio/{orden_id}/iniciar"), json!({})).await;
    assert_eq!(status, 400, "otro tenant no debería poder ver ni transicionar esta orden");

    let resp = session_b
        .client
        .get(format!("{}/v1/ordenes-servicio/{orden_id}", base_url()))
        .bearer_auth(&session_b.token)
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), 404, "otro tenant no debería poder leer esta orden");
}

/// Las rutas de Órdenes de Servicio pasan por el mismo `permission_guard`
/// que el resto de la app (ver `required_permiso` en main.rs), gateadas por
/// un único permiso de grano grueso `ordenes_servicio.gestionar` - no hay
/// sub-permisos por acción de lifecycle (iniciar/pausar/consumir material),
/// igual que `ventas.gestionar` cubre toda la operación de un cajero.
/// CAJERO tiene ese permiso (ver seed en migrate.rs); CONTADOR no - confirma
/// que el guard realmente bloquea, no solo que un rol entero pasa por
/// `required_roles_legacy` sin más chequeo.
#[tokio::test]
async fn rol_sin_el_permiso_ordenes_servicio_gestionar_es_rechazado() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let orden = session
        .post("/v1/ordenes-servicio", json!({ "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }] }))
        .await;
    let orden_id = orden["id"].as_str().unwrap();

    let nuevo_usuario = session
        .post(
            "/v1/config/usuarios",
            json!({ "nombre": "Contador Sin Permiso", "email": format!("contador-{}@e2e-test.local", session.rnc), "password": "TestPassword123!", "rol": "CONTADOR" }),
        )
        .await;

    let login = session
        .client
        .post(format!("{}/v1/auth/login", base_url()))
        .json(&json!({ "email": nuevo_usuario["email"], "password": "TestPassword123!", "rnc": session.rnc }))
        .send()
        .await
        .expect("login failed");
    assert!(login.status().is_success());
    let login_body: serde_json::Value = login.json().await.expect("login response not JSON");
    let contador_token = login_body["token"].as_str().expect("no token").to_string();

    // CONTADOR no tiene ordenes_servicio.gestionar - el permission_guard
    // debe rechazarlo con 403 (y, al ser un permiso nuevo sin equivalente
    // legacy, sin caer al fallback de required_roles_legacy).
    let resp = session
        .client
        .post(format!("{}/v1/ordenes-servicio/{orden_id}/iniciar", base_url()))
        .bearer_auth(&contador_token)
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), 403, "CONTADOR no tiene ordenes_servicio.gestionar - el permission_guard debe rechazarlo");
}

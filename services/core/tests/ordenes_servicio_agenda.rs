//! Fase E - Programación de técnicos: detección de conflictos de agenda al
//! asignar un técnico o al reprogramar una orden, y el endpoint de agenda
//! (`GET /v1/ordenes-servicio/agenda`).
//!
//! Reglas que este binario fija (ver
//! docs/superpowers/specs/2026-09-21-phase-e-scheduling-design.md):
//!   * Un técnico está ocupado por una orden que tiene `fecha_programada` Y
//!     ambas horas, y cuyo estado no es CANCELADA ni COMPLETADA.
//!   * Solapamiento = intervalo semiabierto `[hora_inicio, hora_fin)`:
//!     adyacente (una termina cuando la otra empieza) NO es conflicto.
//!   * Horas NULL: nunca producen un 409, solo un `avisos[]` blando de
//!     "ese técnico ya tiene trabajo ese día".
//!   * `confirmar_conflicto: true` fuerza la escritura y deja rastro en
//!     `auditoria` (regla blanda a propósito: dos trabajos cortos en el mismo
//!     bloque son una decisión legítima del despachador).
//!
//! Ver tests/common/mod.rs para el harness compartido.

mod common;
use common::*;
use rust_decimal_macros::dec;
use serde_json::{json, Value};

const DIA: &str = "2026-11-10";
const OTRO_DIA: &str = "2026-11-11";

/// PATCH no existe en el harness compartido (`TenantSession` solo trae
/// get/post) y `tests/common/mod.rs` lo comparten los demás binarios de la
/// suite - se hace acá, local a este archivo, en vez de tocarlo.
async fn patch_expect(session: &TenantSession, path: &str, body: Value) -> (reqwest::StatusCode, Value) {
    let resp = session
        .client
        .patch(format!("{}{}", base_url(), path))
        .bearer_auth(&session.token)
        .json(&body)
        .send()
        .await
        .unwrap_or_else(|e| panic!("PATCH {path} failed: {e}"));
    let status = resp.status();
    let text = resp.text().await.unwrap_or_else(|e| panic!("PATCH {path} body unreadable: {e}"));
    let payload = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, payload)
}

/// Igual que `TenantSession::get` pero sin `assert!(success)`, para las
/// rutas que este binario necesita ver fallar (rango inválido en la agenda).
async fn get_expect(session: &TenantSession, path: &str) -> (reqwest::StatusCode, Value) {
    let resp = session
        .client
        .get(format!("{}{}", base_url(), path))
        .bearer_auth(&session.token)
        .send()
        .await
        .unwrap_or_else(|e| panic!("GET {path} failed: {e}"));
    let status = resp.status();
    let text = resp.text().await.unwrap_or_else(|e| panic!("GET {path} body unreadable: {e}"));
    let payload = serde_json::from_str(&text).unwrap_or(Value::String(text));
    (status, payload)
}

async fn create_empleado_named(session: &TenantSession, nombre: &str) -> Value {
    session.post("/v1/empleados", json!({ "nombre": nombre, "salario_mensual": "30000" })).await
}

/// Una orden de una sola línea de servicio, opcionalmente ya programada.
async fn crear_orden(session: &TenantSession, servicio_id: &Value, fecha: Option<&str>, desde: Option<&str>, hasta: Option<&str>) -> Value {
    let mut body = json!({ "items": [{ "producto_id": servicio_id, "cantidad": "1", "precio_unitario": "500" }] });
    if let Some(f) = fecha {
        body["fecha_programada"] = json!(f);
    }
    if let Some(h) = desde {
        body["hora_inicio"] = json!(h);
    }
    if let Some(h) = hasta {
        body["hora_fin"] = json!(h);
    }
    session.post("/v1/ordenes-servicio", body).await
}

async fn asignar(session: &TenantSession, orden_id: &str, empleado_id: &Value, rol: &str) -> (reqwest::StatusCode, Value) {
    session
        .post_expect(&format!("/v1/ordenes-servicio/{orden_id}/tecnicos"), json!({ "empleado_id": empleado_id, "rol": rol }))
        .await
}

fn id_of(v: &Value) -> String {
    v["id"].as_str().expect("sin id").to_string()
}

/// El código legible que la UI ya usa para una orden (ver
/// `ordenes-servicio/[id]/page.tsx`): `OS-` + los primeros 8 del UUID.
fn codigo_de(orden: &Value) -> String {
    format!("OS-{}", id_of(orden)[..8].to_uppercase())
}

// ---------------------------------------------------------------------------
// E1 - detección de conflictos al asignar un técnico
// ---------------------------------------------------------------------------

#[tokio::test]
async fn solapar_al_mismo_tecnico_el_mismo_dia_es_rechazado_con_409_y_detalle() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Juan Técnico").await;

    let ocupada = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    let (status, _) = asignar(&session, &id_of(&ocupada), &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "la primera asignación no tiene con qué chocar");

    // 10:00-12:00 pisa 09:00-11:00.
    let nueva = crear_orden(&session, &servicio["id"], Some(DIA), Some("10:00"), Some("12:00")).await;
    let (status, body) = asignar(&session, &id_of(&nueva), &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert_eq!(status, 409, "un solapamiento real debe ser 409, no 400: {body}");

    let conflictos = body["conflictos"].as_array().expect("el 409 debe traer conflictos[]");
    assert_eq!(conflictos.len(), 1, "{body}");
    let c = &conflictos[0];
    assert_eq!(c["orden_id"], ocupada["id"], "debe señalar la orden que ya ocupaba el bloque");
    assert_eq!(c["codigo"], codigo_de(&ocupada), "el código legible, no solo el UUID");
    assert_eq!(c["fecha_programada"], DIA);
    assert_eq!(c["hora_inicio"], "09:00:00");
    assert_eq!(c["hora_fin"], "11:00:00");
    assert_eq!(c["empleado_id"], tecnico["id"]);
    assert_eq!(c["empleado_nombre"], "Juan Técnico");

    // El mensaje es para un usuario dominicano, en español.
    let mensaje = body["error"].as_str().unwrap_or_default();
    assert!(mensaje.to_lowercase().contains("técnico") || mensaje.to_lowercase().contains("tecnico"), "mensaje: {mensaje}");

    // Y nada se escribió: la orden nueva sigue sin técnicos.
    let detalle = session.get(&format!("/v1/ordenes-servicio/{}", id_of(&nueva))).await;
    assert_eq!(detalle["tecnicos"].as_array().unwrap().len(), 0, "un 409 no debe dejar la asignación a medias");
}

#[tokio::test]
async fn rangos_adyacentes_no_son_conflicto() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Pedro Técnico").await;

    let manana = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session, &id_of(&manana), &tecnico["id"], "TECNICO_PRINCIPAL").await;

    // Empieza exactamente cuando la otra termina: `[09:00,11:00)` y
    // `[11:00,13:00)` no se tocan.
    let tarde = crear_orden(&session, &servicio["id"], Some(DIA), Some("11:00"), Some("13:00")).await;
    let (status, body) = asignar(&session, &id_of(&tarde), &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "adyacente no es solapamiento: {body}");
    assert_eq!(body["avisos"].as_array().map(|a| a.len()).unwrap_or(0), 0, "tampoco debe haber aviso blando: {body}");
}

#[tokio::test]
async fn el_mismo_bloque_en_otro_dia_no_es_conflicto() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Luis Técnico").await;

    let hoy = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session, &id_of(&hoy), &tecnico["id"], "TECNICO_PRINCIPAL").await;

    let manana = crear_orden(&session, &servicio["id"], Some(OTRO_DIA), Some("09:00"), Some("11:00")).await;
    let (status, body) = asignar(&session, &id_of(&manana), &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "otro día no compite por el mismo bloque: {body}");
}

#[tokio::test]
async fn ordenes_canceladas_o_completadas_no_bloquean_el_bloque() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let cancelada_tec = create_empleado_named(&session, "Técnico A").await;
    let completada_tec = create_empleado_named(&session, "Técnico B").await;

    // Una orden CANCELADA libera su bloque.
    let cancelada = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session, &id_of(&cancelada), &cancelada_tec["id"], "TECNICO_PRINCIPAL").await;
    session.post(&format!("/v1/ordenes-servicio/{}/cancelar", id_of(&cancelada)), json!({})).await;

    let nueva = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:30"), Some("10:30")).await;
    let (status, body) = asignar(&session, &id_of(&nueva), &cancelada_tec["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "una orden cancelada no ocupa a nadie: {body}");

    // Y una COMPLETADA tampoco (el trabajo ya se hizo).
    let completada = crear_orden(&session, &servicio["id"], Some(DIA), Some("14:00"), Some("16:00")).await;
    asignar(&session, &id_of(&completada), &completada_tec["id"], "TECNICO_PRINCIPAL").await;
    session.post(&format!("/v1/ordenes-servicio/{}/iniciar", id_of(&completada)), json!({})).await;
    session.post(&format!("/v1/ordenes-servicio/{}/completar", id_of(&completada)), json!({})).await;

    let otra = crear_orden(&session, &servicio["id"], Some(DIA), Some("14:30"), Some("15:30")).await;
    let (status, body) = asignar(&session, &id_of(&otra), &completada_tec["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "una orden completada no ocupa a nadie: {body}");
}

#[tokio::test]
async fn el_override_confirmar_conflicto_asigna_de_todos_modos_y_queda_auditado() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Ramón Técnico").await;

    let ocupada = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session, &id_of(&ocupada), &tecnico["id"], "TECNICO_PRINCIPAL").await;

    let nueva = crear_orden(&session, &servicio["id"], Some(DIA), Some("10:00"), Some("12:00")).await;
    let nueva_id = id_of(&nueva);
    let (status, _) = asignar(&session, &nueva_id, &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert_eq!(status, 409, "sin el flag, choca");

    // Con el flag explícito, la regla blanda cede.
    let (status, body) = session
        .post_expect(
            &format!("/v1/ordenes-servicio/{nueva_id}/tecnicos"),
            json!({ "empleado_id": tecnico["id"], "rol": "TECNICO_PRINCIPAL", "confirmar_conflicto": true }),
        )
        .await;
    assert!(status.is_success(), "con confirmar_conflicto debe pasar: {body}");
    assert_eq!(body["empleado_id"], tecnico["id"]);

    let detalle = session.get(&format!("/v1/ordenes-servicio/{nueva_id}")).await;
    assert_eq!(detalle["tecnicos"].as_array().unwrap().len(), 1);

    // Cada override deja rastro: un despachador no puede saltarse la agenda
    // en silencio.
    let auditoria = session
        .get(&format!("/v1/auditoria?entidad=orden_servicio&entidadId={nueva_id}&pageSize=50"))
        .await;
    let acciones: Vec<&str> = auditoria["items"].as_array().unwrap().iter().filter_map(|e| e["accion"].as_str()).collect();
    assert!(
        acciones.contains(&"ORDEN_SERVICIO_CONFLICTO_AGENDA_OMITIDO"),
        "el override debe auditarse; acciones vistas: {acciones:?}"
    );
    let entrada = auditoria["items"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["accion"] == "ORDEN_SERVICIO_CONFLICTO_AGENDA_OMITIDO")
        .unwrap();
    let auditados = entrada["detalle"]["conflictos"].as_array().expect("el detalle debe nombrar los conflictos omitidos");
    assert_eq!(auditados.len(), 1);
    assert_eq!(auditados[0]["orden_id"], ocupada["id"]);
}

#[tokio::test]
async fn sin_horas_nunca_hay_409_pero_si_un_aviso_blando_del_mismo_dia() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Técnico Sin Horas").await;

    // Orden solo con fecha: trabajo pendiente del día, no un bloque reservado.
    let solo_fecha = crear_orden(&session, &servicio["id"], Some(DIA), None, None).await;
    asignar(&session, &id_of(&solo_fecha), &tecnico["id"], "TECNICO_PRINCIPAL").await;

    // Otra orden con horas completas el mismo día: NO es conflicto duro,
    // pero el despachador debe enterarse.
    let con_horas = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    let (status, body) = asignar(&session, &id_of(&con_horas), &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "fecha sin horas no reserva bloque: {body}");
    let avisos = body["avisos"].as_array().expect("debe traer avisos[]");
    assert_eq!(avisos.len(), 1, "{body}");
    assert_eq!(avisos[0]["orden_id"], solo_fecha["id"]);
    assert!(avisos[0]["hora_inicio"].is_null(), "el aviso es justamente por no tener horas: {body}");

    // Y en la dirección contraria tampoco bloquea.
    let otra_sin_horas = crear_orden(&session, &servicio["id"], Some(DIA), None, None).await;
    let (status, body) = asignar(&session, &id_of(&otra_sin_horas), &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "dos órdenes sin horas el mismo día no chocan: {body}");
    assert_eq!(body["avisos"].as_array().unwrap().len(), 2, "pero sí avisan de las otras dos del día: {body}");
}

#[tokio::test]
async fn una_orden_sin_fecha_programada_nunca_produce_conflicto() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Técnico Libre").await;

    let programada = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session, &id_of(&programada), &tecnico["id"], "TECNICO_PRINCIPAL").await;

    let sin_fecha = crear_orden(&session, &servicio["id"], None, None, None).await;
    let (status, body) = asignar(&session, &id_of(&sin_fecha), &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "sin fecha no hay agenda contra la cual chocar: {body}");
    assert_eq!(body["avisos"].as_array().map(|a| a.len()).unwrap_or(0), 0, "{body}");
}

#[tokio::test]
async fn hora_fin_no_posterior_a_hora_inicio_es_rechazada() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;

    // Rango de duración cero: se rechaza en la escritura en vez de dejar
    // `[t,t)` (que no solaparía con nada) circulando por la agenda.
    let (status, body) = session
        .post_expect(
            "/v1/ordenes-servicio",
            json!({
                "fecha_programada": DIA, "hora_inicio": "09:00", "hora_fin": "09:00",
                "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }]
            }),
        )
        .await;
    assert_eq!(status, 400, "rango de duración cero: {body}");

    // Invertido (incluye el caso "trabajo que cruza la medianoche", que dos
    // columnas TIME del mismo día no pueden representar).
    let (status, body) = session
        .post_expect(
            "/v1/ordenes-servicio",
            json!({
                "fecha_programada": DIA, "hora_inicio": "22:00", "hora_fin": "02:00",
                "items": [{ "producto_id": servicio["id"], "cantidad": "1", "precio_unitario": "500" }]
            }),
        )
        .await;
    assert_eq!(status, 400, "rango invertido: {body}");

    // Y lo mismo por PATCH.
    let orden = crear_orden(&session, &servicio["id"], Some(DIA), None, None).await;
    let (status, body) = patch_expect(&session, &format!("/v1/ordenes-servicio/{}", id_of(&orden)), json!({ "hora_inicio": "15:00", "hora_fin": "14:00" })).await;
    assert_eq!(status, 400, "rango invertido por PATCH: {body}");
}

#[tokio::test]
async fn dos_tecnicos_en_la_misma_orden_no_chocan_entre_si() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let principal = create_empleado_named(&session, "Principal").await;
    let asistente = create_empleado_named(&session, "Asistente").await;

    let orden = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    let orden_id = id_of(&orden);

    let (status, body) = asignar(&session, &orden_id, &principal["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "{body}");
    // El ASISTENTE trabaja el mismo bloque, en la MISMA orden: eso es
    // colaboración, no doble reserva.
    let (status, body) = asignar(&session, &orden_id, &asistente["id"], "ASISTENTE").await;
    assert!(status.is_success(), "multi-técnico sigue funcionando: {body}");

    let detalle = session.get(&format!("/v1/ordenes-servicio/{orden_id}")).await;
    assert_eq!(detalle["tecnicos"].as_array().unwrap().len(), 2);

    // Pero el asistente sí choca con OTRA orden en el mismo bloque.
    let otra = crear_orden(&session, &servicio["id"], Some(DIA), Some("10:00"), Some("10:30")).await;
    let (status, _) = asignar(&session, &id_of(&otra), &asistente["id"], "ASISTENTE").await;
    assert_eq!(status, 409, "otra orden en el mismo bloque sí es doble reserva");
}

#[tokio::test]
async fn reprogramar_una_orden_detecta_el_conflicto_de_sus_tecnicos_ya_asignados() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Técnico Reprogramado").await;

    let fija = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session, &id_of(&fija), &tecnico["id"], "TECNICO_PRINCIPAL").await;

    // Esta arranca en otro día, así que asignar al mismo técnico es legal...
    let movible = crear_orden(&session, &servicio["id"], Some(OTRO_DIA), Some("09:00"), Some("11:00")).await;
    let movible_id = id_of(&movible);
    let (status, body) = asignar(&session, &movible_id, &tecnico["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "{body}");

    // ...pero moverla al día ocupado sí choca.
    let (status, body) = patch_expect(&session, &format!("/v1/ordenes-servicio/{movible_id}"), json!({ "fecha_programada": DIA })).await;
    assert_eq!(status, 409, "reprogramar sobre un bloque ocupado debe chocar igual que asignar: {body}");
    assert_eq!(body["conflictos"].as_array().unwrap()[0]["orden_id"], fija["id"]);

    // Y la fecha NO cambió.
    let detalle = session.get(&format!("/v1/ordenes-servicio/{movible_id}")).await;
    assert_eq!(detalle["fecha_programada"], OTRO_DIA, "un 409 no debe reprogramar a medias");

    // Con el override sí, y auditado.
    let (status, body) = patch_expect(&session, &format!("/v1/ordenes-servicio/{movible_id}"), json!({ "fecha_programada": DIA, "confirmar_conflicto": true })).await;
    assert!(status.is_success(), "{body}");
    assert_eq!(body["fecha_programada"], DIA);

    let auditoria = session
        .get(&format!("/v1/auditoria?entidad=orden_servicio&entidadId={movible_id}&pageSize=50"))
        .await;
    let acciones: Vec<&str> = auditoria["items"].as_array().unwrap().iter().filter_map(|e| e["accion"].as_str()).collect();
    assert!(acciones.contains(&"ORDEN_SERVICIO_CONFLICTO_AGENDA_OMITIDO"), "acciones: {acciones:?}");
}

#[tokio::test]
async fn reprogramar_a_un_hueco_libre_no_choca() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let tecnico = create_empleado_named(&session, "Técnico Con Hueco").await;

    let fija = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session, &id_of(&fija), &tecnico["id"], "TECNICO_PRINCIPAL").await;

    let movible = crear_orden(&session, &servicio["id"], Some(OTRO_DIA), Some("11:00"), Some("12:00")).await;
    let movible_id = id_of(&movible);
    asignar(&session, &movible_id, &tecnico["id"], "TECNICO_PRINCIPAL").await;

    // 11:00-12:00 arranca justo cuando la otra termina.
    let (status, body) = patch_expect(&session, &format!("/v1/ordenes-servicio/{movible_id}"), json!({ "fecha_programada": DIA })).await;
    assert!(status.is_success(), "el hueco adyacente está libre: {body}");
    assert_eq!(body["fecha_programada"], DIA);
}

#[tokio::test]
async fn un_tecnico_de_otro_tenant_no_entra_en_la_deteccion_de_conflictos() {
    let session_a = register_tenant().await;
    let session_b = register_tenant().await;
    let servicio_a = create_servicio(&session_a).await;
    let servicio_b = create_servicio(&session_b).await;
    let tecnico_a = create_empleado_named(&session_a, "Técnico del tenant A").await;
    let tecnico_b = create_empleado_named(&session_b, "Técnico del tenant B").await;

    let orden_a = crear_orden(&session_a, &servicio_a["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    asignar(&session_a, &id_of(&orden_a), &tecnico_a["id"], "TECNICO_PRINCIPAL").await;

    // Mismo día y mismo bloque, otro tenant: no se ven entre sí.
    let orden_b = crear_orden(&session_b, &servicio_b["id"], Some(DIA), Some("09:00"), Some("11:00")).await;
    let (status, body) = asignar(&session_b, &id_of(&orden_b), &tecnico_b["id"], "TECNICO_PRINCIPAL").await;
    assert!(status.is_success(), "{body}");
}

// ---------------------------------------------------------------------------
// E2 - endpoint de agenda
// ---------------------------------------------------------------------------

#[tokio::test]
async fn la_agenda_filtra_por_rango_de_fechas_por_empleado_y_excluye_canceladas() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;
    let ana = create_empleado_named(&session, "Ana Técnica").await;
    let beto = create_empleado_named(&session, "Beto Técnico").await;

    let de_ana = crear_orden(&session, &servicio["id"], Some(DIA), Some("08:00"), Some("09:00")).await;
    asignar(&session, &id_of(&de_ana), &ana["id"], "TECNICO_PRINCIPAL").await;

    let de_beto = crear_orden(&session, &servicio["id"], Some(DIA), Some("13:00"), Some("15:00")).await;
    asignar(&session, &id_of(&de_beto), &beto["id"], "TECNICO_PRINCIPAL").await;

    let otro_dia = crear_orden(&session, &servicio["id"], Some(OTRO_DIA), Some("08:00"), Some("09:00")).await;
    asignar(&session, &id_of(&otro_dia), &ana["id"], "TECNICO_PRINCIPAL").await;

    let cancelada = crear_orden(&session, &servicio["id"], Some(DIA), Some("16:00"), Some("17:00")).await;
    asignar(&session, &id_of(&cancelada), &ana["id"], "TECNICO_PRINCIPAL").await;
    session.post(&format!("/v1/ordenes-servicio/{}/cancelar", id_of(&cancelada)), json!({})).await;

    // Fuera de rango del todo.
    crear_orden(&session, &servicio["id"], Some("2026-12-25"), Some("08:00"), Some("09:00")).await;
    // Sin fecha programada: no pertenece a ninguna agenda.
    crear_orden(&session, &servicio["id"], None, None, None).await;

    // Un solo día.
    let dia = session.get(&format!("/v1/ordenes-servicio/agenda?desde={DIA}&hasta={DIA}")).await;
    let ids: Vec<&str> = dia["items"].as_array().unwrap().iter().filter_map(|o| o["id"].as_str()).collect();
    assert_eq!(dia["total"], 2, "solo las dos activas de ese día: {dia}");
    assert!(ids.contains(&de_ana["id"].as_str().unwrap()));
    assert!(ids.contains(&de_beto["id"].as_str().unwrap()));
    assert!(!ids.contains(&cancelada["id"].as_str().unwrap()), "una orden cancelada no es trabajo planificado");

    // Orden por defecto: fecha y luego hora ascendente.
    assert_eq!(dia["items"][0]["id"], de_ana["id"], "08:00 va antes que 13:00: {dia}");

    // La fila trae lo que la pantalla necesita sin una segunda llamada.
    let fila = &dia["items"][0];
    assert_eq!(fila["codigo"], codigo_de(&de_ana));
    assert_eq!(fila["estado"], "PROGRAMADA");
    assert_eq!(fila["hora_inicio"], "08:00:00");
    assert_eq!(fila["hora_fin"], "09:00:00");
    let tecnicos = fila["tecnicos"].as_array().expect("cada fila trae sus técnicos");
    assert_eq!(tecnicos.len(), 1);
    assert_eq!(tecnicos[0]["empleado_id"], ana["id"]);
    assert_eq!(tecnicos[0]["nombre"], "Ana Técnica");
    assert_eq!(tecnicos[0]["rol"], "TECNICO_PRINCIPAL");

    // Rango de dos días.
    let semana = session.get(&format!("/v1/ordenes-servicio/agenda?desde={DIA}&hasta={OTRO_DIA}")).await;
    assert_eq!(semana["total"], 3, "{semana}");

    // Filtrado por empleado.
    let solo_ana = session
        .get(&format!("/v1/ordenes-servicio/agenda?desde={DIA}&hasta={OTRO_DIA}&empleado_id={}", ana["id"].as_str().unwrap()))
        .await;
    assert_eq!(solo_ana["total"], 2, "Ana tiene dos órdenes activas en el rango: {solo_ana}");
    let ids: Vec<&str> = solo_ana["items"].as_array().unwrap().iter().filter_map(|o| o["id"].as_str()).collect();
    assert!(!ids.contains(&de_beto["id"].as_str().unwrap()));

    // Envoltorio de paginación estándar del repo.
    assert!(solo_ana["page"].is_number() && solo_ana["page_size"].is_number() && solo_ana["total_pages"].is_number(), "{solo_ana}");
}

#[tokio::test]
async fn la_agenda_incluye_ordenes_sin_hora_y_sin_tecnico() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;

    // Programada para el día pero sin bloque ni técnico: sigue siendo trabajo
    // del día y la pantalla tiene que poder mostrarla.
    let suelta = crear_orden(&session, &servicio["id"], Some(DIA), None, None).await;

    let agenda = session.get(&format!("/v1/ordenes-servicio/agenda?desde={DIA}&hasta={DIA}")).await;
    assert_eq!(agenda["total"], 1, "{agenda}");
    assert_eq!(agenda["items"][0]["id"], suelta["id"]);
    assert!(agenda["items"][0]["hora_inicio"].is_null());
    assert_eq!(agenda["items"][0]["tecnicos"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn la_agenda_rechaza_rangos_invalidos_o_desmedidos() {
    let session = register_tenant().await;

    let (status, body) = get_expect(&session, &format!("/v1/ordenes-servicio/agenda?desde={OTRO_DIA}&hasta={DIA}")).await;
    assert_eq!(status, 400, "hasta anterior a desde: {body}");

    let (status, body) = get_expect(&session, "/v1/ordenes-servicio/agenda?desde=2026-01-01&hasta=2027-01-01").await;
    assert_eq!(status, 400, "un rango de un año no es una agenda: {body}");

    let (status, body) = get_expect(&session, &format!("/v1/ordenes-servicio/agenda?desde={DIA}")).await;
    assert_eq!(status, 400, "falta hasta: {body}");
}

#[tokio::test]
async fn la_agenda_no_cruza_tenants_y_la_ruta_no_pisa_la_de_detalle() {
    let session_a = register_tenant().await;
    let session_b = register_tenant().await;
    let servicio_a = create_servicio(&session_a).await;

    let orden = crear_orden(&session_a, &servicio_a["id"], Some(DIA), Some("09:00"), Some("11:00")).await;

    let agenda_b = session_b.get(&format!("/v1/ordenes-servicio/agenda?desde={DIA}&hasta={DIA}")).await;
    assert_eq!(agenda_b["total"], 0, "la agenda de otro tenant está vacía: {agenda_b}");

    // `agenda` es un segmento estático que convive con `/:id` - el detalle
    // por UUID debe seguir resolviendo al handler de detalle.
    let detalle = session_a.get(&format!("/v1/ordenes-servicio/{}", id_of(&orden))).await;
    assert_eq!(detalle["id"], orden["id"]);
    assert!(detalle["items"].is_array(), "sigue siendo la respuesta completa de detalle: {detalle}");
}

#[tokio::test]
async fn la_agenda_respeta_el_permiso_del_modulo() {
    let session = register_tenant().await;

    let nuevo_usuario = session
        .post(
            "/v1/config/usuarios",
            json!({ "nombre": "Contador Sin Permiso", "email": format!("contador-agenda-{}@e2e-test.local", session.rnc), "password": "TestPassword123!", "rol": "CONTADOR" }),
        )
        .await;

    let login = session
        .client
        .post(format!("{}/v1/auth/login", base_url()))
        .json(&json!({ "email": nuevo_usuario["email"], "password": "TestPassword123!", "rnc": session.rnc }))
        .send()
        .await
        .expect("login failed");
    let login_body: Value = login.json().await.expect("login response not JSON");
    let contador_token = login_body["token"].as_str().expect("no token").to_string();

    // La agenda cuelga del prefijo /v1/ordenes-servicio, así que hereda
    // `ordenes_servicio.gestionar` sin tocar `required_permiso`.
    let resp = session
        .client
        .get(format!("{}/v1/ordenes-servicio/agenda?desde={DIA}&hasta={DIA}", base_url()))
        .bearer_auth(&contador_token)
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), 403, "CONTADOR no tiene ordenes_servicio.gestionar");
}

#[tokio::test]
async fn crear_una_orden_con_horas_las_guarda_y_la_deja_programada() {
    let session = register_tenant().await;
    let servicio = create_servicio(&session).await;

    let orden = crear_orden(&session, &servicio["id"], Some(DIA), Some("09:30"), Some("11:45")).await;
    assert_eq!(orden["fecha_programada"], DIA);
    assert_eq!(orden["hora_inicio"], "09:30:00");
    assert_eq!(orden["hora_fin"], "11:45:00");
    // Misma regla que ya tenía `update_orden`: fijar una fecha programada
    // agenda la orden, no la deja en BORRADOR.
    assert_eq!(orden["estado"], "PROGRAMADA");

    // Sin fecha programada sigue naciendo en BORRADOR (sin cambios).
    let sin_fecha = crear_orden(&session, &servicio["id"], None, None, None).await;
    assert_eq!(sin_fecha["estado"], "BORRADOR");
    // Los totales siguen calculándose igual que antes (una línea de RD$500 + 18%).
    assert_decimal_eq(decimal_field(&orden, "total"), dec!(590.00), "total");

    // Y el formato con segundos también se acepta.
    let con_segundos = crear_orden(&session, &servicio["id"], Some(DIA), Some("14:00:00"), Some("15:00:00")).await;
    assert_eq!(con_segundos["hora_inicio"], "14:00:00");
}

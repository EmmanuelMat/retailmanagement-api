//! Devoluciones parciales de una venta (Fase D): una Nota de Crédito que se
//! lleva solo parte de la venta, tantas veces como haga falta hasta agotarla.
//!
//! Lo que se verifica aquí es dinero: el monto acreditado sale de la línea
//! vendida (precio, descuento e ITBIS tal como se vendieron), la suma de las
//! devoluciones de una línea es exactamente la línea original, no se puede
//! devolver más de lo vendido, y el inventario, la caja, el saldo del cliente
//! fiado, el asiento contable y el IT-1 siguen a la devolución.
//!
//! Estos tests corren con la factura electrónica del tenant APAGADA
//! (`PUT /v1/config/empresa`), igual que un colmado que todavía no migró a
//! e-CF: la devolución se registra sin documento fiscal. El pipeline de firma
//! y envío a DGII está fuera del alcance de esta suite (necesita un P12 real),
//! como ya documenta `libro_diario_mayor.rs`.

mod common;

use common::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::{json, Value};

/// Apaga la factura electrónica del tenant. Sin e-CF activa, una Nota de
/// Crédito se registra igual (stock, caja, contabilidad) pero sin emitir el
/// E34 - que es lo que permite probar la plata sin un certificado DGII.
async fn apagar_factura_electronica(session: &TenantSession) {
    let resp = session
        .client
        .put(format!("{}/v1/config/empresa", base_url()))
        .bearer_auth(&session.token)
        .json(&json!({
            "razon_social": format!("Test Tenant {}", session.rnc),
            "direccion": "Calle Test #1",
            "ambiente_dgii": "TesteCF",
            "factura_electronica_activa": false,
        }))
        .send()
        .await
        .expect("PUT /v1/config/empresa falló");
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "PUT /v1/config/empresa devolvió {status}: {text}");
    let body: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    assert_eq!(body["factura_electronica_activa"], false, "la factura electrónica debía quedar apagada: {body}");
}

/// `create_producto` del harness compartido no fija `costo`, y sin costo no
/// hay Costo de Ventas que reversar - aquí sí hace falta.
async fn producto_con_costo(session: &TenantSession, precio_venta: Decimal, costo: Decimal, stock: Decimal) -> Value {
    session
        .post(
            "/v1/productos",
            json!({
                "sku": format!("SKU-{}", uuid::Uuid::new_v4()),
                "nombre": "Refresco de prueba e2e",
                "itbis_tipo": "GRAVADO_18",
                "precio_venta": precio_venta,
                "costo": costo,
                "stock_actual": stock,
            }),
        )
        .await
}

async fn devolver(session: &TenantSession, venta_id: &str, motivo: &str, items: Option<Value>) -> Value {
    let mut body = json!({ "motivo": motivo });
    if let Some(items) = items {
        body["items"] = items;
    }
    session.post(&format!("/v1/ventas/{venta_id}/nota-credito"), body).await
}

async fn devolver_esperando(session: &TenantSession, venta_id: &str, items: Value) -> (reqwest::StatusCode, Value) {
    session
        .post_expect(&format!("/v1/ventas/{venta_id}/nota-credito"), json!({ "motivo": "Devolución", "items": items }))
        .await
}

async fn stock_de(session: &TenantSession, producto_id: &str) -> Decimal {
    decimal_field(&session.get(&format!("/v1/productos/{producto_id}")).await, "stock_actual")
}

async fn lineas_por_referencia(session: &TenantSession, referencia_tipo: &str) -> Vec<Value> {
    let page = session.get(&format!("/v1/contabilidad/asientos?referenciaTipo={referencia_tipo}&pageSize=200")).await;
    page["items"].as_array().cloned().unwrap_or_default()
}

fn suma_cuenta(lineas: &[Value], cuenta: &str, campo: &str) -> Decimal {
    lineas.iter().filter(|l| l["cuenta"] == cuenta).map(|l| decimal_field(l, campo)).sum()
}

/// Período IT-1 ("YYYYMM") derivado del `created_at` del propio documento, no
/// del reloj del test: una venta creada el último segundo del mes no puede
/// caer en un período distinto al que se le consulta.
fn period_of(doc: &Value) -> String {
    let created = doc["created_at"].as_str().expect("created_at");
    format!("{}{}", &created[0..4], &created[5..7])
}

fn item_id(venta: &Value, idx: usize) -> String {
    venta["items"][idx]["id"].as_str().expect("venta.items[].id").to_string()
}

// ---------------------------------------------------------------------------

#[tokio::test]
async fn devolucion_parcial_acredita_solo_lo_devuelto_con_descuento_e_itbis() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    // precio 50.00 x 4 = 200.00 bruto, descuento 20.00 de línea:
    //   subtotal 180.00, ITBIS 18% = 32.40, total 212.40. Stock 10 -> 6.
    let producto = producto_con_costo(&session, dec!(50.00), dec!(30.00), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();
    let venta = session
        .post(
            "/v1/ventas",
            json!({
                "items": [{ "producto_id": producto_id, "cantidad": "4", "descuento": "20.00" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();
    assert_decimal_eq(decimal_field(&venta, "total"), dec!(212.40), "venta.total");

    // Devolver 1 de las 4 unidades: se acredita UN CUARTO de la línea tal como
    // se vendió (con su descuento prorrateado), no el precio de lista.
    //   subtotal 180.00 / 4 = 45.00, ITBIS 32.40 / 4 = 8.10, total 53.10.
    let nota = devolver(
        &session,
        &venta_id,
        "Una unidad defectuosa",
        Some(json!([{ "venta_item_id": item_id(&venta, 0), "cantidad": "1" }])),
    )
    .await;

    assert_decimal_eq(decimal_field(&nota, "subtotal"), dec!(45.00), "nota.subtotal");
    assert_decimal_eq(decimal_field(&nota, "itbis_total"), dec!(8.10), "nota.itbis_total");
    assert_decimal_eq(decimal_field(&nota, "total"), dec!(53.10), "nota.total");
    assert_eq!(nota["es_parcial"], true, "una devolución de 1 de 4 es parcial: {nota}");
    // El precio de lista de una unidad es 50.00 + 9.00 de ITBIS = 59.00: si la
    // devolución ignorara el descuento de la venta, el cliente se llevaría
    // 5.90 de más.
    assert!(decimal_field(&nota, "total") < dec!(59.00), "la devolución no puede ignorar el descuento de la venta");

    // La venta sigue viva: solo se anula cuando no queda nada por devolver.
    let venta_despues = session.get(&format!("/v1/ventas/{venta_id}")).await;
    assert_eq!(venta_despues["estado"], "COMPLETADA", "una devolución parcial NO anula la venta: {venta_despues}");

    // Solo vuelve al inventario lo devuelto: 10 - 4 vendidas + 1 devuelta = 7.
    assert_decimal_eq(stock_de(&session, &producto_id).await, dec!(7), "stock tras devolver 1 de 4");

    // Y solo sale de caja lo acreditado.
    let caja = caja_resumen(&session).await;
    assert_decimal_eq(decimal_field(&caja, "ingresos"), dec!(212.40), "caja.ingresos");
    assert_decimal_eq(decimal_field(&caja, "egresos"), dec!(53.10), "caja.egresos");

    // Y la venta muestra cuánto lleva devuelto cada línea.
    let devoluciones = session.get(&format!("/v1/ventas/{venta_id}/devoluciones")).await;
    let lineas = devoluciones["lineas"].as_array().expect("lineas");
    assert_eq!(lineas.len(), 1, "una línea de venta: {devoluciones}");
    assert_decimal_eq(decimal_field(&lineas[0], "cantidad_devuelta"), dec!(1), "cantidad_devuelta");
}

#[tokio::test]
async fn el_tope_por_linea_es_acumulado_entre_notas_y_rechaza_la_sobre_devolucion() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    let producto = producto_con_costo(&session, dec!(100.00), dec!(60.00), dec!(10)).await;
    let venta = session
        .post(
            "/v1/ventas",
            json!({
                "items": [{ "producto_id": producto["id"], "cantidad": "5" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();
    let linea = item_id(&venta, 0);

    // Primera parcial: 3 de 5.
    devolver(&session, &venta_id, "Devolución 1", Some(json!([{ "venta_item_id": linea, "cantidad": "3" }]))).await;

    // Segunda por 3 más: 3 + 3 = 6 > 5. Se rechaza contra el ACUMULADO, no
    // contra la cantidad vendida - este es el caso que un tope por nota
    // individual dejaría pasar.
    let (status, body) = devolver_esperando(&session, &venta_id, json!([{ "venta_item_id": linea, "cantidad": "3" }])).await;
    assert!(status.is_client_error(), "devolver 3 más de 5 (ya iban 3) debe rechazarse, devolvió {status}: {body}");
    let mensaje = body.as_str().unwrap_or_default();
    assert!(mensaje.contains("solo quedan"), "el error debe decir cuánto queda por devolver, en español: {mensaje}");

    // El rechazo no dejó rastro: ni stock, ni caja, ni una nota a medias.
    assert_decimal_eq(stock_de(&session, producto["id"].as_str().unwrap()).await, dec!(8), "stock tras 1 devolución de 3");
    let notas = session.get(&format!("/v1/notas-credito?ventaId={venta_id}")).await;
    assert_eq!(notas["total"], 1, "la devolución rechazada no debe haber creado una nota: {notas}");

    // Las 2 que sí quedan sí entran, y ahí la venta queda ANULADA.
    devolver(&session, &venta_id, "Devolución 2", Some(json!([{ "venta_item_id": linea, "cantidad": "2" }]))).await;
    let venta_despues = session.get(&format!("/v1/ventas/{venta_id}")).await;
    assert_eq!(venta_despues["estado"], "ANULADA", "devuelto todo => la venta queda ANULADA: {venta_despues}");

    // Y una tercera ya no tiene nada que devolver.
    let (status, _) = devolver_esperando(&session, &venta_id, json!([{ "venta_item_id": linea, "cantidad": "1" }])).await;
    assert!(status.is_client_error(), "no se puede devolver sobre una venta ya devuelta por completo");
}

#[tokio::test]
async fn la_suma_de_las_devoluciones_de_una_linea_es_exactamente_la_linea_original() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    // Caso elegido para que el prorrateo NO dé exacto: 10.00 x 3 = 30.00 menos
    // 0.01 de descuento => subtotal 29.99, que entre 3 da 9.996666... Con
    // redondeo puro cada tercio se acreditaría a 10.00 y la venta devolvería
    // 30.00: un centavo regalado. La política es prorratear al centavo y
    // asignar el resto a la ÚLTIMA devolución, que es la que cierra la línea.
    let producto = producto_con_costo(&session, dec!(10.00), dec!(4.00), dec!(10)).await;
    let venta = session
        .post(
            "/v1/ventas",
            json!({
                "items": [{ "producto_id": producto["id"], "cantidad": "3", "descuento": "0.01" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();
    let linea = item_id(&venta, 0);
    assert_decimal_eq(decimal_field(&venta, "subtotal"), dec!(29.99), "venta.subtotal");

    let mut suma_subtotal = Decimal::ZERO;
    let mut suma_itbis = Decimal::ZERO;
    let mut suma_total = Decimal::ZERO;
    let mut totales = Vec::new();
    for i in 0..3 {
        let nota = devolver(
            &session,
            &venta_id,
            &format!("Unidad {}", i + 1),
            Some(json!([{ "venta_item_id": linea, "cantidad": "1" }])),
        )
        .await;
        suma_subtotal += decimal_field(&nota, "subtotal");
        suma_itbis += decimal_field(&nota, "itbis_total");
        suma_total += decimal_field(&nota, "total");
        totales.push(decimal_field(&nota, "total"));
    }

    // La regla que hace que esto sea dinero y no una aproximación.
    assert_decimal_eq(suma_subtotal, decimal_field(&venta, "subtotal"), "Σ subtotales devueltos vs subtotal de la venta");
    assert_decimal_eq(suma_itbis, decimal_field(&venta, "itbis_total"), "Σ ITBIS devuelto vs ITBIS de la venta");
    assert_decimal_eq(suma_total, decimal_field(&venta, "total"), "Σ devoluciones vs total de la venta");
    // Y el resto cayó en la última, no repartido ni en la primera.
    assert_eq!(totales[0], totales[1], "las devoluciones intermedias se prorratean por igual: {totales:?}");
    assert!(totales[2] < totales[0], "la última devolución absorbe el resto: {totales:?}");

    // Caja: se devolvió exactamente lo que entró, ni un centavo más.
    let caja = caja_resumen(&session).await;
    assert_decimal_eq(decimal_field(&caja, "egresos"), decimal_field(&caja, "ingresos"), "caja: egresos devueltos vs ingresos de la venta");
}

#[tokio::test]
async fn devolucion_de_venta_fiada_baja_el_saldo_del_cliente_y_no_saca_efectivo() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    let cliente = session.post("/v1/clientes", json!({ "nombre": "Cliente Fiado", "limite_credito": "10000.00" })).await;
    let cliente_id = cliente["id"].as_str().unwrap().to_string();
    let producto = producto_con_costo(&session, dec!(100.00), dec!(60.00), dec!(10)).await;

    // 2 x 100.00 => subtotal 200.00, ITBIS 36.00, total 236.00, todo a crédito.
    let venta = session
        .post(
            "/v1/ventas",
            json!({
                "items": [{ "producto_id": producto["id"], "cantidad": "2" }],
                "metodo_pago": "FIADO",
                "cliente_id": cliente_id,
            }),
        )
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();
    assert_decimal_eq(
        decimal_field(&session.get(&format!("/v1/clientes/{cliente_id}")).await, "saldo_pendiente"),
        dec!(236.00),
        "saldo del cliente tras la venta fiada",
    );

    devolver(&session, &venta_id, "Devolvió una", Some(json!([{ "venta_item_id": item_id(&venta, 0), "cantidad": "1" }]))).await;

    // Una venta fiada nunca ingresó efectivo: la devolución baja la deuda, no
    // saca dinero de la gaveta.
    assert_decimal_eq(
        decimal_field(&session.get(&format!("/v1/clientes/{cliente_id}")).await, "saldo_pendiente"),
        dec!(118.00),
        "saldo del cliente tras devolver 1 de 2",
    );
    let caja = caja_resumen(&session).await;
    assert_decimal_eq(decimal_field(&caja, "egresos"), dec!(0), "una devolución fiada no mueve caja");
    assert_decimal_eq(decimal_field(&caja, "ingresos"), dec!(0), "una venta fiada tampoco movió caja");
}

#[tokio::test]
async fn el_asiento_de_una_devolucion_parcial_es_proporcional_y_cuadra() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    // 5 x 100.00 con costo 60.00: subtotal 500.00, ITBIS 90.00, total 590.00,
    // Costo de Ventas 300.00.
    let producto = producto_con_costo(&session, dec!(100.00), dec!(60.00), dec!(20)).await;
    let venta = session
        .post(
            "/v1/ventas",
            json!({ "items": [{ "producto_id": producto["id"], "cantidad": "5" }], "metodo_pago": "EFECTIVO" }),
        )
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();

    // Devolver 2 de 5 => 2/5 de cada cuenta: ingresos 200.00, ITBIS 36.00,
    // caja 236.00, costo/inventario 2 * 60.00 = 120.00.
    devolver(&session, &venta_id, "Devolución parcial", Some(json!([{ "venta_item_id": item_id(&venta, 0), "cantidad": "2" }]))).await;

    let sync = session.post("/v1/contabilidad/sincronizar", json!({})).await;
    assert_eq!(sync["notas_credito_procesadas"], 1, "la nota parcial debe generar su asiento: {sync}");

    let lineas = lineas_por_referencia(&session, "NOTA_CREDITO").await;
    assert_decimal_eq(suma_cuenta(&lineas, "4100 Ingresos por Ventas", "debe"), dec!(200.00), "4100 se debita por lo acreditado");
    assert_decimal_eq(suma_cuenta(&lineas, "2100 ITBIS por Pagar", "debe"), dec!(36.00), "2100 se debita por el ITBIS acreditado");
    assert_decimal_eq(suma_cuenta(&lineas, "1100 Caja y Bancos", "haber"), dec!(236.00), "1100 se acredita por el efectivo devuelto");
    assert_decimal_eq(suma_cuenta(&lineas, "1200 Inventario", "debe"), dec!(120.00), "1200 vuelve a entrar al costo de lo devuelto");
    assert_decimal_eq(suma_cuenta(&lineas, "5050 Costo de Ventas", "haber"), dec!(120.00), "5050 se acredita por el costo de lo devuelto");

    // Nada de esto puede ser el espejo completo de la venta: ahí 4100 iría por
    // 500.00 y el libro quedaría con ingresos negativos.
    let total_debe: Decimal = lineas.iter().map(|l| decimal_field(l, "debe")).sum();
    let total_haber: Decimal = lineas.iter().map(|l| decimal_field(l, "haber")).sum();
    assert_decimal_eq(total_debe, total_haber, "el asiento de la nota parcial cuadra");
    assert_decimal_eq(total_debe, dec!(356.00), "el asiento parcial mueve 236.00 + 120.00, no el asiento entero de la venta");

    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn devolucion_total_sin_items_conserva_el_comportamiento_de_siempre() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    let producto = producto_con_costo(&session, dec!(100.00), dec!(60.00), dec!(10)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();
    let venta = session
        .post("/v1/ventas", json!({ "items": [{ "producto_id": producto_id, "cantidad": "2" }], "metodo_pago": "EFECTIVO" }))
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();

    // Sin `items` en la petición: devolución total, exactamente como antes de
    // que existieran las parciales.
    let nota = devolver(&session, &venta_id, "Devolución total", None).await;
    assert_decimal_eq(decimal_field(&nota, "total"), decimal_field(&venta, "total"), "la nota total acredita el total de la venta");
    assert_decimal_eq(decimal_field(&nota, "subtotal"), decimal_field(&venta, "subtotal"), "subtotal de la nota total");
    assert_decimal_eq(decimal_field(&nota, "itbis_total"), decimal_field(&venta, "itbis_total"), "ITBIS de la nota total");
    assert_eq!(nota["es_parcial"], false, "una devolución total no es parcial: {nota}");

    let venta_despues = session.get(&format!("/v1/ventas/{venta_id}")).await;
    assert_eq!(venta_despues["estado"], "ANULADA", "la devolución total sigue anulando la venta");
    assert_decimal_eq(stock_de(&session, &producto_id).await, dec!(10), "todo el stock vuelve");

    session.post("/v1/contabilidad/sincronizar", json!({})).await;

    // El asiento de la nota total sigue siendo el espejo exacto del de la
    // venta: mismas cuentas, debe y haber intercambiados.
    let venta_lineas = lineas_por_referencia(&session, "VENTA").await;
    let nota_lineas = lineas_por_referencia(&session, "NOTA_CREDITO").await;
    assert_eq!(nota_lineas.len(), venta_lineas.len(), "la nota total tiene las mismas líneas que la venta");
    for cuenta in ["1100 Caja y Bancos", "4100 Ingresos por Ventas", "2100 ITBIS por Pagar", "5050 Costo de Ventas", "1200 Inventario"] {
        assert_decimal_eq(suma_cuenta(&nota_lineas, cuenta, "debe"), suma_cuenta(&venta_lineas, cuenta, "haber"), &format!("{cuenta}: debe de la nota == haber de la venta"));
        assert_decimal_eq(suma_cuenta(&nota_lineas, cuenta, "haber"), suma_cuenta(&venta_lineas, cuenta, "debe"), &format!("{cuenta}: haber de la nota == debe de la venta"));
    }
    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn it1_resta_el_itbis_de_una_devolucion_parcial_sin_perder_el_de_la_venta() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    let producto = producto_con_costo(&session, dec!(100.00), dec!(60.00), dec!(20)).await;
    let venta = session
        .post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "5" }], "metodo_pago": "EFECTIVO" }))
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();

    // ITBIS facturado 90.00; se devuelven 2 de 5 => 36.00 acreditados.
    let nota = devolver(&session, &venta_id, "Devolución parcial", Some(json!([{ "venta_item_id": item_id(&venta, 0), "cantidad": "2" }]))).await;
    let period = period_of(&venta);
    assert_eq!(period, period_of(&nota), "venta y nota deben caer en el mismo período IT-1 para esta aserción");

    let it1 = session.get(&format!("/v1/reports/it1?period={period}")).await;
    assert_decimal_eq(decimal_field(&it1, "itbis_trasladado"), dec!(54.00), "IT-1: 90.00 facturados menos 36.00 acreditados por la nota parcial");
}

#[tokio::test]
async fn it1_no_resta_dos_veces_el_itbis_de_una_devolucion_total() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    let producto = producto_con_costo(&session, dec!(100.00), dec!(60.00), dec!(20)).await;
    let venta = session
        .post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "5" }], "metodo_pago": "EFECTIVO" }))
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();

    // La devolución total deja la venta ANULADA. Si el IT-1 sacara de la suma
    // las ventas anuladas Y además restara la nota, declararía -90.00 de ITBIS
    // trasladado por una operación que acabó en cero.
    let nota = devolver(&session, &venta_id, "Devolución total", None).await;
    let period = period_of(&venta);
    assert_eq!(period, period_of(&nota), "venta y nota deben caer en el mismo período IT-1 para esta aserción");

    let it1 = session.get(&format!("/v1/reports/it1?period={period}")).await;
    assert_decimal_eq(decimal_field(&it1, "itbis_trasladado"), dec!(0), "IT-1: venta facturada y devuelta en el mismo periodo netea a cero, no a negativo");
}

#[tokio::test]
async fn la_devolucion_queda_en_la_bitacora_de_auditoria_con_sus_lineas() {
    let session = register_tenant().await;
    apagar_factura_electronica(&session).await;
    abrir_caja(&session, dec!(0)).await;

    let producto = producto_con_costo(&session, dec!(100.00), dec!(60.00), dec!(10)).await;
    let venta = session
        .post("/v1/ventas", json!({ "items": [{ "producto_id": producto["id"], "cantidad": "3" }], "metodo_pago": "EFECTIVO" }))
        .await;
    let venta_id = venta["id"].as_str().unwrap().to_string();
    devolver(&session, &venta_id, "Producto defectuoso", Some(json!([{ "venta_item_id": item_id(&venta, 0), "cantidad": "1" }]))).await;

    let bitacora = session.get("/v1/auditoria?pageSize=50").await;
    let entrada = bitacora["items"]
        .as_array()
        .expect("auditoria.items")
        .iter()
        .find(|e| e["accion"] == "NOTA_CREDITO_EMITIDA")
        .unwrap_or_else(|| panic!("no hay NOTA_CREDITO_EMITIDA en la bitácora: {bitacora}"));
    assert_eq!(entrada["detalle"]["es_parcial"], true, "el detalle dice si fue parcial: {entrada}");
    let lineas = entrada["detalle"]["lineas"].as_array().expect("detalle.lineas");
    assert_eq!(lineas.len(), 1, "el detalle registra qué líneas se devolvieron: {entrada}");
    assert_decimal_eq(decimal_field(&lineas[0], "cantidad"), dec!(1), "y cuánto de cada una");
}

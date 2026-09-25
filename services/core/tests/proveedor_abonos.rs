//! Fase F5: abono a proveedor (pago de `2110 Cuentas por Pagar`).
//!
//! Espejo del lado cliente (`cliente_abonos`): una compra FIADO crea la
//! deuda y hasta ahora solo podía cerrarse con un asiento manual
//! (docs/14 §4). El saldo del proveedor es **derivado** (compras FIADO no
//! anuladas − notas de crédito − abonos), no una columna, así que estos
//! tests lo leen del propio endpoint de proveedores.

mod common;

use common::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde_json::json;

/// Crea un proveedor y una compra FIADO contra él. Devuelve
/// `(proveedor_id, total_de_la_compra)`.
async fn proveedor_con_deuda(session: &TenantSession, costo_unitario: Decimal, cantidad: &str) -> (String, Decimal) {
    let proveedor = session.post("/v1/proveedores", json!({ "nombre": "Distribuidora e2e", "rnc": "101000001" })).await;
    let proveedor_id = proveedor["id"].as_str().unwrap().to_string();
    let producto = create_producto(session, dec!(0), dec!(0)).await;

    let vence = (chrono::Utc::now().date_naive() + chrono::Duration::days(30)).to_string();
    let compra = session
        .post(
            "/v1/compras",
            json!({
                "proveedor_id": proveedor_id,
                "items": [{ "producto_id": producto["id"], "cantidad": cantidad, "costo_unitario": costo_unitario }],
                "metodo_pago": "FIADO",
                "fecha_vencimiento": vence,
            }),
        )
        .await;
    (proveedor_id, decimal_field(&compra, "total"))
}

async fn saldo_proveedor(session: &TenantSession, proveedor_id: &str) -> Decimal {
    let proveedor = session.get(&format!("/v1/proveedores/{proveedor_id}")).await;
    decimal_field(&proveedor, "saldo_pendiente")
}

#[tokio::test]
async fn una_compra_fiado_deja_saldo_pendiente_en_el_proveedor() {
    let session = register_tenant().await;
    let (proveedor_id, total) = proveedor_con_deuda(&session, dec!(50.00), "10").await;

    assert_decimal_eq(saldo_proveedor(&session, &proveedor_id).await, total, "el saldo del proveedor es el total de la compra FIADO");

    // Y también se ve en el listado, que es donde el usuario lo busca.
    let listado = session.get("/v1/proveedores?pageSize=50").await;
    let fila = listado["items"].as_array().unwrap().iter().find(|p| p["id"] == proveedor_id.as_str()).expect("proveedor en el listado");
    assert_decimal_eq(decimal_field(fila, "saldo_pendiente"), total, "saldo_pendiente en el listado");
}

#[tokio::test]
async fn un_abono_en_efectivo_baja_el_saldo_y_registra_egreso_de_caja() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(5000.00)).await;
    let (proveedor_id, total) = proveedor_con_deuda(&session, dec!(50.00), "10").await;

    let caja_antes = decimal_field(&caja_resumen(&session).await, "egresos");

    session.post(&format!("/v1/proveedores/{proveedor_id}/abonos"), json!({ "monto": "200.00" })).await;

    assert_decimal_eq(saldo_proveedor(&session, &proveedor_id).await, total - dec!(200.00), "el saldo baja por el abono");
    assert_decimal_eq(
        decimal_field(&caja_resumen(&session).await, "egresos") - caja_antes,
        dec!(200.00),
        "un abono en efectivo sale de la caja",
    );

    let abonos = session.get(&format!("/v1/proveedores/{proveedor_id}/abonos")).await;
    assert_eq!(abonos.as_array().map(|a| a.len()), Some(1), "el abono queda listado: {abonos}");
    assert_eq!(abonos[0]["metodo_pago"], "EFECTIVO");
}

/// Una transferencia no toca la caja física, pero sí baja la deuda y sí
/// genera el mismo asiento (ver partner_service::registrar_abono_proveedor).
#[tokio::test]
async fn una_transferencia_no_toca_la_caja_pero_si_el_saldo() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0)).await;
    let (proveedor_id, total) = proveedor_con_deuda(&session, dec!(50.00), "10").await;

    let caja_antes = decimal_field(&caja_resumen(&session).await, "egresos");
    session
        .post(&format!("/v1/proveedores/{proveedor_id}/abonos"), json!({ "monto": "100.00", "metodo_pago": "TRANSFERENCIA" }))
        .await;

    assert_decimal_eq(saldo_proveedor(&session, &proveedor_id).await, total - dec!(100.00), "el saldo baja igual");
    assert_decimal_eq(
        decimal_field(&caja_resumen(&session).await, "egresos"),
        caja_antes,
        "una transferencia no mueve la caja física",
    );
}

#[tokio::test]
async fn el_abono_postea_dr_cuentas_por_pagar_cr_caja_y_deja_el_mayor_cuadrado() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(5000.00)).await;
    let (proveedor_id, total) = proveedor_con_deuda(&session, dec!(50.00), "10").await;
    session.post(&format!("/v1/proveedores/{proveedor_id}/abonos"), json!({ "monto": "300.00" })).await;

    let sync = session.post("/v1/contabilidad/sincronizar", json!({})).await;
    assert_eq!(sync["abonos_proveedor_procesados"], 1, "el loop nuevo de sincronizar debe procesarlo: {sync}");

    let lineas = session.get("/v1/contabilidad/asientos?referenciaTipo=ABONO_PROVEEDOR&pageSize=50").await;
    let lineas = lineas["items"].as_array().expect("asientos.items");
    assert_eq!(lineas.len(), 2, "un abono son dos líneas (2110 / 1100): {lineas:?}");
    let por_pagar = lineas.iter().find(|l| l["cuenta"] == "2110 Cuentas por Pagar").expect("línea 2110");
    assert_decimal_eq(decimal_field(por_pagar, "debe"), dec!(300.00), "Dr 2110 Cuentas por Pagar");
    let caja = lineas.iter().find(|l| l["cuenta"] == "1100 Caja y Bancos").expect("línea 1100");
    assert_decimal_eq(decimal_field(caja, "haber"), dec!(300.00), "Cr 1100 Caja y Bancos");

    // La cuenta por pagar en el mayor baja exactamente lo abonado.
    let (filas, _, _) = libro_mayor(&session).await;
    let fila_2110 = filas.iter().find(|f| f["cuenta"] == "2110 Cuentas por Pagar").expect("2110 en el mayor");
    assert_decimal_eq(
        decimal_field(fila_2110, "haber") - decimal_field(fila_2110, "debe"),
        total - dec!(300.00),
        "el pasivo 2110 queda en la deuda restante",
    );

    assert_ledger_balanced(&session).await;

    // Segunda sincronización: idempotente, no vuelve a postear.
    let otra = session.post("/v1/contabilidad/sincronizar", json!({})).await;
    assert_eq!(otra["abonos_proveedor_procesados"], 0, "no debe duplicarse: {otra}");
}

#[tokio::test]
async fn no_se_puede_abonar_mas_de_lo_que_se_debe_ni_a_quien_no_debe_nada() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(5000.00)).await;
    let (proveedor_id, total) = proveedor_con_deuda(&session, dec!(50.00), "10").await;

    let (status, body) = session
        .post_expect(&format!("/v1/proveedores/{proveedor_id}/abonos"), json!({ "monto": total + dec!(0.01) }))
        .await;
    assert!(!status.is_success(), "un sobrepago debe rechazarse, dio {status}: {body}");

    let sin_deuda = session.post("/v1/proveedores", json!({ "nombre": "Proveedor sin deuda e2e" })).await;
    let (status2, body2) = session
        .post_expect(&format!("/v1/proveedores/{}/abonos", sin_deuda["id"].as_str().unwrap()), json!({ "monto": "10.00" }))
        .await;
    assert!(!status2.is_success(), "abonar a quien no se le debe nada debe rechazarse, dio {status2}: {body2}");

    let (status3, body3) = session.post_expect(&format!("/v1/proveedores/{proveedor_id}/abonos"), json!({ "monto": "0" })).await;
    assert!(!status3.is_success(), "un monto cero debe rechazarse, dio {status3}: {body3}");

    // Nada de lo anterior tocó el saldo.
    assert_decimal_eq(saldo_proveedor(&session, &proveedor_id).await, total, "ningún rechazo debe alterar el saldo");
}

#[tokio::test]
async fn el_abono_a_proveedor_queda_auditado() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(5000.00)).await;
    let (proveedor_id, _) = proveedor_con_deuda(&session, dec!(50.00), "10").await;
    session.post(&format!("/v1/proveedores/{proveedor_id}/abonos"), json!({ "monto": "75.00" })).await;

    let page = session.get("/v1/auditoria?accion=ABONO_PROVEEDOR_REGISTRADO&pageSize=50").await;
    let entrada = page["items"]
        .as_array()
        .and_then(|i| i.iter().find(|e| e["accion"] == "ABONO_PROVEEDOR_REGISTRADO"))
        .unwrap_or_else(|| panic!("el abono a proveedor debe auditarse: {page}"));
    assert_eq!(entrada["entidad"], "proveedor");
    assert_eq!(entrada["entidad_id"], proveedor_id.as_str());
}

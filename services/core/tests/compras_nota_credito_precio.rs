//! Purchase NOTA_CREDITO that is only a price/discount adjustment
//! (`ajuste_solo_precio: true`) — no goods go back to the supplier.
//!
//! Phase A made a purchase NOTA_CREDITO mean "physical return": stock SALIDA +
//! weighted-average `costo` recomputed. This file pins the *other* case, which
//! previously either decremented stock wrongly or was rejected outright by the
//! stock guard. The invariant under test is the same one Phase A protected:
//!
//!     GL "1200 Inventario" (net debe - haber) == stock_actual * costo
//!
//! With no quantity movement, the only way to keep it is to push the whole
//! credit into the unit cost at unchanged quantity.

mod common;

use common::*;
use rust_decimal_macros::dec;

/// `GET` that returns the raw body instead of parsed JSON — the 606 endpoint
/// serves `text/plain`, so `TenantSession::get` cannot be used for it.
async fn get_text(session: &TenantSession, path: &str) -> String {
    let resp = session
        .client
        .get(format!("{}{}", base_url(), path))
        .bearer_auth(&session.token)
        .send()
        .await
        .unwrap_or_else(|e| panic!("GET {path} failed: {e}"));
    let status = resp.status();
    let body = resp.text().await.unwrap_or_else(|e| panic!("GET {path} body unreadable: {e}"));
    assert!(status.is_success(), "GET {path} returned {status}: {body}");
    body
}

/// Buys 10 units at 50.00 EXENTO (subtotal 500.00, no ITBIS so the ledger
/// entry is exactly 1200/1100) and returns the product id.
async fn compra_base(session: &TenantSession) -> String {
    let producto = create_producto(session, dec!(0), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();
    session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "10", "costo_unitario": "50.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "EFECTIVO",
            }),
        )
        .await;
    producto_id
}

#[tokio::test]
async fn nota_credito_solo_precio_no_mueve_stock_y_baja_el_costo() {
    let session = register_tenant().await;
    abrir_caja(&session, dec!(0.00)).await;
    let producto_id = compra_base(&session).await;

    // Supplier grants a 5.00/unit rebate on the 10 units already invoiced:
    // subtotal 50.00, EXENTO so total 50.00. Nothing physically moves.
    let nota = session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "10", "costo_unitario": "5.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "EFECTIVO",
                "tipo_documento": "NOTA_CREDITO",
                "ncf_modificado": "B0100000001",
                "ajuste_solo_precio": true,
            }),
        )
        .await;
    assert_decimal_eq(decimal_field(&nota, "total"), dec!(50.00), "nota_credito.total");

    let producto = session.get(&format!("/v1/productos/{producto_id}")).await;
    let stock = decimal_field(&producto, "stock_actual");
    let costo = decimal_field(&producto, "costo");
    assert_decimal_eq(stock, dec!(10), "a price-only credit note must NOT move stock");
    // (50.00 * 10 - 50.00) / 10 = 45.00
    assert_decimal_eq(costo, dec!(45.00), "costo after the rebate");

    // Kardex: only the original purchase's ENTRADA, no phantom SALIDA.
    let movimientos = session.get(&format!("/v1/inventario/movimientos?productoId={producto_id}&pageSize=50")).await;
    let items = movimientos["items"].as_array().expect("movimientos.items");
    assert_eq!(items.len(), 1, "a price-only credit note must not write a kardex movement: {items:?}");

    // Caja still gets the refund (same as the physical-return path).
    let resumen = caja_resumen(&session).await;
    assert_decimal_eq(decimal_field(&resumen, "egresos"), dec!(500.00), "caja egresos: the original FACTURA");
    assert_decimal_eq(decimal_field(&resumen, "ingresos"), dec!(50.00), "caja ingresos: the credit-note refund");

    session.post("/v1/contabilidad/sincronizar", serde_json::json!({})).await;
    let asientos = session.get("/v1/contabilidad/asientos?referenciaTipo=COMPRA&pageSize=50").await;
    let lineas = asientos["items"].as_array().expect("asientos.items");

    let mut inventario_neto = rust_decimal::Decimal::ZERO;
    for linea in lineas {
        if linea["cuenta"] == "1200 Inventario" {
            inventario_neto += decimal_field(linea, "debe") - decimal_field(linea, "haber");
        }
    }
    assert_decimal_eq(inventario_neto, dec!(450.00), "net GL 1200 Inventario (500.00 in, 50.00 credited)");
    assert_decimal_eq(stock * costo, inventario_neto, "stock_actual * costo must still equal net GL 1200");

    assert_ledger_balanced(&session).await;
}

#[tokio::test]
async fn nota_credito_solo_precio_se_reporta_en_606_y_resta_del_it1() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(0), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();

    // FACTURA 10 @ 50.00 GRAVADO_18 => subtotal 500.00, itbis 90.00.
    let factura = session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "10", "costo_unitario": "50.00", "itbis_tipo": "GRAVADO_18" }],
                "metodo_pago": "EFECTIVO",
                "ncf_proveedor": "B0100000001",
            }),
        )
        .await;
    // Price-only NC 10 @ 5.00 GRAVADO_18 => subtotal 50.00, itbis 9.00.
    session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "10", "costo_unitario": "5.00", "itbis_tipo": "GRAVADO_18" }],
                "metodo_pago": "EFECTIVO",
                "ncf_proveedor": "B0400000007",
                "tipo_documento": "NOTA_CREDITO",
                "ncf_modificado": "B0100000001",
                "ajuste_solo_precio": true,
            }),
        )
        .await;

    let created_at = factura["created_at"].as_str().expect("compra.created_at");
    let period = format!("{}{}", &created_at[0..4], &created_at[5..7]);

    // 606: the credit note is its own row carrying `ncf_modificado` (casilla 5),
    // exactly like the physical-return case — unchanged by this phase.
    let txt = get_text(&session, &format!("/v1/reports/606?period={period}")).await;
    let filas: Vec<&str> = txt.lines().skip(1).filter(|l| !l.is_empty()).collect();
    assert_eq!(filas.len(), 2, "606 must list the FACTURA and the credit note as two rows: {txt}");
    let nota_row = filas.iter().find(|l| l.contains("B0400000007")).expect("credit-note row in the 606");
    let campos: Vec<&str> = nota_row.split('|').collect();
    assert_eq!(campos[4], "B0100000001", "606 casilla 5 (NCF Modificado) on the credit note row: {nota_row}");

    // IT-1: creditable ITBIS nets the note out (90.00 - 9.00).
    let it1 = session.get(&format!("/v1/reports/it1?period={period}")).await;
    assert_decimal_eq(decimal_field(&it1, "itbis_acreditable"), dec!(81.00), "IT-1 itbis_acreditable");
}

#[tokio::test]
async fn nota_credito_solo_precio_sin_stock_se_rechaza() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(0), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();

    // Nothing in stock: the discounted goods are already sold, so the credit
    // belongs in 5050 Costo de Ventas, which this path cannot post.
    let (status, body) = session
        .post_expect(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "10", "costo_unitario": "5.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "EFECTIVO",
                "tipo_documento": "NOTA_CREDITO",
                "ncf_modificado": "B0100000001",
                "ajuste_solo_precio": true,
            }),
        )
        .await;
    assert!(status.is_client_error(), "price-only credit note with zero stock should be rejected, got {status}: {body}");
}

#[tokio::test]
async fn nota_credito_solo_precio_mayor_al_valor_en_libros_se_rechaza() {
    let session = register_tenant().await;
    let producto_id = compra_base(&session).await; // 10 units @ 50.00 => 500.00 carrying value

    let (status, body) = session
        .post_expect(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "10", "costo_unitario": "60.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "EFECTIVO",
                "tipo_documento": "NOTA_CREDITO",
                "ncf_modificado": "B0100000001",
                "ajuste_solo_precio": true,
            }),
        )
        .await;
    assert!(
        status.is_client_error(),
        "a 600.00 credit against a 500.00 carrying value should be rejected, got {status}: {body}"
    );
}

#[tokio::test]
async fn ajuste_solo_precio_no_se_acepta_en_una_factura() {
    let session = register_tenant().await;
    let producto = create_producto(&session, dec!(0), dec!(0)).await;
    let producto_id = producto["id"].as_str().unwrap().to_string();

    let (status, body) = session
        .post_expect(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "10", "costo_unitario": "50.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "EFECTIVO",
                "ajuste_solo_precio": true,
            }),
        )
        .await;
    assert!(status.is_client_error(), "ajuste_solo_precio only applies to a NOTA_CREDITO, got {status}: {body}");
}

#[tokio::test]
async fn nota_credito_sin_la_bandera_sigue_siendo_devolucion_fisica() {
    let session = register_tenant().await;
    let producto_id = compra_base(&session).await;

    // Same request minus `ajuste_solo_precio`: the Phase A behaviour must be
    // byte-for-byte unchanged (stock 10 -> 8).
    session
        .post(
            "/v1/compras",
            serde_json::json!({
                "items": [{ "producto_id": producto_id, "cantidad": "2", "costo_unitario": "50.00", "itbis_tipo": "EXENTO" }],
                "metodo_pago": "EFECTIVO",
                "tipo_documento": "NOTA_CREDITO",
                "ncf_modificado": "B0100000001",
            }),
        )
        .await;

    let producto = session.get(&format!("/v1/productos/{producto_id}")).await;
    assert_decimal_eq(decimal_field(&producto, "stock_actual"), dec!(8), "default NOTA_CREDITO still returns goods");
    assert_decimal_eq(decimal_field(&producto, "costo"), dec!(50.00), "(500.00 - 100.00) / 8 = 50.00");
}

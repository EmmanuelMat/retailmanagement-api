//! Shared harness for the backend money e2e suite (services/core/tests/*.rs).
//!
//! These are black-box HTTP tests against a real `fiscal-core` server that is
//! already running (started by scripts/e2e/run-local.sh or
//! .github/workflows/e2e.yml, against a real ephemeral Postgres, *before*
//! `cargo test` is invoked) - not a server spawned per test binary. A single
//! shared instance avoids every `tests/*.rs` file (cargo compiles each into
//! its own process) fighting over the same port, and keeps this suite truly
//! black-box: zero production code is touched or made test-aware, requests
//! go over real HTTP exactly like any other client.
//!
//! Test isolation: every test registers its own fresh tenant (unique RNC via
//! `register_tenant`), so tests can run under `cargo test`'s default
//! parallelism with no shared mutable state and no need to reset tables
//! between tests.
//!
//! Money precision: every response field carrying an amount is a JSON
//! *string* (see `rust_decimal`'s default `Serialize` impl - this repo does
//! not enable the `serde-float` feature), e.g. `"total": "118.00"`. Always
//! parse those into `rust_decimal::Decimal` and compare with `==`/
//! `assert_decimal_eq` - never `f64`, never string-equality on a formatted
//! display value.

use rust_decimal::Decimal;
use serde_json::{json, Value};
use std::str::FromStr;

pub fn base_url() -> String {
    std::env::var("CORE_HTTP_URL").unwrap_or_else(|_| "http://localhost:3001".to_string())
}

/// A fresh, valid (9-digit numeric) RNC per call - `auth_service::register`
/// requires 9-11 numeric digits and tenant-wide uniqueness. Sourced from a
/// UUIDv4's own randomness (not a per-process counter + clock jitter - an
/// earlier version of this used `subsec_nanos()`, which turned out to have
/// coarser real resolution than its name implies on at least one CI/dev
/// machine, so parallel `#[tokio::test]`s starting in the same tick reliably
/// collided in practice, not just in theory).
fn unique_rnc() -> String {
    let bytes = uuid::Uuid::new_v4();
    let mut n: u64 = 0;
    for b in &bytes.as_bytes()[..8] {
        n = (n << 8) | (*b as u64);
    }
    // 9 digits, first digit always 1-9 (never a leading zero).
    (100_000_000 + (n % 900_000_000)).to_string()
}

pub struct TenantSession {
    pub client: reqwest::Client,
    pub token: String,
    pub rnc: String,
}

impl TenantSession {
    pub async fn get(&self, path: &str) -> Value {
        let resp = self
            .client
            .get(format!("{}{}", base_url(), path))
            .bearer_auth(&self.token)
            .send()
            .await
            .unwrap_or_else(|e| panic!("GET {path} failed: {e}"));
        let status = resp.status();
        let body: Value = resp.json().await.unwrap_or_else(|e| panic!("GET {path} response not JSON: {e}"));
        assert!(status.is_success(), "GET {path} returned {status}: {body}");
        body
    }

    /// Like `get`, but returns the raw status alongside the body instead of
    /// asserting success - for tests that expect (and must verify) a
    /// rejection, e.g. an over-cap payroll advance request.
    ///
    /// Handlers here return errors as axum's `(StatusCode, String)`, which
    /// renders as a plain-text body, not JSON - unlike every success
    /// response. So the body is read as text first and only parsed as JSON
    /// if it actually looks like JSON; otherwise it's wrapped as a JSON
    /// string so callers can still inspect the error message uniformly via
    /// `payload.as_str()` (success) or `payload["..."]` (error, rarely
    /// needed) without the helper panicking on a plain-text error body.
    pub async fn post_expect(&self, path: &str, body: Value) -> (reqwest::StatusCode, Value) {
        let resp = self
            .client
            .post(format!("{}{}", base_url(), path))
            .bearer_auth(&self.token)
            .json(&body)
            .send()
            .await
            .unwrap_or_else(|e| panic!("POST {path} failed: {e}"));
        let status = resp.status();
        let text = resp.text().await.unwrap_or_else(|e| panic!("POST {path} response body unreadable: {e}"));
        let payload = serde_json::from_str(&text).unwrap_or(Value::String(text));
        (status, payload)
    }

    pub async fn post(&self, path: &str, body: Value) -> Value {
        let (status, payload) = self.post_expect(path, body).await;
        assert!(status.is_success(), "POST {path} returned {status}: {payload}");
        payload
    }
}

pub async fn register_tenant() -> TenantSession {
    let client = reqwest::Client::new();
    let rnc = unique_rnc();
    let body = json!({
        "rnc": rnc,
        "razon_social": format!("Test Tenant {rnc}"),
        "direccion": "Calle Test #1",
        "admin_nombre": "Test Admin",
        "admin_email": format!("admin-{rnc}@e2e-test.local"),
        "admin_password": "TestPassword123!",
    });
    let resp = client
        .post(format!("{}/v1/auth/register", base_url()))
        .json(&body)
        .send()
        .await
        .expect("register request failed - is fiscal-core running? (see scripts/e2e/run-local.sh)");
    let status = resp.status();
    // Error responses render as plain text (axum's `(StatusCode, String)`),
    // not JSON, unlike every success response - see `post_expect` above.
    let text = resp.text().await.expect("register response body unreadable");
    let payload: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
    assert!(status.is_success(), "register failed ({status}): {payload}");
    let token = payload["token"].as_str().expect("no token in register response").to_string();
    TenantSession { client, token, rnc }
}

/// Parses a money field that the server always sends as a JSON string
/// (e.g. `body["total"]`) into an exact `Decimal`. Panics with the raw
/// field on anything else, since a missing/mistyped money field is a test
/// bug worth failing loudly on.
pub fn decimal_field(value: &Value, field: &str) -> Decimal {
    let raw = value
        .get(field)
        .unwrap_or_else(|| panic!("missing field {field:?} in {value}"))
        .as_str()
        .unwrap_or_else(|| panic!("field {field:?} is not a JSON string in {value}"));
    Decimal::from_str(raw).unwrap_or_else(|e| panic!("field {field:?} ({raw:?}) is not a valid Decimal: {e}"))
}

/// The single rule this whole suite enforces on money: exact `Decimal`
/// equality, never a float, never a tolerance/epsilon comparison.
pub fn assert_decimal_eq(actual: Decimal, expected: Decimal, context: &str) {
    assert_eq!(actual, expected, "{context}: expected {expected}, got {actual}");
}

/// Flips a tenant's `tipo_negocio` via the staff-only endpoint (the only
/// write path - see `staff_service::set_tipo_negocio`). Requires
/// `VENDOR_ADMIN_SECRET` in the environment, same as the running server (see
/// `scripts/e2e/env.sh` - both are sourced into the same shell before
/// `cargo test` runs).
pub async fn set_tipo_negocio(session: &TenantSession, tipo_negocio: &str) {
    let secret = std::env::var("VENDOR_ADMIN_SECRET").expect("VENDOR_ADMIN_SECRET debe estar en el entorno del test (ver scripts/e2e/env.sh)");
    let resp = session
        .client
        .put(format!("{}/v1/staff/tenants/{}/tipo-negocio", base_url(), session.rnc))
        .header("X-Vendor-Secret", secret)
        .json(&json!({ "tipo_negocio": tipo_negocio }))
        .send()
        .await
        .unwrap_or_else(|e| panic!("PUT tipo-negocio failed: {e}"));
    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    assert!(status.is_success(), "PUT tipo-negocio returned {status}: {text}");
}

pub async fn abrir_caja(session: &TenantSession, monto_inicial: Decimal) -> Value {
    session.post("/v1/caja/abrir", json!({ "monto_inicial": monto_inicial })).await
}

pub async fn cerrar_caja(session: &TenantSession, monto_final: Decimal) -> Value {
    session.post("/v1/caja/cerrar", json!({ "monto_final": monto_final })).await
}

pub async fn caja_resumen(session: &TenantSession) -> Value {
    session.get("/v1/caja/resumen").await
}

/// Creates a product with an explicit price/stock (both default to zero
/// server-side, which is useless for a sale) and `GRAVADO_18` ITBIS unless
/// overridden - matches `catalog_service::ITBIS_TIPOS`.
pub async fn create_producto(session: &TenantSession, precio_venta: Decimal, stock_actual: Decimal) -> Value {
    let sku = format!("SKU-{}", uuid::Uuid::new_v4());
    session
        .post(
            "/v1/productos",
            json!({
                "sku": sku,
                "nombre": "Producto de prueba e2e",
                "itbis_tipo": "GRAVADO_18",
                "precio_venta": precio_venta,
                "stock_actual": stock_actual,
            }),
        )
        .await
}

/// Producto tipo SERVICIO: sin precio_venta ni stock (ver
/// catalog_service::CatalogService::create_producto) - el precio se manda
/// por línea al usarlo en una venta/cotización/conduce.
pub async fn create_servicio(session: &TenantSession) -> Value {
    let sku = format!("SRV-{}", uuid::Uuid::new_v4());
    session
        .post(
            "/v1/productos",
            json!({
                "sku": sku,
                "nombre": "Servicio de prueba e2e",
                "itbis_tipo": "GRAVADO_18",
                "tipo": "SERVICIO",
            }),
        )
        .await
}

pub async fn create_empleado(session: &TenantSession, salario_mensual: Decimal) -> Value {
    session
        .post(
            "/v1/empleados",
            json!({
                "nombre": "Empleado de prueba e2e",
                "salario_mensual": salario_mensual,
            }),
        )
        .await
}

/// Reads every account's balance via `GET /v1/contabilidad/libro-mayor`
/// (mirrors `contabilidad_service::libro_mayor`'s own `SUM(debe)`/
/// `SUM(haber)` GROUP BY cuenta query) and returns the account rows plus
/// the grand totals across all of them.
pub async fn libro_mayor(session: &TenantSession) -> (Vec<Value>, Decimal, Decimal) {
    let page = session.get("/v1/contabilidad/libro-mayor?pageSize=200").await;
    let items = page["items"].as_array().cloned().unwrap_or_default();
    let mut total_debe = Decimal::ZERO;
    let mut total_haber = Decimal::ZERO;
    for row in &items {
        total_debe += decimal_field(row, "debe");
        total_haber += decimal_field(row, "haber");
    }
    (items, total_debe, total_haber)
}

/// The suite's golden invariant: this tenant's books balance exactly. Call
/// after `POST /v1/contabilidad/sincronizar` so every money-writing flow
/// (ventas/compras/gastos/adelantos/nómina) has had its ledger lines
/// generated - see `contabilidad_service::sincronizar`.
pub async fn assert_ledger_balanced(session: &TenantSession) {
    let (items, total_debe, total_haber) = libro_mayor(session).await;
    assert_eq!(
        total_debe, total_haber,
        "ledger out of balance for tenant {}: SUM(debe)={} != SUM(haber)={} across accounts {:?}",
        session.rnc, total_debe, total_haber, items
    );
}

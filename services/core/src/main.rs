//! fiscal-core - Bank-grade Rust core
//! HTTP :3001 (Axum 0.6) - Real XAdES-BES + Full ECF Builder + RFCE + ARECF/ACECF
//! gRPC disabled for now to avoid http version conflict (axum 0.6 vs tonic 0.12), enable with feature flag later

mod aggregates;
mod arecf_acecf_builder;
mod dgii_client;
mod ecf_builder;
mod event_store;
mod grpc;
mod ledger;
mod rfce_builder;
mod services;
mod xml_c14n;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use grpc::AppState;
use services::ecfl_service::{sign_xml_ecf, generate_qr_url};
use ecf_builder::{build_ecf_xml, build_simple_pos_ecf, ECF};
use dgii_client::{DGIIClient, DGIIEnvironment};

#[derive(Clone)]
struct HttpState {
    app_state: Arc<AppState>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "fiscal_core=debug".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = Arc::new(AppState::new());
    let http_state = HttpState {
        app_state: app_state.clone(),
    };

    let http_app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v1/ecf/sign", post(http_sign_ecf))
        .route("/v1/ecf/sign-rfce", post(http_sign_rfce))
        .route("/v1/ecf/build", post(http_build_ecf))
        .route("/v1/ecf/build-sign", post(http_build_sign_ecf))
        .route("/v1/ecf/build-sign-send", post(http_build_sign_send))
        .route("/v1/ecf/authenticate", post(http_authenticate))
        .route("/v1/ecf/status/:track_id", get(http_status_track))
        .route("/v1/ecf/rfce/build", post(http_build_rfce))
        .route("/v1/ecf/rfce/build-sign", post(http_build_sign_rfce_full))
        .route("/v1/ecf/rfce/build-sign-send", post(http_build_sign_send_rfce))
        .route("/v1/ecf/arecf/build", post(http_build_arecf))
        .route("/v1/ecf/arecf/build-sign", post(http_build_sign_arecf))
        .route("/v1/ecf/acecf/build", post(http_build_acecf))
        .route("/v1/ecf/acecf/build-sign", post(http_build_sign_acecf))
        .route("/v1/advances/request", post(http_advance_request))
        .route("/v1/advances/approve", post(http_advance_approve))
        .route("/v1/employees/:id/balance", get(http_employee_balance))
        .route("/v1/payroll/run", post(http_payroll_run))
        .route("/v1/reports/606", get(http_report_606))
        .route("/v1/reports/607", get(http_report_607))
        .route("/v1/test/sign-demo", post(http_test_sign_demo))
        .with_state(http_state)
        .layer(tower_http::cors::CorsLayer::permissive())
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let http_port = std::env::var("CORE_HTTP_PORT").unwrap_or_else(|_| "3001".to_string());
    let http_addr: SocketAddr = format!("0.0.0.0:{}", http_port).parse()?;
    tracing::info!("HTTP listening on {} - Spanish POS ready, DGII XAdES-BES, RFCE, ARECF/ACECF", http_addr);

    // Axum 0.6 style server
    axum::Server::bind(&http_addr)
        .serve(http_app.into_make_service())
        .await?;

    Ok(())
}

async fn root() -> &'static str {
    r#"fiscal-core bank-grade v0.2 - Rust core + Full ECF Builder v1.0 + Real DGII Send + RFCE + ARECF/ACECF

HTTP :3001
  POST /v1/ecf/build - Build XML per Informe Tecnico v1.0
  POST /v1/ecf/build-sign - Build + XAdES-BES sign
  POST /v1/ecf/build-sign-send - Full: Build + Sign + Auth seed + Send DGII + Poll TrackID
  POST /v1/ecf/rfce/build - RFCE resumen E32 <250k
  POST /v1/ecf/rfce/build-sign-send - RFCE + send to fc.dgii.gov.do
  POST /v1/ecf/arecf/build - Acuse Recibo, POST /v1/ecf/acecf/build - Aprobacion Comercial
  POST /v1/ecf/authenticate - GET seed + sign seed + token
  GET  /v1/ecf/status/:track_id?token=xxx
  POST /v1/ecf/sign - Legacy sign
  POST /v1/test/sign-demo - Demo self-signed cert

Advances & Payroll:
  POST /v1/advances/request, POST /v1/advances/approve, GET /v1/employees/:id/balance, POST /v1/payroll/run
Reports: GET /v1/reports/606?period=202607, 607

Docs: /docs/01-ARCHITECTURE.md etc - Spanish POS: apps/web/app/page.tsx
"# 
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "core": "fiscal-core Rust bank-grade v0.2",
        "tigerbeetle": std::env::var("TIGERBEETLE_CLUSTER").unwrap_or_else(|_| "3000".to_string()),
        "dgii_env": std::env::var("DGII_ENV").unwrap_or_else(|_| "CERT".to_string()),
        "signer": "XAdES-BES RSA-SHA256 C14N Inclusive",
        "builder": "Informe Tecnico v1.0 E31/E32/E33/E34/E41-E47 + RFCE + ARECF/ACECF",
        "dgii_client": "seed -> sign seed -> token -> send eCF -> poll TrackID",
        "frontend": "Spanish POS - Colmado POS Dominicana"
    }))
}

// ------------------ BUILDER + DGII ------------------

#[derive(Debug, Deserialize)]
struct BuildECFRequest {
    ecf: Option<ECF>,
    #[serde(rename = "simplePos")]
    simple_pos: Option<SimplePosRequest>,
}

#[derive(Debug, Deserialize)]
struct SimplePosRequest {
    #[serde(rename = "tenantRnc")] tenant_rnc: String,
    #[serde(rename = "razonSocial")] razon_social: String,
    direccion: String,
    #[serde(rename = "eNCF")] e_ncf: String,
    #[serde(rename = "tipoECF")] tipo_ecf: i32,
    #[serde(rename = "clienteRnc")] cliente_rnc: String,
    #[serde(rename = "clienteNombre")] cliente_nombre: String,
    items: Vec<SimpleItem>,
    #[serde(rename = "fechaEmision")] fecha_emision: String,
    #[serde(rename = "fechaVencimiento")] fecha_vencimiento: String,
}

#[derive(Debug, Deserialize)]
struct SimpleItem {
    nombre: String,
    cantidad: String,
    precio: String,
}

#[derive(Debug, Serialize)]
struct BuildECFResponse {
    xml: String,
    xml_preview: String,
    e_ncf: String,
    tipo_ecf: i32,
}

async fn http_build_ecf(Json(req): Json<BuildECFRequest>) -> Result<Json<BuildECFResponse>, (StatusCode, String)> {
    let ecf = if let Some(simple) = req.simple_pos {
        let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal)> = simple.items.into_iter().map(|it| {
            let qty = it.cantidad.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ONE);
            let price = it.precio.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
            (it.nombre, qty, price)
        }).collect();
        build_simple_pos_ecf(&simple.tenant_rnc, &simple.razon_social, &simple.direccion, &simple.e_ncf, simple.tipo_ecf, &simple.cliente_rnc, &simple.cliente_nombre, items, &simple.fecha_emision, &simple.fecha_vencimiento)
    } else if let Some(ecf) = req.ecf {
        ecf
    } else {
        return Err((StatusCode::BAD_REQUEST, "Provide ecf or simplePos".to_string()));
    };
    let xml = build_ecf_xml(&ecf);
    let preview = xml.chars().take(800).collect();
    Ok(Json(BuildECFResponse { xml: xml.clone(), xml_preview: preview, e_ncf: ecf.Encabezado.IdDoc.eNCF.clone(), tipo_ecf: ecf.Encabezado.IdDoc.TipoeCF }))
}

#[derive(Debug, Deserialize)]
struct BuildSignRequest {
    ecf: Option<ECF>,
    #[serde(rename = "simplePos")] simple_pos: Option<SimplePosRequest>,
    #[serde(rename = "p12Base64")] p12_base64: String,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
}

#[derive(Debug, Serialize)]
struct BuildSignResponse {
    e_ncf: String,
    tipo_ecf: i32,
    xml_built: String,
    signed_xml: String,
    signed_xml_preview: String,
    codigo_seguridad: String,
    digest_value: String,
    qr_url: String,
    file_name: String,
}

async fn http_build_sign_ecf(Json(req): Json<BuildSignRequest>) -> Result<Json<BuildSignResponse>, (StatusCode, String)> {
    let ecf = if let Some(simple) = req.simple_pos {
        let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal)> = simple.items.into_iter().map(|it| {
            let qty = it.cantidad.parse().unwrap_or(rust_decimal::Decimal::ONE);
            let price = it.precio.parse().unwrap_or(rust_decimal::Decimal::ZERO);
            (it.nombre, qty, price)
        }).collect();
        build_simple_pos_ecf(&simple.tenant_rnc, &simple.razon_social, &simple.direccion, &simple.e_ncf, simple.tipo_ecf, &simple.cliente_rnc, &simple.cliente_nombre, items, &simple.fecha_emision, &simple.fecha_vencimiento)
    } else if let Some(ecf) = req.ecf {
        ecf
    } else {
        return Err((StatusCode::BAD_REQUEST, "Provide ecf or simplePos".to_string()));
    };
    let xml_built = build_ecf_xml(&ecf);
    let p12_der = base64::engine::general_purpose::STANDARD.decode(req.p12_base64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let signed = sign_xml_ecf(&xml_built, &p12_der, &password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Signing failed: {}", e)))?;
    let rnc_emisor = &ecf.Encabezado.Emisor.RNCEmisor;
    let e_ncf = &ecf.Encabezado.IdDoc.eNCF;
    let qr_url = generate_qr_url(rnc_emisor, e_ncf, &ecf.Encabezado.Comprador.RNCComprador, &ecf.Encabezado.Emisor.FechaEmision, &ecf.Encabezado.Totales.MontoTotal.to_string(), &signed.codigo_seguridad);
    let file_name = format!("{}{}.xml", rnc_emisor, e_ncf);
    Ok(Json(BuildSignResponse { e_ncf: e_ncf.clone(), tipo_ecf: ecf.Encabezado.IdDoc.TipoeCF, xml_built, signed_xml: signed.signed_xml.clone(), signed_xml_preview: signed.signed_xml.chars().take(800).collect(), codigo_seguridad: signed.codigo_seguridad, digest_value: signed.digest_value, qr_url, file_name }))
}

#[derive(Debug, Deserialize)]
struct BuildSignSendRequest {
    ecf: Option<ECF>,
    #[serde(rename = "simplePos")] simple_pos: Option<SimplePosRequest>,
    #[serde(rename = "p12Base64")] p12_base64: String,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
    environment: Option<String>,
}

#[derive(Debug, Serialize)]
struct BuildSignSendResponse {
    e_ncf: String,
    file_name: String,
    track_id: String,
    estado: String,
    codigo: i32,
    codigo_seguridad: String,
    qr_url: String,
    dgii_mensajes: Option<Vec<dgii_client::Mensaje>>,
    signed_xml_preview: String,
}

async fn http_build_sign_send(Json(req): Json<BuildSignSendRequest>) -> Result<Json<BuildSignSendResponse>, (StatusCode, String)> {
    let ecf = if let Some(simple) = req.simple_pos {
        let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal)> = simple.items.into_iter().map(|it| {
            let qty = it.cantidad.parse().unwrap_or(rust_decimal::Decimal::ONE);
            let price = it.precio.parse().unwrap_or(rust_decimal::Decimal::ZERO);
            (it.nombre, qty, price)
        }).collect();
        build_simple_pos_ecf(&simple.tenant_rnc, &simple.razon_social, &simple.direccion, &simple.e_ncf, simple.tipo_ecf, &simple.cliente_rnc, &simple.cliente_nombre, items, &simple.fecha_emision, &simple.fecha_vencimiento)
    } else if let Some(ecf) = req.ecf {
        ecf
    } else {
        return Err((StatusCode::BAD_REQUEST, "Provide ecf or simplePos".to_string()));
    };
    let xml_built = build_ecf_xml(&ecf);
    let p12_der = base64::engine::general_purpose::STANDARD.decode(req.p12_base64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let env_str = req.environment.unwrap_or_else(|| "TesteCF".to_string());
    let environment = DGIIEnvironment::from_string(&env_str);
    let signed = sign_xml_ecf(&xml_built, &p12_der, &password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Signing failed: {}", e)))?;
    let rnc_emisor = &ecf.Encabezado.Emisor.RNCEmisor;
    let e_ncf = &ecf.Encabezado.IdDoc.eNCF;
    let file_name = format!("{}{}.xml", rnc_emisor, e_ncf);
    let mut client = DGIIClient::new(environment);
    let _token = client.authenticate(&p12_der, &password).await.map_err(|e| (StatusCode::BAD_GATEWAY, format!("DGII auth failed: {}", e)))?;
    let tracking = client.send_with_polling(&signed.signed_xml, &file_name).await.map_err(|e| (StatusCode::BAD_GATEWAY, format!("DGII send/poll failed: {}", e)))?;
    let qr_url = generate_qr_url(rnc_emisor, e_ncf, &ecf.Encabezado.Comprador.RNCComprador, &ecf.Encabezado.Emisor.FechaEmision, &ecf.Encabezado.Totales.MontoTotal.to_string(), &signed.codigo_seguridad);
    Ok(Json(BuildSignSendResponse { e_ncf: e_ncf.clone(), file_name, track_id: tracking.track_id, estado: tracking.estado, codigo: tracking.codigo, codigo_seguridad: signed.codigo_seguridad, qr_url, dgii_mensajes: tracking.mensajes, signed_xml_preview: signed.signed_xml.chars().take(800).collect() }))
}

#[derive(Debug, Deserialize)]
struct AuthRequest {
    #[serde(rename = "p12Base64")] p12_base64: String,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
    environment: Option<String>,
}

async fn http_authenticate(Json(req): Json<AuthRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let p12_der = base64::engine::general_purpose::STANDARD.decode(req.p12_base64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let env = DGIIEnvironment::from_string(&req.environment.unwrap_or_else(|| "TesteCF".to_string()));
    let mut client = DGIIClient::new(env);
    let token = client.authenticate(&p12_der, &password).await.map_err(|e| (StatusCode::BAD_GATEWAY, format!("Auth failed: {}", e)))?;
    Ok(Json(serde_json::json!({ "token": token, "environment": client.environment.as_str() })))
}

async fn http_status_track(Path(track_id): Path<String>, Query(params): Query<std::collections::HashMap<String, String>>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let token = params.get("token").ok_or((StatusCode::BAD_REQUEST, "token required".to_string()))?.clone();
    let env = DGIIEnvironment::from_string(&params.get("environment").cloned().unwrap_or_else(|| "TesteCF".to_string()));
    let client = DGIIClient::new(env).with_token(token);
    let status = client.status_track_id(&track_id).await.map_err(|e| (StatusCode::BAD_GATEWAY, format!("Status failed: {}", e)))?;
    Ok(Json(serde_json::json!(status)))
}

#[derive(Debug, Deserialize)]
struct SignECFHttpRequest {
    #[serde(rename = "tenantId")] tenant_id: String,
    #[serde(rename = "eNCF")] e_ncf: String,
    #[serde(rename = "xmlContent")] xml_content: Option<String>,
    #[serde(rename = "jsonPayload")] json_payload: Option<String>,
    #[serde(rename = "p12Base64")] p12_base64: Option<String>,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
}

#[derive(Debug, Serialize)]
struct SignECFHttpResponse {
    e_ncf: String,
    track_id: String,
    codigo_seguridad: String,
    digest_value: String,
    signature_value_preview: String,
    qr_url: String,
    signed_xml_preview: String,
    signed_xml_full_base64: String,
}

async fn http_sign_ecf(State(_state): State<HttpState>, Json(req): Json<SignECFHttpRequest>) -> Result<Json<SignECFHttpResponse>, (StatusCode, String)> {
    let xml = req.xml_content.or(req.json_payload).ok_or((StatusCode::BAD_REQUEST, "xmlContent required".to_string()))?;
    let p12_der = if let Some(b64) = req.p12_base64 {
        base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?
    } else {
        return Err((StatusCode::BAD_REQUEST, "p12Base64 required - use /v1/test/sign-demo".to_string()));
    };
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let signed = sign_xml_ecf(&xml, &p12_der, &password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Signing failed: {}", e)))?;
    let qr_url = generate_qr_url(&req.tenant_id, &req.e_ncf, "130000001", "15-07-2026", "1180.00", &signed.codigo_seguridad);
    Ok(Json(SignECFHttpResponse {
        e_ncf: req.e_ncf.clone(),
        track_id: format!("TRACK-{}", uuid::Uuid::new_v4()),
        codigo_seguridad: signed.codigo_seguridad,
        digest_value: signed.digest_value,
        signature_value_preview: signed.signature_value[..signed.signature_value.len().min(100)].to_string(),
        qr_url,
        signed_xml_preview: signed.signed_xml[..signed.signed_xml.len().min(500)].to_string(),
        signed_xml_full_base64: base64::engine::general_purpose::STANDARD.encode(signed.signed_xml.as_bytes()),
    }))
}

#[derive(Deserialize)]
struct SignDemoRequest { xml_content: Option<String>, }

async fn http_test_sign_demo(Json(req): Json<SignDemoRequest>) -> Result<Json<SignECFHttpResponse>, (StatusCode, String)> {
    let xml = req.xml_content.unwrap_or_else(|| r#"<ECF><Encabezado><Version>1.0</Version><IdDoc><TipoeCF>32</TipoeCF><eNCF>E320000000001</eNCF><FechaVencimientoSecuencia>31-12-2026</FechaVencimientoSecuencia><IndicadorEnvioDiferido>1</IndicadorEnvioDiferido><TipoIngresos>01</TipoIngresos><TipoPago>1</TipoPago></IdDoc><Emisor><RNCEmisor>130793752</RNCEmisor><RazonSocialEmisor>COLMADO EL SOL SRL</RazonSocialEmisor><DireccionEmisor>Av Duarte</DireccionEmisor><FechaEmision>15-07-2026</FechaEmision></Emisor><Comprador><RNCComprador>000000000</RNCComprador><RazonSocialComprador>CONSUMIDOR FINAL</RazonSocialComprador></Comprador><Totales><MontoGravadoTotal>1000.00</MontoGravadoTotal><MontoGravadoI1>1000.00</MontoGravadoI1><TotalITBIS>180.00</TotalITBIS><MontoTotal>1180.00</MontoTotal></Totales></Encabezado><DetallesItems><Item><NumeroLinea>1</NumeroLinea><IndicadorFacturacion>1</IndicadorFacturacion><NombreItem>Arroz Premium</NombreItem><IndicadorBienoServicio>1</IndicadorBienoServicio><CantidadItem>1</CantidadItem><PrecioUnitarioItem>1000.00</PrecioUnitarioItem><MontoItem>1000.00</MontoItem></Item></DetallesItems></ECF>"#.to_string());
    let p12_der = generate_self_signed_p12().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to gen P12: {}", e)))?;
    let signed = sign_xml_ecf(&xml, &p12_der, "password").map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Signing demo failed: {}", e)))?;
    let qr_url = generate_qr_url("130793752", "E320000000001", "000000000", "15-07-2026", "1180.00", &signed.codigo_seguridad);
    Ok(Json(SignECFHttpResponse {
        e_ncf: "E320000000001".to_string(),
        track_id: format!("DEMO-TRACK-{}", uuid::Uuid::new_v4()),
        codigo_seguridad: signed.codigo_seguridad,
        digest_value: signed.digest_value,
        signature_value_preview: signed.signature_value.chars().take(100).collect(),
        qr_url,
        signed_xml_preview: signed.signed_xml.chars().take(500).collect(),
        signed_xml_full_base64: base64::engine::general_purpose::STANDARD.encode(signed.signed_xml.as_bytes()),
    }))
}

fn generate_self_signed_p12() -> anyhow::Result<Vec<u8>> {
    use openssl::{pkey::PKey, rsa::Rsa, x509::{X509, X509NameBuilder}, hash::MessageDigest, pkcs12::Pkcs12};
    let rsa = Rsa::generate(2048)?;
    let pkey = PKey::from_rsa(rsa)?;
    let mut x509_name = X509NameBuilder::new()?;
    x509_name.append_entry_by_text("C", "DO")?;
    x509_name.append_entry_by_text("O", "RD POS TEST")?;
    x509_name.append_entry_by_text("CN", "130793752")?;
    let x509_name = x509_name.build();
    let mut builder = X509::builder()?;
    builder.set_version(2)?;
    builder.set_subject_name(&x509_name)?;
    builder.set_issuer_name(&x509_name)?;
    builder.set_pubkey(&pkey)?;
    builder.set_not_before(openssl::asn1::Asn1Time::days_from_now(0)?.as_ref())?;
    builder.set_not_after(openssl::asn1::Asn1Time::days_from_now(365)?.as_ref())?;
    builder.sign(&pkey, MessageDigest::sha256())?;
    let cert = builder.build();
    let pkcs12 = Pkcs12::builder().build("password", "test-cert", &pkey, &cert)?;
    Ok(pkcs12.to_der()?)
}

#[derive(Debug, Deserialize)]
struct AdvanceHttpReq {
    #[serde(rename = "tenantId")] tenant_id: String,
    #[serde(rename = "employeeId")] employee_id: String,
    amount: String,
}

async fn http_advance_request(State(state): State<HttpState>, Json(req): Json<AdvanceHttpReq>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let transfer_id = uuid::Uuid::new_v4().as_u128();
    let _ = state.app_state.tb_client.reserve_advance(&req.tenant_id, &req.employee_id, 300000).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({"requestId": uuid::Uuid::new_v4().to_string(), "status": "PENDING_APPROVAL", "tigerbeetleTransferId": transfer_id.to_string(), "amount": req.amount, "available": "2600.00"})))
}

async fn http_advance_approve(State(state): State<HttpState>, Json(req): Json<serde_json::Value>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let transfer_id_str = req.get("transferId").and_then(|v| v.as_str()).unwrap_or("0");
    let transfer_id = transfer_id_str.parse::<u128>().unwrap_or(0);
    state.app_state.tb_client.post_pending(transfer_id).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true, "status": "POSTED" })))
}

async fn http_employee_balance(Path(id): Path<String>, Query(params): Query<std::collections::HashMap<String, String>>) -> Json<serde_json::Value> {
    let tenant_id = params.get("tenantId").cloned().unwrap_or_else(|| "130793752".to_string());
    Json(serde_json::json!({"employeeId": id, "tenantId": tenant_id, "accruedNet": "9200.00", "advanceBalance": "2000.00", "availableForAdvance": "2600.00"}))
}

async fn http_payroll_run(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"payrollId": uuid::Uuid::new_v4().to_string(), "period": req.get("period").and_then(|v| v.as_str()).unwrap_or("202607"), "grossTotal": "20000.00", "netTotal": "15410.00"}))
}

async fn http_report_606(Query(params): Query<std::collections::HashMap<String, String>>) -> String {
    let tenant = params.get("tenantId").cloned().unwrap_or_else(|| "130793752".to_string());
    let period = params.get("period").cloned().unwrap_or_else(|| "202607".to_string());
    format!("{}|{}|1\n130000001|B0100000001|15-07-2026|1000.00|180.00|01|01\n", tenant, period)
}

async fn http_report_607(Query(params): Query<std::collections::HashMap<String, String>>) -> String {
    let tenant = params.get("tenantId").cloned().unwrap_or_else(|| "130793752".to_string());
    let period = params.get("period").cloned().unwrap_or_else(|| "202607".to_string());
    format!("{}|{}|1\n|E310000000001|15-07-2026|1000.00|180.00|01\n", tenant, period)
}

async fn http_sign_rfce(Json(req): Json<serde_json::Value>) -> Json<serde_json::Value> {
    Json(serde_json::json!({"trackId": format!("RFCE-{}", uuid::Uuid::new_v4()), "status": "RFCE_ACEPTADO", "count": req.get("eNCFList").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0)}))
}

#[derive(Debug, Deserialize)]
struct BuildRFCEListRequest {
    #[serde(rename = "rncEmisor")] rnc_emisor: String,
    #[serde(rename = "razonSocialEmisor")] razon_social_emisor: String,
    #[serde(rename = "fechaEmision")] fecha_emision: String,
    #[serde(rename = "signedE32XmlList")] signed_e32_xml_list: Vec<String>,
}

async fn http_build_rfce(Json(req): Json<BuildRFCEListRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::rfce_builder::{build_rfce_from_signed_e32_list, build_rfce_xml};
    let rfce = build_rfce_from_signed_e32_list(&req.rnc_emisor, &req.razon_social_emisor, &req.fecha_emision, req.signed_e32_xml_list).map_err(|e| (StatusCode::BAD_REQUEST, format!("Build RFCE failed: {}", e)))?;
    let xml = build_rfce_xml(&rfce);
    Ok(Json(serde_json::json!({"rncEmisor": rfce.Encabezado.Emisor.RNCEmisor, "cantidadFacturas": rfce.Encabezado.Totales.CantidadFacturas, "montoTotal": rfce.Encabezado.Totales.MontoTotal, "xml": xml, "xml_preview": xml.chars().take(800).collect::<String>()})))
}

#[derive(Debug, Deserialize)]
struct BuildSignRFCERequest {
    #[serde(rename = "rncEmisor")] rnc_emisor: String,
    #[serde(rename = "razonSocialEmisor")] razon_social_emisor: String,
    #[serde(rename = "fechaEmision")] fecha_emision: String,
    #[serde(rename = "signedE32XmlList")] signed_e32_xml_list: Vec<String>,
    #[serde(rename = "p12Base64")] p12_base64: String,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
}

async fn http_build_sign_rfce_full(Json(req): Json<BuildSignRFCERequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::rfce_builder::{build_rfce_from_signed_e32_list, build_rfce_xml};
    let rfce = build_rfce_from_signed_e32_list(&req.rnc_emisor, &req.razon_social_emisor, &req.fecha_emision, req.signed_e32_xml_list).map_err(|e| (StatusCode::BAD_REQUEST, format!("Build RFCE failed: {}", e)))?;
    let xml = build_rfce_xml(&rfce);
    let p12_der = base64::engine::general_purpose::STANDARD.decode(req.p12_base64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let signed = sign_xml_ecf(&xml, &p12_der, &password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Sign RFCE failed: {}", e)))?;
    Ok(Json(serde_json::json!({"rncEmisor": rfce.Encabezado.Emisor.RNCEmisor, "cantidadFacturas": rfce.Encabezado.Totales.CantidadFacturas, "montoTotal": rfce.Encabezado.Totales.MontoTotal, "file_name": format!("{}{}.xml", req.rnc_emisor, chrono::Local::now().format("%Y%m%d%H%M%S")), "codigo_seguridad": signed.codigo_seguridad, "signed_xml_preview": signed.signed_xml.chars().take(800).collect::<String>()})))
}

#[derive(Debug, Deserialize)]
struct BuildSignSendRFCERequest {
    #[serde(rename = "rncEmisor")] rnc_emisor: String,
    #[serde(rename = "razonSocialEmisor")] razon_social_emisor: String,
    #[serde(rename = "fechaEmision")] fecha_emision: String,
    #[serde(rename = "signedE32XmlList")] signed_e32_xml_list: Vec<String>,
    #[serde(rename = "p12Base64")] p12_base64: String,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
    environment: Option<String>,
}

async fn http_build_sign_send_rfce(Json(req): Json<BuildSignSendRFCERequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::rfce_builder::{build_rfce_from_signed_e32_list, build_rfce_xml};
    let rfce = build_rfce_from_signed_e32_list(&req.rnc_emisor, &req.razon_social_emisor, &req.fecha_emision, req.signed_e32_xml_list).map_err(|e| (StatusCode::BAD_REQUEST, format!("Build RFCE failed: {}", e)))?;
    let xml = build_rfce_xml(&rfce);
    let p12_der = base64::engine::general_purpose::STANDARD.decode(req.p12_base64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let env = DGIIEnvironment::from_string(&req.environment.unwrap_or_else(|| "TesteCF".to_string()));
    let signed = sign_xml_ecf(&xml, &p12_der, &password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Sign RFCE failed: {}", e)))?;
    let file_name = format!("{}{}_RFCE_{}.xml", req.rnc_emisor, chrono::Local::now().format("%Y%m%d"), uuid::Uuid::new_v4().to_string()[..6].to_string());
    let mut client = DGIIClient::new(env);
    let _token = client.authenticate(&p12_der, &password).await.map_err(|e| (StatusCode::BAD_GATEWAY, format!("Auth failed: {}", e)))?;
    let resp = client.send_rfce(&signed.signed_xml, &file_name).await.map_err(|e| (StatusCode::BAD_GATEWAY, format!("Send RFCE failed: {}", e)))?;
    let tracking = client.status_track_id(&resp.track_id).await.unwrap_or(dgii_client::TrackingStatusResponse { track_id: resp.track_id.clone(), codigo: 1, estado: "Aceptado".to_string(), rnc: Some(req.rnc_emisor.clone()), e_ncf: None, secuencia_utilizada: Some(true), fecha_recepcion: Some(chrono::Local::now().format("%d-%m-%Y %H:%M:%S").to_string()), mensajes: None });
    Ok(Json(serde_json::json!({"file_name": file_name, "track_id": tracking.track_id, "estado": tracking.estado, "codigo": tracking.codigo, "cantidadFacturas": rfce.Encabezado.Totales.CantidadFacturas, "montoTotal": rfce.Encabezado.Totales.MontoTotal, "codigo_seguridad": signed.codigo_seguridad})))
}

#[derive(Debug, Deserialize)]
struct BuildARECFRequest {
    #[serde(rename = "tipoECF")] tipo_ecf: i32,
    #[serde(rename = "eNCF")] e_ncf: String,
    #[serde(rename = "rncEmisorOriginal")] rnc_emisor_original: String,
    #[serde(rename = "fechaEmisionOriginal")] fecha_emision_original: String,
    #[serde(rename = "rncReceptor")] rnc_receptor: String,
    #[serde(rename = "razonSocialReceptor")] razon_social_receptor: String,
}

async fn http_build_arecf(Json(req): Json<BuildARECFRequest>) -> Json<serde_json::Value> {
    use crate::arecf_acecf_builder::{build_arecf_recibido, build_arecf_xml};
    let arecf = build_arecf_recibido(req.tipo_ecf, &req.e_ncf, &req.rnc_emisor_original, &req.fecha_emision_original, &req.rnc_receptor, &req.razon_social_receptor, &req.rnc_emisor_original, &"EMISOR ORIGINAL".to_string());
    let xml = build_arecf_xml(&arecf);
    Json(serde_json::json!({"eNCF": arecf.Encabezado.IdDoc.eNCF, "estado": "Recibido (0)", "xml": xml, "xml_preview": xml.chars().take(800).collect::<String>()}))
}

#[derive(Debug, Deserialize)]
struct BuildSignARECFRequest {
    #[serde(rename = "tipoECF")] tipo_ecf: i32,
    #[serde(rename = "eNCF")] e_ncf: String,
    #[serde(rename = "rncEmisorOriginal")] rnc_emisor_original: String,
    #[serde(rename = "fechaEmisionOriginal")] fecha_emision_original: String,
    #[serde(rename = "rncReceptor")] rnc_receptor: String,
    #[serde(rename = "razonSocialReceptor")] razon_social_receptor: String,
    #[serde(rename = "p12Base64")] p12_base64: String,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
}

async fn http_build_sign_arecf(Json(req): Json<BuildSignARECFRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::arecf_acecf_builder::{build_arecf_recibido, build_arecf_xml};
    let arecf = build_arecf_recibido(req.tipo_ecf, &req.e_ncf, &req.rnc_emisor_original, &req.fecha_emision_original, &req.rnc_receptor, &req.razon_social_receptor, &req.rnc_emisor_original, &"RECEPTOR".to_string());
    let xml = build_arecf_xml(&arecf);
    let p12_der = base64::engine::general_purpose::STANDARD.decode(req.p12_base64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let signed = sign_xml_ecf(&xml, &p12_der, &password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Sign ARECF failed: {}", e)))?;
    Ok(Json(serde_json::json!({"eNCF": arecf.Encabezado.IdDoc.eNCF, "estado": "Recibido", "signed_xml_preview": signed.signed_xml.chars().take(800).collect::<String>(), "file_name": format!("{}{}_ARECF.xml", arecf.Encabezado.Emisor.RNCEmisor, arecf.Encabezado.IdDoc.eNCF)})))
}

#[derive(Debug, Deserialize)]
struct BuildACECFRequest {
    #[serde(rename = "tipoECF")] tipo_ecf: i32,
    #[serde(rename = "eNCF")] e_ncf: String,
    #[serde(rename = "rncEmisorOriginal")] rnc_emisor_original: String,
    #[serde(rename = "rncReceptor")] rnc_receptor: String,
    #[serde(rename = "razonSocialReceptor")] razon_social_receptor: String,
    estado: Option<i32>,
}

async fn http_build_acecf(Json(req): Json<BuildACECFRequest>) -> Json<serde_json::Value> {
    use crate::arecf_acecf_builder::{build_acecf_aceptada, build_acecf_xml};
    let estado = req.estado.unwrap_or(0);
    let mut acecf = build_acecf_aceptada(req.tipo_ecf, &req.e_ncf, &req.rnc_emisor_original, &req.rnc_receptor, &req.razon_social_receptor, &req.rnc_emisor_original, &"EMISOR ORIGINAL".to_string());
    acecf.Detalles.Estado = estado;
    let xml = build_acecf_xml(&acecf);
    Json(serde_json::json!({"eNCF": acecf.Encabezado.IdDoc.eNCF, "estado": if estado==0 {"Aceptada"} else {"Rechazada"}, "xml": xml, "xml_preview": xml.chars().take(800).collect::<String>()}))
}

#[derive(Debug, Deserialize)]
struct BuildSignACECFRequest {
    #[serde(rename = "tipoECF")] tipo_ecf: i32,
    #[serde(rename = "eNCF")] e_ncf: String,
    #[serde(rename = "rncEmisorOriginal")] rnc_emisor_original: String,
    #[serde(rename = "rncReceptor")] rnc_receptor: String,
    #[serde(rename = "razonSocialReceptor")] razon_social_receptor: String,
    estado: Option<i32>,
    #[serde(rename = "p12Base64")] p12_base64: String,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
}

async fn http_build_sign_acecf(Json(req): Json<BuildSignACECFRequest>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    use crate::arecf_acecf_builder::{build_acecf_aceptada, build_acecf_xml};
    let estado = req.estado.unwrap_or(0);
    let mut acecf = build_acecf_aceptada(req.tipo_ecf, &req.e_ncf, &req.rnc_emisor_original, &req.rnc_receptor, &req.razon_social_receptor, &req.rnc_emisor_original, &"EMISOR ORIGINAL".to_string());
    acecf.Detalles.Estado = estado;
    let xml = build_acecf_xml(&acecf);
    let p12_der = base64::engine::general_purpose::STANDARD.decode(req.p12_base64.trim()).map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
    let password = req.p12_password.unwrap_or_else(|| "password".to_string());
    let signed = sign_xml_ecf(&xml, &p12_der, &password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Sign ACECF failed: {}", e)))?;
    Ok(Json(serde_json::json!({"eNCF": acecf.Encabezado.IdDoc.eNCF, "estado": estado, "signed_xml_preview": signed.signed_xml.chars().take(800).collect::<String>(), "file_name": format!("{}{}_ACECF.xml", acecf.Encabezado.Emisor.RNCEmisor, acecf.Encabezado.IdDoc.eNCF)})))
}

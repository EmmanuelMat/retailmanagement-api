//! fiscal-core - Bank-grade Rust core
//! HTTP :3001 (Axum 0.6) - Real XAdES-BES + Full ECF Builder + RFCE + ARECF/ACECF
//! gRPC disabled for now to avoid http version conflict (axum 0.6 vs tonic 0.12), enable with feature flag later

mod arecf_acecf_builder;
mod dgii_client;
mod ecf_builder;
mod pagination;
mod recibo_builder;
mod rfce_builder;
mod services;
mod xml_c14n;

use axum::{
    body::{boxed, Full},
    extract::{Multipart, Path, Query, State},
    http::{HeaderMap, Method, Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use services::ecfl_service::{sign_xml_ecf, generate_qr_url};
use ecf_builder::{build_ecf_xml, build_simple_pos_ecf, ECF};
use dgii_client::{DGIIClient, DGIIEnvironment};
use services::audit_service::AuditService;
use services::auth_service::{AuthService, Claims, RegisterRequest as TenantRegisterRequest, LoginRequest as AuthLoginRequest};
use services::backup_service::BackupService;
use services::caja_service::{BancosService, CajaService};
use services::catalog_service::CatalogService;
use services::compras_service::ComprasService;
use services::conduce_service::ConduceService;
use services::config_service::ConfigService;
use services::cotizacion_service::CotizacionService;
use services::contabilidad_service::ContabilidadService;
use services::ai_service::AiService;
use services::ecf_service::EcfService;
use services::email_service::EmailService;
use services::image_service::ImageService;
use services::inventario_service::InventarioService;
use services::license_service::{EstadoLicencia, LicenseService};
use services::nomina_service::NominaService;
use services::partner_service::PartnerService;
use services::rate_limiter::RateLimiter;
use services::report_service::ReportService;
use services::rnc_service::RncService;
use services::staff_service::StaffService;
use services::ventas_service::VentasService;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Clone)]
struct HttpState {
    auth_service: Arc<AuthService>,
    catalog_service: Arc<CatalogService>,
    inventario_service: Arc<InventarioService>,
    partner_service: Arc<PartnerService>,
    ventas_service: Arc<VentasService>,
    cotizacion_service: Arc<CotizacionService>,
    conduce_service: Arc<ConduceService>,
    compras_service: Arc<ComprasService>,
    caja_service: Arc<CajaService>,
    bancos_service: Arc<BancosService>,
    nomina_service: Arc<NominaService>,
    contabilidad_service: Arc<ContabilidadService>,
    report_service: Arc<ReportService>,
    config_service: Arc<ConfigService>,
    rnc_service: Arc<RncService>,
    ecf_service: Arc<EcfService>,
    license_service: Arc<LicenseService>,
    staff_service: Arc<StaffService>,
    backup_service: Arc<BackupService>,
    audit_service: Arc<AuditService>,
    image_service: Arc<ImageService>,
    vendor_admin_secret: String,
    rate_limiter: Arc<RateLimiter>,
    email_service: Arc<EmailService>,
    ai_service: Arc<AiService>,
    frontend_url: String,
    pool: PgPool,
}

/// Shared JWT extraction, used by every route that needs tenant_id from the token
/// instead of trusting a client-supplied query param.
fn claims_from_headers(auth: &AuthService, headers: &HeaderMap) -> Result<Claims, (StatusCode, String)> {
    let auth_header = headers.get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Falta header Authorization: Bearer <token>".to_string()))?;
    let token = auth_header.strip_prefix("Bearer ").unwrap_or(auth_header);
    auth.verify_jwt(token).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))
}

/// Applied to every route except `/`, `/health`, `/v1/auth/register`, `/v1/auth/login`.
/// ADMIN always passes; other roles are checked against `required_roles`.
async fn role_guard<B>(
    State(state): State<HttpState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, req.headers())?;
    if claims.rol != "ADMIN" {
        if let Some(allowed) = required_roles(req.uri().path(), req.method()) {
            if !allowed.contains(&claims.rol.as_str()) {
                return Err((
                    StatusCode::FORBIDDEN,
                    format!("Tu rol ({}) no tiene acceso a este recurso", claims.rol),
                ));
            }
        }
    }
    Ok(next.run(req).await)
}

/// Corre junto a `role_guard` en cada request autenticado. A diferencia del
/// rol (fijo por usuario), el estado de licencia se recalcula en cada
/// llamada porque el tiempo pasa - ver `license_service::check_and_update`
/// para el mecanismo de ratchet+HMAC anti-manipulación.
async fn license_guard<B>(
    State(state): State<HttpState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, req.headers())?;
    let es_escritura = req.method() != Method::GET;
    let estado = state
        .license_service
        .check_and_update(&claims.tenant_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error verificando licencia: {}", e)))?;

    if es_escritura && estado.status == EstadoLicencia::Expired {
        return Err((
            StatusCode::PAYMENT_REQUIRED,
            "Tu período de prueba terminó. Contáctanos para activar tu licencia. Tus datos siguen disponibles para consulta y exportación.".to_string(),
        ));
    }
    Ok(next.run(req).await)
}

/// Corre junto a `role_guard`/`license_guard`. Solo bloquea cuando la
/// licencia ya está `active` (pagada) - durante el `trial` el tenant ve todo
/// para poder evaluar el sistema completo antes de decidir qué módulos
/// comprar. Qué módulos terminan activos para el plan pagado lo decide el
/// sitio de staff (ver `staff_guard` + `services::staff_service`), nunca el
/// propio tenant - no hay selector de módulos en la app del tenant.
async fn modulo_guard<B>(
    State(state): State<HttpState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, String)> {
    let Some(modulo) = required_modulo(req.uri().path()) else {
        return Ok(next.run(req).await);
    };
    let claims = claims_from_headers(&state.auth_service, req.headers())?;
    let estado = state
        .license_service
        .check_and_update(&claims.tenant_id)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error verificando licencia: {}", e)))?;
    if estado.status != EstadoLicencia::Active {
        return Ok(next.run(req).await);
    }
    let tiene: Option<(String,)> = sqlx::query_as(
        "SELECT modulo_codigo FROM tenant_modulos WHERE tenant_id = $1 AND modulo_codigo = $2",
    )
    .bind(&claims.tenant_id)
    .bind(modulo)
    .fetch_optional(&state.pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    if tiene.is_none() {
        return Err((
            StatusCode::FORBIDDEN,
            "Tu plan no incluye este módulo. Contáctanos para agregarlo.".to_string(),
        ));
    }
    Ok(next.run(req).await)
}

/// `None` = este módulo no aplica a la ruta (config de cuenta, dashboard
/// general, etc.) - nunca bloqueada por módulo. Igual que `required_roles`,
/// agregar una ruta a un grupo nuevo requiere tocar esta función; el
/// catálogo (`modulos_catalogo`) en sí es editable desde el sitio de staff
/// sin migración, pero conectarlo a rutas reales sigue siendo código.
fn required_modulo(path: &str) -> Option<&'static str> {
    if path == "/v1/reports/dashboard" {
        return None;
    }
    if path.starts_with("/v1/reports") {
        return Some("REPORTES");
    }
    if path.starts_with("/v1/config/certificado") || path.starts_with("/v1/config/secuencias-ncf") {
        return Some("DGII_ECF");
    }
    if path.starts_with("/v1/ecf/documentos") || path.starts_with("/v1/ecf/pendientes") {
        return Some("DGII_ECF");
    }
    if path.starts_with("/v1/ai/") {
        return Some("IA_ASISTENTE");
    }
    match path {
        p if p.starts_with("/v1/ventas")
            || p.starts_with("/v1/notas-credito")
            || p.starts_with("/v1/clientes")
            || p.starts_with("/v1/cotizaciones")
            || p.starts_with("/v1/conduces") =>
        {
            Some("POS_VENTAS")
        }
        p if p.starts_with("/v1/productos") || p.starts_with("/v1/categorias") || p.starts_with("/v1/inventario") => {
            Some("INVENTARIO")
        }
        p if p.starts_with("/v1/compras") || p.starts_with("/v1/proveedores") || p.starts_with("/v1/gastos") => {
            Some("COMPRAS_GASTOS")
        }
        p if p.starts_with("/v1/contabilidad") => Some("CONTABILIDAD"),
        p if p.starts_with("/v1/caja") || p.starts_with("/v1/bancos") => Some("CAJA_BANCOS"),
        p if p.starts_with("/v1/empleados") || p.starts_with("/v1/nomina") => Some("NOMINA"),
        _ => None,
    }
}

/// Gatea todas las rutas `/v1/staff/*` - nunca con el JWT de un tenant, solo
/// con `X-Vendor-Secret` (el mismo secreto que ya usaba, en solitario,
/// `http_activar_licencia`). Sin login individual de staff todavía - ver
/// `activado_por` en `tenant_modulos` para cuándo eso haga falta.
async fn staff_guard<B>(
    State(state): State<HttpState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, (StatusCode, String)> {
    let provided = req.headers().get("X-Vendor-Secret").and_then(|h| h.to_str().ok()).unwrap_or("");
    if provided.is_empty() || provided != state.vendor_admin_secret {
        return Err((StatusCode::FORBIDDEN, "X-Vendor-Secret inválido o ausente".to_string()));
    }
    Ok(next.run(req).await)
}

/// `None` = any authenticated role is fine. Checked most-specific-prefix first.
/// Keep in sync with apps/web/lib/roles.ts (frontend nav/route mirror of this table).
fn required_roles(path: &str, method: &Method) -> Option<&'static [&'static str]> {
    if path.starts_with("/v1/ecf/documentos") || path.starts_with("/v1/ecf/pendientes") {
        return Some(&["CONTADOR"]);
    }
    if path.starts_with("/v1/ecf/") || path.starts_with("/v1/test/") {
        return Some(&[]); // ADMIN-only (dev/test signing tooling, unused by real pages)
    }
    if path.starts_with("/v1/productos") || path.starts_with("/v1/categorias") {
        return if method == Method::GET {
            Some(&["CAJERO", "ALMACEN"])
        } else {
            Some(&["ALMACEN"])
        };
    }
    match path {
        p if p.starts_with("/v1/ventas")
            || p.starts_with("/v1/notas-credito")
            || p.starts_with("/v1/caja")
            || p.starts_with("/v1/clientes")
            || p.starts_with("/v1/cotizaciones")
            || p.starts_with("/v1/conduces") =>
        {
            Some(&["CAJERO"])
        }
        p if p.starts_with("/v1/inventario")
            || p.starts_with("/v1/compras")
            || p.starts_with("/v1/proveedores") =>
        {
            Some(&["ALMACEN"])
        }
        p if p.starts_with("/v1/contabilidad")
            || p.starts_with("/v1/bancos")
            || p.starts_with("/v1/gastos")
            || p.starts_with("/v1/reports/606")
            || p.starts_with("/v1/auditoria") =>
        {
            Some(&["CONTADOR"])
        }
        p if p.starts_with("/v1/empleados")
            || p.starts_with("/v1/nomina")
            || p.starts_with("/v1/config")
            || p.starts_with("/v1/tenants")
            || p.starts_with("/v1/backup") =>
        {
            Some(&[]) // ADMIN-only
        }
        // /v1/auth/me, /v1/rnc/:rnc, /v1/reports/dashboard, /v1/license/status -> any authenticated role
        _ => None,
    }
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

    // Postgres pool for Auth + EventStore
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/fiscal_core".to_string());
    let pool = match PgPool::connect(&database_url).await {
        Ok(p) => {
            tracing::info!("Postgres connected: {}", database_url);
            p
        },
        Err(e) => {
            tracing::warn!("Postgres connection failed ({}), using lazy pool - some auth endpoints will fail until DB up: {}", database_url, e);
            PgPool::connect_lazy(&database_url).expect("Invalid DATABASE_URL")
        }
    };

    let jwt_secret = std::env::var("JWT_SECRET").expect("JWT_SECRET debe estar configurado (ver .env.example) - genera uno con `openssl rand -base64 32`");
    let vendor_admin_secret = std::env::var("VENDOR_ADMIN_SECRET").expect("VENDOR_ADMIN_SECRET debe estar configurado (ver .env.example)");
    // Validados aquí para fallar al arrancar, no en el primer request que los
    // use - license_service.rs y config_service.rs los vuelven a leer
    // internamente cuando los necesitan.
    std::env::var("LICENSE_SECRET").expect("LICENSE_SECRET debe estar configurado (ver .env.example)");
    std::env::var("CERT_ENCRYPTION_KEY").expect("CERT_ENCRYPTION_KEY debe estar configurado (ver .env.example) - 32 bytes en base64");
    let frontend_url = std::env::var("FRONTEND_URL").unwrap_or_else(|_| "http://localhost:4000".to_string());
    let auth_service = Arc::new(AuthService::new(pool.clone(), jwt_secret));
    let catalog_service = Arc::new(CatalogService::new(pool.clone()));
    let inventario_service = Arc::new(InventarioService::new(pool.clone()));
    let partner_service = Arc::new(PartnerService::new(pool.clone()));
    let ventas_service = Arc::new(VentasService::new(pool.clone()));
    let cotizacion_service = Arc::new(CotizacionService::new(pool.clone()));
    let conduce_service = Arc::new(ConduceService::new(pool.clone()));
    let compras_service = Arc::new(ComprasService::new(pool.clone()));
    let caja_service = Arc::new(CajaService::new(pool.clone()));
    let bancos_service = Arc::new(BancosService::new(pool.clone()));
    let nomina_service = Arc::new(NominaService::new(pool.clone()));
    let contabilidad_service = Arc::new(ContabilidadService::new(pool.clone()));
    let report_service = Arc::new(ReportService::new(pool.clone()));
    let config_service = Arc::new(ConfigService::new(pool.clone()));
    let rnc_service = Arc::new(RncService::new(pool.clone()));
    let ecf_service = Arc::new(EcfService::new(pool.clone()));
    let license_service = Arc::new(LicenseService::new(pool.clone()));
    let staff_service = Arc::new(StaffService::new(pool.clone()));
    let backup_service = Arc::new(BackupService::new(database_url.clone()));
    let audit_service = Arc::new(AuditService::new(pool.clone()));
    // Miniaturas de fotos de producto - solo la ruta se guarda en Postgres,
    // el JPEG reescalado vive en disco bajo este directorio (servido
    // estáticamente vía ServeDir en /uploads, ver más abajo).
    let uploads_dir = std::env::var("UPLOADS_DIR").unwrap_or_else(|_| "uploads".to_string());
    let image_service = Arc::new(ImageService::new(uploads_dir.clone()));
    // 5 fallos -> bloqueado 15 min. Compartido entre login y forgot-password
    // (claves con prefijo distinto, ver http_login / http_forgot_password).
    let rate_limiter = Arc::new(RateLimiter::new(5, Duration::from_secs(15 * 60)));
    let email_service = Arc::new(EmailService::new(
        std::env::var("RESEND_API_KEY").ok().filter(|s| !s.is_empty()),
        std::env::var("RESEND_FROM_EMAIL").unwrap_or_else(|_| "onboarding@resend.dev".to_string()),
    ));
    let ai_service = Arc::new(AiService::new(
        pool.clone(),
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://localhost:11434".to_string()),
        // 1b en vez de 3b por defecto: menos RAM/CPU en hardware modesto de
        // colmado. Cambiar solo requiere OLLAMA_MODEL en .env, sin tocar código.
        std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "llama3.2:1b".to_string()),
    ));

    // Respaldo automático cada 24h - ver backup_service.rs. No hay cron
    // externo: arranca con el proceso, corre en background durante toda
    // la vida del servidor.
    {
        let backup_service = backup_service.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(24 * 60 * 60));
            loop {
                interval.tick().await;
                if let Err(e) = backup_service.run_backup().await {
                    tracing::warn!("Respaldo automático falló: {}", e);
                }
            }
        });
    }

    let http_state = HttpState {
        auth_service,
        catalog_service,
        inventario_service,
        partner_service,
        ventas_service,
        cotizacion_service,
        conduce_service,
        compras_service,
        caja_service,
        bancos_service,
        nomina_service,
        contabilidad_service,
        report_service,
        config_service,
        rnc_service,
        ecf_service,
        license_service,
        staff_service,
        backup_service,
        audit_service,
        image_service,
        vendor_admin_secret,
        rate_limiter,
        email_service,
        ai_service,
        frontend_url: frontend_url.clone(),
        pool,
    };

    let protected = Router::new()
        // MODULO 1: Auth y Multi-tenancy
        .route("/v1/tenants/:rnc", get(http_get_tenant))
        .route("/v1/tenants/:rnc/usuarios", get(http_list_usuarios))
        .route("/v1/auth/me", get(http_me))
        // MODULO 2: Categorias y Productos
        .route("/v1/categorias", get(http_list_categorias).post(http_create_categoria))
        .route("/v1/categorias/:id", axum::routing::put(http_update_categoria).delete(http_delete_categoria))
        .route("/v1/productos", get(http_list_productos).post(http_create_producto))
        .route("/v1/productos/:id", get(http_get_producto).put(http_update_producto).delete(http_delete_producto))
        .route("/v1/productos/:id/imagen", post(http_upload_producto_imagen))
        // MODULO 3: Inventario (kardex)
        .route("/v1/inventario/resumen", get(http_inventario_resumen))
        .route("/v1/inventario/movimientos", get(http_list_movimientos).post(http_create_movimiento))
        // MODULO 4: Clientes y Proveedores
        .route("/v1/clientes", get(http_list_clientes).post(http_create_cliente))
        .route("/v1/clientes/:id", get(http_get_cliente).put(http_update_cliente).delete(http_delete_cliente))
        .route("/v1/proveedores", get(http_list_proveedores).post(http_create_proveedor))
        .route("/v1/proveedores/:id", get(http_get_proveedor).put(http_update_proveedor).delete(http_delete_proveedor))
        // MODULO 5: Ventas / Punto de Venta
        .route("/v1/ventas", get(http_list_ventas).post(http_create_venta))
        .route("/v1/ventas/:id", get(http_get_venta))
        .route("/v1/ventas/:id/emitir-ecf", post(http_emitir_ecf_venta))
        .route("/v1/ventas/:id/imprimir", post(http_imprimir_venta))
        .route("/v1/ventas/:id/nota-credito", post(http_crear_nota_credito))
        .route("/v1/notas-credito/:id", get(http_get_nota_credito))
        .route("/v1/cotizaciones", get(http_list_cotizaciones).post(http_create_cotizacion))
        .route("/v1/cotizaciones/:id", get(http_get_cotizacion))
        .route("/v1/cotizaciones/:id/rechazar", post(http_rechazar_cotizacion))
        .route("/v1/cotizaciones/:id/convertir", post(http_convertir_cotizacion))
        .route("/v1/conduces", get(http_list_conduces).post(http_create_conduce))
        .route("/v1/conduces/:id", get(http_get_conduce))
        .route("/v1/ecf/documentos", get(http_list_ecf_documentos))
        .route("/v1/ecf/pendientes/reintentar", post(http_reintentar_pendientes))
        // MODULO 6: Compras y Gastos
        .route("/v1/compras", get(http_list_compras).post(http_create_compra))
        .route("/v1/compras/:id", get(http_get_compra))
        .route("/v1/compras/:id/anular", post(http_anular_compra))
        .route("/v1/gastos", get(http_list_gastos).post(http_create_gasto))
        // MODULO 9: Caja y Bancos
        .route("/v1/caja/resumen", get(http_caja_resumen))
        .route("/v1/caja/abrir", post(http_caja_abrir))
        .route("/v1/caja/cerrar", post(http_caja_cerrar))
        .route("/v1/caja/movimientos", get(http_caja_movimientos))
        .route("/v1/caja/sesiones", get(http_caja_sesiones))
        .route("/v1/bancos", get(http_list_bancos).post(http_create_banco))
        .route("/v1/bancos/:id/movimientos", get(http_list_banco_movimientos).post(http_create_banco_movimiento))
        // MODULO 8: Nomina y Adelantos
        .route("/v1/empleados", get(http_list_empleados).post(http_create_empleado))
        .route("/v1/empleados/:id", get(http_get_empleado).put(http_update_empleado).delete(http_delete_empleado))
        .route("/v1/nomina/adelantos", get(http_list_adelantos).post(http_request_adelanto))
        .route("/v1/nomina/adelantos/:id/aprobar", post(http_approve_adelanto))
        .route("/v1/nomina/adelantos/:id/rechazar", post(http_reject_adelanto))
        .route("/v1/nomina/periodos", get(http_list_periodos))
        .route("/v1/nomina/periodos/:id", get(http_get_periodo))
        .route("/v1/nomina/run", post(http_run_payroll))
        // MODULO 7: Contabilidad (Libro Diario / Libro Mayor)
        .route("/v1/contabilidad/asientos", get(http_list_asientos).post(http_create_asiento))
        .route("/v1/contabilidad/asientos/:id/reversar", post(http_reversar_asiento))
        .route("/v1/contabilidad/libro-mayor", get(http_libro_mayor))
        .route("/v1/contabilidad/libro-mayor/:cuenta", get(http_libro_mayor_detalle))
        .route("/v1/contabilidad/libro-diario", get(http_libro_diario))
        .route("/v1/contabilidad/cuentas", get(http_list_cuentas))
        .route("/v1/contabilidad/periodos", get(http_list_periodos_contables))
        .route("/v1/contabilidad/periodos/:anio/:mes/cerrar", post(http_cerrar_periodo))
        .route("/v1/contabilidad/sincronizar", post(http_sincronizar_contabilidad))
        // MODULO 10: Reportes y Dashboard
        .route("/v1/reports/606", get(http_report_606))
        .route("/v1/reports/606/csv", get(http_report_606_csv))
        .route("/v1/reports/dashboard", get(http_dashboard_resumen))
        .route("/v1/ai/digest", get(http_ai_digest))
        .route("/v1/ai/chat", post(http_ai_chat))
        // MODULO 11: Configuracion DGII y Empresa
        .route("/v1/config/empresa", get(http_get_empresa).put(http_update_empresa))
        .route("/v1/config/usuarios", get(http_config_list_usuarios).post(http_config_create_usuario))
        .route("/v1/config/usuarios/:id", axum::routing::put(http_config_update_usuario).delete(http_config_deactivate_usuario))
        .route("/v1/config/secuencias-ncf", get(http_list_secuencias).post(http_create_secuencia))
        .route("/v1/config/secuencias-ncf/:id/estado", axum::routing::put(http_set_secuencia_estado))
        .route("/v1/config/certificado", get(http_certificado_status).post(http_upload_certificado))
        .route("/v1/config/impresora", get(http_get_impresora).put(http_update_impresora))
        .route("/v1/config/impresora/test", post(http_test_impresora))
        .route("/v1/rnc/:rnc", get(http_lookup_rnc))
        // Licencia de prueba
        .route("/v1/license/status", get(http_license_status))
        // Módulos que el sitio de staff le asignó a este tenant - de solo
        // lectura, no hay selector de módulos en la app del tenant.
        .route("/v1/tenants/me/modulos", get(http_mis_modulos))
        // Respaldo local (pg_dump) - ADMIN-only
        .route("/v1/backup/descargar", get(http_backup_descargar))
        // Fiado: abono contra el saldo de un cliente
        .route("/v1/clientes/:id/abonos", post(http_registrar_abono))
        // Bitácora de auditoría
        .route("/v1/auditoria", get(http_list_auditoria))
        // DGII e-CF
        .route("/v1/ecf/sign", post(http_sign_ecf))
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
        .route("/v1/test/sign-demo", get(http_test_sign_demo_get).post(http_test_sign_demo))
        .route_layer(middleware::from_fn_with_state(http_state.clone(), role_guard))
        .route_layer(middleware::from_fn_with_state(http_state.clone(), license_guard))
        .route_layer(middleware::from_fn_with_state(http_state.clone(), modulo_guard));

    // Panel interno del equipo de ventas: nunca alcanzable con el JWT de un
    // tenant, solo con X-Vendor-Secret (ver `staff_guard`). Deliberadamente
    // fuera de `protected`.
    let staff = Router::new()
        .route("/v1/staff/tenants", get(http_staff_list_tenants).post(http_staff_create_tenant))
        .route("/v1/staff/tenants/:rnc", get(http_staff_get_tenant))
        .route("/v1/staff/tenants/:rnc/modulos", get(http_staff_list_modulos_tenant).put(http_staff_set_modulos_tenant))
        .route("/v1/staff/tenants/:rnc/empresa", get(http_staff_get_empresa).put(http_staff_update_empresa))
        .route("/v1/staff/tenants/:rnc/certificado", get(http_staff_certificado_status).post(http_staff_upload_certificado))
        .route("/v1/staff/tenants/:rnc/secuencias-ncf", get(http_staff_list_secuencias).post(http_staff_create_secuencia))
        .route("/v1/staff/tenants/:rnc/activar-licencia", post(http_staff_activar_licencia))
        .route("/v1/staff/tenants/:rnc/reenviar-invitacion", post(http_staff_reenviar_invitacion))
        .route("/v1/staff/modulos", get(http_staff_list_catalogo).post(http_staff_create_modulo_catalogo))
        .route_layer(middleware::from_fn_with_state(http_state.clone(), staff_guard));

    let http_app = Router::new()
        .route("/", get(root))
        .route("/health", get(health))
        .route("/v1/auth/register", post(http_register))
        .route("/v1/auth/login", post(http_login))
        .route("/v1/auth/forgot-password", post(http_forgot_password))
        .route("/v1/auth/reset-password", post(http_reset_password))
        // Vendedor-only: activa la licencia tras confirmar el pago. Deliberadamente
        // fuera de `protected` - no requiere JWT de tenant, solo X-Vendor-Secret
        // (ver http_activar_licencia), para que un ADMIN de tenant no pueda
        // desbloquear su propia prueba con su propio token.
        .route("/v1/tenants/:rnc/activar-licencia", post(http_activar_licencia))
        .merge(protected)
        .merge(staff)
        .with_state(http_state)
        // Miniaturas de producto: servidas como archivos estáticos, sin JWT.
        // Deliberado - son fotos de producto, no datos sensibles del tenant.
        .nest_service("/uploads", tower_http::services::ServeDir::new(&uploads_dir))
        .layer(
            tower_http::cors::CorsLayer::new()
                .allow_origin(
                    frontend_url
                        .parse::<axum::http::HeaderValue>()
                        .expect("FRONTEND_URL inválida - debe ser un origen como http://localhost:4000"),
                )
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
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
    r#"fiscal-core v0.3 - Rust core + Full ECF Builder v1.0 + Real DGII Send + RFCE + ARECF/ACECF

HTTP :3001
  POST /v1/auth/register, /v1/auth/login, GET /v1/auth/me - Auth y Multi-tenancy
  GET/POST /v1/categorias, PUT/DELETE /v1/categorias/:id
  GET/POST /v1/productos, GET/PUT/DELETE /v1/productos/:id

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

Docs: /docs/01-ARCHITECTURE.md etc - Spanish POS: apps/web/app/page.tsx
"#
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "core": "fiscal-core Rust v0.3",
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
    #[serde(rename = "itbisTipo")]
    itbis_tipo: Option<String>,
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
        let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal, String)> = simple.items.into_iter().map(|it| {
            let qty = it.cantidad.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ONE);
            let price = it.precio.parse::<rust_decimal::Decimal>().unwrap_or(rust_decimal::Decimal::ZERO);
            (it.nombre, qty, price, it.itbis_tipo.unwrap_or_else(|| "GRAVADO_18".to_string()))
        }).collect();
        build_simple_pos_ecf(&simple.tenant_rnc, &simple.razon_social, &simple.direccion, &simple.e_ncf, simple.tipo_ecf, &simple.cliente_rnc, &simple.cliente_nombre, items, &simple.fecha_emision, &simple.fecha_vencimiento, None, 0, None)
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
        let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal, String)> = simple.items.into_iter().map(|it| {
            let qty = it.cantidad.parse().unwrap_or(rust_decimal::Decimal::ONE);
            let price = it.precio.parse().unwrap_or(rust_decimal::Decimal::ZERO);
            (it.nombre, qty, price, it.itbis_tipo.unwrap_or_else(|| "GRAVADO_18".to_string()))
        }).collect();
        build_simple_pos_ecf(&simple.tenant_rnc, &simple.razon_social, &simple.direccion, &simple.e_ncf, simple.tipo_ecf, &simple.cliente_rnc, &simple.cliente_nombre, items, &simple.fecha_emision, &simple.fecha_vencimiento, None, 0, None)
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
        let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal, String)> = simple.items.into_iter().map(|it| {
            let qty = it.cantidad.parse().unwrap_or(rust_decimal::Decimal::ONE);
            let price = it.precio.parse().unwrap_or(rust_decimal::Decimal::ZERO);
            (it.nombre, qty, price, it.itbis_tipo.unwrap_or_else(|| "GRAVADO_18".to_string()))
        }).collect();
        build_simple_pos_ecf(&simple.tenant_rnc, &simple.razon_social, &simple.direccion, &simple.e_ncf, simple.tipo_ecf, &simple.cliente_rnc, &simple.cliente_nombre, items, &simple.fecha_emision, &simple.fecha_vencimiento, None, 0, None)
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

// ------------------ MODULO 1: Auth y Multi-tenancy ------------------

async fn http_register(
    State(state): State<HttpState>,
    Json(req): Json<TenantRegisterRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    match state.auth_service.register(req).await {
        Ok(resp) => Ok(Json(serde_json::json!({
            "success": true,
            "mensaje": "Negocio registrado exitosamente • RNC activo • Usuario ADMIN creado • Eventos TenantRegistrado y UsuarioCreado en ledger",
            "token": resp.token,
            "usuario": resp.usuario,
            "tenant": resp.tenant,
            "siguientePaso": "Sube tu certificado P12 DGII en /v1/tenants/:rnc/certificado y configura secuencias e-NCF en /configuracion/dgii"
        }))),
        Err(e) => Err((StatusCode::BAD_REQUEST, format!("Error registro: {}", e))),
    }
}

async fn http_login(
    State(state): State<HttpState>,
    Json(req): Json<AuthLoginRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let key = format!("login:{}:{}", req.rnc.as_deref().unwrap_or(""), req.email.to_lowercase());
    if state.rate_limiter.bloqueado(&key) {
        return Err((StatusCode::TOO_MANY_REQUESTS, "Demasiados intentos fallidos. Espera unos minutos e intenta de nuevo.".to_string()));
    }
    match state.auth_service.login(req).await {
        Ok(resp) => {
            state.rate_limiter.limpiar(&key);
            Ok(Json(serde_json::json!({
                "success": true,
                "mensaje": "Sesión iniciada • JWT 12h • tenant_id = RNC",
                "token": resp.token,
                "usuario": resp.usuario,
                "tenant": resp.tenant
            })))
        }
        Err(e) => {
            state.rate_limiter.registrar_fallo(&key);
            Err((StatusCode::UNAUTHORIZED, format!("Login falló: {}", e)))
        }
    }
}

#[derive(Debug, Deserialize)]
struct ForgotPasswordRequest {
    rnc: Option<String>,
    email: String,
}

/// La respuesta es intencionalmente idéntica exista o no el correo/usuario -
/// solo así "olvidé mi contraseña" no sirve para enumerar cuentas registradas.
const FORGOT_PASSWORD_MENSAJE: &str = "Si el correo existe, te enviamos un enlace para restablecer tu contraseña.";

async fn http_forgot_password(
    State(state): State<HttpState>,
    Json(req): Json<ForgotPasswordRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let key = format!("forgot:{}", req.email.to_lowercase());
    if state.rate_limiter.bloqueado(&key) {
        // Mismo mensaje genérico - ni siquiera el límite de intentos debe
        // delatar si el correo existe.
        return Ok(Json(serde_json::json!({ "mensaje": FORGOT_PASSWORD_MENSAJE })));
    }
    state.rate_limiter.registrar_fallo(&key);

    match state.auth_service.iniciar_reset_password(req.rnc.as_deref(), &req.email).await {
        Ok(Some((_, nombre, token))) => {
            let reset_url = format!("{}/restablecer-password?token={}", state.frontend_url, token);
            if let Err(e) = state.email_service.send_password_reset(&req.email, &nombre, &reset_url).await {
                tracing::warn!("Fallo enviando correo de reset a {}: {}", req.email, e);
            }
        }
        Ok(None) => {}
        Err(e) => tracing::warn!("Error iniciando reset de contraseña: {}", e),
    }
    Ok(Json(serde_json::json!({ "mensaje": FORGOT_PASSWORD_MENSAJE })))
}

#[derive(Debug, Deserialize)]
struct ResetPasswordRequest {
    token: String,
    new_password: String,
}

async fn http_reset_password(
    State(state): State<HttpState>,
    Json(req): Json<ResetPasswordRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if req.new_password.len() < 8 {
        return Err((StatusCode::BAD_REQUEST, "La contraseña debe tener al menos 8 caracteres".to_string()));
    }
    state.auth_service.completar_reset_password(&req.token, &req.new_password).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true, "mensaje": "Contraseña actualizada. Ya puedes iniciar sesión." })))
}

async fn http_get_tenant(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let rnc = rnc.replace("-", "");
    if claims.tenant_id != rnc {
        return Err((StatusCode::FORBIDDEN, "No tienes acceso a este negocio".to_string()));
    }
    match state.auth_service.get_tenant(&rnc).await {
        Ok(tenant) => Ok(Json(serde_json::json!(tenant))),
        Err(e) => Err((StatusCode::NOT_FOUND, format!("Tenant no encontrado: {}", e))),
    }
}

async fn http_list_usuarios(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let rnc = rnc.replace("-", "");
    if claims.tenant_id != rnc {
        return Err((StatusCode::FORBIDDEN, "No tienes acceso a este negocio".to_string()));
    }
    match state.auth_service.list_usuarios(&rnc).await {
        Ok(usuarios) => Ok(Json(serde_json::json!({
            "rnc": rnc,
            "total": usuarios.len(),
            "usuarios": usuarios,
            "roles": ["ADMIN - Dueño total", "CAJERO - Solo POS y Caja", "ALMACEN - Solo Inventario", "CONTADOR - Solo Reportes y DGII"]
        }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Error listando usuarios: {}", e))),
    }
}

async fn http_me(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    Ok(Json(serde_json::json!({
        "autenticado": true,
        "usuario_id": claims.sub,
        "tenant_id": claims.tenant_id,
        "rol": claims.rol,
        "email": claims.email,
        "expira": claims.exp,
        "mensaje": "Token válido • tenant_id = RNC para multi-tenancy"
    })))
}

// ------------------ Licencia de prueba ------------------

async fn http_license_status(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::license_service::LicenseState>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.license_service.check_and_update(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Qué módulos le asignó el sitio de staff a este tenant - de solo lectura;
/// no existe un endpoint de escritura alcanzable con el JWT del propio
/// tenant (ver `services::staff_service` + `staff_guard`).
async fn http_mis_modulos(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<Vec<services::staff_service::ModuloAsignado>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.staff_service.list_modulos_tenant(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// El vendedor confirma un pago y activa la licencia manualmente, fuera del
/// flujo normal de tenant/JWT - ver el comentario en el router (`main()`)
/// sobre por qué este endpoint vive en `public`.
async fn http_activar_licencia(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(rnc): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let provided = headers.get("X-Vendor-Secret").and_then(|h| h.to_str().ok()).unwrap_or("");
    if provided.is_empty() || provided != state.vendor_admin_secret {
        return Err((StatusCode::FORBIDDEN, "X-Vendor-Secret inválido o ausente".to_string()));
    }
    let rnc = rnc.replace("-", "");
    state.license_service.activate(&rnc).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&rnc, None, "LICENCIA_ACTIVADA", "tenant", None, serde_json::json!({})).await;
    Ok(Json(serde_json::json!({ "ok": true, "mensaje": "Licencia activada" })))
}

// ------------------ Panel de staff (ventas/onboarding) ------------------
// Todo lo de aquí abajo vive detrás de `staff_guard` (X-Vendor-Secret) en el
// router `staff` de `main()` - nunca alcanzable con el JWT de un tenant.

async fn http_staff_list_tenants(
    State(state): State<HttpState>,
) -> Result<Json<Vec<services::staff_service::TenantResumen>>, (StatusCode, String)> {
    state.staff_service.list_tenants().await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_staff_get_tenant(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
) -> Result<Json<services::staff_service::TenantResumen>, (StatusCode, String)> {
    state.staff_service.get_tenant(&rnc).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Tenant no encontrado".to_string()))
}

#[derive(Debug, Deserialize)]
struct StaffCreateTenantRequest {
    rnc: String,
    razon_social: String,
    direccion: String,
    telefono: Option<String>,
    correo: String,
    admin_nombre: String,
    factura_electronica_activa: Option<bool>,
}

/// El onboarding real de un cliente pagado: el staff crea el negocio y el
/// usuario ADMIN con una contraseña de un solo uso que nunca se expone (ni
/// al staff ni por la red) - se descarta de inmediato y en su lugar se le
/// manda al cliente un enlace de "definir tu contraseña" (mismo mecanismo
/// que "olvidé mi contraseña", ver `iniciar_reset_password`). No hay
/// selector de módulos aquí a propósito - se asignan aparte en
/// `PUT /v1/staff/tenants/:rnc/modulos`, después de hablar con el cliente.
async fn http_staff_create_tenant(
    State(state): State<HttpState>,
    Json(req): Json<StaffCreateTenantRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let contrasena_temporal = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
    let resp = state
        .auth_service
        .register(TenantRegisterRequest {
            rnc: req.rnc,
            razon_social: req.razon_social,
            direccion: req.direccion,
            telefono: req.telefono,
            correo: Some(req.correo.clone()),
            factura_electronica_activa: req.factura_electronica_activa,
            admin_nombre: req.admin_nombre,
            admin_email: req.correo.clone(),
            admin_password: contrasena_temporal,
        })
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("No se pudo crear el negocio: {}", e)))?;

    state.audit_service.log(&resp.tenant.rnc, None, "TENANT_CREADO_POR_STAFF", "tenant", None, serde_json::json!({})).await;
    enviar_invitacion(&state, &resp.tenant.rnc, &req.correo).await;

    Ok(Json(serde_json::json!({
        "ok": true,
        "tenant": resp.tenant,
        "mensaje": "Negocio creado. Se envió un correo de invitación para que el cliente defina su contraseña.",
    })))
}

/// Compartido por `http_staff_create_tenant` y `http_staff_reenviar_invitacion`.
/// Best-effort: igual que el resto de envíos de correo en este archivo, un
/// fallo de correo no debe tumbar la operación que lo disparó.
async fn enviar_invitacion(state: &HttpState, rnc: &str, correo: &str) {
    match state.auth_service.iniciar_reset_password(Some(rnc), correo).await {
        Ok(Some((_, nombre, token))) => {
            let reset_url = format!("{}/restablecer-password?token={}", state.frontend_url, token);
            if let Err(e) = state.email_service.send_password_reset(correo, &nombre, &reset_url).await {
                tracing::warn!("Fallo enviando invitación a {}: {}", correo, e);
            }
        }
        Ok(None) => tracing::warn!("No se encontró usuario ADMIN para {} al enviar invitación", rnc),
        Err(e) => tracing::warn!("Error generando invitación para {}: {}", rnc, e),
    }
}

async fn http_staff_reenviar_invitacion(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let tenant = state.staff_service.get_tenant(&rnc).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "Tenant no encontrado".to_string()))?;
    let correo = tenant.correo.ok_or((StatusCode::BAD_REQUEST, "Este tenant no tiene correo registrado".to_string()))?;
    enviar_invitacion(&state, &rnc, &correo).await;
    Ok(Json(serde_json::json!({ "ok": true, "mensaje": "Invitación reenviada" })))
}

async fn http_staff_list_modulos_tenant(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
) -> Result<Json<Vec<services::staff_service::ModuloAsignado>>, (StatusCode, String)> {
    state.staff_service.list_modulos_tenant(&rnc).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_staff_set_modulos_tenant(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
    Json(req): Json<services::staff_service::SetModulosRequest>,
) -> Result<Json<Vec<services::staff_service::ModuloAsignado>>, (StatusCode, String)> {
    state.staff_service.set_modulos_tenant(&rnc, &req.codigos, Some("staff")).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&rnc, None, "MODULOS_ACTUALIZADOS", "tenant", None, serde_json::json!({ "codigos": req.codigos })).await;
    state.staff_service.list_modulos_tenant(&rnc).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_staff_activar_licencia(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    state.license_service.activate(&rnc).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&rnc, None, "LICENCIA_ACTIVADA", "tenant", None, serde_json::json!({})).await;
    Ok(Json(serde_json::json!({ "ok": true, "mensaje": "Licencia activada" })))
}

// -- Mi Negocio y DGII, en nombre del tenant (mismo servicio y misma forma
// que /v1/config/*, la app del tenant sigue teniendo su propia página para
// esto sin cambios - esto es para que el staff lo prepare en el onboarding).

async fn http_staff_get_empresa(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
) -> Result<Json<services::config_service::Empresa>, (StatusCode, String)> {
    state.config_service.get_empresa(&rnc).await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

async fn http_staff_update_empresa(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
    Json(req): Json<services::config_service::UpdateEmpresaRequest>,
) -> Result<Json<services::config_service::Empresa>, (StatusCode, String)> {
    state.config_service.update_empresa(&rnc, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_staff_certificado_status(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
) -> Result<Json<Option<services::config_service::CertificadoStatus>>, (StatusCode, String)> {
    state.config_service.certificado_status(&rnc).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_staff_upload_certificado(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
    Json(req): Json<services::config_service::UploadCertificadoRequest>,
) -> Result<Json<services::config_service::CertificadoStatus>, (StatusCode, String)> {
    state.config_service.upload_certificado(&rnc, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_staff_list_secuencias(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
) -> Result<Json<Vec<services::config_service::SecuenciaNcf>>, (StatusCode, String)> {
    state.config_service.list_secuencias(&rnc).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_staff_create_secuencia(
    State(state): State<HttpState>,
    Path(rnc): Path<String>,
    Json(req): Json<services::config_service::CreateSecuenciaRequest>,
) -> Result<Json<services::config_service::SecuenciaNcf>, (StatusCode, String)> {
    state.config_service.create_secuencia(&rnc, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

// -- Catálogo de módulos (extensible sin migración - ver comentario en
// services::staff_service y en la migración modulos_catalogo/tenant_modulos).

async fn http_staff_list_catalogo(
    State(state): State<HttpState>,
) -> Result<Json<Vec<services::staff_service::ModuloCatalogo>>, (StatusCode, String)> {
    state.staff_service.list_catalogo().await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_staff_create_modulo_catalogo(
    State(state): State<HttpState>,
    Json(req): Json<services::staff_service::CreateModuloRequest>,
) -> Result<Json<services::staff_service::ModuloCatalogo>, (StatusCode, String)> {
    state.staff_service.create_modulo_catalogo(req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

// ------------------ Respaldo local ------------------

async fn http_backup_descargar(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Response, (StatusCode, String)> {
    claims_from_headers(&state.auth_service, &headers)?;
    let path = state.backup_service.run_backup().await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error generando respaldo: {}", e)))?;
    let bytes = tokio::fs::read(&path).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Error leyendo respaldo: {}", e)))?;
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("backup.dump").to_string();
    Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "application/octet-stream")
        .header(axum::http::header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
        .body(boxed(Full::from(bytes)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

// ------------------ Bitácora de auditoría ------------------

#[derive(Debug, Deserialize)]
struct ListAuditoriaParams {
    #[serde(rename = "usuarioId")] usuario_id: Option<Uuid>,
    accion: Option<String>,
    entidad: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_auditoria(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListAuditoriaParams>,
) -> Result<Json<pagination::Page<services::audit_service::AuditoriaEntry>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (entradas, total) = state.audit_service.list(
        &claims.tenant_id,
        params.usuario_id,
        params.accion,
        params.entidad,
        params.fecha_desde,
        params.fecha_hasta,
        &page,
        &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(10);
    Ok(Json(pagination::Page::new(entradas, page.page_number(), page_size, total)))
}

// ------------------ MODULO 2: Categorias y Productos ------------------

#[derive(Debug, Deserialize)]
struct ListCategoriasParams {
    search: Option<String>,
    activo: Option<bool>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_categorias(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListCategoriasParams>,
) -> Result<Json<pagination::Page<services::catalog_service::Categoria>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (categorias, total) = state.catalog_service.list_categorias(&claims.tenant_id, params.search, params.activo, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(categorias, page.page_number(), page_size, total)))
}

async fn http_create_categoria(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::catalog_service::CreateCategoriaRequest>,
) -> Result<Json<services::catalog_service::Categoria>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.catalog_service.create_categoria(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_update_categoria(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::catalog_service::UpdateCategoriaRequest>,
) -> Result<Json<services::catalog_service::Categoria>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.catalog_service.update_categoria(&claims.tenant_id, id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_delete_categoria(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.catalog_service.delete_categoria(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
struct ListProductosParams {
    #[serde(rename = "categoriaId")] categoria_id: Option<Uuid>,
    search: Option<String>,
    #[serde(rename = "unidadMedida")] unidad_medida: Option<String>,
    activo: Option<bool>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_productos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListProductosParams>,
) -> Result<Json<pagination::Page<services::catalog_service::Producto>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (productos, total) = state.catalog_service.list_productos(
        &claims.tenant_id,
        params.categoria_id,
        params.search,
        params.unidad_medida,
        params.activo,
        &page,
        &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(productos, page.page_number(), page_size, total)))
}

async fn http_get_producto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::catalog_service::Producto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.catalog_service.get_producto(&claims.tenant_id, id).await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

async fn http_create_producto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::catalog_service::CreateProductoRequest>,
) -> Result<Json<services::catalog_service::Producto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.catalog_service.create_producto(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_update_producto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::catalog_service::UpdateProductoRequest>,
) -> Result<Json<services::catalog_service::Producto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).ok();
    let producto = state.catalog_service.update_producto(&claims.tenant_id, id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, usuario_id, "PRODUCTO_ACTUALIZADO", "producto", Some(id),
        serde_json::json!({ "precio_venta": producto.precio_venta, "costo": producto.costo })).await;
    Ok(Json(producto))
}

async fn http_upload_producto_imagen(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    mut multipart: Multipart,
) -> Result<Json<services::catalog_service::Producto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).ok();

    let mut bytes: Option<Vec<u8>> = None;
    while let Some(field) = multipart.next_field().await
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Multipart inválido: {}", e)))?
    {
        if field.name() == Some("imagen") {
            bytes = Some(field.bytes().await
                .map_err(|e| (StatusCode::BAD_REQUEST, format!("No se pudo leer el archivo: {}", e)))?
                .to_vec());
        }
    }
    let bytes = bytes.ok_or((StatusCode::BAD_REQUEST, "Falta el campo 'imagen' en el formulario".to_string()))?;

    let imagen_url = state.image_service.save_thumbnail(&claims.tenant_id, id, bytes).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let producto = state.catalog_service.set_imagen(&claims.tenant_id, id, &imagen_url).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, usuario_id, "PRODUCTO_IMAGEN_ACTUALIZADA", "producto", Some(id),
        serde_json::json!({ "imagen_url": imagen_url })).await;
    Ok(Json(producto))
}

async fn http_delete_producto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.catalog_service.delete_producto(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ------------------ MODULO 3: Inventario (kardex) ------------------

async fn http_inventario_resumen(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::inventario_service::ResumenInventario>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.inventario_service.resumen(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Debug, Deserialize)]
struct ListMovimientosParams {
    #[serde(rename = "productoId")] producto_id: Option<Uuid>,
    tipo: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_movimientos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListMovimientosParams>,
) -> Result<Json<pagination::Page<services::inventario_service::MovimientoConProducto>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (movimientos, total) = state.inventario_service.list_movimientos(
        &claims.tenant_id,
        params.producto_id,
        params.tipo,
        params.fecha_desde,
        params.fecha_hasta,
        &page,
        &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(movimientos, page.page_number(), page_size, total)))
}

async fn http_create_movimiento(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::inventario_service::CreateMovimientoRequest>,
) -> Result<Json<services::inventario_service::Movimiento>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let movimiento = state.inventario_service.create_movimiento(&claims.tenant_id, usuario_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "MOVIMIENTO_INVENTARIO_MANUAL", "producto", Some(movimiento.producto_id),
        serde_json::json!({ "tipo": movimiento.tipo, "cantidad": movimiento.cantidad })).await;
    Ok(Json(movimiento))
}

/// Query params shared by list endpoints with no entity-specific filters —
/// just pagination + sort.
#[derive(Debug, Deserialize)]
struct PageSortParams {
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

// ------------------ MODULO 4: Clientes y Proveedores ------------------

#[derive(Debug, Deserialize)]
struct SearchParams {
    search: Option<String>,
    activo: Option<bool>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_clientes(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<pagination::Page<services::partner_service::Cliente>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (clientes, total) = state.partner_service.list_clientes(&claims.tenant_id, params.search, params.activo, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(clientes, page.page_number(), page_size, total)))
}

async fn http_get_cliente(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::partner_service::Cliente>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.get_cliente(&claims.tenant_id, id).await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

async fn http_create_cliente(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::partner_service::CreateClienteRequest>,
) -> Result<Json<services::partner_service::Cliente>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.create_cliente(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_update_cliente(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::partner_service::UpdateClienteRequest>,
) -> Result<Json<services::partner_service::Cliente>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.update_cliente(&claims.tenant_id, id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_delete_cliente(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.delete_cliente(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn http_registrar_abono(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::partner_service::CreateAbonoRequest>,
) -> Result<Json<services::partner_service::ClienteAbono>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let abono = state.partner_service.registrar_abono(&claims.tenant_id, id, usuario_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "ABONO_REGISTRADO", "cliente", Some(id),
        serde_json::json!({ "monto": abono.monto })).await;
    Ok(Json(abono))
}

async fn http_list_proveedores(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<pagination::Page<services::partner_service::Proveedor>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (proveedores, total) = state.partner_service.list_proveedores(&claims.tenant_id, params.search, params.activo, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(proveedores, page.page_number(), page_size, total)))
}

async fn http_get_proveedor(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::partner_service::Proveedor>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.get_proveedor(&claims.tenant_id, id).await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

async fn http_create_proveedor(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::partner_service::CreateProveedorRequest>,
) -> Result<Json<services::partner_service::Proveedor>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.create_proveedor(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_update_proveedor(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::partner_service::UpdateProveedorRequest>,
) -> Result<Json<services::partner_service::Proveedor>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.update_proveedor(&claims.tenant_id, id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_delete_proveedor(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.partner_service.delete_proveedor(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ------------------ MODULO 5: Ventas / Punto de Venta ------------------

#[derive(Debug, Serialize)]
struct VentaCompletaResponse {
    #[serde(flatten)]
    venta: services::ventas_service::Venta,
    items: Vec<services::ventas_service::VentaItem>,
}

#[derive(Debug, Deserialize)]
struct ListVentasParams {
    #[serde(rename = "clienteId")] cliente_id: Option<Uuid>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    search: Option<String>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_ventas(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListVentasParams>,
) -> Result<Json<pagination::Page<services::ventas_service::VentaConCliente>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (ventas, total) = state.ventas_service.list_ventas(
        &claims.tenant_id,
        params.cliente_id,
        params.fecha_desde,
        params.fecha_hasta,
        params.search,
        &page,
        &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(ventas, page.page_number(), page_size, total)))
}

#[derive(Debug, Deserialize)]
struct AprobacionAdmin {
    email: String,
    password: String,
}

/// Wrapper solo-HTTP: mantiene la aprobación (un asunto de auth/transporte)
/// fuera de `ventas_service::CreateVentaRequest`, que solo conoce reglas de
/// negocio (ver DescuentoRequiereAprobacion en ventas_service.rs).
#[derive(Debug, Deserialize)]
struct CreateVentaHttpRequest {
    #[serde(flatten)]
    venta: services::ventas_service::CreateVentaRequest,
    aprobacion_admin: Option<AprobacionAdmin>,
}

async fn http_create_venta(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<CreateVentaHttpRequest>,
) -> Result<Json<VentaCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;

    // Si viene una aprobación, se verifica ANTES de tocar ventas_service -
    // credenciales de ADMIN inválidas deben fallar de inmediato, nunca
    // colarse como si no se hubiera pedido aprobación.
    let aprobado_por = match req.aprobacion_admin {
        Some(a) => Some(
            state.auth_service.verify_admin_credentials(&claims.tenant_id, &a.email, &a.password).await
                .map_err(|e| (StatusCode::FORBIDDEN, e.to_string()))?,
        ),
        None => None,
    };

    let completa = state.ventas_service.create_venta(&claims.tenant_id, usuario_id, &claims.rol, req.venta, aprobado_por).await
        .map_err(|e| {
            if e.downcast_ref::<services::ventas_service::DescuentoRequiereAprobacion>().is_some() {
                (StatusCode::FORBIDDEN, e.to_string())
            } else {
                (StatusCode::BAD_REQUEST, e.to_string())
            }
        })?;

    let descuento_total: rust_decimal::Decimal = completa.items.iter().map(|i| i.descuento).sum();
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "VENTA_CREADA", "venta", Some(completa.venta.id),
        serde_json::json!({ "total": completa.venta.total, "metodo_pago": completa.venta.metodo_pago, "descuento_total": descuento_total })).await;
    if let Some(admin_id) = aprobado_por {
        state.audit_service.log(&claims.tenant_id, Some(admin_id), "DESCUENTO_APROBADO_POR_ADMIN", "venta", Some(completa.venta.id),
            serde_json::json!({ "descuento_total": descuento_total, "cajero_id": usuario_id })).await;
    }
    Ok(Json(VentaCompletaResponse { venta: completa.venta, items: completa.items }))
}

async fn http_get_venta(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<VentaCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let completa = state.ventas_service.get_venta(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(VentaCompletaResponse { venta: completa.venta, items: completa.items }))
}

#[derive(Debug, Deserialize, Default)]
struct EmitirEcfRequest {
    /// Si se omite, se usa el certificado P12 guardado en Configuración → DGII.
    #[serde(rename = "p12Base64")] p12_base64: Option<String>,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
    environment: Option<String>,
    #[serde(rename = "sendToDgii")] send_to_dgii: Option<bool>,
}

/// Emite el e-CF real de una venta: asigna el siguiente número de la
/// secuencia DGII autorizada (nunca un timestamp local), construye y firma
/// el XML, lo transmite a DGII en tiempo real por defecto, y guarda el XML
/// firmado + la respuesta para la retención de 10 años. Si DGII no responde
/// (sin conexión, caído), la venta no se bloquea pero tampoco se marca como
/// fiscalmente aceptada: queda en `CONTINGENCIA_PENDIENTE` para reintento
/// (ver /v1/ventas/:id/reintentar-envio).
async fn http_emitir_ecf_venta(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<EmitirEcfRequest>,
) -> Result<Json<services::ventas_service::Venta>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let completa = state.ventas_service.get_venta(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    if completa.venta.e_ncf.is_some() {
        return Err((StatusCode::BAD_REQUEST, "Esta venta ya tiene un e-CF emitido".to_string()));
    }
    let tenant = state.auth_service.get_tenant(&claims.tenant_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    if !tenant.factura_electronica_activa {
        return Err((
            StatusCode::BAD_REQUEST,
            "Este negocio no tiene factura electrónica (e-CF) activada. Actívala en Configuración → Mi negocio.".to_string(),
        ));
    }

    let (cliente_rnc, cliente_nombre, cliente_direccion) = match completa.venta.cliente_id {
        Some(cid) => match state.partner_service.get_cliente(&claims.tenant_id, cid).await {
            Ok(c) => (c.rnc_cedula.unwrap_or_else(|| "000000000".to_string()), c.nombre, c.direccion),
            Err(_) => ("000000000".to_string(), "CONSUMIDOR FINAL".to_string(), None),
        },
        None => ("000000000".to_string(), "CONSUMIDOR FINAL".to_string(), None),
    };

    let tipo_ecf = completa.venta.tipo_ecf.unwrap_or(32);
    services::ecf_service::requiere_identificacion(tipo_ecf, completa.venta.total, Some(&cliente_rnc), cliente_direccion.as_deref())
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal, String)> = completa.items.iter()
        .map(|it| (it.nombre.clone(), it.cantidad, it.precio_unitario, it.itbis_tipo.clone()))
        .collect();

    let (e_ncf, fecha_vencimiento_secuencia) = state.ecf_service.allocar_siguiente_ncf(&claims.tenant_id, tipo_ecf).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let now = chrono::Local::now();
    let fecha_emision = now.format("%d-%m-%Y").to_string();
    let fecha_vencimiento = fecha_vencimiento_secuencia.format("%d-%m-%Y").to_string();

    let ecf = build_simple_pos_ecf(
        &claims.tenant_id, &tenant.razon_social, &tenant.direccion, &e_ncf, tipo_ecf, &cliente_rnc, &cliente_nombre,
        items, &fecha_emision, &fecha_vencimiento, cliente_direccion.as_deref(), 0, None,
    );
    let xml_built = build_ecf_xml(&ecf);

    let (p12_der, password) = if let Some(b64) = req.p12_base64.as_deref().filter(|s| !s.trim().is_empty()) {
        let der = base64::engine::general_purpose::STANDARD.decode(b64.trim())
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
        (der, req.p12_password.unwrap_or_else(|| "password".to_string()))
    } else {
        state.config_service.get_certificado_activo(&claims.tenant_id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::BAD_REQUEST, "No hay p12Base64 en la petición ni certificado guardado en Configuración → DGII".to_string()))?
    };
    let signed = sign_xml_ecf(&xml_built, &p12_der, &password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Signing failed: {}", e)))?;
    let qr_url = generate_qr_url(&claims.tenant_id, &e_ncf, &cliente_rnc, &fecha_emision, &completa.venta.total.to_string(), &signed.codigo_seguridad);

    // Envío en tiempo real por defecto. `sendToDgii: false` explícito solo
    // sirve para pruebas locales sin tocar la red de DGII.
    let (estado_dgii, track_id, mensaje_dgii) = if req.send_to_dgii.unwrap_or(true) {
        let ambiente = req.environment.clone().unwrap_or_else(|| tenant.ambiente_dgii.clone());
        let env = DGIIEnvironment::from_string(&ambiente);
        let mut client = DGIIClient::new(env);
        match client.authenticate(&p12_der, &password).await {
            Ok(_) => match client.send_with_polling(&signed.signed_xml, &format!("{}{}.xml", claims.tenant_id, e_ncf)).await {
                Ok(tracking) => (tracking.estado, Some(tracking.track_id), None),
                Err(e) => ("CONTINGENCIA_PENDIENTE".to_string(), None, Some(e.to_string())),
            },
            Err(e) => ("CONTINGENCIA_PENDIENTE".to_string(), None, Some(e.to_string())),
        }
    } else {
        ("FIRMADO_NO_ENVIADO".to_string(), None, None)
    };

    state.ecf_service.registrar_documento(services::ecf_service::NuevoDocumento {
        tenant_id: &claims.tenant_id,
        referencia_tipo: "VENTA",
        referencia_id: id,
        tipo_ecf,
        e_ncf: &e_ncf,
        xml_firmado: &signed.signed_xml,
        estado_dgii: &estado_dgii,
        track_id: track_id.as_deref(),
        codigo_seguridad: &signed.codigo_seguridad,
        qr_url: &qr_url,
        mensaje_dgii: mensaje_dgii.as_deref(),
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let venta = state.ventas_service.set_ecf_result(
        &claims.tenant_id, id, &e_ncf, tipo_ecf, &estado_dgii, track_id.as_deref(), Some(&signed.codigo_seguridad), Some(&qr_url),
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(venta))
}

/// Reintenta el envío a DGII de todos los documentos e-CF (Ventas y Notas de
/// Crédito) que quedaron en `CONTINGENCIA_PENDIENTE` por falta de
/// conectividad — punto 7 de la migración NCF -> e-CF.
async fn http_reintentar_pendientes(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let pendientes = state.ecf_service.list_pendientes_contingencia(&claims.tenant_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let (p12_der, password) = state.config_service.get_certificado_activo(&claims.tenant_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::BAD_REQUEST, "No hay certificado guardado en Configuración → DGII".to_string()))?;
    let tenant = state.auth_service.get_tenant(&claims.tenant_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let env = DGIIEnvironment::from_string(&tenant.ambiente_dgii);

    let mut reenviados = 0;
    let mut siguen_pendientes = 0;
    for doc in pendientes {
        let Ok(Some(xml)) = state.ecf_service.get_xml(&claims.tenant_id, doc.id).await else { continue };
        let mut client = DGIIClient::new(env.clone());
        let resultado = match client.authenticate(&p12_der, &password).await {
            Ok(_) => client.send_with_polling(&xml, &format!("{}{}.xml", claims.tenant_id, doc.e_ncf)).await,
            Err(e) => Err(e),
        };
        match resultado {
            Ok(tracking) => {
                state.ecf_service.actualizar_estado(doc.id, &tracking.estado, Some(&tracking.track_id), None).await
                    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
                if doc.referencia_tipo == "VENTA" {
                    state.ventas_service.set_ecf_result(&claims.tenant_id, doc.referencia_id, &doc.e_ncf, doc.tipo_ecf, &tracking.estado, Some(&tracking.track_id), doc.codigo_seguridad.as_deref(), doc.qr_url.as_deref()).await.ok();
                } else {
                    state.ventas_service.set_nota_credito_ecf_result(&claims.tenant_id, doc.referencia_id, &doc.e_ncf, &tracking.estado, doc.codigo_seguridad.as_deref(), doc.qr_url.as_deref()).await.ok();
                }
                reenviados += 1;
            }
            Err(_) => siguen_pendientes += 1,
        }
    }

    Ok(Json(serde_json::json!({ "reenviados": reenviados, "siguen_pendientes": siguen_pendientes })))
}

#[derive(Debug, Deserialize)]
struct CrearNotaCreditoRequest {
    motivo: String,
    #[serde(rename = "p12Base64")] p12_base64: Option<String>,
    #[serde(rename = "p12Password")] p12_password: Option<String>,
    environment: Option<String>,
    #[serde(rename = "sendToDgii")] send_to_dgii: Option<bool>,
}

/// Emite una Nota de Crédito (e-CF Tipo 34) para una venta ya facturada:
/// revierte stock y caja, referencia el e-NCF original vía
/// InformacionReferencia (nunca edita/anula la venta en sitio), y sigue el
/// mismo pipeline real de firma + envío + retención que una venta.
async fn http_crear_nota_credito(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(venta_id): Path<Uuid>,
    Json(req): Json<CrearNotaCreditoRequest>,
) -> Result<Json<services::ventas_service::NotaCredito>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;

    let venta_original = state.ventas_service.get_venta(&claims.tenant_id, venta_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let e_ncf_original = venta_original.venta.e_ncf.clone()
        .ok_or((StatusCode::BAD_REQUEST, "La venta original no tiene un e-CF emitido — no se puede emitir una Nota de Crédito".to_string()))?;

    let (nota, _items) = state.ventas_service.create_nota_credito(&claims.tenant_id, usuario_id, venta_id, &req.motivo).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "NOTA_CREDITO_EMITIDA", "venta", Some(venta_id),
        serde_json::json!({ "motivo": req.motivo, "total": nota.total })).await;

    let tenant = state.auth_service.get_tenant(&claims.tenant_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let (cliente_rnc, cliente_nombre, cliente_direccion) = match venta_original.venta.cliente_id {
        Some(cid) => match state.partner_service.get_cliente(&claims.tenant_id, cid).await {
            Ok(c) => (c.rnc_cedula.unwrap_or_else(|| "000000000".to_string()), c.nombre, c.direccion),
            Err(_) => ("000000000".to_string(), "CONSUMIDOR FINAL".to_string(), None),
        },
        None => ("000000000".to_string(), "CONSUMIDOR FINAL".to_string(), None),
    };

    let tipo_ecf = 34;
    let (e_ncf, fecha_vencimiento_secuencia) = state.ecf_service.allocar_siguiente_ncf(&claims.tenant_id, tipo_ecf).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let now = chrono::Local::now();
    let fecha_emision = now.format("%d-%m-%Y").to_string();
    let fecha_vencimiento = fecha_vencimiento_secuencia.format("%d-%m-%Y").to_string();

    let items: Vec<(String, rust_decimal::Decimal, rust_decimal::Decimal, String)> = venta_original.items.iter()
        .map(|it| (it.nombre.clone(), it.cantidad, it.precio_unitario, it.itbis_tipo.clone()))
        .collect();

    let ecf = build_simple_pos_ecf(
        &claims.tenant_id, &tenant.razon_social, &tenant.direccion, &e_ncf, tipo_ecf, &cliente_rnc, &cliente_nombre,
        items, &fecha_emision, &fecha_vencimiento, cliente_direccion.as_deref(), 0,
        Some((&e_ncf_original, &req.motivo)),
    );
    let xml_built = build_ecf_xml(&ecf);

    let (p12_der, password) = if let Some(b64) = req.p12_base64.as_deref().filter(|s| !s.trim().is_empty()) {
        let der = base64::engine::general_purpose::STANDARD.decode(b64.trim())
            .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid p12Base64: {}", e)))?;
        (der, req.p12_password.unwrap_or_else(|| "password".to_string()))
    } else {
        state.config_service.get_certificado_activo(&claims.tenant_id).await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
            .ok_or((StatusCode::BAD_REQUEST, "No hay p12Base64 en la petición ni certificado guardado en Configuración → DGII".to_string()))?
    };
    let signed = sign_xml_ecf(&xml_built, &p12_der, &password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Signing failed: {}", e)))?;
    let qr_url = generate_qr_url(&claims.tenant_id, &e_ncf, &cliente_rnc, &fecha_emision, &nota.total.to_string(), &signed.codigo_seguridad);

    let (estado_dgii, track_id, mensaje_dgii) = if req.send_to_dgii.unwrap_or(true) {
        let ambiente = req.environment.clone().unwrap_or_else(|| tenant.ambiente_dgii.clone());
        let env = DGIIEnvironment::from_string(&ambiente);
        let mut client = DGIIClient::new(env);
        match client.authenticate(&p12_der, &password).await {
            Ok(_) => match client.send_with_polling(&signed.signed_xml, &format!("{}{}.xml", claims.tenant_id, e_ncf)).await {
                Ok(tracking) => (tracking.estado, Some(tracking.track_id), None),
                Err(e) => ("CONTINGENCIA_PENDIENTE".to_string(), None, Some(e.to_string())),
            },
            Err(e) => ("CONTINGENCIA_PENDIENTE".to_string(), None, Some(e.to_string())),
        }
    } else {
        ("FIRMADO_NO_ENVIADO".to_string(), None, None)
    };

    state.ecf_service.registrar_documento(services::ecf_service::NuevoDocumento {
        tenant_id: &claims.tenant_id,
        referencia_tipo: "NOTA_CREDITO",
        referencia_id: nota.id,
        tipo_ecf,
        e_ncf: &e_ncf,
        xml_firmado: &signed.signed_xml,
        estado_dgii: &estado_dgii,
        track_id: track_id.as_deref(),
        codigo_seguridad: &signed.codigo_seguridad,
        qr_url: &qr_url,
        mensaje_dgii: mensaje_dgii.as_deref(),
    }).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let nota = state.ventas_service.set_nota_credito_ecf_result(
        &claims.tenant_id, nota.id, &e_ncf, &estado_dgii, Some(&signed.codigo_seguridad), Some(&qr_url),
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(nota))
}

async fn http_get_nota_credito(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::ventas_service::NotaCredito>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.ventas_service.get_nota_credito(&claims.tenant_id, id).await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

// ------------------ MODULO 5b: Cotizaciones ------------------

#[derive(Debug, Serialize)]
struct CotizacionCompletaResponse {
    #[serde(flatten)]
    cotizacion: services::cotizacion_service::Cotizacion,
    items: Vec<services::cotizacion_service::CotizacionItem>,
}

#[derive(Debug, Deserialize)]
struct ListCotizacionesParams {
    estado: Option<String>,
    #[serde(rename = "clienteId")] cliente_id: Option<Uuid>,
    search: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_cotizaciones(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListCotizacionesParams>,
) -> Result<Json<pagination::Page<services::cotizacion_service::CotizacionConCliente>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (cotizaciones, total) = state.cotizacion_service.list_cotizaciones(
        &claims.tenant_id, params.estado, params.cliente_id, params.search, params.fecha_desde, params.fecha_hasta, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(cotizaciones, page.page_number(), page_size, total)))
}

async fn http_create_cotizacion(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::cotizacion_service::CreateCotizacionRequest>,
) -> Result<Json<CotizacionCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;

    let completa = state.cotizacion_service.create_cotizacion(&claims.tenant_id, usuario_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "COTIZACION_CREADA", "cotizacion", Some(completa.cotizacion.id),
        serde_json::json!({ "total": completa.cotizacion.total })).await;
    Ok(Json(CotizacionCompletaResponse { cotizacion: completa.cotizacion, items: completa.items }))
}

async fn http_get_cotizacion(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CotizacionCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let completa = state.cotizacion_service.get_cotizacion(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(CotizacionCompletaResponse { cotizacion: completa.cotizacion, items: completa.items }))
}

async fn http_rechazar_cotizacion(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::cotizacion_service::Cotizacion>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let cotizacion = state.cotizacion_service.marcar_estado(&claims.tenant_id, id, "RECHAZADA").await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(cotizacion))
}

#[derive(Debug, Deserialize)]
struct ConvertirCotizacionRequest {
    metodo_pago: Option<String>,
    tipo_ecf: Option<i32>,
    aprobacion_admin: Option<AprobacionAdmin>,
}

/// Convierte una cotización en Venta real reutilizando
/// `ventas_service::create_venta` sin cambios - así toda venta que salga de
/// aquí pasa por el mismo gate de caja abierta, límite de descuento y
/// crédito fiado que cualquier venta hecha desde el POS.
async fn http_convertir_cotizacion(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<ConvertirCotizacionRequest>,
) -> Result<Json<VentaCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;

    let cotizacion_completa = state.cotizacion_service.get_cotizacion(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    if cotizacion_completa.cotizacion.estado == "CONVERTIDA" {
        return Err((StatusCode::BAD_REQUEST, "Esta cotización ya fue convertida en venta".to_string()));
    }
    if cotizacion_completa.cotizacion.estado == "RECHAZADA" {
        return Err((StatusCode::BAD_REQUEST, "Esta cotización fue rechazada".to_string()));
    }

    let venta_req = services::ventas_service::CreateVentaRequest {
        cliente_id: cotizacion_completa.cotizacion.cliente_id,
        items: cotizacion_completa.items.iter().map(|it| services::ventas_service::CreateVentaItemRequest {
            producto_id: it.producto_id,
            cantidad: it.cantidad,
            descuento: Some(it.descuento),
        }).collect(),
        metodo_pago: req.metodo_pago,
        tipo_ecf: req.tipo_ecf,
        entrega_diferida: None,
    };

    let aprobado_por = match req.aprobacion_admin {
        Some(a) => Some(
            state.auth_service.verify_admin_credentials(&claims.tenant_id, &a.email, &a.password).await
                .map_err(|e| (StatusCode::FORBIDDEN, e.to_string()))?,
        ),
        None => None,
    };

    let completa = state.ventas_service.create_venta(&claims.tenant_id, usuario_id, &claims.rol, venta_req, aprobado_por).await
        .map_err(|e| {
            if e.downcast_ref::<services::ventas_service::DescuentoRequiereAprobacion>().is_some() {
                (StatusCode::FORBIDDEN, e.to_string())
            } else {
                (StatusCode::BAD_REQUEST, e.to_string())
            }
        })?;

    state.cotizacion_service.marcar_convertida(&claims.tenant_id, id, completa.venta.id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "COTIZACION_CONVERTIDA", "cotizacion", Some(id),
        serde_json::json!({ "venta_id": completa.venta.id, "total": completa.venta.total })).await;

    Ok(Json(VentaCompletaResponse { venta: completa.venta, items: completa.items }))
}

// ------------------ MODULO 5c: Conduces ------------------

#[derive(Debug, Serialize)]
struct ConduceCompletaResponse {
    #[serde(flatten)]
    conduce: services::conduce_service::Conduce,
    items: Vec<services::conduce_service::ConduceItem>,
}

#[derive(Debug, Deserialize)]
struct ListConducesQuery {
    venta_id: Option<Uuid>,
    search: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_conduces(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(q): Query<ListConducesQuery>,
) -> Result<Json<pagination::Page<services::conduce_service::ConduceConVenta>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: q.page, page_size: q.page_size };
    let sort = pagination::SortParams { sort_by: q.sort_by, sort_dir: q.sort_dir };
    let (conduces, total) = state.conduce_service.list_conduces(
        &claims.tenant_id, q.venta_id, q.search, q.fecha_desde, q.fecha_hasta, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(conduces, page.page_number(), page_size, total)))
}

async fn http_create_conduce(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::conduce_service::CreateConduceRequest>,
) -> Result<Json<ConduceCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;

    let venta_id = req.venta_id;
    let completa = state.conduce_service.create_conduce(&claims.tenant_id, usuario_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "CONDUCE_CREADO", "conduce", Some(completa.conduce.id),
        serde_json::json!({ "venta_id": venta_id })).await;
    Ok(Json(ConduceCompletaResponse { conduce: completa.conduce, items: completa.items }))
}

async fn http_get_conduce(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<ConduceCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let completa = state.conduce_service.get_conduce(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(ConduceCompletaResponse { conduce: completa.conduce, items: completa.items }))
}

#[derive(Debug, Deserialize)]
struct ListEcfDocumentosParams {
    #[serde(rename = "estadoDgii")] estado_dgii: Option<String>,
    #[serde(rename = "tipoEcf")] tipo_ecf: Option<i32>,
    #[serde(rename = "referenciaTipo")] referencia_tipo: Option<String>,
    search: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_ecf_documentos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListEcfDocumentosParams>,
) -> Result<Json<pagination::Page<services::ecf_service::EcfDocumento>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (docs, total) = state.ecf_service.list_documentos(
        &claims.tenant_id, params.estado_dgii, params.tipo_ecf, params.referencia_tipo, params.search, params.fecha_desde, params.fecha_hasta, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(docs, page.page_number(), page_size, total)))
}

/// Imprime el ticket de una venta (e-CF si ya fue emitido, o ticket simple si
/// no) en la impresora de red configurada en Configuración → Impresora.
async fn http_imprimir_venta(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let completa = state.ventas_service.get_venta(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    let tenant = state.auth_service.get_tenant(&claims.tenant_id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;

    let cliente = match completa.venta.cliente_id {
        Some(cid) => state.partner_service.get_cliente(&claims.tenant_id, cid).await.ok(),
        None => None,
    };

    let impresora = state.config_service.get_impresora(&claims.tenant_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let recibo = recibo_builder::ReciboVenta {
        emisor: recibo_builder::ReciboEmisor {
            razon_social: tenant.razon_social,
            rnc: claims.tenant_id.clone(),
            direccion: tenant.direccion,
            telefono: tenant.telefono,
        },
        cliente: recibo_builder::ReciboCliente {
            nombre: cliente.as_ref().map(|c| c.nombre.clone()).unwrap_or_else(|| "CONSUMIDOR FINAL".to_string()),
            rnc_cedula: cliente.and_then(|c| c.rnc_cedula),
        },
        items: completa.items.iter().map(|it| recibo_builder::ReciboLinea {
            nombre: it.nombre.clone(),
            cantidad: it.cantidad,
            precio_unitario: it.precio_unitario,
            subtotal: it.subtotal,
        }).collect(),
        subtotal: completa.venta.subtotal,
        itbis_total: completa.venta.itbis_total,
        total: completa.venta.total,
        metodo_pago: completa.venta.metodo_pago.clone(),
        fecha_emision: completa.venta.created_at.with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M").to_string(),
        fiscal: match (&completa.venta.e_ncf, &completa.venta.codigo_seguridad, &completa.venta.qr_url) {
            (Some(e_ncf), Some(codigo), Some(qr)) => Some(recibo_builder::ReciboFiscal {
                e_ncf: e_ncf.clone(),
                tipo_ecf: completa.venta.tipo_ecf.unwrap_or(32),
                fecha_vencimiento_secuencia: completa.venta.created_at.format("%d-%m-%Y").to_string(),
                codigo_seguridad: codigo.clone(),
                qr_url: qr.clone(),
                fecha_firma: completa.venta.created_at.with_timezone(&chrono::Local).format("%d-%m-%Y %H:%M:%S").to_string(),
            }),
            _ => None,
        },
    };

    let bytes = recibo_builder::build_recibo_escpos(&recibo, impresora.ancho_mm, impresora.copias);

    state.config_service.imprimir_bytes(&claims.tenant_id, &bytes).await
        .map_err(|e| (StatusCode::BAD_GATEWAY, e.to_string()))?;

    Ok(Json(serde_json::json!({ "impreso": true })))
}

// ------------------ MODULO 6: Compras y Gastos ------------------

#[derive(Debug, Serialize)]
struct CompraCompletaResponse {
    #[serde(flatten)]
    compra: services::compras_service::Compra,
    items: Vec<services::compras_service::CompraItem>,
}

#[derive(Debug, Deserialize)]
struct ListComprasParams {
    #[serde(rename = "proveedorId")] proveedor_id: Option<Uuid>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_compras(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListComprasParams>,
) -> Result<Json<pagination::Page<services::compras_service::CompraConProveedor>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (compras, total) = state.compras_service.list_compras(
        &claims.tenant_id,
        params.proveedor_id,
        params.fecha_desde,
        params.fecha_hasta,
        &page,
        &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(compras, page.page_number(), page_size, total)))
}

async fn http_create_compra(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::compras_service::CreateCompraRequest>,
) -> Result<Json<CompraCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let completa = state.compras_service.create_compra(&claims.tenant_id, usuario_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(CompraCompletaResponse { compra: completa.compra, items: completa.items }))
}

async fn http_get_compra(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<CompraCompletaResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let completa = state.compras_service.get_compra(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(CompraCompletaResponse { compra: completa.compra, items: completa.items }))
}

/// Anula una compra para que quede excluida del 606 (ver
/// ComprasService::anular_compra). Si la compra ya fue reportada en un
/// período anterior, la corrección correcta es una Nota de Crédito de
/// compra (POST /v1/compras con tipo_documento=NOTA_CREDITO), no anular.
async fn http_anular_compra(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::compras_service::Compra>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let compra = state.compras_service.anular_compra(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(compra))
}

#[derive(Debug, Deserialize)]
struct ListGastosParams {
    categoria: Option<String>,
    #[serde(rename = "proveedorId")] proveedor_id: Option<Uuid>,
    search: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_gastos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListGastosParams>,
) -> Result<Json<pagination::Page<services::compras_service::Gasto>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (gastos, total) = state.compras_service.list_gastos(
        &claims.tenant_id, params.categoria, params.proveedor_id, params.search, params.fecha_desde, params.fecha_hasta, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(gastos, page.page_number(), page_size, total)))
}

async fn http_create_gasto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::compras_service::CreateGastoRequest>,
) -> Result<Json<services::compras_service::Gasto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    state.compras_service.create_gasto(&claims.tenant_id, usuario_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

// ------------------ MODULO 9: Caja y Bancos ------------------

async fn http_caja_resumen(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::caja_service::CajaResumen>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.caja_service.resumen(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_caja_abrir(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::caja_service::AbrirCajaRequest>,
) -> Result<Json<services::caja_service::CajaSesion>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    state.caja_service.abrir(&claims.tenant_id, usuario_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_caja_cerrar(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::caja_service::CerrarCajaRequest>,
) -> Result<Json<services::caja_service::CajaSesion>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.caja_service.cerrar(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

#[derive(Debug, Deserialize)]
struct ListCajaMovimientosParams {
    tipo: Option<String>,
    #[serde(rename = "referenciaTipo")] referencia_tipo: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_caja_movimientos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListCajaMovimientosParams>,
) -> Result<Json<pagination::Page<services::caja_service::CajaMovimiento>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (movimientos, total) = state.caja_service.list_movimientos(
        &claims.tenant_id, params.tipo, params.referencia_tipo, params.fecha_desde, params.fecha_hasta, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(movimientos, page.page_number(), page_size, total)))
}

#[derive(Debug, Deserialize)]
struct ListCajaSesionesParams {
    estado: Option<String>,
    #[serde(rename = "usuarioId")] usuario_id: Option<Uuid>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_caja_sesiones(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListCajaSesionesParams>,
) -> Result<Json<pagination::Page<services::caja_service::CajaSesion>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (sesiones, total) = state.caja_service.list_sesiones(&claims.tenant_id, params.estado, params.usuario_id, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(sesiones, page.page_number(), page_size, total)))
}

#[derive(Debug, Deserialize)]
struct ListBancosParams {
    search: Option<String>,
    activo: Option<bool>,
    #[serde(rename = "tipoCuenta")] tipo_cuenta: Option<String>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_bancos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListBancosParams>,
) -> Result<Json<pagination::Page<services::caja_service::Banco>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (bancos, total) = state.bancos_service.list(&claims.tenant_id, params.search, params.activo, params.tipo_cuenta, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(bancos, page.page_number(), page_size, total)))
}

async fn http_create_banco(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::caja_service::CreateBancoRequest>,
) -> Result<Json<services::caja_service::Banco>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.bancos_service.create(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_list_banco_movimientos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Query(params): Query<PageSortParams>,
) -> Result<Json<pagination::Page<services::caja_service::BancoMovimiento>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (movimientos, total) = state.bancos_service.list_movimientos(&claims.tenant_id, id, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(movimientos, page.page_number(), page_size, total)))
}

async fn http_create_banco_movimiento(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::caja_service::CreateBancoMovimientoRequest>,
) -> Result<Json<services::caja_service::BancoMovimiento>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    state.bancos_service.create_movimiento(&claims.tenant_id, id, usuario_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

// ------------------ MODULO 8: Nomina y Adelantos ------------------

#[derive(Debug, Deserialize)]
struct ListEmpleadosParams {
    search: Option<String>,
    puesto: Option<String>,
    activo: Option<bool>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_empleados(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListEmpleadosParams>,
) -> Result<Json<pagination::Page<services::nomina_service::EmpleadoConDisponible>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (empleados, total) = state.nomina_service.list_empleados(&claims.tenant_id, params.search, params.puesto, params.activo, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(empleados, page.page_number(), page_size, total)))
}

async fn http_get_empleado(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::nomina_service::Empleado>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.nomina_service.get_empleado(&claims.tenant_id, id).await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

async fn http_create_empleado(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::nomina_service::CreateEmpleadoRequest>,
) -> Result<Json<services::nomina_service::Empleado>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.nomina_service.create_empleado(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_update_empleado(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::nomina_service::UpdateEmpleadoRequest>,
) -> Result<Json<services::nomina_service::Empleado>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.nomina_service.update_empleado(&claims.tenant_id, id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_delete_empleado(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.nomina_service.delete_empleado(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

#[derive(Debug, Deserialize)]
struct ListAdelantosParams {
    estado: Option<String>,
    #[serde(rename = "empleadoId")] empleado_id: Option<Uuid>,
    search: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_adelantos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListAdelantosParams>,
) -> Result<Json<pagination::Page<services::nomina_service::AdelantoConEmpleado>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (adelantos, total) = state.nomina_service.list_adelantos(
        &claims.tenant_id, params.estado, params.empleado_id, params.search, params.fecha_desde, params.fecha_hasta, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(adelantos, page.page_number(), page_size, total)))
}

async fn http_request_adelanto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::nomina_service::CreateAdelantoRequest>,
) -> Result<Json<services::nomina_service::Adelanto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    state.nomina_service.request_adelanto(&claims.tenant_id, usuario_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_approve_adelanto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::nomina_service::Adelanto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let adelanto = state.nomina_service.approve_adelanto(&claims.tenant_id, id, usuario_id).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "ADELANTO_APROBADO", "adelanto", Some(id),
        serde_json::json!({ "monto": adelanto.monto })).await;
    Ok(Json(adelanto))
}

async fn http_reject_adelanto(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<services::nomina_service::Adelanto>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).ok();
    let adelanto = state.nomina_service.reject_adelanto(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, usuario_id, "ADELANTO_RECHAZADO", "adelanto", Some(id), serde_json::json!({})).await;
    Ok(Json(adelanto))
}

async fn http_list_periodos(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let periodos = state.nomina_service.list_periodos(&claims.tenant_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "total": periodos.len(), "periodos": periodos })))
}

#[derive(Debug, Serialize)]
struct PeriodoConDetalles {
    #[serde(flatten)]
    periodo: services::nomina_service::NominaPeriodo,
    detalles: Vec<services::nomina_service::NominaDetalle>,
}

async fn http_get_periodo(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<Json<PeriodoConDetalles>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let (periodo, detalles) = state.nomina_service.get_periodo(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))?;
    Ok(Json(PeriodoConDetalles { periodo, detalles }))
}

async fn http_run_payroll(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::nomina_service::RunPayrollRequest>,
) -> Result<Json<PeriodoConDetalles>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let (periodo, detalles) = state.nomina_service.run_payroll(&claims.tenant_id, usuario_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "NOMINA_CORRIDA", "nomina_periodo", Some(periodo.id),
        serde_json::json!({ "total_neto": periodo.total_neto, "empleados": detalles.len() })).await;
    Ok(Json(PeriodoConDetalles { periodo, detalles }))
}

// ------------------ MODULO 7: Contabilidad (Libro Mayor) ------------------

#[derive(Debug, Deserialize)]
struct AsientosParams {
    cuenta: Option<String>,
    #[serde(rename = "referenciaTipo")] referencia_tipo: Option<String>,
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_list_asientos(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<AsientosParams>,
) -> Result<Json<pagination::Page<services::contabilidad_service::Asiento>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (asientos, total) = state.contabilidad_service.list_asientos(
        &claims.tenant_id, params.cuenta, params.referencia_tipo, params.fecha_desde, params.fecha_hasta, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(asientos, page.page_number(), page_size, total)))
}

async fn http_create_asiento(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::contabilidad_service::CreateAsientoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let asientos = state.contabilidad_service.create_asiento_manual(&claims.tenant_id, usuario_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    Ok(Json(serde_json::json!({ "asientos": asientos })))
}

#[derive(Debug, Deserialize)]
struct LibroMayorParams {
    search: Option<String>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_libro_mayor(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<LibroMayorParams>,
) -> Result<Json<pagination::Page<services::contabilidad_service::CuentaResumen>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (cuentas, total) = state.contabilidad_service.libro_mayor(&claims.tenant_id, params.search, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(cuentas, page.page_number(), page_size, total)))
}

async fn http_sincronizar_contabilidad(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::contabilidad_service::SincronizarResultado>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.contabilidad_service.sincronizar(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

#[derive(Debug, Deserialize)]
struct ReversarAsientoRequest {
    motivo: Option<String>,
}

async fn http_reversar_asiento(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<ReversarAsientoRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let asientos = state.contabilidad_service.reversar_asiento(&claims.tenant_id, usuario_id, id, req.motivo).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "ASIENTO_REVERSADO", "asiento", Some(id), serde_json::json!({})).await;
    Ok(Json(serde_json::json!({ "asientos": asientos })))
}

#[derive(Debug, Deserialize)]
struct LibroMayorDetalleParams {
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
}

async fn http_libro_mayor_detalle(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(cuenta): Path<String>,
    Query(params): Query<LibroMayorDetalleParams>,
) -> Result<Json<pagination::Page<services::contabilidad_service::MovimientoCuenta>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let (movimientos, total) = state.contabilidad_service.libro_mayor_detalle(
        &claims.tenant_id, &cuenta, params.fecha_desde, params.fecha_hasta, &page,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(movimientos, page.page_number(), page_size, total)))
}

#[derive(Debug, Deserialize)]
struct LibroDiarioParams {
    #[serde(rename = "fechaDesde")] fecha_desde: Option<chrono::NaiveDate>,
    #[serde(rename = "fechaHasta")] fecha_hasta: Option<chrono::NaiveDate>,
    origen: Option<String>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_libro_diario(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<LibroDiarioParams>,
) -> Result<Json<pagination::Page<services::contabilidad_service::AsientoConLineas>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (asientos, total) = state.contabilidad_service.libro_diario(
        &claims.tenant_id, params.fecha_desde, params.fecha_hasta, params.origen, &page, &sort,
    ).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(asientos, page.page_number(), page_size, total)))
}

async fn http_list_cuentas(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<Vec<services::contabilidad_service::CuentaContable>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.contabilidad_service.list_cuentas(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_list_periodos_contables(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<Vec<services::contabilidad_service::Periodo>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.contabilidad_service.list_periodos(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_cerrar_periodo(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path((anio, mes)): Path<(i32, i32)>,
) -> Result<Json<services::contabilidad_service::Periodo>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let usuario_id = Uuid::parse_str(&claims.sub).map_err(|e| (StatusCode::UNAUTHORIZED, format!("Token inválido: {}", e)))?;
    let periodo = state.contabilidad_service.cerrar_periodo(&claims.tenant_id, anio, mes, usuario_id).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, Some(usuario_id), "PERIODO_CERRADO", "periodo_contable", Some(periodo.id),
        serde_json::json!({ "anio": anio, "mes": mes })).await;
    Ok(Json(periodo))
}

// ------------------ MODULO 10: Reportes y Dashboard ------------------

#[derive(Debug, Deserialize)]
struct ReportParams {
    period: String,
}

async fn http_report_606(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ReportParams>,
) -> Result<String, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.report_service.generate_606(&claims.tenant_id, &params.period).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

#[derive(Debug, Deserialize)]
struct ReportCsvParams {
    #[serde(rename = "fechaDesde")] fecha_desde: chrono::NaiveDate,
    #[serde(rename = "fechaHasta")] fecha_hasta: chrono::NaiveDate,
}

fn csv_response(csv: String, filename: &str) -> Result<Response, (StatusCode, String)> {
    Response::builder()
        .status(StatusCode::OK)
        .header(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8")
        .header(axum::http::header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
        .body(boxed(Full::from(csv)))
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_report_606_csv(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ReportCsvParams>,
) -> Result<Response, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let csv = state.report_service.generate_606_csv(&claims.tenant_id, params.fecha_desde, params.fecha_hasta).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    csv_response(csv, &format!("606_{}_{}.csv", params.fecha_desde, params.fecha_hasta))
}

async fn http_dashboard_resumen(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::report_service::DashboardResumen>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.report_service.dashboard_resumen(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Infalible - `AiService::digest_diario` ya resuelve internamente cualquier
/// fallo (Ollama caído/lento) con un mensaje de respaldo, así que este
/// handler no tiene rama de error.
async fn http_ai_digest(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::ai_service::DigestResult>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    Ok(Json(state.ai_service.digest_diario(&claims.tenant_id).await))
}

#[derive(Debug, Deserialize)]
struct ChatRequest {
    mensaje: String,
}

/// Infalible - `AiService::chat` ya resuelve internamente cualquier fallo
/// (Ollama caído/lento/JSON mal formado) con una respuesta de texto plano,
/// nunca con un error. Las acciones propuestas solo llenan un formulario
/// existente en el frontend - la creación real sigue pasando por su propio
/// endpoint con su propio role_guard, esto nunca ejecuta una mutación.
async fn http_ai_chat(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<ChatRequest>,
) -> Result<Json<services::ai_service::ChatResponse>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    Ok(Json(state.ai_service.chat(&claims.tenant_id, &claims.rol, &req.mensaje).await))
}

// ------------------ MODULO 11: Configuracion DGII y Empresa ------------------

async fn http_get_empresa(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::config_service::Empresa>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.get_empresa(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::NOT_FOUND, e.to_string()))
}

async fn http_update_empresa(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::config_service::UpdateEmpresaRequest>,
) -> Result<Json<services::config_service::Empresa>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.update_empresa(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

#[derive(Debug, Deserialize)]
struct ListUsuariosParams {
    search: Option<String>,
    rol: Option<String>,
    activo: Option<bool>,
    page: Option<i64>,
    #[serde(rename = "pageSize")] page_size: Option<i64>,
    #[serde(rename = "sortBy")] sort_by: Option<String>,
    #[serde(rename = "sortDir")] sort_dir: Option<String>,
}

async fn http_config_list_usuarios(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Query(params): Query<ListUsuariosParams>,
) -> Result<Json<pagination::Page<services::config_service::Usuario>>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let page = pagination::PageParams { page: params.page, page_size: params.page_size };
    let sort = pagination::SortParams { sort_by: params.sort_by, sort_dir: params.sort_dir };
    let (usuarios, total) = state.config_service.list_usuarios(&claims.tenant_id, params.search, params.rol, params.activo, &page, &sort).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let page_size = page.limit(20);
    Ok(Json(pagination::Page::new(usuarios, page.page_number(), page_size, total)))
}

async fn http_config_create_usuario(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::config_service::CreateUsuarioRequest>,
) -> Result<Json<services::config_service::Usuario>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let admin_id = Uuid::parse_str(&claims.sub).ok();
    let usuario = state.config_service.create_usuario(&claims.tenant_id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, admin_id, "USUARIO_CREADO", "usuario", Some(usuario.id),
        serde_json::json!({ "email": usuario.email, "rol": usuario.rol })).await;
    Ok(Json(usuario))
}

async fn http_config_update_usuario(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<services::config_service::UpdateUsuarioRequest>,
) -> Result<Json<services::config_service::Usuario>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let admin_id = Uuid::parse_str(&claims.sub).ok();
    let usuario = state.config_service.update_usuario(&claims.tenant_id, id, req).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, admin_id, "USUARIO_ACTUALIZADO", "usuario", Some(id),
        serde_json::json!({ "rol": usuario.rol, "activo": usuario.activo })).await;
    Ok(Json(usuario))
}

async fn http_config_deactivate_usuario(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let admin_id = Uuid::parse_str(&claims.sub).ok();
    state.config_service.deactivate_usuario(&claims.tenant_id, id).await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    state.audit_service.log(&claims.tenant_id, admin_id, "USUARIO_DESACTIVADO", "usuario", Some(id), serde_json::json!({})).await;
    Ok(StatusCode::NO_CONTENT)
}

async fn http_list_secuencias(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let secuencias = state.config_service.list_secuencias(&claims.tenant_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "total": secuencias.len(), "secuencias": secuencias })))
}

async fn http_create_secuencia(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::config_service::CreateSecuenciaRequest>,
) -> Result<Json<services::config_service::SecuenciaNcf>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.create_secuencia(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

#[derive(Debug, Deserialize)]
struct SetEstadoSecuenciaRequest {
    estado: String,
}

async fn http_set_secuencia_estado(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(id): Path<Uuid>,
    Json(req): Json<SetEstadoSecuenciaRequest>,
) -> Result<Json<services::config_service::SecuenciaNcf>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.set_secuencia_estado(&claims.tenant_id, id, &req.estado).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_certificado_status(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    let status = state.config_service.certificado_status(&claims.tenant_id).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok(Json(serde_json::json!({ "certificado": status })))
}

async fn http_upload_certificado(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::config_service::UploadCertificadoRequest>,
) -> Result<Json<services::config_service::CertificadoStatus>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.upload_certificado(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_get_impresora(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::config_service::ImpresoraConfig>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.get_impresora(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

async fn http_update_impresora(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Json(req): Json<services::config_service::UpdateImpresoraRequest>,
) -> Result<Json<services::config_service::ImpresoraConfig>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.update_impresora(&claims.tenant_id, req).await
        .map(Json)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
}

async fn http_test_impresora(
    State(state): State<HttpState>,
    headers: HeaderMap,
) -> Result<Json<services::config_service::TestImpresoraResult>, (StatusCode, String)> {
    let claims = claims_from_headers(&state.auth_service, &headers)?;
    state.config_service.test_impresora(&claims.tenant_id).await
        .map(Json)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}

/// Busca un RNC/Cédula en el padrón DGII importado localmente (ver
/// bin/import_rnc.rs). Requiere sesión válida pero el dato en sí es público
/// y no está tenant-scoped - es el mismo registro nacional para todos.
async fn http_lookup_rnc(
    State(state): State<HttpState>,
    headers: HeaderMap,
    Path(rnc): Path<String>,
) -> Result<Json<services::rnc_service::RncRecord>, (StatusCode, String)> {
    claims_from_headers(&state.auth_service, &headers)?;
    state.rnc_service.lookup(&rnc).await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .ok_or((StatusCode::NOT_FOUND, "RNC/Cédula no encontrado en el padrón DGII".to_string()))
        .map(Json)
}

async fn http_test_sign_demo_get() -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // GET version for easy curl without body
    let xml = r#"<ECF><Encabezado><Version>1.0</Version><IdDoc><TipoeCF>32</TipoeCF><eNCF>E320000000001</eNCF><FechaVencimientoSecuencia>31-12-2026</FechaVencimientoSecuencia><IndicadorEnvioDiferido>1</IndicadorEnvioDiferido><TipoIngresos>01</TipoIngresos><TipoPago>1</TipoPago></IdDoc><Emisor><RNCEmisor>130793752</RNCEmisor><RazonSocialEmisor>COLMADO EL SOL SRL</RazonSocialEmisor><DireccionEmisor>Av Duarte</DireccionEmisor><FechaEmision>15-07-2026</FechaEmision></Emisor><Comprador><RNCComprador>000000000</RNCComprador><RazonSocialComprador>CONSUMIDOR FINAL</RazonSocialComprador></Comprador><Totales><MontoGravadoTotal>1000.00</MontoGravadoTotal><MontoGravadoI1>1000.00</MontoGravadoI1><TotalITBIS>180.00</TotalITBIS><MontoTotal>1180.00</MontoTotal></Totales></Encabezado><DetallesItems><Item><NumeroLinea>1</NumeroLinea><IndicadorFacturacion>1</IndicadorFacturacion><NombreItem>Arroz Premium</NombreItem><IndicadorBienoServicio>1</IndicadorBienoServicio><CantidadItem>1</CantidadItem><PrecioUnitarioItem>1000.00</PrecioUnitarioItem><MontoItem>1000.00</MontoItem></Item></DetallesItems></ECF>"#;
    let p12_der = generate_self_signed_p12().map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to gen P12: {}", e)))?;
    let signed = sign_xml_ecf(xml, &p12_der, "password").map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Signing failed: {}", e)))?;
    let qr_url = generate_qr_url("130793752", "E320000000001", "000000000", "15-07-2026", "1180.00", &signed.codigo_seguridad);
    Ok(Json(serde_json::json!({
        "e_ncf": "E320000000001",
        "track_id": format!("DEMO-GET-TRACK-{}", uuid::Uuid::new_v4()),
        "codigo_seguridad": signed.codigo_seguridad,
        "digest_value": signed.digest_value,
        "qr_url": qr_url,
        "signed_xml_preview": signed.signed_xml.chars().take(500).collect::<String>(),
        "mensaje": "Demo firma XAdES-BES real con cert auto-firmado (no válido para DGII prod, solo prueba builder+signer+QR)"
    })))
}


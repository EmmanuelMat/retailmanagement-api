//! Auth Service - Multi-tenancy con RNC como tenant_id
//! Registro negocio + Login con JWT + Roles
//! Event Sourcing: TenantRegistrado, UsuarioCreado

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::Engine as _;
use jsonwebtoken::{encode, decode, Header, Validation, EncodingKey, DecodingKey};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::{Utc, Duration};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // user_id
    pub tenant_id: String, // RNC
    pub rol: String, // ADMIN, CAJERO, ALMACEN, CONTADOR
    pub email: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tenant {
    pub rnc: String, // PK, 9-11 digitos
    pub razon_social: String,
    pub nombre_comercial: Option<String>,
    pub direccion: String,
    pub telefono: Option<String>,
    pub correo: Option<String>,
    pub logo_url: Option<String>,
    pub ambiente_dgii: String, // TesteCF, CerteCF, eCF
    pub factura_electronica_activa: bool,
    pub activo: bool,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Usuario {
    pub id: Uuid,
    pub tenant_id: String, // RNC
    pub nombre: String,
    pub email: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub rol: String, // ADMIN, CAJERO, ALMACEN, CONTADOR - etiqueta legible, ver rol_id para autorización real
    pub rol_id: Option<Uuid>,
    pub must_change_password: bool,
    pub activo: bool,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub rnc: String, // 130793752
    pub razon_social: String, // COLMADO EL SOL SRL
    pub direccion: String,
    pub telefono: Option<String>,
    pub correo: Option<String>,
    /// Si el negocio factura e-CF ante la DGII. `None`/ausente = true (comportamiento
    /// histórico); explícito en el formulario de registro para no asumirlo en silencio.
    pub factura_electronica_activa: Option<bool>,
    // Admin inicial
    pub admin_nombre: String, // Emmanuel Rosario
    pub admin_email: String,
    pub admin_password: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    #[serde(default)]
    pub rnc: Option<String>, // opcional, si no se envia, busca por email unico
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub usuario: UsuarioPublic,
    pub tenant: Tenant,
}

#[derive(Debug, Serialize)]
pub struct UsuarioPublic {
    pub id: Uuid,
    pub tenant_id: String,
    pub nombre: String,
    pub email: String,
    pub rol: String,
    pub rol_id: Option<Uuid>,
    /// Bypass total (equivalente al rol ADMIN de hoy) - ver `roles.es_admin`.
    pub es_admin: bool,
    /// Códigos de `permisos_catalogo` que tiene el rol de este usuario. No
    /// viaja en el JWT (ver Claims) - se recalcula en cada login/consulta,
    /// así que revocar un permiso aplica de inmediato, no en el próximo login.
    pub permisos: Vec<String>,
    pub must_change_password: bool,
    pub activo: bool,
}

pub struct AuthService {
    pool: PgPool,
    jwt_secret: String,
}

impl AuthService {
    pub fn new(pool: PgPool, jwt_secret: String) -> Self {
        Self { pool, jwt_secret }
    }

    // Hash password con Argon2
    fn hash_password(password: &str) -> anyhow::Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let hash = argon2.hash_password(password.as_bytes(), &salt)
            .map_err(|e| anyhow::anyhow!("Hash failed: {}", e))?;
        Ok(hash.to_string())
    }

    fn verify_password(hash: &str, password: &str) -> anyhow::Result<bool> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| anyhow::anyhow!("Invalid hash: {}", e))?;
        Ok(Argon2::default().verify_password(password.as_bytes(), &parsed_hash).is_ok())
    }

    fn generate_jwt(&self, usuario: &Usuario) -> anyhow::Result<String> {
        let now = Utc::now();
        let exp = now + Duration::hours(12); // token 12 horas
        let claims = Claims {
            sub: usuario.id.to_string(),
            tenant_id: usuario.tenant_id.clone(),
            rol: usuario.rol.clone(),
            email: usuario.email.clone(),
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_bytes()),
        )?;
        Ok(token)
    }

    pub fn verify_jwt(&self, token: &str) -> anyhow::Result<Claims> {
        let data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;
        Ok(data.claims)
    }

    /// Duplica la consulta de `RolesService::permisos_de_usuario` con
    /// `self.pool` directo en vez de inyectar RolesService aquí - evita
    /// reestructurar el constructor de AuthService por dos SELECT.
    async fn permisos_para(&self, usuario_id: Uuid) -> anyhow::Result<(bool, Vec<String>)> {
        let es_admin: Option<bool> = sqlx::query_scalar(
            "SELECT r.es_admin FROM usuarios u JOIN roles r ON r.id = u.rol_id WHERE u.id = $1",
        )
        .bind(usuario_id)
        .fetch_optional(&self.pool)
        .await?;
        let permisos: Vec<String> = sqlx::query_scalar(
            "SELECT rp.permiso_codigo FROM usuarios u JOIN role_permisos rp ON rp.role_id = u.rol_id WHERE u.id = $1",
        )
        .bind(usuario_id)
        .fetch_all(&self.pool)
        .await?;
        Ok((es_admin.unwrap_or(false), permisos))
    }

    pub async fn usuario_public(&self, u: &Usuario) -> anyhow::Result<UsuarioPublic> {
        let (es_admin, permisos) = self.permisos_para(u.id).await?;
        Ok(UsuarioPublic {
            id: u.id,
            tenant_id: u.tenant_id.clone(),
            nombre: u.nombre.clone(),
            email: u.email.clone(),
            rol: u.rol.clone(),
            rol_id: u.rol_id,
            es_admin,
            permisos,
            must_change_password: u.must_change_password,
            activo: u.activo,
        })
    }

    // Registro negocio + admin inicial - Todo en transaccion + eventos
    pub async fn register(&self, req: RegisterRequest) -> anyhow::Result<AuthResponse> {
        // Validar RNC: 9-11 digitos, solo numeros
        let rnc_clean = req.rnc.replace("-", "").replace(" ", "");
        if rnc_clean.len() < 9 || rnc_clean.len() > 11 || !rnc_clean.chars().all(|c| c.is_numeric()) {
            anyhow::bail!("RNC inválido: debe ser 9-11 dígitos numéricos");
        }

        // Verificar si tenant ya existe
        let existing: Option<(String,)> = sqlx::query_as("SELECT rnc FROM tenants WHERE rnc = $1")
            .bind(&rnc_clean)
            .fetch_optional(&self.pool)
            .await?;
        if existing.is_some() {
            anyhow::bail!("RNC ya registrado: {}", rnc_clean);
        }

        // Verificar email no existe en ese tenant
        let existing_user: Option<(String,)> = sqlx::query_as("SELECT email FROM usuarios WHERE email = $1 AND tenant_id = $2")
            .bind(&req.admin_email.to_lowercase())
            .bind(&rnc_clean)
            .fetch_optional(&self.pool)
            .await?;
        if existing_user.is_some() {
            anyhow::bail!("Email ya registrado en este RNC: {}", req.admin_email);
        }

        let tenant_id = rnc_clean.clone();
        let user_id = Uuid::new_v4();
        let password_hash = Self::hash_password(&req.admin_password)?;
        let now = Utc::now();

        // Transaccion: crear tenant + usuario + eventos
        let mut tx = self.pool.begin().await?;

        // 1. Crear tenant
        sqlx::query(
            r#"INSERT INTO tenants (rnc, razon_social, direccion, telefono, correo, ambiente_dgii, factura_electronica_activa, activo, created_at)
               VALUES ($1, $2, $3, $4, $5, 'TesteCF', $6, true, $7)"#
        )
        .bind(&rnc_clean)
        .bind(&req.razon_social)
        .bind(&req.direccion)
        .bind(&req.telefono)
        .bind(&req.correo)
        .bind(req.factura_electronica_activa.unwrap_or(true))
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // 2. Crear usuario admin - rol_id resuelto por subquery contra el rol
        // ADMIN sembrado por la migración (ver roles_service.rs/migrate.rs).
        sqlx::query(
            r#"INSERT INTO usuarios (id, tenant_id, nombre, email, password_hash, rol, rol_id, activo, created_at)
               VALUES ($1, $2, $3, $4, $5, 'ADMIN', (SELECT id FROM roles WHERE codigo = 'ADMIN'), true, $6)"#
        )
        .bind(user_id)
        .bind(&rnc_clean)
        .bind(&req.admin_nombre)
        .bind(&req.admin_email.to_lowercase())
        .bind(&password_hash)
        .bind(now)
        .execute(&mut *tx)
        .await?;

        // 3. Eventos Event Sourcing (append to events table)
        let tenant_event = serde_json::json!({
            "rnc": rnc_clean,
            "razonSocial": req.razon_social,
            "direccion": req.direccion,
            "adminEmail": req.admin_email
        });
        sqlx::query(
            r#"INSERT INTO events (aggregate_type, aggregate_id, version, event_type, payload, metadata, tenant_id, prev_hash, hash)
               VALUES ('Tenant', $1, 1, 'TenantRegistrado', $2, $3, $4, '0', $5)"#
        )
        .bind(Uuid::new_v4())
        .bind(&tenant_event)
        .bind(serde_json::json!({"source": "register", "email": req.admin_email}))
        .bind(&rnc_clean)
        .bind(format!("hash_{}", Uuid::new_v4())) // simplified hash, real should be SHA256(prev+payload)
        .execute(&mut *tx)
        .await?;

        let user_event = serde_json::json!({
            "usuarioId": user_id,
            "tenantId": rnc_clean,
            "nombre": req.admin_nombre,
            "email": req.admin_email,
            "rol": "ADMIN"
        });
        sqlx::query(
            r#"INSERT INTO events (aggregate_type, aggregate_id, version, event_type, payload, metadata, tenant_id, prev_hash, hash)
               VALUES ('Usuario', $1, 1, 'UsuarioCreado', $2, $3, $4, '0', $5)"#
        )
        .bind(user_id)
        .bind(&user_event)
        .bind(serde_json::json!({"source": "register"}))
        .bind(&rnc_clean)
        .bind(format!("hash_{}", Uuid::new_v4()))
        .execute(&mut *tx)
        .await?;

        // 4. Plan de cuentas inicial (ver docs/12-LIBRO-DIARIO-LIBRO-MAYOR-PLAN.md).
        // Misma lista que la semilla de backfill en bin/migrate.rs - si se
        // agrega una cuenta ahí, agrégala aquí también.
        const PLAN_DE_CUENTAS: &[(&str, &str, &str, &str)] = &[
            ("1100", "Caja y Bancos", "ACTIVO", "DEUDORA"),
            ("1110", "Cuentas por Cobrar", "ACTIVO", "DEUDORA"),
            ("1150", "ITBIS Adelantado", "ACTIVO", "DEUDORA"),
            ("1160", "Depósitos Bancarios", "ACTIVO", "DEUDORA"),
            ("1200", "Inventario", "ACTIVO", "DEUDORA"),
            ("1300", "Anticipos a Empleados", "ACTIVO", "DEUDORA"),
            ("2100", "ITBIS por Pagar", "PASIVO", "ACREEDORA"),
            ("2200", "Retenciones y Descuentos", "PASIVO", "ACREEDORA"),
            ("4100", "Ingresos por Ventas", "INGRESO", "ACREEDORA"),
            ("4200", "Otros Ingresos", "INGRESO", "ACREEDORA"),
            ("5050", "Costo de Ventas", "GASTO", "DEUDORA"),
            ("5100", "Gasto de Nómina", "GASTO", "DEUDORA"),
            ("5210", "Gasto de Alquiler", "GASTO", "DEUDORA"),
            ("5220", "Gasto de Servicios", "GASTO", "DEUDORA"),
            ("5230", "Gasto de Transporte", "GASTO", "DEUDORA"),
            ("5290", "Otros Gastos Operativos", "GASTO", "DEUDORA"),
            ("5295", "Ajuste de Inventario (Merma)", "GASTO", "DEUDORA"),
        ];
        for (codigo, nombre, tipo, naturaleza) in PLAN_DE_CUENTAS {
            sqlx::query(
                r#"INSERT INTO cuentas_contables (tenant_id, codigo, nombre, tipo, naturaleza)
                   VALUES ($1, $2, $3, $4, $5)"#,
            )
            .bind(&rnc_clean)
            .bind(codigo)
            .bind(nombre)
            .bind(tipo)
            .bind(naturaleza)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        // Fetch created records
        let tenant = self.get_tenant(&rnc_clean).await?;
        let usuario = self.get_usuario_by_id(user_id).await?;
        let token = self.generate_jwt(&usuario)?;
        let usuario_public = self.usuario_public(&usuario).await?;

        Ok(AuthResponse {
            token,
            usuario: usuario_public,
            tenant,
        })
    }

    pub async fn login(&self, req: LoginRequest) -> anyhow::Result<AuthResponse> {
        // Buscar usuario por email (y opcional RNC)
        let usuario_row = if let Some(rnc) = &req.rnc {
            sqlx::query_as::<_, UsuarioRow>(
                "SELECT id, tenant_id, nombre, email, password_hash, rol, rol_id, must_change_password, activo, created_at FROM usuarios WHERE email = $1 AND tenant_id = $2 AND activo = true"
            )
            .bind(req.email.to_lowercase())
            .bind(rnc.replace("-", ""))
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, UsuarioRow>(
                "SELECT id, tenant_id, nombre, email, password_hash, rol, rol_id, must_change_password, activo, created_at FROM usuarios WHERE email = $1 AND activo = true LIMIT 1"
            )
            .bind(req.email.to_lowercase())
            .fetch_optional(&self.pool)
            .await?
        };

        let row = usuario_row.ok_or_else(|| anyhow::anyhow!("Credenciales inválidas: email no encontrado"))?;

        if !Self::verify_password(&row.password_hash, &req.password)? {
            anyhow::bail!("Credenciales inválidas: contraseña incorrecta");
        }

        let usuario = Usuario {
            id: row.id,
            tenant_id: row.tenant_id.clone(),
            nombre: row.nombre,
            email: row.email,
            password_hash: row.password_hash,
            rol: row.rol,
            rol_id: row.rol_id,
            must_change_password: row.must_change_password,
            activo: row.activo,
            created_at: row.created_at,
        };

        let tenant = self.get_tenant(&usuario.tenant_id).await?;
        let token = self.generate_jwt(&usuario)?;

        // Evento SesionIniciada
        sqlx::query(
            r#"INSERT INTO events (aggregate_type, aggregate_id, version, event_type, payload, metadata, tenant_id, prev_hash, hash)
               VALUES ('Usuario', $1, (SELECT COALESCE(MAX(version),0)+1 FROM events WHERE aggregate_id = $1), 'SesionIniciada', $2, $3, $4, '0', $5)"#
        )
        .bind(usuario.id)
        .bind(serde_json::json!({"email": usuario.email, "tenantId": usuario.tenant_id}))
        .bind(serde_json::json!({"ip": "unknown"}))
        .bind(&usuario.tenant_id)
        .bind(format!("hash_{}", Uuid::new_v4()))
        .execute(&self.pool)
        .await
        .ok(); // ignore event error for login

        let usuario_public = self.usuario_public(&usuario).await?;

        Ok(AuthResponse {
            token,
            usuario: usuario_public,
            tenant,
        })
    }

    /// Verifica email+contraseña de un ADMIN del tenant, sin emitir un JWT ni
    /// tocar la sesión de quien está pidiendo la verificación (usado para el
    /// popup de aprobación de descuentos en el POS: el cajero sigue logueado,
    /// solo se confirma que un ADMIN autorizó la operación puntual). No revela
    /// si falló por email, contraseña o rol incorrecto - todos lucen iguales.
    pub async fn verify_admin_credentials(&self, tenant_id: &str, email: &str, password: &str) -> anyhow::Result<Uuid> {
        let row = sqlx::query_as::<_, UsuarioRow>(
            "SELECT id, tenant_id, nombre, email, password_hash, rol, rol_id, must_change_password, activo, created_at
             FROM usuarios WHERE email = $1 AND tenant_id = $2 AND activo = true",
        )
        .bind(email.to_lowercase())
        .bind(tenant_id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| anyhow::anyhow!("Credenciales de administrador inválidas"))?;

        if row.rol != "ADMIN" || !Self::verify_password(&row.password_hash, password)? {
            anyhow::bail!("Credenciales de administrador inválidas");
        }
        Ok(row.id)
    }

    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Primer paso de "olvidé mi contraseña". `Ok(None)` significa "no existe
    /// tal usuario (o está inactivo)" - el caller (http_forgot_password) debe
    /// responder exactamente igual que en el caso `Some` para no revelar qué
    /// correos están registrados.
    pub async fn iniciar_reset_password(&self, rnc: Option<&str>, email: &str) -> anyhow::Result<Option<(Uuid, String, String)>> {
        let usuario_row = if let Some(rnc) = rnc {
            sqlx::query_as::<_, UsuarioRow>(
                "SELECT id, tenant_id, nombre, email, password_hash, rol, rol_id, must_change_password, activo, created_at
                 FROM usuarios WHERE email = $1 AND tenant_id = $2 AND activo = true",
            )
            .bind(email.to_lowercase())
            .bind(rnc.replace("-", ""))
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, UsuarioRow>(
                "SELECT id, tenant_id, nombre, email, password_hash, rol, rol_id, must_change_password, activo, created_at
                 FROM usuarios WHERE email = $1 AND activo = true LIMIT 1",
            )
            .bind(email.to_lowercase())
            .fetch_optional(&self.pool)
            .await?
        };

        let Some(row) = usuario_row else { return Ok(None) };

        let mut token_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut token_bytes);
        let raw_token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(token_bytes);
        let token_hash = Self::hash_token(&raw_token);
        let expires_at = Utc::now() + Duration::minutes(30);

        sqlx::query("INSERT INTO password_reset_tokens (usuario_id, token_hash, expires_at) VALUES ($1, $2, $3)")
            .bind(row.id)
            .bind(&token_hash)
            .bind(expires_at)
            .execute(&self.pool)
            .await?;

        Ok(Some((row.id, row.nombre, raw_token)))
    }

    /// Segundo paso: consume el token (de un solo uso) y fija la nueva
    /// contraseña. Un solo mensaje de error para token incorrecto, vencido o
    /// ya usado - que luzcan idénticos desde afuera es intencional.
    pub async fn completar_reset_password(&self, token: &str, new_password: &str) -> anyhow::Result<()> {
        let token_hash = Self::hash_token(token);
        let mut tx = self.pool.begin().await?;

        let row: Option<(Uuid, Uuid)> = sqlx::query_as(
            "SELECT id, usuario_id FROM password_reset_tokens
             WHERE token_hash = $1 AND used_at IS NULL AND expires_at > NOW()
             FOR UPDATE",
        )
        .bind(&token_hash)
        .fetch_optional(&mut *tx)
        .await?;

        let (token_id, usuario_id) = row.ok_or_else(|| anyhow::anyhow!("Enlace inválido o expirado"))?;

        let new_hash = Self::hash_password(new_password)?;
        sqlx::query("UPDATE usuarios SET password_hash = $1 WHERE id = $2")
            .bind(&new_hash)
            .bind(usuario_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("UPDATE password_reset_tokens SET used_at = NOW() WHERE id = $1")
            .bind(token_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;
        Ok(())
    }

    pub async fn get_tenant(&self, rnc: &str) -> anyhow::Result<Tenant> {
        let row = sqlx::query_as::<_, TenantRow>("SELECT rnc, razon_social, nombre_comercial, direccion, telefono, correo, logo_url, ambiente_dgii, factura_electronica_activa, activo, created_at FROM tenants WHERE rnc = $1")
            .bind(rnc)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Tenant no encontrado: {}", rnc))?;

        Ok(Tenant {
            rnc: row.rnc,
            razon_social: row.razon_social,
            nombre_comercial: row.nombre_comercial,
            direccion: row.direccion,
            telefono: row.telefono,
            correo: row.correo,
            logo_url: row.logo_url,
            ambiente_dgii: row.ambiente_dgii,
            factura_electronica_activa: row.factura_electronica_activa,
            activo: row.activo,
            created_at: row.created_at,
        })
    }

    pub async fn get_usuario_by_id(&self, id: Uuid) -> anyhow::Result<Usuario> {
        let row = sqlx::query_as::<_, UsuarioRow>("SELECT id, tenant_id, nombre, email, password_hash, rol, rol_id, must_change_password, activo, created_at FROM usuarios WHERE id = $1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Usuario no encontrado"))?;

        Ok(Usuario {
            id: row.id,
            tenant_id: row.tenant_id,
            nombre: row.nombre,
            email: row.email,
            password_hash: row.password_hash,
            rol: row.rol,
            rol_id: row.rol_id,
            must_change_password: row.must_change_password,
            activo: row.activo,
            created_at: row.created_at,
        })
    }

    pub async fn list_usuarios(&self, tenant_id: &str) -> anyhow::Result<Vec<UsuarioPublic>> {
        let rows = sqlx::query_as::<_, UsuarioRow>("SELECT id, tenant_id, nombre, email, password_hash, rol, rol_id, must_change_password, activo, created_at FROM usuarios WHERE tenant_id = $1 ORDER BY created_at DESC")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await?;

        let mut resultado = Vec::with_capacity(rows.len());
        for r in rows {
            let (es_admin, permisos) = self.permisos_para(r.id).await?;
            resultado.push(UsuarioPublic {
                id: r.id,
                tenant_id: r.tenant_id,
                nombre: r.nombre,
                email: r.email,
                rol: r.rol,
                rol_id: r.rol_id,
                es_admin,
                permisos,
                must_change_password: r.must_change_password,
                activo: r.activo,
            });
        }
        Ok(resultado)
    }

    /// Fija una contraseña nueva y limpia el flag - usado cuando el propio
    /// usuario cumple el `must_change_password` forzado por staff (ver
    /// `admin_reset_password`). Devuelve un JWT nuevo (el viejo seguía
    /// siendo válido pero apuntaba a un usuario todavía flagged) para que el
    /// frontend pueda seguir sin pedir un segundo login.
    pub async fn set_new_password(&self, usuario_id: Uuid, new_password: &str) -> anyhow::Result<AuthResponse> {
        let hash = Self::hash_password(new_password)?;
        sqlx::query("UPDATE usuarios SET password_hash = $1, must_change_password = false WHERE id = $2")
            .bind(&hash)
            .bind(usuario_id)
            .execute(&self.pool)
            .await?;

        let usuario = self.get_usuario_by_id(usuario_id).await?;
        let tenant = self.get_tenant(&usuario.tenant_id).await?;
        let token = self.generate_jwt(&usuario)?;
        let usuario_public = self.usuario_public(&usuario).await?;
        Ok(AuthResponse { token, usuario: usuario_public, tenant })
    }

    /// Reset de contraseña iniciado por STAFF (sin correo) - genera una
    /// temporal al azar, la guarda hasheada, marca `must_change_password`, y
    /// devuelve el texto plano UNA sola vez para que el staff la transmita
    /// por el canal que sea (teléfono, WhatsApp, en persona). Nunca se
    /// vuelve a mostrar ni se guarda en texto plano en ningún lado.
    pub async fn admin_reset_password(&self, tenant_id: &str, usuario_id: Uuid) -> anyhow::Result<String> {
        const ALFABETO: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnpqrstuvwxyz23456789";
        // Bloque explícito: `ThreadRng` no es `Send`, así que debe quedar
        // fuera de alcance ANTES del próximo `.await` (la query de abajo) -
        // de lo contrario el future de este método deja de ser Send y axum
        // rechaza el handler entero con un error de trait bastante opaco.
        let temporal: String = {
            let mut rng = rand::thread_rng();
            (0..10)
                .map(|_| ALFABETO[(rng.next_u32() as usize) % ALFABETO.len()] as char)
                .collect()
        };

        let hash = Self::hash_password(&temporal)?;
        let actualizado = sqlx::query(
            "UPDATE usuarios SET password_hash = $1, must_change_password = true WHERE id = $2 AND tenant_id = $3",
        )
        .bind(&hash)
        .bind(usuario_id)
        .bind(tenant_id)
        .execute(&self.pool)
        .await?;
        if actualizado.rows_affected() == 0 {
            anyhow::bail!("Usuario no encontrado en este negocio");
        }
        Ok(temporal)
    }
}

// Row structs for sqlx

#[derive(sqlx::FromRow)]
struct TenantRow {
    rnc: String,
    razon_social: String,
    nombre_comercial: Option<String>,
    direccion: String,
    telefono: Option<String>,
    correo: Option<String>,
    logo_url: Option<String>,
    ambiente_dgii: String,
    factura_electronica_activa: bool,
    activo: bool,
    created_at: chrono::DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct UsuarioRow {
    id: Uuid,
    tenant_id: String,
    nombre: String,
    email: String,
    password_hash: String,
    rol: String,
    rol_id: Option<Uuid>,
    must_change_password: bool,
    activo: bool,
    created_at: chrono::DateTime<Utc>,
}

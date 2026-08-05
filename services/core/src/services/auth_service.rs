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
    pub rol: String, // ADMIN, CAJERO, ALMACEN, CONTADOR
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
    pub activo: bool,
}

impl From<Usuario> for UsuarioPublic {
    fn from(u: Usuario) -> Self {
        Self {
            id: u.id,
            tenant_id: u.tenant_id,
            nombre: u.nombre,
            email: u.email,
            rol: u.rol,
            activo: u.activo,
        }
    }
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

        // 2. Crear usuario admin
        sqlx::query(
            r#"INSERT INTO usuarios (id, tenant_id, nombre, email, password_hash, rol, activo, created_at)
               VALUES ($1, $2, $3, $4, $5, 'ADMIN', true, $6)"#
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

        // 4. Crear cuentas TigerBeetle base para tenant (mock, real TB crea cuentas)
        // En event_store real, aquí llamaríamos ledger::create_base_accounts(tenant_id)

        tx.commit().await?;

        // Fetch created records
        let tenant = self.get_tenant(&rnc_clean).await?;
        let usuario = self.get_usuario_by_id(user_id).await?;
        let token = self.generate_jwt(&usuario)?;

        Ok(AuthResponse {
            token,
            usuario: usuario.into(),
            tenant,
        })
    }

    pub async fn login(&self, req: LoginRequest) -> anyhow::Result<AuthResponse> {
        // Buscar usuario por email (y opcional RNC)
        let usuario_row = if let Some(rnc) = &req.rnc {
            sqlx::query_as::<_, UsuarioRow>(
                "SELECT id, tenant_id, nombre, email, password_hash, rol, activo, created_at FROM usuarios WHERE email = $1 AND tenant_id = $2 AND activo = true"
            )
            .bind(req.email.to_lowercase())
            .bind(rnc.replace("-", ""))
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, UsuarioRow>(
                "SELECT id, tenant_id, nombre, email, password_hash, rol, activo, created_at FROM usuarios WHERE email = $1 AND activo = true LIMIT 1"
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

        Ok(AuthResponse {
            token,
            usuario: usuario.into(),
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
            "SELECT id, tenant_id, nombre, email, password_hash, rol, activo, created_at
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
                "SELECT id, tenant_id, nombre, email, password_hash, rol, activo, created_at
                 FROM usuarios WHERE email = $1 AND tenant_id = $2 AND activo = true",
            )
            .bind(email.to_lowercase())
            .bind(rnc.replace("-", ""))
            .fetch_optional(&self.pool)
            .await?
        } else {
            sqlx::query_as::<_, UsuarioRow>(
                "SELECT id, tenant_id, nombre, email, password_hash, rol, activo, created_at
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
        let row = sqlx::query_as::<_, UsuarioRow>("SELECT id, tenant_id, nombre, email, password_hash, rol, activo, created_at FROM usuarios WHERE id = $1")
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
            activo: row.activo,
            created_at: row.created_at,
        })
    }

    pub async fn list_usuarios(&self, tenant_id: &str) -> anyhow::Result<Vec<UsuarioPublic>> {
        let rows = sqlx::query_as::<_, UsuarioRow>("SELECT id, tenant_id, nombre, email, password_hash, rol, activo, created_at FROM usuarios WHERE tenant_id = $1 ORDER BY created_at DESC")
            .bind(tenant_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| UsuarioPublic {
            id: r.id,
            tenant_id: r.tenant_id,
            nombre: r.nombre,
            email: r.email,
            rol: r.rol,
            activo: r.activo,
        }).collect())
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
    activo: bool,
    created_at: chrono::DateTime<Utc>,
}

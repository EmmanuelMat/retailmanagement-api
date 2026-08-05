//! Envío de correo vía Resend (https://resend.com, plan gratis 3000/mes, sin
//! tarjeta) - API REST plana, reutiliza `reqwest` que ya es dependencia del
//! workspace para el cliente HTTP de DGII.

use anyhow::Result;
use serde_json::json;

pub struct EmailService {
    api_key: Option<String>,
    from: String,
}

impl EmailService {
    pub fn new(api_key: Option<String>, from: String) -> Self {
        if api_key.is_none() {
            tracing::warn!("RESEND_API_KEY no configurado - los correos de restablecer contraseña se omitirán (se registrará un aviso por cada intento)");
        }
        Self { api_key, from }
    }

    /// Si no hay API key configurada, se omite el envío silenciosamente (solo
    /// un warning en logs) en vez de fallar - el correo es una funcionalidad
    /// que se degrada con gracia, a diferencia de los secretos de seguridad
    /// (JWT/licencia/certificados) que sí impiden arrancar el servidor.
    pub async fn send_password_reset(&self, to: &str, nombre: &str, reset_url: &str) -> Result<()> {
        let Some(api_key) = &self.api_key else {
            // Sin RESEND_API_KEY no hay forma de que el enlace le llegue al
            // usuario por otro medio - se imprime aquí para poder probar el
            // flujo completo en desarrollo sin depender de una cuenta Resend real.
            tracing::warn!("RESEND_API_KEY no configurado - enlace de reset para {} (no se envió por correo): {}", to, reset_url);
            return Ok(());
        };

        let html = format!(
            r#"<p>Hola {nombre},</p>
<p>Recibimos una solicitud para restablecer tu contraseña de Colmado POS. Este enlace vence en 30 minutos:</p>
<p><a href="{reset_url}">{reset_url}</a></p>
<p>Si no fuiste tú quien lo solicitó, puedes ignorar este correo - tu contraseña no cambiará.</p>"#,
        );

        let client = reqwest::Client::new();
        let res = client
            .post("https://api.resend.com/emails")
            .bearer_auth(api_key)
            .json(&json!({
                "from": self.from,
                "to": to,
                "subject": "Restablecer tu contraseña - Colmado POS",
                "html": html,
            }))
            .send()
            .await?;

        if !res.status().is_success() {
            let status = res.status();
            let body = res.text().await.unwrap_or_default();
            anyhow::bail!("Resend respondió {}: {}", status, body);
        }
        Ok(())
    }
}

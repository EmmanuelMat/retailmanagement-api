//! DGII e-CF Real Client - Auth + Send + Poll
//! Implements flow per dgii-ecf library (victors1681) and DGII Informe Técnico v1.0

use anyhow::{Context, Result};
use base64::Engine as _;
use reqwest::multipart;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum DGIIEnvironment {
    TesteCF,
    CerteCF,
    ECF,
}

impl DGIIEnvironment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::TesteCF => "TesteCF",
            Self::CerteCF => "CerteCF",
            Self::ECF => "eCF",
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "prod" | "ecf" => Self::ECF,
            "cert" | "certecf" => Self::CerteCF,
            _ => Self::TesteCF,
        }
    }

    pub fn base_url_ecf(&self) -> String {
        format!("https://ecf.dgii.gov.do/{}", self.as_str())
    }

    pub fn base_url_cf(&self) -> String {
        format!("https://fc.dgii.gov.do/{}", self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthToken {
    pub token: String,
    #[serde(default)]
    pub expira: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InvoiceResponse {
    #[serde(rename = "trackId")]
    pub track_id: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub mensaje: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackingStatusResponse {
    #[serde(rename = "trackId")]
    pub track_id: String,
    pub codigo: i32,
    pub estado: String,
    #[serde(default)]
    pub rnc: Option<String>,
    #[serde(rename = "encf", default)]
    pub e_ncf: Option<String>,
    #[serde(rename = "secuenciaUtilizada", default)]
    pub secuencia_utilizada: Option<bool>,
    #[serde(rename = "fechaRecepcion", default)]
    pub fecha_recepcion: Option<String>,
    #[serde(default)]
    pub mensajes: Option<Vec<Mensaje>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mensaje {
    #[serde(default)]
    pub valor: String,
    #[serde(default)]
    pub codigo: i32,
}

pub struct DGIIClient {
    pub environment: DGIIEnvironment,
    pub http_client: reqwest::Client,
    pub token: Option<String>,
}

impl DGIIClient {
    pub fn new(environment: DGIIEnvironment) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(false)
            .build()
            .expect("Failed to build http client");
        Self { environment, http_client, token: None }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
        self
    }

    pub async fn get_seed(&self) -> Result<String> {
        let url = format!("{}/Autenticacion/api/Autenticacion/Semilla", self.environment.base_url_ecf());
        tracing::info!("DGII GET seed: {}", url);
        let resp = self.http_client.get(&url).send().await.context("Failed to GET seed")?;
        let status = resp.status();
        let text = resp.text().await.context("Failed to read seed body")?;
        if !status.is_success() {
            anyhow::bail!("GET seed failed {}: {}", status, text);
        }
        Ok(text)
    }

    pub async fn authenticate_with_seed(&self, signed_seed_xml: &str) -> Result<AuthToken> {
        let url = format!("{}/autenticacion/api/Autenticacion/ValidarSemilla", self.environment.base_url_ecf());
        tracing::info!("DGII POST validate seed: {}", url);
        let part = multipart::Part::text(signed_seed_xml.to_string())
            .file_name("seed.xml")
            .mime_str("application/xml")?;
        let form = multipart::Form::new().part("xml", part);
        let resp = self.http_client.post(&url).multipart(form).send().await.context("Failed to POST validate seed")?;
        let status = resp.status();
        let text = resp.text().await.context("Failed to read token body")?;
        if !status.is_success() {
            anyhow::bail!("Validate seed failed {}: {}", status, text);
        }
        let token_data: AuthToken = serde_json::from_str(&text).unwrap_or(AuthToken {
            token: text.trim_matches('"').to_string(),
            expira: None,
        });
        tracing::info!("DGII Auth token obtained, len={}", token_data.token.len());
        Ok(token_data)
    }

    pub async fn authenticate(&mut self, p12_der: &[u8], password: &str) -> Result<String> {
        use crate::services::ecfl_service::sign_xml_ecf;
        let seed_xml = self.get_seed().await?;
        let signed_seed = sign_xml_ecf(&seed_xml, p12_der, password).context("Failed to sign seed XML")?;
        let token_data = self.authenticate_with_seed(&signed_seed.signed_xml).await?;
        self.token = Some(token_data.token.clone());
        Ok(token_data.token)
    }

    pub async fn send_ecf(&self, signed_ecf_xml: &str, file_name: &str) -> Result<InvoiceResponse> {
        let token = self.token.as_ref().context("No token - call authenticate first")?;
        let url = format!("{}/recepcion/api/FacturasElectronicas", self.environment.base_url_ecf());
        tracing::info!("DGII POST send eCF: {} file={}", url, file_name);
        let part = multipart::Part::text(signed_ecf_xml.to_string())
            .file_name(file_name.to_string())
            .mime_str("application/xml")?;
        let form = multipart::Form::new().part("xml", part);
        let resp = self.http_client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form).send().await.context("Failed to POST eCF")?;
        let status = resp.status();
        let text = resp.text().await.context("Failed to read send eCF body")?;
        if !status.is_success() {
            anyhow::bail!("Send eCF failed {}: {}", status, text);
        }
        let invoice_resp: InvoiceResponse = if text.contains("trackId") || text.contains("track_id") {
            serde_json::from_str(&text).unwrap_or(InvoiceResponse {
                track_id: text.trim_matches('"').to_string(),
                error: None,
                mensaje: None,
            })
        } else {
            InvoiceResponse {
                track_id: text.trim_matches('"').to_string(),
                error: None,
                mensaje: None,
            }
        };
        tracing::info!("DGII eCF sent, trackId={}", invoice_resp.track_id);
        Ok(invoice_resp)
    }

    pub async fn send_rfce(&self, signed_rfce_xml: &str, file_name: &str) -> Result<InvoiceResponse> {
        let token = self.token.as_ref().context("No token")?;
        let url = format!("{}/recepcionfc/api/recepcion/ecf", self.environment.base_url_cf());
        tracing::info!("DGII POST send RFCE: {} file={}", url, file_name);
        let part = multipart::Part::text(signed_rfce_xml.to_string())
            .file_name(file_name.to_string())
            .mime_str("application/xml")?;
        let form = multipart::Form::new().part("xml", part);
        let resp = self.http_client.post(&url)
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form).send().await.context("Failed to POST RFCE")?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Send RFCE failed {}: {}", status, text);
        }
        let invoice_resp: InvoiceResponse = serde_json::from_str(&text).unwrap_or(InvoiceResponse {
            track_id: text.trim_matches('"').to_string(),
            error: None,
            mensaje: None,
        });
        Ok(invoice_resp)
    }

    pub async fn status_track_id(&self, track_id: &str) -> Result<TrackingStatusResponse> {
        let token = self.token.as_ref().context("No token")?;
        let url = format!("{}/consultaresultado/api/Consultas/Estado", self.environment.base_url_ecf());
        tracing::info!("DGII GET status trackId={} url={}", track_id, url);
        let resp = self.http_client.get(&url)
            .header("Authorization", format!("Bearer {}", token))
            .query(&[("trackId", track_id)])
            .send().await.context("Failed to GET track status")?;
        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Status trackId failed {}: {}", status, text);
        }
        let tracking: TrackingStatusResponse = serde_json::from_str(&text).context(format!("Parse tracking resp: {}", text))?;
        tracing::info!("TrackId {} estado={} codigo={}", track_id, tracking.estado, tracking.codigo);
        Ok(tracking)
    }

    pub async fn send_with_polling(&self, signed_ecf_xml: &str, file_name: &str) -> Result<TrackingStatusResponse> {
        let invoice_resp = self.send_ecf(signed_ecf_xml, file_name).await?;
        let track_id = invoice_resp.track_id;
        let mut delay = Duration::from_secs(1);
        let max_retries = 10;
        for attempt in 0..max_retries {
            tokio::time::sleep(delay).await;
            match self.status_track_id(&track_id).await {
                Ok(s) => {
                    match s.estado.as_str() {
                        "Aceptado" | "AceptadoCondicional" | "Rechazado" => return Ok(s),
                        _ => {
                            tracing::info!("TrackId {} still in process (attempt {}/{}) retry in {:?}", track_id, attempt+1, max_retries, delay);
                            delay = std::cmp::min(delay * 2, Duration::from_secs(8));
                        }
                    }
                },
                Err(e) => {
                    tracing::warn!("Poll attempt {} failed: {}", attempt+1, e);
                    delay = std::cmp::min(delay * 2, Duration::from_secs(8));
                }
            }
        }
        anyhow::bail!("Polling timeout for trackId {}", track_id)
    }

    pub fn generate_qr_url(rnc_emisor: &str, e_ncf: &str, rnc_comprador: &str, fecha_emision: &str, monto_total: &str, codigo_seguridad: &str) -> String {
        format!(
            "https://ecf.dgii.gov.do/eCF/ConsultaTimbre?RNCEmisor={}&eNCF={}&RNCComprador={}&FechaEmision={}&MontoTotal={}&CodigoSeguridad={}",
            urlencoding(rnc_emisor),
            urlencoding(e_ncf),
            urlencoding(rnc_comprador),
            urlencoding(fecha_emision),
            urlencoding(monto_total),
            urlencoding(codigo_seguridad)
        )
    }
}

fn urlencoding(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            _ => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_env_urls() {
        let env = DGIIEnvironment::TesteCF;
        assert!(env.base_url_ecf().contains("ecf.dgii.gov.do/TesteCF"));
        assert!(env.base_url_cf().contains("fc.dgii.gov.do/TesteCF"));
    }
}

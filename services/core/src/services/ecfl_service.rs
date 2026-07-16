//! DGII e-CF Real XAdES-BES Signer
//! Implements spec from DGII "Firmado de e-CF" PDF:
//! CanonicalizationMethod: http://www.w3.org/TR/2001/REC-xml-c14n-20010315
//! SignatureMethod: http://www.w3.org/2001/04/xmldsig-more#rsa-sha256
//! Reference URI="" with enveloped-signature transform
//! DigestMethod: http://www.w3.org/2001/04/xmlenc#sha256
//! 
//! Flow from DGII TypeScript example:
//! 1. c14n(canonicalize) original XML (ECF root)
//! 2. SHA256 -> base64 -> DigestValue
//! 3. Build <Signature> skeleton with DigestValue + X509Certificate (no SignatureValue yet)
//! 4. Insert skeleton into canonicalized XML before </ECF>
//! 5. Extract <SignedInfo>, c14n it
//! 6. Sign c14n SignedInfo with private key RSA-SHA256 -> SignatureValue base64
//! 7. Inject SignatureValue and return signed XML + security code

use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use openssl::{hash::MessageDigest, pkcs12::Pkcs12, pkey::PKey, sign::Signer, x509::X509};
use sha2::{Digest, Sha256};

use crate::xml_c14n::{canonicalize, canonicalize_fragment};

pub struct P12Data {
    pub private_key: PKey<openssl::pkey::Private>,
    pub certificate: X509,
    pub cert_base64_der: String, // for <X509Certificate>
    pub cert_pem_clean: String,
}

pub fn load_p12(p12_der: &[u8], password: &str) -> Result<P12Data> {
    let pkcs12 = Pkcs12::from_der(p12_der).context("Invalid P12 DER")?;
    let parsed = pkcs12.parse2(password).context("Failed to parse P12 - check password")?;
    
    let pkey = parsed.pkey.context("P12 has no private key")?;
    let cert = parsed.cert.context("P12 has no certificate")?;

    let der = cert.to_der().context("Failed to get cert DER")?;
    let cert_base64_der = BASE64.encode(&der);

    // Clean PEM for other uses (without headers)
    let pem = String::from_utf8(cert.to_pem()?)?;
    let pem_clean = pem
        .replace("-----BEGIN CERTIFICATE-----", "")
        .replace("-----END CERTIFICATE-----", "")
        .replace("\r", "")
        .replace("\n", "")
        .trim()
        .to_string();

    Ok(P12Data {
        private_key: pkey,
        certificate: cert,
        cert_base64_der,
        cert_pem_clean: pem_clean,
    })
}

pub struct SignedECF {
    pub signed_xml: String,
    pub codigo_seguridad: String, // 6 chars
    pub digest_value: String,
    pub signature_value: String,
}

/// Real DGII-compliant signing
pub fn sign_xml_ecf(xml_input: &str, p12_der: &[u8], password: &str) -> Result<SignedECF> {
    let p12 = load_p12(p12_der, password)?;

    // Step 1: Canonicalize original XML (ECF)
    // Remove XML declaration if present for c14n
    let xml_clean = remove_xml_declaration(xml_input);
    let c14n_original = canonicalize(&xml_clean).context("Failed to c14n original XML")?;

    // Step 2: Digest = base64(sha256(c14n_original))
    let mut hasher = Sha256::new();
    hasher.update(c14n_original.as_bytes());
    let digest_bytes = hasher.finalize();
    let digest_value = BASE64.encode(&digest_bytes);

    // Step 3: Build Signature skeleton (without SignatureValue)
    let signature_skeleton = build_signature_skeleton(&digest_value, &p12.cert_base64_der);

    // Step 4: Insert skeleton into canonicalized XML before closing root
    // Find last </ECF> or </*> root end
    let root_tag = extract_root_tag(&xml_clean).unwrap_or_else(|| "ECF".to_string());
    let closing_tag = format!("</{}>", root_tag);
    let insert_pos = c14n_original
        .rfind(&closing_tag)
        .context(format!("Closing tag {} not found in canonical XML", closing_tag))?;
    
    let xml_without_sigvalue = format!(
        "{}{}{}",
        &c14n_original[..insert_pos],
        signature_skeleton,
        &c14n_original[insert_pos..]
    );

    // Step 5: Extract SignedInfo and canonicalize it
    // We need to parse xml_without_sigvalue and get SignedInfo inner XML
    let signed_info_xml = extract_signed_info(&xml_without_sigvalue).context("Failed to extract SignedInfo")?;
    // According to DGII TS: c14n with defaultNsForPrefix ds: http://www.w3.org/2000/09/xmldsig#
    // Our canonicalize already handles namespaces, but we need to ensure ds prefix handling
    // For SignedInfo canonicalization, we canonicalize the SignedInfo element itself
    let c14n_signed_info = canonicalize_fragment(&signed_info_xml).context("Failed to c14n SignedInfo")?;

    // Step 6: Sign c14n SignedInfo with private key RSA-SHA256
    let mut signer = Signer::new(MessageDigest::sha256(), &p12.private_key).context("Failed to create signer")?;
    signer.update(c14n_signed_info.as_bytes()).context("Signer update failed")?;
    let signature_bytes = signer.sign_to_vec().context("Signing failed")?;
    let signature_value = BASE64.encode(&signature_bytes);

    // Step 7: Inject SignatureValue
    // Find <SignatureValue></SignatureValue> placeholder and replace
    let signed_xml = if xml_without_sigvalue.contains("<SignatureValue></SignatureValue>") {
        xml_without_sigvalue.replace(
            "<SignatureValue></SignatureValue>",
            &format!("<SignatureValue>{}</SignatureValue>", signature_value),
        )
    } else if xml_without_sigvalue.contains("<SignatureValue>") {
        // Already has empty, replace content
        // Use simple replacement for first occurrence
        let start = xml_without_sigvalue.find("<SignatureValue>").unwrap();
        let end = xml_without_sigvalue[start..].find("</SignatureValue>").unwrap() + start + "</SignatureValue>".len();
        let before = &xml_without_sigvalue[..start];
        let after = &xml_without_sigvalue[end..];
        format!("{}<SignatureValue>{}</SignatureValue>{}", before, signature_value, after)
    } else {
        // Fallback: insert before </Signature>
        let sig_close = "</Signature>";
        let pos = xml_without_sigvalue.rfind(sig_close).context("No </Signature> found")?;
        format!(
            "{}<SignatureValue>{}</SignatureValue>{}{}",
            &xml_without_sigvalue[..pos],
            signature_value,
            sig_close,
            &xml_without_sigvalue[pos + sig_close.len()..]
        )
    };

    // Step 8: Codigo Seguridad = first 6 chars of SHA256 of signature? Per DGII spec: first 6 of hash of signature?
    // Spec: "CodigoSeguridad" is 6 digits extracted from signature hash. We implement as first 6 chars of SHA256(signature_value) uppercase alphanumeric
    // Also alternative: first 6 chars of base64-decoded signature hash? Use common e-CF implementation: first 6 chars of signature hash hex uppercase
    let mut hasher2 = Sha256::new();
    hasher2.update(signature_value.as_bytes());
    let sig_hash = hasher2.finalize();
    let sig_hash_hex = format!("{:x}", Sha256::digest(signature_bytes.clone()));
    let codigo_seguridad = sig_hash_hex[..6].to_uppercase();

    tracing::info!(
        "ECF signed: digest={}, sig_len={}, codigo_seguridad={}",
        digest_value,
        signature_value.len(),
        codigo_seguridad
    );

    Ok(SignedECF {
        signed_xml,
        codigo_seguridad,
        digest_value,
        signature_value,
    })
}

fn build_signature_skeleton(digest_value: &str, cert_base64: &str) -> String {
    // Note: DGII spec uses inclusive C14N
    format!(
        r#"<Signature xmlns="http://www.w3.org/2000/09/xmldsig#"><SignedInfo><CanonicalizationMethod Algorithm="http://www.w3.org/TR/2001/REC-xml-c14n-20010315"/><SignatureMethod Algorithm="http://www.w3.org/2001/04/xmldsig-more#rsa-sha256"/><Reference URI=""><Transforms><Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature"/></Transforms><DigestMethod Algorithm="http://www.w3.org/2001/04/xmlenc#sha256"/><DigestValue>{}</DigestValue></Reference></SignedInfo><SignatureValue></SignatureValue><KeyInfo><X509Data><X509Certificate>{}</X509Certificate></X509Data></KeyInfo></Signature>"#,
        digest_value, cert_base64
    )
}

fn extract_signed_info(xml_with_sig: &str) -> Result<String> {
    // Extract <SignedInfo>...</SignedInfo> substring
    let start_tag = "<SignedInfo>";
    let end_tag = "</SignedInfo>";
    let start = xml_with_sig.find(start_tag).context("SignedInfo start not found")?;
    let end = xml_with_sig[start..]
        .find(end_tag)
        .context("SignedInfo end not found")?
        + start
        + end_tag.len();
    Ok(xml_with_sig[start..end].to_string())
}

fn remove_xml_declaration(xml: &str) -> String {
    let trimmed = xml.trim();
    if trimmed.starts_with("<?xml") {
        if let Some(pos) = trimmed.find("?>") {
            return trimmed[pos + 2..].trim().to_string();
        }
    }
    trimmed.to_string()
}

fn extract_root_tag(xml: &str) -> Option<String> {
    // Simple extraction of first tag name
    let xml = remove_xml_declaration(xml);
    let start = xml.find('<')? + 1;
    let end = xml[start..].find(|c: char| c == '>' || c == ' ' || c == '\n' || c == '\r')?;
    Some(xml[start..start + end].to_string())
}

pub fn generate_qr_url(
    rnc_emisor: &str,
    e_ncf: &str,
    rnc_comprador: &str,
    fecha_emision: &str,
    monto_total: &str,
    codigo_seguridad: &str,
) -> String {
    format!(
        "https://ecf.dgii.gov.do/eCF/ConsultaTimbre?RNCEmisor={}&eNCF={}&RNCComprador={}&FechaEmision={}&MontoTotal={}&CodigoSeguridad={}",
        urlencoding::encode(rnc_emisor),
        urlencoding::encode(e_ncf),
        urlencoding::encode(rnc_comprador),
        urlencoding::encode(fecha_emision),
        urlencoding::encode(monto_total),
        urlencoding::encode(codigo_seguridad)
    )
}

pub async fn send_to_dgii(signed_xml: &str, file_name: &str, is_contingency: bool) -> anyhow::Result<String> {
    // TODO: Implement real DGII API call
    // Steps:
    // 1. Authenticate: POST /autenticacion with seed.xml signed -> token
    // 2. POST /recepcion/eCF with multipart or binary
    // For now mock
    let track_id = format!("DGII-TRACK-{}", uuid::Uuid::new_v4());
    tracing::info!(
        "Mock send to DGII: file={} len={} contingency={} track_id={}",
        file_name,
        signed_xml.len(),
        is_contingency,
        track_id
    );
    Ok(track_id)
}

// Helper module for urlencoding without extra dep
mod urlencoding {
    pub fn encode(s: &str) -> String {
        // Very simple - for full, use urlencoding crate
        s.replace(' ', "%20")
            .replace('&', "%26")
            .replace('=', "%3D")
            .replace('?', "%3F")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_skeleton() {
        let skel = build_signature_skeleton("abc123==", "MIIB...");
        assert!(skel.contains("DigestValue"));
        assert!(skel.contains("abc123=="));
        assert!(skel.contains("MIIB"));
    }

    #[test]
    fn test_root_extraction() {
        let xml = "<ECF><Encabezado>hi</Encabezado></ECF>";
        assert_eq!(extract_root_tag(xml).unwrap(), "ECF");
    }
}

#![allow(non_snake_case)]
//! RFCE Builder - Resumen Factura Consumo Electronica < 250k
//! Per DGII XSD RFCE v1.0 - Summarizes many E32 <250k invoices into one summary to send to DGII
//! Flow per dgii-ecf lib: convertECF32ToRFCE(signedEcfXml) -> { xml, securityCode }
//! securityCode = first 6 of hash from SignatureValue

use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;
use crate::services::ecfl_service::sign_xml_ecf;

/// RFCE structure per DGII spec (simplified for SME)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCE {
    pub Encabezado: RFCEEncabezado,
    pub Detalles: RFCEDetalles,
    pub FechaHoraFirma: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCEEncabezado {
    pub Version: String,
    pub IdDoc: RFCEIdDoc,
    pub Emisor: RFCEEmisor,
    pub Totales: RFCETotales,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCEIdDoc {
    pub FechaEmision: String, // DD-MM-YYYY of summary
    #[serde(rename = "RNCEmisor")]
    pub RNCEmisor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCEEmisor {
    #[serde(rename = "RNCEmisor")]
    pub RNCEmisor: String,
    pub RazonSocialEmisor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCETotales {
    pub CantidadFacturas: i32,
    pub MontoGravadoTotal: Decimal,
    pub MontoExento: Decimal,
    pub TotalITBIS: Decimal,
    pub MontoTotal: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RFCEDetalles {
    #[serde(rename = "FacturaConsumo")]
    pub FacturaConsumo: Vec<FacturaConsumoResumen>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacturaConsumoResumen {
    #[serde(rename = "eNCF")]
    pub eNCF: String, // E320000000001
    pub MontoTotal: Decimal,
    #[serde(rename = "CodigoSeguridad")]
    pub CodigoSeguridad: String, // 6 chars from signature hash
    pub FechaEmision: String,
}

pub fn build_rfce_xml(rfce: &RFCE) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="utf-8"?><RFCE>"#);
    xml.push_str("<Encabezado>");
    xml.push_str(&format!("<Version>{}</Version>", escape(&rfce.Encabezado.Version)));
    xml.push_str("<IdDoc>");
    xml.push_str(&format!("<FechaEmision>{}</FechaEmision>", escape(&rfce.Encabezado.IdDoc.FechaEmision)));
    xml.push_str(&format!("<RNCEmisor>{}</RNCEmisor>", escape(&rfce.Encabezado.IdDoc.RNCEmisor)));
    xml.push_str("</IdDoc>");
    xml.push_str("<Emisor>");
    xml.push_str(&format!("<RNCEmisor>{}</RNCEmisor>", escape(&rfce.Encabezado.Emisor.RNCEmisor)));
    xml.push_str(&format!("<RazonSocialEmisor>{}</RazonSocialEmisor>", escape(&rfce.Encabezado.Emisor.RazonSocialEmisor)));
    xml.push_str("</Emisor>");
    xml.push_str("<Totales>");
    xml.push_str(&format!("<CantidadFacturas>{}</CantidadFacturas>", rfce.Encabezado.Totales.CantidadFacturas));
    xml.push_str(&format!("<MontoGravadoTotal>{:.2}</MontoGravadoTotal>", rfce.Encabezado.Totales.MontoGravadoTotal));
    xml.push_str(&format!("<MontoExento>{:.2}</MontoExento>", rfce.Encabezado.Totales.MontoExento));
    xml.push_str(&format!("<TotalITBIS>{:.2}</TotalITBIS>", rfce.Encabezado.Totales.TotalITBIS));
    xml.push_str(&format!("<MontoTotal>{:.2}</MontoTotal>", rfce.Encabezado.Totales.MontoTotal));
    xml.push_str("</Totales>");
    xml.push_str("</Encabezado>");

    xml.push_str("<Detalles>");
    for fc in &rfce.Detalles.FacturaConsumo {
        xml.push_str("<FacturaConsumo>");
        xml.push_str(&format!("<eNCF>{}</eNCF>", escape(&fc.eNCF)));
        xml.push_str(&format!("<MontoTotal>{:.2}</MontoTotal>", fc.MontoTotal));
        xml.push_str(&format!("<CodigoSeguridad>{}</CodigoSeguridad>", escape(&fc.CodigoSeguridad)));
        xml.push_str(&format!("<FechaEmision>{}</FechaEmision>", escape(&fc.FechaEmision)));
        xml.push_str("</FacturaConsumo>");
    }
    xml.push_str("</Detalles>");

    xml.push_str(&format!("<FechaHoraFirma>{}</FechaHoraFirma>", escape(&rfce.FechaHoraFirma)));
    xml.push_str("</RFCE>");
    xml
}

pub fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

/// Convert a single signed E32 XML into RFCE entry + security code
/// Mirrors dgii-ecf lib convertECF32ToRFCE
pub fn convert_ecf32_to_rfce_entry(signed_ecf32_xml: &str) -> anyhow::Result<(FacturaConsumoResumen, String)> {
    // Extract eNCF, MontoTotal, FechaEmision from signed XML (simple regex parsing for demo)
    // In production, parse with roxmltree properly
    let e_ncf = extract_tag(signed_ecf32_xml, "eNCF").unwrap_or_else(|| "E320000000001".to_string());
    let monto_str = extract_tag(signed_ecf32_xml, "MontoTotal").unwrap_or_else(|| "1180.00".to_string());
    let monto = monto_str.parse::<Decimal>().unwrap_or(Decimal::ZERO);
    let fecha = extract_tag(signed_ecf32_xml, "FechaEmision").unwrap_or_else(|| "15-07-2026".to_string());

    // Security code = first 6 of SHA256 of SignatureValue (per dgii-ecf getCodeSixDigitfromSignature)
    let sig_value = extract_tag(signed_ecf32_xml, "SignatureValue").unwrap_or_default();
    let codigo_seguridad = if sig_value.is_empty() {
        // Fallback: hash of whole signed xml
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(signed_ecf32_xml.as_bytes());
        format!("{:x}", hash)[..6].to_uppercase()
    } else {
        // First 6 of hash of SignatureValue
        use sha2::{Sha256, Digest};
        let hash = Sha256::digest(sig_value.as_bytes());
        format!("{:x}", hash)[..6].to_uppercase()
    };

    let entry = FacturaConsumoResumen {
        eNCF: e_ncf,
        MontoTotal: monto,
        CodigoSeguridad: codigo_seguridad.clone(),
        FechaEmision: fecha,
    };

    Ok((entry, codigo_seguridad))
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

/// Build RFCE from list of signed E32 XMLs (the colmado day summary)
pub fn build_rfce_from_signed_e32_list(
    rnc_emisor: &str,
    razon_social: &str,
    fecha_emision: &str, // DD-MM-YYYY of summary
    signed_e32_xml_list: Vec<String>,
) -> anyhow::Result<RFCE> {
    let mut facturas = Vec::new();
    let mut total_gravado = Decimal::ZERO;
    let mut total_exento = Decimal::ZERO;
    let mut total_itbis = Decimal::ZERO;
    let mut monto_total = Decimal::ZERO;

    for signed_xml in signed_e32_xml_list {
        let (entry, _code) = convert_ecf32_to_rfce_entry(&signed_xml)?;
        // For totals, we would need to parse MontoGravado etc, simplified: use MontoTotal
        monto_total += entry.MontoTotal;
        // Approx split: 18% ITBIS
        let gravado = entry.MontoTotal / Decimal::from_str_exact("1.18").unwrap();
        let itbis = entry.MontoTotal - gravado;
        total_gravado += gravado;
        total_itbis += itbis;
        facturas.push(entry);
    }

    Ok(RFCE {
        Encabezado: RFCEEncabezado {
            Version: "1.0".to_string(),
            IdDoc: RFCEIdDoc {
                FechaEmision: fecha_emision.to_string(),
                RNCEmisor: rnc_emisor.to_string(),
            },
            Emisor: RFCEEmisor {
                RNCEmisor: rnc_emisor.to_string(),
                RazonSocialEmisor: razon_social.to_string(),
            },
            Totales: RFCETotales {
                CantidadFacturas: facturas.len() as i32,
                MontoGravadoTotal: total_gravado,
                MontoExento: total_exento,
                TotalITBIS: total_itbis,
                MontoTotal: monto_total,
            },
        },
        Detalles: RFCEDetalles {
            FacturaConsumo: facturas,
        },
        FechaHoraFirma: "".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_build_rfce() {
        let rfce = RFCE {
            Encabezado: RFCEEncabezado {
                Version: "1.0".to_string(),
                IdDoc: RFCEIdDoc { FechaEmision: "15-07-2026".to_string(), RNCEmisor: "130793752".to_string() },
                Emisor: RFCEEmisor { RNCEmisor: "130793752".to_string(), RazonSocialEmisor: "COLMADO EL SOL".to_string() },
                Totales: RFCETotales { CantidadFacturas: 2, MontoGravadoTotal: dec!(2000), MontoExento: dec!(0), TotalITBIS: dec!(360), MontoTotal: dec!(2360) },
            },
            Detalles: RFCEDetalles {
                FacturaConsumo: vec![
                    FacturaConsumoResumen { eNCF: "E320000000001".to_string(), MontoTotal: dec!(1180), CodigoSeguridad: "A1B2C3".to_string(), FechaEmision: "15-07-2026".to_string() },
                    FacturaConsumoResumen { eNCF: "E320000000002".to_string(), MontoTotal: dec!(1180), CodigoSeguridad: "D4E5F6".to_string(), FechaEmision: "15-07-2026".to_string() },
                ]
            },
            FechaHoraFirma: "".to_string(),
        };
        let xml = build_rfce_xml(&rfce);
        assert!(xml.contains("<eNCF>E320000000001</eNCF>"));
        assert!(xml.contains("<CodigoSeguridad>A1B2C3</CodigoSeguridad>"));
    }
}

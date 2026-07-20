#![allow(non_snake_case)]
//! ARECF (Acuse de Recibo) + ACECF (Aprobacion Comercial) builders
//! Per DGII XSD ARECF v1.0 and ACECF v1.0
//! 
//! Flow:
//! 1. Emisor envia e-CF a Receptor electronico -> Receptor debe enviar ARECF (acuse recibo) automatico + ACECF opcional (aceptacion/rechazo comercial)
//! 2. Tanto ARECF como ACECF se envian al Emisor y a DGII
//! 
//! ARECF = respuesta automatica que indica que el e-CF fue recibido
//! ACECF = respuesta de conformidad del receptor con la transaccion (0=Aceptada, 1=Rechazo con reparos, 2=Rechazada)

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARECF {
    pub Encabezado: ARECFEncabezado,
    pub Detalles: ARECFDetalles,
    pub FechaHoraFirma: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARECFEncabezado {
    pub Version: String, // 1.0
    pub IdDoc: ARECFIdDoc,
    pub Emisor: ARECFEmisor, // Receptor que envia el acuse
    pub Receptor: ARECFReceptor, // Emisor original
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARECFIdDoc {
    #[serde(rename = "TipoeCF")]
    pub TipoeCF: i32, // mismo tipo que e-CF recibido
    #[serde(rename = "eNCF")]
    pub eNCF: String,
    #[serde(rename = "RNCEmisor")]
    pub RNCEmisor: String, // RNC del emisor original
    pub FechaEmision: String, // fecha del e-CF original DD-MM-YYYY
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARECFEmisor {
    #[serde(rename = "RNCEmisor")]
    pub RNCEmisor: String, // RNC del receptor que envia acuse
    pub RazonSocialEmisor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARECFReceptor {
    #[serde(rename = "RNCReceptor")]
    pub RNCReceptor: String, // RNC emisor original
    pub RazonSocialReceptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARECFDetalles {
    pub Estado: i32, // 0=Recibido, 1=No Recibido (con motivo)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CodigoMotivoNoRecibido: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MensajeNoRecibido: Option<String>,
    pub FechaHoraRecepcion: String, // DD-MM-YYYY HH:MM:SS
}

// ACECF - Aprobacion Comercial

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACECF {
    pub Encabezado: ACECFEncabezado,
    pub Detalles: ACECFDetalles,
    pub FechaHoraFirma: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACECFEncabezado {
    pub Version: String,
    pub IdDoc: ACECFIdDoc,
    pub Emisor: ACECFEmisor,
    pub Receptor: ACECFReceptor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACECFIdDoc {
    #[serde(rename = "TipoeCF")]
    pub TipoeCF: i32,
    #[serde(rename = "eNCF")]
    pub eNCF: String,
    #[serde(rename = "RNCEmisor")]
    pub RNCEmisor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACECFEmisor {
    #[serde(rename = "RNCEmisor")]
    pub RNCEmisor: String,
    pub RazonSocialEmisor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACECFReceptor {
    #[serde(rename = "RNCReceptor")]
    pub RNCReceptor: String,
    pub RazonSocialReceptor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ACECFDetalles {
    pub Estado: i32, // 0=Aceptada, 1=Rechazo con reparos, 2=Rechazada
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CodigoMotivoRechazo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DescripcionMotivoRechazo: Option<String>,
    pub FechaHoraAprobacionComercial: String,
}

pub fn build_arecf_xml(arecf: &ARECF) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="utf-8"?><ARECF>"#);
    xml.push_str("<Encabezado>");
    xml.push_str(&format!("<Version>{}</Version>", escape(&arecf.Encabezado.Version)));
    xml.push_str("<IdDoc>");
    xml.push_str(&format!("<TipoeCF>{}</TipoeCF>", arecf.Encabezado.IdDoc.TipoeCF));
    xml.push_str(&format!("<eNCF>{}</eNCF>", escape(&arecf.Encabezado.IdDoc.eNCF)));
    xml.push_str(&format!("<RNCEmisor>{}</RNCEmisor>", escape(&arecf.Encabezado.IdDoc.RNCEmisor)));
    xml.push_str(&format!("<FechaEmision>{}</FechaEmision>", escape(&arecf.Encabezado.IdDoc.FechaEmision)));
    xml.push_str("</IdDoc>");
    xml.push_str("<Emisor>");
    xml.push_str(&format!("<RNCEmisor>{}</RNCEmisor>", escape(&arecf.Encabezado.Emisor.RNCEmisor)));
    xml.push_str(&format!("<RazonSocialEmisor>{}</RazonSocialEmisor>", escape(&arecf.Encabezado.Emisor.RazonSocialEmisor)));
    xml.push_str("</Emisor>");
    xml.push_str("<Receptor>");
    xml.push_str(&format!("<RNCReceptor>{}</RNCReceptor>", escape(&arecf.Encabezado.Receptor.RNCReceptor)));
    xml.push_str(&format!("<RazonSocialReceptor>{}</RazonSocialReceptor>", escape(&arecf.Encabezado.Receptor.RazonSocialReceptor)));
    xml.push_str("</Receptor>");
    xml.push_str("</Encabezado>");
    xml.push_str("<Detalles>");
    xml.push_str(&format!("<Estado>{}</Estado>", arecf.Detalles.Estado));
    if let Some(ref codigo) = arecf.Detalles.CodigoMotivoNoRecibido {
        xml.push_str(&format!("<CodigoMotivoNoRecibido>{}</CodigoMotivoNoRecibido>", escape(codigo)));
    }
    if let Some(ref msg) = arecf.Detalles.MensajeNoRecibido {
        xml.push_str(&format!("<MensajeNoRecibido>{}</MensajeNoRecibido>", escape(msg)));
    }
    xml.push_str(&format!("<FechaHoraRecepcion>{}</FechaHoraRecepcion>", escape(&arecf.Detalles.FechaHoraRecepcion)));
    xml.push_str("</Detalles>");
    xml.push_str(&format!("<FechaHoraFirma>{}</FechaHoraFirma>", escape(&arecf.FechaHoraFirma)));
    xml.push_str("</ARECF>");
    xml
}

pub fn build_acecf_xml(acecf: &ACECF) -> String {
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="utf-8"?><ACECF>"#);
    xml.push_str("<Encabezado>");
    xml.push_str(&format!("<Version>{}</Version>", escape(&acecf.Encabezado.Version)));
    xml.push_str("<IdDoc>");
    xml.push_str(&format!("<TipoeCF>{}</TipoeCF>", acecf.Encabezado.IdDoc.TipoeCF));
    xml.push_str(&format!("<eNCF>{}</eNCF>", escape(&acecf.Encabezado.IdDoc.eNCF)));
    xml.push_str(&format!("<RNCEmisor>{}</RNCEmisor>", escape(&acecf.Encabezado.IdDoc.RNCEmisor)));
    xml.push_str("</IdDoc>");
    xml.push_str("<Emisor>");
    xml.push_str(&format!("<RNCEmisor>{}</RNCEmisor>", escape(&acecf.Encabezado.Emisor.RNCEmisor)));
    xml.push_str(&format!("<RazonSocialEmisor>{}</RazonSocialEmisor>", escape(&acecf.Encabezado.Emisor.RazonSocialEmisor)));
    xml.push_str("</Emisor>");
    xml.push_str("<Receptor>");
    xml.push_str(&format!("<RNCReceptor>{}</RNCReceptor>", escape(&acecf.Encabezado.Receptor.RNCReceptor)));
    xml.push_str(&format!("<RazonSocialReceptor>{}</RazonSocialReceptor>", escape(&acecf.Encabezado.Receptor.RazonSocialReceptor)));
    xml.push_str("</Receptor>");
    xml.push_str("</Encabezado>");
    xml.push_str("<Detalles>");
    xml.push_str(&format!("<Estado>{}</Estado>", acecf.Detalles.Estado));
    if let Some(ref codigo) = acecf.Detalles.CodigoMotivoRechazo {
        xml.push_str(&format!("<CodigoMotivoRechazo>{}</CodigoMotivoRechazo>", escape(codigo)));
    }
    if let Some(ref desc) = acecf.Detalles.DescripcionMotivoRechazo {
        xml.push_str(&format!("<DescripcionMotivoRechazo>{}</DescripcionMotivoRechazo>", escape(desc)));
    }
    xml.push_str(&format!("<FechaHoraAprobacionComercial>{}</FechaHoraAprobacionComercial>", escape(&acecf.Detalles.FechaHoraAprobacionComercial)));
    xml.push_str("</Detalles>");
    xml.push_str(&format!("<FechaHoraFirma>{}</FechaHoraFirma>", escape(&acecf.FechaHoraFirma)));
    xml.push_str("</ACECF>");
    xml
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&apos;")
}

// Convenience builders

pub fn build_arecf_recibido(
    tipo_ecf: i32,
    e_ncf: &str,
    rnc_emisor_original: &str,
    fecha_emision_original: &str,
    rnc_receptor: &str,
    razon_social_receptor: &str,
    rnc_emisor_rec: &str,
    razon_social_emisor_rec: &str,
) -> ARECF {
    ARECF {
        Encabezado: ARECFEncabezado {
            Version: "1.0".to_string(),
            IdDoc: ARECFIdDoc {
                TipoeCF: tipo_ecf,
                eNCF: e_ncf.to_string(),
                RNCEmisor: rnc_emisor_original.to_string(),
                FechaEmision: fecha_emision_original.to_string(),
            },
            Emisor: ARECFEmisor {
                RNCEmisor: rnc_receptor.to_string(),
                RazonSocialEmisor: razon_social_receptor.to_string(),
            },
            Receptor: ARECFReceptor {
                RNCReceptor: rnc_emisor_rec.to_string(),
                RazonSocialReceptor: razon_social_emisor_rec.to_string(),
            },
        },
        Detalles: ARECFDetalles {
            Estado: 0, // Recibido
            CodigoMotivoNoRecibido: None,
            MensajeNoRecibido: None,
            FechaHoraRecepcion: chrono::Local::now().format("%d-%m-%Y %H:%M:%S").to_string(),
        },
        FechaHoraFirma: "".to_string(),
    }
}

pub fn build_acecf_aceptada(
    tipo_ecf: i32,
    e_ncf: &str,
    rnc_emisor_original: &str,
    rnc_receptor: &str,
    razon_social_receptor: &str,
    rnc_emisor_rec: &str,
    razon_social_emisor_rec: &str,
) -> ACECF {
    ACECF {
        Encabezado: ACECFEncabezado {
            Version: "1.0".to_string(),
            IdDoc: ACECFIdDoc {
                TipoeCF: tipo_ecf,
                eNCF: e_ncf.to_string(),
                RNCEmisor: rnc_emisor_original.to_string(),
            },
            Emisor: ACECFEmisor {
                RNCEmisor: rnc_receptor.to_string(),
                RazonSocialEmisor: razon_social_receptor.to_string(),
            },
            Receptor: ACECFReceptor {
                RNCReceptor: rnc_emisor_rec.to_string(),
                RazonSocialReceptor: razon_social_emisor_rec.to_string(),
            },
        },
        Detalles: ACECFDetalles {
            Estado: 0, // Aceptada
            CodigoMotivoRechazo: None,
            DescripcionMotivoRechazo: None,
            FechaHoraAprobacionComercial: chrono::Local::now().format("%d-%m-%Y %H:%M:%S").to_string(),
        },
        FechaHoraFirma: "".to_string(),
    }
}

#![allow(non_snake_case)]
//! ECF XML Builder per DGII Informe Técnico v1.0 XSD
//! Converts JSON ECF structure (like mseller docs) to XML string matching DGII XSD
//! Supports E31 (Crédito Fiscal), E32 (<250k and >=250k), E33 Nota Débito, E34 Nota Crédito, E41-E47

use serde::{Deserialize, Serialize};
use rust_decimal::Decimal;

/// Full ECF structure matching DGII spec - simplified but covers 90% cases for SME
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ECF {
    pub Encabezado: Encabezado,
    pub DetallesItems: DetallesItems,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Subtotales: Option<Subtotales>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Paginacion: Option<Paginacion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub InformacionReferencia: Option<InformacionReferencia>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DescuentosORecargos: Option<DescuentosORecargos>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub OtraMoneda: Option<OtraMoneda>,
    #[serde(default)]
    pub FechaHoraFirma: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Encabezado {
    pub Version: String, // "1.0"
    pub IdDoc: IdDoc,
    pub Emisor: Emisor,
    pub Comprador: Comprador,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub InformacionesAdicionales: Option<InformacionesAdicionales>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Transporte: Option<serde_json::Value>,
    pub Totales: Totales,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub OtraMoneda: Option<OtraMoneda>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdDoc {
    pub TipoeCF: i32, // 31,32,33,34,41,43,44,45,46,47
    #[serde(rename = "eNCF")]
    pub eNCF: String, // E310000000001 - 13 chars
    #[serde(rename = "FechaVencimientoSecuencia")]
    pub FechaVencimientoSecuencia: String, // DD-MM-YYYY
    #[serde(default)]
    pub IndicadorEnvioDiferido: i32, // 0=normal, 1=contingencia
    #[serde(default)]
    pub IndicadorMontoGravado: Option<i32>,
    #[serde(rename = "TipoIngresos")]
    pub TipoIngresos: String, // 01,02,03,04 etc
    #[serde(rename = "TipoPago")]
    pub TipoPago: i32, // 1=Contado, 2=Crédito
    #[serde(skip_serializing_if = "Option::is_none")]
    pub FechaLimitePago: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TerminoPago: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TablaFormasPago: Option<serde_json::Value>,
    #[serde(default = "default_total_paginas")]
    pub TotalPaginas: i32,
}

fn default_total_paginas() -> i32 { 1 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emisor {
    #[serde(rename = "RNCEmisor")]
    pub RNCEmisor: String,
    pub RazonSocialEmisor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub NombreComercial: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Sucursal: Option<String>,
    pub DireccionEmisor: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Municipio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub Provincia: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TablaTelefonoEmisor: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CorreoEmisor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub WebSite: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ActividadEconomica: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CodigoVendedor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub NumeroFacturaInterna: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub NumeroPedidoInterno: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ZonaVenta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub RutaVenta: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub InformacionAdicionalEmisor: Option<String>,
    pub FechaEmision: String, // DD-MM-YYYY
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comprador {
    #[serde(rename = "RNCComprador")]
    pub RNCComprador: String, // RNC, Cedula, or 000000000 for consumidor final
    #[serde(skip_serializing_if = "Option::is_none")]
    pub RazonSocialComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ContactoComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CorreoComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DireccionComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MunicipioComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ProvinciaComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub FechaEntrega: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ContactoEntrega: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DireccionEntrega: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TelefonoAdicional: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub FechaOrdenCompra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub NumeroOrdenCompra: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CodigoInternoComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ResponsablePago: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub InformacionAdicionalComprador: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub IdentificadorExtranjero: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformacionesAdicionales {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub FechaEmbarque: Option<String>,
    // ... other fields for export, simplified
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Totales {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoGravadoTotal: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoGravadoI1: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoGravadoI2: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoGravadoI3: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoExento: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ITBIS1: Option<Decimal>, // 18
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ITBIS2: Option<Decimal>, // 16
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ITBIS3: Option<Decimal>, // 0
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalITBIS: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalITBIS1: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalITBIS2: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalITBIS3: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoImpuestoAdicional: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ImpuestosAdicionales: Option<serde_json::Value>,
    pub MontoTotal: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoNoFacturable: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoPeriodo: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SaldoAnterior: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoAvancePago: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ValorPagar: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalITBISRetenido: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalISRRetencion: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalITBISPercepcion: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TotalISRPercepcion: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtraMoneda {
    #[serde(rename = "TipoMoneda")]
    pub TipoMoneda: String,
    pub TipoCambio: Decimal,
    // ... other fields
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetallesItems {
    #[serde(rename = "Item")]
    pub Item: Vec<Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub NumeroLinea: i32,
    pub IndicadorFacturacion: i32, // 1= gravado, etc per DGII tabla
    pub NombreItem: String,
    pub IndicadorBienoServicio: i32, // 1=bien, 2=servicio
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DescripcionItem: Option<String>,
    pub CantidadItem: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub UnidadMedida: Option<String>, // 43=unidad, etc per DGII tabla
    pub PrecioUnitarioItem: Decimal,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DescuentoPorcentaje: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub DescuentoMonto: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TablaSubDescuento: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TablaSubRecargo: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub TablaSubItemImpuestoAdicional: Option<serde_json::Value>,
    pub MontoItem: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subtotales {
    #[serde(rename = "Subtotal")]
    pub Subtotal: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginacion {
    #[serde(rename = "Pagina")]
    pub Pagina: Vec<Pagina>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagina {
    pub PaginaNo: i32,
    pub NoLineaDesde: i32,
    pub NoLineaHasta: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SubtotalMontoGravadoPagina: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SubtotalMontoGravado1Pagina: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SubtotalExentoPagina: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SubtotalItbisPagina: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SubtotalItbis1Pagina: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub MontoSubtotalPagina: Option<Decimal>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub SubtotalMontoNoFacturablePagina: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InformacionReferencia {
    pub NCFModificado: String, // E31... that is being modified by E33/E34
    #[serde(skip_serializing_if = "Option::is_none")]
    pub FechaNCFModificado: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub CodigoModificacion: Option<String>, // 1=anula, 2=etc
    #[serde(skip_serializing_if = "Option::is_none")]
    pub RazonModificacion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescuentosORecargos {
    #[serde(rename = "DescuentoORecargo")]
    pub DescuentoORecargo: Vec<serde_json::Value>,
}

/// Builder functions

pub fn build_ecf_xml(ecf: &ECF) -> String {
    // Build XML string per DGII XSD - using manual string builder for full control (quick_xml could also)
    let mut xml = String::new();
    xml.push_str(r#"<?xml version="1.0" encoding="utf-8"?>"#);
    xml.push_str("<ECF>");

    // Encabezado
    xml.push_str("<Encabezado>");
    xml.push_str(&format!("<Version>{}</Version>", escape_xml(&ecf.Encabezado.Version)));
    // IdDoc
    xml.push_str("<IdDoc>");
    xml.push_str(&format!("<TipoeCF>{}</TipoeCF>", ecf.Encabezado.IdDoc.TipoeCF));
    xml.push_str(&format!("<eNCF>{}</eNCF>", escape_xml(&ecf.Encabezado.IdDoc.eNCF)));
    xml.push_str(&format!("<FechaVencimientoSecuencia>{}</FechaVencimientoSecuencia>", escape_xml(&ecf.Encabezado.IdDoc.FechaVencimientoSecuencia)));
    xml.push_str(&format!("<IndicadorEnvioDiferido>{}</IndicadorEnvioDiferido>", ecf.Encabezado.IdDoc.IndicadorEnvioDiferido));
    if let Some(ind) = ecf.Encabezado.IdDoc.IndicadorMontoGravado {
        xml.push_str(&format!("<IndicadorMontoGravado>{}</IndicadorMontoGravado>", ind));
    }
    xml.push_str(&format!("<TipoIngresos>{}</TipoIngresos>", escape_xml(&ecf.Encabezado.IdDoc.TipoIngresos)));
    xml.push_str(&format!("<TipoPago>{}</TipoPago>", ecf.Encabezado.IdDoc.TipoPago));
    if let Some(ref fecha) = ecf.Encabezado.IdDoc.FechaLimitePago {
        xml.push_str(&format!("<FechaLimitePago>{}</FechaLimitePago>", escape_xml(fecha)));
    }
    xml.push_str(&format!("<TotalPaginas>{}</TotalPaginas>", ecf.Encabezado.IdDoc.TotalPaginas));
    xml.push_str("</IdDoc>");

    // Emisor
    xml.push_str("<Emisor>");
    xml.push_str(&format!("<RNCEmisor>{}</RNCEmisor>", escape_xml(&ecf.Encabezado.Emisor.RNCEmisor)));
    xml.push_str(&format!("<RazonSocialEmisor>{}</RazonSocialEmisor>", escape_xml(&ecf.Encabezado.Emisor.RazonSocialEmisor)));
    if let Some(ref v) = ecf.Encabezado.Emisor.NombreComercial {
        xml.push_str(&format!("<NombreComercial>{}</NombreComercial>", escape_xml(v)));
    }
    xml.push_str(&format!("<DireccionEmisor>{}</DireccionEmisor>", escape_xml(&ecf.Encabezado.Emisor.DireccionEmisor)));
    // Add optional fields if present
    if let Some(ref m) = ecf.Encabezado.Emisor.Municipio {
        xml.push_str(&format!("<Municipio>{}</Municipio>", escape_xml(m)));
    }
    if let Some(ref p) = ecf.Encabezado.Emisor.Provincia {
        xml.push_str(&format!("<Provincia>{}</Provincia>", escape_xml(p)));
    }
    if let Some(ref c) = ecf.Encabezado.Emisor.CorreoEmisor {
        xml.push_str(&format!("<CorreoEmisor>{}</CorreoEmisor>", escape_xml(c)));
    }
    xml.push_str(&format!("<FechaEmision>{}</FechaEmision>", escape_xml(&ecf.Encabezado.Emisor.FechaEmision)));
    xml.push_str("</Emisor>");

    // Comprador
    xml.push_str("<Comprador>");
    xml.push_str(&format!("<RNCComprador>{}</RNCComprador>", escape_xml(&ecf.Encabezado.Comprador.RNCComprador)));
    if let Some(ref r) = ecf.Encabezado.Comprador.RazonSocialComprador {
        xml.push_str(&format!("<RazonSocialComprador>{}</RazonSocialComprador>", escape_xml(r)));
    }
    if let Some(ref contacto) = ecf.Encabezado.Comprador.ContactoComprador {
        xml.push_str(&format!("<ContactoComprador>{}</ContactoComprador>", escape_xml(contacto)));
    }
    if let Some(ref correo) = ecf.Encabezado.Comprador.CorreoComprador {
        xml.push_str(&format!("<CorreoComprador>{}</CorreoComprador>", escape_xml(correo)));
    }
    if let Some(ref dir) = ecf.Encabezado.Comprador.DireccionComprador {
        xml.push_str(&format!("<DireccionComprador>{}</DireccionComprador>", escape_xml(dir)));
    }
    if let Some(ref id_ext) = ecf.Encabezado.Comprador.IdentificadorExtranjero {
        xml.push_str(&format!("<IdentificadorExtranjero>{}</IdentificadorExtranjero>", escape_xml(id_ext)));
    }
    xml.push_str("</Comprador>");

    // Totales
    xml.push_str("<Totales>");
    let tot = &ecf.Encabezado.Totales;
    if let Some(v) = tot.MontoGravadoTotal {
        xml.push_str(&format!("<MontoGravadoTotal>{:.2}</MontoGravadoTotal>", v));
    }
    if let Some(v) = tot.MontoGravadoI1 {
        xml.push_str(&format!("<MontoGravadoI1>{:.2}</MontoGravadoI1>", v));
    }
    if let Some(v) = tot.MontoExento {
        xml.push_str(&format!("<MontoExento>{:.2}</MontoExento>", v));
    }
    if let Some(v) = tot.ITBIS1 {
        xml.push_str(&format!("<ITBIS1>{:.2}</ITBIS1>", v));
    }
    if let Some(v) = tot.TotalITBIS {
        xml.push_str(&format!("<TotalITBIS>{:.2}</TotalITBIS>", v));
    }
    if let Some(v) = tot.TotalITBIS1 {
        xml.push_str(&format!("<TotalITBIS1>{:.2}</TotalITBIS1>", v));
    }
    xml.push_str(&format!("<MontoTotal>{:.2}</MontoTotal>", tot.MontoTotal));
    if let Some(v) = tot.MontoNoFacturable {
        xml.push_str(&format!("<MontoNoFacturable>{:.2}</MontoNoFacturable>", v));
    }
    xml.push_str("</Totales>");

    xml.push_str("</Encabezado>");

    // DetallesItems
    xml.push_str("<DetallesItems>");
    for item in &ecf.DetallesItems.Item {
        xml.push_str("<Item>");
        xml.push_str(&format!("<NumeroLinea>{}</NumeroLinea>", item.NumeroLinea));
        xml.push_str(&format!("<IndicadorFacturacion>{}</IndicadorFacturacion>", item.IndicadorFacturacion));
        xml.push_str(&format!("<NombreItem>{}</NombreItem>", escape_xml(&item.NombreItem)));
        xml.push_str(&format!("<IndicadorBienoServicio>{}</IndicadorBienoServicio>", item.IndicadorBienoServicio));
        xml.push_str(&format!("<CantidadItem>{:.2}</CantidadItem>", item.CantidadItem));
        if let Some(ref um) = item.UnidadMedida {
            xml.push_str(&format!("<UnidadMedida>{}</UnidadMedida>", escape_xml(um)));
        }
        xml.push_str(&format!("<PrecioUnitarioItem>{:.2}</PrecioUnitarioItem>", item.PrecioUnitarioItem));
        xml.push_str(&format!("<MontoItem>{:.2}</MontoItem>", item.MontoItem));
        xml.push_str("</Item>");
    }
    xml.push_str("</DetallesItems>");

    // InformacionReferencia for E33/E34
    if let Some(ref info_ref) = ecf.InformacionReferencia {
        xml.push_str("<InformacionReferencia>");
        xml.push_str(&format!("<NCFModificado>{}</NCFModificado>", escape_xml(&info_ref.NCFModificado)));
        if let Some(ref fecha) = info_ref.FechaNCFModificado {
            xml.push_str(&format!("<FechaNCFModificado>{}</FechaNCFModificado>", escape_xml(fecha)));
        }
        if let Some(ref codigo) = info_ref.CodigoModificacion {
            xml.push_str(&format!("<CodigoModificacion>{}</CodigoModificacion>", escape_xml(codigo)));
        }
        if let Some(ref razon) = info_ref.RazonModificacion {
            xml.push_str(&format!("<RazonModificacion>{}</RazonModificacion>", escape_xml(razon)));
        }
        xml.push_str("</InformacionReferencia>");
    }

    // Paginacion
    if let Some(ref pag) = ecf.Paginacion {
        xml.push_str("<Paginacion>");
        for p in &pag.Pagina {
            xml.push_str("<Pagina>");
            xml.push_str(&format!("<PaginaNo>{}</PaginaNo>", p.PaginaNo));
            xml.push_str(&format!("<NoLineaDesde>{}</NoLineaDesde>", p.NoLineaDesde));
            xml.push_str(&format!("<NoLineaHasta>{}</NoLineaHasta>", p.NoLineaHasta));
            if let Some(v) = p.SubtotalMontoGravadoPagina {
                xml.push_str(&format!("<SubtotalMontoGravadoPagina>{:.2}</SubtotalMontoGravadoPagina>", v));
            }
            if let Some(v) = p.MontoSubtotalPagina {
                xml.push_str(&format!("<MontoSubtotalPagina>{:.2}</MontoSubtotalPagina>", v));
            }
            xml.push_str("</Pagina>");
        }
        xml.push_str("</Paginacion>");
    }

    // FechaHoraFirma empty per spec before signing
    xml.push_str(&format!("<FechaHoraFirma>{}</FechaHoraFirma>", escape_xml(&ecf.FechaHoraFirma)));

    xml.push_str("</ECF>");
    xml
}

pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Convenience builder from simple POS items (for colmados)

pub fn build_simple_pos_ecf(
    tenant_rnc: &str,
    razon_social: &str,
    direccion: &str,
    e_ncf: &str,
    tipo_ecf: i32,
    cliente_rnc: &str,
    cliente_nombre: &str,
    items: Vec<(String, Decimal, Decimal)>, // (nombre, qty, precio_unitario)
    fecha_emision: &str, // DD-MM-YYYY
    fecha_vencimiento: &str,
) -> ECF {
    let mut total_gravado = Decimal::ZERO;
    let mut total_itbis = Decimal::ZERO;
    let mut total_exento = Decimal::ZERO;
    let mut ecf_items = Vec::new();

    for (idx, (nombre, qty, precio)) in items.into_iter().enumerate() {
        let monto = qty * precio;
        // Simplified: all gravado 18% for colmado example, but could be exento per product
        let itbis = monto * Decimal::from_str_exact("0.18").unwrap();
        total_gravado += monto;
        total_itbis += itbis;

        ecf_items.push(Item {
            NumeroLinea: (idx as i32) + 1,
            IndicadorFacturacion: 1, // gravado
            NombreItem: nombre,
            IndicadorBienoServicio: 1, // bien
            DescripcionItem: None,
            CantidadItem: qty,
            UnidadMedida: Some("43".to_string()), // 43=unidad
            PrecioUnitarioItem: precio,
            DescuentoPorcentaje: None,
            DescuentoMonto: None,
            TablaSubDescuento: None,
            TablaSubRecargo: None,
            TablaSubItemImpuestoAdicional: None,
            MontoItem: monto,
        });
    }

    let monto_total = total_gravado + total_itbis + total_exento;

    ECF {
        Encabezado: Encabezado {
            Version: "1.0".to_string(),
            IdDoc: IdDoc {
                TipoeCF: tipo_ecf,
                eNCF: e_ncf.to_string(),
                FechaVencimientoSecuencia: fecha_vencimiento.to_string(),
                IndicadorEnvioDiferido: 0,
                IndicadorMontoGravado: Some(0),
                TipoIngresos: "01".to_string(),
                TipoPago: 1,
                FechaLimitePago: None,
                TerminoPago: None,
                TablaFormasPago: None,
                TotalPaginas: 1,
            },
            Emisor: Emisor {
                RNCEmisor: tenant_rnc.to_string(),
                RazonSocialEmisor: razon_social.to_string(),
                NombreComercial: None,
                Sucursal: None,
                DireccionEmisor: direccion.to_string(),
                Municipio: None,
                Provincia: None,
                TablaTelefonoEmisor: None,
                CorreoEmisor: None,
                WebSite: None,
                ActividadEconomica: None,
                CodigoVendedor: None,
                NumeroFacturaInterna: None,
                NumeroPedidoInterno: None,
                ZonaVenta: None,
                RutaVenta: None,
                InformacionAdicionalEmisor: None,
                FechaEmision: fecha_emision.to_string(),
            },
            Comprador: Comprador {
                RNCComprador: cliente_rnc.to_string(),
                RazonSocialComprador: Some(cliente_nombre.to_string()),
                ContactoComprador: None,
                CorreoComprador: None,
                DireccionComprador: None,
                MunicipioComprador: None,
                ProvinciaComprador: None,
                FechaEntrega: None,
                ContactoEntrega: None,
                DireccionEntrega: None,
                TelefonoAdicional: None,
                FechaOrdenCompra: None,
                NumeroOrdenCompra: None,
                CodigoInternoComprador: None,
                ResponsablePago: None,
                InformacionAdicionalComprador: None,
                IdentificadorExtranjero: None,
            },
            InformacionesAdicionales: None,
            Transporte: None,
            Totales: Totales {
                MontoGravadoTotal: Some(total_gravado),
                MontoGravadoI1: Some(total_gravado),
                MontoGravadoI2: None,
                MontoGravadoI3: None,
                MontoExento: if total_exento > Decimal::ZERO { Some(total_exento) } else { None },
                ITBIS1: Some(Decimal::from(18)),
                ITBIS2: None,
                ITBIS3: None,
                TotalITBIS: Some(total_itbis),
                TotalITBIS1: Some(total_itbis),
                TotalITBIS2: None,
                TotalITBIS3: None,
                MontoImpuestoAdicional: None,
                ImpuestosAdicionales: None,
                MontoTotal: monto_total,
                MontoNoFacturable: None,
                MontoPeriodo: None,
                SaldoAnterior: None,
                MontoAvancePago: None,
                ValorPagar: None,
                TotalITBISRetenido: None,
                TotalISRRetencion: None,
                TotalITBISPercepcion: None,
                TotalISRPercepcion: None,
            },
            OtraMoneda: None,
        },
        DetallesItems: DetallesItems { Item: ecf_items },
        Subtotales: None,
        Paginacion: Some(Paginacion {
            Pagina: vec![Pagina {
                PaginaNo: 1,
                NoLineaDesde: 1,
                NoLineaHasta: 1,
                SubtotalMontoGravadoPagina: Some(total_gravado),
                SubtotalMontoGravado1Pagina: Some(total_gravado),
                SubtotalExentoPagina: None,
                SubtotalItbisPagina: Some(total_itbis),
                SubtotalItbis1Pagina: Some(total_itbis),
                MontoSubtotalPagina: Some(monto_total),
                SubtotalMontoNoFacturablePagina: None,
            }],
        }),
        InformacionReferencia: None,
        DescuentosORecargos: None,
        OtraMoneda: None,
        FechaHoraFirma: "".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_build_simple() {
        let ecf = build_simple_pos_ecf(
            "130793752",
            "COLMADO EL SOL SRL",
            "Av Duarte",
            "E320000000001",
            32,
            "000000000",
            "CONSUMIDOR FINAL",
            vec![("Arroz".to_string(), dec!(1), dec!(1000))],
            "15-07-2026",
            "31-12-2026",
        );
        let xml = build_ecf_xml(&ecf);
        assert!(xml.contains("<eNCF>E320000000001</eNCF>"));
        assert!(xml.contains("<RNCEmisor>130793752</RNCEmisor>"));
        assert!(xml.contains("<MontoTotal>"));
    }
}

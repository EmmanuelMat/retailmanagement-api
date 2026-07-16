//! Sale Aggregate - Event-driven POS sale with DGII e-CF flow

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleState {
    pub id: Uuid,
    pub tenant_id: String,
    pub e_ncf: Option<String>, // E320000000001
    pub tipo_ecf: i32, // 31,32
    pub items: Vec<SaleItem>,
    pub total: Decimal,
    pub itbis_total: Decimal,
    pub status: SaleStatus,
    pub track_id: Option<String>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SaleStatus { Draft, Completed, DGIIPending, DGIIAccepted, DGIIRejected, Voided }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleItem {
    pub sku: String,
    pub name: String,
    pub qty: Decimal,
    pub unit_price: Decimal,
    pub itbis_type: String, // 18,16,EXENTO
    pub itbis_amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaleCommand {
    CreateSale { tipo_ecf: i32, client_rnc: Option<String> },
    AddItem { sku: String, qty: Decimal },
    CompleteSale,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaleEvent {
    SaleCreated { tipo_ecf: i32, client_rnc: Option<String> },
    ItemAdded { sku: String, qty: Decimal, price: Decimal },
    SaleCompleted { total: Decimal, itbis_total: Decimal, e_ncf: String },
    ETicketSigningRequested { json_payload: String },
    ETicketAccepted { track_id: String, qr_url: String, codigo_seguridad: String },
    ETicketRejected { reason: String },
    SaleVoided { reason: String },
}

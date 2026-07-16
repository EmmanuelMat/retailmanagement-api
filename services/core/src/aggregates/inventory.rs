//! Inventory Aggregate

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InventoryState {
    pub sku: String,
    pub qty_on_hand: Decimal,
    pub qty_reserved: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryCommand {
    ReceiveStock { qty: Decimal },
    ReserveStock { qty: Decimal, sale_id: Uuid },
    CommitStock { sale_id: Uuid },
    ReleaseStock { sale_id: Uuid },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InventoryEvent {
    StockReceived { qty: Decimal },
    StockReserved { qty: Decimal, sale_id: Uuid, tb_transfer_id: u128 },
    StockCommitted { sale_id: Uuid },
    StockReleased { sale_id: Uuid },
}

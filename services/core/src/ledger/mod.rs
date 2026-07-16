//! TigerBeetle Ledger - Double-entry accounting DB

// For now, mock client. Replace with real tigerbeetle client crate when cluster ready.

pub struct TigerBeetleClient {
    // client: tigerbeetle::Client,
    pub cluster_url: String,
}

impl TigerBeetleClient {
    pub fn new(cluster_url: String) -> Self {
        Self { cluster_url }
    }

    /// Create linked transfers atomically - e.g., sale + ITBIS + COGS
    pub async fn create_linked_transfers(&self, transfers: Vec<Transfer>) -> anyhow::Result<Vec<TransferResult>> {
        tracing::info!("TB create_linked_transfers: {} transfers", transfers.len());
        // TODO: call real TB
        // In MVP, log and return OK
        Ok(vec![])
    }

    /// Two-phase: reserve (pending) then post or void - perfect for payroll advances
    pub async fn reserve_advance(&self, tenant_id: &str, employee_id: &str, amount_cents: u128) -> anyhow::Result<u128> {
        let transfer_id = uuid::Uuid::new_v4().as_u128();
        tracing::info!("TB reserve_advance tenant={} employee={} amount={} id={}", tenant_id, employee_id, amount_cents, transfer_id);
        // Flags: pending
        Ok(transfer_id)
    }

    pub async fn post_pending(&self, transfer_id: u128) -> anyhow::Result<()> {
        tracing::info!("TB post_pending id={}", transfer_id);
        Ok(())
    }

    pub async fn void_pending(&self, transfer_id: u128) -> anyhow::Result<()> {
        tracing::info!("TB void_pending id={}", transfer_id);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Transfer {
    pub id: u128,
    pub debit_account: u128,
    pub credit_account: u128,
    pub amount: u128, // cents
    pub ledger: u32, // 700 = DOP
    pub code: u16, // 10=venta, 30=adelanto, 31=nomina
    pub flags: u16, // linked, pending etc
    pub user_data: u128,
}

pub struct TransferResult {
    pub transfer_id: u128,
    pub error: Option<String>,
}

pub fn tb_account_id(tenant_rnc: &str, account_name: &str) -> u128 {
    // Deterministic hash to u128 for same tenant always same account IDs
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    format!("{}:{}", tenant_rnc, account_name).hash(&mut hasher);
    hasher.finish() as u128 + 1 // avoid 0
}

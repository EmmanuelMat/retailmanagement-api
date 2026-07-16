pub mod fiscal;
pub mod payroll;

use std::sync::Arc;
use crate::ledger::TigerBeetleClient;

pub struct AppState {
    pub tb_client: Arc<TigerBeetleClient>,
    // pub pg_pool: Arc<PgPool>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            tb_client: Arc::new(TigerBeetleClient::new(
                std::env::var("TIGERBEETLE_CLUSTER").unwrap_or_else(|_| "3000".to_string()),
            )),
        }
    }
}

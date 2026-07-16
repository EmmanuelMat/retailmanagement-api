//! Payroll Service with Advance Engine

use crate::aggregates::employee::{EmployeeState, EmployeeCommand, handle_command};
use crate::ledger::TigerBeetleClient;
use rust_decimal::Decimal;
use uuid::Uuid;

pub struct PayrollService {
    pub tb_client: TigerBeetleClient,
    // pub pg_pool: PgPool
}

impl PayrollService {
    pub async fn request_advance(
        &self,
        employee_state: &EmployeeState,
        amount: Decimal,
        reason: String,
    ) -> Result<Uuid, String> {
        // 1. Business rule check (50% rule inside aggregate)
        let cmd = EmployeeCommand::RequestAdvance { amount, reason: reason.clone() };
        let events = handle_command(employee_state, cmd).map_err(|e| e)?;
        
        // 2. Reserve in TigerBeetle (pending transfer)
        let amount_cents = (amount * Decimal::from(100)).to_string().parse::<u128>().unwrap_or(0);
        let tb_id = self.tb_client.reserve_advance(&employee_state.tenant_id, &employee_state.id.to_string(), amount_cents).await.map_err(|e| e.to_string())?;
        
        // 3. Append events to EventStore (would be here)
        // let request_id = events[0].request_id...
        tracing::info!("Advance requested: employee={} amount={} tb_id={}", employee_state.id, amount, tb_id);
        
        Ok(Uuid::new_v4()) // request_id
    }

    pub async fn approve_advance(&self, transfer_id: u128) -> anyhow::Result<()> {
        self.tb_client.post_pending(transfer_id).await?;
        Ok(())
    }

    pub async fn run_payroll(&self, tenant_id: &str, period: &str, employees: Vec<EmployeeState>) -> anyhow::Result<String> {
        let payroll_id = Uuid::new_v4();
        tracing::info!("Running payroll tenant={} period={} employees={} payroll_id={}", tenant_id, period, employees.len(), payroll_id);
        
        // For each employee, create linked transfers
        for emp in employees {
            // ... create linked transfers for gross, tss, isr, advance deduction
            // See docs/05-PAYROLL-ADVANCE.md accounting
        }
        
        Ok(payroll_id.to_string())
    }
}

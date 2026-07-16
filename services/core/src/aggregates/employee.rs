//! Employee Aggregate - Bank-grade payroll advance logic
//! Event Sourcing: Command -> Events -> State rebuilt

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmployeeState {
    pub id: Uuid,
    pub tenant_id: String,
    pub name: String,
    pub salary: Decimal, // monthly
    pub accrued_net: Decimal, // earned but unpaid
    pub advance_balance: Decimal, // amount taken as advance
    pub max_advance_per_period: Decimal, // e.g., 10000
    pub version: i64,
}

impl EmployeeState {
    pub fn new(id: Uuid, tenant_id: String, name: String, salary: Decimal) -> Self {
        Self {
            id,
            tenant_id,
            name,
            salary,
            accrued_net: dec!(0),
            advance_balance: dec!(0),
            max_advance_per_period: dec!(10000),
            version: 0,
        }
    }

    pub fn available_for_advance(&self) -> Decimal {
        // 50% rule + min reserve 1000
        let available = self.accrued_net - self.advance_balance;
        let fifty_percent = self.accrued_net * dec!(0.50);
        let capped = if available < fifty_percent { available } else { fifty_percent };
        if capped < dec!(0) { dec!(0) } else { capped - dec!(1000).min(capped) }
    }

    pub fn apply(&mut self, event: EmployeeEvent) {
        match event {
            EmployeeEvent::EmployeeCreated { .. } => {},
            EmployeeEvent::HoursAccrued { amount } => {
                self.accrued_net += amount;
                self.version += 1;
            },
            EmployeeEvent::AdvanceApproved { amount } => {
                self.advance_balance += amount;
                self.version += 1;
            },
            EmployeeEvent::PayrollExecuted { advance_deduction, .. } => {
                // Payroll settles: accrued reset, advance_balance reduced by deduction
                self.accrued_net = dec!(0);
                self.advance_balance -= advance_deduction;
                if self.advance_balance < dec!(0) { self.advance_balance = dec!(0); }
                self.version += 1;
            }
            _ => { self.version += 1; }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmployeeCommand {
    ClockIn { hours: Decimal, rate_per_hour: Decimal },
    RequestAdvance { amount: Decimal, reason: String },
    ApproveAdvance { request_id: Uuid, amount: Decimal, approved_by: String },
    RunPayroll { period: String, gross: Decimal, tss: Decimal, isr: Decimal },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmployeeEvent {
    EmployeeCreated { name: String, salary: Decimal },
    HoursAccrued { amount: Decimal },
    AdvanceRequested { request_id: Uuid, amount: Decimal, reason: String },
    AdvanceApproved { request_id: Uuid, amount: Decimal, approved_by: String, tigerbeetle_transfer_id: u128 },
    AdvanceRejected { request_id: Uuid, reason: String },
    PayrollExecuted { period: String, gross: Decimal, tss: Decimal, isr: Decimal, advance_deduction: Decimal, net: Decimal },
}

pub fn handle_command(state: &EmployeeState, cmd: EmployeeCommand) -> Result<Vec<EmployeeEvent>, String> {
    match cmd {
        EmployeeCommand::ClockIn { hours, rate_per_hour } => {
            let gross = hours * rate_per_hour;
            // Estimate net: - TSS 5.91% and ISR 0 for simplicity
            let tss = gross * dec!(0.0591);
            let net = gross - tss;
            Ok(vec![EmployeeEvent::HoursAccrued { amount: net }])
        },
        EmployeeCommand::RequestAdvance { amount, reason } => {
            if amount <= dec!(0) { return Err("Amount must be >0".into()) }
            let available = state.available_for_advance();
            if amount > available {
                return Err(format!("Exceeds available for advance: available RD$ {}, requested RD$ {}", available, amount))
            }
            let request_id = Uuid::new_v4();
            Ok(vec![EmployeeEvent::AdvanceRequested { request_id, amount, reason }])
        },
        EmployeeCommand::ApproveAdvance { request_id, amount, approved_by } => {
            // In real world, check TigerBeetle pending transfer exists and post it
            let tb_id = Uuid::new_v4().as_u128(); // idempotent transfer id
            Ok(vec![EmployeeEvent::AdvanceApproved { request_id, amount, approved_by, tigerbeetle_transfer_id: tb_id }])
        },
        EmployeeCommand::RunPayroll { period, gross, tss, isr } => {
            let advance_deduction = state.advance_balance.min(gross - tss - isr);
            let net = gross - tss - isr - advance_deduction;
            Ok(vec![EmployeeEvent::PayrollExecuted { period, gross, tss, isr, advance_deduction, net }])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_50_percent_rule() {
        let mut state = EmployeeState::new(Uuid::new_v4(), "130793752".into(), "Maria".into(), dec!(20000));
        state.accrued_net = dec!(9200);
        // Available should be 50% = 4600 - 1000 reserve = 3600
        let available = state.available_for_advance();
        assert!(available > dec!(0));
        // Request 2000 OK
        let cmd = EmployeeCommand::RequestAdvance { amount: dec!(2000), reason: "Medicina".into() };
        assert!(handle_command(&state, cmd).is_ok());
        // Request 5000 should fail (>50%)
        let cmd2 = EmployeeCommand::RequestAdvance { amount: dec!(5000), reason: "X".into() };
        assert!(handle_command(&state, cmd2).is_err());
    }
}

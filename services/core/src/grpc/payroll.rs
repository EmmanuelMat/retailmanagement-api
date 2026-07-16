use tonic::{Request, Response, Status};
use rust_decimal::Decimal;
use std::str::FromStr;

pub mod proto {
    tonic::include_proto!("payroll_core");
}
use proto::payroll_service_server::PayrollService;
use proto::*;

#[derive(Debug, Default)]
pub struct PayrollServiceImpl;

#[tonic::async_trait]
impl PayrollService for PayrollServiceImpl {
    async fn clock_in(
        &self,
        request: Request<ClockInRequest>,
    ) -> Result<Response<ClockInResponse>, Status> {
        let req = request.into_inner();
        let hours = Decimal::from_str(&req.hours).unwrap_or(Decimal::from(0));
        let rate = Decimal::from_str(&req.rate_per_hour).unwrap_or(Decimal::from(0));
        let gross = hours * rate;
        let tss = gross * Decimal::from_str("0.0591").unwrap();
        let net = gross - tss;
        Ok(Response::new(ClockInResponse {
            accrued_net: net.to_string(),
        }))
    }

    async fn request_advance(
        &self,
        request: Request<AdvanceRequest>,
    ) -> Result<Response<AdvanceResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("Payroll RequestAdvance tenant={} emp={} amount_cents={}", req.tenant_id, req.employee_id, req.amount_cents);

        // Parse amount cents -> Decimal DOP
        let amount_cents = req.amount_cents.parse::<i64>().unwrap_or(0);
        let amount = Decimal::from(amount_cents) / Decimal::from(100);

        // Mock check 50% rule - in real, load EmployeeAggregate from EventStore
        if amount > Decimal::from(5000) {
            return Err(Status::failed_precondition(format!("Exceeds 50% rule or available, requested {}", amount)));
        }

        // Reserve in TigerBeetle (pending)
        let tb_transfer_id = uuid::Uuid::new_v4().as_u128() as u64; // mock lower 64 bits

        Ok(Response::new(AdvanceResponse {
            request_id: uuid::Uuid::new_v4().to_string(),
            status: "PENDING_APPROVAL".to_string(),
            tigerbeetle_transfer_id: tb_transfer_id,
        }))
    }

    async fn approve_advance(
        &self,
        request: Request<ApproveRequest>,
    ) -> Result<Response<ApproveResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("ApproveAdvance request_id={} by={}", req.request_id, req.approved_by);
        // Post pending TB transfer
        Ok(Response::new(ApproveResponse { ok: true }))
    }

    async fn run_payroll(
        &self,
        request: Request<PayrollRunRequest>,
    ) -> Result<Response<PayrollRunResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("RunPayroll tenant={} period={}", req.tenant_id, req.period);
        Ok(Response::new(PayrollRunResponse {
            payroll_id: uuid::Uuid::new_v4().to_string(),
            gross_total: "2000000".to_string(), // cents
            net_total: "1541000".to_string(),
        }))
    }

    async fn get_employee_balance(
        &self,
        request: Request<EmployeeBalanceRequest>,
    ) -> Result<Response<EmployeeBalanceResponse>, Status> {
        let req = request.into_inner();
        // Mock - in real load from read model projector
        Ok(Response::new(EmployeeBalanceResponse {
            employee_id: req.employee_id,
            accrued_net_cents: "920000".to_string(), // 9200.00
            advance_balance_cents: "200000".to_string(), // 2000 taken
            available_for_advance_cents: "260000".to_string(), // 50% - reserve
            max_allowed_cents: "460000".to_string(),
        }))
    }
}

use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc_exists = std::process::Command::new("protoc")
        .arg("--version")
        .output()
        .is_ok();

    if !protoc_exists {
        println!("cargo:warning=protoc not found, creating dummy proto generated files for HTTP-only build");
        let out_dir = std::env::var("OUT_DIR").unwrap();
        std::fs::create_dir_all(&out_dir).unwrap();
        
        // Dummy fiscal_core.rs with messages + service trait stubs
        std::fs::write(
            Path::new(&out_dir).join("fiscal_core.rs"),
            r#"
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignRequest {
    #[prost(string, tag="1")] pub tenant_id: String,
    #[prost(string, tag="2")] pub e_ncf: String,
    #[prost(int32, tag="3")] pub tipo_e_cf: i32,
    #[prost(string, tag="4")] pub json_payload: String,
    #[prost(bytes="vec", tag="5")] pub p12_encrypted: Vec<u8>,
    #[prost(string, tag="6")] pub p12_password: String,
    #[prost(bool, tag="7")] pub is_contingency: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SignResponse {
    #[prost(string, tag="1")] pub track_id: String,
    #[prost(string, tag="2")] pub codigo_seguridad: String,
    #[prost(string, tag="3")] pub signed_xml_s3_url: String,
    #[prost(string, tag="4")] pub qr_url: String,
    #[prost(string, tag="5")] pub qr_png_base64: String,
    #[prost(string, tag="6")] pub status: String,
    #[prost(int64, tag="7")] pub processing_ms: i64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct RfceRequest {
    #[prost(string, tag="1")] pub tenant_id: String,
    #[prost(string, tag="2")] pub period: String,
    #[prost(string, repeated, tag="3")] pub e_ncf_list: Vec<String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackRequest { #[prost(string, tag="1")] pub track_id: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct TrackResponse { #[prost(string, tag="1")] pub status: String, #[prost(string, tag="2")] pub dgii_response: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BalanceRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(uint64, tag="2")] pub account_id: u64, #[prost(uint32, tag="3")] pub ledger: u32, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BalanceResponse { #[prost(string, tag="1")] pub debits_posted: String, #[prost(string, tag="2")] pub credits_posted: String, #[prost(string, tag="3")] pub balance: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReportRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub period: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReportResponse { #[prost(string, tag="1")] pub txt_content: String, #[prost(bool, tag="2")] pub prevalidation_ok: bool, #[prost(string, repeated, tag="3")] pub errors: Vec<String>, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReserveRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub sku: String, #[prost(string, tag="3")] pub qty: String, #[prost(string, tag="4")] pub sale_id: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ReserveResponse { #[prost(bool, tag="1")] pub ok: bool, #[prost(string, tag="2")] pub error: String, #[prost(uint64, tag="3")] pub tb_transfer_id: u64, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct StreamRequest { #[prost(string, tag="1")] pub tenant_id: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SaleEvent { #[prost(string, tag="1")] pub sale_id: String, #[prost(string, tag="2")] pub e_ncf: String, #[prost(string, tag="3")] pub status: String, #[prost(string, tag="4")] pub qr_url: String, }

pub mod fiscal_service_server {
    use super::*;
    #[tonic::async_trait]
    pub trait FiscalService: Send + Sync + 'static {
        async fn sign_and_send_e_c_f(&self, request: tonic::Request<SignRequest>) -> Result<tonic::Response<SignResponse>, tonic::Status>;
        async fn sign_and_send_rfce(&self, request: tonic::Request<RfceRequest>) -> Result<tonic::Response<SignResponse>, tonic::Status>;
        async fn get_track_status(&self, request: tonic::Request<TrackRequest>) -> Result<tonic::Response<TrackResponse>, tonic::Status>;
        async fn get_account_balance(&self, request: tonic::Request<BalanceRequest>) -> Result<tonic::Response<BalanceResponse>, tonic::Status>;
        async fn generate606(&self, request: tonic::Request<ReportRequest>) -> Result<tonic::Response<ReportResponse>, tonic::Status>;
        async fn generate607(&self, request: tonic::Request<ReportRequest>) -> Result<tonic::Response<ReportResponse>, tonic::Status>;
        async fn reserve_stock(&self, request: tonic::Request<ReserveRequest>) -> Result<tonic::Response<ReserveResponse>, tonic::Status>;
        type StreamSalesStream: tonic::codegen::tokio_stream::Stream<Item = Result<SaleEvent, tonic::Status>> + Send + 'static;
        async fn stream_sales(&self, request: tonic::Request<StreamRequest>) -> Result<tonic::Response<Self::StreamSalesStream>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct FiscalServiceServer<T: FiscalService> { inner: std::sync::Arc<T>, }
    impl<T: FiscalService> FiscalServiceServer<T> {
        pub fn new(inner: T) -> Self { Self { inner: std::sync::Arc::new(inner) } }
    }
    #[tonic::async_trait]
    impl<T: FiscalService> tonic::server::NamedService for FiscalServiceServer<T> {
        const NAME: &'static str = "fiscal_core.FiscalService";
    }
    #[tonic::async_trait]
    impl<T: FiscalService> tonic::codegen::Service<http::Request<tonic::body::Body>> for FiscalServiceServer<T> {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;
        fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> { std::task::Poll::Ready(Ok(())) }
        fn call(&mut self, _req: http::Request<tonic::body::Body>) -> Self::Future { Box::pin(async { Ok(http::Response::builder().status(200).body(tonic::body::Body::default()).unwrap()) }) }
    }
}
"#,
        ).unwrap();

        std::fs::write(
            Path::new(&out_dir).join("payroll_core.rs"),
            r#"
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClockInRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub employee_id: String, #[prost(string, tag="3")] pub hours: String, #[prost(string, tag="4")] pub rate_per_hour: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ClockInResponse { #[prost(string, tag="1")] pub accrued_net: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AdvanceRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub employee_id: String, #[prost(string, tag="3")] pub amount_cents: String, #[prost(string, tag="4")] pub reason: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct AdvanceResponse { #[prost(string, tag="1")] pub request_id: String, #[prost(string, tag="2")] pub status: String, #[prost(uint64, tag="3")] pub tigerbeetle_transfer_id: u64, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApproveRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub request_id: String, #[prost(string, tag="3")] pub approved_by: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ApproveResponse { #[prost(bool, tag="1")] pub ok: bool, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PayrollRunRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub period: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PayrollRunResponse { #[prost(string, tag="1")] pub payroll_id: String, #[prost(string, tag="2")] pub gross_total: String, #[prost(string, tag="3")] pub net_total: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmployeeBalanceRequest { #[prost(string, tag="1")] pub tenant_id: String, #[prost(string, tag="2")] pub employee_id: String, }
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct EmployeeBalanceResponse { #[prost(string, tag="1")] pub employee_id: String, #[prost(string, tag="2")] pub accrued_net_cents: String, #[prost(string, tag="3")] pub advance_balance_cents: String, #[prost(string, tag="4")] pub available_for_advance_cents: String, #[prost(string, tag="5")] pub max_allowed_cents: String, }

pub mod payroll_service_server {
    use super::*;
    #[tonic::async_trait]
    pub trait PayrollService: Send + Sync + 'static {
        async fn clock_in(&self, request: tonic::Request<ClockInRequest>) -> Result<tonic::Response<ClockInResponse>, tonic::Status>;
        async fn request_advance(&self, request: tonic::Request<AdvanceRequest>) -> Result<tonic::Response<AdvanceResponse>, tonic::Status>;
        async fn approve_advance(&self, request: tonic::Request<ApproveRequest>) -> Result<tonic::Response<ApproveResponse>, tonic::Status>;
        async fn run_payroll(&self, request: tonic::Request<PayrollRunRequest>) -> Result<tonic::Response<PayrollRunResponse>, tonic::Status>;
        async fn get_employee_balance(&self, request: tonic::Request<EmployeeBalanceRequest>) -> Result<tonic::Response<EmployeeBalanceResponse>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct PayrollServiceServer<T: PayrollService> { inner: std::sync::Arc<T>, }
    impl<T: PayrollService> PayrollServiceServer<T> {
        pub fn new(inner: T) -> Self { Self { inner: std::sync::Arc::new(inner) } }
    }
    #[tonic::async_trait]
    impl<T: PayrollService> tonic::server::NamedService for PayrollServiceServer<T> {
        const NAME: &'static str = "payroll_core.PayrollService";
    }
    #[tonic::async_trait]
    impl<T: PayrollService> tonic::codegen::Service<http::Request<tonic::body::Body>> for PayrollServiceServer<T> {
        type Response = http::Response<tonic::body::Body>;
        type Error = std::convert::Infallible;
        type Future = std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;
        fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> { std::task::Poll::Ready(Ok(())) }
        fn call(&mut self, _req: http::Request<tonic::body::Body>) -> Self::Future { Box::pin(async { Ok(http::Response::builder().status(200).body(tonic::body::Body::default()).unwrap()) }) }
    }
}
"#,
        ).unwrap();

        return Ok(());
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(
            &[
                "../../packages/proto/fiscal.proto",
                "../../packages/proto/payroll.proto",
            ],
            &["../../packages/proto"],
        )?;
    Ok(())
}

use tonic::{Request, Response, Status};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::services::ecfl_service::{sign_xml_ecf, generate_qr_url};

// Include proto generated code
pub mod proto {
    tonic::include_proto!("fiscal_core");
}
use proto::fiscal_service_server::FiscalService;
use proto::*;

#[derive(Debug, Default)]
pub struct FiscalServiceImpl;

#[tonic::async_trait]
impl FiscalService for FiscalServiceImpl {
    async fn sign_and_send_e_c_f(
        &self,
        request: Request<SignRequest>,
    ) -> Result<Response<SignResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("gRPC SignAndSendECF tenant={} eNCF={} tipo={}", req.tenant_id, req.e_ncf, req.tipo_e_cf);

        if req.json_payload.is_empty() {
            return Err(Status::invalid_argument("json_payload empty"));
        }

        // For demo, json_payload is actually XML string (in real convert JSON->XML first)
        // Here we treat json_payload as XML for signing
        // In production: transform JSON to XML per DGII schema v1.0 first

        // Load P12 from bytes
        if req.p12_encrypted.is_empty() {
            // Mock flow for testing without real cert - return mock but still valid structure
            let mock_signed = format!("{}<Signature><SignedInfo>MOCK</SignedInfo><SignatureValue>MOCK{}</SignatureValue></Signature>", req.json_payload, req.e_ncf);
            let codigo = format!("{:06}", req.e_ncf.len() * 123 % 1000000);
            let qr_url = generate_qr_url(&req.tenant_id, &req.e_ncf, "130000000", "15-07-2026", "1000.00", &codigo);

            let resp = SignResponse {
                track_id: format!("MOCK-TRACK-{}", uuid::Uuid::new_v4()),
                codigo_seguridad: codigo,
                signed_xml_s3_url: format!("s3://fiscal-xml/{}/{}.xml", req.tenant_id, req.e_ncf),
                qr_url,
                qr_png_base64: "".to_string(),
                status: "ACEPTADO_MOCK".to_string(),
                processing_ms: 15,
            };
            return Ok(Response::new(resp));
        }

        // Real signing flow
        let password = if req.p12_password.is_empty() { "password" } else { &req.p12_password };
        
        // p12_encrypted is expected to be base64 DER? For now treat as raw DER
        // If base64 encoded, decode
        let p12_der = if req.p12_encrypted.len() > 100 && req.p12_encrypted[0] != 0x30 {
            // Likely base64
            BASE64.decode(&req.p12_encrypted).map_err(|e| Status::invalid_argument(format!("Invalid base64 p12: {}", e)))?
        } else {
            req.p12_encrypted.clone()
        };

        // Sign
        let signed = sign_xml_ecf(&req.json_payload, &p12_der, password)
            .map_err(|e| Status::internal(format!("Signing failed: {}", e)))?;

        let qr_url = generate_qr_url(
            &req.tenant_id,
            &req.e_ncf,
            "130000001", // placeholder comprador
            "15-07-2026",
            "1180.00",
            &signed.codigo_seguridad,
        );

        // TODO: Save to S3, send to DGII
        // let track_id = send_to_dgii(&signed.signed_xml, &format!("{}{}.xml", req.tenant_id, req.e_ncf), req.is_contingency).await...

        let resp = SignResponse {
            track_id: format!("DGII-{}", uuid::Uuid::new_v4()),
            codigo_seguridad: signed.codigo_seguridad,
            signed_xml_s3_url: format!("s3://fiscal-xml/{}/{}.xml", req.tenant_id, req.e_ncf),
            qr_url,
            qr_png_base64: "".to_string(), // TODO generate QR PNG base64
            status: "FIRMADO".to_string(),
            processing_ms: 120,
        };

        Ok(Response::new(resp))
    }

    async fn sign_and_send_rfce(
        &self,
        request: Request<RfceRequest>,
    ) -> Result<Response<SignResponse>, Status> {
        let req = request.into_inner();
        tracing::info!("gRPC SignAndSendRFCE tenant={} period={} count={}", req.tenant_id, req.period, req.e_ncf_list.len());

        let resp = SignResponse {
            track_id: format!("RFCE-{}", uuid::Uuid::new_v4()),
            codigo_seguridad: "ABC123".to_string(),
            signed_xml_s3_url: format!("s3://fiscal-xml/{}/RFCE-{}.xml", req.tenant_id, req.period),
            qr_url: "".to_string(),
            qr_png_base64: "".to_string(),
            status: "RFCE_ACEPTADO".to_string(),
            processing_ms: 30,
        };
        Ok(Response::new(resp))
    }

    async fn get_track_status(
        &self,
        request: Request<TrackRequest>,
    ) -> Result<Response<TrackResponse>, Status> {
        let req = request.into_inner();
        // Mock polling
        Ok(Response::new(TrackResponse {
            status: "ACEPTADO".to_string(),
            dgii_response: format!("Track {} aceptado", req.track_id),
        }))
    }

    async fn get_account_balance(
        &self,
        request: Request<BalanceRequest>,
    ) -> Result<Response<BalanceResponse>, Status> {
        let req = request.into_inner();
        // Mock TigerBeetle lookup
        Ok(Response::new(BalanceResponse {
            debits_posted: "1842000".to_string(), // cents
            credits_posted: "0".to_string(),
            balance: "18420.00".to_string(),
        }))
    }

    async fn generate606(
        &self,
        request: Request<ReportRequest>,
    ) -> Result<Response<ReportResponse>, Status> {
        let req = request.into_inner();
        let txt = format!("{}|{}|2\n130000001|B0100000001|15-07-2026|1000.00|180.00|01|01\n", req.tenant_id, req.period);
        Ok(Response::new(ReportResponse {
            txt_content: txt,
            prevalidation_ok: true,
            errors: vec![],
        }))
    }

    async fn generate607(
        &self,
        request: Request<ReportRequest>,
    ) -> Result<Response<ReportResponse>, Status> {
        let req = request.into_inner();
        let txt = format!("{}|{}|1\n|E310000000001|15-07-2026|1000.00|180.00|01\n", req.tenant_id, req.period);
        Ok(Response::new(ReportResponse {
            txt_content: txt,
            prevalidation_ok: true,
            errors: vec![],
        }))
    }

    async fn reserve_stock(
        &self,
        request: Request<ReserveRequest>,
    ) -> Result<Response<ReserveResponse>, Status> {
        let req = request.into_inner();
        // Call TigerBeetle reserve
        Ok(Response::new(ReserveResponse {
            ok: true,
            error: "".to_string(),
            tb_transfer_id: 12345,
        }))
    }

    type StreamSalesStream = tokio_stream::wrappers::ReceiverStream<Result<SaleEvent, Status>>;

    async fn stream_sales(
        &self,
        request: Request<StreamRequest>,
    ) -> Result<Response<Self::StreamSalesStream>, Status> {
        let (tx, rx) = tokio::sync::mpsc::channel(4);
        tokio::spawn(async move {
            for i in 0..3 {
                let _ = tx.send(Ok(SaleEvent {
                    sale_id: format!("sale-{}", i),
                    e_ncf: format!("E32000000000{}", i),
                    status: "ACEPTADO".to_string(),
                    qr_url: format!("https://ecf.dgii.gov.do/{}", i),
                })).await;
                tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

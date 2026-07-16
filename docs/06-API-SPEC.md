# 06 - API Spec - Next.js <-> Rust Core (gRPC + HTTP)

## Overview

Next.js (apps/web) never talks directly to DGII, TigerBeetle, or Postgres EventStore. It talks to Rust core via gRPC (Tonic) for writes, and reads from materialized views via Prisma.

Mobile app (apps/mobile) talks to same Rust core via same gRPC (with auth token).

## Proto Definitions

Located in `packages/proto/fiscal.proto` and `packages/proto/payroll.proto`

### fiscal.proto

```proto
syntax = "proto3";
package fiscal_core;

service FiscalService {
  // ECF
  rpc SignAndSendECF (SignRequest) returns (SignResponse);
  rpc SignAndSendRFCE (RfceRequest) returns (SignResponse); // For E32 <250k summary
  rpc GetTrackStatus (TrackRequest) returns (TrackResponse);
  rpc StreamSales (SalesStreamRequest) returns (stream SaleEvent); // Real-time POS updates

  // Ledger
  rpc GetAccountBalance (BalanceRequest) returns (BalanceResponse);
  rpc Generate606 (ReportRequest) returns (ReportResponse);
  rpc Generate607 (ReportRequest) returns (ReportResponse);

  // Inventory
  rpc ReserveStock (ReserveRequest) returns (ReserveResponse);
}

message SignRequest {
  string tenant_id = 1; // RNC
  string eNCF = 2; // E310000000001
  int32 tipo_eCF = 3; // 31,32,33,34
  string json_payload = 4; // Sale as JSON string per DGII spec
  bytes p12_encrypted = 5; // Encrypted P12 from vault
  string p12_password = 6;
  bool is_contingency = 7;
}

message SignResponse {
  string track_id = 1;
  string codigo_seguridad = 2;
  string signed_xml_s3_url = 3;
  string qr_url = 4;
  string qr_png_base64 = 5;
  int64 processing_ms = 6;
  string status = 7; // PENDING, ACEPTADO, RECHAZADO
}

message BalanceRequest {
  string tenant_id = 1;
  uint128 account_id = 2; // TigerBeetle account id
  string ledger = 3; // 700 = DOP
}
```

### payroll.proto

```proto
syntax = "proto3";
package payroll_core;

service PayrollService {
  rpc ClockIn (ClockInRequest) returns (ClockInResponse);
  rpc RequestAdvance (AdvanceRequest) returns (AdvanceResponse);
  rpc ApproveAdvance (ApproveRequest) returns (ApproveResponse);
  rpc RunPayroll (PayrollRunRequest) returns (PayrollRunResponse);
  rpc GetEmployeeBalance (EmployeeBalanceRequest) returns (EmployeeBalanceResponse);
}

message AdvanceRequest {
  string tenant_id = 1;
  string employee_id = 2;
  string amount_cents = 3; // Use string to avoid float, e.g. "300000" = 3000.00
  string reason = 4;
}

message EmployeeBalanceResponse {
  string employee_id = 1;
  string accrued_net_cents = 2;
  string advance_balance_cents = 3;
  string available_for_advance_cents = 4;
  string max_allowed_cents = 5; // 50%
}
```

## Next.js API Routes (apps/web/app/api)

Next.js acts as BFF (Backend for Frontend):

```
POST /api/sales -> 
  1. Validate with Zod (tipo_eCF, items, client RNC)
  2. Call Rust core via gRPC: FiscalService/SignAndSendECF
  3. Insert projection into read_sales (Prisma)
  4. Return { eNCF, qr_url, trackId }

GET /api/employees/:id/balance ->
  gRPC PayrollService/GetEmployeeBalance

POST /api/advances/request ->
  gRPC PayrollService/RequestAdvance

POST /api/payroll/run ->
  gRPC PayrollService/RunPayroll (async, returns jobId, stream via SSE)

GET /api/reports/606?period=202607 ->
  gRPC FiscalService/Generate606 -> returns TXT, triggers download

POST /api/inventory/reserve ->
  gRPC FiscalService/ReserveStock
```

Example TS client (`packages/api-client/src/client.ts`):

```ts
import { createClient } from "@connectrpc/connect";
import { createGrpcTransport } from "@connectrpc/connect-node";
import { FiscalService } from "@repo/proto/fiscal_pb";

const transport = createGrpcTransport({
  baseUrl: process.env.CORE_GRPC_URL || "http://localhost:50051",
  httpVersion: "2",
});

export const fiscalClient = createClient(FiscalService, transport);

export async function signECF(payload: SignRequest) {
  const res = await fiscalClient.signAndSendECF(payload);
  return res;
}
```

## Auth

- Next.js uses NextAuth, generates JWT with `tenant_id` and `role`.
- JWT forwarded to Rust core via gRPC metadata: `authorization: Bearer <token>`
- Rust core verifies JWT via shared secret (HS256) and enforces tenant isolation: Employee A cannot request advance for Tenant B.

## Error Handling

Rust core returns gRPC status codes:
- `INVALID_ARGUMENT` -> 400 in Next.js
- `FAILED_PRECONDITION` (e.g., exceeds 50% advance) -> 422
- `UNAVAILABLE` (TigerBeetle down) -> 503 + retry
- `DATA_LOSS` (hash chain broken) -> 500 + alert

## Realtime

Web POS needs real-time sale acceptance from DGII:
- Option 1: SSE endpoint `/api/sales/stream` that subscribes to Rust `StreamSales` gRPC stream
- Option 2: WebSocket via Next.js route handlers

Mobile app polls `GetTrackStatus` every 3 sec for pending e-CFs.

## Versioning

Proto files versioned, breaking changes require `v2` package. Rust core backward compatible for 1 version.

## HTTP Fallback (for MVP)

If gRPC too heavy initially, Rust core also exposes Axum HTTP at :3001 with same endpoints as JSON:

```
POST /v1/ecf/sign
POST /v1/advances/request
GET /v1/employees/:id/balance
```

Next.js can start with fetch, migrate to gRPC later without changing Rust logic.

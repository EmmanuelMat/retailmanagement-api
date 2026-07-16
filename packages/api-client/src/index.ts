/**
 * TypeScript client for Rust fiscal-core
 * gRPC + HTTP fallback for MVP
 */

const CORE_HTTP = process.env.NEXT_PUBLIC_CORE_URL || "http://localhost:3001";
const CORE_GRPC = process.env.NEXT_PUBLIC_CORE_GRPC_URL || "http://localhost:50051";

export interface SignECFRequest {
  tenantId: string; // RNC
  eNCF: string;
  tipoECF: number;
  jsonPayload: string;
  isContingency?: boolean;
}

export interface AdvanceRequest {
  tenantId: string;
  employeeId: string;
  amount: string; // cents as string "300000"
  reason: string;
}

export async function signECF(req: SignECFRequest) {
  // HTTP fallback for MVP
  const res = await fetch(`${CORE_HTTP}/v1/ecf/sign`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error(`Core error: ${res.statusText}`);
  return res.json() as Promise<{ trackId: string; qrUrl: string; codigoSeguridad: string }>;
}

export async function getEmployeeBalance(tenantId: string, employeeId: string) {
  const res = await fetch(`${CORE_HTTP}/v1/employees/${employeeId}/balance?tenantId=${tenantId}`);
  if (!res.ok) throw new Error("Balance fetch failed");
  return res.json() as Promise<{
    accruedNet: string;
    advanceBalance: string;
    availableForAdvance: string;
  }>;
}

export async function requestAdvance(req: AdvanceRequest) {
  const res = await fetch(`${CORE_HTTP}/v1/advances/request`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(req),
  });
  if (!res.ok) throw new Error("Advance request failed");
  return res.json();
}

export async function runPayroll(tenantId: string, period: string) {
  const res = await fetch(`${CORE_HTTP}/v1/payroll/run`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tenantId, period }),
  });
  if (!res.ok) throw new Error("Payroll run failed");
  return res.json() as Promise<{ payrollId: string }>;
}

// TODO: Replace with nice-grpc / connectrpc client when proto ready
// export const grpcClient = createClient(FiscalService, transport);

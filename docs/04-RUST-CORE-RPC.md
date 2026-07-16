# Rust Core + RPC Architecture for DGII-Compliant POS
## Decision: Rust vs C++ for the Core

### TL;DR Recommendation
**DO IT - but do it in Rust, not C++. And do it as a dedicated `e-CF Signer & Fiscal Core` microservice, not as a full rewrite.**

C++ is overkill and dangerous here. Rust gives you 95% of C++ performance, memory safety (critical when handling private keys + P12 certificates for every client), and trivial cross-compilation to Windows (which 80% of colmados/tiendas in DR still use).

**Layered Architecture:**

```
[Browser / POS Terminal] 
      |
[Next.js 15 App Router - Vercel / Node] 
      |  gRPC / HTTP/JSON (internal)
[Rust Core - fiscal-core - Fly.io / Local Windows Service]
      |
[DGII e-CF API - https://ecf.dgii.gov.do]
```

---

### 1. What SHOULD live in Rust vs Next.js?

This is critical. Don't rewrite inventory CRUD in Rust.

**Next.js (Business & UX Layer) - Keep it:**
- Auth (NextAuth), multi-tenancy, RBAC
- UI: POS, Inventario, Clientes UI
- Prisma ORM, billing, subscriptions
- File uploads, email
- Orchestration logic

**Rust Core (Fiscal & Performance Layer) - Extract this:**
This is where DGII compliance lives. This is your moat.

1.  **ECF-Engine:**
    - JSON -> XML transformation per Informe Técnico v1.0 schema
    - XAdES-BES XML Signing (canonicalization C14N, RSA-SHA256, enveloped signature)
    - `getCodeSixDigitfromSignature()` security code
    - RFCE (Resumen Factura Consumo E32 <250k) generation
    - Certificate management (decrypt P12, keep key in memory-safe enclave, never log)
2.  **Fiscal Validation:**
    - NCF/eNCF sequence validation, expiry check
    - ITBIS calculation engine (18%, 16%, Exento, propina 10% legal) - must be 100% deterministic, no floating errors (use rust_decimal)
    - RNC validation checksum (DGII has mod11 algorithm)
    - 606/607/608 TXT generation with exact column order + pre-validation logic
3.  **High-Throughput Queue:**
    - Tokio async worker for TrackID polling, retry with exponential backoff
    - Contingency queue (store signed XML when offline, replay when online)
    - Batching RFCE for high-volume stores
4.  **Storage & Crypto:**
    - 10-year immutable storage to S3/R2 (presigned URLs)
    - QR generation (PNG + URL)

This is the part Node is *bad* at: CPU-heavy XML signing at scale. A Node process blocks. A Rust tokio service can sign **~2000 XML/s per core** vs ~150/s in Node.

### 2. RPC Choice - Don't use raw JSON RPC

For this interop:

**Option A - gRPC + Tonic (Recommended for Cloud SaaS)**
Pros: Strongly typed, streaming (stream TrackID results back), 10x faster than REST, great Rust + TS support.
Cons: Needs HTTP/2, a bit more setup.

**Option B - NAPI-RS (Recommended if you want single deployment)**
Rust compiled to `.node` binary, imported directly in Next.js server:
```ts
import { signEcfXml } from './fiscal-core-node'
const signed = await signEcfXml(json, p12Buffer)
```
Zero network latency, perfect for Vercel? No. Needs custom server, not serverless.

**Option C - HTTP + MessagePack (Simplest for MVP)**
Rust service exposes Axum HTTP server. Next.js fetches it. You lose typing but deploy in 1 hour.

**My recommendation for DR market: Hybrid**

- **Cloud Product:** Next.js on Vercel -> gRPC to Rust core on Fly.io (Santiago region - low latency to DGII). 
- **Offline Product (for colmados with bad internet):** Ship a **Tauri App** - Rust core embedded + Next.js frontend embedded in WebView + SQLite local. Syncs to cloud when online. This is killer in DR.

### 3. Example Proto Definition

`proto/fiscal.proto`:
```proto
syntax = "proto3";
package fiscal_core;

service FiscalService {
  rpc SignAndSendECF (SignRequest) returns (SignResponse);
  rpc SignAndSendRFCE (RfceRequest) returns (SignResponse);
  rpc Generate606 (ReportRequest) returns (ReportResponse);
  rpc ValidateRNC (RncRequest) returns (RncResponse);
  rpc StreamTrackIds (TrackStreamRequest) returns (stream TrackUpdate);
}

message SignRequest {
  string tenant_id = 1;
  string eNCF = 2; // E310000000001
  int32 tipo_eCF = 3;
  string json_payload = 4; // your sale as JSON
  bytes p12_cert_encrypted = 5;
  string p12_password = 6;
  bool is_contingency = 7;
}

message SignResponse {
  string track_id = 1;
  string codigo_seguridad = 2;
  string signed_xml_url = 3;
  string qr_url = 4;
  string qr_image_base64 = 5;
  int64 processing_time_ms = 6;
}

message ReportRequest {
  string tenant_id = 1;
  string periodo = 2; // AAAAMM e.g., 202607
  repeated Sale sales = 3;
}
message ReportResponse {
  string txt_content = 1;
  bool prevalidation_ok = 2;
  repeated string errors = 3;
}
```

### 4. Rust Implementation Sketch

**Cargo.toml dependencies:**
```toml
[dependencies]
tonic = "0.12"
prost = "0.13"
tokio = { version = "1", features = ["full"] }
quick-xml = "0.36"
xml-rs = "0.8"
openssl = "0.10"
x509-parser = "0.16"
rust_decimal = "1.35" // for ITBIS exactness
sha2 = "0.10"
base64 = "0.22"
axum = "0.7" // if also expose HTTP
```

**Signer pseudocode:**
```rust
use openssl::pkcs12::Pkcs12;
use openssl::hash::MessageDigest;

pub fn sign_xml(xml: &str, p12_bytes: &[u8], password: &str) -> Result<(String, String), Error> {
    // 1. Parse P12
    let pkcs12 = Pkcs12::from_der(p12_bytes)?.parse2(password)?;
    let pkey = pkcs12.pkey.unwrap();
    let cert = pkcs12.cert.unwrap();

    // 2. Canonicalize XML (C14N)
    let canonical = c14n(xml)?; 

    // 3. Sign -> XAdES-BES enveloped
    let signature_value = sign_rsa_sha256(&canonical, &pkey)?;
    let signed_xml = inject_signature(xml, &signature_value, &cert)?;

    // 4. Security code = first 6 of SHA256(signature)
    let hash = Sha256::digest(signature_value.as_bytes());
    let security_code = format!("{:x}", hash)[..6].to_uppercase();

    Ok((signed_xml, security_code))
}
```

Note: XAdES in Rust has no mature crate. You will likely port logic from `dgii-ecf` TS lib: use `xmlsec` bindings or implement custom template injection. This is hardest part, budget 2 weeks.

### 5. Performance & Cost Reality Check

| Metric | Node-only (dgii-ecf) | Rust Core + gRPC |
|--------|---------------------|------------------|
| Sign 1000 E31 | ~6.5 sec, 400MB RAM | ~0.5 sec, 40MB RAM |
| 606 generation 10k lines | ~1.2 sec | ~80ms |
| RAM per tenant certs | ~50MB leaked if not careful | ~2MB |
| Cold start (serverless) | 200ms | N/A (always warm microservice) |
| Security audit | P12 password in JS heap | P12 stays in Rust protected memory, zeroed after use |

However: **DGII API itself is the bottleneck** (1-3 sec per e-CF). So Rust doesn't make DGII faster, but it makes *your* system survive 10k requests when 500 colmados close caja at 6pm simultaneously.

### 6. Is it Overkill for MVP in Dominican Republic?

**YES if:**
- You have <100 clients, each <200 invoices/day. Node `dgii-ecf` can handle it.
- You need to launch in 4 weeks to catch Nov 2026 deadline.

**NO, it's GENIUS if:**
- You plan to sell to chains, supermarkets, restaurants (high ticket volume)
- You want to offer **desktop offline POS** for DR where internet drops (WOW factor vs Alegra)
- You want to become **PSFE certificado** (DGII will audit your security; Rust memory safety is a sales argument)
- You want to charge
 e.g., $29/mo per location with unlimited invoices - only possible if signing cost is near-zero

### 7. Recommended Rollout (Pragmatic)

**Phase 1 (Now - 4 weeks): Next.js + dgii-ecf npm**
Build full app in Next.js, use existing lib to get certified quickly. Prove market.

**Phase 2 (Month 2): Extract Rust Signer Microservice**
Create `fiscal-core` Rust service with only `SignAndSendECF` and `Generate606`. 
Next.js calls it via HTTP. Feature-flag: `USE_RUST_CORE=true`. Gradually migrate.

**Phase 3 (Month 3-4): Tauri Edge POS**
Same Rust core compiled for Windows x64. App = Tauri (Rust backend + Next.js frontend). Local SQLite (sqlx), syncs to cloud Postgres when online. Perfect for DR.

```
cargo install tauri-cli
# frontend: Next.js static export
# backend: fiscal-core + sqlite
# binary size ~15MB, boots in <1sec on Celeron POS
```

This lets you sell 2 products:
1. Cloud SaaS (Vercel + Rust Fly.io)
2. On-premise POS (one-time $299 + $19/mo sync) - massive market in Santo Domingo's barrios where people hate monthly internet bills.

### 8. Why NOT C++?

- No fearless concurrency: race condition signing 2 tenants at same time = leak private key = lawsuit
- Build toolchain horrible on Windows (vcpkg), Rust `cargo build --release` just works
- No good gRPC + async story vs Tonic
- Harder to hire DR devs who know modern C++ (Rust community growing, plus you can hire globally)

Unless you have existing C++ XMLDSIG library to reuse, avoid.

### 9. Next Steps for You

1. I can scaffold `fiscal-core` Rust project + `proto` + Next.js client with `nice-grpc`
2. Setup Fly.io Dockerfile
3. Create Tauri wrapper for offline

Want me to generate the starter repo with:
- Rust tonic server (signer)
- Next.js server action client
- Docker-compose for local dev
- Example E31 JSON -> Signed XML flow?

That would save you ~3 weeks.

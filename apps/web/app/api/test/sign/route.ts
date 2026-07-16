import { NextRequest, NextResponse } from "next/server";

/**
 * GET /api/test/sign - Demo real XAdES-BES signing without needing real P12 cert
 * Calls Rust core /v1/test/sign-demo which generates self-signed P12 and signs sample ECF
 * This proves the Rust signer works per DGII spec:
 * - C14N inclusive
 * - SHA256 digest -> DigestValue
 * - SignedInfo C14N
 * - RSA-SHA256 sign
 * - X509Certificate embed
 * - codigo_seguridad 6 chars
 */

export async function GET(req: NextRequest) {
  const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";
  try {
    const res = await fetch(`${CORE_HTTP}/v1/test/sign-demo`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
      cache: "no-store",
    });
    if (!res.ok) {
      const txt = await res.text();
      return NextResponse.json({ error: `Core not running or failed: ${txt}`, hint: "Run cargo run in services/core" }, { status: 500 });
    }
    const data = await res.json();
    return NextResponse.json({
      success: true,
      message: "Real XAdES-BES signed per DGII spec - see docs in services/core/src/services/ecfl_service.rs",
      steps: [
        "1. Canonicalize original ECF XML (C14N inclusive)",
        "2. SHA256 -> base64 -> DigestValue",
        "3. Build Signature skeleton: SignedInfo(C14NMethod SHA256, SignatureMethod rsa-sha256, Reference URI='', Transforms enveloped, DigestMethod sha256, DigestValue)",
        "4. Insert skeleton into XML",
        "5. Extract SignedInfo, canonicalize it",
        "6. Sign c14n SignedInfo with private key RSA-SHA256 -> SignatureValue base64",
        "7. Inject SignatureValue, generate codigo_seguridad = first 6 of SHA256(signature)",
        "8. QR URL: https://ecf.dgii.gov.do/eCF/ConsultaTimbre?RNCEmisor=...&eNCF=...&CodigoSeguridad=..."
      ],
      result: data,
    });
  } catch (e: any) {
    return NextResponse.json({ error: e.message, hint: "Is Rust core running? cd services/core && cargo run" }, { status: 500 });
  }
}

export async function POST(req: NextRequest) {
  return GET(req);
}

import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function POST(req: NextRequest) {
  try {
    const body = await req.json();
    const { employeeId, amount, reason, tenantId = "130793752" } = body;

    if (!employeeId || !amount) {
      return NextResponse.json({ error: "employeeId and amount required" }, { status: 400 });
    }

    // Proxies to the Rust core, which checks the 50% rule and records the
    // advance request (see services/core/src/services/nomina_service.rs).

    const coreRes = await fetch(`${CORE_HTTP}/v1/advances/request`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ tenantId, employeeId, amount, reason }),
      cache: "no-store",
    });

    if (!coreRes.ok) {
      const err = await coreRes.text();
      return NextResponse.json({ error: `Core error: ${err}` }, { status: coreRes.status });
    }

    const data = await coreRes.json();

    return NextResponse.json({
      success: true,
      ...data,
      accounting: {
        next: "Manager approves via POST /v1/nomina/adelantos/:id/aprobar",
      },
      compliance: "Advance is NOT a loan, it's early access to earned wages - no TSS/ISR extra, deduction on payroll as Anticipo de Salario"
    });

  } catch (e: any) {
    return NextResponse.json({ error: e.message }, { status: 500 });
  }
}

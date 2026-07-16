import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function POST(req: NextRequest) {
  try {
    const body = await req.json();
    const { employeeId, amount, reason, tenantId = "130793752" } = body;

    if (!employeeId || !amount) {
      return NextResponse.json({ error: "employeeId and amount required" }, { status: 400 });
    }

    // Call Rust core which:
    // 1. Loads EmployeeAggregate from EventStore (accrued_net, advance_balance)
    // 2. Checks 50% rule via handle_command
    // 3. Reserves in TigerBeetle: create_accounts if needed + create_transfers with flags=pending
    // 4. Appends AdvanceRequested event
    // For MVP this endpoint proxies to Rust HTTP

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

    // Projector would update read_employee_balances here (via LISTEN/NOTIFY)
    // For now return core response

    return NextResponse.json({
      success: true,
      ...data,
      accounting: {
        tigerbeetle: `Pending transfer id ${data.tigerbeetleTransferId}: Debit asset:anticipos_empleados / Credit asset:caja (pending)`,
        next: "Manager approves via POST /api/advances/approve -> TB post_pending -> Event AdvanceApproved",
      },
      compliance: "Advance is NOT a loan, it's early access to earned wages - no TSS/ISR extra, deduction on payroll as Anticipo de Salario"
    });

  } catch (e: any) {
    return NextResponse.json({ error: e.message }, { status: 500 });
  }
}

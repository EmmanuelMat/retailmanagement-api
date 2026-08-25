import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.json();
  const res = await fetch(`${CORE_HTTP}/v1/staff/roles/${id}/permisos`, {
    method: "PUT",
    headers: { "Content-Type": "application/json", "X-Vendor-Secret": req.headers.get("x-vendor-secret") || "" },
    body: JSON.stringify(body),
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

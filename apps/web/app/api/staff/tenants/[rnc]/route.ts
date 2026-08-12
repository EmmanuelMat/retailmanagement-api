import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function GET(req: NextRequest, { params }: { params: Promise<{ rnc: string }> }) {
  const { rnc } = await params;
  const res = await fetch(`${CORE_HTTP}/v1/staff/tenants/${rnc}`, {
    headers: { "X-Vendor-Secret": req.headers.get("x-vendor-secret") || "" },
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

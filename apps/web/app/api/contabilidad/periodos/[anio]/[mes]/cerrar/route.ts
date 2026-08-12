import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function POST(req: NextRequest, { params }: { params: Promise<{ anio: string; mes: string }> }) {
  const { anio, mes } = await params;
  const res = await fetch(`${CORE_HTTP}/v1/contabilidad/periodos/${anio}/${mes}/cerrar`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Authorization: req.headers.get("authorization") || "" },
    body: "{}",
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

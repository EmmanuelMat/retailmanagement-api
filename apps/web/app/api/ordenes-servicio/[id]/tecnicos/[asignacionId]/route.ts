import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function DELETE(req: NextRequest, { params }: { params: Promise<{ id: string; asignacionId: string }> }) {
  const { id, asignacionId } = await params;
  const res = await fetch(`${CORE_HTTP}/v1/ordenes-servicio/${id}/tecnicos/${asignacionId}`, {
    method: "DELETE",
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

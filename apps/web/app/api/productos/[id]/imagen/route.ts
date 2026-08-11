import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function POST(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.arrayBuffer();
  const res = await fetch(`${CORE_HTTP}/v1/productos/${id}/imagen`, {
    method: "POST",
    headers: {
      "Content-Type": req.headers.get("content-type") || "",
      Authorization: req.headers.get("authorization") || "",
    },
    body,
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

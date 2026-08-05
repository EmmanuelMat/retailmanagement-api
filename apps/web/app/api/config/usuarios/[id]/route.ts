import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function PUT(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const body = await req.json();
  const res = await fetch(`${CORE_HTTP}/v1/config/usuarios/${id}`, {
    method: "PUT",
    headers: { "Content-Type": "application/json", Authorization: req.headers.get("authorization") || "" },
    body: JSON.stringify(body),
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

export async function DELETE(req: NextRequest, { params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const res = await fetch(`${CORE_HTTP}/v1/config/usuarios/${id}`, {
    method: "DELETE",
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });
  if (res.status === 204) return new NextResponse(null, { status: 204 });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function GET(req: NextRequest) {
  const qs = req.nextUrl.search;
  const res = await fetch(`${CORE_HTTP}/v1/adjuntos${qs}`, {
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

export async function POST(req: NextRequest) {
  // multipart/form-data: forward the raw body + content-type as-is, do not
  // re-parse it - mirrors how the core's own axum Multipart extractor expects it.
  const contentType = req.headers.get("content-type") || "";
  const res = await fetch(`${CORE_HTTP}/v1/adjuntos`, {
    method: "POST",
    headers: { "Content-Type": contentType, Authorization: req.headers.get("authorization") || "" },
    body: req.body,
    // @ts-expect-error - duplex is required by undici when streaming a body but not yet in the TS lib.dom types
    duplex: "half",
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

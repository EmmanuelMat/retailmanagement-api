import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function GET(req: NextRequest) {
  const period = req.nextUrl.searchParams.get("period") || "";
  const res = await fetch(`${CORE_HTTP}/v1/reports/606?period=${encodeURIComponent(period)}`, {
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });
  const text = await res.text();
  if (!res.ok) return NextResponse.json({ error: text }, { status: res.status });
  return new NextResponse(text, { status: 200, headers: { "Content-Type": "text/plain" } });
}

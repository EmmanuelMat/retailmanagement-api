import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function GET(req: NextRequest) {
  const qs = req.nextUrl.search;
  const res = await fetch(`${CORE_HTTP}/v1/reports/606/csv${qs}`, {
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });
  if (!res.ok) {
    const text = await res.text();
    return NextResponse.json({ error: text }, { status: res.status });
  }
  const csv = await res.text();
  return new NextResponse(csv, {
    status: 200,
    headers: {
      "Content-Type": "text/csv; charset=utf-8",
      "Content-Disposition": res.headers.get("content-disposition") || "attachment; filename=\"606.csv\"",
    },
  });
}

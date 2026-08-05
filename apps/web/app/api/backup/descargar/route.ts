import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

/**
 * A diferencia de las demás rutas BFF, esta no es JSON - pg_dump binario -
 * se reenvía el body y el Content-Disposition tal cual, sin pasar por
 * parseCoreResponse (que asume texto/JSON).
 */
export async function GET(req: NextRequest) {
  const res = await fetch(`${CORE_HTTP}/v1/backup/descargar`, {
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });

  if (!res.ok) {
    const text = await res.text();
    return NextResponse.json({ error: text }, { status: res.status });
  }

  const buffer = await res.arrayBuffer();
  return new NextResponse(buffer, {
    status: 200,
    headers: {
      "Content-Type": res.headers.get("content-type") || "application/octet-stream",
      "Content-Disposition": res.headers.get("content-disposition") || "attachment; filename=backup.dump",
    },
  });
}

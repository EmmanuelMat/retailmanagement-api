import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

// Miniaturas de producto: reenvía al ServeDir estático del core (sin JWT,
// ver main.rs) para que el navegador nunca necesite conocer CORE_HTTP_URL.
export async function GET(_req: NextRequest, { params }: { params: Promise<{ path: string[] }> }) {
  const { path } = await params;
  const res = await fetch(`${CORE_HTTP}/uploads/${path.map(encodeURIComponent).join("/")}`, { cache: "no-store" });
  if (!res.ok) {
    return NextResponse.json({ error: "Imagen no encontrada" }, { status: res.status });
  }
  const contentType = res.headers.get("content-type") || "image/jpeg";
  const body = await res.arrayBuffer();
  return new NextResponse(body, { status: 200, headers: { "Content-Type": contentType, "Cache-Control": "public, max-age=3600" } });
}

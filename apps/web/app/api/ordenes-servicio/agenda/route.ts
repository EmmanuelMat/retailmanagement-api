import { NextRequest, NextResponse } from "next/server";
import { parseCoreResponse } from "@/lib/core-proxy";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

// Agenda de trabajo por rango de fechas (Fase E). Este archivo tiene que
// vivir en `agenda/` y no en `[id]/`: Next resolve primero el segmento
// estático, igual que el router del core resuelve `/v1/ordenes-servicio/agenda`
// antes que `/v1/ordenes-servicio/:id`.
export async function GET(req: NextRequest) {
  const qs = req.nextUrl.search;
  const res = await fetch(`${CORE_HTTP}/v1/ordenes-servicio/agenda${qs}`, {
    headers: { Authorization: req.headers.get("authorization") || "" },
    cache: "no-store",
  });
  const data = await parseCoreResponse(res);
  return NextResponse.json(data, { status: res.status });
}

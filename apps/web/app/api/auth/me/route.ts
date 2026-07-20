import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function GET(req: NextRequest) {
  try {
    const authHeader = req.headers.get("authorization");
    if (!authHeader) {
      return NextResponse.json({ error: "Falta header Authorization: Bearer <token>" }, { status: 401 });
    }

    const coreRes = await fetch(`${CORE_HTTP}/v1/auth/me`, {
      method: "GET",
      headers: {
        "Authorization": authHeader,
      },
      cache: "no-store",
    });

    const data = await coreRes.json();

    if (!coreRes.ok) {
      return NextResponse.json({ error: data.error || "Token inválido" }, { status: coreRes.status });
    }

    return NextResponse.json(data);

  } catch (e: any) {
    return NextResponse.json({ error: e.message }, { status: 500 });
  }
}

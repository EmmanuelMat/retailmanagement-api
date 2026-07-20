import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function POST(req: NextRequest) {
  try {
    const body = await req.json();
    const { email, password, rnc } = body;

    if (!email || !password) {
      return NextResponse.json({ error: "Email y contraseña requeridos" }, { status: 400 });
    }

    const coreRes = await fetch(`${CORE_HTTP}/v1/auth/login`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ email: email.toLowerCase().trim(), password, rnc: rnc?.replace(/-/g, "").trim() }),
      cache: "no-store",
    });

    const data = await coreRes.json();

    if (!coreRes.ok) {
      return NextResponse.json({ error: data.error || "Credenciales inválidas" }, { status: coreRes.status });
    }

    // Retorna token + usuario + tenant
    return NextResponse.json({
      success: true,
      mensaje: data.mensaje || "Sesión iniciada",
      token: data.token,
      usuario: data.usuario,
      tenant: data.tenant,
    });

  } catch (e: any) {
    console.error("API /api/auth/login error", e);
    return NextResponse.json({ error: e.message || "Error interno" }, { status: 500 });
  }
}

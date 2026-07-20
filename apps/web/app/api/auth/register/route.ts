import { NextRequest, NextResponse } from "next/server";

const CORE_HTTP = process.env.CORE_HTTP_URL || "http://localhost:3001";

export async function POST(req: NextRequest) {
  try {
    const body = await req.json();
    const { rnc, razon_social, direccion, telefono, correo, admin_nombre, admin_email, admin_password } = body;

    // Validaciones básicas en español
    if (!rnc || !razon_social || !direccion || !admin_nombre || !admin_email || !admin_password) {
      return NextResponse.json({ error: "Faltan campos obligatorios: RNC, razón social, dirección, admin_nombre, admin_email, admin_password" }, { status: 400 });
    }

    const rncClean = rnc.replace(/-/g, "").trim();
    if (rncClean.length < 9 || rncClean.length > 11 || !/^\d+$/.test(rncClean)) {
      return NextResponse.json({ error: "RNC inválido: debe ser 9-11 dígitos numéricos" }, { status: 400 });
    }

    // Llama al núcleo Rust
    const coreRes = await fetch(`${CORE_HTTP}/v1/auth/register`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        rnc: rncClean,
        razon_social,
        direccion,
        telefono,
        correo,
        admin_nombre,
        admin_email,
        admin_password,
      }),
      cache: "no-store",
    });

    const data = await coreRes.json();
    
    if (!coreRes.ok) {
      return NextResponse.json({ error: data.error || data.mensaje || "Error registro en núcleo Rust" }, { status: coreRes.status });
    }

    // Respuesta exitosa con token
    return NextResponse.json({
      success: true,
      mensaje: data.mensaje || "Negocio registrado exitosamente",
      token: data.token,
      usuario: data.usuario,
      tenant: data.tenant,
    }, { status: 201 });

  } catch (e: any) {
    console.error("API /api/auth/register error", e);
    return NextResponse.json({ error: e.message || "Error interno" }, { status: 500 });
  }
}

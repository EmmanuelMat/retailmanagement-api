"use client";
import { useState } from "react";
import Link from "next/link";

export default function RegistroPage() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState<any>(null);

  async function handleRegister(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setLoading(true);
    setError("");
    setSuccess(null);
    
    const form = new FormData(e.currentTarget);
    const data = {
      rnc: (form.get("rnc") as string).replace(/-/g, "").trim(),
      razon_social: form.get("razon_social") as string,
      direccion: form.get("direccion") as string,
      telefono: form.get("telefono") as string,
      correo: form.get("correo") as string,
      admin_nombre: form.get("admin_nombre") as string,
      admin_email: form.get("admin_email") as string,
      admin_password: form.get("admin_password") as string,
    };

    // Validación RNC Dominicana
    if (data.rnc.length < 9 || data.rnc.length > 11 || !/^\d+$/.test(data.rnc)) {
      setError("RNC inválido: debe ser 9-11 dígitos numéricos sin guiones");
      setLoading(false);
      return;
    }

    try {
      const res = await fetch("/api/auth/register", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(data),
      });
      const result = await res.json();
      if (!res.ok) throw new Error(result.error || "Error registro");

      setSuccess(result);
      localStorage.setItem("token", result.token);
      localStorage.setItem("usuario", JSON.stringify(result.usuario));
      localStorage.setItem("tenant", JSON.stringify(result.tenant));
      document.cookie = `token=${result.token}; path=/; max-age=43200`;

      setTimeout(() => {
        window.location.href = "/pos";
      }, 1500);
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen bg-[#0c0a09] flex items-center justify-center p-6 relative overflow-hidden">
      <div className="absolute inset-0 pointer-events-none">
        <div className="absolute -top-[20%] -right-[10%] w-[60%] h-[60%] rounded-full bg-gradient-to-br from-sky-500/[0.10] to-blue-600/[0.03] blur-[100px]" />
      </div>

      <div className="relative w-full max-w-[560px]">
        <div className="rounded-[24px] border-[4px] border-black bg-white text-black p-8 shadow-[12px_12px_0px_black]">
          <div className="flex items-center gap-3 mb-6">
            <div className="h-12 w-12 rounded-[14px] bg-emerald-400 border-[3px] border-black flex items-center justify-center text-2xl shadow-[3px_3px_0px_black]">🏪</div>
            <div>
              <h1 className="font-black text-[20px] tracking-tight leading-none">REGISTRAR MI NEGOCIO • RNC</h1>
              <p className="text-[10px] font-bold bg-black text-white inline-block px-2 py-0.5 rounded-full mt-1">MÓDULO 1 • AUTH + MULTI-TENANCY</p>
            </div>
          </div>

          <h2 className="font-black text-[18px] tracking-tight">Crea tu tenant • RNC = tenant_id aislado</h2>
          <p className="text-[11px] font-bold opacity-60 mt-1">Cada RNC es un tenant aislado con su propio ledger TigerBeetle, usuarios, productos, ventas. Eventos TenantRegistrado + UsuarioCreado en EventStore hash-encadenado.</p>

          <form onSubmit={handleRegister} className="mt-6 space-y-4">
            <div className="grid grid-cols-2 gap-3">
              <div className="col-span-2">
                <label className="text-[11px] font-black tracking-widest">RNC • 9-11 DÍGITOS (SIN GUIONES) *</label>
                <input name="rnc" required placeholder="130793752" pattern="\d{9,11}" className="mt-1 w-full bg-amber-50 border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[14px] font-black tracking-wider focus:outline-none focus:border-amber-500" />
                <p className="text-[9px] font-mono opacity-50 mt-1">Este será tu tenant_id. Validación: solo números, 9-11 dígitos. Se creará cuenta TigerBeetle base.</p>
              </div>
              <div className="col-span-2">
                <label className="text-[11px] font-black tracking-widest">RAZÓN SOCIAL *</label>
                <input name="razon_social" required placeholder="COLMADO EL SOL SRL" className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold focus:outline-none focus:border-amber-400" />
              </div>
              <div className="col-span-2">
                <label className="text-[11px] font-black tracking-widest">DIRECCIÓN *</label>
                <input name="direccion" required placeholder="Av Duarte #123, Villa Consuelo, SDO" className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold" />
              </div>
              <div>
                <label className="text-[11px] font-black tracking-widest">TELÉFONO</label>
                <input name="telefono" placeholder="809-555-0101" className="mt-1 w-full bg-[#fafaf9] border-[2px] border-black rounded-[12px] px-4 py-2.5 text-[12px] font-bold" />
              </div>
              <div>
                <label className="text-[11px] font-black tracking-widest">CORREO NEGOCIO</label>
                <input name="correo" type="email" placeholder="info@colmadoelsol.do" className="mt-1 w-full bg-[#fafaf9] border-[2px] border-black rounded-[12px] px-4 py-2.5 text-[12px] font-bold" />
              </div>
            </div>

            <div className="border-t-[3px] border-black border-dashed pt-4">
              <h3 className="font-black text-[13px]">👤 USUARIO ADMIN INICIAL • Dueño</h3>
              <p className="text-[10px] opacity-60 mt-1">Este usuario tendrá rol ADMIN (acceso total). Luego podrás crear CAJERO, ALMACÉN, CONTADOR en Configuración → Usuarios.</p>
              <div className="grid grid-cols-2 gap-3 mt-3">
                <div className="col-span-2">
                  <label className="text-[11px] font-black tracking-widest">NOMBRE COMPLETO ADMIN *</label>
                  <input name="admin_nombre" required placeholder="Emmanuel Rosario" className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold" />
                </div>
                <div>
                  <label className="text-[11px] font-black tracking-widest">EMAIL ADMIN *</label>
                  <input name="admin_email" type="email" required placeholder="emmanuel@colmado.com" className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold" />
                </div>
                <div>
                  <label className="text-[11px] font-black tracking-widest">CONTRASEÑA ADMIN *</label>
                  <input name="admin_password" type="password" required placeholder="•••••••• (min 8)" minLength={8} className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold" />
                </div>
              </div>
            </div>

            {error && (
              <div className="rounded-[12px] border-[3px] border-red-500 bg-red-50 text-red-700 p-3 text-[11px] font-black">
                ❌ {error}
              </div>
            )}

            {success && (
              <div className="rounded-[12px] border-[3px] border-emerald-500 bg-emerald-50 text-emerald-800 p-3 text-[11px] font-black">
                ✅ {success.mensaje} • RNC {success.tenant?.rnc} • Usuario {success.usuario?.email} • Redirigiendo a POS...
              </div>
            )}

            <button disabled={loading} className="w-full bg-black text-white border-[3px] border-black rounded-full py-3.5 font-black text-[13px] tracking-wider shadow-[4px_4px_0px_black] hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[6px_6px_0px_black] transition-all disabled:opacity-50">
              {loading ? "Creando Tenant • TigerBeetle + EventStore + Argon2..." : "Registrar Negocio • Crear Tenant + Admin + Ledger Base"}
            </button>

            <div className="text-center">
              <Link href="/login" className="text-[11px] font-black underline hover:bg-amber-400 px-1">¿Ya tienes negocio? Iniciar Sesión</Link>
            </div>
          </form>

          <div className="mt-6 rounded-[12px] border-2 border-dashed border-black/20 bg-stone-50 p-3">
            <p className="font-black text-[10px]">🏦 LO QUE PASA AL REGISTRAR:</p>
            <ol className="text-[10px] mt-2 space-y-1 list-decimal list-inside leading-tight">
              <li>Valida RNC 9-11 dígitos, no existe</li>
              <li>Transacción Postgres: INSERT tenants + INSERT usuarios (password Argon2 hash) + 2 eventos en events (TenantRegistrado, UsuarioCreado) + hash encadenado</li>
              <li>Crea cuentas TigerBeetle base: asset:caja, banco, inventario, anticipos, liability:itbis, etc</li>
              <li>Genera JWT 12h con tenant_id=RNC + rol=ADMIN</li>
              <li>Redirige a POS con token en localStorage y cookie</li>
            </ol>
          </div>
        </div>
      </div>
    </div>
  );
}

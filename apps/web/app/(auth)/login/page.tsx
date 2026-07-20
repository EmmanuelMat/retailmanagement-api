"use client";
import { useState } from "react";
import Link from "next/link";

export default function LoginPage() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function handleLogin(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setLoading(true);
    setError("");
    const form = new FormData(e.currentTarget);
    const email = form.get("email") as string;
    const password = form.get("password") as string;
    const rnc = form.get("rnc") as string;

    try {
      const res = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password, rnc }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Credenciales inválidas");
      
      // Guarda token en localStorage y cookie (para middleware)
      localStorage.setItem("token", data.token);
      localStorage.setItem("usuario", JSON.stringify(data.usuario));
      localStorage.setItem("tenant", JSON.stringify(data.tenant));
      document.cookie = `token=${data.token}; path=/; max-age=43200`; // 12h
      
      window.location.href = "/pos";
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen bg-[#0c0a09] flex items-center justify-center p-6 relative overflow-hidden">
      <div className="absolute inset-0 pointer-events-none">
        <div className="absolute -top-[30%] -left-[10%] w-[60%] h-[60%] rounded-full bg-gradient-to-br from-amber-500/[0.12] to-orange-600/[0.03] blur-[100px]" />
        <div className="absolute -bottom-[20%] -right-[10%] w-[50%] h-[50%] rounded-full bg-gradient-to-br from-emerald-500/[0.08] to-teal-600/[0.02] blur-[80px]" />
      </div>

      <div className="relative w-full max-w-[420px]">
        <div className="rounded-[24px] border-[4px] border-black bg-white text-black p-8 shadow-[12px_12px_0px_black]">
          <div className="flex items-center gap-3 mb-6">
            <div className="h-12 w-12 rounded-[14px] bg-amber-400 border-[3px] border-black flex items-center justify-center text-2xl shadow-[3px_3px_0px_black]">☀️</div>
            <div>
              <h1 className="font-black text-[20px] tracking-tight leading-none">COLMADO EL SOL</h1>
              <p className="text-[10px] font-bold bg-black text-white inline-block px-2 py-0.5 rounded-full mt-1">POS • NÚCLEO BANCARIO</p>
            </div>
          </div>

          <h2 className="font-black text-[22px] tracking-tight">Iniciar Sesión • Bienvenido</h2>
          <p className="text-[12px] font-bold opacity-60 mt-1">Acceso seguro con JWT • Multi-tenancy por RNC • Roles: ADMIN, CAJERO, ALMACÉN, CONTADOR</p>

          <form onSubmit={handleLogin} className="mt-6 space-y-4">
            <div>
              <label className="text-[11px] font-black tracking-widest">RNC EMPRESA (Opcional si email único)</label>
              <input name="rnc" placeholder="130793752" className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold focus:outline-none focus:border-amber-400" />
              <p className="text-[9px] font-mono opacity-50 mt-1">Si tienes mismo email en varios negocios, coloca RNC para identificar tenant</p>
            </div>
            <div>
              <label className="text-[11px] font-black tracking-widest">Correo Electrónico</label>
              <input name="email" type="email" required placeholder="emmanuel@colmado.com" className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold focus:outline-none focus:border-amber-400" />
            </div>
            <div>
              <label className="text-[11px] font-black tracking-widest">Contraseña</label>
              <input name="password" type="password" required placeholder="••••••••" className="mt-1 w-full bg-[#fafaf9] border-[2.5px] border-black rounded-[12px] px-4 py-3 text-[13px] font-bold focus:outline-none focus:border-amber-400" />
            </div>

            {error && (
              <div className="rounded-[12px] border-2 border-red-500 bg-red-50 text-red-700 p-3 text-[11px] font-bold">
                ❌ {error}
              </div>
            )}

            <button disabled={loading} className="w-full bg-black text-white border-[3px] border-black rounded-full py-3.5 font-black text-[13px] tracking-wider shadow-[4px_4px_0px_black] hover:translate-x-[-2px] hover:translate-y-[-2px] hover:shadow-[6px_6px_0px_black] transition-all disabled:opacity-50">
              {loading ? "Verificando • TigerBeetle + JWT..." : "Entrar • RNC = tenant_id"}
            </button>

            <div className="flex justify-between text-[11px] font-bold">
              <Link href="/registro" className="underline hover:bg-amber-400 px-1">¿No tienes negocio? Registrar RNC</Link>
              <span className="opacity-50">¿Olvidaste clave?</span>
            </div>
          </form>

          <div className="mt-6 rounded-[12px] border-2 border-dashed border-black/20 bg-amber-50 p-3">
            <p className="font-black text-[10px]">🔐 SEGURIDAD BANCO-GRADO</p>
            <p className="text-[10px] mt-1 leading-tight">Password Argon2 + JWT 12h con tenant_id=RNC + rol • Eventos SesionIniciada en ledger hash-encadenado • Multi-tenancy aislado por RNC</p>
          </div>
        </div>

        <p className="text-center text-[10px] font-mono text-stone-500 mt-4">Hecho en 🇩🇴 SDO • EventStore Postgres + TigerBeetle • DGII e-CF • Español 100%</p>
      </div>
    </div>
  );
}

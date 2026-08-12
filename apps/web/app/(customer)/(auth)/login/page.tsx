"use client";
import { useEffect, useState } from "react";
import Link from "next/link";
import { Store, ShieldCheck, Zap, BookOpen } from "lucide-react";
import { Button, Card, Input, Label } from "@repo/ui";

function getCookie(name: string): string {
  const match = document.cookie.match(new RegExp(`(?:^|; )${name}=([^;]*)`));
  return match ? decodeURIComponent(match[1]) : "";
}

export default function LoginPage() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [rnc, setRnc] = useState("");

  // Restaura el RNC del último login exitoso — el resto del negocio (nombre
  // de usuario, contraseña) sí cambia entre sesiones, pero el RNC del
  // tenant casi nunca cambia y retiparlo cada vez es fricción pura.
  useEffect(() => {
    const saved = getCookie("rnc");
    if (saved) setRnc(saved);
  }, []);

  async function handleLogin(e: React.FormEvent<HTMLFormElement>) {
    e.preventDefault();
    setLoading(true);
    setError("");
    const form = new FormData(e.currentTarget);
    const email = form.get("email") as string;
    const password = form.get("password") as string;
    const rncValue = (form.get("rnc") as string).trim();

    try {
      const res = await fetch("/api/auth/login", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email, password, rnc: rncValue }),
      });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || "Credenciales inválidas");

      // Guarda token en localStorage y cookie (para middleware)
      localStorage.setItem("token", data.token);
      localStorage.setItem("usuario", JSON.stringify(data.usuario));
      localStorage.setItem("tenant", JSON.stringify(data.tenant));
      document.cookie = `token=${data.token}; path=/; max-age=43200`; // 12h
      if (rncValue) {
        document.cookie = `rnc=${encodeURIComponent(rncValue)}; path=/; max-age=31536000`; // 1 año
      }

      window.location.href = "/pos";
    } catch (err: any) {
      setError(err.message);
    } finally {
      setLoading(false);
    }
  }

  return (
    <div className="min-h-screen grid grid-cols-1 lg:grid-cols-2">
      {/* Brand panel */}
      <div className="hidden lg:flex flex-col justify-between bg-primary text-primary-foreground p-12 relative overflow-hidden">
        <div className="absolute inset-0 bg-grid-dots opacity-10" />
        <Link href="/" className="relative flex items-center gap-2.5">
          <div className="h-9 w-9 rounded-md bg-primary-foreground/15 flex items-center justify-center">
            <Store className="h-4.5 w-4.5" />
          </div>
          <span className="font-serif font-semibold text-[17px]">Colmado POS</span>
        </Link>
        <div className="relative max-w-md">
          <h2 className="text-3xl font-bold leading-tight">Todo tu colmado, en un solo lugar.</h2>
          <p className="mt-4 text-primary-foreground/80 leading-relaxed">
            POS, facturación e-CF, inventario, contabilidad y nómina — todo en un mismo sistema.
          </p>
          <ul className="mt-8 space-y-4">
            <li className="flex items-center gap-3 text-sm">
              <ShieldCheck className="h-4.5 w-4.5 shrink-0" /> Cumple Ley 32-23 DGII
            </li>
            <li className="flex items-center gap-3 text-sm">
              <Zap className="h-4.5 w-4.5 shrink-0" /> Cobra y factura en segundos
            </li>
            <li className="flex items-center gap-3 text-sm">
              <BookOpen className="h-4.5 w-4.5 shrink-0" /> Contabilidad automática
            </li>
          </ul>
        </div>
        <p className="relative text-xs text-primary-foreground/60">Hecho en 🇩🇴 Santo Domingo</p>
      </div>

      {/* Form panel */}
      <div className="flex items-center justify-center p-6 bg-muted/30 lg:bg-background">
        <div className="w-full max-w-[420px]">
          <div className="lg:hidden flex items-center gap-3 mb-6">
            <div className="h-10 w-10 rounded-md bg-primary text-primary-foreground flex items-center justify-center">
              <Store className="h-5 w-5" />
            </div>
            <div>
              <h1 className="font-bold text-[17px] leading-none">Colmado POS</h1>
              <p className="text-xs text-muted-foreground mt-1">Sistema de gestión para PYMES Dominicanas</p>
            </div>
          </div>

          <Card className="p-8">
            <h2 className="font-bold text-2xl font-serif">Iniciar sesión</h2>
            <p className="text-sm text-muted-foreground mt-1.5">Acceso por RNC (tenant) y rol: Admin, Cajero, Almacén, Contador.</p>

            <form onSubmit={handleLogin} className="mt-7 space-y-4">
              <div className="space-y-1.5">
                <Label htmlFor="rnc">RNC de la empresa (opcional)</Label>
                <Input id="rnc" name="rnc" value={rnc} onChange={(e) => setRnc(e.target.value)} placeholder="130793752" />
                <p className="text-xs text-muted-foreground">Solo necesario si usas el mismo correo en varios negocios.</p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="email">Correo electrónico</Label>
                <Input id="email" name="email" type="email" required placeholder="emmanuel@colmado.com" />
              </div>
              <div className="space-y-1.5">
                <div className="flex items-center justify-between">
                  <Label htmlFor="password">Contraseña</Label>
                  <Link href="/olvide-password" className="text-xs text-primary hover:underline">¿Olvidaste tu contraseña?</Link>
                </div>
                <Input id="password" name="password" type="password" required placeholder="••••••••" />
              </div>

              {error && (
                <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">
                  {error}
                </div>
              )}

              <Button type="submit" disabled={loading} className="w-full" size="lg">
                {loading ? "Verificando..." : "Entrar"}
              </Button>

              <div className="flex justify-between text-sm">
                <Link href="/registro" className="text-primary hover:underline">¿No tienes negocio? Registrar RNC</Link>
              </div>
            </form>
          </Card>

          <p className="text-center text-xs text-muted-foreground mt-4">Hecho en 🇩🇴 · DGII e-CF · Español 100%</p>
        </div>
      </div>
    </div>
  );
}

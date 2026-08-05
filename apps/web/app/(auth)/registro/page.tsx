"use client";
import { useState } from "react";
import Link from "next/link";
import { Store } from "lucide-react";
import { Button, Card, Input, Label } from "@repo/ui";

export default function RegistroPage() {
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [success, setSuccess] = useState<any>(null);
  const [facturaElectronica, setFacturaElectronica] = useState(true);

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
      factura_electronica_activa: facturaElectronica,
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
    <div className="min-h-screen bg-hero-glow flex items-center justify-center p-6 py-12">
      <div className="w-full max-w-[560px]">
        <Link href="/" className="flex items-center gap-2.5 mb-6 justify-center">
          <div className="h-8 w-8 rounded-md bg-primary text-primary-foreground flex items-center justify-center">
            <Store className="h-4 w-4" />
          </div>
          <span className="font-serif font-semibold text-[15px]">Colmado POS</span>
        </Link>

        <Card className="p-8">
          <div className="mb-6">
            <h1 className="font-bold text-2xl font-serif">Registrar mi negocio</h1>
            <p className="text-sm text-muted-foreground mt-1.5">Cada RNC es un negocio (tenant) aislado con sus propios usuarios y datos.</p>
          </div>

          <form onSubmit={handleRegister} className="mt-2 space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="col-span-2 space-y-1.5">
                <Label htmlFor="rnc">RNC o Cédula · 9-11 dígitos, sin guiones *</Label>
                <Input id="rnc" name="rnc" required placeholder="130793752" pattern="\d{9,11}" />
              </div>
              <div className="col-span-2 space-y-1.5">
                <Label>¿Tu negocio factura electrónicamente con la DGII? *</Label>
                <div className="grid grid-cols-2 gap-2">
                  <button
                    type="button"
                    onClick={() => setFacturaElectronica(true)}
                    className={`h-10 rounded-md border text-sm font-medium transition-colors ${
                      facturaElectronica ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:bg-muted"
                    }`}
                  >
                    Sí
                  </button>
                  <button
                    type="button"
                    onClick={() => setFacturaElectronica(false)}
                    className={`h-10 rounded-md border text-sm font-medium transition-colors ${
                      !facturaElectronica ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:bg-muted"
                    }`}
                  >
                    No, aún no
                  </button>
                </div>
                <p className="text-xs text-muted-foreground">
                  Si respondes "No", el sistema funciona igual pero sin intentar emitir e-CF — puedes activarlo cuando quieras desde Configuración.
                </p>
              </div>
              <div className="col-span-2 space-y-1.5">
                <Label htmlFor="razon_social">Razón social *</Label>
                <Input id="razon_social" name="razon_social" required placeholder="COLMADO EL SOL SRL" />
              </div>
              <div className="col-span-2 space-y-1.5">
                <Label htmlFor="direccion">Dirección *</Label>
                <Input id="direccion" name="direccion" required placeholder="Av Duarte #123, Villa Consuelo, SDO" />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="telefono">Teléfono</Label>
                <Input id="telefono" name="telefono" placeholder="809-555-0101" />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="correo">Correo del negocio</Label>
                <Input id="correo" name="correo" type="email" placeholder="info@colmadoelsol.do" />
              </div>
            </div>

            <div className="border-t border-border pt-4">
              <h3 className="font-medium text-sm">Usuario administrador inicial</h3>
              <p className="text-xs text-muted-foreground mt-1">Tendrá rol ADMIN (acceso total). Luego puedes crear Cajero, Almacén y Contador en Configuración → Usuarios.</p>
              <div className="grid grid-cols-2 gap-4 mt-3">
                <div className="col-span-2 space-y-1.5">
                  <Label htmlFor="admin_nombre">Nombre completo *</Label>
                  <Input id="admin_nombre" name="admin_nombre" required placeholder="Emmanuel Rosario" />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="admin_email">Email *</Label>
                  <Input id="admin_email" name="admin_email" type="email" required placeholder="emmanuel@colmado.com" />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="admin_password">Contraseña *</Label>
                  <Input id="admin_password" name="admin_password" type="password" required placeholder="mínimo 8 caracteres" minLength={8} />
                </div>
              </div>
            </div>

            {error && (
              <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>
            )}

            {success && (
              <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">
                {success.mensaje} · RNC {success.tenant?.rnc} · Redirigiendo...
              </div>
            )}

            <Button type="submit" disabled={loading} className="w-full" size="lg">
              {loading ? "Creando negocio..." : "Registrar negocio"}
            </Button>

            <div className="text-center">
              <Link href="/login" className="text-sm text-primary hover:underline">¿Ya tienes negocio? Iniciar sesión</Link>
            </div>
          </form>
        </Card>
      </div>
    </div>
  );
}

"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { ArrowLeft } from "lucide-react";
import { Button, Card, CardContent, Input, Label } from "@repo/ui";
import { apiFetch } from "@/lib/staff-api";

const EMPTY = {
  rnc: "",
  razon_social: "",
  direccion: "",
  telefono: "",
  correo: "",
  admin_nombre: "",
  factura_electronica_activa: true,
};

export default function NuevoTenantPage() {
  const router = useRouter();
  const [values, setValues] = useState(EMPTY);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  function set<K extends keyof typeof EMPTY>(key: K, value: (typeof EMPTY)[K]) {
    setValues((v) => ({ ...v, [key]: value }));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/tenants", {
        method: "POST",
        body: JSON.stringify({
          rnc: values.rnc.replace(/-/g, ""),
          razon_social: values.razon_social,
          direccion: values.direccion,
          telefono: values.telefono || undefined,
          correo: values.correo,
          admin_nombre: values.admin_nombre,
          factura_electronica_activa: values.factura_electronica_activa,
        }),
      });
      router.push(`/tenants/${values.rnc.replace(/-/g, "")}` as any);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <Button variant="ghost" size="sm" onClick={() => router.push("/tenants" as any)} className="mb-2 -ml-2">
          <ArrowLeft className="h-4 w-4" /> Negocios
        </Button>
        <h1 className="text-2xl font-bold tracking-tight">Onboarding de negocio</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Crea el negocio y su usuario ADMIN. El cliente recibe un correo para definir su propia contraseña — nunca
          se le comparte una por aquí. Los módulos se asignan aparte, en la ficha del negocio, después de hablar con
          el cliente.
        </p>
      </div>

      <Card className="max-w-xl">
        <CardContent className="pt-5">
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="rnc">RNC / Cédula *</Label>
                <Input id="rnc" required value={values.rnc} onChange={(e) => set("rnc", e.target.value)} placeholder="130793752" />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="telefono">Teléfono</Label>
                <Input id="telefono" value={values.telefono} onChange={(e) => set("telefono", e.target.value)} placeholder="809-555-0101" />
              </div>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="razon_social">Razón social *</Label>
              <Input id="razon_social" required value={values.razon_social} onChange={(e) => set("razon_social", e.target.value)} placeholder="Colmado El Sol SRL" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="direccion">Dirección *</Label>
              <Input id="direccion" required value={values.direccion} onChange={(e) => set("direccion", e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="correo">Correo del ADMIN *</Label>
              <Input id="correo" type="email" required value={values.correo} onChange={(e) => set("correo", e.target.value)} placeholder="dueño@negocio.com" />
              <p className="text-xs text-muted-foreground">Aquí llega el enlace para definir la contraseña.</p>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="admin_nombre">Nombre del ADMIN *</Label>
              <Input id="admin_nombre" required value={values.admin_nombre} onChange={(e) => set("admin_nombre", e.target.value)} />
            </div>
            <label className="flex items-center gap-2 text-sm">
              <input
                type="checkbox"
                checked={values.factura_electronica_activa}
                onChange={(e) => set("factura_electronica_activa", e.target.checked)}
                className="h-4 w-4 rounded border-border"
              />
              Factura electrónica (e-CF) activada
            </label>

            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

            <Button type="submit" disabled={saving}>{saving ? "Creando..." : "Crear negocio"}</Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Search } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface RncRecord {
  rnc: string;
  nombre: string;
  nombre_comercial: string | null;
  estado: string | null;
}

export interface ProveedorFormValues {
  nombre: string;
  rnc: string;
  telefono: string;
  email: string;
  direccion: string;
  contacto: string;
}

const EMPTY: ProveedorFormValues = { nombre: "", rnc: "", telefono: "", email: "", direccion: "", contacto: "" };

export function ProveedorForm({
  initial,
  submitLabel,
  onSubmit,
}: {
  initial?: Partial<ProveedorFormValues>;
  submitLabel: string;
  onSubmit: (values: ProveedorFormValues) => Promise<void>;
}) {
  const router = useRouter();
  const [values, setValues] = useState<ProveedorFormValues>({ ...EMPTY, ...initial });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [buscando, setBuscando] = useState(false);
  const [rncResult, setRncResult] = useState<RncRecord | null>(null);
  const [rncError, setRncError] = useState("");

  function set<K extends keyof ProveedorFormValues>(key: K, value: string) {
    setValues((v) => ({ ...v, [key]: value }));
    if (key === "rnc") {
      setRncResult(null);
      setRncError("");
    }
  }

  async function handleBuscarRnc() {
    if (!values.rnc.trim()) return;
    setBuscando(true);
    setRncError("");
    setRncResult(null);
    try {
      const data = await apiFetch<RncRecord>(`/api/rnc/${encodeURIComponent(values.rnc)}`);
      setRncResult(data);
      set("nombre", data.nombre_comercial || data.nombre);
    } catch (e: any) {
      setRncError(e.message);
    } finally {
      setBuscando(false);
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await onSubmit(values);
      router.push("/proveedores");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card className="max-w-xl">
      <CardContent className="pt-5">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-1.5">
            <Label htmlFor="nombre">Nombre / Razón social *</Label>
            <Input id="nombre" required value={values.nombre} onChange={(e) => set("nombre", e.target.value)} placeholder="Distribuidora Nacional SRL" />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="rnc">RNC</Label>
            <div className="flex gap-2">
              <Input id="rnc" value={values.rnc} onChange={(e) => set("rnc", e.target.value)} placeholder="130000001" />
              <Button type="button" variant="secondary" onClick={handleBuscarRnc} disabled={buscando || !values.rnc.trim()}>
                <Search className="h-4 w-4" />{buscando ? "Buscando..." : "Buscar en DGII"}
              </Button>
            </div>
            {rncResult && (
              <p className="text-xs text-muted-foreground flex items-center gap-1.5">
                Encontrado: {rncResult.nombre}
                <Badge variant={rncResult.estado === "ACTIVO" ? "success" : "secondary"}>{rncResult.estado || "—"}</Badge>
              </p>
            )}
            {rncError && <p className="text-xs text-muted-foreground">{rncError}</p>}
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="contacto">Persona de contacto</Label>
            <Input id="contacto" value={values.contacto} onChange={(e) => set("contacto", e.target.value)} />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="telefono">Teléfono</Label>
              <Input id="telefono" value={values.telefono} onChange={(e) => set("telefono", e.target.value)} placeholder="809-555-0101" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="email">Correo</Label>
              <Input id="email" type="email" value={values.email} onChange={(e) => set("email", e.target.value)} />
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="direccion">Dirección</Label>
            <Input id="direccion" value={values.direccion} onChange={(e) => set("direccion", e.target.value)} />
          </div>

          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

          <div className="flex gap-3">
            <Button type="submit" disabled={saving}>{saving ? "Guardando..." : submitLabel}</Button>
            <Button type="button" variant="secondary" onClick={() => router.push("/proveedores")}>Cancelar</Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}

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

export interface ClienteFormValues {
  nombre: string;
  rnc_cedula: string;
  telefono: string;
  email: string;
  direccion: string;
  limite_credito: string;
}

const EMPTY: ClienteFormValues = { nombre: "", rnc_cedula: "", telefono: "", email: "", direccion: "", limite_credito: "0" };

export function ClienteForm({
  initial,
  submitLabel,
  onSubmit,
}: {
  initial?: Partial<ClienteFormValues>;
  submitLabel: string;
  onSubmit: (values: ClienteFormValues) => Promise<void>;
}) {
  const router = useRouter();
  const [values, setValues] = useState<ClienteFormValues>({ ...EMPTY, ...initial });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [buscando, setBuscando] = useState(false);
  const [rncResult, setRncResult] = useState<RncRecord | null>(null);
  const [rncError, setRncError] = useState("");

  function set<K extends keyof ClienteFormValues>(key: K, value: string) {
    setValues((v) => ({ ...v, [key]: value }));
    if (key === "rnc_cedula") {
      setRncResult(null);
      setRncError("");
    }
  }

  async function handleBuscarRnc() {
    if (!values.rnc_cedula.trim()) return;
    setBuscando(true);
    setRncError("");
    setRncResult(null);
    try {
      const data = await apiFetch<RncRecord>(`/api/rnc/${encodeURIComponent(values.rnc_cedula)}`);
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
      router.push("/clientes");
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
            <Label htmlFor="nombre">Nombre *</Label>
            <Input id="nombre" required value={values.nombre} onChange={(e) => set("nombre", e.target.value)} placeholder="Juan Pérez" />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="rnc">RNC / Cédula</Label>
            <div className="flex gap-2">
              <Input id="rnc" value={values.rnc_cedula} onChange={(e) => set("rnc_cedula", e.target.value)} placeholder="000000000 para consumidor final" />
              <Button type="button" variant="secondary" onClick={handleBuscarRnc} disabled={buscando || !values.rnc_cedula.trim()}>
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
          <div className="space-y-1.5">
            <Label htmlFor="limite_credito">Límite de crédito (fiado)</Label>
            <Input
              id="limite_credito"
              type="number"
              step="0.01"
              min="0"
              value={values.limite_credito}
              onChange={(e) => set("limite_credito", e.target.value)}
              placeholder="0.00"
            />
            <p className="text-xs text-muted-foreground">Máximo que este cliente puede deber a la vez en ventas fiadas. Déjalo en 0 si no vende fiado a este cliente.</p>
          </div>

          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

          <div className="flex gap-3">
            <Button type="submit" disabled={saving}>{saving ? "Guardando..." : submitLabel}</Button>
            <Button type="button" variant="secondary" onClick={() => router.push("/clientes")}>Cancelar</Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}

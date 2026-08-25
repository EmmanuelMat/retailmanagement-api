"use client";

import { useState } from "react";
import { useRouter } from "next/navigation";
import { Button, Card, CardContent, Input, Label } from "@repo/ui";

export interface EmpleadoFormValues {
  nombre: string;
  cedula: string;
  puesto: string;
  salario_mensual: string;
}

const EMPTY: EmpleadoFormValues = { nombre: "", cedula: "", puesto: "", salario_mensual: "" };

export function EmpleadoForm({
  initial,
  submitLabel,
  onSubmit,
}: {
  initial?: Partial<EmpleadoFormValues>;
  submitLabel: string;
  onSubmit: (values: EmpleadoFormValues) => Promise<void>;
}) {
  const router = useRouter();
  const [values, setValues] = useState<EmpleadoFormValues>({ ...EMPTY, ...initial });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  function set<K extends keyof EmpleadoFormValues>(key: K, value: string) {
    setValues((v) => ({ ...v, [key]: value }));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await onSubmit(values);
      router.push("/nomina/empleados");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card className="max-w-4xl">
      <CardContent className="pt-5">
        <form onSubmit={handleSubmit} className="space-y-5">
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-1.5 md:col-span-2">
              <Label htmlFor="nombre">Nombre completo *</Label>
              <Input id="nombre" required value={values.nombre} onChange={(e) => set("nombre", e.target.value)} placeholder="María Pérez" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="cedula">Cédula</Label>
              <Input id="cedula" value={values.cedula} onChange={(e) => set("cedula", e.target.value)} placeholder="001-1234567-8" />
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="puesto">Puesto</Label>
              <Input id="puesto" value={values.puesto} onChange={(e) => set("puesto", e.target.value)} placeholder="Cajera" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="salario">Salario mensual *</Label>
              <Input id="salario" type="number" step="0.01" required value={values.salario_mensual} onChange={(e) => set("salario_mensual", e.target.value)} placeholder="20000.00" />
            </div>
          </div>

          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

          <div className="flex gap-3">
            <Button type="submit" disabled={saving}>{saving ? "Guardando..." : submitLabel}</Button>
            <Button type="button" variant="secondary" onClick={() => router.push("/nomina/empleados")}>Cancelar</Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}

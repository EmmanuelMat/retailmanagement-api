"use client";

import { useEffect, useState } from "react";
import { ProveedorForm, ProveedorFormValues } from "../proveedor-form";
import { apiFetch } from "@/lib/api";

export default function NuevoProveedorPage() {
  // Si venimos del Asistente con una acción confirmada, precarga el
  // formulario - la creación real sigue pasando por el mismo onSubmit/POST
  // de siempre, la IA nunca crea nada directamente.
  const [initial, setInitial] = useState<Partial<ProveedorFormValues> | undefined>(undefined);

  useEffect(() => {
    const raw = sessionStorage.getItem("ai_prefill_proveedor");
    if (!raw) return;
    sessionStorage.removeItem("ai_prefill_proveedor");
    try {
      const parsed = JSON.parse(raw);
      setInitial(Object.fromEntries(Object.entries(parsed).map(([k, v]) => [k, String(v)])));
    } catch {}
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nuevo proveedor</h1>
        <p className="text-sm text-muted-foreground mt-1">RNC, contacto y datos para compras.</p>
      </div>
      <ProveedorForm
        key={initial ? "prefilled" : "empty"}
        initial={initial}
        submitLabel="Crear proveedor"
        onSubmit={async (values) => {
          await apiFetch("/api/proveedores", {
            method: "POST",
            body: JSON.stringify({
              nombre: values.nombre,
              rnc: values.rnc || undefined,
              telefono: values.telefono || undefined,
              email: values.email || undefined,
              direccion: values.direccion || undefined,
              contacto: values.contacto || undefined,
            }),
          });
        }}
      />
    </div>
  );
}

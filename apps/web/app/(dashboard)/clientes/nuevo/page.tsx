"use client";

import { useEffect, useState } from "react";
import { ClienteForm, ClienteFormValues } from "../cliente-form";
import { apiFetch } from "@/lib/api";

export default function NuevoClientePage() {
  // Si venimos del Asistente con una acción confirmada, precarga el
  // formulario - la creación real sigue pasando por el mismo onSubmit/POST
  // de siempre, la IA nunca crea nada directamente.
  const [initial, setInitial] = useState<Partial<ClienteFormValues> | undefined>(undefined);

  useEffect(() => {
    const raw = sessionStorage.getItem("ai_prefill_cliente");
    if (!raw) return;
    sessionStorage.removeItem("ai_prefill_cliente");
    try {
      const parsed = JSON.parse(raw);
      setInitial(Object.fromEntries(Object.entries(parsed).map(([k, v]) => [k, String(v)])));
    } catch {}
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nuevo cliente</h1>
        <p className="text-sm text-muted-foreground mt-1">Nombre, RNC/cédula y datos de contacto.</p>
      </div>
      <ClienteForm
        key={initial ? "prefilled" : "empty"}
        initial={initial}
        submitLabel="Crear cliente"
        onSubmit={async (values) => {
          await apiFetch("/api/clientes", {
            method: "POST",
            body: JSON.stringify({
              nombre: values.nombre,
              rnc_cedula: values.rnc_cedula || undefined,
              telefono: values.telefono || undefined,
              email: values.email || undefined,
              direccion: values.direccion || undefined,
            }),
          });
        }}
      />
    </div>
  );
}

"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { ClienteForm, ClienteFormValues } from "../cliente-form";
import { apiFetch } from "@/lib/api";

interface ClienteDto {
  id: string;
  nombre: string;
  rnc_cedula: string | null;
  telefono: string | null;
  email: string | null;
  direccion: string | null;
  limite_credito: string;
}

export default function EditarClientePage() {
  const params = useParams<{ id: string }>();
  const [initial, setInitial] = useState<Partial<ClienteFormValues> | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<ClienteDto>(`/api/clientes/${params.id}`)
      .then((c) =>
        setInitial({
          nombre: c.nombre,
          rnc_cedula: c.rnc_cedula || "",
          telefono: c.telefono || "",
          email: c.email || "",
          direccion: c.direccion || "",
          limite_credito: c.limite_credito,
        })
      )
      .catch((e) => setError(e.message));
  }, [params.id]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Editar cliente</h1>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>}
      {initial && (
        <ClienteForm
          initial={initial}
          submitLabel="Guardar cambios"
          onSubmit={async (values) => {
            await apiFetch(`/api/clientes/${params.id}`, {
              method: "PUT",
              body: JSON.stringify({
                nombre: values.nombre,
                rnc_cedula: values.rnc_cedula || undefined,
                telefono: values.telefono || undefined,
                email: values.email || undefined,
                direccion: values.direccion || undefined,
                limite_credito: values.limite_credito || undefined,
              }),
            });
          }}
        />
      )}
    </div>
  );
}

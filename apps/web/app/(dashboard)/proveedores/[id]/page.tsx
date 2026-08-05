"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { ProveedorForm, ProveedorFormValues } from "../proveedor-form";
import { apiFetch } from "@/lib/api";

interface ProveedorDto {
  id: string;
  nombre: string;
  rnc: string | null;
  telefono: string | null;
  email: string | null;
  direccion: string | null;
  contacto: string | null;
}

export default function EditarProveedorPage() {
  const params = useParams<{ id: string }>();
  const [initial, setInitial] = useState<Partial<ProveedorFormValues> | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<ProveedorDto>(`/api/proveedores/${params.id}`)
      .then((p) =>
        setInitial({
          nombre: p.nombre,
          rnc: p.rnc || "",
          telefono: p.telefono || "",
          email: p.email || "",
          direccion: p.direccion || "",
          contacto: p.contacto || "",
        })
      )
      .catch((e) => setError(e.message));
  }, [params.id]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Editar proveedor</h1>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>}
      {initial && (
        <ProveedorForm
          initial={initial}
          submitLabel="Guardar cambios"
          onSubmit={async (values) => {
            await apiFetch(`/api/proveedores/${params.id}`, {
              method: "PUT",
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
      )}
    </div>
  );
}

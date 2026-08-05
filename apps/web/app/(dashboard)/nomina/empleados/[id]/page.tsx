"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { EmpleadoForm, EmpleadoFormValues } from "../empleado-form";
import { apiFetch } from "@/lib/api";

interface EmpleadoDto {
  id: string;
  nombre: string;
  cedula: string | null;
  puesto: string | null;
  salario_mensual: string;
}

export default function EditarEmpleadoPage() {
  const params = useParams<{ id: string }>();
  const [initial, setInitial] = useState<Partial<EmpleadoFormValues> | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<EmpleadoDto>(`/api/empleados/${params.id}`)
      .then((e) =>
        setInitial({
          nombre: e.nombre,
          cedula: e.cedula || "",
          puesto: e.puesto || "",
          salario_mensual: e.salario_mensual,
        })
      )
      .catch((e) => setError(e.message));
  }, [params.id]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Editar empleado</h1>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>}
      {initial && (
        <EmpleadoForm
          initial={initial}
          submitLabel="Guardar cambios"
          onSubmit={async (values) => {
            await apiFetch(`/api/empleados/${params.id}`, {
              method: "PUT",
              body: JSON.stringify({
                nombre: values.nombre,
                cedula: values.cedula || undefined,
                puesto: values.puesto || undefined,
                salario_mensual: values.salario_mensual,
              }),
            });
          }}
        />
      )}
    </div>
  );
}

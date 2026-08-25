"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { UserCog, Calculator } from "lucide-react";
import { EmpleadoForm, EmpleadoFormValues } from "../empleado-form";
import { MandatoCalculadora } from "../mandato-calculadora";
import { apiFetch } from "@/lib/api";

interface EmpleadoDto {
  id: string;
  nombre: string;
  cedula: string | null;
  puesto: string | null;
  salario_mensual: string;
}

const TABS = [
  { key: "datos", label: "Datos", icon: UserCog },
  { key: "mandato", label: "Mandato", icon: Calculator },
] as const;

export default function EditarEmpleadoPage() {
  const params = useParams<{ id: string }>();
  const [initial, setInitial] = useState<Partial<EmpleadoFormValues> | null>(null);
  const [error, setError] = useState("");
  const [tab, setTab] = useState<(typeof TABS)[number]["key"]>("datos");

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
        <>
          <div className="flex gap-1 border-b border-border max-w-xl">
            {TABS.map((t) => {
              const Icon = t.icon;
              return (
                <button
                  key={t.key}
                  onClick={() => setTab(t.key)}
                  className={`flex items-center gap-1.5 px-3.5 py-2.5 text-sm font-medium border-b-2 -mb-px transition-colors ${
                    tab === t.key ? "border-primary text-primary" : "border-transparent text-muted-foreground hover:text-foreground"
                  }`}
                >
                  <Icon className="h-4 w-4" />
                  {t.label}
                </button>
              );
            })}
          </div>

          {tab === "datos" && (
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

          {tab === "mandato" && <MandatoCalculadora salarioMensual={initial.salario_mensual || "0"} />}
        </>
      )}
    </div>
  );
}

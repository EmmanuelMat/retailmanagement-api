"use client";

import { EmpleadoForm } from "../empleado-form";
import { apiFetch } from "@/lib/api";

export default function NuevoEmpleadoPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nuevo empleado</h1>
        <p className="text-sm text-muted-foreground mt-1">Nombre, puesto y salario mensual.</p>
      </div>
      <EmpleadoForm
        submitLabel="Crear empleado"
        onSubmit={async (values) => {
          await apiFetch("/api/empleados", {
            method: "POST",
            body: JSON.stringify({
              nombre: values.nombre,
              cedula: values.cedula || undefined,
              puesto: values.puesto || undefined,
              salario_mensual: values.salario_mensual,
            }),
          });
        }}
      />
    </div>
  );
}

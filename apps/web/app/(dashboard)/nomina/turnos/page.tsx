import { ComingSoon } from "@repo/ui";

export default function TurnosPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Turnos y asistencia</h1>
        <p className="text-sm text-muted-foreground mt-1">Entrada/salida y horas acumuladas por empleado.</p>
      </div>
      <ComingSoon title="Control de asistencia" description="La nómina actual corre sobre salario mensual fijo, sin horas variables por turno todavía." />
    </div>
  );
}

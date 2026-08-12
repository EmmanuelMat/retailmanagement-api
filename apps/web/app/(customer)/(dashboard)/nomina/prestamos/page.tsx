import { ComingSoon } from "@repo/ui";

export default function PrestamosPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Préstamos</h1>
        <p className="text-sm text-muted-foreground mt-1">Préstamos a empleados con cuotas fijas (distinto de un adelanto).</p>
      </div>
      <ComingSoon
        title="Préstamos con cuotas"
        description="Para necesidades inmediatas usa Nómina → Adelantos (50% del sueldo, sin cuotas)."
      />
    </div>
  );
}

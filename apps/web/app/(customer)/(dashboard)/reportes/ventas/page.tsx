import Link from "next/link";
import { ComingSoon } from "@repo/ui";

export default function ReporteVentasPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Reporte de ventas</h1>
        <p className="text-sm text-muted-foreground mt-1">Top productos, ticket promedio, horas pico.</p>
      </div>
      <ComingSoon
        title="Analítica de ventas"
        description={"El historial detallado ya está en Ventas — el desglose por producto/hora llega en una fase posterior."}
      />
      <Link href="/ventas" className="text-sm text-primary hover:underline">Ir a Ventas →</Link>
    </div>
  );
}

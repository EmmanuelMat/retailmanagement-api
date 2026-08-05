import Link from "next/link";
import { ComingSoon } from "@repo/ui";

export default function ReporteInventarioPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Reporte de inventario</h1>
        <p className="text-sm text-muted-foreground mt-1">Valorización, mermas y vencidos.</p>
      </div>
      <ComingSoon
        title="Analítica de inventario"
        description="El valor total y el kardex ya están en Inventario — mermas/vencidos llegan en una fase posterior."
      />
      <Link href="/inventario" className="text-sm text-primary hover:underline">Ir a Inventario →</Link>
    </div>
  );
}

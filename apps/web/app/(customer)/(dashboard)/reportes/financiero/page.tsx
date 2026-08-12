import Link from "next/link";
import { ComingSoon } from "@repo/ui";

export default function ReporteFinancieroPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Reporte financiero</h1>
        <p className="text-sm text-muted-foreground mt-1">Estado de resultados y balance general.</p>
      </div>
      <ComingSoon
        title="Estados financieros formales"
        description="Los saldos por cuenta ya están en Contabilidad → Libro mayor — el formato de estado de resultados llega en una fase posterior."
      />
      <Link href="/contabilidad/libro-mayor" className="text-sm text-primary hover:underline">Ir a Libro mayor →</Link>
    </div>
  );
}

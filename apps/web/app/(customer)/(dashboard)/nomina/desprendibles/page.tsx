import { ComingSoon } from "@repo/ui";

export default function DesprendiblesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Desprendibles de pago</h1>
        <p className="text-sm text-muted-foreground mt-1">Comprobante en PDF por empleado de cada corrida.</p>
      </div>
      <ComingSoon
        title="PDF de desprendibles"
        description="Los montos ya se calculan en cada corrida (Nómina → Correr nómina) — la exportación a PDF llega en una fase posterior."
      />
    </div>
  );
}

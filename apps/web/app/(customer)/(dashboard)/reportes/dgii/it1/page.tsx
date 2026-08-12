import { ComingSoon } from "@repo/ui";

export default function ReporteIt1Page() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">IT-1</h1>
        <p className="text-sm text-muted-foreground mt-1">Declaración jurada mensual de ITBIS.</p>
      </div>
      <ComingSoon title="Formato IT-1" description="El ITBIS cobrado y adelantado ya se calcula en Contabilidad → Libro mayor; el formato específico de la DGII llega después." />
    </div>
  );
}

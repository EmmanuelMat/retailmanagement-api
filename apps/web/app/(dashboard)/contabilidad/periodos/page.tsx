import { ComingSoon } from "@repo/ui";

export default function PeriodosPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Períodos contables</h1>
        <p className="text-sm text-muted-foreground mt-1">Cierre mensual para bloquear cambios en meses ya reportados.</p>
      </div>
      <ComingSoon title="Cierre de período" description="El libro mayor ya registra todo por fecha — el bloqueo formal de meses cerrados llega en una fase posterior." />
    </div>
  );
}

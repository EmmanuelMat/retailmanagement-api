import { ComingSoon } from "@repo/ui";

export default function InventarioFisicoPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Inventario físico</h1>
        <p className="text-sm text-muted-foreground mt-1">Conteo y comparación contra el sistema.</p>
      </div>
      <ComingSoon
        title="Conteo de inventario"
        description="Para ajustar stock manualmente mientras tanto, usa Inventario → Movimientos con tipo Ajuste."
      />
    </div>
  );
}

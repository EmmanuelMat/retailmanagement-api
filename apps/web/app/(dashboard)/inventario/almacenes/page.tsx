import { ComingSoon } from "@repo/ui";

export default function AlmacenesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Almacenes</h1>
        <p className="text-sm text-muted-foreground mt-1">Por ahora todo el inventario opera en un solo almacén.</p>
      </div>
      <ComingSoon
        title="Multi-almacén"
        description="Traslados y stock por ubicación llegan en una fase posterior — el kardex actual ya cubre entradas, salidas y ajustes."
      />
    </div>
  );
}

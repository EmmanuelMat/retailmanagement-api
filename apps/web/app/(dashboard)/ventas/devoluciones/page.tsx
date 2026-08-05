import { ComingSoon } from "@repo/ui";

export default function DevolucionesPage() {
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Devoluciones</h1>
        <p className="text-sm text-muted-foreground mt-1">Notas de crédito (e-CF tipo 34) sobre una venta original.</p>
      </div>
      <ComingSoon
        title="Notas de crédito"
        description="Requiere referenciar el e-NCF original — llega junto con la emisión fiscal completa en Configuración DGII."
      />
    </div>
  );
}

"use client";

import Link from "next/link";
import { Lock, FileBarChart, Package } from "lucide-react";
import { Card, CardContent, Button } from "@repo/ui";

export default function TrialExpiradoPage() {
  return (
    <div className="max-w-lg mx-auto py-10 space-y-6">
      <Card>
        <CardContent className="pt-6 text-center space-y-4">
          <div className="mx-auto h-12 w-12 rounded-full bg-warning/10 text-warning flex items-center justify-center">
            <Lock className="h-6 w-6" />
          </div>
          <div>
            <h1 className="text-xl font-bold font-serif">Tu período de prueba terminó</h1>
            <p className="text-sm text-muted-foreground mt-2">
              Ya no se pueden registrar ventas, compras u otros movimientos nuevos. Tus datos siguen
              guardados y disponibles para consulta y exportación — nada se pierde.
            </p>
          </div>
          <div className="rounded-md border border-border bg-muted/40 p-3.5 text-sm text-left">
            Contáctanos para activar tu licencia y seguir usando el sistema sin interrupciones.
          </div>
          <Button className="w-full" disabled title="Activación manual por el equipo de ventas">
            Activar licencia
          </Button>
        </CardContent>
      </Card>

      <div>
        <p className="text-xs font-medium text-muted-foreground mb-2 px-1">Mientras tanto, puedes seguir consultando</p>
        <div className="grid grid-cols-2 gap-2.5">
          <Link href="/ventas">
            <div className="flex flex-col items-center gap-2 rounded-lg border border-border p-4 text-center hover:border-primary/40 hover:bg-primary/5 transition-colors">
              <FileBarChart className="h-4 w-4 text-primary" />
              <span className="text-xs font-medium">Ver ventas</span>
            </div>
          </Link>
          <Link href="/inventario/productos">
            <div className="flex flex-col items-center gap-2 rounded-lg border border-border p-4 text-center hover:border-primary/40 hover:bg-primary/5 transition-colors">
              <Package className="h-4 w-4 text-primary" />
              <span className="text-xs font-medium">Ver inventario</span>
            </div>
          </Link>
        </div>
      </div>
    </div>
  );
}

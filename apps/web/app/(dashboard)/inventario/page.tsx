"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Package, AlertTriangle, DollarSign, ArrowRight } from "lucide-react";
import { Card, CardContent, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Resumen {
  total_productos: number;
  valor_total: string;
  bajo_minimo: number;
}

export default function InventarioPage() {
  const [resumen, setResumen] = useState<Resumen | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch<Resumen>("/api/inventario/resumen")
      .then(setResumen)
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Inventario</h1>
          <p className="text-sm text-muted-foreground mt-1">Stock valorizado, alertas de mínimo y movimientos.</p>
        </div>
        <Link href="/inventario/movimientos" className="text-sm text-primary hover:underline flex items-center gap-1">
          Ver kardex <ArrowRight className="h-4 w-4" />
        </Link>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <Card>
          <CardContent className="pt-5 flex items-start justify-between">
            <div>
              <p className="text-xs text-muted-foreground">Productos activos</p>
              <p className="text-2xl font-bold mt-1">{loading ? "…" : resumen?.total_productos ?? 0}</p>
            </div>
            <div className="h-9 w-9 rounded-md bg-primary/10 text-primary flex items-center justify-center">
              <Package className="h-4.5 w-4.5" />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5 flex items-start justify-between">
            <div>
              <p className="text-xs text-muted-foreground">Valor de inventario</p>
              <p className="text-2xl font-bold mt-1">{loading ? "…" : formatDOP(resumen?.valor_total ?? "0")}</p>
            </div>
            <div className="h-9 w-9 rounded-md bg-primary/10 text-primary flex items-center justify-center">
              <DollarSign className="h-4.5 w-4.5" />
            </div>
          </CardContent>
        </Card>
        <Card>
          <CardContent className="pt-5 flex items-start justify-between">
            <div>
              <p className="text-xs text-muted-foreground">Bajo stock mínimo</p>
              <p className="text-2xl font-bold mt-1">{loading ? "…" : resumen?.bajo_minimo ?? 0}</p>
            </div>
            <div className="h-9 w-9 rounded-md bg-destructive/100/10 text-destructive flex items-center justify-center">
              <AlertTriangle className="h-4.5 w-4.5" />
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

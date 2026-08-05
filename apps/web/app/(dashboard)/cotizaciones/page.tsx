"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { FileText, Plus } from "lucide-react";
import { Badge, Button, Card, CardContent, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Cotizacion {
  id: string;
  cliente_nombre: string | null;
  total: string;
  estado: string;
  fecha_vencimiento: string | null;
  created_at: string;
}

const ESTADO_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  PENDIENTE: "warning",
  ACEPTADA: "default",
  CONVERTIDA: "success",
  RECHAZADA: "destructive",
  VENCIDA: "secondary",
};

export default function CotizacionesPage() {
  const [cotizaciones, setCotizaciones] = useState<Cotizacion[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch<{ cotizaciones: Cotizacion[] }>("/api/cotizaciones").then((d) => setCotizaciones(d.cotizaciones)).finally(() => setLoading(false));
  }, []);

  return (
    <div className="space-y-6">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Cotizaciones</h1>
          <p className="text-sm text-muted-foreground mt-1">Propuestas sin compromiso — conviértelas en venta cuando el cliente acepte.</p>
        </div>
        <Link href={"/cotizaciones/nueva" as any}>
          <Button><Plus className="h-4 w-4" />Nueva cotización</Button>
        </Link>
      </div>

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : cotizaciones.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <FileText className="h-6 w-6" />
              Aún no hay cotizaciones.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Cliente</TableHead>
                  <TableHead>Vence</TableHead>
                  <TableHead>Estado</TableHead>
                  <TableHead className="text-right">Total</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {cotizaciones.map((c) => (
                  <TableRow key={c.id} className="cursor-pointer">
                    <TableCell className="text-xs text-muted-foreground">
                      <Link href={`/cotizaciones/${c.id}` as any} className="hover:text-primary">{new Date(c.created_at).toLocaleString("es-DO")}</Link>
                    </TableCell>
                    <TableCell>{c.cliente_nombre || "Consumidor final"}</TableCell>
                    <TableCell className="text-muted-foreground">{c.fecha_vencimiento ? new Date(c.fecha_vencimiento).toLocaleDateString("es-DO") : "—"}</TableCell>
                    <TableCell><Badge variant={ESTADO_VARIANT[c.estado] || "default"}>{c.estado}</Badge></TableCell>
                    <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(c.total)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

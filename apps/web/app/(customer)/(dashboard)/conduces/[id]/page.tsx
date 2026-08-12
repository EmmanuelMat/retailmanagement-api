"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { FileText } from "lucide-react";
import { Button, Card, CardContent, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface ConduceItem {
  id: string;
  sku: string;
  nombre: string;
  cantidad: string;
  unidad: string | null;
  observaciones: string | null;
}

interface ConduceDetalle {
  id: string;
  venta_id: string;
  direccion_entrega: string | null;
  orden_compra: string | null;
  vehiculo_placa: string | null;
  conductor: string | null;
  notas: string | null;
  entregado_por: string | null;
  recibido_por: string | null;
  created_at: string;
  items: ConduceItem[];
}

export default function ConduceDetallePage() {
  const params = useParams<{ id: string }>();
  const [conduce, setConduce] = useState<ConduceDetalle | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<ConduceDetalle>(`/api/conduces/${params.id}`).then(setConduce).catch((e) => setError(e.message));
  }, [params.id]);

  if (error) return <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>;
  if (!conduce) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <div className="space-y-6 max-w-2xl">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Entrega</h1>
          <p className="text-sm text-muted-foreground mt-1">{new Date(conduce.created_at).toLocaleString("es-DO")}</p>
          {conduce.direccion_entrega && <p className="text-sm text-muted-foreground">Dirección: {conduce.direccion_entrega}</p>}
        </div>
        <div className="flex items-center gap-3">
          <Link href={`/imprimir/conduce/${conduce.id}` as any} target="_blank">
            <Button size="sm" variant="secondary"><FileText className="h-3.5 w-3.5" />Imprimir</Button>
          </Link>
          <Link href={`/ventas/${conduce.venta_id}` as any} className="text-primary hover:underline text-sm font-medium">
            Ver venta
          </Link>
        </div>
      </div>

      {(conduce.orden_compra || conduce.vehiculo_placa || conduce.conductor) && (
        <Card>
          <CardContent className="pt-5 grid grid-cols-2 gap-3 text-sm">
            {conduce.orden_compra && <div><p className="text-xs text-muted-foreground">Orden de compra</p><p>{conduce.orden_compra}</p></div>}
            {conduce.vehiculo_placa && <div><p className="text-xs text-muted-foreground">Vehículo / Placa</p><p>{conduce.vehiculo_placa}</p></div>}
            {conduce.conductor && <div><p className="text-xs text-muted-foreground">Conductor</p><p>{conduce.conductor}</p></div>}
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>SKU</TableHead>
                <TableHead>Producto</TableHead>
                <TableHead className="text-right">Cantidad</TableHead>
                <TableHead>Unidad</TableHead>
                <TableHead>Observaciones</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {conduce.items.map((it) => (
                <TableRow key={it.id}>
                  <TableCell className="font-mono text-xs text-muted-foreground">{it.sku}</TableCell>
                  <TableCell className="font-medium">{it.nombre}</TableCell>
                  <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
                  <TableCell className="text-muted-foreground">{it.unidad || "—"}</TableCell>
                  <TableCell className="text-muted-foreground">{it.observaciones || "—"}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      {(conduce.notas || conduce.entregado_por || conduce.recibido_por) && (
        <Card>
          <CardContent className="pt-5 space-y-3 text-sm">
            {conduce.notas && <div><p className="text-xs text-muted-foreground">Notas de entrega</p><p>{conduce.notas}</p></div>}
            <div className="grid grid-cols-2 gap-3">
              {conduce.entregado_por && <div><p className="text-xs text-muted-foreground">Entregado por</p><p>{conduce.entregado_por}</p></div>}
              {conduce.recibido_por && <div><p className="text-xs text-muted-foreground">Recibido por</p><p>{conduce.recibido_por}</p></div>}
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

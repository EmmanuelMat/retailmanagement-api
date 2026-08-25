"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import { Badge, Button, Card, CardContent, Input, Label, Select, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { ESTADO_VARIANT } from "../page";

interface OrdenCompraItem {
  id: string;
  sku: string;
  nombre: string;
  cantidad_solicitada: string;
  cantidad_recibida: string;
  costo_unitario: string;
}

interface OrdenCompraDetalle {
  id: string;
  proveedor_id: string;
  estado: string;
  subtotal: string;
  total: string;
  fecha: string;
  fecha_esperada: string | null;
  notas: string | null;
  items: OrdenCompraItem[];
}

interface Proveedor {
  id: string;
  nombre: string;
}

const ESTADO_LABEL: Record<string, string> = {
  BORRADOR: "Borrador",
  ENVIADA: "Enviada",
  RECIBIDA_PARCIAL: "Recibida parcial",
  RECIBIDA: "Recibida",
  CANCELADA: "Cancelada",
};

export default function OrdenCompraDetallePage() {
  const params = useParams<{ id: string }>();
  const router = useRouter();
  const [orden, setOrden] = useState<OrdenCompraDetalle | null>(null);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [error, setError] = useState("");
  const [cantidades, setCantidades] = useState<Record<string, string>>({});
  const [metodoPago, setMetodoPago] = useState("EFECTIVO");
  const [fechaVencimiento, setFechaVencimiento] = useState("");
  const [saving, setSaving] = useState(false);

  function load() {
    apiFetch<OrdenCompraDetalle>(`/api/ordenes-compra/${params.id}`).then(setOrden).catch((e) => setError(e.message));
  }

  useEffect(() => {
    load();
    apiFetch<{ items: Proveedor[] }>("/api/proveedores?pageSize=1000&activo=true").then((d) => setProveedores(d.items)).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [params.id]);

  async function handleRecibir() {
    if (!orden) return;
    const items = orden.items
      .filter((it) => cantidades[it.id] && Number(cantidades[it.id]) > 0)
      .map((it) => ({ item_id: it.id, cantidad: cantidades[it.id] }));
    if (items.length === 0) {
      setError("Indica cuánto recibiste de al menos una línea.");
      return;
    }
    if (metodoPago === "FIADO" && !fechaVencimiento) {
      setError("Una compra fiada necesita fecha de vencimiento.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/ordenes-compra/${orden.id}/recibir`, {
        method: "POST",
        body: JSON.stringify({ items, metodo_pago: metodoPago, fecha_vencimiento: metodoPago === "FIADO" ? fechaVencimiento : undefined }),
      });
      setCantidades({});
      load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleCancelar() {
    if (!orden) return;
    try {
      await apiFetch(`/api/ordenes-compra/${orden.id}/cancelar`, { method: "POST" });
      load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  if (error && !orden) return <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>;
  if (!orden) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  const proveedorNombre = proveedores.find((p) => p.id === orden.proveedor_id)?.nombre;
  const puedeRecibir = orden.estado === "BORRADOR" || orden.estado === "ENVIADA" || orden.estado === "RECIBIDA_PARCIAL";
  const puedeCancelar = orden.estado !== "RECIBIDA" && orden.estado !== "CANCELADA";

  return (
    <div className="space-y-6 max-w-6xl">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Orden de Compra</h1>
          <p className="text-sm text-muted-foreground mt-1">{proveedorNombre || "—"} · {new Date(orden.fecha).toLocaleDateString("es-DO")}</p>
        </div>
        <div className="text-right space-y-2">
          <Badge variant={ESTADO_VARIANT[orden.estado] || "default"}>{ESTADO_LABEL[orden.estado] || orden.estado}</Badge>
          {puedeCancelar && <div><Button size="sm" variant="destructive" onClick={handleCancelar}>Cancelar orden</Button></div>}
        </div>
      </div>

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Producto</TableHead>
                <TableHead className="text-right">Solicitado</TableHead>
                <TableHead className="text-right">Recibido</TableHead>
                <TableHead className="text-right">Costo c/u</TableHead>
                {puedeRecibir && <TableHead className="text-right">Recibir ahora</TableHead>}
              </TableRow>
            </TableHeader>
            <TableBody>
              {orden.items.map((it) => {
                const pendiente = Number(it.cantidad_solicitada) - Number(it.cantidad_recibida);
                return (
                  <TableRow key={it.id}>
                    <TableCell className="font-medium">{it.nombre}</TableCell>
                    <TableCell className="text-right tabular-nums">{it.cantidad_solicitada}</TableCell>
                    <TableCell className="text-right tabular-nums">{it.cantidad_recibida}</TableCell>
                    <TableCell className="text-right tabular-nums">{formatDOP(it.costo_unitario)}</TableCell>
                    {puedeRecibir && (
                      <TableCell className="text-right">
                        {pendiente > 0 ? (
                          <Input
                            type="number" step="0.01" className="w-24 h-8 text-xs ml-auto" placeholder={`máx ${pendiente}`}
                            value={cantidades[it.id] || ""} onChange={(e) => setCantidades((c) => ({ ...c, [it.id]: e.target.value }))}
                          />
                        ) : (
                          <span className="text-xs text-muted-foreground">Completo</span>
                        )}
                      </TableCell>
                    )}
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-5 space-y-1 text-sm max-w-xs ml-auto">
          <div className="flex justify-between text-muted-foreground"><span>Subtotal estimado</span><span className="tabular-nums">{formatDOP(orden.subtotal)}</span></div>
          <div className="flex justify-between font-bold text-base pt-1 border-t border-border mt-1"><span>Total estimado</span><span className="tabular-nums">{formatDOP(orden.total)}</span></div>
        </CardContent>
      </Card>

      {puedeRecibir && (
        <Card>
          <CardContent className="pt-5 space-y-3 max-w-sm ml-auto">
            <p className="text-sm font-semibold">Recibir mercancía</p>
            <div className="grid grid-cols-2 gap-3">
              <div className="space-y-1.5">
                <Label htmlFor="metodo">Método de pago</Label>
                <Select id="metodo" value={metodoPago} onChange={(e) => setMetodoPago(e.target.value)}>
                  <option value="EFECTIVO">Efectivo</option>
                  <option value="TARJETA">Tarjeta</option>
                  <option value="TRANSFERENCIA">Transferencia</option>
                  <option value="FIADO">Fiado</option>
                </Select>
              </div>
              {metodoPago === "FIADO" && (
                <div className="space-y-1.5">
                  <Label htmlFor="vencimiento">Vence</Label>
                  <Input id="vencimiento" type="date" value={fechaVencimiento} onChange={(e) => setFechaVencimiento(e.target.value)} />
                </div>
              )}
            </div>
            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
            <Button className="w-full" disabled={saving} onClick={handleRecibir}>{saving ? "Procesando..." : "Recibir y crear compra"}</Button>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

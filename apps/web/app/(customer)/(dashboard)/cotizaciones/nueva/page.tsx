"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Plus, Trash2 } from "lucide-react";
import { Button, Card, CardContent, Input, Label, Select, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Producto {
  id: string;
  sku: string;
  nombre: string;
  itbis_tipo: string;
  precio_venta: string;
}

interface Cliente {
  id: string;
  nombre: string;
}

interface Linea {
  productoId: string;
  cantidad: string;
  descuento: string;
}

const ITBIS_RATE: Record<string, number> = { GRAVADO_18: 0.18, GRAVADO_16: 0.16, EXENTO: 0 };

export default function NuevaCotizacionPage() {
  const router = useRouter();
  const [productos, setProductos] = useState<Producto[]>([]);
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [clienteId, setClienteId] = useState("");
  const [fechaVencimiento, setFechaVencimiento] = useState("");
  const [lineas, setLineas] = useState<Linea[]>([{ productoId: "", cantidad: "", descuento: "" }]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<{ items: Producto[] }>("/api/productos?pageSize=5000&activo=true").then((d) => setProductos(d.items)).catch(() => {});
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
  }, []);

  function updateLinea(i: number, patch: Partial<Linea>) {
    setLineas((ls) => ls.map((l, idx) => (idx === i ? { ...l, ...patch } : l)));
  }

  function addLinea() {
    setLineas((ls) => [...ls, { productoId: "", cantidad: "", descuento: "" }]);
  }

  function removeLinea(i: number) {
    setLineas((ls) => ls.filter((_, idx) => idx !== i));
  }

  const total = lineas.reduce((sum, l) => {
    const prod = productos.find((p) => p.id === l.productoId);
    if (!prod) return sum;
    const bruto = (Number(l.cantidad) || 0) * Number(prod.precio_venta);
    const descuento = Math.min(Number(l.descuento) || 0, bruto);
    const sub = bruto - descuento;
    return sum + sub + sub * (ITBIS_RATE[prod.itbis_tipo] || 0);
  }, 0);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const items = lineas.filter((l) => l.productoId && l.cantidad);
    if (items.length === 0) {
      setError("Agrega al menos un producto con cantidad.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      const cotizacion = await apiFetch<{ id: string }>("/api/cotizaciones", {
        method: "POST",
        body: JSON.stringify({
          cliente_id: clienteId || undefined,
          fecha_vencimiento: fechaVencimiento || undefined,
          items: items.map((l) => ({
            producto_id: l.productoId,
            cantidad: l.cantidad,
            descuento: l.descuento || undefined,
          })),
        }),
      });
      router.push(`/cotizaciones/${cotizacion.id}` as any);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nueva cotización</h1>
        <p className="text-sm text-muted-foreground mt-1">No afecta stock ni caja — es solo una propuesta para el cliente.</p>
      </div>

      <Card>
        <CardContent className="pt-5">
          <form onSubmit={handleSubmit} className="space-y-5">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="cliente">Cliente</Label>
                <Select id="cliente" value={clienteId} onChange={(e) => setClienteId(e.target.value)}>
                  <option value="">Consumidor final</option>
                  {clientes.map((c) => <option key={c.id} value={c.id}>{c.nombre}</option>)}
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="vence">Válida hasta</Label>
                <Input id="vence" type="date" value={fechaVencimiento} onChange={(e) => setFechaVencimiento(e.target.value)} />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Productos</Label>
              {lineas.map((l, i) => (
                <div key={i} className="grid grid-cols-[1fr_90px_110px_32px] gap-2 items-end">
                  <Select value={l.productoId} onChange={(e) => updateLinea(i, { productoId: e.target.value })}>
                    <option value="">Producto…</option>
                    {productos.map((p) => <option key={p.id} value={p.id}>{p.sku} · {p.nombre} ({formatDOP(p.precio_venta)})</option>)}
                  </Select>
                  <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
                  <Input type="number" step="0.01" placeholder="Descuento RD$" value={l.descuento} onChange={(e) => updateLinea(i, { descuento: e.target.value })} />
                  <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
              <Button type="button" variant="secondary" size="sm" onClick={addLinea}>
                <Plus className="h-4 w-4" />Agregar línea
              </Button>
            </div>

            <div className="flex items-center justify-end">
              <p className="text-lg font-bold">Total: {formatDOP(total)}</p>
            </div>

            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

            <div className="flex gap-3">
              <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Crear cotización"}</Button>
              <Button type="button" variant="secondary" onClick={() => router.push("/cotizaciones")}>Cancelar</Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

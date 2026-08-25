"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Plus, Trash2 } from "lucide-react";
import { Button, Card, CardContent, Input, Label, Select } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Producto {
  id: string;
  sku: string;
  nombre: string;
  tipo: "PRODUCTO" | "SERVICIO";
}

interface Cliente {
  id: string;
  nombre: string;
}

interface Linea {
  productoId: string;
  cantidad: string;
  descripcion: string;
  observaciones: string;
}

export default function NuevoConducePage() {
  const router = useRouter();
  const [productos, setProductos] = useState<Producto[]>([]);
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [clienteId, setClienteId] = useState("");
  const [direccionEntrega, setDireccionEntrega] = useState("");
  const [notas, setNotas] = useState("");
  const [lineas, setLineas] = useState<Linea[]>([{ productoId: "", cantidad: "", descripcion: "", observaciones: "" }]);
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
    setLineas((ls) => [...ls, { productoId: "", cantidad: "", descripcion: "", observaciones: "" }]);
  }

  function removeLinea(i: number) {
    setLineas((ls) => ls.filter((_, idx) => idx !== i));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const items = lineas.filter((l) => l.productoId && l.cantidad);
    if (items.length === 0) {
      setError("Agrega al menos una línea con producto/servicio y cantidad.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      const conduce = await apiFetch<{ id: string }>("/api/conduces", {
        method: "POST",
        body: JSON.stringify({
          cliente_id: clienteId || undefined,
          direccion_entrega: direccionEntrega || undefined,
          notas: notas || undefined,
          items: items.map((l) => ({
            producto_id: l.productoId,
            cantidad: l.cantidad,
            descripcion: l.descripcion || undefined,
            observaciones: l.observaciones || undefined,
          })),
        }),
      });
      router.push(`/conduces/${conduce.id}` as any);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6 max-w-5xl">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nuevo conduce</h1>
        <p className="text-sm text-muted-foreground mt-1">Sin precio ni ITBIS — solo cantidad y logística de entrega, independiente de una venta.</p>
      </div>

      <Card>
        <CardContent className="pt-5">
          <form onSubmit={handleSubmit} className="space-y-5">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="cliente">Cliente</Label>
                <Select id="cliente" value={clienteId} onChange={(e) => setClienteId(e.target.value)}>
                  <option value="">Sin especificar</option>
                  {clientes.map((c) => <option key={c.id} value={c.id}>{c.nombre}</option>)}
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="direccion">Dirección de entrega</Label>
                <Input id="direccion" value={direccionEntrega} onChange={(e) => setDireccionEntrega(e.target.value)} />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Productos</Label>
              {lineas.map((l, i) => (
                <div key={i} className="grid grid-cols-[1fr_1fr_90px_32px] gap-2 items-end">
                  <Select value={l.productoId} onChange={(e) => updateLinea(i, { productoId: e.target.value })}>
                    <option value="">Producto/servicio…</option>
                    {productos.map((p) => (
                      <option key={p.id} value={p.id}>{p.sku} · {p.nombre}{p.tipo === "SERVICIO" ? " (servicio)" : ""}</option>
                    ))}
                  </Select>
                  <Input
                    placeholder="Descripción (opcional)"
                    value={l.descripcion}
                    onChange={(e) => updateLinea(i, { descripcion: e.target.value })}
                  />
                  <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
                  <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
              <Button type="button" variant="secondary" size="sm" onClick={addLinea}>
                <Plus className="h-4 w-4" />Agregar línea
              </Button>
            </div>

            <div className="space-y-1.5">
              <Label htmlFor="notas">Notas</Label>
              <Input id="notas" value={notas} onChange={(e) => setNotas(e.target.value)} />
            </div>

            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

            <div className="flex gap-3">
              <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Crear conduce"}</Button>
              <Button type="button" variant="secondary" onClick={() => router.push("/conduces")}>Cancelar</Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

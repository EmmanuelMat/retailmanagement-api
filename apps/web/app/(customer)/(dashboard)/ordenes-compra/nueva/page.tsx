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
  tipo: "PRODUCTO" | "SERVICIO";
  costo: string;
}

interface Proveedor {
  id: string;
  nombre: string;
}

interface Linea {
  productoId: string;
  cantidad: string;
  costoUnitario: string;
}

export default function NuevaOrdenCompraPage() {
  const router = useRouter();
  const [productos, setProductos] = useState<Producto[]>([]);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [proveedorId, setProveedorId] = useState("");
  const [fechaEsperada, setFechaEsperada] = useState("");
  const [lineas, setLineas] = useState<Linea[]>([{ productoId: "", cantidad: "", costoUnitario: "" }]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<{ items: Producto[] }>("/api/productos?pageSize=5000&activo=true&tipo=PRODUCTO").then((d) => setProductos(d.items)).catch(() => {});
    apiFetch<{ items: Proveedor[] }>("/api/proveedores?pageSize=1000&activo=true").then((d) => setProveedores(d.items)).catch(() => {});
  }, []);

  function updateLinea(i: number, patch: Partial<Linea>) {
    setLineas((ls) => ls.map((l, idx) => (idx === i ? { ...l, ...patch } : l)));
  }

  function addLinea() {
    setLineas((ls) => [...ls, { productoId: "", cantidad: "", costoUnitario: "" }]);
  }

  function removeLinea(i: number) {
    setLineas((ls) => ls.filter((_, idx) => idx !== i));
  }

  function onProductoChange(i: number, productoId: string) {
    const prod = productos.find((p) => p.id === productoId);
    updateLinea(i, { productoId, costoUnitario: prod ? prod.costo : "" });
  }

  const total = lineas.reduce((sum, l) => sum + (Number(l.cantidad) || 0) * (Number(l.costoUnitario) || 0), 0);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    if (!proveedorId) {
      setError("Selecciona un proveedor.");
      return;
    }
    const items = lineas.filter((l) => l.productoId && l.cantidad && l.costoUnitario);
    if (items.length === 0) {
      setError("Agrega al menos un producto con cantidad y costo.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      const orden = await apiFetch<{ id: string }>("/api/ordenes-compra", {
        method: "POST",
        body: JSON.stringify({
          proveedor_id: proveedorId,
          fecha_esperada: fechaEsperada || undefined,
          items: items.map((l) => ({ producto_id: l.productoId, cantidad_solicitada: l.cantidad, costo_unitario: l.costoUnitario })),
        }),
      });
      router.push(`/ordenes-compra/${orden.id}` as any);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6 max-w-6xl">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nueva orden de compra</h1>
        <p className="text-sm text-muted-foreground mt-1">No mueve inventario todavía — recibirla (total o parcialmente) crea la compra real.</p>
      </div>

      <Card>
        <CardContent className="pt-5">
          <form onSubmit={handleSubmit} className="space-y-5">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="proveedor">Proveedor</Label>
                <Select id="proveedor" value={proveedorId} onChange={(e) => setProveedorId(e.target.value)}>
                  <option value="">Selecciona un proveedor…</option>
                  {proveedores.map((p) => <option key={p.id} value={p.id}>{p.nombre}</option>)}
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="fechaEsperada">Fecha esperada</Label>
                <Input id="fechaEsperada" type="date" value={fechaEsperada} onChange={(e) => setFechaEsperada(e.target.value)} />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Productos</Label>
              {lineas.map((l, i) => (
                <div key={i} className="grid grid-cols-[1fr_90px_120px_32px] gap-2 items-end">
                  <Select value={l.productoId} onChange={(e) => onProductoChange(i, e.target.value)}>
                    <option value="">Producto…</option>
                    {productos.map((p) => <option key={p.id} value={p.id}>{p.sku} · {p.nombre}</option>)}
                  </Select>
                  <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
                  <Input type="number" step="0.01" placeholder="Costo c/u" value={l.costoUnitario} onChange={(e) => updateLinea(i, { costoUnitario: e.target.value })} />
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
              <p className="text-lg font-bold">Total estimado: {formatDOP(total)}</p>
            </div>

            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

            <div className="flex gap-3">
              <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Crear orden"}</Button>
              <Button type="button" variant="secondary" onClick={() => router.push("/ordenes-compra")}>Cancelar</Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

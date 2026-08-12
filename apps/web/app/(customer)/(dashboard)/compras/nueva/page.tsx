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
}

interface Proveedor {
  id: string;
  nombre: string;
}

interface Linea {
  productoId: string;
  cantidad: string;
  costoUnitario: string;
  itbisTipo: string;
}

const ITBIS_RATE: Record<string, number> = { GRAVADO_18: 0.18, GRAVADO_16: 0.16, EXENTO: 0 };

export default function NuevaCompraPage() {
  const router = useRouter();
  const [productos, setProductos] = useState<Producto[]>([]);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [proveedorId, setProveedorId] = useState("");
  const [ncfProveedor, setNcfProveedor] = useState("");
  const [metodoPago, setMetodoPago] = useState("EFECTIVO");
  const [lineas, setLineas] = useState<Linea[]>([{ productoId: "", cantidad: "", costoUnitario: "", itbisTipo: "EXENTO" }]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<{ items: Producto[] }>("/api/productos?pageSize=5000&activo=true").then((d) => setProductos(d.items)).catch(() => {});
    apiFetch<{ items: Proveedor[] }>("/api/proveedores?pageSize=1000&activo=true").then((d) => setProveedores(d.items)).catch(() => {});
  }, []);

  function updateLinea(i: number, patch: Partial<Linea>) {
    setLineas((ls) => ls.map((l, idx) => (idx === i ? { ...l, ...patch } : l)));
  }

  function addLinea() {
    setLineas((ls) => [...ls, { productoId: "", cantidad: "", costoUnitario: "", itbisTipo: "EXENTO" }]);
  }

  function removeLinea(i: number) {
    setLineas((ls) => ls.filter((_, idx) => idx !== i));
  }

  const total = lineas.reduce((sum, l) => {
    const sub = (Number(l.cantidad) || 0) * (Number(l.costoUnitario) || 0);
    return sum + sub + sub * (ITBIS_RATE[l.itbisTipo] || 0);
  }, 0);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const items = lineas.filter((l) => l.productoId && l.cantidad && l.costoUnitario);
    if (items.length === 0) {
      setError("Agrega al menos un producto con cantidad y costo.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/compras", {
        method: "POST",
        body: JSON.stringify({
          proveedor_id: proveedorId || undefined,
          ncf_proveedor: ncfProveedor || undefined,
          metodo_pago: metodoPago,
          items: items.map((l) => ({
            producto_id: l.productoId,
            cantidad: l.cantidad,
            costo_unitario: l.costoUnitario,
            itbis_tipo: l.itbisTipo,
          })),
        }),
      });
      router.push("/compras");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6 max-w-3xl">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nueva compra</h1>
        <p className="text-sm text-muted-foreground mt-1">Aumenta stock y recalcula el costo promedio de cada producto.</p>
      </div>

      <Card>
        <CardContent className="pt-5">
          <form onSubmit={handleSubmit} className="space-y-5">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="proveedor">Proveedor</Label>
                <Select id="proveedor" value={proveedorId} onChange={(e) => setProveedorId(e.target.value)}>
                  <option value="">Sin especificar</option>
                  {proveedores.map((p) => <option key={p.id} value={p.id}>{p.nombre}</option>)}
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="ncf">NCF del proveedor</Label>
                <Input id="ncf" value={ncfProveedor} onChange={(e) => setNcfProveedor(e.target.value)} placeholder="B0100000123" />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Productos</Label>
              {lineas.map((l, i) => (
                <div key={i} className="grid grid-cols-[1fr_90px_110px_100px_32px] gap-2 items-end">
                  <Select value={l.productoId} onChange={(e) => {
                    const prod = productos.find((p) => p.id === e.target.value);
                    updateLinea(i, { productoId: e.target.value, itbisTipo: prod?.itbis_tipo || "EXENTO" });
                  }}>
                    <option value="">Producto…</option>
                    {productos.map((p) => <option key={p.id} value={p.id}>{p.sku} · {p.nombre}</option>)}
                  </Select>
                  <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
                  <Input type="number" step="0.01" placeholder="Costo c/u" value={l.costoUnitario} onChange={(e) => updateLinea(i, { costoUnitario: e.target.value })} />
                  <Select value={l.itbisTipo} onChange={(e) => updateLinea(i, { itbisTipo: e.target.value })}>
                    <option value="GRAVADO_18">18%</option>
                    <option value="GRAVADO_16">16%</option>
                    <option value="EXENTO">Exento</option>
                  </Select>
                  <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </div>
              ))}
              <Button type="button" variant="secondary" size="sm" onClick={addLinea}>
                <Plus className="h-4 w-4" />Agregar línea
              </Button>
            </div>

            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-1.5">
                <Label htmlFor="metodo">Método de pago</Label>
                <Select id="metodo" value={metodoPago} onChange={(e) => setMetodoPago(e.target.value)}>
                  <option value="EFECTIVO">Efectivo</option>
                  <option value="TARJETA">Tarjeta</option>
                  <option value="TRANSFERENCIA">Transferencia</option>
                </Select>
              </div>
              <div className="flex items-end justify-end">
                <p className="text-lg font-bold">Total: {formatDOP(total)}</p>
              </div>
            </div>

            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

            <div className="flex gap-3">
              <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Registrar compra"}</Button>
              <Button type="button" variant="secondary" onClick={() => router.push("/compras")}>Cancelar</Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

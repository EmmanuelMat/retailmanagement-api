"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Plus, Trash2 } from "lucide-react";
import { Button, Card, CardContent, Input, Label, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { ClientePicker } from "../../cliente-picker";
import { ProductoPicker } from "../../producto-picker";

interface Producto {
  id: string;
  sku: string;
  nombre: string;
  itbis_tipo: string;
  precio_venta: string | null;
  tipo: "PRODUCTO" | "SERVICIO";
}

interface Linea {
  productoId: string;
  cantidad: string;
  descuento: string;
  precioUnitario: string;
  descripcion: string;
}

const ITBIS_RATE: Record<string, number> = { GRAVADO_18: 0.18, GRAVADO_16: 0.16, EXENTO: 0 };

export default function NuevaCotizacionPage() {
  const router = useRouter();
  const [productos, setProductos] = useState<Producto[]>([]);
  const [clienteId, setClienteId] = useState("");
  const [fechaVencimiento, setFechaVencimiento] = useState("");
  const [lineas, setLineas] = useState<Linea[]>([{ productoId: "", cantidad: "", descuento: "", precioUnitario: "", descripcion: "" }]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  useEffect(() => {
    // Un tenant SERVICIOS cotiza solo trabajo (servicios) - los materiales
    // se agregan después, ya con la orden de servicio, en su pestaña
    // Materiales (ver ordenes-servicio/[id]/page.tsx), no aquí.
    let esServicios = false;
    try {
      const raw = localStorage.getItem("tenant");
      if (raw) esServicios = JSON.parse(raw).tipo_negocio === "SERVICIOS";
    } catch {}
    const tipoQuery = esServicios ? "&tipo=SERVICIO" : "";
    apiFetch<{ items: Producto[] }>(`/api/productos?pageSize=5000&activo=true${tipoQuery}`).then((d) => setProductos(d.items)).catch(() => {});
  }, []);

  function updateLinea(i: number, patch: Partial<Linea>) {
    setLineas((ls) => ls.map((l, idx) => (idx === i ? { ...l, ...patch } : l)));
  }

  function addLinea() {
    setLineas((ls) => [...ls, { productoId: "", cantidad: "", descuento: "", precioUnitario: "", descripcion: "" }]);
  }

  function removeLinea(i: number) {
    setLineas((ls) => ls.filter((_, idx) => idx !== i));
  }

  function precioLinea(l: Linea): number {
    return Number(l.precioUnitario) || 0;
  }

  const total = lineas.reduce((sum, l) => {
    const prod = productos.find((p) => p.id === l.productoId);
    if (!prod) return sum;
    const bruto = (Number(l.cantidad) || 0) * precioLinea(l);
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
            precio_unitario: l.precioUnitario,
            descripcion: l.descripcion || undefined,
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
    <div className="space-y-6 max-w-5xl">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nueva cotización</h1>
        <p className="text-sm text-muted-foreground mt-1">No afecta stock ni caja — es solo una propuesta para el cliente.</p>
      </div>

      <Card>
        <CardContent className="pt-5">
          <form onSubmit={handleSubmit} className="space-y-5">
            <div className="grid grid-cols-2 gap-4 items-start">
              <ClientePicker clienteId={clienteId} onChange={setClienteId} />
              <div className="space-y-1.5">
                <Label htmlFor="vence">Válida hasta</Label>
                <Input id="vence" type="date" value={fechaVencimiento} onChange={(e) => setFechaVencimiento(e.target.value)} />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Productos</Label>
              {lineas.map((l, i) => (
                <div key={i} className="grid gap-2 items-end grid-cols-[1fr_90px_110px_110px_1fr_32px]">
                  <ProductoPicker
                    productos={productos}
                    value={l.productoId}
                    onChange={(id) => {
                      const nuevo = productos.find((p) => p.id === id);
                      updateLinea(i, { productoId: id, precioUnitario: nuevo?.tipo === "PRODUCTO" ? (nuevo.precio_venta || "") : "" });
                    }}
                  />
                  <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
                  <Input type="number" step="0.01" placeholder="Precio c/u" value={l.precioUnitario} onChange={(e) => updateLinea(i, { precioUnitario: e.target.value })} />
                  <Input type="number" step="0.01" placeholder="Descuento RD$" value={l.descuento} onChange={(e) => updateLinea(i, { descuento: e.target.value })} />
                  <Input placeholder="Descripción (opcional)" value={l.descripcion} onChange={(e) => updateLinea(i, { descripcion: e.target.value })} />
                  <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)}>
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

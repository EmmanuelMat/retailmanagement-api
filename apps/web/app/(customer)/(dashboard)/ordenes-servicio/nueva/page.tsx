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
  precio_venta: string | null;
  tipo: "PRODUCTO" | "SERVICIO";
}

interface Cliente {
  id: string;
  nombre: string;
}

interface Condicion {
  id: string;
  codigo: string;
  nombre: string;
}

interface Linea {
  productoId: string;
  cantidad: string;
  descuento: string;
  precioUnitario: string;
}

const ITBIS_RATE: Record<string, number> = { GRAVADO_18: 0.18, GRAVADO_16: 0.16, EXENTO: 0 };
const PRIORIDADES = [
  { value: "BAJA", label: "Baja" },
  { value: "NORMAL", label: "Normal" },
  { value: "ALTA", label: "Alta" },
  { value: "URGENTE", label: "Urgente" },
];

export default function NuevaOrdenServicioPage() {
  const router = useRouter();
  const [productos, setProductos] = useState<Producto[]>([]);
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [condiciones, setCondiciones] = useState<Condicion[]>([]);

  const [clienteId, setClienteId] = useState("");
  const [condicionId, setCondicionId] = useState("");
  const [prioridad, setPrioridad] = useState("NORMAL");
  const [fechaProgramada, setFechaProgramada] = useState("");
  const [direccion, setDireccion] = useState("");
  const [descripcion, setDescripcion] = useState("");
  const [lineas, setLineas] = useState<Linea[]>([{ productoId: "", cantidad: "", descuento: "", precioUnitario: "" }]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<{ items: Producto[] }>("/api/productos?pageSize=5000&activo=true").then((d) => setProductos(d.items)).catch(() => {});
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
    apiFetch<Condicion[]>("/api/condiciones-orden").then(setCondiciones).catch(() => {});
  }, []);

  function updateLinea(i: number, patch: Partial<Linea>) {
    setLineas((ls) => ls.map((l, idx) => (idx === i ? { ...l, ...patch } : l)));
  }

  function addLinea() {
    setLineas((ls) => [...ls, { productoId: "", cantidad: "", descuento: "", precioUnitario: "" }]);
  }

  function removeLinea(i: number) {
    setLineas((ls) => ls.filter((_, idx) => idx !== i));
  }

  function precioLinea(l: Linea, prod: Producto | undefined): number {
    if (!prod) return 0;
    return prod.tipo === "SERVICIO" ? Number(l.precioUnitario) || 0 : Number(prod.precio_venta) || 0;
  }

  const total = lineas.reduce((sum, l) => {
    const prod = productos.find((p) => p.id === l.productoId);
    if (!prod) return sum;
    const bruto = (Number(l.cantidad) || 0) * precioLinea(l, prod);
    const descuento = Math.min(Number(l.descuento) || 0, bruto);
    const sub = bruto - descuento;
    return sum + sub + sub * (ITBIS_RATE[prod.itbis_tipo] || 0);
  }, 0);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const items = lineas.filter((l) => l.productoId && l.cantidad);
    if (items.length === 0) {
      setError("Agrega al menos un producto o servicio con cantidad.");
      return;
    }
    const servicioSinPrecio = items.some((l) => {
      const prod = productos.find((p) => p.id === l.productoId);
      return prod?.tipo === "SERVICIO" && !(Number(l.precioUnitario) > 0);
    });
    if (servicioSinPrecio) {
      setError("Escribe el precio de cada servicio agregado.");
      return;
    }
    setSaving(true);
    setError("");
    try {
      const orden = await apiFetch<{ id: string }>("/api/ordenes-servicio", {
        method: "POST",
        body: JSON.stringify({
          cliente_id: clienteId || undefined,
          condicion_id: condicionId || undefined,
          prioridad,
          fecha_programada: fechaProgramada || undefined,
          direccion: direccion || undefined,
          descripcion: descripcion || undefined,
          items: items.map((l) => {
            const prod = productos.find((p) => p.id === l.productoId);
            return {
              producto_id: l.productoId,
              cantidad: l.cantidad,
              descuento: l.descuento || undefined,
              precio_unitario: prod?.tipo === "SERVICIO" ? l.precioUnitario : undefined,
            };
          }),
        }),
      });
      router.push(`/ordenes-servicio/${orden.id}` as any);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6 max-w-6xl">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nueva orden de servicio</h1>
        <p className="text-sm text-muted-foreground mt-1">Empieza en Borrador — asigna técnico, materiales y factura desde el detalle una vez creada.</p>
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
                <Label htmlFor="condicion">Condición</Label>
                <Select id="condicion" value={condicionId} onChange={(e) => setCondicionId(e.target.value)}>
                  <option value="">Sin especificar</option>
                  {condiciones.map((c) => <option key={c.id} value={c.id}>{c.nombre}</option>)}
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="prioridad">Prioridad</Label>
                <Select id="prioridad" value={prioridad} onChange={(e) => setPrioridad(e.target.value)}>
                  {PRIORIDADES.map((p) => <option key={p.value} value={p.value}>{p.label}</option>)}
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="fechaProgramada">Fecha programada</Label>
                <Input id="fechaProgramada" type="date" value={fechaProgramada} onChange={(e) => setFechaProgramada(e.target.value)} />
              </div>
              <div className="space-y-1.5 col-span-2">
                <Label htmlFor="direccion">Dirección</Label>
                <Input id="direccion" value={direccion} onChange={(e) => setDireccion(e.target.value)} placeholder="Dirección donde se realizará el trabajo" />
              </div>
              <div className="space-y-1.5 col-span-2">
                <Label htmlFor="descripcion">Descripción del trabajo</Label>
                <Input id="descripcion" value={descripcion} onChange={(e) => setDescripcion(e.target.value)} placeholder="Qué reporta o pide el cliente" />
              </div>
            </div>

            <div className="space-y-2">
              <Label>Productos / Servicios</Label>
              {lineas.map((l, i) => {
                const prod = productos.find((p) => p.id === l.productoId);
                const esServicio = prod?.tipo === "SERVICIO";
                return (
                  <div key={i} className={`grid gap-2 items-end ${esServicio ? "grid-cols-[1fr_90px_110px_110px_32px]" : "grid-cols-[1fr_90px_110px_32px]"}`}>
                    <Select value={l.productoId} onChange={(e) => updateLinea(i, { productoId: e.target.value })}>
                      <option value="">Producto o servicio…</option>
                      {productos.map((p) => (
                        <option key={p.id} value={p.id}>
                          {p.sku} · {p.nombre} {p.tipo === "SERVICIO" ? "(servicio)" : `(${formatDOP(p.precio_venta || "0")})`}
                        </option>
                      ))}
                    </Select>
                    <Input type="number" step="0.01" placeholder="Cant." value={l.cantidad} onChange={(e) => updateLinea(i, { cantidad: e.target.value })} />
                    {esServicio && (
                      <Input type="number" step="0.01" placeholder="Precio c/u" value={l.precioUnitario} onChange={(e) => updateLinea(i, { precioUnitario: e.target.value })} />
                    )}
                    <Input type="number" step="0.01" placeholder="Descuento RD$" value={l.descuento} onChange={(e) => updateLinea(i, { descuento: e.target.value })} />
                    <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 1}>
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                );
              })}
              <Button type="button" variant="secondary" size="sm" onClick={addLinea}>
                <Plus className="h-4 w-4" />Agregar línea
              </Button>
            </div>

            <div className="flex items-center justify-end">
              <p className="text-lg font-bold">Total: {formatDOP(total)}</p>
            </div>

            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

            <div className="flex gap-3">
              <Button type="submit" disabled={saving}>{saving ? "Guardando..." : "Crear orden"}</Button>
              <Button type="button" variant="secondary" onClick={() => router.push("/ordenes-servicio")}>Cancelar</Button>
            </div>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

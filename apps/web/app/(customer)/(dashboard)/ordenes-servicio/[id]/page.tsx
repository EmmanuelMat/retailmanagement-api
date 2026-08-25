"use client";

import { useEffect, useMemo, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { FileText, Plus, Trash2 } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Select, Tabs, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { ESTADO_VARIANT } from "../page";

interface OrdenItem {
  id: string;
  producto_id: string;
  sku: string;
  nombre: string;
  tipo: "PRODUCTO" | "SERVICIO";
  cantidad: string;
  precio_unitario: string;
  descuento: string;
  itbis_monto: string;
  subtotal: string;
}

interface OrdenTecnico {
  id: string;
  empleado_id: string;
  rol: string;
  fecha_asignacion: string;
}

interface OrdenMaterial {
  id: string;
  producto_id: string;
  cantidad_planificada: string;
  cantidad_utilizada: string;
  costo_unitario: string | null;
}

interface OrdenNota {
  id: string;
  tipo: string;
  contenido: string;
  created_at: string;
}

interface OrdenDetalle {
  id: string;
  cliente_id: string | null;
  cotizacion_id: string | null;
  venta_id: string | null;
  condicion_id: string | null;
  estado: string;
  prioridad: string;
  fecha: string;
  fecha_programada: string | null;
  direccion: string | null;
  descripcion: string | null;
  subtotal: string;
  descuento: string;
  itbis_total: string;
  total: string;
  notas: string | null;
  created_at: string;
  items: OrdenItem[];
  tecnicos: OrdenTecnico[];
  materiales: OrdenMaterial[];
  notas_registro: OrdenNota[];
}

interface Cliente { id: string; nombre: string; }
interface Condicion { id: string; codigo: string; nombre: string; }
interface Empleado { id: string; nombre: string; }
interface Producto { id: string; sku: string; nombre: string; tipo: "PRODUCTO" | "SERVICIO"; itbis_tipo: string; precio_venta: string | null; }
interface AuditoriaEntry { id: string; accion: string; created_at: string; }

const ESTADO_LABEL: Record<string, string> = {
  BORRADOR: "Borrador", PROGRAMADA: "Programada", EN_PROCESO: "En proceso",
  PAUSADA: "Pausada", COMPLETADA: "Completada", CANCELADA: "Cancelada",
};
const ROL_TECNICO_LABEL: Record<string, string> = { TECNICO_PRINCIPAL: "Técnico principal", ASISTENTE: "Asistente" };
const NOTA_TIPO_VARIANT: Record<string, "default" | "secondary" | "accent"> = { INTERNA: "secondary", TECNICO: "accent", CLIENTE: "default", SISTEMA: "secondary" };

const TABS = [
  { value: "resumen", label: "Resumen" },
  { value: "items", label: "Items" },
  { value: "tecnicos", label: "Técnicos" },
  { value: "materiales", label: "Materiales" },
  { value: "notas", label: "Notas" },
  { value: "actividad", label: "Actividad" },
  { value: "facturacion", label: "Facturación" },
];

export default function OrdenServicioDetallePage() {
  const params = useParams<{ id: string }>();
  const [orden, setOrden] = useState<OrdenDetalle | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [tab, setTab] = useState("resumen");

  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [condiciones, setCondiciones] = useState<Condicion[]>([]);
  const [empleados, setEmpleados] = useState<Empleado[]>([]);
  const [productos, setProductos] = useState<Producto[]>([]);
  const [auditoria, setAuditoria] = useState<AuditoriaEntry[]>([]);

  function load() {
    apiFetch<OrdenDetalle>(`/api/ordenes-servicio/${params.id}`).then(setOrden).catch((e) => setError(e.message));
  }

  useEffect(() => {
    load();
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
    apiFetch<Condicion[]>("/api/condiciones-orden").then(setCondiciones).catch(() => {});
    apiFetch<{ items: Empleado[] }>("/api/empleados?pageSize=1000&activo=true").then((d) => setEmpleados(d.items)).catch(() => {});
    apiFetch<{ items: Producto[] }>("/api/productos?pageSize=5000&activo=true").then((d) => setProductos(d.items)).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [params.id]);

  useEffect(() => {
    if (tab !== "actividad" || !params.id) return;
    apiFetch<{ items: AuditoriaEntry[] }>(`/api/auditoria?entidad=orden_servicio&entidadId=${params.id}&pageSize=50&sortBy=created_at&sortDir=desc`)
      .then((d) => setAuditoria(d.items))
      .catch(() => {});
  }, [tab, params.id]);

  const clienteNombre = clientes.find((c) => c.id === orden?.cliente_id)?.nombre;
  const condicionNombre = condiciones.find((c) => c.id === orden?.condicion_id)?.nombre;
  const tecnicoPrincipal = orden?.tecnicos.find((t) => t.rol === "TECNICO_PRINCIPAL");
  const nombreEmpleado = (id: string) => empleados.find((e) => e.id === id)?.nombre || "—";
  const nombreProducto = (id: string) => productos.find((p) => p.id === id);

  async function runAction(path: string, onOk?: () => void) {
    setBusy(true);
    setError("");
    try {
      await apiFetch(`/api/ordenes-servicio/${params.id}${path}`, { method: "POST", body: JSON.stringify({}) });
      load();
      onOk?.();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setBusy(false);
    }
  }

  if (error && !orden) return <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>;
  if (!orden) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  const codigo = `OS-${orden.id.slice(0, 8).toUpperCase()}`;
  const puedeCancelar = !["COMPLETADA", "CANCELADA"].includes(orden.estado);

  return (
    <div className="space-y-4 max-w-6xl">
      <div className="flex items-start justify-between flex-wrap gap-3">
        <div className="flex items-center gap-3 flex-wrap">
          <h1 className="text-2xl font-bold font-serif tracking-tight">Orden de Servicio</h1>
          <span className="font-mono text-sm text-muted-foreground">{codigo}</span>
          <Badge variant={ESTADO_VARIANT[orden.estado] || "default"}>{ESTADO_LABEL[orden.estado] || orden.estado}</Badge>
        </div>
        <div className="flex gap-2 flex-wrap">
          <Link href={`/imprimir/orden-servicio/${orden.id}` as any} target="_blank">
            <Button size="sm" variant="secondary"><FileText className="h-3.5 w-3.5" />Imprimir</Button>
          </Link>
          {(orden.estado === "BORRADOR" || orden.estado === "PROGRAMADA" || orden.estado === "PAUSADA") && (
            <Button size="sm" disabled={busy} onClick={() => runAction("/iniciar")}>Iniciar</Button>
          )}
          {orden.estado === "EN_PROCESO" && (
            <>
              <Button size="sm" variant="secondary" disabled={busy} onClick={() => runAction("/pausar")}>Pausar</Button>
              <Button size="sm" disabled={busy} onClick={() => runAction("/completar")}>Completar</Button>
            </>
          )}
          {puedeCancelar && (
            <Button size="sm" variant="destructive" disabled={busy} onClick={() => runAction("/cancelar")}>Cancelar</Button>
          )}
        </div>
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}

      <Card>
        <CardContent className="pt-5 grid grid-cols-2 sm:grid-cols-4 gap-4 text-sm">
          <div><p className="text-xs text-muted-foreground">Cliente</p><p className="font-medium">{clienteNombre || "Consumidor final"}</p></div>
          <div><p className="text-xs text-muted-foreground">Condición</p><p className="font-medium">{condicionNombre || "—"}</p></div>
          <div><p className="text-xs text-muted-foreground">Prioridad</p><p className="font-medium">{orden.prioridad}</p></div>
          <div><p className="text-xs text-muted-foreground">Técnico principal</p><p className="font-medium">{tecnicoPrincipal ? nombreEmpleado(tecnicoPrincipal.empleado_id) : "Sin asignar"}</p></div>
          <div><p className="text-xs text-muted-foreground">Fecha</p><p className="text-muted-foreground">{new Date(orden.fecha).toLocaleDateString("es-DO")}</p></div>
          <div><p className="text-xs text-muted-foreground">Programada</p><p className="text-muted-foreground">{orden.fecha_programada ? new Date(orden.fecha_programada).toLocaleDateString("es-DO") : "—"}</p></div>
          <div><p className="text-xs text-muted-foreground">Dirección</p><p className="text-muted-foreground">{orden.direccion || "—"}</p></div>
          <div><p className="text-xs text-muted-foreground">Cotización origen</p><p className="text-muted-foreground">{orden.cotizacion_id ? <Link href={`/cotizaciones/${orden.cotizacion_id}` as any} className="text-primary hover:underline">Ver cotización</Link> : "—"}</p></div>
        </CardContent>
      </Card>

      <Card>
        <Tabs
          items={TABS.map((t) => ({
            ...t,
            badge: t.value === "tecnicos" ? orden.tecnicos.length
              : t.value === "materiales" ? orden.materiales.length
              : t.value === "notas" ? orden.notas_registro.length
              : undefined,
          }))}
          value={tab}
          onChange={setTab}
          className="px-4"
        />
        <CardContent className="pt-5">
          {tab === "resumen" && <ResumenTab orden={orden} />}
          {tab === "items" && <ItemsTab orden={orden} productos={productos} onChanged={load} />}
          {tab === "tecnicos" && <TecnicosTab orden={orden} empleados={empleados} nombreEmpleado={nombreEmpleado} onChanged={load} />}
          {tab === "materiales" && <MaterialesTab orden={orden} productos={productos} nombreProducto={nombreProducto} onChanged={load} />}
          {tab === "notas" && <NotasTab orden={orden} onChanged={load} />}
          {tab === "actividad" && <ActividadTab entradas={auditoria} />}
          {tab === "facturacion" && <FacturacionTab orden={orden} onChanged={load} />}
        </CardContent>
      </Card>
    </div>
  );
}

function TotalsBox({ orden }: { orden: OrdenDetalle }) {
  return (
    <div className="text-sm space-y-1 max-w-xs ml-auto pt-3">
      <div className="flex justify-between text-muted-foreground"><span>Subtotal</span><span className="tabular-nums">{formatDOP(orden.subtotal)}</span></div>
      <div className="flex justify-between text-muted-foreground"><span>Descuento</span><span className="tabular-nums">{formatDOP(orden.descuento)}</span></div>
      <div className="flex justify-between text-muted-foreground"><span>ITBIS</span><span className="tabular-nums">{formatDOP(orden.itbis_total)}</span></div>
      <div className="flex justify-between font-bold text-base pt-1 border-t border-border mt-1"><span>Total</span><span className="tabular-nums">{formatDOP(orden.total)}</span></div>
    </div>
  );
}

function ResumenTab({ orden }: { orden: OrdenDetalle }) {
  return (
    <div className="space-y-4">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Producto</TableHead>
            <TableHead className="text-right">Cant.</TableHead>
            <TableHead className="text-right">Precio</TableHead>
            <TableHead className="text-right">ITBIS</TableHead>
            <TableHead className="text-right">Subtotal</TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {orden.items.map((it) => (
            <TableRow key={it.id}>
              <TableCell>
                <span className="font-medium">{it.nombre}</span>
                {it.tipo === "SERVICIO" && <Badge variant="accent" className="ml-2">Servicio</Badge>}
              </TableCell>
              <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
              <TableCell className="text-right tabular-nums">{formatDOP(it.precio_unitario)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatDOP(it.itbis_monto)}</TableCell>
              <TableCell className="text-right tabular-nums font-medium">{formatDOP(it.subtotal)}</TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <TotalsBox orden={orden} />
      {orden.descripcion && (
        <div className="text-sm text-muted-foreground bg-muted/40 rounded-md p-3">{orden.descripcion}</div>
      )}
    </div>
  );
}

function ItemsTab({ orden, productos, onChanged }: { orden: OrdenDetalle; productos: Producto[]; onChanged: () => void }) {
  const [productoId, setProductoId] = useState("");
  const [cantidad, setCantidad] = useState("");
  const [precioUnitario, setPrecioUnitario] = useState("");
  const [descuento, setDescuento] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const soloLectura = ["COMPLETADA", "CANCELADA"].includes(orden.estado);
  const prod = productos.find((p) => p.id === productoId);
  const esServicio = prod?.tipo === "SERVICIO";

  async function handleAdd() {
    if (!productoId || !cantidad) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/items`, {
        method: "POST",
        body: JSON.stringify({
          producto_id: productoId,
          cantidad,
          descuento: descuento || undefined,
          precio_unitario: esServicio ? precioUnitario : undefined,
        }),
      });
      setProductoId(""); setCantidad(""); setPrecioUnitario(""); setDescuento("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleRemove(itemId: string) {
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/items/${itemId}`, { method: "DELETE" });
      onChanged();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="space-y-4">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>Producto</TableHead>
            <TableHead className="text-right">Cant.</TableHead>
            <TableHead className="text-right">Precio c/u</TableHead>
            <TableHead className="text-right">Descuento</TableHead>
            <TableHead className="text-right">Subtotal</TableHead>
            {!soloLectura && <TableHead className="w-10"></TableHead>}
          </TableRow>
        </TableHeader>
        <TableBody>
          {orden.items.map((it) => (
            <TableRow key={it.id}>
              <TableCell className="font-medium">{it.nombre}</TableCell>
              <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
              <TableCell className="text-right tabular-nums">{formatDOP(it.precio_unitario)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatDOP(it.descuento)}</TableCell>
              <TableCell className="text-right tabular-nums font-medium">{formatDOP(it.subtotal)}</TableCell>
              {!soloLectura && (
                <TableCell className="text-right">
                  <Button size="icon" variant="ghost" onClick={() => handleRemove(it.id)}><Trash2 className="h-3.5 w-3.5" /></Button>
                </TableCell>
              )}
            </TableRow>
          ))}
        </TableBody>
      </Table>

      {!soloLectura && (
        <div className={`grid gap-2 items-end ${esServicio ? "grid-cols-[1fr_90px_110px_110px_auto]" : "grid-cols-[1fr_90px_110px_auto]"}`}>
          <Select value={productoId} onChange={(e) => setProductoId(e.target.value)}>
            <option value="">Producto o servicio…</option>
            {productos.map((p) => <option key={p.id} value={p.id}>{p.sku} · {p.nombre}</option>)}
          </Select>
          <Input type="number" step="0.01" placeholder="Cant." value={cantidad} onChange={(e) => setCantidad(e.target.value)} />
          {esServicio && <Input type="number" step="0.01" placeholder="Precio c/u" value={precioUnitario} onChange={(e) => setPrecioUnitario(e.target.value)} />}
          <Input type="number" step="0.01" placeholder="Descuento RD$" value={descuento} onChange={(e) => setDescuento(e.target.value)} />
          <Button type="button" size="sm" disabled={saving || !productoId || !cantidad} onClick={handleAdd}><Plus className="h-4 w-4" />Agregar</Button>
        </div>
      )}
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
      <TotalsBox orden={orden} />
    </div>
  );
}

function TecnicosTab({ orden, empleados, nombreEmpleado, onChanged }: { orden: OrdenDetalle; empleados: Empleado[]; nombreEmpleado: (id: string) => string; onChanged: () => void }) {
  const [empleadoId, setEmpleadoId] = useState("");
  const [rol, setRol] = useState("TECNICO_PRINCIPAL");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  async function handleAsignar() {
    if (!empleadoId) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/tecnicos`, { method: "POST", body: JSON.stringify({ empleado_id: empleadoId, rol }) });
      setEmpleadoId("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleQuitar(asignacionId: string) {
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/tecnicos/${asignacionId}`, { method: "DELETE" });
      onChanged();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="space-y-4">
      {orden.tecnicos.length === 0 ? (
        <p className="text-sm text-muted-foreground py-6 text-center">Sin técnicos asignados.</p>
      ) : (
        <div className="space-y-2">
          {orden.tecnicos.map((t) => (
            <div key={t.id} className="flex items-center gap-3 border border-border rounded-md p-3">
              <div className="flex-1">
                <p className="text-sm font-medium">{nombreEmpleado(t.empleado_id)}</p>
                <p className="text-xs text-muted-foreground">Asignado {new Date(t.fecha_asignacion).toLocaleDateString("es-DO")}</p>
              </div>
              <Badge variant={t.rol === "TECNICO_PRINCIPAL" ? "accent" : "secondary"}>{ROL_TECNICO_LABEL[t.rol] || t.rol}</Badge>
              <Button size="icon" variant="ghost" onClick={() => handleQuitar(t.id)}><Trash2 className="h-3.5 w-3.5" /></Button>
            </div>
          ))}
        </div>
      )}
      <div className="grid grid-cols-[1fr_160px_auto] gap-2 items-end">
        <Select value={empleadoId} onChange={(e) => setEmpleadoId(e.target.value)}>
          <option value="">Empleado…</option>
          {empleados.map((e) => <option key={e.id} value={e.id}>{e.nombre}</option>)}
        </Select>
        <Select value={rol} onChange={(e) => setRol(e.target.value)}>
          <option value="TECNICO_PRINCIPAL">Técnico principal</option>
          <option value="ASISTENTE">Asistente</option>
        </Select>
        <Button size="sm" disabled={saving || !empleadoId} onClick={handleAsignar}><Plus className="h-4 w-4" />Asignar</Button>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
    </div>
  );
}

function MaterialesTab({ orden, productos, nombreProducto, onChanged }: { orden: OrdenDetalle; productos: Producto[]; nombreProducto: (id: string) => Producto | undefined; onChanged: () => void }) {
  const [productoId, setProductoId] = useState("");
  const [cantidadPlanificada, setCantidadPlanificada] = useState("");
  const [consumos, setConsumos] = useState<Record<string, string>>({});
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  const productosDisponibles = useMemo(
    () => productos.filter((p) => p.tipo === "PRODUCTO" && !orden.materiales.some((m) => m.producto_id === p.id)),
    [productos, orden.materiales]
  );

  async function handleAgregar() {
    if (!productoId || !cantidadPlanificada) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/materiales`, {
        method: "POST",
        body: JSON.stringify({ producto_id: productoId, cantidad_planificada: cantidadPlanificada }),
      });
      setProductoId(""); setCantidadPlanificada("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleConsumir(materialId: string) {
    const cantidad = consumos[materialId];
    if (!cantidad) return;
    setError("");
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/materiales/${materialId}/consumir`, { method: "POST", body: JSON.stringify({ cantidad }) });
      setConsumos((c) => ({ ...c, [materialId]: "" }));
      onChanged();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="space-y-4">
      {orden.materiales.length === 0 ? (
        <p className="text-sm text-muted-foreground py-6 text-center">Sin materiales registrados.</p>
      ) : (
        <div className="space-y-2">
          {orden.materiales.map((m) => {
            const prod = nombreProducto(m.producto_id);
            const pct = Number(m.cantidad_planificada) > 0 ? Math.min(100, (Number(m.cantidad_utilizada) / Number(m.cantidad_planificada)) * 100) : 0;
            return (
              <div key={m.id} className="border border-border rounded-md p-3 grid grid-cols-[1.6fr_1fr_1fr_auto] gap-3 items-center text-sm">
                <div>
                  <p className="font-medium">{prod?.nombre || "Producto"}</p>
                  <p className="text-xs text-muted-foreground font-mono">{prod?.sku}</p>
                </div>
                <div><p className="text-xs text-muted-foreground">Planificado</p><p>{m.cantidad_planificada}</p></div>
                <div>
                  <p className="text-xs text-muted-foreground">Consumido</p>
                  <p>{m.cantidad_utilizada}</p>
                  <div className="h-1.5 rounded-full bg-muted mt-1 w-20 overflow-hidden"><div className="h-full bg-primary" style={{ width: `${pct}%` }} /></div>
                </div>
                <div className="flex gap-1.5">
                  <Input
                    type="number" step="0.01" placeholder="Cant." className="w-20 h-8 text-xs"
                    value={consumos[m.id] || ""} onChange={(e) => setConsumos((c) => ({ ...c, [m.id]: e.target.value }))}
                  />
                  <Button size="sm" variant="secondary" disabled={!consumos[m.id]} onClick={() => handleConsumir(m.id)}>Consumir</Button>
                </div>
              </div>
            );
          })}
        </div>
      )}

      <div className="grid grid-cols-[1fr_140px_auto] gap-2 items-end">
        <Select value={productoId} onChange={(e) => setProductoId(e.target.value)}>
          <option value="">Producto…</option>
          {productosDisponibles.map((p) => <option key={p.id} value={p.id}>{p.sku} · {p.nombre}</option>)}
        </Select>
        <Input type="number" step="0.01" placeholder="Cant. planificada" value={cantidadPlanificada} onChange={(e) => setCantidadPlanificada(e.target.value)} />
        <Button size="sm" disabled={saving || !productoId || !cantidadPlanificada} onClick={handleAgregar}><Plus className="h-4 w-4" />Agregar</Button>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
      <p className="text-xs text-muted-foreground">Un producto ya facturado como línea (pestaña Items) no puede registrarse también aquí — evita descontar el inventario dos veces.</p>
    </div>
  );
}

function NotasTab({ orden, onChanged }: { orden: OrdenDetalle; onChanged: () => void }) {
  const [tipo, setTipo] = useState("INTERNA");
  const [contenido, setContenido] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  async function handleAgregar() {
    if (!contenido.trim()) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/ordenes-servicio/${orden.id}/notas`, { method: "POST", body: JSON.stringify({ tipo, contenido }) });
      setContenido("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-4">
      {orden.notas_registro.length === 0 ? (
        <p className="text-sm text-muted-foreground py-6 text-center">Sin notas todavía.</p>
      ) : (
        <div className="space-y-3">
          {orden.notas_registro.map((n) => (
            <div key={n.id} className="border-b border-border pb-3 last:border-0">
              <div className="flex items-center gap-2 mb-1">
                <Badge variant={NOTA_TIPO_VARIANT[n.tipo] || "secondary"}>{n.tipo}</Badge>
                <span className="text-xs text-muted-foreground">{new Date(n.created_at).toLocaleString("es-DO")}</span>
              </div>
              <p className="text-sm">{n.contenido}</p>
            </div>
          ))}
        </div>
      )}
      <div className="flex gap-2">
        <Select value={tipo} onChange={(e) => setTipo(e.target.value)} className="max-w-[140px]">
          <option value="INTERNA">Interna</option>
          <option value="TECNICO">Técnico</option>
          <option value="CLIENTE">Cliente</option>
        </Select>
        <Input value={contenido} onChange={(e) => setContenido(e.target.value)} placeholder="Escribir una nota…" onKeyDown={(e) => e.key === "Enter" && handleAgregar()} />
        <Button size="sm" disabled={saving || !contenido.trim()} onClick={handleAgregar}>Agregar</Button>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
    </div>
  );
}

function ActividadTab({ entradas }: { entradas: AuditoriaEntry[] }) {
  if (entradas.length === 0) return <p className="text-sm text-muted-foreground py-6 text-center">Sin actividad registrada.</p>;
  return (
    <div className="space-y-3">
      {entradas.map((e) => (
        <div key={e.id} className="flex gap-3">
          <div className="h-2 w-2 rounded-full bg-primary mt-1.5 shrink-0" />
          <div>
            <p className="text-sm">{e.accion.replaceAll("_", " ").toLowerCase()}</p>
            <p className="text-xs text-muted-foreground font-mono">{new Date(e.created_at).toLocaleString("es-DO")}</p>
          </div>
        </div>
      ))}
    </div>
  );
}

function FacturacionTab({ orden, onChanged }: { orden: OrdenDetalle; onChanged: () => void }) {
  const [metodoPago, setMetodoPago] = useState("EFECTIVO");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  async function handleFacturar() {
    setSaving(true);
    setError("");
    try {
      const venta = await apiFetch<{ id: string }>(`/api/ordenes-servicio/${orden.id}/crear-factura`, { method: "POST", body: JSON.stringify({ metodo_pago: metodoPago }) });
      onChanged();
      window.location.href = `/ventas/${venta.id}`;
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  if (orden.venta_id) {
    return (
      <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">
        Facturada — <Link href={`/ventas/${orden.venta_id}` as any} className="underline font-medium">ver venta</Link>.
      </div>
    );
  }

  if (orden.estado !== "COMPLETADA") {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <p className="text-sm">Esta orden todavía no se ha facturado.</p>
        <p className="text-xs mt-1">Completa el trabajo para habilitar la facturación.</p>
      </div>
    );
  }

  return (
    <div className="max-w-xs space-y-3">
      <div className="space-y-1.5">
        <Label htmlFor="metodo">Método de pago</Label>
        <Select id="metodo" value={metodoPago} onChange={(e) => setMetodoPago(e.target.value)}>
          <option value="EFECTIVO">Efectivo</option>
          <option value="TARJETA">Tarjeta</option>
          <option value="TRANSFERENCIA">Transferencia</option>
          <option value="FIADO">Fiado</option>
        </Select>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}
      <Button className="w-full" disabled={saving} onClick={handleFacturar}>{saving ? "Procesando..." : "Crear factura"}</Button>
    </div>
  );
}

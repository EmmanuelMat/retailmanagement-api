"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import { FileText, Plus, Trash2 } from "lucide-react";
import { Badge, Button, Card, CardContent, Dialog, Input, Label, Select, Tabs, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch, ApiError } from "@/lib/api";
import { ClientePicker } from "../../cliente-picker";

interface CotizacionItem {
  id: string;
  sku: string;
  nombre: string;
  cantidad: string;
  precio_unitario: string;
  descuento: string;
  itbis_monto: string;
  subtotal: string;
}

interface Producto {
  id: string;
  sku: string;
  nombre: string;
  itbis_tipo: string;
  precio_venta: string | null;
  tipo: "PRODUCTO" | "SERVICIO";
}

interface CotizacionDetalle {
  id: string;
  cliente_id: string | null;
  subtotal: string;
  itbis_total: string;
  total: string;
  estado: string;
  fecha_vencimiento: string | null;
  venta_id: string | null;
  created_at: string;
  items: CotizacionItem[];
}

interface Cliente { id: string; nombre: string; }
interface AuditoriaEntry { id: string; accion: string; created_at: string; }

const ESTADO_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  PENDIENTE: "warning",
  ACEPTADA: "default",
  CONVERTIDA: "success",
  RECHAZADA: "destructive",
  VENCIDA: "secondary",
};
const ESTADO_LABEL: Record<string, string> = {
  PENDIENTE: "Pendiente", ACEPTADA: "Aceptada", CONVERTIDA: "Convertida", RECHAZADA: "Rechazada", VENCIDA: "Vencida",
};

const TABS = [
  { value: "resumen", label: "Resumen" },
  { value: "conversion", label: "Conversión" },
  { value: "actividad", label: "Actividad" },
];

export default function CotizacionDetallePage() {
  const params = useParams<{ id: string }>();
  const router = useRouter();
  const [cotizacion, setCotizacion] = useState<CotizacionDetalle | null>(null);
  const [error, setError] = useState("");
  const [rechazando, setRechazando] = useState(false);
  const [tab, setTab] = useState("resumen");

  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [productos, setProductos] = useState<Producto[]>([]);
  const [auditoria, setAuditoria] = useState<AuditoriaEntry[]>([]);
  const [esServicios, setEsServicios] = useState(false);
  const [cambiandoCliente, setCambiandoCliente] = useState(false);
  const [nuevoClienteId, setNuevoClienteId] = useState("");
  const [guardandoCliente, setGuardandoCliente] = useState(false);

  useEffect(() => {
    try {
      const raw = localStorage.getItem("tenant");
      if (raw) setEsServicios(JSON.parse(raw).tipo_negocio === "SERVICIOS");
    } catch {}
  }, []);

  function load() {
    apiFetch<CotizacionDetalle>(`/api/cotizaciones/${params.id}`).then(setCotizacion).catch((e) => setError(e.message));
  }

  useEffect(() => {
    load();
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
    const tipoQuery = esServicios ? "&tipo=SERVICIO" : "";
    apiFetch<{ items: Producto[] }>(`/api/productos?pageSize=5000&activo=true${tipoQuery}`).then((d) => setProductos(d.items)).catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [params.id, esServicios]);

  async function handleGuardarCliente() {
    setGuardandoCliente(true);
    setError("");
    try {
      await apiFetch(`/api/cotizaciones/${params.id}/cliente`, {
        method: "PUT",
        body: JSON.stringify({ cliente_id: nuevoClienteId || null }),
      });
      setCambiandoCliente(false);
      load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setGuardandoCliente(false);
    }
  }

  useEffect(() => {
    if (tab !== "actividad" || !params.id) return;
    apiFetch<{ items: AuditoriaEntry[] }>(`/api/auditoria?entidad=cotizacion&entidadId=${params.id}&pageSize=50&sortBy=created_at&sortDir=desc`)
      .then((d) => setAuditoria(d.items))
      .catch(() => {});
  }, [tab, params.id]);

  const clienteNombre = clientes.find((c) => c.id === cotizacion?.cliente_id)?.nombre;

  async function handleRechazar() {
    setRechazando(true);
    try {
      await apiFetch(`/api/cotizaciones/${params.id}/rechazar`, { method: "POST" });
      load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setRechazando(false);
    }
  }

  if (error && !cotizacion) return <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>;
  if (!cotizacion) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  const codigo = `COT-${cotizacion.id.slice(0, 8).toUpperCase()}`;
  const puedeConvertir = cotizacion.estado === "PENDIENTE" || cotizacion.estado === "ACEPTADA";
  // Igual que el guard en cotizacion_service.rs::add_item/remove_item/update_cliente -
  // ya no se puede editar una vez CONVERTIDA (venta/orden real) o RECHAZADA.
  const puedeEditar = cotizacion.estado !== "CONVERTIDA" && cotizacion.estado !== "RECHAZADA";

  return (
    <div className="space-y-4 max-w-6xl">
      <div className="flex items-start justify-between flex-wrap gap-3">
        <div className="flex items-center gap-3 flex-wrap">
          <h1 className="text-2xl font-bold font-serif tracking-tight">Cotización</h1>
          <span className="font-mono text-sm text-muted-foreground">{codigo}</span>
          <Badge variant={ESTADO_VARIANT[cotizacion.estado] || "default"}>{ESTADO_LABEL[cotizacion.estado] || cotizacion.estado}</Badge>
        </div>
        <div className="flex gap-2 flex-wrap">
          <Link href={`/imprimir/cotizacion/${cotizacion.id}` as any} target="_blank">
            <Button size="sm" variant="secondary"><FileText className="h-3.5 w-3.5" />Imprimir</Button>
          </Link>
          {puedeConvertir && (
            <Button size="sm" variant="destructive" disabled={rechazando} onClick={handleRechazar}>Rechazar</Button>
          )}
        </div>
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}

      <Card>
        <CardContent className="pt-5 grid grid-cols-2 sm:grid-cols-4 gap-4 text-sm">
          <div>
            <p className="text-xs text-muted-foreground">Cliente</p>
            <p className="font-medium flex items-center gap-2">
              {clienteNombre || "Consumidor final"}
              {puedeEditar && (
                <button
                  type="button"
                  className="text-xs font-normal text-primary hover:underline"
                  onClick={() => { setNuevoClienteId(cotizacion.cliente_id || ""); setCambiandoCliente(true); }}
                >
                  Cambiar
                </button>
              )}
            </p>
          </div>
          <div><p className="text-xs text-muted-foreground">Fecha</p><p className="text-muted-foreground">{new Date(cotizacion.created_at).toLocaleDateString("es-DO")}</p></div>
          <div><p className="text-xs text-muted-foreground">Válida hasta</p><p className="text-muted-foreground">{cotizacion.fecha_vencimiento ? new Date(cotizacion.fecha_vencimiento).toLocaleDateString("es-DO") : "—"}</p></div>
          <div><p className="text-xs text-muted-foreground">Venta destino</p><p className="text-muted-foreground">{cotizacion.venta_id ? <Link href={`/ventas/${cotizacion.venta_id}` as any} className="text-primary hover:underline">Ver venta</Link> : "—"}</p></div>
        </CardContent>
      </Card>

      <Dialog open={cambiandoCliente} onClose={() => setCambiandoCliente(false)} title="Cambiar cliente">
        <div className="space-y-4">
          <ClientePicker clienteId={nuevoClienteId} onChange={setNuevoClienteId} />
          <div className="flex gap-3">
            <Button className="flex-1" disabled={guardandoCliente} onClick={handleGuardarCliente}>
              {guardandoCliente ? "Guardando..." : "Guardar"}
            </Button>
            <Button variant="secondary" onClick={() => setCambiandoCliente(false)}>Cancelar</Button>
          </div>
        </div>
      </Dialog>

      <Card>
        <Tabs items={TABS} value={tab} onChange={setTab} className="px-4" />
        <CardContent className="pt-5">
          {tab === "resumen" && (
            <ResumenTab cotizacion={cotizacion} productos={productos} puedeEditar={puedeEditar} onChanged={load} />
          )}
          {tab === "conversion" && (
            <ConversionTab
              cotizacion={cotizacion}
              puedeConvertir={puedeConvertir}
              esServicios={esServicios}
              onConvertida={(destino) => router.push(destino as any)}
            />
          )}
          {tab === "actividad" && <ActividadTab entradas={auditoria} />}
        </CardContent>
      </Card>
    </div>
  );
}

function TotalsBox({ cotizacion }: { cotizacion: CotizacionDetalle }) {
  return (
    <div className="text-sm space-y-1 max-w-xs ml-auto pt-3">
      <div className="flex justify-between text-muted-foreground"><span>Subtotal</span><span className="tabular-nums">{formatDOP(cotizacion.subtotal)}</span></div>
      <div className="flex justify-between text-muted-foreground"><span>ITBIS</span><span className="tabular-nums">{formatDOP(cotizacion.itbis_total)}</span></div>
      <div className="flex justify-between font-bold text-base pt-1 border-t border-border mt-1"><span>Total</span><span className="tabular-nums">{formatDOP(cotizacion.total)}</span></div>
    </div>
  );
}

function ResumenTab({
  cotizacion,
  productos,
  puedeEditar,
  onChanged,
}: {
  cotizacion: CotizacionDetalle;
  productos: Producto[];
  puedeEditar: boolean;
  onChanged: () => void;
}) {
  const [productoId, setProductoId] = useState("");
  const [cantidad, setCantidad] = useState("");
  const [descuento, setDescuento] = useState("");
  const [precioUnitario, setPrecioUnitario] = useState("");
  const [agregando, setAgregando] = useState(false);
  const [quitandoId, setQuitandoId] = useState<string | null>(null);
  const [error, setError] = useState("");

  const prodSel = productos.find((p) => p.id === productoId);
  const esServicio = prodSel?.tipo === "SERVICIO";

  async function handleAdd() {
    if (!productoId || !cantidad) return;
    if (esServicio && !(Number(precioUnitario) > 0)) {
      setError("Escribe el precio de este servicio.");
      return;
    }
    setAgregando(true);
    setError("");
    try {
      await apiFetch(`/api/cotizaciones/${cotizacion.id}/items`, {
        method: "POST",
        body: JSON.stringify({
          producto_id: productoId,
          cantidad,
          descuento: descuento || undefined,
          precio_unitario: esServicio ? precioUnitario : undefined,
        }),
      });
      setProductoId("");
      setCantidad("");
      setDescuento("");
      setPrecioUnitario("");
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setAgregando(false);
    }
  }

  async function handleRemove(itemId: string) {
    setQuitandoId(itemId);
    setError("");
    try {
      await apiFetch(`/api/cotizaciones/${cotizacion.id}/items/${itemId}`, { method: "DELETE" });
      onChanged();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setQuitandoId(null);
    }
  }

  return (
    <div className="space-y-4">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead>SKU</TableHead>
            <TableHead>Producto</TableHead>
            <TableHead className="text-right">Cant.</TableHead>
            <TableHead className="text-right">Precio</TableHead>
            <TableHead className="text-right">Descuento</TableHead>
            <TableHead className="text-right">Subtotal</TableHead>
            {puedeEditar && <TableHead className="w-10" />}
          </TableRow>
        </TableHeader>
        <TableBody>
          {cotizacion.items.map((it) => (
            <TableRow key={it.id}>
              <TableCell className="font-mono text-xs text-muted-foreground">{it.sku}</TableCell>
              <TableCell className="font-medium">{it.nombre}</TableCell>
              <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
              <TableCell className="text-right tabular-nums">{formatDOP(it.precio_unitario)}</TableCell>
              <TableCell className="text-right tabular-nums">{formatDOP(it.descuento)}</TableCell>
              <TableCell className="text-right tabular-nums font-medium">{formatDOP(it.subtotal)}</TableCell>
              {puedeEditar && (
                <TableCell className="text-right">
                  <Button size="icon" variant="ghost" disabled={quitandoId === it.id} onClick={() => handleRemove(it.id)}>
                    <Trash2 className="h-4 w-4" />
                  </Button>
                </TableCell>
              )}
            </TableRow>
          ))}
        </TableBody>
      </Table>

      {puedeEditar && (
        <div className={`grid gap-2 items-end ${esServicio ? "grid-cols-[1fr_90px_110px_110px_auto]" : "grid-cols-[1fr_90px_110px_auto]"}`}>
          <Select value={productoId} onChange={(e) => setProductoId(e.target.value)}>
            <option value="">Producto…</option>
            {productos.map((p) => (
              <option key={p.id} value={p.id}>
                {p.sku} · {p.nombre} {p.tipo === "SERVICIO" ? "(servicio)" : `(${formatDOP(p.precio_venta || "0")})`}
              </option>
            ))}
          </Select>
          <Input type="number" step="0.01" placeholder="Cant." value={cantidad} onChange={(e) => setCantidad(e.target.value)} />
          {esServicio && (
            <Input type="number" step="0.01" placeholder="Precio c/u" value={precioUnitario} onChange={(e) => setPrecioUnitario(e.target.value)} />
          )}
          <Input type="number" step="0.01" placeholder="Descuento RD$" value={descuento} onChange={(e) => setDescuento(e.target.value)} />
          <Button type="button" size="sm" disabled={agregando || !productoId || !cantidad} onClick={handleAdd}>
            <Plus className="h-4 w-4" />{agregando ? "..." : "Agregar"}
          </Button>
        </div>
      )}

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}

      <TotalsBox cotizacion={cotizacion} />
    </div>
  );
}

function ConversionTab({
  cotizacion,
  puedeConvertir,
  esServicios,
  onConvertida,
}: {
  cotizacion: CotizacionDetalle;
  puedeConvertir: boolean;
  esServicios: boolean;
  onConvertida: (destino: string) => void;
}) {
  const [metodoPago, setMetodoPago] = useState("EFECTIVO");
  const [convirtiendo, setConvirtiendo] = useState(false);
  const [convertirError, setConvertirError] = useState("");
  const [convirtiendoOrden, setConvirtiendoOrden] = useState(false);

  const [mostrarAprobacion, setMostrarAprobacion] = useState(false);
  const [aprobacionMensaje, setAprobacionMensaje] = useState("");
  const [adminEmail, setAdminEmail] = useState("");
  const [adminPassword, setAdminPassword] = useState("");

  async function handleConvertirAOrden() {
    setConvirtiendoOrden(true);
    setConvertirError("");
    try {
      const orden = await apiFetch<{ id: string }>(`/api/cotizaciones/${cotizacion.id}/convertir-a-orden`, { method: "POST", body: JSON.stringify({}) });
      onConvertida(`/ordenes-servicio/${orden.id}`);
    } catch (e: any) {
      setConvertirError(e.message);
    } finally {
      setConvirtiendoOrden(false);
    }
  }

  async function handleConvertir(adminCreds?: { email: string; password: string }) {
    setConvirtiendo(true);
    setConvertirError("");
    try {
      const venta = await apiFetch<{ id: string }>(`/api/cotizaciones/${cotizacion.id}/convertir`, {
        method: "POST",
        body: JSON.stringify({ metodo_pago: metodoPago, aprobacion_admin: adminCreds }),
      });
      setMostrarAprobacion(false);
      onConvertida(`/ventas/${venta.id}`);
    } catch (e: any) {
      if (e instanceof ApiError && e.status === 403 && e.message.startsWith("DESCUENTO_REQUIERE_APROBACION")) {
        setAprobacionMensaje(e.message);
        setMostrarAprobacion(true);
      } else {
        setConvertirError(e.message);
      }
    } finally {
      setConvirtiendo(false);
    }
  }

  if (cotizacion.venta_id) {
    return (
      <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">
        Convertida en venta — <Link href={`/ventas/${cotizacion.venta_id}` as any} className="underline font-medium">ver venta</Link>.
      </div>
    );
  }

  if (!puedeConvertir) {
    return (
      <div className="text-center py-8 text-muted-foreground">
        <p className="text-sm">Esta cotización ya no se puede convertir.</p>
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
          <option value="FIADO">{esServicios ? "A crédito" : "Fiado"}</option>
        </Select>
      </div>
      {convertirError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{convertirError}</div>}
      <Button className="w-full" onClick={() => handleConvertir()} disabled={convirtiendo}>
        {convirtiendo ? "Procesando..." : "Convertir a venta"}
      </Button>
      <Button className="w-full" variant="secondary" onClick={handleConvertirAOrden} disabled={convirtiendoOrden}>
        {convirtiendoOrden ? "Procesando..." : "Convertir a orden de servicio"}
      </Button>

      {mostrarAprobacion && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4">
          <Card className="w-full max-w-sm">
            <CardContent className="pt-5 space-y-3">
              <div>
                <p className="text-sm font-semibold">Aprobación de administrador requerida</p>
                <p className="text-xs text-muted-foreground mt-1">{aprobacionMensaje}</p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="adminEmail">Correo del administrador</Label>
                <Input id="adminEmail" type="email" value={adminEmail} onChange={(e) => setAdminEmail(e.target.value)} autoFocus />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="adminPassword">Contraseña</Label>
                <Input
                  id="adminPassword"
                  type="password"
                  value={adminPassword}
                  onChange={(e) => setAdminPassword(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && adminEmail && adminPassword && handleConvertir({ email: adminEmail, password: adminPassword })}
                />
              </div>
              {convertirError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{convertirError}</div>}
              <div className="flex gap-2">
                <Button
                  className="flex-1"
                  disabled={convirtiendo || !adminEmail || !adminPassword}
                  onClick={() => handleConvertir({ email: adminEmail, password: adminPassword })}
                >
                  {convirtiendo ? "Verificando..." : "Aprobar y convertir"}
                </Button>
                <Button variant="secondary" onClick={() => { setMostrarAprobacion(false); setAdminEmail(""); setAdminPassword(""); setConvertirError(""); }}>
                  Cancelar
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
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

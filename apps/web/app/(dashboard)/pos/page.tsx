"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Search, Plus, Minus, Trash2, ShoppingCart, FileCheck, Printer, CheckCircle2, XCircle, IdCard, ChevronDown } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Select, formatDOP } from "@repo/ui";
import { apiFetch, ApiError } from "@/lib/api";

interface Producto {
  id: string;
  sku: string;
  nombre: string;
  itbis_tipo: "GRAVADO_18" | "GRAVADO_16" | "EXENTO";
  precio_venta: string;
  stock_actual: string;
}

interface Cliente {
  id: string;
  nombre: string;
  rnc_cedula?: string | null;
  direccion?: string | null;
}

interface RncRecord {
  rnc: string;
  nombre: string;
  nombre_comercial: string | null;
  estado: string | null;
}

interface CarritoLinea {
  producto: Producto;
  cantidad: number;
  descuento: number;
}

const ITBIS_RATE: Record<string, number> = { GRAVADO_18: 0.18, GRAVADO_16: 0.16, EXENTO: 0 };
const ITBIS_LABEL: Record<string, string> = { GRAVADO_18: "18%", GRAVADO_16: "16%", EXENTO: "Exento" };

interface VentaItem {
  nombre: string;
  cantidad: string;
  precio_unitario: string;
  subtotal: string;
}

interface VentaResult {
  id: string;
  subtotal: string;
  itbis_total: string;
  total: string;
  metodo_pago: string;
  tipo_ecf: number | null;
  e_ncf: string | null;
  qr_url: string | null;
  codigo_seguridad: string | null;
  estado_dgii: string | null;
  created_at: string;
  items?: VentaItem[];
}

export default function PosPage() {
  const [productos, setProductos] = useState<Producto[]>([]);
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [search, setSearch] = useState("");
  const [carrito, setCarrito] = useState<CarritoLinea[]>([]);
  const [clienteId, setClienteId] = useState("");
  const [facturaElectronicaActiva, setFacturaElectronicaActiva] = useState(true);
  const [tipoEcf, setTipoEcf] = useState("32");
  const [metodoPago, setMetodoPago] = useState("EFECTIVO");
  const [entregaDiferida, setEntregaDiferida] = useState(false);
  const [cobrando, setCobrando] = useState(false);
  const [error, setError] = useState("");
  const [ventaResult, setVentaResult] = useState<VentaResult | null>(null);
  const [postVentaStatus, setPostVentaStatus] = useState<{ ecfError?: string; impreso: boolean; printError?: string }>({ impreso: false });

  const [rncQuery, setRncQuery] = useState("");
  const [rncBuscando, setRncBuscando] = useState(false);
  const [rncFound, setRncFound] = useState<RncRecord | null>(null);
  const [rncNotFound, setRncNotFound] = useState(false);
  const [quickNombre, setQuickNombre] = useState("");
  const [quickDireccion, setQuickDireccion] = useState("");
  const [usandoCliente, setUsandoCliente] = useState(false);
  const [mostrarRnc, setMostrarRnc] = useState(false);

  const [mostrarAprobacion, setMostrarAprobacion] = useState(false);
  const [aprobacionMensaje, setAprobacionMensaje] = useState("");
  const [adminEmail, setAdminEmail] = useState("");
  const [adminPassword, setAdminPassword] = useState("");
  const [aprobacionError, setAprobacionError] = useState("");

  const [cajaAbierta, setCajaAbierta] = useState<boolean | null>(null);

  useEffect(() => {
    apiFetch<{ items: Producto[] }>("/api/productos?pageSize=5000&activo=true").then((d) => setProductos(d.items)).catch(() => {});
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
    apiFetch<{ sesion: unknown | null }>("/api/caja/resumen").then((d) => setCajaAbierta(!!d.sesion)).catch(() => {});
    try {
      const raw = localStorage.getItem("tenant");
      if (raw) {
        const t = JSON.parse(raw);
        if (t.factura_electronica_activa === false) setFacturaElectronicaActiva(false);
      }
    } catch {}
  }, []);

  const MAX_VISIBLE = 60;

  const matched = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return productos;
    return productos.filter((p) => p.nombre.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q));
  }, [search, productos]);

  // Un colmado puede tener miles de SKUs — pintar todos como botones a la
  // vez no ayuda a nadie a encontrar el producto más rápido, solo pesa el
  // DOM. Se corta a un número manejable y se pide refinar la búsqueda.
  const filtered = useMemo(() => matched.slice(0, MAX_VISIBLE), [matched]);
  const hayMasSinMostrar = matched.length > filtered.length;

  const totals = useMemo(() => {
    let subtotal = 0, itbis = 0;
    for (const l of carrito) {
      const precio = Number(l.producto.precio_venta);
      const bruto = precio * l.cantidad;
      const descuento = Math.min(l.descuento, bruto);
      const lineSub = bruto - descuento;
      subtotal += lineSub;
      itbis += lineSub * ITBIS_RATE[l.producto.itbis_tipo];
    }
    return { subtotal, itbis, total: subtotal + itbis };
  }, [carrito]);

  function addToCart(producto: Producto) {
    setCarrito((c) => {
      const existing = c.find((l) => l.producto.id === producto.id);
      if (existing) {
        return c.map((l) => (l.producto.id === producto.id ? { ...l, cantidad: l.cantidad + 1 } : l));
      }
      return [...c, { producto, cantidad: 1, descuento: 0 }];
    });
  }

  function updateQty(productoId: string, delta: number) {
    setCarrito((c) =>
      c
        .map((l) => (l.producto.id === productoId ? { ...l, cantidad: l.cantidad + delta } : l))
        .filter((l) => l.cantidad > 0)
    );
  }

  function updateDescuento(productoId: string, value: string) {
    const monto = Math.max(0, Number(value) || 0);
    setCarrito((c) => c.map((l) => (l.producto.id === productoId ? { ...l, descuento: monto } : l)));
  }

  function removeLine(productoId: string) {
    setCarrito((c) => c.filter((l) => l.producto.id !== productoId));
  }

  async function handleVerificarRnc() {
    const rnc = rncQuery.trim();
    if (!rnc) return;
    setRncBuscando(true);
    setRncFound(null);
    setRncNotFound(false);
    setError("");
    try {
      const data = await apiFetch<RncRecord>(`/api/rnc/${encodeURIComponent(rnc)}`);
      setRncFound(data);
      setQuickNombre(data.nombre_comercial || data.nombre);
    } catch (e) {
      // Solo un 404 real significa "no está en el padrón DGII" — cualquier
      // otro fallo (red, servidor caído, etc.) se muestra como error real en
      // vez de decirle al cajero que el RNC no existe cuando sí existe.
      if (e instanceof ApiError && e.status === 404) {
        setRncNotFound(true);
        setQuickNombre("");
        setQuickDireccion("");
      } else {
        setError(e instanceof Error ? e.message : "No se pudo verificar el RNC");
      }
    } finally {
      setRncBuscando(false);
    }
  }

  // Usa (o crea) un Cliente a partir del RNC/Cédula escrito directamente en
  // el POS, sin tener que ir primero a la sección Clientes — necesario para
  // comprador de paso que pide Crédito Fiscal o para Consumo >= RD$250,000.
  async function handleUsarClienteRnc() {
    const rnc = rncQuery.trim().replace(/\D/g, "");
    if (!rnc || !quickNombre.trim()) return;
    setUsandoCliente(true);
    setError("");
    try {
      const existing = clientes.find((c) => (c.rnc_cedula || "").replace(/\D/g, "") === rnc);
      let cliente: Cliente;
      if (existing) {
        cliente = await apiFetch<Cliente>(`/api/clientes/${existing.id}`, {
          method: "PUT",
          body: JSON.stringify({ nombre: quickNombre, rnc_cedula: rnc, direccion: quickDireccion || undefined }),
        });
        setClientes((cs) => cs.map((c) => (c.id === cliente.id ? cliente : c)));
      } else {
        cliente = await apiFetch<Cliente>("/api/clientes", {
          method: "POST",
          body: JSON.stringify({ nombre: quickNombre, rnc_cedula: rnc, direccion: quickDireccion || undefined }),
        });
        setClientes((cs) => [...cs, cliente]);
      }
      setClienteId(cliente.id);
      setRncQuery("");
      setRncFound(null);
      setRncNotFound(false);
      setQuickNombre("");
      setQuickDireccion("");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setUsandoCliente(false);
    }
  }

  async function handleCobrar(adminCreds?: { email: string; password: string }) {
    if (carrito.length === 0) return;
    if (metodoPago === "FIADO" && !clienteId) {
      setError("Una venta fiada necesita un cliente seleccionado");
      return;
    }
    setCobrando(true);
    setError("");
    if (adminCreds) setAprobacionError("");
    try {
      let venta = await apiFetch<VentaResult>("/api/ventas", {
        method: "POST",
        body: JSON.stringify({
          cliente_id: clienteId || undefined,
          metodo_pago: metodoPago,
          tipo_ecf: Number(tipoEcf),
          items: carrito.map((l) => ({
            producto_id: l.producto.id,
            cantidad: String(l.cantidad),
            descuento: String(Math.min(l.descuento, Number(l.producto.precio_venta) * l.cantidad)),
          })),
          entrega_diferida: entregaDiferida || undefined,
          aprobacion_admin: adminCreds,
        }),
      });
      setMostrarAprobacion(false);
      setAdminEmail("");
      setAdminPassword("");
      const items = venta.items;

      const status: { ecfError?: string; impreso: boolean; printError?: string } = { impreso: false };

      // Emisión de e-CF automática usando el certificado guardado en
      // Configuración → DGII (si no hay uno guardado, esto falla y se deja
      // la venta como "sin e-CF" para emitir manualmente después). La
      // respuesta de este endpoint no trae los renglones, así que se
      // reponen desde la venta original para no perderlos en el recibo.
      // Si el negocio no tiene e-CF activado, ni se intenta — la venta
      // queda como un ticket normal, sin avisos de DGII.
      if (facturaElectronicaActiva) {
        try {
          venta = await apiFetch<VentaResult>(`/api/ventas/${venta.id}/emitir-ecf`, {
            method: "POST",
            body: JSON.stringify({}),
          });
          venta.items = items;
        } catch (e: any) {
          status.ecfError = e.message;
        }
      }

      // Cada venta cerrada intenta imprimir el ticket en la impresora de red
      // configurada. Si falla (impresora apagada/no configurada) no bloquea
      // el cierre de la venta — solo se informa en pantalla.
      try {
        await apiFetch(`/api/ventas/${venta.id}/imprimir`, { method: "POST" });
        status.impreso = true;
      } catch (e: any) {
        status.printError = e.message;
      }

      setPostVentaStatus(status);
      setVentaResult(venta);
      setCarrito([]);
      apiFetch<{ items: Producto[] }>("/api/productos?pageSize=5000&activo=true").then((d) => setProductos(d.items)).catch(() => {});
    } catch (e: any) {
      if (e instanceof ApiError && e.message.startsWith("CAJA_NO_ABIERTA")) {
        // Caso raro: la caja se cerró (en otra pestaña/usuario) después de
        // cargar esta página, cuando el botón ya debería estar deshabilitado.
        setCajaAbierta(false);
        setError(e.message.replace("CAJA_NO_ABIERTA: ", ""));
      } else if (e instanceof ApiError && e.status === 403 && e.message.startsWith("DESCUENTO_REQUIERE_APROBACION")) {
        // El cajero pidió más descuento del que su cuenta permite sin
        // autorización — se pide a un administrador que confirme aquí mismo,
        // sin cerrar la sesión del cajero.
        setAprobacionMensaje(e.message);
        setMostrarAprobacion(true);
      } else if (adminCreds) {
        // Ya estábamos reintentando con credenciales de admin y aun así
        // falló (contraseña incorrecta, no es ADMIN, etc.) — se muestra
        // dentro del mismo popup en vez de cerrarlo.
        setAprobacionError(e.message);
      } else {
        setError(e.message);
      }
    } finally {
      setCobrando(false);
    }
  }

  if (ventaResult) {
    const cliente = clientes.find((c) => c.id === clienteId);
    return (
      <VentaConfirmada
        venta={ventaResult}
        cliente={cliente}
        status={postVentaStatus}
        onStatusChange={setPostVentaStatus}
        onVentaChange={setVentaResult}
        onNuevaVenta={() => {
          setVentaResult(null);
          setClienteId("");
          setTipoEcf("32");
        }}
      />
    );
  }

  return (
    <div className="grid grid-cols-1 lg:grid-cols-[1fr_380px] gap-6 h-[calc(100vh-104px)]">
      <div className="flex flex-col min-h-0">
        <div className="relative mb-4">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Buscar producto por nombre o SKU..." className="pl-9" />
        </div>
        <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 overflow-y-auto pb-4">
          {filtered.map((p) => {
            const stock = Number(p.stock_actual);
            return (
              <button
                key={p.id}
                onClick={() => stock > 0 && addToCart(p)}
                disabled={stock <= 0}
                className="text-left rounded-lg border border-border bg-background p-3 hover:border-primary transition-colors disabled:opacity-40 disabled:cursor-not-allowed"
              >
                <p className="text-xs text-muted-foreground font-mono">{p.sku}</p>
                <p className="text-sm font-semibold mt-1 leading-tight">{p.nombre}</p>
                <div className="flex items-center justify-between mt-2">
                  <span className="text-sm font-bold">{formatDOP(p.precio_venta)}</span>
                  <Badge variant={p.itbis_tipo === "EXENTO" ? "secondary" : "default"}>{ITBIS_LABEL[p.itbis_tipo]}</Badge>
                </div>
                <p className="text-[11px] text-muted-foreground mt-1">{stock > 0 ? `Stock: ${p.stock_actual}` : "Sin stock"}</p>
              </button>
            );
          })}
          {filtered.length === 0 && (
            <p className="col-span-full text-sm text-muted-foreground py-10 text-center">No hay productos que coincidan.</p>
          )}
        </div>
        {hayMasSinMostrar && (
          <p className="text-xs text-muted-foreground text-center py-2 border-t border-border shrink-0">
            Mostrando {filtered.length} de {matched.length} — sigue escribiendo para refinar la búsqueda.
          </p>
        )}
      </div>

      <Card className="flex flex-col sticky top-0 h-[calc(100vh-104px)]">
        <CardContent className="flex flex-col flex-1 min-h-0 pt-5">
          <div className="flex items-center gap-2 mb-4">
            <ShoppingCart className="h-4 w-4 text-primary" />
            <h2 className="font-bold text-sm">Venta actual</h2>
          </div>

          <div className="flex-1 overflow-y-auto space-y-2 min-h-0">
            {carrito.length === 0 ? (
              <p className="text-sm text-muted-foreground text-center py-10">Agrega productos para empezar.</p>
            ) : (
              carrito.map((l) => (
                <div key={l.producto.id} className="border border-border rounded-md p-2 space-y-1.5">
                  <div className="flex items-center gap-2">
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">{l.producto.nombre}</p>
                      <p className="text-xs text-muted-foreground">{formatDOP(l.producto.precio_venta)} c/u</p>
                    </div>
                    <div className="flex items-center gap-1">
                      <Button size="icon" variant="ghost" className="h-6 w-6" onClick={() => updateQty(l.producto.id, -1)}><Minus className="h-3 w-3" /></Button>
                      <span className="text-sm w-6 text-center tabular-nums">{l.cantidad}</span>
                      <Button size="icon" variant="ghost" className="h-6 w-6" onClick={() => updateQty(l.producto.id, 1)}><Plus className="h-3 w-3" /></Button>
                    </div>
                    <Button size="icon" variant="ghost" className="h-6 w-6" onClick={() => removeLine(l.producto.id)}><Trash2 className="h-3.5 w-3.5" /></Button>
                  </div>
                  <div className="flex items-center gap-1.5 pl-0.5">
                    <span className="text-[11px] text-muted-foreground shrink-0">Descuento RD$</span>
                    <Input
                      type="number"
                      min={0}
                      step="0.01"
                      value={l.descuento || ""}
                      onChange={(e) => updateDescuento(l.producto.id, e.target.value)}
                      placeholder="0.00"
                      className="h-7 text-xs"
                    />
                  </div>
                </div>
              ))
            )}
          </div>

          <div className="border-t border-border pt-4 mt-4 space-y-3">
            <div className="space-y-1.5">
              <button
                type="button"
                onClick={() => setMostrarRnc((v) => !v)}
                className="flex items-center justify-between w-full rounded-md border border-border p-2.5 text-xs font-medium hover:bg-muted transition-colors"
              >
                <span>RNC / Crédito Fiscal {rncFound || rncNotFound ? "· en curso" : "(opcional)"}</span>
                <ChevronDown className={`h-3.5 w-3.5 text-muted-foreground transition-transform ${mostrarRnc ? "rotate-180" : ""}`} />
              </button>

              {mostrarRnc && (
                <div className="space-y-2 pt-1.5">
                  <Label htmlFor="rncQuery">RNC / Cédula del cliente</Label>
                  <div className="flex gap-2">
                    <Input
                      id="rncQuery"
                      value={rncQuery}
                      onChange={(e) => {
                        setRncQuery(e.target.value);
                        setRncFound(null);
                        setRncNotFound(false);
                      }}
                      placeholder="130793752"
                      onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), handleVerificarRnc())}
                    />
                    <Button type="button" variant="secondary" onClick={handleVerificarRnc} disabled={rncBuscando || !rncQuery.trim()}>
                      <IdCard className="h-4 w-4" />{rncBuscando ? "..." : "Verificar"}
                    </Button>
                  </div>

                  {rncFound && (
                    <div className="rounded-md border border-border p-2.5 space-y-2">
                      <p className="text-xs text-muted-foreground flex items-center gap-1.5">
                        Encontrado en DGII
                        <Badge variant={rncFound.estado === "ACTIVO" ? "success" : "secondary"}>{rncFound.estado || "—"}</Badge>
                      </p>
                      <Input value={quickNombre} onChange={(e) => setQuickNombre(e.target.value)} placeholder="Nombre del cliente" />
                      <Input value={quickDireccion} onChange={(e) => setQuickDireccion(e.target.value)} placeholder="Dirección (requerida para Crédito Fiscal)" />
                      <Button type="button" size="sm" className="w-full" onClick={handleUsarClienteRnc} disabled={usandoCliente || !quickNombre.trim()}>
                        {usandoCliente ? "Guardando..." : "Usar este cliente en la venta"}
                      </Button>
                    </div>
                  )}

                  {rncNotFound && (
                    <div className="rounded-md border border-warning/20 bg-warning/10 p-2.5 space-y-2">
                      <p className="text-xs text-warning">No encontrado en el padrón DGII. Puedes registrarlo manualmente:</p>
                      <Input value={quickNombre} onChange={(e) => setQuickNombre(e.target.value)} placeholder="Nombre del cliente *" />
                      <Input value={quickDireccion} onChange={(e) => setQuickDireccion(e.target.value)} placeholder="Dirección (requerida para Crédito Fiscal)" />
                      <Button type="button" size="sm" className="w-full" onClick={handleUsarClienteRnc} disabled={usandoCliente || !quickNombre.trim()}>
                        {usandoCliente ? "Guardando..." : "Usar este cliente en la venta"}
                      </Button>
                    </div>
                  )}
                </div>
              )}
            </div>

            <div className="grid grid-cols-2 gap-2">
              <div className="space-y-1.5">
                <Label htmlFor="cliente">Cliente</Label>
                <Select
                  id="cliente"
                  value={clienteId}
                  onChange={(e) => {
                    setClienteId(e.target.value);
                    const c = clientes.find((cl) => cl.id === e.target.value);
                    const tieneRnc = !!c?.rnc_cedula && c.rnc_cedula !== "000000000";
                    if (!tieneRnc) setTipoEcf("32");
                  }}
                >
                  <option value="">Consumidor final</option>
                  {clientes.map((c) => <option key={c.id} value={c.id}>{c.nombre}</option>)}
                </Select>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="metodo">Método de pago</Label>
                <Select id="metodo" value={metodoPago} onChange={(e) => setMetodoPago(e.target.value)}>
                  <option value="EFECTIVO">Efectivo</option>
                  <option value="TARJETA">Tarjeta</option>
                  <option value="TRANSFERENCIA">Transferencia</option>
                  <option value="CREDITO">Crédito</option>
                  <option value="FIADO">Fiado</option>
                </Select>
              </div>
            </div>
            {(() => {
              if (!facturaElectronicaActiva) return null;
              const clienteSel = clientes.find((c) => c.id === clienteId);
              const tieneRnc = !!clienteSel?.rnc_cedula && clienteSel.rnc_cedula !== "000000000";
              return tieneRnc ? (
                <div className="space-y-1.5">
                  <Label htmlFor="tipoEcf">Tipo de comprobante</Label>
                  <Select id="tipoEcf" value={tipoEcf} onChange={(e) => setTipoEcf(e.target.value)}>
                    <option value="32">Consumo (32)</option>
                    <option value="31">Crédito Fiscal (31)</option>
                  </Select>
                </div>
              ) : null;
            })()}

            {metodoPago === "FIADO" && !clienteId && (
              <div className="rounded-md border border-warning/20 bg-warning/10 text-warning p-2 text-xs">
                Selecciona un cliente arriba para fiar esta venta.
              </div>
            )}

            <label className="flex items-start gap-2 text-xs text-muted-foreground rounded-md border border-border p-2.5">
              <input
                type="checkbox"
                className="mt-0.5"
                checked={entregaDiferida}
                onChange={(e) => setEntregaDiferida(e.target.checked)}
              />
              <span>
                <span className="block font-medium text-foreground">Entrega diferida</span>
                La mercancía no sale completa ahora — se factura todo, pero el cliente la recoge en varias partes. El stock se descuenta según se vaya entregando (Ventas → Entregas).
              </span>
            </label>

            {cajaAbierta === false && (
              <div className="rounded-md border border-warning/20 bg-warning/10 text-warning p-2 text-xs">
                La caja está cerrada. <Link href="/caja" className="underline font-medium">Ábrela antes de vender</Link>.
              </div>
            )}

            <div className="text-sm space-y-1 pt-1">
              <div className="flex justify-between text-muted-foreground"><span>Subtotal</span><span className="tabular-nums">{formatDOP(totals.subtotal)}</span></div>
              <div className="flex justify-between text-muted-foreground"><span>ITBIS</span><span className="tabular-nums">{formatDOP(totals.itbis)}</span></div>
              <div className="flex justify-between font-bold text-base pt-1"><span>Total</span><span className="tabular-nums">{formatDOP(totals.total)}</span></div>
            </div>

            {totals.total >= 250000 && !clienteId && (
              <div className="rounded-md border border-warning/20 bg-warning/10 text-warning p-2 text-xs">
                Ventas desde RD$250,000 requieren RNC o Cédula del cliente — selecciona uno arriba.
              </div>
            )}

            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{error}</div>}

            <Button
              className="w-full"
              size="lg"
              disabled={carrito.length === 0 || cobrando || cajaAbierta === false || (metodoPago === "FIADO" && !clienteId)}
              onClick={() => handleCobrar()}
            >
              {cobrando ? "Procesando..." : `Cobrar ${formatDOP(totals.total)}`}
            </Button>
          </div>
        </CardContent>
      </Card>

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
                  onKeyDown={(e) => e.key === "Enter" && adminEmail && adminPassword && handleCobrar({ email: adminEmail, password: adminPassword })}
                />
              </div>
              {aprobacionError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{aprobacionError}</div>}
              <div className="flex gap-2">
                <Button
                  className="flex-1"
                  disabled={cobrando || !adminEmail || !adminPassword}
                  onClick={() => handleCobrar({ email: adminEmail, password: adminPassword })}
                >
                  {cobrando ? "Verificando..." : "Aprobar y cobrar"}
                </Button>
                <Button
                  variant="secondary"
                  onClick={() => {
                    setMostrarAprobacion(false);
                    setAdminEmail("");
                    setAdminPassword("");
                    setAprobacionError("");
                  }}
                >
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

const TIPO_ECF_LABEL: Record<number, string> = {
  31: "Factura de Crédito Fiscal Electrónica",
  32: "Factura de Consumo Electrónica",
  33: "Nota de Débito Electrónica",
  34: "Nota de Crédito Electrónica",
};

function VentaConfirmada({
  venta,
  cliente,
  status,
  onStatusChange,
  onVentaChange,
  onNuevaVenta,
}: {
  venta: VentaResult;
  cliente?: Cliente;
  status: { ecfError?: string; impreso: boolean; printError?: string };
  onStatusChange: (s: { ecfError?: string; impreso: boolean; printError?: string }) => void;
  onVentaChange: (v: VentaResult) => void;
  onNuevaVenta: () => void;
}) {
  const [tenant, setTenant] = useState<{ razon_social?: string; rnc?: string; direccion?: string; telefono?: string; factura_electronica_activa?: boolean } | null>(null);
  const facturaElectronicaActiva = tenant?.factura_electronica_activa !== false;
  const [showEmitir, setShowEmitir] = useState(false);
  const [p12Password, setP12Password] = useState("");
  const [p12File, setP12File] = useState<File | null>(null);
  const [sendToDgii, setSendToDgii] = useState(false);
  const [emitiendo, setEmitiendo] = useState(false);
  const [emitirError, setEmitirError] = useState("");
  const [reimprimiendo, setReimprimiendo] = useState(false);

  useEffect(() => {
    try {
      const raw = localStorage.getItem("tenant");
      if (raw) setTenant(JSON.parse(raw));
    } catch {}
  }, []);

  async function handleEmitir(e: React.FormEvent) {
    e.preventDefault();
    if (!p12File) return;
    setEmitiendo(true);
    setEmitirError("");
    try {
      const buf = await p12File.arrayBuffer();
      const p12Base64 = btoa(String.fromCharCode(...new Uint8Array(buf)));
      const result = await apiFetch<VentaResult>(`/api/ventas/${venta.id}/emitir-ecf`, {
        method: "POST",
        body: JSON.stringify({ p12Base64, p12Password: p12Password || undefined, sendToDgii }),
      });
      result.items = venta.items;
      onVentaChange(result);
      onStatusChange({ ...status, ecfError: undefined });
      setShowEmitir(false);
    } catch (e: any) {
      setEmitirError(e.message);
    } finally {
      setEmitiendo(false);
    }
  }

  async function handleReimprimir() {
    setReimprimiendo(true);
    try {
      await apiFetch(`/api/ventas/${venta.id}/imprimir`, { method: "POST" });
      onStatusChange({ ...status, impreso: true, printError: undefined });
    } catch (e: any) {
      onStatusChange({ ...status, impreso: false, printError: e.message });
    } finally {
      setReimprimiendo(false);
    }
  }

  const fechaFirma = new Date(venta.created_at).toLocaleString("es-DO", { dateStyle: "short", timeStyle: "medium" });

  return (
    <div className="max-w-md mx-auto mt-6 space-y-4 pb-10">
      <div className="text-center space-y-1">
        <div className="h-12 w-12 rounded-full bg-primary/10 text-primary flex items-center justify-center mx-auto">
          <FileCheck className="h-6 w-6" />
        </div>
        <h1 className="text-lg font-bold">Venta registrada</h1>
      </div>

      <div className="flex items-center justify-center gap-2">
        {facturaElectronicaActiva && (venta.e_ncf ? (
          <Badge variant={
            venta.estado_dgii === "ACEPTADO" ? "success"
              : venta.estado_dgii === "RECHAZADO" ? "destructive"
              : venta.estado_dgii === "CONTINGENCIA_PENDIENTE" ? "warning"
              : "secondary"
          }>
            e-CF {venta.estado_dgii === "CONTINGENCIA_PENDIENTE" ? "pendiente de envío" : venta.estado_dgii}
          </Badge>
        ) : (
          <Badge variant="secondary">Sin e-CF</Badge>
        ))}
        {status.impreso ? (
          <Badge variant="success"><Printer className="h-3 w-3 mr-1 inline" />Impreso</Badge>
        ) : (
          <Badge variant="destructive"><Printer className="h-3 w-3 mr-1 inline" />No impreso</Badge>
        )}
      </div>

      {/* Vista previa del recibo, con el mismo formato de una factura de consumo electrónica dominicana */}
      <Card className="font-mono text-xs">
        <CardContent className="pt-5 space-y-2">
          <div className="text-center space-y-0.5">
            <p className="font-bold text-sm">{tenant?.razon_social || "Mi negocio"}</p>
            <p>RNC: {tenant?.rnc || "—"}</p>
            {tenant?.direccion && <p>{tenant.direccion}</p>}
            {tenant?.telefono && <p>Tel: {tenant.telefono}</p>}
          </div>
          <div className="border-t border-dashed border-border" />
          {venta.e_ncf ? (
            <div className="space-y-0.5">
              <p className="text-center font-bold">{TIPO_ECF_LABEL[venta.tipo_ecf ?? 32] ?? "Comprobante Fiscal Electrónico"}</p>
              <div className="flex justify-between"><span>NCF:</span><span>{venta.e_ncf}</span></div>
              <div className="flex justify-between"><span>Fecha:</span><span>{fechaFirma}</span></div>
              <div className="flex justify-between"><span>Estado DGII:</span><span>{venta.estado_dgii}</span></div>
            </div>
          ) : (
            <p className="text-center font-bold">{facturaElectronicaActiva ? "Ticket de venta (sin e-CF)" : "Ticket de venta"}</p>
          )}
          <div className="flex justify-between"><span>Método de pago:</span><span>{venta.metodo_pago}</span></div>
          <div className="border-t border-dashed border-border" />
          <div>
            <p>Cliente: {cliente?.nombre || "Consumidor final"}</p>
            {cliente?.rnc_cedula && <p>RNC/Cédula: {cliente.rnc_cedula}</p>}
          </div>
          <div className="border-t border-dashed border-border" />
          <div className="space-y-1">
            {venta.items?.map((it, i) => (
              <div key={i} className="flex justify-between">
                <span className="truncate pr-2">{it.nombre} x{it.cantidad}</span>
                <span className="tabular-nums shrink-0">{formatDOP(it.subtotal)}</span>
              </div>
            ))}
          </div>
          <div className="border-t border-dashed border-border" />
          <div className="flex justify-between"><span>Subtotal:</span><span className="tabular-nums">{formatDOP(venta.subtotal)}</span></div>
          <div className="flex justify-between"><span>ITBIS:</span><span className="tabular-nums">{formatDOP(venta.itbis_total)}</span></div>
          <div className="flex justify-between font-bold text-sm pt-1"><span>TOTAL RD$:</span><span className="tabular-nums">{formatDOP(venta.total)}</span></div>
          {venta.codigo_seguridad && (
            <>
              <div className="border-t border-dashed border-border" />
              <div className="text-center space-y-0.5">
                <p>Código de Seguridad: {venta.codigo_seguridad}</p>
                <p className="text-[10px] text-muted-foreground">El código QR de verificación DGII se imprime en el ticket físico.</p>
              </div>
            </>
          )}
        </CardContent>
      </Card>

      {facturaElectronicaActiva && status.ecfError && (
        <div className="rounded-md border border-warning/20 bg-warning/10 text-warning p-2 text-xs flex items-start gap-2">
          <XCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
          <span>No se emitió e-CF automáticamente: {status.ecfError}. Puedes emitirlo manualmente abajo, o configura un certificado en Configuración → DGII para que se emita solo la próxima vez.</span>
        </div>
      )}
      {status.printError && (
        <div className="rounded-md border border-warning/20 bg-warning/10 text-warning p-2 text-xs flex items-start gap-2">
          <XCircle className="h-3.5 w-3.5 shrink-0 mt-0.5" />
          <span>No se pudo imprimir: {status.printError}</span>
        </div>
      )}
      {status.impreso && (
        <div className="rounded-md border border-success/20 bg-success/10 text-success p-2 text-xs flex items-center gap-2">
          <CheckCircle2 className="h-3.5 w-3.5 shrink-0" />
          <span>Ticket enviado a la impresora configurada.</span>
        </div>
      )}

      <Button variant="secondary" className="w-full" onClick={handleReimprimir} disabled={reimprimiendo}>
        <Printer className="h-4 w-4" />{reimprimiendo ? "Imprimiendo..." : "Reimprimir ticket"}
      </Button>

      {facturaElectronicaActiva && !venta.e_ncf && !showEmitir && (
        <Button variant="secondary" className="w-full" onClick={() => setShowEmitir(true)}>Emitir e-CF con certificado</Button>
      )}

      {facturaElectronicaActiva && showEmitir && (
        <Card>
          <CardContent className="pt-5">
            <form onSubmit={handleEmitir} className="space-y-3">
              <div className="space-y-1.5">
                <Label htmlFor="p12">Certificado P12</Label>
                <input id="p12" type="file" accept=".p12,.pfx" onChange={(e) => setP12File(e.target.files?.[0] || null)} className="text-sm" required />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="p12pass">Contraseña del certificado</Label>
                <Input id="p12pass" type="password" value={p12Password} onChange={(e) => setP12Password(e.target.value)} />
              </div>
              <label className="flex items-center gap-2 text-sm">
                <input type="checkbox" checked={sendToDgii} onChange={(e) => setSendToDgii(e.target.checked)} />
                Enviar a DGII (si no, solo firma)
              </label>
              {emitirError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{emitirError}</div>}
              <Button type="submit" disabled={emitiendo} className="w-full">{emitiendo ? "Firmando..." : "Firmar y emitir"}</Button>
            </form>
          </CardContent>
        </Card>
      )}

      <Button className="w-full" onClick={onNuevaVenta}>Nueva venta</Button>
    </div>
  );
}

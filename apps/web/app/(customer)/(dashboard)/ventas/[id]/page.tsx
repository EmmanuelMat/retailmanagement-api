"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { Printer, Undo2, Truck, FileText } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface VentaItem {
  id: string;
  sku: string;
  nombre: string;
  cantidad: string;
  precio_unitario: string;
  itbis_monto: string;
  subtotal: string;
  cantidad_entregada: string;
}

interface VentaDetalle {
  id: string;
  subtotal: string;
  itbis_total: string;
  total: string;
  metodo_pago: string;
  estado: string;
  e_ncf: string | null;
  estado_dgii: string | null;
  qr_url: string | null;
  codigo_seguridad: string | null;
  entrega_diferida: boolean;
  created_at: string;
  items: VentaItem[];
}

interface ConduceResumen {
  id: string;
  direccion_entrega: string | null;
  created_at: string;
}

interface NotaCredito {
  id: string;
  e_ncf: string | null;
  estado_dgii: string | null;
  total: string;
}

function estadoDgiiVariant(estado: string | null): "success" | "destructive" | "warning" | "secondary" {
  if (estado === "ACEPTADO") return "success";
  if (estado === "RECHAZADO") return "destructive";
  if (estado === "CONTINGENCIA_PENDIENTE") return "warning";
  return "secondary";
}

export default function VentaDetallePage() {
  const params = useParams<{ id: string }>();
  const [venta, setVenta] = useState<VentaDetalle | null>(null);
  const [error, setError] = useState("");
  const [imprimiendo, setImprimiendo] = useState(false);
  const [printMsg, setPrintMsg] = useState("");
  const [showNotaCredito, setShowNotaCredito] = useState(false);
  const [motivo, setMotivo] = useState("");
  const [emitiendoNota, setEmitiendoNota] = useState(false);
  const [notaError, setNotaError] = useState("");
  const [nota, setNota] = useState<NotaCredito | null>(null);

  const [conduces, setConduces] = useState<ConduceResumen[]>([]);
  const [cantidadesEntrega, setCantidadesEntrega] = useState<Record<string, string>>({});
  const [observacionesEntrega, setObservacionesEntrega] = useState<Record<string, string>>({});
  const [direccionEntrega, setDireccionEntrega] = useState("");
  const [ordenCompra, setOrdenCompra] = useState("");
  const [vehiculoPlaca, setVehiculoPlaca] = useState("");
  const [conductor, setConductor] = useState("");
  const [entregadoPor, setEntregadoPor] = useState("");
  const [recibidoPor, setRecibidoPor] = useState("");
  const [notasEntrega, setNotasEntrega] = useState("");
  const [registrandoEntrega, setRegistrandoEntrega] = useState(false);
  const [entregaError, setEntregaError] = useState("");

  function cargarVenta() {
    apiFetch<VentaDetalle>(`/api/ventas/${params.id}`).then(setVenta).catch((e) => setError(e.message));
  }

  function cargarConduces() {
    apiFetch<{ items: ConduceResumen[] }>(`/api/conduces?venta_id=${params.id}&pageSize=100`).then((d) => setConduces(d.items)).catch(() => {});
  }

  useEffect(() => {
    cargarVenta();
    cargarConduces();
  }, [params.id]);

  async function handleRegistrarEntrega(e: React.FormEvent) {
    e.preventDefault();
    if (!venta) return;
    const items = venta.items
      .map((it) => ({
        venta_item_id: it.id,
        cantidad: cantidadesEntrega[it.id] || "",
        observaciones: observacionesEntrega[it.id] || undefined,
      }))
      .filter((it) => Number(it.cantidad) > 0);
    if (items.length === 0) {
      setEntregaError("Indica la cantidad a entregar de al menos un producto.");
      return;
    }
    setRegistrandoEntrega(true);
    setEntregaError("");
    try {
      await apiFetch("/api/conduces", {
        method: "POST",
        body: JSON.stringify({
          venta_id: venta.id,
          direccion_entrega: direccionEntrega || undefined,
          orden_compra: ordenCompra || undefined,
          vehiculo_placa: vehiculoPlaca || undefined,
          conductor: conductor || undefined,
          entregado_por: entregadoPor || undefined,
          recibido_por: recibidoPor || undefined,
          notas: notasEntrega || undefined,
          items,
        }),
      });
      setCantidadesEntrega({});
      setObservacionesEntrega({});
      setOrdenCompra("");
      setVehiculoPlaca("");
      setConductor("");
      setEntregadoPor("");
      setRecibidoPor("");
      setNotasEntrega("");
      cargarVenta();
      cargarConduces();
    } catch (e: any) {
      setEntregaError(e.message);
    } finally {
      setRegistrandoEntrega(false);
    }
  }

  async function handleReimprimir() {
    setImprimiendo(true);
    setPrintMsg("");
    try {
      await apiFetch(`/api/ventas/${params.id}/imprimir`, { method: "POST" });
      setPrintMsg("Ticket enviado a la impresora.");
    } catch (e: any) {
      setPrintMsg(e.message);
    } finally {
      setImprimiendo(false);
    }
  }

  async function handleEmitirNotaCredito(e: React.FormEvent) {
    e.preventDefault();
    setEmitiendoNota(true);
    setNotaError("");
    try {
      const result = await apiFetch<NotaCredito>(`/api/ventas/${params.id}/nota-credito`, {
        method: "POST",
        body: JSON.stringify({ motivo }),
      });
      setNota(result);
      setShowNotaCredito(false);
      cargarVenta();
    } catch (e: any) {
      setNotaError(e.message);
    } finally {
      setEmitiendoNota(false);
    }
  }

  if (error) return <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>;
  if (!venta) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <div className="space-y-6 max-w-3xl">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Venta</h1>
          <p className="text-sm text-muted-foreground mt-1">{new Date(venta.created_at).toLocaleString("es-DO")}</p>
        </div>
        <div className="text-right space-y-2">
          {venta.e_ncf ? (
            <div>
              <p className="font-mono text-sm">{venta.e_ncf}</p>
              <Badge variant={estadoDgiiVariant(venta.estado_dgii)} className="mt-1">
                {venta.estado_dgii === "CONTINGENCIA_PENDIENTE" ? "pendiente de envío" : venta.estado_dgii}
              </Badge>
            </div>
          ) : (
            <Badge variant="secondary">Sin e-CF emitido</Badge>
          )}
          {venta.estado === "ANULADA" && <Badge variant="destructive" className="mt-1">ANULADA</Badge>}
          <div className="flex justify-end gap-2">
            <Link href={`/imprimir/venta/${venta.id}` as any} target="_blank">
              <Button size="sm" variant="secondary">
                <FileText className="h-3.5 w-3.5" />Imprimir factura
              </Button>
            </Link>
            <Button size="sm" variant="secondary" onClick={handleReimprimir} disabled={imprimiendo}>
              <Printer className="h-3.5 w-3.5" />{imprimiendo ? "Imprimiendo..." : "Reimprimir"}
            </Button>
            {venta.e_ncf && venta.estado !== "ANULADA" && !nota && (
              <Button size="sm" variant="secondary" onClick={() => setShowNotaCredito((s) => !s)}>
                <Undo2 className="h-3.5 w-3.5" />Nota de Crédito
              </Button>
            )}
          </div>
        </div>
      </div>

      {printMsg && <p className="text-xs text-muted-foreground">{printMsg}</p>}

      {showNotaCredito && (
        <Card className="max-w-md ml-auto">
          <CardContent className="pt-5">
            <form onSubmit={handleEmitirNotaCredito} className="space-y-3">
              <div className="space-y-1.5">
                <Label htmlFor="motivo">Motivo de la devolución *</Label>
                <Input id="motivo" required value={motivo} onChange={(e) => setMotivo(e.target.value)} placeholder="Producto defectuoso, cambio de opinión, etc." />
              </div>
              <p className="text-xs text-muted-foreground">
                Revierte el stock y la caja de esta venta y emite una Nota de Crédito (e-CF Tipo 34) que referencia el e-NCF original. La venta original queda ANULADA pero su historial permanece intacto.
              </p>
              {notaError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{notaError}</div>}
              <div className="flex gap-2">
                <Button type="submit" disabled={emitiendoNota}>{emitiendoNota ? "Emitiendo..." : "Emitir Nota de Crédito"}</Button>
                <Button type="button" variant="secondary" onClick={() => setShowNotaCredito(false)}>Cancelar</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      {nota && (
        <Card className="max-w-md ml-auto">
          <CardContent className="pt-5 space-y-2">
            <p className="text-sm font-semibold">Nota de Crédito emitida</p>
            <p className="font-mono text-sm">{nota.e_ncf}</p>
            <Badge variant={estadoDgiiVariant(nota.estado_dgii)}>
              {nota.estado_dgii === "CONTINGENCIA_PENDIENTE" ? "pendiente de envío" : nota.estado_dgii}
            </Badge>
            <p className="text-sm tabular-nums">Monto: {formatDOP(nota.total)}</p>
          </CardContent>
        </Card>
      )}

      <Card>
        <CardContent className="p-0">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>SKU</TableHead>
                <TableHead>Producto</TableHead>
                <TableHead className="text-right">Cant.</TableHead>
                <TableHead className="text-right">Precio</TableHead>
                <TableHead className="text-right">ITBIS</TableHead>
                <TableHead className="text-right">Subtotal</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {venta.items.map((it) => (
                <TableRow key={it.id}>
                  <TableCell className="font-mono text-xs text-muted-foreground">{it.sku}</TableCell>
                  <TableCell className="font-medium">{it.nombre}</TableCell>
                  <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
                  <TableCell className="text-right tabular-nums">{formatDOP(it.precio_unitario)}</TableCell>
                  <TableCell className="text-right tabular-nums">{formatDOP(it.itbis_monto)}</TableCell>
                  <TableCell className="text-right tabular-nums font-medium">{formatDOP(it.subtotal)}</TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-5 space-y-1 text-sm max-w-xs ml-auto">
          <div className="flex justify-between text-muted-foreground"><span>Subtotal</span><span className="tabular-nums">{formatDOP(venta.subtotal)}</span></div>
          <div className="flex justify-between text-muted-foreground"><span>ITBIS</span><span className="tabular-nums">{formatDOP(venta.itbis_total)}</span></div>
          <div className="flex justify-between font-bold text-base pt-1 border-t border-border mt-1"><span>Total</span><span className="tabular-nums">{formatDOP(venta.total)}</span></div>
          <p className="text-xs text-muted-foreground pt-2">Pago: {venta.metodo_pago}</p>
          {venta.qr_url && <p className="text-xs text-muted-foreground break-all pt-2">{venta.qr_url}</p>}
        </CardContent>
      </Card>

      {venta.entrega_diferida && (
        <Card>
          <CardContent className="pt-5 space-y-4">
            <div className="flex items-center gap-2">
              <Truck className="h-4 w-4 text-primary" />
              <h2 className="font-bold text-sm">Entregas</h2>
            </div>
            <p className="text-xs text-muted-foreground -mt-2">
              Esta venta es de entrega diferida — la mercancía sale en partes. El stock se descuenta según se registre cada entrega, no al facturar.
            </p>

            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Producto</TableHead>
                  <TableHead className="text-right">Cant.</TableHead>
                  <TableHead className="text-right">Entregado</TableHead>
                  <TableHead className="text-right">Pendiente</TableHead>
                  <TableHead className="text-right">Entregar ahora</TableHead>
                  <TableHead>Observaciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {venta.items.map((it) => {
                  const pendiente = Number(it.cantidad) - Number(it.cantidad_entregada);
                  return (
                    <TableRow key={it.id}>
                      <TableCell className="font-medium">{it.nombre}</TableCell>
                      <TableCell className="text-right tabular-nums">{it.cantidad}</TableCell>
                      <TableCell className="text-right tabular-nums">{it.cantidad_entregada}</TableCell>
                      <TableCell className="text-right tabular-nums">{pendiente}</TableCell>
                      <TableCell className="text-right">
                        {pendiente > 0 ? (
                          <Input
                            type="number"
                            min={0}
                            max={pendiente}
                            step="0.01"
                            className="h-8 w-24 ml-auto text-right"
                            value={cantidadesEntrega[it.id] || ""}
                            onChange={(e) => setCantidadesEntrega((c) => ({ ...c, [it.id]: e.target.value }))}
                            placeholder="0"
                          />
                        ) : (
                          <span className="text-xs text-muted-foreground">Completo</span>
                        )}
                      </TableCell>
                      <TableCell>
                        {pendiente > 0 && (
                          <Input
                            className="h-8 w-32"
                            value={observacionesEntrega[it.id] || ""}
                            onChange={(e) => setObservacionesEntrega((o) => ({ ...o, [it.id]: e.target.value }))}
                            placeholder="Estado, daños..."
                          />
                        )}
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>

            {venta.items.some((it) => Number(it.cantidad) - Number(it.cantidad_entregada) > 0) && (
              <form onSubmit={handleRegistrarEntrega} className="space-y-3 max-w-md">
                <div className="space-y-1.5">
                  <Label htmlFor="direccionEntrega">Dirección de entrega</Label>
                  <Input id="direccionEntrega" value={direccionEntrega} onChange={(e) => setDireccionEntrega(e.target.value)} placeholder="Calle, número, sector..." />
                </div>
                <div className="grid grid-cols-2 gap-3">
                  <div className="space-y-1.5">
                    <Label htmlFor="ordenCompra">Orden de compra</Label>
                    <Input id="ordenCompra" value={ordenCompra} onChange={(e) => setOrdenCompra(e.target.value)} placeholder="Opcional" />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="vehiculoPlaca">Vehículo / Placa</Label>
                    <Input id="vehiculoPlaca" value={vehiculoPlaca} onChange={(e) => setVehiculoPlaca(e.target.value)} placeholder="Camión, ABC-1234" />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="conductor">Conductor</Label>
                    <Input id="conductor" value={conductor} onChange={(e) => setConductor(e.target.value)} placeholder="Nombre y cédula" />
                  </div>
                  <div />
                  <div className="space-y-1.5">
                    <Label htmlFor="entregadoPor">Entregado por</Label>
                    <Input id="entregadoPor" value={entregadoPor} onChange={(e) => setEntregadoPor(e.target.value)} placeholder="Nombre y cédula" />
                  </div>
                  <div className="space-y-1.5">
                    <Label htmlFor="recibidoPor">Recibido por</Label>
                    <Input id="recibidoPor" value={recibidoPor} onChange={(e) => setRecibidoPor(e.target.value)} placeholder="Nombre y cédula" />
                  </div>
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="notasEntrega">Notas de entrega</Label>
                  <Input id="notasEntrega" value={notasEntrega} onChange={(e) => setNotasEntrega(e.target.value)} placeholder="Opcional" />
                </div>
                {entregaError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{entregaError}</div>}
                <Button type="submit" disabled={registrandoEntrega}>{registrandoEntrega ? "Registrando..." : "Registrar entrega"}</Button>
              </form>
            )}

            {conduces.length > 0 && (
              <div className="space-y-1.5 pt-2 border-t border-border">
                <p className="text-xs font-medium text-muted-foreground pt-2">Historial de entregas</p>
                <Table>
                  <TableHeader>
                    <TableRow>
                      <TableHead>Fecha</TableHead>
                      <TableHead>Dirección</TableHead>
                    </TableRow>
                  </TableHeader>
                  <TableBody>
                    {conduces.map((c) => (
                      <TableRow key={c.id}>
                        <TableCell className="text-xs text-muted-foreground">
                          <Link href={`/conduces/${c.id}` as any} className="hover:text-primary">{new Date(c.created_at).toLocaleString("es-DO")}</Link>
                        </TableCell>
                        <TableCell className="text-muted-foreground">{c.direccion_entrega || "—"}</TableCell>
                      </TableRow>
                    ))}
                  </TableBody>
                </Table>
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

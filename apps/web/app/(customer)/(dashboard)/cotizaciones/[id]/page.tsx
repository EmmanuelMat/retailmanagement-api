"use client";

import { useEffect, useState } from "react";
import { useParams, useRouter } from "next/navigation";
import Link from "next/link";
import { FileText } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Select, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch, ApiError } from "@/lib/api";

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

interface CotizacionDetalle {
  id: string;
  subtotal: string;
  itbis_total: string;
  total: string;
  estado: string;
  fecha_vencimiento: string | null;
  venta_id: string | null;
  created_at: string;
  items: CotizacionItem[];
}

const ESTADO_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  PENDIENTE: "warning",
  ACEPTADA: "default",
  CONVERTIDA: "success",
  RECHAZADA: "destructive",
  VENCIDA: "secondary",
};

export default function CotizacionDetallePage() {
  const params = useParams<{ id: string }>();
  const router = useRouter();
  const [cotizacion, setCotizacion] = useState<CotizacionDetalle | null>(null);
  const [error, setError] = useState("");
  const [rechazando, setRechazando] = useState(false);

  const [metodoPago, setMetodoPago] = useState("EFECTIVO");
  const [convirtiendo, setConvirtiendo] = useState(false);
  const [convertirError, setConvertirError] = useState("");

  const [mostrarAprobacion, setMostrarAprobacion] = useState(false);
  const [aprobacionMensaje, setAprobacionMensaje] = useState("");
  const [adminEmail, setAdminEmail] = useState("");
  const [adminPassword, setAdminPassword] = useState("");

  function load() {
    apiFetch<CotizacionDetalle>(`/api/cotizaciones/${params.id}`).then(setCotizacion).catch((e) => setError(e.message));
  }

  useEffect(() => { load(); }, [params.id]);

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

  async function handleConvertir(adminCreds?: { email: string; password: string }) {
    setConvirtiendo(true);
    setConvertirError("");
    try {
      const venta = await apiFetch<{ id: string }>(`/api/cotizaciones/${params.id}/convertir`, {
        method: "POST",
        body: JSON.stringify({ metodo_pago: metodoPago, aprobacion_admin: adminCreds }),
      });
      setMostrarAprobacion(false);
      router.push(`/ventas/${venta.id}` as any);
    } catch (e: any) {
      if (e instanceof ApiError && e.status === 403 && e.message.startsWith("DESCUENTO_REQUIERE_APROBACION")) {
        setAprobacionMensaje(e.message);
        setMostrarAprobacion(true);
      } else if (adminCreds) {
        setConvertirError(e.message);
      } else {
        setConvertirError(e.message);
      }
    } finally {
      setConvirtiendo(false);
    }
  }

  if (error) return <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-xl">{error}</div>;
  if (!cotizacion) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  const puedeConvertir = cotizacion.estado === "PENDIENTE" || cotizacion.estado === "ACEPTADA";

  return (
    <div className="space-y-6 max-w-3xl">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Cotización</h1>
          <p className="text-sm text-muted-foreground mt-1">{new Date(cotizacion.created_at).toLocaleString("es-DO")}</p>
        </div>
        <div className="text-right space-y-2">
          <Badge variant={ESTADO_VARIANT[cotizacion.estado] || "default"}>{cotizacion.estado}</Badge>
          <div className="flex justify-end gap-2">
            <Link href={`/imprimir/cotizacion/${cotizacion.id}` as any} target="_blank">
              <Button size="sm" variant="secondary"><FileText className="h-3.5 w-3.5" />Imprimir</Button>
            </Link>
            {puedeConvertir && (
              <Button size="sm" variant="secondary" onClick={handleRechazar} disabled={rechazando}>Rechazar</Button>
            )}
          </div>
        </div>
      </div>

      {cotizacion.venta_id && (
        <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">
          Convertida en venta — <a href={`/ventas/${cotizacion.venta_id}`} className="underline font-medium">ver venta</a>.
        </div>
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
                <TableHead className="text-right">Descuento</TableHead>
                <TableHead className="text-right">Subtotal</TableHead>
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
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-5 space-y-1 text-sm max-w-xs ml-auto">
          <div className="flex justify-between text-muted-foreground"><span>Subtotal</span><span className="tabular-nums">{formatDOP(cotizacion.subtotal)}</span></div>
          <div className="flex justify-between text-muted-foreground"><span>ITBIS</span><span className="tabular-nums">{formatDOP(cotizacion.itbis_total)}</span></div>
          <div className="flex justify-between font-bold text-base pt-1 border-t border-border mt-1"><span>Total</span><span className="tabular-nums">{formatDOP(cotizacion.total)}</span></div>
        </CardContent>
      </Card>

      {puedeConvertir && (
        <Card>
          <CardContent className="pt-5 space-y-3 max-w-xs ml-auto">
            <p className="text-sm font-semibold">Convertir a venta</p>
            <div className="space-y-1.5">
              <Label htmlFor="metodo">Método de pago</Label>
              <Select id="metodo" value={metodoPago} onChange={(e) => setMetodoPago(e.target.value)}>
                <option value="EFECTIVO">Efectivo</option>
                <option value="TARJETA">Tarjeta</option>
                <option value="TRANSFERENCIA">Transferencia</option>
                <option value="FIADO">Fiado</option>
              </Select>
            </div>
            {convertirError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-2 text-xs">{convertirError}</div>}
            <Button className="w-full" onClick={() => handleConvertir()} disabled={convirtiendo}>
              {convirtiendo ? "Procesando..." : "Convertir a venta"}
            </Button>
          </CardContent>
        </Card>
      )}

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

"use client";

import { useEffect, useState } from "react";
import { Lock, Unlock, Wallet } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface CajaSesion {
  id: string;
  monto_inicial: string;
  monto_final: string | null;
  monto_esperado: string | null;
  diferencia: string | null;
  estado: string;
  abierta_at: string;
}

interface CajaMovimiento {
  id: string;
  tipo: "INGRESO" | "EGRESO";
  concepto: string;
  monto: string;
  metodo_pago: string | null;
  created_at: string;
}

interface CajaResumen {
  sesion: CajaSesion | null;
  ingresos: string;
  egresos: string;
  saldo_actual: string;
}

export default function CajaPage() {
  const [resumen, setResumen] = useState<CajaResumen | null>(null);
  const [movimientos, setMovimientos] = useState<CajaMovimiento[]>([]);
  const [montoInicial, setMontoInicial] = useState("");
  const [montoFinal, setMontoFinal] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [cierreResultado, setCierreResultado] = useState<CajaSesion | null>(null);

  async function load() {
    try {
      const [r, m] = await Promise.all([
        apiFetch<CajaResumen>("/api/caja/resumen"),
        apiFetch<{ movimientos: CajaMovimiento[] }>("/api/caja/movimientos"),
      ]);
      setResumen(r);
      setMovimientos(m.movimientos.slice(0, 20));
    } catch (e: any) {
      setError(e.message);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleAbrir(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/caja/abrir", { method: "POST", body: JSON.stringify({ monto_inicial: montoInicial || "0" }) });
      setMontoInicial("");
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleCerrar(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      const result = await apiFetch<CajaSesion>("/api/caja/cerrar", { method: "POST", body: JSON.stringify({ monto_final: montoFinal || "0" }) });
      setCierreResultado(result);
      setMontoFinal("");
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  if (!resumen) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Caja</h1>
        <p className="text-sm text-muted-foreground mt-1">Apertura, movimientos del turno y cierre.</p>
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      {cierreResultado && (
        <Card>
          <CardContent className="pt-5">
            <h2 className="font-bold text-sm mb-2">Caja cerrada</h2>
            <div className="text-sm space-y-1 text-muted-foreground">
              <p>Esperado: {formatDOP(cierreResultado.monto_esperado || "0")}</p>
              <p>Contado: {formatDOP(cierreResultado.monto_final || "0")}</p>
              <p className={Number(cierreResultado.diferencia) !== 0 ? "text-destructive font-medium" : "text-success font-medium"}>
                Diferencia: {formatDOP(cierreResultado.diferencia || "0")}
              </p>
            </div>
          </CardContent>
        </Card>
      )}

      {!resumen.sesion ? (
        <Card className="max-w-sm">
          <CardContent className="pt-5">
            <div className="flex items-center gap-2 mb-3">
              <Unlock className="h-4 w-4 text-primary" />
              <h2 className="font-bold text-sm">Abrir caja</h2>
            </div>
            <form onSubmit={handleAbrir} className="space-y-3">
              <div className="space-y-1.5">
                <Label htmlFor="inicial">Monto inicial en caja</Label>
                <Input id="inicial" type="number" step="0.01" value={montoInicial} onChange={(e) => setMontoInicial(e.target.value)} placeholder="0.00" required />
              </div>
              <Button type="submit" disabled={saving} className="w-full">Abrir turno</Button>
            </form>
          </CardContent>
        </Card>
      ) : (
        <>
          <div className="grid grid-cols-1 sm:grid-cols-4 gap-4">
            <Card>
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Monto inicial</p>
                <p className="text-xl font-bold mt-1">{formatDOP(resumen.sesion.monto_inicial)}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Ingresos</p>
                <p className="text-xl font-bold mt-1 text-success">{formatDOP(resumen.ingresos)}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Egresos</p>
                <p className="text-xl font-bold mt-1 text-destructive">{formatDOP(resumen.egresos)}</p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Saldo esperado</p>
                <p className="text-xl font-bold mt-1">{formatDOP(resumen.saldo_actual)}</p>
              </CardContent>
            </Card>
          </div>

          <Card className="max-w-sm">
            <CardContent className="pt-5">
              <div className="flex items-center gap-2 mb-3">
                <Lock className="h-4 w-4 text-primary" />
                <h2 className="font-bold text-sm">Cerrar caja</h2>
              </div>
              <form onSubmit={handleCerrar} className="space-y-3">
                <div className="space-y-1.5">
                  <Label htmlFor="final">Monto contado en caja</Label>
                  <Input id="final" type="number" step="0.01" value={montoFinal} onChange={(e) => setMontoFinal(e.target.value)} placeholder="0.00" required />
                </div>
                <Button type="submit" variant="secondary" disabled={saving} className="w-full">Cerrar turno</Button>
              </form>
            </CardContent>
          </Card>
        </>
      )}

      <Card>
        <CardContent className="p-0">
          {movimientos.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <Wallet className="h-6 w-6" />
              Sin movimientos de caja todavía.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Concepto</TableHead>
                  <TableHead>Tipo</TableHead>
                  <TableHead className="text-right">Monto</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {movimientos.map((m) => (
                  <TableRow key={m.id}>
                    <TableCell className="text-xs text-muted-foreground">{new Date(m.created_at).toLocaleString("es-DO")}</TableCell>
                    <TableCell>{m.concepto}</TableCell>
                    <TableCell><Badge variant={m.tipo === "INGRESO" ? "success" : "destructive"}>{m.tipo}</Badge></TableCell>
                    <TableCell className="text-right font-mono tabular-nums">{formatDOP(m.monto)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

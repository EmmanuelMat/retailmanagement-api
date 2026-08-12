"use client";

import { Suspense, useEffect, useState } from "react";
import { Lock, Unlock, Wallet } from "lucide-react";
import {
  Badge, Button, Card, CardContent, Input, Label, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";

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

interface MovimientosFilters {
  tipo?: string;
}

export default function CajaPage() {
  return (
    <Suspense fallback={null}>
      <CajaPageContent />
    </Suspense>
  );
}

function CajaPageContent() {
  const [resumen, setResumen] = useState<CajaResumen | null>(null);
  const [montoInicial, setMontoInicial] = useState("");
  const [montoFinal, setMontoFinal] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [cierreResultado, setCierreResultado] = useState<CajaSesion | null>(null);

  const {
    items: movimientos,
    total,
    totalPages,
    loading: movimientosLoading,
    error: movimientosError,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
    refresh: refreshMovimientos,
  } = useServerTable<CajaMovimiento, MovimientosFilters>({
    path: "/api/caja/movimientos",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  async function loadResumen() {
    try {
      const r = await apiFetch<CajaResumen>("/api/caja/resumen");
      setResumen(r);
    } catch (e: any) {
      setError(e.message);
    }
  }

  useEffect(() => {
    loadResumen();
  }, []);

  async function handleAbrir(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/caja/abrir", { method: "POST", body: JSON.stringify({ monto_inicial: montoInicial || "0" }) });
      setMontoInicial("");
      await loadResumen();
      refreshMovimientos();
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
      await loadResumen();
      refreshMovimientos();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  if (!resumen) return <p className="text-sm text-muted-foreground">Cargando...</p>;

  return (
    <div className="space-y-4">
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
              <p data-testid="caja-cierre-esperado">Esperado: {formatDOP(cierreResultado.monto_esperado || "0")}</p>
              <p data-testid="caja-cierre-contado">Contado: {formatDOP(cierreResultado.monto_final || "0")}</p>
              <p
                data-testid="caja-cierre-diferencia"
                className={Number(cierreResultado.diferencia) !== 0 ? "text-destructive font-medium" : "text-success font-medium"}
              >
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
              <Button type="submit" disabled={saving} className="w-full" data-testid="caja-abrir-submit">Abrir turno</Button>
            </form>
          </CardContent>
        </Card>
      ) : (
        <>
          <div className="grid grid-cols-1 sm:grid-cols-4 gap-4">
            <Card className="h-full">
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Monto inicial</p>
                <p className="text-xl font-bold mt-1" data-testid="caja-monto-inicial">{formatDOP(resumen.sesion.monto_inicial)}</p>
              </CardContent>
            </Card>
            <Card className="h-full">
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Ingresos</p>
                <p className="text-xl font-bold mt-1 text-success" data-testid="caja-ingresos">{formatDOP(resumen.ingresos)}</p>
              </CardContent>
            </Card>
            <Card className="h-full">
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Egresos</p>
                <p className="text-xl font-bold mt-1 text-destructive" data-testid="caja-egresos">{formatDOP(resumen.egresos)}</p>
              </CardContent>
            </Card>
            <Card className="h-full">
              <CardContent className="pt-5">
                <p className="text-xs text-muted-foreground">Saldo esperado</p>
                <p className="text-xl font-bold mt-1" data-testid="caja-saldo-esperado">{formatDOP(resumen.saldo_actual)}</p>
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
                <Button type="submit" variant="secondary" disabled={saving} className="w-full" data-testid="caja-cerrar-submit">Cerrar turno</Button>
              </form>
            </CardContent>
          </Card>
        </>
      )}

      <div className="flex flex-wrap gap-3">
        <Select
          value={state.filters.tipo || ""}
          onChange={(e) => setFilters({ tipo: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todos los tipos</option>
          <option value="INGRESO">Ingreso</option>
          <option value="EGRESO">Egreso</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={movimientosLoading}
        error={movimientosError}
        isEmpty={movimientos.length === 0}
        emptyIcon={<Wallet className="h-6 w-6" />}
        emptyMessage="Sin movimientos de caja todavía."
        pagination={
          <Pagination
            page={state.page}
            totalPages={totalPages}
            total={total}
            pageSize={state.pageSize}
            onPageChange={setPage}
            onPageSizeChange={setPageSize}
          />
        }
      >
        <Table>
          <TableHeader>
            <TableRow>
              <SortableTableHead column="created_at" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Fecha</SortableTableHead>
              <TableHead>Concepto</TableHead>
              <TableHead>Tipo</TableHead>
              <SortableTableHead column="monto" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Monto</SortableTableHead>
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
      </ScrollableTableCard>
    </div>
  );
}

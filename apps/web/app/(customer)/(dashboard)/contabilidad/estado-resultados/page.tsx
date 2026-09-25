"use client";

import { Fragment, useCallback, useEffect, useState } from "react";
import { TrendingUp } from "lucide-react";
import {
  Input,
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
  ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface LineaEstado {
  codigo: string;
  nombre: string;
  monto: string;
}

interface EstadoResultados {
  desde: string;
  hasta: string;
  ingresos: string;
  costo_ventas: string;
  utilidad_bruta: string;
  gastos: string;
  resultado: string;
  sin_clasificar: string;
  detalle_ingresos: LineaEstado[];
  detalle_costo_ventas: LineaEstado[];
  detalle_gastos: LineaEstado[];
  detalle_sin_clasificar: LineaEstado[];
}

function hoyISO() {
  return new Date().toISOString().slice(0, 10);
}

function primerDiaDelMes() {
  return hoyISO().slice(0, 8) + "01";
}

export default function EstadoResultadosPage() {
  const [desde, setDesde] = useState(primerDiaDelMes);
  const [hasta, setHasta] = useState(hoyISO);
  const [estado, setEstado] = useState<EstadoResultados | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const data = await apiFetch<EstadoResultados>(`/api/contabilidad/estado-resultados?desde=${desde}&hasta=${hasta}`);
      setEstado(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [desde, hasta]);

  useEffect(() => {
    load();
  }, [load]);

  const bloques: { titulo: string; lineas: LineaEstado[] }[] = estado
    ? [
        { titulo: "Ingresos", lineas: estado.detalle_ingresos },
        { titulo: "Costo de ventas", lineas: estado.detalle_costo_ventas },
        { titulo: "Gastos operativos", lineas: estado.detalle_gastos },
        { titulo: "Cuentas sin clasificar", lineas: estado.detalle_sin_clasificar },
      ].filter((b) => b.lineas.length > 0)
    : [];

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Estado de resultados</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Ingresos, costo de ventas y gastos del período, tomados del libro mayor. El período no está cerrado: el resultado es provisional.
        </p>
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <div className="space-y-1.5">
          <label htmlFor="er-desde" className="text-xs text-muted-foreground">Desde</label>
          <Input id="er-desde" type="date" value={desde} max={hasta} onChange={(e) => setDesde(e.target.value)} className="max-w-[170px]" />
        </div>
        <div className="space-y-1.5">
          <label htmlFor="er-hasta" className="text-xs text-muted-foreground">Hasta</label>
          <Input id="er-hasta" type="date" value={hasta} min={desde} onChange={(e) => setHasta(e.target.value)} className="max-w-[170px]" />
        </div>
      </div>

      {estado && !loading && (
        <div className="grid grid-cols-2 sm:grid-cols-4 gap-3" data-testid="estado-resultados-resumen">
          <Resumen label="Ingresos" monto={estado.ingresos} />
          <Resumen label="Costo de ventas" monto={estado.costo_ventas} />
          <Resumen label="Utilidad bruta" monto={estado.utilidad_bruta} />
          <Resumen label="Gastos operativos" monto={estado.gastos} />
        </div>
      )}

      <ScrollableTableCard
        loading={loading}
        error={error || null}
        isEmpty={bloques.length === 0}
        emptyIcon={<TrendingUp className="h-6 w-6" />}
        emptyMessage="Sin ingresos ni gastos registrados en este período."
        pagination={null}
      >
        <Table data-testid="estado-resultados-table">
          <TableHeader>
            <TableRow>
              <TableHead>Cuenta</TableHead>
              <TableHead className="text-right">Monto</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {bloques.map((bloque) => (
              <Fragment key={bloque.titulo}>
                <TableRow className="bg-muted/30">
                  <TableCell colSpan={2} className="font-semibold text-xs uppercase tracking-wide">{bloque.titulo}</TableCell>
                </TableRow>
                {bloque.lineas.map((l) => (
                  <TableRow key={`${bloque.titulo}-${l.codigo}`} data-testid="estado-resultados-row" data-codigo={l.codigo}>
                    <TableCell className="pl-6">
                      <span className="font-mono text-xs text-muted-foreground mr-2">{l.codigo}</span>
                      {l.nombre}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">{formatDOP(l.monto)}</TableCell>
                  </TableRow>
                ))}
              </Fragment>
            ))}
            {estado && (
              <TableRow className="border-t-2 border-foreground/20">
                <TableCell className="font-bold">Resultado del período</TableCell>
                <TableCell
                  className={`text-right tabular-nums font-bold ${Number(estado.resultado) < 0 ? "text-destructive" : "text-success"}`}
                  data-testid="estado-resultados-resultado"
                >
                  {formatDOP(estado.resultado)}
                </TableCell>
              </TableRow>
            )}
          </TableBody>
        </Table>
      </ScrollableTableCard>

      {estado && Number(estado.sin_clasificar) !== 0 && (
        <p className="text-xs text-destructive">
          Hay {formatDOP(estado.sin_clasificar)} en cuentas que no existen en el plan de cuentas. Revísalas: no entran en ningún subtotal.
        </p>
      )}
    </div>
  );
}

function Resumen({ label, monto }: { label: string; monto: string }) {
  return (
    <div className="rounded-md border border-border p-3">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p className="text-lg font-semibold tabular-nums mt-0.5">{formatDOP(monto)}</p>
    </div>
  );
}

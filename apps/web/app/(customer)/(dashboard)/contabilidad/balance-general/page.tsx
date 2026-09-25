"use client";

import { Fragment, useCallback, useEffect, useState } from "react";
import { Scale } from "lucide-react";
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

interface BalanceGeneral {
  al: string;
  activo: string;
  pasivo: string;
  patrimonio: string;
  resultado_acumulado: string;
  sin_clasificar: string;
  total_pasivo_y_patrimonio: string;
  cuadra: boolean;
  detalle_activo: LineaEstado[];
  detalle_pasivo: LineaEstado[];
  detalle_patrimonio: LineaEstado[];
  detalle_sin_clasificar: LineaEstado[];
}

function hoyISO() {
  return new Date().toISOString().slice(0, 10);
}

export default function BalanceGeneralPage() {
  const [al, setAl] = useState(hoyISO);
  const [balance, setBalance] = useState<BalanceGeneral | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    try {
      const data = await apiFetch<BalanceGeneral>(`/api/contabilidad/balance-general?al=${al}`);
      setBalance(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [al]);

  useEffect(() => {
    load();
  }, [load]);

  const bloques: { titulo: string; lineas: LineaEstado[]; total: string }[] = balance
    ? [
        { titulo: "Activo", lineas: balance.detalle_activo, total: balance.activo },
        { titulo: "Pasivo", lineas: balance.detalle_pasivo, total: balance.pasivo },
        {
          titulo: "Patrimonio",
          // El resultado del ejercicio todavía no está cerrado contra
          // patrimonio (no hay asiento de cierre), así que se muestra como
          // una línea calculada aparte de las cuentas 3xxx.
          lineas: [
            ...balance.detalle_patrimonio,
            { codigo: "—", nombre: "Resultado del ejercicio (sin cerrar)", monto: balance.resultado_acumulado },
          ],
          total: String(Number(balance.patrimonio) + Number(balance.resultado_acumulado)),
        },
        { titulo: "Cuentas sin clasificar", lineas: balance.detalle_sin_clasificar, total: balance.sin_clasificar },
      ].filter((b) => b.lineas.length > 0)
    : [];

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Balance general</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Activo, pasivo y patrimonio a una fecha, tomados del libro mayor. Sin cierre de ejercicio, el resultado aparece como una línea propia del
          patrimonio.
        </p>
      </div>

      <div className="flex flex-wrap items-end gap-3">
        <div className="space-y-1.5">
          <label htmlFor="bg-al" className="text-xs text-muted-foreground">Al</label>
          <Input id="bg-al" type="date" value={al} onChange={(e) => setAl(e.target.value)} className="max-w-[170px]" />
        </div>
      </div>

      {balance && !loading && (
        <div
          className={`rounded-md border p-3 text-sm ${balance.cuadra ? "border-success/20 bg-success/10 text-success" : "border-destructive/20 bg-destructive/10 text-destructive"}`}
          data-testid="balance-general-cuadre"
        >
          {balance.cuadra
            ? `Cuadra: Activo ${formatDOP(balance.activo)} = Pasivo + Patrimonio ${formatDOP(balance.total_pasivo_y_patrimonio)}`
            : `No cuadra: Activo ${formatDOP(balance.activo)} vs Pasivo + Patrimonio ${formatDOP(balance.total_pasivo_y_patrimonio)}. Revisa las cuentas sin clasificar.`}
        </div>
      )}

      <ScrollableTableCard
        loading={loading}
        error={error || null}
        isEmpty={bloques.length === 0}
        emptyIcon={<Scale className="h-6 w-6" />}
        emptyMessage="Sin movimientos contables hasta esta fecha."
        pagination={null}
      >
        <Table data-testid="balance-general-table">
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
                  <TableCell className="font-semibold text-xs uppercase tracking-wide">{bloque.titulo}</TableCell>
                  <TableCell className="text-right tabular-nums font-semibold text-xs">{formatDOP(bloque.total)}</TableCell>
                </TableRow>
                {bloque.lineas.map((l) => (
                  <TableRow key={`${bloque.titulo}-${l.codigo}`} data-testid="balance-general-row" data-codigo={l.codigo}>
                    <TableCell className="pl-6">
                      <span className="font-mono text-xs text-muted-foreground mr-2">{l.codigo}</span>
                      {l.nombre}
                    </TableCell>
                    <TableCell className="text-right tabular-nums">{formatDOP(l.monto)}</TableCell>
                  </TableRow>
                ))}
              </Fragment>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

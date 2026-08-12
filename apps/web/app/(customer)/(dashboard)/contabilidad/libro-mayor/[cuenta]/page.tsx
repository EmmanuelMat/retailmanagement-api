"use client";

import { Suspense } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import { ArrowLeft, BookOpen } from "lucide-react";
import {
  Table, TableBody, TableCell, TableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP, Input,
} from "@repo/ui";
import { useServerTable } from "@/lib/use-server-table";

interface MovimientoCuenta {
  fecha: string;
  descripcion: string;
  debe: string;
  haber: string;
  saldo: string;
}

interface MovimientoFilters {
  fechaDesde?: string;
  fechaHasta?: string;
}

export default function LibroMayorDetallePage() {
  return (
    <Suspense fallback={null}>
      <LibroMayorDetalleContent />
    </Suspense>
  );
}

function LibroMayorDetalleContent() {
  const params = useParams<{ cuenta: string }>();
  const cuenta = decodeURIComponent(params.cuenta);

  const {
    items: movimientos,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    setFilters,
  } = useServerTable<MovimientoCuenta, MovimientoFilters>({
    path: `/api/contabilidad/libro-mayor/${encodeURIComponent(cuenta)}`,
    initialPageSize: 50,
  });

  const saldoFinal = movimientos.length > 0 ? movimientos[movimientos.length - 1]!.saldo : "0";

  return (
    <div className="space-y-4">
      <div>
        <Link href="/contabilidad/libro-mayor" className="text-sm text-muted-foreground hover:text-foreground inline-flex items-center gap-1 mb-2">
          <ArrowLeft className="h-3.5 w-3.5" /> Libro mayor
        </Link>
        <h1 className="text-2xl font-bold font-serif tracking-tight">{cuenta}</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Cada movimiento que afecta esta cuenta, en orden cronológico, con saldo acumulado. Saldo actual: <span className="font-semibold text-foreground">{formatDOP(saldoFinal)}</span>
        </p>
      </div>

      <div className="flex flex-wrap gap-3">
        <Input type="date" value={state.filters.fechaDesde || ""} onChange={(e) => setFilters({ fechaDesde: e.target.value || undefined })} className="max-w-[160px]" />
        <Input type="date" value={state.filters.fechaHasta || ""} onChange={(e) => setFilters({ fechaHasta: e.target.value || undefined })} className="max-w-[160px]" />
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={movimientos.length === 0}
        emptyIcon={<BookOpen className="h-6 w-6" />}
        emptyMessage="Sin movimientos en esta cuenta todavía."
        maxHeight="calc(100vh - 420px)"
        pagination={
          <Pagination page={state.page} totalPages={totalPages} total={total} pageSize={state.pageSize} onPageChange={setPage} onPageSizeChange={setPageSize} />
        }
      >
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Fecha</TableHead>
              <TableHead>Descripción</TableHead>
              <TableHead className="text-right">Debe</TableHead>
              <TableHead className="text-right">Haber</TableHead>
              <TableHead className="text-right">Saldo</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {movimientos.map((m, i) => (
              <TableRow key={i}>
                <TableCell className="text-xs text-muted-foreground">{m.fecha}</TableCell>
                <TableCell className="text-muted-foreground">{m.descripcion}</TableCell>
                <TableCell className="text-right tabular-nums">{Number(m.debe) > 0 ? formatDOP(m.debe) : "—"}</TableCell>
                <TableCell className="text-right tabular-nums">{Number(m.haber) > 0 ? formatDOP(m.haber) : "—"}</TableCell>
                <TableCell className="text-right tabular-nums font-semibold">{formatDOP(m.saldo)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

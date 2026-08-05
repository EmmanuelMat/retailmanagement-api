"use client";

import { Suspense } from "react";
import { History } from "lucide-react";
import {
  Badge, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { useServerTable } from "@/lib/use-server-table";

interface CajaSesion {
  id: string;
  monto_inicial: string;
  monto_final: string | null;
  monto_esperado: string | null;
  diferencia: string | null;
  estado: string;
  abierta_at: string;
  cerrada_at: string | null;
}

interface SesionesFilters {
  estado?: string;
}

export default function HistorialCajaPage() {
  return (
    <Suspense fallback={null}>
      <HistorialCajaPageContent />
    </Suspense>
  );
}

function HistorialCajaPageContent() {
  const {
    items: sesiones,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<CajaSesion, SesionesFilters>({
    path: "/api/caja/sesiones",
    initialPageSize: 20,
    initialSortBy: "abierta_at",
    initialSortDir: "desc",
  });

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Historial de caja</h1>
        <p className="text-sm text-muted-foreground mt-1">Arqueos de cada turno abierto y cerrado.</p>
      </div>

      <div className="flex flex-wrap gap-3">
        <Select
          value={state.filters.estado || ""}
          onChange={(e) => setFilters({ estado: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todos los estados</option>
          <option value="ABIERTA">Abierta</option>
          <option value="CERRADA">Cerrada</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={sesiones.length === 0}
        emptyIcon={<History className="h-6 w-6" />}
        emptyMessage="Sin sesiones de caja todavía."
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
              <SortableTableHead column="abierta_at" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Apertura</SortableTableHead>
              <TableHead>Cierre</TableHead>
              <TableHead className="text-right">Inicial</TableHead>
              <TableHead className="text-right">Contado</TableHead>
              <SortableTableHead column="diferencia" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Diferencia</SortableTableHead>
              <TableHead>Estado</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {sesiones.map((s) => (
              <TableRow key={s.id}>
                <TableCell className="text-xs text-muted-foreground">{new Date(s.abierta_at).toLocaleString("es-DO")}</TableCell>
                <TableCell className="text-xs text-muted-foreground">{s.cerrada_at ? new Date(s.cerrada_at).toLocaleString("es-DO") : "—"}</TableCell>
                <TableCell className="text-right tabular-nums">{formatDOP(s.monto_inicial)}</TableCell>
                <TableCell className="text-right tabular-nums">{s.monto_final ? formatDOP(s.monto_final) : "—"}</TableCell>
                <TableCell className={`text-right tabular-nums ${s.diferencia && Number(s.diferencia) !== 0 ? "text-destructive" : ""}`}>
                  {s.diferencia ? formatDOP(s.diferencia) : "—"}
                </TableCell>
                <TableCell><Badge variant={s.estado === "ABIERTA" ? "default" : "secondary"}>{s.estado}</Badge></TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

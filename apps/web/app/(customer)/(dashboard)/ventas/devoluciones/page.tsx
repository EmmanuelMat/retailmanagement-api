"use client";

import { Suspense } from "react";
import Link from "next/link";
import { Undo2 } from "lucide-react";
import {
  Badge,
  Input,
  Table,
  TableBody,
  TableCell,
  TableHead,
  SortableTableHead,
  TableHeader,
  TableRow,
  Pagination,
  ScrollableTableCard,
  formatDOP,
} from "@repo/ui";
import { useServerTable } from "@/lib/use-server-table";

interface NotaCredito {
  id: string;
  venta_id: string;
  venta_e_ncf: string | null;
  cliente_nombre: string | null;
  motivo: string;
  total: string;
  es_parcial: boolean;
  e_ncf: string | null;
  estado_dgii: string | null;
  created_at: string;
}

interface DevolucionesFilters {
  fechaDesde?: string;
  fechaHasta?: string;
}

const DGII_VARIANT: Record<string, "default" | "success" | "warning" | "destructive"> = {
  ACEPTADO: "success",
  FIRMADO_NO_ENVIADO: "warning",
  CONTINGENCIA_PENDIENTE: "warning",
  RECHAZADO: "destructive",
};

export default function DevolucionesPage() {
  return (
    <Suspense fallback={null}>
      <DevolucionesPageContent />
    </Suspense>
  );
}

function DevolucionesPageContent() {
  const { items: notas, total, totalPages, loading, error, state, setPage, setPageSize, toggleSort, setFilters } =
    useServerTable<NotaCredito, DevolucionesFilters>({
      path: "/api/notas-credito",
      initialPageSize: 20,
      initialSortBy: "created_at",
      initialSortDir: "desc",
    });

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Devoluciones</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Notas de crédito (e-CF tipo 34) emitidas sobre una venta. Para registrar una nueva, entra a la venta y usa
          &ldquo;Devolver productos&rdquo;.
        </p>
      </div>

      <div className="flex flex-wrap gap-3">
        <Input
          type="date"
          aria-label="Desde"
          value={state.filters.fechaDesde || ""}
          onChange={(e) => setFilters({ fechaDesde: e.target.value || undefined })}
          className="max-w-[160px]"
        />
        <Input
          type="date"
          aria-label="Hasta"
          value={state.filters.fechaHasta || ""}
          onChange={(e) => setFilters({ fechaHasta: e.target.value || undefined })}
          className="max-w-[160px]"
        />
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={notas.length === 0}
        emptyIcon={<Undo2 className="h-6 w-6" />}
        emptyMessage="Todavía no hay devoluciones. Se registran desde el detalle de una venta."
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
              <SortableTableHead column="created_at" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>
                Fecha
              </SortableTableHead>
              <TableHead>Cliente</TableHead>
              <TableHead>Venta (e-NCF)</TableHead>
              <TableHead>e-NCF de la nota</TableHead>
              <TableHead>Tipo</TableHead>
              <TableHead>Motivo</TableHead>
              <SortableTableHead column="total" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">
                Total
              </SortableTableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {notas.map((n) => (
              <TableRow key={n.id} data-testid="devolucion-row">
                <TableCell className="text-xs text-muted-foreground">
                  <Link href={`/ventas/${n.venta_id}` as any} className="hover:text-primary">
                    {new Date(n.created_at).toLocaleString("es-DO")}
                  </Link>
                </TableCell>
                <TableCell>{n.cliente_nombre || "Consumidor final"}</TableCell>
                <TableCell className="font-mono text-xs">{n.venta_e_ncf || "—"}</TableCell>
                <TableCell className="font-mono text-xs">
                  {n.e_ncf ? (
                    <span className="flex items-center gap-2">
                      {n.e_ncf}
                      {n.estado_dgii && (
                        <Badge variant={DGII_VARIANT[n.estado_dgii] || "default"}>
                          {n.estado_dgii === "CONTINGENCIA_PENDIENTE" ? "pendiente de envío" : n.estado_dgii}
                        </Badge>
                      )}
                    </span>
                  ) : (
                    <Badge variant="secondary">Sin e-CF</Badge>
                  )}
                </TableCell>
                <TableCell>
                  <Badge variant={n.es_parcial ? "warning" : "secondary"}>{n.es_parcial ? "Parcial" : "Total"}</Badge>
                </TableCell>
                <TableCell className="text-muted-foreground max-w-[18rem] truncate" title={n.motivo}>
                  {n.motivo}
                </TableCell>
                <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(n.total)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

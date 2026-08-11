"use client";

import { Suspense, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Receipt } from "lucide-react";
import {
  Badge,
  Input,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  SortableTableHead,
  TableHeader,
  TableRow,
  Pagination,
  ScrollableTableCard,
  QuerySearchInput,
  dateRangeFilter,
  lookupFilter,
  formatDOP,
  type QueryPrefixDef,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";

interface Venta {
  id: string;
  cliente_nombre: string | null;
  total: string;
  metodo_pago: string;
  estado: string;
  e_ncf: string | null;
  estado_dgii: string | null;
  created_at: string;
}

interface Cliente {
  id: string;
  nombre: string;
}

interface VentasFilters {
  clienteId?: string;
  fechaDesde?: string;
  fechaHasta?: string;
  search?: string;
}

const DGII_VARIANT: Record<string, "default" | "success" | "warning" | "destructive"> = {
  Aceptado: "success",
  ACEPTADO: "success",
  FIRMADO_NO_ENVIADO: "warning",
  Rechazado: "destructive",
  RECHAZADO: "destructive",
};

export default function VentasPage() {
  return (
    <Suspense fallback={null}>
      <VentasPageContent />
    </Suspense>
  );
}

function VentasPageContent() {
  const [clientes, setClientes] = useState<Cliente[]>([]);

  useEffect(() => {
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
  }, []);

  const {
    items: ventas,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<Venta, VentasFilters>({
    path: "/api/ventas",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [rawQuery, setRawQuery] = useState(state.filters.search || "");

  // "id:" pasa directo al filtro `search` existente (que ya hace LIKE prefijo
  // sobre v.id::text en el servidor); "client:" resuelve contra la lista de
  // clientes ya cargada; "date:" acepta un día o un rango "desde..hasta".
  const queryPrefixes: QueryPrefixDef[] = useMemo(
    () => [
      { prefix: "id", label: "id de venta (prefijo)", apply: (value) => (value.trim() ? { search: value.trim() } : null) },
      { prefix: "client", label: "nombre de cliente", apply: lookupFilter(clientes, (c) => c.nombre, (c) => c.id, "clienteId") },
      { prefix: "date", label: "YYYY-MM-DD o YYYY-MM-DD..YYYY-MM-DD", apply: dateRangeFilter("fechaDesde", "fechaHasta") },
    ],
    [clientes]
  );

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Ventas</h1>
        <p className="text-sm text-muted-foreground mt-1">Historial de ventas, e-NCF y estado DGII.</p>
      </div>

      <div className="flex flex-wrap gap-3">
        <QuerySearchInput
          value={rawQuery}
          onChange={setRawQuery}
          onParsed={({ text, filters }) => setFilters({ search: text || undefined, ...filters } as Partial<VentasFilters>)}
          prefixes={queryPrefixes}
          placeholder="ID de venta, o client:, date:..."
          className="flex-1 max-w-sm"
        />
        <Select
          value={state.filters.clienteId || ""}
          onChange={(e) => setFilters({ clienteId: e.target.value || undefined })}
          className="max-w-[220px]"
        >
          <option value="">Todos los clientes</option>
          {clientes.map((c) => (
            <option key={c.id} value={c.id}>{c.nombre}</option>
          ))}
        </Select>
        <Input
          type="date"
          value={state.filters.fechaDesde || ""}
          onChange={(e) => setFilters({ fechaDesde: e.target.value || undefined })}
          className="max-w-[160px]"
        />
        <Input
          type="date"
          value={state.filters.fechaHasta || ""}
          onChange={(e) => setFilters({ fechaHasta: e.target.value || undefined })}
          className="max-w-[160px]"
        />
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={ventas.length === 0}
        emptyIcon={<Receipt className="h-6 w-6" />}
        emptyMessage="Aún no hay ventas. Regístralas desde el Punto de Venta."
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
              <TableHead>Cliente</TableHead>
              <TableHead>e-NCF</TableHead>
              <SortableTableHead column="estado" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Estado DGII</SortableTableHead>
              <TableHead>Pago</TableHead>
              <SortableTableHead column="total" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Total</SortableTableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {ventas.map((v) => (
              <TableRow key={v.id} className="cursor-pointer">
                <TableCell className="text-xs text-muted-foreground">
                  <Link href={`/ventas/${v.id}` as any} className="hover:text-primary">{new Date(v.created_at).toLocaleString("es-DO")}</Link>
                </TableCell>
                <TableCell>{v.cliente_nombre || "Consumidor final"}</TableCell>
                <TableCell className="font-mono text-xs">{v.e_ncf || "—"}</TableCell>
                <TableCell>{v.estado_dgii ? <Badge variant={DGII_VARIANT[v.estado_dgii] || "default"}>{v.estado_dgii}</Badge> : <Badge variant="secondary">Sin emitir</Badge>}</TableCell>
                <TableCell className="text-muted-foreground">{v.metodo_pago}</TableCell>
                <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(v.total)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

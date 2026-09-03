"use client";

import { Suspense } from "react";
import Link from "next/link";
import { ClipboardList, Plus, Printer, Search } from "lucide-react";
import {
  Badge, Button, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";
import { ESTADO_VARIANT } from "./estado-variant";

interface OrdenCompra {
  id: string;
  proveedor_nombre: string | null;
  estado: string;
  total: string;
  fecha_esperada: string | null;
  created_at: string;
}

interface OrdenesFilters {
  estado?: string;
  search?: string;
}

const ESTADO_LABEL: Record<string, string> = {
  BORRADOR: "Borrador",
  ENVIADA: "Enviada",
  RECIBIDA_PARCIAL: "Recibida parcial",
  RECIBIDA: "Recibida",
  CANCELADA: "Cancelada",
};

export default function OrdenesCompraPage() {
  return (
    <Suspense fallback={null}>
      <OrdenesCompraPageContent />
    </Suspense>
  );
}

function OrdenesCompraPageContent() {
  const {
    items: ordenes,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<OrdenCompra, OrdenesFilters>({
    path: "/api/ordenes-compra",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Órdenes de Compra</h1>
          <p className="text-sm text-muted-foreground mt-1">Intención de compra a un proveedor — recibirla crea la compra real y mueve inventario.</p>
        </div>
        <Link href={"/ordenes-compra/nueva" as any}>
          <Button><Plus className="h-4 w-4" />Nueva orden</Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar..." className="pl-9" />
        </div>
        <Select
          value={state.filters.estado || ""}
          onChange={(e) => setFilters({ estado: e.target.value || undefined })}
          className="max-w-[200px]"
        >
          <option value="">Todos los estados</option>
          {Object.entries(ESTADO_LABEL).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={ordenes.length === 0}
        emptyIcon={<ClipboardList className="h-6 w-6" />}
        emptyMessage="Aún no hay órdenes de compra."
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
              <TableHead>Proveedor</TableHead>
              <SortableTableHead column="estado" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Estado</SortableTableHead>
              <SortableTableHead column="fecha_esperada" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Esperada</SortableTableHead>
              <SortableTableHead column="total" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Total</SortableTableHead>
              <TableHead className="w-16 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {ordenes.map((o) => (
              <TableRow key={o.id} className="cursor-pointer">
                <TableCell className="text-xs text-muted-foreground">
                  <Link href={`/ordenes-compra/${o.id}` as any} className="hover:text-primary">{new Date(o.created_at).toLocaleString("es-DO")}</Link>
                </TableCell>
                <TableCell>{o.proveedor_nombre || "—"}</TableCell>
                <TableCell><Badge variant={ESTADO_VARIANT[o.estado] || "default"}>{ESTADO_LABEL[o.estado] || o.estado}</Badge></TableCell>
                <TableCell className="text-muted-foreground">{o.fecha_esperada ? new Date(o.fecha_esperada).toLocaleDateString("es-DO") : "—"}</TableCell>
                <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(o.total)}</TableCell>
                <TableCell className="text-right">
                  <Link href={`/imprimir/orden-compra/${o.id}` as any} target="_blank">
                    <Button size="icon" variant="ghost" title="Imprimir"><Printer className="h-4 w-4" /></Button>
                  </Link>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

"use client";

import { Suspense } from "react";
import Link from "next/link";
import { PackageCheck, Plus, Printer, Search } from "lucide-react";
import {
  Button, Input,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard,
} from "@repo/ui";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Conduce {
  id: string;
  // Ausente en un conduce standalone (sin Venta previa) - ver conduce_service.rs.
  venta_id: string | null;
  cliente_nombre: string | null;
  direccion_entrega: string | null;
  created_at: string;
}

interface ConducesFilters {
  search?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

export default function ConducesPage() {
  return (
    <Suspense fallback={null}>
      <ConducesPageContent />
    </Suspense>
  );
}

function ConducesPageContent() {
  const {
    items: conduces,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<Conduce, ConducesFilters>({
    path: "/api/conduces",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Conduces</h1>
          <p className="text-sm text-muted-foreground mt-1">Historial de entregas parciales.</p>
        </div>
        <Link href={"/conduces/nuevo" as any}>
          <Button size="sm"><Plus className="h-4 w-4" />Nuevo conduce</Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por cliente o dirección..." className="pl-9" />
        </div>
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
        isEmpty={conduces.length === 0}
        emptyIcon={<PackageCheck className="h-6 w-6" />}
        emptyMessage="Aún no hay entregas registradas."
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
              <TableHead>Dirección</TableHead>
              <TableHead>Venta</TableHead>
              <TableHead className="w-16 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {conduces.map((c) => (
              <TableRow key={c.id} className="cursor-pointer">
                <TableCell className="text-xs text-muted-foreground">
                  <Link href={`/conduces/${c.id}` as any} className="hover:text-primary">{new Date(c.created_at).toLocaleString("es-DO")}</Link>
                </TableCell>
                <TableCell>{c.cliente_nombre || "Consumidor final"}</TableCell>
                <TableCell className="text-muted-foreground">{c.direccion_entrega || "—"}</TableCell>
                <TableCell>
                  {c.venta_id ? (
                    <Link href={`/ventas/${c.venta_id}` as any} className="text-primary hover:underline text-xs font-mono">
                      Ver venta
                    </Link>
                  ) : (
                    <span className="text-xs text-muted-foreground">—</span>
                  )}
                </TableCell>
                <TableCell className="text-right">
                  <Link href={`/imprimir/conduce/${c.id}` as any} target="_blank">
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

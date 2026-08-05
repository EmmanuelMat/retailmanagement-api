"use client";

import { Suspense } from "react";
import { BookOpen, Search } from "lucide-react";
import {
  Input,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface CuentaResumen {
  cuenta: string;
  debe: string;
  haber: string;
  saldo: string;
}

interface LibroMayorFilters {
  search?: string;
}

export default function LibroMayorPage() {
  return (
    <Suspense fallback={null}>
      <LibroMayorPageContent />
    </Suspense>
  );
}

function LibroMayorPageContent() {
  const {
    items: cuentas,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<CuentaResumen, LibroMayorFilters>({
    path: "/api/contabilidad/libro-mayor",
    initialPageSize: 20,
    initialSortBy: "cuenta",
    initialSortDir: "asc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Libro mayor</h1>
        <p className="text-sm text-muted-foreground mt-1">Saldo acumulado por cuenta contable.</p>
      </div>

      <div className="relative max-w-sm">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por cuenta..." className="pl-9" />
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={cuentas.length === 0}
        emptyIcon={<BookOpen className="h-6 w-6" />}
        emptyMessage="Sin movimientos contables todavía."
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
              <SortableTableHead column="cuenta" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Cuenta</SortableTableHead>
              <SortableTableHead column="debe" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Debe</SortableTableHead>
              <SortableTableHead column="haber" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Haber</SortableTableHead>
              <TableHead className="text-right">Saldo</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {cuentas.map((c) => (
              <TableRow key={c.cuenta}>
                <TableCell className="font-medium">{c.cuenta}</TableCell>
                <TableCell className="text-right tabular-nums text-muted-foreground">{formatDOP(c.debe)}</TableCell>
                <TableCell className="text-right tabular-nums text-muted-foreground">{formatDOP(c.haber)}</TableCell>
                <TableCell className="text-right tabular-nums font-semibold">{formatDOP(c.saldo)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

"use client";

import { Suspense } from "react";
import Link from "next/link";
import { Plus, Pencil, Trash2, Users2, Search } from "lucide-react";
import {
  Button, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Empleado {
  id: string;
  nombre: string;
  puesto: string | null;
  salario_mensual: string;
  disponible_adelanto: string;
}

interface EmpleadosFilters {
  search?: string;
  activo?: string;
}

export default function EmpleadosPage() {
  return (
    <Suspense fallback={null}>
      <EmpleadosPageContent />
    </Suspense>
  );
}

function EmpleadosPageContent() {
  const {
    items: empleados,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
    refresh,
  } = useServerTable<Empleado, EmpleadosFilters>({
    path: "/api/empleados",
    initialPageSize: 20,
    initialSortBy: "nombre",
    initialSortDir: "asc",
    initialFilters: { activo: "true" },
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function handleDelete(id: string) {
    if (!confirm("¿Desactivar este empleado?")) return;
    try {
      await apiFetch(`/api/empleados/${id}`, { method: "DELETE" });
      refresh();
    } catch (e: any) {
      alert(e.message);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Empleados</h1>
          <p className="text-sm text-muted-foreground mt-1">Equipo y disponible para adelanto (50% del sueldo mensual).</p>
        </div>
        <Link href="/nomina/empleados/nuevo">
          <Button><Plus className="h-4 w-4" />Nuevo empleado</Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por nombre o cédula..." className="pl-9" />
        </div>
        <Select
          value={state.filters.activo ?? ""}
          onChange={(e) => setFilters({ activo: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="true">Activos</option>
          <option value="false">Inactivos</option>
          <option value="">Todos</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={empleados.length === 0}
        emptyIcon={<Users2 className="h-6 w-6" />}
        emptyMessage="No hay empleados registrados."
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
              <SortableTableHead column="nombre" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Nombre</SortableTableHead>
              <TableHead>Puesto</TableHead>
              <SortableTableHead column="salario_mensual" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Salario mensual</SortableTableHead>
              <TableHead className="text-right">Disponible adelanto</TableHead>
              <TableHead className="w-24 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {empleados.map((e) => (
              <TableRow key={e.id}>
                <TableCell className="font-medium">{e.nombre}</TableCell>
                <TableCell className="text-muted-foreground">{e.puesto || "—"}</TableCell>
                <TableCell className="text-right tabular-nums">{formatDOP(e.salario_mensual)}</TableCell>
                <TableCell className="text-right tabular-nums text-success">{formatDOP(e.disponible_adelanto)}</TableCell>
                <TableCell className="text-right">
                  <div className="flex justify-end gap-1">
                    <Link href={`/nomina/empleados/${e.id}` as any}>
                      <Button size="icon" variant="ghost"><Pencil className="h-4 w-4" /></Button>
                    </Link>
                    <Button size="icon" variant="ghost" onClick={() => handleDelete(e.id)}><Trash2 className="h-4 w-4" /></Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

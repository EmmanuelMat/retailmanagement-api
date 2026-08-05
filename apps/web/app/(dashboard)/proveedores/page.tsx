"use client";

import { Suspense } from "react";
import Link from "next/link";
import { Plus, Pencil, Trash2, Truck, Search } from "lucide-react";
import {
  Button, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Proveedor {
  id: string;
  nombre: string;
  rnc: string | null;
  contacto: string | null;
  telefono: string | null;
}

interface ProveedoresFilters {
  search?: string;
  activo?: string;
}

export default function ProveedoresPage() {
  return (
    <Suspense fallback={null}>
      <ProveedoresPageContent />
    </Suspense>
  );
}

function ProveedoresPageContent() {
  const {
    items: proveedores,
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
  } = useServerTable<Proveedor, ProveedoresFilters>({
    path: "/api/proveedores",
    initialPageSize: 20,
    initialSortBy: "nombre",
    initialSortDir: "asc",
    initialFilters: { activo: "true" },
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function handleDelete(id: string) {
    if (!confirm("¿Desactivar este proveedor?")) return;
    try {
      await apiFetch(`/api/proveedores/${id}`, { method: "DELETE" });
      refresh();
    } catch (e: any) {
      alert(e.message);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Proveedores</h1>
          <p className="text-sm text-muted-foreground mt-1">Para compras y el reporte 606.</p>
        </div>
        <Link href="/proveedores/nuevo">
          <Button><Plus className="h-4 w-4" />Nuevo proveedor</Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por nombre o RNC..." className="pl-9" />
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
        isEmpty={proveedores.length === 0}
        emptyIcon={<Truck className="h-6 w-6" />}
        emptyMessage="No hay proveedores todavía."
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
              <TableHead>RNC</TableHead>
              <TableHead>Contacto</TableHead>
              <TableHead>Teléfono</TableHead>
              <TableHead className="w-24 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {proveedores.map((p) => (
              <TableRow key={p.id}>
                <TableCell className="font-medium">{p.nombre}</TableCell>
                <TableCell className="font-mono text-xs text-muted-foreground">{p.rnc || "—"}</TableCell>
                <TableCell className="text-muted-foreground">{p.contacto || "—"}</TableCell>
                <TableCell className="text-muted-foreground">{p.telefono || "—"}</TableCell>
                <TableCell className="text-right">
                  <div className="flex justify-end gap-1">
                    <Link href={`/proveedores/${p.id}` as any}>
                      <Button size="icon" variant="ghost"><Pencil className="h-4 w-4" /></Button>
                    </Link>
                    <Button size="icon" variant="ghost" onClick={() => handleDelete(p.id)}><Trash2 className="h-4 w-4" /></Button>
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

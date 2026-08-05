"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { Plus, ShoppingBag } from "lucide-react";
import {
  Button,
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
  formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";

interface Compra {
  id: string;
  proveedor_nombre: string | null;
  total: string;
  metodo_pago: string;
  created_at: string;
}

interface Proveedor {
  id: string;
  nombre: string;
}

interface ComprasFilters {
  proveedorId?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

export default function ComprasPage() {
  return (
    <Suspense fallback={null}>
      <ComprasPageContent />
    </Suspense>
  );
}

function ComprasPageContent() {
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);

  useEffect(() => {
    apiFetch<{ items: Proveedor[] }>("/api/proveedores?pageSize=1000&activo=true").then((d) => setProveedores(d.items)).catch(() => {});
  }, []);

  const {
    items: compras,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<Compra, ComprasFilters>({
    path: "/api/compras",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Compras</h1>
          <p className="text-sm text-muted-foreground mt-1">Compras a proveedores — alimentan costo promedio y el reporte 606.</p>
        </div>
        <Link href="/compras/nueva">
          <Button><Plus className="h-4 w-4" />Nueva compra</Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <Select
          value={state.filters.proveedorId || ""}
          onChange={(e) => setFilters({ proveedorId: e.target.value || undefined })}
          className="max-w-[220px]"
        >
          <option value="">Todos los proveedores</option>
          {proveedores.map((p) => (
            <option key={p.id} value={p.id}>{p.nombre}</option>
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
        isEmpty={compras.length === 0}
        emptyIcon={<ShoppingBag className="h-6 w-6" />}
        emptyMessage="No hay compras registradas todavía."
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
              <TableHead>Pago</TableHead>
              <SortableTableHead column="total" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Total</SortableTableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {compras.map((c) => (
              <TableRow key={c.id}>
                <TableCell className="text-xs text-muted-foreground">{new Date(c.created_at).toLocaleString("es-DO")}</TableCell>
                <TableCell className="font-medium">{c.proveedor_nombre || "—"}</TableCell>
                <TableCell className="text-muted-foreground">{c.metodo_pago}</TableCell>
                <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(c.total)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

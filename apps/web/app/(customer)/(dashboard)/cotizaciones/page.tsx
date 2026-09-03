"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { FileText, Plus, Printer, Search } from "lucide-react";
import {
  Badge, Button, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Cotizacion {
  id: string;
  cliente_nombre: string | null;
  total: string;
  estado: string;
  fecha_vencimiento: string | null;
  created_at: string;
}

interface Cliente {
  id: string;
  nombre: string;
}

interface CotizacionesFilters {
  estado?: string;
  clienteId?: string;
  search?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

const ESTADO_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  PENDIENTE: "warning",
  ACEPTADA: "default",
  CONVERTIDA: "success",
  RECHAZADA: "destructive",
  VENCIDA: "secondary",
};

export default function CotizacionesPage() {
  return (
    <Suspense fallback={null}>
      <CotizacionesPageContent />
    </Suspense>
  );
}

function CotizacionesPageContent() {
  const [clientes, setClientes] = useState<Cliente[]>([]);

  useEffect(() => {
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
  }, []);

  const {
    items: cotizaciones,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<Cotizacion, CotizacionesFilters>({
    path: "/api/cotizaciones",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Cotizaciones</h1>
          <p className="text-sm text-muted-foreground mt-1">Propuestas sin compromiso — conviértelas en venta cuando el cliente acepte.</p>
        </div>
        <Link href={"/cotizaciones/nueva" as any}>
          <Button><Plus className="h-4 w-4" />Nueva cotización</Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por cliente..." className="pl-9" />
        </div>
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
        <Select
          value={state.filters.estado || ""}
          onChange={(e) => setFilters({ estado: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todos los estados</option>
          <option value="PENDIENTE">Pendiente</option>
          <option value="ACEPTADA">Aceptada</option>
          <option value="CONVERTIDA">Convertida</option>
          <option value="RECHAZADA">Rechazada</option>
          <option value="VENCIDA">Vencida</option>
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
        isEmpty={cotizaciones.length === 0}
        emptyIcon={<FileText className="h-6 w-6" />}
        emptyMessage="Aún no hay cotizaciones."
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
              <SortableTableHead column="fecha_vencimiento" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Vence</SortableTableHead>
              <SortableTableHead column="estado" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Estado</SortableTableHead>
              <SortableTableHead column="total" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Total</SortableTableHead>
              <TableHead className="w-16 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {cotizaciones.map((c) => (
              <TableRow key={c.id} className="cursor-pointer">
                <TableCell className="text-xs text-muted-foreground">
                  <Link href={`/cotizaciones/${c.id}` as any} className="hover:text-primary">{new Date(c.created_at).toLocaleString("es-DO")}</Link>
                </TableCell>
                <TableCell>{c.cliente_nombre || "Consumidor final"}</TableCell>
                <TableCell className="text-muted-foreground">{c.fecha_vencimiento ? new Date(c.fecha_vencimiento).toLocaleDateString("es-DO") : "—"}</TableCell>
                <TableCell><Badge variant={ESTADO_VARIANT[c.estado] || "default"}>{c.estado}</Badge></TableCell>
                <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(c.total)}</TableCell>
                <TableCell className="text-right">
                  <Link href={`/imprimir/cotizacion/${c.id}` as any} target="_blank">
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

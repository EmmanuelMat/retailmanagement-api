"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { Wrench, Plus, Search } from "lucide-react";
import {
  Badge, Button, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface OrdenServicio {
  id: string;
  cliente_nombre: string | null;
  estado: string;
  prioridad: string;
  condicion_nombre: string | null;
  fecha_programada: string | null;
  total: string;
  created_at: string;
}

interface Cliente {
  id: string;
  nombre: string;
}

interface OrdenesFilters {
  estado?: string;
  clienteId?: string;
  search?: string;
}

export const ESTADO_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  BORRADOR: "secondary",
  PROGRAMADA: "default",
  EN_PROCESO: "warning",
  PAUSADA: "secondary",
  COMPLETADA: "success",
  CANCELADA: "destructive",
};

const ESTADO_LABEL: Record<string, string> = {
  BORRADOR: "Borrador",
  PROGRAMADA: "Programada",
  EN_PROCESO: "En proceso",
  PAUSADA: "Pausada",
  COMPLETADA: "Completada",
  CANCELADA: "Cancelada",
};

export default function OrdenesServicioPage() {
  return (
    <Suspense fallback={null}>
      <OrdenesServicioPageContent />
    </Suspense>
  );
}

function OrdenesServicioPageContent() {
  const [clientes, setClientes] = useState<Cliente[]>([]);

  useEffect(() => {
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
  }, []);

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
  } = useServerTable<OrdenServicio, OrdenesFilters>({
    path: "/api/ordenes-servicio",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  return (
    <div className="space-y-4">
      <div className="flex items-start justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Órdenes de Servicio</h1>
          <p className="text-sm text-muted-foreground mt-1">Trabajos agendados, en proceso o completados — con técnico, materiales y facturación.</p>
        </div>
        <Link href={"/ordenes-servicio/nueva" as any}>
          <Button><Plus className="h-4 w-4" />Nueva orden</Button>
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
          className="max-w-[180px]"
        >
          <option value="">Todos los estados</option>
          {Object.entries(ESTADO_LABEL).map(([k, v]) => <option key={k} value={k}>{v}</option>)}
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={ordenes.length === 0}
        emptyIcon={<Wrench className="h-6 w-6" />}
        emptyMessage="Aún no hay órdenes de servicio."
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
              <TableHead>Condición</TableHead>
              <SortableTableHead column="prioridad" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Prioridad</SortableTableHead>
              <SortableTableHead column="fecha_programada" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Programada</SortableTableHead>
              <SortableTableHead column="estado" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Estado</SortableTableHead>
              <SortableTableHead column="total" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Total</SortableTableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {ordenes.map((o) => (
              <TableRow key={o.id} className="cursor-pointer">
                <TableCell className="text-xs text-muted-foreground">
                  <Link href={`/ordenes-servicio/${o.id}` as any} className="hover:text-primary">{new Date(o.created_at).toLocaleString("es-DO")}</Link>
                </TableCell>
                <TableCell>{o.cliente_nombre || "Consumidor final"}</TableCell>
                <TableCell className="text-muted-foreground">{o.condicion_nombre || "—"}</TableCell>
                <TableCell className="text-muted-foreground">{o.prioridad}</TableCell>
                <TableCell className="text-muted-foreground">{o.fecha_programada ? new Date(o.fecha_programada).toLocaleDateString("es-DO") : "—"}</TableCell>
                <TableCell><Badge variant={ESTADO_VARIANT[o.estado] || "default"}>{ESTADO_LABEL[o.estado] || o.estado}</Badge></TableCell>
                <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(o.total)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

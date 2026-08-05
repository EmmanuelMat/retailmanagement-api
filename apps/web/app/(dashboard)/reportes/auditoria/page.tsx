"use client";

import { Suspense, useEffect, useState } from "react";
import { History, Search } from "lucide-react";
import {
  Badge, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow, Pagination,
  ScrollableTableCard,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface AuditoriaEntry {
  id: string;
  usuario_id: string | null;
  accion: string;
  entidad: string;
  entidad_id: string | null;
  detalle: Record<string, unknown>;
  created_at: string;
}

interface Usuario {
  id: string;
  nombre: string;
  email: string;
}

interface AuditoriaFilters {
  usuarioId?: string;
  accion?: string;
  entidad?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

const ACCION_VARIANT: Record<string, "default" | "success" | "warning" | "destructive" | "secondary"> = {
  VENTA_CREADA: "success",
  NOTA_CREDITO_EMITIDA: "warning",
  ABONO_REGISTRADO: "success",
  USUARIO_DESACTIVADO: "destructive",
  ADELANTO_RECHAZADO: "destructive",
  LICENCIA_ACTIVADA: "success",
};

const FIVE_MINUTES_MS = 5 * 60 * 1000;

export default function AuditoriaPage() {
  return (
    <Suspense fallback={null}>
      <AuditoriaPageContent />
    </Suspense>
  );
}

function AuditoriaPageContent() {
  const [usuarios, setUsuarios] = useState<Usuario[]>([]);

  useEffect(() => {
    apiFetch<{ items: Usuario[] }>("/api/config/usuarios?pageSize=1000").then((d) => setUsuarios(d.items)).catch(() => {});
  }, []);

  const {
    items: entradas,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<AuditoriaEntry, AuditoriaFilters>({
    path: "/api/auditoria",
    initialPageSize: 10,
    initialSortBy: "created_at",
    initialSortDir: "desc",
    pollIntervalMs: FIVE_MINUTES_MS,
  });

  const [accionInput, setAccionInput] = useSearchFilterSync(state.filters.accion || "", (accion) => setFilters({ accion }));

  const usuarioNombre = (id: string | null) => usuarios.find((u) => u.id === id)?.nombre || "—";

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Auditoría</h1>
        <p className="text-sm text-muted-foreground mt-1">Quién hizo qué — ventas, descuentos, inventario, usuarios, nómina y licencia. Se actualiza automáticamente cada 5 minutos.</p>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={accionInput} onChange={(e) => setAccionInput(e.target.value)} placeholder="Buscar por acción..." className="pl-9" />
        </div>
        <Select
          value={state.filters.usuarioId || ""}
          onChange={(e) => setFilters({ usuarioId: e.target.value || undefined })}
          className="max-w-[200px]"
        >
          <option value="">Todos los usuarios</option>
          {usuarios.map((u) => (
            <option key={u.id} value={u.id}>{u.nombre}</option>
          ))}
        </Select>
        <Input
          value={state.filters.entidad || ""}
          onChange={(e) => setFilters({ entidad: e.target.value || undefined })}
          placeholder="Entidad (ej. venta)"
          className="max-w-[160px]"
        />
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
        isEmpty={entradas.length === 0}
        emptyIcon={<History className="h-6 w-6" />}
        emptyMessage="Todavía no hay actividad registrada."
        pagination={
          <Pagination
            page={state.page}
            totalPages={totalPages}
            total={total}
            pageSize={state.pageSize}
            onPageChange={setPage}
            onPageSizeChange={setPageSize}
            pageSizeOptions={[10, 20, 50, 100]}
          />
        }
      >
        <Table>
          <TableHeader>
            <TableRow>
              <SortableTableHead column="created_at" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Fecha</SortableTableHead>
              <TableHead>Usuario</TableHead>
              <SortableTableHead column="accion" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Acción</SortableTableHead>
              <SortableTableHead column="entidad" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Entidad</SortableTableHead>
              <TableHead>Detalle</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {entradas.map((e) => (
              <TableRow key={e.id}>
                <TableCell className="text-muted-foreground whitespace-nowrap">
                  {new Date(e.created_at).toLocaleString("es-DO", { dateStyle: "short", timeStyle: "short" })}
                </TableCell>
                <TableCell className="text-muted-foreground">{usuarioNombre(e.usuario_id)}</TableCell>
                <TableCell>
                  <Badge variant={ACCION_VARIANT[e.accion] || "default"}>{e.accion.replaceAll("_", " ")}</Badge>
                </TableCell>
                <TableCell className="text-muted-foreground">{e.entidad}</TableCell>
                <TableCell className="font-mono text-xs text-muted-foreground max-w-md truncate">
                  {Object.keys(e.detalle || {}).length > 0 ? JSON.stringify(e.detalle) : "—"}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

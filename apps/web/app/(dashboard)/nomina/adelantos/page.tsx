"use client";

import { Suspense, useEffect, useState } from "react";
import { Plus, Check, X, HandCoins, Search } from "lucide-react";
import {
  Badge, Button, Card, CardHeader, CardTitle, CardContent, Label, Select, Input,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Empleado {
  id: string;
  nombre: string;
  disponible_adelanto: string;
}

interface Adelanto {
  id: string;
  empleado_id: string;
  empleado_nombre: string;
  monto: string;
  motivo: string | null;
  estado: "PENDIENTE" | "APROBADO" | "RECHAZADO" | "DESCONTADO";
  created_at: string;
}

interface AdelantosFilters {
  estado?: string;
  empleadoId?: string;
  search?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

const ESTADO_VARIANT: Record<string, "default" | "secondary" | "success" | "destructive"> = {
  PENDIENTE: "default",
  APROBADO: "success",
  RECHAZADO: "destructive",
  DESCONTADO: "secondary",
};

export default function AdelantosPage() {
  return (
    <Suspense fallback={null}>
      <AdelantosPageContent />
    </Suspense>
  );
}

function AdelantosPageContent() {
  const [empleados, setEmpleados] = useState<Empleado[]>([]);
  const [formError, setFormError] = useState("");
  const [empleadoId, setEmpleadoId] = useState("");
  const [monto, setMonto] = useState("");
  const [motivo, setMotivo] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    apiFetch<{ items: Empleado[] }>("/api/empleados?pageSize=1000&activo=true").then((d) => setEmpleados(d.items)).catch(() => {});
  }, []);

  const {
    items: adelantos,
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
  } = useServerTable<Adelanto, AdelantosFilters>({
    path: "/api/nomina/adelantos",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!empleadoId || !monto) return;
    setSaving(true);
    setFormError("");
    try {
      await apiFetch("/api/nomina/adelantos", {
        method: "POST",
        body: JSON.stringify({ empleado_id: empleadoId, monto, motivo: motivo || undefined }),
      });
      setMonto("");
      setMotivo("");
      refresh();
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleAprobar(id: string) {
    try {
      await apiFetch(`/api/nomina/adelantos/${id}/aprobar`, { method: "POST" });
      refresh();
    } catch (e: any) {
      setFormError(e.message);
    }
  }

  async function handleRechazar(id: string) {
    try {
      await apiFetch(`/api/nomina/adelantos/${id}/rechazar`, { method: "POST" });
      refresh();
    } catch (e: any) {
      setFormError(e.message);
    }
  }

  const selectedDisponible = empleados.find((e) => e.id === empleadoId)?.disponible_adelanto;

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Adelantos de nómina</h1>
        <p className="text-sm text-muted-foreground mt-1">Regla del 50% del sueldo mensual, sin intereses.</p>
      </div>

      <Card>
        <CardHeader><CardTitle className="text-sm">Solicitar adelanto</CardTitle></CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="grid grid-cols-1 sm:grid-cols-4 gap-3 items-end">
            <div className="sm:col-span-2 space-y-1.5">
              <Label htmlFor="empleado">Empleado</Label>
              <Select id="empleado" value={empleadoId} onChange={(e) => setEmpleadoId(e.target.value)} required>
                <option value="">Selecciona…</option>
                {empleados.map((e) => <option key={e.id} value={e.id}>{e.nombre}</option>)}
              </Select>
              {selectedDisponible && <p className="text-xs text-muted-foreground">Disponible: {formatDOP(selectedDisponible)}</p>}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="monto">Monto</Label>
              <Input id="monto" type="number" step="0.01" value={monto} onChange={(e) => setMonto(e.target.value)} placeholder="0.00" required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="motivo">Motivo</Label>
              <Input id="motivo" value={motivo} onChange={(e) => setMotivo(e.target.value)} placeholder="Opcional" />
            </div>
            <Button type="submit" disabled={saving} className="sm:col-span-4 sm:w-fit"><Plus className="h-4 w-4" />Solicitar</Button>
          </form>
          {formError && <p className="text-sm text-destructive mt-2">{formError}</p>}
        </CardContent>
      </Card>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por empleado..." className="pl-9" />
        </div>
        <Select
          value={state.filters.empleadoId || ""}
          onChange={(e) => setFilters({ empleadoId: e.target.value || undefined })}
          className="max-w-[200px]"
        >
          <option value="">Todos los empleados</option>
          {empleados.map((e) => <option key={e.id} value={e.id}>{e.nombre}</option>)}
        </Select>
        <Select
          value={state.filters.estado || ""}
          onChange={(e) => setFilters({ estado: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todos los estados</option>
          <option value="PENDIENTE">Pendiente</option>
          <option value="APROBADO">Aprobado</option>
          <option value="RECHAZADO">Rechazado</option>
          <option value="DESCONTADO">Descontado</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error}
        isEmpty={adelantos.length === 0}
        emptyIcon={<HandCoins className="h-6 w-6" />}
        emptyMessage="No hay adelantos registrados."
        maxHeight="calc(100vh - 460px)"
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
              <TableHead>Empleado</TableHead>
              <TableHead>Motivo</TableHead>
              <SortableTableHead column="monto" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Monto</SortableTableHead>
              <TableHead>Estado</TableHead>
              <TableHead className="w-20 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {adelantos.map((a) => (
              <TableRow key={a.id}>
                <TableCell className="text-xs text-muted-foreground">{new Date(a.created_at).toLocaleDateString("es-DO")}</TableCell>
                <TableCell className="font-medium">{a.empleado_nombre}</TableCell>
                <TableCell className="text-muted-foreground">{a.motivo || "—"}</TableCell>
                <TableCell className="text-right tabular-nums">{formatDOP(a.monto)}</TableCell>
                <TableCell><Badge variant={ESTADO_VARIANT[a.estado]}>{a.estado}</Badge></TableCell>
                <TableCell className="text-right">
                  {a.estado === "PENDIENTE" && (
                    <div className="flex justify-end gap-1">
                      <Button size="icon" variant="ghost" onClick={() => handleAprobar(a.id)}><Check className="h-4 w-4" /></Button>
                      <Button size="icon" variant="ghost" onClick={() => handleRechazar(a.id)}><X className="h-4 w-4" /></Button>
                    </div>
                  )}
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

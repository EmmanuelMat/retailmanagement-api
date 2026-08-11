"use client";

import { Suspense, useEffect, useState } from "react";
import { Plus, History } from "lucide-react";
import {
  Badge, Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Select, AsyncCombobox,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow, Pagination,
  ScrollableTableCard,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";

interface Producto {
  id: string;
  sku: string;
  nombre: string;
  codigo_barras?: string | null;
  stock_actual?: string;
}

interface Movimiento {
  id: string;
  producto_id: string;
  producto_nombre: string;
  producto_sku: string;
  tipo: "ENTRADA" | "SALIDA" | "AJUSTE";
  cantidad: string;
  motivo: string | null;
  created_at: string;
}

interface MovimientosFilters {
  productoId?: string;
  tipo?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

const TIPO_VARIANT: Record<string, "default" | "secondary" | "destructive"> = {
  ENTRADA: "default",
  SALIDA: "destructive",
  AJUSTE: "secondary",
};

// Búsqueda server-side por nombre, SKU o código de barras — nunca carga el
// catálogo completo (puede tener miles de SKUs).
async function buscarProductos(query: string): Promise<Producto[]> {
  const qs = new URLSearchParams({ pageSize: "20", activo: "true" });
  if (query.trim()) qs.set("search", query.trim());
  const data = await apiFetch<{ items: Producto[] }>(`/api/productos?${qs.toString()}`);
  return data.items;
}

export default function MovimientosPage() {
  return (
    <Suspense fallback={null}>
      <MovimientosPageContent />
    </Suspense>
  );
}

function MovimientosPageContent() {
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState("");

  const [productoSel, setProductoSel] = useState<Producto | null>(null);
  const [cantidad, setCantidad] = useState("");
  const [motivo, setMotivo] = useState("");

  const {
    items: movimientos,
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
  } = useServerTable<Movimiento, MovimientosFilters>({
    path: "/api/inventario/movimientos",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  // Reconstruye la etiqueta del filtro de producto cuando la página carga
  // con un ?productoId= ya en la URL (ej. al volver atrás) — el filtro en
  // sí ya funciona sin esto, esto solo repone lo que se ve en el combobox.
  const [productoFiltro, setProductoFiltro] = useState<Producto | null>(null);
  useEffect(() => {
    const id = state.filters.productoId;
    if (id && productoFiltro?.id !== id) {
      apiFetch<Producto>(`/api/productos/${id}`).then(setProductoFiltro).catch(() => {});
    } else if (!id && productoFiltro) {
      setProductoFiltro(null);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.filters.productoId]);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!productoSel || !cantidad) return;
    setSaving(true);
    setFormError("");
    try {
      await apiFetch("/api/inventario/movimientos", {
        method: "POST",
        body: JSON.stringify({ producto_id: productoSel.id, tipo: "AJUSTE", cantidad, motivo: motivo || undefined }),
      });
      setProductoSel(null);
      setCantidad("");
      setMotivo("");
      refresh();
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Movimientos · Kardex</h1>
        <p className="text-sm text-muted-foreground mt-1">Historial de entradas, salidas (automáticas) y ajustes manuales de inventario.</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Ajuste de Inventario</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="grid grid-cols-1 sm:grid-cols-4 gap-3 items-start">
            <div className="sm:col-span-2 space-y-1.5">
              <Label htmlFor="producto">Producto</Label>
              <AsyncCombobox<Producto>
                id="producto"
                value={productoSel}
                onChange={setProductoSel}
                fetchOptions={buscarProductos}
                getKey={(p) => p.id}
                getLabel={(p) => `${p.sku} · ${p.nombre}`}
                getSublabel={(p) => (p.stock_actual ? `Stock: ${p.stock_actual}` : null)}
                placeholder="Nombre, SKU o código de barras..."
                emptyMessage="Sin productos que coincidan."
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="cantidad">Cantidad</Label>
              <Input id="cantidad" type="number" step="0.01" value={cantidad} onChange={(e) => setCantidad(e.target.value)} placeholder="10" required />
              <p className="text-xs text-muted-foreground">Usa negativo para reducir stock.</p>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="motivo">Motivo</Label>
              <Input id="motivo" value={motivo} onChange={(e) => setMotivo(e.target.value)} placeholder="Ej. Merma, recuento" />
            </div>
            <Button type="submit" disabled={saving || !productoSel} className="sm:col-span-4 sm:w-fit">
              <Plus className="h-4 w-4" />
              Registrar ajuste
            </Button>
          </form>
        </CardContent>
      </Card>

      <div className="flex flex-wrap gap-3">
        <AsyncCombobox<Producto>
          value={productoFiltro}
          onChange={(p) => {
            setProductoFiltro(p);
            setFilters({ productoId: p?.id });
          }}
          fetchOptions={buscarProductos}
          getKey={(p) => p.id}
          getLabel={(p) => `${p.sku} · ${p.nombre}`}
          placeholder="Filtrar por producto..."
          emptyMessage="Sin productos que coincidan."
          className="max-w-[260px]"
        />
        <Select
          value={state.filters.tipo || ""}
          onChange={(e) => setFilters({ tipo: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todos los tipos</option>
          <option value="ENTRADA">Entrada</option>
          <option value="SALIDA">Salida</option>
          <option value="AJUSTE">Ajuste</option>
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
        error={error || formError || null}
        isEmpty={movimientos.length === 0}
        emptyIcon={<History className="h-6 w-6" />}
        emptyMessage="Aún no hay movimientos registrados."
        maxHeight="calc(100vh - 340px)"
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
              <TableHead>Producto</TableHead>
              <TableHead>Tipo</TableHead>
              <SortableTableHead column="cantidad" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Cantidad</SortableTableHead>
              <TableHead>Motivo</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {movimientos.map((m) => (
              <TableRow key={m.id}>
                <TableCell className="text-muted-foreground text-xs">{new Date(m.created_at).toLocaleString("es-DO")}</TableCell>
                <TableCell>
                  <span className="font-medium">{m.producto_nombre}</span>{" "}
                  <span className="text-xs text-muted-foreground font-mono">{m.producto_sku}</span>
                </TableCell>
                <TableCell><Badge variant={TIPO_VARIANT[m.tipo]}>{m.tipo}</Badge></TableCell>
                <TableCell className={`text-right font-mono tabular-nums ${Number(m.cantidad) < 0 ? "text-destructive" : ""}`}>
                  {Number(m.cantidad) > 0 ? "+" : ""}{m.cantidad}
                </TableCell>
                <TableCell className="text-muted-foreground">{m.motivo || "—"}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

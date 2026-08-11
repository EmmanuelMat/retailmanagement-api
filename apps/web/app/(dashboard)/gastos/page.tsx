"use client";

import { Suspense, useEffect, useState } from "react";
import { Plus, Receipt, Search } from "lucide-react";
import {
  Button, Card, CardHeader, CardTitle, CardContent, Input, Label, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Gasto {
  id: string;
  concepto: string;
  categoria: string;
  monto: string;
  created_at: string;
}

interface GastosFilters {
  categoria?: string;
  search?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

export default function GastosPage() {
  return (
    <Suspense fallback={null}>
      <GastosPageContent />
    </Suspense>
  );
}

function GastosPageContent() {
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState("");
  const [concepto, setConcepto] = useState("");
  const [categoria, setCategoria] = useState("OTROS");
  const [monto, setMonto] = useState("");

  useEffect(() => {
    // Si venimos del Asistente con una acción confirmada, precarga el
    // formulario de arriba - la creación real sigue pasando por el mismo
    // handleCreate/POST de siempre, la IA nunca crea nada directamente.
    const raw = sessionStorage.getItem("ai_prefill_gasto");
    if (!raw) return;
    sessionStorage.removeItem("ai_prefill_gasto");
    try {
      const parsed = JSON.parse(raw);
      if (parsed.concepto) setConcepto(String(parsed.concepto));
      if (parsed.categoria) setCategoria(String(parsed.categoria));
      if (parsed.monto) setMonto(String(parsed.monto));
    } catch {}
  }, []);

  const {
    items: gastos,
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
  } = useServerTable<Gasto, GastosFilters>({
    path: "/api/gastos",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!concepto.trim() || !monto) return;
    setSaving(true);
    setFormError("");
    try {
      await apiFetch("/api/gastos", { method: "POST", body: JSON.stringify({ concepto, categoria, monto }) });
      setConcepto("");
      setMonto("");
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
        <h1 className="text-2xl font-bold font-serif tracking-tight">Gastos</h1>
        <p className="text-sm text-muted-foreground mt-1">Gastos operativos que no pasan por inventario (alquiler, servicios, transporte).</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Registrar gasto</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="grid grid-cols-1 sm:grid-cols-4 gap-3 items-start">
            <div className="sm:col-span-2 space-y-1.5">
              <Label htmlFor="concepto">Concepto</Label>
              <Input id="concepto" value={concepto} onChange={(e) => setConcepto(e.target.value)} placeholder="Factura de luz" required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="categoria">Categoría</Label>
              <Select id="categoria" value={categoria} onChange={(e) => setCategoria(e.target.value)}>
                <option value="ALQUILER">Alquiler</option>
                <option value="SERVICIOS">Servicios</option>
                <option value="TRANSPORTE">Transporte</option>
                <option value="OTROS">Otros</option>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="monto">Monto</Label>
              <Input id="monto" type="number" step="0.01" value={monto} onChange={(e) => setMonto(e.target.value)} placeholder="0.00" required />
            </div>
            <Button type="submit" disabled={saving} className="sm:col-span-4 sm:w-fit">
              <Plus className="h-4 w-4" />
              Registrar
            </Button>
          </form>
          {formError && <p className="text-sm text-destructive mt-2">{formError}</p>}
        </CardContent>
      </Card>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por concepto..." className="pl-9" />
        </div>
        <Select
          value={state.filters.categoria || ""}
          onChange={(e) => setFilters({ categoria: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todas las categorías</option>
          <option value="ALQUILER">Alquiler</option>
          <option value="SERVICIOS">Servicios</option>
          <option value="TRANSPORTE">Transporte</option>
          <option value="OTROS">Otros</option>
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
        isEmpty={gastos.length === 0}
        emptyIcon={<Receipt className="h-6 w-6" />}
        emptyMessage="No hay gastos registrados."
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
              <TableHead>Concepto</TableHead>
              <TableHead>Categoría</TableHead>
              <SortableTableHead column="monto" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Monto</SortableTableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {gastos.map((g) => (
              <TableRow key={g.id}>
                <TableCell className="text-xs text-muted-foreground">{new Date(g.created_at).toLocaleDateString("es-DO")}</TableCell>
                <TableCell className="font-medium">{g.concepto}</TableCell>
                <TableCell className="text-muted-foreground">{g.categoria}</TableCell>
                <TableCell className="text-right font-mono tabular-nums">{formatDOP(g.monto)}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

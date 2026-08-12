"use client";

import { Suspense, useState } from "react";
import { Plus, Pencil, Trash2, Tags, Search } from "lucide-react";
import {
  Button, Card, CardHeader, CardTitle, CardContent, Input, Label, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Categoria {
  id: string;
  nombre: string;
  color: string | null;
  icono: string | null;
  orden: number;
  activo: boolean;
}

interface CategoriasFilters {
  search?: string;
  activo?: string;
}

export default function CategoriasPage() {
  return (
    <Suspense fallback={null}>
      <CategoriasPageContent />
    </Suspense>
  );
}

function CategoriasPageContent() {
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState("");
  const [actionError, setActionError] = useState("");
  const [nombre, setNombre] = useState("");
  const [icono, setIcono] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editNombre, setEditNombre] = useState("");

  const {
    items: categorias,
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
  } = useServerTable<Categoria, CategoriasFilters>({
    path: "/api/categorias",
    initialPageSize: 20,
    initialSortBy: "orden",
    initialSortDir: "asc",
    initialFilters: { activo: "true" },
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!nombre.trim()) return;
    setSaving(true);
    setFormError("");
    try {
      await apiFetch("/api/categorias", { method: "POST", body: JSON.stringify({ nombre, icono: icono || undefined }) });
      setNombre("");
      setIcono("");
      refresh();
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setSaving(false);
    }
  }

  function startEdit(cat: Categoria) {
    setEditingId(cat.id);
    setEditNombre(cat.nombre);
  }

  async function saveEdit(id: string) {
    try {
      await apiFetch(`/api/categorias/${id}`, { method: "PUT", body: JSON.stringify({ nombre: editNombre }) });
      setEditingId(null);
      refresh();
    } catch (e: any) {
      setActionError(e.message);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm("¿Desactivar esta categoría?")) return;
    try {
      await apiFetch(`/api/categorias/${id}`, { method: "DELETE" });
      refresh();
    } catch (e: any) {
      setActionError(e.message);
    }
  }

  return (
    <div className="max-w-3xl space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Categorías</h1>
        <p className="text-sm text-muted-foreground mt-1">Organiza tus productos por categoría (Víveres, Bebidas, Limpieza, etc).</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Nueva categoría</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="flex items-end gap-3">
            <div className="flex-1 space-y-1.5">
              <Label htmlFor="nombre">Nombre</Label>
              <Input id="nombre" value={nombre} onChange={(e) => setNombre(e.target.value)} placeholder="Víveres" required />
            </div>
            <div className="w-24 space-y-1.5">
              <Label htmlFor="icono">Ícono</Label>
              <Input id="icono" value={icono} onChange={(e) => setIcono(e.target.value)} placeholder="🌽" maxLength={2} />
            </div>
            <Button type="submit" disabled={saving}>
              <Plus className="h-4 w-4" />
              Agregar
            </Button>
          </form>
          {formError && <p className="text-sm text-destructive mt-2">{formError}</p>}
        </CardContent>
      </Card>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por nombre..." className="pl-9" />
        </div>
        <Select
          value={state.filters.activo ?? ""}
          onChange={(e) => setFilters({ activo: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="true">Activas</option>
          <option value="false">Inactivas</option>
          <option value="">Todas</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error || actionError || null}
        isEmpty={categorias.length === 0}
        emptyIcon={<Tags className="h-6 w-6" />}
        emptyMessage="Aún no hay categorías. Crea la primera arriba."
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
              <SortableTableHead column="nombre" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Categoría</SortableTableHead>
              <TableHead className="w-32 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {categorias.map((cat) => (
              <TableRow key={cat.id}>
                <TableCell>
                  {editingId === cat.id ? (
                    <Input value={editNombre} onChange={(e) => setEditNombre(e.target.value)} className="h-8 max-w-xs" />
                  ) : (
                    <span className="flex items-center gap-2">
                      {cat.icono && <span>{cat.icono}</span>}
                      {cat.nombre}
                    </span>
                  )}
                </TableCell>
                <TableCell className="text-right">
                  {editingId === cat.id ? (
                    <div className="flex justify-end gap-2">
                      <Button size="sm" variant="secondary" onClick={() => setEditingId(null)}>Cancelar</Button>
                      <Button size="sm" onClick={() => saveEdit(cat.id)}>Guardar</Button>
                    </div>
                  ) : (
                    <div className="flex justify-end gap-1">
                      <Button size="icon" variant="ghost" onClick={() => startEdit(cat)}>
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button size="icon" variant="ghost" onClick={() => handleDelete(cat.id)}>
                        <Trash2 className="h-4 w-4" />
                      </Button>
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

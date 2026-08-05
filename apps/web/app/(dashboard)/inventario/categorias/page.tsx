"use client";

import { useEffect, useState } from "react";
import { Plus, Pencil, Trash2, Tags } from "lucide-react";
import { Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Categoria {
  id: string;
  nombre: string;
  color: string | null;
  icono: string | null;
  orden: number;
  activo: boolean;
}

export default function CategoriasPage() {
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [nombre, setNombre] = useState("");
  const [icono, setIcono] = useState("");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editNombre, setEditNombre] = useState("");

  async function load() {
    setLoading(true);
    setError("");
    try {
      const data = await apiFetch<{ categorias: Categoria[] }>("/api/categorias");
      setCategorias(data.categorias);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!nombre.trim()) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/categorias", { method: "POST", body: JSON.stringify({ nombre, icono: icono || undefined }) });
      setNombre("");
      setIcono("");
      await load();
    } catch (e: any) {
      setError(e.message);
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
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function handleDelete(id: string) {
    if (!confirm("¿Desactivar esta categoría?")) return;
    try {
      await apiFetch(`/api/categorias/${id}`, { method: "DELETE" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="max-w-3xl space-y-6">
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
        </CardContent>
      </Card>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : categorias.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <Tags className="h-6 w-6" />
              Aún no hay categorías. Crea la primera arriba.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Categoría</TableHead>
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
          )}
        </CardContent>
      </Card>
    </div>
  );
}

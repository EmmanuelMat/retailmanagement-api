"use client";

import { useEffect, useState } from "react";
import { Plus, LayoutGrid } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label } from "@repo/ui";
import { apiFetch } from "@/lib/staff-api";

interface ModuloCatalogo {
  codigo: string;
  nombre: string;
  descripcion: string | null;
  activo: boolean;
}

const EMPTY = { codigo: "", nombre: "", descripcion: "" };

export default function ModulosCatalogoPage() {
  const [modulos, setModulos] = useState<ModuloCatalogo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [showForm, setShowForm] = useState(false);
  const [values, setValues] = useState(EMPTY);
  const [saving, setSaving] = useState(false);

  function load() {
    setLoading(true);
    apiFetch<ModuloCatalogo[]>("/api/modulos")
      .then(setModulos)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }

  useEffect(load, []);

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/modulos", {
        method: "POST",
        body: JSON.stringify({
          codigo: values.codigo.trim().toUpperCase().replace(/\s+/g, "_"),
          nombre: values.nombre.trim(),
          descripcion: values.descripcion.trim() || undefined,
        }),
      });
      setValues(EMPTY);
      setShowForm(false);
      load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Catálogo de módulos</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Módulos vendibles. Agregar uno aquí lo deja disponible para asignar a negocios — conectarlo a rutas
            reales del sistema todavía requiere un cambio de código puntual.
          </p>
        </div>
        <Button onClick={() => setShowForm((s) => !s)}>
          <Plus className="h-4 w-4" />Nuevo módulo
        </Button>
      </div>

      {showForm && (
        <Card className="max-w-xl">
          <CardContent className="pt-5">
            <form onSubmit={handleSubmit} className="space-y-4">
              <div className="space-y-1.5">
                <Label htmlFor="codigo">Código *</Label>
                <Input
                  id="codigo"
                  required
                  value={values.codigo}
                  onChange={(e) => setValues((v) => ({ ...v, codigo: e.target.value }))}
                  placeholder="EJ_NUEVO_MODULO"
                  className="font-mono"
                />
                <p className="text-xs text-muted-foreground">Identificador único, en mayúsculas y guiones bajos.</p>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="nombre">Nombre *</Label>
                <Input id="nombre" required value={values.nombre} onChange={(e) => setValues((v) => ({ ...v, nombre: e.target.value }))} placeholder="Módulo de ejemplo" />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="descripcion">Descripción</Label>
                <Input id="descripcion" value={values.descripcion} onChange={(e) => setValues((v) => ({ ...v, descripcion: e.target.value }))} />
              </div>
              {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
              <div className="flex gap-3">
                <Button type="submit" disabled={saving}>{saving ? "Creando..." : "Crear módulo"}</Button>
                <Button type="button" variant="secondary" onClick={() => setShowForm(false)}>Cancelar</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      {!showForm && error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
        {loading ? (
          <p className="text-sm text-muted-foreground">Cargando...</p>
        ) : modulos.length === 0 ? (
          <div className="col-span-full text-center text-muted-foreground py-12">
            <LayoutGrid className="h-6 w-6 mx-auto mb-2 opacity-50" />
            No hay módulos en el catálogo.
          </div>
        ) : (
          modulos.map((m) => (
            <Card key={m.codigo}>
              <CardContent className="pt-4 pb-4">
                <div className="flex items-start justify-between gap-2">
                  <div className="min-w-0">
                    <p className="text-sm font-semibold">{m.nombre}</p>
                    <p className="text-xs font-mono text-muted-foreground mt-0.5">{m.codigo}</p>
                  </div>
                  <Badge variant={m.activo ? "success" : "secondary"}>{m.activo ? "Activo" : "Inactivo"}</Badge>
                </div>
                {m.descripcion && <p className="text-sm text-muted-foreground mt-2">{m.descripcion}</p>}
              </CardContent>
            </Card>
          ))
        )}
      </div>
    </div>
  );
}

"use client";

import { useEffect, useState } from "react";
import { Plus, ShieldCheck, ChevronDown, ChevronRight } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label } from "@repo/ui";
import { apiFetch } from "@/lib/staff-api";

interface PermisoCatalogo {
  codigo: string;
  nombre: string;
  descripcion: string | null;
  orden: number;
  activo: boolean;
}

interface RolConPermisos {
  id: string;
  codigo: string;
  nombre: string;
  es_admin: boolean;
  created_at: string;
  permisos: string[];
}

const EMPTY = { codigo: "", nombre: "" };

export default function RolesPage() {
  const [roles, setRoles] = useState<RolConPermisos[]>([]);
  const [permisos, setPermisos] = useState<PermisoCatalogo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const [showForm, setShowForm] = useState(false);
  const [values, setValues] = useState(EMPTY);
  const [nuevosPermisos, setNuevosPermisos] = useState<Set<string>>(new Set());
  const [saving, setSaving] = useState(false);

  const [abierto, setAbierto] = useState<string | null>(null);

  async function load() {
    setLoading(true);
    setError("");
    try {
      const [r, p] = await Promise.all([
        apiFetch<RolConPermisos[]>("/api/roles"),
        apiFetch<PermisoCatalogo[]>("/api/permisos"),
      ]);
      setRoles(r);
      setPermisos(p);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function crear(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/roles", {
        method: "POST",
        body: JSON.stringify({
          codigo: values.codigo.trim().toUpperCase().replace(/\s+/g, "_"),
          nombre: values.nombre.trim(),
          permisos: [...nuevosPermisos],
        }),
      });
      setValues(EMPTY);
      setNuevosPermisos(new Set());
      setShowForm(false);
      await load();
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
          <h1 className="text-2xl font-bold tracking-tight">Roles</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Bundles de permisos que se asignan a los usuarios de cada negocio (pestaña Usuarios, dentro de cada
            negocio). Compartido entre todos los negocios — crear un rol aquí lo deja disponible para todos.
          </p>
        </div>
        <Button onClick={() => setShowForm((s) => !s)}><Plus className="h-4 w-4" />Nuevo rol</Button>
      </div>

      {showForm && (
        <Card className="max-w-2xl">
          <CardContent className="pt-5">
            <form onSubmit={crear} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="codigo">Código *</Label>
                  <Input id="codigo" required value={values.codigo} onChange={(e) => setValues((v) => ({ ...v, codigo: e.target.value }))} placeholder="CAJERO_SENIOR" className="font-mono" />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="nombre">Nombre *</Label>
                  <Input id="nombre" required value={values.nombre} onChange={(e) => setValues((v) => ({ ...v, nombre: e.target.value }))} placeholder="Cajero Senior" />
                </div>
              </div>
              <div className="space-y-1.5">
                <Label>Permisos</Label>
                <PermisoCheckboxGrid
                  permisos={permisos}
                  seleccionados={nuevosPermisos}
                  onToggle={(codigo) =>
                    setNuevosPermisos((prev) => {
                      const next = new Set(prev);
                      if (next.has(codigo)) next.delete(codigo);
                      else next.add(codigo);
                      return next;
                    })
                  }
                />
              </div>
              {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
              <div className="flex gap-3">
                <Button type="submit" disabled={saving}>{saving ? "Creando..." : "Crear rol"}</Button>
                <Button type="button" variant="secondary" onClick={() => setShowForm(false)}>Cancelar</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      {!showForm && error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      {loading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : (
        <div className="space-y-3">
          {roles.map((r) => (
            <RolCard
              key={r.id}
              rol={r}
              permisos={permisos}
              abierto={abierto === r.id}
              onToggleAbierto={() => setAbierto((a) => (a === r.id ? null : r.id))}
              onSaved={load}
            />
          ))}
        </div>
      )}
    </div>
  );
}

function PermisoCheckboxGrid({
  permisos,
  seleccionados,
  onToggle,
}: {
  permisos: PermisoCatalogo[];
  seleccionados: Set<string>;
  onToggle: (codigo: string) => void;
}) {
  return (
    <div className="rounded-md border border-border divide-y divide-border max-h-80 overflow-y-auto">
      {permisos.map((p) => (
        <label key={p.codigo} className="flex items-start gap-3 py-2 px-3 cursor-pointer hover:bg-muted/50">
          <input
            type="checkbox"
            checked={seleccionados.has(p.codigo)}
            onChange={() => onToggle(p.codigo)}
            className="h-4 w-4 mt-0.5 rounded border-border"
          />
          <div className="min-w-0">
            <p className="text-sm font-medium">{p.nombre}</p>
            <p className="text-xs font-mono text-muted-foreground">{p.codigo}</p>
          </div>
        </label>
      ))}
    </div>
  );
}

function RolCard({
  rol,
  permisos,
  abierto,
  onToggleAbierto,
  onSaved,
}: {
  rol: RolConPermisos;
  permisos: PermisoCatalogo[];
  abierto: boolean;
  onToggleAbierto: () => void;
  onSaved: () => void;
}) {
  const [seleccionados, setSeleccionados] = useState<Set<string>>(new Set(rol.permisos));
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    setSeleccionados(new Set(rol.permisos));
  }, [rol.permisos]);

  async function guardar() {
    setSaving(true);
    setError("");
    try {
      await apiFetch(`/api/roles/${rol.id}/permisos`, {
        method: "PUT",
        body: JSON.stringify({ permisos: [...seleccionados] }),
      });
      setSaved(true);
      onSaved();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card>
      <CardContent className="pt-4 pb-4">
        <button onClick={onToggleAbierto} className="flex items-center justify-between w-full text-left">
          <div className="flex items-center gap-2.5">
            {abierto ? <ChevronDown className="h-4 w-4 text-muted-foreground shrink-0" /> : <ChevronRight className="h-4 w-4 text-muted-foreground shrink-0" />}
            <div>
              <p className="text-sm font-semibold">{rol.nombre}</p>
              <p className="text-xs font-mono text-muted-foreground">{rol.codigo}</p>
            </div>
          </div>
          {rol.es_admin ? (
            <Badge variant="success"><ShieldCheck className="h-3 w-3" />Acceso total</Badge>
          ) : (
            <Badge variant="secondary">{rol.permisos.length} permisos</Badge>
          )}
        </button>

        {abierto && (
          <div className="mt-4 pt-4 border-t border-border space-y-3">
            {rol.es_admin ? (
              <p className="text-sm text-muted-foreground">
                Este rol tiene acceso total (bypass) — no se le puede quitar ni agregar permisos individuales.
              </p>
            ) : (
              <>
                <PermisoCheckboxGrid
                  permisos={permisos}
                  seleccionados={seleccionados}
                  onToggle={(codigo) =>
                    setSeleccionados((prev) => {
                      const next = new Set(prev);
                      if (next.has(codigo)) next.delete(codigo);
                      else next.add(codigo);
                      setSaved(false);
                      return next;
                    })
                  }
                />
                {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
                {saved && <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">Permisos actualizados.</div>}
                <Button size="sm" onClick={guardar} disabled={saving}>{saving ? "Guardando..." : "Guardar permisos"}</Button>
              </>
            )}
          </div>
        )}
      </CardContent>
    </Card>
  );
}

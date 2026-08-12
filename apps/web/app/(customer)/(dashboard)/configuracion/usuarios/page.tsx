"use client";

import { Fragment, Suspense, useState } from "react";
import { Plus, Trash2, UserCog, KeyRound, Search } from "lucide-react";
import {
  Badge, Button, Card, CardContent, Input, Label, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Usuario {
  id: string;
  nombre: string;
  email: string;
  rol: string;
  descuento_maximo_sin_aprobacion: string;
  activo: boolean;
}

interface UsuariosFilters {
  search?: string;
  rol?: string;
  activo?: string;
}

const ROLES = ["ADMIN", "CAJERO", "ALMACEN", "CONTADOR"];

const EMPTY_NEW = { nombre: "", email: "", password: "", rol: "CAJERO", descuento_maximo_sin_aprobacion: "0" };

export default function UsuariosPage() {
  return (
    <Suspense fallback={null}>
      <UsuariosPageContent />
    </Suspense>
  );
}

function UsuariosPageContent() {
  const [formError, setFormError] = useState("");
  const [actionError, setActionError] = useState("");
  const [showForm, setShowForm] = useState(false);
  const [nuevo, setNuevo] = useState(EMPTY_NEW);
  const [saving, setSaving] = useState(false);
  const [resetId, setResetId] = useState<string | null>(null);
  const [resetPassword, setResetPassword] = useState("");
  const [resetSaving, setResetSaving] = useState(false);
  const [resetError, setResetError] = useState("");
  const [eliminandoId, setEliminandoId] = useState<string | null>(null);

  const {
    items: usuarios,
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
  } = useServerTable<Usuario, UsuariosFilters>({
    path: "/api/config/usuarios",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "asc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setFormError("");
    try {
      await apiFetch("/api/config/usuarios", { method: "POST", body: JSON.stringify(nuevo) });
      setNuevo(EMPTY_NEW);
      setShowForm(false);
      refresh();
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleRolChange(u: Usuario, rol: string) {
    try {
      await apiFetch(`/api/config/usuarios/${u.id}`, { method: "PUT", body: JSON.stringify({ nombre: u.nombre, rol, activo: u.activo }) });
      refresh();
    } catch (e: any) {
      setActionError(e.message);
    }
  }

  async function handleDescuentoChange(u: Usuario, value: string) {
    try {
      await apiFetch(`/api/config/usuarios/${u.id}`, {
        method: "PUT",
        body: JSON.stringify({ nombre: u.nombre, rol: u.rol, activo: u.activo, descuento_maximo_sin_aprobacion: value || "0" }),
      });
      refresh();
    } catch (e: any) {
      setActionError(e.message);
    }
  }

  async function handleToggleActivo(u: Usuario) {
    try {
      await apiFetch(`/api/config/usuarios/${u.id}`, { method: "PUT", body: JSON.stringify({ nombre: u.nombre, rol: u.rol, activo: !u.activo }) });
      refresh();
    } catch (e: any) {
      setActionError(e.message);
    }
  }

  function abrirReset(id: string) {
    setResetId(id);
    setResetPassword("");
    setResetError("");
  }

  async function handleReset(u: Usuario) {
    if (resetPassword.length < 8) {
      setResetError("La contraseña debe tener al menos 8 caracteres");
      return;
    }
    setResetSaving(true);
    setResetError("");
    try {
      await apiFetch(`/api/config/usuarios/${u.id}`, {
        method: "PUT",
        body: JSON.stringify({ nombre: u.nombre, rol: u.rol, activo: u.activo, password: resetPassword }),
      });
      setResetId(null);
    } catch (e: any) {
      setResetError(e.message);
    } finally {
      setResetSaving(false);
    }
  }

  async function handleDeactivate(id: string) {
    try {
      await apiFetch(`/api/config/usuarios/${id}`, { method: "DELETE" });
      setEliminandoId(null);
      refresh();
    } catch (e: any) {
      setActionError(e.message);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Usuarios y roles</h1>
          <p className="text-sm text-muted-foreground mt-1">Cajeros, almacén, contador y administradores del negocio.</p>
        </div>
        <Button onClick={() => setShowForm((s) => !s)}><Plus className="h-4 w-4" />Nuevo usuario</Button>
      </div>

      {showForm && (
        <Card className="max-w-xl">
          <CardContent className="pt-5">
            <form onSubmit={handleCreate} className="space-y-4">
              <div className="grid grid-cols-2 gap-4">
                <div className="space-y-1.5">
                  <Label htmlFor="nombre">Nombre *</Label>
                  <Input id="nombre" required value={nuevo.nombre} onChange={(e) => setNuevo((v) => ({ ...v, nombre: e.target.value }))} />
                </div>
                <div className="space-y-1.5">
                  <Label htmlFor="rol">Rol *</Label>
                  <Select id="rol" required value={nuevo.rol} onChange={(e) => setNuevo((v) => ({ ...v, rol: e.target.value }))}>
                    {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
                  </Select>
                </div>
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="email">Correo *</Label>
                <Input id="email" type="email" required value={nuevo.email} onChange={(e) => setNuevo((v) => ({ ...v, email: e.target.value }))} />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="password">Contraseña *</Label>
                <Input id="password" type="password" required minLength={8} value={nuevo.password} onChange={(e) => setNuevo((v) => ({ ...v, password: e.target.value }))} />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="descuento">Límite de descuento sin aprobación (RD$)</Label>
                <Input
                  id="descuento"
                  type="number"
                  min={0}
                  step="0.01"
                  value={nuevo.descuento_maximo_sin_aprobacion}
                  onChange={(e) => setNuevo((v) => ({ ...v, descuento_maximo_sin_aprobacion: e.target.value }))}
                />
                <p className="text-xs text-muted-foreground">Por encima de este monto, el POS pedirá aprobación de un administrador. No aplica a rol ADMIN.</p>
              </div>
              {formError && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{formError}</div>}
              <div className="flex gap-3">
                <Button type="submit" disabled={saving}>{saving ? "Creando..." : "Crear usuario"}</Button>
                <Button type="button" variant="secondary" onClick={() => setShowForm(false)}>Cancelar</Button>
              </div>
            </form>
          </CardContent>
        </Card>
      )}

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por nombre o correo..." className="pl-9" />
        </div>
        <Select
          value={state.filters.rol || ""}
          onChange={(e) => setFilters({ rol: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todos los roles</option>
          {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
        </Select>
        <Select
          value={state.filters.activo ?? ""}
          onChange={(e) => setFilters({ activo: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="">Todos</option>
          <option value="true">Activos</option>
          <option value="false">Inactivos</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error || actionError || null}
        isEmpty={usuarios.length === 0}
        emptyIcon={<UserCog className="h-6 w-6" />}
        emptyMessage="No hay usuarios registrados."
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
              <SortableTableHead column="nombre" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Nombre</SortableTableHead>
              <TableHead>Correo</TableHead>
              <SortableTableHead column="rol" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Rol</SortableTableHead>
              <TableHead>Límite descuento (RD$)</TableHead>
              <TableHead>Estado</TableHead>
              <TableHead className="w-24 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {usuarios.map((u) => (
              <Fragment key={u.id}>
                <TableRow>
                  <TableCell className="font-medium">{u.nombre}</TableCell>
                  <TableCell className="text-muted-foreground">{u.email}</TableCell>
                  <TableCell>
                    <Select value={u.rol} onChange={(e) => handleRolChange(u, e.target.value)} className="h-8 w-36">
                      {ROLES.map((r) => <option key={r} value={r}>{r}</option>)}
                    </Select>
                  </TableCell>
                  <TableCell>
                    {u.rol === "ADMIN" ? (
                      <span className="text-xs text-muted-foreground">Ilimitado</span>
                    ) : (
                      <Input
                        type="number"
                        min={0}
                        step="0.01"
                        defaultValue={u.descuento_maximo_sin_aprobacion}
                        onBlur={(e) => e.target.value !== u.descuento_maximo_sin_aprobacion && handleDescuentoChange(u, e.target.value)}
                        className="h-8 w-28 text-sm"
                      />
                    )}
                  </TableCell>
                  <TableCell>
                    <button onClick={() => handleToggleActivo(u)}>
                      <Badge variant={u.activo ? "success" : "secondary"}>{u.activo ? "Activo" : "Inactivo"}</Badge>
                    </button>
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      <Button size="icon" variant="ghost" title="Restablecer contraseña" onClick={() => abrirReset(u.id)}>
                        <KeyRound className="h-4 w-4" />
                      </Button>
                      <Button size="icon" variant="ghost" onClick={() => setEliminandoId(u.id)}><Trash2 className="h-4 w-4" /></Button>
                    </div>
                  </TableCell>
                </TableRow>
                {eliminandoId === u.id && (
                  <TableRow>
                    <TableCell colSpan={6} className="bg-muted/30">
                      <div className="flex flex-wrap items-center gap-2 py-1">
                        <span className="text-sm">¿Desactivar a {u.nombre}?</span>
                        <Button size="sm" variant="destructive" onClick={() => handleDeactivate(u.id)}>Desactivar</Button>
                        <Button size="sm" variant="ghost" onClick={() => setEliminandoId(null)}>Cancelar</Button>
                      </div>
                    </TableCell>
                  </TableRow>
                )}
                {resetId === u.id && (
                  <TableRow>
                    <TableCell colSpan={6} className="bg-muted/30">
                      <div className="flex flex-wrap items-end gap-2 py-1">
                        <div className="space-y-1">
                          <label className="text-xs text-muted-foreground">Nueva contraseña para {u.nombre}</label>
                          <Input
                            type="password"
                            minLength={8}
                            value={resetPassword}
                            onChange={(e) => setResetPassword(e.target.value)}
                            placeholder="Mínimo 8 caracteres"
                            className="h-8 w-48 text-sm"
                          />
                        </div>
                        <Button size="sm" disabled={resetSaving} onClick={() => handleReset(u)}>
                          {resetSaving ? "Guardando..." : "Restablecer"}
                        </Button>
                        <Button size="sm" variant="ghost" onClick={() => setResetId(null)}>Cancelar</Button>
                        {resetError && <span className="text-xs text-destructive">{resetError}</span>}
                      </div>
                    </TableCell>
                  </TableRow>
                )}
              </Fragment>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

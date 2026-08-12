"use client";

import { Fragment, Suspense, useState } from "react";
import Link from "next/link";
import { Plus, Pencil, Trash2, Users, Search, HandCoins } from "lucide-react";
import {
  Badge, Button, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Cliente {
  id: string;
  nombre: string;
  rnc_cedula: string | null;
  telefono: string | null;
  email: string | null;
  saldo_pendiente: string;
  limite_credito: string;
}

interface ClientesFilters {
  search?: string;
  activo?: string;
}

export default function ClientesPage() {
  return (
    <Suspense fallback={null}>
      <ClientesPageContent />
    </Suspense>
  );
}

function ClientesPageContent() {
  const [error, setError] = useState("");
  const [eliminandoId, setEliminandoId] = useState<string | null>(null);
  const [abonandoId, setAbonandoId] = useState<string | null>(null);
  const [abonoMonto, setAbonoMonto] = useState("");
  const [abonoMetodo, setAbonoMetodo] = useState("EFECTIVO");
  const [abonoGuardando, setAbonoGuardando] = useState(false);
  const [abonoError, setAbonoError] = useState("");

  const {
    items: clientes,
    total,
    totalPages,
    loading,
    error: loadError,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
    refresh,
  } = useServerTable<Cliente, ClientesFilters>({
    path: "/api/clientes",
    initialPageSize: 20,
    initialSortBy: "nombre",
    initialSortDir: "asc",
    initialFilters: { activo: "true" },
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function handleDelete(id: string) {
    try {
      await apiFetch(`/api/clientes/${id}`, { method: "DELETE" });
      setEliminandoId(null);
      refresh();
    } catch (e: any) {
      setError(e.message);
    }
  }

  function abrirAbono(id: string) {
    setAbonandoId(id);
    setAbonoMonto("");
    setAbonoMetodo("EFECTIVO");
    setAbonoError("");
  }

  async function handleAbonar(id: string) {
    const monto = Number(abonoMonto);
    if (!monto || monto <= 0) {
      setAbonoError("Ingresa un monto válido");
      return;
    }
    setAbonoGuardando(true);
    setAbonoError("");
    try {
      await apiFetch(`/api/clientes/${id}/abonos`, {
        method: "POST",
        body: JSON.stringify({ monto: String(monto), metodo_pago: abonoMetodo }),
      });
      setAbonandoId(null);
      refresh();
    } catch (e: any) {
      setAbonoError(e.message);
    } finally {
      setAbonoGuardando(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Clientes</h1>
          <p className="text-sm text-muted-foreground mt-1">Consumidor final o clientes con RNC para crédito fiscal.</p>
        </div>
        <Link href="/clientes/nuevo">
          <Button><Plus className="h-4 w-4" />Nuevo cliente</Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por nombre o RNC..." className="pl-9" />
        </div>
        <Select
          value={state.filters.activo ?? ""}
          onChange={(e) => setFilters({ activo: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="true">Activos</option>
          <option value="false">Inactivos</option>
          <option value="">Todos</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={loadError || error || null}
        isEmpty={clientes.length === 0}
        emptyIcon={<Users className="h-6 w-6" />}
        emptyMessage="No hay clientes todavía."
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
              <TableHead>RNC / Cédula</TableHead>
              <TableHead>Teléfono</TableHead>
              <TableHead>Correo</TableHead>
              <SortableTableHead column="saldo_pendiente" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Saldo fiado</SortableTableHead>
              <TableHead>Límite crédito</TableHead>
              <TableHead className="w-32 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {clientes.map((c) => {
              const saldo = Number(c.saldo_pendiente || "0");
              return (
                <Fragment key={c.id}>
                  <TableRow>
                    <TableCell className="font-medium">{c.nombre}</TableCell>
                    <TableCell className="font-mono text-xs text-muted-foreground">{c.rnc_cedula || "—"}</TableCell>
                    <TableCell className="text-muted-foreground">{c.telefono || "—"}</TableCell>
                    <TableCell className="text-muted-foreground">{c.email || "—"}</TableCell>
                    <TableCell>
                      {saldo > 0 ? <Badge variant="warning">{formatDOP(c.saldo_pendiente)}</Badge> : <span className="text-muted-foreground">—</span>}
                    </TableCell>
                    <TableCell className="text-muted-foreground">{formatDOP(c.limite_credito)}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        {saldo > 0 && (
                          <Button size="icon" variant="ghost" title="Registrar abono" onClick={() => abrirAbono(c.id)}>
                            <HandCoins className="h-4 w-4" />
                          </Button>
                        )}
                        <Link href={`/clientes/${c.id}` as any}>
                          <Button size="icon" variant="ghost"><Pencil className="h-4 w-4" /></Button>
                        </Link>
                        <Button size="icon" variant="ghost" onClick={() => setEliminandoId(c.id)}><Trash2 className="h-4 w-4" /></Button>
                      </div>
                    </TableCell>
                  </TableRow>
                  {eliminandoId === c.id && (
                    <TableRow>
                      <TableCell colSpan={7} className="bg-muted/30">
                        <div className="flex flex-wrap items-center gap-2 py-1">
                          <span className="text-sm">¿Desactivar a {c.nombre}?</span>
                          <Button size="sm" variant="destructive" onClick={() => handleDelete(c.id)}>Desactivar</Button>
                          <Button size="sm" variant="ghost" onClick={() => setEliminandoId(null)}>Cancelar</Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  )}
                  {abonandoId === c.id && (
                    <TableRow>
                      <TableCell colSpan={7} className="bg-muted/30">
                        <div className="flex flex-wrap items-end gap-2 py-1">
                          <div className="space-y-1">
                            <label className="text-xs text-muted-foreground">Monto del abono</label>
                            <Input
                              type="number"
                              min={0}
                              step="0.01"
                              value={abonoMonto}
                              onChange={(e) => setAbonoMonto(e.target.value)}
                              placeholder="0.00"
                              className="h-8 w-32 text-sm"
                            />
                          </div>
                          <div className="space-y-1">
                            <label className="text-xs text-muted-foreground">Método</label>
                            <Select value={abonoMetodo} onChange={(e) => setAbonoMetodo(e.target.value)} className="h-8 text-sm">
                              <option value="EFECTIVO">Efectivo</option>
                              <option value="TARJETA">Tarjeta</option>
                              <option value="TRANSFERENCIA">Transferencia</option>
                            </Select>
                          </div>
                          <Button size="sm" disabled={abonoGuardando} onClick={() => handleAbonar(c.id)}>
                            {abonoGuardando ? "Guardando..." : "Registrar abono"}
                          </Button>
                          <Button size="sm" variant="ghost" onClick={() => setAbonandoId(null)}>Cancelar</Button>
                          {abonoError && <span className="text-xs text-destructive">{abonoError}</span>}
                        </div>
                      </TableCell>
                    </TableRow>
                  )}
                </Fragment>
              );
            })}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

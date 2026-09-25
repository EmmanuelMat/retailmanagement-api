"use client";

import { Fragment, Suspense, useState } from "react";
import Link from "next/link";
import { Plus, Pencil, Trash2, Truck, Search, HandCoins } from "lucide-react";
import {
  Button, ConfirmDialog, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard, formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface Proveedor {
  id: string;
  nombre: string;
  rnc: string | null;
  contacto: string | null;
  telefono: string | null;
  /** Deuda por compras a crédito, derivada en el core (ver
   *  partner_service::SALDO_PROVEEDOR_SQL). Llega como string decimal. */
  saldo_pendiente: string;
}

interface ProveedoresFilters {
  search?: string;
  activo?: string;
}

export default function ProveedoresPage() {
  return (
    <Suspense fallback={null}>
      <ProveedoresPageContent />
    </Suspense>
  );
}

function ProveedoresPageContent() {
  const {
    items: proveedores,
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
  } = useServerTable<Proveedor, ProveedoresFilters>({
    path: "/api/proveedores",
    initialPageSize: 20,
    initialSortBy: "nombre",
    initialSortDir: "asc",
    initialFilters: { activo: "true" },
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));
  const [eliminandoId, setEliminandoId] = useState<string | null>(null);
  const [deleteError, setDeleteError] = useState("");
  // Abono a proveedor: paga la deuda de compras a crédito (2110 Cuentas por
  // Pagar). Mueve dinero, así que pasa por ConfirmDialog como el resto de
  // las acciones irreversibles.
  const [abonando, setAbonando] = useState<Proveedor | null>(null);
  const [abonoMonto, setAbonoMonto] = useState("");
  const [abonoMetodo, setAbonoMetodo] = useState("EFECTIVO");
  const [abonoBusy, setAbonoBusy] = useState(false);
  const [abonoError, setAbonoError] = useState("");

  function abrirAbono(p: Proveedor) {
    setAbonando(p);
    setAbonoMonto(p.saldo_pendiente);
    setAbonoMetodo("EFECTIVO");
    setAbonoError("");
  }

  async function handleAbono() {
    if (!abonando) return;
    setAbonoBusy(true);
    setAbonoError("");
    try {
      await apiFetch(`/api/proveedores/${abonando.id}/abonos`, {
        method: "POST",
        body: JSON.stringify({ monto: abonoMonto, metodo_pago: abonoMetodo }),
      });
      setAbonando(null);
      refresh();
    } catch (e: any) {
      setAbonoError(e.message);
    } finally {
      setAbonoBusy(false);
    }
  }

  async function handleDelete(id: string) {
    setDeleteError("");
    try {
      await apiFetch(`/api/proveedores/${id}`, { method: "DELETE" });
      setEliminandoId(null);
      refresh();
    } catch (e: any) {
      setDeleteError(e.message);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Proveedores</h1>
          <p className="text-sm text-muted-foreground mt-1">Para compras y el reporte 606.</p>
        </div>
        <Link href="/proveedores/nuevo">
          <Button><Plus className="h-4 w-4" />Nuevo proveedor</Button>
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
        error={error}
        isEmpty={proveedores.length === 0}
        emptyIcon={<Truck className="h-6 w-6" />}
        emptyMessage="No hay proveedores todavía."
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
              <TableHead>RNC</TableHead>
              <TableHead>Contacto</TableHead>
              <TableHead>Teléfono</TableHead>
              <TableHead className="text-right">Saldo por pagar</TableHead>
              <TableHead className="w-32 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {proveedores.map((p) => (
              <Fragment key={p.id}>
                <TableRow>
                  <TableCell className="font-medium">{p.nombre}</TableCell>
                  <TableCell className="font-mono text-xs text-muted-foreground">{p.rnc || "—"}</TableCell>
                  <TableCell className="text-muted-foreground">{p.contacto || "—"}</TableCell>
                  <TableCell className="text-muted-foreground">{p.telefono || "—"}</TableCell>
                  <TableCell
                    className={`text-right tabular-nums ${Number(p.saldo_pendiente) > 0 ? "font-semibold" : "text-muted-foreground"}`}
                    data-testid="proveedor-saldo"
                  >
                    {formatDOP(p.saldo_pendiente)}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      {Number(p.saldo_pendiente) > 0 && (
                        <Button
                          size="icon"
                          variant="ghost"
                          aria-label={`Abonar a ${p.nombre}`}
                          title="Registrar abono"
                          data-testid="proveedor-abonar"
                          onClick={() => abrirAbono(p)}
                        >
                          <HandCoins className="h-4 w-4" />
                        </Button>
                      )}
                      <Link href={`/proveedores/${p.id}` as any}>
                        <Button size="icon" variant="ghost"><Pencil className="h-4 w-4" /></Button>
                      </Link>
                      <Button size="icon" variant="ghost" onClick={() => setEliminandoId(p.id)}><Trash2 className="h-4 w-4" /></Button>
                    </div>
                  </TableCell>
                </TableRow>
                {eliminandoId === p.id && (
                  <TableRow>
                    <TableCell colSpan={6} className="bg-muted/30">
                      <div className="flex flex-wrap items-center gap-2 py-1">
                        <span className="text-sm">¿Desactivar a {p.nombre}?</span>
                        <Button size="sm" variant="destructive" onClick={() => handleDelete(p.id)}>Desactivar</Button>
                        <Button size="sm" variant="ghost" onClick={() => setEliminandoId(null)}>Cancelar</Button>
                        {deleteError && <span className="text-xs text-destructive">{deleteError}</span>}
                      </div>
                    </TableCell>
                  </TableRow>
                )}
              </Fragment>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>

      <ConfirmDialog
        open={abonando !== null}
        onClose={() => setAbonando(null)}
        onConfirm={handleAbono}
        busy={abonoBusy}
        confirmLabel="Registrar abono"
        title={`Abonar a ${abonando?.nombre ?? ""}`}
        description={
          <div className="space-y-3">
            <p>
              Saldo por pagar: <strong className="tabular-nums">{formatDOP(abonando?.saldo_pendiente ?? "0")}</strong>. El abono baja la deuda y,
              si es en efectivo, sale de la caja.
            </p>
            <div className="space-y-1.5">
              <label htmlFor="abono-monto" className="text-xs text-muted-foreground">Monto</label>
              <Input
                id="abono-monto"
                type="number"
                min="0"
                step="0.01"
                value={abonoMonto}
                onChange={(e) => setAbonoMonto(e.target.value)}
                data-testid="proveedor-abono-monto"
              />
            </div>
            <div className="space-y-1.5">
              <label htmlFor="abono-metodo" className="text-xs text-muted-foreground">Método de pago</label>
              <Select id="abono-metodo" value={abonoMetodo} onChange={(e) => setAbonoMetodo(e.target.value)}>
                <option value="EFECTIVO">Efectivo (sale de la caja)</option>
                <option value="TRANSFERENCIA">Transferencia</option>
                <option value="CHEQUE">Cheque</option>
              </Select>
            </div>
            {abonoError && <p className="text-xs text-destructive">{abonoError}</p>}
          </div>
        }
      />
    </div>
  );
}

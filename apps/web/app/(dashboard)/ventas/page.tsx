"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Receipt, Search } from "lucide-react";
import {
  Badge,
  Card,
  CardContent,
  Input,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  SortableTableHead,
  TableHeader,
  TableRow,
  Pagination,
  formatDOP,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useDebouncedValue } from "@/lib/use-debounced-value";

interface Venta {
  id: string;
  cliente_nombre: string | null;
  total: string;
  metodo_pago: string;
  estado: string;
  e_ncf: string | null;
  estado_dgii: string | null;
  created_at: string;
}

interface Cliente {
  id: string;
  nombre: string;
}

interface VentasFilters {
  clienteId?: string;
  fechaDesde?: string;
  fechaHasta?: string;
  search?: string;
}

const DGII_VARIANT: Record<string, "default" | "success" | "warning" | "destructive"> = {
  Aceptado: "success",
  ACEPTADO: "success",
  FIRMADO_NO_ENVIADO: "warning",
  Rechazado: "destructive",
  RECHAZADO: "destructive",
};

export default function VentasPage() {
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [searchInput, setSearchInput] = useState("");
  const debouncedSearch = useDebouncedValue(searchInput, 250);

  useEffect(() => {
    apiFetch<{ clientes: Cliente[] }>("/api/clientes").then((d) => setClientes(d.clientes)).catch(() => {});
  }, []);

  const {
    items: ventas,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
  } = useServerTable<Venta, VentasFilters>({
    path: "/api/ventas",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  useEffect(() => {
    setFilters({ search: debouncedSearch });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [debouncedSearch]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Ventas</h1>
        <p className="text-sm text-muted-foreground mt-1">Historial de ventas, e-NCF y estado DGII.</p>
      </div>

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por ID de venta..." className="pl-9" />
        </div>
        <Select
          value={state.filters.clienteId || ""}
          onChange={(e) => setFilters({ clienteId: e.target.value || undefined })}
          className="max-w-[220px]"
        >
          <option value="">Todos los clientes</option>
          {clientes.map((c) => (
            <option key={c.id} value={c.id}>{c.nombre}</option>
          ))}
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

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : ventas.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <Receipt className="h-6 w-6" />
              Aún no hay ventas. Regístralas desde el Punto de Venta.
            </div>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <SortableTableHead column="created_at" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Fecha</SortableTableHead>
                    <TableHead>Cliente</TableHead>
                    <TableHead>e-NCF</TableHead>
                    <SortableTableHead column="estado" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Estado DGII</SortableTableHead>
                    <TableHead>Pago</TableHead>
                    <SortableTableHead column="total" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Total</SortableTableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {ventas.map((v) => (
                    <TableRow key={v.id} className="cursor-pointer">
                      <TableCell className="text-xs text-muted-foreground">
                        <Link href={`/ventas/${v.id}` as any} className="hover:text-primary">{new Date(v.created_at).toLocaleString("es-DO")}</Link>
                      </TableCell>
                      <TableCell>{v.cliente_nombre || "Consumidor final"}</TableCell>
                      <TableCell className="font-mono text-xs">{v.e_ncf || "—"}</TableCell>
                      <TableCell>{v.estado_dgii ? <Badge variant={DGII_VARIANT[v.estado_dgii] || "default"}>{v.estado_dgii}</Badge> : <Badge variant="secondary">Sin emitir</Badge>}</TableCell>
                      <TableCell className="text-muted-foreground">{v.metodo_pago}</TableCell>
                      <TableCell className="text-right font-mono tabular-nums font-medium">{formatDOP(v.total)}</TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>
              <div className="px-3">
                <Pagination
                  page={state.page}
                  totalPages={totalPages}
                  total={total}
                  pageSize={state.pageSize}
                  onPageChange={setPage}
                  onPageSizeChange={setPageSize}
                />
              </div>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

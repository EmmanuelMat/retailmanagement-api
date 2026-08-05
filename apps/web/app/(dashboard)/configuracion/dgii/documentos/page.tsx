"use client";

import { Suspense, useEffect, useState } from "react";
import Link from "next/link";
import { RefreshCw, Search } from "lucide-react";
import {
  Badge, Button, Input, Select,
  Table, TableBody, TableCell, TableHead, SortableTableHead, TableHeader, TableRow,
  Pagination, ScrollableTableCard,
} from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";
import { useSearchFilterSync } from "@/lib/use-search-filter-sync";

interface EcfDocumento {
  id: string;
  referencia_tipo: string;
  referencia_id: string;
  tipo_ecf: number;
  e_ncf: string;
  estado_dgii: string;
  track_id: string | null;
  codigo_seguridad: string | null;
  mensaje_dgii: string | null;
  created_at: string;
}

interface DocumentosFilters {
  estadoDgii?: string;
  referenciaTipo?: string;
  search?: string;
  fechaDesde?: string;
  fechaHasta?: string;
}

function estadoVariant(estado: string): "success" | "destructive" | "warning" | "secondary" {
  if (estado === "ACEPTADO") return "success";
  if (estado === "RECHAZADO") return "destructive";
  if (estado === "CONTINGENCIA_PENDIENTE") return "warning";
  return "secondary";
}

export default function DocumentosEcfPage() {
  return (
    <Suspense fallback={null}>
      <DocumentosEcfPageContent />
    </Suspense>
  );
}

function DocumentosEcfPageContent() {
  const [pendientes, setPendientes] = useState(0);
  const [reintentando, setReintentando] = useState(false);
  const [reintentoMsg, setReintentoMsg] = useState("");

  const {
    items: docs,
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
  } = useServerTable<EcfDocumento, DocumentosFilters>({
    path: "/api/ecf/documentos",
    initialPageSize: 20,
    initialSortBy: "created_at",
    initialSortDir: "desc",
  });

  const [searchInput, setSearchInput] = useSearchFilterSync(state.filters.search || "", (search) => setFilters({ search }));

  async function loadPendientesCount() {
    try {
      const r = await apiFetch<{ total: number }>("/api/ecf/documentos?estadoDgii=CONTINGENCIA_PENDIENTE&pageSize=1");
      setPendientes(r.total);
    } catch {}
  }

  useEffect(() => {
    loadPendientesCount();
  }, []);

  async function handleReintentar() {
    setReintentando(true);
    setReintentoMsg("");
    try {
      const result = await apiFetch<{ reenviados: number; siguen_pendientes: number }>("/api/ecf/pendientes/reintentar", { method: "POST" });
      setReintentoMsg(`${result.reenviados} reenviados, ${result.siguen_pendientes} siguen pendientes.`);
      refresh();
      loadPendientesCount();
    } catch (e: any) {
      setReintentoMsg(e.message);
    } finally {
      setReintentando(false);
    }
  }

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Documentos e-CF</h1>
          <p className="text-sm text-muted-foreground mt-1">
            Historial de e-CF firmados (Ventas y Notas de Crédito), con el XML retenido para auditoría DGII.
          </p>
        </div>
        {pendientes > 0 && (
          <Button onClick={handleReintentar} disabled={reintentando}>
            <RefreshCw className="h-4 w-4" />{reintentando ? "Reintentando..." : `Reintentar ${pendientes} pendiente(s)`}
          </Button>
        )}
      </div>

      {reintentoMsg && <p className="text-sm text-muted-foreground">{reintentoMsg}</p>}

      <div className="flex flex-wrap gap-3">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input value={searchInput} onChange={(e) => setSearchInput(e.target.value)} placeholder="Buscar por e-NCF..." className="pl-9" />
        </div>
        <Select
          value={state.filters.estadoDgii || ""}
          onChange={(e) => setFilters({ estadoDgii: e.target.value || undefined })}
          className="max-w-[200px]"
        >
          <option value="">Todos los estados</option>
          <option value="ACEPTADO">Aceptado</option>
          <option value="RECHAZADO">Rechazado</option>
          <option value="CONTINGENCIA_PENDIENTE">Pendiente de envío</option>
        </Select>
        <Select
          value={state.filters.referenciaTipo || ""}
          onChange={(e) => setFilters({ referenciaTipo: e.target.value || undefined })}
          className="max-w-[180px]"
        >
          <option value="">Todas las referencias</option>
          <option value="VENTA">Venta</option>
          <option value="NOTA_CREDITO">Nota de Crédito</option>
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
        isEmpty={docs.length === 0}
        emptyMessage="No hay documentos e-CF emitidos todavía."
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
              <SortableTableHead column="tipo_ecf" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Tipo</SortableTableHead>
              <TableHead>e-NCF</TableHead>
              <TableHead>Referencia</TableHead>
              <TableHead>Estado DGII</TableHead>
              <TableHead>Mensaje</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {docs.map((d) => (
              <TableRow key={d.id}>
                <TableCell className="text-muted-foreground">{new Date(d.created_at).toLocaleString("es-DO")}</TableCell>
                <TableCell>{d.tipo_ecf}</TableCell>
                <TableCell className="font-mono text-xs">{d.e_ncf}</TableCell>
                <TableCell>
                  {d.referencia_tipo === "VENTA" ? (
                    <Link href={`/ventas/${d.referencia_id}` as any} className="text-primary hover:underline">Venta</Link>
                  ) : (
                    "Nota de Crédito"
                  )}
                </TableCell>
                <TableCell>
                  <Badge variant={estadoVariant(d.estado_dgii)}>
                    {d.estado_dgii === "CONTINGENCIA_PENDIENTE" ? "pendiente de envío" : d.estado_dgii}
                  </Badge>
                </TableCell>
                <TableCell className="text-xs text-muted-foreground max-w-xs truncate" title={d.mensaje_dgii || ""}>{d.mensaje_dgii || "—"}</TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

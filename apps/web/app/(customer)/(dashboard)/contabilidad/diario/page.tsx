"use client";

import { Suspense, useState } from "react";
import { BookOpenCheck, Undo2 } from "lucide-react";
import { Badge, Button, Input, Select, ScrollableTableCard, Pagination, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";

interface Linea {
  cuenta: string;
  debe: string;
  haber: string;
}

interface AsientoConLineas {
  cabecera: {
    id: string;
    fecha: string;
    descripcion: string;
    origen: "AUTOMATICO" | "MANUAL" | "REVERSION";
    referencia_tipo: string | null;
    referencia_id: string | null;
    reversa_de: string | null;
  };
  lineas: Linea[];
}

interface DiarioFilters {
  fechaDesde?: string;
  fechaHasta?: string;
  origen?: string;
}

const ORIGEN_BADGE: Record<string, "default" | "secondary" | "warning"> = {
  AUTOMATICO: "secondary",
  MANUAL: "default",
  REVERSION: "warning",
};

export default function LibroDiarioPage() {
  return (
    <Suspense fallback={null}>
      <LibroDiarioPageContent />
    </Suspense>
  );
}

function LibroDiarioPageContent() {
  const [abriendoMotivo, setAbriendoMotivo] = useState<string | null>(null);
  const [motivo, setMotivo] = useState("");
  const [reversando, setReversando] = useState<string | null>(null);
  const [error, setError] = useState("");

  const {
    items: asientos,
    total,
    totalPages,
    loading,
    error: loadError,
    state,
    setPage,
    setPageSize,
    setFilters,
    refresh,
  } = useServerTable<AsientoConLineas, DiarioFilters>({
    path: "/api/contabilidad/libro-diario",
    initialPageSize: 20,
    initialSortBy: "fecha",
    initialSortDir: "desc",
  });

  async function reversar(id: string) {
    setReversando(id);
    setError("");
    try {
      await apiFetch(`/api/contabilidad/asientos/${id}/reversar`, { method: "POST", body: JSON.stringify({ motivo: motivo.trim() || undefined }) });
      setAbriendoMotivo(null);
      setMotivo("");
      refresh();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setReversando(null);
    }
  }

  const yaReversados = new Set(asientos.filter((a) => a.cabecera.reversa_de).map((a) => a.cabecera.reversa_de));

  return (
    <div className="space-y-4">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Libro diario</h1>
        <p className="text-sm text-muted-foreground mt-1">Cada transacción, en orden cronológico, con sus líneas de debe y haber agrupadas.</p>
      </div>

      <div className="flex flex-wrap gap-3">
        <Input type="date" value={state.filters.fechaDesde || ""} onChange={(e) => setFilters({ fechaDesde: e.target.value || undefined })} className="max-w-[160px]" />
        <Input type="date" value={state.filters.fechaHasta || ""} onChange={(e) => setFilters({ fechaHasta: e.target.value || undefined })} className="max-w-[160px]" />
        <Select value={state.filters.origen || ""} onChange={(e) => setFilters({ origen: e.target.value || undefined })} className="max-w-[180px]">
          <option value="">Todos los orígenes</option>
          <option value="AUTOMATICO">Automático</option>
          <option value="MANUAL">Manual</option>
          <option value="REVERSION">Reversión</option>
        </Select>
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <ScrollableTableCard
        loading={loading}
        error={loadError}
        isEmpty={asientos.length === 0}
        emptyIcon={<BookOpenCheck className="h-6 w-6" />}
        emptyMessage="Sin asientos todavía."
        maxHeight="calc(100vh - 380px)"
        pagination={
          <Pagination page={state.page} totalPages={totalPages} total={total} pageSize={state.pageSize} onPageChange={setPage} onPageSizeChange={setPageSize} />
        }
      >
        <div className="divide-y divide-border">
          {asientos.map((a) => {
            const totalDebe = a.lineas.reduce((s, l) => s + Number(l.debe), 0);
            const puedeReversar = a.cabecera.origen !== "REVERSION" && !yaReversados.has(a.cabecera.id);
            return (
              <div key={a.cabecera.id} className="py-3 px-1">
                <div className="flex items-start justify-between gap-3">
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs text-muted-foreground">{a.cabecera.fecha}</span>
                      <Badge variant={ORIGEN_BADGE[a.cabecera.origen] || "default"}>{a.cabecera.origen}</Badge>
                      {a.cabecera.referencia_tipo && <Badge variant="outline">{a.cabecera.referencia_tipo}</Badge>}
                    </div>
                    <p className="text-sm font-medium mt-1">{a.cabecera.descripcion}</p>
                  </div>
                  <div className="flex items-center gap-2 shrink-0">
                    <span className="text-sm font-semibold tabular-nums">{formatDOP(totalDebe)}</span>
                    {puedeReversar && abriendoMotivo !== a.cabecera.id && (
                      <Button size="sm" variant="ghost" onClick={() => { setAbriendoMotivo(a.cabecera.id); setMotivo(""); }}>
                        <Undo2 className="h-3.5 w-3.5" /> Reversar
                      </Button>
                    )}
                  </div>
                </div>
                <table className="w-full mt-2 text-sm">
                  <tbody>
                    {a.lineas.map((l, i) => (
                      <tr key={i} className="text-muted-foreground">
                        <td className="py-0.5 pl-4">{l.cuenta}</td>
                        <td className="py-0.5 text-right tabular-nums w-28">{Number(l.debe) > 0 ? formatDOP(l.debe) : ""}</td>
                        <td className="py-0.5 text-right tabular-nums w-28">{Number(l.haber) > 0 ? formatDOP(l.haber) : ""}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
                {abriendoMotivo === a.cabecera.id && (
                  <div className="mt-2 flex items-center gap-2 pl-4">
                    <Input
                      value={motivo}
                      onChange={(e) => setMotivo(e.target.value)}
                      placeholder="Motivo de la reversión (opcional)"
                      className="max-w-sm"
                      autoFocus
                    />
                    <Button size="sm" variant="destructive" onClick={() => reversar(a.cabecera.id)} disabled={reversando === a.cabecera.id}>
                      {reversando === a.cabecera.id ? "Reversando..." : "Confirmar"}
                    </Button>
                    <Button size="sm" variant="ghost" onClick={() => setAbriendoMotivo(null)} disabled={reversando === a.cabecera.id}>
                      Cancelar
                    </Button>
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </ScrollableTableCard>
    </div>
  );
}

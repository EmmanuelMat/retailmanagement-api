"use client";

import { useEffect, useState } from "react";
import { History } from "lucide-react";
import { Badge, Card, CardContent, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface CajaSesion {
  id: string;
  monto_inicial: string;
  monto_final: string | null;
  monto_esperado: string | null;
  diferencia: string | null;
  estado: string;
  abierta_at: string;
  cerrada_at: string | null;
}

export default function HistorialCajaPage() {
  const [sesiones, setSesiones] = useState<CajaSesion[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch<{ sesiones: CajaSesion[] }>("/api/caja/sesiones").then((d) => setSesiones(d.sesiones)).finally(() => setLoading(false));
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Historial de caja</h1>
        <p className="text-sm text-muted-foreground mt-1">Arqueos de cada turno abierto y cerrado.</p>
      </div>

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : sesiones.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <History className="h-6 w-6" />
              Sin sesiones de caja todavía.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Apertura</TableHead>
                  <TableHead>Cierre</TableHead>
                  <TableHead className="text-right">Inicial</TableHead>
                  <TableHead className="text-right">Contado</TableHead>
                  <TableHead className="text-right">Diferencia</TableHead>
                  <TableHead>Estado</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {sesiones.map((s) => (
                  <TableRow key={s.id}>
                    <TableCell className="text-xs text-muted-foreground">{new Date(s.abierta_at).toLocaleString("es-DO")}</TableCell>
                    <TableCell className="text-xs text-muted-foreground">{s.cerrada_at ? new Date(s.cerrada_at).toLocaleString("es-DO") : "—"}</TableCell>
                    <TableCell className="text-right tabular-nums">{formatDOP(s.monto_inicial)}</TableCell>
                    <TableCell className="text-right tabular-nums">{s.monto_final ? formatDOP(s.monto_final) : "—"}</TableCell>
                    <TableCell className={`text-right tabular-nums ${s.diferencia && Number(s.diferencia) !== 0 ? "text-destructive" : ""}`}>
                      {s.diferencia ? formatDOP(s.diferencia) : "—"}
                    </TableCell>
                    <TableCell><Badge variant={s.estado === "ABIERTA" ? "default" : "secondary"}>{s.estado}</Badge></TableCell>
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

"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { RefreshCw } from "lucide-react";
import { Badge, Button, Card, CardContent, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@repo/ui";
import { apiFetch } from "@/lib/api";

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

function estadoVariant(estado: string): "success" | "destructive" | "warning" | "secondary" {
  if (estado === "ACEPTADO") return "success";
  if (estado === "RECHAZADO") return "destructive";
  if (estado === "CONTINGENCIA_PENDIENTE") return "warning";
  return "secondary";
}

export default function DocumentosEcfPage() {
  const [docs, setDocs] = useState<EcfDocumento[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [reintentando, setReintentando] = useState(false);
  const [reintentoMsg, setReintentoMsg] = useState("");

  async function load() {
    setLoading(true);
    try {
      const data = await apiFetch<{ documentos: EcfDocumento[] }>("/api/ecf/documentos");
      setDocs(data.documentos);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  const pendientes = docs.filter((d) => d.estado_dgii === "CONTINGENCIA_PENDIENTE").length;

  async function handleReintentar() {
    setReintentando(true);
    setReintentoMsg("");
    try {
      const result = await apiFetch<{ reenviados: number; siguen_pendientes: number }>("/api/ecf/pendientes/reintentar", { method: "POST" });
      setReintentoMsg(`${result.reenviados} reenviados, ${result.siguen_pendientes} siguen pendientes.`);
      await load();
    } catch (e: any) {
      setReintentoMsg(e.message);
    } finally {
      setReintentando(false);
    }
  }

  return (
    <div className="space-y-6">
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
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : docs.length === 0 ? (
            <p className="p-8 text-center text-sm text-muted-foreground">No hay documentos e-CF emitidos todavía.</p>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Tipo</TableHead>
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
          )}
        </CardContent>
      </Card>
    </div>
  );
}

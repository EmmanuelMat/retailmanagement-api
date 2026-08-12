"use client";

import { useEffect, useState } from "react";
import { Lock, LockOpen } from "lucide-react";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle, Select } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Periodo {
  id: string;
  anio: number;
  mes: number;
  estado: "ABIERTO" | "CERRADO";
  cerrado_at: string | null;
}

const MESES = [
  "Enero", "Febrero", "Marzo", "Abril", "Mayo", "Junio",
  "Julio", "Agosto", "Septiembre", "Octubre", "Noviembre", "Diciembre",
];

export default function PeriodosPage() {
  const [periodos, setPeriodos] = useState<Periodo[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [cerrando, setCerrando] = useState(false);
  const [confirmando, setConfirmando] = useState(false);
  const now = new Date();
  const [anio, setAnio] = useState(String(now.getFullYear()));
  const [mes, setMes] = useState(String(now.getMonth() + 1));

  async function load() {
    setLoading(true);
    try {
      const data = await apiFetch<Periodo[]>("/api/contabilidad/periodos");
      setPeriodos(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function cerrarPeriodo(a: string, m: string) {
    setCerrando(true);
    setError("");
    try {
      await apiFetch(`/api/contabilidad/periodos/${a}/${m}/cerrar`, { method: "POST" });
      setConfirmando(false);
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setCerrando(false);
    }
  }

  const anios = Array.from({ length: 5 }, (_, i) => now.getFullYear() - i);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Períodos contables</h1>
        <p className="text-sm text-muted-foreground mt-1">Cierre mensual para bloquear cambios en meses ya reportados.</p>
      </div>

      <Card>
        <CardHeader><CardTitle className="text-sm">Cerrar un período</CardTitle></CardHeader>
        <CardContent>
          <div className="flex flex-wrap items-end gap-3">
            <div className="space-y-1.5">
              <label className="text-xs text-muted-foreground">Mes</label>
              <Select value={mes} onChange={(e) => setMes(e.target.value)}>
                {MESES.map((nombre, i) => (
                  <option key={i} value={i + 1}>{nombre}</option>
                ))}
              </Select>
            </div>
            <div className="space-y-1.5">
              <label className="text-xs text-muted-foreground">Año</label>
              <Select value={anio} onChange={(e) => setAnio(e.target.value)}>
                {anios.map((a) => (
                  <option key={a} value={a}>{a}</option>
                ))}
              </Select>
            </div>
            {!confirmando ? (
              <Button onClick={() => setConfirmando(true)} variant="destructive">
                <Lock className="h-4 w-4" />
                Cerrar período
              </Button>
            ) : (
              <>
                <Button onClick={() => cerrarPeriodo(anio, mes)} disabled={cerrando} variant="destructive">
                  <Lock className="h-4 w-4" />
                  {cerrando ? "Cerrando..." : `Confirmar cierre de ${MESES[Number(mes) - 1]} ${anio}`}
                </Button>
                <Button onClick={() => setConfirmando(false)} disabled={cerrando} variant="ghost">
                  Cancelar
                </Button>
              </>
            )}
          </div>
          <p className="text-xs text-muted-foreground mt-2">
            Un período cerrado no acepta asientos nuevos ni reversiones con fecha dentro de ese mes. Para corregir algo de un mes cerrado, se
            registra un asiento nuevo hoy — nunca se reabre el mes.
          </p>
        </CardContent>
      </Card>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardHeader><CardTitle className="text-sm">Historial</CardTitle></CardHeader>
        <CardContent>
          {loading ? (
            <p className="text-sm text-muted-foreground">Cargando...</p>
          ) : periodos.length === 0 ? (
            <p className="text-sm text-muted-foreground">Ningún período tiene actividad todavía — se crean automáticamente al registrar el primer asiento de cada mes.</p>
          ) : (
            <div className="space-y-1.5">
              {periodos.map((p) => (
                <div key={p.id} className="flex items-center justify-between text-sm py-2 border-b border-border last:border-0">
                  <span>{MESES[p.mes - 1]} {p.anio}</span>
                  <Badge variant={p.estado === "CERRADO" ? "secondary" : "outline"} className="gap-1">
                    {p.estado === "CERRADO" ? <Lock className="h-3 w-3" /> : <LockOpen className="h-3 w-3" />}
                    {p.estado === "CERRADO" ? "Cerrado" : "Abierto"}
                  </Badge>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

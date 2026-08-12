"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { RefreshCw, BookOpen, BookOpenCheck, ScrollText, Lock, ArrowRight } from "lucide-react";
import { Button, Card, CardContent, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface CuentaResumen {
  cuenta: string;
  debe: string;
  haber: string;
  saldo: string;
}

export default function ContabilidadPage() {
  const [cuentas, setCuentas] = useState<CuentaResumen[]>([]);
  const [loading, setLoading] = useState(true);
  const [syncing, setSyncing] = useState(false);
  const [mensaje, setMensaje] = useState("");
  const [error, setError] = useState("");

  async function load() {
    setLoading(true);
    try {
      const data = await apiFetch<{ items: CuentaResumen[] }>("/api/contabilidad/libro-mayor?pageSize=1000");
      setCuentas(data.items);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleSincronizar() {
    setSyncing(true);
    setError("");
    setMensaje("");
    try {
      const r = await apiFetch<{
        ventas_procesadas: number;
        compras_procesadas: number;
        nomina_procesada: number;
        gastos_procesados: number;
        adelantos_procesados: number;
        abonos_procesados: number;
        notas_credito_procesadas: number;
        ajustes_procesados: number;
        banco_procesados: number;
      }>("/api/contabilidad/sincronizar", { method: "POST" });
      const partes = [
        [r.ventas_procesadas, "ventas"],
        [r.compras_procesadas, "compras"],
        [r.gastos_procesados, "gastos"],
        [r.adelantos_procesados, "adelantos"],
        [r.nomina_procesada, "nóminas"],
        [r.abonos_procesados, "abonos"],
        [r.notas_credito_procesadas, "notas de crédito"],
        [r.ajustes_procesados, "ajustes de inventario"],
        [r.banco_procesados, "movimientos bancarios"],
      ] as const;
      const resumen = partes.filter(([n]) => n > 0).map(([n, label]) => `${n} ${label}`).join(", ");
      setMensaje(resumen ? `${resumen} registrados en el libro mayor.` : "Todo al día - no había nada nuevo que sincronizar.");
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSyncing(false);
    }
  }

  const totalDebe = cuentas.reduce((s, c) => s + Number(c.debe), 0);
  const totalHaber = cuentas.reduce((s, c) => s + Number(c.haber), 0);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Contabilidad</h1>
          <p className="text-sm text-muted-foreground mt-1">Libro mayor por partida doble, generado desde Ventas, Compras y Nómina.</p>
        </div>
        <Button onClick={handleSincronizar} disabled={syncing} variant="secondary">
          <RefreshCw className="h-4 w-4" />
          {syncing ? "Sincronizando..." : "Generar asientos automáticos"}
        </Button>
      </div>

      {mensaje && <div className="rounded-md border border-success/20 bg-success/10 text-success p-3 text-sm">{mensaje}</div>}
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
        <Link href="/contabilidad/diario">
          <Card className="hover:border-primary transition-colors">
            <CardContent className="pt-5 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold flex items-center gap-2"><BookOpenCheck className="h-4 w-4" />Libro diario</p>
                <p className="text-xs text-muted-foreground mt-1">Transacciones cronológicas, agrupadas</p>
              </div>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </CardContent>
          </Card>
        </Link>
        <Link href="/contabilidad/libro-mayor">
          <Card className="hover:border-primary transition-colors">
            <CardContent className="pt-5 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold flex items-center gap-2"><BookOpen className="h-4 w-4" />Libro mayor</p>
                <p className="text-xs text-muted-foreground mt-1">Saldo por cuenta, con detalle</p>
              </div>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </CardContent>
          </Card>
        </Link>
        <Link href="/contabilidad/asientos">
          <Card className="hover:border-primary transition-colors">
            <CardContent className="pt-5 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold flex items-center gap-2"><ScrollText className="h-4 w-4" />Asientos manuales</p>
                <p className="text-xs text-muted-foreground mt-1">Registrar un asiento, buscar líneas sueltas</p>
              </div>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </CardContent>
          </Card>
        </Link>
        <Link href="/contabilidad/periodos">
          <Card className="hover:border-primary transition-colors">
            <CardContent className="pt-5 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold flex items-center gap-2"><Lock className="h-4 w-4" />Períodos contables</p>
                <p className="text-xs text-muted-foreground mt-1">Cerrar meses ya reportados</p>
              </div>
              <ArrowRight className="h-4 w-4 text-muted-foreground" />
            </CardContent>
          </Card>
        </Link>
      </div>

      <Card>
        <CardContent className="pt-5">
          <div className="flex justify-between text-sm mb-3">
            <p className="font-semibold">Balance de comprobación</p>
            <p className={totalDebe === totalHaber ? "text-success" : "text-destructive"}>
              Debe {formatDOP(totalDebe)} · Haber {formatDOP(totalHaber)}
            </p>
          </div>
          {loading ? (
            <p className="text-sm text-muted-foreground">Cargando...</p>
          ) : cuentas.length === 0 ? (
            <p className="text-sm text-muted-foreground">Sin asientos todavía. Usa "Generar asientos automáticos" para partir de tus ventas y compras.</p>
          ) : (
            <div className="space-y-1.5">
              {cuentas.map((c) => (
                <div key={c.cuenta} className="flex justify-between text-sm py-1 border-b border-border last:border-0">
                  <span>{c.cuenta}</span>
                  <span className="tabular-nums font-medium">{formatDOP(c.saldo)}</span>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

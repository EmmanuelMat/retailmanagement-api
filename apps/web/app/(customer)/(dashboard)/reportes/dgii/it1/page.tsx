"use client";

import { useState } from "react";
import { Button, Card, CardContent, Input, Label, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface It1Resumen {
  period: string;
  itbis_trasladado: string;
  itbis_acreditable: string;
  saldo_periodo_anterior: string;
  itbis_a_pagar: string;
  saldo_a_favor_siguiente: string;
}

export default function ReporteIt1Page() {
  const now = new Date();
  const [period, setPeriod] = useState(`${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, "0")}`);
  const [resumen, setResumen] = useState<It1Resumen | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  async function handleGenerar(e: React.FormEvent) {
    e.preventDefault();
    setLoading(true);
    setError("");
    setResumen(null);
    try {
      const data = await apiFetch<It1Resumen>(`/api/reports/it1?period=${period}`);
      setResumen(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  const aPagar = resumen ? Number(resumen.itbis_a_pagar) > 0 : false;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">IT-1</h1>
        <p className="text-sm text-muted-foreground mt-1">Declaración jurada mensual de ITBIS.</p>
      </div>

      <Card className="max-w-md">
        <CardContent className="pt-5">
          <form onSubmit={handleGenerar} className="flex items-end gap-3">
            <div className="flex-1 space-y-1.5">
              <Label htmlFor="period">Período (AAAAMM)</Label>
              <Input id="period" value={period} onChange={(e) => setPeriod(e.target.value)} placeholder="202608" required maxLength={6} />
            </div>
            <Button type="submit" disabled={loading}>{loading ? "Calculando..." : "Calcular"}</Button>
          </form>
          <p className="text-xs text-muted-foreground mt-2">
            ITBIS trasladado menos acreditable, ajustado por el saldo a favor del período anterior. No incluye el
            desglose de Anexo A (ventas por tasa, retenciones) — para eso, usa la Oficina Virtual con estos totales
            como referencia.
          </p>
          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm mt-3">{error}</div>}
        </CardContent>
      </Card>

      {resumen && (
        <Card className="max-w-md">
          <CardContent className="pt-5 space-y-1 text-sm">
            <div className="flex justify-between text-muted-foreground">
              <span>ITBIS trasladado (ventas)</span>
              <span className="tabular-nums">{formatDOP(resumen.itbis_trasladado)}</span>
            </div>
            <div className="flex justify-between text-muted-foreground">
              <span>ITBIS acreditable (compras)</span>
              <span className="tabular-nums">{formatDOP(resumen.itbis_acreditable)}</span>
            </div>
            <div className="flex justify-between text-muted-foreground">
              <span>Saldo a favor del período anterior</span>
              <span className="tabular-nums">{formatDOP(resumen.saldo_periodo_anterior)}</span>
            </div>
            <div className="flex justify-between font-bold text-base pt-2 mt-1 border-t border-border">
              <span>{aPagar ? "ITBIS a pagar" : "Saldo a favor"}</span>
              <span className="tabular-nums">{formatDOP(aPagar ? resumen.itbis_a_pagar : resumen.saldo_a_favor_siguiente)}</span>
            </div>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

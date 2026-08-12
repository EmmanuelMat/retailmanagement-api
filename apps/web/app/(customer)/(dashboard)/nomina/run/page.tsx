"use client";

import { useState } from "react";
import { PlayCircle } from "lucide-react";
import { Button, Card, CardContent, Input, Label, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface NominaDetalle {
  id: string;
  empleado_nombre: string;
  salario_bruto: string;
  tss: string;
  isr: string;
  adelantos_descuento: string;
  neto: string;
}

interface PeriodoResultado {
  id: string;
  periodo: string;
  total_bruto: string;
  total_neto: string;
  detalles: NominaDetalle[];
}

export default function RunNominaPage() {
  const [periodo, setPeriodo] = useState("");
  const [fechaPago, setFechaPago] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [resultado, setResultado] = useState<PeriodoResultado | null>(null);

  async function handleRun(e: React.FormEvent) {
    e.preventDefault();
    if (!periodo.trim()) return;
    setSaving(true);
    setError("");
    try {
      const result = await apiFetch<PeriodoResultado>("/api/nomina/run", {
        method: "POST",
        body: JSON.stringify({ periodo, fecha_pago: fechaPago || undefined }),
      });
      setResultado(result);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Correr nómina</h1>
        <p className="text-sm text-muted-foreground mt-1">Calcula TSS 5.91%, descuenta adelantos aprobados y registra el egreso en caja.</p>
      </div>

      <Card className="max-w-xl">
        <CardContent className="pt-5">
          <form onSubmit={handleRun} className="grid grid-cols-2 gap-4 items-end">
            <div className="space-y-1.5">
              <Label htmlFor="periodo">Período</Label>
              <Input id="periodo" value={periodo} onChange={(e) => setPeriodo(e.target.value)} placeholder="2026-07 quincena 2" required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="fechaPago">Fecha de pago</Label>
              <Input id="fechaPago" type="date" value={fechaPago} onChange={(e) => setFechaPago(e.target.value)} />
            </div>
            <Button type="submit" disabled={saving} className="col-span-2 w-fit"><PlayCircle className="h-4 w-4" />Correr nómina</Button>
          </form>
          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm mt-4">{error}</div>}
        </CardContent>
      </Card>

      {resultado && (
        <Card>
          <CardContent className="p-0">
            <div className="p-5 border-b border-border flex justify-between">
              <p className="font-semibold text-sm">{resultado.periodo}</p>
              <p className="text-sm">Bruto: <strong>{formatDOP(resultado.total_bruto)}</strong> · Neto: <strong>{formatDOP(resultado.total_neto)}</strong></p>
            </div>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Empleado</TableHead>
                  <TableHead className="text-right">Bruto</TableHead>
                  <TableHead className="text-right">TSS</TableHead>
                  <TableHead className="text-right">Adelantos</TableHead>
                  <TableHead className="text-right">Neto</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {resultado.detalles.map((d) => (
                  <TableRow key={d.id}>
                    <TableCell className="font-medium">{d.empleado_nombre}</TableCell>
                    <TableCell className="text-right tabular-nums">{formatDOP(d.salario_bruto)}</TableCell>
                    <TableCell className="text-right tabular-nums text-muted-foreground">-{formatDOP(d.tss)}</TableCell>
                    <TableCell className="text-right tabular-nums text-muted-foreground">-{formatDOP(d.adelantos_descuento)}</TableCell>
                    <TableCell className="text-right tabular-nums font-medium">{formatDOP(d.neto)}</TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      )}
    </div>
  );
}

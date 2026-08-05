"use client";

import { useEffect, useState } from "react";
import { Plus, Trash2, ScrollText } from "lucide-react";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Asiento {
  id: string;
  fecha: string;
  cuenta: string;
  descripcion: string;
  debe: string;
  haber: string;
  referencia_tipo: string | null;
}

interface Linea {
  cuenta: string;
  debe: string;
  haber: string;
}

export default function AsientosPage() {
  const [asientos, setAsientos] = useState<Asiento[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);
  const [descripcion, setDescripcion] = useState("");
  const [lineas, setLineas] = useState<Linea[]>([{ cuenta: "", debe: "", haber: "" }, { cuenta: "", debe: "", haber: "" }]);

  async function load() {
    setLoading(true);
    try {
      const data = await apiFetch<{ asientos: Asiento[] }>("/api/contabilidad/asientos");
      setAsientos(data.asientos);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  function updateLinea(i: number, patch: Partial<Linea>) {
    setLineas((ls) => ls.map((l, idx) => (idx === i ? { ...l, ...patch } : l)));
  }

  function addLinea() {
    setLineas((ls) => [...ls, { cuenta: "", debe: "", haber: "" }]);
  }

  function removeLinea(i: number) {
    setLineas((ls) => ls.filter((_, idx) => idx !== i));
  }

  const totalDebe = lineas.reduce((s, l) => s + (Number(l.debe) || 0), 0);
  const totalHaber = lineas.reduce((s, l) => s + (Number(l.haber) || 0), 0);
  const cuadra = totalDebe === totalHaber && totalDebe > 0;

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!descripcion.trim() || !cuadra) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/contabilidad/asientos", {
        method: "POST",
        body: JSON.stringify({
          descripcion,
          lineas: lineas.filter((l) => l.cuenta).map((l) => ({ cuenta: l.cuenta, debe: l.debe || "0", haber: l.haber || "0" })),
        }),
      });
      setDescripcion("");
      setLineas([{ cuenta: "", debe: "", haber: "" }, { cuenta: "", debe: "", haber: "" }]);
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Libro diario</h1>
        <p className="text-sm text-muted-foreground mt-1">Asientos automáticos (Ventas/Compras/Nómina) y manuales.</p>
      </div>

      <Card>
        <CardHeader><CardTitle className="text-sm">Nuevo asiento manual</CardTitle></CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="space-y-3">
            <div className="space-y-1.5">
              <Label htmlFor="descripcion">Descripción</Label>
              <Input id="descripcion" value={descripcion} onChange={(e) => setDescripcion(e.target.value)} placeholder="Depreciación de equipos" required />
            </div>
            {lineas.map((l, i) => (
              <div key={i} className="grid grid-cols-[1fr_120px_120px_32px] gap-2 items-end">
                <Input placeholder="Cuenta (ej. 5200 Alquiler)" value={l.cuenta} onChange={(e) => updateLinea(i, { cuenta: e.target.value })} />
                <Input type="number" step="0.01" placeholder="Debe" value={l.debe} onChange={(e) => updateLinea(i, { debe: e.target.value, haber: "" })} />
                <Input type="number" step="0.01" placeholder="Haber" value={l.haber} onChange={(e) => updateLinea(i, { haber: e.target.value, debe: "" })} />
                <Button type="button" size="icon" variant="ghost" onClick={() => removeLinea(i)} disabled={lineas.length === 2}><Trash2 className="h-4 w-4" /></Button>
              </div>
            ))}
            <div className="flex items-center justify-between">
              <Button type="button" variant="secondary" size="sm" onClick={addLinea}><Plus className="h-4 w-4" />Agregar línea</Button>
              <p className={`text-sm ${cuadra ? "text-success" : "text-muted-foreground"}`}>
                Debe {formatDOP(totalDebe)} · Haber {formatDOP(totalHaber)} {cuadra && "✓ Cuadra"}
              </p>
            </div>
            {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}
            <Button type="submit" disabled={saving || !cuadra}>{saving ? "Guardando..." : "Registrar asiento"}</Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : asientos.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <ScrollText className="h-6 w-6" />
              Sin asientos todavía.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Cuenta</TableHead>
                  <TableHead>Descripción</TableHead>
                  <TableHead>Origen</TableHead>
                  <TableHead className="text-right">Debe</TableHead>
                  <TableHead className="text-right">Haber</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {asientos.map((a) => (
                  <TableRow key={a.id}>
                    <TableCell className="text-xs text-muted-foreground">{a.fecha}</TableCell>
                    <TableCell className="font-medium">{a.cuenta}</TableCell>
                    <TableCell className="text-muted-foreground">{a.descripcion}</TableCell>
                    <TableCell>{a.referencia_tipo && <Badge variant="secondary">{a.referencia_tipo}</Badge>}</TableCell>
                    <TableCell className="text-right tabular-nums">{Number(a.debe) > 0 ? formatDOP(a.debe) : "—"}</TableCell>
                    <TableCell className="text-right tabular-nums">{Number(a.haber) > 0 ? formatDOP(a.haber) : "—"}</TableCell>
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

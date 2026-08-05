"use client";

import { useEffect, useState } from "react";
import { Plus, Receipt } from "lucide-react";
import { Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Select, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Gasto {
  id: string;
  concepto: string;
  categoria: string;
  monto: string;
  created_at: string;
}

export default function GastosPage() {
  const [gastos, setGastos] = useState<Gasto[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [concepto, setConcepto] = useState("");
  const [categoria, setCategoria] = useState("OTROS");
  const [monto, setMonto] = useState("");

  useEffect(() => {
    // Si venimos del Asistente con una acción confirmada, precarga el
    // formulario de arriba - la creación real sigue pasando por el mismo
    // handleCreate/POST de siempre, la IA nunca crea nada directamente.
    const raw = sessionStorage.getItem("ai_prefill_gasto");
    if (!raw) return;
    sessionStorage.removeItem("ai_prefill_gasto");
    try {
      const parsed = JSON.parse(raw);
      if (parsed.concepto) setConcepto(String(parsed.concepto));
      if (parsed.categoria) setCategoria(String(parsed.categoria));
      if (parsed.monto) setMonto(String(parsed.monto));
    } catch {}
  }, []);

  async function load() {
    setLoading(true);
    try {
      const data = await apiFetch<{ gastos: Gasto[] }>("/api/gastos");
      setGastos(data.gastos);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleCreate(e: React.FormEvent) {
    e.preventDefault();
    if (!concepto.trim() || !monto) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/gastos", { method: "POST", body: JSON.stringify({ concepto, categoria, monto }) });
      setConcepto("");
      setMonto("");
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
        <h1 className="text-2xl font-bold font-serif tracking-tight">Gastos</h1>
        <p className="text-sm text-muted-foreground mt-1">Gastos operativos que no pasan por inventario (alquiler, servicios, transporte).</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-sm">Registrar gasto</CardTitle>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="grid grid-cols-1 sm:grid-cols-4 gap-3 items-end">
            <div className="sm:col-span-2 space-y-1.5">
              <Label htmlFor="concepto">Concepto</Label>
              <Input id="concepto" value={concepto} onChange={(e) => setConcepto(e.target.value)} placeholder="Factura de luz" required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="categoria">Categoría</Label>
              <Select id="categoria" value={categoria} onChange={(e) => setCategoria(e.target.value)}>
                <option value="ALQUILER">Alquiler</option>
                <option value="SERVICIOS">Servicios</option>
                <option value="TRANSPORTE">Transporte</option>
                <option value="OTROS">Otros</option>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="monto">Monto</Label>
              <Input id="monto" type="number" step="0.01" value={monto} onChange={(e) => setMonto(e.target.value)} placeholder="0.00" required />
            </div>
            <Button type="submit" disabled={saving} className="sm:col-span-4 sm:w-fit">
              <Plus className="h-4 w-4" />
              Registrar
            </Button>
          </form>
        </CardContent>
      </Card>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : gastos.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <Receipt className="h-6 w-6" />
              No hay gastos registrados.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Concepto</TableHead>
                  <TableHead>Categoría</TableHead>
                  <TableHead className="text-right">Monto</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {gastos.map((g) => (
                  <TableRow key={g.id}>
                    <TableCell className="text-xs text-muted-foreground">{new Date(g.created_at).toLocaleDateString("es-DO")}</TableCell>
                    <TableCell className="font-medium">{g.concepto}</TableCell>
                    <TableCell className="text-muted-foreground">{g.categoria}</TableCell>
                    <TableCell className="text-right font-mono tabular-nums">{formatDOP(g.monto)}</TableCell>
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

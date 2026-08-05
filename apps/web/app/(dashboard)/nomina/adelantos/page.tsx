"use client";

import { useEffect, useState } from "react";
import { Plus, Check, X, HandCoins } from "lucide-react";
import { Badge, Button, Card, CardContent, CardHeader, CardTitle, Label, Select, Input, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Empleado {
  id: string;
  nombre: string;
  disponible_adelanto: string;
}

interface Adelanto {
  id: string;
  empleado_id: string;
  empleado_nombre: string;
  monto: string;
  motivo: string | null;
  estado: "PENDIENTE" | "APROBADO" | "RECHAZADO" | "DESCONTADO";
  created_at: string;
}

const ESTADO_VARIANT: Record<string, "default" | "secondary" | "success" | "destructive"> = {
  PENDIENTE: "default",
  APROBADO: "success",
  RECHAZADO: "destructive",
  DESCONTADO: "secondary",
};

export default function AdelantosPage() {
  const [empleados, setEmpleados] = useState<Empleado[]>([]);
  const [adelantos, setAdelantos] = useState<Adelanto[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [empleadoId, setEmpleadoId] = useState("");
  const [monto, setMonto] = useState("");
  const [motivo, setMotivo] = useState("");
  const [saving, setSaving] = useState(false);

  async function load() {
    setLoading(true);
    try {
      const [e, a] = await Promise.all([
        apiFetch<{ empleados: Empleado[] }>("/api/empleados"),
        apiFetch<{ adelantos: Adelanto[] }>("/api/nomina/adelantos"),
      ]);
      setEmpleados(e.empleados);
      setAdelantos(a.adelantos);
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
    if (!empleadoId || !monto) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/nomina/adelantos", {
        method: "POST",
        body: JSON.stringify({ empleado_id: empleadoId, monto, motivo: motivo || undefined }),
      });
      setMonto("");
      setMotivo("");
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleAprobar(id: string) {
    try {
      await apiFetch(`/api/nomina/adelantos/${id}/aprobar`, { method: "POST" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  async function handleRechazar(id: string) {
    try {
      await apiFetch(`/api/nomina/adelantos/${id}/rechazar`, { method: "POST" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  const selectedDisponible = empleados.find((e) => e.id === empleadoId)?.disponible_adelanto;

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Adelantos de nómina</h1>
        <p className="text-sm text-muted-foreground mt-1">Regla del 50% del sueldo mensual, sin intereses.</p>
      </div>

      <Card>
        <CardHeader><CardTitle className="text-sm">Solicitar adelanto</CardTitle></CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="grid grid-cols-1 sm:grid-cols-4 gap-3 items-end">
            <div className="sm:col-span-2 space-y-1.5">
              <Label htmlFor="empleado">Empleado</Label>
              <Select id="empleado" value={empleadoId} onChange={(e) => setEmpleadoId(e.target.value)} required>
                <option value="">Selecciona…</option>
                {empleados.map((e) => <option key={e.id} value={e.id}>{e.nombre}</option>)}
              </Select>
              {selectedDisponible && <p className="text-xs text-muted-foreground">Disponible: {formatDOP(selectedDisponible)}</p>}
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="monto">Monto</Label>
              <Input id="monto" type="number" step="0.01" value={monto} onChange={(e) => setMonto(e.target.value)} placeholder="0.00" required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="motivo">Motivo</Label>
              <Input id="motivo" value={motivo} onChange={(e) => setMotivo(e.target.value)} placeholder="Opcional" />
            </div>
            <Button type="submit" disabled={saving} className="sm:col-span-4 sm:w-fit"><Plus className="h-4 w-4" />Solicitar</Button>
          </form>
        </CardContent>
      </Card>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : adelantos.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <HandCoins className="h-6 w-6" />
              No hay adelantos registrados.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Empleado</TableHead>
                  <TableHead>Motivo</TableHead>
                  <TableHead className="text-right">Monto</TableHead>
                  <TableHead>Estado</TableHead>
                  <TableHead className="w-20 text-right">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {adelantos.map((a) => (
                  <TableRow key={a.id}>
                    <TableCell className="text-xs text-muted-foreground">{new Date(a.created_at).toLocaleDateString("es-DO")}</TableCell>
                    <TableCell className="font-medium">{a.empleado_nombre}</TableCell>
                    <TableCell className="text-muted-foreground">{a.motivo || "—"}</TableCell>
                    <TableCell className="text-right tabular-nums">{formatDOP(a.monto)}</TableCell>
                    <TableCell><Badge variant={ESTADO_VARIANT[a.estado]}>{a.estado}</Badge></TableCell>
                    <TableCell className="text-right">
                      {a.estado === "PENDIENTE" && (
                        <div className="flex justify-end gap-1">
                          <Button size="icon" variant="ghost" onClick={() => handleAprobar(a.id)}><Check className="h-4 w-4" /></Button>
                          <Button size="icon" variant="ghost" onClick={() => handleRechazar(a.id)}><X className="h-4 w-4" /></Button>
                        </div>
                      )}
                    </TableCell>
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

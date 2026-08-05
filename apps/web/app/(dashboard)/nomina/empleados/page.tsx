"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Plus, Pencil, Trash2, Users2 } from "lucide-react";
import { Button, Card, CardContent, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Empleado {
  id: string;
  nombre: string;
  puesto: string | null;
  salario_mensual: string;
  disponible_adelanto: string;
}

export default function EmpleadosPage() {
  const [empleados, setEmpleados] = useState<Empleado[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  async function load() {
    setLoading(true);
    try {
      const data = await apiFetch<{ empleados: Empleado[] }>("/api/empleados");
      setEmpleados(data.empleados);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    load();
  }, []);

  async function handleDelete(id: string) {
    if (!confirm("¿Desactivar este empleado?")) return;
    try {
      await apiFetch(`/api/empleados/${id}`, { method: "DELETE" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Empleados</h1>
          <p className="text-sm text-muted-foreground mt-1">Equipo y disponible para adelanto (50% del sueldo mensual).</p>
        </div>
        <Link href="/nomina/empleados/nuevo">
          <Button><Plus className="h-4 w-4" />Nuevo empleado</Button>
        </Link>
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : empleados.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <Users2 className="h-6 w-6" />
              No hay empleados registrados.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Nombre</TableHead>
                  <TableHead>Puesto</TableHead>
                  <TableHead className="text-right">Salario mensual</TableHead>
                  <TableHead className="text-right">Disponible adelanto</TableHead>
                  <TableHead className="w-24 text-right">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {empleados.map((e) => (
                  <TableRow key={e.id}>
                    <TableCell className="font-medium">{e.nombre}</TableCell>
                    <TableCell className="text-muted-foreground">{e.puesto || "—"}</TableCell>
                    <TableCell className="text-right tabular-nums">{formatDOP(e.salario_mensual)}</TableCell>
                    <TableCell className="text-right tabular-nums text-success">{formatDOP(e.disponible_adelanto)}</TableCell>
                    <TableCell className="text-right">
                      <div className="flex justify-end gap-1">
                        <Link href={`/nomina/empleados/${e.id}` as any}>
                          <Button size="icon" variant="ghost"><Pencil className="h-4 w-4" /></Button>
                        </Link>
                        <Button size="icon" variant="ghost" onClick={() => handleDelete(e.id)}><Trash2 className="h-4 w-4" /></Button>
                      </div>
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

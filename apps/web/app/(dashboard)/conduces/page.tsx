"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { PackageCheck } from "lucide-react";
import { Card, CardContent, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Conduce {
  id: string;
  venta_id: string;
  cliente_nombre: string | null;
  direccion_entrega: string | null;
  created_at: string;
}

export default function ConducesPage() {
  const [conduces, setConduces] = useState<Conduce[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch<{ conduces: Conduce[] }>("/api/conduces").then((d) => setConduces(d.conduces)).finally(() => setLoading(false));
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Conduces</h1>
        <p className="text-sm text-muted-foreground mt-1">Historial de entregas parciales — se registran desde la venta a la que pertenecen.</p>
      </div>

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : conduces.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <PackageCheck className="h-6 w-6" />
              Aún no hay entregas registradas.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Fecha</TableHead>
                  <TableHead>Cliente</TableHead>
                  <TableHead>Dirección</TableHead>
                  <TableHead>Venta</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {conduces.map((c) => (
                  <TableRow key={c.id} className="cursor-pointer">
                    <TableCell className="text-xs text-muted-foreground">
                      <Link href={`/conduces/${c.id}` as any} className="hover:text-primary">{new Date(c.created_at).toLocaleString("es-DO")}</Link>
                    </TableCell>
                    <TableCell>{c.cliente_nombre || "Consumidor final"}</TableCell>
                    <TableCell className="text-muted-foreground">{c.direccion_entrega || "—"}</TableCell>
                    <TableCell>
                      <Link href={`/ventas/${c.venta_id}` as any} className="text-primary hover:underline text-xs font-mono">
                        Ver venta
                      </Link>
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

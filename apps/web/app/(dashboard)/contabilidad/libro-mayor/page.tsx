"use client";

import { useEffect, useState } from "react";
import { BookOpen } from "lucide-react";
import { Card, CardContent, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface CuentaResumen {
  cuenta: string;
  debe: string;
  haber: string;
  saldo: string;
}

export default function LibroMayorPage() {
  const [cuentas, setCuentas] = useState<CuentaResumen[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    apiFetch<{ cuentas: CuentaResumen[] }>("/api/contabilidad/libro-mayor").then((d) => setCuentas(d.cuentas)).finally(() => setLoading(false));
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Libro mayor</h1>
        <p className="text-sm text-muted-foreground mt-1">Saldo acumulado por cuenta contable.</p>
      </div>

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : cuentas.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <BookOpen className="h-6 w-6" />
              Sin movimientos contables todavía.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Cuenta</TableHead>
                  <TableHead className="text-right">Debe</TableHead>
                  <TableHead className="text-right">Haber</TableHead>
                  <TableHead className="text-right">Saldo</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {cuentas.map((c) => (
                  <TableRow key={c.cuenta}>
                    <TableCell className="font-medium">{c.cuenta}</TableCell>
                    <TableCell className="text-right tabular-nums text-muted-foreground">{formatDOP(c.debe)}</TableCell>
                    <TableCell className="text-right tabular-nums text-muted-foreground">{formatDOP(c.haber)}</TableCell>
                    <TableCell className="text-right tabular-nums font-semibold">{formatDOP(c.saldo)}</TableCell>
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

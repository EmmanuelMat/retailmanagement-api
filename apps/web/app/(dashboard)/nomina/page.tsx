"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { Users2, HandCoins, PlayCircle, ArrowRight } from "lucide-react";
import { Card, CardContent, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Empleado {
  id: string;
  salario_mensual: string;
}

interface Adelanto {
  estado: string;
  monto: string;
}

export default function NominaPage() {
  const [empleados, setEmpleados] = useState<Empleado[]>([]);
  const [adelantos, setAdelantos] = useState<Adelanto[]>([]);

  useEffect(() => {
    apiFetch<{ empleados: Empleado[] }>("/api/empleados").then((d) => setEmpleados(d.empleados)).catch(() => {});
    apiFetch<{ adelantos: Adelanto[] }>("/api/nomina/adelantos").then((d) => setAdelantos(d.adelantos)).catch(() => {});
  }, []);

  const nomina = empleados.reduce((sum, e) => sum + Number(e.salario_mensual), 0);
  const adelantosPendientes = adelantos.filter((a) => a.estado === "PENDIENTE").reduce((sum, a) => sum + Number(a.monto), 0);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nómina</h1>
        <p className="text-sm text-muted-foreground mt-1">Equipo, adelantos y corridas de pago.</p>
      </div>

      <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
        <Link href="/nomina/empleados">
          <Card className="hover:border-primary transition-colors">
            <CardContent className="pt-5 flex items-start justify-between">
              <div>
                <p className="text-xs text-muted-foreground">Empleados activos</p>
                <p className="text-2xl font-bold mt-1">{empleados.length}</p>
                <p className="text-xs text-muted-foreground mt-1">Nómina mensual: {formatDOP(nomina)}</p>
              </div>
              <div className="h-9 w-9 rounded-md bg-primary/10 text-primary flex items-center justify-center"><Users2 className="h-4.5 w-4.5" /></div>
            </CardContent>
          </Card>
        </Link>
        <Link href="/nomina/adelantos">
          <Card className="hover:border-primary transition-colors">
            <CardContent className="pt-5 flex items-start justify-between">
              <div>
                <p className="text-xs text-muted-foreground">Adelantos pendientes</p>
                <p className="text-2xl font-bold mt-1">{formatDOP(adelantosPendientes)}</p>
              </div>
              <div className="h-9 w-9 rounded-md bg-primary/10 text-primary flex items-center justify-center"><HandCoins className="h-4.5 w-4.5" /></div>
            </CardContent>
          </Card>
        </Link>
        <Link href="/nomina/run">
          <Card className="hover:border-primary transition-colors">
            <CardContent className="pt-5 flex items-center justify-between">
              <div>
                <p className="text-sm font-semibold">Correr nómina</p>
                <p className="text-xs text-muted-foreground mt-1">Nueva corrida de pago</p>
              </div>
              <div className="h-9 w-9 rounded-md bg-primary/10 text-primary flex items-center justify-center"><PlayCircle className="h-4.5 w-4.5" /></div>
            </CardContent>
          </Card>
        </Link>
      </div>
    </div>
  );
}

"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { CalendarDays, ArrowLeft, Clock, MapPin } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Label, Select } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { ESTADO_VARIANT } from "../estado-variant";

interface AgendaTecnico {
  empleado_id: string;
  nombre: string;
  rol: string;
}

interface AgendaOrden {
  id: string;
  codigo: string;
  cliente_nombre: string | null;
  estado: string;
  prioridad: string;
  condicion_nombre: string | null;
  fecha_programada: string;
  hora_inicio: string | null;
  hora_fin: string | null;
  direccion: string | null;
  descripcion: string | null;
  total: string;
  tecnicos: AgendaTecnico[];
}

interface Empleado {
  id: string;
  nombre: string;
}

const ESTADO_LABEL: Record<string, string> = {
  BORRADOR: "Borrador",
  PROGRAMADA: "Programada",
  EN_PROCESO: "En proceso",
  PAUSADA: "Pausada",
  COMPLETADA: "Completada",
};

const SIN_TECNICO = "__sin_tecnico__";

/** `YYYY-MM-DD` del día local, sin pasar por UTC (`toISOString()` corre el
 * día en Santo Domingo, UTC-4). */
function isoDia(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
}

function sumarDias(iso: string, dias: number): string {
  const [y, m, d] = iso.split("-").map(Number);
  const fecha = new Date(y!, (m ?? 1) - 1, d ?? 1);
  fecha.setDate(fecha.getDate() + dias);
  return isoDia(fecha);
}

/** "09:00:00" -> "09:00". El core manda TIME con segundos. */
function hhmm(hora: string | null): string | null {
  return hora ? hora.slice(0, 5) : null;
}

function rangoHorario(o: AgendaOrden): string {
  const desde = hhmm(o.hora_inicio);
  const hasta = hhmm(o.hora_fin);
  if (desde && hasta) return `${desde} – ${hasta}`;
  if (desde) return `Desde ${desde}`;
  return "Sin hora";
}

function fechaLarga(iso: string): string {
  const [y, m, d] = iso.split("-").map(Number);
  return new Date(y!, (m ?? 1) - 1, d ?? 1).toLocaleDateString("es-DO", { weekday: "long", day: "numeric", month: "long" });
}

export default function AgendaOrdenesServicioPage() {
  const [vista, setVista] = useState<"dia" | "semana">("dia");
  const [dia, setDia] = useState(() => isoDia(new Date()));
  const [empleadoId, setEmpleadoId] = useState("");
  const [empleados, setEmpleados] = useState<Empleado[]>([]);
  const [ordenes, setOrdenes] = useState<AgendaOrden[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const desde = dia;
  const hasta = vista === "dia" ? dia : sumarDias(dia, 6);

  useEffect(() => {
    apiFetch<{ items: Empleado[] }>("/api/empleados?pageSize=1000&activo=true").then((d) => setEmpleados(d.items)).catch(() => {});
  }, []);

  useEffect(() => {
    setLoading(true);
    setError("");
    const filtro = empleadoId ? `&empleado_id=${empleadoId}` : "";
    apiFetch<{ items: AgendaOrden[] }>(`/api/ordenes-servicio/agenda?desde=${desde}&hasta=${hasta}&pageSize=500${filtro}`)
      .then((d) => setOrdenes(d.items))
      .catch((e: any) => setError(e.message))
      .finally(() => setLoading(false));
  }, [desde, hasta, empleadoId]);

  // Una orden con dos técnicos aparece bajo cada uno: la agenda se lee
  // "¿qué tiene Juan hoy?", no "¿a quién le tocó esta orden?".
  const grupos = useMemo(() => {
    const porTecnico = new Map<string, { nombre: string; ordenes: AgendaOrden[] }>();
    for (const o of ordenes) {
      if (o.tecnicos.length === 0) {
        const g = porTecnico.get(SIN_TECNICO) || { nombre: "Sin técnico asignado", ordenes: [] };
        g.ordenes.push(o);
        porTecnico.set(SIN_TECNICO, g);
        continue;
      }
      for (const t of o.tecnicos) {
        const g = porTecnico.get(t.empleado_id) || { nombre: t.nombre, ordenes: [] };
        g.ordenes.push(o);
        porTecnico.set(t.empleado_id, g);
      }
    }
    return [...porTecnico.entries()]
      .sort((a, b) => (a[0] === SIN_TECNICO ? 1 : b[0] === SIN_TECNICO ? -1 : a[1].nombre.localeCompare(b[1].nombre, "es")))
      .map(([id, g]) => ({ id, ...g }));
  }, [ordenes]);

  return (
    <div className="space-y-4 max-w-6xl">
      <div className="flex items-start justify-between flex-wrap gap-3">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Agenda de técnicos</h1>
          <p className="text-sm text-muted-foreground mt-1">Trabajo programado por técnico. Las órdenes canceladas no aparecen.</p>
        </div>
        <Link href={"/ordenes-servicio" as any}>
          <Button variant="secondary" size="sm"><ArrowLeft className="h-4 w-4" />Volver a las órdenes</Button>
        </Link>
      </div>

      <Card>
        <CardContent className="pt-5 flex flex-wrap gap-3 items-end">
          <div className="space-y-1.5">
            <Label htmlFor="agenda-dia">{vista === "dia" ? "Día" : "Semana desde"}</Label>
            <Input id="agenda-dia" type="date" value={dia} onChange={(e) => setDia(e.target.value)} className="max-w-[180px]" />
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="agenda-vista">Vista</Label>
            <Select id="agenda-vista" value={vista} onChange={(e) => setVista(e.target.value as "dia" | "semana")} className="max-w-[140px]">
              <option value="dia">Día</option>
              <option value="semana">7 días</option>
            </Select>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="agenda-empleado">Técnico</Label>
            <Select id="agenda-empleado" value={empleadoId} onChange={(e) => setEmpleadoId(e.target.value)} className="max-w-[220px]">
              <option value="">Todos los técnicos</option>
              {empleados.map((e) => <option key={e.id} value={e.id}>{e.nombre}</option>)}
            </Select>
          </div>
          <div className="flex gap-2 ml-auto">
            <Button variant="secondary" size="sm" onClick={() => setDia(sumarDias(dia, vista === "dia" ? -1 : -7))}>Anterior</Button>
            <Button variant="secondary" size="sm" onClick={() => setDia(isoDia(new Date()))}>Hoy</Button>
            <Button variant="secondary" size="sm" onClick={() => setDia(sumarDias(dia, vista === "dia" ? 1 : 7))}>Siguiente</Button>
          </div>
        </CardContent>
      </Card>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      {loading ? (
        <p className="text-sm text-muted-foreground">Cargando agenda...</p>
      ) : grupos.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center text-muted-foreground">
            <CalendarDays className="h-6 w-6 mx-auto mb-2" />
            <p className="text-sm" data-testid="agenda-vacia">No hay trabajo programado en este rango.</p>
          </CardContent>
        </Card>
      ) : (
        <div className="space-y-4" data-testid="agenda-grupos">
          {grupos.map((g) => (
            <Card key={g.id} data-testid="agenda-grupo">
              <CardContent className="pt-5 space-y-3">
                <div className="flex items-center justify-between">
                  <h2 className="font-semibold">{g.nombre}</h2>
                  <span className="text-xs text-muted-foreground">{g.ordenes.length} {g.ordenes.length === 1 ? "orden" : "órdenes"}</span>
                </div>
                <div className="space-y-2">
                  {g.ordenes.map((o) => (
                    <Link
                      key={`${g.id}-${o.id}`}
                      href={`/ordenes-servicio/${o.id}` as any}
                      className="flex flex-wrap items-center gap-x-4 gap-y-1 border border-border rounded-md p-3 hover:border-primary/50 transition-colors"
                      data-testid="agenda-orden"
                    >
                      <span className="font-mono text-xs text-muted-foreground w-24 shrink-0">{o.codigo}</span>
                      <span className="text-sm tabular-nums flex items-center gap-1.5 w-32 shrink-0">
                        <Clock className="h-3.5 w-3.5 text-muted-foreground" />{rangoHorario(o)}
                      </span>
                      <span className="text-sm font-medium flex-1 min-w-[10rem]">{o.cliente_nombre || "Consumidor final"}</span>
                      {vista === "semana" && <span className="text-xs text-muted-foreground capitalize">{fechaLarga(o.fecha_programada)}</span>}
                      {o.direccion && (
                        <span className="text-xs text-muted-foreground flex items-center gap-1 max-w-[16rem] truncate">
                          <MapPin className="h-3 w-3 shrink-0" />{o.direccion}
                        </span>
                      )}
                      {o.prioridad !== "NORMAL" && <Badge variant="secondary">{o.prioridad}</Badge>}
                      <Badge variant={ESTADO_VARIANT[o.estado] || "default"}>{ESTADO_LABEL[o.estado] || o.estado}</Badge>
                    </Link>
                  ))}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

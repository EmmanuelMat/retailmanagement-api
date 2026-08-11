"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { DollarSign, Package, AlertTriangle, Lock, Unlock, ShoppingCart, Plus, Users, ShoppingBag, PlayCircle, X, Sparkles } from "lucide-react";
import { Card, CardContent, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { isRouteAllowed } from "@/lib/roles";

interface DashboardResumen {
  ventas_hoy_total: string;
  ventas_hoy_cantidad: number;
  productos_bajo_minimo: number;
  valor_inventario: string;
  caja_abierta: boolean;
}

interface AiDigest {
  mensaje: string;
  generado_por_ia: boolean;
}

const ACCIONES = [
  { label: "Nueva venta", href: "/pos", icon: ShoppingCart },
  { label: "Nuevo producto", href: "/inventario/productos/nuevo", icon: Plus },
  { label: "Nuevo cliente", href: "/clientes/nuevo", icon: Users },
  { label: "Nueva compra", href: "/compras/nueva", icon: ShoppingBag },
  { label: "Correr nómina", href: "/nomina/run", icon: PlayCircle },
  { label: "Caja", href: "/caja", icon: Lock },
];

export default function DashboardPage() {
  const searchParams = useSearchParams();
  const [resumen, setResumen] = useState<DashboardResumen | null>(null);
  const [tenant, setTenant] = useState<{ razon_social?: string; rnc?: string } | null>(null);
  const [usuario, setUsuario] = useState<{ rol?: string } | null>(null);
  const [denied, setDenied] = useState(searchParams.get("denied"));
  const [digest, setDigest] = useState<AiDigest | null>(null);
  const [digestLoading, setDigestLoading] = useState(true);

  useEffect(() => {
    apiFetch<DashboardResumen>("/api/reports/dashboard").then(setResumen).catch(() => {});
    try {
      const raw = localStorage.getItem("tenant");
      if (raw) setTenant(JSON.parse(raw));
      const u = localStorage.getItem("usuario");
      if (u) setUsuario(JSON.parse(u));
    } catch {}
  }, []);

  useEffect(() => {
    // Fetch aparte a propósito: en frío (sin caché) puede tardar varios
    // segundos (modelo local por CPU) y no debe retrasar las tarjetas de
    // arriba, que ya tienen sus propios datos listos.
    apiFetch<AiDigest>("/api/ai/digest")
      .then(setDigest)
      .catch(() => {})
      .finally(() => setDigestLoading(false));
  }, []);

  const acciones = ACCIONES.filter((a) => isRouteAllowed(a.href, usuario?.rol));

  return (
    <div className="space-y-6">
      {denied && (
        <div
          className={`flex items-start justify-between gap-3 rounded-md border p-3.5 text-sm ${
            denied === "dgii" ? "border-warning/20 bg-warning/10 text-warning" : "border-destructive/20 bg-destructive/10 text-destructive"
          }`}
        >
          <span>
            {denied === "dgii"
              ? "Esta sección no aplica: tu negocio no tiene factura electrónica (e-CF) activada. Actívala en Configuración → Mi negocio si la necesitas."
              : "No tienes permiso para acceder a esa sección."}
          </span>
          <button onClick={() => setDenied(null)} aria-label="Cerrar" className="shrink-0 hover:opacity-70">
            <X className="h-4 w-4" />
          </button>
        </div>
      )}

      <div>
        <h1 className="text-2xl font-bold font-serif">{tenant?.razon_social || "Dashboard"}</h1>
        <p className="text-sm text-muted-foreground mt-1.5 capitalize">
          {tenant?.rnc ? `RNC ${tenant.rnc} · ` : ""}
          {new Date().toLocaleDateString("es-DO", { weekday: "long", year: "numeric", month: "long", day: "numeric" })}
        </p>
      </div>

      <Card className="border-primary/15 bg-primary/[0.03]">
        <CardContent className="pt-5">
          <div className="flex items-center gap-2 mb-2">
            <Sparkles className="h-4 w-4 text-primary" />
            <p className="text-sm font-semibold">Resumen del día</p>
          </div>
          {digestLoading ? (
            <div className="space-y-1.5 animate-pulse">
              <div className="h-3 bg-muted rounded w-3/4" />
              <div className="h-3 bg-muted rounded w-1/2" />
            </div>
          ) : digest ? (
            <>
              <p className="text-sm text-foreground/85 leading-relaxed">{digest.mensaje}</p>
              {!digest.generado_por_ia && (
                <p className="text-xs text-muted-foreground mt-1.5">(IA no disponible, resumen simple)</p>
              )}
            </>
          ) : (
            <p className="text-sm text-muted-foreground">No se pudo cargar el resumen.</p>
          )}
        </CardContent>
      </Card>

      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4">
        <Card className="h-full hover:shadow-card transition-shadow">
          <CardContent className="pt-5 flex items-start justify-between">
            <div>
              <p className="text-xs font-medium text-muted-foreground">Ventas hoy</p>
              <p className="text-2xl font-bold mt-1.5">{resumen ? formatDOP(resumen.ventas_hoy_total) : "…"}</p>
              <p className="text-xs text-muted-foreground mt-1">{resumen?.ventas_hoy_cantidad ?? 0} transacciones</p>
            </div>
            <div className="h-10 w-10 rounded-lg bg-primary/10 text-primary flex items-center justify-center"><DollarSign className="h-4.5 w-4.5" /></div>
          </CardContent>
        </Card>
        <Card className="h-full hover:shadow-card transition-shadow">
          <CardContent className="pt-5 flex items-start justify-between">
            <div>
              <p className="text-xs font-medium text-muted-foreground">Valor de inventario</p>
              <p className="text-2xl font-bold mt-1.5">{resumen ? formatDOP(resumen.valor_inventario) : "…"}</p>
            </div>
            <div className="h-10 w-10 rounded-lg bg-primary/10 text-primary flex items-center justify-center"><Package className="h-4.5 w-4.5" /></div>
          </CardContent>
        </Card>
        <Card className="h-full hover:shadow-card transition-shadow">
          <CardContent className="pt-5 flex items-start justify-between">
            <div>
              <p className="text-xs font-medium text-muted-foreground">Bajo stock mínimo</p>
              <p className="text-2xl font-bold mt-1.5">{resumen?.productos_bajo_minimo ?? "…"}</p>
            </div>
            <div className="h-10 w-10 rounded-lg bg-destructive/10 text-destructive flex items-center justify-center"><AlertTriangle className="h-4.5 w-4.5" /></div>
          </CardContent>
        </Card>
        <Link href="/caja">
          <Card className="hover:shadow-card hover:border-primary/40 transition-all h-full">
            <CardContent className="pt-5 flex items-start justify-between">
              <div>
                <p className="text-xs font-medium text-muted-foreground">Caja</p>
                <p className="text-lg font-bold mt-1.5">{resumen?.caja_abierta ? "Abierta" : "Cerrada"}</p>
              </div>
              <div className="h-10 w-10 rounded-lg bg-primary/10 text-primary flex items-center justify-center">
                {resumen?.caja_abierta ? <Unlock className="h-4.5 w-4.5" /> : <Lock className="h-4.5 w-4.5" />}
              </div>
            </CardContent>
          </Card>
        </Link>
      </div>

      <Card>
        <CardContent className="pt-5">
          <p className="text-sm font-semibold mb-3.5">Acciones rápidas</p>
          <div className="grid grid-cols-2 sm:grid-cols-3 lg:grid-cols-6 gap-2.5">
            {acciones.map((a) => (
              <Link key={a.href} href={a.href as any}>
                <div className="flex flex-col items-center gap-2 rounded-lg border border-border p-4 text-center hover:border-primary/40 hover:bg-primary/5 hover:-translate-y-0.5 transition-all">
                  <div className="h-8 w-8 rounded-md bg-primary/10 text-primary flex items-center justify-center">
                    <a.icon className="h-4 w-4" />
                  </div>
                  <span className="text-xs font-medium">{a.label}</span>
                </div>
              </Link>
            ))}
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

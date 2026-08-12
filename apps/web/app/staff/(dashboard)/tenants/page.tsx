"use client";

import { useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Plus, Search, Building2 } from "lucide-react";
import { Badge, Button, Input, Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@repo/ui";
import { apiFetch } from "@/lib/staff-api";

interface TenantResumen {
  rnc: string;
  razon_social: string;
  correo: string | null;
  telefono: string | null;
  license_status: "trial" | "active" | "expired";
  trial_started_at: string;
  trial_days: number;
  license_activated_at: string | null;
  activo: boolean;
  created_at: string;
}

function diasRestantes(t: TenantResumen): number {
  const inicio = new Date(t.trial_started_at).getTime();
  const transcurridos = Math.floor((Date.now() - inicio) / 86400000);
  return Math.max(t.trial_days - transcurridos, 0);
}

function EstadoBadge({ t }: { t: TenantResumen }) {
  if (t.license_status === "active") return <Badge variant="success">Activo</Badge>;
  if (t.license_status === "expired") return <Badge variant="destructive">Vencido</Badge>;
  const dias = diasRestantes(t);
  return <Badge variant={dias <= 14 ? "warning" : "secondary"}>Prueba · {dias}d</Badge>;
}

export default function TenantsPage() {
  const [tenants, setTenants] = useState<TenantResumen[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");

  useEffect(() => {
    apiFetch<TenantResumen[]>("/api/tenants")
      .then(setTenants)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  const filtrados = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return tenants;
    return tenants.filter((t) => t.razon_social.toLowerCase().includes(q) || t.rnc.includes(q) || (t.correo || "").toLowerCase().includes(q));
  }, [tenants, search]);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Negocios</h1>
          <p className="text-sm text-muted-foreground mt-1">Todos los tenants — prueba, activos y vencidos.</p>
        </div>
        <Link href={"/tenants/nuevo" as any}>
          <Button><Plus className="h-4 w-4" />Nuevo negocio</Button>
        </Link>
      </div>

      <div className="relative max-w-sm">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Buscar por nombre, RNC o correo..." className="pl-9" />
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <div className="rounded-lg border border-border bg-surface overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Negocio</TableHead>
              <TableHead className="font-mono">RNC</TableHead>
              <TableHead>Correo</TableHead>
              <TableHead>Estado</TableHead>
              <TableHead className="w-20 text-right">Ver</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              <TableRow><TableCell colSpan={5} className="text-center text-muted-foreground py-8">Cargando...</TableCell></TableRow>
            ) : filtrados.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center text-muted-foreground py-12">
                  <Building2 className="h-6 w-6 mx-auto mb-2 opacity-50" />
                  No hay negocios todavía.
                </TableCell>
              </TableRow>
            ) : (
              filtrados.map((t) => (
                <TableRow key={t.rnc}>
                  <TableCell className="font-medium">{t.razon_social}</TableCell>
                  <TableCell className="font-mono text-xs text-muted-foreground">{t.rnc}</TableCell>
                  <TableCell className="text-muted-foreground">{t.correo || "—"}</TableCell>
                  <TableCell><EstadoBadge t={t} /></TableCell>
                  <TableCell className="text-right">
                    <Link href={`/tenants/${t.rnc}` as any}>
                      <Button size="sm" variant="ghost">Ver</Button>
                    </Link>
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>
    </div>
  );
}

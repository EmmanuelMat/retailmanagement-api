"use client";

import { Fragment, useEffect, useState } from "react";
import Link from "next/link";
import { Plus, Pencil, Trash2, Users, Search, HandCoins } from "lucide-react";
import { Badge, Button, Card, CardContent, Input, Select, Table, TableBody, TableCell, TableHead, TableHeader, TableRow, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Cliente {
  id: string;
  nombre: string;
  rnc_cedula: string | null;
  telefono: string | null;
  email: string | null;
  saldo_pendiente: string;
  limite_credito: string;
}

export default function ClientesPage() {
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [search, setSearch] = useState("");
  const [abonandoId, setAbonandoId] = useState<string | null>(null);
  const [abonoMonto, setAbonoMonto] = useState("");
  const [abonoMetodo, setAbonoMetodo] = useState("EFECTIVO");
  const [abonoGuardando, setAbonoGuardando] = useState(false);
  const [abonoError, setAbonoError] = useState("");

  async function load() {
    setLoading(true);
    try {
      const qs = search ? `?search=${encodeURIComponent(search)}` : "";
      const data = await apiFetch<{ clientes: Cliente[] }>(`/api/clientes${qs}`);
      setClientes(data.clientes);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const t = setTimeout(load, 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [search]);

  async function handleDelete(id: string) {
    if (!confirm("¿Desactivar este cliente?")) return;
    try {
      await apiFetch(`/api/clientes/${id}`, { method: "DELETE" });
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  function abrirAbono(id: string) {
    setAbonandoId(id);
    setAbonoMonto("");
    setAbonoMetodo("EFECTIVO");
    setAbonoError("");
  }

  async function handleAbonar(id: string) {
    const monto = Number(abonoMonto);
    if (!monto || monto <= 0) {
      setAbonoError("Ingresa un monto válido");
      return;
    }
    setAbonoGuardando(true);
    setAbonoError("");
    try {
      await apiFetch(`/api/clientes/${id}/abonos`, {
        method: "POST",
        body: JSON.stringify({ monto: String(monto), metodo_pago: abonoMetodo }),
      });
      setAbonandoId(null);
      await load();
    } catch (e: any) {
      setAbonoError(e.message);
    } finally {
      setAbonoGuardando(false);
    }
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Clientes</h1>
          <p className="text-sm text-muted-foreground mt-1">Consumidor final o clientes con RNC para crédito fiscal.</p>
        </div>
        <Link href="/clientes/nuevo">
          <Button><Plus className="h-4 w-4" />Nuevo cliente</Button>
        </Link>
      </div>

      <div className="relative max-w-sm">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <Input value={search} onChange={(e) => setSearch(e.target.value)} placeholder="Buscar por nombre o RNC..." className="pl-9" />
      </div>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      <Card>
        <CardContent className="p-0">
          {loading ? (
            <p className="p-5 text-sm text-muted-foreground">Cargando...</p>
          ) : clientes.length === 0 ? (
            <div className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2">
              <Users className="h-6 w-6" />
              No hay clientes todavía.
            </div>
          ) : (
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Nombre</TableHead>
                  <TableHead>RNC / Cédula</TableHead>
                  <TableHead>Teléfono</TableHead>
                  <TableHead>Correo</TableHead>
                  <TableHead>Saldo fiado</TableHead>
                  <TableHead className="w-32 text-right">Acciones</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {clientes.map((c) => {
                  const saldo = Number(c.saldo_pendiente || "0");
                  return (
                    <Fragment key={c.id}>
                      <TableRow>
                        <TableCell className="font-medium">{c.nombre}</TableCell>
                        <TableCell className="font-mono text-xs text-muted-foreground">{c.rnc_cedula || "—"}</TableCell>
                        <TableCell className="text-muted-foreground">{c.telefono || "—"}</TableCell>
                        <TableCell className="text-muted-foreground">{c.email || "—"}</TableCell>
                        <TableCell>
                          {saldo > 0 ? <Badge variant="warning">{formatDOP(c.saldo_pendiente)}</Badge> : <span className="text-muted-foreground">—</span>}
                        </TableCell>
                        <TableCell className="text-right">
                          <div className="flex justify-end gap-1">
                            {saldo > 0 && (
                              <Button size="icon" variant="ghost" title="Registrar abono" onClick={() => abrirAbono(c.id)}>
                                <HandCoins className="h-4 w-4" />
                              </Button>
                            )}
                            <Link href={`/clientes/${c.id}` as any}>
                              <Button size="icon" variant="ghost"><Pencil className="h-4 w-4" /></Button>
                            </Link>
                            <Button size="icon" variant="ghost" onClick={() => handleDelete(c.id)}><Trash2 className="h-4 w-4" /></Button>
                          </div>
                        </TableCell>
                      </TableRow>
                      {abonandoId === c.id && (
                        <TableRow>
                          <TableCell colSpan={6} className="bg-muted/30">
                            <div className="flex flex-wrap items-end gap-2 py-1">
                              <div className="space-y-1">
                                <label className="text-xs text-muted-foreground">Monto del abono</label>
                                <Input
                                  type="number"
                                  min={0}
                                  step="0.01"
                                  value={abonoMonto}
                                  onChange={(e) => setAbonoMonto(e.target.value)}
                                  placeholder="0.00"
                                  className="h-8 w-32 text-sm"
                                />
                              </div>
                              <div className="space-y-1">
                                <label className="text-xs text-muted-foreground">Método</label>
                                <Select value={abonoMetodo} onChange={(e) => setAbonoMetodo(e.target.value)} className="h-8 text-sm">
                                  <option value="EFECTIVO">Efectivo</option>
                                  <option value="TARJETA">Tarjeta</option>
                                  <option value="TRANSFERENCIA">Transferencia</option>
                                </Select>
                              </div>
                              <Button size="sm" disabled={abonoGuardando} onClick={() => handleAbonar(c.id)}>
                                {abonoGuardando ? "Guardando..." : "Registrar abono"}
                              </Button>
                              <Button size="sm" variant="ghost" onClick={() => setAbonandoId(null)}>Cancelar</Button>
                              {abonoError && <span className="text-xs text-destructive">{abonoError}</span>}
                            </div>
                          </TableCell>
                        </TableRow>
                      )}
                    </Fragment>
                  );
                })}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>
    </div>
  );
}

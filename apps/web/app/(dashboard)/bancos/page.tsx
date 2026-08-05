"use client";

import { useEffect, useState } from "react";
import { Plus, Landmark, ArrowDownCircle, ArrowUpCircle } from "lucide-react";
import { Button, Card, CardContent, CardHeader, CardTitle, Input, Label, Select, formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Banco {
  id: string;
  nombre_banco: string;
  numero_cuenta: string | null;
  tipo_cuenta: string;
  saldo: string;
}

export default function BancosPage() {
  const [bancos, setBancos] = useState<Banco[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const [nombreBanco, setNombreBanco] = useState("");
  const [numeroCuenta, setNumeroCuenta] = useState("");
  const [tipoCuenta, setTipoCuenta] = useState("CORRIENTE");
  const [saldoInicial, setSaldoInicial] = useState("");
  const [saving, setSaving] = useState(false);

  const [movBanco, setMovBanco] = useState<Record<string, { tipo: string; monto: string }>>({});

  async function load() {
    setLoading(true);
    try {
      const data = await apiFetch<{ items: Banco[] }>("/api/bancos?pageSize=200&activo=true");
      setBancos(data.items);
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
    if (!nombreBanco.trim()) return;
    setSaving(true);
    setError("");
    try {
      await apiFetch("/api/bancos", {
        method: "POST",
        body: JSON.stringify({ nombre_banco: nombreBanco, numero_cuenta: numeroCuenta || undefined, tipo_cuenta: tipoCuenta, saldo: saldoInicial || undefined }),
      });
      setNombreBanco("");
      setNumeroCuenta("");
      setSaldoInicial("");
      await load();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  async function handleMovimiento(bancoId: string) {
    const mov = movBanco[bancoId];
    if (!mov?.monto) return;
    setError("");
    try {
      await apiFetch(`/api/bancos/${bancoId}/movimientos`, {
        method: "POST",
        body: JSON.stringify({ tipo: mov.tipo || "DEPOSITO", monto: mov.monto }),
      });
      setMovBanco((m) => ({ ...m, [bancoId]: { tipo: "DEPOSITO", monto: "" } }));
      await load();
    } catch (e: any) {
      setError(e.message);
    }
  }

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Bancos</h1>
        <p className="text-sm text-muted-foreground mt-1">Cuentas bancarias y sus depósitos/retiros manuales.</p>
      </div>

      <Card>
        <CardHeader><CardTitle className="text-sm">Nueva cuenta</CardTitle></CardHeader>
        <CardContent>
          <form onSubmit={handleCreate} className="grid grid-cols-1 sm:grid-cols-5 gap-3 items-end">
            <div className="sm:col-span-2 space-y-1.5">
              <Label htmlFor="nombreBanco">Banco</Label>
              <Input id="nombreBanco" value={nombreBanco} onChange={(e) => setNombreBanco(e.target.value)} placeholder="Banco Popular" required />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="numeroCuenta">No. cuenta</Label>
              <Input id="numeroCuenta" value={numeroCuenta} onChange={(e) => setNumeroCuenta(e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="tipoCuenta">Tipo</Label>
              <Select id="tipoCuenta" value={tipoCuenta} onChange={(e) => setTipoCuenta(e.target.value)}>
                <option value="CORRIENTE">Corriente</option>
                <option value="AHORROS">Ahorros</option>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="saldoInicial">Saldo inicial</Label>
              <Input id="saldoInicial" type="number" step="0.01" value={saldoInicial} onChange={(e) => setSaldoInicial(e.target.value)} placeholder="0.00" />
            </div>
            <Button type="submit" disabled={saving} className="sm:col-span-5 sm:w-fit"><Plus className="h-4 w-4" />Agregar cuenta</Button>
          </form>
        </CardContent>
      </Card>

      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

      {loading ? (
        <p className="text-sm text-muted-foreground">Cargando...</p>
      ) : bancos.length === 0 ? (
        <Card><CardContent className="p-8 text-center text-sm text-muted-foreground flex flex-col items-center gap-2"><Landmark className="h-6 w-6" />No hay cuentas bancarias todavía.</CardContent></Card>
      ) : (
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          {bancos.map((b) => (
            <Card key={b.id}>
              <CardContent className="pt-5">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="font-semibold text-sm">{b.nombre_banco}</p>
                    <p className="text-xs text-muted-foreground">{b.numero_cuenta || "—"} · {b.tipo_cuenta}</p>
                  </div>
                  <p className="text-xl font-bold tabular-nums">{formatDOP(b.saldo)}</p>
                </div>
                <div className="flex gap-2 mt-4">
                  <Select
                    className="w-28"
                    value={movBanco[b.id]?.tipo || "DEPOSITO"}
                    onChange={(e) => setMovBanco((m) => ({ ...m, [b.id]: { tipo: e.target.value, monto: m[b.id]?.monto || "" } }))}
                  >
                    <option value="DEPOSITO">Depósito</option>
                    <option value="RETIRO">Retiro</option>
                  </Select>
                  <Input
                    type="number"
                    step="0.01"
                    placeholder="Monto"
                    value={movBanco[b.id]?.monto || ""}
                    onChange={(e) => setMovBanco((m) => ({ ...m, [b.id]: { tipo: m[b.id]?.tipo || "DEPOSITO", monto: e.target.value } }))}
                  />
                  <Button size="icon" variant="secondary" onClick={() => handleMovimiento(b.id)}>
                    {movBanco[b.id]?.tipo === "RETIRO" ? <ArrowUpCircle className="h-4 w-4" /> : <ArrowDownCircle className="h-4 w-4" />}
                  </Button>
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}
    </div>
  );
}

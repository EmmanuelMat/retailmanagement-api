"use client";

import { useEffect, useState } from "react";
import { ChevronDown, IdCard } from "lucide-react";
import { Badge, Button, Input, Label, Select } from "@repo/ui";
import { apiFetch, ApiError } from "@/lib/api";

interface Cliente {
  id: string;
  nombre: string;
  rnc_cedula?: string | null;
  direccion?: string | null;
}

interface RncRecord {
  rnc: string;
  nombre: string;
  nombre_comercial: string | null;
  estado: string | null;
}

/**
 * Selector de cliente reusado en cotizaciones (crear y editar) y modelado
 * exactamente sobre el bloque RNC/Cliente de pos/page.tsx: cliente
 * registrado vía <select>, o resuelto por RNC/Cédula contra el padrón DGII
 * (creando o reusando un Cliente liviano), o registrado sin RNC (solo
 * nombre) para alguien que no es cliente todavía - "Consumidor final" sigue
 * siendo simplemente clienteId === "".
 */
export function ClientePicker({ clienteId, onChange }: { clienteId: string; onChange: (clienteId: string) => void }) {
  const [clientes, setClientes] = useState<Cliente[]>([]);
  const [rncQuery, setRncQuery] = useState("");
  const [rncBuscando, setRncBuscando] = useState(false);
  const [rncFound, setRncFound] = useState<RncRecord | null>(null);
  const [rncNotFound, setRncNotFound] = useState(false);
  const [quickNombre, setQuickNombre] = useState("");
  const [quickDireccion, setQuickDireccion] = useState("");
  const [guardando, setGuardando] = useState(false);
  const [mostrarPanel, setMostrarPanel] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<{ items: Cliente[] }>("/api/clientes?pageSize=1000&activo=true").then((d) => setClientes(d.items)).catch(() => {});
  }, []);

  async function handleVerificarRnc() {
    const rnc = rncQuery.trim();
    if (!rnc) return;
    setRncBuscando(true);
    setRncFound(null);
    setRncNotFound(false);
    setError("");
    try {
      const data = await apiFetch<RncRecord>(`/api/rnc/${encodeURIComponent(rnc)}`);
      setRncFound(data);
      setQuickNombre(data.nombre_comercial || data.nombre);
    } catch (e) {
      if (e instanceof ApiError && e.status === 404) {
        setRncNotFound(true);
        setQuickNombre("");
        setQuickDireccion("");
      } else {
        setError(e instanceof Error ? e.message : "No se pudo verificar el RNC");
      }
    } finally {
      setRncBuscando(false);
    }
  }

  function limpiarPanel() {
    setRncQuery("");
    setRncFound(null);
    setRncNotFound(false);
    setQuickNombre("");
    setQuickDireccion("");
    setMostrarPanel(false);
  }

  async function handleUsarClienteRnc() {
    const rnc = rncQuery.trim().replace(/\D/g, "");
    if (!rnc || !quickNombre.trim()) return;
    setGuardando(true);
    setError("");
    try {
      const existing = clientes.find((c) => (c.rnc_cedula || "").replace(/\D/g, "") === rnc);
      let cliente: Cliente;
      if (existing) {
        cliente = await apiFetch<Cliente>(`/api/clientes/${existing.id}`, {
          method: "PUT",
          body: JSON.stringify({ nombre: quickNombre, rnc_cedula: rnc, direccion: quickDireccion || undefined }),
        });
        setClientes((cs) => cs.map((c) => (c.id === cliente.id ? cliente : c)));
      } else {
        cliente = await apiFetch<Cliente>("/api/clientes", {
          method: "POST",
          body: JSON.stringify({ nombre: quickNombre, rnc_cedula: rnc, direccion: quickDireccion || undefined }),
        });
        setClientes((cs) => [...cs, cliente]);
      }
      onChange(cliente.id);
      limpiarPanel();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setGuardando(false);
    }
  }

  async function handleRegistrarSinRnc() {
    if (!quickNombre.trim()) return;
    setGuardando(true);
    setError("");
    try {
      const cliente = await apiFetch<Cliente>("/api/clientes", {
        method: "POST",
        body: JSON.stringify({ nombre: quickNombre, direccion: quickDireccion || undefined }),
      });
      setClientes((cs) => [...cs, cliente]);
      onChange(cliente.id);
      limpiarPanel();
    } catch (e: any) {
      setError(e.message);
    } finally {
      setGuardando(false);
    }
  }

  return (
    <div className="space-y-2">
      <div className="space-y-1.5">
        <Label htmlFor="cliente">Cliente</Label>
        <Select id="cliente" value={clienteId} onChange={(e) => onChange(e.target.value)}>
          <option value="">Consumidor final</option>
          {clientes.map((c) => (
            <option key={c.id} value={c.id}>{c.nombre}</option>
          ))}
        </Select>
      </div>

      <button
        type="button"
        onClick={() => setMostrarPanel((v) => !v)}
        className="flex items-center justify-between w-full rounded-md border border-border p-2.5 text-xs font-medium hover:bg-muted transition-colors"
      >
        <span>Buscar por RNC o registrar uno nuevo</span>
        <ChevronDown className={`h-3.5 w-3.5 text-muted-foreground transition-transform ${mostrarPanel ? "rotate-180" : ""}`} />
      </button>

      {mostrarPanel && (
        <div className="space-y-3 rounded-md border border-border p-2.5">
          <div className="space-y-2">
            <Label htmlFor="rncQuery">RNC / Cédula</Label>
            <div className="flex gap-2">
              <Input
                id="rncQuery"
                value={rncQuery}
                onChange={(e) => {
                  setRncQuery(e.target.value);
                  setRncFound(null);
                  setRncNotFound(false);
                }}
                placeholder="130793752"
                onKeyDown={(e) => e.key === "Enter" && (e.preventDefault(), handleVerificarRnc())}
              />
              <Button type="button" variant="secondary" onClick={handleVerificarRnc} disabled={rncBuscando || !rncQuery.trim()}>
                <IdCard className="h-4 w-4" />{rncBuscando ? "..." : "Verificar"}
              </Button>
            </div>

            {rncFound && (
              <div className="rounded-md border border-border p-2.5 space-y-2">
                <p className="text-xs text-muted-foreground flex items-center gap-1.5">
                  Encontrado en DGII
                  <Badge variant={rncFound.estado === "ACTIVO" ? "success" : "secondary"}>{rncFound.estado || "—"}</Badge>
                </p>
                <Input value={quickNombre} onChange={(e) => setQuickNombre(e.target.value)} placeholder="Nombre del cliente" />
                <Input value={quickDireccion} onChange={(e) => setQuickDireccion(e.target.value)} placeholder="Dirección (opcional)" />
                <Button type="button" size="sm" className="w-full" onClick={handleUsarClienteRnc} disabled={guardando || !quickNombre.trim()}>
                  {guardando ? "Guardando..." : "Usar este cliente"}
                </Button>
              </div>
            )}

            {rncNotFound && (
              <div className="rounded-md border border-warning/20 bg-warning/10 p-2.5 space-y-2">
                <p className="text-xs text-warning">No encontrado en el padrón DGII. Puedes registrarlo manualmente:</p>
                <Input value={quickNombre} onChange={(e) => setQuickNombre(e.target.value)} placeholder="Nombre del cliente *" />
                <Input value={quickDireccion} onChange={(e) => setQuickDireccion(e.target.value)} placeholder="Dirección (opcional)" />
                <Button type="button" size="sm" className="w-full" onClick={handleUsarClienteRnc} disabled={guardando || !quickNombre.trim()}>
                  {guardando ? "Guardando..." : "Usar este cliente"}
                </Button>
              </div>
            )}
          </div>

          {!rncFound && !rncNotFound && (
            <div className="space-y-2 pt-2 border-t border-border">
              <p className="text-xs text-muted-foreground">O regístralo sin RNC — alguien que aún no es cliente:</p>
              <Input value={quickNombre} onChange={(e) => setQuickNombre(e.target.value)} placeholder="Nombre *" />
              <Input value={quickDireccion} onChange={(e) => setQuickDireccion(e.target.value)} placeholder="Dirección (opcional)" />
              <Button type="button" size="sm" variant="secondary" className="w-full" onClick={handleRegistrarSinRnc} disabled={guardando || !quickNombre.trim()}>
                {guardando ? "Guardando..." : "Registrar y usar"}
              </Button>
            </div>
          )}

          {error && <p className="text-xs text-destructive">{error}</p>}
        </div>
      )}
    </div>
  );
}

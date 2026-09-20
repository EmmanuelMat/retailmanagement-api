"use client";

import { useMemo, useState } from "react";
import { Search } from "lucide-react";
import { Input } from "@repo/ui";

export interface ProductoPickerItem {
  id: string;
  sku: string;
  nombre: string;
  tipo: "PRODUCTO" | "SERVICIO";
  precio_venta: string | null;
}

/**
 * Buscador de producto/servicio por nombre o código, compartido entre
 * cotizaciones y órdenes de servicio (ver
 * docs/superpowers/specs/2026-09-13-cotizacion-servicio-pricing-design.md).
 * Filtro client-side sobre la lista ya cargada, igual que pos/page.tsx -
 * un catálogo de miles de SKUs no justifica un roundtrip por cada tecla.
 */
export function ProductoPicker({
  productos,
  value,
  onChange,
  placeholder = "Buscar por nombre o código…",
}: {
  productos: ProductoPickerItem[];
  value: string;
  onChange: (id: string) => void;
  placeholder?: string;
}) {
  const [query, setQuery] = useState("");
  const [abierto, setAbierto] = useState(false);

  const seleccionado = productos.find((p) => p.id === value);

  const filtrados = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return productos.slice(0, 20);
    return productos.filter((p) => p.nombre.toLowerCase().includes(q) || p.sku.toLowerCase().includes(q)).slice(0, 20);
  }, [query, productos]);

  return (
    <div className="relative">
      <div className="relative">
        <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-3.5 w-3.5 text-muted-foreground" />
        <Input
          className="pl-8"
          value={abierto ? query : seleccionado ? `${seleccionado.sku} · ${seleccionado.nombre}` : ""}
          placeholder={placeholder}
          onFocus={() => {
            setAbierto(true);
            setQuery("");
          }}
          onChange={(e) => setQuery(e.target.value)}
          onBlur={() => setTimeout(() => setAbierto(false), 150)}
        />
      </div>
      {abierto && (
        <div className="absolute z-10 mt-1 w-full max-h-64 overflow-y-auto rounded-md border border-border bg-surface shadow-md">
          {filtrados.length === 0 && <p className="p-2.5 text-xs text-muted-foreground">Sin resultados</p>}
          {filtrados.map((p) => (
            <button
              key={p.id}
              type="button"
              className="flex w-full items-center justify-between gap-2 px-2.5 py-2 text-left text-sm hover:bg-muted transition-colors"
              onMouseDown={(e) => {
                e.preventDefault();
                onChange(p.id);
                setAbierto(false);
              }}
            >
              <span>
                <span className="font-mono text-xs text-muted-foreground mr-2">{p.sku}</span>
                {p.nombre}
              </span>
              <span className="text-xs text-muted-foreground shrink-0">
                {p.tipo === "SERVICIO" ? "Servicio" : `RD$ ${Number(p.precio_venta || 0).toLocaleString("es-DO", { minimumFractionDigits: 2 })}`}
              </span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

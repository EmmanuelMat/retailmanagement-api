"use client";

import { useEffect, useState } from "react";
import { ProductoForm, ProductoFormValues } from "../producto-form";
import { apiFetch } from "@/lib/api";

export default function NuevoProductoPage() {
  // Si venimos del Asistente con una acción confirmada, precarga el
  // formulario - la creación real sigue pasando por el mismo onSubmit/POST
  // de siempre, la IA nunca crea nada directamente.
  const [initial, setInitial] = useState<Partial<ProductoFormValues> | undefined>(undefined);

  useEffect(() => {
    const raw = sessionStorage.getItem("ai_prefill_producto");
    if (!raw) return;
    sessionStorage.removeItem("ai_prefill_producto");
    try {
      const parsed = JSON.parse(raw);
      setInitial(Object.fromEntries(Object.entries(parsed).map(([k, v]) => [k, String(v)])));
    } catch {}
  }, []);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Nuevo producto</h1>
        <p className="text-sm text-muted-foreground mt-1">SKU, categoría, ITBIS, costo y precio.</p>
      </div>
      <ProductoForm
        key={initial ? "prefilled" : "empty"}
        initial={initial}
        submitLabel="Crear producto"
        onSubmit={async (values) => {
          await apiFetch("/api/productos", {
            method: "POST",
            body: JSON.stringify({
              sku: values.sku,
              nombre: values.nombre,
              categoria_id: values.categoria_id || undefined,
              descripcion: values.descripcion || undefined,
              itbis_tipo: values.itbis_tipo,
              unidad_medida: values.unidad_medida,
              costo: values.costo,
              precio_venta: values.precio_venta,
              stock_actual: values.stock_actual,
              stock_minimo: values.stock_minimo,
            }),
          });
        }}
      />
    </div>
  );
}

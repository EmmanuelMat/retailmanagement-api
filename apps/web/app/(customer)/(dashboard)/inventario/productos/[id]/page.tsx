"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { ProductoForm, ProductoFormValues } from "../producto-form";
import { apiFetch } from "@/lib/api";

interface ProductoDto {
  id: string;
  categoria_id: string | null;
  proveedor_id: string | null;
  sku: string;
  nombre: string;
  descripcion: string | null;
  unidad_medida: string;
  itbis_tipo: string;
  costo: string;
  precio_venta: string | null;
  stock_actual: string;
  stock_minimo: string;
  imagen_url: string | null;
  tipo: "PRODUCTO" | "SERVICIO";
}

export default function EditarProductoPage() {
  const params = useParams<{ id: string }>();
  const [initial, setInitial] = useState<Partial<ProductoFormValues> | null>(null);
  const [imagenUrl, setImagenUrl] = useState<string | null>(null);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<ProductoDto>(`/api/productos/${params.id}`)
      .then((p) => {
        setImagenUrl(p.imagen_url);
        setInitial({
          sku: p.sku,
          nombre: p.nombre,
          categoria_id: p.categoria_id || "",
          proveedor_id: p.proveedor_id || "",
          descripcion: p.descripcion || "",
          itbis_tipo: p.itbis_tipo,
          unidad_medida: p.unidad_medida,
          costo: p.costo,
          precio_venta: p.precio_venta ?? "0",
          stock_actual: p.stock_actual,
          stock_minimo: p.stock_minimo,
          tipo: p.tipo,
        });
      })
      .catch((e) => setError(e.message));
  }, [params.id]);

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-2xl font-bold font-serif tracking-tight">Editar producto</h1>
        <p className="text-sm text-muted-foreground mt-1">SKU, categoría, ITBIS, costo y precio.</p>
      </div>
      {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm max-w-2xl">{error}</div>}
      {initial && (
        <ProductoForm
          initial={initial}
          productoId={params.id}
          imagenUrl={imagenUrl}
          submitLabel="Guardar cambios"
          onSubmit={async (values) => {
            await apiFetch(`/api/productos/${params.id}`, {
              method: "PUT",
              body: JSON.stringify({
                sku: values.sku,
                nombre: values.nombre,
                categoria_id: values.categoria_id || null,
                proveedor_id: values.proveedor_id || null,
                descripcion: values.descripcion || undefined,
                itbis_tipo: values.itbis_tipo,
                unidad_medida: values.unidad_medida,
                tipo: values.tipo,
                costo: values.tipo === "SERVICIO" ? undefined : values.costo,
                precio_venta: values.tipo === "SERVICIO" ? undefined : values.precio_venta,
                stock_actual: values.tipo === "SERVICIO" ? undefined : values.stock_actual,
                stock_minimo: values.tipo === "SERVICIO" ? undefined : values.stock_minimo,
              }),
            });
          }}
        />
      )}
    </div>
  );
}

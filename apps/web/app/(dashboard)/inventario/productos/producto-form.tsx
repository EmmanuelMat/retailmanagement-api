"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { Button, Card, CardContent, Input, Label, Select } from "@repo/ui";
import { apiFetch } from "@/lib/api";

interface Categoria {
  id: string;
  nombre: string;
}

export interface ProductoFormValues {
  sku: string;
  nombre: string;
  categoria_id: string;
  descripcion: string;
  itbis_tipo: string;
  unidad_medida: string;
  costo: string;
  precio_venta: string;
  stock_actual: string;
  stock_minimo: string;
}

const EMPTY: ProductoFormValues = {
  sku: "",
  nombre: "",
  categoria_id: "",
  descripcion: "",
  itbis_tipo: "GRAVADO_18",
  unidad_medida: "43",
  costo: "0",
  precio_venta: "0",
  stock_actual: "0",
  stock_minimo: "0",
};

export function ProductoForm({
  initial,
  submitLabel,
  onSubmit,
}: {
  initial?: Partial<ProductoFormValues>;
  submitLabel: string;
  onSubmit: (values: ProductoFormValues) => Promise<void>;
}) {
  const router = useRouter();
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [values, setValues] = useState<ProductoFormValues>({ ...EMPTY, ...initial });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    apiFetch<{ items: Categoria[] }>("/api/categorias?pageSize=1000&activo=true").then((d) => setCategorias(d.items)).catch(() => {});
  }, []);

  function set<K extends keyof ProductoFormValues>(key: K, value: ProductoFormValues[K]) {
    setValues((v) => ({ ...v, [key]: value }));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      await onSubmit(values);
      router.push("/inventario/productos");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  return (
    <Card className="max-w-2xl">
      <CardContent className="pt-5">
        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="sku">SKU *</Label>
              <Input id="sku" required value={values.sku} onChange={(e) => set("sku", e.target.value)} placeholder="ARR-001" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="categoria">Categoría</Label>
              <Select id="categoria" value={values.categoria_id} onChange={(e) => set("categoria_id", e.target.value)}>
                <option value="">Sin categoría</option>
                {categorias.map((c) => (
                  <option key={c.id} value={c.id}>{c.nombre}</option>
                ))}
              </Select>
            </div>
            <div className="col-span-2 space-y-1.5">
              <Label htmlFor="nombre">Nombre *</Label>
              <Input id="nombre" required value={values.nombre} onChange={(e) => set("nombre", e.target.value)} placeholder="Arroz Premium" />
            </div>
            <div className="col-span-2 space-y-1.5">
              <Label htmlFor="descripcion">Descripción</Label>
              <Input id="descripcion" value={values.descripcion} onChange={(e) => set("descripcion", e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="itbis">ITBIS *</Label>
              <Select id="itbis" value={values.itbis_tipo} onChange={(e) => set("itbis_tipo", e.target.value)}>
                <option value="GRAVADO_18">Gravado 18%</option>
                <option value="GRAVADO_16">Gravado 16%</option>
                <option value="EXENTO">Exento</option>
              </Select>
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="unidad">Unidad de medida (DGII)</Label>
              <Input id="unidad" value={values.unidad_medida} onChange={(e) => set("unidad_medida", e.target.value)} placeholder="43 = unidad" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="costo">Costo</Label>
              <Input id="costo" type="number" step="0.01" value={values.costo} onChange={(e) => set("costo", e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="precio">Precio de venta *</Label>
              <Input id="precio" type="number" step="0.01" required value={values.precio_venta} onChange={(e) => set("precio_venta", e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="stock">Stock actual</Label>
              <Input id="stock" type="number" step="0.01" value={values.stock_actual} onChange={(e) => set("stock_actual", e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="stockMin">Stock mínimo (alerta)</Label>
              <Input id="stockMin" type="number" step="0.01" value={values.stock_minimo} onChange={(e) => set("stock_minimo", e.target.value)} />
            </div>
          </div>

          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

          <div className="flex gap-3">
            <Button type="submit" disabled={saving}>{saving ? "Guardando..." : submitLabel}</Button>
            <Button type="button" variant="secondary" onClick={() => router.push("/inventario/productos")}>Cancelar</Button>
          </div>
        </form>
      </CardContent>
    </Card>
  );
}

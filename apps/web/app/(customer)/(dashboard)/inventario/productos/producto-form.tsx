"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import { ImagePlus } from "lucide-react";
import { Button, Card, CardContent, Input, Label, Select } from "@repo/ui";
import { apiFetch, imagenSrc, uploadProductoImagen } from "@/lib/api";

interface Categoria {
  id: string;
  nombre: string;
}

interface Proveedor {
  id: string;
  nombre: string;
}

export interface ProductoFormValues {
  sku: string;
  nombre: string;
  categoria_id: string;
  proveedor_id: string;
  descripcion: string;
  itbis_tipo: string;
  unidad_medida: string;
  costo: string;
  precio_venta: string;
  stock_actual: string;
  stock_minimo: string;
  tipo: "PRODUCTO" | "SERVICIO";
}

const EMPTY: ProductoFormValues = {
  sku: "",
  nombre: "",
  categoria_id: "",
  proveedor_id: "",
  descripcion: "",
  itbis_tipo: "GRAVADO_18",
  unidad_medida: "43",
  costo: "0",
  precio_venta: "0",
  stock_actual: "0",
  stock_minimo: "0",
  tipo: "PRODUCTO",
};

export function ProductoForm({
  initial,
  submitLabel,
  onSubmit,
  onSuccess,
  onCancel,
  bare,
  productoId,
  imagenUrl,
}: {
  initial?: Partial<ProductoFormValues>;
  submitLabel: string;
  onSubmit: (values: ProductoFormValues) => Promise<any>;
  /** If given, called with onSubmit's result instead of navigating to /inventario/productos — for embedding this form outside its own page (e.g. inside a Dialog). */
  onSuccess?: (result: any) => void;
  /** If given, the Cancelar button calls this instead of navigating to /inventario/productos. */
  onCancel?: () => void;
  /** Skip the Card wrapper — for embedding inside a container that already provides one (e.g. a Dialog). */
  bare?: boolean;
  // La foto solo puede subirse una vez el producto existe (el endpoint
  // necesita su id) - por eso es opcional y solo se muestra al editar.
  productoId?: string;
  imagenUrl?: string | null;
}) {
  const router = useRouter();
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [values, setValues] = useState<ProductoFormValues>({ ...EMPTY, ...initial });
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [imagen, setImagen] = useState(imagenUrl || null);
  const [uploadingImagen, setUploadingImagen] = useState(false);
  const [imagenError, setImagenError] = useState("");

  async function handleImagenChange(e: React.ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0];
    if (!file || !productoId) return;
    setUploadingImagen(true);
    setImagenError("");
    try {
      const { imagen_url } = await uploadProductoImagen(productoId, file);
      setImagen(imagen_url);
    } catch (err: any) {
      setImagenError(err.message);
    } finally {
      setUploadingImagen(false);
      e.target.value = "";
    }
  }

  useEffect(() => {
    apiFetch<{ items: Categoria[] }>("/api/categorias?pageSize=1000&activo=true").then((d) => setCategorias(d.items)).catch(() => {});
    apiFetch<{ items: Proveedor[] }>("/api/proveedores?pageSize=1000&activo=true").then((d) => setProveedores(d.items)).catch(() => {});
  }, []);

  function set<K extends keyof ProductoFormValues>(key: K, value: ProductoFormValues[K]) {
    setValues((v) => ({ ...v, [key]: value }));
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSaving(true);
    setError("");
    try {
      const result = await onSubmit(values);
      if (onSuccess) onSuccess(result);
      else router.push("/inventario/productos");
    } catch (e: any) {
      setError(e.message);
    } finally {
      setSaving(false);
    }
  }

  const form = (
        <form onSubmit={handleSubmit} className="space-y-5">
          {productoId && (
            <div className="flex items-center gap-4">
              <div className="h-20 w-20 rounded-md border border-border bg-surface flex items-center justify-center overflow-hidden shrink-0">
                {imagen ? (
                  // eslint-disable-next-line @next/next/no-img-element
                  <img src={imagenSrc(imagen) || undefined} alt="Foto del producto" className="h-full w-full object-cover" />
                ) : (
                  <ImagePlus className="h-6 w-6 text-muted-foreground" />
                )}
              </div>
              <div className="space-y-1">
                <Label htmlFor="imagen">Foto del producto</Label>
                <Input id="imagen" type="file" accept="image/png,image/jpeg,image/webp" onChange={handleImagenChange} disabled={uploadingImagen} className="max-w-xs" />
                {uploadingImagen && <p className="text-xs text-muted-foreground">Subiendo...</p>}
                {imagenError && <p className="text-xs text-destructive">{imagenError}</p>}
              </div>
            </div>
          )}
          <div className="space-y-1.5">
            <Label htmlFor="tipo">Tipo *</Label>
            <div className="flex gap-2 max-w-md">
              <button
                type="button"
                onClick={() => set("tipo", "PRODUCTO")}
                className={`flex-1 rounded-md border px-3 py-2 text-sm font-medium transition-colors ${
                  values.tipo === "PRODUCTO" ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:text-foreground"
                }`}
              >
                Producto
              </button>
              <button
                type="button"
                onClick={() => set("tipo", "SERVICIO")}
                className={`flex-1 rounded-md border px-3 py-2 text-sm font-medium transition-colors ${
                  values.tipo === "SERVICIO" ? "border-primary bg-primary/10 text-primary" : "border-border text-muted-foreground hover:text-foreground"
                }`}
              >
                Servicio
              </button>
            </div>
            {values.tipo === "SERVICIO" && (
              <p className="text-xs text-muted-foreground">Sin precio fijo ni stock — el precio se define al cotizar o facturar, según el trabajo.</p>
            )}
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
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
            <div className="space-y-1.5">
              <Label htmlFor="proveedor">Proveedor</Label>
              <Select id="proveedor" value={values.proveedor_id} onChange={(e) => set("proveedor_id", e.target.value)}>
                <option value="">Sin proveedor</option>
                {proveedores.map((p) => (
                  <option key={p.id} value={p.id}>{p.nombre}</option>
                ))}
              </Select>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-1.5 md:col-span-2">
              <Label htmlFor="nombre">Nombre *</Label>
              <Input id="nombre" required value={values.nombre} onChange={(e) => set("nombre", e.target.value)} placeholder="Arroz Premium" />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="itbis">ITBIS *</Label>
              <Select id="itbis" value={values.itbis_tipo} onChange={(e) => set("itbis_tipo", e.target.value)}>
                <option value="GRAVADO_18">Gravado 18%</option>
                <option value="GRAVADO_16">Gravado 16%</option>
                <option value="EXENTO">Exento</option>
              </Select>
            </div>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-1.5 md:col-span-2">
              <Label htmlFor="descripcion">Descripción</Label>
              <Input id="descripcion" value={values.descripcion} onChange={(e) => set("descripcion", e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="unidad">Unidad de medida (DGII)</Label>
              <Input id="unidad" value={values.unidad_medida} onChange={(e) => set("unidad_medida", e.target.value)} placeholder="43 = unidad" />
            </div>
          </div>

          {values.tipo === "PRODUCTO" && (
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
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
          )}

          {error && <div className="rounded-md border border-destructive/20 bg-destructive/10 text-destructive p-3 text-sm">{error}</div>}

          <div className="flex gap-3">
            <Button type="submit" disabled={saving}>{saving ? "Guardando..." : submitLabel}</Button>
            <Button type="button" variant="secondary" onClick={() => (onCancel ? onCancel() : router.push("/inventario/productos"))}>Cancelar</Button>
          </div>
        </form>
  );

  if (bare) return form;
  return (
    <Card className="max-w-5xl">
      <CardContent className="pt-5">{form}</CardContent>
    </Card>
  );
}

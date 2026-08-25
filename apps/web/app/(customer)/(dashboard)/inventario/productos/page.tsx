"use client";

import { Fragment, Suspense, useEffect, useMemo, useState } from "react";
import Link from "next/link";
import { Plus, Pencil, Trash2, Package, ImageOff } from "lucide-react";
import {
  Badge,
  Button,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  SortableTableHead,
  TableHeader,
  TableRow,
  Pagination,
  ScrollableTableCard,
  QuerySearchInput,
  lookupFilter,
  formatDOP,
  type QueryPrefixDef,
} from "@repo/ui";
import { apiFetch, imagenSrc } from "@/lib/api";
import { useServerTable } from "@/lib/use-server-table";

interface Categoria {
  id: string;
  nombre: string;
}

interface Proveedor {
  id: string;
  nombre: string;
}

interface Producto {
  id: string;
  categoria_id: string | null;
  proveedor_id: string | null;
  sku: string;
  nombre: string;
  unidad_medida: string;
  itbis_tipo: "GRAVADO_18" | "GRAVADO_16" | "EXENTO";
  costo: string;
  precio_venta: string | null;
  stock_actual: string;
  stock_minimo: string;
  activo: boolean;
  imagen_url: string | null;
  tipo: "PRODUCTO" | "SERVICIO";
}

interface ProductosFilters {
  search?: string;
  categoriaId?: string;
  unidadMedida?: string;
  activo?: string;
}

const ITBIS_LABEL: Record<string, string> = { GRAVADO_18: "18%", GRAVADO_16: "16%", EXENTO: "Exento" };
const ITBIS_VARIANT: Record<string, "default" | "secondary" | "warning"> = { GRAVADO_18: "default", GRAVADO_16: "secondary", EXENTO: "warning" };

export default function ProductosPage() {
  return (
    <Suspense fallback={null}>
      <ProductosPageContent />
    </Suspense>
  );
}

function ProductosPageContent() {
  const [categorias, setCategorias] = useState<Categoria[]>([]);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const [actionError, setActionError] = useState("");
  const [eliminandoId, setEliminandoId] = useState<string | null>(null);

  useEffect(() => {
    apiFetch<{ items: Categoria[] }>("/api/categorias?pageSize=1000&activo=true").then((d) => setCategorias(d.items)).catch(() => {});
    apiFetch<{ items: Proveedor[] }>("/api/proveedores?pageSize=1000&activo=true").then((d) => setProveedores(d.items)).catch(() => {});
  }, []);

  const {
    items: productos,
    total,
    totalPages,
    loading,
    error,
    state,
    setPage,
    setPageSize,
    toggleSort,
    setFilters,
    refresh,
  } = useServerTable<Producto, ProductosFilters>({
    path: "/api/productos",
    initialPageSize: 20,
    initialSortBy: "nombre",
    initialSortDir: "asc",
    initialFilters: { activo: "true" },
  });

  const [rawQuery, setRawQuery] = useState(state.filters.search || "");

  // Sistema de búsqueda avanzada por prefijos (ver query-search.ts en
  // @repo/ui) - "category:" resuelve contra las categorías ya cargadas en
  // memoria (misma lista que usa el <Select> de abajo, sin llamada extra al
  // servidor); "status:" es un mapeo directo sin necesidad de búsqueda.
  const queryPrefixes: QueryPrefixDef[] = useMemo(
    () => [
      { prefix: "category", label: "nombre de categoría", apply: lookupFilter(categorias, (c) => c.nombre, (c) => c.id, "categoriaId") },
      {
        prefix: "status",
        label: "activo | inactivo",
        apply: (value) => {
          const v = value.trim().toLowerCase();
          if (["activo", "activa", "si", "sí", "true"].includes(v)) return { activo: "true" };
          if (["inactivo", "inactiva", "no", "false"].includes(v)) return { activo: "false" };
          return null;
        },
      },
    ],
    [categorias]
  );

  const unidades = Array.from(new Set(["UNIDAD", "PAQUETE", "GALON", "LITRO", "PIE", "LIBRA", "SET", "METRO", "SACO", "BOTELLA", "JUEGO", "YARDA", "CARTON", "CAJA", "KIT"]));

  async function handleDelete(id: string) {
    setActionError("");
    try {
      await apiFetch(`/api/productos/${id}`, { method: "DELETE" });
      setEliminandoId(null);
      refresh();
    } catch (e: any) {
      setActionError(e.message);
    }
  }

  const categoriaNombre = (id: string | null) => categorias.find((c) => c.id === id)?.nombre || "—";
  const proveedorNombre = (id: string | null) => proveedores.find((p) => p.id === id)?.nombre || "—";

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold font-serif tracking-tight">Productos</h1>
          <p className="text-sm text-muted-foreground mt-1">Catálogo con ITBIS, SKU, unidad, costo y precio.</p>
        </div>
        <Link href="/inventario/productos/nuevo">
          <Button>
            <Plus className="h-4 w-4" />
            Nuevo producto
          </Button>
        </Link>
      </div>

      <div className="flex flex-wrap gap-3">
        <QuerySearchInput
          value={rawQuery}
          onChange={setRawQuery}
          onParsed={({ text, filters }) => setFilters({ search: text || undefined, ...filters } as Partial<ProductosFilters>)}
          prefixes={queryPrefixes}
          placeholder="Nombre, SKU, category:, status:..."
          className="flex-1 max-w-sm"
        />
        <Select
          value={state.filters.categoriaId || ""}
          onChange={(e) => setFilters({ categoriaId: e.target.value || undefined })}
          className="max-w-[200px]"
        >
          <option value="">Todas las categorías</option>
          {categorias.map((c) => (
            <option key={c.id} value={c.id}>{c.nombre}</option>
          ))}
        </Select>
        <Select
          value={state.filters.unidadMedida || ""}
          onChange={(e) => setFilters({ unidadMedida: e.target.value || undefined })}
          className="max-w-[180px]"
        >
          <option value="">Todas las unidades</option>
          {unidades.map((u) => (
            <option key={u} value={u}>{u}</option>
          ))}
        </Select>
        <Select
          value={state.filters.activo ?? ""}
          onChange={(e) => setFilters({ activo: e.target.value || undefined })}
          className="max-w-[160px]"
        >
          <option value="true">Activos</option>
          <option value="false">Inactivos</option>
          <option value="">Todos</option>
        </Select>
      </div>

      <ScrollableTableCard
        loading={loading}
        error={error || actionError || null}
        isEmpty={productos.length === 0}
        emptyIcon={<Package className="h-6 w-6" />}
        emptyMessage="No hay productos todavía. Crea el primero."
        pagination={
          <Pagination
            page={state.page}
            totalPages={totalPages}
            total={total}
            pageSize={state.pageSize}
            onPageChange={setPage}
            onPageSizeChange={setPageSize}
          />
        }
      >
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-12"></TableHead>
              <SortableTableHead column="sku" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>SKU</SortableTableHead>
              <SortableTableHead column="nombre" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort}>Nombre</SortableTableHead>
              <TableHead>Categoría</TableHead>
              <TableHead>Proveedor</TableHead>
              <TableHead>Unidad</TableHead>
              <TableHead>ITBIS</TableHead>
              <TableHead className="text-right">Costo</TableHead>
              <SortableTableHead column="precio_venta" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Precio</SortableTableHead>
              <SortableTableHead column="stock_actual" activeSort={state.sortBy} sortDir={state.sortDir} onSort={toggleSort} className="text-right">Stock</SortableTableHead>
              <TableHead className="w-24 text-right">Acciones</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {productos.map((p) => (
              <Fragment key={p.id}>
                <TableRow>
                  <TableCell>
                    <div className="h-9 w-9 rounded-md border border-border bg-surface flex items-center justify-center overflow-hidden">
                      {p.imagen_url ? (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img src={imagenSrc(p.imagen_url) || undefined} alt={p.nombre} loading="lazy" className="h-full w-full object-cover" />
                      ) : (
                        <ImageOff className="h-4 w-4 text-muted-foreground" />
                      )}
                    </div>
                  </TableCell>
                  <TableCell className="font-mono text-xs text-muted-foreground">{p.sku}</TableCell>
                  <TableCell className="font-medium">
                    {p.nombre}
                    {p.tipo === "SERVICIO" && <Badge variant="secondary" className="ml-2">Servicio</Badge>}
                  </TableCell>
                  <TableCell className="text-muted-foreground">{categoriaNombre(p.categoria_id)}</TableCell>
                  <TableCell className="text-muted-foreground">{proveedorNombre(p.proveedor_id)}</TableCell>
                  <TableCell className="text-muted-foreground">{p.unidad_medida}</TableCell>
                  <TableCell>
                    <Badge variant={ITBIS_VARIANT[p.itbis_tipo]}>{ITBIS_LABEL[p.itbis_tipo]}</Badge>
                  </TableCell>
                  <TableCell className="text-right">{formatDOP(p.costo)}</TableCell>
                  <TableCell className="text-right font-medium">{p.tipo === "SERVICIO" ? "—" : formatDOP(p.precio_venta || "0")}</TableCell>
                  <TableCell className="text-right">
                    {p.tipo === "SERVICIO" ? (
                      "—"
                    ) : (
                      <span className={Number(p.stock_actual) <= Number(p.stock_minimo) ? "text-destructive font-medium" : ""}>
                        {p.stock_actual}
                      </span>
                    )}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-1">
                      <Link href={`/inventario/productos/${p.id}` as any}>
                        <Button size="icon" variant="ghost">
                          <Pencil className="h-4 w-4" />
                        </Button>
                      </Link>
                      <Button size="icon" variant="ghost" onClick={() => setEliminandoId(p.id)}>
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
                {eliminandoId === p.id && (
                  <TableRow>
                    <TableCell colSpan={11} className="bg-muted/30">
                      <div className="flex flex-wrap items-center gap-2 py-1">
                        <span className="text-sm">¿Desactivar {p.nombre}?</span>
                        <Button size="sm" variant="destructive" onClick={() => handleDelete(p.id)}>Desactivar</Button>
                        <Button size="sm" variant="ghost" onClick={() => setEliminandoId(null)}>Cancelar</Button>
                      </div>
                    </TableCell>
                  </TableRow>
                )}
              </Fragment>
            ))}
          </TableBody>
        </Table>
      </ScrollableTableCard>
    </div>
  );
}

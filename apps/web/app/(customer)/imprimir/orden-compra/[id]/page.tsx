"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { useEmpresa } from "../../_components/hooks";
import { BarraAcciones, BarraSeccion, CampoDato, EncabezadoDocumento, EstilosImpresion, PieDocumento } from "../../_components/DocumentoBase";

interface OrdenCompraItem {
  id: string;
  sku: string;
  nombre: string;
  proveedor_id: string | null;
  cantidad_solicitada: string;
  cantidad_recibida: string;
  costo_unitario: string;
}

interface OrdenCompraDetalle {
  id: string;
  estado: string;
  subtotal: string;
  total: string;
  fecha: string;
  fecha_esperada: string | null;
  notas: string | null;
  items: OrdenCompraItem[];
}

interface Proveedor {
  id: string;
  nombre: string;
}

const ESTADO_LABEL: Record<string, string> = {
  BORRADOR: "Borrador",
  ENVIADA: "Enviada",
  RECIBIDA_PARCIAL: "Recibida parcial",
  RECIBIDA: "Recibida",
  CANCELADA: "Cancelada",
};

export default function ImprimirOrdenCompraPage() {
  const params = useParams<{ id: string }>();
  const [orden, setOrden] = useState<OrdenCompraDetalle | null>(null);
  const [proveedores, setProveedores] = useState<Proveedor[]>([]);
  const empresa = useEmpresa();

  useEffect(() => {
    apiFetch<OrdenCompraDetalle>(`/api/ordenes-compra/${params.id}`).then(setOrden).catch(() => {});
    apiFetch<{ items: Proveedor[] }>("/api/proveedores?pageSize=1000&activo=true").then((d) => setProveedores(d.items)).catch(() => {});
  }, [params.id]);

  if (!orden) return <p className="p-6 text-sm text-gray-500">Cargando...</p>;

  const fecha = new Date(orden.fecha).toLocaleDateString("es-DO");
  const noDocumento = `OC-${orden.id.slice(0, 8).toUpperCase()}`;
  const proveedoresDistintos = new Set(orden.items.map((it) => it.proveedor_id).filter((id): id is string => !!id));
  const proveedorResumen =
    proveedoresDistintos.size === 0 ? "Sin proveedor especificado" :
    proveedoresDistintos.size === 1 ? proveedores.find((p) => p.id === [...proveedoresDistintos][0])?.nombre || "—" :
    "Varios proveedores (ver detalle por línea)";

  const camposDerecha = [
    { label: "No. Orden", value: noDocumento },
    { label: "Fecha", value: fecha },
    { label: "Esperada", value: orden.fecha_esperada ? new Date(orden.fecha_esperada).toLocaleDateString("es-DO") : "N/A" },
    { label: "Estado", value: ESTADO_LABEL[orden.estado] || orden.estado },
  ];

  return (
    <>
      <EstilosImpresion />
      <BarraAcciones volverHref={`/ordenes-compra/${orden.id}`} />
      <div className="hoja">
        <EncabezadoDocumento empresa={empresa} tipo="ORDEN_COMPRA" titulo="ORDEN DE COMPRA" camposDerecha={camposDerecha} />

        <BarraSeccion tipo="ORDEN_COMPRA">PROVEEDOR</BarraSeccion>
        <div className="py-2">
          <CampoDato label="Proveedor(es)" value={proveedorResumen} />
        </div>

        <table className="w-full mt-3 text-[11.5px] border-collapse">
          <thead>
            <tr className="text-white" style={{ backgroundColor: "#2e6b47" }}>
              <th className="text-left font-bold py-1.5 px-2">Producto</th>
              <th className="text-left font-bold py-1.5 px-2">Proveedor</th>
              <th className="text-right font-bold py-1.5 px-2 w-24">Solicitado</th>
              <th className="text-right font-bold py-1.5 px-2 w-24">Recibido</th>
              <th className="text-right font-bold py-1.5 px-2 w-28">Costo c/u</th>
              <th className="text-right font-bold py-1.5 px-2 w-28">Subtotal</th>
            </tr>
          </thead>
          <tbody>
            {orden.items.map((it) => (
              <tr key={it.id} className="border-b border-gray-200">
                <td className="py-1.5 px-2">{it.nombre}</td>
                <td className="py-1.5 px-2 text-gray-600">{proveedores.find((p) => p.id === it.proveedor_id)?.nombre || "—"}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">{it.cantidad_solicitada}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">{it.cantidad_recibida}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">{Number(it.costo_unitario).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">
                  {(Number(it.costo_unitario) * Number(it.cantidad_solicitada)).toLocaleString("es-DO", { minimumFractionDigits: 2 })}
                </td>
              </tr>
            ))}
          </tbody>
        </table>

        <div className="flex justify-end mt-3">
          <div className="w-56 text-[11.5px] space-y-1">
            <div className="flex justify-between"><span>Subtotal estimado:</span><span className="tabular-nums">RD$ {Number(orden.subtotal).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</span></div>
            <div className="flex justify-between font-bold text-[13px] pt-1 border-t border-gray-300"><span>TOTAL estimado:</span><span className="tabular-nums">RD$ {Number(orden.total).toLocaleString("es-DO", { minimumFractionDigits: 2 })}</span></div>
          </div>
        </div>

        {orden.notas && (
          <>
            <BarraSeccion tipo="ORDEN_COMPRA">NOTAS</BarraSeccion>
            <div className="min-h-[40px] bg-[#eaf6fc] mb-3 p-2 text-[11.5px]">{orden.notas}</div>
          </>
        )}

        <PieDocumento>Este documento no tiene valor fiscal. El ITBIS y costo final se fijan al recibir la mercancía.</PieDocumento>
      </div>
    </>
  );
}

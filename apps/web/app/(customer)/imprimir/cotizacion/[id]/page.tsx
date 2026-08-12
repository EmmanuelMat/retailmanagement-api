"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useCliente, useEmpresa } from "../../_components/hooks";
import { BarraAcciones, BarraSeccion, CampoDato, EncabezadoDocumento, EstilosImpresion, PieDocumento } from "../../_components/DocumentoBase";

interface CotizacionItem {
  id: string;
  nombre: string;
  cantidad: string;
  precio_unitario: string;
  subtotal: string;
}

interface CotizacionDetalle {
  id: string;
  cliente_id: string | null;
  subtotal: string;
  itbis_total: string;
  total: string;
  fecha_vencimiento: string | null;
  created_at: string;
  items: CotizacionItem[];
}

export default function ImprimirCotizacionPage() {
  const params = useParams<{ id: string }>();
  const [cotizacion, setCotizacion] = useState<CotizacionDetalle | null>(null);
  const empresa = useEmpresa();
  const cliente = useCliente(cotizacion?.cliente_id);

  useEffect(() => {
    apiFetch<CotizacionDetalle>(`/api/cotizaciones/${params.id}`).then(setCotizacion).catch(() => {});
  }, [params.id]);

  if (!cotizacion) return <p className="p-6 text-sm text-gray-500">Cargando...</p>;

  const fecha = new Date(cotizacion.created_at).toLocaleDateString("es-DO");
  const noCotizacion = `COT-${cotizacion.id.slice(0, 8).toUpperCase()}`;

  return (
    <>
      <EstilosImpresion />
      <BarraAcciones volverHref={`/cotizaciones/${cotizacion.id}`} />
      <div className="hoja">
        <EncabezadoDocumento
          empresa={empresa}
          tipo="COTIZACION"
          titulo="COTIZACIÓN"
          camposDerecha={[
            { label: "No. Cotización", value: noCotizacion },
            { label: "Fecha", value: fecha },
            { label: "Válida hasta", value: cotizacion.fecha_vencimiento ? new Date(cotizacion.fecha_vencimiento).toLocaleDateString("es-DO") : "" },
          ]}
        />

        <BarraSeccion tipo="COTIZACION">DATOS DEL CLIENTE</BarraSeccion>
        <div className="grid grid-cols-2 gap-x-6 py-2">
          <CampoDato label="Nombre / Razón Social" value={cliente?.nombre || "Consumidor Final"} />
          <CampoDato label="RNC / Cédula" value={cliente?.rnc_cedula || ""} />
          <CampoDato label="Dirección" value={cliente?.direccion || ""} />
          <CampoDato label="Teléfono" value={cliente?.telefono || ""} />
        </div>

        <table className="w-full mt-3 text-[11.5px] border-collapse">
          <thead>
            <tr className="text-white" style={{ backgroundColor: "#2f5fa8" }}>
              <th className="text-left font-bold py-1.5 px-2 w-16">Cant.</th>
              <th className="text-left font-bold py-1.5 px-2">Descripción</th>
              <th className="text-right font-bold py-1.5 px-2 w-28">Precio Unit.</th>
              <th className="text-right font-bold py-1.5 px-2 w-32">Importe (RD$)</th>
            </tr>
          </thead>
          <tbody>
            {cotizacion.items.map((it) => (
              <tr key={it.id} className="border-b border-gray-200">
                <td className="py-1.5 px-2 tabular-nums">{it.cantidad}</td>
                <td className="py-1.5 px-2">{it.nombre}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">{formatDOP(it.precio_unitario)}</td>
                <td className="py-1.5 px-2 text-right tabular-nums">{formatDOP(it.subtotal)}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <div className="flex justify-end mt-3">
          <div className="w-64 text-[11.5px] space-y-1">
            <div className="flex justify-between"><span>Subtotal (antes de ITBIS):</span><span className="tabular-nums">{formatDOP(cotizacion.subtotal)}</span></div>
            <div className="flex justify-between"><span>Total ITBIS estimado:</span><span className="tabular-nums">{formatDOP(cotizacion.itbis_total)}</span></div>
            <div className="flex justify-between font-bold text-[13px] px-2 py-1.5" style={{ backgroundColor: "#c9a227", color: "#1a1a1a" }}>
              <span>TOTAL ESTIMADO (RD$):</span><span className="tabular-nums">{formatDOP(cotizacion.total)}</span>
            </div>
          </div>
        </div>

        <BarraSeccion tipo="COTIZACION">CONDICIONES DE LA COTIZACIÓN</BarraSeccion>
        <div className="min-h-[48px] bg-[#eaf6fc] mb-8" />

        <div className="mt-16 text-center text-[11px] text-gray-600 max-w-xs">
          <div className="border-t border-gray-400 pt-1.5">Firma autorizada — {empresa?.nombre_comercial || empresa?.razon_social}</div>
        </div>

        <PieDocumento>
          Esta cotización es un estimado y no constituye un comprobante fiscal. El ITBIS mostrado es referencial y se formalizará en la factura correspondiente.
        </PieDocumento>
      </div>
    </>
  );
}

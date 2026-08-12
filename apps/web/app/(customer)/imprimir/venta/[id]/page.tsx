"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { formatDOP } from "@repo/ui";
import { apiFetch } from "@/lib/api";
import { useCliente, useEmpresa } from "../../_components/hooks";
import { BarraAcciones, BarraSeccion, CampoDato, EncabezadoDocumento, EstilosImpresion, PieDocumento } from "../../_components/DocumentoBase";

interface VentaItem {
  id: string;
  nombre: string;
  cantidad: string;
  precio_unitario: string;
  subtotal: string;
}

interface VentaDetalle {
  id: string;
  cliente_id: string | null;
  subtotal: string;
  itbis_total: string;
  total: string;
  metodo_pago: string;
  e_ncf: string | null;
  created_at: string;
  items: VentaItem[];
}

export default function ImprimirFacturaPage() {
  const params = useParams<{ id: string }>();
  const [venta, setVenta] = useState<VentaDetalle | null>(null);
  const empresa = useEmpresa();
  const cliente = useCliente(venta?.cliente_id);

  useEffect(() => {
    apiFetch<VentaDetalle>(`/api/ventas/${params.id}`).then(setVenta).catch(() => {});
  }, [params.id]);

  if (!venta) return <p className="p-6 text-sm text-gray-500">Cargando...</p>;

  const fecha = new Date(venta.created_at).toLocaleDateString("es-DO");

  return (
    <>
      <EstilosImpresion />
      <BarraAcciones volverHref={`/ventas/${venta.id}`} />
      <div className="hoja">
        <EncabezadoDocumento
          empresa={empresa}
          tipo="FACTURA"
          titulo="FACTURA"
          camposDerecha={[
            { label: "NCF", value: venta.e_ncf || "Sin emitir" },
            { label: "Fecha Emisión", value: fecha },
          ]}
        />

        <BarraSeccion tipo="FACTURA">DATOS DEL CLIENTE</BarraSeccion>
        <div className="grid grid-cols-2 gap-x-6 py-2">
          <CampoDato label="Nombre / Razón Social" value={cliente?.nombre || "Consumidor Final"} />
          <CampoDato label="RNC / Cédula" value={cliente?.rnc_cedula || ""} />
          <CampoDato label="Dirección" value={cliente?.direccion || ""} />
          <CampoDato label="Teléfono" value={cliente?.telefono || ""} />
          <CampoDato label="Condición de Pago" value={venta.metodo_pago} />
        </div>

        <table className="w-full mt-3 text-[11.5px] border-collapse">
          <thead>
            <tr className="text-white" style={{ backgroundColor: "#1e3a5f" }}>
              <th className="text-left font-bold py-1.5 px-2 w-16">Cant.</th>
              <th className="text-left font-bold py-1.5 px-2">Descripción</th>
              <th className="text-right font-bold py-1.5 px-2 w-28">Precio Unit.</th>
              <th className="text-right font-bold py-1.5 px-2 w-32">Importe (RD$)</th>
            </tr>
          </thead>
          <tbody>
            {venta.items.map((it) => (
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
            <div className="flex justify-between"><span>Subtotal (antes de ITBIS):</span><span className="tabular-nums">{formatDOP(venta.subtotal)}</span></div>
            <div className="flex justify-between"><span>Total ITBIS (18%):</span><span className="tabular-nums">{formatDOP(venta.itbis_total)}</span></div>
            <div className="flex justify-between font-bold text-[13px] px-2 py-1.5" style={{ backgroundColor: "#c9a227", color: "#1a1a1a" }}>
              <span>TOTAL A PAGAR (RD$):</span><span className="tabular-nums">{formatDOP(venta.total)}</span>
            </div>
          </div>
        </div>

        <BarraSeccion tipo="FACTURA">CONDICIONES Y NOTAS</BarraSeccion>
        <div className="min-h-[48px] bg-[#eaf6fc] mb-8" />

        <div className="grid grid-cols-2 gap-12 mt-16 text-center text-[11px] text-gray-600">
          <div className="border-t border-gray-400 pt-1.5">Autorizado por</div>
          <div className="border-t border-gray-400 pt-1.5">Recibido conforme (Cliente)</div>
        </div>

        <PieDocumento>
          Este comprobante solo tiene validez fiscal si el NCF está vigente y registrado ante la DGII. Validar en dgii.gov.do.
        </PieDocumento>
      </div>
    </>
  );
}

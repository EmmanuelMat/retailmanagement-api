"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { useCliente, useEmpresa } from "../../_components/hooks";
import { BarraAcciones, BarraSeccion, CampoDato, EncabezadoDocumento, EstilosImpresion, PieDocumento } from "../../_components/DocumentoBase";

interface ConduceItem {
  id: string;
  sku: string;
  nombre: string;
  cantidad: string;
  unidad: string | null;
  observaciones: string | null;
}

interface ConduceDetalle {
  id: string;
  venta_id: string;
  direccion_entrega: string | null;
  orden_compra: string | null;
  vehiculo_placa: string | null;
  conductor: string | null;
  notas: string | null;
  entregado_por: string | null;
  recibido_por: string | null;
  created_at: string;
  items: ConduceItem[];
}

interface VentaResumen {
  id: string;
  cliente_id: string | null;
  e_ncf: string | null;
}

export default function ImprimirConducePage() {
  const params = useParams<{ id: string }>();
  const [conduce, setConduce] = useState<ConduceDetalle | null>(null);
  const [venta, setVenta] = useState<VentaResumen | null>(null);
  const empresa = useEmpresa();
  const cliente = useCliente(venta?.cliente_id);

  useEffect(() => {
    apiFetch<ConduceDetalle>(`/api/conduces/${params.id}`).then(setConduce).catch(() => {});
  }, [params.id]);

  useEffect(() => {
    if (!conduce?.venta_id) return;
    apiFetch<VentaResumen>(`/api/ventas/${conduce.venta_id}`).then(setVenta).catch(() => {});
  }, [conduce?.venta_id]);

  if (!conduce) return <p className="p-6 text-sm text-gray-500">Cargando...</p>;

  const fecha = new Date(conduce.created_at).toLocaleDateString("es-DO");
  const noConduce = `CND-${conduce.id.slice(0, 8).toUpperCase()}`;

  return (
    <>
      <EstilosImpresion />
      <BarraAcciones volverHref={`/conduces/${conduce.id}`} />
      <div className="hoja">
        <EncabezadoDocumento
          empresa={empresa}
          tipo="CONDUCE"
          titulo="CONDUCE"
          camposDerecha={[
            { label: "No. Conduce", value: noConduce },
            { label: "Fecha", value: fecha },
            { label: "Factura Relacionada", value: venta?.e_ncf || (venta ? venta.id.slice(0, 8).toUpperCase() : "") },
            { label: "Orden de Compra", value: conduce.orden_compra || "N/A" },
          ]}
        />

        <BarraSeccion tipo="CONDUCE">ENTREGA A</BarraSeccion>
        <div className="grid grid-cols-2 gap-x-6 py-2">
          <CampoDato label="Nombre / Razón Social" value={cliente?.nombre || "Consumidor Final"} />
          <CampoDato label="RNC / Cédula" value={cliente?.rnc_cedula || ""} />
          <CampoDato label="Dirección de entrega" value={conduce.direccion_entrega || cliente?.direccion || ""} />
          <CampoDato label="Teléfono" value={cliente?.telefono || ""} />
          <CampoDato label="Vehículo / Placa" value={conduce.vehiculo_placa || ""} />
          <CampoDato label="Conductor" value={conduce.conductor || ""} />
        </div>

        <table className="w-full mt-3 text-[11.5px] border-collapse">
          <thead>
            <tr className="text-white" style={{ backgroundColor: "#4a4a4a" }}>
              <th className="text-left font-bold py-1.5 px-2 w-16">Cant.</th>
              <th className="text-left font-bold py-1.5 px-2 w-24">Unidad</th>
              <th className="text-left font-bold py-1.5 px-2">Descripción del artículo</th>
              <th className="text-left font-bold py-1.5 px-2 w-40">Observaciones</th>
            </tr>
          </thead>
          <tbody>
            {conduce.items.map((it) => (
              <tr key={it.id} className="border-b border-gray-200">
                <td className="py-1.5 px-2 tabular-nums">{it.cantidad}</td>
                <td className="py-1.5 px-2">{it.unidad || "—"}</td>
                <td className="py-1.5 px-2">{it.nombre}</td>
                <td className="py-1.5 px-2">{it.observaciones || ""}</td>
              </tr>
            ))}
          </tbody>
        </table>

        <BarraSeccion tipo="CONDUCE">NOTAS DE ENTREGA</BarraSeccion>
        <div className="min-h-[40px] bg-[#eaf6fc] mb-3 p-2 text-[11.5px]">{conduce.notas || ""}</div>

        <div className="grid grid-cols-2 gap-x-6 py-2">
          <CampoDato label="Entregado por" value={conduce.entregado_por || ""} />
          <CampoDato label="Recibido por" value={conduce.recibido_por || ""} />
        </div>

        <div className="grid grid-cols-2 gap-12 mt-14 text-center text-[11px] text-gray-600">
          <div className="border-t border-gray-400 pt-1.5">Firma de quien entrega</div>
          <div className="border-t border-gray-400 pt-1.5">Firma de quien recibe</div>
        </div>

        <PieDocumento>
          Este conduce es un comprobante de entrega/transporte de mercancía y NO tiene valor fiscal ni sustituye la factura con NCF correspondiente.
        </PieDocumento>
      </div>
    </>
  );
}

"use client";

import { useEffect, useState } from "react";
import { useParams } from "next/navigation";
import { apiFetch } from "@/lib/api";
import { useCliente, useEmpresa } from "../../_components/hooks";
import { BarraAcciones, BarraSeccion, CampoDato, EncabezadoDocumento, EstilosImpresion, PieDocumento } from "../../_components/DocumentoBase";

interface OrdenDetalle {
  id: string;
  cliente_id: string | null;
  condicion_id: string | null;
  estado: string;
  prioridad: string;
  fecha_programada: string | null;
  direccion: string | null;
  descripcion: string | null;
  notas: string | null;
  created_at: string;
}

interface Condicion {
  id: string;
  nombre: string;
}

export default function ImprimirOrdenServicioPage() {
  const params = useParams<{ id: string }>();
  const [orden, setOrden] = useState<OrdenDetalle | null>(null);
  const [condiciones, setCondiciones] = useState<Condicion[]>([]);
  const empresa = useEmpresa();
  const cliente = useCliente(orden?.cliente_id);

  useEffect(() => {
    apiFetch<OrdenDetalle>(`/api/ordenes-servicio/${params.id}`).then(setOrden).catch(() => {});
    apiFetch<Condicion[]>("/api/condiciones-orden").then(setCondiciones).catch(() => {});
  }, [params.id]);

  if (!orden) return <p className="p-6 text-sm text-gray-500">Cargando...</p>;

  const fecha = new Date(orden.created_at).toLocaleDateString("es-DO");
  const noDocumento = `OS-${orden.id.slice(0, 8).toUpperCase()}`;
  const condicionNombre = condiciones.find((c) => c.id === orden.condicion_id)?.nombre;

  const camposDerecha = [
    { label: "No. Orden", value: noDocumento },
    { label: "Fecha", value: fecha },
    { label: "Condición", value: condicionNombre || "N/A" },
    { label: "Prioridad", value: orden.prioridad },
    { label: "Programada", value: orden.fecha_programada ? new Date(orden.fecha_programada).toLocaleDateString("es-DO") : "N/A" },
  ];

  return (
    <>
      <EstilosImpresion />
      <BarraAcciones volverHref={`/ordenes-servicio/${orden.id}`} />
      <div className="hoja">
        <EncabezadoDocumento empresa={empresa} tipo="ORDEN_SERVICIO" titulo="ORDEN DE SERVICIO" camposDerecha={camposDerecha} />

        <BarraSeccion tipo="ORDEN_SERVICIO">CLIENTE</BarraSeccion>
        <div className="grid grid-cols-2 gap-x-6 py-2">
          <CampoDato label="Nombre / Razón Social" value={cliente?.nombre || "Consumidor Final"} />
          <CampoDato label="RNC / Cédula" value={cliente?.rnc_cedula || ""} />
          <CampoDato label="Dirección" value={orden.direccion || cliente?.direccion || ""} />
          <CampoDato label="Teléfono" value={cliente?.telefono || ""} />
        </div>

        {orden.descripcion && (
          <>
            <BarraSeccion tipo="ORDEN_SERVICIO">DESCRIPCIÓN DEL TRABAJO</BarraSeccion>
            <div className="min-h-[40px] bg-[#eaf6fc] mb-3 p-2 text-[11.5px]">{orden.descripcion}</div>
          </>
        )}

        <div className="grid grid-cols-2 gap-12 mt-14 text-center text-[11px] text-gray-600">
          <div className="border-t border-gray-400 pt-1.5">Firma del técnico</div>
          <div className="border-t border-gray-400 pt-1.5">Firma del cliente</div>
        </div>

        <PieDocumento>Este documento no tiene valor fiscal ni sustituye la factura con NCF correspondiente.</PieDocumento>
      </div>
    </>
  );
}

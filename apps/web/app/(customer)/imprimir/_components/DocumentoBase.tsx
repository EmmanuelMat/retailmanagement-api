"use client";

import { Printer, ArrowLeft } from "lucide-react";
import Link from "next/link";
import type { Empresa } from "./tipos";

/** Reglas de impresión compartidas por Factura/Cotización/Conduce - tamaño
 * carta, sin la barra de herramientas de pantalla, sin fondo oscuro del
 * resto de la app (estos documentos son del negocio del tenant, no de la
 * app). "Descargar PDF" es el diálogo nativo del navegador (Guardar como
 * PDF) - no hay librería de PDF en el proyecto y esto produce un PDF real
 * con texto seleccionable, no una captura de pantalla.
 */
export function EstilosImpresion() {
  return (
    <style>{`
      @page { size: letter; margin: 1.4cm; }
      html, body { background: #f3f4f6; }
      @media print {
        html, body { background: #fff; }
        .no-print { display: none !important; }
        .hoja { box-shadow: none !important; margin: 0 !important; border: none !important; }
      }
      .hoja {
        background: #fff;
        color: #1a1a1a;
        max-width: 850px;
        margin: 24px auto;
        padding: 40px 48px;
        box-shadow: 0 1px 3px rgba(0,0,0,0.15), 0 8px 24px rgba(0,0,0,0.08);
        font-family: Georgia, 'Times New Roman', serif;
        font-size: 13px;
        line-height: 1.4;
      }
    `}</style>
  );
}

export function BarraAcciones({ volverHref }: { volverHref: string }) {
  return (
    <div className="no-print max-w-[850px] mx-auto mt-4 px-1 flex items-center justify-between">
      <Link href={volverHref as any} className="inline-flex items-center gap-1.5 text-sm text-gray-600 hover:text-gray-900">
        <ArrowLeft className="h-4 w-4" />Volver
      </Link>
      <button
        onClick={() => window.print()}
        className="inline-flex items-center gap-1.5 rounded-md bg-gray-900 text-white text-sm font-medium px-4 py-2 hover:bg-gray-700"
      >
        <Printer className="h-4 w-4" />Imprimir / Descargar PDF
      </button>
    </div>
  );
}

const COLORES: Record<"FACTURA" | "COTIZACION" | "CONDUCE" | "ORDEN_SERVICIO" | "ORDEN_COMPRA", string> = {
  FACTURA: "#1e3a5f",
  COTIZACION: "#2f5fa8",
  CONDUCE: "#4a4a4a",
  ORDEN_SERVICIO: "#8a5a1f",
  ORDEN_COMPRA: "#2e6b47",
};

/** Encabezado compartido: logo + datos del emisor a la izquierda, franja de
 * color con el tipo de documento a la derecha. El logo viene de Mi negocio
 * (logo_url) - a diferencia del resto de la app, este documento es del
 * negocio del tenant y nunca lleva atribución a Colmado POS.
 */
export function EncabezadoDocumento({
  empresa,
  tipo,
  titulo,
  camposDerecha,
}: {
  empresa: Empresa | null;
  tipo: "FACTURA" | "COTIZACION" | "CONDUCE" | "ORDEN_SERVICIO" | "ORDEN_COMPRA";
  titulo: string;
  camposDerecha: { label: string; value: string }[];
}) {
  const color = COLORES[tipo];
  return (
    <div className="flex items-start justify-between gap-6 pb-4 mb-2">
      <div className="flex items-start gap-3 min-w-0">
        {empresa?.logo_url && (
          // eslint-disable-next-line @next/next/no-img-element
          <img src={empresa.logo_url} alt="" className="h-16 w-16 rounded object-cover shrink-0 border border-gray-200" />
        )}
        <div className="min-w-0">
          <h1 className="text-2xl font-bold leading-tight">{empresa?.nombre_comercial || empresa?.razon_social || ""}</h1>
          <div className="text-[11.5px] text-gray-700 mt-1 space-y-0.5">
            {empresa?.rnc && <p>RNC: {empresa.rnc}</p>}
            {empresa?.direccion && <p>Dirección: {empresa.direccion}</p>}
            {empresa?.telefono && <p>Teléfono: {empresa.telefono}</p>}
            {empresa?.correo && <p>Email: {empresa.correo}</p>}
          </div>
        </div>
      </div>
      <div className="shrink-0 text-right">
        <div className="text-white font-bold text-lg px-5 py-2 rounded-sm" style={{ backgroundColor: color }}>
          {titulo}
        </div>
        <div className="mt-2 text-[11.5px] space-y-1">
          {camposDerecha.map((c) => (
            <div key={c.label} className="flex justify-end gap-2">
              <span className="font-semibold text-gray-700">{c.label}:</span>
              <span className="bg-[#eaf6fc] px-2 rounded-sm min-w-[110px] text-left">{c.value || "—"}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export function BarraSeccion({ tipo, children }: { tipo: "FACTURA" | "COTIZACION" | "CONDUCE" | "ORDEN_SERVICIO" | "ORDEN_COMPRA"; children: React.ReactNode }) {
  return (
    <div className="text-white text-[12px] font-bold px-3 py-1.5 mt-3" style={{ backgroundColor: COLORES[tipo] }}>
      {children}
    </div>
  );
}

export function CampoDato({ label, value }: { label: string; value: string }) {
  return (
    <div className="flex items-baseline gap-2 py-0.5">
      <span className="font-semibold text-gray-700 text-[11.5px] shrink-0">{label}:</span>
      <span className="bg-[#eaf6fc] px-2 rounded-sm text-[11.5px] flex-1">{value || "—"}</span>
    </div>
  );
}

export function PieDocumento({ children }: { children: React.ReactNode }) {
  return <p className="text-[10.5px] text-gray-500 italic text-center mt-8 pt-3 border-t border-gray-200">{children}</p>;
}

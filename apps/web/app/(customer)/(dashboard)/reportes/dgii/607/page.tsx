import { ReporteTxt } from "../reporte-txt";

export default function Reporte607Page() {
  return (
    <ReporteTxt
      titulo="Reporte 607 · Ventas"
      descripcion="TXT de ventas no electrónicas del período (solo aplica si e-CF está desactivado; las ventas con e-CF ya se reportan directo a la DGII)."
      endpoint="/api/reports/607"
      filePrefix="607_"
    />
  );
}

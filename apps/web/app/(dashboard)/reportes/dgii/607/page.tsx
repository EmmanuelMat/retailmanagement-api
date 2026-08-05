import { ReporteTxt } from "../reporte-txt";

export default function Reporte607Page() {
  return (
    <ReporteTxt
      titulo="Reporte 607 · Ventas"
      descripcion="TXT de ventas con e-NCF emitido del período."
      endpoint="/api/reports/607"
      filePrefix="607_"
    />
  );
}

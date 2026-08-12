import { ReporteTxt } from "../reporte-txt";

export default function Reporte606Page() {
  return (
    <ReporteTxt
      titulo="Reporte 606 · Compras"
      descripcion="TXT de compras del período para la Norma 07-2018."
      endpoint="/api/reports/606"
      filePrefix="606_"
    />
  );
}

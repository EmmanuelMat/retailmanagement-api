"use client";

import { useMemo, useState } from "react";
import { Card, CardContent, Input, Label, formatDOP } from "@repo/ui";

// Horas laborales mensuales = 8 × 23.83 (días laborables promedio/mes,
// estándar DR) — constante en todo el cálculo, no depende del empleado.
const HORAS_LABORALES_MENSUALES = 8 * 23.83;

const SFS_RATE = 0.0304; // Seguro Familiar de Salud, empleado
const AFP_RATE = 0.0287; // Fondo de pensiones, empleado
const SFS_EMPLEADOR_RATE = 0.0709;
const AFP_EMPLEADOR_RATE = 0.071;
const RIESGOS_LABORALES_RATE = 0.013;
const INFOTEP_RATE = 0.01;

// Tramos ISR anuales (DGII) — cada fórmula ya incorpora el impuesto de los
// tramos inferiores vía su constante (+31,232 / +79,776), así que es
// "tramo más alto alcanzado", no una suma de tramos.
function isrAnual(salarioAnual: number): number {
  if (salarioAnual > 867123) return (salarioAnual - 867123) * 0.25 + 79776;
  if (salarioAnual > 624329) return (salarioAnual - 624329) * 0.2 + 31232;
  if (salarioAnual > 416220) return (salarioAnual - 416220) * 0.15;
  return 0;
}

function fila(label: string, valor: number, bold = false) {
  return (
    <div className={`flex justify-between text-sm ${bold ? "font-bold text-base" : "text-muted-foreground"}`}>
      <span>{label}</span>
      <span className="tabular-nums">{formatDOP(valor)}</span>
    </div>
  );
}

/**
 * Calculadora de salario neto adaptada de un ejercicio de clase de
 * programación (fórmulas exactas de SFS/AFP/ISR/aportes patronales para
 * República Dominicana). Puramente en el cliente, sin persistencia — cada
 * cálculo es un "qué pasaría si" sobre el salario actual del empleado, no
 * un hecho histórico que necesite guardarse (a diferencia de una corrida de
 * nómina real, ver nomina/run). No reemplaza el cálculo de
 * `nomina_service.rs` (que hoy usa un TSS plano del 5.91% e ISR en cero) -
 * es una herramienta de referencia para el empleado/administrador.
 */
export function MandatoCalculadora({ salarioMensual }: { salarioMensual: string }) {
  const [horasExtras, setHorasExtras] = useState("0");
  const [porcentaje, setPorcentaje] = useState("1.35");

  const calculo = useMemo(() => {
    const salarioBruto = Number(salarioMensual) || 0;
    const horas = Number(horasExtras) || 0;
    const pct = Number(porcentaje) || 0;

    const valorHora = salarioBruto / HORAS_LABORALES_MENSUALES;
    const montoHorasExtras = valorHora * pct * horas;

    const sfs = salarioBruto * SFS_RATE;
    const afp = salarioBruto * AFP_RATE;
    const salarioNetoBase = salarioBruto - sfs - afp;
    const salarioAnual = salarioNetoBase * 12;
    const isr = isrAnual(salarioAnual) / 12;
    const salarioNetoFinal = salarioNetoBase - isr + montoHorasExtras;

    const sfsEmpleador = salarioBruto * SFS_EMPLEADOR_RATE;
    const afpEmpleador = salarioBruto * AFP_EMPLEADOR_RATE;
    const riesgosLaborales = salarioBruto * RIESGOS_LABORALES_RATE;
    const infotep = salarioBruto * INFOTEP_RATE;
    const costoTotalEmpleador = salarioBruto + sfsEmpleador + afpEmpleador + riesgosLaborales + infotep;

    return {
      valorHora, montoHorasExtras, sfs, afp, salarioNetoBase, salarioAnual, isr, salarioNetoFinal,
      sfsEmpleador, afpEmpleador, riesgosLaborales, infotep, costoTotalEmpleador,
    };
  }, [salarioMensual, horasExtras, porcentaje]);

  return (
    <div className="space-y-4 max-w-xl">
      <Card>
        <CardContent className="pt-5 space-y-4">
          <div>
            <p className="text-sm font-semibold">Horas extras</p>
            <p className="text-xs text-muted-foreground mt-0.5">
              La ley dominicana paga las primeras 68 horas extras del mes a 1.35x y el resto a 2x — ajusta el
              porcentaje según corresponda a las horas que estás calculando.
            </p>
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-1.5">
              <Label htmlFor="horasExtras">Horas extras trabajadas</Label>
              <Input id="horasExtras" type="number" min={0} step="0.5" value={horasExtras} onChange={(e) => setHorasExtras(e.target.value)} />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="porcentaje">Porcentaje (1.35 o 2.0)</Label>
              <Input id="porcentaje" type="number" min={1} step="0.05" value={porcentaje} onChange={(e) => setPorcentaje(e.target.value)} />
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-5 space-y-1.5">
          <p className="text-sm font-semibold mb-2">Aporte del empleado</p>
          {fila("Salario bruto", Number(salarioMensual) || 0)}
          {fila("Valor por hora", calculo.valorHora)}
          {fila("Monto por horas extras", calculo.montoHorasExtras)}
          {fila("SFS (3.04%)", -calculo.sfs)}
          {fila("AFP (2.87%)", -calculo.afp)}
          {fila("Salario neto base", calculo.salarioNetoBase)}
          {fila("ISR (mensual)", -calculo.isr)}
          <div className="pt-2 mt-1 border-t border-border">{fila("Salario neto final", calculo.salarioNetoFinal, true)}</div>
        </CardContent>
      </Card>

      <Card>
        <CardContent className="pt-5 space-y-1.5">
          <p className="text-sm font-semibold mb-2">Aporte patronal</p>
          {fila("SFS empleador (7.09%)", calculo.sfsEmpleador)}
          {fila("AFP empleador (7.10%)", calculo.afpEmpleador)}
          {fila("Riesgos laborales (1.3%)", calculo.riesgosLaborales)}
          {fila("INFOTEP (1%)", calculo.infotep)}
          <div className="pt-2 mt-1 border-t border-border">{fila("Costo total empleador", calculo.costoTotalEmpleador, true)}</div>
        </CardContent>
      </Card>
    </div>
  );
}

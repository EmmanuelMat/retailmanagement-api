//! Estados financieros - Fase F3 (Estado de Resultados y Balance General).
//!
//! Read models puros sobre `asientos_contables`: no escriben nada y no
//! duplican ninguna regla de negocio. La clasificación sale de
//! `cuentas_contables` (`tipo` para el bloque, `naturaleza` para el signo),
//! unida por el **código** de la cuenta - el primer token del string
//! `"<codigo> <nombre>"` con que se guardan las líneas (mismo criterio que
//! `ContabilidadService::codigo_de_cuenta`, que es lo que valida las
//! escrituras desde F2).
//!
//! Tres decisiones que el lector de un estado necesita saber:
//!
//! 1. **Cuentas sin fila en `cuentas_contables`** (fantasmas creados antes
//!    de F2, o de un plan editado a mano): no se descartan ni se les adivina
//!    el tipo. Van a un bloque propio `sin_clasificar`, con su detalle, para
//!    que un descuadre sea visible y auditable en vez de silencioso.
//! 2. **Costo de ventas vs. gastos operativos**: dentro de `tipo = 'GASTO'`,
//!    los códigos que empiezan en `50` son costo de ventas (hoy solo
//!    `5050 Costo de Ventas`); el resto son gastos operativos. Es el propio
//!    esquema de numeración del plan sembrado.
//! 3. **Período abierto**: no existe asiento de cierre (fuera de alcance),
//!    así que `3200 Resultados del Ejercicio` nunca recibe el resultado y el
//!    balance lo muestra como `resultado_acumulado`: el P&L de *toda* la
//!    historia hasta la fecha de corte. Por eso
//!    `Activo + Sin clasificar = Pasivo + Patrimonio + Resultado acumulado`.
//!
//! Esa identidad no se asume: se calcula y se reporta en `cuadra`. Es
//! consecuencia aritmética de que `create_entry` solo acepta asientos
//! balanceados (`Σ(debe − haber) = 0` sobre todas las líneas), repartida por
//! `tipo`.

use chrono::NaiveDate;
use rust_decimal::Decimal;
use serde::Serialize;
use sqlx::PgPool;

/// Una cuenta con su saldo ya firmado según su `naturaleza`.
#[derive(Debug, Serialize)]
pub struct LineaEstado {
    pub codigo: String,
    pub nombre: String,
    pub monto: Decimal,
}

#[derive(Debug, Serialize)]
pub struct EstadoResultados {
    pub desde: NaiveDate,
    pub hasta: NaiveDate,
    pub ingresos: Decimal,
    pub costo_ventas: Decimal,
    pub utilidad_bruta: Decimal,
    pub gastos: Decimal,
    pub resultado: Decimal,
    /// Neto (debe − haber) de las líneas cuya cuenta no está en el plan.
    /// No entra en ningún subtotal de arriba; se muestra para que el usuario
    /// sepa que hay movimiento sin clasificar.
    pub sin_clasificar: Decimal,
    pub detalle_ingresos: Vec<LineaEstado>,
    pub detalle_costo_ventas: Vec<LineaEstado>,
    pub detalle_gastos: Vec<LineaEstado>,
    pub detalle_sin_clasificar: Vec<LineaEstado>,
}

#[derive(Debug, Serialize)]
pub struct BalanceGeneral {
    pub al: NaiveDate,
    pub activo: Decimal,
    pub pasivo: Decimal,
    pub patrimonio: Decimal,
    /// Resultado del ejercicio todavía sin cerrar contra patrimonio.
    pub resultado_acumulado: Decimal,
    pub sin_clasificar: Decimal,
    pub total_pasivo_y_patrimonio: Decimal,
    /// `activo + sin_clasificar == pasivo + patrimonio + resultado_acumulado`.
    pub cuadra: bool,
    pub detalle_activo: Vec<LineaEstado>,
    pub detalle_pasivo: Vec<LineaEstado>,
    pub detalle_patrimonio: Vec<LineaEstado>,
    pub detalle_sin_clasificar: Vec<LineaEstado>,
}

/// Fila cruda del GROUP BY: cuenta, su clasificación (NULL si no está en el
/// plan) y sus totales de debe/haber en el rango pedido.
struct SaldoCuenta {
    codigo: String,
    nombre: String,
    tipo: Option<String>,
    naturaleza: Option<String>,
    debe: Decimal,
    haber: Decimal,
}

impl SaldoCuenta {
    /// Saldo firmado según la naturaleza de la cuenta: una DEUDORA crece con
    /// el debe, una ACREEDORA con el haber. Sin fila en el plan se asume la
    /// convención deudora (debe − haber), que es lo que mantiene la
    /// identidad del balance exacta.
    fn saldo(&self) -> Decimal {
        match self.naturaleza.as_deref() {
            Some("ACREEDORA") => self.haber - self.debe,
            _ => self.debe - self.haber,
        }
    }

    fn linea(&self) -> LineaEstado {
        LineaEstado { codigo: self.codigo.clone(), nombre: self.nombre.clone(), monto: self.saldo() }
    }

    /// `true` para las cuentas de resultado cuyo código empieza en `50`
    /// (costo de ventas), por el esquema de numeración del plan sembrado.
    fn es_costo_de_ventas(&self) -> bool {
        self.codigo.starts_with("50")
    }
}

pub struct EstadosFinancierosService {
    pool: PgPool,
}

impl EstadosFinancierosService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Saldos por cuenta en `[desde, hasta]`, ya clasificados. El LEFT JOIN
    /// es a propósito: una cuenta sin fila en el plan sale con `tipo = NULL`
    /// y termina en el bloque "sin clasificar" en vez de desaparecer del
    /// estado (lo que haría cuadrar el reporte mintiendo).
    ///
    /// El join **no** filtra por `cuentas_contables.activo`: desactivar una
    /// cuenta no debe reclasificar su historia. `activo` solo gobierna las
    /// escrituras nuevas (ver `ContabilidadService::validar_cuentas`).
    async fn saldos(&self, tenant_id: &str, desde: Option<NaiveDate>, hasta: NaiveDate) -> anyhow::Result<Vec<SaldoCuenta>> {
        let rows: Vec<(String, String, Option<String>, Option<String>, Decimal, Decimal)> = sqlx::query_as(
            r#"SELECT split_part(ac.cuenta, ' ', 1) AS codigo,
                      COALESCE(MAX(cc.nombre), MAX(ac.cuenta)) AS nombre,
                      MAX(cc.tipo) AS tipo,
                      MAX(cc.naturaleza) AS naturaleza,
                      COALESCE(SUM(ac.debe), 0) AS debe,
                      COALESCE(SUM(ac.haber), 0) AS haber
               FROM asientos_contables ac
               LEFT JOIN cuentas_contables cc
                 ON cc.tenant_id = ac.tenant_id AND cc.codigo = split_part(ac.cuenta, ' ', 1)
               WHERE ac.tenant_id = $1
                 AND ($2::date IS NULL OR ac.fecha >= $2)
                 AND ac.fecha <= $3
               GROUP BY split_part(ac.cuenta, ' ', 1)
               ORDER BY 1"#,
        )
        .bind(tenant_id)
        .bind(desde)
        .bind(hasta)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .into_iter()
            .map(|(codigo, nombre, tipo, naturaleza, debe, haber)| SaldoCuenta { codigo, nombre, tipo, naturaleza, debe, haber })
            .collect())
    }

    pub async fn estado_resultados(&self, tenant_id: &str, desde: NaiveDate, hasta: NaiveDate) -> anyhow::Result<EstadoResultados> {
        if desde > hasta {
            anyhow::bail!("La fecha \"desde\" no puede ser posterior a \"hasta\"");
        }
        let saldos = self.saldos(tenant_id, Some(desde), hasta).await?;

        let mut estado = EstadoResultados {
            desde,
            hasta,
            ingresos: Decimal::ZERO,
            costo_ventas: Decimal::ZERO,
            utilidad_bruta: Decimal::ZERO,
            gastos: Decimal::ZERO,
            resultado: Decimal::ZERO,
            sin_clasificar: Decimal::ZERO,
            detalle_ingresos: Vec::new(),
            detalle_costo_ventas: Vec::new(),
            detalle_gastos: Vec::new(),
            detalle_sin_clasificar: Vec::new(),
        };

        for s in &saldos {
            let saldo = s.saldo();
            match s.tipo.as_deref() {
                Some("INGRESO") => {
                    estado.ingresos += saldo;
                    estado.detalle_ingresos.push(s.linea());
                }
                Some("GASTO") if s.es_costo_de_ventas() => {
                    estado.costo_ventas += saldo;
                    estado.detalle_costo_ventas.push(s.linea());
                }
                Some("GASTO") => {
                    estado.gastos += saldo;
                    estado.detalle_gastos.push(s.linea());
                }
                // ACTIVO / PASIVO / PATRIMONIO no son cuentas de resultado.
                Some(_) => {}
                None => {
                    estado.sin_clasificar += saldo;
                    estado.detalle_sin_clasificar.push(s.linea());
                }
            }
        }

        estado.utilidad_bruta = estado.ingresos - estado.costo_ventas;
        estado.resultado = estado.utilidad_bruta - estado.gastos;
        Ok(estado)
    }

    pub async fn balance_general(&self, tenant_id: &str, al: NaiveDate) -> anyhow::Result<BalanceGeneral> {
        // Sin `desde`: un balance es acumulado desde el primer asiento.
        let saldos = self.saldos(tenant_id, None, al).await?;

        let mut balance = BalanceGeneral {
            al,
            activo: Decimal::ZERO,
            pasivo: Decimal::ZERO,
            patrimonio: Decimal::ZERO,
            resultado_acumulado: Decimal::ZERO,
            sin_clasificar: Decimal::ZERO,
            total_pasivo_y_patrimonio: Decimal::ZERO,
            cuadra: false,
            detalle_activo: Vec::new(),
            detalle_pasivo: Vec::new(),
            detalle_patrimonio: Vec::new(),
            detalle_sin_clasificar: Vec::new(),
        };

        let mut ingresos = Decimal::ZERO;
        let mut gastos = Decimal::ZERO;
        for s in &saldos {
            let saldo = s.saldo();
            match s.tipo.as_deref() {
                Some("ACTIVO") => {
                    balance.activo += saldo;
                    balance.detalle_activo.push(s.linea());
                }
                Some("PASIVO") => {
                    balance.pasivo += saldo;
                    balance.detalle_pasivo.push(s.linea());
                }
                Some("PATRIMONIO") => {
                    balance.patrimonio += saldo;
                    balance.detalle_patrimonio.push(s.linea());
                }
                Some("INGRESO") => ingresos += saldo,
                Some("GASTO") => gastos += saldo,
                Some(_) => {}
                None => {
                    balance.sin_clasificar += saldo;
                    balance.detalle_sin_clasificar.push(s.linea());
                }
            }
        }

        balance.resultado_acumulado = ingresos - gastos;
        balance.total_pasivo_y_patrimonio = balance.pasivo + balance.patrimonio + balance.resultado_acumulado;
        balance.cuadra = balance.activo + balance.sin_clasificar == balance.total_pasivo_y_patrimonio;
        Ok(balance)
    }
}

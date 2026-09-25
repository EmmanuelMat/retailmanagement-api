# Phase F — Estados financieros, integridad del mayor y abonos a proveedor

Fecha: 2026-09-21 · Rama: `feat/phase-f-accounting-reports` · Base: `origin/main` (Fases A y C ya integradas)

Responde a `docs/14-COMPLIANCE-WORKFLOW-UIUX-AUDIT.md` §4 y a los puntos 8, 11 y 12 del punch list.
Todas las afirmaciones del audit fueron verificadas contra el código actual antes de escribir esto.

## Qué se verificó (no se asumió)

| Afirmación de docs/14 | Verificado |
| --- | --- |
| `create_entry` es el único choke point, balanceado e idempotente | Sí — `contabilidad_service.rs:394`, `UNIQUE(tenant_id, referencia_tipo, referencia_id)` + `ON CONFLICT DO NOTHING` |
| `cuenta` nunca se valida contra `cuentas_contables` | Sí — `create_entry` inserta el string tal cual; `cuentas_contables` solo se lee en `list_cuentas` |
| No existe Estado de Resultados ni Balance General | Sí — no hay ruta ni servicio; solo libro mayor/diario |
| `http_create_asiento`, `http_create_compra`, `http_create_gasto`, `http_create_banco_movimiento` y `sincronizar` no auditan | Sí — ninguno llama `audit_service.log` (`main.rs` 3532, 3599, 3749, 3974, 4009) |
| No hay forma de pagar `2110 Cuentas por Pagar` en la app | Sí — no existe `proveedor_abonos` ni endpoint; `proveedores` no tiene `saldo_pendiente` |
| `sincronizar` solo corre a mano | Sí — única llamada: `POST /v1/contabilidad/sincronizar` |
| Fase A sembró `3100 Capital Social` / `3200 Resultados del Ejercicio` | Sí — `auth_service.rs:315` y `bin/migrate.rs:730` |

## Alcance, en orden de prioridad

### F1 — Auditar las rutas de escritura sin bitácora
`main.rs`: agregar `audit_service.log(...)` (mismo patrón que `VENTA_CREADA` / `ASIENTO_REVERSADO`) en
`http_create_asiento` (`ASIENTO_MANUAL_CREADO`), `http_create_compra` (`COMPRA_REGISTRADA`),
`http_create_gasto` (`GASTO_REGISTRADO`), `http_create_banco_movimiento` (`BANCO_MOVIMIENTO_REGISTRADO`)
y `http_sincronizar_contabilidad` (`CONTABILIDAD_SINCRONIZADA`, con los conteos por tipo).
Cada llamada es una línea después del `?` del servicio, por lo que un fallo de negocio nunca deja rastro falso.

### F2 — Validar el código de cuenta al escribir
Las líneas son strings `"<codigo> <nombre>"`. En `create_entry`, antes de insertar:
tomar el primer token (`split_whitespace().next()`) de cada línea y exigir que exista en
`cuentas_contables` del tenant con `activo = true`. Si falta → error en español, 400 en el handler.

**Se matchea solo por código, nunca por nombre.** Renombrar una cuenta en el plan no debe invalidar
asientos automáticos históricos, y las cadenas hardcodeadas de `sincronizar` no deberían tener que
seguir la semilla carácter por carácter (incluyendo tildes y paréntesis, p. ej.
`"5295 Ajuste de Inventario (Merma)"`).

Las 18 cuentas que usa `sincronizar` están todas en la semilla, así que ninguna contabilización
automática se rompe. La consulta es una sola por asiento
(`WHERE codigo = ANY($2)`), dentro de la misma transacción.

### F3 — Estados financieros
Dos endpoints nuevos, leídos de `asientos_contables` con `LEFT JOIN cuentas_contables`
`ON cc.codigo = split_part(ac.cuenta, ' ', 1)` (el mismo criterio de matching que F2):

- `GET /v1/contabilidad/estado-resultados?desde=&hasta=` → `ingresos`, `costo_ventas`, `utilidad_bruta`,
  `gastos`, `resultado`, más el detalle por cuenta de cada bloque.
- `GET /v1/contabilidad/balance-general?al=` → `activo`, `pasivo`, `patrimonio`, `resultado_acumulado`,
  `cuadra`, más detalle.

Signos: `naturaleza` DEUDORA ⇒ `debe − haber`; ACREEDORA ⇒ `haber − debe`.
Costo de ventas se separa de los gastos por prefijo de código: `tipo = 'GASTO'` con código que empieza
en `50` es costo de ventas (hoy solo `5050`), el resto son gastos operativos.

**Cuadre:** como todo asiento balancea, `Σ(debe−haber) = 0` sobre todas las líneas, de donde
`Activo + Sin clasificar = Pasivo + Patrimonio + Resultado acumulado`. La identidad se calcula, no se
asume: el campo `cuadra` la reporta y los tests la verifican sobre datos reales (ventas, compras,
gastos, nómina).

**Cuentas sin fila en `cuentas_contables`** (fantasmas creados antes de F2): no se descartan ni se
adivina su tipo. Van a un bloque propio `sin_clasificar` con su detalle, visible en la página, para que
el descuadre sea auditable en vez de silencioso.

**Relación con el período abierto:** no existe asiento de cierre (fuera de alcance), así que
`3200 Resultados del Ejercicio` está siempre en cero y el resultado del ejercicio nunca se capitaliza.
Por eso el balance muestra `resultado_acumulado` (el P&L de *toda* la historia hasta `al`) como una
línea propia dentro de patrimonio, y el Estado de Resultados del período es **provisional**.

UI: `apps/web/app/api/contabilidad/{estado-resultados,balance-general}/route.ts` (proxies delgados) y
dos páginas bajo `contabilidad/` con tablas planas (`ScrollableTableCard`), enlazadas desde el landing.
Sin librerías de gráficos.

### F4 — Mantener el mayor al día
**Decisión: sincronizar-al-leer**, en los handlers de lectura de contabilidad (los dos estados nuevos,
`libro-mayor`, `libro-mayor/:cuenta`, `libro-diario`, `asientos`), vía un helper único y **best-effort**:
si `sincronizar` falla, se registra un `warn` y la lectura se sirve igual.

Por qué, frente a las alternativas:
- *Sincronizar después de cada escritura de dinero*: tocaría ~10 handlers de módulos que otras fases
  están editando (máximo riesgo de conflicto) y un fallo de contabilidad contaminaría la respuesta de
  una venta que sí se guardó.
- *Job de fondo por intervalo*: requiere enumerar tenants y un task que corre para siempre; en tests y
  en dev queda trabajo de fondo no determinista.
- *Al leer*: el trabajo se hace exactamente donde importa, sin tocar ningún módulo ajeno.

Modos de fallo, asumidos a propósito:
1. **Concurrencia** — dos lecturas simultáneas lanzan dos `sincronizar`; la segunda bloquea sobre el
   `UNIQUE` y termina en `DO NOTHING`. Ya cubierto por
   `concurrent_sincronizar_calls_never_double_post_the_same_sale`.
2. **Costo en tenants grandes** — cada lectura escanea las tablas de origen con `NOT EXISTS`. Barato
   cuando no hay nada nuevo, pero es una consulta por tabla por lectura. Si duele, el siguiente paso es
   un guard por tenant (marca de última sincronización) — no se hace ahora.
3. **Período cerrado** — si hay movimiento sin contabilizar dentro de un mes cerrado, `sincronizar`
   aborta. Best-effort significa que la vista sigue funcionando y el usuario ve el mayor tal como está.
4. **Un GET ahora escribe.** Es el costo explícito de esta decisión.

`POST /v1/contabilidad/sincronizar` sigue existiendo y sigue siendo idempotente.

### F5 — Abonos a proveedor
Espejo del lado cliente (`cliente_abonos` / `partner_service::registrar_abono`):
- Tabla `proveedor_abonos` (bloque idempotente al final de `migrate.rs`).
- `GET/POST /v1/proveedores/:id/abonos`, con auditoría `ABONO_PROVEEDOR_REGISTRADO`.
- **Saldo del proveedor derivado, no una columna nueva**: `compras` FIADO no anuladas − notas de
  crédito FIADO − abonos. Evita una tercera fuente de verdad que pueda derivar (el audit ya señala tres
  saldos de caja/banco sin conciliar).
- Caja: abono en efectivo escribe un `caja_movimientos` EGRESO. Una transferencia (`TRANSFERENCIA` /
  `CHEQUE`) **no** toca la caja física — se registra igual y el asiento sigue siendo
  `Dr 2110 / Cr 1100`, porque el plan de cuentas tiene una sola cuenta "Caja y Bancos".
- Nuevo loop en `sincronizar` (`referencia_tipo = 'ABONO_PROVEEDOR'`), como bloque propio al final.
- UI: acción en la lista de proveedores con `ConfirmDialog` de `packages/ui`.

## Archivos

Nuevos: `services/core/src/services/estados_financieros.rs` · `services/core/tests/estados_financieros.rs` ·
`services/core/tests/proveedor_abonos.rs` · `apps/web/app/api/contabilidad/{estado-resultados,balance-general}/route.ts` ·
`apps/web/app/api/proveedores/[id]/abonos/route.ts` ·
`apps/web/app/(customer)/(dashboard)/contabilidad/{estado-resultados,balance-general}/page.tsx`

Tocados (hunks pequeños y aditivos): `services/core/src/main.rs` (rutas + handlers + `audit_service.log`) ·
`services/core/src/services/contabilidad_service.rs` (validación en `create_entry`, loop nuevo de abonos) ·
`services/core/src/services/partner_service.rs` (abonos a proveedor) · `services/core/src/bin/migrate.rs`
(bloque nuevo al final) · `contabilidad/page.tsx` y `proveedores/page.tsx` (enlaces / acción).

**No se toca la rama `NOTA_CREDITO` de `sincronizar`** (la está reescribiendo la Fase D).

## Riesgos

- F2 puede rechazar un asiento manual que hoy pasa. Es el objetivo, pero cambia un contrato: se corre
  la suite completa de backend antes de dar por bueno el commit.
- La identidad del balance depende de que cada `tipo` tenga su `naturaleza` convencional
  (ACTIVO/GASTO ⇒ DEUDORA, PASIVO/PATRIMONIO/INGRESO ⇒ ACREEDORA). Hoy `cuentas_contables` es solo
  semilla (no hay endpoint de escritura), así que se cumple; `cuadra` lo reporta en vez de asumirlo.
- Sincronizar-al-leer convierte GETs en escrituras (ver modos de fallo arriba).

## Fuera de alcance (explícito)

Asiento de cierre / cierre de ejercicio · UI de aportes de capital · multimoneda · presupuestos ·
gráficos · Formato 609 · conciliación de las tres fuentes de saldo de caja/banco · backfill de las
cuentas fantasma que ya existan · cualquier cosa de otras fases (devoluciones parciales, agenda de
técnicos, ISR).

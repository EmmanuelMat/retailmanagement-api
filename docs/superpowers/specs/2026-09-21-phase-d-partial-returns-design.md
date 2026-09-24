# Fase D — Devoluciones parciales (Nota de Crédito por línea)

Rama: `feat/phase-d-partial-returns`. Cierra el punto 5 del audit (`docs/14`): hoy
`ventas_service::create_nota_credito` es "V1: solo devolución total" — revierte el 100 % del stock
y de la caja, marca la venta `ANULADA` y emite un único E34. Devolver 1 de 5 refrescos obliga a
anular la venta completa.

## Alcance de esta rebanada

Devolver **una cantidad por línea** de una venta ya facturada, tantas veces como haga falta hasta
agotar la línea, con su Nota de Crédito (E34) parcial, su reversión de inventario/caja, su asiento
contable proporcional y su UI.

Fuera de alcance (se reportan como Observaciones, no se tocan): conversión de unidades de medida,
pago dividido, cambio de precio en la devolución, multi-cajero, `NOTA_DEBITO` (E33), notas de
crédito de **compra** (`compras_service.rs`, Fase B), y el indicador de los 30 días del ITBIS
(ver "Riesgos").

## Modelo de datos

Bloque idempotente al final de `migrate.rs`:

```sql
CREATE TABLE IF NOT EXISTS nota_credito_items (
  id, nota_credito_id -> notas_credito(id) ON DELETE CASCADE,
  venta_item_id -> venta_items(id), producto_id -> productos(id),
  sku, nombre, cantidad, precio_unitario, descuento, itbis_tipo,
  itbis_monto, subtotal, costo_unitario, created_at
);
ALTER TABLE notas_credito ADD COLUMN IF NOT EXISTS es_parcial BOOLEAN NOT NULL DEFAULT false;
```

`es_parcial` es el discriminador que usa contabilidad: `false` (incluye todas las filas
históricas) ⇒ camino de hoy, intacto.

## API

- `POST /v1/ventas/:id/nota-credito` gana `items: [{ venta_item_id, cantidad }]` **opcional**.
  Ausente = devolución total, comportamiento byte-idéntico al de hoy.
- `GET /v1/ventas/:id/devoluciones` (nuevo) — notas de la venta + cantidad devuelta por línea.
  Endpoint aparte en vez de un campo nuevo en `VentaItem`, para no tocar `create_venta`/`get_venta`.
- `GET /v1/notas-credito` (nuevo) — listado paginado para la página `ventas/devoluciones`.

## Reglas de dinero

1. **El crédito sale de la línea original vendida**, nunca del precio actual del producto:
   `precio_unitario`, `descuento` e `itbis_tipo` se leen de `venta_items`.
2. **Prorrateo + resto al último**: para devolver `q` de `C` unidades,
   `subtotal_q = round2(subtotal_linea * q / C)` (half-up, `MidpointAwayFromZero`), `itbis_q` igual.
   Cuando la devolución **agota** la línea, `subtotal_q = subtotal_linea − Σ(devueltos previos)`.
   Así `Σ devoluciones parciales de una línea == la línea original`, exacto, sin centavos perdidos.
   *Decisión: el resto se asigna a la última devolución, no a la primera ni repartido.*
3. **Tope acumulado por línea**, con `SELECT ... FOR UPDATE` sobre `venta_items` (mismo patrón que
   `conduce_service::create_conduce`) + `FOR UPDATE` sobre la venta: devolver más de lo que queda
   se rechaza con mensaje en español.
4. **Stock**: se reingresa `min(Σdevuelto, cantidad_entregada) − min(Σdevuelto_previo, entregada)`,
   o sea solo lo que realmente salió (ventas de entrega diferida). Ítems `SERVICIO` nunca mueven
   stock. Movimiento `ENTRADA` con `referencia_tipo = 'NOTA_CREDITO'`, como hoy.
5. **Caja / cliente**: venta no-FIADO ⇒ `caja_movimientos` EGRESO por el monto acreditado.
   Venta FIADO ⇒ `clientes.saldo_pendiente -= monto` y **ninguna** salida de caja.
   *Esto corrige la devolución total de hoy*, que saca efectivo de la caja por una venta fiada que
   nunca lo ingresó y deja la deuda del cliente intacta — el asiento ya acreditaba 1110, así que
   caja y libro discrepaban.
6. **Estado de la venta**: `ANULADA` solo cuando todas las líneas quedan devueltas por completo;
   si no, sigue `COMPLETADA`. El e-NCF y el estado DGII de la venta no se tocan nunca.

## Contabilidad (`sincronizar`, rama NOTA_CREDITO)

- `es_parcial = false` ⇒ espejo del asiento de la venta (debe/haber invertidos): **idéntico a hoy**.
- `es_parcial = true` ⇒ asiento construido desde las líneas de **la nota**:
  4100 debe = subtotal acreditado · 2100 debe = ITBIS acreditado ·
  1200 debe / 5050 haber = Σ(cantidad devuelta × `costo_unitario` de la línea vendida) ·
  1100 (o 1110 si la venta fue FIADO) haber = total acreditado.
  Mismo `create_entry`, mismo `UNIQUE(tenant_id, referencia_tipo, referencia_id)` ⇒ idempotente.

## e-CF (E34)

- Solo las líneas devueltas, con su cantidad devuelta y su descuento prorrateado, de modo que los
  totales del E34 nunca superen los de la factura original.
- `build_simple_pos_ecf` no sabía de descuentos (`MontoItem = cantidad × precio`), lo que inflaba
  el E34 de cualquier venta con descuento. Se añade `build_pos_ecf_con_descuento` (con `MontoItem`
  neto y `<DescuentoMonto>` en el XML) y `build_simple_pos_ecf` queda como wrapper con descuento 0
  — ningún call site existente cambia.
- `CodigoModificacion`: `1` (anulación total) para la devolución total, `3` (corrección de montos:
  "devoluciones, descuentos o bonificaciones") para la parcial. Hoy está fijo en `1`.
  Fuente: DGII, *Formato Comprobante Fiscal Electrónico (e-CF) v1.0* y la ficha CA4275 de la
  Comunidad de Ayuda DGII (`https://ayuda.dgii.gov.do/.../ca4275-...`), consultadas 2026-09-21.
- `FechaNCFModificado` (obligatorio en el formato, hoy se omite) se llena con la fecha de la venta.
- Se conserva intacto el pipeline de firma/envío y el estado `CONTINGENCIA_PENDIENTE`.
- Si el tenant **no** tiene `factura_electronica_activa`, la nota se registra sin e-CF en vez de
  fallar con 400. Hoy un colmado sin e-CF no puede devolver nada. Con e-CF activa el
  comportamiento no cambia (sigue exigiendo el e-NCF de la venta).

## IT-1

`report_service::itbis_neto_del_rango` suma ventas `estado = 'COMPLETADA'` y resta
`notas_credito.itbis_total`. Para la nota **parcial** es correcto (la venta sigue COMPLETADA).
Para la **total** resta dos veces (la venta sale de la suma al pasar a ANULADA *y* la nota resta).
Se corrige a `estado IN ('COMPLETADA','ANULADA')`, que es el tratamiento fiscal correcto: el ITBIS
se declara en el período de la factura y se acredita en el período de la nota.

## Archivos

`services/core/src/bin/migrate.rs` (bloque al final) · `services/core/src/services/ventas_service.rs`
· `services/core/src/services/contabilidad_service.rs` (solo la rama NOTA_CREDITO) ·
`services/core/src/services/report_service.rs` (1 línea) · `services/core/src/ecf_builder.rs` ·
`services/core/src/main.rs` (request + handler + 2 rutas) ·
`services/core/tests/ventas_devoluciones_parciales.rs` (nuevo) ·
`apps/web/app/(customer)/(dashboard)/ventas/[id]/page.tsx` ·
`apps/web/app/(customer)/(dashboard)/ventas/devoluciones/page.tsx` ·
`apps/web/app/api/notas-credito/route.ts` + `apps/web/app/api/ventas/[id]/devoluciones/route.ts` (nuevos) ·
`apps/web/cypress/e2e/devolucion-parcial.cy.ts` (nuevo).

## Riesgos

- **Regla de los 30 días del ITBIS**: DGII (Reglamento 293-11, arts. 8 y 28; ficha CA4275) distingue
  la nota de crédito emitida dentro de 30 días calendario de la posterior, donde el ITBIS ya no se
  rebaja. Este sistema no modela ese indicador ni hoy ni después de esta fase. **No se implementa**
  aquí porque cambiaría los montos de las devoluciones totales existentes; queda reportado.
- Cambiar caja→saldo_pendiente en FIADO altera la devolución total de hoy. Ningún test existente la
  cubre (requiere P12), así que la red de seguridad es el test nuevo, no la suite vieja.
- `sincronizar` es archivo compartido con la Fase F: el diff se limita a la rama NOTA_CREDITO.

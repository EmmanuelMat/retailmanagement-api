# Cotización / Orden de Servicio pricing rework + auto product codes

Date: 2026-09-13
Status: Approved, ready for implementation planning

## Problem

Client feedback on the SERVICIOS workflow:

1. In cotizaciones, `PRODUCTO`-type lines silently use the catalog's fixed
   `precio_venta`; only `SERVICIO`-type lines require a manually typed
   price. The client wants every cotización line manually priced,
   regardless of tipo — the catalog price should be a starting suggestion,
   never an unreviewable default.
2. Product/service codes (`sku`) are typed by hand. The client wants them
   auto-generated from the name (e.g. "Fumigación" → `FUM-0001`), while
   still letting staff search products/services by name or code.
3. Órdenes de servicio currently carry full pricing (items with
   precio/descuento/ITBIS, an order-level subtotal/ITBIS/total, and a
   priced items table on the printed copy). The client wants the order
   itself to carry no price at all — just which service(s), quantity, and
   a description of the work. Price should only be entered once, at the
   point the completed order is turned into an invoice (venta).
4. The materiales tab (planned/used parts, inventory consumption) must
   keep working exactly as it does today, and must never appear — with or
   without price — on the printed orden de servicio.
5. POS (direct sales, not originating from a cotización or orden de
   servicio) is explicitly **out of scope** — it keeps today's behavior
   (PRODUCTO auto-priced from catalog, SERVICIO manually priced).

## Non-goals

- No change to `ventas_service.rs` / `create_venta`'s pricing branch logic.
  POS keeps calling it exactly as today.
- No change to `orden_servicio_materiales`, the materiales tab, or the
  `consumir` inventory-consumption endpoint.
- No removal of `productos.precio_venta` — it remains required for
  `tipo = 'PRODUCTO'` and is now purely a suggested default.
- No new sequence/counter table for code generation — the next code is
  derived from existing rows at request time.

## Design

### A. Data model changes

All changes are additive/nullable-widening — no destructive migration,
no backfill needed; historical cotización/orden/venta rows keep whatever
prices they already have.

- `orden_servicio_items`: make `precio_unitario`, `descuento`,
  `itbis_tipo`, `itbis_monto`, `subtotal` nullable. The order stops
  carrying price entirely; these columns are simply left null going
  forward.
- `ordenes_servicio.subtotal` / `.itbis_total` / `.total`: left in place
  (avoid a destructive column drop) but always computed as `0` from now
  on. The UI stops rendering that block (see Resumen tab below).
- `orden_servicio_items.observaciones` (already exists, currently unused
  by any UI or the create/nueva forms): wire it up as the line-level
  "Descripción" field — no new column needed here.
- `cotizacion_items` and `venta_items`: add a new nullable
  `descripcion TEXT` column. This is where a cotización line's note
  (e.g. "2 habitaciones, tratamiento inicial") lives, and it flows
  through to the venta when a cotización converts.
- `productos.precio_venta`: unchanged schema; existing
  `chk_productos_precio_venta_por_tipo` constraint stays as-is.

### B. Auto-generated product/service code

- New endpoint: `GET /v1/productos/sugerir-codigo?nombre=...` in
  `catalog_service.rs`.
  - Strip accents and non-letters from `nombre`, uppercase, take the
    first 3 letters as the prefix (e.g. "Fumigación" → `FUM`).
  - Query existing SKUs for the tenant matching `FUM-%`, take the
    highest existing sequence, return `FUM-<next, zero-padded to 4>`.
  - No new table — the sequence is derived from `productos` rows at
    request time, so it self-heals if a code is deleted.
  - Applies uniformly to `PRODUCTO` and `SERVICIO` — both draw from the
    same per-prefix sequence; a product and a service can share a prefix
    and just advance the same counter.
- `producto-form.tsx`: replace the free-text SKU input with a field that
  auto-fills from this endpoint (debounced on `nombre` changes) but
  remains editable. `create_producto`'s existing uniqueness check
  (`catalog_service.rs:329`, "Ya existe un producto con SKU: {}") is the
  final guard against a stale/collided suggestion; on that error the
  form re-requests a suggestion.

### C. Cotización: always-manual pricing + search + description

- `cotizacion_service.rs`: remove the `tipo == "SERVICIO"` branch in both
  `crear_cotizacion` (~L124-131) and `agregar_item` (~L237-244). Every
  line now requires `precio_unitario` in the request — `PRODUCTO` no
  longer silently falls back to `precio_venta_catalogo`. ITBIS math is
  unchanged (`itbis_rate(tipo) * line_subtotal`), it's just now always
  driven by a manually-entered price.
- `cotizaciones/nueva/page.tsx`: every line's price input is always
  rendered (today it's conditionally rendered only for `esServicio`),
  pre-filled with `producto.precio_venta` for `PRODUCTO` lines (empty
  for `SERVICIO`) as a starting suggestion, always editable.
- New shared component (modeled on `cliente-picker.tsx`), replacing the
  full-list `<Select>` in `cotizaciones/nueva/page.tsx` and in
  `ordenes-servicio/nueva/page.tsx` + its `ItemsTab`: type-ahead search
  against the existing `GET /v1/productos?search=` (already filters by
  `nombre`/`sku` server-side, `catalog_service.rs:253`), rendering
  `codigo · nombre`. **Scope: cotización and orden de servicio only.**
  POS is unchanged — it already has its own working search input
  (`pos/page.tsx:373`).
- New "Descripción" text input per línea in `cotizaciones/nueva`, saved
  into the new `cotizacion_items.descripcion` column. Optional, free
  text.

### D. Orden de servicio: no pricing, price moves to facturación

- `ordenes-servicio/nueva/page.tsx` and `ItemsTab`
  (`ordenes-servicio/[id]/page.tsx`): drop `precioUnitario`/`descuento`/
  ITBIS/subtotal from the line UI entirely. A line becomes: search
  picker (service) + cantidad + descripción (wired to `observaciones`).
  No total shown anywhere on the creation form or items tab.
- `orden_servicio_service.rs`: `create_orden`'s two pricing-computation
  blocks (~L227-249 item insert, ~L519-544 add-item) collapse into plain
  inserts with all price columns left null — no more
  "es un servicio: falta precio_unitario" validation on this path.
  `CreateOrdenServicioItemRequest` drops its `precio_unitario` and
  `descuento` fields entirely.
- **Resumen tab** (`page.tsx:238-241`): remove the
  subtotal/descuento/ITBIS/total block — nothing to show.
- **Facturación tab redesign**: this becomes the one place price is
  entered, once, at billing time. For each of the orden's existing items,
  render name + cantidad, plus a required `precio_unitario` input **for
  `SERVICIO` items only**; a `PRODUCTO` item (only possible on a
  non-SERVICIOS-tenant orden — see below) shows its catalog
  `precio_venta` read-only instead, since `create_venta` ignores whatever
  price is submitted for `PRODUCTO` lines and always uses the catalog
  value (`ventas_service.rs:200-210`) — showing an editable field there
  would be misleading.
  - `FacturarOrdenRequest` (main.rs) gains an `items: Vec<{ producto_id,
    precio_unitario: Option<Decimal>, descuento: Option<Decimal> }>`
    field — `precio_unitario` required by the frontend only for
    `SERVICIO` lines, mirroring `create_venta`'s own validation.
  - `http_facturar_orden` (`main.rs:3200`) builds
    `CreateVentaItemRequest` from these submitted values instead of
    reading `it.precio_unitario` off the orden's items (which no longer
    carry one).
  - `ventas_service::create_venta` itself is **unchanged**. In a
    SERVICIOS-tenant, every orden item is `tipo = SERVICIO` in practice
    (catalogs are filtered to `SERVICIO` at creation time), so its
    existing "SERVICIO requires manual price" branch is satisfied
    naturally; a `PRODUCTO` item on a non-SERVICIOS tenant's orden falls
    through to the existing catalog-price branch untouched.
- **Cotización → orden conversion** (`http_convertir_cotizacion_a_orden`,
  `main.rs:3276-3293`): stop forwarding `precio_unitario`/`descuento`
  from the cotización item into the orden item request (the field no
  longer exists on that request struct). Map the cotización item's new
  `descripcion` into the orden item's `observaciones`.
- **Print template** (`imprimir/orden-servicio/[id]/page.tsx`): remove
  the items table (lines 82-101, which lists product/service name, price,
  and subtotal) and the subtotal/ITBIS/total block (103-109) entirely.
  Keep client info, "Descripción del trabajo," and the signature lines.
  No products, no services list, no price — matches "the products are
  not to be printed there either."

### E. Unaffected: materiales tab & inventory

`orden_servicio_materiales` (planned/used quantity + cost) and the
`consumir` inventory-consumption endpoint are untouched — they already
have no sale-price field. Add a regression test confirming stock still
decrements correctly through `movimientos_inventario` after this
refactor, since it sits directly adjacent to code that's changing.

## Testing plan

- `services/core/tests/servicio_producto.rs` and `ordenes_servicio.rs`:
  these almost certainly assert today's behavior (SERVICIO-only manual
  price, priced orden items) — update, don't just add alongside.
- New/updated Rust tests: cotización requires manual price for `PRODUCTO`
  lines now; `create_orden` rejects/ignores price fields; SKU-suggestion
  endpoint (prefix derivation, sequence increment, collision → re-suggest
  on uniqueness error); `http_facturar_orden` sources price from the
  request body, not from orden items.
- Frontend manual pass: cotización creation (search picker, manual price
  on a `PRODUCTO` line, ITBIS math, descripción field); orden de servicio
  creation → completar → facturación (entering price at that step) →
  resulting venta looks right; printed orden de servicio shows no
  items/price/products; materiales tab still consumes inventory
  correctly.

## Open items for the implementation plan

- Exact accent-stripping approach for the code prefix (likely a small
  Unicode-normalize + filter-alpha helper in `catalog_service.rs`).
- Whether the SKU-suggestion endpoint needs tenant-scoped rate limiting
  given it's called on every keystroke (debounce on the frontend should
  make this a non-issue, but worth a note in the plan).

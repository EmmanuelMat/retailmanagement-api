# Phase B — DGII compliance completion (design note)

Branch `feat/phase-b-compliance-completion`. Follow-up to Phase A on the items docs/14 §1 and §4 left open.
Every tax/legal value below is sourced; the sources are listed at the bottom and repeated in code comments.

## Scope slice

**This phase was cut short by an infrastructure budget limit. B1, B2 and the
documentation half of B5 shipped; B3 and B4 did not — see "Not delivered" below
and the gap analyses in the PR body.** The B3/B4 designs are kept here because
the research behind them was done and sourced; they are a plan, not code.

| # | Item | Status |
|---|------|--------|
| B1 | Price-only supplier credit note | **Delivered** — `ajuste_solo_precio` flag on `CreateCompraRequest` + new path in `compras_service` |
| B2 | Drop the stale "XAdES-BES" label | **Delivered** — comment/copy-only edits, no signing logic touched |
| B3 | ISR withholding in payroll | **Not delivered** — scale sourced (below), no code |
| B4 | Format 609 (Pagos al Exterior) | **Not delivered** — layout sourced (below), no code |
| B5 | `codigo_seguridad` verification | **Partially delivered** — dead variable removed + ambiguity documented in code; no test |

### Not delivered (and why)

* **B3 (ISR).** The 2026 scale *is* sourced (DGII CA687 / Resolución
  DDG-AR1-2026-00001, below) and the design below is implementable as written,
  but payroll withholding changes real take-home pay and needs an accountant's
  sign-off plus worked-example tests; it was not worth half-shipping.
* **B4 (609).** The 13 columns, their order and the pipe-delimited TXT are
  sourced, but DGII does **not** publish the numeric codes its Excel tool
  serialises the five enumerated columns to. Shipping a generator on a guessed
  codification would produce a file that looks right and is rejected.
* **B5 (test).** No official sample pairing a SignatureValue with its expected
  security code exists, so there is nothing to assert against beyond
  determinism. The derivation is deliberately unchanged.

## B1 — price-only purchase NOTA_CREDITO

Today a purchase `NOTA_CREDITO` always means "goods went back to the supplier": stock SALIDA, weighted-average
`costo` recomputed downward, caja INGRESO, reversed ledger entry (Phase A). A supplier credit note that is only a
price/discount adjustment has no goods movement, so that path either wrongly decrements stock or is rejected by
the `stock_actual < cantidad` guard once the goods are sold.

**Approach.** Add `ajuste_solo_precio: Option<bool>` to `CreateCompraRequest`, default `false` — every existing
caller and test keeps today's behaviour byte for byte. It is only accepted when `tipo_documento = NOTA_CREDITO`.

On the price-only path, per line:
* **no** stock change and **no** `inventario_movimientos` row (nothing physically moved; the kardex must stay a
  pure quantity ledger);
* `costo` is reduced so that the Phase A invariant `GL 1200 Inventario == Σ(stock_actual × costo)` still holds.
  The ledger credits `1200 Inventario` by the note's `subtotal` (unchanged `contabilidad_service` branch), so the
  product's carrying value must drop by exactly the same amount at unchanged quantity:
  `costo_nuevo = (costo_actual × stock_actual − linea_subtotal) / stock_actual`.
* The line's `cantidad × costo_unitario` is read as "the total price reduction for this product" — the natural
  encoding is `cantidad` = units originally invoiced, `costo_unitario` = per-unit price drop.

**Two refusals instead of silently breaking the invariant:**
1. `stock_actual = 0` → the discounted goods are already sold, so the credit belongs in `5050 Costo de Ventas`,
   not in `1200 Inventario`. There is no line-level COGS-credit path here (`contabilidad_service` derives the
   entry from the `compras` header, and that file belongs to another phase this week), so this is rejected with a
   message pointing at a manual journal entry.
2. `linea_subtotal > costo_actual × stock_actual` → the credit exceeds the carrying value; same reasoning.

Everything else is deliberately untouched and therefore stays consistent for free: caja INGRESO / CxP handling,
the reversed ledger direction, `report_service::itbis_neto_del_rango` (already nets purchase credit notes out),
and the 606 (a credit note is already its own row with `ncf_modificado`). `FACTURA` and `NOTA_DEBITO` unchanged.

## B2 — "XAdES-BES" → "XML-DSig enveloped (DGII e-CF spec)"

DGII's *Firmado de e-CF* PDF (read for this phase) contains no XAdES element at all — no `QualifyingProperties`,
`SignedProperties`, `SigningCertificate` or `SigningTime`; its sample `<Signature>` is stock `xmldsig#` with
exactly the four algorithm URIs the repo already implements. Labels only: `apps/web/app/api/sales/route.ts`,
`apps/web/app/api/test/sign/route.ts` (+ its `demo.md`), `apps/web/lib/core-client.ts`, the header comments of
`ecfl_service.rs` and `main.rs`, the two `main.rs` strings, and the body of docs 09 and the labels in docs 00–04.
**File names stay** (they are referenced elsewhere); doc 09 gets a correction note at the top.

## B3 — ISR withholding

New `services/core/src/services/isr_escala.rs`: the annual Art. 296 scale as `const` `Decimal`s with the source
URL, the resolution number and the effective year, plus `isr_mensual(salario_bruto_mensual, tss_empleado)`.

Method (all of it sourced, see below): taxable base = monthly gross less the employee's TSS contributions
(AFP + SFS, the `TSS_RATE = 5.91%` already withheld here) → annualise ×12 → apply the progressive annual scale →
divide by 12 → round to 2 decimals. `run_payroll` pays a full `salario_mensual` per run, i.e. it is a monthly
payroll, so only the monthly withholding is implemented; quincenal is not a period this system can produce today.

Ledger: none needed. `contabilidad_service`'s NOMINA branch books `bruto − neto − adelantos` to
`2200 Retenciones y Descuentos`, so ISR flows there automatically the moment `neto` drops. That file is left
untouched.

## B4 — Format 609

Smallest model that can produce the file: one new table `pagos_exterior` holding exactly the 13 documented
columns (optionally linked to a `proveedores` row, but not requiring one — the beneficiary of a 609 payment is by
definition not a local RNC supplier). `proveedores` is **not** altered.

* `pago_exterior_service.rs` (new file): create + list, with validation of the DGII-enumerated fields.
* `report_service::generate_609` (TXT, pipe-delimited, header `RNC|AAAAMM|cantidad`) and `generate_609_csv`,
  mirroring `generate_606{,_csv}`.
* `main.rs`: one contiguous route block `/v1/pagos-exterior` + `/v1/reports/609{,/csv}`; the `reportes.dgii`
  permission match is extended in place.
* Web: `api/reports/609{,/csv}/route.ts` proxies + `reportes/dgii/609/page.tsx` reusing `ReporteTxt`, plus one
  entry in the `REPORTES` array.

**Known gap, deliberately not invented (see "Not verified"):** DGII's 609 *instructivo* documents the 13 columns,
their order, the dropdown value lists and the pipe-delimited TXT, but it does **not** publish the numeric codes
those dropdowns serialise to in the TXT (unlike the 606, whose code tables are published). This phase emits the
DGII label strings verbatim and marks the codification `UNVERIFIED` in code, tests and the PR. Retention is a
captured field (`isr_retenido`, `renta_presunta`), never computed: the general Art. 305 rate is 27%, but reduced
rates exist per concept and per treaty, so computing it here would be guessing.

## B5 — `codigo_seguridad`

`sign_xml_ecf` currently returns `sha256(raw signature bytes)[..6]` uppercased, and carries a dead `hasher2` that
computes `sha256(base64 SignatureValue string)`. DGII's own *Firmado de e-CF* PDF does not mention the security
code at all, and no official sample pairing a SignatureValue with its expected code was found. Per the phase
rules the derivation is therefore **not changed**: the dead variable goes, a black-box test through
`/v1/test/sign-demo` pins determinism/shape/consistency with the QR URL, and the ambiguity is documented as
needing a real DGII test-environment round trip.

## Files actually changed

New: `services/core/tests/compras_nota_credito_precio.rs` (6 tests).
Edited (small, localised hunks): `compras_service.rs`, `bin/migrate.rs` (one `ADD COLUMN IF NOT EXISTS` at the end),
`ecfl_service.rs` (comments only), the B2 label sites (`apps/web/app/api/sales/route.ts`,
`apps/web/app/api/test/sign/{route.ts,demo.md}`, `apps/web/lib/core-client.ts`, `main.rs`) and docs 00–04, 09–11.
`contabilidad_service.rs`, `report_service.rs`, `ventas_service.rs` and `nomina_service.rs` are untouched.

## Risks

* B1's cost formula is the only thing standing between a price-only note and a broken `1200` invariant; it is
  covered by a test that reads the GL and the product back.
* B3 changes take-home pay. The scale is sourced but **must** be checked by an accountant before any real run.
* B4's TXT codification is unverified against DGII's own Excel tool — flagged, not hidden.

## Explicitly out of scope

PSFE certification, 607/608 (exempt for a fully-electronic issuer, docs/14), the sales-side NOTA_CREDITO ledger
branch and `ventas_service` (Phase D), `contabilidad_service.rs` and the `main.rs` audit calls (Phase F), a CRUD
UI for `pagos_exterior`, the IT-1 Anexo A breakdown, labour-law accruals (regalía/cesantía/vacaciones), and any
backfill of pre-Phase-A purchase credit notes.

## Sources actually read

* DGII, *Comunidad de Ayuda* CA687 "¿Cuál es la escala salarial correspondiente al año 2026 del ISR?" —
  2026 Art. 296 scale, Resolución DDG-AR1-2026-00001.
  <https://ayuda.dgii.gov.do/conversations/impuesto-sobre-la-renta-isr/ca687-cul-es-la-escala-salarial-correspondiente-al-ao-2026-del-impuesto-sobre-la-renta-isr/696a664277932619036537b8>
* DGII, *Instructivo Llenado y Remisión del Formato de Envío de Pagos por Servicios al Exterior (609)* (rev. 2026)
  — the 13 columns, their order, the value lists, the `AAAAMM` header and the `DGII_F_609_<RNC>_<periodo>.TXT`
  file name. <https://dgii.gov.do/publicacionesOficiales/bibliotecaVirtual/contribuyentes/formatoEnvioDatos/Documents/8-Instructivo%20Llenado%20y%20Remisi%C3%B3n%20del%20Formato%20de%20Env%C3%ADo%20de%20Pagos%20por%20Servicios%20al%20Exterior%20(609).pdf>
* DGII, *Firmado Comprobantes Fiscales Electrónicos (e-CF)* — plain XML-DSig, the four algorithm URIs, no XAdES.
  <https://dgii.gov.do/cicloContribuyente/facturacion/comprobantesFiscalesElectronicosE-CF/Documentacin%20sobre%20eCF/Instructivos%20sobre%20Facturaci%C3%B3n%20Electr%C3%B3nica/Firmado%20de%20e-CF.pdf>
* Código Tributario (Ley 11-92) Art. 296 (escala), Art. 297 (tasa) and Art. 305 (retención 27% sobre pagos al
  exterior); Ley 87-01 (las cotizaciones del afiliado a AFP/SFS reducen la base del ISR asalariado).

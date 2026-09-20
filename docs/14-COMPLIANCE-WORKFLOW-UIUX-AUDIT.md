# Investigation Audit — DGII Compliance, Retail/Service Workflows, Accounting, UI/UX

Date: 2026-09-18. Investigation only — no code changed as part of this doc. Produced by five parallel research passes (each cross-checking live web sources against the actual current code, not just the other docs in this folder).

## TL;DR

The core is genuinely solid — better than typical vibe-coded POS work. Double-entry posting is atomic, idempotent, and enforced at insertion (`services/core/src/services/contabilidad_service.rs::create_entry`); FIADO credit-limit enforcement and payroll-advance (EWA) accounting are correctly modeled with no interest/loan treatment; discount authorization has a real manager-approval flow; and completed service orders bill through the *same* `create_venta` path as retail sales, inheriting e-CF + ledger rigor for free.

But there are several **money-correctness bugs**, **one compliance-claim risk**, and **missing safety features** that would matter fast in real use.

## Cross-cutting critical findings (fix first)

1. ~~**Purchase-side Nota de Crédito/Débito posts as a new purchase, not a reversal.**~~ **FIXED in Phase A (branch `fix/phase-a-critical-fixes`).** A purchase `NOTA_CREDITO` now decreases stock (SALIDA movement), logs a caja INGRESO for non-FIADO purchases, and posts a reversed ledger entry (credits 1200/1150, debits 1100 or 2110); `NOTA_DEBITO` is deliberately unchanged. A related fix found during review: the IT-1 declaration's creditable-ITBIS total (`report_service.rs`, `itbis_neto_del_rango`) now nets purchase credit notes out (it previously added them), so IT-1 agrees with the ledger; the 606 is unchanged because it correctly lists a credit note as its own row with `ncf_modificado`. **Caveat: rows already synced before this fix are NOT corrected** — any purchase `NOTA_CREDITO` that was previously posted to the ledger with the old wrong direction (and whose stock was inflated) still needs a separate data remediation; no backfill was done.
2. ~~**No equity (Patrimonio) accounts exist in the chart of accounts.**~~ **FIXED in Phase A (branch `fix/phase-a-critical-fixes`) — accounts only.** `3100 Capital Social` and `3200 Resultados del Ejercicio` (PATRIMONIO, ACREEDORA) are now seeded for new tenants (`auth_service.rs`) and backfilled for existing tenants via `migrate.rs`. This only adds the accounts: a balance sheet / P&L report, booking owner capital contributions/draws, and a period close are still not built (later phase).
3. ~~Verify whether `send_to_dgii()` is still mocked in the live sale path.~~ **RESOLVED — confirmed not a bug (2026-09-18).** The live venta e-CF handler in `services/core/src/main.rs` (the e-CF section of the venta-creation flow, ~line 2370-2400) already calls the real `DGIIClient::authenticate` + `send_with_polling`, with a legitimate `CONTINGENCIA_PENDIENTE` fallback if DGII is unreachable. The fabricated-TrackID mock in `ecfl_service.rs`/`ecf_service.rs` is only reachable via the separate `/v1/test/sign-demo` endpoint, which is clearly a demo/test path and not used by real sales.
4. ~~**Marketing overclaims exceed what's built.**~~ **FIXED in Phase A (branch `fix/phase-a-critical-fixes`)** by the `README.md` and landing-page (`apps/web/app/(customer)/page.tsx`) copy edits: the Format 608/609 report claims were removed, the "XAdES-BES" label became "XML-DSig" (per DGII's e-CF spec), and "listo para certificación PSFE DGII" became "En preparación para certificación PSFE DGII (aún no certificado)". This was a copy-only fix: Format 609 is still **not implemented** (the README now says so) and PSFE certification is still **not achieved** (its prerequisites — 3 already-certified clients, live DGII send/poll, a complete XML builder — are still unmet).
5. **No returns/void UI exists for retail sales.** `apps/web/app/(customer)/(dashboard)/ventas/devoluciones/page.tsx` is a literal `<ComingSoon>` stub. There *is* a working nota-de-crédito path on the sale detail page (`ventas/[id]/page.tsx`), but it's full-reversal-only (v1, by explicit code comment) — no partial returns, no dedicated returns workflow/search.
6. **Technician double-booking is silently possible.** `orden_servicio_service.rs::asignar_tecnico` does zero conflict checking against schedule — full stack: the DB has unused `hora_inicio`/`hora_fin` columns, no UI reads or writes them, and no dispatch/calendar board exists anywhere.
7. **High-stakes, money-moving actions have no confirmation step.** Running payroll (`nomina/run`), approving/rejecting a payroll advance (`nomina/adelantos`), and cancelling a service order all fire on a single click — while deactivating a mere producto/categoría gets a two-step inline confirm. The safer pattern already exists in the codebase; it's just not applied where the stakes are highest.

**Correction / reconciliation across agents:** an earlier internal doc (`docs/12-LIBRO-DIARIO-LIBRO-MAYOR-PLAN.md`, §1.2.7) claimed sales-side `notas_credito` never reach the ledger. The accounting audit read the *current* code directly and confirmed this has since been fixed — `contabilidad_service.rs`'s `NOTA_CREDITO` branch (lines ~723-760) correctly mirrors the original venta's entry. The real bug was on the **purchase** side (item 1 above, since fixed in Phase A), which no prior doc had flagged.

---

## 1. DGII legal / e-CF compliance

**Confirmed accurate:**
- Ley 32-23 mandatory rollout calendar (Grandes Nacionales since May 2024; Grandes Locales/Medianos since 15 Nov 2025; Pequeños/Micro/No Clasificados extended to 15 Nov 2026 per DGII's 6 May 2026 aviso).
- e-CF types E31–E47 (Crédito Fiscal, Consumo, Notas Débito/Crédito, Compras/Proveedor Informal, Gastos Menores, Regímenes Especiales, Gubernamental, Exportaciones, Pagos al Exterior) — all match DGII's "Guía del Contribuyente No.5" and Informe Técnico e-CF v1.0.
- ITBIS rates: 18% general, 16% reducida (yogurt, mantequilla, café, azúcares, cacao/chocolate), exento (carne, leche, pan, salud, educación, transporte terrestre, alquiler de vivienda, servicios financieros) per Código Tributario Art. 343.
- RFCE / ARECF / ACECF acronyms and flows match DGII's own format PDFs almost verbatim.

**Wrong or overstated:**
- **"XAdES-BES" is a mislabel.** DGII's actual "Firmado de e-CF" spec is a plain XML-DSig enveloped signature (namespace `xmldsig#`, .NET's stock `SignedXml`) — it contains **no** XAdES elements (no `QualifyingProperties`, `SignedProperties`, `SigningCertificate`, `SigningTime`). The concrete algorithm URIs the repo implements (C14N 2001, rsa-sha256, sha256, enveloped-signature, `Reference URI=""`) do match DGII's spec exactly — only the "XAdES-BES" branding in docs/09 and the README was inaccurate and should be corrected to "XML-DSig enveloped signature (DGII e-CF spec)." The README was corrected in Phase A (see item 4); docs/09 still uses the "XAdES-BES" label.
- **`codigo_seguridad` derivation is ambiguous/unverified.** `sign_xml_ecf()` in `ecfl_service.rs` computes an unused `hasher2` over the base64 *string* of SignatureValue, but the value actually returned (`sig_hash_hex`) is SHA-256 of the raw signature *bytes*. Should be verified byte-for-byte against a real DGII test-environment round trip before any precertification attempt.
- **Reports 606/607/608/609/IT-1:**
  - No Format 607 generator — correctly justified in code comments: a 100%-e-CF issuer is exempt (Norma 07-2018 Art. 4 Párr. III).
  - No Format 608 generator — also correctly exempt for a fully-electronic issuer (assuming the system never falls back to traditional series-B NCF; unverified whether an offline mode could break this assumption).
  - **No Format 609 (Pagos al Exterior) generator anywhere**, and unlike 607/608 there is **no DGII exemption for 609** — this is a genuine, unaddressed compliance gap if this business ever pays a foreign supplier.
  - IT-1 (`report_service.rs::generate_it1`) deliberately omits the ITBIS-by-rate breakdown and retention/advance line items the real Anexo A requires — a simplified approximation, not the full form.
- **PSFE certification claim ("listo para certificación PSFE DGII") was overstated** — the claim was reworded in Phase A; see cross-cutting item 4.

**Other gaps found in code, not in the repo's own docs:**
- Payroll ISR withholding is hardcoded to zero (`nomina_service.rs`) regardless of salary — a real DGII/TSS withholding gap for employees above the exemption threshold.
- `ecf_documentos` stores signed XML for the 10-year retention requirement, but nothing enforces immutability (no WORM guarantee, no checksum-on-read) — for a PSFE-grade claim this should be demonstrably tamper-evident.
- Sequence allocation for E31/E32 (`EcfService::registrar_documento`) is solid: atomic `SELECT ... FOR UPDATE`, correct RNC-threshold validation, correct VENCIDA/AGOTADA handling. No issue here.

---

## 2. Retail POS workflow

**Strengths:**
- FIADO/credit accounts (`clientes.saldo_pendiente` + `limite_credito`) are well modeled and enforced server-side at sale time — arguably better than many "libreta de fiado" competitors in the DR market.
- Discount authorization: `usuarios.descuento_maximo_sin_aprobacion` gates cashiers, with a live admin-credential-approval modal when exceeded — good manager-override UX, matches mature POS patterns.
- Nota de Crédito correctly never mutates the original signed sale — it marks the venta `ANULADA` and creates a separate e-CF Tipo 34 document, which is the DGII-required pattern.

**Gaps:**
- **Returns/refunds are full-reversal-only** (v1, by explicit code comment in `create_nota_credito`) — no partial return, no line/quantity selection.
- **No split/mixed payment tender** — `metodo_pago` is a single enum per sale; can't pay part cash + part card.
- **No unit-of-measure conversion** — `productos.unidad_medida` is a single free-text field with no conversion table; selling the same SKU by libra, unidad, and caja (a core colmado behavior for rice, produce, sodas) requires manual duplicate SKUs.
- **No multi-almacén (multi-location) support** — self-acknowledged "Coming Soon" stub in `inventario/almacenes/page.tsx`; `productos.stock_actual` is a single column.
- **Single caja session per tenant, not per-cashier/terminal** — two cashiers cannot each have their own drawer/float simultaneously; variance is attributed to one shared session.
- **No price-override path for regular products** — the discount-approval mechanism wasn't extended to ad-hoc price changes (e.g. price-matching a damaged item).
- **No lot/expiry tracking** — no `lote`/`vencimiento` columns anywhere; relevant for perishables colmados commonly stock.
- **No fiado aging alerts** — competing DR-specific products (TuColmadoRD, MiColmado) flag overdue fiado at ~14 days; this repo has no equivalent surfaced in the UI.
- Cash reconciliation at caja close is lump-sum only (no denomination/bill-count breakdown); low-stock "alerts" are a passive dashboard count, not an actionable notification list.

---

## 3. Service operations (Órdenes de Servicio / Cotizaciones)

**Strengths:**
- Materials consumption (`consumir_material`) is transactional and correctly decrements real inventory via `InventarioService::apply_movimiento_tx`, tagged and blocked against double-counting or consuming after completion/cancellation.
- A completed orden bills through the exact same `create_venta` path as retail sales (`http_facturar_orden`), inheriting e-CF generation and ledger entries "for free," with double-billing blocked via a `venta_id.is_some()` check.
- Multi-technician assignment (`rol`: TECNICO_PRINCIPAL/ASISTENTE) works structurally.

**Gaps:**
- **No technician availability/conflict check** — `asignar_tecnico` validates only tenant/active-employee, nothing about scheduling conflicts. (New finding, deeper than the previously-known "no calendar" gap.)
- **No dispatch/calendar board** — `hora_inicio`/`hora_fin` columns exist in the DB but are unused by any UI; order lists just print the date as text.
- **No client-facing quote approval** (link/portal/e-signature) — 100% staff-mediated; `cotizaciones` API requires staff auth on every call. Already scoped as a deferred "Portal de cliente" subsystem in `docs/13`.
- **No notification/reminder service** (SMS/email on `PROGRAMADA`, "técnico en camino") — zero code references found.
- **Dead `ACEPTADA`/`VENCIDA` cotización states** — never written anywhere; no explicit accept step exists (staff either rejects or directly converts), and no auto-expiry job for stale quotes.
- **`equipos_cliente` (client equipment history) doesn't exist** — confirmed still true per `docs/13`'s own backlog.
- Minor: no stock-availability check when *planning* materials (`cantidad_planificada`) — though this matches the repo's own cited benchmark (QuickBooks Desktop also only does soft allocation on Sales Orders).
- **Note:** an in-progress worktree (`.worktrees/cotizacion-servicio-pricing`, 15 commits, not yet merged into `main`) is actively reworking cotización/orden pricing — any fix plan for the above should target whichever version is intended to ship.

---

## 4. Accounting & financial reports

**Strengths:**
- `create_entry` is the single choke point for every posting path (manual entries, all 9 `sincronizar` event loops, reversals); it rejects unbalanced/malformed entries and inserts atomically. Idempotency is DB-enforced via a unique constraint + `ON CONFLICT DO NOTHING`, closing a previously-known TOCTOU race. Covered by a real integration test (`services/core/tests/ledger_invariant.rs`).
- EWA/adelanto accounting matches the Netchex/DailyPay/Payactiv "advance against earned wages, not a loan" model exactly: debits an **asset** (Anticipos a Empleados), never a liability, no interest field anywhere in the schema.
- Caja apertura/cierre reconciliation against `caja_movimientos` is textbook and correctly implemented.

**Gaps:**
- ~~**Purchase-side Nota de Crédito/Débito bug**~~ — **FIXED in Phase A**; see cross-cutting item 1 (rows synced before the fix are not remediated).
- ~~**No equity accounts**~~ — **FIXED in Phase A** (accounts only); see cross-cutting item 2.
- **No Estado de Resultados (P&L) or Balance General (balance sheet) report exists anywhere** — only a flat Libro Mayor. Even once the equity gap is fixed, nothing renders the ledger as the two statements a contador or DGII actually asks for.
- **No Format 609 (Pagos al Exterior)** — confirmed required regardless of e-CF status, confirmed absent, unlike 607/608 which are correctly and explicitly justified as exempt.
- **Chart of accounts (`cuentas_contables`) is seeded but not enforced** — `create_entry` never validates a `cuenta` string against it; a typo in a manual journal entry creates a permanent, silent "ghost account."
- **No way to pay down accounts payable in-app** — `proveedores` has no `saldo_pendiente` column and no abono/payment endpoint exists, unlike the customer side (`cliente_abonos`); once a FIADO purchase posts, its liability can only be closed via a manual journal entry.
- **`sincronizar` is a manually-triggered batch job, not real-time** — every sale/purchase/expense/payroll/advance/bank movement is recorded in its own module's table instantly, but doesn't appear in the general ledger until a human clicks "Sincronizar." No cron/scheduled job calls it automatically.
- **Audit trail gap on the highest-risk write path**: manual journal entry creation (`http_create_asiento`), plus `http_create_compra`, `http_create_gasto`, and `http_create_banco_movimiento`, are not logged to the `auditoria` table — while sales, payroll, and reversal-critical actions are.
- Three independent "bank/cash balance" numbers exist (`caja_movimientos`/`banco_movimientos` real-time totals, `bancos.saldo`, and the GL's 1100/1160 accounts) with nothing cross-checking them; the GL also commingles all bank accounts into one line, precluding true per-account reconciliation.
- Adjacent, already self-documented as out of scope: ISR hardcoded to zero, and no accrual liabilities for regalía pascual/cesantía/vacaciones (payroll runs on a pure cash basis).

---

## 5. UI/UX

**Important scoping note:** `apps/web/app/staff/**` is an **internal vendor/reseller admin console** (login via a shared vendor secret; its own layout metadata literally says "no es el producto para clientes"). The actual colmado/PYME-facing product — POS, inventario, contabilidad, nómina, cotizaciones, órdenes de servicio, reportes DGII — lives under **`apps/web/app/(customer)/(dashboard)/**`**. This naming is genuinely confusing (it misled this very audit at first) and is itself worth fixing — e.g. rename to `(vendor-admin)` / `(tenant)`.

**Strengths:**
- A real shared design system across POS, cotizaciones, órdenes de servicio, nómina, contabilidad, and inventario (`useServerTable` + `ScrollableTableCard` + consistent badges/headers/buttons) — not independently-built modules.
- Good error recovery in the POS (distinguishes RNC-not-found vs. real network errors, handles a `CAJA_NO_ABIERTA` race gracefully).
- Deliberate, tenant-aware terminology (e.g. "Fiado" relabels to "A crédito" for `SERVICIOS`-type tenants) and a `data-negocio` theme override (teal vs. maroon) that doesn't leak across contexts.
- Focus-visible rings and proper `<label>`/`htmlFor` pairing are baked into the shared `packages/ui` component library by default.

**Gaps:**
- **No confirmation step on irreversible, money-moving actions** — nómina run, aprobar/rechazar adelanto, cancelar orden de servicio all fire on one click, inconsistent with the two-step confirm already used for producto/categoría deactivation.
- **`ventas/devoluciones` is a stub** — no user-facing way to search/manage returns (the only return path is the full-reversal nota de crédito button on a sale's own detail page).
- **No cash-tendered/change-due field in POS checkout** for Efectivo payments — cashiers must do that math themselves.
- **POS cart quantity-stepper touch targets are ~24×24px**, well under the ~44px minimum recommended for a touch terminal used under time pressure.
- **DGII 606/IT-1 reports show the raw pipe-delimited government TXT file as the primary result**, with no human-readable summary ("N compras, ITBIS acreditable RD$X") shown first.
- **`apps/mobile` is a non-functional static mockup** — hardcoded sample values, buttons with no `onPress` handlers, and a completely different visual language (raw inline styles, fixed dark theme) than the web app's shared token system. It does not implement the "POS móvil + adelantos empleado" feature the README advertises.
- Minor: icon-only buttons (trash/pencil/print) generally lack visible `aria-label`s; worth a follow-up screen-reader pass.

---

## Prioritized punch list (cross-domain)

1. ~~Fix purchase-side Nota de Crédito/Débito ledger + inventory treatment~~ — **FIXED in Phase A** (purchase `NOTA_CREDITO` only; `NOTA_DEBITO` unchanged; rows synced before the fix are not remediated). See cross-cutting finding #1 above.
2. ~~Add Patrimonio/equity accounts to the chart of accounts~~ — **FIXED in Phase A** (accounts `3100`/`3200` only; no balance sheet/P&L or period close yet). See cross-cutting finding #2 above.
3. ~~Confirm whether the mocked `send_to_dgii()` path is reachable from the live sale flow~~ — **RESOLVED, not a bug.** See the corrected cross-cutting finding #3 above.
4. ~~Correct or remove the 608/609/PSFE-readiness marketing claims until they're actually true~~ — **FIXED in Phase A** (copy only; Format 609 still not implemented, PSFE certification still not achieved). See cross-cutting finding #4 above.
5. Build a real returns/void flow (even partial-only internally, ahead of full DGII nota-de-crédito nuance) and give it a working `devoluciones` UI (retail + UI/UX, critical).
6. Add technician conflict detection and a basic dispatch view for `ordenes_servicio` (service ops, critical).
7. Add confirmation steps to nómina run, adelanto approval, and orden cancellation, reusing the existing two-step pattern (UI/UX, critical, cheap fix).
8. Build Estado de Resultados / Balance General reporting once equity accounts exist (accounting, high).
9. Implement Format 609, or explicitly document that this POS is scoped to never handle foreign-supplier payments (DGII compliance, high).
10. Add unit-of-measure conversion for products sold by libra/unidad/caja (retail, high — very common colmado behavior).
11. Enforce `cuenta` against `cuentas_contables` on every ledger write; close the audit-trail gaps on manual journal entries, compras, gastos, and bank movements (accounting, moderate).
12. Add a proveedor-side payment/abono mechanism; automate or surface staleness of `sincronizar` (accounting, moderate).
13. Add cash-tendered/change-due to POS checkout; enlarge cart qty-stepper touch targets; give DGII reports a human-readable summary before the raw TXT (UI/UX, moderate).
14. Resolve the `staff` vs `(customer)` route-group naming confusion (UI/UX, moderate).
15. Multi-cajero/multi-almacén support, split tender, lot/expiry tracking, fiado aging alerts, client equipment history, client-facing quote approval portal — larger scope items, correctly already understood as future work in most cases (retail + service ops, moderate/lower — sequence per business need).
16. Treat `apps/mobile` as unbuilt for planning purposes; either restart it on shared `@repo/ui` tokens or scope it explicitly as mock-only (UI/UX, moderate).
17. Real ISR withholding calculation; DR labor-law accrual liabilities (regalía pascual, cesantía, vacaciones) (payroll/accounting, minor — pre-existing known limitation).

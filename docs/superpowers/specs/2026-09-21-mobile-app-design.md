# Mobile app: current state, options and recommendation (Phase G — design only)

Date: 2026-09-21 · Status: **decision document, nothing implemented** · Audience: product + eng team

This phase deliberately ships **no code**. `docs/14-COMPLIANCE-WORKFLOW-UIUX-AUDIT.md:126` flags `apps/mobile`
as "a non-functional static mockup", and punch-list item 16 (`docs/14-...:148`) says to either restart it or
scope it as mock-only. That is a product decision with a multi-week price tag, so this document gives the team
the facts and the trade-offs instead of guessing. Every claim below was re-verified against the code on this
branch (base `524a78f`).

---

## 1. Current state, as facts

### 1.1 What `apps/mobile` actually contains

The whole app is **two files**:

| File | Lines | Content |
|---|---|---|
| `apps/mobile/package.json` | 25 | deps + `expo start` scripts |
| `apps/mobile/src/index.tsx` | 69 | one static screen |

There is **no** `app.json`/`app.config.js`, no `tsconfig.json`, no `babel.config.js`, no `metro.config.js`,
no `assets/`, no icon/splash, no `index.js` calling `registerRootComponent`, no tests, no navigation library,
no state management, no `.env` handling, no EAS config. `apps/mobile/package.json:5` sets
`"main": "src/index.tsx"`, and that module only default-exports a component — nothing registers a root
component. **This project cannot start**; it is a screenshot rendered as JSX, not an app.

`apps/mobile/src/index.tsx` in detail:
- Every number is hardcoded: `RD$ 6,400.00` (`:20`), `RD$ 3,200.00` (`:26`), `RD$ 341.34` (`:58`),
  `E320000000129` (`:42`), `46 facturas ACEPTADAS` (`:64`), employee "Juan Carlos • Almacén" (`:17`).
- Both `TouchableOpacity` (`:30`, `:57`) have **no `onPress`** — the two headline buttons ("Solicitar Adelanto",
  "Cobrar … Generar E32 … QR DGII") do nothing.
- Zero imports from `@repo/api-client` or `@repo/ui` despite both being declared deps
  (`apps/mobile/package.json:16-17`); the file header comment at `:4` claims "Comparte @repo/api-client" and
  `:3` claims "Conecta al mismo núcleo Rust vía gRPC". Neither is true — the only import is from
  `react-native` (`:7`).
- `react-native-paper` and `expo-secure-store` (`:18-19`) are installed and never imported.
- Visual language is raw inline styles on a fixed `#09090b` dark theme, unrelated to the web app's
  Tailwind token system (`apps/web/app/(customer)/globals.css`, `packages/ui/src/*`).

### 1.2 Wiring into the monorepo

- **pnpm workspace: yes.** `pnpm-workspace.yaml:2` globs `apps/*`, and `pnpm-lock.yaml:22-51` has a real
  `apps/mobile` importer with resolved versions: `expo@52.0.49`, `react-native@0.76.5`, `react@18.3.1`,
  `react-native-paper@5.15.3`, `expo-secure-store@14.0.1`. So every `pnpm install --frozen-lockfile` — including
  CI at `.github/workflows/e2e.yml:51` — downloads the full Expo/Metro/Babel toolchain for a dead app.
- **Turborepo: effectively no.** `turbo.json:6-22` defines `build`, `dev`, `lint`, `type-check`, `test`.
  `apps/mobile/package.json:6-11` declares only `start`, `android`, `ios`, `dev`. So `turbo run build`,
  `lint`, `type-check` and `test` all **silently skip** the package. Nothing ever compiles it.
- **CI: no.** `.github/workflows/e2e.yml` builds only `pnpm --filter web build` (`:124`) and runs Cypress
  against web (`:141`). `deploy-cloud-run.yml` deploys the Rust core. Mobile is never built, typechecked,
  linted, or tested anywhere.
- **Version hazard.** Root `package.json:36` pins `pnpm.overrides["@types/react"]: "^19.0.3"`, which
  resolves `@types/react@19.2.17` into `apps/mobile` (`pnpm-lock.yaml:46-48`) while `react` there is `18.3.1`.
  Anyone who adds a `tsc` step will hit that mismatch on day one.
- **Expo SDK 52 is stale.** Expo's own version table now lists SDK 54–57 as current
  ([docs.expo.dev/versions/latest](https://docs.expo.dev/versions/latest/)); SDK 55 shipped 2026-02-25 with
  RN 0.83 ([changelog](https://expo.dev/changelog/sdk-55)). SDK 52 is from Nov 2024. Also relevant to any
  "just run it in Expo Go" plan: App Store Expo Go has been stuck on SDK 54 since May 2026 and Expo now
  explicitly steers non-beginners to development builds
  ([changelog](https://expo.dev/changelog/expo-go-and-app-store-may-2026)). Restarting means an SDK 52 → 57
  jump plus an EAS build/credential setup that does not exist today.

### 1.3 `packages/api-client` — stale and unused

`packages/api-client/src/index.ts` (66 lines) exports four functions. Three of them call **endpoints that do
not exist** in the Rust core:

| api-client call | Real route in `services/core/src/main.rs` |
|---|---|
| `GET /v1/employees/:id/balance` (`:36`) | ✗ — real: `GET /v1/empleados/:id` (`main.rs:626`) |
| `POST /v1/advances/request` (`:46`) | ✗ — real: `POST /v1/nomina/adelantos` (`main.rs:627`) |
| `POST /v1/payroll/run` (`:57`) | ✗ — real: `POST /v1/nomina/run` (`main.rs:632`) |
| `POST /v1/ecf/sign` (`:26`) | ✓ `main.rs:672` — but gated by permission `ecf.dev_tools` (`main.rs:392`) |

It also never sends an `Authorization` header, so even the one real route would 401 behind
`permission_guard` (`main.rs:112-150`, applied at `main.rs:686`). It reads `process.env.NEXT_PUBLIC_*`
(`:6-7`), which is a Next.js convention, not an Expo one. It is **imported by nothing**: grep for
`@repo/api-client` across `apps/` and `packages/` returns only the two `package.json` declarations
(`apps/web/package.json:20`, `apps/mobile/package.json:16`) and the dead comment in
`apps/mobile/src/index.tsx:4`. The web app talks to the core through its own BFF (`apps/web/lib/api.ts`,
`apps/web/lib/core-proxy.ts`).

**Treat `packages/api-client` as dead code, not as a shared foundation.**

### 1.4 `packages/ui` — web-only, not reusable on React Native

Every component is DOM + Tailwind: `packages/ui/src/button.tsx:1-20` builds `className` strings via
`class-variance-authority` and renders `<button>`; `packages/ui/src/utils.ts` is `clsx` + `tailwind-merge`;
`packages/ui/package.json:11-16` depends on `react-dom` and `lucide-react`. **None of it runs on React
Native.** Only
`formatDOP` (`utils.ts:6-9`, pure `Intl`) is portable. The *design tokens* (CSS variables in
`apps/web/app/(customer)/globals.css`) are reusable as values, but the components are not.

### 1.5 Where mobile is advertised (do not edit these files in this phase — proposed wording below)

| Location | Exact claim today | Proposed replacement |
|---|---|---|
| `README.md:14` | `mobile/       -> Expo React Native (POS móvil + adelantos empleado)` | `mobile/       -> Maqueta estática (no funcional). Ver docs/superpowers/specs/2026-09-21-mobile-app-design.md` |
| `README.md:84` | `**Móvil:** Empleado ve "Ganado hoy RD$6,400 • Disponible RD$3,200 (50%)" + botón "Solicitar Adelanto RD$2,000" -> gRPC PayrollService/RequestAdvance` | Delete the line. There is no `PayrollService` gRPC service and no employee-facing endpoint. |
| `README.md:86` | `Archivo: apps/web/app/page.tsx y apps/mobile/src/index.tsx - 100% español.` | `Archivo: apps/web/app/(customer)/page.tsx` — `apps/web/app/page.tsx` no longer exists. |
| `README.md:27` | `proto/  -> fiscal.proto + payroll.proto` | Delete — `packages/proto/` does not exist (`ls packages/` → `api-client`, `ui`). |
| `apps/web/app/(customer)/page.tsx:48` | "…tú apruebas **desde el celular**, la nómina descuenta sola." | "…lo apruebas desde el sistema y la nómina lo descuenta sola." |
| `…/page.tsx:122` (FAQ) | "…el empleado lo pide **desde el celular**, tú apruebas…" | "…el empleado lo solicita, tú lo apruebas, y se descuenta solo en la próxima nómina." |
| `…/page.tsx:126` (FAQ) | "**La app móvil es opcional** para que tus empleados vean su disponible de adelanto." | "Todo corre en el navegador del celular, la tablet o la computadora — no hay que instalar nada." |
| `…/page.tsx:312-313` | "…tú apruebas **desde el celular**…" / bullet "Aprobación **en un toque desde el celular**" | "…lo apruebas en un clic…" / "Aprobación en un clic, con confirmación" |
| `…/page.tsx:118` (FAQ, adjacent) | "El terminal POS **sigue vendiendo localmente** y sincroniza los comprobantes en cuanto vuelve la conexión." | **Also false today** — there is no offline mode anywhere (§3, hard constraints). Replace with the truthful contingency story: "Si DGII no responde, la venta se completa igual y el comprobante queda en *pendiente de envío* para reintentarse (`CONTINGENCIA_PENDIENTE`). Si se cae tu internet, el sistema no vende hasta que vuelva." |

Only the last one is arguably outside "mobile"; it is listed because it is the claim most likely to be
believed by a colmado owner on a bad-internet street, and it is wrong for the same architectural reason the
mobile POS is hard.

---

## 2. What mobile *should* be — candidate roles

The product has two tenant types, set only by the vendor console
(`services/core/src/services/staff_service.rs:98-111`, values `COLMADO` | `SERVICIOS`,
`auth_service.rs:39-43`). Authorization is JWT (12h, `auth_service.rs:138`) + a permission catalogue of 25
codes (`services/core/src/bin/migrate.rs:1148-1174`) mapped to routes by `required_permiso`
(`main.rs:382-422`).

**A. Employee self-service (view earnings, request/track advances).** This is what the README and landing
page promise. Three problems — the first two structural, the third a truth-in-advertising one:
1. **Employees are not users.** `empleados` (`migrate.rs:370-380`) has *no* `usuario_id`, and `usuarios`
   (`migrate.rs:77-88`, + `ALTER`s at `:90`, `:1144-1145`) has no `empleado_id`. An employee literally cannot
   log in as themselves. This needs a new link column, an invite/onboarding flow, and a new `EMPLEADO` role.
2. **The permission model has no self-scope.** `main.rs:415` maps *all* of `/v1/empleados` and `/v1/nomina`
   to the single permission `nomina.gestionar`. Granting an employee access to their own balance today would
   also grant them `POST /v1/nomina/run` (`main.rs:632`) and `POST /v1/nomina/adelantos/:id/aprobar`
   (`main.rs:628`) — i.e. run payroll and approve their own advance. A real self-service app requires new
   `nomina.mis_datos`-style permissions plus per-row `empleado_id == claims.empleado_id` enforcement.
3. Honesty check on the promise itself: `disponible_adelanto`
   (`nomina_service.rs:244-254`) is `salario_mensual × 0.5 − adelantos PENDIENTE/APROBADO`. It is **not**
   "ganado hoy" — there is no time/attendance data in the system at all. The mockup's "Horas: 8h • Tarifa:
   RD$800" (`apps/mobile/src/index.tsx:21`) and the landing page's "cuánto han ganado hoy"
   (`page.tsx:312`) describe a feature that does not exist and is not on any plan. Shipping employee
   self-service does **not** by itself make that claim true.

Applies to both tenant types. Highest emotional value (this is the differentiator the marketing leads with),
lowest technical reuse.

**B. Cashier POS móvil.** Only meaningful for `COLMADO`. It would call `POST /v1/ventas` (`main.rs:570`) and
`POST /v1/ventas/:id/emitir-ecf` (`main.rs:572`) with `ventas.gestionar` + `caja.gestionar` — permissions
`CAJERO` already has (`migrate.rs:1188-1190`). See §3 for why the hard constraints make this the *worst* first
slice despite the endpoints existing.

**C. Owner dashboard (sales / caja / alerts).** `GET /v1/reports/dashboard` (`main.rs:647`) and
`GET /v1/ai/digest` (`main.rs:648`) fall through `required_permiso` to `None` (`main.rs:420`), i.e. any
authenticated user; `GET /v1/caja/resumen` (`main.rs:617`) needs `caja.gestionar` (`main.rs:399`); approving
advances needs
`nomina.gestionar` (`main.rs:628`). **An owner already has all of this today** — an ADMIN's `es_admin`
bypasses every permission check (`roles_service.rs:146`). So role C is a *presentation* problem, not an
API problem: it is read-mostly plus two one-tap writes (aprobar/rechazar adelanto), and it needs zero core
changes. Applies to both tenant types.

**D. Nothing.** Delete `apps/mobile`, fix the copy, keep the promise off the market until someone asks for it.

---

## 3. Hard constraints that make a phone POS expensive

These are specific to this product and are why option (b) below beats option (a) for a POS role.

1. **The e-CF signing certificate must stay on the server.** The tenant's `.p12` is uploaded via
   `POST /v1/config/certificado` (`main.rs:656`), validated, AES-256-GCM encrypted with `CERT_ENCRYPTION_KEY`
   and stored in `certificados_dgii` (`config_service.rs:432-469`); signing decrypts it in-process
   (`config_service.rs:484-500` → `ecfl_service::load_p12`). Putting that private key on a cashier's phone
   would mean one INDOTEL-recognised signing key per handset, each one a loss/theft/root incident — and the
   signature is what makes the invoice legally valid
   ([INDOTEL, servicios electrónicos de confianza](https://indotel.gob.do/firma-digital/servicios-electronicos-de-confianza/)).
   **Conclusion: a mobile POS must sign server-side. It is a thin client, not an offline signer.**
2. **Fiscal sequence allocation is server-side and transactional.** `allocar_siguiente_ncf`
   (`ecf_service.rs:93-136`) takes `SELECT … FOR UPDATE` on `secuencias_ncf`, never reuses a number, and marks
   sequences `VENCIDA`/`AGOTADA`. A device cannot pre-allocate e-NCFs offline without risking duplicate or
   out-of-order fiscal numbers.
3. **There is no offline mode anywhere, and contingencia is not it.** Grep across `services/core/src` and
   `apps/web` finds contingency only as a *DGII transmission* fallback: if the DGII endpoint fails the sale
   still completes and the document parks in `CONTINGENCIA_PENDIENTE`
   (`ecf_service.rs:13-15`, `main.rs:2304`, `:2378-2380`) for retry via
   `POST /v1/ecf/pendientes/reintentar` (`main.rs:587`). If the *tenant's own* internet is down, nothing works
   — the sale never reaches `POST /v1/ventas`. True offline POS would require local sequence leasing, local
   ledger staging and conflict resolution: a project in its own right, unrelated to the choice of client.
4. **One caja session per tenant.** `caja_service.rs:63-72` selects `WHERE tenant_id = $1 AND estado='ABIERTA'
   … LIMIT 1` and `:101-103` refuses a second open session. `ventas_service.rs:156-176` rejects any sale for a
   `COLMADO` tenant with `CAJA_NO_ABIERTA` when none is open. So a phone POS **shares one cash drawer** with
   the web POS — two cashiers taking cash into the same session, and whoever closes it
   (`caja_service.rs:118-126`) reconciles both. Multi-cajero/multi-caja is already known future work
   (`docs/14-...:147`, item 15). **A phone POS without it creates a cash-control problem, not a feature.**
5. **Printing is already server-driven.** Thermal printing goes through a network printer configured per
   tenant (`config_service.rs:142-152` → `ip`/`puerto`; `POST /v1/ventas/:id/imprimir`, `main.rs:573`). A
   phone does not need Bluetooth printing to be useful — one more reason a thin client suffices.

---

## 4. Options with honest trade-offs

Assumptions for every estimate: **one full-time senior engineer**, familiar with this codebase, no designer,
no QA; estimates are build-to-first-real-user, excluding store review latency; "person-weeks" = 5-day weeks.

### Option (a) — Rebuild the Expo/React Native app on shared tokens

| | |
|---|---|
| **What it is** | Delete the mockup, scaffold Expo SDK 57 + expo-router, port the design tokens to a RN theme, build a native token store on `expo-secure-store`, write a typed HTTP client against the real `/v1` routes, ship via EAS Build to TestFlight + Play internal track. |
| **Effort** | **9–14 person-weeks** for role A or C alone. Breakdown: toolchain/EAS/credentials + Apple & Google developer accounts 2w; RN token/component layer (there is nothing to reuse — §1.4) 2–3w; auth + secure storage + 12h-token refresh UX 1w; the feature screens 2–3w; the backend work role A needs (§2.A: `empleado↔usuario` link, new permissions, self-scoped endpoints, invite flow) 2–3w; store submission, device testing, release process 2w. Role B on top: +4–6w (§3 constraints, especially multi-caja). |
| **Reuses** | Design *token values* only. Some Spanish copy. Nothing else — `packages/ui` is DOM-only, `packages/api-client` is dead. |
| **Risks** | Second UI system to keep in sync with web forever (every new field, every terminology change — e.g. the `COLMADO`/`SERVICIOS` "Fiado"→"A crédito" relabel noted in `docs/14-...:117` — must be done twice). App-store review cycles for every fix. SDK upgrades ~3×/year, each a day-to-week of churn. Requires an Apple Developer account (US$99/yr) and a Google Play account, plus someone who owns signing credentials. Expo Go is not a shipping path any more (§1.2), so a dev build pipeline is mandatory from day one. |
| **Maintenance** | **High and permanent.** Realistically ~1 engineer-day/week steady state, plus forced SDK/OS upgrades. |
| **When it wins** | Only if you need something the web cannot do: barcode scanning at speed, offline operation, reliable push notifications on iOS, or a native app-store presence as a sales asset. |

### Option (b) — Make the existing Next.js app an installable, responsive PWA

| | |
|---|---|
| **What it is** | Add `manifest.webmanifest` + icons (there is no `apps/web/public/` directory at all today), a `viewport`/`theme-color` export in `apps/web/app/(customer)/layout.tsx` (currently only `metadata`, `:19-23`), a minimal service worker for shell caching and a clear offline *error* screen (not offline selling — §3.3), an "Instalar app" hint, and a mobile UX pass on the screens that matter. |
| **Effort** | **2–4 person-weeks** for role C (owner dashboard) or role A's *UI* — plus the same 2–3w of backend work if role A is chosen (§2.A), which is unavoidable under any option. |
| **Reuses** | **Everything**: auth (`apps/web/lib/api.ts`, `middleware.ts:35-40`), the BFF proxy, `packages/ui`, the token/theme system, the Cypress suite, the CI pipeline, the deploy. |
| **What already works** | The dashboard is genuinely responsive: sidebar collapses to a drawer (`apps/web/app/(customer)/(dashboard)/layout.tsx:326,328,398,408`), and the POS reflows to a single column with a collapsible cart on phones (`…/pos/page.tsx:376-436`). Phase C already enlarged the cart touch targets and added cash-received/change-due. **A cashier can already use the POS from a phone browser today.** |
| **Risks** | iOS is the weak spot: install is manual (Safari has no `beforeinstallprompt`), Background Sync is unsupported, and Web Push requires the user to have added the app to the Home Screen first — no silent push, no background wake ([MagicBell](https://www.magicbell.com/blog/pwa-ios-limitations-safari-support-complete-guide), [MobiLoud](https://www.mobiloud.com/blog/progressive-web-apps-ios/)). So "notify the owner that an advance was requested" is *not* reliably deliverable on iPhone. No app-store listing. A badly-scoped service worker can serve stale JS after a deploy — keep it shell-only and network-first for `/api/*`. |
| **Maintenance** | **Near zero.** One codebase, one CI, one deploy. |

### Option (c) — Drop the mobile promise, fix the marketing

| | |
|---|---|
| **What it is** | Delete `apps/mobile` and `packages/api-client`, apply the copy changes in §1.5, note in `README.md` that the product is browser-based and mobile-responsive. |
| **Effort** | **0.5–1 person-week**, entirely copy + deletion + one regression run. |
| **Reuses** | n/a. |
| **Risks** | Sales loses a talking point ("app móvil"). If any prospect was sold on it, that has to be walked back. Deleting `apps/mobile` also drops the whole Expo/Metro/Babel dependency tree from every `pnpm install` — CI and every new checkout (time saved not measured). |
| **Maintenance** | Negative — removes maintenance. |

---

## 5. Recommendation

**Do (c) now, then (b) scoped to the owner dashboard. Do not do (a).**

Reasoning:

1. **The claims are false today and that is the urgent part.** Phase A already corrected the 608/609/PSFE
   marketing claims (`docs/14-...:136`) for exactly this reason; the mobile and offline claims are the same
   class of problem and are still live in `README.md:14,84` and `apps/web/app/(customer)/page.tsx:48,118,122,126,312,313`.
   Copy fixes cost days and stop the bleeding regardless of which build option is chosen later.
2. **The phone use case is already largely served.** The dashboard and POS are responsive (§4b). What is missing
   is installability, a home-screen icon, and a mobile-first *dashboard*, not a second app.
3. **A native app buys almost nothing here.** The certificate stays on the server (§3.1), sequence allocation
   stays on the server (§3.2), offline is out of scope for both options (§3.3), and printing is already
   server-driven (§3.5). The only genuine native advantages — reliable iOS push and barcode scanning — are not
   in the MVP.
4. **A phone POS is the wrong first slice.** It is blocked behind multi-caja/multi-cajero (§3.4), which is
   already parked as future work. Shipping it first would create a cash-control regression.
5. **Employee self-service is a backend project wearing a frontend costume.** §2.A items 1–2 must be built
   before any client exists, and item 3 means the headline "ganado hoy" claim stays false until time/attendance
   exists. Both options pay that cost identically, so the client choice should not be made on its account.

### MVP slice — "Panel del dueño, instalable" (option b, role C)

**Ships:**
- `apps/web/public/manifest.webmanifest` + maskable icons; `viewport`/`theme-color` exports in
  `apps/web/app/(customer)/layout.tsx`; an "Instalar app" prompt (with an iOS-specific "Compartir → Añadir a
  pantalla de inicio" hint, since Safari has no install event).
- A mobile-first `/dashboard` reading `GET /v1/reports/dashboard` (`main.rs:647`) and
  `GET /v1/caja/resumen` (`main.rs:617`): ventas del día, caja abierta/cerrada + saldo, adelantos pendientes,
  e-CF en `CONTINGENCIA_PENDIENTE` (`GET /v1/ecf/documentos?estadoDgii=CONTINGENCIA_PENDIENTE`, as the existing
  `configuracion/dgii/documentos` page already does at `:79`).
- One-tap aprobar/rechazar adelanto from that screen, reusing the Phase C confirm dialog.
- A shell-only service worker: cache the app shell and static assets; **never** cache `/api/*`; show an explicit
  "Sin conexión — no se puede vender ni facturar" screen rather than pretending to work offline.

**Explicitly deferred (write this down so nobody assumes it):** offline sales; any notification/push; a phone
POS; employee self-service login; barcode scanning; app-store distribution; anything requiring the certificate
on the device.

### Phased plan with acceptance criteria

| Phase | Work | Acceptance |
|---|---|---|
| **G1 — Truth** (0.5–1 wk) | Apply §1.5 copy; delete `apps/mobile` and `packages/api-client`; drop `@repo/api-client` from `apps/web/package.json:20`; refresh the lockfile. | `grep -ri "app móvil\|desde el celular\|sigue vendiendo localmente" apps/web README.md` returns nothing. `pnpm install --frozen-lockfile` succeeds and the lockfile no longer has an `apps/mobile` importer. `pnpm --filter web build` + the full Cypress suite green. |
| **G2 — Installable** (1 wk) | Manifest, icons, viewport, service worker, install hint. | Lighthouse "Installable" passes on Android Chrome. Installed app opens standalone with the right icon/name. After a deploy, a hard reload serves the new build (no stale-asset bug). Offline → the explicit offline screen, never a blank page. Existing Cypress suite still green. |
| **G3 — Panel del dueño** (1–2 wk) | The mobile-first dashboard + one-tap adelanto actions. | New Cypress spec at 390×844 viewport: login → dashboard shows ventas del día, estado de caja, adelantos pendientes, e-CF pendientes; approving an adelanto asks for confirmation and updates the list. All touch targets ≥44px. Works for both `COLMADO` and `SERVICIOS` tenants (caja tile hidden for `SERVICIOS` — `ventas_service.rs:166`). |
| **G4 — decision gate** | Re-evaluate. Only if G3 is in real use and the team still wants employee self-service: build the backend (`empleado↔usuario`, `EMPLEADO` role, self-scoped `nomina.mis_datos` permissions, invite flow) *first*, then add the screens to the same PWA. | A logged-in employee can read only their own balance and adelantos, and a test proves they get 403 on `POST /v1/nomina/run` and on approving any adelanto. |

---

## 6. Open questions for the team

1. **Who is the mobile user?** Owner, cashier, or employee? Everything above changes with the answer. This
   document recommends *owner*; that is a guess about the business, not a fact from the code.
2. **Is the "app móvil" promise load-bearing in any signed deal or pitch deck?** If yes, option (c) needs a
   customer-communication plan, not just a copy edit.
3. **Is anyone willing to own app-store accounts, signing credentials and release cadence?** If the answer is
   no, option (a) is off the table regardless of its merits.
4. **Does "ganado hoy" (earned today) matter enough to build time/attendance?** Without it the 50% rule stays
   `salario_mensual × 0.5` (`nomina_service.rs:251`) and the marketing line stays untrue.
5. **When is multi-cajero / multi-caja scheduled?** A phone POS is blocked on it. It is currently item 15 on
   the punch list (`docs/14-...:147`).
6. **Is offline selling a real requirement for Dominican colmados, or a story?** It is the single most
   expensive item discussed here and it is orthogonal to native-vs-web.
7. **Delete or archive `apps/mobile`?** Deleting removes a misleading artifact and install weight; keeping it
   preserves the visual reference someone once liked. Recommendation: delete it — §1.1 above records what it
   contained, and `git log` keeps the file itself recoverable.

## 7. Risk if the answer is "keep the claims as they are"

- **Misrepresentation to customers.** `README.md:14` and the landing FAQ (`page.tsx:126`) tell a prospect an
  app exists; `page.tsx:118` tells them the POS keeps selling with no internet. Neither is true. In a market
  where the buyer is a small-business owner choosing a fiscal system, a discovered false claim about *offline
  selling* is worse than not having the feature — it is the scenario they will test.
- **A trap for the next engineer.** The declared `@repo/api-client` dependency and the "Conecta al mismo
  núcleo Rust vía gRPC" comment (`apps/mobile/src/index.tsx:3-4`) will make someone believe there is a working
  client to build on. There isn't; three of its four endpoints don't exist (§1.3).
- **Silent rot.** Nothing builds, typechecks or tests `apps/mobile` (§1.2), so it will keep drifting while
  looking like a real workspace package, and the `react@18` / `@types/react@19` mismatch will greet whoever
  tries.
- **Planning error.** Anyone estimating "finish the mobile app" from the README will price a 1-week polish job
  for what is a 9–14 week build (§4a).

---

**Sources consulted for the non-code claims:**
[Expo SDK versions](https://docs.expo.dev/versions/latest/) ·
[Expo SDK 55 changelog](https://expo.dev/changelog/sdk-55) ·
[Expo Go and the App Store, May 2026](https://expo.dev/changelog/expo-go-and-app-store-may-2026) ·
[PWA iOS limitations and Safari support (2026)](https://www.magicbell.com/blog/pwa-ios-limitations-safari-support-complete-guide) ·
[Do PWAs work on iOS? (2026)](https://www.mobiloud.com/blog/progressive-web-apps-ios/) ·
[INDOTEL — servicios electrónicos de confianza](https://indotel.gob.do/firma-digital/servicios-electronicos-de-confianza/) ·
[DGII — documentación sobre e-CF](https://dgii.gov.do/cicloContribuyente/facturacion/comprobantesFiscalesElectronicosE-CF/Paginas/documentacionSobreE-CF.aspx)

**Not verified:** effort estimates are judgement, not measurement. No device or browser testing was performed
in this phase (no code was written). The INDOTEL/DGII sources confirm that the signing certificate is what
makes an e-CF legally valid and that private-key custody is the holder's responsibility; **no source was found
that explicitly prohibits storing a certificate on a mobile device** — the §3.1 conclusion is an operational
risk argument, not a cited legal prohibition, and should be confirmed with the team's DGII/PSFE advisor before
being quoted to a customer.

# Phase E — Técnico scheduling & conflict detection (design note)

Date: 2026-09-21 · Branch `feat/phase-e-technician-scheduling` · Audit refs: `docs/14` cross-cutting #6 +
§3 "Gaps", `docs/13` "Programación / Dispatch board".

## Verified against current code (not just the audit)

- `orden_servicio_service.rs::asignar_tecnico` (line ~578) validates tenant + `empleados.activo` and inserts.
  **Zero** schedule checking. Confirmed.
- `ordenes_servicio.hora_inicio` / `hora_fin` are `TIME` columns (`migrate.rs` ~852-853). `UpdateOrdenServicioRequest`
  already accepts both (`COALESCE` update); `CreateOrdenServicioRequest` does **not**.
- No UI reads or writes them: `nueva/page.tsx` has a bare `type="date"` input; `[id]/page.tsx` renders
  `fecha_programada` read-only and has **no** way to change it at all. Confirmed.
- Multi-técnico (`TECNICO_PRINCIPAL` / `ASISTENTE`) is a plain insert into `orden_servicio_tecnicos` with no
  uniqueness constraint. Confirmed.
- Base already contains PR #9 (pricing rework) and Phase C's cancel `ConfirmDialog`.

## Scope slice

E1 backend conflict detection · E2 `GET /v1/ordenes-servicio/agenda` · E3 Spanish UI (time inputs, 409 override
through `ConfirmDialog`, a plain agenda list page). Nothing else.

## Semantics (the product decisions)

A técnico is *occupied* by an orden when that orden has a `fecha_programada` **and both** `hora_inicio` and
`hora_fin`, and its `estado` is not `CANCELADA` / `COMPLETADA`.

1. **Overlap rule** — half-open `[hora_inicio, hora_fin)` on the same `fecha_programada`:
   `a.inicio < b.fin AND b.inicio < a.fin`. Adjacent ranges (09:00-11:00 then 11:00-13:00) do **not** conflict.
2. **NULL times** — an orden with a date but no complete hour range is a *pending, unslotted* job, not a booked
   slot. It never produces a hard 409 in either direction. The successful response instead carries a soft
   `avisos[]` ("same-day notice") so the dispatcher sees the técnico already has work that day.
3. **Zero-length / inverted ranges** — rejected at write time with a Spanish 400 (`hora_fin` must be strictly
   after `hora_inicio`). This removes the "does `[t,t)` overlap anything?" ambiguity at the source instead of
   defining a special case. Consequence: an overnight range (22:00→02:00) cannot be stored — it is not
   representable in two same-day `TIME` columns anyway. Listed as a limitation, not solved here.
4. **Soft rule + override** — a conflict returns **409** with the conflicting orders (código, cliente, fecha,
   horas). Passing `confirmar_conflicto: true` performs the write anyway and writes an `auditoria` row
   (`ORDEN_SERVICIO_CONFLICTO_AGENDA_OMITIDO`) naming the conflicting orders. Two short jobs in one slot is a
   legitimate dispatch decision; hard-blocking it would be wrong.
5. **Both write paths are checked** — assigning a técnico (new empleado vs. this orden's slot) *and* `PATCH`ing
   an orden's `fecha_programada`/`hora_inicio`/`hora_fin` (this orden's already-assigned técnicos vs. their
   other orders). Multi-técnico orders are unaffected: each assignment is checked independently, and two
   técnicos on the *same* orden never conflict with each other (the query excludes the orden itself).

## API changes

- `POST /v1/ordenes-servicio` — `CreateOrdenServicioRequest` gains `hora_inicio` / `hora_fin` (optional).
- `POST /v1/ordenes-servicio/:id/tecnicos` — body gains `confirmar_conflicto: bool` (default false).
  On conflict: `409` with JSON `{ error, conflictos: [{ orden_id, codigo, cliente_nombre, fecha_programada,
  hora_inicio, hora_fin, estado, empleado_id, empleado_nombre }] }`. Success: `{ ...asignacion, avisos: [...] }`.
- `PATCH /v1/ordenes-servicio/:id` — same `confirmar_conflicto` flag and the same 409 shape.
- `GET /v1/ordenes-servicio/agenda?desde=&hasta=[&empleado_id=]` — required ISO dates, range capped at 62 days.
  Returns the repo's standard `Page<T>` envelope with `page`/`pageSize`/`sortBy`/`sortDir`, sortable on
  `fecha_programada`, `hora_inicio`, `prioridad`, `estado`; default `fecha_programada ASC, hora_inicio ASC`.
  Each row: orden id, código, cliente, estado, prioridad, fecha, horas, total, and its técnicos
  (`[{ empleado_id, nombre, rol }]`). `CANCELADA` orders are excluded (an agenda is a work plan).
  Gated by the existing `ordenes_servicio.gestionar` permiso via the `/v1/ordenes-servicio` prefix — no new
  permiso, no `required_permiso` edit. Route is registered next to the other ordenes-servicio routes;
  the static `agenda` segment wins over `/:id` in matchit (same pattern as `/v1/tenants/me/modulos`).

## Data model

No new tables, no new columns — `hora_inicio`/`hora_fin` already exist. One additive, idempotent block in
`migrate.rs` inside the MODULO 15 section:

- `idx_ordenes_servicio_programada ON ordenes_servicio(tenant_id, fecha_programada) WHERE fecha_programada IS NOT NULL`
  — the agenda range scan and the conflict query both filter on it.
- `idx_orden_servicio_tecnicos_empleado ON orden_servicio_tecnicos(empleado_id)` — the conflict query looks up
  "every other orden of this empleado"; today only an `orden_servicio_id` index exists.

## Files

- `services/core/src/services/orden_servicio_service.rs` (private to this module — the bulk of the change)
- `services/core/src/main.rs` (one new route line + one handler block + `confirmar_conflicto` plumbing — additive)
- `services/core/src/bin/migrate.rs` (one idempotent index block)
- `services/core/tests/ordenes_servicio_agenda.rs` (new binary)
- `apps/web/app/api/ordenes-servicio/agenda/route.ts` (new proxy)
- `apps/web/app/(customer)/(dashboard)/ordenes-servicio/agenda/page.tsx` (new page) + link from the list page
- `apps/web/app/(customer)/(dashboard)/ordenes-servicio/nueva/page.tsx`, `[id]/page.tsx` (time inputs, 409 dialog)
- `apps/web/lib/api.ts` (`ApiError` gains an optional `data` field so the 409 payload survives to the UI)
- `apps/web/cypress/e2e/orden-servicio-agenda.cy.ts` (new spec)

## Risks

- `main.rs` and `migrate.rs` are shared with four parallel agents — hunks are kept small and contiguous.
- Route ordering: `/v1/ordenes-servicio/agenda` vs `/v1/ordenes-servicio/:id`. Verified precedent in this same
  router (`/v1/tenants/:rnc` + `/v1/tenants/me/modulos`), plus a backend test that hits both.
- The conflict check is a read-then-write without a lock. Two simultaneous assignments could both pass. Given
  the rule is deliberately soft (an override exists), a serialized guard is not worth the contention — noted,
  not fixed.
- Adding the strict `hora_fin > hora_inicio` validation is a new 400 on a previously-unvalidated field. Nothing
  in the current UI writes those fields, so no existing caller can regress.

## Explicitly out of scope

Notifications/SMS/email reminders · client portal and remote quote approval · `equipos_cliente` history · the
dead `ACEPTADA`/`VENCIDA` cotización states · drag-and-drop dispatch board and any calendar library ·
availability / shift / skill / travel-time modelling · recurring maintenance contracts · técnico commissions.

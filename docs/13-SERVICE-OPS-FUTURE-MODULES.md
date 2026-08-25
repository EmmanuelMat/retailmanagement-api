# Service Operations — Future Modules Backlog

Not scheduled, not started. Ideas surfaced while building the Órdenes de Servicio / Órdenes de Compra module (see `08-DATABASE.md` for the current schema), informed by researching QuickBooks Desktop's Sales Order form and modern field-service SaaS (Kickserv, Jobber, ServiceTitan, Housecall Pro).

## Small extensions of what already exists

- **Programación / Dispatch board** — a visual calendar (drag técnicos onto time slots, see who's free) instead of today's plain `fecha_programada` field + técnico dropdown. This is the single biggest gap vs. every field-service competitor (ServiceTitan's Dispatch Board, Jobber's per-visit scheduling). Candidate módulo code: `PROGRAMACION`.
- **Recordatorios automáticos** — SMS/email when an orden becomes `PROGRAMADA`, or a "técnico en camino" notification. Needs a notification service (doesn't exist today) but is otherwise cheap once built.
- **Historial de equipos del cliente** — tie a serviced unit (e.g. "el split de la oficina") to a `cliente` so future órdenes can reference prior service history/warranty. Would need a new `equipos_cliente` table, FK'd from `orden_servicio_items` or `orden_servicio_notas`.

## New subsystems

- **Contratos de mantenimiento recurrente** — auto-generate an `orden_servicio` on a schedule (e.g. monthly A/C service) instead of every job starting from a cotización. Candidate módulo code: `CONTRATOS`.
- **Portal de cliente** — self-service view of a client's own órdenes/cotizaciones/facturas; approve an estimate online without a phone call. Candidate módulo code: `PORTAL_CLIENTE`.
- **Rentabilidad por orden** — compare labor + materials cost against what was billed, per job. Pairs naturally with the existing `orden_servicio_materiales` planned-vs-consumed split and `orden_servicio_tecnicos` assignment — the data is already there, this would mostly be a reporting view.
- **Comisión por técnico** — feed completed `ordenes_servicio` into the existing Nómina module for commission-based pay, since `orden_servicio_tecnicos` already tracks who did the work.
- **`customer_jobs` / project grouping** — group multiple órdenes under one property/project (QuickBooks' "Customer:Job" concept). Explicitly deferred during the original build since it wasn't confirmed as needed — revisit if a client has recurring multi-visit work at the same site.

## Context

Built as of 2026-08-25: `ordenes_servicio` + items/técnicos/materiales/notas, `ordenes_compra` + items, `adjuntos`, and an additive `permisos_catalogo`/`roles`/`role_permisos` layer — all backend-complete and tested (see the approved plan and its 27 backend tests). Frontend for this module was still in progress as of this doc's creation.

/**
 * Frontend mirror of the permission/route matrix enforced for real in
 * services/core/src/main.rs (`required_permiso`). This copy only drives
 * nav visibility and a redirect for UX — the Rust core is the source
 * of truth for actual authorization. Keep the two in sync.
 *
 * Roles are no longer a fixed 4-value enum: they're created and assigned
 * from the staff site (see apps/web/app/staff/(dashboard)/roles), each one
 * a bundle of permission codes from `permisos_catalogo`. `usuario.rol` is
 * kept only as a display label.
 */
export interface UsuarioAuth {
  rol?: string;
  es_admin?: boolean;
  permisos?: string[];
}

const ROUTE_PERMISOS: [string, string][] = [
  ["/pos", "ventas.gestionar"],
  ["/ventas", "ventas.gestionar"],
  ["/cotizaciones", "cotizaciones.gestionar"],
  ["/conduces", "conduces.gestionar"],
  ["/caja", "caja.gestionar"],
  ["/clientes", "clientes.gestionar"],
  ["/ordenes-servicio", "ordenes_servicio.gestionar"],
  ["/inventario", "inventario.gestionar"],
  ["/compras", "compras.gestionar"],
  ["/ordenes-compra", "ordenes_compra.gestionar"],
  ["/proveedores", "proveedores.gestionar"],
  ["/contabilidad", "contabilidad.gestionar"],
  ["/bancos", "bancos.gestionar"],
  ["/gastos", "gastos.gestionar"],
  ["/reportes", "reportes.dgii"],
  ["/nomina", "nomina.gestionar"],
  ["/configuracion", "config.gestionar"],
];

/** `/dashboard` and any other unlisted route are allowed for every user. */
export function isRouteAllowed(pathname: string, usuario?: UsuarioAuth | null): boolean {
  if (!usuario) return false;
  if (usuario.es_admin) return true;
  const match = ROUTE_PERMISOS.find(([prefix]) => pathname.startsWith(prefix));
  return !match || (usuario.permisos ?? []).includes(match[1]);
}

const DGII_ROUTE_PREFIXES = ["/reportes/dgii", "/configuracion/dgii"];

/** Sections that only make sense when the tenant has e-CF turned on. */
export function isDgiiRoute(pathname: string): boolean {
  return DGII_ROUTE_PREFIXES.some((prefix) => pathname.startsWith(prefix));
}

/**
 * Frontend mirror of `required_modulo` in services/core/src/main.rs. Only
 * drives nav visibility here — the Rust core is the real enforcement (see
 * `modulo_guard`). Which modules a tenant has is decided on the staff site,
 * never by the tenant themselves — there is no module picker anywhere in
 * this app.
 *
 * A route's second element is a list of module codes — any ONE of them
 * being active is enough. Ventas/cotizaciones/clientes serve both the
 * POS/retail flow and a SERVICIOS tenant's Cotización → Orden de Servicio
 * → Facturación flow, so they stay reachable under ORDENES_SERVICIO alone,
 * even for a tenant that never activated POS_VENTAS (no cash register).
 */
const ROUTE_MODULOS: [string, string[]][] = [
  ["/pos", ["POS_VENTAS"]],
  ["/ventas", ["POS_VENTAS", "ORDENES_SERVICIO"]],
  ["/cotizaciones", ["POS_VENTAS", "ORDENES_SERVICIO"]],
  ["/conduces", ["POS_VENTAS"]],
  ["/clientes", ["POS_VENTAS", "ORDENES_SERVICIO"]],
  ["/ordenes-servicio", ["ORDENES_SERVICIO"]],
  ["/inventario", ["INVENTARIO"]],
  ["/compras", ["COMPRAS_GASTOS"]],
  ["/ordenes-compra", ["COMPRAS_GASTOS"]],
  ["/proveedores", ["COMPRAS_GASTOS"]],
  ["/gastos", ["COMPRAS_GASTOS"]],
  ["/contabilidad", ["CONTABILIDAD"]],
  ["/caja", ["CAJA_BANCOS"]],
  ["/bancos", ["CAJA_BANCOS"]],
  ["/nomina", ["NOMINA"]],
  ["/reportes/dgii", ["DGII_ECF"]],
  ["/configuracion/dgii", ["DGII_ECF"]],
];

/**
 * `modulosActivos === null` means "still loading" — fails CLOSED (hide) for
 * that window rather than open, so a tenant never sees a module they aren't
 * entitled to even for a moment. The dashboard shell
 * (app/(customer)/(dashboard)/layout.tsx) gates its own render on this
 * resolving first, so in the normal path nothing calls this while it's
 * still null; this default is the safe fallback for any other caller (or a
 * fetch that fails and never resolves). Mirrors the backend: staff's
 * `tenant_modulos` configuration applies regardless of license status, so a
 * `trial` tenant sees exactly what staff assigned, not the full system.
 */
export function isModuloAllowed(pathname: string, modulosActivos: Set<string> | null): boolean {
  if (!modulosActivos) return false;
  const match = ROUTE_MODULOS.find(([prefix]) => pathname.startsWith(prefix));
  return !match || match[1].some((codigo) => modulosActivos.has(codigo));
}

/** Same rule as `isModuloAllowed`, for features that aren't tied to a route
 * (e.g. the AI widget/digest, gated by IA_ASISTENTE) — check a module code
 * directly instead of matching a pathname prefix. */
export function isModuloActivo(codigo: string, modulosActivos: Set<string> | null): boolean {
  if (!modulosActivos) return false;
  return modulosActivos.has(codigo);
}

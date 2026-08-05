/**
 * Frontend mirror of the role/route matrix enforced for real in
 * services/core/src/main.rs (`required_roles`). This copy only drives
 * nav visibility and a redirect for UX — the Rust core is the source
 * of truth for actual authorization. Keep the two in sync.
 */
export const ROLES = ["ADMIN", "CAJERO", "ALMACEN", "CONTADOR"] as const;
export type Role = (typeof ROLES)[number];

const ROUTE_ROLES: [string, Role[]][] = [
  ["/pos", ["CAJERO"]],
  ["/ventas", ["CAJERO"]],
  ["/cotizaciones", ["CAJERO"]],
  ["/conduces", ["CAJERO"]],
  ["/caja", ["CAJERO"]],
  ["/clientes", ["CAJERO"]],
  ["/inventario", ["ALMACEN"]],
  ["/compras", ["ALMACEN"]],
  ["/proveedores", ["ALMACEN"]],
  ["/contabilidad", ["CONTADOR"]],
  ["/bancos", ["CONTADOR"]],
  ["/gastos", ["CONTADOR"]],
  ["/reportes", ["CONTADOR"]],
  ["/nomina", ["ADMIN"]],
  ["/configuracion", ["ADMIN"]],
];

/** `/dashboard` and any other unlisted route are allowed for every role. */
export function isRouteAllowed(pathname: string, rol?: string): boolean {
  if (!rol) return false;
  if (rol === "ADMIN") return true;
  const match = ROUTE_ROLES.find(([prefix]) => pathname.startsWith(prefix));
  return !match || match[1].includes(rol as Role);
}

const DGII_ROUTE_PREFIXES = ["/reportes/dgii", "/configuracion/dgii"];

/** Sections that only make sense when the tenant has e-CF turned on. */
export function isDgiiRoute(pathname: string): boolean {
  return DGII_ROUTE_PREFIXES.some((prefix) => pathname.startsWith(prefix));
}

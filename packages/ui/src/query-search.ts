// Parser + helpers para el sistema de búsqueda avanzada por prefijos
// (ej. "category:bebidas status:activo arroz"). Puro/sin React para que sea
// testeable y reusable fuera de QuerySearchInput si algún día hace falta.

export interface QueryToken {
  prefix: string;
  value: string;
}

export interface ParsedQuery {
  /** Texto restante, sin los tokens "prefijo:valor" reconocidos. */
  text: string;
  tokens: QueryToken[];
}

// prefijo:valor  |  prefijo:"valor con espacios"
const TOKEN_RE = /([a-zA-Z]+):("[^"]*"|\S+)/g;

export function parseQueryTokens(raw: string): ParsedQuery {
  const tokens: QueryToken[] = [];
  const text = raw
    .replace(TOKEN_RE, (_match, prefix: string, value: string) => {
      tokens.push({ prefix: prefix.toLowerCase(), value: value.replace(/^"|"$/g, "") });
      return "";
    })
    .replace(/\s+/g, " ")
    .trim();
  return { text, tokens };
}

export type FilterPatch = Record<string, string | undefined>;

/**
 * Helper para prefijos tipo "date:2026-01-01..2026-01-31" (o un solo día).
 * Devuelve null ante sintaxis inválida - QuerySearchInput lo trata entonces
 * como texto libre en vez de fallar la búsqueda completa.
 */
export function dateRangeFilter(fromKey: string, toKey: string) {
  const isValidDate = (s: string) => /^\d{4}-\d{2}-\d{2}$/.test(s);
  return (value: string): FilterPatch | null => {
    const parts = value.split("..");
    if (parts.length === 2 && isValidDate(parts[0]) && isValidDate(parts[1])) {
      return { [fromKey]: parts[0], [toKey]: parts[1] };
    }
    if (parts.length === 1 && isValidDate(parts[0])) {
      return { [fromKey]: parts[0], [toKey]: parts[0] };
    }
    return null;
  };
}

/**
 * Helper para prefijos tipo "category:bebidas" o "client:Juan" que resuelven
 * un nombre contra una lista ya cargada en memoria (ej. el combo de
 * categorías/clientes que la página ya trae para su <Select>). Para listas
 * grandes que no se precargan por completo, pásale una función async que
 * busque en el servidor en su lugar (ver `QueryPrefixDef.apply`).
 */
export function lookupFilter<T>(items: T[], getLabel: (item: T) => string, getId: (item: T) => string, filterKey: string) {
  return (value: string): FilterPatch | null => {
    const needle = value.trim().toLowerCase();
    if (!needle) return null;
    const match = items.find((item) => getLabel(item).toLowerCase().includes(needle));
    return match ? { [filterKey]: getId(match) } : null;
  };
}

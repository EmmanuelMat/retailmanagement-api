"use client";

import * as React from "react";
import { Search } from "lucide-react";
import { cn } from "./utils";
import { parseQueryTokens, type FilterPatch } from "./query-search";

export interface QueryPrefixDef {
  /** Sin ":" - ej. "category", "status", "date". Se compara sin distinguir mayúsculas. */
  prefix: string;
  /** Texto de ayuda mostrado en el autocompletado, ej. "nombre de categoría". */
  label: string;
  /**
   * Traduce el valor del prefijo a un parche de filtros del `useServerTable`
   * de la página (ej. { categoriaId: "uuid" }). Devolver `null` significa
   * "no reconocido" - el token vuelve a formar parte del texto libre en vez
   * de romper la búsqueda (manejo tolerante de queries inválidas).
   */
  apply: (value: string) => FilterPatch | null | Promise<FilterPatch | null>;
}

export interface ParsedQueryResult {
  text: string;
  filters: FilterPatch;
}

export interface QuerySearchInputProps {
  value: string;
  onChange: (value: string) => void;
  /** Se llama (debounced) con el texto libre + los filtros resueltos de los prefijos reconocidos. */
  onParsed: (result: ParsedQueryResult) => void;
  prefixes: QueryPrefixDef[];
  placeholder?: string;
  debounceMs?: number;
  className?: string;
  id?: string;
}

/**
 * Input de búsqueda con soporte de prefijos tipo "category:bebidas
 * status:activo arroz" - ver query-search.ts para el parser. Reusable: cada
 * página define sus propios QueryPrefixDef según qué filtros ya soporta su
 * endpoint (ver productos/page.tsx y ventas/page.tsx para dos variantes,
 * una con resolución sincrónica contra una lista precargada y otra con
 * paso directo a un filtro existente).
 */
export function QuerySearchInput({
  value,
  onChange,
  onParsed,
  prefixes,
  placeholder = "Buscar...",
  debounceMs = 350,
  className,
  id,
}: QuerySearchInputProps) {
  const [showHints, setShowHints] = React.useState(false);
  const inputRef = React.useRef<HTMLInputElement>(null);
  const requestId = React.useRef(0);

  React.useEffect(() => {
    const reqId = ++requestId.current;
    const t = setTimeout(async () => {
      const { text, tokens } = parseQueryTokens(value);
      const filters: FilterPatch = {};
      const leftover: string[] = [];
      for (const token of tokens) {
        const def = prefixes.find((p) => p.prefix.toLowerCase() === token.prefix);
        if (!def) {
          leftover.push(`${token.prefix}:${token.value}`);
          continue;
        }
        let result: FilterPatch | null;
        try {
          result = await def.apply(token.value);
        } catch {
          result = null;
        }
        if (result) {
          Object.assign(filters, result);
        } else {
          leftover.push(token.value);
        }
      }
      if (requestId.current !== reqId) return;
      onParsed({ text: [text, ...leftover].filter(Boolean).join(" ").trim(), filters });
      // eslint-disable-next-line react-hooks/exhaustive-deps
    }, debounceMs);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [value, debounceMs]);

  const currentWord = React.useMemo(() => {
    const caret = inputRef.current?.selectionStart ?? value.length;
    const match = /(\S+)$/.exec(value.slice(0, caret));
    return match ? match[1] : "";
  }, [value]);

  const suggestions = React.useMemo(() => {
    if (!currentWord || currentWord.includes(":")) return [];
    return prefixes.filter((p) => p.prefix.toLowerCase().startsWith(currentWord.toLowerCase()));
  }, [currentWord, prefixes]);

  function applySuggestion(prefix: string) {
    const caret = inputRef.current?.selectionStart ?? value.length;
    const before = value.slice(0, caret).replace(/(\S+)$/, `${prefix}:`);
    const after = value.slice(caret);
    onChange(`${before}${after}`);
    requestAnimationFrame(() => {
      inputRef.current?.focus();
      inputRef.current?.setSelectionRange(before.length, before.length);
    });
  }

  return (
    <div className={cn("relative", className)}>
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <input
          ref={inputRef}
          id={id}
          className="flex h-10 w-full rounded-md border border-border bg-surface pl-9 pr-3 py-2 text-sm placeholder:text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 focus-visible:border-primary/50"
          value={value}
          placeholder={placeholder}
          onChange={(e) => {
            onChange(e.target.value);
            setShowHints(true);
          }}
          onFocus={() => setShowHints(true)}
          onBlur={() => setShowHints(false)}
          autoComplete="off"
        />
      </div>
      {showHints && suggestions.length > 0 && (
        <div className="absolute z-20 mt-1 w-full max-h-56 overflow-y-auto rounded-md border border-border bg-surface shadow-lifted">
          {suggestions.map((s) => (
            <button
              key={s.prefix}
              type="button"
              onMouseDown={(e) => e.preventDefault()}
              onClick={() => applySuggestion(s.prefix)}
              className="flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left text-sm hover:bg-muted/60"
            >
              <span className="font-medium font-mono">{s.prefix}:</span>
              <span className="text-xs text-muted-foreground">{s.label}</span>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

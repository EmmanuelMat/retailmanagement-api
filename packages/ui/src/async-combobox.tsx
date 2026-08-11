"use client";

import * as React from "react";
import { Search, X, Loader2 } from "lucide-react";
import { cn } from "./utils";

export interface AsyncComboboxProps<T> {
  value: T | null;
  onChange: (value: T | null) => void;
  /** Called with the current search text (debounced). Return matching results — do the search server-side. */
  fetchOptions: (query: string) => Promise<T[]>;
  getKey: (item: T) => string;
  getLabel: (item: T) => string;
  /** Optional second line per result row (e.g. SKU, stock, cédula). */
  getSublabel?: (item: T) => string | null | undefined;
  placeholder?: string;
  emptyMessage?: string;
  /** Milliseconds to wait after typing before searching. Default 300. */
  debounceMs?: number;
  disabled?: boolean;
  id?: string;
  className?: string;
}

/**
 * Server-side searchable autocomplete — the reusable replacement for a
 * plain <Select> that loads an entire table into the browser. The consumer
 * supplies `fetchOptions`, so this component has no opinion on the API
 * shape and works with any paginated/searchable list endpoint.
 */
export function AsyncCombobox<T>({
  value,
  onChange,
  fetchOptions,
  getKey,
  getLabel,
  getSublabel,
  placeholder = "Buscar...",
  emptyMessage = "Sin resultados.",
  debounceMs = 300,
  disabled,
  id,
  className,
}: AsyncComboboxProps<T>) {
  const [open, setOpen] = React.useState(false);
  const [query, setQuery] = React.useState("");
  const [results, setResults] = React.useState<T[]>([]);
  const [loading, setLoading] = React.useState(false);
  const [error, setError] = React.useState("");
  const [highlight, setHighlight] = React.useState(0);

  const rootRef = React.useRef<HTMLDivElement>(null);
  const requestId = React.useRef(0);

  React.useEffect(() => {
    if (!open) return;
    const id = ++requestId.current;
    setLoading(true);
    setError("");
    const t = setTimeout(() => {
      fetchOptions(query)
        .then((items) => {
          if (requestId.current === id) {
            setResults(items);
            setHighlight(0);
          }
        })
        .catch((e) => {
          if (requestId.current === id) setError(e instanceof Error ? e.message : "Error al buscar");
        })
        .finally(() => {
          if (requestId.current === id) setLoading(false);
        });
    }, debounceMs);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, open, debounceMs]);

  React.useEffect(() => {
    function onClickOutside(e: MouseEvent) {
      if (rootRef.current && !rootRef.current.contains(e.target as Node)) setOpen(false);
    }
    document.addEventListener("mousedown", onClickOutside);
    return () => document.removeEventListener("mousedown", onClickOutside);
  }, []);

  function select(item: T) {
    onChange(item);
    setOpen(false);
    setQuery("");
  }

  function clear() {
    onChange(null);
    setQuery("");
    setResults([]);
  }

  function onKeyDown(e: React.KeyboardEvent) {
    if (!open) return;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setHighlight((h) => Math.min(h + 1, results.length - 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setHighlight((h) => Math.max(h - 1, 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (results[highlight]) select(results[highlight]);
    } else if (e.key === "Escape") {
      setOpen(false);
    }
  }

  const showingSelected = value && !open;

  return (
    <div ref={rootRef} className={cn("relative", className)}>
      <div className="relative">
        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
        <input
          id={id}
          disabled={disabled}
          className={cn(
            "flex h-10 w-full rounded-md border border-border bg-surface pl-9 pr-8 py-2 text-sm placeholder:text-muted-foreground transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40 focus-visible:border-primary/50 disabled:opacity-50"
          )}
          value={showingSelected ? getLabel(value as T) : query}
          placeholder={placeholder}
          onFocus={() => {
            setOpen(true);
            setQuery("");
          }}
          onChange={(e) => {
            setQuery(e.target.value);
            if (!open) setOpen(true);
          }}
          onKeyDown={onKeyDown}
          autoComplete="off"
        />
        {value && (
          <button
            type="button"
            onClick={clear}
            className="absolute right-2.5 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
            aria-label="Limpiar selección"
          >
            <X className="h-3.5 w-3.5" />
          </button>
        )}
      </div>

      {open && (
        <div className="absolute z-20 mt-1 w-full max-h-64 overflow-y-auto rounded-md border border-border bg-surface shadow-lifted">
          {loading ? (
            <div className="flex items-center gap-2 px-3 py-2.5 text-sm text-muted-foreground">
              <Loader2 className="h-3.5 w-3.5 animate-spin" /> Buscando...
            </div>
          ) : error ? (
            <div className="px-3 py-2.5 text-sm text-destructive">{error}</div>
          ) : results.length === 0 ? (
            <div className="px-3 py-2.5 text-sm text-muted-foreground">{emptyMessage}</div>
          ) : (
            results.map((item, i) => {
              const sublabel = getSublabel?.(item);
              return (
                <button
                  type="button"
                  key={getKey(item)}
                  onClick={() => select(item)}
                  onMouseEnter={() => setHighlight(i)}
                  className={cn(
                    "flex w-full flex-col items-start gap-0.5 px-3 py-2 text-left text-sm transition-colors",
                    i === highlight ? "bg-muted" : "hover:bg-muted/60"
                  )}
                >
                  <span className="font-medium">{getLabel(item)}</span>
                  {sublabel && <span className="text-xs text-muted-foreground">{sublabel}</span>}
                </button>
              );
            })
          )}
        </div>
      )}
    </div>
  );
}

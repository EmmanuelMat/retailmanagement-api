import { useCallback, useEffect, useRef, useState } from "react";
import { apiFetch, ApiError } from "./api";

export type SortDir = "asc" | "desc";

export interface ServerTableState<F extends Record<string, any>> {
  page: number;
  pageSize: number;
  sortBy: string | null;
  sortDir: SortDir;
  filters: F;
}

export interface UseServerTableOptions<F extends Record<string, any>> {
  /** BFF route, e.g. "/api/productos" - not the core URL. */
  path: string;
  initialPageSize?: number;
  initialSortBy?: string | null;
  initialSortDir?: SortDir;
  initialFilters?: F;
  /** When set, silently re-fetches on this interval without flipping `loading`. */
  pollIntervalMs?: number;
}

interface CoreListResponse<T> {
  items: T[];
  page: number;
  page_size: number;
  total: number;
  total_pages: number;
}

export interface UseServerTableResult<T, F extends Record<string, any>> {
  items: T[];
  total: number;
  totalPages: number;
  loading: boolean;
  error: string | null;
  state: ServerTableState<F>;
  setPage: (page: number) => void;
  setPageSize: (size: number) => void;
  /** Click same column: asc<->desc toggle. Click new column: starts asc. Resets page to 1. */
  toggleSort: (column: string) => void;
  /** Shallow-merges into filters and resets page to 1. */
  setFilters: (patch: Partial<F>) => void;
  refresh: () => void;
}

export function useServerTable<T = any, F extends Record<string, any> = Record<string, never>>(
  opts: UseServerTableOptions<F>
): UseServerTableResult<T, F> {
  const {
    path,
    initialPageSize = 20,
    initialSortBy = null,
    initialSortDir = "asc",
    initialFilters = {} as F,
    pollIntervalMs,
  } = opts;

  const [state, setState] = useState<ServerTableState<F>>({
    page: 1,
    pageSize: initialPageSize,
    sortBy: initialSortBy,
    sortDir: initialSortDir,
    filters: initialFilters,
  });

  const [items, setItems] = useState<T[]>([]);
  const [total, setTotal] = useState(0);
  const [totalPages, setTotalPages] = useState(1);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [refreshTick, setRefreshTick] = useState(0);

  const stateKey = JSON.stringify(state);

  const fetchPage = useCallback(
    async (silent: boolean) => {
      if (!silent) setLoading(true);
      setError(null);
      const qs = new URLSearchParams();
      qs.set("page", String(state.page));
      qs.set("pageSize", String(state.pageSize));
      if (state.sortBy) {
        qs.set("sortBy", state.sortBy);
        qs.set("sortDir", state.sortDir);
      }
      for (const [key, value] of Object.entries(state.filters)) {
        if (value === undefined || value === null || value === "") continue;
        qs.set(key, String(value));
      }
      try {
        const res = await apiFetch<CoreListResponse<T>>(`${path}?${qs.toString()}`);
        setItems(res.items);
        setTotal(res.total);
        setTotalPages(res.total_pages);
      } catch (e) {
        setError(e instanceof ApiError ? e.message : "Error al cargar los datos");
      } finally {
        if (!silent) setLoading(false);
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [path, stateKey]
  );

  useEffect(() => {
    fetchPage(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [fetchPage, refreshTick]);

  const intervalRef = useRef<ReturnType<typeof setInterval> | null>(null);
  useEffect(() => {
    if (!pollIntervalMs) return;
    intervalRef.current = setInterval(() => fetchPage(true), pollIntervalMs);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pollIntervalMs, fetchPage]);

  const setPage = useCallback((page: number) => {
    setState((prev) => ({ ...prev, page }));
  }, []);

  const setPageSize = useCallback((pageSize: number) => {
    setState((prev) => ({ ...prev, pageSize, page: 1 }));
  }, []);

  const toggleSort = useCallback((column: string) => {
    setState((prev) => {
      if (prev.sortBy === column) {
        return { ...prev, sortDir: prev.sortDir === "asc" ? "desc" : "asc", page: 1 };
      }
      return { ...prev, sortBy: column, sortDir: "asc", page: 1 };
    });
  }, []);

  const setFilters = useCallback((patch: Partial<F>) => {
    setState((prev) => ({ ...prev, filters: { ...prev.filters, ...patch }, page: 1 }));
  }, []);

  const refresh = useCallback(() => setRefreshTick((t) => t + 1), []);

  return { items, total, totalPages, loading, error, state, setPage, setPageSize, toggleSort, setFilters, refresh };
}

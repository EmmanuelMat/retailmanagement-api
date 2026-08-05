import { useCallback, useEffect, useRef, useState } from "react";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
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

const RESERVED_QUERY_KEYS = new Set(["page", "pageSize", "sortBy", "sortDir"]);

/** Shared by the fetch call and the URL sync so the query-string shape never drifts between the two. */
function buildQueryString<F extends Record<string, any>>(state: ServerTableState<F>): URLSearchParams {
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
  return qs;
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

  const router = useRouter();
  const pathname = usePathname();
  const searchParams = useSearchParams();

  // Seeded once from the URL on mount so filters/sort/page survive navigating
  // away and back (e.g. browser back button) instead of always resetting to
  // the page's defaults.
  const [state, setState] = useState<ServerTableState<F>>(() => {
    const urlPage = Number(searchParams.get("page"));
    const urlPageSize = Number(searchParams.get("pageSize"));
    const urlSortDir = searchParams.get("sortDir");
    const filters = { ...initialFilters } as Record<string, any>;
    for (const [key, value] of searchParams.entries()) {
      if (RESERVED_QUERY_KEYS.has(key)) continue;
      filters[key] = value;
    }
    return {
      page: urlPage > 0 ? urlPage : 1,
      pageSize: urlPageSize > 0 ? urlPageSize : initialPageSize,
      sortBy: searchParams.get("sortBy") || initialSortBy,
      sortDir: urlSortDir === "asc" || urlSortDir === "desc" ? urlSortDir : initialSortDir,
      filters: filters as F,
    };
    // Only read the URL on first mount - after that `state` is the source of truth.
    // eslint-disable-next-line react-hooks/exhaustive-deps
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
      try {
        const res = await apiFetch<CoreListResponse<T>>(`${path}?${buildQueryString(state).toString()}`);
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

  // Keep the URL in sync (replace, not push, so this doesn't spam history)
  // so page/pageSize/sortBy/sortDir/filters are restored on back-navigation.
  // Polling refreshes (refreshTick/silent fetches) don't touch `state`, so
  // they never trigger this.
  useEffect(() => {
    router.replace(`${pathname}?${buildQueryString(state).toString()}` as any, { scroll: false });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stateKey, pathname]);

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

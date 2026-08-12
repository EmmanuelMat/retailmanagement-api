/**
 * Fetch helper for the staff console's BFF routes, which live under
 * /api/staff/* to stay out of the customer app's /api/* namespace (both
 * apps now share this Next.js project, split by hostname in middleware.ts).
 * Auth is a single shared secret (X-Vendor-Secret), not a per-tenant JWT -
 * see services/core/src/services/staff_service.rs for the scaling note.
 *
 * Pages call this the same way apps/staff's original client did (e.g.
 * apiFetch("/api/tenants")) - the /api/staff prefix is added here so
 * those call sites didn't need to change when the app was folded in.
 */
export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

export async function apiFetch<T = any>(path: string, opts: RequestInit = {}): Promise<T> {
  const secret = typeof window !== "undefined" ? localStorage.getItem("vendorSecret") : null;
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(secret ? { "X-Vendor-Secret": secret } : {}),
    ...(opts.headers as Record<string, string>),
  };

  const url = path.startsWith("/api/") ? `/api/staff${path.slice(4)}` : path;
  const res = await fetch(url, { ...opts, headers, cache: "no-store" });
  const isJson = res.headers.get("content-type")?.includes("application/json");
  const data = isJson ? await res.json() : await res.text();

  if (!res.ok) {
    const message = typeof data === "string" ? data : data?.error || `Error ${res.status}`;
    if (res.status === 403 && typeof window !== "undefined") {
      localStorage.removeItem("vendorSecret");
      window.location.href = "/login";
      return new Promise(() => {});
    }
    throw new ApiError(message, res.status);
  }
  return data as T;
}

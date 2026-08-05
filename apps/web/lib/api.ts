/**
 * Fetch helper for BFF routes (/api/*) that attaches the tenant's JWT.
 * The token is stored client-side by the login flow (see app/(auth)/login).
 */
export class ApiError extends Error {
  status: number;
  constructor(message: string, status: number) {
    super(message);
    this.status = status;
  }
}

export async function apiFetch<T = any>(path: string, opts: RequestInit = {}): Promise<T> {
  const token = typeof window !== "undefined" ? localStorage.getItem("token") : null;
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(token ? { Authorization: `Bearer ${token}` } : {}),
    ...(opts.headers as Record<string, string>),
  };

  const res = await fetch(path, { ...opts, headers, cache: "no-store" });
  const isJson = res.headers.get("content-type")?.includes("application/json");
  const data = isJson ? await res.json() : await res.text();

  if (!res.ok) {
    const message = typeof data === "string" ? data : data?.error || `Error ${res.status}`;
    // La sesión expiró (JWT de 12h) o el token es inválido: en vez de dejar
    // que cada pantalla falle con su propio mensaje confuso, se limpia la
    // sesión y se manda a login con el path actual para volver después.
    if (res.status === 401 && typeof window !== "undefined") {
      localStorage.removeItem("token");
      localStorage.removeItem("usuario");
      localStorage.removeItem("tenant");
      document.cookie = "token=; path=/; max-age=0";
      const redirect = encodeURIComponent(window.location.pathname + window.location.search);
      window.location.href = `/login?redirect=${redirect}`;
      // Bloquea indefinidamente: la navegación ya está en curso, no tiene
      // sentido que el caller siga ejecutando con datos parciales/erróneos.
      return new Promise(() => {});
    }
    // 402 = período de prueba vencido (ver license_guard en el core). A
    // diferencia de un 401, la sesión sigue siendo válida - no se limpia,
    // solo se manda a la pantalla de "prueba vencida" en vez de dejar que
    // cada pantalla muestre su propio error crudo de "Payment Required".
    if (res.status === 402 && typeof window !== "undefined" && window.location.pathname !== "/trial-expirado") {
      window.location.href = "/trial-expirado";
      return new Promise(() => {});
    }
    throw new ApiError(message, res.status);
  }
  return data as T;
}

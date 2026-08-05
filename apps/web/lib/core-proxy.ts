/**
 * The Rust core returns JSON on success but plain text on error
 * (axum's `(StatusCode, String)` handlers serialize the String as text/plain).
 * BFF routes must not blindly call `res.json()` or they crash on every error path.
 */
export async function parseCoreResponse(res: Response): Promise<any> {
  const isJson = res.headers.get("content-type")?.includes("application/json");
  if (isJson) return res.json();
  const text = await res.text();
  return { error: text };
}

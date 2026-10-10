const cache = new Map(),
  pending = new Map();
let pauseUntil = 0;
let epoch = 0;
export function clearOnlineCache() {
  cache.clear();
  pending.clear();
  epoch++;
}
export async function api(action, params = {}, body, refresh = false) {
  const url = `/api/whakoom/${action}?${new URLSearchParams(params)}`;
  if (
    !body &&
    !refresh &&
    cache.has(url) &&
    cache.get(url).expires > Date.now()
  )
    return structuredClone(cache.get(url).value);
  if (!body && pending.has(url)) return pending.get(url);
  if (action !== "logout" && Date.now() < pauseUntil)
    throw new Error(
      "Whakoom pidió una pausa. Esperá un momento antes de consultar.",
    );
  const current = epoch;
  const task = (async () => {
    const response = await fetch(url, {
      method: body ? "POST" : "GET",
      credentials: "same-origin",
      cache: "no-store",
      signal: AbortSignal.timeout(30000),
      ...(body
        ? {
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(body),
          }
        : {}),
    });
    if (response.status === 429)
      pauseUntil =
        Date.now() +
        Math.min(
          3600,
          Math.max(30, Number(response.headers.get("retry-after")) || 60),
        ) *
          1000;
    let result;
    try {
      result = await response.json();
    } catch {
      throw new Error(
        "La conexión online necesita desplegarse con su servidor. Esta vista estática todavía no lo tiene.",
      );
    }
    if (!response.ok)
      throw new Error(result.error || "No se pudo conectar con Whakoom.");
    if (!body && action !== "session" && current === epoch) {
      for (const [key, entry] of cache)
        if (entry.expires <= Date.now()) cache.delete(key);
      if (cache.size >= 200) cache.delete(cache.keys().next().value);
      cache.set(url, { value: result, expires: Date.now() + 120000 });
    }
    if (body) clearOnlineCache();
    return result;
  })();
  if (!body) pending.set(url, task);
  try {
    return await task;
  } finally {
    if (!body && pending.get(url) === task) pending.delete(url);
  }
}

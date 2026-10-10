const NAME = "whakoom-shell-v2";
self.addEventListener("install", (event) =>
  event.waitUntil(
    caches.open(NAME).then((cache) => cache.addAll(["/", "/logo.svg"])),
  ),
);
self.addEventListener("activate", (event) =>
  event.waitUntil(
    caches
      .keys()
      .then((names) =>
        Promise.all(
          names
            .filter(
              (name) => name.startsWith("whakoom-shell-") && name !== NAME,
            )
            .map((name) => caches.delete(name)),
        ),
      ),
  ),
);
self.addEventListener("fetch", (event) => {
  const url = new URL(event.request.url);
  if (
    event.request.method !== "GET" ||
    url.origin !== self.location.origin ||
    !(
      url.pathname === "/" ||
      url.pathname === "/logo.svg" ||
      url.pathname.startsWith("/_astro/")
    )
  )
    return;
  event.respondWith(
    (async () => {
      const cache = await caches.open(NAME);
      try {
        const response = await fetch(event.request);
        if (response.ok) {
          await cache.put(event.request, response.clone());
          const keys = await cache.keys();
          for (const key of keys.slice(0, Math.max(0, keys.length - 40)))
            await cache.delete(key);
        }
        return response;
      } catch {
        return (await cache.match(event.request)) ?? Response.error();
      }
    })(),
  );
});

import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { once } from "node:events";
const root = fileURLToPath(new URL("../", import.meta.url));
const existing = process.env.WEB_TEST_URL;
const url = existing || "http://127.0.0.1:4322";
const server = existing
  ? null
  : spawn(
      process.execPath,
      [
        "node_modules/astro/bin/astro.mjs",
        "preview",
        "--host",
        "127.0.0.1",
        "--port",
        "4322",
      ],
      { cwd: root, stdio: "ignore", windowsHide: true },
    );
let ready = false;
try {
  for (let attempt = 0; attempt < 100; attempt++) {
    try {
      ready = (await fetch(url, { signal: AbortSignal.timeout(1000) })).ok;
    } catch {}
    if (ready) break;
    if (server && server.exitCode !== null && server.exitCode !== 0)
      throw new Error("No se pudo iniciar el servidor de pruebas.");
    await new Promise((resolve) => setTimeout(resolve, 200));
  }
  if (!ready) throw new Error("El servidor de pruebas no respondió.");
  const child = spawn(process.execPath, ["tests/browser.mjs"], {
    cwd: root,
    env: { ...process.env, WEB_TEST_URL: url },
    stdio: "inherit",
    windowsHide: true,
  });
  const [code] = await once(child, "exit");
  if (code !== 0) process.exitCode = code || 1;
} finally {
  if (server && ready) {
    const stop = spawn(
      process.execPath,
      ["node_modules/astro/bin/astro.mjs", "preview", "stop"],
      { cwd: root, stdio: "ignore", windowsHide: true },
    );
    await once(stop, "exit");
  }
  server?.kill();
}

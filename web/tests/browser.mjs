import { chromium } from "playwright";
import assert from "node:assert/strict";
import { readFile, mkdir } from "node:fs/promises";
import { open } from "../src/lib/crypto.mjs";

const browser = await chromium.launch({
  headless: true,
  ...(process.env.CHROME_CHANNEL
    ? { channel: process.env.CHROME_CHANNEL }
    : {}),
});
const context = await browser.newContext({
  viewport: { width: 1360, height: 900 },
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
const output = process.env.SCREENSHOT_DIR;
if (output) await mkdir(output, { recursive: true });
const password = "fictional-test-password";
const entry = (id, fields) => ({
  item: {
    key: `comic${id}`,
    title: `Historia de ejemplo ${id}`,
    issue: "#1",
    url: `https://www.whakoom.com/comics/${id}/ejemplo`,
    cover: "",
  },
  notes: "",
  ...fields,
});
const library = {
  owner: "lector_ejemplo",
  entries: {
    comica: entry("a", { owned: true, read: true }),
    comicb: entry("b", { owned: true, reading: true }),
    comicc: entry("c", { wanted: true }),
    comicd: entry("d", { owned: true }),
  },
};
let authenticated = false;
let allowSync = true;
await page.addInitScript(() => localStorage.setItem("whakoom-tutorial", "1"));
page.on("dialog", (dialog) => dialog.accept());
await page.route("**/api/whakoom/**", async (route) => {
  const url = new URL(route.request().url()),
    action = url.pathname.split("/").pop();
  let data;
  if (action === "session")
    data = {
      authenticated,
      configured: true,
      user: authenticated
        ? { username: library.owner, avatar: "", pro: true }
        : null,
    };
  else if (action === "login") {
    authenticated = true;
    data = {
      authenticated: true,
      user: { username: library.owner, avatar: "", pro: true },
    };
  } else if (action === "collection" && allowSync)
    data = { items: [], next: null };
  else if (action === "detail") {
    const key = url.searchParams.get("url").split("/")[4];
    const entry = library.entries[`comic${key}`];
    data = {
      item: { ...entry.item, owned: Boolean(entry.owned) },
      read: Boolean(entry.read),
      wanted: Boolean(entry.wanted),
      personal_rating: 0,
      comments: [],
      isbn: [],
    };
  } else if (action === "people")
    data = {
      people: [{ username: "reader", name: "Reader", pro: true }],
      cursor: null,
    };
  else if (action === "catalog")
    data = { items: [library.entries.comica.item], next: null };
  else if (action === "lists") data = { lists: [] };
  else {
    await route.fulfill({
      status: 503,
      json: { error: "Simulated offline state" },
    });
    return;
  }
  await route.fulfill({ json: data });
});
try {
  await page.goto(process.env.WEB_TEST_URL ?? "http://127.0.0.1:4321");
  if (output) await page.screenshot({ path: `${output}/web-locked.png` });
  await page.locator("#login").waitFor({ state: "visible" });
  assert.equal(await page.locator("#locked").isVisible(), false);
  await page.locator("#username").fill(library.owner);
  await page.locator("#login-password").fill("fictional-account-password");
  await page.locator("#login-form button").click();
  await page.locator("#locked").waitFor({ state: "visible" });
  assert.equal(await page.locator("#login-password").inputValue(), "");
  await page.locator("#password").fill(password);
  await page.locator("#unlock-form button").click();
  await page.locator("#workspace").waitFor({ state: "visible" });
  await page.waitForFunction(
    () =>
      document.querySelector("#sync-state").textContent ===
      "Conectado con Whakoom",
  );
  allowSync = false;
  await page.locator("[data-section=settings]").click();
  await page.locator("#import-file").setInputFiles({
    name: "example.json",
    mimeType: "application/json",
    buffer: Buffer.from(JSON.stringify(library)),
  });
  await page.waitForFunction(
    () => document.querySelectorAll(".book").length === 3,
  );
  await page.locator("[data-section=reading]").click();
  assert.equal(await page.locator(".book").count(), 2);
  await page.locator(".book").first().click();
  const note = "<script>literal private note</script>";
  await page.locator("textarea").fill(note);
  await page.waitForFunction(
    () =>
      document.querySelector("#status").textContent ===
      "Guardado cifrado en este navegador.",
  );
  await page
    .locator("#panel button")
    .filter({ hasText: "← Volver" })
    .first()
    .click();
  const stored = await page.evaluate(
    () =>
      new Promise((resolve) => {
        const request = indexedDB.open("whakoom-library");
        request.onsuccess = () => {
          const db = request.result;
          const read = db
            .transaction("vault")
            .objectStore("vault")
            .get("library");
          read.onsuccess = () => {
            resolve(read.result);
            db.close();
          };
        };
      }),
  );
  assert.equal(JSON.stringify(stored).includes("literal private note"), false);
  assert.equal((await open(stored, password)).data.entries.comica.notes, note);
  const downloadEvent = page.waitForEvent("download");
  await page.locator("[data-section=settings]").click();
  await page.locator("#export").click();
  const download = await downloadEvent;
  const envelope = JSON.parse(await readFile(await download.path(), "utf8"));
  assert.equal(
    (await open(envelope, password)).data.entries.comica.notes,
    note,
  );
  if (output) await page.screenshot({ path: `${output}/web-library.png` });
  await page.locator("[data-section=badges]").click();
  assert.equal(await page.locator(".badge").count(), 21);
  if (output)
    await page.screenshot({ path: `${output}/web-badges.png`, fullPage: true });
  await page.locator("[data-section=people]").click();
  await page.locator(".carousel-track .person-card").first().waitFor();
  assert.equal(
    await page
      .locator(".carousel-track")
      .evaluate((n) => getComputedStyle(n).animationName),
    "carousel",
  );
  await page.locator("[data-section=statistics]").click();
  assert.equal(await page.locator(".chart-month").count(), 12);
  await page.locator("[data-section=settings]").click();
  await page.locator("#panel select").first().selectOption("light");
  assert.equal(await page.locator("html").getAttribute("data-theme"), "light");
  await page.locator("#lock").click();
  await page.locator("#locked").waitFor({ state: "visible" });
  assert.equal(await page.locator("textarea").count(), 0);
  await page.reload();
  await page.locator("#locked").waitFor({ state: "visible" });
  await page.locator("#password").fill("wrong-but-long-password");
  await page.locator("#unlock-form button").click();
  await page.waitForFunction(() =>
    document.querySelector("#status").textContent.includes("incorrecta"),
  );
  await page.locator("#password").fill(password);
  await page.locator("#unlock-form button").click();
  await page.locator("#workspace").waitFor({ state: "visible" });
  await page.locator("[data-section=library]").click();
  await page.locator(".book").first().click();
  assert.equal(await page.locator("textarea").inputValue(), note);
  await page
    .locator("#panel button")
    .filter({ hasText: "← Volver" })
    .first()
    .click();
  await page.setViewportSize({ width: 390, height: 844 });
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  if (output)
    await page.screenshot({ path: `${output}/web-mobile.png`, fullPage: true });
  assert.deepEqual(errors, []);
  console.log(
    "Browser: import, encrypted persistence/export, lock, wrong password, reading states and mobile layout passed.",
  );
} finally {
  await browser.close();
}

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
try {
  await page.goto(process.env.WEB_TEST_URL ?? "http://127.0.0.1:4321");
  if (output) await page.screenshot({ path: `${output}/web-locked.png` });
  await page.locator("#password").fill(password);
  await page.locator("#unlock-form button").click();
  await page.locator("#workspace").waitFor({ state: "visible" });
  await page
    .locator("#import-file")
    .setInputFiles({
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
  await page.locator("#detail .close").click();
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
  await page.locator("#export").click();
  const download = await downloadEvent;
  const envelope = JSON.parse(await readFile(await download.path(), "utf8"));
  assert.equal(
    (await open(envelope, password)).data.entries.comica.notes,
    note,
  );
  if (output) await page.screenshot({ path: `${output}/web-library.png` });
  await page.locator("#lock").click();
  await page.locator("#locked").waitFor({ state: "visible" });
  assert.equal(await page.locator("textarea").count(), 0);
  await page.reload();
  await page.locator("#password").fill("wrong-but-long-password");
  await page.locator("#unlock-form button").click();
  await page.waitForFunction(() =>
    document.querySelector("#status").textContent.includes("incorrecta"),
  );
  await page.locator("#password").fill(password);
  await page.locator("#unlock-form button").click();
  await page.locator("#workspace").waitFor({ state: "visible" });
  await page.locator(".book").first().click();
  assert.equal(await page.locator("textarea").inputValue(), note);
  await page.locator("#detail .close").click();
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

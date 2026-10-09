import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { createKey, seal, open, validateEnvelope } from "../src/lib/crypto.mjs";
import {
  libraryFrom,
  safeCover,
  visibleEntries,
  totals,
} from "../src/lib/model.mjs";

test("portable protocol fixture matches the Rust client", async () => {
  const envelope = JSON.parse(
    await readFile(new URL("./portable-fixture.json", import.meta.url), "utf8"),
  );
  assert.deepEqual((await open(envelope, "portable-test-password")).data, {
    owner: "portable-test",
    entries: {},
  });
});

test("encrypted vault authenticates, uses a new nonce and rejects excessive work", async () => {
  const data = { owner: "example", entries: {}, privateNote: "test" };
  const session = await createKey("example-password-only");
  const first = await seal(data, session),
    second = await seal(data, session);
  assert.notEqual(first.nonce, second.nonce);
  assert.deepEqual((await open(first, "example-password-only")).data, data);
  await assert.rejects(open(first, "wrong-password"));
  const corrupted = {
    ...first,
    ciphertext: first.ciphertext.replace(
      /^./,
      first.ciphertext[0] === "A" ? "B" : "A",
    ),
  };
  await assert.rejects(open(corrupted, "example-password-only"));
  assert.throws(() => validateEnvelope({ ...first, iterations: 4294967295 }));
});
test("imports exclude credentials, validate identities and keep reading separate", () => {
  const entry = (key, fields) => ({
    item: { key, title: "<script>literal title</script>" },
    ...fields,
  });
  const library = libraryFrom({
    owner: "example",
    cookie: "secret",
    entries: {
      comica: entry("comica", { owned: true }),
      comicb: entry("comicb", { read: true }),
      comicc: entry("comicc", { reading: true }),
      edicion123: entry("edicion123", { owned: false }),
    },
  });
  assert.equal(library.cookie, undefined);
  assert.equal(visibleEntries(library, "reading").length, 2);
  assert.equal(totals(library).owned, 1);
  assert.throws(() =>
    libraryFrom({ owner: "a", entries: { comica: entry("comicOTHER", {}) } }),
  );
  assert.equal(
    safeCover("https://i1.whakoom.com/cover.png"),
    "https://i1.whakoom.com/cover.png",
  );
  for (const url of [
    "javascript:alert(1)",
    "https://whakoom.com.evil.test/a",
    "https://user:pass@i1.whakoom.com/a",
    "http://i1.whakoom.com/a",
  ])
    assert.equal(safeCover(url), "");
});

import { test } from "node:test";
import assert from "node:assert/strict";
import { sealSession, openSession, sessionCookie } from "../server/session.mjs";
import {
  Whakoom,
  trustedUrl,
  parseItems,
  parseDetail,
} from "../server/whakoom.mjs";
import { makeHandler } from "../netlify/functions/whakoom.mjs";
import { badgeList, updateBadges } from "../src/lib/badges.mjs";
import { editionViews } from "../src/lib/model.mjs";
const secret = "ab".repeat(32),
  origin = "https://web.example";
const user = { username: "example", name: "Example", avatar: "", pro: false };
const cookie = sessionCookie(
  sealSession({ user, cookie: "auth=fictional-cookie" }, secret),
);
const req = (action, body, extra = {}) =>
  new Request(`${origin}/api/whakoom/${action}`, {
    method: body === undefined ? "GET" : "POST",
    headers: { cookie, origin, "content-type": "application/json", ...extra },
    ...(body === undefined ? {} : { body: JSON.stringify(body) }),
  });
test("online session is authenticated ciphertext, short-lived and HttpOnly", () => {
  const token = sealSession({ user, cookie: "auth=secret" }, secret, 1000);
  assert.equal(token.includes("secret"), false);
  assert.equal(openSession(token, secret, 1001).cookie, "auth=secret");
  assert.equal(openSession(token, secret, 7201001), null);
  assert.equal(openSession(token, "cd".repeat(32), 1001), null);
  assert.equal(
    openSession(token.slice(0, -8) + "xxxxxxxx", secret, 1001),
    null,
  );
  assert.match(sessionCookie(token), /HttpOnly; Secure; SameSite=Strict/);
  assert.throws(() => sealSession({ user, cookie: "a".repeat(4000) }, secret));
});
test("auth and CSRF checks run before contacting Whakoom", async () => {
  let calls = 0;
  const handler = makeHandler({
    secret,
    fetcher: async () => {
      calls++;
      throw new Error("unexpected request");
    },
  });
  assert.equal(
    (await handler(req("collection", undefined, { cookie: "" }))).status,
    401,
  );
  assert.equal(
    (
      await handler(
        req(
          "change",
          { field: "read", value: true },
          { origin: "https://evil.example" },
        ),
      )
    ).status,
    403,
  );
  assert.equal((await handler(req("login", {}, { origin: "" }))).status, 403);
  assert.equal(
    (await handler(req("change", { field: "unknown", value: true }))).status,
    400,
  );
  assert.equal((await handler(req("collection?page=999999"))).status, 400);
  assert.equal(
    (await handler(req("login", { username: "x", password: "a".repeat(9000) })))
      .status,
    413,
  );
  assert.equal(calls, 0);
  const logout = await handler(req("logout", {}));
  assert.match(logout.headers.get("set-cookie"), /Max-Age=0/);
  assert.equal(logout.headers.get("cache-control"), "private, no-store");
});
test("Whakoom login uses the fresh CSRF form and never returns cookies to JavaScript", async () => {
  const calls = [];
  const handler = makeHandler({
    secret,
    fetcher: async (url, init) => {
      calls.push([String(url), init]);
      if (init.method === "GET")
        return new Response(
          '<form><input name="__RequestVerificationToken" value="fresh-token"></form>',
          { headers: { "set-cookie": "csrf=test; Path=/; Secure" } },
        );
      const body = new URLSearchParams(init.body);
      assert.equal(body.get("__RequestVerificationToken"), "fresh-token");
      assert.equal(body.get("username"), "example");
      assert.equal(body.get("userpassw"), "fictional-password");
      assert.match(init.headers.Cookie, /csrf=test/);
      return new Response(
        '<div id="user-avatar"><img alt="example" src="https://i1.whakoom.com/avatar.jpg"><span class="pro-badge"></span></div>',
        {
          headers: {
            "set-cookie":
              "auth=fictional; Secure; HttpOnly; Domain=.whakoom.com",
          },
        },
      );
    },
  });
  const result = await handler(
    req("login", { username: "example", password: "fictional-password" }),
  );
  assert.equal(result.status, 200);
  const data = await result.json();
  assert.equal(data.user.pro, true);
  assert.equal(JSON.stringify(data).includes("fictional"), false);
  assert.equal(calls.length, 2);
  assert.equal(calls[0][0], "https://www.whakoom.com/login");
  const token = result.headers.get("set-cookie").split(";")[0].split("=")[1];
  assert.match(openSession(token, secret).cookie, /auth=fictional/);
});
test("redirects cannot leak cookies and 429 is respected", async () => {
  let calls = 0;
  const api = new Whakoom("auth=private", async () => {
    calls++;
    return new Response(null, {
      status: 302,
      headers: { location: "https://evil.example/" },
    });
  });
  await assert.rejects(api.request("/login"), /Dirección no permitida/);
  assert.equal(calls, 1);
  for (const path of [
    "http://www.whakoom.com/",
    "//evil.example/x",
    "https://www.whakoom.com.evil/x",
    "https://user:pass@www.whakoom.com/",
  ])
    assert.throws(() => trustedUrl(path));
  const limited = new Whakoom(
    "",
    async () =>
      new Response(null, { status: 429, headers: { "retry-after": "120" } }),
  );
  await assert.rejects(
    limited.request("/"),
    (error) => error.status === 429 && error.retry === 120,
  );
});
test("parsers keep titles and reviews as text and reject foreign cover URLs", () => {
  const items = parseItems(
    '<li class="got-it"><a href="/comics/ABC/title"><img src="https://evil.example/x"><strong>&lt;script&gt;literal&lt;/script&gt;</strong></a></li>',
  );
  assert.equal(items[0].cover, "");
  assert.equal(items[0].title, "<script>literal</script>");
  assert.equal(items[0].owned, true);
  const detail = parseDetail(
    '<div class="comic-detail" data-item-id="42"><div class="b-info"><h1>Title</h1></div></div><div class="wiki-content"><p>Synopsis</p></div><div itemprop="review"><span itemprop="author">reader</span><p itemprop="reviewBody">&lt;script&gt;literal&lt;/script&gt;</p></div>',
    { key: "comicABC" },
  );
  assert.equal(detail.numeric_id, 42);
  assert.equal(detail.comments[0].body, "<script>literal</script>");
});
test("failed remote confirmations never report success", async () => {
  const html =
    '<div class="comic-detail" data-item-id="42"><div class="b-info"><h1>Title</h1></div></div>';
  const api = new Whakoom("", async (_, init) =>
    init.method === "POST"
      ? new Response(JSON.stringify({ d: { RCode: 0, GotIt: true } }))
      : new Response(html),
  );
  await assert.rejects(
    api.change("/comics/ABC/title", "owned", true),
    (error) => error.status === 409,
  );
});
test("all 21 achievements match Desktop IDs and persist after removing comics", () => {
  const library = { entries: {} };
  updateBadges(library, 1);
  for (let n = 0; n < 10; n++)
    library.entries[`comic${n}`] = {
      item: { key: `comic${n}` },
      owned: true,
      read: true,
    };
  assert.equal(updateBadges(library, 2).length, 2);
  assert.equal(updateBadges(library, 3).length, 0);
  library.entries = {};
  assert.equal(badgeList(library).find((b) => b.id === "read-10").earned, true);
  assert.equal(badgeList(library).length, 21);
  for (const id of ["owned-1000", "owned-2000", "owned-4000"])
    assert(badgeList(library).some((b) => b.id === id));
});
test("missing volumes use the earliest gap, including fragmented collections", () => {
  const item = (number) => ({
    key: `comic${number}`,
    title: "Series",
    issue: `#${number}`,
  });
  const library = {
    entries: {
      comic16: { item: item(16), owned: true },
      comic19: { item: item(19), owned: true },
    },
    editions: {
      edicion1: {
        item: { key: "edicion1", title: "Series" },
        complete: true,
        volumes: [item(20), item(19), item(17), item(16), item(5)],
      },
    },
  };
  assert.equal(editionViews(library, true)[0].item.issue, "#5");
  library.entries.comic5 = { item: item(5), owned: true };
  assert.equal(editionViews(library, true)[0].item.issue, "#17");
});

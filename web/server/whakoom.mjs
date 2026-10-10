import { load } from "cheerio";
import { safeCover } from "../src/lib/model.mjs";

export const BASE = "https://www.whakoom.com";
export class ServiceError extends Error {
  constructor(message, status = 502, retry = 0) {
    super(message);
    this.status = status;
    this.retry = retry;
  }
}
export function trustedUrl(path) {
  const url = new URL(path, BASE);
  if (
    url.origin !== BASE ||
    url.username ||
    url.password ||
    url.hash ||
    url.port
  )
    throw new ServiceError("Dirección no permitida.", 400);
  return url;
}
export function itemKey(path) {
  const url = trustedUrl(path);
  const match = /^\/(comics|ediciones)\/([A-Za-z0-9_-]+)(?:\/|$)/.exec(
    url.pathname,
  );
  return match
    ? `${match[1] === "comics" ? "comic" : "edicion"}${match[2]}`
    : "";
}
export function coverUrl(value) {
  if (!value) return "";
  try {
    return safeCover(new URL(value, BASE).href);
  } catch {
    return "";
  }
}
export function identity(html) {
  const $ = load(html),
    username = $("#user-avatar img").attr("alt") ?? "";
  if (!/^[A-Za-z0-9_-]{1,80}$/.test(username)) return null;
  return {
    username,
    name: username,
    avatar: coverUrl($("#user-avatar img").attr("src")),
    pro:
      $("#user-avatar .pro-badge,#uz-info .mn-user-prof .pro-badge").length > 0,
  };
}
export function parseItems(html) {
  const $ = load(html),
    result = new Map();
  $("a[href]").each((_, node) => {
    const a = $(node);
    let url, key;
    try {
      url = trustedUrl(a.attr("href"));
      key = itemKey(url);
    } catch {
      return;
    }
    if (!key || result.has(key) || !a.find("img,strong").length) return;
    const row = a.closest("li,.sresult,.item");
    const title = (
      a.find("strong").first().text() ||
      a.attr("title") ||
      a.find("img").attr("alt") ||
      a.text()
    ).trim();
    if (!title) return;
    const issue =
      row.find(".issue-number").first().text().trim() ||
      a
        .find("span")
        .filter((_, el) => $(el).text().trim().startsWith("#"))
        .first()
        .text()
        .trim();
    const img = a.find("img").first();
    result.set(key, {
      key,
      url: url.href,
      title: title.slice(0, 2000),
      issue,
      cover: coverUrl(
        img.attr("data-src") || img.attr("data-original") || img.attr("src"),
      ),
      publisher: row.find(".publisher,.pub").first().text().trim(),
      owned:
        a.closest(".got-it").length > 0 ||
        row.find("button.rem[data-item-id]").length > 0,
      community_rating:
        Number(row.find(".rate-avg").first().text().replace(",", ".")) || 0,
    });
  });
  return [...result.values()];
}
export function parseDetail(html, item) {
  const $ = load(html),
    txt = (s) => $(s).first().text().trim(),
    attr = (s, a) => $(s).first().attr(a) ?? "";
  const title = txt(
    ".b-info h1 span,.b-info h1,.edition-header h1,h1[itemprop=name]",
  );
  if (!title) throw new ServiceError("Whakoom no devolvió una ficha válida.");
  return {
    item: {
      ...item,
      title,
      cover:
        coverUrl(attr(".comic-cover a,.edition-cover a", "href")) || item.cover,
      owned: $(".comicteca.rem,.owners .rem").length > 0,
      community_rating:
        Number(
          txt(".b-info [itemprop=ratingValue],.rate-avg").replace(",", "."),
        ) || 0,
    },
    description: $(".wiki-content p,.about-this-edition p")
      .map((_, el) => $(el).text().trim())
      .get()
      .join("\n\n"),
    publisher: txt(".lang-pub [itemprop=publisher]"),
    language: txt(".lang-pub [itemprop=inLanguage]"),
    numeric_id: Number(attr(".comic-detail", "data-item-id")),
    wish_kind: attr(".pub-detail", "data-item-type") || "comic",
    wish_id:
      attr(".pub-detail", "data-item-id") || item.key.replace(/^comic/, ""),
    wanted: $("button.wanted.active").length > 0,
    read:
      $(
        ".comic-read .not-readed,.comic-read table.read-list tr[data-item-date]",
      ).length > 0,
    personal_rating:
      Number(
        attr(".my-comic-review .stars,.my-review .stars", "data-item-id"),
      ) || 0,
    votes: txt(".rate-count,[itemprop=ratingCount]"),
    owners: txt(".alsohavethis h2 a span"),
    isbn: $(".barcodes [itemprop=isbn]")
      .map((_, el) => $(el).text().trim())
      .get(),
    comments: $(".review[data-item-id],[itemprop=review]")
      .map((_, el) => ({
        id: $(el).attr("data-item-id") || String(_),
        author: $(el).find("[itemprop=author]").text().trim(),
        body: $(el).find("[itemprop=reviewBody]").text().trim(),
        rating:
          Number(
            $(el).find("[itemprop=ratingValue]").text().replace(",", "."),
          ) || 0,
      }))
      .get()
      .slice(0, 100),
  };
}
export function parsePeople(html) {
  const $ = load(html),
    seen = new Map();
  $("ul.users-list > li,.sresult-user").each((_, el) => {
    const row = $(el),
      a = row.find("a.avatar,p.img a,a.username,a.un").first();
    let username;
    try {
      username = trustedUrl(a.attr("href")).pathname.replace(/^\/|\/$/g, "");
    } catch {
      return;
    }
    if (!/^[A-Za-z0-9_-]{1,80}$/.test(username)) return;
    seen.set(username, {
      username,
      name:
        row
          .find(".user-name,.username,.name,a.un,strong")
          .first()
          .text()
          .trim() || username,
      avatar: coverUrl(row.find("img").first().attr("src")),
      pro: row.find(".pro-badge").length > 0,
    });
  });
  return [...seen.values()];
}
async function boundedText(response, max = 4 * 1024 * 1024) {
  const reader = response.body?.getReader();
  if (!reader) return "";
  const chunks = [];
  let size = 0;
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > max) throw new ServiceError("Respuesta demasiado grande.");
      chunks.push(value);
    }
    return Buffer.concat(chunks).toString("utf8");
  } finally {
    await reader.cancel().catch(() => {});
  }
}
export class Whakoom {
  constructor(cookie = "", fetcher = fetch) {
    this.cookies = new Map(
      cookie
        .split(";")
        .map((v) => v.trim())
        .filter(Boolean)
        .map((v) => {
          const i = v.indexOf("=");
          return [v.slice(0, i), v.slice(i + 1)];
        }),
    );
    this.fetcher = fetcher;
  }
  get cookie() {
    return [...this.cookies].map(([k, v]) => `${k}=${v}`).join("; ");
  }
  async request(path, body, form = false) {
    let url = trustedUrl(path),
      method = body === undefined ? "GET" : "POST";
    const signal = AbortSignal.timeout(20000);
    for (let redirects = 0; redirects <= 5; redirects++) {
      const headers = {
        "User-Agent": "WhakoomWeb/1.0 (unofficial client)",
        Accept: "application/json,text/html",
        Cookie: this.cookie,
      };
      if (method === "POST") {
        headers.Origin = BASE;
        headers.Referer = `${BASE}/`;
        headers["Content-Type"] = form
          ? "application/x-www-form-urlencoded"
          : "application/json; charset=utf-8";
      }
      const response = await this.fetcher(url, {
        method,
        headers,
        body:
          method === "POST"
            ? form
              ? new URLSearchParams(body).toString()
              : JSON.stringify(body)
            : undefined,
        redirect: "manual",
        signal,
      });
      for (const value of response.headers.getSetCookie()) {
        const parts = value.split(";"),
          pair = parts.shift(),
          index = pair.indexOf("=");
        const domain = parts
          .find((p) => /^\s*domain=/i.test(p))
          ?.split("=")[1]
          ?.trim()
          .replace(/^\./, "")
          .toLowerCase();
        if (domain && domain !== "whakoom.com" && domain !== "www.whakoom.com")
          continue;
        if (index < 1 || /[\r\n;]/.test(pair)) continue;
        const name = pair.slice(0, index),
          val = pair.slice(index + 1);
        if (!val || parts.some((p) => /^\s*max-age=0$/i.test(p)))
          this.cookies.delete(name);
        else this.cookies.set(name, val);
      }
      if ([301, 302, 303, 307, 308].includes(response.status)) {
        await response.body?.cancel();
        url = trustedUrl(new URL(response.headers.get("location"), url));
        if ([301, 302, 303].includes(response.status)) method = "GET";
        continue;
      }
      if (!response.ok) {
        await response.body?.cancel();
        if (response.status === 429)
          throw new ServiceError(
            "Whakoom pide una pausa. Volvé a intentar más tarde.",
            429,
            Math.min(
              3600,
              Math.max(30, Number(response.headers.get("retry-after")) || 60),
            ),
          );
        throw new ServiceError(
          response.status === 403
            ? "Whakoom requiere una verificación que este servidor no puede completar."
            : "No se pudo consultar Whakoom.",
        );
      }
      return boundedText(response);
    }
    throw new ServiceError("Demasiadas redirecciones.");
  }
  async post(path, body) {
    let value;
    try {
      value = JSON.parse(await this.request(path, body));
      value = value.d ?? value;
      return typeof value === "string" ? JSON.parse(value) : value;
    } catch (error) {
      if (error instanceof ServiceError) throw error;
      throw new ServiceError("Whakoom no devolvió una respuesta compatible.");
    }
  }
  async login(username, password) {
    const $ = load(await this.request("/login"));
    const token = $("form input[name=__RequestVerificationToken]").attr(
      "value",
    );
    if (!token)
      throw new ServiceError(
        "Whakoom requiere una verificación adicional para iniciar sesión.",
        403,
      );
    const html = await this.request(
      "/login",
      {
        username,
        userpassw: password,
        remember: "true",
        __RequestVerificationToken: token,
        dologin2: "",
      },
      true,
    );
    const user = identity(html) ?? identity(await this.request("/"));
    if (!user)
      throw new ServiceError(
        "No se pudo iniciar sesión. Revisá las credenciales o la verificación de Whakoom.",
        401,
      );
    return user;
  }
  async collection(kind, page) {
    const wished = kind === "wanted";
    const data = await this.post(
      wished ? "/mywishlist.aspx/List" : "/mycollection/comics.aspx/List",
      wished
        ? { p: page, o: 0 }
        : {
            s: "",
            pub: "",
            au: "",
            m: 0,
            r: 0,
            wr: 0,
            idc: -1,
            idp: -1,
            nt: "",
            co: 0,
            p: page,
            lm: true,
          },
    );
    if (
      typeof data.Html !== "string" ||
      (!data.Html.trim() && Array.isArray(data.C) && data.C.length > 0)
    )
      throw new ServiceError(
        "Whakoom no devolvió la colección en un formato compatible.",
      );
    const items = parseItems(data.Html);
    return {
      items,
      next:
        !items.length ||
        String(data.ExtraInfo) === "0" ||
        (wished && String(data.ExtraInfo) === "2")
          ? null
          : page + 1,
    };
  }
  async detail(path) {
    const url = trustedUrl(path),
      key = itemKey(url);
    if (!key) throw new ServiceError("Ficha inválida.", 400);
    return parseDetail(await this.request(url), { key, url: url.href });
  }
  async change(path, field, value) {
    const detail = await this.detail(path),
      key = detail.item.key;
    if (key.startsWith("edicion") && field === "rating") {
      const id = Number(key.slice(7));
      if (!Number.isSafeInteger(id) || id <= 0)
        throw new ServiceError("Serie inválida.", 400);
      const result = await this.post("/wkws.asmx/EditionRate", {
        e: id,
        rt: value,
      });
      if (String(result.ExtraInfo) !== "1")
        throw new ServiceError("Whakoom no confirmó la valoración.", 409);
      const fresh = await this.detail(path);
      if (fresh.personal_rating !== value)
        throw new ServiceError(
          "Whakoom todavía no confirmó la valoración.",
          409,
        );
      return fresh;
    }
    if (key.startsWith("edicion") && !["wanted"].includes(field))
      throw new ServiceError("Abrí un tomo para modificar su estado.", 400);
    if (field === "owned") {
      const result = await this.post("/wkws.asmx/arcm", {
        c: key,
        a: value,
        fe: false,
        em: 0,
      });
      if (![0, 1].includes(result.RCode))
        throw new ServiceError("Whakoom no confirmó el cambio.");
    } else if (field === "wanted")
      await this.post(
        `/wkws.asmx/${value ? "AddWishList" : "RemoveWishList"}`,
        { t: detail.wish_kind, i: detail.wish_id },
      );
    else {
      if (!Number.isSafeInteger(detail.numeric_id) || detail.numeric_id <= 0)
        throw new ServiceError("No se encontró el identificador del tomo.");
      if (field === "read")
        await this.post(
          `/wkws.asmx/${value ? "ComicRead" : "ComicUnRead"}`,
          value
            ? {
                c: detail.numeric_id,
                rr: false,
                crd: "",
                r: detail.personal_rating,
              }
            : { c: detail.numeric_id },
        );
      else
        await this.post("/wkws.asmx/ComicRate", {
          c: detail.numeric_id,
          r: value,
        });
    }
    const fresh = await this.detail(path),
      actual =
        field === "owned"
          ? fresh.item.owned
          : field === "rating"
            ? fresh.personal_rating
            : fresh[field];
    if (actual !== value)
      throw new ServiceError(
        "Whakoom todavía no confirmó el cambio. Actualizá la ficha antes de reintentarlo.",
        409,
      );
    return fresh;
  }
}

import { load } from "cheerio";
import {
  Whakoom,
  ServiceError,
  parseItems,
  parsePeople,
  trustedUrl,
  coverUrl,
} from "../../server/whakoom.mjs";
import {
  openSession,
  sealSession,
  sessionCookie,
  tokenFrom,
} from "../../server/session.mjs";

export const config = {
  path: "/api/whakoom/*",
  excludedPath: "/api/whakoom/login",
  rateLimit: {
    action: "rate_limit",
    aggregateBy: ["domain", "ip"],
    windowSize: 60,
    windowLimit: 60,
  },
};
const response = (data, status = 200, extra = {}) =>
  new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "private, no-store",
      Vary: "Cookie",
      "X-Content-Type-Options": "nosniff",
      ...extra,
    },
  });
async function bodyOf(request) {
  if (!request.headers.get("content-type")?.startsWith("application/json"))
    throw new ServiceError("Formato inválido.", 415);
  if (Number(request.headers.get("content-length")) > 8192)
    throw new ServiceError("Solicitud demasiado grande.", 413);
  const reader = request.body?.getReader(),
    chunks = [];
  let size = 0;
  if (!reader) throw new ServiceError("Solicitud vacía.", 400);
  try {
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      size += value.length;
      if (size > 8192)
        throw new ServiceError("Solicitud demasiado grande.", 413);
      chunks.push(value);
    }
    const body = JSON.parse(Buffer.concat(chunks).toString("utf8"));
    if (!body || typeof body !== "object" || Array.isArray(body))
      throw new ServiceError("Solicitud inválida.", 400);
    return body;
  } catch (error) {
    if (error instanceof ServiceError) throw error;
    throw new ServiceError("Solicitud inválida.", 400);
  } finally {
    await reader.cancel().catch(() => {});
  }
}
export function makeHandler({
  secret = process.env.WHAKOOM_SESSION_KEY,
  fetcher = fetch,
  origin = process.env.WHAKOOM_APP_ORIGIN,
} = {}) {
  return async (request) => {
    try {
      const url = new URL(request.url),
        action = url.pathname.replace(/^\/api\/whakoom\//, "");
      if (!["GET", "POST"].includes(request.method))
        return response({ error: "Método no permitido." }, 405);
      if (
        request.method === "POST" &&
        request.headers.get("origin") !== (origin || url.origin)
      )
        throw new ServiceError("Origen no autorizado.", 403);
      const authenticated = openSession(tokenFrom(request), secret);
      if (action === "session" && request.method === "GET")
        return response({
          authenticated: Boolean(authenticated),
          user: authenticated?.user ?? null,
          configured: /^[a-f0-9]{64}$/i.test(secret ?? ""),
        });
      if (action === "logout" && request.method === "POST")
        return response({ ok: true }, 200, { "Set-Cookie": sessionCookie() });
      if (action === "login" && request.method === "POST") {
        if (!/^[a-f0-9]{64}$/i.test(secret ?? ""))
          throw new ServiceError(
            "El inicio de sesión aún no está configurado en este servidor.",
            503,
          );
        const body = await bodyOf(request);
        if (
          typeof body.username !== "string" ||
          !body.username.trim() ||
          body.username.length > 254 ||
          typeof body.password !== "string" ||
          !body.password ||
          body.password.length > 1024
        )
          throw new ServiceError("Credenciales inválidas.", 400);
        const api = new Whakoom("", fetcher),
          user = await api.login(body.username.trim(), body.password);
        body.password = "";
        return response({ authenticated: true, user }, 200, {
          "Set-Cookie": sessionCookie(
            sealSession({ user, cookie: api.cookie }, secret),
          ),
        });
      }
      if (!authenticated)
        throw new ServiceError("Iniciá sesión para continuar.", 401);
      const api = new Whakoom(authenticated.cookie, fetcher);
      const page = Number(url.searchParams.get("page") || 1);
      if (!Number.isSafeInteger(page) || page < 1 || page > 1000)
        throw new ServiceError("Página inválida.", 400);
      const username =
        url.searchParams.get("user") || authenticated.user.username;
      if (!/^[A-Za-z0-9_-]{1,80}$/.test(username))
        throw new ServiceError("Usuario inválido.", 400);
      let result;
      if (request.method === "GET") {
        if (action === "collection") {
          const kind = url.searchParams.get("kind") || "owned";
          if (!["owned", "wanted"].includes(kind))
            throw new ServiceError("Colección inválida.", 400);
          result = await api.collection(kind, page);
        } else if (action === "detail")
          result = await api.detail(url.searchParams.get("url"));
        else if (action === "edition") {
          const edition = url.searchParams.get("id");
          if (!/^\d{1,12}$/.test(edition ?? ""))
            throw new ServiceError("Serie inválida.", 400);
          const data = await api.post("/pwkws.asmx/EditionComicsPage", {
            e: Number(edition),
            p: page,
            m: 1,
            o: 0,
          });
          const items = parseItems(data.Html || "");
          result = {
            items,
            next:
              items.length && String(data.ExtraInfo) !== "0" ? page + 1 : null,
          };
        } else if (action === "catalog") {
          const query = url.searchParams.get("q")?.trim() || "",
            mode = url.searchParams.get("mode") || "popular";
          if (query.length > 200)
            throw new ServiceError("Búsqueda demasiado larga.", 400);
          if (query) {
            const data = await api.post("/search.aspx/Query", {
              q: query,
              ft: 0,
              fit: "",
              fp: "",
              fl: "",
              p: page,
            });
            result = {
              items: parseItems(data.searchResult || ""),
              next: Number(data.nextPage) > 0 ? Number(data.nextPage) : null,
            };
          } else {
            const paths = {
              popular: "/explore",
              rated: "/explore/top_rated",
              novels: "/explore/oneshot",
              all: "/explore/whole_catalog",
            };
            if (!paths[mode]) throw new ServiceError("Sección inválida.", 400);
            const html = await api.request(`${paths[mode]}?page=${page}`),
              $ = load(html);
            let next = null;
            $("a[href]").each((_, node) => {
              try {
                const target = trustedUrl($(node).attr("href"));
                const value = Number(target.searchParams.get("page"));
                if (
                  target.pathname.startsWith("/explore") &&
                  value > page &&
                  value <= 1000 &&
                  (!next || value < next)
                )
                  next = value;
              } catch {}
            });
            result = { items: parseItems(html), next };
          }
        } else if (action === "people") {
          const relation = url.searchParams.get("relation") || "following";
          if (!["following", "followers"].includes(relation))
            throw new ServiceError("Relación inválida.", 400);
          const html = await api.request(`/${username}/${relation}`),
            $ = load(html);
          const cursor = Number(url.searchParams.get("cursor"));
          if (cursor) {
            const mode = Number($("#hdMode").val());
            if (
              !Number.isSafeInteger(mode) ||
              !Number.isSafeInteger(cursor) ||
              cursor < 1
            )
              throw new ServiceError("Paginación inválida.", 400);
            const data = await api.post("/pwkws.asmx/PProfileFollowPage", {
              m: mode,
              p: cursor,
            });
            const html = Array.isArray(data.Html)
              ? data.Html.join("")
              : data.Html || "";
            result = {
              people: parsePeople(`<ul class="users-list">${html}</ul>`),
              cursor:
                Number(data.ExtraInfo) > 0 ? String(data.ExtraInfo) : null,
            };
          } else
            result = {
              people: parsePeople(html),
              cursor: $("#hdNextPage").val() || null,
            };
        } else if (action === "profile") {
          const html = await api.request(`/${username}`),
            $ = load(html);
          if (!$("#public-profile-h h1").text().trim())
            throw new ServiceError(
              "No se pudo leer este perfil de Whakoom.",
              404,
            );
          result = {
            username,
            name: $("#public-profile-h h1").text().trim() || username,
            avatar: coverUrl(
              $("#public-profile-h .avatar img").first().attr("src"),
            ),
            bio: $("#public-profile-h .bio,#public-profile-h .about")
              .first()
              .text()
              .trim(),
            pro: $("#public-profile-h .pro-badge").length > 0,
            items: parseItems(html),
          };
        } else if (action === "lists") {
          const mode = url.searchParams.get("mode") || "discover",
            paths = {
              discover: "/lists",
              popular: "/lists/most_popular/last_week",
              mine: `/${username}/lists`,
              favorites: `/${username}/listsliked`,
            };
          if (!paths[mode]) throw new ServiceError("Listas inválidas.", 400);
          const $ = load(await api.request(paths[mode])),
            seen = new Map();
          $("a[href*='/lists/']").each((_, node) => {
            const a = $(node);
            let target;
            try {
              target = trustedUrl(a.attr("href"));
            } catch {
              return;
            }
            if (!/^\/[A-Za-z0-9_-]+\/lists\/[^/]+_\d+$/.test(target.pathname))
              return;
            const row = a.closest(".list-item,.v2-list-item");
            const title = row.find("h3 a,h2 a,.title a").first().text().trim();
            if (!title) return;
            seen.set(target.href, {
              url: target.href,
              title,
              cover: coverUrl(
                row.find(".coverSample img,.covers img").first().attr("src"),
              ),
            });
          });
          result = { lists: [...seen.values()].filter((l) => l.title) };
        } else if (action === "list") {
          const path = trustedUrl(url.searchParams.get("url"));
          if (!/^\/[A-Za-z0-9_-]+\/lists\/[^/]+_\d+$/.test(path.pathname))
            throw new ServiceError("Lista inválida.", 400);
          result = { items: parseItems(await api.request(path)), next: null };
        } else throw new ServiceError("Sección no disponible.", 404);
      } else {
        const body = await bodyOf(request);
        if (action === "change") {
          if (
            !["owned", "wanted", "read", "rating"].includes(body.field) ||
            (body.field === "rating"
              ? !Number.isInteger(body.value) ||
                body.value < 0 ||
                body.value > 5
              : typeof body.value !== "boolean")
          )
            throw new ServiceError("Cambio inválido.", 400);
          result = await api.change(body.url, body.field, body.value);
        } else if (action === "review") {
          if (
            typeof body.text !== "string" ||
            body.text.length > 1000 ||
            !Number.isInteger(body.rating) ||
            body.rating < 0 ||
            body.rating > 5
          )
            throw new ServiceError("Opinión inválida.", 400);
          const detail = await api.detail(body.url),
            edition = detail.item.key.startsWith("edicion"),
            id = edition ? Number(detail.item.key.slice(7)) : detail.numeric_id;
          if (!Number.isSafeInteger(id) || id <= 0)
            throw new ServiceError("Ficha inválida.", 400);
          const data = await api.post(
            edition
              ? "/wkws.asmx/UpdateEditionReview"
              : "/wkws.asmx/updateComicReview",
            edition
              ? { e: id, rv: body.text }
              : { c: id, rv: body.text, rt: body.rating },
          );
          if (String(data.ExtraInfo) !== "1")
            throw new ServiceError("Whakoom no confirmó la opinión.", 409);
          const saved = await api.post(
            edition ? "/wkws.asmx/EditionReview" : "/wkws.asmx/ComicReview",
            edition ? { e: id } : { c: id },
          );
          if (
            load(saved.Html || "")("textarea#txtReview")
              .text()
              .replace(/\r\n/g, "\n") !== body.text.replace(/\r\n/g, "\n")
          )
            throw new ServiceError(
              "La opinión todavía no coincide con Whakoom.",
              409,
            );
          result = { ok: true };
        } else throw new ServiceError("Acción no disponible.", 404);
      }
      return response(
        result,
        200,
        api.cookie !== authenticated.cookie
          ? {
              "Set-Cookie": sessionCookie(
                sealSession(
                  { user: authenticated.user, cookie: api.cookie },
                  secret,
                ),
              ),
            }
          : {},
      );
    } catch (error) {
      return response(
        {
          error:
            error instanceof ServiceError
              ? error.message
              : "No se pudo completar la conexión. Volvé a intentar más tarde.",
        },
        error instanceof ServiceError ? error.status : 502,
        error.retry ? { "Retry-After": String(error.retry) } : {},
      );
    }
  };
}
export default makeHandler();

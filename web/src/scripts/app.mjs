import { renderDetail } from "./detail.mjs";
import { createKey, open, seal } from "../lib/crypto.mjs";
import { readVault, saveVault, deleteVault } from "../lib/database.mjs";
import {
  libraryFrom,
  visibleEntries,
  totals,
  editionViews,
} from "../lib/model.mjs";
import { badgeList, updateBadges } from "../lib/badges.mjs";
import { icon } from "../lib/icons.mjs";
import { api, clearOnlineCache } from "../lib/online.mjs";
import { el, btn, img } from "./dom.mjs";
import {
  renderBadges,
  renderStats,
  renderTutorial,
  renderSettings,
} from "./views.mjs";
const $ = (id) => document.getElementById(id);
let library,
  session,
  user,
  section = "library",
  limit = 60,
  timer,
  imported,
  saves = Promise.resolve(),
  generation = 0,
  syncing = false,
  detailKey = null,
  listView = false;
let results = [],
  people = [],
  lists = [],
  next = null,
  mode = "popular",
  relation = "following",
  listMode = "discover";
let libraryMode = "volumes";
let prefs;
try {
  prefs = JSON.parse(localStorage.getItem("whakoom-appearance"));
} catch {}
prefs ||= { theme: "dark", animations: true, sound: false };
const status = (text, error = false) => {
  $("status").textContent = text;
  $("status").classList.toggle("error", error);
};
document.addEventListener("whakoom-error", (e) => status(e.detail, true));
function applyPrefs() {
  document.documentElement.dataset.theme = prefs.theme;
  document.documentElement.classList.toggle("no-motion", !prefs.animations);
  localStorage.setItem("whakoom-appearance", JSON.stringify(prefs));
}
applyPrefs();
function save() {
  clearTimeout(timer);
  timer = null;
  if (!session || !library) return Promise.resolve();
  const data = structuredClone(library),
    key = session;
  saves = saves
    .catch(() => {})
    .then(async () => {
      await saveVault(await seal(data, key));
      status("Guardado cifrado en este navegador.");
    });
  saves.catch((e) => status(e.message, true));
  return saves;
}
function award(quiet = false) {
  const newBadges = updateBadges(library);
  if (quiet || !newBadges.length) return;
  const toast = $("achievement");
  toast.replaceChildren(
    icon("trophy"),
    el("strong", `Insignia desbloqueada · ${newBadges[0].title}`),
    btn("×", () => (toast.hidden = true)),
  );
  toast.hidden = false;
  if (prefs.sound) {
    const Audio = window.AudioContext || window.webkitAudioContext;
    if (Audio) {
      const a = new Audio(),
        o = a.createOscillator(),
        g = a.createGain();
      o.connect(g);
      g.connect(a.destination);
      g.gain.setValueAtTime(0.08, a.currentTime);
      g.gain.exponentialRampToValueAtTime(0.001, a.currentTime + 0.5);
      o.frequency.value = 880;
      o.start();
      o.stop(a.currentTime + 0.5);
      o.onended = () => a.close();
    }
  }
}
function accountButton() {
  $("account").disabled = !user;
  $("account-avatar").replaceChildren(
    user ? img(user.avatar, "avatar") : icon("user"),
  );
  $("account-name").replaceChildren(
    el("strong", user?.username || "Tu cuenta"),
    el("small", user ? "Gestionar tu cuenta" : "Iniciá sesión para comenzar"),
  );
  if (user?.pro) $("account-name").append(el("span", "PRO", "pro"));
}
async function begin(cache) {
  generation++;
  if (!cache) {
    session = null;
    library = { owner: user.username, entries: {} };
  }
  library.owner = user.username;
  $("locked").hidden = $("login").hidden = true;
  $("workspace").hidden = false;
  $("lock").hidden = false;
  award(true);
  render();
  await save();
  if (!localStorage.getItem("whakoom-tutorial")) tutorial();
  sync().catch((e) => status(e.message, true));
}
async function sync() {
  if (syncing || !library) return;
  syncing = true;
  const gen = generation,
    target = library;
  try {
    for (const kind of ["owned", "wanted"]) {
      let page = 1;
      const seen = new Set(),
        pages = new Set();
      while (page && gen === generation) {
        if (pages.has(page) || pages.size >= 1000)
          throw new Error("Whakoom repitió una página de la colección.");
        pages.add(page);
        const data = await api("collection", { kind, page });
        if (gen !== generation) return;
        for (const item of data.items) {
          seen.add(item.key);
          target.entries[item.key] ??= { item, notes: "" };
          target.entries[item.key].item = item;
          target.entries[item.key][kind] = true;
        }
        $("sync-state").textContent =
          `Sincronizando ${kind === "owned" ? "biblioteca" : "deseados"} · ${seen.size} fichas`;
        if (
          !detailKey &&
          ["library", "wanted", "reading", "statistics"].includes(section)
        )
          render();
        page = data.next;
      }
      if (gen !== generation) return;
      for (const [key, e] of Object.entries(target.entries))
        if (!seen.has(key)) e[kind] = false;
    }
    if (gen !== generation) return;
    award(true);
    await save();
    $("sync-state").textContent = "Conectado con Whakoom";
    if (
      !detailKey &&
      ["library", "wanted", "reading", "statistics"].includes(section)
    )
      render();
  } finally {
    if (gen === generation) syncing = false;
  }
}
function tabs(options, active, choose) {
  $("tabs").replaceChildren(
    ...options.map(([value, label]) =>
      btn(label, () => choose(value), value === active ? "active" : ""),
    ),
  );
}
function panel() {
  $("books").hidden = $("statistics").hidden = true;
  $("panel").hidden = false;
  $("panel").replaceChildren();
  return $("panel");
}
const titles = {
  library: "Mi biblioteca",
  reading: "Lecturas",
  wanted: "Deseados",
  statistics: "Tus ritmos",
  badges: "Tus insignias",
  catalog: "Catálogo",
  people: "Personas",
  settings: "Ajustes",
  account: "Tu cuenta",
};
function render() {
  if (!library) return;
  detailKey = null;
  $("heading").textContent = titles[section];
  $("subtitle").textContent =
    {
      catalog: "Encontrá tu próxima historia.",
      people: "Las personas que comparten tus historias.",
      badges: "Cada historia cuenta. Celebrá tu recorrido.",
      statistics: "Tu colección, compras y lecturas.",
    }[section] || "Un lugar para todas tus historias.";
  $("owner").textContent = `Biblioteca de ${library.owner}`;
  document
    .querySelectorAll("[data-section]")
    .forEach((n) =>
      n.classList.toggle("active", n.dataset.section === section),
    );
  $("panel").hidden = true;
  $("panel").replaceChildren();
  $("statistics").hidden = section !== "statistics";
  $("books").hidden = [
    "statistics",
    "badges",
    "settings",
    "account",
    "people",
  ].includes(section);
  $("books").classList.toggle("list-view", listView);
  $("books").replaceChildren();
  $("tabs").replaceChildren();
  $("section-intro").replaceChildren();
  $("empty").hidden = $("more").hidden = true;
  $("search").closest(".toolbar").hidden = [
    "statistics",
    "badges",
    "settings",
    "account",
    "people",
  ].includes(section);
  if (section === "statistics")
    return renderStats($("statistics"), library, totals(library));
  if (section === "badges")
    return renderBadges(panel(), badgeList(library), tilt);
  if (section === "settings")
    return renderSettings(panel(), {
      prefs,
      applyPrefs,
      tutorial,
      session,
      importFile,
      exportBackup,
      remove: async () => {
        if (!confirm("¿Borrar sólo el respaldo local? Tu cuenta no cambia."))
          return;
        await saves;
        await deleteVault();
        await lock();
      },
    });
  if (section === "account") return account();
  if (section === "people") return renderPeople();
  if (section === "catalog") {
    tabs(
      [
        ["popular", "Populares"],
        ["rated", "Mejor valorados"],
        ["novels", "Novelas gráficas"],
        ["all", "Todos"],
        ["users", "Usuarios"],
        ["lists", "Listas"],
      ],
      mode,
      (v) => {
        mode = v;
        results = [];
        lists = [];
        next = null;
        render();
        loadOnline();
      },
    );
    if (mode === "users") {
      const root = panel(),
        form = el("form"),
        input = el("input");
      input.placeholder = "Nombre de usuario de Whakoom";
      input.required = true;
      input.maxLength = 80;
      form.append(input, el("button", "Ver perfil"));
      form.onsubmit = (e) => {
        e.preventDefault();
        openPerson(input.value.trim()).catch((e) => status(e.message, true));
      };
      root.append(
        el("h2", "Encontrá una persona"),
        el(
          "p",
          "Ingresá su nombre de usuario para ver su perfil y conexiones.",
          "hint",
        ),
        form,
      );
      return;
    }
    if (mode === "lists") return renderLists();
    drawBooks(
      results.map((item) => library.entries[item.key] || { item }),
      next !== null,
    );
    return;
  }
  if (section === "library") {
    tabs(
      [
        ["volumes", "Tomos"],
        ["series", "Series"],
        ["missing", "Tomos faltantes"],
      ],
      libraryMode,
      (value) => {
        libraryMode = value;
        render();
      },
    );
    if (libraryMode !== "volumes") {
      const entries = editionViews(
        library,
        libraryMode === "missing",
        $("search").value,
      );
      if (!entries.length)
        $("section-intro").append(
          el(
            "p",
            "Abrí una serie del catálogo o importá sus tomos de Desktop para ver el progreso y el primero que te falta.",
            "hint",
          ),
        );
      drawBooks(entries);
      return;
    }
  }
  drawBooks(visibleEntries(library, section, $("search").value));
}
export function tilt(card) {
  card.onpointermove = (e) => {
    if (!prefs.animations || e.pointerType === "touch") return;
    const r = card.getBoundingClientRect(),
      x = (e.clientX - r.left) / r.width,
      y = (e.clientY - r.top) / r.height;
    card.style.setProperty("--tilt-x", `${(y - 0.5) * -10}deg`);
    card.style.setProperty("--tilt-y", `${(x - 0.5) * 10}deg`);
    card.style.setProperty("--shine-x", `${x * 100}%`);
    card.style.setProperty("--shine-y", `${y * 100}%`);
  };
  card.onpointerleave = () => {
    card.style.removeProperty("--tilt-x");
    card.style.removeProperty("--tilt-y");
  };
}
function drawBooks(entries, more = false) {
  $("books").replaceChildren();
  for (const e of entries.slice(0, limit)) {
    const card = btn("", () => openDetail(e), "book");
    card.append(
      img(e.item.cover),
      el("strong", e.item.title),
      el("small", e.item.issue || e.item.publisher || ""),
    );
    if (e.item.community_rating > 0)
      card.append(
        el(
          "small",
          `★ ${e.item.community_rating.toLocaleString("es", { maximumFractionDigits: 1 })}`,
          "gold",
        ),
      );
    if (e.read || e.reading)
      card.append(
        el(
          "small",
          e.read ? "✓ Leído" : "◇ Leyendo",
          e.read ? "read" : "reading",
        ),
      );
    tilt(card);
    if (e.series) {
      const bar = el("progress");
      bar.max = e.series.volumes.length || 1;
      bar.value = e.series.volumes.length - e.missing;
      card.append(
        bar,
        el(
          "small",
          e.missing === 0 && e.series.complete
            ? "Serie completada"
            : `Te faltan ${e.missing} tomos${e.series.complete ? "" : " · lista parcial"}`,
        ),
      );
    }
    $("books").append(card);
  }
  $("empty").hidden = entries.length > 0;
  $("more").hidden = entries.length <= limit && !more;
}
async function loadOnline(more = false, refresh = false) {
  const gen = generation,
    selected = mode,
    query = $("search").value;
  status("Consultando Whakoom…");
  try {
    if (selected === "users") return;
    if (selected === "lists") {
      const data = await api("lists", { mode: listMode }, undefined, refresh);
      if (gen !== generation || section !== "catalog" || mode !== selected)
        return;
      lists = data.lists;
      render();
    } else {
      const data = await api(
        "catalog",
        { mode: selected, q: query, page: more ? next : 1 },
        undefined,
        refresh,
      );
      if (
        gen !== generation ||
        section !== "catalog" ||
        mode !== selected ||
        $("search").value !== query
      )
        return;
      results = more
        ? [
            ...new Map(
              [...results, ...data.items].map((i) => [i.key, i]),
            ).values(),
          ]
        : data.items;
      next = data.next;
      render();
    }
    status("");
  } catch (e) {
    status(e.message, true);
  }
}
function renderLists() {
  const controls = el("div", undefined, "tabs");
  for (const [value, label] of [
    ["discover", "Descubrir"],
    ["popular", "Populares"],
    ["mine", "Mis listas"],
    ["favorites", "Favoritas"],
  ])
    controls.append(
      btn(
        label,
        () => {
          listMode = value;
          loadOnline();
        },
        listMode === value ? "active" : "",
      ),
    );
  $("section-intro").append(controls);
  for (const list of lists) {
    const card = btn(
      "",
      async () => {
        const data = await api("list", { url: list.url });
        if (section !== "catalog") return;
        $("section-intro").replaceChildren(
          btn("← Listas", render),
          el("h2", list.title),
        );
        drawBooks(data.items.map((item) => ({ item })));
      },
      "book",
    );
    card.append(img(list.cover), el("strong", list.title));
    $("books").append(card);
  }
  $("empty").hidden = lists.length > 0;
}
async function openDetail(entry) {
  entry = library.entries[entry.item.key] ??= entry;
  const gen = generation,
    key = entry.item.key;
  detailKey = key;
  const root = panel();
  $("tabs").replaceChildren();
  root.append(btn("← Volver", render));
  drawDetail(root, entry);
  try {
    const remote = await api(
      "detail",
      { url: entry.item.url },
      undefined,
      true,
    );
    if (gen !== generation || detailKey !== key) return;
    const local = (library.entries[key] ??= { item: remote.item, notes: "" });
    Object.assign(local, {
      item: remote.item,
      owned: remote.item.owned,
      wanted: remote.wanted,
      read: remote.read,
      rating: remote.personal_rating,
    });
    root.replaceChildren(btn("← Volver", render));
    drawDetail(root, local, remote);
    await save();
    if (key.startsWith("edicion"))
      edition(key, gen).catch((e) => status(e.message, true));
  } catch (e) {
    if (gen === generation && detailKey === key) status(e.message, true);
  }
}
function drawDetail(root, entry, remote) {
  renderDetail(root, entry, remote, {
    library,
    mutate,
    openPerson,
    save,
    openDetail,
    render,
    notesChanged: () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        award();
        save();
      }, 500);
      status("Guardando tus notas…");
    },
  });
}
async function mutate(entry, field, value) {
  const gen = generation;
  status("Confirmando en Whakoom…");
  const fresh = await api("change", {}, { url: entry.item.url, field, value });
  if (gen !== generation) return;
  Object.assign(entry, {
    item: fresh.item,
    owned: fresh.item.owned,
    wanted: fresh.wanted,
    read: fresh.read,
    rating: fresh.personal_rating,
  });
  award();
  await save();
  if (detailKey === entry.item.key) {
    const root = panel();
    root.append(btn("← Volver", render));
    drawDetail(root, entry, fresh);
  }
  status("Cambio confirmado en Whakoom.");
}
async function edition(key, gen) {
  let page = 1;
  const seen = new Set(),
    volumes = new Map(),
    grid = el("div", undefined, "books edition-volumes");
  $("panel").append(el("h3", "Tomos de la serie"), grid);
  while (page && gen === generation && detailKey === key) {
    if (seen.has(page)) throw new Error("Página de serie repetida.");
    seen.add(page);
    const data = await api("edition", { id: key.slice(7), page });
    if (gen !== generation || detailKey !== key) return;
    for (const item of data.items) {
      volumes.set(item.key, item);
      const card = btn(
        "",
        () => openDetail(library.entries[item.key] || { item }),
        "book",
      );
      card.append(
        img(item.cover),
        el("strong", item.issue || item.title),
        el(
          "small",
          library.entries[item.key]?.owned ? "✓ Lo tenés" : "Te falta",
        ),
      );
      grid.append(card);
    }
    page = data.next;
  }
  if (gen === generation && !page) {
    library.editions ??= {};
    library.editions[key] = {
      item: library.entries[key].item,
      volumes: [...volumes.values()],
      complete: true,
    };
    award();
    await save();
  }
}
function writeReview(entry) {
  const form = el("form", undefined, "notes-panel"),
    text = el("textarea"),
    submit = el("button", "Publicar en Whakoom", "opinion");
  text.maxLength = 1000;
  text.placeholder = "Compartí tu opinión…";
  form.append(el("h3", "Tu opinión"), text, submit);
  const gen = generation;
  form.onsubmit = async (e) => {
    e.preventDefault();
    submit.disabled = true;
    try {
      await api(
        "review",
        {},
        { url: entry.item.url, text: text.value, rating: entry.rating || 0 },
      );
      if (gen !== generation) return;
      form.remove();
      status("Opinión confirmada en Whakoom.");
      openDetail(entry);
    } catch (e) {
      status(e.message, true);
    } finally {
      submit.disabled = false;
    }
  };
  $("panel").append(form);
  text.focus();
}
async function loadPeople(refresh = false) {
  const gen = generation,
    rel = relation;
  if (rel === "favorites") {
    people = Object.values(library.web_favorite_people || {});
    renderPeople();
    return;
  }
  let cursor;
  const seen = new Map(),
    pages = new Set();
  do {
    if (cursor && pages.has(cursor)) break;
    pages.add(cursor);
    const data = await api(
      "people",
      { relation: rel, ...(cursor ? { cursor } : {}) },
      undefined,
      refresh,
    );
    if (gen !== generation || section !== "people" || relation !== rel) return;
    for (const p of data.people) seen.set(p.username, p);
    people = [...seen.values()];
    renderPeople();
    cursor = data.cursor;
  } while (cursor && pages.size < 500);
}
function renderPeople() {
  const root = panel();
  tabs(
    [
      ["following", "Seguidos"],
      ["followers", "Seguidores"],
      ["favorites", "Favoritos"],
    ],
    relation,
    (v) => {
      relation = v;
      people = [];
      renderPeople();
      loadPeople().catch((e) => status(e.message, true));
    },
  );
  const heading = el("div", undefined, "people-heading");
  heading.append(
    el(
      "h2",
      `${people.length} ${relation === "following" ? "personas que seguís" : relation === "followers" ? "seguidores" : "personas favoritas"}`,
    ),
    btn("⟳", () => loadPeople(true), "icon-button"),
  );
  root.append(heading);
  const carousel = el("div", undefined, "carousel"),
    track = el("div", undefined, "carousel-track");
  for (let copy = 0; copy < 2; copy++) {
    const group = el("div", undefined, "carousel-group");
    if (copy) group.setAttribute("aria-hidden", "true");
    for (const p of people) {
      const card = btn("", () => openPerson(p.username), "person-card");
      if (copy) card.tabIndex = -1;
      card.append(
        img(p.avatar, "avatar"),
        el("strong", p.name || p.username),
        el("small", `@${p.username}`),
      );
      if (p.pro) card.append(el("span", "PRO", "pro"));
      group.append(card);
    }
    track.append(group);
  }
  carousel.append(track);
  root.append(carousel);
  if (!people.length)
    root.append(
      el("p", "Las personas aparecerán acá al consultar tu cuenta.", "empty"),
    );
}
async function openPerson(username) {
  if (!/^[A-Za-z0-9_-]{1,80}$/.test(username))
    throw new Error("Nombre de usuario no disponible.");
  const gen = generation,
    data = await api("profile", { user: username });
  if (gen !== generation) return;
  const root = panel(),
    hero = el("div", undefined, "profile-hero"),
    favorites = (library.web_favorite_people ??= {});
  hero.append(img(data.avatar, "avatar"), el("h2", data.name));
  if (data.pro) hero.append(el("span", "PRO", "pro"));
  hero.append(
    btn(
      favorites[username] ? "♥ Favorito" : "♡ Guardar persona",
      () => {
        if (favorites[username]) delete favorites[username];
        else favorites[username] = data;
        save();
        openPerson(username);
      },
      "wanted",
    ),
  );
  root.append(btn("← Volver", render), hero, el("p", data.bio || ""));
  const controls = el("div", undefined, "tabs");
  for (const rel of ["following", "followers"])
    controls.append(
      btn(rel === "following" ? "Seguidos" : "Seguidores", async () => {
        const data = await api("people", { user: username, relation: rel });
        if (gen !== generation) return;
        const grid = el("div", undefined, "person-grid");
        for (const p of data.people) {
          const card = btn("", () => openPerson(p.username), "person-card");
          card.append(img(p.avatar, "avatar"), el("strong", p.name));
          grid.append(card);
        }
        root.replaceChildren(
          btn("← Perfil", () => openPerson(username)),
          grid,
        );
      }),
    );
  root.append(controls, el("h3", "Actividad y cómics"));
  const grid = el("div", undefined, "books");
  for (const item of data.items) {
    const card = btn(
      "",
      () => openDetail(library.entries[item.key] || { item }),
      "book",
    );
    card.append(img(item.cover), el("strong", item.title));
    grid.append(card);
  }
  root.append(grid);
}
function account() {
  const root = panel(),
    hero = el("div", undefined, "profile-hero");
  hero.append(img(user.avatar, "avatar"), el("h2", user.username));
  if (user.pro) hero.append(el("span", "PRO", "pro"));
  const grid = el("div", undefined, "settings-grid"),
    connection = el("section", undefined, "notes-panel"),
    achievements = el("section", undefined, "notes-panel");
  connection.append(
    el("h2", "Conexión y seguridad"),
    el(
      "p",
      "La sesión se cifra en una cookie HttpOnly y expira en dos horas.",
      "hint",
    ),
    btn("Desconectar y borrar sesión", logout),
  );
  achievements.append(
    el("h2", "Tus logros"),
    el(
      "p",
      `${badgeList(library).filter((b) => b.earned).length} insignias desbloqueadas`,
    ),
    btn("Ver insignias", () => {
      section = "badges";
      render();
    }),
  );
  grid.append(connection, achievements);
  root.append(hero, grid);
}
async function importFile(e) {
  const file = e.target.files[0];
  e.target.value = "";
  if (!file || !session) return;
  try {
    if (file.size > 32 * 1024 * 1024)
      throw new Error("Respaldo demasiado grande.");
    const data = JSON.parse(await file.text());
    if (data?.format) {
      imported = data;
      $("import-dialog").showModal();
    } else await importData(data);
  } catch (e) {
    status(e.message, true);
  }
}
async function importData(data) {
  const candidate = libraryFrom(data);
  if (candidate.owner.toLocaleLowerCase() !== user.username.toLocaleLowerCase())
    throw new Error("Este respaldo pertenece a otra cuenta.");
  if (
    Object.keys(library.entries).length &&
    !confirm(
      "¿Reemplazar tu biblioteca local con el respaldo? No se cambia la cuenta online.",
    )
  )
    return;
  await save();
  generation++;
  syncing = false;
  library = candidate;
  award(true);
  await save();
  section = "library";
  render();
  status("Respaldo importado y guardado cifrado.");
}
async function exportBackup() {
  if (!session) throw new Error("Abrí un guardado cifrado para exportar.");
  await save();
  const url = URL.createObjectURL(
      new Blob([JSON.stringify(await seal(library, session))], {
        type: "application/json",
      }),
    ),
    a = el("a");
  a.href = url;
  a.download = "whakoom-biblioteca.whakoom";
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
async function lock() {
  await save();
  generation++;
  syncing = false;
  library = session = imported = null;
  detailKey = null;
  people = [];
  lists = [];
  results = [];
  clearOnlineCache();
  for (const id of ["panel", "books", "statistics"]) $(id).replaceChildren();
  for (const id of ["import-dialog", "gallery", "tutorial"]) $(id).close();
  $("import-password").value = "";
  $("workspace").hidden = true;
  $("locked").hidden = false;
  $("lock").hidden = true;
  $("achievement").hidden = true;
  status("Biblioteca bloqueada.");
}
async function logout() {
  await api("logout", {}, {});
  await lock();
  user = null;
  $("locked").hidden = true;
  $("login").hidden = false;
  accountButton();
  status("Sesión cerrada.");
}
function tutorial() {
  renderTutorial($("tutorial"));
}
$("login-form").onsubmit = async (e) => {
  e.preventDefault();
  e.submitter.disabled = true;
  try {
    const data = await api(
      "login",
      {},
      { username: $("username").value, password: $("login-password").value },
    );
    user = data.user;
    accountButton();
    $("login").hidden = true;
    $("locked").hidden = false;
    status("");
  } catch (error) {
    status(error.message, true);
  } finally {
    $("login-password").value = "";
    e.submitter.disabled = false;
  }
};
$("unlock-form").onsubmit = async (e) => {
  e.preventDefault();
  if (!user) return;
  e.submitter.disabled = true;
  try {
    const stored = await readVault();
    if (stored) {
      const result = await open(stored, $("password").value);
      library = libraryFrom(result.data);
      if (
        library.owner &&
        library.owner.toLocaleLowerCase() !== user.username.toLocaleLowerCase()
      )
        throw new Error("La biblioteca cifrada pertenece a otra cuenta.");
      session = result.session;
    } else {
      session = await createKey($("password").value);
      library = { owner: user.username, entries: {} };
    }
    $("password").value = "";
    await begin(true);
  } catch (error) {
    library = session = null;
    status(error.message, true);
  } finally {
    e.submitter.disabled = false;
  }
};
$("memory-only").onclick = () => begin(false);
$("disconnect").onclick = () => logout().catch((e) => status(e.message, true));
$("lock").onclick = () => lock().catch((e) => status(e.message, true));
$("account").onclick = () => {
  if (!library) return;
  section = "account";
  render();
};
for (const n of document.querySelectorAll("[data-icon]"))
  n.prepend(icon(n.dataset.icon));
$("search-icon").append(icon("search"));
$("view").append(icon("list"));
for (const n of document.querySelectorAll("[data-section]"))
  n.onclick = () => {
    if (!library) return;
    section = n.dataset.section;
    limit = 60;
    $("search").value = "";
    render();
    if (section === "catalog") loadOnline();
    if (section === "people")
      loadPeople().catch((e) => status(e.message, true));
  };
let debounce;
$("search").oninput = () => {
  limit = 60;
  clearTimeout(debounce);
  if (section === "catalog") debounce = setTimeout(loadOnline, 400);
  else render();
};
$("view").onclick = () => {
  listView = !listView;
  $("view").replaceChildren(icon(listView ? "grid" : "list"));
  render();
};
$("refresh").onclick = () => {
  clearOnlineCache();
  if (section === "catalog") loadOnline(false, true);
  else sync().catch((e) => status(e.message, true));
};
$("more").onclick = () => {
  limit += 60;
  if (section === "catalog" && next) loadOnline(true);
  else render();
};
$("import-form").onsubmit = async (e) => {
  e.preventDefault();
  e.submitter.disabled = true;
  try {
    const result = await open(imported, $("import-password").value);
    await importData(result.data);
    imported = null;
    $("import-dialog").close();
  } catch (error) {
    status(error.message, true);
  } finally {
    $("import-password").value = "";
    e.submitter.disabled = false;
  }
};
$("cancel-import").onclick = () => {
  imported = null;
  $("import-password").value = "";
  $("import-dialog").close();
};
window.addEventListener("beforeunload", (e) => {
  if (timer) {
    e.preventDefault();
    e.returnValue = "";
  }
});
accountButton();
api("session")
  .then((data) => {
    if (data.authenticated) {
      user = data.user;
      accountButton();
      $("login").hidden = true;
      $("locked").hidden = false;
    } else if (!data.configured)
      status(
        "La conexión con Whakoom todavía no está configurada en este servidor.",
      );
  })
  .catch((e) => status(e.message, true));
if ("serviceWorker" in navigator && location.protocol === "https:")
  navigator.serviceWorker.register("/sw.js").catch(() => {});

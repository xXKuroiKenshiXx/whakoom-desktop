import { createKey, open, seal } from "../lib/crypto.mjs";
import { readVault, saveVault, deleteVault } from "../lib/database.mjs";
import {
  libraryFrom,
  visibleEntries,
  safeCover,
  totals,
} from "../lib/model.mjs";

const byId = (id) => document.getElementById(id);
let library = null,
  session = null,
  section = "library",
  limit = 60,
  timer,
  imported;
let saves = Promise.resolve();
const status = (message, error = false) => {
  byId("status").textContent = message;
  byId("status").classList.toggle("error", error);
};
const element = (tag, text, className) => {
  const node = document.createElement(tag);
  if (text !== undefined) node.textContent = text;
  if (className) node.className = className;
  return node;
};
function save() {
  clearTimeout(timer);
  timer = undefined;
  if (!session || !library) return saves;
  const data = structuredClone(library),
    currentSession = session;
  saves = saves
    .catch(() => {})
    .then(async () => {
      await saveVault(await seal(data, currentSession));
      status("Guardado cifrado en este navegador.");
    });
  saves.catch((error) => status(error.message, true));
  return saves;
}
function render() {
  if (!library) return;
  const titles = {
    library: "Mi biblioteca",
    reading: "Lecturas",
    wanted: "Deseados",
    statistics: "Tus ritmos",
  };
  byId("heading").textContent = titles[section];
  byId("subtitle").textContent =
    section === "reading"
      ? "Lo leído y lo que estás leyendo."
      : "Tu colección, cifrada en este navegador.";
  byId("owner").textContent = library.owner
    ? `Biblioteca de ${library.owner}`
    : "Biblioteca local";
  document
    .querySelectorAll("[data-section]")
    .forEach((button) =>
      button.classList.toggle("active", button.dataset.section === section),
    );
  const stats = section === "statistics";
  byId("statistics").hidden = !stats;
  byId("books").hidden = stats;
  byId("empty").hidden = stats;
  byId("more").hidden = true;
  byId("books").replaceChildren();
  if (stats) {
    const data = totals(library),
      nodes = [];
    for (const [label, value] of [
      ["Tomos en tu biblioteca", data.owned],
      ["Leídos", data.read],
      ["Leyendo", data.reading],
      ...data.currencies.entries(),
    ]) {
      const article = element("article");
      article.append(
        element("span", label),
        element(
          "strong",
          new Intl.NumberFormat("es", { maximumFractionDigits: 2 }).format(
            value,
          ),
        ),
      );
      nodes.push(article);
    }
    byId("statistics").replaceChildren(...nodes);
    return;
  }
  const entries = visibleEntries(library, section, byId("search").value);
  byId("empty").hidden = entries.length > 0;
  for (const entry of entries.slice(0, limit)) {
    const button = element("button", undefined, "book");
    const cover = safeCover(entry.item.cover);
    if (cover) {
      const image = document.createElement("img");
      image.src = cover;
      image.alt = "";
      image.loading = "lazy";
      image.decoding = "async";
      image.referrerPolicy = "no-referrer";
      image.addEventListener(
        "error",
        () => image.replaceWith(element("span", "▤", "cover-fallback")),
        { once: true },
      );
      button.append(image);
    } else button.append(element("span", "▤", "cover-fallback"));
    button.append(
      element("strong", entry.item.title),
      element("small", entry.item.issue ?? ""),
    );
    if (entry.read || entry.reading)
      button.append(
        element(
          "small",
          entry.read ? "✓ Leído" : "◇ Leyendo",
          entry.read ? "read" : "reading",
        ),
      );
    button.addEventListener("click", () => detail(entry));
    byId("books").append(button);
  }
  byId("more").hidden = entries.length <= limit;
}
function detail(entry) {
  const content = byId("detail-content");
  content.replaceChildren();
  const cover = safeCover(entry.item.cover);
  if (cover) {
    const image = document.createElement("img");
    image.src = cover;
    image.alt = "";
    image.className = "detail-cover";
    image.referrerPolicy = "no-referrer";
    content.append(image);
  }
  content.append(
    element("p", entry.item.issue ?? "", "eyebrow"),
    element("h2", entry.item.title),
  );
  const actions = element("div", undefined, "detail-actions");
  for (const [field, label] of [
    ["read", "✓ Leído"],
    ["reading", "◇ Leyendo"],
  ]) {
    const button = element(
      "button",
      label,
      `${field} ${entry[field] ? "selected" : ""}`,
    );
    button.addEventListener("click", () => {
      entry[field] = !entry[field];
      if (entry[field]) entry[field === "read" ? "reading" : "read"] = false;
      if (entry.read && !entry.read_date) {
        const now = new Date();
        entry.read_date = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
      }
      save();
      render();
      detail(entry);
    });
    actions.append(button);
  }
  content.append(actions, element("label", "Notas personales"));
  const notes = document.createElement("textarea");
  notes.value = entry.notes ?? "";
  notes.maxLength = 100000;
  notes.addEventListener("input", () => {
    entry.notes = notes.value;
    clearTimeout(timer);
    timer = setTimeout(save, 500);
    status("Guardando tus notas…");
  });
  content.append(
    notes,
    element(
      "p",
      "Los cambios se guardan localmente. Exportá un respaldo para abrirlos en Desktop.",
      "hint",
    ),
  );
  if (!byId("detail").open) byId("detail").showModal();
}
byId("unlock-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  const button = event.submitter;
  button.disabled = true;
  try {
    const stored = await readVault();
    if (stored) {
      const result = await open(stored, byId("password").value);
      library = libraryFrom(result.data);
      session = result.session;
    } else {
      session = await createKey(byId("password").value);
      library = { schema_version: 1, owner: "", entries: {} };
      await save();
    }
    byId("password").value = "";
    byId("locked").hidden = true;
    byId("workspace").hidden = false;
    byId("lock").hidden = false;
    render();
    status("Biblioteca abierta. La clave permanece sólo en memoria.");
  } catch (error) {
    library = null;
    session = null;
    status(error.message, true);
  } finally {
    button.disabled = false;
  }
});
byId("lock").addEventListener("click", async () => {
  try {
    await save();
  } catch {
    return;
  }
  library = session = imported = null;
  byId("detail").close();
  byId("import-dialog").close();
  byId("import-password").value = "";
  byId("detail-content").replaceChildren();
  byId("books").replaceChildren();
  byId("statistics").replaceChildren();
  byId("owner").textContent = "";
  byId("workspace").hidden = true;
  byId("locked").hidden = false;
  byId("lock").hidden = true;
  byId("heading").textContent = "Mi biblioteca";
  status("Biblioteca bloqueada.");
});
document.querySelectorAll("[data-section]").forEach((button) =>
  button.addEventListener("click", () => {
    section = button.dataset.section;
    limit = 60;
    render();
  }),
);
byId("search").addEventListener("input", () => {
  limit = 60;
  render();
});
byId("more").addEventListener("click", () => {
  limit += 60;
  render();
});
async function importData(data) {
  const candidate = libraryFrom(data);
  if (
    Object.keys(library.entries).length &&
    !confirm(
      "¿Reemplazar la biblioteca local con este respaldo? Exportá antes si querés conservarla.",
    )
  )
    return;
  await save();
  const encrypted = await seal(candidate, session);
  await saveVault(encrypted);
  library = candidate;
  limit = 60;
  render();
  status("Respaldo importado y guardado cifrado.");
}
byId("import-file").addEventListener("change", async (event) => {
  const file = event.target.files[0];
  event.target.value = "";
  if (!file || !session) return;
  try {
    if (file.size > 32 * 1024 * 1024)
      throw new Error("El respaldo supera el límite de 32 MB.");
    const data = JSON.parse(await file.text());
    if (data?.format) {
      imported = data;
      byId("import-dialog").showModal();
    } else await importData(data);
  } catch (error) {
    status(error.message, true);
  }
});
byId("import-form").addEventListener("submit", async (event) => {
  event.preventDefault();
  event.submitter.disabled = true;
  try {
    const result = await open(imported, byId("import-password").value);
    await importData(result.data);
    imported = null;
    byId("import-dialog").close();
  } catch (error) {
    status(error.message, true);
  } finally {
    byId("import-password").value = "";
    event.submitter.disabled = false;
  }
});
byId("cancel-import").addEventListener("click", () => {
  imported = null;
  byId("import-password").value = "";
  byId("import-dialog").close();
});
byId("export").addEventListener("click", async () => {
  try {
    await save();
    const envelope = await seal(library, session);
    const url = URL.createObjectURL(
      new Blob([JSON.stringify(envelope)], { type: "application/json" }),
    );
    const link = document.createElement("a");
    link.href = url;
    link.download = "whakoom-biblioteca.whakoom";
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    status("Respaldo exportado. Usa la contraseña de esta biblioteca local.");
  } catch (error) {
    status(error.message, true);
  }
});
byId("remove").addEventListener("click", async () => {
  if (
    !confirm(
      "¿Borrar la biblioteca cifrada de este navegador? No cambia tu cuenta de Whakoom.",
    )
  )
    return;
  try {
    clearTimeout(timer);
    await saves;
    await deleteVault();
    location.reload();
  } catch (error) {
    status(error.message, true);
  }
});
window.addEventListener("beforeunload", (event) => {
  if (timer) {
    event.preventDefault();
    event.returnValue = "";
  }
});
if ("serviceWorker" in navigator && location.protocol === "https:")
  navigator.serviceWorker.register("/sw.js").catch(() => {});

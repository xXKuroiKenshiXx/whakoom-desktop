import { el, btn } from "./dom.mjs";
import { icon } from "../lib/icons.mjs";

export function renderBadges(root, badges, tilt) {
  root.append(
    el(
      "h2",
      `${badges.filter((b) => b.earned).length} de ${badges.length} insignias`,
    ),
    el(
      "p",
      "Logros locales que se conservan en tu respaldo. No equivalen a una suscripción Pro ni a un ranking global.",
      "hint",
    ),
  );
  const grid = el("div", undefined, "badge-grid");
  for (const b of badges) {
    const card = el(
        "article",
        undefined,
        `badge ${b.tier} ${b.earned ? "earned" : ""}`,
      ),
      symbol = el("div", undefined, "badge-symbol"),
      bar = el("progress");
    symbol.append(icon(b.icon));
    bar.max = b.target;
    bar.value = Math.min(b.current, b.target);
    card.append(
      symbol,
      el(
        "small",
        { bronze: "BRONCE", silver: "PLATA", gold: "ORO", prism: "PRISMA" }[
          b.tier
        ],
      ),
      el("h3", b.title),
      el("p", b.description),
      bar,
      el("small", b.earned ? "✓ Desbloqueada" : `${b.current} / ${b.target}`),
    );
    tilt(card);
    grid.append(card);
  }
  root.append(grid);
}
export function renderStats(root, library, data) {
  root.replaceChildren(el("h2", "Tu colección en cifras", "gold"));
  const grid = el("div", undefined, "stat-grid");
  for (const [label, value] of [
    ["Tomos", data.owned],
    ["Leídos", data.read],
    ["Leyendo", data.reading],
    ...data.currencies.entries(),
  ]) {
    const card = el("article");
    card.append(
      el("span", label),
      el(
        "strong",
        new Intl.NumberFormat("es", { maximumFractionDigits: 2 }).format(value),
      ),
    );
    grid.append(card);
  }
  root.append(grid);
  const block = el("section", undefined, "notes-panel"),
    head = el("div", undefined, "year-control"),
    year = el("input");
  year.type = "number";
  year.min = "1900";
  year.max = "2200";
  year.value = new Date().getFullYear();
  head.append(el("label", "Año"), year);
  const chart = el("div", undefined, "rhythm-chart");
  const draw = () => {
    chart.replaceChildren();
    const months = Array.from({ length: 12 }, () => [0, 0]);
    for (const e of Object.values(library.entries).filter((e) =>
      e.item.key.startsWith("comic"),
    ))
      for (const [i, date] of [
        [0, e.purchase_date],
        [1, e.read_date],
        ...(Array.isArray(e.rereads) ? e.rereads.map((date) => [1, date]) : []),
      ])
        if (
          typeof date === "string" &&
          /^\d{4}-\d{2}-\d{2}$/.test(date) &&
          date.startsWith(`${year.value}-`)
        ) {
          const m = Number(date.slice(5, 7)) - 1;
          if (m >= 0 && m < 12) months[m][i]++;
        }
    const max = Math.max(1, ...months.flat());
    months.forEach((counts, m) => {
      const col = el("div", undefined, "chart-month"),
        bars = el("div", undefined, "chart-bars");
      counts.forEach((count, i) => {
        const bar = el("div", undefined, i ? "reading-bar" : "purchase-bar");
        bar.style.height = `${Math.max(2, (count / max) * 140)}px`;
        bar.title = `${count} ${i ? "lecturas" : "compras"}`;
        bars.append(bar);
      });
      col.append(
        bars,
        el(
          "small",
          new Intl.DateTimeFormat("es", { month: "short" }).format(
            new Date(2026, m, 1),
          ),
        ),
      );
      chart.append(col);
    });
  };
  year.onchange = draw;
  draw();
  block.append(
    el("h2", "Tu ritmo de compras y lectura"),
    head,
    chart,
    el(
      "p",
      "Compras · celeste / Lecturas · violeta. Sólo se cuentan fechas registradas; importar no equivale a comprar.",
      "hint",
    ),
  );
  root.append(block);
}
export function renderSettings(
  root,
  { prefs, applyPrefs, tutorial, session, importFile, exportBackup, remove },
) {
  const grid = el("div", undefined, "settings-grid"),
    appearance = el("section", undefined, "notes-panel");
  appearance.append(el("h2", "Apariencia"));
  const label = el("label", "Tema"),
    theme = el("select");
  for (const [v, text] of [
    ["dark", "Oscuro"],
    ["light", "Claro"],
  ]) {
    const opt = el("option", text);
    opt.value = v;
    theme.append(opt);
  }
  theme.value = prefs.theme;
  theme.onchange = () => {
    prefs.theme = theme.value;
    applyPrefs();
  };
  label.append(theme);
  appearance.append(label);
  for (const [field, text] of [
    ["animations", "Animaciones y efectos"],
    ["sound", "Sonido de insignias"],
  ]) {
    const wrap = el("label", text, "switch"),
      input = el("input");
    input.type = "checkbox";
    input.checked = prefs[field];
    input.onchange = () => {
      prefs[field] = input.checked;
      applyPrefs();
    };
    wrap.prepend(input);
    appearance.append(wrap);
  }
  appearance.append(btn("Ver tutorial", tutorial));
  const storage = el("section", undefined, "notes-panel");
  storage.append(
    el("h2", "Almacenamiento"),
    el(
      "p",
      session
        ? "Biblioteca cifrada en este navegador."
        : "Tus datos locales duran sólo esta sesión.",
      "hint",
    ),
  );
  const importLabel = el("label", "Importar respaldo", "button"),
    file = el("input");
  importLabel.htmlFor = "import-file";
  file.id = "import-file";
  file.type = "file";
  file.accept = ".json,.whakoom";
  file.hidden = true;
  file.disabled = !session;
  file.onchange = importFile;
  const exportBtn = btn("Exportar cifrado ↗", exportBackup);
  exportBtn.id = "export";
  exportBtn.disabled = !session;
  const removeBtn = btn("Borrar biblioteca de este navegador", remove);
  removeBtn.id = "remove";
  storage.append(importLabel, file, exportBtn, removeBtn);
  grid.append(appearance, storage);
  root.append(
    grid,
    el(
      "p",
      "Las portadas usan la caché HTTP del navegador. La web no guarda una copia adicional ni conserva contraseñas.",
      "hint",
    ),
  );
}
const steps = [
  [
    "Dos valoraciones, dos colores",
    "Las doradas son la valoración de la comunidad. Las violetas son tu puntuación y se guardan en Whakoom desde la ficha.",
    "ratings",
  ],
  [
    "Las personas de tus historias",
    "Seguidos y seguidores vienen de Whakoom. Tus personas favoritas son una selección local que se guarda en el respaldo.",
    "people",
  ],
  [
    "Cada historia cuenta",
    "Las 21 insignias celebran tus lecturas, tomos, series y notas. Son locales y se conservan aunque tu biblioteca cambie.",
    "trophy",
  ],
  [
    "Conocé tus ritmos",
    "Las estadísticas usan fechas registradas. Importar no cuenta como comprar. Tus notas e importes son locales; este cliente no incluye todas las funciones Pro.",
    "chart",
  ],
];
export function renderTutorial(dialog) {
  let step = 0;
  const draw = () => {
    const [title, text, symbol] = steps[step],
      root = document.getElementById("tutorial-content"),
      visual = el("div", undefined, "tutorial-visual");
    root.replaceChildren(
      el("p", "TU APP, A TU MANERA", "eyebrow"),
      el("h2", title),
    );
    if (symbol === "ratings")
      for (const [shape, color] of [
        ["star", "gold"],
        ["violet", "violet"],
      ]) {
        const row = el("div", undefined, `rating ${color}`);
        for (let n = 0; n < 5; n++) row.append(icon(shape));
        visual.append(row);
      }
    else visual.append(icon(symbol));
    root.append(visual, el("p", text));
    document.getElementById("tutorial-back").disabled = step === 0;
    document.getElementById("tutorial-next").textContent =
      step === steps.length - 1 ? "Empezar →" : "Siguiente →";
  };
  document.getElementById("tutorial-back").onclick = () => {
    step--;
    draw();
  };
  document.getElementById("tutorial-next").onclick = () => {
    if (++step === steps.length) {
      localStorage.setItem("whakoom-tutorial", "1");
      dialog.close();
    } else draw();
  };
  draw();
  dialog.showModal();
}

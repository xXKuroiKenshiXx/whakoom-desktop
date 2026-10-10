import { el, btn, img } from "./dom.mjs";
import { icon } from "../lib/icons.mjs";
import { safeCover } from "../lib/model.mjs";
const $ = (id) => document.getElementById(id);
export function renderDetail(root, entry, remote, context) {
  const { library, mutate, openPerson, save, openDetail, render } = context;
  const layout = el("article", undefined, "detail-layout"),
    cover = el("div", undefined, "cover-column"),
    zoom = btn(
      "",
      () => {
        const url = safeCover(entry.item.cover);
        if (url) {
          $("gallery-image").src = url;
          $("gallery").showModal();
        }
      },
      "cover-zoom",
    );
  zoom.setAttribute("aria-label", "Ampliar portada");
  zoom.append(img(entry.item.cover, "detail-cover"));
  cover.append(zoom);
  const info = el("div", undefined, "detail-info");
  info.append(
    el("p", entry.item.issue || "SERIE", "eyebrow"),
    el("h2", entry.item.title),
  );
  const community = el("div", undefined, "rating gold");
  for (let n = 1; n <= 5; n++) {
    const s = icon("star");
    if (n <= Math.round(entry.item.community_rating || 0))
      s.classList.add("filled");
    community.append(s);
  }
  community.append(
    el(
      "span",
      (entry.item.community_rating || 0).toLocaleString("es", {
        minimumFractionDigits: 1,
      }),
    ),
  );
  if (remote?.votes) community.append(el("span", remote.votes, "votes"));
  const personal = el("div", undefined, "rating violet");
  for (let n = 1; n <= 5; n++) {
    const b = btn("", () => mutate(entry, "rating", n), "star-button");
    b.setAttribute("aria-label", `Valorar con ${n} estrellas`);
    const s = icon("violet");
    if (n <= (entry.rating || 0)) s.classList.add("filled");
    b.append(s);
    personal.append(b);
  }
  personal.append(
    el(
      "span",
      (entry.rating || 0).toLocaleString("es", { minimumFractionDigits: 1 }),
    ),
  );
  info.append(community, personal);
  if (remote?.publisher) info.append(el("p", remote.publisher, "hint"));
  if (remote?.isbn?.length)
    info.append(el("p", `ISBN · ${remote.isbn.join(" · ")}`, "hint"));
  if (remote?.owners)
    info.append(el("p", `${remote.owners} personas lo tienen`, "hint"));
  const actions = el("div", undefined, "detail-actions");
  if (entry.item.key.startsWith("comic"))
    for (const [field, label, mark] of [
      ["owned", "Lo tengo", "shelf"],
      ["wanted", "Lo quiero", "heart"],
      ["read", "Leído", "book"],
    ]) {
      const b = btn(
        "",
        () => mutate(entry, field, !entry[field]),
        `${field} ${entry[field] ? "selected" : ""}`,
      );
      b.append(icon(mark), el("span", label));
      actions.append(b);
    }
  actions.append(
    btn("Escribir o editar mi opinión", () => writeReview(entry), "opinion"),
  );
  info.append(actions);
  layout.append(cover, info);
  root.append(layout);
  if (remote?.description)
    root.append(
      el("h3", "Sinopsis"),
      el("p", remote.description.normalize("NFKC"), "synopsis"),
    );
  if (entry.item.key.startsWith("comic")) notes(root, entry, context);
  if (remote) {
    const comments = el("section", undefined, "comments");
    comments.append(el("h3", "Opiniones de la comunidad"));
    const reactions = (library.web_reactions ??= {});
    for (const comment of [...remote.comments].sort(
      (a, b) =>
        (reactions[`${entry.item.key}:${b.id}`] || 0) -
        (reactions[`${entry.item.key}:${a.id}`] || 0),
    )) {
      const c = el("article", undefined, "comment");
      c.append(
        btn(
          comment.author || "Lector",
          () => openPerson(comment.author),
          "text-button",
        ),
        el("p", comment.body.normalize("NFKC")),
      );
      const controls = el("div", undefined, "comment-actions"),
        key = `${entry.item.key}:${comment.id}`;
      for (const [v, label] of [
        [1, "♡"],
        [-1, "↓"],
      ])
        controls.append(
          btn(
            label,
            () => {
              reactions[key] = reactions[key] === v ? 0 : v;
              save();
              root.replaceChildren(btn("← Volver", render));
              renderDetail(root, entry, remote, context);
            },
            reactions[key] === v ? "selected" : "",
          ),
        );
      c.append(controls);
      comments.append(c);
    }
    root.append(comments);
  }
}
function notes(root, entry, context) {
  const { save, openDetail, notesChanged } = context;
  const block = el("section", undefined, "notes-panel"),
    grid = el("div", undefined, "form-grid");
  block.append(el("h3", "Tu lectura y tus notas"));
  for (const [field, label] of [
    ["read_date", "Fecha de lectura"],
    ["purchase_date", "Fecha de compra"],
  ]) {
    const wrap = el("label", label),
      input = el("input");
    input.type = "date";
    input.value = entry[field] || "";
    input.onchange = () => {
      entry[field] = input.value;
      save();
    };
    wrap.append(input);
    grid.append(wrap);
  }
  const costLabel = el("label", "Importe pagado"),
    cost = el("input");
  cost.type = "number";
  cost.min = "0";
  cost.max = "100000000";
  cost.step = ".01";
  cost.value = entry.cost || "";
  cost.onchange = () => {
    const n = Number(cost.value);
    if (Number.isFinite(n) && n >= 0 && n <= 1e8) {
      entry.cost = n;
      save();
    }
  };
  costLabel.append(cost);
  const currencyLabel = el("label", "Moneda"),
    currency = el("select");
  for (const code of ["ARS", "EUR", "USD", "BRL", "CLP", "MXN", "UYU"]) {
    const option = el("option", code);
    option.value = code;
    currency.append(option);
  }
  currency.value = entry.currency || "ARS";
  currency.onchange = () => {
    entry.currency = currency.value;
    save();
  };
  currencyLabel.append(currency);
  grid.append(costLabel, currencyLabel);
  block.append(
    grid,
    btn(entry.reading ? "✓ Leyendo" : "◇ Marcar como leyendo", () => {
      entry.reading = !entry.reading;
      save();
      openDetail(entry);
    }),
    el("label", "Notas personales"),
  );
  const wrap = el("div", undefined, "notes-wrap"),
    text = el("textarea");
  text.value = entry.notes || "";
  text.maxLength = 100000;
  text.placeholder = "Qué te dejó esta historia…";
  text.oninput = () => {
    entry.notes = text.value;
    notesChanged();
  };
  const emojis = el("select", undefined, "emoji-picker");
  emojis.setAttribute("aria-label", "Añadir emoji");
  for (const e of ["☺", "💜", "📖", "✨", "😭", "🔥", "⭐"])
    emojis.append(el("option", e));
  emojis.onchange = () => {
    text.setRangeText(
      emojis.value,
      text.selectionStart,
      text.selectionEnd,
      "end",
    );
    text.dispatchEvent(new Event("input"));
    emojis.selectedIndex = 0;
    text.focus();
  };
  wrap.append(text, emojis);
  block.append(
    wrap,
    el(
      "p",
      "Las notas, fechas, importes y Leyendo son locales. Los estados, valoraciones y opiniones se confirman en Whakoom.",
      "hint",
    ),
  );
  root.append(block);
}

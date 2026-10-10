import { safeCover } from "../lib/model.mjs";
export const el = (tag, text, cls) => {
  const n = document.createElement(tag);
  if (text !== undefined) n.textContent = text;
  if (cls) n.className = cls;
  return n;
};
export const btn = (text, action, cls = "") => {
  const n = el("button", text, cls);
  n.type = "button";
  n.onclick = () =>
    Promise.resolve()
      .then(action)
      .catch((e) =>
        document.dispatchEvent(
          new CustomEvent("whakoom-error", { detail: e.message }),
        ),
      );
  return n;
};
export const img = (url, cls = "") => {
  const src = safeCover(url);
  if (!src) return el("span", "▤", `cover-fallback ${cls}`);
  const n = el("img", undefined, cls);
  n.src = src;
  n.alt = "";
  n.loading = "lazy";
  n.decoding = "async";
  n.referrerPolicy = "no-referrer";
  n.onerror = () => n.replaceWith(el("span", "▤", `cover-fallback ${cls}`));
  return n;
};

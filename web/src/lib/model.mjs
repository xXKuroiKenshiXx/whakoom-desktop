export function safeCover(value) {
  try {
    const url = new URL(value);
    return url.protocol === "https:" &&
      !url.username &&
      !url.password &&
      !url.port &&
      (url.hostname === "whakoom.com" || url.hostname.endsWith(".whakoom.com"))
      ? url.href
      : "";
  } catch {
    return "";
  }
}
export function libraryFrom(data) {
  if (
    !data ||
    typeof data !== "object" ||
    !data.entries ||
    Array.isArray(data.entries) ||
    typeof data.owner !== "string" ||
    data.owner.length > 80 ||
    Object.keys(data.entries).length > 100000
  ) {
    throw new Error("El archivo no es una biblioteca compatible.");
  }
  const entries = Object.create(null);
  for (const [key, entry] of Object.entries(data.entries)) {
    if (
      !/^(comic|edicion)[A-Za-z0-9_-]+$/.test(key) ||
      entry?.item?.key !== key ||
      typeof entry.item.title !== "string" ||
      entry.item.title.length > 2000 ||
      (entry.notes !== undefined &&
        (typeof entry.notes !== "string" || entry.notes.length > 100000)) ||
      (entry.cost !== undefined &&
        (!Number.isFinite(entry.cost) ||
          entry.cost < 0 ||
          entry.cost > 100000000)) ||
      ["owned", "read", "reading", "wanted"].some(
        (name) => entry[name] !== undefined && typeof entry[name] !== "boolean",
      )
    ) {
      throw new Error("El respaldo contiene una ficha inválida.");
    }
    entries[key] = structuredClone(entry);
  }
  // Explicitly omit credentials and unknown top-level fields.
  const result = { schema_version: 1, owner: data.owner, entries };
  for (const field of [
    "editions",
    "outbox",
    "account",
    "friends",
    "followers",
    "favorite_people",
    "inbox",
    "reactions",
    "recent",
    "reading_order",
    "attachments",
    "online_readings",
    "badges",
    "web_favorite_people",
    "web_reactions",
  ]) {
    if (Object.hasOwn(data, field))
      result[field] = structuredClone(data[field]);
  }
  return result;
}
export function visibleEntries(library, section, query = "") {
  return Object.values(library.entries).filter(
    (entry) =>
      entry.item.key.startsWith("comic") &&
      (section === "reading"
        ? entry.read || entry.reading
        : section === "wanted"
          ? entry.wanted
          : entry.owned) &&
      `${entry.item.title} ${entry.item.issue ?? ""}`
        .toLocaleLowerCase()
        .includes(query.toLocaleLowerCase()),
  );
}
export function totals(library) {
  const entries = Object.values(library.entries).filter((entry) =>
    entry.item.key.startsWith("comic"),
  );
  const currencies = new Map();
  for (const entry of entries)
    if (entry.owned && Number.isFinite(entry.cost) && entry.cost > 0) {
      const currency =
        typeof entry.currency === "string" && entry.currency
          ? entry.currency
          : "Sin moneda";
      currencies.set(currency, (currencies.get(currency) ?? 0) + entry.cost);
    }
  return {
    owned: entries.filter((e) => e.owned).length,
    read: entries.filter((e) => e.read).length,
    reading: entries.filter((e) => e.reading && !e.read).length,
    currencies,
  };
}
export function editionViews(library, missingOnly = false, query = "") {
  const comparator = new Intl.Collator("es", {
    numeric: true,
    sensitivity: "base",
  });
  return Object.values(library.editions ?? {})
    .filter(
      (e) =>
        e.item &&
        Array.isArray(e.volumes) &&
        e.volumes.some((v) => library.entries[v.key]?.owned),
    )
    .map((e) => {
      const missing = e.volumes
        .filter((v) => !library.entries[v.key]?.owned)
        .sort((a, b) => comparator.compare(a.issue || "", b.issue || ""));
      if (missingOnly && !missing.length) return null;
      return {
        item: missingOnly ? missing[0] : e.item,
        series: e,
        missing: missing.length,
      };
    })
    .filter(
      (e) =>
        e &&
        e.item.title.toLocaleLowerCase().includes(query.toLocaleLowerCase()),
    );
}

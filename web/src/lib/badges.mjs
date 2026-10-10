export const definitions = [
  ["read-10", "Primera lectura", "Leé 10 tomos", "book", "bronze", "read", 10],
  ["read-50", "Lector constante", "Leé 50 tomos", "book", "silver", "read", 50],
  ["read-100", "Centena", "Leé 100 tomos", "trophy", "gold", "read", 100],
  [
    "owned-10",
    "Estantería inicial",
    "Añadí 10 tomos",
    "shelf",
    "bronze",
    "owned",
    10,
  ],
  [
    "owned-100",
    "Gran colección",
    "Añadí 100 tomos",
    "shelf",
    "gold",
    "owned",
    100,
  ],
  [
    "collections-5",
    "Coleccionista",
    "Seguí 5 series",
    "grid",
    "silver",
    "collections",
    5,
  ],
  [
    "wanted-10",
    "En la mira",
    "Guardá 10 tomos deseados",
    "heart",
    "bronze",
    "wanted",
    10,
  ],
  [
    "rated-10",
    "Criterio propio",
    "Valorá 10 tomos",
    "star",
    "bronze",
    "rated",
    10,
  ],
  [
    "read-250",
    "Maratón de historias",
    "Leé 250 tomos",
    "book",
    "prism",
    "read",
    250,
  ],
  [
    "read-500",
    "Biblioteca vivida",
    "Leé 500 tomos",
    "trophy",
    "prism",
    "read",
    500,
  ],
  [
    "owned-250",
    "Estanterías sin fin",
    "Añadí 250 tomos",
    "shelf",
    "prism",
    "owned",
    250,
  ],
  [
    "owned-1000",
    "Biblioteca monumental",
    "Añadí 1000 tomos",
    "shelf",
    "prism",
    "owned",
    1000,
  ],
  [
    "owned-2000",
    "Archivo de historias",
    "Añadí 2000 tomos",
    "trophy",
    "prism",
    "owned",
    2000,
  ],
  [
    "owned-4000",
    "Universo de papel",
    "Añadí 4000 tomos",
    "trophy",
    "prism",
    "owned",
    4000,
  ],
  [
    "owned-500",
    "Archivo legendario",
    "Añadí 500 tomos",
    "trophy",
    "prism",
    "owned",
    500,
  ],
  [
    "complete-1",
    "Primera serie completa",
    "Tené todos los tomos de una serie",
    "shield",
    "bronze",
    "complete",
    1,
  ],
  [
    "complete-10",
    "Cerrando historias",
    "Completá 10 series",
    "shield",
    "gold",
    "complete",
    10,
  ],
  [
    "notes-10",
    "Entre líneas",
    "Escribí notas en 10 tomos",
    "quill",
    "silver",
    "notes",
    10,
  ],
  [
    "reread-10",
    "Volver a casa",
    "Registrá 10 relecturas con fecha",
    "refresh",
    "silver",
    "rereads",
    10,
  ],
  [
    "organized-25",
    "Todo en su lugar",
    "Organizá 25 tomos con etiquetas o ubicación",
    "list",
    "silver",
    "organized",
    25,
  ],
  ["rated-50", "Voz de lector", "Valorá 50 tomos", "star", "gold", "rated", 50],
];
export function badgeList(library) {
  const entries = Object.values(library.entries).filter((e) =>
    e.item.key.startsWith("comic"),
  );
  const editions = Object.values(library.editions ?? {}).filter((e) =>
    Array.isArray(e.volumes),
  );
  const counts = Object.fromEntries(
    ["owned", "read", "wanted"].map((field) => [
      field,
      entries.filter((e) => e[field]).length,
    ]),
  );
  counts.collections = editions.filter((e) =>
    e.volumes.some((v) => library.entries[v.key]?.owned),
  ).length;
  counts.complete = editions.filter(
    (e) =>
      e.complete &&
      e.volumes.length &&
      e.volumes.every((v) => library.entries[v.key]?.owned),
  ).length;
  counts.notes = entries.filter((e) => e.notes?.trim()).length;
  counts.rated = entries.filter((e) => e.rating > 0).length;
  counts.organized = entries.filter(
    (e) => e.owned && (e.tags?.trim() || e.location?.trim()),
  ).length;
  counts.rereads = entries.reduce(
    (n, e) =>
      n +
      (Array.isArray(e.rereads)
        ? e.rereads.filter((d) => /^\d{4}-\d{2}-\d{2}$/.test(d)).length
        : 0),
    0,
  );
  return definitions.map(
    ([id, title, description, icon, tier, field, target]) => ({
      id,
      title,
      description,
      icon,
      tier,
      target,
      current: counts[field] ?? 0,
      earned: Boolean(library.badges?.earned?.[id]) || counts[field] >= target,
    }),
  );
}
export function updateBadges(library, now = Math.floor(Date.now() / 1000)) {
  const progress = (library.badges ??= { known: [], earned: {} });
  if (!Array.isArray(progress.known)) progress.known = [];
  if (!progress.earned || typeof progress.earned !== "object")
    progress.earned = {};
  const known = new Set(progress.known),
    newly = [];
  for (const badge of badgeList(library)) {
    if (badge.current >= badge.target && !progress.earned[badge.id]) {
      progress.earned[badge.id] = now;
      if (known.has(badge.id)) newly.push(badge);
    }
    known.add(badge.id);
  }
  progress.known = [...known];
  return newly;
}

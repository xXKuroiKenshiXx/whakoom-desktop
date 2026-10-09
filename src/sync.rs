use crate::{api::Item, storage::Library};
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Change {
    Owned(bool),
    Wanted(bool),
    Read { read: bool, date: String },
    Rating(u8),
    Notes(String),
    Review(crate::reviews::Draft),
    EditionFavorite(bool),
    EditionOwned(bool),
}
impl Change {
    pub fn title(&self) -> &'static str {
        match self {
            Self::Owned(_) => "Colección",
            Self::Wanted(_) | Self::EditionFavorite(_) => "Deseados",
            Self::Read { .. } => "Lecturas",
            Self::Rating(_) => "Valoraciones",
            Self::Notes(_) => "Notas personales",
            Self::Review(_) => "Opiniones",
            Self::EditionOwned(_) => "Colecciones completas",
        }
    }
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Owned(_) => "owned",
            Self::Wanted(_) => "wanted",
            Self::Read { .. } => "read",
            Self::Rating(_) => "rating",
            Self::Notes(_) => "notes",
            Self::Review(_) => "review",
            Self::EditionFavorite(_) => "favorite",
            Self::EditionOwned(_) => "edition-owned",
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pending {
    pub item: Item,
    pub change: Change,
    #[serde(default)]
    pub error: String,
    #[serde(default)]
    pub retry_at: u64,
    #[serde(default)]
    pub attempts: u32,
}
impl Pending {
    pub fn unavailable(&self) -> bool {
        self.error.contains("no permite") || self.error.contains("no dispone")
    }
    pub fn key(&self) -> String {
        format!("{}:{}", self.item.key, self.change.kind())
    }
}
pub fn enqueue(library: &mut Library, item: &Item, change: Change) {
    if item.key.starts_with("edicion")
        && let Change::Wanted(value) | Change::EditionFavorite(value) = &change
    {
        library.ensure(item).wanted = *value;
        let saved = library.editions.entry(item.key.clone()).or_default();
        saved.item = item.clone();
        saved.favorite = *value;
        let alternate = if matches!(change, Change::EditionFavorite(_)) {
            "wanted"
        } else {
            "favorite"
        };
        library.outbox.remove(&format!("{}:{alternate}", item.key));
    }
    if matches!(change, Change::Review(_)) {
        library.outbox.remove(&format!("{}:rating", item.key));
    }
    if let Change::Rating(rating) = &change
        && let Some(pending) = library.outbox.get_mut(&format!("{}:review", item.key))
        && let Change::Review(draft) = &mut pending.change
    {
        draft.rating = *rating;
    }
    // Editing one volume supersedes the bulk intent for that volume. Expand the
    // rest into durable individual intents so a later bulk retry cannot undo it.
    if matches!(change, Change::Owned(_)) {
        let affected: Vec<_> = library
            .outbox
            .values()
            .filter_map(|pending| {
                let Change::EditionOwned(desired) = pending.change else {
                    return None;
                };
                let edition = library.editions.get(&pending.item.key)?;
                edition
                    .volumes
                    .iter()
                    .any(|volume| volume.key == item.key)
                    .then(|| (pending.key(), desired, edition.volumes.clone()))
            })
            .collect();
        for (key, desired, volumes) in affected {
            library.outbox.remove(&key);
            for volume in volumes {
                let key = format!("{}:owned", volume.key);
                library.outbox.entry(key).or_insert_with(|| Pending {
                    item: volume,
                    change: Change::Owned(desired),
                    error: String::new(),
                    retry_at: 0,
                    attempts: 0,
                });
            }
        }
    }
    let error = if matches!(change, Change::Notes(_)) {
        library
            .outbox
            .values()
            .find(|p| {
                notes_permission_denied(p)
                    || (p.item.key == item.key
                        && matches!(p.change, Change::Notes(_))
                        && p.unavailable())
            })
            .map(|p| p.error.clone())
            .unwrap_or_default()
    } else {
        String::new()
    };
    let pending = Pending {
        item: item.clone(),
        change,
        error,
        retry_at: 0,
        attempts: 0,
    };
    library.outbox.insert(pending.key(), pending);
}

fn notes_permission_denied(pending: &Pending) -> bool {
    matches!(pending.change, Change::Notes(_))
        && pending
            .error
            .starts_with("Whakoom no permite editar notas con los permisos actuales")
}

pub fn pause_unavailable_notes(library: &mut Library, pending: &Pending, error: &str) {
    if !matches!(pending.change, Change::Notes(_))
        || !error.starts_with("Whakoom no permite editar notas con los permisos actuales")
    {
        return;
    }
    for current in library.outbox.values_mut() {
        if matches!(current.change, Change::Notes(_)) {
            current.error = error.into();
        }
    }
}

pub fn apply_known_restrictions(library: &mut Library) -> bool {
    let Some(error) = library
        .outbox
        .values()
        .find(|p| notes_permission_denied(p))
        .map(|p| p.error.clone())
    else {
        return false;
    };
    let mut changed = false;
    for pending in library.outbox.values_mut() {
        if matches!(pending.change, Change::Notes(_)) && pending.error != error {
            pending.error = error.clone();
            changed = true;
        }
    }
    changed
}

pub fn retry(library: &mut Library, unavailable: bool) {
    for pending in library
        .outbox
        .values_mut()
        .filter(|p| p.unavailable() == unavailable)
    {
        pending.error.clear();
        pending.retry_at = 0;
    }
}

pub fn confirm(library: &mut Library, pending: &Pending) {
    if library
        .outbox
        .get(&pending.key())
        .is_some_and(|p| p.change == pending.change)
    {
        library.outbox.remove(&pending.key());
    }
}
pub fn reconcile(library: &mut Library, owned: &[Item], wanted: &[Item]) {
    if library.schema_version == 0 {
        fn migrate(library: &mut Library, item: &Item, change: Change) {
            if !library
                .outbox
                .contains_key(&format!("{}:{}", item.key, change.kind()))
            {
                enqueue(library, item, change);
            }
        }
        let entries: Vec<_> = library.entries.values().cloned().collect();
        for entry in entries {
            if entry.owned && !owned.iter().any(|i| i.key == entry.item.key) {
                migrate(library, &entry.item, Change::Owned(true));
            }
            if !entry.owned && entry.item.owned && owned.iter().any(|i| i.key == entry.item.key) {
                migrate(library, &entry.item, Change::Owned(false));
            }
            if entry.wanted && !wanted.iter().any(|i| i.key == entry.item.key) {
                migrate(library, &entry.item, Change::Wanted(true));
            }
            if entry.rating > 0 {
                migrate(library, &entry.item, Change::Rating(entry.rating));
            }
            if !entry.notes.is_empty() {
                migrate(library, &entry.item, Change::Notes(entry.notes));
            }
            if entry.read {
                migrate(
                    library,
                    &entry.item,
                    Change::Read {
                        read: true,
                        date: entry.read_date,
                    },
                );
            }
        }
        let editions: Vec<_> = library
            .editions
            .values()
            .filter(|e| e.favorite && !wanted.iter().any(|i| i.key == e.item.key))
            .map(|e| e.item.clone())
            .collect();
        for item in editions {
            migrate(library, &item, Change::EditionFavorite(true));
        }
        library.schema_version = 1;
    }
    let owned_keys: std::collections::HashSet<_> = owned.iter().map(|i| &i.key).collect();
    let wanted_keys: std::collections::HashSet<_> = wanted.iter().map(|i| &i.key).collect();
    for item in owned.iter().chain(wanted) {
        library.ensure(item);
    }
    let protected: std::collections::HashSet<_> = library
        .outbox
        .values()
        .filter(|p| matches!(p.change, Change::EditionOwned(_)))
        .filter_map(|p| library.editions.get(&p.item.key))
        .flat_map(|e| e.volumes.iter().map(|i| i.key.clone()))
        .collect();
    for (key, entry) in &mut library.entries {
        if !library.outbox.contains_key(&format!("{key}:owned")) && !protected.contains(key) {
            entry.owned = owned_keys.contains(key);
        }
        if !library.outbox.contains_key(&format!("{key}:wanted"))
            && !library.outbox.contains_key(&format!("{key}:favorite"))
        {
            entry.wanted = wanted_keys.contains(key);
        }
    }
    for item in wanted.iter().filter(|i| i.key.starts_with("edicion")) {
        let saved = library.editions.entry(item.key.clone()).or_default();
        saved.item = item.clone();
        if !library
            .outbox
            .contains_key(&format!("{}:favorite", item.key))
            && !library.outbox.contains_key(&format!("{}:wanted", item.key))
        {
            saved.favorite = true;
        }
    }
    for (key, saved) in &mut library.editions {
        if !library.outbox.contains_key(&format!("{key}:favorite"))
            && !library.outbox.contains_key(&format!("{key}:wanted"))
        {
            saved.favorite = wanted_keys.contains(key);
        }
    }
}
pub fn pages(
    mut fetch: impl FnMut(u32) -> Result<crate::api::Page, String>,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Vec<Item>, String> {
    let mut result = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut page = 1;
    for _ in 0..1250 {
        if cancelled() {
            return Err("Actualización detenida; se conserva la biblioteca anterior".into());
        }
        let data = fetch(page)?;
        if data.items.is_empty() {
            return Ok(result);
        }
        let previous = seen.len();
        result.extend(
            data.items
                .into_iter()
                .filter(|i| seen.insert(i.key.clone())),
        );
        if seen.len() == previous {
            return Err("Whakoom repitió una página; se conserva la biblioteca anterior".into());
        }
        match data.next {
            None => return Ok(result),
            Some(next) if next > page => page = next,
            _ => return Err("La paginación no avanzó".into()),
        }
    }
    Err("Se alcanzó el límite de páginas".into())
}

pub fn missing_volumes<'a>(
    volumes: &'a [Item],
    owned: &std::collections::HashSet<String>,
    desired: bool,
) -> Vec<&'a Item> {
    volumes
        .iter()
        .filter(|item| owned.contains(&item.key) != desired)
        .collect()
}

/// Bounded, resumable work: reread the account after each batch, including no-op writes.
pub fn resume_edition(
    volumes: &[Item],
    desired: bool,
    batch: usize,
    snapshot: impl FnMut() -> Result<std::collections::HashSet<String>, String>,
    mut write: impl FnMut(&Item) -> Result<(), String>,
) -> Result<(), String> {
    resume_edition_batch(volumes, desired, batch, snapshot, |items| {
        for item in items {
            write(item)?;
        }
        Ok(())
    })
}
pub fn resume_edition_batch(
    volumes: &[Item],
    desired: bool,
    batch: usize,
    mut snapshot: impl FnMut() -> Result<std::collections::HashSet<String>, String>,
    mut write_batch: impl FnMut(&[&Item]) -> Result<(), String>,
) -> Result<(), String> {
    let before = snapshot()?;
    let missing = missing_volumes(volumes, &before, desired);
    if missing.is_empty() {
        return Ok(());
    }
    let write_result = write_batch(&missing[..missing.len().min(batch)]);
    let after = snapshot()?;
    let remaining = missing_volumes(volumes, &after, desired).len();
    if remaining == 0 {
        Ok(())
    } else if let Err(error) = write_result {
        Err(format!(
            "{error}. Quedan {remaining} tomos pendientes; se reintentará"
        ))
    } else if remaining < missing.len() {
        Err(format!(
            "Sincronizando serie: {} de {} tomos confirmados; continúa automáticamente",
            volumes.len() - remaining,
            volumes.len()
        ))
    } else {
        Err(
            "Whakoom todavía no confirmó los tomos pendientes; se reintentará automáticamente"
                .into(),
        )
    }
}

#[cfg(test)]
mod review_tests {
    use super::*;
    #[test]
    fn denied_notes_are_local_and_do_not_block_other_changes_or_repeat_requests() {
        let mut library = Library::default();
        let first = Item {
            key: "comic1".into(),
            title: "Uno".into(),
            ..Default::default()
        };
        let second = Item {
            key: "comic2".into(),
            title: "Dos".into(),
            ..Default::default()
        };
        enqueue(&mut library, &first, Change::Notes("Nota uno".into()));
        enqueue(&mut library, &second, Change::Notes("Nota dos".into()));
        enqueue(&mut library, &second, Change::Owned(true));
        let pending = library.outbox["comic1:notes"].clone();
        let denied = "Whakoom no permite editar notas con los permisos actuales de esta cuenta. La nota se conserva en tu PC";
        library.outbox.get_mut("comic1:notes").unwrap().error = denied.into();
        assert!(apply_known_restrictions(&mut library));
        assert!(!apply_known_restrictions(&mut library));
        pause_unavailable_notes(&mut library, &pending, denied);
        assert!(library.outbox["comic1:notes"].unavailable());
        assert!(library.outbox["comic2:notes"].unavailable());
        assert!(!library.outbox["comic2:owned"].unavailable());
        retry(&mut library, false);
        assert!(library.outbox["comic1:notes"].unavailable());
        enqueue(&mut library, &second, Change::Notes("Nota editada".into()));
        assert!(library.outbox["comic2:notes"].unavailable());
        assert_eq!(
            library.outbox["comic2:notes"].change,
            Change::Notes("Nota editada".into())
        );
        let mut restored: Library =
            serde_json::from_slice(&serde_json::to_vec(&library).unwrap()).unwrap();
        assert!(restored.outbox["comic2:notes"].unavailable());
        retry(&mut restored, true);
        assert!(restored.outbox.values().all(|p| !p.unavailable()));
        assert_eq!(restored.outbox.len(), 3);
    }
    #[test]
    fn a_ficha_restriction_does_not_disable_notes_in_other_fichas_or_network_retries() {
        let mut library = Library::default();
        let first = Item {
            key: "edicion1".into(),
            ..Default::default()
        };
        let second = Item {
            key: "comic2".into(),
            ..Default::default()
        };
        enqueue(&mut library, &first, Change::Notes("Uno".into()));
        let error = "La cuenta no dispone del editor de notas para esta ficha";
        library.outbox.get_mut("edicion1:notes").unwrap().error = error.into();
        let pending = library.outbox["edicion1:notes"].clone();
        pause_unavailable_notes(&mut library, &pending, error);
        enqueue(&mut library, &second, Change::Notes("Dos".into()));
        assert!(!library.outbox["comic2:notes"].unavailable());
        library.outbox.get_mut("comic2:notes").unwrap().error = "HTTP 429".into();
        assert!(!library.outbox["comic2:notes"].unavailable());
        enqueue(&mut library, &first, Change::Notes("Editada".into()));
        assert!(library.outbox["edicion1:notes"].unavailable());
    }
    #[test]
    fn public_review_is_durable_and_later_rating_cannot_be_overwritten_by_its_retry() {
        let mut library = Library::default();
        let item = Item {
            key: "comicABC".into(),
            url: "https://www.whakoom.com/comics/ABC/demo".into(),
            ..Default::default()
        };
        enqueue(&mut library, &item, Change::Rating(2));
        enqueue(
            &mut library,
            &item,
            Change::Review(crate::reviews::Draft {
                body: "Me encantó".into(),
                rating: 4,
            }),
        );
        assert!(!library.outbox.contains_key("comicABC:rating"));
        enqueue(&mut library, &item, Change::Rating(5));
        let restored: Library =
            serde_json::from_slice(&serde_json::to_vec(&library).unwrap()).unwrap();
        let Change::Review(draft) = &restored.outbox["comicABC:review"].change else {
            panic!("Missing draft");
        };
        assert_eq!(draft.rating, 5);
        assert_eq!(draft.body, "Me encantó");
        restored.validate().unwrap();
    }
}

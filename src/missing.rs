//! Missing published volumes of the exact editions in the user's collection.
use crate::{
    api::{Api, Item},
    series,
    storage::Library,
    sync,
};
use std::collections::HashSet;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Filter {
    #[default]
    All,
    Owned,
    Missing,
}
impl Filter {
    pub const ALL: [Self; 3] = [Self::All, Self::Owned, Self::Missing];
    pub fn title(self) -> &'static str {
        match self {
            Self::All => "Todos",
            Self::Owned => "Tengo",
            Self::Missing => "Faltan",
        }
    }
    pub fn accepts(self, library: &Library, item: &Item) -> bool {
        match self {
            Self::All => true,
            Self::Owned => owned(library, item),
            Self::Missing => !owned(library, item),
        }
    }
}
pub fn owned(library: &Library, item: &Item) -> bool {
    library
        .entries
        .get(&item.key)
        .map_or(item.owned, |e| e.owned)
}
pub fn progress(library: &Library, volumes: &[Item]) -> Option<(usize, usize)> {
    let edition = edition_for(library, volumes)?;
    let saved = library.editions.get(&edition.key).filter(|e| e.complete)?;
    Some((
        saved.volumes.iter().filter(|v| owned(library, v)).count(),
        saved.volumes.len(),
    ))
}
#[derive(Clone)]
pub struct Candidate {
    pub representative: Item,
    pub edition: Option<Item>,
}
pub fn edition_for(library: &Library, volumes: &[Item]) -> Option<Item> {
    volumes
        .iter()
        .find_map(|v| {
            library
                .entries
                .get(&v.key)?
                .details
                .as_ref()?
                .edition
                .clone()
        })
        .or_else(|| {
            library
                .editions
                .values()
                .find(|e| {
                    e.volumes
                        .iter()
                        .any(|v| volumes.iter().any(|i| i.key == v.key))
                })
                .map(|e| e.item.clone())
        })
}
pub fn candidates(library: &Library) -> Vec<Candidate> {
    let mut result = std::collections::BTreeMap::new();
    for entry in library
        .entries
        .values()
        .filter(|e| e.owned && e.item.key.starts_with("comic"))
    {
        let edition = edition_for(library, std::slice::from_ref(&entry.item));
        let identity = edition
            .as_ref()
            .map_or_else(|| series::identity(&entry.item), |e| e.key.clone());
        result.entry(identity).or_insert_with(|| Candidate {
            representative: entry.item.clone(),
            edition,
        });
    }
    result.into_values().collect()
}
#[derive(Clone)]
pub struct Suggestion {
    pub edition: Item,
    pub next: Item,
    pub count: usize,
}
pub fn suggestions(library: &Library) -> Vec<Suggestion> {
    let mut result = vec![];
    for saved in library.editions.values().filter(|e| e.complete) {
        let mut volumes = saved.volumes.clone();
        volumes.sort_by(series::volume_order);
        let Some(last) = volumes.iter().rposition(|v| owned(library, v)) else {
            continue;
        };
        let missing: Vec<_> = volumes.iter().filter(|v| !owned(library, v)).collect();
        let next = volumes
            .iter()
            .skip(last + 1)
            .find(|v| !owned(library, v))
            .or_else(|| missing.first().copied());
        if let Some(next) = next {
            result.push(Suggestion {
                edition: saved.item.clone(),
                next: next.clone(),
                count: missing.len(),
            });
        }
    }
    result.sort_by_cached_key(|s| s.edition.title.to_lowercase());
    result
}
impl Api {
    pub fn missing_editions(
        &self,
        candidates: &[Candidate],
        mut cancelled: impl FnMut() -> bool,
        mut receive: impl FnMut(Item, Vec<Item>),
    ) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut errors = vec![];
        for candidate in candidates {
            if cancelled() {
                break;
            }
            let result = (|| {
                let edition = match &candidate.edition {
                    Some(i) => i.clone(),
                    None => self
                        .full_detail(&candidate.representative)?
                        .edition
                        .ok_or("No se pudo identificar la edición de una colección")?,
                };
                if !seen.insert(edition.key.clone()) {
                    return Ok(());
                }
                let volumes = sync::pages(|p| self.edition(&edition, p), &mut cancelled)?;
                if !volumes
                    .iter()
                    .any(|v| v.key == candidate.representative.key)
                {
                    return Err("La edición recibida no incluye el tomo de tu colección".into());
                }
                receive(edition, volumes);
                Ok(())
            })();
            if let Err(error) = result {
                errors.push(error);
            }
        }
        errors
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn collection_progress_requires_a_complete_edition_and_uses_local_ownership() {
        let mut l = Library::default();
        let e = Item {
            key: "edicion12".into(),
            ..Default::default()
        };
        let volumes = vec![volume(1), volume(2), volume(3)];
        l.ensure(&volumes[0]).owned = true;
        l.cache_edition(&e, &volumes, false);
        assert_eq!(progress(&l, &volumes[..1]), None);
        l.cache_edition(&e, &volumes, true);
        assert_eq!(progress(&l, &volumes[..1]), Some((1, 3)));
        l.ensure(&volumes[1]).owned = true;
        assert_eq!(progress(&l, &volumes[..1]), Some((2, 3)));
        l.ensure(&volumes[0]).owned = false;
        assert_eq!(progress(&l, &volumes[..1]), Some((1, 3)));
    }

    use super::*;
    #[test]
    fn known_editions_with_the_same_title_stay_separate() {
        let mut l = Library::default();
        for n in [1, 2] {
            let v = volume(n);
            l.ensure(&v).owned = true;
            l.cache_edition(
                &Item {
                    key: format!("edicion{n}"),
                    ..Default::default()
                },
                &[v],
                true,
            );
        }
        assert_eq!(candidates(&l).len(), 2);
    }
    fn volume(n: usize) -> Item {
        Item {
            key: format!("comic{n}"),
            title: "Ejemplo".into(),
            issue: format!("#{n}"),
            url: format!("https://www.whakoom.com/comics/{n}/ejemplo/{n}"),
            ..Default::default()
        }
    }
    #[test]
    fn next_missing_prefers_the_next_published_volume_and_keeps_earlier_gaps() {
        let mut l = Library::default();
        let e = Item {
            key: "edicion12".into(),
            title: "Ejemplo".into(),
            url: "https://www.whakoom.com/ediciones/12/ejemplo".into(),
            ..Default::default()
        };
        let volumes = vec![volume(1), volume(2), volume(3)];
        l.cache_edition(&e, &volumes, true);
        l.ensure(&volumes[1]).owned = true;
        let s = suggestions(&l);
        assert_eq!(s[0].next.key, "comic3");
        assert_eq!(s[0].count, 2);
        l.ensure(&volumes[2]).owned = true;
        assert_eq!(suggestions(&l)[0].next.key, "comic1");
        l.ensure(&volumes[0]).owned = true;
        assert!(suggestions(&l).is_empty());
    }
    #[test]
    fn partial_editions_and_uncollected_favorites_are_not_missing_collections() {
        let mut l = Library::default();
        let e = Item {
            key: "edicion12".into(),
            ..Default::default()
        };
        let volumes = vec![volume(1), volume(2)];
        l.cache_edition(&e, &volumes, true);
        assert!(suggestions(&l).is_empty());
        l.ensure(&volumes[0]).owned = true;
        l.editions.get_mut(&e.key).unwrap().complete = false;
        assert!(suggestions(&l).is_empty());
        assert!(Filter::Owned.accepts(&l, &volumes[0]));
        assert!(Filter::Missing.accepts(&l, &volumes[1]));
        assert_eq!(l.editions[&e.key].volumes.len(), 2);
    }
}

//! Local achievements: no requests, paid entitlements or global leaderboard.
use crate::{icons::Icon, storage::Library};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tier {
    Bronze,
    Silver,
    Gold,
    Prism,
}
impl Tier {
    pub fn title(self) -> &'static str {
        match self {
            Self::Bronze => "Bronce",
            Self::Silver => "Plata",
            Self::Gold => "Oro",
            Self::Prism => "Prisma",
        }
    }
}

#[derive(Clone)]
pub struct Badge {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub icon: Icon,
    pub tier: Tier,
    pub current: usize,
    pub target: usize,
}
impl Badge {
    pub fn unlocked(&self) -> bool {
        self.current >= self.target
    }
    pub fn progress(&self) -> f32 {
        (self.current as f32 / self.target.max(1) as f32).min(1.)
    }
}

/// Unlocks survive removal of comics and restarting the app. The initial library
/// is a silent baseline to avoid playing a burst of old achievements on startup.
/// New badge definitions are baselined too.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Progress {
    pub known: BTreeSet<String>,
    pub earned: BTreeMap<String, u64>,
}
impl Progress {
    pub fn update(&mut self, badges: &[Badge], now: u64) -> Vec<Badge> {
        let mut newly_earned = Vec::new();
        for badge in badges {
            let known = !self.known.insert(badge.id.into());
            if badge.unlocked() && !self.earned.contains_key(badge.id) {
                self.earned.insert(badge.id.into(), now);
                if known {
                    newly_earned.push(badge.clone());
                }
            }
        }
        newly_earned
    }
    pub fn validate(&self) -> Result<(), String> {
        let valid = |id: &String| {
            !id.is_empty()
                && id.len() <= 64
                && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-')
        };
        if self.known.len() > 128
            || self.earned.len() > 128
            || self.known.iter().any(|id| !valid(id))
            || self
                .earned
                .keys()
                .any(|id| !valid(id) || !self.known.contains(id))
        {
            return Err("Datos de insignias inválidos".into());
        }
        Ok(())
    }
}

pub fn all(library: &Library) -> Vec<Badge> {
    use Tier::*;
    let stats = library.stats();
    let owned = |key: &String| library.entries.get(key).is_some_and(|e| e.owned);
    let collections = library
        .editions
        .values()
        .filter(|e| e.volumes.iter().any(|v| owned(&v.key)))
        .count();
    let complete = library
        .editions
        .values()
        .filter(|e| e.complete && !e.volumes.is_empty() && e.volumes.iter().all(|v| owned(&v.key)))
        .count();
    let volumes = library
        .entries
        .values()
        .filter(|e| e.item.key.starts_with("comic"))
        .collect::<Vec<_>>();
    let rated = volumes.iter().filter(|e| e.rating > 0).count();
    let notes = volumes
        .iter()
        .filter(|e| !e.notes.trim().is_empty())
        .count();
    let organized = volumes
        .iter()
        .filter(|e| e.owned && (!e.tags.trim().is_empty() || !e.location.trim().is_empty()))
        .count();
    [
        (
            "read-10",
            "Primera lectura",
            "Leé 10 tomos",
            Icon::OpenBook,
            Bronze,
            stats.read,
            10,
        ),
        (
            "read-50",
            "Lector constante",
            "Leé 50 tomos",
            Icon::OpenBook,
            Silver,
            stats.read,
            50,
        ),
        (
            "read-100",
            "Centena",
            "Leé 100 tomos",
            Icon::Trophy,
            Gold,
            stats.read,
            100,
        ),
        (
            "owned-10",
            "Estantería inicial",
            "Añadí 10 tomos",
            Icon::Shelf,
            Bronze,
            stats.owned,
            10,
        ),
        (
            "owned-100",
            "Gran colección",
            "Añadí 100 tomos",
            Icon::Shelf,
            Gold,
            stats.owned,
            100,
        ),
        (
            "collections-5",
            "Coleccionista",
            "Seguí 5 series",
            Icon::Grid,
            Silver,
            collections,
            5,
        ),
        (
            "wanted-10",
            "En la mira",
            "Guardá 10 tomos deseados",
            Icon::Heart,
            Bronze,
            stats.wanted,
            10,
        ),
        (
            "rated-10",
            "Criterio propio",
            "Valorá 10 tomos",
            Icon::Star,
            Bronze,
            rated,
            10,
        ),
        (
            "read-250",
            "Maratón de historias",
            "Leé 250 tomos",
            Icon::OpenBook,
            Prism,
            stats.read,
            250,
        ),
        (
            "read-500",
            "Biblioteca vivida",
            "Leé 500 tomos",
            Icon::Trophy,
            Prism,
            stats.read,
            500,
        ),
        (
            "owned-250",
            "Estanterías sin fin",
            "Añadí 250 tomos",
            Icon::Shelf,
            Prism,
            stats.owned,
            250,
        ),
        (
            "owned-500",
            "Archivo legendario",
            "Añadí 500 tomos",
            Icon::Trophy,
            Prism,
            stats.owned,
            500,
        ),
        (
            "complete-1",
            "Primera serie completa",
            "Tené todos los tomos de una serie",
            Icon::Shield,
            Bronze,
            complete,
            1,
        ),
        (
            "complete-10",
            "Cerrando historias",
            "Completá 10 series",
            Icon::Shield,
            Gold,
            complete,
            10,
        ),
        (
            "notes-10",
            "Entre líneas",
            "Escribí notas en 10 tomos",
            Icon::Quill,
            Silver,
            notes,
            10,
        ),
        (
            "reread-10",
            "Volver a casa",
            "Registrá 10 relecturas con fecha",
            Icon::Refresh,
            Silver,
            stats.rereads,
            10,
        ),
        (
            "organized-25",
            "Todo en su lugar",
            "Organizá 25 tomos con etiquetas o ubicación",
            Icon::List,
            Silver,
            organized,
            25,
        ),
        (
            "rated-50",
            "Voz de lector",
            "Valorá 50 tomos",
            Icon::Star,
            Gold,
            rated,
            50,
        ),
    ]
    .into_iter()
    .map(
        |(id, title, description, icon, tier, current, target)| Badge {
            id,
            title,
            description,
            icon,
            tier,
            current,
            target,
        },
    )
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        api::Item,
        storage::{Entry, SavedEdition},
    };
    #[test]
    fn unlocks_are_once_only_persisted_and_retained_when_counts_drop() {
        let mut library = Library::default();
        let initial = all(&library);
        assert!(library.badges.update(&initial, 1).is_empty());
        for index in 0..10 {
            let item = Item {
                key: format!("comic-{index}"),
                ..Default::default()
            };
            library.entries.insert(
                item.key.clone(),
                Entry {
                    item,
                    read: true,
                    ..Default::default()
                },
            );
        }
        let current = all(&library);
        let earned = library.badges.update(&current, 2);
        assert_eq!(earned.len(), 1);
        assert_eq!(earned[0].id, "read-10");
        let mut restored: Library =
            serde_json::from_str(&serde_json::to_string(&library).unwrap()).unwrap();
        let current = all(&restored);
        assert!(restored.badges.update(&current, 3).is_empty());
        restored.entries.clear();
        assert!(
            restored
                .badges
                .update(&all(&Library::default()), 4)
                .is_empty()
        );
        assert_eq!(restored.badges.earned["read-10"], 2);
        restored.badges.validate().unwrap();
    }
    #[test]
    fn cached_editions_arent_owned_or_complete_and_ten_extra_badges_are_unique() {
        let mut library = Library::default();
        let item = Item {
            key: "comic-one".into(),
            ..Default::default()
        };
        library.editions.insert(
            "edicion1".into(),
            SavedEdition {
                volumes: vec![item.clone()],
                complete: true,
                ..Default::default()
            },
        );
        let badges = all(&library);
        assert_eq!(badges.len(), 18);
        assert_eq!(
            badges.iter().map(|b| b.id).collect::<BTreeSet<_>>().len(),
            18
        );
        assert_eq!(
            badges
                .iter()
                .find(|b| b.id == "complete-1")
                .unwrap()
                .current,
            0
        );
        library.entries.insert(
            item.key.clone(),
            Entry {
                item,
                owned: true,
                ..Default::default()
            },
        );
        assert!(
            all(&library)
                .iter()
                .find(|b| b.id == "complete-1")
                .unwrap()
                .unlocked()
        );
        let mut progress = Progress::default();
        assert!(progress.update(&all(&library), 5).is_empty());
        assert!(progress.earned.contains_key("complete-1"));
        let mut corrupted = Progress::default();
        corrupted.known.insert("../other-account".into());
        assert!(corrupted.validate().is_err());
    }
}

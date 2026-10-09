use crate::storage::Library;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Badge {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub icon: &'static str,
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

/// Badges are deliberately derived from the local library. This keeps them
/// available offline and avoids pretending that Whakoom exposes a global
/// leaderboard endpoint for third-party clients.
pub fn all(library: &Library) -> Vec<Badge> {
    let stats = library.stats();
    let collections = library
        .editions
        .values()
        .filter(|edition| edition.complete && !edition.volumes.is_empty())
        .count();
    let rated = library
        .entries
        .values()
        .filter(|entry| entry.rating > 0)
        .count();
    [
        (
            "read-10",
            "Primera lectura",
            "Leé 10 tomos",
            "📖",
            stats.read,
            10,
        ),
        (
            "read-50",
            "Lector constante",
            "Leé 50 tomos",
            "📚",
            stats.read,
            50,
        ),
        (
            "read-100",
            "Centena",
            "Leé 100 tomos",
            "🏆",
            stats.read,
            100,
        ),
        (
            "owned-10",
            "Estantería inicial",
            "Añadí 10 tomos",
            "📦",
            stats.owned,
            10,
        ),
        (
            "owned-100",
            "Gran colección",
            "Añadí 100 tomos",
            "🗃️",
            stats.owned,
            100,
        ),
        (
            "collections-5",
            "Coleccionista",
            "Completá o seguí 5 series",
            "🧩",
            collections,
            5,
        ),
        (
            "wanted-10",
            "En la mira",
            "Guardá 10 tomos deseados",
            "💛",
            stats.wanted,
            10,
        ),
        (
            "rated-10",
            "Criterio propio",
            "Valorá 10 tomos",
            "⭐",
            rated,
            10,
        ),
    ]
    .into_iter()
    .map(|(id, title, description, icon, current, target)| Badge {
        id,
        title,
        description,
        icon,
        current,
        target,
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{api::Item, storage::Entry};

    #[test]
    fn badges_are_derived_from_local_progress() {
        let mut library = Library::default();
        for index in 0..10 {
            let entry = Entry {
                item: Item {
                    key: format!("comic-{index}"),
                    title: format!("Comic {index}"),
                    ..Default::default()
                },
                read: true,
                ..Default::default()
            };
            library.entries.insert(entry.item.key.clone(), entry);
        }
        let first = all(&library)
            .into_iter()
            .find(|badge| badge.id == "read-10")
            .unwrap();
        assert!(first.unlocked());
        assert_eq!(first.progress(), 1.);
    }
}

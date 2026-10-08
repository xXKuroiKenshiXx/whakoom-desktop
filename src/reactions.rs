use crate::{discussion::Review, storage};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub type Reactions = BTreeMap<String, BTreeMap<String, Reaction>>;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Reaction {
    pub value: i8,
    pub review: Review,
}
pub fn key(review: &Review) -> String {
    if review.id.is_empty() {
        storage::key(&format!(
            "{}:{}:{}",
            review.author, review.date, review.body
        ))
    } else {
        storage::key(&format!("{}:{}", review.id, review.author))
    }
}
pub fn value(reactions: &Reactions, title: &str, review: &Review) -> i8 {
    reactions
        .get(title)
        .and_then(|r| r.get(&key(review)))
        .map_or(0, |r| r.value)
}
pub fn toggle(reactions: &mut Reactions, title: &str, review: &Review, vote: i8) {
    if ![-1, 1].contains(&vote) {
        return;
    }
    let reviews = reactions.entry(title.into()).or_default();
    let id = key(review);
    if reviews.get(&id).is_some_and(|r| r.value == vote) {
        reviews.remove(&id);
    } else {
        reviews.insert(
            id,
            Reaction {
                value: vote,
                review: review.clone(),
            },
        );
    }
    if reviews.is_empty() {
        reactions.remove(title);
    }
}
pub fn ordered(reactions: &Reactions, title: &str, current: &[Review]) -> Vec<Review> {
    let mut reviews = current.to_vec();
    let mut seen: std::collections::HashSet<_> = current.iter().map(key).collect();
    if let Some(saved) = reactions.get(title) {
        for (id, reaction) in saved {
            if seen.insert(id.clone()) {
                reviews.push(reaction.review.clone());
            }
        }
    }
    reviews.sort_by_key(|review| -value(reactions, title, review));
    reviews
}
pub fn validate(reactions: &Reactions) -> Result<(), String> {
    if reactions.values().map(|r| r.len()).sum::<usize>() > 100_000 {
        return Err("Demasiadas reacciones en el respaldo".into());
    }
    for (title, reviews) in reactions {
        if !(title.starts_with("comic")
            || title
                .strip_prefix("edicion")
                .is_some_and(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())))
        {
            return Err("Ficha de reacción inválida".into());
        }
        for (id, reaction) in reviews {
            if ![-1, 1].contains(&reaction.value)
                || *id != key(&reaction.review)
                || !reaction.review.rating.is_finite()
                || !(0.0..=5.0).contains(&reaction.review.rating)
            {
                return Err("Reacción inválida en el respaldo".into());
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn review(id: &str) -> Review {
        Review {
            id: id.into(),
            author: "lector".into(),
            body: id.into(),
            ..Default::default()
        }
    }
    #[test]
    fn reactions_persist_pin_unloaded_reviews_and_respect_title_and_account() {
        let (a, b, c) = (review("a"), review("b"), review("c"));
        let mut library = storage::Library {
            owner: "alice".into(),
            ..Default::default()
        };
        toggle(&mut library.reactions, "comicABC", &c, 1);
        toggle(&mut library.reactions, "comicABC", &a, -1);
        let saved: storage::Library =
            serde_json::from_slice(&serde_json::to_vec(&library).unwrap()).unwrap();
        let order = ordered(&saved.reactions, "comicABC", &[a.clone(), b.clone()]);
        assert_eq!(
            order.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
            ["c", "b", "a"]
        );
        assert_eq!(ordered(&saved.reactions, "edicion1", &[b]).len(), 1);
        assert_eq!(
            value(&storage::Library::default().reactions, "comicABC", &c),
            0
        );
        toggle(&mut library.reactions, "comicABC", &c, 1);
        assert_eq!(value(&library.reactions, "comicABC", &c), 0);
        validate(&library.reactions).unwrap();
    }
}

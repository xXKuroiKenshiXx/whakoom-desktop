use crate::api::Item;
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Clone, Debug)]
pub struct Series {
    pub key: String,
    pub title: String,
    pub publisher: String,
    pub volumes: Vec<Item>,
}

// The comic URL carries the edition slug. Include the publisher so editions
// from different publishers stay separate. Single-volume titles keep their ID.
pub fn identity(item: &Item) -> String {
    let Ok(url) = url::Url::parse(&item.url) else {
        return item.key.clone();
    };
    let parts: Vec<_> = url.path_segments().into_iter().flatten().collect();
    if parts.first() == Some(&"comics")
        && parts.len() >= 3
        && (!item.issue.is_empty() || parts.get(3).is_some_and(|n| n.parse::<f64>().is_ok()))
    {
        format!(
            "edition:{}:{}",
            parts[2].to_lowercase(),
            item.publisher.trim().to_lowercase()
        )
    } else {
        item.key.clone()
    }
}

fn number(item: &Item) -> Option<f64> {
    let issue = item.issue.trim().trim_start_matches('#');
    let numeric: String = issue
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == '.')
        .collect();
    numeric.parse().ok().or_else(|| {
        url::Url::parse(&item.url)
            .ok()?
            .path_segments()?
            .nth(3)?
            .parse()
            .ok()
    })
}

pub fn volume_order(a: &Item, b: &Item) -> Ordering {
    match (number(a), number(b)) {
        (Some(a), Some(b)) => a.total_cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
    .then_with(|| a.issue.cmp(&b.issue))
    .then_with(|| a.key.cmp(&b.key))
}

pub fn group(items: &[Item]) -> Vec<Series> {
    let mut groups: BTreeMap<String, Series> = BTreeMap::new();
    for item in items {
        let key = identity(item);
        let group = groups.entry(key.clone()).or_insert_with(|| Series {
            key,
            title: item.title.clone(),
            publisher: item.publisher.clone(),
            volumes: vec![],
        });
        group.volumes.push(item.clone());
    }
    let mut groups: Vec<_> = groups.into_values().collect();
    for g in &mut groups {
        g.volumes.sort_by(volume_order);
        g.title = g.volumes[0].title.clone();
    }
    groups.sort_by_cached_key(|g| (g.title.to_lowercase(), g.publisher.to_lowercase()));
    groups
}

pub fn ease_out(t: f32) -> f32 {
    1. - (1. - t.clamp(0., 1.)).powi(3)
}

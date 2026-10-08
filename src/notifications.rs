use crate::social::Activity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Inbox {
    pub activities: Vec<Activity>,
    pub known: BTreeSet<String>,
    pub unread: BTreeSet<String>,
    pub initialized: bool,
}
impl Inbox {
    pub fn update(&mut self, activities: Vec<Activity>) {
        for activity in &activities {
            let key = format!("{}:{}", activity.user, activity.id);
            if self.initialized && !self.known.contains(&key) {
                self.unread.insert(key.clone());
            }
            self.known.insert(key);
        }
        self.initialized = true;
        self.activities = activities.into_iter().take(60).collect();
        // Retain only entries still visible in the current server feed.
        let visible: BTreeSet<_> = self
            .activities
            .iter()
            .map(|a| format!("{}:{}", a.user, a.id))
            .collect();
        self.unread.retain(|id| visible.contains(id));
        if self.known.len() > 500 {
            self.known = visible;
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn entry(id: &str) -> Activity {
        Activity {
            id: id.into(),
            user: "amiga".into(),
            ..Default::default()
        }
    }
    #[test]
    fn first_poll_is_a_baseline_and_subsequent_new_activity_is_unread() {
        let mut inbox = Inbox::default();
        inbox.update(vec![entry("1")]);
        assert!(inbox.unread.is_empty());
        inbox.update(vec![entry("2"), entry("1")]);
        assert_eq!(inbox.unread.len(), 1);
        inbox.unread.clear();
        inbox.update(vec![entry("2")]);
        assert!(inbox.unread.is_empty());
    }
}

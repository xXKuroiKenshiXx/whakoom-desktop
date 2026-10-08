use std::cell::Cell;
use whakoom_desktop::{
    api::{Item, Page},
    catalog,
    storage::Library,
};

fn volume(id: &str, number: u32) -> Item {
    Item {
        key: format!("comic{id}"),
        title: "Serie".into(),
        issue: format!("#{number}"),
        url: format!("https://www.whakoom.com/comics/{id}/serie/{number}"),
        ..Default::default()
    }
}
fn edition() -> Item {
    Item {
        key: "edicion123".into(),
        title: "Serie".into(),
        url: "https://www.whakoom.com/ediciones/123/serie".into(),
        publisher: "Editorial".into(),
        ..Default::default()
    }
}
#[test]
fn full_series_follows_pages_deduplicates_and_keeps_personal_edits() {
    let mut requested = vec![];
    let volumes = catalog::all_volumes(
        |page| {
            requested.push(page);
            Ok(match page {
                1 => Page {
                    items: vec![volume("a", 10), volume("b", 2)],
                    next: Some(2),
                },
                2 => Page {
                    items: vec![volume("b", 2), volume("c", 1)],
                    next: Some(3),
                },
                3 => Page::default(),
                _ => panic!("Página inesperada"),
            })
        },
        || false,
    )
    .unwrap();
    assert_eq!(requested, vec![1, 2, 3]);
    assert_eq!(
        volumes.iter().map(|i| i.issue.as_str()).collect::<Vec<_>>(),
        vec!["#1", "#2", "#10"]
    );
    let mut library = Library::default();
    let entry = library.ensure(&volume("b", 2));
    entry.owned = true;
    entry.read = true;
    entry.notes = "Dedicatoria".into();
    entry.rating = 5;
    library.favorite_edition(&edition());
    assert_eq!(
        library.add_complete_edition(&edition(), &volumes).unwrap(),
        2
    );
    assert_eq!(
        library.add_complete_edition(&edition(), &volumes).unwrap(),
        0
    );
    assert_eq!(library.stats().owned, 3);
    assert_eq!(library.stats().publishers["Editorial"], 3);
    assert_eq!(library.entries["comicb"].notes, "Dedicatoria");
    assert!(library.entries["comicb"].read);
    assert_eq!(library.entries["comicb"].rating, 5);
    assert!(library.editions["edicion123"].favorite);
    let (year, month, day) = whakoom_desktop::calendar::today();
    assert_eq!(
        library.entries["comica"].purchase_date,
        format!("{year:04}-{month:02}-{day:02}")
    );
    assert!(
        library.entries["comicb"].purchase_date.is_empty(),
        "An already owned, undated comic must not become a purchase today"
    );
}
#[test]
fn failed_repeated_or_cancelled_pages_never_produce_a_partial_series() {
    let mut requested = vec![];
    let result = catalog::all_volumes(
        |page| {
            requested.push(page);
            if page == 2 {
                Err("Sin conexión".into())
            } else {
                Ok(Page {
                    items: vec![volume("a", 1)],
                    next: Some(2),
                })
            }
        },
        || false,
    );
    assert!(result.is_err());
    assert_eq!(requested, vec![1, 2]);
    assert!(
        catalog::all_volumes(
            |page| Ok(Page {
                items: vec![volume("a", 1)],
                next: Some(page + 1)
            }),
            || false
        )
        .is_err()
    );
    let cancelled = Cell::new(false);
    assert!(
        catalog::all_volumes(
            |_| {
                cancelled.set(true);
                Ok(Page {
                    items: vec![volume("a", 1)],
                    next: None,
                })
            },
            || cancelled.get()
        )
        .is_err()
    );
}
#[test]
fn favorites_and_complete_series_survive_backup_and_old_library_still_loads() {
    let mut library: Library = serde_json::from_str(r#"{"owner":"local","entries":{}}"#).unwrap();
    assert!(library.editions.is_empty());
    library.favorite_edition(&edition());
    library
        .add_complete_edition(&edition(), &[volume("a", 1)])
        .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("backup.json");
    std::fs::write(&path, serde_json::to_vec(&library).unwrap()).unwrap();
    let mut restored = Library::import(&path, "another-reader").unwrap();
    assert_eq!(restored.owner, "another-reader");
    assert!(restored.editions["edicion123"].favorite);
    assert!(restored.editions["edicion123"].complete);
    assert!(restored.entries["comica"].owned);
    assert!(!restored.favorite_edition(&edition()));
    assert!(restored.entries["comica"].owned);
}
#[test]
fn invalid_edition_import_does_not_touch_the_library() {
    let mut library = Library::default();
    library.ensure(&volume("a", 1)).notes = "Guardar".into();
    let before = serde_json::to_vec(&library).unwrap();
    let mut invalid = volume("b", 2);
    invalid.url = "https://another.example/comics/b/serie/2".into();
    assert!(
        library
            .add_complete_edition(&edition(), &[volume("a", 1), invalid])
            .is_err()
    );
    assert_eq!(serde_json::to_vec(&library).unwrap(), before);
    assert!(library.add_complete_edition(&edition(), &[]).is_err());
    assert_eq!(serde_json::to_vec(&library).unwrap(), before);
}

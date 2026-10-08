use whakoom_desktop::{
    api::{Item, Page},
    storage::Library,
    sync::{self, Change},
};

fn item(id: &str) -> Item {
    Item {
        key: format!("comic{id}"),
        url: format!("https://www.whakoom.com/comics/{id}/test"),
        title: "Test".into(),
        ..Default::default()
    }
}
#[test]
fn edition_batches_resume_after_restart_and_never_acknowledge_partial_results() {
    use std::{cell::RefCell, collections::HashSet};
    let volumes: Vec<_> = (0..15).map(|n| item(&n.to_string())).collect();
    let online = RefCell::new(HashSet::new());
    let snapshot = || Ok(online.borrow().clone());
    let write = |item: &Item| {
        online.borrow_mut().insert(item.key.clone());
        Ok(())
    };
    let error = sync::resume_edition(&volumes, true, 12, snapshot, write).unwrap_err();
    assert!(error.starts_with("Sincronizando serie: 12 de 15"));
    assert_eq!(online.borrow().len(), 12);
    let mut writes = 0;
    sync::resume_edition(&volumes, true, 12, snapshot, |item| {
        writes += 1;
        write(item)
    })
    .unwrap();
    assert_eq!(writes, 3);
    sync::resume_edition(&volumes, true, 12, snapshot, |_| {
        panic!("Confirmed comics must not be sent again")
    })
    .unwrap();
    let error = sync::resume_edition(&volumes, false, 12, snapshot, |item| {
        online.borrow_mut().remove(&item.key);
        Ok(())
    })
    .unwrap_err();
    assert!(error.starts_with("Sincronizando serie: 12 de 15"));
    sync::resume_edition(&volumes, false, 12, snapshot, |item| {
        online.borrow_mut().remove(&item.key);
        Ok(())
    })
    .unwrap();
    assert!(online.borrow().is_empty());
}
#[test]
fn edition_retries_verify_server_state_even_when_individual_writes_report_success() {
    let volumes = [item("a"), item("b")];
    let result = sync::resume_edition(&volumes, true, 12, || Ok(Default::default()), |_| Ok(()));
    assert!(result.unwrap_err().contains("todavía no confirmó"));
    let result = sync::resume_edition(
        &volumes,
        true,
        12,
        || Err("sin red".into()),
        |_| panic!("Never write without a complete snapshot"),
    );
    assert_eq!(result.unwrap_err(), "sin red");
}
#[test]
fn partially_failed_batch_is_rechecked_and_kept_pending() {
    use std::{cell::RefCell, collections::HashSet};
    let volumes = [item("a"), item("b"), item("c")];
    let online = RefCell::new(HashSet::new());
    let error = sync::resume_edition_batch(
        &volumes,
        true,
        12,
        || Ok(online.borrow().clone()),
        |items| {
            online.borrow_mut().insert(items[0].key.clone());
            Err("HTTP 429".into())
        },
    )
    .unwrap_err();
    assert!(error.contains("429"));
    assert!(error.contains("2 tomos pendientes"));
    assert_eq!(online.borrow().len(), 1);
}
#[test]
fn latest_intent_survives_confirmation_of_an_inflight_request() {
    let mut library = Library::default();
    let comic = item("a");
    sync::enqueue(&mut library, &comic, Change::Owned(true));
    let sent = library.outbox.values().next().unwrap().clone();
    sync::enqueue(&mut library, &comic, Change::Owned(false));
    sync::confirm(&mut library, &sent);
    assert_eq!(library.outbox.len(), 1);
    assert_eq!(library.outbox["comica:owned"].change, Change::Owned(false));
    let latest = library.outbox.values().next().unwrap().clone();
    sync::confirm(&mut library, &latest);
    assert!(library.outbox.is_empty());
}
#[test]
fn individual_edit_during_bulk_sync_cannot_be_undone_by_a_later_bulk_retry() {
    let mut library = Library::default();
    let edition = Item {
        key: "edicion1".into(),
        url: "/ediciones/1/test".into(),
        ..Default::default()
    };
    let volumes = [item("a"), item("b")];
    library.cache_edition(&edition, &volumes, true);
    sync::enqueue(&mut library, &edition, Change::EditionOwned(true));
    let sent = library.outbox.values().next().unwrap().clone();
    sync::enqueue(&mut library, &volumes[0], Change::Owned(false));
    sync::confirm(&mut library, &sent);
    assert_eq!(library.outbox.len(), 2);
    assert_eq!(library.outbox["comica:owned"].change, Change::Owned(false));
    assert_eq!(library.outbox["comicb:owned"].change, Change::Owned(true));
}
#[test]
fn remote_deletions_apply_but_do_not_destroy_pending_changes_or_notes() {
    let mut library = Library::default();
    let a = item("a");
    let b = item("b");
    library.ensure(&a).owned = true;
    library.ensure(&a).notes = "Personal".into();
    library.ensure(&b).owned = true;
    sync::enqueue(&mut library, &b, Change::Owned(true));
    sync::reconcile(&mut library, &[], &[]);
    assert!(!library.entries["comica"].owned);
    assert_eq!(library.entries["comica"].notes, "Personal");
    assert!(library.entries["comicb"].owned);
    sync::enqueue(&mut library, &b, Change::Owned(false));
    library.entries.get_mut("comicb").unwrap().owned = false;
    sync::reconcile(&mut library, &[b], &[]);
    assert!(!library.entries["comicb"].owned);
}
#[test]
fn bulk_intent_protects_all_cached_volumes_and_favorites() {
    let mut library = Library::default();
    let a = item("a");
    let edition = Item {
        key: "edicion42".into(),
        url: "https://www.whakoom.com/ediciones/42/test".into(),
        ..Default::default()
    };
    library
        .add_complete_edition(&edition, std::slice::from_ref(&a))
        .unwrap();
    library.favorite_edition(&edition);
    sync::enqueue(&mut library, &edition, Change::EditionOwned(true));
    sync::enqueue(&mut library, &edition, Change::EditionFavorite(true));
    sync::reconcile(&mut library, &[], &[]);
    assert!(library.entries["comica"].owned);
    assert!(library.editions["edicion42"].favorite);
}
#[test]
fn legacy_local_edits_migrate_once_without_losing_personal_data() {
    let mut library = Library {
        schema_version: 0,
        ..Default::default()
    };
    let a = item("a");
    let entry = library.ensure(&a);
    entry.owned = true;
    entry.rating = 4;
    entry.notes = "Conservar".into();
    sync::reconcile(&mut library, &[], &[]);
    assert!(library.entries["comica"].owned);
    assert_eq!(library.outbox.len(), 3);
    let sent = library.outbox["comica:rating"].clone();
    sync::confirm(&mut library, &sent);
    sync::reconcile(&mut library, &[], &[]);
    assert!(!library.outbox.contains_key("comica:rating"));
    assert_eq!(library.schema_version, 1);
}
#[test]
fn pending_errors_and_retry_times_survive_a_restart() {
    let mut library = Library {
        owner: "reader".into(),
        ..Default::default()
    };
    sync::enqueue(&mut library, &item("a"), Change::Rating(3));
    let p = library.outbox.get_mut("comica:rating").unwrap();
    p.error = "Sin conexión".into();
    p.attempts = 2;
    p.retry_at = 1234;
    let restored: Library = serde_json::from_slice(&serde_json::to_vec(&library).unwrap()).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.owner, "reader");
    let p = &restored.outbox["comica:rating"];
    assert_eq!((p.attempts, p.retry_at), (2, 1234));
    assert_eq!(p.error, "Sin conexión");
}
#[test]
fn incomplete_or_repeating_pagination_never_returns_a_partial_snapshot() {
    let result = sync::pages(
        |page| {
            if page == 1 {
                Ok(Page {
                    items: vec![item("a")],
                    next: Some(2),
                })
            } else {
                Err("Conexión perdida".into())
            }
        },
        || false,
    );
    assert_eq!(result.unwrap_err(), "Conexión perdida");
    assert!(
        sync::pages(
            |p| Ok(Page {
                items: vec![item("a")],
                next: Some(p + 1)
            }),
            || false
        )
        .unwrap_err()
        .contains("repitió")
    );
    let mut calls = 0;
    let result = sync::pages(
        |p| {
            calls += 1;
            Ok(Page {
                items: if p == 1 { vec![item("a")] } else { vec![] },
                next: Some(p + 1),
            })
        },
        || false,
    )
    .unwrap();
    assert_eq!(result.len(), 1);
    assert_eq!(calls, 2);
    assert!(sync::pages(|_| unreachable!(), || true).is_err());
}

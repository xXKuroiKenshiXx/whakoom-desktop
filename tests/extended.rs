use whakoom_desktop::{api::Item, storage::Library};

fn comic(id: &str) -> Item {
    Item {
        key: format!("comic{id}"),
        url: format!("https://www.whakoom.com/comics/{id}/demo/1"),
        title: "Demo".into(),
        ..Default::default()
    }
}

#[test]
fn reading_history_location_photos_and_queue_survive_backup() {
    let dir = tempfile::tempdir().unwrap();
    let image = dir.path().join("signature.png");
    image::RgbaImage::from_pixel(16, 16, image::Rgba([120, 30, 80, 255]))
        .save(&image)
        .unwrap();
    let (id, photo) = whakoom_desktop::photos::import(&image).unwrap();
    let mut library = Library {
        owner: "test".into(),
        ..Default::default()
    };
    let item = comic("abc");
    let entry = library.ensure(&item);
    entry.owned = true;
    entry.read = true;
    entry.read_date = "2026-10-08".into();
    entry.readings = vec!["2026-09-01".into(), "2026-10-08".into()];
    entry.location = "Estante 2".into();
    entry.condition = "Muy bueno".into();
    entry.photos.push(id.clone());
    library.attachments.insert(id, photo);
    library.reading_order.push(item.key.clone());
    library.recent.search("Demo");
    library.recent.visit(&item);
    library.validate().unwrap();
    let backup = dir.path().join("backup.json");
    std::fs::write(&backup, serde_json::to_vec(&library).unwrap()).unwrap();
    let restored = Library::import(&backup, "another-owner").unwrap();
    assert_eq!(restored.owner, "another-owner");
    assert_eq!(restored.entries[&item.key].location, "Estante 2");
    assert_eq!(restored.reading_order, vec![item.key]);
    assert_eq!(restored.stats().rereads, 1);
    assert_eq!(restored.stats().reading_months.len(), 2);
    assert_eq!(restored.attachments.len(), 1);
}

#[test]
fn corrupt_photo_references_and_invalid_history_fail_before_import() {
    let mut library = Library {
        owner: "test".into(),
        ..Default::default()
    };
    let item = comic("abc");
    library.ensure(&item).readings.push("2026-02-31".into());
    assert!(library.validate().is_err());
    library.ensure(&item).readings.clear();
    library.ensure(&item).photos.push("../../private".into());
    assert!(library.validate().is_err());
    library.ensure(&item).photos.clear();
    library.reading_order = vec![item.key.clone(), item.key];
    assert!(library.validate().is_err());
}

#[test]
fn recent_queries_are_bounded_deduplicated_and_private_to_the_library() {
    let mut library = Library::default();
    library.recent.search("  BATMAN ");
    library.recent.search("batman");
    assert_eq!(library.recent.queries, vec!["batman"]);
    for n in 0..40 {
        library.recent.search(&n.to_string());
        library.recent.visit(&comic(&format!("a{n}")));
    }
    assert_eq!(library.recent.queries.len(), 16);
    assert_eq!(library.recent.visited.len(), 24);
    assert!(Library::default().recent.visited.is_empty());
}

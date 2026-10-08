use whakoom_desktop::{api::Item, series, storage::Preferences};

fn volume(id: &str, title: &str, issue: &str, publisher: &str) -> Item {
    Item {
        key: format!("comic{id}"),
        title: title.into(),
        issue: format!("#{issue}"),
        publisher: publisher.into(),
        url: format!("https://www.whakoom.com/comics/{id}/fullmetal_alchemist/{issue}"),
        ..Default::default()
    }
}

#[test]
fn groups_owned_volumes_by_edition_and_publisher_in_numeric_order() {
    let items = vec![
        volume("a", "Fullmetal Alchemist", "10", "Ivrea"),
        volume("b", "Fullmetal Alchemist", "2", "Ivrea"),
        volume("c", "Fullmetal Alchemist", "1", "Ivrea"),
        volume("d", "Fullmetal Alchemist", "1", "Norma"),
    ];
    let groups = series::group(&items);
    assert_eq!(groups.len(), 2);
    let ivrea = groups.iter().find(|g| g.publisher == "Ivrea").unwrap();
    assert_eq!(
        ivrea
            .volumes
            .iter()
            .map(|i| i.issue.as_str())
            .collect::<Vec<_>>(),
        vec!["#1", "#2", "#10"]
    );
    assert_eq!(
        groups.iter().map(|g| g.volumes.len()).sum::<usize>(),
        items.len()
    );
}

#[test]
fn single_volumes_and_different_editions_do_not_merge_by_title() {
    let mut a = volume("a", "Fullmetal Alchemist", "1", "Ivrea");
    let mut b = volume("b", "Fullmetal Alchemist", "2", "Ivrea");
    b.url = "https://www.whakoom.com/comics/b/fullmetal_alchemist_kanzenban/2".into();
    let mut c = a.clone();
    c.key = "comicC".into();
    c.issue.clear();
    c.url = "https://www.whakoom.com/comics/C/tomo_unico".into();
    a.issue.clear();
    a.url = "https://www.whakoom.com/comics/a/tomo_unico".into();
    assert_eq!(series::group(&[a, b, c]).len(), 3);
}

#[test]
fn grouping_keeps_case_sensitive_comic_ids_and_fractional_issues() {
    let items = vec![
        volume("AbC", "Serie", "2.5", "Ivrea"),
        volume("abc", "Serie", "2", "Ivrea"),
        volume("x", "Serie", "0", "Ivrea"),
    ];
    let groups = series::group(&items);
    assert_eq!(
        groups[0]
            .volumes
            .iter()
            .map(|i| i.key.as_str())
            .collect::<Vec<_>>(),
        vec!["comicx", "comicabc", "comicAbC"]
    );
}

#[test]
fn old_preferences_enable_new_views_and_animation_choice_is_persistent() {
    let old: Preferences = serde_json::from_str(r#"{"dark":false,"reading_goal":30}"#).unwrap();
    assert!(old.animations && old.series_view);
    assert!(!old.dark);
    let prefs: Preferences =
        serde_json::from_str(r#"{"animations":false,"series_view":false}"#).unwrap();
    let saved: Preferences = serde_json::from_str(&serde_json::to_string(&prefs).unwrap()).unwrap();
    assert!(!saved.animations && !saved.series_view);
}

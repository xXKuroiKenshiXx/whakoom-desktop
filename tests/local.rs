use std::fs;
use whakoom_desktop::{
    api::{self, Item},
    covers::{CoverClient, normalized_url},
    storage::{self, Library, Writer},
};

fn item() -> Item {
    Item {
        key: "comicAbC".into(),
        title: "A & B".into(),
        url: "https://www.whakoom.com/comics/AbC/a_b".into(),
        ..Default::default()
    }
}

#[test]
fn backup_preserves_personal_data_and_imports_into_selected_account() {
    let dir = tempfile::tempdir().unwrap();
    let mut lib = Library {
        owner: "reader-one".into(),
        ..Default::default()
    };
    let e = lib.ensure(&item());
    e.owned = true;
    e.read = true;
    e.cost = 12.5;
    e.notes = "Primera edición\nCon dedicatoria".into();
    e.tags = "Manga, manga, Favoritos".into();
    e.rating = 4;
    e.read_date = "2026-10-07".into();
    // Refreshing catalogue metadata must preserve all personal edits.
    let mut updated = item();
    updated.title = "Nuevo título".into();
    lib.ensure(&updated);
    let path = dir.path().join("backup.json");
    storage::atomic_write(&path, &serde_json::to_vec(&lib).unwrap()).unwrap();
    let restored = Library::import(&path, "reader-two").unwrap();
    assert_eq!(restored.owner, "reader-two");
    let e = &restored.entries["comicAbC"];
    assert_eq!(e.notes, "Primera edición\nCon dedicatoria");
    assert!(e.owned && e.read);
    assert_eq!(e.rating, 4);
    assert_eq!(e.read_date, "2026-10-07");
    assert_eq!(e.item.title, "Nuevo título");
    let stats = restored.stats();
    assert_eq!((stats.owned, stats.read, stats.pending), (1, 1, 0));
    assert_eq!(stats.spending, 12.5);
    assert_eq!(stats.tags["manga"], 1);
}

#[test]
fn invalid_backups_cannot_redirect_requests_or_insert_invalid_numbers() {
    let mut lib = Library::default();
    lib.ensure(&item());
    lib.entries.get_mut("comicAbC").unwrap().item.url = "https://other.example/comics/AbC".into();
    assert!(lib.validate().is_err());
    lib.entries.get_mut("comicAbC").unwrap().item = item();
    lib.entries.get_mut("comicAbC").unwrap().cost = f64::NAN;
    assert!(lib.validate().is_err());
    lib.entries.get_mut("comicAbC").unwrap().cost = 0.;
    lib.entries.get_mut("comicAbC").unwrap().rating = 9;
    assert!(lib.validate().is_err());
}

#[test]
fn csv_handles_quotes_newlines_and_spreadsheet_formulas() {
    let mut lib = Library::default();
    let e = lib.ensure(&item());
    e.item.title = "=1+1".into();
    e.notes = "Dice \"hola\"\nsegunda línea".into();
    let csv = lib.csv();
    assert!(csv.starts_with('\u{feff}'));
    assert!(csv.contains("\"'=1+1\""));
    assert!(csv.contains("\"Dice \"\"hola\"\"\nsegunda línea\""));
}

#[test]
fn writer_finishes_pending_writes_and_replaces_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.json");
    storage::atomic_write(&path, b"old").unwrap();
    {
        let writer = Writer::new();
        writer.file(path.clone(), b"updated".to_vec());
        writer.flush().unwrap();
        assert!(writer.errors.try_recv().is_err());
        assert_eq!(fs::read(&path).unwrap(), b"updated");
        writer.file(path.clone(), b"saved on close".to_vec());
    }
    assert_eq!(fs::read(&path).unwrap(), b"saved on close");
}

#[test]
fn cached_covers_load_offline_and_corrupt_cache_reports_error() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://i1.whakoom.com/small/cover.png";
    let path = dir.path().join(format!("{}.img", storage::key(url)));
    let img = image::RgbaImage::from_pixel(300, 600, image::Rgba([255, 100, 0, 255]));
    img.save_with_format(&path, image::ImageFormat::Png)
        .unwrap();
    let client = CoverClient::at(dir.path().into()).unwrap();
    let decoded = client.get(url, true).unwrap();
    assert_eq!(decoded.dimensions(), (225, 450));
    fs::write(&path, b"not an image").unwrap();
    assert!(client.get(url, true).unwrap_err().contains("no guardada"));
    assert!(
        client
            .get("https://i1.whakoom.com/missing.png", true)
            .is_err()
    );
}

#[test]
fn covers_accept_real_cdn_and_reject_external_or_insecure_urls() {
    assert_eq!(
        normalized_url("//i1.whakoom.com/a.jpg").unwrap(),
        "https://i1.whakoom.com/a.jpg"
    );
    assert_eq!(
        normalized_url("/images/a.png").unwrap(),
        "https://www.whakoom.com/images/a.png"
    );
    for url in [
        "http://i1.whakoom.com/a.jpg",
        "https://evil.example/a.jpg",
        "https://i1.whakoom.com.evil.example/a.jpg",
        "https://user:pass@i1.whakoom.com/a.jpg",
        "https://i1.whakoom.com:444/a.jpg",
    ] {
        assert!(normalized_url(url).is_err(), "{url}");
    }
}

#[test]
fn native_login_reads_current_csrf_field_and_requires_it() {
    let html = r#"<form><input type="hidden" name="__RequestVerificationToken" value="token&amp;value"><input id="username"><input id="userpassw"></form>"#;
    assert_eq!(api::login_token(html).unwrap(), "token&value");
    assert!(api::login_token("<form>Sin token</form>").is_err());
}

#[test]
fn monthly_reading_statistics_require_valid_dates() {
    assert_eq!(storage::reading_month("2024-02-29"), Some("2024-02"));
    for date in [
        "2026-02-29",
        "2026-13-01",
        "2026-10-00",
        "fecha",
        "éééé-10-07",
        "2026-04-31",
    ] {
        assert!(storage::reading_month(date).is_none(), "{date}");
    }
    let mut lib = Library::default();
    let e = lib.ensure(&item());
    e.read = true;
    e.read_date = "2026-10-07".into();
    assert_eq!(lib.stats().reading_months["2026-10"], 1);
}

#[test]
fn csv_neutralizes_formulas_hidden_after_whitespace() {
    let mut lib = Library::default();
    lib.ensure(&item()).notes = "\t  =HYPERLINK(\"https://example.invalid\")".into();
    assert!(lib.csv().contains("\"'\t  =HYPERLINK"));
}

#[cfg(unix)]
#[test]
fn atomic_write_replaces_symlinks_without_following_them_and_keeps_data_private() {
    use std::os::unix::{fs::PermissionsExt, fs::symlink};
    let dir = tempfile::tempdir().unwrap();
    let victim = dir.path().join("other-user-data");
    let destination = dir.path().join("library.json");
    fs::write(&victim, b"untouched").unwrap();
    symlink(&victim, &destination).unwrap();
    storage::atomic_write(&destination, b"new library").unwrap();
    assert_eq!(fs::read(&victim).unwrap(), b"untouched");
    assert_eq!(fs::read(&destination).unwrap(), b"new library");
    assert_eq!(
        fs::metadata(&destination).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

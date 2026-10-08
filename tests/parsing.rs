use serde_json::json;
use whakoom_desktop::api::*;

#[test]
fn edition_cards_read_ownership_from_the_volume_button_only() {
    let html = r#"<button class='rem-all'>Quitar serie</button><ul class='v2-cover-list'><li><a href='/comics/aa/ejemplo/1'><img src='https://i1.whakoom.com/small/a.jpg'><strong>Ejemplo</strong><span class='issue-number'>#1</span></a><button class='rem' data-item-id='aa'>Lo tengo</button></li><li class='get-it'><a href='/comics/bb/ejemplo/2'><img src='https://i1.whakoom.com/small/b.jpg'><strong>Ejemplo</strong><span class='issue-number'>#2</span></a><button class='add' data-item-id='bb'>Lo tengo</button></li></ul>"#;
    let items = parse_items(html);
    assert_eq!(items.len(), 2);
    assert!(items[0].owned);
    assert!(!items[1].owned);
}

#[test]
fn current_public_news_has_real_ids_and_decoded_text() {
    let items = parse_items(include_str!("fixtures/news-2026-10.html"));
    assert_eq!(items.len(), 10);
    assert_eq!(items[0].key, "comicnTWAk");
    assert_eq!(items[0].title, "Batman de dos mundos");
    assert_eq!(items[1].issue, "#8");
    assert!(items[0].cover.starts_with("https://i1.whakoom.com/small/"));
}

#[test]
fn current_public_detail_extracts_schema_fields() {
    let item = Item {
        key: "comicnTWAk".into(),
        url: format!("{BASE}/comics/nTWAk/batman_de_dos_mundos"),
        ..Default::default()
    };
    let d = parse_detail(include_str!("fixtures/comic-2026-10.html"), &item).unwrap();
    assert_eq!(d.item.title, "Batman de dos mundos");
    assert_eq!(d.numeric_id, Some(2465324));
    assert_eq!(d.publisher, "Panini Comics España");
    assert_eq!(d.language, "Español (España)");
    assert_eq!(d.date, "2026-09-19");
    assert_eq!(d.authors.len(), 6);
    assert_eq!(d.authors[2], "Javier Fernández");
    assert!(d.description.contains("Contiene"));
    assert!(!d.item.owned);
}

#[test]
fn authenticated_state_is_read_from_buttons_not_assumed() {
    let html = r#"<div class="pub-detail" data-item-type="comic" data-item-id="abc"><div class="comic-detail" data-item-id="42"><div class="b-info"><h1><span>Prueba</span></h1><a class="comicteca rem"></a><button class="wanted active"></button><div class="comic-read"><button class="not-readed"></button></div></div></div></div>"#;
    let d = parse_detail(html, &Item::default()).unwrap();
    assert!(d.item.owned && d.wanted && d.read);
    assert_eq!(d.wish_id, "abc");
}

#[test]
fn authentication_redirect_is_not_a_comic() {
    let login = r#"<form action="/login?ReturnUrl=/comics/abc"><h1>Inicia sesión</h1></form>"#;
    assert!(
        parse_detail(login, &Item::default())
            .unwrap_err()
            .contains("sesión")
    );
    assert!(parse_profile(login).is_none());
    assert_eq!(
        parse_profile(r#"<a id="user-avatar"><img alt="lector" src="/avatar.jpg"></a>"#),
        Some("lector".into())
    );
}

#[test]
fn supports_aspnet_old_and_current_envelopes() {
    assert_eq!(
        unwrap_response(json!({"d":{"Html":"<p>ok</p>"}})).unwrap()["Html"],
        "<p>ok</p>"
    );
    assert_eq!(unwrap_response(json!({"Html":"ok"})).unwrap()["Html"], "ok");
    assert_eq!(
        unwrap_response(json!({"d":"{\"Html\":\"ok\"}"})).unwrap()["Html"],
        "ok"
    );
    assert!(unwrap_response(json!({"d":"not json"})).is_err());
}

#[test]
fn rejects_external_urls_before_attaching_session() {
    for input in [
        "https://evil.example/comics/abc",
        "//evil.example/comics/abc",
        "http://www.whakoom.com/comics/abc",
        "https://www.whakoom.com.evil.example/comics/abc",
        "https://user:pass@www.whakoom.com/comics/abc",
        "https://www.whakoom.com:444/comics/abc",
    ] {
        assert!(safe_url(input).is_err(), "{input}");
    }
    assert_eq!(key_from_url("/comics/abc/title/1"), Some("comicabc".into()));
    assert_eq!(
        key_from_url("/ediciones/42/serie"),
        Some("edicion42".into())
    );
    assert!(key_from_url("/login").is_none());
}

#[test]
fn ignores_duplicate_actions_and_decodes_entities() {
    let html = r#"<li class="got-it"><a href="/comics/AbC/name"><img src="https://i1.whakoom.com/a.jpg"><strong>A &amp; B</strong><span class="issue-number">#2</span></a><a href="/comics/AbC/name">Lo tengo</a></li><a href="/comics/AbC/name"><strong>Duplicate</strong></a>"#;
    let items = parse_items(html);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].title, "A & B");
    assert!(items[0].owned);
}

#[test]
fn current_search_pairs_title_and_cover_from_sibling_links() {
    let html = r#"<div class="sresult sresult-t-2"><p class="img"><a href="/ediciones/77885/batman_2012-2024"><img src="https://i1.whakoom.com/thumb/cover.jpg"></a></p><p class="title"><a href="/ediciones/77885/batman_2012-2024">Batman (2012-2024)</a></p><p>Colección de <strong>152 números</strong>. Grapa</p></div><div class="sresult sresult-t-1"><p class="img"><a href="/comics/HP9Dq/coleccion/1"><img src="https://i1.whakoom.com/thumb/silencio.jpg"></a></p><p class="title"><a href="/comics/HP9Dq/coleccion/1">Batman: Silencio [Parte 1]</a></p></div>"#;
    let items = parse_items(html);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].title, "Batman (2012-2024)");
    assert_eq!(items[1].key, "comicHP9Dq");
    assert!(!items[1].cover.is_empty());
}

#[test]
fn current_collection_pairs_cover_full_title_issue_and_publisher() {
    let html = r#"<div class="item"><p class="image"><a href="/comics/89bXx/berserk_master_edition/1" class="comic"><img src="https://i1.whakoom.com/thumb/berserk.jpg"></a></p><p class="title"><a href="/comics/89bXx/berserk_master_edition/1">Berserk Master Edition <strong>#1</strong></a></p><p class="publisher">Panini Comics Espa&#241;a</p></div><div class="item"><p class="image"><a href="/comics/j9bID/gyo"><img src="https://i1.whakoom.com/thumb/gyo.jpg"></a></p><p class="title"><a href="/comics/j9bID/gyo">Gyo</a></p><p class="publisher">Planeta C&#243;mic</p></div>"#;
    let items = parse_items(html);
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].title, "Berserk Master Edition");
    assert_eq!(items[0].issue, "#1");
    assert_eq!(items[0].publisher, "Panini Comics España");
    assert!(!items[0].cover.is_empty());
    assert_eq!(items[1].title, "Gyo");
    assert!(!items[1].cover.is_empty());
}

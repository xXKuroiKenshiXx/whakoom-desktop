use whakoom_desktop::{
    api::{self, Item},
    social,
};
#[test]
fn account_avatar_and_following_profiles_are_taken_from_whakoom() {
    let identity = social::identity(r#"<a id="user-avatar"><img alt="reader" src="https://i1.whakoom.com/avatar/real.jpg"></a>"#).unwrap();
    assert_eq!(identity.username, "reader");
    assert!(identity.avatar.ends_with("real.jpg"));
    let friends = social::friends(
        r#"<ul class="users-list"><li><a class="avatar" href="/friend"><img src="https://i1.whakoom.com/avatar/friend.jpg"></a><a class="un">Amiga</a></li><li><a class="avatar" href="https://evil.example/stolen"><img></a></li></ul>"#,
    );
    assert_eq!(friends.len(), 1);
    assert_eq!(friends[0].name, "Amiga");
    assert_eq!(friends[0].username, "friend");
    assert!(social::user_path("../login").is_err());
    assert!(social::identity("<h1>Login</h1>").is_err());
}
#[test]
fn public_activity_has_real_volume_links_and_lazy_images() {
    let user = social::profile(r#"<div id="public-profile-h"><h1>Amiga</h1><a class="avatar"><img src="https://i1.whakoom.com/avatar/f.jpg"></a><div class="comic-count"><strong>42</strong></div></div><ul class="base-items"><li><span>Añadió a su colección</span><a class="activity" href="/friend/activity/123"></a><div class="comic-list"><a href="/comics/abc/una_serie/1"><img src="placeholder" data-src="https://i1.whakoom.com/small/cover.jpg"><span>#1</span></a></div></li></ul>"#, "friend").unwrap();
    assert_eq!(user.comics, "42");
    assert_eq!(user.activity.len(), 1);
    let activity = &user.activity[0];
    assert_eq!(activity.id, "123");
    assert_eq!(activity.comics[0].key, "comicabc");
    assert_eq!(activity.comics[0].issue, "#1");
    assert!(activity.comics[0].cover.ends_with("cover.jpg"));
}
#[test]
fn public_and_personal_ratings_are_independent_and_dates_are_validated() {
    let html = r#"<div class="b-info"><h1>Tomo</h1><span itemprop="ratingValue">4,6</span></div><div class="my-comic-review"><span class="stars" data-item-id="3"></span></div><div class="comic-read"><table class="read-list"><tr data-item-date="20261007"></tr></table></div>"#;
    let detail = api::parse_detail(html, &Item::default()).unwrap();
    assert_eq!(detail.item.community_rating, 4.6);
    assert_eq!(detail.personal_rating, 3);
    assert_eq!(detail.read_date, "2026-10-07");
    let invalid = api::parse_detail(
        &html.replace("4,6", "NaN").replace("20261007", "20260230"),
        &Item::default(),
    )
    .unwrap();
    assert_eq!(invalid.item.community_rating, 0.);
    assert!(invalid.read_date.is_empty());
}

use whakoom_desktop::{
    api::{self, Item},
    social,
};

#[test]
fn pro_labels_belong_to_the_person_not_the_upgrade_menu_or_bio() {
    let header = "<a class='mmn-unlimited' href='/upgrade'>Pro</a>";
    let profile = "<div id='public-profile-h'><h1>Lectora</h1><p class='bio'>Soy Pro</p></div>";
    assert!(
        !social::profile(&format!("{header}{profile}"), "lectora")
            .unwrap()
            .pro
    );
    let pro = profile.replace("<h1>", "<div class='pro-badge'><span>Pro</span></div><h1>");
    assert!(social::profile(&pro, "lectora").unwrap().pro);
    let people = social::friends(
        "<ul class='users-list'><li><a class='avatar' href='/lectora'><img></a><span class='pro-badge'>Pro</span></li><li><a class='avatar' href='/otro'><img></a><p class='bio'><span class='pro-badge'>Pro</span></p></li></ul>",
    );
    assert!(people[0].pro);
    assert!(!people[1].pro);
    assert!(
        !social::identity(&format!(
            "{header}<a id='user-avatar'><img alt='lectora'></a>"
        ))
        .unwrap()
        .pro
    );
    let old: social::User = serde_json::from_str("{\"username\":\"lectora\"}").unwrap();
    assert!(!old.pro);
    let restored: social::User =
        serde_json::from_str(&serde_json::to_string(&people[0]).unwrap()).unwrap();
    assert!(restored.pro);
}

#[test]
fn review_pro_is_scoped_to_author_and_survives_old_backups() {
    let markup = "<div class='review' data-item-id='1' itemprop='review'><a itemprop='author'>lectora</a><span class='pro-badge'>Pro</span><p itemprop='reviewBody'>Buena</p></div>";
    let parsed = whakoom_desktop::discussion::parse(markup);
    assert!(parsed.reviews[0].pro);
    let old = serde_json::to_value(&parsed.reviews[0]).unwrap();
    let mut old = old.as_object().unwrap().clone();
    old.remove("pro");
    let restored: whakoom_desktop::discussion::Review = serde_json::from_value(old.into()).unwrap();
    assert!(!restored.pro);
    let fake = markup
        .replace("<span class='pro-badge'>Pro</span>", "")
        .replace("Buena", "<span class='pro-badge'>Pro</span>");
    assert!(!whakoom_desktop::discussion::parse(&fake).reviews[0].pro);
}
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
#[test]
fn followers_use_their_own_route_and_survive_backup_without_replacing_following() {
    assert_eq!(social::Relation::Followers.path(), "followers");
    assert_eq!(social::Relation::Following.path(), "following");
    let mut library = whakoom_desktop::storage::Library {
        owner: "reader".into(),
        ..Default::default()
    };
    library.friends.push(social::User {
        username: "following".into(),
        ..Default::default()
    });
    library.followers.push(social::User {
        username: "follower".into(),
        ..Default::default()
    });
    let restored: whakoom_desktop::storage::Library =
        serde_json::from_str(&serde_json::to_string(&library).unwrap()).unwrap();
    assert_eq!(restored.friends[0].username, "following");
    assert_eq!(restored.followers[0].username, "follower");
    let mut old_data = serde_json::to_value(&library).unwrap();
    old_data.as_object_mut().unwrap().remove("followers");
    let old: whakoom_desktop::storage::Library = serde_json::from_value(old_data).unwrap();
    assert!(old.followers.is_empty());
}

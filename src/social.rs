use crate::api::{self, Item};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Relation {
    #[default]
    Following,
    Followers,
}
impl Relation {
    pub fn path(self) -> &'static str {
        match self {
            Self::Following => "following",
            Self::Followers => "followers",
        }
    }
    pub fn title(self) -> &'static str {
        match self {
            Self::Following => "Seguidos",
            Self::Followers => "Seguidores",
        }
    }
}

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct User {
    pub username: String,
    pub name: String,
    pub avatar: String,
    pub url: String,
    pub comics: String,
    pub followers: String,
    pub bio: String,
    pub activity: Vec<Activity>,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Activity {
    pub id: String,
    pub user: String,
    pub message: String,
    pub url: String,
    pub comics: Vec<Item>,
}
fn selector(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
fn text(e: ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn value(h: &Html, s: &str) -> String {
    h.select(&selector(s)).next().map(text).unwrap_or_default()
}
fn attr(h: &Html, s: &str, a: &str) -> String {
    h.select(&selector(s))
        .next()
        .and_then(|e| e.value().attr(a))
        .unwrap_or_default()
        .into()
}
pub fn user_path(name: &str) -> Result<String, String> {
    if name.is_empty()
        || name.len() > 80
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-".contains(c))
    {
        return Err("Nombre de usuario inválido".into());
    }
    Ok(format!("/{name}"))
}
pub fn identity(html: &str) -> Result<User, String> {
    let h = Html::parse_document(html);
    let username = attr(&h, "#user-avatar img", "alt");
    Ok(User {
        url: api::safe_url(&user_path(&username)?)?,
        name: username.clone(),
        username,
        avatar: attr(&h, "#user-avatar img", "src"),
        ..Default::default()
    })
}
pub fn profile(html: &str, username: &str) -> Result<User, String> {
    let h = Html::parse_document(html);
    let name = value(&h, "#public-profile-h h1");
    if name.is_empty() {
        return Err("No se pudo leer el perfil de Whakoom".into());
    }
    let mut result = User {
        username: username.into(),
        name,
        url: api::safe_url(&user_path(username)?)?,
        avatar: attr(&h, "#public-profile-h .avatar img", "src"),
        comics: value(&h, "#public-profile-h .comic-count strong"),
        followers: value(&h, "#public-profile-h .followers a"),
        bio: value(&h, "#public-profile-h .bio, #public-profile-h .about"),
        ..Default::default()
    };
    result.activity = activity(html, Some(username));
    Ok(result)
}
pub fn friends(html: &str) -> Vec<User> {
    let h = Html::parse_document(html);
    h.select(&selector("ul.users-list > li"))
        .filter_map(|entry| {
            let a = entry.select(&selector("a.avatar[href]")).next()?;
            let url = api::safe_url(a.value().attr("href")?).ok()?;
            let username = url::Url::parse(&url)
                .ok()?
                .path()
                .trim_matches('/')
                .to_owned();
            user_path(&username).ok()?;
            let name = entry
                .select(&selector(".user-name, .username, .name, a.username, a.un"))
                .next()
                .map(text)
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| username.clone());
            Some(User {
                name,
                username,
                url,
                avatar: a
                    .select(&selector("img"))
                    .next()
                    .and_then(|i| i.value().attr("src"))
                    .unwrap_or_default()
                    .into(),
                ..Default::default()
            })
        })
        .collect()
}

pub fn activity(html: &str, username: Option<&str>) -> Vec<Activity> {
    let h = Html::parse_document(html);
    let mut result = Vec::new();
    for entry in h.select(&selector("ul.base-items > li")) {
        let link = entry
            .select(&selector("a.activity, a.comments"))
            .next()
            .and_then(|e| e.value().attr("href"))
            .unwrap_or_default();
        let Ok(url) = api::safe_url(link) else {
            continue;
        };
        let username = username.map(str::to_owned).unwrap_or_else(|| {
            entry
                .select(&selector("span > a[href]"))
                .next()
                .map(text)
                .unwrap_or_default()
        });
        if user_path(&username).is_err() {
            continue;
        }
        let id = url.rsplit('/').next().unwrap_or_default().to_owned();
        let message = entry
            .children()
            .filter_map(ElementRef::wrap)
            .find(|e| e.value().name() == "span")
            .map(text)
            .unwrap_or_default();
        if message.is_empty() {
            continue;
        }
        let comics = entry
            .select(&selector(".comic-list a[href]"))
            .filter_map(|a| {
                let url = api::safe_url(a.value().attr("href")?).ok()?;
                let key = api::key_from_url(&url)?;
                let img = a.select(&selector("img")).next()?;
                let cover = img
                    .value()
                    .attr("data-src")
                    .filter(|s| !s.is_empty())
                    .or_else(|| img.value().attr("src"))
                    .unwrap_or_default()
                    .into();
                let title = img
                    .value()
                    .attr("alt")
                    .filter(|s| !s.is_empty())
                    .map(str::to_owned)
                    .unwrap_or_else(|| {
                        url::Url::parse(&url)
                            .ok()
                            .and_then(|u| {
                                u.path_segments()
                                    .and_then(|mut p| p.nth(2))
                                    .map(|s| s.replace('_', " "))
                            })
                            .unwrap_or_else(|| "Cómic".into())
                    });
                Some(Item {
                    key,
                    title,
                    url,
                    cover,
                    issue: a
                        .select(&selector("span"))
                        .next()
                        .map(text)
                        .unwrap_or_default(),
                    ..Default::default()
                })
            })
            .collect();
        result.push(Activity {
            id,
            user: username.clone(),
            message,
            url,
            comics,
        });
    }
    result
}

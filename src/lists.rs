use crate::api::{self, Api, Item};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ComicList {
    pub id: u64,
    pub url: String,
    pub title: String,
    pub description: String,
    pub creator: String,
    pub covers: Vec<String>,
    pub count: String,
    pub likes: String,
    pub liked: bool,
    pub comics: Vec<Item>,
    pub next: Option<u32>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ListPage {
    pub lists: Vec<ComicList>,
    pub next: Option<u32>,
}
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub enum Section {
    #[default]
    Discover,
    Popular,
    Recent,
    Mine,
    Favorites,
}
impl Section {
    pub const ALL: [Self; 5] = [
        Self::Discover,
        Self::Popular,
        Self::Recent,
        Self::Mine,
        Self::Favorites,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Discover => "Descubrir",
            Self::Popular => "Populares",
            Self::Recent => "Últimas listas",
            Self::Mine => "Mis listas",
            Self::Favorites => "Listas favoritas",
        }
    }
    pub fn path(self, owner: &str) -> Result<String, String> {
        Ok(match self {
            Self::Discover => "/lists".into(),
            Self::Popular => "/lists/most_popular/last_week".into(),
            Self::Recent => "/lists/last_updated/last_month".into(),
            Self::Mine => format!("{}/lists", crate::social::user_path(owner)?),
            Self::Favorites => format!("{}/listsliked", crate::social::user_path(owner)?),
        })
    }
}
#[derive(Clone, Default)]
pub struct Draft {
    pub title: String,
    pub description: String,
    pub private: bool,
    pub ranked: bool,
    pub kind: u8,
    pub comics: Vec<Item>,
}
fn select(s: &str) -> Selector {
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
fn value(doc: &Html, css: &str) -> String {
    doc.select(&select(css))
        .next()
        .map(text)
        .unwrap_or_default()
}
pub fn list_url(input: &str) -> Result<String, String> {
    let safe = api::safe_url(input)?;
    let mut parsed = url::Url::parse(&safe).map_err(|_| "Lista inválida")?;
    let parts: Vec<_> = parsed.path().trim_matches('/').split('/').collect();
    if parts.len() != 3
        || parts[1] != "lists"
        || crate::social::user_path(parts[0]).is_err()
        || parts[2]
            .rsplit('_')
            .next()
            .and_then(|v| v.parse::<u64>().ok())
            .is_none_or(|id| id == 0)
    {
        return Err("Enlace de lista inválido".into());
    }
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}
pub fn parse_page(html: &str, page: u32) -> ListPage {
    let doc = Html::parse_document(html);
    let mut seen = HashSet::new();
    let mut lists = Vec::new();
    for block in doc.select(&select(".list-item, .v2-list-item")) {
        let Some(link) = block
            .select(&select("h3 a[href], h2 a[href], .title a[href]"))
            .next()
        else {
            continue;
        };
        let Some(url) = link.value().attr("href").and_then(|u| list_url(u).ok()) else {
            continue;
        };
        if !seen.insert(url.clone()) {
            continue;
        }
        let field = |css: &str| {
            block
                .select(&select(css))
                .next()
                .map(text)
                .unwrap_or_default()
        };
        let id = url
            .rsplit('_')
            .next()
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();
        lists.push(ComicList {
            id,
            url,
            title: text(link),
            description: field(".desc"),
            creator: field(".user"),
            count: field(".ccount"),
            likes: field(".like"),
            covers: block
                .select(&select(".coverSample img, .covers img"))
                .filter_map(|img| {
                    img.value()
                        .attr("src")
                        .or_else(|| img.value().attr("data-original"))
                })
                .filter_map(|url| crate::covers::normalized_url(url).ok())
                .take(5)
                .collect(),
            ..Default::default()
        });
    }
    let next = doc
        .select(&select("a[href]"))
        .filter_map(|a| {
            let safe = api::safe_url(a.value().attr("href")?).ok()?;
            let url = url::Url::parse(&safe).ok()?;
            url.query_pairs().find(|(k, _)| k == "page")?.1.parse().ok()
        })
        .find(|p: &u32| *p > page && *p <= 1000)
        .or_else(|| {
            let container = doc
                .select(&select("#listcontainer[data-item-mode][data-item-period]"))
                .next()?;
            let actual = container
                .value()
                .attr("data-item-page")?
                .parse::<u32>()
                .ok()?;
            (actual < 1000
                && doc.select(&select("#loadmore")).next().is_some()
                && !lists.is_empty())
            .then_some(actual + 1)
        });
    ListPage { lists, next }
}
pub fn parse_detail(html: &str, url: &str) -> Result<ComicList, String> {
    let url = list_url(url)?;
    let doc = Html::parse_document(html);
    let root = doc
        .select(&select("#list[data-item-id]"))
        .next()
        .ok_or("No se pudo leer la lista")?;
    let id = root
        .value()
        .attr("data-item-id")
        .and_then(|s| s.parse::<u64>().ok())
        .filter(|id| *id > 0)
        .ok_or("ID de lista inválido")?;
    if url.rsplit('_').next().and_then(|s| s.parse::<u64>().ok()) != Some(id) {
        return Err("La ficha pertenece a otra lista".into());
    }
    let comics = api::parse_items(&format!(
        "<div>{}</div>",
        doc.select(&select("#issuelist"))
            .next()
            .map(|e| e.html())
            .unwrap_or_default()
    ));
    let next = doc
        .select(&select("#loadmoreissues[data-p-next]"))
        .next()
        .and_then(|e| e.value().attr("data-p-next"))
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|p| *p > 1);
    Ok(ComicList {
        id,
        url,
        title: value(&doc, "#list h1 > span"),
        description: doc
            .select(&select(".list-desc-text p"))
            .map(text)
            .collect::<Vec<_>>()
            .join("\n\n"),
        creator: value(&doc, ".createdby a"),
        count: value(&doc, "#list h1 small"),
        likes: value(&doc, ".p-like .text"),
        liked: doc.select(&select(".p-like.active")).next().is_some(),
        comics,
        next,
        ..Default::default()
    })
}
pub fn payload(draft: &Draft, ids: &[u64]) -> Result<Value, String> {
    if draft.title.trim().is_empty()
        || draft.title.chars().count() > 150
        || draft.description.chars().count() > 8000
        || ![0, 2, 3].contains(&draft.kind)
        || ids.len() > 1000
        || ids.contains(&0)
        || ids.len() != draft.comics.len()
        || ids.iter().collect::<HashSet<_>>().len() != ids.len()
    {
        return Err("Revisá el nombre, la descripción y los tomos de la lista".into());
    }
    Ok(
        json!({"id":"-1","tt":draft.title.trim(),"dsc":draft.description,"p":draft.private,"r":draft.ranked,"lt":draft.kind,"cl":ids.iter().map(u64::to_string).collect::<Vec<_>>()}),
    )
}
impl Api {
    pub fn lists(&self, section: Section, owner: &str, page: u32) -> Result<ListPage, String> {
        if !(1..=1000).contains(&page) {
            return Err("Página inválida".into());
        }
        let html = self.html(&section.path(owner)?)?;
        if page == 1 {
            return Ok(parse_page(&html, page));
        }
        let doc = Html::parse_document(&html);
        let container = doc
            .select(&select("#listcontainer[data-item-mode][data-item-period]"))
            .next()
            .ok_or("Esta sección no tiene más páginas")?;
        let mode = container
            .value()
            .attr("data-item-mode")
            .ok_or("Modo de lista ausente")?;
        let period = container
            .value()
            .attr("data-item-period")
            .ok_or("Período de lista ausente")?;
        let result = self.post(
            "/lists/listlists.aspx/NextPage",
            json!({"lm":mode,"lp":period,"p":page}),
        )?;
        if result.get("ExtraInfo").and_then(Value::as_str) != Some("1") {
            return Err("Whakoom no confirmó la página de listas".into());
        }
        let mut parsed = parse_page(
            result
                .get("Html")
                .and_then(Value::as_str)
                .ok_or("Página de listas incompleta")?,
            page,
        );
        parsed.next = (result.get("ControlID").and_then(Value::as_str) != Some("-1")
            && !parsed.lists.is_empty()
            && page < 1000)
            .then_some(page + 1);
        Ok(parsed)
    }
    pub fn comic_list(&self, url: &str) -> Result<ComicList, String> {
        let url = list_url(url)?;
        parse_detail(&self.html(&url)?, &url)
    }
    pub fn list_comics(&self, list: &ComicList, page: u32) -> Result<crate::api::Page, String> {
        if list.id == 0 || !(2..=1000).contains(&page) {
            return Err("Lista o página inválida".into());
        }
        let result = self.post(
            "/lists/listdetail.aspx/SeriesPage",
            json!({"id":list.id,"f":0,"p":page}),
        )?;
        let html = result
            .get("Html")
            .and_then(Value::as_str)
            .ok_or("Lista incompleta")?;
        Ok(crate::api::Page {
            items: api::parse_items(html),
            next: result
                .get("ExtraInfo")
                .and_then(Value::as_str)
                .and_then(|s| s.parse().ok())
                .filter(|p| *p > page),
        })
    }
    pub fn favorite_list(&self, list: &ComicList, desired: bool) -> Result<ComicList, String> {
        let safe = list_url(&list.url)?;
        if safe.rsplit('_').next().and_then(|s| s.parse::<u64>().ok()) != Some(list.id) {
            return Err("Identidad de lista incorrecta".into());
        }
        self.post(
            "/lists/listdetail.aspx/Like",
            json!({"id":list.id,"l":desired}),
        )?;
        let updated = self.comic_list(&list.url)?;
        if updated.liked != desired {
            return Err("Whakoom no confirmó el favorito de la lista".into());
        }
        Ok(updated)
    }
    pub fn create_list(&self, draft: &Draft) -> Result<ComicList, String> {
        let validation_ids: Vec<_> = (1..=draft.comics.len() as u64).collect();
        payload(draft, &validation_ids)?;
        self.html("/lists/new")?;
        let mut ids = Vec::new();
        for item in &draft.comics {
            if !item.key.starts_with("comic") {
                return Err("Elegí tomos individuales para la lista".into());
            }
            ids.push(
                self.full_detail(item)?
                    .numeric_id
                    .ok_or("Falta el identificador del tomo")?,
            );
        }
        let body = payload(draft, &ids)?;
        let result = self.post("/lists/newseries.aspx/Update", body)?;
        if result.get("ExtraInfo").and_then(Value::as_str) != Some("1") {
            return Err(result
                .get("Title")
                .and_then(Value::as_str)
                .unwrap_or("Whakoom no confirmó la creación de la lista")
                .into());
        }
        self.comic_list(
            result
                .get("Html")
                .and_then(Value::as_str)
                .ok_or("No se recibió el enlace de la lista creada")?,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn list_links_and_payloads_are_validated_before_writes() {
        assert!(list_url("https://evil.invalid/user/lists/demo_1").is_err());
        assert!(list_url("/u/lists/../../login_1").is_err());
        assert!(list_url("/u/lists/demo_1").is_ok());
        let draft = Draft {
            title: "Lista".into(),
            comics: vec![Item::default()],
            private: true,
            ..Default::default()
        };
        let body = payload(&draft, &[42]).unwrap();
        assert_eq!(body["p"], true);
        assert_eq!(body["cl"], json!(["42"]));
        assert!(payload(&draft, &[]).is_err());
        assert!(payload(&draft, &[0]).is_err());
    }
    #[test]
    fn list_detail_keeps_order_and_rejects_a_different_identity() {
        let html = r#"<div id="list" data-item-id="12"><h1><span>Mi lista</span><small>1 cómic</small></h1><span class="p-like active"><span class="text">4</span></span><ul id="issuelist"><li><span class="image"><a href="/comics/abc/demo/1"><img src="https://i1.whakoom.com/thumb/a.jpg"></a></span><span class="title"><a href="/comics/abc/demo/1">Demo</a></span></li></ul><button id="loadmoreissues" data-p-next="2"></button></div>"#;
        let list = parse_detail(html, "/user/lists/demo_12").unwrap();
        assert!(list.liked);
        assert_eq!(list.comics.len(), 1);
        assert_eq!(list.next, Some(2));
        assert!(parse_detail(html, "/user/lists/demo_13").is_err());
    }
}

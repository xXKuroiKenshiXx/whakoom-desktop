use crate::api::{self, Api, Page};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Section {
    #[default]
    Popular,
    TopRated,
    GraphicNovels,
    AllComics,
    Wanted,
}
impl Section {
    pub const EXPLORE: [Self; 4] = [
        Self::Popular,
        Self::TopRated,
        Self::GraphicNovels,
        Self::AllComics,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Popular => "Populares",
            Self::TopRated => "Mejor valorados",
            Self::GraphicNovels => "Novelas gráficas",
            Self::AllComics => "Todos los cómics",
            Self::Wanted => "Buscados",
        }
    }
    pub fn path(self) -> &'static str {
        match self {
            Self::Popular => "/explore",
            Self::TopRated => "/explore/top_rated",
            Self::GraphicNovels => "/explore/oneshot",
            Self::AllComics => "/explore/whole_catalog",
            Self::Wanted => "/buscados",
        }
    }
}
pub fn parse(html: &str, page: u32) -> Page {
    let doc = Html::parse_document(html);
    let links = Selector::parse("a[href]").unwrap();
    let next = doc
        .select(&links)
        .filter_map(|a| {
            let url = url::Url::parse(&api::safe_url(a.value().attr("href")?).ok()?).ok()?;
            if !url.path().starts_with("/explore") {
                return None;
            }
            url.query_pairs()
                .find(|(key, _)| key == "page")?
                .1
                .parse::<u32>()
                .ok()
        })
        .find(|next| *next > page && *next <= 1000);
    Page {
        items: api::parse_items(html),
        next,
    }
}
impl Api {
    pub fn discover(&self, section: Section, page: u32) -> Result<Page, String> {
        if !(1..=1000).contains(&page) {
            return Err("Página inválida".into());
        }
        if section == Section::Wanted {
            return self.collection(page, "", true);
        }
        let path = format!("{}?page={page}", section.path());
        Ok(parse(&self.html(&path)?, page))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_and_external_links_cannot_become_catalog_items_or_next_pages() {
        let page = parse(
            r#"<a href="https://evil.invalid/?page=99">Next</a><a href="/explore?page=2">Más</a><a class="title" href="/ediciones/123/demo"><img src="https://i1.whakoom.com/small/a.jpg"><strong>Serie</strong></a>"#,
            1,
        );
        assert_eq!(page.next, Some(2));
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].title, "Serie");
    }
}

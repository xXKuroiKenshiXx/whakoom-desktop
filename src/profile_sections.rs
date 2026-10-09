use crate::{
    api::{self, Api, Page},
    lists, social,
};
use scraper::{Html, Selector};
use serde_json::json;

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Section {
    #[default]
    Activity,
    Collection,
    Wanted,
    Lists,
    Following,
    Followers,
}
impl Section {
    pub const ALL: [Self; 6] = [
        Self::Activity,
        Self::Collection,
        Self::Wanted,
        Self::Lists,
        Self::Following,
        Self::Followers,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Activity => "Actividad",
            Self::Collection => "Comicteca",
            Self::Wanted => "Buscados",
            Self::Lists => "Listas",
            Self::Following => "Seguidos",
            Self::Followers => "Seguidores",
        }
    }
    fn suffix(self) -> &'static str {
        match self {
            Self::Activity => "",
            Self::Collection => "/collection",
            Self::Wanted => "/wanted",
            Self::Lists => "/lists",
            Self::Following => "/following",
            Self::Followers => "/followers",
        }
    }
}
#[derive(Clone, Default)]
pub struct Content {
    pub comics: Page,
    pub lists: lists::ListPage,
    pub people: Vec<social::User>,
}
pub fn parse_comics(html: &str) -> Result<Page, String> {
    let doc = Html::parse_document(html);
    let root = doc
        .select(&Selector::parse("ul.ct-edition-list").unwrap())
        .next()
        .ok_or("Esta comicteca no está disponible: puede ser privada o estar restringida")?;
    let next = doc
        .select(&Selector::parse("#hdNextPage").unwrap())
        .next()
        .and_then(|e| e.value().attr("value"))
        .and_then(|v| v.parse::<u32>().ok())
        .filter(|n| *n > 1 && *n <= 1000);
    Ok(Page {
        items: api::parse_items(&root.html()),
        next,
    })
}
impl Api {
    pub fn profile_section(
        &self,
        user: &str,
        section: Section,
        page: u32,
    ) -> Result<Content, String> {
        if !(1..=1000).contains(&page) {
            return Err("Página inválida".into());
        }
        if section == Section::Lists {
            return Ok(Content {
                lists: self.lists(lists::Section::Mine, user, page)?,
                ..Default::default()
            });
        }
        if matches!(section, Section::Following | Section::Followers) {
            if page != 1 {
                return Err("Esta sección no admite esa página".into());
            }
            let path = format!("{}{}", social::user_path(user)?, section.suffix());
            let html = self.html(&path)?;
            return Ok(Content {
                people: social::friends(&html),
                ..Default::default()
            });
        }
        if !matches!(section, Section::Collection | Section::Wanted) {
            return Err("Sección inválida".into());
        }
        let path = format!("{}{}", social::user_path(user)?, section.suffix());
        let html = self.html(&path)?;
        if page == 1 {
            return Ok(Content {
                comics: parse_comics(&html)?,
                ..Default::default()
            });
        }
        let doc = Html::parse_document(&html);
        let mode = doc
            .select(&Selector::parse("#hdMode").unwrap())
            .next()
            .and_then(|e| e.value().attr("value"))
            .and_then(|v| v.parse::<u32>().ok())
            .ok_or("No hay más páginas disponibles para este perfil")?;
        let data = self.post_referred(
            "/pwkws.asmx/PProfilePage",
            json!({"m":mode,"p":page}),
            &path,
        )?;
        let fragment = data["Html"].as_str().ok_or("Página de perfil incompleta")?;
        let items = api::parse_items(fragment);
        let next = data["ExtraInfo"]
            .as_u64()
            .or_else(|| data["ExtraInfo"].as_str()?.parse().ok())
            .filter(|n| *n > page as u64 && *n <= 1000)
            .map(|n| n as u32);
        Ok(Content {
            comics: Page { items, next },
            ..Default::default()
        })
    }
    pub fn search_users(
        &self,
        query: &str,
        page: u32,
    ) -> Result<(Vec<social::User>, Option<u32>), String> {
        if query.trim().is_empty() || query.len() > 500 || !(1..=1000).contains(&page) {
            return Err("Búsqueda de usuarios inválida".into());
        }
        let data = self.post(
            "/search.aspx/Query",
            json!({"q":query,"ft":5,"fit":"","fp":"","fl":"","p":page}),
        )?;
        let html = data["searchResult"]
            .as_str()
            .ok_or("Cambió la búsqueda de usuarios")?;
        let next = data["nextPage"]
            .as_u64()
            .filter(|p| *p > page as u64 && *p <= 1000)
            .map(|p| p as u32);
        Ok((social::friends(html), next))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn restricted_profiles_are_not_treated_as_empty_collections() {
        assert!(parse_comics("<div class='private'>Privado</div>").is_err());
        assert!(
            parse_comics("<ul class='ct-edition-list'></ul>")
                .unwrap()
                .items
                .is_empty()
        );
        let page=parse_comics("<ul class='ct-edition-list'><li><a href='/ediciones/123/test'><img alt='Serie'></a></li></ul><input id='hdNextPage' value='2'>").unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.next, Some(2));
        assert_eq!(Section::Wanted.suffix(), "/wanted");
    }
    #[test]
    fn user_search_markup_opens_only_valid_profile_paths() {
        let html = "<div class='sresult-user'><p class='img'><a href='/reader'><img src='https://i1.whakoom.com/avatar.png'></a></p><p><a href='/reader'><strong>Reader</strong> @reader</a></p></div><div class='sresult-user'><p class='img'><a href='https://evil.example/reader'><img></a></p></div>";
        let users = social::friends(html);
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].username, "reader");
        assert_eq!(users[0].name, "Reader");
    }
}

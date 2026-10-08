use crate::{
    api::{self, Api, Item},
    sync,
};
use scraper::{Html, Selector};
use std::collections::HashSet;

pub fn initial(html: &str) -> Result<Vec<Item>, String> {
    let doc = Html::parse_document(html);
    let list = doc
        .select(&Selector::parse("ul.ct-edition-list").unwrap())
        .next()
        .ok_or("Whakoom no devolvió la lista de Buscados; se conserva la copia anterior")?;
    Ok(api::parse_items(&list.html()))
}
pub fn merge(first: Vec<Item>, rest: Vec<Item>) -> Vec<Item> {
    let mut seen = HashSet::new();
    first
        .into_iter()
        .chain(rest)
        .filter(|item| seen.insert(item.key.clone()))
        .collect()
}
impl Api {
    pub fn wanted_all(&self, mut cancelled: impl FnMut() -> bool) -> Result<Vec<Item>, String> {
        if cancelled() {
            return Err("Actualización detenida".into());
        }
        let first = initial(&self.html("/buscados")?)?;
        let rest = sync::pages(|page| self.collection(page, "", true), &mut cancelled)?;
        Ok(merge(first, rest))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_page_and_service_are_merged_without_dropping_editions_or_duplicate_volumes() {
        let first=initial("<ul class='ct-edition-list'><li><p class='image'><a href='/comics/a/demo/1'><img src='https://i1.whakoom.com/small/a.jpg'></a></p><p class='title'><a href='/comics/a/demo/1'>Demo</a></p></li><li><p class='image'><a href='/ediciones/123/serie'><img src='https://i1.whakoom.com/small/b.jpg'></a></p><p class='title'><a href='/ediciones/123/serie'>Serie completa</a></p></li></ul>").unwrap();
        assert_eq!(first.len(), 2);
        let rest = vec![
            first[0].clone(),
            Item {
                key: "comicb".into(),
                ..Default::default()
            },
        ];
        let merged = merge(first, rest);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[1].key, "edicion123");
        assert!(initial("<form action='/login'></form>").is_err());
        assert!(
            initial("<ul class='ct-edition-list'></ul>")
                .unwrap()
                .is_empty()
        );
    }
}

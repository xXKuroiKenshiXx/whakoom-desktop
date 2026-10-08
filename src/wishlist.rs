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
pub fn complete_known(
    mut current: Vec<Item>,
    candidates: &[Item],
    mut wanted: impl FnMut(&Item) -> Result<bool, String>,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Vec<Item>, String> {
    let mut seen: HashSet<_> = current.iter().map(|i| i.key.clone()).collect();
    for item in candidates {
        if cancelled() {
            return Err("Actualización detenida; se conserva la copia anterior".into());
        }
        if seen.insert(item.key.clone()) && wanted(item)? {
            current.push(item.clone());
        }
    }
    Ok(current)
}
impl Api {
    pub fn wanted_complete(
        &self,
        candidates: &[Item],
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<Vec<Item>, String> {
        let mut result = self.wanted_all(&mut cancelled)?;
        let owner = self.profile()?;
        let profile = sync::pages(
            |page| {
                self.profile_section(&owner, crate::profile_sections::Section::Wanted, page)
                    .map(|p| p.comics)
            },
            &mut cancelled,
        )?;
        result = merge(result, profile);
        complete_known(
            result,
            candidates,
            |item| self.detail(item).map(|d| d.wanted),
            &mut cancelled,
        )
    }
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

#[cfg(test)]
mod completeness_tests {
    use super::*;
    #[test]
    fn missing_wishes_are_verified_and_removed_cached_wishes_do_not_return() {
        let items: Vec<_> = ["comica", "comicb", "edicion12"]
            .into_iter()
            .map(|key| Item {
                key: key.into(),
                ..Default::default()
            })
            .collect();
        let result = complete_known(
            vec![items[0].clone()],
            &items,
            |i| Ok(i.key == "edicion12"),
            || false,
        )
        .unwrap();
        assert_eq!(
            result.iter().map(|i| i.key.as_str()).collect::<Vec<_>>(),
            ["comica", "edicion12"]
        );
        assert!(complete_known(vec![], &items, |_| Err("Sin red".into()), || false).is_err());
        assert!(complete_known(vec![], &items, |_| Ok(true), || true).is_err());
    }
}

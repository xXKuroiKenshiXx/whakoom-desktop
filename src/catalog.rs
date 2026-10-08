use crate::api::{Item, Page};
use std::collections::HashSet;

/// Collect a complete edition before changing any collection marks.
pub fn all_volumes(
    mut fetch: impl FnMut(u32) -> Result<Page, String>,
    mut cancelled: impl FnMut() -> bool,
) -> Result<Vec<Item>, String> {
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let mut page_number = 1;
    for _ in 0..1250 {
        if cancelled() {
            return Err("Importación de la serie detenida. Tu colección no cambió".into());
        }
        let page = fetch(page_number)?;
        if cancelled() {
            return Err("Importación de la serie detenida. Tu colección no cambió".into());
        }
        if page.items.is_empty() {
            if page.next.is_some() {
                return Err(
                    "Whakoom devolvió una página vacía con más resultados pendientes".into(),
                );
            }
            break;
        }
        let previous = seen.len();
        for item in page.items {
            if !item.key.starts_with("comic")
                || crate::api::key_from_url(&item.url).as_ref() != Some(&item.key)
            {
                return Err("La serie contiene una ficha inválida".into());
            }
            if seen.insert(item.key.clone()) {
                items.push(item);
            }
        }
        if seen.len() == previous {
            return Err("Whakoom repitió una página. No se añadió una serie incompleta".into());
        }
        if items.len() > 100_000 {
            return Err("La serie supera el límite de 100.000 tomos".into());
        }
        match page.next {
            None => break,
            Some(next) if next > page_number => page_number = next,
            Some(_) => return Err("La paginación de la serie no avanzó".into()),
        }
        if page_number > 1250 {
            return Err("La serie supera el límite de 1.250 páginas".into());
        }
    }
    if items.is_empty() {
        return Err("La serie no tiene tomos disponibles para añadir".into());
    }
    items.sort_by(crate::series::volume_order);
    Ok(items)
}

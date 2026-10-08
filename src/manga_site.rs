use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, io::Read, time::Duration};
const BASE: &str = "https://www.listadomanga.es";
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Link {
    pub title: String,
    pub url: String,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Block {
    pub title: String,
    pub text: String,
    pub cover: String,
    pub links: Vec<Link>,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Page {
    pub url: String,
    pub title: String,
    pub blocks: Vec<Block>,
    pub results: Vec<Link>,
}
fn sel(value: &str) -> Selector {
    Selector::parse(value).unwrap()
}
pub fn safe_url(value: &str) -> Result<String, String> {
    let url = url::Url::parse(BASE)
        .unwrap()
        .join(value)
        .map_err(|_| "Dirección de Listado Manga inválida")?;
    if url.scheme() != "https"
        || !matches!(
            url.host_str(),
            Some("www.listadomanga.es" | "listadomanga.es")
        )
        || url.port_or_known_default() != Some(443)
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Sólo se permite navegar por Listado Manga".into());
    }
    Ok(url.into())
}
pub fn search_url(query: &str) -> String {
    let mut url = url::Url::parse(&format!("{BASE}/buscar.php")).unwrap();
    url.query_pairs_mut().append_pair("b", query);
    url.into()
}
pub fn parse(html: &str, url: &str) -> Result<Page, String> {
    let doc = Html::parse_document(html);
    if url::Url::parse(&safe_url(url)?).is_ok_and(|u| u.path() == "/lista.php") {
        let mut seen = HashSet::new();
        let results = doc
            .select(&sel("a[href]"))
            .filter_map(|e| {
                let url = safe_url(e.value().attr("href")?).ok()?;
                if url::Url::parse(&url).ok()?.path() != "/coleccion.php"
                    || !seen.insert(url.clone())
                {
                    return None;
                }
                let title = e.text().collect::<String>().trim().to_string();
                (!title.is_empty()).then_some(Link { title, url })
            })
            .take(10000)
            .collect();
        return Ok(Page {
            url: safe_url(url)?,
            title: "Listado Manga".into(),
            results,
            ..Default::default()
        });
    }
    let mut blocks = vec![];
    for element in doc.select(&sel("table[class^='ventana_id'] > tbody > tr > td.izq, table[class^='ventana_id'] > tbody > tr > td.cen")) {
        let title=element.select(&sel("h1,h2,h3")).next().map(|e|e.text().collect::<String>()).unwrap_or_default();
        let fragment=Html::parse_fragment(&element.inner_html().replace("<br>","\n").replace("<br/>","\n").replace("<br />","\n"));
        let text=fragment.root_element().descendants().filter_map(|n|n.value().as_text().filter(|_|!n.ancestors().filter_map(scraper::ElementRef::wrap).any(|e|matches!(e.value().name(),"script"|"style"))).map(|t|t.to_string())).collect::<String>().lines().map(str::trim).filter(|t|!t.is_empty()).collect::<Vec<_>>().join("\n");
        if text.is_empty() {continue;}
        let cover=element.select(&sel("img.portada[src]")).next().and_then(|e|e.value().attr("src")).filter(|s| s.starts_with("https://static.listadomanga.com/")).unwrap_or_default().into();
        let mut seen=HashSet::new();
        let links=element.select(&sel("a[href]")).filter_map(|e| {let url=safe_url(e.value().attr("href")?).ok()?;let title=e.text().collect::<String>().trim().to_string();(!title.is_empty()&&seen.insert(url.clone())).then_some(Link{title,url})}).take(1500).collect();
        blocks.push(Block{title,text:text.chars().take(20000).collect(),cover,links});
        if blocks.len()>=2000 {break;}
    }
    if blocks.is_empty() {
        return Err("No se pudo leer el contenido de Listado Manga".into());
    }
    let title = doc
        .select(&sel("h1"))
        .next()
        .map(|e| e.text().collect())
        .unwrap_or_else(|| "Listado Manga".into());
    Ok(Page {
        url: safe_url(url)?,
        title,
        blocks,
        ..Default::default()
    })
}
pub fn fetch(url: &str, query: Option<&str>) -> Result<Page, String> {
    let target = if let Some(q) = query {
        if q.trim().is_empty() || q.len() > 500 {
            return Err("Escribí un título para buscar".into());
        }
        search_url(q)
    } else {
        safe_url(url)?
    };
    let response = reqwest::blocking::Client::builder()
        .user_agent(crate::api::USER_AGENT)
        .timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::custom(|a| {
            if a.previous().len() < 5 && safe_url(a.url().as_str()).is_ok() {
                a.follow()
            } else {
                a.error("Redirección externa rechazada")
            }
        }))
        .build()
        .map_err(|e| e.to_string())?
        .get(&target)
        .send()
        .map_err(|_| "No se pudo conectar con Listado Manga")?
        .error_for_status()
        .map_err(|_| "Listado Manga no devolvió la página")?;
    let mut bytes = vec![];
    response
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err("Página demasiado grande".into());
    }
    if let Some(q) = query {
        let data: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "Respuesta de búsqueda inválida")?;
        let results = data["colecciones"]
            .as_array()
            .ok_or("Cambió la búsqueda de Listado Manga")?
            .iter()
            .filter_map(|v| {
                let id = v["id"]
                    .as_u64()
                    .or_else(|| v["id"].as_str()?.parse().ok())?;
                let title = v["nombre"].as_str()?.to_string();
                Some(Link {
                    title,
                    url: format!("{BASE}/coleccion.php?id={id}"),
                })
            })
            .take(1000)
            .collect();
        Ok(Page {
            url: format!("{BASE}/buscador.php"),
            title: format!("Listado Manga · {q}"),
            results,
            ..Default::default()
        })
    } else {
        parse(
            &String::from_utf8(bytes).map_err(|_| "Página con codificación inválida")?,
            &target,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn navigation_is_restricted_and_collection_text_preserves_lines() {
        assert!(safe_url("https://evil.example/a").is_err());
        assert!(safe_url("javascript:alert(1)").is_err());
        let p=parse("<table class='ventana_id1'><tr><td class='izq'><h1>Serie</h1><b>Formato:</b> Tomo<br/>12 números<a href='/coleccion.php?id=31'>Otra edición</a><script>evil()</script></td></tr></table>","/coleccion.php?id=31").unwrap();
        assert!(p.blocks[0].text.contains('\n'));
        assert!(!p.blocks[0].text.contains("evil()"));
        assert_eq!(p.blocks[0].links.len(), 1);
        assert!(search_url("A&B #2").contains("A%26B+%232"));
    }
}

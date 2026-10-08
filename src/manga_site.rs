use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, io::Read, time::Duration};
const BASE: &str = "https://www.listadomanga.es";
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Link {
    pub title: String,
    pub url: String,
    #[serde(default)]
    pub cover: String,
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
    #[serde(default)]
    pub queries: Vec<String>,
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
                (!title.is_empty()).then_some(Link {
                    title,
                    url,
                    ..Default::default()
                })
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
        let links=element.select(&sel("a[href]")).filter_map(|e| {let url=safe_url(e.value().attr("href")?).ok()?;let title=e.text().collect::<String>().trim().to_string();(!title.is_empty()&&seen.insert(url.clone())).then_some(Link{title,url,..Default::default()})}).take(1500).collect();
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
fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(crate::api::USER_AGENT)
        .timeout(Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::custom(|a| {
            if a.previous().len() < 5 && safe_url(a.url().as_str()).is_ok() {
                a.follow()
            } else {
                a.error("Redirección externa rechazada")
            }
        }))
        .build()
        .map_err(|e| e.to_string())
}
fn read(client: &reqwest::blocking::Client, url: &str) -> Result<Vec<u8>, String> {
    let response = client
        .get(safe_url(url)?)
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
    Ok(bytes)
}
fn words(query: &str) -> Vec<String> {
    query
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}
pub fn search_variants(query: &str) -> Vec<String> {
    let mut result = vec![];
    let mut seen = HashSet::new();
    let mut add = |q: String| {
        let q = q.trim().to_string();
        if !q.is_empty() && seen.insert(q.to_lowercase()) {
            result.push(q);
        }
    };
    add(query.trim().to_string());
    add(query.replace('\'', "’"));
    let base = query
        .split(" - ")
        .next()
        .unwrap_or(query)
        .split('(')
        .next()
        .unwrap_or(query)
        .trim();
    add(base.into());
    let tokens = words(base);
    add(tokens.join(" "));
    for token in tokens.iter().filter(|w| {
        w.len() >= 3
            && !matches!(
                w.as_str(),
                "the" | "and" | "del" | "los" | "las" | "edition" | "edición" | "complete"
            )
    }) {
        add(token.clone());
        if token.chars().count() > 6 {
            add(token.chars().take(5).collect());
        }
    }
    result.truncate(6);
    result
}
fn parse_search(bytes: &[u8]) -> Result<Vec<Link>, String> {
    let data: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|_| "Respuesta de búsqueda inválida")?;
    Ok(data["colecciones"]
        .as_array()
        .ok_or("Cambió la búsqueda de Listado Manga")?
        .iter()
        .filter_map(|v| {
            let id = v["id"]
                .as_u64()
                .or_else(|| v["id"].as_str()?.parse().ok())?;
            Some(Link {
                title: v["nombre"].as_str()?.to_string(),
                url: format!("{BASE}/coleccion.php?id={id}"),
                ..Default::default()
            })
        })
        .take(1000)
        .collect())
}
pub fn fetch(url: &str, query: Option<&str>) -> Result<Page, String> {
    let connector = client()?;
    let Some(q) = query else {
        let target = safe_url(url)?;
        return parse(
            &String::from_utf8(read(&connector, &target)?)
                .map_err(|_| "Página con codificación inválida")?,
            &target,
        );
    };
    if q.trim().is_empty() || q.len() > 500 {
        return Err("Escribí un título para buscar".into());
    }
    let queries = search_variants(q);
    let mut results = vec![];
    let mut errors = vec![];
    let mut seen = HashSet::new();
    for batch in queries.chunks(3) {
        let replies = std::thread::scope(|scope| {
            let jobs: Vec<_> = batch
                .iter()
                .map(|variant| {
                    let c = &connector;
                    scope.spawn(move || parse_search(&read(c, &search_url(variant))?))
                })
                .collect();
            jobs.into_iter()
                .map(|job| {
                    job.join()
                        .unwrap_or_else(|_| Err("La búsqueda se interrumpió".into()))
                })
                .collect::<Vec<_>>()
        });
        for reply in replies {
            match reply {
                Ok(links) => {
                    for link in links {
                        if seen.insert(link.url.clone()) {
                            results.push(link);
                        }
                    }
                }
                Err(e) => errors.push(e),
            }
        }
    }
    if errors.len() == queries.len() {
        return Err(errors.into_iter().next().unwrap());
    }
    let tokens = words(q);
    results.sort_by_cached_key(|link| {
        let title = words(&link.title).join(" ");
        std::cmp::Reverse(
            tokens
                .iter()
                .filter(|t| t.len() >= 3 && title.contains(t.as_str()))
                .count(),
        )
    });
    results.truncate(100);
    Ok(Page {
        url: format!("{BASE}/buscador.php"),
        title: format!("Listado Manga · {q}"),
        results,
        queries,
        ..Default::default()
    })
}
#[cfg(test)]
mod tests {
    #[test]
    fn search_variants_include_typographic_and_partial_titles_without_duplicate_requests() {
        let variants =
            search_variants("A Returner's Magic Should Be Special - Complete (Hardcover)");
        assert!(variants.len() <= 6);
        assert!(variants.iter().any(|q| q.contains("Returner’s")));
        assert!(variants.iter().any(|q| q == "returner"));
        let chain = search_variants("Chainsaw Man");
        assert!(
            chain.contains(&"chainsaw".into())
                && chain.contains(&"chain".into())
                && chain.contains(&"man".into())
        );
        assert_eq!(
            chain
                .iter()
                .map(|q| q.to_lowercase())
                .collect::<HashSet<_>>()
                .len(),
            chain.len()
        );
        let parsed = parse_search(
            br#"{"colecciones":[{"id":"31","nombre":"Ejemplo"},{"id":"../evil","nombre":"Mal"}]}"#,
        )
        .unwrap();
        assert_eq!(parsed.len(), 1);
        assert!(parsed[0].cover.is_empty());
    }

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

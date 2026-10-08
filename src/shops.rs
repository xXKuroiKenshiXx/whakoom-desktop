use crate::api::{Api, Detail, Item};
use scraper::{ElementRef, Html, Selector};
#[derive(Clone, Default, Debug)]
pub struct Shop {
    pub title: String,
    pub url: String,
    pub price: String,
}
pub fn query(item: &Item) -> String {
    format!("{} {}", item.title, item.issue.trim_start_matches('#'))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn mercado(item: &Item, currency: &str) -> String {
    let country = match currency {
        "MXN" => "com.mx",
        "BRL" => "com.br",
        "CLP" => "cl",
        "COP" => "com.co",
        "UYU" => "com.uy",
        "PEN" => "com.pe",
        _ => "com.ar",
    };
    let mut url = url::Url::parse(&format!("https://listado.mercadolibre.{country}/")).unwrap();
    url.path_segments_mut()
        .unwrap()
        .push(&query(item).replace(' ', "-"));
    url.into()
}
pub fn amazon(item: &Item) -> String {
    let mut url = url::Url::parse("https://www.amazon.es/s").unwrap();
    url.query_pairs_mut().append_pair("k", &query(item));
    url.into()
}
pub fn safe_url(value: &str) -> Option<String> {
    if let Ok(url) = crate::api::safe_url(value) {
        return Some(url);
    }
    let u = url::Url::parse(value).ok()?;
    if u.scheme() != "https"
        || u.port_or_known_default() != Some(443)
        || !u.username().is_empty()
        || u.password().is_some()
    {
        return None;
    }
    matches!(
        u.host_str(),
        Some(
            "www.amazon.es"
                | "amazon.es"
                | "www.amazon.com"
                | "amazon.com"
                | "amzn.to"
                | "listado.mercadolibre.com.ar"
                | "listado.mercadolibre.com.mx"
                | "listado.mercadolibre.com.br"
                | "listado.mercadolibre.cl"
                | "listado.mercadolibre.com.co"
                | "listado.mercadolibre.com.uy"
                | "listado.mercadolibre.com.pe"
        )
    )
    .then(|| u.into())
}
pub fn parse(html: &str) -> Vec<Shop> {
    let doc = Html::parse_fragment(html);
    doc.select(&Selector::parse("a.buy-action[href]").unwrap())
        .filter_map(|a| {
            if a.value()
                .classes()
                .any(|c| c == "disabled" || c == "no-stock")
            {
                return None;
            }
            let url = safe_url(a.value().attr("href")?)?;
            let parent = a
                .ancestors()
                .filter_map(ElementRef::wrap)
                .find(|e| e.value().classes().any(|c| c == "shop"));
            let shop_name = parent
                .and_then(|e| e.select(&Selector::parse(".shop-name").unwrap()).next())
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            let price = parent
                .and_then(|e| e.select(&Selector::parse("strong.price").unwrap()).next())
                .map(|e| e.text().collect::<String>().trim().to_string())
                .unwrap_or_default();
            let title = if a.value().classes().any(|c| c == "buy-amz") {
                "Amazon · Whakoom".into()
            } else if !shop_name.is_empty() {
                shop_name
            } else {
                a.text().collect::<String>().trim().to_string()
            };
            Some(Shop { title, url, price })
        })
        .collect()
}
impl Api {
    pub fn shops(&self, detail: &Detail) -> Result<Vec<Shop>, String> {
        let full = if detail.shop_id.is_empty() {
            self.full_detail(&detail.item)?
        } else {
            detail.clone()
        };
        if full.shop_id.is_empty() {
            return Ok(vec![]);
        }
        let data = self.post(
            "/pwkws.asmx/ShopComicShops",
            serde_json::json!({"cguid":full.shop_id}),
        )?;
        Ok(parse(
            data["Html"]
                .as_str()
                .ok_or("No se pudieron leer las tiendas de Whakoom")?,
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn market_queries_encode_titles_and_volume_without_external_code() {
        let item = Item {
            title: "Serie & amigos".into(),
            issue: "#3".into(),
            ..Default::default()
        };
        assert!(mercado(&item, "ARS").starts_with("https://listado.mercadolibre.com.ar/"));
        assert!(mercado(&item, "ARS").ends_with("Serie-&-amigos-3"));
        assert!(amazon(&item).contains("Serie+%26+amigos+3"));
        assert!(safe_url("javascript:alert(1)").is_none());
        assert!(safe_url("https://www.amazon.es.evil.example/a").is_none());
        assert_eq!(parse("<a class='buy-action buy-amz' href='/buy?a=1'>Comprar</a><a class='buy-action' href='https://evil.example'>Comprar</a>").len(),1);
    }
}

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::{io::Read, time::Duration};

pub const BASE: &str = "https://whakoom.zendesk.com";
#[derive(Clone, Copy, Default, Debug, Serialize, Deserialize)]
pub enum Request {
    #[default]
    Topics,
    Posts(u64, u32),
    Thread(u64),
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Topic {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub description: String,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Post {
    pub id: u64,
    pub title: String,
    #[serde(default, alias = "details")]
    pub body: String,
    pub html_url: String,
    #[serde(default)]
    pub comment_count: usize,
    #[serde(default)]
    pub created_at: String,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Comment {
    pub body: String,
    #[serde(default)]
    pub created_at: String,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub topics: Vec<Topic>,
    #[serde(default)]
    pub posts: Vec<Post>,
    #[serde(default)]
    pub post: Option<Post>,
    #[serde(default)]
    pub comments: Vec<Comment>,
    pub more: bool,
}
pub fn safe_url(input: &str) -> Result<String, String> {
    let url = url::Url::parse(BASE)
        .unwrap()
        .join(input)
        .map_err(|_| "Enlace de ayuda inválido")?;
    if url.scheme() != "https"
        || url.host_str() != Some("whakoom.zendesk.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("Sólo se admiten enlaces del centro de ayuda de Whakoom".into());
    }
    Ok(url.to_string())
}
pub fn plain(html: &str) -> String {
    let doc = scraper::Html::parse_fragment(html);
    doc.root_element()
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn get(client: &Client, path: &str) -> Result<serde_json::Value, String> {
    let response = client
        .get(safe_url(path)?)
        .send()
        .map_err(|_| "No se pudo conectar al centro de ayuda")?;
    let status = response.status();
    if !status.is_success() {
        return Err(format!("El centro de ayuda respondió HTTP {status}"));
    }
    let mut bytes = Vec::new();
    response
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "No se pudo leer la ayuda")?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("La respuesta de ayuda es demasiado grande".into());
    }
    serde_json::from_slice(&bytes).map_err(|_| "Formato de ayuda no reconocido".into())
}
pub fn fetch(request: Request, offline: bool) -> Result<Page, String> {
    let cache = crate::session::data_dir().join("pages").join(format!(
        "help-{}.json",
        crate::storage::key(&format!("{request:?}"))
    ));
    if offline {
        return std::fs::read(cache)
            .ok()
            .filter(|b| b.len() <= 2 * 1024 * 1024)
            .and_then(|b| serde_json::from_slice(&b).ok())
            .ok_or("No hay una copia guardada de esta sección de ayuda".into());
    }
    let client = Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent(crate::api::USER_AGENT)
        .redirect(reqwest::redirect::Policy::custom(|attempt| {
            if attempt.previous().len() >= 4 || safe_url(attempt.url().as_str()).is_err() {
                attempt.error("Redirección de ayuda rechazada")
            } else {
                attempt.follow()
            }
        }))
        .build()
        .map_err(|e| e.to_string())?;
    let mut result = Page::default();
    match request {
        Request::Topics => {
            result.topics = serde_json::from_value(
                get(&client, "/api/v2/community/topics.json?per_page=100")?["topics"].clone(),
            )
            .map_err(|_| "Categorías inválidas")?;
        }
        Request::Posts(id, page) if id > 0 && (1..=100).contains(&page) => {
            let response = get(
                &client,
                &format!(
                    "/api/v2/community/topics/{id}/posts.json?page={page}&per_page=25&sort_by=recent_activity"
                ),
            )?;
            result.posts = serde_json::from_value(response["posts"].clone())
                .map_err(|e| format!("Publicaciones inválidas: {e}"))?;
            result.more = response["next_page"].as_str().is_some();
        }
        Request::Thread(id) if id > 0 => {
            result.post = serde_json::from_value(
                get(&client, &format!("/api/v2/community/posts/{id}.json"))?["post"].clone(),
            )
            .map_err(|_| "Publicación inválida")?;
            let response = get(
                &client,
                &format!("/api/v2/community/posts/{id}/comments.json?per_page=100"),
            )?;
            result.comments = serde_json::from_value(response["comments"].clone())
                .map_err(|_| "Comentarios inválidos")?;
            result.more = response["next_page"].as_str().is_some();
        }
        _ => return Err("Solicitud de ayuda inválida".into()),
    }
    let bytes = serde_json::to_vec(&result).map_err(|e| e.to_string())?;
    if bytes.len() <= 2 * 1024 * 1024 {
        let _ = crate::storage::atomic_write(&cache, &bytes);
    }
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn help_links_are_isolated_from_account_credentials_and_html_is_plain_text() {
        assert!(safe_url("https://whakoom.zendesk.com.evil.invalid/hc/es").is_err());
        assert!(safe_url("https://user:password@whakoom.zendesk.com/hc/es").is_err());
        assert!(safe_url("http://whakoom.zendesk.com/hc/es").is_err());
        assert!(safe_url("https://www.whakoom.com/login").is_err());
        assert_eq!(plain("<p>Una &amp; otra</p><p>idea</p>"), "Una & otra idea");
        let post: Post=serde_json::from_value(serde_json::json!({"id":123,"title":"Idea","details":"<p>Una propuesta</p>","html_url":"https://whakoom.zendesk.com/hc/es/community/posts/123"})).unwrap();
        assert_eq!(plain(&post.body), "Una propuesta");
    }
}

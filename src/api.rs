use reqwest::cookie::CookieStore;
use reqwest::{blocking::Client, header};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{collections::HashSet, io::Read, time::Duration};

pub const BASE: &str = "https://www.whakoom.com";
pub const USER_AGENT: &str = concat!(
    "WhakoomDesktop/",
    env!("CARGO_PKG_VERSION"),
    " (unofficial desktop client)"
);
const MAX_BODY: u64 = 4 * 1024 * 1024;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Item {
    pub key: String,
    pub title: String,
    pub issue: String,
    pub url: String,
    pub cover: String,
    #[serde(default)]
    pub publisher: String,
    pub owned: bool,
    #[serde(default)]
    pub community_rating: f32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Detail {
    pub item: Item,
    pub description: String,
    pub authors: Vec<String>,
    pub publisher: String,
    pub language: String,
    pub date: String,
    pub rating: String,
    pub numeric_id: Option<u64>,
    pub wish_kind: String,
    pub wish_id: String,
    pub wanted: bool,
    pub read: bool,
    #[serde(default)]
    pub read_date: String,
    #[serde(default)]
    pub personal_rating: u8,
    #[serde(default)]
    pub edition: Option<Item>,
    #[serde(default)]
    pub discussion: crate::discussion::Discussion,
    #[serde(default)]
    pub isbn: Vec<String>,
    #[serde(default)]
    pub owners: Option<usize>,
    #[serde(default)]
    pub shop_id: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Page {
    pub items: Vec<Item>,
    pub next: Option<u32>,
}

fn sel(s: &str) -> Selector {
    Selector::parse(s).expect("selector interno válido")
}
fn text(e: ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
fn first(h: &Html, s: &str) -> String {
    h.select(&sel(s)).next().map(text).unwrap_or_default()
}
fn attr(h: &Html, s: &str, a: &str) -> String {
    h.select(&sel(s))
        .next()
        .and_then(|e| e.value().attr(a))
        .unwrap_or_default()
        .to_owned()
}
fn public_rating(value: &str) -> f32 {
    value
        .replace(',', ".")
        .parse::<f32>()
        .ok()
        .filter(|r| r.is_finite() && (0.0..=5.0).contains(r))
        .unwrap_or_default()
}

pub fn safe_url(input: &str) -> Result<String, String> {
    let base = url::Url::parse(BASE).unwrap();
    let url = base.join(input).map_err(|_| "URL inválida")?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.whakoom.com")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Err("Sólo se aceptan enlaces HTTPS de www.whakoom.com".into());
    }
    Ok(url.to_string())
}

pub fn key_from_url(url: &str) -> Option<String> {
    let u = url::Url::parse(&safe_url(url).ok()?).ok()?;
    let p: Vec<_> = u.path_segments()?.collect();
    let id = *p.get(1)?;
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    match *p.first()? {
        "comics" => Some(format!("comic{id}")),
        "ediciones" => Some(format!("edicion{id}")),
        _ => None,
    }
}

pub fn parse_items(html: &str) -> Vec<Item> {
    let h = Html::parse_fragment(html);
    let mut seen = HashSet::new();
    h.select(&sel("a[href]"))
        .filter_map(|a| {
            let href = a.value().attr("href")?;
            let key = key_from_url(href)?;
            let img = a.select(&sel("img")).next();
            let strong = a.select(&sel("strong")).next();
            let result = a.ancestors().filter_map(ElementRef::wrap).find(|e| {
                e.value().name() == "li"
                    || e.value().classes().any(|c| c == "sresult" || c == "item")
            });
            // Exclude toolbar links (Lo tengo, user actions, duplicate text links).
            if img.is_none() && strong.is_none() {
                return None;
            }
            let mut title = strong
                .map(text)
                .unwrap_or_else(|| a.value().attr("title").unwrap_or_default().to_owned());
            if title.is_empty() {
                title = img
                    .and_then(|i| i.value().attr("alt"))
                    .unwrap_or_default()
                    .to_owned();
            }
            let mut issue = a
                .select(&sel(".issue-number"))
                .next()
                .map(text)
                .or_else(|| {
                    a.select(&sel("span"))
                        .map(text)
                        .find(|t| t.starts_with('#'))
                })
                .unwrap_or_default();
            if let Some(result) = result
                && let Some(link) = result.select(&sel(".title a")).next()
                && link.value().attr("href").and_then(key_from_url).as_ref() == Some(&key)
            {
                title = text(link);
                if let Some(number) = link
                    .select(&sel("strong"))
                    .next()
                    .map(text)
                    .filter(|n| n.starts_with('#'))
                {
                    issue = number;
                    title = title.strip_suffix(&issue).unwrap_or(&title).trim().into();
                }
            }
            if title.is_empty() || !seen.insert(key.clone()) {
                return None;
            }
            let owned = a
                .ancestors()
                .filter_map(ElementRef::wrap)
                .any(|e| e.value().classes().any(|c| c == "got-it"))
                || result
                    .is_some_and(|r| r.select(&sel("button.rem[data-item-id]")).next().is_some());
            Some(Item {
                key,
                title,
                issue,
                url: safe_url(href).ok()?,
                cover: img
                    .and_then(|i| i.value().attr("src").or_else(|| i.value().attr("data-src")))
                    .unwrap_or_default()
                    .to_owned(),
                publisher: result
                    .and_then(|r| r.select(&sel(".publisher, .pub")).next())
                    .map(text)
                    .unwrap_or_default(),
                owned,
                community_rating: result
                    .and_then(|r| r.select(&sel(".rate-avg, [itemprop='ratingValue']")).next())
                    .map(text)
                    .map(|s| public_rating(&s))
                    .unwrap_or_default(),
            })
        })
        .collect()
}

pub fn parse_detail(html: &str, item: &Item) -> Result<Detail, String> {
    let h = Html::parse_document(html);
    if h.select(&sel("form[action*='/login']")).next().is_some() {
        return Err("Iniciá sesión en Whakoom para abrir esta ficha".into());
    }
    let title = first(
        &h,
        ".b-info h1 span, .b-info h1, .edition-header h1, h1[itemprop='name']",
    );
    if title.is_empty() {
        return Err("Whakoom cambió el formato de esta ficha; podés abrirla en su web".into());
    }
    let mut result = Detail {
        item: Item {
            title,
            ..item.clone()
        },
        ..Default::default()
    };
    let cover = attr(&h, ".comic-cover a, .edition-cover a", "href");
    if !cover.is_empty() {
        result.item.cover = cover;
    }
    result.description = h
        .select(&sel(".wiki-content p, .about-this-edition p"))
        .map(text)
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    result.authors = h.select(&sel(".authors a")).map(text).collect();
    result.publisher = first(&h, ".lang-pub [itemprop='publisher']");
    result.language = first(&h, ".lang-pub [itemprop='inLanguage']");
    result.date = attr(&h, ".info [itemprop='datePublished']", "content");
    result.rating = first(&h, ".b-info [itemprop='ratingValue'], .rate-avg");
    result.item.community_rating = public_rating(&result.rating);
    result.personal_rating = attr(
        &h,
        ".my-comic-review .stars, .my-review .stars",
        "data-item-id",
    )
    .parse()
    .ok()
    .filter(|r| *r <= 5)
    .unwrap_or_default();
    result.numeric_id = attr(&h, ".comic-detail", "data-item-id").parse().ok();
    result.shop_id = attr(&h, ".w-comic", "data-item-id");
    result.edition = h
        .select(&sel(".comic-detail a[href*='/ediciones/']"))
        .find_map(|a| {
            let url = safe_url(a.value().attr("href")?).ok()?;
            Some(Item {
                key: key_from_url(&url)?,
                title: text(a),
                url,
                publisher: result.publisher.clone(),
                ..Default::default()
            })
        });
    result.wish_kind = attr(&h, ".pub-detail", "data-item-type");
    result.wish_id = attr(&h, ".pub-detail", "data-item-id");
    if result.wish_kind.is_empty() {
        result.wish_kind = "comic".into();
    }
    if result.wish_id.is_empty() {
        result.wish_id = item.key.strip_prefix("comic").unwrap_or_default().into();
    }
    result.item.owned = h
        .select(&sel(".comicteca.rem, .owners .rem"))
        .next()
        .is_some();
    result.wanted = h.select(&sel("button.wanted.active")).next().is_some();
    result.read = h
        .select(&sel(
            ".comic-read .not-readed, .comic-read table.read-list tr[data-item-date]",
        ))
        .next()
        .is_some();
    let date = attr(
        &h,
        ".comic-read table.read-list tr[data-item-date]",
        "data-item-date",
    );
    if date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()) {
        let formatted = format!("{}-{}-{}", &date[..4], &date[4..6], &date[6..]);
        if crate::storage::reading_month(&formatted).is_some() {
            result.read_date = formatted;
        }
    } else if crate::storage::reading_month(&date).is_some() {
        result.read_date = date;
    }
    result.discussion = crate::discussion::parse(html);
    result.isbn = h
        .select(&sel(".barcodes [itemprop='isbn']"))
        .map(text)
        .filter(|s| !s.is_empty())
        .collect();
    if result.isbn.is_empty() {
        let value = attr(&h, "meta[property='books:isbn']", "content");
        if !value.is_empty() {
            result.isbn.push(value);
        }
    }
    result.owners = first(&h, ".alsohavethis h2 a span")
        .chars()
        .filter(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .ok();
    Ok(result)
}

pub fn unwrap_response(value: Value) -> Result<Value, String> {
    let v = value.get("d").unwrap_or(&value);
    if let Some(s) = v.as_str() {
        serde_json::from_str(s).map_err(|_| "Respuesta JSON de Whakoom inválida".into())
    } else {
        Ok(v.clone())
    }
}

pub fn parse_profile(html: &str) -> Option<String> {
    let h = Html::parse_document(html);
    let name = attr(&h, "#user-avatar img", "alt");
    if !name.is_empty() {
        return Some(name);
    }
    None
}

#[derive(Clone)]
pub struct Api {
    client: Client,
    cookie: String,
    jar: std::sync::Arc<reqwest::cookie::Jar>,
}
impl Api {
    pub fn identity(&self) -> Result<crate::social::User, String> {
        crate::social::identity(&self.request("/", None)?)
    }
    pub fn user_profile(&self, username: &str) -> Result<crate::social::User, String> {
        crate::social::profile(
            &self.request(&crate::social::user_path(username)?, None)?,
            username,
        )
    }
    pub fn friends(&self, username: &str) -> Result<Vec<crate::social::User>, String> {
        self.connections(username, crate::social::Relation::Following)
    }
    pub fn connections(
        &self,
        username: &str,
        relation: crate::social::Relation,
    ) -> Result<Vec<crate::social::User>, String> {
        let path = format!(
            "{}/{}",
            crate::social::user_path(username)?,
            relation.path()
        );
        let html = self.request(&path, None)?;
        let h = Html::parse_document(&html);
        let cursor = attr(&h, "#hdNextPage", "value");
        let mut result = crate::social::friends(&html);
        if cursor.is_empty() {
            return Ok(result);
        }
        let mode: u32 = attr(&h, "#hdMode", "value")
            .parse()
            .map_err(|_| "Cambió el formato de paginación de amigos")?;
        let mut page: u32 = cursor.parse().map_err(|_| "Página de amigos inválida")?;
        let mut seen: HashSet<_> = result.iter().map(|u| u.username.clone()).collect();
        let mut cursors = HashSet::new();
        for _ in 0..500 {
            if !cursors.insert(page) {
                return Err("Whakoom repitió una página de amigos".into());
            }
            let raw = self.request_referred(
                "/pwkws.asmx/PProfileFollowPage",
                Some(json!({"m":mode,"p":page})),
                Some(&path),
            )?;
            let data = unwrap_response(
                serde_json::from_str(&raw).map_err(|_| "Respuesta de amigos inválida")?,
            )?;
            let html = if let Some(html) = data.get("Html").and_then(Value::as_str) {
                html.to_owned()
            } else if let Some(html) = data.get("Html").and_then(Value::as_array) {
                html.iter()
                    .filter_map(Value::as_str)
                    .collect::<Vec<_>>()
                    .join("")
            } else {
                return Err("Whakoom no devolvió la página de amigos".into());
            };
            let users = crate::social::friends(&format!("<ul class='users-list'>{html}</ul>"));
            if users.is_empty() {
                return Ok(result);
            }
            let previous = seen.len();
            result.extend(
                users
                    .into_iter()
                    .filter(|u| seen.insert(u.username.clone())),
            );
            if seen.len() == previous {
                return Err("Whakoom repitió la lista de amigos".into());
            }
            let next = data
                .get("ExtraInfo")
                .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()));
            match next {
                Some(next) if next > 0 && next <= u32::MAX as u64 => page = next as u32,
                _ => return Ok(result),
            }
        }
        Err("Se alcanzó el límite de páginas de amigos".into())
    }
    pub fn apply(&self, pending: &crate::sync::Pending) -> Result<(), String> {
        use crate::sync::Change;
        match &pending.change {
            Change::EditionOwned(value) => {
                let volumes = crate::catalog::all_volumes(
                    |page| self.edition(&pending.item, page),
                    || false,
                )?;
                let owned = || {
                    crate::sync::pages(|page| self.collection(page, "", false), || false)
                        .map(|items| items.into_iter().map(|item| item.key).collect())
                };
                let initial = owned()?;
                if crate::sync::missing_volumes(&volumes, &initial, *value).is_empty() {
                    return Ok(());
                }
                let mut cached_snapshot = Some(initial);
                // The bulk endpoint may partially succeed or return a numeric ExtraInfo.
                // Its response is never sufficient to discard durable user intent.
                if pending.attempts == 0 {
                    cached_snapshot = None;
                    let id: u64 = pending
                        .item
                        .key
                        .strip_prefix("edicion")
                        .ok_or("No es una edición")?
                        .parse()
                        .map_err(|_| "ID de edición inválido")?;
                    let _ = if *value {
                        self.post("/wkws.asmx/AddAllComics", json!({"e":id}))
                    } else {
                        self.post("/wkws.asmx/RemAllComics", json!({"eid":id}))
                    };
                }
                crate::sync::resume_edition_batch(
                    &volumes,
                    *value,
                    12,
                    || match cached_snapshot.take() {
                        Some(owned) => Ok(owned),
                        None => owned(),
                    },
                    |items| {
                        for chunk in items.chunks(3) {
                            let results = std::thread::scope(|scope| {
                                let requests: Vec<_> = chunk
                                    .iter()
                                    .map(|item| {
                                        scope.spawn(move || {
                                            self.mutate(
                                                &Detail {
                                                    item: (*item).clone(),
                                                    ..Default::default()
                                                },
                                                Action::Owned(*value),
                                            )
                                        })
                                    })
                                    .collect();
                                requests
                                    .into_iter()
                                    .map(|request| {
                                        request.join().unwrap_or_else(|_| {
                                            Err("El conector de un tomo se interrumpió".into())
                                        })
                                    })
                                    .collect::<Vec<_>>()
                            });
                            for result in results {
                                result?;
                            }
                        }
                        Ok(())
                    },
                )
            }
            Change::Owned(value) => self.mutate(
                &Detail {
                    item: pending.item.clone(),
                    ..Default::default()
                },
                Action::Owned(*value),
            ),
            Change::EditionFavorite(value) => {
                let detail = self.detail(&pending.item)?;
                self.mutate(&detail, Action::Wanted(*value))
            }
            Change::Wanted(value) => {
                self.mutate(&self.detail(&pending.item)?, Action::Wanted(*value))
            }
            Change::Rating(value) => {
                if pending.item.key.starts_with("edicion") {
                    let id: u64 = pending
                        .item
                        .key
                        .trim_start_matches("edicion")
                        .parse()
                        .map_err(|_| "ID de serie inválido")?;
                    let result = self.post("/wkws.asmx/EditionRate", json!({"e":id,"rt":value}))?;
                    if result.get("ExtraInfo").and_then(Value::as_str) == Some("1") {
                        Ok(())
                    } else {
                        Err("Whakoom no confirmó la valoración de la serie".into())
                    }
                } else {
                    self.mutate(&self.detail(&pending.item)?, Action::Rating(*value))
                }
            }
            Change::Read { read, date } => {
                let detail = self.detail(&pending.item)?;
                if !read {
                    return self.mutate(&detail, Action::Read(false));
                }
                let compact = date.replace('-', "");
                if !date.is_empty() && crate::storage::reading_month(date).is_none() {
                    return Err(
                        "Usá una fecha válida YYYY-MM-DD para sincronizar la lectura".into(),
                    );
                }
                let result = self.post("/wkws.asmx/ComicRead", json!({"c":detail.numeric_id.ok_or("Falta el ID de lectura")?,"rr":false,"crd":compact,"r":detail.personal_rating}))?;
                if result.get("ExtraInfo").and_then(Value::as_str) == Some("1") {
                    Ok(())
                } else {
                    Err("Whakoom no confirmó la lectura".into())
                }
            }
            Change::Review(draft) => self.publish_review(&self.detail(&pending.item)?, draft),
            Change::Notes(notes) => {
                let cid = pending
                    .item
                    .key
                    .strip_prefix("comic")
                    .ok_or("La cuenta no dispone del editor de notas para esta ficha")?;
                let data = self.post("/wkws.asmx/editmycomic", json!({"cid":cid}))?;
                if data.get("ExtraInfo").and_then(Value::as_str) == Some("0") {
                    return Err("Whakoom no permite editar notas con los permisos actuales de esta cuenta. La nota se conserva en tu PC".into());
                }
                let html = data
                    .get("Html")
                    .and_then(Value::as_str)
                    .ok_or("No se recibió el formulario de notas")?;
                let h = Html::parse_fragment(html);
                if h.select(&sel("#txtNotes")).next().is_none() {
                    return Err("La cuenta no dispone del editor de notas de Whakoom".into());
                }
                let mut grade = attr(&h, "#ddlGrade option[selected]", "value");
                if grade.is_empty() {
                    grade = attr(&h, "#ddlGrade option", "value");
                }
                let place = attr(&h, "#txtComicPlace", "value");
                let hidden = h
                    .select(&sel("#chkPublic"))
                    .next()
                    .is_some_and(|e| e.value().attr("checked").is_none());
                let result = self.post(
                    "/wkws.asmx/updateMyComic",
                    json!({"cid":cid,"idg":grade,"n":notes,"p":place,"h":hidden}),
                )?;
                if result.get("ExtraInfo").and_then(Value::as_str) == Some("1") {
                    Ok(())
                } else {
                    Err("Whakoom no confirmó la nota; se conserva localmente".into())
                }
            }
        }
    }
    pub fn html(&self, path: &str) -> Result<String, String> {
        self.request(path, None)
    }
    pub fn login(
        username: &str,
        password: &str,
    ) -> Result<(Self, crate::session::Session, String), String> {
        let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
        let client = Client::builder()
            .cookie_provider(jar.clone())
            .timeout(Duration::from_secs(25))
            .connect_timeout(Duration::from_secs(8))
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::custom(|a| {
                if a.previous().len() < 5 && safe_url(a.url().as_str()).is_ok() {
                    a.follow()
                } else {
                    a.error("Redirección de login no admitida")
                }
            }))
            .build()
            .map_err(|e| e.to_string())?;
        let login_permit = crate::traffic::before(BASE)?;
        let get = client
            .get(format!("{BASE}/login"))
            .send()
            .map_err(|_| "No se pudo conectar con Whakoom. Revisá tu conexión")?;
        login_permit.check(&get)?;
        if !get.status().is_success() {
            return Err("Whakoom requiere una verificación del acceso. Usá Verificar acceso en esta ventana".into());
        }
        let raw = limited_response(get)?;
        drop(login_permit);
        let token = login_token(&raw)?;
        let login_permit = crate::traffic::before(BASE)?;
        let response = client
            .post(format!("{BASE}/login"))
            .header(header::REFERER, format!("{BASE}/login"))
            .header(header::ORIGIN, BASE)
            .form(&[
                ("username", username),
                ("userpassw", password),
                ("remember", "true"),
                ("__RequestVerificationToken", token.as_str()),
                ("dologin2", ""),
            ])
            .send()
            .map_err(|_| "No se pudo enviar el login. Probá de nuevo")?;
        login_permit.check(&response)?;
        if !response.status().is_success() {
            return Err(
                "Whakoom rechazó el acceso. Comprobá las credenciales o usá Verificar acceso"
                    .into(),
            );
        }
        let html = limited_response(response)?;
        drop(login_permit);
        let name=parse_profile(&html).ok_or_else(||{
            let h=Html::parse_document(&html);let msg=first(&h,".validation-summary-errors, .field-validation-error, .login-error");
            if msg.is_empty(){"No se pudo iniciar sesión. Revisá las credenciales; si son correctas, usá Verificar acceso".into()}else{msg.chars().take(240).collect::<String>()}
        })?;
        let cookie = jar
            .cookies(&url::Url::parse(BASE).unwrap())
            .and_then(|v| v.to_str().ok().map(str::to_owned))
            .ok_or("Whakoom no devolvió una sesión")?;
        let session = crate::session::Session {
            cookie: cookie.clone(),
            user_agent: USER_AGENT.into(),
            username: name.clone(),
        };
        Ok((Self::with_user_agent(cookie, USER_AGENT)?, session, name))
    }
    pub fn new(cookie: String) -> Result<Self, String> {
        Self::with_user_agent(cookie, USER_AGENT)
    }
    pub fn with_user_agent(cookie: String, user_agent: &str) -> Result<Self, String> {
        let jar = std::sync::Arc::new(reqwest::cookie::Jar::default());
        let base = url::Url::parse(BASE).unwrap();
        for pair in cookie.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            jar.add_cookie_str(&format!("{pair}; Path=/; Secure"), &base);
        }
        let client = Client::builder()
            .cookie_provider(jar.clone())
            .timeout(Duration::from_secs(25))
            .user_agent(user_agent)
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 5 {
                    attempt.error("Demasiadas redirecciones")
                } else if safe_url(attempt.url().as_str()).is_err() {
                    attempt.error("Redirección fuera de Whakoom")
                } else {
                    attempt.follow()
                }
            }))
            .build()
            .map_err(|e| e.to_string())?;
        Ok(Self {
            client,
            cookie,
            jar,
        })
    }
    fn request(&self, path: &str, body: Option<Value>) -> Result<String, String> {
        self.request_referred(path, body, None)
    }
    fn request_referred(
        &self,
        path: &str,
        body: Option<Value>,
        referer: Option<&str>,
    ) -> Result<String, String> {
        let url = safe_url(path)?;
        let permit = crate::traffic::before(&url)?;
        let req = if let Some(body) = body {
            self.client
                .post(url)
                .json(&body)
                .header("X-Requested-With", "XMLHttpRequest")
                .header(header::ORIGIN, BASE)
        } else {
            self.client.get(url)
        };
        let response = req
            .header(header::REFERER, safe_url(referer.unwrap_or("/"))?)
            .send()
            .map_err(|e| format!("No se pudo conectar: {e}"))?;
        permit.check(&response)?;
        let status = response.status();
        if status.as_u16() == 401 {
            return Err(
                "Whakoom requiere una sesión válida. Iniciá sesión o reconectá tu cuenta".into(),
            );
        }
        if status.as_u16() == 403 {
            return Err("Whakoom rechazó esta consulta (403). Probá reconectar la cuenta desde el login oficial".into());
        }
        if !status.is_success() {
            return Err(format!("Whakoom respondió HTTP {status}"));
        }
        if response.url().path() == "/login" && path != "/login" {
            return Err("Tu sesión expiró. Volvé a iniciar sesión".into());
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_BODY + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BODY {
            return Err("Respuesta demasiado grande".into());
        }
        String::from_utf8(bytes).map_err(|_| "Respuesta no válida en UTF-8".into())
    }
    pub fn post(&self, path: &str, body: Value) -> Result<Value, String> {
        let raw = self.request(path, Some(body))?;
        unwrap_response(serde_json::from_str(&raw).map_err(|_| {
            "Whakoom devolvió HTML en lugar de datos; reconectá tu sesión".to_owned()
        })?)
    }
    pub fn post_referred(&self, path: &str, body: Value, referer: &str) -> Result<Value, String> {
        let raw = self.request_referred(path, Some(body), Some(referer))?;
        unwrap_response(serde_json::from_str(&raw).map_err(|_| "Respuesta de perfil inválida")?)
    }
    pub fn profile(&self) -> Result<String, String> {
        parse_profile(&self.request("/", None)?).ok_or_else(|| "Todavía no hay una sesión autenticada. Completá el login oficial y pulsá Conectar sesión".into())
    }
    pub fn news(&self, month: &str) -> Result<Page, String> {
        if !month.is_empty()
            && month != "upcoming"
            && !(month.len() == 6 && month.chars().all(|c| c.is_ascii_digit()))
        {
            return Err("Usá un mes YYYYMM o upcoming".into());
        }
        let path = if month.is_empty() {
            "/newtitles".into()
        } else {
            format!("/newtitles/{month}")
        };
        let items = parse_items(&self.request(&path, None)?);
        if items.is_empty() {
            return Err("No se encontraron novedades en esta página".into());
        }
        Ok(Page { items, next: None })
    }
    pub fn search(&self, query: &str, page: u32) -> Result<Page, String> {
        let data = self.post(
            "/search.aspx/Query",
            json!({"q":query,"ft":0,"fit":"","fp":"","fl":"","p":page}),
        )?;
        let html = data
            .get("searchResult")
            .and_then(Value::as_str)
            .ok_or("Cambió el formato de búsqueda de Whakoom")?;
        let next = data
            .get("nextPage")
            .and_then(Value::as_u64)
            .filter(|n| *n > 0)
            .map(|n| n as u32);
        Ok(Page {
            items: parse_items(html),
            next,
        })
    }
    pub fn collection(&self, page: u32, query: &str, wishlist: bool) -> Result<Page, String> {
        let (path, body) = if wishlist {
            ("/mywishlist.aspx/List", json!({"p":page,"o":0}))
        } else {
            (
                "/mycollection/comics.aspx/List",
                json!({"s":query,"pub":"","au":"","m":0,"r":0,"wr":0,"idc":-1,"idp":-1,"nt":"","co":0,"p":page,"lm":true}),
            )
        };
        let data = self.post(path, body)?;
        let end = data
            .get("ExtraInfo")
            .is_some_and(|value| value.as_str() == Some("0") || value.as_u64() == Some(0));
        let html = data
            .get("Html")
            .and_then(Value::as_str)
            .or_else(|| end.then_some(""))
            .ok_or("Cambió el formato de tu colección")?;
        if html.is_empty()
            && data
                .get("C")
                .and_then(Value::as_array)
                .is_some_and(|a| !a.is_empty())
        {
            return Err("Whakoom devolvió el modo galería. Abrí tu comicteca en la web y seleccioná vista de lista".into());
        }
        let items = parse_items(html);
        // List endpoints do not all supply nextPage. An empty next page ends pagination.
        let next = if items.is_empty()
            || end
            || (wishlist
                && data
                    .get("ExtraInfo")
                    .is_some_and(|v| v.as_str() == Some("2") || v.as_u64() == Some(2)))
        {
            None
        } else {
            Some(page + 1)
        };
        let items = if wishlist && !query.is_empty() {
            let query = query.to_lowercase();
            items
                .into_iter()
                .filter(|item| item.title.to_lowercase().contains(&query))
                .collect()
        } else {
            items
        };
        Ok(Page { items, next })
    }
    pub fn detail(&self, item: &Item) -> Result<Detail, String> {
        if !self.cookie.is_empty() && item.key.starts_with("comic") {
            let data = self.post("/pwkws.asmx/QuickView", json!({"cid":item.key}))?;
            if let Some(html) = data
                .get("Html")
                .and_then(Value::as_str)
                .filter(|h| !h.is_empty())
            {
                return parse_detail(html, item);
            }
        }
        parse_detail(&self.request(&item.url, None)?, item)
    }
    pub fn full_detail(&self, item: &Item) -> Result<Detail, String> {
        let mut full = parse_detail(&self.request(&item.url, None)?, item)?;
        // Full pages can omit account-only panels that QuickView supplies.
        if !self.cookie.is_empty() && item.key.starts_with("comic") {
            let personal = self.detail(item)?;
            full.numeric_id = personal.numeric_id.or(full.numeric_id);
            full.personal_rating = personal.personal_rating;
            full.item.owned = personal.item.owned;
            full.wanted = personal.wanted;
            full.read = personal.read;
            full.read_date = personal.read_date;
        }
        Ok(full)
    }
    pub fn discussion_page(
        &self,
        item: &Item,
        numeric_id: Option<u64>,
        page: u32,
    ) -> Result<crate::discussion::Discussion, String> {
        let (path, body) = if let Some(id) = item
            .key
            .strip_prefix("edicion")
            .and_then(|s| s.parse::<u64>().ok())
        {
            ("/pwkws.asmx/EditionCommentPage", json!({"p":page,"s":id}))
        } else {
            (
                "/pwkws.asmx/ComicCommentPage",
                json!({"p":page,"c":numeric_id.ok_or("Falta el identificador del tomo")?}),
            )
        };
        let response = self.post(path, body)?;
        let html = response
            .get("Html")
            .and_then(Value::as_str)
            .ok_or("Respuesta de opiniones inválida")?;
        let mut result = crate::discussion::parse(html);
        result.next = response
            .get("ExtraInfo")
            .and_then(|v| {
                v.as_str()
                    .and_then(|s| s.parse().ok())
                    .or_else(|| v.as_u64().and_then(|n| u32::try_from(n).ok()))
            })
            .filter(|n| *n > page);
        Ok(result)
    }
    pub fn suggestion_form(
        &self,
        request: crate::contributions::Request,
    ) -> Result<crate::contributions::Suggestion, String> {
        use scraper::{Html, Selector};
        if self.cookie.is_empty() {
            return Err("Conectá tu cuenta para sugerir cambios".into());
        }
        let item = request
            .item
            .as_ref()
            .ok_or("Falta la ficha de la sugerencia")?;
        let validated =
            crate::contributions::Request::for_item(crate::contributions::Action::Suggest, item)?;
        if request.action != crate::contributions::Action::Suggest || request.url != validated.url {
            return Err("La sugerencia no corresponde a la ficha seleccionada".into());
        }
        let html = self.request(&request.url, None)?;
        let document = Html::parse_document(&html);
        let bid = document
            .select(&Selector::parse("a.show-bug-report, button.add-bug-report").unwrap())
            .find_map(|a| a.value().attr("data-item-id"))
            .filter(|s| {
                !s.is_empty()
                    && s.len() <= 128
                    && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
            })
            .ok_or("Whakoom no permite sugerencias en esta ficha o solicita iniciar sesión")?;
        let response = self.post_referred(
            "/wkws.asmx/br",
            serde_json::json!({"bid":bid,"tid":null}),
            &request.url,
        )?;
        crate::contributions::Suggestion::parse(request, response["Html"].as_str().unwrap_or(""))
    }
    pub fn submit_suggestion(&self, form: &crate::contributions::Suggestion) -> Result<(), String> {
        form.body()?;
        // Revalidate server identity and permissions immediately before submitting.
        let fresh = self.suggestion_form(form.request.clone())?;
        if fresh.id != form.id
            || fresh.kind != form.kind
            || !fresh.types.iter().any(|(id, _)| id == &form.selected)
        {
            return Err("La ficha cambió. Volvé a abrir el formulario antes de enviar".into());
        }
        let response = self.post_referred(
            &format!("/wkws.asmx/pb{}", fresh.kind),
            form.body()?,
            &form.request.url,
        )?;
        if matches!(response["ExtraInfo"].as_str(), Some("1" | "2")) {
            Ok(())
        } else {
            Err(response["Title"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or("Whakoom no confirmó la sugerencia")
                .into())
        }
    }
    pub fn account_page(
        &self,
        section: crate::account::Section,
    ) -> Result<crate::account::Page, String> {
        if self.cookie.is_empty() {
            return Err("Conectá tu cuenta para consultar esta sección".into());
        }
        crate::account::parse(&self.request(section.path(), None)?, section)
    }
    fn account_response(
        &self,
        request: reqwest::blocking::RequestBuilder,
    ) -> Result<String, String> {
        let permit = crate::traffic::before(BASE)?;
        let response = request
            .send()
            .map_err(|_| "No se pudo guardar en Whakoom. Revisá tu conexión")?;
        permit.check(&response)?;
        if !response.status().is_success() {
            return Err(format!("Whakoom respondió HTTP {}", response.status()));
        }
        if response.url().path() == "/login" {
            return Err("Whakoom solicita volver a iniciar sesión para confirmar el cambio".into());
        }
        let mut bytes = Vec::new();
        response
            .take(MAX_BODY + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_BODY {
            return Err("Respuesta demasiado grande".into());
        }
        let html = String::from_utf8(bytes).map_err(|_| "Respuesta inválida".to_owned())?;
        if let Some(error) = crate::account::errors(&html) {
            return Err(error);
        }
        Ok(html)
    }
    pub fn save_account(
        &self,
        submission: &crate::account::Submission,
    ) -> Result<crate::account::Page, String> {
        let path = submission.section.path();
        let original = self.request(path, None)?;
        let before = crate::account::parse(&original, submission.section)?;
        let token = crate::account::token(&original, path)?;
        let mut fields = crate::account::post_fields(submission, token);
        let request = self
            .client
            .post(safe_url(path)?)
            .header(header::ORIGIN, BASE)
            .header(header::REFERER, safe_url(path)?)
            .form(&fields);
        // Erase temporary password copies as soon as the encoded request exists.
        use zeroize::Zeroize;
        for (name, value) in &mut fields {
            if name.contains("password") {
                value.zeroize();
            }
        }
        let returned = self.account_response(request)?;
        if let Some(error) = crate::account::errors(&returned) {
            return Err(error);
        }
        let saved = self.account_page(submission.section)?;
        for name in submission
            .section
            .fields()
            .iter()
            .filter(|n| !n.contains("password") && **n != "code")
        {
            if !crate::account::values_match(
                name,
                saved.values.get(*name),
                submission.values.get(*name),
            ) {
                return Err("Whakoom no confirmó todos los cambios. Revisá los datos y tu contraseña actual".into());
            }
        }
        if submission.section == crate::account::Section::Subscription {
            // The code field alone is not a success indicator: failed redemption can clear it too.
            let h = Html::parse_document(&returned);
            let confirmed = h
                .select(&sel(".success, .alert-success, .message-success"))
                .map(text)
                .any(|t| !t.is_empty())
                || saved.subscription != before.subscription
                || saved.renewal_date != before.renewal_date;
            if !confirmed {
                return Err("No se pudo confirmar el canje. Consultá el estado de tu suscripción antes de reintentar".into());
            }
        }
        Ok(saved)
    }
    pub fn upload_avatar(&self, path: &std::path::Path) -> Result<crate::account::Page, String> {
        let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
        if size > 5 * 1024 * 1024 {
            return Err("La imagen debe pesar menos de 5 MiB".into());
        }
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let format =
            image::guess_format(&bytes).map_err(|_| "Elegí una imagen PNG, JPEG o WebP")?;
        let (ext, mime) = match format {
            image::ImageFormat::Png => ("png", "image/png"),
            image::ImageFormat::Jpeg => ("jpg", "image/jpeg"),
            image::ImageFormat::WebP => ("webp", "image/webp"),
            _ => return Err("Formato de imagen no admitido".into()),
        };
        let mut reader = image::ImageReader::with_format(std::io::Cursor::new(&bytes), format);
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(4096);
        limits.max_image_height = Some(4096);
        limits.max_alloc = Some(64 * 1024 * 1024);
        reader.limits(limits);
        reader
            .decode()
            .map_err(|_| "No se pudo leer la imagen (máximo 4096 × 4096)")?;
        let path = "/micuenta/miperfil.aspx/newavatar";
        let html = self.request(crate::account::Section::Profile.path(), None)?;
        let before = crate::account::parse(&html, crate::account::Section::Profile)?.avatar;
        let token = crate::account::token(&html, path)?;
        let part = reqwest::blocking::multipart::Part::bytes(bytes)
            .file_name(format!("avatar.{ext}"))
            .mime_str(mime)
            .map_err(|e| e.to_string())?;
        let form = reqwest::blocking::multipart::Form::new()
            .text("__RequestVerificationToken", token)
            .part("fuavatar", part);
        self.account_response(
            self.client
                .post(safe_url(path)?)
                .header(header::ORIGIN, BASE)
                .header(
                    header::REFERER,
                    safe_url(crate::account::Section::Profile.path())?,
                )
                .multipart(form),
        )?;
        let after = self.account_page(crate::account::Section::Profile)?;
        if after.avatar == before {
            return Err("Whakoom no confirmó una nueva foto de perfil".into());
        }
        Ok(after)
    }
    pub fn unblock(&self, id: &str) -> Result<crate::account::Page, String> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_digit()) {
            return Err("Usuario bloqueado inválido".into());
        }
        self.post("/micuenta/blockedusers.aspx/Unblock", json!({"u":id}))?;
        let page = self.account_page(crate::account::Section::Blocked)?;
        if page.blocked.iter().any(|(u, _)| u == id) {
            return Err("Whakoom no confirmó el desbloqueo".into());
        }
        Ok(page)
    }
    pub fn cancel_subscription(&self) -> Result<crate::account::Page, String> {
        let before = self.account_page(crate::account::Section::Subscription)?;
        if !before.can_cancel {
            return Err("Tu suscripción no tiene una renovación pendiente para cancelar".into());
        }
        self.post(
            "/micuenta/mysubscription.aspx/CancelSubscription",
            json!({}),
        )?;
        let after = self.account_page(crate::account::Section::Subscription)?;
        if after.can_cancel {
            return Err("Whakoom no confirmó la cancelación de la renovación".into());
        }
        Ok(after)
    }
    pub fn persist_account_session(&self, username: &str) -> Result<(), String> {
        if let Some(mut saved) = crate::session::load() {
            saved.username = username.into();
            saved.cookie = self
                .jar
                .cookies(&url::Url::parse(BASE).unwrap())
                .and_then(|h| h.to_str().ok().map(str::to_owned))
                .ok_or("La sesión perdió sus cookies")?;
            crate::session::save(&saved)?;
        }
        Ok(())
    }
    pub fn edition(&self, item: &Item, page: u32) -> Result<Page, String> {
        let id: u64 = item
            .key
            .strip_prefix("edicion")
            .ok_or("No es una edición")?
            .parse()
            .map_err(|_| "ID de edición inválido")?;
        let data = self.post(
            "/pwkws.asmx/EditionComicsPage",
            json!({"e":id,"p":page,"m":1,"o":0}),
        )?;
        let html = data
            .get("Html")
            .and_then(Value::as_str)
            .ok_or("Respuesta de edición inválida")?;
        let mut items = parse_items(html);
        for volume in &mut items {
            if volume.publisher.is_empty() {
                volume.publisher.clone_from(&item.publisher);
            }
        }
        let next =
            if !items.is_empty() && !data.get("ExtraInfo").is_some_and(|v| v == "0" || v == 0) {
                Some(page + 1)
            } else {
                None
            };
        Ok(Page { items, next })
    }
    pub fn mutate(&self, detail: &Detail, action: Action) -> Result<(), String> {
        if self.cookie.is_empty() {
            return Err("Iniciá sesión para actualizar tu cuenta".into());
        }
        let id = detail.numeric_id;
        let (path, body) = match action {
            Action::Owned(value) => (
                "/wkws.asmx/arcm",
                json!({"c":detail.item.key,"a":value,"fe":false,"em":0}),
            ),
            Action::Wanted(value) => (
                if value {
                    "/wkws.asmx/AddWishList"
                } else {
                    "/wkws.asmx/RemoveWishList"
                },
                json!({"t":detail.wish_kind,"i":detail.wish_id}),
            ),
            Action::Read(value) => (
                if value {
                    "/wkws.asmx/ComicRead"
                } else {
                    "/wkws.asmx/ComicUnRead"
                },
                if value {
                    json!({"c":id.ok_or("No se encontró el ID numérico")?,"rr":false,"crd":"","r":detail.personal_rating})
                } else {
                    json!({"c":id.ok_or("No se encontró el ID numérico")?})
                },
            ),
            Action::Rating(value) => (
                "/wkws.asmx/ComicRate",
                json!({"c":id.ok_or("No se encontró el ID numérico")?,"r":value}),
            ),
        };
        let data = self.post(path, body)?;
        if let Action::Wanted(value) = action {
            return if self.detail(&detail.item)?.wanted == value {
                Ok(())
            } else {
                Err("Whakoom no confirmó el cambio en Lo quiero".into())
            };
        }

        // Current arcm returns RCode=1 for an unchanged state and an unhelpful GotIt.
        // Confirm the actual account state before accepting an idempotent retry.
        if let Action::Owned(value) = action
            && data.get("RCode").and_then(Value::as_i64) == Some(1)
            && self.detail(&detail.item)?.item.owned == value
        {
            return Ok(());
        }
        let success = match action {
            Action::Owned(v) => {
                data.get("RCode").and_then(Value::as_i64) == Some(0)
                    && data.get("GotIt").and_then(Value::as_bool) == Some(v)
            }
            Action::Rating(v) => {
                data.get("ExtraInfo")
                    .and_then(Value::as_str)
                    .and_then(|s| s.parse::<u8>().ok())
                    == Some(v)
            }
            _ => data.get("ExtraInfo").and_then(Value::as_str) == Some("1"),
        };
        if success {
            Ok(())
        } else {
            Err(data
                .get("Message")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())
                .unwrap_or("Whakoom no confirmó el cambio; se conserva pendiente en tu PC")
                .into())
        }
    }
}

pub fn login_token(html: &str) -> Result<String, String> {
    let h = Html::parse_document(html);
    let token = attr(&h, "form input[name='__RequestVerificationToken']", "value");
    if token.is_empty() {
        Err(
            "El acceso necesita verificación. Usá Verificar acceso para completar el login oficial"
                .into(),
        )
    } else {
        Ok(token)
    }
}
fn limited_response(response: reqwest::blocking::Response) -> Result<String, String> {
    let mut bytes = vec![];
    response
        .take(MAX_BODY + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BODY {
        return Err("Respuesta demasiado grande".into());
    }
    String::from_utf8(bytes).map_err(|_| "Respuesta de login inválida".into())
}

#[derive(Clone, Copy, Debug)]
pub enum Action {
    Owned(bool),
    Wanted(bool),
    Read(bool),
    Rating(u8),
}

pub fn fetch_cover(url: &str) -> Result<image::RgbaImage, String> {
    crate::covers::CoverClient::new()?.get(url, false)
}

#[cfg(test)]
mod metadata_tests {
    use super::*;
    #[test]
    fn isbn_and_owner_counts_follow_the_live_markup_without_confusing_rating_votes() {
        let item = Item {
            key: "comicABC".into(),
            url: format!("{BASE}/comics/ABC/demo"),
            ..Default::default()
        };
        let detail=parse_detail("<div class='b-info'><h1>Demo</h1><span class='rate-count'>99</span></div><div class='alsohavethis'><h2><a><span>13.480</span> personas</a></h2></div><ul class='barcodes'><li itemprop='isbn'>978-6-076-36085-9</li><li itemprop='isbn'>978-8-411-01427-4</li></ul>",&item).unwrap();
        assert_eq!(detail.owners, Some(13480));
        assert_eq!(detail.isbn.len(), 2);
        assert_eq!(detail.discussion.votes, "99");
        let unknown = parse_detail("<div class='b-info'><h1>Demo</h1></div>", &item).unwrap();
        assert!(unknown.owners.is_none());
        assert!(unknown.isbn.is_empty());
    }
}

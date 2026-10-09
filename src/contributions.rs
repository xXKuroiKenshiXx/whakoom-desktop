//! Entry points into Whakoom's official collaborative catalogue forms.
use crate::api::{self, Item};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Create,
    Edit,
    Suggest,
    AddVolume,
}
impl Action {
    pub fn title(self) -> &'static str {
        match self {
            Self::Create => "Crear cómic o colección",
            Self::Edit => "Modificar ficha",
            Self::Suggest => "Sugerir un cambio",
            Self::AddVolume => "Añadir tomos a la serie",
        }
    }
    pub fn instructions(self) -> &'static str {
        match self {
            Self::Create => {
                "Comprobá que no exista antes de crear una ficha. Whakoom te pedirá título, idioma, editorial y números publicados."
            }
            Self::Edit => {
                "El editor oficial se abre aquí. Los campos disponibles dependen de la edición y de los permisos de tu cuenta."
            }
            Self::Suggest => {
                "Elegí el tipo de error y explicá la corrección. Tu sugerencia se envía al equipo de Whakoom desde su formulario."
            }
            Self::AddVolume => {
                "Usá Añadir números o Crear el siguiente número en la serie original. Comprobá la numeración antes de guardar."
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Request {
    pub action: Action,
    pub url: String,
    pub item: Option<Item>,
}
impl Request {
    pub fn create(title: &str) -> Result<Self, String> {
        if title.len() > 500 || title.chars().any(char::is_control) {
            return Err("Título de búsqueda inválido".into());
        }
        let mut url = url::Url::parse(&api::safe_url("/newedition")?).unwrap();
        if !title.trim().is_empty() {
            url.query_pairs_mut().append_pair("s", title.trim());
        }
        Ok(Self {
            action: Action::Create,
            url: url.into(),
            item: None,
        })
    }
    pub fn for_item(action: Action, item: &Item) -> Result<Self, String> {
        let url = api::safe_url(&item.url)?;
        let parsed = url::Url::parse(&url).unwrap();
        if action == Action::Create
            || api::key_from_url(&url).as_deref() != Some(&item.key)
            || parsed.query().is_some()
            || parsed.fragment().is_some()
            || action == Action::AddVolume && !item.key.starts_with("edicion")
        {
            return Err("Ficha inválida para colaborar".into());
        }
        Ok(Self {
            action,
            url,
            item: Some(item.clone()),
        })
    }
    /// Only opens an official menu or dialog; never writes or submits any data.
    pub fn opening_script(&self) -> String {
        let path = serde_json::to_string(url::Url::parse(&self.url).unwrap().path()).unwrap();
        let selector = match self.action {
            Action::Suggest => "a.show-bug-report, button.add-bug-report",
            Action::AddVolume => "a.create-next-issue, button.add-issues",
            _ => "",
        };
        let editing = self.action == Action::Edit;
        let creating = self.action == Action::Create;
        format!(
            r#"(()=>{{
            if(window.top!==window || location.origin!=='https://www.whakoom.com')return;
            const style=document.createElement('style');
            style.textContent='#header,#topHeader,#wrapper-header,#footer,.ad-container,.adsbygoogle{{display:none!important}} body{{padding-top:0!important}} #wrapper,#wrapperBody,#content{{margin-top:0!important}}';
            let attempts=0;
            const timer=setInterval(()=>{{
                if(document.head && !style.isConnected)document.head.append(style);
                if(location.pathname!=={path} || {creating}){{clearInterval(timer);return;}}
                let target='{selector}' ? document.querySelector('{selector}') : null;
                if({editing}) target=[...document.querySelectorAll('a,button')].find(e=>/^(modificar( ficha| datos)?|editar( ficha)?|modify|edit)$/i.test(e.textContent.trim()));
                if(target){{clearInterval(timer);target.click();return;}}
                if(++attempts>=40){{
                    clearInterval(timer);
                    const message=document.createElement('p');
                    message.textContent='Whakoom no habilitó este formulario para tu cuenta. Podés volver y usar Sugerir un cambio; no se ha enviado ninguna modificación.';
                    message.style.cssText='padding:20px;margin:16px;background:#172532;color:#f5f7fa;font:16px sans-serif;border-radius:12px';
                    (document.querySelector('#content')||document.body)?.prepend(message);
                }}
            }},250);
        }})();"#
        )
    }
}

/// Parse saved Cookie header pairs, without permitting injected cookie attributes.
pub fn cookie_pairs(header: &str) -> Result<Vec<(&str, &str)>, String> {
    if header.len() > 64 * 1024 || header.chars().any(char::is_control) {
        return Err("Sesión inválida".into());
    }
    header
        .split(';')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (name, value) = pair.split_once('=').ok_or("Cookie inválida")?;
            if name.is_empty()
                || !name
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&c))
                || value
                    .bytes()
                    .any(|c| c.is_ascii_whitespace() || c == b'"' || c == b'\\')
            {
                return Err("Cookie inválida".into());
            }
            Ok((name, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn targets_are_encoded_and_match_the_selected_item() {
        let r = Request::create("A & B / #1 日本語").unwrap();
        assert_eq!(
            url::Url::parse(&r.url)
                .unwrap()
                .query_pairs()
                .next()
                .unwrap()
                .1,
            "A & B / #1 日本語"
        );
        let item = Item {
            key: "edicion123".into(),
            url: "https://www.whakoom.com/ediciones/123/serie".into(),
            ..Default::default()
        };
        assert!(Request::for_item(Action::Edit, &item).is_ok());
        assert!(Request::for_item(Action::AddVolume, &item).is_ok());
        let wrong = Item {
            key: "edicion456".into(),
            ..item.clone()
        };
        assert!(Request::for_item(Action::Suggest, &wrong).is_err());
        for url in [
            "https://evil.example/ediciones/123/serie",
            "https://www.whakoom.com/ediciones/123/serie?redirect=bad",
            "https://www.whakoom.com/ediciones/123/serie#bad",
        ] {
            assert!(
                Request::for_item(
                    Action::Edit,
                    &Item {
                        url: url.into(),
                        ..item.clone()
                    }
                )
                .is_err()
            );
        }
        let comic = Item {
            key: "comicabc".into(),
            url: "/comics/abc/serie/1".into(),
            ..Default::default()
        };
        assert!(Request::for_item(Action::AddVolume, &comic).is_err());
        assert!(
            Request::for_item(Action::Suggest, &comic)
                .unwrap()
                .opening_script()
                .contains("a.show-bug-report")
        );
    }
    #[test]
    fn cookies_cannot_inject_attributes_or_headers() {
        assert_eq!(
            cookie_pairs("Session=abc==; cf_clearance=token").unwrap(),
            vec![("Session", "abc=="), ("cf_clearance", "token")]
        );
        for value in [
            "Session=x\r\nHost: evil",
            "Session=x; Domain=evil example",
            "bad name=x",
            "Secure",
            "Session=\"injected\"",
            "Session=x\\y",
        ] {
            assert!(cookie_pairs(value).is_err());
        }
    }
}

/// An ephemeral correction form. Identity and report types come from the server,
/// never from a user-supplied endpoint. Drafts are not stored with account data.
#[derive(Clone, Debug)]
pub struct Suggestion {
    pub request: Request,
    pub id: u64,
    pub kind: String,
    pub types: Vec<(String, String)>,
    pub selected: String,
    pub comment: String,
    pub extra: String,
}
impl Suggestion {
    pub fn parse(request: Request, html: &str) -> Result<Self, String> {
        use scraper::{Html, Selector};
        let document = Html::parse_fragment(html);
        let root = document
            .select(&Selector::parse("#bugReport").unwrap())
            .next()
            .ok_or(
                "Whakoom no permite sugerencias en esta ficha. Revisá la conexión de tu cuenta",
            )?;
        let id = root
            .value()
            .attr("data-item-id")
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|id| *id > 0)
            .ok_or("Identificador de sugerencia inválido")?;
        let kind = root.value().attr("data-item-type").unwrap_or("");
        if !matches!(kind, "e" | "c") {
            return Err("Tipo de sugerencia no reconocido".into());
        }
        let types: Vec<_> = root
            .select(&Selector::parse(".bug-type a[href]").unwrap())
            .filter_map(|a| {
                let value = a.value().attr("href")?.strip_prefix("#br-")?;
                if value.is_empty() || value.len() > 8 || !value.bytes().all(|b| b.is_ascii_digit())
                {
                    return None;
                }
                let title = a.text().collect::<Vec<_>>().join(" ").trim().to_owned();
                (!title.is_empty()).then(|| (value.to_owned(), title))
            })
            .collect();
        if types.is_empty() {
            return Err("Whakoom no devolvió tipos de corrección disponibles".into());
        }
        Ok(Self {
            request,
            id,
            kind: kind.into(),
            selected: types[0].0.clone(),
            types,
            comment: String::new(),
            extra: String::new(),
        })
    }
    pub fn body(&self) -> Result<serde_json::Value, String> {
        if self.comment.trim().is_empty()
            || self.comment.len() > 10000
            || self.extra.len() > 2000
            || !self.types.iter().any(|(id, _)| id == &self.selected)
            || !matches!(self.kind.as_str(), "e" | "c")
            || self.id == 0
        {
            return Err(
                "Elegí una corrección y escribí una explicación de hasta 10.000 caracteres".into(),
            );
        }
        Ok(
            serde_json::json!({"biid":self.id,"bt":self.selected,"bc":self.comment.trim(),"ed":self.extra.trim(),"bpd":null}),
        )
    }
}

#[cfg(test)]
mod suggestion_tests {
    use super::*;
    fn request() -> Request {
        Request::for_item(
            Action::Suggest,
            &Item {
                key: "edicion123".into(),
                url: "/ediciones/123/title".into(),
                ..Default::default()
            },
        )
        .unwrap()
    }
    #[test]
    fn server_types_and_submission_are_validated() {
        let html = r##"<div id="bugReport" data-item-id="123" data-item-type="e"><div class="bug-type"><a href="#br-2">Título incorrecto</a><a href="https://evil.test">No permitido</a></div></div>"##;
        let mut form = Suggestion::parse(request(), html).unwrap();
        assert_eq!(form.types.len(), 1);
        assert!(form.body().is_err());
        form.comment = " Corregir la tilde 日本語 ".into();
        assert_eq!(form.body().unwrap()["bc"], "Corregir la tilde 日本語");
        form.selected = "999".into();
        assert!(form.body().is_err());
        assert!(
            Suggestion::parse(request(), &html.replace("type=\"e\"", "type=\"../../bad\""))
                .is_err()
        );
        assert!(Suggestion::parse(request(), "<p>Inicia sesión</p>").is_err());
    }
}

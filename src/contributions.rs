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
                "En la ficha original, abrí Modificar desde el menú de opciones. Los campos disponibles dependen de la edición y de tus permisos."
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
        if self.action == Action::Create {
            return String::new();
        }
        let path = url::Url::parse(&self.url).unwrap().path().to_string();
        let path = serde_json::to_string(&path).unwrap();
        let selector = match self.action {
            Action::Suggest => "a.show-bug-report, button.add-bug-report",
            Action::AddVolume => "a.create-next-issue, button.add-issues",
            _ => "",
        };
        let editing = self.action == Action::Edit;
        format!(
            r#"(()=>{{
            if(window.top!==window || location.origin!=='https://www.whakoom.com')return;
            let attempts=0;
            const timer=setInterval(()=>{{
                if(++attempts>40){{clearInterval(timer);return;}}
                if(location.pathname!=={path} || !document.querySelector('#user-avatar img'))return;
                let target='{selector}' ? document.querySelector('{selector}') : null;
                if({editing}) target=[...document.querySelectorAll('.mn-opt a, .mn-opt button, .edition-header a, .edition-header button, .menu.edition a, ul.v2-menu a')].find(e=>/^(modificar|editar|modify|edit)$/i.test(e.textContent.trim()));
                if(target){{clearInterval(timer);target.click();}}
            }},500);
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

//! Native controls backed by Whakoom's own authenticated account forms.
//! CSRF tokens and passwords live only in memory and are never included in a Page.
use crate::{api, social};
use scraper::{ElementRef, Html, Selector};
use std::collections::BTreeMap;
use zeroize::{Zeroize, Zeroizing};
#[derive(Clone, Default)]
pub struct Fields(pub BTreeMap<String, String>);
impl std::ops::Deref for Fields {
    type Target = BTreeMap<String, String>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for Fields {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Zeroize for Fields {
    fn zeroize(&mut self) {
        for value in self.0.values_mut() {
            value.zeroize();
        }
        self.0.clear();
    }
}

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub enum Section {
    #[default]
    Profile,
    Account,
    Subscription,
    Notifications,
    Region,
    Privacy,
    Blocked,
}
impl Section {
    pub const ALL: [Self; 7] = [
        Self::Profile,
        Self::Account,
        Self::Subscription,
        Self::Notifications,
        Self::Region,
        Self::Privacy,
        Self::Blocked,
    ];
    pub fn title(self) -> &'static str {
        match self {
            Self::Profile => "Perfil",
            Self::Account => "Conexión y seguridad",
            Self::Subscription => "Suscripción",
            Self::Notifications => "Notificaciones",
            Self::Region => "Idioma y país",
            Self::Privacy => "Privacidad",
            Self::Blocked => "Usuarios bloqueados",
        }
    }
    pub fn path(self) -> &'static str {
        match self {
            Self::Profile => "/micuenta/miperfil.aspx",
            Self::Account => "/micuenta/misdatos.aspx",
            Self::Subscription => "/micuenta/mysubscription.aspx",
            Self::Notifications => "/micuenta/notifications.aspx",
            Self::Region => "/micuenta/languageandregion.aspx",
            Self::Privacy => "/micuenta/privacy.aspx",
            Self::Blocked => "/micuenta/blockedusers.aspx",
        }
    }
    pub fn fields(self) -> &'static [&'static str] {
        match self {
            Self::Profile => &["name", "bio"],
            Self::Account => &[
                "nickname",
                "email",
                "newpassword",
                "newpassword2",
                "password",
            ],
            Self::Subscription => &["code"],
            Self::Notifications => &[
                "offers",
                "mynews",
                "newfollower",
                "newcomment",
                "friendsactivity",
                "whakoomnews",
                "tips",
            ],
            Self::Region => &["uilang", "country", "languages"],
            Self::Privacy => &["privateaccount", "privatecollection"],
            Self::Blocked => &[],
        }
    }
}
#[derive(Clone, Default)]
pub struct Page {
    pub section: Section,
    pub values: Zeroizing<Fields>,
    pub options: BTreeMap<String, Vec<(String, String)>>,
    pub avatar: String,
    pub blocked: Vec<(String, social::User)>,
    pub subscription: String,
    pub can_cancel: bool,
    pub renewal_date: String,
}
pub struct Submission {
    pub section: Section,
    pub values: Zeroizing<Fields>,
}
impl Submission {
    pub fn new(page: &Page) -> Result<Self, String> {
        if page.section == Section::Account {
            if page.values.get("password").is_none_or(|v| v.is_empty()) {
                return Err("Ingresá tu contraseña actual para confirmar los cambios".into());
            }
            if page.values.get("newpassword") != page.values.get("newpassword2") {
                return Err("Las nuevas contraseñas no coinciden".into());
            }
        }
        if page.section == Section::Subscription
            && page.values.get("code").is_none_or(|v| v.trim().is_empty())
        {
            return Err("Ingresá un código de Whakoom".into());
        }
        Ok(Self {
            section: page.section,
            values: page.values.clone(),
        })
    }
}
pub fn post_fields(submission: &Submission, token: String) -> Vec<(String, String)> {
    let mut fields: Vec<_> = submission
        .section
        .fields()
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                submission.values.get(*name).cloned().unwrap_or_default(),
            )
        })
        .collect();
    fields.push(("__RequestVerificationToken".into(), token));
    if submission.section == Section::Account {
        fields.push(("update".into(), "update".into()));
    }
    fields
}
pub fn selector(s: &str) -> Selector {
    Selector::parse(s).unwrap()
}
fn text(e: ElementRef<'_>) -> String {
    e.text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}
pub fn form<'a>(h: &'a Html, path: &str) -> Result<ElementRef<'a>, String> {
    h.select(&selector("form[action]"))
        .find(|e| {
            e.value()
                .attr("action")
                .and_then(|a| api::safe_url(a).ok())
                .is_some_and(|a| a == format!("{}{path}", api::BASE))
        })
        .ok_or_else(|| {
            "No se encontró el formulario de cuenta. La sesión puede haber expirado".into()
        })
}
pub fn token(html: &str, path: &str) -> Result<String, String> {
    form(&Html::parse_document(html), path)?
        .select(&selector("input[name='__RequestVerificationToken']"))
        .next()
        .and_then(|e| e.value().attr("value"))
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| "Whakoom no proporcionó la verificación del formulario".into())
}
pub fn errors(html: &str) -> Option<String> {
    let h = Html::parse_document(html);
    let messages: Vec<_> = h.select(&selector(".validation-summary-errors, .field-validation-error, .alert-danger, .error-message, .form-error, #privateprofile .error, .content-dashboard .error")).map(text).filter(|t| !t.is_empty()).collect();
    (!messages.is_empty()).then(|| messages.join(" · "))
}
pub fn parse(html: &str, section: Section) -> Result<Page, String> {
    let h = Html::parse_document(html);
    let mut page = Page {
        section,
        ..Default::default()
    };
    if section != Section::Blocked {
        let f = form(&h, section.path())?;
        for name in section.fields() {
            let field = f.select(&selector(&format!("[name='{name}']"))).next();
            if field.is_none() {
                return Err(format!(
                    "Whakoom cambió el formulario: falta el campo {name}"
                ));
            }
            let value = field
                .map(|e| match e.value().name() {
                    "textarea" => e.text().collect::<String>(),
                    "select" => e
                        .select(&selector("option[selected]"))
                        .next()
                        .or_else(|| e.select(&selector("option")).next())
                        .and_then(|o| o.value().attr("value"))
                        .unwrap_or_default()
                        .into(),
                    _ if e.value().attr("type") == Some("password") => String::new(),
                    _ if e.value().attr("type") == Some("checkbox") => {
                        if e.value().attr("checked").is_some() {
                            "true"
                        } else {
                            "false"
                        }
                        .into()
                    }
                    _ => e.value().attr("value").unwrap_or_default().into(),
                })
                .unwrap_or_default();
            page.values.insert((*name).into(), value);
        }
        for select in f.select(&selector("select[name]")) {
            page.options.insert(
                select.value().attr("name").unwrap().into(),
                select
                    .select(&selector("option[value]"))
                    .map(|o| (o.value().attr("value").unwrap().into(), text(o)))
                    .collect(),
            );
        }
    }
    page.avatar = h
        .select(&selector("#user-avatar img"))
        .next()
        .and_then(|e| e.value().attr("src"))
        .unwrap_or_default()
        .into();
    if section == Section::Subscription {
        page.can_cancel = h.select(&selector(".subscr-cancel")).next().is_some();
        page.renewal_date = h
            .select(&selector(".subscr-cancel"))
            .next()
            .and_then(|e| e.value().attr("data-item-date"))
            .unwrap_or_default()
            .into();
        page.subscription = h
            .select(&selector(".subscr-status"))
            .map(text)
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" · ");
        if page.subscription.is_empty() {
            page.subscription = h
                .select(&selector("h3"))
                .map(text)
                .collect::<Vec<_>>()
                .join(" · ");
        }
    }
    if section == Section::Blocked {
        if h.select(&selector("#user-avatar img")).next().is_none() {
            return Err("Iniciá sesión para consultar los usuarios bloqueados".into());
        }
        for row in h.select(&selector("li[data-item-id]")) {
            let Some(id) = row.value().attr("data-item-id") else {
                continue;
            };
            let Some(link) = row.select(&selector("a[href]")).find(|a| {
                a.value()
                    .attr("href")
                    .is_some_and(|href| href.starts_with('/') && href[1..].split('/').count() == 1)
            }) else {
                continue;
            };
            let username = link.value().attr("href").unwrap().trim_start_matches('/');
            if social::user_path(username).is_err() {
                continue;
            }
            let avatar = row
                .select(&selector("img"))
                .next()
                .and_then(|i| i.value().attr("src"))
                .unwrap_or_default();
            page.blocked.push((
                id.into(),
                social::User {
                    username: username.into(),
                    name: text(link),
                    avatar: avatar.into(),
                    ..Default::default()
                },
            ));
        }
    }
    Ok(page)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csrf_selects_correct_form_and_does_not_enter_page() {
        let html = r#"<form action='/micuenta/miperfil.aspx/newavatar'><input name='__RequestVerificationToken' value='avatar-token'></form><form action='/micuenta/miperfil.aspx'><input name='__RequestVerificationToken' value='profile-token'><input name='name' value='Nombre'><textarea name='bio'>Hola &amp; mundo</textarea></form>"#;
        assert_eq!(
            token(html, Section::Profile.path()).unwrap(),
            "profile-token"
        );
        let page = parse(html, Section::Profile).unwrap();
        assert_eq!(page.values["bio"], "Hola & mundo");
        assert!(!page.values.contains_key("__RequestVerificationToken"));
    }
    #[test]
    fn checkbox_password_and_select_preserve_browser_semantics() {
        let p = parse("<form action='/micuenta/privacy.aspx'><input type='checkbox' name='privateaccount' value='true' checked><input type='checkbox' name='privatecollection' value='true'></form>", Section::Privacy).unwrap();
        assert_eq!(p.values["privateaccount"], "true");
        assert_eq!(p.values["privatecollection"], "false");
        let p = parse("<form action='/micuenta/misdatos.aspx'><input name='nickname'><input name='email'><input name='newpassword' type='password'><input name='newpassword2' type='password'><input type='password' name='password' value='never-store'></form>", Section::Account).unwrap();
        assert!(p.values["password"].is_empty());
        assert!(Submission::new(&p).is_err());
    }
    #[test]
    fn region_keeps_multiple_publication_languages() {
        let p = parse("<form action='/micuenta/languageandregion.aspx'><select name='uilang'><option value='10'>Español</option></select><select name='country'><option value='32' selected>Argentina</option></select><input name='languages' value='[11274,10]'></form>", Section::Region).unwrap();
        assert_eq!(p.values["languages"], "[11274,10]");
        assert_eq!(p.options["country"][0].1, "Argentina");
    }
    #[test]
    fn writes_only_allowed_fields_with_a_fresh_token_and_explicit_false() {
        let mut page = Page {
            section: Section::Privacy,
            ..Default::default()
        };
        page.values.insert("privateaccount".into(), "false".into());
        page.values
            .insert("privatecollection".into(), "true".into());
        page.values
            .insert("deleteaccount".into(), "deleteaccount".into());
        page.values
            .insert("__RequestVerificationToken".into(), "stale".into());
        let fields = post_fields(&Submission::new(&page).unwrap(), "fresh".into());
        assert_eq!(fields.len(), 3);
        assert!(fields.contains(&("privateaccount".into(), "false".into())));
        assert!(fields.contains(&("__RequestVerificationToken".into(), "fresh".into())));
        assert!(!fields.iter().any(|(name, _)| name == "deleteaccount"));
    }
    #[test]
    fn rejects_mismatched_passwords_and_preserves_secrets_only_in_memory() {
        let mut page = Page {
            section: Section::Account,
            ..Default::default()
        };
        page.values.insert("password".into(), "current".into());
        page.values.insert("newpassword".into(), "new".into());
        page.values
            .insert("newpassword2".into(), "different".into());
        assert!(Submission::new(&page).is_err());
        page.values.insert("newpassword2".into(), "new".into());
        let submission = Submission::new(&page).unwrap();
        let fields = post_fields(&submission, "fresh".into());
        assert!(fields.contains(&("update".into(), "update".into())));
        assert!(fields.contains(&("password".into(), "current".into())));
        let html = "<div id='privateprofile'><p class='error'>Contraseña incorrecta</p></div>";
        assert_eq!(errors(html).as_deref(), Some("Contraseña incorrecta"));
    }
    #[test]
    fn blocked_users_and_subscription_are_real_server_values() {
        let html = "<div id='user-avatar'><img></div><ul><li data-item-id='42'><a href='/lector'><img src='https://i1.whakoom.com/avatar/a.jpg'>lector</a><button class='delete'></button></li></ul>";
        let page = parse(html, Section::Blocked).unwrap();
        assert_eq!(page.blocked.len(), 1);
        assert_eq!(page.blocked[0].0, "42");
        let html = "<form action='/micuenta/mysubscription.aspx'><input name='code'></form><p class='subscr-status'>Eres Pro</p><a class='subscr-cancel' data-item-date='2027-01-01'>Cancelar</a>";
        let page = parse(html, Section::Subscription).unwrap();
        assert!(page.can_cancel);
        assert_eq!(page.subscription, "Eres Pro");
        assert_eq!(page.renewal_date, "2027-01-01");
    }
}

pub fn values_match(name: &str, a: Option<&String>, b: Option<&String>) -> bool {
    if name == "languages" {
        let parse = |s: Option<&String>| {
            s.and_then(|s| serde_json::from_str::<Vec<u64>>(s).ok())
                .map(|v| v.into_iter().collect::<std::collections::BTreeSet<_>>())
        };
        return parse(a).is_some() && parse(a) == parse(b);
    }
    a.map(|s| s.trim()) == b.map(|s| s.trim())
}
#[cfg(test)]
mod contract_validation {
    use super::*;
    #[test]
    fn missing_server_fields_fail_closed_and_language_order_is_irrelevant() {
        assert!(
            parse(
                "<form action='/micuenta/privacy.aspx'></form>",
                Section::Privacy
            )
            .is_err()
        );
        assert!(values_match(
            "languages",
            Some(&"[10,11274]".into()),
            Some(&"[11274, 10]".into())
        ));
        assert!(!values_match(
            "languages",
            Some(&"[10]".into()),
            Some(&"[11274]".into())
        ));
    }
}

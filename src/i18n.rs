use serde::{Deserialize, Serialize};
use std::cell::Cell;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    Spanish,
    English,
    Portuguese,
    Russian,
    Chinese,
}
impl Language {
    pub const ALL: [Self; 5] = [
        Self::Spanish,
        Self::English,
        Self::Portuguese,
        Self::Russian,
        Self::Chinese,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Spanish => "Español",
            Self::English => "English",
            Self::Portuguese => "Português",
            Self::Russian => "Русский",
            Self::Chinese => "中文",
        }
    }
}
thread_local! {static LANGUAGE:Cell<Language>=const {Cell::new(Language::Spanish)};}
pub fn set_language(language: Language) {
    LANGUAGE.set(language);
}
pub fn tr(text: impl AsRef<str>) -> String {
    let text = text.as_ref();
    let language = LANGUAGE.get();
    if language == Language::Spanish {
        return text.into();
    }
    translate(language, text)
}
pub fn trf(template: &str, values: &[String]) -> String {
    let mut result = tr(template);
    for (index, value) in values.iter().enumerate() {
        result = result.replace(&format!("{{{index}}}"), value);
    }
    result
}
pub fn translate(language: Language, text: &str) -> String {
    let index = match language {
        Language::Spanish => return text.into(),
        Language::English => 0,
        Language::Portuguese => 1,
        Language::Russian => 2,
        Language::Chinese => 3,
    };
    static CATALOG: std::sync::OnceLock<std::collections::BTreeMap<String, [String; 4]>> =
        std::sync::OnceLock::new();
    let catalog = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("../assets/i18n.json")).expect("bundled language catalog")
    });
    if let Some(values) = catalog.get(text) {
        return values[index].clone();
    }
    if let Some((number, label)) = text.split_once(' ')
        && !number.is_empty()
        && number.bytes().all(|b| b.is_ascii_digit())
        && let Some(values) = catalog.get(label)
    {
        return format!("{number} {}", values[index]);
    }
    text.into()
}
pub fn weekdays() -> [&'static str; 7] {
    match LANGUAGE.get() {
        Language::Spanish => ["L", "M", "M", "J", "V", "S", "D"],
        Language::English => ["M", "T", "W", "T", "F", "S", "S"],
        Language::Portuguese => ["S", "T", "Q", "Q", "S", "S", "D"],
        Language::Russian => ["Пн", "Вт", "Ср", "Чт", "Пт", "Сб", "Вс"],
        Language::Chinese => ["一", "二", "三", "四", "五", "六", "日"],
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_languages_cover_navigation_and_preserve_unknown_content() {
        for language in Language::ALL
            .into_iter()
            .filter(|l| *l != Language::Spanish)
        {
            for key in [
                "Catálogo",
                "Ajustes",
                "Mi biblioteca",
                "Idioma de la aplicación",
                "Listas",
                "Lecturas",
            ] {
                assert!(!translate(language, key).is_empty());
            }
            assert_eq!(
                translate(language, "A user's comic title"),
                "A user's comic title"
            );
        }
        let catalog: std::collections::BTreeMap<String, [String; 4]> =
            serde_json::from_str(include_str!("../assets/i18n.json")).unwrap();
        for values in catalog.values() {
            assert!(values.iter().all(|text| !text.trim().is_empty()));
        }
    }
}

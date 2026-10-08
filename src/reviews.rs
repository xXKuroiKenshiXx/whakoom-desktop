use crate::api::{Api, Detail};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Default, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Draft {
    pub body: String,
    pub rating: u8,
}
impl Draft {
    pub fn validate(&self) -> Result<(), String> {
        if self.rating > 5 || self.body.encode_utf16().count() > 1000 {
            Err("La opinión admite hasta 1000 caracteres y 5 estrellas".into())
        } else {
            Ok(())
        }
    }
}
pub fn request(detail: &Detail, draft: Option<&Draft>) -> Result<(&'static str, Value), String> {
    let edition = detail
        .item
        .key
        .strip_prefix("edicion")
        .and_then(|n| n.parse::<u64>().ok());
    let id = edition
        .or(detail.numeric_id)
        .filter(|id| *id > 0)
        .ok_or("No se encontró el identificador de esta ficha")?;
    if edition.is_none() && !detail.item.key.starts_with("comic") {
        return Err("Ficha de opinión inválida".into());
    }
    if let Some(draft) = draft {
        draft.validate()?;
        Ok(if edition.is_some() {
            (
                "/wkws.asmx/UpdateEditionReview",
                json!({"e":id,"rv":draft.body}),
            )
        } else {
            (
                "/wkws.asmx/updateComicReview",
                json!({"c":id,"rv":draft.body,"rt":draft.rating}),
            )
        })
    } else {
        Ok(if edition.is_some() {
            ("/wkws.asmx/EditionReview", json!({"e":id}))
        } else {
            ("/wkws.asmx/ComicReview", json!({"c":id}))
        })
    }
}
pub fn parse(response: &Value) -> Result<Draft, String> {
    let html = response
        .get("Html")
        .and_then(Value::as_str)
        .ok_or("Whakoom no devolvió el formulario de opinión")?;
    let doc = Html::parse_fragment(html);
    let textarea = doc
        .select(&Selector::parse("textarea#txtReview").unwrap())
        .next()
        .ok_or("Whakoom no permite editar esta opinión con la sesión actual")?;
    let rating = doc
        .select(&Selector::parse(".rating-edit[data-item-id]").unwrap())
        .next()
        .and_then(|e| e.value().attr("data-item-id"))
        .and_then(|n| n.parse().ok())
        .unwrap_or(0);
    let result = Draft {
        body: textarea.text().collect(),
        rating,
    };
    result.validate()?;
    Ok(result)
}
impl Api {
    pub fn personal_review(&self, detail: &Detail) -> Result<Draft, String> {
        let (path, body) = request(detail, None)?;
        parse(&self.post(path, body)?)
    }
    pub fn publish_review(&self, detail: &Detail, draft: &Draft) -> Result<(), String> {
        let (path, body) = request(detail, Some(draft))?;
        let response = self.post(path, body)?;
        if response.get("ExtraInfo").and_then(Value::as_str) != Some("1") {
            return Err("Whakoom no confirmó la opinión; se conserva pendiente de enviar".into());
        }
        let saved = self.personal_review(detail)?;
        if saved.body.replace("\r\n", "\n") != draft.body.replace("\r\n", "\n")
            || (!detail.item.key.starts_with("edicion") && saved.rating != draft.rating)
        {
            return Err("La opinión todavía no coincide con la guardada en Whakoom".into());
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn requests_match_the_live_web_contract_and_preserve_unicode_text() {
        let comic = Detail {
            numeric_id: Some(42),
            item: crate::api::Item {
                key: "comicABC".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        let draft = Draft {
            body: "Una gran lectura 💜\n<sin spoilers>".into(),
            rating: 4,
        };
        assert_eq!(
            request(&comic, Some(&draft)).unwrap(),
            (
                "/wkws.asmx/updateComicReview",
                json!({"c":42,"rv":draft.body,"rt":4})
            )
        );
        let edition = Detail {
            item: crate::api::Item {
                key: "edicion123".into(),
                ..Default::default()
            },
            ..Default::default()
        };
        assert_eq!(
            request(&edition, Some(&draft)).unwrap().1,
            json!({"e":123,"rv":draft.body})
        );
        assert_eq!(parse(&json!({"Html":"<textarea id='txtReview'>Una gran lectura 💜\n&lt;sin spoilers&gt;</textarea><div class='rating-edit' data-item-id='4'></div>"})).unwrap(), draft);
        assert!(request(&Detail::default(), Some(&draft)).is_err());
        assert!(
            Draft {
                body: "a".repeat(1001),
                rating: 3
            }
            .validate()
            .is_err()
        );
        assert!(parse(&json!({"Html":"<p>Inicia sesión</p>"})).is_err());
        assert!(
            Draft {
                body: "💜".repeat(501),
                rating: 4
            }
            .validate()
            .is_err()
        );
    }
}

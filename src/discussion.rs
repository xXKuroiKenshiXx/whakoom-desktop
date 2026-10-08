use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Discussion {
    pub votes: String,
    pub reviews: Vec<Review>,
    pub next: Option<u32>,
}
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Review {
    pub id: String,
    pub author: String,
    pub avatar: String,
    pub date: String,
    pub rating: f32,
    pub body: String,
}
fn sel(s: &str) -> Selector {
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
pub fn parse(html: &str) -> Discussion {
    let h = Html::parse_document(html);
    let mut result = Discussion {
        votes: h.select(&sel(".b-info .rate-count, .edition-detail .rate-count, .rate-count, [itemprop='ratingCount']")).next().map(|e| e.value().attr("content").map(str::to_owned).unwrap_or_else(|| text(e))).unwrap_or_default(),
        next: h.select(&sel("#loadmoreedreviews[data-p-next], #loadRevNextPage[data-p-next]")).next().and_then(|e| e.value().attr("data-p-next")).and_then(|s| s.parse().ok()).filter(|n| *n > 0),
        ..Default::default()
    };
    for row in h.select(&sel(".review[data-item-id], [itemprop='review']")) {
        let value = |s: &str| row.select(&sel(s)).next().map(text).unwrap_or_default();
        let author = value("[itemprop='author']");
        if author.is_empty() {
            continue;
        }
        let body = value("[itemprop='reviewBody']");
        let rating = value("[itemprop='ratingValue']")
            .replace(',', ".")
            .parse::<f32>()
            .ok()
            .filter(|n| n.is_finite() && (0.0..=5.0).contains(n))
            .unwrap_or_default();
        result.reviews.push(Review {
            id: row.value().attr("data-item-id").unwrap_or_default().into(),
            author,
            body,
            rating,
            avatar: row
                .select(&sel(".image img, img"))
                .next()
                .and_then(|e| e.value().attr("src"))
                .unwrap_or_default()
                .into(),
            date: row
                .select(&sel("[itemprop='datePublished']"))
                .next()
                .and_then(|e| e.value().attr("content"))
                .map(str::to_owned)
                .unwrap_or_else(|| value(".date")),
        });
    }
    result
}
pub fn append(current: &mut Discussion, page: Discussion) {
    current.next = page.next;
    for review in page.reviews {
        if !current
            .reviews
            .iter()
            .any(|r| r.id == review.id && r.author == review.author)
        {
            current.reviews.push(review);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_review_markup_and_pagination() {
        let d = parse(
            r#"<span class='rate-count'>17</span><div class='review' data-item-id='23-42' itemprop='review'><a itemprop='author'>lectora</a><span itemprop='ratingValue'>4</span><meta itemprop='datePublished' content='2026-01-02'><p itemprop='reviewBody'>Buena &amp; entretenida.</p></div><p id='loadmoreedreviews' data-p-next='2'></p>"#,
        );
        assert_eq!(d.votes, "17");
        assert_eq!(d.next, Some(2));
        assert_eq!(d.reviews[0].body, "Buena & entretenida.");
        assert_eq!(d.reviews[0].rating, 4.);
        let mut merged = d.clone();
        append(&mut merged, d);
        assert_eq!(merged.reviews.len(), 1);
    }
}

use crate::{api::Api, storage};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct OnlineReadings {
    pub months: BTreeMap<String, usize>,
}
impl OnlineReadings {
    pub fn validate(&self) -> Result<(), String> {
        if self.months.len() > 240
            || self.months.iter().any(|(m, n)| {
                storage::reading_month(&format!("{m}-01")) != Some(m.as_str()) || *n > 100_000_000
            })
        {
            return Err("Estadísticas de lectura inválidas".into());
        }
        Ok(())
    }
}
pub fn parse(html: &str) -> Result<OnlineReadings, String> {
    let doc = Html::parse_document(html);
    let chart = doc.select(&Selector::parse("#readingstatistics .chart[data-item-values][data-item-ids]").unwrap()).next().ok_or("Whakoom no ofrece estadísticas de lectura para esta cuenta; las estadísticas locales siguen disponibles.")?;
    let counts: Vec<usize> = serde_json::from_str(&format!(
        "[{}]",
        chart.value().attr("data-item-values").unwrap_or_default()
    ))
    .map_err(|_| "Formato de estadísticas no reconocido")?;
    let ids: Vec<u32> = serde_json::from_str(&format!(
        "[{}]",
        chart.value().attr("data-item-ids").unwrap_or_default()
    ))
    .map_err(|_| "Fechas de estadísticas inválidas")?;
    if counts.len() != ids.len() || ids.len() > 240 {
        return Err("Gráfico de estadísticas incompleto".into());
    }
    let result = OnlineReadings {
        months: ids
            .into_iter()
            .zip(counts)
            .map(|(id, count)| (format!("{:04}-{:02}", id / 100, id % 100), count))
            .collect(),
    };
    result.validate()?;
    Ok(result)
}
impl Api {
    pub fn reading_statistics(&self) -> Result<OnlineReadings, String> {
        parse(&self.html("/mycollection/read/statistics")?)
    }
}
#[derive(Clone, Default)]
pub struct Annual {
    pub purchases: [usize; 12],
    pub readings: [usize; 12],
    pub spending: f64,
    pub undated_purchases: usize,
    pub undated_readings: usize,
    pub online_months: usize,
}
pub fn annual(library: &storage::Library, year: i32) -> Annual {
    let prefix = format!("{year:04}-");
    let month_index = |date: &str| {
        storage::reading_month(date)
            .filter(|m| m.starts_with(&prefix))
            .and_then(|m| m[5..7].parse::<usize>().ok())
            .map(|m| m - 1)
    };
    let mut result = Annual::default();
    for entry in library
        .entries
        .values()
        .filter(|e| e.item.key.starts_with("comic"))
    {
        if let Some(month) = month_index(&entry.purchase_date) {
            result.purchases[month] += 1;
            result.spending += entry.cost;
        } else if entry.owned && entry.purchase_date.is_empty() {
            result.undated_purchases += 1;
        }
        let mut dates: std::collections::BTreeSet<&str> =
            entry.readings.iter().map(String::as_str).collect();
        if entry.read && !entry.read_date.is_empty() {
            dates.insert(&entry.read_date);
        }
        if entry.read && dates.is_empty() {
            result.undated_readings += 1;
        }
        for date in dates {
            if let Some(month) = month_index(date) {
                result.readings[month] += 1;
            }
        }
    }
    if let Some(online) = &library.online_readings {
        for (month, count) in &online.months {
            if let Some(index) = month_index(&format!("{month}-01")) {
                result.readings[index] = *count;
                result.online_months += 1;
            }
        }
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn imports_authorized_chart_and_rejects_gated_or_invalid_pages() {
        let value = parse("<div id='readingstatistics'><div class='chart' data-item-values='3,8' data-item-ids='202601,202602'></div></div>").unwrap();
        assert_eq!(value.months["2026-02"], 8);
        assert!(parse("<div class='myc-basic'>Pro</div>").is_err());
        assert!(parse("<div id='readingstatistics'><div class='chart' data-item-values='2' data-item-ids='202613'></div></div>").is_err());
    }
    #[test]
    fn annual_counts_real_dates_not_import_time_and_merges_without_double_counting() {
        let mut library = storage::Library::default();
        library.entries.insert(
            "comicA".into(),
            storage::Entry {
                item: crate::api::Item {
                    key: "comicA".into(),
                    ..Default::default()
                },
                owned: true,
                purchase_date: "2026-02-03".into(),
                cost: 30.,
                read: true,
                read_date: "2026-02-05".into(),
                readings: vec!["2026-02-05".into(), "2025-12-01".into()],
                ..Default::default()
            },
        );
        library.entries.insert(
            "comicB".into(),
            storage::Entry {
                item: crate::api::Item {
                    key: "comicB".into(),
                    ..Default::default()
                },
                owned: true,
                ..Default::default()
            },
        );
        let local = annual(&library, 2026);
        assert_eq!(local.purchases[1], 1);
        assert_eq!(local.readings[1], 1);
        assert_eq!(local.undated_purchases, 1);
        library.online_readings = Some(OnlineReadings {
            months: BTreeMap::from([("2026-02".into(), 7)]),
        });
        assert_eq!(annual(&library, 2026).readings[1], 7);
        assert_eq!(annual(&library, 2026).spending, 30.);
    }
}

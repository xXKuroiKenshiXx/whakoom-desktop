pub const CURRENCIES: [&str; 13] = [
    "ARS", "USD", "EUR", "BRL", "MXN", "CLP", "COP", "UYU", "PEN", "GBP", "JPY", "CNY", "RUB",
];
pub fn totals(values: &std::collections::BTreeMap<String, f64>) -> String {
    let values: Vec<_> = values
        .iter()
        .filter(|(_, value)| **value > 0.)
        .map(|(currency, value)| {
            format!(
                "{value:.2} {}",
                if currency.is_empty() {
                    crate::i18n::tr("Sin moneda definida")
                } else {
                    currency.clone()
                }
            )
        })
        .collect();
    if values.is_empty() {
        "0.00".into()
    } else {
        values.join(" · ")
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn different_currencies_are_never_added_for_display() {
        let values = std::collections::BTreeMap::from([("ARS".into(), 500.), ("USD".into(), 10.)]);
        assert_eq!(super::totals(&values), "500.00 ARS · 10.00 USD");
    }
}

#[cfg(test)]
mod storage_tests {
    #[test]
    fn currency_roundtrips_and_legacy_cost_is_not_assigned_a_currency() {
        let entry: crate::storage::Entry = serde_json::from_str(r#"{"cost":111.0}"#).unwrap();
        assert!(entry.currency.is_empty());
        let mut library = crate::storage::Library::default();
        for (key, currency, cost) in [("a", "ARS", 500.), ("b", "USD", 10.)] {
            let item = crate::api::Item {
                key: format!("comic{key}"),
                url: format!("https://www.whakoom.com/comics/{key}/demo/1"),
                ..Default::default()
            };
            let entry = library.ensure(&item);
            entry.owned = true;
            entry.cost = cost;
            entry.currency = currency.into();
        }
        let restored: crate::storage::Library =
            serde_json::from_slice(&serde_json::to_vec(&library).unwrap()).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.stats().spending_by_currency.len(), 2);
        assert_eq!(
            super::totals(&restored.stats().spending_by_currency),
            "500.00 ARS · 10.00 USD"
        );
        library.entries.get_mut("comica").unwrap().currency = "invalid".into();
        assert!(library.validate().is_err());
        assert!(restored.csv().contains("Moneda"));
    }
}

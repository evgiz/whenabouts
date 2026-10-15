//! Runs the cases in `tests/cases/<locale>.toml`.
//!
//! Each case is `"query" = { "matched text" = "clause", ... }`, grouped in
//! sections. An empty table means the query must give no detections.

use std::collections::BTreeMap;
use std::fs;

#[test]
fn norwegian() {
    run_cases("nb");
}

#[test]
fn english() {
    run_cases("en");
}

type Detections = BTreeMap<String, String>;

fn run_cases(locale: &str) {
    let path = format!("{}/tests/cases/{locale}.toml", env!("CARGO_MANIFEST_DIR"));
    let sections: BTreeMap<String, BTreeMap<String, Detections>> =
        toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();

    let mut failures = Vec::new();
    let mut total = 0;
    for (section, cases) in &sections {
        for (query, expected) in cases {
            total += 1;
            let actual: Detections = whenabouts::parse(query, locale)
                .unwrap()
                .phrases()
                .iter()
                .map(|phrase| (phrase.text().to_string(), phrase.clause().to_string()))
                .collect();

            if actual != *expected {
                failures.push(format!(
                    "[{section}] {query:?}\n  expected: {}\n  actual:   {}",
                    show(expected),
                    show(&actual)
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {total} cases failed:\n\n{}\n",
        failures.len(),
        failures.join("\n\n")
    );
}

fn show(detections: &Detections) -> String {
    let items: Vec<_> = detections
        .iter()
        .map(|(text, clause)| format!("{text:?} = {clause:?}"))
        .collect();
    format!("{{ {} }}", items.join(", "))
}

#[test]
fn locales() {
    let expected = whenabouts::parse("i fjor", "nb").unwrap();
    assert_eq!(expected.phrases().len(), 1);
    for locale in ["no", "nb-NO", "nb_NO", "NB"] {
        assert_eq!(
            whenabouts::parse("i fjor", locale),
            Ok(expected.clone()),
            "{locale}"
        );
    }
    let expected = whenabouts::parse("last year", "en").unwrap();
    assert_eq!(expected.phrases().len(), 1);
    for locale in ["en-US", "en_GB", "EN"] {
        assert_eq!(
            whenabouts::parse("last year", locale),
            Ok(expected.clone()),
            "{locale}"
        );
    }
    for locale in ["xx", ""] {
        let error = whenabouts::parse("i fjor", locale).unwrap_err();
        assert_eq!(error.locale(), locale);
    }
}

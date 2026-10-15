use std::borrow::Cow;
use std::fmt;
use std::ops::{Range, RangeInclusive};
use std::slice;

use crate::clause::Clause;
use crate::date::Date;
use crate::language::Language;
use crate::resolve::Occurrences;

/// Finds the time expressions in `query`, in the language of `locale`, like
/// `"nb"`, `"nb-NO"` or `"nb_NO"`. A shortcut for [`Language::for_locale`]
/// and [`Language::parse`].
///
/// ```
/// let parsed = whenabouts::parse("bilder av katter i mai i fjor", "nb")?;
///
/// assert_eq!(parsed.rest(), "bilder av katter");
/// assert_eq!(parsed.phrases()[0].text(), "i mai i fjor");
///
/// assert!(whenabouts::parse("cats in May", "xx").is_err());
/// # Ok::<(), whenabouts::UnsupportedLocale>(())
/// ```
pub fn parse<'q>(query: &'q str, locale: &str) -> Result<Parsed<'q>, UnsupportedLocale> {
    Ok(Language::for_locale(locale)?.parse(query))
}

impl Language {
    /// Finds the time expressions in `query`.
    ///
    /// ```
    /// let nb = whenabouts::Language::for_locale("nb")?;
    /// for query in ["katter i fjor", "hunder i sommer"] {
    ///     assert_eq!(nb.parse(query).phrases().len(), 1);
    /// }
    /// # Ok::<(), whenabouts::UnsupportedLocale>(())
    /// ```
    #[must_use]
    pub fn parse<'q>(&self, query: &'q str) -> Parsed<'q> {
        let phrases = self
            .detect(query)
            .into_iter()
            .map(|detection| Phrase {
                text: &query[detection.span.clone()],
                span: detection.span,
                clause: detection.clause,
            })
            .collect();
        Parsed { query, phrases }
    }
}

/// A locale whose language isn't bundled, from [`parse`] or
/// [`Language::for_locale`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsupportedLocale {
    locale: String,
}

impl UnsupportedLocale {
    pub(crate) fn new(locale: &str) -> UnsupportedLocale {
        UnsupportedLocale {
            locale: locale.to_string(),
        }
    }

    /// The locale, as given.
    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }
}

impl fmt::Display for UnsupportedLocale {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "unsupported locale {:?}", self.locale)
    }
}

impl std::error::Error for UnsupportedLocale {}

/// A query split into its time expressions and the rest, from [`parse`] or
/// [`Language::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed<'q> {
    query: &'q str,
    phrases: Vec<Phrase<'q>>,
}

impl<'q> Parsed<'q> {
    /// The time expressions, in the order they appear.
    #[must_use]
    pub fn phrases(&self) -> &[Phrase<'q>] {
        &self.phrases
    }

    /// Whether the query has no time expressions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.phrases.is_empty()
    }

    /// The query without its time expressions, with whitespace tidied up.
    /// Useful for searching the rest of the query. The query itself if
    /// nothing was found.
    ///
    /// ```
    /// let parsed = whenabouts::parse("bilder fra jul 2019 og påske 2020", "nb").unwrap();
    /// assert_eq!(parsed.rest(), "bilder og");
    /// ```
    #[must_use]
    pub fn rest(&self) -> Cow<'q, str> {
        if self.phrases.is_empty() {
            return Cow::Borrowed(self.query);
        }
        let mut rest = String::new();
        let mut from = 0;
        for phrase in &self.phrases {
            rest.push_str(&self.query[from..phrase.span.start]);
            rest.push(' ');
            from = phrase.span.end;
        }
        rest.push_str(&self.query[from..]);
        Cow::Owned(rest.split_whitespace().collect::<Vec<_>>().join(" "))
    }

    /// Whether `date` matches any of the time expressions, as seen today. See
    /// [`Parsed::contains_as_of`] and [`Date::today`].
    #[must_use]
    pub fn contains(&self, date: Date) -> bool {
        self.contains_as_of(Date::today(), date)
    }

    /// Whether `date` matches any of the time expressions, as seen on `today`.
    /// Never, if there are none. To check many dates, collect
    /// [`Occurrences::between`] into [`DateRanges`](crate::DateRanges) once instead.
    ///
    /// ```
    /// use whenabouts::Date;
    ///
    /// let date = |y, m, d| Date::new(y, m, d).unwrap();
    /// let parsed = whenabouts::parse("katter i mai i fjor", "nb").unwrap();
    /// assert!(parsed.contains_as_of(date(2026, 10, 15), date(2025, 5, 17)));
    /// assert!(!parsed.contains_as_of(date(2026, 10, 15), date(2026, 5, 17)));
    /// ```
    #[must_use]
    pub fn contains_as_of(&self, today: Date, date: Date) -> bool {
        self.phrases
            .iter()
            .any(|phrase| phrase.clause.contains_as_of(today, date))
    }

    /// The date ranges of all the time expressions, as seen today. See
    /// [`Parsed::occurrences_as_of`] and [`Date::today`].
    #[must_use]
    pub fn occurrences(&self) -> Occurrences<'_> {
        self.occurrences_as_of(Date::today())
    }

    /// The date ranges of all the time expressions, as seen on `today`. See
    /// [`Occurrences`].
    ///
    /// ```
    /// use whenabouts::Date;
    ///
    /// let date = |y, m, d| Date::new(y, m, d).unwrap();
    /// let today = date(2026, 10, 15);
    /// let parsed = whenabouts::parse("jul og påske", "nb").unwrap();
    ///
    /// let latest: Vec<_> = parsed.occurrences_as_of(today).backward().take(2).collect();
    /// assert_eq!(latest, [
    ///     date(2026, 3, 29)..=date(2026, 4, 6),
    ///     date(2025, 12, 24)..=date(2025, 12, 26),
    /// ]);
    /// ```
    #[must_use]
    pub fn occurrences_as_of(&self, today: Date) -> Occurrences<'_> {
        let clauses = self.phrases.iter().map(|phrase| &phrase.clause).collect();
        Occurrences::new(clauses, today)
    }

    /// From the start of the first date range to the end of the last, as seen
    /// today. See [`Parsed::bounds_as_of`] and [`Date::today`].
    #[must_use]
    pub fn bounds(&self) -> Option<RangeInclusive<Date>> {
        self.bounds_as_of(Date::today())
    }

    /// From the start of the first date range to the end of the last, as seen
    /// on `today`, or `None` if there are no time expressions or one repeats
    /// forever, like "mai". Useful to narrow a search before checking dates
    /// with [`Parsed::contains_as_of`].
    ///
    /// ```
    /// use whenabouts::Date;
    ///
    /// let date = |y, m, d| Date::new(y, m, d).unwrap();
    /// let today = date(2026, 10, 15);
    /// let parsed = whenabouts::parse("jul 2019 og påske 2020", "nb").unwrap();
    /// assert_eq!(parsed.bounds_as_of(today), Some(date(2019, 12, 24)..=date(2020, 4, 13)));
    ///
    /// let parsed = whenabouts::parse("bilder fra mai", "nb").unwrap();
    /// assert_eq!(parsed.bounds_as_of(today), None);
    /// ```
    #[must_use]
    pub fn bounds_as_of(&self, today: Date) -> Option<RangeInclusive<Date>> {
        let mut bounds = self
            .phrases
            .iter()
            .map(|phrase| phrase.clause.bounds_as_of(today));
        let first = bounds.next()??;
        bounds.try_fold(first, |all, bounds| {
            let bounds = bounds?;
            Some((*all.start()).min(*bounds.start())..=(*all.end()).max(*bounds.end()))
        })
    }
}

impl<'a, 'q> IntoIterator for &'a Parsed<'q> {
    type Item = &'a Phrase<'q>;
    type IntoIter = slice::Iter<'a, Phrase<'q>>;

    fn into_iter(self) -> Self::IntoIter {
        self.phrases.iter()
    }
}

/// A time expression in a query, like "i mai i fjor".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phrase<'q> {
    text: &'q str,
    span: Range<usize>,
    clause: Clause,
}

impl<'q> Phrase<'q> {
    /// The expression as written in the query.
    #[must_use]
    pub fn text(&self) -> &'q str {
        self.text
    }

    /// Byte range of the expression in the query.
    #[must_use]
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }

    /// What the expression means.
    #[must_use]
    pub fn clause(&self) -> &Clause {
        &self.clause
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DateRanges;

    #[test]
    fn backward_is_latest_first() {
        let date = |y, m, d| Date::new(y, m, d).unwrap();
        let today = date(2026, 10, 15);
        let parsed = parse("bilder fra oktober", "nb").unwrap();
        let latest: Vec<_> = parsed.occurrences_as_of(today).backward().take(5).collect();
        let october = |year| date(year, 10, 1)..=date(year, 10, 31);
        assert_eq!(
            latest,
            [
                date(2026, 10, 1)..=date(2026, 10, 15),
                october(2025),
                october(2024),
                october(2023),
                october(2022),
            ]
        );
        let ended: Vec<_> = parsed
            .occurrences_as_of(today)
            .backward()
            .filter(|range| *range.end() < today)
            .take(2)
            .collect();
        assert_eq!(ended, [october(2025), october(2024)]);

        // Two expressions, merged latest first.
        let parsed = parse("jul og påske", "nb").unwrap();
        let latest: Vec<_> = parsed.occurrences_as_of(today).backward().take(3).collect();
        assert_eq!(
            latest,
            [
                date(2026, 3, 29)..=date(2026, 4, 6),
                date(2025, 12, 24)..=date(2025, 12, 26),
                date(2025, 4, 13)..=date(2025, 4, 21),
            ]
        );
    }

    #[test]
    fn collected_ranges_agree_with_contains() {
        let date = |y, m, d| Date::new(y, m, d).unwrap();
        let today = date(2026, 10, 15);
        // Overlapping: Christmas is inside the Christmas holidays.
        for query in ["jul 2019 og påske 2020", "jul og juleferien"] {
            let parsed = parse(query, "nb").unwrap();
            assert_eq!(parsed.phrases().len(), 2, "{query}");
            let resolved: DateRanges = parsed
                .occurrences_as_of(today)
                .between(date(2019, 1, 1), date(2020, 12, 31))
                .collect();
            let ranges = resolved.ranges();
            assert!(
                ranges
                    .windows(2)
                    .all(|pair| pair[0].end().tomorrow() < *pair[1].start()),
                "{query}"
            );
            for year in 2019..2021 {
                for (month, day) in (1..=12).flat_map(|m| (1..=31).map(move |d| (m, d))) {
                    let Some(day) = Date::new(year, month, day) else {
                        continue;
                    };
                    assert_eq!(
                        resolved.contains(day),
                        parsed.contains_as_of(today, day),
                        "{query} on {day}"
                    );
                }
            }
        }
    }
}

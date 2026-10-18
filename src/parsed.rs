use std::borrow::Cow;
use std::fmt;
use std::ops::{Range, RangeInclusive};
use std::slice;

use crate::clause::Clause;
use crate::date::Date;
use crate::language::Language;
use crate::resolve::Ranges;

/// Finds the time expressions in `query`, in English, as seen today. See
/// [`parse_as_of`] and [`Date::today`].
#[must_use]
pub fn parse(query: &str) -> Parsed<'_> {
    parse_as_of(query, Date::today())
}

/// Finds the time expressions in `query`, in English, as seen on `today`. For
/// other languages, see [`Language::for_locale`] and [`Language::parse_as_of`].
///
/// ```
/// use whenabouts::date;
///
/// let parsed = whenabouts::parse_as_of("pictures of cats last may", date(2026, 10, 15));
/// assert_eq!(parsed.rest(), "pictures of cats");
/// assert_eq!(parsed.phrases()[0].text(), "last may");
/// ```
#[must_use]
pub fn parse_as_of(query: &str, today: Date) -> Parsed<'_> {
    Language::english().parse_as_of(query, today)
}

impl Language {
    /// Finds the time expressions in `query`, as seen today. See
    /// [`Language::parse_as_of`] and [`Date::today`].
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
        self.parse_as_of(query, Date::today())
    }

    /// Finds the time expressions in `query`, as seen on `today`.
    ///
    /// ```
    /// use whenabouts::{Language, date};
    ///
    /// let nb = Language::for_locale("nb")?;
    /// let parsed = nb.parse_as_of("katter i fjor", date(2026, 10, 15));
    /// assert_eq!(parsed.bounds(), Some(date(2025, 1, 1)..=date(2025, 12, 31)));
    /// # Ok::<(), whenabouts::UnsupportedLocale>(())
    /// ```
    #[must_use]
    pub fn parse_as_of<'q>(&self, query: &'q str, today: Date) -> Parsed<'q> {
        let phrases = self
            .detect(query)
            .into_iter()
            .map(|detection| Phrase {
                text: &query[detection.span.clone()],
                span: detection.span,
                clause: detection.clause,
            })
            .collect();
        Parsed {
            query,
            phrases,
            today,
        }
    }
}

/// A locale whose language isn't bundled, from [`Language::for_locale`].
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

/// A query split into its time expressions and the rest, as seen on a given
/// day, from [`parse`] or [`Language::parse`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed<'q> {
    query: &'q str,
    phrases: Vec<Phrase<'q>>,
    today: Date,
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

    /// The day the query is seen on.
    #[must_use]
    pub fn today(&self) -> Date {
        self.today
    }

    /// The query without its time expressions, with whitespace tidied up.
    /// Useful for searching the rest of the query. The query itself if
    /// nothing was found.
    ///
    /// ```
    /// let parsed = whenabouts::parse("photos from christmas 2019 and easter 2020");
    /// assert_eq!(parsed.rest(), "photos and");
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

    /// Whether `date` matches any of the time expressions. Never, if there are
    /// none. To check many dates, collect [`Parsed::between`] into
    /// [`RangeSet`](crate::RangeSet) once instead.
    ///
    /// ```
    /// use whenabouts::date;
    ///
    /// let parsed = whenabouts::parse_as_of("cats in may last year", date(2026, 10, 15));
    /// assert!(parsed.contains(date(2025, 5, 17)));
    /// assert!(!parsed.contains(date(2026, 5, 17)));
    /// ```
    #[must_use]
    pub fn contains(&self, date: Date) -> bool {
        self.phrases
            .iter()
            .any(|phrase| phrase.clause.contains_as_of(self.today, date))
    }

    /// The date ranges up to and including today, latest first, of all the
    /// time expressions. See [`Ranges::backward`].
    ///
    /// ```
    /// use whenabouts::date;
    ///
    /// let parsed = whenabouts::parse_as_of("christmas and easter", date(2026, 10, 15));
    ///
    /// let latest: Vec<_> = parsed.backward().take(2).collect();
    /// assert_eq!(latest, [
    ///     date(2026, 4, 3)..=date(2026, 4, 6),
    ///     date(2025, 12, 24)..=date(2025, 12, 25),
    /// ]);
    /// ```
    pub fn backward(&self) -> impl Iterator<Item = RangeInclusive<Date>> + '_ {
        self.ranges().backward()
    }

    /// The date ranges up to and including `date`, latest first, of all the
    /// time expressions. See [`Ranges::backward_from`].
    pub fn backward_from(&self, date: Date) -> impl Iterator<Item = RangeInclusive<Date>> + '_ {
        self.ranges().backward_from(date)
    }

    /// The date ranges from today on, earliest first, of all the time
    /// expressions. See [`Ranges::forward`].
    pub fn forward(&self) -> impl Iterator<Item = RangeInclusive<Date>> + '_ {
        self.ranges().forward()
    }

    /// The date ranges from `date` on, earliest first, of all the time
    /// expressions. See [`Ranges::forward_from`].
    pub fn forward_from(&self, date: Date) -> impl Iterator<Item = RangeInclusive<Date>> + '_ {
        self.ranges().forward_from(date)
    }

    /// The date ranges from `first` to `last`, both included, earliest first,
    /// of all the time expressions. See [`Ranges::between`].
    pub fn between(
        &self,
        first: Date,
        last: Date,
    ) -> impl Iterator<Item = RangeInclusive<Date>> + '_ {
        self.ranges().between(first, last)
    }

    /// From the start of the first date range to the end of the last, or
    /// `None` if there are no time expressions or one repeats forever, like
    /// "mai". Useful to narrow a search before checking dates with
    /// [`Parsed::contains`].
    ///
    /// ```
    /// use whenabouts::date;
    ///
    /// let today = date(2026, 10, 15);
    /// let parsed = whenabouts::parse_as_of("christmas 2019 and easter 2020", today);
    /// assert_eq!(parsed.bounds(), Some(date(2019, 12, 24)..=date(2020, 4, 13)));
    ///
    /// let parsed = whenabouts::parse_as_of("photos from may", today);
    /// assert_eq!(parsed.bounds(), None);
    /// ```
    #[must_use]
    pub fn bounds(&self) -> Option<RangeInclusive<Date>> {
        let mut bounds = self
            .phrases
            .iter()
            .map(|phrase| phrase.clause.bounds_as_of(self.today));
        let first = bounds.next()??;
        bounds.try_fold(first, |all, bounds| {
            let bounds = bounds?;
            Some((*all.start()).min(*bounds.start())..=(*all.end()).max(*bounds.end()))
        })
    }

    /// The date ranges of all the time expressions.
    fn ranges(&self) -> Ranges<'_> {
        let clauses = self.phrases.iter().map(|phrase| &phrase.clause).collect();
        Ranges::new(clauses, self.today)
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
    use crate::RangeSet;
    use crate::date::date;

    fn nb(query: &str, today: Date) -> Parsed<'_> {
        Language::for_locale("nb")
            .unwrap()
            .parse_as_of(query, today)
    }

    #[test]
    fn backward_is_latest_first() {
        let today = date(2026, 10, 15);
        let parsed = nb("bilder fra oktober", today);
        let latest: Vec<_> = parsed.backward().take(5).collect();
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
            .backward()
            .filter(|range| *range.end() < today)
            .take(2)
            .collect();
        assert_eq!(ended, [october(2025), october(2024)]);

        // Two expressions, merged latest first.
        let parsed = nb("jul og påske", today);
        let latest: Vec<_> = parsed.backward().take(3).collect();
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
    fn backward_and_forward_stay_within_1900_to_2100() {
        let parsed = nb("bilder fra mai", date(2026, 10, 15));
        let past: Vec<_> = parsed.backward().collect();
        assert_eq!(past.len(), 127);
        assert_eq!(past.last(), Some(&(date(1900, 5, 1)..=date(1900, 5, 31))));
        let future: Vec<_> = parsed.forward().collect();
        assert_eq!(future.len(), 74);
        assert_eq!(future.last(), Some(&(date(2100, 5, 1)..=date(2100, 5, 31))));
        assert_eq!(parsed.backward_from(date(1899, 12, 31)).next(), None);
        assert_eq!(parsed.forward_from(date(2101, 1, 1)).next(), None);
    }

    #[test]
    fn collected_ranges_agree_with_contains() {
        let today = date(2026, 10, 15);
        // Overlapping: Christmas is inside the Christmas holidays.
        for query in ["jul 2019 og påske 2020", "jul og juleferien"] {
            let parsed = nb(query, today);
            assert_eq!(parsed.phrases().len(), 2, "{query}");
            let resolved: RangeSet = parsed
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
                        parsed.contains(day),
                        "{query} on {day}"
                    );
                }
            }
        }
    }
}

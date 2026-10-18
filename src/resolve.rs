//! What a clause means for concrete dates.

use std::cmp::Reverse;
use std::ops::{Range, RangeInclusive};

use jiff::civil;
use jiff::{Span, ToSpan};

use crate::clause::{Anchor, Clause, MonthDay, Part, Unit};
use crate::date::{self, Date};

/// Date ranges, sorted and merged, to check many dates quickly. Collect them
/// from [`Ranges`] or [`Parsed`](crate::Parsed), like [`Ranges::between`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RangeSet {
    /// Sorted, and neither overlapping nor touching.
    ranges: Vec<RangeInclusive<Date>>,
}

impl RangeSet {
    /// Sorts the ranges and merges the ones that overlap or touch.
    pub(crate) fn new(mut ranges: Vec<RangeInclusive<Date>>) -> RangeSet {
        ranges.sort_by_key(|range| *range.start());
        let mut merged: Vec<RangeInclusive<Date>> = Vec::with_capacity(ranges.len());
        for range in ranges {
            match merged.last_mut() {
                Some(last) if *range.start() <= last.end().tomorrow() => {
                    *last = *last.start()..=(*last.end()).max(*range.end());
                }
                _ => merged.push(range),
            }
        }
        RangeSet { ranges: merged }
    }

    /// Whether `date` is in one of the ranges.
    #[must_use]
    pub fn contains(&self, date: Date) -> bool {
        let i = self.ranges.partition_point(|range| *range.end() < date);
        self.ranges
            .get(i)
            .is_some_and(|range| *range.start() <= date)
    }

    /// The ranges, earliest first.
    #[must_use]
    pub fn ranges(&self) -> &[RangeInclusive<Date>] {
        &self.ranges
    }
}

impl FromIterator<RangeInclusive<Date>> for RangeSet {
    fn from_iter<I: IntoIterator<Item = RangeInclusive<Date>>>(ranges: I) -> RangeSet {
        RangeSet::new(ranges.into_iter().collect())
    }
}

/// The date ranges of a clause, as seen on a given day, from
/// [`Clause::ranges`]. [`Parsed`](crate::Parsed) has the same methods for all
/// of its time expressions.
///
/// Every way to get them starts from a date you give, so a clause that repeats
/// forever, like "May", never starts at the beginning of time. The ranges come
/// one at a time, and are cut to fit the dates asked for. Ranges of different
/// clauses may overlap until collected into [`RangeSet`].
///
/// ```
/// use whenabouts::{Clause, RangeSet, date};
///
/// let may: Clause = "month(5)".parse().unwrap();
/// let today = date(2026, 10, 15);
/// let mays = may.ranges_as_of(today);
///
/// // The latest first.
/// let latest: Vec<_> = mays.backward().take(2).collect();
/// assert_eq!(latest, [
///     date(2026, 5, 1)..=date(2026, 5, 31),
///     date(2025, 5, 1)..=date(2025, 5, 31),
/// ]);
///
/// // The earliest first.
/// let next = mays.forward().next();
/// assert_eq!(next, Some(date(2027, 5, 1)..=date(2027, 5, 31)));
///
/// let since_2020: RangeSet = mays.between(date(2020, 1, 1), today).collect();
/// assert!(since_2020.contains(date(2024, 5, 17)));
/// assert!(!since_2020.contains(date(2019, 5, 17)));
/// ```
#[derive(Debug, Clone)]
pub struct Ranges<'a> {
    clauses: Vec<&'a Clause>,
    today: Date,
}

impl<'a> Ranges<'a> {
    pub(crate) fn new(clauses: Vec<&'a Clause>, today: Date) -> Ranges<'a> {
        Ranges { clauses, today }
    }

    /// The ranges up to and including today, latest first, back to 1900.
    /// Today is the day these ranges are seen on. See [`Ranges::backward_from`].
    ///
    /// ```
    /// use whenabouts::{Clause, date};
    ///
    /// let october: Clause = "month(10)".parse().unwrap();
    /// let today = date(2026, 10, 15);
    ///
    /// let latest: Vec<_> = october.ranges_as_of(today).backward().take(3).collect();
    /// assert_eq!(latest, [
    ///     date(2026, 10, 1)..=date(2026, 10, 15), // cut at today
    ///     date(2025, 10, 1)..=date(2025, 10, 31),
    ///     date(2024, 10, 1)..=date(2024, 10, 31),
    /// ]);
    /// ```
    pub fn backward(&self) -> impl Iterator<Item = RangeInclusive<Date>> + use<'a> {
        self.backward_from(self.today)
    }

    /// The ranges up to and including `date`, latest first, back to 1900. One
    /// under way is cut at `date`; to leave it out, keep the ranges with
    /// `*range.end() < date`. Cheap to take a few of, even for a clause that
    /// repeats forever.
    ///
    /// ```
    /// use whenabouts::{Clause, date};
    ///
    /// let october: Clause = "month(10)".parse().unwrap();
    /// let today = date(2026, 10, 15);
    ///
    /// let before_2024: Vec<_> = october
    ///     .ranges_as_of(today)
    ///     .backward_from(date(2023, 12, 31))
    ///     .take(2)
    ///     .collect();
    /// assert_eq!(before_2024, [
    ///     date(2023, 10, 1)..=date(2023, 10, 31),
    ///     date(2022, 10, 1)..=date(2022, 10, 31),
    /// ]);
    /// ```
    pub fn backward_from(
        &self,
        date: Date,
    ) -> impl Iterator<Item = RangeInclusive<Date>> + use<'a> {
        let order = Order::Nearest(date.to_jiff());
        self.merge(order, date::to_jiff(date::FIRST..=date), |range| {
            Reverse(*range.end())
        })
    }

    /// The ranges from today on, earliest first, up to the end of 2100. Today
    /// is the day these ranges are seen on. See [`Ranges::forward_from`].
    ///
    /// ```
    /// use whenabouts::{Clause, date};
    ///
    /// let summer: Clause = "month(6..8)".parse().unwrap();
    /// let today = date(2026, 7, 10);
    ///
    /// let next: Vec<_> = summer.ranges_as_of(today).forward().take(2).collect();
    /// assert_eq!(next, [
    ///     date(2026, 7, 10)..=date(2026, 8, 31), // cut at today
    ///     date(2027, 6, 1)..=date(2027, 8, 31),
    /// ]);
    /// ```
    pub fn forward(&self) -> impl Iterator<Item = RangeInclusive<Date>> + use<'a> {
        self.forward_from(self.today)
    }

    /// The ranges from `date` on, earliest first, up to the end of 2100. One
    /// under way is cut at `date`.
    ///
    /// ```
    /// use whenabouts::{Clause, date};
    ///
    /// let summer: Clause = "month(6..8)".parse().unwrap();
    /// let today = date(2026, 10, 15);
    ///
    /// let first = summer.ranges_as_of(today).forward_from(date(2000, 1, 1)).next();
    /// assert_eq!(first, Some(date(2000, 6, 1)..=date(2000, 8, 31)));
    /// ```
    pub fn forward_from(&self, date: Date) -> impl Iterator<Item = RangeInclusive<Date>> + use<'a> {
        self.merge(Order::Earliest, date::to_jiff(date..=date::LAST), |range| {
            *range.start()
        })
    }

    /// The ranges from `first` to `last`, both included, earliest first.
    /// Ranges that stick out are cut to fit. To check many dates, collect them
    /// into [`RangeSet`].
    ///
    /// ```
    /// use whenabouts::{RangeSet, date};
    ///
    /// let today = date(2026, 10, 15);
    /// let parsed = whenabouts::parse_as_of("july and august", today);
    ///
    /// let since_2025: Vec<_> = parsed.between(date(2025, 1, 1), today).collect();
    /// assert_eq!(since_2025, [
    ///     date(2025, 7, 1)..=date(2025, 7, 31),
    ///     date(2025, 8, 1)..=date(2025, 8, 31),
    ///     date(2026, 7, 1)..=date(2026, 7, 31),
    ///     date(2026, 8, 1)..=date(2026, 8, 31),
    /// ]);
    ///
    /// let this_year: RangeSet = parsed.between(date(2026, 1, 1), today).collect();
    /// assert_eq!(this_year.ranges(), [date(2026, 7, 1)..=date(2026, 8, 31)]);
    /// ```
    pub fn between(
        &self,
        first: Date,
        last: Date,
    ) -> impl Iterator<Item = RangeInclusive<Date>> + use<'a> {
        let within = date::to_jiff(first..=last);
        self.merge(Order::Earliest, within, |range| *range.start())
    }

    /// The ranges of every clause within `within`, in `order`, merged into one
    /// list sorted by `key`.
    fn merge<K: Ord + 'a, F: Fn(&RangeInclusive<Date>) -> K + 'a>(
        &self,
        order: Order,
        within: Range<civil::Date>,
        key: F,
    ) -> impl Iterator<Item = RangeInclusive<Date>> + use<'a, K, F> {
        let today = self.today.to_jiff();
        let lists = self
            .clauses
            .iter()
            .map(|&clause| {
                clause
                    .ranges_in(today, order, within.clone())
                    .map(date::from_jiff)
            })
            .collect();
        merge_by_key(lists, key)
    }
}

impl Clause {
    /// Whether `date` matches the clause, as seen today. See
    /// [`Clause::contains_as_of`] and [`Date::today`].
    #[must_use]
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn contains(&self, date: Date) -> bool {
        self.contains_as_of(Date::today(), date)
    }

    /// The date ranges the clause covers, as seen today. See
    /// [`Clause::ranges_as_of`] and [`Date::today`].
    #[must_use]
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn ranges(&self) -> Ranges<'_> {
        self.ranges_as_of(Date::today())
    }

    /// The date ranges the clause covers, as seen on `today`. See
    /// [`Ranges`].
    #[must_use]
    pub fn ranges_as_of(&self, today: Date) -> Ranges<'_> {
        Ranges::new(vec![self], today)
    }

    /// From the start of the first date range to the end of the last, as seen
    /// today, or `None` if the clause repeats forever. See
    /// [`Clause::bounds_as_of`] and [`Date::today`].
    #[must_use]
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn bounds(&self) -> Option<RangeInclusive<Date>> {
        self.bounds_as_of(Date::today())
    }

    /// From the start of the first date range to the end of the last, as seen
    /// on `today`, or `None` if the clause repeats forever, like "May". Useful
    /// to narrow a search before checking dates with [`Clause::contains_as_of`].
    ///
    /// ```
    /// use whenabouts::{Clause, date};
    ///
    /// let today = date(2026, 10, 15);
    /// let weekends: Clause = "weekday(sat..sun) from ago(year, 1)".parse().unwrap();
    /// assert_eq!(weekends.bounds_as_of(today), Some(date(2025, 1, 4)..=date(2025, 12, 28)));
    ///
    /// let may: Clause = "month(5)".parse().unwrap();
    /// assert_eq!(may.bounds_as_of(today), None);
    /// ```
    #[must_use]
    pub fn bounds_as_of(&self, today: Date) -> Option<RangeInclusive<Date>> {
        self.anchor()?;
        let all = date::to_jiff(..);
        let ranges: RangeSet = self
            .ranges_in(today.to_jiff(), Order::Earliest, all)
            .map(date::from_jiff)
            .collect();
        let (first, last) = (ranges.ranges().first()?, ranges.ranges().last()?);
        Some(*first.start()..=*last.end())
    }

    /// The ranges within `within`, as seen on `today`, in `order`.
    fn ranges_in(
        &self,
        today: civil::Date,
        order: Order,
        within: Range<civil::Date>,
    ) -> Box<dyn Iterator<Item = Range<civil::Date>> + '_> {
        let Range {
            start: from,
            end: until,
        } = within;
        let clip = move |range: Range<civil::Date>| {
            let range = range.start.max(from)..range.end.min(until);
            (!range.is_empty()).then_some(range)
        };

        // An anchor alone, or back(n), is a single range.
        let single = match self.anchor() {
            Some(anchor) if self.parts().is_empty() => Some(anchor.days(today)),
            Some(Anchor::Back(n)) => {
                let started = self.started_by(today, civil::Date::MIN.year()..=today.year());
                Some(match n.checked_sub(1) {
                    None => started.into_iter().next(),
                    Some(n) => started.filter(|range| range.end <= today).nth(n as usize),
                })
            }
            _ => None,
        };
        if let Some(range) = single {
            return Box::new(range.and_then(clip).into_iter());
        }

        // Only scan years that can hold a kept occurrence. One may start the
        // year before: a winter, or ISO week 1.
        let anchor_days = match self.anchor().map(|anchor| anchor.days(today)) {
            // The anchor is before the first date or after the last.
            Some(None) => return Box::new(std::iter::empty()),
            Some(days) => days,
            None => None,
        };
        let (mut first, mut last) = (from.year() - 1, until.year());
        if let Some(days) = &anchor_days {
            first = first.max(days.start.year() - 1);
            last = last.min(days.end.year());
        }
        let keep = move |range: Range<civil::Date>| {
            let anchored = anchor_days
                .as_ref()
                .is_none_or(|days| days.contains(&self.anchor_day(range.start)));
            if anchored { clip(range) } else { None }
        };

        let near = match order {
            Order::Earliest => {
                let ranges = (first..=last).flat_map(|year| self.occurrences_in(year));
                return Box::new(ranges.filter_map(keep));
            }
            Order::Nearest(near) => near,
        };
        let before = self
            .started_by(near, first..=last.min(near.year()))
            .filter_map(keep.clone());
        let after = (first.max(near.year())..=last)
            .flat_map(|year| self.occurrences_in(year))
            .filter(move |range| range.start > near)
            .filter_map(keep);
        Box::new(nearest(near, before, after))
    }

    /// Whether `date` matches the clause, as seen on `today`. To check many
    /// dates, collect [`Ranges::between`] into [`RangeSet`] once instead.
    ///
    /// ```
    /// use whenabouts::date;
    ///
    /// let may_last_year: whenabouts::Clause = "month(5) from ago(year, 1)".parse().unwrap();
    /// let today = date(2026, 10, 15);
    /// assert!(may_last_year.contains_as_of(today, date(2025, 5, 17)));
    /// assert!(!may_last_year.contains_as_of(today, date(2026, 5, 17)));
    /// ```
    #[must_use]
    pub fn contains_as_of(&self, today: Date, date: Date) -> bool {
        let (today, date) = (today.to_jiff(), date.to_jiff());
        if !self.in_period(date) {
            return false;
        }
        match self.anchor() {
            None => true,
            // Which occurrence is the latest depends on the ones before it.
            Some(Anchor::Back(_)) => {
                let day = date..date.saturating_add(1.day());
                self.ranges_in(today, Order::Nearest(date), day)
                    .next()
                    .is_some()
            }
            Some(anchor) => anchor.days(today).is_some_and(|days| {
                let start = if self.parts().is_empty() {
                    date
                } else {
                    self.occurrence_start(date)
                };
                days.contains(&self.anchor_day(start))
            }),
        }
    }

    /// The first day of the occurrence of the period that `date` is in.
    fn occurrence_start(&self, date: civil::Date) -> civil::Date {
        let mut start = date;
        while let Ok(before) = start.yesterday()
            && self.in_period(before)
        {
            start = before;
        }
        start
    }

    fn in_period(&self, date: civil::Date) -> bool {
        self.parts().iter().all(|part| part.contains(date))
    }

    /// The occurrences of the period that start in `year`, in order.
    fn occurrences_in(&self, year: i16) -> Vec<Range<civil::Date>> {
        let Ok(first) = civil::Date::new(year, 1, 1) else {
            return Vec::new();
        };
        let end = |start: civil::Date| {
            start
                .series(1.day())
                .find(|&day| !self.in_period(day))
                .unwrap_or(civil::Date::MAX)
        };
        first
            .series(1.day())
            .take_while(|day| day.year() == year)
            .filter(|&day| {
                self.in_period(day) && day.yesterday().is_ok_and(|before| !self.in_period(before))
            })
            .map(|start| start..end(start))
            .collect()
    }

    /// The occurrences in `years` that start by `date`, latest first.
    fn started_by(
        &self,
        date: civil::Date,
        years: RangeInclusive<i16>,
    ) -> impl Iterator<Item = Range<civil::Date>> + '_ {
        years
            .rev()
            .flat_map(|year| self.occurrences_in(year).into_iter().rev())
            .filter(move |range| range.start <= date)
    }

    /// The day that decides which anchor an occurrence belongs to: its start,
    /// or for weeks the Thursday of its first week, so weeks follow ISO years.
    fn anchor_day(&self, start: civil::Date) -> civil::Date {
        if self
            .parts()
            .iter()
            .any(|part| matches!(part, Part::Week(..)))
        {
            Unit::WEEK.start(start).saturating_add(3.days())
        } else {
            start
        }
    }
}

/// Merges lists of ranges, each sorted by `key`, into one.
pub(crate) fn merge_by_key<K: Ord>(
    lists: Vec<impl Iterator<Item = RangeInclusive<Date>>>,
    key: impl Fn(&RangeInclusive<Date>) -> K,
) -> impl Iterator<Item = RangeInclusive<Date>> {
    let mut lists: Vec<_> = lists.into_iter().map(Iterator::peekable).collect();
    std::iter::from_fn(move || {
        let (_, first) = lists
            .iter_mut()
            .enumerate()
            .filter_map(|(i, list)| Some((key(list.peek()?), i)))
            .min()?;
        lists[first].next()
    })
}

/// The order of [`Clause::ranges_in`].
#[derive(Clone, Copy)]
enum Order {
    Earliest,
    /// Nearest this day first.
    Nearest(civil::Date),
}

/// Merges two lists of ranges, each sorted by distance from `date`.
fn nearest(
    date: civil::Date,
    before: impl Iterator<Item = Range<civil::Date>>,
    after: impl Iterator<Item = Range<civil::Date>>,
) -> impl Iterator<Item = Range<civil::Date>> {
    let (mut before, mut after) = (before.peekable(), after.peekable());
    std::iter::from_fn(move || {
        let take_before = match (before.peek(), after.peek()) {
            (Some(b), Some(a)) => gap(b, date) <= gap(a, date),
            (b, _) => b.is_some(),
        };
        if take_before {
            before.next()
        } else {
            after.next()
        }
    })
}

/// Days between `range` and `date`, 0 if `date` is in it.
fn gap(range: &Range<civil::Date>, date: civil::Date) -> i32 {
    if date < range.start {
        (range.start - date).get_days()
    } else if date >= range.end {
        (date - range.end).get_days() + 1
    } else {
        0
    }
}

impl Part {
    fn contains(self, date: civil::Date) -> bool {
        let day = date.day() as u8;
        let month = date.month() as u8;
        match self {
            Part::Month(a, b) => between(month, a, b),
            Part::Week(a, b) => between(date.iso_week_date().week() as u8, a, b),
            Part::Easter(a, b) => {
                let easter = easter(date.year());
                let days = |n: i16| easter.saturating_add(i64::from(n).days());
                days(a) <= date && date <= days(b)
            }
            Part::Date(a, b) => between(MonthDay { month, day }, a, b),
            Part::Day(a, b) => between(day, a, b),
            Part::Weekday(a, b) => between(
                date.weekday().to_monday_zero_offset() as u8,
                a as u8,
                b as u8,
            ),
        }
    }
}

impl Anchor {
    /// The days the anchor covers, or `None` for `back(n)`, which depends on
    /// the period, or for an anchor outside the dates there are.
    fn days(self, today: civil::Date) -> Option<Range<civil::Date>> {
        let tomorrow = today.saturating_add(1.day());
        Some(match self {
            Anchor::Year(year) => {
                let start = civil::Date::new(year, 1, 1).ok()?;
                start..start.saturating_add(1.year())
            }
            Anchor::Ago(unit, n) => {
                let start = unit.start(today).checked_sub(unit.times(n)?).ok()?;
                start..start.saturating_add(unit.times(1)?)
            }
            Anchor::Last(unit, n) => tomorrow.checked_sub(unit.times(n)?).ok()?..tomorrow,
            Anchor::Back(_) => return None,
        })
    }
}

impl Unit {
    /// The first day of the unit that `date` is in.
    fn start(self, date: civil::Date) -> civil::Date {
        match self {
            Unit::Day => date,
            Unit::Week(first) => {
                let since = (date.weekday().to_monday_zero_offset() - first as i8).rem_euclid(7);
                date.saturating_sub(i64::from(since).days())
            }
            Unit::Month => date.first_of_month(),
            Unit::Year => date.first_of_year(),
        }
    }

    /// `n` of the unit, or `None` if that is more than the dates there are.
    fn times(self, n: u32) -> Option<Span> {
        let n = i64::from(n);
        let span = Span::new();
        match self {
            Unit::Day => span.try_days(n),
            Unit::Week(_) => span.try_weeks(n),
            Unit::Month => span.try_months(n),
            Unit::Year => span.try_years(n),
        }
        .ok()
    }
}

/// Whether `x` is in `a..=b`, wrapping around when `a` comes after `b`.
fn between<T: PartialOrd>(x: T, a: T, b: T) -> bool {
    if a <= b {
        a <= x && x <= b
    } else {
        a <= x || x <= b
    }
}

/// Easter Sunday, by the anonymous Gregorian algorithm.
fn easter(year: i16) -> civil::Date {
    let y = i32::from(year);
    let (a, b, c) = (y.rem_euclid(19), y.div_euclid(100), y.rem_euclid(100));
    let (d, e) = (b.div_euclid(4), b.rem_euclid(4));
    let g = (b - (b + 8).div_euclid(25) + 1).div_euclid(3);
    let h = (19 * a + b - d - g + 15).rem_euclid(30);
    let l = (32 + 2 * e + 2 * c.div_euclid(4) - h - c.rem_euclid(4)).rem_euclid(7);
    let m = (a + 11 * h + 22 * l).div_euclid(451);
    let n = h + l - 7 * m + 114;
    jiff::civil::date(year, (n / 31) as i8, (n % 31 + 1) as i8)
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::date::date;

    #[test]
    fn finds_easter() {
        assert_eq!(easter(2025), civil::date(2025, 4, 20));
        assert_eq!(easter(2026), civil::date(2026, 4, 5));
        assert_eq!(easter(2038), civil::date(2038, 4, 25));
    }

    #[test]
    fn contains_dates() {
        // A Thursday.
        let today = date(2026, 10, 15);
        for (clause, day, expected) in [
            ("month(5) from ago(year, 1)", date(2025, 5, 17), true),
            ("month(5) from ago(year, 1)", date(2026, 5, 17), false),
            ("month(12..2) from year(2024)", date(2025, 2, 10), true),
            ("month(12..2) from year(2024)", date(2024, 1, 10), false),
            ("date(12-31..01-01) from year(2020)", date(2021, 1, 1), true),
            (
                "date(12-31..01-01) from year(2020)",
                date(2020, 1, 1),
                false,
            ),
            // ISO week 1 of 2025 starts 2024-12-30, and of 2026 on 2025-12-29.
            ("week(1) from year(2025)", date(2024, 12, 30), true),
            ("week(1) from year(2025)", date(2025, 12, 29), false),
            ("easter(-2)", date(2026, 4, 3), true),
            ("easter(-2)", date(2025, 4, 18), true),
            ("easter(-2)", date(2025, 4, 17), false),
            ("month(7) & weekday(fri)", date(2026, 7, 3), true),
            ("month(7) & weekday(fri)", date(2026, 7, 4), false),
            ("month(6..8) from back(0)", date(2026, 7, 1), true),
            ("month(6..8) from back(0)", date(2025, 7, 1), false),
            // Summer 2026 has ended, so it is the one before as well.
            ("month(6..8) from back(1)", date(2026, 7, 1), true),
            ("month(6..8) from back(1)", date(2025, 7, 1), false),
            ("month(6..8) from back(2)", date(2025, 7, 1), true),
            // Autumn is under way, so the one before is last year's.
            ("month(9..11) from back(1)", date(2025, 10, 1), true),
            ("month(9..11) from back(1)", date(2026, 10, 1), false),
            ("date(12-24..12-26) from back(0)", date(2025, 12, 25), true),
            ("weekday(sat..sun) from back(0)", date(2026, 10, 11), true),
            ("weekday(sat..sun) from back(0)", date(2026, 10, 4), false),
            ("day(3) from ago(month, 1)", date(2026, 9, 3), true),
            ("ago(day, 0)", today, true),
            ("ago(week, 1)", date(2026, 10, 5), true),
            ("ago(week, 1)", date(2026, 10, 12), false),
            ("last(month, 3)", date(2026, 7, 16), true),
            ("last(month, 3)", date(2026, 7, 15), false),
            ("last(month, 3)", date(2026, 10, 16), false),
            ("ago(year, 100000)", date(2026, 1, 1), false),
            ("ago(year, 15000)", date(-9999, 1, 1), false),
            ("last(week, 4000000000)", date(2000, 1, 1), false),
            ("month(5) from ago(year, 15000)", date(2000, 5, 1), false),
        ] {
            let parsed: Clause = clause.parse().unwrap();
            assert_eq!(
                parsed.contains_as_of(today, day),
                expected,
                "{clause} on {day}"
            );
        }
    }

    #[test]
    fn finds_ranges_nearest_first() {
        let today = date(2026, 10, 15);
        let all = date(2000, 1, 1)..date(2100, 1, 1);
        let may = |year| date(year, 5, 1)..=date(year, 5, 31);
        for (clause, near, within, expected) in [
            // May 2026 is nearest, then 2025, then 2027.
            (
                "month(5)",
                date(2026, 1, 10),
                all.clone(),
                vec![may(2026), may(2025), may(2027)],
            ),
            (
                "month(5)",
                today,
                date(2025, 5, 20)..date(2026, 5, 10),
                vec![
                    date(2026, 5, 1)..=date(2026, 5, 9),
                    date(2025, 5, 20)..=date(2025, 5, 31),
                ],
            ),
            (
                "month(12..2) from year(2024)",
                today,
                date(2025, 1, 1)..date(2026, 1, 1),
                vec![date(2025, 1, 1)..=date(2025, 2, 28)],
            ),
            (
                "weekday(fri) from ago(week, 1)",
                today,
                all.clone(),
                vec![date(2026, 10, 9)..=date(2026, 10, 9)],
            ),
            (
                "month(6..8) from back(2)",
                today,
                all.clone(),
                vec![date(2025, 6, 1)..=date(2025, 8, 31)],
            ),
            (
                "last(month, 3)",
                today,
                all.clone(),
                vec![date(2026, 7, 16)..=date(2026, 10, 15)],
            ),
            (
                "month(5) from year(2020)",
                today,
                date(2021, 1, 1)..date(2022, 1, 1),
                vec![],
            ),
            // Anchors before the first date there is.
            ("ago(year, 15000)", today, all.clone(), vec![]),
            ("last(week, 4000000000)", today, all.clone(), vec![]),
            ("month(5) from ago(year, 15000)", today, all.clone(), vec![]),
        ] {
            let parsed: Clause = clause.parse().unwrap();
            let found: Vec<_> = parsed
                .ranges_in(
                    today.to_jiff(),
                    Order::Nearest(near.to_jiff()),
                    date::to_jiff(within),
                )
                .map(date::from_jiff)
                .take(3)
                .collect();
            assert_eq!(found, expected, "{clause}");
        }
    }

    #[test]
    fn collected_ranges_agree_with_contains() {
        let today = date(2026, 10, 15);
        for clause in [
            "month(12..2) from year(2024)",
            "easter(-7..1)",
            "weekday(sat..sun) from back(1)",
            "month(7) & weekday(fri)",
            "last(month, 3)",
            "week(1) from year(2025)",
            "week(52..1) from year(2024)",
            "month(5) from ago(year, 1)",
            "weekday(fri) from ago(week, 1)",
            "date(12-31..01-01) from year(2024)",
            "easter(-2) from year(2024)",
            "ago(month, 2)",
        ] {
            let parsed: Clause = clause.parse().unwrap();
            let all = parsed.ranges_as_of(today);
            let ranges: Vec<_> = all.between(date(2023, 1, 1), date(2026, 12, 31)).collect();
            assert!(
                ranges
                    .windows(2)
                    .all(|pair| pair[0].end() < pair[1].start()),
                "{clause}: earliest first"
            );
            let past: Vec<_> = all.backward().take(4).collect();
            assert!(
                past.windows(2).all(|pair| pair[0].start() > pair[1].end()),
                "{clause}: latest first"
            );
            let resolved: RangeSet = ranges.into_iter().collect();
            let days = civil::date(2023, 1, 1).series(1.day()).take(4 * 365);
            for day in days.map(Date::from_jiff) {
                let expected = parsed.contains_as_of(today, day);
                assert_eq!(resolved.contains(day), expected, "{clause} on {day}");
            }
        }
    }
}

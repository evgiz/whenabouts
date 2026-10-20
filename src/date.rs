use std::fmt;
use std::ops::{Bound, Range, RangeBounds, RangeInclusive};

use jiff::{ToSpan, civil};

/// A calendar date.
///
/// ```
/// use whenabouts::Date;
///
/// let date = Date::new(2024, 6, 1).unwrap();
/// assert_eq!(date.to_string(), "2024-06-01");
/// assert!(Date::new(2024, 2, 30).is_none());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i16,
    month: u8,
    day: u8,
}

/// The date, for dates written in code. Usable in constants. See [`Date::new`]
/// for dates that may not exist.
///
/// # Panics
///
/// If the date doesn't exist.
///
/// ```
/// use whenabouts::{Date, date};
///
/// const LAUNCH: Date = date(2024, 6, 1);
/// assert_eq!(LAUNCH, Date::new(2024, 6, 1).unwrap());
/// ```
///
/// ```should_panic
/// whenabouts::date(2024, 2, 30);
/// ```
#[must_use]
pub const fn date(year: i16, month: u8, day: u8) -> Date {
    assert!(month <= 12 && day <= 31, "invalid date");
    // Panics if the date doesn't exist.
    civil::date(year, month as i8, day as i8);
    Date { year, month, day }
}

impl Date {
    /// The date, or `None` if it doesn't exist. Years go from -9999 to 9999.
    /// See [`date`] for dates written in code.
    pub fn new(year: i16, month: u8, day: u8) -> Option<Date> {
        let month = i8::try_from(month).ok()?;
        let day = i8::try_from(day).ok()?;
        civil::Date::new(year, month, day).ok().map(Date::from_jiff)
    }

    /// -9999–9999.
    #[must_use]
    pub fn year(self) -> i16 {
        self.year
    }

    /// 1–12.
    #[must_use]
    pub fn month(self) -> u8 {
        self.month
    }

    /// 1–31.
    #[must_use]
    pub fn day(self) -> u8 {
        self.day
    }

    /// Today, in this machine's time zone. On a server that is often UTC, which
    /// may not be the user's date; pass the user's date to
    /// [`parse_as_of`](crate::parse_as_of) or the `_as_of` methods instead.
    #[must_use]
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub fn today() -> Date {
        Date::from_jiff(jiff::Zoned::now().date())
    }

    pub(crate) fn from_jiff(date: civil::Date) -> Date {
        Date {
            year: date.year(),
            month: date.month() as u8,
            day: date.day() as u8,
        }
    }

    pub(crate) fn to_jiff(self) -> civil::Date {
        civil::date(self.year, self.month as i8, self.day as i8)
    }

    /// The day after, or this day if it is the last one there is.
    pub(crate) fn tomorrow(self) -> Date {
        self.to_jiff().tomorrow().map_or(self, Date::from_jiff)
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// The first day ranges are found from, and the year slot reads, when nothing
/// earlier is asked for.
pub(crate) const FIRST: Date = Date {
    year: 1900,
    month: 1,
    day: 1,
};

/// The last day ranges are found to, and the year slot reads, when nothing
/// later is asked for.
pub(crate) const LAST: Date = Date {
    year: 2100,
    month: 12,
    day: 31,
};

/// Any range of dates, like `..today` or `..`, as a half-open range.
pub(crate) fn to_jiff(range: impl RangeBounds<Date>) -> Range<civil::Date> {
    let start = match range.start_bound() {
        Bound::Included(date) => date.to_jiff(),
        Bound::Excluded(date) => date.to_jiff().saturating_add(1.day()),
        Bound::Unbounded => civil::Date::MIN,
    };
    let end = match range.end_bound() {
        Bound::Included(date) => date.to_jiff().saturating_add(1.day()),
        Bound::Excluded(date) => date.to_jiff(),
        Bound::Unbounded => civil::Date::MAX,
    };
    start..end
}

/// A non-empty range from the calculations, which end the day after, as the
/// inclusive range users get.
pub(crate) fn from_jiff(range: Range<civil::Date>) -> RangeInclusive<Date> {
    let last = range.end.yesterday().unwrap_or(range.end);
    Date::from_jiff(range.start)..=Date::from_jiff(last)
}

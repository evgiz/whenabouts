//! Find time expressions in search queries, and the dates they mean.
//!
//! [`parse`] splits a query into its time expressions and the rest. Each one
//! has a [`Clause`] that describes *which* times match ("May", "last year",
//! "Christmas Eve 2019"), not a concrete date range, so checking a date
//! against it depends on today's date.
//!
//! [`parse`] uses a bundled [`Language`]. To add your own phrases to one, or
//! write a whole language, see [`Language::extend`] and
//! [`Language::from_toml`].
//!
//! Methods like [`Parsed::contains`] use today in this machine's time zone.
//! Their `_as_of` versions, like [`Parsed::contains_as_of`], take today's date
//! instead: for tests, or on a server, where the machine's date may not be the
//! user's.
//!
//! ```
//! use whenabouts::Date;
//!
//! let parsed = whenabouts::parse("bilder av katter i mai i fjor", "nb")?;
//!
//! assert_eq!(parsed.phrases()[0].text(), "i mai i fjor");
//! assert_eq!(parsed.rest(), "bilder av katter");
//!
//! let today = Date::new(2026, 10, 15).unwrap();
//! assert!(parsed.contains_as_of(today, Date::new(2025, 5, 17).unwrap()));
//!
//! // Or as seen today.
//! let last_may = Date::new(Date::today().year() - 1, 5, 17).unwrap();
//! assert!(parsed.contains(last_may));
//! # Ok::<(), whenabouts::UnsupportedLocale>(())
//! ```

mod clause;
mod date;
mod detect;
mod language;
mod parsed;
mod resolve;
mod tokenize;

pub use clause::{Clause, ParseClauseError};
pub use date::Date;
pub use language::{Language, LanguageError};
pub use parsed::{Parsed, Phrase, UnsupportedLocale, parse};
pub use resolve::{DateRanges, Occurrences};

/// Runs the examples in the README as tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

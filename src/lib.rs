//! Find time expressions in search queries, and the dates they mean.
//!
//! [`parse`] splits a query into its time expressions and the rest. Each one
//! has a [`Clause`] that describes *which* times match ("May", "last year",
//! "Christmas Eve 2019"), not a concrete date range, so checking a date
//! against it depends on today's date.
//!
//! [`parse`] reads English. For another bundled [`Language`], see
//! [`Language::for_locale`]. To add your own phrases to one, or write a whole
//! language, see [`Language::extend`] and [`Language::from_toml`].
//!
//! [`parse`] sees the query as of today in this machine's time zone.
//! [`parse_as_of`] takes today's date instead: for tests, or on a server,
//! where the machine's date may not be the user's.
//!
//! ```
//! use whenabouts::{Date, date};
//!
//! let parsed = whenabouts::parse("pictures of cats in may last year");
//!
//! assert_eq!(parsed.phrases()[0].text(), "in may last year");
//! assert_eq!(parsed.rest(), "pictures of cats");
//!
//! // As seen today.
//! let last_may = date(Date::today().year() - 1, 5, 17);
//! assert!(parsed.contains(last_may));
//!
//! // Or as seen on a given day.
//! let today = date(2026, 10, 15);
//! let parsed = whenabouts::parse_as_of("pictures of cats in may last year", today);
//! assert!(parsed.contains(date(2025, 5, 17)));
//! ```

mod clause;
mod date;
mod detect;
mod language;
mod parsed;
mod resolve;
mod tokenize;

pub use clause::{Clause, ParseClauseError};
pub use date::{Date, date};
pub use language::{Language, LanguageError};
pub use parsed::{Parsed, Phrase, UnsupportedLocale, parse, parse_as_of};
pub use resolve::{RangeSet, Ranges};

/// Runs the examples in the README as tests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;

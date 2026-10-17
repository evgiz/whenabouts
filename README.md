# Whenabouts

Fast natural-language date period extraction for search queries.

Automatically detect phrases such as "last week", "summer of 2023" or "christmas 2 years ago" and convert them into clauses or date ranges. Fast and easily extensible.

[Try the live demo](https://evgiz.github.io/whenabouts/).

## Usage

It is simple to check whether a date is included in the query or to extract the semantic part of the query:

```rust

use whenabouts::Date;
let day_in_may = Date::new(2026, 5, 23).unwrap();

// Parse a natural language query
let query = "pictures of cats in may";
let parsed = whenabouts::parse(query, "en").unwrap();

// Assert a date in may is included
assert!(parsed.contains(day_in_may));

// Get the remaining semantic part of the query
assert_eq!(parsed.rest(), "pictures of cats");

```

## Filtering using date ranges

A typical use case is to filter results e.g. in a database query.
To do this we must extract date range occurrences from the parsed result.

Specific queries, such as "january 2023", only match a single range, while some phrases like "weekend" repeat indefinitely. To avoid dealing with a potentially huge number of date ranges, use the lazy helpers to limit which ones you consider:

- `backward()`: ranges up to and including today, latest first
- `forward()`: ranges from today on, earliest first
- `between(first, last)`: ranges between two dates, earliest first

Example:

```rust

use whenabouts::{Date, DateRanges};

// The reference point for today
let date = |y, m, d| Date::new(y, m, d).unwrap();
let today = date(2026, 10, 15);

// Parse the natural language query
let query = "hiking photos from october";
let parsed = whenabouts::parse(query, "en").unwrap();
let octobers = parsed.occurrences_as_of(today);

// Get the 3 latest octobers (including the current one)
let latest: Vec<_> = octobers.backward().take(3).collect();
assert_eq!(latest[0], date(2026, 10, 1)..=date(2026, 10, 15)); // Cut at today
assert_eq!(latest[1], date(2025, 10, 1)..=date(2025, 10, 31));
assert_eq!(latest[2], date(2024, 10, 1)..=date(2024, 10, 31));

// Or every october since 2020
let since_2020: DateRanges = octobers.between(date(2020, 1, 1), today).collect();
assert_eq!(since_2020.ranges().len(), 7);

```

## Languages and customization

The supported built-in languages are found in the `languages` folder.
It is easy to add support for new languages or extend existing ones:

```rust

use whenabouts::Language;

// Load a built-in language
let en = Language::for_locale("en").unwrap();

// Extend with a custom phrase
let en_custom = en.extend(r#"
  [phrases]
  "winter holiday{,s}" = "week(8..9)"
"#).unwrap();

// Use the language to parse
let parsed = en_custom.parse("pictures from the winter holiday");
assert_eq!(parsed.rest(), "pictures");

// Load a fully custom language
let custom = Language::from_toml(r#"
  prepositions = ["in"]
  [phrases]
  "yesteryear" = "ago(year, 1)"
"#).unwrap();

let parsed = custom.parse("songs in yesteryear");
assert_eq!(parsed.rest(), "songs");

```

## Configuration files

To configure a new language, create a TOML file that binds
phrases, units, numbers, prepositions and similar to whenabouts clauses.

```toml
[phrases]
"yesteryear" = "ago(year, 1)"
"<n> <unit> ago" = "ago(<unit>, <n>)"
```

See existing language configurations for useful examples.

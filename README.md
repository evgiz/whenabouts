# Whenabouts

Fast natural-language date parser for search queries.

Automatically detect phrases such as "last week" or "summer of 2023" and convert them into clauses or date ranges.
Fast and easily extensible. Written in Rust.

### [Try the live demo](https://evgiz.github.io/whenabouts/).

[Crate on crates.io](https://crates.io/crates/whenabouts) - [Docs on docs.rs](https://docs.rs/whenabouts)

## Installation

```sh
cargo add whenabouts
```

## Basic usage

It is simple to check whether a date is included in a query and extract the semantic part of the text:

```rust

use whenabouts::date;

// Parse a natural language query
let query = "pictures of cats in may";
let parsed = whenabouts::parse(query);

// Assert a date in may is included
let day_in_may = date(2026, 5, 23);
assert!(parsed.contains(day_in_may));

// Get the remaining semantic part of the query
assert_eq!(parsed.rest(), "pictures of cats");

```

## Filtering using date ranges

A typical use case is to filter results by time, e.g. in a database query.

Specific queries, such as "january 2023", only match a single date range. Other phrases, like "in winter" or "on a weekend", repeat indefinitely. To avoid dealing with very large number of date ranges, use the lazy iterator helpers to limit which ones to consider:

- `backward()`: iterate ranges backward in time from today
- `forward()`: iterate ranges forward in time from today
- `between(first, last)`: iterate ranges between two dates, earliest first

All are inclusive. Forward/backward only returns ranges between years 1900-2100, but for certain queries this can still return thousands of date ranges. Make sure to consider this in your filtering logic.

Example:

```rust

use whenabouts::{RangeSet, date};

// The reference point for today
let today = date(2026, 10, 15);

// Parse a natural language query
let query = "hiking photos from october";
let parsed = whenabouts::parse_as_of(query, today);

// Get the 3 latest octobers (including the current one)
let latest: Vec<_> = parsed.backward().take(3).collect();
assert_eq!(latest[0], date(2026, 10, 1)..=date(2026, 10, 15)); // Cut at today
assert_eq!(latest[1], date(2025, 10, 1)..=date(2025, 10, 31));
assert_eq!(latest[2], date(2024, 10, 1)..=date(2024, 10, 31));

// Or every october since 2020
let since_2020: RangeSet = parsed.between(date(2020, 1, 1), today).collect();
assert_eq!(since_2020.len(), 7);

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

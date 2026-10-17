//! The browser demo's way into whenabouts, without any bindings generator.
//!
//! The page writes `"<locale>\n<query>"` into memory from [`alloc`], calls
//! [`parse`], and reads the JSON it returns from [`output`].

use std::cell::RefCell;

use serde::Serialize;
use whenabouts::{Date, Language};

thread_local! {
    static OUTPUT: RefCell<String> = const { RefCell::new(String::new()) };
}

#[derive(Serialize)]
struct Output {
    error: Option<String>,
    phrases: Vec<PhraseOutput>,
}

#[derive(Serialize)]
struct PhraseOutput {
    text: String,
    /// UTF-16 offsets, as JavaScript counts.
    start: usize,
    end: usize,
    clause: String,
    /// The latest range up to today.
    latest: Option<[String; 2]>,
    /// From the first range to the last, or `None` if the phrase repeats forever.
    bounds: Option<[String; 2]>,
    /// The ranges in the years shown.
    ranges: Vec<[String; 2]>,
}

/// Room for `len` bytes of input.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    let mut buffer = Vec::<u8>::with_capacity(len);
    let ptr = buffer.as_mut_ptr();
    std::mem::forget(buffer);
    ptr
}

/// Frees memory from [`alloc`].
///
/// # Safety
///
/// `ptr` and `len` must come from one call to [`alloc`].
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: *mut u8, len: usize) {
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len) });
}

/// Where the JSON from the last [`parse`] is.
#[unsafe(no_mangle)]
pub extern "C" fn output() -> *const u8 {
    OUTPUT.with_borrow(|output| output.as_ptr())
}

/// Parses `"<locale>\n<query>"` as seen on `today`, with the ranges from
/// `first_year` to `last_year`.
/// Returns the length of the JSON at [`output`].
///
/// # Safety
///
/// `ptr` must point to `len` bytes of UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse(
    ptr: *const u8,
    len: usize,
    y: i32,
    m: i32,
    d: i32,
    first_year: i32,
    last_year: i32,
) -> usize {
    let input = unsafe { std::slice::from_raw_parts(ptr, len) };
    let input = std::str::from_utf8(input).unwrap_or("");
    let (locale, query) = input.split_once('\n').unwrap_or(("en", input));

    let output = run(locale, query, y, m, d, first_year, last_year);
    let json = serde_json::to_string(&output).unwrap();
    OUTPUT.with_borrow_mut(|output| *output = json);
    OUTPUT.with_borrow(String::len)
}

fn run(
    locale: &str,
    query: &str,
    y: i32,
    m: i32,
    d: i32,
    first_year: i32,
    last_year: i32,
) -> Output {
    let date = |y: i32, m: i32, d: i32| Date::new(y as i16, m as u8, d as u8);
    let (Some(today), Some(first), Some(last)) = (
        date(y, m, d),
        date(first_year, 1, 1),
        date(last_year, 12, 31),
    ) else {
        return error("invalid date");
    };
    let language = match Language::for_locale(locale) {
        Ok(language) => language,
        Err(e) => return error(&e.to_string()),
    };

    let parsed = language.parse(query);
    let utf16 = |bytes: usize| query[..bytes].encode_utf16().count();
    let pair = |range: std::ops::RangeInclusive<Date>| {
        [range.start().to_string(), range.end().to_string()]
    };
    let phrases = parsed
        .phrases()
        .iter()
        .map(|phrase| {
            let occurrences = phrase.clause().occurrences_as_of(today);
            PhraseOutput {
                text: phrase.text().to_string(),
                start: utf16(phrase.span().start),
                end: utf16(phrase.span().end),
                clause: phrase.clause().to_string(),
                latest: occurrences.backward().next().map(pair),
                bounds: phrase.clause().bounds_as_of(today).map(pair),
                ranges: occurrences.between(first, last).map(pair).collect(),
            }
        })
        .collect();

    Output {
        error: None,
        phrases,
    }
}

fn error(message: &str) -> Output {
    Output {
        error: Some(message.to_string()),
        phrases: Vec::new(),
    }
}

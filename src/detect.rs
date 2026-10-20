use std::ops::Range;

use crate::clause::Clause;
use crate::language::Language;
use crate::tokenize::{Token, tokenize};

/// A time expression found by [`Language::detect`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Detection {
    /// Byte range of the expression in the query.
    pub(crate) span: Range<usize>,
    pub(crate) clause: Clause,
}

impl Language {
    /// Finds the time expressions in `query`.
    pub(crate) fn detect(&self, query: &str) -> Vec<Detection> {
        let tokens = tokenize(query);

        let found = find_phrases(self, &tokens);
        let mut found = combine_neighbours(self, &tokens, found);
        include_prepositions(self, &tokens, &mut found);

        found
            .into_iter()
            .filter(|found| !found.partial && found.clause.is_specific())
            .map(|found| Detection {
                span: tokens[found.start].span.start..tokens[found.end - 1].span.end,
                clause: found.clause,
            })
            .collect()
    }
}

/// A phrase found in the tokens.
struct Found {
    /// Token range.
    start: usize,
    end: usize,
    clause: Clause,
    partial: bool,
}

/// Goes through the tokens from left to right, taking the longest phrase at each point.
fn find_phrases(language: &Language, tokens: &[Token]) -> Vec<Found> {
    let mut found = Vec::new();
    let mut start = 0;
    while start < tokens.len() {
        let longest = language
            .phrases_from(&tokens[start].text)
            .filter_map(|phrase| {
                let (len, clause) = phrase.matches(language, tokens, start)?;
                Some(Found {
                    start,
                    end: start + len,
                    clause,
                    partial: phrase.partial,
                })
            })
            // On a tie the last phrase wins, so added phrases override earlier ones.
            .max_by_key(|found| found.end);

        match longest {
            Some(longest) => {
                start = longest.end;
                found.push(longest);
            }
            None => start += 1,
        }
    }
    found
}

/// Combines times next to each other: "mai" + "i fjor" → May last year.
/// Order doesn't matter, so "i fjor mai" works too.
fn combine_neighbours(language: &Language, tokens: &[Token], found: Vec<Found>) -> Vec<Found> {
    let mut combined: Vec<Found> = Vec::new();
    for next in found {
        if let Some(previous) = combined.last_mut() {
            let between = &tokens[previous.end..next.start];
            let close = between.len() <= 1
                && between
                    .iter()
                    .all(|token| language.joiners.contains(&token.text));

            if close && let Some(clause) = previous.clause.combine(&next.clause) {
                previous.end = next.end;
                previous.clause = clause;
                previous.partial = false;
                continue;
            }
        }
        combined.push(next);
    }
    combined
}

/// Includes the longest preposition before a time in its span: "2019" → "fra 2019",
/// "summer" → "in the summer".
fn include_prepositions(language: &Language, tokens: &[Token], found: &mut [Found]) {
    let mut free_from = 0;
    for found in found {
        let before = &tokens[free_from..found.start];
        let longest = language
            .prepositions
            .iter()
            .filter(|words| {
                words.len() <= before.len()
                    && before[before.len() - words.len()..]
                        .iter()
                        .zip(words.iter())
                        .all(|(token, word)| token.text == *word)
            })
            .map(Vec::len)
            .max();
        found.start -= longest.unwrap_or(0);
        free_from = found.end;
    }
}

use std::ops::Range;

/// A word, number or single punctuation character, in lowercase. Numbers and
/// letters are split apart: "17th" is "17" and "th".
pub(crate) struct Token {
    pub(crate) text: String,
    /// Byte range in the original text.
    pub(crate) span: Range<usize>,
}

pub(crate) fn tokenize(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.char_indices().peekable();

    while let Some((start, c)) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        let mut end = start + c.len_utf8();
        if c.is_alphanumeric() {
            while let Some(&(i, next)) = chars.peek() {
                if !next.is_alphanumeric() || next.is_numeric() != c.is_numeric() {
                    break;
                }
                end = i + next.len_utf8();
                chars.next();
            }
        }
        tokens.push(Token {
            text: text[start..end].to_lowercase(),
            span: start..end,
        });
    }
    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_words_numbers_and_punctuation() {
        let tokens = tokenize("Påske 17.05, i fjor");
        let texts: Vec<_> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["påske", "17", ".", "05", ",", "i", "fjor"]);
        assert_eq!(tokens[0].span, 0..6);
    }

    #[test]
    fn splits_numbers_from_letters() {
        let tokens = tokenize("17th 3yrs");
        let texts: Vec<_> = tokens.iter().map(|t| t.text.as_str()).collect();
        assert_eq!(texts, ["17", "th", "3", "yrs"]);
        assert_eq!(tokens[1].span, 2..4);
    }
}

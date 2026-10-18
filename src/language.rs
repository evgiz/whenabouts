//! Language files: the words and phrases of a language, and what they mean.
//! The format is documented on [`Language::from_toml`].

use std::collections::HashMap;
use std::fmt;
use std::sync::LazyLock;

use crate::clause::{Clause, ParseClauseError, Unit, Weekday};
use crate::date;
use crate::parsed::UnsupportedLocale;
use crate::tokenize::{Token, tokenize};

/// The words and phrases of a language, to find time expressions in queries
/// with [`Language::parse`].
///
/// Use a bundled language with [`Language::for_locale`], add your own phrases
/// to it with [`Language::extend`], or write a whole language with
/// [`Language::from_toml`].
#[derive(Clone)]
pub struct Language {
    /// Words that may come before a time, like "i" in "i mai", each as its tokens.
    pub(crate) prepositions: Vec<Vec<String>>,
    /// Words allowed between two times that belong together, like "i" in "mai i 2020".
    pub(crate) joiners: Vec<String>,
    pub(crate) patterns: Vec<Pattern>,
    numbers: HashMap<String, u32>,
    units: HashMap<String, Unit>,
    /// The day weeks back from now start on, like "last week".
    week_start: Weekday,
}

/// A pattern of words and slots, and the clause it means.
#[derive(Clone)]
pub(crate) struct Pattern {
    elements: Vec<Element>,
    clause: String,
    /// Only counts next to another time.
    pub(crate) partial: bool,
}

#[derive(Clone)]
enum Element {
    Word(String),
    Slot(Slot),
}

/// A placeholder in a pattern, written `<name>`.
#[derive(Clone, Copy)]
enum Slot {
    /// A count, as digits or a word: "3", "tre".
    N,
    Day,
    Month,
    Year,
    /// A two-digit year: "23" → 2023, "95" → 1995.
    ShortYear,
    Unit,
}

/// The language file, as written. Tables keep their order, so later phrases
/// win ties the same way every time.
struct File {
    week_start: Option<String>,
    prepositions: Vec<String>,
    joiners: Vec<String>,
    numbers: Vec<(String, u32)>,
    units: Vec<(String, String)>,
    phrases: Vec<(String, String)>,
    partial: Vec<(String, String)>,
}

/// A mistake in a language file, from [`Language::from_toml`] or
/// [`Language::extend`], like `phrase "jul": invalid clause part: "month(13)"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguageError {
    message: String,
}

impl Language {
    /// The bundled language for a locale like `"nb"`, `"nb-NO"` or `"nb_NO"`.
    ///
    /// ```
    /// use whenabouts::Language;
    ///
    /// let nb = Language::for_locale("nb-NO")?;
    /// assert_eq!(nb.parse("bilder fra i fjor").rest(), "bilder");
    ///
    /// assert!(Language::for_locale("xx").is_err());
    /// # Ok::<(), whenabouts::UnsupportedLocale>(())
    /// ```
    pub fn for_locale(locale: &str) -> Result<&'static Language, UnsupportedLocale> {
        let language = locale.split(['-', '_']).next().unwrap_or("").to_lowercase();
        match language.as_str() {
            "nb" | "no" => Ok(&NB),
            "en" => Ok(&EN),
            _ => Err(UnsupportedLocale::new(locale)),
        }
    }

    /// The bundled English, used by [`parse`](crate::parse).
    pub(crate) fn english() -> &'static Language {
        &EN
    }

    /// Reads a language file.
    ///
    /// ```toml
    /// week_start = "mon"                # the day weeks start on, for "forrige uke"; mon by default
    /// prepositions = ["i", "i løpet av"] # may come before a time: "i mai", "i løpet av 2019"
    /// joiners = ["i", ","]              # allowed between two times that belong together: "mai i 2020"
    ///
    /// [numbers]                         # words read by the <n> slot
    /// "{en,ett}" = 1
    ///
    /// [units]                           # words read by the <unit> slot: day, week, month or year
    /// "uke{,n,a}" = "week"
    ///
    /// [phrases]                         # pattern = clause
    /// "jul{,en,a}" = "date(12-24..12-26)"
    /// "for <n> <unit> siden" = "ago(<unit>, <n>)"
    ///
    /// [partial]                         # only counts next to another time: "mai" in "mai 2020"
    /// "mai" = "month(5)"
    /// ```
    ///
    /// Every section may be left out. In patterns, `jul{,en,a}` means "jul",
    /// "julen" and "jula", and the slots are `<n>`, `<day>`, `<month>`, `<year>`,
    /// `<yy>` (two-digit year) and `<unit>`. Clauses are written
    /// `<period> from <anchor>`; see [`Clause`].
    ///
    /// ```
    /// use whenabouts::Language;
    ///
    /// let mini = Language::from_toml(r#"
    ///     prepositions = ["in"]
    ///     [phrases]
    ///     "{christmas,xmas}" = "date(12-25)"
    ///     "<year>" = "year(<year>)"
    /// "#)?;
    /// let parsed = mini.parse("presents in xmas 2019");
    /// assert_eq!(parsed.phrases()[0].clause().to_string(), "date(12-25) from year(2019)");
    /// # Ok::<(), whenabouts::LanguageError>(())
    /// ```
    pub fn from_toml(source: &str) -> Result<Language, LanguageError> {
        Language::empty().extend(source)
    }

    /// This language with a language file added, like your own phrases on top
    /// of a bundled language. Added phrases win over existing ones that match
    /// as much. The file is written as for [`Language::from_toml`].
    ///
    /// ```
    /// use whenabouts::Language;
    ///
    /// let custom = Language::for_locale("nb").unwrap().extend(r#"
    ///     [phrases]
    ///     "hyttetur{,en}" = "month(7)"
    /// "#)?;
    ///
    /// let parsed = custom.parse("bilder fra hytteturen i fjor");
    /// assert_eq!(parsed.phrases()[0].clause().to_string(), "month(7) from ago(year, 1)");
    /// # Ok::<(), whenabouts::LanguageError>(())
    /// ```
    pub fn extend(&self, source: &str) -> Result<Language, LanguageError> {
        let file = File::read(source)?;
        let mut language = self.clone();
        language.add(file)?;
        Ok(language)
    }

    /// A language without any words.
    fn empty() -> Language {
        Language {
            prepositions: Vec::new(),
            joiners: Vec::new(),
            patterns: Vec::new(),
            numbers: HashMap::new(),
            units: HashMap::new(),
            week_start: Weekday::Mon,
        }
    }

    /// Adds the contents of a language file.
    fn add(&mut self, file: File) -> Result<(), LanguageError> {
        if let Some(start) = file.week_start {
            self.week_start = Weekday::from_name(&start).ok_or_else(|| {
                LanguageError::new(format!("week_start: unknown weekday {start:?}"))
            })?;
        }
        let prepositions = file.prepositions.iter().map(|words| {
            let tokens: Vec<_> = tokenize(words)
                .into_iter()
                .map(|token| token.text)
                .collect();
            (!tokens.is_empty())
                .then_some(tokens)
                .ok_or_else(|| LanguageError::new(format!("preposition {words:?}: empty")))
        });
        self.prepositions
            .extend(prepositions.collect::<Result<Vec<_>, _>>()?);
        self.joiners.extend(file.joiners);

        for (section, partial) in [(file.phrases, false), (file.partial, true)] {
            for (written, clause) in section {
                let error =
                    |error: String| LanguageError::new(format!("phrase {written:?}: {error}"));
                for pattern in expand(&written)? {
                    let compiled = Pattern {
                        elements: parse_pattern(&pattern).map_err(error)?,
                        clause: clause.clone(),
                        partial,
                    };
                    compiled
                        .fill(&example_values())
                        .map_err(|e| error(e.to_string()))?;
                    self.patterns.push(compiled);
                }
            }
        }

        for (words, unit) in file.units {
            let unit = Unit::from_name(&unit).ok_or_else(|| {
                LanguageError::new(format!("unit {words:?}: unknown unit {unit:?}"))
            })?;
            for word in expand(&words)? {
                self.units.insert(word, unit);
            }
        }

        for (words, n) in file.numbers {
            for word in expand(&words)? {
                self.numbers.insert(word, n);
            }
        }
        Ok(())
    }
}

impl File {
    /// Reads the file. Sections it doesn't know are left alone.
    fn read(source: &str) -> Result<File, LanguageError> {
        let table: toml::Table = source
            .parse()
            .map_err(|error: toml::de::Error| LanguageError::new(error.to_string()))?;
        let text = |value: &toml::Value| value.as_str().map(str::to_string);
        let count = |value: &toml::Value| value.as_integer().and_then(|n| u32::try_from(n).ok());
        Ok(File {
            week_start: table
                .get("week_start")
                .map(|value| text(value).ok_or_else(|| wrong_type("week_start", "a string", value)))
                .transpose()?,
            prepositions: list(&table, "prepositions")?,
            joiners: list(&table, "joiners")?,
            numbers: section(&table, "numbers", "a whole number", count)?,
            units: section(&table, "units", "a string", text)?,
            phrases: section(&table, "phrases", "a string", text)?,
            partial: section(&table, "partial", "a string", text)?,
        })
    }
}

/// A list of strings, or an empty one if it's left out.
fn list(table: &toml::Table, key: &str) -> Result<Vec<String>, LanguageError> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    let items = value
        .as_array()
        .ok_or_else(|| wrong_type(key, "a list", value))?;
    items
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_string)
                .ok_or_else(|| wrong_type(key, "a list of strings", item))
        })
        .collect()
}

/// The entries of a table, in order, or none if it's left out.
fn section<T>(
    table: &toml::Table,
    key: &str,
    expected: &str,
    read: impl Fn(&toml::Value) -> Option<T>,
) -> Result<Vec<(String, T)>, LanguageError> {
    let Some(value) = table.get(key) else {
        return Ok(Vec::new());
    };
    let entries = value
        .as_table()
        .ok_or_else(|| wrong_type(key, "a table", value))?;
    entries
        .iter()
        .map(|(name, value)| {
            let read = read(value)
                .ok_or_else(|| wrong_type(&format!("{key} {name:?}"), expected, value))?;
            Ok((name.clone(), read))
        })
        .collect()
}

fn wrong_type(key: &str, expected: &str, value: &toml::Value) -> LanguageError {
    LanguageError::new(format!(
        "{key}: expected {expected}, found {}",
        value.type_str()
    ))
}

static EN: LazyLock<Language> =
    LazyLock::new(|| bundled("en", include_str!("../languages/en.toml")));
static NB: LazyLock<Language> =
    LazyLock::new(|| bundled("nb", include_str!("../languages/nb.toml")));

/// Reads a language file that comes with the library. Mistakes in it are bugs,
/// and the tests catch them.
fn bundled(name: &str, source: &str) -> Language {
    Language::from_toml(source).unwrap_or_else(|error| panic!("languages/{name}.toml: {error}"))
}

impl LanguageError {
    fn new(message: String) -> LanguageError {
        LanguageError { message }
    }
}

impl fmt::Display for LanguageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for LanguageError {}

impl fmt::Debug for Language {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Language").finish_non_exhaustive()
    }
}

impl Pattern {
    /// Matches the phrase against the tokens at `start`.
    /// Returns the number of tokens matched and the clause.
    pub(crate) fn matches(
        &self,
        language: &Language,
        tokens: &[Token],
        start: usize,
    ) -> Option<(usize, Clause)> {
        let mut values = Vec::new();
        for (i, element) in self.elements.iter().enumerate() {
            let token = tokens.get(start + i)?;
            match element {
                Element::Word(word) if *word == token.text => {}
                Element::Word(_) => return None,
                Element::Slot(slot) => values.push((*slot, slot.read(language, &token.text)?)),
            }
        }
        let clause = self
            .fill(&values)
            .ok()?
            .with_week_start(language.week_start);
        Some((self.elements.len(), clause))
    }

    /// Fills the slot values into the clause, e.g. `ago(<unit>, <n>)` → `ago(year, 3)`.
    fn fill(&self, values: &[(Slot, String)]) -> Result<Clause, ParseClauseError> {
        let mut clause = self.clause.clone();
        for (slot, value) in values {
            clause = clause.replace(&format!("<{}>", slot.name()), value);
        }
        clause.parse()
    }
}

impl Slot {
    fn name(self) -> &'static str {
        match self {
            Slot::N => "n",
            Slot::Day => "day",
            Slot::Month => "month",
            Slot::Year => "year",
            Slot::ShortYear => "yy",
            Slot::Unit => "unit",
        }
    }

    fn from_name(name: &str) -> Option<Slot> {
        use Slot::*;
        [N, Day, Month, Year, ShortYear, Unit]
            .into_iter()
            .find(|slot| slot.name() == name)
    }

    /// Reads a token into this slot, if it fits.
    fn read(self, language: &Language, text: &str) -> Option<String> {
        let digits = || -> Option<u32> {
            text.chars()
                .all(|c| c.is_ascii_digit())
                .then(|| text.parse().ok())?
        };
        let value = match self {
            Slot::N => digits().or_else(|| language.numbers.get(text).copied())?,
            Slot::Day => digits().filter(|_| text.len() <= 2)?,
            Slot::Month => digits().filter(|_| text.len() <= 2)?,
            Slot::Year => {
                let years = date::FIRST.year() as u32..=date::LAST.year() as u32;
                digits().filter(|n| text.len() == 4 && years.contains(n))?
            }
            Slot::ShortYear => match digits().filter(|_| text.len() == 2)? {
                n @ 0..70 => 2000 + n,
                n => 1900 + n,
            },
            Slot::Unit => return language.units.get(text).map(|unit| unit.name().to_string()),
        };
        Some(value.to_string())
    }
}

/// Values used to check that every phrase in a file gives a valid clause.
fn example_values() -> Vec<(Slot, String)> {
    [
        (Slot::N, "1"),
        (Slot::Day, "1"),
        (Slot::Month, "1"),
        (Slot::Year, "2000"),
        (Slot::ShortYear, "2000"),
        (Slot::Unit, "year"),
    ]
    .into_iter()
    .map(|(slot, value)| (slot, value.to_string()))
    .collect()
}

/// Splits a pattern into words and `<slots>`.
fn parse_pattern(pattern: &str) -> Result<Vec<Element>, String> {
    let tokens = tokenize(pattern);
    if tokens.is_empty() {
        return Err("empty pattern".to_string());
    }
    let mut elements = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i].text == "<" {
            let slot = tokens
                .get(i + 1)
                .and_then(|name| Slot::from_name(&name.text))
                .filter(|_| tokens.get(i + 2).is_some_and(|close| close.text == ">"))
                .ok_or_else(|| format!("invalid slot in {pattern:?}"))?;
            elements.push(Element::Slot(slot));
            i += 3;
        } else {
            elements.push(Element::Word(tokens[i].text.clone()));
            i += 1;
        }
    }
    Ok(elements)
}

/// Expands word forms: `jul{,en,a}` → `jul`, `julen`, `jula`.
fn expand(text: &str) -> Result<Vec<String>, LanguageError> {
    let Some(open) = text.find('{') else {
        return Ok(vec![text.to_string()]);
    };
    let close = text[open..]
        .find('}')
        .ok_or_else(|| LanguageError::new(format!("missing '}}' in {text:?}")))?;
    let (before, after) = (&text[..open], &text[open + close + 1..]);

    let mut expanded = Vec::new();
    for form in text[open + 1..open + close].split(',') {
        expanded.extend(expand(&format!("{before}{form}{after}"))?);
    }
    Ok(expanded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_word_forms() {
        assert_eq!(expand("jul{,en,a}").unwrap(), ["jul", "julen", "jula"]);
        assert_eq!(
            expand("{jul,påske}{,n}").unwrap(),
            ["jul", "juln", "påske", "påsken"]
        );
    }

    #[test]
    fn reads_a_language_file() {
        let language = Language::from_toml(
            r#"
            [phrases]
            "hyttetur{,en}" = "month(7)"
        "#,
        )
        .unwrap();
        assert_eq!(language.detect("bilder fra hytteturen").len(), 1);
    }

    #[test]
    fn added_phrases_win() {
        let nb = Language::for_locale("nb").unwrap();
        let custom = nb.extend("[phrases]\n\"jul\" = \"month(12)\"").unwrap();
        assert_eq!(nb.detect("jul")[0].clause.to_string(), "date(12-24..12-26)");
        assert_eq!(custom.detect("jul")[0].clause.to_string(), "month(12)");
    }

    #[test]
    fn extends_every_section() {
        let nb = Language::for_locale("nb").unwrap();
        let custom = nb
            .extend(
                r#"
                prepositions = ["etter"]
                joiners = ["og"]

                [numbers]
                "dusin" = 12

                [units]
                "yr" = "year"

                [partial]
                "sommarn" = "month(6..8)"
                "#,
            )
            .unwrap();

        for (query, expected) in [
            ("etter jul", Some(("etter jul", "date(12-24..12-26)"))),
            (
                "juni og 2020",
                Some(("juni og 2020", "month(6) from year(2020)")),
            ),
            (
                "for dusin yr siden",
                Some(("for dusin yr siden", "ago(year, 12)")),
            ),
            (
                "sommarn 2020",
                Some(("sommarn 2020", "month(6..8) from year(2020)")),
            ),
            ("sommarn", None),
        ] {
            let found = custom.detect(query);
            let actual = found
                .first()
                .map(|found| (&query[found.span.clone()], found.clause.to_string()));
            let expected = expected.map(|(text, clause)| (text, clause.to_string()));
            if expected.is_some() {
                assert_ne!(nb.detect(query), found, "needs the extension: {query}");
            }
            assert_eq!(actual, expected, "{query}");
        }
    }

    #[test]
    fn reports_mistakes() {
        for (source, message) in [
            ("[phrases", "unclosed table"),
            (
                "[phrases]\n\"jul\" = \"month(13)\"",
                "phrase \"jul\": invalid clause part: \"month(13)\"",
            ),
            (
                "[phrases]\n\"{,}\" = \"month(5)\"",
                "phrase \"{,}\": empty pattern",
            ),
            (
                "[phrases]\n\"for <x> siden\" = \"month(5)\"",
                "phrase \"for <x> siden\": invalid slot in \"for <x> siden\"",
            ),
            (
                "[phrases]\n\"jul{en\" = \"month(5)\"",
                "missing '}' in \"jul{en\"",
            ),
            (
                "[units]\n\"tiår\" = \"decade\"",
                "unit \"tiår\": unknown unit \"decade\"",
            ),
            (
                "[phrases]\n\"jul\" = 5",
                "phrases \"jul\": expected a string, found integer",
            ),
            (
                "[numbers]\n\"en\" = -1",
                "numbers \"en\": expected a whole number, found integer",
            ),
            (
                "prepositions = \"i\"",
                "prepositions: expected a list, found string",
            ),
            ("phrases = 3", "phrases: expected a table, found integer"),
        ] {
            let error = Language::from_toml(source).unwrap_err().to_string();
            assert!(error.contains(message), "{source:?}: {error}");
        }
    }

    #[test]
    fn reads_the_week_start() {
        let sunday = Language::from_toml(
            r#"
            week_start = "sun"
            [phrases]
            "last week" = "ago(week, 1)"
        "#,
        )
        .unwrap();
        assert_eq!(
            sunday.detect("last week")[0].clause.to_string(),
            "ago(week(sun), 1)"
        );
        assert!(Language::from_toml("week_start = \"funday\"").is_err());
    }

    #[test]
    fn loads_bundled_languages() {
        assert!(Language::for_locale("nb").is_ok());
        assert!(Language::for_locale("nb-NO").is_ok());
        assert!(Language::for_locale("xx").is_err());
    }

    #[test]
    fn reads_two_digit_years() {
        let language = Language::for_locale("nb").unwrap();
        assert_eq!(
            Slot::ShortYear.read(language, "23").as_deref(),
            Some("2023")
        );
        assert_eq!(
            Slot::ShortYear.read(language, "95").as_deref(),
            Some("1995")
        );
        assert_eq!(Slot::ShortYear.read(language, "2023"), None);
    }
}

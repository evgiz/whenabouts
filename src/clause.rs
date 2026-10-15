use std::fmt;
use std::str::FromStr;

/// Which times match: a period, and an anchor that picks occurrences of it.
///
/// - The period is parts joined by `&`: a time is in the period when it matches
///   every part. Each unbroken stretch of the period is one occurrence, like
///   every 17 May, or every winter from December to February.
/// - The anchor picks the occurrences that start inside it. Weeks follow ISO
///   years: week 1 of 2025 starts on 30 December 2024.
///
/// Printed (and written in language files) as `<period> from <anchor>`:
/// `month(12..2) from year(2024)` is December 2024 to February 2025.
/// Without an anchor, every occurrence matches: `month(5)` is any May.
/// Without a period, the anchor is the time itself: `ago(year, 1)` is last year.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Clause {
    parts: Vec<Part>,
    anchor: Option<Anchor>,
}

/// One condition in the period of a [`Clause`]. Ranges are inclusive, and wrap
/// around when the start comes after the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Part {
    /// Months of the year, 1–12. `Month(12, 2)` is December to February.
    Month(u8, u8),
    /// Weeks of the year (ISO), 1–53.
    Week(u8, u8),
    /// Days relative to Easter Sunday. `Easter(-2, -2)` is Good Friday.
    Easter(i16, i16),
    /// Dates of the year. `date(12-31..01-01)` is New Year's Eve to New Year's Day.
    Date(MonthDay, MonthDay),
    /// Days of the month, 1–31.
    Day(u8, u8),
    /// Days of the week. `Weekday(Sat, Sun)` is the weekend.
    Weekday(Weekday, Weekday),
}

/// Which occurrences of a [`Clause`]'s period match: those that start inside the anchor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Anchor {
    /// A calendar year.
    Year(i16),
    /// The unit `n` steps back from now. `Ago(Year, 1)` is last year, `Ago(Day, 0)` is today.
    /// Weeks start on Monday, or the day given: `ago(week(sun), 1)`.
    Ago(Unit, u32),
    /// The last `n` units up to now. `Last(Month, 3)` is the last three months.
    Last(Unit, u32),
    /// `back(0)` is the latest occurrence that has started, even if it is under
    /// way. `back(n)` is the `n`-th latest that has ended: in October,
    /// `month(6..8) from back(1)` is this year's summer, and in July last year's.
    Back(u32),
}

/// A date without a year, printed `12-31`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct MonthDay {
    /// 1–12.
    pub(crate) month: u8,
    /// 1–31.
    pub(crate) day: u8,
}

impl MonthDay {
    fn is_valid(self) -> bool {
        const DAYS: [u8; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        (1..=12).contains(&self.month) && (1..=DAYS[self.month as usize - 1]).contains(&self.day)
    }
}

/// A clause that couldn't be read, from [`Clause`]'s `FromStr`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseClauseError {
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Unit {
    Day,
    /// A week starting on the given day.
    Week(Weekday),
    Month,
    Year,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Weekday {
    Mon,
    Tue,
    Wed,
    Thu,
    Fri,
    Sat,
    Sun,
}

impl Clause {
    /// The parts of the period, in a fixed order. Empty when the clause is only an anchor.
    pub(crate) fn parts(&self) -> &[Part] {
        &self.parts
    }

    pub(crate) fn anchor(&self) -> Option<Anchor> {
        self.anchor
    }

    /// The clause, or `None` if it pins down a level twice or has an impossible date.
    pub(crate) fn new(mut parts: Vec<Part>, mut anchor: Option<Anchor>) -> Option<Clause> {
        // The last n weeks are the last 7n days, whichever day weeks start on.
        if let Some(Anchor::Last(Unit::Week(_), n)) = anchor {
            anchor = Some(Anchor::Last(Unit::WEEK, n));
        }
        parts.sort();
        parts.dedup();
        // One month and its days is a date: "17. mai" → date(05-17).
        let month = parts.iter().find_map(|part| match *part {
            Part::Month(a, b) if a == b => Some(a),
            _ => None,
        });
        let days = parts.iter().find_map(|part| match *part {
            Part::Day(from, to) => Some((from, to)),
            _ => None,
        });
        if let (Some(month), Some((from, to))) = (month, days) {
            parts.retain(|part| !matches!(part, Part::Month(..) | Part::Day(..)));
            parts.push(Part::Date(
                MonthDay { month, day: from },
                MonthDay { month, day: to },
            ));
            parts.sort();
        }
        let clause = Clause { parts, anchor };
        clause.is_valid().then_some(clause)
    }

    fn is_valid(&self) -> bool {
        let mut levels = self.anchor.map_or(0, Anchor::levels);
        for part in &self.parts {
            if !part.is_valid() || levels & part.levels() != 0 {
                return false;
            }
            levels |= part.levels();
        }
        true
    }

    /// Combines two clauses, unless they describe the same thing.
    /// "May" and "last year" combine; "May" and "December" do not.
    pub(crate) fn combine(&self, other: &Clause) -> Option<Clause> {
        let first = self.latest_gives_way_to(other);
        let second = other.latest_gives_way_to(self);
        if first.levels() & second.levels() != 0 {
            return None;
        }
        let parts = [first.parts, second.parts].concat();
        Clause::new(parts, first.anchor.or(second.anchor))
    }

    /// Whether the clause says enough. A day of the month needs a month or a
    /// weekday: "3." or "3. i fjor" says too little, "friday the 13th" enough.
    /// "The latest" needs something to be the latest of.
    pub(crate) fn is_specific(&self) -> bool {
        let has_day = self.parts.iter().any(|part| matches!(part, Part::Day(..)));
        let only_back = self.parts.is_empty() && matches!(self.anchor, Some(Anchor::Back(_)));
        (!has_day || self.levels() & (MONTH | WEEKDAY) != 0) && !only_back
    }

    /// The clause, with weeks back from now starting on `start`.
    pub(crate) fn with_week_start(mut self, start: Weekday) -> Clause {
        if let Some(Anchor::Ago(Unit::Week(_), n)) = self.anchor {
            self.anchor = Some(Anchor::Ago(Unit::Week(start), n));
        }
        self
    }

    /// The clause, without "the latest" if the other clause has another anchor:
    /// "i sommer" + "2024" is summer 2024, not the latest summer.
    fn latest_gives_way_to(&self, other: &Clause) -> Clause {
        let mut clause = self.clone();
        let latest = Some(Anchor::Back(0));
        if clause.anchor == latest && other.anchor.is_some() && other.anchor != latest {
            clause.anchor = None;
        }
        clause
    }

    fn levels(&self) -> u8 {
        let parts = self.parts.iter().map(Part::levels);
        let anchor = self.anchor.map(Anchor::levels);
        parts.chain(anchor).fold(0, |levels, part| levels | part)
    }
}

// Which levels of the calendar a part or anchor pins down. Two clauses can only
// be combined if they don't pin down the same level. Every anchor pins down the
// year, so a clause has at most one anchor. The weekday is a level of its own,
// so it narrows down any date: "friday the 13th", "thanksgiving" (a Thursday
// in 22–28 November).
const YEAR: u8 = 1;
const MONTH: u8 = 2;
const DAY: u8 = 4;
const WEEKDAY: u8 = 8;

impl Part {
    fn levels(&self) -> u8 {
        match self {
            Part::Month(..) => MONTH,
            Part::Week(..) | Part::Easter(..) | Part::Date(..) => MONTH | DAY,
            Part::Day(..) => DAY,
            Part::Weekday(..) => WEEKDAY,
        }
    }

    fn is_valid(&self) -> bool {
        match *self {
            Part::Month(a, b) => (1..=12).contains(&a) && (1..=12).contains(&b),
            Part::Week(a, b) => (1..=53).contains(&a) && (1..=53).contains(&b),
            Part::Day(a, b) => (1..=31).contains(&a) && (1..=31).contains(&b),
            Part::Date(a, b) => a.is_valid() && b.is_valid(),
            _ => true,
        }
    }
}

impl Anchor {
    fn levels(self) -> u8 {
        match self {
            Anchor::Year(_) | Anchor::Back(_) | Anchor::Ago(Unit::Year, _) => YEAR,
            Anchor::Ago(Unit::Month | Unit::Week(_), _) => YEAR | MONTH,
            Anchor::Ago(Unit::Day, _) | Anchor::Last(..) => YEAR | MONTH | DAY,
        }
    }

    fn is_valid(self) -> bool {
        !matches!(self, Anchor::Last(_, 0))
    }
}

impl Unit {
    /// A week starting on Monday, as in ISO weeks.
    pub(crate) const WEEK: Unit = Unit::Week(Weekday::Mon);

    pub(crate) fn name(self) -> &'static str {
        match self {
            Unit::Day => "day",
            Unit::Week(_) => "week",
            Unit::Month => "month",
            Unit::Year => "year",
        }
    }

    /// The unit by name, with weeks starting on Monday.
    pub(crate) fn from_name(name: &str) -> Option<Unit> {
        [Unit::Day, Unit::WEEK, Unit::Month, Unit::Year]
            .into_iter()
            .find(|unit| unit.name() == name)
    }
}

impl Weekday {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Weekday::Mon => "mon",
            Weekday::Tue => "tue",
            Weekday::Wed => "wed",
            Weekday::Thu => "thu",
            Weekday::Fri => "fri",
            Weekday::Sat => "sat",
            Weekday::Sun => "sun",
        }
    }

    pub(crate) fn from_name(name: &str) -> Option<Weekday> {
        use Weekday::*;
        [Mon, Tue, Wed, Thu, Fri, Sat, Sun]
            .into_iter()
            .find(|day| day.name() == name)
    }
}

impl fmt::Display for Clause {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for (i, part) in self.parts.iter().enumerate() {
            if i > 0 {
                f.write_str(" & ")?;
            }
            write!(f, "{part}")?;
        }
        match self.anchor {
            Some(anchor) if self.parts.is_empty() => write!(f, "{anchor}"),
            Some(anchor) => write!(f, " from {anchor}"),
            None => Ok(()),
        }
    }
}

impl fmt::Display for Part {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Part::Month(a, b) => write_range(f, "month", a, b),
            Part::Week(a, b) => write_range(f, "week", a, b),
            Part::Easter(a, b) => write_range(f, "easter", a, b),
            Part::Date(a, b) => write_range(f, "date", a, b),
            Part::Day(a, b) => write_range(f, "day", a, b),
            Part::Weekday(a, b) => write_range(f, "weekday", a, b),
        }
    }
}

impl fmt::Display for Anchor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Anchor::Year(year) => write!(f, "year({year})"),
            Anchor::Ago(unit, n) => write!(f, "ago({unit}, {n})"),
            Anchor::Last(unit, n) => write!(f, "last({unit}, {n})"),
            Anchor::Back(n) => write!(f, "back({n})"),
        }
    }
}

fn write_range<T: PartialEq + fmt::Display>(
    f: &mut fmt::Formatter,
    name: &str,
    a: T,
    b: T,
) -> fmt::Result {
    if a == b {
        write!(f, "{name}({a})")
    } else {
        write!(f, "{name}({a}..{b})")
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Unit::Week(start) if start != Weekday::Mon => write!(f, "week({start})"),
            _ => f.write_str(self.name()),
        }
    }
}

impl fmt::Display for Weekday {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// Reads the printed form, e.g. `month(5) from ago(year, 1)`.
impl FromStr for Clause {
    type Err = ParseClauseError;

    fn from_str(text: &str) -> Result<Clause, ParseClauseError> {
        let (period, anchor) = match text.split_once(" from ") {
            Some((period, anchor)) => (Some(period), Some(anchor.parse()?)),
            None => match text.parse::<Anchor>() {
                Ok(anchor) => (None, Some(anchor)),
                Err(_) => (Some(text), None),
            },
        };
        let parts = match period {
            Some(period) => period
                .split('&')
                .map(|part| part.parse())
                .collect::<Result<_, _>>()?,
            None => Vec::new(),
        };
        Clause::new(parts, anchor).ok_or_else(|| ParseClauseError::new("invalid clause", text))
    }
}

impl FromStr for Part {
    type Err = ParseClauseError;

    fn from_str(text: &str) -> Result<Part, ParseClauseError> {
        let error = || ParseClauseError::new("invalid clause part", text);
        let (name, args) = call(text).ok_or_else(error)?;

        let part = match name {
            "month" => range(args)
                .map(|(a, b)| Part::Month(a, b))
                .ok_or_else(error)?,
            "week" => range(args)
                .map(|(a, b)| Part::Week(a, b))
                .ok_or_else(error)?,
            "easter" => range(args)
                .map(|(a, b)| Part::Easter(a, b))
                .ok_or_else(error)?,
            "date" => range(args)
                .map(|(a, b)| Part::Date(a, b))
                .ok_or_else(error)?,
            "day" => range(args)
                .map(|(a, b)| Part::Day(a, b))
                .ok_or_else(error)?,
            "weekday" => range(args)
                .map(|(WeekdayName(a), WeekdayName(b))| Part::Weekday(a, b))
                .ok_or_else(error)?,
            _ => return Err(error()),
        };

        if part.is_valid() {
            Ok(part)
        } else {
            Err(error())
        }
    }
}

impl FromStr for Anchor {
    type Err = ParseClauseError;

    fn from_str(text: &str) -> Result<Anchor, ParseClauseError> {
        let error = || ParseClauseError::new("invalid anchor", text);
        let (name, args) = call(text).ok_or_else(error)?;

        let anchor = match name {
            "year" => Anchor::Year(args.trim().parse().map_err(|_| error())?),
            "ago" => unit_count(args)
                .map(|(unit, n)| Anchor::Ago(unit, n))
                .ok_or_else(error)?,
            "last" => unit_count(args)
                .map(|(unit, n)| Anchor::Last(unit, n))
                .ok_or_else(error)?,
            "back" => Anchor::Back(args.trim().parse().map_err(|_| error())?),
            _ => return Err(error()),
        };

        if anchor.is_valid() {
            Ok(anchor)
        } else {
            Err(error())
        }
    }
}

/// Splits `name(args)` into the name and the arguments.
fn call(text: &str) -> Option<(&str, &str)> {
    let (name, args) = text.trim().strip_suffix(')')?.split_once('(')?;
    Some((name.trim(), args))
}

/// Reads `unit, n`, where the unit may be a week with its start, `week(sun)`.
fn unit_count(text: &str) -> Option<(Unit, u32)> {
    let (unit, n) = text.split_once(',')?;
    let unit = match call(unit) {
        Some(("week", start)) => Unit::Week(Weekday::from_name(start.trim())?),
        _ => Unit::from_name(unit.trim())?,
    };
    Some((unit, n.trim().parse().ok()?))
}

/// A weekday written by name, `sat`.
#[derive(Clone, Copy)]
struct WeekdayName(Weekday);

impl FromStr for WeekdayName {
    type Err = ();

    fn from_str(text: &str) -> Result<WeekdayName, ()> {
        Weekday::from_name(text).map(WeekdayName).ok_or(())
    }
}

impl fmt::Display for MonthDay {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}-{:02}", self.month, self.day)
    }
}

/// Reads `12-31`.
impl FromStr for MonthDay {
    type Err = ParseClauseError;

    fn from_str(text: &str) -> Result<MonthDay, ParseClauseError> {
        let error = || ParseClauseError::new("invalid date", text);
        let (month, day) = text.split_once('-').ok_or_else(error)?;
        Ok(MonthDay {
            month: month.parse().map_err(|_| error())?,
            day: day.parse().map_err(|_| error())?,
        })
    }
}

impl ParseClauseError {
    fn new(what: &str, text: &str) -> ParseClauseError {
        ParseClauseError {
            message: format!("{what}: {text:?}"),
        }
    }
}

impl fmt::Display for ParseClauseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for ParseClauseError {}

/// Reads `a..b`, or a single `a` meaning `a..a`.
fn range<T: FromStr + Copy>(text: &str) -> Option<(T, T)> {
    match text.split_once("..") {
        Some((a, b)) => Some((a.trim().parse().ok()?, b.trim().parse().ok()?)),
        None => {
            let a = text.trim().parse().ok()?;
            Some((a, a))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clause(text: &str) -> Clause {
        text.parse().unwrap()
    }

    #[test]
    fn prints_parts_in_a_fixed_order() {
        let clause = clause("weekday(fri) & month(7) from year(2024)");
        assert_eq!(
            clause.to_string(),
            "month(7) & weekday(fri) from year(2024)"
        );
    }

    #[test]
    fn reads_what_it_prints() {
        for text in [
            "month(6..8) from year(2024)",
            "easter(-7..1)",
            "weekday(fri) from ago(week, 1)",
            "month(6..8) from back(1)",
            "date(12-31..01-01) from year(2020)",
            "week(8..9)",
            "date(12-24)",
            "date(12-24..12-26)",
            "weekday(sat..sun)",
            "last(month, 3)",
            "ago(year, 1)",
            "ago(week(sun), 1)",
            "weekday(fri) from ago(week(sat), 0)",
            "date(11-22..11-28) & weekday(thu)",
            "day(13) & weekday(fri)",
            "week(32) & weekday(fri)",
        ] {
            assert_eq!(clause(text).to_string(), text);
        }
    }

    #[test]
    fn rejects_invalid_clauses() {
        for text in [
            "month(13)",
            "day(0)",
            "ago(decade, 1)",
            "year",
            "weekday(fredag)",
            "last(day, 0)",
            "year(2024) & month(5)",
            "month(5) from month(6)",
            " from year(2024)",
            "month(5) & month(6)",
            "month(5) & date(05-17)",
            "date(02-30)",
            "month(2) & day(31)",
            "ago(week(funday), 1)",
            "weekday(fri) & weekday(sat)",
        ] {
            assert!(text.parse::<Clause>().is_err(), "{text}");
        }
    }

    #[test]
    fn combines_only_different_levels() {
        let may = clause("month(5)");
        let last_year = clause("ago(year, 1)");

        let combined = may.combine(&last_year).unwrap();
        assert_eq!(combined.to_string(), "month(5) from ago(year, 1)");
        assert!(may.combine(&clause("month(12)")).is_none());
        assert!(last_year.combine(&clause("year(2020)")).is_none());
    }

    #[test]
    fn weeks_start_on_monday_unless_told() {
        assert_eq!(clause("ago(week(mon), 1)").to_string(), "ago(week, 1)");
        let sunday = clause("ago(week, 1)").with_week_start(Weekday::Sun);
        assert_eq!(sunday.to_string(), "ago(week(sun), 1)");
        // The last n weeks don't depend on where weeks start.
        assert_eq!(clause("last(week(sun), 2)").to_string(), "last(week, 2)");
    }

    #[test]
    fn month_and_day_become_a_date() {
        assert_eq!(clause("month(5) & day(17)").to_string(), "date(05-17)");
    }

    #[test]
    fn latest_gives_way_to_another_anchor() {
        let combined = clause("month(6..8) from back(0)").combine(&clause("year(2024)"));
        assert_eq!(combined.unwrap().to_string(), "month(6..8) from year(2024)");
    }

    #[test]
    fn says_enough_to_count() {
        for (text, specific) in [
            ("day(3)", false),
            ("day(3) from ago(year, 1)", false),
            ("day(3) from ago(month, 1)", true),
            ("date(05-03)", true),
            ("year(2020)", true),
            ("back(0)", false),
            ("month(5) from back(0)", true),
            ("day(13) & weekday(fri)", true),
        ] {
            assert_eq!(clause(text).is_specific(), specific, "{text}");
        }
    }
}

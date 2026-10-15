//! Checks which dates a query matches, as seen on a given day.
//!
//! Covers every entry in `languages/nb.toml`. Most checks are seen on
//! [`THURSDAY`]; the ones that depend on where today falls have their own day.

use whenabouts::{Clause, Date};

/// 2026-10-15. Easter that year is April 5.
const THURSDAY: &str = "2026-10-15";

#[test]
fn months() {
    let day = on("nb", THURSDAY);
    for (queries, first, last, before, after) in [
        (
            &["januar", "i jan"][..],
            "2026-01-01",
            "2026-01-31",
            "2025-12-31",
            "2026-02-01",
        ),
        (
            &["februar", "feb"],
            "2026-02-01",
            "2026-02-28",
            "2026-01-31",
            "2026-03-01",
        ),
        (
            &["mars"],
            "2026-03-01",
            "2026-03-31",
            "2026-02-28",
            "2026-04-01",
        ),
        (
            &["april", "apr"],
            "2026-04-01",
            "2026-04-30",
            "2026-03-31",
            "2026-05-01",
        ),
        (
            &["i mai", "fra mai", "om mai", "rundt mai", "under mai"],
            "2026-05-01",
            "2026-05-31",
            "2026-04-30",
            "2026-06-01",
        ),
        (
            &["juni"],
            "2026-06-01",
            "2026-06-30",
            "2026-05-31",
            "2026-07-01",
        ),
        (
            &["juli"],
            "2026-07-01",
            "2026-07-31",
            "2026-06-30",
            "2026-08-01",
        ),
        (
            &["august", "aug"],
            "2026-08-01",
            "2026-08-31",
            "2026-07-31",
            "2026-09-01",
        ),
        (
            &["september", "sep", "sept"],
            "2026-09-01",
            "2026-09-30",
            "2026-08-31",
            "2026-10-01",
        ),
        (
            &["oktober", "okt"],
            "2026-10-01",
            "2026-10-31",
            "2026-09-30",
            "2026-11-01",
        ),
        (
            &["november", "nov"],
            "2026-11-01",
            "2026-11-30",
            "2026-10-31",
            "2026-12-01",
        ),
        (
            &["desember", "des"],
            "2026-12-01",
            "2026-12-31",
            "2026-11-30",
            "2027-01-01",
        ),
    ] {
        for query in queries {
            day.query(query)
                .matches([first, last])
                .misses([before, after]);
        }
    }
    // Without a year, a month matches in every year.
    day.query("mars").matches(["1999-03-15", "2040-03-15"]);
    day.query("jan 2020")
        .matches(["2020-01-01", "2020-01-31"])
        .misses(["2021-01-15"]);
    day.query("mars 2021")
        .matches(["2021-03-01"])
        .misses(["2020-03-01", "2021-04-01"]);
}

#[test]
fn seasons() {
    let day = on("nb", THURSDAY);
    day.query("våren")
        .matches(["2026-03-01", "2026-05-31", "2019-04-01"])
        .misses(["2026-02-28", "2026-06-01"]);
    for query in ["sommer", "sommeren"] {
        day.query(query)
            .matches(["2026-06-01", "2026-08-31"])
            .misses(["2026-05-31", "2026-09-01"]);
    }
    for query in ["høst", "høsten"] {
        day.query(query)
            .matches(["2026-09-01", "2026-11-30"])
            .misses(["2026-08-31", "2026-12-01"]);
    }
    // Winter spans New Year.
    for query in ["vinter", "vinteren", "vintren"] {
        day.query(query)
            .matches(["2025-12-01", "2026-01-15", "2026-02-28"])
            .misses(["2025-11-30", "2026-03-01"]);
    }
    day.query("vår 2020")
        .matches(["2020-03-01", "2020-05-31"])
        .misses(["2021-04-01"]);
}

#[test]
fn weekdays() {
    let day = on("nb", THURSDAY);
    for (words, date, before, after) in [
        ("mandag", "2026-10-12", "2026-10-11", "2026-10-13"),
        ("tirsdag", "2026-10-13", "2026-10-12", "2026-10-14"),
        ("onsdag", "2026-10-14", "2026-10-13", "2026-10-15"),
        ("torsdag", "2026-10-15", "2026-10-14", "2026-10-16"),
        ("fredag", "2026-10-16", "2026-10-15", "2026-10-17"),
        ("lørdag", "2026-10-17", "2026-10-16", "2026-10-18"),
        ("søndag", "2026-10-18", "2026-10-17", "2026-10-19"),
    ] {
        for query in [
            words.to_string(),
            format!("{words}en"),
            format!("på {words}"),
        ] {
            day.query(&query).matches([date]).misses([before, after]);
        }
    }
    day.query("fredag forrige uke")
        .matches(["2026-10-09"])
        .misses(["2026-10-02", "2026-10-16"]);
    day.query("forrige fredag")
        .matches(["2026-10-09"])
        .misses(["2026-10-02", "2026-10-16"]);
}

#[test]
fn weekends() {
    let day = on("nb", THURSDAY);
    for query in [
        "helgen",
        "helga",
        "i helgen",
        "i helga",
        "ihelgen",
        "denne helgen",
    ] {
        day.query(query)
            .matches(["2026-10-10", "2026-10-11"])
            .misses(["2026-10-09", "2026-10-12", "2026-10-03", "2026-10-17"]);
    }
    // The last one that has ended.
    day.query("forrige helg")
        .matches(["2026-10-10", "2026-10-11"])
        .misses(["2026-10-03", "2026-10-17"]);
    // Any weekend that year.
    day.query("helg i fjor")
        .matches(["2025-10-11", "2025-10-12"])
        .misses(["2025-10-10", "2026-10-10"]);
}

#[test]
fn fixed_holidays() {
    let day = on("nb", THURSDAY);
    for (queries, date, before, after) in [
        (
            &["julaften", "julaftenen", "juleaften", "juleaftenen"][..],
            "12-24",
            "12-23",
            "12-25",
        ),
        (
            &["1. juledag", "første juledag", "første juledagen"],
            "12-25",
            "12-24",
            "12-26",
        ),
        (
            &["2. juledag", "andre juledag", "andre juledagen"],
            "12-26",
            "12-25",
            "12-27",
        ),
        (
            &["nyttårsaften", "nyttårsaftenen"],
            "12-31",
            "12-30",
            "01-01",
        ),
        (&["nyttårsdag", "nyttårsdagen"], "01-01", "12-31", "01-02"),
        (
            &["syttende mai", "17. mai", "nasjonaldag", "nasjonaldagen"],
            "05-17",
            "05-16",
            "05-18",
        ),
        (
            &["første mai", "1. mai", "arbeidernes dag"],
            "05-01",
            "04-30",
            "05-02",
        ),
        (
            &["sankthans", "sankthansaften", "st. hans", "st. hansaften"],
            "06-23",
            "06-22",
            "06-24",
        ),
        (
            &["valentinsdag", "valentinsdagen"],
            "02-14",
            "02-13",
            "02-15",
        ),
        (
            &["luciadag", "luciadagen", "på lucia"],
            "12-13",
            "12-12",
            "12-14",
        ),
        (
            &["halloween", "allehelgensaften"],
            "10-31",
            "10-30",
            "11-01",
        ),
    ] {
        for query in queries {
            day.query(query)
                .matches([format!("2026-{date}"), format!("2019-{date}")])
                .misses([format!("2026-{before}"), format!("2026-{after}")]);
        }
    }
}

#[test]
fn christmas_and_new_year() {
    let day = on("nb", THURSDAY);
    for query in ["jul", "julen", "jula"] {
        day.query(query)
            .matches(["2026-12-24", "2026-12-26", "2019-12-25"])
            .misses(["2026-12-23", "2026-12-27"]);
    }
    for query in ["nyttår", "nyttåret"] {
        day.query(query)
            .matches(["2025-12-31", "2026-01-01"])
            .misses(["2025-12-30", "2026-01-02"]);
    }
    // The year is the one New Year's Eve is in.
    day.query("nyttåret 2020")
        .matches(["2020-12-31", "2021-01-01"])
        .misses(["2020-01-01", "2019-12-31"]);
    for query in ["advent", "adventen", "adventstid", "adventstiden"] {
        day.query(query)
            .matches(["2026-11-27", "2026-12-24"])
            .misses(["2026-11-26", "2026-12-25"]);
    }
    for query in ["romjul", "romjulen", "romjula"] {
        day.query(query)
            .matches(["2026-12-27", "2026-12-31"])
            .misses(["2026-12-26", "2027-01-01"]);
    }
}

#[test]
fn easter_holidays() {
    let day = on("nb", THURSDAY);
    // Palm Sunday to Easter Monday.
    for query in ["påske", "påsken", "påska", "påskeferie", "påskeferien"] {
        day.query(query)
            .matches(["2026-03-29", "2026-04-06", "2025-04-20"])
            .misses(["2026-03-28", "2026-04-07"]);
    }
    for (queries, date) in [
        (&["palmesøndag", "palmesøndagen"][..], "2026-03-29"),
        (&["skjærtorsdag", "skjærtorsdagen"], "2026-04-02"),
        (&["langfredag", "langfredagen"], "2026-04-03"),
        (&["påskeaften", "påskeaftenen"], "2026-04-04"),
        (
            &["1. påskedag", "første påskedag", "første påskedagen"],
            "2026-04-05",
        ),
        (
            &["2. påskedag", "andre påskedag", "andre påskedagen"],
            "2026-04-06",
        ),
        (
            &[
                "kristi himmelfart",
                "kristi himmelfartsdag",
                "kristi himmelfartsdagen",
            ],
            "2026-05-14",
        ),
        (
            &["1. pinsedag", "første pinsedag", "første pinsedagen"],
            "2026-05-24",
        ),
        (
            &["2. pinsedag", "andre pinsedag", "andre pinsedagen"],
            "2026-05-25",
        ),
    ] {
        for query in queries {
            day.query(query)
                .matches([date])
                .misses(["2026-04-01", "2026-05-26"]);
        }
    }
    for query in ["pinse", "pinsen"] {
        day.query(query)
            .matches(["2026-05-24", "2026-05-25"])
            .misses(["2026-05-23", "2026-05-26"]);
    }
    // Easter 2019 is April 21, and 2022 April 17.
    day.query("påske 2019")
        .matches(["2019-04-14", "2019-04-22"])
        .misses(["2019-04-13", "2019-04-23", "2026-04-05"]);
    day.query("andre pinsedag 2022")
        .matches(["2022-06-06"])
        .misses(["2022-06-05", "2026-05-25"]);
}

#[test]
fn school_holidays() {
    let day = on("nb", THURSDAY);
    // Weeks 8 to 9 of 2026 are February 16 to March 1.
    for query in ["vinterferie", "vinterferien"] {
        day.query(query)
            .matches(["2026-02-16", "2026-03-01"])
            .misses(["2026-02-15", "2026-03-02"]);
    }
    // Weeks 40 to 41 are September 28 to October 11.
    for query in ["høstferie", "høstferien"] {
        day.query(query)
            .matches(["2026-09-28", "2026-10-11"])
            .misses(["2026-09-27", "2026-10-12"]);
    }
    // Weeks 27 to 31 are June 29 to August 2.
    for query in ["fellesferie", "fellesferien"] {
        day.query(query)
            .matches(["2026-06-29", "2026-08-02"])
            .misses(["2026-06-28", "2026-08-03"]);
    }
    for query in ["sommerferie", "sommerferien"] {
        day.query(query)
            .matches(["2026-06-15", "2026-08-25"])
            .misses(["2026-06-14", "2026-08-26"]);
    }
    // Christmas holidays reach into the next year.
    for query in ["juleferie", "juleferien"] {
        day.query(query)
            .matches(["2025-12-20", "2026-01-03"])
            .misses(["2025-12-19", "2026-01-04"]);
    }
    day.query("juleferien 2019")
        .matches(["2019-12-20", "2020-01-03"])
        .misses(["2019-01-03", "2020-12-20"]);
}

#[test]
fn latest() {
    let day = on("nb", THURSDAY);
    for query in ["i vår", "ivår"] {
        day.query(query)
            .matches(["2026-03-01", "2026-05-31"])
            .misses(["2025-04-01", "2027-04-01"]);
    }
    for query in ["i sommer", "isommer", "denne sommeren"] {
        day.query(query)
            .matches(["2026-06-01", "2026-08-31"])
            .misses(["2025-07-01", "2026-09-01"]);
    }
    // Autumn is under way.
    for query in ["i høst", "ihøst"] {
        day.query(query)
            .matches(["2026-09-01", "2026-11-30"])
            .misses(["2025-10-01", "2026-12-01"]);
    }
    for query in ["i vinter", "ivinter"] {
        day.query(query)
            .matches(["2025-12-01", "2026-02-28"])
            .misses(["2024-12-15", "2026-12-01"]);
    }
    // Christmas 2026 hasn't come yet.
    for query in ["i jul", "i julen", "i jula", "ijula"] {
        day.query(query)
            .matches(["2025-12-24", "2025-12-26"])
            .misses(["2024-12-24", "2026-12-24"]);
    }
    for query in ["i påske", "i påsken", "i påska", "ipåsken"] {
        day.query(query)
            .matches(["2026-03-29", "2026-04-06"])
            .misses(["2025-04-20", "2027-03-28"]);
    }
    // The last one that has ended, so this year's if it is over.
    day.query("forrige sommer")
        .matches(["2026-06-01", "2026-08-31"])
        .misses(["2025-07-01"]);
    for query in ["forrige jul", "sist jul"] {
        day.query(query)
            .matches(["2025-12-24"])
            .misses(["2024-12-24", "2026-12-24"]);
    }
    day.query("forrige påske")
        .matches(["2026-04-05"])
        .misses(["2025-04-20"]);
    day.query("forrige mai")
        .matches(["2026-05-01", "2026-05-31"])
        .misses(["2025-05-15"]);
    day.query("forrige sommerferie")
        .matches(["2026-06-15"])
        .misses(["2025-06-15"]);
    // Autumn is under way, so the one before.
    day.query("forrige høst")
        .matches(["2025-10-01"])
        .misses(["2026-10-01"]);
}

#[test]
fn relative_days() {
    let day = on("nb", THURSDAY);
    for query in ["i dag", "idag", "i morges", "imorges", "i kveld", "ikveld"] {
        day.query(query)
            .matches(["2026-10-15"])
            .misses(["2026-10-14", "2026-10-16"]);
    }
    for query in ["i går", "igår", "i går kveld", "igår kveld", "igårkveld"] {
        day.query(query)
            .matches(["2026-10-14"])
            .misses(["2026-10-13", "2026-10-15"]);
    }
    for query in ["i forgårs", "iforgårs"] {
        day.query(query)
            .matches(["2026-10-13"])
            .misses(["2026-10-12", "2026-10-14"]);
    }
}

#[test]
fn relative_years() {
    let day = on("nb", THURSDAY);
    for (queries, year) in [
        (&["i år", "iår", "dette året", "dette år"][..], 2026),
        (&["i fjor", "ifjor", "forrige år"], 2025),
        (&["i forfjor", "iforfjor"], 2024),
    ] {
        for query in queries {
            day.query(query)
                .matches([format!("{year}-01-01"), format!("{year}-12-31")])
                .misses([format!("{}-12-31", year - 1), format!("{}-01-01", year + 1)]);
        }
    }
}

#[test]
fn this_and_previous() {
    let day = on("nb", THURSDAY);
    for query in ["denne uke", "denne uken", "denne uka"] {
        day.query(query)
            .matches(["2026-10-12", "2026-10-18"])
            .misses(["2026-10-11", "2026-10-19"]);
    }
    for query in ["forrige uke", "forrige uka"] {
        day.query(query)
            .matches(["2026-10-05", "2026-10-11"])
            .misses(["2026-10-04", "2026-10-12"]);
    }
    for query in ["denne måneden", "denne måned"] {
        day.query(query)
            .matches(["2026-10-01", "2026-10-31"])
            .misses(["2026-09-30", "2026-11-01"]);
    }
    day.query("forrige måned")
        .matches(["2026-09-01", "2026-09-30"])
        .misses(["2026-08-31", "2026-10-01"]);
    for query in ["denne dagen", "dette døgnet"] {
        day.query(query)
            .matches(["2026-10-15"])
            .misses(["2026-10-14"]);
    }
    day.query("forrige døgn")
        .matches(["2026-10-14"])
        .misses(["2026-10-15"]);
}

#[test]
fn ago() {
    let day = on("nb", THURSDAY);
    for (n, date) in [
        ("1", "2026-10-14"),
        ("en", "2026-10-14"),
        ("ett", "2026-10-14"),
        ("ei", "2026-10-14"),
        ("et", "2026-10-14"),
        ("to", "2026-10-13"),
        ("tre", "2026-10-12"),
        ("fire", "2026-10-11"),
        ("fem", "2026-10-10"),
        ("seks", "2026-10-09"),
        ("sju", "2026-10-08"),
        ("syv", "2026-10-08"),
        ("åtte", "2026-10-07"),
        ("ni", "2026-10-06"),
        ("ti", "2026-10-05"),
        ("elleve", "2026-10-04"),
        ("tolv", "2026-10-03"),
        ("30", "2026-09-15"),
    ] {
        for query in [format!("for {n} dager siden"), format!("{n} døgn siden")] {
            day.query(&query).matches([date]).misses(["2026-10-15"]);
        }
    }
    for query in ["for to uker siden", "to uke siden"] {
        day.query(query)
            .matches(["2026-09-28", "2026-10-04"])
            .misses(["2026-09-27", "2026-10-05"]);
    }
    day.query("for tre måneder siden")
        .matches(["2026-07-01", "2026-07-31"])
        .misses(["2026-06-30", "2026-08-01"]);
    day.query("for 3 år siden")
        .matches(["2023-01-01", "2023-12-31"])
        .misses(["2022-12-31", "2024-01-01"]);
}

#[test]
fn last() {
    let day = on("nb", THURSDAY);
    // Up to and including today.
    for query in ["siste uke", "den siste uken", "den siste uka"] {
        day.query(query)
            .matches(["2026-10-09", "2026-10-15"])
            .misses(["2026-10-08", "2026-10-16"]);
    }
    for query in ["siste måned", "den siste måneden"] {
        day.query(query)
            .matches(["2026-09-16", "2026-10-15"])
            .misses(["2026-09-15", "2026-10-16"]);
    }
    for query in ["siste år", "det siste året"] {
        day.query(query)
            .matches(["2025-10-16", "2026-10-15"])
            .misses(["2025-10-15", "2026-10-16"]);
    }
    for query in ["siste døgn", "det siste døgnet"] {
        day.query(query)
            .matches(["2026-10-15"])
            .misses(["2026-10-14", "2026-10-16"]);
    }
    for query in ["siste ti dager", "de siste 10 dagene"] {
        day.query(query)
            .matches(["2026-10-06", "2026-10-15"])
            .misses(["2026-10-05", "2026-10-16"]);
    }
    day.query("de siste 3 månedene")
        .matches(["2026-07-16", "2026-10-15"])
        .misses(["2026-07-15", "2026-10-16"]);
}

#[test]
fn weeks() {
    let day = on("nb", THURSDAY);
    // ISO week 32 of 2026 is August 3 to 9, and of 2025 August 4 to 10.
    day.query("uke 32")
        .matches(["2026-08-03", "2026-08-09", "2025-08-04"])
        .misses(["2026-08-02", "2026-08-10"]);
    day.query("uke 32 i fjor")
        .matches(["2025-08-04", "2025-08-10"])
        .misses(["2025-08-03", "2025-08-11", "2026-08-03"]);
}

#[test]
fn years_and_dates() {
    let day = on("nb", THURSDAY);
    for query in ["2024", "fra 2024"] {
        day.query(query)
            .matches(["2024-01-01", "2024-12-31"])
            .misses(["2023-12-31", "2025-01-01"]);
    }
    for query in ["17.05.2020", "17. mai 2020", "2020-05-17"] {
        day.query(query)
            .matches(["2020-05-17"])
            .misses(["2020-05-16", "2020-05-18", "2021-05-17"]);
    }
    day.query("14.07.23")
        .matches(["2023-07-14"])
        .misses(["2024-07-14"]);
    day.query("3. juni")
        .matches(["2026-06-03", "2019-06-03"])
        .misses(["2026-06-02", "2026-06-04"]);
    day.query("3. forrige måned")
        .matches(["2026-09-03"])
        .misses(["2026-10-03", "2026-09-04"]);
}

#[test]
fn short_years() {
    let day = on("nb", THURSDAY);
    for query in ["sommer 23", "sommeren '23", "sommer -23"] {
        day.query(query)
            .matches(["2023-06-01", "2023-08-31"])
            .misses(["2024-07-01", "1923-07-01"]);
    }
    day.query("sommeren '95")
        .matches(["1995-07-01"])
        .misses(["2095-07-01"]);
    day.query("lucia 2023")
        .matches(["2023-12-13"])
        .misses(["2024-12-13"]);
}

#[test]
fn combinations() {
    let day = on("nb", THURSDAY);
    for query in ["mai i fjor", "i fjor mai", "i mai i fjor", "mai ifjor"] {
        day.query(query)
            .matches(["2025-05-01", "2025-05-31"])
            .misses(["2026-05-15", "2024-05-15"]);
    }
    for query in ["i fjor sommer", "sommeren i fjor"] {
        day.query(query)
            .matches(["2025-07-01"])
            .misses(["2026-07-01"]);
    }
    day.query("julaften i fjor")
        .matches(["2025-12-24"])
        .misses(["2026-12-24", "2024-12-24"]);
    day.query("i februar for to år siden")
        .matches(["2024-02-29"])
        .misses(["2025-02-01"]);
    day.query("lørdag forrige uke")
        .matches(["2026-10-10"])
        .misses(["2026-10-03", "2026-10-17"]);
}

/// Winter started last year, and the week before is in 2025.
#[test]
fn new_years_day() {
    let day = on("nb", "2026-01-01");
    day.query("i vinter")
        .matches(["2025-12-01", "2026-01-01"])
        .misses(["2024-12-15", "2026-12-01"]);
    day.query("forrige uke")
        .matches(["2025-12-22", "2025-12-28"])
        .misses(["2025-12-21", "2025-12-29"]);
    day.query("i jula")
        .matches(["2025-12-24"])
        .misses(["2024-12-24"]);
}

/// Easter 2026 is April 5, and already under way.
#[test]
fn during_easter() {
    let day = on("nb", "2026-04-01");
    day.query("i påsken")
        .matches(["2026-03-29", "2026-04-06"])
        .misses(["2026-04-07", "2025-04-20"]);
    day.query("forrige påske")
        .matches(["2025-04-20"])
        .misses(["2026-04-05"]);
}

/// The weekend is under way.
#[test]
fn on_saturday() {
    let day = on("nb", "2026-10-17");
    day.query("i helgen")
        .matches(["2026-10-17", "2026-10-18"])
        .misses(["2026-10-10"]);
    day.query("forrige helg")
        .matches(["2026-10-10", "2026-10-11"])
        .misses(["2026-10-17"]);
    // Yesterday was the last Friday.
    day.query("forrige fredag")
        .matches(["2026-10-16"])
        .misses(["2026-10-09"]);
}

/// A Friday under way.
#[test]
fn on_friday() {
    let day = on("nb", "2026-10-16");
    day.query("forrige fredag")
        .matches(["2026-10-09"])
        .misses(["2026-10-16", "2026-10-02"]);
}

/// English weeks start on Sunday: this week is Sunday 11 to Saturday 17 October.
#[test]
fn english_weeks() {
    let day = on("en", THURSDAY);
    day.query("this week")
        .matches(["2026-10-11", "2026-10-17"])
        .misses(["2026-10-10", "2026-10-18"]);
    day.query("last week")
        .matches(["2026-10-04", "2026-10-10"])
        .misses(["2026-10-03", "2026-10-11"]);
    day.query("saturday last week")
        .matches(["2026-10-10"])
        .misses(["2026-10-03", "2026-10-17"]);
    // ISO week numbers don't change.
    day.query("week 42")
        .matches(["2026-10-12", "2026-10-18"])
        .misses(["2026-10-11"]);
}

#[test]
fn english_weekday_holidays() {
    let day = on("en", THURSDAY);
    day.query("thanksgiving 2025")
        .matches(["2025-11-27"])
        .misses(["2025-11-26", "2025-11-28", "2025-11-20"]);
    day.query("black friday 2025")
        .matches(["2025-11-28"])
        .misses(["2025-11-21"]);
    day.query("mother's day")
        .matches(["2026-05-10", "2025-05-11"])
        .misses(["2026-05-03", "2026-05-17"]);
    day.query("friday the 13th")
        .matches(["2026-02-13", "2026-03-13", "2026-11-13"])
        .misses(["2026-05-13", "2026-02-20"]);
    day.query("friday in week 32")
        .matches(["2026-08-07"])
        .misses(["2026-08-06", "2026-08-14"]);
}

fn on(locale: &'static str, today: &str) -> Day {
    Day {
        locale,
        today: date(today),
    }
}

struct Day {
    locale: &'static str,
    today: Date,
}

impl Day {
    /// The query must have exactly one time expression.
    #[track_caller]
    fn query(&self, query: &str) -> Check {
        let parsed = whenabouts::parse(query, self.locale).unwrap();
        let [phrase] = parsed.phrases() else {
            panic!("{query:?}: expected one phrase, got {parsed:?}");
        };
        Check {
            query: query.to_string(),
            clause: phrase.clause().clone(),
            today: self.today,
        }
    }
}

struct Check {
    query: String,
    clause: Clause,
    today: Date,
}

impl Check {
    #[track_caller]
    fn matches(self, days: impl IntoIterator<Item: AsRef<str>>) -> Check {
        self.expect(days, true)
    }

    #[track_caller]
    fn misses(self, days: impl IntoIterator<Item: AsRef<str>>) -> Check {
        self.expect(days, false)
    }

    #[track_caller]
    fn expect(self, days: impl IntoIterator<Item: AsRef<str>>, expected: bool) -> Check {
        for day in days.into_iter().map(|day| date(day.as_ref())) {
            assert!(
                self.clause.contains_as_of(self.today, day) == expected,
                "{:?} ({}) on {}: {} {day}",
                self.query,
                self.clause,
                self.today,
                if expected {
                    "should match"
                } else {
                    "should not match"
                },
            );
        }
        self
    }
}

#[track_caller]
fn date(text: &str) -> Date {
    let parts: Vec<_> = text.split('-').collect();
    let [year, month, day] = parts.as_slice() else {
        panic!("not a date: {text}");
    };
    Date::new(
        year.parse().unwrap(),
        month.parse().unwrap(),
        day.parse().unwrap(),
    )
    .unwrap_or_else(|| panic!("not a date: {text}"))
}

//! The description sentences over the shipped item database: every item
//! whose card builds a sentence is described with the installed phrase
//! tables, printed (`cargo test -p exa_items desc_phrase_corpus --
//! --nocapture`), and checked against the original's rules, restated here
//! from the two sentence builders rather than from Exarust's code:
//!
//! * a description of two bytes or fewer is left alone;
//! * a fixed-name item (seed word -1) only loses its first 0x10 byte and
//!   the byte after it, and takes "A "/"An " when its first byte is at
//!   least `Z` (the article test is the first byte against a, e, i, o, u);
//! * otherwise the quality mark is `round_ties_even(quality / 2.55)` and
//!   the condition mark the same of the wear byte;
//! * an unbracketed sentence is "A "/"An ", a quality phrase whose range
//!   holds the quality mark (the first 15 made rows, or the 8 high rows for
//!   a dash-marked weapon name), a space, the name and at most one
//!   condition phrase whose range holds the condition mark, ending in a
//!   dot;
//! * a bracketed one (`…[xx]`) keeps the name, then "It is " (or "They are
//!   " for an apparel `p` marker), the quality phrase and ", but" when both
//!   marks are on the same side of 50 or " and" when not; a `c` marker
//!   skips the quality phrase and the join;
//! * an apparel sentence of a variant 1–8 puts the variant word,
//!   capitalised, and a space in front of the title.

use std::collections::BTreeMap;

use exa_assets::{GameData, PackKind, locate_install};
use exa_formats::desc_phrases::{DescPhrases, PhraseRow};
use exa_formats::objstrings::Library;
use exa_formats::rdb::Database;
use exa_formats::text::takes_an;

use crate::card::{FLAWLESS_ROWS, GREAT_ROWS, HIGH_ROWS, MADE_ROWS, PERFECT_ROWS};
use crate::catalogue::ItemCatalogue;
use crate::desc_phrase::{FIXED_SEED, SentenceSource, describe};

fn mark(byte: u8) -> u8 {
    (f64::from(byte) / 2.55).round_ties_even() as u8
}

fn fits(rows: &[PhraseRow], count: usize, mark: u8) -> Vec<&str> {
    rows.iter()
        .take(count)
        .filter(|row| row.low <= mark && mark <= row.high)
        .map(|row| row.text.as_str())
        .collect()
}

fn article(text: &str) -> &'static str {
    if takes_an(text) { "An " } else { "A " }
}

/// The original's fixed-name edit: the first 0x10 and the byte after it
/// go, then an article when the first byte is at least `Z`.
fn fixed(text: &str) -> String {
    let mut bytes = text.as_bytes().to_vec();
    if let Some(at) = bytes.iter().position(|&b| b == 0x10) {
        let end = (at + 2).min(bytes.len());
        bytes.drain(at..end);
    }
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if text.as_bytes().first().is_some_and(|&b| b >= b'Z') {
        format!("{}{text}", article(&text))
    } else {
        text
    }
}

/// What a sentence's tail may be: nothing, or one of the condition phrases
/// whose range holds the mark (with its leading space unless it starts
/// with a comma) and a dot.
fn condition_tails(book: &DescPhrases, dashed: bool, condition: u8) -> Vec<String> {
    let mut rows = if dashed {
        fits(&book.great, GREAT_ROWS, condition)
    } else {
        let mut both = fits(&book.flawless, FLAWLESS_ROWS, condition);
        both.extend(fits(&book.perfect, PERFECT_ROWS, condition));
        both
    };
    rows.dedup();
    let mut out: Vec<String> = rows
        .into_iter()
        .map(|p| {
            if p.starts_with(',') {
                format!("{p}.")
            } else {
                format!(" {p}.")
            }
        })
        .collect();
    out.push(String::new());
    out
}

#[derive(Default)]
struct Tally {
    short: usize,
    fixed: usize,
    plain: usize,
    bracketed: usize,
    crafted: usize,
    dashed: usize,
    variant: usize,
}

#[test]
fn desc_phrase_corpus() {
    let Some(dir) = locate_install(None) else {
        eprintln!("no game install; skipping");
        return;
    };
    let Ok(exe) = std::fs::read(dir.join("Exanima.exe")) else {
        eprintln!("no Exanima.exe; skipping");
        return;
    };
    let Ok(game) = GameData::open(&dir) else {
        eprintln!("the game data does not open; skipping");
        return;
    };
    let resource = game.pack(PackKind::Resource).expect("a resource pack");
    let db = Database::parse(resource.get("objdb.rdb").expect("objdb.rdb")).unwrap();
    let strings = resource
        .get("objstrings.rdb")
        .and_then(|bytes| Library::parse(bytes).ok());
    let catalogue = ItemCatalogue::of_database_with_strings(&db, strings.as_ref());
    let book = DescPhrases::read(&exe).expect("the phrase tables");
    let mut tally = Tally::default();
    let mut lines = BTreeMap::new();
    // One roll threads through every card, as a session's would.
    let mut state = 0x1234_5678_u32;
    for record in &db.records {
        let Some(facts) = catalogue.card_facts(record.id) else {
            continue;
        };
        let Some(source) = facts.sentence.filter(|_| facts.titled) else {
            continue;
        };
        let title = facts.name.clone();
        let before = facts.description.clone();
        let out = describe(&book, state, source, &title, &before);
        state = out.state;
        lines.insert(
            record.id,
            format!(
                "{:#07x} {title:?} {source:?}\n    {before:?}\n -> {:?} / {:?}",
                record.id, out.title, out.description
            ),
        );
        check(&book, source, &title, &before, &out, &mut tally);
    }
    for line in lines.values() {
        println!("{line}");
    }
    println!(
        "{} sentences: {} short, {} fixed, {} plain, {} bracketed ({} crafted), {} dashed, {} variant titles",
        lines.len(),
        tally.short,
        tally.fixed,
        tally.plain,
        tally.bracketed,
        tally.crafted,
        tally.dashed,
        tally.variant
    );
    // The shipped database has every kind of sentence but variant titles:
    // its templates carry no armour variant (a copy's roll sets it).
    assert!(lines.len() > 100, "only {} sentences", lines.len());
    assert!(tally.plain > 0 && tally.bracketed > 0 && tally.crafted > 0);
    assert!(tally.fixed > 0 && tally.dashed > 0 && tally.short > 0);
}

fn check(
    book: &DescPhrases,
    source: SentenceSource,
    title: &str,
    before: &str,
    out: &crate::desc_phrase::Described,
    tally: &mut Tally,
) {
    let (seed, quality, damage, variant, apparel) = match source {
        SentenceSource::Weapon {
            seed,
            quality,
            damage,
        } => (seed, quality, damage, 0, false),
        SentenceSource::Apparel {
            seed,
            variant,
            quality,
            damage,
        } => (seed, quality, damage, variant, true),
    };
    let text = out.description.as_str();
    if before.len() < 3 {
        tally.short += 1;
        assert_eq!(text, before);
        assert_eq!(out.title, title);
        return;
    }
    if seed == FIXED_SEED {
        tally.fixed += 1;
        assert_eq!(text, fixed(before), "fixed {before:?}");
        assert_eq!(out.title, title);
        return;
    }
    let q = mark(quality);
    let c = mark(damage);
    // The name as the builders cut it.
    let mut name = before.as_bytes().to_vec();
    let mut tail = String::new();
    if !apparel && name.last() == Some(&b'.') {
        name.pop();
    }
    let bracket = name.last() == Some(&b']');
    let plural = apparel && bracket && name.len() >= 3 && name[name.len() - 3] == b'p';
    let crafted = bracket && name.len() >= 2 && name[name.len() - 2] == b'c';
    if bracket {
        name.truncate(name.len().saturating_sub(4));
    } else if apparel {
        if name.starts_with(b"A ") {
            name.drain(..2);
        }
        if name.len() >= 2
            && let Some(at) = name[..name.len() - 1].iter().position(|&b| b == b'.')
        {
            tail = String::from_utf8_lossy(&name[at + 1..]).into_owned();
            name.truncate(at + 1);
        }
    }
    if apparel && name.last() == Some(&b'.') {
        name.pop();
    }
    let dashed = !apparel && !crafted && name.first() == Some(&b'-');
    if dashed {
        name.remove(0);
        tally.dashed += 1;
    }
    let qualities: Vec<&str> = if crafted {
        vec![""]
    } else if dashed {
        fits(&book.high, HIGH_ROWS, q)
    } else {
        fits(&book.made, MADE_ROWS, q)
    };
    let qualities = if qualities.is_empty() {
        vec![""]
    } else {
        qualities
    };
    // The variant word goes into the title, and over an apparel name's 0x10.
    let word = (apparel && !crafted && (1..=8).contains(&variant))
        .then(|| book.variants[usize::from(variant) - 1].as_str());
    if let Some(word) = word {
        tally.variant += 1;
        let mut expected = format!("{word} {title}").into_bytes();
        expected[0] = expected[0].to_ascii_uppercase();
        assert_eq!(
            out.title.as_bytes(),
            expected.as_slice(),
            "title of {before:?}"
        );
    } else {
        assert_eq!(out.title, title);
    }
    let name_variants: Vec<String> = {
        let plain = String::from_utf8_lossy(&name).into_owned();
        if !apparel || crafted || !name.contains(&0x10) {
            vec![plain]
        } else if let Some(word) = word {
            let at = name.iter().position(|&b| b == 0x10).unwrap();
            let mut spliced = name[..at].to_vec();
            spliced.extend_from_slice(word.as_bytes());
            if at + 1 < name.len() {
                spliced.push(b' ');
                spliced.extend_from_slice(&name[at + 2..]);
            }
            vec![String::from_utf8_lossy(&spliced).into_owned()]
        } else {
            vec![fixed_mark(&name)]
        }
    };
    let tails = condition_tails(book, dashed, c);
    let mut allowed = Vec::new();
    for name in &name_variants {
        for quality in &qualities {
            let head = if crafted {
                format!("{name}It is")
            } else if bracket {
                let verb = if plural { "They are " } else { "It is " };
                let join = if (q <= 50) == (c <= 50) {
                    ", but"
                } else {
                    " and"
                };
                format!("{name}{verb}{quality}{join}")
            } else {
                format!("{quality} {name}")
            };
            for end in &tails {
                let body = format!("{head}{end}{tail}");
                allowed.push(if bracket {
                    body
                } else {
                    format!("{}{body}", article(&body))
                });
            }
        }
    }
    assert!(
        allowed.iter().any(|a| a == text),
        "{before:?} (q {q}, c {c}) gave {text:?}, not one of {allowed:?}"
    );
    if bracket {
        tally.bracketed += 1;
        if crafted {
            tally.crafted += 1;
        }
    } else {
        tally.plain += 1;
    }
}

/// An apparel name's 0x10 pair deleted (no variant word to put there).
fn fixed_mark(name: &[u8]) -> String {
    let mut bytes = name.to_vec();
    if let Some(at) = bytes.iter().position(|&b| b == 0x10) {
        let end = (at + 2).min(bytes.len());
        bytes.drain(at..end);
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

//! The generated sentence an item card adds to an apparel or weapon
//! description, from the phrase tables of the installed executable.
//!
//! The card builder rewrites the item's description through
//! [`apparel_sentence`] when the item has an apparel body and no weapon page,
//! and through [`item_sentence`] when its model is a factory model (name
//! starting with `@`) and it shows a weapon page; with neither a weapon page
//! nor an apparel body, the weapon rule reads the shield section's bytes in
//! the same places, as the original does. The apparel rule also puts the
//! armour-variant word ("heavy", "blued", …) in front of the card's title.
//! The rewritten description then goes through the card's condition blocks
//! and pronoun markers.
//!
//! The tables are [`exa_formats::desc_phrases`], read once from
//! `Exanima.exe` in the game folder by [`load_phrases`]. Without them the
//! description is left as it is.
//!
//! The original draws from the engine's single shared generator; the card
//! keeps its own state, which the caller passes in.
//!
//! Evidence: the card builder's sentence branch and the two sentence
//! builders of the item description unit (REA, checked against the
//! instructions: the marks are `byte / 2.55` in double precision converted
//! with round-to-nearest-even).

use bevy::prelude::*;
use exa_assets::Game;
use exa_formats::desc_phrases::{DescPhrases, PhraseRow};

use crate::card::{ApparelLine, Phrase, PhraseTables, apparel_sentence, item_sentence};

/// A seed word of -1: the item keeps its template's fixed name.
pub const FIXED_SEED: u32 = 0xFFFF_FFFF;

/// The phrase tables, once read from the executable.
#[derive(Resource, Debug, Default, Clone)]
pub struct PhraseBook(pub Option<DescPhrases>);

/// Which sentence builder a card uses and the bytes it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SentenceSource {
    /// A weapon (or, failing a weapon page and an apparel body, a shield)
    /// section: its first seed word, quality and damage bytes.
    Weapon { seed: u32, quality: u8, damage: u8 },
    /// An apparel body: its paint seed, armour variant, quality and damage.
    Apparel {
        seed: u32,
        variant: u8,
        quality: u8,
        damage: u8,
    },
}

fn byte(body: &[u8], at: usize) -> u8 {
    body.get(at).copied().unwrap_or(0)
}

fn word(body: &[u8], at: usize) -> u32 {
    body.get(at..at + 4)
        .and_then(|w| w.try_into().ok())
        .map_or(0, u32::from_le_bytes)
}

impl SentenceSource {
    /// The weapon rule over a weapon or shield section body (seed at 4,
    /// quality at 0xC, damage at 0xD). Missing bytes read as zero.
    pub fn weapon(section: &[u8]) -> Self {
        Self::Weapon {
            seed: word(section, 4),
            quality: byte(section, 0xC),
            damage: byte(section, 0xD),
        }
    }

    /// The apparel rule over an apparel body (variant at 0x1AA, seed at
    /// 0x1AC, quality at 0x1B0, damage at 0x1B1).
    pub fn apparel(body: &[u8]) -> Self {
        Self::Apparel {
            seed: word(body, 0x1AC),
            variant: byte(body, 0x1AA),
            quality: byte(body, 0x1B0),
            damage: byte(body, 0x1B1),
        }
    }

    /// The card builder's choice: a weapon page (a weapon section that is
    /// not withheld) uses the weapon rule on a factory model; otherwise an
    /// apparel body uses the apparel rule, whatever its armour type;
    /// otherwise a shield section uses the weapon rule on a factory model.
    pub fn choose(
        weapon_page: Option<&[u8]>,
        body: Option<&[u8]>,
        shield: Option<&[u8]>,
        factory: bool,
    ) -> Option<Self> {
        if let Some(weapon) = weapon_page {
            factory.then(|| Self::weapon(weapon))
        } else if let Some(body) = body {
            Some(Self::apparel(body))
        } else {
            shield.filter(|_| factory).map(Self::weapon)
        }
    }
}

/// A card's title and description after the sentence, and the roll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Described {
    pub state: u32,
    pub title: String,
    pub description: String,
}

fn phrases(rows: &[PhraseRow]) -> Vec<Phrase<'_>> {
    rows.iter()
        .map(|row| Phrase {
            text: &row.text,
            low: row.low,
            high: row.high,
        })
        .collect()
}

/// The variant word of variants 1 to 8; empty otherwise.
pub fn variant_word(book: &DescPhrases, variant: u8) -> &str {
    if (1..=8).contains(&variant) {
        book.variants
            .get(usize::from(variant) - 1)
            .map_or("", String::as_str)
    } else {
        ""
    }
}

/// Rewrites a card's title and description with the sentence `source`
/// asks for, drawing from `state`.
pub fn describe(
    book: &DescPhrases,
    state: u32,
    source: SentenceSource,
    title: &str,
    description: &str,
) -> Described {
    let (made, high, flawless, perfect, great) = (
        phrases(&book.made),
        phrases(&book.high),
        phrases(&book.flawless),
        phrases(&book.perfect),
        phrases(&book.great),
    );
    let tables = PhraseTables {
        made: &made,
        high: &high,
        flawless: &flawless,
        perfect: &perfect,
        great: &great,
    };
    match source {
        SentenceSource::Weapon {
            seed,
            quality,
            damage,
        } => {
            let sentence = item_sentence(
                state,
                description,
                seed == FIXED_SEED,
                quality,
                damage,
                &tables,
            );
            Described {
                state: sentence.state,
                title: title.to_string(),
                description: sentence.text,
            }
        }
        SentenceSource::Apparel {
            seed,
            variant,
            quality,
            damage,
        } => {
            let line = ApparelLine {
                quality,
                condition: damage,
                material: variant,
                material_name: variant_word(book, variant),
                side: title,
            };
            let sentence = apparel_sentence(state, description, seed == FIXED_SEED, line, &tables);
            Described {
                state: sentence.state,
                title: sentence.side,
                description: sentence.text,
            }
        }
    }
}

/// Reads the phrase tables from the game folder's executable, once. A
/// missing or unreadable executable leaves the book empty.
pub fn load_phrases(game: Option<Res<Game>>, mut book: ResMut<PhraseBook>, mut tried: Local<bool>) {
    if *tried {
        return;
    }
    let Some(game) = game else {
        return;
    };
    *tried = true;
    let path = game.0.root().join("Exanima.exe");
    match std::fs::read(&path).map(|bytes| DescPhrases::read(&bytes)) {
        Ok(Ok(phrases)) => {
            info!("item phrases: {} quality rows", phrases.made.len());
            book.0 = Some(phrases);
        }
        Ok(Err(e)) => warn!("item phrases: {} does not parse: {e}", path.display()),
        Err(e) => warn!("item phrases: cannot read {}: {e}", path.display()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(text: &str, low: u8, high: u8) -> PhraseRow {
        PhraseRow {
            text: text.into(),
            low,
            high,
        }
    }

    /// A small synthetic book in the shipped tables' shape (not their text).
    pub(crate) fn book() -> DescPhrases {
        DescPhrases {
            flawless: vec![row("in mint shape", 0, 50), row("in rough shape", 51, 100)],
            perfect: vec![row("in fallback shape", 0, 100)],
            high: vec![row("top grade", 0, 100)],
            great: vec![row("in grand shape", 0, 100)],
            made: vec![row("finely made", 51, 100), row("roughly made", 0, 50)],
            variants: (1..=8).map(|i| format!("v{i}")).collect(),
        }
    }

    #[test]
    fn the_card_picks_its_sentence_rule() {
        let weapon = [0u8; 0x10];
        let body = [0u8; 0x1B2];
        let shield = [7u8; 0x10];
        assert_eq!(
            SentenceSource::choose(Some(&weapon), Some(&body), Some(&shield), false),
            None
        );
        assert!(matches!(
            SentenceSource::choose(Some(&weapon), Some(&body), None, true),
            Some(SentenceSource::Weapon { .. })
        ));
        assert!(matches!(
            SentenceSource::choose(None, Some(&body), Some(&shield), false),
            Some(SentenceSource::Apparel { .. })
        ));
        assert_eq!(
            SentenceSource::choose(None, None, Some(&shield), true),
            Some(SentenceSource::Weapon {
                seed: 0x0707_0707,
                quality: 7,
                damage: 7
            })
        );
        assert_eq!(
            SentenceSource::choose(None, None, Some(&shield), false),
            None
        );
        let mut body = vec![0u8; 0x1B2];
        body[0x1AA] = 2;
        body[0x1AC..0x1B0].copy_from_slice(&FIXED_SEED.to_le_bytes());
        body[0x1B0] = 200;
        body[0x1B1] = 9;
        assert_eq!(
            SentenceSource::apparel(&body),
            SentenceSource::Apparel {
                seed: FIXED_SEED,
                variant: 2,
                quality: 200,
                damage: 9
            }
        );
        // A short section reads zeros.
        assert_eq!(
            SentenceSource::weapon(&[1, 2]),
            SentenceSource::Weapon {
                seed: 0,
                quality: 0,
                damage: 0
            }
        );
    }

    #[test]
    fn an_apparel_sentence_names_its_variant_and_condition() {
        let source = SentenceSource::Apparel {
            seed: 3,
            variant: 2,
            quality: 200,
            damage: 10,
        };
        let out = describe(&book(), 0, source, "gambeson", "A gambeson.");
        // 200 / 2.55 = 78.4 → 78: one quality candidate; 10 / 2.55 → 4:
        // "in mint shape". Two draws.
        assert_eq!(out.description, "A finely made gambeson in mint shape.");
        assert_eq!(out.title, "V2 gambeson");
        let once = 0u32.wrapping_mul(0x0808_8405).wrapping_add(1);
        assert_eq!(out.state, once.wrapping_mul(0x0808_8405).wrapping_add(1));
        // A fixed seed keeps the name and adds an article only from Z on.
        let fixed = SentenceSource::Apparel {
            seed: FIXED_SEED,
            variant: 2,
            quality: 200,
            damage: 10,
        };
        let out = describe(&book(), 5, fixed, "Coat", "Coat of plates");
        assert_eq!((out.state, out.title.as_str()), (5, "Coat"));
        assert_eq!(out.description, "Coat of plates");
    }

    #[test]
    fn a_weapon_sentence_reads_its_quality_and_damage() {
        let source = SentenceSource::Weapon {
            seed: 1,
            quality: 30,
            damage: 200,
        };
        let out = describe(&book(), 0, source, "Mace", "mace");
        assert_eq!(out.description, "A roughly made mace in rough shape.");
        assert_eq!(out.title, "Mace");
        let dashed = describe(&book(), 0, source, "Mace", "-mace");
        assert_eq!(dashed.description, "A top grade mace in grand shape.");
        assert_eq!(variant_word(&book(), 0), "");
        assert_eq!(variant_word(&book(), 9), "");
        assert_eq!(variant_word(&book(), 8), "v8");
    }

    /// The installed executable's tables build a whole sentence: the best
    /// quality and condition marks each have one row.
    #[test]
    fn real_tables_build_a_sentence() {
        let Ok(dir) = std::env::var("EXANIMA_DIR") else {
            eprintln!("EXANIMA_DIR not set; skipping");
            return;
        };
        let Ok(exe) = std::fs::read(std::path::Path::new(&dir).join("Exanima.exe")) else {
            eprintln!("no Exanima.exe in EXANIMA_DIR; skipping");
            return;
        };
        let book = DescPhrases::read(&exe).unwrap();
        let source = SentenceSource::Weapon {
            seed: 1,
            quality: 255,
            damage: 0,
        };
        let out = describe(&book, 0, source, "Sword", "sword");
        let expected = format!("{} sword {}.", book.made[0].text, book.flawless[0].text);
        let article = if exa_formats::text::takes_an(&expected) {
            "An"
        } else {
            "A"
        };
        assert_eq!(out.description, format!("{article} {expected}"));
        let source = SentenceSource::Apparel {
            seed: 1,
            variant: 2,
            quality: 255,
            damage: 0,
        };
        let out = describe(&book, 0, source, "Coat", "A coat.");
        assert!(out.title.ends_with(" Coat"));
        assert!(out.title.len() > "Coat".len() + 1);
    }
}

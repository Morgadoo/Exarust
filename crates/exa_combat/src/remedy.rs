//! A salve and an elixir used on a character.
//!
//! The operative section's kind byte selects the use. Kind 8 adds to the
//! heal pool when the body's rule bits take remedies: 0.1 when the
//! section's value word is 50, otherwise 0.2, in double precision, stored
//! back as a float. Kind 14 sets the potion flag and stamps the world clock
//! (whole seconds, the save's base plus the game clock's seconds) plus
//! 1800 s, then one unit draw (the process-wide twister, left in double)
//! becomes the pour's pitch, `0.9 + 0.2·draw`, at volume 0.25 on
//! `LqdA_Pour_01`. Either kind ORs `0xC80000`, whether or not the body
//! takes remedies, into the item used directly or into its container's
//! inventory entry when there is one ([`ItemSource`]). The clock is the
//! game's [`WorldClock`], whose saved base `exa_game::world_clock` restores
//! at a load and at a session start. A later body tick clears the potion flag
//! once the world clock is past that stamp. Rules: `docs/formats/OTHER.md`
//! (Regeneration), `docs/PLAYER.md` (Using an item).

use bevy::prelude::Resource;
use exa_formats::save_chunk::GameClock;

use crate::condition::StateFlags;

/// Operative kind of a healing salve.
pub const SALVE: u8 = 8;
/// Operative kind of an elixir.
pub const ELIXIR: u8 = 14;
/// The operative value word that fills the smaller share of the heal pool.
pub const SMALL_SALVE: u16 = 50;
/// Added to the heal pool, in double precision.
pub const SALVE_FILL: f64 = 0.2;
/// Added when the operative value word is [`SMALL_SALVE`].
pub const SMALL_SALVE_FILL: f64 = 0.1;
/// An elixir lasts this many seconds of the world clock.
pub const ELIXIR_S: u32 = 1800;
/// The pour's volume.
pub const POUR_VOLUME: f32 = 0.25;
/// The pour's pitch is this plus [`POUR_PITCH_SPAN`] times the unit draw.
pub const POUR_PITCH_BASE: f64 = 0.9;
/// The pour's pitch span, in double precision.
pub const POUR_PITCH_SPAN: f64 = 0.2;
/// The sample an elixir plays.
pub const POUR_SAMPLE: &str = "LqdA_Pour_01";
/// ORed into the used item's flags or its inventory entry's.
pub const ITEM_SPENT: u32 = 0x00C8_0000;

/// The world clock behind the scripts' `WorldTime`: the save's whole
/// seconds plus the game clock's seconds since the load. The game keeps it
/// as a resource; the save's base is restored at a load and at a session
/// start (`exa_game::world_clock`), and without one the clock counts from
/// zero.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WorldClock(pub GameClock);

/// The world clock in seconds at `now_ms` of the game clock.
pub fn world_seconds(clock: Option<&WorldClock>, now_ms: u32) -> u32 {
    match clock {
        Some(clock) => clock.0.now(now_ms),
        None => GameClock::new().now(now_ms),
    }
}

/// Where the used item comes from, as the original's use dispatcher knows
/// it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemSource {
    /// The item itself (the hand's target is the open container).
    Direct,
    /// The item's container's inventory and the use's entry word: the
    /// original marks the entry at that word less one, and nothing when
    /// the word is zero.
    Inventory(usize),
}

/// The item the use is holding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemUse {
    /// The operative section's kind byte.
    pub kind: u8,
    /// The operative section's value word.
    pub value: u16,
    /// The item's flags.
    pub flags: u32,
    /// Where the item is used from.
    pub source: ItemSource,
}

impl ItemSource {
    /// Whether the original has an item or inventory entry to mark with the
    /// spent flags.
    pub fn marks(self) -> bool {
        match self {
            ItemSource::Direct => true,
            ItemSource::Inventory(entries) => entries > 0,
        }
    }
}

/// The character the item is used on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemedyTarget {
    pub heal_pool: f32,
    pub flags: u32,
    pub potion_until: u32,
    /// [`exa_formats::character_stats::BodyRules::TAKES_REMEDIES`].
    pub takes_remedies: bool,
}

/// What the use leaves on the character and on the item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Applied {
    pub heal_pool: f32,
    pub flags: u32,
    pub potion_until: u32,
    /// The elixir's pour. A salve, a refused elixir, and any other kind
    /// leave this empty, and the unit draw is not taken.
    pub pour: Option<Pour>,
    pub item_flags: u32,
}

/// The elixir's pour: volume 0.25, pitch `0.9 + 0.2·draw` in double
/// precision, then narrowed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pour {
    pub volume: f32,
    pub pitch: f32,
}

/// The heal pool after a salve's add. The value word 50 adds 0.1; any
/// other adds 0.2. The sum is in double precision and stored as a float.
pub fn fill_salve(pool: f32, value: u16) -> f32 {
    let add = if value == SMALL_SALVE {
        SMALL_SALVE_FILL
    } else {
        SALVE_FILL
    };
    (f64::from(pool) + add) as f32
}

/// Pitch of an elixir's pour from one unit draw in `[0, 1)`.
pub fn pour_pitch(unit: f64) -> f32 {
    (unit * POUR_PITCH_SPAN + POUR_PITCH_BASE) as f32
}

/// Use `item` on `target` at world-clock `now` (seconds). `unit` is drawn
/// only for an elixir the body accepts.
pub fn apply_remedy(
    item: ItemUse,
    target: RemedyTarget,
    now: u32,
    unit: impl FnOnce() -> f64,
) -> Applied {
    if item.kind != SALVE && item.kind != ELIXIR {
        return Applied {
            heal_pool: target.heal_pool,
            flags: target.flags,
            potion_until: target.potion_until,
            pour: None,
            item_flags: item.flags,
        };
    }
    let mut applied = Applied {
        heal_pool: target.heal_pool,
        flags: target.flags,
        potion_until: target.potion_until,
        pour: None,
        item_flags: if item.source.marks() {
            item.flags | ITEM_SPENT
        } else {
            item.flags
        },
    };
    if target.takes_remedies {
        if item.kind == SALVE {
            applied.heal_pool = fill_salve(target.heal_pool, item.value);
        } else {
            applied.flags |= StateFlags::POTION;
            applied.potion_until = now.wrapping_add(ELIXIR_S);
            applied.pour = Some(Pour {
                volume: POUR_VOLUME,
                pitch: pour_pitch(unit()),
            });
        }
    }
    applied
}

/// The body tick's potion check: while the flag is set, a world clock past
/// the stamp clears it. The compare is unsigned, so a clock that wrapped
/// below the stamp leaves the flag.
pub fn expire_potion(flags: u32, until: u32, now: u32) -> u32 {
    if flags & StateFlags::POTION != 0 && now > until {
        flags & !StateFlags::POTION
    } else {
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Fighter;
    use crate::condition::StateFlags;
    use bevy::prelude::*;
    use exa_formats::character_stats::BodyRules;
    use exa_physics::body::{BodyTicks, TickedCharacter};

    fn human() -> RemedyTarget {
        RemedyTarget {
            heal_pool: 0.3,
            flags: 0,
            potion_until: 0,
            takes_remedies: true,
        }
    }

    fn salve(value: u16) -> ItemUse {
        ItemUse {
            kind: SALVE,
            value,
            flags: 0x11,
            source: ItemSource::Direct,
        }
    }

    /// A salve adds 0.2, or 0.1 when the value word is 50, and marks the
    /// item spent. A body that does not take remedies keeps its pool, and
    /// neither path draws.
    #[test]
    fn a_salve_fills_the_heal_pool() {
        let mut drew = false;
        let draw = || {
            drew = true;
            0.0
        };
        let filled = apply_remedy(salve(1), human(), 0, draw);
        assert!(!drew);
        assert_eq!(filled.heal_pool, 0.5);
        assert_eq!(filled.item_flags, 0x11 | ITEM_SPENT);
        assert!(filled.pour.is_none());
        assert_eq!(filled.flags, 0);

        let mut drew = false;
        let small = apply_remedy(salve(SMALL_SALVE), human(), 0, || {
            drew = true;
            0.0
        });
        assert!(!drew);
        assert_eq!(small.heal_pool, 0.4);

        let mut drew = false;
        let refused = apply_remedy(
            salve(1),
            RemedyTarget {
                takes_remedies: false,
                ..human()
            },
            0,
            || {
                drew = true;
                0.0
            },
        );
        assert!(!drew);
        assert_eq!(refused.heal_pool, 0.3);
        assert_eq!(refused.item_flags, 0x11 | ITEM_SPENT);

        assert!(fill_salve(f32::NAN, 1).is_nan());
        let again = fill_salve(filled.heal_pool, 7);
        assert_eq!(again, 0.7);

        let mut fighter = Fighter::of_stats(
            None,
            exa_formats::character_stats::CharacterStats {
                rules: BodyRules(BodyRules::TAKES_REMEDIES),
                ..Default::default()
            },
        );
        fighter.health.heal_pool = 0.0;
        let mut flags = StateFlags::default();
        fighter.take_remedy(&mut flags, salve(1), 0, || 0.0);
        assert_eq!(fighter.health.heal_pool, 0.2);
        assert_eq!(flags.0, 0);
    }

    /// The spent flags go to the item itself, or to the last inventory
    /// entry only when the list has one; an empty list marks nothing.
    #[test]
    fn the_spent_flags_only_go_to_an_existing_item_or_entry() {
        assert!(ItemSource::Direct.marks());
        assert!(ItemSource::Inventory(1).marks());
        assert!(!ItemSource::Inventory(0).marks());
        let mut item = salve(1);
        item.source = ItemSource::Inventory(0);
        assert_eq!(apply_remedy(item, human(), 0, || 0.0).item_flags, 0x11);
        item.source = ItemSource::Inventory(2);
        assert_eq!(
            apply_remedy(item, human(), 0, || 0.0).item_flags,
            0x11 | ITEM_SPENT
        );
    }

    /// An elixir sets the potion flag, stamps now + 1800 s, and pours from
    /// the one unit draw. A body that does not take remedies is not poured
    /// on, and another kind leaves the character and the item.
    #[test]
    fn an_elixir_sets_the_potion_and_pours() {
        let mut drew = false;
        let drunk = apply_remedy(
            ItemUse {
                kind: ELIXIR,
                value: 0,
                flags: 1,
                source: ItemSource::Direct,
            },
            human(),
            1_000,
            || {
                drew = true;
                0.5
            },
        );
        assert!(drew);
        assert_eq!(drunk.flags, StateFlags::POTION);
        assert_eq!(drunk.potion_until, 2_800);
        assert_eq!(drunk.heal_pool, 0.3);
        assert_eq!(drunk.item_flags, 1 | ITEM_SPENT);
        let pour = drunk.pour.unwrap();
        assert_eq!(pour.volume, 0.25);
        assert_eq!(pour.pitch, 1.0);
        assert_eq!(pour_pitch(0.0), 0.9);

        let mut drew = false;
        let refused = apply_remedy(
            ItemUse {
                kind: ELIXIR,
                value: 50,
                flags: 0,
                source: ItemSource::Direct,
            },
            RemedyTarget {
                takes_remedies: false,
                flags: 0x4,
                potion_until: 9,
                ..human()
            },
            1_000,
            || {
                drew = true;
                0.0
            },
        );
        assert!(!drew);
        assert!(refused.pour.is_none());
        assert_eq!((refused.flags, refused.potion_until), (0x4, 9));
        assert_eq!(refused.item_flags, ITEM_SPENT);

        let mut drew = false;
        let other = apply_remedy(
            ItemUse {
                kind: 9,
                value: 1,
                flags: 3,
                source: ItemSource::Direct,
            },
            human(),
            0,
            || {
                drew = true;
                0.0
            },
        );
        assert!(!drew);
        assert_eq!(other.item_flags, 3);
        assert_eq!(other.heal_pool, 0.3);

        let restamped = apply_remedy(
            ItemUse {
                kind: ELIXIR,
                value: 0,
                flags: 0,
                source: ItemSource::Direct,
            },
            RemedyTarget {
                flags: StateFlags::POTION,
                potion_until: 50,
                ..human()
            },
            u32::MAX - 10,
            || 0.0,
        );
        assert_eq!(
            restamped.potion_until,
            (u32::MAX - 10).wrapping_add(ELIXIR_S)
        );
        assert_eq!(restamped.flags, StateFlags::POTION);
    }

    /// The tick clears the flag once the clock is past the stamp, and a
    /// wrapped clock that still compares below the stamp leaves it.
    #[test]
    fn the_potion_expires_when_the_clock_passes_its_stamp() {
        let on = StateFlags::POTION | 0x4;
        assert_eq!(expire_potion(on, 1000, 1000), on);
        assert_eq!(expire_potion(on, 1000, 1001), 0x4);
        assert_eq!(expire_potion(on, 1000, 999), on);
        assert_eq!(expire_potion(0x4, 0, 50), 0x4);
        assert_eq!(expire_potion(on, u32::MAX, 0), on);
    }

    /// The upkeep of a ticked fighter clears a potion whose stamp the
    /// world clock (the save's base plus the game clock's seconds) passes.
    #[test]
    fn the_upkeep_expires_the_ticked_fighters_potion() {
        let mut app = App::new();
        app.add_systems(Update, crate::regen::regenerate_red);
        app.insert_resource(WorldClock(GameClock::restore(1000, 0)));
        let mut fighter = Fighter::of_stats(
            None,
            exa_formats::character_stats::CharacterStats {
                rules: BodyRules(BodyRules::TAKES_REMEDIES),
                ..Default::default()
            },
        );
        fighter.potion_until = 1000;
        let body = app
            .world_mut()
            .spawn((fighter, StateFlags(StateFlags::POTION)))
            .id();
        app.insert_resource(TickedCharacter {
            body,
            character: None,
        });
        // 996 ms: still the saved second, 1000; 1008 ms: 1001, past the
        // stamp.
        app.insert_resource(BodyTicks(83));
        app.update();
        let flags = app.world().get::<StateFlags>(body).unwrap().0;
        assert_eq!(flags, StateFlags::POTION);

        *app.world_mut().resource_mut::<BodyTicks>() = BodyTicks(84);
        app.update();
        let flags = app.world().get::<StateFlags>(body).unwrap().0;
        assert_eq!(flags, 0);
        assert_eq!(
            world_seconds(Some(&WorldClock(GameClock::restore(1000, 0))), 84 * 12),
            1001
        );
    }
}

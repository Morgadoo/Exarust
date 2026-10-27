//! The player controls' rules that are not the hand's drags nor the hotkey
//! dispatch: what a click and the Ctrl/double-click interaction name and
//! do, the values an item used on the world leaves (a salve, an elixir, a
//! cleaning or a repair kit) with its timed list, the reach a walk-to use
//! measures, the movement request's drag and lift modes, the controls'
//! gates and permissions, the power bar's hold timer, and the start and end
//! of play. Rules: `docs/PLAYER.md` (Using what the cursor is over, Using an
//! item, The reach) and `docs/PLAYER.md` (Hotkeys, Combat mode).
//!
//! **A click.** The click turns a point farther than 70 cm toward first.
//! On a placed object a usable section of kind 2 is a lock: with the key
//! its first opening writes a journal line and pays 100 experience, or
//! 1000 when the section's own kind is 3; an already open lock opens its
//! container, and kinds 0 and 1 are containers too. Other kinds and an
//! object with no usable section show the item. A dead character runs its
//! roles' interact scripts, the player opens its character screen with
//! 5000 units, and a living one is the click callback's (Exarust has no
//! dialogue). The Ctrl/double-click interaction is separate: a usable item
//! is taken in hand (kind 1), a consumable (kind 0 subtype 1) is used after
//! 2100 ms, a button (kind 5, not held) is pushed, and a container opens;
//! the player opens the inventory, a corpse is looted, and a living
//! character talks when its roles' condition passes or a dialogue role
//! links it to the player.
//!
//! **Using an item.** The operative section's kind byte names the use: 8
//! is a salve that adds 0.2 to the heal pool (0.1 when the value word is
//! 50) when the body takes remedies, 14 is an elixir that sets the potion
//! flag and stamps the world clock's seconds plus 1800, 9 cleans and 10
//! repairs. Both pour kinds also mark the item (or its container's entry)
//! with 0xC80000. A cleaning kit takes an item's dirt byte down to 0x20
//! when it is over that; a repair kit lowers the wear byte by 255 over the
//! material's rating `(r + 5) / 10` (at least 1), never under zero. Both
//! schedule a timed value that moves 0.0005 of the gap a millisecond (two
//! seconds); a list entry is dropped when it arrives, and a value is
//! written only while it lowers the item's byte or arrives.
//!
//! **The reach.** A point within 245 cm of the world's floor and 300 cm of
//! the hip across the ground is in reach; a door's own place is measured to
//! 300 cm high and 350 cm across, a wheel's with the plain limits. Outside
//! combat mode a point that is not a door must also lie in the player's
//! navigation region with no flagged mesh between. A comparison with a NaN
//! passes. A walk-to point has its own world-height test (`pick::walk_within_reach`).
//!
//! **Start and end of play.** The start makes the AI's activation centre
//! the player with a 40 m radius (its square, 16 000 000 cm²); the end puts
//! the centre back at the origin with 200 m (400 000 000 cm²).
//!
//! Not wired in Exarust: the item cursor and inventory windows the click
//! and use outcomes open, and the scene script registry behind a corpse's
//! scripts and a living character's talk condition. Those outcomes are
//! modelled and tested; `exa_ai::roles` owns the script lookup itself.
//!
//! Evidence: the controls' click, interaction, use, reach, request, gate,
//! permission and start/end routines, read in the instructions
//! (`context/re/player-control-hand.md`).

use bevy::math::Vec3;
use bevy::prelude::Resource;

/// The usable section's subtypes the use dispatcher reads.
pub const POUR: u8 = 8;
pub const POUR_MARKS: u8 = 14;
pub const CLEAN: u8 = 9;
pub const REPAIR: u8 = 10;
/// The operative value word that fills the smaller share of the heal pool.
pub const SMALL_POUR_TYPE: u16 = 50;
/// The world-clock span (s) an elixir's potion flag lasts.
pub const POUR_MARK_S: u32 = exa_combat::remedy::ELIXIR_S;
/// A cleaning kit leaves the dirt byte at this.
pub const CLEAN_TO: u8 = 0x20;

/// The heal pool a salve leaves: `pool` plus 0.1 when the operative value
/// word is 50 and 0.2 otherwise, added in double precision. The use adds
/// this only when the body takes remedies ([`exa_combat::remedy::apply_remedy`]).
pub fn poured(pool: f32, liquid_type: u16) -> f32 {
    exa_combat::remedy::fill_salve(pool, liquid_type)
}

/// The cleaning kit's target: the dirt byte goes to 0x20 when it is over
/// it; at or under it nothing is cleaned.
pub fn clean_to(dirt: u8) -> Option<u8> {
    (dirt > CLEAN_TO).then_some(CLEAN_TO)
}

/// The repair kit's target: the wear byte drops by 255 over the material's
/// rating step `(rating + 5) / 10` (at least 1), never under zero; an
/// unworn item is not repaired.
pub fn repair_to(wear: u8, rating: u8) -> Option<u8> {
    if wear == 0 {
        return None;
    }
    let step = ((i32::from(rating) + 5) / 10).max(1);
    Some((i32::from(wear) - 255 / step).max(0) as u8)
}

/// A timed value on an item: the byte's kind (cleaning or repair), the
/// value it starts at and the value it stops at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timed {
    /// [`CLEAN`] or [`REPAIR`].
    pub kind: u8,
    pub from: u8,
    pub to: u8,
    /// The game clock (ms) it started.
    pub started_ms: u32,
}

/// The fraction of its gap a timed value moves each millisecond (in double
/// precision), so a full value takes two seconds.
pub const TIMED_RATE: f64 = 0.0005;

impl Timed {
    /// The value at `now_ms`: from toward to by 0.0005 of the gap a ms,
    /// rounded to the nearest (ties to even), or the end value once the gap
    /// is spent. Returns the value and whether the entry is done.
    pub fn value(&self, now_ms: u32) -> (u8, bool) {
        let elapsed = f64::from(now_ms) - f64::from(self.started_ms);
        let f = (elapsed * TIMED_RATE) as f32;
        if f < 1.0 {
            let from = f32::from(self.from);
            let to = f32::from(self.to);
            (((to - from) * f + from).round_ties_even() as u8, false)
        } else {
            (self.to, true)
        }
    }
}

/// The timed list: entries applied to an item each frame, removed when they
/// arrive. The original holds them in an array, takes a finished entry out
/// by moving the last into its place, and counts its entries with a length.
/// (It writes a value only while it lowers the item's byte or arrives;
/// Exarust returns every value and leaves that to the writer.)
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TimedList {
    pub entries: Vec<TimedItem>,
}

/// A timed entry and the item it changes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimedItem {
    pub timed: Timed,
    pub item: u32,
}

impl TimedList {
    /// Adds an entry, returning its index.
    pub fn push(&mut self, item: TimedItem) -> usize {
        self.entries.push(item);
        self.entries.len() - 1
    }

    /// One frame at `now_ms` against `current`, which gives the item's byte
    /// at section `section` (0 the usable section, 1 the weapon section).
    /// The value is written only while it lowers that byte or arrives, as
    /// the original writes it; an arriving entry is removed by moving the
    /// last into its slot. Returns the section writes in the loop's order.
    pub fn tick_sections(
        &mut self,
        now_ms: u32,
        mut current: impl FnMut(u32, u8) -> u8,
    ) -> Vec<(u32, u8, u8)> {
        let mut writes = Vec::with_capacity(self.entries.len() * 2);
        let mut i = 0;
        while i < self.entries.len() {
            let entry = self.entries[i];
            let (value, done) = entry.timed.value(now_ms);
            for section in [0u8, 1] {
                let old = current(entry.item, section);
                if old > value || done {
                    writes.push((entry.item, section, value));
                }
            }
            if done {
                self.entries.swap_remove(i);
            } else {
                i += 1;
            }
        }
        writes
    }

    /// One frame against the entries' own starting bytes: every value that
    /// lowers its start (or arrives) is returned, in the loop's order.
    pub fn tick(&mut self, now_ms: u32) -> Vec<(u32, u8)> {
        let mut writes = Vec::with_capacity(self.entries.len());
        let mut i = 0;
        while i < self.entries.len() {
            let entry = self.entries[i];
            let (value, done) = entry.timed.value(now_ms);
            if entry.timed.from > value || done {
                writes.push((entry.item, value));
            }
            if done {
                self.entries.swap_remove(i);
            } else {
                i += 1;
            }
        }
        writes
    }
}

/// The squared activation radius (cm²) the start of play gives the AI
/// around the player: 16 000 000, 40 m.
pub const START_ACTIVATION_RADIUS2: f32 = 16_000_000.0;
/// The squared activation radius (cm²) the end of play resets it to:
/// 400 000 000, 200 m.
pub const END_ACTIVATION_RADIUS2: f32 = 400_000_000.0;

/// The player-control singleton's state at creation: the hotkey permission
/// mask 31, and two flags the constructor sets to 1. Only the constructor
/// is traced: what reads the flags, and so their names, are guesses
/// [guess: controls and combat], and nothing in Exarust reads this
/// resource. The running player keeps its combat state on the body
/// (`exa_game::player::PlayerBody`), which starts out of combat.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerControl {
    /// The hotkey permission mask: 31 at creation.
    pub hotkey_mask: u32,
    /// The constructor's first flag: 1 [guess: the controls flag].
    pub controls: bool,
    /// The constructor's second flag: 1 [guess: the combat flag].
    pub combat: bool,
}

impl Default for PlayerControl {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayerControl {
    /// The singleton as its constructor leaves it.
    pub fn new() -> Self {
        Self {
            hotkey_mask: 31,
            controls: true,
            combat: true,
        }
    }
}

/// The session state the start and end of play set: the AI activation
/// centre (m) and squared radius (cm²), and whether the player's controls
/// are in play.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlaySession {
    pub activation_centre: Vec3,
    pub activation_radius2: f32,
    /// The player's controls run (the original's playing flag).
    pub playing: bool,
}

impl Default for PlaySession {
    fn default() -> Self {
        Self {
            activation_centre: Vec3::ZERO,
            activation_radius2: END_ACTIVATION_RADIUS2,
            playing: false,
        }
    }
}

impl PlaySession {
    /// The start of play at the player's `centre`: the controls run and the
    /// AI activates around the player (a 40 m radius).
    pub fn begin(&mut self, centre: Vec3) {
        self.playing = true;
        self.activation_centre = centre;
        self.activation_radius2 = START_ACTIVATION_RADIUS2;
    }

    /// The end of play: the controls stop, the activation centre goes back
    /// to the origin with a 200 m radius.
    pub fn end(&mut self) {
        self.playing = false;
        self.activation_centre = Vec3::ZERO;
        self.activation_radius2 = END_ACTIVATION_RADIUS2;
    }
}

/// A point farther than this (cm) is turned toward before it is used.
pub const TURN_BEYOND_CM: f32 = 70.0;
/// A point is used from at most this height (cm).
pub const REACH_HEIGHT_CM: f32 = 245.0;
/// A door's reach: 300 cm high and 350 cm across; a wheel's is the plain
/// 300 cm across.
pub const DOOR_HEIGHT_CM: f32 = 300.0;
pub const DOOR_ACROSS_CM: f32 = 350.0;
pub const REACH_ACROSS_CM: f32 = 300.0;
/// A door unlock's reward: 100 for a lock, 1000 for the main door kind 3.
pub const UNLOCK_REWARD: u32 = 100;
pub const UNLOCK_REWARD_MAIN: u32 = 1000;
/// The usable section's kinds: a lock, the main door, a wheel, a crank and
/// a button.
pub const USABLE_LOCK: u8 = 2;
pub const USABLE_MAIN: u8 = 3;
pub const USABLE_BUTTON: u8 = 5;
/// A character screen opens with 5000 units of its own.
pub const CHARACTER_SCREEN_COST: u32 = 5000;
/// A consumable of usable kind 0, subtype 1, is used this many ms after
/// the interaction.
pub const CONSUME_MS: u32 = 2100;

/// What the click (and the Ctrl/double-click interaction) sees under the
/// cursor: a placed object's sections, or a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickTarget {
    /// Nothing was hit.
    Nothing,
    /// A placed object. `usable` is its usable section's kind byte when it
    /// has one, `subtype` its subtype byte; `lock_open` and `reward_ready`
    /// are the lock section's two words, `key_ok` the player's key test,
    /// `container` whether it has a container section, `first_touch` the
    /// node's first-touch bit, and `push_released` whether a button's own
    /// flag word allows it to fire.
    Placed {
        usable: Option<u8>,
        subtype: u8,
        lock_open: bool,
        reward_ready: bool,
        key_ok: bool,
        container: bool,
        first_touch: bool,
        push_released: bool,
    },
    /// A character.
    Player,
    DeadCharacter,
    LivingCharacter,
}

impl ClickTarget {
    /// A placed object with no usable or container section and no lock.
    pub fn plain() -> Self {
        ClickTarget::Placed {
            usable: None,
            subtype: 0,
            lock_open: false,
            reward_ready: false,
            key_ok: false,
            container: false,
            first_touch: false,
            push_released: true,
        }
    }
}

/// What a click does after the turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickOutcome {
    Nothing,
    /// Show the item (its card or the narrator's line).
    Inspect,
    /// Open the container's window.
    OpenContainer,
    /// Unlock a lock: the journal line and the experience it pays.
    Unlock {
        reward: u32,
    },
    /// The click callback (the living character's line, or an object's
    /// own click).
    Callback,
    /// The player's own character screen.
    CharacterScreen {
        experience: u32,
    },
    /// A dead character's roles' interact scripts.
    CorpseScripts,
}

/// The first-touch bit of a placed object is cleared by any hit.
pub fn click(target: ClickTarget) -> ClickOutcome {
    match target {
        ClickTarget::Nothing => ClickOutcome::Nothing,
        ClickTarget::Player => ClickOutcome::CharacterScreen {
            experience: CHARACTER_SCREEN_COST,
        },
        ClickTarget::DeadCharacter => ClickOutcome::CorpseScripts,
        ClickTarget::LivingCharacter => ClickOutcome::Callback,
        ClickTarget::Placed {
            usable,
            lock_open,
            reward_ready,
            key_ok,
            container,
            ..
        } => match usable {
            Some(USABLE_LOCK) => {
                if lock_open {
                    ClickOutcome::OpenContainer
                } else if key_ok && reward_ready {
                    ClickOutcome::Unlock {
                        reward: UNLOCK_REWARD,
                    }
                } else {
                    ClickOutcome::Nothing
                }
            }
            Some(USABLE_MAIN) => {
                if lock_open {
                    ClickOutcome::OpenContainer
                } else if key_ok && reward_ready {
                    ClickOutcome::Unlock {
                        reward: UNLOCK_REWARD_MAIN,
                    }
                } else {
                    ClickOutcome::Nothing
                }
            }
            Some(kind) if kind < USABLE_LOCK => {
                if container {
                    ClickOutcome::OpenContainer
                } else {
                    ClickOutcome::Inspect
                }
            }
            Some(_) | None => {
                if usable.is_none() && container {
                    ClickOutcome::OpenContainer
                } else {
                    ClickOutcome::Inspect
                }
            }
        },
    }
}

/// What the Ctrl/double-click interaction does on a placed object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectInteraction {
    Nothing,
    /// Use kind 0, subtype 1: consumed after [`CONSUME_MS`].
    Consume {
        due_ms: u32,
    },
    /// Use kind 0, other subtype, or kind 0 with no subtype: open the item.
    Open,
    /// Take the usable item in hand (kind 1).
    UseInHand,
    /// Push a button (kind 5, not held).
    Push,
    /// Open the container section.
    OpenContainer,
    /// Show the item.
    Inspect,
}

/// The placed object's interaction, as the double click's handler decides
/// it. `first_touch` gates kind 0 and 1; `push_released` gates the button.
pub fn interact_object(target: ClickTarget, now_ms: u32) -> ObjectInteraction {
    let ClickTarget::Placed {
        usable,
        subtype,
        container,
        first_touch,
        push_released,
        ..
    } = target
    else {
        return ObjectInteraction::Nothing;
    };
    let Some(kind) = usable else {
        return if container {
            ObjectInteraction::OpenContainer
        } else {
            ObjectInteraction::Inspect
        };
    };
    match kind {
        0 => {
            if !first_touch {
                ObjectInteraction::Nothing
            } else if subtype == 1 {
                ObjectInteraction::Consume {
                    due_ms: now_ms.wrapping_add(CONSUME_MS),
                }
            } else {
                ObjectInteraction::Open
            }
        }
        1 => {
            if first_touch {
                ObjectInteraction::UseInHand
            } else {
                ObjectInteraction::Nothing
            }
        }
        USABLE_BUTTON => {
            if push_released {
                ObjectInteraction::Push
            } else {
                ObjectInteraction::Nothing
            }
        }
        2 | USABLE_MAIN => {
            if container {
                ObjectInteraction::OpenContainer
            } else {
                ObjectInteraction::Inspect
            }
        }
        _ => {
            if container {
                ObjectInteraction::OpenContainer
            } else {
                ObjectInteraction::Inspect
            }
        }
    }
}

/// What the Ctrl/double-click interaction does on a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterInteraction {
    Inventory,
    Loot,
    /// The roles' talk condition passed.
    Talk,
    /// A dialogue role links the character to the player.
    Dialogue,
    Nothing,
}

/// The character interaction: the player's inventory, a corpse's loot, or a
/// living character's talk condition and dialogue role.
pub fn interact_character(
    target: ClickTarget,
    condition_passes: bool,
    role_links_player: bool,
) -> CharacterInteraction {
    match target {
        ClickTarget::Player => CharacterInteraction::Inventory,
        ClickTarget::DeadCharacter => CharacterInteraction::Loot,
        ClickTarget::LivingCharacter => {
            if condition_passes {
                CharacterInteraction::Talk
            } else if role_links_player {
                CharacterInteraction::Dialogue
            } else {
                CharacterInteraction::Nothing
            }
        }
        _ => CharacterInteraction::Nothing,
    }
}

/// Whether the Ctrl/double-click interaction may run: in reach, no modal
/// form, no GUI drag callback, not spectating, a player, and in combat mode
/// only with Ctrl held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InteractGate {
    pub out_of_reach: bool,
    pub modal: bool,
    pub gui_callback: bool,
    pub spectate: bool,
    pub player: bool,
    pub combat: bool,
    pub ctrl: bool,
}

pub fn interaction_allowed(g: InteractGate) -> bool {
    !g.out_of_reach
        && !g.modal
        && !g.gui_callback
        && !g.spectate
        && g.player
        && (!g.combat || g.ctrl)
}

/// A second press this many ms after the first, under 2 pixels from it, is
/// a double click.
pub const DOUBLE_CLICK_MS: u64 = 450;
pub const DOUBLE_CLICK_PX: i64 = 2;

/// Whether a press at `at`, `now_ms`, is the double click of the press at
/// `was`, `was_ms`: strictly under 450 ms and strictly under 2 pixels on
/// both axes.
pub fn is_double_click(was: (i32, i32), was_ms: u64, at: (i32, i32), now_ms: u64) -> bool {
    now_ms < was_ms + DOUBLE_CLICK_MS
        && (i64::from(at.0) - i64::from(was.0)).abs() < DOUBLE_CLICK_PX
        && (i64::from(at.1) - i64::from(was.1)).abs() < DOUBLE_CLICK_PX
}

/// Where a walk-to use stops: the clicked point (m) and the player's feet
/// (m). A point beyond 245 cm (350 cm for a door) leaves the walk's goal
/// that far short along the ground; a nearer point needs no walk.
pub fn walk_stop(at: Vec3, point: Vec3, door: bool) -> Option<Vec3> {
    let flat = (point - at).with_y(0.0);
    let distance = flat.length() * 100.0;
    let stop = if door {
        DOOR_ACROSS_CM
    } else {
        REACH_HEIGHT_CM
    };
    if distance > stop && distance > 0.0 {
        Some(at + flat.normalize_or_zero() * ((distance - stop) / 100.0))
    } else {
        None
    }
}

/// A sound a use asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseSoundKind {
    Pour,
    Clean,
    Repair,
}

/// A sound the use plays: its kind and the unit random draw for its pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UseSound {
    pub kind: UseSoundKind,
    pub volume: f32,
    /// `0.9 + 0.2·draw`.
    pub pitch: f32,
    /// `true` at the point taken, `false` at the player.
    pub at_point: bool,
}

/// The use a clicked object starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UseOutcome {
    Nothing,
    /// A salve added this much to the target's heal pool (double precision).
    Salve(f32),
    /// An elixir: its potion flag and the world-seconds stamp it lasts to.
    Elixir {
        flag: u32,
        until_s: u32,
    },
    /// A timed value on the item's sections.
    Timed(Timed),
}

/// The complete use: what it does, the marks it sets on the item or its
/// container entry (0xC80000 for both pour kinds), and its sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Use {
    pub outcome: UseOutcome,
    pub marks: u32,
    pub sound: Option<UseSound>,
}

/// The pour kinds' mark on the item or its container entry.
pub const POUR_MARK: u32 = 0xC80000;
/// The elixir's potion flag on the body, and the world seconds it lasts.
pub const POTION_FLAG: u32 = exa_combat::condition::StateFlags::POTION;
pub const ELIXIR_S: u32 = exa_combat::remedy::ELIXIR_S;

/// A salve's heal-pool addition: 0.2, or 0.1 when the value word is 50, in
/// double precision. Only when the body takes remedies.
pub fn salve(liquid_type: u16, takes_remedies: bool) -> Option<f32> {
    takes_remedies.then_some(if liquid_type == SMALL_POUR_TYPE {
        0.1
    } else {
        0.2
    })
}

/// An elixir: the potion flag and the world-clock second it lasts to, only
/// when the body takes remedies.
pub fn elixir(takes_remedies: bool, world_clock_s: u32) -> Option<(u32, u32)> {
    takes_remedies.then_some((POTION_FLAG, world_clock_s.wrapping_add(ELIXIR_S)))
}

/// A use's pitch: `0.9 + 0.2·draw`, then its volume by kind.
pub fn use_sound(kind: UseSoundKind, draw: f32, at_point: bool) -> UseSound {
    let volume = match kind {
        UseSoundKind::Pour => 0.25,
        UseSoundKind::Clean => 0.3,
        UseSoundKind::Repair => 0.15,
    };
    UseSound {
        kind,
        volume,
        pitch: draw * 0.2 + 0.9,
        at_point,
    }
}

/// The use of an item whose usable kind is `kind` on the world: a pour
/// kind 8 or 14 marks the target and (when the body takes remedies) warms
/// or lights it; a cleaning or repair kit schedules its timed byte.
#[allow(clippy::too_many_arguments)]
pub fn use_on(
    kind: u8,
    value_word: u16,
    takes_remedies: bool,
    world_clock_s: u32,
    wear: Option<u8>,
    dirt: Option<u8>,
    rating: u8,
    now_ms: u32,
    sound_draw: f32,
    at_point: bool,
) -> Use {
    match kind {
        POUR => Use {
            outcome: salve(value_word, takes_remedies)
                .map_or(UseOutcome::Nothing, UseOutcome::Salve),
            marks: POUR_MARK,
            sound: None,
        },
        POUR_MARKS => Use {
            outcome: elixir(takes_remedies, world_clock_s).map_or(
                UseOutcome::Nothing,
                |(flag, until_s)| UseOutcome::Elixir { flag, until_s },
            ),
            marks: POUR_MARK,
            sound: Some(use_sound(UseSoundKind::Pour, sound_draw, at_point)),
        },
        CLEAN => {
            let timed = dirt.and_then(clean_timed).map(|mut t| {
                t.started_ms = now_ms;
                t
            });
            Use {
                outcome: timed.map_or(UseOutcome::Nothing, UseOutcome::Timed),
                marks: 0,
                sound: timed
                    .is_some()
                    .then(|| use_sound(UseSoundKind::Clean, sound_draw, at_point)),
            }
        }
        REPAIR => {
            let timed = wear.and_then(|w| repair_timed(w, rating)).map(|mut t| {
                t.started_ms = now_ms;
                t
            });
            Use {
                outcome: timed.map_or(UseOutcome::Nothing, UseOutcome::Timed),
                marks: 0,
                sound: timed
                    .is_some()
                    .then(|| use_sound(UseSoundKind::Repair, sound_draw, at_point)),
            }
        }
        _ => Use {
            outcome: UseOutcome::Nothing,
            marks: 0,
            sound: None,
        },
    }
}

impl UseOutcome {
    /// The timed entry a use leaves, if any.
    pub fn timed(self) -> Option<Timed> {
        match self {
            UseOutcome::Timed(t) => Some(t),
            _ => None,
        }
    }
}

/// The cleaning kit's timed value: from the current dirt down to 0x20,
/// when the dirt is over it.
pub fn clean_timed(dirt: u8) -> Option<Timed> {
    (dirt > CLEAN_TO).then_some(Timed {
        kind: CLEAN,
        from: dirt,
        to: CLEAN_TO,
        started_ms: 0,
    })
}

/// The repair kit's timed value: from the current wear down by 255 over
/// the material's `(rating + 5) / 10` (at least 1), never under zero. An
/// unworn item is not repaired.
pub fn repair_timed(wear: u8, rating: u8) -> Option<Timed> {
    repair_to(wear, rating).map(|to| Timed {
        kind: REPAIR,
        from: wear,
        to,
        started_ms: 0,
    })
}

/// A scene object that carries a script the player's use can run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptObject {
    pub at: Vec3,
    pub object: u32,
}

/// The script objects within the use's reach: a node's distance squared
/// from the point under 5 cm² and an object id. The scene walks its object
/// nodes in order and keeps that order.
pub fn nearby_scripts(point: Vec3, objects: impl IntoIterator<Item = ScriptObject>) -> Vec<u32> {
    objects
        .into_iter()
        .filter(|o| o.object != 0 && (o.at - point).length_squared() < 5.0)
        .map(|o| o.object)
        .collect()
}

/// Where a use's marks go: the world item itself, or a container's entry
/// `entry` with the entry's own word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkTarget {
    Item,
    Container { entry: u32 },
}

/// The words a pour marks: the item's flags word, or the container's entry
/// word, ORed with 0xC80000. A container entry word of zero marks nothing.
pub fn mark_used(target: MarkTarget, flags: u32) -> Option<u32> {
    match target {
        MarkTarget::Item => Some(flags | POUR_MARK),
        MarkTarget::Container { entry: 0 } => None,
        MarkTarget::Container { entry } => Some(entry | POUR_MARK),
    }
}

/// The controls' movement request mode, from the hand's drag state and the
/// combat branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestMode {
    /// The W/S/right-button request runs.
    Normal,
    /// Combat mode measures the reach and keeps no request.
    Combat,
    /// The lift places its object; no movement request.
    Lift,
    /// A door or wheel drag leaves the request to the keys and the right
    /// button.
    Mechanism,
    /// A refused press keeps the request as it was.
    Refused,
    /// An item use's reach only.
    Use,
}

/// The movement request's mode, in the original's branch order: down or
/// dead first, then the use, the lift, a door or wheel, a refusal, combat
/// and the normal request.
pub fn request_mode(state: crate::hand::DragState, combat: bool) -> RequestMode {
    use crate::hand::DragState;
    match state {
        DragState::Use => RequestMode::Use,
        DragState::Lift | DragState::GuiPending | DragState::GuiCallback => RequestMode::Lift,
        DragState::Door | DragState::Wheel | DragState::Cloth => RequestMode::Mechanism,
        DragState::Refused => RequestMode::Refused,
        DragState::Idle | DragState::Physical if combat => RequestMode::Combat,
        _ => RequestMode::Normal,
    }
}

/// The controls' gates as the frame reads them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ControlGates {
    /// The controls are off (`0xac`).
    pub off: bool,
    /// A screen blocks the controls (`0xad`).
    pub blocked: bool,
    /// A second block (`0xaf`).
    pub second_block: bool,
    /// The spectating view (`0xb0`).
    pub spectate: bool,
}

/// What the controls' frame does this time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameAction {
    /// Run the request and the idle turn.
    Run,
    /// Turn the camera as a blocked screen and stop.
    BlockedTurn,
    /// Nothing (spectating, off or a second block).
    Stop,
    /// The active view is not the game's: stamp and stop.
    Stamp,
}

/// The frame's order: spectate, no player, a blocked screen, the controls
/// off, the second block, then the active view.
pub fn frame_action(gates: ControlGates, player: bool, game_view_active: bool) -> FrameAction {
    if gates.spectate || !player {
        FrameAction::Stop
    } else if gates.blocked {
        FrameAction::BlockedTurn
    } else if gates.off || gates.second_block {
        FrameAction::Stop
    } else if !game_view_active {
        FrameAction::Stamp
    } else {
        FrameAction::Run
    }
}

/// The permissions the key handler is given: all five in play, four while
/// a spirit form or an open document holds the controls.
pub const PERMISSIONS_PLAY: u32 = 0x1F;
pub const PERMISSIONS_POWERS: u32 = 0x4;

/// The key permissions for a frame: all five normally, four while a modal
/// document (a dialogue or a spirit form) holds the controls.
pub fn permissions(document_open: bool) -> u32 {
    if document_open {
        PERMISSIONS_POWERS
    } else {
        PERMISSIONS_PLAY
    }
}

/// The power bar's hold on a picked slot: the slot (from 1) and the game
/// clock (ms) the hold ends, 500 ms after the pick.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PowerHold {
    pub slot: u8,
    pub until_ms: u32,
}

pub const POWER_HOLD_MS: u32 = 500;

impl PowerHold {
    /// Picks `slot` (from 0) at `now_ms`, holding it for 500 ms (the
    /// original stores the slot plus one and stamps `now + 500`).
    pub fn pick(slot: i32, now_ms: u32) -> Self {
        Self {
            slot: (slot + 1).max(0) as u8,
            until_ms: now_ms.wrapping_add(POWER_HOLD_MS),
        }
    }

    /// One frame: the bar's command this frame, `Some(slot)` (from 0) while
    /// the hold stands and `Some(-1)` (use the picked power) once it has
    /// expired. The caller clears the hold when it takes the command.
    pub fn frame(&mut self, now_ms: u32) -> Option<i32> {
        if self.slot == 0 {
            return None;
        }
        if now_ms > self.until_ms {
            self.slot = 0;
        }
        Some(i32::from(self.slot) - 1)
    }

    /// The command was taken.
    pub fn taken(&mut self) {
        self.slot = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A salve fills the heal pool by 0.2, or 0.1 when the value word is
    /// 50, in double precision: 0.3 + 0.2 is 0.5 exactly once narrowed.
    #[test]
    fn a_salve_fills_in_double_precision() {
        assert_eq!(poured(0.3, 1), 0.5);
        assert_eq!(poured(0.3, SMALL_POUR_TYPE), 0.4);
        assert_eq!(poured(0.0, 7), 0.2);
    }

    /// The cleaning kit takes the dirt to 0x20 from over it; the repair kit
    /// lowers the wear by 255 over the rating step, floored at zero, and
    /// leaves an unworn item.
    #[test]
    fn cleaning_and_repairing() {
        assert_eq!(clean_to(0x30), Some(0x20));
        assert_eq!(clean_to(0x21), Some(0x20));
        assert_eq!(clean_to(0x20), None);
        assert_eq!(clean_to(0), None);
        // rating 25 → (25 + 5)/10 = 3 → 255/3 = 85 off.
        assert_eq!(repair_to(100, 25), Some(15));
        assert_eq!(repair_to(50, 25), Some(0));
        // rating 0 → step 1 → 255 off, floored at 0.
        assert_eq!(repair_to(100, 0), Some(0));
        // rating 45 → 5 → 51 off.
        assert_eq!(repair_to(100, 45), Some(49));
        assert_eq!(repair_to(0, 45), None);
    }

    /// A timed value moves 0.0005 of its gap a millisecond, rounding to the
    /// nearest byte (ties to even), and is exactly its end at two seconds;
    /// a clock behind its start runs it backwards.
    #[test]
    fn a_timed_value_eases_over_two_seconds() {
        let t = Timed {
            kind: 10,
            from: 100,
            to: 0,
            started_ms: 1000,
        };
        assert_eq!(t.value(1000), (100, false));
        assert_eq!(t.value(2000), (50, false));
        assert_eq!(t.value(2500), (25, false));
        assert_eq!(t.value(3000), (0, true));
        assert_eq!(t.value(9000), (0, true));
        // 0.0005 of a 32 gap at 15 ms is 0.24: 48 − 0.24 = 47.76, which
        // the nearest-even cast takes to 48.
        let small = Timed {
            kind: 9,
            from: 0x30,
            to: 0x10,
            started_ms: 0,
        };
        assert_eq!(small.value(15), (48, false));
        // 100 ms before its start: f = −0.05, 100 + 5 = 105.
        assert_eq!(t.value(900), (105, false));
    }

    /// The list applies every entry and takes out the finished ones by
    /// moving the last into their slot.
    #[test]
    fn the_timed_list_drops_finished_entries() {
        let mut list = TimedList::default();
        list.push(TimedItem {
            timed: Timed {
                kind: 10,
                from: 100,
                to: 0,
                started_ms: 0,
            },
            item: 7,
        });
        list.push(TimedItem {
            timed: Timed {
                kind: 9,
                from: 0x30,
                to: 0x10,
                started_ms: 0,
            },
            item: 9,
        });
        let at = |list: &mut TimedList, ms| list.tick(ms);
        assert_eq!(at(&mut list, 1000), vec![(7, 50), (9, 0x20)]);
        assert_eq!(list.entries.len(), 2);
        assert_eq!(at(&mut list, 2000), vec![(7, 0), (9, 0x10)]);
        assert!(list.entries.is_empty());
    }

    /// The start of play centres the AI's activation on the player with
    /// 40 m; the end resets it to the origin with 200 m.
    #[test]
    fn the_start_and_end_of_play() {
        let mut s = PlaySession::default();
        assert!(!s.playing);
        s.begin(Vec3::new(10.0, 0.0, 20.0));
        assert!(s.playing);
        assert_eq!(s.activation_centre, Vec3::new(10.0, 0.0, 20.0));
        assert_eq!(s.activation_radius2, 16_000_000.0);
        s.end();
        assert!(!s.playing);
        assert_eq!(s.activation_centre, Vec3::ZERO);
        assert_eq!(s.activation_radius2, 400_000_000.0);
        // The squared radii root to 40 m and 200 m.
        assert_eq!(START_ACTIVATION_RADIUS2.sqrt(), 4000.0);
        assert_eq!(END_ACTIVATION_RADIUS2.sqrt(), 20_000.0);
    }

    /// The player-control singleton starts with the hotkey mask 31 and both
    /// of the constructor's flags set.
    #[test]
    fn the_player_control_starts_as_its_constructor() {
        let c = PlayerControl::new();
        assert_eq!(c.hotkey_mask, 31);
        assert!(c.controls);
        assert!(c.combat);
        assert_eq!(PlayerControl::default(), c);
    }

    #[allow(clippy::too_many_arguments)]
    fn placed_with(
        usable: Option<u8>,
        subtype: u8,
        lock_open: bool,
        reward_ready: bool,
        key_ok: bool,
        container: bool,
        first_touch: bool,
        push_released: bool,
    ) -> ClickTarget {
        ClickTarget::Placed {
            usable,
            subtype,
            lock_open,
            reward_ready,
            key_ok,
            container,
            first_touch,
            push_released,
        }
    }

    fn placed(usable: Option<u8>, subtype: u8) -> ClickTarget {
        placed_with(usable, subtype, false, false, false, false, true, true)
    }

    /// A plain lock needs the key and pays 100 or 1000 once; an open lock
    /// opens the container; kinds 0 and 1 are containers; other kinds and
    /// no usable section inspect. A character is its screen, scripts or a
    /// callback.
    #[test]
    fn a_click_names_the_object_and_the_character() {
        assert_eq!(click(ClickTarget::Nothing), ClickOutcome::Nothing);
        assert_eq!(
            click(ClickTarget::Player),
            ClickOutcome::CharacterScreen { experience: 5_000 }
        );
        assert_eq!(
            click(ClickTarget::DeadCharacter),
            ClickOutcome::CorpseScripts
        );
        assert_eq!(click(ClickTarget::LivingCharacter), ClickOutcome::Callback);
        let lock = placed(Some(USABLE_LOCK), 0);
        assert_eq!(click(lock), ClickOutcome::Nothing);
        let key = placed_with(Some(USABLE_LOCK), 0, false, true, true, false, true, true);
        assert_eq!(click(key), ClickOutcome::Unlock { reward: 100 });
        let main = placed_with(Some(USABLE_MAIN), 0, false, true, true, false, true, true);
        assert_eq!(click(main), ClickOutcome::Unlock { reward: 1000 });
        let open = placed_with(Some(USABLE_LOCK), 0, true, true, true, false, true, true);
        assert_eq!(click(open), ClickOutcome::OpenContainer);
        let container = placed_with(Some(1), 0, false, false, false, true, true, true);
        assert_eq!(click(container), ClickOutcome::OpenContainer);
        assert_eq!(click(placed(Some(1), 0)), ClickOutcome::Inspect);
        assert_eq!(click(placed(Some(USABLE_BUTTON), 0)), ClickOutcome::Inspect);
        assert_eq!(click(ClickTarget::plain()), ClickOutcome::Inspect);
    }

    /// Kind 0 subtype 1 consumes after 2100 ms; another subtype opens; kind
    /// 1 takes the item; kind 5 pushes when released; containers open.
    #[test]
    fn the_interaction_reads_the_usable_section() {
        assert_eq!(
            interact_object(placed(Some(0), 1), 1_000),
            ObjectInteraction::Consume { due_ms: 3_100 }
        );
        assert_eq!(
            interact_object(placed(Some(0), 0), 0),
            ObjectInteraction::Open
        );
        assert_eq!(
            interact_object(
                placed_with(Some(0), 1, false, false, false, false, false, true),
                0
            ),
            ObjectInteraction::Nothing
        );
        assert_eq!(
            interact_object(placed(Some(1), 0), 0),
            ObjectInteraction::UseInHand
        );
        assert_eq!(
            interact_object(placed(Some(USABLE_BUTTON), 0), 0),
            ObjectInteraction::Push
        );
        assert_eq!(
            interact_object(
                placed_with(
                    Some(USABLE_BUTTON),
                    0,
                    false,
                    false,
                    false,
                    false,
                    true,
                    false
                ),
                0
            ),
            ObjectInteraction::Nothing
        );
        let container = placed_with(Some(2), 0, false, false, false, true, true, true);
        assert_eq!(
            interact_object(container, 0),
            ObjectInteraction::OpenContainer
        );
        assert_eq!(
            interact_character(ClickTarget::Player, false, false),
            CharacterInteraction::Inventory
        );
        assert_eq!(
            interact_character(ClickTarget::DeadCharacter, false, false),
            CharacterInteraction::Loot
        );
        assert_eq!(
            interact_character(ClickTarget::LivingCharacter, true, false),
            CharacterInteraction::Talk
        );
        assert_eq!(
            interact_character(ClickTarget::LivingCharacter, false, true),
            CharacterInteraction::Dialogue
        );
        assert_eq!(
            interact_character(ClickTarget::LivingCharacter, false, false),
            CharacterInteraction::Nothing
        );
    }

    /// The interaction gate: in reach, no modal or GUI callback, not
    /// spectating, a player, and combat mode only with Ctrl.
    #[test]
    fn the_interaction_gate() {
        let ok = InteractGate {
            out_of_reach: false,
            modal: false,
            gui_callback: false,
            spectate: false,
            player: true,
            combat: false,
            ctrl: false,
        };
        assert!(interaction_allowed(ok));
        for blocked in [
            InteractGate {
                out_of_reach: true,
                ..ok
            },
            InteractGate { modal: true, ..ok },
            InteractGate {
                gui_callback: true,
                ..ok
            },
            InteractGate {
                spectate: true,
                ..ok
            },
            InteractGate {
                player: false,
                ..ok
            },
        ] {
            assert!(!interaction_allowed(blocked));
        }
        assert!(!interaction_allowed(InteractGate {
            combat: true,
            ctrl: false,
            ..ok
        }));
        assert!(interaction_allowed(InteractGate {
            combat: true,
            ctrl: true,
            ..ok
        }));
    }

    /// A double click is strictly under 450 ms and under 2 pixels on both
    /// axes.
    #[test]
    fn the_double_click_window() {
        assert!(is_double_click((10, 10), 1000, (11, 9), 1449));
        assert!(!is_double_click((10, 10), 1000, (11, 9), 1450));
        assert!(!is_double_click((10, 10), 1000, (12, 10), 1100));
        assert!(!is_double_click((10, 10), 1000, (10, 12), 1100));
    }

    /// A liquid pours 0.1 for the 50 word and 0.2 otherwise, only when the
    /// body takes remedies; both pour kinds mark the item 0xC80000. An
    /// elixir stamps the world clock plus 1800 s.
    #[test]
    fn a_pour_warms_or_lights_and_marks() {
        assert_eq!(salve(50, true), Some(0.1));
        assert_eq!(salve(7, true), Some(0.2));
        assert_eq!(salve(7, false), None);
        let pour = use_on(8, 7, true, 0, None, None, 0, 0, 0.5, false);
        assert_eq!(pour.outcome, UseOutcome::Salve(0.2));
        assert_eq!(pour.marks, POUR_MARK);
        assert_eq!(pour.sound, None);
        let cold = use_on(8, 7, false, 0, None, None, 0, 0, 0.5, false);
        assert_eq!(cold.outcome, UseOutcome::Nothing);
        assert_eq!(cold.marks, POUR_MARK);
        let light = use_on(14, 7, true, 100, None, None, 0, 0, 0.5, true);
        assert_eq!(
            light.outcome,
            UseOutcome::Elixir {
                flag: POTION_FLAG,
                until_s: 1900
            }
        );
        assert_eq!(light.marks, POUR_MARK);
        let sound = light.sound.unwrap();
        assert_eq!(sound.kind, UseSoundKind::Pour);
        assert_eq!(sound.volume, 0.25);
        assert!((sound.pitch - 1.0).abs() < 1e-6);
        assert!(sound.at_point);
        assert_eq!(elixir(false, 100), None);
    }

    /// A cleaning kit stops at 0x20; a repair kit takes 255 over the rating
    /// step and an unworn item is left. Both start their timed value now.
    #[test]
    fn a_kit_schedules_its_timed_value() {
        let clean = use_on(9, 0, true, 0, None, Some(0x30), 0, 1000, 0.0, false);
        let timed = clean.outcome.timed().unwrap();
        assert_eq!(
            (timed.kind, timed.from, timed.to, timed.started_ms),
            (9, 0x30, 0x20, 1000)
        );
        assert_eq!(clean.sound.unwrap().volume, 0.3);
        let repair = use_on(10, 0, true, 0, Some(100), None, 25, 2000, 0.0, false);
        let timed = repair.outcome.timed().unwrap();
        assert_eq!(
            (timed.kind, timed.from, timed.to, timed.started_ms),
            (10, 100, 15, 2000)
        );
        assert_eq!(repair.sound.unwrap().volume, 0.15);
        assert_eq!(
            use_on(10, 0, true, 0, Some(0), None, 25, 0, 0.0, false).outcome,
            UseOutcome::Nothing
        );
        assert_eq!(clean_timed(0x20), None);
        assert_eq!(repair_timed(0, 25), None);
    }

    /// The timed list writes a section only while the byte lowers or the
    /// entry arrives, applies the same value to both sections, and removes
    /// an arrived entry by moving the last into its slot.
    #[test]
    fn the_timed_list_writes_only_what_lowers() {
        let mut list = TimedList::default();
        list.push(TimedItem {
            timed: Timed {
                kind: 10,
                from: 100,
                to: 0,
                started_ms: 1000,
            },
            item: 7,
        });
        // At the start neither section changes: no write.
        assert!(list.tick_sections(1000, |_, _| 100).is_empty());
        // Halfway: both sections go to 50.
        assert_eq!(
            list.tick_sections(2000, |_, _| 100),
            vec![(7, 0, 50), (7, 1, 50)]
        );
        // One section already lower than the new value: only the other writes.
        assert_eq!(
            list.tick_sections(2500, |_, section| if section == 0 { 20 } else { 50 }),
            vec![(7, 1, 25)]
        );
        // Done writes both even when one is already at the target.
        assert_eq!(
            list.tick_sections(3000, |_, _| 0),
            vec![(7, 0, 0), (7, 1, 0)]
        );
        assert!(list.entries.is_empty());
        // A clock behind the start raises the value and writes nothing.
        list.push(TimedItem {
            timed: Timed {
                kind: 9,
                from: 0x30,
                to: 0x20,
                started_ms: 1000,
            },
            item: 9,
        });
        assert_eq!(list.tick_sections(900, |_, _| 0x30,), Vec::new());
    }

    /// The movement request's mode branch order.
    #[test]
    fn the_request_mode() {
        use crate::hand::DragState;
        assert_eq!(request_mode(DragState::Idle, false), RequestMode::Normal);
        assert_eq!(request_mode(DragState::Idle, true), RequestMode::Combat);
        assert_eq!(request_mode(DragState::Physical, true), RequestMode::Combat);
        assert_eq!(
            request_mode(DragState::Physical, false),
            RequestMode::Normal
        );
        assert_eq!(request_mode(DragState::Use, false), RequestMode::Use);
        assert_eq!(request_mode(DragState::Lift, false), RequestMode::Lift);
        assert_eq!(
            request_mode(DragState::GuiPending, false),
            RequestMode::Lift
        );
        assert_eq!(request_mode(DragState::Door, false), RequestMode::Mechanism);
        assert_eq!(
            request_mode(DragState::Wheel, false),
            RequestMode::Mechanism
        );
        assert_eq!(
            request_mode(DragState::Cloth, false),
            RequestMode::Mechanism
        );
        assert_eq!(
            request_mode(DragState::Refused, false),
            RequestMode::Refused
        );
    }

    /// The frame's gate order and the key permissions.
    #[test]
    fn the_control_gates() {
        let run = ControlGates::default();
        assert_eq!(frame_action(run, true, true), FrameAction::Run);
        assert_eq!(frame_action(run, false, true), FrameAction::Stop);
        assert_eq!(
            frame_action(
                ControlGates {
                    spectate: true,
                    ..run
                },
                true,
                true
            ),
            FrameAction::Stop
        );
        assert_eq!(
            frame_action(
                ControlGates {
                    blocked: true,
                    ..run
                },
                true,
                false
            ),
            FrameAction::BlockedTurn
        );
        assert_eq!(
            frame_action(ControlGates { off: true, ..run }, true, true),
            FrameAction::Stop
        );
        assert_eq!(
            frame_action(
                ControlGates {
                    second_block: true,
                    ..run
                },
                true,
                true
            ),
            FrameAction::Stop
        );
        assert_eq!(frame_action(run, true, false), FrameAction::Stamp);
        assert_eq!(permissions(false), PERMISSIONS_PLAY);
        assert_eq!(permissions(true), PERMISSIONS_POWERS);
    }

    /// The power bar's hold selects its slot every frame and uses it at
    /// 500 ms; the caller clears it.
    #[test]
    fn the_power_hold() {
        let mut hold = PowerHold::pick(2, 1000);
        assert_eq!(hold.frame(1000), Some(2));
        assert_eq!(hold.frame(1500), Some(2));
        assert_eq!(hold.frame(1501), Some(-1));
        hold.taken();
        assert_eq!(hold.frame(1600), None);
        let mut no = PowerHold::pick(-1, 0);
        assert_eq!(no.frame(0), None);
    }

    /// Script objects within 5 cm² of the point keep their order; the item
    /// or container entry takes the pour mark, a zero entry nothing.
    #[test]
    fn the_script_reach_and_marks() {
        let point = Vec3::ZERO;
        let objects = [
            ScriptObject {
                at: Vec3::new(1.0, 0.0, 0.0),
                object: 7,
            },
            ScriptObject {
                at: Vec3::new(3.0, 0.0, 0.0),
                object: 8,
            },
            ScriptObject {
                at: Vec3::new(0.0, 0.0, 0.0),
                object: 0,
            },
        ];
        assert_eq!(nearby_scripts(point, objects), vec![7]);
        assert_eq!(mark_used(MarkTarget::Item, 1), Some(POUR_MARK | 1));
        assert_eq!(
            mark_used(MarkTarget::Container { entry: 2 }, 0),
            Some(POUR_MARK | 2)
        );
        assert_eq!(mark_used(MarkTarget::Container { entry: 0 }, 0), None);
    }

    /// A walk-to point stops 245 cm short, or 350 cm for a door, along the
    /// ground; a near point needs no walk.
    #[test]
    fn the_walk_stop() {
        let feet = Vec3::ZERO;
        let far = Vec3::new(0.0, 0.0, 5.0);
        let goal = walk_stop(feet, far, false).unwrap();
        assert!((goal.z - (5.0 - 2.45)).abs() < 1e-5, "{goal}");
        let goal = walk_stop(feet, far, true).unwrap();
        assert!((goal.z - (5.0 - 3.5)).abs() < 1e-5, "{goal}");
        assert_eq!(walk_stop(feet, Vec3::new(0.0, 0.0, 2.0), false), None);
    }
}

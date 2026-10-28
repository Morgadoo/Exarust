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
//! On a placed object with no usable section it shows the item. A usable
//! section of kind 2 is a lock: open, it opens its container; closed, with
//! the key and its reward still due, its first opening writes a journal
//! line and pays 100 experience, or 1000 when the lock's subtype is 3.
//! Kinds 0 and 1 open the container; other kinds show the item. A dead
//! character runs its roles' interact scripts; the player's own body shows
//! the health and focus bars for 5000 ms and then runs the click callback,
//! as a living character does (Exarust has no dialogue). The double-click
//! interaction is separate (in combat mode it needs Ctrl held): while the
//! object is unmarked a usable item is taken in hand (kind 1, which ends
//! it) and a consumable (kind 0 subtype 1) is used after 2100 ms; a button
//! (kind 5, its flag clear) is pushed; then a container section opens. The
//! player opens the inventory, a corpse is looted, and a living character
//! talks when its roles' condition passes or a dialogue role links it to
//! the player.
//!
//! **Using an item.** The operative section's kind byte names the use: 8
//! is a salve that adds 0.2 to the heal pool (0.1 when the value word is
//! 50) when the body takes remedies, 14 is an elixir that sets the potion
//! flag and stamps the world clock's seconds plus 1800, 9 cleans and 10
//! repairs. Both pour kinds also mark the item (or its container's entry)
//! with 0xC80000, a repair with 0xFF0000. A cleaning kit takes an item's dirt byte down to 0x20
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
/// It writes a value only while it lowers the item's byte or arrives.
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
    /// at section `section` (0 the usable section, 1 the weapon section),
    /// or `None` when the item has no such section. The value is written
    /// only while it lowers that byte or arrives, as the original writes
    /// it. When the item has the usable section and its byte is not
    /// written, the weapon section is not looked at this frame (the
    /// original goes straight to the next entry). An arriving entry is
    /// removed by moving the last into its slot. Returns the section writes
    /// in the loop's order.
    pub fn tick_sections(
        &mut self,
        now_ms: u32,
        mut current: impl FnMut(u32, u8) -> Option<u8>,
    ) -> Vec<(u32, u8, u8)> {
        let mut writes = Vec::with_capacity(self.entries.len() * 2);
        let mut i = 0;
        while i < self.entries.len() {
            let entry = self.entries[i];
            let (value, done) = entry.timed.value(now_ms);
            let mut next_section = true;
            if let Some(old) = current(entry.item, 0) {
                if old > value || done {
                    writes.push((entry.item, 0, value));
                } else {
                    next_section = false;
                }
            }
            if next_section
                && let Some(old) = current(entry.item, 1)
                && (old > value || done)
            {
                writes.push((entry.item, 1, value));
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
/// A lock's unlock reward: 100, or 1000 when the lock section's subtype is
/// [`MAIN_LOCK_SUBTYPE`].
pub const UNLOCK_REWARD: u32 = 100;
pub const UNLOCK_REWARD_MAIN: u32 = 1000;
pub const MAIN_LOCK_SUBTYPE: u8 = 3;
/// The usable section's kinds the click and the interaction read: a lock
/// and a button (kinds 0 and 1 are the consumable and the hand-held item).
pub const USABLE_LOCK: u8 = 2;
pub const USABLE_BUTTON: u8 = 5;
/// A click on the player's own body shows the health and focus bars for
/// this many ms.
pub const SHOW_BARS_MS: u32 = 5000;
/// A consumable of usable kind 0, subtype 1, is used this many ms after
/// the interaction.
pub const CONSUME_MS: u32 = 2100;

/// What the click (and the double-click interaction) sees under the
/// cursor: a placed object's sections, or a character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickTarget {
    /// Nothing was hit.
    Nothing,
    /// A placed object. `usable` is its usable section's kind byte when it
    /// has one and `subtype` that section's subtype byte; `lock_open` and
    /// `reward_ready` are the lock section's two words, `key_ok` the
    /// player's key test, `container` whether it has a container section,
    /// `unmarked` whether its used-mark byte (the one a pour or a repair
    /// sets) is still clear, and `push_released` whether a button's own
    /// flag word allows it to fire.
    Placed {
        usable: Option<u8>,
        subtype: u8,
        lock_open: bool,
        reward_ready: bool,
        key_ok: bool,
        container: bool,
        unmarked: bool,
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
            unmarked: true,
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
    /// The player's own body: the health and focus bars are shown for
    /// `ms`, then the click callback runs.
    ShowBars {
        ms: u32,
    },
    /// A dead character's roles' interact scripts.
    CorpseScripts,
}

/// The click's outcome. On a placed object: no usable section shows the
/// item; a lock (kind 2) that is open opens its container, a closed one
/// with the key and its reward still due pays 100 (1000 for the lock
/// subtype 3), else nothing; kinds 0 and 1 open the container; any other
/// kind shows the item. The object's first-touch bit is cleared by any
/// hit.
pub fn click(target: ClickTarget) -> ClickOutcome {
    match target {
        ClickTarget::Nothing => ClickOutcome::Nothing,
        ClickTarget::Player => ClickOutcome::ShowBars { ms: SHOW_BARS_MS },
        ClickTarget::DeadCharacter => ClickOutcome::CorpseScripts,
        ClickTarget::LivingCharacter => ClickOutcome::Callback,
        ClickTarget::Placed {
            usable,
            subtype,
            lock_open,
            reward_ready,
            key_ok,
            ..
        } => match usable {
            None => ClickOutcome::Inspect,
            Some(USABLE_LOCK) => {
                if lock_open {
                    ClickOutcome::OpenContainer
                } else if key_ok && reward_ready {
                    ClickOutcome::Unlock {
                        reward: if subtype == MAIN_LOCK_SUBTYPE {
                            UNLOCK_REWARD_MAIN
                        } else {
                            UNLOCK_REWARD
                        },
                    }
                } else {
                    ClickOutcome::Nothing
                }
            }
            Some(kind) if kind < USABLE_LOCK => ClickOutcome::OpenContainer,
            Some(_) => ClickOutcome::Inspect,
        },
    }
}

/// What the double-click interaction does with a placed object's usable
/// section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectInteraction {
    Nothing,
    /// Use kind 0, subtype 1: consumed after [`CONSUME_MS`].
    Consume {
        due_ms: u32,
    },
    /// Use kind 0, another subtype: the item is taken at once (guess: the
    /// object database call it makes is not traced further).
    Open,
    /// Take the usable item in hand (kind 1); the interaction ends here.
    UseInHand,
    /// Push a button (kind 5, its flag clear).
    Push,
}

/// The placed object's interaction, as the double click's handler decides
/// it, and whether its container then opens. Kinds 0 and 1 act only while
/// the object is unmarked; a button fires while its flag allows. Only the
/// hand-held item (kind 1) ends the interaction; every other branch,
/// including none, goes on to open the container section when there is
/// one.
pub fn interact_object(target: ClickTarget, now_ms: u32) -> (ObjectInteraction, bool) {
    let ClickTarget::Placed {
        usable,
        subtype,
        container,
        unmarked,
        push_released,
        ..
    } = target
    else {
        return (ObjectInteraction::Nothing, false);
    };
    let action = match usable {
        Some(0) if unmarked => {
            if subtype == 1 {
                ObjectInteraction::Consume {
                    due_ms: now_ms.wrapping_add(CONSUME_MS),
                }
            } else {
                ObjectInteraction::Open
            }
        }
        Some(1) if unmarked => return (ObjectInteraction::UseInHand, false),
        Some(USABLE_BUTTON) if push_released => ObjectInteraction::Push,
        _ => ObjectInteraction::Nothing,
    };
    (action, container)
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

/// A second press of the same button within the system's double-click
/// time of the first, under 2 pixels from it, is a double click. The
/// original asks Windows for that time at start-up (`GetDoubleClickTime`);
/// Exarust uses Windows' default, 500 ms.
pub const DOUBLE_CLICK_MS: u64 = 500;
pub const DOUBLE_CLICK_PX: i64 = 2;

/// Whether a press at `at`, `now_ms`, is the double click of the press at
/// `was`, `was_ms`: strictly under [`DOUBLE_CLICK_MS`] and strictly under 2
/// pixels on both axes.
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
/// container entry (0xC80000 for both pour kinds, 0xFF0000 for a repair),
/// and its sound.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Use {
    pub outcome: UseOutcome,
    pub marks: u32,
    pub sound: Option<UseSound>,
}

/// The pour kinds' mark on the item or its container entry.
pub const POUR_MARK: u32 = 0xC80000;
/// The repair kit's mark on the item or its container entry.
pub const REPAIR_MARK: u32 = 0xFF0000;
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
        // The unit draw is a double; the sum is taken in double and stored
        // as a float.
        pitch: (f64::from(draw) * 0.2 + 0.9) as f32,
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
        POUR_MARKS => {
            let lit = elixir(takes_remedies, world_clock_s);
            Use {
                outcome: lit.map_or(UseOutcome::Nothing, |(flag, until_s)| UseOutcome::Elixir {
                    flag,
                    until_s,
                }),
                marks: POUR_MARK,
                // The pour sound plays only with the potion, at the body.
                sound: lit.map(|_| use_sound(UseSoundKind::Pour, sound_draw, at_point)),
            }
        }
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
            // A repair marks the item (or its entry) 0xFF0000 and plays at
            // the player, wherever the item is.
            Use {
                outcome: timed.map_or(UseOutcome::Nothing, UseOutcome::Timed),
                marks: if timed.is_some() { REPAIR_MARK } else { 0 },
                sound: timed
                    .is_some()
                    .then(|| use_sound(UseSoundKind::Repair, sound_draw, false)),
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
    /// The node's place (cm).
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

/// Where a use's marks go: the world item itself, or a container's entry.
/// `slot` counts from one (zero is no entry) and `word` is the entry's own
/// word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkTarget {
    Item,
    Container { slot: u32, word: u32 },
}

/// The word a use marks: the item's flags word, or the container entry's
/// word, ORed with `mark` ([`POUR_MARK`] or [`REPAIR_MARK`]). A container
/// with no entry slot marks nothing.
pub fn mark_used(target: MarkTarget, flags: u32, mark: u32) -> Option<u32> {
    match target {
        MarkTarget::Item => Some(flags | mark),
        MarkTarget::Container { slot: 0, .. } => None,
        MarkTarget::Container { word, .. } => Some(word | mark),
    }
}

/// What the right press on the picked object asks for (the use cursor): a
/// usable section whose kind byte is zero opens its container; any other
/// kind starts the use; an object with no usable section is left to the
/// GUI's own release callback. No body under the node starts nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsePress {
    Ignore,
    /// A usable kind of zero: the container opens.
    Open,
    /// A usable kind above zero: the use waits for the release.
    Begin,
}

pub fn use_press(has_body: bool, usable: Option<u8>) -> UsePress {
    if !has_body {
        return UsePress::Ignore;
    }
    match usable {
        Some(0) => UsePress::Open,
        Some(_) => UsePress::Begin,
        None => UsePress::Ignore,
    }
}

/// What a use aimed at the world hits, from the pick's classification: a
/// hinge (4), a living character (0x10), a usable item (bit 2 set, so class
/// 3) or nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseTarget {
    /// A door: a matching key unlocks it, another rattles it.
    Door,
    /// A character: the held liquid is poured on it.
    Character,
    /// A world item: the clean/repair dispatch.
    Item,
    Nothing,
}

/// The use's target for a world classification, tested in the original's
/// order (4, then 0x10, then bit 2).
pub fn use_target(classification: u8) -> UseTarget {
    if classification == 4 {
        UseTarget::Door
    } else if classification == 0x10 {
        UseTarget::Character
    } else if classification & 2 != 0 {
        UseTarget::Item
    } else {
        UseTarget::Nothing
    }
}

/// What the use cursor's release is over: the world view (with the
/// pick's target), a GUI view showing a character, an item container
/// window, or another GUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseOver {
    World(UseTarget),
    CharacterView,
    /// An item container window; `same_item` when its entry under the
    /// cursor is the item the use holds.
    ItemWindow {
        same_item: bool,
    },
    OtherGui,
}

/// The picked door's lock, read from its key section: none, open, or
/// locked with a key that does or does not match the held item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoorLock {
    None,
    Open,
    Locked { key_matches: bool },
}

/// What a use's release does, as the dispatcher branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseRelease {
    UnlockDoor,
    RattleDoor,
    Pour,
    CleanWorld,
    RepairWorld,
    CleanGui,
    RepairGui,
    TransferGui,
    Nothing,
}

/// The release of a use, in the original's order. Over the world view a
/// door (4) unlocks when it is locked and the held item is its key and
/// rattles when locked with another; a character (0x10) takes a liquid
/// (kinds 8 and 14); a world item (bit 2) with an item id is cleaned (9)
/// or repaired (10). Over the GUI a character view takes the liquid, and
/// an item window is the GUI use ([`gui_use`]). `held_kind` is the held
/// item's usable kind (0 when it has none).
pub fn use_release(over: UseOver, held_kind: u8, door: DoorLock, item_id: u32) -> UseRelease {
    let pours = held_kind == POUR || held_kind == POUR_MARKS;
    match over {
        UseOver::World(UseTarget::Door) => match door {
            DoorLock::Locked { key_matches: true } => UseRelease::UnlockDoor,
            DoorLock::Locked { key_matches: false } => UseRelease::RattleDoor,
            DoorLock::None | DoorLock::Open => UseRelease::Nothing,
        },
        UseOver::World(UseTarget::Character) | UseOver::CharacterView if pours => UseRelease::Pour,
        UseOver::World(UseTarget::Item) if item_id != 0 => match held_kind {
            CLEAN => UseRelease::CleanWorld,
            REPAIR => UseRelease::RepairWorld,
            _ => UseRelease::Nothing,
        },
        UseOver::ItemWindow { same_item } => match gui_use(held_kind, same_item) {
            GuiUse::Clean => UseRelease::CleanGui,
            GuiUse::Repair => UseRelease::RepairGui,
            GuiUse::Transfer => UseRelease::TransferGui,
            GuiUse::Nothing => UseRelease::Nothing,
        },
        _ => UseRelease::Nothing,
    }
}

/// The held item's use on a GUI item slot (the cursor over an inventory
/// entry): a cleaning kit (kind 9) and a repair kit (10) work on the entry,
/// and a kind 0x10 moves the entry when it is the same item the use holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuiUse {
    Clean,
    Repair,
    Transfer,
    Nothing,
}

pub fn gui_use(kind: u8, same_item: bool) -> GuiUse {
    match kind {
        9 => GuiUse::Clean,
        10 => GuiUse::Repair,
        0x10 if same_item => GuiUse::Transfer,
        _ => GuiUse::Nothing,
    }
}

/// Where the used item came from, so the use can remove it: the scene (a
/// world body) or a container entry with a slot (0 for no entry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseHome {
    Scene,
    Container { slot: u32 },
}

/// What a finished use does with the item it used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseAfter {
    /// The world body is taken out of the scene.
    RemoveBody,
    /// The container entry `slot` (from one) is taken out.
    RemoveSlot(u32),
    Nothing,
}

pub fn after_use(home: Option<UseHome>) -> UseAfter {
    match home {
        None => UseAfter::Nothing,
        Some(UseHome::Scene) => UseAfter::RemoveBody,
        Some(UseHome::Container { slot: 0 }) => UseAfter::Nothing,
        Some(UseHome::Container { slot }) => UseAfter::RemoveSlot(slot - 1),
    }
}

/// A sound played at a point: the original's volume 0.25, pitch 1.0.
#[derive(Debug, Clone, PartialEq)]
pub struct PointSound {
    pub name: String,
    pub at: Vec3,
    pub volume: f32,
    pub pitch: f32,
}

/// A point sound for `name` at `at`, or `None` for an unnamed sound.
pub fn point_sound(name: &str, at: Vec3) -> Option<PointSound> {
    (!name.is_empty()).then(|| PointSound {
        name: name.to_string(),
        at,
        volume: 0.25,
        pitch: 1.0,
    })
}

/// A door's drag link word bits: the stop bit a suppressed request sets,
/// which the release clears, and the wake span.
pub const DOOR_LINK_STOP: u32 = 0x100000;
pub const DOOR_WAKE_MS: u32 = 2000;

/// The link word after a release clears the stop bit.
pub fn door_link_resumed(flags: u32) -> u32 {
    flags & !DOOR_LINK_STOP
}

/// The link word while the movement request is suppressed: the stop bit
/// is set.
pub fn door_link_stopped(flags: u32) -> u32 {
    flags | DOOR_LINK_STOP
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
    /// A door drag sets its cursor and leaves the request to the keys and
    /// the right button.
    Mechanism,
    /// A refused press keeps the request as it was.
    Refused,
    /// An item use's reach only.
    Use,
}

/// The movement request's mode, in the original's branch order (after the
/// down-or-dead test, which the caller makes): an item use, the lift, a
/// door (whose drag then runs the normal request), a refusal, combat mode,
/// and the normal request. A wheel, cloth, physical or GUI drag is not
/// tested here and takes the combat or normal branch.
pub fn request_mode(state: crate::hand::DragState, combat: bool) -> RequestMode {
    use crate::hand::DragState;
    match state {
        DragState::Use => RequestMode::Use,
        DragState::Lift => RequestMode::Lift,
        DragState::Door => RequestMode::Mechanism,
        DragState::Refused => RequestMode::Refused,
        _ if combat => RequestMode::Combat,
        _ => RequestMode::Normal,
    }
}

/// The controls' gates as the frame reads them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ControlGates {
    /// The controls are off.
    pub off: bool,
    /// A screen blocks the controls.
    pub blocked: bool,
    /// A second block.
    pub second_block: bool,
    /// The spectating view.
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

/// A game mode the controls unit registers: the story's Character mode and
/// the Spectate view. The original appends them to the game's mode table in
/// this order, each with a start and an end routine, and keeps each mode's
/// id (the table's length after the append, plus two) for the session to
/// enter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Character,
    Spectate,
}

/// One mode-table entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameModeEntry {
    pub name: &'static str,
    pub mode: GameMode,
    /// Its place in the table, from zero.
    pub index: u32,
    /// The id the session asks for: the table's length after the append
    /// plus two, so the place plus three.
    pub id: u32,
}

impl GameMode {
    /// The name the original stores in the mode table.
    pub fn name(self) -> &'static str {
        match self {
            GameMode::Character => "Character",
            GameMode::Spectate => "Spectate",
        }
    }
}

/// The two mode entries the unit init appends to a table of `registered`
/// entries, in order.
pub fn register_game_modes(registered: u32) -> [GameModeEntry; 2] {
    [GameMode::Character, GameMode::Spectate].map(|mode| {
        let index = registered + mode as u32;
        GameModeEntry {
            name: mode.name(),
            mode,
            index,
            // The table's new length (place + 1) plus two.
            id: index + 3,
        }
    })
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
        unmarked: bool,
        push_released: bool,
    ) -> ClickTarget {
        ClickTarget::Placed {
            usable,
            subtype,
            lock_open,
            reward_ready,
            key_ok,
            container,
            unmarked,
            push_released,
        }
    }

    fn placed(usable: Option<u8>, subtype: u8) -> ClickTarget {
        placed_with(usable, subtype, false, false, false, false, true, true)
    }

    /// No usable section inspects, container or not; a closed lock needs
    /// the key and its reward due and pays 100, or 1000 for the lock
    /// subtype 3; an open lock opens the container; kinds 0 and 1 always
    /// open the container; other kinds (3 included) inspect. The player's
    /// body shows the bars for 5000 ms.
    #[test]
    fn a_click_names_the_object_and_the_character() {
        assert_eq!(click(ClickTarget::Nothing), ClickOutcome::Nothing);
        assert_eq!(
            click(ClickTarget::Player),
            ClickOutcome::ShowBars { ms: 5_000 }
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
        let main = placed_with(Some(USABLE_LOCK), 3, false, true, true, false, true, true);
        assert_eq!(click(main), ClickOutcome::Unlock { reward: 1000 });
        let spent = placed_with(Some(USABLE_LOCK), 3, false, false, true, false, true, true);
        assert_eq!(click(spent), ClickOutcome::Nothing);
        let open = placed_with(Some(USABLE_LOCK), 0, true, false, false, false, true, true);
        assert_eq!(click(open), ClickOutcome::OpenContainer);
        assert_eq!(click(placed(Some(0), 0)), ClickOutcome::OpenContainer);
        assert_eq!(click(placed(Some(1), 0)), ClickOutcome::OpenContainer);
        let kind3 = placed_with(Some(3), 0, false, true, true, true, true, true);
        assert_eq!(click(kind3), ClickOutcome::Inspect);
        assert_eq!(click(placed(Some(USABLE_BUTTON), 0)), ClickOutcome::Inspect);
        assert_eq!(click(ClickTarget::plain()), ClickOutcome::Inspect);
        let boxed = placed_with(None, 0, false, false, false, true, true, true);
        assert_eq!(click(boxed), ClickOutcome::Inspect);
    }

    /// Kind 0 subtype 1 consumes after 2100 ms and another subtype is
    /// taken, only while unmarked; kind 1 takes the item and ends the
    /// interaction; kind 5 pushes while its flag allows; every other branch
    /// goes on to the container section.
    #[test]
    fn the_interaction_reads_the_usable_section() {
        let n = ObjectInteraction::Nothing;
        assert_eq!(
            interact_object(placed(Some(0), 1), 1_000),
            (ObjectInteraction::Consume { due_ms: 3_100 }, false)
        );
        let full = placed_with(Some(0), 1, false, false, false, true, true, true);
        assert_eq!(
            interact_object(full, 1_000),
            (ObjectInteraction::Consume { due_ms: 3_100 }, true)
        );
        assert_eq!(
            interact_object(placed(Some(0), 0), 0),
            (ObjectInteraction::Open, false)
        );
        let marked = placed_with(Some(0), 1, false, false, false, true, false, true);
        assert_eq!(interact_object(marked, 0), (n, true));
        let hand = placed_with(Some(1), 0, false, false, false, true, true, true);
        assert_eq!(
            interact_object(hand, 0),
            (ObjectInteraction::UseInHand, false)
        );
        let marked = placed_with(Some(1), 0, false, false, false, true, false, true);
        assert_eq!(interact_object(marked, 0), (n, true));
        assert_eq!(
            interact_object(placed(Some(USABLE_BUTTON), 0), 0),
            (ObjectInteraction::Push, false)
        );
        let held = placed_with(
            Some(USABLE_BUTTON),
            0,
            false,
            false,
            false,
            false,
            true,
            false,
        );
        assert_eq!(interact_object(held, 0), (n, false));
        let lock = placed_with(Some(2), 0, false, false, false, true, true, true);
        assert_eq!(interact_object(lock, 0), (n, true));
        let lone = placed_with(None, 0, false, false, false, false, true, true);
        assert_eq!(interact_object(lone, 0), (n, false));
        assert_eq!(interact_object(ClickTarget::Player, 0), (n, false));
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

    /// A double click is strictly under 500 ms and under 2 pixels on both
    /// axes.
    #[test]
    fn the_double_click_window() {
        assert!(is_double_click((10, 10), 1000, (11, 9), 1499));
        assert!(!is_double_click((10, 10), 1000, (11, 9), 1500));
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
        // Without the remedy flag the elixir only marks: no potion, no sound.
        let dull = use_on(14, 7, false, 100, None, None, 0, 0, 0.5, true);
        assert_eq!(dull.outcome, UseOutcome::Nothing);
        assert_eq!(dull.marks, POUR_MARK);
        assert_eq!(dull.sound, None);
        // The pitch sum is taken in double: 0.9 + 0.2·0.3 rounds once.
        let p = use_sound(UseSoundKind::Pour, 0.3, false).pitch;
        assert_eq!(p, (f64::from(0.3f32) * 0.2 + 0.9) as f32);
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
        assert_eq!(repair.marks, REPAIR_MARK);
        assert!(!repair.sound.unwrap().at_point);
        assert_eq!(clean.marks, 0);
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
        assert!(list.tick_sections(1000, |_, _| Some(100)).is_empty());
        // Halfway: both sections go to 50.
        assert_eq!(
            list.tick_sections(2000, |_, _| Some(100)),
            vec![(7, 0, 50), (7, 1, 50)]
        );
        // The weapon section alone lower: it is written.
        assert_eq!(
            list.tick_sections(2500, |_, section| if section == 0 {
                Some(50)
            } else {
                Some(20)
            }),
            vec![(7, 0, 25)]
        );
        // The usable section not lowered: the weapon section is skipped
        // this frame even though it would lower.
        assert!(
            list.tick_sections(2600, |_, section| if section == 0 {
                Some(10)
            } else {
                Some(90)
            })
            .is_empty()
        );
        // No usable section: the weapon section is tested on its own.
        assert_eq!(
            list.tick_sections(2600, |_, section| (section == 1).then_some(90)),
            vec![(7, 1, 20)]
        );
        // Done writes both even when one is already at the target.
        assert_eq!(
            list.tick_sections(3000, |_, _| Some(0)),
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
        assert_eq!(list.tick_sections(900, |_, _| Some(0x30)), Vec::new());
    }

    /// The movement request's mode branch order.
    #[test]
    fn the_request_mode() {
        use crate::hand::DragState;
        assert_eq!(request_mode(DragState::Idle, false), RequestMode::Normal);
        assert_eq!(request_mode(DragState::Idle, true), RequestMode::Combat);
        assert_eq!(request_mode(DragState::Physical, true), RequestMode::Combat);
        assert_eq!(request_mode(DragState::Wheel, true), RequestMode::Combat);
        assert_eq!(request_mode(DragState::Door, true), RequestMode::Mechanism);
        assert_eq!(
            request_mode(DragState::Physical, false),
            RequestMode::Normal
        );
        assert_eq!(request_mode(DragState::Use, false), RequestMode::Use);
        assert_eq!(request_mode(DragState::Lift, false), RequestMode::Lift);
        assert_eq!(
            request_mode(DragState::GuiPending, false),
            RequestMode::Normal
        );
        assert_eq!(request_mode(DragState::Door, false), RequestMode::Mechanism);
        assert_eq!(request_mode(DragState::Wheel, false), RequestMode::Normal);
        assert_eq!(request_mode(DragState::Cloth, false), RequestMode::Normal);
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
        assert_eq!(
            mark_used(MarkTarget::Item, 1, POUR_MARK),
            Some(POUR_MARK | 1)
        );
        assert_eq!(
            mark_used(MarkTarget::Container { slot: 1, word: 2 }, 0, REPAIR_MARK),
            Some(REPAIR_MARK | 2)
        );
        assert_eq!(
            mark_used(MarkTarget::Container { slot: 0, word: 2 }, 0, POUR_MARK),
            None
        );
    }

    /// The use release: a locked door unlocks with its key and rattles
    /// with another, an unlocked or keyless door does nothing; a liquid
    /// pours on a character in the world or a character view; a kit
    /// cleans or repairs a world item with an id; an item window is the GUI
    /// use.
    #[test]
    fn the_use_release_branch() {
        let world = UseOver::World;
        let locked = |key_matches| DoorLock::Locked { key_matches };
        assert_eq!(
            use_release(world(UseTarget::Door), 0, locked(true), 0),
            UseRelease::UnlockDoor
        );
        assert_eq!(
            use_release(world(UseTarget::Door), 0, locked(false), 0),
            UseRelease::RattleDoor
        );
        assert_eq!(
            use_release(world(UseTarget::Door), 0, DoorLock::Open, 0),
            UseRelease::Nothing
        );
        assert_eq!(
            use_release(world(UseTarget::Door), 0, DoorLock::None, 0),
            UseRelease::Nothing
        );
        for over in [world(UseTarget::Character), UseOver::CharacterView] {
            assert_eq!(use_release(over, POUR, DoorLock::None, 0), UseRelease::Pour);
            assert_eq!(
                use_release(over, POUR_MARKS, DoorLock::None, 0),
                UseRelease::Pour
            );
            assert_eq!(
                use_release(over, CLEAN, DoorLock::None, 0),
                UseRelease::Nothing
            );
        }
        assert_eq!(
            use_release(world(UseTarget::Item), CLEAN, DoorLock::None, 5),
            UseRelease::CleanWorld
        );
        assert_eq!(
            use_release(world(UseTarget::Item), REPAIR, DoorLock::None, 5),
            UseRelease::RepairWorld
        );
        assert_eq!(
            use_release(world(UseTarget::Item), REPAIR, DoorLock::None, 0),
            UseRelease::Nothing
        );
        assert_eq!(
            use_release(world(UseTarget::Item), POUR, DoorLock::None, 5),
            UseRelease::Nothing
        );
        let window = |same_item| UseOver::ItemWindow { same_item };
        assert_eq!(
            use_release(window(false), CLEAN, DoorLock::None, 0),
            UseRelease::CleanGui
        );
        assert_eq!(
            use_release(window(false), REPAIR, DoorLock::None, 0),
            UseRelease::RepairGui
        );
        assert_eq!(
            use_release(window(true), 0x10, DoorLock::None, 0),
            UseRelease::TransferGui
        );
        assert_eq!(
            use_release(window(false), 0x10, DoorLock::None, 0),
            UseRelease::Nothing
        );
        assert_eq!(
            use_release(UseOver::OtherGui, CLEAN, DoorLock::None, 0),
            UseRelease::Nothing
        );
        assert_eq!(
            use_release(world(UseTarget::Nothing), CLEAN, DoorLock::None, 0),
            UseRelease::Nothing
        );
    }

    /// The unit init appends Character then Spectate, each id the table's
    /// new length plus two.
    #[test]
    fn the_game_modes_register_in_order() {
        let modes = register_game_modes(0);
        assert_eq!(modes[0].name, "Character");
        assert_eq!(modes[0].mode, GameMode::Character);
        assert_eq!((modes[0].index, modes[0].id), (0, 3));
        assert_eq!(modes[1].name, "Spectate");
        assert_eq!(modes[1].mode, GameMode::Spectate);
        assert_eq!((modes[1].index, modes[1].id), (1, 4));
        let modes = register_game_modes(7);
        assert_eq!((modes[0].index, modes[0].id), (7, 10));
        assert_eq!((modes[1].index, modes[1].id), (8, 11));
    }

    /// The right press: a usable kind zero opens the container, another
    /// kind waits for the release, and no body or no usable section asks
    /// for nothing.
    #[test]
    fn the_use_press_and_its_target() {
        assert_eq!(use_press(false, Some(9)), UsePress::Ignore);
        assert_eq!(use_press(true, None), UsePress::Ignore);
        assert_eq!(use_press(true, Some(0)), UsePress::Open);
        assert_eq!(use_press(true, Some(9)), UsePress::Begin);
        assert_eq!(use_press(true, Some(14)), UsePress::Begin);
        assert_eq!(use_target(4), UseTarget::Door);
        assert_eq!(use_target(0x10), UseTarget::Character);
        assert_eq!(use_target(3), UseTarget::Item);
        assert_eq!(use_target(1), UseTarget::Nothing);
        assert_eq!(use_target(0x40), UseTarget::Nothing);
        assert_eq!(use_target(0), UseTarget::Nothing);
    }

    /// A GUI use: a cleaning or repair kit works on the entry; a kind 0x10
    /// moves it only when it is the same item.
    #[test]
    fn the_gui_use_and_the_used_item() {
        assert_eq!(gui_use(9, false), GuiUse::Clean);
        assert_eq!(gui_use(10, false), GuiUse::Repair);
        assert_eq!(gui_use(0x10, true), GuiUse::Transfer);
        assert_eq!(gui_use(0x10, false), GuiUse::Nothing);
        assert_eq!(gui_use(8, true), GuiUse::Nothing);
        // A used world body is removed; a container entry one less than its
        // slot; no home and slot zero do nothing.
        assert_eq!(after_use(None), UseAfter::Nothing);
        assert_eq!(after_use(Some(UseHome::Scene)), UseAfter::RemoveBody);
        assert_eq!(
            after_use(Some(UseHome::Container { slot: 3 })),
            UseAfter::RemoveSlot(2)
        );
        assert_eq!(
            after_use(Some(UseHome::Container { slot: 0 })),
            UseAfter::Nothing
        );
        // A point sound is at 0.25 and pitch 1.0; an unnamed one is silent.
        let s = point_sound("CleanItem01", Vec3::new(1.0, 2.0, 3.0)).unwrap();
        assert_eq!(s.volume, 0.25);
        assert_eq!(s.pitch, 1.0);
        assert_eq!(s.at, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(point_sound("", Vec3::ZERO), None);
        // The door link's stop bit.
        assert_eq!(door_link_stopped(0x4000), 0x4000 | DOOR_LINK_STOP);
        assert_eq!(door_link_resumed(0x4000 | DOOR_LINK_STOP), 0x4000);
        assert_eq!(DOOR_WAKE_MS, 2000);
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

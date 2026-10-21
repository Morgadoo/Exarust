//! The player controls' rules that are not the hand's drags nor the hotkey
//! dispatch: what a click on the world names and does (a door's key and
//! reward, a container, a corpse's scripts, a person to talk to), the
//! usable item a hand carries and puts on the world (a pour, a cleaning or
//! a repair kit with its timed list), the reach against the navigation
//! region, the start and end of play, and the walk-to-cursor latch.
//! Rules: `docs/PLAYER.md` (Using what the cursor is over, The reach).
//!
//! **A click's name.** A hinge (a door or a lever) is opened, or unlocked
//! with the player's key: the first unlock writes a journal entry and
//! awards 100 or 1000 experience. A fitting of kind 2 is a lock, a
//! container opens, a dead character runs its roles' interact scripts, the
//! player opens its character screen and a living person is talked to. A
//! double click does the same as Ctrl's interaction.
//!
//! **Using an item.** A carried item with a usable section kind 8 or 14
//! pours: kind 8 warms what it is poured on by 0.1 or 0.2 (a 50-typed
//! liquid the larger), kind 14 lights a fire, sets its flag and a 1800 ms
//! timer. A cleaning kit (kind 9) takes the dirt of the item it is used on
//! down to 0.032 (the byte stops over 0x20), a repair kit (kind 10) raises
//! the condition by a tenth of its material's rounded-up rating a fifth of
//! a second. Both schedule a timed value that moves at 0.0005 of the gap a
//! millisecond (two seconds) and is applied to the item as it goes; a list
//! entry is dropped when it arrives.
//!
//! **The reach.** A point within 245 cm of the world's floor and 300 cm of
//! the hip across the ground is in reach; a door's own place is measured to
//! 300 cm high and 350 cm across, a wheel's with the plain limits. When the
//! player's navigation region has a height field, the region's own test
//! must pass too. A comparison with a NaN passes.
//!
//! Evidence: the interact handler, the use dispatcher, the timed list, the
//! reach functions and the start/end of play, read in the instructions
//! (`context/re/player-control-hand.md`).

use bevy::math::Vec3;
use bevy::prelude::Resource;

use crate::pick::Pick;

/// A point is used from at most this height (cm, the world's own height).
pub const REACH_HEIGHT_CM: f32 = 245.0;
/// A door's reach: 300 cm high and 350 cm across; a wheel's is the plain
/// 300 cm across.
pub const DOOR_HEIGHT_CM: f32 = 300.0;
pub const DOOR_ACROSS_CM: f32 = 350.0;
pub const REACH_ACROSS_CM: f32 = 300.0;
/// A point farther than this (cm) turns the player toward it first.
pub const TURN_BEYOND_CM: f32 = 70.0;
/// A door unlock's reward (experience), by the usable section's kind: the
/// 3-kind door gives 1000, the others 100.
pub const UNLOCK_REWARD: u32 = 100;
pub const UNLOCK_REWARD_MAIN: u32 = 1000;
/// The usable section's kinds: a lock, a door and the others.
pub const USABLE_LOCK: u8 = 2;
pub const USABLE_MAIN: u8 = 3;

/// What a click on the world does, after its pick is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interact {
    /// Nothing: no hit, a level surface or a refused object.
    Nothing,
    /// Open a container.
    OpenContainer,
    /// A hinge: open or close it; `key` asks for the player's key first.
    Hinge {
        locked: bool,
        key_ok: bool,
        /// The reward the first unlock pays (0 when it was opened before).
        reward: u32,
    },
    /// The player's own character screen.
    CharacterScreen,
    /// A corpse's roles' "player interacts" scripts.
    CorpseScripts,
    /// A living character to talk to.
    Talk,
    /// A usable item on a prop with no use of its own: look at it.
    Look,
}

/// The parts of a clicked object the interaction reads.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Click {
    /// The clicked thing, as [`crate::pick::classify`] names it.
    pub named: Pick,
    /// The object's usable section kind, when it has one.
    pub usable: Option<u8>,
    /// A lock's own kind byte: 3 is the main door (1000 experience), the
    /// others 100.
    pub lock_kind: Option<u8>,
    /// The player has the key the lock wants.
    pub key_ok: bool,
    /// The lock has never been opened (the journal has no entry).
    pub first_time: bool,
    /// A container's window can open.
    pub container: bool,
}

impl Default for Click {
    fn default() -> Self {
        Self {
            named: Pick::Nothing,
            usable: None,
            lock_kind: None,
            key_ok: false,
            first_time: false,
            container: false,
        }
    }
}

/// What a click does. A lock opens only with the key; its first opening pays
/// 1000 experience for a door (usable kind 3) and 100 otherwise.
pub fn interact(click: Click) -> Interact {
    match click.named {
        Pick::Nothing | Pick::Loose | Pick::Usable | Pick::Scene | Pick::Wheel | Pick::Crank => {
            // A prop without a use of its own: a usable item is looked at
            // (the original's container window), nothing else happens.
            if click.usable.is_some() {
                Interact::Look
            } else if click.container {
                Interact::OpenContainer
            } else {
                Interact::Nothing
            }
        }
        Pick::Hinge => {
            let locked = click.usable == Some(USABLE_LOCK);
            if locked && !click.key_ok {
                return Interact::Nothing;
            }
            let reward = if locked && click.first_time {
                if click.lock_kind == Some(USABLE_MAIN) {
                    UNLOCK_REWARD_MAIN
                } else {
                    UNLOCK_REWARD
                }
            } else {
                0
            };
            Interact::Hinge {
                locked,
                key_ok: click.key_ok,
                reward,
            }
        }
        Pick::Corpse => Interact::CorpseScripts,
        Pick::Person => Interact::Talk,
    }
}

/// Whether the click is the player's own body: it opens the character screen
/// (5000 experience units of its own, the original's call).
pub fn clicks_self(named: Pick, self_clicked: bool) -> bool {
    self_clicked && named == Pick::Person
}

/// A click shorter than this (ms) after the one before, within 2 pixels, is
/// a double click; it does what Ctrl does.
pub const DOUBLE_CLICK_MS: u64 = 300;
pub const DOUBLE_CLICK_PX: i32 = 2;

/// Whether a press at `at` is the double click of the press before, at
/// `was` and `was_ms`.
pub fn is_double_click(was: (i32, i32), was_ms: u64, at: (i32, i32), now_ms: u64) -> bool {
    now_ms < was_ms + DOUBLE_CLICK_MS
        && (at.0 - was.0).abs() <= DOUBLE_CLICK_PX
        && (at.1 - was.1).abs() <= DOUBLE_CLICK_PX
}

/// The usable item a hand carries, as the use dispatcher reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UseKind {
    /// A liquid (section kind 8): pours and warms.
    Liquid {
        hot: bool,
    },
    /// A fire starter (section kind 14): lights the target.
    Fire,
    /// A cleaning kit (kind 9).
    Clean,
    /// A repair kit (kind 10).
    Repair,
    Other,
}

impl UseKind {
    /// The kind byte of the item's usable section, as the original reads it.
    pub fn of(kind: u8) -> Self {
        match kind {
            8 => Self::Liquid { hot: false },
            14 => Self::Fire,
            9 => Self::Clean,
            10 => Self::Repair,
            _ => Self::Other,
        }
    }
}

/// The warmth a pour adds to the target (`hot`, the 50-typed liquid, adds
/// 0.1, every other 0.2). The original's field is the target's own
/// temperature; Exarust keeps the delta.
pub fn pour_warmth(hot: bool) -> f32 {
    if hot { 0.1 } else { 0.2 }
}

/// What using an item on a target starts.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Use {
    Nothing,
    /// Add `warmth` to the target.
    Warm {
        warmth: f32,
        hot: bool,
    },
    /// Light the target, its flag and a timer of this many ms.
    Ignite {
        timer_ms: u32,
    },
    /// A timed value on the item: its byte `from` toward `to`.
    Timed(Timed),
}

/// The original's ignition timer on a lit target (ms of the game clock).
pub const IGNITE_MS: u32 = 1800;

/// A timed value on an item: its kind (the cleaning byte or the condition
/// byte), the value it starts at and the value it stops at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timed {
    /// 9 for a cleaning, 10 for a repair.
    pub kind: u8,
    pub from: u8,
    pub to: u8,
    /// The game clock (ms) it started.
    pub started_ms: u32,
}

/// The fractions a timed value moves each millisecond (0.0005, in double
/// precision), so a full value takes two seconds.
pub const TIMED_RATE: f64 = 0.0005;

impl Timed {
    /// The value at `now_ms`: from toward to by 0.0005 of the gap a ms,
    /// rounded to the nearest even byte, or the end value once the gap is
    /// spent. Returns the value and whether the entry is done.
    pub fn value(&self, now_ms: u32) -> (u8, bool) {
        let elapsed = f64::from(now_ms.wrapping_sub(self.started_ms));
        let f = (elapsed * TIMED_RATE) as f32;
        if f < 1.0 {
            let from = f32::from(self.from);
            let to = f32::from(self.to);
            (round_ties_even((to - from) * f + from) as u8, false)
        } else {
            (self.to, true)
        }
    }
}

/// The engine's nearest-even integer cast of a float.
fn round_ties_even(v: f32) -> i32 {
    v.round_ties_even() as i32
}

/// The cleaning kit's target value: the dirt byte stops over 0x20, so
/// 0x20 is kept and anything at or under it is not cleaned. `Some(to)` when
/// it cleans.
pub fn clean_to(dirt: u8) -> Option<u8> {
    if dirt > 0x20 { Some(dirt - 0x20) } else { None }
}

/// The repair kit's target condition: the count is 0xFF divided by the
/// material's `(rating + 5) / 10` (at least 1), and the condition drops by
/// that, never under zero.
pub fn repair_to(condition: u8, rating: u8) -> u8 {
    let step = ((u16::from(rating) + 5) / 10).max(1);
    condition.saturating_sub((255 / step) as u8)
}

/// Picks the use for a carried `kind` on the clicked `named` object, whose
/// item is `target` when it has one.
pub fn use_on(kind: UseKind, named: Pick, target: Option<&ItemState>, now_ms: u32) -> Use {
    match kind {
        UseKind::Liquid { hot } => match named {
            Pick::Person | Pick::Corpse => Use::Warm {
                warmth: pour_warmth(hot),
                hot,
            },
            _ => Use::Nothing,
        },
        UseKind::Fire => Use::Ignite {
            timer_ms: IGNITE_MS,
        },
        UseKind::Clean => {
            let Some(item) = target else {
                return Use::Nothing;
            };
            match clean_to(item.dirt) {
                Some(to) => Use::Timed(Timed {
                    kind: 9,
                    from: item.dirt,
                    to,
                    started_ms: now_ms,
                }),
                None => Use::Nothing,
            }
        }
        UseKind::Repair => {
            let Some(item) = target else {
                return Use::Nothing;
            };
            Use::Timed(Timed {
                kind: 10,
                from: item.condition,
                to: repair_to(item.condition, item.rating),
                started_ms: now_ms,
            })
        }
        UseKind::Other => Use::Nothing,
    }
}

/// The target item's dirt and condition bytes, and its material rating.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemState {
    pub dirt: u8,
    pub condition: u8,
    pub rating: u8,
}

/// Where a click's auto-walk stops: `point` is the clicked point and `at`
/// the player's feet (m). A point farther than the walk's stop (245 cm, or
/// 350 cm for a door) leaves a goal that far short of it; a nearer one needs
/// no walk. (The stop distances are the original's 245–350 cm range.)
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

/// The timed list: entries applied to an item each frame, removed when they
/// arrive. The original holds them in an array, takes a finished entry out
/// by moving the last into its place, and counts its entries with a length.
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

    /// One frame at `now_ms`: every entry's value, and the entries that
    /// arrive are removed (the last moved into the slot), as the original's
    /// loop does. Returns the values to write, in the order the loop reads
    /// the shrinking array.
    pub fn tick(&mut self, now_ms: u32) -> Vec<(u32, u8)> {
        let mut writes = Vec::with_capacity(self.entries.len());
        let mut i = 0;
        while i < self.entries.len() {
            let entry = self.entries[i];
            let (value, done) = entry.timed.value(now_ms);
            writes.push((entry.item, value));
            if done {
                self.entries.swap_remove(i);
            } else {
                i += 1;
            }
        }
        writes
    }
}

/// The navigation region's answer for a point: no height field, inside the
/// region, or outside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region {
    None,
    Inside,
    Outside,
}

/// What the reach is measured to, and its own limits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reach {
    /// The point's height in the world (cm), for a plain point.
    pub height_cm: f32,
    /// The offset from the player's hip (cm).
    pub offset_cm: Vec3,
    /// A door's own place (300 high, 350 across), a wheel's (plain).
    pub frame: ReachFrame,
    /// The player's navigation region at the point.
    pub region: Region,
    /// The cursor's picked node is a sim object (a door/wheel) whose own
    /// region test applies; a plain point skips it.
    pub object: bool,
}

/// Whether the point is in reach: its height (or a door's) under the cap,
/// its vertical offset under 300 cm, its ground offset under 300 (350 for a
/// door), and the region's test when the player has a height field. Every
/// ordered comparison fails only when it is ordered against; a NaN passes.
pub fn within_reach(r: Reach) -> bool {
    if r.object {
        match r.region {
            Region::Outside => return false,
            Region::None | Region::Inside => {}
        }
    }
    let (height, across) = match r.frame {
        ReachFrame::Door => (DOOR_HEIGHT_CM, DOOR_ACROSS_CM),
        ReachFrame::Point | ReachFrame::Wheel => (REACH_HEIGHT_CM, REACH_ACROSS_CM),
    };
    if height < r.height_cm {
        return false;
    }
    // The original's ordered compares fail only on an ordered "over"; a
    // NaN passes them (the unordered branch skips the refusal).
    if r.offset_cm.y.abs() > REACH_ACROSS_CM {
        return false;
    }
    let flat2 = r.offset_cm.x * r.offset_cm.x + r.offset_cm.z * r.offset_cm.z;
    if flat2 > across * across {
        return false;
    }
    true
}

/// What a reach measures to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachFrame {
    Point,
    Door,
    Wheel,
}

/// The controls' walk-to-cursor latch: the right button asks for a walk; its
/// release keeps the request until the walk reaches key segment 0 or 4, and
/// the reach is not measured meanwhile. This is the `0x96c` flag's state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct WalkLatch {
    /// A movement request is held (the button or key down, or a release
    /// still finishing its stride).
    pub held: bool,
    /// The game clock (ms) the request began.
    pub since_ms: u32,
}

impl WalkLatch {
    /// One frame: `wanted` is a movement key or the right button; `moving`
    /// the body's last movement decision; `segment` the walk's key segment
    /// (0 or 4 let a released request go). The reach is skipped while the
    /// latch is held.
    pub fn frame(&mut self, wanted: bool, moving: bool, segment: Option<usize>, now_ms: u32) {
        if wanted {
            if !self.held {
                self.held = true;
                self.since_ms = now_ms;
            }
        } else if !moving || segment.is_none_or(|s| s == 0 || s == 4) {
            self.held = false;
        }
    }

    /// The reach is skipped while the latch holds.
    pub fn skips_reach(&self) -> bool {
        self.held
    }
}

/// The activation radius (cm) the game starts a level with: its square is
/// 16 000 000 (4000 cm, 40 m).
pub const START_ACTIVATION_RADIUS2: f32 = 16_000_000.0;
/// The radius (cm) the end of play resets the activation centre to: its
/// square is 400 000 000 (200 m).
pub const END_ACTIVATION_RADIUS2: f32 = 400_000_000.0;
/// The camera's start zoom and range, the preset the start of play creates.
pub const START_ZOOM: f32 = 1300.0;
pub const ZOOM_MIN: f32 = 800.0;
pub const ZOOM_MAX: f32 = 2000.0;
/// The debug setting's zoom and pitch-ish second value when the flag is set.
pub const DEBUG_ZOOM: f32 = 800.0;
pub const DEBUG_SECOND: f32 = 600.0;

/// The session state the start and end of play set: the AI activation
/// centre and radius, and whether the player's controls are in play.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct PlaySession {
    pub activation_centre: Vec3,
    pub activation_radius2: f32,
    /// The player's controls run (the original's playing flag).
    pub playing: bool,
    /// The camera zoom and its range.
    pub zoom: f32,
    /// The debug setting's override of the camera preset.
    pub debug_camera: bool,
}

impl Default for PlaySession {
    fn default() -> Self {
        Self {
            activation_centre: Vec3::ZERO,
            activation_radius2: END_ACTIVATION_RADIUS2,
            playing: false,
            zoom: START_ZOOM,
            debug_camera: false,
        }
    }
}

impl PlaySession {
    /// The start of play at the player's `centre`: the controls run, the AI
    /// activates around the player (a 40 m radius) and the camera starts at
    /// 1300 (800..2000), or at the debug preset.
    pub fn begin(&mut self, centre: Vec3) {
        self.playing = true;
        self.activation_centre = centre;
        self.activation_radius2 = START_ACTIVATION_RADIUS2;
        self.zoom = if self.debug_camera {
            DEBUG_ZOOM
        } else {
            START_ZOOM
        };
    }

    /// The end of play: the controls stop, the activation centre goes back
    /// to the origin and its radius squared to 4e8 (200 m).
    pub fn end(&mut self) {
        self.playing = false;
        self.activation_centre = Vec3::ZERO;
        self.activation_radius2 = END_ACTIVATION_RADIUS2;
    }
}

/// Whether the camera's target yaw turns on a blocked screen: a blocked
/// screen (`0x58`) turns it whatever the pointer and the player do, so long
/// as no window consumes input; a modal form (a dialogue, the menu) or a
/// lifter/drag stops it; an open window stops it too and restamps the idle
/// clock.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CameraTurn {
    pub blocked: bool,
    pub modal: bool,
    pub window_open: bool,
    pub button_held: bool,
}

/// Whether a frame may turn the idle camera. A blocked screen turns it with
/// no idle test; otherwise the normal idle watch decides.
pub fn turns_when_blocked(turn: CameraTurn, idle: bool) -> bool {
    !turn.modal && !turn.window_open && !turn.button_held && (turn.blocked || idle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click(named: Pick) -> Click {
        Click {
            named,
            ..Click::default()
        }
    }

    /// A lock opens only with the key and pays its first opening: 1000 for
    /// the main door kind, 100 for the others. A plain hinge opens free.
    #[test]
    fn a_door_opens_with_its_key_and_pays_once() {
        let locked = Click {
            usable: Some(USABLE_LOCK),
            key_ok: false,
            first_time: true,
            ..click(Pick::Hinge)
        };
        assert_eq!(interact(locked), Interact::Nothing);
        let opened = Click {
            key_ok: true,
            ..locked
        };
        assert_eq!(
            interact(opened),
            Interact::Hinge {
                locked: true,
                key_ok: true,
                reward: 100
            }
        );
        let main = Click {
            lock_kind: Some(USABLE_MAIN),
            ..opened
        };
        assert_eq!(
            interact(main),
            Interact::Hinge {
                locked: true,
                key_ok: true,
                reward: 1000
            }
        );
        let again = Click {
            first_time: false,
            ..main
        };
        assert_eq!(
            interact(again),
            Interact::Hinge {
                locked: true,
                key_ok: true,
                reward: 0
            }
        );
        let plain = Click {
            first_time: true,
            ..click(Pick::Hinge)
        };
        assert_eq!(
            interact(plain),
            Interact::Hinge {
                locked: false,
                key_ok: false,
                reward: 0
            }
        );
    }

    /// A container opens, a corpse runs its scripts, a person is talked to,
    /// a usable prop is looked at and the rest does nothing.
    #[test]
    fn a_click_names_the_interaction() {
        let container = Click {
            container: true,
            ..click(Pick::Loose)
        };
        assert_eq!(interact(container), Interact::OpenContainer);
        assert_eq!(interact(click(Pick::Corpse)), Interact::CorpseScripts);
        assert_eq!(interact(click(Pick::Person)), Interact::Talk);
        let vial = Click {
            usable: Some(8),
            ..click(Pick::Usable)
        };
        assert_eq!(interact(vial), Interact::Look);
        assert_eq!(interact(click(Pick::Nothing)), Interact::Nothing);
        assert!(clicks_self(Pick::Person, true));
        assert!(!clicks_self(Pick::Person, false));
        assert!(!clicks_self(Pick::Corpse, true));
    }

    /// A second press within 300 ms and two pixels is a double click.
    #[test]
    fn a_double_click_is_close_in_time_and_space() {
        assert!(is_double_click((10, 10), 1000, (12, 8), 1299));
        assert!(!is_double_click((10, 10), 1000, (12, 8), 1300));
        assert!(!is_double_click((10, 10), 1000, (13, 10), 1100));
        assert!(!is_double_click((10, 10), 1000, (10, 13), 1100));
    }

    /// A liquid pours 0.1 from the hot kind and 0.2 otherwise; a fire
    /// lights for 1800 ms.
    #[test]
    fn a_use_warms_or_lights() {
        assert_eq!(pour_warmth(true), 0.1);
        assert_eq!(pour_warmth(false), 0.2);
        assert_eq!(
            use_on(UseKind::Liquid { hot: true }, Pick::Person, None, 0),
            Use::Warm {
                warmth: 0.1,
                hot: true
            }
        );
        assert_eq!(
            use_on(UseKind::Fire, Pick::Usable, None, 0),
            Use::Ignite { timer_ms: 1800 }
        );
        assert_eq!(
            use_on(UseKind::Liquid { hot: false }, Pick::Loose, None, 0),
            Use::Nothing
        );
    }

    /// The cleaning kit stops the dirt byte over 0x20 (0.032 is kept); the
    /// repair kit takes a tenth of the material rating, rounded up, from
    /// the condition.
    #[test]
    fn cleaning_and_repairing() {
        assert_eq!(clean_to(0x30), Some(0x10));
        assert_eq!(clean_to(0x21), Some(1));
        assert_eq!(clean_to(0x20), None);
        assert_eq!(clean_to(0), None);
        // rating 25 → (25 + 5)/10 = 3 → 255/3 = 85 off.
        assert_eq!(repair_to(100, 25), 15);
        assert_eq!(repair_to(50, 25), 0);
        // rating 0 → step 1 → 255 off, floored at 0.
        assert_eq!(repair_to(100, 0), 0);
        // rating 45 → 5 → 51 off.
        assert_eq!(repair_to(100, 45), 49);
        let item = ItemState {
            dirt: 0x30,
            condition: 100,
            rating: 25,
        };
        let clean = use_on(UseKind::Clean, Pick::Usable, Some(&item), 1000);
        assert_eq!(
            clean,
            Use::Timed(Timed {
                kind: 9,
                from: 0x30,
                to: 0x10,
                started_ms: 1000
            })
        );
        let repair = use_on(UseKind::Repair, Pick::Usable, Some(&item), 1000);
        assert_eq!(
            repair,
            Use::Timed(Timed {
                kind: 10,
                from: 100,
                to: 15,
                started_ms: 1000
            })
        );
    }

    /// A timed value moves 0.0005 of its gap a millisecond, rounding to the
    /// nearest even byte, and is exactly its end at two seconds.
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

    /// The reach: 245 cm high and 300 across for a point, 300 and 350 for a
    /// door, the vertical offset to 300, and the region's own test only for
    /// a sim object.
    #[test]
    fn the_reach_keeps_its_limits_and_the_regions() {
        let base = Reach {
            height_cm: 245.0,
            offset_cm: Vec3::new(300.0, 0.0, 0.0),
            frame: ReachFrame::Point,
            region: Region::None,
            object: false,
        };
        assert!(within_reach(base));
        assert!(!within_reach(Reach {
            height_cm: 245.1,
            ..base
        }));
        assert!(!within_reach(Reach {
            offset_cm: Vec3::new(300.1, 0.0, 0.0),
            ..base
        }));
        assert!(!within_reach(Reach {
            offset_cm: Vec3::new(0.0, -300.1, 0.0),
            ..base
        }));
        assert!(within_reach(Reach {
            frame: ReachFrame::Door,
            height_cm: 300.0,
            offset_cm: Vec3::new(350.0, 0.0, 0.0),
            ..base
        }));
        assert!(!within_reach(Reach {
            frame: ReachFrame::Door,
            height_cm: 300.1,
            ..base
        }));
        assert!(!within_reach(Reach {
            frame: ReachFrame::Wheel,
            height_cm: 250.0,
            ..base
        }));
        // A plain point ignores the region; a sim object fails outside it.
        assert!(within_reach(Reach {
            region: Region::Outside,
            ..base
        }));
        assert!(!within_reach(Reach {
            object: true,
            region: Region::Outside,
            ..base
        }));
        assert!(within_reach(Reach {
            object: true,
            region: Region::Inside,
            ..base
        }));
        // NaN passes every ordered test.
        assert!(within_reach(Reach {
            height_cm: f32::NAN,
            offset_cm: Vec3::new(f32::NAN, f32::NAN, f32::NAN),
            ..base
        }));
    }

    /// The latch holds after a key or button release until the walk reaches
    /// segment 0 or 4, and skips the reach meanwhile; a stopped body lets it
    /// go at once.
    #[test]
    fn the_walk_to_cursor_latch_holds_through_the_stride() {
        let mut latch = WalkLatch::default();
        assert!(!latch.skips_reach());
        latch.frame(true, false, None, 100);
        assert!(latch.skips_reach() && latch.since_ms == 100);
        latch.frame(false, true, Some(2), 150);
        assert!(latch.skips_reach());
        latch.frame(false, true, Some(4), 200);
        assert!(!latch.skips_reach());
        latch.frame(true, true, None, 300);
        latch.frame(false, false, Some(1), 350);
        assert!(!latch.skips_reach());
    }

    /// The start of play activates a 40 m radius around the player and the
    /// end puts it back to the origin with 200 m; the camera starts at 1300,
    /// or 800 with the debug setting.
    #[test]
    fn the_start_and_end_of_play() {
        let mut s = PlaySession::default();
        assert!(!s.playing);
        s.begin(Vec3::new(10.0, 0.0, 20.0));
        assert!(s.playing);
        assert_eq!(s.activation_centre, Vec3::new(10.0, 0.0, 20.0));
        assert_eq!(s.activation_radius2, 16_000_000.0);
        assert_eq!(s.zoom, 1300.0);
        s.end();
        assert!(!s.playing);
        assert_eq!(s.activation_centre, Vec3::ZERO);
        assert_eq!(s.activation_radius2, 400_000_000.0);
        let mut d = PlaySession {
            debug_camera: true,
            ..PlaySession::default()
        };
        d.begin(Vec3::ZERO);
        assert_eq!(d.zoom, DEBUG_ZOOM);
        // The radius squared roots to 40 m and 200 m.
        assert_eq!(START_ACTIVATION_RADIUS2.sqrt(), 4000.0);
        assert_eq!(END_ACTIVATION_RADIUS2.sqrt(), 20_000.0);
    }

    /// A blocked screen turns the camera with no idle test; a modal form, an
    /// open window or a button stops it.
    #[test]
    fn a_blocked_screen_turns_the_camera() {
        let turn = CameraTurn {
            blocked: true,
            modal: false,
            window_open: false,
            button_held: false,
        };
        assert!(turns_when_blocked(turn, false));
        assert!(turns_when_blocked(turn, true));
        for blocked in [
            CameraTurn {
                modal: true,
                ..turn
            },
            CameraTurn {
                window_open: true,
                ..turn
            },
            CameraTurn {
                button_held: true,
                ..turn
            },
        ] {
            assert!(!turns_when_blocked(blocked, true));
        }
        let free = CameraTurn {
            blocked: false,
            ..turn
        };
        assert!(!turns_when_blocked(free, false));
        assert!(turns_when_blocked(free, true));
    }

    /// A click farther than the stop leaves a goal short of the point; 245 cm
    /// (350 for a door) away or nearer needs no walk.
    #[test]
    fn a_click_walks_to_its_stop() {
        let at = Vec3::ZERO;
        let goal = walk_stop(at, Vec3::new(0.0, 0.0, 5.0), false).unwrap();
        assert!((goal.z - 2.55).abs() < 1e-5, "{goal}");
        let goal = walk_stop(at, Vec3::new(0.0, 0.0, 5.0), true).unwrap();
        assert!((goal.z - 1.5).abs() < 1e-5, "{goal}");
        assert_eq!(walk_stop(at, Vec3::new(0.0, 0.0, 2.45), false), None);
        assert_eq!(walk_stop(at, Vec3::new(0.0, 0.0, 2.0), false), None);
    }
}

/// The input gate's state (F135/F136), in Exarust's terms: whether the
/// mouse is free/look-enabled, which Steam action set the original selects
/// (0 none, 1 interact, 2 combat), and the player's own flags the switches
/// clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InputGate {
    /// The mouse cursor look is on (the controls set their read callback).
    pub cursor_active: bool,
    /// The right stick's look is disabled instead (the spectate switch's
    /// own flag).
    pub look_disabled: bool,
    /// The controls' own "off" flag.
    pub controls_off: bool,
    /// The player's input is blocked (`+0x58`).
    pub player_blocked: bool,
    /// The held keys are cleared and the combat/heading flags reset.
    pub keys_cleared: bool,
    /// The Steam action set the original selects: 0, 1 or 2.
    pub steam_set: u8,
    /// The free-cursor flag (`+0xd80`), cleared when the input goes off.
    pub free_cursor: bool,
}

/// The game's input switch (F135): enabled, the cursor look is on, the
/// player is unblocked and the Steam set is the interact (1) or combat (2)
/// one; disabled, the look callback is dropped, the keys and the player's
/// combat/heading/aim flags are cleared, the player is blocked and the
/// Steam set is 0.
pub fn gameplay_input(enabled: bool, combat: bool) -> InputGate {
    if enabled {
        InputGate {
            cursor_active: true,
            look_disabled: false,
            controls_off: false,
            player_blocked: false,
            keys_cleared: false,
            steam_set: if combat { 2 } else { 1 },
            free_cursor: true,
        }
    } else {
        InputGate {
            cursor_active: false,
            look_disabled: false,
            controls_off: true,
            player_blocked: true,
            keys_cleared: true,
            steam_set: 0,
            free_cursor: false,
        }
    }
}

/// The spectate input switch (F136): enabled, the cursor look is on and the
/// player is unblocked; disabled, the look callback is dropped, the keys
/// are cleared, the spectate look flag (`+0xaf`) is set and the player is
/// blocked.
pub fn spectate_input(enabled: bool, combat: bool) -> InputGate {
    if enabled {
        InputGate {
            cursor_active: true,
            look_disabled: false,
            controls_off: false,
            player_blocked: false,
            keys_cleared: false,
            steam_set: if combat { 2 } else { 1 },
            free_cursor: true,
        }
    } else {
        InputGate {
            cursor_active: false,
            look_disabled: true,
            controls_off: false,
            player_blocked: true,
            keys_cleared: true,
            steam_set: 0,
            free_cursor: false,
        }
    }
}

/// The spectate mode (F137): whether spectating, the controls' dialogue
/// flag and the permission mask. Spectating narrows the mask to powers (4);
/// leaving restores all five.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spectate {
    pub spectating: bool,
    pub controls_flag: bool,
    pub permission: u32,
}

pub const PLAY_PERMISSIONS: u32 = 0x1F;
pub const SPECTATE_PERMISSIONS: u32 = 0x4;

/// The spectate switch.
pub fn spectate(enabled: bool) -> Spectate {
    if enabled {
        Spectate {
            spectating: true,
            controls_flag: true,
            permission: SPECTATE_PERMISSIONS,
        }
    } else {
        Spectate {
            spectating: false,
            controls_flag: false,
            permission: PLAY_PERMISSIONS,
        }
    }
}

/// Whether the viewport's input word asks for a GUI task (F62): bit 0 opens
/// the task through the interface's call. Returns whether it would open;
/// the task itself is the GUI layer's.
pub fn opens_gui_task(input_flags: u32) -> bool {
    input_flags & 1 != 0
}

/// The input a disabled controls object keeps (F134): the controls' clock,
/// the mouse read off, the keys cleared, the player blocked, and its
/// combat/heading/aim words and the drag stepping reset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisabledInput {
    /// The game clock (ms) the switch ran.
    pub now_ms: u32,
    pub gate: InputGate,
    /// The player's combat word, heading word and drag step are cleared.
    pub combat_cleared: bool,
}

/// Disables the player's input, as the death and menu switches do (F134):
/// the controls' clock takes `now`, the gameplay gate goes off, and the
/// player's combat/heading/aim words reset.
pub fn disable_input(now_ms: u32) -> DisabledInput {
    DisabledInput {
        now_ms,
        gate: gameplay_input(false, false),
        combat_cleared: true,
    }
}

/// A sound the controls ask for at a world point (F110): the item/event's
/// sound name, its place, gain 0.25 and natural pitch, not looped.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundRequest {
    pub name: String,
    pub at: Vec3,
    pub gain: f32,
    pub pitch: f32,
    pub looping: bool,
}

/// The controls' point-sound request (F110): the name is resolved by the
/// audio environment and played at `at` with a quarter gain and pitch 1.
pub fn point_sound(name: impl Into<String>, at: Vec3) -> SoundRequest {
    SoundRequest {
        name: name.into(),
        at,
        gain: 0.25,
        pitch: 1.0,
        looping: false,
    }
}

/// The key direction word from the movement keys and the gamepad stick
/// (F29): the stick over 0.01 (double) is 0x8000 (the caller keeps its
/// vector); otherwise W is bit 1, A bit 2, S bit 4 and D bit 8, with A and
/// D cancelling and S taken only when W is not held (W cleared in its
/// place).
pub fn key_direction_word(w: bool, a: bool, s: bool, d: bool, stick: f64) -> u32 {
    if stick > 0.01 {
        return 0x8000;
    }
    let mut word = 0;
    if a {
        word = 2;
    }
    if d {
        if word == 2 {
            word = 0;
        } else {
            word = 8;
        }
    }
    if w {
        word |= 1;
    }
    if s {
        if word & 1 == 0 {
            word |= 4;
        } else {
            word &= !1;
        }
    }
    word
}

/// The direction a key word asks in a body's frame (F28): W along
/// `forward`, A against `right`, D along it, S against `forward`, then
/// normalized. A zero vector when nothing is held.
pub fn key_direction(word: u32, forward: Vec3, right: Vec3) -> Vec3 {
    let mut dir = Vec3::ZERO;
    if word & 1 != 0 {
        dir = forward;
    }
    if word & 2 != 0 {
        dir -= right;
    }
    if word & 4 != 0 {
        dir -= forward;
    }
    if word & 8 != 0 {
        dir += right;
    }
    dir.normalize_or_zero()
}

/// Whether a walk-to goal is still over 20 cm away (F30's squared
/// comparison against 400 cm²; both points in metres).
pub fn walk_goal_far(from: Vec3, goal: Vec3) -> bool {
    let d = (goal - from) * 100.0;
    d.length_squared() > 400.0
}

/// The power tree's delayed key (F143): the slot is 1-based and its
/// deadline is half a second on.
pub fn power_hold(index: usize, now_ms: u32) -> (usize, u32) {
    (index + 1, now_ms.wrapping_add(500))
}

/// One registered game mode (F152): its 1-based id, its name, the level
/// kind a start gives it (id + 2), and the callbacks' names kept as data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameMode {
    pub id: usize,
    pub name: &'static str,
    pub level_kind: usize,
}

/// The unit's game-mode registration (F152): Character then Spectate, each
/// taking the counter's next id and the id plus two as its level kind.
pub fn register_game_modes(first_id: usize) -> [GameMode; 2] {
    [
        GameMode {
            id: first_id,
            name: "Character",
            level_kind: first_id + 2,
        },
        GameMode {
            id: first_id + 1,
            name: "Spectate",
            level_kind: first_id + 3,
        },
    ]
}

#[cfg(test)]
mod more_tests {
    use super::*;

    /// The gameplay and spectate switches: enabled restores the cursor and
    /// the player; disabled blocks, clears and drops the Steam set.
    #[test]
    fn the_input_switches() {
        let on = gameplay_input(true, false);
        assert!(on.cursor_active && !on.player_blocked && !on.keys_cleared);
        assert_eq!(on.steam_set, 1);
        let combat = gameplay_input(true, true);
        assert_eq!(combat.steam_set, 2);
        let off = gameplay_input(false, false);
        assert!(!off.cursor_active && off.controls_off && off.player_blocked);
        assert!(off.keys_cleared && off.steam_set == 0 && !off.free_cursor);
        let s = spectate_input(false, false);
        assert!(s.look_disabled && s.player_blocked && !s.controls_off);
        assert!(spectate_input(true, false).cursor_active);
    }

    /// Spectating keeps only the powers permission; leaving restores all.
    #[test]
    fn the_spectate_permissions() {
        assert_eq!(
            spectate(true),
            Spectate {
                spectating: true,
                controls_flag: true,
                permission: 4
            }
        );
        assert_eq!(
            spectate(false),
            Spectate {
                spectating: false,
                controls_flag: false,
                permission: 0x1F
            }
        );
    }

    #[test]
    fn the_power_tree_hold_is_half_a_second() {
        assert_eq!(power_hold(0, 1000), (1, 1500));
        let late = u32::MAX - 200;
        assert_eq!(power_hold(9, late), (10, late.wrapping_add(500)));
    }

    /// The key word pairs A with D and S with W; the stick wins past 0.01.
    #[test]
    fn the_key_direction_word_and_direction() {
        assert_eq!(key_direction_word(false, false, false, false, 0.0), 0);
        assert_eq!(key_direction_word(true, false, false, false, 0.0), 1);
        assert_eq!(key_direction_word(false, true, false, false, 0.0), 2);
        // A and D cancel.
        assert_eq!(key_direction_word(false, true, false, true, 0.0), 0);
        assert_eq!(key_direction_word(true, true, false, true, 0.0), 1);
        // S with W clears W and does not take S (both cancel).
        assert_eq!(key_direction_word(true, false, true, false, 0.0), 0);
        assert_eq!(key_direction_word(false, false, true, false, 0.0), 4);
        assert_eq!(key_direction_word(true, false, false, true, 0.01), 9);
        assert_eq!(key_direction_word(true, false, false, true, 0.02), 0x8000);
        // The direction in a body frame.
        let f = Vec3::Z;
        let r = Vec3::X;
        assert_eq!(key_direction(1, f, r), f);
        assert_eq!(key_direction(2, f, r), -r);
        assert_eq!(key_direction(4, f, r), -f);
        assert_eq!(key_direction(8, f, r), r);
        assert_eq!(key_direction(0, f, r), Vec3::ZERO);
        let diag = key_direction(9, f, r);
        assert!((diag - (f + r).normalize()).length() < 1e-6);
        // A goal 20.01 cm away is far; 20 cm is not.
        assert!(walk_goal_far(Vec3::ZERO, Vec3::new(0.2001, 0.0, 0.0)));
        assert!(!walk_goal_far(Vec3::ZERO, Vec3::new(0.2, 0.0, 0.0)));
    }

    /// The viewport's GUI task bit, the disabled input's fields and the
    /// point sound's numbers.
    #[test]
    fn the_controls_gui_task_disabled_input_and_sound() {
        assert!(opens_gui_task(1));
        assert!(!opens_gui_task(2));
        let off = disable_input(1234);
        assert_eq!(off.now_ms, 1234);
        assert!(off.gate.player_blocked && off.gate.keys_cleared && !off.gate.cursor_active);
        assert_eq!(off.gate.steam_set, 0);
        assert!(off.combat_cleared);
        let s = point_sound("doorcreak01", Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(s.gain, 0.25);
        assert_eq!(s.pitch, 1.0);
        assert!(!s.looping);
        assert_eq!(s.at, Vec3::new(1.0, 2.0, 3.0));
    }

    /// Character and Spectate take ids 1 and 2 and level kinds 3 and 4.
    #[test]
    fn the_game_modes_register_in_order() {
        let modes = register_game_modes(1);
        assert_eq!(modes[0].id, 1);
        assert_eq!(modes[0].name, "Character");
        assert_eq!(modes[0].level_kind, 3);
        assert_eq!(modes[1].id, 2);
        assert_eq!(modes[1].name, "Spectate");
        assert_eq!(modes[1].level_kind, 4);
    }
}

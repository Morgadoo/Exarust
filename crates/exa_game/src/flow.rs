//! The game's top-level flow: start-up, the mode switch, starting and ending
//! story, practice and arena sessions, level changes, pause and the
//! per-frame session tick.
//!
//! The original keeps this flow in one unit of plain routines over a handful
//! of globals: a game-mode word, a session-phase word, the current session
//! hook, the practice ladder, the story player's wake-up flags and two save
//! managers. [`Flow`] is that state and its methods are those routines. They
//! return [`Step`]s — what the original calls do to the rest of the game
//! (show a menu page, change the level, save, frame the camera, warp the
//! mouse) — so the rules are pure and testable and any layer can apply the
//! steps it owns. [`FlowPlugin`] runs the flow in the app: it boots it, ticks
//! it on the game clock, follows the player into a play level and turns a
//! teleporter journey into a level exit.
//!
//! The menu pages are [`crate::menu_pages`]' (the page table and the click
//! handler) and the running screen is [`crate::menu_screen`]'s; the save
//! packages are [`crate::slots`]' and [`crate::session`]'s; the practice
//! ladder's numbering and heal are [`crate::practice`]'s; the arena hub's
//! music and manager fall are [`crate::arena_hub`]'s. This module owns the
//! order those pieces run in. The splash screen is [`crate::splash`] and the
//! story scripts' `GameEvent` is [`crate::game_event`].
//!
//! Evidence: the game-main unit's boot, mode switch, session starters and
//! hooks, end menu, level exit, death and revival, quit and autosave.

use bevy::prelude::*;
use exa_formats::save_package;

use crate::arena_hub;
use crate::arena_match::Track;
use crate::menu_pages::{Page, PageState};
use crate::practice::{self, Ladder};

/// The title menu: no game loaded.
pub const MENU_MODE: u32 = 0;
/// The practice ladder (the arena's sparring fights).
pub const PRACTICE_MODE: u32 = 2;
/// The story campaign.
pub const STORY_MODE: u32 = 4;
/// The arena career (company, matches, the hub).
pub const ARENA_MODE: u32 = 8;
/// The bit that makes a game-mode word a scripted mode; its hook table is
/// outside this component. The versus lobby is `0x1010`.
pub const SCRIPT_MODE_BIT: u32 = 0x1000;
/// The versus lobby's mode word.
pub const VERSUS_MODE: u32 = 0x1010;

/// The transition screen is up; the phase that was running is kept.
pub const PHASE_TRANSITION: u32 = 0;
/// The main menu is up and no session runs.
pub const PHASE_MENU: u32 = 1;
/// A session is running.
pub const PHASE_PLAYING: u32 = 3;
/// The practice ladder is finished; the end menu shows.
pub const PHASE_WON: u32 = 4;
/// The session ended (death, defeat or a forfeit); the end menu shows.
pub const PHASE_ENDED: u32 = 5;
/// A practice round is won; the end menu shows.
pub const PHASE_ROUND_WON: u32 = 8;
/// A forfeit is being settled.
pub const PHASE_FORFEIT: u32 = 0x10;
/// The game is paused. The original tests this word before it autosaves on a
/// quit; the pause screen itself is the in-game menu page.
pub const PHASE_PAUSED: u32 = 0x20;

/// The condition word of a dead character.
pub const DEAD: u32 = 0xA00;
/// The dying bit of the condition word (the falls branch).
pub const DYING: u32 = 0x800;
/// The condition bit the story's wake-up check waits for.
pub const WAKING: u32 = 0x20;
/// How long the transition screen waits before it runs its action, in
/// milliseconds.
pub const TRANSITION_MS: u32 = 100;
/// How long after a practice fight starts a round can be won.
pub const PRACTICE_GUARD_MS: u32 = 1000;
/// How long a knocked-out story player waits before it is revived.
pub const KNOCKOUT_MS: u32 = 5000;
/// How long after the story's revive the player is fully up.
pub const WAKE_DELAY_MS: u32 = 2000;
/// How long before the story's delayed wake-up the scripted motion starts.
pub const WAKE_LEAD_MS: u32 = 3000;
/// How long a get-up holds its pose before the final write.
pub const GET_UP_MS: u32 = 500;
/// The wake-up motion each background starts the story with.
pub const START_MOTION_ANIMATION: u32 = 0x1d;
pub const START_MOTION_VILLAGER: u32 = 0x1b;
pub const START_MOTION_WAKING: u32 = 0x13;
/// The health at or above which a fallen player is knocked out rather than
/// dead, compared in double precision.
pub const ALIVE_HEALTH: f64 = 0.01;

/// The practice ladder's first fight number. The first partner is
/// `opponent01` (novice) or `opponent09` (expert).
pub const FIRST_FIGHT: i32 = 1;

/// The victory screen's menu number.
pub const MENU_WON: u32 = 2;
/// The next-fight screen's menu number.
pub const MENU_NEXT: u32 = 3;
/// The reconstructed-player end menu's number.
pub const MENU_RECONSTRUCTED: u32 = 4;
/// The arena's end menu (continue, lobby, exit).
pub const MENU_ARENA: u32 = 5;

/// The frame's task runs only while no main menu covers the game.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Hook {
    /// No session runs (the title, the arena hub's static screens, scripted
    /// modes).
    #[default]
    None,
    /// The story's per-frame hook.
    Story,
    /// The practice round's per-frame hook.
    Practice,
    /// The story player is knocked out and waits for revival.
    Knockout,
}

/// A game-mode word. Four modes are switched on by value; any other value is
/// a scripted mode with its own hook table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Menu,
    Practice,
    Story,
    Arena,
    /// Another word: a scripted mode (versus is [`VERSUS_MODE`]).
    Other(u32),
}

impl Mode {
    /// The word of this mode.
    pub const fn word(self) -> u32 {
        match self {
            Mode::Menu => MENU_MODE,
            Mode::Practice => PRACTICE_MODE,
            Mode::Story => STORY_MODE,
            Mode::Arena => ARENA_MODE,
            Mode::Other(word) => word,
        }
    }

    /// The mode a word names.
    pub const fn from_word(word: u32) -> Self {
        match word {
            MENU_MODE => Mode::Menu,
            PRACTICE_MODE => Mode::Practice,
            STORY_MODE => Mode::Story,
            ARENA_MODE => Mode::Arena,
            other => Mode::Other(other),
        }
    }

    /// Whether a session is loaded (any mode but the title).
    pub const fn is_game(self) -> bool {
        !matches!(self, Mode::Menu)
    }

    /// Whether the mode writes a save package of its own.
    pub const fn saves(self) -> bool {
        matches!(self, Mode::Story | Mode::Arena)
    }

    /// Whether the mode keeps a checkpoint beside its save.
    pub const fn uses_checkpoints(self) -> bool {
        matches!(self, Mode::Story)
    }

    /// Whether the mode belongs to the practice family (the arena's sparring
    /// ladder): the end menu branches on `mode & 2`, which only practice
    /// sets.
    pub const fn practice_family(self) -> bool {
        self.word() & 2 != 0
    }

    /// The frame hook the mode starts with. The arena hub has none of its
    /// own; practice and story install theirs when a round or level starts.
    pub const fn hook(self) -> Hook {
        match self {
            Mode::Practice => Hook::Practice,
            Mode::Story => Hook::Story,
            _ => Hook::None,
        }
    }
}

/// A session-phase word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Transition,
    Menu,
    Playing,
    Won,
    Ended,
    RoundWon,
    Forfeit,
    Paused,
    /// A word the executable does not use.
    Other(u32),
}

impl Phase {
    /// The word of this phase.
    pub const fn word(self) -> u32 {
        match self {
            Phase::Transition => PHASE_TRANSITION,
            Phase::Menu => PHASE_MENU,
            Phase::Playing => PHASE_PLAYING,
            Phase::Won => PHASE_WON,
            Phase::Ended => PHASE_ENDED,
            Phase::RoundWon => PHASE_ROUND_WON,
            Phase::Forfeit => PHASE_FORFEIT,
            Phase::Paused => PHASE_PAUSED,
            Phase::Other(word) => word,
        }
    }

    /// The phase a word names.
    pub const fn from_word(word: u32) -> Self {
        match word {
            PHASE_TRANSITION => Phase::Transition,
            PHASE_MENU => Phase::Menu,
            PHASE_PLAYING => Phase::Playing,
            PHASE_WON => Phase::Won,
            PHASE_ENDED => Phase::Ended,
            PHASE_ROUND_WON => Phase::RoundWon,
            PHASE_FORFEIT => Phase::Forfeit,
            PHASE_PAUSED => Phase::Paused,
            other => Phase::Other(other),
        }
    }
}

/// A viewport, in physical pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
}

/// Where the camera is put and what it looks at, with the mouse warp that
/// goes with the shot. Distances are centimetres, as everywhere in the game's
/// own geometry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CameraFrame {
    pub camera: Vec3,
    pub target: Vec3,
    /// The movement speed forced on the view (`Some(1300.0)` after a session
    /// start; `None` keeps the view's own).
    pub speed: Option<f32>,
    /// The client point the cursor is moved to.
    pub warp: (i32, i32),
}

/// The camera placement rules, as pure arithmetic.
pub mod camera {
    use super::{CameraFrame, Viewport};
    use bevy::math::Vec3;

    /// The camera's offset from the point in the far shot (the arena
    /// unit's own matches use it), 300, 1180, 1300 cm.
    pub const FAR_OFFSET: Vec3 = Vec3::new(300.0, 1180.0, 1300.0);
    /// The camera's offset from the point in the near shot (the practice
    /// start and next fight, the story continue, the arena hub), 700, 1180,
    /// 1100 cm.
    pub const NEAR_OFFSET: Vec3 = Vec3::new(700.0, 1180.0, 1100.0);
    /// The speed a fast shot forces on the view.
    pub const START_SPEED: f32 = 1300.0;
    /// The camera's offset from the player in the story start's shot.
    pub const STORY_OFFSET: Vec3 = Vec3::new(600.0, 600.0, 1300.0);
    /// The horizontal cursor offset of the point shots, per 1/1140 of the
    /// client height.
    pub const WARP_X: f32 = 120.0;
    /// The vertical cursor offset of the same shots.
    pub const WARP_Y: f32 = 200.0;
    /// The story shot's horizontal cursor offset.
    pub const STORY_WARP_X: f32 = 200.0;

    /// A single-precision value converted to an integer with the processor's
    /// default rounding: nearest, ties to even, and the integer-indefinite
    /// value for a NaN or an out-of-range result (what the original's
    /// conversion instruction does; Rust's `as` saturates).
    fn round_to_i64(value: f32) -> i64 {
        if !value.is_finite() || value >= 9.223_372e18 || value < -9.223_372e18 {
            i64::MIN
        } else {
            value.round_ties_even() as i64
        }
    }

    /// The shot's mouse warp: the client centre plus the scaled offsets. The
    /// scale is the client height over 1140 in double precision, kept in
    /// single precision for the products; the halves are unsigned shifts of
    /// the 32-bit values.
    pub fn warp(width: u32, height: u32, dx: f32, dy: f32) -> (i32, i32) {
        let scale = (f64::from(height) * 0.000_877_192_982_456_140_4) as f32;
        let cx = i64::from(width >> 1) + round_to_i64(scale * dx);
        let cy = i64::from(height >> 1) - round_to_i64(scale * dy);
        (cx as i32, cy as i32)
    }

    /// Frames `point`: the target at the point, the camera at the point plus
    /// [`FAR_OFFSET`] (`far`) or [`NEAR_OFFSET`], the start speed when
    /// `fast`, and the shot's cursor warp. (The original sets the camera to
    /// the offset and adds the point to it in place.)
    pub fn frame_point(point: Vec3, fast: bool, far: bool, viewport: Viewport) -> CameraFrame {
        CameraFrame {
            camera: (if far { FAR_OFFSET } else { NEAR_OFFSET }) + point,
            target: point,
            speed: fast.then_some(START_SPEED),
            warp: warp(viewport.width, viewport.height, WARP_X, WARP_Y),
        }
    }

    /// Frames two points: the camera at `camera`, looking at `target`.
    pub fn frame_two_points(
        target: Vec3,
        camera: Vec3,
        fast: bool,
        viewport: Viewport,
    ) -> CameraFrame {
        CameraFrame {
            camera,
            target,
            speed: fast.then_some(START_SPEED),
            warp: warp(viewport.width, viewport.height, WARP_X, WARP_Y),
        }
    }

    /// Frames the player at `point`: the camera at the point plus
    /// [`STORY_OFFSET`], the start speed, and the cursor moved left of
    /// centre.
    pub fn frame_player(point: Vec3, viewport: Viewport) -> CameraFrame {
        CameraFrame {
            camera: point + STORY_OFFSET,
            target: point,
            speed: Some(START_SPEED),
            warp: warp(viewport.width, viewport.height, -STORY_WARP_X, 0.0),
        }
    }
}

/// A spawned character's placement: a rotation (row-major 3×3) and a
/// position in centimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub rotation: [[f32; 3]; 3],
    pub position: Vec3,
}

/// The identity rotation.
pub const NO_ROTATION: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// The practice spawn table's rotation: a half turn about the vertical axis.
pub const PRACTICE_ROTATION: [[f32; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];

/// The practice arena's spawn point: the middle of the ring.
pub const PRACTICE_POINT: Vec3 = Vec3::new(0.0, 0.0, 500.0);

/// The story spawn table's position: the starting room's corner.
pub const STORY_POINT: Vec3 = Vec3::new(8200.0, 0.0, 3920.0);

/// The degrees-to-radians factor the spawn tables use for their angles.
pub const DEGREES: f32 = 0.017_453_292_384_743_69;

/// A rotation about the vertical axis: [`rotation`] of the yaw alone.
pub fn yaw(degrees: f32) -> [[f32; 3]; 3] {
    rotation(0.0, degrees, 0.0)
}

/// The engine's rotation from three angles in degrees (x, y, z), as the
/// spawn tables build it: each angle times [`DEGREES`] in single precision,
/// its sine and cosine from the x87 unit (rounded to single), and the rows
///
/// - `(cz·cy, sz, −cz·sy)`,
/// - `(sx·sy + −(cx·sz)·cy, cx·cz, sx·cy + (cx·sz)·sy)`,
/// - `(cx·sy + (sx·sz)·cy, −sx·cz, cx·cy + −(sx·sz)·sy)`,
///
/// each sum in that order. A yaw alone gives `(cy, 0, −sy)`, `(0, 1, 0)`,
/// `(sy, 0, cy)`.
pub fn rotation(x: f32, y: f32, z: f32) -> [[f32; 3]; 3] {
    let sin_cos = |degrees: f32| {
        let angle = f64::from(degrees * DEGREES);
        (angle.sin() as f32, angle.cos() as f32)
    };
    let (sx, cx) = sin_cos(x);
    let (sy, cy) = sin_cos(y);
    let (sz, cz) = sin_cos(z);
    let cxsz = cx * sz;
    let sxsz = sx * sz;
    [
        [cz * cy, sz, -cz * sy],
        [sx * sy + -cxsz * cy, cx * cz, sx * cy + cxsz * sy],
        [cx * sy + sxsz * cy, -sx * cz, cx * cy + -sxsz * sy],
    ]
}

/// The story background a new story starts from: the unknown hero, a knight,
/// a proctor, or the villager. [`crate::create::background_preset`] names the
/// same four.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Background {
    Unknown,
    Knight,
    Proctor,
    Villager,
}

impl Background {
    /// The background of the character creator's index, or the unknown one.
    pub const fn from_index(index: u32) -> Self {
        match index {
            1 => Background::Knight,
            2 => Background::Proctor,
            3 => Background::Villager,
            _ => Background::Unknown,
        }
    }

    /// The creator's index.
    pub const fn index(self) -> u32 {
        match self {
            Background::Unknown => 0,
            Background::Knight => 1,
            Background::Proctor => 2,
            Background::Villager => 3,
        }
    }
}

/// What a new story does for its background: the role added to the player,
/// the story flag, the spawn pose, the starting light and the companion.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoryStart {
    pub background: Background,
    /// The role ID added to the player's roles.
    pub role: u32,
    /// The story flag word the record gets.
    pub flags: u32,
    pub placement: Placement,
    /// The starting light: the unknown background's and the villager's; the
    /// knight and proctor start without one.
    pub light: Option<StoryLight>,
    /// Derrin, on the unknown background only.
    pub companion: Option<Companion>,
}

/// The story's starting light: a placed light object, its rotation and its
/// intensity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoryLight {
    pub position: Vec3,
    pub rotation_degrees: Vec3,
    pub intensity: f32,
}

/// A story companion placed with the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Companion {
    pub template: &'static str,
    pub position: Vec3,
    /// The facing, in degrees.
    pub facing: f32,
    /// The record's item flag word.
    pub flags: u32,
}

/// The starting light's intensity (a byte-sized 256, stored as a float).
pub const STORY_LIGHT_INTENSITY: f32 = 256.0;
/// The companion's record flag.
pub const COMPANION_FLAGS: u32 = 0x500;

/// What starting a new story does, from its background and the player's
/// height. The spawn table has three entries: the unknown background (the
/// villager's room, with Derrin), the knight and proctor (one pose), and the
/// villager (another).
pub fn story_start(background: Background, body_height: f32) -> StoryStart {
    match background {
        Background::Unknown => StoryStart {
            background,
            role: 0x0e,
            flags: 0x200,
            placement: Placement {
                rotation: yaw(150.0),
                position: Vec3::new(
                    STORY_POINT.x - 100.0,
                    body_height * 5.0 + 25.0,
                    STORY_POINT.z,
                ),
            },
            light: Some(StoryLight {
                position: Vec3::new(8045.0, 4.7, 4075.0),
                rotation_degrees: Vec3::new(-48.0, 110.0, 88.0),
                intensity: STORY_LIGHT_INTENSITY,
            }),
            companion: Some(Companion {
                template: "xac_derrin",
                position: Vec3::new(5808.0, 0.0, -4075.0),
                facing: -90.0,
                flags: COMPANION_FLAGS,
            }),
        },
        Background::Knight | Background::Proctor => StoryStart {
            background,
            role: 0x1b,
            flags: 0,
            placement: Placement {
                rotation: yaw(-50.0),
                position: Vec3::new(STORY_POINT.x, 210.0, STORY_POINT.z),
            },
            light: None,
            companion: None,
        },
        Background::Villager => StoryStart {
            background,
            role: 0x18,
            flags: 0x600,
            placement: Placement {
                rotation: rotation(50.0, -50.0, 2.0),
                position: Vec3::new(STORY_POINT.x, 300.0, STORY_POINT.z),
            },
            light: Some(StoryLight {
                position: Vec3::new(8000.0, 500.0, 4000.0),
                rotation_degrees: Vec3::new(8.0, -150.0, 50.0),
                intensity: STORY_LIGHT_INTENSITY,
            }),
            companion: None,
        },
    }
}

/// The story spawn height rule: the player's pose sits `5 + health·5` cm
/// above the spawn table's floor, or 13 cm higher for a body that wakes up
/// (the original tests one flag of the player's record).
pub fn spawn_height(health: f32, waking: bool) -> f32 {
    health * 5.0 + if waking { 13.0 } else { 5.0 }
}

/// The practice spawn height: 5 cm above the arena floor.
pub fn practice_height(health: f32) -> f32 {
    health * 5.0 + 5.0
}

/// One row of the end-of-session menu.
pub type EndRow = &'static str;

/// The end-of-session menu the original builds: its rows, its scale and
/// whether it refreshes the pad bindings.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EndMenu {
    /// The menu number (2 victory, 3 next fight, 4 reconstructed, 5 arena).
    pub kind: u32,
    pub rows: &'static [EndRow],
    /// The form's item scale.
    pub scale: f32,
    /// The pad/steam bindings are refreshed for the menu.
    pub pad_bindings: bool,
}

impl EndMenu {
    /// The labels, for callers that only need them.
    pub fn labels(&self) -> &'static [EndRow] {
        self.rows
    }
}

const WON_ROWS: &[EndRow] = &["EXIT;"];
const PRACTICE_OVER_ROWS: &[EndRow] = &["RESTART", "EXIT"];
const RECONSTRUCTED_ROWS: &[EndRow] = &["CONTINUE", "EXIT"];
const ARENA_ROWS: &[EndRow] = &["CONTINUE", "LOBBY", "EXIT"];
const NEXT_ROWS: &[EndRow] = &["NEXT FIGHT"];

/// The end-of-session menu of `kind` in `mode`.
///
/// Kind 4 is the reconstructed player's (CONTINUE, EXIT). Kind 2 is the
/// practice ladder's end: a story or arena session shows the single `EXIT;`
/// row, practice RESTART and EXIT. Kind 5 is the arena's (CONTINUE, LOBBY,
/// EXIT). Every other kind is the next-fight page. The scales are the form's
/// 0.9 (default), 0.85 (two rows) and 0.78 (the arena's).
pub fn end_menu(kind: u32, mode: Mode) -> EndMenu {
    match kind {
        MENU_RECONSTRUCTED => EndMenu {
            kind,
            rows: RECONSTRUCTED_ROWS,
            scale: 0.85,
            pad_bindings: false,
        },
        MENU_WON => {
            if mode.practice_family() {
                EndMenu {
                    kind,
                    rows: PRACTICE_OVER_ROWS,
                    scale: 0.85,
                    pad_bindings: false,
                }
            } else {
                EndMenu {
                    kind,
                    rows: WON_ROWS,
                    scale: 0.9,
                    pad_bindings: false,
                }
            }
        }
        MENU_ARENA => EndMenu {
            kind,
            rows: ARENA_ROWS,
            scale: 0.78,
            pad_bindings: true,
        },
        _ => EndMenu {
            kind,
            rows: NEXT_ROWS,
            scale: 0.9,
            pad_bindings: false,
        },
    }
}

/// What the practice manager learned when it looked for a fallen opponent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundEnd {
    /// A living player and no dead opponent.
    None,
    /// The eighth fight was won.
    Last,
    /// A round before the last was won.
    Next,
    /// The player is dead: the death path runs.
    PlayerDead,
}

/// The practice round's check: the phase must be playing and the start guard
/// must have passed. A falling player ends nothing unless it is dead (then
/// the death path runs); otherwise the first dead other character ends the
/// round. `None` means the check does not run.
pub fn practice_round(
    phase: Phase,
    now: u32,
    start_ms: u32,
    player_condition: u32,
    others: &[u32],
    fight: i32,
) -> Option<RoundEnd> {
    if phase != Phase::Playing
        || u64::from(start_ms) + u64::from(PRACTICE_GUARD_MS) > u64::from(now)
    {
        return None;
    }
    if player_condition & DYING != 0 {
        return Some(if player_condition == DEAD {
            RoundEnd::PlayerDead
        } else {
            RoundEnd::None
        });
    }
    if others.contains(&DEAD) {
        return Some(if fight == practice::PRACTICE_FIGHTS {
            RoundEnd::Last
        } else {
            RoundEnd::Next
        });
    }
    Some(RoundEnd::None)
}

/// The story's wake-up flags. The original keeps three bytes and the start
/// time they share; the delayed branch starts a scripted motion three
/// seconds before its start time, then changes the pose.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Wake {
    /// The first wake-up animation plays.
    pub animating: bool,
    /// The player is getting up; the pose is held until the start time.
    pub getting_up: bool,
    /// A scripted motion waits for its lead time.
    pub delayed: bool,
    /// The time the wake-up or the get-up starts.
    pub start_ms: u32,
}

/// A wake-up pose write: the motion requested, the body's condition word,
/// the delay it starts after, and the move and turn factors.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WakePose {
    pub request: Option<u32>,
    pub condition: u32,
    pub delay_ms: i32,
    pub turn: f32,
    pub move_factor: f32,
    pub flags: u32,
}

/// The story session's per-frame state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StoryState {
    /// The last whole second the discovery system ran at.
    pub last_discovery_s: u32,
    pub wake: Wake,
    /// The knock-out start time; `u32::MAX - 5000` until a knock-out.
    pub knockout_ms: u32,
    /// The reconstruction state: 0 none, 1 can be rebuilt, 2 stripped.
    pub reconstruction: u8,
    /// The player's record ID.
    pub record: u32,
    /// The record ID the checkpoint was made at.
    pub checkpoint: u32,
}

/// The practice session's state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PracticeState {
    pub ladder: Ladder,
    pub fight: i32,
    pub start_ms: u32,
    /// The next end page draws the second tip set instead of a random tip.
    pub next_tip: bool,
}

impl Default for PracticeState {
    fn default() -> Self {
        Self {
            ladder: Ladder::Novice,
            fight: 0,
            start_ms: 0,
            next_tip: false,
        }
    }
}

/// The settings unlock words: the practice ladder's expert difficulty and
/// the story backgrounds. The original reads and writes them as 4-byte
/// settings entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Unlocks {
    /// The expert ladder is unlocked.
    pub expert: bool,
    /// The story background bits.
    pub backgrounds: u32,
}

/// The settings script's hook name the game main unit registers.
pub const SCRIPT_HOOK: &str = "GameEvent";
/// The settings keys the game main unit registers. Their values are read by
/// the units named next to them, not here.
pub const SETTINGS_KEYS: [&str; 6] = [
    "CursorSize",
    "CursorInt",
    "CursorCom",
    "FirstRun",
    "ShowGameTips",
    "CheckPoints",
];

impl Unlocks {
    /// Reads an unlock word from the settings bytes: a 4-byte little-endian
    /// integer, or `None` when the entry is missing or short.
    pub fn read_word(bytes: &[u8]) -> Option<u32> {
        let word = bytes.get(0..4)?;
        Some(u32::from_le_bytes([word[0], word[1], word[2], word[3]]))
    }

    /// Reads the background word, leaving the current bits when the entry is
    /// missing.
    pub fn read_backgrounds(&mut self, bytes: &[u8]) {
        if let Some(word) = Self::read_word(bytes) {
            self.backgrounds = word;
        }
    }

    /// Reads the expert unlock's word: anything but zero unlocks it.
    pub fn read_expert(&mut self, bytes: &[u8]) {
        if let Some(word) = Self::read_word(bytes) {
            self.expert = word != 0;
        }
    }

    /// The background bits as the settings entry's 4 bytes.
    pub fn background_bytes(&self) -> [u8; 4] {
        self.backgrounds.to_le_bytes()
    }

    /// The expert unlock as the settings entry's 4 bytes: one when unlocked.
    pub fn expert_bytes(&self) -> [u8; 4] {
        u32::from(self.expert).to_le_bytes()
    }
}

/// A task the game's boot performs. The pieces that live outside this
/// component are named, not performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootTask {
    /// Create the window's graphics context and the GUI's graphics.
    Window,
    /// Open the audio device.
    Audio,
    /// Load the actor/character template library.
    ActorLibrary,
    /// Initialise the game layer's interface.
    GameGui,
    /// Initialise the item interface and the skill screen.
    ItemGui,
    /// Read the hardware settings.
    HardwareConfig,
    /// Apply the first-run settings.
    FirstRun,
    /// Read the unlock words.
    Settings,
    /// Create a save manager for the mode.
    SaveManager(Mode),
    /// Initialise the arena's data.
    ArenaData,
    /// Create the transition screen.
    Transition,
    /// Register the script hooks (the story game events and the settings
    /// keys).
    Scripts,
    /// Show the splash screen.
    Splash,
    /// Build the title page.
    MainMenu,
}

/// A scripted mode's hook.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptHook {
    Enter,
    Begin,
    End,
}

/// One thing the flow asks the game to do. The original performs these
/// through its globals and forms; Exarust names them so a host can apply the
/// ones it owns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Step {
    Boot(BootTask),
    /// Build and show a main-menu page.
    ShowMenu(Page),
    HideMenu,
    /// Start a music track at a volume; `restart` begins it from the top.
    Music {
        name: &'static str,
        volume: f32,
        restart: bool,
    },
    /// Tick the dialogue engine.
    Dialogue,
    /// Run the discovery system's once-a-second pass.
    Discovery,
    /// Show one of the help tips.
    Tip(u32),
    /// Load a level.
    LoadLevel {
        locale: &'static str,
        /// The spawn point within the level; zero is the level's own.
        spawn: u32,
        /// Put the level's objects in a fresh world first.
        teardown: bool,
    },
    /// Place the player.
    PlacePlayer(Placement),
    /// Add the player's story role and flags.
    StoryPlayer {
        role: u32,
        flags: u32,
    },
    /// Create the story's starting light.
    StoryLight(StoryLight),
    /// Place the story's companion.
    Companion(Companion),
    /// Frame the camera.
    Camera(CameraFrame),
    /// Move the mouse cursor to this client point.
    WarpCursor {
        x: i32,
        y: i32,
    },
    /// Add or remove the frame task that runs the session hook.
    SessionTask(bool),
    /// The session hook now in force.
    Hook(Hook),
    /// Write the save; `drop_on_low_health` is the death rule.
    Save {
        drop_on_low_health: bool,
    },
    /// Delete the current slot's save.
    DropSave,
    /// Keep the save just written as the checkpoint.
    Checkpoint,
    /// Leave the world: props, bodies, particles, blood and the level.
    TeardownWorld,
    /// Show the end-of-session menu.
    EndMenu(EndMenu),
    /// Hide the end-of-session menu.
    HideEndMenu,
    /// Clear or set the menu's help flag (the form field the help screen
    /// clears when it closes).
    MenuHelp(bool),
    /// The GUI's scale was re-quantised; both menus lay out again.
    GuiScale(f32),
    /// The arena's own pause-screen callback runs.
    ArenaPaused,
    /// The session forfeits the match.
    Forfeit,
    /// A story hook another unit installed runs.
    CustomHook,
    /// The menu's hover sound: this gain at the menu's volume.
    Hover {
        volume: f32,
        gain: f32,
    },
    /// The menu confirms its selected row.
    MenuConfirm,
    /// Give or take the player's controls.
    Controls(bool),
    /// Strip the player's gear (the reconstruction device).
    StripPlayer,
    /// Write the player's own record back into the save.
    PlayerData,
    /// Show or hide the game's view.
    GameGui(bool),
    /// Write a wake-up or get-up pose.
    Wake(WakePose),
    /// The motion the player is asked to play.
    RequestAction(u32),
    /// The health the practice heal writes (red, then yellow).
    Health {
        red_bits: u32,
        yellow_bits: u32,
    },
    /// A scripted mode's hook.
    ScriptMode {
        mode: u32,
        hook: ScriptHook,
    },
    /// The transition screen starts; the action runs after
    /// [`TRANSITION_MS`].
    Transition,
    /// Leave the application (the title's EXIT).
    Quit,
}

/// The action behind the transition screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Close the session and go back to the title.
    QuitToMenu,
    /// The story's death screen CONTINUE: reload the player's record and
    /// respawn at the fifth level's portal.
    StoryRespawn,
    /// Revive the knocked-out story player where it fell.
    Revive,
    /// Start the next practice fight.
    NextPracticeFight,
    /// Load a named level at a spawn point.
    Locale { locale: &'static str, spawn: u32 },
}

/// The running transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Transition {
    action: Action,
    at_ms: u32,
}

/// A practice start's data. The partner is `opponent01` (novice) or
/// `opponent09` (expert); the boss music plays on the last fight.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PracticeStart {
    pub ladder: Ladder,
    pub fight: i32,
    pub partner: [u8; 10],
    pub boss: bool,
    pub locale: &'static str,
    pub music: Track,
}

/// The music a practice start plays.
pub fn practice_music(ladder: Ladder, alternative: bool) -> Track {
    let (novice, expert) = if alternative {
        ("Msc_ArenaNov_DnB", "Msc_ArenaExp_DnB")
    } else {
        ("Msc_AreanNov", "Msc_AreanExp")
    };
    let (name, bits) = match ladder {
        Ladder::Novice => (novice, 0x3dcc_cccd),
        Ladder::Expert => (expert, 0x3e80_0000),
    };
    Track {
        name,
        volume: f32::from_bits(bits),
    }
}

/// The boss music the last practice fight plays.
pub fn practice_boss_music(alternative: bool, expert: bool) -> Track {
    let (name, bits) = match (alternative, expert) {
        (false, false) => ("Msc_ArenaBossN", 0x3e00_0000),
        (false, true) => ("Msc_ArenaBossE", 0x3e00_0000),
        (true, false) => ("Msc_ArenaBsN_DnB", 0x3e19_999a),
        (true, true) => ("Msc_ArenaBsE_DnB", 0x3e19_999a),
    };
    Track {
        name,
        volume: f32::from_bits(bits),
    }
}

/// The menu theme and its volume: the plain theme at 0.1, the alternative
/// at 0.2 ([`exa_ui::menu::MENU_THEME`] and its drum-and-bass twin).
pub fn menu_theme(alternative: bool) -> Track {
    let (name, volume) = if alternative {
        exa_ui::menu::MENU_THEME_DNB
    } else {
        exa_ui::menu::MENU_THEME
    };
    Track { name, volume }
}

/// The two save managers the game creates at boot: the mode, its player-data
/// signature, its data version and the package magic it writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SaveManager {
    pub mode: Mode,
    pub signature: u32,
    pub data_version: Option<u32>,
    /// The package magic every manager writes.
    pub package_magic: u32,
}

/// The save managers of the story and the arena.
pub const SAVE_MANAGERS: [SaveManager; 2] = [
    SaveManager {
        mode: Mode::Story,
        signature: save_package::STORY_SIGNATURE,
        data_version: None,
        package_magic: save_package::PACKAGE_MAGIC,
    },
    SaveManager {
        mode: Mode::Arena,
        signature: save_package::ARENA_SIGNATURE,
        data_version: Some(save_package::ARENA_DATA_VERSION),
        package_magic: save_package::PACKAGE_MAGIC,
    },
];

/// A player's body state, as the flow reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerReading {
    pub condition: u32,
    pub health: f32,
}

/// A level exit requested by a trigger, a teleporter or a script.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LevelExit {
    /// The campaign level's number, 1-based.
    pub level: u32,
    /// The spawn point in the new level.
    pub spawn: u32,
    /// Bit 0 asks for the fade and the AI hand-over.
    pub flags: u32,
}

/// The level-exit save rule's inputs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CheckpointRule {
    pub level: u32,
    /// The `CheckPoints` setting.
    pub setting: bool,
    /// The save already holds `exanima04` or the level the exit leads to (no
    /// new checkpoint is made then).
    pub already_saved: bool,
}

/// What a level exit writes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitSave {
    /// The session is saved (the player's state gathered, the save written
    /// without the death rule) and the written save is kept as the
    /// checkpoint; on level 6 the level's time is stamped.
    Checkpoint,
    /// The session is saved, without a checkpoint.
    Save,
}

/// The level-exit save rule: every exit saves. Levels below 7 make a
/// checkpoint when the `CheckPoints` setting is on (level 6 always asks for
/// one) and the save holds neither `exanima04` nor the level yet; every
/// other exit saves plainly.
pub fn exit_save(rule: CheckpointRule) -> ExitSave {
    let wanted = rule.level < 7 && (rule.setting || rule.level == 6);
    if wanted && !rule.already_saved {
        ExitSave::Checkpoint
    } else {
        ExitSave::Save
    }
}

/// The campaign level's locale, 1-based, or `None` for a number the locale
/// table does not name (the exit then does nothing).
pub fn level_locale(level: u32) -> Option<&'static str> {
    exa_script::story::CAMPAIGN
        .get(usize::try_from(level).ok()?.checked_sub(1)?)
        .copied()
}

/// The reconstruction state that can be rebuilt (a reconstruction device was
/// used).
pub const REBUILDABLE: u8 = 1;
/// The reconstruction state after a strip.
pub const STRIPPED: u8 = 2;

/// The flow's whole state.
#[derive(Clone, Debug, Resource)]
pub struct Flow {
    pub mode: Mode,
    pub phase: Phase,
    /// The phase the transition screen was opened over.
    pub saved_phase: Phase,
    /// The main menu's form is visible.
    pub menu_open: bool,
    pub hook: Hook,
    transition: Option<Transition>,
    pub story: StoryState,
    pub practice: PracticeState,
    pub unlocks: Unlocks,
    /// The story's save slot, when it has one.
    pub slot: Option<u32>,
    /// The story and arena saves exist.
    pub has_save: [bool; 2],
    /// The versus lobby is available.
    pub versus: bool,
    /// The end-of-session menu shown, if any.
    pub end_menu: Option<EndMenu>,
    /// A player is alive in each game mode (practice, story, arena), the
    /// original's three player pointers.
    pub player: [bool; 3],
    /// The boot has run.
    booted: bool,
    /// The alternative music setting (the DnB tracks).
    pub alternative_music: bool,
    /// The flow's own tick counter.
    frames: u64,
    /// The red health the next practice fight heals (the host keeps the
    /// player's health; this is the copy the ladder reads).
    practice_health: f32,
    /// The maximum health the practice heal uses.
    practice_max: f32,
    /// The client size the camera shots are framed for.
    pub viewport: Viewport,
    /// Another unit installed a story hook (the story tick calls it).
    pub custom_hook: bool,
    /// The fifth level's respawn portal, as the host found it when the level
    /// loaded.
    pub respawn_portal: Option<Placement>,
}

impl Default for Flow {
    fn default() -> Self {
        Self {
            mode: Mode::Menu,
            phase: Phase::Menu,
            saved_phase: Phase::Menu,
            menu_open: false,
            hook: Hook::None,
            transition: None,
            story: StoryState {
                knockout_ms: u32::MAX - KNOCKOUT_MS,
                ..Default::default()
            },
            practice: PracticeState::default(),
            unlocks: Unlocks::default(),
            slot: None,
            has_save: [false; 2],
            versus: false,
            end_menu: None,
            player: [false; 3],
            booted: false,
            alternative_music: false,
            frames: 0,
            practice_health: 0.0,
            practice_max: 0.0,
            viewport: Viewport {
                width: 1600,
                height: 900,
            },
            custom_hook: false,
            respawn_portal: None,
        }
    }
}

/// The index of a mode's player slot.
fn player_slot(mode: Mode) -> Option<usize> {
    match mode {
        Mode::Practice => Some(0),
        Mode::Story => Some(1),
        Mode::Arena => Some(2),
        _ => None,
    }
}

impl Flow {
    /// The page state the menu table reads.
    pub fn page_state(&self) -> PageState {
        PageState {
            has_save: match self.mode {
                Mode::Story => self.has_save[0],
                Mode::Arena => self.has_save[1],
                _ => self.has_save[0] || self.has_save[1],
            },
            in_game: self.mode.is_game(),
            arena: self.versus,
            expert: self.unlocks.expert,
        }
    }

    /// The page the main menu opens: the in-game page in a session, else the
    /// title.
    pub fn menu_page(&self) -> Page {
        if self.mode.is_game() {
            Page::InGame
        } else {
            Page::Title
        }
    }

    /// The start-up sequence, once. The original creates the game layer,
    /// reads the settings, makes the two save managers, creates the
    /// transition screen, registers the script hooks, shows the splash and
    /// builds the title page.
    pub fn boot(&mut self) -> Vec<Step> {
        if self.booted {
            return Vec::new();
        }
        self.booted = true;
        let mut steps = vec![
            Step::Boot(BootTask::FirstRun),
            Step::Boot(BootTask::Window),
            Step::Boot(BootTask::ActorLibrary),
            Step::Boot(BootTask::GameGui),
            Step::Boot(BootTask::ItemGui),
            Step::Boot(BootTask::HardwareConfig),
            Step::Boot(BootTask::Settings),
            Step::Boot(BootTask::SaveManager(Mode::Story)),
            Step::Boot(BootTask::SaveManager(Mode::Arena)),
            Step::Boot(BootTask::ArenaData),
            Step::Boot(BootTask::Transition),
            Step::Boot(BootTask::Scripts),
            Step::Boot(BootTask::Audio),
            Step::Boot(BootTask::Splash),
            Step::Boot(BootTask::MainMenu),
        ];
        self.mode = Mode::Menu;
        self.phase = Phase::Menu;
        self.hook = Hook::None;
        self.menu_open = true;
        // The shipped build's first-run switch sets the second end-page tip.
        self.practice.next_tip = true;
        steps.push(Step::ShowMenu(Page::Title));
        steps.push(Step::SessionTask(false));
        steps
    }

    /// Switches the game mode. A switch to the running mode does nothing.
    /// Otherwise the reconstruction state and its checkpoint record clear,
    /// the outgoing mode's player is stored and the incoming mode's
    /// restored, and the incoming mode's hook is set: the story's hook for
    /// story, none for the arena, the title and a scripted mode (whose enter
    /// hook runs), and practice keeps the hook it had (the practice start
    /// installs its own). Entering practice from outside the practice family
    /// also restarts the round clock and forgets the save slot.
    ///
    /// The original's per-mode flag words are kept as the mode properties
    /// they describe: story mode uses checkpoints, arena mode overrides the
    /// inventory and writes its own tables. That reading is a guess from the
    /// notes; the mode words, the player slots and the order are exact.
    pub fn switch_mode(&mut self, mode: Mode, now: u32) -> Vec<Step> {
        if mode == self.mode {
            return Vec::new();
        }
        let mut steps = Vec::new();
        self.story.reconstruction = 0;
        self.story.checkpoint = 0;
        let was_practice = self.mode.practice_family();
        if let Some(slot) = player_slot(self.mode) {
            self.player[slot] = true;
        }
        self.mode = mode;
        self.end_menu = None;
        match mode {
            Mode::Practice => {
                if !was_practice {
                    self.practice.start_ms = now;
                    self.slot = None;
                }
            }
            Mode::Story => {
                self.hook = Hook::Story;
                steps.push(Step::Hook(Hook::Story));
            }
            Mode::Arena | Mode::Menu => {
                self.hook = Hook::None;
                steps.push(Step::Hook(Hook::None));
            }
            Mode::Other(word) => {
                self.hook = Hook::None;
                steps.push(Step::Hook(Hook::None));
                steps.push(Step::ScriptMode {
                    mode: word,
                    hook: ScriptHook::Enter,
                });
            }
        }
        steps
    }

    /// Starts the story from the character creator: the background's spawn
    /// pose, roles, light and companion, the camera on the player, and the
    /// story hook.
    pub fn start_story(
        &mut self,
        background: Background,
        body_height: f32,
        viewport: Viewport,
        now: u32,
    ) -> Vec<Step> {
        let start = story_start(background, body_height);
        self.mode = Mode::Story;
        self.hook = Hook::Story;
        self.phase = Phase::Playing;
        self.viewport = viewport;
        self.story = StoryState {
            knockout_ms: u32::MAX - KNOCKOUT_MS,
            ..Default::default()
        };
        // The three backgrounds start a different wake-up: the unknown one
        // plays its animation, the villager gets up, and the knight and
        // proctor wait for a scripted motion. All three start three seconds
        // after the spawn.
        let wake_start = now.wrapping_add(WAKE_LEAD_MS);
        let motion = match background {
            Background::Unknown => {
                self.story.wake = Wake {
                    animating: true,
                    start_ms: wake_start,
                    ..Default::default()
                };
                START_MOTION_ANIMATION
            }
            Background::Villager => {
                self.story.wake = Wake {
                    getting_up: true,
                    start_ms: wake_start,
                    ..Default::default()
                };
                START_MOTION_VILLAGER
            }
            Background::Knight | Background::Proctor => {
                self.story.wake = Wake {
                    delayed: true,
                    start_ms: wake_start,
                    ..Default::default()
                };
                START_MOTION_WAKING
            }
        };
        let mut steps = vec![
            Step::SessionTask(true),
            Step::LoadLevel {
                locale: "exanima01",
                spawn: 0,
                teardown: true,
            },
            Step::StoryPlayer {
                role: start.role,
                flags: start.flags,
            },
            Step::PlacePlayer(start.placement),
        ];
        if let Some(light) = start.light {
            steps.push(Step::StoryLight(light));
        }
        steps.push(Step::RequestAction(motion));
        steps.push(Step::Camera(camera::frame_player(
            start.placement.position,
            viewport,
        )));
        if let Some(companion) = start.companion {
            steps.push(Step::Companion(companion));
        }
        steps.push(Step::Hook(Hook::Story));
        steps
    }

    /// Starts the practice ladder at its first fight. The arena is `arena`
    /// (or `arenae` for the expert ladder); the partner is `opponent01` (or
    /// `opponent09`), the music is the ladder's, the camera the fast near shot and
    /// the player stands at the ring's middle, facing the camera.
    pub fn start_practice(
        &mut self,
        ladder: Ladder,
        health: f32,
        viewport: Viewport,
        now: u32,
    ) -> Vec<Step> {
        self.mode = Mode::Practice;
        self.phase = Phase::Playing;
        self.hook = Hook::Practice;
        self.practice = PracticeState {
            ladder,
            fight: FIRST_FIGHT,
            start_ms: now,
            ..Default::default()
        };
        let step = practice::next_practice(0, ladder, health, health);
        let start = PracticeStart {
            ladder,
            fight: FIRST_FIGHT,
            partner: step.name,
            boss: false,
            locale: match ladder {
                Ladder::Novice => "arena",
                Ladder::Expert => "arenae",
            },
            music: practice_music(ladder, self.alternative_music),
        };
        let placement = Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(PRACTICE_POINT.x, practice_height(health), PRACTICE_POINT.z),
        };
        self.viewport = viewport;
        vec![
            Step::TeardownWorld,
            Step::GameGui(false),
            Step::SessionTask(true),
            Step::LoadLevel {
                locale: start.locale,
                spawn: 0,
                teardown: true,
            },
            Step::PlacePlayer(placement),
            Step::Camera(camera::frame_point(PRACTICE_POINT, true, false, viewport)),
            Step::Music {
                name: start.music.name,
                volume: start.music.volume,
                restart: true,
            },
            Step::Hook(Hook::Practice),
        ]
    }

    /// Continues a story save: the save's locale is loaded (or the first
    /// level when there is none), the player nudged out of any exit trigger
    /// by the host, and the near shot framed on the player's body, fast.
    pub fn continue_story(
        &mut self,
        save_locale: Option<&'static str>,
        player: Placement,
        viewport: Viewport,
    ) -> Vec<Step> {
        self.mode = Mode::Story;
        self.hook = Hook::Story;
        self.phase = Phase::Playing;
        self.viewport = viewport;
        vec![
            Step::SessionTask(true),
            Step::LoadLevel {
                locale: save_locale.unwrap_or("exanima01"),
                spawn: 0,
                teardown: true,
            },
            Step::PlacePlayer(player),
            Step::Camera(camera::frame_point(player.position, true, false, viewport)),
            Step::Hook(Hook::Story),
        ]
    }

    /// Continues the arena hub: the arena save (already loaded by the caller)
    /// on the hub's single locale, the hub's music, and the camera on the
    /// manager. The hub's music draw takes the caller's roll.
    pub fn continue_arena(
        &mut self,
        player: Placement,
        viewport: Viewport,
        roll: &mut u32,
    ) -> Vec<Step> {
        self.mode = Mode::Arena;
        self.hook = Hook::None;
        self.phase = Phase::Playing;
        self.viewport = viewport;
        let music = arena_hub::pick_hub_music(roll, self.alternative_music);
        vec![
            Step::SessionTask(true),
            Step::TeardownWorld,
            Step::LoadLevel {
                locale: "arenahub",
                spawn: 0,
                teardown: false,
            },
            Step::PlacePlayer(player),
            Step::Camera(camera::frame_point(player.position, true, false, viewport)),
            Step::Music {
                name: music.track.name,
                volume: music.track.volume,
                restart: true,
            },
            Step::ShowMenu(Page::InGame),
            Step::Hook(Hook::None),
            Step::SessionTask(true),
        ]
    }

    /// The original's session begin: the frame task and the quit/autosave
    /// hook are registered, the phase becomes playing and the menu closes.
    pub fn begin(&mut self) -> Vec<Step> {
        self.phase = Phase::Playing;
        self.menu_open = false;
        vec![Step::SessionTask(true), Step::Controls(true)]
    }

    /// The pause: the in-game menu is up and the phase records it. The
    /// original's pause word is only read by its autosave; the game clock's
    /// pause is the frame loop's.
    pub fn pause(&mut self) -> Vec<Step> {
        if self.phase == Phase::Playing {
            self.saved_phase = self.phase;
            self.phase = Phase::Paused;
            self.menu_open = true;
            return vec![Step::ShowMenu(Page::InGame), Step::Controls(false)];
        }
        Vec::new()
    }

    /// The resume: the menu closes and the saved phase comes back.
    pub fn resume(&mut self) -> Vec<Step> {
        if self.phase == Phase::Paused {
            self.phase = self.saved_phase;
            self.menu_open = false;
            return vec![Step::HideMenu, Step::Controls(true)];
        }
        Vec::new()
    }

    /// Opens the transition screen over `action`; it runs after
    /// [`TRANSITION_MS`]. The phase before the transition is kept.
    pub fn transition(&mut self, action: Action, now: u32) -> Vec<Step> {
        self.saved_phase = self.phase;
        self.phase = Phase::Transition;
        self.transition = Some(Transition {
            action,
            at_ms: now.wrapping_add(TRANSITION_MS),
        });
        vec![Step::Transition]
    }

    /// Runs the transition's action when its time has come, or `None`.
    pub fn transition_due(&mut self, now: u32) -> Option<Vec<Step>> {
        let transition = self.transition?;
        if u64::from(now) < u64::from(transition.at_ms) {
            return None;
        }
        self.transition = None;
        Some(self.apply(transition.action, now))
    }

    fn apply(&mut self, action: Action, now: u32) -> Vec<Step> {
        match action {
            Action::QuitToMenu => self.quit_to_menu(),
            Action::StoryRespawn => self.story_respawn(now),
            Action::Revive => self.revive(now),
            Action::NextPracticeFight => self.practice_next(now),
            Action::Locale { locale, spawn } => {
                self.phase = Phase::Playing;
                vec![Step::LoadLevel {
                    locale,
                    spawn,
                    teardown: true,
                }]
            }
        }
    }

    /// The frame's whole game work, in the original's order: the dialogue
    /// engine and then the session hook, but only while no main menu covers
    /// the game and no transition is running.
    pub fn frame(&mut self, now: u32, player: Option<PlayerReading>, others: &[u32]) -> Vec<Step> {
        self.frames = self.frames.wrapping_add(1);
        if self.menu_open {
            return Vec::new();
        }
        if self.phase == Phase::Transition {
            return self.transition_due(now).unwrap_or_default();
        }
        if self.phase != Phase::Playing {
            return Vec::new();
        }
        let mut steps = vec![Step::Dialogue];
        match self.hook {
            Hook::Story => steps.extend(self.story_tick(now, player)),
            Hook::Knockout => steps.extend(self.knockout_tick(now, player)),
            Hook::Practice => steps.extend(self.practice_tick(now, player, others)),
            Hook::None => {}
        }
        steps
    }

    /// The story's per-frame hook: the wake-up flags, the once-a-second
    /// discovery pass, and the knocked-out or dead player.
    pub fn story_tick(&mut self, now: u32, player: Option<PlayerReading>) -> Vec<Step> {
        if self.phase != Phase::Playing {
            return Vec::new();
        }
        let Some(player) = player else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        if player.condition & DYING != 0 {
            if player.condition == DEAD {
                if f64::from(player.health) < ALIVE_HEALTH {
                    return self.die(player.health);
                }
                return self.knock_out(now);
            }
            return steps;
        }
        let wake = &mut self.story.wake;
        if wake.animating {
            if player.condition & WAKING == 0 {
                wake.animating = false;
            }
        } else if wake.getting_up {
            if u64::from(wake.start_ms) + u64::from(GET_UP_MS) < u64::from(now) {
                steps.push(Step::Wake(WakePose {
                    request: None,
                    condition: 0,
                    delay_ms: 0,
                    turn: 0.4,
                    move_factor: 0.4,
                    flags: 0x20,
                }));
                wake.getting_up = false;
            } else {
                steps.push(Step::Wake(WakePose {
                    request: None,
                    condition: 0,
                    delay_ms: 0,
                    turn: 0.49,
                    move_factor: 0.49,
                    flags: 0x600,
                }));
            }
        } else if wake.delayed
            && u64::from(wake.start_ms.wrapping_sub(WAKE_LEAD_MS)) < u64::from(now)
        {
            steps.push(Step::RequestAction(0x20));
            wake.delayed = false;
        }
        let second = now / 1000;
        if second > self.story.last_discovery_s {
            self.story.last_discovery_s = second;
            steps.push(Step::Discovery);
        }
        if self.custom_hook {
            steps.push(Step::CustomHook);
        }
        steps
    }

    /// The practice round's per-frame hook: after the guard second, the first
    /// dead opponent ends the round; the last fight wins the ladder.
    pub fn practice_tick(
        &mut self,
        now: u32,
        player: Option<PlayerReading>,
        others: &[u32],
    ) -> Vec<Step> {
        let player_condition = player.map_or(0, |p| p.condition);
        match practice_round(
            self.phase,
            now,
            self.practice.start_ms,
            player_condition,
            others,
            self.practice.fight,
        ) {
            Some(RoundEnd::None) | None => Vec::new(),
            Some(RoundEnd::PlayerDead) => self.die(player.map_or(0.0, |p| p.health)),
            Some(RoundEnd::Last) => {
                self.phase = Phase::Won;
                self.hook = Hook::None;
                let menu = end_menu(MENU_WON, self.mode);
                self.end_menu = Some(menu);
                // The novice ladder unlocks expert and shows its tip; the
                // expert ladder shows the other one.
                let mut steps = Vec::new();
                if self.practice.ladder == Ladder::Novice {
                    steps.push(Step::Tip(5));
                    if self.mode == Mode::Practice {
                        self.unlocks.expert = true;
                    }
                } else {
                    steps.push(Step::Tip(6));
                }
                steps.push(Step::EndMenu(menu));
                steps.push(Step::Hook(Hook::None));
                steps
            }
            Some(RoundEnd::Next) => {
                self.phase = Phase::RoundWon;
                self.hook = Hook::None;
                let menu = end_menu(MENU_NEXT, self.mode);
                self.end_menu = Some(menu);
                let tip = if self.practice.next_tip { 1 } else { 0 };
                self.practice.next_tip = false;
                vec![Step::Tip(tip), Step::EndMenu(menu), Step::Hook(Hook::None)]
            }
        }
    }

    /// The practice ladder's next fight: the count advances, health gains a
    /// tenth of the maximum, the partner name and boss music change, and the
    /// round starts again.
    pub fn practice_next(&mut self, now: u32) -> Vec<Step> {
        let step = practice::next_practice(
            self.practice.fight,
            self.practice.ladder,
            self.practice_health,
            self.practice_max,
        );
        self.practice.fight = step.fight;
        self.practice.start_ms = now;
        self.phase = Phase::Playing;
        self.hook = Hook::Practice;
        self.end_menu = None;
        let red = f32::from_bits(step.red_bits);
        let placement = Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(PRACTICE_POINT.x, practice_height(red), PRACTICE_POINT.z),
        };
        let mut steps = vec![
            Step::HideEndMenu,
            Step::MenuHelp(false),
            Step::Health {
                red_bits: step.red_bits,
                yellow_bits: step.yellow_bits,
            },
            Step::PlacePlayer(placement),
            Step::Camera(camera::frame_point(
                PRACTICE_POINT,
                false,
                false,
                self.viewport,
            )),
            Step::Hook(Hook::Practice),
            Step::SessionTask(true),
        ];
        if step.boss {
            let music = practice_boss_music(
                self.alternative_music,
                self.practice.ladder == Ladder::Expert,
            );
            steps.push(Step::Music {
                name: music.name,
                volume: music.volume,
                restart: true,
            });
        }
        steps
    }

    /// The player's death: the end menu, the story save's fate and, for a
    /// rebuilt player, the strip. The end menu is the reconstructed one when
    /// the player can be rebuilt.
    pub fn die(&mut self, health: f32) -> Vec<Step> {
        self.phase = Phase::Ended;
        self.saved_phase = Phase::Ended;
        self.hook = Hook::None;
        let mut steps = vec![
            Step::GameGui(false),
            Step::Controls(false),
            Step::Hook(Hook::None),
        ];
        let alive = f64::from(health) >= ALIVE_HEALTH;
        steps.push(Step::Tip(if alive { 2 } else { 3 }));
        let mut reconstructed = false;
        if !alive && self.mode == Mode::Story {
            if self.story.reconstruction == 0 || self.story.checkpoint != self.story.record {
                steps.push(Step::DropSave);
            } else {
                if self.story.reconstruction == REBUILDABLE {
                    steps.push(Step::StripPlayer);
                }
                self.story.reconstruction = STRIPPED;
                reconstructed = true;
            }
        }
        let kind = if reconstructed {
            MENU_RECONSTRUCTED
        } else {
            MENU_WON
        };
        let menu = end_menu(kind, self.mode);
        self.end_menu = Some(menu);
        steps.push(Step::EndMenu(menu));
        steps
    }

    /// The story player is knocked out: the lens and controls go, the first
    /// tip shows, and the five-second revival check starts.
    pub fn knock_out(&mut self, now: u32) -> Vec<Step> {
        self.hook = Hook::Knockout;
        self.story.knockout_ms = now;
        vec![Step::GameGui(false), Step::Controls(false), Step::Tip(2)]
    }

    /// The knocked-out player's check: five seconds after the knock-out a
    /// living player is revived behind the transition, a dead one dies.
    pub fn knockout_tick(&mut self, now: u32, player: Option<PlayerReading>) -> Vec<Step> {
        let Some(player) = player else {
            return Vec::new();
        };
        if f64::from(player.health) < ALIVE_HEALTH {
            return self.die(player.health);
        }
        if u64::from(self.story.knockout_ms) + u64::from(KNOCKOUT_MS) < u64::from(now) {
            self.story.knockout_ms = u32::MAX - KNOCKOUT_MS;
            self.transition(Action::Revive, now)
        } else {
            Vec::new()
        }
    }

    /// The revived story player: the lens returns, the body gets its waking
    /// flag, a two-second delay and the wake-up; the story hook resumes.
    pub fn revive(&mut self, now: u32) -> Vec<Step> {
        self.phase = Phase::Playing;
        self.hook = Hook::Story;
        self.story.wake = Wake {
            animating: true,
            getting_up: false,
            delayed: false,
            start_ms: now.wrapping_add(WAKE_DELAY_MS),
        };
        vec![
            Step::Controls(true),
            Step::GameGui(true),
            Step::Wake(WakePose {
                request: None,
                condition: 0,
                delay_ms: WAKE_DELAY_MS as i32,
                turn: 0.0,
                move_factor: 0.0,
                flags: 0x40,
            }),
            Step::Hook(Hook::Story),
            Step::SessionTask(true),
        ]
    }

    /// The story's death screen CONTINUE: the save is written (without the
    /// death rule), the world torn down and `exanima05` loaded; the player
    /// stands at the level's respawn portal (its pose, or the identity at the
    /// origin when the level has none) 15 cm higher and plays the get-up
    /// motion. The camera is not moved.
    pub fn story_respawn(&mut self, now: u32) -> Vec<Step> {
        self.mode = Mode::Story;
        self.phase = Phase::Playing;
        self.hook = Hook::Story;
        self.story.wake = Wake::default();
        self.story.reconstruction = REBUILDABLE;
        let _ = now;
        let portal = self.respawn_portal.unwrap_or(Placement {
            rotation: NO_ROTATION,
            position: Vec3::ZERO,
        });
        let placement = Placement {
            rotation: portal.rotation,
            position: portal.position + Vec3::Y * 15.0,
        };
        vec![
            Step::Save {
                drop_on_low_health: false,
            },
            Step::TeardownWorld,
            Step::LoadLevel {
                locale: "exanima05",
                spawn: 0,
                teardown: true,
            },
            Step::PlacePlayer(placement),
            Step::RequestAction(0x2f),
            Step::Hook(Hook::Story),
        ]
    }

    /// The end-of-life story CONTINUE of a save with no record: the player
    /// data is written back, then the respawn runs behind the transition.
    pub fn continue_after_death(&mut self, now: u32) -> Vec<Step> {
        let mut steps = vec![Step::PlayerData];
        steps.extend(self.transition(Action::StoryRespawn, now));
        steps
    }

    /// The application's quit callback, which the session begin installs:
    /// in the paused phase the arena unit's pause callback runs (whatever
    /// the mode); while playing, the game's interface closes and the session
    /// autosaves: a story always with the death rule (a dead player's save
    /// is dropped), an arena from its in-game page without it, or a forfeit
    /// from its pause page; practice writes nothing. `page` is the main
    /// menu's page. Nothing returns to the title: the application is closing.
    pub fn quit_session(&mut self, page: Page) -> Vec<Step> {
        let mut steps = Vec::new();
        if self.phase == Phase::Paused {
            steps.push(Step::ArenaPaused);
        }
        if self.phase == Phase::Playing {
            steps.push(Step::GameGui(false));
            match self.mode {
                Mode::Story => steps.push(Step::Save {
                    drop_on_low_health: true,
                }),
                Mode::Arena => match page {
                    Page::InGame => steps.push(Step::Save {
                        drop_on_low_health: false,
                    }),
                    Page::ArenaPause => steps.push(Step::Forfeit),
                    _ => {}
                },
                _ => {}
            }
        }
        steps
    }

    /// Back to the title: leave the session's world, hide the game, switch to
    /// the menu mode, build the title page, play its theme and show it.
    pub fn quit_to_menu(&mut self) -> Vec<Step> {
        let mut steps = self.leave_session();
        self.mode = Mode::Menu;
        self.phase = Phase::Menu;
        self.saved_phase = Phase::Menu;
        self.hook = Hook::None;
        self.end_menu = None;
        self.menu_open = true;
        let theme = menu_theme(self.alternative_music);
        steps.push(Step::Music {
            name: theme.name,
            volume: theme.volume,
            restart: true,
        });
        steps.push(Step::ShowMenu(Page::Title));
        steps.push(Step::SessionTask(false));
        steps
    }

    /// Leaving a session: the hook and the frame task stop, the dialogue and
    /// interface go, the world is torn down and the player pointers are
    /// cleared. A scripted mode's end hook is named.
    pub fn leave_session(&mut self) -> Vec<Step> {
        let mut steps = vec![
            Step::Hook(Hook::None),
            Step::SessionTask(false),
            Step::GameGui(false),
            Step::Dialogue,
        ];
        self.hook = Hook::None;
        match self.mode {
            Mode::Arena => steps.push(Step::ScriptMode {
                mode: ARENA_MODE,
                hook: ScriptHook::End,
            }),
            Mode::Other(word) if word & SCRIPT_MODE_BIT != 0 => steps.push(Step::ScriptMode {
                mode: word,
                hook: ScriptHook::End,
            }),
            _ => {}
        }
        steps.push(Step::TeardownWorld);
        self.player = [false; 3];
        steps
    }

    /// The story level change (a trigger, a portal or a script). Nothing
    /// happens during a transition or for a level the locale table does not
    /// name. Otherwise a player whose condition has neither dead bit loses
    /// the controls, the game view goes, the save rule runs (every exit
    /// saves; early levels also keep a checkpoint) and the new level loads at
    /// the exit's spawn point behind the transition. The fade bit's effects
    /// and AI hand-over and the locale's ambient music are the host's.
    pub fn level_exit(
        &mut self,
        exit: LevelExit,
        rule: CheckpointRule,
        player_condition: u32,
        now: u32,
    ) -> Vec<Step> {
        if self.phase == Phase::Transition {
            return Vec::new();
        }
        let Some(locale) = level_locale(exit.level) else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        if player_condition & DEAD == 0 {
            steps.push(Step::Controls(false));
        }
        steps.push(Step::GameGui(false));
        let checkpoint = exit_save(rule) == ExitSave::Checkpoint;
        steps.push(Step::Save {
            drop_on_low_health: false,
        });
        if checkpoint {
            steps.push(Step::Checkpoint);
        }
        steps.extend(self.transition(
            Action::Locale {
                locale,
                spawn: exit.spawn,
            },
            now,
        ));
        steps
    }

    /// The title's EXIT: leave whatever session runs and quit.
    pub fn quit_application(&mut self) -> Vec<Step> {
        let mut steps = if self.mode.is_game() {
            self.leave_session()
        } else {
            Vec::new()
        };
        steps.push(Step::Quit);
        steps
    }

    /// The named-locale exit: the original checks the locale exists before it
    /// transitions to its load. A missing locale does nothing.
    pub fn leave_to_locale(&mut self, locale: &'static str, exists: bool, now: u32) -> Vec<Step> {
        if !exists {
            return Vec::new();
        }
        self.transition(Action::Locale { locale, spawn: 0 }, now)
    }

    /// The class select's cancel: a scripted mode's end hook runs, then the
    /// session leaves to the title behind the transition.
    pub fn cancel_class_select(&mut self, now: u32) -> Vec<Step> {
        let mut steps = Vec::new();
        if let Mode::Other(word) = self.mode
            && word & SCRIPT_MODE_BIT != 0
        {
            steps.push(Step::ScriptMode {
                mode: word,
                hook: ScriptHook::End,
            });
        }
        steps.extend(self.transition(Action::QuitToMenu, now));
        steps
    }

    /// The class select's cancel form's final step: back to the title.
    pub fn cancel_to_menu(&mut self) -> Vec<Step> {
        self.quit_to_menu()
    }

    /// The splash's hand-over: the title page is built and shown, and the
    /// frame task is not running.
    pub fn after_splash(&mut self) -> Vec<Step> {
        self.mode = Mode::Menu;
        self.phase = Phase::Menu;
        self.menu_open = true;
        vec![Step::ShowMenu(Page::Title), Step::SessionTask(false)]
    }

    /// The help screen closed: the menu's help flag clears.
    pub fn help_closed(&mut self) -> Vec<Step> {
        vec![Step::MenuHelp(false)]
    }

    /// The menu's confirm key with a hovered row: the hover sound (a random
    /// gain of 0.4 to 0.8 at volume 0.06) and the row's confirmation. A menu
    /// that is not showing does nothing.
    pub fn menu_confirm(&mut self, menu_visible: bool, hovered: bool, roll: u32) -> Vec<Step> {
        if !menu_visible {
            return Vec::new();
        }
        let mut steps = Vec::new();
        if let Some(gain) = hover_sound(hovered, roll) {
            steps.push(Step::Hover {
                volume: HOVER_VOLUME,
                gain,
            });
        }
        steps.push(Step::MenuConfirm);
        steps
    }

    /// The GUI scale notification: when the quantised scale moved, both
    /// menus lay out again.
    pub fn gui_scale_changed(&self, previous: f32, height: i32) -> Vec<Step> {
        let scale = gui_scale(height);
        if scale.to_bits() == previous.to_bits() {
            Vec::new()
        } else {
            vec![Step::GuiScale(scale)]
        }
    }

    /// The versus lobby's availability and the alternative-music setting.
    pub fn set_versus(&mut self, versus: bool, alternative_music: bool) {
        self.versus = versus;
        self.alternative_music = alternative_music;
    }

    /// The boot has run.
    pub fn booted(&self) -> bool {
        self.booted
    }

    /// The flow's tick counter.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// The red health the next practice fight heals. The host keeps the
    /// player's health; this is the copy the ladder reads.
    pub fn set_practice_health(&mut self, red: f32, max: f32) {
        self.practice_health = red;
        self.practice_max = max;
    }

    /// The practice heal's red health.
    pub fn practice_health(&self) -> f32 {
        self.practice_health
    }
}

/// The GUI scale notification's value: the row scale times 0.5, which the
/// GUI support code compares before it re-lays out both menus.
pub fn gui_scale(height: i32) -> f32 {
    exa_ui::menu::menu_row_scale(height) * 0.5
}

/// The hover sound the menu plays when its hover id is set: a random gain of
/// 0.4 to 0.8 at volume 0.06. The sound's name is chosen by the caller (the
/// original plays the sound ID stored on the menu form).
pub fn hover_sound(row: bool, roll: u32) -> Option<f32> {
    if !row {
        return None;
    }
    let draw = (f64::from(roll) / 4_294_967_296.0) as f32;
    Some(draw * 0.4 + 0.4)
}

/// The volume every menu hover is played at.
pub const HOVER_VOLUME: f32 = 0.06;

/// The practice scene filter: a level's node stays unless its class is the
/// player's, the character database's or one of the locale's own trigger
/// classes. A node of item type 0x11 has its own list filtered the same way;
/// a node of the 0x4000 type is filtered, and every other node stays.
pub fn node_stays(
    item_type: u32,
    class: u32,
    locale_classes: &[u32],
    player_class: u32,
    character_class: u32,
) -> bool {
    let removed =
        class == player_class || class == character_class || locale_classes.contains(&class);
    if item_type & 0xfff == 0x11 {
        return !removed;
    }
    if item_type & 0x7000 == 0x4000 {
        return !removed;
    }
    true
}

/// An exit trigger as the nudge reads it: its centre and bounding radius,
/// its box's half extents and its turn (three rows; a local axis `i` is
/// row `i`), and whether its object is an exit marker.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExitTrigger {
    pub centre: Vec3,
    /// The trigger's bounding radius (the first reach check).
    pub radius: f32,
    pub extent: Vec3,
    pub rotation: [[f32; 3]; 3],
    /// The trigger's object is an exit marker (the object's 0x20 flag).
    pub exit_marker: bool,
}

/// The result of the exit-trigger nudge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExitNudge {
    pub position: Vec3,
    /// The local axis the player was pushed along (0 = x, 2 = z).
    pub axis: usize,
    /// How far along that axis's row, in centimetres.
    pub distance: f32,
}

/// What the nudge adds past the trigger's edge.
pub const EXIT_MARGIN: f32 = 75.0;
/// What the reach checks allow: added to the radius before it is squared,
/// and the largest squared box distance that still counts.
pub const EXIT_REACH: f32 = 50.0;

/// The original's exit-trigger nudge of the player at `player` (a continued
/// story's player standing in a level exit is moved out of it), or `None`
/// when the trigger does not hold the player.
///
/// The trigger holds the player when the squared distance to its centre
/// (x² + y², then + z²) is at most `(radius + 50)²`, the squared distance to
/// its box (each axis's overhang squared, summed from 0) is at most 50 — the
/// original compares the squared value with the plain 50 — and its object is
/// an exit marker; a NaN passes both reach checks. The offset from the
/// centre, its height zeroed, is turned into the trigger's frame
/// (`local_i = (x·r_i0 + y·r_i1) + z·r_i2`), and the axis with the smaller
/// clearance (`extent − |local|`; z only when strictly smaller) is taken.
/// The player moves along that axis's row by `(extent − local) + 75` when
/// the local offset is positive, else by the negated amount: the original's
/// arithmetic, which for a negative offset pushes `extent + |local| + 75`
/// toward the negative side.
pub fn exit_nudge(player: Vec3, trigger: ExitTrigger) -> Option<ExitNudge> {
    let d = player - trigger.centre;
    let squared = (d.x * d.x + d.y * d.y) + d.z * d.z;
    let reach = trigger.radius + EXIT_REACH;
    if reach * reach < squared {
        return None;
    }
    if box_distance_squared(d, trigger.rotation, trigger.extent) > EXIT_REACH {
        return None;
    }
    if !trigger.exit_marker {
        return None;
    }
    let local = turn_into(Vec3::new(d.x, 0.0, d.z), trigger.rotation);
    let clearance_x = trigger.extent.x - local.x.abs();
    let clearance_z = trigger.extent.z - local.z.abs();
    let axis = if clearance_z < clearance_x { 2 } else { 0 };
    let value = local[axis];
    let push = (trigger.extent[axis] - value) + EXIT_MARGIN;
    let distance = if value > 0.0 { push } else { -push };
    let row = Vec3::from_array(trigger.rotation[axis]);
    Some(ExitNudge {
        position: player + row * distance,
        axis,
        distance,
    })
}

/// The squared distance from an offset to a trigger's box: the offset turned
/// into the box's frame, made absolute, and each axis whose bits exceed the
/// half extent's (an integer compare, as the original's) adds its overhang
/// squared, from 0 in x, y, z order. The same rule as
/// [`exa_physics::sector::mesh_box_distance2`], on the trigger's row turn.
fn box_distance_squared(offset: Vec3, rotation: [[f32; 3]; 3], extent: Vec3) -> f32 {
    let local = turn_into(offset, rotation).abs();
    let mut sum = 0.0;
    for k in 0..3 {
        if (extent[k].to_bits() as i32) < (local[k].to_bits() as i32) {
            let over = local[k] - extent[k];
            sum += over * over;
        }
    }
    sum
}

/// An offset in a trigger's frame: each local axis is the offset's dot
/// product with that row, summed `(x + y) + z`.
fn turn_into(offset: Vec3, rotation: [[f32; 3]; 3]) -> Vec3 {
    let dot = |row: [f32; 3]| (offset.x * row[0] + offset.y * row[1]) + offset.z * row[2];
    Vec3::new(dot(rotation[0]), dot(rotation[1]), dot(rotation[2]))
}

/// The flow's Bevy plugin: the flow runs on the game clock, follows the
/// player into a play level and turns a teleporter journey into a level
/// exit.
pub struct FlowPlugin;

impl Plugin for FlowPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Flow>()
            .add_systems(Startup, boot)
            .add_systems(Update, (tick, follow_player, travel).chain());
    }
}

fn boot(mut flow: ResMut<Flow>) {
    for step in flow.boot() {
        debug!("game flow: {step:?}");
    }
}

/// The game clock the flow runs on is the fixed step's (12 ms a tick); the
/// player and the other characters' conditions come from the running world.
fn tick(
    mut flow: ResMut<Flow>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    players: Query<
        (Option<&exa_combat::Condition>, Option<&exa_combat::Fighter>),
        With<crate::player::PlayerBody>,
    >,
    others: Query<&exa_combat::Condition, Without<crate::player::PlayerBody>>,
) {
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    let player = players
        .single()
        .ok()
        .map(|(condition, fighter)| PlayerReading {
            condition: condition.map_or(0, |condition| condition.0),
            // A body without a fighter record is treated as unhurt.
            health: fighter.map_or(0.25, |fighter| fighter.health.red),
        });
    let others: Vec<u32> = others.iter().map(|condition| condition.0).collect();
    for step in flow.frame(now, player, &others) {
        trace!("game flow: {step:?}");
    }
}

/// A play level's player starts the practice hook (the sandbox is Exarust's
/// stand-in for a practice session; the debug views have no session flow of
/// the original's).
fn follow_player(
    mut flow: ResMut<Flow>,
    players: Query<(), Added<crate::player::PlayerBody>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
) {
    if players.is_empty() || flow.mode.is_game() {
        return;
    }
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    for step in flow.switch_mode(Mode::Practice, now) {
        debug!("game flow: {step:?}");
    }
    flow.menu_open = false;
    flow.phase = Phase::Playing;
    flow.hook = Hook::Practice;
    flow.practice.start_ms = now;
    debug!("game flow: a play level is running; the practice hook watches it");
}

/// A teleporter journey is the game's level exit.
fn travel(
    mut flow: ResMut<Flow>,
    mut journeys: MessageReader<crate::destination::TravelToLevel>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
) {
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    for journey in journeys.read() {
        let steps = flow.level_exit(
            LevelExit {
                level: journey.level,
                spawn: journey.arrival,
                flags: 1,
            },
            CheckpointRule {
                level: journey.level,
                setting: true,
                already_saved: false,
            },
            0,
            now,
        );
        for step in steps {
            info!(
                "game flow: {} asks for level {} (spawn {}): {step:?}",
                journey.user, journey.level, journey.arrival
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mode_and_phase_words_round_trip() {
        for mode in [
            Mode::Menu,
            Mode::Practice,
            Mode::Story,
            Mode::Arena,
            Mode::Other(VERSUS_MODE),
        ] {
            assert_eq!(Mode::from_word(mode.word()), mode);
        }
        assert_eq!(Mode::from_word(1), Mode::Other(1));
        assert!(!Mode::Arena.practice_family());
        assert!(Mode::Practice.practice_family());
        assert!(!Mode::Story.practice_family());
        assert!(Mode::Story.uses_checkpoints());
        assert!(!Mode::Arena.uses_checkpoints());
        for phase in [
            Phase::Transition,
            Phase::Menu,
            Phase::Playing,
            Phase::Won,
            Phase::Ended,
            Phase::RoundWon,
            Phase::Forfeit,
            Phase::Paused,
        ] {
            assert_eq!(Phase::from_word(phase.word()), phase);
        }
    }

    #[test]
    fn the_camera_shots_and_their_cursor_warps() {
        let view = Viewport {
            width: 1600,
            height: 1140,
        };
        // The offset is added to the point: (300, 1180, 1300) + (0, 0, 500).
        let shot = camera::frame_point(PRACTICE_POINT, true, true, view);
        assert_eq!(shot.camera, Vec3::new(300.0, 1180.0, 1800.0));
        assert_eq!(shot.target, Vec3::new(0.0, 0.0, 500.0));
        assert_eq!(shot.speed, Some(1300.0));
        assert_eq!(shot.warp, (920, 370));
        let near = camera::frame_point(PRACTICE_POINT, false, false, view);
        assert_eq!(near.camera, Vec3::new(700.0, 1180.0, 1600.0));
        assert_eq!(near.speed, None);
        let two = camera::frame_two_points(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0), true, view);
        assert_eq!(two.camera, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(two.target, Vec3::ZERO);
        let player = camera::frame_player(Vec3::new(10.0, 20.0, 30.0), view);
        assert_eq!(player.camera, Vec3::new(610.0, 620.0, 1330.0));
        assert_eq!(player.target, Vec3::new(10.0, 20.0, 30.0));
        assert_eq!(player.warp, (600, 570));
        // 200·(900/1140) = 157.89, which rounds to 158.
        assert_eq!(camera::warp(1600, 900, 0.0, 200.0), (800, 292));
    }

    #[test]
    fn the_spawn_rotation_is_the_engines_row_layout() {
        // A quarter turn of yaw: rows (cy, 0, −sy), (0, 1, 0), (sy, 0, cy).
        let quarter = yaw(90.0);
        assert!((quarter[0][2] + 1.0).abs() < 1e-6);
        assert!((quarter[2][0] - 1.0).abs() < 1e-6);
        assert!(quarter[0][0].abs() < 1e-6);
        assert_eq!(quarter[1], [0.0, 1.0, 0.0]);
        // A roll alone: rows (cz, sz, 0), (−sz, cz, 0), (0, 0, 1).
        let roll = rotation(0.0, 0.0, 90.0);
        assert!((roll[0][1] - 1.0).abs() < 1e-6);
        assert!((roll[1][0] + 1.0).abs() < 1e-6);
        assert_eq!(roll[2], [0.0, 0.0, 1.0]);
        // A pitch alone: rows (1, 0, 0), (0, cx, sx), (0, −sx, cx).
        let pitch = rotation(90.0, 0.0, 0.0);
        assert_eq!(pitch[0], [1.0, 0.0, 0.0]);
        assert!((pitch[1][2] - 1.0).abs() < 1e-6);
        assert!((pitch[2][1] + 1.0).abs() < 1e-6);
        assert_eq!(rotation(0.0, 0.0, 0.0), NO_ROTATION);
    }

    #[test]
    fn a_new_story_spawns_by_background() {
        let unknown = story_start(Background::Unknown, 1.0);
        assert_eq!(unknown.role, 0x0e);
        assert_eq!(unknown.flags, 0x200);
        assert_eq!(unknown.placement.position.x, 8100.0);
        assert_eq!(unknown.placement.position.y, 30.0);
        assert_eq!(unknown.placement.position.z, 3920.0);
        assert_eq!(unknown.placement.rotation, yaw(150.0));
        let light = unknown.light.unwrap();
        assert_eq!(light.position, Vec3::new(8045.0, 4.7, 4075.0));
        assert_eq!(light.rotation_degrees, Vec3::new(-48.0, 110.0, 88.0));
        assert_eq!(light.intensity, 256.0);
        let derrin = unknown.companion.unwrap();
        assert_eq!(derrin.template, "xac_derrin");
        assert_eq!(derrin.position, Vec3::new(5808.0, 0.0, -4075.0));
        assert_eq!(derrin.facing, -90.0);
        assert_eq!(derrin.flags, 0x500);

        let knight = story_start(Background::Knight, 1.0);
        assert_eq!(knight.role, 0x1b);
        assert_eq!(knight.flags, 0);
        assert_eq!(knight.placement.position.y, 210.0);
        assert!(knight.companion.is_none());
        assert!(knight.light.is_none());
        let proctor = story_start(Background::Proctor, 1.0);
        assert_eq!(proctor.placement, knight.placement);
        let villager = story_start(Background::Villager, 1.0);
        assert_eq!(villager.role, 0x18);
        assert_eq!(villager.flags, 0x600);
        assert_eq!(villager.placement.position.y, 300.0);
        assert_ne!(villager.placement.rotation, NO_ROTATION);
        let villager_light = villager.light.unwrap();
        assert_eq!(villager_light.position, Vec3::new(8000.0, 500.0, 4000.0));
        assert_eq!(
            villager_light.rotation_degrees,
            Vec3::new(8.0, -150.0, 50.0)
        );
        assert_eq!(spawn_height(0.25, false), 6.25);
        assert_eq!(spawn_height(0.25, true), 14.25);
        assert_eq!(practice_height(0.25), 6.25);
        // The unknown spawn height follows the body's height.
        assert_eq!(
            story_start(Background::Unknown, 1.8).placement.position.y,
            34.0
        );
    }

    #[test]
    fn the_end_menus() {
        let won = end_menu(MENU_WON, Mode::Story);
        assert_eq!(won.rows, &["EXIT;"]);
        assert_eq!(won.scale, 0.9);
        let over = end_menu(MENU_WON, Mode::Practice);
        assert_eq!(over.rows, &["RESTART", "EXIT"]);
        assert_eq!(over.scale, 0.85);
        let reconstructed = end_menu(MENU_RECONSTRUCTED, Mode::Story);
        assert_eq!(reconstructed.rows, &["CONTINUE", "EXIT"]);
        assert_eq!(reconstructed.scale, 0.85);
        let arena = end_menu(MENU_ARENA, Mode::Arena);
        assert_eq!(arena.rows, &["CONTINUE", "LOBBY", "EXIT"]);
        assert_eq!(arena.scale, 0.78);
        assert!(arena.pad_bindings);
        let next = end_menu(MENU_NEXT, Mode::Practice);
        assert_eq!(next.rows, &["NEXT FIGHT"]);
        assert_eq!(next.scale, 0.9);
        assert_eq!(next.labels(), &["NEXT FIGHT"]);
    }

    #[test]
    fn the_practice_hook_waits_then_ends_rounds() {
        let mut flow = Flow {
            mode: Mode::Practice,
            phase: Phase::Playing,
            practice: PracticeState {
                ladder: Ladder::Novice,
                fight: 1,
                start_ms: 5000,
                next_tip: false,
            },
            ..Default::default()
        };
        assert!(flow.practice_tick(5500, None, &[DEAD]).is_empty());
        assert!(flow.practice_tick(6000, None, &[0]).is_empty());
        let steps = flow.practice_tick(6000, None, &[DEAD]);
        assert_eq!(flow.phase, Phase::RoundWon);
        assert!(steps.contains(&Step::EndMenu(end_menu(MENU_NEXT, Mode::Practice))));
        assert!(steps.contains(&Step::Tip(0)));
        let mut flow = Flow {
            mode: Mode::Practice,
            phase: Phase::Playing,
            practice: PracticeState {
                ladder: Ladder::Novice,
                fight: 8,
                start_ms: 0,
                next_tip: false,
            },
            ..Default::default()
        };
        let steps = flow.practice_tick(1000, None, &[DEAD]);
        assert_eq!(flow.phase, Phase::Won);
        assert!(flow.unlocks.expert);
        assert!(steps.contains(&Step::Tip(5)));
        assert!(steps.contains(&Step::EndMenu(end_menu(MENU_WON, Mode::Practice))));
        // A dying player does not end the round.
        let mut flow = Flow {
            mode: Mode::Practice,
            phase: Phase::Playing,
            practice: PracticeState {
                fight: 1,
                start_ms: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(
            flow.practice_tick(
                2000,
                Some(PlayerReading {
                    condition: DYING,
                    health: 0.1
                }),
                &[DEAD]
            )
            .is_empty()
        );
        // A dead player dies only once the guard second has passed and in
        // the playing phase, as the round check's own branch.
        let dead = Some(PlayerReading {
            condition: DEAD,
            health: 0.0,
        });
        flow.practice.start_ms = 5000;
        assert!(flow.practice_tick(5999, dead, &[]).is_empty());
        assert_eq!(flow.phase, Phase::Playing);
        let steps = flow.practice_tick(6000, dead, &[]);
        assert_eq!(flow.phase, Phase::Ended);
        assert!(steps.contains(&Step::Hook(Hook::None)));
        assert!(!steps.contains(&Step::SessionTask(false)));
        assert!(flow.practice_tick(7000, dead, &[]).is_empty());
    }

    #[test]
    fn the_practice_ladder_advances_and_heals() {
        let mut flow = Flow {
            mode: Mode::Practice,
            phase: Phase::RoundWon,
            practice: PracticeState {
                ladder: Ladder::Expert,
                fight: 7,
                start_ms: 0,
                next_tip: false,
            },
            ..Default::default()
        };
        flow.set_practice_health(0.2, 0.25);
        let steps = flow.practice_next(9000);
        assert_eq!(flow.practice.fight, 8);
        assert_eq!(flow.phase, Phase::Playing);
        // 0.2f32 as a double plus a tenth of 0.25, rounded to single:
        // 0x3e666667.
        let healed = (f64::from(0.2f32) + 0.25 * 0.1) as f32;
        assert!(steps.contains(&Step::Health {
            red_bits: healed.to_bits(),
            yellow_bits: healed.to_bits()
        }));
        assert_eq!(healed.to_bits(), 0x3e66_6667);
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Music {
                name: "Msc_ArenaBossE",
                ..
            }
        )));
        // The partner naming: the eighth expert fight is opponent16.
        let step = practice::next_practice(7, Ladder::Expert, 0.2, 0.25);
        assert_eq!(&step.name, b"opponent16");
        assert!(step.boss);
    }

    #[test]
    fn a_story_knockout_revives_after_five_seconds() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            hook: Hook::Story,
            ..Default::default()
        };
        let dead = PlayerReading {
            condition: DEAD,
            health: 0.1,
        };
        let steps = flow.story_tick(1000, Some(dead));
        assert_eq!(flow.hook, Hook::Knockout);
        assert!(steps.contains(&Step::Tip(2)));
        assert!(flow.knockout_tick(5000, Some(dead)).is_empty());
        let steps = flow.knockout_tick(6001, Some(dead));
        assert_eq!(flow.phase, Phase::Transition);
        assert!(steps.contains(&Step::Transition));
        let steps = flow.transition_due(6101).unwrap();
        assert_eq!(flow.phase, Phase::Playing);
        assert_eq!(flow.hook, Hook::Story);
        assert!(
            steps
                .iter()
                .any(|step| matches!(step, Step::Wake(WakePose { flags: 0x40, .. })))
        );
        assert_eq!(flow.story.wake.start_ms, 8101);
        // A dead body dies instead.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            hook: Hook::Knockout,
            story: StoryState {
                knockout_ms: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        let steps = flow.knockout_tick(
            6000,
            Some(PlayerReading {
                condition: DEAD,
                health: 0.0,
            }),
        );
        assert_eq!(flow.phase, Phase::Ended);
        assert!(steps.contains(&Step::Tip(3)));
    }

    #[test]
    fn the_story_hook_wakes_and_ticks_discovery() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            hook: Hook::Story,
            ..Default::default()
        };
        let alive = PlayerReading {
            condition: 0,
            health: 0.2,
        };
        let steps = flow.story_tick(100, Some(alive));
        assert!(!steps.contains(&Step::Discovery));
        assert_eq!(flow.story.last_discovery_s, 0);
        let steps = flow.story_tick(1000, Some(alive));
        assert!(steps.contains(&Step::Discovery));
        assert_eq!(flow.story.last_discovery_s, 1);
        // The delayed wake-up's motion starts three seconds before its time.
        flow.story.wake.delayed = true;
        flow.story.wake.start_ms = 10_000;
        assert!(
            !flow
                .story_tick(6000, Some(alive))
                .contains(&Step::RequestAction(0x20))
        );
        assert!(
            flow.story_tick(7001, Some(alive))
                .contains(&Step::RequestAction(0x20))
        );
        assert!(!flow.story.wake.delayed);
        // The get-up holds its pose for half a second, then writes the other
        // pose and stops.
        flow.story.wake.getting_up = true;
        flow.story.wake.start_ms = 10_000;
        let steps = flow.story_tick(10_200, Some(alive));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Wake(WakePose { turn, flags: 0x600, .. }) if *turn == 0.49
        )));
        let steps = flow.story_tick(10_501, Some(alive));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Wake(WakePose { turn, flags: 0x20, .. }) if *turn == 0.4
        )));
        assert!(!flow.story.wake.getting_up);
    }

    #[test]
    fn a_story_death_drops_the_save_or_strips_the_player() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            hook: Hook::Story,
            story: StoryState {
                reconstruction: 0,
                checkpoint: 7,
                record: 7,
                ..Default::default()
            },
            ..Default::default()
        };
        let steps = flow.die(0.0);
        assert_eq!(flow.phase, Phase::Ended);
        assert!(steps.contains(&Step::DropSave));
        assert!(steps.contains(&Step::Tip(3)));
        // A rebuildable player is stripped and gets the reconstructed menu.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            story: StoryState {
                reconstruction: REBUILDABLE,
                checkpoint: 7,
                record: 7,
                ..Default::default()
            },
            ..Default::default()
        };
        let steps = flow.die(0.0);
        assert!(steps.contains(&Step::StripPlayer));
        assert_eq!(flow.story.reconstruction, STRIPPED);
        assert!(steps.contains(&Step::EndMenu(end_menu(MENU_RECONSTRUCTED, Mode::Story))));
        // A living player's defeat does not touch the save.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            story: StoryState::default(),
            ..Default::default()
        };
        let steps = flow.die(0.2);
        assert!(!steps.contains(&Step::DropSave));
        assert!(steps.contains(&Step::Tip(2)));
        // Practice's defeat shows the restart rows.
        let mut flow = Flow {
            mode: Mode::Practice,
            phase: Phase::Playing,
            ..Default::default()
        };
        let _ = flow.die(0.0);
        assert!(flow.end_menu.unwrap().rows.contains(&"RESTART"));
    }

    #[test]
    fn every_level_exit_saves_and_early_ones_keep_a_checkpoint() {
        let rule = CheckpointRule {
            level: 3,
            setting: true,
            already_saved: false,
        };
        assert_eq!(exit_save(rule), ExitSave::Checkpoint);
        // The setting off, or a save that already holds the level (or the
        // fourth): a plain save.
        let off = CheckpointRule {
            setting: false,
            ..rule
        };
        assert_eq!(exit_save(off), ExitSave::Save);
        let held = CheckpointRule {
            already_saved: true,
            ..rule
        };
        assert_eq!(exit_save(held), ExitSave::Save);
        // Level 6 asks for its checkpoint whatever the setting says.
        assert_eq!(
            exit_save(CheckpointRule { level: 6, ..off }),
            ExitSave::Checkpoint
        );
        // Past level 6 every exit saves plainly.
        assert_eq!(
            exit_save(CheckpointRule { level: 7, ..rule }),
            ExitSave::Save
        );
        assert_eq!(level_locale(1), Some("exanima01"));
        assert_eq!(level_locale(5), Some("exanima05"));
        assert_eq!(level_locale(0), None);
        assert_eq!(level_locale(99), None);
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        // A level the table does not name does nothing.
        let none = LevelExit {
            level: 40,
            spawn: 0,
            flags: 0,
        };
        assert!(flow.level_exit(none, rule, 0, 1000).is_empty());
        let steps = flow.level_exit(
            LevelExit {
                level: 3,
                spawn: 4,
                flags: 1,
            },
            rule,
            0,
            1000,
        );
        assert_eq!(
            steps,
            vec![
                Step::Controls(false),
                Step::GameGui(false),
                Step::Save {
                    drop_on_low_health: false
                },
                Step::Checkpoint,
                Step::Transition,
            ]
        );
        assert_eq!(flow.phase, Phase::Transition);
        // A transition already up swallows another exit.
        let next = LevelExit {
            level: 4,
            spawn: 0,
            flags: 0,
        };
        assert!(flow.level_exit(next, rule, 0, 1000).is_empty());
        // The level loads at the exit's spawn point when the transition runs.
        assert!(flow.transition_due(1099).is_none());
        let steps = flow.transition_due(1100).unwrap();
        assert_eq!(
            steps,
            vec![Step::LoadLevel {
                locale: "exanima03",
                spawn: 4,
                teardown: true,
            }]
        );
        assert_eq!(flow.phase, Phase::Playing);
        // A dead player keeps no controls to lose.
        let steps = flow.level_exit(next, held, DEAD, 2000);
        assert_eq!(steps[0], Step::GameGui(false));
        assert!(!steps.contains(&Step::Checkpoint));
    }

    #[test]
    fn the_exit_trigger_nudges_the_player_out() {
        let trigger = ExitTrigger {
            centre: Vec3::ZERO,
            radius: 200.0,
            extent: Vec3::new(100.0, 100.0, 50.0),
            rotation: NO_ROTATION,
            exit_marker: true,
        };
        // Clearances 10 (x) and 50 (z): out along x by (100 − 90) + 75.
        let nudge = exit_nudge(Vec3::new(90.0, 10.0, 0.0), trigger).unwrap();
        assert_eq!(nudge.axis, 0);
        assert_eq!(nudge.distance, 85.0);
        assert_eq!(nudge.position, Vec3::new(175.0, 10.0, 0.0));
        // The negative branch takes the extent minus the signed offset and
        // negates it: −(100 + 90 + 75) = −265 cm (the original's arithmetic).
        let nudge = exit_nudge(Vec3::new(-90.0, 0.0, 0.0), trigger).unwrap();
        assert_eq!(nudge.distance, -265.0);
        assert_eq!(nudge.position.x, -355.0);
        let nudge = exit_nudge(Vec3::new(0.0, 0.0, 45.0), trigger).unwrap();
        assert_eq!(nudge.axis, 2);
        assert_eq!(nudge.distance, 80.0);
        // 1 cm past the box: its squared overhang 1 is within 50.
        let nudge = exit_nudge(Vec3::new(0.0, 0.0, 51.0), trigger).unwrap();
        assert_eq!(nudge.axis, 2);
        assert_eq!(nudge.distance, 74.0);
        // The squared overhang is compared with the plain 50: 7 cm out
        // (49) still counts, 8 cm out (64) does not.
        assert!(exit_nudge(Vec3::new(0.0, 0.0, 57.0), trigger).is_some());
        assert!(exit_nudge(Vec3::new(0.0, 0.0, 58.0), trigger).is_none());
        assert_eq!(
            exit_nudge(
                Vec3::new(90.0, 0.0, 0.0),
                ExitTrigger {
                    exit_marker: false,
                    ..trigger
                }
            ),
            None
        );
        // The radius check is three-dimensional: a 100 cm radius plus the
        // 50 cm reach holds a point 150 cm away, not 151 cm, and not 160 cm
        // straight up.
        let long = ExitTrigger {
            radius: 100.0,
            extent: Vec3::new(300.0, 200.0, 50.0),
            ..trigger
        };
        assert!(exit_nudge(Vec3::new(150.0, 0.0, 0.0), long).is_some());
        assert!(exit_nudge(Vec3::new(151.0, 0.0, 0.0), long).is_none());
        assert!(exit_nudge(Vec3::new(0.0, 160.0, 0.0), long).is_none());
        // A half turn: local x is −90, so the push is −265 along row 0,
        // (−1, 0, 0): the player moves 265 cm toward +x.
        let nudge = exit_nudge(
            Vec3::new(90.0, 0.0, 0.0),
            ExitTrigger {
                rotation: PRACTICE_ROTATION,
                ..trigger
            },
        )
        .unwrap();
        assert_eq!(nudge.distance, -265.0);
        assert_eq!(nudge.position.x, 355.0);
        // A quarter turn: row 0 is (0, 0, −1), so a player at z = −90 has
        // local x = 90 and is pushed 85 cm along that row, to z = −175.
        let nudge = exit_nudge(
            Vec3::new(0.0, 0.0, -90.0),
            ExitTrigger {
                rotation: yaw(90.0),
                ..trigger
            },
        )
        .unwrap();
        assert_eq!(nudge.axis, 0);
        assert!((nudge.distance - 85.0).abs() < 1e-3);
        assert!((nudge.position.z + 175.0).abs() < 1e-3);
        assert!(nudge.position.x.abs() < 1e-3);
    }

    #[test]
    fn the_node_filter_keeps_the_scenes_own_nodes() {
        // Nodes of other item types are never filtered.
        assert!(node_stays(0, 5, &[6, 7], 100, 0x6c));
        assert!(node_stays(0, 100, &[6, 7], 100, 0x6c));
        assert!(node_stays(0x4000, 5, &[6, 7], 100, 0x6c));
        assert!(!node_stays(0x4000, 100, &[6, 7], 100, 0x6c));
        assert!(!node_stays(0x4000, 0x6c, &[6, 7], 100, 0x6c));
        assert!(!node_stays(0x4000, 7, &[6, 7], 100, 0x6c));
        assert!(!node_stays(0x11, 6, &[6], 100, 0x6c));
        assert!(node_stays(0x11, 1, &[6], 100, 0x6c));
        assert!(!node_stays(0x4000, 100, &[], 100, 0x6c));
        assert!(node_stays(0x2000, 100, &[], 100, 0x6c));
    }

    #[test]
    fn the_flow_boots_into_the_title_and_switches_modes() {
        let mut flow = Flow::default();
        let steps = flow.boot();
        assert!(steps.contains(&Step::Boot(BootTask::Splash)));
        assert!(steps.contains(&Step::ShowMenu(Page::Title)));
        assert_eq!(flow.mode, Mode::Menu);
        assert_eq!(flow.phase, Phase::Menu);
        assert_eq!(flow.menu_page(), Page::Title);
        assert!(flow.boot().is_empty());
        assert!(flow.booted());
        // Practice keeps the hook it had: its start installs its own.
        let steps = flow.switch_mode(Mode::Practice, 1000);
        assert_eq!(flow.mode, Mode::Practice);
        assert_eq!(flow.hook, Hook::None);
        assert!(steps.is_empty());
        assert_eq!(flow.practice.start_ms, 1000);
        assert_eq!(flow.menu_page(), Page::InGame);
        // A switch clears the reconstruction state.
        flow.story.reconstruction = REBUILDABLE;
        flow.story.checkpoint = 7;
        let steps = flow.switch_mode(Mode::Story, 1000);
        assert_eq!(steps, vec![Step::Hook(Hook::Story)]);
        assert_eq!(flow.story.reconstruction, 0);
        assert_eq!(flow.story.checkpoint, 0);
        assert!(!flow.has_save[0]);
        assert!(flow.page_state().in_game);
        // The same mode does nothing.
        assert!(flow.switch_mode(Mode::Story, 1000).is_empty());
        // A scripted mode runs its enter hook.
        let steps = flow.switch_mode(Mode::Other(VERSUS_MODE), 1000);
        assert!(steps.contains(&Step::ScriptMode {
            mode: VERSUS_MODE,
            hook: ScriptHook::Enter
        }));
    }

    #[test]
    fn a_practice_start_uses_the_ring_and_its_music() {
        let mut flow = Flow::default();
        let view = Viewport {
            width: 1600,
            height: 1140,
        };
        let steps = flow.start_practice(Ladder::Expert, 0.2, view, 500);
        assert_eq!(flow.mode, Mode::Practice);
        assert_eq!(flow.practice.fight, 1);
        assert!(steps.contains(&Step::LoadLevel {
            locale: "arenae",
            spawn: 0,
            teardown: true,
        }));
        assert!(steps.contains(&Step::Camera(camera::frame_point(
            PRACTICE_POINT,
            true,
            false,
            view
        ))));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Music {
                name: "Msc_AreanExp",
                ..
            }
        )));
        assert_eq!(practice_music(Ladder::Expert, false).volume, 0.25);
        assert_eq!(
            practice_music(Ladder::Novice, false).volume.to_bits(),
            0x3dcc_cccd
        );
        assert_eq!(
            practice_music(Ladder::Novice, true).name,
            "Msc_ArenaNov_DnB"
        );
        assert_eq!(practice_boss_music(false, true).name, "Msc_ArenaBossE");
        assert_eq!(
            practice_boss_music(true, false).volume.to_bits(),
            0x3e19_999a
        );
        assert_eq!(menu_theme(false).name, "MenuTheme");
        assert_eq!(menu_theme(false).volume, 0.1);
        assert_eq!(menu_theme(true).volume.to_bits(), 0x3e4c_cccd);
        // The first partner's name.
        let first = practice::next_practice(0, Ladder::Expert, 0.2, 0.25);
        assert_eq!(&first.name, b"opponent09");
    }

    #[test]
    fn a_story_start_and_a_continue_place_the_player() {
        let mut flow = Flow::default();
        let view = Viewport {
            width: 800,
            height: 600,
        };
        let steps = flow.start_story(Background::Unknown, 1.8, view, 500);
        assert!(steps.contains(&Step::RequestAction(0x1d)));
        assert!(flow.story.wake.animating);
        assert_eq!(flow.story.wake.start_ms, 3500);
        assert_eq!(flow.mode, Mode::Story);
        assert_eq!(flow.phase, Phase::Playing);
        assert!(steps.contains(&Step::LoadLevel {
            locale: "exanima01",
            spawn: 0,
            teardown: true
        }));
        assert!(steps.contains(&Step::StoryPlayer {
            role: 0x0e,
            flags: 0x200
        }));
        assert!(steps.iter().any(|step| matches!(step, Step::Companion(_))));
        assert!(steps.iter().any(|step| matches!(step, Step::StoryLight(_))));
        let mut flow = Flow::default();
        let placement = Placement {
            rotation: NO_ROTATION,
            position: Vec3::new(1.0, 2.0, 3.0),
        };
        let steps = flow.continue_story(Some("exanima02"), placement, view);
        assert!(steps.contains(&Step::LoadLevel {
            locale: "exanima02",
            spawn: 0,
            teardown: true
        }));
        assert!(steps.contains(&Step::Camera(camera::frame_point(
            placement.position,
            true,
            false,
            view
        ))));
        let steps = flow.continue_story(None, placement, view);
        assert!(steps.contains(&Step::LoadLevel {
            locale: "exanima01",
            spawn: 0,
            teardown: true
        }));
        let mut roll = 0;
        let steps = flow.continue_arena(placement, view, &mut roll);
        assert!(steps.contains(&Step::LoadLevel {
            locale: "arenahub",
            spawn: 0,
            teardown: false
        }));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Music {
                name: "Msc_Hub01",
                ..
            }
        )));
    }

    #[test]
    fn the_frame_runs_dialogue_then_the_hook_only_in_play() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            hook: Hook::Story,
            ..Default::default()
        };
        let alive = Some(PlayerReading {
            condition: 0,
            health: 0.2,
        });
        let steps = flow.frame(1000, alive, &[]);
        assert_eq!(steps.first(), Some(&Step::Dialogue));
        assert!(steps.contains(&Step::Discovery));
        flow.menu_open = true;
        assert!(flow.frame(2000, alive, &[]).is_empty());
        flow.menu_open = false;
        flow.phase = Phase::Ended;
        assert!(flow.frame(2000, alive, &[]).is_empty());
        assert_eq!(flow.frames(), 3);
        // The transition runs first.
        flow.phase = Phase::Playing;
        flow.transition(Action::Revive, 3000);
        assert!(flow.frame(3050, alive, &[]).is_empty());
        assert!(
            flow.frame(3100, alive, &[])
                .iter()
                .any(|step| matches!(step, Step::Hook(Hook::Story)))
        );
    }

    #[test]
    fn a_pause_and_a_quit_go_through_the_phase_words() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = flow.pause();
        assert_eq!(flow.phase, Phase::Paused);
        assert_eq!(flow.saved_phase, Phase::Playing);
        assert!(steps.contains(&Step::Controls(false)));
        let steps = flow.resume();
        assert_eq!(flow.phase, Phase::Playing);
        assert!(steps.contains(&Step::Controls(true)));
        let steps = flow.quit_session(Page::InGame);
        // The application quit's story autosave keeps the death rule, and
        // nothing returns to the title.
        assert!(steps.contains(&Step::Save {
            drop_on_low_health: true
        }));
        assert!(!steps.contains(&Step::Transition));
        // The quit to the title runs behind the transition.
        assert_eq!(
            flow.transition(Action::QuitToMenu, 100),
            vec![Step::Transition]
        );
        let steps = flow.transition_due(200).unwrap();
        assert_eq!(flow.mode, Mode::Menu);
        assert!(steps.contains(&Step::ShowMenu(Page::Title)));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Music {
                name: "MenuTheme",
                ..
            }
        )));
        // The title's EXIT leaves the application.
        assert!(flow.quit_application().contains(&Step::Quit));
    }

    #[test]
    fn the_unlock_words_round_trip() {
        let mut unlocks = Unlocks::default();
        unlocks.read_backgrounds(&[0x05, 0, 0, 0]);
        assert_eq!(unlocks.backgrounds, 5);
        unlocks.read_expert(&[1, 0, 0, 0]);
        assert!(unlocks.expert);
        assert_eq!(unlocks.background_bytes(), [5, 0, 0, 0]);
        assert_eq!(unlocks.expert_bytes(), [1, 0, 0, 0]);
        // A missing or short entry leaves the word alone.
        unlocks.read_backgrounds(&[7, 0]);
        assert_eq!(unlocks.backgrounds, 5);
        unlocks.read_expert(&[0, 0, 0, 0]);
        assert!(!unlocks.expert);
        assert_eq!(Unlocks::read_word(&[1, 2, 3, 4]), Some(0x0403_0201));
        assert_eq!(Unlocks::read_word(&[]), None);
        assert_eq!(SCRIPT_HOOK, "GameEvent");
        assert_eq!(
            SETTINGS_KEYS,
            [
                "CursorSize",
                "CursorInt",
                "CursorCom",
                "FirstRun",
                "ShowGameTips",
                "CheckPoints"
            ]
        );
    }

    #[test]
    fn the_named_locale_requires_the_level() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        assert!(flow.leave_to_locale("exanima04", false, 100).is_empty());
        assert_eq!(flow.phase, Phase::Playing);
        let steps = flow.leave_to_locale("exanima04", true, 100);
        assert!(steps.contains(&Step::Transition));
        assert_eq!(flow.phase, Phase::Transition);
        let steps = flow.transition_due(200).unwrap();
        assert!(steps.contains(&Step::LoadLevel {
            locale: "exanima04",
            spawn: 0,
            teardown: true
        }));
    }

    #[test]
    fn the_class_cancel_and_the_splash_hand_over() {
        let mut flow = Flow {
            mode: Mode::Other(VERSUS_MODE),
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = flow.cancel_class_select(500);
        assert!(steps.contains(&Step::ScriptMode {
            mode: VERSUS_MODE,
            hook: ScriptHook::End
        }));
        assert!(steps.contains(&Step::Transition));
        let _ = flow.transition_due(600);
        assert_eq!(flow.mode, Mode::Menu);
        let mut flow = Flow::default();
        let steps = flow.after_splash();
        assert!(steps.contains(&Step::ShowMenu(Page::Title)));
        assert_eq!(flow.phase, Phase::Menu);
        // The scale notification only fires when it moved.
        assert!(flow.gui_scale_changed(gui_scale(1080), 1080).is_empty());
        assert_eq!(flow.gui_scale_changed(0.0, 1080), vec![Step::GuiScale(0.5)]);
        // The story respawn stands the player 15 cm over the portal and
        // leaves the camera.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Ended,
            viewport: Viewport {
                width: 1600,
                height: 1140,
            },
            ..Default::default()
        };
        let steps = flow.story_respawn(0);
        // No portal: the identity at the origin.
        assert!(steps.contains(&Step::PlacePlayer(Placement {
            rotation: NO_ROTATION,
            position: Vec3::new(0.0, 15.0, 0.0)
        })));
        flow.respawn_portal = Some(Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(1.0, 2.0, 3.0),
        });
        let steps = flow.story_respawn(0);
        assert!(steps.contains(&Step::PlacePlayer(Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(1.0, 17.0, 3.0)
        })));
        assert!(steps.contains(&Step::Save {
            drop_on_low_health: false
        }));
        assert!(!steps.iter().any(|step| matches!(step, Step::Camera(_))));
    }

    #[test]
    fn the_arena_quit_saves_or_forfeits() {
        let mut flow = Flow {
            mode: Mode::Arena,
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = flow.quit_session(Page::InGame);
        assert!(steps.contains(&Step::Save {
            drop_on_low_health: false
        }));
        assert!(!steps.contains(&Step::Forfeit));
        let mut flow = Flow {
            mode: Mode::Arena,
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = flow.quit_session(Page::ArenaPause);
        assert!(steps.contains(&Step::Forfeit));
        assert!(!steps.contains(&Step::Save {
            drop_on_low_health: false
        }));
        // A paused quit runs the arena unit's pause callback, in any mode,
        // and saves nothing.
        for mode in [Mode::Arena, Mode::Story] {
            let mut flow = Flow {
                mode,
                phase: Phase::Paused,
                ..Default::default()
            };
            assert_eq!(flow.quit_session(Page::InGame), vec![Step::ArenaPaused]);
        }
        // A stripped story player's quit keeps the death rule too.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            story: StoryState {
                reconstruction: STRIPPED,
                ..Default::default()
            },
            ..Default::default()
        };
        let steps = flow.quit_session(Page::InGame);
        assert!(steps.contains(&Step::Save {
            drop_on_low_health: true
        }));
        // A practice quit writes no save.
        let mut flow = Flow {
            mode: Mode::Practice,
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = flow.quit_session(Page::InGame);
        assert_eq!(steps, vec![Step::GameGui(false)]);
    }

    #[test]
    fn the_gui_scale_save_managers_and_hover() {
        assert_eq!(gui_scale(1080), 0.5);
        assert_eq!(gui_scale(453), 0.1875);
        assert_eq!(SAVE_MANAGERS[0].signature, save_package::STORY_SIGNATURE);
        assert_eq!(SAVE_MANAGERS[1].signature, save_package::ARENA_SIGNATURE);
        assert_eq!(SAVE_MANAGERS[1].data_version, Some(9));
        assert_eq!(SAVE_MANAGERS[0].package_magic, 0xDA7A_EA02);
        assert_eq!(hover_sound(false, 0), None);
        // The roll 0x80000000 is a half: 0.4·0.5 + 0.4.
        assert_eq!(hover_sound(true, 0x8000_0000), Some(0.6));
        assert_eq!(HOVER_VOLUME, 0.06);
        assert_eq!(Flow::default().practice_health(), 0.0);
        let mut flow = Flow::default();
        assert_eq!(flow.help_closed(), vec![Step::MenuHelp(false)]);
        assert!(flow.menu_confirm(false, true, 0).is_empty());
        assert_eq!(
            flow.menu_confirm(true, true, 0x8000_0000),
            vec![
                Step::Hover {
                    volume: 0.06,
                    gain: 0.6
                },
                Step::MenuConfirm
            ]
        );
        assert_eq!(flow.menu_confirm(true, false, 0), vec![Step::MenuConfirm]);
    }
}

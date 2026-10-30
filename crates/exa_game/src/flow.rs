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
use exa_formats::characters::{Carried, ItemRef, NEW_ENTRY_WORD};
use exa_formats::save_package;

use crate::arena_hub;
use crate::arena_match::Track;
use crate::end_menu_screen::EndMenuScreen;
use crate::menu_pages::{MenuCommand, Page, PageState};
use crate::menu_screen::{MenuChoice, MenuScreen, TitleLogo};
use crate::practice::{self, Ladder};
use exa_ui::transition::{Phase as ScreenPhase, TransitionFlags, TransitionScreen};

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
/// The character creator is showing.
pub const PHASE_CREATING: u32 = 2;
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
/// The class select (a setup form before a practice game) is showing.
pub const PHASE_CLASS_SELECT: u32 = 0x40;

/// The condition word of a dead character.
pub const DEAD: u32 = 0xA00;
/// The dying bit of the condition word (the falls branch).
pub const DYING: u32 = 0x800;
/// The condition bit the story's wake-up check waits for.
pub const WAKING: u32 = 0x20;
/// How many frames a transition waits for a level change it asked for to
/// show up before it goes on without it (a host with no level loader).
pub const LOAD_WAIT_FRAMES: u32 = 30;
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

/// The motion the story respawn asks the reloaded player for.
pub const RESPAWN_MOTION: u32 = 0x2f;

/// The practice ladder's first fight number. The first partner is
/// `opponent01` (novice) or `opponent09` (expert).
pub const FIRST_FIGHT: i32 = 1;

/// The role a practice mode's player starts with.
pub const PRACTICE_ROLE: u32 = 0x10;

/// The motion a practice start and each next fight asks the player for.
pub const PRACTICE_MOTION: u32 = 3;

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
    /// The character creator's setup form.
    Creating,
    Playing,
    Won,
    Ended,
    RoundWon,
    Forfeit,
    Paused,
    /// The class select's setup form.
    ClassSelect,
    /// A word the executable does not use.
    Other(u32),
}

impl Phase {
    /// The word of this phase.
    pub const fn word(self) -> u32 {
        match self {
            Phase::Transition => PHASE_TRANSITION,
            Phase::Menu => PHASE_MENU,
            Phase::Creating => PHASE_CREATING,
            Phase::Playing => PHASE_PLAYING,
            Phase::Won => PHASE_WON,
            Phase::Ended => PHASE_ENDED,
            Phase::RoundWon => PHASE_ROUND_WON,
            Phase::Forfeit => PHASE_FORFEIT,
            Phase::Paused => PHASE_PAUSED,
            Phase::ClassSelect => PHASE_CLASS_SELECT,
            Phase::Other(word) => word,
        }
    }

    /// The phase a word names.
    pub const fn from_word(word: u32) -> Self {
        match word {
            PHASE_TRANSITION => Phase::Transition,
            PHASE_MENU => Phase::Menu,
            PHASE_CREATING => Phase::Creating,
            PHASE_PLAYING => Phase::Playing,
            PHASE_WON => Phase::Won,
            PHASE_ENDED => Phase::Ended,
            PHASE_ROUND_WON => Phase::RoundWon,
            PHASE_FORFEIT => Phase::Forfeit,
            PHASE_PAUSED => Phase::Paused,
            PHASE_CLASS_SELECT => Phase::ClassSelect,
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

/// The point the practice shots look at: 5 m past the ring's middle, where
/// the partner stands (the start view's target keeps its 20 cm height).
pub const PRACTICE_POINT: Vec3 = Vec3::new(0.0, 0.0, 500.0);

/// Where the practice ladder's player stands: the ring's middle, with the
/// body's own height for the vertical. The spawn table's own vertical (500)
/// is the placeholder the start and each next fight overwrite, so the
/// horizontal position is the origin and only the height varies.
pub const PRACTICE_SPAWN: Vec3 = Vec3::new(0.0, 0.0, 0.0);

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

/// The camera view a session start writes before it places the player: the
/// forward direction and the point the view looks at, in centimetres.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StartView {
    pub direction: Vec3,
    pub target: Vec3,
}

/// The story start's view: forward along +z, looking at the room's far
/// corner (the level-start point with its height cleared).
pub const STORY_VIEW: StartView = StartView {
    direction: Vec3::new(0.0, 0.0, 1.0),
    target: Vec3::new(8200.0, 0.0, 4000.0),
};

/// The practice start's view: forward along −z, looking at the ring's middle
/// 20 cm up.
pub const PRACTICE_VIEW: StartView = StartView {
    direction: Vec3::new(0.0, 0.0, -1.0),
    target: Vec3::new(0.0, 20.0, 500.0),
};

/// The level-start point a story start hands the loader: (8200, 25, 3920).
/// The spawn table's own poses are absolute; this is the save manager's
/// starting point for the level.
pub const STORY_SPAWN: Vec3 = Vec3::new(8200.0, 25.0, 3920.0);

/// What a new story does for its background: the roles added to the player,
/// the story flag word, the spawn pose, the starting light, the companion
/// and the record writes (gear and powers) the start makes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoryStart {
    pub background: Background,
    /// The role ID added to the player's roles.
    pub role: u32,
    /// The whole role list the start gives the record, in order: the list
    /// is cleared, the two shared roles (7 and 1) are added, then the
    /// background's own.
    pub roles: &'static [u32],
    /// The story flag word the record gets.
    pub flags: u32,
    pub placement: Placement,
    /// The level-start point the save manager is given before the load.
    pub spawn_point: Vec3,
    /// The camera view written before the player is placed.
    pub view: StartView,
    /// The twelve-byte start record the loader is handed for the duration
    /// of the start: the start motion, then two zero words.
    pub start_record: [u32; 3],
    /// The item put in the record's off hand: the knight's and proctor's
    /// item 0x1f.
    pub off_hand: Option<ItemRef>,
    /// The entry appended to the record's carried list: the unknown
    /// background's item 0x10c (its third word 0x0080_0020) and the knight's
    /// and proctor's item 0x1f with flags 0x100 (the default third word).
    pub carried: Option<Carried>,
    /// The power word and the float beside the power slots the unknown
    /// background writes: the thaumaturge's
    /// [`creation_defaults`](crate::create::creation_defaults) values.
    pub powers: Option<(u32, f32)>,
    /// The condition word the start writes (the villager's 0x600).
    pub condition: Option<u32>,
    /// The animator flags the start writes (the villager's 0x600).
    pub animator: Option<u32>,
    /// The turn factor the start writes (the villager's 0.45, the knight's
    /// and proctor's 1.0; the unknown background leaves it).
    pub turn: Option<f32>,
    /// The lie-down the start makes after its first motion: the unknown
    /// background lies down to wake 3000 ms after the start
    /// ([`Step::KnockOut`]); the others start standing or getting up.
    pub knock_out_ms: Option<u32>,
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

/// How long after a new story starts the unknown background's lie-down
/// lets the player wake.
pub const STORY_WAKE_DELAY_MS: u32 = 3000;

/// The starting light's intensity (a byte-sized 256, stored as a float).
pub const STORY_LIGHT_INTENSITY: f32 = 256.0;
/// The companion's record flag.
pub const COMPANION_FLAGS: u32 = 0x500;

/// The two roles every new story adds before the background's own.
pub const STORY_COMMON_ROLES: [u32; 2] = [7, 1];
/// The item the unknown background's record carries.
pub const STORY_UNKNOWN_ITEM: u32 = 0x10c;
/// The third word of the unknown background's carried entry (its two
/// half-words 0x20 and 0x80 replace the default 0x28 and 4).
pub const STORY_UNKNOWN_ITEM_WORD: u32 = 0x0080_0020;
/// The item the knight and proctor hold in the off hand and carry.
pub const STORY_KNIGHT_ITEM: u32 = 0x1f;
/// The flags of the knight's and proctor's carried entry.
pub const STORY_KNIGHT_ITEM_FLAGS: u32 = 0x100;

/// What starting a new story does, from its background and the player's
/// height. The spawn table has three entries: the unknown background (the
/// villager's room, with Derrin), the knight and proctor (one pose), and the
/// villager (another).
pub fn story_start(background: Background, body_height: f32) -> StoryStart {
    match background {
        Background::Unknown => StoryStart {
            background,
            role: 0x0e,
            roles: &[7, 1, 0x0e],
            flags: 0x200,
            placement: Placement {
                rotation: yaw(150.0),
                position: Vec3::new(
                    STORY_POINT.x - 100.0,
                    body_height * 5.0 + 25.0,
                    STORY_POINT.z,
                ),
            },
            spawn_point: STORY_SPAWN,
            view: STORY_VIEW,
            start_record: [START_MOTION_ANIMATION, 0, 0],
            off_hand: None,
            carried: Some(Carried {
                item: STORY_UNKNOWN_ITEM,
                flags: 0,
                extra: STORY_UNKNOWN_ITEM_WORD,
            }),
            powers: {
                let powers = crate::create::creation_defaults();
                Some((powers.flags as u32, powers.value))
            },
            condition: None,
            animator: None,
            turn: None,
            knock_out_ms: Some(STORY_WAKE_DELAY_MS),
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
            roles: &[7, 1, 0x1b],
            flags: 0,
            placement: Placement {
                rotation: yaw(-50.0),
                position: Vec3::new(STORY_POINT.x, 210.0, STORY_POINT.z),
            },
            spawn_point: STORY_SPAWN,
            view: STORY_VIEW,
            start_record: [START_MOTION_WAKING, 0, 0],
            off_hand: Some(ItemRef {
                id: STORY_KNIGHT_ITEM,
                flags: 0,
            }),
            carried: Some(Carried {
                item: STORY_KNIGHT_ITEM,
                flags: STORY_KNIGHT_ITEM_FLAGS,
                extra: NEW_ENTRY_WORD,
            }),
            powers: None,
            condition: None,
            animator: None,
            turn: Some(1.0),
            knock_out_ms: None,
            light: None,
            companion: None,
        },
        Background::Villager => StoryStart {
            background,
            role: 0x18,
            roles: &[7, 1, 0x18],
            flags: 0x600,
            placement: Placement {
                rotation: rotation(50.0, -50.0, 2.0),
                position: Vec3::new(STORY_POINT.x, 300.0, STORY_POINT.z),
            },
            spawn_point: STORY_SPAWN,
            view: STORY_VIEW,
            start_record: [START_MOTION_VILLAGER, 0, 0],
            off_hand: None,
            carried: None,
            powers: None,
            condition: Some(0x600),
            animator: Some(0x600),
            turn: Some(0.45),
            knock_out_ms: None,
            light: Some(StoryLight {
                position: Vec3::new(8000.0, 500.0, 4000.0),
                rotation_degrees: Vec3::new(8.0, -150.0, 50.0),
                intensity: STORY_LIGHT_INTENSITY,
            }),
            companion: None,
        },
    }
}

/// The practice start's spawn height: the player body's smoothed standing
/// height plus 5 cm. (The next fight computes the same plus 5, or 13 in
/// combat mode, then overwrites it with the body's measured stand height
/// before it places the body: [`Flow::stand_height`].)
pub fn practice_height(standing: f32) -> f32 {
    standing + 5.0
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
    /// No row starts selected: a cursor that misses every row picks none
    /// instead of the first (the arena's menu sets it as it is built; a
    /// session's begin and continue clear it). The reading of the flag
    /// comes from the menus' shared row hit test.
    pub unselected: bool,
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
/// row, a mode with bit 2 (practice) RESTART and EXIT. Kind 5 is the
/// arena's (CONTINUE, LOBBY, EXIT). Every other kind is the next-fight
/// page. The scales are the form's 0.9 (default), 0.85 (two rows) and 0.78
/// (the arena's). The arena's menu alone refreshes the pad bindings and
/// starts with no row selected.
pub fn end_menu(kind: u32, mode: Mode) -> EndMenu {
    match kind {
        MENU_RECONSTRUCTED => EndMenu {
            kind,
            rows: RECONSTRUCTED_ROWS,
            scale: 0.85,
            pad_bindings: false,
            unselected: false,
        },
        MENU_WON => {
            if mode.practice_family() {
                EndMenu {
                    kind,
                    rows: PRACTICE_OVER_ROWS,
                    scale: 0.85,
                    pad_bindings: false,
                    unselected: false,
                }
            } else {
                EndMenu {
                    kind,
                    rows: WON_ROWS,
                    scale: 0.9,
                    pad_bindings: false,
                    unselected: false,
                }
            }
        }
        MENU_ARENA => EndMenu {
            kind,
            rows: ARENA_ROWS,
            scale: 0.78,
            pad_bindings: true,
            unselected: true,
        },
        _ => EndMenu {
            kind,
            rows: NEXT_ROWS,
            scale: 0.9,
            pad_bindings: false,
            unselected: false,
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

/// A wake clock write on a waking player: the knock-out's wake time
/// ([`exa_combat::condition::KnockOut::wake_at_ms`]), which the wake tick
/// compares with the game clock.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipClock {
    /// The wake time becomes the character clock less this many
    /// milliseconds. (The original reads the clock from the character
    /// manager the body points at, the same base its lie-down adds its
    /// delay to; it is taken as the game clock.)
    MotionMinus(i32),
    /// The wake time moves on by this many milliseconds.
    Push(i32),
}

impl ClipClock {
    /// The wake time after this write, from the current one and the game
    /// clock `now` (32-bit sums, as the original's).
    pub fn applied(self, wake_at_ms: u32, now: u32) -> u32 {
        match self {
            Self::MotionMinus(ms) => now.wrapping_add_signed(ms.wrapping_neg()),
            Self::Push(ms) => wake_at_ms.wrapping_add_signed(ms),
        }
    }
}

/// A wake-up pose write: the fields of the player's animator the original
/// writes on the frames of a get-up or a revival. A field that is `None` is
/// not written that frame, so the previous value stays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WakePose {
    /// The animator's flags word: 0x600 while the get-up holds its first
    /// pose, 0x20 once the get-up's clock has started, 0x40 for the
    /// revival's wake.
    pub flags: u32,
    /// The flags are set with a bitwise or instead of replaced.
    pub or_flags: bool,
    /// The blend the clip starts with.
    pub blend: Option<f32>,
    /// The move factor.
    pub move_factor: Option<f32>,
    /// The turn factor.
    pub turn_factor: Option<f32>,
    /// The clip clock write.
    pub clock: Option<ClipClock>,
}

impl WakePose {
    /// The pose a get-up holds until its clock starts: flags 0x600, blend 1,
    /// move and turn 0.49.
    pub const fn hold() -> Self {
        Self {
            flags: 0x600,
            or_flags: false,
            blend: Some(1.0),
            move_factor: Some(0.49),
            turn_factor: Some(0.49),
            clock: None,
        }
    }

    /// The pose the get-up writes when its clock starts: flags 0x20, blend
    /// 0.4, turn 0.4, and the clock at the motion's value less 1490 ms.
    pub const fn finish() -> Self {
        Self {
            flags: 0x20,
            or_flags: false,
            blend: Some(0.4),
            move_factor: None,
            turn_factor: Some(0.4),
            clock: Some(ClipClock::MotionMinus(1490)),
        }
    }

    /// The revival's write: the wake flag set and the clip clock pushed two
    /// seconds on.
    pub const fn revived() -> Self {
        Self {
            flags: 0x40,
            or_flags: true,
            blend: None,
            move_factor: None,
            turn_factor: None,
            clock: Some(ClipClock::Push(WAKE_DELAY_MS as i32)),
        }
    }
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
    /// The record identity the reconstruction word was raised for: a
    /// reconstruction device's scan writes the scanned player's identity
    /// here beside [`Self::reconstruction`], and a death rebuilds only when it
    /// matches [`Self::record`].
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

/// The settings store's first key word: the entries the game reads at boot
/// live under it.
pub const SETTINGS_SECTION: u32 = 1;
/// The expert-ladder unlock's settings key.
pub const EXPERT_UNLOCK_KEY: u32 = 0x6431_ac01;
/// The story-background unlock's settings key.
pub const BACKGROUND_UNLOCK_KEY: u32 = 0x6431_ac02;

/// The two unlock words the launch reads from the settings store. The
/// running app fills it from the user's store; the boot applies it before
/// the title page is built.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UnlockSettings {
    /// The entry's 4 bytes, when the store holds it.
    pub expert: Option<[u8; 4]>,
    pub backgrounds: Option<[u8; 4]>,
}

impl UnlockSettings {
    /// Reads the two entries from a settings store's `[1, key]` paths.
    pub fn from_store(store: &exa_formats::settings_store::SettingsStore) -> Self {
        let word = |key: u32| {
            let bytes = store.get(&[SETTINGS_SECTION, key])?;
            let four = bytes.get(0..4)?;
            Some([four[0], four[1], four[2], four[3]])
        };
        Self {
            expert: word(EXPERT_UNLOCK_KEY),
            backgrounds: word(BACKGROUND_UNLOCK_KEY),
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

/// One settings-file key the game main unit registers: its name and the
/// value kind the settings unit is told to read (`4` size, `5` integer,
/// `3` boolean).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsKey {
    pub name: &'static str,
    pub kind: u32,
}

/// The settings keys the game main unit registers. Their values are read by
/// the units named next to them, not here.
pub const SETTINGS_KEYS: [SettingsKey; 6] = [
    SettingsKey {
        name: "CursorSize",
        kind: 4,
    },
    SettingsKey {
        name: "CursorInt",
        kind: 5,
    },
    SettingsKey {
        name: "CursorCom",
        kind: 5,
    },
    SettingsKey {
        name: "FirstRun",
        kind: 3,
    },
    SettingsKey {
        name: "ShowGameTips",
        kind: 3,
    },
    SettingsKey {
        name: "CheckPoints",
        kind: 3,
    },
];

/// The strings the unit releases at shutdown: the two menu theme names and
/// its text shader source.
pub const FINALISED_STRINGS: [&str; 3] = [
    exa_ui::menu::MENU_THEME.0,
    exa_ui::menu::MENU_THEME_DNB.0,
    crate::menu_text::MENU_TEXT_SHADER,
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

    /// Reads the expert unlock's word: bit 0 unlocks the expert ladder (the
    /// page's disabled-row test reads that bit).
    pub fn read_expert(&mut self, bytes: &[u8]) {
        if let Some(word) = Self::read_word(bytes) {
            self.expert = word & 1 != 0;
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

    /// Applies the two words the launch read. A missing entry leaves its
    /// current value, as the original's failed read does.
    pub fn apply_settings(&mut self, settings: UnlockSettings) {
        if let Some(bytes) = settings.expert {
            self.read_expert(&bytes);
        }
        if let Some(bytes) = settings.backgrounds {
            self.read_backgrounds(&bytes);
        }
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
    /// Load the actor/character template library (and `actors.rpk`).
    ActorLibrary,
    /// Initialise the game layer's interface.
    GameGui,
    /// Initialise the item interface.
    ItemGui,
    /// Initialise the skill screen's interface.
    SkillGui,
    /// Read the hardware settings.
    HardwareConfig,
    /// Hand the current client's row scale to the GUI.
    GuiScale,
    /// Register the menu's key task.
    MenuTask,
    /// Apply the first-run end-page tip.
    FirstRunTip,
    /// Apply the first-run settings.
    FirstRun,
    /// Read the unlock words.
    Settings,
    /// Initialise the experience system.
    Experience,
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
    /// Filter the loaded level's nodes: a container's children and the
    /// player's own objects go (the practice scene filter).
    PracticeFilter,
    /// Place the player.
    PlacePlayer(Placement),
    /// Give the player its story roles (the list cleared first) and flags,
    /// and the gear and powers only the background's start writes.
    StoryPlayer {
        roles: &'static [u32],
        flags: u32,
        off_hand: Option<ItemRef>,
        carried: Option<Carried>,
        powers: Option<(u32, f32)>,
    },
    /// The twelve-byte record a story start hands the loader: the start
    /// motion and two zero words.
    StartRecord([u32; 3]),
    /// Create the story's starting light.
    StoryLight(StoryLight),
    /// Write the story start's condition word, animator flags and turn
    /// factor (the fields each background sets; `None` leaves one).
    StoryPose {
        condition: Option<u32>,
        flags: Option<u32>,
        turn_factor: Option<f32>,
    },
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
    /// Close the save manager's open package.
    CloseSave,
    /// Keep the save just written as the checkpoint.
    Checkpoint,
    /// Stamp the level-6 checkpoint with the engine clock (the level's own
    /// time; the save unit reads it).
    LevelTime,
    /// The screen fades out over the level change (the exit's fade bit).
    FadeOut,
    /// The AI takes over the player's body for the level change (the exit's
    /// fade bit).
    AiHandover,
    /// Leave the world: props, bodies, particles, blood and the level.
    TeardownWorld,
    /// Show the end-of-session menu.
    EndMenu(EndMenu),
    /// Hide the end-of-session menu.
    HideEndMenu,
    /// Close the prelude's tips and popup.
    ClosePrelude,
    /// Clear or set the menu's help flag (the form field the help screen
    /// clears when it closes).
    MenuHelp(bool),
    /// Set or clear the end menu's no-row-selected flag
    /// ([`EndMenu::unselected`]).
    EndMenuUnselected(bool),
    /// Show or hide the title logo the main-menu form draws above its rows.
    TitleLogo(bool),
    /// The GUI's scale was re-quantised; both menus lay out again.
    GuiScale(f32),
    /// The arena's own pause-screen callback runs.
    ArenaPaused,
    /// The session forfeits the match.
    Forfeit,
    /// A story hook another unit installed runs.
    CustomHook,
    /// The end menu's click sound: at this volume and pitch.
    Hover {
        volume: f32,
        pitch: f32,
    },
    /// The end menu confirms its selected row.
    MenuConfirm,
    /// Give or take the player's controls. Taking them is the controls'
    /// disable (the knock-out and the death path); giving them is their
    /// enable (the revive) or their re-attach to the player at a session
    /// start or the story respawn, which also clears the disabled flag.
    Controls(bool),
    /// Turn the player's combat mode on or off: the player controls' setter
    /// (the practice start and each next fight turn it on, the revive off).
    CombatMode(bool),
    /// Lay the player down to wake: the condition becomes down (0x200), the
    /// state flags' wake nibble knocked-out and waking (0x30), the tone and
    /// stance factor 0, the support pins' gain 0.08 and the wake clock the
    /// game clock plus `delay_ms` (the new story's unknown background waits
    /// 3000 ms, the story respawn none).
    KnockOut {
        delay_ms: u32,
    },
    /// Raise the knocked-out player's dead body (the story revive): only a
    /// dead condition word (0xA00) is raised; combat mode is left, the
    /// yellow health becomes half the red and the body is laid down to wake
    /// at once.
    ReviveBody,
    /// Strip the player's gear (the reconstruction device).
    StripPlayer,
    /// Clear the player node's story bit (the reconstruction mark).
    ClearStoryFlag,
    /// Write the player's own record back into the save, with the record's
    /// flags word (the death screen's CONTINUE passes 0x100).
    PlayerData {
        flags: u32,
    },
    /// Show or hide the game's view.
    GameGui(bool),
    /// Write a wake-up or get-up pose.
    Wake(WakePose),
    /// The motion the player is asked to play.
    RequestAction(u32),
    /// Add the practice mode's player role.
    PracticePlayer {
        role: u32,
    },
    /// The health the practice heal writes (red, then yellow).
    Health {
        red_bits: u32,
        yellow_bits: u32,
    },
    /// Replace the player's condition word (the next practice fight clears
    /// the round's dead or waking bits).
    Condition(u32),
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
    /// Write a 4-byte value into the settings store under section 1 (the
    /// expert-ladder and story-background unlock words).
    SettingsWrite {
        key: u32,
        bytes: [u8; 4],
    },
    /// The unit's own tables are released at shutdown (the scripted-mode
    /// hook table and the three menu strings).
    Finalise,
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
    /// Leave the session to the menu form and its theme (practice's
    /// RESTART).
    LeaveToMenuForm,
    /// Load a named level at a spawn point.
    Locale { locale: &'static str, spawn: u32 },
    /// The class select's cancel: the scripted mode's end hook and the
    /// return to the title.
    ClassCancel,
}

/// The running transition: its action, the screen's mode flags and, once
/// it has run a frame, the screen itself
/// ([`exa_ui::transition::TransitionScreen`]).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Transition {
    action: Action,
    flags: TransitionFlags,
    screen: Option<TransitionScreen>,
    /// The action asked for a level change: the screen stays black (its
    /// clock held) until the change is done, as the original's action loads
    /// the level before it returns to the screen.
    load: Option<LoadWait>,
}

/// A transition waiting for the level change its action asked for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LoadWait {
    /// The change was seen in progress.
    seen: bool,
    /// Frames waited without seeing it.
    frames: u32,
}

/// What the running app reads from one frame of the transition screen.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransitionFrame {
    /// The steps the screen's action returned, on the frame it ran.
    pub steps: Vec<Step>,
    /// The screen's black alpha byte (0 when no transition runs).
    pub alpha: u8,
    /// The music gain to write while the music fades out with the screen.
    pub music: Option<f32>,
    /// The screen reached black with the music fade on: the main track
    /// stops and forgets its name.
    pub stop_music: bool,
    /// The screen-transition audio fade to write.
    pub audio_fade: Option<f32>,
    /// The screen reached black: the world fade is reset to 1.
    pub world_fade_reset: bool,
    /// The screen faded back in and closed this frame.
    pub closed: bool,
}

/// What the transition screen reads from the running app each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TransitionInput {
    /// The real clock, in milliseconds (the screen's stamp clock).
    pub clock_ms: u32,
    /// The main track's current gain, when it has a voice.
    pub music: Option<f32>,
    /// The screen-transition audio fade now in force.
    pub audio_fade: f32,
    /// A level change is in progress (torn down, loading).
    pub loading: bool,
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
    /// The camera view the start writes before it places the player.
    pub view: StartView,
}

/// The music a practice start plays.
pub fn practice_music(ladder: Ladder, alternative: bool) -> Track {
    let (name, bits) = match (alternative, ladder) {
        (false, Ladder::Novice) => ("Msc_AreanNov", 0x3dcc_cccd),
        (false, Ladder::Expert) => ("Msc_AreanExp", 0x3e80_0000),
        (true, Ladder::Novice) => ("Msc_ArenaNov_DnB", 0x3e19_999a),
        (true, Ladder::Expert) => ("Msc_ArenaExp_DnB", 0x3e19_999a),
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

/// A save manager object as the boot builds it.
///
/// The constructor takes the file prefix and an identifier word. It keeps a
/// copy of the prefix and its length, stores the identifier, and starts two
/// words at −1 (a count and a current slot by their use; the routines that
/// set them are not traced). The boot then installs the load and save hooks,
/// the arena's also a third, writes the package-magic word and registers the
/// player-data signature.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveFamily {
    pub prefix: String,
    /// The prefix's length, kept as its own word.
    pub prefix_len: u32,
    /// The constructor's identifier word.
    pub id_word: u32,
    /// The first word that starts at −1.
    pub count: i32,
    /// The second word that starts at −1.
    pub current: i32,
    /// Whether the boot installs a third hook (the arena's only).
    pub extra_hook: bool,
    /// The magic word the boot writes into the manager.
    pub package_magic: u32,
}

impl SaveFamily {
    /// The constructor with the boot's follow-up writes.
    pub fn new(prefix: &str, id_word: u32, extra_hook: bool, package_magic: u32) -> Self {
        Self {
            prefix: prefix.to_owned(),
            prefix_len: prefix.len() as u32,
            id_word,
            count: -1,
            current: -1,
            extra_hook,
            package_magic,
        }
    }
}

/// The story manager's file prefix and constructor identifier.
pub const STORY_PREFIX: &str = "Exanima";
pub const STORY_ID_WORD: u32 = 0x6431_AC03;
/// The arena manager's file prefix and constructor identifier.
pub const ARENA_PREFIX: &str = "Arena";
pub const ARENA_ID_WORD: u32 = 0x6431_AC13;
/// The magic word the boot writes into the arena manager. It is not the
/// story's package magic; what reads it is not traced.
pub const ARENA_MANAGER_MAGIC: u32 = 0xDA7A_AA00;

/// The story's save manager object.
pub fn story_family() -> SaveFamily {
    SaveFamily::new(
        STORY_PREFIX,
        STORY_ID_WORD,
        false,
        save_package::PACKAGE_MAGIC,
    )
}

/// The arena's save manager object.
pub fn arena_family() -> SaveFamily {
    SaveFamily::new(ARENA_PREFIX, ARENA_ID_WORD, true, ARENA_MANAGER_MAGIC)
}

/// A player's body state, as the flow reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerReading {
    pub condition: u32,
    /// The player's animator flags word (the get-up's 0x600, the wake's
    /// 0x20/0x40); [`WAKING`] there ends the first wake-up animation.
    pub animator: u32,
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

/// The arena pause page's FORFEIT row ran: the running arena match settles
/// as forfeited and returns to the hub.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForfeitMatch;

/// A 4-byte settings entry the flow asks the host to write. The unlock words
/// live under section 1, as every boot-time entry does.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SettingsEntry {
    pub section: u32,
    pub key: u32,
    pub bytes: [u8; 4],
}

/// The steps the flow asked for since the last frame: every producer pushes
/// here and [`apply_steps`] applies the ones the running app owns.
#[derive(Resource, Debug, Default)]
pub struct FlowSteps {
    pub steps: Vec<Step>,
}

/// Whether the boot's title page is shown at start-up. The original always
/// boots to it; Exarust's launch modes load a level or a model directly, so
/// the launcher asks for the title only on the splash path.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BootToMenu(pub bool);

impl Default for BootToMenu {
    fn default() -> Self {
        Self(true)
    }
}

/// Whether the game's interface is drawn over the world (the original's game
/// view). The frame flow closes it on a death, an end-menu exit and a level
/// change, and opens it again on a revival.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameView(pub bool);

impl Default for GameView {
    fn default() -> Self {
        Self(true)
    }
}

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
    /// The level exit's fade bit set the transition screen's long hold
    /// (a 2000 ms fade and a 1000 ms hold) for the next transition.
    long_transition: bool,
    pub story: StoryState,
    pub practice: PracticeState,
    pub unlocks: Unlocks,
    /// The expert-ladder unlock has already been written to the settings
    /// store (the original's flag, so a second win does not write it again).
    pub expert_written: bool,
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
    /// The player body's measured stand height (its root over its lowest
    /// contact), which the next practice fight places the body at.
    pub stand_height: f32,
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
            long_transition: false,
            story: StoryState {
                knockout_ms: u32::MAX - KNOCKOUT_MS,
                ..Default::default()
            },
            practice: PracticeState::default(),
            unlocks: Unlocks::default(),
            expert_written: false,
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
            stand_height: 0.0,
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
    /// The page state the menu table reads. The two managers' saves are kept
    /// apart: the title's CONTINUE reads the story's, the start page's the
    /// arena's. The in-game page reads NEW GAME for a mode with bit 2
    /// (value 2: practice), and the arena pause page's last row reads the
    /// session's value-4 bit.
    pub fn page_state(&self) -> PageState {
        PageState {
            has_save: self.has_save[0] || self.has_save[1],
            story_save: self.has_save[0],
            arena_save: self.has_save[1],
            new_game: self.mode.practice_family(),
            arena: self.versus,
            expert: self.unlocks.expert,
            session: self.phase.word(),
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
        // The boot's settings read leaves the "expert ladder already
        // unlocked on disk" flag set when the stored word is not zero.
        self.expert_written = self.unlocks.expert;
        let mut steps = vec![
            Step::Boot(BootTask::Scripts),
            Step::Boot(BootTask::Window),
            Step::Boot(BootTask::FirstRun),
            Step::Boot(BootTask::ActorLibrary),
            Step::Boot(BootTask::GameGui),
            Step::Boot(BootTask::ItemGui),
            Step::Boot(BootTask::SkillGui),
            Step::Boot(BootTask::HardwareConfig),
            Step::Boot(BootTask::GuiScale),
            Step::Boot(BootTask::MenuTask),
            Step::Boot(BootTask::FirstRunTip),
            Step::Boot(BootTask::Settings),
            Step::Boot(BootTask::Experience),
            Step::Boot(BootTask::SaveManager(Mode::Story)),
            Step::Boot(BootTask::SaveManager(Mode::Arena)),
            Step::Boot(BootTask::Transition),
            Step::Boot(BootTask::ArenaData),
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
            // The player's roles, gear and powers first (the original writes
            // them before the level is loaded), then the level, the light,
            // the starting motion, the place and the camera shot.
            Step::StoryPlayer {
                roles: start.roles,
                flags: start.flags,
                off_hand: start.off_hand,
                carried: start.carried,
                powers: start.powers,
            },
            Step::LoadLevel {
                locale: "exanima01",
                spawn: 0,
                teardown: true,
            },
            Step::StartRecord(start.start_record),
            // The player controls are attached to the loaded player (their
            // disabled flag cleared) before the background's light and
            // motion.
            Step::Controls(true),
        ];
        if let Some(light) = start.light {
            steps.push(Step::StoryLight(light));
        }
        steps.push(Step::RequestAction(motion));
        if let Some(delay_ms) = start.knock_out_ms {
            steps.push(Step::KnockOut { delay_ms });
        }
        if start.condition.is_some() || start.animator.is_some() || start.turn.is_some() {
            steps.push(Step::StoryPose {
                condition: start.condition,
                flags: start.animator,
                turn_factor: start.turn,
            });
        }
        steps.push(Step::PlacePlayer(start.placement));
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
        standing: f32,
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
        let step = practice::next_practice(0, ladder, 0.0, 0.0);
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
            view: PRACTICE_VIEW,
        };
        let placement = Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(
                PRACTICE_SPAWN.x,
                practice_height(standing),
                PRACTICE_SPAWN.z,
            ),
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
            Step::PracticeFilter,
            Step::PracticePlayer {
                role: PRACTICE_ROLE,
            },
            // The controls are attached to the player, then combat mode is
            // turned on (the original skips it only in the story mode,
            // which never starts practice).
            Step::Controls(true),
            Step::CombatMode(true),
            Step::RequestAction(PRACTICE_MOTION),
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

    /// Continues a story save: the menu form closes, the first wake-up's
    /// animating flag clears, the save's locale is loaded (or the first level
    /// when there is none), the player nudged out of any exit trigger by the
    /// host, and the near shot framed on the player's body, fast.
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
        self.menu_open = false;
        // A continued save clears the first wake-up's animating flag; the
        // get-up and delayed flags and the start time are left as they are.
        self.story.wake.animating = false;
        vec![
            Step::HideMenu,
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

    /// The arena's pause: its own form sets the paused phase (0x20) and
    /// builds the arena pause page. The story's in-game page covers the game
    /// without a phase of its own, as the original's frame task only hides
    /// behind the visible menu.
    pub fn pause(&mut self) -> Vec<Step> {
        if self.phase == Phase::Playing {
            self.saved_phase = self.phase;
            self.phase = Phase::Paused;
            self.menu_open = true;
            return vec![Step::ShowMenu(Page::ArenaPause), Step::Controls(false)];
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

    /// Opens the transition screen over `action`: the screen fades to black
    /// and runs the action there ([`Flow::transition_tick`]). `fade_audio`
    /// fades the music out with the screen and stops it at black; each
    /// caller passes the original's argument. The screen's short-hold flag
    /// (the third argument) is clear for every caller in this unit; the
    /// long hold is the level exit's fade bit, which the original sets on
    /// the screen before it opens and the open leaves alone. The phase
    /// before the transition is kept and the phase word becomes the
    /// transition's.
    pub fn transition(&mut self, action: Action, fade_audio: bool) -> Vec<Step> {
        self.saved_phase = self.phase;
        self.phase = Phase::Transition;
        let hold_1000 = core::mem::take(&mut self.long_transition)
            || self.transition.is_some_and(|t| t.flags.hold_1000);
        self.transition = Some(Transition {
            action,
            flags: TransitionFlags {
                fade_audio,
                hold_1000,
                ..Default::default()
            },
            screen: None,
            load: None,
        });
        vec![Step::Transition]
    }

    /// Whether a transition screen is up.
    pub fn transition_running(&self) -> bool {
        self.transition.is_some()
    }

    /// One frame of the transition screen, which the original runs as a GUI
    /// task whatever covers the game. The screen starts on its first frame
    /// (the clock is the frame's real clock, as the screen's stamp clock),
    /// fades to black, runs its action there, holds two frames and the
    /// hold time and fades back in. An action that changes the level holds
    /// the screen at black until the change is done: the original loads the
    /// level inside the action, so its hold starts after the load. The
    /// original's close also hides the form and clears four engine words
    /// whose roles are not identified; they are not modelled.
    pub fn transition_tick(&mut self, input: TransitionInput, now: u32) -> TransitionFrame {
        let mut frame = TransitionFrame::default();
        let Some(mut transition) = self.transition else {
            return frame;
        };
        let clock = input.clock_ms;
        let mut screen = transition.screen.unwrap_or_else(|| {
            let mut screen =
                TransitionScreen::new(clock, transition.flags, input.music.unwrap_or(0.0));
            // The screen writes the audio fade only below 1, from the value
            // the engine holds.
            screen.dim = input.audio_fade;
            screen
        });
        if let Some(mut wait) = transition.load {
            if input.loading {
                wait.seen = true;
            } else if !wait.seen && wait.frames < LOAD_WAIT_FRAMES {
                wait.frames += 1;
            }
            let done = !input.loading && (wait.seen || wait.frames >= LOAD_WAIT_FRAMES);
            // Held: the screen's clock follows the frame's, so its hold
            // starts when the change is done.
            screen.start = clock;
            if !done {
                transition.load = Some(wait);
                transition.screen = Some(screen);
                self.transition = Some(transition);
                frame.alpha = screen.alpha;
                return frame;
            }
            transition.load = None;
        }
        let before = screen.phase;
        let dim = screen.dim;
        let step = screen.step(clock);
        let fading_out = before == ScreenPhase::FadingOut;
        if fading_out && transition.flags.fade_audio && input.music.is_some() {
            frame.music = Some(screen.music);
        }
        if fading_out && screen.phase != ScreenPhase::FadingOut {
            // Black: the original resets the world fade on every path, and
            // with the music fade on it stops the main track and zeroes the
            // screen's audio fade.
            frame.world_fade_reset = true;
            frame.stop_music = transition.flags.fade_audio;
        }
        if screen.dim != dim {
            frame.audio_fade = Some(screen.dim);
        }
        frame.alpha = step.alpha;
        transition.flags = screen.flags;
        transition.screen = Some(screen);
        self.transition = Some(transition);
        if step.callback {
            frame.steps = self.apply(transition.action, now);
            let changes_level = frame
                .steps
                .iter()
                .any(|step| matches!(step, Step::LoadLevel { .. } | Step::TeardownWorld));
            if changes_level && let Some(running) = self.transition.as_mut() {
                running.load = Some(LoadWait::default());
            }
        }
        if step.close {
            self.transition = None;
            frame.closed = true;
        }
        frame
    }

    fn apply(&mut self, action: Action, now: u32) -> Vec<Step> {
        match action {
            Action::QuitToMenu => self.quit_to_menu(),
            Action::StoryRespawn => self.story_respawn(now),
            Action::Revive => self.revive(now),
            Action::NextPracticeFight => self.practice_next(now),
            Action::LeaveToMenuForm => self.leave_to_menu_form(),
            Action::ClassCancel => self.class_cancel(),
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
    /// the game and the phase word is not the transition's (zero). Any
    /// other phase runs it: the phases that end a session (won, ended)
    /// clear the hook themselves, so only the dialogue engine ticks there.
    pub fn frame(&mut self, now: u32, player: Option<PlayerReading>, others: &[u32]) -> Vec<Step> {
        self.frames = self.frames.wrapping_add(1);
        if self.menu_open {
            return Vec::new();
        }
        if self.phase == Phase::Transition {
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
            // The original reads the animator's flags word, not the body's
            // condition: the first animation runs while that word holds 0x20
            // and ends once the motion has cleared the bit.
            if player.animator & WAKING == 0 {
                wake.animating = false;
            }
        } else if wake.getting_up {
            if u64::from(wake.start_ms) + u64::from(GET_UP_MS) < u64::from(now) {
                steps.push(Step::Wake(WakePose::finish()));
                wake.getting_up = false;
            } else {
                steps.push(Step::Wake(WakePose::hold()));
            }
        } else if wake.delayed
            && i64::from(wake.start_ms) - i64::from(WAKE_LEAD_MS) < i64::from(now)
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
                // The end menu's own practice branch: the novice ladder shows
                // tip 5, writes the expert-ladder setting once (a flag keeps
                // later wins from writing it again) and unlocks the page; the
                // expert ladder shows tip 6.
                let mut steps = Vec::new();
                if self.practice.ladder == Ladder::Novice {
                    steps.push(Step::Tip(5));
                    if !self.expert_written {
                        self.expert_written = true;
                        self.unlocks.expert = true;
                        steps.push(Step::SettingsWrite {
                            key: EXPERT_UNLOCK_KEY,
                            bytes: Unlocks {
                                expert: true,
                                ..Default::default()
                            }
                            .expert_bytes(),
                        });
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
        let placement = Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(PRACTICE_SPAWN.x, self.stand_height, PRACTICE_SPAWN.z),
        };
        let mut steps = self.close_end_menu();
        steps.extend([
            // The original clears the scene and imports the arena's stored
            // level section again, through the same practice scene filter
            // the first fight's load used (the save unit's combatant
            // preparation). Exarust reloads the arena from the archive in
            // process and runs the filter after it, the same as
            // `start_practice`.
            Step::LoadLevel {
                locale: match self.practice.ladder {
                    Ladder::Novice => "arena",
                    Ladder::Expert => "arenae",
                },
                spawn: 0,
                teardown: true,
            },
            Step::PracticeFilter,
            Step::MenuHelp(false),
            // Combat mode goes on for the fight; the controls were never
            // taken (a won round leaves them).
            Step::CombatMode(true),
            Step::Health {
                red_bits: step.red_bits,
                yellow_bits: step.yellow_bits,
            },
            // The next fight starts from a clear condition word.
            Step::Condition(0),
            Step::RequestAction(PRACTICE_MOTION),
            Step::PlacePlayer(placement),
            Step::Camera(camera::frame_point(
                PRACTICE_POINT,
                false,
                false,
                self.viewport,
            )),
            Step::Hook(Hook::Practice),
            Step::SessionTask(true),
        ]);
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
        // A NaN health takes the alive branch too: the original's `0.01 <= h`
        // test jumps to the alive tip on the unordered result as well.
        let alive = f64::from(health) >= ALIVE_HEALTH || health.is_nan();
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
            self.transition(Action::Revive, false)
        } else {
            Vec::new()
        }
    }

    /// The revived story player, behind the transition: a session in a game
    /// mode is saved first (without the death rule), combat mode is left,
    /// the dead body is raised and laid down, its wake held (0x40) and its
    /// wake clock pushed two seconds on, the controls and the game view
    /// return, the wake-up animation is watched again two seconds on, and
    /// the story hook resumes.
    pub fn revive(&mut self, now: u32) -> Vec<Step> {
        self.phase = Phase::Playing;
        self.hook = Hook::Story;
        self.story.wake = Wake {
            animating: true,
            getting_up: false,
            delayed: false,
            start_ms: now.wrapping_add(WAKE_DELAY_MS),
        };
        let mut steps = vec![Step::ClosePrelude];
        if self.mode.is_game() {
            steps.push(Step::Save {
                drop_on_low_health: false,
            });
        }
        // Combat mode is left before the dead body is raised and laid down
        // (held, its wake two seconds later); the controls are enabled and
        // the game view returns.
        steps.extend([
            Step::CombatMode(false),
            Step::ReviveBody,
            Step::Wake(WakePose::revived()),
            Step::Controls(true),
            Step::GameGui(true),
            Step::Hook(Hook::Story),
        ]);
        steps
    }

    /// The story's death screen CONTINUE: the player node's story bit
    /// clears, the save is written (without the death rule), the world torn
    /// down and `exanima05` loaded (the loader's second word, 1, is not
    /// traced and is read as the load's own flag); the player stands at the
    /// level's respawn portal (its pose, or the identity at the origin when
    /// the level has none) 15 cm higher, the controls are attached to it
    /// again (which gives them back after the death), and it lies down to
    /// wake at once and plays the get-up motion. The
    /// reconstruction state becomes rebuildable. The camera is not moved.
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
            Step::ClearStoryFlag,
            Step::Save {
                drop_on_low_health: false,
            },
            // The end menu hides and the prelude closes before the world
            // goes.
            Step::HideEndMenu,
            Step::ClosePrelude,
            Step::TeardownWorld,
            Step::LoadLevel {
                locale: "exanima05",
                spawn: 0,
                teardown: true,
            },
            // The controls are attached to the reloaded player, which gives
            // them back after the death took them; the player lies down to
            // wake at once and plays the get-up motion where it is placed.
            Step::Controls(true),
            Step::KnockOut { delay_ms: 0 },
            Step::RequestAction(RESPAWN_MOTION),
            Step::PlacePlayer(placement),
            Step::Hook(Hook::Story),
        ]
    }

    /// The world, the game view and the flow's own session state go: the
    /// game layer's leave. The original does it only when its game object
    /// exists.
    pub fn leave_game_layer(&mut self) -> Vec<Step> {
        vec![Step::TeardownWorld, Step::GameGui(false)]
    }

    /// Builds and shows the main-menu form.
    pub fn show_menu_form(&mut self) -> Vec<Step> {
        self.menu_open = true;
        vec![Step::ShowMenu(self.menu_page())]
    }

    /// Hides the main-menu form.
    pub fn hide_menu_form(&mut self) -> Vec<Step> {
        self.menu_open = false;
        vec![Step::HideMenu]
    }

    /// Leaves the session, shows the main-menu form and plays the menu
    /// theme (the original's short form of the return to the title; it
    /// builds no page).
    pub fn leave_to_menu_form(&mut self) -> Vec<Step> {
        let mut steps = self.leave_session();
        self.phase = Phase::Menu;
        self.saved_phase = Phase::Menu;
        self.menu_open = true;
        let theme = menu_theme(self.alternative_music);
        steps.push(Step::ShowMenu(self.menu_page()));
        steps.push(Step::Music {
            name: theme.name,
            volume: theme.volume,
            restart: true,
        });
        steps
    }

    /// The story end menu's EXIT row: the game view and the dialogue close.
    /// A player the reconstruction device stripped clears its story bit and
    /// is saved without the death rule; any other story writes its save with
    /// the death rule, then leaves to the title behind the transition.
    pub fn story_exit(&mut self) -> Vec<Step> {
        let mut steps = vec![Step::GameGui(false), Step::Controls(false), Step::Dialogue];
        if self.story.reconstruction == STRIPPED {
            steps.push(Step::ClearStoryFlag);
            steps.push(Step::Save {
                drop_on_low_health: false,
            });
        } else {
            steps.push(Step::Save {
                drop_on_low_health: true,
            });
        }
        steps.extend(self.transition(Action::QuitToMenu, true));
        steps
    }

    /// Closes the end-of-session menu and the prelude: what the next-fight
    /// start and the level-load finish both do before anything else.
    pub fn close_end_menu(&mut self) -> Vec<Step> {
        self.end_menu = None;
        vec![Step::HideEndMenu, Step::ClosePrelude]
    }

    /// The level-load finish behind the transition: the end menu and the
    /// prelude close and the session is playing again. (The original also
    /// clears the pending exit words, saves the level's start state and runs
    /// the scene's own loaded callback, which are the host's.)
    pub fn level_loaded(&mut self) -> Vec<Step> {
        self.phase = Phase::Playing;
        self.close_end_menu()
    }

    /// The in-game page opened (F1): the session hook and the dialogue stop
    /// until it closes, as the original's frame task does.
    pub fn menu_opened(&mut self) {
        self.menu_open = true;
    }

    /// The in-game page closed: the hook and the dialogue run again.
    pub fn menu_closed(&mut self) {
        self.menu_open = false;
    }

    /// The unit's finalisation: the script hook is forgotten and its tables
    /// released. Exarust has no globals to tear down, so this is the step the
    /// host runs at shutdown.
    pub fn finalise(&mut self) -> Vec<Step> {
        self.custom_hook = false;
        vec![Step::Finalise]
    }

    /// The session begin's own work around the mode's starter: the frame
    /// task starts, the session phase becomes playing and the end menu's
    /// no-row-selected flag clears. The original runs all of this around the starter (the
    /// story's new game, the practice start or the arena's data init), which
    /// Exarust's own starters return as steps; the starter's steps come
    /// first here.
    pub fn begin_session(&mut self, mut starter: Vec<Step>) -> Vec<Step> {
        self.phase = Phase::Playing;
        // The begin itself does not touch the menu form: the starter's level
        // load hides it (see `Step::LoadLevel`).
        self.menu_open = false;
        self.end_menu = None;
        starter.push(Step::SessionTask(true));
        starter.push(Step::EndMenuUnselected(false));
        starter
    }

    /// The session continue's own work around the mode's continue: the menu
    /// form closes, the session phase becomes playing, the frame task starts
    /// and the end menu's no-row-selected flag clears. The mode's continue steps (the story's or the arena hub's)
    /// come first.
    pub fn continue_session(&mut self, mut resumed: Vec<Step>) -> Vec<Step> {
        self.phase = Phase::Playing;
        if self.menu_open {
            // The original hides the form before the mode's continue runs.
            self.menu_open = false;
            resumed.insert(0, Step::HideMenu);
        }
        resumed.push(Step::SessionTask(true));
        resumed.push(Step::EndMenuUnselected(false));
        resumed
    }

    /// The end-of-life story CONTINUE of a save with no record: the player
    /// data is written back, then the respawn runs behind the transition.
    pub fn continue_after_death(&mut self) -> Vec<Step> {
        // The record the write-back is handed carries 0x100 in its flags
        // word (the death screen's CONTINUE mode).
        let mut steps = vec![Step::PlayerData { flags: 0x100 }];
        steps.extend(self.transition(Action::StoryRespawn, true));
        steps
    }

    /// The application's quit callback, which the session begin installs:
    /// in the paused phase the arena unit's pause callback runs (whatever
    /// the mode); while playing, the game's interface closes and the session
    /// autosaves: a story always with the death rule (a dead player's save
    /// is dropped), an arena from its in-game page without it, or a forfeit
    /// from its pause page; practice writes nothing. Every playing quit then
    /// closes the save manager's package. `page` is the main menu's page.
    /// Nothing returns to the title: the application is closing.
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
            steps.push(Step::CloseSave);
        }
        steps
    }

    /// Back to the title: leave the session's world, hide the game, switch to
    /// the menu mode, show the menu form's title logo, build the title page,
    /// play its theme (before the page shows) and show it.
    pub fn quit_to_menu(&mut self) -> Vec<Step> {
        let mut steps = self.leave_session();
        // The switch to the title clears the reconstruction state and its
        // checkpoint record, as every mode switch does.
        self.story.reconstruction = 0;
        self.story.checkpoint = 0;
        self.mode = Mode::Menu;
        self.phase = Phase::Menu;
        self.saved_phase = Phase::Menu;
        self.hook = Hook::None;
        self.end_menu = None;
        self.menu_open = true;
        steps.push(Step::TitleLogo(true));
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
    /// interface go, the prelude closes, the world is torn down and the
    /// player pointers are cleared. A scripted mode's end hook is named.
    pub fn leave_session(&mut self) -> Vec<Step> {
        let mut steps = vec![
            Step::Hook(Hook::None),
            Step::SessionTask(false),
            Step::GameGui(false),
            Step::Dialogue,
            Step::ClosePrelude,
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
        // After the mode's end hook: the save package closes and the end
        // menu hides, then the world goes.
        steps.push(Step::CloseSave);
        steps.push(Step::HideEndMenu);
        steps.push(Step::TeardownWorld);
        self.player = [false; 3];
        self.end_menu = None;
        steps
    }

    /// The story level change (a trigger, a portal or a script). Nothing
    /// happens during a transition or for a level the locale table does not
    /// name. Otherwise a player whose condition has neither dead bit loses
    /// the controls, the game view goes, the save rule runs (every exit
    /// saves; early levels also keep a checkpoint) and the new level loads at
    /// the exit's spawn point behind the transition. The fade bit also gives
    /// the transition screen its long hold (a 2000 ms fade, a 1000 ms hold);
    /// `music_changes` is the story's level-entry music check for the
    /// destination ([`exa_audio::story_music_changes`]), which fades the
    /// music out with the screen.
    pub fn level_exit(
        &mut self,
        exit: LevelExit,
        rule: CheckpointRule,
        player_condition: u32,
        music_changes: bool,
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
            // The sixth level's checkpoint also takes the engine clock: the
            // level's own time, read back by the save.
            if exit.level == 6 {
                steps.push(Step::LevelTime);
            }
        }
        if exit.flags & 1 != 0 {
            steps.push(Step::FadeOut);
            steps.push(Step::AiHandover);
            self.long_transition = true;
        }
        steps.extend(self.transition(
            Action::Locale {
                locale,
                spawn: exit.spawn,
            },
            music_changes,
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
    pub fn leave_to_locale(&mut self, locale: &'static str, exists: bool) -> Vec<Step> {
        if !exists {
            return Vec::new();
        }
        self.transition(Action::Locale { locale, spawn: 0 }, false)
    }

    /// The class select's cancel: behind the transition (the music fading
    /// with it when the caller asks), the class select form is released, a
    /// scripted mode's end hook runs and the session leaves to the title.
    /// The original runs all of it as the transition's action, at black.
    pub fn cancel_class_select(&mut self, fade_audio: bool) -> Vec<Step> {
        self.transition(Action::ClassCancel, fade_audio)
    }

    /// The class cancel's action: the scripted mode's end hook, then back
    /// to the title.
    fn class_cancel(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        if let Mode::Other(word) = self.mode
            && word & SCRIPT_MODE_BIT != 0
        {
            steps.push(Step::ScriptMode {
                mode: word,
                hook: ScriptHook::End,
            });
        }
        steps.extend(self.quit_to_menu());
        steps
    }

    /// The class select's cancel form's final step: back to the title.
    pub fn cancel_to_menu(&mut self) -> Vec<Step> {
        self.quit_to_menu()
    }

    /// The class select's own cancel (not behind the transition): a scripted
    /// mode's end hook runs, the selection form closes and the flow goes
    /// straight back to the title.
    pub fn class_cancel_direct(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        if let Mode::Other(word) = self.mode
            && word & SCRIPT_MODE_BIT != 0
        {
            steps.push(Step::ScriptMode {
                mode: word,
                hook: ScriptHook::End,
            });
        }
        steps.extend(self.quit_to_menu());
        steps
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

    /// The end-of-session menu's confirm key: when the menu shows, its click
    /// sound (when it has one) plays at volume 0.06 and a random pitch of 0.8
    /// to 1.2, and the selected row is confirmed. `draw` is the process
    /// generator's number in [0, 1). A menu that is not showing does nothing.
    pub fn menu_confirm(&mut self, menu_visible: bool, has_sound: bool, draw: f64) -> Vec<Step> {
        if !menu_visible {
            return Vec::new();
        }
        let mut steps = Vec::new();
        if let Some(pitch) = hover_sound(has_sound, draw) {
            steps.push(Step::Hover {
                volume: HOVER_VOLUME,
                pitch,
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

/// The end menu's click sound's pitch when the menu has a sound: the draw
/// times 0.4 plus 0.8 in double precision, kept in single (0.8 to 1.2). The
/// sound is the one stored on the menu form.
pub fn hover_sound(has_sound: bool, draw: f64) -> Option<f32> {
    has_sound.then_some((draw * 0.4 + 0.8) as f32)
}

/// The volume the end menu's click sound plays at.
pub const HOVER_VOLUME: f32 = 0.06;

/// Whether an object id is the player's own: its record's id, its second id
/// or one of the ids in its record's item list. The practice scene filter
/// removes such objects from a level loaded around the player, so the level
/// does not hold a second copy of the player's things.
fn players_own(id: u32, player_id: u32, second_id: u32, item_ids: &[u32]) -> bool {
    id == player_id || id == second_id || item_ids.contains(&id)
}

/// The practice scene filter on a level's top node: a container node (item
/// type 0x11) always stays and has its children filtered by
/// [`child_stays`]; a node of the 0x4000 family is removed when its object
/// is the player's own; every other node stays.
pub fn node_stays(
    item_type: u32,
    id: u32,
    item_ids: &[u32],
    player_id: u32,
    second_id: u32,
) -> bool {
    if item_type & 0xfff == 0x11 {
        return true;
    }
    if item_type & 0x7000 == 0x4000 {
        return !players_own(id, player_id, second_id, item_ids);
    }
    true
}

/// The practice scene filter on a container's child: a child without an
/// object (id 0) stays; one whose object is the player's own is removed.
pub fn child_stays(id: u32, item_ids: &[u32], player_id: u32, second_id: u32) -> bool {
    id == 0 || !players_own(id, player_id, second_id, item_ids)
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
    // Only an ordered, strictly positive offset takes the positive branch:
    // a zero or unordered (NaN) offset takes the negated one.
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
            .init_resource::<FlowSteps>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<crate::save_live::SaveStep>()
            .init_resource::<GameView>()
            .init_resource::<BootToMenu>()
            .init_resource::<crate::transition_screen::TransitionOverlay>()
            .add_message::<MenuChoice>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_systems(Startup, boot)
            .add_systems(
                Update,
                (
                    apply_menu_choices,
                    tick,
                    follow_player,
                    travel,
                    run_transition,
                    apply_steps,
                    reset_level_state,
                )
                    .chain(),
            );
    }
}

/// Forgets the per-level state when the lifecycle tore a level down: the
/// navigation room cells (their entity ids belong to the old world) and the
/// hub entry (so the destination's hub runs its entry and places its
/// manager). The settled return placement survives into the hub's level.
fn reset_level_state(
    changes: Option<Res<exa_world::level_lifecycle::LevelChangeState>>,
    mut seen: Local<u64>,
    mut rooms: Option<ResMut<exa_ai::rooms::RoomCells>>,
    mut hub: Option<ResMut<crate::arena_hub_live::ArenaHub>>,
) {
    let Some(changes) = changes else {
        return;
    };
    if changes.changes == *seen {
        return;
    }
    *seen = changes.changes;
    if let Some(rooms) = rooms.as_mut() {
        **rooms = exa_ai::rooms::RoomCells::default();
    }
    if let Some(hub) = hub.as_mut() {
        hub.level_changed();
    }
}

fn boot(
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    show_title: Option<Res<BootToMenu>>,
    unlocks: Option<Res<UnlockSettings>>,
) {
    if let Some(unlocks) = unlocks.as_deref() {
        flow.unlocks.apply_settings(*unlocks);
    }
    for step in flow.boot() {
        debug!("game flow: {step:?}");
        if show_title.as_deref().is_some_and(|show| !show.0)
            && matches!(step, Step::ShowMenu(Page::Title))
        {
            continue;
        }
        queued.steps.push(step);
    }
}

/// The game clock the flow runs on is the fixed step's (12 ms a tick); the
/// player and the other characters' conditions come from the running world.
#[allow(clippy::type_complexity)]
fn tick(
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    players: Query<
        (
            Option<&exa_combat::Condition>,
            Option<&exa_combat::Fighter>,
            Option<&exa_combat::StateFlags>,
        ),
        With<crate::player::PlayerBody>,
    >,
    others: Query<&exa_combat::Condition, Without<crate::player::PlayerBody>>,
) {
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    let player = players
        .single()
        .ok()
        .map(|(condition, fighter, flags)| PlayerReading {
            condition: condition.map_or(0, |condition| condition.0),
            animator: flags.map_or(0, |flags| flags.0),
            // A body without a fighter record is treated as unhurt.
            health: fighter.map_or(0.25, |fighter| fighter.health.red),
        });
    let others: Vec<u32> = others.iter().map(|condition| condition.0).collect();
    for step in flow.frame(now, player, &others) {
        trace!("game flow: {step:?}");
        queued.steps.push(step);
    }
}

/// A play level's player starts a practice-mode session (the sandbox is
/// Exarust's stand-in for one; the debug views have no session flow of the
/// original's). The practice round hook is not installed: a directly
/// launched level is not a practice arena, and its corpses or any fallen
/// character would end a "round" at once. A practice started from the menu
/// installs the hook itself.
fn follow_player(
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    menu: Option<Res<MenuScreen>>,
    players: Query<(), Added<crate::player::PlayerBody>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut clock: Option<MessageWriter<crate::world_clock::RestoreWorldClock>>,
) {
    if players.is_empty() || flow.mode.is_game() {
        return;
    }
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    for step in flow.switch_mode(Mode::Practice, now) {
        debug!("game flow: {step:?}");
        queued.steps.push(step);
    }
    // A fresh session's world time starts at its load: the original's clock
    // counts the saved seconds (zero here) plus the ticks since the load, so
    // the splash and the menu do not age the world.
    if let Some(clock) = clock.as_mut() {
        clock.write(crate::world_clock::RestoreWorldClock { saved_seconds: 0 });
    }
    // A title the splash put up stays over the level until it is closed.
    flow.menu_open = menu.is_some_and(|menu| menu.open);
    flow.phase = Phase::Playing;
    flow.hook = Hook::None;
    flow.practice.start_ms = now;
    debug!("game flow: a play level is running as a practice-mode session");
}

/// A teleporter journey is the game's level exit. The destination's
/// level-entry music check reads the track the music's main voice plays.
fn travel(
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    mut journeys: Option<MessageReader<crate::destination::TravelToLevel>>,
    audio: Option<Res<exa_audio::AudioEnvironment>>,
) {
    let Some(journeys) = journeys.as_mut() else {
        return;
    };
    let playing = audio.as_ref().map_or("", |audio| audio.music_name());
    for journey in journeys.read() {
        let music_changes = level_locale(journey.level).is_some_and(|locale| {
            exa_audio::story_music_changes_for_key(locale.as_bytes(), playing)
        });
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
            music_changes,
        );
        for step in steps {
            info!(
                "game flow: {} asks for level {} (spawn {}): {step:?}",
                journey.user, journey.level, journey.arrival
            );
            queued.steps.push(step);
        }
    }
}

/// Runs the transition screen once a frame on the real clock, whatever
/// covers the game (the original runs it as a GUI task). Its action's steps
/// join the frame's; its music fade, music stop and audio fades go to the
/// audio environment; its alpha byte to [`crate::transition_screen`]. A
/// level change counts as in progress from its teardown to the end of its
/// loading screen.
#[allow(clippy::too_many_arguments)]
fn run_transition(
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    mut overlay: ResMut<crate::transition_screen::TransitionOverlay>,
    time: Res<Time<Real>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut audio: Option<ResMut<exa_audio::AudioEnvironment>>,
    mut bank: Option<ResMut<exa_audio::SoundBank>>,
    loading: (
        Option<Res<exa_ui::LoadingStatus>>,
        Option<Res<exa_world::level_lifecycle::LevelChangeReady>>,
    ),
) {
    if !flow.transition_running() {
        if overlay.0 != 0 {
            overlay.0 = 0;
        }
        return;
    }
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    let input = TransitionInput {
        clock_ms: crate::splash::clock_ms(&time),
        music: audio.as_ref().and_then(|audio| audio.music_gain()),
        audio_fade: audio.as_ref().map_or(1.0, |audio| audio.buses.transition),
        loading: loading.0.is_some() || loading.1.is_some(),
    };
    let frame = flow.transition_tick(input, now);
    if let Some(audio) = audio.as_mut() {
        if let Some(gain) = frame.music {
            audio.set_music_gain(gain);
        }
        if frame.stop_music
            && let Some(bank) = bank.as_mut()
        {
            audio.music_stop(bank, true);
        }
        if let Some(fade) = frame.audio_fade {
            audio.buses.transition = fade;
        }
        if frame.world_fade_reset {
            audio.buses.world_fade = 1.0;
        }
    }
    overlay.0 = frame.alpha;
    for step in &frame.steps {
        debug!("game flow: transition action: {step:?}");
    }
    queued.steps.extend(frame.steps);
}

/// Runs the commands the main menu handed over. The menu's own page changes
/// and the resume already happened in [`crate::menu_screen`]; the session,
/// restart and leave work below turns into the same [`Step`]s the flow's
/// methods return and is applied by [`apply_steps`]. The creator's rows
/// belong to [`crate::creator_screen`] and the saves' character screen (the
/// CONTINUE rows) to [`crate::save_screen`]; the manual is not wired yet.
fn apply_menu_choices(
    mut choices: MessageReader<MenuChoice>,
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    arena_match: Option<Res<crate::arena_match::ArenaMatch>>,
) {
    for MenuChoice(command) in choices.read() {
        let steps = match command {
            MenuCommand::Resume => {
                if flow.phase == Phase::Paused {
                    flow.resume()
                } else {
                    flow.menu_closed();
                    vec![Step::Controls(true)]
                }
            }
            // The creator opens for these ([`crate::creator_screen`]): the
            // practice and versus characters and the arena's new career at
            // the free creator, the story's BEGIN at the background picker.
            // Its commit starts the session.
            MenuCommand::NewPractice
            | MenuCommand::Practice { .. }
            | MenuCommand::BeginStory
            | MenuCommand::NewCareer
            | MenuCommand::Versus => Vec::new(),
            MenuCommand::ExitArena | MenuCommand::ExitToTitle => flow.quit_to_menu(),
            MenuCommand::QuitStory => flow.story_exit(),
            MenuCommand::FinishMatch { forfeit } => {
                if flow.mode == Mode::Story {
                    flow.story_exit()
                } else if *forfeit && arena_match.is_some() {
                    // The arena pause page's FORFEIT: the live match settles
                    // and returns to the hub. The end page's EXIT, a won
                    // session's row, still leaves to the title.
                    vec![Step::HideMenu, Step::Controls(true), Step::Forfeit]
                } else {
                    flow.quit_to_menu()
                }
            }
            // The saves' character screen opens for these
            // ([`crate::save_screen`]); its CONTINUE loads the slot.
            MenuCommand::CharacterScreen(_) => Vec::new(),
            MenuCommand::None | MenuCommand::ShowPage(_) | MenuCommand::Quit => Vec::new(),
            MenuCommand::OpenManual | MenuCommand::OpenSettings => {
                warn!("main menu command {command:?} is not wired");
                Vec::new()
            }
        };
        for step in &steps {
            debug!("game flow: menu {command:?}: {step:?}");
        }
        queued.steps.extend(steps);
    }
}

/// Applies the flow's steps to the running app's own pieces: the menu and
/// end-menu forms, the title logo, the interface and controls flags and the
/// practice heal's health. A level load or world teardown becomes an
/// [`exa_world::level_lifecycle::LevelChangeRequest`] and a player placement
/// a [`crate::level_change::PlayerPlacement`]; the camera and save are
/// logged and left to the host. The transition screen runs in
/// [`run_transition`].
#[allow(clippy::too_many_arguments)]
fn apply_steps(
    mut commands: Commands,
    mut queued: ResMut<FlowSteps>,
    flow: Res<Flow>,
    mut menu: Option<ResMut<MenuScreen>>,
    mut end: Option<ResMut<EndMenuScreen>>,
    mut logo: Option<ResMut<TitleLogo>>,
    mut view: Option<ResMut<GameView>>,
    mut session: Option<ResMut<crate::player_controls::PlaySession>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut players: Query<PlayerParts, With<crate::player::PlayerBody>>,
    mut cameras: Query<&mut crate::player::ChaseCamera>,
    mut changes: MessageWriter<exa_world::level_lifecycle::LevelChangeRequest>,
    mut forfeit: MessageWriter<ForfeitMatch>,
    mut settings: MessageWriter<SettingsEntry>,
    mut exit: MessageWriter<AppExit>,
    mut saves: MessageWriter<crate::save_live::SaveStep>,
) {
    if queued.steps.is_empty() {
        return;
    }
    let steps = core::mem::take(&mut queued.steps);
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    // A knock-out written on a body that did not carry one yet: its insert
    // is deferred, so a later wake write in the batch changes this copy.
    let mut fresh_knock: Option<(Entity, exa_combat::condition::KnockOut)> = None;
    // A teardown in the same batch as a load is the load's own; one request
    // does both, so the destination is not torn down twice.
    let loads = steps
        .iter()
        .any(|step| matches!(step, Step::LoadLevel { .. }));
    for step in steps {
        match step {
            Step::ShowMenu(page) => {
                if let Some(menu) = menu.as_mut() {
                    menu.game = flow.mode.word();
                    menu.session = flow.phase.word();
                    menu.state = flow.page_state();
                    menu.open_at(page);
                }
            }
            Step::HideMenu => {
                if let Some(menu) = menu.as_mut() {
                    menu.close();
                }
            }
            Step::EndMenu(end_menu) => {
                if let Some(end) = end.as_mut() {
                    end.show(end_menu);
                }
            }
            Step::HideEndMenu => {
                if let Some(end) = end.as_mut() {
                    end.close();
                }
            }
            Step::LoadLevel {
                locale,
                spawn,
                teardown,
            } => {
                // The level loader's first work is the app's: it hides the
                // menu form and closes the end menu and the prelude. The
                // change itself is the lifecycle's; the host loads the level
                // through the same stages as a start-up load.
                if let Some(menu) = menu.as_mut() {
                    menu.close();
                }
                if let Some(end) = end.as_mut() {
                    end.close();
                }
                crate::level_change::request(&mut changes, Some(locale), spawn, teardown);
            }
            Step::TeardownWorld => {
                // A load in this batch tears down by itself.
                if !loads {
                    crate::level_change::request(&mut changes, None, 0, true);
                }
            }
            Step::PlacePlayer(placement) => {
                commands.insert_resource(crate::level_change::PlayerPlacement::of_flow(&placement));
            }
            Step::EndMenuUnselected(unselected) => {
                if let Some(end) = end.as_mut() {
                    end.unselected = unselected;
                    if end.open {
                        end.selected = if unselected { None } else { Some(0) };
                    }
                }
            }
            Step::MenuHelp(help) => {
                if let Some(menu) = menu.as_mut() {
                    menu.set_covered(help);
                }
            }
            Step::TitleLogo(shown) => {
                if let Some(logo) = logo.as_mut() {
                    logo.0 = shown;
                }
            }
            Step::GameGui(shown) => {
                if let Some(view) = view.as_mut() {
                    view.0 = shown;
                }
            }
            Step::Controls(on) => {
                if let Some(session) = session.as_mut() {
                    session.playing = on;
                }
            }
            Step::Camera(frame) => {
                // The shot turns the chase camera to its bearing; the
                // original's view position, forced speed and the cursor
                // warp are not taken (the chase camera keeps its distance
                // and pitch rules).
                for mut camera in &mut cameras {
                    camera.face_shot(frame.camera, frame.target);
                }
            }
            Step::CombatMode(on) => {
                for mut player in &mut players {
                    player.body.set_combat(on);
                }
            }
            Step::Health {
                red_bits,
                yellow_bits,
            } => {
                for player in &mut players {
                    if let Some(mut fighter) = player.fighter {
                        fighter.health.red = f32::from_bits(red_bits);
                        fighter.health.yellow = f32::from_bits(yellow_bits);
                    }
                }
            }
            Step::Condition(word) => {
                for player in &mut players {
                    if let Some(mut condition) = player.condition {
                        condition.0 = word;
                    }
                }
            }
            Step::KnockOut { delay_ms } => {
                for mut player in &mut players {
                    let mut condition = player.condition.as_deref().copied().unwrap_or_default();
                    let mut flags = player.flags.as_deref().copied().unwrap_or_default();
                    let knock =
                        exa_combat::condition::knock_out(&mut condition, &mut flags, now, delay_ms);
                    if player.knock.is_none() {
                        fresh_knock = Some((player.entity, knock));
                    }
                    write_player(&mut commands, &mut player, condition, flags, knock);
                    if let Some(body) = player.ragdoll.as_mut() {
                        body.knock_out();
                    }
                }
            }
            Step::ReviveBody => {
                for mut player in &mut players {
                    let mut condition = player.condition.as_deref().copied().unwrap_or_default();
                    let mut flags = player.flags.as_deref().copied().unwrap_or_default();
                    let red = player.fighter.as_ref().map_or(0.0, |f| f.health.red);
                    // The record's weapon slots are left as they are (the
                    // hands' re-arm is not part of the story revive here).
                    let Some(revival) =
                        exa_combat::condition::revive(&mut condition, &mut flags, red, false, now)
                    else {
                        continue;
                    };
                    player.body.set_combat(false);
                    if let Some(fighter) = player.fighter.as_mut() {
                        fighter.health.yellow = revival.yellow;
                    }
                    if player.knock.is_none() {
                        fresh_knock = Some((player.entity, revival.knock));
                    }
                    write_player(&mut commands, &mut player, condition, flags, revival.knock);
                    if let Some(body) = player.ragdoll.as_mut() {
                        if body.drive.is_some() {
                            body.knock_out();
                        } else {
                            // A limp body gets its muscles back over the
                            // race and stats it had (`crate::rise` gives
                            // its clips and controller back).
                            let drive = body.revival_drive().unwrap_or_default();
                            body.revive(drive);
                        }
                    }
                }
            }
            Step::Wake(pose) => {
                for mut player in &mut players {
                    let current = player.flags.as_deref().copied().unwrap_or_default();
                    let flags = if pose.or_flags {
                        current.0 | pose.flags
                    } else {
                        pose.flags
                    };
                    match player.flags.as_mut() {
                        Some(word) => word.0 = flags,
                        None => {
                            commands
                                .entity(player.entity)
                                .insert(exa_combat::StateFlags(flags));
                        }
                    }
                    let wake_write = |knock: &mut exa_combat::condition::KnockOut| {
                        if let Some(ready) = pose.blend {
                            knock.ready = ready;
                        }
                        if let Some(clock) = pose.clock {
                            knock.wake_at_ms = clock.applied(knock.wake_at_ms, now);
                        }
                    };
                    if let Some(knock) = player.knock.as_mut() {
                        wake_write(knock);
                    } else if let Some((entity, knock)) = fresh_knock.as_mut()
                        && *entity == player.entity
                    {
                        wake_write(knock);
                        commands.entity(*entity).insert(*knock);
                    }
                    if let Some(drive) = player.ragdoll.as_mut().and_then(|b| b.drive.as_mut()) {
                        if let Some(stance) = pose.move_factor {
                            drive.set_stance(stance);
                        }
                        if let Some(tone) = pose.turn_factor {
                            drive.tone = tone;
                        }
                    }
                }
            }
            Step::StoryPose {
                condition,
                flags,
                turn_factor,
            } => {
                for mut player in &mut players {
                    if let (Some(word), Some(current)) = (condition, player.condition.as_mut()) {
                        current.0 = word;
                    }
                    if let Some(word) = flags {
                        match player.flags.as_mut() {
                            Some(current) => current.0 = word,
                            None => {
                                commands
                                    .entity(player.entity)
                                    .insert(exa_combat::StateFlags(word));
                            }
                        }
                    }
                    if let (Some(tone), Some(drive)) = (
                        turn_factor,
                        player.ragdoll.as_mut().and_then(|b| b.drive.as_mut()),
                    ) {
                        drive.tone = tone;
                    }
                }
            }
            Step::Music {
                name,
                volume,
                restart,
            } => {
                commands.insert_resource(exa_audio::LevelMusic(name.to_ascii_lowercase()));
                commands.insert_resource(exa_audio::LevelMusicVolume(volume));
                if restart {
                    commands.insert_resource(exa_audio::LevelMusicStart(0.0));
                }
            }
            Step::Forfeit => {
                forfeit.write(ForfeitMatch);
            }
            Step::SettingsWrite { key, bytes } => {
                settings.write(SettingsEntry {
                    section: SETTINGS_SECTION,
                    key,
                    bytes,
                });
            }
            Step::Quit => {
                exit.write(AppExit::Success);
            }
            // The save unit's work on the current slot
            // ([`crate::save_live`], idle without a save folder).
            Step::Save { drop_on_low_health } => {
                saves.write(crate::save_live::SaveStep::Save { drop_on_low_health });
            }
            Step::DropSave => {
                saves.write(crate::save_live::SaveStep::Drop);
            }
            Step::StripPlayer => {
                // The reconstruction device's strip: the player's live body
                // loses its hands and worn data, keeps the underwear rule and
                // is left at just under half health (the record's 0x7FFF).
                for player in &mut players {
                    if let Some(mut fighter) = player.fighter {
                        crate::teleporters::strip_body(&mut commands, player.entity, &mut fighter);
                    }
                }
            }
            Step::Checkpoint => {
                saves.write(crate::save_live::SaveStep::Checkpoint);
            }
            other => {
                debug!("game flow step not applied by the app: {other:?}");
            }
        }
    }
}

/// The player body's parts the flow's steps write.
#[derive(bevy::ecs::query::QueryData)]
#[query_data(mutable)]
struct PlayerParts {
    entity: Entity,
    body: &'static mut crate::player::PlayerBody,
    condition: Option<&'static mut exa_combat::Condition>,
    flags: Option<&'static mut exa_combat::StateFlags>,
    knock: Option<&'static mut exa_combat::condition::KnockOut>,
    fighter: Option<&'static mut exa_combat::Fighter>,
    ragdoll: Option<&'static mut exa_physics::body::Ragdoll>,
}

/// Writes a knock-out's condition, flags and wake timer on the player,
/// inserting what the body does not carry yet.
fn write_player(
    commands: &mut Commands,
    player: &mut PlayerPartsItem,
    condition: exa_combat::Condition,
    flags: exa_combat::StateFlags,
    knock: exa_combat::condition::KnockOut,
) {
    let mut entity = commands.entity(player.entity);
    match player.condition.as_mut() {
        Some(current) => **current = condition,
        None => {
            entity.insert(condition);
        }
    }
    match player.flags.as_mut() {
        Some(current) => **current = flags,
        None => {
            entity.insert(flags);
        }
    }
    match player.knock.as_mut() {
        Some(current) => **current = knock,
        None => {
            entity.insert(knock);
        }
    }
}

/// Runs a transition screen to black (its first frame at clock 0, the
/// next 3 s on, past every fade) and returns its action's steps, run at the
/// game clock `now`.
#[cfg(test)]
pub(crate) fn run_to_black_at(flow: &mut Flow, now: u32) -> Vec<Step> {
    let input = |clock_ms| TransitionInput {
        clock_ms,
        music: None,
        audio_fade: 1.0,
        loading: false,
    };
    let first = flow.transition_tick(input(0), now);
    assert!(first.steps.is_empty(), "the action runs only at black");
    flow.transition_tick(input(3000), now).steps
}

/// [`run_to_black_at`] at game clock 0.
#[cfg(test)]
pub(crate) fn run_to_black(flow: &mut Flow) -> Vec<Step> {
    run_to_black_at(flow, 0)
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
        // The knight and proctor hold item 0x1f in the off hand and carry it
        // with flags 0x100 and the default third word (0x28, 4); their roles
        // are only 7, 1 and their own.
        assert_eq!(knight.roles, &[7, 1, 0x1b]);
        assert_eq!(knight.off_hand, Some(ItemRef { id: 0x1f, flags: 0 }));
        assert_eq!(
            knight.carried,
            Some(Carried {
                item: 0x1f,
                flags: 0x100,
                extra: 0x0004_0028,
            })
        );
        assert_eq!(knight.powers, None);
        let proctor = story_start(Background::Proctor, 1.0);
        assert_eq!(proctor.placement, knight.placement);
        assert_eq!(proctor.carried, knight.carried);
        let villager = story_start(Background::Villager, 1.0);
        assert_eq!(villager.role, 0x18);
        assert_eq!(villager.flags, 0x600);
        assert_eq!(villager.condition, Some(0x600));
        assert_eq!(villager.animator, Some(0x600));
        assert_eq!(villager.turn, Some(0.45));
        assert_eq!(knight.condition, None);
        assert_eq!(knight.turn, Some(1.0));
        assert_eq!(unknown.condition, None);
        assert_eq!(unknown.turn, None);
        assert_eq!(villager.placement.position.y, 300.0);
        assert_ne!(villager.placement.rotation, NO_ROTATION);
        let villager_light = villager.light.unwrap();
        assert_eq!(villager_light.position, Vec3::new(8000.0, 500.0, 4000.0));
        assert_eq!(
            villager_light.rotation_degrees,
            Vec3::new(8.0, -150.0, 50.0)
        );
        assert_eq!(practice_height(90.0), 95.0);
        // The unknown spawn height follows the body's height.
        assert_eq!(
            story_start(Background::Unknown, 1.8).placement.position.y,
            34.0
        );
        // Every story start hands the loader the same spawn point and view,
        // and the start record carries the background's wake motion.
        for (background, motion) in [
            (Background::Unknown, START_MOTION_ANIMATION),
            (Background::Knight, START_MOTION_WAKING),
            (Background::Proctor, START_MOTION_WAKING),
            (Background::Villager, START_MOTION_VILLAGER),
        ] {
            let start = story_start(background, 1.0);
            assert_eq!(start.spawn_point, Vec3::new(8200.0, 25.0, 3920.0));
            assert_eq!(start.view, STORY_VIEW);
            assert_eq!(start.view.direction, Vec3::new(0.0, 0.0, 1.0));
            assert_eq!(start.view.target, Vec3::new(8200.0, 0.0, 4000.0));
            assert_eq!(start.start_record, [motion, 0, 0]);
        }
        let start = story_start(Background::Knight, 1.0);
        // The start record is the bare motion, not the spawn pose.
        assert_eq!(start.placement.position, Vec3::new(8200.0, 210.0, 3920.0));
    }

    #[test]
    fn a_practice_start_looks_at_the_ring_and_carries_its_view() {
        let mut flow = Flow::default();
        let viewport = Viewport {
            width: 1000,
            height: 500,
        };
        let steps = flow.start_practice(Ladder::Novice, 90.0, viewport, 0);
        let start = PracticeStart {
            ladder: Ladder::Novice,
            fight: FIRST_FIGHT,
            partner: *b"opponent01",
            boss: false,
            locale: "arena",
            music: practice_music(Ladder::Novice, false),
            view: PRACTICE_VIEW,
        };
        assert_eq!(start.view.direction, Vec3::new(0.0, 0.0, -1.0));
        assert_eq!(start.view.target, Vec3::new(0.0, 20.0, 500.0));
        assert!(steps.contains(&Step::PlacePlayer(Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(0.0, 95.0, 0.0),
        })));
        assert!(steps.contains(&Step::Camera(camera::frame_point(
            PRACTICE_POINT,
            true,
            false,
            viewport
        ))));
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
        assert!(arena.unselected);
        assert!(!won.unselected);
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
                    animator: 0,
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
            animator: 0,
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
        flow.stand_height = 88.5;
        let steps = flow.practice_next(9000);
        // The body stands at its measured stand height, over the ring's
        // middle; the shot still looks 5 m ahead at the partner's place.
        assert!(steps.contains(&Step::PlacePlayer(Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(0.0, 88.5, 0.0),
        })));
        assert_eq!(flow.practice.fight, 8);
        assert_eq!(flow.phase, Phase::Playing);
        // The arena is imported again through the practice filter.
        let load = steps
            .iter()
            .position(|step| {
                *step
                    == Step::LoadLevel {
                        locale: "arenae",
                        spawn: 0,
                        teardown: true,
                    }
            })
            .expect("the arena reloads");
        assert_eq!(steps[load + 1], Step::PracticeFilter);
        // The fight turns combat mode on; the controls are not touched.
        assert!(steps.contains(&Step::CombatMode(true)));
        assert!(!steps.iter().any(|step| matches!(step, Step::Controls(_))));
        assert!(steps.contains(&Step::HideEndMenu));
        // 0.2f32 as a double plus a tenth of 0.25, rounded to single:
        // 0x3e666667.
        let healed = (f64::from(0.2f32) + 0.25 * 0.1) as f32;
        assert!(steps.contains(&Step::Health {
            red_bits: healed.to_bits(),
            yellow_bits: healed.to_bits()
        }));
        assert_eq!(healed.to_bits(), 0x3e66_6667);
        // The round's condition word clears before the motion starts.
        assert!(steps.contains(&Step::Condition(0)));
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
            animator: 0,
            health: 0.1,
        };
        let steps = flow.story_tick(1000, Some(dead));
        assert_eq!(flow.hook, Hook::Knockout);
        assert!(steps.contains(&Step::Tip(2)));
        assert!(flow.knockout_tick(5000, Some(dead)).is_empty());
        let steps = flow.knockout_tick(6001, Some(dead));
        assert_eq!(flow.phase, Phase::Transition);
        assert!(steps.contains(&Step::Transition));
        let steps = run_to_black_at(&mut flow, 6101);
        assert_eq!(flow.phase, Phase::Playing);
        assert_eq!(flow.hook, Hook::Story);
        assert!(
            steps
                .iter()
                .any(|step| matches!(step, Step::Wake(WakePose { flags: 0x40, .. })))
        );
        assert_eq!(flow.story.wake.start_ms, 8101);
        // The revival closes the prelude, then saves the story.
        assert_eq!(steps.first(), Some(&Step::ClosePrelude));
        assert_eq!(
            steps.get(1),
            Some(&Step::Save {
                drop_on_low_health: false
            })
        );
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
                animator: 0,
                health: 0.0,
            }),
        );
        assert_eq!(flow.phase, Phase::Ended);
        assert!(steps.contains(&Step::Tip(3)));
    }

    #[test]
    fn the_wake_animation_ends_on_the_animator_flag_not_the_condition() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            hook: Hook::Story,
            ..Default::default()
        };
        flow.story.wake.animating = true;
        // The animator's 0x20 keeps the first animation running even with a
        // clear condition word.
        let animator_waking = PlayerReading {
            condition: 0,
            animator: WAKING,
            health: 0.2,
        };
        flow.story_tick(1000, Some(animator_waking));
        assert!(flow.story.wake.animating);
        // Once the animator flags drop the bit the animation is done; the
        // condition's 0x20 (the waking bit) does not keep it running.
        let condition_waking = PlayerReading {
            condition: WAKING,
            animator: 0,
            health: 0.2,
        };
        flow.story_tick(2000, Some(condition_waking));
        assert!(!flow.story.wake.animating);
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
            animator: 0,
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
            Step::Wake(WakePose { turn_factor: Some(turn), flags: 0x600, or_flags: false, .. }) if *turn == 0.49
        )));
        let steps = flow.story_tick(10_501, Some(alive));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Wake(WakePose { turn_factor: Some(turn), flags: 0x20, or_flags: false, .. }) if *turn == 0.4
        )));
        assert!(!flow.story.wake.getting_up);
        // The two writes are the original's whole animator write: the held
        // pose is flags 0x600, blend 1, move and turn 0.49; the finished one
        // is flags 0x20, blend 0.4, turn 0.4 and the clock at the motion's
        // value less 1490 ms.
        assert_eq!(
            WakePose::hold(),
            WakePose {
                flags: 0x600,
                or_flags: false,
                blend: Some(1.0),
                move_factor: Some(0.49),
                turn_factor: Some(0.49),
                clock: None,
            }
        );
        assert_eq!(
            WakePose::finish(),
            WakePose {
                flags: 0x20,
                or_flags: false,
                blend: Some(0.4),
                move_factor: None,
                turn_factor: Some(0.4),
                clock: Some(ClipClock::MotionMinus(1490)),
            }
        );
        assert_eq!(
            WakePose::revived(),
            WakePose {
                flags: 0x40,
                or_flags: true,
                blend: None,
                move_factor: None,
                turn_factor: None,
                clock: Some(ClipClock::Push(2000)),
            }
        );
    }

    #[test]
    fn the_revive_closes_the_prelude_and_wakes_the_player() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Ended,
            hook: Hook::Knockout,
            ..Default::default()
        };
        let steps = flow.revive(4000);
        assert!(steps.contains(&Step::ClosePrelude));
        assert!(steps.contains(&Step::Save {
            drop_on_low_health: false
        }));
        assert!(steps.contains(&Step::Controls(true)));
        assert!(steps.contains(&Step::GameGui(true)));
        assert!(steps.contains(&Step::Wake(WakePose::revived())));
        assert!(steps.contains(&Step::Hook(Hook::Story)));
        // Combat mode off, the body raised, then its held wake, then the
        // controls (the combat-mode setter's 0 is not the controls').
        let at = |wanted: Step| steps.iter().position(|step| *step == wanted).unwrap();
        assert!(at(Step::CombatMode(false)) < at(Step::ReviveBody));
        assert!(at(Step::ReviveBody) < at(Step::Wake(WakePose::revived())));
        assert!(at(Step::Wake(WakePose::revived())) < at(Step::Controls(true)));
        assert_eq!(flow.phase, Phase::Playing);
        // The wake-up animation is watched from two seconds on.
        assert!(flow.story.wake.animating);
        assert!(!flow.story.wake.getting_up);
        assert!(!flow.story.wake.delayed);
        assert_eq!(flow.story.wake.start_ms, 6000);
    }

    #[test]
    fn the_death_screens_continue_writes_the_player_back_first() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Ended,
            ..Default::default()
        };
        let steps = flow.continue_after_death();
        assert_eq!(steps[0], Step::PlayerData { flags: 0x100 });
        assert!(steps.contains(&Step::Transition));
        assert_eq!(flow.phase, Phase::Transition);
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
        // A NaN health takes the alive branch: tip 2, no drop and no strip,
        // the plain end menu.
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
        let steps = flow.die(f32::NAN);
        assert!(steps.contains(&Step::Tip(2)));
        assert!(!steps.contains(&Step::DropSave));
        assert!(!steps.contains(&Step::StripPlayer));
        assert!(steps.contains(&Step::EndMenu(end_menu(MENU_WON, Mode::Story))));
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
        // The sixth level's checkpoint takes the level's time; an earlier
        // checkpoint does not.
        let mut sixth = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = sixth.level_exit(
            LevelExit {
                level: 6,
                spawn: 0,
                flags: 0,
            },
            CheckpointRule {
                level: 6,
                setting: false,
                already_saved: false,
            },
            0,
            false,
        );
        assert!(steps.contains(&Step::Checkpoint));
        assert!(steps.contains(&Step::LevelTime));
        let mut third = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = third.level_exit(
            LevelExit {
                level: 3,
                spawn: 0,
                flags: 0,
            },
            rule,
            0,
            false,
        );
        assert!(steps.contains(&Step::Checkpoint));
        assert!(!steps.contains(&Step::LevelTime));
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
        assert!(flow.level_exit(none, rule, 0, false).is_empty());
        let steps = flow.level_exit(
            LevelExit {
                level: 3,
                spawn: 4,
                flags: 1,
            },
            rule,
            0,
            false,
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
                Step::FadeOut,
                Step::AiHandover,
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
        assert!(flow.level_exit(next, rule, 0, false).is_empty());
        // The level loads at the exit's spawn point when the screen is black.
        let steps = run_to_black(&mut flow);
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
        let steps = flow.level_exit(next, held, DEAD, false);
        assert_eq!(steps[0], Step::GameGui(false));
        assert!(!steps.contains(&Step::Checkpoint));
        // Without the fade bit nothing fades or hands over.
        assert!(!steps.contains(&Step::FadeOut));
        assert!(!steps.contains(&Step::AiHandover));
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
        // A zero offset takes the negative branch: -(50 + 75).
        let nudge = exit_nudge(Vec3::ZERO, trigger).unwrap();
        assert_eq!((nudge.axis, nudge.distance), (2, -125.0));
        assert_eq!(nudge.position, Vec3::new(0.0, 0.0, -125.0));
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
    fn the_node_filter_removes_the_players_own_objects() {
        // Nodes of other item types are never filtered.
        assert!(node_stays(0, 5, &[6, 7], 100, 0x6c));
        assert!(node_stays(0, 100, &[6, 7], 100, 0x6c));
        assert!(node_stays(0x2000, 100, &[], 100, 0x6c));
        // The 0x4000 family loses the player's own objects.
        assert!(node_stays(0x4000, 5, &[6, 7], 100, 0x6c));
        assert!(!node_stays(0x4000, 100, &[6, 7], 100, 0x6c));
        assert!(!node_stays(0x4000, 0x6c, &[6, 7], 100, 0x6c));
        assert!(!node_stays(0x4000, 7, &[6, 7], 100, 0x6c));
        // A container always stays; its children are filtered, and a child
        // without an object stays.
        assert!(node_stays(0x11, 100, &[6], 100, 0x6c));
        assert!(node_stays(0x3011, 6, &[6], 100, 0x6c));
        assert!(!child_stays(6, &[6], 100, 0x6c));
        assert!(!child_stays(100, &[], 100, 0x6c));
        assert!(child_stays(1, &[6], 100, 0x6c));
        assert!(child_stays(0, &[0], 0, 0));
    }

    #[test]
    fn the_flow_boots_into_the_title_and_switches_modes() {
        let mut flow = Flow::default();
        let steps = flow.boot();
        assert!(steps.contains(&Step::Boot(BootTask::Splash)));
        assert!(steps.contains(&Step::ShowMenu(Page::Title)));
        // The boot's order: the script hooks first, then graphics and the
        // first-run settings, the interfaces, the hardware and menu hooks,
        // the settings and experience, the two save managers, the transition
        // screen, the arena data, the audio, the splash and the title.
        let order: Vec<usize> = [
            BootTask::Scripts,
            BootTask::Window,
            BootTask::FirstRun,
            BootTask::ActorLibrary,
            BootTask::GameGui,
            BootTask::ItemGui,
            BootTask::SkillGui,
            BootTask::HardwareConfig,
            BootTask::MenuTask,
            BootTask::Settings,
            BootTask::Experience,
            BootTask::SaveManager(Mode::Story),
            BootTask::SaveManager(Mode::Arena),
            BootTask::Transition,
            BootTask::ArenaData,
            BootTask::Audio,
            BootTask::Splash,
            BootTask::MainMenu,
        ]
        .into_iter()
        .map(|task| {
            steps
                .iter()
                .position(|step| *step == Step::Boot(task))
                .unwrap()
        })
        .collect();
        assert!(order.windows(2).all(|pair| pair[0] < pair[1]));
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
        assert!(!flow.page_state().new_game);
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
    fn a_novice_win_writes_the_expert_unlock_once() {
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
        let steps = flow.practice_tick(6000, None, &[DEAD]);
        // The novice ladder's end: tip 5, the settings word, then the menu.
        assert!(steps.contains(&Step::Tip(5)));
        let write = steps
            .iter()
            .find(|step| matches!(step, Step::SettingsWrite { .. }))
            .copied()
            .expect("the novice win writes the unlock");
        assert_eq!(
            write,
            Step::SettingsWrite {
                key: EXPERT_UNLOCK_KEY,
                bytes: [1, 0, 0, 0],
            }
        );
        assert!(flow.unlocks.expert);
        assert!(flow.expert_written);
        // A second novice win writes nothing more.
        flow.phase = Phase::Playing;
        let again = flow.practice_tick(7000, None, &[DEAD]);
        assert!(again.contains(&Step::Tip(5)));
        assert!(
            !again
                .iter()
                .any(|step| matches!(step, Step::SettingsWrite { .. }))
        );
        // The expert ladder's end shows tip 6 and writes nothing.
        let mut expert = Flow {
            mode: Mode::Practice,
            phase: Phase::Playing,
            practice: PracticeState {
                ladder: Ladder::Expert,
                fight: 8,
                start_ms: 0,
                next_tip: false,
            },
            ..Default::default()
        };
        let steps = expert.practice_tick(6000, None, &[DEAD]);
        assert!(steps.contains(&Step::Tip(6)));
        assert!(
            !steps
                .iter()
                .any(|step| matches!(step, Step::SettingsWrite { .. }))
        );
        // A boot whose store already holds the unlock does not write it.
        let mut stored = Flow::default();
        stored.unlocks.expert = true;
        stored.boot();
        assert!(stored.expert_written);
    }

    #[test]
    fn a_practice_start_uses_the_ring_and_its_music() {
        let mut flow = Flow::default();
        let view = Viewport {
            width: 1600,
            height: 1140,
        };
        let steps = flow.start_practice(Ladder::Expert, 92.0, view, 500);
        assert_eq!(flow.mode, Mode::Practice);
        // The standing height plus 5 cm, over the ring's middle, half turned;
        // the shot looks 5 m ahead at the partner's place.
        assert!(steps.contains(&Step::PlacePlayer(Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(0.0, 97.0, 0.0),
        })));
        assert_eq!(flow.practice.fight, 1);
        assert!(steps.contains(&Step::PracticeFilter));
        assert!(steps.contains(&Step::PracticePlayer {
            role: PRACTICE_ROLE
        }));
        assert!(steps.contains(&Step::RequestAction(PRACTICE_MOTION)));
        assert!(steps.contains(&Step::Controls(true)));
        // The controls are attached, then combat mode goes on.
        let at = |wanted: Step| steps.iter().position(|step| *step == wanted).unwrap();
        assert_eq!(at(Step::CombatMode(true)), at(Step::Controls(true)) + 1);
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
        // Both drum-and-bass ladders play at 0.15.
        assert_eq!(
            practice_music(Ladder::Novice, true).volume.to_bits(),
            0x3e19_999a
        );
        assert_eq!(
            practice_music(Ladder::Expert, true).volume.to_bits(),
            0x3e19_999a
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
        // The unknown background lies down to wake 3000 ms on, right after
        // its motion; the controls are attached after the load.
        let at = |wanted: Step| steps.iter().position(|step| *step == wanted).unwrap();
        assert_eq!(
            at(Step::KnockOut { delay_ms: 3000 }),
            at(Step::RequestAction(0x1d)) + 1
        );
        assert!(at(Step::Controls(true)) < at(Step::RequestAction(0x1d)));
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
            roles: &[7, 1, 0x0e],
            flags: 0x200,
            off_hand: None,
            carried: Some(Carried {
                item: 0x10c,
                flags: 0,
                extra: 0x0080_0020,
            }),
            powers: Some((0x0051_0000, 4.0)),
        }));
        assert!(
            !steps
                .iter()
                .any(|step| matches!(step, Step::StoryPose { .. }))
        );
        assert!(steps.iter().any(|step| matches!(step, Step::Companion(_))));
        assert!(steps.iter().any(|step| matches!(step, Step::StoryLight(_))));
        // The villager gets up: the pose writes its condition, flags and
        // turn factor and asks for motion 0x1b.
        let mut flow = Flow::default();
        let steps = flow.start_story(Background::Villager, 1.8, view, 500);
        assert!(flow.story.wake.getting_up);
        assert!(steps.contains(&Step::RequestAction(START_MOTION_VILLAGER)));
        assert!(
            !steps
                .iter()
                .any(|step| matches!(step, Step::KnockOut { .. }))
        );
        assert!(steps.contains(&Step::StoryPose {
            condition: Some(0x600),
            flags: Some(0x600),
            turn_factor: Some(0.45),
        }));
        // The knight waits for a scripted motion and starts at turn 1.0.
        let mut flow = Flow::default();
        let steps = flow.start_story(Background::Knight, 1.8, view, 500);
        assert!(flow.story.wake.delayed);
        assert!(steps.contains(&Step::RequestAction(START_MOTION_WAKING)));
        assert!(steps.contains(&Step::StoryPose {
            condition: None,
            flags: None,
            turn_factor: Some(1.0),
        }));
        let mut flow = Flow::default();
        let placement = Placement {
            rotation: NO_ROTATION,
            position: Vec3::new(1.0, 2.0, 3.0),
        };
        let steps = flow.continue_story(Some("exanima02"), placement, view);
        assert_eq!(steps.first(), Some(&Step::HideMenu));
        assert!(!flow.story.wake.animating);
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
            animator: 0,
            health: 0.2,
        });
        let steps = flow.frame(1000, alive, &[]);
        assert_eq!(steps.first(), Some(&Step::Dialogue));
        assert!(steps.contains(&Step::Discovery));
        flow.menu_open = true;
        assert!(flow.frame(2000, alive, &[]).is_empty());
        flow.menu_open = false;
        // Any phase but the transition's runs the frame: an ended session
        // has no hook, so only the dialogue ticks.
        flow.phase = Phase::Ended;
        flow.hook = Hook::None;
        assert_eq!(flow.frame(2000, alive, &[]), vec![Step::Dialogue]);
        assert_eq!(flow.frames(), 3);
        // The transition's phase word stops it; the screen's action runs on
        // its own frame, at black.
        flow.phase = Phase::Playing;
        flow.hook = Hook::Story;
        flow.transition(Action::Revive, false);
        assert!(flow.frame(3050, alive, &[]).is_empty());
        assert!(
            run_to_black_at(&mut flow, 3100)
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
            flow.transition(Action::QuitToMenu, true),
            vec![Step::Transition]
        );
        let steps = run_to_black(&mut flow);
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
        // Only bit 0 unlocks: the page's disabled-row test reads that bit.
        unlocks.read_expert(&[2, 0, 0, 0]);
        assert!(!unlocks.expert);
        unlocks.read_expert(&[3, 0, 0, 0]);
        assert!(unlocks.expert);
        assert_eq!(Unlocks::read_word(&[1, 2, 3, 4]), Some(0x0403_0201));
        assert_eq!(Unlocks::read_word(&[]), None);
        assert_eq!(SCRIPT_HOOK, "GameEvent");
        let names: Vec<&str> = SETTINGS_KEYS.iter().map(|key| key.name).collect();
        assert_eq!(
            names,
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
        assert!(flow.leave_to_locale("exanima04", false).is_empty());
        assert_eq!(flow.phase, Phase::Playing);
        let steps = flow.leave_to_locale("exanima04", true);
        assert!(steps.contains(&Step::Transition));
        assert_eq!(flow.phase, Phase::Transition);
        let steps = run_to_black(&mut flow);
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
        // The end hook and the return to the title are the transition's
        // action, at black.
        let steps = flow.cancel_class_select(false);
        assert_eq!(steps, vec![Step::Transition]);
        let steps = run_to_black(&mut flow);
        assert_eq!(
            steps.first(),
            Some(&Step::ScriptMode {
                mode: VERSUS_MODE,
                hook: ScriptHook::End
            })
        );
        assert_eq!(flow.mode, Mode::Menu);
        // The class select's direct cancel goes back to the title at once,
        // after the scripted mode's end hook.
        let mut flow = Flow {
            mode: Mode::Other(VERSUS_MODE),
            phase: Phase::ClassSelect,
            menu_open: true,
            ..Default::default()
        };
        let steps = flow.class_cancel_direct();
        assert!(steps.contains(&Step::ScriptMode {
            mode: VERSUS_MODE,
            hook: ScriptHook::End
        }));
        assert!(!steps.contains(&Step::Transition));
        assert_eq!(flow.mode, Mode::Menu);
        assert_eq!(flow.phase, Phase::Menu);
        // Closing the end menu closes the prelude too and forgets the menu.
        let mut flow = Flow {
            end_menu: Some(end_menu(MENU_WON, Mode::Story)),
            ..Default::default()
        };
        assert_eq!(
            flow.close_end_menu(),
            vec![Step::HideEndMenu, Step::ClosePrelude]
        );
        assert!(flow.end_menu.is_none());
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
        assert!(steps.contains(&Step::ClearStoryFlag));
        assert!(steps.contains(&Step::HideEndMenu));
        assert!(steps.contains(&Step::ClosePrelude));
        // The respawn re-attaches the player controls to the new body; it
        // does not take them away.
        assert!(!steps.contains(&Step::Controls(false)));
        assert!(!steps.iter().any(|step| matches!(step, Step::Camera(_))));
        // After the load: the controls back, the lie-down with no delay,
        // the get-up motion, then the placement.
        let load = steps
            .iter()
            .position(|step| matches!(step, Step::LoadLevel { .. }))
            .unwrap();
        assert_eq!(
            steps[load + 1..load + 4],
            [
                Step::Controls(true),
                Step::KnockOut { delay_ms: 0 },
                Step::RequestAction(RESPAWN_MOTION),
            ]
        );
        assert!(matches!(steps[load + 4], Step::PlacePlayer(_)));
    }

    #[test]
    fn the_story_end_menu_exit_saves_by_the_reconstruction_state() {
        // A stripped player clears the story mark and is saved without the
        // death rule.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Ended,
            story: StoryState {
                reconstruction: STRIPPED,
                ..Default::default()
            },
            ..Default::default()
        };
        let steps = flow.story_exit();
        assert_eq!(
            steps,
            vec![
                Step::GameGui(false),
                Step::Controls(false),
                Step::Dialogue,
                Step::ClearStoryFlag,
                Step::Save {
                    drop_on_low_health: false
                },
                Step::Transition,
            ]
        );
        assert_eq!(flow.phase, Phase::Transition);
        let _ = run_to_black(&mut flow);
        assert_eq!(flow.mode, Mode::Menu);
        // Any other story writes its save with the death rule and does not
        // clear the mark.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Ended,
            ..Default::default()
        };
        let steps = flow.story_exit();
        assert!(steps.contains(&Step::Save {
            drop_on_low_health: true
        }));
        assert!(!steps.contains(&Step::ClearStoryFlag));
        assert!(steps.contains(&Step::Transition));
    }

    #[test]
    fn the_arena_pauses_forfeit_row_runs_the_forfeit_step() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<MenuChoice>()
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .add_systems(Update, apply_menu_choices);
        app.insert_resource(crate::arena_match::ArenaMatch::begin(1));
        app.world_mut().resource_mut::<Flow>().mode = Mode::Arena;
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::FinishMatch { forfeit: true }));
        app.update();
        let steps = core::mem::take(&mut app.world_mut().resource_mut::<FlowSteps>().steps);
        assert_eq!(
            steps,
            vec![Step::HideMenu, Step::Controls(true), Step::Forfeit]
        );
        // Without a live match the same row leaves to the title.
        app.world_mut()
            .remove_resource::<crate::arena_match::ArenaMatch>();
        app.world_mut()
            .write_message(MenuChoice(MenuCommand::FinishMatch { forfeit: true }));
        app.update();
        let steps = core::mem::take(&mut app.world_mut().resource_mut::<FlowSteps>().steps);
        assert!(!steps.contains(&Step::Forfeit));
        assert!(
            steps
                .iter()
                .any(|step| matches!(step, Step::ShowMenu(Page::Title)))
        );
    }

    #[test]
    fn the_arenas_pause_page_and_resume() {
        let mut flow = Flow {
            mode: Mode::Arena,
            phase: Phase::Playing,
            ..Default::default()
        };
        assert_eq!(
            flow.pause(),
            vec![Step::ShowMenu(Page::ArenaPause), Step::Controls(false)]
        );
        assert_eq!(flow.phase, Phase::Paused);
        assert!(flow.menu_open);
        // A pause from any other phase does nothing.
        assert!(flow.pause().is_empty());
        assert_eq!(flow.resume(), vec![Step::HideMenu, Step::Controls(true)]);
        assert_eq!(flow.phase, Phase::Playing);
        assert!(!flow.menu_open);
        assert!(flow.resume().is_empty());
    }

    #[test]
    fn a_session_begins_and_continues_around_its_starter() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Ended,
            end_menu: Some(end_menu(MENU_WON, Mode::Story)),
            menu_open: true,
            ..Default::default()
        };
        let steps = flow.begin_session(vec![Step::LoadLevel {
            locale: "exanima01",
            spawn: 0,
            teardown: true,
        }]);
        assert_eq!(
            steps,
            vec![
                Step::LoadLevel {
                    locale: "exanima01",
                    spawn: 0,
                    teardown: true
                },
                Step::SessionTask(true),
                Step::EndMenuUnselected(false),
            ]
        );
        assert_eq!(flow.phase, Phase::Playing);
        assert!(!flow.menu_open);
        assert!(flow.end_menu.is_none());
        // A continue closes the menu form first, then starts the task.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Menu,
            menu_open: true,
            ..Default::default()
        };
        let steps = flow.continue_session(vec![Step::LoadLevel {
            locale: "exanima02",
            spawn: 0,
            teardown: true,
        }]);
        assert_eq!(
            steps,
            vec![
                Step::HideMenu,
                Step::LoadLevel {
                    locale: "exanima02",
                    spawn: 0,
                    teardown: true
                },
                Step::SessionTask(true),
                Step::EndMenuUnselected(false),
            ]
        );
        assert_eq!(flow.phase, Phase::Playing);
        assert!(!flow.menu_open);
    }

    #[test]
    fn back_to_the_title_clears_the_reconstruction_state() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Ended,
            story: StoryState {
                reconstruction: REBUILDABLE,
                checkpoint: 9,
                ..Default::default()
            },
            ..Default::default()
        };
        let steps = flow.quit_to_menu();
        assert_eq!(flow.mode, Mode::Menu);
        assert_eq!(flow.phase, Phase::Menu);
        assert_eq!(flow.story.reconstruction, 0);
        assert_eq!(flow.story.checkpoint, 0);
        assert!(flow.menu_open);
        // The title logo shows again; the help flag and the GUI scale are
        // not touched.
        assert!(steps.contains(&Step::TitleLogo(true)));
        assert!(
            !steps
                .iter()
                .any(|step| matches!(step, Step::MenuHelp(_) | Step::GuiScale(_)))
        );
        assert_eq!(
            steps
                .iter()
                .filter(|step| matches!(step, Step::Music { .. }))
                .count(),
            1
        );
        assert_eq!(steps.last(), Some(&Step::SessionTask(false)));
    }

    #[test]
    fn the_units_settings_keys_and_finalisation() {
        let names: Vec<&str> = SETTINGS_KEYS.iter().map(|key| key.name).collect();
        assert_eq!(
            names,
            [
                "CursorSize",
                "CursorInt",
                "CursorCom",
                "FirstRun",
                "ShowGameTips",
                "CheckPoints"
            ]
        );
        let kinds: Vec<u32> = SETTINGS_KEYS.iter().map(|key| key.kind).collect();
        assert_eq!(kinds, [4, 5, 5, 3, 3, 3]);
        assert_eq!(SCRIPT_HOOK, "GameEvent");
        assert_eq!(
            FINALISED_STRINGS,
            [
                "MenuTheme",
                "MenuTheme_DnB",
                crate::menu_text::MENU_TEXT_SHADER
            ]
        );
        let mut flow = Flow {
            custom_hook: true,
            ..Default::default()
        };
        assert_eq!(flow.finalise(), vec![Step::Finalise]);
        assert!(!flow.custom_hook);
    }

    #[test]
    fn the_level_load_finish_and_the_menu_routines() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Transition,
            end_menu: Some(end_menu(MENU_WON, Mode::Story)),
            menu_open: true,
            ..Default::default()
        };
        assert_eq!(
            flow.level_loaded(),
            vec![Step::HideEndMenu, Step::ClosePrelude]
        );
        assert_eq!(flow.phase, Phase::Playing);
        assert!(flow.end_menu.is_none());
        // F1 in a session stops the hook behind the page and starts it
        // again on the way out.
        flow.menu_opened();
        assert!(flow.menu_open);
        assert!(flow.frame(2000, None, &[]).is_empty());
        flow.menu_closed();
        assert!(!flow.menu_open);
        assert_eq!(flow.frame(2000, None, &[]), vec![Step::Dialogue]);
        flow.mode = Mode::Menu;
        assert_eq!(flow.show_menu_form(), vec![Step::ShowMenu(Page::Title)]);
        assert!(flow.menu_open);
        assert_eq!(flow.hide_menu_form(), vec![Step::HideMenu]);
        assert!(!flow.menu_open);
        assert_eq!(
            flow.leave_game_layer(),
            vec![Step::TeardownWorld, Step::GameGui(false)]
        );
    }

    #[test]
    fn leaving_a_session_to_the_menu_form_plays_the_theme() {
        let mut flow = Flow {
            mode: Mode::Practice,
            phase: Phase::Won,
            hook: Hook::None,
            ..Default::default()
        };
        let steps = flow.leave_to_menu_form();
        assert_eq!(flow.phase, Phase::Menu);
        assert!(flow.menu_open);
        assert!(steps.contains(&Step::TeardownWorld));
        assert!(steps.iter().any(|step| matches!(
            step,
            Step::Music {
                name: "MenuTheme",
                restart: true,
                ..
            }
        )));
        assert_eq!(
            steps.last(),
            Some(&Step::Music {
                name: "MenuTheme",
                volume: 0.1,
                restart: true,
            })
        );
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
        assert_eq!(steps, vec![Step::GameGui(false), Step::CloseSave]);
    }

    #[test]
    fn the_gui_scale_save_managers_and_hover() {
        assert_eq!(gui_scale(1080), 0.5);
        assert_eq!(gui_scale(453), 0.1875);
        assert_eq!(SAVE_MANAGERS[0].signature, save_package::STORY_SIGNATURE);
        assert_eq!(SAVE_MANAGERS[1].signature, save_package::ARENA_SIGNATURE);
        assert_eq!(SAVE_MANAGERS[1].data_version, Some(9));
        assert_eq!(SAVE_MANAGERS[0].package_magic, 0xDA7A_EA02);
        let story = story_family();
        assert_eq!((story.prefix.as_str(), story.prefix_len), ("Exanima", 7));
        assert_eq!((story.count, story.current), (-1, -1));
        assert_eq!(story.id_word, 0x6431_ac03);
        assert!(!story.extra_hook);
        assert_eq!(story.package_magic, 0xDA7A_EA02);
        let arena = arena_family();
        assert_eq!((arena.prefix.as_str(), arena.prefix_len), ("Arena", 5));
        assert_eq!(arena.id_word, 0x6431_ac13);
        assert!(arena.extra_hook);
        assert_eq!(arena.package_magic, 0xDA7A_AA00);
        assert_eq!(hover_sound(false, 0.5), None);
        // A half draw: 0.5·0.4 + 0.8 = 1.0; the ends are 0.8 and 1.2.
        assert_eq!(hover_sound(true, 0.5), Some(1.0));
        assert_eq!(hover_sound(true, 0.0), Some(0.8));
        assert_eq!(HOVER_VOLUME.to_bits(), 0x3d75_c28f);
        assert_eq!(Flow::default().practice_health(), 0.0);
        let mut flow = Flow::default();
        assert_eq!(flow.help_closed(), vec![Step::MenuHelp(false)]);
        assert!(flow.menu_confirm(false, true, 0.5).is_empty());
        assert_eq!(
            flow.menu_confirm(true, true, 0.5),
            vec![
                Step::Hover {
                    volume: 0.06,
                    pitch: 1.0
                },
                Step::MenuConfirm
            ]
        );
        assert_eq!(flow.menu_confirm(true, false, 0.5), vec![Step::MenuConfirm]);
    }

    #[test]
    fn a_direct_launch_can_skip_the_boot_title_page() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .insert_resource(BootToMenu(false))
            .insert_resource(UnlockSettings {
                expert: Some(1u32.to_le_bytes()),
                backgrounds: Some(5u32.to_le_bytes()),
            })
            .add_plugins((crate::menu_screen::MenuScreenPlugin, FlowPlugin));
        app.update();
        assert!(
            !app.world()
                .resource::<crate::menu_screen::MenuScreen>()
                .open
        );
        let flow = app.world().resource::<Flow>();
        assert_eq!(flow.mode, Mode::Menu);
        assert!(flow.booted());
        // The launch's two settings words reach the boot.
        assert!(flow.unlocks.expert);
        assert_eq!(flow.unlocks.backgrounds, 5);
    }

    #[test]
    fn the_unlock_entries_are_read_from_the_settings_store() {
        use exa_formats::settings_store::SettingsStore;

        let mut store = SettingsStore::new();
        store
            .set(&[SETTINGS_SECTION, EXPERT_UNLOCK_KEY], &[1, 0, 0, 0])
            .unwrap();
        store
            .set(&[SETTINGS_SECTION, BACKGROUND_UNLOCK_KEY], &[5, 0, 0, 0])
            .unwrap();
        let settings = UnlockSettings::from_store(&store);
        assert_eq!(settings.expert, Some([1, 0, 0, 0]));
        assert_eq!(settings.backgrounds, Some([5, 0, 0, 0]));
        let mut unlocks = Unlocks::default();
        unlocks.apply_settings(settings);
        assert!(unlocks.expert);
        assert_eq!(unlocks.backgrounds, 5);
        // A missing entry leaves the current word alone.
        let mut unlocks = Unlocks {
            expert: true,
            backgrounds: 7,
        };
        unlocks.apply_settings(UnlockSettings::default());
        assert!(unlocks.expert);
        assert_eq!(unlocks.backgrounds, 7);
    }

    #[test]
    fn a_player_death_opens_the_end_menu_and_its_exit_returns_to_the_title() {
        use exa_physics::body::BodyTicks;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                FlowPlugin,
                crate::end_menu_screen::EndMenuScreenPlugin,
            ));
        app.insert_resource(BodyTicks(100));
        // The boot opens the title; then the session takes over.
        app.update();
        app.world_mut()
            .resource_mut::<crate::menu_screen::MenuScreen>()
            .close();
        {
            let mut flow = app.world_mut().resource_mut::<Flow>();
            flow.mode = Mode::Story;
            flow.phase = Phase::Playing;
            flow.hook = Hook::Story;
            flow.menu_open = false;
            flow.story = StoryState {
                reconstruction: REBUILDABLE,
                record: 3,
                checkpoint: 3,
                knockout_ms: u32::MAX - KNOCKOUT_MS,
                ..Default::default()
            };
        }
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                exa_combat::Condition(DEAD),
                exa_combat::Fighter {
                    health: exa_combat::Health {
                        yellow: 0.0,
                        red: 0.0,
                        max: 0.25,
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .id();
        assert!(
            app.world()
                .get::<crate::player::PlayerBody>(player)
                .is_some()
        );
        app.update();
        // The reconstructed story player's end menu: CONTINUE and EXIT.
        let end = app
            .world()
            .resource::<crate::end_menu_screen::EndMenuScreen>();
        assert!(end.open);
        assert_eq!(end.menu.unwrap().rows, &["CONTINUE", "EXIT"]);
        assert_eq!(end.selected, Some(0));
        // EXIT runs the story's end-menu exit behind the transition.
        app.world_mut()
            .resource_mut::<crate::end_menu_screen::EndMenuScreen>()
            .selected = Some(1);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Enter);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        assert_eq!(app.world().resource::<Flow>().phase, Phase::Transition);
        assert!(
            !app.world()
                .resource::<crate::end_menu_screen::EndMenuScreen>()
                .open
        );
        // The screen fades to black on the real clock and the quit to the
        // title shows the title page again.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(400),
        ));
        for _ in 0..4 {
            app.update();
        }
        assert_ne!(
            app.world()
                .resource::<crate::transition_screen::TransitionOverlay>()
                .0,
            0
        );
        let flow = app.world().resource::<Flow>();
        assert_eq!(flow.mode, Mode::Menu);
        assert_eq!(flow.phase, Phase::Menu);
        let menu = app.world().resource::<crate::menu_screen::MenuScreen>();
        assert!(menu.open);
        assert_eq!(menu.page, Page::Title);
    }

    fn screen_input(clock_ms: u32) -> TransitionInput {
        TransitionInput {
            clock_ms,
            music: None,
            audio_fade: 1.0,
            loading: false,
        }
    }

    /// The screen fades to black over 600 ms and runs its action only past
    /// it, holds two frames and 500 ms, fades back in over 600 ms and closes.
    #[test]
    fn the_transition_runs_its_action_at_black() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            hook: Hook::Knockout,
            ..Default::default()
        };
        assert_eq!(
            flow.transition(Action::Revive, false),
            vec![Step::Transition]
        );
        assert!(flow.transition_running());
        let first = flow.transition_tick(screen_input(1000), 0);
        assert_eq!((first.alpha, first.steps.len()), (0, 0));
        let half = flow.transition_tick(screen_input(1300), 0);
        assert_eq!(half.alpha, 128);
        // 600 ms make exactly one, which is not past black.
        let full = flow.transition_tick(screen_input(1600), 0);
        assert_eq!(full.alpha, 255);
        assert!(full.steps.is_empty());
        assert_eq!(flow.phase, Phase::Transition);
        let black = flow.transition_tick(screen_input(1601), 0);
        assert!(black.world_fade_reset && !black.stop_music);
        assert!(black.steps.contains(&Step::Hook(Hook::Story)));
        // Two frames, then the 500 ms hold, still black.
        for clock in [1610, 1620, 1630] {
            assert_eq!(flow.transition_tick(screen_input(clock), 0).alpha, 255);
        }
        assert_eq!(flow.transition_tick(screen_input(2130), 0).alpha, 255);
        // Past the hold the fade back in starts and takes 600 ms.
        flow.transition_tick(screen_input(2131), 0);
        assert_eq!(flow.transition_tick(screen_input(2431), 0).alpha, 128);
        let closed = flow.transition_tick(screen_input(2731), 0);
        assert!(closed.closed);
        assert_eq!(closed.alpha, 0);
        assert!(!flow.transition_running());
        assert_eq!(
            flow.transition_tick(screen_input(2800), 0),
            TransitionFrame::default()
        );
    }

    /// The level exit's fade bit gives the screen its long hold: a 2000 ms
    /// fade and a 1000 ms hold; an exit without it fades over 600 ms.
    #[test]
    fn the_exit_fade_bit_lengthens_the_transition() {
        let rule = CheckpointRule {
            level: 3,
            setting: false,
            already_saved: false,
        };
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        let exit = LevelExit {
            level: 3,
            spawn: 0,
            flags: 1,
        };
        flow.level_exit(exit, rule, 0, false);
        flow.transition_tick(screen_input(0), 0);
        assert_eq!(flow.transition_tick(screen_input(1000), 0).alpha, 128);
        assert!(flow.transition_tick(screen_input(2000), 0).steps.is_empty());
        let black = flow.transition_tick(screen_input(2001), 0);
        assert!(
            black
                .steps
                .iter()
                .any(|step| matches!(step, Step::LoadLevel { .. }))
        );
        // A plain exit: the 600 ms fade.
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        flow.level_exit(LevelExit { flags: 0, ..exit }, rule, 0, false);
        flow.transition_tick(screen_input(0), 0);
        assert!(!flow.transition_tick(screen_input(601), 0).steps.is_empty());
    }

    /// With the music fade on the music falls with the screen, stops at
    /// black with the audio fade at 0, and the audio fade rises back with
    /// the fade in. Without it the audio fade at 1 is never written.
    #[test]
    fn the_transition_fades_the_music_with_the_screen() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        flow.continue_after_death();
        let input = |clock_ms, audio_fade, loading| TransitionInput {
            clock_ms,
            music: Some(0.5),
            audio_fade,
            loading,
        };
        let first = flow.transition_tick(input(0, 1.0, false), 0);
        assert_eq!(first.music, Some(0.5));
        let half = flow.transition_tick(input(300, 1.0, false), 0);
        assert_eq!(half.music, Some(0.25));
        let black = flow.transition_tick(input(700, 1.0, false), 0);
        assert!(black.stop_music && black.world_fade_reset);
        assert_eq!(black.audio_fade, Some(0.0));
        assert!(!black.steps.is_empty());
        // The respawn loads a level: held while it loads.
        let held = flow.transition_tick(input(710, 0.0, true), 0);
        assert_eq!((held.music, held.alpha), (None, 255));
        flow.transition_tick(input(720, 0.0, false), 0);
        flow.transition_tick(input(730, 0.0, false), 0);
        flow.transition_tick(input(740, 0.0, false), 0);
        flow.transition_tick(input(1241, 0.0, false), 0);
        let rising = flow.transition_tick(input(1541, 0.0, false), 0);
        assert_eq!(rising.audio_fade, Some(0.5));
        let closed = flow.transition_tick(input(1841, 0.5, false), 0);
        assert!(closed.closed);
        assert_eq!(closed.audio_fade, Some(1.0));

        let mut flow = Flow::default();
        flow.transition(Action::Revive, false);
        for clock in [0, 300, 700, 710, 720, 730, 1240, 1540, 1840] {
            let frame = flow.transition_tick(input(clock, 1.0, false), 0);
            assert_eq!((frame.music, frame.audio_fade), (None, None));
            assert!(!frame.stop_music);
        }
        assert!(!flow.transition_running());
    }

    /// An action that changes the level keeps the screen black while the
    /// change runs; the hold starts when it is done. A change that never
    /// shows up is waited for a bounded number of frames.
    #[test]
    fn the_transition_waits_for_the_level_change() {
        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        flow.leave_to_locale("exanima04", true);
        let input = |clock_ms, loading| TransitionInput {
            clock_ms,
            music: None,
            audio_fade: 1.0,
            loading,
        };
        flow.transition_tick(input(0, false), 0);
        assert!(!flow.transition_tick(input(601, false), 0).steps.is_empty());
        for clock in (700..5000).step_by(100) {
            assert_eq!(flow.transition_tick(input(clock, true), 0).alpha, 255);
        }
        // Done at 5000: two frames, the 500 ms hold from then, still black.
        for clock in [5000, 5010, 5020, 5520] {
            assert_eq!(flow.transition_tick(input(clock, false), 0).alpha, 255);
        }
        flow.transition_tick(input(5521, false), 0);
        assert_eq!(flow.transition_tick(input(5821, false), 0).alpha, 128);

        let mut flow = Flow::default();
        flow.leave_to_locale("exanima04", true);
        flow.transition_tick(input(0, false), 0);
        flow.transition_tick(input(601, false), 0);
        for _ in 0..LOAD_WAIT_FRAMES {
            assert_eq!(flow.transition_tick(input(700, false), 0).alpha, 255);
        }
        // The wait is over: the hold starts.
        flow.transition_tick(input(700, false), 0);
        flow.transition_tick(input(700, false), 0);
        flow.transition_tick(input(700, false), 0);
        flow.transition_tick(input(1201, false), 0);
        assert!(flow.transition_tick(input(1801, false), 0).closed);
    }

    /// The end menu's next fight fades the music only into the last fight.
    #[test]
    fn the_next_fight_fades_the_music_into_the_boss() {
        use crate::end_menu_screen::EndMenuScreen;
        for (fight, fades) in [(6, false), (7, true)] {
            let mut flow = Flow {
                mode: Mode::Practice,
                phase: Phase::RoundWon,
                ..Default::default()
            };
            flow.practice.fight = fight;
            let mut screen = EndMenuScreen::default();
            screen.show(end_menu(MENU_NEXT, Mode::Practice));
            screen.selected = Some(0);
            assert_eq!(screen.confirm(&mut flow), vec![Step::Transition]);
            let input = TransitionInput {
                clock_ms: 0,
                music: Some(1.0),
                audio_fade: 1.0,
                loading: false,
            };
            assert_eq!(flow.transition_tick(input, 0).music.is_some(), fades);
        }
    }

    /// A load step becomes a level change request, a teardown step is
    /// folded into a load in the same batch, and a placement is carried to
    /// the host in metres.
    #[test]
    fn the_app_turns_a_load_and_a_placement_into_a_level_change() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_systems(Update, apply_steps);
        let placement = Placement {
            rotation: yaw(90.0),
            position: Vec3::new(100.0, 200.0, 300.0),
        };
        app.world_mut().resource_mut::<FlowSteps>().steps.extend([
            Step::TeardownWorld,
            Step::LoadLevel {
                locale: "arenahub",
                spawn: 2,
                teardown: true,
            },
            Step::PlacePlayer(placement),
        ]);
        app.update();
        let requests: Vec<exa_world::level_lifecycle::LevelChangeRequest> = app
            .world_mut()
            .resource_mut::<Messages<exa_world::level_lifecycle::LevelChangeRequest>>()
            .drain()
            .collect();
        // One request: the teardown is the load's own.
        assert_eq!(
            requests,
            vec![exa_world::level_lifecycle::LevelChangeRequest {
                level: Some("arenahub".into()),
                spawn: 2,
                teardown: true,
            }]
        );
        let at = app
            .world()
            .resource::<crate::level_change::PlayerPlacement>();
        assert!((at.position - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-6);
        assert!((at.yaw - std::f32::consts::FRAC_PI_2).abs() < 1e-6);

        // The wake steps write the player's animator flags word: the
        // get-up replaces it, the revival sets bits on it.
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                exa_combat::Condition(DEAD),
            ))
            .id();
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::Wake(WakePose::hold()));
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::StateFlags>(player),
            Some(&exa_combat::StateFlags(0x600))
        );
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::Wake(WakePose::revived()));
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::StateFlags>(player),
            Some(&exa_combat::StateFlags(0x640))
        );
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::StoryPose {
                condition: None,
                flags: Some(0x600),
                turn_factor: None,
            });
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::StateFlags>(player),
            Some(&exa_combat::StateFlags(0x600))
        );
        // The practice round's condition write lands on the body.
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::Condition(0));
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::Condition>(player),
            Some(&exa_combat::Condition(0))
        );

        // Combat mode follows its step.
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::CombatMode(true));
        app.update();
        assert!(
            app.world()
                .get::<crate::player::PlayerBody>(player)
                .unwrap()
                .combat
        );

        // The lie-down at game clock 1200 ms: down, the wake nibble 0x30
        // (the other bits kept), the wake clock 3000 ms on.
        app.insert_resource(exa_physics::body::BodyTicks(100));
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::KnockOut { delay_ms: 3000 });
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::Condition>(player),
            Some(&exa_combat::Condition::DOWN)
        );
        assert_eq!(
            app.world().get::<exa_combat::StateFlags>(player),
            Some(&exa_combat::StateFlags(0x630))
        );
        assert_eq!(
            app.world()
                .get::<exa_combat::condition::KnockOut>(player)
                .map(|k| k.wake_at_ms),
            Some(4200)
        );
    }

    /// A story death of a scanned player: the flow asks for the strip and the
    /// app applies it, clearing the hands and worn data, keeping the
    /// underwear rule and leaving yellow and red health just under half.
    #[test]
    fn the_app_strips_a_reconstructed_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_systems(Update, apply_steps);
        let mut fighter = exa_combat::Fighter::default();
        fighter.health.max = 100.0;
        fighter.health.red = 0.0;
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                fighter,
                exa_character::Outfit::default(),
                exa_character::Wielded::model("ub05ssworda"),
            ))
            .id();
        let mut flow = Flow {
            mode: Mode::Story,
            ..Default::default()
        };
        flow.story.reconstruction = REBUILDABLE;
        flow.story.record = 7;
        flow.story.checkpoint = 7;
        let steps = flow.die(0.0);
        assert!(steps.contains(&Step::StripPlayer));
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .extend(steps);
        app.update();
        let world = app.world();
        assert!(world.get::<exa_character::Wielded>(player).is_none());
        assert!(world.get::<exa_character::WearsUnderwear>(player).is_some());
        let health = world.get::<exa_combat::Fighter>(player).unwrap().health;
        assert!((health.red - 49.999_237).abs() < 1e-3, "{health:?}");
        assert!((health.yellow - 49.999_237).abs() < 1e-3, "{health:?}");
    }

    /// The story revive on a dead player body at game clock 1200 ms: combat
    /// mode is left, the yellow health becomes half the red, the body lies
    /// down with no delay, the held bit is set and its wake clock is pushed
    /// 2000 ms on (to 3200), and the controls come back. A living body is
    /// not raised.
    #[test]
    fn the_app_raises_the_revived_player_and_holds_its_wake() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::player_controls::PlaySession>()
            .insert_resource(exa_physics::body::BodyTicks(100))
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_systems(Update, apply_steps);
        let mut fighter = exa_combat::Fighter::default();
        fighter.health.red = 0.5;
        fighter.health.yellow = 0.0;
        let mut body = crate::player::PlayerBody::new(0.0);
        body.set_combat(true);
        let player = app
            .world_mut()
            .spawn((body, fighter, exa_combat::Condition(DEAD)))
            .id();
        let mut flow = Flow {
            mode: Mode::Story,
            ..Default::default()
        };
        let steps = flow.revive(1200);
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .extend(steps);
        app.update();
        let world = app.world();
        assert!(
            !world
                .get::<crate::player::PlayerBody>(player)
                .unwrap()
                .combat
        );
        assert_eq!(
            world
                .get::<exa_combat::Fighter>(player)
                .unwrap()
                .health
                .yellow,
            0.25
        );
        assert_eq!(
            world.get::<exa_combat::Condition>(player),
            Some(&exa_combat::Condition::DOWN)
        );
        assert_eq!(
            world.get::<exa_combat::StateFlags>(player),
            Some(&exa_combat::StateFlags(0x70))
        );
        assert_eq!(
            world
                .get::<exa_combat::condition::KnockOut>(player)
                .map(|k| k.wake_at_ms),
            Some(3200)
        );
        assert!(
            world
                .resource::<crate::player_controls::PlaySession>()
                .playing
        );

        // A living body is left standing.
        *app.world_mut()
            .get_mut::<exa_combat::Condition>(player)
            .unwrap() = exa_combat::Condition::NORMAL;
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::ReviveBody);
        app.update();
        assert_eq!(
            app.world().get::<exa_combat::Condition>(player),
            Some(&exa_combat::Condition::NORMAL)
        );
    }

    /// The wake clock writes: the get-up's start puts the wake time 1490 ms
    /// before the clock, the revive pushes it 2000 ms on, both as 32-bit
    /// sums.
    #[test]
    fn the_wake_clock_writes_wrap_as_32_bit_sums() {
        assert_eq!(ClipClock::MotionMinus(1490).applied(7, 5000), 3510);
        assert_eq!(ClipClock::MotionMinus(1490).applied(7, 1000), 0xffff_fe16);
        assert_eq!(ClipClock::Push(2000).applied(1000, 0), 3000);
        assert_eq!(ClipClock::Push(2000).applied(u32::MAX, 0), 1999);
    }
}

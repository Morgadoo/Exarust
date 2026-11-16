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
/// The bit the session leave tests before it looks a mode up in the
/// scripted-mode table ([`ScriptModes`]). The versus lobby is `0x1010`.
pub const SCRIPT_MODE_BIT: u32 = 0x1000;
/// The versus lobby's mode word. Nothing in Exanima 0.9.5 registers it in
/// the scripted-mode table, and the flag that shows its menu row is never
/// set there.
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
/// How far above the respawn portal's frame a respawned player is placed
/// (cm): the story respawn's and the save load's pending respawn's.
pub const PORTAL_LIFT_CM: f32 = 15.0;

/// The practice ladder's first fight number. The first partner is
/// `opponent01` (novice) or `opponent09` (expert).
pub const FIRST_FIGHT: i32 = 1;

/// The role a practice mode's player starts with.
pub const PRACTICE_ROLE: u32 = 0x10;

/// The motion a practice start and each next fight asks the player for.
pub const PRACTICE_MOTION: u32 = 3;

/// How far over the pose's pelvis target the practice start puts the
/// pelvis (cm), over the origin.
pub const PRACTICE_START_LIFT_CM: f32 = 5.0;

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
    /// The distance the camera's placement takes (`Some(1300.0)` after a
    /// session start: the zoom starts again from it; `None` keeps the view's
    /// own).
    pub distance: Option<f32>,
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
    /// The distance a fast shot places the camera at: the zoom's start.
    pub const START_DISTANCE: f32 = 1300.0;
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
    /// [`FAR_OFFSET`] (`far`) or [`NEAR_OFFSET`], the start distance when
    /// `fast`, and the shot's cursor warp. (The original sets the camera to
    /// the offset and adds the point to it in place.)
    pub fn frame_point(point: Vec3, fast: bool, far: bool, viewport: Viewport) -> CameraFrame {
        CameraFrame {
            camera: (if far { FAR_OFFSET } else { NEAR_OFFSET }) + point,
            target: point,
            distance: fast.then_some(START_DISTANCE),
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
            distance: fast.then_some(START_DISTANCE),
            warp: warp(viewport.width, viewport.height, WARP_X, WARP_Y),
        }
    }

    /// Frames the player at `point`: the camera at the point plus
    /// [`STORY_OFFSET`], the start distance, and the cursor moved left of
    /// centre.
    pub fn frame_player(point: Vec3, viewport: Viewport) -> CameraFrame {
        CameraFrame {
            camera: point + STORY_OFFSET,
            target: point,
            distance: Some(START_DISTANCE),
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

/// Where a respawn stands the player at `portal`: its frame lifted by
/// [`PORTAL_LIFT_CM`].
pub fn portal_lifted(portal: Placement) -> Placement {
    Placement {
        rotation: portal.rotation,
        position: portal.position + Vec3::Y * PORTAL_LIFT_CM,
    }
}

/// The identity rotation.
pub const NO_ROTATION: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// The practice spawn table's rotation: a half turn about the vertical axis.
pub const PRACTICE_ROTATION: [[f32; 3]; 3] = [[-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0]];

/// The practice spawn table's position (x, y, z = 0, 0, 500 cm). The start
/// and each next fight copy the table and overwrite only its vertical (y)
/// with the body's height, so the player stands at x 0, z 500; the placement
/// takes x and y as one 8-byte read and z on its own. The start shot frames
/// the table's own point.
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
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Background {
    #[default]
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
    /// The word the start puts in the spawn request beside the record
    /// (0x200, 0 or 0x600). [guess: the loaded body's condition word; the
    /// start's later writes give the body the same condition: the unknown
    /// background's lie-down 0x200, the villager's 0x600.]
    pub flags: u32,
    pub placement: Placement,
    /// The level-start point the save manager is given before the load.
    pub spawn_point: Vec3,
    /// The camera view written before the player is placed.
    pub view: StartView,
    /// The twelve-byte start record the spawn request points at for the
    /// duration of the start: the start motion, then two zero words. The
    /// request (the record, the word above, the level-start point) goes to
    /// the character spawner before the level loads.
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
    /// The starting light: the torch item laid in the world, the unknown
    /// background's beside the player and the villager's; the knight and
    /// proctor hold theirs in the off hand.
    pub light: Option<StoryLight>,
    /// Derrin, on the unknown background only.
    pub companion: Option<Companion>,
}

/// The story's starting light: the game's torch item (0x1f), made as a
/// world object from the game database with item flags 0, given its
/// position and its rotation from three angles ([`rotation`]), its scene
/// node raised to resource level 256, and added to the world, where it
/// falls and settles as any loose item does. (The unknown background's
/// torch lies at (8045, 4.7, 4075) cm; a retail capture finds it settled
/// at about (8045, 20, 4131).)
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoryLight {
    /// The game item made.
    pub item: u32,
    /// Where it is made, in centimetres.
    pub position: Vec3,
    /// Its three angles, in degrees.
    pub rotation_degrees: Vec3,
    /// The resource level its scene node is raised to.
    pub level: u32,
}

impl StoryLight {
    /// The item's frame as the engine builds it: [`rotation`] of the three
    /// angles.
    pub fn rows(&self) -> [[f32; 3]; 3] {
        let a = self.rotation_degrees;
        rotation(a.x, a.y, a.z)
    }
}

/// A story companion placed with the player.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Companion {
    pub template: &'static str,
    pub position: Vec3,
    /// The spawn record's facing word. The record takes radians (the
    /// hub's placements convert their degrees); the story start passes
    /// -90 unconverted, so the companion faces -90 rad. The level load
    /// places the companion (`exarust`'s viewer).
    pub facing: f32,
    /// The spawn record's flag word.
    pub flags: u32,
}

/// How long after a new story starts the unknown background's lie-down
/// lets the player wake.
pub const STORY_WAKE_DELAY_MS: u32 = 3000;

/// The starting torch: the game database's item 0x1f ("Torch A01").
pub const STORY_TORCH_ITEM: u32 = 0x1f;
/// The resource level the starting torch's scene node is raised to.
pub const STORY_TORCH_LEVEL: u32 = 256;
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
                item: STORY_TORCH_ITEM,
                position: Vec3::new(8045.0, 4.7, 4075.0),
                rotation_degrees: Vec3::new(-48.0, 110.0, 88.0),
                level: STORY_TORCH_LEVEL,
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
                item: STORY_TORCH_ITEM,
                position: Vec3::new(8000.0, 500.0, 4000.0),
                rotation_degrees: Vec3::new(8.0, -150.0, 50.0),
                level: STORY_TORCH_LEVEL,
            }),
            companion: None,
        },
    }
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
    /// The last whole second the story's once-a-second task list ran at
    /// (the pending level tasks; not modelled). The discovery system's
    /// pass is not gated by it: it runs every story frame.
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
    /// The background the story was started from (the original's session
    /// word: a new story writes the creator's pick, a load the save's
    /// start background; the unknown one otherwise). The story events act
    /// only on the unknown background.
    pub background: Background,
    /// The memory fragments seen, the original's session word: a fragment
    /// event or-s its low 16 bits in, an awakening makes it `0xffff`, a new
    /// story clears it, a save keeps it (its memory-flags chunk) and a load
    /// restores it.
    pub memory: u32,
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

/// A scripted mode's hook: one of the slots of its entry in the original's
/// game-mode table ([`ScriptModes`]). The end menu's slot is
/// [`crate::end_menu_screen::ScriptedEnd`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptHook {
    /// The mode switch entered the mode.
    Enter,
    /// The session start (after the mode's character is committed).
    Begin,
    /// The session leave. The arena's own leave is named with this hook
    /// too, under [`ARENA_MODE`].
    Leave,
    /// The character creator's cancel; the only slot the original tests for
    /// null before it calls it.
    Cancel,
}

/// One registered scripted mode: its word and whether its entry fills the
/// creator-cancel slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptModeEntry {
    pub word: u32,
    pub cancel: bool,
}

/// The original's table of scripted game modes: one entry per registered
/// mode word, each carrying its enter, begin, leave, creator-cancel and
/// end-menu hooks and the player record it kept when the mode was left.
/// Every lookup takes the first entry whose word matches.
///
/// Exanima 0.9.5 never fills it: the table's count is only ever read, and
/// nothing in the image holds the table's address. Its lookups are all in
/// place (the mode switch, the session start and leave, the creator's
/// cancel and the end menu), so a mode registered here runs where the
/// original would run it; with the table empty, every scripted-mode branch
/// does nothing, as in the retail game.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScriptModes {
    pub entries: Vec<ScriptModeEntry>,
}

impl ScriptModes {
    /// Registers a mode word (a word already present is appended again;
    /// the first entry keeps winning the lookups).
    pub fn register(&mut self, word: u32, cancel: bool) {
        self.entries.push(ScriptModeEntry { word, cancel });
    }

    /// The first entry for `word`.
    #[must_use]
    pub fn find(&self, word: u32) -> Option<(usize, ScriptModeEntry)> {
        self.entries
            .iter()
            .position(|entry| entry.word == word)
            .map(|index| (index, self.entries[index]))
    }

    /// The registered words, in order (the end menu's handler lookup).
    #[must_use]
    pub fn words(&self) -> Vec<u32> {
        self.entries.iter().map(|entry| entry.word).collect()
    }
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
    /// End every conversation: live ones end with their topic recorded,
    /// all are released, a pending narration is dropped and the reply
    /// panel closes (the session leave's and the story exit's,
    /// [`exa_script::EndAllConversations`]).
    EndDialogues,
    /// The first wake-up's animation starts being watched: the new story's
    /// unknown background sets it once its level is in, after the load's
    /// GUI reset cleared it ([`crate::load_reset`]).
    FirstWake,
    /// Run the discovery system's pass (every story frame; its conversions
    /// keep their own clocks).
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
    /// Where the next [`Step::PlacePlayer`] puts the pelvis: the frame
    /// height its caller writes before the living placement
    /// ([`crate::limp::FrameHeight`]). The practice start writes the pose's
    /// pelvis target plus 5 cm over the origin, each next fight its stand
    /// height over the origin; with no such step a placement lands on the
    /// floor the save landing probes. Taken with the placement
    /// ([`crate::level_change::PlacedFrame`]).
    PlacedFrame(crate::limp::FrameHeight),
    /// Place the player at the respawn portal the loaded level names: the
    /// load takes the session-start arrival word, so its reconstruction
    /// device (or a transition marker naming the same words) is the
    /// player's object, and the player stands at its frame 15 cm up
    /// ([`crate::level_change::PortalArrival`], applied once the level is
    /// in).
    PlaceAtPortal(crate::level_change::PortalArrival),
    /// Give the placed player's body the settling state for this many
    /// ticks once it stands where [`Step::PlacePlayer`] put it (gravity
    /// without inertia, `exa_physics::body::Ragdoll::settle_for`): the
    /// story respawn's, applied with the placement once the level is in
    /// ([`crate::level_change::PlacedSettle`]).
    SettlePlayer(i32),
    /// Move the placed player out of the level exit that holds it (the
    /// story continue's nudge, [`exit_nudge`]): applied with the placement
    /// once the level is in, against the level's exit zones
    /// ([`crate::level_change::NudgeOutOfExits`]).
    ExitNudge,
    /// Give the player its story roles (the list cleared first) and the
    /// gear and powers only the background's start writes, on the record
    /// the level load spawns the player from; `flags` is the spawn
    /// request's word ([`StoryStart::flags`]).
    StoryPlayer {
        roles: &'static [u32],
        flags: u32,
        off_hand: Option<ItemRef>,
        carried: Option<Carried>,
        powers: Option<(u32, f32)>,
    },
    /// The twelve-byte record a story start hands the spawner: the start
    /// motion and two zero words. The loaded player takes it with the
    /// record's writes ([`Step::StoryPlayer`]).
    StartRecord([u32; 3]),
    /// Lay the story's starting torch in the world ([`StoryLight`]).
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
    /// End the GUI's held left button: a held drag ends as a release
    /// would end it ([`crate::controls_live::EndGuiDrag`]).
    EndGuiDrag,
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
    /// The new game's empty slot in the mode's manager, which becomes the
    /// current slot the session's saves write.
    NewSlot,
    /// Close the save manager's open package.
    CloseSave,
    /// Keep the save just written as the checkpoint.
    Checkpoint,
    /// Stamp the level-6 checkpoint with the engine clock (the level's own
    /// time; the save unit reads it).
    LevelTime,
    /// The portal use on the player: the dissolve burst seeded over its
    /// body and `portaluse01` (the exit's portal bit; the zone exits of the
    /// level portals set it, a teleporter journey does not). Applied as
    /// [`crate::effects::portal::PortalUse`].
    PortalUse,
    /// The AI takes over the player's body for the level change (the exit's
    /// portal bit).
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
    /// The arena's EXIT from the in-game page, before its transition: every
    /// game screen and the power tree's bar close, the first unfit
    /// specialist leaves the company, the career is saved without the death
    /// rule and the arena's leave runs.
    ArenaExit,
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
    /// The player controls' block pair ([`crate::player_controls::PlayerControl::set_blocked`]):
    /// `true` blocks them (the knock-out, the death, the practice ladder's
    /// last win), `false` releases them (the revive).
    Block(bool),
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
    /// The story revive's alarm return, before its save: while an alarm is
    /// armed the awake golems around the player go home and sleep, and
    /// when one did the player is put outside the alarm box
    /// ([`crate::golem_alarm::return_from_alarm`] takes the step before the
    /// steps are applied).
    AlarmReturn,
    /// The story revive's world pass, after the alarm return: every AI
    /// character's reset through its behaviour (healed, contacts cleared, a
    /// roaming one moved along fresh wander plans and asked the space-free
    /// query; [`crate::revive_pass::reset_brains`] takes the step before
    /// the steps are applied).
    ReviveBrains,
    /// Raise the knocked-out player's dead body (the story revive): only a
    /// dead condition word (0xA00) is raised; combat mode is left, the
    /// yellow health becomes half the red and the body is laid down to wake
    /// at once.
    ReviveBody,
    /// Strip the player's gear (the reconstruction device).
    StripPlayer,
    /// Clear the player node's story bit (the reconstruction mark).
    ClearStoryFlag,
    /// The Mind sight's quit hook: the Sense Minds overlay goes off and
    /// drops its tracked minds ([`crate::mind::draw::reset_sense_overlay`]).
    ResetMindSight,
    /// The player's own record is handed back for a rebuild: its identity
    /// reset (the race byte cleared) and, while the content stream's hook is
    /// installed, its request posted with the word 0x1000 and the world
    /// scene; `flags` is the request's word before that (the death screen's
    /// CONTINUE passes 0x100, which the post replaces). [Not applied:
    /// Exarust's respawn keeps the surviving body.]
    PlayerData {
        flags: u32,
    },
    /// Turn the in-game GUI's frame hook (the HUD) on or off: each session
    /// start and continue turns it on once its level is in, the session's
    /// leave turns it off.
    GameGui(bool),
    /// The game GUI's sweep: the screens it names close, then the power
    /// tree's part ([`crate::gui_screens::sweep_screens`]). The death, the
    /// knock-out, the level exit, the story exit and the session leave
    /// sweep every screen with the shown power bar; the application's quit
    /// the container family.
    SweepScreens(crate::gui_screens::ScreenSweep),
    /// The death fade's reset: a fade that paused the game gives the game
    /// time back, and the fade ends ([`crate::fall::reset_death_fade`]);
    /// a running heartbeat then starts its fade-out, as the original's
    /// reset ends.
    ResetDeathFade,
    /// A running low-health heartbeat starts its 1 s fade-out
    /// ([`exa_ui::Heartbeat::start_fade`]): the death and the knock-out
    /// start it first.
    FadeHeartbeat,
    /// Write a wake-up or get-up pose.
    Wake(WakePose),
    /// The motion the player is asked to play.
    RequestAction(u32),
    /// The next fight's motion: the word is cleared first, so the clip starts
    /// again even when it is already playing, and the start keeps no capture,
    /// so it does not blend in from the pose the body held.
    SnapAction(u32),
    /// The next fight's behaviour reset, before the new opponent exists. The
    /// stored combat target and the contact list are dropped. Combat mode
    /// stays on, so the stance tick does not do this itself.
    ForgetTarget,
    /// The staging freeze: every AI tick runs the idle stand in place of its
    /// state until the transition screen has held and faded back in
    /// (`exa_ai::freeze`). The practice start and the next fight set it as
    /// they finish their staging, before the session hook.
    FreezeAi,
    /// The arena's staging callback ran at black: the end menu's next
    /// round (or tournament round), or the match start the hub's Begin and
    /// the rematch hold ([`crate::arena_fade`]). The match start changes
    /// the level, so the screen stays black over its load.
    ArenaStage(ArenaStage),
    /// Add the practice mode's player role.
    PracticePlayer {
        role: u32,
    },
    /// The practice start and the next practice fight dress the player's kit:
    /// preparation, then equip with no scene nodes. It runs once the player
    /// stands, which is when the record exists.
    DressPlayerKit,
    /// The next practice fight's preparation before the arena goes: the
    /// player's own items are copied into the kit's temporary database
    /// ([`crate::save_kit::KitScratch`]).
    PreparePlayerKit,
    /// Its equip once the arena is in again: the temporary database's items
    /// are copied into the new level's local tier.
    EquipPlayerKit,
    /// Stand the practice partner named by the ten-byte `opponentNN` buffer.
    SpawnPracticePartner {
        name: [u8; 10],
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
    /// The class select's cancel: the class select form is released and
    /// the flow goes back to the title. No scripted-mode hook runs.
    ClassCancel,
    /// The character creator's cancel: the creator form is released, a
    /// registered mode's creator-cancel hook runs (when its entry has one)
    /// and the flow goes back to the title.
    CreatorCancel,
    /// The arena end menu's Continue into the next round or the next
    /// tournament round: the round is staged at black and the session
    /// plays again.
    ArenaRound,
    /// The hub's Begin or the rematch: the match starts at black (its level
    /// loads behind the screen).
    ArenaMatchStart,
    /// The arena end menu's Continue after the match's last round: the
    /// match is settled at black, the hub's level loads behind the screen
    /// and the session plays again there.
    ArenaReturn,
}

/// Which arena staging a transition's callback runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaStage {
    /// The end menu's next round, standard or tournament.
    Round,
    /// The match start: the held launch goes out.
    MatchStart,
    /// The return to the hub: the held match is settled and the hub's level
    /// asked for.
    Return,
}

impl Action {
    /// The callback is a staging: it raises the pause counter (only while
    /// it is zero) for the screen to lower two frames later, and sets the
    /// AI staging freeze the screen's close clears.
    pub const fn stages(self) -> bool {
        matches!(
            self,
            Action::NextPracticeFight
                | Action::ArenaRound
                | Action::ArenaMatchStart
                | Action::ArenaReturn
        )
    }
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
    /// The callback was a staging: the pause counter is raised (when it is
    /// zero), so the simulation sleeps from here.
    pub raise_pause: bool,
    /// The second frame after the callback: the pause counter is lowered
    /// (when it is above zero).
    pub lower_pause: bool,
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
    /// The level's number: the save rule reads it ([`CheckpointRule`]).
    pub level: u32,
    /// The spawn point in the new level.
    pub spawn: u32,
    /// Bit 0 asks for the fade and the AI hand-over.
    pub flags: u32,
    /// The destination's name when the caller looked it up (an exit zone's
    /// `locales.rdb` name, [`intern_locale`]); `None` reads `level` as the
    /// campaign level ([`level_locale`]).
    pub name: Option<&'static str>,
}

/// The 16-byte header name a kind-7 use looks up, with `A`–`Z` lowered.
/// The original copies those sixteen bytes and adds `0x20` to each byte in
/// that range, then looks for that text with `.rfc` on it.
pub fn header_key(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            'A'..='Z' => char::from(c as u8 + 0x20),
            other => other,
        })
        .collect()
}

/// A level name made to live as long as the program, so a looked-up
/// destination can ride a [`Step`]: each distinct name is kept once.
pub fn intern_locale(name: &str) -> &'static str {
    static NAMES: std::sync::Mutex<Vec<&'static str>> = std::sync::Mutex::new(Vec::new());
    let mut names = NAMES
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(&kept) = names.iter().find(|&&kept| kept == name) {
        return kept;
    }
    let kept: &'static str = Box::leak(name.to_owned().into_boxed_str());
    names.push(kept);
    kept
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

/// Whether the in-game GUI is drawn over the world (the original's GUI frame
/// hook on the game's viewport): each session start and continue turns it
/// on once its level is in, and the session's leave turns it off.
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
    /// The level exit's portal bit set the transition screen's long hold
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
    /// The versus lobby is available: the arena page shows VERSUS. Exanima
    /// 0.9.5 never sets the original's flag; `--versus` sets this one.
    pub versus: bool,
    /// The scripted game modes registered so far (none in 0.9.5).
    pub script_modes: ScriptModes,
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
    /// The player body's measured stand height (cm) the practice start's
    /// caller hands it. The practice placements take the fresh pose's own
    /// heights instead, as the original reads them from the body as it
    /// places it ([`Step::PlacedFrame`]), so the flow does not read it.
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
            script_modes: ScriptModes::default(),
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
                // Any other word: the frame hook goes and a registered
                // mode's enter hook runs. An unregistered word (the versus
                // lobby in 0.9.5) only becomes the mode word.
                self.hook = Hook::None;
                steps.push(Step::Hook(Hook::None));
                if self.script_modes.find(word).is_some() {
                    steps.push(Step::ScriptMode {
                        mode: word,
                        hook: ScriptHook::Enter,
                    });
                }
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
        // A new story clears the fragments seen and keeps its background.
        self.story = StoryState {
            knockout_ms: u32::MAX - KNOCKOUT_MS,
            background,
            ..Default::default()
        };
        // The three backgrounds start a different wake-up: the unknown one
        // plays its animation, the villager gets up, and the knight and
        // proctor wait for a scripted motion. All three start three seconds
        // after the spawn.
        let wake_start = now.wrapping_add(WAKE_LEAD_MS);
        let motion = match background {
            Background::Unknown => {
                // The animating flag is set once the level is in
                // ([`Step::FirstWake`]): the load's GUI reset clears it.
                self.story.wake = Wake {
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
        // The game layer's leave first (its teardown is the load's own).
        let mut steps = self.leave_game_layer();
        steps.extend([
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
            // Once the level is in, the in-game GUI's frame hook goes on.
            Step::GameGui(true),
            Step::StartRecord(start.start_record),
            // The player controls are attached to the loaded player (their
            // disabled flag cleared) before the background's light and
            // motion.
            Step::Controls(true),
        ]);
        if let Some(light) = start.light {
            steps.push(Step::StoryLight(light));
        }
        // After the light the original clears the three wake flags and, for
        // the unknown background, sets the animating one: after its load,
        // whose GUI reset clears that flag.
        if background == Background::Unknown {
            steps.push(Step::FirstWake);
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
        // The table's frame with its vertical overwritten by the pose's
        // pelvis target plus 5 cm: over the origin, so the frame's floor is
        // 0 and the living placement adds the target the fresh pose gives
        // ([`Step::PlacedFrame`]). The caller's `standing` is not read: the
        // original reads the word from the body as it places it.
        let _ = standing;
        let placement = Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(PRACTICE_POINT.x, 0.0, PRACTICE_POINT.z),
        };
        self.viewport = viewport;
        let mut steps = self.leave_game_layer();
        steps.extend([
            Step::SessionTask(true),
            Step::LoadLevel {
                locale: start.locale,
                spawn: 0,
                teardown: true,
            },
            // Once the arena is in, the in-game GUI's frame hook goes on.
            Step::GameGui(true),
            // The arena is imported without a filter: the original passes
            // its practice scene filter to the named level load, which never
            // reads that argument.
            Step::PracticePlayer {
                role: PRACTICE_ROLE,
            },
            // The original prepares the player's kit before the arena import
            // and equips it, with no scene nodes, once the player is
            // registered. Both passes run here, once this player stands.
            Step::DressPlayerKit,
            Step::SpawnPracticePartner { name: step.name },
            // The controls are attached to the player, then combat mode is
            // turned on (the original skips it only in the story mode,
            // which never starts practice).
            Step::Controls(true),
            Step::CombatMode(true),
            Step::RequestAction(PRACTICE_MOTION),
            Step::PlacedFrame(crate::limp::FrameHeight::Target(PRACTICE_START_LIFT_CM)),
            Step::PlacePlayer(placement),
            Step::Camera(camera::frame_point(PRACTICE_POINT, true, false, viewport)),
            Step::Music {
                name: start.music.name,
                volume: start.music.volume,
                restart: true,
            },
            Step::FreezeAi,
            Step::Hook(Hook::Practice),
        ]);
        steps
    }

    /// Continues a story save: the menu form closes, the first wake-up's
    /// animating flag clears, the save's locale is loaded (or the first level
    /// when there is none), the player placed and nudged out of the exit
    /// that holds it ([`Step::ExitNudge`]), and the near shot framed on the
    /// player's body, fast.
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
            // Once the level is in, the in-game GUI's frame hook goes on.
            Step::GameGui(true),
            Step::PlacePlayer(player),
            Step::ExitNudge,
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
            // Once the hub is in, the in-game GUI's frame hook goes on.
            Step::GameGui(true),
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
    /// long hold is the level exit's portal bit, which the original sets on
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
        frame.lower_pause = step.lower_pause;
        transition.flags = screen.flags;
        transition.screen = Some(screen);
        self.transition = Some(transition);
        if step.callback {
            frame.raise_pause = transition.action.stages();
            frame.steps = self.apply(transition.action, now);
            let changes_level = frame.steps.iter().any(|step| {
                matches!(
                    step,
                    Step::LoadLevel { .. }
                        | Step::TeardownWorld
                        | Step::ArenaStage(ArenaStage::MatchStart | ArenaStage::Return)
                )
            });
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
            Action::ClassCancel => self.quit_to_menu(),
            Action::CreatorCancel => self.creator_cancel(),
            // The next round's routine ends with the playing phase.
            Action::ArenaRound => {
                self.phase = Phase::Playing;
                vec![Step::ArenaStage(ArenaStage::Round)]
            }
            // Exarust's match start keeps the hub's phase until the arena's
            // session marks it playing (`arena_live`).
            Action::ArenaMatchStart => {
                self.phase = self.saved_phase;
                vec![Step::ArenaStage(ArenaStage::MatchStart)]
            }
            // The return's routine clears the phase word, enters the hub and
            // ends with the playing phase.
            Action::ArenaReturn => {
                self.phase = Phase::Playing;
                vec![Step::ArenaStage(ArenaStage::Return)]
            }
            Action::Locale { locale, spawn } => {
                // The end menu and the prelude close, the pending level
                // loads, and the session plays again.
                let mut steps = self.level_loaded();
                steps.push(Step::LoadLevel {
                    locale,
                    spawn,
                    teardown: true,
                });
                steps
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
        // The once-a-second gate runs the pending task list only; the
        // discovery hook after it runs every frame the hook gets this far.
        let second = now / 1000;
        if second > self.story.last_discovery_s {
            self.story.last_discovery_s = second;
        }
        steps.push(Step::Discovery);
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
                // The branch runs only in the practice mode.
                let mut steps = Vec::new();
                if self.mode == Mode::Practice {
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
                    // The practice end menu blocks the controls (only in
                    // the practice mode, and only for the won ladder).
                    steps.push(Step::Block(true));
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
    /// The story scripts' `GameEvent` hook ([`crate::game_event::handle`]):
    /// the background-unlock event sets its bit in the background word and
    /// writes the whole word to the settings store at once (the store is
    /// flushed with it); an event whose bit is already set writes nothing.
    /// A fragment's bits join the session's fragment word, and an awakening
    /// sets the word whole. The hook runs whatever the mode, as the original
    /// registers it once at start-up. The other effects are returned, in the
    /// handler's order, for the session to apply to the player.
    pub fn game_event(
        &mut self,
        code: u32,
        story: crate::game_event::StoryContext,
        player: &crate::game_event::EventPlayer,
        draws: &mut dyn FnMut() -> f64,
    ) -> (Vec<Step>, Vec<crate::game_event::Effect>) {
        let effects =
            crate::game_event::handle(code, story, player, self.unlocks.backgrounds, draws);
        let mut steps = Vec::new();
        let mut rest = Vec::new();
        for effect in effects {
            match effect {
                crate::game_event::Effect::Unlock { bits } => {
                    self.unlocks.backgrounds |= bits;
                    steps.push(Step::SettingsWrite {
                        key: BACKGROUND_UNLOCK_KEY,
                        bytes: self.unlocks.background_bytes(),
                    });
                }
                crate::game_event::Effect::Collected(bits) => self.story.memory |= bits,
                crate::game_event::Effect::SetCollected(word) => self.story.memory = word,
                other => rest.push(other),
            }
        }
        (steps, rest)
    }

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
        // The table's frame with its vertical overwritten by the body's
        // stand height once the fight's motion has started: over the
        // origin, the living placement adds the fresh pose's stand height
        // ([`Step::PlacedFrame`]). (The original first writes the pelvis
        // target plus 5, or 13 with its combat byte, and overwrites it.)
        let placement = Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(PRACTICE_POINT.x, 0.0, PRACTICE_POINT.z),
        };
        let mut steps = self.close_end_menu();
        steps.extend([
            // The kit's preparation runs before the arena goes, as the
            // original's next fight prepares the player's items into the
            // temporary database and then clears the live local tier.
            Step::PreparePlayerKit,
            // The original clears the scene and imports the arena again
            // (the save unit's combatant preparation). It passes the
            // practice scene filter, which that reload never reads, so no
            // node is filtered. Exarust reloads the arena from the archive
            // in process.
            Step::LoadLevel {
                locale: match self.practice.ladder {
                    Ladder::Novice => "arena",
                    Ladder::Expert => "arenae",
                },
                spawn: 0,
                teardown: true,
            },
            // The next fight equips the player's prepared kit, with no
            // scene nodes, into the new arena's local tier before it places
            // the player. The behaviour reset then drops the stored target,
            // before the partner exists.
            Step::EquipPlayerKit,
            Step::ForgetTarget,
            Step::SpawnPracticePartner { name: step.name },
            Step::MenuHelp(false),
            // Combat mode goes on for the fight; the controls were never
            // taken (a won round leaves them).
            Step::CombatMode(true),
            Step::Health {
                red_bits: step.red_bits,
                yellow_bits: step.yellow_bits,
            },
            // The next fight starts from a clear condition word, then the
            // Defence stance again: the motion word is cleared first, so a
            // body already in it restarts, with no blend from the pose it held.
            Step::Condition(0),
            Step::SnapAction(PRACTICE_MOTION),
            Step::PlacedFrame(crate::limp::FrameHeight::Stand),
            Step::PlacePlayer(placement),
            Step::Camera(camera::frame_point(
                PRACTICE_POINT,
                false,
                false,
                self.viewport,
            )),
            Step::FreezeAi,
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
            Step::FadeHeartbeat,
            Step::EndGuiDrag,
            Step::SweepScreens(crate::gui_screens::ScreenSweep::EveryAndBar),
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
        // The death blocks the controls after its tip and the save's fate,
        // before the end menu is built.
        steps.push(Step::Block(true));
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
        // The knock-out blocks the controls after its tip.
        vec![
            Step::FadeHeartbeat,
            Step::EndGuiDrag,
            Step::SweepScreens(crate::gui_screens::ScreenSweep::EveryAndBar),
            Step::Controls(false),
            Step::Tip(2),
            Step::Block(true),
        ]
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

    /// The revived story player, behind the transition: the alarm return
    /// runs ([`Step::AlarmReturn`]), a session in a game
    /// mode is saved (without the death rule), combat mode is left,
    /// the dead body is raised and laid down, its wake held (0x40) and its
    /// wake clock pushed two seconds on, the controls
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
        // The world's revive pass runs before the save: the alarm return,
        // then every character's behaviour reset, the hold of every living
        // body and the drop of their kept pairs (`revive_pass`).
        let mut steps = vec![Step::ClosePrelude, Step::AlarmReturn, Step::ReviveBrains];
        if self.mode.is_game() {
            steps.push(Step::Save {
                drop_on_low_health: false,
            });
        }
        // Combat mode is left before the dead body is raised and laid down
        // (held, its wake two seconds later); the controls are enabled. (The
        // in-game GUI stayed on through the knock-out.)
        steps.extend([
            Step::CombatMode(false),
            Step::ReviveBody,
            Step::Wake(WakePose::revived()),
            // The block's release, after the wake is held.
            Step::Block(false),
            Step::Controls(true),
            Step::Hook(Hook::Story),
        ]);
        steps
    }

    /// Where a respawn puts the player: the respawn portal's frame lifted
    /// by [`PORTAL_LIFT_CM`], as the original's story respawn and its save
    /// load of a pending respawn place the player. `None` while no portal
    /// is known.
    pub fn portal_placement(&self) -> Option<Placement> {
        self.respawn_portal.map(portal_lifted)
    }

    /// The story's death screen CONTINUE: the player node's story bit
    /// clears, the save is written (without the death rule), the world torn
    /// down and `exanima05` loaded (the loader's second word, 1, is not
    /// traced and is read as the load's own flag) with the session-start
    /// arrival word, which names the level's reconstruction device as the
    /// player's object; the player stands at that respawn portal (its
    /// pose, or the identity at the origin when the level has none) 15 cm
    /// higher ([`Step::PlaceAtPortal`]), the controls are attached to it
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
        let without = portal_lifted(Placement {
            rotation: NO_ROTATION,
            position: Vec3::ZERO,
        });
        vec![
            // The respawn's scene reset starts with the death fade's reset.
            Step::ResetDeathFade,
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
            Step::PlaceAtPortal(crate::level_change::PortalArrival {
                without: Some(without),
                settle: 0,
            }),
            // Then the player's node tree settles for 16 ticks, wherever
            // it was placed.
            Step::SettlePlayer(exa_physics::body::PLACED_SETTLE),
            Step::Hook(Hook::Story),
        ]
    }

    /// The game layer's leave before a new session: the world goes and the
    /// game's view form is shown again (out of the creator's view). The
    /// in-game GUI is not touched: each start turns its frame hook on once
    /// its level is in ([`Step::GameGui`]). The original does it only when
    /// its game object exists. [Exarust draws its 3D view whatever form
    /// shows, so the view form has no step.]
    pub fn leave_game_layer(&mut self) -> Vec<Step> {
        vec![Step::TeardownWorld]
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

    /// The in-game page's EXIT in the arena, as the main menu's click runs
    /// it: the arena's exit ([`Step::ArenaExit`]: the screens closed, the
    /// unfit specialist's departure, the save, the arena's leave), then the
    /// leave to the title behind the transition. [A fourth word the click
    /// clears before the transition is not traced.]
    pub fn arena_exit(&mut self) -> Vec<Step> {
        let mut steps = vec![Step::ArenaExit];
        steps.extend(self.transition(Action::QuitToMenu, true));
        steps
    }

    /// The story end menu's EXIT row: the game screens and the dialogue
    /// close.
    /// A player the reconstruction device stripped clears its story bit and
    /// is saved without the death rule; any other story writes its save with
    /// the death rule, then leaves to the title behind the transition.
    pub fn story_exit(&mut self) -> Vec<Step> {
        // The death fade is reset first, as the original's story leave does.
        let mut steps = vec![
            Step::ResetDeathFade,
            Step::SweepScreens(crate::gui_screens::ScreenSweep::EveryAndBar),
            Step::Controls(false),
            // Every conversation ends (the original's third call).
            Step::EndDialogues,
        ];
        if self.story.reconstruction == STRIPPED {
            steps.push(Step::ClearStoryFlag);
            steps.push(Step::Save {
                drop_on_low_health: false,
            });
        } else {
            steps.push(Step::Save {
                drop_on_low_health: true,
            });
            // The quit hook the Mind sight registers while it shows.
            steps.push(Step::ResetMindSight);
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

    /// The level-load action behind the transition, before its load: the end
    /// menu and the prelude close and the session is playing again. (The
    /// original also cancels the glow pulses, clears the pending exit words
    /// and runs the experience's level-entry hook, which are the host's:
    /// `glow_pulse`, `level_change`, `experience_live`.)
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
        // A new story or career first makes its empty slot in the mode's
        // manager, before the mode's start; practice and the scripted modes
        // make none.
        if matches!(self.mode, Mode::Story | Mode::Arena) {
            starter.insert(0, Step::NewSlot);
        }
        // The begin itself does not touch the menu form: the starter's level
        // load hides it (see `Step::LoadLevel`).
        self.menu_open = false;
        self.end_menu = None;
        // A registered scripted mode's begin hook stands where the arena,
        // story and practice starts stand, before the frame task starts.
        if let Mode::Other(word) = self.mode
            && self.script_modes.find(word).is_some()
        {
            starter.push(Step::ScriptMode {
                mode: word,
                hook: ScriptHook::Begin,
            });
        }
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
            // The container family's screens close first.
            steps.push(Step::SweepScreens(
                crate::gui_screens::ScreenSweep::ContainerFamily,
            ));
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

    /// Leaving a session: the hook and the frame task stop, the interface
    /// goes, every conversation ends, the prelude closes, the world is torn
    /// down and the player pointers are cleared. A scripted mode's end hook
    /// is named.
    pub fn leave_session(&mut self) -> Vec<Step> {
        // The original's leave sweeps the game screens, ends every
        // conversation, then resets the death fade; the in-game GUI's frame
        // hook goes near its end, before the world. [guess: the session
        // task's place is not traced]
        let mut steps = vec![
            Step::Hook(Hook::None),
            Step::SessionTask(false),
            Step::SweepScreens(crate::gui_screens::ScreenSweep::EveryAndBar),
            Step::EndDialogues,
            Step::ResetDeathFade,
            Step::ClosePrelude,
        ];
        self.hook = Hook::None;
        match self.mode {
            Mode::Arena => steps.push(Step::ScriptMode {
                mode: ARENA_MODE,
                hook: ScriptHook::Leave,
            }),
            // Another mode with the scripted bit runs its registered leave
            // hook; the bit is tested first, then the table.
            Mode::Other(word)
                if word & SCRIPT_MODE_BIT != 0 && self.script_modes.find(word).is_some() =>
            {
                steps.push(Step::ScriptMode {
                    mode: word,
                    hook: ScriptHook::Leave,
                });
            }
            _ => {}
        }
        // After the mode's end hook: the save package closes and the end
        // menu hides, then the world goes.
        steps.push(Step::CloseSave);
        steps.push(Step::HideEndMenu);
        steps.push(Step::GameGui(false));
        steps.push(Step::TeardownWorld);
        self.player = [false; 3];
        self.end_menu = None;
        steps
    }

    /// The story level change (a trigger, a portal or a script). Nothing
    /// happens during a transition or for a level the locale table does not
    /// name. Otherwise a player whose condition has neither dead bit loses
    /// the controls, the game screens close, the save rule runs (every exit
    /// saves; early levels also keep a checkpoint) and the new level loads at
    /// the exit's spawn point behind the transition. The portal bit also gives
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
        let Some(locale) = exit.name.or_else(|| level_locale(exit.level)) else {
            return Vec::new();
        };
        let mut steps = Vec::new();
        if player_condition & DEAD == 0 {
            steps.push(Step::Controls(false));
        }
        steps.push(Step::EndGuiDrag);
        steps.push(Step::SweepScreens(
            crate::gui_screens::ScreenSweep::EveryAndBar,
        ));
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
            steps.push(Step::PortalUse);
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

    /// The class select's cancel: behind the transition the class select
    /// form is released and the session leaves to the title. The original
    /// runs it as the transition's action, at black, and runs no
    /// scripted-mode hook (the class select is the story's).
    pub fn cancel_class_select(&mut self, fade_audio: bool) -> Vec<Step> {
        self.transition(Action::ClassCancel, fade_audio)
    }

    /// The character creator's cancel (its first page's BACK and its cancel
    /// key, outside the arena recruit): behind the transition, with the music
    /// left playing, the creator form is released, a registered mode's
    /// creator-cancel hook runs and the session leaves to the title.
    pub fn cancel_creator(&mut self) -> Vec<Step> {
        self.transition(Action::CreatorCancel, false)
    }

    /// The creator cancel's action: the mode's entry is looked up by word
    /// alone (no scripted-bit test) and its cancel slot runs when the entry
    /// fills it; then back to the title, whose session leave runs the
    /// mode's leave hook.
    fn creator_cancel(&mut self) -> Vec<Step> {
        let mut steps = Vec::new();
        let word = self.mode.word();
        if let Some((_, entry)) = self.script_modes.find(word)
            && entry.cancel
        {
            steps.push(Step::ScriptMode {
                mode: word,
                hook: ScriptHook::Cancel,
            });
        }
        steps.extend(self.quit_to_menu());
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
    has_sound.then(|| exa_ui::menu::menu_sound_pitch(draw))
}

/// The volume the end menu's click sound plays at (the main menu's cue
/// gain).
pub const HOVER_VOLUME: f32 = exa_ui::menu::MENU_SOUND_GAIN;

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

/// The flow's frame systems other plugins order against: the transition
/// that runs a pending action (and queues its steps), then the application
/// of the queued steps.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FlowSystems {
    Transition,
    ApplySteps,
}

impl Plugin for FlowPlugin {
    fn build(&self, app: &mut App) {
        exa_physics::sim_app::add_fixed_commands(app);
        app.init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<AfterLoad>()
            .init_resource::<crate::save_kit::KitScratch>()
            .init_resource::<crate::left_behind::LeftBody>()
            .init_resource::<crate::level_change::SessionArrival>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<exa_script::ScriptEffect>()
            // What the story events' effects go through.
            .add_message::<exa_script::Narrate>()
            .add_message::<exa_combat::pain::PainSounded>()
            .add_message::<exa_audio::PointSounded>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            // The session leave's end of the conversations and the camera
            // shots' pointer warp.
            .add_message::<exa_script::EndAllConversations>()
            .add_message::<crate::os_pointer::WarpPointer>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .init_resource::<GameView>()
            .init_resource::<BootToMenu>()
            .init_resource::<crate::transition_screen::TransitionOverlay>()
            .add_message::<MenuChoice>()
            .add_message::<ReadNamedLevel>()
            .init_resource::<crate::doll_view::DollItems>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_systems(Startup, boot)
            .add_systems(
                Update,
                (
                    (apply_menu_choices, tick, follow_player).chain(),
                    notify_gui_scale.before(FlowSystems::ApplySteps),
                    (
                        open_named_level,
                        travel,
                        cross_exit_zones,
                        crate::game_event_live::run_game_events,
                        run_transition.in_set(FlowSystems::Transition),
                        release_after_load,
                        apply_steps.in_set(FlowSystems::ApplySteps),
                        reset_level_state,
                    )
                        .chain(),
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
    fixed: Option<ResMut<exa_physics::sim_app::FixedCommands>>,
    mut hub: Option<ResMut<crate::arena_hub_live::ArenaHub>>,
) {
    let Some(changes) = changes else {
        return;
    };
    if changes.changes == *seen {
        return;
    }
    *seen = changes.changes;
    // The room lists belong to the brains' tick: reset at the next step.
    if let Some(mut fixed) = fixed {
        fixed.post(|world| {
            if let Some(mut rooms) = world.get_resource_mut::<exa_ai::rooms::RoomLists>() {
                *rooms = exa_ai::rooms::RoomLists::default();
            }
        });
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
    after: Option<Res<AfterLoad>>,
) {
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    // While a load's steps wait, the player is not in the level yet (the
    // original's hook first runs after its load and start writes).
    let held = after.is_some_and(|after| !after.steps.is_empty());
    let player = players
        .single()
        .ok()
        .filter(|_| !held)
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
/// installs the hook itself. With [`StorySession`] the level runs as a
/// story-mode session with the story hook instead.
fn follow_player(
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    menu: Option<Res<MenuScreen>>,
    players: Query<(), Added<crate::player::PlayerBody>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut clock: Option<MessageWriter<crate::world_clock::RestoreWorldClock>>,
    story: Option<Res<StorySession>>,
) {
    if players.is_empty() || flow.mode.is_game() {
        return;
    }
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
    let mode = if story.is_some() {
        Mode::Story
    } else {
        Mode::Practice
    };
    for step in flow.switch_mode(mode, now) {
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
    flow.practice.start_ms = now;
    if story.is_some() {
        // The story hook runs: its knock-out, revive and death.
        info!("game flow: a play level is running as a story-mode session");
    } else {
        flow.hook = Hook::None;
        debug!("game flow: a play level is running as a practice-mode session");
    }
}

/// A debug aid (`--story-session`): a play level the flow follows runs as
/// a story-mode session, so the story hook's knock-out, revive (with its
/// alarm return) and death run as in the story, instead of a practice-mode
/// session without a hook.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct StorySession;

/// A kind-7 use asks to load the level its header names.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadNamedLevel {
    pub id: u32,
}

/// Loads the level a kind-7 use named, when `name.rfc` is in the resource
/// pack. A missing record, an empty name or a missing file leaves the
/// current level.
fn open_named_level(
    mut reads: MessageReader<ReadNamedLevel>,
    game: Option<Res<exa_assets::Game>>,
    mut items: ResMut<crate::doll_view::DollItems>,
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
) {
    items.ensure(game.as_deref());
    for read in reads.read() {
        let Some(name) = items.get(read.id).map(|item| header_key(&item.name)) else {
            continue;
        };
        if name.is_empty() {
            continue;
        }
        let file = format!("{name}.rfc");
        let exists = game.as_ref().is_some_and(|game| {
            game.0
                .pack(exa_assets::PackKind::Resource)
                .is_some_and(|pack| pack.get(&file).is_some())
        });
        if !exists {
            continue;
        }
        for step in flow.leave_to_locale(intern_locale(&name), true) {
            info!("game flow: a read use asks for {name}: {step:?}");
            queued.steps.push(step);
        }
    }
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
            // The journey's level change passes no flags: no portal use, no
            // AI hand-over and no long transition (the bubble has already
            // drawn the journey).
            LevelExit {
                level: journey.level,
                spawn: journey.arrival,
                flags: 0,
                name: None,
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

/// An exit zone the player entered is the game's level change (the
/// original queues it as a deferred call from the zone check). Its level
/// is a `locales.rdb` id and the save rule reads it as such; the
/// checkpoint setting is the settings' (on without them), and the save is
/// taken to hold neither `exanima04` nor the destination yet. The
/// destination's level-entry music check reads the track the music's main
/// voice plays.
fn cross_exit_zones(
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    mut crossed: Option<MessageReader<crate::zone_scripts::ExitZoneCrossed>>,
    audio: Option<Res<exa_audio::AudioEnvironment>>,
    config: Option<Res<crate::hw_config::HardwareConfig>>,
) {
    let Some(crossed) = crossed.as_mut() else {
        return;
    };
    let playing = audio.as_ref().map_or("", |audio| audio.music_name());
    let setting = config.as_ref().is_none_or(|c| c.live.checkpoints);
    for exit in crossed.read() {
        let name = exit.destination.as_deref().map(intern_locale);
        let music_changes = name.is_some_and(|locale| {
            exa_audio::story_music_changes_for_key(locale.as_bytes(), playing)
        });
        let steps = flow.level_exit(
            LevelExit {
                level: exit.level,
                spawn: exit.entrance,
                flags: u32::from(exit.flagged),
                name,
            },
            CheckpointRule {
                level: exit.level,
                setting,
                already_saved: false,
            },
            exit.condition,
            music_changes,
        );
        if steps.is_empty() {
            info!(
                "game flow: the exit zone to level {} ({:?}) changes nothing",
                exit.level, exit.destination
            );
        }
        for step in steps {
            info!(
                "game flow: the exit zone asks for level {} ({:?}, entrance {}): {step:?}",
                exit.level, exit.destination, exit.entrance
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
    (mut pause, mut fixed): (
        Option<ResMut<exa_physics::sim_app::SimPause>>,
        Option<ResMut<exa_physics::sim_app::FixedCommands>>,
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
    // A staging's callback raises the pause counter while it is zero; the
    // screen's second frame after the callback lowers it while it is above.
    if let Some(pause) = pause.as_mut() {
        if frame.raise_pause && pause.raised() == 0 {
            pause.raise();
            debug!("game flow: the staging raised the pause counter");
        }
        if frame.lower_pause && pause.raised() > 0 {
            pause.lower();
            debug!("game flow: the transition lowered the pause counter");
        }
    }
    // The close clears the AI staging freeze, as the original's screen
    // clears its byte when the fade back in ends.
    if frame.closed {
        exa_ai::freeze::release(fixed.as_deref_mut(), None);
    }
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
/// CONTINUE rows) to [`crate::save_screen`]; the manual to
/// [`crate::help_live`].
fn apply_menu_choices(
    mut choices: MessageReader<MenuChoice>,
    mut flow: ResMut<Flow>,
    mut queued: ResMut<FlowSteps>,
    arena_match: Option<Res<crate::arena_match::ArenaMatch>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
) {
    let now = ticks.as_ref().map_or(0, |ticks| ticks.millis() as u32);
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
            | MenuCommand::NewCareer => Vec::new(),
            // VERSUS switches the mode word to the versus lobby's before the
            // creator opens: the frame hook goes, and a registered mode's
            // enter hook would run (none is registered in 0.9.5).
            MenuCommand::Versus => {
                info!("game flow: VERSUS: the mode becomes {VERSUS_MODE:#x}");
                flow.switch_mode(Mode::Other(VERSUS_MODE), now)
            }
            MenuCommand::ExitArena if flow.mode == Mode::Arena => flow.arena_exit(),
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
            // The manual ([`crate::help_live`]) and the Settings form
            // ([`crate::settings_form`]) open from the same choice.
            MenuCommand::OpenManual | MenuCommand::OpenSettings => Vec::new(),
        };
        for step in &steps {
            debug!("game flow: menu {command:?}: {step:?}");
        }
        queued.steps.extend(steps);
    }
}

/// Hides the tips popup: the counts stay, the banner goes.
fn hide_tip(
    tips: Option<&mut crate::prelude::Tips>,
    banner: Option<&mut crate::prelude::TipBannerText>,
) {
    if let Some(tips) = tips {
        tips.hide();
    }
    if let Some(banner) = banner {
        banner.0.clear();
    }
}

/// Starts `motion` on every player. `snap` is the next fight: the clip
/// starts even when it is already playing, and the start's capture is
/// dropped so the body does not blend in from the pose it held.
fn request_action(
    motions: &Option<Res<exa_anim::Motions>>,
    players: &mut Query<PlayerParts, With<crate::player::PlayerBody>>,
    motion: u32,
    snap: bool,
) {
    for mut player in players.iter_mut() {
        let Some(animator) = player.animator.as_mut() else {
            continue;
        };
        // Already in the clip: leave it, unless the start is forced.
        if !snap
            && motions.as_deref().and_then(|set| set.action_name(motion))
                == Some(animator.action.as_str())
        {
            continue;
        }
        // The body tick starts it when the body is free, and drops it when
        // the body is busy or fallen. A forced start also drops the capture.
        animator.requested = motion as i32;
        animator.snap_start = snap;
        debug!("game flow: requested motion {motion:#x}");
    }
}

/// The window's resize notification, as the game main unit registers it at
/// boot ([`BootTask::GuiScale`]): the GUI scale is taken again from the
/// window's height and, when it moved, both menus lay out again
/// ([`Flow::gui_scale_changed`]). The first frame takes the boot's scale.
fn notify_gui_scale(
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    flow: Option<Res<Flow>>,
    mut steps: ResMut<FlowSteps>,
    mut last: Local<Option<f32>>,
) {
    let (Some(flow), Ok(window)) = (flow, windows.single()) else {
        return;
    };
    let height = window.physical_height() as i32;
    let Some(previous) = *last else {
        *last = Some(gui_scale(height));
        return;
    };
    let out = flow.gui_scale_changed(previous, height);
    if !out.is_empty() {
        *last = Some(gui_scale(height));
        steps.steps.extend(out);
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
    mut tips: Option<ResMut<crate::prelude::Tips>>,
    mut banner: Option<ResMut<crate::prelude::TipBannerText>>,
    mut logo: Option<ResMut<TitleLogo>>,
    mut view: Option<ResMut<GameView>>,
    mut session: Option<ResMut<crate::player_controls::PlaySession>>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut players: Query<PlayerParts, With<crate::player::PlayerBody>>,
    motions: Option<Res<exa_anim::Motions>>,
    mut cameras: Query<&mut crate::player::ChaseCamera>,
    mut out: FlowOutputs,
    mut story: StoryParts,
) {
    if queued.steps.is_empty() {
        return;
    }
    let mut steps = core::mem::take(&mut queued.steps);
    // What follows a level load acts on the loaded level and its player:
    // the original runs it after its load returns. It waits in
    // [`AfterLoad`] until the change is in and the player stands in it; a
    // placement is carried across the load by its own resource.
    if let Some(load) = steps
        .iter()
        .position(|step| matches!(step, Step::LoadLevel { .. }))
    {
        let tail = steps.split_off(load + 1);
        let (now_steps, later): (Vec<Step>, Vec<Step>) = tail.into_iter().partition(|step| {
            matches!(
                step,
                Step::PlacePlayer(_)
                    | Step::PlacedFrame(_)
                    | Step::PlaceAtPortal(_)
                    | Step::SettlePlayer(_)
                    | Step::ExitNudge
            )
        });
        steps.extend(now_steps);
        if !later.is_empty() {
            if let Some(after) = story.after_load.as_mut() {
                after.defer(later);
            } else {
                steps.extend(later);
            }
        }
    }
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
                // The end menu's hide also hides the tips popup.
                hide_tip(tips.as_deref_mut(), banner.as_deref_mut());
            }
            Step::ResetMindSight => {
                commands.queue(crate::mind::draw::reset_sense_overlay);
            }
            Step::ResetDeathFade => {
                if let Some(fade) = story.fade.as_mut() {
                    crate::fall::reset_death_fade(fade, story.time.as_deref_mut());
                }
                fade_heartbeat(&mut story);
            }
            Step::FadeHeartbeat => fade_heartbeat(&mut story),
            Step::ClosePrelude => {
                hide_tip(tips.as_deref_mut(), banner.as_deref_mut());
            }
            Step::Tip(index) => {
                if let Some(tips) = tips.as_mut() {
                    let shown = tips.show(usize::try_from(index).unwrap_or(usize::MAX));
                    if let Some(banner) = banner.as_mut() {
                        banner.0 = shown
                            .and_then(crate::prelude::banner_text)
                            .unwrap_or_default()
                            .to_string();
                    }
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
                hide_tip(tips.as_deref_mut(), banner.as_deref_mut());
                crate::level_change::request(&mut out.changes, Some(locale), spawn, teardown);
            }
            Step::TeardownWorld => {
                // The world reset clears what it owns (the weapons' blood
                // list) whether or not a load follows.
                out.resets.write(exa_world::level_lifecycle::WorldReset);
                // A load in this batch tears down by itself.
                if !loads {
                    crate::level_change::request(&mut out.changes, None, 0, true);
                }
            }
            Step::PlacePlayer(placement) => {
                commands.insert_resource(crate::level_change::PlayerPlacement::of_flow(&placement));
            }
            Step::PlaceAtPortal(arrival) => {
                commands.insert_resource(arrival);
            }
            Step::ExitNudge => {
                commands.insert_resource(crate::level_change::NudgeOutOfExits);
            }
            Step::SettlePlayer(ticks) => {
                commands.insert_resource(crate::level_change::PlacedSettle(ticks));
            }
            Step::PlacedFrame(height) => {
                commands.insert_resource(crate::level_change::PlacedFrame(height));
            }
            Step::EndDialogues => {
                // The dialogue engine runs in its own system: the end reaches
                // it as a message, read before its next update.
                commands.queue(|world: &mut World| {
                    if let Some(mut ends) =
                        world.get_resource_mut::<Messages<exa_script::EndAllConversations>>()
                    {
                        ends.write(exa_script::EndAllConversations);
                    }
                });
            }
            Step::FirstWake => {
                commands.queue(|world: &mut World| {
                    if let Some(mut flow) = world.get_resource_mut::<Flow>() {
                        flow.story.wake.animating = true;
                        info!("game flow: the first wake-up's animation is watched");
                    }
                });
            }
            Step::PracticePlayer { role } => {
                // The practice start clears the player's roles and adds the
                // practice role (with the parents it inherits) before the
                // player is spawned; a player the level load spawned already
                // holds it, a surviving one takes it here.
                if let Some(book) = story.roles.as_deref() {
                    for player in &mut players {
                        match player.roles {
                            Some(mut roles) => {
                                *roles = exa_ai::roles::Roles::only(&book.book, role)
                            }
                            None => {
                                commands
                                    .entity(player.entity)
                                    .insert(exa_ai::roles::Roles::only(&book.book, role));
                            }
                        }
                    }
                }
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
            Step::GuiScale(scale) => {
                // Both menus lay out again: the title menu redraws its
                // cached rows; the end menu lays out from the window at
                // every draw.
                if let Some(menu) = menu.as_mut() {
                    menu.relayout();
                }
                info!("gui scale: {scale} (the menus lay out again)");
            }
            Step::TitleLogo(shown) => {
                if let Some(logo) = logo.as_mut() {
                    logo.0 = shown;
                }
                // The return to the title sets the title flag with the
                // logo: the version line shows and the frame-rate readout
                // (cleared by the session's leave) does not.
                if shown && let Some(overlays) = out.overlays.as_mut() {
                    overlays.enter_title();
                }
            }
            Step::SessionTask(true) => {
                // Every session start and continue clears the title flag and
                // hands the viewport's frame-rate flag the ShowFPS setting.
                let show = out.show_fps.as_ref().is_none_or(|show| show.0);
                if let Some(overlays) = out.overlays.as_mut() {
                    overlays.start_session(show);
                }
                debug!("game flow step: SessionTask(true)");
            }
            Step::SweepScreens(sweep) => {
                commands.queue(move |world: &mut World| {
                    crate::gui_screens::sweep_screens(world, sweep);
                });
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
            Step::EndGuiDrag => {
                commands.queue(crate::controls_live::EndGuiDrag::ask);
            }
            Step::Block(on) => {
                if let Some(control) = out.control.as_deref_mut() {
                    crate::controls_live::block_controls(control, on, "flow");
                }
            }
            Step::Camera(frame) => {
                // The shot ends with its cursor warp while the window has
                // the focus (the original's gate is the window's active
                // flag): the client point the shot framed, in physical
                // pixels, posted with the warp hold.
                let warp = frame.warp;
                commands.queue(move |world: &mut World| {
                    let mut windows =
                        world.query_filtered::<&Window, With<bevy::window::PrimaryWindow>>();
                    let Some(scale) = windows
                        .iter(world)
                        .find(|window| window.focused)
                        .map(Window::scale_factor)
                    else {
                        return;
                    };
                    if let Some(mut warps) =
                        world.get_resource_mut::<Messages<crate::os_pointer::WarpPointer>>()
                    {
                        let to = Vec2::new(warp.0 as f32, warp.1 as f32) / scale;
                        warps.write(crate::os_pointer::WarpPointer(to));
                        debug!("camera shot: the pointer is warped to {to}");
                    }
                });
                // The shot places the chase camera as the original's
                // placement does: the bearing of the eye from the target,
                // and a fast shot's distance as the zoom. The view position
                // is not taken (the chase camera keeps its own distance and
                // pitch rules).
                for mut camera in &mut cameras {
                    camera.face_shot(frame.camera, frame.target, frame.distance);
                    info!(
                        "camera shot: eye {} target {} distance {:?}: yaw {:.6} zoom {}",
                        frame.camera,
                        frame.target,
                        frame.distance,
                        camera.yaw,
                        camera.zoom()
                    );
                }
            }
            Step::CombatMode(on) => {
                for mut player in &mut players {
                    // Entering combat mode drops the double-click clock and
                    // the walk bits. A mode already on keeps both.
                    if player.body.set_combat(on) {
                        if let Some(req) = player.locomotion.as_mut() {
                            req.entered_combat();
                        }
                        // The entry closes every in-game screen and the
                        // power tree, as Tab's does.
                        commands.queue(|world: &mut World| {
                            crate::gui_screens::sweep_screens(
                                world,
                                crate::gui_screens::ScreenSweep::Every,
                            );
                        });
                    }
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
                        out.fixed
                            .write(player.entity, body, |body| body.knock_out());
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
                        out.fixed.write(player.entity, body, |body| {
                            if body.drive.is_some() {
                                body.knock_out();
                            } else {
                                // A limp body gets its muscles back over the
                                // race and stats it had (`crate::rise` gives
                                // its clips and controller back).
                                let drive = body.revival_drive().unwrap_or_default();
                                body.revive(drive);
                            }
                        });
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
                    if let Some(body) = player.ragdoll.as_mut() {
                        let (stance, tone) = (pose.move_factor, pose.turn_factor);
                        out.fixed.write(player.entity, body, move |body| {
                            if let Some(drive) = body.drive.as_mut() {
                                if let Some(stance) = stance {
                                    drive.set_stance(stance);
                                }
                                if let Some(tone) = tone {
                                    drive.tone = tone;
                                }
                            }
                        });
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
                    if let (Some(tone), Some(body)) = (turn_factor, player.ragdoll.as_mut()) {
                        out.fixed.write(player.entity, body, move |body| {
                            if let Some(drive) = body.drive.as_mut() {
                                drive.tone = tone;
                            }
                        });
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
            Step::RequestAction(motion) => {
                // Asked, not started. The body tick plays it when the body
                // is free and drops it when the body is busy or fallen. A
                // body already in that action is left as it is.
                request_action(&motions, &mut players, motion, false);
            }
            Step::SnapAction(motion) => {
                // The next fight: the same request, and the start drops the
                // capture so the clip does not blend in from the old pose.
                request_action(&motions, &mut players, motion, true);
            }
            Step::FreezeAi => {
                // The staging freeze, posted for the next step: held by the
                // transition screen it runs under, else counted.
                if flow.transition_running() {
                    exa_ai::freeze::stage_held(Some(&mut out.fixed), None);
                } else {
                    exa_ai::freeze::stage(Some(&mut out.fixed), None);
                }
            }
            Step::ArenaStage(stage) => {
                // The arena's staging runs in its own systems this frame
                // ([`crate::arena_fade`]).
                if let Some(staging) = out.arena.as_mut() {
                    staging.fire(stage);
                }
            }
            Step::ForgetTarget => {
                // The behaviour reset, before the new opponent exists: the
                // stored target and the contact list. Combat mode is left on.
                for mut player in &mut players {
                    player.body.forget_target();
                    if let Some(practice) = player.practice.as_mut() {
                        practice.contacts.clear();
                    }
                }
            }
            Step::Forfeit => {
                out.forfeit.write(ForfeitMatch);
            }
            Step::SettingsWrite { key, bytes } => {
                out.settings.write(SettingsEntry {
                    section: SETTINGS_SECTION,
                    key,
                    bytes,
                });
            }
            Step::Quit => {
                out.exit.write(AppExit::Success);
            }
            // The save unit's work on the current slot
            // ([`crate::save_live`], idle without a save folder).
            Step::Save { drop_on_low_health } => {
                out.saves
                    .write(crate::save_live::SaveStep::Save { drop_on_low_health });
            }
            Step::DropSave => {
                out.saves.write(crate::save_live::SaveStep::Drop);
            }
            Step::NewSlot => {
                out.saves.write(crate::save_live::SaveStep::NewSlot);
            }
            Step::PreparePlayerKit => {
                for mut player in &mut players {
                    let Some(character) = player.character.as_deref_mut() else {
                        continue;
                    };
                    let own = match player.kit.as_deref() {
                        Some(kit) => Some(&kit.0),
                        None => story.objects.as_deref().map(|live| live.db()),
                    };
                    let (prepared, temp) =
                        crate::save_kit::prepare_into(&mut character.record, own);
                    info!(
                        "practice kit: {} worn and {} carried own items prepared ({} reused a hand), {} records in the temporary database",
                        prepared.worn_resolved,
                        prepared.carried_resolved,
                        prepared.carried_reused,
                        temp.local.len()
                    );
                    if let Some(scratch) = story.scratch.as_mut() {
                        scratch.db = Some(temp);
                    }
                }
            }
            Step::EquipPlayerKit => {
                let temp = story
                    .scratch
                    .as_mut()
                    .and_then(|scratch| scratch.db.take())
                    .unwrap_or_default();
                for mut player in &mut players {
                    if let Some(character) = player.character.as_deref_mut() {
                        let own = match player.kit.as_deref_mut() {
                            Some(kit) => Some(&mut kit.0),
                            None => story.objects.as_deref_mut().map(|live| live.db_mut()),
                        };
                        let equipped = crate::save_kit::equip_from(
                            &mut character.record,
                            &mut [],
                            None,
                            &temp,
                            own,
                        );
                        info!(
                            "practice kit: equipped into the new local tier: hands {:x?}, {} worn, {} carried ({} reused a hand)",
                            equipped.hands,
                            equipped.worn_instantiated,
                            equipped.carried_instantiated,
                            equipped.carried_reused
                        );
                    }
                    if let (Some(character), Some(possessions)) = (
                        player.character.as_deref(),
                        player.possessions.as_deref_mut(),
                    ) {
                        crate::save_kit::mirror_kit(&character.record, possessions);
                    }
                }
            }
            Step::DressPlayerKit => {
                for mut player in &mut players {
                    // The player's own database when the body carries one,
                    // otherwise the level's local database. Neither loaded
                    // leaves own ids as they are.
                    if let Some(kit) = player.kit.as_deref_mut() {
                        if let Some(character) = player.character.as_deref_mut() {
                            crate::save_kit::dress_kit(
                                &mut character.record,
                                &mut [],
                                None,
                                Some(&mut kit.0),
                            );
                        }
                    } else if let Some(character) = player.character.as_deref_mut() {
                        crate::save_kit::dress_kit(
                            &mut character.record,
                            &mut [],
                            None,
                            story.objects.as_deref_mut().map(|live| live.db_mut()),
                        );
                    }
                    if let (Some(character), Some(possessions)) = (
                        player.character.as_deref(),
                        player.possessions.as_deref_mut(),
                    ) {
                        crate::save_kit::mirror_kit(&character.record, possessions);
                    }
                }
            }
            Step::SpawnPracticePartner { name } => {
                let partner = String::from_utf8_lossy(&name);
                let partner = partner.trim_end_matches('\0');
                if !partner.is_empty() {
                    out.bodies
                        .write(crate::scene_bodies::SceneCommand::Spawn(Box::new(
                            crate::scene_bodies::practice_partner_plan(partner, now),
                        )));
                }
            }
            Step::StripPlayer => {
                // The reconstruction device's strip
                // ([`crate::left_behind::leave_behind`]): the dead body keeps
                // a dressed copy of its record, in the world store, and the
                // player goes on with the stripped record, which the next
                // save writes as the player's.
                let level = story.roles.as_deref().map_or(0, |roles| roles.locale);
                for mut player in &mut players {
                    let Some(character) = player.character.as_mut() else {
                        continue;
                    };
                    let dressed = match player.possessions.as_deref() {
                        Some(possessions) => crate::save_live::record_with_possessions(
                            character.record.clone(),
                            possessions,
                        ),
                        None => character.record.clone(),
                    };
                    let strip = crate::left_behind::leave_behind(
                        dressed,
                        story.roles.as_deref().map(|roles| &roles.book),
                        story.world.as_deref_mut(),
                        level,
                    );
                    info!(
                        "game flow: the dead player's body keeps the dressed record {:#x} (level {level}); the player goes on stripped",
                        strip.corpse.id
                    );
                    // The body takes the copy; nothing is refitted (it
                    // stays dressed).
                    character.bypass_change_detection().record = strip.corpse;
                    if let Some(left) = story.left.as_mut() {
                        **left = crate::left_behind::LeftBody {
                            body: Some(player.entity),
                            player: Some(strip.player),
                            exported: false,
                        };
                    }
                }
            }
            Step::ClearStoryFlag => {
                // The body's owned mark goes: the next save writes it into
                // the level section. Without a strip the body's record is
                // the player's own (the world store's player slot).
                let level = story.roles.as_deref().map_or(0, |roles| roles.locale);
                for mut player in &mut players {
                    let Some(left) = story.left.as_mut() else {
                        continue;
                    };
                    if left.body != Some(player.entity) {
                        **left = crate::left_behind::LeftBody {
                            body: Some(player.entity),
                            player: None,
                            exported: true,
                        };
                        if let Some(character) = player.character.as_mut() {
                            let record = &mut character.bypass_change_detection().record;
                            record.id = exa_formats::characters::WORLD_DB
                                | crate::save_live::PLAYER_RECORD_ID;
                            record.locale = level;
                        }
                    }
                    left.exported = true;
                    info!(
                        "game flow: the player's body {} loses its owned mark; the save writes it into the level",
                        player.entity
                    );
                }
            }
            Step::Checkpoint => {
                out.saves.write(crate::save_live::SaveStep::Checkpoint);
            }
            Step::Discovery => {
                out.discovery
                    .write(crate::experience_live::DiscoveryTick { now });
            }
            Step::StoryPlayer {
                roles,
                flags,
                off_hand,
                carried,
                powers,
            } => {
                // A new story starts with an empty world tier, as the
                // original's new game clears it before the first level.
                if let Some(objects) = story.objects.as_mut() {
                    objects.db_mut().clear_world();
                }
                // The record the level load spawns the player from takes the
                // start's writes before the load, as the original's does.
                if let Some(record) = story.created.as_mut().and_then(|c| c.0.as_mut()) {
                    story_record(record, off_hand, carried, powers);
                }
                commands.insert_resource(StoryStartPlayer {
                    roles,
                    flags,
                    off_hand,
                    carried,
                });
            }
            Step::StartRecord(record) => {
                // The spawned player takes the request: its word, the
                // record's role list and gear, and the start motion.
                let start = story.start.as_deref().copied();
                for mut player in &mut players {
                    if let Some(start) = start {
                        match player.condition.as_mut() {
                            Some(condition) => condition.0 = start.flags,
                            None => {
                                commands
                                    .entity(player.entity)
                                    .insert(exa_combat::Condition(start.flags));
                            }
                        }
                        if let (Some(book), Some(roles)) =
                            (story.roles.as_deref(), player.roles.as_mut())
                        {
                            **roles = story_roles(&book.book, start.roles);
                        }
                        if let Some(possessions) = player.possessions.as_mut() {
                            story_possessions(possessions, start.off_hand, start.carried);
                        }
                    }
                    if let Some(animator) = player.animator.as_mut()
                        && motions
                            .as_deref()
                            .and_then(|set| set.action_name(record[0]))
                            != Some(animator.action.as_str())
                    {
                        animator.requested = record[0] as i32;
                        animator.snap_start = false;
                        debug!(
                            "game flow: start record motion {:#x} is requested",
                            record[0]
                        );
                    }
                }
                commands.remove_resource::<StoryStartPlayer>();
            }
            Step::StoryLight(light) => {
                // The start lays its torch in the world. A player that
                // carries Exarust's torch keeps it: the torch is not laid
                // twice, and where the story's torch starts stays the
                // owner's call (`--no-torch` gives the original's start).
                let carries = players.iter().any(|player| player.torch);
                if carries {
                    info!(
                        "game flow: the story start's torch (item {:#x} at {} cm, angles {}) stays in the player's hand",
                        light.item, light.position, light.rotation_degrees
                    );
                } else {
                    info!(
                        "game flow: the story start lays its torch (item {:#x}) at {} cm, angles {}",
                        light.item, light.position, light.rotation_degrees
                    );
                    commands.insert_resource(crate::torch::StoryTorch(Some(
                        crate::torch::story_torch_transform(&light),
                    )));
                }
            }
            Step::LevelTime => {
                // The level-6 checkpoint keeps the engine clock.
                commands.insert_resource(crate::save_live::LevelTime(now));
            }
            Step::PortalUse => {
                // The exit's portal bit: the portal use on the player (its
                // dissolve burst and sound, `effects::portal`).
                for player in players.iter() {
                    info!(
                        "game flow: the portal use on the player {:?}",
                        player.entity
                    );
                    commands.write_message(crate::effects::portal::PortalUse {
                        body: player.entity,
                    });
                }
            }
            Step::ArenaExit => {
                info!("game flow: the arena's exit");
                commands.queue(|world: &mut World| {
                    crate::gui_screens::sweep_screens(
                        world,
                        crate::gui_screens::ScreenSweep::EveryAndBar,
                    );
                    if let Some(mut career) =
                        world.get_resource_mut::<crate::hub_windows::ArenaCareer>()
                        && let Some(departure) = career.leave_before_match()
                    {
                        info!("arena exit: specialist left ({departure:?})");
                    }
                    world.write_message(crate::save_live::SaveStep::Save {
                        drop_on_low_health: false,
                    });
                });
                out.arena_leave.write(crate::arena_hub_live::ArenaLeave);
            }
            Step::ScriptMode {
                mode: ARENA_MODE,
                hook: ScriptHook::Leave,
            } => {
                // The arena's own leave ([`crate::arena_hub_live::arena_leave`]).
                info!("game flow: the arena's leave");
                out.arena_leave.write(crate::arena_hub_live::ArenaLeave);
            }
            Step::ScriptMode { mode, hook } => {
                // A registered scripted mode's hook. Exarust has no script
                // that registers one (nor does Exanima 0.9.5), so this only
                // names it.
                info!("game flow: scripted mode {mode:#x} hook {hook:?}");
            }
            Step::CloseSave => {
                // The original closes the save manager's open package file.
                // Exarust's save slots open a file per read or write and
                // drop it at the end of that operation, so no package is
                // open between steps: nothing is left to close.
                debug!("game flow: the save package is closed (no file stays open)");
            }
            other => {
                debug!("game flow step not applied by the app: {other:?}");
            }
        }
    }
}

/// The messages the flow's applied steps send: a level change, a world
/// reset, a forfeit, a
/// settings entry, the application's exit, the save unit's work and the
/// discovery pass.
#[derive(bevy::ecs::system::SystemParam)]
struct FlowOutputs<'w> {
    changes: MessageWriter<'w, exa_world::level_lifecycle::LevelChangeRequest>,
    resets: MessageWriter<'w, exa_world::level_lifecycle::WorldReset>,
    forfeit: MessageWriter<'w, ForfeitMatch>,
    settings: MessageWriter<'w, SettingsEntry>,
    exit: MessageWriter<'w, AppExit>,
    saves: MessageWriter<'w, crate::save_live::SaveStep>,
    discovery: MessageWriter<'w, crate::experience_live::DiscoveryTick>,
    bodies: MessageWriter<'w, crate::scene_bodies::SceneCommand>,
    arena_leave: MessageWriter<'w, crate::arena_hub_live::ArenaLeave>,
    /// The steps' writes to the player's body, posted for the next step
    /// ([`exa_physics::sim_app::FixedCommands::write`]).
    fixed: ResMut<'w, exa_physics::sim_app::FixedCommands>,
    /// The player-control singleton the block pair writes.
    control: Option<ResMut<'w, crate::player_controls::PlayerControl>>,
    /// The viewport's frame-rate and title flags, and the ShowFPS setting a
    /// session start hands the first.
    overlays: Option<ResMut<'w, crate::view_overlay::ViewOverlays>>,
    show_fps: Option<Res<'w, exa_ui::ShowFps>>,
    /// The arena's held stagings, which a transition's callback fires.
    arena: Option<ResMut<'w, crate::arena_fade::ArenaStaging>>,
}

/// The resources the story start's and the tips' steps write.
#[derive(bevy::ecs::system::SystemParam)]
struct StoryParts<'w> {
    created: Option<ResMut<'w, crate::creator_screen::CreatedCharacter>>,
    roles: Option<Res<'w, exa_ai::brain::RoleDatabase>>,
    start: Option<Res<'w, StoryStartPlayer>>,
    after_load: Option<ResMut<'w, AfterLoad>>,
    /// The live object database (its local tier the loaded level's), which
    /// the practice kit passes copy through when the player carries no
    /// database of its own.
    objects: Option<ResMut<'w, exa_assets::live_objects::LiveObjects>>,
    /// The kit's temporary database across the next practice fight's load.
    scratch: Option<ResMut<'w, crate::save_kit::KitScratch>>,
    /// The death fade a session's leave, the story's exit and its respawn
    /// reset, and the game time it gives back.
    fade: Option<ResMut<'w, exa_ui::DeathFade>>,
    time: Option<ResMut<'w, Time<Virtual>>>,
    /// The world character store the strip's copy joins, and the flow's
    /// record of the dead player's body ([`crate::left_behind`]).
    world: Option<ResMut<'w, crate::world_characters::WorldCharacters>>,
    left: Option<ResMut<'w, crate::left_behind::LeftBody>>,
    /// The low-health heartbeat the death, the knock-out and the fade
    /// reset fade out, on the real clock.
    heartbeat: Option<ResMut<'w, exa_ui::Heartbeat>>,
    real: Option<Res<'w, Time<Real>>>,
}

/// Starts a running heartbeat's fade-out on the real clock.
fn fade_heartbeat(story: &mut StoryParts) {
    if let (Some(beat), Some(real)) = (story.heartbeat.as_mut(), story.real.as_ref())
        && beat.start_fade(real.elapsed_secs_f64())
    {
        info!("the heartbeat fades out");
    }
}

/// The story start's writes the spawned player takes with the start
/// record ([`Step::StoryPlayer`], applied at [`Step::StartRecord`]).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct StoryStartPlayer {
    pub roles: &'static [u32],
    pub flags: u32,
    pub off_hand: Option<ItemRef>,
    pub carried: Option<Carried>,
}

/// The story start's record writes: the off hand's item, the carried
/// entry appended, the record's power word (the specialist domains,
/// [`CharacterRecord::extra_word`](exa_formats::characters::CharacterRecord::extra_word))
/// and the potential beside the power slots, which Exarust keeps in the
/// power block's word (written when the record has a block).
pub fn story_record(
    record: &mut exa_formats::characters::CharacterRecord,
    off_hand: Option<ItemRef>,
    carried: Option<Carried>,
    powers: Option<(u32, f32)>,
) {
    if let Some(item) = off_hand {
        record.hands[1] = item;
    }
    if let Some(entry) = carried {
        record.inventory.push(entry);
    }
    if let Some((word, potential)) = powers {
        record.extra_word = word;
        if let Some(block) = record.powers.as_mut() {
            block.word = potential.to_bits();
        }
    }
}

/// The start's role list on a cleared list: each id added in order, with
/// the parents it inherits.
pub fn story_roles(book: &exa_formats::roles::RoleDb, ids: &[u32]) -> exa_ai::roles::Roles {
    let mut roles = exa_ai::roles::Roles::default();
    for &id in ids {
        roles.add(book, id);
    }
    roles
}

/// The start's gear on a spawned body's possessions: the off hand's item
/// and the carried entry at its saved cell.
pub fn story_possessions(
    possessions: &mut exa_items::possession::Possessions,
    off_hand: Option<ItemRef>,
    carried: Option<Carried>,
) {
    if let Some(item) = off_hand {
        possessions.out[1] = exa_items::possession::Hand {
            id: item.id,
            flags: item.flags,
        };
    }
    if let Some(entry) = carried {
        possessions.carried.push(exa_items::possession::Carried {
            id: entry.item,
            flags: entry.flags,
            x: entry.extra as u16 as i16,
            y: (entry.extra >> 16) as u16 as i16,
        });
    }
}

/// How many frames steps held for a load wait for its loading to show
/// before they run anyway.
pub const AFTER_LOAD_WAIT_FRAMES: u32 = 600;
/// Quiet frames (no loading screen) after a load before its steps run.
pub const AFTER_LOAD_QUIET_FRAMES: u32 = 2;

/// The steps that follow a level load, held until the load is in and a
/// player stands in the level.
#[derive(Resource, Debug, Default)]
pub struct AfterLoad {
    pub steps: Vec<Step>,
    /// The load's loading screen has been seen.
    pub seen_loading: bool,
    /// Frames since the steps were held.
    pub frames: u32,
    /// Frames without loading since the loading was seen.
    pub quiet: u32,
}

impl AfterLoad {
    /// Holds a load's steps.
    pub fn defer(&mut self, steps: Vec<Step>) {
        self.steps.extend(steps);
        self.seen_loading = false;
        self.frames = 0;
        self.quiet = 0;
    }

    /// One frame: whether the held steps run now, from the loading state,
    /// whether a player stands in the level and whether a placement is
    /// still waiting for it.
    pub fn ready(&mut self, loading: bool, player: bool, placing: bool) -> bool {
        if self.steps.is_empty() {
            return false;
        }
        self.frames = self.frames.saturating_add(1);
        if loading {
            self.seen_loading = true;
            self.quiet = 0;
            return false;
        }
        self.quiet = self.quiet.saturating_add(1);
        if !self.seen_loading {
            return self.frames >= AFTER_LOAD_WAIT_FRAMES;
        }
        self.quiet >= AFTER_LOAD_QUIET_FRAMES && player && !placing
    }
}

/// Hands the steps held for a load back once the load is in.
fn release_after_load(
    mut after: ResMut<AfterLoad>,
    mut queued: ResMut<FlowSteps>,
    ready: Option<Res<exa_world::level_lifecycle::LevelChangeReady>>,
    status: Option<Res<exa_ui::LoadingStatus>>,
    placement: Option<Res<crate::level_change::PlayerPlacement>>,
    players: Query<(), With<crate::player::PlayerBody>>,
) {
    let loading = ready.is_some() || status.is_some();
    if after.ready(loading, !players.is_empty(), placement.is_some()) {
        let steps = core::mem::take(&mut after.steps);
        debug!("game flow: {} steps run after the load", steps.len());
        queued.steps.extend(steps);
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
    animator: Option<&'static mut exa_anim::Animator>,
    possessions: Option<&'static mut exa_items::possession::Possessions>,
    character: Option<&'static mut exa_character::record::CharacterData>,
    kit: Option<&'static mut crate::save_kit::OwnDatabase>,
    roles: Option<&'static mut exa_ai::roles::Roles>,
    practice: Option<&'static mut crate::player_practice::PlayerPractice>,
    locomotion: Option<&'static mut exa_physics::body::locomotion::Locomotion>,
    torch: Has<crate::torch::CarriesTorch>,
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
    fn a_header_name_is_lowered_for_the_level_file() {
        assert_eq!(header_key("Exanima01"), "exanima01");
        assert_eq!(header_key("already"), "already");
        assert_eq!(header_key("A_B"), "a_b");
        assert!(Flow::default().leave_to_locale("missing", false).is_empty());
    }

    #[test]
    fn hiding_the_tip_clears_the_banner_and_keeps_the_counts() {
        let mut tips = crate::prelude::Tips::default();
        let shown = tips.show(3);
        let mut banner = crate::prelude::TipBannerText(
            shown
                .and_then(crate::prelude::banner_text)
                .unwrap_or_default()
                .to_string(),
        );
        assert_eq!(banner.0, "You have died.");
        let counts = tips.shown;
        hide_tip(Some(&mut tips), Some(&mut banner));
        assert!(banner.0.is_empty());
        assert_eq!(tips.current, None);
        assert_eq!(tips.shown, counts);
    }

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
        assert_eq!(shot.distance, Some(1300.0));
        assert_eq!(shot.warp, (920, 370));
        let near = camera::frame_point(PRACTICE_POINT, false, false, view);
        assert_eq!(near.camera, Vec3::new(700.0, 1180.0, 1600.0));
        assert_eq!(near.distance, None);
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
        assert_eq!(light.item, 0x1f);
        assert_eq!(light.level, 256);
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
            position: Vec3::new(0.0, 0.0, 500.0),
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
        // The body stands at the table's point, its vertical the fresh
        // pose's stand height over the origin (the caller's reading is not
        // used).
        let place = steps
            .iter()
            .position(|step| {
                *step
                    == Step::PlacePlayer(Placement {
                        rotation: PRACTICE_ROTATION,
                        position: Vec3::new(0.0, 0.0, 500.0),
                    })
            })
            .expect("the player is placed");
        assert_eq!(
            steps[place - 1],
            Step::PlacedFrame(crate::limp::FrameHeight::Stand)
        );
        assert_eq!(flow.practice.fight, 8);
        assert_eq!(flow.phase, Phase::Playing);
        // The arena is imported again, with no scene filter.
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
        assert_eq!(steps[load + 1], Step::EquipPlayerKit);
        assert_eq!(steps[load - 1], Step::PreparePlayerKit);
        // The stored target is dropped before the partner exists.
        assert_eq!(steps[load + 2], Step::ForgetTarget);
        assert_eq!(
            steps[load + 3],
            Step::SpawnPracticePartner {
                name: *b"opponent16",
            }
        );
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
        // The round's condition word clears before the motion starts, and
        // the stance starts again with no blend from the pose it held.
        assert!(steps.contains(&Step::Condition(0)));
        assert!(steps.contains(&Step::SnapAction(PRACTICE_MOTION)));
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

    /// Leaving an arena session runs the arena's leave: the hub is busy and
    /// its click hook gone until the next entry.
    /// The in-game page's EXIT in the arena, through the live step pass:
    /// the unfit specialist leaves the company with his log record, the
    /// save is asked for without the death rule, the arena's leave runs
    /// (the hub busy, its click hook gone) and the transition starts.
    #[test]
    fn the_arenas_exit_sends_the_unfit_away_saves_and_leaves() {
        use crate::arena_hub::HubHook;
        use crate::arena_hub_live::ArenaHub;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::player_controls::PlaySession>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .insert_resource(ArenaHub {
                entered: true,
                busy: false,
                click_hook: HubHook::Click,
                ..Default::default()
            })
            .add_systems(
                Update,
                (apply_steps, crate::arena_hub_live::leave_arena).chain(),
            );
        let mut career = crate::hub_windows::ArenaCareer::default();
        career
            .company
            .hire("Doc", crate::career::Profession::Physician)
            .unwrap();
        career.company.roster[0].health = 500;
        app.insert_resource(career);
        let mut flow = Flow {
            mode: Mode::Arena,
            phase: Phase::Playing,
            ..Default::default()
        };
        let steps = flow.arena_exit();
        assert_eq!(steps.first(), Some(&Step::ArenaExit));
        assert!(steps.contains(&Step::Transition));
        assert!(flow.transition.is_some());
        app.insert_resource(flow);
        app.world_mut().resource_mut::<FlowSteps>().steps = steps;
        app.update();
        let career = app.world().resource::<crate::hub_windows::ArenaCareer>();
        assert!(career.company.roster.is_empty());
        assert_eq!(career.staff_left, vec![0]);
        let saves: Vec<crate::save_live::SaveStep> = app
            .world_mut()
            .resource_mut::<Messages<crate::save_live::SaveStep>>()
            .drain()
            .collect();
        assert!(saves.contains(&crate::save_live::SaveStep::Save {
            drop_on_low_health: false
        }));
        let hub = app.world().resource::<ArenaHub>();
        assert!(hub.busy);
        assert_eq!(hub.click_hook, HubHook::None);
    }

    #[test]
    fn leaving_an_arena_session_runs_the_arenas_leave() {
        use crate::arena_hub::HubHook;
        use crate::arena_hub_live::ArenaHub;
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::player_controls::PlaySession>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .insert_resource(ArenaHub {
                entered: true,
                busy: false,
                click_hook: HubHook::Click,
                ..Default::default()
            })
            .add_systems(
                Update,
                (apply_steps, crate::arena_hub_live::leave_arena).chain(),
            );
        let mut flow = Flow {
            mode: Mode::Arena,
            ..Default::default()
        };
        let steps = flow.leave_session();
        app.insert_resource(flow);
        app.world_mut().resource_mut::<FlowSteps>().steps = steps;
        app.update();
        let hub = app.world().resource::<ArenaHub>();
        assert!(hub.busy);
        assert_eq!(hub.click_hook, HubHook::None);
        // Another mode's leave does not touch the hub.
        let mut app2 = app;
        app2.insert_resource(ArenaHub {
            entered: true,
            click_hook: HubHook::Click,
            ..Default::default()
        });
        let mut flow = Flow {
            mode: Mode::Practice,
            ..Default::default()
        };
        let steps = flow.leave_session();
        app2.insert_resource(flow);
        app2.world_mut().resource_mut::<FlowSteps>().steps = steps;
        app2.update();
        let hub = app2.world().resource::<ArenaHub>();
        assert!(!hub.busy);
        assert_eq!(hub.click_hook, HubHook::Click);
    }

    /// The next fight's behaviour reset drops the stored target and the
    /// contact list while combat mode stays on.
    #[test]
    fn the_next_fight_drops_the_stored_target() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::player_controls::PlaySession>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .add_systems(Update, apply_steps);
        let foe = app.world_mut().spawn_empty().id();
        let mut body = crate::player::PlayerBody::new(0.0).with_target(foe);
        body.set_combat(true);
        let mut practice = crate::player_practice::PlayerPractice::default();
        practice.contacts.push(crate::player_behaviour::Contact {
            body: foe.to_bits(),
            score: 1.0,
            closeness: 1.0,
            facing: 1.0,
        });
        let player = app.world_mut().spawn((body, practice)).id();
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::ForgetTarget);
        app.update();
        let world = app.world();
        let body = world.get::<crate::player::PlayerBody>(player).unwrap();
        assert!(body.combat);
        assert_eq!(body.foe(), None);
        assert!(
            world
                .get::<crate::player_practice::PlayerPractice>(player)
                .unwrap()
                .contacts
                .is_empty()
        );
    }

    /// The practice start and the next fight set the staging freeze as they
    /// finish their staging, before the session hook; the step posts it for
    /// the next fixed step, where it sets the simulation's freeze.
    #[test]
    fn the_practice_staging_sets_the_staging_freeze() {
        let mut flow = Flow::default();
        let viewport = Viewport {
            width: 1920,
            height: 1080,
        };
        let start = flow.start_practice(Ladder::Novice, 90.0, viewport, 0);
        let freeze = start.iter().position(|s| *s == Step::FreezeAi).unwrap();
        assert_eq!(start[freeze + 1], Step::Hook(Hook::Practice));
        let next = flow.practice_next(9000);
        let freeze = next.iter().position(|s| *s == Step::FreezeAi).unwrap();
        assert_eq!(next[freeze + 1], Step::Hook(Hook::Practice));

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::player_controls::PlaySession>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .init_resource::<exa_ai::freeze::StagingFreeze>()
            .add_systems(Update, apply_steps);
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::FreezeAi);
        app.update();
        // Posted, not yet written: the next fixed step takes it.
        assert!(
            !app.world()
                .resource::<exa_ai::freeze::StagingFreeze>()
                .frozen()
        );
        exa_physics::sim_app::run_fixed_commands(app.world_mut());
        assert!(
            app.world()
                .resource::<exa_ai::freeze::StagingFreeze>()
                .frozen()
        );
    }

    /// The death and the knock-out start a running heartbeat's fade-out
    /// first, and the death fade's reset starts it last; a heartbeat that
    /// is not running is left alone.
    #[test]
    fn deaths_knock_outs_and_the_fade_reset_fade_the_heartbeat() {
        let mut flow = Flow::default();
        assert_eq!(flow.die(0.0)[0], Step::FadeHeartbeat);
        assert_eq!(flow.knock_out(0)[0], Step::FadeHeartbeat);
        let run = |step: Step, running: bool| {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .init_resource::<Flow>()
                .init_resource::<FlowSteps>()
                .init_resource::<GameView>()
                .init_resource::<crate::player_controls::PlaySession>()
                .init_resource::<exa_ui::DeathFade>()
                .insert_resource(exa_ui::Heartbeat {
                    started: running.then_some(0.0),
                    fading: None,
                })
                .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
                .add_message::<exa_world::level_lifecycle::WorldReset>()
                .add_message::<ForfeitMatch>()
                .add_message::<SettingsEntry>()
                .add_message::<AppExit>()
                .add_message::<crate::save_live::SaveStep>()
                .add_message::<crate::experience_live::DiscoveryTick>()
                .add_message::<crate::arena_hub_live::ArenaLeave>()
                .add_message::<crate::scene_bodies::SceneCommand>()
                .init_resource::<exa_physics::sim_app::FixedCommands>()
                .add_systems(Update, apply_steps);
            app.world_mut().resource_mut::<FlowSteps>().steps.push(step);
            app.update();
            app.world().resource::<exa_ui::Heartbeat>().fading
        };
        assert!(run(Step::FadeHeartbeat, true).is_some());
        assert!(run(Step::ResetDeathFade, true).is_some());
        assert_eq!(run(Step::FadeHeartbeat, false), None);
        assert_eq!(run(Step::ResetDeathFade, false), None);
    }

    /// A script asks for a motion. It does not start the clip: a busy or
    /// fallen body still gets to keep or drop the request.
    #[test]
    fn a_script_asks_for_a_motion_and_does_not_start_it() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::player_controls::PlaySession>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .insert_resource(exa_anim::Motions::fallback())
            .add_systems(Update, apply_steps);
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                exa_anim::Animator::new("Default", "Idle"),
            ))
            .id();
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::RequestAction(0x1f));
        app.update();
        let anim = app.world().get::<exa_anim::Animator>(player).unwrap();
        assert_eq!(anim.action, "Idle");
        assert_eq!(anim.requested, 0x1f);
        assert!(!anim.snap_start);
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::SnapAction(0x13));
        app.update();
        let anim = app.world().get::<exa_anim::Animator>(player).unwrap();
        assert_eq!(anim.action, "Idle");
        assert_eq!(anim.requested, 0x13);
        assert!(anim.snap_start);
    }

    /// Entering combat mode clears the backwards, heading-driven and
    /// turn-about bits. Turning it on again leaves whatever was set since.
    #[test]
    fn entering_combat_clears_the_walk_bits() {
        use exa_physics::body::locomotion::flags::{BACK, KEYS, SPRINT, TURN_ABOUT};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::player_controls::PlaySession>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .add_systems(Update, apply_steps);
        let req = exa_physics::body::locomotion::Locomotion {
            flags: BACK | KEYS | TURN_ABOUT | SPRINT,
            ..Default::default()
        };
        let player = app
            .world_mut()
            .spawn((crate::player::PlayerBody::new(0.0), req))
            .id();
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::CombatMode(true));
        app.update();
        let flags = |app: &App| {
            app.world()
                .get::<exa_physics::body::locomotion::Locomotion>(player)
                .unwrap()
                .flags
        };
        assert_eq!(flags(&app), SPRINT);
        app.world_mut()
            .get_mut::<exa_physics::body::locomotion::Locomotion>(player)
            .unwrap()
            .flags = BACK | KEYS | TURN_ABOUT | SPRINT;
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::CombatMode(true));
        app.update();
        assert_eq!(flags(&app), BACK | KEYS | TURN_ABOUT | SPRINT);
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
        // The revival closes the prelude, runs the world pass (the alarm
        // return, then the brains' reset), then saves the story.
        assert_eq!(steps.first(), Some(&Step::ClosePrelude));
        assert_eq!(steps.get(1), Some(&Step::AlarmReturn));
        assert_eq!(steps.get(2), Some(&Step::ReviveBrains));
        assert_eq!(
            steps.get(3),
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
        // The discovery pass runs every frame; the second gate only moves.
        let steps = flow.story_tick(100, Some(alive));
        assert!(steps.contains(&Step::Discovery));
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
        assert!(!steps.iter().any(|step| matches!(step, Step::GameGui(_))));
        assert!(steps.contains(&Step::Wake(WakePose::revived())));
        assert!(steps.contains(&Step::Hook(Hook::Story)));
        // Combat mode off, the body raised, then its held wake, then the
        // controls (the combat-mode setter's 0 is not the controls').
        let at = |wanted: Step| steps.iter().position(|step| *step == wanted).unwrap();
        assert!(at(Step::CombatMode(false)) < at(Step::ReviveBody));
        assert!(at(Step::ReviveBody) < at(Step::Wake(WakePose::revived())));
        assert!(at(Step::Wake(WakePose::revived())) < at(Step::Controls(true)));
        // The alarm return runs after the prelude closes and before the
        // save and the body's revive.
        assert!(at(Step::ClosePrelude) < at(Step::AlarmReturn));
        assert!(
            at(Step::AlarmReturn)
                < at(Step::Save {
                    drop_on_low_health: false
                })
        );
        assert!(at(Step::AlarmReturn) < at(Step::ReviveBody));
        assert!(at(Step::AlarmReturn) < at(Step::ReviveBrains));
        assert!(at(Step::ReviveBrains) < at(Step::ReviveBody));
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
                name: None,
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
                name: None,
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
            name: None,
        };
        assert!(flow.level_exit(none, rule, 0, false).is_empty());
        let steps = flow.level_exit(
            LevelExit {
                level: 3,
                spawn: 4,
                flags: 1,
                name: None,
            },
            rule,
            0,
            false,
        );
        assert_eq!(
            steps,
            vec![
                Step::Controls(false),
                Step::EndGuiDrag,
                Step::SweepScreens(crate::gui_screens::ScreenSweep::EveryAndBar),
                Step::Save {
                    drop_on_low_health: false
                },
                Step::Checkpoint,
                Step::PortalUse,
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
            name: None,
        };
        assert!(flow.level_exit(next, rule, 0, false).is_empty());
        // The level loads at the exit's spawn point when the screen is black.
        let steps = run_to_black(&mut flow);
        assert_eq!(
            steps,
            vec![
                Step::HideEndMenu,
                Step::ClosePrelude,
                Step::LoadLevel {
                    locale: "exanima03",
                    spawn: 4,
                    teardown: true,
                }
            ]
        );
        assert_eq!(flow.phase, Phase::Playing);
        // A dead player keeps no controls to lose.
        let steps = flow.level_exit(next, held, DEAD, false);
        assert_eq!(steps[0], Step::EndGuiDrag);
        assert_eq!(
            steps[1],
            Step::SweepScreens(crate::gui_screens::ScreenSweep::EveryAndBar)
        );
        assert!(!steps.contains(&Step::Checkpoint));
        // Without the portal bit nothing dissolves or hands over.
        assert!(!steps.contains(&Step::PortalUse));
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
        // An unregistered word (the versus lobby in 0.9.5) only becomes the
        // mode word: the frame hook goes and no scripted hook runs.
        let steps = flow.switch_mode(Mode::Other(VERSUS_MODE), 1000);
        assert_eq!(steps, vec![Step::Hook(Hook::None)]);
        assert_eq!(flow.mode, Mode::Other(VERSUS_MODE));
        // A registered one runs its enter hook.
        flow.script_modes.register(0x1020, false);
        let steps = flow.switch_mode(Mode::Other(0x1020), 1000);
        assert_eq!(
            steps,
            vec![
                Step::Hook(Hook::None),
                Step::ScriptMode {
                    mode: 0x1020,
                    hook: ScriptHook::Enter
                }
            ]
        );
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
        // Outside the practice mode the end menu's practice branch does not
        // run: no tip, no settings word.
        let mut arena = Flow {
            mode: Mode::Arena,
            phase: Phase::Playing,
            practice: PracticeState {
                ladder: Ladder::Novice,
                fight: 8,
                start_ms: 0,
                next_tip: false,
            },
            ..Default::default()
        };
        let steps = arena.practice_tick(6000, None, &[DEAD]);
        assert!(!steps.contains(&Step::Tip(5)));
        assert!(!arena.expert_written);
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
        // The table's point (x 0, z 500), half turned, its vertical the
        // fresh pose's pelvis target plus 5 cm over the origin.
        assert!(steps.contains(&Step::PlacePlayer(Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(0.0, 0.0, 500.0),
        })));
        assert!(steps.contains(&Step::PlacedFrame(crate::limp::FrameHeight::Target(5.0))));
        assert_eq!(flow.practice.fight, 1);
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
        // The first wake-up is watched once the level is in (after the
        // load's GUI reset), before the motion.
        assert!(!flow.story.wake.animating);
        let load = steps
            .iter()
            .position(|step| matches!(step, Step::LoadLevel { .. }))
            .unwrap();
        assert!(load < at(Step::FirstWake));
        assert_eq!(at(Step::FirstWake) + 1, at(Step::RequestAction(0x1d)));
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
        // The player is nudged out of an exit right after its placement.
        let placed = steps
            .iter()
            .position(|step| matches!(step, Step::PlacePlayer(_)))
            .unwrap();
        assert_eq!(steps[placed + 1], Step::ExitNudge);
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
    fn a_registered_scripted_mode_begins_and_leaves_through_its_hooks() {
        // 0.9.5: nothing registered, so the versus lobby's begin and leave
        // run no scripted hook.
        let mut flow = Flow {
            mode: Mode::Other(VERSUS_MODE),
            ..Default::default()
        };
        let begun = flow.begin_session(Vec::new());
        assert_eq!(
            begun,
            vec![Step::SessionTask(true), Step::EndMenuUnselected(false)]
        );
        assert!(
            !flow
                .leave_session()
                .iter()
                .any(|step| matches!(step, Step::ScriptMode { .. }))
        );
        // Registered: the begin hook comes before the frame task starts, and
        // the leave hook comes after the dialogue and prelude close, before
        // the save closes.
        let mut flow = Flow {
            mode: Mode::Other(VERSUS_MODE),
            ..Default::default()
        };
        flow.script_modes.register(VERSUS_MODE, false);
        let begun = flow.begin_session(Vec::new());
        assert_eq!(
            begun,
            vec![
                Step::ScriptMode {
                    mode: VERSUS_MODE,
                    hook: ScriptHook::Begin
                },
                Step::SessionTask(true),
                Step::EndMenuUnselected(false)
            ]
        );
        let left = flow.leave_session();
        let hook = left
            .iter()
            .position(|step| {
                *step
                    == Step::ScriptMode {
                        mode: VERSUS_MODE,
                        hook: ScriptHook::Leave,
                    }
            })
            .unwrap();
        let prelude = left
            .iter()
            .position(|step| *step == Step::ClosePrelude)
            .unwrap();
        let save = left
            .iter()
            .position(|step| *step == Step::CloseSave)
            .unwrap();
        assert!(prelude < hook && hook < save);
        // A registered word without the scripted bit begins through its hook
        // but leaves without it: the leave tests the bit first.
        let mut flow = Flow {
            mode: Mode::Other(0x20),
            ..Default::default()
        };
        flow.script_modes.register(0x20, false);
        assert!(flow.begin_session(Vec::new()).contains(&Step::ScriptMode {
            mode: 0x20,
            hook: ScriptHook::Begin
        }));
        assert!(
            !flow
                .leave_session()
                .iter()
                .any(|step| matches!(step, Step::ScriptMode { .. }))
        );
        // The arena leaves through its own leave, named under its word.
        let mut flow = Flow {
            mode: Mode::Arena,
            ..Default::default()
        };
        assert!(flow.leave_session().contains(&Step::ScriptMode {
            mode: ARENA_MODE,
            hook: ScriptHook::Leave
        }));
    }

    #[test]
    fn the_class_cancel_and_the_splash_hand_over() {
        let mut flow = Flow {
            mode: Mode::Other(VERSUS_MODE),
            phase: Phase::Playing,
            ..Default::default()
        };
        // The class select's cancel returns to the title behind the
        // transition and runs no scripted hook, registered or not.
        flow.script_modes.register(VERSUS_MODE, true);
        let steps = flow.cancel_class_select(false);
        assert_eq!(steps, vec![Step::Transition]);
        let steps = run_to_black(&mut flow);
        assert!(!steps.iter().any(|step| matches!(
            step,
            Step::ScriptMode {
                hook: ScriptHook::Cancel,
                ..
            }
        )));
        assert!(steps.contains(&Step::ShowMenu(Page::Title)));
        assert_eq!(flow.mode, Mode::Menu);
        // The creator's cancel: behind the transition (the music left
        // playing), the registered mode's cancel hook and then its leave hook
        // (the title's session leave), then the title.
        let mut flow = Flow {
            mode: Mode::Other(VERSUS_MODE),
            phase: Phase::Menu,
            ..Default::default()
        };
        flow.script_modes.register(VERSUS_MODE, true);
        assert_eq!(flow.cancel_creator(), vec![Step::Transition]);
        assert!(!flow.transition.unwrap().flags.fade_audio);
        let steps = run_to_black(&mut flow);
        let cancel = steps.iter().position(|step| {
            *step
                == Step::ScriptMode {
                    mode: VERSUS_MODE,
                    hook: ScriptHook::Cancel,
                }
        });
        let leave = steps.iter().position(|step| {
            *step
                == Step::ScriptMode {
                    mode: VERSUS_MODE,
                    hook: ScriptHook::Leave,
                }
        });
        assert!(cancel.unwrap() < leave.unwrap());
        assert_eq!(flow.mode, Mode::Menu);
        // An entry without the cancel slot, or no entry (0.9.5), runs none.
        for registered in [false, true] {
            let mut flow = Flow {
                mode: Mode::Other(VERSUS_MODE),
                ..Default::default()
            };
            if registered {
                flow.script_modes.register(VERSUS_MODE, false);
            }
            flow.cancel_creator();
            let steps = run_to_black(&mut flow);
            assert!(!steps.iter().any(|step| matches!(
                step,
                Step::ScriptMode {
                    hook: ScriptHook::Cancel,
                    ..
                }
            )));
            assert_eq!(
                steps.iter().any(|step| matches!(
                    step,
                    Step::ScriptMode {
                        hook: ScriptHook::Leave,
                        ..
                    }
                )),
                registered
            );
            assert!(steps.contains(&Step::ShowMenu(Page::Title)));
        }
        // The cancel slot is looked up by word alone: a registered word
        // without the scripted bit runs it too, but its session leave does
        // not run the leave hook.
        let mut flow = Flow {
            mode: Mode::Other(0x20),
            ..Default::default()
        };
        flow.script_modes.register(0x20, true);
        flow.cancel_creator();
        let steps = run_to_black(&mut flow);
        assert!(steps.contains(&Step::ScriptMode {
            mode: 0x20,
            hook: ScriptHook::Cancel
        }));
        assert!(!steps.contains(&Step::ScriptMode {
            mode: 0x20,
            hook: ScriptHook::Leave
        }));
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
        // A portal the last load found does not decide the placement: the
        // new load names its own.
        flow.respawn_portal = Some(Placement {
            rotation: PRACTICE_ROTATION,
            position: Vec3::new(1.0, 2.0, 3.0),
        });
        let steps = flow.story_respawn(0);
        // Without a portal in the level: the identity at the origin, 15 cm
        // up.
        assert!(
            steps.contains(&Step::PlaceAtPortal(crate::level_change::PortalArrival {
                without: Some(Placement {
                    rotation: NO_ROTATION,
                    position: Vec3::new(0.0, 15.0, 0.0)
                }),
                settle: 0,
            }))
        );
        assert_eq!(
            flow.portal_placement(),
            Some(Placement {
                rotation: PRACTICE_ROTATION,
                position: Vec3::new(1.0, 17.0, 3.0)
            })
        );
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
        assert!(matches!(steps[load + 4], Step::PlaceAtPortal(_)));
        // The placed player settles for 16 ticks.
        assert_eq!(steps[load + 5], Step::SettlePlayer(16));
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
                Step::ResetDeathFade,
                Step::SweepScreens(crate::gui_screens::ScreenSweep::EveryAndBar),
                Step::Controls(false),
                Step::EndDialogues,
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
        // and runs the Mind sight's quit hook after it.
        assert!(steps.contains(&Step::ResetMindSight));
        assert!(steps.contains(&Step::Transition));
    }

    /// The death fade is reset where the original resets it: the session's
    /// leave, the story's end-menu exit (first) and its respawn (first).
    #[test]
    fn the_leaves_and_the_respawn_reset_the_death_fade() {
        let mut flow = Flow {
            mode: Mode::Story,
            ..Default::default()
        };
        assert!(flow.leave_session().contains(&Step::ResetDeathFade));
        assert_eq!(flow.story_exit()[0], Step::ResetDeathFade);
        assert_eq!(flow.story_respawn(0)[0], Step::ResetDeathFade);
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
        // A new story's begin makes its slot first.
        assert_eq!(
            steps,
            vec![
                Step::NewSlot,
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
        assert_eq!(flow.leave_game_layer(), vec![Step::TeardownWorld]);
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
        assert_eq!(
            steps,
            vec![
                Step::SweepScreens(crate::gui_screens::ScreenSweep::ContainerFamily),
                Step::CloseSave
            ]
        );
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

    /// The death ends the GUI's held left button: a drag the player held
    /// when the death came ends as a release (the next frame's input sees
    /// the button up).
    #[test]
    fn a_player_death_lets_go_of_the_held_button() {
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
                crate::controls_live::ControlsLivePlugin,
            ));
        app.insert_resource(BodyTicks(100));
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
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .press(MouseButton::Left);
        app.world_mut().spawn((
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
        ));
        app.update();
        assert!(
            app.world()
                .resource::<crate::end_menu_screen::EndMenuScreen>()
                .open,
            "the death ran"
        );
        app.update();
        let buttons = app.world().resource::<ButtonInput<MouseButton>>();
        assert!(!buttons.pressed(MouseButton::Left));
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

    /// A staging's callback (the arena's next round) raises the pause and
    /// fires its stage with the playing phase; the screen's second frame
    /// after it lowers the pause, and no other frame does. A callback that
    /// is not a staging raises nothing.
    #[test]
    fn a_staging_callback_raises_the_pause_for_two_frames() {
        let mut flow = Flow {
            mode: Mode::Arena,
            phase: Phase::Playing,
            ..Default::default()
        };
        flow.transition(Action::ArenaRound, false);
        assert_eq!(flow.phase, Phase::Transition);
        flow.transition_tick(screen_input(1000), 0);
        let black = flow.transition_tick(screen_input(1601), 0);
        assert!(black.raise_pause && !black.lower_pause);
        assert_eq!(black.steps, vec![Step::ArenaStage(ArenaStage::Round)]);
        assert_eq!(flow.phase, Phase::Playing);
        let lowers: Vec<bool> = [1610, 1620, 1630, 1640]
            .into_iter()
            .map(|clock| flow.transition_tick(screen_input(clock), 0).lower_pause)
            .collect();
        assert_eq!(lowers, [false, false, true, false]);

        let mut flow = Flow {
            mode: Mode::Story,
            phase: Phase::Playing,
            ..Default::default()
        };
        flow.transition(Action::Revive, false);
        flow.transition_tick(screen_input(1000), 0);
        assert!(!flow.transition_tick(screen_input(1601), 0).raise_pause);
        assert!(Action::ArenaMatchStart.stages() && Action::NextPracticeFight.stages());
    }

    /// The match start holds the screen black over its level change, as a
    /// level load does, and gives the phase back.
    #[test]
    fn the_match_start_holds_the_screen_over_its_load() {
        let mut flow = Flow {
            phase: Phase::Playing,
            ..Default::default()
        };
        flow.transition(Action::ArenaMatchStart, true);
        flow.transition_tick(screen_input(1000), 0);
        let black = flow.transition_tick(screen_input(1601), 0);
        assert_eq!(black.steps, vec![Step::ArenaStage(ArenaStage::MatchStart)]);
        assert!(black.stop_music);
        assert_eq!(flow.phase, Phase::Playing);
        // Loading: the screen stays at black and its frames wait.
        let mut loading = screen_input(5000);
        loading.loading = true;
        for _ in 0..5 {
            let frame = flow.transition_tick(loading, 0);
            assert_eq!(frame.alpha, 255);
            assert!(!frame.lower_pause);
        }
        let lowers: Vec<bool> = [5010, 5020, 5030]
            .into_iter()
            .map(|clock| flow.transition_tick(screen_input(clock), 0).lower_pause)
            .collect();
        assert_eq!(lowers, [false, false, true]);
    }

    /// The return to the hub stages (its callback raises the pause, as the
    /// original's return routine does while the counter is zero), stops the
    /// music at black, ends with the playing phase and holds the screen
    /// black over the hub's load.
    #[test]
    fn the_return_to_the_hub_holds_the_screen_over_its_load() {
        let mut flow = Flow {
            mode: Mode::Arena,
            phase: Phase::Playing,
            ..Default::default()
        };
        flow.transition(Action::ArenaReturn, true);
        assert!(Action::ArenaReturn.stages());
        flow.transition_tick(screen_input(1000), 0);
        let black = flow.transition_tick(screen_input(1601), 0);
        assert_eq!(black.steps, vec![Step::ArenaStage(ArenaStage::Return)]);
        assert!(black.raise_pause && black.stop_music);
        assert_eq!(flow.phase, Phase::Playing);
        let mut loading = screen_input(5000);
        loading.loading = true;
        for _ in 0..5 {
            let frame = flow.transition_tick(loading, 0);
            assert_eq!(frame.alpha, 255);
            assert!(!frame.lower_pause);
        }
        let lowers: Vec<bool> = [5010, 5020, 5030]
            .into_iter()
            .map(|clock| flow.transition_tick(screen_input(clock), 0).lower_pause)
            .collect();
        assert_eq!(lowers, [false, false, true]);
    }

    /// The level exit's portal bit gives the screen its long hold: a 2000 ms
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
            name: None,
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

    /// The unknown background's start: the record takes the letter (item
    /// 0x10c at the cell (0x20, 0x80)) before the load; what follows the
    /// load waits for it, then the spawned player takes the spawn word
    /// 0x200, the roles 7, 1 and 0x0e, the letter in its pack and the
    /// start's knock-out, and the torch is laid at the start's pose.
    #[test]
    fn the_story_start_reaches_the_record_and_the_spawned_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<AfterLoad>()
            .init_resource::<crate::prelude::Tips>()
            .insert_resource(crate::creator_screen::CreatedCharacter(Some(
                exa_formats::characters::CharacterRecord::reset(),
            )))
            .init_resource::<exa_ai::brain::RoleDatabase>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .add_systems(Update, (release_after_load, apply_steps).chain());
        let view = Viewport {
            width: 1600,
            height: 1080,
        };
        let steps =
            app.world_mut()
                .resource_mut::<Flow>()
                .start_story(Background::Unknown, 1.0, view, 0);
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .extend(steps);
        app.update();
        let record = app
            .world()
            .resource::<crate::creator_screen::CreatedCharacter>()
            .0
            .clone()
            .unwrap();
        assert_eq!(
            record.inventory.last(),
            Some(&Carried {
                item: 0x10c,
                flags: 0,
                extra: 0x0080_0020,
            })
        );
        assert_eq!(record.hands[1], ItemRef::default());
        let start = *app.world().resource::<StoryStartPlayer>();
        assert_eq!(start.roles, &[7, 1, 0x0e]);
        assert_eq!(start.flags, 0x200);
        // The placement crosses the load; the rest waits for it.
        assert!(
            app.world()
                .contains_resource::<crate::level_change::PlayerPlacement>()
        );
        let held = &app.world().resource::<AfterLoad>().steps;
        assert!(held.contains(&Step::StartRecord([START_MOTION_ANIMATION, 0, 0])));
        assert!(held.contains(&Step::KnockOut { delay_ms: 3000 }));
        assert!(held.iter().any(|step| matches!(step, Step::StoryLight(_))));
        assert!(!held.iter().any(|step| matches!(step, Step::PlacePlayer(_))));
        // The level loads; the viewer then spawns the player and places it.
        app.insert_resource(exa_ui::LoadingStatus::default());
        app.update();
        assert!(!app.world().resource::<AfterLoad>().steps.is_empty());
        app.world_mut().remove_resource::<exa_ui::LoadingStatus>();
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                exa_combat::Condition(0),
                exa_items::possession::Possessions::default(),
                exa_ai::roles::Roles::default(),
            ))
            .id();
        app.update();
        assert!(
            !app.world().resource::<AfterLoad>().steps.is_empty(),
            "the placement still waits"
        );
        app.world_mut()
            .remove_resource::<crate::level_change::PlayerPlacement>();
        for _ in 0..3 {
            app.update();
        }
        assert!(app.world().resource::<AfterLoad>().steps.is_empty());
        let world = app.world();
        assert_eq!(world.get::<exa_combat::Condition>(player).unwrap().0, 0x200);
        let roles: Vec<u32> = world
            .get::<exa_ai::roles::Roles>(player)
            .unwrap()
            .ids()
            .collect();
        assert!(roles.contains(&7) && roles.contains(&1) && roles.contains(&0x0e));
        let pack = world
            .get::<exa_items::possession::Possessions>(player)
            .unwrap();
        assert_eq!(
            pack.carried.last(),
            Some(&exa_items::possession::Carried {
                id: 0x10c,
                flags: 0,
                x: 0x20,
                y: 0x80,
            })
        );
        assert!(
            world
                .get::<exa_combat::condition::KnockOut>(player)
                .is_some()
        );
        let torch = world.resource::<crate::torch::StoryTorch>().0.unwrap();
        assert!((torch.translation - Vec3::new(80.45, 0.047, 40.75)).length() < 1e-5);
        assert!(!world.contains_resource::<StoryStartPlayer>());
    }

    /// The unknown background's start writes the power word and the
    /// potential beside the slots, not the word into the potential.
    #[test]
    fn the_unknown_start_writes_the_power_word_and_potential() {
        let mut record = exa_formats::characters::CharacterRecord::reset();
        crate::create::apply_creation_defaults(&mut record);
        record.powers.as_mut().unwrap().word = 9.0f32.to_bits();
        record.extra_word = 0;
        let start = story_start(Background::Unknown, 1.0);
        story_record(&mut record, start.off_hand, start.carried, start.powers);
        assert_eq!(record.extra_word, 0x0051_0000);
        assert_eq!(record.powers.as_ref().unwrap().word, 4.0f32.to_bits());
    }

    /// The knight's start: the torch in the off hand and one carried with
    /// flags 0x100; no torch is laid.
    #[test]
    fn the_knight_s_record_holds_the_torch() {
        let mut record = exa_formats::characters::CharacterRecord::reset();
        let start = story_start(Background::Knight, 1.0);
        story_record(&mut record, start.off_hand, start.carried, start.powers);
        assert_eq!(record.hands[1], ItemRef { id: 0x1f, flags: 0 });
        assert_eq!(
            record.inventory.last().map(|c| (c.item, c.flags)),
            Some((0x1f, 0x100))
        );
        assert!(start.light.is_none());
        let mut pack = exa_items::possession::Possessions::default();
        story_possessions(&mut pack, start.off_hand, start.carried);
        assert_eq!(pack.out[1].id, 0x1f);
        // The new entry's cell is the default (40, 4).
        assert_eq!((pack.carried[0].x, pack.carried[0].y), (40, 4));
        let roles = story_roles(&exa_formats::roles::RoleDb::default(), start.roles);
        // The list keeps the newest role first.
        assert_eq!(roles.ids().collect::<Vec<_>>(), vec![0x1b, 1, 7]);
    }

    /// The tips: the knock-out's hint 2, the prelude's close, and the
    /// level-6 checkpoint's clock stamp.
    #[test]
    fn the_tip_and_level_time_steps_are_applied() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::prelude::Tips>()
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .add_systems(Update, apply_steps);
        let steps = app.world_mut().resource_mut::<Flow>().knock_out(0);
        assert!(steps.contains(&Step::Tip(2)));
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .extend(steps);
        app.update();
        let tips = app.world().resource::<crate::prelude::Tips>();
        assert_eq!(tips.current, Some(crate::prelude::Shown::Hint(1)));
        assert_eq!(
            tips.current.and_then(crate::prelude::Shown::text),
            Some("You have been defeated.")
        );
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .extend([Step::ClosePrelude, Step::LevelTime]);
        app.update();
        assert_eq!(app.world().resource::<crate::prelude::Tips>().current, None);
        assert!(
            app.world()
                .contains_resource::<crate::save_live::LevelTime>()
        );
    }

    /// Held steps wait for the loading screen, then two quiet frames, a
    /// player and no pending placement; a load that never shows runs them
    /// after the wait.
    #[test]
    fn steps_held_for_a_load_wait_for_it() {
        let mut after = AfterLoad::default();
        assert!(!after.ready(false, true, false), "nothing held");
        after.defer(vec![Step::Controls(true)]);
        assert!(!after.ready(true, false, false));
        assert!(!after.ready(false, true, false), "one quiet frame");
        assert!(!after.ready(false, true, true), "the placement waits");
        assert!(after.ready(false, true, false));
        let mut lost = AfterLoad::default();
        lost.defer(vec![Step::Controls(true)]);
        for _ in 1..AFTER_LOAD_WAIT_FRAMES {
            assert!(!lost.ready(false, true, false));
        }
        assert!(lost.ready(false, true, false));
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
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
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
            Step::PlaceAtPortal(crate::level_change::PortalArrival {
                without: None,
                settle: 16,
            }),
            Step::SettlePlayer(16),
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
        // The settling goes across the load with the placement.
        assert_eq!(
            app.world()
                .get_resource::<crate::level_change::PlacedSettle>()
                .copied(),
            Some(crate::level_change::PlacedSettle(16))
        );
        // So does the portal placement, which the host resolves once the
        // level is in.
        assert_eq!(
            app.world()
                .get_resource::<crate::level_change::PortalArrival>()
                .copied(),
            Some(crate::level_change::PortalArrival {
                without: None,
                settle: 16,
            })
        );

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
    /// app applies the original's record strip: the dead body keeps its gear
    /// and its health (it stays a dressed corpse) under a dressed copy of its
    /// record, which joins the world store, and the flow keeps the stripped
    /// record the player goes on with.
    #[test]
    fn the_app_strips_a_reconstructed_player() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .init_resource::<GameView>()
            .init_resource::<crate::left_behind::LeftBody>()
            .insert_resource(crate::world_characters::WorldCharacters::new_story())
            .add_message::<exa_world::level_lifecycle::LevelChangeRequest>()
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
            .add_systems(Update, apply_steps);
        let mut fighter = exa_combat::Fighter::default();
        fighter.health.max = 100.0;
        fighter.health.red = 0.0;
        let mut record = exa_formats::characters::CharacterRecord::reset();
        record.hands[0] = ItemRef::new(0x31, 0);
        record.worn.push(ItemRef::new(0x32, 0));
        let possessions = exa_items::possession::Possessions::of_record(&record);
        let player = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                fighter,
                exa_character::Outfit::default(),
                exa_character::Wielded::model("ub05ssworda"),
                exa_character::record::CharacterData::of_record(record),
                possessions,
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
        // The body is left as it fell: armed, dressed, dead.
        assert!(world.get::<exa_character::Wielded>(player).is_some());
        let health = world.get::<exa_combat::Fighter>(player).unwrap().health;
        assert_eq!(health.red, 0.0);
        let copy = &world
            .get::<exa_character::record::CharacterData>(player)
            .unwrap()
            .record;
        assert_eq!(copy.id, exa_formats::characters::WORLD_DB | 2);
        assert_eq!(copy.hands[0].id, 0x31);
        assert!(copy.worn.iter().any(|w| w.id == 0x32));
        let left = world.resource::<crate::left_behind::LeftBody>();
        assert_eq!(left.body, Some(player));
        assert!(!left.exported);
        let stripped = left.player.as_ref().unwrap();
        assert!(!stripped.hands[0].is_some());
        assert_eq!(
            stripped.worn,
            [ItemRef::new(exa_formats::characters::UNDERWEAR_ITEM, 0)]
        );
        assert_eq!(
            (stripped.pools.yellow, stripped.pools.red),
            (0x7FFF, 0x7FFF)
        );
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
            .add_message::<exa_world::level_lifecycle::WorldReset>()
            .add_message::<ForfeitMatch>()
            .add_message::<SettingsEntry>()
            .add_message::<AppExit>()
            .add_message::<crate::save_live::SaveStep>()
            .add_message::<crate::experience_live::DiscoveryTick>()
            .add_message::<crate::arena_hub_live::ArenaLeave>()
            .add_message::<crate::scene_bodies::SceneCommand>()
            .init_resource::<exa_physics::sim_app::FixedCommands>()
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

    /// The background-unlock event sets bit 4 in the background word and
    /// writes the whole word once; a second event writes nothing. Other
    /// codes write nothing.
    #[test]
    fn the_game_event_hook_writes_the_background_unlock_once() {
        let mut flow = Flow::default();
        flow.unlocks.backgrounds = 1;
        let story = crate::game_event::StoryContext {
            unknown_background: false,
        };
        let player = crate::game_event::EventPlayer {
            memory_role: false,
            raise_dead: false,
            class: 0,
            female: false,
            potential: 0.0,
            collected: 0,
            voice_volume: 1.0,
            voice_pitch: 1.0,
        };
        let (steps, rest) =
            flow.game_event(crate::game_event::UNLOCK_EVENT, story, &player, &mut || 0.0);
        assert_eq!(
            steps,
            vec![Step::SettingsWrite {
                key: BACKGROUND_UNLOCK_KEY,
                bytes: [5, 0, 0, 0],
            }]
        );
        assert!(rest.is_empty());
        assert_eq!(flow.unlocks.backgrounds, 5);
        let (again, _) =
            flow.game_event(crate::game_event::UNLOCK_EVENT, story, &player, &mut || 0.0);
        assert!(again.is_empty());
        let (other, _) = flow.game_event(0xda_0115, story, &player, &mut || 0.0);
        assert!(other.is_empty());
    }

    /// The flow keeps the fragments seen: a fragment's bits are or-ed in,
    /// an awakening sets the word whole, and a new story clears it and
    /// keeps its background.
    #[test]
    fn the_flow_keeps_the_fragments_seen_and_the_background() {
        let mut flow = Flow::default();
        let story = crate::game_event::StoryContext {
            unknown_background: true,
        };
        let player = crate::game_event::EventPlayer {
            memory_role: true,
            raise_dead: false,
            class: 0,
            female: false,
            potential: 0.0,
            collected: 0,
            voice_volume: 1.0,
            voice_pitch: 1.0,
        };
        let (_, rest) = flow.game_event(0xc0_0002, story, &player, &mut || 0.0);
        assert_eq!(flow.story.memory, 2);
        assert!(
            !rest
                .iter()
                .any(|e| matches!(e, crate::game_event::Effect::Collected(_)))
        );
        flow.game_event(
            crate::game_event::AWAKENING_MIND,
            story,
            &player,
            &mut || 0.0,
        );
        assert_eq!(flow.story.memory, 0xffff);
        let view = Viewport {
            width: 1280,
            height: 720,
        };
        flow.start_story(Background::Villager, 1.8, view, 0);
        assert_eq!(flow.story.memory, 0);
        assert_eq!(flow.story.background, Background::Villager);
    }

    /// A script's `GameEvent(4)` reaches the flow's consumer, whose write
    /// becomes the settings entry the host's store saves.
    #[test]
    fn a_script_game_event_reaches_the_settings_entry() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Flow>()
            .init_resource::<FlowSteps>()
            .add_message::<exa_script::ScriptEffect>()
            .add_message::<exa_script::Narrate>()
            .add_message::<exa_combat::pain::PainSounded>()
            .add_message::<exa_audio::PointSounded>()
            .add_systems(Update, crate::game_event_live::run_game_events);
        app.world_mut().write_message(exa_script::ScriptEffect(
            exa_script::dialogue::Effect::GameEvent(crate::game_event::UNLOCK_EVENT),
        ));
        app.update();
        let steps = &app.world().resource::<FlowSteps>().steps;
        assert_eq!(
            steps,
            &vec![Step::SettingsWrite {
                key: BACKGROUND_UNLOCK_KEY,
                bytes: [4, 0, 0, 0],
            }]
        );
        assert_eq!(app.world().resource::<Flow>().unlocks.backgrounds, 4);
    }

    /// The game flow's app with the real plugins: the menu, the flow, the
    /// end menu, the GUI reset and the script engine (its role library
    /// given, as the installation's would be).
    fn session_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .insert_resource(BootToMenu(false))
            .insert_resource(exa_script::Scripts::default())
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                FlowPlugin,
                crate::end_menu_screen::EndMenuScreenPlugin,
                crate::load_reset::LoadResetPlugin,
                exa_script::ScriptPlugin,
            ));
        app.update();
        app
    }

    /// Leaving a session ends every conversation: the reply panel the
    /// player had open closes and a pending narration is dropped, as the
    /// story's end-menu exit does too.
    #[test]
    fn leaving_the_session_ends_every_conversation() {
        for leave in [
            Flow::leave_session as fn(&mut Flow) -> Vec<Step>,
            Flow::story_exit,
        ] {
            let mut app = session_app();
            {
                let mut talk = app.world_mut().resource_mut::<exa_script::Conversations>();
                talk.engine.panel.open = true;
                talk.engine.panel.partner = Some(exa_script::bubbles::Speaker::Narrator);
            }
            let steps = {
                let mut flow = app.world_mut().resource_mut::<Flow>();
                flow.mode = Mode::Story;
                flow.phase = Phase::Playing;
                leave(&mut flow)
            };
            assert!(steps.contains(&Step::EndDialogues));
            assert!(!steps.contains(&Step::Dialogue));
            app.world_mut().resource_mut::<FlowSteps>().steps = steps;
            app.update();
            app.update();
            let talk = app.world().resource::<exa_script::Conversations>();
            assert!(!talk.engine.panel.open);
            assert!(talk.engine.narration.is_none());
        }
    }

    /// The new story's first wake-up is watched once its level is in: the
    /// load's GUI reset clears the flag (as every load's does) before the
    /// start's steps set it.
    #[test]
    fn the_first_wake_is_set_after_the_loads_reset() {
        let mut app = session_app();
        app.world_mut().resource_mut::<Flow>().story.wake.animating = true;
        let steps = {
            let mut flow = app.world_mut().resource_mut::<Flow>();
            let viewport = flow.viewport;
            flow.start_story(Background::Unknown, 1.0, viewport, 0)
        };
        app.world_mut().resource_mut::<FlowSteps>().steps = steps;
        app.update();
        app.update();
        // The load was asked for and its reset ran; the start's later steps
        // wait for the level.
        assert!(!app.world().resource::<Flow>().story.wake.animating);
        assert!(
            app.world()
                .resource::<AfterLoad>()
                .steps
                .contains(&Step::FirstWake)
        );
        for _ in 0..AFTER_LOAD_WAIT_FRAMES + 2 {
            app.update();
        }
        assert!(app.world().resource::<AfterLoad>().steps.is_empty());
        assert!(app.world().resource::<Flow>().story.wake.animating);
        // A later load (a door) clears it again, as the original's does.
        app.world_mut()
            .write_message(exa_world::level_lifecycle::LevelChangeRequest::change(
                "exanima02",
                0,
            ));
        app.update();
        app.update();
        assert!(!app.world().resource::<Flow>().story.wake.animating);
    }

    /// The practice start gives the player its roles anew: the list cleared,
    /// then the practice role.
    #[test]
    fn the_practice_start_gives_the_player_its_role() {
        let mut app = session_app();
        app.insert_resource(exa_ai::brain::RoleDatabase::default());
        let mut held = exa_ai::roles::Roles::default();
        held.add(
            &exa_formats::roles::RoleDb::default(),
            exa_ai::roles::PLAYER,
        );
        let player = app
            .world_mut()
            .spawn((crate::player::PlayerBody::new(0.0), held))
            .id();
        app.world_mut().resource_mut::<FlowSteps>().steps = vec![Step::PracticePlayer {
            role: PRACTICE_ROLE,
        }];
        app.update();
        let roles = app.world().get::<exa_ai::roles::Roles>(player).unwrap();
        assert_eq!(roles.ids().collect::<Vec<_>>(), vec![PRACTICE_ROLE]);
    }

    /// The practice placements carry their frame height across the arena's
    /// load with the placement: the next fight's stand height over the
    /// origin.
    #[test]
    fn a_practice_placement_carries_its_frame_height() {
        let mut app = session_app();
        let steps = {
            let mut flow = app.world_mut().resource_mut::<Flow>();
            flow.mode = Mode::Practice;
            flow.practice.ladder = Ladder::Novice;
            flow.practice_next(0)
        };
        app.world_mut().resource_mut::<FlowSteps>().steps = steps;
        app.update();
        assert_eq!(
            app.world()
                .get_resource::<crate::level_change::PlacedFrame>()
                .copied(),
            Some(crate::level_change::PlacedFrame(
                crate::limp::FrameHeight::Stand
            ))
        );
        let at = app
            .world()
            .resource::<crate::level_change::PlayerPlacement>();
        assert_eq!(at.position, Vec3::new(0.0, 0.0, 5.0));
    }

    /// A camera shot warps the pointer to the point it framed while the
    /// window has the focus, and not without it.
    #[test]
    fn a_camera_shot_warps_the_focused_pointer() {
        let mut app = session_app();
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1600, 1140)
                        .with_scale_factor_override(2.0),
                    ..Default::default()
                },
                bevy::window::PrimaryWindow,
            ))
            .id();
        let view = Viewport {
            width: 1600,
            height: 1140,
        };
        let frame = camera::frame_point(PRACTICE_POINT, true, false, view);
        app.world_mut().resource_mut::<FlowSteps>().steps = vec![Step::Camera(frame)];
        app.update();
        let warps: Vec<crate::os_pointer::WarpPointer> = app
            .world_mut()
            .resource_mut::<Messages<crate::os_pointer::WarpPointer>>()
            .drain()
            .collect();
        assert_eq!(
            warps,
            vec![crate::os_pointer::WarpPointer(
                Vec2::new(frame.warp.0 as f32, frame.warp.1 as f32) / 2.0
            )]
        );
        app.world_mut().get_mut::<Window>(window).unwrap().focused = false;
        app.world_mut().resource_mut::<FlowSteps>().steps = vec![Step::Camera(frame)];
        app.update();
        assert!(
            app.world_mut()
                .resource_mut::<Messages<crate::os_pointer::WarpPointer>>()
                .drain()
                .next()
                .is_none()
        );
    }

    /// The in-game GUI goes on with each start once its level is in, stays
    /// on through a death (which sweeps the game screens) and goes off with
    /// the session's leave.
    #[test]
    fn the_in_game_gui_follows_the_session() {
        let mut app = session_app();
        app.insert_resource(GameView(false));
        let steps = {
            let mut flow = app.world_mut().resource_mut::<Flow>();
            let viewport = flow.viewport;
            flow.start_practice(Ladder::Novice, 0.0, viewport, 0)
        };
        assert!(!steps.contains(&Step::GameGui(false)));
        app.world_mut().resource_mut::<FlowSteps>().steps = steps;
        for _ in 0..AFTER_LOAD_WAIT_FRAMES + 3 {
            app.update();
        }
        assert!(app.world().resource::<GameView>().0);
        let steps = app.world_mut().resource_mut::<Flow>().die(0.0);
        assert!(steps.contains(&Step::SweepScreens(
            crate::gui_screens::ScreenSweep::EveryAndBar
        )));
        app.world_mut().resource_mut::<FlowSteps>().steps = steps;
        app.update();
        assert!(app.world().resource::<GameView>().0);
        let steps = app.world_mut().resource_mut::<Flow>().leave_session();
        app.world_mut().resource_mut::<FlowSteps>().steps = steps;
        app.update();
        assert!(!app.world().resource::<GameView>().0);
    }
}

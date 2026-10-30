//! Exarust launcher.
//!
//! ```text
//! exarust [--game <dir>] [options] level <name>          a level with its props and lights
//! exarust [--game <dir>] [options] model <pack> <entry>  one model
//! exarust [--game <dir>] [options] texture <name>        one texture (first light)
//! exarust [--game <dir>] [options] character [moveset/action]  the human body playing a motion
//! exarust [--game <dir>] [options] play <level>          walk the level as the player (WASD, Shift, right-drag)
//!
//! options: --camera x,y,z,yaw,pitch   start pose (metres, degrees)
//!          --at x,y,z                 play mode: start here (metres)
//!          --corpse x,y,z             play mode: a dead human lies here (metres); repeatable
//!          --torch x,y,z              a burning torch fire here (metres), e.g. to stand in for a held one
//!          --fov <degrees>            vertical field of view (default 45)
//!          --no-shadows               lights cast no shadows (for software rendering)
//!          --bevy-sun-cascades        the sun keeps Bevy's cascades instead of the original's fitted map
//!          --no-gi                    no voxel GI (ambient from the environment cube)
//!          --no-supersample           draw at window resolution (default: 4× supersampling, as the original)
//!          --splash                   the original's start-up splash (the BME logos, about five seconds), then the main menu
//!          --gi-warm-up <steps>       GI steps run when a level loads (default 600)
//!          --size WxH                 window size (default 1600x1080)
//!          --max-fps <n|auto>         hold the frame to n a second, or to the display's rate (off unless FPSLimit is set)
//!          --no-settings              ignore the user's settings files (see below)
//!          --benchmark                run the GPU benchmark and take its quality levels
//!          --screenshot <file.png>    save a frame and exit
//!          --frames <n>               frames to wait before the screenshot, after loading (default 90)
//!          --ev <ev100>               camera exposure (default 6, lower is brighter)
//!          --all-rooms                tile-grid levels: show every room (no black for rooms the player cannot see)
//!          --limp                     character view: the body goes limp and falls
//!          --msm                      model view: every mesh smoothed once, as the original's `msm` debug
//!                                     command and its character-creation hair are (Loop subdivision)
//!          --race <n>                 character view: the body of race n (a character record's race byte)
//!          --physique x,y[,h,a,f,w]   character view and the player: the body (and its apparel) morphed by this
//!                                     physique (two physique values, height, age, feral level, female if w ≠ 0)
//!          --shove <frame>            a body blow to every simulated character body at that frame after loading
//!          --shove-props <frame>      play mode: a blow to every prop body within 3 m of the player at that frame
//!                                     after loading, away from the player and up; logs how far each moved once
//!                                     they all sleep again
//!          --kinesis-pick <frame>     play mode: from that frame after loading (waiting for one), a running Kinesis of the player takes
//!                                     the nearest prop body within 5 m, as the hand's pick hands it one
//!          --outfit a,b,c             apparel for character/play (Characters.rpk entries), over the underwear
//!          --weapon <name>            weapon in the right hand (Objects.rpk entry)
//!          --opponent                 an armed duellist in play mode; practice and arena fights start in combat mode (Tab toggles it, left click attacks)
//!          --original-spawn           living bodies spawn in the original's order (key pick, pelvis on the frame,
//!                                     leg snap); their pelvis height is logged for 120 ticks at debug level
//!          --flat-floor               play mode: the player spawns with the state word's flat-floor bit 0x1000, as
//!                                     the original's lying (mode 2) spawn and its character viewers set it: the
//!                                     feet stand on a floor at 0 without probing and the thighs splay 5°
//!          --opponent-shield <m|any>  every duellist (the debug one and the arena's) carries this shield on its off
//!                                     arm (an Objects.rpk model, e.g. "Shield R01"; `any`: the first shield item whose
//!                                     model loads); a blade that meets it is blocked (logged "a block" at info)
//!          --shield <m|any>           play mode: the player carries this shield on its off arm (no torch then)
//!          --still-opponent           the duellist stands its ground: it faces the player in its stance but neither walks nor strikes
//!          --arena-match <id>         in an arena, lay the match out as the original's template <id> does (1 duel, 2 doubles,
//!                                     3 skirmish, 4 fray, 5 reserve, 6 pugilism, 7 elimination, 8 challenger, 9 captain,
//!                                     12 brawl, 13 valiance, 14 valour): the player takes the company's first place and a
//!                                     duellist stands on each opponent's place; the match tick ends its rounds
//!          --arena-level <n>          the arena match's rank, the generated opponents' level (default 0)
//!          --arena-tier <n>           the arena match's gear tier (default 0)
//!          --arena-weapons <ids>      the drawn weapon variant per opponent, comma separated
//!          --arena-auto               the original's dormant auto-match switch: a round's end continues by itself and,
//!                                     4 s after the return to the hub, the last board match starts again with the
//!                                     company's combatants (a debug tool for unattended arena runs)
//!          --trace <file.csv>         the player's body (else every simulated body) after every 83 Hz tick, in cm
//!          --trace-all                with --trace: every simulated character body, the player's among them
//!          --debug-bodies             start with the body overlay and joint readout shown (F3 toggles it)
//!          --debug-nav                draw the level's navigation grid around the view (F6 toggles it)
//!          --nav-refresh-moved        debug: a door or prop that comes to rest somewhere new refreshes the navigation grid
//!                                     under it (the original refreshes only where a lifted item is let go)
//!          --static-doors             debug: hinged doors stay static level geometry, as before they became bodies
//!          --static-mechanisms        debug: levers and push surfaces stay static level geometry, as before they became bodies
//!          --sliding-doors            debug: sliding doors (portcullises) become bodies too; their lift is not reproduced
//!          --debug-gui-form           open one empty GUI form through the draw skin's path
//!          --open-settings <page>     once loaded, open the menu and the Settings form on its settings or controls page
//!          --open-creator <mode>      once loaded, open the character creator: story (the background picker),
//!                                     story-all (the picker with every background unlocked), story:N
//!                                     (background N), practice, expert, arena, recruit or versus
//!          --open-container FRAME     open the container nearest the player in its popup that many frames after the load
//!          --debug-path x,y,z         play mode: plan a path from the player's feet to this point (metres) and draw it,
//!                                     again whenever the player has moved a metre
//!          --searcher <id>            play mode: the level character with this record id (hex with 0x) takes the
//!                                     Explorer role: it follows the player and searches the dead bodies near it
//!          --debug-column x,z[/x,z…]  once loaded, log every mesh over each floor point: name, heights, shadow level, how it casts
//!          --profiler                 frame profiler: main/render thread and GPU times, overlay shown (F10 toggles)
//!          --perf-log <seconds>       frame profiler on, its report logged every so often
//! keys:    F9                         level and play views: the map panel (view switches, switch to another level)
//!          --joint-trace <file.csv>   every joint of every simulated body after every 83 Hz tick (degrees, cm)
//!          --pair-trace <file.csv>    every contact between two body parts found in each 83 Hz tick (cm)
//!          --fixed-frames             every frame advances game time by exactly 1/60 s, so runs repeat tick for tick
//!          --sim-speed <f>            run the simulation at this speed (e.g. 0.25)
//!          --pause-at-tick <n>        pause the simulation at the end of body tick n
//!          --pause-on-stretch <cm>    pause the simulation when a link goes this far past its slack
//!          --pause-after-jet <n>      pause the simulation n body ticks after the first blood jet
//!          --pause-on-hit <n>         pause the simulation n ticks after the player's first blow lands
//!          --pause-after-collapse <n> pause the simulation n ticks after the first killing blow starts a collapse
//!          --press key@frame,...      press keys (Ctrl, Shift and Esc among them) or mouse buttons (LMB, RMB, MMB) at frames after
//!                                     loading (e.g. `.@400,F8@500,LMB@600`); `key@a-b` holds it from frame a to b
//!          --cursor x,y               play mode: the cursor stays at this window point (logical pixels), for unattended runs
//!          --click frame,...          click the left mouse button at frames after loading
//!          --object-click F[d][l],...  debug: click the placed container nearest the player at frames after
//!                                     loading (d: the double click, l: as a hit on level geometry)
//!          --save-dir <dir>           the folder the game saves into (a new slot only; a slot file that holds bytes is
//!                                     never written over) and loads `--arena-slot` from
//!          --save-at frame,...        with --save-dir: save the session to a new slot at frames after the load (the
//!                                     story at the loaded level, or the arena company when the hub is loaded)
//!          --arena-slot <n>           with --save-dir: the arena hub, once entered, loads ArenaNNN.rsg as the original's
//!                                     full load does (tables, shelf regeneration, world clock, player record reset)
//!          --level-change lvl@frame,...  play mode: change level in process at frames after the first load (loading
//!                                     frames do not count), logging entity, physics-body and asset counts per level
//!          --debug-wound b,r[,dx,dy,dz]  a test wound on the player's (or the character view's) body part b
//!                                     (1–18 or chest, luarm, …), radius r cm, smeared along d cm; repeatable
//!          --weapon-blood <a>         the player's (or the character view's) weapon holds blood a (0–1) from the start
//!          --debug-spray b,R[,dx,dy,dz]  a blood jet from the player's (or the character view's) body part b every
//!                                     second, as a bleeding blow of size R swinging along d would spray; repeatable
//!          --debug-arcs               play mode: the Shock power's arcs on the player's right hand
//!          --debug-barricade          play mode: a force barricade raised 2 m in front of the player
//!          --debug-wisps              play mode: the constant displacement's wisps on the player's body
//!          --powers <id,id,...>       play mode: the powers the player knows (hex or decimal), for the power bar
//!          --health yellow,red        play mode: the player's starting health (a human's full health is 0.25)
//!          --perks a,b,...            play mode: the player's perks (e.g. `remise,feint`: hold to chain the other
//!                                     side's attack, renew a released swing)
//!          --heartbeat                play mode: the low-health heartbeat in any level (the original: story mode only)
//!          --undying                  play mode: the player's health is refilled after every frame and body tick, so long
//!                                     scripted fights run on (a single blow from full health can still kill)
//!          --feral <f>                play mode: the player's record has feral level f (0–1): its blows' red damage
//!                                     goes to yellow, and it spawns with a feral character's stats
//! ```
//!
//! The user's settings files (`docs/formats/SETTINGS.md`) are read from
//! `EXARUST_USER_DIR`, else the platform's configuration folder and
//! `exarust`: the window size, full screen, vertical sync, supersampling,
//! the frame-rate readout and the GUI size they name apply unless the
//! command line says otherwise, and the store's key bindings are used.
//!
//! In tile-grid levels only the rooms the player can see are drawn, as in
//! the original: the player's room and those behind a door within 2 m; the
//! rooms behind their doors are covered by black lids and the rest is black.
//! In `level` mode with `--camera` the rooms follow the point 20 m in front
//! of the camera (the original camera's target); `level` without
//! `--camera` (the overview) and `--all-rooms` show every room.
//!
//! Controls: hold the right mouse button to look, WASD to move, Q/E down/up,
//! Shift to go faster. Debug keys (`docs/DEBUGGING.md`): F3 body overlay, F4
//! only the inspected body, F5 inspect the body under the cursor, F6 the
//! navigation grid, F7 start every keyframe track of the level, P pause,
//! `.` step one tick, `[` / `]` slower / faster.

mod column;
mod frame_loop;
mod map_panel;
mod message_box;
mod profiler;
mod user_settings;
mod viewer;

use std::ops::Range;
use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use exa_assets::{Game, GameData, locate_install};

#[derive(Resource, Clone, Debug)]
pub enum View {
    Level(String),
    Model { pack: String, entry: String },
    Texture(String),
    Character { motion: String },
    Play(String),
}

#[derive(Resource, Clone, Debug, Default)]
pub struct Options {
    pub camera: Option<[f32; 5]>,
    /// Vertical field of view, degrees.
    pub fov: Option<f32>,
    /// Lights cast no shadows.
    pub no_shadows: bool,
    /// The sun keeps Bevy's cascades in terrain levels too.
    pub bevy_sun_cascades: bool,
    /// The voxel GI settings.
    pub gi: exa_world::gi_volume::GiSettings,
    /// Draw at window resolution instead of 4× supersampling.
    pub no_supersample: bool,
    /// Hide the frame-rate readout.
    pub no_fps: bool,
    /// Show the original's start-up splash (the BME logos) over the start,
    /// then the main menu.
    pub splash: bool,
    /// Run the GPU benchmark after the start and take its quality levels.
    pub benchmark: bool,
    /// Present frames as soon as they are drawn (for measuring).
    pub no_vsync: bool,
    /// The original's frame-rate cap: positive is a fixed rate, negative
    /// automatic (the display's rate), zero off. `None` (off) unless the
    /// command line or the settings file's `FPSLimit` gives one.
    pub max_fps: Option<i32>,
    /// Window size in logical pixels.
    pub size: Option<(u32, u32)>,
    /// Full screen over the primary display (borderless; the settings'
    /// `ScreenWidth`/`ScreenHeight` can ask for another display mode).
    pub fullscreen: bool,
    /// Ignore the user's settings files.
    pub no_settings: bool,
    /// Play mode: start here instead of the level's start (metres).
    pub at: Option<[f32; 3]>,
    /// A torch fire at this point (metres).
    pub torch: Option<[f32; 3]>,
    /// Play mode: dead humans lying at these points (metres).
    pub corpses: Vec<[f32; 3]>,
    pub screenshot: Option<PathBuf>,
    pub frames: u32,
    /// Camera exposure; dungeon interiors need far less than daylight.
    pub ev100: f32,
    /// Show every room of a tile-grid level, as if all were active.
    pub all_rooms: bool,
    /// Hold "forward" in play mode (for unattended checks).
    pub autowalk: bool,
    /// Character view: let the body go limp and fall.
    pub limp: bool,
    /// Model view: smooth every mesh once, as the original's `msm` debug
    /// command smooths a scene.
    pub msm: bool,
    /// Character view: the body of this race (character race byte).
    pub race: Option<u8>,
    /// Character view and the player: the physique the body (and its
    /// apparel) is morphed by; none leaves it as its model stores it.
    pub physique: Option<exa_formats::morph::Physique>,
    /// Deal every simulated character body a blow at this frame.
    pub shove: Option<u32>,
    /// Deal every prop body near the player a blow at this frame.
    pub shove_props: Option<u32>,
    /// Hand the player's running Kinesis the nearest prop at this frame.
    pub kinesis_pick: Option<u32>,
    /// Apparel for the character (`Characters.rpk` entries).
    pub outfit: Vec<String>,
    /// Weapon in the right hand (`Objects.rpk` entry).
    pub weapon: Option<String>,
    /// Spawn an armed opponent in play mode.
    pub opponent: bool,
    /// The opponent stands its ground (no steps, no strikes).
    pub still_opponent: bool,
    /// A shield on the duellist's off arm (an object model, or `any`).
    pub opponent_shield: Option<String>,
    /// A shield on the player's off arm (an object model, or `any`).
    pub shield: Option<String>,
    /// Spawn living bodies in the original's order (key pick, pelvis on the
    /// frame, leg snap) instead of the default clip-pose spawn.
    pub original_spawn: bool,
    /// Play mode: the player spawns with the flat-floor bit (the original's
    /// lying spawn's state word).
    pub flat_floor: bool,
    /// Lay an arena match out as the original's template of this id does.
    pub arena_match: Option<i32>,
    /// The arena match's rank, for the generated opponents' level.
    pub arena_level: i32,
    /// The arena match's gear tier.
    pub arena_tier: i32,
    /// The drawn weapon variants, per opponent in spawn order. Zero is none.
    pub arena_weapons: Vec<u32>,
    /// Write the body state after every tick here (CSV).
    pub trace: Option<PathBuf>,
    /// Trace every simulated character body, the player's among them.
    pub trace_all: bool,
    /// Start with the body overlay shown.
    pub debug_bodies: bool,
    /// Start with the navigation grid drawn.
    pub debug_nav: bool,
    /// Refresh the navigation grid under moving meshes that come to rest
    /// (Exarust's debug option).
    pub nav_refresh_moved: bool,
    /// Keep hinged doors static level geometry instead of bodies under the
    /// hand (Exarust's debug option).
    pub static_doors: bool,
    /// Keep levers and push surfaces static level geometry (Exarust's
    /// debug option).
    pub static_mechanisms: bool,
    /// Make sliding doors bodies with their rules (Exarust's debug option:
    /// their lift is not reproduced).
    pub sliding_doors: bool,
    /// Open one empty form through the GUI draw path.
    pub debug_gui_form: bool,
    /// Open the menu and the Settings form on this page (0 Settings, 1
    /// Controls) once loaded.
    pub open_settings: Option<u8>,
    /// Open the character creator in this mode once loaded
    /// (`--open-creator`).
    pub open_creator: Option<String>,
    /// Play mode: open the placed container nearest the player in its
    /// popup this many frames after the load (`--open-container`).
    pub open_container: Option<u32>,
    /// Play mode: the debug path's goal (`--debug-path`, metres).
    pub debug_path: Option<[f32; 3]>,
    /// Play mode: the level character with this record id takes the
    /// Explorer role and follows the player, searching the dead
    /// (`--searcher`).
    pub searcher: Option<u32>,
    /// Once loaded, log every mesh over these floor points (x, z in metres).
    pub debug_column: Vec<[f32; 2]>,
    /// Log the frame profiler's report every this many seconds.
    pub perf_log: Option<f32>,
    /// Turn the frame profiler on with its overlay shown (F10 toggles it).
    pub profiler: bool,
    /// Write every joint after every tick here (CSV).
    pub joint_trace: Option<PathBuf>,
    /// Write every contact between body parts after every tick here (CSV).
    pub pair_trace: Option<PathBuf>,
    /// Every frame advances game time by exactly 1/60 s, so that runs
    /// repeat tick for tick.
    pub fixed_frames: bool,
    /// The simulation's speed, its scripted pauses (tick; cm past a slack).
    pub sim_speed: Option<f32>,
    pub pause_at_tick: Option<u64>,
    pub pause_on_stretch: Option<f32>,
    /// Pause the simulation this many ticks after the first blood jet.
    pub pause_after_jet: Option<u64>,
    /// Pause the simulation this many ticks after the player's first blow
    /// lands.
    pub pause_on_hit: Option<u64>,
    /// Pause the simulation this many ticks after the first body starts
    /// its collapse.
    pub pause_after_collapse: Option<u64>,
    /// Keys and buttons pressed at frames after the load (for unattended
    /// checks): each is held over its frames and let go at the range's end.
    pub press: Vec<(Press, Range<u32>)>,
    /// Play mode: a cursor fixed at this window point (logical pixels).
    pub cursor: Option<[f32; 2]>,
    /// Left mouse button clicks at frames after the load.
    pub clicks: Vec<u32>,
    /// `--object-click`: scripted clicks on the nearest placed container.
    pub object_clicks: Vec<exa_game::object_click::ScriptedObjectClick>,
    /// Play mode: in-process level changes at frames after the first load
    /// (`level@frame`), each logged with an entity, physics and asset count.
    pub level_changes: Vec<(String, u32)>,
    /// `--save-dir`: the folder the running game saves into and loads
    /// arena slots from.
    pub save_dir: Option<std::path::PathBuf>,
    /// `--save-at`: frames after the load at which a save to a new slot is
    /// asked for.
    pub save_at: Vec<u32>,
    /// `--arena-slot`: the arena slot the hub loads once entered.
    pub arena_slot: Option<u32>,
    /// Test wounds on the player's or the character view's body.
    pub debug_wounds: Vec<exa_character::blood::DebugWound>,
    /// Test blood jets from the player's or the character view's body.
    pub debug_sprays: Vec<exa_character::spray::DebugSpray>,
    /// Blood the player's or the character view's weapon starts with.
    pub weapon_blood: Option<exa_character::blood::DebugWeaponBlood>,
    /// Test thaumaturgy effects on the player.
    pub thaum_debug: exa_game::thaum::live::ThaumDebug,
    /// The player's starting yellow and red health.
    pub health: Option<[f32; 2]>,
    /// The player's perks.
    pub perks: exa_formats::characters::Perks,
    /// Let the low-health heartbeat play in any level.
    pub heartbeat: bool,
    /// Refill the player's health after every frame and body tick.
    pub undying: bool,
    /// Play mode: every arena opponent drops dead at this frame after the
    /// load, so the round ends and its end screen opens (a debug tool).
    pub arena_fell: Option<u32>,
    /// `--arena-auto`: the auto-match switch (continue and rematch alone).
    pub arena_auto: bool,
    /// Play mode: the player's record's feral level (0..=1).
    pub feral: Option<f32>,
    /// Play mode: powers the player knows (`--powers`, hex or decimal).
    pub powers: Vec<u32>,
}

/// What a `--press` entry presses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Key(KeyCode),
    Button(MouseButton),
}

/// The window's title, also the caption of the original's message boxes.
const WINDOW_TITLE: &str = "Exarust";

fn usage() -> AppExit {
    eprintln!(
        "usage: exarust [--game <dir>] [--camera x,y,z,yaw,pitch] [--screenshot out.png] \
         (level <name> | model <pack> <entry> | texture <name>)"
    );
    AppExit::from_code(2)
}

/// The key a `--press` entry names: a letter, `.`, `[`, `]`, `Tab`, `Ctrl`,
/// `Shift` or `F1`–`F12`.
fn key_named(name: &str) -> Option<KeyCode> {
    let fkeys = [
        KeyCode::F1,
        KeyCode::F2,
        KeyCode::F3,
        KeyCode::F4,
        KeyCode::F5,
        KeyCode::F6,
        KeyCode::F7,
        KeyCode::F8,
        KeyCode::F9,
        KeyCode::F10,
        KeyCode::F11,
        KeyCode::F12,
    ];
    let digits = [
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    let letters = [
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
    ];
    match name {
        "." => Some(KeyCode::Period),
        "[" => Some(KeyCode::BracketLeft),
        "]" => Some(KeyCode::BracketRight),
        "Tab" => Some(KeyCode::Tab),
        "Esc" => Some(KeyCode::Escape),
        "Enter" => Some(KeyCode::Enter),
        "Ctrl" => Some(KeyCode::ControlLeft),
        "Shift" => Some(KeyCode::ShiftLeft),
        _ => {
            if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<usize>().ok()) {
                return n.checked_sub(1).and_then(|i| fkeys.get(i)).copied();
            }
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphabetic() => letters
                    .get((c.to_ascii_uppercase() as u8 - b'A') as usize)
                    .copied(),
                // The digit row (the power bar's slot keys).
                (Some(c), None) if c.is_ascii_digit() => {
                    digits.get((c as u8 - b'0') as usize).copied()
                }
                _ => None,
            }
        }
    }
}

/// The perks a `--perks` list names (case does not matter), none for an
/// empty list; `None` when a name is not a perk.
fn parse_perks(list: &str) -> Option<exa_formats::characters::Perks> {
    use exa_formats::characters::{Perk, Perks};
    let mut perks = Perks::default();
    for name in list.split(',').map(str::trim).filter(|n| !n.is_empty()) {
        let perk = Perk::ALL
            .into_iter()
            .find(|p| p.name().eq_ignore_ascii_case(name))?;
        perks.insert(perk);
    }
    Some(perks)
}

/// The mouse button a `--press` entry names: `LMB`, `RMB` or `MMB`.
fn button_named(name: &str) -> Option<MouseButton> {
    match name {
        "LMB" => Some(MouseButton::Left),
        "RMB" => Some(MouseButton::Right),
        "MMB" => Some(MouseButton::Middle),
        _ => None,
    }
}

/// Reads `key@frame,...`, where a frame may be a range `from-to` the key
/// is held over (let go at `to`); a single frame is let go the frame after.
fn parse_presses(list: &str) -> Option<Vec<(Press, Range<u32>)>> {
    list.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|entry| {
            let (key, frames) = entry.trim().rsplit_once('@')?;
            let press = button_named(key)
                .map(Press::Button)
                .or_else(|| key_named(key).map(Press::Key))?;
            let (from, to) = match frames.split_once('-') {
                Some((from, to)) => (from.parse().ok()?, to.parse().ok()?),
                None => {
                    let at: u32 = frames.parse().ok()?;
                    (at, at.checked_add(1)?)
                }
            };
            (to > from).then_some((press, from..to))
        })
        .collect()
}

/// Reads the frame-rate cap: a positive rate, `auto` (the display's rate)
/// or `0` (off).
fn parse_max_fps(value: &str) -> Option<i32> {
    if value == "auto" {
        return Some(-1);
    }
    match value.parse::<i32>() {
        Ok(n) if n >= 0 => Some(n),
        _ => None,
    }
}

/// Reads `level@frame,...`.
fn parse_level_changes(list: &str) -> Option<Vec<(String, u32)>> {
    list.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|entry| {
            let (level, frame) = entry.trim().rsplit_once('@')?;
            (!level.is_empty()).then_some((level.to_string(), frame.parse().ok()?))
        })
        .collect()
}

/// Reads `frame,...`.
fn parse_clicks(list: &str) -> Option<Vec<u32>> {
    list.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|f| f.trim().parse().ok())
        .collect()
}

/// Registers the debug commands the original's units install: `cshd` from
/// the shader programs and `msm` from the mesh subdivider.
fn register_debug_commands(
    mut registry: ResMut<exa_game::commands::CommandRegistry>,
    server: Res<AssetServer>,
) {
    let server = server.clone();
    registry.register("cshd", move |_| {
        let count = exa_render::shader_reload::reload(&server);
        info!("cshd: reloading {count} shader programs");
    });
    registry.register("msm", |argument| {
        // The subdivider's own scene walk needs a live level; the model view
        // smooths at load through `--msm`, which runs the same Loop
        // subdivision on every mesh.
        let argument = String::from_utf8_lossy(argument);
        if argument.is_empty() {
            info!("msm: smoothing a whole scene; use the model view's --msm");
        } else {
            info!("msm: smoothing {argument}; use the model view's --msm");
        }
    });
}

fn main() -> AppExit {
    let mut args = std::env::args().skip(1).peekable();
    let mut game_dir: Option<PathBuf> = None;
    let mut options = Options {
        frames: 90,
        ev100: 6.0,
        ..default()
    };
    let mut positional = Vec::new();
    while let Some(a) = args.next() {
        match a.as_str() {
            "--game" => game_dir = args.next().map(PathBuf::from),
            "--screenshot" => options.screenshot = args.next().map(PathBuf::from),
            "--all-rooms" => options.all_rooms = true,
            "--limp" => options.limp = true,
            "--msm" => options.msm = true,
            "--no-shadows" => options.no_shadows = true,
            "--bevy-sun-cascades" => options.bevy_sun_cascades = true,
            "--no-gi" => options.gi.enabled = false,
            "--no-supersample" => options.no_supersample = true,
            "--no-fps" => options.no_fps = true,
            "--splash" => options.splash = true,
            "--benchmark" => options.benchmark = true,
            "--no-vsync" => options.no_vsync = true,
            "--max-fps" => {
                let value = args.next().unwrap_or_default();
                match parse_max_fps(&value) {
                    Some(cap) => options.max_fps = Some(cap),
                    None => {
                        eprintln!("--max-fps: cannot read {value:?} (a rate, auto or 0)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--no-settings" => options.no_settings = true,
            "--gi-warm-up" => {
                if let Some(n) = args.next().and_then(|v| v.parse().ok()) {
                    options.gi.warm_up = n;
                }
            }
            "--size" => {
                options.size = args.next().and_then(|v| {
                    let (w, h) = v.split_once('x')?;
                    Some((w.parse().ok()?, h.parse().ok()?))
                })
            }
            "--fov" => options.fov = args.next().and_then(|v| v.parse().ok()),
            "--race" => options.race = args.next().and_then(|v| v.parse().ok()),
            "--physique" => {
                let text = args.next().unwrap_or_default();
                match exa_assets::morph::parse_physique(&text) {
                    Some(p) => options.physique = Some(p),
                    None => {
                        eprintln!(
                            "--physique: cannot read {text:?} (x,y[,height,age,feral,female])"
                        );
                        return AppExit::from_code(2);
                    }
                }
            }
            "--shove" => options.shove = args.next().and_then(|v| v.parse().ok()),
            "--shove-props" => options.shove_props = args.next().and_then(|v| v.parse().ok()),
            "--kinesis-pick" => options.kinesis_pick = args.next().and_then(|v| v.parse().ok()),
            "--autowalk" => options.autowalk = true,
            "--weapon" => options.weapon = args.next(),
            "--opponent" => options.opponent = true,
            "--opponent-shield" => {
                options.opponent = true;
                options.opponent_shield = args.next();
            }
            "--shield" => options.shield = args.next(),
            "--arena-match" => {
                options.opponent = true;
                options.arena_match = args.next().and_then(|v| v.parse().ok());
            }
            "--arena-level" => {
                options.opponent = true;
                options.arena_level = args.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            "--arena-tier" => {
                options.opponent = true;
                options.arena_tier = args.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            }
            "--arena-weapons" => {
                options.opponent = true;
                options.arena_weapons = args
                    .next()
                    .map(|list| {
                        list.split(',')
                            .filter_map(|value| value.parse().ok())
                            .collect()
                    })
                    .unwrap_or_default();
            }
            "--original-spawn" => options.original_spawn = true,
            "--flat-floor" => options.flat_floor = true,
            "--still-opponent" => {
                options.opponent = true;
                options.still_opponent = true;
            }
            "--trace" => options.trace = args.next().map(PathBuf::from),
            "--trace-all" => options.trace_all = true,
            "--debug-bodies" => options.debug_bodies = true,
            "--debug-nav" => options.debug_nav = true,
            "--nav-refresh-moved" => options.nav_refresh_moved = true,
            "--static-doors" => options.static_doors = true,
            "--static-mechanisms" => options.static_mechanisms = true,
            "--sliding-doors" => options.sliding_doors = true,
            "--debug-gui-form" => options.debug_gui_form = true,
            "--open-settings" => {
                let page = args.next().unwrap_or_default();
                options.open_settings = match page.as_str() {
                    "settings" => Some(0),
                    "controls" => Some(1),
                    _ => {
                        eprintln!("--open-settings: settings or controls, not {page:?}");
                        return AppExit::from_code(2);
                    }
                };
            }
            "--open-creator" => {
                let word = args.next().unwrap_or_default();
                if exa_game::creator_screen::CreatorEntry::parse(&word).is_none() {
                    eprintln!(
                        "--open-creator: story, story-all, story:N, practice, expert, arena, recruit or versus, not {word:?}"
                    );
                    return AppExit::from_code(2);
                }
                options.open_creator = Some(word);
            }
            "--open-container" => options.open_container = args.next().and_then(|v| v.parse().ok()),
            "--perf-log" => options.perf_log = args.next().and_then(|v| v.parse().ok()),
            "--profiler" => options.profiler = true,
            "--debug-column" => {
                options.debug_column = args
                    .next()
                    .unwrap_or_default()
                    .split('/')
                    .filter_map(|point| {
                        let v: Vec<f32> = point
                            .split(',')
                            .filter_map(|x| x.trim().parse().ok())
                            .collect();
                        <[f32; 2]>::try_from(v.as_slice()).ok()
                    })
                    .collect();
            }
            "--joint-trace" => options.joint_trace = args.next().map(PathBuf::from),
            "--pair-trace" => options.pair_trace = args.next().map(PathBuf::from),
            "--fixed-frames" => options.fixed_frames = true,
            "--sim-speed" => {
                options.sim_speed = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|s: &f32| *s > 0.0 && s.is_finite())
            }
            "--pause-at-tick" => options.pause_at_tick = args.next().and_then(|v| v.parse().ok()),
            "--pause-after-jet" => {
                options.pause_after_jet = args.next().and_then(|v| v.parse().ok())
            }
            "--pause-on-stretch" => {
                options.pause_on_stretch = args.next().and_then(|v| v.parse().ok())
            }
            "--pause-on-hit" => options.pause_on_hit = args.next().and_then(|v| v.parse().ok()),
            "--pause-after-collapse" => {
                options.pause_after_collapse = args.next().and_then(|v| v.parse().ok())
            }
            "--press" => {
                let list = args.next().unwrap_or_default();
                match parse_presses(&list) {
                    Some(p) => options.press = p,
                    None => {
                        eprintln!("--press: cannot read {list:?} (key@frame,...)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--cursor" | "--health" => {
                let v: Vec<f32> = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect();
                let pair = <[f32; 2]>::try_from(v.as_slice()).ok();
                if a == "--cursor" {
                    options.cursor = pair;
                } else {
                    options.health = pair;
                }
            }
            "--heartbeat" => options.heartbeat = true,
            "--undying" => options.undying = true,
            "--arena-fell" => options.arena_fell = args.next().and_then(|v| v.parse().ok()),
            "--arena-auto" => options.arena_auto = true,
            "--debug-arcs" => options.thaum_debug.arcs = true,
            "--debug-barricade" => options.thaum_debug.barricade = true,
            "--debug-wisps" => options.thaum_debug.wisps = true,
            "--powers" => {
                let list = args.next().unwrap_or_default();
                match exa_game::thaum::cast::parse_power_list(&list) {
                    Some(powers) => options.powers = powers,
                    None => {
                        eprintln!("--powers: cannot read {list:?} (id,id,...)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--feral" => {
                options.feral = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .filter(|f: &f32| f.is_finite())
            }
            "--perks" => {
                let list = args.next().unwrap_or_default();
                match parse_perks(&list) {
                    Some(p) => options.perks = p,
                    None => {
                        eprintln!("--perks: cannot read {list:?} (remise,feint,...)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--outfit" => {
                options.outfit = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            "--click" => {
                let list = args.next().unwrap_or_default();
                match parse_clicks(&list) {
                    Some(c) => options.clicks = c,
                    None => {
                        eprintln!("--click: cannot read {list:?} (frame,...)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--object-click" => {
                let list = args.next().unwrap_or_default();
                match exa_game::object_click::ScriptedObjectClick::parse_list(&list) {
                    Some(c) => options.object_clicks = c,
                    None => {
                        eprintln!("--object-click: cannot read {list:?} (frame[d][l],...)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--save-dir" => options.save_dir = args.next().map(std::path::PathBuf::from),
            "--save-at" => {
                let list = args.next().unwrap_or_default();
                match list
                    .split(',')
                    .map(|frame| frame.trim().parse::<u32>().ok())
                    .collect::<Option<Vec<u32>>>()
                {
                    Some(frames) => options.save_at = frames,
                    None => {
                        eprintln!("--save-at: cannot read {list:?} (frame,...)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--arena-slot" => {
                let text = args.next().unwrap_or_default();
                match text.parse::<u32>() {
                    Ok(number) if (1..=999).contains(&number) => options.arena_slot = Some(number),
                    _ => {
                        eprintln!("--arena-slot: cannot read {text:?} (1-999)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--level-change" => {
                let list = args.next().unwrap_or_default();
                match parse_level_changes(&list) {
                    Some(changes) => options.level_changes = changes,
                    None => {
                        eprintln!("--level-change: cannot read {list:?} (level@frame,...)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--debug-wound" => {
                let text = args.next().unwrap_or_default();
                match exa_character::blood::DebugWound::parse(&text) {
                    Some(w) => options.debug_wounds.push(w),
                    None => {
                        eprintln!(
                            "--debug-wound: cannot read {text:?} (bone,radius_cm[,dx,dy,dz])"
                        );
                        return AppExit::from_code(2);
                    }
                }
            }
            "--debug-spray" => {
                let text = args.next().unwrap_or_default();
                match exa_character::spray::DebugSpray::parse(&text) {
                    Some(s) => options.debug_sprays.push(s),
                    None => {
                        eprintln!("--debug-spray: cannot read {text:?} (bone,R[,dx,dy,dz])");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--weapon-blood" => {
                let text = args.next().unwrap_or_default();
                match exa_character::blood::DebugWeaponBlood::parse(&text) {
                    Some(b) => options.weapon_blood = Some(b),
                    None => {
                        eprintln!("--weapon-blood: cannot read {text:?} (a number from 0 to 1)");
                        return AppExit::from_code(2);
                    }
                }
            }
            "--ev" => options.ev100 = args.next().and_then(|v| v.parse().ok()).unwrap_or(6.0),
            "--frames" => options.frames = args.next().and_then(|v| v.parse().ok()).unwrap_or(90),
            "--camera" => {
                let v: Vec<f32> = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect();
                if let Ok(pose) = <[f32; 5]>::try_from(v.as_slice()) {
                    options.camera = Some(pose);
                }
            }
            "--searcher" => {
                let v = args.next().unwrap_or_default();
                let id = match v.strip_prefix("0x") {
                    Some(hex) => u32::from_str_radix(hex, 16).ok(),
                    None => v.parse().ok(),
                };
                if id.is_none() {
                    eprintln!("--searcher: cannot read {v:?} (a record id)");
                }
                options.searcher = id;
            }
            "--corpse" => {
                let v: Vec<f32> = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect();
                match <[f32; 3]>::try_from(v.as_slice()) {
                    Ok(at) => options.corpses.push(at),
                    Err(_) => eprintln!("--corpse: cannot read {v:?} (x,y,z)"),
                }
            }
            "--at" | "--torch" | "--debug-path" => {
                let v: Vec<f32> = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect();
                let point = <[f32; 3]>::try_from(v.as_slice()).ok();
                match a.as_str() {
                    "--at" => options.at = point,
                    "--torch" => options.torch = point,
                    _ => options.debug_path = point,
                }
            }
            _ => positional.push(a),
        }
    }
    let view = match positional
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .as_slice()
    {
        ["level", name] => View::Level((*name).to_string()),
        ["model", pack, entry @ ..] if !entry.is_empty() => View::Model {
            pack: (*pack).to_string(),
            entry: entry.join(" "),
        },
        ["texture", name] => View::Texture((*name).to_string()),
        ["play", name] => View::Play((*name).to_string()),
        ["character"] => View::Character {
            motion: "Default/Walk".into(),
        },
        ["character", motion @ ..] => View::Character {
            motion: motion.join(" "),
        },
        [] => View::Texture("humandiff".into()),
        _ => return usage(),
    };

    let Some(root) = locate_install(game_dir.as_deref()) else {
        eprintln!(
            "no Exanima installation: pass --game <dir> or set EXANIMA_DIR (see .env.example)"
        );
        return AppExit::from_code(2);
    };
    let data = match GameData::open(&root) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("cannot open {}: {e}", root.display());
            return AppExit::from_code(2);
        }
    };
    let user = if options.no_settings {
        None
    } else {
        user_settings::load(&data.project().name)
    };
    if let Some(user) = &user {
        user.apply_to(&mut options);
    }
    // The settings screen's options as the launcher starts with them; its
    // plugin applies the renderer, audio and window parts.
    let hardware = if let Some(user) = &user {
        let mut hardware = user.hardware(
            options.fullscreen,
            options.size,
            options.no_vsync,
            options.no_supersample,
            options.no_fps,
        );
        if options.benchmark {
            hardware.start_benchmark();
        }
        Some(hardware)
    } else {
        None
    };
    let trace = match options
        .trace
        .as_deref()
        .map(exa_game::trace::BodyTrace::create)
    {
        Some(Err(e)) => {
            eprintln!("cannot write the trace: {e}");
            return AppExit::from_code(2);
        }
        Some(Ok(t)) => Some(t.every_body(options.trace_all)),
        None => None,
    };
    let joint_trace = match options
        .joint_trace
        .as_deref()
        .map(exa_game::joint_trace::JointTrace::create)
    {
        Some(Err(e)) => {
            eprintln!("cannot write the joint trace: {e}");
            return AppExit::from_code(2);
        }
        Some(Ok(t)) => Some(t),
        None => None,
    };
    let pair_trace = match options
        .pair_trace
        .as_deref()
        .map(exa_game::pair_trace::PairTrace::create)
    {
        Some(Err(e)) => {
            eprintln!("cannot write the pair trace: {e}");
            return AppExit::from_code(2);
        }
        Some(Ok(t)) => Some(t),
        None => None,
    };

    let fixed_frames = options.fixed_frames;
    let mut app = App::new();
    // The original holds a background window to 32 ms a frame (about 31 a
    // second) and runs the foreground one as fast as its pacing allows.
    // Winit's low-power reactive mode wakes on the window's own events and
    // otherwise waits 32 ms. Unattended runs (a screenshot, fixed frames)
    // keep running at full speed in the background.
    let unattended = options.screenshot.is_some() || options.fixed_frames;
    app.insert_resource(bevy::winit::WinitSettings {
        focused_mode: bevy::winit::UpdateMode::Continuous,
        unfocused_mode: if unattended {
            bevy::winit::UpdateMode::Continuous
        } else {
            bevy::winit::UpdateMode::reactive_low_power(std::time::Duration::from_millis(
                exa_game::frame::UNFOCUSED_INTERVAL_MS as u64,
            ))
        },
    });
    if let Some(trace) = trace {
        app.insert_resource(trace);
    }
    if options.original_spawn {
        app.insert_resource(exa_game::limp::OriginalSpawn);
    }
    if !options.debug_wounds.is_empty() {
        app.insert_resource(exa_character::blood::DebugWounds(
            options.debug_wounds.clone(),
        ));
    }
    if !options.debug_sprays.is_empty() {
        app.insert_resource(exa_character::spray::DebugSprays(
            options.debug_sprays.clone(),
        ));
    }
    if let Some(blood) = options.weapon_blood {
        app.insert_resource(blood);
    }
    if options.thaum_debug != exa_game::thaum::live::ThaumDebug::default() {
        app.insert_resource(options.thaum_debug);
    }
    if let Some(trace) = joint_trace {
        app.insert_resource(trace);
    }
    if let Some(trace) = pair_trace {
        app.insert_resource(trace);
    }
    // The original's level-6 box for a graphics device that cannot run
    // the renderer, captioned with the window's title.
    message_box::install_gpu_failure_box(WINDOW_TITLE, !unattended);
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: WINDOW_TITLE.into(),
                    // The Wayland app id (X11 class): without one, GNOME
                    // names the window "Unknown" in its dialogs.
                    name: Some("exarust".into()),
                    // The original's default window when the settings name
                    // no size: 1600 x 1080.
                    resolution: options.size.unwrap_or((1600, 1080)).into(),
                    // The original's track-size minimum.
                    resize_constraints: bevy::window::WindowResizeConstraints {
                        min_width: exa_game::window::MIN_WINDOW_SIZE.x as f32,
                        min_height: exa_game::window::MIN_WINDOW_SIZE.y as f32,
                        ..default()
                    },
                    mode: frame_loop::launch_window_mode(options.fullscreen),
                    // Swap interval 1 (adaptive while Exarust's frames are
                    // slower than the display) or 0.
                    present_mode: exa_game::window::present_mode(
                        !options.no_vsync,
                        exa_game::window::ADAPTIVE_VSYNC,
                    ),
                    ..default()
                }),
                ..default()
            })
            // A stalled frame breaks the ALSA stream, and rodio then
            // reports the same error in a tight loop (gigabytes of log).
            .set(bevy::log::LogPlugin {
                filter: format!("{},rodio::stream=off", bevy::log::DEFAULT_FILTER),
                ..default()
            }),
    )
    .add_plugins(exa_game::GamePlugin)
    .insert_resource(Game(Arc::new(data)))
    .insert_resource(exa_game::flow::BootToMenu(false))
    .insert_resource(view)
    .insert_resource(options.gi)
    .insert_resource(exa_ui::ShowFps(!options.no_fps))
    .insert_resource(exa_game::splash::SplashAtStart(options.splash))
    .insert_resource(exa_game::gui_screens::DebugGuiForm {
        enabled: options.debug_gui_form,
    })
    .insert_resource(exa_game::settings_form::OpenSettingsOnLoad {
        page: options.open_settings,
        done: false,
    })
    .insert_resource(exa_game::creator_screen::OpenCreatorOnLoad {
        word: options.open_creator.clone(),
        done: false,
    })
    .insert_resource(exa_game::gui_screens::DebugOpenContainer(
        options.open_container,
    ))
    .insert_resource(exa_ui::ScriptedPointer(options.cursor.map(Vec2::from)))
    .insert_resource(exa_render::cube_shadows::PointShadows(!options.no_shadows))
    .insert_resource(exa_world::sun_shadow::SunShadowFit(
        !options.bevy_sun_cascades,
    ))
    .insert_resource(exa_game::body_overlay::BodyOverlay {
        on: options.debug_bodies,
        ..default()
    })
    .insert_resource(exa_world::nav::NavOverlay {
        on: options.debug_nav,
    })
    .insert_resource(exa_world::nav::NavRefreshMoved(options.nav_refresh_moved));
    if let Some(id) = options.searcher {
        app.insert_resource(exa_game::npc_search::DebugSearcher(id));
    }
    app.insert_resource(match options.debug_path {
        Some(goal) => exa_game::path_probe::PathProbe::to(Vec3::from(goal)),
        None => exa_game::path_probe::PathProbe::default(),
    })
    .insert_resource(
        exa_game::sim_control::SimControl::scripted(
            options.sim_speed.unwrap_or(1.0),
            options.pause_at_tick,
            // Centimetres on the command line.
            options.pause_on_stretch.map(|cm| cm / 100.0),
        )
        .after_jet(options.pause_after_jet)
        .with_pause_on_hit(options.pause_on_hit)
        .with_pause_after_collapse(options.pause_after_collapse),
    )
    .insert_resource(exa_game::object_click::ObjectClickScript(
        options.object_clicks.clone(),
    ))
    .insert_resource(exa_game::save_live::ScriptedSaves(options.save_at.clone()));
    if let Some(dir) = &options.save_dir {
        app.insert_resource(exa_game::save_live::SaveFolder(dir.clone()));
        if let Some(number) = options.arena_slot {
            app.insert_resource(exa_game::save_live::ArenaSlotLoad { number });
        }
    }
    app.insert_resource(options).add_plugins((
        frame_loop::FrameLoopPlugin,
        viewer::ViewerPlugin,
        column::ColumnPlugin,
        map_panel::MapPanelPlugin,
        profiler::ProfilerPlugin,
    ));
    if let Some(hardware) = &hardware {
        app.insert_resource(exa_render::quality::ShadowQuality(
            hardware.live.shadow_quality,
        ))
        .insert_resource(exa_render::quality::MultisampleSetting(
            hardware.live.multisample,
        ))
        .insert_resource(exa_render::supersample::SuperSampling(
            hardware.live.supersample > 0,
        ))
        .insert_resource(exa_audio::MusicSetting {
            enabled: hardware.live.music,
        })
        .insert_resource(hardware.clone());
    }
    if let Some(user) = &user {
        if let Some(bindings) = user.bindings() {
            app.insert_resource(bindings);
        }
        app.insert_resource(exa_game::commands::KeyCommands::of_hotkeys(
            &user.settings.hotkeys,
        ));
        app.insert_resource(exa_game::flow::UnlockSettings::from_store(&user.store))
            .insert_resource(user.clone())
            .add_systems(
                Update,
                (
                    user_settings::gui_size_from_display,
                    user_settings::save_applied_bindings,
                ),
            );
    }
    app.add_systems(Startup, register_debug_commands);
    if fixed_frames {
        // Whatever a frame took, it advances game time by 1/60 s: the frames'
        // clocks and the ticks they run repeat from run to run.
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_secs_f64(1.0 / 60.0),
        ));
    }
    let exit = app.run();
    if let Some(user) = &user {
        user.save_store_at_exit();
    }
    exit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perks_are_read_by_name() {
        use exa_formats::characters::Perk;
        let perks = parse_perks("Remise, feint").unwrap();
        assert_eq!(
            perks.iter().collect::<Vec<_>>(),
            [Perk::Remise, Perk::Feint]
        );
        assert!(parse_perks("").unwrap().is_empty());
        assert_eq!(parse_perks("remise,parry"), None);
    }

    #[test]
    fn scripted_presses_are_read() {
        let key = |key, at: u32| (Press::Key(key), at..at + 1);
        let button = |button, at: u32| (Press::Button(button), at..at + 1);
        assert_eq!(
            parse_presses(".@400, P@500,F3@1,]@2,[@3,Tab@4,f@5,LMB@6,RMB@7,MMB@8,Esc@9"),
            Some(vec![
                key(KeyCode::Period, 400),
                key(KeyCode::KeyP, 500),
                key(KeyCode::F3, 1),
                key(KeyCode::BracketRight, 2),
                key(KeyCode::BracketLeft, 3),
                key(KeyCode::Tab, 4),
                key(KeyCode::KeyF, 5),
                button(MouseButton::Left, 6),
                button(MouseButton::Right, 7),
                button(MouseButton::Middle, 8),
                key(KeyCode::Escape, 9),
            ])
        );
        assert_eq!(parse_presses(""), Some(vec![]));
        // The digit row, for the power bar's slots.
        assert_eq!(
            parse_presses("C@60,1@70,Q@80"),
            Some(vec![
                key(KeyCode::KeyC, 60),
                key(KeyCode::Digit1, 70),
                key(KeyCode::KeyQ, 80),
            ])
        );
        assert_eq!(parse_clicks("300, 420,540"), Some(vec![300, 420, 540]));
        assert_eq!(parse_clicks("300,x"), None);
        assert_eq!(
            parse_level_changes("arenahub@200, arena1@450"),
            Some(vec![("arenahub".into(), 200), ("arena1".into(), 450)])
        );
        assert_eq!(parse_level_changes("arenahub"), None);
        assert_eq!(parse_level_changes("@20"), None);
        for bad in [
            "P", "P@x", "F13@1", "esc@2", "@3", "lmb@4", "XMB@5", "RMB@5-5", "RMB@9-4", "W@1-x",
        ] {
            assert_eq!(parse_presses(bad), None, "{bad}");
        }
    }

    #[test]
    fn the_frame_rate_cap_is_read() {
        assert_eq!(parse_max_fps("60"), Some(60));
        assert_eq!(parse_max_fps("0"), Some(0));
        assert_eq!(parse_max_fps("auto"), Some(-1));
        for bad in ["-1", "x", "", "60.0"] {
            assert_eq!(parse_max_fps(bad), None, "{bad}");
        }
    }

    #[test]
    fn scripted_holds_and_mouse_buttons_are_read() {
        assert_eq!(
            parse_presses("RMB@100-400,Ctrl@50-60,LMB@7,Shift@10-20"),
            Some(vec![
                (Press::Button(MouseButton::Right), 100..400),
                (Press::Key(KeyCode::ControlLeft), 50..60),
                (Press::Button(MouseButton::Left), 7..8),
                (Press::Key(KeyCode::ShiftLeft), 10..20),
            ])
        );
    }
}

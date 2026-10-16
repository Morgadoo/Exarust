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
//!          --torch x,y,z              a burning torch fire here (metres), e.g. to stand in for a held one
//!          --fov <degrees>            vertical field of view (default 45)
//!          --no-shadows               lights cast no shadows (for software rendering)
//!          --no-gi                    no voxel GI (ambient from the environment cube)
//!          --no-supersample           draw at window resolution (default: 4× supersampling, as the original)
//!          --gi-warm-up <steps>       GI steps run when a level loads (default 600)
//!          --size WxH                 window size (default 1600x900)
//!          --screenshot <file.png>    save a frame and exit
//!          --frames <n>               frames to wait before the screenshot, after loading (default 90)
//!          --ev <ev100>               camera exposure (default 6, lower is brighter)
//!          --all-rooms                tile-grid levels: show every room (no black for rooms the player cannot see)
//!          --limp                     character view: the body goes limp and falls
//!          --race <n>                 character view: the body of race n (a character record's race byte)
//!          --shove <frame>            a body blow to every simulated body at that frame after loading
//!          --outfit a,b,c             apparel for character/play (Characters.rpk entries), over the underwear
//!          --weapon <name>            weapon in the right hand (Objects.rpk entry)
//!          --opponent                 an armed duellist in play mode; practice and arena fights start in combat mode (Tab toggles it, left click attacks)
//!          --still-opponent           the duellist stands its ground: it faces the player in its stance but neither walks nor strikes
//!          --trace <file.csv>         the player's body (else every simulated body) after every 83 Hz tick, in cm
//!          --debug-bodies             start with the body overlay and joint readout shown (F3 toggles it)
//!          --debug-nav                draw the level's navigation grid around the view (F6 toggles it)
//!          --joint-trace <file.csv>   every joint of every simulated body after every 83 Hz tick (degrees, cm)
//!          --sim-speed <f>            run the simulation at this speed (e.g. 0.25)
//!          --pause-at-tick <n>        pause the simulation at the end of body tick n
//!          --pause-on-stretch <cm>    pause the simulation when a link goes this far past its slack
//!          --pause-after-jet <n>      pause the simulation n body ticks after the first blood jet
//!          --press key@frame,...      press keys (Ctrl among them) or mouse buttons (LMB, RMB, MMB) at frames after
//!                                     loading (e.g. `.@400,P@500,LMB@600`); `key@a-b` holds it from frame a to b
//!          --cursor x,y               play mode: the cursor stays at this window point (logical pixels), for unattended runs
//!          --click frame,...          click the left mouse button at frames after loading
//!          --debug-wound b,r[,dx,dy,dz]  a test wound on the player's (or the character view's) body part b
//!                                     (1–18 or chest, luarm, …), radius r cm, smeared along d cm; repeatable
//!          --weapon-blood <a>         the player's (or the character view's) weapon holds blood a (0–1) from the start
//!          --debug-spray b,R[,dx,dy,dz]  a blood jet from the player's (or the character view's) body part b every
//!                                     second, as a bleeding blow of size R swinging along d would spray; repeatable
//!          --health yellow,red        play mode: the player's starting health (a human's full health is 0.25)
//!          --heartbeat                play mode: the low-health heartbeat in any level (the original: story mode only)
//! ```
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
//! navigation grid, P pause, `.` step one tick, `[` / `]` slower / faster.

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
    /// The voxel GI settings.
    pub gi: exa_world::gi_volume::GiSettings,
    /// Draw at window resolution instead of 4× supersampling.
    pub no_supersample: bool,
    /// Hide the frame-rate readout.
    pub no_fps: bool,
    /// Present frames as soon as they are drawn (for measuring).
    pub no_vsync: bool,
    /// Window size in logical pixels.
    pub size: Option<(u32, u32)>,
    /// Play mode: start here instead of the level's start (metres).
    pub at: Option<[f32; 3]>,
    /// A torch fire at this point (metres).
    pub torch: Option<[f32; 3]>,
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
    /// Character view: the body of this race (character race byte).
    pub race: Option<u8>,
    /// Deal every simulated body a blow at this frame.
    pub shove: Option<u32>,
    /// Apparel for the character (`Characters.rpk` entries).
    pub outfit: Vec<String>,
    /// Weapon in the right hand (`Objects.rpk` entry).
    pub weapon: Option<String>,
    /// Spawn an armed opponent in play mode.
    pub opponent: bool,
    /// The opponent stands its ground (no steps, no strikes).
    pub still_opponent: bool,
    /// Write the body state after every tick here (CSV).
    pub trace: Option<PathBuf>,
    /// Start with the body overlay shown.
    pub debug_bodies: bool,
    /// Start with the navigation grid drawn.
    pub debug_nav: bool,
    /// Write every joint after every tick here (CSV).
    pub joint_trace: Option<PathBuf>,
    /// The simulation's speed, its scripted pauses (tick; cm past a slack).
    pub sim_speed: Option<f32>,
    pub pause_at_tick: Option<u64>,
    pub pause_on_stretch: Option<f32>,
    /// Pause the simulation this many ticks after the first blood jet.
    pub pause_after_jet: Option<u64>,
    /// Keys and buttons pressed at frames after the load (for unattended
    /// checks): each is held over its frames and let go at the range's end.
    pub press: Vec<(Press, Range<u32>)>,
    /// Play mode: a cursor fixed at this window point (logical pixels).
    pub cursor: Option<[f32; 2]>,
    /// Left mouse button clicks at frames after the load.
    pub clicks: Vec<u32>,
    /// Test wounds on the player's or the character view's body.
    pub debug_wounds: Vec<exa_character::blood::DebugWound>,
    /// Test blood jets from the player's or the character view's body.
    pub debug_sprays: Vec<exa_character::spray::DebugSpray>,
    /// Blood the player's or the character view's weapon starts with.
    pub weapon_blood: Option<exa_character::blood::DebugWeaponBlood>,
    /// The player's starting yellow and red health.
    pub health: Option<[f32; 2]>,
    /// Let the low-health heartbeat play in any level.
    pub heartbeat: bool,
}

/// What a `--press` entry presses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Press {
    Key(KeyCode),
    Button(MouseButton),
}

fn usage() -> AppExit {
    eprintln!(
        "usage: exarust [--game <dir>] [--camera x,y,z,yaw,pitch] [--screenshot out.png] \
         (level <name> | model <pack> <entry> | texture <name>)"
    );
    AppExit::from_code(2)
}

/// The key a `--press` entry names: a letter, `.`, `[`, `]`, `Tab`, `Ctrl`
/// or `F1`–`F12`.
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
        "Ctrl" => Some(KeyCode::ControlLeft),
        _ => {
            if let Some(n) = name.strip_prefix('F').and_then(|n| n.parse::<usize>().ok()) {
                return n.checked_sub(1).and_then(|i| fkeys.get(i)).copied();
            }
            let mut chars = name.chars();
            match (chars.next(), chars.next()) {
                (Some(c), None) if c.is_ascii_alphabetic() => letters
                    .get((c.to_ascii_uppercase() as u8 - b'A') as usize)
                    .copied(),
                _ => None,
            }
        }
    }
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

/// Reads `frame,...`.
fn parse_clicks(list: &str) -> Option<Vec<u32>> {
    list.split(',')
        .filter(|s| !s.trim().is_empty())
        .map(|f| f.trim().parse().ok())
        .collect()
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
            "--no-shadows" => options.no_shadows = true,
            "--no-gi" => options.gi.enabled = false,
            "--no-supersample" => options.no_supersample = true,
            "--no-fps" => options.no_fps = true,
            "--no-vsync" => options.no_vsync = true,
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
            "--shove" => options.shove = args.next().and_then(|v| v.parse().ok()),
            "--autowalk" => options.autowalk = true,
            "--weapon" => options.weapon = args.next(),
            "--opponent" => options.opponent = true,
            "--still-opponent" => {
                options.opponent = true;
                options.still_opponent = true;
            }
            "--trace" => options.trace = args.next().map(PathBuf::from),
            "--debug-bodies" => options.debug_bodies = true,
            "--debug-nav" => options.debug_nav = true,
            "--joint-trace" => options.joint_trace = args.next().map(PathBuf::from),
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
            "--at" | "--torch" => {
                let v: Vec<f32> = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .filter_map(|x| x.trim().parse().ok())
                    .collect();
                let point = <[f32; 3]>::try_from(v.as_slice()).ok();
                if a == "--at" {
                    options.at = point;
                } else {
                    options.torch = point;
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
    let trace = match options
        .trace
        .as_deref()
        .map(exa_game::trace::BodyTrace::create)
    {
        Some(Err(e)) => {
            eprintln!("cannot write the trace: {e}");
            return AppExit::from_code(2);
        }
        Some(Ok(t)) => Some(t),
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

    let mut app = App::new();
    if let Some(trace) = trace {
        app.insert_resource(trace);
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
    if let Some(trace) = joint_trace {
        app.insert_resource(trace);
    }
    app.add_plugins(
        DefaultPlugins
            .set(WindowPlugin {
                primary_window: Some(Window {
                    title: "Exarust".into(),
                    // The Wayland app id (X11 class): without one, GNOME
                    // names the window "Unknown" in its dialogs.
                    name: Some("exarust".into()),
                    resolution: options.size.unwrap_or((1600, 900)).into(),
                    present_mode: if options.no_vsync {
                        bevy::window::PresentMode::AutoNoVsync
                    } else {
                        bevy::window::PresentMode::AutoVsync
                    },
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
    .insert_resource(view)
    .insert_resource(options.gi)
    .insert_resource(exa_ui::ShowFps(!options.no_fps))
    .insert_resource(exa_ui::ScriptedPointer(options.cursor.map(Vec2::from)))
    .insert_resource(exa_render::cube_shadows::PointShadows(!options.no_shadows))
    .insert_resource(exa_game::body_overlay::BodyOverlay {
        on: options.debug_bodies,
        ..default()
    })
    .insert_resource(exa_world::nav::NavOverlay {
        on: options.debug_nav,
    })
    .insert_resource(
        exa_game::sim_control::SimControl::scripted(
            options.sim_speed.unwrap_or(1.0),
            options.pause_at_tick,
            // Centimetres on the command line.
            options.pause_on_stretch.map(|cm| cm / 100.0),
        )
        .after_jet(options.pause_after_jet),
    )
    .insert_resource(options)
    .add_plugins(viewer::ViewerPlugin)
    .run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scripted_presses_are_read() {
        let key = |key, at: u32| (Press::Key(key), at..at + 1);
        let button = |button, at: u32| (Press::Button(button), at..at + 1);
        assert_eq!(
            parse_presses(".@400, P@500,F3@1,]@2,[@3,Tab@4,f@5,LMB@6,RMB@7,MMB@8"),
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
            ])
        );
        assert_eq!(parse_presses(""), Some(vec![]));
        assert_eq!(parse_clicks("300, 420,540"), Some(vec![300, 420, 540]));
        assert_eq!(parse_clicks("300,x"), None);
        for bad in [
            "P", "P@x", "F13@1", "Esc@2", "@3", "lmb@4", "XMB@5", "RMB@5-5", "RMB@9-4", "W@1-x",
        ] {
            assert_eq!(parse_presses(bad), None, "{bad}");
        }
    }

    #[test]
    fn scripted_holds_and_mouse_buttons_are_read() {
        assert_eq!(
            parse_presses("RMB@100-400,Ctrl@50-60,LMB@7"),
            Some(vec![
                (Press::Button(MouseButton::Right), 100..400),
                (Press::Key(KeyCode::ControlLeft), 50..60),
                (Press::Button(MouseButton::Left), 7..8),
            ])
        );
    }
}

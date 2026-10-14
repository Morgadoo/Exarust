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
//!          --frames <n>               frames to wait before the screenshot (default 90)
//!          --ev <ev100>               camera exposure (default 6, lower is brighter)
//!          --ceilings                 build ceilings of tile-grid levels
//!          --limp                     character view: the body goes limp and falls
//!          --race <n>                 character view: the body of race n (a character record's race byte)
//!          --shove <frame>            a body blow to every simulated body at that frame
//!          --outfit a,b,c             apparel for character/play (Characters.rpk entries)
//!          --weapon <name>            weapon in the right hand (Objects.rpk entry)
//!          --opponent                 an armed duellist in play mode (F: combat mode, left click: attack)
//! ```
//!
//! Controls: hold the right mouse button to look, WASD to move, Q/E down/up,
//! Shift to go faster.

mod viewer;

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
    /// Build ceiling pieces of tile-grid levels.
    pub ceilings: bool,
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
}

fn usage() -> AppExit {
    eprintln!(
        "usage: exarust [--game <dir>] [--camera x,y,z,yaw,pitch] [--screenshot out.png] \
         (level <name> | model <pack> <entry> | texture <name>)"
    );
    AppExit::from_code(2)
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
            "--ceilings" => options.ceilings = true,
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
            "--outfit" => {
                options.outfit = args
                    .next()
                    .unwrap_or_default()
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
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

    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Exarust".into(),
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
        .insert_resource(options)
        .add_plugins(viewer::ViewerPlugin)
        .run()
}

//! Exarust launcher.
//!
//! ```text
//! exarust [--game <dir>] [options] level <name>          a level with its props and lights
//! exarust [--game <dir>] [options] model <pack> <entry>  one model
//! exarust [--game <dir>] [options] texture <name>        one texture (first light)
//!
//! options: --camera x,y,z,yaw,pitch   start pose (metres, degrees)
//!          --screenshot <file.png>    save a frame and exit
//!          --frames <n>               frames to wait before the screenshot (default 90)
//!          --ev <ev100>               camera exposure (default 6, lower is brighter)
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
}

#[derive(Resource, Clone, Debug, Default)]
pub struct Options {
    pub camera: Option<[f32; 5]>,
    pub screenshot: Option<PathBuf>,
    pub frames: u32,
    /// Camera exposure; dungeon interiors need far less than daylight.
    pub ev100: f32,
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
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Exarust".into(),
                resolution: (1600, 900).into(),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(exa_game::GamePlugin)
        .insert_resource(Game(Arc::new(data)))
        .insert_resource(view)
        .insert_resource(options)
        .add_plugins(viewer::ViewerPlugin)
        .run()
}

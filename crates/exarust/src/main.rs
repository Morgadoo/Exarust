//! Exarust launcher.
//!
//! Today this opens the installation, lists its packs and shows one texture
//! decoded from the original data ("first light"). Usage:
//!
//! ```text
//! exarust [--game <dir>] [texture-name]
//! ```

use std::path::PathBuf;
use std::sync::Arc;

use bevy::prelude::*;
use exa_assets::{Game, GameData, locate_install};

fn main() -> AppExit {
    let mut args = std::env::args().skip(1);
    let mut game_dir: Option<PathBuf> = None;
    let mut texture = String::from("humandiff");
    while let Some(a) = args.next() {
        match a.as_str() {
            "--game" => game_dir = args.next().map(PathBuf::from),
            _ => texture = a,
        }
    }

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
                ..default()
            }),
            ..default()
        }))
        .add_plugins(exa_game::GamePlugin)
        .insert_resource(Game(Arc::new(data)))
        .insert_resource(FirstLight(texture))
        .add_systems(Startup, first_light)
        .run()
}

#[derive(Resource)]
struct FirstLight(String);

fn first_light(
    mut commands: Commands,
    game: Res<Game>,
    request: Res<FirstLight>,
    mut images: ResMut<Assets<Image>>,
) {
    commands.spawn(Camera2d);

    let mut lines = vec![format!(
        "{} — {}",
        game.0.project().name,
        game.0.root().display()
    )];
    for (kind, pack) in game.0.packs() {
        lines.push(format!(
            "{:<13} {:>5} entries",
            kind.file_stem(),
            pack.len()
        ));
    }
    let image = match game.load_texture(&request.0, true) {
        Ok(img) => {
            lines.push(format!("texture: {}", request.0));
            Some(images.add(img))
        }
        Err(e) => {
            lines.push(format!("texture: {e}"));
            None
        }
    };

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            column_gap: Val::Px(24.0),
            padding: UiRect::all(Val::Px(16.0)),
            ..default()
        })
        .with_children(|root| {
            root.spawn((
                Text::new(lines.join("\n")),
                TextColor(Color::srgb(0.9, 0.85, 0.7)),
            ));
            if let Some(handle) = image {
                root.spawn((
                    ImageNode::new(handle),
                    Node {
                        height: Val::Percent(100.0),
                        aspect_ratio: Some(1.0),
                        ..default()
                    },
                ));
            }
        });
}

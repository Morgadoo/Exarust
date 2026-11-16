//! The game's App, built as `main` builds it but headless (no window, no
//! GPU), with the simulation bridge in lockstep: no system of the frame's
//! world or the simulation's may be left unrun for a missing resource or
//! message queue (OpenSpec `check-the-game-app-for-skipped-systems`).
//!
//! The workspace tests build their own small Apps, so a fixed-step system
//! the real game moves into the simulation's world but whose message or
//! resource the bridge does not carry was skipped on every tick unseen
//! (rounds 253 to 276: the casters' walk, the vanish turns, the bodies'
//! impact sounds). Here `exa_physics::skip_watch` records every such system
//! in both worlds while the scenes run.
//!
//! The data-free scene runs everywhere: the arena's command line with no
//! installation (`Game` left out), until the bridge has started and a few
//! hundred ticks have run; only the systems that need the installation may
//! be skipped. With `EXANIMA_DIR` the arena with a duellist and a cast, and
//! the first level, load and play a few hundred ticks with no skip but the
//! headless ones ([`HEADLESS`]).

use bevy::prelude::*;
use exa_physics::skip_watch::{self, Skip, Watched};

use super::{Shell, build_app, parse_command_line};

/// Parameters a headless App has no value for: a renderer's resources (no
/// GPU), the audio output (no device). A system skipped for one of these
/// would run in the game.
const HEADLESS: &[&str] = &[];

/// Parameters a data-free App has no value for: the installation, and what
/// the loads made from it insert.
const DATA: &[&str] = &["exa_assets::Game"];

/// Runs `command` (the game's command line) headless until the fixed step
/// has run `ticks` ticks after the loading screen (at most `updates`
/// frames); returns what the watch saw and the ticks run.
fn run_scene(
    command: &str,
    data: Option<exa_assets::GameData>,
    ticks: u64,
    updates: u32,
) -> (Watched, u64) {
    let args: Vec<String> = command.split(' ').map(str::to_owned).collect();
    let (options, view, _) = parse_command_line(args).expect("the command line reads");
    let (mut app, _) = build_app(options, view, data, Shell::Headless).expect("the App builds");
    skip_watch::watch();
    app.insert_resource(skip_watch::WatchSkips);
    app.world_mut()
        .insert_resource(bevy::ecs::error::FallbackErrorHandler(
            skip_watch::frame_handler,
        ));
    app.finish();
    app.cleanup();
    let mut loaded_at = None;
    let mut ran = 0;
    for _ in 0..updates {
        app.update();
        let world = app.world();
        let now = world
            .get_resource::<exa_physics::body::BodyTicks>()
            .map_or(0, |t| t.0);
        if world.contains_resource::<exa_ui::LoadingStatus>() {
            continue;
        }
        let start = *loaded_at.get_or_insert(now);
        ran = now - start;
        if ran >= ticks {
            break;
        }
    }
    let started = app
        .world()
        .get_resource::<exa_physics::sim_bridge::FrameBridge>()
        .is_some_and(|b| b.started());
    assert!(started, "{command}: the simulation bridge never started");
    let seen = skip_watch::seen();
    skip_watch::stop();
    (seen, ran)
}

/// The skips whose parameter is none of `allowed`, one line each.
fn unexpected(skips: &[Skip], allowed: &[&str]) -> Vec<String> {
    skips
        .iter()
        .filter(|s| !allowed.iter().any(|a| s.param.contains(a)))
        .map(|s| {
            format!(
                "{} world: {} not run ({} runs): {} ({})",
                s.world, s.system, s.runs, s.param, s.message
            )
        })
        .collect()
}

fn report(command: &str, seen: &Watched, allowed: &[&str]) -> Vec<String> {
    let mut lines: Vec<String> = seen
        .unbridged
        .iter()
        .map(|t| format!("simulation world: {t} is touched by a moved system and not bridged"))
        .collect();
    lines.extend(unexpected(&seen.skips, allowed));
    for line in &lines {
        eprintln!("{command}: {line}");
    }
    for skip in seen
        .skips
        .iter()
        .filter(|s| allowed.iter().any(|a| s.param.contains(a)))
    {
        eprintln!(
            "{command}: allowed: {} world: {} not run ({} runs): {}",
            skip.world, skip.system, skip.runs, skip.param
        );
    }
    lines
}

#[test]
fn the_game_app_runs_every_system() {
    // The data-free App: every plugin, the bridge started, ticks run.
    let command = "play arena --fixed-frames --no-settings";
    let (seen, ran) = run_scene(command, None, 300, 1500);
    eprintln!(
        "{command}: {ran} ticks, {} systems not run",
        seen.skips.len()
    );
    let allowed: Vec<&str> = HEADLESS.iter().chain(DATA).copied().collect();
    let mut failures = report(command, &seen, &allowed);
    // With the installation: the arena with a duellist and a cast (the
    // casters' walk, the vanish turns, the fight), and the first level.
    if let Some(root) = std::env::var_os("EXANIMA_DIR") {
        for command in [
            "play arena --opponent --powers 0x401 --debug-cast 0@60 --fixed-frames --no-settings",
            "play exanima01 --fixed-frames --no-settings",
        ] {
            let data = exa_assets::GameData::open(&root).expect("the installation opens");
            let (seen, ran) = run_scene(command, Some(data), 300, 3000);
            eprintln!(
                "{command}: {ran} ticks, {} systems not run",
                seen.skips.len()
            );
            assert!(ran >= 300, "{command}: only {ran} ticks after the load");
            failures.extend(report(command, &seen, HEADLESS));
        }
    } else {
        eprintln!("EXANIMA_DIR not set: the level scenes are skipped");
    }
    assert!(
        failures.is_empty(),
        "systems not run for a missing resource or message:\n{}",
        failures.join("\n")
    );
}

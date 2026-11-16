//! A debug tool (ignored by default): runs the game's command line headless
//! (no window, no GPU, the simulation bridge in lockstep) for a number of
//! ticks after the loading screen, so a body trace (`--trace <file>
//! --trace-all --fixed-frames`) can be taken and two builds' traces
//! compared on any machine with the installation, without a graphics card.
//!
//! `EXA_HEADLESS_TRACE="<ticks>|<command line>"`, e.g.
//! `EXA_HEADLESS_TRACE="100|play exanima01 --fixed-frames --no-settings --trace-all --trace /tmp/t.csv"
//! cargo test -p exarust headless_trace -- --ignored --nocapture`.
//! Two builds' traces are compared row by row (bodies by id): the tool found
//! F560's cause in minutes, where the GPU runner takes a build and a load per
//! variant.
//!
//! Also here: the fixed-step order F560 fixed.

use bevy::prelude::*;

use super::{Shell, build_app, parse_command_line};

#[test]
#[ignore = "a debug tool: set EXA_HEADLESS_TRACE and EXANIMA_DIR"]
fn headless_trace() {
    let Ok(spec) = std::env::var("EXA_HEADLESS_TRACE") else {
        eprintln!("EXA_HEADLESS_TRACE not set");
        return;
    };
    let Some(root) = std::env::var_os("EXANIMA_DIR") else {
        eprintln!("EXANIMA_DIR not set");
        return;
    };
    let (ticks, command) = spec.split_once('|').expect("<ticks>|<command line>");
    let ticks: u64 = ticks.trim().parse().expect("a tick count");
    let args: Vec<String> = command.split(' ').map(str::to_owned).collect();
    let (options, view, _) = parse_command_line(args).expect("the command line reads");
    let data = exa_assets::GameData::open(&root).expect("the installation opens");
    let (mut app, _) =
        build_app(options, view, Some(data), Shell::Headless).expect("the App builds");
    app.finish();
    app.cleanup();
    let mut loaded_at = None;
    for _ in 0..(ticks * 4 + 6000) {
        app.update();
        let world = app.world();
        let now = world
            .get_resource::<exa_physics::body::BodyTicks>()
            .map_or(0, |t| t.0);
        if world.contains_resource::<exa_ui::LoadingStatus>() {
            continue;
        }
        let start = *loaded_at.get_or_insert(now);
        if now - start >= ticks {
            eprintln!(
                "headless trace: {} ticks after the load at {start}",
                now - start
            );
            return;
        }
    }
    panic!("{command}: fewer than {ticks} ticks ran");
}

/// The armour pass's head height comes first in the game's fixed step,
/// before the tick deals its blows, its behaviour and character ticks and
/// the grip builder (F560): left unordered, the schedule's layout decided
/// whether a tick's stages saw the raised arm link, and a round that only
/// added systems elsewhere changed the armed characters' bodies.
#[test]
fn the_head_height_comes_first_in_the_fixed_step() {
    use exa_physics::schedule_order::runs_before;
    let args: Vec<String> = "play arena --fixed-frames --no-settings"
        .split(' ')
        .map(str::to_owned)
        .collect();
    let (options, view, _) = parse_command_line(args).expect("the command line reads");
    let (mut app, _) = build_app(options, view, None, Shell::Headless).expect("the App builds");
    for then in [
        "body::deal_shoves",
        "stages::tick_characters",
        "grip_solve::build_grips",
        "body::step_bodies",
    ] {
        assert!(
            runs_before(&mut app, FixedUpdate, "equip::apply_head_height", then),
            "the head height is not ordered before {then}"
        );
    }
}

//! The audio plugin's frame systems run in a headless App: the second
//! wind's heal and the swing voice's looping sources.

use std::sync::Arc;

use bevy::prelude::*;
use exa_assets::{Game, GameData, locate_install};
use exa_combat::Fighter;
use exa_combat::condition::Condition;
use exa_combat::fighter::{HEAL_PITCH_BITS, HEAL_VOLUME_BITS, SecondWindFired};
use exa_combat::swing_voice::SwingVoiceCue;

use crate::AudioEnvironment;
use crate::sounds::HealSound;

fn app() -> App {
    let mut app = App::new();
    // The messages other plugins add in the game.
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<AudioSource>()
        .add_message::<exa_physics::body::Footstep>()
        .add_message::<exa_physics::mechanism_sound::MechanismSounded>()
        .add_plugins(crate::AudioPlugin);
    app
}

/// The next handle the environment would hand out: how many it has made.
fn handles_made(app: &mut App) -> u32 {
    let mut env = app.world_mut().resource_mut::<AudioEnvironment>();
    env.new_handle().index()
}

#[test]
fn a_second_wind_asks_for_its_heal_at_the_fighter() {
    let mut app = app();
    let at = Vec3::new(1.0, 2.0, 3.0);
    let fighter = app
        .world_mut()
        .spawn(GlobalTransform::from_translation(at))
        .id();
    let gone = app.world_mut().spawn_empty().id();
    app.world_mut().despawn(gone);
    for fighter in [fighter, gone] {
        app.world_mut().write_message(SecondWindFired {
            fighter,
            volume_bits: HEAL_VOLUME_BITS,
            pitch_bits: HEAL_PITCH_BITS,
        });
    }
    app.update();
    let heals: Vec<Vec3> = app
        .world()
        .resource::<Messages<HealSound>>()
        .iter_current_update_messages()
        .map(|h| h.position)
        .collect();
    assert_eq!(heals, vec![at]);
}

/// A fighter's first swing reserves its body's pair of looping sources;
/// its death frees them and a dead fighter's swing reserves nothing; a
/// revived fighter's next swing reserves a fresh pair (skipped without
/// `EXANIMA_DIR`: the swing voice loads its set from the game).
#[test]
fn the_looping_sources_follow_life_and_death() {
    let Some(data) = locate_install(None).and_then(|d| GameData::open(&d).ok()) else {
        return;
    };
    let mut app = app();
    app.insert_resource(Game(Arc::new(data)));
    let fighter = app
        .world_mut()
        .spawn((Fighter::default(), Condition::default()))
        .id();
    let swing = |app: &mut App| {
        app.world_mut().write_message(SwingVoiceCue::Start {
            fighter,
            at: Vec3::ZERO,
            pitch: 1.0,
            now_ms: 1000,
            set: exa_combat::swing_voice::SwingSound::PLAIN.set,
        });
        app.update();
    };
    let before = handles_made(&mut app);
    swing(&mut app);
    // The voice and foley handles, then this probe.
    assert_eq!(handles_made(&mut app), before + 3);
    // A second swing keeps the pair.
    swing(&mut app);
    assert_eq!(handles_made(&mut app), before + 4);
    // Dead: the next frame frees them, and a swing reserves nothing.
    app.world_mut().entity_mut(fighter).insert(Condition::DEAD);
    app.update();
    swing(&mut app);
    assert_eq!(handles_made(&mut app), before + 5);
    // Revived: a fresh pair.
    app.world_mut()
        .entity_mut(fighter)
        .insert(Condition::default());
    swing(&mut app);
    assert_eq!(handles_made(&mut app), before + 8);
}

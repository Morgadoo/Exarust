//! `--debug-lights M` (Exarust's debug option): once, two seconds into
//! play, the log lists every point light within M metres of the player
//! with its colour, intensity, range, GI multiplier, vertical shaping and
//! light-source state, and the GI's base ambient and the scene's ambient.
//! It names the level lights that light a spot, as the start room's pool
//! light and fill (`docs/RENDERING.md`, Lights).

use bevy::prelude::*;

use crate::player::PlayerBody;

pub struct LightProbePlugin;

impl Plugin for LightProbePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugLights>()
            .add_systems(Update, log_lights_near_player);
    }
}

/// `--debug-lights`: once, two seconds into play, log every point light
/// within this many metres of the player (Exarust's debug option).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq)]
pub struct DebugLights(pub Option<f32>);

/// Logs the lights around the player once, for `--debug-lights`.
#[allow(clippy::type_complexity)]
fn log_lights_near_player(
    mut debug: ResMut<DebugLights>,
    time: Res<Time>,
    mut since: Local<f32>,
    players: Query<&GlobalTransform, With<PlayerBody>>,
    volume: Option<Res<exa_render::gi::GiVolume>>,
    env: Option<Res<exa_world::environment::SceneEnvRuntime>>,
    lights: Query<(
        Entity,
        Option<&Name>,
        &GlobalTransform,
        &PointLight,
        Option<&exa_render::gi::GiMultiplier>,
        Option<&exa_render::lights::LightShaping>,
        Option<&exa_world::light_source::LightSource>,
    )>,
) {
    let Some(reach) = debug.0 else {
        return;
    };
    let Some(player) = players.iter().next() else {
        return;
    };
    *since += time.delta_secs();
    if *since < 2.0 {
        return;
    }
    debug.0 = None;
    let at = player.translation();
    let mut near: Vec<_> = lights
        .iter()
        .map(|(e, name, t, l, m, sh, src)| {
            (t.translation().distance(at), e, name, t, l, m, sh, src)
        })
        .filter(|(d, ..)| *d <= reach)
        .collect();
    near.sort_by(|a, b| a.0.total_cmp(&b.0));
    info!(
        "debug lights: {} within {reach} m of the player at {at}; GI base ambient per step {:?}, scene ambient {:?} (linear {:?})",
        near.len(),
        volume.map(|v| v.base_ambient),
        env.as_ref().map(|e| e.env.ambient),
        env.as_ref().map(|e| e.env.ambient_linear),
    );
    for (d, e, name, t, l, m, shaping, source) in near {
        let c = l.color.to_linear();
        info!(
            "  {e} {:?} at {} ({d:.2} m): linear colour ({:.3}, {:.3}, {:.3}), intensity {:.3}, range {:.2} m, gi multiplier {:?}, shaping (y, z) {:?}, source {:?}",
            name.map(|n| n.as_str()),
            t.translation(),
            c.red,
            c.green,
            c.blue,
            l.intensity / exa_world::LUMENS_PER_INTENSITY,
            l.range,
            m.map(|m| m.0),
            shaping.map(|s| s.0),
            source,
        );
    }
}

//! Fires: model nodes with a fire emitter (a torch's `_Fire`, a brazier's
//! `_FireM`) become a flickering omni light above them, as the original
//! builds them. Rules: `docs/RENDERING.md` (Fires).

use bevy::prelude::*;

pub struct FirePlugin;

impl Plugin for FirePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, flicker);
    }
}

/// Fire light colour (sRGB) and the template's intensity and radius (cm)
/// before scaling by the square root of the fire's size.
const COLOUR: [f32; 3] = [1.0, 0.68, 0.43];
const SMALL_INTENSITY: f32 = 8.0;
const SMALL_RADIUS_CM: f32 = 1500.0;
const LARGE_INTENSITY: f32 = 12.0;
/// Fires up to this size use the small template, and between these sizes
/// its radius grows by a quarter.
const SMALL_UP_TO: f32 = 0.35;
const SMALL_WIDENED_FROM: f32 = 0.2;
/// Height of the light above the fire node, metres.
const SMALL_LIFT: f32 = 0.05;
const LARGE_LIFT: f32 = 0.40;
/// A fire light's shadow softness before its jitter.
const SOFTNESS: f32 = 2.0;
/// Flicker and particle steps per second.
pub(crate) const TICK_HZ: f32 = 83.0;

/// A burning fire's light.
#[derive(Component, Debug, Clone)]
pub struct Fire {
    /// Intensity before the flicker, in the original's units.
    base: f32,
    /// Soft-bounded random walk (±1) and its velocity.
    walk: f32,
    speed: f32,
    /// How established the fire is: ramps up from 0 when lit.
    density: f32,
    seed: u32,
    accumulator: f32,
    /// Live particles over the most the flame keeps, once it has a flame.
    pub(crate) liveliness: Option<f32>,
    /// Ticks run.
    ticks: u32,
    /// Shadow-softness jitter, 0.87–1.15.
    jitter: f32,
}

impl Fire {
    /// Whether the fire has burnt long enough for its flame and light to
    /// have built up (two seconds).
    pub fn settled(&self) -> bool {
        self.ticks as f32 >= 2.0 * TICK_HZ
    }
}

/// The light a fire of `size` gives: base intensity, radius (m) and its
/// height above the fire node.
pub fn fire_light(size: f32) -> (f32, f32, f32) {
    let s = size.max(0.01);
    if s <= SMALL_UP_TO {
        let mut radius = SMALL_RADIUS_CM * s.sqrt() / 100.0;
        if s > SMALL_WIDENED_FROM {
            radius *= 1.25;
        }
        (SMALL_INTENSITY * s.sqrt(), radius, SMALL_LIFT)
    } else {
        (
            LARGE_INTENSITY * s.sqrt(),
            SMALL_RADIUS_CM * s.sqrt() / 100.0,
            LARGE_LIFT,
        )
    }
}

/// Spawns the light of a fire of `size` under its node.
pub fn spawn_fire(commands: &mut Commands, node: Entity, size: f32) {
    let (base, radius, lift) = fire_light(size);
    let light = exa_formats::rfc::Light {
        flags: 0,
        color: COLOUR,
        multiplier: 1.0,
        intensity: base,
        radius,
        unknown: [0.0; 2],
    };
    commands.spawn((
        Name::new("_FireLight"),
        crate::point_light(&light),
        Transform::from_xyz(0.0, lift, 0.0),
        Fire {
            base,
            walk: 0.0,
            speed: 0.0,
            density: 0.0,
            seed: node.index_u32().wrapping_mul(0x9E37_79B9) | 1,
            accumulator: 0.0,
            liveliness: None,
            ticks: 0,
            jitter: 1.0,
        },
        crate::flame::FlameSize { size, lift },
        ChildOf(node),
    ));
}

pub(crate) fn unit(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(0x0808_8405).wrapping_add(1);
    (*seed as f64 * 2f64.powi(-32)) as f32
}

/// One step of the original's flicker: a soft-bounded random walk drives
/// ±5 % of the intensity while the fire establishes itself.
fn step(fire: &mut Fire) {
    fire.ticks = fire.ticks.saturating_add(1);
    let u = unit(&mut fire.seed);
    if fire.walk > 1.0 || fire.speed > 1.0 {
        fire.speed -= u;
    } else if fire.walk < -1.0 || fire.speed < -1.0 {
        fire.speed += u;
    } else {
        fire.speed += 2.0 * (u - 0.5);
    }
    let u = unit(&mut fire.seed);
    fire.walk += 0.3 * fire.speed * u;
    let u = unit(&mut fire.seed);
    fire.jitter = (fire.jitter + (u - 0.5) * 0.125).clamp(0.87, 1.15);
    // The fire is as established as its flame is full (fully, without one).
    fire.density = 0.8 * fire.density + 0.2 * fire.liveliness.unwrap_or(1.0);
}

fn flicker(time: Res<Time>, mut fires: Query<(&mut Fire, &mut PointLight)>) {
    for (mut fire, mut light) in &mut fires {
        fire.accumulator += time.delta_secs();
        let mut steps = 0;
        while fire.accumulator >= 1.0 / TICK_HZ && steps < 8 {
            step(&mut fire);
            fire.accumulator -= 1.0 / TICK_HZ;
            steps += 1;
        }
        fire.accumulator = fire.accumulator.min(1.0 / TICK_HZ);
        let intensity = fire.density * (1.0 + 0.05 * fire.walk) * fire.base;
        light.intensity = crate::LUMENS_PER_INTENSITY * intensity.max(0.0);
        light.radius = crate::shadow_radius(SOFTNESS * fire.jitter);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_torch_fire() {
        let (base, radius, lift) = fire_light(0.2);
        assert!((base - 3.578).abs() < 1e-3);
        assert!((radius - 6.708).abs() < 1e-3);
        assert_eq!(lift, SMALL_LIFT);
    }

    #[test]
    fn the_flicker_stays_within_bounds() {
        let mut fire = Fire {
            base: 1.0,
            walk: 0.0,
            speed: 0.0,
            density: 0.0,
            seed: 7,
            accumulator: 0.0,
            liveliness: None,
            ticks: 0,
            jitter: 1.0,
        };
        for _ in 0..10_000 {
            step(&mut fire);
            assert!(fire.walk.abs() < 2.5, "{}", fire.walk);
        }
        assert!((fire.density - 1.0).abs() < 1e-3);
    }
}

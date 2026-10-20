//! Level portals: the sparks that replace a portal marker when a level is
//! loaded, and the dissolve burst thrown over a body that steps through one.
//!
//! A portal marker becomes a `_PrtlPrt01` particle system with a soft blue
//! light. Using a portal builds a second `_PrtlPrt01` system over the body,
//! seeds it with `_PrtlDisp` particles across every model node's bounding
//! box, and plays `portaluse01`.
//!
//! Sizes are the original's centimetres. The seeding rules are plain
//! functions; nothing here draws or spawns.

use bevy::prelude::*;

use crate::arena::Mt19937;
use crate::effects::{Frame, TICK_HZ};
use crate::item_container::Rng;

/// `_PrtlPrt01`, the shared particle material of both systems.
pub const PARTICLE_MATERIAL: &str = "_PrtlPrt01";
/// The effect kind a portal marker's record names.
pub const PORTAL_EFFECT_KIND: u32 = 0x3dd0_e004;
/// The geometry's sub-object kind and its flag word.
pub const SUB_OBJECT_KIND: i32 = 35_044;
pub const GEOMETRY_FLAG: u32 = 0x1000;
/// A level node asks for the replacement through this flag.
pub const IMPORT_FLAG: u32 = 0x10000;
/// The names the systems carry.
pub const PORTAL_NODE_NAME: &str = "_PortalPrt";
pub const DISSOLVE_NODE_NAME: &str = "_PrtlDis";
/// The particle system's box keeps this margin past its live particles.
pub const BOUNDS_MARGIN_CM: f32 = 5.0;

/// The sparks a level's portal marker gives off.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParticleSpec {
    /// Particles started per second.
    pub rate_per_second: f32,
    /// Lifetime, seconds.
    pub life_seconds: f32,
    /// Base half size, centimetres.
    pub half_size_cm: f32,
    /// Initial speed, centimetres per second.
    pub speed_cm_per_second: f32,
    /// Growth over the life, a multiple of the half size.
    pub growth: f32,
}

impl ParticleSpec {
    /// Lifetime in simulation ticks: `trunc(83 · seconds)`.
    pub fn lifetime_ticks(self) -> i32 {
        (TICK_HZ as f32 * self.life_seconds).trunc() as i32
    }
}

/// The portal marker's sparks: 140 a second, 1.5 s, 36 cm, 16 cm/s, ×3.
pub const SPARKS: ParticleSpec = ParticleSpec {
    rate_per_second: 140.0,
    life_seconds: 1.5,
    half_size_cm: 36.0,
    speed_cm_per_second: 16.0,
    growth: 3.0,
};

/// The dissolve burst's particles: 60 a second, 0.5 s, 36 cm, 48 cm/s.
pub const DISSOLVE: ParticleSpec = ParticleSpec {
    rate_per_second: 60.0,
    life_seconds: 0.5,
    half_size_cm: 36.0,
    speed_cm_per_second: 48.0,
    growth: 2.0,
};

/// The soft blue light a portal marker adds, 50 cm above its node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightSpec {
    /// sRGB colour.
    pub colour: [f32; 3],
    /// Colour multiplier (the light record's alpha).
    pub multiplier: f32,
    pub intensity: f32,
    /// Radius, centimetres.
    pub radius_cm: f32,
    pub softness: f32,
    /// Height above the marker node, centimetres.
    pub offset_cm: Vec3,
    /// The light kind: 3, a ranged omni.
    pub kind: i32,
    /// Node flags: ranged with shadows.
    pub flags: u32,
    /// The emitter flag word.
    pub emitter_flags: u32,
    /// Whether the light casts shadows.
    pub casts_shadow: bool,
}

/// The portal light: sRGB (0.49, 0.69, 1), 0.75 intensity, 500 cm, softness
/// 3, 50 cm above the marker, kind 3, ranged with shadows.
pub const PORTAL_LIGHT: LightSpec = LightSpec {
    colour: [0.49, 0.69, 1.0],
    multiplier: 2.0,
    intensity: 0.75,
    radius_cm: 500.0,
    softness: 3.0,
    offset_cm: Vec3::new(0.0, 50.0, 0.0),
    kind: 3,
    flags: 0x0034_0100,
    emitter_flags: 0x1000,
    casts_shadow: true,
};

/// Particle systems are started with this scale; the portal's own system
/// also carries a 0.102 start value and clears this render-flag bit.
pub const SYSTEM_SCALE: f32 = 1.0;
pub const SYSTEM_START: f32 = 0.101_960_79;
pub const SYSTEM_CLEAR_FLAG: u32 = 0x0002_0000;

/// The spark system a level's portal marker becomes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortalSpark {
    pub particle: ParticleSpec,
    pub light: LightSpec,
    /// The geometry's sub-object kind and node flag.
    pub sub_object_kind: i32,
    pub geometry_flag: u32,
    pub material: &'static str,
    pub node_name: &'static str,
    /// The system's start scale and start value.
    pub scale: f32,
    pub start: f32,
    /// The particle system flag cleared after creation.
    pub cleared_flags: u32,
}

/// The marker's spark system: `_PrtlPrt01` sparks and the blue light.
pub const PORTAL: PortalSpark = PortalSpark {
    particle: SPARKS,
    light: PORTAL_LIGHT,
    sub_object_kind: SUB_OBJECT_KIND,
    geometry_flag: GEOMETRY_FLAG,
    material: PARTICLE_MATERIAL,
    node_name: PORTAL_NODE_NAME,
    scale: SYSTEM_SCALE,
    start: SYSTEM_START,
    cleared_flags: SYSTEM_CLEAR_FLAG,
};

/// The dissolve a body gets when it uses a portal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortalDissolve {
    pub particle: ParticleSpec,
    pub material: &'static str,
    pub node_name: &'static str,
    /// The pull from the emitter's velocity into a particle's second start.
    pub pull: f32,
    pub sound: &'static str,
    pub volume: f32,
    pub pitch: f32,
}

/// The portal-use dissolve: `_PrtlPrt01` again, `_PrtlDis`-named, with the
/// body's motion and `portaluse01` at 0.6.
pub const DISSOLVE_EFFECT: PortalDissolve = PortalDissolve {
    particle: DISSOLVE,
    material: PARTICLE_MATERIAL,
    node_name: DISSOLVE_NODE_NAME,
    pull: SEED_VELOCITY_PULL,
    sound: USE_SOUND,
    volume: USE_SOUND_VOLUME,
    pitch: USE_SOUND_PITCH,
};

/// The portal-use sound: volume 0.6 at the body, pitch 1.
pub const USE_SOUND: &str = "portaluse01";
pub const USE_SOUND_VOLUME: f32 = 0.6;
pub const USE_SOUND_PITCH: f32 = 1.0;

/// The seeds the burst divides a node's box into: `round(half extent / 6)`.
pub const GRID_STEP_CM: f32 = 6.0;

/// The burst's random side offset, ±1.5 cm.
pub const SEED_JITTER_CM: f32 = 1.5;
/// A guard against an absurd box: no axis is divided into more steps. The
/// original's particle capacity stops it long before this.
pub const MAX_GRID_STEPS: i32 = 1024;

/// The pull the burst applies between a particle's two starts and the
/// emitter's velocity.
pub const SEED_VELOCITY_PULL: f32 = -0.015;

/// A model node the burst is seeded over: its local box, its frame this
/// tick and last (each composed with the node's inverse rest frame by the
/// caller), and its children.
#[derive(Debug, Clone, PartialEq)]
pub struct SeedNode {
    /// Centre of the node's box, in its own space (cm).
    pub centre: Vec3,
    /// Half extents of the node's box (cm).
    pub half: Vec3,
    /// The node's frame this tick.
    pub current: Frame,
    /// The node's frame last tick.
    pub previous: Frame,
    pub children: Vec<SeedNode>,
}

/// One seeded dissolve particle: where it starts, where Verlet thinks it
/// came from, its life in ticks and its atlas/orientation variant.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Seeded {
    pub at: Vec3,
    pub before: Vec3,
    pub life: i32,
    /// The particle's look: cell and symmetry, 0..31.
    pub look: u8,
    /// The ring slot, for the colour parity.
    pub slot: u32,
}

/// The number of grid steps per axis. A negative half extent gives a
/// negative count, whose loop runs no times; a non-finite one counts as
/// zero.
pub fn grid_steps(half_cm: Vec3) -> [i32; 3] {
    [half_cm.x, half_cm.y, half_cm.z].map(|half| {
        let steps = (half / GRID_STEP_CM).round_ties_even();
        if steps.is_finite() {
            (steps as i32).clamp(-MAX_GRID_STEPS, MAX_GRID_STEPS)
        } else {
            0
        }
    })
}

/// Seed the burst over the nodes, in the original's order: each node, then
/// its children in order; x, then y, then z of its grid.
///
/// Two fifths of the cells take a particle. Life starts at the template's
/// lifetime minus up to a quarter of it, the look is the next five random
/// bits, the position is the cell plus ±1.5 cm per axis, and the two starts
/// straddle the node's move over the last tick with the emitter's velocity
/// pulled in by 1.5 %. Seeding stops when the capacity is reached.
#[allow(clippy::too_many_arguments)]
pub fn seed_burst(
    nodes: &[SeedNode],
    velocity_cm: Vec3,
    capacity: usize,
    lifetime_ticks: i32,
    rng: &mut Rng,
    mt: &mut Mt19937,
) -> Vec<Seeded> {
    let mut out = Vec::new();
    if capacity == 0 {
        return out;
    }
    for node in nodes {
        seed_node(
            node,
            velocity_cm,
            capacity,
            lifetime_ticks,
            rng,
            mt,
            &mut out,
        );
        if out.len() == capacity {
            break;
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn seed_node(
    node: &SeedNode,
    velocity_cm: Vec3,
    capacity: usize,
    lifetime_ticks: i32,
    rng: &mut Rng,
    mt: &mut Mt19937,
    out: &mut Vec<Seeded>,
) {
    let steps = grid_steps(node.half);
    let min = node.centre - node.half;
    for x in 0..=steps[0] {
        for y in 0..=steps[1] {
            for z in 0..=steps[2] {
                if rng.below(5) >= 2 {
                    continue;
                }
                let point = min
                    + Vec3::new(
                        node.half.x * (x as f32 / steps[0] as f32) * 2.0,
                        node.half.y * (y as f32 / steps[1] as f32) * 2.0,
                        node.half.z * (z as f32 / steps[2] as f32) * 2.0,
                    );
                let quarter = ((lifetime_ticks as u32) >> 2) as i32;
                let life = lifetime_ticks - rng.below(quarter.max(0) as u32) as i32;
                let look = rng.below(32) as u8;

                let at = node.current.point(point);
                let mut before = node.previous.point(point);
                before += (at - before) * 0.5;
                let spread = f64::from(SEED_JITTER_CM * 2.0);
                let at = at
                    + Vec3::new(
                        ((mt.unit() - 0.5) * spread + f64::from(at.x)) as f32 - at.x,
                        ((mt.unit() - 0.5) * spread + f64::from(at.y)) as f32 - at.y,
                        ((mt.unit() - 0.5) * spread + f64::from(at.z)) as f32 - at.z,
                    );
                let pull = velocity_cm - before;
                let before = before + pull * SEED_VELOCITY_PULL;
                out.push(Seeded {
                    at,
                    before,
                    life,
                    look,
                    slot: out.len() as u32,
                });
                if out.len() == capacity {
                    return;
                }
            }
        }
    }
    for child in &node.children {
        seed_node(child, velocity_cm, capacity, lifetime_ticks, rng, mt, out);
        if out.len() == capacity {
            return;
        }
    }
}

/// Whether a level node's flags ask for its portal marker to be replaced.
pub fn replaces_marker(flags: u32) -> bool {
    flags & IMPORT_FLAG != 0
}

/// The box after a seeding pass: the midpoint of the min and max particles
/// and half extents five centimetres past them. No particles gives none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurstBounds {
    pub centre: Vec3,
    pub half: Vec3,
}

/// The box over the seeded particles.
pub fn burst_bounds(points: &[Vec3]) -> Option<BurstBounds> {
    let first = *points.first()?;
    let (mut min, mut max) = (first, first);
    for &p in points {
        min = min.min(p);
        max = max.max(p);
    }
    Some(BurstBounds {
        centre: (min + max) * 0.5,
        half: (max - min) * 0.5 + Vec3::splat(BOUNDS_MARGIN_CM),
    })
}

/// The light record a portal marker adds, in the engine's metres.
pub fn portal_light() -> exa_formats::rfc::Light {
    exa_formats::rfc::Light {
        flags: PORTAL_LIGHT.flags,
        color: PORTAL_LIGHT.colour,
        multiplier: PORTAL_LIGHT.multiplier,
        intensity: PORTAL_LIGHT.intensity,
        radius: PORTAL_LIGHT.radius_cm / 100.0,
        softness: PORTAL_LIGHT.softness,
        spot: None,
        ranged: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(half: Vec3) -> SeedNode {
        SeedNode {
            centre: Vec3::ZERO,
            half,
            current: Frame::IDENTITY,
            previous: Frame::IDENTITY,
            children: Vec::new(),
        }
    }

    #[test]
    fn a_portal_light_is_blue_and_ranged() {
        let light = portal_light();
        assert_eq!(light.color, [0.49, 0.69, 1.0]);
        assert_eq!(light.multiplier, 2.0);
        assert_eq!(light.intensity, 0.75);
        assert_eq!(light.radius, 5.0);
        assert_eq!(light.softness, 3.0);
        assert!(light.ranged);
        assert_eq!(PORTAL_LIGHT.offset_cm, Vec3::new(0.0, 50.0, 0.0));
        assert_eq!(PORTAL_LIGHT.flags, 0x0034_0100);
        assert_eq!(PORTAL_LIGHT.emitter_flags, 0x1000);
        assert_eq!(PORTAL_EFFECT_KIND, 0x3dd0_e004);
        assert_eq!(SUB_OBJECT_KIND, 35_044);
        assert!(replaces_marker(IMPORT_FLAG));
        assert!(!replaces_marker(0));
        assert_eq!(PORTAL_NODE_NAME, "_PortalPrt");
        assert_eq!(DISSOLVE_NODE_NAME, "_PrtlDis");
        assert_eq!(PORTAL.particle, SPARKS);
        assert_eq!(PORTAL.light, PORTAL_LIGHT);
        assert_eq!(PORTAL.node_name, "_PortalPrt");
        assert_eq!(PORTAL.cleared_flags, 0x0002_0000);
        assert!((PORTAL.start - 0.101_960_79).abs() < 1e-8);
        assert_eq!(DISSOLVE_EFFECT.particle, DISSOLVE);
        assert_eq!(DISSOLVE_EFFECT.pull, -0.015);
        assert_eq!(DISSOLVE_EFFECT.sound, "portaluse01");
        assert_eq!(DISSOLVE_EFFECT.volume, 0.6);
        assert_eq!(DISSOLVE_EFFECT.pitch, 1.0);
    }

    #[test]
    fn a_burst_box_is_the_midpoint_with_a_five_centimetre_margin() {
        assert_eq!(burst_bounds(&[]), None);
        let boxed = burst_bounds(&[
            Vec3::new(-6.0, 0.0, 0.0),
            Vec3::new(6.0, 2.0, 0.0),
            Vec3::new(0.0, 4.0, 0.0),
        ])
        .unwrap();
        assert_eq!(boxed.centre, Vec3::new(0.0, 2.0, 0.0));
        assert_eq!(boxed.half, Vec3::new(11.0, 7.0, 5.0));
    }

    #[test]
    fn the_lifetimes_are_truncated_tick_counts() {
        assert_eq!(SPARKS.lifetime_ticks(), 124);
        assert_eq!(DISSOLVE.lifetime_ticks(), 41);
        assert_eq!(SPARKS.rate_per_second, 140.0);
        assert_eq!(DISSOLVE.rate_per_second, 60.0);
        assert_eq!(DISSOLVE.half_size_cm, 36.0);
        assert_eq!(SPARKS.speed_cm_per_second, 16.0);
        assert_eq!(DISSOLVE.speed_cm_per_second, 48.0);
        assert_eq!(SPARKS.growth, 3.0);
    }

    #[test]
    fn the_grid_divides_a_half_extent_by_six() {
        assert_eq!(grid_steps(Vec3::new(6.0, 0.0, 12.0)), [1, 0, 2]);
        assert_eq!(grid_steps(Vec3::new(3.0, 9.0, 2.9)), [0, 2, 0]);
        assert_eq!(
            grid_steps(Vec3::new(-6.0, f32::NAN, f32::INFINITY)),
            [-1, 0, 0]
        );
    }

    #[test]
    fn a_burst_seeds_over_the_box_and_stops_at_capacity() {
        // Seed 1: seven of the eight cube cells pass the 2-in-5 gate; the
        // first takes life 41 − 8 = 33 and look 27.
        let mut rng = Rng(1);
        let mut mt = Mt19937::new(1);
        let seeded = seed_burst(
            &[node(Vec3::new(6.0, 6.0, 6.0))],
            Vec3::ZERO,
            32,
            41,
            &mut rng,
            &mut mt,
        );
        assert_eq!(seeded.len(), 6);
        let first = seeded[0];
        assert_eq!(first.life, 33);
        assert_eq!(first.look, 6);
        assert_eq!(first.slot, 0);
        // The first cell is the box's (−6, −6, −6) corner, jittered by the
        // first three random units: 0.4170, 0.9972, 0.7203.
        let at = Vec3::new(-6.248_934, -4.508_446, -5.339_026);
        assert!((first.at - at).length() < 1e-5, "{}", first.at);
        // The second start is the unjittered corner pulled by the velocity:
        // −6 + (−0.015)(0 − (−6)) = −6.09.
        assert_eq!(first.before, Vec3::splat(-6.09));
        let seeded = seed_burst(
            &[node(Vec3::new(6.0, 6.0, 6.0))],
            Vec3::ZERO,
            1,
            41,
            &mut Rng(1),
            &mut Mt19937::new(1),
        );
        assert_eq!(seeded.len(), 1);
        assert_eq!(seeded[0].slot, 0);
        assert!(
            seed_burst(
                &[node(Vec3::new(6.0, 6.0, 6.0))],
                Vec3::ZERO,
                0,
                41,
                &mut Rng(1),
                &mut Mt19937::new(1)
            )
            .is_empty()
        );
    }

    #[test]
    fn a_burst_walks_the_children_after_the_parent() {
        let mut parent = node(Vec3::new(6.0, 6.0, 6.0));
        parent.children.push(SeedNode {
            centre: Vec3::new(100.0, 0.0, 0.0),
            half: Vec3::new(6.0, 6.0, 6.0),
            current: Frame::IDENTITY,
            previous: Frame::IDENTITY,
            children: Vec::new(),
        });
        let mut rng = Rng(1);
        let mut mt = Mt19937::new(1);
        let seeded = seed_burst(&[parent], Vec3::ZERO, 32, 41, &mut rng, &mut mt);
        assert_eq!(seeded.len(), 9);
        assert!(seeded[6..].iter().all(|p| p.at.x > 90.0));
    }

    #[test]
    fn a_burst_carries_the_velocity_into_the_second_start() {
        let mut rng = Rng(1);
        let mut mt = Mt19937::new(1);
        let seeded = seed_burst(
            &[node(Vec3::new(6.0, 6.0, 6.0))],
            Vec3::new(10.0, 20.0, 30.0),
            1,
            41,
            &mut rng,
            &mut mt,
        );
        let p = seeded[0];
        // The identity frames put the unjittered start at the corner, and
        // the velocity pulls the second start from there.
        let start = Vec3::new(-6.0, -6.0, -6.0);
        let expected = start + (Vec3::new(10.0, 20.0, 30.0) - start) * SEED_VELOCITY_PULL;
        assert!(
            (p.before - expected).length() < 1e-4,
            "{} {:?}",
            p.before,
            expected
        );
    }
}

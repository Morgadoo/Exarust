//! Level portals: the sparks that replace a portal marker when a level is
//! loaded, and the dissolve burst thrown over a body that steps through one.
//!
//! A portal marker (a plain node ending in the 0x3dd0_e004 emitter record,
//! `_PortalPrt` in `uwportal01`) becomes a `_PrtlPrt01` particle system
//! with a soft blue light 50 cm above it. Using a portal builds a second
//! `_PrtlPrt01` system, named `_PrtlDisp`, seeds it with particles across
//! every model node's bounding box of the body, pushed out from the body's
//! centre, and plays `portaluse01`.
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
/// The world a marker is loaded into replaces it only with this flag set
/// (the game's own scene).
pub const IMPORT_FLAG: u32 = 0x10000;
/// The names the systems carry.
pub const PORTAL_NODE_NAME: &str = "_PortalPrt";
pub const DISSOLVE_NODE_NAME: &str = "_PrtlDisp";
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

/// Particle systems are started with this scale. The portal's own system
/// also stores the marker's emitter kind word (read as a float it would be
/// 0.102; the identical bit pattern says it is the kind) and clears this
/// render-flag bit.
pub const SYSTEM_SCALE: f32 = 1.0;
pub const SYSTEM_KIND_WORD: u32 = PORTAL_EFFECT_KIND;
pub const SYSTEM_CLEAR_FLAG: u32 = 0x0002_0000;
/// A seeding pass clears this bit of the dissolve system's flags.
pub const BURST_CLEAR_FLAG: u32 = 0x2;

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
    /// The system's start scale and the kind word it stores.
    pub scale: f32,
    pub kind_word: u32,
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
    kind_word: SYSTEM_KIND_WORD,
    cleared_flags: SYSTEM_CLEAR_FLAG,
};

/// The dissolve a body gets when it uses a portal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortalDissolve {
    pub particle: ParticleSpec,
    pub material: &'static str,
    pub node_name: &'static str,
    /// The pull toward the body's centre each seeded particle takes; it is
    /// negative, so it pushes the particles out.
    pub pull: f32,
    pub sound: &'static str,
    pub volume: f32,
    pub pitch: f32,
}

/// The portal-use dissolve: `_PrtlPrt01` again, `_PrtlDisp`-named, pushed
/// out from the body's centre, and `portaluse01` at 0.6.
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

/// The pull the burst applies from a particle toward the body's centre:
/// negative, a push of 1.5 % of the distance out.
pub const SEED_VELOCITY_PULL: f32 = -0.015;

/// A model node the burst is seeded over: its local box, its two frames
/// (each composed with the node's inverse rest frame by the caller), and
/// its children.
#[derive(Debug, Clone, PartialEq)]
pub struct SeedNode {
    /// Centre of the node's box, in its own space (cm).
    pub centre: Vec3,
    /// Half extents of the node's box (cm).
    pub half: Vec3,
    /// The node's frame whose point is jittered and pushed into a
    /// particle's position.
    pub current: Frame,
    /// The node's other frame, whose point is the particle's previous
    /// position as it is (guess: which of the node's two stored frames is
    /// this tick's is not settled by this group; the jittered, pushed
    /// point is the position because only that reading moves the
    /// particles outward).
    pub previous: Frame,
    pub children: Vec<SeedNode>,
}

/// One seeded dissolve particle: its position (the record's second point)
/// and its previous position (the first), its life in ticks and its
/// atlas/orientation variant.
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
/// lifetime minus up to a quarter of it and the look is the next five
/// random bits. The cell's point (taken in double precision) through the
/// previous frame is the particle's previous position; through the current
/// frame it is moved halfway to that, jittered by ±1.5 cm per axis and
/// pushed 1.5 % further from `centre_cm`, the body's root node position.
///
/// Seeding stops when the capacity is reached.
#[allow(clippy::too_many_arguments)]
pub fn seed_burst(
    nodes: &[SeedNode],
    centre_cm: Vec3,
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
        seed_node(node, centre_cm, capacity, lifetime_ticks, rng, mt, &mut out);
        if out.len() == capacity {
            break;
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn seed_node(
    node: &SeedNode,
    centre_cm: Vec3,
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
                let cell = |min: f32, half: f32, i: i32, n: i32| {
                    (f64::from(min) + f64::from(half) * (f64::from(i) / f64::from(n)) * 2.0) as f32
                };
                let point = Vec3::new(
                    cell(min.x, node.half.x, x, steps[0]),
                    cell(min.y, node.half.y, y, steps[1]),
                    cell(min.z, node.half.z, z, steps[2]),
                );
                let quarter = (lifetime_ticks as u32) >> 2;
                let life = lifetime_ticks.wrapping_sub(rng.below(quarter) as i32);
                let look = rng.below(32) as u8;

                let before = node.previous.point(point);
                let mut at = node.current.point(point);
                at += (before - at) * 0.5;
                let spread = f64::from(SEED_JITTER_CM * 2.0);
                at.x = ((mt.unit() - 0.5) * spread + f64::from(at.x)) as f32;
                at.y = ((mt.unit() - 0.5) * spread + f64::from(at.y)) as f32;
                at.z = ((mt.unit() - 0.5) * spread + f64::from(at.z)) as f32;
                at += (centre_cm - at) * SEED_VELOCITY_PULL;
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
        seed_node(child, centre_cm, capacity, lifetime_ticks, rng, mt, out);
        if out.len() == capacity {
            return;
        }
    }
}

/// Whether the world a marker is loaded into replaces it: the world's
/// flags carry [`IMPORT_FLAG`]. The marker's 4-byte payload (its size,
/// 100 on `uwportal01`) is read and not used.
pub fn replaces_marker(world_flags: u32) -> bool {
    world_flags & IMPORT_FLAG != 0
}

/// A marker a loaded node is replaced with: its place and the render value
/// copied from the node it replaces.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImportedMarker {
    pub at_cm: Vec3,
    pub render: f32,
    /// The node's 4-byte payload read as a float (the original reads it and
    /// never uses it).
    pub size: f32,
}

/// The import handler: a marker loaded into a world whose flags carry
/// [`IMPORT_FLAG`] (see [`replaces_marker`]) is replaced by a fresh portal
/// marker at its place, which keeps the loaded node's render value. The
/// payload is four bytes, read only then; a shorter slice is refused rather
/// than read past its end.
pub fn imported_marker(
    world_flags: u32,
    payload: &[u8],
    at_cm: Vec3,
    render: f32,
) -> Option<ImportedMarker> {
    if !replaces_marker(world_flags) {
        return None;
    }
    let bytes: [u8; 4] = payload.get(..4)?.try_into().ok()?;
    Some(ImportedMarker {
        at_cm,
        render,
        size: f32::from_le_bytes(bytes),
    })
}

/// A particle system's ring: its per-slot life, look, position and previous
/// position, its live count and capacity, and the template's lifetime. The
/// original writes a slot's life at one array, its look
/// at another and its two points at a third, and stops the seeding pass the
/// moment the ring is full.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ParticleRing {
    pub life: Vec<i32>,
    pub look: Vec<u8>,
    pub at: Vec<Vec3>,
    pub before: Vec<Vec3>,
    /// Live particles: the system's count word.
    pub count: usize,
    pub lifetime_ticks: i32,
}

impl ParticleRing {
    /// A ring of `capacity` empty slots.
    pub fn new(capacity: usize, lifetime_ticks: i32) -> Self {
        Self {
            life: vec![0; capacity],
            look: vec![0; capacity],
            at: vec![Vec3::ZERO; capacity],
            before: vec![Vec3::ZERO; capacity],
            count: 0,
            lifetime_ticks,
        }
    }

    /// How many particles the ring can hold.
    pub fn capacity(&self) -> usize {
        self.life.len()
    }

    pub fn is_full(&self) -> bool {
        self.count == self.capacity()
    }

    /// Writes the seeded particles into the ring's slots, in order, and
    /// sets the live count. The seeding pass stops at the ring's capacity.
    pub fn write(&mut self, seeded: &[Seeded]) {
        self.count = seeded.len().min(self.capacity());
        for (slot, particle) in seeded.iter().take(self.count).enumerate() {
            self.life[slot] = particle.life;
            self.look[slot] = particle.look;
            self.at[slot] = particle.at;
            self.before[slot] = particle.before;
        }
    }
}

/// A burst that has just started: the seeded particles and the system flag
/// word left after the pass.
#[derive(Debug, Clone, PartialEq)]
pub struct BurstStart {
    pub seeded: Vec<Seeded>,
    pub flags_cleared: u32,
}

/// Start a dissolve burst on a body: stop the ring, seed the body's nodes
/// around its root node's place, and clear the burst flag. The original's
/// flush of the finished particle system is the caller's rebuild.
#[allow(clippy::too_many_arguments)]
pub fn start_dissolve(
    nodes: &[SeedNode],
    centre_cm: Vec3,
    ring: &mut ParticleRing,
    rng: &mut Rng,
    mt: &mut Mt19937,
    flags: u32,
) -> BurstStart {
    let seeded = seed_burst(
        nodes,
        centre_cm,
        ring.life.len(),
        ring.lifetime_ticks,
        rng,
        mt,
    );
    ring.write(&seeded);
    BurstStart {
        seeded,
        flags_cleared: flags & !BURST_CLEAR_FLAG,
    }
}

/// The box after a seeding pass: half way from the min to the max particle,
/// half extents five centimetres past the max, and the sphere through the
/// box's corner. No particles gives none.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurstBounds {
    pub centre: Vec3,
    pub half: Vec3,
    pub radius: f32,
}

/// The box over the seeded particles, in the particle system's arithmetic
/// (the min and max themselves come from the system's own pass, outside
/// this group).
pub fn burst_bounds(points: &[Vec3]) -> Option<BurstBounds> {
    let first = *points.first()?;
    let (mut min, mut max) = (first, first);
    for &p in points {
        min = min.min(p);
        max = max.max(p);
    }
    let centre = min + (max - min) * 0.5;
    let half = (max - centre) + Vec3::splat(BOUNDS_MARGIN_CM);
    let square = (half.x * half.x + half.y * half.y) + half.z * half.z;
    Some(BurstBounds {
        centre,
        half,
        radius: if 0.0 < square { square.sqrt() } else { square },
    })
}

/// The portal light's node: a child of the marker 50 cm above it.
#[derive(Component, Debug)]
pub struct PortalLight;

/// A loaded level portal marker gets the portal's blue light, as the
/// original's import handler builds it on the marker's place when the
/// marker is loaded into the game's scene (the game plugin running is that
/// scene). The marker's spark system is not simulated. The light hangs
/// 50 cm up the marker's own axis; the original's system takes only the
/// marker's position, which is the same for an upright marker.
pub fn light_portals(
    mut commands: Commands,
    markers: Query<Entity, Added<exa_world::PortalMarker>>,
) {
    for marker in &markers {
        commands.spawn((
            Name::new(PORTAL_NODE_NAME),
            PortalLight,
            exa_world::light_bundle(&portal_light()),
            Transform::from_translation(PORTAL_LIGHT.offset_cm / 100.0),
            ChildOf(marker),
        ));
    }
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
        animation: None,
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
        assert_eq!(DISSOLVE_NODE_NAME, "_PrtlDisp");
        assert_eq!(PORTAL.particle, SPARKS);
        assert_eq!(PORTAL.light, PORTAL_LIGHT);
        assert_eq!(PORTAL.node_name, "_PortalPrt");
        assert_eq!(PORTAL.cleared_flags, 0x0002_0000);
        assert_eq!(PORTAL.kind_word, 0x3dd0_e004);
        assert_eq!(BURST_CLEAR_FLAG, 0x2);
        assert_eq!(DISSOLVE_EFFECT.particle, DISSOLVE);
        assert_eq!(DISSOLVE_EFFECT.pull, -0.015);
        assert_eq!(DISSOLVE_EFFECT.sound, "portaluse01");
        assert_eq!(DISSOLVE_EFFECT.volume, 0.6);
        assert_eq!(DISSOLVE_EFFECT.pitch, 1.0);
    }

    #[test]
    fn a_loaded_portal_marker_gets_its_light() {
        let mut app = App::new();
        app.add_systems(Update, light_portals);
        let marker = app
            .world_mut()
            .spawn((
                Transform::default(),
                exa_world::PortalMarker { size: 100.0 },
            ))
            .id();
        app.update();
        let mut lights = app
            .world_mut()
            .query_filtered::<(&ChildOf, &Transform, &PointLight), With<PortalLight>>();
        let found: Vec<_> = lights.iter(app.world()).collect();
        assert_eq!(found.len(), 1);
        let (parent, at, light) = found[0];
        assert_eq!(parent.parent(), marker);
        assert_eq!(at.translation, Vec3::new(0.0, 0.5, 0.0));
        assert_eq!(light.intensity, 0.75 * exa_world::LUMENS_PER_INTENSITY);
        // Run again: an old marker gets no second light.
        app.update();
        assert_eq!(lights.iter(app.world()).count(), 1);
    }

    /// A loaded node with the import flag is replaced by a marker at its
    /// place; the payload is read, and a short one is refused.
    #[test]
    fn an_imported_node_becomes_a_marker_at_its_place() {
        let payload = 100.0f32.to_le_bytes();
        assert_eq!(imported_marker(0, &payload, Vec3::X, 1.0), None);
        let marker = imported_marker(IMPORT_FLAG, &payload, Vec3::X, 0.25).unwrap();
        assert_eq!(marker.at_cm, Vec3::X);
        assert_eq!(marker.render, 0.25);
        assert_eq!(marker.size, 100.0);
        assert_eq!(
            imported_marker(IMPORT_FLAG, &payload[..3], Vec3::X, 1.0),
            None
        );
    }

    #[test]
    fn a_dissolve_fills_its_ring_and_clears_the_burst_flag() {
        let mut ring = ParticleRing::new(4, 41);
        assert_eq!((ring.count, ring.capacity()), (0, 4));
        assert!(!ring.is_full());
        let start = start_dissolve(
            &[node(Vec3::new(6.0, 6.0, 6.0))],
            Vec3::ZERO,
            &mut ring,
            &mut Rng(1),
            &mut Mt19937::new(1),
            0xffff_ffff,
        );
        assert_eq!(start.seeded.len(), 4);
        assert_eq!(ring.count, 4);
        assert!(ring.is_full());
        assert_eq!(start.flags_cleared, 0xffff_fffd);
        // The ring holds the seeded particles in order.
        for (slot, particle) in start.seeded.iter().enumerate() {
            assert_eq!(ring.life[slot], particle.life);
            assert_eq!(ring.look[slot], particle.look);
            assert_eq!(ring.at[slot], particle.at);
            assert_eq!(ring.before[slot], particle.before);
        }
        // A smaller ring takes only its capacity.
        let mut small = ParticleRing::new(1, 41);
        let start = start_dissolve(
            &[node(Vec3::splat(6.0))],
            Vec3::ZERO,
            &mut small,
            &mut Rng(1),
            &mut Mt19937::new(1),
            0x2,
        );
        assert_eq!(small.count, 1);
        assert_eq!(start.seeded.len(), 1);
        assert_eq!(start.flags_cleared, 0);
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
        assert_eq!(boxed.radius, (121.0f32 + 49.0 + 25.0).sqrt());
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
        // The first cell is the box's (−6, −6, −6) corner. Its previous
        // position is the corner itself; its position is jittered by the
        // first three random units (0.4170, 0.9972, 0.7203) and pushed 1.5 %
        // further from the centre: x = −6.249 − 0.015 · 6.249.
        assert_eq!(first.before, Vec3::splat(-6.0));
        let at = Vec3::new(-6.342_667_6, -4.576_072, -5.419_111_7);
        assert!((first.at - at).length() < 1e-5, "{}", first.at);
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
    fn a_burst_pushes_its_particles_out_from_the_centre() {
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
        // The previous position stays the corner; the jittered position is
        // pushed away from the centre (10, 20, 30).
        assert_eq!(p.before, Vec3::splat(-6.0));
        let expected = Vec3::new(-6.492_667_7, -4.876_072_4, -5.869_112);
        assert!((p.at - expected).length() < 1e-5, "{} {:?}", p.at, expected);
    }
}

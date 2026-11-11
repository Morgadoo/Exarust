//! Level portals: the sparks that replace a portal marker when a level is
//! loaded, and the dissolve burst thrown over a body that steps through one.
//!
//! - **The marker's sparks** ([`light_portals`]): a portal marker (a plain
//!   node ending in the 0x3dd0_e004 emitter record, `_PortalPrt` in
//!   `uwportal01`) loaded into the game's scene becomes a `_PortalPrt`
//!   particle system at its place, with the portal effect's own spawn on
//!   an upright ellipse (`exa_world::particles::SpawnRule::PortalRing`),
//!   template [`SPARKS`], and a soft blue light 50 cm above it.
//! - **The portal use** ([`PortalUse`], [`use_portals`]): the level exit's
//!   portal bit (the zone exits of the level portals) runs it on the
//!   player before the long transition. It builds a second system,
//!   `_PrtlDisp` (template [`DISSOLVE`]), seeds it over every model node's
//!   box of the body ([`seed_burst`], `Emitter::seed`), pushed out from the
//!   body's centre, and plays `portaluse01` at the body (0.6, pitch 1).
//!   The seeded particles then age and fly as any other particle of the
//!   system; the system keeps its own spawn at its node, which hangs
//!   unplaced under the body's parent (the level's origin) [guess].
//! - Both draw with the built-in `_PrtlPrt01` material: `PrtDot02`, the
//!   blend word 0x101 (ONE, ONE), unlit (`exa_render::flame`, additive).
//!
//! `--debug-portal-use FRAME` runs the portal use on the player at that
//! frame after the load, without a level change (`docs/DEBUGGING.md`).
//!
//! Sizes are the original's centimetres.

use bevy::prelude::*;
use exa_world::flame::Flame;
use exa_world::particles::{Descriptor, Emitter, Key, Keys, LightDesc, NodeFrame, Seed, SpawnRule};

use crate::arena::Mt19937;
use crate::effects::Frame;
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

/// `_PrtlPrt01`'s particle texture (`PrtDot02.rfi`).
pub const PARTICLE_TEXTURE: &str = "PrtDot02";

/// The marker sparks' colour keys: six, blue, opaque in the middle of the
/// life (the RGB is not scaled at load: only the fires' tables are).
const SPARK_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [10, 10, 64, 0],
    [10, 15, 64, 255],
    [15, 25, 80, 255],
    [15, 40, 100, 255],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
];

/// The dissolve's colour keys: six, a bright blue flash early in the life.
const DISSOLVE_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [10, 10, 64, 255],
    [48, 64, 255, 255],
    [15, 25, 64, 255],
    [4, 10, 32, 255],
    [2, 5, 16, 255],
    [0, 0, 0, 0],
];

/// The marker sparks' template: 140 a second for 1.5 s, half size 36 cm
/// (each slot's drawn once between 18 and 36), speed 16 cm/s, the velocity
/// tripling each second, growth 1, stretch 1.5, spread 1, a pool of 2048,
/// flags 0x4400_2B01 (the key colours with the slot parity, 32 looks,
/// random sizes, the wide pitch walk, diamonds), a light of (0.49, 0.69,
/// 1) at 16 over 900 cm (flicker 0.25) and the `Portal01` loop at 0.1.
pub const SPARKS: Descriptor = Descriptor {
    rate: 140,
    life: 1.5,
    half_size: 36.0,
    speed: 16.0,
    retention: 3.0,
    growth: 1.0,
    stretch: 1.5,
    spawn_radius: 0.0,
    spread: 1.0,
    buoyancy: 0.0,
    centring: 0.0,
    pool: 2048,
    flags: 0x4400_2B01,
    keys: Keys {
        count: 6,
        entries: SPARK_KEYS,
    },
    light: LightDesc {
        colour: [0.49, 0.69, 1.0],
        intensity: 16.0,
        radius: 900.0,
        flicker: 0.25,
    },
    sound: Some("Portal01"),
    volume: 0.1,
    density: 0,
    dose: 0.0,
};

/// The dissolve's template: 60 a second for 0.5 s, half size 36 cm, speed
/// 48 cm/s, no velocity loss, growth 1, stretch 2, spread 1, a pool of
/// 2048, flags 0x4000_2B11 (as the sparks' with the one-shot bit and
/// without the wide pitch walk), no light and no sound.
pub const DISSOLVE: Descriptor = Descriptor {
    rate: 60,
    life: 0.5,
    half_size: 36.0,
    speed: 48.0,
    retention: -1.0,
    growth: 1.0,
    stretch: 2.0,
    spawn_radius: 0.0,
    spread: 1.0,
    buoyancy: 0.0,
    centring: 0.0,
    pool: 2048,
    flags: 0x4000_2B11,
    keys: Keys {
        count: 6,
        entries: DISSOLVE_KEYS,
    },
    light: LightDesc::NONE,
    sound: None,
    volume: 0.0,
    density: 0,
    dose: 0.0,
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

/// The spark system a level's portal marker becomes (its template is
/// [`SPARKS`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortalSpark {
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
    light: PORTAL_LIGHT,
    sub_object_kind: SUB_OBJECT_KIND,
    geometry_flag: GEOMETRY_FLAG,
    material: PARTICLE_MATERIAL,
    node_name: PORTAL_NODE_NAME,
    scale: SYSTEM_SCALE,
    kind_word: SYSTEM_KIND_WORD,
    cleared_flags: SYSTEM_CLEAR_FLAG,
};

/// The dissolve a body gets when it uses a portal (its template is
/// [`DISSOLVE`]).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortalDissolve {
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

/// The portal light's node: a child of the spark system 50 cm above it.
#[derive(Component, Debug)]
pub struct PortalLight;

/// A level portal's spark system (`_PortalPrt`), at its marker's place.
#[derive(Component, Debug)]
pub struct PortalSparks;

/// A portal use's dissolve system (`_PrtlDisp`).
#[derive(Component, Debug)]
pub struct PortalDissolveNode;

/// A loaded level portal marker gets the portal's spark system and blue
/// light, as the original's import handler builds them on the marker's
/// place when the marker is loaded into the game's scene (the game plugin
/// running is that scene): the system node at the marker, scale 1, the
/// portal effect's ring spawn, drawn with `_PrtlPrt01`; the light its
/// child 50 cm up. The system's template light is not driven: the light
/// keeps the import's 0.75 over 500 cm [guess: whether the system's frame
/// pass rewrites the light it is handed is not read].
pub fn light_portals(
    mut commands: Commands,
    markers: Query<(Entity, &GlobalTransform), Added<exa_world::PortalMarker>>,
) {
    for (marker, place) in &markers {
        info!(
            "portal: sparks and light on the marker {marker:?} at {:?} m",
            place.translation()
        );
        let mut emitter = Emitter::new(SPARKS, SYSTEM_SCALE, Vec::new());
        emitter.spawn_rule = SpawnRule::PortalRing {
            scale: SYSTEM_SCALE,
        };
        let seed = marker.index_u32().wrapping_mul(0x9E37_79B9) | 1;
        let system = commands
            .spawn((
                Name::new(PORTAL_NODE_NAME),
                PortalSparks,
                Flame::new(emitter, 0.0, PARTICLE_TEXTURE, seed).additive(),
                Transform::IDENTITY,
                ChildOf(marker),
            ))
            .id();
        commands.spawn((
            Name::new(PORTAL_NODE_NAME),
            PortalLight,
            exa_world::light_bundle(&portal_light()),
            Transform::from_translation(PORTAL_LIGHT.offset_cm / 100.0),
            ChildOf(system),
        ));
    }
}

/// Run the portal use on a body: the dissolve burst over it and
/// `portaluse01` ([`use_portals`]). The level exit's portal bit sends it
/// for the player (`crate::flow`'s `Step::PortalUse`).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalUse {
    pub body: Entity,
}

/// The burst's random streams: the engine's linear sequence and the
/// run-time library's twister, process-wide in the original.
#[derive(Resource)]
pub struct PortalUseDraws {
    pub rng: Rng,
    pub mt: Mt19937,
}

impl Default for PortalUseDraws {
    fn default() -> Self {
        Self {
            rng: Rng(1),
            mt: Mt19937::new(5489),
        }
    }
}

/// A body part as a seeded node: its spheres' box in its own space and
/// its frames (cm) now and where the last tick began.
pub fn part_seed_node(part: &exa_physics::body::Part) -> Option<SeedNode> {
    let mut min = Vec3::splat(f32::INFINITY);
    let mut max = Vec3::splat(f32::NEG_INFINITY);
    for &(centre, radius) in part.spheres.iter() {
        min = min.min(centre - Vec3::splat(radius));
        max = max.max(centre + Vec3::splat(radius));
    }
    if !(min.cmple(max).all()) {
        return None;
    }
    let frame = |affine: bevy::math::Affine3A| Frame {
        x: affine.matrix3.x_axis.into(),
        y: affine.matrix3.y_axis.into(),
        z: affine.matrix3.z_axis.into(),
        at: Vec3::from(affine.translation) * 100.0,
    };
    Some(SeedNode {
        centre: (min + max) * 50.0,
        half: (max - min) * 50.0,
        current: frame(part.transform()),
        previous: frame(part.previous_transform()),
        children: Vec::new(),
    })
}

/// The portal use ([`PortalUse`]): a `_PrtlDisp` system from [`DISSOLVE`]
/// at scale 1, seeded over the body's model nodes (one per body part, its
/// spheres' box [guess: the original walks the body's model nodes and
/// their geometry boxes]) around the root part's place, the burst's
/// ring then flying as the system's own; and `portaluse01` at the body's
/// place, volume 0.6, pitch 1, flags 0, its buffer loaded once.
#[allow(clippy::too_many_arguments)]
pub fn use_portals(
    mut commands: Commands,
    mut uses: MessageReader<PortalUse>,
    bodies: Query<(&exa_physics::body::Ragdoll, &GlobalTransform)>,
    mut draws: ResMut<PortalUseDraws>,
    game: Option<Res<exa_assets::Game>>,
    mut bank: Option<ResMut<exa_audio::SoundBank>>,
    mut sources: Option<ResMut<Assets<AudioSource>>>,
    mut env: Option<ResMut<exa_audio::AudioEnvironment>>,
    mut buffer: Local<Option<Option<u32>>>,
) {
    for request in uses.read() {
        let Ok((body, at)) = bodies.get(request.body) else {
            info!("portal use: {:?} has no body", request.body);
            continue;
        };
        let nodes: Vec<SeedNode> = body.parts.iter().filter_map(part_seed_node).collect();
        let centre = body
            .parts
            .first()
            .map_or(at.translation(), |p| Vec3::from(p.transform().translation))
            * 100.0;
        let mut emitter = Emitter::new(DISSOLVE, SYSTEM_SCALE, Vec::new());
        emitter.enter(NodeFrame::default());
        let PortalUseDraws { rng, mt } = &mut *draws;
        let seeded = seed_burst(
            &nodes,
            centre,
            emitter.params.capacity,
            emitter.params.start_life,
            rng,
            mt,
        );
        let seeds: Vec<Seed> = seeded
            .iter()
            .map(|p| Seed {
                at: p.at,
                before: p.before,
                life: p.life,
                variant: p.look,
            })
            .collect();
        let written = emitter.seed(&seeds);
        let seed = request.body.index_u32().wrapping_mul(0x85EB_CA6B) | 1;
        commands.spawn((
            Name::new(DISSOLVE_NODE_NAME),
            PortalDissolveNode,
            exa_world::level_lifecycle::LevelEntity,
            Flame::new(emitter, 0.0, PARTICLE_TEXTURE, seed).additive(),
            Transform::IDENTITY,
        ));
        let place = at.translation();
        let played = match (
            game.as_deref(),
            bank.as_deref_mut(),
            sources.as_deref_mut(),
            env.as_deref_mut(),
        ) {
            (Some(game), Some(bank), Some(sources), Some(env)) => (*buffer
                .get_or_insert_with(|| bank.load_buffer(game, sources, USE_SOUND)))
            .and_then(|b| env.play_buffer(b, place, USE_SOUND_VOLUME, USE_SOUND_PITCH, 0)),
            _ => None,
        };
        info!(
            "portal use: {:?} at {place:?} m: {written} particles over {} nodes, {USE_SOUND} voice {played:?}",
            request.body,
            nodes.len()
        );
    }
}

/// `--debug-portal-use FRAME`: at that frame after the load the player
/// runs the portal use, with no level change.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugPortalUse(pub Option<u32>);

pub fn debug_portal_use(
    script: Res<DebugPortalUse>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    players: Query<Entity, With<crate::player::PlayerBody>>,
    mut uses: MessageWriter<PortalUse>,
    mut frames: Local<u32>,
) {
    let Some(frame) = script.0 else {
        return;
    };
    if loading.is_some() || *frames > frame {
        return;
    }
    *frames += 1;
    if *frames != frame {
        return;
    }
    match players.iter().next() {
        Some(body) => {
            info!("debug portal use: the player at frame {frame}");
            uses.write(PortalUse { body });
        }
        None => info!("debug portal use: no player at frame {frame}"),
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
        assert_eq!(PORTAL.light, PORTAL_LIGHT);
        assert_eq!(PORTAL.node_name, "_PortalPrt");
        assert_eq!(PORTAL.cleared_flags, 0x0002_0000);
        assert_eq!(PORTAL.kind_word, 0x3dd0_e004);
        assert_eq!(DISSOLVE_EFFECT.pull, -0.015);
        assert_eq!(DISSOLVE_EFFECT.sound, "portaluse01");
        assert_eq!(DISSOLVE_EFFECT.volume, 0.6);
        assert_eq!(DISSOLVE_EFFECT.pitch, 1.0);
    }

    #[test]
    fn a_loaded_portal_marker_gets_its_sparks_and_light() {
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
        let mut systems = app
            .world_mut()
            .query_filtered::<(Entity, &ChildOf, &Flame), With<PortalSparks>>();
        let found: Vec<_> = systems.iter(app.world()).collect();
        assert_eq!(found.len(), 1);
        let (system, parent, flame) = found[0];
        assert_eq!(parent.parent(), marker);
        assert_eq!(
            flame.emitter().spawn_rule,
            SpawnRule::PortalRing { scale: 1.0 }
        );
        assert_eq!(flame.emitter().params.capacity, 2048);
        assert_eq!(flame.emitter().params.start_life, 124);
        assert_eq!(flame.emitter().sound_name(), Some("Portal01"));
        let mut lights = app
            .world_mut()
            .query_filtered::<(&ChildOf, &Transform, &PointLight), With<PortalLight>>();
        let found: Vec<_> = lights.iter(app.world()).collect();
        assert_eq!(found.len(), 1);
        let (parent, at, light) = found[0];
        assert_eq!(parent.parent(), system);
        assert_eq!(at.translation, Vec3::new(0.0, 0.5, 0.0));
        assert_eq!(light.intensity, 0.75 * exa_world::LUMENS_PER_INTENSITY);
        // Run again: an old marker gets no second system.
        app.update();
        assert_eq!(lights.iter(app.world()).count(), 1);
    }

    /// The dissolve's template: a one-shot ring of 2048 that lives 41
    /// ticks; its keys flash blue early in the life.
    #[test]
    fn the_dissolve_is_a_one_shot_ring_of_short_lives() {
        let e = Emitter::new(DISSOLVE, 1.0, Vec::new());
        assert_eq!(e.params.start_life, 41);
        assert_eq!(e.params.capacity, 2048);
        assert_eq!(DISSOLVE.flags & exa_world::particles::flags::ONE_SHOT, 0x10);
        assert_eq!(SPARKS.flags & exa_world::particles::flags::ONE_SHOT, 0);
        assert_eq!(DISSOLVE.keys.sample(3.0, false), [48, 64, 255, 255]);
        assert!(!DISSOLVE.light.is_some());
        assert!(SPARKS.light.is_some());
    }

    /// A portal use seeds a dissolve over the player's body parts and
    /// leaves it in the level.
    #[test]
    fn a_portal_use_seeds_a_dissolve_over_the_body() {
        let mut app = App::new();
        app.add_message::<PortalUse>()
            .init_resource::<PortalUseDraws>()
            .add_systems(Update, use_portals);
        use exa_physics::body::{PartSetup, Ragdoll};
        let part = |bone: usize, parent: Option<usize>, y: f32| PartSetup {
            bone,
            entity: Entity::PLACEHOLDER,
            parent,
            world: bevy::math::Affine3A::from_translation(Vec3::new(0.0, y, 0.0)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: Some(vec![(Vec3::ZERO, 0.15), (Vec3::new(0.0, 0.2, 0.0), 0.1)]),
        };
        let ragdoll = Ragdoll::new(&[part(1, None, 1.0), part(2, Some(0), 1.4)]);
        let body = app
            .world_mut()
            .spawn((ragdoll, GlobalTransform::from_translation(Vec3::Y)))
            .id();
        app.world_mut().write_message(PortalUse { body });
        app.update();
        let mut systems = app
            .world_mut()
            .query_filtered::<&Flame, With<PortalDissolveNode>>();
        let flames: Vec<_> = systems.iter(app.world()).collect();
        assert_eq!(flames.len(), 1);
        let live = flames[0].emitter().live();
        assert!(live > 20, "{live}");
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

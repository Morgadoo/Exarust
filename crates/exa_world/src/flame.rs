//! Flames: the particles of a fire, as the original simulates and draws
//! them. Wisps start at rest on a ring around the fire node, rise by
//! buoyancy (Verlet integration at the 83 Hz tick) with a weak pull toward
//! the axis, and die young or once they stray too far. Each is drawn as a
//! diamond stretched along its motion, coloured over its life by the fire's
//! keys, with the `_FirePrt` material (premultiplied blending). The fire's
//! light follows how many are alive. Rules: `docs/RENDERING.md` (Fires).

use std::collections::VecDeque;

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_assets::{ColorSpace, Game, ModelCache, ModelLoader};
use exa_render::flame::FlameMaterial;

use crate::fire::{Fire, TICK_HZ, unit};

pub struct FlamePlugin;

impl Plugin for FlamePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (load_flame_assets, attach_flames, simulate, build_meshes).chain(),
        );
    }
}

/// The small fire's particle texture: a 2 × 2 atlas of wisps.
const SMALL_TEXTURE: &str = "FirePrt02";

/// The shared particle material, once loaded.
#[derive(Resource)]
struct FlameAssets {
    small: Handle<FlameMaterial>,
}

/// The small fire's colour keys over a particle's life (bytes, used as
/// linear; alpha is the particle's opacity). Keys 2–10 are scaled by 0.7
/// at load in the linear pipeline.
const SMALL_KEYS: [[u8; 4]; 11] = [
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [32, 13, 3, 16],
    [64, 23, 5, 32],
    [192, 127, 67, 32],
    [255, 160, 94, 64],
    [170, 120, 60, 64],
    [90, 30, 7, 64],
    [48, 12, 3, 80],
    [0, 0, 0, 48],
    [0, 0, 0, 0],
];
const KEY_SCALE: f32 = 0.7;

/// Gravity per tick², centimetres.
const GRAVITY: f32 = -980.665 / (TICK_HZ * TICK_HZ);
/// A particle at rest still points up (cm per tick).
const REST_LEAN: f32 = 0.25;
/// Ring slots of the emitter (only their parity matters: odd slots run
/// one colour key later).
const SLOTS: usize = 98;

/// The small template's values for a fire of size `s`, in the original's
/// units (centimetres, ticks).
#[derive(Debug, Clone, Copy)]
struct Template {
    /// Particles started per second.
    rate: f32,
    /// Lifetime, ticks.
    life: i32,
    /// Base half-size, and growth over the life (cm).
    half: f32,
    growth: f32,
    /// Stretch along the motion.
    stretch: f32,
    /// Spawn ring radius (cm).
    ring: f32,
    /// Buoyancy, a multiple of gravity (negative rises).
    buoyancy: f32,
    /// Horizontal pull toward the node per tick.
    pull: f32,
    /// Particles stop living this far from the node (cm).
    reach: f32,
    /// Divides the live count for the fire's density.
    full: f32,
}

fn small_template(s: f32) -> Template {
    Template {
        rate: (250.0 * s).round(),
        life: (TICK_HZ * 1.25) as i32,
        half: 10.0 * (0.5 + 0.5 * s),
        growth: 2.0 * 10.0 * (0.5 + 0.5 * s),
        stretch: 0.6,
        ring: 35.0 * (0.5 * s + s * s),
        buoyancy: -1.25 * s,
        pull: 0.0007,
        reach: 140.0 * s,
        full: (450.0 * (0.9 * s * s + 0.1 * s)).round().max(1.0),
    }
}

#[derive(Debug, Clone, Copy)]
struct Particle {
    /// Position and the previous tick's (cm, world).
    p: Vec3,
    q: Vec3,
    /// Ticks left.
    life: i32,
    /// Atlas cell (low 2 bits) and square symmetry (next 3).
    look: u8,
    /// Ring slot parity.
    odd: bool,
}

/// A fire's particles, on the fire's light entity.
#[derive(Component)]
pub struct Flame {
    template: Template,
    particles: VecDeque<Particle>,
    /// Height of the light above the node the particles start from (m).
    lift: f32,
    /// The node's position this tick and last (cm).
    node: Option<(Vec3, Vec3)>,
    /// Ticks until the next particle.
    owed: f32,
    slot: usize,
    seed: u32,
    accumulator: f32,
    mesh: Handle<Mesh>,
}

impl Flame {
    /// Live particles over the template's "full" count (can exceed 1).
    pub fn liveliness(&self) -> f32 {
        self.particles.iter().filter(|p| p.life > 0).count() as f32 / self.template.full
    }
}

/// Size of a fire, and its light's height above the fire node (metres).
#[derive(Component)]
pub struct FlameSize {
    pub size: f32,
    pub lift: f32,
}

#[allow(clippy::too_many_arguments)]
fn load_flame_assets(
    mut commands: Commands,
    loaded: Option<Res<FlameAssets>>,
    game: Option<Res<Game>>,
    mut cache: ResMut<ModelCache>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut flames: ResMut<Assets<FlameMaterial>>,
    support: Option<Res<bevy::image::CompressedImageFormatSupport>>,
) {
    let (None, Some(game)) = (loaded, game) else {
        return;
    };
    let mut loader = ModelLoader {
        game: &game.0,
        cache: &mut cache,
        meshes: &mut meshes,
        materials: &mut materials,
        images: &mut images,
        bc_supported: exa_assets::model::bc_supported(support.as_deref()),
    };
    let Some((texture, _)) = loader.texture(SMALL_TEXTURE, ColorSpace::Srgb) else {
        return;
    };
    commands.insert_resource(FlameAssets {
        small: flames.add(FlameMaterial { texture }),
    });
}

fn new_flame(size: f32, lift: f32, seed: u32, mesh: Handle<Mesh>) -> Flame {
    Flame {
        template: small_template(size),
        particles: VecDeque::new(),
        lift,
        node: None,
        owed: 0.0,
        slot: 0,
        seed,
        accumulator: 0.0,
        mesh,
    }
}

fn attach_flames(
    mut commands: Commands,
    assets: Option<Res<FlameAssets>>,
    fires: Query<(Entity, &FlameSize), Without<Flame>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(assets) = assets else { return };
    for (e, size) in &fires {
        let mesh = meshes.add(Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        ));
        let seed = e.index_u32().wrapping_mul(0x2C1B_3C6D) | 1;
        commands
            .entity(e)
            .insert(new_flame(size.size, size.lift, seed, mesh.clone()));
        // The particles live in world space, under no parent.
        commands.spawn((
            Name::new("_FireParticles"),
            Mesh3d(mesh),
            MeshMaterial3d(assets.small.clone()),
            Transform::IDENTITY,
            NoFrustumCulling,
            bevy::light::NotShadowCaster,
            bevy::light::NotShadowReceiver,
        ));
    }
}

/// One emitter tick at the node's position (cm): age, drop the dead at the
/// head, integrate, emit.
fn tick(flame: &mut Flame, node: Vec3) {
    let t = flame.template;
    let last = flame.node.map_or(node, |(now, _)| now);
    flame.node = Some((node, last));

    // Ageing: an extra tick lost half the time; particles straying far
    // have their life cut short.
    for i in 0..flame.particles.len() {
        let extra = unit(&mut flame.seed) < 0.5;
        let p = &mut flame.particles[i];
        if extra {
            p.life -= 1;
        }
        let e = p.p - node;
        let d = (e.x * e.x + (0.7 * e.y).powi(2) + e.z * e.z).sqrt();
        let cap = ((t.reach - d) * t.life as f32 / t.reach).round() as i32;
        if cap < p.life {
            p.life = cap;
        } else {
            p.life -= 1;
        }
    }
    let mut died = 0;
    while flame.particles.front().is_some_and(|p| p.life <= 0) {
        flame.particles.pop_front();
        died += 1;
    }

    // Verlet integration with buoyancy and a pull toward the axis.
    for p in &mut flame.particles {
        let mut d = p.p - p.q;
        p.q = p.p;
        d.y += GRAVITY * t.buoyancy;
        p.p += d;
        p.p.x += (node.x - p.p.x) * t.pull;
        p.p.z += (node.z - p.p.z) * t.pull;
    }

    // Emission: on average `rate` a second, sooner after a burst of deaths.
    let interval = TICK_HZ / t.rate * (2.0 / 3.0);
    if flame.owed > 0.0 && unit(&mut flame.seed) < 2.0 / 3.0 {
        flame.owed -= 1.0;
    }
    if died > 1 {
        flame.owed -= (died.min(4) - 1) as f32 * interval;
    }
    while flame.owed < 1.0 {
        flame.owed += interval;
        let theta = std::f32::consts::TAU * unit(&mut flame.seed);
        let r = t.ring * (1.0 - unit(&mut flame.seed).powi(4));
        let p = node + Vec3::new(theta.cos(), 0.0, -theta.sin()) * r;
        // Particles inherit 0.3 of the emitter's own motion.
        let q = p - (node - last) * 0.3;
        let look = (unit(&mut flame.seed) * 32.0) as u8 & 31;
        flame.slot = (flame.slot + 1) % SLOTS;
        flame.particles.push_back(Particle {
            p,
            q,
            life: t.life,
            look,
            odd: flame.slot % 2 == 1,
        });
    }
}

fn simulate(time: Res<Time>, mut fires: Query<(&mut Flame, &mut Fire, &GlobalTransform)>) {
    for (mut flame, mut fire, at) in &mut fires {
        flame.accumulator += time.delta_secs();
        let mut steps = 0;
        let dt = 1.0 / TICK_HZ;
        while flame.accumulator >= dt && steps < 8 {
            let node = (at.translation() - Vec3::Y * flame.lift) * 100.0;
            tick(&mut flame, node);
            flame.accumulator -= dt;
            steps += 1;
        }
        flame.accumulator = flame.accumulator.min(dt);
        fire.liveliness = Some(flame.liveliness());
    }
}

/// A particle's colour at life fraction `t` (linear rgba, 0–1): the keys
/// evenly spaced, odd slots one key later.
fn key_colour(t: f32, odd: bool) -> [f32; 4] {
    let key = |i: usize| {
        let k = SMALL_KEYS[i.min(SMALL_KEYS.len() - 1)];
        let scale = if (2..=10).contains(&i) {
            KEY_SCALE
        } else {
            1.0
        };
        let rgb = |c: u8| (c as f32 * scale).trunc();
        [rgb(k[0]), rgb(k[1]), rgb(k[2]), k[3] as f32]
    };
    let f = 9.0 * t.clamp(0.0, 1.0);
    let k = (f as usize).min(8);
    let w = f - k as f32;
    let (a, b) = (key(k + odd as usize), key(k + 1 + odd as usize));
    std::array::from_fn(|c| (a[c] + (b[c] - a[c]) * w) / 255.0)
}

/// The eight symmetries of the unit square.
fn symmetry(n: u8, u: f32, v: f32) -> (f32, f32) {
    let (u, v) = if n & 4 != 0 { (v, u) } else { (u, v) };
    let u = if n & 1 != 0 { 1.0 - u } else { u };
    let v = if n & 2 != 0 { 1.0 - v } else { v };
    (u, v)
}

fn build_meshes(
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    fires: Query<&Flame>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    for flame in &fires {
        let Some(mut mesh) = meshes.get_mut(&flame.mesh) else {
            continue;
        };
        let t = flame.template;
        let alpha = flame.accumulator * TICK_HZ;
        let node = flame.node.map_or(Vec3::ZERO, |(n, _)| n) / 100.0;
        let to_fire = (node - camera.translation()).normalize_or(Vec3::NEG_Z);
        let n = flame.particles.len();
        let mut positions = Vec::with_capacity(n * 4);
        let mut uvs = Vec::with_capacity(n * 4);
        let mut colours = Vec::with_capacity(n * 4);
        let mut normals = Vec::with_capacity(n * 4);
        let mut indices = Vec::with_capacity(n * 6);
        // Oldest first, unsorted, as the original draws its ring.
        for p in flame.particles.iter().filter(|p| p.life > 0) {
            let at = (p.q + (p.p - p.q) * alpha) / 100.0;
            let age = 1.0 - (p.life as f32 - alpha) / t.life as f32;
            let w = (p.p - p.q) + Vec3::Y * REST_LEAN;
            let speed = w.length();
            let d = -w / speed;
            let across = d.cross(to_fire);
            let l = across.length();
            let half = (t.half + l * age * age * t.growth) / 100.0;
            let stretch = (2.0 - 2.0 / (speed + 1.0)) * t.stretch * d.dot(to_fire).abs().acos();
            let x = across.normalize_or(camera.right().as_vec3());
            let y = to_fire.cross(x).normalize_or(camera.up().as_vec3());
            // Point the long axis along the motion.
            let y = if y.dot(w) < 0.0 { -y } else { y };
            let tips = [
                at - y * (1.0 + stretch) * half,
                at - x * half,
                at + x * half,
                at + y * (1.0 + stretch) * half,
            ];
            // Tail tip to head tip runs along the cell's diagonal.
            let corners = [(0.0, 1.0), (1.0, 1.0), (0.0, 0.0), (1.0, 0.0)];
            let cell = p.look & 3;
            let (u0, v0) = ((cell % 2) as f32 * 0.5, (cell / 2) as f32 * 0.5);
            let colour = key_colour(age, p.odd);
            let base = positions.len() as u32;
            for (tip, (cu, cv)) in tips.iter().zip(corners) {
                let (u, v) = symmetry(p.look >> 2, cu, cv);
                positions.push(tip.to_array());
                uvs.push([u0 + u * 0.5, v0 + v * 0.5]);
                colours.push(colour);
                normals.push((-to_fire).to_array());
            }
            indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
        }
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
        mesh.insert_indices(Indices::U32(indices));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_fade_in_and_out_and_odd_slots_run_later() {
        assert_eq!(key_colour(0.0, false), [0.0; 4]);
        assert_eq!(key_colour(1.0, true), [0.0; 4]);
        // Key 5 (255, 160, 94, 64) scaled by 0.7, reached at t = 4/9 by odd
        // slots.
        let c = key_colour(4.0 / 9.0, true);
        assert!((c[0] - 178.0 / 255.0).abs() < 1e-5, "{c:?}");
        assert!((c[3] - 64.0 / 255.0).abs() < 1e-5);
    }

    #[test]
    fn a_torch_flame_rises_and_stays_near_its_node() {
        let mut flame = new_flame(0.2, 0.05, 3, Handle::default());
        assert_eq!(flame.template.rate, 50.0);
        assert!((flame.template.ring - 4.9).abs() < 1e-4);
        for _ in 0..830 {
            tick(&mut flame, Vec3::ZERO);
        }
        let live: Vec<&Particle> = flame.particles.iter().filter(|p| p.life > 0).collect();
        assert!(live.len() > 10 && live.len() < 60, "{}", live.len());
        let top = live.iter().map(|p| p.p.y).fold(0.0, f32::max);
        assert!(top > 15.0 && top < 60.0, "{top}");
        assert!(live.iter().all(|p| p.p.xz().length() < 6.0));
        // Steady density settles a little above full.
        assert!(flame.liveliness() > 0.7 && flame.liveliness() < 1.6);
    }
}

//! Blood sprays from wounds, as the original's droplet jets fly and fall.
//! Rules: `docs/RENDERING.md` (Blood sprays). All lengths are in cm and
//! the simulation runs at the 83 Hz body tick, as the original's does.
//!
//! - **Jets** ([`Jet`]): each bleeding blow (`exa_combat::wounds::Spray`)
//!   starts one jet at the contact point, fixed to the struck body part so
//!   that it rides with the limb, pointing against the striker's swing. Its
//!   frame: the second axis along the jet, the first across it and the
//!   world's +Z, the third across both ([`jet_frame`]). It sends the blow's
//!   droplets, 36·R of them (at most 60), one at each emission, and then
//!   stops; its place is taken by the next jet.
//! - **Emission**: the scene's sprays share one clock. Each tick it drops
//!   by 1 with probability ⅔; while it is below 1 every running jet sends a
//!   droplet and the clock gains (83/40)·⅔ ticks, so a jet sends 40
//!   droplets a second and lasts 0.9·R s (1.5 s at most).
//! - **A droplet** starts at the jet's origin with a velocity in cm/tick:
//!   the jet's frame applied to (±0.25, 0…1, ±0.25) (uniform), times
//!   48/83; plus 0.15 of how the jet's origin moved since its last
//!   droplet; then averaged with the jet's previous droplet's velocity
//!   (the first jet droplet's with twice the jet's direction as the struck
//!   part's own frame sees it, as the original seeds it). It is placed 0.8
//!   of that velocity ahead of the origin, its previous position 1.2 of it
//!   behind (so it moves at twice the velocity), and lives 62 ticks
//!   (0.75 s).
//! - **Each tick**: every droplet ages a tick, the dead are dropped, and
//!   the rest take a Verlet step with a quarter of gravity (they fall),
//!   keeping 0.95 of their speed a second. No droplet touches anything:
//!   they never stain the floor.
//! - **Drawing** ([`droplet_quad`]): a diamond facing the camera, its long
//!   axis along the droplet's motion less 0.25 cm/tick up (so a slow one
//!   hangs down); half-width its slot's size (12 to 24 cm, drawn once per
//!   ring slot) + ℓ·t²·24, ℓ the sine between the motion and the view and t
//!   the droplet's age; stretched along the motion by (2 − 2/(σ + 1))·angle
//!   (σ the motion in cm/tick, the angle between it and the view). Its
//!   alpha runs 0.5 → 1 → 0.5 → 0.25 → 0 over its life
//!   ([`droplet_alpha`]); its UVs are one of the four splats of `BldPrt02`
//!   under one of the eight symmetries of the square ([`droplet_uvs`]).
//!   The material (`exa_render::spray`) multiplies the frame by the texel
//!   mixed toward white by the alpha.
//!
//! [`DebugSprays`] fires test jets from the player's or the character
//! view's body every second (`--debug-spray`, `docs/DEBUGGING.md`).

use std::collections::{HashMap, VecDeque};

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::image::{CompressedImageFormatSupport, ImageAddressMode, ImageSampler};
use bevy::math::Affine3A;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_anim::BodyPart;
use exa_assets::Game;
use exa_assets::texture::ColorSpace;
use exa_combat::CombatSystems;
use exa_combat::wounds::{Spray, jet_droplets, spray_strength};
use exa_physics::body::{BODY_HZ, BodySystems, Ragdoll};
use exa_render::spray::{SPRAY_TEXTURE, SprayMaterial};

use crate::Dressed;
use crate::blood::{DebugWoundTarget, PartFronts, bone_by_name};

pub struct SprayPlugin;

impl Plugin for SprayPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Spray>()
            .init_resource::<BloodSprays>()
            .add_systems(
                PostUpdate,
                (
                    fire_debug_sprays.run_if(resource_exists::<DebugSprays>),
                    start_jets,
                    load_spray_assets
                        .run_if(resource_exists::<Game>)
                        .run_if(not(resource_exists::<SprayAssets>))
                        .run_if(|s: Res<BloodSprays>| !s.jets.is_empty()),
                    draw_sprays.run_if(resource_exists::<SprayAssets>),
                )
                    .chain()
                    // The blows of this frame, the parts and the camera where
                    // this frame puts them.
                    .after(TransformSystems::Propagate)
                    .after(CombatSystems),
            )
            // On the bodies' poses of the tick.
            .add_systems(FixedUpdate, tick_sprays.after(BodySystems));
    }
}

/// Droplets a running jet sends per second.
pub const DROPLETS_PER_SECOND: f32 = 40.0;
/// A droplet's life, seconds (62 ticks).
pub const DROPLET_LIFE: f32 = 0.75;
/// A droplet's largest half-size, and its growth over its life (cm).
pub const DROPLET_SIZE: f32 = 24.0;
pub const DROPLET_GROWTH: f32 = 24.0;
/// A droplet's speed along the jet, at most (cm/s).
pub const DROPLET_SPEED: f32 = 48.0;
/// The share of its speed a droplet keeps each second.
pub const DROPLET_RETENTION: f32 = 0.95;
/// Stretch of the diamond along the motion.
pub const DROPLET_STRETCH: f32 = 1.0;
/// Speed across the jet, as a share of the speed along it, at most.
pub const DROPLET_SPREAD: f32 = 0.25;
/// Droplets fall at this share of gravity; the drawn motion leans down by
/// as many cm/tick.
pub const DROPLET_FALL: f32 = 0.25;
/// Live droplets at most (the original's pool).
pub const DROPLET_SLOTS: usize = 512;
/// The name of the scene's one spray node.
pub const SPRAY_NODE_NAME: &str = "_BloodSpray";
/// The node flag word the spray node takes (added to the scene's effect
/// list, not an ordinary scene node).
pub const SPRAY_NODE_FLAGS: u32 = 0x1000;
/// The built-in material the droplets draw with.
pub const SPRAY_MATERIAL: &str = "_BloodPrt02";
/// The droplet texture in that material's emissive slot.
pub const SPRAY_MATERIAL_TEXTURE: &str = "BldPrt02";
/// The built-in material record's first flag word.
pub const SPRAY_MATERIAL_FLAGS: u32 = 0x0008_0008;
/// The built-in material record's second flag word.
pub const SPRAY_MATERIAL_EXTRA: u32 = 6;
/// The share of the jet's own motion a new droplet takes.
pub const JET_INHERIT: f32 = 0.15;
/// A new droplet's velocity is averaged with the jet's previous droplet's
/// by this much.
pub const JET_CARRY: f32 = 0.5;
/// A new jet's first "previous droplet" velocity: this times its direction
/// in the struck part's frame (cm/tick).
pub const JET_FIRST_CARRY: f32 = 2.0;
/// A new droplet starts this far ahead along its velocity, and its previous
/// position this far behind.
pub const DROPLET_AHEAD: f32 = 0.8;
pub const DROPLET_BEHIND: f32 = 1.2;
/// The droplets' alpha over their life: keys 0–4 evenly spaced, the fifth
/// repeating the last.
pub const DROPLET_ALPHA_KEYS: [u8; 6] = [128, 255, 128, 64, 0, 0];

const CM_PER_METRE: f32 = 100.0;
/// Gravity per tick², cm.
const GRAVITY: f32 = -980.665 / (BODY_HZ * BODY_HZ);
/// Seconds between two debug jets.
const DEBUG_SPRAY_EVERY: f32 = 1.0;
/// Frames a dressed body waits before its first debug jet (as for the
/// debug wounds).
const DEBUG_SPRAY_DELAY: u32 = 3;

/// Ticks between two emissions: (83/40)·⅔ = 1.383.
pub fn emission_interval() -> f32 {
    BODY_HZ / DROPLETS_PER_SECOND * 0.666_666_7
}

/// A droplet's life in ticks: 83 × 0.75 = 62.25, truncated.
pub fn droplet_life() -> i32 {
    (BODY_HZ * DROPLET_LIFE) as i32
}

/// A droplet's largest speed along the jet, cm/tick.
pub fn droplet_speed() -> f32 {
    DROPLET_SPEED / BODY_HZ
}

/// The share of its speed a droplet keeps each tick: 0.95^(1/83).
pub fn droplet_retention() -> f32 {
    (f64::from(DROPLET_RETENTION).ln() / f64::from(BODY_HZ)).exp() as f32
}

/// A droplet's fall per tick², cm (a quarter of gravity).
pub fn droplet_fall() -> f32 {
    GRAVITY * DROPLET_FALL
}

/// A jet's frame in world space (cm): its second axis along `direction`
/// (made unit; zero stays zero), its first across that and the world's +Z,
/// its third across both; its origin at `origin`. A jet along ±Z has no
/// first or third axis, as in the original.
pub fn jet_frame(origin: Vec3, direction: Vec3) -> Affine3A {
    let along = direction.normalize_or_zero();
    let across = along.cross(Vec3::Z).normalize_or_zero();
    let third = across.cross(along);
    Affine3A::from_cols(across.into(), along.into(), third.into(), origin.into())
}

/// A droplet's age from 0 to 1, `alpha` of the way to the next tick, with
/// `life` ticks left.
pub fn droplet_age(life: i32, alpha: f32) -> f32 {
    1.0 - (life as f32 - alpha) / droplet_life() as f32
}

/// A droplet's alpha at age `age`: the keys evenly spaced over its life,
/// interpolated and rounded to a byte, halves to even (0.5, 1, 0.5, 0.25,
/// 0 at ages 0, ¼, ½, ¾, 1).
pub fn droplet_alpha(age: f32) -> f32 {
    let keys = DROPLET_ALPHA_KEYS.len() - 2;
    let f = keys as f32 * age.clamp(0.0, 1.0);
    let k = (f as usize).min(keys - 1);
    let (a, b) = (
        f32::from(DROPLET_ALPHA_KEYS[k]),
        f32::from(DROPLET_ALPHA_KEYS[k + 1]),
    );
    (a + (b - a) * (f - k as f32)).round_ties_even() / 255.0
}

/// The four splats of the droplet texture (2 × 2 cells).
const CELLS: [Vec2; 4] = [
    Vec2::new(0.0, 0.0),
    Vec2::new(0.5, 0.0),
    Vec2::new(0.0, 0.5),
    Vec2::new(0.5, 0.5),
];
/// The eight symmetries of a cell, as the UVs of the tail, the two sides
/// and the head.
const SYMMETRIES: [[[f32; 2]; 4]; 8] = [
    [[0.0, 0.0], [1.0, 0.0], [0.0, 1.0], [1.0, 1.0]],
    [[1.0, 0.0], [0.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
    [[0.0, 1.0], [1.0, 1.0], [0.0, 0.0], [1.0, 0.0]],
    [[1.0, 1.0], [0.0, 1.0], [1.0, 0.0], [0.0, 0.0]],
    [[0.0, 1.0], [0.0, 0.0], [1.0, 1.0], [1.0, 0.0]],
    [[1.0, 1.0], [1.0, 0.0], [0.0, 1.0], [0.0, 0.0]],
    [[0.0, 0.0], [0.0, 1.0], [1.0, 0.0], [1.0, 1.0]],
    [[1.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 1.0]],
];

/// The UVs of a droplet's tail, sides and head for its look (0–31): the
/// splat `look / 8` under the symmetry `look % 8`.
pub fn droplet_uvs(look: u8) -> [Vec2; 4] {
    let cell = CELLS[usize::from(look >> 3) & 3];
    SYMMETRIES[usize::from(look & 7)].map(|[u, v]| cell + Vec2::new(u, v) * 0.5)
}

/// A droplet of a blood spray (cm, world).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Droplet {
    /// Where it is, and where it was a tick ago.
    pub p: Vec3,
    pub q: Vec3,
    /// Ticks left.
    pub life: i32,
    /// Its splat and symmetry ([`droplet_uvs`]).
    pub look: u8,
    /// Its ring slot, whose size it is drawn at.
    pub slot: u16,
}

/// A droplet's diamond: its tail, its two sides and its head (cm, world),
/// for a droplet of half-size `size` seen along `view` (unit), `alpha` of
/// the way to the next tick. A droplet seen exactly along its motion has
/// no width.
pub fn droplet_quad(d: &Droplet, size: f32, alpha: f32, view: Vec3) -> [Vec3; 4] {
    let moved = d.p - d.q;
    let at = d.q + moved * alpha;
    let w = moved - Vec3::Y * DROPLET_FALL;
    let speed = w.length();
    let dir = -w.normalize_or_zero();
    let across = dir.cross(view);
    let sine = across.length();
    let age = droplet_age(d.life, alpha);
    let half = size + sine * age * age * DROPLET_GROWTH;
    let stretch =
        (2.0 - 2.0 / (speed + 1.0)) * DROPLET_STRETCH * dir.dot(view).abs().min(1.0).acos();
    let side = across.normalize_or_zero();
    let x = side * half;
    let y = side.cross(view) * half * (1.0 + stretch);
    [at - y, at + x, at - x, at + y]
}

/// A jet of a blood spray.
#[derive(Debug, Clone, PartialEq)]
pub struct Jet {
    /// The struck fighter's body model root, and the struck part.
    pub body: Entity,
    pub part: Entity,
    /// The jet's frame in the part's (cm).
    pub local: Affine3A,
    /// Where its origin was at its last emission (cm, world), and how it
    /// moved then.
    pub last: Vec3,
    pub motion: Vec3,
    /// Its last droplet's velocity (cm/tick).
    pub carry: Vec3,
    /// Droplets still to send.
    pub left: i32,
}

impl Jet {
    /// A jet of `droplets` droplets from `contact` (cm, world) along
    /// `direction`, on the part `part` of `body` whose frame is `frame`
    /// (cm, rotation and translation).
    pub fn new(
        body: Entity,
        part: Entity,
        frame: Affine3A,
        contact: Vec3,
        direction: Vec3,
        droplets: i32,
    ) -> Self {
        let local = frame.inverse() * jet_frame(contact, direction);
        Self {
            body,
            part,
            local,
            last: contact,
            motion: Vec3::ZERO,
            // The original seeds it from the jet's axis already turned into
            // the part's frame.
            carry: Vec3::from(local.matrix3.y_axis) * JET_FIRST_CARRY,
            left: droplets,
        }
    }
}

/// The scene's spray node description: its name, material, texture and the
/// flags the original gives the node and its built-in material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SprayNode {
    pub name: &'static str,
    pub material: &'static str,
    pub texture: &'static str,
    /// The node's flag word (`0x1000`: in the scene's effect list).
    pub flags: u32,
    /// The built-in material record's first flag word.
    pub material_flags: u32,
    /// Its second flag word.
    pub material_extra: u32,
}

/// The scene's blood sprays (the original's one `_BloodSpray` system): its
/// jets, its droplets oldest first, the emission clock and the sizes of
/// its ring slots.
#[derive(Resource, Debug, Clone)]
pub struct BloodSprays {
    pub jets: Vec<Jet>,
    pub droplets: VecDeque<Droplet>,
    /// Jets started since the start (for the scripted pauses).
    pub started: u64,
    /// Ticks owed until the next emission.
    owed: f32,
    seed: u32,
    /// Each ring slot's half-size, drawn once (cm).
    sizes: Vec<f32>,
    /// The next ring slot.
    slot: usize,
}

impl Default for BloodSprays {
    fn default() -> Self {
        Self::new(1)
    }
}

impl BloodSprays {
    /// Empty sprays whose random numbers start from `seed`; each of the
    /// 513 ring slots gets a half-size of 12 to 24 cm.
    pub fn new(seed: u32) -> Self {
        let mut sprays = Self {
            jets: Vec::new(),
            droplets: VecDeque::new(),
            started: 0,
            owed: 0.0,
            seed,
            sizes: Vec::new(),
            slot: 0,
        };
        sprays.sizes = (0..=DROPLET_SLOTS)
            .map(|_| DROPLET_SIZE * (sprays.unit() * 0.5 + 0.5))
            .collect();
        sprays
    }

    /// The half-size a droplet is drawn at (cm).
    pub fn size(&self, d: &Droplet) -> f32 {
        self.sizes[usize::from(d.slot) % self.sizes.len()]
    }

    /// Starts a jet in the first finished jet's place, or a new one.
    pub fn start(&mut self, jet: Jet) {
        self.started += 1;
        match self.jets.iter_mut().find(|j| j.left < 1) {
            Some(free) => *free = jet,
            None => self.jets.push(jet),
        }
    }

    /// The scene node the original builds once for the sprays: a geometry
    /// node named `_BloodSpray` with the built-in `_BloodPrt02` material,
    /// flagged into the scene's effect list.
    pub fn scene_node() -> SprayNode {
        SprayNode {
            name: SPRAY_NODE_NAME,
            material: SPRAY_MATERIAL,
            texture: SPRAY_MATERIAL_TEXTURE,
            flags: SPRAY_NODE_FLAGS,
            material_flags: SPRAY_MATERIAL_FLAGS,
            material_extra: SPRAY_MATERIAL_EXTRA,
        }
    }

    /// The original's integer generator: the next 32-bit state.
    fn next(&mut self) -> u32 {
        self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
        self.seed
    }

    /// A whole number from 0 to `n` − 1.
    fn below(&mut self, n: u32) -> u32 {
        ((u64::from(self.next()) * u64::from(n)) >> 32) as u32
    }

    /// A number from 0 to 1.
    fn unit(&mut self) -> f32 {
        (f64::from(self.next()) * 2f64.powi(-32)) as f32
    }

    /// One tick: droplets age, the dead are dropped, the rest take their
    /// step, and the running jets emit. `frames` holds each jet's part
    /// frame this tick (cm, rotation and translation); a jet whose part is
    /// gone stops.
    pub fn tick(&mut self, frames: &[Option<Affine3A>]) {
        for d in &mut self.droplets {
            if d.life > 0 {
                d.life -= 1;
            }
        }
        while self.droplets.front().is_some_and(|d| d.life < 1) {
            self.droplets.pop_front();
        }
        let (fall, keep) = (droplet_fall(), droplet_retention());
        for d in &mut self.droplets {
            let mut step = d.p - d.q;
            d.q = d.p;
            step.y += fall;
            d.p += step * keep;
        }
        if self.owed > 0.0 && self.below(3) != 0 {
            self.owed -= 1.0;
        }
        let interval = emission_interval();
        while self.owed < 1.0 {
            for i in 0..self.jets.len() {
                if self.jets[i].left < 1 {
                    continue;
                }
                let Some(frame) = frames.get(i).copied().flatten() else {
                    self.jets[i].left = 0;
                    continue;
                };
                let world = frame * self.jets[i].local;
                let origin = Vec3::from(world.translation);
                let jet = &mut self.jets[i];
                jet.motion = origin - jet.last;
                jet.last = origin;
                self.emit(i, &world);
                self.jets[i].left -= 1;
            }
            self.owed += interval;
        }
    }

    /// Jet `i`, whose frame is `world` now, sends a droplet.
    fn emit(&mut self, i: usize, world: &Affine3A) {
        // The original's ring would overrun; Exarust drops the droplet.
        if self.droplets.len() >= DROPLET_SLOTS {
            return;
        }
        let slot = self.slot;
        self.slot = (self.slot + 1) % (DROPLET_SLOTS + 1);
        let look = self.below(32) as u8;
        let across = DROPLET_SPREAD * 0.001 * (self.below(2001) as f32 - 1000.0);
        let along = self.below(1001) as f32 * 0.001;
        let third = DROPLET_SPREAD * 0.001 * (self.below(2001) as f32 - 1000.0);
        let m = world.matrix3;
        let mut v = (Vec3::from(m.x_axis) * across
            + Vec3::from(m.y_axis) * along
            + Vec3::from(m.z_axis) * third)
            * droplet_speed();
        let jet = &mut self.jets[i];
        v += jet.motion * JET_INHERIT;
        v = v.lerp(jet.carry, JET_CARRY);
        jet.carry = v;
        let origin = Vec3::from(world.translation);
        self.droplets.push_back(Droplet {
            p: origin + v * DROPLET_AHEAD,
            q: origin - v * DROPLET_BEHIND,
            life: droplet_life(),
            look,
            slot: slot as u16,
        });
    }
}

/// A part's frame for the sprays (cm, its rotation and translation): where
/// the body's last tick left it, or (a body without one) where this frame
/// draws it.
fn part_frame(
    body: Entity,
    part: Entity,
    bodies: &Query<&Ragdoll>,
    transforms: &Query<&GlobalTransform>,
) -> Option<Affine3A> {
    let ticked = bodies
        .get(body)
        .ok()
        .and_then(|r| r.parts.iter().find(|p| p.entity == part))
        .map(|p| p.drawn(1.0));
    let affine = ticked.or_else(|| transforms.get(part).ok().map(GlobalTransform::affine))?;
    let (_, rotation, translation) = affine.to_scale_rotation_translation();
    Some(Affine3A::from_rotation_translation(
        rotation,
        translation * CM_PER_METRE,
    ))
}

/// The body part entity of `body` with bone `bone`.
fn find_part(
    body: Entity,
    bone: usize,
    children: &Query<&Children>,
    parts: &Query<&BodyPart>,
) -> Option<Entity> {
    children
        .iter_descendants(body)
        .find(|&e| parts.get(e).is_ok_and(|p| p.bone == bone))
}

/// Starts a jet for each bleeding blow of the frame, on the struck part.
fn start_jets(
    mut sprays: MessageReader<Spray>,
    mut state: ResMut<BloodSprays>,
    children: Query<&Children>,
    parts: Query<&BodyPart>,
    bodies: Query<&Ragdoll>,
    transforms: Query<&GlobalTransform>,
) {
    for spray in sprays.read() {
        let Some(part) = find_part(spray.body, spray.bone, &children, &parts) else {
            debug!(
                "blood jet: {:?} has no part with bone {}",
                spray.body, spray.bone
            );
            continue;
        };
        let Some(frame) = part_frame(spray.body, part, &bodies, &transforms) else {
            continue;
        };
        info!(
            "blood jet on {:?} bone {}: {} droplets, against a swing of {:.1} m/s",
            spray.body,
            spray.bone,
            spray.droplets,
            spray.swing.length()
        );
        state.start(Jet::new(
            spray.body,
            part,
            frame,
            spray.contact * CM_PER_METRE,
            -spray.swing,
            spray.droplets,
        ));
    }
}

/// One tick of the sprays, on the parts' poses of the tick. The emission
/// clock runs on with no jet, as the original's does.
fn tick_sprays(
    mut sprays: ResMut<BloodSprays>,
    bodies: Query<&Ragdoll>,
    transforms: Query<&GlobalTransform>,
) {
    let frames: Vec<Option<Affine3A>> = sprays
        .jets
        .iter()
        .map(|j| {
            (j.left > 0)
                .then(|| part_frame(j.body, j.part, &bodies, &transforms))
                .flatten()
        })
        .collect();
    sprays.tick(&frames);
}

/// The droplet material once loaded, and the sprays' mesh and the entity
/// drawing it from the first droplet on.
#[derive(Resource)]
struct SprayAssets {
    material: Handle<SprayMaterial>,
    drawn: Option<(Handle<Mesh>, Entity)>,
}

/// Loads the droplet texture the first time a jet starts.
fn load_spray_assets(
    mut commands: Commands,
    game: Res<Game>,
    support: Option<Res<CompressedImageFormatSupport>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<SprayMaterial>>,
    mut tried: Local<bool>,
) {
    if std::mem::replace(&mut *tried, true) {
        return;
    }
    let mut image = match game.load_texture(SPRAY_TEXTURE, ColorSpace::Srgb, support.as_deref()) {
        Ok(image) => image,
        Err(e) => {
            warn!("blood sprays: {e}");
            return;
        }
    };
    // Clamped on both axes: neither the material's load flags nor the
    // file's carry a wrap bit.
    if let ImageSampler::Descriptor(sampler) = &mut image.sampler {
        sampler.address_mode_u = ImageAddressMode::ClampToEdge;
        sampler.address_mode_v = ImageAddressMode::ClampToEdge;
    }
    commands.insert_resource(SprayAssets {
        material: materials.add(SprayMaterial {
            texture: images.add(image),
        }),
        drawn: None,
    });
}

/// Builds the droplets' diamonds for the camera, between the last two
/// ticks, oldest first, into one mesh in world space under no parent;
/// hides it while no droplet flies.
fn draw_sprays(
    mut commands: Commands,
    sprays: Res<BloodSprays>,
    mut assets: ResMut<SprayAssets>,
    time: Res<Time<Fixed>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    mut visibility: Query<&mut Visibility>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    let flying = sprays.droplets.iter().any(|d| d.life > 0);
    if let Some((_, entity)) = assets.drawn {
        match visibility.get_mut(entity) {
            Ok(mut shown) => {
                shown.set_if_neq(if flying {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                });
            }
            // Despawned with a scene: spawned again with the next droplets.
            Err(_) => assets.drawn = None,
        }
    }
    let Some(camera) = cameras.iter().next() else {
        return;
    };
    if !flying {
        return;
    }
    let alpha = time.overstep_fraction();
    let view = camera.forward().as_vec3();
    let n = sprays.droplets.len();
    let mut positions = Vec::with_capacity(n * 4);
    let mut uvs = Vec::with_capacity(n * 4);
    let mut colours = Vec::with_capacity(n * 4);
    let mut normals = Vec::with_capacity(n * 4);
    let mut indices = Vec::with_capacity(n * 6);
    for d in sprays.droplets.iter().filter(|d| d.life > 0) {
        let tips = droplet_quad(d, sprays.size(d), alpha, view);
        let colour = [1.0, 1.0, 1.0, droplet_alpha(droplet_age(d.life, alpha))];
        let base = positions.len() as u32;
        for (tip, uv) in tips.iter().zip(droplet_uvs(d.look)) {
            positions.push((*tip / CM_PER_METRE).to_array());
            uvs.push(uv.to_array());
            colours.push(colour);
            normals.push((-view).to_array());
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }
    let fill = |mesh: &mut Mesh| {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
        mesh.insert_indices(Indices::U32(indices));
    };
    match &assets.drawn {
        Some((handle, _)) => {
            if let Some(mut mesh) = meshes.get_mut(handle) {
                fill(&mut mesh);
            }
        }
        None => {
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
            );
            fill(&mut mesh);
            let handle = meshes.add(mesh);
            let entity = commands
                .spawn((
                    Name::new("_BloodSpray"),
                    Mesh3d(handle.clone()),
                    MeshMaterial3d(assets.material.clone()),
                    Transform::IDENTITY,
                    NoFrustumCulling,
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ))
                .id();
            assets.drawn = Some((handle, entity));
        }
    }
}

/// Test jets fired from every [`DebugWoundTarget`] body every second
/// (`--debug-spray`).
#[derive(Resource, Debug, Clone, Default)]
pub struct DebugSprays(pub Vec<DebugSpray>);

/// A test jet: from body part `bone`, at the front of its meshes (the
/// vertex furthest along the body's facing), as a bleeding blow of size
/// `size` (R) whose striking point moved along `swing` in the body's own
/// frame (+Y up, +Z the way the body faces) would spray.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebugSpray {
    pub bone: usize,
    pub size: f32,
    pub swing: Vec3,
}

impl DebugSpray {
    /// Reads `bone,R[,dx,dy,dz]`: the bone by number (1–18) or by its body
    /// part's name, the wound's size R (over 0), and the swing (any length;
    /// 0,0,−1 by default: a blow driving into the body's front, so the jet
    /// sprays out of it).
    pub fn parse(text: &str) -> Option<Self> {
        let mut fields = text.split(',').map(str::trim);
        let bone = bone_by_name(fields.next()?)?;
        let numbers: Vec<f32> = fields.map(|f| f.parse().ok()).collect::<Option<_>>()?;
        let (size, swing) = match numbers.as_slice() {
            [r] => (*r, Vec3::NEG_Z),
            [r, x, y, z] => (*r, Vec3::new(*x, *y, *z)),
            _ => return None,
        };
        (size > 0.0 && swing != Vec3::ZERO).then_some(Self { bone, size, swing })
    }
}

/// Sends the test jets of each marked body once it is dressed and bound,
/// and again every second.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn fire_debug_sprays(
    sprays: Res<DebugSprays>,
    time: Res<Time>,
    bodies: Query<(Entity, &GlobalTransform), (With<DebugWoundTarget>, With<Dressed>)>,
    fronts: PartFronts,
    mut waiting: Local<HashMap<Entity, u32>>,
    mut due: Local<HashMap<Entity, f32>>,
    mut out: MessageWriter<Spray>,
) {
    let now = time.elapsed_secs();
    for (body, body_transform) in &bodies {
        if !fronts.bound(body) {
            continue;
        }
        let frames = waiting.entry(body).or_default();
        if *frames < DEBUG_SPRAY_DELAY {
            *frames += 1;
            continue;
        }
        let next = due.entry(body).or_insert(now);
        if now < *next {
            continue;
        }
        *next = now + DEBUG_SPRAY_EVERY;
        let facing = body_transform.rotation() * Vec3::Z;
        for spray in &sprays.0 {
            let Some(contact) = fronts.front(body, spray.bone, facing) else {
                warn!(
                    "debug spray: body {body:?} has no visible mesh on bone {}",
                    spray.bone
                );
                continue;
            };
            out.write(Spray {
                body,
                bone: spray.bone,
                contact,
                swing: body_transform.rotation() * spray.swing,
                droplets: jet_droplets(spray_strength(spray.size)),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32, eps: f32) -> bool {
        (a - b).abs() < eps
    }

    /// The template at the 83 Hz tick: an emission every 1.3833 ticks,
    /// droplets living 62 ticks, at most 0.5783 cm/tick along the jet,
    /// keeping 0.999 382 2 of their speed a tick and falling 0.035 588
    /// cm/tick² (a quarter of 0.142 35).
    #[test]
    fn the_template_in_ticks() {
        assert!(close(emission_interval(), 1.383_333_4, 1e-6));
        assert_eq!(droplet_life(), 62);
        assert!(close(droplet_speed(), 0.578_313, 1e-6));
        assert!(close(droplet_retention(), 0.999_382_2, 1e-7));
        assert!(close(droplet_retention().powf(83.0), 0.95, 1e-5));
        assert!(close(droplet_fall(), -0.035_588, 1e-6));
    }

    /// A droplet's alpha runs 128, 255, 128, 64, 0 at ages 0, ¼, ½, ¾, 1,
    /// interpolated and rounded with halves to even: 178.8 → 179 at 0.1,
    /// 191.5 → 192 at ⅛.
    #[test]
    fn droplets_fade_in_and_out() {
        let byte = |age: f32| (droplet_alpha(age) * 255.0).round() as u8;
        assert_eq!(byte(0.0), 128);
        assert_eq!(byte(0.1), 179);
        assert_eq!(byte(0.125), 192);
        assert_eq!(byte(0.25), 255);
        assert_eq!(byte(0.5), 128);
        assert_eq!(byte(0.75), 64);
        assert_eq!(byte(0.875), 32);
        assert_eq!(byte(1.0), 0);
        // Born this tick, a sixty-second of the way to the next.
        assert!(close(droplet_age(62, 0.5), 0.5 / 62.0, 1e-6));
        assert!(close(droplet_age(1, 1.0), 1.0, 1e-6));
    }

    /// The jet's frame: along the jet, across it and +Z, across both; a jet
    /// along ±Z has only its axis.
    #[test]
    fn the_jet_frame() {
        let f = jet_frame(Vec3::new(1.0, 2.0, 3.0), Vec3::new(-4.0, 0.0, 0.0));
        assert_eq!(Vec3::from(f.matrix3.y_axis), Vec3::NEG_X);
        assert_eq!(Vec3::from(f.matrix3.x_axis), Vec3::Y);
        assert_eq!(Vec3::from(f.matrix3.z_axis), Vec3::Z);
        assert_eq!(Vec3::from(f.translation), Vec3::new(1.0, 2.0, 3.0));
        let down = jet_frame(Vec3::ZERO, Vec3::NEG_Y);
        assert_eq!(Vec3::from(down.matrix3.x_axis), Vec3::NEG_X);
        assert_eq!(Vec3::from(down.matrix3.z_axis), Vec3::Z);
        let along_z = jet_frame(Vec3::ZERO, Vec3::Z);
        assert_eq!(Vec3::from(along_z.matrix3.x_axis), Vec3::ZERO);
        assert_eq!(Vec3::from(along_z.matrix3.z_axis), Vec3::ZERO);
    }

    fn jet(frame: Affine3A, contact: Vec3, direction: Vec3, droplets: i32) -> Jet {
        Jet::new(
            Entity::PLACEHOLDER,
            Entity::PLACEHOLDER,
            frame,
            contact,
            direction,
            droplets,
        )
    }

    /// The chest cut's jet (16 droplets) sends exactly 16, one an emission,
    /// in about 33 ticks (40 a second, 0.4 s); each droplet lives 62 ticks,
    /// so the last is gone some 95 ticks after the blow. A 60-droplet jet
    /// runs about 125 ticks (1.5 s).
    #[test]
    fn a_jet_sends_its_droplets_at_forty_a_second() {
        let mut sprays = BloodSprays::new(7);
        let frame = Affine3A::from_translation(Vec3::new(0.0, 130.0, 0.0));
        sprays.start(jet(frame, Vec3::new(0.0, 140.0, 20.0), Vec3::Z, 16));
        let mut sent = 0;
        let mut ticks_to_last = 0;
        for t in 1..=200 {
            let before = sprays.droplets.len();
            sprays.tick(&[Some(frame)]);
            let new = sprays.droplets.iter().filter(|d| d.life == 62).count();
            sent += new;
            if new > 0 {
                ticks_to_last = t;
            }
            assert!(sprays.droplets.len() <= before + new);
        }
        assert_eq!(sent, 16);
        assert_eq!(sprays.jets[0].left, 0);
        assert!((20..=50).contains(&ticks_to_last), "{ticks_to_last}");
        assert!(sprays.droplets.is_empty());

        let mut long = BloodSprays::new(3);
        long.start(jet(frame, Vec3::ZERO, Vec3::X, 60));
        let mut ticks = 0;
        while long.jets[0].left > 0 {
            long.tick(&[Some(frame)]);
            ticks += 1;
        }
        // 60 × 2.075 ticks on average (the ⅔ gate jitters).
        assert!((100..=150).contains(&ticks), "{ticks}");
    }

    /// A droplet at rest falls by a quarter of gravity, keeping 0.95 of its
    /// speed a second: 0.035 57 cm in its first tick, 66.43 cm in the 61 ticks
    /// it moves before its 62nd ages it out, and never sideways.
    #[test]
    fn droplets_fall_at_a_quarter_of_gravity() {
        let mut sprays = BloodSprays::new(1);
        let start = Vec3::new(10.0, 0.0, -5.0);
        sprays.droplets.push_back(Droplet {
            p: start,
            q: start,
            life: 62,
            look: 0,
            slot: 0,
        });
        sprays.tick(&[]);
        let d = sprays.droplets[0];
        assert!(close(
            d.p.y - start.y,
            droplet_fall() * droplet_retention(),
            1e-6
        ));
        assert!(close(d.p.y - start.y, -0.035_566, 1e-5));
        for _ in 0..60 {
            sprays.tick(&[]);
        }
        let (mut v, mut y) = (0.0, 0.0);
        for _ in 0..61 {
            v = (v + droplet_fall()) * droplet_retention();
            y += v;
        }
        let d = sprays.droplets[0];
        assert_eq!(d.life, 1);
        assert!(close(d.p.y - start.y, y, 1e-3), "{} {y}", d.p.y - start.y);
        assert!(close(y, -66.43, 0.01), "{y}");
        assert_eq!((d.p.x, d.p.z), (start.x, start.z));
        sprays.tick(&[]);
        assert!(sprays.droplets.is_empty());
    }

    /// The first droplets fly against the swing: a jet along −X (a blade
    /// swinging +X) on an unturned part sends its first droplet at twice the
    /// average of the random velocity and the seed 2·(−X): 2.0 to 2.58
    /// cm/tick along −X. On a part turned a quarter about +Y the seed is the
    /// jet's axis in the part's frame, (0, 0, −2): the first droplet leans
    /// along −Z, as the original's does, and the lean halves with each
    /// droplet after.
    #[test]
    fn the_first_droplets_carry_the_jets_seed() {
        let contact = Vec3::new(0.0, 140.0, 0.0);
        let mut sprays = BloodSprays::new(11);
        let frame = Affine3A::IDENTITY;
        sprays.start(jet(frame, contact, Vec3::NEG_X, 30));
        while sprays.droplets.is_empty() {
            sprays.tick(&[Some(frame)]);
        }
        let first = sprays.droplets[0];
        let v = first.p - first.q;
        assert!(v.x <= -2.0 && v.x >= -2.58, "{v}");
        assert!(v.y.abs() <= 0.15 && v.z.abs() <= 0.15, "{v}");
        // Starts 0.8 of its velocity ahead of the contact.
        assert!(first.p.distance(contact + v * 0.4) < 1e-4);

        let turned = Affine3A::from_rotation_translation(
            Quat::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(5.0, 120.0, 0.0),
        );
        let mut sprays = BloodSprays::new(11);
        sprays.start(jet(turned, contact, Vec3::NEG_X, 30));
        let seed = sprays.jets[0].carry;
        assert!(seed.distance(Vec3::new(0.0, 0.0, -2.0)) < 1e-5, "{seed}");
        while sprays.droplets.is_empty() {
            sprays.tick(&[Some(turned)]);
        }
        let v = sprays.droplets[0].p - sprays.droplets[0].q;
        assert!(v.z <= -1.85 && v.z >= -2.15, "{v}");
        assert!(v.x <= 0.0 && v.x >= -0.58, "{v}");
        while sprays.droplets.len() < 6 {
            sprays.tick(&[Some(turned)]);
        }
        let sixth = sprays.droplets[5];
        let v = sixth.p - sixth.q;
        // The seed's share is 1/64 by the sixth droplet: 0.0625 of its
        // 2·2 cm/tick, beside at most 0.285 from the random spreads.
        assert!(v.z.abs() < 0.36 && v.x < 0.0, "{v}");
    }

    /// The jet rides with the limb: its droplets start where the moved part
    /// carries the contact, and take 0.15 of how far it moved.
    #[test]
    fn jets_ride_with_the_struck_part() {
        let contact = Vec3::new(0.0, 140.0, 20.0);
        let frame = Affine3A::from_translation(Vec3::new(0.0, 130.0, 0.0));
        let mut sprays = BloodSprays::new(5);
        sprays.start(jet(frame, contact, Vec3::Y, 40));
        let mut origin = contact;
        for t in 0..40 {
            let part = Affine3A::from_rotation_translation(
                Quat::from_rotation_y(0.05 * t as f32),
                Vec3::new(t as f32, 130.0, 0.0),
            );
            sprays.tick(&[Some(part)]);
            if sprays.droplets.back().is_some_and(|d| d.life == 62) {
                origin = part.transform_point3(frame.inverse().transform_point3(contact));
            }
        }
        let jet = &sprays.jets[0];
        assert!(jet.left > 0);
        assert!(jet.last.distance(origin) < 1e-3, "{} {origin}", jet.last);
        let newest = sprays.droplets.back().unwrap();
        let start = (newest.p * DROPLET_BEHIND + newest.q * DROPLET_AHEAD)
            / (DROPLET_AHEAD + DROPLET_BEHIND);
        assert!(start.distance(jet.last) < 1e-3, "{start} {}", jet.last);
        assert!(jet.motion.length() > 0.5);
        // A jet whose part is gone stops at its next emission.
        for _ in 0..10 {
            sprays.tick(&[None]);
            assert!(sprays.droplets.iter().all(|d| d.life < 62));
        }
        assert_eq!(sprays.jets[0].left, 0);
    }

    /// A finished jet's place goes to the next; running jets keep theirs.
    #[test]
    fn finished_jets_make_room() {
        let mut sprays = BloodSprays::new(1);
        let f = Affine3A::IDENTITY;
        sprays.start(jet(f, Vec3::ZERO, Vec3::X, 2));
        sprays.start(jet(f, Vec3::ZERO, Vec3::X, 50));
        while sprays.jets[0].left > 0 {
            sprays.tick(&[Some(f), Some(f)]);
        }
        sprays.start(jet(f, Vec3::ONE, Vec3::Y, 9));
        assert_eq!(sprays.jets.len(), 2);
        assert_eq!((sprays.jets[0].left, sprays.jets[0].last), (9, Vec3::ONE));
        assert!(sprays.jets[1].left > 0);
    }

    /// Every ring slot's half-size lies in 12–24 cm and slots go round all
    /// 513; at most 512 droplets live.
    #[test]
    fn slots_and_their_sizes() {
        let mut sprays = BloodSprays::new(9);
        assert_eq!(sprays.sizes.len(), 513);
        assert!(sprays.sizes.iter().all(|s| (12.0..24.0).contains(s)));
        let lo = sprays.sizes.iter().copied().fold(24.0, f32::min);
        let hi = sprays.sizes.iter().copied().fold(0.0, f32::max);
        assert!(lo < 12.5 && hi > 23.5, "{lo} {hi}");
        // Forty full jets at once would pass 512 droplets within a second.
        let f = Affine3A::IDENTITY;
        let jets: Vec<Option<Affine3A>> = vec![Some(f); 40];
        for _ in 0..40 {
            sprays.start(jet(f, Vec3::ZERO, Vec3::X, 60));
        }
        for _ in 0..40 {
            sprays.tick(&jets);
            assert!(sprays.droplets.len() <= DROPLET_SLOTS);
        }
        assert_eq!(sprays.droplets.len(), DROPLET_SLOTS);
        assert!(sprays.droplets.iter().all(|d| usize::from(d.slot) <= 512));
    }

    /// A slow droplet hangs down: seen from the side at age 0 with half-size
    /// 24, its sides 24 cm out and its tail and head 24·(1 + 0.4·π/2) =
    /// 39.08 cm above and below; at age ½ seen side-on it is 30 cm wide.
    #[test]
    fn a_resting_droplet_hangs_down() {
        let at = Vec3::new(0.0, 100.0, 0.0);
        let d = Droplet {
            p: at,
            q: at,
            life: 62,
            look: 0,
            slot: 0,
        };
        let [tail, a, b, head] = droplet_quad(&d, 24.0, 0.0, Vec3::NEG_Z);
        assert!(tail.distance(at + Vec3::Y * 39.08) < 0.01, "{tail}");
        assert!(head.distance(at - Vec3::Y * 39.08) < 0.01, "{head}");
        assert!(close(a.distance(at), 24.0, 1e-4) && close(b.distance(at), 24.0, 1e-4));
        assert!(close(a.y, at.y, 1e-4) && close(a.z, at.z, 1e-4));
        let older = Droplet { life: 31, ..d };
        let [_, a, _, _] = droplet_quad(&older, 24.0, 0.0, Vec3::NEG_Z);
        assert!(close(a.distance(at), 30.0, 1e-3), "{}", a.distance(at));
        // Seen from above (along its lean) it has no width.
        let [_, a, b, _] = droplet_quad(&d, 24.0, 0.0, Vec3::NEG_Y);
        assert_eq!(a, b);
    }

    /// The UVs: look 0 is the first splat as it is; look 31 the last splat
    /// under the eighth symmetry.
    #[test]
    fn droplet_looks() {
        let uv = |x: f32, y: f32| Vec2::new(x, y);
        assert_eq!(
            droplet_uvs(0),
            [uv(0.0, 0.0), uv(0.5, 0.0), uv(0.0, 0.5), uv(0.5, 0.5)]
        );
        assert_eq!(
            droplet_uvs(31),
            [uv(1.0, 0.5), uv(1.0, 1.0), uv(0.5, 0.5), uv(0.5, 1.0)]
        );
        assert_eq!(
            droplet_uvs(9),
            [uv(1.0, 0.0), uv(0.5, 0.0), uv(1.0, 0.5), uv(0.5, 0.5)]
        );
    }

    #[test]
    fn debug_sprays_parse() {
        assert_eq!(
            DebugSpray::parse("chest,0.5"),
            Some(DebugSpray {
                bone: 9,
                size: 0.5,
                swing: Vec3::NEG_Z
            })
        );
        assert_eq!(
            DebugSpray::parse("15, 1, 1,0,0"),
            Some(DebugSpray {
                bone: 15,
                size: 1.0,
                swing: Vec3::X
            })
        );
        assert_eq!(DebugSpray::parse("chest"), None);
        assert_eq!(DebugSpray::parse("chest,0"), None);
        assert_eq!(DebugSpray::parse("chest,1,0,0,0"), None);
        assert_eq!(DebugSpray::parse("tail,1"), None);
        assert_eq!(DebugSpray::parse("19,1"), None);
    }

    /// The installed droplet texture (skipped without the game): 256², BC1,
    /// no wrap bit in its flags (clamped), a 2 × 2 atlas of red spatter on
    /// white that covers 9–13 % of each cell and keeps off the cells' edges.
    #[test]
    fn shipped_droplet_texture() {
        use exa_formats::rfi::{PixelFormat, Rfi};
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let rfi = Rfi::parse(game.texture(SPRAY_TEXTURE).expect("bldprt02")).unwrap();
        assert_eq!((rfi.header.width, rfi.header.height), (256, 256));
        assert_eq!(rfi.format, PixelFormat::Bc1);
        assert_eq!(rfi.header.flags, 0x20A8_0000);
        let rgba = rfi.decode_rgba8().unwrap();
        let red = |x: usize, y: usize| {
            let p = &rgba[(y * 256 + x) * 4..][..4];
            i32::from(p[0]) - i32::from(p[1]) > 40
        };
        for (cell, share) in [0.092, 0.133, 0.101, 0.104].into_iter().enumerate() {
            let (x0, y0) = ((cell % 2) * 128, (cell / 2) * 128);
            let mut count = 0;
            let mut edge = 0;
            for y in y0..y0 + 128 {
                for x in x0..x0 + 128 {
                    if red(x, y) {
                        count += 1;
                        if [x - x0, y - y0].iter().any(|&c| c == 0 || c == 127) {
                            edge += 1;
                        }
                    }
                }
            }
            let got = count as f32 / (128.0 * 128.0);
            assert!((got - share).abs() < 0.002, "cell {cell}: {got}");
            assert!(edge <= 4, "cell {cell}: {edge} red texels on its edge");
        }
    }

    /// A bleeding blow's spray starts a jet on the struck part, its frame
    /// in the part's, its seed the jet's axis there; a blow on a part the
    /// body lacks starts none.
    #[test]
    fn a_spray_starts_a_jet_on_the_struck_part() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<Spray>()
            .init_resource::<BloodSprays>()
            .add_systems(Update, start_jets);
        let body = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let turn = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
        let chest = app
            .world_mut()
            .spawn((
                BodyPart {
                    bone: 9,
                    rest: Transform::IDENTITY,
                },
                GlobalTransform::from(
                    Transform::from_xyz(0.0, 1.3, 0.0)
                        .with_rotation(turn)
                        .with_scale(Vec3::splat(1.1)),
                ),
                ChildOf(body),
            ))
            .id();
        let contact = Vec3::new(0.0, 1.4, 0.2);
        for bone in [9, 12] {
            app.world_mut().write_message(Spray {
                body,
                bone,
                contact,
                swing: Vec3::new(3.0, 0.0, 0.0),
                droplets: 16,
            });
        }
        app.update();
        let sprays = app.world().resource::<BloodSprays>();
        assert_eq!(sprays.jets.len(), 1);
        let jet = &sprays.jets[0];
        assert_eq!((jet.body, jet.part, jet.left), (body, chest, 16));
        assert!(jet.last.distance(contact * 100.0) < 1e-4);
        assert!(jet.carry.distance(Vec3::new(0.0, 0.0, -2.0)) < 1e-5);
        // The frame ignores the part's scale.
        let frame = Affine3A::from_rotation_translation(turn, Vec3::new(0.0, 130.0, 0.0));
        let origin = (frame * jet.local).translation;
        assert!(Vec3::from(origin).distance(contact * 100.0) < 1e-3);
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    #[test]
    fn the_scene_node_is_the_original_blood_spray() {
        let node = BloodSprays::scene_node();
        assert_eq!(node.name, "_BloodSpray");
        assert_eq!(node.material, "_BloodPrt02");
        assert_eq!(node.texture, "BldPrt02");
        assert_eq!(node.flags, 0x1000);
        assert_eq!(node.material_flags, 0x0008_0008);
        assert_eq!(node.material_extra, 6);
    }
}

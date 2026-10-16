//! Wound blood on characters, as the original paints it: one byte per
//! vertex, the alpha of the mesh's colour stream (255 clean), lowered
//! around each wound and never raised again. The surfaces that show it
//! compare it with a tiling speckle texture
//! (`exa_render::material::BloodProgram`), which gives the dark red, the
//! speckled edges and the wet sheen. Rules: `docs/RENDERING.md` (Blood).
//!
//! - **What is painted**: a character's skinned meshes (the body and every
//!   skinned garment; the original merges them into one skinned mesh) and
//!   every other mesh under its head part. Held items and rigid items on
//!   other parts are never painted, nor meshes hidden under apparel.
//! - **Per-character meshes**: the meshes of a model are shared by every
//!   character wearing it. The first wound that lowers a value of a shared
//!   mesh gives that character its own copy (with a colour stream, white
//!   until painted); later wounds paint the copy in place. A copy costs the
//!   mesh's vertex data again (76 bytes a vertex, 4 of them the colour, and
//!   4 bytes an index) on the CPU and the GPU; clean characters copy
//!   nothing.
//! - **The painter** ([`wound_alpha`]): the wound's centre is the contact
//!   point moved back by half of how the striking point travelled since the
//!   hit, `travel`. Behind the centre the wound is a ball of the wound's
//!   radius; ahead of it, along `travel`, a smear about √2 times as wide
//!   that runs on to the end of the mesh. Vertices are taken where the
//!   current pose puts them.
//!
//! [`PaintWound`] paints one wound now. Wounds from hits come from the
//! combat (`exa_combat::wounds`): a blow deep enough on a race that bleeds
//! waits seven body ticks on the victim, then bleeds ([`Bleed`]) with how
//! the striker moved on since, and is painted here, on corpses too. Blood
//! never spreads, fades or drips.
//! [`DebugWounds`] paints test wounds on the player or the character view's
//! body (`--debug-wound`, `docs/DEBUGGING.md`).

use std::collections::{HashMap, HashSet};

use bevy::math::Affine3A;
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Indices, MeshVertexAttribute, VertexAttributeValues, VertexFormat};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_anim::BodyPart;
use exa_assets::SkinnedPart;
use exa_combat::CombatSystems;
use exa_combat::wounds::Bleed;
use exa_render::material::ExaMaterial;

use crate::Dressed;

pub struct BloodPlugin;

impl Plugin for BloodPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PaintWound>()
            .add_message::<Bleed>()
            .init_resource::<WoundRng>()
            .add_systems(
                PostUpdate,
                (
                    paint_debug_wounds.run_if(resource_exists::<DebugWounds>),
                    bleed_wounds,
                    paint_wounds,
                )
                    .chain()
                    // Vertices are taken where this frame's pose puts them,
                    // after the combat's wounds bled.
                    .after(TransformSystems::Propagate)
                    .after(CombatSystems),
            );
    }
}

/// The vertex stream that carries the blood: the colour stream, as bytes,
/// white with the blood value in alpha (255 clean, 0 soaked).
pub const BLOOD_STREAM: MeshVertexAttribute = MeshVertexAttribute {
    format: VertexFormat::Unorm8x4,
    ..Mesh::ATTRIBUTE_COLOR
};

/// The bone of the head part, whose meshes (hair, helmets, hoods) take
/// blood whether skinned or not.
pub const HEAD: usize = 10;

/// A wound to paint on a character now.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct PaintWound {
    /// The character's body model root (the entity with its `Animator`).
    pub body: Entity,
    /// Where the blow struck, in world space (metres).
    pub contact: Vec3,
    /// How the striking point moved on from the contact point since the
    /// blow, in world space (metres): the smear runs along it.
    pub travel: Vec3,
    /// The wound's radius (metres); the original's is 30·R + 10 cm, R the
    /// blow's lasting damage on the part.
    pub radius: f32,
}

impl PaintWound {
    /// The centre the painter measures from: the contact point less half
    /// the travel.
    pub fn centre(&self) -> Vec3 {
        self.contact - self.travel * 0.5
    }
}

/// A mesh part whose mesh is its character's own copy, carrying the blood
/// stream: later wounds paint it in place.
#[derive(Component, Debug, Clone, Copy)]
pub struct Bloodied;

/// How deep inside a wound a vertex at offset `d` from its centre lies,
/// for a wound of radius `radius` whose striking point travelled `travel`:
/// f = 1 − e/r², 1 on the smear's axis and 0 at its edge; `None` outside.
///
/// The distance measure e is |d|² behind the centre (or with no travel)
/// and |d|²·(1 − cos θ) ahead of it, θ the angle between `d` and `travel`:
/// a ball of radius r behind, and ahead a smear that runs on along the
/// travel, about √2·r wide well ahead (e ≈ ρ²/2 at a distance ρ from its
/// axis).
pub fn wound_depth(d: Vec3, travel: Vec3, radius: f32) -> Option<f32> {
    let r2 = radius * radius;
    let len2 = d.length_squared();
    let mut e = len2;
    let along = travel.length();
    if along > 0.0 {
        let ahead = d.dot(travel) / along;
        if ahead > 0.0 {
            e = len2 * (1.0 - ahead / len2.sqrt());
        }
    }
    (e <= r2).then(|| 1.0 - e / r2)
}

/// The blood value (0..255) the painter gives a vertex at depth `f`, with
/// `u` a uniform random number in [0, 1): with g = f + (1 − f)·u,
/// 255·(1 − f·g²). 0 on the smear's axis, 128–223 at f = ½ and 191–251 at
/// f = ¼: dense at the centre, speckled toward the edge.
pub fn wound_alpha(f: f32, u: f32) -> f32 {
    let g = f + (1.0 - f) * u;
    255.0 * (1.0 - f * g * g)
}

/// Paints `wound` into the blood values `alphas` of vertices at the world
/// positions `positions`, lowering only: a value is written when the new
/// one is below it, rounded to the nearest byte. Returns whether any value
/// fell.
pub fn paint(
    alphas: &mut [u8],
    positions: &[Vec3],
    wound: &PaintWound,
    rng: &mut WoundRng,
) -> bool {
    let centre = wound.centre();
    let mut changed = false;
    for (alpha, p) in alphas.iter_mut().zip(positions) {
        let Some(f) = wound_depth(*p - centre, wound.travel, wound.radius) else {
            continue;
        };
        // The random number is drawn only for vertices inside the wound.
        let value = wound_alpha(f, rng.unit());
        if value < f32::from(*alpha) {
            let byte = value.round_ties_even() as u8;
            changed |= byte < *alpha;
            *alpha = byte;
        }
    }
    changed
}

/// The random numbers of the painter's speckled edge: a fixed sequence, so
/// that a run paints the same wounds the same way.
#[derive(Resource, Debug, Clone)]
pub struct WoundRng(u64);

impl Default for WoundRng {
    fn default() -> Self {
        Self(0x9E37_79B9_7F4A_7C15)
    }
}

impl WoundRng {
    /// The next number, uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        // xorshift64*.
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        let r = x.wrapping_mul(0x2545_F491_4F6C_DD1D);
        (r >> 40) as f32 / (1u64 << 24) as f32
    }
}

/// The blood values of a mesh: its blood stream's alphas, or all clean.
fn blood_of(mesh: &Mesh) -> Vec<u8> {
    match mesh.attribute(BLOOD_STREAM.id) {
        Some(VertexAttributeValues::Unorm8x4(c)) => c.iter().map(|c| c[3]).collect(),
        _ => vec![255; mesh.count_vertices()],
    }
}

/// The blood stream of blood values `alphas`.
fn blood_stream(alphas: &[u8]) -> VertexAttributeValues {
    VertexAttributeValues::Unorm8x4(alphas.iter().map(|&a| [255, 255, 255, a]).collect())
}

/// Where the current pose puts a skinned mesh's vertices, in world space:
/// each vertex through its joints' matrices (the joint part's transform
/// times its inverse bind pose), weighted, as the GPU skins it.
pub fn skinned_positions(
    mesh: &Mesh,
    skin: &SkinnedMesh,
    bindposes: &Assets<SkinnedMeshInverseBindposes>,
    joints: &Query<&GlobalTransform>,
) -> Option<Vec<Vec3>> {
    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
    let Some(VertexAttributeValues::Uint16x4(indices)) =
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
    else {
        return None;
    };
    let Some(VertexAttributeValues::Float32x4(weights)) =
        mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
    else {
        return None;
    };
    let inverse = bindposes.get(&skin.inverse_bindposes)?;
    let matrices: Vec<Affine3A> = skin
        .joints
        .iter()
        .zip(inverse.iter())
        .map(|(&joint, bind)| {
            joints
                .get(joint)
                .map(|g| g.affine() * Affine3A::from_mat4(*bind))
        })
        .collect::<Result<_, _>>()
        .ok()?;
    Some(
        positions
            .iter()
            .zip(indices)
            .zip(weights)
            .map(|((p, i), w)| {
                let p = Vec3::from(*p);
                (0..4)
                    .filter(|&k| w[k] != 0.0)
                    .filter_map(|k| {
                        matrices
                            .get(usize::from(i[k]))
                            .map(|m| m.transform_point3(p) * w[k])
                    })
                    .sum()
            })
            .collect(),
    )
}

/// The parts of `body` a wound can paint: its visible skinned meshes, and
/// its other visible meshes whose nearest body part is the head.
#[allow(clippy::type_complexity)]
fn wound_targets(
    body: Entity,
    children: &Query<&Children>,
    parents: &Query<&ChildOf>,
    parts: &Query<&BodyPart>,
    candidates: &Query<
        (Has<SkinnedPart>, Option<&Visibility>),
        (With<Mesh3d>, With<MeshMaterial3d<ExaMaterial>>),
    >,
) -> Vec<Entity> {
    children
        .iter_descendants(body)
        .filter(|&e| {
            let Ok((skinned, visibility)) = candidates.get(e) else {
                return false;
            };
            if visibility == Some(&Visibility::Hidden) {
                return false;
            }
            skinned
                || parents
                    .iter_ancestors(e)
                    .take_while(|&a| a != body)
                    .find_map(|a| parts.get(a).ok())
                    .is_some_and(|p| p.bone == HEAD)
        })
        .collect()
}

/// Paints the wounds that bleed this frame.
fn bleed_wounds(mut bleeds: MessageReader<Bleed>, mut out: MessageWriter<PaintWound>) {
    for b in bleeds.read() {
        out.write(PaintWound {
            body: b.body,
            contact: b.contact,
            travel: b.travel,
            radius: b.radius,
        });
    }
}

/// Paints every wound sent this frame onto its character's meshes.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn paint_wounds(
    mut commands: Commands,
    mut wounds: MessageReader<PaintWound>,
    mut rng: ResMut<WoundRng>,
    children: Query<&Children>,
    parents: Query<&ChildOf>,
    parts: Query<&BodyPart>,
    candidates: Query<
        (Has<SkinnedPart>, Option<&Visibility>),
        (With<Mesh3d>, With<MeshMaterial3d<ExaMaterial>>),
    >,
    mut meshes_of: Query<(
        &mut Mesh3d,
        Option<&SkinnedMesh>,
        Option<&mut SkinnedPart>,
        Has<Bloodied>,
    )>,
    transforms: Query<&GlobalTransform>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    // Parts given their own copy this frame: `Bloodied` lands only when the
    // commands apply, and a later wound in the same frame paints the copy.
    let mut copied_now = HashSet::new();
    for wound in wounds.read() {
        let (mut painted, mut copied, mut vertices, mut bytes) = (0, 0, 0, 0);
        for part in wound_targets(wound.body, &children, &parents, &parts, &candidates) {
            let Ok((mut mesh3d, skin, skinned, bloodied)) = meshes_of.get_mut(part) else {
                continue;
            };
            let Some(mesh) = meshes.get(&mesh3d.0) else {
                continue;
            };
            let positions = match (skin, skinned.is_some()) {
                (Some(skin), _) => skinned_positions(mesh, skin, &bindposes, &transforms),
                // A skinned part not bound to its skeleton yet is not drawn.
                (None, true) => None,
                (None, false) => transforms.get(part).ok().and_then(|g| {
                    let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION)?.as_float3()?;
                    Some(
                        positions
                            .iter()
                            .map(|p| g.transform_point(Vec3::from(*p)))
                            .collect(),
                    )
                }),
            };
            let Some(positions) = positions else { continue };
            let mut alphas = blood_of(mesh);
            if !paint(&mut alphas, &positions, wound, &mut rng) {
                continue;
            }
            painted += 1;
            vertices += alphas.iter().filter(|&&a| a < 255).count();
            if bloodied || copied_now.contains(&part) {
                if let Some(mut own) = meshes.get_mut(&mesh3d.0) {
                    own.insert_attribute(BLOOD_STREAM, blood_stream(&alphas));
                }
                continue;
            }
            // The character's own copy, from the shared mesh.
            let mut own = mesh.clone();
            own.insert_attribute(BLOOD_STREAM, blood_stream(&alphas));
            bytes += mesh_bytes(&own);
            let handle = meshes.add(own);
            mesh3d.0 = handle.clone();
            if let Some(mut skinned) = skinned {
                skinned.mesh = handle;
            }
            commands.entity(part).insert(Bloodied);
            copied_now.insert(part);
            copied += 1;
        }
        info!(
            "wound on {:?} (radius {:.0} cm): {painted} meshes painted ({vertices} bloodied vertices), {copied} copied ({} KiB)",
            wound.body,
            wound.radius * 100.0,
            bytes / 1024
        );
    }
}

/// Bytes of a mesh's vertex and index data.
fn mesh_bytes(mesh: &Mesh) -> usize {
    let indices = match mesh.indices() {
        Some(Indices::U32(i)) => i.len() * 4,
        Some(Indices::U16(i)) => i.len() * 2,
        None => 0,
    };
    mesh.get_vertex_size() as usize * mesh.count_vertices() + indices
}

/// Test wounds painted on every [`DebugWoundTarget`] body once it is
/// dressed and bound to its skeleton (`--debug-wound`).
#[derive(Resource, Debug, Clone, Default)]
pub struct DebugWounds(pub Vec<DebugWound>);

/// Marks a body that [`DebugWounds`] paints: the player's body and the
/// character view's body, of any race.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct DebugWoundTarget;

/// A test wound: on body part `bone`, at the front of its meshes (the
/// vertex furthest along the body's facing), of radius `radius` (metres),
/// with the striking point's travel `travel` in the body's own frame
/// (metres; +Y up, +Z the way the body faces).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DebugWound {
    pub bone: usize,
    pub radius: f32,
    pub travel: Vec3,
}

/// Body part names `--debug-wound` takes, by bone.
const BONE_NAMES: [(&str, usize); 18] = [
    ("pelvis", 1),
    ("ruleg", 2),
    ("rlleg", 3),
    ("rfoot", 4),
    ("luleg", 5),
    ("llleg", 6),
    ("lfoot", 7),
    ("belly", 8),
    ("chest", 9),
    ("head", 10),
    ("rshld", 11),
    ("ruarm", 12),
    ("rfarm", 13),
    ("lshld", 14),
    ("luarm", 15),
    ("lfarm", 16),
    ("rhand", 17),
    ("lhand", 18),
];

impl DebugWound {
    /// Reads `bone,radius_cm[,dx,dy,dz]`: the bone by number (1–18) or by
    /// its body part's name (`chest`, `luarm`, …), the radius and the
    /// travel in centimetres.
    pub fn parse(text: &str) -> Option<Self> {
        let mut fields = text.split(',').map(str::trim);
        let bone_name = fields.next()?.to_ascii_lowercase();
        let bone = match bone_name.parse::<usize>() {
            Ok(b) => b,
            Err(_) => BONE_NAMES.iter().find(|(n, _)| *n == bone_name)?.1,
        };
        let numbers: Vec<f32> = fields.map(|f| f.parse().ok()).collect::<Option<_>>()?;
        let (radius, travel) = match numbers.as_slice() {
            [r] => (*r, Vec3::ZERO),
            [r, x, y, z] => (*r, Vec3::new(*x, *y, *z)),
            _ => return None,
        };
        ((1..=18).contains(&bone) && radius > 0.0).then_some(Self {
            bone,
            radius: radius / 100.0,
            travel: travel / 100.0,
        })
    }
}

/// Frames a dressed body waits before its test wounds, so that its newly
/// bound skin has a propagated pose.
const DEBUG_WOUND_DELAY: u32 = 3;

/// Sends the test wounds for each marked body once it is dressed and bound.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn paint_debug_wounds(
    wounds: Res<DebugWounds>,
    bodies: Query<(Entity, &GlobalTransform), (With<DebugWoundTarget>, With<Dressed>)>,
    children: Query<&Children>,
    skinned: Query<(
        &SkinnedPart,
        Option<&SkinnedMesh>,
        &Mesh3d,
        Option<&Visibility>,
    )>,
    unbound: Query<(), (With<SkinnedPart>, Without<SkinnedMesh>)>,
    transforms: Query<&GlobalTransform>,
    bindposes: Res<Assets<SkinnedMeshInverseBindposes>>,
    meshes: Res<Assets<Mesh>>,
    mut waiting: Local<HashMap<Entity, u32>>,
    mut done: Local<HashSet<Entity>>,
    mut out: MessageWriter<PaintWound>,
) {
    for (body, body_transform) in &bodies {
        if done.contains(&body) {
            continue;
        }
        if children.iter_descendants(body).any(|e| unbound.contains(e)) {
            continue;
        }
        let frames = waiting.entry(body).or_default();
        *frames += 1;
        if *frames < DEBUG_WOUND_DELAY {
            continue;
        }
        done.insert(body);
        let facing = body_transform.rotation() * Vec3::Z;
        for wound in &wounds.0 {
            let front = children
                .iter_descendants(body)
                .filter_map(|e| skinned.get(e).ok())
                .filter(|(part, _, _, visibility)| {
                    usize::from(part.bone) == wound.bone && *visibility != Some(&Visibility::Hidden)
                })
                .filter_map(|(_, skin, mesh, _)| {
                    skinned_positions(meshes.get(&mesh.0)?, skin?, &bindposes, &transforms)
                })
                .flatten()
                .max_by(|a, b| a.dot(facing).total_cmp(&b.dot(facing)));
            let Some(contact) = front else {
                warn!(
                    "debug wound: body {body:?} has no visible mesh on bone {}",
                    wound.bone
                );
                continue;
            };
            info!(
                "debug wound on bone {} at {contact:.3} m, radius {:.0} cm",
                wound.bone,
                wound.radius * 100.0
            );
            out.write(PaintWound {
                body,
                contact,
                travel: body_transform.rotation() * wound.travel,
                radius: wound.radius,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::asset::RenderAssetUsages;
    use bevy::mesh::PrimitiveTopology;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    /// The wound's shape: a ball of radius r behind its centre (and with no
    /// travel), a smear ahead of it along the travel, √2·r wide well ahead.
    #[test]
    fn wounds_are_balls_behind_and_smears_ahead() {
        let r = 0.2;
        // No travel: a ball.
        assert_eq!(wound_depth(Vec3::ZERO, Vec3::ZERO, r), Some(1.0));
        assert!(close(
            wound_depth(Vec3::new(0.0, 0.0, 0.1), Vec3::ZERO, r).unwrap(),
            0.75
        ));
        assert_eq!(wound_depth(Vec3::new(0.0, 0.0, 0.201), Vec3::ZERO, r), None);
        // A blow travelling 30 cm down: straight below the centre the
        // whole line is soaked, however far; straight above it, the ball.
        let down = Vec3::new(0.0, -0.3, 0.0);
        assert_eq!(wound_depth(Vec3::new(0.0, -1.0, 0.0), down, r), Some(1.0));
        assert_eq!(wound_depth(Vec3::new(0.0, 0.201, 0.0), down, r), None);
        assert!(close(
            wound_depth(Vec3::new(0.0, 0.1, 0.0), down, r).unwrap(),
            0.75
        ));
        // Well ahead, e ≈ ρ²/2: 1.3·r off the axis is inside, 1.5·r not.
        let ahead = |rho: f32| wound_depth(Vec3::new(rho, -20.0, 0.0), down, r);
        assert!(close(ahead(1.3 * r).unwrap(), 1.0 - 1.69 / 2.0));
        assert_eq!(ahead(1.5 * r), None);
        // Across the travel (cos θ = 0) the ball again.
        assert!(close(
            wound_depth(Vec3::new(0.1, 0.0, 0.0), down, r).unwrap(),
            0.75
        ));
    }

    /// 255·(1 − f·g²), g = f + (1 − f)·u: 0 on the axis, 128–223 at f = ½,
    /// 191–251 at f = ¼ (the original's values per vertex).
    #[test]
    fn the_edge_is_speckled() {
        for u in [0.0, 0.3, 0.999] {
            assert_eq!(wound_alpha(1.0, u), 0.0);
        }
        assert!(close(wound_alpha(0.5, 0.0), 223.125));
        assert!(close(wound_alpha(0.5, 1.0), 127.5));
        assert!(close(wound_alpha(0.25, 0.0), 251.015_6));
        assert!(close(wound_alpha(0.25, 1.0), 191.25));
        assert_eq!(wound_alpha(0.0, 0.5), 255.0);
        let mut rng = WoundRng::default();
        let mut again = WoundRng::default();
        for _ in 0..1000 {
            let u = rng.unit();
            assert!((0.0..1.0).contains(&u));
            assert_eq!(u, again.unit());
        }
    }

    /// Blood only accumulates: a value is written only when the new one is
    /// lower, rounded to the nearest byte.
    #[test]
    fn painting_only_lowers() {
        let wound = PaintWound {
            body: Entity::PLACEHOLDER,
            contact: Vec3::ZERO,
            travel: Vec3::ZERO,
            radius: 0.2,
        };
        let positions = [
            Vec3::ZERO,
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::new(0.3, 0.0, 0.0),
        ];
        let mut rng = WoundRng::default();
        // f = 0.75 at 10 cm: 255·(1 − 0.75·g²) lies in 64–147.
        let mut alphas = [255, 20, 255];
        assert!(paint(&mut alphas, &positions, &wound, &mut rng));
        assert_eq!(alphas, [0, 20, 255]);
        let mut fresh = [255, 255, 255];
        paint(&mut fresh, &positions, &wound, &mut WoundRng::default());
        assert!((64..=148).contains(&fresh[1]), "{}", fresh[1]);
        // The same wound again changes nothing it soaked already.
        let mut again = [0, 20, 255];
        assert!(!paint(&mut again, &positions, &wound, &mut rng));
        // The travel moves the centre back by half of it.
        let moved = PaintWound {
            travel: Vec3::new(0.0, -0.3, 0.0),
            ..wound
        };
        assert_eq!(moved.centre(), Vec3::new(0.0, 0.15, 0.0));
    }

    #[test]
    fn debug_wounds_are_read() {
        assert_eq!(
            DebugWound::parse("chest,20"),
            Some(DebugWound {
                bone: 9,
                radius: 0.2,
                travel: Vec3::ZERO
            })
        );
        let arm = DebugWound::parse(" LUArm , 15, 0, -30, 5").unwrap();
        assert_eq!(arm.bone, 15);
        assert!(close(arm.radius, 0.15));
        assert!((arm.travel - Vec3::new(0.0, -0.3, 0.05)).length() < 1e-6);
        assert_eq!(DebugWound::parse("13,12").map(|w| w.bone), Some(13));
        for bad in [
            "chest",
            "chest,0",
            "19,10",
            "0,10",
            "foo,10",
            "chest,10,1,2",
            "chest,x",
        ] {
            assert_eq!(DebugWound::parse(bad), None, "{bad}");
        }
    }

    /// A strip of 11 vertices along x, 10 cm apart, centred on its origin,
    /// skinned wholly to joint 0 when `skinned`.
    fn strip(skinned: bool) -> Mesh {
        let positions: Vec<[f32; 3]> = (-5..=5).map(|i| [i as f32 * 0.1, 0.0, 0.0]).collect();
        let indices: Vec<u32> = (0..9).flat_map(|i| [i, i + 1, i + 2]).collect();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_indices(Indices::U32(indices));
        if skinned {
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_INDEX,
                VertexAttributeValues::Uint16x4(vec![[0; 4]; 11]),
            );
            mesh.insert_attribute(
                Mesh::ATTRIBUTE_JOINT_WEIGHT,
                vec![[1.0f32, 0.0, 0.0, 0.0]; 11],
            );
        }
        mesh
    }

    struct Body {
        root: Entity,
        chest: Entity,
        hidden: Entity,
        helmet: Entity,
        sword: Entity,
    }

    /// A body at `x`: a chest part with a skinned strip and a hidden one, a
    /// head part 1 m up with a rigid strip, and a hand part holding a rigid
    /// strip across the chest; the strips are the shared meshes `skinned`
    /// and `rigid`.
    fn body(world: &mut World, x: f32, skinned: &Handle<Mesh>, rigid: &Handle<Mesh>) -> Body {
        let at = |y: f32| GlobalTransform::from_translation(Vec3::new(x, y, 0.0));
        let part = |bone: usize| BodyPart {
            bone,
            rest: Transform::IDENTITY,
        };
        let bind = world
            .resource_mut::<Assets<SkinnedMeshInverseBindposes>>()
            .add(SkinnedMeshInverseBindposes::from(vec![Mat4::IDENTITY]));
        let root = world.spawn(at(0.0)).id();
        let chest_part = world.spawn((part(9), at(0.0), ChildOf(root))).id();
        let mut skinned_part = || {
            world
                .spawn((
                    Mesh3d(skinned.clone()),
                    MeshMaterial3d::<ExaMaterial>(Handle::default()),
                    SkinnedPart {
                        bone: 9,
                        mesh: skinned.clone(),
                    },
                    SkinnedMesh {
                        inverse_bindposes: bind.clone(),
                        joints: vec![chest_part],
                    },
                    at(0.0),
                    ChildOf(chest_part),
                ))
                .id()
        };
        let chest = skinned_part();
        let hidden = skinned_part();
        world.entity_mut(hidden).insert(Visibility::Hidden);
        let rigid_part = |world: &mut World, parent: Entity, y: f32| {
            world
                .spawn((
                    Mesh3d(rigid.clone()),
                    MeshMaterial3d::<ExaMaterial>(Handle::default()),
                    at(y),
                    ChildOf(parent),
                ))
                .id()
        };
        let head = world.spawn((part(HEAD), at(1.0), ChildOf(root))).id();
        let helmet = rigid_part(world, head, 1.0);
        let hand = world.spawn((part(17), at(0.0), ChildOf(root))).id();
        let holder = world.spawn((at(0.0), ChildOf(hand))).id();
        let sword = rigid_part(world, holder, 0.0);
        Body {
            root,
            chest,
            hidden,
            helmet,
            sword,
        }
    }

    fn alphas(app: &App, part: Entity) -> Option<Vec<u8>> {
        let handle = &app.world().get::<Mesh3d>(part)?.0;
        let mesh = app.world().resource::<Assets<Mesh>>().get(handle)?;
        match mesh.attribute(BLOOD_STREAM.id)? {
            VertexAttributeValues::Unorm8x4(c) => Some(c.iter().map(|c| c[3]).collect()),
            _ => None,
        }
    }

    /// A wound copies only the struck character's meshes it reaches,
    /// paints its skinned meshes and its head's meshes, and never its held
    /// items or hidden parts; later wounds paint the copies in place.
    #[test]
    fn wounds_paint_the_struck_characters_own_meshes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
            .add_plugins(BloodPlugin);
        let (skinned, rigid) = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            (meshes.add(strip(true)), meshes.add(strip(false)))
        };
        let a = body(app.world_mut(), 0.0, &skinned, &rigid);
        let b = body(app.world_mut(), 5.0, &skinned, &rigid);
        let wound = |body: Entity, contact: Vec3, radius: f32| PaintWound {
            body,
            contact,
            travel: Vec3::ZERO,
            radius,
        };
        app.world_mut()
            .write_message(wound(a.root, Vec3::ZERO, 0.25));
        app.update();

        let world = app.world();
        let own = world.get::<Mesh3d>(a.chest).unwrap().0.clone();
        assert_ne!(own, skinned);
        assert!(world.get::<Bloodied>(a.chest).is_some());
        assert_eq!(world.get::<SkinnedPart>(a.chest).unwrap().mesh, own);
        let chest = alphas(&app, a.chest).unwrap();
        // Within 25 cm: the centre soaked, 10 cm out (f = 0.84) between
        // 255·(1 − 0.84) and 255·(1 − 0.84³), 20 cm out (f = 0.36) between
        // 163 and 243; beyond, clean.
        assert_eq!(chest[5], 0);
        for v in [chest[4], chest[6]] {
            assert!((40..=104).contains(&v), "{chest:?}");
        }
        for v in [chest[3], chest[7]] {
            assert!((163..=243).contains(&v), "{chest:?}");
        }
        assert!(chest[..3].iter().chain(&chest[8..]).all(|&v| v == 255));
        // The other character, the hidden part, the sword and the helmet
        // out of reach keep the shared, clean meshes.
        for part in [b.chest, a.hidden, a.sword, a.helmet, b.helmet] {
            assert!(world.get::<Bloodied>(part).is_none());
            assert!(alphas(&app, part).is_none());
        }
        assert_eq!(world.get::<Mesh3d>(b.chest).unwrap().0, skinned);
        assert_eq!(world.get::<Mesh3d>(a.sword).unwrap().0, rigid);

        // A head wound paints the rigid helmet; the chest is out of reach.
        app.world_mut()
            .write_message(wound(a.root, Vec3::new(0.3, 1.0, 0.0), 0.15));
        app.update();
        let helmet = alphas(&app, a.helmet).unwrap();
        assert_eq!(helmet[8], 0);
        assert_eq!(helmet[5], 255);
        assert_eq!(alphas(&app, a.chest).unwrap(), chest);

        // A second chest wound paints the copy in place, darkening only.
        let count = app.world().resource::<Assets<Mesh>>().len();
        app.world_mut()
            .write_message(wound(a.root, Vec3::new(0.3, 0.0, 0.0), 0.15));
        app.update();
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), count);
        assert_eq!(app.world().get::<Mesh3d>(a.chest).unwrap().0, own);
        let again = alphas(&app, a.chest).unwrap();
        assert_eq!(again[8], 0);
        assert!(again.iter().zip(&chest).all(|(n, o)| n <= o));
        assert_eq!(again[..6], chest[..6]);

        // Two wounds on the other body in one frame: its chest is copied
        // once, and the second wound paints that copy.
        let count = app.world().resource::<Assets<Mesh>>().len();
        app.world_mut()
            .write_message(wound(b.root, Vec3::new(4.8, 0.0, 0.0), 0.15));
        app.world_mut()
            .write_message(wound(b.root, Vec3::new(5.2, 0.0, 0.0), 0.15));
        app.update();
        assert_eq!(app.world().resource::<Assets<Mesh>>().len(), count + 1);
        let both = alphas(&app, b.chest).unwrap();
        assert_eq!((both[3], both[7]), (0, 0));
        assert_eq!(both[5], 255);
    }

    /// The original's one-handed sword `sword_a` (90 cm): a cut 70 cm along
    /// it at 20 cm a tick wounds a bare chest with R = 0.457.
    fn sword_cut() -> (exa_combat::damage::WeaponRecord, exa_combat::damage::Strike) {
        use exa_formats::weapon_stats::{StatBytes, at};
        let mut s = StatBytes::default();
        for (off, v) in [
            (at::WEIGHT, 50),
            (at::BALANCE, 80),
            (at::IMPACT, 127),
            (at::THRUST, 54),
            (at::KIND, 2),
            (at::START, 20),
            (at::END, 200),
            (at::SLASH, 72),
        ] {
            s.set(off, v);
        }
        (
            exa_combat::damage::WeaponRecord::of_one_part(0x11, &s, false, 90.0),
            exa_combat::damage::Strike {
                speed: 20.0,
                stab: false,
                overhead: false,
                clock: 0.8,
                along: 70.0,
            },
        )
    }

    /// A sword cut on the chest bleeds seven body ticks after the blow, as
    /// a smear along the blade's follow-through: the blade moved on 20 cm
    /// along +x, so the wound centres 10 cm behind the contact point and
    /// soaks the strip ahead to its end. Only the struck body is painted.
    #[test]
    fn hits_bleed_seven_ticks_later() {
        use exa_combat::Fighter;
        use exa_combat::wounds::WoundQueue;
        use exa_physics::body::{BodyTicks, Shove};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<Assets<Mesh>>()
            .init_resource::<Assets<SkinnedMeshInverseBindposes>>()
            .init_resource::<BodyTicks>()
            .add_message::<Shove>()
            .add_plugins((exa_combat::CombatPlugin, BloodPlugin));
        let (skinned, rigid) = {
            let mut meshes = app.world_mut().resource_mut::<Assets<Mesh>>();
            (meshes.add(strip(true)), meshes.add(strip(false)))
        };
        let a = body(app.world_mut(), 0.0, &skinned, &rigid);
        let b = body(app.world_mut(), 5.0, &skinned, &rigid);
        for root in [a.root, b.root] {
            app.world_mut().entity_mut(root).insert((
                Fighter::default(),
                exa_anim::Animator::new("Default", "Idle"),
            ));
        }
        app.update();
        let blade = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        let (weapon, strike) = sword_cut();
        app.world_mut().write_message(exa_combat::Hit {
            attacker: b.root,
            striker: blade,
            target: a.root,
            bone: 9,
            point: Vec3::ZERO,
            velocity: Vec3::new(3.0, 0.0, 0.0),
            weapon,
            strike,
        });
        app.update();
        let size = app.world().get::<WoundQueue>(a.root).unwrap().pending()[0].size;
        assert!((size - 0.457).abs() < 1e-3, "{size}");
        let r = 0.3 * size + 0.1;
        // The blade follows through.
        *app.world_mut().get_mut::<GlobalTransform>(blade).unwrap() =
            GlobalTransform::from_xyz(0.2, 0.0, 0.0);
        let tick = |app: &mut App| {
            app.world_mut().resource_mut::<BodyTicks>().0 += 1;
            app.update();
        };
        for t in 1..7 {
            tick(&mut app);
            assert!(alphas(&app, a.chest).is_none(), "tick {t}");
        }
        tick(&mut app);
        let chest = alphas(&app, a.chest).expect("painted on the 7th tick");
        // Ahead of the centre (x ≥ −10 cm): soaked to the strip's end.
        assert!(chest[4..].iter().all(|&v| v == 0), "{chest:?}");
        // Behind it a ball of radius r: 10 cm back f = 1 − 0.01/r²,
        // 20 cm back f = 1 − 0.04/r², 30 cm back outside.
        let range = |d: f32| {
            let f = 1.0 - d * d / (r * r);
            (255.0 * (1.0 - f)).floor() as u8..=(255.0 * (1.0 - f * f * f)).ceil() as u8
        };
        assert!(range(0.1).contains(&chest[3]), "{chest:?}");
        assert!(range(0.2).contains(&chest[2]), "{chest:?}");
        assert_eq!(chest[..2], [255, 255]);
        // The striker, the helmet and the sword are untouched.
        for part in [b.chest, a.helmet, a.sword, b.helmet] {
            assert!(alphas(&app, part).is_none());
        }
        assert!(
            app.world()
                .get::<WoundQueue>(b.root)
                .unwrap()
                .pending()
                .is_empty()
        );
    }
}

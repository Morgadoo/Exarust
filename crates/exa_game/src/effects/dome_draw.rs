//! The force domes' draw: the Barrier's cap and the Bubble's sphere,
//! wearing the `_ForceBarrier01` material and the `effectbarrier_v` program
//! with `effectbarrier_f` or `effectbubble_f`.
//!
//! - **The meshes** ([`dome_render_mesh`]): the dome's polyhedron (the
//!   Barrier's subdivided cap with y and z exchanged, the Bubble's
//!   subdivided icosahedron) with its points at the dome's radius (100 cm,
//!   120 cm), every face reversed and then every face as built, two-byte
//!   indices.
//! - **The material**: the `_ForceBarrier01` record, emissive
//!   (0.25, 0.25, 0.25, 1), opacity 0.5, blend word 0x163 (ONE,
//!   ONE_MINUS_SRC_ALPHA), flags 0x480008 (one-sided; the mesh draws every
//!   face both ways). Its routine binds the `_ForceNoise` picture on
//!   unit 0 as `EmiTex`, so the domes count as force effects for
//!   [`crate::thaum::effect_draw::ForceNoise`].
//! - **Each frame** (the dome's frame step, at the fixed step's fraction):
//!   the Barrier turns its node to the blended facing at the blended place
//!   and the Bubble keeps its place; the ripple points, the blended share,
//!   the glow and the hum's levels are the frame's
//!   ([`super::force_dome::Barrier::frame`], [`super::force_dome::Dome::frame`]).
//!   The program's uniforms ([`dome_params`]): `ViewPos` the camera carried
//!   into the node's frame, `ColPt` the eight points, `TexPos` the dome's
//!   texture position and `Fade` `max(1 − shown, 0)` under state 3, the
//!   shown share from 3. `ObjMat` is the frame's projection by the node's
//!   frame. The program declares `TexOfs`, which neither fragment stage
//!   reads.
//!
//! Exarust keeps each draw on its own entity ([`DomeDraw`]), placed at the
//! node with the centimetre-to-metre scale and dropped when the dome
//! leaves. The original draws each supersample layer with its own depth
//! term and the node's level of detail (500 over the squared camera
//! distance) offsetting the layer's texture row; Exarust draws once a
//! frame. Rules: `docs/RENDERING.md` (Force effects).
//!
//! Evidence: the domes' cast and mesh build, the Barrier's frame, the
//! dome's frame, both geometries' program setups and uniform uploads and
//! the `_ForceBarrier01` material record, read from `Exanima.exe`. [inf:
//! the node's turn from the facing is taken as the keep-out's cap frame,
//! z along the flattened facing and y up.]

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::math::Affine3A;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_assets::polyhedra::Polyhedron;
use exa_render::force_effects::{ForceDomeMaterial, ForceDomeParams};

use super::force_dome::{Dome, Phase, barrier_polyhedron, bubble_polyhedron, dome_mesh};
use crate::thaum::barrier_live::{BARRIER_RADIUS, LiveBarrier, cap_frame};
use crate::thaum::bubble_live::{BUBBLE_RADIUS, LiveBubble};
use crate::thaum::dome_sound::DomeHum;
use crate::thaum::effect_draw::ForceNoise;

/// Centimetres a metre.
const CM: f32 = 100.0;
/// The dome draws' names, the material the original binds.
pub const MATERIAL: &str = crate::thaum::force::FORCE_BARRIER_MATERIAL;

/// A dome's mesh: `poly`'s points at `radius` (cm), every face reversed and
/// then every face as built, with two-byte indices.
pub fn dome_render_mesh(poly: &Polyhedron, radius: f32) -> Mesh {
    let (points, faces) = dome_mesh(poly, radius);
    let indices = faces.iter().flatten().map(|&i| i as u16).collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_indices(Indices::U16(indices))
}

/// The program's uniforms for `dome`, the camera `view_cm` in the node's
/// frame.
pub fn dome_params(dome: &Dome, view_cm: Vec3) -> ForceDomeParams {
    ForceDomeParams {
        view_pos: view_cm.extend(0.0),
        tex_pos: Vec4::new(dome.tex_pos[0], dome.tex_pos[1], 0.0, 0.0),
        colour_points: dome.ripples.points.map(Vec4::from_array),
        fade: dome.fade(),
    }
}

/// The two shared meshes, made on first use.
#[derive(Default)]
pub(crate) struct DomeMeshes {
    barrier: Option<Handle<Mesh>>,
    bubble: Option<Handle<Mesh>>,
}

/// A dome's draw entity; it names its dome.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DomeDraw(pub Entity);

/// On a dome: its draw entity, material and the phase last logged.
#[derive(Component, Debug, Clone)]
pub struct DomeDrawn {
    pub draw: Entity,
    pub material: Handle<ForceDomeMaterial>,
    pub logged: Phase,
}

/// The node's frame a draw stands in: the place and turn, in metres, with
/// the centimetre-to-metre scale.
fn node_transform(frame: Affine3A) -> Transform {
    let (_, rotation, translation) = frame.to_scale_rotation_translation();
    Transform {
        translation: translation / CM,
        rotation,
        scale: Vec3::splat(1.0 / CM),
    }
}

/// Puts this frame's uniforms on a dome's draw, or makes the draw.
#[allow(clippy::too_many_arguments)]
fn show_dome(
    commands: &mut Commands,
    entity: Entity,
    bubble: bool,
    dome: &Dome,
    frame: Affine3A,
    camera_cm: Vec3,
    drawn: Option<Mut<DomeDrawn>>,
    noise: &Handle<Image>,
    mesh: impl FnOnce() -> Handle<Mesh>,
    draws: &mut Query<&mut Transform, With<DomeDraw>>,
    materials: &mut Assets<ForceDomeMaterial>,
) {
    let kind = if bubble {
        "force bubble"
    } else {
        "force barrier"
    };
    let params = dome_params(dome, frame.inverse().transform_point3(camera_cm));
    let transform = node_transform(frame);
    match drawn {
        Some(mut drawn) => {
            if let Some(mut m) = materials.get_mut(&drawn.material) {
                m.params = params;
            }
            if let Ok(mut t) = draws.get_mut(drawn.draw) {
                *t = transform;
            }
            if drawn.logged != dome.phase {
                drawn.logged = dome.phase;
                info!(
                    "{kind} {entity:?}: {:?}, share {:.3} shown {:.3}: fade {:.3}, glow {:.3}, ripples {}",
                    dome.phase,
                    dome.share,
                    dome.shown_share,
                    params.fade,
                    dome.glow,
                    dome.ripples.list.len()
                );
            }
        }
        None => {
            let material = materials.add(ForceDomeMaterial {
                params,
                noise: noise.clone(),
                bubble,
            });
            let draw = commands
                .spawn((
                    Name::new(MATERIAL),
                    DomeDraw(entity),
                    Mesh3d(mesh()),
                    MeshMaterial3d(material.clone()),
                    transform,
                    Visibility::Visible,
                    NoFrustumCulling,
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ))
                .id();
            info!(
                "{kind} {entity:?}: drawn by {draw:?} at {:?} cm, {:?}, fade {:.3}",
                frame.translation, dome.phase, params.fade
            );
            commands.entity(entity).insert(DomeDrawn {
                draw,
                material,
                logged: dome.phase,
            });
        }
    }
}

/// Each frame: every Barrier and Bubble runs its frame step and its draw
/// takes the frame's uniforms; the hums take the frame's levels. Nothing
/// is drawn until the noise picture exists.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_domes(
    mut commands: Commands,
    noise: Res<ForceNoise>,
    time: Res<Time<Fixed>>,
    mut shared: Local<DomeMeshes>,
    mut barriers: Query<(Entity, &mut LiveBarrier, Option<&mut DomeDrawn>), Without<LiveBubble>>,
    mut bubbles: Query<(Entity, &mut LiveBubble, Option<&mut DomeDrawn>), Without<LiveBarrier>>,
    mut draws: Query<&mut Transform, With<DomeDraw>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ForceDomeMaterial>>,
) {
    let Some(image) = noise.image.clone() else {
        return;
    };
    let camera = cameras
        .iter()
        .next()
        .map_or(Vec3::ZERO, |c| c.translation() * CM);
    let alpha = time.overstep_fraction();
    for (entity, mut live, drawn) in &mut barriers {
        let live = &mut *live;
        let Some(frame) = live.barrier.frame(alpha, camera) else {
            continue;
        };
        live.sounds.hum = Some(DomeHum {
            at: frame.hum_at,
            volume: frame.dome.hum[0],
            pitch: frame.dome.hum[1],
        });
        let node = cap_frame(frame.at, frame.facing);
        show_dome(
            &mut commands,
            entity,
            false,
            &live.barrier.dome,
            node,
            camera,
            drawn,
            &image,
            || {
                shared
                    .barrier
                    .get_or_insert_with(|| {
                        meshes.add(dome_render_mesh(&barrier_polyhedron(), BARRIER_RADIUS))
                    })
                    .clone()
            },
            &mut draws,
            &mut materials,
        );
    }
    for (entity, mut live, drawn) in &mut bubbles {
        let live = &mut *live;
        let Some(frame) = live.bubble.dome.frame(alpha, camera) else {
            continue;
        };
        let at = live.bubble.dome.at;
        live.sounds.hum = Some(DomeHum {
            at,
            volume: frame.hum[0],
            pitch: frame.hum[1],
        });
        show_dome(
            &mut commands,
            entity,
            true,
            &live.bubble.dome,
            Affine3A::from_translation(at),
            camera,
            drawn,
            &image,
            || {
                shared
                    .bubble
                    .get_or_insert_with(|| {
                        meshes.add(dome_render_mesh(&bubble_polyhedron(), BUBBLE_RADIUS))
                    })
                    .clone()
            },
            &mut draws,
            &mut materials,
        );
    }
}

/// Either live dome.
type LiveDome = Or<(With<LiveBarrier>, With<LiveBubble>)>;

/// The domes' release: the draws of domes that have left the scene go.
pub(crate) fn drop_dome_draws(
    mut commands: Commands,
    draws: Query<(Entity, &DomeDraw)>,
    domes: Query<(), LiveDome>,
) {
    for (entity, DomeDraw(dome)) in &draws {
        if !domes.contains(*dome) {
            info!("dome {dome:?}: left the scene, its draw {entity:?} goes");
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::arena::Mt19937;
    use crate::effects::force_dome::{Barrier, Bubble};

    #[test]
    fn both_meshes_are_their_polyhedra_at_the_radius_both_ways() {
        let mesh = dome_render_mesh(&bubble_polyhedron(), BUBBLE_RADIUS);
        let Some(Indices::U16(indices)) = mesh.indices() else {
            panic!("two-byte indices");
        };
        assert_eq!(indices.len(), 1280 * 2 * 3);
        let (reversed, built) = indices.split_at(1280 * 3);
        for (r, b) in reversed.chunks(3).zip(built.chunks(3)) {
            assert_eq!([r[2], r[1], r[0]], [b[0], b[1], b[2]]);
        }
        let cap = dome_render_mesh(&barrier_polyhedron(), BARRIER_RADIUS);
        let points = cap
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap();
        let Some(Indices::U16(indices)) = cap.indices() else {
            panic!("two-byte indices");
        };
        // Every point the faces use stands on the cap, facing +z, at the
        // full radius (the subdivision leaves some points unused).
        for &i in indices {
            let p = Vec3::from_array(points[usize::from(i)]);
            assert!((p.length() - BARRIER_RADIUS).abs() < 1e-2);
            assert!(p.z >= -1e-2);
        }
    }

    #[test]
    fn the_uniforms_take_the_points_texture_place_and_fade() {
        let (mut bubble, _) =
            Bubble::cast(120.0, Vec3::ZERO, 0, Vec3::ZERO, 0.0, &mut Mt19937::new(3));
        bubble.dome.ripples.strike(Vec3::new(0.0, 120.0, 0.0), 1.0);
        bubble.dome.ripples.tick();
        bubble.dome.shown_share = 0.25;
        bubble.dome.frame(0.5, Vec3::new(0.0, 0.0, 500.0));
        let params = dome_params(&bubble.dome, Vec3::new(0.0, 0.0, 500.0));
        assert_eq!(params.view_pos, Vec4::new(0.0, 0.0, 500.0, 0.0));
        assert_eq!(
            params.tex_pos.truncate().truncate(),
            Vec2::from_array(bubble.dome.tex_pos)
        );
        assert_eq!(
            params.colour_points[0],
            Vec4::from_array(bubble.dome.ripples.points[0])
        );
        assert_eq!(params.colour_points[1].w, 0.0);
        assert_eq!(params.fade, bubble.dome.fade());
    }

    #[test]
    fn the_barrier_node_turns_to_the_facing_with_the_metre_scale() {
        let (mut barrier, _) = Barrier::cast(
            100.0,
            Vec3::new(100.0, 0.0, 0.0),
            0.0,
            0,
            &mut Mt19937::new(3),
        );
        barrier.tick(Vec3::new(100.0, 0.0, 0.0), 0.0, Vec3::X, 12);
        barrier.snapshot(1);
        barrier.snapshot(2);
        let frame = barrier.frame(1.0, Vec3::ZERO).unwrap();
        let t = node_transform(cap_frame(frame.at, frame.facing));
        assert!((t.translation - frame.at / CM).length() < 1e-5);
        // The cap's +z axis is the facing.
        assert!((t.rotation * Vec3::Z - Vec3::X).length() < 1e-3);
        assert_eq!(t.scale, Vec3::splat(0.01));
    }
}

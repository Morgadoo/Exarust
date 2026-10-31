//! The teleport bubble's draw: its sphere, wearing the `_TeleBubble01`
//! program (`effecttele_v`/`effecttele_f`), and a debug helper that puts a
//! bubble on the player.
//!
//! The bubble's mesh is the unit icosahedron subdivided three times, its
//! points scaled to the bubble's 120 cm, with an index list of every face
//! reversed and then every face as built (two-byte indices). Its material
//! record is built in once per scene: emissive (0.25, 0.25, 0.25, 1),
//! opacity 0.5, blend word 0x163, flags 0x480008 (one-sided), the
//! `ForceEfctA2E` emissive layer, and a draw routine of its own. That
//! routine binds the `_ForceNoise` picture on unit 0 (so the record's own
//! layer is never sampled), uses the program, and sets its uniforms:
//!
//! - `ObjMat`: the frame's projection by the bubble node's frame (each
//!   supersample layer's own); `ViewPos`: the camera carried into the
//!   node's frame;
//! - `TexPos`: the bubble's two creation draws;
//! - `Fade`, `Scale`, `Colr`, `Pulse`, `Anim`: [`Bubble::uniforms`].
//!
//! It also sets `TexOfs` per layer, which the program never reads. The
//! bubble's step makes the noise picture when the bubble is made and ticks
//! it every step, so the bubble counts as a force effect for
//! [`crate::thaum::effect_draw::ForceNoise`].
//!
//! Exarust keeps the draw on its own entity ([`BubbleDraw`]), placed at the
//! bubble with the centimetre-to-metre scale and dropped when the bubble
//! leaves. Rules: `docs/RENDERING.md` (Force effects).

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use exa_render::force_effects::{TeleBubbleMaterial, TeleBubbleParams};

use super::bubble::{Bubble, BubbleEffect, BubbleState, MATERIAL, SIZE_CM};
use super::force_dome::{bubble_polyhedron, dome_mesh};
use super::{EffectClock, span_ms};
use crate::thaum::effect_draw::ForceNoise;

/// Centimetres a metre.
const CM: f32 = 100.0;

/// The bubble's sphere: the subdivided icosahedron at 120 cm, every face
/// reversed and then every face as built, with two-byte indices.
pub fn bubble_mesh() -> Mesh {
    let (points, faces) = dome_mesh(&bubble_polyhedron(), SIZE_CM);
    let indices = faces.iter().flatten().map(|&i| i as u16).collect();
    Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD,
    )
    .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, points)
    .with_inserted_indices(Indices::U16(indices))
}

/// The program's uniforms for `bubble` at `now_ms`, the camera `view_cm`
/// in the bubble's frame.
pub fn bubble_params(bubble: &Bubble, now_ms: u32, view_cm: Vec3) -> TeleBubbleParams {
    let u = bubble.uniforms(now_ms);
    TeleBubbleParams {
        view_pos: view_cm.extend(0.0),
        tex_pos: Vec4::new(u.tex_pos[0], u.tex_pos[1], 0.0, 0.0),
        scale: u.scale,
        fade: u.fade,
        pulse: u.pulse,
        anim: u.tex_offset,
        colr: u.colour,
    }
}

/// The shared sphere, made on first use.
#[derive(Resource, Default)]
pub struct BubbleMesh(Option<Handle<Mesh>>);

/// A bubble's draw entity; it names its bubble.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct BubbleDraw(pub Entity);

/// On a bubble: its draw entity, material and the phase last logged.
#[derive(Component, Debug, Clone)]
pub struct BubbleDrawn {
    pub draw: Entity,
    pub material: Handle<TeleBubbleMaterial>,
    pub logged: BubbleState,
}

/// Each frame: a bubble without a draw gets one; every draw follows its
/// bubble and takes this frame's uniforms. Nothing is drawn until the noise
/// picture exists.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_bubbles(
    mut commands: Commands,
    noise: Res<ForceNoise>,
    clock: Option<Res<EffectClock>>,
    mut sphere: Local<BubbleMesh>,
    mut bubbles: Query<(
        Entity,
        &BubbleEffect,
        &GlobalTransform,
        Option<&mut BubbleDrawn>,
    )>,
    mut draws: Query<(&mut Transform, &mut Visibility), With<BubbleDraw>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<TeleBubbleMaterial>>,
) {
    let (Some(image), Some(clock)) = (noise.image.clone(), clock) else {
        return;
    };
    let camera = cameras
        .iter()
        .next()
        .map_or(Vec3::ZERO, |c| c.translation());
    let now = clock.now_ms;
    for (entity, BubbleEffect(bubble), at, drawn) in &mut bubbles {
        let at = at.translation();
        let params = bubble_params(bubble, now, (camera - at) * CM);
        let shown = bubble.state != BubbleState::Gone;
        match drawn {
            Some(mut drawn) => {
                if let Some(mut m) = materials.get_mut(&drawn.material) {
                    m.params = params;
                }
                if let Ok((mut transform, mut visibility)) = draws.get_mut(drawn.draw) {
                    transform.translation = at;
                    *visibility = if shown {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };
                }
                if drawn.logged != bubble.state {
                    drawn.logged = bubble.state;
                    info!(
                        "bubble {entity:?}: {:?} at {} ms (phase {} ms): fade {:.3} scale {:.2} colr {:.2} pulse {:.3}",
                        bubble.state,
                        now,
                        span_ms(now, bubble.stamp),
                        params.fade,
                        params.scale,
                        params.colr,
                        params.pulse
                    );
                }
            }
            None => {
                let mesh = sphere
                    .0
                    .get_or_insert_with(|| meshes.add(bubble_mesh()))
                    .clone();
                let material = materials.add(TeleBubbleMaterial {
                    params,
                    noise: image.clone(),
                });
                let draw = commands
                    .spawn((
                        Name::new(MATERIAL),
                        BubbleDraw(entity),
                        Mesh3d(mesh),
                        MeshMaterial3d(material.clone()),
                        Transform::from_translation(at).with_scale(Vec3::splat(1.0 / CM)),
                        Visibility::Visible,
                        NoFrustumCulling,
                        bevy::light::NotShadowCaster,
                        bevy::light::NotShadowReceiver,
                    ))
                    .id();
                info!(
                    "bubble {entity:?}: drawn by {draw:?} at {at:?}, {:?}, hold {}",
                    bubble.state, bubble.hold
                );
                commands.entity(entity).insert(BubbleDrawn {
                    draw,
                    material,
                    logged: bubble.state,
                });
            }
        }
    }
}

/// The bubble's release: the draws of bubbles that have left the scene go.
pub(crate) fn drop_bubble_draws(
    mut commands: Commands,
    draws: Query<(Entity, &BubbleDraw)>,
    bubbles: Query<(), With<BubbleEffect>>,
) {
    for (entity, BubbleDraw(bubble)) in &draws {
        if !bubbles.contains(*bubble) {
            info!("bubble {bubble:?}: freed, its draw {entity:?} goes");
            commands.entity(entity).despawn();
        }
    }
}

/// A scripted bubble (`--debug-bubble frame[,hold]`): at a frame after the
/// load a bubble is made around the player (98 cm above its feet, as a
/// journey's bubble rises above the teleporter), held (it charges, swells and is
/// freed after 7 s) or not (it rests and shrinks away).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugBubble(pub Option<DebugBubbleAt>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DebugBubbleAt {
    pub frame: u32,
    pub hold: bool,
}

impl DebugBubbleAt {
    /// Reads `frame` or `frame,hold`.
    pub fn parse(text: &str) -> Option<Self> {
        let (frame, hold) = match text.split_once(',') {
            Some((frame, "hold")) => (frame, true),
            Some(_) => return None,
            None => (text, false),
        };
        Some(Self {
            frame: frame.trim().parse().ok()?,
            hold,
        })
    }
}

pub(crate) fn debug_bubble(
    mut commands: Commands,
    script: Res<DebugBubble>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    clock: Res<EffectClock>,
    players: Query<&GlobalTransform, With<crate::player::PlayerBody>>,
    roll: Option<ResMut<crate::thaum::live::EffectRoll>>,
    mut frames: Local<u32>,
) {
    let Some(at) = script.0 else {
        return;
    };
    if loading.is_some() || *frames > at.frame {
        return;
    }
    *frames += 1;
    if *frames != at.frame {
        return;
    }
    let Some(player) = players.iter().next() else {
        info!("debug bubble: no player at frame {}", at.frame);
        return;
    };
    let tex_pos = match roll {
        Some(mut roll) => [roll.0.unit(), roll.0.unit()],
        None => [0.0, 0.0],
    };
    // A journey's bubble rises 98 cm above the teleporter the user stands
    // on; the debug bubble rises as far above the player's feet.
    let place = player.translation() + Vec3::Y * (exa_ui::selection::BUBBLE_LIFT_CM / CM);
    let bubble = commands
        .spawn((
            Name::new("debug bubble"),
            BubbleEffect(Bubble::new(clock.now_ms, at.hold, tex_pos)),
            Transform::from_translation(place),
        ))
        .id();
    info!(
        "debug bubble: {bubble:?} around the player at {place:?}, {} ms, hold {}",
        clock.now_ms, at.hold
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sphere_is_the_subdivided_icosahedron_both_ways() {
        let mesh = bubble_mesh();
        let Some(Indices::U16(indices)) = mesh.indices() else {
            panic!("two-byte indices");
        };
        // 20·4³ faces, twice.
        assert_eq!(indices.len(), 1280 * 2 * 3);
        // The first half is the second reversed.
        let (reversed, built) = indices.split_at(1280 * 3);
        for (r, b) in reversed.chunks(3).zip(built.chunks(3)) {
            assert_eq!([r[2], r[1], r[0]], [b[0], b[1], b[2]]);
        }
        let points = mesh
            .attribute(Mesh::ATTRIBUTE_POSITION)
            .and_then(|a| a.as_float3())
            .unwrap();
        assert_eq!(points.len(), 642);
        for p in points {
            assert!((Vec3::from_array(*p).length() - SIZE_CM).abs() < 1e-3);
        }
    }

    #[test]
    fn the_uniforms_name_the_programs_slots() {
        let mut bubble = Bubble::new(0, true, [0.25, 0.75]);
        bubble.state = BubbleState::Hold;
        bubble.fade = 1.0;
        bubble.charge = 0.9;
        let params = bubble_params(&bubble, 100, Vec3::new(0.0, 0.0, 300.0));
        let u = bubble.uniforms(100);
        assert_eq!(params.view_pos, Vec4::new(0.0, 0.0, 300.0, 0.0));
        assert_eq!(params.tex_pos, Vec4::new(0.25, 0.75, 0.0, 0.0));
        assert_eq!(params.fade, 0.0);
        assert_eq!(params.scale, u.scale);
        assert_eq!(params.colr, u.colour);
        assert_eq!(params.pulse, 0.9);
        assert_eq!(params.anim, u.tex_offset);
        assert!(params.scale > 1.0 && params.colr < 1.0);
    }

    #[test]
    fn a_debug_bubble_reads_its_frame_and_hold() {
        assert_eq!(
            DebugBubbleAt::parse("60"),
            Some(DebugBubbleAt {
                frame: 60,
                hold: false
            })
        );
        assert_eq!(
            DebugBubbleAt::parse("90,hold"),
            Some(DebugBubbleAt {
                frame: 90,
                hold: true
            })
        );
        assert_eq!(DebugBubbleAt::parse("90,x"), None);
        assert_eq!(DebugBubbleAt::parse("x"), None);
    }
}

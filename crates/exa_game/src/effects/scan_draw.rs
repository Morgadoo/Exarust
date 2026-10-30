//! The teleporter scan's draw: the scan aura over a copy of the scanned
//! body's skinned meshes, and its release when the scan ends.
//!
//! A scan node is a child of the body it scans. Its aura material
//! (`_ScanAura01`, blend word 0x163, one-sided) runs `effectshell_v` with
//! `effectscan_f` over the body's skinned copy (each vertex 2 cm out along
//! its normal, refreshed with the skin), with `EmiTex` the shared
//! `_ForceNoise` picture the device's start binds. Its uniforms each frame:
//!
//! - `LclMat`: the inverse of the scanned body node's frame; `ViewPos` the
//!   camera carried into it;
//! - `FdPln`: the body node's up row over the skin box's half height, its
//!   offset putting the plane at the body's feet (plane scale 1);
//! - `Colour`: (0.45, 0.95, 0.33, 1) times the colour scale (3 from the
//!   device, then three times the shown fraction through the looping
//!   close: the flash);
//! - `Fade`: one less the shown fraction.
//!
//! `TexPos` is never set by the scan's program setup, so it stays zero.
//! When the scan reaches its end state it lets the body's copy refresh go,
//! frees a light hung on it and leaves the scene
//! ([`super::scan::tick_scans`] removes the node; its copies go with it).
//!
//! Exarust copies each skinned mesh (body and apparel) into one entity
//! wearing the scan's [`ForceShellMaterial`] in its scan form.
//! Rules: `docs/RENDERING.md` (Force effects).

use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_render::force_effects::{ForceShellMaterial, ForceShellParams, SHELL_PUSH_M};

use super::scan::{PLANE_SCALE, Scan, ScanEffect, ScanState};
use super::shell::{PlaneBounds, fade_plane};
use crate::thaum::effect_draw::ForceNoise;
use crate::thaum::vanish_live::{body_bounds, body_frame, frame_matrix};

/// Centimetres a metre.
const CM: f32 = 100.0;

/// A copy of one of the scanned body's skinned meshes; it names its scan.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScanCopy(pub Entity);

/// A scan's material, once its copies are made.
#[derive(Component, Debug, Clone)]
pub struct ScanDraw(pub Handle<ForceShellMaterial>);

/// The scan program's uniform this frame over the body node's `frame` and
/// skin `bounds`, with the camera at `camera_m`.
pub fn scan_params(
    scan: &Scan,
    frame: &super::Frame,
    bounds: PlaneBounds,
    camera_m: Vec3,
) -> ForceShellParams {
    let local = frame_matrix(frame).inverse();
    let uniforms = scan.uniforms(frame, bounds);
    debug_assert_eq!(uniforms.plane, fade_plane(frame, bounds, PLANE_SCALE));
    ForceShellParams {
        local_from_world: local * Mat4::from_scale(Vec3::splat(CM)),
        view_pos: local.transform_point3(camera_m * CM).extend(0.0),
        fade_plane: Vec4::from_array(uniforms.plane),
        colour_points: [Vec4::ZERO; 8],
        // The scan's program setup never sets `TexPos`.
        tex_pos: Vec4::ZERO,
        colour_int: 0.0,
        tex_scale: super::scan::UNUSED,
        fade: uniforms.fade,
        push: SHELL_PUSH_M,
        gradient: Vec4::ZERO,
        colour: Vec4::from_array(uniforms.colour),
    }
}

/// Each frame: a scan without copies copies its body's skinned meshes;
/// every scan's material takes this frame's uniforms. A gone scan draws
/// nothing.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_scans(
    mut commands: Commands,
    noise: Res<ForceNoise>,
    scans: Query<(Entity, &ScanEffect, &ChildOf, Option<&ScanDraw>)>,
    bodies: Query<(Option<&Ragdoll>, &GlobalTransform)>,
    children: Query<&Children>,
    skinned: Query<
        (&Mesh3d, &SkinnedMesh, Option<&Visibility>),
        (
            Without<ScanCopy>,
            Without<crate::thaum::vanish_live::VanishCopy>,
            Without<crate::thaum::effect_draw::ShellCopy>,
        ),
    >,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    mut materials: ResMut<Assets<ForceShellMaterial>>,
) {
    let Some(image) = noise.image.clone() else {
        return;
    };
    let camera = cameras
        .iter()
        .next()
        .map_or(Vec3::ZERO, |c| c.translation());
    for (entity, ScanEffect(scan), parent, drawn) in &scans {
        if scan.state == ScanState::Gone {
            continue;
        }
        let body = parent.parent();
        let Ok((ragdoll, node)) = bodies.get(body) else {
            continue;
        };
        let frame = body_frame(ragdoll, node);
        let params = scan_params(scan, &frame, body_bounds(ragdoll, &frame), camera);
        match drawn {
            Some(ScanDraw(material)) => {
                if let Some(mut m) = materials.get_mut(material) {
                    m.params = params;
                }
            }
            None => {
                let sources: Vec<(Handle<Mesh>, SkinnedMesh)> = children
                    .iter_descendants(body)
                    .filter_map(|e| {
                        let (mesh, skin, visibility) = skinned.get(e).ok()?;
                        (visibility != Some(&Visibility::Hidden))
                            .then(|| (mesh.0.clone(), skin.clone()))
                    })
                    .collect();
                if sources.is_empty() {
                    debug!("scan {entity:?}: its body has no skinned mesh yet");
                    continue;
                }
                info!(
                    "scan {entity:?}: aura over {} skinned meshes of {body:?}",
                    sources.len()
                );
                let material = materials.add(ForceShellMaterial {
                    params,
                    noise: image.clone(),
                    strikes: false,
                    scan: true,
                });
                for (mesh, skin) in &sources {
                    commands.spawn((
                        Name::new("_ScanAura01 copy"),
                        ScanCopy(entity),
                        Mesh3d(mesh.clone()),
                        skin.clone(),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        Visibility::Inherited,
                        NoFrustumCulling,
                        bevy::light::NotShadowCaster,
                        bevy::light::NotShadowReceiver,
                        ChildOf(body),
                    ));
                }
                commands.entity(entity).insert(ScanDraw(material));
            }
        }
    }
}

/// The scan's release: the copies of a scan that has left the scene go.
pub(crate) fn drop_scan_copies(
    mut commands: Commands,
    copies: Query<(Entity, &ScanCopy)>,
    scans: Query<(), With<ScanEffect>>,
) {
    for (entity, ScanCopy(scan)) in &copies {
        if !scans.contains(*scan) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::Frame;

    #[test]
    fn the_scan_uniform_carries_its_colour_scale_and_fade() {
        let mut scan = Scan::delayed(0, [0.5, 0.25]);
        scan.colour_scale = 3.0;
        scan.shown = 0.25;
        let frame = Frame {
            at: Vec3::new(0.0, 100.0, 0.0),
            ..Frame::IDENTITY
        };
        let bounds = PlaneBounds::Skinned {
            centre_y: 140.0,
            half_y: 40.0,
        };
        let params = scan_params(&scan, &frame, bounds, Vec3::new(0.0, 1.0, 2.0));
        assert_eq!(params.fade, 0.75);
        let expected = Vec4::new(0.45, 0.95, 0.33, 1.0) * 3.0;
        assert!((params.colour - expected).length() < 1e-6);
        assert_eq!(params.tex_pos, Vec4::ZERO);
        assert_eq!(params.colour_points, [Vec4::ZERO; 8]);
        assert_eq!(params.fade_plane, Vec4::new(0.0, 0.025, 0.0, 0.0));
        assert_eq!(params.push, 0.02);
        assert!((params.view_pos.truncate() - Vec3::new(0.0, 0.0, 200.0)).length() < 1e-3);
    }
}

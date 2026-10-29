//! The Force effects' GPU submission: the shared `_ForceNoise` picture
//! ([`ForceNoise`]), the blast-family and torrent ribbons' material
//! ([`show_ribbon`]) and the force-shell aura's skinned copies
//! ([`draw_shell_auras`]), drawn with `exa_render::force_effects`.
//!
//! - **The noise picture** is made the first frame a force effect (a shell
//!   aura, a blast, bolt or wave, a torrent) exists, from the engine's noise
//!   object ([`super::visuals::noise_field`]) seeded from the engine's
//!   generator, and uploaded again (level 0 and its mip chain, the parameter
//!   word's bit 0x40) only when the object's tick reports a new picture
//!   ([`super::effect_material::noise_upload`]). With no force effect alive
//!   the system does not run, so the frame costs nothing.
//! - **The ribbons** wear `ForceRibbonMaterial` (`effectforce_f` for the
//!   blast family, `effecttorrent_f` for the torrent) with the frame's
//!   `TexOfs` and the noise picture as `EmiTex`; the mesh carries the second
//!   coordinate set (`vTx2`).
//! - **The shell aura** draws a copy of each skinned mesh of its caster's
//!   body (its body and apparel), sharing the source's skin, with one
//!   `ForceShellMaterial` an aura. Each frame its uniforms come from
//!   [`super::super::thaum::shell_uniforms`] for a skinned copy: the node's
//!   frame is unturned at the body's root (cm), the skin box is the body
//!   parts' spheres' height span, and the camera is carried into that frame.
//!   A copy whose aura has left the scene is removed with it.
//!
//! Rules: `docs/THAUMATURGY.md` (The shaped-force aura) and
//! `docs/RENDERING.md` (Force effects).

use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use exa_physics::body::{BodyTicks, Ragdoll};
use exa_render::force_effects::{
    ForceRibbonMaterial, ForceShellMaterial, ForceShellParams, SHELL_PUSH_M, noise_image,
    set_noise_pixels,
};
use exa_render::image_noise::NoiseField;

use super::blast_live::LiveBlast;
use super::cast::LiveShellAura;
use super::effect_material::{NOISE_SIDE, NoiseTexture, noise_upload};
use super::live::{EffectMesh, EffectRoll};
use super::torrent_live::LiveTorrent;
use super::visuals::{noise_bytes, noise_field};
use crate::thaum::{ShellBox, ShellFrame, ShellUniforms, shell_uniforms};

/// Centimetres a metre.
const CM: f32 = 100.0;

/// The `_ForceNoise` picture the Force effects bind as `EmiTex`: the
/// engine's noise object and its texture, made on first use.
#[derive(Resource, Default)]
pub struct ForceNoise {
    field: Option<NoiseField>,
    /// The texture, once made.
    pub image: Option<Handle<Image>>,
    /// The uploads so far (the creation counts as the first).
    pub uploads: u32,
}

/// The force effects that bind the noise picture.
type ForceEffect = Or<(With<LiveShellAura>, With<LiveBlast>, With<LiveTorrent>)>;

/// Whether any force effect that binds the noise picture is alive.
pub(crate) fn force_effects_alive(effects: Query<(), ForceEffect>) -> bool {
    !effects.is_empty()
}

/// Makes the noise picture on first use and uploads it again whenever the
/// noise object's tick reports a new picture.
pub(crate) fn tick_force_noise(
    mut noise: ResMut<ForceNoise>,
    mut images: ResMut<Assets<Image>>,
    mut roll: ResMut<EffectRoll>,
    ticks: Option<Res<BodyTicks>>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    let texture = NoiseTexture::default();
    let noise = &mut *noise;
    let Some(field) = noise.field.as_mut() else {
        let field = noise_field(now, || roll.0.step());
        let pixels = noise_bytes(&field);
        noise.image = Some(images.add(noise_image(&pixels, NOISE_SIDE, texture.mipmaps())));
        noise.field = Some(field);
        noise.uploads = 1;
        return;
    };
    let fresh = field.tick(now, || roll.0.step());
    if !fresh {
        return;
    }
    let pixels = noise_bytes(field);
    let Some(upload) = noise_upload(&texture, fresh, &pixels) else {
        return;
    };
    if let Some(handle) = &noise.image
        && let Some(mut image) = images.get_mut(handle)
    {
        set_noise_pixels(&mut image, &pixels, upload.width, upload.mipmaps);
        noise.uploads += 1;
    }
}

/// The material a ribbon wears.
#[derive(Component)]
pub struct RibbonMaterial(pub Handle<ForceRibbonMaterial>);

/// Draws a force ribbon: the first frame spawns its mesh and material, later
/// frames replace the mesh and the material's `TexOfs`. Nothing is drawn
/// until the noise picture exists.
#[allow(clippy::too_many_arguments)]
pub(crate) fn show_ribbon(
    commands: &mut Commands,
    entity: Entity,
    drawn: Option<(&EffectMesh, &RibbonMaterial)>,
    mesh: Mesh,
    tex_ofs: Vec3,
    torrent: bool,
    noise: &ForceNoise,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<ForceRibbonMaterial>,
) {
    let Some(image) = noise.image.clone() else {
        return;
    };
    match drawn {
        Some((EffectMesh(mesh_handle), RibbonMaterial(material))) => {
            if let Some(mut m) = meshes.get_mut(mesh_handle) {
                *m = mesh;
            }
            if let Some(mut m) = materials.get_mut(material)
                && m.params.tex_ofs.truncate() != tex_ofs
            {
                m.params.tex_ofs = tex_ofs.extend(0.0);
            }
        }
        None => {
            let mesh_handle = meshes.add(mesh);
            let material = materials.add(ForceRibbonMaterial::new(image, tex_ofs, torrent));
            commands.entity(entity).insert((
                EffectMesh(mesh_handle.clone()),
                RibbonMaterial(material.clone()),
                Mesh3d(mesh_handle),
                MeshMaterial3d(material),
                Transform::IDENTITY,
                NoFrustumCulling,
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
            ));
        }
    }
}

/// The body whose skin a shell aura copies.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellAuraOf(pub Entity);

/// A shell aura's material, once its copies are made.
#[derive(Component, Debug, Clone)]
pub struct ShellAuraDraw {
    pub material: Handle<ForceShellMaterial>,
    pub copies: usize,
}

/// A copy of a skinned mesh drawn with its aura's material; it names its
/// aura.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShellCopy(pub Entity);

/// A shell frame as the matrix the program's `LclMat` is: the rows are the
/// frame's axes (columns of the matrix) and the translation its origin, so
/// the inverse frame's matrix takes world points into the frame.
pub fn frame_matrix(frame: &ShellFrame) -> Mat4 {
    let [r0, r1, r2] = frame.rows;
    Mat4::from_cols(
        r0.extend(0.0),
        r1.extend(0.0),
        r2.extend(0.0),
        frame.translation.extend(1.0),
    )
}

/// The shell program's uniform from the CPU model's uniforms: `LclMat`
/// composed with the metre-to-centimetre scale (Exarust's world is in
/// metres, the original's in cm).
pub fn shell_params(uniforms: &ShellUniforms) -> ForceShellParams {
    ForceShellParams {
        local_from_world: frame_matrix(&uniforms.local) * Mat4::from_scale(Vec3::splat(CM)),
        view_pos: uniforms.view_pos.extend(0.0),
        fade_plane: Vec4::from_array(uniforms.fade_plane),
        colour_points: uniforms.colour_points.map(Vec4::from_array),
        tex_pos: Vec4::new(uniforms.tex_pos[0], uniforms.tex_pos[1], 0.0, 0.0),
        colour_int: uniforms.colour_int,
        tex_scale: uniforms.tex_scale,
        fade: uniforms.fade,
        push: SHELL_PUSH_M,
    }
}

/// The skin box a skinned copy's fade plane is measured from: the body
/// parts' spheres' height span (cm), its centre and half height.
pub fn skin_box(body: &Ragdoll) -> Option<ShellBox> {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for part in &body.parts {
        let world = part.transform();
        for &(centre, radius) in &part.spheres {
            let y = world.transform_point3(centre).y;
            low = low.min(y - radius);
            high = high.max(y + radius);
        }
    }
    (low <= high).then_some(ShellBox::Skinned {
        centre_y: (low + high) * 0.5 * CM,
        half_y: (high - low) * 0.5 * CM,
    })
}

/// The aura node's frame: unturned (the shell node is created without a
/// turn) at the body's root (cm).
pub fn aura_frame(body: &Ragdoll) -> Option<ShellFrame> {
    let root = body.parts.first()?.transform().translation;
    Some(ShellFrame {
        rows: [Vec3::X, Vec3::Y, Vec3::Z],
        translation: Vec3::from(root) * CM,
    })
}

/// Each frame: a shell aura without copies copies its caster's skinned
/// meshes; every aura's material takes this frame's uniforms.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_shell_auras(
    mut commands: Commands,
    noise: Res<ForceNoise>,
    auras: Query<(
        Entity,
        &LiveShellAura,
        Option<&ShellAuraOf>,
        Option<&ShellAuraDraw>,
    )>,
    bodies: Query<&Ragdoll>,
    children: Query<&Children>,
    skinned: Query<(&Mesh3d, &SkinnedMesh), Without<ShellCopy>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    mut materials: ResMut<Assets<ForceShellMaterial>>,
) {
    let Some(image) = noise.image.clone() else {
        return;
    };
    let view = cameras
        .iter()
        .next()
        .map_or(Vec3::ZERO, |c| c.translation() * CM);
    for (entity, aura, of, drawn) in &auras {
        let Some(ShellAuraOf(caster)) = of.copied() else {
            continue;
        };
        let Ok(body) = bodies.get(caster) else {
            continue;
        };
        let (Some(frame), Some(bounds)) = (aura_frame(body), skin_box(body)) else {
            continue;
        };
        let params = shell_params(&shell_uniforms(&aura.0, true, &frame, bounds, view));
        match drawn {
            Some(draw) => {
                if let Some(mut material) = materials.get_mut(&draw.material) {
                    material.params = params;
                }
            }
            None => {
                let sources: Vec<(Entity, Handle<Mesh>, SkinnedMesh)> = children
                    .iter_descendants(caster)
                    .filter_map(|e| {
                        let (mesh, skin) = skinned.get(e).ok()?;
                        Some((e, mesh.0.clone(), skin.clone()))
                    })
                    .collect();
                if sources.is_empty() {
                    continue;
                }
                let material = materials.add(ForceShellMaterial {
                    params,
                    noise: image.clone(),
                });
                for (source, mesh, skin) in &sources {
                    // A child of the source mesh: it sorts and hides with it.
                    commands.spawn((
                        Name::new("_ForceAura01 copy"),
                        ShellCopy(entity),
                        Mesh3d(mesh.clone()),
                        skin.clone(),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        NoFrustumCulling,
                        bevy::light::NotShadowCaster,
                        bevy::light::NotShadowReceiver,
                        ChildOf(*source),
                    ));
                }
                commands.entity(entity).insert(ShellAuraDraw {
                    material,
                    copies: sources.len(),
                });
            }
        }
    }
}

/// Removes the copies of an aura that has left the scene.
pub(crate) fn drop_shell_copies(
    mut commands: Commands,
    copies: Query<(Entity, &ShellCopy)>,
    auras: Query<(), With<LiveShellAura>>,
) {
    for (entity, ShellCopy(aura)) in &copies {
        if !auras.contains(*aura) {
            commands.entity(entity).despawn();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::ForceShell;

    #[test]
    fn the_local_matrix_takes_world_metres_into_the_frame_in_cm() {
        let frame = ShellFrame {
            rows: [Vec3::Z, Vec3::Y, -Vec3::X],
            translation: Vec3::new(100.0, 20.0, -50.0),
        };
        let shell = ForceShell::new(0, [0.25, 0.5]);
        let bounds = ShellBox::Skinned {
            centre_y: 90.0,
            half_y: 90.0,
        };
        let uniforms = shell_uniforms(&shell, true, &frame, bounds, Vec3::new(0.0, 160.0, 300.0));
        let params = shell_params(&uniforms);
        for world_m in [
            Vec3::ZERO,
            Vec3::new(1.2, 0.4, -0.3),
            Vec3::new(-2.0, 1.7, 5.0),
        ] {
            let lpos = params.local_from_world.transform_point3(world_m);
            let expected = frame.local(world_m * CM);
            assert!((lpos - expected).length() < 1e-3, "{lpos} vs {expected}");
        }
        assert_eq!(params.view_pos.truncate(), uniforms.view_pos);
        assert_eq!(params.tex_pos, Vec4::new(0.25, 0.5, 0.0, 0.0));
        assert_eq!(params.fade, 1.0);
        assert_eq!(params.colour_int, 1.0);
        assert_eq!(params.tex_scale, 0.015);
        assert_eq!(params.push, 0.02);
        assert_eq!(params.fade_plane, Vec4::from_array(uniforms.fade_plane));
    }

    #[test]
    fn a_ripple_reaches_the_programs_colour_points() {
        let mut shell = ForceShell::new(0, [0.0, 0.0]);
        let mut roll = || 0.5;
        shell.hit([10.0, 20.0, 30.0], 1.0, 1000, &mut roll);
        shell.step_ripples();
        let frame = ShellFrame::IDENTITY;
        let bounds = ShellBox::Skinned {
            centre_y: 90.0,
            half_y: 90.0,
        };
        let params = shell_params(&shell_uniforms(&shell, true, &frame, bounds, Vec3::ZERO));
        assert_eq!(
            params.colour_points[0].truncate(),
            Vec3::new(10.0, 20.0, 30.0)
        );
        assert!(params.colour_points[0].w > 0.0);
        assert_eq!(params.colour_points[1], Vec4::ZERO);
    }

    #[test]
    fn a_shell_aura_copies_its_casters_skinned_meshes_until_it_leaves() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Mesh>()
            .init_asset::<ForceShellMaterial>()
            .init_resource::<ForceNoise>()
            .init_resource::<EffectRoll>()
            .add_systems(
                Update,
                (
                    tick_force_noise.run_if(force_effects_alive),
                    draw_shell_auras,
                    drop_shell_copies,
                )
                    .chain(),
            );
        let body = crate::thaum::blast_live::test_bodies::scattered(
            &mut crate::arena::Mt19937::new(1),
            Vec3::new(0.0, 1.0, 0.0),
            0.5,
        );
        let mesh = app
            .world_mut()
            .resource_mut::<Assets<Mesh>>()
            .add(Mesh::from(Cuboid::default()));
        let caster = app.world_mut().spawn(body).id();
        app.world_mut()
            .spawn((Mesh3d(mesh), SkinnedMesh::default(), ChildOf(caster)));
        let aura = app
            .world_mut()
            .spawn((
                LiveShellAura(ForceShell::new(0, [0.0, 0.0])),
                ShellAuraOf(caster),
            ))
            .id();
        app.update();
        app.update();
        let draw = app
            .world()
            .get::<ShellAuraDraw>(aura)
            .expect("the aura draws")
            .clone();
        assert_eq!(draw.copies, 1);
        let mut copies = app.world_mut().query::<&ShellCopy>();
        assert_eq!(copies.iter(app.world()).count(), 1);
        let materials = app.world().resource::<Assets<ForceShellMaterial>>();
        let params = materials.get(&draw.material).unwrap().params;
        // An opening shell shows nothing yet: the fade is 1.
        assert_eq!(params.fade, 1.0);
        app.world_mut().entity_mut(aura).despawn();
        app.update();
        let mut copies = app.world_mut().query::<&ShellCopy>();
        assert_eq!(copies.iter(app.world()).count(), 0);
    }

    #[test]
    fn the_noise_is_made_once_and_uploaded_when_fresh() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Image>()
            .init_resource::<ForceNoise>()
            .init_resource::<EffectRoll>()
            .insert_resource(BodyTicks::default())
            .add_systems(Update, tick_force_noise.run_if(force_effects_alive));
        app.update();
        assert!(
            app.world().resource::<ForceNoise>().image.is_none(),
            "no force effect: no noise picture"
        );
        app.world_mut()
            .spawn(LiveShellAura(ForceShell::new(0, [0.0, 0.0])));
        app.update();
        let noise = app.world().resource::<ForceNoise>();
        assert_eq!(noise.uploads, 1);
        let handle = noise.image.clone().expect("the picture is made");
        let images = app.world().resource::<Assets<Image>>();
        let image = images.get(&handle).expect("the picture is an image");
        assert_eq!(image.texture_descriptor.mip_level_count, 9);
        // A second second of game time turns the noise object's phase.
        app.world_mut().resource_mut::<BodyTicks>().0 += 100;
        app.update();
        assert_eq!(app.world().resource::<ForceNoise>().uploads, 2);
    }
}

//! The Force effects' GPU submission: the shared `_ForceNoise` picture
//! ([`ForceNoise`]), the blast-family and torrent ribbons' material
//! ([`show_ribbon`]) and the force-shell aura's skinned copies
//! ([`draw_shell_auras`]), drawn with `exa_render::force_effects`.
//!
//! - **The noise picture** is made the first frame a force effect (a shell
//!   aura, a blast, bolt or wave, a torrent, a teleporter scan or bubble) exists, from the engine's noise
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
//!   `ForceShellMaterial` an aura. The copies are made in the caster's own
//!   child order, so they take the parts' order (the original copies part
//!   by part into one index buffer; Exarust keeps one entity a part), each
//!   copy's indices its sections' triangles in order. Each frame its uniforms come from
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
use super::live::{EffectMesh, FrameEffectRoll};
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
type ForceEffect = Or<(
    With<LiveShellAura>,
    With<LiveBlast>,
    With<LiveTorrent>,
    With<crate::effects::scan::ScanEffect>,
    With<crate::effects::bubble::BubbleEffect>,
    With<super::barrier_live::LiveBarrier>,
    With<super::bubble_live::LiveBubble>,
)>;

/// Whether any force effect that binds the noise picture is alive.
pub(crate) fn force_effects_alive(effects: Query<(), ForceEffect>) -> bool {
    !effects.is_empty()
}

/// Makes the noise picture on first use and uploads it again whenever the
/// noise object's tick reports a new picture.
pub(crate) fn tick_force_noise(
    mut noise: ResMut<ForceNoise>,
    mut images: ResMut<Assets<Image>>,
    mut roll: ResMut<FrameEffectRoll>,
    ticks: Option<Res<BodyTicks>>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    let texture = NoiseTexture::default();
    let noise = &mut *noise;
    let Some(field) = noise.field.as_mut() else {
        let field = noise_field(now, || roll.0.0.step());
        let pixels = noise_bytes(&field);
        noise.image = Some(images.add(noise_image(&pixels, NOISE_SIDE, texture.mipmaps())));
        noise.field = Some(field);
        noise.uploads = 1;
        return;
    };
    let fresh = field.tick(now, || roll.0.0.step());
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
        gradient: Vec4::ZERO,
        colour: Vec4::ZERO,
    }
}

/// The skin box a skinned copy's fade plane is measured from: the body
/// parts' spheres' height span (cm), its centre and half height.
pub fn skin_box(body: &Ragdoll) -> Option<ShellBox> {
    let mut low = f32::INFINITY;
    let mut high = f32::NEG_INFINITY;
    for part in &body.parts {
        let world = part.transform();
        for &(centre, radius) in part.spheres.iter() {
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

/// The frame and skin box of a caster without a physical body: unturned at
/// `at` (cm, the node's place, at the feet), a box 180 cm tall from there.
pub fn standing_frame(at: Vec3) -> (ShellFrame, ShellBox) {
    (
        ShellFrame {
            rows: [Vec3::X, Vec3::Y, Vec3::Z],
            translation: at,
        },
        ShellBox::Skinned {
            centre_y: at.y + STANDING_HALF_HEIGHT,
            half_y: STANDING_HALF_HEIGHT,
        },
    )
}

/// Half a standing body's height (cm) for a caster without a physical body.
const STANDING_HALF_HEIGHT: f32 = 90.0;

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
        &Transform,
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
    for (entity, aura, of, drawn, place) in &auras {
        let Some(ShellAuraOf(caster)) = of.copied() else {
            continue;
        };
        // The body's root and parts when the caster has a physical body;
        // otherwise the node's own place (cm) and a standing body's box.
        let fitted = bodies
            .get(caster)
            .ok()
            .and_then(|body| Some((aura_frame(body)?, skin_box(body)?)));
        let (frame, bounds) = fitted.unwrap_or_else(|| standing_frame(place.translation));
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
                    debug!("force shell aura {entity:?}: its caster has no skinned mesh yet");
                    continue;
                }
                info!(
                    "force shell aura {entity:?}: {} skinned copies of {caster:?}",
                    sources.len()
                );
                let material = materials.add(ForceShellMaterial {
                    params,
                    noise: image.clone(),
                    strikes: false,
                    scan: false,
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

/// A rigid aura's fit: its body's meshes' box in the body's first part's
/// frame (m), found when its copies are made.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct RigidFit {
    pub centre: Vec3,
    pub half: Vec3,
}

/// The frame of a rigid body's first part (cm): its axes and its origin.
pub fn part_frame(world: bevy::math::Affine3A) -> ShellFrame {
    let m = world.matrix3;
    ShellFrame {
        rows: [
            Vec3::from(m.x_axis).normalize_or(Vec3::X),
            Vec3::from(m.y_axis).normalize_or(Vec3::Y),
            Vec3::from(m.z_axis).normalize_or(Vec3::Z),
        ],
        translation: Vec3::from(world.translation) * CM,
    }
}

/// The box (m, in `frame_from_world`'s frame) around the corners of each
/// mesh box (`centre`, `half`, its mesh's world transform).
pub fn fit_boxes(
    frame_from_world: bevy::math::Affine3A,
    boxes: impl IntoIterator<Item = (Vec3, Vec3, bevy::math::Affine3A)>,
) -> Option<RigidFit> {
    let mut lo = Vec3::splat(f32::INFINITY);
    let mut hi = Vec3::splat(f32::NEG_INFINITY);
    for (centre, half, world) in boxes {
        for k in 0..8 {
            let sign = Vec3::new(
                if k & 1 == 0 { -1.0 } else { 1.0 },
                if k & 2 == 0 { -1.0 } else { 1.0 },
                if k & 4 == 0 { -1.0 } else { 1.0 },
            );
            let corner = world.transform_point3(centre + half * sign);
            let local = frame_from_world.transform_point3(corner);
            lo = lo.min(local);
            hi = hi.max(local);
        }
    }
    (lo.x <= hi.x).then(|| RigidFit {
        centre: (lo + hi) * 0.5,
        half: (hi - lo) * 0.5,
    })
}

/// A rigid aura's program uniform for this frame: the shell's uniforms over
/// the body's first part's `frame` (the copies carry the world positions,
/// so `LclMat` takes them into that frame, as the original's object space
/// is), the fade plane from the fitted box; a strikes aura's fade plane,
/// gradient and four ripple points are the strikes program's.
pub fn rigid_params(
    shell: &crate::thaum::ForceShell,
    frame: &ShellFrame,
    fit: RigidFit,
    strikes: Option<&super::hold_live::StrikesAura>,
    view: Vec3,
) -> ForceShellParams {
    let centre = fit.centre * CM;
    let half = fit.half * CM;
    let world_centre = frame.translation
        + frame.rows[0] * centre.x
        + frame.rows[1] * centre.y
        + frame.rows[2] * centre.z;
    let bounds = ShellBox::Rigid {
        centre_y: world_centre.y,
        half,
        radius: half.length(),
    };
    let mut uniforms = shell_uniforms(shell, false, frame, bounds, view);
    uniforms.local = frame.inverse();
    let mut params = shell_params(&uniforms);
    if let Some(strikes) = strikes {
        params.fade_plane = Vec4::from_array(crate::thaum::strikes_fade_plane(
            centre.y,
            half.y,
            shell.plane_scale,
        ));
        params.gradient = Vec4::from_array(crate::thaum::strikes_gradient(
            strikes.record,
            centre.y,
            half.y,
        ));
        for point in params.colour_points.iter_mut().skip(4) {
            *point = Vec4::ZERO;
        }
    }
    params
}

/// Each frame: a rigid aura without copies copies its body's rigid meshes
/// (a prop's, a held weapon's) and fits their box; every rigid aura's
/// material takes this frame's uniforms in the body's first part's frame.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_rigid_auras(
    mut commands: Commands,
    noise: Res<ForceNoise>,
    auras: Query<(
        Entity,
        &LiveShellAura,
        &super::hold_live::RigidAuraOf,
        Option<&ShellAuraDraw>,
        Option<&RigidFit>,
        Option<&super::hold_live::StrikesAura>,
    )>,
    bodies: Query<&Ragdoll>,
    globals: Query<&GlobalTransform>,
    children: Query<&Children>,
    meshes: Query<
        (
            &Mesh3d,
            &GlobalTransform,
            Option<&bevy::camera::primitives::Aabb>,
        ),
        (Without<SkinnedMesh>, Without<ShellCopy>),
    >,
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
    for (entity, aura, of, drawn, fit, strikes) in &auras {
        let target = of.0;
        let world = match bodies.get(target).ok().and_then(|b| b.parts.first()) {
            Some(part) => part.transform(),
            None => match globals.get(target) {
                Ok(g) => g.affine(),
                Err(_) => continue,
            },
        };
        let frame = part_frame(world);
        match (drawn, fit) {
            (Some(draw), Some(fit)) => {
                if let Some(mut material) = materials.get_mut(&draw.material) {
                    material.params = rigid_params(&aura.0, &frame, *fit, strikes, view);
                }
            }
            _ => {
                let sources: Vec<(Entity, Handle<Mesh>, Vec3, Vec3, bevy::math::Affine3A)> =
                    children
                        .iter_descendants(target)
                        .filter_map(|e| {
                            let (mesh, global, aabb) = meshes.get(e).ok()?;
                            let aabb = aabb?;
                            Some((
                                e,
                                mesh.0.clone(),
                                Vec3::from(aabb.center),
                                Vec3::from(aabb.half_extents),
                                global.affine(),
                            ))
                        })
                        .collect();
                let rigid = bevy::math::Affine3A::from_mat3_translation(
                    bevy::math::Mat3::from_cols(frame.rows[0], frame.rows[1], frame.rows[2]),
                    frame.translation / CM,
                );
                let Some(fit) = fit_boxes(
                    rigid.inverse(),
                    sources.iter().map(|&(_, _, c, h, w)| (c, h, w)),
                ) else {
                    debug!("force aura {entity:?}: {target:?} has no bounded mesh yet");
                    continue;
                };
                let material = materials.add(ForceShellMaterial {
                    params: rigid_params(&aura.0, &frame, fit, strikes, view),
                    noise: image.clone(),
                    strikes: strikes.is_some(),
                    scan: false,
                });
                for (source, mesh, ..) in &sources {
                    commands.spawn((
                        Name::new(if strikes.is_some() {
                            "effectfrcstrk copy"
                        } else {
                            "_ForceAura01 rigid copy"
                        }),
                        ShellCopy(entity),
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(material.clone()),
                        Transform::IDENTITY,
                        bevy::light::NotShadowCaster,
                        bevy::light::NotShadowReceiver,
                        ChildOf(*source),
                    ));
                }
                info!(
                    "force aura {entity:?}: {} rigid copies of {target:?} (strikes: {}), box half {:?} m",
                    sources.len(),
                    strikes.is_some(),
                    fit.half
                );
                commands.entity(entity).insert((
                    ShellAuraDraw {
                        material,
                        copies: sources.len(),
                    },
                    fit,
                ));
            }
        }
    }
}

/// The shell's impact sample, played at a hard hit.
const SHELL_HIT_SOUND: &str = "thmfrc_shi";

/// What a shell hit's sample takes from the game (none in tests without it).
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ShellHitAudio<'w> {
    game: Option<Res<'w, exa_assets::Game>>,
    bank: Option<ResMut<'w, exa_audio::SoundBank>>,
    sources: Option<ResMut<'w, Assets<AudioSource>>>,
    env: Option<ResMut<'w, exa_audio::AudioEnvironment>>,
}

impl ShellHitAudio<'_> {
    fn play(&mut self, set: &mut Option<u32>, at: Vec3, volume: f32, pitch: f32) {
        let (Some(game), Some(bank), Some(sources), Some(env)) = (
            self.game.as_deref(),
            self.bank.as_deref_mut(),
            self.sources.as_deref_mut(),
            self.env.as_deref_mut(),
        ) else {
            return;
        };
        let id = *set.get_or_insert_with(|| bank.load_set(game, sources, SHELL_HIT_SOUND));
        env.play_set(bank, id, at, volume, pitch, 0);
    }
}

/// A hard limb impact on a shelled caster ripples that shell at the contact,
/// in the aura's frame (cm from the body's root), and plays `thmfrc_shi`
/// when the shell's 400 ms gate is open. A caster with no shell is left
/// alone. The impact asks only once its volume is over 0.3.
pub(crate) fn ripple_shell_hits(
    mut hits: MessageReader<exa_combat::impact::ShellHit>,
    mut auras: Query<(&ShellAuraOf, &mut LiveShellAura)>,
    bodies: Query<&Ragdoll>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: Option<ResMut<FrameEffectRoll>>,
    mut audio: ShellHitAudio,
    mut set: Local<Option<u32>>,
) {
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    for hit in hits.read() {
        let place = bodies
            .get(hit.fighter)
            .ok()
            .and_then(aura_frame)
            .map(|frame| {
                let p = frame.local(hit.point * CM);
                [p.x, p.y, p.z]
            })
            .unwrap_or([0.0; 3]);
        for (of, mut aura) in &mut auras {
            if of.0 != hit.fighter {
                continue;
            }
            let mut unit = || roll.as_mut().map(|r| r.0.0.unit()).unwrap_or(0.0);
            if let Some((pitch, volume)) = aura.0.hit(place, hit.volume, now, &mut unit) {
                audio.play(&mut set, hit.point, volume, pitch);
            }
        }
    }
}

/// Force Strikes' hook on its caster's landed blows ([`exa_combat::StrikeHooked`]):
/// the aura on the caster's primary weapon (the item in its primary
/// striker's slot) ripples at the contact, in the weapon's frame (cm), at
/// half the hook's strength, and sounds `thmfrc_shi` at the contact when
/// the aura's 400 ms gate is open (its gain 1.5 and base pitch 0.8). A
/// caster whose primary weapon carries no aura, or that runs no Strikes,
/// is left alone.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn ripple_strikes(
    mut hooks: MessageReader<exa_combat::StrikeHooked>,
    casters: Query<(
        &super::cast::PowerCaster,
        Option<&exa_combat::attack::Attacker>,
    )>,
    blades: Query<(Entity, &exa_combat::Blade), With<exa_physics::grip::HeldGrip>>,
    mut auras: Query<&mut LiveShellAura, With<super::hold_live::StrikesAura>>,
    bodies: Query<&Ragdoll>,
    globals: Query<&GlobalTransform>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: Option<ResMut<FrameEffectRoll>>,
    mut audio: ShellHitAudio,
    mut set: Local<Option<u32>>,
) {
    let now = ticks.as_ref().map_or(0, |t| t.millis()) as u32;
    for hook in hooks.read() {
        let Ok((caster, attacker)) = casters.get(hook.attacker) else {
            continue;
        };
        // The blow reaches the power holding the strike hook.
        if !caster.hooks.strike_is(super::force::STRIKES) {
            continue;
        }
        let Some(power) = caster.active.iter().find_map(|entry| match &entry.class {
            super::cast::ActiveClass::Strikes(power) if power.state != super::power::ENDING => {
                Some(power)
            }
            _ => None,
        }) else {
            continue;
        };
        let Some(slot) = attacker.and_then(|a| a.primary) else {
            continue;
        };
        let Some(primary) = blades
            .iter()
            .find(|(_, blade)| blade.wielder == hook.attacker && blade.slot == slot)
            .map(|(entity, _)| entity)
        else {
            continue;
        };
        let Some(shell) = power
            .auras
            .iter()
            .find(|aura| aura.item == primary.to_bits())
            .and_then(|aura| aura.shell)
            .and_then(Entity::try_from_bits)
        else {
            continue;
        };
        let Ok(mut aura) = auras.get_mut(shell) else {
            continue;
        };
        let world = match bodies.get(primary).ok().and_then(|b| b.parts.first()) {
            Some(part) => part.transform(),
            None => match globals.get(primary) {
                Ok(g) => g.affine(),
                Err(_) => continue,
            },
        };
        let local = part_frame(world).local(hook.point * CM);
        let mut unit = || roll.as_mut().map(|r| r.0.0.unit()).unwrap_or(0.0);
        let sound = aura
            .0
            .hit(local.to_array(), hook.strength * 0.5, now, &mut unit);
        info!(
            "force strikes: {:?}'s blow ripples the aura {shell:?} on {primary:?} at {local:?} cm \
             (strength {:.3}, {} ripples)",
            hook.attacker,
            hook.strength,
            aura.0.ripples.len()
        );
        if let Some((pitch, volume)) = sound {
            audio.play(&mut set, hook.point, volume, pitch);
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

    /// A hard impact over volume 0.3 ripples the caster's shell at the
    /// contact, measured in centimetres from the body's root. Another
    /// body's shell is left alone.
    #[test]
    fn a_hard_impact_ripples_the_casters_shell() {
        use bevy::math::Affine3A;
        use exa_physics::body::PartSetup;

        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<exa_combat::impact::ShellHit>()
            .insert_resource(BodyTicks(1_000))
            .add_systems(Update, ripple_shell_hits);
        let caster = app
            .world_mut()
            .spawn(Ragdoll::new(&[PartSetup {
                bone: 1,
                entity: Entity::PLACEHOLDER,
                parent: None,
                world: Affine3A::from_translation(Vec3::new(2.0, 0.0, 0.0)),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                spheres: None,
                origin: Vec3::ZERO,
                links: Vec::new(),
                foot: None,
                pins: Vec::new(),
            }]))
            .id();
        let other = app.world_mut().spawn_empty().id();
        app.world_mut().spawn((
            ShellAuraOf(caster),
            LiveShellAura(ForceShell::new(0, [0.0, 0.0])),
        ));
        app.world_mut().spawn((
            ShellAuraOf(other),
            LiveShellAura(ForceShell::new(0, [0.0, 0.0])),
        ));
        app.world_mut().write_message(exa_combat::impact::ShellHit {
            fighter: caster,
            point: Vec3::new(3.0, 0.4, 0.0),
            volume: 0.5,
        });
        app.update();
        let mut shells = app.world_mut().query::<(&ShellAuraOf, &LiveShellAura)>();
        let mut found = None;
        let mut spared = true;
        for (of, aura) in shells.iter(app.world()) {
            if of.0 == caster {
                found = Some(aura.0.ripples.clone());
            }
            if of.0 == other {
                spared = aura.0.ripples.is_empty();
            }
        }
        let ripples = found.expect("the caster's shell");
        assert_eq!(ripples.len(), 1);
        assert_eq!(ripples[0].strength, 6.0);
        assert_eq!(ripples[0].at, [100.0, 40.0, 0.0]);
        assert!(spared);
    }

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
    fn a_rigid_fit_boxes_the_meshes_in_the_part_frame() {
        use bevy::math::{Affine3A, Mat3};
        // A part turned a quarter about y at (1, 2, 3) m; one mesh box of
        // half (0.1, 0.2, 0.3) at the part's origin.
        let world = Affine3A::from_mat3_translation(
            Mat3::from_rotation_y(std::f32::consts::FRAC_PI_2),
            Vec3::new(1.0, 2.0, 3.0),
        );
        let fit = fit_boxes(
            world.inverse(),
            [(Vec3::ZERO, Vec3::new(0.1, 0.2, 0.3), world)],
        )
        .unwrap();
        assert!(fit.centre.length() < 1e-5);
        assert!((fit.half - Vec3::new(0.1, 0.2, 0.3)).length() < 1e-5);
        let frame = part_frame(world);
        assert!((frame.translation - Vec3::new(100.0, 200.0, 300.0)).length() < 1e-3);
        assert!(fit_boxes(Affine3A::IDENTITY, []).is_none());
    }

    #[test]
    fn a_rigid_aura_takes_world_points_into_its_part_frame() {
        let frame = ShellFrame {
            rows: [Vec3::Z, Vec3::Y, -Vec3::X],
            translation: Vec3::new(100.0, 50.0, 0.0),
        };
        let fit = RigidFit {
            centre: Vec3::new(0.0, 0.2, 0.0),
            half: Vec3::new(0.1, 0.2, 0.1),
        };
        let shell = ForceShell::held(0, [0.0; 2], 30.0);
        let params = rigid_params(&shell, &frame, fit, None, Vec3::ZERO);
        let p = Vec3::new(1.3, 0.7, -0.2);
        let lpos = params.local_from_world.transform_point3(p);
        assert!((lpos - frame.local(p * CM)).length() < 1e-3);
        assert_eq!(params.colour_int, 2.8);
        assert_eq!(params.gradient, Vec4::ZERO);
        // The box bottom is the part's origin (centre 20 cm up, half 20):
        // the plane scale 0.5 over the 20 cm half height.
        assert!((params.fade_plane.y - 0.5 / 20.0).abs() < 1e-6);
        assert!(params.fade_plane.w.abs() < 1e-6);
        let strikes = super::super::hold_live::StrikesAura {
            record: Some([0.5, 0.9, 100.0]),
        };
        let mut weapon = ForceShell::strikes(0, [0.0; 2]);
        let mut roll = || 0.5;
        for k in 0..6 {
            weapon.hit([k as f32, 0.0, 0.0], 1.0, 1000 + 500 * k, &mut roll);
        }
        weapon.step_ripples();
        let params = rigid_params(&weapon, &frame, fit, Some(&strikes), Vec3::ZERO);
        assert_eq!(
            params.gradient.y,
            crate::thaum::strikes_gradient(strikes.record, 20.0, 20.0)[1]
        );
        assert_eq!(
            params.fade_plane,
            Vec4::from_array(crate::thaum::strikes_fade_plane(20.0, 20.0, 1.0))
        );
        assert!(params.colour_points[3].w > 0.0);
        assert_eq!(params.colour_points[4], Vec4::ZERO);
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
            .init_resource::<FrameEffectRoll>()
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
                Transform::default(),
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
            .init_resource::<FrameEffectRoll>()
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

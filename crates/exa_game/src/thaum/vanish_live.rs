//! The live vanish effect: the displacement shell over a vanishing body and
//! the vanish's own drawing of that body, for Vanish and the Long Door.
//!
//! The original's vanish effect (one per cast) keeps the body's
//! displacement shell (`_EffectShell01`, `dispshell_v`/`dispshell_f`) and
//! runs its phase table ([`crate::effects::vanish`]) in two frame callbacks:
//!
//! - **Before the scene** it works out the body's shown amount from the
//!   current entry's flags ([`VanishTable::phase_fade`]) and, while that is
//!   above one half, runs the entry's segment pass (bits 0x200 and 0x400).
//! - **After the scene** it moves the table on when the entry's end has
//!   passed ([`VanishTable::advance`]), runs the segment pass while the
//!   shown amount is at one half or below, then draws the shell with
//!   `Interior` at one less the shown amount: the shell's outer rim over a
//!   shown body, the darkened inside of the silhouette over a hidden one.
//!
//! The segment pass's depth half (0x200) draws the body's parts with colour
//! writes off and depth writes on; its fade half (0x400) draws them blended
//! at the shown amount. A body whose show call left bits 0x30 set is drawn
//! by this pass only ([`VanishTable::draws_bodies`]).
//!
//! The shell's own timeline ([`Shell`]) opens over 400 ms from the cast,
//! and the power writes its state word as it goes: closing (2) when the
//! body vanishes, opening (1, stamped now) when the body comes back, and
//! closing again with leave to finish when the come-back ends. The shell's
//! close calls the vanish's phase hook ([`VanishTable::phased_body_done`]):
//! the table moves to its shown entry, or, on its last entry, gives the
//! body back and lets the shell go.
//!
//! In Exarust the shell is a copy of each skinned mesh of the caster
//! (sharing its skin) with one [`DispShellMaterial`] a vanish; the depth
//! pass is a second copy with the material's depth-only form, sorted just
//! ahead of the shell. The body's own meshes are hidden while the vanish
//! draws them and neither half of its pass shows them; the fade half is
//! taken as the body shown while its amount is above one half [guess: the
//! original blends the parts at the shown amount over the entry's 100 or
//! 200 ms]. `EmiTex` is the record's emissive layer, `FlameletsA3`.
//!
//! Rules: `docs/THAUMATURGY.md` (Displacement) and `docs/RENDERING.md`
//! (Force effects).

use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_render::force_effects::{
    DISP_SHELL_TEXTURE, DispShellMaterial, DispShellParams, SHELL_PUSH_M,
};

use super::cast::{ActiveClass, PowerCaster};
use super::power::displacement::{AI_PICK, FADING_IN, FADING_OUT, TAIL, VanishBase};
use crate::effects::shell::{
    DEPTH_PASS, FADE_PASS, OPACITY, PLANE_SCALE, PlaneBounds, Shell, ShellState, TEX_SCALE,
    fade_plane,
};
use crate::effects::vanish::{Vanish as VanishTable, VanishBody};
use crate::effects::{EffectClock, Frame};

/// Centimetres a metre.
const CM: f32 = 100.0;

/// A vanish effect node's live state: the phase table, the shell it holds
/// and what the power last said.
#[derive(Component, Debug, Clone)]
pub struct LiveVanish {
    /// The vanishing body.
    pub caster: Entity,
    pub table: VanishTable,
    /// The displacement shell, until it has closed with leave to finish.
    pub shell: Option<Shell>,
    /// The vanish still holds the shell and draws it (the table's last
    /// hook lets it go).
    pub holds_shell: bool,
    /// The power's phase the last step saw.
    pub phase: i32,
    /// The shown amount and the entry's flags of this frame's first
    /// callback.
    pub shown: f32,
    pub flags: u32,
    /// The table and the shell have taken the effects' clock (a vanish
    /// made by a cast starts on the next step, as a shell created with a
    /// zero stamp takes the clock it is first stepped on).
    pub started: bool,
}

impl LiveVanish {
    /// A vanish cast at `now_ms` over `caster`: the table started as a
    /// phased (not instant) vanish of one body, and a shell opening from
    /// now with `tex_pos`, the shell's two creation draws.
    pub fn new(caster: Entity, now_ms: u32, tex_pos: [f32; 2]) -> Self {
        let body = VanishBody {
            flags: 0,
            shown: 1.0,
            call: crate::effects::vanish::HIDE_CALL,
        };
        let table = VanishTable::start(None, false, now_ms, vec![body], Vec::new());
        Self {
            caster,
            shown: table.phase_fade(now_ms),
            flags: table.phase_entry().flags,
            table,
            shell: Some(Shell::new(now_ms, tex_pos)),
            holds_shell: true,
            phase: super::power::displacement::HIDDEN,
            started: true,
        }
    }

    /// A vanish a cast has just made: it starts on its first step.
    pub fn waiting(caster: Entity, tex_pos: [f32; 2]) -> Self {
        Self {
            started: false,
            ..Self::new(caster, 0, tex_pos)
        }
    }

    /// Starts a waiting vanish at `now_ms`, keeping its shell's draws.
    pub fn start(&mut self, now_ms: u32) {
        if self.started {
            return;
        }
        let tex_pos = self.shell.as_ref().map_or([0.0; 2], |s| s.tex_pos);
        *self = Self::new(self.caster, now_ms, tex_pos);
    }

    /// Follows the power's phase: the shell's state words and the table's
    /// restart the power writes on its way through the phases. `base` is
    /// the power's shared vanish state.
    pub fn follow(&mut self, base: &VanishBase, now_ms: u32) {
        let rank = |phase: i32| if phase == AI_PICK { 0 } else { phase };
        let (was, is) = (rank(self.phase), rank(base.phase));
        self.phase = base.phase;
        if was >= is {
            return;
        }
        if was < FADING_OUT && is >= FADING_OUT {
            // The body vanishes: the shell closes from wherever it is.
            if let Some(shell) = &mut self.shell {
                shell.state = ShellState::Closing;
            }
        }
        if was < FADING_IN && is >= FADING_IN {
            // The body comes back: the shell opens again from now, the
            // bodies are revealed and the table moves on with a new clock.
            if let Some(shell) = &mut self.shell {
                shell.stamp = now_ms;
                shell.state = ShellState::Opening;
            }
            self.table.come_back(now_ms);
        }
        if was < TAIL && is >= TAIL && !base.shared {
            // The come-back ends: the shell closes and may then leave.
            if let Some(shell) = &mut self.shell {
                shell.state = ShellState::Closing;
                shell.finish();
            }
        }
    }

    /// One frame of the effect at `now_ms`: the shell's own step (its close
    /// calls the table's phase hook), the first callback's shown amount and
    /// flags, then the second callback's table step.
    pub fn frame(&mut self, now_ms: u32) {
        if let Some(shell) = &mut self.shell {
            let step = shell.step(now_ms);
            if step.end {
                let table_end = self.table.phase_entry().end == -1;
                self.table.phased_body_done();
                if table_end {
                    self.holds_shell = false;
                }
            }
            if step.remove {
                self.shell = None;
            }
        }
        self.shown = self.table.phase_fade(now_ms);
        self.flags = self.table.phase_entry().flags;
        self.table.advance(now_ms);
    }

    /// Whether the shell is drawn this frame.
    pub fn shell_drawn(&self) -> bool {
        self.holds_shell && self.shell.is_some()
    }

    /// Whether the segment pass's depth half draws the body's parts.
    pub fn depth_pass(&self) -> bool {
        self.flags & DEPTH_PASS != 0
    }

    /// Whether the body's own meshes are seen: always while the scene draws
    /// them, else only while the fade half shows it above one half.
    pub fn body_seen(&self) -> bool {
        !self.table.draws_bodies() || (self.flags & FADE_PASS != 0 && self.shown > 0.5)
    }

    /// The shell's `Interior`: one less the shown amount.
    pub fn interior(&self) -> f32 {
        1.0 - self.shown
    }
}

/// The node frame the shell's uniforms take: the body's root part's place
/// (cm) turned as the caster's node, or the caster's own place.
pub fn body_frame(body: Option<&Ragdoll>, node: &GlobalTransform) -> Frame {
    let (_, rotation, translation) = node.to_scale_rotation_translation();
    let at = body
        .and_then(|b| b.parts.first())
        .map_or(translation, |part| Vec3::from(part.transform().translation));
    Frame {
        x: rotation * Vec3::X,
        y: rotation * Vec3::Y,
        z: rotation * Vec3::Z,
        at: at * CM,
    }
}

/// A frame as a matrix (cm).
pub fn frame_matrix(frame: &Frame) -> Mat4 {
    Mat4::from_cols(
        frame.x.extend(0.0),
        frame.y.extend(0.0),
        frame.z.extend(0.0),
        frame.at.extend(1.0),
    )
}

/// The fade plane's extent: the body parts' spheres' height span, or a
/// standing body's 180 cm from the node's place.
pub fn body_bounds(body: Option<&Ragdoll>, frame: &Frame) -> PlaneBounds {
    match body.and_then(super::effect_draw::skin_box) {
        Some(crate::thaum::ShellBox::Skinned { centre_y, half_y }) => {
            PlaneBounds::Skinned { centre_y, half_y }
        }
        _ => PlaneBounds::Skinned {
            centre_y: frame.at.y + 90.0,
            half_y: 90.0,
        },
    }
}

/// The displacement shell's uniform this frame: `LclMat` the inverse of
/// the body node's frame with the metre-to-centimetre scale, the camera
/// carried into that frame, the fade plane up the body, `TexPos`, `TexOfs`
/// rolling over 40 s, `Fade` (1 − shown²)² and `Interior`.
pub fn shell_params(
    shell: &Shell,
    interior: f32,
    frame: &Frame,
    bounds: PlaneBounds,
    camera_m: Vec3,
    now_ms: u32,
) -> DispShellParams {
    let local = frame_matrix(frame).inverse();
    let uniforms = shell.uniforms(now_ms);
    DispShellParams {
        local_from_world: local * Mat4::from_scale(Vec3::splat(CM)),
        view_pos: local.transform_point3(camera_m * CM).extend(0.0),
        fade_plane: Vec4::from_array(fade_plane(frame, bounds, PLANE_SCALE)),
        tex: Vec4::new(
            uniforms.tex_pos[0],
            uniforms.tex_pos[1],
            uniforms.tex_offset,
            0.0,
        ),
        fade: uniforms.fade,
        interior,
        tex_scale: TEX_SCALE,
        opacity: OPACITY,
        push: SHELL_PUSH_M,
    }
}

/// The materials of a vanish whose copies are made.
#[derive(Component, Debug, Clone)]
pub struct VanishDraw {
    pub shell: Handle<DispShellMaterial>,
    pub depth: Handle<DispShellMaterial>,
}

/// A copy of one of the caster's skinned meshes, drawn for the vanish it
/// names: the shell, or with `depth` the depth pass.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanishCopy {
    pub vanish: Entity,
    pub depth: bool,
}

/// A caster mesh the vanish hides, with the visibility it had.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct VanishHidden {
    pub by: Entity,
    pub was: Visibility,
}

/// Each fixed step after the casters' tick: every vanish follows its
/// power's phase and runs its frame.
pub(crate) fn step_vanishes(
    clock: Option<Res<EffectClock>>,
    casters: Query<&PowerCaster>,
    mut vanishes: Query<(Entity, &mut LiveVanish)>,
) {
    let Some(clock) = clock else {
        return;
    };
    for (entity, mut live) in &mut vanishes {
        live.start(clock.now_ms);
        let base = casters.get(live.caster).ok().and_then(|caster| {
            caster
                .active
                .iter()
                .filter(|entry| entry.entity == Some(entity))
                .find_map(|entry| match &entry.class {
                    ActiveClass::Vanish(power) => Some(power.base),
                    ActiveClass::LongDoor(power) => Some(power.base),
                    _ => None,
                })
        });
        if let Some(base) = base {
            live.follow(&base, clock.now_ms);
        }
        live.frame(clock.now_ms);
    }
}

/// The caster's own meshes: every mesh under it that is not a copy an
/// effect made.
type OwnMesh = (
    Without<VanishCopy>,
    Without<super::effect_draw::ShellCopy>,
    Without<crate::effects::scan_draw::ScanCopy>,
);

/// Each frame: a vanish without copies copies its caster's shown skinned
/// meshes twice (the shell and the depth pass); every vanish's materials
/// take this frame's uniforms, its copies show as its callbacks draw them
/// and the caster's own meshes hide while the vanish draws the body.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_vanishes(
    mut commands: Commands,
    clock: Option<Res<EffectClock>>,
    game: Option<Res<exa_assets::Game>>,
    mut texture: Local<Option<Handle<Image>>>,
    images: Option<ResMut<Assets<Image>>>,
    vanishes: Query<(Entity, &LiveVanish, Option<&VanishDraw>)>,
    bodies: Query<(Option<&Ragdoll>, &GlobalTransform)>,
    children: Query<&Children>,
    skinned: Query<(&Mesh3d, &SkinnedMesh), OwnMesh>,
    mut own: Query<(Entity, &mut Visibility, Option<&VanishHidden>), (With<Mesh3d>, OwnMesh)>,
    mut copy_meshes: Query<(&VanishCopy, &mut Visibility), (With<Mesh3d>, Without<VanishHidden>)>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    materials: Option<ResMut<Assets<DispShellMaterial>>>,
) {
    if vanishes.is_empty() {
        return;
    }
    let (Some(clock), Some(mut images), Some(mut materials)) = (clock, images, materials) else {
        return;
    };
    let image = texture
        .get_or_insert_with(|| load_texture(game.as_deref(), &mut images))
        .clone();
    let camera = cameras
        .iter()
        .next()
        .map_or(Vec3::ZERO, |c| c.translation());
    for (entity, live, drawn) in &vanishes {
        let Ok((body, node)) = bodies.get(live.caster) else {
            continue;
        };
        let frame = body_frame(body, node);
        let bounds = body_bounds(body, &frame);
        let params = live.shell.as_ref().map(|shell| {
            shell_params(shell, live.interior(), &frame, bounds, camera, clock.now_ms)
        });
        match drawn {
            Some(draw) => {
                if let Some(params) = params
                    && let Some(mut material) = materials.get_mut(&draw.shell)
                {
                    material.params = params;
                }
            }
            None => {
                let sources: Vec<(Handle<Mesh>, SkinnedMesh)> = children
                    .iter_descendants(live.caster)
                    .filter_map(|e| {
                        let (mesh, skin) = skinned.get(e).ok()?;
                        // A part its apparel covers stays out of the copy.
                        let hidden = own
                            .get(e)
                            .is_ok_and(|(_, v, h)| *v == Visibility::Hidden && h.is_none());
                        (!hidden).then(|| (mesh.0.clone(), skin.clone()))
                    })
                    .collect();
                if sources.is_empty() {
                    debug!("vanish {entity:?}: its caster has no skinned mesh yet");
                    continue;
                }
                info!(
                    "vanish {entity:?}: displacement shell over {} skinned meshes of {:?}",
                    sources.len(),
                    live.caster
                );
                let shell = materials.add(DispShellMaterial {
                    params: params.unwrap_or_default(),
                    texture: image.clone(),
                    depth_only: false,
                });
                let depth = materials.add(DispShellMaterial {
                    params: DispShellParams {
                        push: 0.0,
                        ..DispShellParams::default()
                    },
                    texture: image.clone(),
                    depth_only: true,
                });
                for (mesh, skin) in &sources {
                    for (material, is_depth, name) in [
                        (&depth, true, "_EffectShell01 depth pass"),
                        (&shell, false, "_EffectShell01 copy"),
                    ] {
                        // A child of the caster: it leaves with the caster,
                        // and the caster's own meshes can hide without it.
                        commands.spawn((
                            Name::new(name),
                            VanishCopy {
                                vanish: entity,
                                depth: is_depth,
                            },
                            Mesh3d(mesh.clone()),
                            skin.clone(),
                            MeshMaterial3d(material.clone()),
                            Transform::IDENTITY,
                            Visibility::Hidden,
                            NoFrustumCulling,
                            bevy::light::NotShadowCaster,
                            bevy::light::NotShadowReceiver,
                            ChildOf(live.caster),
                        ));
                    }
                }
                commands.entity(entity).insert(VanishDraw { shell, depth });
            }
        }
        // The caster's own meshes hide while the vanish draws the body
        // and neither half of its pass shows it.
        let seen = live.body_seen();
        for e in children.iter_descendants(live.caster) {
            let Ok((mesh, mut visibility, hidden)) = own.get_mut(e) else {
                continue;
            };
            match (seen, hidden) {
                (false, None) => {
                    commands.entity(mesh).insert(VanishHidden {
                        by: entity,
                        was: *visibility,
                    });
                    *visibility = Visibility::Hidden;
                }
                (true, Some(hidden)) if hidden.by == entity => {
                    *visibility = hidden.was;
                    commands.entity(mesh).remove::<VanishHidden>();
                }
                _ => {}
            }
        }
    }
    for (copy, mut visibility) in &mut copy_meshes {
        let Ok((_, live, _)) = vanishes.get(copy.vanish) else {
            continue;
        };
        let shown = if copy.depth {
            live.depth_pass() && live.shell.is_some()
        } else {
            live.shell_drawn()
        };
        let want = if shown {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != want {
            *visibility = want;
        }
    }
}

/// Removes the copies of a vanish that has left and gives the caster's
/// meshes it hid their visibility back.
pub(crate) fn drop_vanish_copies(
    mut commands: Commands,
    copies: Query<(Entity, &VanishCopy)>,
    mut hidden: Query<(Entity, &VanishHidden, &mut Visibility)>,
    vanishes: Query<(), With<LiveVanish>>,
) {
    for (entity, copy) in &copies {
        if !vanishes.contains(copy.vanish) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, hid, mut visibility) in &mut hidden {
        if !vanishes.contains(hid.by) {
            *visibility = hid.was;
            commands.entity(entity).remove::<VanishHidden>();
        }
    }
}

/// The shell's picture: the record's emissive layer from the game's
/// textures, its values read raw; a mid-grey texel when the install does
/// not have it.
fn load_texture(game: Option<&exa_assets::Game>, images: &mut Assets<Image>) -> Handle<Image> {
    let loaded = game.map(|game| {
        game.load_texture(
            DISP_SHELL_TEXTURE,
            exa_assets::texture::ColorSpace::Linear,
            None,
        )
    });
    match loaded {
        Some(Ok(image)) => images.add(image),
        Some(Err(e)) => {
            warn!("displacement shell: {DISP_SHELL_TEXTURE}: {e}; a grey texel stands in");
            images.add(grey_texel())
        }
        None => images.add(grey_texel()),
    }
}

fn grey_texel() -> Image {
    Image::new_fill(
        bevy::render::render_resource::Extent3d::default(),
        bevy::render::render_resource::TextureDimension::D2,
        &[128, 128, 128, 255],
        bevy::render::render_resource::TextureFormat::Rgba8Unorm,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::power::displacement::{HIDDEN, HOLDING};

    fn base(phase: i32) -> VanishBase {
        VanishBase {
            phase,
            deadline: 0,
            released: false,
            shared: false,
            shown: 1.0,
            step: 1,
        }
    }

    /// A player's Vanish, cast at 0 ms, released at 2000 ms: the power
    /// hides the body at 400 ms, holds from 700, comes back at 2000 over
    /// 400 ms and ends 300 ms later.
    #[test]
    fn a_vanish_shows_the_shell_hides_the_body_and_brings_it_back() {
        let caster = Entity::from_bits(7);
        let mut live = LiveVanish::new(caster, 0, [0.25, 0.5]);
        let run = |live: &mut LiveVanish, phase: i32, now: u32| {
            live.follow(&base(phase), now);
            live.frame(now);
        };
        // The cast: the shell opens over a shown body, outer rim only.
        run(&mut live, HIDDEN, 0);
        assert!(live.shell_drawn());
        assert!(live.body_seen());
        assert_eq!(live.interior(), 0.0);
        run(&mut live, HIDDEN, 200);
        assert_eq!(live.shell.as_ref().unwrap().shown, 0.5);
        // At 350 ms the table stages the body: the vanish draws it now.
        run(&mut live, HIDDEN, 351);
        assert!(live.table.draws_bodies());
        // 350–450 ms: the fade entry, depth and fade passes, the body
        // shown above one half.
        run(&mut live, HIDDEN, 380);
        assert!(live.depth_pass());
        assert!(live.body_seen());
        // The power hides the body at 400 ms: the shell starts closing.
        run(&mut live, FADING_OUT, 400);
        assert_eq!(live.shell.as_ref().unwrap().state, ShellState::Fading);
        run(&mut live, FADING_OUT, 420);
        assert!(!live.body_seen());
        assert!(live.interior() > 0.5);
        // Past 450 ms: depth only, the body hidden, the inside drawn.
        run(&mut live, FADING_OUT, 455);
        run(&mut live, FADING_OUT, 460);
        assert_eq!(live.flags, 0x300);
        assert!(!live.body_seen());
        assert_eq!(live.interior(), 1.0);
        // The shell closes at 700 ms; its hook moves the table to its
        // shown entry: no pass, the body still out of sight.
        run(&mut live, FADING_OUT, 700);
        run(&mut live, HOLDING, 701);
        assert_eq!(live.table.phase, 4);
        assert!(!live.depth_pass());
        assert!(!live.body_seen());
        assert!(live.shell_drawn());
        assert_eq!(live.shell.as_ref().unwrap().fade(), 1.0);
        // The release at 2000 ms: the shell opens again from now and the
        // table moves to its depth entry over a hidden body.
        run(&mut live, FADING_IN, 2_000);
        assert_eq!(live.table.phase, 5);
        assert_eq!(live.shell.as_ref().unwrap().state, ShellState::Opening);
        run(&mut live, FADING_IN, 2_100);
        assert!(live.depth_pass());
        assert!(!live.body_seen());
        assert_eq!(live.shell.as_ref().unwrap().shown, 0.25);
        // 2200–2400 ms: the body fades back in.
        run(&mut live, FADING_IN, 2_300);
        run(&mut live, FADING_IN, 2_350);
        assert!(live.body_seen());
        // At 2400 ms the table gives the body back to the scene.
        run(&mut live, TAIL, 2_401);
        assert!(!live.table.draws_bodies());
        assert!(live.body_seen());
        assert!(live.shell.as_ref().unwrap().can_finish);
        // The shell closes over 300 ms, the table ends and the shell goes.
        run(&mut live, TAIL, 2_402);
        run(&mut live, TAIL, 2_710);
        assert!(!live.holds_shell);
        run(&mut live, TAIL, 2_720);
        assert!(live.shell.is_none());
        assert!(live.body_seen());
    }

    #[test]
    fn a_cast_vanish_starts_on_its_first_step() {
        let mut live = LiveVanish::waiting(Entity::from_bits(3), [0.5, 0.25]);
        assert!(!live.started);
        live.start(1_000);
        assert!(live.started);
        assert_eq!(live.table.since_ms, 1_000);
        let shell = live.shell.as_ref().unwrap();
        assert_eq!((shell.stamp, shell.tex_pos), (1_000, [0.5, 0.25]));
        live.start(2_000);
        assert_eq!(live.table.since_ms, 1_000);
    }

    #[test]
    fn the_shell_uniform_runs_up_the_body_in_its_frame() {
        let shell = Shell::new(0, [0.125, 0.75]);
        let frame = Frame {
            at: Vec3::new(100.0, 90.0, -50.0),
            ..Frame::IDENTITY
        };
        let bounds = PlaneBounds::Skinned {
            centre_y: 90.0,
            half_y: 90.0,
        };
        let params = shell_params(
            &shell,
            0.25,
            &frame,
            bounds,
            Vec3::new(1.0, 1.6, 3.0),
            20_000,
        );
        // World metres into the frame's centimetres.
        let lpos = params
            .local_from_world
            .transform_point3(Vec3::new(1.5, 0.9, -0.5));
        assert!((lpos - Vec3::new(50.0, 0.0, 0.0)).length() < 1e-3);
        assert!((params.view_pos.truncate() - Vec3::new(0.0, 70.0, 350.0)).length() < 1e-3);
        // The plane: up over the half height, zero at the feet.
        assert_eq!(params.fade_plane, Vec4::new(0.0, 1.0 / 90.0, 0.0, 1.0));
        assert_eq!(params.tex, Vec4::new(0.125, 0.75, 0.5, 0.0));
        assert_eq!(params.fade, 1.0);
        assert_eq!(params.interior, 0.25);
        assert_eq!(params.tex_scale, 0.015);
        assert_eq!(params.opacity, 1.0);
        assert_eq!(params.push, 0.02);
    }
}

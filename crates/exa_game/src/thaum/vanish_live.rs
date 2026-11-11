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
//! The fade half (0x400) draws each body part with its material's opacity
//! set to the shown amount and a plain stage switched to its blended one,
//! after the depth half laid the body's depth, so only the nearest layer
//! shows. The staging entries set the body nodes' fade bit, and while it
//! is set the shadow passes draw them dithered at the node fade, which the
//! fade half sets to one less the shown amount: the shadow thins out with
//! the body, stays nearly gone while it is hidden and comes back with it
//! until the hide clears the bit.
//!
//! A running Constant Displacement keeps a shell of its own on the body
//! ([`ConstantShell`]): its start makes the shell before the tendril image,
//! it rests open while the power runs and closes at its end. A vanish or
//! Long Door cast meanwhile takes that shell instead of making one (no
//! creation draws), runs it through its phases and leaves it open for the
//! constant at its end.
//!
//! In Exarust the shell is a copy of each skinned mesh of the caster
//! (sharing its skin) with one [`DispShellMaterial`] a vanish; the depth
//! pass is a second copy with the material's depth-only form, sorted just
//! ahead of the shell. The body's own meshes hide while only the depth
//! half draws them; for the fade half each part takes a blended copy of
//! its material at the shown amount that writes its own depth (the depth
//! copy steps aside on those frames), and a part without a material of the
//! original's kind shows while the amount is above one half. The dither is
//! [`ShadowDither`] on the parts, read by the cached cube shadows. A lent
//! shell lives on its constant's node and is copied into the vanish for
//! each step. `EmiTex` is the record's emissive layer, `FlameletsA3`.
//!
//! Rules: `docs/THAUMATURGY.md` (Displacement) and `docs/RENDERING.md`
//! (Force effects).

use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::skinning::SkinnedMesh;
use bevy::prelude::*;
use exa_physics::body::Ragdoll;
use exa_render::cube_shadows::ShadowDither;
use exa_render::force_effects::{
    DISP_SHELL_TEXTURE, DispShellMaterial, DispShellParams, SHELL_PUSH_M,
};
use exa_render::material::{ExaMaterial, flags2};

use super::cast::{ActiveClass, PowerCaster};
use super::power::displacement::{AI_PICK, FADING_IN, FADING_OUT, TAIL, VanishBase};
use crate::effects::shell::{
    DEPTH_PASS, FADE_PASS, PLANE_SCALE, PlaneBounds, Shell, ShellState, fade_plane,
};
use crate::effects::vanish::{
    CONSTANT_DISPLACEMENT, LENT_FLAG, PHASED_FLAGS, SHOWN_BIT, Vanish as VanishTable, VanishBody,
};
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
    /// The summoner whose Summon runs this vanish over its demon (`caster`
    /// is then the demon's body); none for a vanish over its own caster.
    pub summoner: Option<Entity>,
    /// The running Constant Displacement whose shell this vanish took
    /// ([`ConstantShell`]): the vanish runs that shell, as the original's
    /// vanish start reuses it, and gives it back when it goes.
    pub lent_from: Option<Entity>,
    /// The bodies' shadow fade (the original's node fade the dithered
    /// caster pass reads): one less the shown amount, written on the frames
    /// whose entry runs the fade half, kept in between.
    pub shadow_fade: f32,
    /// This frame's bodies' state as the scene and the shadow passes see
    /// it: left by the last frame's table step (the original's post-scene
    /// callback), so a staging step takes the bodies from the next frame.
    pub scene_draws: bool,
    pub dithered: bool,
}

/// How the vanish leaves the body's own meshes this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BodyLook {
    /// The scene draws them.
    Scene,
    /// The fade half draws them blended at this opacity (the shown
    /// amount), over the depth half.
    Blend(f32),
    /// Only the depth half draws them, or nothing does.
    Hidden,
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
            summoner: None,
            lent_from: None,
            shadow_fade: 0.0,
            scene_draws: true,
            dithered: false,
        }
    }

    /// The summon's vanish over its arrived `demon` at `now_ms`: the
    /// summon's table with the body revealed, and the shell resting at
    /// nothing shown (opening over 3000 ms and closing over 1000 ms when
    /// the summon says, at opacity 2, its texture scale over the demon's
    /// `size`).
    pub fn summon(
        demon: Entity,
        summoner: Entity,
        now_ms: u32,
        tex_pos: [f32; 2],
        size: f32,
    ) -> Self {
        use super::power::displacement::{
            SUMMON_GROW, SUMMON_SHELL_CLOSE_MS, SUMMON_SHELL_OPACITY, SUMMON_SHELL_OPEN_MS,
        };
        let body = VanishBody {
            flags: 0,
            shown: 1.0,
            call: crate::effects::vanish::HIDE_CALL,
        };
        let table = VanishTable::summon(now_ms, vec![body]);
        Self {
            caster: demon,
            shown: table.phase_fade(now_ms),
            flags: table.phase_entry().flags,
            table,
            shell: Some(Shell::resting(
                now_ms,
                tex_pos,
                SUMMON_SHELL_OPEN_MS,
                SUMMON_SHELL_CLOSE_MS,
                SUMMON_SHELL_OPACITY,
                size,
            )),
            holds_shell: true,
            phase: SUMMON_GROW,
            started: true,
            summoner: Some(summoner),
            lent_from: None,
            shadow_fade: 0.0,
            scene_draws: true,
            dithered: false,
        }
    }

    /// Follows the summon's step: the shell opens from now and the table
    /// moves on when the held phase ends; the shell closes with leave to
    /// finish when the demon is let go.
    pub fn follow_summon(&mut self, step: &super::power::displacement::SummonStep, now_ms: u32) {
        // Each step acts once: on the tick its phase changed.
        if step.phase == self.phase {
            return;
        }
        if step.open_shell {
            if let Some(shell) = &mut self.shell {
                shell.stamp = now_ms;
                shell.state = ShellState::Opening;
            }
            self.table.summon_opens(now_ms);
        }
        if step.release
            && let Some(shell) = &mut self.shell
        {
            shell.state = ShellState::Closing;
            shell.finish();
        }
        self.phase = step.phase;
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
        *self = Self {
            summoner: self.summoner,
            lent_from: self.lent_from,
            ..Self::new(self.caster, now_ms, tex_pos)
        };
    }

    /// Takes the shell of a running Constant Displacement (`from` its
    /// node) in place of a shell of its own, as the original's vanish start
    /// does when the caster's power list holds one: the shell keeps its
    /// state and stamp, and the phased body is lent (its flags gain 0x20).
    pub fn lend(&mut self, shell: Shell, from: Entity) {
        self.shell = Some(shell);
        self.lent_from = Some(from);
        self.table.lent = true;
        self.table.phased = Some(PHASED_FLAGS | LENT_FLAG);
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
        // The fade half writes the bodies' node fade, which the dithered
        // shadow pass reads.
        if self.flags & FADE_PASS != 0 {
            self.shadow_fade = 1.0 - self.shown;
        }
        self.scene_draws = !self.table.draws_bodies();
        self.dithered = self
            .table
            .bodies
            .iter()
            .any(|body| body.flags & SHOWN_BIT != 0);
        self.table.advance(now_ms);
    }

    /// How the body's own meshes are drawn: by the scene while no show
    /// call has left bits 0x30 on them, else by the segment pass, blended
    /// at the shown amount when the entry runs its fade half.
    pub fn body_look(&self) -> BodyLook {
        if self.scene_draws {
            BodyLook::Scene
        } else if self.flags & FADE_PASS != 0 {
            BodyLook::Blend(self.shown)
        } else {
            BodyLook::Hidden
        }
    }

    /// The bodies' shadow fade while their fade bit (bit 4 of the node's
    /// cut-away word, set by the staging entries and cleared by the hide)
    /// dithers them into the shadow maps.
    pub fn shadow_dither(&self) -> Option<f32> {
        self.dithered.then_some(self.shadow_fade)
    }

    /// Whether the shell is drawn this frame.
    pub fn shell_drawn(&self) -> bool {
        self.holds_shell && self.shell.is_some()
    }

    /// Whether the segment pass's depth half draws the body's parts.
    pub fn depth_pass(&self) -> bool {
        self.flags & DEPTH_PASS != 0
    }

    /// Whether a body mesh without a blended form of its material is
    /// seen: always while the scene draws it, else only while the fade half
    /// shows it above one half.
    pub fn body_seen(&self) -> bool {
        self.scene_draws || (self.flags & FADE_PASS != 0 && self.shown > 0.5)
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
        tex_scale: shell.tex_scale,
        opacity: shell.opacity,
        push: SHELL_PUSH_M,
    }
}

/// The displacement shell a running Constant Displacement keeps on its
/// caster's body: the original's constant start makes the body's shell
/// (the same `_EffectShell01` node a vanish makes, its leave-to-finish flag
/// cleared), opening over 400 ms and resting open while the power runs; the
/// power's end asks it to close, and nothing lets it finish, so the closed
/// shell stays on the body drawing nothing (Exarust takes it away once
/// closed). The scene draws it (its `Interior` never written, 0). A vanish
/// cast while it runs takes this shell instead of making its own
/// ([`LiveVanish::lend`]) and runs it until the vanish goes.
#[derive(Component, Debug, Clone)]
pub struct ConstantShell {
    /// The displaced body.
    pub caster: Entity,
    pub shell: Shell,
    /// The vanish running the shell now, if one took it.
    pub lent_to: Option<Entity>,
    /// The shell has taken the effects' clock.
    pub started: bool,
    /// The power has ended and asked the shell to close.
    pub ended: bool,
}

impl ConstantShell {
    /// The shell a constant cast has just made, with its two creation
    /// draws; it starts on its first step.
    pub fn waiting(caster: Entity, tex_pos: [f32; 2]) -> Self {
        Self {
            caster,
            shell: Shell::new(0, tex_pos),
            lent_to: None,
            started: false,
            ended: false,
        }
    }

    /// One fixed step at `now_ms`; `running` says whether the caster's
    /// power list still holds the Constant Displacement. Its end asks the
    /// shell to close (only an opening or resting shell reacts). A lent
    /// shell is stepped by its vanish. Answers whether the closed shell can
    /// leave.
    pub fn step(&mut self, now_ms: u32, running: bool) -> bool {
        if !self.started {
            self.started = true;
            self.shell.stamp = now_ms;
        }
        if !running && !self.ended {
            self.ended = true;
            self.shell.start_fade();
        }
        if self.lent_to.is_none() {
            self.shell.step(now_ms);
        }
        self.ended
            && self.lent_to.is_none()
            && matches!(self.shell.state, ShellState::Ending | ShellState::Gone)
    }

    /// Whether the constant draws its shell this frame (a lent shell is
    /// drawn by its vanish).
    pub fn drawn(&self) -> bool {
        self.lent_to.is_none() && self.shell.state != ShellState::Gone
    }
}

/// Each fixed step after the casters' tick: every constant displacement's
/// shell steps, then every vanish follows its power's phase and runs its
/// frame (a vanish over a lent shell runs that shell and hands it back).
///
/// Debug tool (`EXA_VANISH_HOLD=<ms>`): each vanish's and constant shell's
/// clock stops that many milliseconds after its start, so a screenshot taken
/// any time later shows that moment of the effect (the powers' own phases
/// still run).
pub(crate) fn step_vanishes(
    mut commands: Commands,
    clock: Option<Res<EffectClock>>,
    casters: Query<&PowerCaster>,
    mut vanishes: Query<(Entity, &mut LiveVanish)>,
    mut constants: Query<(Entity, &mut ConstantShell)>,
    mut hold: Local<
        Option<(
            Option<u32>,
            bevy::platform::collections::HashMap<Entity, u32>,
        )>,
    >,
) {
    let Some(clock) = clock else {
        return;
    };
    let (hold_ms, starts) = hold.get_or_insert_with(|| {
        let ms = std::env::var("EXA_VANISH_HOLD")
            .ok()
            .and_then(|v| v.parse::<u32>().ok());
        if let Some(ms) = ms {
            info!("vanishes and constant shells held {ms} ms after their start (EXA_VANISH_HOLD)");
        }
        (ms, Default::default())
    });
    let mut now_of = |entity: Entity| match hold_ms {
        Some(ms) => {
            let start = *starts.entry(entity).or_insert(clock.now_ms);
            clock.now_ms.min(start.wrapping_add(*ms))
        }
        None => clock.now_ms,
    };
    for (entity, mut constant) in &mut constants {
        // A vanish gone with the shell gives it back.
        if constant.lent_to.is_some_and(|v| !vanishes.contains(v)) {
            constant.lent_to = None;
        }
        let running = casters.get(constant.caster).is_ok_and(|caster| {
            caster
                .active
                .iter()
                .any(|entry| entry.id == CONSTANT_DISPLACEMENT)
        });
        let now = now_of(entity);
        if constant.step(now, running) {
            debug!("constant displacement shell {entity:?}: closed");
            commands.entity(entity).despawn();
        }
    }
    for (entity, mut live) in &mut vanishes {
        let now = now_of(entity);
        let fresh = !live.started;
        live.start(now);
        // The vanish start takes a running constant displacement's shell.
        if fresh
            && live.summoner.is_none()
            && let Some((from, mut constant)) = constants
                .iter_mut()
                .find(|(_, c)| c.caster == live.caster && c.lent_to.is_none() && !c.ended)
        {
            constant.lent_to = Some(entity);
            let shell = constant.shell.clone();
            live.lend(shell, from);
            info!("vanish {entity:?}: takes the constant displacement's shell {from:?}");
        }
        // A lent shell lives on its constant: run it here, write it back.
        let mut lent = live
            .lent_from
            .and_then(|from| constants.get_mut(from).ok())
            .filter(|(_, c)| c.lent_to == Some(entity));
        if let Some((_, constant)) = &lent {
            live.shell = Some(constant.shell.clone());
        }
        // A summon's vanish follows its summoner's step.
        if let Some(summoner) = live.summoner {
            let step = casters.get(summoner).ok().and_then(|caster| {
                caster
                    .active
                    .iter()
                    .filter(|entry| entry.entity == Some(entity))
                    .find_map(|entry| match &entry.class {
                        ActiveClass::Summon(power) => Some(power.last),
                        _ => None,
                    })
            });
            if let Some(step) = step {
                live.follow_summon(&step, now);
            }
        } else {
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
                live.follow(&base, now);
            }
        }
        live.frame(now);
        debug!(
            "vanish {entity:?} at {} ms: entry {} ({:#x}), shown {:.3}, body {:?}, shadow {:?}, shell {:?}",
            now,
            live.table.phase,
            live.flags,
            live.shown,
            live.body_look(),
            live.shadow_dither(),
            live.shell.as_ref().map(|s| (s.state, s.shown)),
        );
        if let Some((_, constant)) = &mut lent
            && let Some(shell) = &live.shell
        {
            constant.shell = shell.clone();
        }
    }
}

/// The caster's own meshes: every mesh under it that is not a copy an
/// effect made.
type OwnMesh = (
    Without<VanishCopy>,
    Without<super::effect_draw::ShellCopy>,
    Without<crate::effects::scan_draw::ScanCopy>,
);

/// The materials of a vanish (or a constant displacement's shell) whose
/// copies are made: the shell, and the vanish's depth pass.
#[derive(Component, Debug, Clone)]
pub struct VanishDraw {
    pub shell: Handle<DispShellMaterial>,
    pub depth: Option<Handle<DispShellMaterial>>,
}

/// A copy of one of the caster's skinned meshes, drawn for the vanish (or
/// the constant displacement's shell) it names: the shell, or with `depth`
/// the depth pass.
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

/// A caster mesh the vanish's fade half draws blended: its own material
/// while the vanish holds it, and the blended copy it draws with.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct VanishFaded {
    pub by: Entity,
    pub original: Handle<ExaMaterial>,
    pub faded: Handle<ExaMaterial>,
}

/// A caster mesh the vanish dithers into the shadow maps
/// ([`ShadowDither`]).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanishDither {
    pub by: Entity,
}

/// The skinned meshes of `caster` a shell copies: each shown skinned mesh
/// under it (a part its apparel covers stays out of the copy).
#[allow(clippy::type_complexity)]
fn shell_sources(
    caster: Entity,
    children: &Query<&Children>,
    skinned: &Query<(&Mesh3d, &SkinnedMesh), OwnMesh>,
    shown: &Query<(&Visibility, Has<VanishHidden>), (With<Mesh3d>, OwnMesh)>,
) -> Vec<(Handle<Mesh>, SkinnedMesh)> {
    children
        .iter_descendants(caster)
        .filter_map(|e| {
            let (mesh, skin) = skinned.get(e).ok()?;
            let hidden = shown
                .get(e)
                .is_ok_and(|(v, ours)| *v == Visibility::Hidden && !ours);
            (!hidden).then(|| (mesh.0.clone(), skin.clone()))
        })
        .collect()
}

/// Spawns the shell copies of `sources` for `owner` (and, with `depth`,
/// the depth pass's copies), as children of `caster`: they leave with it,
/// and the caster's own meshes can hide without them.
fn spawn_copies(
    commands: &mut Commands,
    owner: Entity,
    caster: Entity,
    sources: &[(Handle<Mesh>, SkinnedMesh)],
    shell: &Handle<DispShellMaterial>,
    depth: Option<&Handle<DispShellMaterial>>,
) {
    for (mesh, skin) in sources {
        let depth_copy = depth.map(|m| (m, true, "_EffectShell01 depth pass"));
        for (material, is_depth, name) in
            depth_copy
                .into_iter()
                .chain([(shell, false, "_EffectShell01 copy")])
        {
            commands.spawn((
                Name::new(name),
                VanishCopy {
                    vanish: owner,
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
                ChildOf(caster),
            ));
        }
    }
}

/// Each frame: a vanish without copies copies its caster's shown skinned
/// meshes twice (the shell and the depth pass), a constant displacement's
/// shell once; every shell's material takes this frame's uniforms and its
/// copies show as its callbacks draw them.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(crate) fn draw_vanishes(
    mut commands: Commands,
    clock: Option<Res<EffectClock>>,
    game: Option<Res<exa_assets::Game>>,
    mut texture: Local<Option<Handle<Image>>>,
    images: Option<ResMut<Assets<Image>>>,
    vanishes: Query<(Entity, &LiveVanish, Option<&VanishDraw>)>,
    constants: Query<(Entity, &ConstantShell, Option<&VanishDraw>)>,
    bodies: Query<(Option<&Ragdoll>, &GlobalTransform)>,
    children: Query<&Children>,
    skinned: Query<(&Mesh3d, &SkinnedMesh), OwnMesh>,
    shown: Query<(&Visibility, Has<VanishHidden>), (With<Mesh3d>, OwnMesh)>,
    mut copy_meshes: Query<(&VanishCopy, &mut Visibility), (With<Mesh3d>, Without<VanishHidden>)>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    materials: Option<ResMut<Assets<DispShellMaterial>>>,
) {
    if vanishes.is_empty() && constants.is_empty() {
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
    // Every shell this frame: its owner, caster, shell, interior, whether
    // it has a depth pass, and its materials once made.
    let shells = vanishes
        .iter()
        .map(|(entity, live, drawn)| {
            (
                entity,
                live.caster,
                live.shell.as_ref(),
                live.interior(),
                true,
                drawn,
            )
        })
        .chain(constants.iter().map(|(entity, constant, drawn)| {
            (
                entity,
                constant.caster,
                constant.drawn().then_some(&constant.shell),
                0.0,
                false,
                drawn,
            )
        }));
    for (entity, caster, shell, interior, with_depth, drawn) in shells {
        let Ok((body, node)) = bodies.get(caster) else {
            continue;
        };
        let frame = body_frame(body, node);
        let bounds = body_bounds(body, &frame);
        let params =
            shell.map(|shell| shell_params(shell, interior, &frame, bounds, camera, clock.now_ms));
        match drawn {
            Some(draw) => {
                if let Some(params) = params
                    && let Some(mut material) = materials.get_mut(&draw.shell)
                {
                    material.params = params;
                }
            }
            None => {
                // A constant's shell lent to a vanish waits to come back.
                let Some(params) = params else {
                    continue;
                };
                let sources = shell_sources(caster, &children, &skinned, &shown);
                if sources.is_empty() {
                    debug!("shell {entity:?}: its caster has no skinned mesh yet");
                    continue;
                }
                info!(
                    "{} {entity:?}: displacement shell over {} skinned meshes of {caster:?}",
                    if with_depth {
                        "vanish"
                    } else {
                        "constant displacement"
                    },
                    sources.len(),
                );
                let shell = materials.add(DispShellMaterial {
                    params,
                    texture: image.clone(),
                    depth_only: false,
                });
                let depth = with_depth.then(|| {
                    materials.add(DispShellMaterial {
                        params: DispShellParams {
                            push: 0.0,
                            ..DispShellParams::default()
                        },
                        texture: image.clone(),
                        depth_only: true,
                    })
                });
                spawn_copies(
                    &mut commands,
                    entity,
                    caster,
                    &sources,
                    &shell,
                    depth.as_ref(),
                );
                commands.entity(entity).insert(VanishDraw { shell, depth });
            }
        }
    }
    for (copy, mut visibility) in &mut copy_meshes {
        let shown = if let Ok((_, live, _)) = vanishes.get(copy.vanish) {
            if copy.depth {
                // While the fade half draws the body blended, its copy lays
                // its own depth.
                live.depth_pass()
                    && live.shell.is_some()
                    && !matches!(live.body_look(), BodyLook::Blend(_))
            } else {
                live.shell_drawn()
            }
        } else if let Ok((_, constant, _)) = constants.get(copy.vanish) {
            !copy.depth && constant.drawn()
        } else {
            continue;
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

/// The blended form of a body part's material for the fade half: its
/// opacity replaced by the shown amount (the original sets the material's
/// opacity and switches a plain stage to its blended one), writing depth
/// while above zero so only its nearest layer shows, as the depth half
/// laid it.
fn faded_copy(original: &ExaMaterial, opacity: f32) -> ExaMaterial {
    let mut m = original.clone();
    if m.blend_state.is_none() && !matches!(m.alpha_mode, AlphaMode::Blend) {
        m.alpha_mode = AlphaMode::Blend;
    }
    m.params.flags2 |= flags2::CUT_AWAY_BLEND;
    m.params.cut_away_opacity = opacity;
    m
}

/// The caster's own meshes a vanish draws: their look and visibility.
type BodyMesh<'a> = (
    Entity,
    &'a mut Visibility,
    Option<&'a VanishHidden>,
    Option<&'a mut MeshMaterial3d<ExaMaterial>>,
    Option<&'a VanishFaded>,
    Option<&'a ShadowDither>,
    Option<&'a VanishDither>,
);

/// Each frame after the copies: the caster's own meshes take the vanish's
/// look. The scene draws them until a show call takes them; then the fade
/// half draws them blended at the shown amount (a part without a material
/// of the original's kind shows while that is above one half), and
/// otherwise they hide. While the bodies' fade bit is set they cast
/// dithered at the shadow fade.
#[allow(clippy::type_complexity)]
pub(crate) fn fade_vanish_bodies(
    mut commands: Commands,
    vanishes: Query<(Entity, &LiveVanish)>,
    children: Query<&Children>,
    mut own: Query<BodyMesh, (With<Mesh3d>, OwnMesh)>,
    materials: Option<ResMut<Assets<ExaMaterial>>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    for (entity, live) in &vanishes {
        let look = live.body_look();
        let dither = live.shadow_dither();
        for e in children.iter_descendants(live.caster) {
            let Ok((mesh, mut visibility, hidden, material, faded, shadow, dithered)) =
                own.get_mut(e)
            else {
                continue;
            };
            let ours = |by: Entity| by == entity;
            // The shadow's dither.
            match dither {
                Some(fade) if shadow.map(|s| s.0) != Some(fade) => {
                    if dithered.is_none_or(|d| ours(d.by)) {
                        commands
                            .entity(mesh)
                            .insert((ShadowDither(fade), VanishDither { by: entity }));
                    }
                }
                None if dithered.is_some_and(|d| ours(d.by)) => {
                    commands
                        .entity(mesh)
                        .remove::<(ShadowDither, VanishDither)>();
                }
                _ => {}
            }
            // The blended form of its material.
            let blend = match look {
                BodyLook::Blend(opacity) if material.is_some() => Some(opacity),
                _ => None,
            };
            match (blend, material, faded) {
                (Some(opacity), Some(mut material), None) => {
                    if let Some(own) = materials.get(&material.0) {
                        let copy = faded_copy(own, opacity);
                        let faded = materials.add(copy);
                        commands.entity(mesh).insert(VanishFaded {
                            by: entity,
                            original: material.0.clone(),
                            faded: faded.clone(),
                        });
                        material.0 = faded;
                    }
                }
                (Some(opacity), Some(_), Some(faded)) if ours(faded.by) => {
                    if materials
                        .get(&faded.faded)
                        .is_some_and(|m| m.params.cut_away_opacity != opacity)
                        && let Some(mut m) = materials.get_mut(&faded.faded)
                    {
                        m.params.cut_away_opacity = opacity;
                    }
                }
                (None, Some(mut material), Some(faded)) if ours(faded.by) => {
                    if material.0 == faded.faded {
                        material.0 = faded.original.clone();
                    }
                    materials.remove(&faded.faded);
                    commands.entity(mesh).remove::<VanishFaded>();
                }
                _ => {}
            }
            // Its visibility.
            let seen = match look {
                BodyLook::Scene => true,
                BodyLook::Blend(_) => blend.is_some() || live.body_seen(),
                BodyLook::Hidden => false,
            };
            match (seen, hidden) {
                (false, None) => {
                    commands.entity(mesh).insert(VanishHidden {
                        by: entity,
                        was: *visibility,
                    });
                    *visibility = Visibility::Hidden;
                }
                (true, Some(hidden)) if ours(hidden.by) => {
                    *visibility = hidden.was;
                    commands.entity(mesh).remove::<VanishHidden>();
                }
                _ => {}
            }
        }
    }
}

/// Removes the copies of a vanish (or a constant's shell) that has left and
/// gives the caster's meshes it hid, faded or dithered their own look back.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub(crate) fn drop_vanish_copies(
    mut commands: Commands,
    copies: Query<(Entity, &VanishCopy)>,
    mut hidden: Query<(Entity, &VanishHidden, &mut Visibility)>,
    mut faded: Query<(Entity, &VanishFaded, &mut MeshMaterial3d<ExaMaterial>)>,
    dithered: Query<(Entity, &VanishDither)>,
    vanishes: Query<(), With<LiveVanish>>,
    constants: Query<(), With<ConstantShell>>,
    materials: Option<ResMut<Assets<ExaMaterial>>>,
) {
    for (entity, copy) in &copies {
        if !vanishes.contains(copy.vanish) && !constants.contains(copy.vanish) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, hid, mut visibility) in &mut hidden {
        if !vanishes.contains(hid.by) {
            *visibility = hid.was;
            commands.entity(entity).remove::<VanishHidden>();
        }
    }
    let mut materials = materials;
    for (entity, f, mut material) in &mut faded {
        if !vanishes.contains(f.by) {
            if material.0 == f.faded {
                material.0 = f.original.clone();
            }
            if let Some(materials) = materials.as_mut() {
                materials.remove(&f.faded);
            }
            commands.entity(entity).remove::<VanishFaded>();
        }
    }
    for (entity, d) in &dithered {
        if !vanishes.contains(d.by) {
            commands
                .entity(entity)
                .remove::<(ShadowDither, VanishDither)>();
        }
    }
}

/// The shell's picture: the record's emissive layer from the game's
/// textures, its values read raw; a mid-grey texel when the install does
/// not have it.
pub(crate) fn load_texture(
    game: Option<&exa_assets::Game>,
    images: &mut Assets<Image>,
) -> Handle<Image> {
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

pub(crate) fn grey_texel() -> Image {
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

    /// The fade half draws the body blended at the shown amount, and the
    /// staging entries' fade bit dithers its shadow at one less the shown
    /// amount of the last fade-half frame.
    #[test]
    fn the_fade_half_blends_the_body_and_dithers_its_shadow() {
        let mut live = LiveVanish::new(Entity::from_bits(7), 0, [0.25, 0.5]);
        let run = |live: &mut LiveVanish, phase: i32, now: u32| {
            live.follow(&base(phase), now);
            live.frame(now);
        };
        run(&mut live, HIDDEN, 0);
        assert_eq!(live.body_look(), BodyLook::Scene);
        assert_eq!(live.shadow_dither(), None);
        // 350 ms: the stage call sets the fade bit, seen from the next
        // frame on, the shadow not yet thinned.
        run(&mut live, HIDDEN, 351);
        assert_eq!(live.body_look(), BodyLook::Scene);
        assert_eq!(live.shadow_dither(), None);
        run(&mut live, HIDDEN, 352);
        assert!(live.shadow_dither().is_some_and(|f| f < 0.05));
        // 350–450 ms: blended at the shown amount, the shadow thinning.
        run(&mut live, FADING_OUT, 400);
        let BodyLook::Blend(opacity) = live.body_look() else {
            panic!("{:?}", live.body_look());
        };
        assert!((opacity - 0.5).abs() < 0.02, "{opacity}");
        assert_eq!(live.shadow_dither(), Some(1.0 - opacity));
        // The entry's last frame past its end blends at nothing and drops
        // the whole shadow.
        run(&mut live, FADING_OUT, 460);
        assert_eq!(live.body_look(), BodyLook::Blend(0.0));
        // Hidden: depth only, the shadow kept at the last fade.
        run(&mut live, FADING_OUT, 470);
        assert_eq!(live.body_look(), BodyLook::Hidden);
        assert_eq!(live.shadow_dither(), Some(1.0));
        // The come-back fades the shadow in and the hide clears the bit.
        run(&mut live, FADING_OUT, 700);
        run(&mut live, HOLDING, 701);
        run(&mut live, FADING_IN, 2_000);
        run(&mut live, FADING_IN, 2_201);
        run(&mut live, FADING_IN, 2_300);
        let BodyLook::Blend(opacity) = live.body_look() else {
            panic!("{:?}", live.body_look());
        };
        assert!((opacity - 0.5).abs() < 0.02, "{opacity}");
        assert_eq!(live.shadow_dither(), Some(1.0 - opacity));
        // The hide at 400 ms gives the body back from the next frame.
        run(&mut live, TAIL, 2_401);
        run(&mut live, TAIL, 2_402);
        assert_eq!(live.body_look(), BodyLook::Scene);
        assert_eq!(live.shadow_dither(), None);
    }

    /// The constant displacement's shell opens over 400 ms, rests open
    /// while the power runs, and closes over 300 ms when it ends, never
    /// finishing on its own.
    #[test]
    fn a_constant_shell_opens_rests_and_closes_with_its_power() {
        let mut c = ConstantShell::waiting(Entity::from_bits(5), [0.5, 0.5]);
        assert!(!c.step(1_000, true));
        assert_eq!(c.shell.stamp, 1_000);
        c.step(1_200, true);
        assert_eq!(c.shell.shown, 0.5);
        c.step(1_401, true);
        assert_eq!(c.shell.state, ShellState::Resting);
        assert_eq!(c.shell.fade(), 0.0);
        assert!(c.drawn());
        // The power ends: the shell closes and may leave once closed.
        assert!(!c.step(5_000, false));
        assert!(!c.step(5_001, false));
        assert!(!c.step(5_150, false));
        assert!(c.step(5_302, false));
        assert!(!c.shell.can_finish);
    }

    /// A vanish over a running constant displacement takes its shell open
    /// and resting, closes it as the body goes and reopens it as the body
    /// comes back, but leaves it open at its end (the shell stays the
    /// constant's).
    #[test]
    fn a_vanish_runs_the_constant_shell_and_leaves_it_open() {
        let mut constant = ConstantShell::waiting(Entity::from_bits(5), [0.5, 0.5]);
        constant.step(0, true);
        constant.step(500, true);
        let mut live = LiveVanish::waiting(Entity::from_bits(5), [0.0; 2]);
        live.start(1_000);
        live.lend(constant.shell.clone(), Entity::from_bits(9));
        assert!(live.table.lent);
        assert_eq!(live.shell.as_ref().unwrap().state, ShellState::Resting);
        let shared = |phase: i32| VanishBase {
            shared: true,
            ..base(phase)
        };
        let run = |live: &mut LiveVanish, phase: i32, now: u32| {
            live.follow(&shared(phase), now);
            live.frame(now);
        };
        run(&mut live, HIDDEN, 1_000);
        run(&mut live, FADING_OUT, 1_400);
        assert_eq!(live.shell.as_ref().unwrap().state, ShellState::Fading);
        run(&mut live, FADING_OUT, 1_700);
        run(&mut live, HOLDING, 1_701);
        run(&mut live, FADING_IN, 3_000);
        run(&mut live, FADING_IN, 3_401);
        run(&mut live, TAIL, 3_402);
        run(&mut live, TAIL, 3_700);
        let shell = live.shell.as_ref().unwrap();
        assert_eq!(shell.state, ShellState::Resting);
        assert!(!shell.can_finish);
        assert_eq!(live.body_look(), BodyLook::Scene);
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

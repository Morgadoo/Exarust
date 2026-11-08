//! The thaumaturgy effects in the running game: arcs, barricades and
//! displacement wisps as entities whose simulations ([`super::arcs`],
//! [`super::barricade`], [`super::wisps`]) run at the body tick against the
//! physical bodies and the level, and whose meshes are rebuilt each frame.
//!
//! - **Each tick** (after the bodies): an arc follows its hand, ticks,
//!   ends at the level (the static colliders) and tests every other body's
//!   parts, sending [`ArcShocked`] for a struck character. A shock reaches
//!   a fighter once the game clock is within 150 ms of its next shock time
//!   ([`ShockReady`]), which then moves to 500 ms plus a draw below 200 ms
//!   of the engine's generator ahead; a fighter that is not collapsing or
//!   dead loses the strength in yellow health (floored at 0) and takes the
//!   shock-stagger flag, which the next body tick spends
//!   ([`exa_physics::shock::shock_stagger_tick`]). A barricade
//!   ticks on the game clock, ends once its caster is 20 m away, and
//!   shoves every body part it holds: each sphere's push moves the part's
//!   particles ([`exa_physics::body::Ragdoll::push_sphere`]), the body
//!   wakes for 32 ticks, and each part of a character struck in front
//!   knocks it back a step (doubled, at most its step length), sets its
//!   instability counter to 100 and, while its energy is above 0.25,
//!   lowers it, so a body struck by several parts loses that much for
//!   each. The part that plays the impact sound raises that character's
//!   group alert and adds concern of 0.5, at gain 1, toward the wall.
//!   While it stands its line is stamped on the navigation grid's
//!   penalty (`exa_world::nav::stamp_line`, from the first tick a grid is
//!   there) and taken back when its fall ends. Wisps follow their body
//!   parts.
//! - **Each frame**: the arcs' and wisps' ribbons and the barricade's slab
//!   are drawn with [`exa_render::glow::GlowMaterial`], a stand-in for the
//!   original's effect programs, which are not ported (the arcs' ribbon
//!   carries its texture coordinates). An effect that ends leaves the
//!   scene.
//! - **Debug** ([`ThaumDebug`], `--debug-arcs`, `--debug-barricade`,
//!   `--debug-wisps`): the Shock power's hand arcs on the player's right
//!   hand (respawned while they last), a barricade raised 2 m in front of
//!   the player, and the constant displacement's wisps on the player's
//!   body. The player cannot cast yet: Exarust has no power bar.
//!
//! A barricade, a blast and a wave are force entities the target pass weighs.
//! A held
//! weapon's strike against the wall breaks its holder's attack. A shock
//! weapon's arcs
//! (light points) place their
//! tendril light each frame. An emitter that carries a voxel light places
//! it at the arcs' mean and sets its intensity from the frame.
//! Rules: `docs/THAUMATURGY.md`.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::NoFrustumCulling;
use bevy::mesh::{Indices, PrimitiveTopology};
use bevy::prelude::*;
use bevy::transform::TransformSystems;
use exa_ai::brain::{CharacterId, Npc, RoleDatabase};
use exa_ai::combatant::{self, Contact};
use exa_ai::contacts::ContactRecord;
use exa_ai::manager::raise_alert;
use exa_ai::roles::{Partner, Relations, Roles};
use exa_combat::swing::SwingState;
use exa_combat::{Condition, Fighter, StateFlags};
use exa_physics::Physics;
use exa_physics::body::locomotion::Locomotor;
use exa_physics::body::{BodySystems, BodyTicks, Ragdoll, step_length};
use exa_render::electricity::ArcMaterial;
use exa_render::gi::VoxelLight;
use exa_render::glow::GlowMaterial;
use exa_world::nav::{NavGrid, NavPatch, stamp_line, unstamp_line};

use super::arcs::{ARC_COLOUR, ArcEmitter, ArcLight, ArcLimb, HAND_ARCS_OFFSET, hand_arcs};
use super::barricade::{
    Barricade, GridFrame, WAKE_TICKS, WallBody, WallBodyKind, WallFrame, barricade_mesh, nav_stamp,
    power_ends, slide_mesh,
};
use super::displacement::body_wisps;
use super::engine::{Roll, normalize};
use super::force::{GroundHit, place_barricade};
use super::wisps::{Strand, Wisps};
use crate::arena::Mt19937;
use crate::player::PlayerBody;

/// Centimetres a metre: the original's effects run in cm.
const CM: f32 = 100.0;
/// A shock reaches a fighter this many ms before its next shock time.
pub const SHOCK_EARLY_MS: u64 = 150;
/// After a shock the next one is due this many ms later ...
pub const SHOCK_SPACING_MS: u64 = 500;
/// ... plus a draw below this many ms of the engine's generator.
pub const SHOCK_SPREAD_MS: u64 = 200;
/// The chest's bone and the right hand's.
const CHEST: usize = 9;
const RIGHT_HAND: usize = 17;

pub struct ThaumEffectsPlugin;

impl Plugin for ThaumEffectsPlugin {
    fn build(&self, app: &mut App) {
        bridge_thaum_effects_types(app);
        exa_physics::sim_app::add_frame_commands(app);
        app.add_message::<ArcShocked>()
            .insert_resource(EffectDraws(Mt19937::new(5489)))
            .init_resource::<EffectRoll>()
            .init_resource::<FrameEffectDraws>()
            .init_resource::<FrameEffectRoll>()
            .init_resource::<super::effect_draw::ForceNoise>()
            .add_systems(
                FixedUpdate,
                (tick_arcs, tick_barricades, tick_wisps, shock_fighters)
                    .chain()
                    .after(BodySystems),
            )
            .add_systems(
                exa_ai::BrainChain,
                see_force_effects.before(exa_ai::brain::ai_tick),
            )
            .add_systems(
                Update,
                (
                    spawn_debug_effects.run_if(resource_exists::<ThaumDebug>),
                    super::effect_light::debug_force_casts.run_if(
                        |debug: Option<Res<ThaumDebug>>| debug.is_some_and(|d| d.cast_every > 0),
                    ),
                ),
            )
            .add_systems(
                PostUpdate,
                (
                    draw_arcs,
                    draw_barricades,
                    draw_wisps,
                    (
                        super::effect_draw::tick_force_noise
                            .run_if(super::effect_draw::force_effects_alive),
                        (
                            super::blast_live::draw_blasts,
                            super::torrent_live::draw_torrents,
                            super::effect_draw::draw_shell_auras,
                            super::effect_draw::draw_rigid_auras,
                            super::effect_draw::drop_shell_copies,
                            crate::effects::scan_draw::draw_scans,
                            crate::effects::scan_draw::drop_scan_copies,
                            crate::effects::bubble_draw::draw_bubbles,
                            crate::effects::bubble_draw::drop_bubble_draws,
                            (
                                crate::effects::dome_draw::draw_domes,
                                super::dome_sound::sound_domes,
                            )
                                .chain(),
                            super::shell_hum::sound_shell_hums
                                .after(super::effect_draw::draw_rigid_auras),
                            crate::effects::dome_draw::drop_dome_draws,
                            super::vanish_live::draw_vanishes,
                            super::vanish_live::drop_vanish_copies,
                        ),
                        (
                            super::effect_light::blast_lights,
                            super::effect_light::torrent_lights,
                            super::effect_light::bubble_lights,
                            super::effect_light::barrier_lights,
                            super::effect_light::force_bubble_lights,
                            super::effect_light::barricade_lights,
                            super::effect_light::shell_lights,
                            super::effect_light::drop_orphan_lights,
                        )
                            .chain(),
                    )
                        .chain(),
                )
                    .after(TransformSystems::Propagate),
            );
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_thaum_effects_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_message::<exa_combat::impact::LimbImpact>()
        .bridge_message::<crate::thaum::cast::Otherwhere>()
        .bridge_message::<crate::thaum::hold_live::HoldEnd>()
        .bridge_message::<crate::thaum::kinesis_live::KinesisPicked>()
        .bridge_message::<crate::thaum::live::ArcShocked>()
        .bridge_message::<crate::thaum::throw_live::ThrowFade>()
        .bridge_copy::<exa_ai::DuelPowers>()
        .bridge_copy_opaque_resource::<exa_assets::Game>()
        .bridge_copy::<exa_character::held::HeldMoveset>()
        .bridge_copy::<crate::effects::danger::Hazard>()
        .bridge_copy_in_resource::<crate::player::CursorGround>()
        .bridge_copy::<crate::thaum::barrier_live::LiveBarrier>()
        .bridge_copy::<crate::thaum::blast_live::LiveBlast>()
        .bridge_copy::<crate::thaum::bubble_live::LiveBubble>()
        .bridge_copy::<crate::thaum::cast::LiveShellAura>()
        .bridge_copy::<crate::thaum::cast::PowerCaster>()
        .bridge_copy::<crate::thaum::hold_live::StrikesAura>()
        .bridge_copy_in_resource::<crate::thaum::kinesis_live::KinesisCursor>()
        .bridge_copy_in_resource::<crate::thaum::kinesis_live::KinesisKeys>()
        .bridge_snapshot_resource::<crate::thaum::kinesis_live::KinesisPickHook>()
        .bridge_move_resource::<crate::thaum::live::EffectDraws>()
        .bridge_move_resource::<crate::thaum::live::EffectRoll>()
        .bridge_copy::<crate::thaum::live::LiveArcs>()
        .bridge_copy::<crate::thaum::live::LiveBarricade>()
        .bridge_copy::<crate::thaum::live::LiveWisps>()
        .bridge_copy::<crate::thaum::live::NavStamped>()
        .bridge_copy::<crate::thaum::live::ShockReady>()
        .bridge_copy::<crate::thaum::torrent_live::LiveTorrent>()
        .bridge_copy::<crate::thaum::user_stamp::UserStamp>()
        .bridge_copy::<crate::thaum::vanish_live::LiveVanish>()
        .bridge_copy::<crate::thaum::wave_live::LiveWave>()
        .bridge_copy::<exa_physics::grip::GripRelease>()
        .bridge_copy::<exa_physics::prop::PropBody>()
        .bridge_copy_in_resource::<exa_physics::prop::StandIns>();
}

/// The runtime's random stream the effects draw from.
#[derive(Resource, Debug)]
pub struct EffectDraws(pub Mt19937);

impl Default for EffectDraws {
    /// The stream the effects start from.
    fn default() -> Self {
        Self(Mt19937::new(5489))
    }
}

/// The engine's own generator, for the effects that use it.
#[derive(Resource, Default, Debug)]
pub struct EffectRoll(pub Roll);

/// The draws of the effects the frame runs (their drawing, the GUI's casts,
/// the items' abilities): the original's run-time stream and engine
/// generator are process-wide, drawn from by its main thread and its
/// simulation thread with no order between them, so each side keeps its
/// own sequence here, in its own order. [Exarust: the original interleaves
/// the two threads' draws as their timing falls.]
#[derive(Resource, Default, Debug)]
pub struct FrameEffectDraws(pub EffectDraws);

/// The frame's engine generator for its effects ([`FrameEffectDraws`]).
#[derive(Resource, Default, Debug)]
pub struct FrameEffectRoll(pub EffectRoll);

impl std::ops::Deref for FrameEffectDraws {
    type Target = EffectDraws;
    fn deref(&self) -> &EffectDraws {
        &self.0
    }
}

impl std::ops::DerefMut for FrameEffectDraws {
    fn deref_mut(&mut self) -> &mut EffectDraws {
        &mut self.0
    }
}

impl std::ops::Deref for FrameEffectRoll {
    type Target = EffectRoll;
    fn deref(&self) -> &EffectRoll {
        &self.0
    }
}

impl std::ops::DerefMut for FrameEffectRoll {
    fn deref_mut(&mut self) -> &mut EffectRoll {
        &mut self.0
    }
}

/// Which test effects to put on the player (the debug flags).
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ThaumDebug {
    pub arcs: bool,
    pub barricade: bool,
    pub wisps: bool,
    /// Seconds between the player's debug casts of its first power (0:
    /// none), [`super::effect_light::debug_force_casts`].
    pub cast_every: u32,
}

/// A character an arc struck.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct ArcShocked {
    pub body: Entity,
    pub direction: Vec3,
    pub strength: f32,
}

/// When a fighter's next shock is due (ms of game time); a fighter without
/// one has never been shocked.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShockReady(pub u64);

/// What a shock that lands does: when the next one is due, the yellow health
/// left, and the hit sound's pitch and gain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShockTaken {
    pub next: u64,
    pub yellow: f32,
    pub pitch: f32,
    pub gain: f32,
    /// The pain cry when the shock burns: its gain, then its pitch. A release
    /// and a collapsing body draw none.
    pub cry: Option<(f32, f32)>,
}

/// The shock reaction's clock and health rule, as the original's character
/// takes a shock: `None` while the shock comes too early, and that refusal
/// draws nothing. A shock that lands first draws the hit sound (pitch from
/// the first draw, gain from the second). A released race (`released`, and
/// not dying) is not burnt: its next shock is 500 ms on and its health is
/// kept. Every other shock then draws the extra delay below 200 ms.
#[allow(clippy::too_many_arguments)]
pub fn shock_reaction(
    ready: Option<u64>,
    now: u64,
    yellow: f32,
    strength: f32,
    dying: bool,
    released: bool,
    rings: bool,
    roll: &mut Roll,
) -> Option<ShockTaken> {
    if ready.is_some_and(|next| next > now + SHOCK_EARLY_MS) {
        return None;
    }
    let pitch = (f64::from(roll.unit()) * 0.4 + 0.8) as f32;
    let gain = (f64::from(roll.unit()) * 0.1 + 0.3) as f32;
    // A race that only rings, and a released race, take the flat 500 ms and
    // are not burnt. The hit sound was already drawn.
    let quiet = rings || (released && !dying);
    let (next, yellow) = if quiet {
        (now + SHOCK_SPACING_MS, yellow)
    } else {
        let spread = (u64::from(roll.step()) * SHOCK_SPREAD_MS) >> 32;
        let left = if dying {
            yellow
        } else {
            let left = yellow - strength;
            if left < 0.0 { 0.0 } else { left }
        };
        (now + SHOCK_SPACING_MS + spread, left)
    };
    // A burn cries after the delay: the first draw is the pitch, the second
    // the gain. A release and a collapsing body do not.
    let cry = if !dying && !quiet {
        let first = roll.unit();
        let second = roll.unit();
        Some((
            (f64::from(second) * 0.2 + 0.4) as f32,
            (f64::from(first) * 0.2 + 0.8) as f32,
        ))
    } else {
        None
    };
    Some(ShockTaken {
        next,
        yellow,
        pitch,
        gain,
        cry,
    })
}

/// Arcs in the scene: the body and bone whose part they ride (its place is
/// their frame's anchor, the hand arcs' node offset turned with it), and
/// the body that casts them.
#[derive(Component, Clone, Debug)]
pub struct LiveArcs {
    pub arcs: ArcEmitter,
    pub hand: Option<(Entity, usize)>,
    /// The held weapon these arcs ride, when they are a shock weapon's.
    pub weapon: Option<Entity>,
    pub caster: Option<Entity>,
    /// Kept in the scene when its tendrils are gone (the test arcs).
    pub keep: bool,
}

/// A barricade in the scene and who cast it.
#[derive(Component, Clone, Debug)]
pub struct LiveBarricade {
    pub wall: Barricade,
    pub caster: Option<Entity>,
}

/// Wisps on a body: each strand rides the part of the body at its slot.
#[derive(Component, Clone, Debug)]
pub struct LiveWisps {
    pub wisps: Wisps,
    pub body: Entity,
    pub parts: Vec<usize>,
}

/// The mesh an effect draws.
#[derive(Component)]
pub struct EffectMesh(pub Handle<Mesh>);

/// The patch a barricade's line stamped on the navigation grid, taken back
/// when its fall ends.
#[derive(Component, Clone, Debug)]
pub struct NavStamped(pub NavPatch);

/// The world place (cm) of a point given in a part's space (cm).
fn part_point(body: &Ragdoll, part: usize, local_cm: Vec3) -> Option<Vec3> {
    let p = body.parts.get(part)?;
    Some(p.transform().transform_point3(local_cm / CM) * CM)
}

fn bone_part(body: &Ragdoll, bone: usize) -> Option<usize> {
    body.parts.iter().position(|p| p.bone == bone)
}

pub(crate) fn tick_arcs(
    mut arcs: Query<&mut LiveArcs>,
    bodies: Query<(Entity, &Ragdoll)>,
    physics: Option<Res<Physics>>,
    mut draws: ResMut<EffectDraws>,
    mut shocked: MessageWriter<ArcShocked>,
) {
    for mut live in &mut arcs {
        let live = &mut *live;
        if let Some((body, bone)) = live.hand {
            let anchor = bodies
                .get(body)
                .ok()
                .and_then(|(_, b)| part_point(b, bone_part(b, bone)?, HAND_ARCS_OFFSET));
            live.arcs.anchor = anchor.or(live.arcs.anchor);
        }
        if let Some(weapon) = live.weapon
            && let Ok((_, body)) = bodies.get(weapon)
            && let Some(part) = body.parts.iter().find(|part| !part.spheres.is_empty())
        {
            let placed = part.transform();
            live.arcs.anchor = Some(Vec3::from(placed.translation) * CM);
            let (_, rotation, _) = placed.to_scale_rotation_translation();
            live.arcs.frame = rotation;
            // The spheres stay in the weapon's own frame: a point more than
            // 10 cm along its local y is where a tendril may start, turned
            // with the weapon.
            live.arcs.source_spheres = part
                .spheres
                .iter()
                .map(|(centre, _)| *centre * CM)
                .collect();
        }
        live.arcs.tick(&mut draws.0);
        if let Some(physics) = physics.as_deref() {
            live.arcs.clip_to_level(|start, dir, len| {
                physics
                    .fixed_hit(start / CM, dir, len / CM)
                    .map(|hit| hit * CM)
            });
        }
        for (entity, body) in &bodies {
            if Some(entity) == live.caster || body.drive.is_none() {
                continue;
            }
            let chest = bone_part(body, CHEST)
                .and_then(|k| part_point(body, k, Vec3::ZERO))
                .unwrap_or(Vec3::ZERO);
            for part in &body.parts {
                let world = part.transform();
                let limb = ArcLimb {
                    character: entity.to_bits(),
                    caster: false,
                    at: Vec3::from(world.translation) * CM,
                    chest,
                };
                // Each segment is the limb mesh's swept query: a 20 cm
                // sphere along it, in the limb's frame.
                let hit = live.arcs.strike_limb(
                    &limb,
                    |probe| {
                        exa_physics::body::pairs::swept_touches(
                            part,
                            probe.start / CM,
                            probe.end / CM,
                            probe.reach / CM,
                        )
                    },
                    &mut draws.0,
                );
                if let Some(shock) = hit {
                    debug!(
                        "arcs shock {entity} (strength {}) through a limb at {:?}",
                        shock.strength, limb.at
                    );
                    shocked.write(ArcShocked {
                        body: entity,
                        direction: shock.direction,
                        strength: shock.strength,
                    });
                }
            }
        }
    }
}

#[allow(clippy::type_complexity)]
pub(crate) fn shock_fighters(
    mut shocks: MessageReader<ArcShocked>,
    mut fighters: Query<(
        &mut Fighter,
        Option<&mut ShockReady>,
        Option<&mut Condition>,
        Option<&mut StateFlags>,
        Option<&mut Ragdoll>,
        Option<&mut exa_physics::body::locomotion::Locomotion>,
        Option<&mut exa_physics::body::locomotion::Locomotor>,
        Option<&mut exa_anim::Animator>,
        Option<&mut exa_combat::voice::CharacterVoice>,
        Option<&exa_character::record::CharacterData>,
    )>,
    ticks: Option<Res<BodyTicks>>,
    motions: Option<Res<exa_anim::Motions>>,
    mut roll: ResMut<EffectRoll>,
    mut commands: Commands,
    (game, mut frame): (
        Option<Res<exa_assets::Game>>,
        ResMut<exa_physics::sim_app::FrameCommands>,
    ),
) {
    let now = ticks.map_or(0, |t| t.millis());
    // Next shock times set this tick on fighters that had none yet (their
    // component arrives with the commands).
    let mut set: Vec<(Entity, u64)> = Vec::new();
    for shock in shocks.read() {
        let Ok((
            mut fighter,
            ready,
            mut condition,
            flags,
            body,
            mut locomotion,
            mut locomotor,
            mut animator,
            mut voice,
            character,
        )) = fighters.get_mut(shock.body)
        else {
            continue;
        };
        let dying = condition.as_deref().is_some_and(|c| c.dying());
        let known = ready.as_deref().map(|r| r.0).or_else(|| {
            set.iter()
                .rev()
                .find(|(e, _)| *e == shock.body)
                .map(|&(_, next)| next)
        });
        let Some(taken) = shock_reaction(
            known,
            now,
            fighter.health.yellow,
            shock.strength,
            dying,
            fighter.stats.shocks_release,
            character.is_some_and(|c| c.record.race == 1),
            &mut roll.0,
        ) else {
            continue;
        };
        let next = taken.next;
        let yellow = taken.yellow;
        let chest = body
            .as_deref()
            .and_then(|b| b.drive.as_ref())
            .and_then(|d| d.balance().chest());
        // The frame's bank plays the hit and the cry the step decides on.
        let (gain, pitch) = (taken.gain, taken.pitch);
        frame.send(move |world| {
            exa_audio::step_sounds::play_clip_named(world, "elechit", chest, gain, pitch);
        });
        let rings = character.is_some_and(|c| c.record.race == 1);
        if fighter.stats.shocks_release && !dying {
            // A release race is not burnt: it is put into the shocked
            // state, its step ends and its clip rate stops.
            if let Some(mut body) = body {
                body.force_release(now as u32);
            }
            if let Some(condition) = condition.as_deref_mut() {
                *condition = Condition::SHOCKED;
            }
        } else if !rings {
            fighter.health.yellow = yellow;
            if !dying {
                if let Some(condition) = condition.as_deref_mut() {
                    *condition = Condition::NORMAL;
                }
                if let Some(mut flags) = flags {
                    flags.0 |= StateFlags::SHOCK_STAGGER;
                }
            }
            let cry_line = if body.as_deref().is_some_and(|b| b.second_grip()) {
                1
            } else {
                2
            };
            if !dying && let Some(mut body) = body {
                body.shock_stagger(shock.direction);
            }
            if !dying {
                // The walk stops and the Defence stance starts with no blend.
                if let Some(req) = locomotion.as_deref_mut() {
                    req.moving = false;
                    req.free_walk = false;
                }
                if let Some(motor) = locomotor.as_deref_mut() {
                    motor.amount = 0.0;
                }
                if let (Some(motions), Some(animator)) =
                    (motions.as_deref(), animator.as_deref_mut())
                {
                    motions.start_requested(animator, 3);
                }
                if let Some((gain, pitch)) = taken.cry
                    && let Some(voice) = voice.as_deref_mut()
                {
                    let name = voice.set(cry_line).unwrap_or_default().to_string();
                    let found = game
                        .as_deref()
                        .is_some_and(|g| exa_audio::SoundBank::set_size(g, &name) > 0);
                    let answer = voice.speak(cry_line, found, now as u32, gain, pitch);
                    if let exa_combat::voice::Answer::Played(spoken) = answer
                        && let Some(at) = chest
                    {
                        frame.send(move |world| {
                            exa_audio::step_sounds::play_set_named(
                                world,
                                &name,
                                false,
                                at,
                                spoken.volume,
                                spoken.pitch,
                            );
                        });
                    }
                }
            }
        }
        match ready {
            Some(mut ready) => ready.0 = next,
            None => {
                set.push((shock.body, next));
                commands.entity(shock.body).insert(ShockReady(next));
            }
        }
    }
}

/// One frame of an arc hum. `voice` is the looping voice, if it has started.
fn drive_arc_hum(
    audio: &mut ShockAudio,
    voice: &mut Option<u32>,
    name: Option<&str>,
    at: Vec3,
    gain: f32,
    pitch: Option<f32>,
    stop: bool,
) {
    if stop {
        if let Some(id) = voice.take()
            && let Some(env) = audio.env.as_mut()
            && let Some(playing) = env.voice_mut(id)
        {
            playing.flags |= exa_audio::voice_flags::STOP;
        }
        return;
    }
    let Some(name) = name else {
        return;
    };
    let (Some(game), Some(bank), Some(sources), Some(env)) = (
        audio.game.as_deref(),
        audio.bank.as_deref_mut(),
        audio.sources.as_deref_mut(),
        audio.env.as_deref_mut(),
    ) else {
        return;
    };
    if voice.is_none() {
        let set = bank.load_set(game, sources, name);
        *voice = env.play_set(
            bank,
            set,
            at,
            gain,
            pitch.unwrap_or(1.0),
            exa_audio::voice_flags::LOOPING,
        );
        return;
    }
    if let Some(id) = *voice
        && let Some(playing) = env.voice_mut(id)
    {
        apply_arc_hum(playing, gain, pitch, at);
    }
}

/// The frame's hum on a voice that is already looping: its gain, its pitch
/// when the arcs are filled enough to have one, and where it sits.
fn apply_arc_hum(voice: &mut exa_audio::Voice, gain: f32, pitch: Option<f32>, at: Vec3) {
    voice.gain = gain;
    if let Some(pitch) = pitch {
        voice.pitch = pitch;
    }
    voice.position = at;
}

/// The voxel light an arc emitter owns.
#[derive(Component, Debug, Clone, Copy)]
struct HeldVoxel(Entity);

/// Bits the frame sets on a voxel light's flag word after the colour update.
const VOXEL_FRAME_FLAGS: u32 = 9;

/// The frame's voxel light, at the arcs' mean. A point light is not this.
/// An ended frame drops it. An emitter with no lamp (the kind set on its
/// own) places nothing.
fn drive_arc_voxel(
    commands: &mut Commands,
    entity: Entity,
    held: Option<&HeldVoxel>,
    lights: &mut Query<(&mut VoxelLight, &mut Transform, &mut GlobalTransform), Without<Camera3d>>,
    arcs: &ArcEmitter,
    strength: f32,
    stop: bool,
) {
    let Some(lamp) = arcs.voxel else {
        return;
    };
    if stop {
        if let Some(held) = held {
            commands.entity(held.0).despawn();
            commands.entity(entity).remove::<HeldVoxel>();
        }
        return;
    }
    let at = arcs.mean / CM;
    let place = Transform::from_translation(at);
    match held.and_then(|held| lights.get_mut(held.0).ok()) {
        Some((mut light, mut transform, mut global)) => {
            light.set_intensity(strength);
            light.flags |= VOXEL_FRAME_FLAGS;
            *transform = place;
            *global = GlobalTransform::from(place);
        }
        None => {
            let mut light = VoxelLight::new(lamp.color, strength);
            light.base_intensity = lamp.base_intensity;
            light.radius = lamp.radius / CM;
            light.active_radius = light.radius;
            light.flags |= VOXEL_FRAME_FLAGS;
            let id = commands
                .spawn((
                    Name::new("arc voxel light"),
                    light,
                    place,
                    GlobalTransform::from(place),
                    ChildOf(entity),
                ))
                .id();
            commands.entity(entity).insert(HeldVoxel(id));
        }
    }
}

/// The frame's tendril light for arcs that gather light points. A point light
/// and a voxel light are not this. An empty frame, or one that has ended,
/// drops the light.
fn drive_arc_light(
    commands: &mut Commands,
    entity: Entity,
    held: Option<&super::effect_light::EffectLight>,
    shapes: &mut Query<(
        &mut exa_render::gi::GiShape,
        &mut exa_render::gi::ShapeVolume,
    )>,
    arcs: &ArcEmitter,
    strength: f32,
    stop: bool,
) {
    if arcs.light_kind != ArcLight::Points {
        return;
    }
    if stop || arcs.lights.is_empty() {
        if let Some(held) = held {
            commands.entity(held.0).despawn();
            commands
                .entity(entity)
                .remove::<super::effect_light::EffectLight>();
        }
        return;
    }
    let points: Vec<_> = arcs.lights.iter().map(|p| (p.at, p.weight)).collect();
    let Some(placed) =
        super::effect_light::arc_shape(&points, strength, super::effect_light::GI_LIGHT_SCALE)
    else {
        return;
    };
    super::effect_light::place_light(commands, entity, held, shapes, placed, "arc light");
}

/// What plays `elechit`. Absent in a test with no game loaded.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ShockAudio<'w> {
    game: Option<Res<'w, exa_assets::Game>>,
    bank: Option<ResMut<'w, exa_audio::SoundBank>>,
    sources: Option<ResMut<'w, bevy::asset::Assets<bevy::audio::AudioSource>>>,
    env: Option<ResMut<'w, exa_audio::AudioEnvironment>>,
}

/// Concern a front-limb impact adds toward the wall, and the gain it is
/// added with.
const WALL_CONCERN: f32 = 0.5;
const WALL_CONCERN_GAIN: f32 = 1.0;

/// The impact's concern, on the contact the sight pass keeps for the wall.
/// The attitude it scales by is the contact's soothing term.
fn raise_wall_concern(record: &mut ContactRecord, aggression: f32) {
    let mut contact = Contact::new(Entity::PLACEHOLDER);
    contact.concern = record.concern;
    contact.attitude = record.soothe;
    contact.flags = record.flags;
    combatant::add_concern(&mut contact, aggression, WALL_CONCERN, WALL_CONCERN_GAIN);
    record.concern = contact.concern;
    record.flags = contact.flags;
}

/// The queries the force sight reads besides the watching brain.
#[derive(bevy::ecs::system::SystemParam)]
struct ForceSight<'w, 's> {
    hazards: Query<'w, 's, (Entity, &'static crate::effects::danger::Hazard)>,
    blasts: Query<'w, 's, &'static super::blast_live::LiveBlast>,
    waves: Query<'w, 's, &'static super::wave_live::LiveWave>,
    walls: Query<'w, 's, &'static LiveBarricade>,
    ids: Query<'w, 's, (Entity, &'static CharacterId)>,
    roles: Query<'w, 's, &'static Roles>,
    db: Option<Res<'w, RoleDatabase>>,
    ticks: Option<Res<'w, BodyTicks>>,
}

/// The last attitude the viewer keeps about whoever raised the effect. No
/// stored relation, and no character to keep one about, leaves it 0. The
/// mind powers are applied by the caller.
fn caster_relation(
    caster: Entity,
    relations: Option<&mut Relations>,
    ids: &Query<(Entity, &CharacterId)>,
    role_sets: &Query<&Roles>,
    locale: u32,
    now: u32,
) -> f32 {
    if let Some(relations) = relations
        && let Ok((_, id)) = ids.get(caster)
        && let Ok(roles) = role_sets.get(caster)
    {
        let partner = Partner {
            id: id.0,
            roles,
            former: None,
        };
        relations.last_attitude(&partner, now, locale)
    } else {
        0.0
    }
}

/// Who raised a force effect, when the live effect is still in the scene.
fn effect_caster(
    entity: Entity,
    blasts: &Query<&super::blast_live::LiveBlast>,
    waves: &Query<&super::wave_live::LiveWave>,
    walls: &Query<&LiveBarricade>,
) -> Option<Entity> {
    if let Ok(blast) = blasts.get(entity) {
        return Some(blast.caster);
    }
    if let Ok(wave) = waves.get(entity) {
        return Some(wave.caster);
    }
    walls.get(entity).ok().and_then(|wall| wall.caster)
}

/// Each brain's sight of the force effects: a barricade, a blast and a wave
/// within 12 m. The assessment's fear is the contact's alarm, scaled by the
/// viewer's relation to whoever raised it. A blast the brain itself cast
/// keeps only the cleared fear.
fn see_force_effects(
    sight: ForceSight,
    powers: Query<&combatant::MindPowers>,
    casts: Query<&crate::mind::casts::MindBody>,
    power_casters: Query<&super::cast::PowerCaster>,
    bodies: Query<&Ragdoll>,
    mut watchers: Query<(Entity, &mut Npc, Option<&mut Relations>)>,
) {
    let by_id: std::collections::HashMap<u32, Entity> = sight
        .ids
        .iter()
        .map(|(entity, id)| (id.0, entity))
        .collect();
    let effects: Vec<_> = sight.hazards.iter().collect();
    let now = sight
        .ticks
        .as_ref()
        .map_or(0, |ticks| ticks.millis() as u32);
    let locale = sight.db.as_ref().map_or(0, |db| db.locale);
    for (who, mut npc, mut relations) in &mut watchers {
        npc.force_sights.clear();
        npc.wall_steps.clear();
        npc.blast_steps.clear();
        let Some(body) = bodies.get(who).ok().filter(|body| body.drive.is_some()) else {
            continue;
        };
        let chest = body_chest(body);
        let facing = body
            .parts
            .first()
            .map(|part| {
                let (_, rotation, _) = part.transform().to_scale_rotation_translation();
                rotation * Vec3::Z
            })
            .unwrap_or(Vec3::Z);
        let viewer = crate::effects::danger::Viewer {
            chest,
            look: facing,
            facing,
        };
        // The resistance technique averages a binding with the attitude it
        // found. It is the technique bit on the viewer's cast body.
        let resist = casts
            .get(who)
            .is_ok_and(|body| body.cast.techniques & crate::mind::casts::RESISTS_CONTROL != 0)
            || power_casters.get(who).is_ok_and(|caster| {
                caster.body.techniques & crate::mind::casts::RESISTS_CONTROL != 0
            });
        for (effect, hazard) in &effects {
            let caster = effect_caster(*effect, &sight.blasts, &sight.waves, &sight.walls);
            let mut mind = powers
                .get(who)
                .map(|powers| powers.effects.clone())
                .unwrap_or_default();
            if let Ok(body) = casts.get(who) {
                mind.extend(body.sight_effects(|id| by_id.get(&id).copied()));
            }
            let relation = caster.map_or(0.0, |caster| {
                let raw = caster_relation(
                    caster,
                    relations.as_deref_mut(),
                    &sight.ids,
                    &sight.roles,
                    locale,
                    now,
                );
                combatant::trust(caster, raw, resist, &mind)
            });
            let Some(seen) = hazard.assess(&viewer, relation, caster == Some(who)) else {
                continue;
            };
            let id = effect.to_bits();
            let mut record = npc
                .contacts
                .iter()
                .find(|contact| contact.id == id)
                .copied()
                .unwrap_or(ContactRecord {
                    id,
                    ..ContactRecord::default()
                });
            record.object = true;
            if seen.flags & crate::effects::danger::SEEN_FLAG != 0 {
                record.flags |= combatant::CONTACT_NOTICED;
            }
            record.alarm = seen.fear;
            let at = match hazard {
                crate::effects::danger::Hazard::Barricade { wall, .. } => {
                    npc.wall_steps.push(exa_ai::combat::WallStep {
                        centre: wall.centre,
                        along: wall.along,
                        half_width: wall.half_width,
                    });
                    crate::effects::danger::wall_point(wall, chest)
                }
                crate::effects::danger::Hazard::Blast { at, released } => {
                    if let Ok(live) = sight.blasts.get(*effect) {
                        let caster_body = bodies.get(live.caster).ok();
                        npc.blast_steps.push(exa_ai::combat::BlastStep {
                            chest: caster_body.map(body_chest).unwrap_or(Vec3::ZERO),
                            aim: live.blast.aim,
                            released: *released,
                            at: *at,
                        });
                    }
                    *at
                }
                crate::effects::danger::Hazard::Wave { at, .. } => *at,
            };
            npc.force_sights.push(exa_ai::contacts::Sighting {
                contact: record,
                character: false,
                position_cm: at,
                room: -1,
            });
        }
    }
}

/// The chest of a body, in centimetres: the chest bone, else its first part.
fn body_chest(body: &Ragdoll) -> Vec3 {
    bone_part(body, CHEST)
        .and_then(|part| part_point(body, part, Vec3::ZERO))
        .or_else(|| {
            body.parts
                .first()
                .map(|part| Vec3::from(part.transform().translation) * CM)
        })
        .unwrap_or(Vec3::ZERO)
}

/// A character's energy after the knocks of the parts a barricade struck in
/// front this tick: each lowers it by its loss while it is above 0.25.
pub fn energy_after_knocks(energy: f32, losses: &[f32]) -> f32 {
    losses
        .iter()
        .fold(energy, |e, &loss| if 0.25 < e { e - loss } else { e })
}

#[allow(clippy::too_many_arguments)]
fn tick_barricades(
    mut commands: Commands,
    mut walls: Query<(Entity, &mut LiveBarricade, Option<&NavStamped>)>,
    mut bodies: Query<(Entity, &mut Ragdoll, Option<&mut SwingState>)>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<EffectRoll>,
    mut nav: Option<ResMut<NavGrid>>,
    mut minds: Query<&mut Npc>,
) {
    let now = ticks.map_or(0, |t| t.millis()) as u32;
    for (wall_entity, mut live, stamped) in &mut walls {
        let live = &mut *live;
        if let Some(caster) = live.caster
            && let Ok(body) = bodies.get(caster)
            && let Some(at) = body.1.parts.first().map(|p| p.transform().translation * CM)
            && power_ends(0, live.wall.centre, at.into())
        {
            live.wall.end();
        }
        if live.wall.tick(now) {
            debug!("barricade {wall_entity}: gone");
        }
        // The standing wall is what the AI's sight sees: it carries its
        // hazard while it rises or stands and drops it when it falls.
        if live.wall.state == crate::thaum::barricade::state::GONE {
            commands
                .entity(wall_entity)
                .remove::<crate::effects::danger::Hazard>();
        } else if live.wall.entity {
            commands
                .entity(wall_entity)
                .insert(crate::effects::danger::Hazard::Barricade {
                    wall: crate::effects::danger::Wall {
                        centre: live.wall.centre,
                        along: live.wall.frame.x,
                        half_width: live.wall.half_width,
                    },
                    rising: live.wall.state == crate::thaum::barricade::state::RISING,
                    // The entity's strong byte is not modelled (a guess: no
                    // stronger fear).
                    strong: false,
                });
        }
        let mut holders = Vec::new();
        // Characters whose front limb played the impact sound this tick.
        let mut noticed = Vec::new();
        for (who, mut body, mut swing) in &mut bodies {
            let character = body.drive.as_ref().map(|d| {
                let chest = bone_part(&body, CHEST)
                    .and_then(|k| part_point(&body, k, Vec3::ZERO))
                    .unwrap_or(Vec3::ZERO);
                let step = step_length(d.stats.scale, d.balance().extension());
                (chest, d.stats.scale, step)
            });
            let mut touched = false;
            // Each part struck in front knocks the character back and
            // lowers its energy again.
            let mut knocks: Vec<(Vec3, f32)> = Vec::new();
            for k in 0..body.parts.len() {
                let part = &body.parts[k];
                let world = part.transform();
                let spheres: Vec<(Vec3, f32)> = part
                    .spheres
                    .iter()
                    .map(|&(c, r)| (world.transform_point3(c) * CM, r * CM))
                    .collect();
                let reach = spheres
                    .iter()
                    .map(|&(c, r)| c.distance(Vec3::from(world.translation) * CM) + r)
                    .fold(0.0, f32::max);
                let tested = WallBody {
                    centre: Vec3::from(world.translation) * CM,
                    radius: reach,
                    mobility: part.inverse_mass,
                    step: character.map_or(80.0, |c| c.2),
                    spheres,
                    kind: if part.state.has(exa_physics::part::PartState::STRIKE_CHAIN) {
                        WallBodyKind::Weapon
                    } else {
                        match character {
                            Some((chest, scale, _)) => WallBodyKind::Limb { chest, scale },
                            None => WallBodyKind::Other,
                        }
                    },
                };
                let shove = live.wall.shove(&tested, now, &mut roll.0);
                // A held weapon's strike-chain bit: the wall breaks its
                // holder's attack. The swing lives on the holder.
                if shove.interrupt {
                    if let Some(holder) = body.held_by {
                        holders.push(holder);
                    } else if let Some(state) = swing.as_mut() {
                        state.force_interrupt(u64::from(now));
                    }
                }
                touched |= shove.touched;
                if shove.alarm {
                    noticed.push(who);
                }
                for (sphere, push) in &shove.pushes {
                    body.push_sphere(k, *sphere, *push / CM);
                }
                if let Some(step) = shove.knockback {
                    knocks.push((step, shove.energy_loss));
                }
            }
            if touched {
                body.touch(WAKE_TICKS);
            }
            if let Some(&(step, _)) = knocks.last() {
                // The original's knockback sets the step's target and does
                // not wake the body, so no blow is queued; every part's
                // knockback aims at the same target.
                body.begin_knockback(step / CM);
                if let Some(drive) = body.drive.as_mut() {
                    drive.balance_mut().set_counter(100);
                    let losses: Vec<f32> = knocks.iter().map(|k| k.1).collect();
                    drive.energy = energy_after_knocks(drive.energy, &losses);
                }
            }
        }
        for holder in holders {
            if let Ok((_, _, Some(mut swing))) = bodies.get_mut(holder) {
                swing.force_interrupt(u64::from(now));
            }
        }
        // The sound's notice: the hitter's group alert, and concern toward
        // this wall when the sight pass has already made it a contact.
        for who in noticed {
            let Ok(mut npc) = minds.get_mut(who) else {
                continue;
            };
            let aggression = npc.personality.map_or(0.0, |p| p.restraint);
            (npc.alert_count, npc.alert_until_ms) = raise_alert(npc.alert_count, now);
            let id = wall_entity.to_bits();
            if let Some(record) = npc.contacts.iter_mut().find(|c| c.id == id) {
                raise_wall_concern(record, aggression);
            }
        }
        if live.wall.state == super::barricade::state::GONE {
            // The finished fall takes the stamp back off the grid.
            if let (Some(NavStamped(patch)), Some(grid)) = (stamped, nav.as_mut()) {
                unstamp_line(grid, patch);
            }
            commands.entity(wall_entity).despawn();
        } else if stamped.is_none()
            && let Some(grid) = nav.as_mut()
        {
            // The original stamps the wall's line as it enters the world; the
            // running game does so on the first tick a grid is there.
            let (width, depth) = grid.size();
            let frame = GridFrame {
                width: width as u32,
                height: depth as u32,
                scale: 1.0 / 30.0,
            };
            let (a, b) = live.wall.ends();
            let line = nav_stamp(&frame, a, b);
            let size = (line.size.0.max(0) as u32, line.size.1.max(0) as u32);
            let patch = stamp_line(grid, line.centre, size, &line.points);
            live.wall.stamp = Some(line);
            commands.entity(wall_entity).insert(NavStamped(patch));
        }
    }
}

fn tick_wisps(
    mut commands: Commands,
    mut sets: Query<(Entity, &mut LiveWisps)>,
    bodies: Query<&Ragdoll>,
    mut roll: ResMut<EffectRoll>,
) {
    for (entity, mut live) in &mut sets {
        let live = &mut *live;
        let Ok(body) = bodies.get(live.body) else {
            commands.entity(entity).despawn();
            continue;
        };
        let parts = &live.parts;
        let world = |s: &Strand| {
            s.body
                .and_then(|slot| parts.get(slot))
                .and_then(|&k| part_point(body, k, s.local))
                .unwrap_or(s.local)
        };
        live.wisps.tick(world, &mut roll.0);
    }
}

#[allow(clippy::too_many_arguments)]
fn spawn_debug_effects(
    mut commands: Commands,
    debug: Res<ThaumDebug>,
    players: Query<(Entity, &Ragdoll, &Locomotor), With<PlayerBody>>,
    physics: Option<Res<exa_physics::collision_share::FramePhysics>>,
    statics: Option<Res<exa_physics::StaticColliders>>,
    ticks: Option<Res<BodyTicks>>,
    mut draws: ResMut<FrameEffectDraws>,
    mut roll: ResMut<FrameEffectRoll>,
    mut done: Local<ThaumDebug>,
) {
    let Some((player, body, facing)) = players.iter().next() else {
        return;
    };
    if body.parts.is_empty() {
        return;
    }
    if debug.arcs && !done.arcs {
        done.arcs = true;
        let mut arcs = ArcEmitter::new(&hand_arcs(), &mut draws.0.0);
        arcs.anchor =
            bone_part(body, RIGHT_HAND).and_then(|k| part_point(body, k, HAND_ARCS_OFFSET));
        commands.spawn((
            Name::new("_DebugArcs"),
            LiveArcs {
                arcs,
                hand: Some((player, RIGHT_HAND)),
                weapon: None,
                caster: Some(player),
                keep: true,
            },
        ));
        info!("debug arcs on the player's right hand");
    }
    if debug.barricade && !done.barricade {
        done.barricade = true;
        let root = body.parts[0].transform();
        let at = Vec3::from(root.translation) * CM;
        // The model faces +Z at yaw 0.
        let forward = normalize(Vec3::new(facing.yaw.sin(), 0.0, facing.yaw.cos()));
        let feet = at + forward * 200.0;
        let hit = GroundHit {
            point: Vec3::new(feet.x, feet.y - 80.0, feet.z),
            direction: Vec3::Y,
            exact: true,
        };
        // The original's floor cast: static meshes of floor nodes only.
        let floor = |from: Vec3, reach: f32| {
            let depth = physics
                .as_deref()
                .zip(statics.as_deref())
                .and_then(|(p, s)| p.static_floor_below(s, from / CM, reach / CM));
            let depth = depth.map(|d| d * CM);
            info!(
                "barricade floor cast: from {:.0} cm, {}",
                from.y,
                depth.map_or("no floor node within 210 cm".to_string(), |d| format!(
                    "a floor node {d:.1} cm down"
                ))
            );
            depth
        };
        let place = place_barricade(hit, at, floor, roll.0.0.unit());
        let frame = WallFrame::facing(place.centre, place.facing);
        let now = ticks.as_deref().map_or(0, |t| t.millis()) as u32;
        let trim = |start: Vec3, dir: Vec3, len: f32| {
            physics
                .as_deref()
                .and_then(|p| p.fixed_hit(start / CM, dir, len / CM))
                .map(|d| d * CM)
        };
        let wall = Barricade::raise(frame, 0.25, now, trim, None, &mut draws.0.0);
        commands.spawn((
            Name::new("_DebugBarricade"),
            LiveBarricade {
                wall,
                caster: Some(player),
            },
        ));
        info!("debug barricade at {:?}", place.centre);
    }
    if debug.wisps && !done.wisps {
        done.wisps = true;
        let bones: Vec<usize> = body.parts.iter().map(|p| p.bone).collect();
        let mut parts = Vec::new();
        let mut strands = Vec::new();
        for (bone, anchor) in body_wisps(&bones) {
            if let Some(k) = bone_part(body, bone) {
                strands.push(Strand::new(anchor, Some(parts.len())));
                parts.push(k);
            }
        }
        let world = |s: &Strand| {
            s.body
                .and_then(|slot| parts.get(slot))
                .and_then(|&k| part_point(body, k, s.local))
                .unwrap_or(s.local)
        };
        let mut wisps = Wisps::new(strands, 12.0, world, || draws.0.0.unit() as f32);
        wisps.place(12, world, &mut roll.0.0);
        commands.spawn((
            Name::new("_DebugWisps"),
            LiveWisps {
                wisps,
                body: player,
                parts,
            },
        ));
        info!("debug wisps on the player's body");
    }
}

/// The glow tint each effect draws with.
const WALL_TINT: Vec4 = Vec4::new(0.45, 0.33, 0.95, 0.25);

fn mesh_of(positions: Vec<[f32; 3]>, colours: Vec<[f32; 4]>, indices: Vec<u32>) -> Mesh {
    let uvs = vec![[0.0, 0.0]; positions.len()];
    mesh_of_uv(positions, uvs, colours, indices)
}

/// A mesh with its texture coordinates (the arcs' ribbon scrolls them).
pub(super) fn mesh_of_uv(
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colours: Vec<[f32; 4]>,
    indices: Vec<u32>,
) -> Mesh {
    let normals = vec![[0.0, 1.0, 0.0]; positions.len()];
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
    mesh.insert_indices(Indices::U32(indices));
    mesh
}

/// A force ribbon (cm, colour bytes) as a Bevy mesh (m, linear colours).
pub(super) fn mesh_of_ribbon(ribbon: &crate::effects::ribbon::RibbonMesh) -> Mesh {
    let positions = ribbon
        .positions
        .iter()
        .map(|p| (*p / CM).to_array())
        .collect();
    let uvs = ribbon.uvs.clone();
    let colours = ribbon
        .colours
        .iter()
        .map(|c| {
            [
                f32::from(c[0]) / 255.0,
                f32::from(c[1]) / 255.0,
                f32::from(c[2]) / 255.0,
                f32::from(c[3]) / 255.0,
            ]
        })
        .collect();
    let indices = ribbon.indices.iter().map(|&i| u32::from(i)).collect();
    let mut mesh = mesh_of_uv(positions, uvs, colours, indices);
    // The second coordinate set (`vTx2`): the effect's texture row.
    if ribbon.uvs2.len() == ribbon.positions.len() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_1, ribbon.uvs2.clone());
    }
    mesh
}

#[allow(clippy::too_many_arguments)]
pub(super) fn show(
    commands: &mut Commands,
    entity: Entity,
    drawn: Option<&EffectMesh>,
    mesh: Mesh,
    tint: Vec4,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<GlowMaterial>,
) {
    match drawn {
        Some(EffectMesh(handle)) => {
            if let Some(mut m) = meshes.get_mut(handle) {
                *m = mesh;
            }
        }
        None => {
            let handle = meshes.add(mesh);
            commands.entity(entity).insert((
                EffectMesh(handle.clone()),
                Mesh3d(handle),
                MeshMaterial3d(materials.add(GlowMaterial { tint })),
                Transform::IDENTITY,
                NoFrustumCulling,
                bevy::light::NotShadowCaster,
                bevy::light::NotShadowReceiver,
            ));
        }
    }
}

fn camera_axis(cameras: &Query<&GlobalTransform, exa_render::supersample::SceneCamera>) -> Vec3 {
    cameras
        .iter()
        .next()
        .map_or(Vec3::NEG_Z, |c| c.forward().as_vec3())
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn draw_arcs(
    mut commands: Commands,
    mut arcs: Query<(
        Entity,
        &mut LiveArcs,
        Option<&EffectMesh>,
        Option<&MeshMaterial3d<ArcMaterial>>,
        Option<&super::effect_light::EffectLight>,
    )>,
    mut shapes: Query<(
        &mut exa_render::gi::GiShape,
        &mut exa_render::gi::ShapeVolume,
    )>,
    held_voxels: Query<&HeldVoxel>,
    mut voxels: Query<(&mut VoxelLight, &mut Transform, &mut GlobalTransform), Without<Camera3d>>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    mut draws: ResMut<FrameEffectDraws>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: Option<ResMut<Assets<ArcMaterial>>>,
    mut look: ArcLook,
    mut audio: ShockAudio,
    mut hums: Local<bevy::platform::collections::HashMap<Entity, Option<u32>>>,
) {
    let eye = camera_axis(&cameras);
    let alpha = time.overstep_fraction();
    for (entity, mut live, drawn, material, light) in &mut arcs {
        let voxel = held_voxels.get(entity).ok();
        let frame = live.arcs.frame(alpha, eye, 0.0, |p| p, &mut draws.0.0);
        let at = live.arcs.anchor.map(|p| p / CM).unwrap_or(Vec3::ZERO);
        drive_arc_hum(
            &mut audio,
            hums.entry(entity).or_default(),
            live.arcs.sound,
            at,
            frame.gain,
            frame.pitch,
            frame.removed,
        );
        drive_arc_light(
            &mut commands,
            entity,
            light,
            &mut shapes,
            &live.arcs,
            frame.light,
            frame.removed,
        );
        drive_arc_voxel(
            &mut commands,
            entity,
            voxel,
            &mut voxels,
            &live.arcs,
            frame.light,
            frame.removed,
        );
        if frame.removed && !live.keep {
            commands.entity(entity).despawn();
            continue;
        }
        let r = &frame.ribbon;
        let positions = r.positions.iter().map(|p| (*p / CM).to_array()).collect();
        let uvs = r.uvs.to_vec();
        let [red, green, blue] = ARC_COLOUR.map(|c| f32::from(c) / 255.0);
        let colours = r
            .alphas
            .iter()
            .map(|&a| [red, green, blue, f32::from(a) / 255.0])
            .collect();
        let indices = r.drawn().iter().map(|&i| u32::from(i)).collect();
        let mesh = mesh_of_uv(positions, uvs, colours, indices);
        // `_ElecMat01`'s program reads the emitter's shown scroll as the
        // second texture offset.
        let offset = Vec4::new(0.0, frame.scroll, 0.0, 0.0);
        let Some(materials) = materials.as_deref_mut() else {
            continue;
        };
        match drawn {
            Some(EffectMesh(handle)) => {
                if let Some(mut m) = meshes.get_mut(handle) {
                    *m = mesh;
                }
                if let Some(material) = material
                    && let Some(mut m) = materials.get_mut(&material.0)
                {
                    m.tex_offset = offset;
                }
            }
            None => {
                let handle = meshes.add(mesh);
                let mut material = ArcMaterial::new(look.texture());
                material.tex_offset = offset;
                commands.entity(entity).insert((
                    EffectMesh(handle.clone()),
                    Mesh3d(handle),
                    MeshMaterial3d(materials.add(material)),
                    Transform::IDENTITY,
                    NoFrustumCulling,
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ));
            }
        }
    }
}

/// `_ElecMat01`'s emissive layer, which the electricity unit's material
/// record names `BeamNoise01.rfi`: the texture pack's entry is its stem.
pub const ARC_TEXTURE: &str = "beamnoise01";

/// The arcs' picture, loaded on first use: [`ARC_TEXTURE`] with its values
/// read raw, or a mid-grey texel when the install does not have it.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct ArcLook<'w, 's> {
    loaded: Local<'s, Option<Handle<Image>>>,
    images: Option<ResMut<'w, Assets<Image>>>,
    game: Option<Res<'w, exa_assets::Game>>,
}

impl ArcLook<'_, '_> {
    fn texture(&mut self) -> Handle<Image> {
        if let Some(handle) = self.loaded.as_ref() {
            return handle.clone();
        }
        let Some(images) = self.images.as_deref_mut() else {
            return Handle::default();
        };
        let handle = match self.game.as_deref().map(|game| {
            game.load_texture(ARC_TEXTURE, exa_assets::texture::ColorSpace::Linear, None)
        }) {
            Some(Ok(image)) => images.add(image),
            Some(Err(e)) => {
                warn!("arcs: {ARC_TEXTURE}: {e}; a grey texel stands in");
                images.add(super::vanish_live::grey_texel())
            }
            None => images.add(super::vanish_live::grey_texel()),
        };
        *self.loaded = Some(handle.clone());
        handle
    }
}

fn draw_barricades(
    mut commands: Commands,
    mut walls: Query<(Entity, &mut LiveBarricade, Option<&EffectMesh>)>,
    time: Res<Time<Fixed>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<GlowMaterial>>,
    mut slab: Local<Option<super::barricade::WallMesh>>,
) {
    let alpha = time.overstep_fraction();
    let slab = slab.get_or_insert_with(barricade_mesh);
    for (entity, mut live, drawn) in &mut walls {
        let Some(out) = live.wall.frame(alpha) else {
            continue;
        };
        let at = if out.mode == 1 {
            out.at
        } else {
            live.wall.centre
        };
        let frame = WallFrame {
            at,
            ..live.wall.frame
        };
        let (local, _) = slide_mesh(slab, out.slide);
        let height = (out.half_height / super::barricade::HALF_HEIGHT).max(0.0);
        let positions = local
            .iter()
            .map(|p| (frame.world(Vec3::new(p.x, p.y * height, p.z)) / CM).to_array())
            .collect();
        let colours = vec![[1.0; 4]; local.len()];
        let indices = slab
            .triangles
            .iter()
            .flatten()
            .map(|&i| u32::from(i))
            .collect();
        let mesh = mesh_of(positions, colours, indices);
        show(
            &mut commands,
            entity,
            drawn,
            mesh,
            WALL_TINT,
            &mut meshes,
            &mut materials,
        );
    }
}

/// The displacement tendrils' picture (`_TDispMat01`'s emissive layer),
/// loaded once, and the material every strip shares.
#[derive(Resource, Clone)]
pub(crate) struct TendrilLook(Handle<exa_render::force_effects::DispTendrilMaterial>);

#[allow(clippy::too_many_arguments)]
fn draw_wisps(
    mut commands: Commands,
    mut sets: Query<(Entity, &mut LiveWisps, Option<&EffectMesh>)>,
    cameras: Query<&GlobalTransform, exa_render::supersample::SceneCamera>,
    time: Res<Time<Fixed>>,
    mut meshes: ResMut<Assets<Mesh>>,
    materials: Option<ResMut<Assets<exa_render::force_effects::DispTendrilMaterial>>>,
    mut images: ResMut<Assets<Image>>,
    game: Option<Res<exa_assets::Game>>,
    look: Option<Res<TendrilLook>>,
) {
    let Some(mut materials) = materials else {
        return;
    };
    if sets.is_empty() {
        return;
    }
    let material = match look {
        Some(look) => look.0.clone(),
        None => {
            let texture = super::vanish_live::load_texture(game.as_deref(), &mut images);
            let handle =
                materials.add(exa_render::force_effects::DispTendrilMaterial::new(texture));
            commands.insert_resource(TendrilLook(handle.clone()));
            handle
        }
    };
    let eye = camera_axis(&cameras);
    let alpha = time.overstep_fraction();
    for (entity, mut live, drawn) in &mut sets {
        let r = live.wisps.ribbon(alpha, eye);
        if r.removed {
            commands.entity(entity).despawn();
            continue;
        }
        // The strip as the original uploads it: positions, the first
        // coordinate set (the side and the texture distance along), and the
        // colour bytes (`vCol · 1/255`). The second set (the light share and
        // the strip's running distance) is uploaded too but the surface
        // program `_TDispMat01` builds never reads it.
        let positions = r.positions.iter().map(|p| (*p / CM).to_array()).collect();
        let colours = r
            .colours
            .iter()
            .map(|c| c.map(|b| f32::from(b) / 255.0))
            .collect();
        let mesh = mesh_of_uv(positions, r.uvs.clone(), colours, r.indices);
        match drawn {
            Some(EffectMesh(handle)) => {
                if let Some(mut m) = meshes.get_mut(handle) {
                    *m = mesh;
                }
            }
            None => {
                let handle = meshes.add(mesh);
                commands.entity(entity).insert((
                    EffectMesh(handle.clone()),
                    Mesh3d(handle),
                    MeshMaterial3d(material.clone()),
                    Transform::IDENTITY,
                    NoFrustumCulling,
                    bevy::light::NotShadowCaster,
                    bevy::light::NotShadowReceiver,
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The plugin's systems run with a barricade standing on a navigation
    /// grid (their parameters resolve, the stamp goes on and comes off).
    #[test]
    fn the_effects_run_with_a_barricade_on_a_grid() {
        use bevy::asset::AssetPlugin;
        use exa_world::nav::{Layer, NavCell};
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<GlowMaterial>()
            .init_asset::<Image>()
            .init_asset::<exa_render::force_effects::ForceRibbonMaterial>()
            .init_asset::<exa_render::force_effects::ForceShellMaterial>()
            .init_asset::<exa_render::force_effects::DispShellMaterial>()
            .init_asset::<exa_render::force_effects::TeleBubbleMaterial>()
            .init_asset::<exa_render::force_effects::ForceDomeMaterial>()
            .add_plugins(ThaumEffectsPlugin);
        let mut grid = NavGrid::empty(40, 40, 0.0);
        for z in 0..40 {
            for x in 0..40 {
                let floor = NavCell {
                    floor: 10,
                    ..NavCell::default()
                };
                grid.set_cell(x, z, Layer::Lower, floor);
            }
        }
        app.insert_resource(grid);
        let frame = WallFrame::facing(Vec3::new(0.0, 50.0, 0.0), Vec3::Z);
        let mut draws = Mt19937::new(5489);
        let wall = Barricade::raise(frame, 0.25, 0, |_, _, _| None, None, &mut draws);
        let wall = app
            .world_mut()
            .spawn(LiveBarricade { wall, caster: None })
            .id();
        app.world_mut().run_schedule(FixedUpdate);
        app.update();
        assert!(app.world().get::<NavStamped>(wall).is_some());
        let stamped: u32 = {
            let grid = app.world().resource::<NavGrid>();
            (0..40)
                .flat_map(|z| (0..40).map(move |x| (x, z)))
                .map(|(x, z)| u32::from(grid.cell(x, z, Layer::Lower).penalty))
                .sum()
        };
        assert!(stamped > 0, "the line is on the grid");
        app.world_mut()
            .get_mut::<LiveBarricade>(wall)
            .unwrap()
            .wall
            .state = super::super::barricade::state::GONE;
        app.world_mut().run_schedule(FixedUpdate);
        let grid = app.world().resource::<NavGrid>();
        assert!(
            (0..40).all(|z| (0..40).all(|x| grid.cell(x, z, Layer::Lower).penalty == 0)),
            "the fall takes it back"
        );
    }

    /// A looping hum takes the frame's gain, and its pitch only when the
    /// frame has one. A stop asks the voice to end.
    #[test]
    fn the_arc_hum_follows_the_frame() {
        let mut voice = exa_audio::Voice {
            pitch: 1.0,
            ..Default::default()
        };
        apply_arc_hum(&mut voice, 0.25, None, Vec3::X);
        assert_eq!(voice.gain, 0.25);
        assert_eq!(voice.pitch, 1.0);
        assert_eq!(voice.position, Vec3::X);
        apply_arc_hum(&mut voice, 0.1, Some(0.7), Vec3::Y);
        assert_eq!(voice.pitch, 0.7);
        assert_eq!(voice.position, Vec3::Y);
    }

    /// Arcs that gather light points register a tendril light and drop it
    /// when they end. A hand arc, which carries a voxel light, does not.
    #[test]
    fn a_shock_weapons_arcs_place_their_light() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<ArcMaterial>()
            .insert_resource(FrameEffectDraws::default())
            .add_systems(Update, draw_arcs);
        let mut arcs = ArcEmitter::allocate(1, 4, 2.0);
        arcs.light_kind = ArcLight::Points;
        arcs.light = 0.02;
        let arc = app
            .world_mut()
            .spawn(LiveArcs {
                arcs,
                hand: None,
                weapon: None,
                caster: None,
                keep: true,
            })
            .id();
        let hand = ArcEmitter::new(&hand_arcs(), &mut Mt19937::new(1));
        let bare = app
            .world_mut()
            .spawn(LiveArcs {
                arcs: hand,
                hand: None,
                weapon: None,
                caster: None,
                keep: true,
            })
            .id();
        app.update();
        assert!(
            app.world()
                .get::<crate::thaum::effect_light::EffectLight>(bare)
                .is_none(),
            "a voxel light is not this tendril light"
        );
        let held = app
            .world()
            .get::<crate::thaum::effect_light::EffectLight>(arc)
            .expect("a light")
            .0;
        let placed = app.world().get::<exa_render::gi::GiShape>(held).unwrap();
        assert_eq!(placed.program, exa_render::gi::GiShapeProgram::Multipoint);
        assert_eq!(placed.atten, 16_000.0);
        let got = placed.points.clone();
        let live = app.world().get::<LiveArcs>(arc).unwrap();
        let points: Vec<_> = live.arcs.lights.iter().map(|p| (p.at, p.weight)).collect();
        let strength = live.arcs.tendrils[0].life * live.arcs.light;
        let (expect, _) = crate::thaum::effect_light::arc_shape(
            &points,
            strength,
            crate::thaum::effect_light::GI_LIGHT_SCALE,
        )
        .expect("the same points");
        assert_eq!(got.as_ref(), expect.points.as_ref());
        {
            let mut live = app.world_mut().get_mut::<LiveArcs>(arc).unwrap();
            live.arcs.ending = true;
            live.arcs.total = 0;
        }
        app.update();
        assert!(
            app.world()
                .get::<crate::thaum::effect_light::EffectLight>(arc)
                .is_none()
        );
        assert!(app.world().get::<exa_render::gi::GiShape>(held).is_none());
    }

    /// Live arcs draw with `_ElecMat01`'s program: their own arc material,
    /// whose texture offset is (0, the emitter's shown scroll).
    #[test]
    fn live_arcs_draw_with_the_electricity_program() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<ArcMaterial>()
            .insert_resource(FrameEffectDraws::default())
            .add_systems(Update, draw_arcs);
        let mut arcs = ArcEmitter::new(&hand_arcs(), &mut Mt19937::new(1));
        arcs.scroll = [0.25, 0.25];
        let arc = app
            .world_mut()
            .spawn(LiveArcs {
                arcs,
                hand: None,
                weapon: None,
                caster: None,
                keep: true,
            })
            .id();
        app.update();
        let handle = app
            .world()
            .get::<MeshMaterial3d<ArcMaterial>>(arc)
            .expect("an arc material")
            .0
            .clone();
        let materials = app.world().resource::<Assets<ArcMaterial>>();
        let material = materials.get(&handle).unwrap();
        assert_eq!(material.tex_offset, Vec4::new(0.0, 0.25, 0.0, 0.0));
        assert_eq!(material.ele_colour.truncate(), Vec3::new(0.65, 0.4, 1.0));
    }

    /// A hand arc's voxel light sits at the arcs' mean. Its intensity is the
    /// frame's, and it leaves when the arcs end.
    #[test]
    fn a_hand_arcs_voxel_light_follows_the_mean() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<ArcMaterial>()
            .insert_resource(FrameEffectDraws::default())
            .add_systems(Update, draw_arcs);
        let mut arcs = ArcEmitter::new(&hand_arcs(), &mut Mt19937::new(1));
        arcs.total = arcs.segments * arcs.tendrils.len() as i32;
        arcs.mean = Vec3::new(100.0, 200.0, -50.0);
        let arc = app
            .world_mut()
            .spawn(LiveArcs {
                arcs,
                hand: None,
                weapon: None,
                caster: None,
                keep: true,
            })
            .id();
        app.update();
        let held = app.world().get::<HeldVoxel>(arc).expect("a voxel light").0;
        let (color, base, radius, active, flags, intensity) = {
            let light = app.world().get::<VoxelLight>(held).unwrap();
            (
                light.color,
                light.base_intensity,
                light.radius,
                light.active_radius,
                light.flags,
                light.intensity,
            )
        };
        assert_eq!(color, [0.6, 0.6, 1.0]);
        assert_eq!(base, 16.0);
        assert_eq!(radius, 1.8);
        assert_eq!(active, 1.8);
        assert_eq!(flags & VOXEL_FRAME_FLAGS, VOXEL_FRAME_FLAGS);
        let live = app.world().get::<LiveArcs>(arc).unwrap();
        let fill = (live.arcs.total as f32
            / (live.arcs.segments as f32 * live.arcs.tendrils.len() as f32))
            * 2.0;
        let expect = live.arcs.light * fill * live.arcs.light_flicker * 1.5;
        assert!((intensity - expect).abs() < 1e-4, "{intensity}");
        assert_eq!(
            app.world().get::<Transform>(held).unwrap().translation,
            Vec3::new(1.0, 2.0, -0.5)
        );
        {
            let mut live = app.world_mut().get_mut::<LiveArcs>(arc).unwrap();
            live.arcs.ending = true;
            live.arcs.total = 0;
        }
        app.update();
        assert!(app.world().get::<HeldVoxel>(arc).is_none());
        assert!(app.world().get::<VoxelLight>(held).is_none());
    }

    /// A held weapon whose strike chain meets the wall breaks the holder's
    /// attack. The same weapon, not striking, does not.
    #[test]
    fn a_striking_weapon_breaks_on_the_wall() {
        use bevy::math::Affine3A;
        use exa_physics::body::PartSetup;
        let frame = WallFrame::facing(Vec3::new(0.0, 100.0, 0.0), Vec3::X);
        let mut draws = Mt19937::new(1);
        let mut wall = Barricade::raise(frame, 0.25, 1000, |_, _, _| None, None, &mut draws);
        wall.tick(1000 + 1700);
        let mut app = App::new();
        app.insert_resource(BodyTicks(5_000))
            .init_resource::<EffectRoll>()
            .add_systems(Update, tick_barricades);
        app.world_mut().spawn(LiveBarricade { wall, caster: None });
        let holder = app
            .world_mut()
            .spawn((
                Ragdoll::new(&[]),
                SwingState {
                    strength: 0.0,
                    ..SwingState::default()
                },
            ))
            .id();
        let at = Vec3::new(0.05, 1.0, 0.0);
        let mut weapon = Ragdoll::new(&[PartSetup {
            bone: 0,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(at),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: Some(vec![(Vec3::ZERO, 0.1)]),
        }]);
        weapon.held_by = Some(holder);
        let weapon_id = app.world_mut().spawn(weapon).id();
        app.update();
        let swing = app.world().get::<SwingState>(holder).unwrap();
        assert_eq!(swing.strength, 0.0, "not striking, so no break");
        {
            let mut body = app.world_mut().get_mut::<Ragdoll>(weapon_id).unwrap();
            body.parts[0].state = body.parts[0]
                .state
                .with(exa_physics::part::PartState::STRIKE_CHAIN);
        }
        app.update();
        let swing = app.world().get::<SwingState>(holder).unwrap();
        assert_eq!(swing.strength, 1.0);
        assert_ne!(
            swing.flags & exa_combat::swing::flags::STALLING,
            0,
            "the holder's attack stalls"
        );
    }

    /// A front limb that plays the impact sound raises the hitter's group
    /// alert and adds concern toward the wall. Another contact is left alone,
    /// and the same clock does not notice twice.
    #[test]
    fn a_front_limb_tells_its_ai_about_the_wall() {
        use bevy::math::Affine3A;
        use exa_ai::combatant::CONTACT_CONCERN_FLAGS;
        use exa_physics::body::PartSetup;
        let frame = WallFrame::facing(Vec3::new(-15.0, 100.0, 0.0), Vec3::X);
        let mut draws = Mt19937::new(1);
        let mut wall = Barricade::raise(frame, 0.25, 1000, |_, _, _| None, None, &mut draws);
        wall.tick(1000 + 1700);
        let mut app = App::new();
        app.insert_resource(BodyTicks(5_000))
            .init_resource::<EffectRoll>()
            .add_systems(Update, tick_barricades);
        let wall_id = app
            .world_mut()
            .spawn(LiveBarricade { wall, caster: None })
            .id();
        let other = app.world_mut().spawn_empty().id();
        let mut roll = 1u32;
        let mut npc = Npc::new([0.0; 4]);
        npc.alert_count = 4;
        npc.alert_until_ms = 1;
        npc.personality = Some(exa_ai::personality::personality(
            0.0,
            [0.0, 0.0, -25.0, 0.0],
            0.0,
            false,
            &mut roll,
        ));
        let aggression = npc.personality.unwrap().restraint;
        npc.contacts.push(ContactRecord {
            id: wall_id.to_bits(),
            concern: 1.0,
            ..ContactRecord::default()
        });
        npc.contacts.push(ContactRecord {
            id: other.to_bits(),
            concern: 2.0,
            ..ContactRecord::default()
        });
        let at = Vec3::new(0.0, 1.0, 0.0);
        let body = Ragdoll::new(&[PartSetup {
            bone: 0,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(at),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: Some(vec![(Vec3::ZERO, 0.1)]),
        }])
        .with_drive();
        let who = app.world_mut().spawn((body, npc)).id();
        app.update();
        let now = BodyTicks(5_000).millis() as u32;
        let npc = app.world().get::<Npc>(who).unwrap();
        assert_eq!((npc.alert_count, npc.alert_until_ms), raise_alert(4, now));
        let mut expect = Contact::new(Entity::PLACEHOLDER);
        expect.concern = 1.0;
        let raised = exa_ai::combatant::add_concern(&mut expect, aggression, 0.5, 1.0);
        assert_eq!(npc.contacts[0].concern, raised);
        assert_eq!(npc.contacts[0].flags, CONTACT_CONCERN_FLAGS);
        assert_eq!(npc.contacts[1].concern, 2.0);
        assert_eq!(npc.contacts[1].flags, 0);
        app.update();
        let npc = app.world().get::<Npc>(who).unwrap();
        assert_eq!(npc.alert_count, 5, "the sound's gap has not passed");
    }

    /// A brain within sight of a barricade, a blast and a wave sees each of
    /// them. A wall 13 m off is not seen.
    #[test]
    fn a_brain_sees_the_force_effects_in_front_of_it() {
        use bevy::math::Affine3A;
        use exa_physics::body::PartSetup;
        let mut app = App::new();
        app.add_systems(Update, see_force_effects);
        let near = app
            .world_mut()
            .spawn((
                crate::effects::danger::Hazard::Barricade {
                    wall: crate::effects::danger::Wall {
                        centre: Vec3::new(0.0, 100.0, 200.0),
                        along: Vec3::X,
                        half_width: 150.0,
                    },
                    rising: true,
                    strong: false,
                },
                Name::new("_NearWall"),
            ))
            .id();
        app.world_mut()
            .spawn(crate::effects::danger::Hazard::Barricade {
                wall: crate::effects::danger::Wall {
                    centre: Vec3::new(0.0, 100.0, 1400.0),
                    along: Vec3::X,
                    half_width: 150.0,
                },
                rising: false,
                strong: false,
            });
        let blast = app
            .world_mut()
            .spawn(crate::effects::danger::Hazard::Blast {
                at: Vec3::new(0.0, 100.0, 300.0),
                released: false,
            })
            .id();
        let wave = app
            .world_mut()
            .spawn(crate::effects::danger::Hazard::Wave {
                at: Vec3::new(0.0, 100.0, 400.0),
                released: true,
            })
            .id();
        let body = Ragdoll::new(&[PartSetup {
            bone: 0,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: Affine3A::from_translation(Vec3::new(0.0, 1.0, 0.0)),
            centre: Vec3::ZERO,
            half: Vec3::splat(0.1),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
            spheres: None,
        }])
        .with_drive();
        let who = app.world_mut().spawn((body, Npc::new([0.0; 4]))).id();
        app.update();
        let npc = app.world().get::<Npc>(who).unwrap();
        assert_eq!(npc.force_sights.len(), 3);
        let sight = npc
            .force_sights
            .iter()
            .find(|sight| sight.contact.id == near.to_bits())
            .unwrap();
        assert!(sight.contact.object);
        assert!(!sight.character);
        assert_ne!(sight.contact.flags & exa_ai::combatant::CONTACT_NOTICED, 0);
        assert!(sight.contact.alarm > 0.0, "{}", sight.contact.alarm);
        assert_eq!(sight.position_cm, Vec3::new(0.0, 100.0, 200.0));
        let blast = npc
            .force_sights
            .iter()
            .find(|sight| sight.contact.id == blast.to_bits())
            .unwrap();
        assert!(blast.contact.alarm > 0.0);
        assert_eq!(blast.position_cm, Vec3::new(0.0, 100.0, 300.0));
        let wave = npc
            .force_sights
            .iter()
            .find(|sight| sight.contact.id == wave.to_bits())
            .unwrap();
        assert!(wave.contact.alarm > 0.0, "a released wave raises its fear");
        assert_eq!(wave.position_cm, Vec3::new(0.0, 100.0, 400.0));
    }

    /// A full relation to the caster takes the fear to nothing. A brain with
    /// no relation still fears the same wall.
    #[test]
    fn a_friend_does_not_fear_the_wall_they_raised() {
        use bevy::math::Affine3A;
        use exa_ai::roles::Relation;
        use exa_physics::body::PartSetup;
        let mut app = App::new();
        app.add_systems(Update, see_force_effects);
        let caster = app
            .world_mut()
            .spawn((CharacterId(1), Roles::default()))
            .id();
        let mut draws = Mt19937::new(1);
        let wall = Barricade::raise(
            WallFrame::facing(Vec3::new(0.0, 100.0, 200.0), Vec3::Z),
            0.25,
            0,
            |_, _, _| None,
            None,
            &mut draws,
        );
        app.world_mut().spawn((
            LiveBarricade {
                wall,
                caster: Some(caster),
            },
            crate::effects::danger::Hazard::Barricade {
                wall: crate::effects::danger::Wall {
                    centre: Vec3::new(0.0, 100.0, 200.0),
                    along: Vec3::X,
                    half_width: 150.0,
                },
                rising: true,
                strong: false,
            },
        ));
        let body = || {
            Ragdoll::new(&[PartSetup {
                bone: 0,
                entity: Entity::PLACEHOLDER,
                parent: None,
                world: Affine3A::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            }])
            .with_drive()
        };
        let mut relations = Relations::default();
        relations.global.insert(
            1,
            Relation {
                last: 100.0,
                ..Relation::default()
            },
        );
        let friend = app
            .world_mut()
            .spawn((body(), Npc::new([0.0; 4]), relations))
            .id();
        let stranger = app.world_mut().spawn((body(), Npc::new([0.0; 4]))).id();
        app.update();
        let friend = app.world().get::<Npc>(friend).unwrap();
        assert_eq!(friend.force_sights.len(), 1);
        assert_eq!(friend.force_sights[0].contact.alarm, 0.0);
        let stranger = app.world().get::<Npc>(stranger).unwrap();
        assert!(stranger.force_sights[0].contact.alarm > 0.0);
    }

    /// A frenzy wipes the relation, so a friend fears the wall again. Suspicion
    /// lowers it and the fear rises. A binding to the caster takes the fear away.
    #[test]
    fn a_mind_power_changes_the_fear_of_the_wall() {
        use bevy::math::Affine3A;
        use exa_ai::combatant::{Effect, MindPowers, POWER_BIND, POWER_FRENZY, POWER_SUSPICION};
        use exa_ai::roles::Relation;
        use exa_physics::body::PartSetup;
        let mut app = App::new();
        app.add_systems(Update, see_force_effects);
        let caster = app
            .world_mut()
            .spawn((CharacterId(1), Roles::default()))
            .id();
        let mut draws = Mt19937::new(1);
        let wall = Barricade::raise(
            WallFrame::facing(Vec3::new(0.0, 100.0, 200.0), Vec3::Z),
            0.25,
            0,
            |_, _, _| None,
            None,
            &mut draws,
        );
        app.world_mut().spawn((
            LiveBarricade {
                wall,
                caster: Some(caster),
            },
            crate::effects::danger::Hazard::Barricade {
                wall: crate::effects::danger::Wall {
                    centre: Vec3::new(0.0, 100.0, 200.0),
                    along: Vec3::X,
                    half_width: 150.0,
                },
                rising: true,
                strong: false,
            },
        ));
        let body = || {
            Ragdoll::new(&[PartSetup {
                bone: 0,
                entity: Entity::PLACEHOLDER,
                parent: None,
                world: Affine3A::from_translation(Vec3::new(0.0, 1.0, 0.0)),
                centre: Vec3::ZERO,
                half: Vec3::splat(0.1),
                origin: Vec3::ZERO,
                links: vec![],
                foot: None,
                pins: Vec::new(),
                spheres: None,
            }])
            .with_drive()
        };
        let mut relations = Relations::default();
        relations.global.insert(
            1,
            Relation {
                last: 100.0,
                ..Relation::default()
            },
        );
        let power = |kind, source| MindPowers {
            effects: vec![Effect {
                kind,
                source,
                target: None,
                state: 0,
            }],
        };
        let frenzied = app
            .world_mut()
            .spawn((
                body(),
                Npc::new([0.0; 4]),
                relations,
                power(POWER_FRENZY, None),
            ))
            .id();
        let mut cast = crate::mind::casts::MindBody::new(1);
        cast.powers.push(crate::mind::casts::MindPower::recorded(
            POWER_SUSPICION,
            None,
            None,
        ));
        let suspicious = app
            .world_mut()
            .spawn((body(), Npc::new([0.0; 4]), cast))
            .id();
        let bound = app
            .world_mut()
            .spawn((body(), Npc::new([0.0; 4]), power(POWER_BIND, Some(caster))))
            .id();
        let mut resisted_body = crate::mind::casts::MindBody::new(1);
        resisted_body.cast.techniques = crate::mind::casts::RESISTS_CONTROL;
        resisted_body
            .powers
            .push(crate::mind::casts::MindPower::recorded(
                POWER_BIND,
                Some(1),
                None,
            ));
        let resisted = app
            .world_mut()
            .spawn((body(), Npc::new([0.0; 4]), resisted_body))
            .id();
        let plain = app.world_mut().spawn((body(), Npc::new([0.0; 4]))).id();
        app.update();
        let alarm = |id| {
            app.world().get::<Npc>(id).unwrap().force_sights[0]
                .contact
                .alarm
        };
        assert!(alarm(frenzied) > 0.0, "a frenzy clears the friendship");
        assert_eq!(alarm(bound), 0.0, "a binding trusts the caster");
        let resisted = alarm(resisted);
        let plain = alarm(plain);
        let suspicious = alarm(suspicious);
        assert!(
            (suspicious - plain * 1.5).abs() < 1e-4,
            "suspicion {suspicious} against {plain}"
        );
        assert!(
            (resisted - plain * 0.5).abs() < 1e-4,
            "resistance {resisted} against {plain}"
        );
    }

    #[test]
    fn a_shock_comes_within_150_ms_of_its_time() {
        let mut roll = Roll(0);
        // The hit sound takes two draws, then the delay. From state 0 that
        // delay is 172 ms, so the next shock is due at 672.
        let taken = shock_reaction(None, 0, 0.25, 0.05, false, false, false, &mut roll).unwrap();
        assert_eq!(taken.next, 672);
        assert!((taken.yellow - 0.2).abs() < 1e-6);
        assert!((taken.pitch - 0.8).abs() < 1e-5);
        // 499 + 150 is short of 672 by more than the window.
        assert_eq!(
            shock_reaction(Some(672), 521, 0.2, 0.05, false, false, false, &mut roll),
            None
        );
        assert_eq!(roll, Roll(1_172_187_917), "a refused shock draws nothing");
        let taken =
            shock_reaction(Some(672), 522, 0.2, 0.05, false, false, false, &mut roll).unwrap();
        assert_eq!(taken.next, 1_054);
        assert!(taken.cry.is_some());
        assert!((taken.yellow - 0.15).abs() < 1e-6);
        // Collapsing or dead: the clock moves, the health stays.
        let kept = shock_reaction(None, 0, 0.1, 0.05, true, false, false, &mut roll).unwrap();
        assert_eq!(kept.yellow, 0.1);
        assert!(kept.cry.is_none());
        assert_eq!(
            shock_reaction(None, 0, 0.03, 0.05, false, false, false, &mut roll).map(|s| s.yellow),
            Some(0.0)
        );
        // A released race: the sound's two draws, then 500 ms on, no delay
        // draw, its health kept.
        let before = roll;
        let released =
            shock_reaction(None, 1_000, 0.2, 0.05, false, true, false, &mut roll).unwrap();
        assert_eq!(released.next, 1_500);
        assert_eq!(released.yellow, 0.2);
        assert_ne!(roll, before);
        // Race index 1 rings: the hit's two draws, a flat 500 ms, no burn.
        let mut roll = Roll(0);
        let rung = shock_reaction(None, 0, 0.25, 0.05, false, false, true, &mut roll).unwrap();
        assert_eq!(rung.next, 500);
        assert_eq!(rung.yellow, 0.25);
        assert!(rung.cry.is_none());
        assert_eq!(roll, Roll(0x0808_8406));
    }

    #[test]
    fn a_shocked_fighter_waits_for_its_next_shock() {
        let mut app = App::new();
        app.add_message::<ArcShocked>()
            .insert_resource(BodyTicks(0))
            .insert_resource(EffectRoll(Roll(0)))
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .add_systems(Update, shock_fighters);
        let fighter = app.world_mut().spawn(Fighter::default()).id();
        let shock = ArcShocked {
            body: fighter,
            direction: Vec3::Y,
            strength: 0.05,
        };
        let yellow = |app: &App| app.world().get::<Fighter>(fighter).unwrap().health.yellow;
        // Two arcs in one tick: the second comes too early.
        app.world_mut().write_message(shock);
        app.world_mut().write_message(shock);
        app.update();
        assert!((yellow(&app) - 0.20).abs() < 1e-6);
        assert_eq!(
            app.world().get::<ShockReady>(fighter),
            Some(&ShockReady(672))
        );
        // 43 ticks are 516 ms: 516 + 150 is short of 672.
        app.insert_resource(BodyTicks(43));
        app.world_mut().write_message(shock);
        app.update();
        assert!((yellow(&app) - 0.20).abs() < 1e-6);
        // 44 ticks are 528 ms: within 150 ms of 672.
        app.insert_resource(BodyTicks(44));
        app.world_mut().write_message(shock);
        app.update();
        assert!((yellow(&app) - 0.15).abs() < 1e-6);
        assert_eq!(
            app.world().get::<ShockReady>(fighter),
            Some(&ShockReady(1_060))
        );
        assert_eq!(
            app.world().get::<StateFlags>(fighter).map(|f| f.0),
            Some(StateFlags::SHOCK_STAGGER)
        );
    }

    /// A burn stops the walk: the request is no longer moving or free, and
    /// the locomotion amount is 0.
    #[test]
    fn a_shock_stops_the_walk() {
        use exa_physics::body::locomotion::{Locomotion, Locomotor};
        let mut app = App::new();
        app.add_message::<ArcShocked>()
            .insert_resource(BodyTicks(0))
            .insert_resource(EffectRoll(Roll(0)))
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .add_systems(Update, shock_fighters);
        let req = Locomotion {
            moving: true,
            free_walk: true,
            ..Default::default()
        };
        let motor = Locomotor {
            amount: 4.0,
            ..Default::default()
        };
        let fighter = app.world_mut().spawn((Fighter::default(), req, motor)).id();
        app.world_mut().write_message(ArcShocked {
            body: fighter,
            direction: Vec3::X,
            strength: 0.05,
        });
        app.update();
        let req = app.world().get::<Locomotion>(fighter).unwrap();
        assert!(!req.moving && !req.free_walk);
        assert_eq!(app.world().get::<Locomotor>(fighter).unwrap().amount, 0.0);
    }

    /// Race index 1 only rings: its health stays and it is not staggered.
    #[test]
    fn a_ringing_race_is_not_burnt() {
        let mut app = App::new();
        app.add_message::<ArcShocked>()
            .insert_resource(BodyTicks(0))
            .insert_resource(EffectRoll(Roll(0)))
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .add_systems(Update, shock_fighters);
        let mut data = exa_character::record::CharacterData::default();
        data.record.race = 1;
        let fighter = app
            .world_mut()
            .spawn((Fighter::default(), data, StateFlags::NONE))
            .id();
        app.world_mut().write_message(ArcShocked {
            body: fighter,
            direction: Vec3::X,
            strength: 0.05,
        });
        app.update();
        let yellow = app.world().get::<Fighter>(fighter).unwrap().health.yellow;
        assert!((yellow - 0.25).abs() < 1e-6);
        assert_eq!(
            app.world().get::<ShockReady>(fighter),
            Some(&ShockReady(500))
        );
        assert_eq!(app.world().get::<StateFlags>(fighter).map(|f| f.0), Some(0));
    }

    /// A collapsing fighter keeps its yellow and does not take the stagger.
    #[test]
    fn a_collapsing_fighter_is_not_staggered() {
        let mut app = App::new();
        app.add_message::<ArcShocked>()
            .insert_resource(BodyTicks(0))
            .insert_resource(EffectRoll(Roll(0)))
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .add_systems(Update, shock_fighters);
        let fighter = app
            .world_mut()
            .spawn((Fighter::default(), Condition::COLLAPSING))
            .id();
        app.world_mut().write_message(ArcShocked {
            body: fighter,
            direction: Vec3::Y,
            strength: 0.05,
        });
        app.update();
        let yellow = app.world().get::<Fighter>(fighter).unwrap().health.yellow;
        assert!((yellow - 0.25).abs() < 1e-6);
        assert_eq!(app.world().get::<StateFlags>(fighter).map(|f| f.0), Some(0));
    }

    #[test]
    fn each_struck_part_lowers_the_energy_above_a_quarter() {
        assert!((energy_after_knocks(0.26, &[0.004; 3]) - 0.248).abs() < 1e-6);
        assert_eq!(energy_after_knocks(0.25, &[0.004]), 0.25);
        assert_eq!(energy_after_knocks(1.0, &[]), 1.0);
    }

    /// The plugin builds and ticks with no game and no barricade: the
    /// interrupt path's query must resolve after the plugin's messages and
    /// assets are in place.
    #[test]
    fn the_effects_plugin_ticks_without_a_game() {
        use bevy::asset::AssetPlugin;
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, AssetPlugin::default()))
            .init_asset::<Mesh>()
            .init_asset::<GlowMaterial>()
            .init_asset::<Image>()
            .init_asset::<exa_render::force_effects::ForceRibbonMaterial>()
            .init_asset::<exa_render::force_effects::ForceShellMaterial>()
            .init_asset::<exa_render::force_effects::DispShellMaterial>()
            .init_asset::<exa_render::force_effects::TeleBubbleMaterial>()
            .init_asset::<exa_render::force_effects::ForceDomeMaterial>()
            .add_plugins(ThaumEffectsPlugin);
        for _ in 0..3 {
            app.update();
        }
    }
}

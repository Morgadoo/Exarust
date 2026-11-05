//! The mind-effect runtime: who owns which body, and how the player's
//! behaviour moves to another body and back, as the original's Control
//! Mind and Spirit Form run it.
//!
//! - **The mind component** ([`MindBody`], [`attach_minds`]): every
//!   character carries one: its power list (the Mind classes, with the body
//!   that cast an effect and the body it works on) and its status word,
//!   whose bits say who owns the body. [`HANDED_OVER`] (`0x10_0000`) marks
//!   a body whose player behaviour another body now runs (the caster of a
//!   running Control Mind or Spirit Form); [`IN_CONTROL`] (`0x60_0000`) a
//!   body the player was handed; the spirit body carries
//!   [`SPIRIT_BODY_STATUS`] (`0x22_0000`). On a body with a power caster the
//!   caster's body is the one the casts and the ticks read and write.
//! - **The player** ([`MindControl`]): the original keeps two pointers, the
//!   player's body and the body it left. Exarust's player is the body with
//!   [`PlayerBody`] under the root with [`Player`] (which the camera, the
//!   rooms and the listener follow); [`MindControl::held`] is the body left
//!   behind.
//! - **Casting** ([`cast_mind`]): the bar's Mind words run the Mind front
//!   end and its casts on the caster's body, at the body the mind pick
//!   found ([`MindPick`], [`pick_mind_target`], `mind::sight`'s pick): the
//!   character nearest the pointer on screen, within the projection of
//!   40 cm at its depth, other than the player and not collapsing.
//! - **The power walk** ([`tick_minds`], each body tick after the casters):
//!   Control Mind spends 0.002 of focus a tick while it binds and, 2 s
//!   after the cast, hands the player over ([`possess`]); its end gives the
//!   control back behind the transition screen ([`release`]). The target's
//!   possession marker ends its caster's Control Mind when it ends. Spirit
//!   Form drains the spirit's focus each tick; at its end the spirit screen
//!   fades and the transition screen brings the player home
//!   ([`spirit_home`]).
//! - **Possession** ([`possess`]): the caster's behaviour is held (its
//!   locomotion stilled, as the combatant's step clear does), the target's
//!   AI behaviour is set aside and the target gets a new player behaviour
//!   whose aim is the caster's; the player root's markers move to the
//!   target's root (the camera follows on the next frame, as the original's
//!   queued camera call); the caster keeps [`HANDED_OVER`], the target gains
//!   [`IN_CONTROL`] and the possession marker. **Release** ([`release`]) at
//!   the transition's black: the target's status becomes the caster's less
//!   the control bits (the original's own write), the caster's loses
//!   [`HANDED_OVER`], the target's AI comes back and the caster's
//!   behaviour runs again.
//! - **Spirit Form**: the cast asks the spirit body ([`SpiritBodyAsked`]):
//!   a new record of race byte 14 placed 60 cm along the caster's facing on
//!   its ground height. The game builds it and marks it [`SpiritOf`];
//!   [`adopt_spirit`] then gives it a player behaviour whose spawn snap is
//!   already counted (4), the caster's power caster and mind list (the
//!   original shares the list and rebinds its entries to the spirit), its
//!   status bits and the player root's markers, and holds the caster's
//!   behaviour. [`spirit_home`] gives them back and removes the spirit.
//! - **The hit's break** ([`break_on_blows`]): a blow on a body that is not
//!   down, whose AI is not set aside by a possession and whose status has
//!   [`HANDED_OVER`] ends the Spirit Form it is the body of, or the Control
//!   Mind it owns ([`mind_break`]).
//! - **The combatant's overrides** ([`feed_mind_powers`],
//!   [`take_broken_holds`]): before the brains' tick each body's list is
//!   written as its [`exa_ai::combatant::MindPowers`], which the perception
//!   reads; a hold the perception broke ends its power after the tick.
//!
//! Evidence: Control Mind's control handler, handover, hand-back and target
//! unlink, Spirit Form's cast, spirit-body builder, control handler, end and
//! home, the mind pick, the combatant base's hit and the mind-break walk,
//! and the transition screen they open, read from `Exanima.exe`
//! (instructions for the compares and the clock arithmetic).
//! Rules: `docs/THAUMATURGY.md` (Mind in the running game).

use bevy::prelude::*;
use exa_ai::brain::Npc;
use exa_ai::combatant::{self, Combatant, MindPowers};
use exa_ai::{DuelPowers, Duellist};
use exa_combat::{BlowNoticed, Condition, Fighter};
use exa_physics::body::BodyTicks;
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_ui::transition::{TransitionFlags, TransitionScreen};

pub use super::casts::MindBody;
use super::casts::{
    self, BodyId, CONTROL_MIND, MindClass, MindOutcome, MindPower, MindRequest, MindScene,
    POSSESSION_MARK, SPIRIT_FORM,
};
use super::sight::{PickCandidate, PickView, pick_target};
use super::{SpiritForm, step_spirit_form};
use crate::player::{Player, PlayerBody};
use crate::thaum::cast::PowerCaster;
use crate::thaum::casting::CastBody;
use crate::thaum::power::mind::Possession;
use crate::thaum::power::{ENDING, HANDED_OVER, IN_CONTROL, SPIRIT_DISTANCE};

/// The status bits the spirit body carries.
pub const SPIRIT_BODY_STATUS: u32 = crate::thaum::power::SPIRIT_STATUS;
/// The race byte of the spirit body's record (its profession word 0xE00).
pub const SPIRIT_RACE_BYTE: u8 = (crate::thaum::power::SPIRIT_RACE >> 8) as u8;
/// The spawn-snap count the spirit's player behaviour starts at.
pub const SPIRIT_SNAP_TICKS: u8 = crate::thaum::power::SPIRIT_BEHAVIOUR_MODE as u8;
/// The condition bit of a collapsing, knocked-out or dead body.
const OUT: u32 = combatant::CONDITION_OUT;
/// The mask the mind-break walk compares power ids under.
const BREAK_MASK: u32 = 0x10_ffff;
/// The masked id of Spirit Form and of Control Mind in the break's walk.
const BREAK_SPIRIT: u32 = 0x106;
const BREAK_CONTROL: u32 = 0x10c;
/// The id of Sense Minds under the pick's 0xffff mask: a body running it
/// picks without the line-of-sight test.
const SENSE_LOW: u32 = 0x101;

/// The player's two bodies, as the original keeps them: the body the player
/// left (the caster of a running Control Mind or Spirit Form) and the
/// transition the hand-back runs behind.
#[derive(Resource, Debug, Default)]
pub struct MindControl {
    /// The body the player left; `None` while the player is in its own.
    pub held: Option<Entity>,
    fade: Option<MindFade>,
    /// The spirit screen's value while it runs.
    pub spirit_screen: Option<SpiritForm>,
}

/// The transition screen a hand-back opens and what it runs at black.
#[derive(Debug, Clone, Copy)]
struct MindFade {
    screen: TransitionScreen,
    action: FadeAction,
}

/// What the transition runs at black.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FadeAction {
    /// Control Mind's hand-back.
    Release { caster: Entity, target: Entity },
    /// Spirit Form's way home.
    SpiritHome { spirit: Entity, body: Entity },
}

/// The transition screen's alpha byte while a hand-back runs.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MindFadeOverlay(pub u8);

/// The player behaviour a body holds while another body runs the player:
/// the original keeps it on the body with its hold byte set.
#[derive(Component, Debug)]
pub struct HeldPlayer(pub PlayerBody);

/// The AI behaviour a possessed body sets aside: the original keeps it in
/// the Control Mind power and marks it held.
#[derive(Component, Debug, Default)]
pub struct HeldBehaviour {
    pub duellist: Option<Duellist>,
    pub npc: Option<Npc>,
    pub powers: Option<DuelPowers>,
}

/// The spirit body of `body`: the game puts it on the spirit's model once it
/// has built it.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiritOf {
    pub body: Entity,
}

/// The spirit is in place: its behaviour and powers are the player's.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpiritAdopted;

/// Spirit Form asks for its body: a new record of race byte 14 at `at`
/// (feet, m) facing `yaw`, for the body `caster`.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct SpiritBodyAsked {
    pub caster: Entity,
    pub at: Vec3,
    pub yaw: f32,
}

/// The body the mind pick found this frame.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MindPick(pub Option<Entity>);

/// A screen point the mind pick reads instead of the window's cursor (the
/// debug cast aims it at a character).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct MindAim(pub Option<Vec3>);

/// The runtime's systems.
pub struct MindRuntimePlugin;

impl Plugin for MindRuntimePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MindControl>()
            .init_resource::<MindFadeOverlay>()
            .init_resource::<MindPick>()
            .init_resource::<MindAim>()
            .add_message::<SpiritBodyAsked>()
            .add_message::<BlowNoticed>()
            .add_systems(
                Update,
                (
                    attach_minds,
                    adopt_spirit_bodies,
                    pick_mind_target
                        .after(crate::thaum::debug_cast::run_debug_casts)
                        .before(crate::thaum::cast::run_bar_commands),
                    run_mind_fade,
                    step_spirit_screen,
                    log_mind_bodies,
                ),
            )
            .add_systems(
                FixedUpdate,
                (
                    feed_mind_powers.before(exa_ai::BrainTick),
                    take_broken_holds.after(exa_ai::BrainTick),
                    tick_minds
                        .after(exa_physics::body::BodySystems)
                        .after(crate::thaum::cast::tick_casters),
                    break_on_blows
                        .after(exa_ai::struck::HitReactions)
                        .after(tick_minds),
                ),
            );
    }
}

/// A body id for an entity: its index, unique among the live entities.
pub fn body_id(entity: Entity) -> BodyId {
    entity.index_u32()
}

/// Gives every character a mind: a body with a fighter and a condition (a
/// spirit body takes its caster's list when it is adopted).
/// Its record kind is living (1), or undead (4) for the skeleton and the
/// ancient [guess: the record kind word is not read from the record; these
/// are the two undead races of the human frame].
#[allow(clippy::type_complexity)]
pub fn attach_minds(
    mut commands: Commands,
    bodies: Query<
        (Entity, Option<&exa_character::record::CharacterData>),
        (
            With<Fighter>,
            With<Condition>,
            Without<MindBody>,
            Without<SpiritOf>,
        ),
    >,
) {
    for (entity, data) in &bodies {
        let mut mind = MindBody::new(body_id(entity));
        if data.is_some_and(|d| matches!(d.record.race, 1 | 2)) {
            mind.record_kind = 4;
        }
        commands.entity(entity).insert(mind);
    }
}

/// The body a mind's casts and ticks read: the power caster's when the body
/// has one, else the mind's own.
fn body_of<'a>(mind: &'a mut MindBody, caster: Option<&'a mut PowerCaster>) -> &'a mut CastBody {
    match caster {
        Some(caster) => &mut caster.body,
        None => &mut mind.cast,
    }
}

/// The status word of a body.
fn status_of(world: &World, entity: Entity) -> u32 {
    if let Some(caster) = world.get::<PowerCaster>(entity) {
        return caster.body.status_bits;
    }
    world
        .get::<MindBody>(entity)
        .map_or(0, |mind| mind.cast.status_bits)
}

/// Writes a body's status word, on its caster's body and its mind's.
fn set_status(world: &mut World, entity: Entity, status: impl Fn(u32) -> u32) {
    if let Some(mut caster) = world.get_mut::<PowerCaster>(entity) {
        caster.body.status_bits = status(caster.body.status_bits);
    }
    if let Some(mut mind) = world.get_mut::<MindBody>(entity) {
        mind.cast.status_bits = status(mind.cast.status_bits);
    }
}

/// What a Mind cast from the bar answered.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindCastAnswer {
    pub code: i32,
    pub outcome: Option<MindOutcome>,
}

/// The bar's Mind dispatch: the front end (while the caster runs Control
/// Mind only Control Mind is cast, which ends it) and the row's cast on the
/// caster's body `body` at the picked `target`. The caster's mind list is
/// the one the cast reads and writes.
pub fn cast_mind(
    asked: u32,
    body: &mut CastBody,
    caster: Entity,
    target: Option<Entity>,
    minds: &mut Query<&mut MindBody>,
    scene: &MindScene,
) -> MindCastAnswer {
    let refused = MindCastAnswer {
        code: casts::REFUSED,
        outcome: None,
    };
    let Ok(controlling) = minds.get(caster).map(casts::controlling) else {
        return refused;
    };
    let Some((row, cost, _held)) = casts::mind_front_end(asked, body, controlling) else {
        return refused;
    };
    let target = target.filter(|&t| t != caster);
    let outcome = match target {
        Some(target) => {
            let Ok([mut mind, mut other]) = minds.get_many_mut([caster, target]) else {
                return refused;
            };
            std::mem::swap(body, &mut mind.cast);
            let outcome = casts::mind_cast(&row, cost, &mut mind, Some(&mut other), scene);
            std::mem::swap(body, &mut mind.cast);
            outcome
        }
        None => {
            let Ok(mut mind) = minds.get_mut(caster) else {
                return refused;
            };
            std::mem::swap(body, &mut mind.cast);
            let outcome = casts::mind_cast(&row, cost, &mut mind, None, scene);
            std::mem::swap(body, &mut mind.cast);
            outcome
        }
    };
    MindCastAnswer {
        code: outcome.code,
        outcome: Some(outcome),
    }
}

/// Where the spirit body appears: the caster's feet moved 60 cm along its
/// facing (the original then lets the scene's placement move the point,
/// which is not reproduced).
pub fn spirit_point(feet: Vec3, yaw: f32) -> Vec3 {
    feet + Vec3::new(yaw.sin(), 0.0, yaw.cos()) * (SPIRIT_DISTANCE / 100.0)
}

/// Each frame, the body the mind pick ([`pick_target`]) finds under the
/// pointer (the window's, or the debug cast's aim), the camera's vertical
/// projection scale as the zoom and the depth in cm. Every character counts
/// as an AI of kind 1 with its rules bit clear, and the line of sight is
/// taken as clear [guess: the original asks the player's scene a ray from
/// eye to eye, which the running game has no query for].
#[allow(clippy::type_complexity)]
fn pick_mind_target(
    mut pick: ResMut<MindPick>,
    aim: Res<MindAim>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform, &Projection), With<crate::player::ChaseCamera>>,
    players: Query<(Entity, Option<&MindBody>), With<PlayerBody>>,
    bodies: Query<(Entity, &GlobalTransform, &Condition), (With<Fighter>, With<MindBody>)>,
) {
    let Ok((camera, eye, projection)) = cameras.single() else {
        pick.0 = None;
        return;
    };
    let Some((player, mind)) = players.iter().next() else {
        pick.0 = None;
        return;
    };
    let cursor = match aim.0 {
        Some(point) => camera.world_to_viewport(eye, point).ok(),
        None => windows.single().ok().and_then(Window::cursor_position),
    };
    let Some(cursor) = cursor else {
        pick.0 = None;
        return;
    };
    let height = camera.logical_viewport_size().map_or(720.0, |size| size.y);
    let zoom = match projection {
        Projection::Perspective(p) => 1.0 / (p.fov * 0.5).tan(),
        _ => 1.0,
    };
    let sense = mind.is_some_and(|m| m.powers.iter().any(|p| p.id & 0xffff == SENSE_LOW));
    let candidates: Vec<PickCandidate> = bodies
        .iter()
        .filter(|(entity, ..)| *entity != player)
        .filter_map(|(entity, at, condition)| {
            let point = at.translation();
            let screen = camera.world_to_viewport(eye, point).ok()?;
            // The depth in cm, as the radius's 40 is.
            let depth = (point - eye.translation()).dot(eye.forward().as_vec3()) * 100.0;
            (depth > 0.0).then_some(PickCandidate {
                id: body_id(entity),
                ai_kind: 1,
                rules: 0,
                condition: condition.0,
                visible: true,
                screen: [screen.x, screen.y, depth],
            })
        })
        .collect();
    let view = PickView {
        centre: [cursor.x as i32, cursor.y as i32],
        height: height as i32,
        zoom,
    };
    let picked = pick_target(&candidates, &view, false, sense);
    pick.0 = picked.and_then(|id| {
        bodies
            .iter()
            .map(|(entity, ..)| entity)
            .find(|&entity| body_id(entity) == id)
    });
}

/// What the power walk asks of the world after a body's tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MindAction {
    Possess {
        caster: Entity,
        target: Entity,
    },
    BeginRelease {
        caster: Entity,
        target: Entity,
    },
    UnlinkTarget {
        target: Entity,
        caster: BodyId,
    },
    EndControlOf {
        caster: BodyId,
    },
    SpiritEnds {
        spirit: Entity,
        body: Option<BodyId>,
    },
}

/// The Mind powers' walk, each body tick: Control Mind's binding, the
/// possession marker and Spirit Form (the other Mind classes keep their
/// state; their ticks are not wired). A power that answers bit 2 leaves
/// the list.
#[allow(clippy::type_complexity)]
pub fn tick_minds(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    mut minds: Query<(Entity, &mut MindBody, Option<&mut PowerCaster>)>,
    conditions: Query<&Condition>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    let ids: Vec<(BodyId, Entity)> = minds.iter().map(|(e, m, _)| (m.id, e)).collect();
    let entity_of = |id: BodyId| ids.iter().find(|(i, _)| *i == id).map(|(_, e)| *e);
    let mut actions = Vec::new();
    for (entity, mut mind, caster) in &mut minds {
        if mind.powers.is_empty() {
            continue;
        }
        let mind = &mut *mind;
        let mut powers = std::mem::take(&mut mind.powers);
        let body = body_of(mind, caster.map(Mut::into_inner));
        powers.retain_mut(|power| {
            let result = match &mut power.class {
                MindClass::ControlMind(control) => {
                    let target = control.target.and_then(entity_of);
                    let gone = target
                        .and_then(|t| conditions.get(t).ok())
                        .is_none_or(|c| c.0 & OUT != 0);
                    let step = control.control(0, now, gone, body);
                    if let Some(target) = target {
                        if step.swap {
                            actions.push(MindAction::Possess {
                                caster: entity,
                                target,
                            });
                        }
                        if step.unlink_target {
                            actions.push(MindAction::UnlinkTarget {
                                target,
                                caster: body_id(entity),
                            });
                        }
                        if step.restore && control.transferred {
                            actions.push(MindAction::BeginRelease {
                                caster: entity,
                                target,
                            });
                        }
                    }
                    step.result
                }
                MindClass::Possession(marker) => {
                    let mut scratch = CastBody::default();
                    let result = marker.control(0, &mut scratch);
                    if result & 2 != 0
                        && let Some(caster) = power.caster
                    {
                        actions.push(MindAction::EndControlOf { caster });
                    }
                    result
                }
                MindClass::Spirit(spirit) => {
                    let step = spirit.control(0, body.focus);
                    body.focus = step.focus;
                    if step.ends {
                        actions.push(MindAction::SpiritEnds {
                            spirit: entity,
                            body: power.caster,
                        });
                    }
                    step.result
                }
                _ => 0,
            };
            result & 2 == 0
        });
        // A cast during the walk may have added entries; keep them after.
        powers.append(&mut mind.powers);
        mind.powers = powers;
    }
    for action in actions {
        let entity_of = |id: BodyId| ids.iter().find(|(i, _)| *i == id).map(|(_, e)| *e);
        match action {
            MindAction::Possess { caster, target } => {
                commands.queue(move |world: &mut World| possess(world, caster, target));
            }
            MindAction::BeginRelease { caster, target } => {
                commands.queue(move |world: &mut World| {
                    begin_fade(world, FadeAction::Release { caster, target });
                });
            }
            MindAction::UnlinkTarget { target, caster } => {
                commands.queue(move |world: &mut World| unlink_target(world, target, caster));
            }
            MindAction::EndControlOf { caster } => {
                if let Some(caster) = entity_of(caster) {
                    commands.queue(move |world: &mut World| {
                        end_power(world, caster, CONTROL_MIND);
                    });
                }
            }
            MindAction::SpiritEnds { spirit, body } => {
                let body = body.and_then(entity_of);
                commands.queue(move |world: &mut World| spirit_ends(world, spirit, body));
            }
        }
    }
}

/// Ends the first power with exactly this id on a body, as the original's
/// end call does (state word 2). Answers whether one was found.
pub fn end_power(world: &mut World, entity: Entity, id: u32) -> bool {
    let Some(mut mind) = world.get_mut::<MindBody>(entity) else {
        return false;
    };
    match mind.powers.iter_mut().find(|p| p.id == id) {
        Some(power) => {
            power.end();
            true
        }
        None => false,
    }
}

/// The caster's end unlinks its target: the target's possession marker
/// whose caster is this body ends.
fn unlink_target(world: &mut World, target: Entity, caster: BodyId) {
    if let Some(mut mind) = world.get_mut::<MindBody>(target)
        && let Some(marker) = mind
            .powers
            .iter_mut()
            .find(|p| p.id == POSSESSION_MARK && p.caster == Some(caster))
    {
        marker.end();
    }
}

/// The root a body hangs under (the character's physics root).
fn root_of(world: &World, body: Entity) -> Option<Entity> {
    world.get::<ChildOf>(body).map(ChildOf::parent)
}

/// Moves the player root's markers (the player, the rooms' anchor) from
/// one body's root to another's.
fn move_player_root(world: &mut World, from: Entity, to: Entity) {
    let (Some(from_root), Some(to_root)) = (root_of(world, from), root_of(world, to)) else {
        return;
    };
    if from_root == to_root {
        return;
    }
    let mut old = world.entity_mut(from_root);
    let anchored = old.contains::<exa_world::room_view::RoomAnchor>();
    old.remove::<(Player, exa_world::room_view::RoomAnchor)>();
    let mut new = world.entity_mut(to_root);
    new.insert(Player);
    if anchored {
        new.insert(exa_world::room_view::RoomAnchor);
    }
}

/// Holds a body's player behaviour: it is kept on the body with its
/// locomotion stilled (the original's step clear on the held behaviour).
fn hold_player(world: &mut World, body: Entity) -> Option<f32> {
    let mut entity = world.entity_mut(body);
    let player = entity.take::<PlayerBody>()?;
    let aim = player.aim;
    entity.insert(HeldPlayer(player));
    if let Some(mut request) = entity.get_mut::<Locomotion>() {
        request.moving = false;
        request.free_walk = false;
    }
    if let Some(mut combatant) = entity.get_mut::<Combatant>() {
        combatant.clear_step();
    }
    Some(aim)
}

/// The body's held player behaviour runs again.
fn resume_player(world: &mut World, body: Entity) {
    let mut entity = world.entity_mut(body);
    if let Some(HeldPlayer(player)) = entity.take::<HeldPlayer>() {
        entity.insert(player);
    }
}

/// Control Mind's handover: the target becomes the player's body.
pub fn possess(world: &mut World, caster: Entity, target: Entity) {
    if world.get_entity(caster).is_err() || world.get_entity(target).is_err() {
        return;
    }
    set_status(world, caster, |s| s | HANDED_OVER);
    set_status(world, target, |s| s | IN_CONTROL);
    let caster_id = body_id(caster);
    if let Some(mut mind) = world.get_mut::<MindBody>(target) {
        let mut marker = MindPower::new(POSSESSION_MARK, MindClass::Possession(Possession::new()));
        marker.caster = Some(caster_id);
        marker.target = Some(mind.id);
        mind.powers.push(marker);
    }
    let aim = hold_player(world, caster);
    // The target's AI behaviour is set aside, held.
    let mut entity = world.entity_mut(target);
    let held = HeldBehaviour {
        duellist: entity.take::<Duellist>(),
        npc: entity.take::<Npc>(),
        powers: entity.take::<DuelPowers>(),
    };
    let yaw = entity.get::<Locomotor>().map_or(0.0, |l| l.yaw);
    let mut player = PlayerBody::new(yaw);
    if let Some(aim) = aim {
        player.aim = aim;
    }
    entity.insert((held, player));
    move_player_root(world, caster, target);
    world.resource_mut::<MindControl>().held = Some(caster);
    info!("control mind: the player takes {target:?}; {caster:?} is held");
}

/// Control Mind's hand-back, at the transition's black.
pub fn release(world: &mut World, caster: Entity, target: Entity) {
    if world.get_entity(target).is_ok() {
        // The original writes the caster's status, less the control bits,
        // over the released body's.
        let caster_status = status_of(world, caster);
        set_status(world, target, |_| caster_status & !IN_CONTROL);
        let mut entity = world.entity_mut(target);
        entity.remove::<PlayerBody>();
        if let Some(held) = entity.take::<HeldBehaviour>() {
            if let Some(duellist) = held.duellist {
                entity.insert(duellist);
            }
            if let Some(npc) = held.npc {
                entity.insert(npc);
            }
            if let Some(powers) = held.powers {
                entity.insert(powers);
            }
        }
    }
    if world.get_entity(caster).is_ok() {
        set_status(world, caster, |s| s & !HANDED_OVER);
        resume_player(world, caster);
        if world.get_entity(target).is_ok() {
            move_player_root(world, target, caster);
        }
    }
    world.resource_mut::<MindControl>().held = None;
    info!("control mind: the player is back in {caster:?}; {target:?} has its own mind");
}

/// Opens the transition screen whose black runs `action` (the original's
/// default flags: no audio fade, the 600 ms fade and the 500 ms hold).
fn begin_fade(world: &mut World, action: FadeAction) {
    let now = world
        .get_resource::<Time<Real>>()
        .map_or(0, crate::splash::clock_ms);
    let screen = TransitionScreen::new(now, TransitionFlags::default(), 0.0);
    world.resource_mut::<MindControl>().fade = Some(MindFade { screen, action });
    info!("mind: the transition opens for {action:?}");
}

/// Runs the hand-back's transition screen each frame on the real clock.
fn run_mind_fade(world: &mut World) {
    let Some(mut fade) = world.resource::<MindControl>().fade else {
        if world.resource::<MindFadeOverlay>().0 != 0 {
            world.resource_mut::<MindFadeOverlay>().0 = 0;
        }
        return;
    };
    let now = world
        .get_resource::<Time<Real>>()
        .map_or(0, crate::splash::clock_ms);
    let step = fade.screen.step(now);
    world.resource_mut::<MindFadeOverlay>().0 = step.alpha;
    if step.callback {
        fade.screen.callback = false;
        match fade.action {
            FadeAction::Release { caster, target } => release(world, caster, target),
            FadeAction::SpiritHome { spirit, body } => spirit_home(world, spirit, body),
        }
    }
    let done = step.close;
    let mut control = world.resource_mut::<MindControl>();
    control.fade = if done { None } else { Some(fade) };
    if done {
        world.resource_mut::<MindFadeOverlay>().0 = 0;
    }
}

/// Builds the spirit-body request for a Spirit Form cast by `caster`.
pub fn spirit_request(feet: Vec3, yaw: f32, caster: Entity) -> SpiritBodyAsked {
    SpiritBodyAsked {
        caster,
        at: spirit_point(feet, yaw),
        yaw,
    }
}

/// Gives the spirit body the player once the game has built it.
fn adopt_spirit_bodies(
    mut commands: Commands,
    spirits: Query<(Entity, &SpiritOf), Without<SpiritAdopted>>,
) {
    for (spirit, of) in &spirits {
        let body = of.body;
        commands.entity(spirit).insert(SpiritAdopted);
        commands.queue(move |world: &mut World| adopt_spirit(world, body, spirit));
    }
}

/// The spirit body takes the player: the caster's behaviour is held, the
/// spirit gets a player behaviour whose spawn snap is counted (4), the
/// caster's powers and mind list, the spirit's status bits and the player
/// root's markers; its AI is dropped.
pub fn adopt_spirit(world: &mut World, body: Entity, spirit: Entity) {
    if world.get_entity(body).is_err() || world.get_entity(spirit).is_err() {
        return;
    }
    let aim = hold_player(world, body);
    let mut caster = world.entity_mut(body).take::<PowerCaster>();
    // Each body keeps its own status word: the one left behind keeps the
    // caster's (its handover bit among them), the spirit starts from its own.
    let list = world
        .get_mut::<MindBody>(body)
        .map(|mut mind| {
            if let Some(caster) = &caster {
                mind.cast.status_bits = caster.body.status_bits;
            }
            std::mem::take(&mut mind.powers)
        })
        .unwrap_or_default();
    let mut entity = world.entity_mut(spirit);
    if !entity.contains::<MindBody>() {
        entity.insert(MindBody::new(body_id(spirit)));
    }
    if let (Some(caster), Some(mind)) = (caster.as_mut(), entity.get::<MindBody>()) {
        caster.body.status_bits = mind.cast.status_bits;
    }
    entity.remove::<(Duellist, Npc, DuelPowers)>();
    let yaw = entity.get::<Locomotor>().map_or(0.0, |l| l.yaw);
    let mut player = PlayerBody::new(yaw).with_snap_ticks(SPIRIT_SNAP_TICKS);
    if let Some(aim) = aim {
        player.aim = aim;
    }
    entity.insert(player);
    if let Some(caster) = caster {
        entity.insert(caster);
    }
    if let Some(mut mind) = entity.get_mut::<MindBody>() {
        mind.powers = list;
    }
    set_status(world, spirit, |s| s | SPIRIT_BODY_STATUS);
    move_player_root(world, body, spirit);
    let mut control = world.resource_mut::<MindControl>();
    control.held = Some(body);
    let now = world
        .get_resource::<BodyTicks>()
        .map_or(0, |t| t.millis() as i64);
    world.resource_mut::<MindControl>().spirit_screen = Some(SpiritForm {
        state: 3,
        value: 0.0,
        stamp: now,
    });
    info!("spirit form: the player takes the spirit {spirit:?}; {body:?} is left behind");
}

/// Spirit Form ends: the caster's handover bit clears, the spirit screen
/// fades and the transition takes the player home.
fn spirit_ends(world: &mut World, spirit: Entity, body: Option<Entity>) {
    let body = body.or_else(|| world.get::<SpiritOf>(spirit).map(|s| s.body));
    let Some(body) = body else {
        return;
    };
    set_status(world, body, |s| s & !HANDED_OVER);
    let now = world
        .get_resource::<BodyTicks>()
        .map_or(0, |t| t.millis() as i64);
    if let Some(screen) = world.resource_mut::<MindControl>().spirit_screen.as_mut() {
        screen.state = 2;
        screen.stamp = now;
    }
    info!(
        "spirit form ends: {} {} (volume {})",
        "the sound",
        crate::thaum::power::mind::SPIRIT_END_SOUND,
        crate::thaum::power::mind::SPIRIT_END_VOLUME
    );
    begin_fade(world, FadeAction::SpiritHome { spirit, body });
}

/// The way home, at the transition's black: the body takes the spirit's
/// focus, reserve and capacity change and the power list back, its
/// behaviour runs again, the player root's markers come back and the
/// spirit leaves the scene.
pub fn spirit_home(world: &mut World, spirit: Entity, body: Entity) {
    if world.get_entity(body).is_err() {
        return;
    }
    let mut caster = None;
    let mut list = Vec::new();
    if let Ok(mut entity) = world.get_entity_mut(spirit) {
        caster = entity.take::<PowerCaster>();
        if let Some(mut mind) = entity.get_mut::<MindBody>() {
            list = std::mem::take(&mut mind.powers);
        }
        entity.remove::<PlayerBody>();
    }
    if let Some(mut caster) = caster {
        if let Some(mind) = world.get::<MindBody>(body) {
            caster.body.status_bits = mind.cast.status_bits;
        }
        world.entity_mut(body).insert(caster);
    }
    if let Some(mut mind) = world.get_mut::<MindBody>(body) {
        mind.powers = list;
    }
    resume_player(world, body);
    if world.get_entity(spirit).is_ok() {
        move_player_root(world, spirit, body);
        let root = root_of(world, spirit).unwrap_or(spirit);
        world.entity_mut(root).despawn();
    }
    let mut control = world.resource_mut::<MindControl>();
    control.held = None;
    control.spirit_screen = None;
    info!("spirit form: the player is home in {body:?}; the spirit {spirit:?} leaves");
}

/// Steps the spirit screen's value on the body clock (its strength and fade
/// have no draw yet; they are logged at debug level).
fn step_spirit_screen(mut control: ResMut<MindControl>, ticks: Option<Res<BodyTicks>>) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as i64);
    let Some(screen) = control.spirit_screen.as_mut() else {
        return;
    };
    let step = step_spirit_form(screen, now, 0.0);
    debug!(
        "spirit screen: state {} value {:.3} strength {:.1} fade {:?}",
        step.state, step.value, step.strength, step.screen
    );
}

/// While the player is out of its own body, once a second of body ticks:
/// where the player's body and the held body stand (a trace for the runs
/// that check the swap).
fn log_mind_bodies(
    control: Res<MindControl>,
    ticks: Option<Res<BodyTicks>>,
    players: Query<(Entity, &GlobalTransform), With<PlayerBody>>,
    places: Query<&GlobalTransform>,
    mut last: Local<u64>,
) {
    let Some(held) = control.held else {
        return;
    };
    let now = ticks.as_deref().map_or(0, |t| t.0);
    if now < *last + 83 {
        return;
    }
    *last = now;
    for (player, at) in &players {
        info!(
            "mind: tick {now}: the player's body {player:?} at {:.2?}, the held body {held:?} at {:.2?}",
            at.translation(),
            places.get(held).map(GlobalTransform::translation).ok()
        );
    }
}

/// The hit's break: a blow on a body that is not down or dead, whose AI is
/// not set aside, and whose status has [`HANDED_OVER`].
#[allow(clippy::type_complexity)]
fn break_on_blows(
    mut commands: Commands,
    mut blows: MessageReader<BlowNoticed>,
    bodies: Query<(Option<&PowerCaster>, &MindBody), Without<HeldBehaviour>>,
) {
    for blow in blows.read() {
        if blow.down {
            continue;
        }
        let Ok((caster, mind)) = bodies.get(blow.target) else {
            continue;
        };
        let status = caster.map_or(mind.cast.status_bits, |c| c.body.status_bits);
        if status & HANDED_OVER == 0 {
            continue;
        }
        let target = blow.target;
        info!("mind break: {target:?} is struck while its mind is elsewhere");
        commands.queue(move |world: &mut World| {
            mind_break(world, target);
        });
    }
}

/// The mind-break walk over a struck body's list: a Spirit Form whose body
/// it is ends on the spirit; a Control Mind it owns ends. The original's
/// list is shared with the spirit; Exarust moves it, so a body whose list
/// is with its spirit ends the spirit's form. Answers what it ended.
pub fn mind_break(world: &mut World, body: Entity) -> Option<u32> {
    let id = body_id(body);
    let own = world.get::<MindBody>(body).and_then(|mind| {
        mind.powers.iter().find_map(|p| {
            let masked = p.id & BREAK_MASK;
            let spirit = masked == BREAK_SPIRIT && p.caster == Some(id);
            (spirit || masked == BREAK_CONTROL).then_some(p.id)
        })
    });
    if let Some(power) = own {
        end_power(world, body, power);
        return Some(power);
    }
    let mut spirits = world.query::<(Entity, &SpiritOf)>();
    let spirit = spirits
        .iter(world)
        .find(|(_, of)| of.body == body)
        .map(|(e, _)| e)?;
    end_power(world, spirit, SPIRIT_FORM).then_some(SPIRIT_FORM)
}

/// Before the brains' tick: each body's mind list as the effects the
/// perception reads.
fn feed_mind_powers(
    mut commands: Commands,
    minds: Query<(Entity, &MindBody, Option<&MindPowers>)>,
) {
    let ids: Vec<(BodyId, Entity)> = minds.iter().map(|(e, m, _)| (m.id, e)).collect();
    let entity_of = |id: BodyId| ids.iter().find(|(i, _)| *i == id).map(|(_, e)| *e);
    for (entity, mind, current) in &minds {
        let effects = mind.sight_effects(entity_of);
        if current.is_none_or(|c| c.effects != effects) {
            commands.entity(entity).insert(MindPowers { effects });
        }
    }
}

/// After the brains' tick: a hold the perception broke ends its power.
fn take_broken_holds(mut minds: Query<(&mut MindBody, &MindPowers)>) {
    for (mut mind, powers) in &mut minds {
        for (power, effect) in mind.powers.iter_mut().zip(&powers.effects) {
            if effect.state == combatant::MIND_BROKEN
                && power.id == effect.kind
                && power.state() != ENDING
            {
                info!("mind: a hold {:#x} breaks in the perception", power.id);
                power.end();
            }
        }
    }
}

/// A Mind cast's answer the bar acts on: the spirit body it asks for.
pub fn spirit_asked(outcome: &MindOutcome) -> bool {
    matches!(outcome.request, MindRequest::SpiritForm { .. })
}

/// The bar's Use on a body the player was handed (its status has the
/// control bit `0x20_0000`): the original's cast entry casts nothing and
/// asks its caller to end the control, which ends the held body's Control
/// Mind or the spirit's Spirit Form.
pub fn end_control(world: &mut World) -> bool {
    let Some(held) = world.resource::<MindControl>().held else {
        return false;
    };
    if end_power(world, held, CONTROL_MIND) {
        return true;
    }
    let mut spirits = world.query::<(Entity, &SpiritOf)>();
    let Some(spirit) = spirits
        .iter(world)
        .find(|(_, of)| of.body == held)
        .map(|(e, _)| e)
    else {
        return false;
    };
    end_power(world, spirit, SPIRIT_FORM)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::power::mind::ControlMindCaster;

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<MindControl>();
        world
    }

    /// A character: a root with a body under it.
    fn character(world: &mut World, player: bool) -> Entity {
        let root = world.spawn(Transform::default()).id();
        if player {
            world.entity_mut(root).insert(Player);
        }
        let body = world
            .spawn((ChildOf(root), Condition::default(), Locomotion::default()))
            .id();
        let mind = MindBody::new(body_id(body));
        world.entity_mut(body).insert(mind);
        if player {
            world.entity_mut(body).insert(PlayerBody::new(0.5));
        } else {
            world
                .entity_mut(body)
                .insert(Duellist::new(Entity::PLACEHOLDER));
        }
        body
    }

    #[test]
    fn a_possession_moves_the_player_and_back() {
        let mut world = world();
        let caster = character(&mut world, true);
        let target = character(&mut world, false);
        possess(&mut world, caster, target);
        assert!(world.get::<PlayerBody>(target).is_some());
        assert!(world.get::<PlayerBody>(caster).is_none());
        assert!(world.get::<HeldPlayer>(caster).is_some());
        assert!(world.get::<Duellist>(target).is_none());
        assert!(world.get::<HeldBehaviour>(target).is_some());
        assert_eq!(world.get::<PlayerBody>(target).unwrap().aim, 0.5);
        let target_root = root_of(&world, target).unwrap();
        let caster_root = root_of(&world, caster).unwrap();
        assert!(world.get::<Player>(target_root).is_some());
        assert!(world.get::<Player>(caster_root).is_none());
        assert_eq!(status_of(&world, caster) & HANDED_OVER, HANDED_OVER);
        assert_eq!(status_of(&world, target) & IN_CONTROL, IN_CONTROL);
        let marker = &world.get::<MindBody>(target).unwrap().powers[0];
        assert_eq!(marker.id, POSSESSION_MARK);
        assert_eq!(marker.caster, Some(body_id(caster)));
        assert_eq!(world.resource::<MindControl>().held, Some(caster));

        release(&mut world, caster, target);
        assert!(world.get::<PlayerBody>(caster).is_some());
        assert!(world.get::<PlayerBody>(target).is_none());
        assert!(world.get::<Duellist>(target).is_some());
        assert!(world.get::<Player>(caster_root).is_some());
        assert!(world.get::<Player>(target_root).is_none());
        assert_eq!(status_of(&world, caster) & HANDED_OVER, 0);
        // The released body takes the caster's word less the control bits,
        // written before the caster's handover bit clears.
        assert_eq!(status_of(&world, target), HANDED_OVER);
        assert_eq!(world.resource::<MindControl>().held, None);
    }

    #[test]
    fn the_released_body_keeps_the_casters_other_bits() {
        let mut world = world();
        let caster = character(&mut world, true);
        let target = character(&mut world, false);
        possess(&mut world, caster, target);
        set_status(&mut world, caster, |s| s | 0x40);
        release(&mut world, caster, target);
        // The caster's word was 0x10_0040 when the original copied it.
        assert_eq!(status_of(&world, target), HANDED_OVER | 0x40);
    }

    #[test]
    fn the_binding_hands_over_after_two_seconds() {
        let mut app = App::new();
        app.init_resource::<MindControl>();
        app.add_systems(Update, tick_minds);
        let caster = character(app.world_mut(), true);
        let target = character(app.world_mut(), false);
        let mut control = ControlMindCaster::new(0.35, 0);
        control.target = Some(body_id(target));
        let mut entry = MindPower::new(CONTROL_MIND, MindClass::ControlMind(control));
        entry.target = Some(body_id(target));
        app.world_mut()
            .get_mut::<MindBody>(caster)
            .unwrap()
            .powers
            .push(entry);
        app.world_mut().insert_resource(BodyTicks(0));
        app.update();
        assert!(app.world().get::<PlayerBody>(caster).is_some());
        // 2 s is not past the binding; one tick later it hands over.
        app.world_mut().insert_resource(BodyTicks(2000 / 12 + 1));
        app.update();
        assert!(app.world().get::<PlayerBody>(target).is_some());
        let mind = app.world().get::<MindBody>(caster).unwrap();
        assert_eq!(mind.powers[0].state(), crate::thaum::power::RUNNING);
        // The binding held 0.002 of focus a tick.
        assert!(mind.cast.spent > 0.0);
    }

    #[test]
    fn a_blow_on_the_held_body_ends_its_control() {
        let mut world = world();
        let caster = character(&mut world, true);
        let target = character(&mut world, false);
        let mut control = ControlMindCaster::new(0.35, 0);
        control.target = Some(body_id(target));
        control.state = crate::thaum::power::RUNNING;
        let entry = MindPower::new(CONTROL_MIND, MindClass::ControlMind(control));
        world
            .get_mut::<MindBody>(caster)
            .unwrap()
            .powers
            .push(entry);
        assert_eq!(mind_break(&mut world, caster), Some(CONTROL_MIND));
        assert_eq!(
            world.get::<MindBody>(caster).unwrap().powers[0].state(),
            ENDING
        );
    }

    #[test]
    fn a_blow_on_the_body_left_behind_ends_the_spirit() {
        let mut world = world();
        let body = character(&mut world, true);
        let spirit = character(&mut world, false);
        world.entity_mut(spirit).insert(SpiritOf { body });
        let mut entry = MindPower::new(
            SPIRIT_FORM,
            MindClass::Spirit(crate::thaum::power::mind::SpiritPower::new(0.001)),
        );
        entry.caster = Some(body_id(body));
        world.get_mut::<MindBody>(body).unwrap().powers.push(entry);
        let mut caster = PowerCaster::new(&[SPIRIT_FORM]);
        caster.body.status_bits = HANDED_OVER;
        world.entity_mut(body).insert(caster);
        adopt_spirit(&mut world, body, spirit);
        assert!(world.get::<MindBody>(body).unwrap().powers.is_empty());
        // The body left behind keeps the handover bit; the spirit has its
        // own word and the powers.
        assert_eq!(status_of(&world, body), HANDED_OVER);
        assert_eq!(status_of(&world, spirit), SPIRIT_BODY_STATUS);
        assert!(world.get::<PowerCaster>(spirit).is_some());
        assert_eq!(
            world.get::<PlayerBody>(spirit).unwrap().snap_ticks(),
            SPIRIT_SNAP_TICKS
        );
        assert_eq!(
            status_of(&world, spirit) & SPIRIT_BODY_STATUS,
            SPIRIT_BODY_STATUS
        );
        assert_eq!(mind_break(&mut world, body), Some(SPIRIT_FORM));
        assert_eq!(
            world.get::<MindBody>(spirit).unwrap().powers[0].state(),
            ENDING
        );
        spirit_ends(&mut world, spirit, Some(body));
        spirit_home(&mut world, spirit, body);
        assert!(world.get::<PlayerBody>(body).is_some());
        assert_eq!(world.get::<PowerCaster>(body).unwrap().body.status_bits, 0);
        assert!(world.get_entity(spirit).is_err());
        assert_eq!(world.get::<MindBody>(body).unwrap().powers.len(), 1);
    }

    #[test]
    fn the_spirit_appears_sixty_centimetres_ahead() {
        let at = spirit_point(Vec3::new(1.0, 0.0, 2.0), 0.0);
        assert!((at - Vec3::new(1.0, 0.0, 2.6)).length() < 1e-6);
    }
}

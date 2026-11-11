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
use crate::thaum::power::mind::{CONFUSED, ENRAGED, MASKED, Possession};
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
/// The rules bit the mind pick skips: the armour pass sets it on a body
/// wearing a mind ward.
pub const WARD_RULE: u32 = 0x10;

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
#[derive(Component, Debug, Default, Clone)]
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

/// The body the dead powers' pick found this frame (collapsed bodies
/// allowed): Raise Dead and Command Dead cast at it.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MindDeadPick(pub Option<Entity>);

/// A screen point the mind pick reads instead of the window's cursor (the
/// debug cast aims it at a character).
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq)]
pub struct MindAim(pub Option<Vec3>);

/// The runtime's systems.
pub struct MindRuntimePlugin;

impl Plugin for MindRuntimePlugin {
    fn build(&self, app: &mut App) {
        bridge_mind_runtime_types(app);
        app.init_resource::<MindControl>()
            .init_resource::<MindFadeOverlay>()
            .init_resource::<MindPick>()
            .init_resource::<MindDeadPick>()
            .init_resource::<MindAim>()
            .add_message::<SpiritBodyAsked>()
            .add_message::<BlowNoticed>()
            .add_message::<exa_audio::PointSounded>()
            .add_message::<exa_audio::MusicOverlay>()
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
                    feed_mind_status.before(exa_ai::BrainTick),
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

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_mind_runtime_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_copy::<HeldBehaviour>()
        .bridge_message::<exa_audio::PointSounded>()
        .bridge_message::<exa_combat::BlowNoticed>()
        .bridge_copy::<exa_ai::combatant::MindPowers>()
        .bridge_copy::<crate::mind::casts::MindBody>();
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
#[derive(Debug, Clone, PartialEq)]
pub struct MindCastAnswer {
    pub code: i32,
    pub outcome: Option<MindOutcome>,
    /// The changes the cast made to its target's AI, for the live brain
    /// ([`post_mind_ai`]).
    pub ai: Vec<casts::MindAiChange>,
}

/// What a cast reads off its target's body, as the original's casts read
/// it straight from the body: the condition word, the rules, the feral
/// level (the dead powers' decay), the yellow and red health, the record's
/// profession word (the dead powers' character flags: its `0x40` bit
/// refuses them, its race byte keeps a corpse from the revival) and the
/// encumbrance and build factor the revival's step reach reads. The mind
/// keeps a copy, which the bar refreshes before each cast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindLive {
    pub condition: u32,
    pub rules: u32,
    pub decay: f32,
    pub yellow: f32,
    pub red: f32,
    pub profession: u32,
    pub load: f32,
    pub build: f32,
}

impl MindLive {
    pub fn of(
        condition: &Condition,
        fighter: Option<&Fighter>,
        data: Option<&exa_character::record::CharacterData>,
        body: Option<&exa_physics::body::Ragdoll>,
    ) -> Self {
        let stats = body.and_then(|b| b.drive.as_ref()).map(|d| d.stats);
        Self {
            condition: condition.0,
            rules: fighter.map_or(0, |f| f.stats.rules.0),
            decay: fighter.map_or(0.0, |f| f.stats.feral),
            yellow: fighter.map_or(1.0, |f| f.health.yellow),
            red: fighter.map_or(1.0, |f| f.health.red),
            profession: data.map_or(0, |d| d.record.profession()),
            load: stats.map_or(0.0, |s| s.encumbrance),
            build: stats.map_or(1.0, |s| s.scale.max(0.0).sqrt()),
        }
    }

    /// Writes the values on the target's mind.
    pub fn write(&self, mind: &mut MindBody) {
        mind.cast.condition = self.condition;
        mind.rules = self.rules;
        mind.decay = self.decay;
        mind.yellow = self.yellow;
        mind.red = self.red;
        mind.character_flags = self.profession;
        mind.load = self.load;
        mind.build = self.build;
    }
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
    live: Option<MindLive>,
    minds: &mut Query<&mut MindBody>,
    scene: &MindScene,
) -> MindCastAnswer {
    let refused = MindCastAnswer {
        code: casts::REFUSED,
        outcome: None,
        ai: Vec::new(),
    };
    let Ok(controlling) = minds.get(caster).map(casts::controlling) else {
        return refused;
    };
    let Some((row, cost, _held)) = casts::mind_front_end(asked, body, controlling) else {
        return refused;
    };
    let target = target.filter(|&t| t != caster);
    let mut ai = Vec::new();
    let outcome = match target {
        Some(target) => {
            let Ok([mut mind, mut other]) = minds.get_many_mut([caster, target]) else {
                return refused;
            };
            if let Some(live) = live {
                live.write(&mut other);
            }
            std::mem::swap(body, &mut mind.cast);
            let outcome = casts::mind_cast(&row, cost, &mut mind, Some(&mut other), scene);
            std::mem::swap(body, &mut mind.cast);
            ai = other.ai.take_changes();
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
        ai,
    }
}

/// Hands a cast's changes to its target's AI to the world that runs the
/// brains (the simulation's behind the bridge, at the start of its next
/// step), where they run on the brain's own values ([`drive_mind_ai`]).
pub fn post_mind_ai(
    fixed: Option<&mut exa_physics::sim_app::FixedCommands>,
    commands: &mut Commands,
    target: Entity,
    changes: Vec<casts::MindAiChange>,
) {
    if changes.is_empty() {
        return;
    }
    match fixed {
        Some(fixed) => fixed.run(drive_mind_ai, (target, changes)),
        None => commands.queue(move |world: &mut World| {
            if let Err(e) = world.run_system_cached_with(drive_mind_ai, (target, changes)) {
                warn!("mind ai: {e}");
            }
        }),
    }
}

/// The brain's values a Mind cast moves, as [`casts::MindAi`]: its
/// contacts (a contact's body is the character it names), the morale's
/// panic, alarm and courage, the integration's readiness, calm and level,
/// and the body's target.
pub fn mind_ai_of(npc: &Npc, target: Option<Entity>) -> casts::MindAi {
    casts::MindAi {
        contacts: npc
            .contacts
            .iter()
            .map(|c| casts::MindContact {
                body: Entity::try_from_bits(c.id).map_or(u32::MAX, body_id),
                noticed: c.noticed,
                concern: c.concern,
                hostility: c.hostility,
                others: [c.sighting, c.attention, c.concern_target, c.soothe],
            })
            .collect(),
        panic: npc.morale.panic,
        alarm: npc.morale.alarm,
        readiness: npc.integration.readiness,
        courage: npc.morale.courage,
        calm: npc.integration.decay,
        level: npc.integration.level,
        focus: target.map(body_id),
        rearm: npc.reapply,
        changes: Vec::new(),
    }
}

/// Writes [`casts::MindAi`]'s values back on the brain and the body's
/// target (cleared when the change cleared its focus).
pub fn write_mind_ai(ai: &casts::MindAi, npc: &mut Npc, target: &mut Option<Entity>) {
    for (c, m) in npc.contacts.iter_mut().zip(&ai.contacts) {
        c.noticed = m.noticed;
        c.concern = m.concern;
        c.hostility = m.hostility;
        [c.sighting, c.attention, c.concern_target, c.soothe] = m.others;
    }
    npc.morale.panic = ai.panic;
    npc.morale.alarm = ai.alarm;
    npc.integration.alarm = ai.alarm;
    npc.integration.readiness = ai.readiness;
    npc.morale.courage = ai.courage;
    npc.integration.courage = ai.courage;
    npc.integration.decay = ai.calm;
    npc.integration.level = ai.level;
    if ai.focus.is_none() {
        *target = None;
    }
    npc.reapply = ai.rearm;
}

/// A Mind cast's changes to a body's AI, run on its live brain: the values
/// are read off the brain, changed as the cast changes them, and written
/// back. A body without a brain (the arena's duellist, the player) keeps
/// no such values.
#[allow(clippy::type_complexity)]
pub fn drive_mind_ai(
    In((entity, changes)): In<(Entity, Vec<casts::MindAiChange>)>,
    mut brains: Query<(&mut Npc, Option<&mut Combatant>)>,
) {
    let Ok((mut npc, combatant)) = brains.get_mut(entity) else {
        info!("mind ai: {entity:?} has no brain for {changes:?}");
        return;
    };
    let mut scratch = None;
    let target = match combatant {
        Some(c) => &mut c.into_inner().target,
        None => &mut scratch,
    };
    let mut ai = mind_ai_of(&npc, *target);
    let before = (ai.panic, ai.alarm, ai.readiness, ai.courage, ai.level);
    for &change in &changes {
        ai.apply(change);
    }
    write_mind_ai(&ai, &mut npc, target);
    info!(
        "mind ai: {entity:?}'s brain takes {changes:?}: panic {:.3} -> {:.3}, alarm {:.3} -> {:.3}, \
         readiness {:.3} -> {:.3}, courage {:.3} -> {:.3}, level {} -> {}, calm {:.3}, \
         {} contacts, target {:?}, re-arm {}",
        before.0,
        ai.panic,
        before.1,
        ai.alarm,
        before.2,
        ai.readiness,
        before.3,
        ai.courage,
        before.4,
        ai.level,
        ai.calm,
        ai.contacts.len(),
        *target,
        ai.rearm
    );
}

/// Where the spirit body appears: the caster's feet moved 60 cm along its
/// facing (the original then lets the scene's placement move the point,
/// which is not reproduced).
pub fn spirit_point(feet: Vec3, yaw: f32) -> Vec3 {
    feet + Vec3::new(yaw.sin(), 0.0, yaw.cos()) * (SPIRIT_DISTANCE / 100.0)
}

/// Whether the player's view sees a body, as the original's mind pick asks
/// it: the level's light-reach query between the two bodies' places (the
/// same room, or a door link of one room into the other's cell whose door
/// point lies within reach of it, through an open door). Without a level
/// room grid the original's default answer is yes.
pub fn view_sees(rooms: Option<&exa_world::room_view::LevelRooms>, from: Vec3, to: Vec3) -> bool {
    let Some(level) = rooms else {
        return true;
    };
    level
        .rooms
        .light_reach(from.xz(), to.xz(), |link| level.tracker.is_open(link))
}

/// Each frame, the bodies the mind pick ([`pick_target`]) finds under the
/// pointer (the window's, or the debug cast's aim), with and without the
/// collapsed, the camera's vertical projection scale as the zoom and the
/// depth in cm. Every character counts as an AI of kind 1; a body wearing a
/// mind ward carries the rules bit `0x10` (the armour pass sets it with the
/// masked bit) and is never picked; without Sense Minds the player's view
/// must see it ([`view_sees`]).
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn pick_mind_target(
    mut pick: ResMut<MindPick>,
    mut dead_pick: ResMut<MindDeadPick>,
    aim: Res<MindAim>,
    rooms: Option<Res<exa_world::room_view::LevelRooms>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform, &Projection), With<crate::player::ChaseCamera>>,
    players: Query<(Entity, &GlobalTransform, Option<&MindBody>), With<PlayerBody>>,
    bodies: Query<
        (
            Entity,
            &GlobalTransform,
            &Condition,
            Has<exa_ai::brain::MindWarded>,
        ),
        (With<Fighter>, With<MindBody>),
    >,
) {
    let Ok((camera, eye, projection)) = cameras.single() else {
        pick.0 = None;
        dead_pick.0 = None;
        return;
    };
    let Some((player, player_at, mind)) = players.iter().next() else {
        pick.0 = None;
        dead_pick.0 = None;
        return;
    };
    let cursor = match aim.0 {
        Some(point) => camera.world_to_viewport(eye, point).ok(),
        None => windows.single().ok().and_then(Window::cursor_position),
    };
    let Some(cursor) = cursor else {
        pick.0 = None;
        dead_pick.0 = None;
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
        .filter_map(|(entity, at, condition, warded)| {
            let point = at.translation();
            let screen = camera.world_to_viewport(eye, point).ok()?;
            // The depth in cm, as the radius's 40 is.
            let depth = (point - eye.translation()).dot(eye.forward().as_vec3()) * 100.0;
            (depth > 0.0).then_some(PickCandidate {
                id: body_id(entity),
                ai_kind: 1,
                rules: if warded { WARD_RULE } else { 0 },
                condition: condition.0,
                visible: view_sees(rooms.as_deref(), player_at.translation(), point),
                screen: [screen.x, screen.y, depth],
            })
        })
        .collect();
    let view = PickView {
        centre: [cursor.x as i32, cursor.y as i32],
        height: height as i32,
        zoom,
    };
    let entity_of = |id: BodyId| {
        bodies
            .iter()
            .map(|(entity, ..)| entity)
            .find(|&entity| body_id(entity) == id)
    };
    pick.0 = pick_target(&candidates, &view, false, sense).and_then(entity_of);
    dead_pick.0 = pick_target(&candidates, &view, true, sense).and_then(entity_of);
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
    /// A caster's end unlinks its target: the first entry `id` on body
    /// `on` whose caster is `caster` ends.
    EndMarker {
        on: BodyId,
        id: u32,
        caster: BodyId,
    },
    /// A marker's end ends its caster's power: the first entry `id` on body
    /// `on` whose target is `target` ends.
    EndCasterPower {
        on: BodyId,
        id: u32,
        target: BodyId,
    },
    /// The body's AI re-arms its behaviour (the original's behaviour byte).
    Rearm(Entity),
    /// A Command Dead caster's end lays its raised body down again.
    LayDown(BodyId),
}

/// The status bits the timed Mind classes own on a body.
const MIND_STATUS_BITS: u32 = MASKED | CONFUSED | ENRAGED;

/// What one walk over a body's list ended, for the log.
fn class_name(class: &MindClass) -> &'static str {
    match class {
        MindClass::SenseInstant(_) => "Sense Minds",
        MindClass::Sense(_) => "Sense Minds (persistent)",
        MindClass::MaskInstant(_) => "Mask Mind",
        MindClass::Mask(_) => "Mask Mind (persistent)",
        MindClass::Confusion(_) => "Confusion",
        MindClass::Enrage(_) => "Enrage",
        MindClass::Trust(_) => "Trust",
        MindClass::Suspicion(_) => "Suspicion",
        MindClass::Loyalty(_) => "Loyalty",
        MindClass::LoyaltyMark(_) => "Loyalty's marker",
        MindClass::CommandDead(_) => "Command Dead",
        MindClass::CommandDeadMark(_) => "Command Dead's marker",
        MindClass::ControlMind(_) => "Control Mind",
        MindClass::Possession(_) => "the possession marker",
        MindClass::Spirit(_) => "Spirit Form",
        MindClass::Other(_) => "a power",
    }
}

/// The Mind powers' walk, each body tick, as the original's list walk asks
/// each entry's control with event 0:
///
/// - **Control Mind**, the possession marker and **Spirit Form**: the
///   binding, the handover and the form's drain (above).
/// - **Sense Minds**: the instant one answers 4 when its end passes (the
///   overlay's fade starts) and 2 two seconds later (on the real clock);
///   the persistent one runs until it is ended and gives its focus back.
/// - **Mask Mind**: the instant one keeps the masked bit for 15 s and
///   clears it as it expires; the persistent one sets the bit every tick
///   and clears it at its end, giving its focus back.
/// - **Confusion** and **Enrage**: 15 s on the scene clock (unsigned
///   elapsed time), then the confused (`0x4_0000`) or enraged
///   (`0x8_0000`) bit clears on the body that carries the effect.
/// - **Trust** and **Suspicion**: 20 s (signed 64-bit elapsed time), then
///   the marker ends.
/// - **Loyalty**: the caster's power waits for its end, then ends the
///   target's marker and gives its focus back; the marker counts its ticks
///   (its strength follows the relation after 800), and at its end ends
///   the caster's power, re-arms its AI and plays `ThmMnd_E01` at 0.15.
/// - **Command Dead**: the caster's end ends the raised body's marker and
///   lays the body down again; the marker's end ends the caster's power and
///   re-arms its AI.
///
/// A power that answers bit 2 leaves the list. Each body's three status
/// bits the walk owns are kept on the power caster's body (when it has one)
/// and copied back to its mind.
#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn tick_minds(
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    mut minds: Query<(Entity, &mut MindBody, Option<&mut PowerCaster>)>,
    conditions: Query<&Condition>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
    mut frame: Option<ResMut<exa_physics::sim_app::FrameCommands>>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    let ids: Vec<(BodyId, Entity)> = minds.iter().map(|(e, m, _)| (m.id, e)).collect();
    let entity_of = |id: BodyId| ids.iter().find(|(i, _)| *i == id).map(|(_, e)| *e);
    let mut actions = Vec::new();
    for (entity, mut mind, mut caster) in &mut minds {
        if mind.powers.is_empty() {
            continue;
        }
        let mind = &mut *mind;
        let own_id = mind.id;
        // The casts write a target's bits on its mind; its caster's body is
        // the word the walk reads.
        if let Some(caster) = caster.as_deref_mut() {
            caster.body.status_bits |= mind.cast.status_bits & MIND_STATUS_BITS;
        }
        let mut powers = std::mem::take(&mut mind.powers);
        let body = body_of(mind, caster.as_deref_mut());
        powers.retain_mut(|power| {
            let caster_id = power.caster;
            let target_id = power.target;
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
                MindClass::SenseInstant(sense) => {
                    let result = sense.control(now);
                    if result == 4 {
                        info!("sense minds: {entity:?}'s sense passes its end; the overlay fades");
                    }
                    result
                }
                MindClass::Sense(sense) => sense.control(0, body),
                MindClass::MaskInstant(mask) => {
                    let shown = mask.step(now, body);
                    if shown.code == 4 {
                        info!("mask mind: {entity:?}'s inner mask starts to fade");
                    }
                    shown.code
                }
                MindClass::Mask(mask) => mask.control(0, body),
                MindClass::Confusion(timed) | MindClass::Enrage(timed) => {
                    let step = timed.control(now, body.status_bits);
                    body.status_bits = step.flags;
                    step.result
                }
                MindClass::Trust(timed) | MindClass::Suspicion(timed) => timed.control(now),
                MindClass::Loyalty(loyalty) => {
                    let step = loyalty.control(0, None, body);
                    if step.unlink
                        && let Some(target) = loyalty.target.or(target_id)
                    {
                        actions.push(MindAction::EndMarker {
                            on: target,
                            id: casts::LOYALTY_MARK,
                            caster: own_id,
                        });
                    }
                    step.result
                }
                MindClass::LoyaltyMark(mark) => {
                    let result = mark.control(0, 0.0);
                    if result & 2 != 0 {
                        if let Some(caster) = mark.caster.or(caster_id) {
                            actions.push(MindAction::EndCasterPower {
                                on: caster,
                                id: casts::LOYALTY,
                                target: own_id,
                            });
                        }
                        actions.push(MindAction::Rearm(entity));
                        play_mind_sound(&mut sounds, casts::END_SOUND);
                    }
                    result
                }
                MindClass::CommandDead(command) => {
                    let step = command.control(0, None, body);
                    if step.unlink
                        && let Some(target) = command.target.or(target_id)
                    {
                        actions.push(MindAction::EndMarker {
                            on: target,
                            id: casts::COMMAND_DEAD_MARK,
                            caster: own_id,
                        });
                        actions.push(MindAction::LayDown(target));
                    }
                    step.result
                }
                MindClass::CommandDeadMark(mark) => {
                    let (result, ended) = mark.control(0);
                    if ended {
                        if let Some(caster) = caster_id {
                            actions.push(MindAction::EndCasterPower {
                                on: caster,
                                id: casts::COMMAND_DEAD,
                                target: own_id,
                            });
                        }
                        actions.push(MindAction::Rearm(entity));
                    }
                    result
                }
                MindClass::Other(_) => 0,
            };
            if result & 2 != 0 {
                info!(
                    "mind: {} {:#x} on {entity:?} ends at {now} ms",
                    class_name(&power.class),
                    power.id
                );
            }
            result & 2 == 0
        });
        // A cast during the walk may have added entries; keep them after.
        powers.append(&mut mind.powers);
        mind.powers = powers;
        if let Some(caster) = caster.as_deref() {
            mind.cast.status_bits = (mind.cast.status_bits & !MIND_STATUS_BITS)
                | (caster.body.status_bits & MIND_STATUS_BITS);
        }
    }
    for action in actions {
        let entity_of = |id: BodyId| ids.iter().find(|(i, _)| *i == id).map(|(_, e)| *e);
        match action {
            // The handover and the hand-back's transition are the frame's:
            // they move the player behaviour and the view and write the
            // control state the frame keeps, so behind the bridge they run
            // on the frame's world after the step.
            MindAction::Possess { caster, target } => {
                let run = move |world: &mut World| possess(world, caster, target);
                match frame.as_deref_mut() {
                    Some(frame) => frame.send(run),
                    None => commands.queue(run),
                }
            }
            MindAction::BeginRelease { caster, target } => {
                let run = move |world: &mut World| {
                    begin_fade(world, FadeAction::Release { caster, target });
                };
                match frame.as_deref_mut() {
                    Some(frame) => frame.send(run),
                    None => commands.queue(run),
                }
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
                let run = move |world: &mut World| spirit_ends(world, spirit, body);
                match frame.as_deref_mut() {
                    Some(frame) => frame.send(run),
                    None => commands.queue(run),
                }
            }
            MindAction::EndMarker { on, id, caster } => {
                if let Some(on) = entity_of(on) {
                    commands.queue(move |world: &mut World| {
                        end_entry(world, on, id, |p| p.caster == Some(caster));
                    });
                }
            }
            MindAction::EndCasterPower { on, id, target } => {
                if let Some(on) = entity_of(on) {
                    commands.queue(move |world: &mut World| {
                        end_entry(world, on, id, |p| p.target == Some(target));
                    });
                }
            }
            MindAction::Rearm(body) => {
                commands.queue(move |world: &mut World| {
                    if let Some(mut mind) = world.get_mut::<MindBody>(body) {
                        mind.ai.rearm = true;
                    }
                    match world.get_mut::<Npc>(body) {
                        Some(mut npc) => {
                            npc.reapply = true;
                            info!("mind: {body:?}'s AI re-arms its behaviour");
                        }
                        None => info!("mind: {body:?} has no brain to re-arm"),
                    }
                });
            }
            MindAction::LayDown(target) => {
                if let Some(target) = entity_of(target) {
                    commands.queue(move |world: &mut World| {
                        if let Err(e) = world.run_system_cached_with(lay_down, target) {
                            warn!("command dead: {e}");
                        }
                    });
                }
            }
        }
    }
}

/// A raise the bar's cast asked for, run on the world that steps the
/// bodies at the start of the next fixed step (the simulation's behind the
/// bridge, where the bodies' condition, health and muscles live).
pub fn post_raise(
    fixed: Option<&mut exa_physics::sim_app::FixedCommands>,
    commands: &mut Commands,
    target: Entity,
    revival: Option<casts::Revival>,
) {
    match fixed {
        Some(fixed) => fixed.run(raise_corpse, (target, revival)),
        None => commands.queue(move |world: &mut World| {
            if let Err(e) = world.run_system_cached_with(raise_corpse, (target, revival)) {
                warn!("raise dead: {e}");
            }
        }),
    }
}

/// The dead powers' raise of a corpse, in the original's order: the loot
/// panel open on the body closes (Exarust opens none on a corpse: logged),
/// a body without decay is revived (its record's and its character's feral
/// level become the revival's 0.2, its rules gain the decayed bit and its
/// most energy follows), the body is revived as the AI manager's rise
/// revives it ([`crate::rise::revive_corpse`]: knocked out at once, its
/// muscles back, so it gets up), and its yellow and red health become a
/// half and full, written over the revive's own yellow.
#[allow(clippy::type_complexity)]
pub fn raise_corpse(
    In((entity, revival)): In<(Entity, Option<casts::Revival>)>,
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    mut bodies: Query<(
        &mut Condition,
        Option<&mut exa_combat::StateFlags>,
        Option<&mut exa_combat::condition::KnockOut>,
        &mut Fighter,
        &mut exa_physics::body::Ragdoll,
        Option<&mut exa_combat::voice::CharacterVoice>,
        Option<&mut exa_character::held::Hands>,
        Option<&mut Npc>,
        Option<&exa_ai::roles::Roles>,
        Option<&exa_items::possession::Possessions>,
        Option<&mut exa_character::record::CharacterData>,
    )>,
) {
    let now = ticks.map_or(0, |t| t.millis() as u32);
    let Ok((
        mut condition,
        flags,
        knock,
        mut fighter,
        mut body,
        voice,
        hands,
        npc,
        roles,
        possessions,
        data,
    )) = bodies.get_mut(entity)
    else {
        warn!("raise dead: {entity:?} has no body to raise");
        return;
    };
    if *condition != Condition::DEAD {
        info!(
            "raise dead: {entity:?} is no longer dead ({:#x}); nothing rises",
            condition.0
        );
        return;
    }
    debug!("raise dead: no loot panel is open on {entity:?} (Exarust opens none on a corpse)");
    if let Some(revival) = revival {
        if let Some(mut data) = data {
            data.record.feral = revival.decay;
        }
        fighter.stats.feral = revival.decay;
        fighter.stats.rules.0 |= casts::DECAYED_RULE;
        fighter.stats.max_energy = revival.most_energy;
        info!(
            "raise dead: {entity:?} is revived with decay {}: gait scale {}, pace {} ticks, \
             most energy {}, stamina {}",
            revival.decay,
            revival.gait_scale,
            revival.pace_ticks,
            revival.most_energy,
            revival.stamina
        );
    }
    let parts = crate::rise::RevivedParts {
        flags,
        knock,
        voice,
        hands,
        npc,
        roles,
        possessions,
    };
    let Some(revived) = crate::rise::revive_corpse(
        &mut commands,
        entity,
        now,
        &mut condition,
        &mut fighter,
        &mut body,
        parts,
    ) else {
        return;
    };
    if let Some(revival) = revival {
        // The revival's values on the body the revive muscled again: the
        // pins' factor (the gait scale), the most energy and the feral
        // level, whose step reach is the revival's stamina, and the root's
        // link-iteration count (the pace), which the body tick then holds
        // at its feral cadence.
        let reach = body.drive.as_mut().map(|drive| {
            drive.stats.pin_factor = revival.gait_scale;
            drive.stats.max_energy = revival.most_energy;
            drive.stats.feral = revival.decay;
            drive.stats.foot_reach()
        });
        body.set_link_iterations(revival.pace_ticks);
        info!(
            "raise dead: {entity:?}'s body takes the revival: pins {}, pace {} ticks, most energy \
             {}, step reach {reach:?} (the revival's stamina {})",
            revival.gait_scale, revival.pace_ticks, revival.most_energy, revival.stamina
        );
    }
    fighter.health.yellow = casts::RAISED_YELLOW;
    fighter.health.red = casts::RAISED_RED;
    commands.entity(entity).remove::<(
        exa_combat::condition::DeathClock,
        exa_combat::collapse::Collapse,
    )>();
    info!(
        "raise dead: {entity:?} rises at {now} ms: yellow {} red {} (the revive's yellow {:.3}), \
         race stance {}, scale {}, unarmed set-up {}",
        casts::RAISED_YELLOW,
        casts::RAISED_RED,
        revived.yellow,
        revived.stance,
        revived.scale,
        revived.rearm
    );
}

/// Command Dead's end lays the raised body down again: it collapses as a
/// lethal wound starts a collapse ([`exa_combat::collapse::start_collapse`]
/// at its energy), and its dying tick then lets it die. The original also
/// clears a body word beside it whose use is not traced (not reproduced).
#[allow(clippy::type_complexity)]
pub fn lay_down(
    In(entity): In<Entity>,
    mut commands: Commands,
    mut bodies: Query<(
        &mut Condition,
        Option<&mut exa_combat::StateFlags>,
        Option<&mut exa_anim::Animator>,
        &mut exa_physics::body::Ragdoll,
        Option<&mut exa_combat::collapse::Hands>,
    )>,
) {
    let Ok((mut condition, flags, anim, mut body, hands)) = bodies.get_mut(entity) else {
        return;
    };
    let Some(mut anim) = anim else {
        info!("command dead: {entity:?} has no clip player yet; it does not lie down");
        return;
    };
    let energy = body.drive.as_ref().map_or(0.0, |d| d.energy);
    let mut word = flags.as_deref().copied().unwrap_or_default();
    if !exa_combat::collapse::start_collapse(
        &mut condition,
        &mut word,
        &mut anim,
        Some(&mut body),
        energy,
        hands.map(Mut::into_inner),
    ) {
        info!("command dead: {entity:?} is collapsing or dead already");
        return;
    }
    match flags {
        Some(mut f) => *f = word,
        None => {
            commands.entity(entity).insert(word);
        }
    }
    commands
        .entity(entity)
        .insert(exa_combat::collapse::Collapse::default());
    info!("command dead: the raised body {entity:?} lies down again (energy {energy:.3})");
}

/// Ends the first entry `id` on `body` that `matches` (the original's
/// unlink walks stop at the first match).
fn end_entry(world: &mut World, body: Entity, id: u32, matches: impl Fn(&MindPower) -> bool) {
    let Some(mut mind) = world.get_mut::<MindBody>(body) else {
        return;
    };
    if let Some(power) = mind.powers.iter_mut().find(|p| p.id == id && matches(p)) {
        power.end();
        info!("mind: {id:#x} on {body:?} is told to end");
    }
}

/// A Mind sound as the original plays its cast and end sounds: at the
/// listener, unattenuated, on the interface bus (flags `0x4060`), pitch 1.
pub const MIND_SOUND_FLAGS: u32 = 0x4060;

/// Plays a Mind sound.
pub fn play_mind_sound(
    sounds: &mut MessageWriter<exa_audio::PointSounded>,
    sound: casts::MindSound,
) {
    info!("mind sound: {} at volume {}", sound.name, sound.volume);
    sounds.write(exa_audio::PointSounded {
        name: sound.name.to_string(),
        position: Vec3::ZERO,
        volume: sound.volume,
        pitch: 1.0,
        flags: MIND_SOUND_FLAGS,
    });
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
    // The handover first sweeps the GUI's forms: the inventory, container,
    // journal and loot windows close, then the power tree closes, or the
    // shown bar falls.
    crate::gui_screens::sweep_screens(world, crate::gui_screens::ScreenSweep::ContainerFamily);
    // The status bits and the marker are written on the world that walks
    // the powers (see `on_power_walk`).
    on_power_walk(world, false, move |world: &mut World| {
        possess_minds(world, caster, target);
    });
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

/// The handover's minds: the caster keeps [`HANDED_OVER`], the target
/// gains [`IN_CONTROL`] and the possession marker whose caster is the
/// caster.
fn possess_minds(world: &mut World, caster: Entity, target: Entity) {
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
    info!("control mind: {caster:?} is handed over, {target:?} carries the possession marker");
}

/// The hand-back's minds: the original writes the caster's status, less
/// the control bits, over the released body's, then the caster's loses
/// [`HANDED_OVER`].
fn release_minds(world: &mut World, caster: Entity, target: Entity) {
    if world.get_entity(target).is_ok() {
        let caster_status = status_of(world, caster);
        set_status(world, target, |_| caster_status & !IN_CONTROL);
    }
    if world.get_entity(caster).is_ok() {
        set_status(world, caster, |s| s & !HANDED_OVER);
    }
    info!("control mind: {caster:?}'s and {target:?}'s status words are handed back");
}

/// Control Mind's hand-back, at the transition's black.
pub fn release(world: &mut World, caster: Entity, target: Entity) {
    // The hand-back sweeps the GUI's forms first, as the handover does.
    crate::gui_screens::sweep_screens(world, crate::gui_screens::ScreenSweep::ContainerFamily);
    // The status words are written on the world that walks the powers.
    on_power_walk(world, false, move |world: &mut World| {
        release_minds(world, caster, target);
    });
    if world.get_entity(target).is_ok() {
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
    // The list moves on the world that walks it (see `move_mind_list`).
    let left_status = caster.as_ref().map(|c| c.body.status_bits);
    let mut entity = world.entity_mut(spirit);
    if !entity.contains::<MindBody>() {
        entity.insert(MindBody::new(body_id(spirit)));
    }
    if let (Some(caster), Some(mind)) = (caster.as_mut(), entity.get::<MindBody>()) {
        caster.body.status_bits = mind.cast.status_bits;
    }
    entity.remove::<(Duellist, Npc, DuelPowers)>();
    let yaw = entity.get::<Locomotor>().map_or(0.0, |l| l.yaw);
    // The spirit's race (the wraith) hovers: its combat mode stays off.
    let mut player = PlayerBody::new(yaw)
        .with_snap_ticks(SPIRIT_SNAP_TICKS)
        .hovering();
    if let Some(aim) = aim {
        player.aim = aim;
    }
    entity.insert(player);
    if let Some(caster) = caster {
        entity.insert(caster);
    }
    set_status(world, spirit, |s| s | SPIRIT_BODY_STATUS);
    // The spirit's new mind reaches the simulation with this frame's
    // changes; the list moves after them.
    on_power_walk(world, false, move |world: &mut World| {
        move_mind_list(world, body, spirit, left_status);
    });
    move_player_root(world, body, spirit);
    let mut control = world.resource_mut::<MindControl>();
    control.held = Some(body);
    let now = world
        .get_resource::<Time<Real>>()
        .map_or(0, |t| i64::from(crate::splash::clock_ms(t)));
    world.resource_mut::<MindControl>().spirit_screen = Some(SpiritForm {
        state: 3,
        value: 0.0,
        stamp: now,
    });
    info!("spirit form: the player takes the spirit {spirit:?}; {body:?} is left behind");
}

/// Runs `change` to the power lists on the world that walks them. Behind
/// the simulation bridge that is the simulation's: the change is posted to
/// its next step, `first` before the frame's other changes of this frame
/// are applied there, else after them. The frame's copy of a list can lag
/// the simulation's by the steps in flight (a slow frame applies a hundred
/// steps computed before its last write), so a list moved there, or any
/// write of a mind there, could hand the simulation a stale list that
/// wins. In one world the change runs now.
fn on_power_walk(world: &mut World, first: bool, change: impl FnOnce(&mut World) + Send + 'static) {
    match world.get_resource_mut::<exa_physics::sim_app::FixedCommands>() {
        Some(mut fixed) if fixed.bridged() => {
            if first {
                fixed.post_first_bridged(change);
            } else {
                fixed.post(change);
            }
        }
        _ => change(world),
    }
}

/// Moves a body's power list to another (the original shares the list
/// between the caster and its spirit), writing the status word `left` on
/// the body it leaves when given.
fn move_mind_list(world: &mut World, from: Entity, to: Entity, left: Option<u32>) {
    let list = match world.get_mut::<MindBody>(from) {
        Some(mut mind) => {
            if let Some(status) = left {
                mind.cast.status_bits = status;
            }
            std::mem::take(&mut mind.powers)
        }
        None => Vec::new(),
    };
    let n = list.len();
    match world.get_mut::<MindBody>(to) {
        Some(mut mind) => mind.powers = list,
        None => warn!("mind: {to:?} has no mind to take {from:?}'s {n} powers"),
    }
    info!("mind: {n} powers move from {from:?} to {to:?}");
}

/// Spirit Form ends: the caster's handover bit clears, the spirit screen
/// fades and the transition takes the player home.
fn spirit_ends(world: &mut World, spirit: Entity, body: Option<Entity>) {
    let body = body.or_else(|| world.get::<SpiritOf>(spirit).map(|s| s.body));
    let Some(body) = body else {
        return;
    };
    // The handover bit clears on the world that walks the powers.
    on_power_walk(world, false, move |world: &mut World| {
        set_status(world, body, |s| s & !HANDED_OVER);
        info!("spirit form: {body:?}'s handover bit clears");
    });
    let now = world
        .get_resource::<Time<Real>>()
        .map_or(0, |t| i64::from(crate::splash::clock_ms(t)));
    if let Some(screen) = world.resource_mut::<MindControl>().spirit_screen.as_mut() {
        screen.state = 2;
        screen.stamp = now;
    }
    // The screen's end plays `ThmMnd_E02` at 0.1 and fades the overlay
    // music out.
    let sound = casts::MindSound {
        name: crate::thaum::power::mind::SPIRIT_END_SOUND,
        volume: crate::thaum::power::mind::SPIRIT_END_VOLUME,
    };
    info!("mind sound: {} at volume {}", sound.name, sound.volume);
    world.write_message(exa_audio::PointSounded {
        name: sound.name.to_string(),
        position: Vec3::ZERO,
        volume: sound.volume,
        pitch: 1.0,
        flags: MIND_SOUND_FLAGS,
    });
    world.write_message(exa_audio::MusicOverlay::Out);
    info!("spirit form ends: the spirit screen fades and the spirit music fades out");
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
    if let Ok(mut entity) = world.get_entity_mut(spirit) {
        caster = entity.take::<PowerCaster>();
        entity.remove::<PlayerBody>();
    }
    if let Some(mut caster) = caster {
        if let Some(mind) = world.get::<MindBody>(body) {
            caster.body.status_bits = mind.cast.status_bits;
        }
        world.entity_mut(body).insert(caster);
    }
    // The list goes home before this frame's changes (the spirit's
    // despawn among them) reach the simulation.
    on_power_walk(world, true, move |world: &mut World| {
        move_mind_list(world, spirit, body, None);
    });
    // The caster's body takes the body's status word there too, after this
    // frame's changes (the caster's move home among them): the frame's
    // copy of the word may not have the end's cleared handover bit yet.
    on_power_walk(world, false, move |world: &mut World| {
        let status = world.get::<MindBody>(body).map(|m| m.cast.status_bits);
        if let (Some(status), Some(mut caster)) = (status, world.get_mut::<PowerCaster>(body)) {
            caster.body.status_bits = status;
        }
    });
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

/// The spirit screen's frame hook, on the real clock: the value charges,
/// holds or fades ([`step_spirit_form`]) and, while it charges or holds,
/// its eased value `1 − (1 − v)²` is the post-processing's desaturation
/// (the final pass's `Desat`: the scene goes grey toward its green
/// channel). A fading screen leaves the desaturation where the hold left
/// it; the screen's stop, at the way home's black, puts it back to 0.
///
/// The hook also writes two scene-environment values the original saved
/// when it built the screen (a colour scaled by `1 − s` and a value blended
/// toward 1000 by the strength), and uploads its 512² noise picture to a
/// texture the post processor holds; neither is drawn here (the fields are
/// not identified [guess: the fog's colour and span]; the final pass's
/// program declares the texture and never samples it).
fn step_spirit_screen(
    mut control: ResMut<MindControl>,
    real: Option<Res<Time<Real>>>,
    mut posts: Query<&mut exa_render::post::ExaPost>,
    mut shown: Local<bool>,
    mut last_log: Local<u32>,
) {
    let now = real.as_deref().map_or(0, crate::splash::clock_ms);
    let Some(screen) = control.spirit_screen.as_mut() else {
        if *shown {
            *shown = false;
            for mut post in &mut posts {
                post.desaturate = 0.0;
            }
            info!("spirit screen: stopped, desaturation 0");
        }
        return;
    };
    let step = step_spirit_form(screen, i64::from(now), 0.0);
    if let Some(desat) = step.screen {
        *shown = true;
        for mut post in &mut posts {
            post.desaturate = desat;
        }
    }
    if now >= *last_log + 250 {
        *last_log = now;
        info!(
            "spirit screen: state {} value {:.3} strength {:.1} desaturation {:?}",
            step.state, step.value, step.strength, step.screen
        );
    }
}

/// While the player is out of its own body, once a second of body ticks:
/// where the player's body and the held body stand (a trace for the runs
/// that check the swap).
#[allow(clippy::type_complexity)]
fn log_mind_bodies(
    control: Res<MindControl>,
    ticks: Option<Res<BodyTicks>>,
    players: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&exa_physics::body::Ragdoll>,
            Option<&Locomotion>,
        ),
        With<PlayerBody>,
    >,
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
    for (player, at, ragdoll, locomotion) in &players {
        // The first part (the pelvis) is where a physical body really is.
        let pelvis = ragdoll
            .and_then(|r| r.parts.first())
            .map(|p| p.transform().translation);
        // The lift's target pelvis height and the character's ground (m):
        // a hovering spirit aims at its clip height over the probe's ground.
        let drive = ragdoll.and_then(|r| r.drive.as_ref());
        info!(
            "mind: tick {now}: the player's body {player:?} at {:.2?} (pelvis {:.2?}, {} parts, moving {:?}, lift target {:.3?}, ground {:.3?}), the held body {held:?} at {:.2?}",
            at.translation(),
            pelvis,
            ragdoll.map_or(0, |r| r.parts.len()),
            locomotion.map(|l| l.moving),
            drive.and_then(|d| d.lift_target()),
            drive.map(|d| d.character_ground()),
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
        let resists_control = mind.cast.techniques & casts::RESISTS_CONTROL != 0;
        if current.is_none_or(|c| c.effects != effects || c.resists_control != resists_control) {
            commands.entity(entity).insert(MindPowers {
                effects,
                resists_control,
            });
        }
    }
}

/// Before the brains' tick: the enraged bit of each body's status word is
/// the contact integration's marked body (its contacts' hostility follows
/// the soothe term instead of the attitude).
fn feed_mind_status(mut brains: Query<(&MindBody, Option<&PowerCaster>, &mut Npc)>) {
    for (mind, caster, mut npc) in &mut brains {
        let status = caster.map_or(mind.cast.status_bits, |c| c.body.status_bits)
            | (mind.cast.status_bits & MIND_STATUS_BITS);
        let enraged = status & ENRAGED != 0;
        if npc.integration.marked_body != enraged {
            npc.integration.marked_body = enraged;
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

/// The control's end a cast while controlling another body asks for: the
/// held body's Control Mind, or else the Spirit Form of the spirit that
/// left it, is told to end. The two bodies are found on this world (the
/// frame's: the player's control is the frame's), and the end is posted to
/// the world that walks the powers, run at the start of its next step after
/// that step's copies in. A write on the frame's copy of the power list
/// could be overtaken there by a newer copy out of a free-running
/// simulation and the end lost. Answers whether a body was found.
pub fn end_control(world: &mut World) -> bool {
    let Some(held) = world.resource::<MindControl>().held else {
        info!("mind: the control's end finds no held body");
        return false;
    };
    let mut spirits = world.query::<(Entity, &SpiritOf)>();
    let spirit = spirits
        .iter(world)
        .find(|(_, of)| of.body == held)
        .map(|(e, _)| e);
    let run = move |world: &mut World| {
        end_control_of(world, held, spirit);
    };
    match world.get_resource_mut::<exa_physics::sim_app::FixedCommands>() {
        Some(mut fixed) => fixed.post(run),
        None => run(world),
    }
    true
}

/// Ends the held body's Control Mind, or else its spirit's Spirit Form.
pub fn end_control_of(world: &mut World, held: Entity, spirit: Option<Entity>) -> bool {
    if end_power(world, held, CONTROL_MIND) {
        info!("mind: {held:?}'s Control Mind is told to end");
        return true;
    }
    if let Some(spirit) = spirit
        && end_power(world, spirit, SPIRIT_FORM)
    {
        info!("mind: the Spirit Form of {spirit:?} (left {held:?}) is told to end");
        return true;
    }
    warn!(
        "mind: the control's end finds no Control Mind on {held:?} and no Spirit Form on {spirit:?}"
    );
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::thaum::power::mind::ControlMindCaster;

    fn world() -> World {
        let mut world = World::new();
        world.init_resource::<MindControl>();
        world.init_resource::<Messages<exa_audio::PointSounded>>();
        world.init_resource::<Messages<exa_audio::MusicOverlay>>();
        world
    }

    /// An app that runs the power walk at a body tick.
    fn walk_app() -> App {
        let mut app = App::new();
        app.init_resource::<MindControl>();
        app.add_message::<exa_audio::PointSounded>();
        app.add_systems(Update, tick_minds);
        app
    }

    fn walk_at(app: &mut App, ticks: u64) {
        app.world_mut().insert_resource(BodyTicks(ticks));
        app.update();
    }

    fn cast_scene(now_ms: u32) -> MindScene {
        MindScene {
            now_ms,
            real_ms: now_ms,
            ..MindScene::default()
        }
    }

    /// Casts a row from `caster` at `target` in the world.
    fn cast_at(world: &mut World, id: u32, caster: Entity, target: Entity, now_ms: u32) -> i32 {
        let row = *casts::mind_row(id).unwrap();
        let mut a = world.get::<MindBody>(caster).unwrap().clone();
        let mut b = world.get::<MindBody>(target).unwrap().clone();
        let outcome = casts::mind_cast(&row, row.cost, &mut a, Some(&mut b), &cast_scene(now_ms));
        *world.get_mut::<MindBody>(caster).unwrap() = a;
        *world.get_mut::<MindBody>(target).unwrap() = b;
        outcome.code
    }

    #[test]
    fn confusion_and_enrage_expire_after_fifteen_seconds() {
        let mut app = walk_app();
        let caster = character(app.world_mut(), true);
        let target = character(app.world_mut(), false);
        for id in [casts::CONFUSION, casts::ENRAGE] {
            assert_eq!(
                cast_at(app.world_mut(), id, caster, target, 0),
                casts::STARTED
            );
        }
        let bits = CONFUSED | ENRAGED;
        // 15 000 ms is not past; the first tick after clears both bits.
        walk_at(&mut app, 15_000 / 12);
        let mind = app.world().get::<MindBody>(target).unwrap();
        assert_eq!(mind.cast.status_bits & bits, bits);
        assert_eq!(mind.powers.len(), 2);
        walk_at(&mut app, 15_000 / 12 + 1);
        let mind = app.world().get::<MindBody>(target).unwrap();
        assert_eq!(mind.cast.status_bits & bits, 0);
        assert!(mind.powers.is_empty());
    }

    #[test]
    fn trust_and_suspicion_last_twenty_seconds() {
        let mut app = walk_app();
        let caster = character(app.world_mut(), true);
        let target = character(app.world_mut(), false);
        app.world_mut()
            .get_mut::<MindBody>(caster)
            .unwrap()
            .cast
            .focus = 1.0;
        assert_eq!(
            cast_at(app.world_mut(), casts::TRUST, caster, target, 0),
            casts::STARTED
        );
        assert_eq!(
            cast_at(app.world_mut(), casts::SUSPICION, caster, target, 0),
            casts::STARTED
        );
        let mind = app.world().get::<MindBody>(target).unwrap();
        // The perception reads the trust marker's caster as the trusted body.
        let trust = mind
            .powers
            .iter()
            .find(|p| p.id == casts::TRUST_MARK)
            .unwrap();
        assert_eq!(trust.target, Some(body_id(caster)));
        walk_at(&mut app, 20_000 / 12);
        assert_eq!(app.world().get::<MindBody>(target).unwrap().powers.len(), 2);
        walk_at(&mut app, 20_000 / 12 + 1);
        assert!(
            app.world()
                .get::<MindBody>(target)
                .unwrap()
                .powers
                .is_empty()
        );
    }

    #[test]
    fn the_instant_mask_holds_its_bit_fifteen_seconds() {
        let mut app = walk_app();
        let caster = character(app.world_mut(), true);
        let other = character(app.world_mut(), false);
        assert_eq!(
            cast_at(app.world_mut(), casts::MASK, caster, other, 0),
            casts::STARTED
        );
        walk_at(&mut app, 1);
        let mind = app.world().get::<MindBody>(caster).unwrap();
        assert_ne!(mind.cast.status_bits & MASKED, 0);
        walk_at(&mut app, 15_012 / 12 + 1);
        let mind = app.world().get::<MindBody>(caster).unwrap();
        assert_eq!(mind.cast.status_bits & MASKED, 0);
        assert!(mind.powers.is_empty());
    }

    #[test]
    fn the_instant_sense_fades_two_seconds_past_its_end() {
        let mut app = walk_app();
        let caster = character(app.world_mut(), true);
        let other = character(app.world_mut(), false);
        app.world_mut()
            .get_mut::<MindBody>(caster)
            .unwrap()
            .cast
            .focus = 1.0;
        assert_eq!(
            cast_at(app.world_mut(), casts::SENSE, caster, other, 0),
            casts::STARTED
        );
        // Ends at 4000 ms; 4 answers once, then 2 past 6000 ms.
        walk_at(&mut app, 4_008 / 12 + 1);
        assert_eq!(app.world().get::<MindBody>(caster).unwrap().powers.len(), 1);
        walk_at(&mut app, 6_000 / 12 + 1);
        assert!(
            app.world()
                .get::<MindBody>(caster)
                .unwrap()
                .powers
                .is_empty()
        );
    }

    #[test]
    fn loyalty_runs_until_ended_and_unlinks_both_halves() {
        let mut app = walk_app();
        let caster = character(app.world_mut(), true);
        let target = character(app.world_mut(), false);
        {
            let mut mind = app.world_mut().get_mut::<MindBody>(caster).unwrap();
            mind.cast.focus = 1.0;
            mind.cast.slots = 3;
        }
        assert_eq!(
            cast_at(app.world_mut(), casts::LOYALTY, caster, target, 0),
            casts::STARTED
        );
        for tick in 1..100 {
            walk_at(&mut app, tick);
        }
        // Running from the cast: no second marker is linked.
        let marks = |app: &App| {
            app.world()
                .get::<MindBody>(target)
                .unwrap()
                .powers
                .iter()
                .filter(|p| p.id == casts::LOYALTY_MARK)
                .count()
        };
        assert_eq!(marks(&app), 1);
        assert_eq!(app.world().get::<MindBody>(caster).unwrap().powers.len(), 1);
        // The marker's end ends the caster's power on the next walk.
        {
            let mut mind = app.world_mut().get_mut::<MindBody>(target).unwrap();
            mind.ai.rearm = false;
            mind.powers[0].end();
        }
        walk_at(&mut app, 100);
        walk_at(&mut app, 101);
        assert_eq!(marks(&app), 0);
        assert!(
            app.world()
                .get::<MindBody>(caster)
                .unwrap()
                .powers
                .is_empty()
        );
        assert!(app.world().get::<MindBody>(target).unwrap().ai.rearm);
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

    /// Behind the bridge the handover, the hand-back and the spirit's end
    /// write no mind on the frame: each posts its status words (and the
    /// handover its marker) to the world that walks the powers.
    #[test]
    fn behind_the_bridge_the_control_writes_its_minds_on_the_step() {
        let mut world = world();
        let mut fixed = exa_physics::sim_app::FixedCommands::default();
        fixed.set_bridged(true);
        world.insert_resource(fixed);
        let caster = character(&mut world, true);
        let target = character(&mut world, false);
        possess(&mut world, caster, target);
        assert!(world.get::<PlayerBody>(target).is_some());
        assert_eq!(
            (status_of(&world, caster), status_of(&world, target)),
            (0, 0)
        );
        assert!(world.get::<MindBody>(target).unwrap().powers.is_empty());
        let run_posts = |world: &mut World| {
            let posts = world
                .resource_mut::<exa_physics::sim_app::FixedCommands>()
                .take_posts();
            let n = posts.len();
            for post in posts {
                post(world);
            }
            n
        };
        assert_eq!(run_posts(&mut world), 1);
        assert_eq!(status_of(&world, caster), HANDED_OVER);
        assert_eq!(status_of(&world, target), IN_CONTROL);
        assert_eq!(
            world.get::<MindBody>(target).unwrap().powers[0].id,
            POSSESSION_MARK
        );
        release(&mut world, caster, target);
        assert!(world.get::<PlayerBody>(caster).is_some());
        assert_eq!(status_of(&world, caster), HANDED_OVER);
        assert_eq!(run_posts(&mut world), 1);
        assert_eq!(
            (status_of(&world, caster), status_of(&world, target)),
            (0, HANDED_OVER)
        );

        let spirit = character(&mut world, false);
        set_status(&mut world, caster, |s| s | HANDED_OVER);
        spirit_ends(&mut world, spirit, Some(caster));
        assert_eq!(status_of(&world, caster), HANDED_OVER);
        assert_eq!(run_posts(&mut world), 1);
        assert_eq!(status_of(&world, caster), 0);
    }

    /// A Mind cast's changes run on the live brain: its values are read off
    /// the brain, changed as the cast changes them and written back.
    #[test]
    fn a_casts_changes_reach_the_live_brain() {
        let mut world = world();
        let caster = character(&mut world, true);
        let target = character(&mut world, false);
        let mut npc = Npc::new([0.0; 4]);
        npc.contacts.push(exa_ai::contacts::ContactRecord {
            id: caster.to_bits(),
            concern: 12.0,
            hostility: 4.0,
            noticed: 1.0,
            ..Default::default()
        });
        npc.morale.panic = 5.0;
        npc.morale.alarm = 0.5;
        npc.integration.readiness = 60.0;
        npc.integration.level = 3;
        let combatant = Combatant {
            target: Some(caster),
            ..Combatant::default()
        };
        world.entity_mut(target).insert((npc, combatant));
        let run = |world: &mut World, changes: Vec<casts::MindAiChange>| {
            world
                .run_system_cached_with(drive_mind_ai, (target, changes))
                .unwrap();
        };
        run(&mut world, vec![casts::MindAiChange::Calm]);
        let npc = world.get::<Npc>(target).unwrap();
        let c = npc.contacts[0];
        // 12 concern loses 10 and gives 30; 4 hostility all goes, 12.
        assert_eq!((c.concern, c.hostility, c.noticed), (2.0, 0.0, 43.0));
        assert_eq!(
            (
                npc.morale.panic,
                npc.integration.readiness,
                npc.morale.alarm
            ),
            (0.0, 10.0, 0.0)
        );
        assert_eq!(npc.integration.decay, 1.0);
        run(
            &mut world,
            vec![
                casts::MindAiChange::Fear,
                casts::MindAiChange::Embolden,
                casts::MindAiChange::Bind {
                    caster: body_id(caster),
                },
                casts::MindAiChange::Rearm,
            ],
        );
        let npc = world.get::<Npc>(target).unwrap();
        assert_eq!(npc.morale.alarm, 20.0);
        assert_eq!(npc.morale.courage, 50.5);
        assert_eq!(
            (npc.contacts[0].concern, npc.contacts[0].noticed),
            (0.0, 30.0)
        );
        assert!(npc.reapply);
        run(&mut world, vec![casts::MindAiChange::Confusion]);
        let npc = world.get::<Npc>(target).unwrap();
        assert_eq!((npc.morale.panic, npc.integration.level), (50.0, 0));
        assert_eq!(npc.contacts[0].noticed, 0.0);
        assert_eq!(world.get::<Combatant>(target).unwrap().target, None);
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
        app.add_message::<exa_audio::PointSounded>();
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

    /// A cast while in the spirit asks the control's end: it is posted to
    /// the world that walks the powers (behind the bridge the
    /// simulation's), not written on this world's copy, and ends the
    /// spirit's Spirit Form when it runs.
    #[test]
    fn the_control_end_is_posted_to_the_power_walk() {
        let mut world = world();
        world.init_resource::<exa_physics::sim_app::FixedCommands>();
        let body = character(&mut world, true);
        let spirit = character(&mut world, false);
        world.entity_mut(spirit).insert(SpiritOf { body });
        let entry = MindPower::new(
            SPIRIT_FORM,
            MindClass::Spirit(crate::thaum::power::mind::SpiritPower::new(0.001)),
        );
        world
            .get_mut::<MindBody>(spirit)
            .unwrap()
            .powers
            .push(entry);
        world.resource_mut::<MindControl>().held = Some(body);
        assert!(end_control(&mut world));
        let state = |world: &World| world.get::<MindBody>(spirit).unwrap().powers[0].state();
        assert_ne!(state(&world), ENDING);
        let posts = world
            .resource_mut::<exa_physics::sim_app::FixedCommands>()
            .take_posts();
        assert_eq!(posts.len(), 1);
        for post in posts {
            post(&mut world);
        }
        assert_eq!(state(&world), ENDING);
    }

    /// Behind the bridge the spirit's list moves on the simulation's world:
    /// the frame's adoption leaves both lists as they are and posts the move
    /// after its changes; the way home posts the move back before them.
    #[test]
    fn behind_the_bridge_the_spirit_list_moves_on_the_step() {
        let mut world = world();
        let mut fixed = exa_physics::sim_app::FixedCommands::default();
        fixed.set_bridged(true);
        world.insert_resource(fixed);
        let body = character(&mut world, true);
        let spirit = character(&mut world, false);
        world.entity_mut(spirit).insert(SpiritOf { body });
        let entry = MindPower::new(
            SPIRIT_FORM,
            MindClass::Spirit(crate::thaum::power::mind::SpiritPower::new(0.001)),
        );
        world.get_mut::<MindBody>(body).unwrap().powers.push(entry);
        let mut caster = PowerCaster::new(&[SPIRIT_FORM]);
        caster.body.status_bits = HANDED_OVER;
        world.entity_mut(body).insert(caster);
        adopt_spirit(&mut world, body, spirit);
        let count = |world: &World, e: Entity| world.get::<MindBody>(e).unwrap().powers.len();
        assert_eq!((count(&world, body), count(&world, spirit)), (1, 0));
        let mut fixed = world.resource_mut::<exa_physics::sim_app::FixedCommands>();
        let (writes, posts) = (fixed.take_writes(), fixed.take_posts());
        assert_eq!((writes.len(), posts.len()), (0, 1));
        for post in posts {
            post(&mut world);
        }
        assert_eq!((count(&world, body), count(&world, spirit)), (0, 1));
        assert_eq!(status_of(&world, body), HANDED_OVER);
        spirit_home(&mut world, spirit, body);
        let mut fixed = world.resource_mut::<exa_physics::sim_app::FixedCommands>();
        let (writes, posts) = (fixed.take_writes(), fixed.take_posts());
        // The list's move before the frame's changes, the caster's status
        // word after them.
        assert_eq!((writes.len(), posts.len()), (1, 1));
        // The move runs before the spirit's despawn reaches the
        // simulation; the frame's copies are left alone.
        assert_eq!(count(&world, body), 0);
        drop(writes);
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
        // The wraith hovers: combat mode cannot be turned on.
        {
            let mut player = world.get_mut::<PlayerBody>(spirit).unwrap();
            assert!(!player.toggle_combat());
            assert!(!player.set_combat(true));
            assert!(!player.combat);
        }
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

//! The posture stage of a character's tick, in the original's order: the
//! condition (`exa_combat::condition::track_condition`), the collapse's
//! dying tick (`exa_combat::collapse::dying_tick`), the falls, staggers and
//! get-ups (`crate::balance::take_posture`), the fall check
//! (`crate::fall::fall_to_death`); then what the body does this tick, which
//! its muscles and balance read ([`Bearing`]): moving and running,
//! striding, off balance while its posture stage takes an off-balance
//! branch, getting up and rising through its get-up's first quarter,
//! carried by its controller, turning unless its condition is helpless,
//! collapsing, kicking, casting, switching weapons, walking and off its
//! feet by its condition, its clip's progress, and whether a shield is in
//! the off hand. The attack stage copies the parry flag onto that bearing
//! after the parry check.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::Condition;
use exa_combat::attack::{self, Attacker};
use exa_combat::condition::{PostureBranch, StateFlags};
use exa_physics::CharacterBody;
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_physics::body::{Bearing, Ragdoll, TickedCharacter};

use crate::limp::{bearing, rising};
use crate::player::PlayerBody;

/// Tells the ticked body what its character does this tick (its gait floor
/// is the movement stage's).
#[allow(clippy::type_complexity)]
pub(crate) fn take_bearing(
    ticked: Res<TickedCharacter>,
    motions: Option<Res<Motions>>,
    controllers: Query<&CharacterBody>,
    mut bodies: Query<(
        &mut Ragdoll,
        &Animator,
        Option<(&Locomotion, &Locomotor)>,
        Option<&PlayerBody>,
        Option<&PostureBranch>,
        Option<&Condition>,
        Option<&StateFlags>,
        Option<&Attacker>,
    )>,
) {
    let Some(motions) = motions else { return };
    let Ok((mut body, anim, locomotion, player, branch, condition, flags, attacker)) =
        bodies.get_mut(ticked.body)
    else {
        return;
    };
    let controller = ticked.character.and_then(|c| controllers.get(c).ok());
    let state = condition.copied().unwrap_or_default();
    let branch = branch.map_or_else(Default::default, |b| b.0);
    let crouch = player.map_or(0.0, PlayerBody::crouch);
    let mut bearing: Bearing = bearing(controller, locomotion, crouch, anim, branch, state);
    bearing.rising = rising(&motions, anim, state);
    // Dormant, or in exactly the shock's stagger, its stance factor is 1.
    // The flag's bit, with whatever else is set, makes the pins' power 1.
    bearing.full_stance =
        state == Condition::DORMANT || flags.map_or(0, |f| f.0) == StateFlags::SHOCK_STAGGER;
    bearing.shock_stagger = flags.is_some_and(|f| f.has(StateFlags::SHOCK_STAGGER));
    bearing.shield = attacker.is_some_and(|a| a.stance_class == attack::STANCE_SHIELD);
    bearing.progress = motions
        .clip(anim, &anim.action)
        .map_or(0.0, |c| anim.clip_progress(c));
    if let Some(drive) = body.drive.as_mut() {
        drive.dormant = state == Condition::DORMANT;
    }
    body.set_bearing(bearing);
}

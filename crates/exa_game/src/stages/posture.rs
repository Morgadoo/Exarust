//! The posture stage of a character's tick, in the original's order: the
//! condition (`exa_combat::condition::track_condition`), the collapse's
//! dying tick (`exa_combat::collapse::dying_tick`), the falls, staggers and
//! get-ups (`crate::balance::take_posture`), the fall check
//! (`crate::fall::fall_to_death`); then what the body does this tick, which
//! its muscles and balance read ([`Bearing`]): moving and running,
//! striding, off balance while its posture stage takes an off-balance
//! branch, getting up and rising through its get-up's first quarter,
//! carried by its controller, turning unless its condition is helpless,
//! collapsing.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::Condition;
use exa_combat::condition::PostureBranch;
use exa_physics::CharacterBody;
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
        Option<&PlayerBody>,
        Option<&PostureBranch>,
        Option<&Condition>,
    )>,
) {
    let Some(motions) = motions else { return };
    let Ok((mut body, anim, player, branch, condition)) = bodies.get_mut(ticked.body) else {
        return;
    };
    let controller = ticked.character.and_then(|c| controllers.get(c).ok());
    let state = condition.copied().unwrap_or_default();
    let branch = branch.map_or_else(Default::default, |b| b.0);
    let mut bearing: Bearing = bearing(controller, player, anim, branch, state);
    bearing.rising = rising(&motions, anim, state);
    body.set_bearing(bearing);
}

//! The posture stage of a character's tick: what the body's character does
//! this tick, which its muscles and balance read ([`Bearing`]): moving and
//! running, striding, off balance from its fall until it has got up,
//! getting up and rising through its get-up's first quarter, carried by its
//! controller, turning unless its condition is helpless, collapsing. The
//! original decides its condition, falls and get-ups here; Exarust decides
//! the condition and the collapse's dying tick
//! (`exa_combat::condition::PostureTick`) and then the falls (`balance`)
//! over every body before the character ticks.

use bevy::prelude::*;
use exa_anim::{Animator, Motions};
use exa_combat::Condition;
use exa_physics::CharacterBody;
use exa_physics::body::{Fallen, Ragdoll, TickedCharacter};

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
        Option<&Fallen>,
        Option<&Condition>,
    )>,
) {
    let Some(motions) = motions else { return };
    let Ok((mut body, anim, player, fallen, condition)) = bodies.get_mut(ticked.body) else {
        return;
    };
    let controller = ticked.character.and_then(|c| controllers.get(c).ok());
    let state = condition.copied().unwrap_or_default();
    let mut bearing = bearing(controller, player, anim, fallen, state);
    bearing.rising = rising(&motions, anim, fallen);
    body.set_bearing(bearing);
}

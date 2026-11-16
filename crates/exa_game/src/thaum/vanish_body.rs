//! The vanishing body: what Vanish and the Long Door do to the body they
//! take away and bring back, and the tendril image they leave on it.
//!
//! The original's two instant Displacement classes run the shared vanish
//! phases ([`super::power::displacement::step_vanish`]); at three of their
//! turns they change the body itself (simulation side) and its tendril
//! image (drawn):
//!
//! - **The vanish** (the charge's end, as the fade-out starts): the body's
//!   state flags gain the vanished bit ([`StateFlags::VANISHED`]); the
//!   bodies from the character's node down are disabled (the original's
//!   state bit 0x800: the body-to-body stage leaves them out, so nothing
//!   meets the vanished body or the items it holds, and a bite stuck in it
//!   breaks on its next pass); the living body drops its first breakable
//!   grab that still holds ([`exa_combat::bite::BiteHold::drop_first_grab`]).
//!   Vanish sets the bit before the drop, the Long Door after it.
//! - **The come-back** (the hold's end): the bodies are enabled again; the
//!   living body also loses its kinematic bit and settles for 1000 ticks
//!   (gravity without inertia, [`Ragdoll::settle_for`]); then the body is
//!   placed ([`super::cast::Otherwhere`]).
//! - **The show** (the fade-in's end): the vanished bit clears and the
//!   living body's settling ends.
//!
//! The tendril image ([`ImageStep`]): the vanish start puts one on a
//! player's body (after the shell's two draws); the fade-out's end freezes
//! and fades it; the come-back (and an AI door's found place) puts up a new
//! one, which the fade-in's end fades out. Each image is the body's wisps
//! ([`super::cast::spawn_body_wisps`]), sized 12 times the body's size.
//!
//! In Exarust the character's node tree is its own body and the items its
//! hands hold ([`HeldGrip`]); the living body is the character's. The
//! original queues the new image's creation to the next frame; Exarust
//! makes it in the same step, after the casters' walk. Over a shell lent
//! by a running Constant Displacement the original also takes the
//! constant's image, ends it and hands the new one to the constant; Exarust
//! leaves the constant's image as it is and makes none for the vanish.
//!
//! Evidence: the two classes' control routines, the vanish start, its
//! after-image callback, the image routine, the node-tree disable with the
//! grab drop, the enable with the settling, and the settling's clear, read
//! from `Exanima.exe`.

use bevy::prelude::*;
use exa_combat::StateFlags;
use exa_combat::bite::BiteHold;
use exa_physics::body::{Ragdoll, SolverStages};
use exa_physics::grip::HeldGrip;

use super::cast::{ActiveClass, PowerCaster, spawn_body_wisps};
use super::live::{EffectDraws, EffectRoll, LiveWisps};
use super::power::displacement::{ImageStep, LONG_DOOR, VANISH, VanishStep};

/// Ticks a returning body settles for (gravity without inertia): the
/// enable's stamp.
pub const RETURN_SETTLE: i32 = 1000;

/// What a vanish turn does to the body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyTurn {
    Keep,
    /// The fade-out starts: the vanished bit, the disable, the grab drop.
    Vanish,
    /// The hold ends: the enable and the settling.
    Return,
    /// The fade-in ends: the vanished bit and the settling clear.
    Shown,
}

/// A turn of a Vanish or a Long Door that changes its body or its image,
/// written by the casters' walk.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct VanishTurn {
    pub caster: Entity,
    /// The power id of the class ([`VANISH`] or [`LONG_DOOR`]).
    pub power: u32,
    pub body: BodyTurn,
    pub image: ImageStep,
    /// The shell is lent by a running Constant Displacement.
    pub shared: bool,
}

impl VanishTurn {
    /// The turn a step makes, if it makes one.
    pub fn of(caster: Entity, power: u32, step: &VanishStep, shared: bool) -> Option<Self> {
        let body = if step.hide {
            BodyTurn::Vanish
        } else if step.moved {
            BodyTurn::Return
        } else if step.show {
            BodyTurn::Shown
        } else {
            BodyTurn::Keep
        };
        (body != BodyTurn::Keep || step.image != ImageStep::Keep).then_some(Self {
            caster,
            power,
            body,
            image: step.image,
            shared,
        })
    }
}

/// Registers what the turns' fixed-step system touches.
pub(crate) fn bridge_vanish_body_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.add_message::<VanishTurn>()
        .bridge_copy::<StateFlags>()
        .bridge_copy::<HeldGrip>();
}

/// The class's tendril image slot.
fn image_slot(caster: &mut PowerCaster, power: u32) -> Option<&mut Option<u64>> {
    caster.active.iter_mut().find_map(|entry| match &mut entry.class {
        ActiveClass::Vanish(p) if power == VANISH => Some(&mut p.node.image),
        ActiveClass::LongDoor(p) if power == LONG_DOOR => Some(&mut p.node.image),
        _ => None,
    })
}

/// Each body step, after the casters' walk and before the placements:
/// carries out the vanish turns on the bodies and their images.
#[allow(clippy::too_many_arguments)]
pub(crate) fn turn_vanishing_bodies(
    mut commands: Commands,
    mut turns: MessageReader<VanishTurn>,
    mut casters: Query<&mut PowerCaster>,
    mut bodies: Query<(Entity, &mut Ragdoll, Option<&HeldGrip>)>,
    mut flags: Query<&mut StateFlags>,
    mut images: Query<&mut LiveWisps>,
    mut stages: Option<ResMut<SolverStages>>,
    mut draws: ResMut<EffectDraws>,
    mut roll: ResMut<EffectRoll>,
) {
    for turn in turns.read().copied() {
        turn_body(&turn, &mut bodies, &mut flags, stages.as_deref_mut());
        if turn.shared || turn.image == ImageStep::Keep {
            continue;
        }
        let Ok(mut caster) = casters.get_mut(turn.caster) else {
            continue;
        };
        let Some(slot) = image_slot(&mut caster, turn.power) else {
            continue;
        };
        match turn.image {
            ImageStep::End | ImageStep::Fade => {
                let Some(old) = slot.take().and_then(Entity::try_from_bits) else {
                    continue;
                };
                if let Ok(mut live) = images.get_mut(old) {
                    if turn.image == ImageStep::End {
                        live.wisps.end();
                    } else {
                        live.wisps.fade_out();
                    }
                }
                debug!(
                    "vanish of {:?}: its tendril image {old:?} {}",
                    turn.caster,
                    if turn.image == ImageStep::End {
                        "freezes and fades"
                    } else {
                        "fades"
                    }
                );
            }
            ImageStep::New => {
                let Ok((_, body, _)) = bodies.get(turn.caster) else {
                    continue;
                };
                let image = spawn_body_wisps(&mut commands, turn.caster, body, &mut draws, &mut roll);
                *slot = Some(image.to_bits());
                debug!("vanish of {:?}: a new tendril image {image:?}", turn.caster);
            }
            ImageStep::Lend | ImageStep::Keep => {}
        }
    }
}

/// The turn's change to the body and the items it holds.
fn turn_body(
    turn: &VanishTurn,
    bodies: &mut Query<(Entity, &mut Ragdoll, Option<&HeldGrip>)>,
    flags: &mut Query<&mut StateFlags>,
    stages: Option<&mut SolverStages>,
) {
    if turn.body == BodyTurn::Keep {
        return;
    }
    let caster = turn.caster;
    let Some(key) = bodies
        .get(caster)
        .ok()
        .and_then(|(_, body, _)| body.parts.first().map(|p| p.entity))
    else {
        return;
    };
    let mut items = 0;
    for (entity, mut body, grip) in bodies.iter_mut() {
        let held = grip.is_some_and(|g| g.holder_key == key && !g.released);
        if entity != caster && !held {
            continue;
        }
        let living = entity == caster;
        match turn.body {
            BodyTurn::Vanish => body.set_disabled(true),
            BodyTurn::Return => {
                body.set_disabled(false);
                if living {
                    body.set_kinematic(false);
                    body.settle_for(RETURN_SETTLE);
                }
            }
            BodyTurn::Shown => {
                if living {
                    body.settle_for(0);
                }
            }
            BodyTurn::Keep => {}
        }
        items += usize::from(!living);
    }
    match turn.body {
        BodyTurn::Vanish => {
            if let Ok(mut word) = flags.get_mut(caster) {
                word.0 |= StateFlags::VANISHED;
            }
            let dropped = stages
                .and_then(|s| s.get_mut::<BiteHold>())
                .and_then(|hold| {
                    let mut all: Vec<_> = bodies.iter_mut().collect();
                    let at = all.iter().position(|(e, ..)| *e == caster)?;
                    let mut refs: Vec<&mut Ragdoll> =
                        all.iter_mut().map(|(_, b, _)| &mut **b).collect();
                    hold.drop_first_grab(&mut refs, at)
                });
            info!(
                "vanish of {caster:?}: vanished, its body and {items} held item(s) disabled{}",
                if dropped.is_some() {
                    ", its first holding grab dropped"
                } else {
                    ""
                }
            );
        }
        BodyTurn::Return => info!(
            "vanish of {caster:?}: enabled with {items} held item(s), settling for \
             {RETURN_SETTLE} ticks"
        ),
        BodyTurn::Shown => {
            if let Ok(mut word) = flags.get_mut(caster) {
                word.0 &= !StateFlags::VANISHED;
            }
            info!("vanish of {caster:?}: shown, its settling ends");
        }
        BodyTurn::Keep => {}
    }
}

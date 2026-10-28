//! Held items as solver bodies: the cross-body stage that pulls an item's
//! body to the hand, the second hand and the arm that hold it.
//!
//! The original keeps a held item as its own rigid body whose parent is the
//! hand body, and the grip's link records live on the item. Its link
//! iteration runs those records every iteration, pushing the item and the
//! hand by their mobility shares. Exarust's item body is a separate
//! [`Ragdoll`], so this stage is the cross-body equivalent: for each
//! [`HeldGrip`] it finds the item's body and the character's body, then
//! applies every enabled link with the limp rigid-body link law (a pivot
//! closes `strength/(1 + 0.001·|d|²)` of its gap, in cm²; a slack link
//! takes back `strength` times the excess beyond its slack), the item and
//! the hand each moving by their share of the mobility sum. A link with no
//! hand part (a world weld) moves the item alone.
//!
//! The item body never meets its holder's body (nor another item the same
//! character holds): that is the original's all-ones group mask on held
//! items, which makes the pair stage see them as one articulated tree.
//!
//! Evidence: the original's grip builder and link iteration, and
//! [`crate::grip`]'s records.

use bevy::prelude::*;

use crate::body::{BodySystems, Ragdoll, SolverStage, SolverStages};
use crate::grip::{self, DISABLED, GripLinks, GripRelease, HeldGrip, HeldKind};

/// The grip stage's order: the builder and the per-tick sync. The game's
/// grip-mode and flip systems run before [`GripSolveSet::Sync`], so this
/// tick's solver sees their links.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GripSolveSet {
    Build,
    Sync,
}

/// A pivot's softening per cm².
const SOFTENING: f32 = 0.001;

/// One held item's links, as the solver stage reads them.
#[derive(Debug, Clone)]
pub struct GripGroup {
    /// The item body's root part (the entity carrying [`HeldGrip`]).
    pub item: Entity,
    /// A part of the character's body (its holding hand), to find it by.
    pub probe: Entity,
    pub links: GripLinks,
}

/// The cross-body grip stage. [`sync_grips`] fills it before each tick's
/// solver from the [`HeldGrip`] components.
#[derive(Resource, Debug, Default)]
pub struct GripSolver {
    pub groups: Vec<GripGroup>,
}

/// The body of `probe`'s part, and the part's index.
fn find_part(bodies: &[&mut Ragdoll], probe: Entity) -> Option<(usize, usize)> {
    bodies.iter().enumerate().find_map(|(b, body)| {
        body.parts
            .iter()
            .position(|p| p.entity == probe)
            .map(|p| (b, p))
    })
}

/// The body whose first part is `key` (the item's root part).
fn find_body(bodies: &[&mut Ragdoll], key: Entity) -> Option<usize> {
    bodies
        .iter()
        .position(|b| b.parts.first().is_some_and(|p| p.entity == key))
}

/// The correction of one grip link and the two shares, from the item's and
/// the character's bodies as they stand.
fn link_correction(
    item: &Ragdoll,
    holder: &Ragdoll,
    link: &mut crate::grip::GripLink,
) -> Option<(Vec3, f32, f32)> {
    let a = item.point_at(link.item, &link.on_item)?;
    let b = match link.hand {
        Some(part) => holder.point_at(part, &link.on_hand)?,
        None => link.anchor?,
    };
    // The limp rigid body's own link law, with a pivot's 0.001 softening.
    let correction = crate::body::limp_law(
        b - a,
        link.slack,
        link.strength,
        SOFTENING,
        link.smoothing,
        &mut link.length,
    )?;
    correction
        .is_finite()
        .then_some((correction, link.shares.0, link.shares.1))
}

impl SolverStage for GripSolver {
    fn links(&mut self, bodies: &mut [&mut Ragdoll], _iteration: usize) {
        for group in &mut self.groups {
            let Some(item_body) = find_body(bodies, group.item) else {
                continue;
            };
            let Some((holder_body, _)) = find_part(bodies, group.probe) else {
                continue;
            };
            if item_body == holder_body {
                continue;
            }
            for link in &mut group.links.links {
                if link.flags & DISABLED != 0 {
                    continue;
                }
                let Some((correction, item_share, holder_share)) =
                    link_correction(bodies[item_body], bodies[holder_body], link)
                else {
                    continue;
                };
                bodies[item_body].push_weights(link.item, &link.on_item, correction * item_share);
                if let Some(part) = link.hand {
                    bodies[holder_body].push_weights(
                        part,
                        &link.on_hand,
                        -correction * holder_share,
                    );
                }
            }
        }
    }

    fn coupled(&self, bodies: &[&mut Ragdoll], coupled: &mut [bool]) {
        for group in &self.groups {
            if let Some(i) = find_body(bodies, group.item) {
                coupled[i] = true;
            }
            if let Some((i, _)) = find_part(bodies, group.probe) {
                coupled[i] = true;
            }
        }
    }
}

/// Builds the grip links of every [`HeldGrip`] whose item and character
/// bodies both exist, in the original's builder. It runs before the tick's
/// solver; [`mark_held`] then wakes the item's body.
pub fn build_grips(mut held: Query<(Entity, &mut HeldGrip)>, bodies: Query<&Ragdoll>) {
    for (entity, mut grip) in &mut held {
        if grip.links.is_some() || grip.released {
            continue;
        }
        let Ok(item) = bodies.get(entity) else {
            continue;
        };
        // Find the character's body by one of the hand or arm parts.
        let probe = grip
            .hands
            .iter()
            .chain(grip.arms.iter())
            .flatten()
            .next()
            .copied();
        let Some(probe) = probe else {
            continue;
        };
        let Some(holder) = bodies
            .iter()
            .find(|b| b.parts.iter().any(|p| p.entity == probe))
        else {
            continue;
        };
        let part_of = |e: Entity| holder.parts.iter().position(|p| p.entity == e);
        let holding = grip.spec.hand.min(1);
        let other = 1 - holding;
        let Some(hold_index) = grip.hands[holding].and_then(part_of) else {
            continue;
        };
        // A grip that needs a second hand or an arm refuses to build without
        // one (the original reports an invalid configuration).
        let other_index = if grip.spec.second_hand.is_some() || grip.spec.extra.is_some() {
            match grip.hands[other].and_then(part_of) {
                Some(i) => i,
                None => continue,
            }
        } else {
            grip.hands[other].and_then(part_of).unwrap_or(hold_index)
        };
        let arm_index = if grip.spec.arm.is_some() {
            match grip.arms[holding].and_then(part_of) {
                Some(i) => i,
                None => continue,
            }
        } else {
            hold_index
        };
        let mut hands = [hold_index, hold_index];
        let mut arms = [arm_index, arm_index];
        hands[holding] = hold_index;
        hands[other] = other_index;
        arms[holding] = arm_index;
        if let Some(i) = grip.arms[other].and_then(part_of) {
            arms[other] = i;
        }
        let Some(item_part) = item.parts.get(HeldGrip::ITEM_PART) else {
            continue;
        };
        let Some(hold_part) = holder.parts.get(hold_index) else {
            continue;
        };
        let item_to_hand = hold_part.transform().inverse() * item_part.transform();
        let bindings = grip::GripBindings {
            item_part: HeldGrip::ITEM_PART,
            hand_parts: hands,
            arm_parts: arms,
        };
        let built = match grip.kind {
            // The torch's own two pivots and its soft weld, from its attach
            // routine rather than the grip table.
            HeldKind::Torch => grip::torch_links(item, holder, bindings, item_to_hand),
            HeldKind::Shield => {
                // The original subtracts the other hand's stored rest
                // translation from the two face points, after the item
                // node's turn. [guess: the part's origin stands in]
                let other = 1 - holding;
                let offset = holder
                    .parts
                    .get(hands[other])
                    .map_or(Vec3::ZERO, |p| p.rest_offset() * 100.0);
                grip::shield_links(item, holder, bindings, holding, item_to_hand, offset)
            }
            HeldKind::Weapon => grip::build(
                item,
                holder,
                bindings,
                &grip.spec,
                grip.scale,
                grip.size,
                item_to_hand,
                grip.flip,
                grip.second_hand_factor,
            ),
        };
        if let Some(mut links) = built {
            // The world welds a held weapon gets, both built disabled: a
            // stiff one at its origin, and, when its holder may parry, the
            // soft (parry) weld 0.3 of the node box's top above the grip,
            // at the grip itself for a two-hander of the exact class 0x30.
            // A shield's and a torch's builders have already added their
            // own soft weld.
            if grip.kind == HeldKind::Weapon {
                let top = if grip.class == 0x30 {
                    0.0
                } else {
                    item.parts.first().map_or(0.0, |p| {
                        let box_top = p.pairing.mesh.centre.y + p.pairing.mesh.half.y;
                        (f64::from(box_top * 100.0) * 0.3) as f32
                    })
                };
                let soft = Vec3::new(0.0, top, 0.0);
                if let Some(welds) = grip::world_links(item, bindings, soft, grip.soft_weld) {
                    for weld in welds {
                        if weld.strength < 1.0 {
                            links.world_soft = Some(links.links.len());
                        } else {
                            links.world_hard = Some(links.links.len());
                        }
                        links.links.push(weld);
                    }
                }
            }
            grip.links = Some(links);
        }
    }
}

/// Marks every item with built links as held: it never meets its holder's
/// body, it is a held part (class bit 2 for the contact classifier), and it
/// stays awake so the grip stage can pull it.
pub fn mark_held(mut held: Query<(&HeldGrip, &mut Ragdoll)>) {
    for (grip, mut body) in &mut held {
        if grip.links.is_none() {
            continue;
        }
        body.held_by = Some(grip.holder_key);
        body.set_held_group();
        for part in &mut body.parts {
            part.state = part.state.with(crate::part::PartState::HELD);
            // A held item's friction is the character constructor's 0.5.
            part.friction = 0.5;
        }
        body.wake_to(crate::body::WAKE_COUNT);
    }
}

/// Releases the items whose holder has left the world, and those a
/// collapse let go of ([`GripRelease`]): the link records stop acting, the
/// item body loses its holder and held mask, takes the dropped weapon's
/// friction and falls free. A holder that only went limp
/// or died keeps its item: the original welds a held item to a dead body
/// ([`GripLinks::weld_on_death`]) instead of dropping it, and its collapse
/// is the routine that lets the hands go.
pub fn release_freed_grips(
    mut commands: Commands,
    mut held: Query<(Entity, &mut HeldGrip, &mut Ragdoll, Has<GripRelease>)>,
    bodies: Query<&Ragdoll, Without<HeldGrip>>,
) {
    for (entity, mut grip, mut body, released) in &mut held {
        if grip.released || grip.links.is_none() {
            continue;
        }
        let owner = bodies
            .iter()
            .find(|b| b.parts.first().is_some_and(|p| p.entity == grip.holder_key));
        if owner.is_some() && !released {
            continue;
        }
        grip.released = true;
        grip.links = None;
        body.held_by = None;
        body.reset_groups();
        for part in &mut body.parts {
            part.state = crate::part::PartState(0);
            part.friction = 0.75;
        }
        body.wake_to(crate::body::WAKE_COUNT);
        commands
            .entity(entity)
            .remove::<crate::grip::HeldGrip>()
            .remove::<crate::grip::GripRelease>()
            .remove::<ChildOf>();
    }
}

/// Copies the built links into the solver stage for this tick.
pub fn sync_grips(held: Query<(Entity, &HeldGrip)>, mut stages: ResMut<SolverStages>) {
    let mut groups = Vec::new();
    for (entity, grip) in &held {
        let Some(links) = &grip.links else {
            continue;
        };
        let probe = grip
            .hands
            .iter()
            .chain(grip.arms.iter())
            .flatten()
            .next()
            .copied();
        let Some(probe) = probe else {
            continue;
        };
        groups.push(GripGroup {
            item: entity,
            probe,
            links: links.clone(),
        });
    }
    if let Some(solver) = stages.get_mut::<GripSolver>() {
        solver.groups = groups;
    }
}

/// Keeps each grip link's smoothed length after the tick's solver: the
/// stage works on a copy of the links, and a slack link's damper must carry
/// its length from tick to tick as the original's link records do.
pub fn store_grip_lengths(mut held: Query<&mut HeldGrip>, stages: Res<SolverStages>) {
    let Some(solver) = stages.get::<GripSolver>() else {
        return;
    };
    for group in &solver.groups {
        let Ok(mut grip) = held.get_mut(group.item) else {
            continue;
        };
        let Some(links) = grip.links.as_mut() else {
            continue;
        };
        for (kept, solved) in links.links.iter_mut().zip(&group.links.links) {
            kept.length = solved.length;
        }
    }
}

/// Installs the grip stage into the solver's stages.
pub fn install_stage(mut stages: ResMut<SolverStages>) {
    if stages.get::<GripSolver>().is_none() {
        stages.add(GripSolver::default());
    }
}

/// Adds the grip stage and its per-tick sync. Called by
/// [`crate::PhysicsPlugin`].
pub fn plugin(app: &mut App) {
    app.add_systems(Startup, install_stage)
        .configure_sets(
            FixedUpdate,
            GripSolveSet::Build
                .before(GripSolveSet::Sync)
                .before(BodySystems),
        )
        .add_systems(
            FixedUpdate,
            (build_grips, release_freed_grips, mark_held)
                .chain()
                .in_set(GripSolveSet::Build),
        )
        .add_systems(FixedUpdate, sync_grips.in_set(GripSolveSet::Sync))
        .add_systems(FixedUpdate, store_grip_lengths.after(BodySystems));
}

#[cfg(test)]
#[path = "grip_solve_test.rs"]
mod tests;

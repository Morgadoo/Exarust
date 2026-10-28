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
use crate::grip::{self, DISABLED, GripLinks, HeldGrip};

/// Units: the original's links work in centimetres.
const CM: f32 = 100.0;
/// A pivot's softening per cm².
const SOFTENING: f32 = 0.001;
/// Gaps below this (m²) are left alone.
const EPSILON2: f32 = 1.19e-11;
/// A slack at or below this (m) is a pivot, as the limp link law reads it.
const PIVOT_SLACK: f32 = 1.19e-9;
/// A slack link's smoothed length does not follow a change below this (m).
const LENGTH_EPSILON: f32 = 1.19e-9;

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
    let d = b - a;
    let len2 = d.length_squared();
    if len2 <= EPSILON2 {
        return None;
    }
    let len = len2.sqrt();
    let correction = if link.slack <= PIVOT_SLACK {
        // The original's pivot: d·strength/(1 + 0.001·len²), with len in
        // cm.
        d * (link.strength / (len2 * CM * CM * SOFTENING + 1.0))
    } else if len > link.slack {
        link.length = link.slack;
        d * (link.strength * (len - link.slack) / len)
    } else {
        // Inside the slack the limp link is a damper on its smoothed
        // length (the grip main links' table slack starts full).
        let old = link.length;
        #[allow(clippy::neg_cmp_op_on_partial_ord)]
        if !((len - old).abs() > LENGTH_EPSILON) {
            return None;
        }
        link.length = old * link.smoothing + (1.0 - link.smoothing) * len;
        d * (link.strength * (len - old) / len)
    };
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
        if let Some(mut links) = grip::build(
            item,
            holder,
            grip::GripBindings {
                item_part: HeldGrip::ITEM_PART,
                hand_parts: hands,
                arm_parts: arms,
            },
            &grip.spec,
            grip.scale,
            grip.size,
            item_to_hand,
            grip.flip,
            grip.second_hand_factor,
        ) {
            // The two world welds every armed character's item gets: a soft
            // one 0.3 of the item's node box height above the grip and a
            // stiff one at its origin, both built disabled.
            let centre = item
                .parts
                .first()
                .map(|p| p.pairing.mesh.centre)
                .unwrap_or(Vec3::ZERO);
            let soft = Vec3::new(0.0, (centre.y * 2.0) * 0.3 * 100.0, 0.0);
            if let Some((soft, hard)) = grip::world_links(
                item,
                grip::GripBindings {
                    item_part: HeldGrip::ITEM_PART,
                    hand_parts: hands,
                    arm_parts: arms,
                },
                soft,
            ) {
                links.world_soft = Some(links.links.len());
                links.links.push(soft);
                links.world_hard = Some(links.links.len());
                links.links.push(hard);
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

/// Releases the items whose holder has gone limp or left: the link records
/// stop acting, the item body loses its holder and held mask, takes the
/// dropped weapon's friction and falls free (`0x155a20`/`0x155880`).
pub fn release_freed_grips(
    mut commands: Commands,
    mut held: Query<(Entity, &mut HeldGrip, &mut Ragdoll)>,
    bodies: Query<&Ragdoll, Without<HeldGrip>>,
) {
    for (entity, mut grip, mut body) in &mut held {
        if grip.released || grip.links.is_none() {
            continue;
        }
        let owner = bodies
            .iter()
            .find(|b| b.parts.first().is_some_and(|p| p.entity == grip.holder_key));
        let let_go = owner.is_none_or(|o| o.drive.is_none());
        if !let_go {
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

/// Installs the grip stage into the solver's stages.
pub fn install_stage(mut stages: ResMut<SolverStages>) {
    if stages.get::<GripSolver>().is_none() {
        stages.add(GripSolver::default());
    }
}

/// Adds the grip stage and its per-tick sync. Called by
/// [`crate::PhysicsPlugin`].
pub fn plugin(app: &mut App) {
    app.add_systems(Startup, install_stage).add_systems(
        FixedUpdate,
        (build_grips, release_freed_grips, mark_held, sync_grips)
            .chain()
            .before(BodySystems),
    );
}

#[cfg(test)]
#[path = "grip_solve_test.rs"]
mod tests;

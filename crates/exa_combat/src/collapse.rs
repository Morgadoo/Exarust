//! The collapse before death: a lethal wound does not kill at once. As in
//! the original, the fighter's condition becomes collapsing (0x800) and it
//! starts Fall Backward with a blend of round((1.2 − E) × 300) frames, E
//! its energy after the blow, eased smoothly; it no longer stands on its
//! controller, and its lower legs leave the living lower legs' collision
//! layer. Each tick its dying tick runs in place of the posture
//! decision ([`dying_tick`]): its heading wanders, it throws its arms wide
//! with Rebalance (20 frames, linear) once its balance is under 0.8 (not a
//! body of size 1.2 or more), its tone falls 0.003 and its support pins'
//! gain 0.0003; its body meanwhile follows its pelvis with its frame
//! exactly, its lift fades by its tone squared and its arms' stiffness is
//! 0.2 (`exa_physics::body::Bearing::collapsing`). At a tone of 0.25,
//! 250 ticks (3.0 s) after the blow, it dies: dead (0xA00), the may-rise
//! rule and rise clock settled, its yellow health 0 (it regenerates while
//! collapsing), a plain ragdoll that keeps the motion it has. Rules:
//! `docs/formats/RSQ.md` (Condition, The collapse).

use bevy::prelude::*;
use exa_anim::{Animator, Ease};
use exa_physics::body::{
    BodyTicks, CharacterStage, CharacterTick, GoLimp, Ragdoll, RigidPart, TickedCharacter, pairs,
    step_length,
};

use crate::condition::{Condition, DeathClock, StateFlags, settle_death, track_condition};
use crate::fall::FallDeath;
use crate::{Fighter, Roll};

/// The clip a lethal wound starts.
pub const FALL_BACKWARD: &str = "Fall Backward";
/// The clip a collapsing body throws its arms wide with.
pub const REBALANCE: &str = "Rebalance";
/// Fall Backward starts with round((1.2 − E) × 300) frames of blend,
/// computed in double precision.
const BLEND_FROM: f64 = 1.2;
const BLEND_SCALE: f64 = 300.0;
/// Tone lost each dying tick (subtracted in double precision).
const TONE_LOSS: f64 = 0.003;
/// At or under this tone (half the original's 0.5) the body dies; its
/// muscles off, its tone becomes 0.01.
pub const DEATH_TONE: f32 = 0.25;
const DEAD_TONE: f32 = 0.01;
/// The support pins' gain lost each dying tick (in double precision); once
/// under 0 it is set to 0.01.
const PIN_LOSS: f64 = 0.0003;
const PIN_RESET: f32 = 0.01;
/// Bodies of this size or more (in double precision) never Rebalance.
const BIG_FROM: f64 = 1.2;
/// Under this balance (double) a collapsing body Rebalances, with this
/// blend.
const REBALANCE_BELOW: f64 = 0.8;
const REBALANCE_BLEND: u32 = 20;
/// Each tick the heading's x and z each move by (random − 0.5) × 0.5.
const JITTER: f32 = 0.5;

/// The blend (frames) Fall Backward starts with at `energy`, the energy
/// after the killing blow: round((1.2 − E) × 300), ties to even (60 at
/// full energy, 345 at the daze's floor of 0.05).
pub fn collapse_blend(energy: f32) -> u32 {
    ((BLEND_FROM - f64::from(energy)) * BLEND_SCALE)
        .round_ties_even()
        .max(0.0) as u32
}

/// The heading (x, z, a unit vector across the ground) after one dying
/// tick, from two random draws `a` and `b` in 0..1: each component moves
/// by (draw − 0.5) × 0.5, at most a quarter, then it is a unit vector
/// again.
pub fn jitter_heading(heading: Vec2, a: f32, b: f32) -> Vec2 {
    let moved = Vec2::new(
        heading.x + (a - 0.5) * JITTER,
        heading.y + (b - 0.5) * JITTER,
    );
    moved.try_normalize().unwrap_or(heading)
}

/// One dying tick of the tone: 0.003 less; at 0.25 or less the muscles go
/// and the tone is 0.01. Returns the tone and whether the body dies.
pub fn lose_tone(tone: f32) -> (f32, bool) {
    let tone = (f64::from(tone) - TONE_LOSS) as f32;
    if tone <= DEATH_TONE {
        (DEAD_TONE, true)
    } else {
        (tone, false)
    }
}

/// One dying tick of the support pins' gain: 0.0003 less, and 0.01 once
/// it went under 0.
pub fn fade_pins(gain: f32) -> f32 {
    let gain = (f64::from(gain) - PIN_LOSS) as f32;
    if gain < 0.0 { PIN_RESET } else { gain }
}

/// A balance that widens above this double scales the reach by itself. A
/// float 0.4 widens above it, so it takes that path.
const STEP_BALANCE: f64 = 0.4;
/// Below that double the reach is widened and multiplied by this double.
const STEP_SHARE: f64 = 0.4;
/// The tone is lifted by this before it is squared.
const TONE_LIFT: f32 = 0.5;

/// What a collapsing body brings to one stagger step.
#[derive(Clone, Copy, Debug)]
pub struct CollapseStep {
    /// A step is already running.
    pub stepping: bool,
    /// The body's step length.
    pub reach: f32,
    pub balance: f32,
    /// The stored square root of the body scale.
    pub scale_root: f32,
    pub tone: f32,
    pub chest: Vec3,
    /// The chest's fast velocity, in the same units as the chest.
    pub velocity: Vec3,
}

/// The step a collapse starts: its scaled length, the chest target, and
/// the landing count and step kind, both cleared.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StaggerStep {
    pub length: f32,
    pub target: Vec3,
    pub landings: u32,
    pub kind: u32,
}

/// One stagger step while a body collapses, unless it is already stepping.
/// A balance that widens above 0.4 scales the reach by the balance. A
/// lower one widens the reach and multiplies by 0.4. The span squares the
/// tone plus one half and multiplies by that length and the scale root.
/// The target is the chest plus the velocity's reciprocal-square-root unit
/// times the span. A zero velocity leaves the chest. Landings and the step
/// kind are 0.
pub fn stagger_step(step: &CollapseStep) -> Option<StaggerStep> {
    if step.stepping {
        return None;
    }
    let length = if f64::from(step.balance) > STEP_BALANCE {
        step.reach * step.balance
    } else {
        (f64::from(step.reach) * STEP_SHARE) as f32
    };
    let rise = step.tone + TONE_LIFT;
    let span = rise * rise * (length * step.scale_root);
    let direction = unit_velocity(step.velocity);
    Some(StaggerStep {
        length,
        target: step.chest + direction * span,
        landings: 0,
        kind: 0,
    })
}

fn unit_velocity(velocity: Vec3) -> Vec3 {
    let sum = (velocity.x * velocity.x + velocity.z * velocity.z) + velocity.y * velocity.y;
    if sum == 0.0 {
        return Vec3::ZERO;
    }
    let scale = reciprocal_sqrt(sum);
    Vec3::new(velocity.x * scale, velocity.y * scale, velocity.z * scale)
}

fn reciprocal_sqrt(value: f32) -> f32 {
    #[cfg(target_arch = "x86_64")]
    {
        // The same approximate reciprocal square root the step uses to unit a velocity.
        unsafe {
            use std::arch::x86_64::{_mm_cvtss_f32, _mm_rsqrt_ss, _mm_set_ss};
            _mm_cvtss_f32(_mm_rsqrt_ss(_mm_set_ss(value)))
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        1.0 / value.sqrt()
    }
}

/// Whether a collapsing body of `size` with `balance` throws its arms wide
/// (Rebalance): under size 1.2 and balance 0.8, both in double precision.
pub fn rebalances(size: f32, balance: f32) -> bool {
    f64::from(size) < BIG_FROM && f64::from(balance) < REBALANCE_BELOW
}

/// A dying tick's Rebalance: when the body [`rebalances`], Rebalance (a
/// looping held pose) starts with 20 frames of blend, linear, unless it
/// already plays (the clip start then does nothing). Returns whether it
/// started.
pub fn throw_arms_wide(anim: &mut Animator, size: f32, balance: f32) -> bool {
    if !rebalances(size, balance) || anim.action == REBALANCE {
        return false;
    }
    anim.start(REBALANCE, true, None, REBALANCE_BLEND, Ease::Linear);
    true
}

/// On a collapsing fighter (its body model root): the heading its dying
/// ticks drift (x, z; taken from where the body faces at the first) and
/// how many it has had. `released` is set once the held weapons have been
/// dropped.
#[derive(Component, Debug, Clone, Copy, Default, PartialEq)]
pub struct Collapse {
    pub heading: Option<Vec2>,
    pub ticks: u32,
    pub released: bool,
}

/// One part of a held weapon. The drop copies the weapon's owner down this
/// tree and stamps the weapon as every part's root.
#[derive(Clone, Debug, PartialEq)]
pub struct WeaponPart {
    pub id: u64,
    pub owner: u64,
    pub root: u64,
    pub children: Vec<WeaponPart>,
    pub object_type: u32,
    pub object_flags: u32,
    pub mark: i32,
    pub iterations: i32,
}

/// A weapon in a hand. The collapse drops it and keeps its velocity.
#[derive(Clone, Debug, PartialEq)]
pub struct HeldWeapon {
    pub id: u32,
    /// The item's own mobility, restored when it leaves the hand.
    pub own_mobility: f32,
    pub mobility: f32,
    pub friction: f32,
    pub striker: u32,
    pub velocity: Vec3,
    pub part: WeaponPart,
    /// The parent's child ids, when the weapon has a parent.
    pub parent: Option<Vec<u64>>,
    pub links: i32,
    pub grip: i32,
    pub awake: u8,
    /// The weapon's rigid shape, when it can enter the solver.
    pub shape: Option<RigidPart>,
}

/// A weapon that has left the hand: its own mobility, friction 0.75, no
/// striker flags, the velocity it already had, and itself as owner.
#[derive(Clone, Debug, PartialEq)]
pub struct LooseWeapon {
    pub id: u32,
    pub mobility: f32,
    pub drag: f32,
    pub friction: f32,
    pub striker: u32,
    pub velocity: Vec3,
    pub part: WeaponPart,
    pub parent: Option<Vec<u64>>,
    pub links: i32,
    pub grip: i32,
    pub awake: u8,
    pub shape: Option<RigidPart>,
    /// Set once the dying tick has put this weapon into the solver.
    pub embodied: bool,
}

/// The two hands. `count` is how many slots are filled. `class_word` is
/// the weapon class. `primary` is cleared once nothing is held.
#[derive(Component, Clone, Debug, Default, PartialEq)]
pub struct Hands {
    pub count: u32,
    pub slots: [Option<HeldWeapon>; 2],
    pub class_word: u32,
    pub primary: u64,
    pub loose: Vec<LooseWeapon>,
}

const DROP_FRICTION: f32 = 0.75;
const DROP_DRAG: f64 = 0.175;
/// The iteration base added to the number of parts the root walk visits.
const PART_BASE: i32 = 2;
const LOOSE_MARK: i32 = -2;
const LOOSE_TYPE: u32 = 0x89;
const LOOSE_BIT: u32 = 0x400000;

/// Drops every held weapon. Each keeps its velocity, takes its own
/// mobility, friction 0.75 and a drag of that mobility times 0.175. The
/// striker flags clear. The weapon becomes its own owner, and that owner
/// is copied onto every child. The hands, the class word and the primary
/// are empty afterwards. An empty pair stays empty.
pub fn release_hands(hands: &mut Hands) {
    for slot in &mut hands.slots {
        let Some(held) = slot.take() else {
            continue;
        };
        hands.loose.push(loosen(held));
    }
    hands.count = 0;
    hands.class_word = 0;
    hands.primary = 0;
}

fn loosen(held: HeldWeapon) -> LooseWeapon {
    let id = held.part.id;
    let mut part = held.part;
    part.owner = id;
    own_children(&mut part);
    let visited = stamp_root(&mut part, id);
    part.iterations = PART_BASE + visited - 1;
    let grip = if held.grip >= 0 { -1 } else { held.grip };
    LooseWeapon {
        id: held.id,
        mobility: held.own_mobility,
        drag: (f64::from(held.own_mobility) * DROP_DRAG) as f32,
        friction: DROP_FRICTION,
        striker: 0,
        velocity: held.velocity,
        part,
        parent: without_child(held.parent, id),
        links: 0,
        grip,
        awake: 0,
        shape: held.shape,
        embodied: false,
    }
}

/// A solid dropped weapon as one free body: its own mobility, the rigid
/// bodies' friction, its spheres, and the velocity it already had. It is
/// awake, so the solver steps it. A missing or flat shape stays out.
pub fn free_body(loose: &LooseWeapon) -> Option<Ragdoll> {
    let shape = loose.shape.as_ref().filter(|shape| shape.is_solid())?;
    let mut shape = shape.clone();
    shape.mobility = loose.mobility;
    let mut body = Ragdoll::rigid(std::slice::from_ref(&shape));
    body.set_velocity(loose.velocity);
    body.wake();
    Some(body)
}

/// Copies the part's owner onto every child. A part with a child, an
/// object type whose low twelve bits are 0x89, and object bit 0x400000,
/// sets its mark to -2.
fn own_children(part: &mut WeaponPart) {
    if part.children.is_empty() {
        return;
    }
    for child in &mut part.children {
        child.owner = part.owner;
        own_children(child);
    }
    if (part.object_type & 0xfff) == LOOSE_TYPE && (part.object_flags & LOOSE_BIT) != 0 {
        part.mark = LOOSE_MARK;
    }
}

/// Writes `root` onto this part and every descendant. Returns how many
/// parts that visit counted.
fn stamp_root(part: &mut WeaponPart, root: u64) -> i32 {
    part.root = root;
    let mut visited = 1;
    for child in &mut part.children {
        visited += stamp_root(child, root);
    }
    visited
}

fn without_child(parent: Option<Vec<u64>>, id: u64) -> Option<Vec<u64>> {
    let mut list = parent?;
    if let Some(index) = list.iter().position(|child| *child == id) {
        list.remove(index);
    }
    Some(list)
}

/// Starts a lethal wound's collapse, unless the fighter collapses or is
/// dead already: no longer waking, condition collapsing, Fall Backward
/// (a looping held pose) starting from where the body is with
/// [`collapse_blend`] frames at `energy` (the clip start does nothing when
/// it already plays), eased smoothly either way; its body no longer stands
/// on its controller, and its lower legs leave the living lower legs'
/// layer, so they meet other characters' again. Returns whether it
/// started.
pub fn start_collapse(
    condition: &mut Condition,
    flags: &mut StateFlags,
    anim: &mut Animator,
    body: Option<&mut Ragdoll>,
    energy: f32,
    hands: Option<&mut Hands>,
) -> bool {
    if condition.dying() {
        return false;
    }
    flags.0 &= !StateFlags::WAKING;
    *condition = Condition::COLLAPSING;
    if let Some(hands) = hands {
        release_hands(hands);
    }
    if anim.action != FALL_BACKWARD {
        let blend = collapse_blend(energy);
        anim.start(FALL_BACKWARD, true, None, blend, Ease::Smooth);
    } else {
        anim.looping = true;
        anim.then = None;
    }
    anim.ease = Ease::Smooth;
    if let Some(body) = body {
        body.fall();
        pairs::set_leg_layers(&mut body.parts, 0);
    }
    true
}

/// The dying tick's stagger walk, in centimetres inside [`stagger_step`]
/// and metres for [`Ragdoll::walk_stagger`]. A step that is already running
/// writes nothing. Without a measured chest there is nothing to walk.
fn walk_collapse(body: &mut Ragdoll) {
    let ready = {
        let Some(drive) = body.drive.as_ref() else {
            return;
        };
        let Some(chest) = drive.balance().chest() else {
            return;
        };
        let scale = drive.stats.scale;
        let frame = drive.anchor().unwrap_or(Vec3::new(chest.x, 0.0, chest.z));
        let step = CollapseStep {
            stepping: drive.stepping(),
            reach: step_length(scale, drive.balance().extension()),
            balance: drive.balance().value,
            scale_root: scale.max(0.0).sqrt(),
            tone: drive.tone,
            chest: chest * 100.0,
            velocity: drive.balance().fast_velocity() * 100.0,
        };
        (step, frame, scale)
    };
    let Some(step) = stagger_step(&ready.0) else {
        return;
    };
    let spots: Vec<Option<Vec3>> = (0..body.legs().len()).map(|i| body.ankle(i)).collect();
    let _walked = body.walk_stagger(step.target / 100.0, ready.1, &spots, ready.2);
}

pub struct CollapsePlugin;

impl Plugin for CollapsePlugin {
    fn build(&self, app: &mut App) {
        // The collapsing branch of the posture stage, in place of the
        // posture decision (which leaves the collapsing alone).
        app.add_systems(
            CharacterTick,
            dying_tick
                .in_set(CharacterStage::Posture)
                .after(track_condition),
        );
    }
}

/// The ticked body's dying tick, in its posture stage while it collapses:
/// its heading drifts by two draws of the original's generator (drawn in
/// the characters' tick order); under balance 0.8 a body under size 1.2
/// starts Rebalance (once: the start does nothing while it plays); its
/// tone falls 0.003 and its pins' gain 0.0003. Unless a step is already
/// running, it walks the stagger from the chest and its fast velocity.
/// At a tone of 0.25 it dies
/// there and then: [`crate::condition::settle_death`] writes the condition,
/// the may-rise rule and the rise clock, its yellow health becomes 0, its
/// muscles go, it keeps its motion, and the rest of its tick does not run
/// ([`crate::condition::ticked_lives`]); the frame strips its clip and
/// controller.
#[allow(clippy::type_complexity)]
pub fn dying_tick(
    ticked: Res<TickedCharacter>,
    mut commands: Commands,
    ticks: Option<Res<BodyTicks>>,
    mut roll: Local<Roll>,
    mut bodies: Query<(
        &mut Condition,
        &FallDeath,
        &mut Collapse,
        &mut Ragdoll,
        &mut Animator,
        &mut Fighter,
        &GlobalTransform,
        Option<&mut Hands>,
    )>,
) {
    let entity = ticked.body;
    if let Ok((mut condition, fall, mut collapse, mut body, mut anim, mut fighter, gt, hands)) =
        bodies.get_mut(entity)
    {
        if !condition.is_collapsing() {
            return;
        }
        let dies = {
            let Some(drive) = body.drive.as_mut() else {
                return;
            };
            let heading = collapse.heading.unwrap_or_else(|| {
                (gt.rotation() * Vec3::Z)
                    .xz()
                    .try_normalize()
                    .unwrap_or(Vec2::Y)
            });
            let (a, b) = (roll.next(), roll.next());
            collapse.heading = Some(jitter_heading(heading, a, b));
            if throw_arms_wide(&mut anim, fighter.stats.size, drive.balance().value) {
                info!(
                    "{entity:?} rebalances {} ticks into its collapse",
                    collapse.ticks
                );
            }
            let (tone, dies) = lose_tone(drive.tone);
            drive.tone = tone;
            drive.pin_gain = fade_pins(drive.pin_gain);
            collapse.ticks += 1;
            dies
        };
        if let Some(mut hands) = hands
            && !collapse.released
        {
            release_hands(&mut hands);
            for loose in &mut hands.loose {
                if loose.embodied {
                    continue;
                }
                let Some(body) = free_body(loose) else {
                    continue;
                };
                commands.spawn(body);
                loose.embodied = true;
            }
            collapse.released = true;
        }
        walk_collapse(&mut body);
        if dies {
            let now_ms = ticks.as_ref().map(|t| t.millis() as u32).unwrap_or(0);
            if let Some(settled) = settle_death(
                *condition,
                fighter.stats.rules,
                fall.stage.death_stage(),
                fighter.health.red,
                fighter.first_flag_red,
                now_ms,
            ) {
                *condition = settled.condition;
                fighter.stats.rules = settled.rules;
                commands.entity(entity).insert(DeathClock {
                    wake_at_ms: settled.wake_at_ms,
                    died_at_ms: now_ms,
                });
            }
            // Death empties the yellow bar the collapse regenerated, writes
            // the class word 0 (0x30 on each first child) and clears the
            // strike chain on those children.
            fighter.health.yellow = 0.0;
            body.set_class_word(0);
            body.clear_death_strikes();
            body.go_limp();
            commands
                .entity(entity)
                .insert(GoLimp {
                    velocity: None,
                    delay: 0,
                })
                .remove::<Collapse>();
            info!(
                "{entity:?} dies {} ticks after its collapse",
                collapse.ticks
            );
        }
    }
}

#[cfg(test)]
#[path = "collapse_test.rs"]
mod tests;

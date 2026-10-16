//! Balance: a living body whose centre of mass strays too far from the line
//! between its feet for too long loses its balance and falls. Once it has
//! settled and recovered its wits it gets up with the get-up clip matching
//! how it lies (on its back or its front), and stands again on its
//! character controller.
//!
//! Rules (`docs/formats/RSQ.md`, Physical body): the threshold is
//! (energy·0.75 + 0.25)·30 cm; the score is stability⁸; a counter grows by
//! 10 − 10·score per tick and, at 100, sets the balance to the score;
//! otherwise balance recovers by (5 − speed in cm/tick)·0.01 per tick. Below
//! 0.6 the body falls. While getting up, balance rises by 0.05 per tick.

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::{Animator, Motions, in_place, sample};
use exa_physics::CharacterBody;
use exa_physics::body::{BODY_HZ, Fallen, Ragdoll};

use crate::player::PlayerBody;

pub struct BalancePlugin;

impl Plugin for BalancePlugin {
    fn build(&self, app: &mut App) {
        // Once a frame, after the frame's body ticks, over the ticks they ran.
        app.add_systems(Update, (keep_balance, get_up).chain());
    }
}

/// Balance of a living physical body (on its body model root).
#[derive(Component, Debug, Clone)]
pub struct Balance {
    /// 1 when steady; the body falls below [`FALL_BELOW`].
    pub value: f32,
    counter: f32,
}

impl Default for Balance {
    fn default() -> Self {
        Self {
            value: 1.0,
            counter: 0.0,
        }
    }
}

impl Balance {
    /// The instability counter: balance is set to the score when it
    /// reaches 100.
    pub fn counter(&self) -> f32 {
        self.counter
    }
}

const FALL_BELOW: f32 = 0.6;
/// Half the support threshold at full energy, metres.
const THRESHOLD: f32 = 0.3;
/// How far ahead the centre of mass is projected by its velocity, seconds.
const LOOK_AHEAD: f32 = 0.1;
/// Fastest the centre of mass is taken to move for the look-ahead, m/s
/// (clip changes snap the pose until clips blend).
const LOOK_AHEAD_SPEED: f32 = 2.0;
/// Score of a steady stance.
const STEADY: f32 = 0.99;
/// Seconds lying down before trying to get up, and the energy and speed
/// needed.
const GET_UP_AFTER: f32 = 2.5;
const GET_UP_ENERGY: f32 = 0.9;
const SETTLED_SPEED: f32 = 0.2;
const RIGHT_FOOT: usize = 4;
const LEFT_FOOT: usize = 7;
const PELVIS: usize = 1;

/// Horizontal distance from `p` to the segment `a`–`b`.
fn off_line(p: Vec3, a: Vec3, b: Vec3) -> f32 {
    let (p, a, b) = (p.xz(), a.xz(), b.xz());
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(1e-6)).clamp(0.0, 1.0);
    p.distance(a + ab * t)
}

/// One frame of the balance rule over `ticks` body ticks; returns whether
/// the body falls.
fn update_balance(balance: &mut Balance, offset: f32, energy: f32, speed: f32, ticks: f32) -> bool {
    let threshold = (energy * 0.75 + 0.25) * THRESHOLD;
    // Steady within the threshold, hopeless past twice it (the original's
    // measure is not known; attack lunges reach about 20 cm).
    let stability = (2.0 - offset / threshold).clamp(0.0, 1.0);
    let score = stability.powi(8);
    // A steady stance clears what small wobbles added up.
    if score >= STEADY {
        balance.counter = 0.0;
    }
    balance.counter += (10.0 - 10.0 * score) * ticks;
    if balance.counter >= 100.0 {
        balance.value = score;
        balance.counter = 0.0;
    } else {
        let cm_per_tick = speed * 100.0 / BODY_HZ;
        balance.value = (balance.value + (5.0 - cm_per_tick).max(0.0) * 0.01 * ticks).min(1.0);
    }
    balance.value < FALL_BELOW
}

#[allow(clippy::type_complexity)]
fn keep_balance(
    mut commands: Commands,
    mut bodies: Query<
        (
            Entity,
            &mut Ragdoll,
            &mut Animator,
            &GlobalTransform,
            Option<&mut Balance>,
        ),
        Without<Fallen>,
    >,
) {
    for (root, mut body, mut anim, root_gt, balance) in &mut bodies {
        let ticks = body.ticks as f32;
        let Some(energy) = body.drive.as_ref().map(|d| d.energy) else {
            continue;
        };
        let Some(mut balance) = balance else {
            commands.entity(root).insert(Balance::default());
            continue;
        };
        let (Some(right), Some(left)) = (body.part_of(RIGHT_FOOT), body.part_of(LEFT_FOOT)) else {
            continue;
        };
        let (a, b) = (right.transform().translation, left.transform().translation);
        let (centre, velocity) = body.centre_of_mass();
        let drift = Vec3::new(velocity.x, 0.0, velocity.z).clamp_length_max(LOOK_AHEAD_SPEED);
        let ahead = centre + drift * LOOK_AHEAD;
        let offset = off_line(ahead, a.into(), b.into());
        let falls = update_balance(&mut balance, offset, energy, velocity.length(), ticks);
        if falls {
            body.fall();
            // Fall the way the body is going.
            let forward = root_gt.rotation() * Vec3::Z;
            let midpoint = Vec3::from((a + b) * 0.5);
            let action = if (ahead - midpoint).dot(forward) > 0.0 {
                "Fall Forward"
            } else {
                "Fall Backward"
            };
            anim.play_once(action, "Unconcious 1");
            commands
                .entity(root)
                .insert(Fallen::default())
                .remove::<exa_combat::Attacking>();
            info!("{root:?} loses its balance");
        }
    }
}

#[allow(clippy::type_complexity)]
fn get_up(
    mut commands: Commands,
    motions: Option<Res<Motions>>,
    mut bodies: Query<(
        Entity,
        &mut Ragdoll,
        &mut Animator,
        &mut Fallen,
        &mut Transform,
        &ChildOf,
        Option<&mut Balance>,
        Option<&mut PlayerBody>,
    )>,
    mut characters: Query<(&mut Transform, &mut CharacterBody), Without<Ragdoll>>,
) {
    let Some(motions) = motions else { return };
    for (root, mut body, mut anim, mut fallen, mut t, parent, balance, player) in &mut bodies {
        let ticks = body.ticks as f32;
        // Body time, which may run slower than the clock.
        fallen.time += ticks / BODY_HZ;
        let Ok((mut ct, mut controller)) = characters.get_mut(parent.parent()) else {
            continue;
        };
        controller.desired_velocity = Vec3::ZERO;
        let stance = if anim.moveset == "Default" {
            "Idle"
        } else {
            "Cmbt - Idle"
        };
        if fallen.getting_up {
            if let Some(mut b) = balance {
                b.value = (b.value + 0.05 * ticks).min(1.0);
                b.counter = 0.0;
            }
            if anim.looping {
                commands.entity(root).remove::<Fallen>();
            }
            continue;
        }
        let energy = body.drive.as_ref().map_or(0.0, |d| d.energy);
        let (_, velocity) = body.centre_of_mass();
        if fallen.time < GET_UP_AFTER || energy < GET_UP_ENERGY || velocity.length() > SETTLED_SPEED
        {
            continue;
        }
        let Some(pelvis) = body.part_of(PELVIS).map(|p| p.transform()) else {
            continue;
        };
        // The get-up clip whose first pose lies the way the body does.
        let lying_front = pelvis.transform_vector3(Vec3::Z).y;
        let Some((action, start)) = ["Get up", "Get Up 2"]
            .iter()
            .filter_map(|&a| {
                let clip = motions.0.clip(&anim.moveset, a)?;
                let time = clip.keys.first()?.time;
                let (rotations, root_pos) = sample(clip, time)?;
                let pose = Affine3A::from_rotation_translation(
                    *rotations.first()?,
                    in_place(clip, time, root_pos),
                );
                Some((a, pose))
            })
            .max_by(|x, y| {
                let fx = x.1.transform_vector3(Vec3::Z).y * lying_front;
                let fy = y.1.transform_vector3(Vec3::Z).y * lying_front;
                fx.total_cmp(&fy)
            })
        else {
            continue;
        };
        // Turn and move the character so the clip's first pose lies on the
        // body: head the same way, pelvis over the pelvis.
        let heading = |m: Affine3A| {
            let up = m.transform_vector3(Vec3::Y);
            up.x.atan2(up.z)
        };
        let yaw = heading(pelvis) - heading(start);
        let turn = Quat::from_rotation_y(yaw);
        let offset = turn * Vec3::from(start.translation);
        let pelvis_at = Vec3::from(pelvis.translation);
        ct.translation.x = pelvis_at.x - offset.x;
        ct.translation.z = pelvis_at.z - offset.z;
        t.rotation = turn;
        if let Some(mut pb) = player {
            pb.yaw = yaw;
        }
        body.stand();
        anim.play_once(action, stance);
        fallen.getting_up = true;
        info!("{root:?} gets up ({action})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steady_bodies_keep_their_balance() {
        let mut b = Balance::default();
        for _ in 0..1000 {
            assert!(!update_balance(&mut b, 0.05, 1.0, 0.0, 1.0));
        }
    }

    #[test]
    fn a_dazed_body_pushed_off_its_feet_falls() {
        let mut b = Balance::default();
        let fell = (0..40).any(|_| update_balance(&mut b, 0.2, 0.3, 0.5, 1.0));
        assert!(fell);
    }

    #[test]
    fn distance_to_the_support_line() {
        let d = off_line(
            Vec3::new(0.0, 1.0, 0.3),
            Vec3::new(-0.2, 0.0, 0.0),
            Vec3::new(0.2, 0.0, 0.0),
        );
        assert!((d - 0.3).abs() < 1e-6);
    }
}

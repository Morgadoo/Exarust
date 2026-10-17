//! Falling and getting up. Each living body measures its balance in its
//! character tick, as the original does (`exa_physics::body::Balance`).
//! A body whose balance has dropped below 0.6 falls, the way it leans; once
//! it has settled and recovered its wits it gets up with the get-up clip
//! matching how it lies (on its back or its front), and stands again on its
//! character controller, which follows its frame to where it rises (the
//! frame re-anchors to its pelvis within 10 cm in the clip's first
//! quarter). While its get-up clip plays its balance rises by 0.05 a tick.
//!
//! The fall starts as the original's posture block starts one, with no
//! blend and the fast ease; the get-up as the original's get-up from the
//! floor, blending over (first key + 20) frames from the lying body's
//! whole physical pose. When the fall and the get-up happen is still
//! Exarust's own (the original's staggers, Rebalance and get-up rules come
//! later). Rules: `docs/formats/RSQ.md` (Physical body).

use bevy::math::Affine3A;
use bevy::prelude::*;
use exa_anim::{Animator, Ease, Motions, sample};
use exa_combat::stance::{Stance, StanceTick};
use exa_physics::CharacterBody;
use exa_physics::body::{BODY_HZ, CharacterTicks, Fallen, Ragdoll};

use crate::player::PlayerBody;

pub struct BalancePlugin;

impl Plugin for BalancePlugin {
    fn build(&self, app: &mut App) {
        // Falls are decided in the fixed step from last tick's balance,
        // after the daze's part of the body tick (as the original's posture
        // block follows its daze block) and before the character ticks;
        // getting up once a frame.
        app.add_systems(
            FixedUpdate,
            fall_off_balance.after(StanceTick).before(CharacterTicks),
        )
        .add_systems(Update, get_up);
    }
}

/// A body falls when its balance is below this.
const FALL_BELOW: f32 = 0.6;
/// Seconds lying down before trying to get up, and the energy and speed
/// needed.
const GET_UP_AFTER: f32 = 2.5;
const GET_UP_ENERGY: f32 = 0.9;
const SETTLED_SPEED: f32 = 0.2;
const PELVIS: usize = 1;
/// A get-up from the floor blends this many frames past its first key.
const GET_UP_BLEND: u32 = 20;

/// The fall clip for a body leaning along `lean` (the unit offset from its
/// support point to its chest) and facing `forward`: Fall Backward when the
/// lean is not ahead of it, else Fall Forward.
fn fall_toward(lean: Vec3, forward: Vec3) -> &'static str {
    if lean.dot(forward) <= 0.0 {
        "Fall Backward"
    } else {
        "Fall Forward"
    }
}

/// Makes every standing living body whose balance dropped below 0.6 last
/// tick fall the way it leans.
#[allow(clippy::type_complexity)]
fn fall_off_balance(
    mut commands: Commands,
    mut bodies: Query<(Entity, &mut Ragdoll, &mut Animator, &GlobalTransform), Without<Fallen>>,
) {
    for (root, mut body, mut anim, root_gt) in &mut bodies {
        let Some(lean) = body
            .drive
            .as_ref()
            .filter(|d| d.balance().value < FALL_BELOW)
            .map(|d| d.balance().lean())
        else {
            continue;
        };
        body.fall();
        let forward = root_gt.rotation() * Vec3::Z;
        anim.play_once(fall_toward(lean, forward), "Unconcious 1");
        anim.blended(0, Ease::Fast);
        commands
            .entity(root)
            .insert(Fallen::default())
            .remove::<exa_combat::Attacking>();
        info!("{root:?} loses its balance");
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
        Option<&mut PlayerBody>,
        Option<&Stance>,
    )>,
    mut characters: Query<&mut CharacterBody, Without<Ragdoll>>,
) {
    let Some(motions) = motions else { return };
    for (root, mut body, mut anim, mut fallen, mut t, parent, player, stance) in &mut bodies {
        let ticks = body.ticks as f32;
        // Body time, which may run slower than the clock.
        fallen.time += ticks / BODY_HZ;
        let Ok(mut controller) = characters.get_mut(parent.parent()) else {
            continue;
        };
        controller.desired_velocity = Vec3::ZERO;
        // The get-up ends in the fighter's stance (the one it holds when
        // the clip ends, `exa_combat::stance`), else its moveset's idle.
        let stance = match stance {
            Some(s) => s.action(),
            None if anim.plays_default_alone() => "Idle",
            None => "Cmbt - Idle",
        };
        if fallen.getting_up {
            // Its balance rises in its character ticks meanwhile.
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
                let clip = motions.clip(&anim, a)?;
                let time = clip.keys.first()?.time;
                let (rotations, root_pos) = sample(clip, time)?;
                let pose = Affine3A::from_rotation_translation(*rotations.first()?, root_pos);
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
        // Turn the body so the clip's first pose lies the way the body does,
        // head the same way. Its frame re-anchors to the pelvis in its
        // character tick as the get-up starts, so the clip's pelvis lies on
        // the body, and its character follows the frame there.
        let heading = |m: Affine3A| {
            let up = m.transform_vector3(Vec3::Y);
            up.x.atan2(up.z)
        };
        let yaw = heading(pelvis) - heading(start);
        let turn = Quat::from_rotation_y(yaw);
        t.rotation = turn;
        if let Some(mut pb) = player {
            pb.yaw = yaw;
        }
        body.stand();
        anim.play_once(action, stance);
        anim.blended(GET_UP_BLEND, Ease::Linear).capture_whole();
        fallen.getting_up = true;
        info!("{root:?} gets up ({action})");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A body falls the way its chest leans from its support point: forward
    /// when the lean is ahead of where it faces, backward otherwise (and
    /// when the lean is straight sideways).
    #[test]
    fn bodies_fall_the_way_they_lean() {
        let forward = Vec3::Z;
        assert_eq!(
            fall_toward(Vec3::new(0.0, 0.9, 0.4), forward),
            "Fall Forward"
        );
        assert_eq!(
            fall_toward(Vec3::new(0.0, 0.9, -0.4), forward),
            "Fall Backward"
        );
        assert_eq!(
            fall_toward(Vec3::new(0.4, 0.9, 0.0), forward),
            "Fall Backward"
        );
    }
}

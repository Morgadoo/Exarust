//! NPC behaviour. For now a duellist: face the target, close to striking
//! distance, attack at intervals with varied strikes. NPCs act through the
//! same pieces as the player (body velocity, animator, attacks).

use bevy::prelude::*;
use exa_anim::Animator;
use exa_combat::{Attacking, Fighter};
use exa_physics::CharacterBody;
use exa_physics::body::Fallen;

pub struct AiPlugin;

impl Plugin for AiPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, duellist);
    }
}

/// A fighter (on its body model root, whose parent carries the
/// `CharacterBody`) that duels `target` (a world-space entity, e.g. the
/// player's physics root).
#[derive(Component, Debug, Clone)]
pub struct Duellist {
    pub target: Entity,
    pub cooldown: f32,
    pub next_strike: usize,
}

/// The duellist strikes from this far (m, between the characters). It is
/// Exarust's stand-in for the original's reach: the arm follows its clip
/// through link muscles, which damp its swing (about 10–15 cm less reach
/// than the clip's pose), and the strike lands only from this close.
const STRIKE_RANGE: f32 = 1.2;
const APPROACH_SPEED: f32 = 1.2;
const STRIKE_INTERVAL: f32 = 2.2;
const STRIKES: [&str; 3] = ["Attack Mid", "Attack Up", "Attack Down"];

#[allow(clippy::type_complexity)]
fn duellist(
    mut commands: Commands,
    time: Res<Time>,
    targets: Query<&GlobalTransform>,
    fighters: Query<&Fighter>,
    children: Query<&Children>,
    mut npcs: Query<(
        Entity,
        &mut Duellist,
        &mut Transform,
        &mut Animator,
        &Fighter,
        &ChildOf,
        Option<&Attacking>,
        Has<Fallen>,
    )>,
    mut roots: Query<(&mut CharacterBody, &GlobalTransform)>,
) {
    for (entity, mut ai, mut t, mut anim, fighter, parent, attacking, fallen) in &mut npcs {
        let Ok((mut body, root_gt)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        body.desired_velocity = Vec3::ZERO;
        if fighter.down || fallen {
            continue;
        }
        let Ok(target) = targets.get(ai.target) else {
            continue;
        };
        // Leave a downed target alone.
        let target_down = std::iter::once(ai.target)
            .chain(children.iter_descendants(ai.target))
            .any(|e| fighters.get(e).is_ok_and(|f| f.down));
        if target_down {
            anim.play_loop("Cmbt - Idle");
            continue;
        }
        let to = target.translation() - root_gt.translation();
        let flat = Vec3::new(to.x, 0.0, to.z);
        let dist = flat.length();
        if dist > 0.05 {
            t.rotation = Quat::from_rotation_y(flat.x.atan2(flat.z));
        }
        ai.cooldown -= time.delta_secs();
        if attacking.is_some() || !anim.looping {
            continue;
        }
        if dist > STRIKE_RANGE {
            body.desired_velocity = flat / dist * APPROACH_SPEED;
            anim.play_loop("Combat Walk");
        } else {
            anim.play_loop("Cmbt - Idle");
            if ai.cooldown <= 0.0 {
                let strike = STRIKES[ai.next_strike % STRIKES.len()];
                ai.next_strike += 1;
                ai.cooldown = STRIKE_INTERVAL;
                exa_combat::start_attack(&mut commands, entity, &mut anim, strike, "Cmbt - Idle");
            }
        }
    }
}

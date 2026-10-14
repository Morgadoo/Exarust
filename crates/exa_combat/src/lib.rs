//! Combat resolution.
//!
//! A held weapon is a segment from its grip to its tip. While its wielder is
//! attacking, the segment is tested against the body parts of other fighters;
//! a contact deals damage that grows with the square of the tip's speed (a
//! stand-in for the original's momentum- and armour-based damage), staggers
//! the target, and knocks it down when its health runs out.

use std::collections::HashSet;

use bevy::prelude::*;
use exa_anim::{Animator, BodyPart, Motions};
use exa_world::PartBounds;

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Hit>().add_systems(
            PostUpdate,
            (track_blades, detect_hits, react_to_hits)
                .chain()
                .after(TransformSystems::Propagate),
        );
    }
}

/// A character that can fight and be hurt (on the body model root).
#[derive(Component, Debug, Clone)]
pub struct Fighter {
    pub health: f32,
    pub max_health: f32,
    pub down: bool,
}

impl Default for Fighter {
    fn default() -> Self {
        Self {
            health: 100.0,
            max_health: 100.0,
            down: false,
        }
    }
}

/// A weapon segment, on the entity holding the weapon model (local +Y).
#[derive(Component, Debug, Clone)]
pub struct Blade {
    /// Fighter (body model root) wielding it.
    pub wielder: Entity,
    /// Tip distance along local +Y (metres).
    pub length: f32,
    pub tip_last: Option<Vec3>,
    pub tip_speed: f32,
}

/// Attack in progress on a fighter: which targets it has already struck.
#[derive(Component, Debug, Default)]
pub struct Attacking {
    pub struck: HashSet<Entity>,
}

/// A weapon contact.
#[derive(Message, Debug, Clone)]
pub struct Hit {
    pub attacker: Entity,
    pub target: Entity,
    pub bone: usize,
    pub damage: f32,
    pub point: Vec3,
}

const PART_RADIUS: f32 = 0.16;
/// Damage per (m/s)² of tip speed.
const DAMAGE_PER_SPEED2: f32 = 0.9;
const MIN_HIT_SPEED: f32 = 1.5;
/// Attack window, as clip progress.
const ACTIVE_FROM: f32 = 0.35;
const ACTIVE_TO: f32 = 0.85;

/// Starts an attack clip on a fighter.
pub fn start_attack(
    commands: &mut Commands,
    fighter: Entity,
    anim: &mut Animator,
    action: &str,
    stance: &str,
) {
    anim.play_once(action, stance);
    commands.entity(fighter).insert(Attacking::default());
}

fn track_blades(time: Res<Time>, mut blades: Query<(&mut Blade, &GlobalTransform)>) {
    let dt = time.delta_secs().max(1e-4);
    for (mut blade, gt) in &mut blades {
        let tip = gt.transform_point(Vec3::Y * blade.length);
        blade.tip_speed = blade
            .tip_last
            .map_or(0.0, |last| (tip - last).length() / dt);
        blade.tip_last = Some(tip);
    }
}

fn detect_hits(
    motions: Option<Res<Motions>>,
    blades: Query<(&Blade, &GlobalTransform)>,
    mut attackers: Query<(&Animator, &mut Attacking)>,
    fighters: Query<(Entity, &Fighter)>,
    children: Query<&Children>,
    parts: Query<(&BodyPart, &GlobalTransform, Option<&PartBounds>)>,
    mut hits: MessageWriter<Hit>,
) {
    let Some(motions) = motions else { return };
    for (blade, gt) in &blades {
        let Ok((anim, mut attack)) = attackers.get_mut(blade.wielder) else {
            continue;
        };
        let progress = anim.progress(&motions.0).unwrap_or(0.0);
        if !(ACTIVE_FROM..=ACTIVE_TO).contains(&progress) || blade.tip_speed < MIN_HIT_SPEED {
            continue;
        }
        let a = gt.translation();
        let b = gt.transform_point(Vec3::Y * blade.length);
        for (target, fighter) in &fighters {
            if target == blade.wielder || fighter.down || attack.struck.contains(&target) {
                continue;
            }
            // The body part nearest the blade, if within reach.
            let contact = children
                .iter_descendants(target)
                .filter_map(|e| {
                    let (part, pgt, bounds) = parts.get(e).ok()?;
                    let (c, r) = match bounds {
                        // The enclosing sphere is generous; use most of it.
                        Some(b) => (pgt.transform_point(b.centre), b.radius * 0.75),
                        None => (pgt.translation(), PART_RADIUS),
                    };
                    let p = closest_on_segment(a, b, c);
                    let d = p.distance(c);
                    (d < r).then_some((d / r, part.bone, p))
                })
                .min_by(|x, y| x.0.total_cmp(&y.0))
                .map(|(_, bone, p)| (bone, p));
            if let Some((bone, point)) = contact {
                debug!(
                    "blade {a} -> {b} speed {:.2} contact bone {bone} at {point}",
                    blade.tip_speed
                );
                attack.struck.insert(target);
                hits.write(Hit {
                    attacker: blade.wielder,
                    target,
                    bone,
                    damage: DAMAGE_PER_SPEED2 * blade.tip_speed * blade.tip_speed,
                    point,
                });
            }
        }
    }
}

fn react_to_hits(
    mut commands: Commands,
    mut hits: MessageReader<Hit>,
    mut fighters: Query<(Entity, &mut Fighter, &mut Animator, Has<Attacking>)>,
) {
    for hit in hits.read() {
        let Ok((_, mut f, mut anim, _)) = fighters.get_mut(hit.target) else {
            continue;
        };
        f.health -= hit.damage;
        info!(
            "hit on bone {} for {:.0} (health {:.0})",
            hit.bone, hit.damage, f.health
        );
        if f.health <= 0.0 && !f.down {
            f.down = true;
            info!("fighter {:?} is down", hit.target);
            // The body goes limp and falls under the blow.
            commands
                .entity(hit.target)
                .insert(exa_physics::body::GoLimp {
                    velocity: Vec3::Y * 0.3,
                    delay: 0,
                });
        } else if !f.down {
            let stance = if anim.moveset == "Default" {
                "Idle"
            } else {
                "Cmbt - Idle"
            };
            let reaction = if hit.bone >= 9 {
                "Head Pain"
            } else {
                "Rebalance"
            };
            anim.play_once(reaction, stance);
        }
    }
    for (e, _, anim, attacking) in &fighters {
        if attacking && anim.looping {
            commands.entity(e).remove::<Attacking>();
        }
    }
}

fn closest_on_segment(a: Vec3, b: Vec3, p: Vec3) -> Vec3 {
    let ab = b - a;
    let t = ((p - a).dot(ab) / ab.length_squared().max(f32::EPSILON)).clamp(0.0, 1.0);
    a + ab * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segment_projection() {
        let p = closest_on_segment(Vec3::ZERO, Vec3::Y, Vec3::new(1.0, 0.5, 0.0));
        assert_eq!(p, Vec3::new(0.0, 0.5, 0.0));
        let end = closest_on_segment(Vec3::ZERO, Vec3::Y, Vec3::new(0.0, 3.0, 0.0));
        assert_eq!(end, Vec3::Y);
    }
}

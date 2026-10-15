//! Combat resolution.
//!
//! A held weapon is a segment from its grip to its tip. While its wielder is
//! attacking, the segment is tested against the body parts of other fighters.
//! A contact is resolved by the original's damage model ([`damage`]): the
//! weapon's stats, where along it the blow lands and how fast, against the
//! struck part's armour layers. It takes health, shoves and dazes the target,
//! and kills it when its yellow health runs out.

pub mod damage;

use std::collections::HashSet;

use bevy::prelude::*;
use damage::{Health, PartArmour, Strike, WeaponRecord};
use exa_anim::{Animator, BodyPart, Motions};
use exa_formats::races::Natural;
use exa_physics::body::BODY_HZ;
use exa_physics::body::{GoLimp, Shove};
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
    pub health: Health,
    /// The race's natural armour, under worn items.
    pub natural: Option<Natural>,
    /// Armour layers per body part (bones 1..18), set when dressed.
    pub armour: [PartArmour; 18],
    /// Dead.
    pub down: bool,
}

impl Default for Fighter {
    fn default() -> Self {
        Self::of_race(1.0, None)
    }
}

impl Fighter {
    /// A fighter of a race with health factor `health` and natural armour.
    pub fn of_race(health: f32, natural: Option<Natural>) -> Self {
        Self {
            health: Health::of_race(health),
            natural,
            armour: damage::armour_layers(natural.as_ref(), &[]),
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
    /// The weapon's stats.
    pub record: WeaponRecord,
    pub tip_last: Option<Vec3>,
    /// Grip and tip last frame, for sweeping the blade between frames.
    pub swept_from: Option<(Vec3, Vec3)>,
    pub tip_speed: f32,
    /// Tip velocity, m/s.
    pub tip_velocity: Vec3,
    /// Grip position last frame.
    pub grip_last: Vec3,
}

impl Blade {
    /// A blade at rest in a fighter's hand.
    pub fn new(wielder: Entity, length: f32, record: WeaponRecord) -> Self {
        Self {
            wielder,
            length,
            record,
            tip_last: None,
            swept_from: None,
            tip_speed: 0.0,
            tip_velocity: Vec3::ZERO,
            grip_last: Vec3::ZERO,
        }
    }
}

/// Positions a blade sweep is tested at between two frames.
const SWEEP_STEPS: usize = 6;

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
    pub point: Vec3,
    /// Velocity of the blade tip, m/s.
    pub velocity: Vec3,
    pub weapon: WeaponRecord,
    pub strike: Strike,
}

const PART_RADIUS: f32 = 0.16;
const MIN_HIT_SPEED: f32 = 1.5;
/// Attack window, as clip progress.
const ACTIVE_FROM: f32 = 0.35;
const ACTIVE_TO: f32 = 0.85;
/// Share of the blade's velocity a struck part takes on contact.
const CONTACT_SHARE: f32 = 0.1;
/// Impact above which a blow shoves the victim, and the shove per unit of
/// impact (the original's 10 cm/tick, in m/s).
const PUSH_FROM: f32 = 0.5;
const PUSH_PER_IMPACT: f32 = 10.0 * BODY_HZ / 100.0;
const CM_PER_M: f32 = 100.0;

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
        blade.swept_from = blade.tip_last.map(|last| (blade.grip_last, last));
        blade.tip_velocity = blade.tip_last.map_or(Vec3::ZERO, |last| (tip - last) / dt);
        blade.tip_speed = blade.tip_velocity.length();
        blade.tip_last = Some(tip);
        blade.grip_last = gt.translation();
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
        let (a0, b0) = blade.swept_from.unwrap_or((a, b));
        for (target, fighter) in &fighters {
            if target == blade.wielder || fighter.down || attack.struck.contains(&target) {
                continue;
            }
            let spheres: Vec<(usize, Vec3, f32)> = children
                .iter_descendants(target)
                .filter_map(|e| {
                    let (part, pgt, bounds) = parts.get(e).ok()?;
                    let (c, r) = match bounds {
                        // The enclosing sphere is generous; use most of it.
                        Some(b) => (pgt.transform_point(b.centre), b.radius * 0.75),
                        None => (pgt.translation(), PART_RADIUS),
                    };
                    Some((part.bone, c, r))
                })
                .collect();
            // The first position of the sweep since last frame that meets a
            // body part, and the part nearest the blade there.
            let contact = (1..=SWEEP_STEPS).find_map(|k| {
                let t = k as f32 / SWEEP_STEPS as f32;
                let (sa, sb) = (a0.lerp(a, t), b0.lerp(b, t));
                spheres
                    .iter()
                    .filter_map(|&(bone, c, r)| {
                        let p = closest_on_segment(sa, sb, c);
                        let d = p.distance(c);
                        // The point on the blade as it is now.
                        let along = sa.distance(p) / sa.distance(sb).max(1e-4);
                        (d < r).then_some((d / r, bone, a.lerp(b, along)))
                    })
                    .min_by(|x, y| x.0.total_cmp(&y.0))
                    .map(|(_, bone, p)| (bone, p))
            });
            if let Some((bone, point)) = contact {
                debug!(
                    "blade {a} -> {b} speed {:.2} contact bone {bone} at {point}",
                    blade.tip_speed
                );
                attack.struck.insert(target);
                // Where along the weapon it landed, and how fast that point
                // moved (the weapon turns about the grip).
                let along = gt.affine().inverse().transform_point3(point).y * CM_PER_M;
                let share = (along / (blade.length * CM_PER_M)).clamp(0.0, 1.0);
                let strike = Strike {
                    speed: blade.tip_speed * share * CM_PER_M / BODY_HZ,
                    stab: anim.action == "Stab",
                    overhead: anim.action == "Overhead Swing",
                    clock: progress,
                    along,
                };
                hits.write(Hit {
                    attacker: blade.wielder,
                    target,
                    bone,
                    point,
                    velocity: blade.tip_velocity,
                    weapon: blade.record,
                    strike,
                });
            }
        }
    }
}

/// The original's 32-bit generator, for armour coverage rolls.
#[derive(Default)]
struct Roll(u32);

impl Roll {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        (self.0 as f64 * 2f64.powi(-32)) as f32
    }
}

fn react_to_hits(
    mut commands: Commands,
    mut hits: MessageReader<Hit>,
    mut shoves: MessageWriter<Shove>,
    mut fighters: Query<(Entity, &mut Fighter, &mut Animator, Has<Attacking>)>,
    mut roll: Local<Roll>,
) {
    for hit in hits.read() {
        let Ok((_, mut f, mut anim, _)) = fighters.get_mut(hit.target) else {
            continue;
        };
        let Some(armour) = hit.bone.checked_sub(1).and_then(|i| f.armour.get(i)) else {
            continue;
        };
        let Some(outcome) = damage::resolve(
            &hit.weapon,
            &hit.strike,
            armour,
            hit.bone,
            f.health.max,
            roll.next(),
        ) else {
            continue;
        };
        let killed = f.health.take(&outcome);
        // The blow pushes the struck part, shoves the body when heavy, and
        // dazes it.
        let along = hit.velocity.normalize_or_zero();
        let push = if outcome.impact > PUSH_FROM {
            PUSH_PER_IMPACT * outcome.impact
        } else {
            0.0
        };
        let velocity = hit.velocity * CONTACT_SHARE + along * push;
        shoves.write(Shove {
            body: hit.target,
            bone: hit.bone,
            velocity,
            daze: outcome.daze,
        });
        info!(
            "hit on bone {} at {:.0} cm, {:.1} cm/tick: impact {:.2}, penetration {:.2} \
             (yellow {:.3}, red {:.3} of {:.2})",
            hit.bone,
            hit.strike.along,
            hit.strike.speed,
            outcome.impact,
            outcome.penetration,
            f.health.yellow,
            f.health.red,
            f.health.max
        );
        if killed && !f.down {
            f.down = true;
            info!("fighter {:?} is down", hit.target);
            // The body goes limp and falls under the blow.
            commands.entity(hit.target).insert(GoLimp {
                velocity: velocity * 0.5 + Vec3::Y * 0.3,
                delay: 0,
            });
        } else if !f.down && outcome.penetration + outcome.impact > 0.2 {
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

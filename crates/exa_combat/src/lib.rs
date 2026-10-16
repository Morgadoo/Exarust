//! Combat resolution.
//!
//! A held weapon is a segment from its grip to its tip, an empty hand's fist
//! one across the hand. While its wielder is attacking, the segments of the
//! strikers its attack strikes with ([`attack::strikes`]) are tested
//! against the body parts of other fighters. When a fighter may attack, and
//! with what, is the original's attack executor ([`attack`]).
//! A contact is resolved by the original's damage model ([`damage`]): the
//! weapon's stats, where along it the blow lands and how fast, against the
//! struck part's armour layers. It takes health, shoves and dazes the target,
//! and kills it when its yellow health runs out. A blow that does not kill
//! starts no clip on the target; a target struck while attacking stops its
//! attack and returns to its stance. A fighter's stance and combat
//! readiness are kept by [`stance`]. A blow whose wound is deep enough
//! bleeds on a victim whose race bleeds, seven body ticks later; at the
//! blow it sprays a jet of blood and the weapon that dealt it draws blood
//! ([`wounds`]).

pub mod attack;
pub mod damage;
pub mod stance;
pub mod swing;
pub mod wounds;

use std::collections::HashSet;

use attack::{AttackMotion, Attacker, Slot};
use bevy::prelude::*;
use damage::{Health, PartArmour, Strike, WeaponRecord};
use exa_anim::{Animator, BodyPart, Ease, Motions};
use exa_formats::races::{Natural, Race};
use exa_physics::body::BODY_HZ;
use exa_physics::body::{BodyTicks, CharacterTicks, GoLimp, Shove};
use exa_world::PartBounds;
use wounds::{BladeBlood, WoundQueue, frame_tetra, tetra_weights};

pub struct CombatPlugin;

impl Plugin for CombatPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Hit>()
            .add_plugins(stance::StancePlugin)
            .add_message::<wounds::Bleed>()
            .add_message::<wounds::Spray>()
            // The attack branch of the body tick, before the clips advance.
            .add_systems(
                FixedUpdate,
                attack::stamp_attack_ticks.before(CharacterTicks),
            )
            .add_systems(
                PostUpdate,
                // Wounds struck this frame wait from the next tick on.
                (
                    wounds::count_down_wounds,
                    track_blades,
                    detect_hits,
                    react_to_hits,
                )
                    .chain()
                    .in_set(CombatSystems)
                    .after(TransformSystems::Propagate),
            );
    }
}

/// The combat's frame systems: wounds counted down, blades tracked, hits
/// found and dealt. In `PostUpdate`, after transforms propagate.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct CombatSystems;

/// A character that can fight and be hurt (on the body model root).
#[derive(Component, Debug, Clone)]
#[require(WoundQueue, Attacker)]
pub struct Fighter {
    pub health: Health,
    /// The race's natural armour, under worn items.
    pub natural: Option<Natural>,
    /// Armour layers per body part (bones 1..18), set when dressed.
    pub armour: [PartArmour; 18],
    /// Dead.
    pub down: bool,
    /// Combat readiness, 0..=1: it rises standing in the combat idle and
    /// halves the wounds a blow deals at 1 ([`stance::readiness_step`]).
    pub readiness: f32,
    /// Its race bleeds: its wounds paint blood (`races.rdb` flag bit 0).
    pub bleeds: bool,
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
            readiness: 0.0,
            bleeds: true,
        }
    }

    /// A fighter of `race`: its health factor, natural armour and whether
    /// it bleeds.
    pub fn of_record(race: &Race) -> Self {
        Self {
            bleeds: race.bleeds(),
            ..Self::of_race(race.health, Some(race.natural))
        }
    }
}

/// A striker's segment along its entity's local +Y: a held weapon's, on the
/// entity holding the weapon model, or a fist's, across the hand.
#[derive(Component, Debug, Clone)]
pub struct Blade {
    /// Fighter (body model root) wielding it.
    pub wielder: Entity,
    /// The held item it is: the first (a weapon, the right fist) or the
    /// second (the left fist).
    pub slot: Slot,
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
    /// A blade at rest in a fighter's hand, its first held item.
    pub fn new(wielder: Entity, length: f32, record: WeaponRecord) -> Self {
        Self {
            wielder,
            slot: Slot::First,
            length,
            record,
            tip_last: None,
            swept_from: None,
            tip_speed: 0.0,
            tip_velocity: Vec3::ZERO,
            grip_last: Vec3::ZERO,
        }
    }

    /// The same striker as the held item in `slot`.
    pub fn in_slot(self, slot: Slot) -> Self {
        Self { slot, ..self }
    }
}

/// Positions a blade sweep is tested at between two frames.
const SWEEP_STEPS: usize = 6;

/// Attack in progress on a fighter: its motion, the fighter's weapon class
/// (which picks the hand that strikes) and which targets it has already
/// struck.
#[derive(Component, Debug)]
pub struct Attacking {
    pub motion: AttackMotion,
    pub weapon_class: u8,
    pub struck: HashSet<Entity>,
}

impl Attacking {
    pub fn new(motion: AttackMotion, weapon_class: u8) -> Self {
        Self {
            motion,
            weapon_class,
            struck: HashSet::new(),
        }
    }
}

impl Default for Attacking {
    /// A main-side attack with a one-handed weapon.
    fn default() -> Self {
        Self::new(AttackMotion::Main(0), 0x10)
    }
}

/// A weapon contact.
#[derive(Message, Debug, Clone)]
pub struct Hit {
    pub attacker: Entity,
    /// The body that struck: the blade (the entity holding the weapon
    /// model), or the fist.
    pub striker: Entity,
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

/// A fist meets a part with its knuckles, which lie inside the hand's box;
/// its segment ends on the box's far face, so a contact there is measured
/// this far (cm) inside it (the original ignores contacts at or past a
/// striker's far end).
const KNUCKLES_INSIDE: f32 = 0.1;

/// Starts an attack on a fighter of weapon class `weapon_class`: its clip,
/// ending in `stance`, and its primary striker, the held item in the
/// motion's slot ([`attack::primary_slot`]).
pub fn start_attack(
    commands: &mut Commands,
    fighter: Entity,
    anim: &mut Animator,
    attacker: &mut Attacker,
    motion: AttackMotion,
    weapon_class: u8,
    stance: &str,
) {
    anim.play_once(motion.action(), stance);
    // The original's first-side attacks start with no blend, eased
    // smoothly from where the body is to the clip's first key.
    anim.blended(0, Ease::Smooth);
    attacker.primary = Some(attack::primary_slot(weapon_class, motion));
    commands
        .entity(fighter)
        .insert(Attacking::new(motion, weapon_class));
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
    blades: Query<(Entity, &Blade, &GlobalTransform)>,
    mut attackers: Query<(&Animator, &mut Attacking)>,
    fighters: Query<(Entity, &Fighter)>,
    children: Query<&Children>,
    parts: Query<(&BodyPart, &GlobalTransform, Option<&PartBounds>)>,
    mut hits: MessageWriter<Hit>,
) {
    let Some(motions) = motions else { return };
    for (striker, blade, gt) in &blades {
        let Ok((anim, mut attack)) = attackers.get_mut(blade.wielder) else {
            continue;
        };
        if !attack::strikes(
            blade.slot,
            blade.record.armed(),
            attack.weapon_class,
            attack.motion,
        ) {
            continue;
        }
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
                let mut along = gt.affine().inverse().transform_point3(point).y * CM_PER_M;
                if !blade.record.armed() {
                    along = along.min(blade.length * CM_PER_M - KNUCKLES_INSIDE);
                }
                let share = (along / (blade.length * CM_PER_M)).clamp(0.0, 1.0);
                let strike = Strike {
                    speed: blade.tip_speed * share * CM_PER_M / BODY_HZ,
                    stab: attack.motion == AttackMotion::Stab,
                    overhead: attack.motion == AttackMotion::Overhead,
                    clock: progress,
                    along,
                };
                hits.write(Hit {
                    attacker: blade.wielder,
                    striker,
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

/// The looping stance a fighter returns to: the follow-up of the clip it is
/// playing once, else its moveset's idle.
fn stance_of(anim: &Animator) -> String {
    match &anim.then {
        Some(then) => then.clone(),
        None if anim.moveset == "Default" => "Idle".into(),
        None => "Cmbt - Idle".into(),
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn react_to_hits(
    mut commands: Commands,
    mut hits: MessageReader<Hit>,
    mut shoves: MessageWriter<Shove>,
    mut sprays: MessageWriter<wounds::Spray>,
    mut fighters: Query<(
        Entity,
        &mut Fighter,
        &mut Animator,
        Has<Attacking>,
        Option<&mut WoundQueue>,
    )>,
    strikers: Query<&GlobalTransform>,
    mut blades: Query<&mut BladeBlood>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: Local<Roll>,
) {
    let tick = ticks.map_or(0, |t| t.0);
    for hit in hits.read() {
        let Ok((_, mut f, mut anim, attacking, queue)) = fighters.get_mut(hit.target) else {
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
            f.readiness,
            roll.next(),
        ) else {
            continue;
        };
        let killed = f.health.take(&outcome);
        // A deep enough wound on a race that bleeds sprays a jet at once
        // and waits on the victim to bleed, anchored to the striker; the
        // killing blow too. The weapon that dealt it draws blood at once,
        // even when the wound is dropped.
        if f.bleeds && wounds::bleeds(outcome.wound) {
            let strength = wounds::spray_strength(outcome.wound);
            sprays.write(wounds::Spray {
                body: hit.target,
                bone: hit.bone,
                contact: hit.point,
                swing: hit.velocity,
                droplets: wounds::jet_droplets(strength),
            });
            if let Some(mut queue) = queue {
                let weights = strikers
                    .get(hit.striker)
                    .ok()
                    .and_then(|g| tetra_weights(hit.point, &frame_tetra(&g.affine())));
                if queue.push(hit.striker, weights, hit.point, outcome.wound) {
                    debug!("wound R {:.3} queued on {:?}", outcome.wound, hit.target);
                } else {
                    info!(
                        "wound R {:.3} dropped: {:?} has seven waiting",
                        outcome.wound, hit.target
                    );
                }
            }
            if let Ok(mut blood) = blades.get_mut(hit.striker) {
                blood.draw(outcome.wound);
                info!(
                    "blade {:?} draws blood from wound R {:.3}: {:.3}",
                    hit.striker, outcome.wound, blood.0
                );
            }
        }
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
            "hit on {:?} bone {} at tick {tick} by {}, {:.0} cm, {:.1} cm/tick: impact {:.2}, \
             penetration {:.2} at readiness {:.2}, wound {:.3} (yellow {:.3}, red {:.3} of {:.2})",
            hit.target,
            hit.bone,
            if hit.weapon.armed() {
                "a weapon"
            } else {
                "a fist"
            },
            hit.strike.along,
            hit.strike.speed,
            outcome.impact,
            outcome.penetration,
            f.readiness,
            outcome.wound,
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
        }
        // A blow that does not kill starts no clip on the struck fighter:
        // its stance changes come from its balance, not from the blow.
        if attacking {
            // Stand-in for the original's swing stall until the swing
            // strength exists: there the daze drains a struck attacker's
            // swing and returns it to its stance within the swing's time;
            // here the attack stops at once and deals no further hit.
            if !f.down {
                let stance = stance_of(&anim);
                anim.play_loop(&stance);
            }
            commands.entity(hit.target).remove::<Attacking>();
        }
    }
    for (e, _, anim, attacking, _) in &fighters {
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
    use bevy::ecs::message::Messages;
    use damage::{DEATH, Outcome};

    #[test]
    fn segment_projection() {
        let p = closest_on_segment(Vec3::ZERO, Vec3::Y, Vec3::new(1.0, 0.5, 0.0));
        assert_eq!(p, Vec3::new(0.0, 0.5, 0.0));
        let end = closest_on_segment(Vec3::ZERO, Vec3::Y, Vec3::new(0.0, 3.0, 0.0));
        assert_eq!(end, Vec3::Y);
    }

    /// An app running only the blow reactions.
    fn reactions() -> App {
        let mut app = App::new();
        app.add_message::<Hit>()
            .add_message::<Shove>()
            .add_message::<wounds::Spray>()
            .add_systems(Update, react_to_hits);
        app
    }

    /// The damage tests' sword cut: 20 cm a tick, 70 cm along the blade,
    /// late in the swing.
    fn cut() -> Strike {
        Strike {
            speed: 20.0,
            stab: false,
            overhead: false,
            clock: 0.8,
            along: 70.0,
        }
    }

    /// What the cut does to bare flesh on `bone` of a human, with the
    /// reactions' first coverage roll.
    fn outcome(bone: usize) -> Outcome {
        let roll = Roll::default().next();
        damage::resolve(
            &damage::sword_a(),
            &cut(),
            &PartArmour::default(),
            bone,
            0.25,
            0.0,
            roll,
        )
        .unwrap()
    }

    /// Cuts `target` on `bone` and runs one update.
    fn strike(app: &mut App, target: Entity, bone: usize) {
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker: Entity::PLACEHOLDER,
            target,
            bone,
            point: Vec3::ZERO,
            velocity: Vec3::new(3.0, 0.0, 0.0),
            weapon: damage::sword_a(),
            strike: cut(),
        });
        app.update();
    }

    fn shoves(app: &App) -> Vec<Shove> {
        let messages = app.world().resource::<Messages<Shove>>();
        messages.get_cursor().read(messages).copied().collect()
    }

    /// A sword cut that wounds but does not kill starts no clip: a bare
    /// forearm (13), chest (9), head (10) or foot (4) of a human in its
    /// stance, each over the 0.2 of penetration and impact that used to
    /// start Head Pain or Rebalance, leaves it in its idle, shoved once on
    /// that part. One getting up keeps playing its get-up.
    #[test]
    fn a_wound_starts_no_clip() {
        for bone in [13, 9, 10, 4] {
            let o = outcome(bone);
            assert!(o.penetration + o.impact > 0.2, "bone {bone}: {o:?}");
            let mut health = Health::of_race(1.0);
            assert!(!health.take(&o), "bone {bone}: the cut kills");
            let mut app = reactions();
            let fighter = app
                .world_mut()
                .spawn((Fighter::default(), Animator::new("Default", "Idle")))
                .id();
            strike(&mut app, fighter, bone);
            let anim = app.world().get::<Animator>(fighter).unwrap();
            assert_eq!(
                (anim.action.as_str(), anim.looping, anim.then.as_deref()),
                ("Idle", true, None),
                "bone {bone}"
            );
            assert_eq!(anim.time, 0.0, "bone {bone}");
            let f = app.world().get::<Fighter>(fighter).unwrap();
            assert!(f.health.yellow < 0.25, "bone {bone}: {:?}", f.health);
            assert!(f.health.yellow >= DEATH && !f.down, "bone {bone}");
            assert_eq!(f.health, health, "bone {bone}");
            let s = shoves(&app);
            assert_eq!(s.len(), 1, "bone {bone}");
            assert_eq!((s[0].body, s[0].bone), (fighter, bone));
            assert_eq!(s[0].daze, o.daze, "bone {bone}");
            assert!(app.world().get::<GoLimp>(fighter).is_none());
        }
        let mut app = reactions();
        let mut anim = Animator::new("Default", "Idle");
        anim.play_once("Get up", "Idle");
        let fighter = app.world_mut().spawn((Fighter::default(), anim)).id();
        strike(&mut app, fighter, 13);
        let anim = app.world().get::<Animator>(fighter).unwrap();
        assert_eq!(
            (anim.action.as_str(), anim.looping, anim.then.as_deref()),
            ("Get up", false, Some("Idle"))
        );
    }

    /// A fighter cut on the forearm in the middle of its attack stops it:
    /// it no longer counts as attacking, so its blade deals no further hit,
    /// and it is back in its combat stance.
    #[test]
    fn a_wound_stops_a_swing() {
        let mut app = reactions();
        let mut anim = Animator::new("One Handed", "Cmbt - Idle");
        anim.play_once("Attack Mid", "Cmbt - Idle");
        anim.time = 12.0;
        let fighter = app
            .world_mut()
            .spawn((Fighter::default(), anim, Attacking::default()))
            .id();
        strike(&mut app, fighter, 13);
        assert!(app.world().get::<Attacking>(fighter).is_none());
        let anim = app.world().get::<Animator>(fighter).unwrap();
        assert_eq!(
            (anim.action.as_str(), anim.looping, anim.then.as_deref()),
            ("Cmbt - Idle", true, None)
        );
        assert_eq!(anim.time, 0.0);
        let f = app.world().get::<Fighter>(fighter).unwrap();
        assert!(!f.down);
        assert_eq!(shoves(&app).len(), 1);
    }

    /// A cut that takes yellow health under 0.01 still sends the body limp
    /// under the blow, and ends an attack it was making.
    #[test]
    fn a_killing_wound_still_goes_limp() {
        for attacking in [false, true] {
            let mut app = reactions();
            let mut fighter = Fighter::default();
            fighter.health.yellow = 0.011;
            let mut anim = Animator::new("One Handed", "Cmbt - Idle");
            let id = if attacking {
                anim.play_once("Attack Mid", "Cmbt - Idle");
                app.world_mut()
                    .spawn((fighter, anim, Attacking::default()))
                    .id()
            } else {
                app.world_mut().spawn((fighter, anim)).id()
            };
            strike(&mut app, id, 9);
            let f = app.world().get::<Fighter>(id).unwrap();
            assert!(f.down && f.health.yellow == 0.0, "{:?}", f.health);
            assert!(app.world().get::<GoLimp>(id).is_some());
            assert!(app.world().get::<Attacking>(id).is_none());
            assert_eq!(shoves(&app).len(), 1);
        }
    }

    /// An app running the wound countdown and the blow reactions, with a
    /// body tick clock.
    fn bleeding() -> App {
        let mut app = App::new();
        app.add_message::<Hit>()
            .add_message::<Shove>()
            .add_message::<wounds::Bleed>()
            .add_message::<wounds::Spray>()
            .init_resource::<exa_physics::body::BodyTicks>()
            .add_systems(Update, (wounds::count_down_wounds, react_to_hits).chain());
        app.update();
        app
    }

    /// The cut on `bone` of `target`, dealt by `striker` at `point`.
    fn cut_by(app: &mut App, striker: Entity, target: Entity, bone: usize, point: Vec3) {
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker,
            target,
            bone,
            point,
            velocity: Vec3::new(3.0, 0.0, 0.0),
            weapon: damage::sword_a(),
            strike: cut(),
        });
    }

    /// One body tick and the frame after it; the wounds that bled.
    fn tick(app: &mut App) -> Vec<wounds::Bleed> {
        app.world_mut()
            .resource_mut::<exa_physics::body::BodyTicks>()
            .0 += 1;
        app.update();
        let messages = app.world().resource::<Messages<wounds::Bleed>>();
        messages.iter_current_update_messages().copied().collect()
    }

    fn waiting(app: &App, fighter: Entity) -> usize {
        app.world()
            .get::<WoundQueue>(fighter)
            .unwrap()
            .pending()
            .len()
    }

    /// A human cut on the bare chest (R = 0.457, over 0.05) bleeds seven
    /// body ticks later, not before, with the radius 30·R + 10 cm (23.7 cm)
    /// and the travel of the blade's struck point since the blow.
    #[test]
    fn a_wound_bleeds_seven_ticks_after_the_blow() {
        let o = outcome(9);
        assert!((o.wound - 0.457).abs() < 1e-3, "{o:?}");
        assert!((o.red - 0.75 * o.wound).abs() < 1e-6);
        let mut app = bleeding();
        let fighter = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        // The blade lies along +Y from its grip, turned; the blow lands
        // 70 cm along it.
        let blade = Transform::from_xyz(0.2, 1.0, 0.5).with_rotation(Quat::from_rotation_z(-1.2));
        let striker = app.world_mut().spawn(GlobalTransform::from(blade)).id();
        let contact = blade.transform_point(Vec3::Y * 0.7);
        cut_by(&mut app, striker, fighter, 9, contact);
        app.update();
        let queued = app.world().get::<WoundQueue>(fighter).unwrap().pending()[0];
        assert_eq!((queued.striker, queued.delay), (striker, 7));
        assert_eq!((queued.contact, queued.size), (contact, o.wound));
        // The blade swings on: by the 7th tick it has turned further and
        // dropped 10 cm.
        let on = blade
            .with_rotation(Quat::from_rotation_z(-1.6))
            .with_translation(Vec3::new(0.2, 0.9, 0.5));
        *app.world_mut().get_mut::<GlobalTransform>(striker).unwrap() = on.into();
        for t in 1..7 {
            assert!(tick(&mut app).is_empty(), "tick {t}");
        }
        let bled = tick(&mut app);
        assert_eq!(bled.len(), 1);
        let b = bled[0];
        assert_eq!((b.body, b.contact), (fighter, contact));
        assert!((b.radius - (0.3 * o.wound + 0.1)).abs() < 1e-6);
        assert!((b.radius - 0.237).abs() < 1e-3);
        let moved = on.transform_point(Vec3::Y * 0.7) - contact;
        assert!(b.travel.distance(moved) < 1e-4, "{:?} {moved:?}", b.travel);
        assert_eq!(waiting(&app, fighter), 0);
        assert!(tick(&mut app).is_empty());
    }

    /// Nothing bleeds on a race that does not: a skeleton (flags 0), cut as
    /// deep, keeps no wound; a human of the same record bleeds.
    #[test]
    fn skeletons_do_not_bleed() {
        let race = |flags: u32| Race {
            id: 2,
            name: "Skeleton".into(),
            flags,
            stem: "Skel".into(),
            movesets: ["Default".into(), "Skeleton".into()],
            health: 1.0,
            natural: Natural::default(),
            body: exa_formats::races::RaceBody::FALLBACK,
            stats: exa_formats::races::RaceStats::FALLBACK,
        };
        let skeleton = Fighter::of_record(&race(0));
        assert!(!skeleton.bleeds);
        assert!(Fighter::of_record(&race(0x0F)).bleeds);
        assert!(Fighter::default().bleeds);
        let mut app = bleeding();
        let bones = app
            .world_mut()
            .spawn((skeleton, Animator::new("Default", "Idle")))
            .id();
        cut_by(&mut app, Entity::PLACEHOLDER, bones, 9, Vec3::ZERO);
        app.update();
        assert!(app.world().get::<Fighter>(bones).unwrap().health.red < 0.25);
        assert_eq!(waiting(&app, bones), 0);
        for _ in 0..8 {
            assert!(tick(&mut app).is_empty());
        }
    }

    /// The threshold is on R, before the red bar's 0.75: a slow cut on
    /// the bare hand at 4 cm a tick (R 0.045) does not bleed, at 6 cm a
    /// tick (R 0.064, red 0.048) it does.
    #[test]
    fn shallow_wounds_do_not_bleed() {
        for (speed, size, bleeds) in [(4.0, 0.0454, false), (6.0, 0.0640, true)] {
            let strike = Strike { speed, ..cut() };
            let roll = Roll::default().next();
            let o = damage::resolve(
                &damage::sword_a(),
                &strike,
                &PartArmour::default(),
                17,
                0.25,
                0.0,
                roll,
            )
            .unwrap();
            assert!((o.wound - size).abs() < 1e-4, "{o:?}");
            assert!(f64::from(o.red) < wounds::BLEED_FROM);
            let mut app = bleeding();
            let id = app
                .world_mut()
                .spawn((Fighter::default(), Animator::new("Default", "Idle")))
                .id();
            app.world_mut().write_message(Hit {
                attacker: Entity::PLACEHOLDER,
                striker: Entity::PLACEHOLDER,
                target: id,
                bone: 17,
                point: Vec3::ZERO,
                velocity: Vec3::X,
                weapon: damage::sword_a(),
                strike,
            });
            app.update();
            assert_eq!(waiting(&app, id), usize::from(bleeds), "{speed} cm/tick");
        }
    }

    /// The killing blow bleeds on the corpse; a wound struck while seven
    /// wait is dropped.
    #[test]
    fn the_fallen_bleed_and_seven_wait_at_most() {
        let mut app = bleeding();
        let mut dying = Fighter::default();
        dying.health.yellow = 0.011;
        let id = app
            .world_mut()
            .spawn((dying, Animator::new("Default", "Idle")))
            .id();
        cut_by(&mut app, Entity::PLACEHOLDER, id, 9, Vec3::ZERO);
        app.update();
        assert!(app.world().get::<Fighter>(id).unwrap().down);
        let bled: Vec<_> = (0..7).flat_map(|_| tick(&mut app)).collect();
        assert_eq!(bled.len(), 1);
        assert_eq!((bled[0].body, bled[0].travel), (id, Vec3::ZERO));

        // A sturdy fighter takes nine cuts in one frame: seven wait, and
        // they bleed one a tick from the 7th tick on.
        let sturdy = app
            .world_mut()
            .spawn((
                Fighter::of_race(100.0, None),
                Animator::new("Default", "Idle"),
            ))
            .id();
        for i in 0..9 {
            cut_by(&mut app, Entity::PLACEHOLDER, sturdy, 9, Vec3::X * i as f32);
        }
        app.update();
        assert_eq!(waiting(&app, sturdy), 7);
        let per_tick: Vec<Vec<Vec3>> = (0..14)
            .map(|_| tick(&mut app).iter().map(|b| b.contact).collect())
            .collect();
        for (t, contacts) in per_tick.iter().enumerate() {
            match t {
                0..6 => assert!(contacts.is_empty(), "tick {}", t + 1),
                6..13 => assert_eq!(contacts, &[Vec3::X * (t - 6) as f32], "tick {}", t + 1),
                _ => assert!(contacts.is_empty()),
            }
        }
    }

    /// The blade that deals a bleeding wound draws half its R at the blow:
    /// 0.2285 from the chest cut (R 0.457), again when the wound is dropped
    /// from a full queue; nothing from a wound that does not bleed (a
    /// skeleton's, or R 0.045 on a hand), and a fist has no blood to draw.
    #[test]
    fn blades_draw_blood_from_bleeding_wounds() {
        let mut app = bleeding();
        let human = app
            .world_mut()
            .spawn((
                Fighter::of_race(100.0, None),
                Animator::new("Default", "Idle"),
            ))
            .id();
        let sword = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, BladeBlood::default()))
            .id();
        let blood = |app: &App| app.world().get::<BladeBlood>(sword).unwrap().0;
        cut_by(&mut app, sword, human, 9, Vec3::ZERO);
        app.update();
        // At the blow, before the wound bleeds seven ticks later.
        assert_eq!(waiting(&app, human), 1);
        assert!((blood(&app) - 0.5 * outcome(9).wound).abs() < 1e-6);
        assert!((blood(&app) - 0.2285).abs() < 1e-3);
        // Seven more in one frame: six wait (one is waiting already), two
        // are dropped, and every one adds its blood, up to 1.
        for _ in 0..7 {
            cut_by(&mut app, sword, human, 9, Vec3::ZERO);
        }
        app.update();
        assert_eq!(waiting(&app, human), 7);
        assert_eq!(blood(&app), 1.0);

        let mut app = bleeding();
        let skeleton = app
            .world_mut()
            .spawn((
                Fighter {
                    bleeds: false,
                    ..Fighter::default()
                },
                Animator::new("Default", "Idle"),
            ))
            .id();
        let human = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        let sword = app
            .world_mut()
            .spawn((GlobalTransform::IDENTITY, BladeBlood::default()))
            .id();
        cut_by(&mut app, sword, skeleton, 9, Vec3::ZERO);
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker: sword,
            target: human,
            bone: 17,
            point: Vec3::ZERO,
            velocity: Vec3::X,
            weapon: damage::sword_a(),
            strike: Strike {
                speed: 4.0,
                ..cut()
            },
        });
        app.update();
        assert_eq!(app.world().get::<BladeBlood>(sword).unwrap().0, 0.0);
        assert!(app.world().get::<Fighter>(human).unwrap().health.red < 0.25);
        // A fist: the struck bleeds, the hand draws nothing.
        let fist = app.world_mut().spawn(GlobalTransform::IDENTITY).id();
        cut_by(&mut app, fist, human, 9, Vec3::ZERO);
        app.update();
        assert_eq!(waiting(&app, human), 1);
        assert!(app.world().get::<BladeBlood>(fist).is_none());
    }

    /// The jets sent this frame.
    fn sprays(app: &App) -> Vec<wounds::Spray> {
        let messages = app.world().resource::<Messages<wounds::Spray>>();
        messages.iter_current_update_messages().copied().collect()
    }

    /// A bleeding blow sprays one jet at the blow, on the struck part at
    /// the contact point, against the swing: the chest cut (R 0.457) sends
    /// 16 droplets, a jet for every cut even when the wound is dropped from
    /// a full queue; a skeleton's cut and a cut on the hand at 4 cm a tick
    /// (R 0.045) spray nothing.
    #[test]
    fn bleeding_blows_spray_at_the_blow() {
        let mut app = bleeding();
        let human = app
            .world_mut()
            .spawn((
                Fighter::of_race(100.0, None),
                Animator::new("Default", "Idle"),
            ))
            .id();
        let contact = Vec3::new(0.1, 1.3, 0.2);
        cut_by(&mut app, Entity::PLACEHOLDER, human, 9, contact);
        app.update();
        let jets = sprays(&app);
        assert_eq!(jets.len(), 1);
        let jet = jets[0];
        assert_eq!((jet.body, jet.bone, jet.contact), (human, 9, contact));
        assert_eq!(jet.swing, Vec3::new(3.0, 0.0, 0.0));
        assert_eq!(jet.droplets, 16);
        assert_eq!(
            jet.droplets,
            wounds::jet_droplets(wounds::spray_strength(outcome(9).wound))
        );
        // Not again on the ticks that follow, when the wound bleeds.
        let bled: Vec<_> = (0..7).flat_map(|_| tick(&mut app)).collect();
        assert_eq!(bled.len(), 1);
        assert!(sprays(&app).is_empty());
        // Eight cuts in a frame: seven wait, one is dropped, all spray.
        for _ in 0..8 {
            cut_by(&mut app, Entity::PLACEHOLDER, human, 9, contact);
        }
        app.update();
        assert_eq!(waiting(&app, human), 7);
        assert_eq!(sprays(&app).len(), 8);

        let skeleton = app
            .world_mut()
            .spawn((
                Fighter {
                    bleeds: false,
                    ..Fighter::default()
                },
                Animator::new("Default", "Idle"),
            ))
            .id();
        cut_by(&mut app, Entity::PLACEHOLDER, skeleton, 9, contact);
        app.world_mut().write_message(Hit {
            attacker: Entity::PLACEHOLDER,
            striker: Entity::PLACEHOLDER,
            target: human,
            bone: 17,
            point: contact,
            velocity: Vec3::X,
            weapon: damage::sword_a(),
            strike: Strike {
                speed: 4.0,
                ..cut()
            },
        });
        app.update();
        assert!(sprays(&app).is_empty());
    }

    /// A synthetic motion set: the "Default" moveset's main-side and
    /// other-side attacks, 100 frames each.
    fn punches() -> Motions {
        use exa_formats::rsq::{Action, Clip, Key, MotionSet, Moveset};
        let key = |time| Key {
            time,
            root: [0.0; 3],
            rotations: Vec::new(),
            flags: Vec::new(),
        };
        let clip = Clip {
            mode: 0,
            keys: vec![key(0.0), key(100.0)],
        };
        let actions = ["Attack Mid", "Attack Combo 1", "Overhead Swing"];
        let set = MotionSet {
            bone_count: 0,
            actions: actions
                .iter()
                .map(|n| Action {
                    name: (*n).into(),
                    params: [0; 2],
                })
                .collect(),
            movesets: vec![Moveset {
                name: "Default".into(),
                parent: String::new(),
                clips: vec![Some(clip); actions.len()],
            }],
        };
        Motions(std::sync::Arc::new(set), std::sync::Arc::default(), 0.0)
    }

    /// The human fist's record, 17.66 cm across the hand.
    fn fist() -> WeaponRecord {
        use exa_formats::weapon_record::{Holder, Lengths, Section};
        WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(-8.0, 9.66),
            &Holder::AVERAGE,
        )
    }

    /// An unarmed puncher halfway through `motion` with both fists driven
    /// into a target's chest (a 20 cm sphere 1 m ahead, at chest height),
    /// knuckles first at 4 m/s; the hits detected.
    fn punch(motion: AttackMotion, weapon_class: u8) -> (Vec<Hit>, [Entity; 2]) {
        let mut app = App::new();
        app.add_message::<Hit>()
            .insert_resource(punches())
            .add_systems(Update, detect_hits);
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        anim.play_once(motion.action(), "Cmbt - Idle");
        anim.time = 50.0;
        let puncher = app
            .world_mut()
            .spawn((
                Fighter::default(),
                anim,
                Attacking::new(motion, weapon_class),
            ))
            .id();
        let chest = Vec3::new(0.0, 1.4, 1.0);
        let target = app.world_mut().spawn(Fighter::default()).id();
        app.world_mut().spawn((
            BodyPart {
                bone: 9,
                rest: Transform::IDENTITY,
            },
            GlobalTransform::from_translation(chest),
            PartBounds {
                centre: Vec3::ZERO,
                radius: 0.2 / 0.75,
                half_extents: Vec3::splat(0.15),
            },
            ChildOf(target),
        ));
        // Each fist's segment runs from the wrist along +Z (local +Y turned
        // forward), its knuckles 5 cm into the chest's sphere.
        let length = fist().length / CM_PER_M;
        let fists = [(Slot::First, -0.05), (Slot::Second, 0.05)].map(|(slot, x)| {
            let wrist = Vec3::new(x, 1.4, 1.0 - 0.2 + 0.05 - length);
            let at = Transform::from_translation(wrist)
                .with_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z));
            let mut blade = Blade::new(puncher, length, fist()).in_slot(slot);
            blade.tip_speed = 4.0;
            blade.tip_velocity = Vec3::Z * 4.0;
            app.world_mut()
                .spawn((blade, GlobalTransform::from(at)))
                .id()
        });
        app.update();
        let messages = app.world().resource::<Messages<Hit>>();
        (
            messages.iter_current_update_messages().cloned().collect(),
            fists,
        )
    }

    /// Unarmed, the main side and the overhead strike with the right fist
    /// only, the other side with the left; the struck point is the
    /// knuckles, 0.1 cm inside the hand's 17.66 cm, so the blow counts. A
    /// held weapon's class makes the fists no strikers at all.
    #[test]
    fn fists_strike_with_the_motions_hand() {
        for (motion, hand) in [
            (AttackMotion::Main(0), 0),
            (AttackMotion::Overhead, 0),
            (AttackMotion::Other(0), 1),
        ] {
            let (hits, fists) = punch(motion, 0x80);
            assert_eq!(hits.len(), 1, "{motion:?}");
            let hit = &hits[0];
            assert_eq!(hit.striker, fists[hand], "{motion:?}");
            assert_eq!(hit.bone, 9);
            assert!(!hit.weapon.armed());
            assert!(
                (hit.strike.along - (17.66 - 0.1)).abs() < 1e-3,
                "{:?}",
                hit.strike
            );
            assert_eq!(hit.strike.overhead, motion == AttackMotion::Overhead);
            let o = damage::resolve(
                &hit.weapon,
                &hit.strike,
                &PartArmour::default(),
                hit.bone,
                0.25,
                0.0,
                0.5,
            );
            assert!(o.is_some_and(|o| o.yellow > 0.0), "{o:?}");
        }
        let (hits, _) = punch(AttackMotion::Main(0), 0x10);
        assert!(hits.is_empty());
    }

    /// A punch bleeds on the one struck, never on the puncher's own fist or
    /// body.
    #[test]
    fn fists_stay_clean() {
        let mut app = bleeding();
        let puncher = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        let fist = app
            .world_mut()
            .spawn((GlobalTransform::from_xyz(0.0, 1.4, 0.4), ChildOf(puncher)))
            .id();
        let struck = app
            .world_mut()
            .spawn((Fighter::default(), Animator::new("Default", "Idle")))
            .id();
        cut_by(&mut app, fist, struck, 9, Vec3::new(0.0, 1.4, 0.5));
        app.update();
        assert_eq!((waiting(&app, struck), waiting(&app, puncher)), (1, 0));
        let bled: Vec<_> = (0..7).flat_map(|_| tick(&mut app)).collect();
        assert_eq!(bled.len(), 1);
        assert_eq!(bled[0].body, struck);
    }
}

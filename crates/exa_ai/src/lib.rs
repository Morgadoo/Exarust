//! NPC behaviour. For now a duellist: once the fight starts it enters
//! combat in its guard, faces the target, closes to striking distance and
//! attacks at intervals with varied strikes, guarding while the target is
//! far and standing in its combat idle near it. NPCs act through the same
//! pieces as the player (body velocity, animator, stance, attacks).

use bevy::prelude::*;
use exa_anim::Animator;
use exa_combat::attack::{self, AttackMotion, Attacker};
use exa_combat::stance::{self, Stance};
use exa_combat::{Attacking, Blade, Fighter};
use exa_physics::CharacterBody;
use exa_physics::body::{BodyTicks, Fallen, MAX_ENERGY, Ragdoll};

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
#[require(Stance)]
pub struct Duellist {
    pub target: Entity,
    pub cooldown: f32,
    pub next_strike: usize,
    /// The fight has started: until then the duellist stands idle.
    pub engaged: bool,
    /// It stands its ground: it faces its target and takes its stance, but
    /// neither walks nor strikes (a debug aid for still comparisons).
    pub holds_ground: bool,
    /// It has entered combat (taken its guard).
    in_combat: bool,
}

impl Duellist {
    /// A duellist of `target`, waiting for the fight to start, its first
    /// strike 2 s after it comes within striking distance.
    pub fn new(target: Entity) -> Self {
        Self {
            target,
            cooldown: 2.0,
            next_strike: 0,
            engaged: false,
            holds_ground: false,
            in_combat: false,
        }
    }
}

/// The duellist strikes from this far (m, between the characters). It is
/// Exarust's stand-in for the original's reach: the arm follows its clip
/// through link muscles, which damp its swing (about 15–20 cm less reach
/// than the clip's pose at the original's swing speed), and the strike
/// lands only from this close.
const STRIKE_RANGE: f32 = 1.15;
const APPROACH_SPEED: f32 = 1.2;
const STRIKE_INTERVAL: f32 = 2.2;
const STRIKES: [AttackMotion; 3] = [
    AttackMotion::Main(0),
    AttackMotion::Main(2),
    AttackMotion::Main(1),
];
/// Exarust's interim footwork clip (the original's AI steps in its stance).
const COMBAT_WALK: &str = "Combat Walk";

/// A fighter's reach (m), as the original's equip refresh sets it: its
/// longest held weapon's record reach (60 cm × its size plus the weapon's
/// sweet spot), else (its fists) its race's unarmed reach (the human's
/// 50 cm).
fn reach_of(fighter: Entity, blades: &Query<&Blade>) -> f32 {
    blades
        .iter()
        .filter(|b| b.wielder == fighter && b.record.armed())
        .map(|b| b.record.reach_metres())
        .reduce(f32::max)
        .unwrap_or(stance::UNARMED_REACH)
}

/// A fighter's weapon class as its strikers make it: a held weapon's hands
/// bits, else the fists' (the claws' for a clawed race's).
fn weapon_class_of(fighter: Entity, blades: &Query<&Blade>) -> u8 {
    let mine = || blades.iter().filter(|b| b.wielder == fighter);
    if let Some(weapon) = mine().find(|b| b.record.armed()) {
        return (weapon.record.flags & 0x30) as u8;
    }
    if mine().any(|b| b.record.flags & 0x40 != 0) {
        0xC0
    } else {
        0x80
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn duellist(
    mut commands: Commands,
    time: Res<Time>,
    ticks: Option<Res<BodyTicks>>,
    targets: Query<&GlobalTransform>,
    bodies: Query<(&Fighter, &GlobalTransform, Option<&Ragdoll>)>,
    children: Query<&Children>,
    blades: Query<&Blade>,
    mut npcs: Query<(
        Entity,
        &mut Duellist,
        &mut Stance,
        &mut Transform,
        &mut Animator,
        &mut Attacker,
        &ChildOf,
        Option<&Attacking>,
        Has<Fallen>,
    )>,
    mut roots: Query<(&mut CharacterBody, &GlobalTransform)>,
) {
    let now = ticks.map_or(0, |t| t.millis());
    for (entity, mut ai, mut stance, mut t, mut anim, mut attacker, parent, attacking, fallen) in
        &mut npcs
    {
        let Ok((mut body, root_gt)) = roots.get_mut(parent.parent()) else {
            continue;
        };
        body.desired_velocity = Vec3::ZERO;
        let Ok((fighter, gt, ragdoll)) = bodies.get(entity) else {
            continue;
        };
        if fighter.down || fallen || !ai.engaged {
            continue;
        }
        let Ok(target) = targets.get(ai.target) else {
            continue;
        };
        if !ai.in_combat {
            // Entering combat: the guard, from the idle it stood in.
            ai.in_combat = true;
            stance::set_stance(&mut stance, &mut anim, Stance::Defence);
            info!("duellist {entity:?} enters combat: {:?}", *stance);
            continue;
        }
        // The target's body: the fighter at or under the target entity.
        let target_body = std::iter::once(ai.target)
            .chain(children.iter_descendants(ai.target))
            .find_map(|e| bodies.get(e).ok());
        // Its stance by the distance between the chests: the guard at four
        // reaches or more, the combat idle within.
        if let Some((_, tgt, tgt_body)) = target_body {
            let d = stance::chest_point(gt, ragdoll).distance(stance::chest_point(tgt, tgt_body));
            let reach = reach_of(entity, &blades);
            let chosen = stance::duellist_stance(d, reach);
            if *stance != chosen {
                info!("duellist {entity:?} stance: {chosen:?} at {d:.2} m (reach {reach:.2} m)");
                *stance = chosen;
            }
        }
        // Its stance starts with 40 frames of blend without a primary
        // striker, else twice the primary's stance blend, eased smoothly.
        let primary_blend = attacker
            .primary_record(entity, blades.iter())
            .map(|r| r.blend);
        // Leave a downed target alone.
        if target_body.is_some_and(|(f, ..)| f.down) {
            if anim.looping {
                stance::take_ai_stance(&mut anim, *stance, primary_blend);
            }
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
        if dist > STRIKE_RANGE && !ai.holds_ground {
            body.desired_velocity = flat / dist * APPROACH_SPEED;
            anim.play_loop(COMBAT_WALK);
            continue;
        }
        // Standing: its stance starts at once, as the original's AI starts
        // it whenever it is free.
        stance::take_ai_stance(&mut anim, *stance, primary_blend);
        // Its request goes through the original's executor: enough energy
        // and the cooldown since its last attack's last tick.
        let energy = ragdoll
            .and_then(|r| r.drive.as_ref())
            .map_or(MAX_ENERGY, |d| d.energy);
        let ready = attack::has_energy(energy, MAX_ENERGY)
            && attack::execute(
                attack::Request::default(),
                now,
                attacker.last_tick_ms,
                attacker.cooldown_ms(entity, blades.iter()),
                None,
            )
            .is_some();
        if dist <= STRIKE_RANGE && !ai.holds_ground && ai.cooldown <= 0.0 && ready {
            let strike = STRIKES[ai.next_strike % STRIKES.len()];
            ai.next_strike += 1;
            ai.cooldown = STRIKE_INTERVAL;
            exa_combat::start_attack(
                &mut commands,
                entity,
                &mut anim,
                &mut attacker,
                blades.iter(),
                strike,
                weapon_class_of(entity, &blades),
                stance.action(),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An unarmed duellist (reach 0.5 m: it guards from 2 m) on a root at
    /// the origin, its target `distance` m away along +Z.
    fn duel(distance: f32, holds_ground: bool) -> (App, Entity, Entity) {
        let mut app = App::new();
        app.init_resource::<Time>().add_systems(Update, duellist);
        let target = app
            .world_mut()
            .spawn((
                GlobalTransform::from_translation(Vec3::new(0.0, 0.0, distance)),
                Fighter::default(),
            ))
            .id();
        let root = app
            .world_mut()
            .spawn((CharacterBody::default(), GlobalTransform::IDENTITY))
            .id();
        let mut ai = Duellist::new(target);
        ai.holds_ground = holds_ground;
        let foe = app
            .world_mut()
            .spawn((
                ai,
                Transform::default(),
                GlobalTransform::IDENTITY,
                Animator::new("Default", "Idle"),
                Fighter::default(),
                ChildOf(root),
            ))
            .id();
        (app, foe, root)
    }

    fn state(app: &App, foe: Entity) -> (Stance, String) {
        let w = app.world();
        (
            *w.get::<Stance>(foe).unwrap(),
            w.get::<Animator>(foe).unwrap().action.clone(),
        )
    }

    fn engage(app: &mut App, foe: Entity) {
        app.world_mut().get_mut::<Duellist>(foe).unwrap().engaged = true;
    }

    /// It stands idle until the fight starts, then takes the guard; far
    /// from its target (8 m, beyond four reaches) it keeps the guard and
    /// walks in with the interim footwork.
    #[test]
    fn the_duellist_enters_combat_in_its_guard() {
        let (mut app, foe, root) = duel(8.0, false);
        app.update();
        assert_eq!(state(&app, foe), (Stance::Idle, "Idle".into()));
        engage(&mut app, foe);
        app.update();
        assert_eq!(state(&app, foe), (Stance::Defence, "Defence stance".into()));
        app.update();
        assert_eq!(state(&app, foe), (Stance::Defence, "Combat Walk".into()));
        let v = app
            .world()
            .get::<CharacterBody>(root)
            .unwrap()
            .desired_velocity;
        assert!((v - Vec3::Z * APPROACH_SPEED).length() < 1e-5, "{v}");
    }

    /// Standing its ground it shows its stance: the guard at 8 m, the
    /// combat idle at 1.5 m (under four reaches), and it never strikes.
    #[test]
    fn a_still_duellist_shows_its_stance_by_distance() {
        let (mut app, foe, root) = duel(8.0, true);
        engage(&mut app, foe);
        app.update();
        app.update();
        assert_eq!(state(&app, foe), (Stance::Defence, "Defence stance".into()));
        let (mut near, foe2, root2) = duel(1.0, true);
        engage(&mut near, foe2);
        near.world_mut().get_mut::<Duellist>(foe2).unwrap().cooldown = 0.0;
        for _ in 0..3 {
            near.update();
        }
        assert_eq!(
            state(&near, foe2),
            (Stance::CombatIdle, "Cmbt - Idle".into())
        );
        assert!(near.world().get::<Attacking>(foe2).is_none());
        for (app, root) in [(&app, root), (&near, root2)] {
            let v = app
                .world()
                .get::<CharacterBody>(root)
                .unwrap()
                .desired_velocity;
            assert_eq!(v, Vec3::ZERO);
        }
    }

    /// Within striking distance it strikes once its cooldown runs out, the
    /// attack ending in its stance.
    #[test]
    fn the_duellist_strikes_into_its_stance() {
        let (mut app, foe, _) = duel(1.0, false);
        engage(&mut app, foe);
        app.update();
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_none());
        app.world_mut().get_mut::<Duellist>(foe).unwrap().cooldown = 0.0;
        app.update();
        let anim = app.world().get::<Animator>(foe).unwrap();
        assert_eq!(anim.action, "Attack Mid");
        assert_eq!(anim.then.as_deref(), Some("Cmbt - Idle"));
        assert!(app.world().get::<Attacking>(foe).is_some());
    }

    /// Its strikes go through the original's executor: none under 80 % of
    /// its most energy (0.79), and with 0.8 none until 400 ms (no primary
    /// striker yet) after its last attack tick on the game clock.
    #[test]
    fn the_duellist_waits_for_its_energy_and_cooldown() {
        let (mut app, foe, _) = duel(1.0, false);
        app.insert_resource(BodyTicks(1000)); // 12 000 ms
        engage(&mut app, foe);
        app.update();
        app.update();
        let part = exa_physics::body::PartSetup {
            bone: 1,
            entity: Entity::PLACEHOLDER,
            parent: None,
            world: bevy::math::Affine3A::IDENTITY,
            centre: Vec3::ZERO,
            half: Vec3::splat(0.08),
            origin: Vec3::ZERO,
            links: vec![],
            foot: None,
            pins: Vec::new(),
        };
        let mut ragdoll = Ragdoll::new(&[part]).with_drive();
        ragdoll.drive.as_mut().unwrap().energy = 0.79;
        app.world_mut().entity_mut(foe).insert(ragdoll);
        app.world_mut().get_mut::<Duellist>(foe).unwrap().cooldown = 0.0;
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_none());
        let mut e = app.world_mut().entity_mut(foe);
        e.get_mut::<Ragdoll>()
            .unwrap()
            .drive
            .as_mut()
            .unwrap()
            .energy = 0.8;
        e.get_mut::<Attacker>().unwrap().last_tick_ms = Some(11_700);
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_none());
        app.world_mut().resource_mut::<BodyTicks>().0 = 1009; // 12 108 ms
        app.update();
        assert!(app.world().get::<Attacking>(foe).is_some());
    }
}

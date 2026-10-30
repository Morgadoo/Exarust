//! What a force puts on a body it holds, and on the weapons under Force
//! Strikes (`docs/THAUMATURGY.md`, Force).
//!
//! **A held body** (Kinesis's pick, Throw's grab from the cursor) takes:
//!
//! - the held environment ([`exa_physics::dynamics::Environment::HELD`]):
//!   no gravity, and the per-second retentions 0.125, 0.0125 and 0.025 by
//!   speed band, in place of its own, until the force lets it go;
//! - a force shell over its meshes ([`ForceShell::held`]: colour 2.8,
//!   fade-plane scale 0.5, texture scale `(1/radius + 0.015)·0.5` for the
//!   body's bounding radius in cm), which opens, holds while the force
//!   holds and closes when it lets go. A prop's shell copies its rigid
//!   meshes ([`RigidAuraOf`]); a dead character's copies its skinned ones
//!   (`super::effect_draw::ShellAuraOf`).
//!
//! A kinesis that hands its body to a throw hands the shell and the
//! environment with it; the throw drops the shell 800 ms after its lift and
//! gives the environment back 200 ms later, or both when it ends first. A
//! kinesis that ends on its own gives both back at once ([`HoldEnd`]).
//!
//! **Force Strikes** puts an aura on each item its caster holds, at most
//! four ([`ForceShell::strikes`]: colour 2.8, fade-plane scale 1, sound gain
//! 1.5 and pitch 0.8, ripple scale 3), drawn with the strikes program
//! (`effectfrcstrk_f`) and its gradient ([`crate::thaum::strikes_gradient`]):
//! brightest at the middle of the weapon's striking part, from its record.
//! An item put away loses its aura, an item taken gains one while fewer
//! than four carry one, and every aura closes when the power ends. While
//! the power runs it steps its auras' ripples itself.
//!
//! Evidence: the original's kinesis pick and end, the throw's creators,
//! lift, drop and end, the shared held-body shell creator, the strikes
//! activation, aura builder, weapon callbacks, tick and end, and the held
//! environment's record and conversion, read in the instructions.
//! [inf: the original swaps the held environment onto every node of the
//! held body's tree and writes the first node's saved one back to all;
//! Exarust's body keeps one environment for all its parts, so the swap is
//! the body's. The bounding radius is the node's; Exarust takes half the
//! diagonal of the body's stand-ins' box. The original also sets a
//! simulation flag on each held node whose reader was not found; it is not
//! reproduced. The original builds the throw's environment at its lift and
//! its shell at its cast; Exarust does both on the cast's first fixed tick.
//! The strikes callbacks run on equip and unequip; Exarust compares the
//! held items each tick. A weapon assembled from several nodes dims its
//! gradient by its last node's radius in the original; Exarust keeps the
//! record's gradient for it.]

use bevy::prelude::*;
use exa_character::grip_runtime::HeldRecord;
use exa_formats::weapon_record::WeaponRecord;
use exa_physics::Physics;
use exa_physics::body::{BodyTicks, Ragdoll};
use exa_physics::dynamics::Environment;
use exa_physics::grip::{HeldGrip, HeldKind};
use exa_physics::prop::{PropBody, StandIns};

use super::cast::{ActiveClass, LiveShellAura, PowerCaster};
use super::effect_draw::ShellAuraOf;
use super::live::EffectRoll;
use super::power::ENDING;
use super::power::force::MOST_AURAS;
use crate::thaum::ForceShell;

const CM: f32 = 100.0;

/// On a shell aura: the body whose rigid meshes it copies (a prop or a
/// held item's body).
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RigidAuraOf(pub Entity);

/// On a strikes aura: its weapon's record values (`start`, `end` and
/// `length`, cm) for the gradient, when the item has a weapon record. The
/// aura draws with the strikes program.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct StrikesAura {
    pub record: Option<[f32; 3]>,
}

/// A force lets go of what it held: the body's environment comes back and
/// the shell closes.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HoldEnd {
    pub body: Option<Entity>,
    pub shell: Option<Entity>,
}

impl HoldEnd {
    /// The release of a power's held body and shell (bits, as the classes
    /// keep them), or `None` when it holds neither.
    pub fn of(body: Option<u64>, shell: Option<u64>) -> Option<Self> {
        let body = body.and_then(Entity::try_from_bits);
        let shell = shell.and_then(Entity::try_from_bits);
        (body.is_some() || shell.is_some()).then_some(Self { body, shell })
    }
}

/// The bounding radius (cm) of a prop body: half the diagonal of its
/// stand-ins' box, else of its parts' spheres.
pub fn body_radius(
    body: Entity,
    ragdoll: &Ragdoll,
    physics: Option<&Physics>,
    stand_ins: Option<&StandIns>,
) -> f32 {
    let mut bounds: Option<(Vec3, Vec3)> = None;
    let handles = stand_ins.and_then(|s| s.0.get(&body));
    for &(handle, _) in handles.into_iter().flatten() {
        if let Some(c) = physics.and_then(|p| p.0.colliders.get(handle)) {
            let aabb = c.compute_aabb();
            let (lo, hi) = (aabb.mins, aabb.maxs);
            bounds = Some(bounds.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
        }
    }
    if bounds.is_none() {
        for part in &ragdoll.parts {
            let world = part.transform();
            for &(centre, radius) in part.spheres.iter() {
                let c = world.transform_point3(centre);
                let (lo, hi) = (c - radius, c + radius);
                bounds = Some(bounds.map_or((lo, hi), |(a, b)| (a.min(lo), b.max(hi))));
            }
        }
    }
    bounds.map_or(1.0, |(lo, hi)| ((hi - lo) * 0.5 * CM).length().max(1.0))
}

/// Puts a held body in the held environment and spawns its shell: a prop's
/// over its rigid meshes, any other body's over its skinned ones. Returns
/// the shell's node.
pub fn grab_body(
    commands: &mut Commands,
    body: Entity,
    ragdoll: &mut Ragdoll,
    prop: bool,
    radius: f32,
    now: u32,
    roll: &mut EffectRoll,
) -> Entity {
    ragdoll.hold_in(Environment::HELD);
    ragdoll.wake();
    let tex_pos = [roll.0.unit(), roll.0.unit()];
    let shell = ForceShell::held(now, tex_pos, radius);
    let mut node = commands.spawn((Name::new("_ForceAura01 held"), LiveShellAura(shell)));
    if prop {
        node.insert(RigidAuraOf(body));
    } else {
        node.insert((ShellAuraOf(body), Transform::default()));
    }
    let shell = node.id();
    info!("force hold: {body:?} in the held environment, shell {shell:?} (radius {radius:.1} cm)");
    shell
}

/// After the caster tick: each released hold gives its body its own
/// environment back and closes its shell.
pub(super) fn end_holds(
    mut ends: MessageReader<HoldEnd>,
    mut bodies: Query<&mut Ragdoll>,
    mut auras: Query<&mut LiveShellAura>,
) {
    for end in ends.read() {
        if let Some(body) = end.body
            && let Ok(mut ragdoll) = bodies.get_mut(body)
        {
            ragdoll.release_held();
            ragdoll.wake();
            debug!("force hold: {body:?} has its environment back");
        }
        if let Some(shell) = end.shell
            && let Ok(mut aura) = auras.get_mut(shell)
        {
            aura.0.start_fade();
            debug!("force hold: shell {shell:?} closes");
        }
    }
}

/// Before the caster tick: a throw from the cursor puts its body in the
/// held environment and spawns its shell.
#[allow(clippy::type_complexity)]
pub(super) fn grab_thrown(
    mut commands: Commands,
    mut casters: Query<&mut PowerCaster>,
    mut bodies: Query<(&mut Ragdoll, Has<PropBody>), Without<PowerCaster>>,
    ticks: Option<Res<BodyTicks>>,
    physics: Option<Res<Physics>>,
    stand_ins: Option<Res<StandIns>>,
    mut roll: ResMut<EffectRoll>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for mut caster in &mut casters {
        for entry in caster.active.iter_mut() {
            let ActiveClass::Throw(power) = &mut entry.class else {
                continue;
            };
            if power.grabbed || power.state == ENDING {
                continue;
            }
            power.grabbed = true;
            let Some(body) = power.body.and_then(Entity::try_from_bits) else {
                continue;
            };
            let Ok((mut ragdoll, prop)) = bodies.get_mut(body) else {
                continue;
            };
            let radius = body_radius(body, &ragdoll, physics.as_deref(), stand_ins.as_deref());
            let shell = grab_body(
                &mut commands,
                body,
                &mut ragdoll,
                prop,
                radius,
                now,
                &mut roll,
            );
            power.shell = Some(shell.to_bits());
            power.material = Some(body.to_bits());
        }
    }
}

/// The items a body holds (by its first part's entity), in entity order:
/// each with its record's striking part and length when it has one.
fn held_items<'a>(
    key: Entity,
    items: impl Iterator<Item = (Entity, &'a HeldGrip, Option<&'a HeldRecord>)>,
) -> Vec<(Entity, Option<[f32; 3]>)> {
    let mut held: Vec<(Entity, Option<[f32; 3]>)> = items
        .filter(|(_, grip, _)| grip.holder_key == key && !grip.released)
        .map(|(entity, grip, record)| {
            let record = record.filter(|_| grip.kind == HeldKind::Weapon).map(|r| {
                let built = WeaponRecord::build(&r.section, grip.flip, r.lengths, &r.holder);
                [built.start, built.end, built.length]
            });
            (entity, record)
        })
        .collect();
    held.sort_by_key(|(entity, _)| *entity);
    held
}

/// After the caster tick: Force Strikes keeps an aura on each item its
/// caster holds (at most four), removes the aura of an item put away, and
/// steps its auras' ripples while it runs.
#[allow(clippy::type_complexity)]
pub(super) fn dress_strikes(
    mut commands: Commands,
    mut casters: Query<(Entity, &mut PowerCaster, &Ragdoll)>,
    items: Query<(Entity, &HeldGrip, Option<&HeldRecord>)>,
    mut auras: Query<&mut LiveShellAura>,
    ticks: Option<Res<BodyTicks>>,
    mut roll: ResMut<EffectRoll>,
) {
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    for (caster_entity, mut caster, ragdoll) in &mut casters {
        let Some(key) = ragdoll.parts.first().map(|p| p.entity) else {
            continue;
        };
        for entry in caster.active.iter_mut() {
            let ActiveClass::Strikes(power) = &mut entry.class else {
                continue;
            };
            if power.state == ENDING {
                continue;
            }
            let held = held_items(key, items.iter());
            // Put away: the aura closes and leaves the list.
            let gone: Vec<(u64, Option<u64>)> = power
                .auras
                .iter()
                .filter(|aura| !held.iter().any(|(e, _)| e.to_bits() == aura.item))
                .map(|aura| (aura.item, aura.shell))
                .collect();
            for (item, shell) in gone {
                power.weapon_change(item, caster_entity.to_bits(), false);
                if let Some(mut aura) = shell
                    .and_then(Entity::try_from_bits)
                    .and_then(|s| auras.get_mut(s).ok())
                {
                    aura.0.start_fade();
                }
                debug!("force strikes: item {item:#x} put away, its aura closes");
            }
            // Taken: an aura while fewer than four carry one.
            for (item, record) in held {
                if power.auras.iter().any(|aura| aura.item == item.to_bits())
                    || power.auras.len() >= MOST_AURAS
                {
                    continue;
                }
                if !power.weapon_change(item.to_bits(), caster_entity.to_bits(), true) {
                    continue;
                }
                let tex_pos = [roll.0.unit(), roll.0.unit()];
                let shell = commands
                    .spawn((
                        Name::new("effectfrcstrk aura"),
                        LiveShellAura(ForceShell::strikes(now, tex_pos)),
                        RigidAuraOf(item),
                        StrikesAura { record },
                    ))
                    .id();
                if let Some(aura) = power.auras.last_mut() {
                    aura.shell = Some(shell.to_bits());
                }
                info!(
                    "force strikes: aura {shell:?} on {item:?} (record {record:?}), {} auras",
                    power.auras.len()
                );
            }
            // The running power steps its auras' ripples.
            for aura in &power.auras {
                if let Some(mut live) = aura
                    .shell
                    .and_then(Entity::try_from_bits)
                    .and_then(|s| auras.get_mut(s).ok())
                {
                    live.0.step_ripples();
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_hold_end_names_what_it_lets_go() {
        assert_eq!(HoldEnd::of(None, None), None);
        let body = World::new().spawn_empty().id();
        let end = HoldEnd::of(Some(body.to_bits()), None).unwrap();
        assert_eq!((end.body, end.shell), (Some(body), None));
    }

    #[test]
    fn a_held_body_floats_until_its_hold_ends() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_message::<HoldEnd>()
            .init_resource::<EffectRoll>()
            .add_systems(Update, end_holds);
        let mut doll = crate::thaum::blast_live::test_bodies::scattered(
            &mut crate::arena::Mt19937::new(3),
            Vec3::new(0.0, 2.0, 0.0),
            0.3,
        );
        let mut commands_world = World::new();
        let body = commands_world.spawn_empty().id();
        let mut queue = bevy::ecs::world::CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &commands_world);
        let mut roll = EffectRoll::default();
        let shell = grab_body(&mut commands, body, &mut doll, true, 40.0, 0, &mut roll);
        queue.apply(&mut commands_world);
        let spawned = commands_world.get::<LiveShellAura>(shell).unwrap();
        assert_eq!(spawned.0.colour, 2.8);
        assert_eq!(spawned.0.tex_scale, crate::thaum::kinesis_scale(40.0));
        assert_eq!(
            commands_world.get::<RigidAuraOf>(shell),
            Some(&RigidAuraOf(body))
        );
        assert_eq!(doll.held_environment(), Some(Environment::HELD));
        let body = app.world_mut().spawn(doll).id();
        let aura = app
            .world_mut()
            .spawn(LiveShellAura(ForceShell::held(0, [0.0; 2], 40.0)))
            .id();
        app.world_mut().write_message(HoldEnd {
            body: Some(body),
            shell: Some(aura),
        });
        app.update();
        assert_eq!(
            app.world().get::<Ragdoll>(body).unwrap().held_environment(),
            None
        );
        assert_eq!(app.world().get::<LiveShellAura>(aura).unwrap().0.state, 2);
    }
}

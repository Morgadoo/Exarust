//! The live Summon: the demon it asks for, its arrival and its release.
//!
//! The original's summoning works in three
//! places:
//!
//! - **The activation** reads the character record `demon a01` from the
//!   character database, registers it under a fresh character id, gives it
//!   role 33 (Activity Wait), sets the familiarity of demon and caster
//!   toward each other to 75 and adds relation role 28 (Ally) both ways,
//!   and places it. Exarust asks for the body once the place is known
//!   ([`DemonAsked`], built by the viewer through the placed-character
//!   path and marked [`Summoned`]); the roles and relations are written at
//!   the arrival, when the demon's components exist.
//! - **The deferred lookup** waits for the demon's body; once the summon is
//!   past its pick, **the arrival** ([`arrive_demons`]) puts the body at the
//!   place facing the caster, holds it (its AI's statue byte: a statue
//!   without a sense of its own perceives nobody), keeps the demon in the
//!   caster's power memory (the AI never summons again), puts the summon's
//!   vanish (its own table, the shell resting at nothing shown) and the
//!   tendril image on the demon, plays `thmdsp_Smn` at the demon at volume
//!   0.4 and pitch `roll·0.2 + 0.9`, and starts the held phase (2000 ms).
//! - **The control** ([`super::power::displacement::Summon`]): the shell
//!   opens over 3000 ms; then the demon is let go ([`release_demons`]: the
//!   statue byte cleared, its body woken for 256 ticks, the tendrils end)
//!   and the shell closes over 1000 ms; a second later the summon ends.
//!
//! After that nothing of the summon remains: the demon is an ordinary
//! character that waits where it stands and fights whom its own roles hate
//! (humans and the undead, for the Demon role), an ally of its summoner.
//! No routine of the original sends it away after a time or at its
//! summoner's death, and none makes it follow.
//!
//! Exarust does not model the body's sleep word (2 while held: the
//! environment's fast retention × 0.95), the caster-body gain the fade
//! writes, nor the summon's light.
//!
//! Evidence: the summon's activation, its deferred body lookup, its arrival
//! and control, the AI's summoning advice, the role and relation routines,
//! read from `Exanima.exe`.

use bevy::prelude::*;
use exa_ai::brain::{CharacterId, Npc, RoleDatabase};
use exa_ai::roles::{Relations, Roles};
use exa_physics::body::{BodyTicks, Ragdoll};

use super::cast::{ActiveClass, PowerCaster};
use super::live::{EffectDraws, EffectRoll, LiveWisps};
use super::power::displacement::{
    DEMON_RECORD, DEMON_RELATION, DEMON_ROLE, DEMON_TOPIC, SUMMON_SOUND_VOLUME, SUMMON_SPAWN,
    summon_pitch,
};
use crate::effects::EffectClock;

/// Centimetres a metre.
const CM: f32 = 100.0;

/// A summon asks for its demon: the character record `record` placed at
/// `at` (feet, m) facing `yaw`, for the summoner `caster`.
#[derive(Message, Debug, Clone, Copy, PartialEq)]
pub struct DemonAsked {
    pub caster: Entity,
    pub record: &'static str,
    pub at: Vec3,
    pub yaw: f32,
}

/// A summoned demon's body, marked by whoever built it for its summoner.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summoned {
    pub caster: Entity,
}

/// A demon its summon has claimed at its arrival.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummonArrived;

/// Registers what the summon's fixed-step systems touch.
pub(crate) fn bridge_summon_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_message::<DemonAsked>()
        .bridge_copy::<Summoned>()
        .bridge_copy::<SummonArrived>();
}

/// Each body step, after the picks: a summon whose place is known asks for
/// its demon once.
pub(crate) fn ask_demons(
    mut casters: Query<(Entity, &mut PowerCaster)>,
    mut asked: MessageWriter<DemonAsked>,
) {
    for (entity, mut caster) in &mut casters {
        for entry in &mut caster.active {
            let ActiveClass::Summon(power) = &mut entry.class else {
                continue;
            };
            if !power.wants_demon() {
                continue;
            }
            let Some(place) = power.place else {
                continue;
            };
            power.asked = true;
            let at = Vec3::from_array(place) / CM;
            info!(
                "summon by {entity:?}: asks for {DEMON_RECORD} at {at:?} m, yaw {:.3}",
                power.yaw
            );
            asked.write(DemonAsked {
                caster: entity,
                record: DEMON_RECORD,
                at,
                yaw: power.yaw,
            });
        }
    }
}

/// What the arrival reads and writes on the demon besides its body.
#[derive(bevy::ecs::system::SystemParam)]
pub(crate) struct DemonMind<'w, 's> {
    ids: Query<'w, 's, &'static CharacterId>,
    roles: Query<'w, 's, &'static mut Roles>,
    relations: Query<'w, 's, &'static mut Relations>,
    npcs: Query<'w, 's, &'static mut Npc>,
    db: Option<Res<'w, RoleDatabase>>,
}

/// Each body step, before the casters' walk: a summon waiting for its body
/// takes the demon built for it once the body exists (the arrival).
#[allow(clippy::too_many_arguments)]
pub(crate) fn arrive_demons(
    mut commands: Commands,
    mut casters: Query<(Entity, &mut PowerCaster)>,
    demons: Query<(Entity, &Summoned, &Ragdoll), Without<SummonArrived>>,
    mut mind: DemonMind,
    ticks: Option<Res<BodyTicks>>,
    clock: Option<Res<EffectClock>>,
    mut draws: ResMut<EffectDraws>,
    mut roll: ResMut<EffectRoll>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
) {
    if demons.is_empty() {
        return;
    }
    let now = ticks.as_deref().map_or(0, |t| t.millis() as u32);
    let effect_now = clock.as_deref().map_or(0, |c| c.now_ms);
    let now_ms = u64::from(now);
    for (demon, summoned, body) in &demons {
        let Ok((caster_entity, mut caster)) = casters.get_mut(summoned.caster) else {
            continue;
        };
        let Some(index) = caster.active.iter().position(|entry| {
            matches!(&entry.class, ActiveClass::Summon(p)
                if p.phase == SUMMON_SPAWN && p.asked && p.demon.is_none())
        }) else {
            continue;
        };
        commands.entity(demon).insert(SummonArrived);
        // The tendril image on the demon's body, then the vanish over it.
        let wisps =
            super::cast::spawn_body_wisps(&mut commands, demon, body, &mut draws, &mut roll);
        let tex_pos = [roll.0.unit(), roll.0.unit()];
        // The demon's size, from its built body's muscles (the build
        // took the spawn's `Physical` request off the root).
        let size = body
            .drive
            .as_ref()
            .map_or(1.0, |drive| drive.stats.size)
            .max(f32::MIN_POSITIVE);
        let vanish = commands
            .spawn((
                Name::new("_SummonVanish"),
                super::vanish_live::LiveVanish::summon(
                    demon,
                    caster_entity,
                    effect_now,
                    tex_pos,
                    size,
                ),
            ))
            .id();
        let entry = &mut caster.active[index];
        entry.entity = Some(vanish);
        if let ActiveClass::Summon(power) = &mut entry.class {
            power.arrive(demon.to_bits(), Some(wisps.to_bits()), now);
        }
        // Held: its AI's statue byte.
        if let Ok(mut npc) = mind.npcs.get_mut(demon) {
            npc.statue = true;
        }
        // The roles and the relations: role 33 on the demon, familiarity 75
        // and relation role 28 each way.
        let demon_id = mind.ids.get(demon).ok().map(|id| id.0);
        let caster_id = mind.ids.get(caster_entity).ok().map(|id| id.0);
        if let Some(db) = mind.db.as_deref().cloned() {
            if let Ok(mut roles) = mind.roles.get_mut(demon) {
                roles.add(&db.book, DEMON_ROLE);
            }
            link(&mut mind, &db, demon, caster_id, now_ms);
            link(&mut mind, &db, caster_entity, demon_id, now_ms);
        }
        // The power memory keeps the demon: the AI's advice refuses from
        // now on.
        caster.summoned = demon_id.or(Some(0));
        let pitch = summon_pitch(draws.0.unit());
        let at = body
            .parts
            .first()
            .map_or(Vec3::ZERO, |p| p.transform().translation.into());
        sounds.write(exa_audio::PointSounded {
            name: super::displacement::SUMMON_SOUND.into(),
            position: at,
            volume: SUMMON_SOUND_VOLUME,
            pitch,
            flags: 0,
        });
        let hex = |id: Option<u32>| id.map_or_else(|| "none".to_string(), |id| format!("{id:#x}"));
        info!(
            "summon by {caster_entity:?}: {DEMON_RECORD} {demon:?} (id {}) arrives, held for \
             2000 ms; role {DEMON_ROLE}, familiarity {DEMON_RELATION} and relation role \
             {DEMON_TOPIC} with {}; sound pitch {pitch:.3}; vanish size {size:.3}",
            hex(demon_id),
            hex(caster_id)
        );
    }
}

/// One side of the summon's link: `who` takes familiarity 75 and relation
/// role 28 toward `other`.
fn link(mind: &mut DemonMind, db: &RoleDatabase, who: Entity, other: Option<u32>, now_ms: u64) {
    let Some(other) = other else {
        return;
    };
    let Ok(roles) = mind.roles.get(who).cloned() else {
        return;
    };
    let Ok(mut relations) = mind.relations.get_mut(who) else {
        return;
    };
    relations.set_familiarity(other, DEMON_RELATION, now_ms as u32, db.locale);
    relations.add_role(&db.book, &roles, other, DEMON_TOPIC, 0, db.locale);
}

/// Each body step, after the casters' walk: a summon whose fade has ended
/// lets its demon go: the statue byte clears (its roles are applied
/// again) and the body wakes for 256 ticks.
pub(crate) fn release_demons(
    casters: Query<(Entity, &PowerCaster)>,
    mut npcs: Query<&mut Npc>,
    mut bodies: Query<&mut Ragdoll>,
    mut released: Local<std::collections::HashSet<Entity>>,
) {
    for (caster_entity, caster) in &casters {
        for entry in &caster.active {
            let ActiveClass::Summon(power) = &entry.class else {
                continue;
            };
            if !power.last.release {
                continue;
            }
            let Some(demon) = power.demon.and_then(Entity::try_from_bits) else {
                continue;
            };
            if !released.insert(demon) {
                continue;
            }
            if let Ok(mut npc) = npcs.get_mut(demon) {
                npc.statue = false;
            }
            if let Ok(mut body) = bodies.get_mut(demon) {
                body.wake();
            }
            info!("summon by {caster_entity:?}: {demon:?} is let go");
        }
    }
}

/// The tendril image of a summon that has let its demon go ends: its
/// wisps trail away and leave once faded.
pub(crate) fn end_demon_wisps(
    casters: Query<&PowerCaster>,
    mut sets: Query<(Entity, &mut LiveWisps)>,
    mut ended: Local<std::collections::HashSet<Entity>>,
) {
    for caster in &casters {
        for entry in &caster.active {
            let ActiveClass::Summon(power) = &entry.class else {
                continue;
            };
            if !power.last.release {
                continue;
            }
            let Some(demon) = power.demon.and_then(Entity::try_from_bits) else {
                continue;
            };
            for (set, mut live) in &mut sets {
                if live.body == demon && ended.insert(set) {
                    live.wisps.end();
                }
            }
        }
    }
}

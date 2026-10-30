//! Runs the encounter scripts an NPC's target pass reports.
//!
//! When an AI's attention moves to a character it has not attended before,
//! the pass reports that contact ([`exa_ai::contacts::PassReport::encounter`]).
//! The original's AI then calls the role unit, which walks the observer's
//! roles in list order and returns every encounter section that is for
//! anyone, for the partner in its locale, or for one of the partner's roles;
//! each is run with the observer as actor 0 and the partner as actor 1.  This
//! system does the same through [`exa_ai::roles::encounter`] and
//! `exa_script`'s engine, carrying the scripts' effects out as
//! [`exa_script::ScriptEffect`] messages.
//!
//! The encounter report is consumed here, so a contact reported once runs
//! its scripts once even though the game updates every frame.  A character
//! without a record id is skipped, as the original needs a character record
//! for the actor slot.

use bevy::prelude::*;
use exa_ai::brain::{CharacterId, Npc};
use exa_ai::roles::{self, Partner, Relations, Roles};

/// The encounter-script bridge.
pub struct RoleScriptsPlugin;

impl Plugin for RoleScriptsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, run_encounter_scripts.after(exa_ai::brain::ai_tick));
    }
}

/// Runs the encounter scripts the last target passes reported.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn run_encounter_scripts(
    scripts: Option<Res<exa_script::Scripts>>,
    mut conversations: Option<ResMut<exa_script::Conversations>>,
    time: Option<Res<Time<Virtual>>>,
    mut characters: ParamSet<(
        Query<(&mut Npc, &Roles, &mut Relations, Option<&CharacterId>)>,
        Query<(Entity, &Roles, Option<&CharacterId>)>,
    )>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    let (Some(scripts), Some(conversations)) = (scripts, conversations.as_mut()) else {
        return;
    };
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    // Who the reported contacts are: their roles and record ids. The map is
    // built before the observer query mutably borrows the relations.
    let partners: Vec<(u64, Roles, u32)> = characters
        .p1()
        .iter()
        .map(|(entity, roles, id)| (entity.to_bits(), roles.clone(), id.map_or(0, |i| i.0)))
        .collect();
    let locale = conversations.locale;
    let mut work = Vec::new();
    for (mut npc, roles, mut relations, id) in characters.p0().iter_mut() {
        let Some(target) = npc.pass.encounter.take() else {
            continue;
        };
        let Some((_, partner_roles, partner)) = partners.iter().find(|p| p.0 == target) else {
            continue;
        };
        let Some(observer) = id.map(|i| i.0) else {
            continue;
        };
        if *partner == 0 || *partner == observer {
            continue;
        }
        let partner = Partner {
            id: *partner,
            roles: partner_roles,
            // A level character that left its level is not reported by this
            // pass; the migration stays the relation query's caller's job.
            former: None,
        };
        for script in roles::encounter(&scripts.roles, roles, &mut relations, &partner, now, locale)
        {
            work.push((observer, partner.id, script.role, script.script.to_vec()));
        }
    }
    if work.is_empty() {
        return;
    }
    let exa_script::Conversations {
        engine,
        cast,
        player,
        player2,
        world_base,
        locale,
        ..
    } = &mut **conversations;
    let mut scene = exa_script::dialogue::Scene {
        library: exa_script::dialogue::Library {
            roles: &scripts.roles,
            narrator: scripts.narrator.as_ref(),
        },
        cast,
        now,
        world_time: exa_script::topics::world_time(*world_base, now, 0),
        player: *player,
        player2: *player2,
        locale: *locale,
        effects: Vec::new(),
    };
    for (observer, partner, role, text) in work {
        engine.run_script(&mut scene, vec![Some(observer), Some(partner)], role, &text);
    }
    for effect in scene.effects.drain(..) {
        effects.write(exa_script::ScriptEffect(effect));
    }
}

#[cfg(test)]
#[path = "role_scripts_test.rs"]
mod tests;

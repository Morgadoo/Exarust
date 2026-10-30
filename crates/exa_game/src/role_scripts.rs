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
//!
//! The original keeps one relation record that both the AI and the scripts
//! read; Exarust keeps the AI's in [`Relations`] and the scripts' in the
//! session cast, so the meeting time the encounter renews is written to the
//! cast's record too (scripts read it as `LastEnc`) before the scripts run.

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
pub fn run_encounter_scripts(
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    time: Option<Res<Time<Virtual>>>,
    mut observers: Query<(&mut Npc, &Roles, &mut Relations, Option<&CharacterId>)>,
    partners: Query<(&Roles, Option<&CharacterId>)>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    let (Some(scripts), Some(mut conversations)) = (scripts, conversations) else {
        return;
    };
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    let locale = conversations.locale;
    let mut met = Vec::new();
    let mut work = Vec::new();
    for (mut npc, roles, mut relations, id) in &mut observers {
        let Some(target) = npc.pass.encounter.take() else {
            continue;
        };
        let Some((partner_roles, partner)) =
            Entity::try_from_bits(target).and_then(|e| partners.get(e).ok())
        else {
            continue;
        };
        let (Some(observer), Some(partner)) = (id.map(|i| i.0), partner.map(|i| i.0)) else {
            continue;
        };
        if partner == 0 || partner == observer {
            continue;
        }
        let partner = Partner {
            id: partner,
            roles: partner_roles,
            // A level character that left its level is not reported by this
            // pass; the migration stays the relation query's caller's job.
            former: None,
        };
        met.push((observer, partner.id));
        for script in roles::encounter(&scripts.roles, roles, &mut relations, &partner, now, locale)
        {
            work.push((observer, partner.id, script.role, script.script.to_vec()));
        }
    }
    for (observer, partner) in met {
        if let Some(character) = conversations.cast.get_mut(observer) {
            character.relation_entry(partner).last_encounter = now;
        }
    }
    if work.is_empty() {
        return;
    }
    let ((), out) = conversations.with_scene(&scripts, now, |engine, scene| {
        for (observer, partner, role, text) in work {
            engine.run_script(scene, vec![Some(observer), Some(partner)], role, &text);
        }
    });
    for effect in out {
        effects.write(exa_script::ScriptEffect(effect));
    }
}

#[cfg(test)]
#[path = "role_scripts_test.rs"]
mod tests;

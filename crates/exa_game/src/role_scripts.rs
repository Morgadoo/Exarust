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
//! **StartDialogue.** Before each script the encounter writes the roles
//! unit's static context: the observer, the partner, the running role and,
//! for an entry matched on one of the partner's roles, that role (the last
//! match stays otherwise). The original's `StartDialogue` reads that
//! context, whichever script calls it, and asks the dialogue engine for the
//! named node between the pair when the owner's arousal is at most 10 and it
//! is in no conversation ([`exa_script::dialogue::EncounterContext`]).
//!
//! The encounter report is consumed here, so a contact reported once runs
//! its scripts once even though the brains tick every body tick.  A character
//! without a record id is skipped, as the original needs a character record
//! for the actor slot.
//!
//! The original keeps one relation record that both the AI and the scripts
//! read; Exarust keeps the AI's in [`Relations`] and the scripts' in the
//! session cast, so the meeting time the encounter renews is written to the
//! cast's record too (scripts read it as `LastEnc`) before the scripts run.
//!
//! **Greetings.** Right after the encounter call, the original's pass greets
//! the newly attended character when the brain's notice scale is not above
//! 6, its readiness not above 1 and the other within 6 m (the pass's
//! [`exa_ai::contacts::PassReport::greet`]). The greeting asks the dialogue
//! engine for a calm class 0 entry node of the pair, with the brain's mood
//! code toward the other: its arousal must not pass 1 and it must not be in
//! a conversation already ([`exa_script::dialogue::Engine::greet`]). The
//! brain's contacts are synced into the cast first, so the mood reads the
//! level and concern the pass just left ([`crate::script_cast::sync_brain`]).

use bevy::prelude::*;
use exa_ai::brain::{CharacterId, Npc};
use exa_ai::roles::{self, Partner, Relations, Roles};

/// The encounter-script bridge.
pub struct RoleScriptsPlugin;

impl Plugin for RoleScriptsPlugin {
    fn build(&self, app: &mut App) {
        bridge_role_scripts_types(app);
        // The encounters a target pass reports run in the brains' tick, every
        // body tick, right after the AI tick that reported them.
        app.add_systems(
            exa_ai::BrainChain,
            (run_encounter_scripts, run_greetings)
                .chain()
                .after(exa_ai::brain::ai_tick)
                .before(exa_ai::DuelSystems),
        );
    }
}

/// The data this plugin's fixed-step systems touch, for the simulation's
/// bridge (`exa_physics::sim_bridge`).
fn bridge_role_scripts_types(app: &mut App) {
    use exa_physics::sim_bridge::SimBridgeAppExt;
    app.bridge_message::<exa_script::ScriptEffect>()
        .bridge_lend_resource::<crate::role_scripts::ExtraEncounterScript>()
        .bridge_lend_resource::<crate::script_cast::ContactMirror>()
        .bridge_lend_resource::<exa_script::Conversations>()
        .bridge_lend_resource::<exa_script::Scripts>();
}

/// A debug tool (`exarust --encounter-script TEXT`): a script run as one
/// more encounter script of every encounter, after the roles' own, with the
/// observer's roles unit context (no role). It lets a run show what an
/// encounter's `StartDialogue` or other builtin does where the shipped
/// encounter scripts do not call it.
#[derive(Resource, Debug, Clone, Default)]
pub struct ExtraEncounterScript(pub Vec<u8>);

/// Runs the encounter scripts the last target passes reported.
#[allow(clippy::too_many_arguments)]
pub fn run_encounter_scripts(
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    extra: Option<Res<ExtraEncounterScript>>,
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
            work.push((
                observer,
                partner.id,
                script.role,
                script.partner_role,
                script.script.to_vec(),
            ));
        }
        if let Some(extra) = extra.as_deref().filter(|e| !e.0.is_empty()) {
            work.push((observer, partner.id, 0, None, extra.0.clone()));
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
        for (observer, partner, role, partner_role, text) in work {
            // The partner role is written only by an entry matched on one
            // of the partner's roles; otherwise the last one stays.
            let kept = engine.encounter.map_or(0, |c| c.partner_role);
            let context = exa_script::dialogue::EncounterContext {
                owner: observer,
                partner,
                role,
                partner_role: partner_role.unwrap_or(kept),
            };
            let started = engine.active.len();
            engine.run_encounter_script(scene, context, &text);
            info!(
                "encounter script of role {role} ({observer} meets {partner}): {:?}",
                String::from_utf8_lossy(&text)
            );
            if engine.active.len() > started {
                let line = engine
                    .bubbles
                    .list
                    .last()
                    .map(|b| String::from_utf8_lossy(&b.text).into_owned());
                info!(
                    "encounter script of role {role}: StartDialogue started a conversation between {observer} and {partner}, first line {line:?}"
                );
            }
        }
    });
    for effect in out {
        effects.write(exa_script::ScriptEffect(effect));
    }
}

/// Runs the greetings the last target passes reported, after their
/// encounter scripts.
#[allow(clippy::type_complexity)]
pub fn run_greetings(
    scripts: Option<Res<exa_script::Scripts>>,
    conversations: Option<ResMut<exa_script::Conversations>>,
    mirror: Option<ResMut<crate::script_cast::ContactMirror>>,
    time: Option<Res<Time<Virtual>>>,
    mut greeters: Query<(&CharacterId, &mut Npc, Option<&Relations>)>,
    ids: Query<&CharacterId>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    let (Some(scripts), Some(mut conversations)) = (scripts, conversations) else {
        return;
    };
    let mut local = crate::script_cast::ContactMirror::default();
    let mut mirror = mirror;
    let mirror = mirror.as_deref_mut().unwrap_or(&mut local);
    let now = time.map_or(0, |t| t.elapsed().as_millis() as u32);
    let character_of = |key: u64| {
        Entity::try_from_bits(key)
            .and_then(|e| ids.get(e).ok())
            .map(|id| id.0)
    };
    for (id, mut npc, relations) in &mut greeters {
        let Some(target) = npc.pass.greet.take() else {
            continue;
        };
        let Some(partner) = character_of(target) else {
            continue;
        };
        let me = id.0;
        if partner == 0 || partner == me {
            continue;
        }
        crate::script_cast::sync_brain(
            &mut conversations.cast,
            mirror,
            me,
            &mut npc,
            relations,
            character_of,
        );
        let mood = exa_script::dialogue::mood(&conversations.cast, me, partner);
        let arousal = conversations.cast.get(me).map_or(0.0, |c| c.arousal);
        let (started, out) = conversations.with_scene(&scripts, now, |engine, scene| {
            engine.greet(scene, me, partner)
        });
        if started {
            info!(
                "character {me} greets {partner}: mood {mood:#x}, arousal {arousal:.2}, a greeting starts"
            );
        } else {
            debug!(
                "character {me} greets {partner}: mood {mood:#x}, arousal {arousal:.2}, no greeting"
            );
        }
        for effect in out {
            effects.write(exa_script::ScriptEffect(effect));
        }
    }
}

#[cfg(test)]
#[path = "role_scripts_test.rs"]
mod tests;

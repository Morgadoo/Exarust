//! Mirroring the running world's characters into the session's script cast.
//!
//! The conversation engine acts on [`exa_script::cast::Cast`], the script
//! side of the role database. The AI's roles, the body's health and
//! condition and the brain's arousal live on the world's entities; this
//! module copies them in each frame, so scripts and dialogues see the same
//! characters the AI does. A character's relations, topic memory and role
//! values stay in the cast (the AI keeps no copy of them).
//!
//! The original keeps one character object the AI and the scripts both use;
//! Exarust keeps the saved relation record in the script cast and mirrors
//! the live fields (the split is Exarust's).

use bevy::prelude::*;
use exa_ai::brain::{CharacterId, Npc};
use exa_ai::roles::Roles;
use exa_combat::{Condition, Fighter};
use exa_script::Conversations;
use exa_script::cast::Reading;

pub struct ScriptCastPlugin;

impl Plugin for ScriptCastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, mirror_cast);
    }
}

/// The component set one live character is mirrored from.
type LiveCharacter<'a> = (
    &'a CharacterId,
    &'a Roles,
    Option<&'a Name>,
    Option<&'a Fighter>,
    Option<&'a Condition>,
    Option<&'a Npc>,
);

/// Copies every character's live fields into the session's cast. Health is
/// the body's yellow pool and its maximum; out of action is the collapsing
/// or dead condition bit; arousal is the brain's, zero for a body without
/// one (a player or a body the AI does not run).
fn mirror_cast(conversations: Option<ResMut<Conversations>>, characters: Query<LiveCharacter<'_>>) {
    let Some(mut conversations) = conversations else {
        return;
    };
    for (id, roles, name, fighter, condition, npc) in &characters {
        let reading = Reading {
            id: id.0,
            name: name.map(|n| n.as_str()),
            roles: &roles.0,
            health: fighter.map(|f| (f.health.yellow, f.health.max)),
            awake: None,
            arousal: Some(npc.and_then(|n| n.personality).map_or(0.0, |p| p.arousal)),
            out_of_action: condition.map(|c| c.dying()),
        };
        conversations.cast.mirror(&[reading]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::roles::HeldRole;

    fn role(id: u32) -> HeldRole {
        HeldRole {
            id,
            weight: 1.0,
            explicit: true,
            inherited: 0,
            values: Vec::new(),
        }
    }

    #[test]
    fn the_worlds_characters_reach_the_script_cast() {
        let mut app = App::new();
        app.add_plugins(ScriptCastPlugin);
        app.insert_resource(Conversations::default());
        app.world_mut().spawn((
            CharacterId(9),
            Roles(vec![role(5)]),
            Name::new("Derrin"),
            Condition::COLLAPSING,
        ));
        app.update();
        let conversations = app.world().resource::<Conversations>();
        let c = conversations.cast.get(9).unwrap();
        assert_eq!(c.name, "Derrin");
        assert_eq!(c.roles.iter().map(|r| r.id).collect::<Vec<_>>(), [5]);
        assert!(c.out_of_action);
        // A body without a brain has arousal zero; its health stays as the
        // cast had it.
        assert_eq!(c.arousal, 0.0);
        assert_eq!(c.health, (1.0, 1.0));
    }
}

//! Mirroring the running world's characters into the session's script cast.
//!
//! The conversation engine acts on [`exa_script::cast::Cast`], the script
//! side of the role database. The AI's roles, the body's health and
//! condition and the brain's arousal live on the world's entities; this
//! module copies them in each frame, so scripts and dialogues see the same
//! characters the AI does. A character's relations, topic memory and role
//! values stay in the cast (the AI keeps no copy of them). When a script
//! changed a character's roles (`AddRole`, `RemoveRole`, a role value) the
//! cast's list is the newer one: it is written back to the AI's [`Roles`]
//! and the change flag is cleared, so the AI applies the roles anew.
//!
//! The player's own character id is the session's player: the conversation
//! engine speaks the player's lines and lists its replies for it. A new
//! player's id carries the world database's tag
//! ([`exa_ai::brain::PLAYER_ID`]), so it never meets a level character's.
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

use crate::player::PlayerBody;

pub struct ScriptCastPlugin;

impl Plugin for ScriptCastPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (mirror_player, mirror_cast).chain());
    }
}

/// The component set one live character is mirrored from.
type LiveCharacter<'a> = (
    &'a CharacterId,
    &'a mut Roles,
    Option<&'a Name>,
    Option<&'a Fighter>,
    Option<&'a Condition>,
    Option<&'a Npc>,
);

/// The session's player is the player body's character id.
fn mirror_player(
    conversations: Option<ResMut<Conversations>>,
    players: Query<&CharacterId, With<PlayerBody>>,
) {
    let Some(mut conversations) = conversations else {
        return;
    };
    let player = players.iter().next().map(|id| id.0);
    if conversations.player != player {
        conversations.player = player;
    }
}

/// Copies every character's live fields into the session's cast. Health is
/// the body's red (lasting) pool and its maximum, the pair the script
/// `Health` property divides; out of action is the collapsing or dead
/// condition bit; arousal is the brain's, zero for a body without one (a
/// player or a body the AI does not run). A role list a script changed goes
/// the other way first.
fn mirror_cast(
    conversations: Option<ResMut<Conversations>>,
    mut characters: Query<LiveCharacter<'_>>,
) {
    let Some(mut conversations) = conversations else {
        return;
    };
    for (id, mut roles, name, fighter, condition, npc) in &mut characters {
        if let Some(c) = conversations.cast.get_mut(id.0)
            && c.roles_changed
        {
            c.roles_changed = false;
            if roles.0 != c.roles {
                roles.0 = c.roles.clone();
            }
        }
        let reading = Reading {
            id: id.0,
            name: name.map(|n| n.as_str()),
            roles: &roles.0,
            health: fighter.map(|f| (f.health.red, f.health.max)),
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

    /// A role a script added survives the next frames: the cast's list goes
    /// back to the AI instead of being replaced by the AI's older one.
    #[test]
    fn a_role_a_script_added_reaches_the_ai() {
        let mut app = App::new();
        app.add_plugins(ScriptCastPlugin);
        app.insert_resource(Conversations::default());
        let body = app
            .world_mut()
            .spawn((CharacterId(4), Roles(vec![role(5)])))
            .id();
        app.update();
        {
            let mut conversations = app.world_mut().resource_mut::<Conversations>();
            let c = conversations.cast.get_mut(4).unwrap();
            c.roles.insert(0, role(7));
            c.roles_changed = true;
        }
        app.update();
        app.update();
        let ids = |r: &[HeldRole]| r.iter().map(|r| r.id).collect::<Vec<_>>();
        assert_eq!(ids(&app.world().get::<Roles>(body).unwrap().0), [7, 5]);
        let conversations = app.world().resource::<Conversations>();
        let c = conversations.cast.get(4).unwrap();
        assert_eq!(ids(&c.roles), [7, 5]);
        assert!(!c.roles_changed);
    }

    /// The player body's character id becomes the session's player.
    #[test]
    fn the_player_body_is_the_sessions_player() {
        let mut app = App::new();
        app.add_plugins(ScriptCastPlugin);
        app.insert_resource(Conversations::default());
        app.world_mut().spawn((CharacterId(6), Roles(Vec::new())));
        app.world_mut()
            .spawn((CharacterId(1), Roles(Vec::new()), PlayerBody::new(0.0)));
        app.update();
        let conversations = app.world().resource::<Conversations>();
        assert_eq!(conversations.player, Some(1));
        assert!(conversations.cast.get(1).is_some());
    }

    /// A level character numbered 1 and the new player (the world
    /// database's 1) are two cast entries.
    #[test]
    fn the_new_player_never_shares_a_level_characters_id() {
        let mut app = App::new();
        app.add_plugins(ScriptCastPlugin);
        app.insert_resource(Conversations::default());
        app.world_mut()
            .spawn((CharacterId(1), Roles(vec![role(8)]), Name::new("Zombie01")));
        app.world_mut().spawn((
            CharacterId(exa_ai::brain::PLAYER_ID),
            Roles(vec![role(7)]),
            Name::new("Ermus"),
            PlayerBody::new(0.0),
        ));
        app.update();
        app.update();
        let conversations = app.world().resource::<Conversations>();
        assert_eq!(conversations.player, Some(0x4000_0001));
        let player = conversations.cast.get(0x4000_0001).unwrap();
        assert_eq!(player.name, "Ermus");
        assert_eq!(player.roles.iter().map(|r| r.id).collect::<Vec<_>>(), [7]);
        assert_eq!(conversations.cast.get(1).unwrap().name, "Zombie01");
    }

    /// The health scripts see is the red pool over the maximum.
    #[test]
    fn the_cast_health_is_the_red_pool() {
        let mut app = App::new();
        app.add_plugins(ScriptCastPlugin);
        app.insert_resource(Conversations::default());
        let mut fighter = Fighter::default();
        fighter.health.max = 0.25;
        fighter.health.yellow = 0.1;
        fighter.health.red = 0.2;
        app.world_mut()
            .spawn((CharacterId(3), Roles(Vec::new()), fighter));
        app.update();
        let conversations = app.world().resource::<Conversations>();
        assert_eq!(conversations.cast.get(3).unwrap().health, (0.2, 0.25));
    }
}

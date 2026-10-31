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
        app.add_message::<exa_script::ScriptEffect>()
            .add_message::<exa_combat::impact::BadFall>()
            .add_message::<exa_combat::DamageTaken>()
            .add_systems(
                Update,
                (
                    mirror_player,
                    grant_witness_roles,
                    mirror_cast,
                    announce_witnessed,
                )
                    .chain(),
            );
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

/// A script's `WitnessRole` grants that role to every character within
/// 18 m of the player that the level's sight test can see. With no level
/// grid the sight test answers yes. The role is added with the parents it
/// inherits; the cast mirror copies the list on afterwards.
fn grant_witness_roles(
    mut effects: MessageReader<exa_script::ScriptEffect>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    scripts: Option<Res<exa_script::Scripts>>,
    players: Query<&GlobalTransform, With<PlayerBody>>,
    mut characters: Query<(&GlobalTransform, &mut Roles)>,
) {
    let asked: Vec<u32> = effects
        .read()
        .filter_map(|effect| match effect.0 {
            exa_script::dialogue::Effect::WitnessRole(role) => Some(role),
            _ => None,
        })
        .collect();
    if asked.is_empty() {
        return;
    }
    let (Some(scripts), Some(player)) = (scripts.as_deref(), players.iter().next()) else {
        return;
    };
    let origin = player.translation() * 100.0;
    let level = level.as_deref();
    for (transform, mut held) in &mut characters {
        let at = transform.translation() * 100.0;
        let delta = at - origin;
        if !exa_script::builtins::witness_range(delta.x, delta.y, delta.z) {
            continue;
        }
        if !exa_ai::rooms::in_sight(level, origin, at) {
            continue;
        }
        for &role in &asked {
            held.add(&scripts.roles, role);
        }
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

/// A witnessed event: a hard fall, or a wound deep enough to notice. Every
/// contact of the character that the level's sight test can see activates
/// the topic. With no level grid the sight test answers yes. A character
/// with no place, or no id, is not seen.
#[allow(clippy::too_many_arguments)]
fn announce_witnessed(
    mut falls: MessageReader<exa_combat::impact::BadFall>,
    mut wounds: MessageReader<exa_combat::DamageTaken>,
    conversations: Option<ResMut<Conversations>>,
    scripts: Option<Res<exa_script::Scripts>>,
    level: Option<Res<exa_world::room_view::LevelRooms>>,
    time: Option<Res<Time<Virtual>>>,
    ids: Query<&CharacterId>,
    places: Query<(&CharacterId, &GlobalTransform)>,
    mut effects: MessageWriter<exa_script::ScriptEffect>,
) {
    if falls.is_empty() && wounds.is_empty() {
        return;
    }
    let (Some(mut conversations), Some(scripts)) = (conversations, scripts.as_deref()) else {
        return;
    };
    let fallen: Vec<Entity> = falls.read().map(|fall| fall.fighter).collect();
    let hurt: Vec<Entity> = wounds.read().map(|wound| wound.fighter).collect();
    let mut at = std::collections::HashMap::new();
    for (id, transform) in &places {
        at.insert(id.0, transform.translation() * 100.0);
    }
    let level = level.as_deref();
    let now = time
        .map(|time| time.elapsed().as_millis() as u32)
        .unwrap_or(0);
    let (_, out) = conversations.with_scene(scripts, now, |_engine, scene| {
        let tell = |scene: &mut exa_script::dialogue::Scene<'_>, fighter: Entity, topic: &[u8]| {
            let Ok(id) = ids.get(fighter) else {
                return;
            };
            exa_script::dialogue::broadcast(
                scene,
                id.0,
                exa_formats::roles::Name::from_bytes(topic),
                |witness, actor| match (at.get(&actor), at.get(&witness)) {
                    (Some(from), Some(to)) => exa_ai::rooms::in_sight(level, *from, *to),
                    _ => false,
                },
            );
        };
        for fighter in fallen {
            tell(scene, fighter, b"BadFall");
        }
        for fighter in hurt {
            tell(scene, fighter, b"DamageTaken");
        }
    });
    for effect in out {
        effects.write(exa_script::ScriptEffect(effect));
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

    fn body(at: Vec3) -> (GlobalTransform, Roles) {
        (GlobalTransform::from_translation(at), Roles::default())
    }

    /// `WitnessRole` grants the role inside 18 m. With no level grid every
    /// character in range is seen; a grid sees the same room or an open
    /// door, and not a character merely in range beyond it.
    #[test]
    fn a_witnessed_role_reaches_who_can_see_the_player() {
        let mut app = App::new();
        app.add_message::<exa_script::ScriptEffect>()
            .insert_resource(exa_script::Scripts::default())
            .add_systems(Update, grant_witness_roles);
        app.world_mut()
            .spawn((PlayerBody::new(0.0), body(Vec3::ZERO)));
        let near = app.world_mut().spawn(body(Vec3::new(10.0, 0.0, 0.0))).id();
        let far = app.world_mut().spawn(body(Vec3::new(20.0, 0.0, 0.0))).id();
        app.world_mut().write_message(exa_script::ScriptEffect(
            exa_script::dialogue::Effect::WitnessRole(31),
        ));
        app.update();
        assert!(app.world().get::<Roles>(near).unwrap().has(31));
        assert!(!app.world().get::<Roles>(far).unwrap().has(31));

        use exa_formats::level::LevelGrid;
        use exa_world::room_view::LevelRooms;
        use exa_world::rooms::{CELL_OPEN, Rooms};
        let mut grid = LevelGrid {
            wall_sets: vec!["GEWS_UWA01".into()],
            floor_sets: vec!["GEFS_UWA01".into()],
            width: 6,
            height: 3,
            edges: vec![0; 18],
            plane_b: vec![0; 18],
            plane_c: vec![0; 18],
        };
        let open = |edges: &mut Vec<u32>, x: u32, y: u32, sides: [u32; 4]| {
            edges[(y * 6 + x) as usize] =
                CELL_OPEN | sides[0] | sides[1] << 4 | sides[2] << 8 | sides[3] << 12;
        };
        open(&mut grid.edges, 1, 1, [7, 7, 7, 0]);
        open(&mut grid.edges, 2, 1, [7, 0, 7, 0xB]);
        open(&mut grid.edges, 3, 1, [7, 0xB, 7, 0]);
        open(&mut grid.edges, 4, 1, [7, 0, 7, 7]);
        let mut walled = App::new();
        walled
            .add_message::<exa_script::ScriptEffect>()
            .insert_resource(exa_script::Scripts::default())
            .insert_resource(LevelRooms::new(std::sync::Arc::new(Rooms::flood(&grid))))
            .add_systems(Update, grant_witness_roles);
        walled
            .world_mut()
            .spawn((PlayerBody::new(0.0), body(Vec3::new(-9.0, 0.0, 3.0))));
        let seen = walled
            .world_mut()
            .spawn(body(Vec3::new(3.0, 0.0, 3.0)))
            .id();
        let hidden = walled
            .world_mut()
            .spawn(body(Vec3::new(9.0, 0.0, 3.0)))
            .id();
        walled.world_mut().write_message(exa_script::ScriptEffect(
            exa_script::dialogue::Effect::WitnessRole(31),
        ));
        walled.update();
        assert!(walled.world().get::<Roles>(seen).unwrap().has(31));
        assert!(!walled.world().get::<Roles>(hidden).unwrap().has(31));
    }

    /// A hard fall broadcasts BadFall, and a noticed wound broadcasts
    /// DamageTaken, to a contact the sight test can see. A witness with no
    /// place is not seen, so its topic stays quiet.
    #[test]
    fn a_hard_fall_reaches_a_witness_who_can_see_it() {
        use exa_formats::roles::{Dialogue, DialogueActor, Name, Node, Role, RoleDb, RoleEntry};
        use exa_script::cast::{Cast, Character, Contact};
        use exa_script::topics::{self, ACTIVATED};

        fn title(s: &str) -> Name {
            Name::from_bytes(s.as_bytes())
        }
        let book = |topic: &str| RoleDb {
            roles: vec![
                Role {
                    id: 31,
                    name: title("Role31"),
                    entries: vec![RoleEntry {
                        kind: 0x2,
                        target: 30,
                        dialogue: Some(Dialogue {
                            target: 30,
                            actors: ["Actor 1", "Actor 2"]
                                .iter()
                                .map(|n| DialogueActor {
                                    name: title(n),
                                    ..DialogueActor::default()
                                })
                                .collect(),
                            nodes: vec![Some(Node {
                                flags: 0x100 | 0xB_0000,
                                title: title(topic),
                                position: [0.0; 2],
                                links: vec![],
                                branches: vec![],
                            })],
                        }),
                        ..RoleEntry::default()
                    }],
                    ..Role::default()
                },
                Role {
                    id: 30,
                    flags: 1,
                    name: title("Role30"),
                    parent: 28,
                    ..Role::default()
                },
            ],
        };
        let run = |topic: &str, witness_placed: bool| {
            let db = book(topic);
            let mut witness = Character::new(1, "Witness");
            witness.add_role(&db, 31);
            witness.add_relation(2, 30, 0);
            let mut fallen = Character::new(2, "Fallen");
            fallen.contacts.push(Contact::new(1));
            let mut cast = Cast::default();
            cast.characters = vec![witness, fallen];
            let mut conversations = Conversations::default();
            conversations.cast = cast;
            let mut app = App::new();
            app.add_plugins(ScriptCastPlugin)
                .insert_resource(exa_script::Scripts {
                    roles: db.clone(),
                    narrator: None,
                })
                .insert_resource(conversations);
            let fighter = app
                .world_mut()
                .spawn((
                    CharacterId(2),
                    GlobalTransform::from_translation(Vec3::ZERO),
                ))
                .id();
            if witness_placed {
                app.world_mut().spawn((
                    CharacterId(1),
                    GlobalTransform::from_translation(Vec3::new(1.0, 0.0, 0.0)),
                ));
            } else {
                app.world_mut().spawn(CharacterId(1));
            }
            if topic == "DamageTaken" {
                app.world_mut()
                    .write_message(exa_combat::DamageTaken { fighter });
            } else {
                app.world_mut()
                    .write_message(exa_combat::impact::BadFall { fighter });
            }
            app.update();
            let cast = &app.world().resource::<Conversations>().cast;
            topics::block(cast.get(1).unwrap(), 2, 30, 31).and_then(|m| m.words.clone())
        };
        assert_eq!(run("BadFall", true), Some(vec![1, ACTIVATED]));
        assert_eq!(run("BadFall", false), None);
        assert_eq!(run("DamageTaken", true), Some(vec![1, ACTIVATED]));
        assert_eq!(run("DamageTaken", false), None);
    }
}

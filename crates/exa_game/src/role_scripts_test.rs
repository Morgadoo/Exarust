//! Tests of the encounter-script bridge.
//! (Included from role_scripts.rs.)

use bevy::ecs::system::RunSystemOnce;

use super::*;
use exa_ai::roles::HeldRole;
use exa_formats::roles::{Name, Role, RoleDb, RoleEntry, SECTION_ENCOUNTER};

fn book() -> RoleDb {
    RoleDb {
        roles: vec![Role {
            id: 1,
            name: Name::from_bytes(b"Encounterer"),
            entries: vec![RoleEntry {
                kind: SECTION_ENCOUNTER,
                target: 0,
                condition: 0,
                encounter: Some(b"GameEvent(00C10EC0);".to_vec()),
                ..RoleEntry::default()
            }],
            ..Role::default()
        }],
    }
}

/// An encounter reported by the observer's pass runs the observer role's
/// encounter script once, with the observer and partner as the two actors,
/// and renews the meeting time on both relation records; the report is
/// consumed so the next frame runs nothing.
#[test]
fn a_reported_encounter_runs_the_role_script_once() {
    let mut world = World::new();
    world.insert_resource(exa_script::Scripts {
        roles: book(),
        narrator: None,
    });
    let mut conversations = exa_script::Conversations::default();
    conversations
        .cast
        .characters
        .push(exa_script::cast::Character::new(1, "Observer"));
    world.insert_resource(conversations);
    let mut time = Time::<Virtual>::default();
    time.advance_by(std::time::Duration::from_millis(1234));
    world.insert_resource(time);
    world.init_resource::<Messages<exa_script::ScriptEffect>>();
    let partner = world.spawn((Roles::default(), CharacterId(2))).id();
    let observer = world
        .spawn((
            Npc::new([0.0; 4]),
            Roles(vec![HeldRole {
                id: 1,
                weight: 0.5,
                explicit: true,
                inherited: 0,
                values: Vec::new(),
            }]),
            Relations::default(),
            CharacterId(1),
        ))
        .id();
    world.get_mut::<Npc>(observer).unwrap().pass.encounter = Some(partner.to_bits());

    world.run_system_once(run_encounter_scripts).unwrap();
    let effects: Vec<_> = world
        .resource::<Messages<exa_script::ScriptEffect>>()
        .iter_current_update_messages()
        .cloned()
        .collect();
    assert!(
        effects
            .iter()
            .any(|e| matches!(&e.0, exa_script::dialogue::Effect::GameEvent(0x00C1_0EC0))),
        "{effects:?}"
    );
    assert_eq!(world.get::<Npc>(observer).unwrap().pass.encounter, None);
    // The meeting time reaches the AI's relation and the scripts' record.
    assert_eq!(
        world
            .get::<Relations>(observer)
            .unwrap()
            .find(2)
            .unwrap()
            .met_ms,
        1234
    );
    let cast = &world.resource::<exa_script::Conversations>().cast;
    assert_eq!(
        cast.get(1).unwrap().relation(2).unwrap().last_encounter,
        1234
    );

    // The report is gone: a second run writes no effect.
    world
        .resource_mut::<Messages<exa_script::ScriptEffect>>()
        .clear();
    world.run_system_once(run_encounter_scripts).unwrap();
    assert_eq!(
        world
            .resource::<Messages<exa_script::ScriptEffect>>()
            .iter_current_update_messages()
            .count(),
        0
    );
}

/// A greeting book: a guard (role 100) greets a Human (role 1) with a calm
/// branch (mood flags 0x40, taken only with no hostility code) before an
/// unflagged one.
fn greeting_book() -> RoleDb {
    use exa_formats::roles::{Branch, Dialogue, DialogueActor, Line, NODE_ENTRY, Node};
    let line = |text: &str| Line {
        speaker: 1,
        text: text.as_bytes().to_vec(),
    };
    let branch = |flags: u32, text: &str| Branch {
        flags,
        lines: vec![line(text)],
        condition: Vec::new(),
        action: Vec::new(),
        replies: Vec::new(),
    };
    let dialogue = Dialogue {
        target: 1,
        actors: ["Actor 1", "Actor 2"]
            .iter()
            .map(|n| DialogueActor {
                name: Name::from_bytes(n.as_bytes()),
                ..DialogueActor::default()
            })
            .collect(),
        nodes: vec![Some(Node {
            flags: NODE_ENTRY | 0x9,
            title: Name::from_bytes(b"Greeting"),
            position: [0.0; 2],
            links: Vec::new(),
            branches: vec![branch(0x40, "Good day."), branch(0, "Halt!")],
        })],
    };
    RoleDb {
        roles: vec![
            Role {
                id: 100,
                name: Name::from_bytes(b"Guard"),
                weight: 50,
                entries: vec![RoleEntry {
                    kind: 0x2,
                    target: 1,
                    dialogue: Some(dialogue),
                    ..RoleEntry::default()
                }],
                ..Role::default()
            },
            Role {
                id: 1,
                name: Name::from_bytes(b"Human"),
                weight: 50,
                ..Role::default()
            },
        ],
    }
}

/// The greeting a pass reports speaks the line the guard's mood allows: a
/// calm contact gets the calm branch; a watched contact with a concern
/// over 4 (mood code 0x600) gets the unflagged one. The contact reaches
/// the mood through the sync, and the report is consumed.
#[test]
fn a_reported_greeting_reads_the_mirrored_contact() {
    let run = |level: i32, concern: f32| {
        let db = greeting_book();
        let mut world = World::new();
        let mut conversations = exa_script::Conversations::default();
        let mut guard = exa_script::cast::Character::new(1, "Guard");
        guard.add_role(&db, 100);
        let mut human = exa_script::cast::Character::new(2, "Human");
        human.add_role(&db, 1);
        conversations.cast.characters = vec![guard, human];
        world.insert_resource(conversations);
        world.insert_resource(exa_script::Scripts {
            roles: db,
            narrator: None,
        });
        world.init_resource::<crate::script_cast::ContactMirror>();
        world.init_resource::<Messages<exa_script::ScriptEffect>>();
        let partner = world.spawn((Roles::default(), CharacterId(2))).id();
        let mut npc = Npc::new([0.0; 4]);
        npc.contacts.push(exa_ai::contacts::ContactRecord {
            id: partner.to_bits(),
            level,
            concern,
            ..Default::default()
        });
        npc.pass.greet = Some(partner.to_bits());
        let observer = world.spawn((npc, CharacterId(1))).id();
        world.run_system_once(run_greetings).unwrap();
        assert_eq!(world.get::<Npc>(observer).unwrap().pass.greet, None);
        let conversations = world.resource::<exa_script::Conversations>();
        let contact = *conversations.cast.get(1).unwrap().contact(2).unwrap();
        (
            conversations
                .engine
                .bubbles
                .list
                .first()
                .map(|b| String::from_utf8_lossy(&b.text).into_owned()),
            contact.state,
            contact.concern,
        )
    };
    assert_eq!(run(0, 0.0), (Some("Good day.".into()), 0, 0.0));
    assert_eq!(run(1, 5.0), (Some("Halt!".into()), 1, 5.0));
}

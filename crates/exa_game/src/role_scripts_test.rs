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

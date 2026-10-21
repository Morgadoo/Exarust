//! Tests of roles and relations.
//! (Included from roles.rs: `#[path = "roles_test.rs"] mod tests;`)

use super::*;
use exa_formats::roles::{INHERITS_PARENT, Name, RELATION_ROLE, SECTION_INTERACTION};

fn attitude_entry(target: u32, value: i8, tolerance: u8) -> RoleEntry {
    RoleEntry {
        kind: ATTITUDE,
        target,
        condition: 0,
        value: u32::from(value as u8) | (u32::from(tolerance) << 8),
        ..RoleEntry::default()
    }
}

fn script_entry(target: u32, condition: u32, text: &[u8]) -> RoleEntry {
    RoleEntry {
        kind: SECTION_ENCOUNTER,
        target,
        condition,
        value: 0,
        encounter: Some(text.to_vec()),
        ..RoleEntry::default()
    }
}

fn add(book: &mut RoleDb, id: u32, flags: u32, parent: u32, weight: u8, entries: Vec<RoleEntry>) {
    book.roles.retain(|r| r.id != id);
    book.roles.push(Role {
        id,
        flags,
        name: Name::from_bytes(format!("role {id}").as_bytes()),
        parent,
        weight: u32::from(weight),
        entries,
        ..Role::default()
    });
}

/// The shipped roles these tests use, with their attitude entries.
fn book() -> RoleDb {
    let mut b = RoleDb::default();
    add(
        &mut b,
        1,
        0,
        0,
        50,
        vec![
            attitude_entry(2, -40, 0),
            attitude_entry(6, -50, 0),
            attitude_entry(4, -100, 0),
        ],
    );
    add(
        &mut b,
        2,
        0,
        0,
        50,
        vec![
            attitude_entry(2, 40, 0),
            attitude_entry(1, -40, 0),
            attitude_entry(11, -100, 0),
        ],
    );
    add(&mut b, 4, 0, 0, 50, vec![]);
    add(&mut b, 6, 0, 0, 50, vec![]);
    add(&mut b, 7, 0, 1, 50, vec![]);
    add(&mut b, 8, INHERITS_PARENT, 2, 50, vec![]);
    add(
        &mut b,
        9,
        INHERITS_PARENT,
        2,
        50,
        vec![
            attitude_entry(1, -100, 0),
            attitude_entry(9, 80, 0),
            attitude_entry(10, 80, 0),
            attitude_entry(6, 60, 0),
        ],
    );
    add(
        &mut b,
        11,
        INHERITS_PARENT,
        4,
        100,
        vec![
            attitude_entry(1, -100, 0),
            attitude_entry(2, -100, 0),
            attitude_entry(11, 100, 60),
        ],
    );
    add(&mut b, 12, INHERITS_PARENT, 6, 50, vec![]);
    add(&mut b, 14, INHERITS_PARENT, 1, 50, vec![]);
    add(&mut b, 16, 0, 15, 100, vec![attitude_entry(16, -100, 0)]);
    add(
        &mut b,
        21,
        RELATION_ROLE,
        0,
        50,
        vec![attitude_entry(0, 0, 100)],
    );
    add(
        &mut b,
        39,
        RELATION_ROLE,
        13,
        50,
        vec![attitude_entry(0, -100, 50)],
    );
    add(&mut b, 41, INHERITS_PARENT, 12, 0, vec![]);
    b
}

fn roles_of(book: &RoleDb, ids: &[u32]) -> Roles {
    let mut r = Roles::default();
    for id in ids {
        r.add(book, *id);
    }
    r
}

fn rate(book: &RoleDb, a: &[u32], b: &[u32]) -> Rating {
    let mine = roles_of(book, a);
    let theirs = roles_of(book, b);
    let mut relations = Relations::default();
    attitude(
        book,
        &mine,
        &mut relations,
        &Partner {
            id: 5,
            roles: &theirs,
            former: None,
        },
        2,
    )
}

/// Golem Sentry adds Golem and Construct after it, each counted once; a
/// second add changes nothing; Golem added explicitly counts Construct
/// twice.
#[test]
fn adding_a_role_adds_its_inherited_parents_newest_first() {
    let b = book();
    let mut r = Roles::default();
    assert!(r.add(&b, 41));
    let seen: Vec<_> =
        r.0.iter()
            .map(|h| (h.id, h.explicit, h.inherited))
            .collect();
    assert_eq!(seen, vec![(41, true, 0), (12, false, 1), (6, false, 1)]);
    assert_eq!(r.0[0].weight, 0.0);
    assert_eq!(r.0[1].weight, 0.5);
    let before = r.clone();
    assert!(r.add(&b, 41));
    assert_eq!(r, before);
    assert!(!r.add(&b, 12));
    let seen: Vec<_> =
        r.0.iter()
            .map(|h| (h.id, h.explicit, h.inherited))
            .collect();
    assert_eq!(seen, vec![(41, true, 0), (12, true, 1), (6, false, 2)]);
}

/// Player has Human as its parent but does not inherit it; the new story
/// character adds Player, Human, then the unknown background (which
/// counts Human once more).
#[test]
fn a_new_story_character_is_player_human_and_its_background() {
    let b = book();
    let r = Roles::new_character(&b, 0);
    let seen: Vec<_> =
        r.0.iter()
            .map(|h| (h.id, h.explicit, h.inherited))
            .collect();
    assert_eq!(seen, vec![(14, true, 0), (1, true, 1), (7, true, 0)]);
    let arena = Roles::arena(&b);
    assert_eq!(arena.ids().collect::<Vec<_>>(), vec![16]);
    let none = Roles::new_character(&b, 4);
    assert_eq!(none.ids().collect::<Vec<_>>(), vec![1, 7]);
}

/// Removing Golem Sentry drops it and counts Golem down; Golem at a count
/// of 1 goes even though it was also explicit, and Construct at 2 stays at
/// 1. An inherited-only role is not removed.
#[test]
fn removing_a_role_counts_its_parents_down() {
    let b = book();
    let mut r = Roles::default();
    r.add(&b, 41);
    r.add(&b, 12);
    r.remove(&b, 6);
    assert_eq!(r.ids().collect::<Vec<_>>(), vec![41, 12, 6]);
    r.remove(&b, 41);
    let seen: Vec<_> = r.0.iter().map(|h| (h.id, h.inherited)).collect();
    assert_eq!(seen, vec![(6, 1)]);
}

/// The matrix of the shipped roles (familiarity 0): a zombie toward a human
/// −20, a skeleton −50, a demon −100 and +100 at tolerance 60 to another
/// demon, a skeleton toward a golem +30, a human toward a demon −50, and the
/// unaligned arena fighter −100 to another.
#[test]
fn the_first_matching_entry_blends_with_the_familiarity() {
    let b = book();
    let zombie = [8];
    let human = [1];
    assert_eq!(rate(&b, &zombie, &human).attitude, -20.0);
    assert_eq!(rate(&b, &[9], &human).attitude, -50.0);
    assert_eq!(rate(&b, &[11], &human).attitude, -100.0);
    let demons = rate(&b, &[11], &[11]);
    assert_eq!((demons.attitude, demons.tolerance), (100.0, 60.0));
    assert_eq!(rate(&b, &[9], &[12]).attitude, 30.0);
    assert_eq!(rate(&b, &human, &[11]).attitude, -50.0);
    assert_eq!(rate(&b, &[16], &[16]).attitude, -100.0);
    let stranger = rate(&b, &[7], &[7]);
    assert_eq!((stranger.attitude, stranger.tolerance), (0.0, 0.0));
}

/// A familiarity of 20 blends: a skeleton (weight 50) toward a human is
/// −100·0.5 + 0.5·20 = −40, and the relation keeps that as its last
/// attitude. A relation role decides first: Truce gives 10 at tolerance 100.
#[test]
fn relation_roles_decide_before_own_roles() {
    let b = book();
    let skeleton = roles_of(&b, &[9]);
    let human = roles_of(&b, &[1]);
    let mut rel = Relations::default();
    rel.create(5, 20.0, 0, 2);
    let partner = Partner {
        id: 5,
        roles: &human,
        former: None,
    };
    let r = attitude(&b, &skeleton, &mut rel, &partner, 2);
    assert_eq!(r.familiarity, 20.0);
    assert_eq!(r.attitude, -40.0);
    assert_eq!(rel.find(5).unwrap().last, -40.0);
    assert!(!rel.add_role(&b, &skeleton, 5, 21, 0, 2));
    let r = attitude(&b, &skeleton, &mut rel, &partner, 2);
    assert_eq!((r.attitude, r.tolerance), (10.0, 100.0));
    assert!(rel.remove_role(5, 21, 2));
    assert!(!rel.remove_role(5, 21, 2));
    assert!(rel.find(5).unwrap().roles.as_ref().unwrap().is_empty());
}

/// An entry aimed at one character holds only in its locale: XA_Derrin's
/// entry for character 0x80000008 of locale 2.
#[test]
fn an_entry_for_one_character_holds_in_its_locale() {
    let mut b = book();
    let mut e = attitude_entry(0x8000_0008, 50, 0);
    e.condition = 0xC000_0002;
    add(&mut b, 19, 0, 1, 50, vec![e]);
    let derrin = roles_of(&b, &[19]);
    let other = Roles::default();
    let mut rel = Relations::default();
    let at = |rel: &mut Relations, id, locale| {
        attitude(
            &b,
            &derrin,
            rel,
            &Partner {
                id,
                roles: &other,
                former: None,
            },
            locale,
        )
        .attitude
    };
    assert_eq!(at(&mut rel, 0x8000_0008, 2), 25.0);
    assert_eq!(at(&mut rel, 0x8000_0008, 3), 0.0);
    assert_eq!(at(&mut rel, 0x8000_0009, 2), 0.0);
}

/// Global characters share one table by their low 28 bits; level
/// characters go to the current locale's table, which a locale 0 keeps
/// unstored. A locale chosen while another is current stays unchosen.
#[test]
fn level_characters_are_kept_per_locale() {
    let mut rel = Relations::default();
    rel.create(0x1000_0005, 1.0, 10, 2);
    assert_eq!(rel.find(5).unwrap().base, 1.0);
    rel.create(0x8000_0005, 2.0, 10, 2);
    assert_eq!(rel.locale(), Some(2));
    assert_eq!(rel.levels.len(), 1);
    rel.select(3);
    assert_eq!(rel.locale(), Some(2));
    assert_eq!(rel.find(0x8000_0005).unwrap().base, 2.0);
    let mut loose = Relations::default();
    loose.create(0x8000_0001, 3.0, 0, 0);
    assert_eq!(loose.locale(), Some(0));
    assert!(loose.levels.is_empty());
    assert_eq!(loose.create(0, 4.0, 0, 2), 0.0);
}

/// The last attitude of a partner never met is 0 and creates its relation;
/// a former level character's relation moves to the global table.
#[test]
fn the_last_attitude_moves_a_former_level_character() {
    let none = Roles::default();
    let mut rel = Relations::default();
    rel.create(0x8000_0003, 0.0, 0, 2);
    rel.find_mut(0x8000_0003).unwrap().last = 33.0;
    let partner = Partner {
        id: 7,
        roles: &none,
        former: Some((2, 0x8000_0003)),
    };
    assert_eq!(rel.last_attitude(&partner, 50, 2), 33.0);
    assert!(rel.find(0x8000_0003).is_none());
    let stranger = Partner {
        id: 9,
        roles: &none,
        former: None,
    };
    assert_eq!(rel.last_attitude(&stranger, 50, 2), 0.0);
    assert_eq!(rel.find(9).unwrap().met_ms, 50);
    rel.add_familiarity(9, 5.0);
    rel.add_familiarity(10, 5.0);
    assert!(rel.find(10).is_none());
    rel.set_familiarity(10, -7.0, 60, 2);
    assert_eq!(
        (rel.find(9).unwrap().base, rel.find(10).unwrap().base),
        (5.0, -7.0)
    );
}

/// A relation role gains each value once; added again with no value it is
/// left; a role the book does not hold is reported.
#[test]
fn relation_roles_keep_their_values_once() {
    let b = book();
    let none = Roles::default();
    let mut rel = Relations::default();
    assert!(!rel.add_role(&b, &none, 4, 39, 12, 2));
    assert!(!rel.add_role(&b, &none, 4, 39, 12, 2));
    assert!(!rel.add_role(&b, &none, 4, 39, 13, 2));
    let held = rel.find(4).unwrap().roles.clone().unwrap();
    assert_eq!(held[0].values, vec![12, 13]);
    assert_eq!(held[0].weight, 0.5);
    assert!(rel.add_role(&b, &none, 4, 999, 0, 2));
}

/// Meeting someone creates the relation or renews its time, and returns the
/// encounter scripts for anyone, for that character in its locale, and for
/// each of its roles, in order.
#[test]
fn an_encounter_returns_the_scripts_that_match() {
    let mut b = book();
    add(
        &mut b,
        42,
        INHERITS_PARENT,
        12,
        0,
        vec![
            script_entry(0, 0, b"anyone"),
            script_entry(0x8000_0002, 0xC000_0002, b"derrin"),
            script_entry(1, 0, b"human"),
            script_entry(6, 0, b"construct"),
        ],
    );
    let golem = roles_of(&b, &[42]);
    let human = roles_of(&b, &[1]);
    let mut rel = Relations::default();
    let partner = Partner {
        id: 0x8000_0002,
        roles: &human,
        former: None,
    };
    let met = encounter(&b, &golem, &mut rel, &partner, 100, 2);
    let got: Vec<_> = met.iter().map(|e| (e.script, e.partner_role)).collect();
    assert_eq!(
        got,
        vec![
            (&b"anyone"[..], None),
            (&b"derrin"[..], None),
            (&b"human"[..], Some(1))
        ]
    );
    assert_eq!(rel.find(0x8000_0002).unwrap().met_ms, 100);
    encounter(&b, &golem, &mut rel, &partner, 200, 2);
    assert_eq!(rel.find(0x8000_0002).unwrap().met_ms, 200);
    assert_eq!(encounter(&b, &golem, &mut rel, &partner, 300, 5).len(), 2);
}

/// The entry finder takes each role's first entry aimed at one of the
/// partner's roles, then at a relation role toward it: an interaction
/// condition behind an attitude-only entry for the same role is not found.
#[test]
fn the_entry_finder_takes_the_first_entry_per_target() {
    let mut b = book();
    let condition = RoleEntry {
        kind: SECTION_INTERACTION,
        target: 7,
        condition: 0,
        value: 0,
        interaction: Some(b"result = 1;".to_vec()),
        ..RoleEntry::default()
    };
    add(&mut b, 45, 0, 0, 0, vec![condition.clone()]);
    add(
        &mut b,
        46,
        0,
        0,
        0,
        vec![attitude_entry(7, 1, 0), condition],
    );
    let player = roles_of(&b, &[7]);
    let rel = Relations::default();
    let partner = Partner {
        id: 1,
        roles: &player,
        former: None,
    };
    let boss = roles_of(&b, &[45]);
    let found = entry_for(&b, &boss, &rel, &partner, SECTION_INTERACTION).unwrap();
    assert_eq!(found.script(SECTION_INTERACTION), Some(&b"result = 1;"[..]));
    let other = roles_of(&b, &[46]);
    assert!(entry_for(&b, &other, &rel, &partner, SECTION_INTERACTION).is_none());
}

/// The finder walks the section table, which has no corpse section: an
/// entry for the Player holding a corpse script gives none through it,
/// though the entry's own corpse accessor finds the script.
#[test]
fn the_entry_finder_has_no_corpse_section() {
    let mut b = book();
    let corpse = RoleEntry {
        kind: SECTION_CORPSE,
        target: 7,
        corpse: Some(b"Actor.Loot();".to_vec()),
        ..RoleEntry::default()
    };
    assert_eq!(corpse.script(SECTION_CORPSE), Some(&b"Actor.Loot();"[..]));
    add(&mut b, 47, 0, 0, 0, vec![corpse]);
    let player = roles_of(&b, &[7]);
    let partner = Partner {
        id: 1,
        roles: &player,
        former: None,
    };
    let holder = roles_of(&b, &[47]);
    let rel = Relations::default();
    assert!(entry_for(&b, &holder, &rel, &partner, SECTION_CORPSE).is_none());
}

/// A version 7 record keeps its roles as saved; an older record adds them
/// again in their order, so Skeleton brings back Undead.
#[test]
fn a_record_gives_its_roles() {
    use exa_formats::characters::{CharacterRecord, Role as HeldRecord};
    let b = book();
    let mut record = CharacterRecord {
        version: 7,
        roles: vec![HeldRecord {
            id: 9,
            weight: 0.0,
            explicit: true,
            inherited: 0,
            variables: vec![3],
        }],
        ..Default::default()
    };
    let saved = Roles::of_record(&record, &b);
    assert_eq!(saved.ids().collect::<Vec<_>>(), vec![9]);
    assert_eq!(saved.0[0].values, vec![3]);
    record.version = 6;
    let legacy = Roles::of_record(&record, &b);
    assert_eq!(legacy.ids().collect::<Vec<_>>(), vec![9, 2]);
    assert!(invalid(&roles_of(&b, &[999]), &Relations::default(), &b, 0));
    assert!(!invalid(&legacy, &Relations::default(), &b, 0));
}

/// For each of the dead character's roles, the corpse section of its first
/// entry aimed at the Player: a role whose first such entry lacks the
/// section gives nothing, even with a later entry that has it.
#[test]
fn a_corpse_script_is_found_for_the_player() {
    let mut b = book();
    let corpse_entry = |target: u32, text: &[u8]| RoleEntry {
        kind: SECTION_CORPSE,
        target,
        corpse: Some(text.to_vec()),
        ..RoleEntry::default()
    };
    add(&mut b, 47, 0, 0, 0, vec![corpse_entry(7, b"Actor.Loot();")]);
    add(&mut b, 48, 0, 0, 0, vec![corpse_entry(6, b"Other();")]);
    add(
        &mut b,
        49,
        0,
        0,
        0,
        vec![attitude_entry(7, 10, 0), corpse_entry(7, b"Later();")],
    );
    let corpse = roles_of(&b, &[47, 48, 49]);
    let scripts = corpse_scripts(&b, &corpse);
    assert_eq!(scripts.len(), 1);
    assert_eq!(scripts[0].role, 47);
    assert_eq!(scripts[0].script, b"Actor.Loot();");
    assert!(corpse_scripts(&b, &roles_of(&b, &[7])).is_empty());
}

/// The interaction wrapper hands out the section of the entry the finder
/// answers.
#[test]
fn the_condition_wrapper_uses_the_finder() {
    let mut b = book();
    add(
        &mut b,
        50,
        0,
        0,
        0,
        vec![RoleEntry {
            kind: SECTION_INTERACTION,
            target: 7,
            interaction: Some(b"result = 1;".to_vec()),
            ..RoleEntry::default()
        }],
    );
    let player = roles_of(&b, &[7]);
    let partner = Partner {
        id: 1,
        roles: &player,
        former: None,
    };
    let rel = Relations::default();
    assert_eq!(
        interaction_condition(&b, &Roles::default(), &rel, &partner),
        None
    );
    assert_eq!(
        interaction_condition(&b, &roles_of(&b, &[50]), &rel, &partner),
        Some(&b"result = 1;"[..])
    );
}

/// StartDialogue copies the calling context; ChangeRole is the original's
/// missing-function error. The unit registers them in this order.
#[test]
fn the_roles_scripts_are_registered_and_start_dialogue_copies_its_context() {
    assert_eq!(SCRIPT_FUNCTIONS, ["StartDialogue", "ChangeRole"]);
    let context = DialogueStart {
        owner: 3,
        partner: 4,
        role: 21,
        partner_role: 7,
    };
    assert_eq!(start_dialogue(context), context);
    let error = change_role().unwrap_err();
    assert_eq!(error.to_string(), "Function \"ChangeRole\" does not exist.");
}

/// A level character's relation removal selects the locale's table when
/// none is current, even when it then finds no relation role.
#[test]
fn removing_a_level_relation_selects_the_level_table() {
    let mut rel = Relations::default();
    assert!(!rel.remove_role(LOCAL_ID | 5, 21, 9));
    assert_eq!(rel.locale(), Some(9));
    assert_eq!(rel.levels.len(), 1);
}

//! Tests of the role database and dialogue graphs.
//! (Included from roles.rs: `#[path = "roles_test.rs"] mod tests;`)

use super::*;

fn u32s(v: &[u32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

fn name(s: &str) -> Vec<u8> {
    let mut n = s.as_bytes().to_vec();
    n.resize(16, 0);
    n
}

fn sized(b: &[u8]) -> Vec<u8> {
    let mut out = u32s(&[b.len() as u32]);
    out.extend(b);
    out
}

/// A self-sized record: the size word counts itself.
fn record(body: Vec<u8>) -> Vec<u8> {
    let mut out = u32s(&[body.len() as u32 + 4]);
    out.extend(body);
    out
}

fn branch(
    flags: u32,
    lines: &[(u32, &str)],
    cond: &str,
    act: &str,
    replies: &[Vec<u8>],
) -> Vec<u8> {
    let mut b = u32s(&[flags, lines.len() as u32]);
    for (speaker, text) in lines {
        b.extend(u32s(&[*speaker]));
        b.extend(sized(text.as_bytes()));
    }
    b.extend(sized(cond.as_bytes()));
    b.extend(sized(act.as_bytes()));
    b.extend(u32s(&[replies.len() as u32]));
    for r in replies {
        b.extend(r);
    }
    record(b)
}

fn node(flags: u32, title: &str, links: &[u32], branches: &[Vec<u8>]) -> Vec<u8> {
    let mut b = u32s(&[flags]);
    b.extend(name(title));
    b.extend(u32s(&[
        300f32.to_bits(),
        150f32.to_bits(),
        links.len() as u32,
    ]));
    b.extend(u32s(links));
    b.extend(u32s(&[branches.len() as u32]));
    for br in branches {
        b.extend(br);
    }
    record(b)
}

/// A graph block: target, size, two actors, the nodes (empty ones as 0).
fn graph(target: u32, nodes: &[Option<Vec<u8>>]) -> Vec<u8> {
    let mut body = u32s(&[2]);
    for (n, colour) in [("Actor 1", 0x80FF_4040u32), ("Actor 2", 0x8040_C840)] {
        body.extend(name(n));
        body.extend(u32s(&[colour, 0, 0]));
    }
    body.extend(u32s(&[nodes.len() as u32]));
    for n in nodes {
        match n {
            Some(n) => body.extend(n),
            None => body.extend(u32s(&[0])),
        }
    }
    let mut b = u32s(&[target, body.len() as u32 + 8]);
    b.extend(body);
    b
}

fn entry(kind: u32, target: u32, cond: u32, value: u32, sections: &[u8]) -> Vec<u8> {
    let mut b = u32s(&[kind, target, cond, value]);
    b.extend(sections);
    record(b)
}

fn role(
    flags: u32,
    nm: &str,
    parent: u32,
    weight: u32,
    init: &str,
    vars: &[&str],
    entries: &[Vec<u8>],
) -> Vec<u8> {
    let mut b = u32s(&[flags]);
    b.extend(name(nm));
    b.extend(u32s(&[
        parent,
        0,
        weight,
        vars.len() as u32,
        0,
        0,
        entries.len() as u32,
        init.len() as u32,
    ]));
    b.extend(init.as_bytes());
    for v in vars {
        b.extend(name(v));
        b.extend(u32s(&[4]));
    }
    for e in entries {
        b.extend(e);
    }
    record(b)
}

fn database(records: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let mut table = Vec::new();
    let mut data: Vec<u8> = Vec::new();
    for (id, r) in records {
        table.extend(u32s(&[*id, 0, data.len() as u32, r.len() as u32]));
        data.extend(r);
    }
    let mut b = u32s(&[MAGIC, table.len() as u32]);
    b.extend(table);
    b.extend(data);
    b
}

fn golem_service() -> Vec<u8> {
    let follow = node(
        0x0100_004F,
        "Follow",
        &[],
        &[branch(
            0x7F0,
            &[(2, "Follow.")],
            "result = (Role.Activity ! 4);",
            "Role.Activity = 4;",
            &[],
        )],
    );
    let mut sections = graph(0x1E, &[Some(follow), None]);
    sections.extend(sized(b"result = 1;"));
    role(
        0,
        "Golem Srv",
        12,
        100,
        "Actor.Activity = Role.Activity;",
        &["Activity"],
        &[
            entry(0x1042, 0x1E, 0, 0x6464, &sections),
            entry(0x1004, 1, 0, 0xA6, &sized(b"ChangeRole(09);")),
        ],
    )
}

#[test]
fn a_role_record_reads_every_section() {
    let b = database(&[
        (46, golem_service()),
        (
            21,
            role(
                1,
                "Truce",
                0,
                50,
                "",
                &[],
                &[entry(0x1000, 0, 0, 0x6400, &[])],
            ),
        ),
    ]);
    let db = RoleDb::parse(&b).unwrap();
    assert_eq!(db.roles.len(), 2);
    let srv = db.get(46).unwrap();
    assert_eq!(srv.name.text(), "Golem Srv");
    assert_eq!((srv.parent, srv.weight), (12, 100));
    assert_eq!(srv.init_script, b"Actor.Activity = Role.Activity;");
    assert_eq!(srv.variables[0].name.bytes(), b"Activity");
    assert_eq!(srv.variables[0].kind, 4);
    let e = &srv.entries[0];
    assert_eq!((e.target, e.attitude(), e.tolerance()), (0x1E, 100, 100));
    assert!(e.has_attitude());
    assert_eq!(e.interaction.as_deref(), Some(&b"result = 1;"[..]));
    let d = e.dialogue.as_ref().unwrap();
    assert_eq!(d.target, 0x1E);
    assert_eq!(d.actors.len(), 2);
    assert_eq!(d.actors[1].colour, 0x8040_C840);
    assert_eq!(d.nodes.len(), 2);
    assert!(d.nodes[1].is_none());
    let follow = d.node(1).unwrap();
    assert_eq!(follow.flags, 0x0100_004F);
    assert_eq!(follow.position, [300.0, 150.0]);
    assert_eq!(
        follow.branches[0].lines[0],
        Line {
            speaker: 2,
            text: b"Follow.".to_vec()
        }
    );
    assert_eq!(follow.branches[0].action, b"Role.Activity = 4;");
    assert_eq!(
        srv.entries[1].encounter.as_deref(),
        Some(&b"ChangeRole(09);"[..])
    );
    assert_eq!(srv.entries[1].attitude(), -90);
    assert!(srv.dialogue_for(0x1E, 0).is_some());
    assert!(srv.dialogue_for(1, 0).is_none());
    let truce = db.get(21).unwrap();
    assert!(truce.is_relation_role() && truce.is_relation_role_byte());
    assert!(!truce.inherits_parent());
}

#[test]
fn replies_nest_and_topics_are_found_without_case() {
    let reply = branch(0x31, &[(2, "Of course.")], "", "", &[]);
    let ask = branch(
        0x30,
        &[(1, "Do you consider me a friend?")],
        "",
        "",
        &[reply],
    );
    let top = branch(
        0x7F0,
        &[(2, "Hello."), (2, "Well met.")],
        "",
        "Actor[0].LeaveDialogue;",
        &[ask],
    );
    let b = graph(
        0,
        &[
            None,
            Some(node(0x0100_000F, "How are you", &[0x1_0002], &[top])),
        ],
    );
    let d = Dialogue::parse(&b).unwrap();
    assert_eq!(d.find(&Name::from_bytes(b"HOW ARE YOU")), Some(2));
    assert_eq!(d.find(&Name::from_bytes(b"How are")), None);
    let n = d.node(2).unwrap();
    assert_eq!(n.links, [0x1_0002]);
    let top = &n.branches[0];
    assert_eq!(top.lines.len(), 2);
    assert_eq!(top.replies[0].replies[0].link(), 1);
    assert_eq!(top.replies[0].replies[0].lines[0].text, b"Of course.");
}

#[test]
fn malformed_graphs_are_errors() {
    let good = graph(
        0,
        &[Some(node(
            0,
            "Wait",
            &[],
            &[branch(0, &[(1, "Wait.")], "", "", &[])],
        ))],
    );
    for cut in [3, 9, 20, good.len() - 1] {
        assert!(Dialogue::parse(&good[..cut]).is_err(), "cut at {cut}");
    }
    // A block size past the data.
    let mut big = good.clone();
    big[4..8].copy_from_slice(&(good.len() as u32 + 1).to_le_bytes());
    assert!(Dialogue::parse(&big).is_err());
    // A branch whose size word is 0 cannot advance.
    let mut body = u32s(&[0]);
    body.extend(name("Stuck"));
    body.extend(u32s(&[0, 0, 0, 1, 0]));
    let zero = graph(0, &[Some(record(body))]);
    assert!(Dialogue::parse(&zero).is_err());
    let mut wrong = database(&[(1, golem_service())]);
    wrong[0] = 0;
    assert!(matches!(RoleDb::parse(&wrong), Err(Error::BadMagic { .. })));
}

#[test]
fn deep_nesting_is_refused() {
    let mut b = branch(0, &[], "", "", &[]);
    for _ in 0..80 {
        b = branch(0, &[], "", "", &[b]);
    }
    let g = graph(0, &[Some(node(0, "Deep", &[], &[b]))]);
    assert!(Dialogue::parse(&g).is_err());
}

/// The installed role database and narrator graph (skipped without the
/// game): all 49 roles parse; Explorer's "How are you" opens with one line
/// and six replies; the narrator has five nodes, Corlian first.
#[test]
fn shipped_roles_and_narrator() {
    let Some(dir) = exa_rpk::locate_install(None) else {
        return;
    };
    let game = exa_rpk::GameData::open(&dir).unwrap();
    let resource = game.pack(exa_rpk::PackKind::Resource).unwrap();
    let db = RoleDb::parse(resource.get("charroles.rdb").unwrap()).unwrap();
    assert_eq!(db.roles.len(), 49);
    let explorer = db.get(31).unwrap();
    assert_eq!(explorer.name.text(), "Explorer");
    let d = explorer.dialogue_for(0x1D, 0).unwrap();
    let how = d
        .node(d.find(&Name::from_bytes(b"How are you")).unwrap())
        .unwrap();
    assert_eq!(how.branches.len(), 1);
    assert_eq!(how.branches[0].replies.len(), 6);
    assert_eq!(
        db.get(46).unwrap().init_script,
        b"Actor.Activity = Role.Activity;"
    );
    let skeleton = db.get(9).unwrap();
    assert!(skeleton.inherits_parent());
    assert_eq!(skeleton.parent, 2);
    assert_eq!(skeleton.entry_for(1).unwrap().attitude(), -100);
    assert_eq!(db.roles.iter().filter(|r| r.is_relation_role()).count(), 8);
    assert_eq!(db.get(43).unwrap().init_script, b"Actor.Activity = 7;");
    let narrator = Dialogue::parse(resource.get("narrator.rcd").unwrap()).unwrap();
    assert_eq!(narrator.nodes.len(), 5);
    assert_eq!(narrator.node(1).unwrap().title.text(), "Corlian");
    assert_eq!(narrator.node(1).unwrap().branches[0].replies.len(), 3);
}

/// The Skeleton of the shipped table: weight 50, inheriting Undead, two
/// attitude entries (Human −100, Skeleton +80 with tolerance 0).
#[test]
fn a_role_reads_its_head_and_attitudes() {
    let skeleton = role(
        INHERITS_PARENT,
        "Skeleton",
        2,
        50,
        "",
        &[],
        &[
            entry(ATTITUDE, 1, 0, 0x9C, &[]),
            entry(ATTITUDE, 9, 0, 0x50, &[]),
        ],
    );
    let db = RoleDb::parse(&database(&[(9, skeleton)])).unwrap();
    let r = db.get(9).unwrap();
    assert_eq!(r.name.text(), "Skeleton");
    assert_eq!((r.parent, r.weight), (2, 50));
    assert!(r.inherits_parent() && !r.is_relation_role());
    assert_eq!(r.share(), 0.5);
    assert_eq!(r.entries[0].attitude(), -100);
    assert_eq!(r.entries[1].attitude(), 80);
    assert_eq!(r.entries[1].tolerance(), 0);
    assert_eq!(r.entry_for(9).unwrap().value, 0x50);
}

/// Faction 1's own entry: attitude +100 with tolerance 100 (`0x6464`); a
/// relation role (Truce) aims at the partner, target 0.
#[test]
fn tolerance_is_the_second_byte_and_a_relation_role_aims_at_the_partner() {
    let truce = role(
        RELATION_ROLE,
        "Truce",
        0,
        50,
        "",
        &[],
        &[entry(ATTITUDE, 0, 0, 0x6400, &[])],
    );
    let r = Role::parse(21, &truce).unwrap();
    assert!(r.is_relation_role());
    assert_eq!(r.entries[0].attitude(), 0);
    assert_eq!(r.entries[0].tolerance(), 100);
    assert!(r.entries[0].for_anyone());
    let faction = entry(ATTITUDE, 17, 0, 0x6464, &[]);
    let f = Role::parse(17, &role(0, "Faction 1", 15, 100, "", &[], &[faction])).unwrap();
    assert_eq!(f.share(), 1.0);
    assert_eq!(
        (f.entries[0].attitude(), f.entries[0].tolerance()),
        (100, 100)
    );
}

/// Script sections are found by their kind bits: the encounter after the
/// dialogue, the interaction condition after both; the dialogue and an
/// absent section have no script.
#[test]
fn scripts_are_found_by_their_bits() {
    let mut tail = graph(7, &[]);
    tail.extend(sized(b"ChangeRole(09);"));
    tail.extend(sized(b"result = 1;"));
    let e = entry(
        SECTION_DIALOGUE | SECTION_ENCOUNTER | SECTION_INTERACTION | ATTITUDE,
        7,
        0,
        0x32,
        &tail,
    );
    let r = Role::parse(1, &role(0, "Talker", 0, 50, "", &[], &[e])).unwrap();
    let e = &r.entries[0];
    assert_eq!(e.dialogue.as_ref().unwrap().target, 7);
    assert_eq!(e.script(SECTION_DIALOGUE), None);
    assert_eq!(e.script(SECTION_ENCOUNTER), Some(&b"ChangeRole(09);"[..]));
    assert_eq!(e.script(SECTION_INTERACTION), Some(&b"result = 1;"[..]));
    assert_eq!(e.script(SECTION_8), None);
}

/// The corpse script is read after the sections of bits 1 to 8 only
/// (Corlian's entry carries it alone, right after the head), so an
/// interaction condition stored with it reads the same bytes.
#[test]
fn the_corpse_script_follows_the_first_four_sections() {
    let alone = entry(SECTION_CORPSE, 7, 0, 0, &sized(b"GameEvent(00C10EC0);"));
    let mut tail = sized(b"x");
    tail.extend(sized(b"GameEvent(1);"));
    let after = entry(SECTION_EXTRA | SECTION_CORPSE, 7, 0, 0, &tail);
    let both = entry(
        SECTION_CORPSE | SECTION_INTERACTION,
        7,
        0,
        0,
        &sized(b"result = 1;"),
    );
    let r = Role::parse(
        25,
        &role(0, "Corlian", 1, 50, "", &[], &[alone, after, both]),
    )
    .unwrap();
    assert_eq!(
        r.entries[0].script(SECTION_CORPSE),
        Some(&b"GameEvent(00C10EC0);"[..])
    );
    assert_eq!(
        r.entries[1].script(SECTION_CORPSE),
        Some(&b"GameEvent(1);"[..])
    );
    assert_eq!(r.entries[2].corpse.as_deref(), Some(&b"result = 1;"[..]));
    assert_eq!(
        r.entries[2].interaction.as_deref(),
        Some(&b"result = 1;"[..])
    );
}

/// An entry aimed at one character: both top bits of the condition set and
/// the locale in the low 30 bits (Derrin's entry, locale 2).
#[test]
fn a_character_entry_names_its_locale() {
    let e = entry(SECTION_ENCOUNTER, 0x8000_0008, 0xC000_0002, 0, &sized(b""));
    let r = Role::parse(19, &role(0, "XA_Derrin", 1, 50, "", &[], &[e])).unwrap();
    assert_eq!(r.entries[0].locale(), Some(2));
    assert!(r.entries[0].for_character() && !r.entries[0].for_anyone());
    assert_eq!(r.entries[0].script(SECTION_ENCOUNTER), Some(&b""[..]));
}

/// Malformed records are errors: an entry under its head, entries past the
/// record, a section running past its entry, and the wrong magic.
#[test]
fn malformed_roles_are_errors() {
    let good = role(0, "R", 0, 50, "", &[], &[entry(ATTITUDE, 1, 0, 0, &[])]);
    let mut short = role(0, "R", 0, 50, "", &[], &[u32s(&[8, 0])]);
    assert!(Role::parse(1, &short).is_err());
    short[0x30..0x34].copy_from_slice(&3u32.to_le_bytes());
    assert!(Role::parse(1, &short).is_err());
    let mut many = good.clone();
    many[0x30..0x34].copy_from_slice(&2u32.to_le_bytes());
    assert!(Role::parse(1, &many).is_err());
    let runs = entry(SECTION_ENCOUNTER, 1, 0, 0, &u32s(&[100]));
    assert!(Role::parse(1, &role(0, "R", 0, 50, "", &[], &[runs])).is_err());
    let mut db = database(&[(1, good)]);
    db[..4].copy_from_slice(&0xDBCB_0D00u32.to_le_bytes());
    assert!(RoleDb::parse(&db).is_err());
    assert!(RoleDb::parse(&[1, 2, 3]).is_err());
}

/// A locale is found by its level's name, without the extension and
/// ignoring case; an unknown name or a bad database has none.
#[test]
fn a_level_names_its_locale() {
    let entry = |n: &str, word: u32| {
        let mut p = n.as_bytes().to_vec();
        p.resize(20, 0);
        p.extend(u32s(&[word]));
        p.resize(44, 0);
        p
    };
    let db = database(&[(1, entry("Exanima", 0)), (2, entry("Exanima01", 1))]);
    assert_eq!(locale_id(&db, "exanima01.rfc"), Some(2));
    assert_eq!(locale_id(&db, "Exanima"), Some(1));
    assert_eq!(locale_id(&db, "arena7.rfc"), None);
    assert_eq!(locale_id(&[0; 3], "exanima01.rfc"), None);
}

/// The shared role list: adding pulls in inherited parents (newest first),
/// removing an explicit role counts the parents down, and the 12-byte
/// stored-list read lands inside the values of a role that has them.
#[test]
fn held_roles_add_inherit_remove_and_pack() {
    let mut db = RoleDb::default();
    db.roles.push(Role {
        id: 6,
        parent: 3,
        flags: INHERITS_PARENT,
        weight: 50,
        ..Role::default()
    });
    db.roles.push(Role {
        id: 3,
        weight: 100,
        ..Role::default()
    });
    let mut list = Vec::new();
    assert!(add_held_role(&db, &mut list, 6));
    let ids: Vec<(u32, bool, u8)> = list
        .iter()
        .map(|r| (r.id, r.explicit, r.inherited))
        .collect();
    assert_eq!(ids, [(6, true, 0), (3, false, 1)]);
    assert_eq!(list[0].weight, 0.5);
    assert_eq!(list[1].weight, 1.0);
    // Asking again changes nothing and reports the role as held.
    assert!(add_held_role(&db, &mut list, 6));
    assert_eq!(list.len(), 2);
    // Removing the child drops the parent held once.
    assert!(remove_held_role(&db, &mut list, 6));
    assert!(list.is_empty());
    // A parent that is not explicit cannot be removed on its own.
    assert!(add_held_role(&db, &mut list, 6));
    assert!(!remove_held_role(&db, &mut list, 3), "not explicit");

    let list = vec![
        HeldRole {
            id: 0x10,
            weight: 0.25,
            explicit: true,
            inherited: 0,
            values: vec![7, 8],
        },
        HeldRole {
            id: 0x20,
            weight: 1.0,
            explicit: true,
            inherited: 0,
            values: Vec::new(),
        },
    ];
    // Entry 0 is 12 bytes plus its two values, so the word at 12 is the
    // first value and the second role starts at byte 20, not 12.
    assert_eq!(held_role_word(&list, 0), 0x10);
    assert_eq!(held_role_word(&list, 1), 7);
    // Byte 24 is the second entry's weight, not its id (byte 20).
    assert_eq!(held_role_word(&list, 2), 0x3F80_0000);
    assert_eq!(held_role_word(&list, 3), 0);
    assert_eq!(held_role_bytes(&list).len(), 20 + 12);
}

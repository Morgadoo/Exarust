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
    let narrator = Dialogue::parse(resource.get("narrator.rcd").unwrap()).unwrap();
    assert_eq!(narrator.nodes.len(), 5);
    assert_eq!(narrator.node(1).unwrap().title.text(), "Corlian");
    assert_eq!(narrator.node(1).unwrap().branches[0].replies.len(), 3);
}

//! Tests of the role list, topic memory, bubbles and the conversation
//! engine, on synthetic role databases.

use exa_formats::roles::{
    Branch, Dialogue, DialogueActor, Line, NODE_ENTRY, NODE_NOT_ENTRY, Name, Node, Role, RoleDb,
    RoleEntry,
};

use crate::bubbles::{self, Bubbles, Phase, Speaker};
use crate::cast::{Cast, Character, Contact};
use crate::dialogue::{self, Effect, Engine, Library, Scene, State};
use crate::topics::{self, ACTIVATED, COUNTING, MemoryKey};

fn name(s: &str) -> Name {
    Name::from_bytes(s.as_bytes())
}

fn branch(
    flags: u32,
    lines: &[(u32, &str)],
    cond: &str,
    act: &str,
    replies: Vec<Branch>,
) -> Branch {
    Branch {
        flags,
        lines: lines
            .iter()
            .map(|&(speaker, t)| Line {
                speaker,
                text: t.as_bytes().to_vec(),
            })
            .collect(),
        condition: cond.as_bytes().to_vec(),
        action: act.as_bytes().to_vec(),
        replies,
    }
}

fn node(flags: u32, title: &str, links: &[u32], branches: Vec<Branch>) -> Option<Node> {
    Some(Node {
        flags,
        title: name(title),
        position: [0.0; 2],
        links: links.to_vec(),
        branches,
    })
}

fn graph(target: u32, nodes: Vec<Option<Node>>) -> Dialogue {
    Dialogue {
        target,
        actors: ["Actor 1", "Actor 2"]
            .iter()
            .map(|n| DialogueActor {
                name: name(n),
                ..DialogueActor::default()
            })
            .collect(),
        nodes,
    }
}

fn entry(kind: u32, target: u32, dialogue: Option<Dialogue>) -> RoleEntry {
    RoleEntry {
        kind,
        target,
        dialogue,
        ..RoleEntry::default()
    }
}

fn role(id: u32, flags: u32, parent: u32, entries: Vec<RoleEntry>) -> Role {
    Role {
        id,
        flags,
        name: name(&format!("Role{id}")),
        parent,
        weight: 50,
        entries,
        ..Role::default()
    }
}

fn scene<'a>(
    db: &'a RoleDb,
    narrator: Option<&'a Dialogue>,
    cast: &'a mut Cast,
    now: u32,
) -> Scene<'a> {
    Scene {
        library: Library {
            roles: db,
            narrator,
        },
        cast,
        now,
        world_time: 1000 + now / 1000,
        player: Some(PLAYER),
        player2: None,
        locale: 2,
        effects: Vec::new(),
    }
}

const GUARD: u32 = 1;
const PLAYER: u32 = 2;
const TRAVELLER: u32 = 3;

fn person(id: u32, roles: &[u32], db: &RoleDb) -> Character {
    let mut c = Character::new(id, &format!("C{id}"));
    for &r in roles.iter().rev() {
        c.add_role(db, r);
    }
    c
}

#[test]
fn roles_inherit_and_count_their_parents() {
    let db = RoleDb {
        roles: vec![
            role(41, 0x1_0000, 12, vec![]),
            role(12, 0x1_0000, 6, vec![]),
            role(6, 0, 0, vec![]),
        ],
    };
    let mut c = Character::new(1, "Golem");
    assert!(c.add_role(&db, 41));
    let ids: Vec<(u32, bool, u8)> = c
        .roles
        .iter()
        .map(|r| (r.id, r.explicit, r.inherited))
        .collect();
    assert_eq!(ids, [(41, true, 0), (12, false, 1), (6, false, 1)]);
    assert_eq!(c.roles[0].weight, 0.5);
    // Asking for a parent marks it explicit and counts its own parents.
    c.add_role(&db, 12);
    let ids: Vec<(u32, bool, u8)> = c
        .roles
        .iter()
        .map(|r| (r.id, r.explicit, r.inherited))
        .collect();
    assert_eq!(ids, [(41, true, 0), (12, true, 1), (6, false, 2)]);
    // Removing the child drops a parent held once, explicit or not.
    assert!(c.remove_role(&db, 41));
    let ids: Vec<(u32, u8)> = c.roles.iter().map(|r| (r.id, r.inherited)).collect();
    assert_eq!(ids, [(6, 1)]);
    // Only explicit roles are removed.
    assert!(!c.remove_role(&db, 6));
}

#[test]
fn relation_roles_file_memory_blocks() {
    let mut c = Character::new(1, "A");
    c.add_relation(2, 0x1E, 0);
    c.add_relation(2, 0x1E, 7);
    c.add_relation(2, 0x1E, 7);
    let r = c.relation(2).unwrap().role(0x1E).unwrap();
    assert_eq!(r.memories.len(), 1);
    assert_eq!(r.memories[0].key, 7);
    assert!(c.remove_relation(2, 0x1E));
    assert!(!c.holds_toward(2, 0x1E));
    assert!(!c.remove_relation(2, 0x1E));
}

#[test]
fn topic_cooldowns_follow_the_table() {
    // Class 9 is ten minutes.
    assert!(topics::available(0, 0x9, 5));
    assert!(!topics::available(1000, 0x9, 1599));
    assert!(topics::available(1000, 0x9, 1600));
    // Class 0 never again, class 15 always.
    assert!(!topics::available(1000, 0x0, 1_000_000));
    assert!(topics::available(1000, 0xF, 1000));
    assert_eq!(topics::COOLDOWN[1], 31_536_000);
    assert_eq!(topics::COOLDOWN[13], 10);
}

#[test]
fn topics_activate_after_their_hits_and_delay() {
    // Two hits, 5 s delay (class 2), a 60 s window (class 11).
    let flags = 0x200 | 0x20_0000 | 0xB_0000;
    let mut w = 0;
    assert!(topics::activate(&mut w, flags, 0, 100));
    assert_eq!(w, COUNTING | 1);
    assert!(!topics::available(w, flags, 100));
    assert!(topics::activate(&mut w, flags, 0, 101));
    assert_eq!(w, ACTIVATED | 106);
    assert!(!topics::available(w, flags, 105));
    assert!(topics::available(w, flags, 106));
    assert!(topics::available(w, flags, 165));
    assert!(!topics::available(w, flags, 166));
    // Hit again inside the window: re-armed now, without the delay.
    assert!(topics::activate(&mut w, flags, 0, 120));
    assert_eq!(w, ACTIVATED | 120);
    // After the window: re-armed with the delay.
    assert!(topics::activate(&mut w, flags, 0, 200));
    assert_eq!(w, ACTIVATED | 205);
    // No re-activation while active with 0x4000.
    assert!(!topics::activate(&mut w, flags | 0x4000, 0, 300));
    assert_eq!(topics::used_time(w), 0);
    assert_eq!(topics::used_time(1234), 1234);
}

#[test]
fn witness_topics_count_distinct_witnesses() {
    let flags = 0x2000 | 0x300;
    let mut w = 0;
    topics::activate(&mut w, flags, 1, 10);
    topics::activate(&mut w, flags, 1, 11);
    assert_eq!(w, COUNTING | 1);
    topics::activate(&mut w, flags, 2, 12);
    assert_eq!(topics::witnesses(w), 2);
    topics::activate(&mut w, flags, 4, 13);
    assert_eq!(w, ACTIVATED | 13);
}

#[test]
fn memory_loads_shifted_when_counts_differ() {
    let mut c = Character::new(1, "A");
    let at = MemoryKey {
        other: 2,
        role: 7,
        key: 19,
    };
    assert_eq!(topics::load(&c, at, 3), (None, false));
    topics::store(&mut c, at, &[7, 8, 9]);
    assert_eq!(topics::load(&c, at, 3), (Some(vec![7, 8, 9]), true));
    // The stored count (3) lands in node 0 and only as many words as the
    // block holds are copied: the last stored word is dropped.
    assert_eq!(topics::load(&c, at, 5), (Some(vec![3, 7, 8, 0, 0]), true));
    assert_eq!(topics::load(&c, at, 2), (Some(vec![3, 7]), true));
    let other = MemoryKey { key: 20, ..at };
    assert_eq!(topics::load(&c, other, 3), (None, true));
    assert_eq!(topics::world_time(100, 5500, 1000), 104);
}

#[test]
fn bubble_durations() {
    let s = Speaker::Character(GUARD);
    assert_eq!(bubbles::duration(s, 10, None), 1500);
    assert_eq!(bubbles::duration(Speaker::Narrator, 10, None), 2775);
    // (2 − 0.105) · 1050 = 1989.75 converts to nearest.
    assert_eq!(bubbles::duration(Speaker::Narrator, 1, None), 1990);
    assert_eq!(bubbles::duration(Speaker::Narrator, 200, None), 11_000);
    assert_eq!(bubbles::duration(s, 10, Some(GUARD)), 750);
}

#[test]
fn bubbles_queue_split_and_fade() {
    let mut b = Bubbles::default();
    let a = Speaker::Character(GUARD);
    let l = Speaker::Character(TRAVELLER);
    assert_eq!(b.say(a, Some(l), b"Hi", 0, None), 0);
    assert_eq!(b.heard.len(), 2);
    assert_eq!(b.heard[0].amount, 21.0);
    assert_eq!(b.say(a, None, b"Yo", 500, None), 1475);
    assert_eq!(b.list[0].phase, Phase::Closing);
    assert_eq!(b.list[1].phase, Phase::Waiting);
    // A CR splits a character's line; the rest waits behind it.
    let mut c = Bubbles::default();
    assert_eq!(c.say(a, None, b"One\r\nTwo", 0, None), 1525);
    assert_eq!(c.list[0].text, b"One");
    assert_eq!(c.list[1].text, b"Two");
    // The narrator's lines are not split.
    let mut n = Bubbles::default();
    n.say(Speaker::Narrator, None, b"A\r\nB", 0, None);
    assert_eq!(n.list.len(), 1);
    // Fade in, hold, fade out.
    let mut f = Bubbles::default();
    f.say(a, None, b"Hello", 0, None);
    f.update(150);
    assert!((f.list[0].alpha - 0.5).abs() < 1e-6);
    f.update(1000);
    assert_eq!(f.list[0].alpha, 1.0);
    f.update(1250 + 750);
    assert!((f.list[0].alpha - 0.5).abs() < 1e-6);
    f.update(1250 + 1501);
    assert!(f.list.is_empty());
}

#[test]
fn a_bubble_is_measured_placed_and_projected() {
    // The text is wrapped at 320 and the height is lines × line height − 2.
    let mut b = Bubbles::default();
    b.say(Speaker::Character(7), None, b"One two", 0, None);
    b.layout(20, |text, width| {
        assert_eq!(*width, bubbles::BUBBLE_WIDTH);
        *width = 123;
        if text.len() > 7 { 2 } else { 1 }
    });
    assert_eq!((b.list[0].width, b.list[0].lines), (123, 1));
    assert_eq!(b.list[0].height, 18);
    // The layout runs once: a later call leaves the measured bubble alone.
    b.layout(20, |_, width| {
        *width = 9;
        9
    });
    assert_eq!((b.list[0].width, b.list[0].height), (123, 18));

    // A narrator line: centred and three quarters down the screen, then
    // kept four units from the edges.
    let view = bubbles::View {
        width: 800,
        height: 600,
        glyph_height: 16,
        camera: None,
    };
    let mut n = b.list[0].clone();
    n.speaker = Speaker::Narrator;
    n.width = 200;
    n.height = 18;
    Bubbles::place_bubble(&mut n, &view, None);
    // (800 − 200) / 2 = 300; 16 + round(450) − 18 = 448.
    assert_eq!((n.x, n.y), (300, 448));
    // Too tall for the top margin: pulled to 4.
    n.height = 600;
    Bubbles::place_bubble(&mut n, &view, None);
    assert_eq!(n.y, 4);
    // Too wide: pushed inside the left margin.
    n.width = 900;
    n.height = 18;
    Bubbles::place_bubble(&mut n, &view, None);
    assert_eq!(n.x, 4);

    // A character line: the head is raised 120 cm, taken to camera space
    // by the rigid inverse view, then divided by the depth.
    let camera = bubbles::Camera {
        inverse_view: [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 5.0],
        projection: [2.0, 2.0],
    };
    let view = bubbles::View {
        camera: Some(camera),
        ..view
    };
    // Camera space is (x, y, z − 5); a point at (0, −120, 0) is 120 below
    // the eye, so y = 300 − (2·600·0.5·120)/5 = −14100 and the screen y is
    // 600 − (−14100) = 14700. The view does not flip y: the projection
    // already returns a bottom-left origin.
    let p = bubbles::project(&camera, &view, [0.0, -120.0, 0.0]);
    assert_eq!(p, [400.0, 14700.0]);
    // 60 cm right and 90 below: x = 400 + (2·800·0.5·60)/5 = 400 + 9600,
    // y = 300 − (2·600·0.5·90)/5 = 300 − 10800, then 600 + 10500.
    let p = bubbles::project(&camera, &view, [60.0, -90.0, 0.0]);
    assert_eq!(p, [400.0 + 9600.0, 11100.0]);
    // A point at or behind the camera's plane clamps to an edge: x >= 0
    // gives the right edge, y < 0 the top (which the flip returns as 0).
    let p = bubbles::project(&camera, &view, [0.0, -120.0, 10.0]);
    assert_eq!(p, [800.0, 600.0]);
    // An unordered component counts as positive; an unordered depth is
    // treated as behind, so a NaN y also lands at the bottom edge.
    let p = bubbles::project(&camera, &view, [f32::NAN, f32::NAN, f32::NAN]);
    assert_eq!(p, [0.0, 600.0]);

    // A character bubble at the screen centre: (300, 16 + 300 − 18).
    let mut c = b.list[0].clone();
    c.speaker = Speaker::Character(7);
    c.width = 200;
    c.height = 18;
    Bubbles::place_bubble(&mut c, &view, Some([0.0, -120.0, 0.0]));
    assert_eq!((c.x, c.y), (300, 298));
    // Without a head the position stays, but the margin clamp still runs.
    c.x = 1;
    c.y = 2;
    Bubbles::place_bubble(&mut c, &view, None);
    assert_eq!((c.x, c.y), (4, 4));
}

#[test]
fn moods_band_the_attitude() {
    let mut cast = Cast::default();
    let mut a = Character::new(GUARD, "A");
    let mut k = Contact::new(TRAVELLER);
    a.contacts.push(k);
    cast.characters.push(a);
    let band = |cast: &mut Cast, att: f32, range: f32, state: u32, concern: f32| {
        let c = &mut cast.characters[0].contacts[0];
        c.attitude = att;
        c.range = range;
        c.state = state;
        c.concern = concern;
        dialogue::mood(cast, GUARD, TRAVELLER)
    };
    assert_eq!(band(&mut cast, 80.0, 0.0, 0, 0.0), 0x10);
    assert_eq!(band(&mut cast, 50.0, 0.0, 0, 0.0), 0x20);
    assert_eq!(band(&mut cast, 0.0, 0.0, 0, 0.0), 0x40);
    assert_eq!(band(&mut cast, -40.0, 0.0, 0, 0.0), 0x80);
    assert_eq!(band(&mut cast, -90.0, 0.0, 0, 0.0), 0x80);
    // Scaled by ((10 − x)/10)²; nothing past 10.
    assert_eq!(band(&mut cast, 80.0, 5.0, 0, 0.0), 0x40);
    assert_eq!(band(&mut cast, 80.0, 11.0, 0, 0.0), 0x40);
    // An unordered contact value is scaled, not zeroed: the attitude turns
    // unordered too and falls through every band test to 0x40.
    assert_eq!(band(&mut cast, 80.0, f32::NAN, 0, 0.0), 0x40);
    assert_eq!(band(&mut cast, 80.0, 10.0, 0, 0.0), 0x40);
    assert_eq!(band(&mut cast, 0.0, 0.0, 1, 5.0), 0x640);
    assert_eq!(band(&mut cast, 0.0, 0.0, 1, 2.0), 0x240);
    assert_eq!(band(&mut cast, 0.0, 0.0, 2, 0.0), 0x640);
    k.attitude = 80.0;
    cast.characters[0].contacts[0] = k;
    assert_eq!(dialogue::mood_code(&cast, GUARD, TRAVELLER), 0x1000_0000);
    cast.characters[0].contacts[0].state = 1;
    assert_eq!(dialogue::mood_code(&cast, GUARD, TRAVELLER), 0x6000_0000);
}

/// A guard (role 100) greets a traveller (Human, 1): two alternative
/// lines, the traveller's answer, then the end. The first use is not
/// recorded without a memory entry; with one it is, and the ten-minute
/// cooldown holds the greeting back.
#[test]
fn an_ai_greets_and_the_topic_is_remembered() {
    let greeting = node(
        NODE_ENTRY | 0x9,
        "Greeting",
        &[],
        vec![branch(
            0,
            &[(1, "Halt."), (1, "Stop.")],
            "",
            "",
            vec![branch(0, &[(2, "I mean no harm.")], "", "", vec![])],
        )],
    );
    let db = RoleDb {
        roles: vec![
            role(
                100,
                0,
                0,
                vec![entry(0x2, 1, Some(graph(1, vec![greeting])))],
            ),
            role(1, 0, 0, vec![]),
        ],
    };
    let mut cast = Cast {
        characters: vec![person(GUARD, &[100], &db), person(TRAVELLER, &[1], &db)],
    };
    let mut e = Engine::default();
    let mut s = scene(&db, None, &mut cast, 0);
    assert!(e.greet(&mut s, GUARD, TRAVELLER));
    assert_eq!(e.bubbles.list[0].text, b"Halt.");
    assert_eq!(e.bubbles.list[0].speaker, Speaker::Character(GUARD));
    let id = e.active[0];
    assert_eq!(e.conversations[id].as_ref().unwrap().next_at, 625);
    // The guard is busy: no second greeting.
    assert!(!e.greet(&mut s, GUARD, TRAVELLER));
    let mut s = scene(&db, None, &mut cast, 626);
    e.update(&mut s);
    assert_eq!(e.bubbles.list[1].text, b"I mean no harm.");
    assert_eq!(e.conversations[id].as_ref().unwrap().next_at, 626 + 875);
    let mut s = scene(&db, None, &mut cast, 1502);
    e.update(&mut s);
    assert!(e.active.is_empty());
    assert!(cast.get(GUARD).unwrap().relation(TRAVELLER).is_none());

    // With the entry that holds the block, the use is recorded.
    cast.get_mut(GUARD).unwrap().add_relation(TRAVELLER, 1, 0);
    let mut s = scene(&db, None, &mut cast, 2000);
    assert!(e.greet(&mut s, GUARD, TRAVELLER));
    for now in [3000, 5000] {
        let mut s = scene(&db, None, &mut cast, now);
        e.update(&mut s);
    }
    // Recorded when the reply group ran out (world time 1003) and again
    // when the conversation ended (1005).
    let words =
        topics::block(cast.get(GUARD).unwrap(), TRAVELLER, 1, 100).and_then(|m| m.words.clone());
    assert_eq!(words, Some(vec![1005]));
    let mut s = scene(&db, None, &mut cast, 6000);
    assert!(!e.greet(&mut s, GUARD, TRAVELLER), "inside the cooldown");
    let mut s = scene(&db, None, &mut cast, 604_000);
    assert!(!e.greet(&mut s, GUARD, TRAVELLER), "599 s later");
    let mut s = scene(&db, None, &mut cast, 605_000);
    assert!(e.greet(&mut s, GUARD, TRAVELLER), "after ten minutes");
}

/// The player asks a topic of the partner's dialogue: the panel lists the
/// player's line, the pick shows it and the partner answers.
#[test]
fn the_player_picks_a_topic() {
    let ask = node(
        0xF,
        "Ask",
        &[],
        vec![branch(
            0,
            &[(2, "Who are you?")],
            "",
            "",
            vec![branch(0, &[(1, "I am {Actor[0].Name}.")], "", "", vec![])],
        )],
    );
    let db = RoleDb {
        roles: vec![
            role(200, 0, 0, vec![entry(0x2, 1, Some(graph(1, vec![ask])))]),
            role(1, 0, 0, vec![]),
        ],
    };
    let mut cast = Cast {
        characters: vec![person(GUARD, &[200], &db), person(PLAYER, &[1], &db)],
    };
    cast.get_mut(GUARD).unwrap().name = "Derrin".to_string();
    let mut e = Engine::default();
    let mut s = scene(&db, None, &mut cast, 0);
    e.talk(&mut s, GUARD);
    assert!(e.panel.open);
    assert_eq!(e.panel.entries.len(), 1);
    assert_eq!(e.panel.entries[0].text, b"Who are you?");
    assert_eq!(e.panel.entries[0].topic, 1);
    e.pick(&mut s, 0);
    assert!(!e.panel.open, "the reply is final");
    assert_eq!(e.bubbles.list[0].speaker, Speaker::Character(PLAYER));
    let id = e.active[0];
    assert_eq!(e.conversations[id].as_ref().unwrap().next_at, 800);
    let mut s = scene(&db, None, &mut cast, 801);
    e.update(&mut s);
    assert_eq!(e.bubbles.list[1].text, b"I am Derrin.");
}

/// The narrator's Corlian node: the text, three replies offered, a pick
/// raises its game event.
#[test]
fn the_narrator_offers_replies() {
    let reply = |t: &str, ev: &str| {
        branch(
            0,
            &[(2, t)],
            "",
            &format!("GameEvent({ev});"),
            vec![branch(0, &[(1, "You feel it.")], "", "", vec![])],
        )
    };
    let corlian = node(
        NODE_ENTRY,
        "Corlian",
        &[0],
        vec![branch(
            0,
            &[(1, "Here lies the body of Corlian.")],
            "",
            "",
            vec![
                reply("I am {Actor[1].Name}.", "C10EC1"),
                reply("I am Corlian.", "C10EC2"),
                reply("I am both.", "C10EC3"),
            ],
        )],
    );
    let narrator = graph(0, vec![corlian]);
    let db = RoleDb { roles: vec![] };
    let mut cast = Cast {
        characters: vec![Character::new(PLAYER, "Ermus")],
    };
    let mut e = Engine::default();
    let mut s = scene(&db, Some(&narrator), &mut cast, 0);
    e.narrate(&mut s, name("Corlian"), 0);
    assert_eq!(e.bubbles.list[0].speaker, Speaker::Narrator);
    assert_eq!(e.bubbles.list[0].duration, 60_000, "held for the reply");
    assert!(e.panel.open);
    let texts: Vec<&[u8]> = e.panel.entries.iter().map(|c| c.text.as_slice()).collect();
    assert_eq!(
        texts,
        [&b"I am Ermus."[..], b"I am Corlian.", b"I am both."]
    );
    e.pick(&mut s, 1);
    assert_eq!(s.effects, [Effect::GameEvent(0xC1_0EC2)]);
    assert_eq!(e.bubbles.list[0].phase, Phase::Closing);
    assert!(!e.panel.open);
    let mut s = scene(&db, Some(&narrator), &mut cast, 61_000);
    e.update(&mut s);
    assert_eq!(e.bubbles.list.last().unwrap().text, b"You feel it.");
}

/// An AI event runs the matching node's action and clears its bit.
#[test]
fn events_run_their_nodes() {
    let engage = node(
        NODE_ENTRY | NODE_NOT_ENTRY | 0x40,
        "Engage",
        &[],
        vec![branch(0, &[(1, "")], "", "Actor[0].AddRole($1F);", vec![])],
    );
    let db = RoleDb {
        roles: vec![
            role(100, 0, 0, vec![entry(0x2, 1, Some(graph(1, vec![engage])))]),
            role(1, 0, 0, vec![]),
            role(0x1F, 0, 0, vec![]),
        ],
    };
    let mut cast = Cast {
        characters: vec![person(GUARD, &[100], &db), person(TRAVELLER, &[1], &db)],
    };
    let mut e = Engine::default();
    let mut s = scene(&db, None, &mut cast, 0);
    assert_eq!(e.ai_events(&mut s, GUARD, TRAVELLER, 2, None), 0);
    assert!(cast.get(GUARD).unwrap().holds(0x1F));
    // Bits with no node stay set.
    let mut s = scene(&db, None, &mut cast, 10);
    assert_eq!(e.ai_events(&mut s, GUARD, TRAVELLER, 8, None), 8);
}

/// `Actor.ActivateTopic(Actor[1], Parting, $28)`: the relation role's
/// dialogue aimed at 0 has its node activated in the owner's memory.
#[test]
fn a_relation_topic_is_activated() {
    let parting = node(0x0100_0000 | 0x100 | 0x3_0000, "Parting", &[0], vec![]);
    let db = RoleDb {
        roles: vec![role(
            40,
            1,
            28,
            vec![entry(0x2, 0, Some(graph(0, vec![None, parting])))],
        )],
    };
    let mut cast = Cast {
        characters: vec![Character::new(GUARD, "A"), Character::new(PLAYER, "B")],
    };
    let mut s = scene(&db, None, &mut cast, 0);
    dialogue::activate_with(&mut s, GUARD, PLAYER, name("parting"), 40, 0, 1);
    let words =
        topics::block(cast.get(GUARD).unwrap(), PLAYER, 40, 0).and_then(|m| m.words.clone());
    assert_eq!(words, Some(vec![0, ACTIVATED | 1000]));
}

#[test]
fn the_highlight_moves_within_the_list() {
    let mut e = Engine::default();
    e.panel.open = true;
    for _ in 0..3 {
        e.panel.entries.push(dialogue::Choice {
            text: Vec::new(),
            topic: 0,
            branch: 0,
        });
    }
    e.move_highlight(false);
    assert_eq!(e.panel.highlight, 0);
    for want in [1, 2, 3, 3] {
        e.move_highlight(true);
        assert_eq!(e.panel.highlight, want);
    }
    e.move_highlight(false);
    assert_eq!(e.panel.highlight, 2);
}

#[test]
fn ending_everything_records_and_releases() {
    let greeting = node(
        NODE_ENTRY | 0x9,
        "Greeting",
        &[],
        vec![branch(
            0,
            &[(1, "Halt.")],
            "",
            "",
            vec![branch(0, &[(2, "No.")], "", "", vec![])],
        )],
    );
    let db = RoleDb {
        roles: vec![
            role(
                100,
                0,
                0,
                vec![entry(0x2, 1, Some(graph(1, vec![greeting])))],
            ),
            role(1, 0, 0, vec![]),
        ],
    };
    let mut guard = person(GUARD, &[100], &db);
    guard.add_relation(TRAVELLER, 1, 0);
    let mut cast = Cast {
        characters: vec![guard, person(TRAVELLER, &[1], &db)],
    };
    let mut e = Engine::default();
    let mut s = scene(&db, None, &mut cast, 0);
    assert!(e.greet(&mut s, GUARD, TRAVELLER));
    e.end_all(&mut s);
    assert!(e.active.is_empty());
    assert!(e.conversations.iter().all(Option::is_none));
    let words =
        topics::block(cast.get(GUARD).unwrap(), TRAVELLER, 1, 100).and_then(|m| m.words.clone());
    assert_eq!(words, Some(vec![1000]));
}

#[test]
fn ended_conversations_are_released() {
    let mut e = Engine::default();
    let db = RoleDb::default();
    let mut cast = Cast::default();
    let mut s = scene(&db, None, &mut cast, 0);
    e.update(&mut s);
    assert!(e.active.is_empty());
    assert_eq!(State::Ended, State::Ended);
}

/// `GameEvent`'s memory flash: a contact holding XA_Derrin (19) has
/// "XAU_Memory" of its dialogue toward the Player role (7, which the
/// event's actor holds) activated, filed under entry 7 and key 19.
#[test]
fn a_witnessed_event_activates_the_contacts_topic() {
    let memory = node(0x100, "XAU_Memory", &[], vec![]);
    let db = RoleDb {
        roles: vec![
            role(19, 0, 0, vec![entry(0x2, 7, Some(graph(7, vec![memory])))]),
            role(7, 0, 0, vec![]),
        ],
    };
    let mut player = person(PLAYER, &[7], &db);
    player.contacts.push(Contact::new(GUARD));
    let mut cast = Cast {
        characters: vec![person(GUARD, &[19], &db), player],
    };
    let mut s = scene(&db, None, &mut cast, 0);
    dialogue::witness_event(&mut s, PLAYER, 19, 7, 1, name("XAU_Memory"));
    let words =
        topics::block(cast.get(GUARD).unwrap(), PLAYER, 7, 19).and_then(|m| m.words.clone());
    assert_eq!(words, Some(vec![ACTIVATED | 1000]));
}

/// A broadcast reaches a witness through the relation role it holds
/// toward the actor (Explorer's dialogue toward Leader).
#[test]
fn a_broadcast_reaches_a_witness_through_its_relation_role() {
    let hurt = node(0x100 | 0xB_0000, "DamageTaken", &[], vec![]);
    let db = RoleDb {
        roles: vec![
            role(31, 0, 0, vec![entry(0x2, 30, Some(graph(30, vec![hurt])))]),
            role(30, 1, 28, vec![]),
        ],
    };
    let mut derrin = person(GUARD, &[31], &db);
    derrin.add_relation(PLAYER, 30, 0);
    let mut player = Character::new(PLAYER, "P");
    player.contacts.push(Contact::new(GUARD));
    let mut cast = Cast {
        characters: vec![derrin, player],
    };
    let mut s = scene(&db, None, &mut cast, 0);
    dialogue::broadcast(&mut s, PLAYER, name("DamageTaken"), |_, _| false);
    assert!(topics::block(cast.get(GUARD).unwrap(), PLAYER, 30, 31).is_none());
    let mut s = scene(&db, None, &mut cast, 0);
    dialogue::broadcast(&mut s, PLAYER, name("DamageTaken"), |_, _| true);
    let words =
        topics::block(cast.get(GUARD).unwrap(), PLAYER, 30, 31).and_then(|m| m.words.clone());
    assert_eq!(words, Some(vec![ACTIVATED | 1000]));
}

/// The mood mask and mood code reproduce the original's ordered compares:
/// an unordered (NaN) value fails every test it reaches.
#[test]
fn the_mood_mask_and_code_test_exactly() {
    use crate::dialogue::{mood, mood_code};
    let mut cast = Cast::default();
    let mut c = Character::new(1, "A");
    c.contacts.push(Contact {
        other: 2,
        concern: 0.0,
        attitude: 0.0,
        state: 0,
        range: 0.0,
    });
    cast.characters.push(c);
    let set = |cast: &mut Cast, f: &dyn Fn(&mut Contact)| {
        let c = cast.get_mut(1).unwrap();
        c.contacts[0] = Contact {
            other: 2,
            concern: 0.0,
            attitude: 0.0,
            state: 0,
            range: 0.0,
        };
        f(&mut c.contacts[0]);
    };

    set(&mut cast, &|c| c.attitude = 80.0);
    assert_eq!(mood(&cast, 1, 2), 0x10);
    assert_eq!(mood_code(&cast, 1, 2), 0x1000_0000);
    set(&mut cast, &|c| c.attitude = 50.0);
    assert_eq!(mood(&cast, 1, 2), 0x20);
    assert_eq!(mood_code(&cast, 1, 2), 0x2000_0000);
    set(&mut cast, &|c| c.attitude = 0.0);
    assert_eq!(mood(&cast, 1, 2), 0x40);
    assert_eq!(mood_code(&cast, 1, 2), 0x3000_0000);
    // Below −30 is the hostile band. The below −60 test after it can never
    // pass: 0x100 and 0x50000000 are unreachable, as in the original.
    set(&mut cast, &|c| c.attitude = -40.0);
    assert_eq!(mood(&cast, 1, 2), 0x80);
    assert_eq!(mood_code(&cast, 1, 2), 0x4000_0000);
    set(&mut cast, &|c| c.attitude = -70.0);
    assert_eq!(mood(&cast, 1, 2), 0x80);
    assert_eq!(mood_code(&cast, 1, 2), 0x4000_0000);
    // An unordered attitude fails every compare: the neutral band.
    set(&mut cast, &|c| c.attitude = f32::NAN);
    assert_eq!(mood(&cast, 1, 2), 0x40);
    assert_eq!(mood_code(&cast, 1, 2), 0x3000_0000);
    // The contact value scales the attitude by (10 − x)²/100; 5 makes an
    // 80 attitude 20, still the neutral band.
    set(&mut cast, &|c| {
        c.attitude = 80.0;
        c.range = 5.0;
    });
    assert_eq!(mood(&cast, 1, 2), 0x40);
    // Past 10 the attitude counts for nothing; an unordered value scales
    // it to NaN, which also lands in the neutral band.
    set(&mut cast, &|c| {
        c.attitude = 80.0;
        c.range = 11.0;
    });
    assert_eq!(mood(&cast, 1, 2), 0x40);
    set(&mut cast, &|c| {
        c.attitude = 80.0;
        c.range = f32::NAN;
    });
    assert_eq!(mood(&cast, 1, 2), 0x40);
    // Concern over 4 makes a hostile code, over 1.2 a wary one; an
    // unordered concern makes none.
    set(&mut cast, &|c| {
        c.state = 1;
        c.concern = 5.0;
    });
    assert_eq!(mood(&cast, 1, 2) & 0x600, 0x600);
    assert_eq!(mood_code(&cast, 1, 2), 0x6000_0000);
    set(&mut cast, &|c| {
        c.state = 1;
        c.concern = f32::NAN;
    });
    assert_eq!(mood(&cast, 1, 2) & 0x600, 0);
    set(&mut cast, &|c| {
        c.state = 1;
        c.concern = 2.0;
    });
    assert_eq!(mood(&cast, 1, 2) & 0x600, 0x200);
    // Hostile bit.
    set(&mut cast, &|c| c.state = 2);
    assert_eq!(mood(&cast, 1, 2) & 0x600, 0x600);
    assert_eq!(mood_code(&cast, 1, 2), 0x7000_0000);
}

/// Both player slots speak reply choices; only the first is shortened.
#[test]
fn the_second_player_slot_is_a_player_too() {
    use crate::dialogue::is_player;
    let db = RoleDb::default();
    let mut cast = Cast::default();
    let mut sc = scene(&db, None, &mut cast, 0);
    sc.player2 = Some(TRAVELLER);
    assert!(is_player(&sc, Speaker::Character(PLAYER)));
    assert!(is_player(&sc, Speaker::Character(TRAVELLER)));
    assert!(!is_player(&sc, Speaker::Character(GUARD)));
    assert!(!is_player(&sc, Speaker::Narrator));
    assert_eq!(
        bubbles::duration(Speaker::Character(TRAVELLER), 10, sc.player),
        1500,
        "the second player's line is not halved"
    );
}

/// `ActivateTopicN` with a target role checks the contact holds the target
/// (for a relation role, that the actor holds it toward the contact), not
/// the actor; the word lands under entry target, key role.
#[test]
fn near_activation_checks_the_contact_and_files_the_word() {
    let memory = node(0x100, "Rumour", &[], vec![]);
    let db = RoleDb {
        roles: vec![
            role(5, 0, 0, vec![entry(0x2, 9, Some(graph(9, vec![memory])))]),
            role(9, 0, 0, vec![]),
        ],
    };
    let mut player = person(PLAYER, &[5], &db);
    player.contacts.push(Contact::new(GUARD));
    let mut cast = Cast {
        characters: vec![person(GUARD, &[], &db), player],
    };
    {
        let mut s = scene(&db, None, &mut cast, 0);
        dialogue::activate_near(&mut s, PLAYER, 5, 9, 1, name("Rumour"));
    }
    assert!(topics::block(cast.get(PLAYER).unwrap(), GUARD, 9, 5).is_none());
    // The actor holding the target does not count.
    cast.get_mut(PLAYER).unwrap().add_role(&db, 9);
    {
        let mut s = scene(&db, None, &mut cast, 0);
        dialogue::activate_near(&mut s, PLAYER, 5, 9, 1, name("Rumour"));
    }
    assert!(topics::block(cast.get(PLAYER).unwrap(), GUARD, 9, 5).is_none());
    cast.get_mut(GUARD).unwrap().add_role(&db, 9);
    {
        let mut s = scene(&db, None, &mut cast, 0);
        dialogue::activate_near(&mut s, PLAYER, 5, 9, 1, name("Rumour"));
    }
    let words = topics::block(cast.get(PLAYER).unwrap(), GUARD, 9, 5).and_then(|m| m.words.clone());
    assert_eq!(words, Some(vec![ACTIVATED | 1000]));
    // Without a target the contact's roles pick the dialogue and the word
    // is filed under entry 0, key role.
    let gossip = node(0x100, "Gossip", &[], vec![]);
    let db = RoleDb {
        roles: vec![
            role(5, 0, 0, vec![entry(0x2, 9, Some(graph(9, vec![gossip])))]),
            role(9, 0, 0, vec![]),
        ],
    };
    let mut player = person(PLAYER, &[5], &db);
    player.contacts.push(Contact::new(GUARD));
    let mut cast = Cast {
        characters: vec![person(GUARD, &[9], &db), player],
    };
    let mut s = scene(&db, None, &mut cast, 0);
    dialogue::activate_near(&mut s, PLAYER, 5, 0, 1, name("Gossip"));
    let words = topics::block(cast.get(PLAYER).unwrap(), GUARD, 0, 5).and_then(|m| m.words.clone());
    assert_eq!(words, Some(vec![ACTIVATED | 1000]));
}

/// A topic event that stores a word checks the actor's roles against the
/// database: a role it lacks is reported.
#[test]
fn a_stored_event_reports_an_unknown_role() {
    let memory = node(0x100, "Rumour", &[], vec![]);
    let db = RoleDb {
        roles: vec![
            role(5, 0, 0, vec![entry(0x2, 9, Some(graph(9, vec![memory])))]),
            role(9, 0, 0, vec![]),
        ],
    };
    let mut player = person(PLAYER, &[5], &db);
    // A role the database does not have (added as the original does, with
    // no record behind it).
    player.add_role(&db, 77);
    player.contacts.push(Contact::new(GUARD));
    let mut cast = Cast {
        characters: vec![person(GUARD, &[9], &db), player],
    };
    let mut s = scene(&db, None, &mut cast, 0);
    dialogue::activate_near(&mut s, PLAYER, 5, 9, 1, name("Rumour"));
    assert_eq!(
        s.effects,
        vec![Effect::Report(
            "Near Activation".to_string(),
            "Invalid role identifier.".to_string()
        )]
    );
}

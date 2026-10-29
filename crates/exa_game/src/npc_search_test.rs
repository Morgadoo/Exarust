//! Tests of the follow and the search in the running game.
//! (Included from npc_search.rs.)

use super::*;
use exa_ai::search::Search;

fn possessions() -> Possessions {
    Possessions {
        flags: 0,
        out: [Hand { id: 7, flags: 0x10 }, Hand::default()],
        stowed: [Hand { id: 9, flags: 0 }, Hand::default()],
        worn: vec![ItemRef { id: 3, flags: 1 }, ItemRef { id: 4, flags: 2 }],
        carried: vec![
            Carried {
                id: 7,
                flags: 0x100,
                x: 5,
                y: 6,
            },
            Carried {
                id: 11,
                flags: 0,
                x: 30,
                y: 40,
            },
        ],
    }
}

/// The search reads the carried list with its flags (the held mark is the
/// entry it skips), the worn items and the hands, main, second, then the
/// stowed set.
#[test]
fn a_pack_reads_the_possessions() {
    let pack = pack_of(&possessions());
    assert_eq!(
        pack.carried,
        vec![
            search::Carried {
                item: 7,
                flags: search::SKIP_FLAG
            },
            search::Carried { item: 11, flags: 0 },
        ]
    );
    assert_eq!(pack.worn, vec![3, 4]);
    assert_eq!(pack.hands, [7, 0, 9, 0]);
    assert!(!pack.locked);
}

/// Writing back keeps what stayed (flags and places), drops what left and
/// adds what came with no flags at the origin; a hand that changed takes
/// the new item with no flags, one that did not keeps its flags.
#[test]
fn a_changed_pack_writes_back() {
    let mut p = possessions();
    let mut pack = pack_of(&p);
    pack.worn.retain(|&w| w != 3);
    pack.worn.push(20);
    pack.carried.remove(1);
    pack.carried.push(search::Carried { item: 21, flags: 0 });
    pack.hands[2] = 22;
    sync_possessions(&mut p, &pack);
    assert_eq!(
        p.worn,
        vec![ItemRef { id: 4, flags: 2 }, ItemRef { id: 20, flags: 0 }]
    );
    assert_eq!(
        p.carried,
        vec![
            Carried {
                id: 7,
                flags: 0x100,
                x: 5,
                y: 6
            },
            Carried {
                id: 21,
                flags: 0,
                x: 0,
                y: 0
            },
        ]
    );
    assert_eq!(p.out[0], Hand { id: 7, flags: 0x10 });
    assert_eq!(p.stowed[0], Hand { id: 22, flags: 0 });
    // An unchanged pack writes back what was there.
    let before = possessions();
    let mut same = before.clone();
    sync_possessions(&mut same, &pack_of(&before));
    assert_eq!(same, before);
}

/// The room grid lists each character present in its 600 cm cell (a 4 × 4
/// grid centred on the origin: cell (2, 2) holds the origin) and keeps the
/// connected flags; one no longer in the world is not listed.
#[test]
fn the_room_grid_lists_the_characters() {
    let size = exa_ai::rooms::RoomGridSize {
        width: 4,
        height: 4,
    };
    let flags = connected_flags(size, |centre| centre.x > 0.0);
    assert_eq!(flags.len(), 16);
    assert_eq!(flags[2 * 4 + 2], search::CONNECTED);
    assert_eq!(flags[2 * 4 + 1], 0);
    let grid = room_grid(
        size,
        &flags,
        &[
            Some(Vec3::new(10.0, 0.0, 10.0)),
            Some(Vec3::new(-10.0, 0.0, 610.0)),
            None,
            Some(Vec3::new(5000.0, 0.0, 0.0)),
        ],
    );
    assert_eq!(grid.origin, (2, 2));
    assert_eq!(grid.characters[2 * 4 + 2], vec![0]);
    assert_eq!(grid.characters[3 * 4 + 1], vec![1]);
    assert_eq!(grid.characters.iter().flatten().count(), 2);
    assert_eq!(grid.flags, flags);
}

/// The search over a refreshed scene: a follower beside the player and a
/// dead body 3 m off in a connected room gathers the body, picks it, looks
/// it over, walks to it (a straight route without a grid), kneels and
/// takes its better weapon; the body's possessions lose it and the
/// follower carries it, then holds it in its hand.
#[test]
fn a_follower_searches_a_dead_body() {
    let mut app = App::new();
    let mut items = SearchItems {
        tried: true,
        ..SearchItems::default()
    };
    let sword = |tier: u8| {
        Loot::Weapon(search::Arm {
            flags: 0x21,
            quality: 255,
            damage: 0,
            thrust: 140,
            tier,
            special: 0,
        })
    };
    items.loot.insert(100, sword(4));
    items.loot.insert(101, sword(1));
    app.insert_resource(items)
        .init_resource::<SearchWorld>()
        .insert_resource(BodyTicks(0))
        .add_systems(Update, follow_and_search);
    // Away from the world origin, where a new search last gathered.
    let player = app
        .world_mut()
        .spawn((
            Player,
            GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 20.0)),
        ))
        .id();
    let root = app
        .world_mut()
        .spawn((
            CharacterBody::default(),
            GlobalTransform::from_translation(Vec3::new(21.5, 0.0, 20.0)),
        ))
        .id();
    let mut npc = Npc::new([0.0; 4]);
    npc.applied = Some(exa_ai::activity::Applied {
        activity: 4,
        idle_activity: 6,
        state: IdleState::Follow,
        leaves_party: false,
        follow_band_cm: Some(400.0),
        phase_word: 1,
        create_idle: None,
        drop_idle: false,
        skipped: Vec::new(),
    });
    npc.search = Some(Search::new());
    let follower = app
        .world_mut()
        .spawn((
            npc,
            exa_ai::Duellist::new(player),
            Condition::default(),
            Transform::default(),
            GlobalTransform::from_translation(Vec3::new(21.5, 0.0, 20.0)),
            ChildOf(root),
            CharacterId(5),
            Possessions {
                carried: vec![Carried {
                    id: 101,
                    flags: 0,
                    x: 0,
                    y: 0,
                }],
                ..Possessions::default()
            },
        ))
        .id();
    let body = app
        .world_mut()
        .spawn((
            Condition::DEAD,
            GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 23.0)),
            CharacterId(6),
            Possessions {
                worn: vec![ItemRef { id: 100, flags: 0 }],
                ..Possessions::default()
            },
        ))
        .id();
    let mut took = false;
    for tick in 1..4000u64 {
        app.world_mut().resource_mut::<BodyTicks>().0 = tick;
        // The walk steers the body; the test moves it there at once.
        let velocity = app
            .world()
            .get::<CharacterBody>(root)
            .unwrap()
            .desired_velocity;
        let mut gt = app.world_mut().get_mut::<GlobalTransform>(root).unwrap();
        *gt = GlobalTransform::from_translation(gt.translation() + velocity * 0.012);
        app.update();
        if app
            .world()
            .get::<Possessions>(body)
            .unwrap()
            .worn
            .is_empty()
        {
            took = true;
            break;
        }
    }
    assert!(took, "the follower takes the body's weapon");
    let mine = app.world().get::<Possessions>(follower).unwrap();
    assert!(
        mine.carried.iter().any(|c| c.id == 100) || mine.out[0].id == 100,
        "{mine:?}"
    );
}

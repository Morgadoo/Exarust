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
/// adds what came at the record's default cell (the record's append writes
/// `0x28, 4`) and worn ones with no flags; a hand that changed takes
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
                x: 0x28,
                y: 4
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
        &[
            (4, Vec3::new(700.0, 0.0, 10.0)),
            (5, Vec3::new(9000.0, 0.0, 0.0)),
        ],
    );
    assert_eq!(grid.origin, (2, 2));
    assert_eq!(grid.characters[2 * 4 + 2], vec![0]);
    assert_eq!(grid.characters[3 * 4 + 1], vec![1]);
    assert_eq!(grid.characters.iter().flatten().count(), 2);
    assert_eq!(grid.objects[2 * 4 + 3], vec![4]);
    assert_eq!(grid.objects.iter().flatten().count(), 1);
    assert_eq!(grid.flags, flags);
}

/// The sector's lists place each object in the cells given, in the order
/// given, wherever its box is; an index off the grid is left out and an
/// object may sit in several cells (a wide static mesh).
#[test]
fn the_sector_lists_place_the_objects() {
    let size = exa_ai::rooms::RoomGridSize {
        width: 4,
        height: 4,
    };
    let flags = connected_flags(size, |_| true);
    let grid = room_grid_in_cells(
        size,
        &flags,
        &[Some(Vec3::new(10.0, 0.0, 10.0))],
        &[(7, 10), (3, 10), (7, 11), (9, 16), (9, -1)],
    );
    assert_eq!(grid.characters[10], vec![0]);
    assert_eq!(grid.objects[10], vec![7, 3]);
    assert_eq!(grid.objects[11], vec![7]);
    assert_eq!(grid.objects.iter().flatten().count(), 3);
}

/// A worn list's changes: each id of the old list is matched once in the
/// new one; what is left over on either side came off or went on.
#[test]
fn worn_changes_match_each_id_once() {
    assert_eq!(worn_changes(&[3, 4, 3], &[3, 5]), (vec![4, 3], vec![5]));
    assert_eq!(worn_changes(&[], &[]), (vec![], vec![]));
}

/// A container written back keeps its remaining entries' flags and places
/// and adds new ones with no flags at the panel's far corner (its width and
/// height times 40, as the original's container add writes them).
#[test]
fn a_changed_container_writes_back() {
    use exa_formats::containers::{Child, Container as Section};
    let child = |id, flags, x| Child { id, flags, x, y: 2 };
    let mut c = Section {
        width: 4,
        height: 4,
        capacity: 10,
        max_item: 5,
        children: vec![child(7, 1, 10), child(0, 0x100, 20), child(9, 3, 30)],
    };
    let model = container_of(&c);
    assert_eq!(model.items, vec![7, 0, 9]);
    assert_eq!(model.max_item, 5);
    let after = search::Container {
        items: vec![0, 9, 12],
        max_item: 5,
    };
    sync_container(&mut c, &after);
    assert_eq!(
        c.children,
        vec![
            child(0, 0x100, 20),
            child(9, 3, 30),
            Child {
                id: 12,
                flags: 0,
                x: 160,
                y: 160
            }
        ]
    );
}

/// An item catalogue of records, each id with its sections (tag, body).
fn catalogue_of(records: &[(u32, Vec<(u32, Vec<u8>)>)]) -> ItemCatalogue {
    let payloads: Vec<Vec<u8>> = records
        .iter()
        .map(|(_, sections)| {
            let mut payload = Vec::new();
            for (tag, body) in sections {
                payload.extend(tag.to_le_bytes());
                payload.extend(((8 + body.len()) as u32).to_le_bytes());
                payload.extend(body);
            }
            payload
        })
        .collect();
    let mut file = exa_formats::objdb::MAGIC.to_le_bytes().to_vec();
    file.extend((16 * (records.len() as u32 + 1)).to_le_bytes());
    let mut offset = 0u32;
    for ((id, _), payload) in records.iter().zip(&payloads) {
        for word in [*id, 0, offset, payload.len() as u32] {
            file.extend(word.to_le_bytes());
        }
        offset += payload.len() as u32;
    }
    file.extend([0u8; 16]);
    for payload in &payloads {
        file.extend(payload);
    }
    let db = exa_formats::rdb::Database::parse(&file).unwrap();
    ItemCatalogue::of_database(&db)
}

/// A weapon section's body with its flags word (the rest zero).
fn weapon_body(flags: u32) -> (u32, Vec<u8>) {
    let mut body = vec![0u8; 0x60];
    body[..4].copy_from_slice(&flags.to_le_bytes());
    (0x4000, body)
}

/// An item catalogue of weapons: each id with its weapon section's flags.
fn weapon_catalogue(weapons: &[(u32, u32)]) -> ItemCatalogue {
    let records: Vec<(u32, Vec<(u32, Vec<u8>)>)> = weapons
        .iter()
        .map(|&(id, flags)| (id, vec![weapon_body(flags)]))
        .collect();
    catalogue_of(&records)
}

/// A follower near the player searches the level's objects: it loots a
/// placed chest's better sword (written back into the chest's section) and
/// takes a loose blade lying in the world, which leaves the world. The
/// re-arm's hands go through the inventory: the take into the hand holds
/// the one-handed blade and the planned arms stow the two-handed sword.
#[test]
fn a_follower_searches_a_chest_and_a_loose_item() {
    let mut app = App::new();
    let mut items = SearchItems {
        tried: true,
        ..SearchItems::default()
    };
    let sword = |flags: u32, tier: u8| {
        Loot::Weapon(search::Arm {
            flags,
            quality: 255,
            damage: 0,
            thrust: 140,
            tier,
            special: 0,
        })
    };
    items.loot.insert(100, sword(0x21, 4));
    items.loot.insert(101, sword(0x21, 1));
    // The loose one is of another class, so neither take outweighs the
    // other (the held one stays listed and counts).
    items.loot.insert(102, sword(0x11, 6));
    app.insert_resource(weapon_catalogue(&[(100, 0x21), (101, 0x21), (102, 0x11)]));
    app.insert_resource(items)
        .init_resource::<SearchWorld>()
        .init_resource::<OpenSearchScreens>()
        .init_resource::<exa_physics::sim_app::FrameCommands>()
        .insert_resource(BodyTicks(0))
        .add_systems(
            Update,
            (
                follow_and_search,
                carry_out_search,
                exa_ai::route_walk::walk_routes,
            )
                .chain(),
        );
    app.world_mut().spawn((
        Player,
        GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 20.0)),
    ));
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
    let player = app.world_mut().spawn_empty().id();
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
    let chest_id = exa_formats::objdb::LOCAL | 0x32;
    let chest = app
        .world_mut()
        .spawn((
            exa_world::PlacedItem {
                id: 0x32,
                tagged_id: chest_id,
                condition: 0,
            },
            exa_world::ObjectSections {
                placement: 0,
                usable: None,
                use_script: None,
                container: Some(exa_formats::containers::Container {
                    width: 4,
                    height: 4,
                    capacity: 20,
                    max_item: 5,
                    children: vec![exa_formats::containers::Child {
                        id: 100,
                        flags: 0,
                        x: 0,
                        y: 0,
                    }],
                }),
            },
            GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 23.0)),
        ))
        .id();
    let loose = app
        .world_mut()
        .spawn((
            exa_world::PlacedItem {
                id: 102,
                tagged_id: 102,
                condition: 0,
            },
            GlobalTransform::from_translation(Vec3::new(23.0, 0.0, 21.0)),
        ))
        .id();
    let chest_items = |app: &App| {
        app.world()
            .get::<exa_world::ObjectSections>(chest)
            .and_then(|s| s.container.as_ref())
            .map(|c| c.children.iter().map(|c| c.id).collect::<Vec<_>>())
            .unwrap_or_default()
    };
    let mut done = false;
    for tick in 1..8000u64 {
        app.world_mut().resource_mut::<BodyTicks>().0 = tick;
        let velocity = app
            .world()
            .get::<CharacterBody>(root)
            .unwrap()
            .desired_velocity;
        let mut gt = app.world_mut().get_mut::<GlobalTransform>(root).unwrap();
        *gt = GlobalTransform::from_translation(gt.translation() + velocity * 0.012);
        app.update();
        if !chest_items(&app).contains(&100) && app.world().get_entity(loose).is_err() {
            done = true;
            break;
        }
    }
    assert!(done, "chest {:?}", chest_items(&app));
    let mine = app.world().get::<Possessions>(follower).unwrap();
    let has = |id| mine.carried.iter().any(|c| c.id == id) || mine.out[0].id == id;
    assert!(has(100) && has(102), "{mine:?}");
    // The best weapon is taken in hand, and it stays listed, marked held.
    assert_eq!(mine.out[0].id, 102, "{mine:?}");
    assert_eq!(mine.stowed[0].id, 100, "{mine:?}");
    assert!(
        mine.carried
            .iter()
            .any(|c| c.id == 102 && c.flags & exa_items::possession::HELD != 0),
        "{mine:?}"
    );
}

/// A dead body that lost a worn item is dressed again: the garment leaves
/// its outfit, and the underwear rule puts the braies back on its worn
/// list first.
#[test]
fn a_robbed_body_is_dressed_again() {
    let mut app = App::new();
    let mut tunic = exa_formats::objdb::Item::parse(0x10, &[]).expect("an empty record parses");
    tunic.worn_model = "TunicA".into();
    app.insert_resource(exa_world::LevelLocalItems(vec![tunic]))
        .init_resource::<SearchWorld>()
        .init_resource::<exa_physics::sim_app::FrameCommands>()
        .add_systems(Update, carry_out_search);
    let body = app
        .world_mut()
        .spawn((
            Possessions::default(),
            exa_character::Outfit(vec![
                exa_character::Garment {
                    model: "tunica".into(),
                    appearance: None,
                    hides: None,
                    armour: None,
                },
                exa_character::Garment {
                    model: "BootsA".into(),
                    appearance: None,
                    hides: None,
                    armour: None,
                },
            ]),
        ))
        .id();
    app.world_mut()
        .resource_mut::<SearchWorld>()
        .effects
        .redress
        .push(Redress {
            body,
            worn: vec![exa_formats::objdb::LOCAL | 0x10],
            out: [0, 0],
            dead: true,
        });
    app.update();
    let outfit = app.world().get::<exa_character::Outfit>(body).unwrap();
    assert_eq!(
        outfit
            .0
            .iter()
            .map(|g| g.model.as_str())
            .collect::<Vec<_>>(),
        vec!["BootsA"]
    );
    let p = app.world().get::<Possessions>(body).unwrap();
    assert_eq!(
        p.worn.iter().map(|w| w.id).collect::<Vec<_>>(),
        vec![search::UNDERWEAR]
    );
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
        .init_resource::<OpenSearchScreens>()
        .init_resource::<exa_physics::sim_app::FrameCommands>()
        .insert_resource(BodyTicks(0))
        .add_systems(
            Update,
            (follow_and_search, exa_ai::route_walk::walk_routes).chain(),
        );
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

/// A follower standing inside its personal stop replans with the closing
/// goal: it backs off along the line from the player to 400 cm from the
/// player, in the closing mode, with the 300 cm radius.
#[test]
fn a_follower_too_close_backs_off_to_four_metres() {
    let mut app = App::new();
    app.init_resource::<SearchItems>()
        .init_resource::<SearchWorld>()
        .init_resource::<OpenSearchScreens>()
        .insert_resource(BodyTicks(1))
        .add_systems(Update, follow_and_search);
    app.world_mut().spawn((
        Player,
        GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 20.0)),
    ));
    let root = app
        .world_mut()
        .spawn((
            CharacterBody::default(),
            GlobalTransform::from_translation(Vec3::new(20.5, 0.0, 20.0)),
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
    let player = app.world_mut().spawn_empty().id();
    let follower = app
        .world_mut()
        .spawn((
            npc,
            exa_ai::Duellist::new(player),
            Condition::default(),
            Transform::default(),
            GlobalTransform::from_translation(Vec3::new(20.5, 0.0, 20.0)),
            ChildOf(root),
        ))
        .id();
    app.update();
    let world = app.world().resource::<SearchWorld>();
    let state = &world.following[&follower];
    assert_eq!(state.follower.mode, exa_ai::idle::CLOSING);
    let goal = Vec3::new(2400.0, 0.0, 2000.0);
    assert!(
        (state.follower.goal - goal).length() < 1e-3,
        "{:?}",
        state.follower.goal
    );
    let route = app
        .world()
        .get::<exa_ai::route_walk::AiRoute>(follower)
        .unwrap();
    assert!(route.follows(exa_ai::route_walk::Walker::Follow));
    assert_eq!(route.goal(), Some(state.follower.goal));
    // The walk over, the closing mode ends with it.
    app.world_mut()
        .get_mut::<exa_ai::route_walk::AiRoute>(follower)
        .unwrap()
        .stop();
    app.world_mut().resource_mut::<BodyTicks>().0 = 2;
    app.world_mut()
        .entity_mut(root)
        .insert(GlobalTransform::from_translation(Vec3::new(
            24.0, 0.0, 20.0,
        )));
    app.update();
    let world = app.world().resource::<SearchWorld>();
    // Four metres off it follows again (the pursuit's mode), no longer
    // closing.
    assert_eq!(
        world.following[&follower].follower.mode,
        exa_ai::idle::FOLLOWING
    );
}

/// The search reads the GUI's open-container registry: the inventory's and
/// the loot windows' characters through their record ids with the corpse
/// tag, and the popups' ids as they are; a closed window leaves it.
#[test]
fn the_search_reads_the_open_container_registry() {
    let mut app = App::new();
    app.init_resource::<crate::gui_screens::GameScreens>()
        .init_resource::<OpenSearchScreens>()
        .add_systems(Update, keep_open_search_screens);
    {
        let mut screens = app
            .world_mut()
            .resource_mut::<crate::gui_screens::GameScreens>();
        screens.show_inventory(true, true, 4, true, 5);
        screens.open_loot(7, 0, 4);
        screens.containers.open(0x40);
    }
    app.update();
    let open = app.world().resource::<OpenSearchScreens>().clone();
    assert!(open.containers.is_open(5 | 0x2000_0000));
    // Character 7's record id is 70; character 5 has none.
    let ids = open.search_ids(|character| (character == 7).then_some(70));
    assert_eq!(ids, vec![70 | search::CORPSE_TAG, 0x40]);
    app.world_mut()
        .resource_mut::<crate::gui_screens::GameScreens>()
        .close_loot(7);
    app.update();
    let open = app.world().resource::<OpenSearchScreens>().clone();
    assert!(!open.containers.is_open(7 | 0x2000_0000));
}

/// The re-arm's hands through the inventory. The stow is the planned arms:
/// a one-handed weapon keeps a light stowed first beside it (its flags
/// with it) and takes the first stowed slot with no flags; a two-handed
/// one clears the second slot's item; a weapon without grip bits changes
/// nothing. The wield marks the first unmarked carried entry held and the
/// take into the hand holds a weapon; an item without a weapon section
/// only gets the mark.
#[test]
fn the_rearm_hands_go_through_the_inventory() {
    use exa_items::possession::{HELD, ItemKind};
    let kind_of = |id: u32| match id {
        5 => Some(ItemKind {
            light: true,
            weapon: Some(0x11),
            ..ItemKind::default()
        }),
        6 => Some(ItemKind {
            weapon: Some(0x11),
            ..ItemKind::default()
        }),
        7 => Some(ItemKind {
            weapon: Some(0x21),
            ..ItemKind::default()
        }),
        8 => Some(ItemKind {
            weapon: Some(0x01),
            ..ItemKind::default()
        }),
        _ => None,
    };
    let mut p = Possessions {
        stowed: [Hand { id: 5, flags: 3 }, Hand::default()],
        ..Possessions::default()
    };
    stow(&mut p, 6, kind_of);
    assert_eq!(p.stowed, [Hand { id: 6, flags: 0 }, Hand { id: 5, flags: 3 }]);
    stow(&mut p, 8, kind_of);
    assert_eq!(p.stowed[0].id, 6);
    stow(&mut p, 7, kind_of);
    assert_eq!(p.stowed, [Hand { id: 7, flags: 0 }, Hand { id: 0, flags: 3 }]);

    let entry = |id, flags| Carried { id, flags, x: 0, y: 0 };
    let mut p = Possessions {
        carried: vec![entry(6, HELD), entry(6, 0), entry(9, 0)],
        ..Possessions::default()
    };
    assert!(wield(&mut p, 6, kind_of));
    assert_eq!(p.out[0].id, 6);
    assert_eq!(p.carried[1].flags, HELD);
    assert!(!wield(&mut p, 9, kind_of));
    assert_eq!((p.out[0].id, p.carried[2].flags), (6, HELD));
}

/// A comment barks through the near activation: the speaker's
/// `ContextPhrase` takes the phrase; with the installed roles
/// (`EXANIMA_DIR`) the Explorer role's `ContextPhrase` topic toward the
/// Leader role (30, which the recruitment gives the follower toward the
/// player) is activated in the follower's memory of the player.
#[test]
fn a_comment_activates_the_context_phrase_topic() {
    let scripts = std::env::var("EXANIMA_DIR")
        .ok()
        .and_then(|dir| exa_assets::GameData::open(std::path::Path::new(&dir)).ok())
        .map(|data| exa_script::Scripts::load(&data).0)
        .unwrap_or_default();
    let mut conversations = exa_script::Conversations::default();
    let mut follower = exa_script::cast::Character::new(5, "Derrin");
    follower.add_role(&scripts.roles, COMMENT_ROLE);
    follower.add_relation(1, COMMENT_TOWARD, 0);
    follower
        .contacts
        .push(exa_script::cast::Contact::new(1));
    conversations.cast.characters = vec![
        follower,
        exa_script::cast::Character::new(1, "player"),
    ];
    let topic = exa_formats::roles::Name::from_bytes(COMMENT_TOPIC);
    let shipped = scripts
        .roles
        .get(COMMENT_ROLE)
        .and_then(|r| r.dialogue_for(COMMENT_TOWARD, 0))
        .and_then(|d| d.find(&topic))
        .is_some();
    let mut app = App::new();
    app.add_message::<SearchComment>()
        .add_message::<exa_script::ScriptEffect>()
        .insert_resource(conversations)
        .insert_resource(scripts)
        .add_systems(Update, bark_search_comments);
    app.world_mut().write_message(SearchComment {
        speaker: 5,
        phrase: "I found a sword.".into(),
    });
    app.update();
    let conversations = app.world().resource::<exa_script::Conversations>();
    let me = conversations.cast.get(5).unwrap();
    assert_eq!(me.context_phrase, "I found a sword.");
    if shipped {
        let block = exa_script::topics::block(me, 1, COMMENT_TOWARD, COMMENT_ROLE);
        let words = block.and_then(|m| m.words.clone()).unwrap_or_default();
        assert!(
            words.iter().skip(1).any(|w| *w != 0),
            "the topic is activated: {words:?}"
        );
    } else if std::env::var("EXANIMA_DIR").is_ok() {
        panic!("the shipped Explorer role has no ContextPhrase topic toward the Leader");
    }
}

/// The leader the search weighs for: the first contact attended 50 or
/// more that is a living character holding the Player role. A follower
/// whose leader is in sight tells it about a special piece instead of
/// taking it (the comment is barked); with the leader dead it takes the
/// piece itself.
#[test]
fn a_living_leader_hears_about_a_special_piece() {
    for dead in [false, true] {
        let mut app = App::new();
        let mut items = SearchItems {
            tried: true,
            ..SearchItems::default()
        };
        let mut piece = search::Apparel {
            coverage: [0; 20],
            protection: [10; 4],
            occupancy: [0; 20],
            special: 1,
            model_mark: 0,
            sound: 2,
        };
        piece.coverage[0] = 100;
        piece.occupancy[0] = 1;
        items.loot.insert(100, Loot::Apparel(Box::new(piece)));
        app.insert_resource(items)
            .init_resource::<SearchWorld>()
            .init_resource::<OpenSearchScreens>()
            .init_resource::<exa_physics::sim_app::FrameCommands>()
            .insert_resource(BodyTicks(0))
            .add_message::<SearchShot>()
            .add_message::<SearchComment>()
            .add_message::<exa_audio::HandlingSounded>()
            .add_systems(
                Update,
                (
                    follow_and_search,
                    carry_out_search,
                    send_search_shots,
                    exa_ai::route_walk::walk_routes,
                )
                    .chain(),
            );
        app.world_mut().spawn((
            Player,
            GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 20.0)),
        ));
        let mut leader_role = exa_formats::roles::HeldRole {
            id: exa_ai::roles::PLAYER,
            weight: 1.0,
            explicit: true,
            inherited: 0,
            values: Vec::new(),
        };
        leader_role.explicit = true;
        let leader = app
            .world_mut()
            .spawn((
                PlayerBody::new(0.0),
                Roles(vec![leader_role]),
                if dead { Condition::DEAD } else { Condition::default() },
                GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 20.0)),
                Possessions::default(),
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
        npc.contacts.push(exa_ai::contacts::ContactRecord {
            id: leader.to_bits(),
            attention: 60.0,
            ..Default::default()
        });
        let follower = app
            .world_mut()
            .spawn((
                npc,
                exa_ai::Duellist::new(leader),
                Condition::default(),
                Transform::default(),
                GlobalTransform::from_translation(Vec3::new(21.5, 0.0, 20.0)),
                ChildOf(root),
                CharacterId(5),
                Possessions::default(),
            ))
            .id();
        let chest_id = exa_formats::objdb::LOCAL | 0x32;
        app.world_mut().spawn((
            exa_world::PlacedItem {
                id: 0x32,
                tagged_id: chest_id,
                condition: 0,
            },
            exa_world::ObjectSections {
                placement: 0,
                usable: None,
                use_script: None,
                container: Some(exa_formats::containers::Container {
                    width: 4,
                    height: 4,
                    capacity: 20,
                    max_item: 5,
                    children: vec![exa_formats::containers::Child {
                        id: 100,
                        flags: 0,
                        x: 0,
                        y: 0,
                    }],
                }),
            },
            GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 23.0)),
        ));
        let mut comments = Vec::new();
        let mut sounds = Vec::new();
        for tick in 1..8000u64 {
            app.world_mut().resource_mut::<BodyTicks>().0 = tick;
            let velocity = app
                .world()
                .get::<CharacterBody>(root)
                .unwrap()
                .desired_velocity;
            let mut gt = app.world_mut().get_mut::<GlobalTransform>(root).unwrap();
            *gt = GlobalTransform::from_translation(gt.translation() + velocity * 0.012);
            app.update();
            comments.extend(
                app.world_mut()
                    .resource_mut::<Messages<SearchComment>>()
                    .drain(),
            );
            sounds.extend(
                app.world_mut()
                    .resource_mut::<Messages<exa_audio::HandlingSounded>>()
                    .drain(),
            );
            let worn = &app.world().get::<Possessions>(follower).unwrap().worn;
            if !comments.is_empty() || !worn.is_empty() {
                break;
            }
        }
        let mine = app.world().get::<Possessions>(follower).unwrap();
        if dead {
            assert!(comments.is_empty(), "{comments:?}");
            assert_eq!(mine.worn.iter().map(|w| w.id).collect::<Vec<_>>(), [100]);
            // The put-on's handling sound plays at the follower.
            assert_eq!(sounds.len(), 1, "{sounds:?}");
            assert_eq!(sounds[0].index, 2);
        } else {
            assert_eq!(comments.len(), 1, "{mine:?}");
            assert_eq!(comments[0].speaker, 5);
            assert!(mine.worn.is_empty());
        }
    }
}

/// A dead body that still grips its shield: when the searcher takes the
/// shield (here listed among the body's worn items, the take's own test),
/// the grip lets go, the shield's node leaves the world, the body's second
/// hand and held entry are cleared and its worn list keeps the item.
#[test]
fn a_shield_taken_off_the_dead_leaves_its_arm() {
    let mut app = App::new();
    let mut items = SearchItems {
        tried: true,
        ..SearchItems::default()
    };
    let mut piece = search::Apparel {
        coverage: [0; 20],
        protection: [10; 4],
        occupancy: [0; 20],
        special: 0,
        model_mark: 0,
        sound: 0,
    };
    piece.coverage[0] = 100;
    piece.occupancy[0] = 1;
    items.loot.insert(200, Loot::Apparel(Box::new(piece)));
    app.insert_resource(items)
        .insert_resource(catalogue_of(&[(200, vec![(0x8000, vec![0u8; 0x40])])]))
        .init_resource::<SearchWorld>()
        .init_resource::<OpenSearchScreens>()
        .init_resource::<exa_physics::sim_app::FrameCommands>()
        .insert_resource(BodyTicks(0))
        .add_message::<SearchShot>()
        .add_message::<SearchComment>()
        .add_message::<exa_audio::HandlingSounded>()
        .add_systems(
            Update,
            (
                follow_and_search,
                carry_out_search,
                send_search_shots,
                exa_ai::route_walk::walk_routes,
            )
                .chain(),
        );
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
            Possessions::default(),
        ))
        .id();
    let shield = app.world_mut().spawn(Name::new("shield")).id();
    let guard = exa_combat::block::ShieldHold {
        item: shield,
        guard: Default::default(),
    };
    let body = app
        .world_mut()
        .spawn((
            Condition::DEAD,
            GlobalTransform::from_translation(Vec3::new(20.0, 0.0, 23.0)),
            CharacterId(6),
            guard,
            Possessions {
                out: [Hand::default(), Hand { id: 200, flags: 0 }],
                worn: vec![ItemRef { id: 200, flags: 0 }],
                carried: vec![Carried {
                    id: 200,
                    flags: exa_items::possession::HELD,
                    x: 0,
                    y: 0,
                }],
                ..Possessions::default()
            },
        ))
        .id();
    let mut gone = false;
    for tick in 1..4000u64 {
        app.world_mut().resource_mut::<BodyTicks>().0 = tick;
        let velocity = app
            .world()
            .get::<CharacterBody>(root)
            .unwrap()
            .desired_velocity;
        let mut gt = app.world_mut().get_mut::<GlobalTransform>(root).unwrap();
        *gt = GlobalTransform::from_translation(gt.translation() + velocity * 0.012);
        app.update();
        if app.world().get_entity(shield).is_err() {
            gone = true;
            break;
        }
    }
    assert!(gone, "the shield leaves the world");
    let theirs = app.world().get::<Possessions>(body).unwrap();
    assert_eq!(theirs.out[1].id, 0);
    assert!(theirs.carried.is_empty(), "{theirs:?}");
    assert_eq!(theirs.worn.iter().map(|w| w.id).collect::<Vec<_>>(), [200]);
    let mine = app.world().get::<Possessions>(follower).unwrap();
    assert_eq!(mine.worn.iter().map(|w| w.id).collect::<Vec<_>>(), [200]);
}

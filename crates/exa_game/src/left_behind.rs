//! The dead player's body left in the level: the reconstruction device's
//! strip and the story's clear of the body's owned mark, which together make
//! the next save write the body, dressed and armed, into the level where the
//! player fell.
//!
//! **What the original does.** When a scanned player dies in the story the
//! death path strips the player's character
//! ([`leave_behind`]): an unshared record is copied first (written and read
//! back), the copy marked left behind (`0x40` in its profession word), its
//! roles replaced by roles 1 and `0xC2` (each added with the parents it
//! inherits), its yellow and red pools zeroed, and the copy joins the world
//! character database under its first free id. The record the player goes on
//! with then loses its four hands, its worn list (the underwear rule runs on
//! the empty list) and its carried list, and is left at `0x7FFF` yellow and
//! red. The dead body takes the copy as its record, so it stays dressed.
//!
//! The death screen's CONTINUE (the respawn) and its EXIT (for a stripped
//! player) then clear the body's node flag `0x1000` before they save. That
//! flag is the level exporter's owned mark: the player's spawn sets it, and
//! the exporter skips a node carrying it with its subtree, which is why a
//! save's level section never holds the living player (the player is in
//! `#playerdata`). With it cleared, the save's level walk writes the body as
//! one of the scene's characters: the copy's world id, its place and heading,
//! the dead condition and its pose tree, while `#playerdata` writes the
//! stripped record. A later load of that level finds the corpse with its
//! gear, the record coming from the world character database.
//!
//! **What Exarust does.** [`LeftBody`] is the flow's record of it:
//! `Step::StripPlayer` runs [`leave_behind`] on the body's live record (its
//! possessions written back), gives the body the copy and keeps the stripped
//! record; `Step::ClearStoryFlag` sets [`LeftBody::exported`]. The story save
//! then writes the stripped record as the player
//! ([`crate::save_live`]), stores the body's record (with its live
//! possessions) in the world store and adds the body to the level section's
//! characters ([`crate::level_save::LevelSaveScene::left_character`]). The
//! next level change forgets it ([`forget_on_level_change`]).
//!
//! Evidence (REA, Exanima.exe, read in the assembly): the death path's
//! strip call on the player, the strip routine (the shared-record test, the
//! copy, its mark, role count cleared and the two role adds, the two pool
//! words cleared, the first free world id and the add, the hands, the worn
//! count, the underwear rule, the carried count, the pool words `0x7FFF`
//! and the record swap), the respawn's and the exit's `and` of the flag
//! word with `0xFFFFEFFF` before the save, and the exporter's skip of a node
//! with `0x1000` (`docs/formats/RFC.md`, the export walk).

use bevy::prelude::*;
use exa_formats::characters::{CharacterRecord, ItemRef};
use exa_formats::roles::RoleDb;

/// The bit the left-behind copy gets in its profession word.
pub const LEFT_BEHIND_MARK: u32 = exa_world::teleporters::LEFT_BEHIND_MARK;
/// The roles the copy is given after its own are dropped.
pub const LEFT_BEHIND_ROLES: [i32; 2] = exa_world::teleporters::LEFT_BEHIND_ROLES;
/// The yellow and red pool words a strip leaves (of `0xFFFF`).
pub const STRIPPED_POOL: u16 = exa_world::teleporters::STRIPPED_WORD;

/// What a strip made of the player's record.
#[derive(Debug, Clone, PartialEq)]
pub struct Strip {
    /// The dressed copy the dead body keeps, under its world id.
    pub corpse: CharacterRecord,
    /// The record the player goes on with.
    pub player: CharacterRecord,
}

/// The strip of a reconstructed player's record `dressed` (its live
/// possessions written back): the copy described in the module notes, put
/// into `world` under its first free id (`None` leaves the id as it was),
/// with `level` (the current level's id) as its level, and the stripped
/// record. `book` gives the copy's roles their inherited parents (without
/// one they are added alone).
///
/// [Exarust's: the copy's level is written here; the original's player
/// record already holds the current level, which the level's deferred start
/// writes on it.]
pub fn leave_behind(
    dressed: CharacterRecord,
    book: Option<&RoleDb>,
    world: Option<&mut crate::world_characters::WorldCharacters>,
    level: u32,
) -> Strip {
    let mut corpse = dressed.clone();
    corpse.set_profession(corpse.profession() | LEFT_BEHIND_MARK);
    let mut roles = exa_ai::roles::Roles::default();
    for role in LEFT_BEHIND_ROLES {
        match book {
            Some(book) => {
                roles.add(book, role as u32);
            }
            // Prepended, as the role add puts a new entry first.
            None => roles.0.insert(
                0,
                exa_formats::roles::HeldRole {
                    id: role as u32,
                    weight: 0.0,
                    explicit: true,
                    inherited: 0,
                    values: Vec::new(),
                },
            ),
        }
    }
    corpse.roles.clear();
    corpse = crate::save_live::record_with_roles(corpse, &roles);
    corpse.pools.yellow = 0;
    corpse.pools.red = 0;
    corpse.locale = level;
    if let Some(world) = world
        && let Some(shared) = world.db.add(0, corpse.clone())
    {
        corpse.id = shared.id();
    }
    let mut player = exa_character::record::CharacterData::of_record(dressed);
    player.record.hands = [ItemRef::NONE; exa_formats::characters::HANDS];
    player.record.worn.clear();
    player.ensure_underwear(|_| None);
    player.record.inventory.clear();
    player.record.pools.yellow = STRIPPED_POOL;
    player.record.pools.red = STRIPPED_POOL;
    Strip {
        corpse,
        player: player.into_record(),
    }
}

/// The dead player's body the flow left in the level.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct LeftBody {
    /// The player's body that keeps the dressed copy.
    pub body: Option<Entity>,
    /// The stripped record the player goes on with (`None` when no strip
    /// ran: the body's record is then the player's own).
    pub player: Option<CharacterRecord>,
    /// The body's owned mark is cleared: the story save writes it into the
    /// level section.
    pub exported: bool,
}

impl LeftBody {
    /// The record a story save writes as the player's: the stripped one
    /// after a strip.
    pub fn player_record(&self) -> Option<&CharacterRecord> {
        self.player.as_ref()
    }

    /// Whether the save writes `body` into the level section.
    pub fn writes(&self, body: Entity) -> bool {
        self.exported && self.body == Some(body)
    }
}

/// A level change despawns the left body with its level: its record stays
/// in the world store and its placement in the saved section.
pub fn forget_on_level_change(
    changes: Option<Res<exa_world::level_lifecycle::LevelChangeState>>,
    mut seen: Local<u64>,
    mut left: ResMut<LeftBody>,
) {
    let Some(changes) = changes else {
        return;
    };
    if changes.changes == *seen {
        return;
    }
    *seen = changes.changes;
    if *left != LeftBody::default() {
        info!("left body: forgotten with its level ({:?})", left.body);
        *left = LeftBody::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::characters::{Carried, ID_MASK, WORLD_DB};

    fn dressed() -> CharacterRecord {
        let mut record = CharacterRecord::reset();
        record.id = WORLD_DB | crate::save_live::PLAYER_RECORD_ID;
        record.hands[0] = ItemRef::new(0x8000_0010, 0);
        record.hands[2] = ItemRef::new(0x8000_0011, 0);
        record.worn = vec![ItemRef::new(0x8000_0020, 0), ItemRef::new(0x8000_0021, 0)];
        record.inventory = vec![Carried {
            item: 0x8000_0030,
            flags: 0,
            extra: 0,
        }];
        record.pools.yellow = 0xFFFF;
        record.pools.red = 0xC000;
        record.pools.focus = 0x1234;
        record
    }

    /// The strip's two records: the copy keeps every item, takes the mark,
    /// the two roles, no pools, the level and the first free world id; the
    /// player's own loses hands, worn and carried items, wears the
    /// underwear and is left at 0x7FFF.
    #[test]
    fn the_strip_leaves_a_dressed_copy_in_the_world_store() {
        let mut world = crate::world_characters::WorldCharacters::new_story();
        let strip = leave_behind(dressed(), None, Some(&mut world), 7);
        let corpse = &strip.corpse;
        assert_eq!(
            corpse.id,
            WORLD_DB | 2,
            "the first free id after the player's"
        );
        assert_eq!(corpse.hands, dressed().hands);
        assert_eq!(corpse.worn, dressed().worn);
        assert_eq!(corpse.inventory, dressed().inventory);
        assert_eq!(corpse.profession() & LEFT_BEHIND_MARK, LEFT_BEHIND_MARK);
        assert_eq!(
            corpse.roles.iter().map(|r| r.id as i32).collect::<Vec<_>>(),
            [0xC2, 1],
            "newest first"
        );
        assert_eq!((corpse.pools.yellow, corpse.pools.red), (0, 0));
        assert_eq!(corpse.pools.focus, 0x1234);
        assert_eq!(corpse.locale, 7);
        let stored = world.db.get(2).expect("the copy is in the store");
        assert_eq!(stored.record().worn, dressed().worn);
        let player = &strip.player;
        assert_eq!(player.id & ID_MASK, crate::save_live::PLAYER_RECORD_ID);
        assert!(player.hands.iter().all(|h| !h.is_some()));
        assert_eq!(
            player.worn,
            [ItemRef::new(exa_formats::characters::UNDERWEAR_ITEM, 0)]
        );
        assert!(player.inventory.is_empty());
        assert_eq!((player.pools.yellow, player.pools.red), (0x7FFF, 0x7FFF));
        assert_eq!(player.profession(), dressed().profession());
    }

    /// The live path through the flow's and the save's plugins: a scanned
    /// player dies in `exanima02` dressed; the strip keeps the dressed copy
    /// on the body; the respawn's owned-mark clear and save then write the
    /// stripped player into `#playerdata`, the copy into the world store and
    /// the body, dead, into the level section. A later change to the level
    /// reads the save's section, whose corpse resolves through the saved
    /// world store with its gear.
    #[test]
    fn the_dead_players_body_stays_in_the_level_with_its_gear() {
        use crate::flow::{Flow, FlowSteps, Mode, Phase, Step};
        use crate::save_live::{SaveFolder, SaveStep};
        let dir = std::env::temp_dir().join(format!("exarust-left-behind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .init_resource::<exa_physics::collision_share::FramePhysics>()
            .init_resource::<exa_world::gi_volume::GiState>()
            .init_resource::<exa_world::nav::NavState>()
            .init_resource::<exa_world::nav::InstalledPathfinder>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                crate::flow::FlowPlugin,
                crate::save_live::SaveLivePlugin,
                exa_world::level_lifecycle::LevelChangePlugin,
            ))
            .insert_resource(SaveFolder(dir.clone()))
            .insert_resource(exa_ai::brain::RoleDatabase {
                book: RoleDb::default(),
                locale: 3,
                registry: exa_ai::activity::IdleRegistry::shipped(),
            })
            .insert_resource(crate::creator_screen::CreatedCharacter(Some(
                CharacterRecord {
                    name: "Ash".to_string(),
                    ..CharacterRecord::reset()
                },
            )));
        app.update();
        {
            let mut flow = app.world_mut().resource_mut::<Flow>();
            flow.mode = Mode::Story;
            flow.phase = Phase::Ended;
            flow.menu_open = false;
        }
        let level = exa_formats::rfc_write::export_save_level(
            &exa_formats::rfc_write::ExportScene {
                whole_scene: true,
                ..Default::default()
            },
            &exa_formats::rfc_write::ExportOptions::default(),
            None,
        )
        .unwrap();
        app.insert_resource(exa_world::LevelSource {
            entry: "exanima02.rfc".to_string(),
            bytes: level.into(),
            from_save: false,
            spawned: Vec::new(),
        });
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "exanima02".to_string(),
        });
        // Shipped items (no tier tag): the save's promotion leaves them.
        let mut record = dressed();
        record.name = "Ash".to_string();
        for hand in &mut record.hands {
            hand.id &= ID_MASK;
        }
        for worn in &mut record.worn {
            worn.id &= ID_MASK;
        }
        for carried in &mut record.inventory {
            carried.item &= ID_MASK;
        }
        let gear = record.clone();
        let possessions = exa_items::possession::Possessions::of_record(&record);
        let body = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                exa_combat::Condition(crate::flow::DEAD),
                exa_combat::Fighter::default(),
                exa_character::record::CharacterData::of_record(record),
                possessions,
                GlobalTransform::from_translation(Vec3::new(1.5, 0.25, -4.0)),
            ))
            .id();
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        // The death's strip.
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::StripPlayer);
        app.update();
        let left = app.world().resource::<LeftBody>().clone();
        assert_eq!(left.body, Some(body));
        assert!(!left.exported);
        let corpse_id = app
            .world()
            .get::<exa_character::record::CharacterData>(body)
            .unwrap()
            .record
            .id;
        assert_eq!(corpse_id, WORLD_DB | 2);
        // The respawn's clear and save.
        app.world_mut().resource_mut::<FlowSteps>().steps.extend([
            Step::ClearStoryFlag,
            Step::Save {
                drop_on_low_health: false,
            },
        ]);
        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<crate::save_live::SaveReport>().saved,
            Some(1)
        );
        let mut list = crate::slots::SlotList::new(crate::slots::Slots::story(&dir));
        assert!(list.any());
        let restored = crate::save_live::restore_story(&mut list, 1, false).unwrap();
        // The player goes on stripped.
        let player = restored.player.clone().unwrap();
        assert!(player.hands.iter().all(|h| !h.is_some()));
        assert!(
            player
                .worn
                .iter()
                .all(|w| w.id == exa_formats::characters::UNDERWEAR_ITEM)
        );
        assert!(player.inventory.is_empty());
        // The world store holds the copy, dressed.
        let store = exa_formats::character_db::CharacterDb::load(
            exa_formats::character_db::WORLD_TAG,
            restored.world_characters.as_deref().unwrap(),
        )
        .unwrap();
        let mut world = crate::world_characters::WorldCharacters::from_save(Some(store));
        let records = world.others();
        let copy = records
            .iter()
            .find(|r| r.id & ID_MASK == 2)
            .expect("the copy is saved in the world store");
        assert_eq!(copy.worn, gear.worn);
        assert_eq!(copy.locale, 3);
        // The level's section holds the body, dead, under the copy's id.
        let section = restored.level_section.clone().expect("the level section");
        assert!(section.is_for("exanima02"));
        let scene = exa_formats::rfc::Scene::parse(&section.bytes).unwrap();
        let placements = &scene.characters.as_ref().unwrap().placements;
        let placed = placements
            .iter()
            .find(|p| p.id == corpse_id)
            .expect("the body's placement");
        assert!(placed.is_dead());
        assert!((placed.position[0] - 1.5).abs() < 1e-4, "{placed:?}");
        // A later visit's load resolves it with its gear, in its level only.
        let found = exa_world::placed_characters(&scene, &[], Some(3), &records);
        let corpse = found
            .iter()
            .find(|c| c.record.id & ID_MASK == 2)
            .expect("the corpse is placed");
        assert!(corpse.dead);
        assert_eq!(corpse.record.worn, gear.worn);
        assert_eq!(corpse.record.hands, gear.hands);
        assert_eq!(corpse.record.inventory, gear.inventory);
        assert!(exa_world::placed_characters(&scene, &[], Some(4), &records).is_empty());
        // The change back to the level reads the save's section and
        // forgets the body.
        app.world_mut()
            .write_message(exa_world::level_lifecycle::LevelChangeRequest::change(
                "exanima02",
                0,
            ));
        app.update();
        app.update();
        let saved = app
            .world()
            .get_resource::<exa_world::SavedLevelSection>()
            .expect("the save's section is handed to the load");
        assert_eq!(saved.bytes, section.bytes);
        assert_eq!(*app.world().resource::<LeftBody>(), LeftBody::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The death screen's CONTINUE through the flow's and the save's
    /// plugins: the stripped record's identity is reset and posted for a
    /// rebuild; the respawn's save writes it as the player, then the player
    /// root (the dead body with the dressed copy) goes and the stripped
    /// record is the next spawn's, so the player wakes in the underwear
    /// while the corpse keeps the gear in the saved level.
    #[test]
    fn the_continue_rebuilds_the_stripped_player_from_its_record() {
        use crate::flow::{Flow, FlowSteps, Mode, Phase, Step};
        use crate::save_live::{LoadedPlayerStream, SaveFolder, SaveStep, SavedPlayer};
        let dir =
            std::env::temp_dir().join(format!("exarust-continue-rebuild-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<exa_ui::gui_draw::GuiFrames>()
            .init_resource::<exa_physics::collision_share::FramePhysics>()
            .init_resource::<exa_world::gi_volume::GiState>()
            .init_resource::<exa_world::nav::NavState>()
            .init_resource::<exa_world::nav::InstalledPathfinder>()
            .add_plugins((
                crate::menu_screen::MenuScreenPlugin,
                crate::flow::FlowPlugin,
                crate::save_live::SaveLivePlugin,
                exa_world::level_lifecycle::LevelChangePlugin,
            ))
            .insert_resource(SaveFolder(dir.clone()))
            .insert_resource(exa_ai::brain::RoleDatabase {
                book: RoleDb::default(),
                locale: 3,
                registry: exa_ai::activity::IdleRegistry::shipped(),
            })
            .insert_resource(crate::creator_screen::CreatedCharacter(Some(
                CharacterRecord {
                    name: "Ash".to_string(),
                    ..CharacterRecord::reset()
                },
            )));
        app.update();
        {
            let mut flow = app.world_mut().resource_mut::<Flow>();
            flow.mode = Mode::Story;
            flow.phase = Phase::Ended;
            flow.menu_open = false;
        }
        let level = exa_formats::rfc_write::export_save_level(
            &exa_formats::rfc_write::ExportScene {
                whole_scene: true,
                ..Default::default()
            },
            &exa_formats::rfc_write::ExportOptions::default(),
            None,
        )
        .unwrap();
        app.insert_resource(exa_world::LevelSource {
            entry: "exanima05.rfc".to_string(),
            bytes: level.into(),
            from_save: false,
            spawned: Vec::new(),
        });
        app.world_mut().spawn(exa_world::LevelRoot {
            name: "exanima05".to_string(),
        });
        let mut record = dressed();
        record.name = "Ash".to_string();
        record.set_profession(record.profession() | 0x0300);
        for hand in &mut record.hands {
            hand.id &= ID_MASK;
        }
        for worn in &mut record.worn {
            worn.id &= ID_MASK;
        }
        for carried in &mut record.inventory {
            carried.item &= ID_MASK;
        }
        let possessions = exa_items::possession::Possessions::of_record(&record);
        let root = app
            .world_mut()
            .spawn((crate::player::Player, Transform::default()))
            .id();
        let body = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                exa_combat::Condition(crate::flow::DEAD),
                exa_combat::Fighter::default(),
                exa_character::record::CharacterData::of_record(record),
                possessions,
                GlobalTransform::from_translation(Vec3::new(1.5, 0.25, -4.0)),
                ChildOf(root),
            ))
            .id();
        app.world_mut().write_message(SaveStep::NewSlot);
        app.update();
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(Step::StripPlayer);
        app.update();
        // The CONTINUE: the stripped record's race byte is cleared and its
        // request posted (no AI start, the world scene).
        let continued = app
            .world_mut()
            .resource_mut::<Flow>()
            .continue_after_death();
        assert_eq!(continued[0], Step::PlayerData { flags: 0x100 });
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .push(continued[0]);
        app.update();
        let stream = app
            .world()
            .get_resource::<LoadedPlayerStream>()
            .expect("the rebuild's request");
        assert_eq!(
            stream.request.flags,
            exa_assets::content_stream::NO_AI_START
        );
        assert_eq!(stream.request.scene, crate::content_requests::WORLD_SCENE);
        assert_eq!(stream.record.race, 0, "the identity reset");
        assert!(
            stream
                .record
                .worn
                .iter()
                .all(|w| w.id == exa_formats::characters::UNDERWEAR_ITEM)
        );
        let left = app.world().resource::<LeftBody>().clone();
        assert_eq!(left.player.as_ref().unwrap().race, 0);
        // The respawn at the transition's black.
        let respawn = app.world_mut().resource_mut::<Flow>().story_respawn(0);
        let save = respawn
            .iter()
            .position(|s| matches!(s, Step::Save { .. }))
            .unwrap();
        let rebuild = respawn
            .iter()
            .position(|s| *s == Step::RebuildPlayer)
            .unwrap();
        let teardown = respawn
            .iter()
            .position(|s| *s == Step::TeardownWorld)
            .unwrap();
        assert!(save < rebuild && rebuild < teardown);
        app.world_mut()
            .resource_mut::<FlowSteps>()
            .steps
            .extend(respawn);
        app.update();
        app.update();
        // The save wrote the stripped, reset record as the player.
        assert_eq!(
            app.world().resource::<crate::save_live::SaveReport>().saved,
            Some(1)
        );
        let mut list = crate::slots::SlotList::new(crate::slots::Slots::story(&dir));
        let restored = crate::save_live::restore_story(&mut list, 1, false).unwrap();
        let saved_player = restored.player.clone().unwrap();
        assert!(saved_player.hands.iter().all(|h| !h.is_some()));
        assert_eq!(
            (saved_player.pools.yellow, saved_player.pools.red),
            (STRIPPED_POOL, STRIPPED_POOL)
        );
        // The old player went with the scene; the next spawn is made from
        // the stripped record.
        assert!(app.world().get_entity(root).is_err());
        assert!(app.world().get_entity(body).is_err());
        let next = app
            .world()
            .get_resource::<SavedPlayer>()
            .expect("the rebuild's record");
        assert_eq!(next.record.name, "Ash");
        assert_eq!(next.record.race, 0);
        assert!(next.record.hands.iter().all(|h| !h.is_some()));
        assert!(next.record.inventory.is_empty());
        // Nothing of the gear is worn (the strip's underwear rule gives the
        // race-3 record none).
        assert!(
            next.record
                .worn
                .iter()
                .all(|w| w.id == exa_formats::characters::UNDERWEAR_ITEM)
        );
        assert_ne!(next.record.worn, dressed().worn);
        assert_eq!(
            (next.record.pools.yellow, next.record.pools.red),
            (STRIPPED_POOL, STRIPPED_POOL)
        );
        // The saved level keeps the corpse with its gear.
        let section = restored.level_section.clone().expect("the level section");
        let scene = exa_formats::rfc::Scene::parse(&section.bytes).unwrap();
        assert!(
            scene
                .characters
                .as_ref()
                .unwrap()
                .placements
                .iter()
                .any(|p| p.is_dead())
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}

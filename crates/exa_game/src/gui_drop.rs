//! The inventory's drop into the world: an item dragged out of the
//! player's pack over the world view is claimed by the controls, put into
//! the scene on the next frame and lifted onto the item cursor.
//!
//! **The claim.** While a drag out of the player's pack is held over the
//! world view (no open form under the pointer), the controls take it
//! ([`item_cursor::claim_drag`] with the item payload): the drop task is
//! queued ([`ItemCursor::queue_drop`]), the controls' word is armed
//! ([`player_controls::SPAWN_ARMED`]) and the panel's drag is handed over,
//! so its release no longer cancels it. An entry held in a hand is not
//! claimed [Exarust's choice: the hands would keep an item the world now
//! holds].
//!
//! **The deferred run** ([`player_controls::run_spawn`]). The original hands
//! the task to a worker thread; Exarust runs it on the next frame. The
//! item's model is loaded and its rigid bodies built; a model with no body
//! is the original's "Game object has no physics" content error (the node
//! freed, the word reset to armed, the drag cancelled). A placed item is
//! put where the cursor's ray meets the level (or the placement limit's
//! height, [`item_cursor::place_point`]), takes the drag's condition word,
//! leaves the pack (the panel's entry and the record's), and the word ends
//! at [`player_controls::SPAWN_DONE`]. The spawned node carries the record's
//! template word, usable and container sections and mechanism records, as a
//! level-placed object does (`exa_world::dropped_object`); once its body
//! exists the hand begins the lift on it (`hand::lift_gui_drops`), so the
//! item rides the cursor until the button that dragged it is let go.
//!
//! The debug flag `--gui-drop ID@FRAME` gives the player's pack the item
//! and queues its drop at that frame after the load, as a drag out of the
//! window would.
//!
//! Evidence: the controls' GUI drag claim, the drop routine and its
//! deferred task (`context/re/player-control-hand.md`).

use bevy::image::CompressedImageFormatSupport;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_assets::{Game, ModelCache, ModelLoader, PackKind, model};
use exa_items::catalogue::ItemCatalogue;
use exa_items::possession::{HELD, Possessions};
use exa_ui::ScriptedPointer;

use crate::body_inventory::{self, PanelKind};
use crate::gui_screens::GameScreens;
use crate::hand::{OtherBodies, PickSources};
use crate::item_cursor::{self, DragClaim, DropTask, ITEM_PAYLOAD, ItemCursor};
use crate::panel_live::{Panel, PanelDrag, PointerOverGui, container_mut};
use crate::player::{ChaseCamera, PlayerBody};
use crate::player_controls::{self, SpawnRun};

/// The drag the controls took from the pack, until its drop runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TakenDrag {
    pub source: Panel,
    pub id: u32,
    pub flags: u32,
    pub bulk: i32,
}

/// The controls' drop word ([`player_controls::SPAWN_ARMED`] once a drag is
/// claimed, [`player_controls::SPAWN_DONE`] once the item is placed, zero
/// when the lift takes it) and the claimed drag.
#[derive(Resource, Debug, Default)]
pub struct GuiDrop {
    pub word: u32,
    pub taken: Option<TakenDrag>,
}

/// A placed drop whose body the hand lifts once it exists: the item, the
/// condition word it took, and whether the body's frame goes to the
/// controls.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiDropLift {
    pub item: u32,
    pub condition: u32,
    pub copy_frame: bool,
}

/// `--gui-drop ID@FRAME`: the item put into the player's pack and dropped
/// at that frame after the load.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuiDropScript(pub Option<(u32, u32)>);

impl GuiDropScript {
    /// Reads `ID@FRAME` (the id in hexadecimal with or without `0x`, or
    /// decimal).
    pub fn parse(text: &str) -> Option<Self> {
        let (id, frame) = text.split_once('@')?;
        Some(Self(Some((parse_id(id)?, frame.trim().parse().ok()?))))
    }
}

/// An item id: `0x`-prefixed hexadecimal or decimal.
pub fn parse_id(text: &str) -> Option<u32> {
    let text = text.trim();
    match text.strip_prefix("0x") {
        Some(hex) => u32::from_str_radix(hex, 16).ok(),
        None => text.parse().ok(),
    }
}

/// Whether the controls take a pack drag over the world: the drag comes
/// from the player's pack and is not a held entry, the button is down and
/// no form is under the pointer, then the controls' own claim rule, which
/// also refuses a point that is out of reach (the hand's flag).
pub fn claims(
    source: Option<Panel>,
    flags: u32,
    button_down: bool,
    over_gui: bool,
    out_of_reach: bool,
    pending: bool,
    holding: bool,
) -> bool {
    source == Some(Panel::PlayerPack)
        && flags & HELD == 0
        && button_down
        && !over_gui
        && item_cursor::claim_drag(ITEM_PAYLOAD, false, out_of_reach, pending, holding)
            == DragClaim::Take
}

/// Claims a pack drag held over the world view (see the module notes).
#[allow(clippy::too_many_arguments)]
pub fn claim_gui_drag(
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    over: Res<PointerOverGui>,
    hand: Res<crate::hand::Hand>,
    mut drag: ResMut<PanelDrag>,
    mut items: ResMut<ItemCursor>,
    mut drop: ResMut<GuiDrop>,
) {
    if drag.source.is_none() {
        return;
    }
    let pending = !items.drops.is_empty() || drop.taken.is_some();
    if !claims(
        drag.source,
        drag.flags,
        buttons.pressed(MouseButton::Left),
        over.0,
        hand.out_of_reach,
        pending,
        items.held.is_some(),
    ) {
        return;
    }
    let Some(pointer) = windows.single().ok().and_then(|w| {
        w.cursor_position()
            .map(|p| (p * w.scale_factor()).as_ivec2())
    }) else {
        return;
    };
    let Some(source) = drag.source else {
        return;
    };
    info!("gui drop: the controls take {:#x} out of the pack", drag.id);
    items.queue_drop(DropTask {
        item: drag.id,
        from_gui: true,
        pointer,
    });
    drop.word |= player_controls::SPAWN_ARMED;
    drop.taken = Some(TakenDrag {
        source,
        id: drag.id,
        flags: drag.flags,
        bulk: drag.bulk,
    });
    *drag = PanelDrag::default();
}

/// The debug drop: at its frame after the load, the item goes into the
/// player's pack record and its drop is queued as a claimed drag would.
fn scripted_gui_drop(
    script: Res<GuiDropScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut players: Query<&mut Possessions, With<PlayerBody>>,
    mut items: ResMut<ItemCursor>,
    mut drop: ResMut<GuiDrop>,
) {
    let Some((id, frame)) = script.0 else {
        return;
    };
    if loading.is_some() || *frames > frame {
        return;
    }
    *frames += 1;
    if *frames != frame {
        return;
    }
    let Some(mut p) = players.iter_mut().next() else {
        info!("gui drop script: no player at frame {frame}");
        return;
    };
    p.carried.push(exa_items::possession::Carried {
        id,
        flags: 0,
        x: 0,
        y: 0,
    });
    info!("gui drop script: {id:#x} dropped from the pack at frame {frame}");
    items.queue_drop(DropTask {
        item: id,
        from_gui: true,
        pointer: IVec2::ZERO,
    });
    drop.word |= player_controls::SPAWN_ARMED;
    drop.taken = Some(TakenDrag {
        source: Panel::PlayerPack,
        id,
        flags: 0,
        bulk: 0,
    });
}

/// What the drop run reads to place the item: the cursor's ray and the
/// world's pick.
#[derive(bevy::ecs::system::SystemParam)]
pub struct DropPlace<'w, 's> {
    scripted: Res<'w, ScriptedPointer>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<ChaseCamera>>,
    sources: PickSources<'w>,
    bodies: Query<'w, 's, OtherBodies, Without<PlayerBody>>,
}

impl DropPlace<'_, '_> {
    /// Where the cursor's ray puts a dropped item: its first hit, or the
    /// placement limit's height (none for a climbing ray).
    fn point(&self) -> Option<Vec3> {
        let ray = crate::hand::cursor_ray(&self.scripted, &self.windows, &self.cameras)?;
        let picked = crate::hand::pick_with(
            Some(ray),
            &self.sources.physics,
            &self.sources.stand_ins,
            &self.sources.collider_nodes,
            &self.bodies,
            None,
        );
        let origin = ray.origin;
        let hit = picked.map(|p| (p.at, p.at.y * 100.0));
        item_cursor::place_point(origin, *ray.direction, hit, item_cursor::PLACEMENT_LIMIT_CM)
    }
}

/// What the drop run needs to build an item's model.
#[derive(bevy::ecs::system::SystemParam)]
pub struct DropAssets<'w> {
    game: Option<Res<'w, Game>>,
    catalogue: Option<Res<'w, ItemCatalogue>>,
    cache: ResMut<'w, ModelCache>,
    meshes: ResMut<'w, Assets<Mesh>>,
    materials: ResMut<'w, Assets<exa_render::material::ExaMaterial>>,
    images: ResMut<'w, Assets<Image>>,
    support: Option<Res<'w, CompressedImageFormatSupport>>,
}

/// Runs the drops queued on the previous frame (see the module notes).
#[allow(clippy::too_many_arguments)]
pub fn run_gui_drops(
    mut commands: Commands,
    mut items: ResMut<ItemCursor>,
    mut drop: ResMut<GuiDrop>,
    mut screens: ResMut<GameScreens>,
    mut assets: DropAssets,
    mut records: Option<ResMut<crate::doll_view::DollItems>>,
    place: DropPlace,
    mut players: Query<&mut Possessions, With<PlayerBody>>,
) {
    if items.drops.is_empty() {
        return;
    }
    for task in items.take_drops() {
        let taken = drop.taken.take();
        let condition = taken.map_or(0, |t| t.flags);
        let cancel = |screens: &mut GameScreens| {
            if let Some(taken) = taken
                && let Some(container) = container_mut(screens, taken.source)
            {
                container.finish_drag(0, true, true, 0, taken.bulk);
            }
        };
        if item_cursor::drop(true, task.from_gui, true) == item_cursor::Drop::NotFromGui {
            cancel(&mut screens);
            continue;
        }
        let model = assets
            .catalogue
            .as_deref()
            .and_then(|c| c.model(task.item))
            .cloned();
        let name = model.as_ref().map_or_else(String::new, |m| m.model.clone());
        let built = match (assets.game.as_deref(), model.as_ref()) {
            (Some(game), Some(item)) => {
                let mut loader = ModelLoader {
                    game: &game.0,
                    cache: &mut assets.cache,
                    meshes: &mut assets.meshes,
                    materials: &mut assets.materials,
                    images: &mut assets.images,
                    bc_supported: model::bc_supported(assets.support.as_deref()),
                };
                match item.model.strip_prefix('@') {
                    Some(factory) => loader.factory_weapon(factory, item.weapon.as_ref()),
                    None => loader
                        .model(PackKind::Objects, &item.model)
                        .or_else(|| loader.model(PackKind::Objlib, &item.model)),
                }
            }
            _ => None,
        };
        // The node is made and its bodies built before the run asks whether
        // it has physics, as the original's task finds the body on the node.
        // An assembled factory weapon's merged node is its body (its
        // section's sound byte picks the hit set).
        let factory_sound = model.as_ref().and_then(|item| {
            item.model.starts_with('@').then(|| {
                item.weapon
                    .as_ref()
                    .map_or(0, |w| w.stats.get(exa_formats::weapon_stats::at::SOUND))
            })
        });
        let spawned = built.as_ref().map(|m| {
            let (root, nodes) = exa_world::spawn_model_nodes(&mut commands, m, None);
            let (parts, sounds) =
                exa_character::item_bodies(m, root, &nodes, &assets.meshes, factory_sound);
            (root, (parts, sounds))
        });
        let has_body = spawned
            .as_ref()
            .is_some_and(|(_, (parts, _))| !parts.is_empty());
        match player_controls::run_spawn(true, drop.word, has_body, condition) {
            SpawnRun::Nothing => {}
            SpawnRun::Dropped => {
                if let Some((root, _)) = spawned {
                    commands.entity(root).despawn();
                }
                cancel(&mut screens);
            }
            SpawnRun::ContentError { word } => {
                error!("Game object has no physics: {name}");
                if let Some((root, _)) = spawned {
                    commands.entity(root).despawn();
                }
                drop.word = word;
                cancel(&mut screens);
            }
            SpawnRun::Placed {
                condition,
                place_limit_cm,
                end_word,
                copy_frame,
            } => {
                let Some((root, (parts, sounds))) = spawned else {
                    continue;
                };
                let at = place.point().unwrap_or_else(|| {
                    // No pick and a climbing ray: in front of the camera.
                    place
                        .cameras
                        .single()
                        .map(|(_, eye)| eye.translation() + eye.forward() * 1.5)
                        .unwrap_or_default()
                });
                info!(
                    "gui drop: {:#x} ({name}) placed at ({:.2}, {:.2}, {:.2}), condition {condition:#x}, limit {place_limit_cm} cm",
                    task.item, at.x, at.y, at.z
                );
                // The spawned record carries what a placed object does:
                // its template word, its usable and container sections and
                // its mechanism records, from the object database record
                // the drag named. Only a game record is looked up (the
                // database is keyed by untagged ids, so a level-tagged id
                // would find another record); without one the node keeps
                // its id word and condition alone.
                let object = exa_world::dropped_object(task.item, condition);
                let record = records
                    .as_deref_mut()
                    .filter(|_| task.item & !exa_formats::characters::ID_MASK == 0)
                    .and_then(|records| {
                        records.ensure(assets.game.as_deref());
                        records.get(task.item).cloned()
                    });
                // The new node is the item's object, as a placed one is:
                // its id word and the condition's third byte.
                let placed = record.as_ref().map_or(
                    exa_world::PlacedItem {
                        id: task.item,
                        tagged_id: task.item,
                        condition: ((condition >> 16) & 0xff) as u8,
                    },
                    |record| exa_world::PlacedItem::of(record, &object),
                );
                commands.entity(root).insert((
                    Transform::from_translation(at),
                    exa_physics::prop::PendingProp(parts),
                    sounds,
                    exa_combat::impact::ImpactBook::default(),
                    placed,
                    GuiDropLift {
                        item: task.item,
                        condition,
                        copy_frame,
                    },
                ));
                if let Some(record) = record.as_ref() {
                    commands
                        .entity(root)
                        .insert(exa_world::markers::PlacedTemplate(record.parent));
                    if let Some(sections) = exa_world::ObjectSections::of(record, &object) {
                        commands.entity(root).insert(sections);
                    }
                    if let Some(mechanisms) =
                        exa_world::mechanisms::MechanismRecords::of(task.item, record)
                    {
                        commands.entity(root).insert(mechanisms);
                    }
                }
                drop.word = end_word;
                // The item leaves the pack: the panel's entry and the
                // record's carried entry.
                if let Some(taken) = taken {
                    if let Some(container) = container_mut(&mut screens, taken.source) {
                        container.finish_drag(0, false, true, 0, taken.bulk);
                    }
                    if let Some(mut p) = players.iter_mut().next() {
                        body_inventory::take(&mut p, PanelKind::Pack, taken.id);
                    }
                }
            }
        }
    }
}

/// The inventory's drop into the world.
pub struct GuiDropPlugin;

impl Plugin for GuiDropPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuiDrop>()
            .init_resource::<GuiDropScript>()
            .init_resource::<ItemCursor>()
            .add_systems(
                Update,
                (run_gui_drops, scripted_gui_drop, claim_gui_drag)
                    .chain()
                    .after(crate::panel_live::PanelInput)
                    .before(crate::hand::HandSystems),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Only a pack drag that is not a held entry, held down over the world
    /// in reach with nothing pending or lifted, is the controls'.
    #[test]
    fn the_controls_claim_a_pack_drag_over_the_world() {
        let pack = Some(Panel::PlayerPack);
        assert!(claims(pack, 0, true, false, false, false, false));
        assert!(!claims(None, 0, true, false, false, false, false));
        assert!(!claims(
            Some(Panel::BodyPack(3)),
            0,
            true,
            false,
            false,
            false,
            false
        ));
        assert!(!claims(pack, HELD, true, false, false, false, false));
        assert!(!claims(pack, 0, false, false, false, false, false));
        assert!(!claims(pack, 0, true, true, false, false, false));
        // The point under the cursor out of reach: the drag is not taken.
        assert!(!claims(pack, 0, true, false, true, false, false));
        // A drop already pending, or an item on the cursor, keeps it.
        assert!(!claims(pack, 0, true, false, false, true, false));
        assert!(!claims(pack, 0, true, false, false, false, true));
    }

    #[test]
    fn the_drop_script_reads_its_item_and_frame() {
        assert_eq!(
            GuiDropScript::parse("0x12@300"),
            Some(GuiDropScript(Some((0x12, 300))))
        );
        assert_eq!(
            GuiDropScript::parse("18@5"),
            Some(GuiDropScript(Some((18, 5))))
        );
        assert_eq!(GuiDropScript::parse("0x12"), None);
        assert_eq!(GuiDropScript::parse("x@1"), None);
    }

    /// A queued drop the next frame runs: the claimed drag's entry leaves
    /// the pack panel when the item is placed, and stays (its drag
    /// cancelled) when the item has no physics.
    #[test]
    fn a_drop_with_no_model_is_the_content_error() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(bevy::asset::AssetPlugin::default())
            .init_asset::<Mesh>()
            .init_asset::<Image>()
            .init_asset::<exa_render::material::ExaMaterial>()
            .init_resource::<ModelCache>()
            .init_resource::<GameScreens>()
            .init_resource::<ScriptedPointer>()
            .init_resource::<ItemCursor>()
            .init_resource::<GuiDrop>()
            .insert_resource(exa_physics::Physics::default())
            .init_resource::<exa_physics::prop::StandIns>()
            .init_resource::<exa_physics::ColliderNodes>()
            .add_systems(Update, run_gui_drops);
        app.world_mut()
            .resource_mut::<ItemCursor>()
            .queue_drop(DropTask {
                item: 0x12,
                from_gui: true,
                pointer: IVec2::ZERO,
            });
        {
            let mut drop = app.world_mut().resource_mut::<GuiDrop>();
            drop.word = player_controls::SPAWN_ARMED;
            drop.taken = Some(TakenDrag {
                source: Panel::PlayerPack,
                id: 0x12,
                flags: 0,
                bulk: 1,
            });
        }
        app.update();
        let drop = app.world().resource::<GuiDrop>();
        assert_eq!(drop.word, player_controls::SPAWN_ARMED);
        assert!(drop.taken.is_none());
        assert!(app.world().resource::<ItemCursor>().drops.is_empty());
    }
}

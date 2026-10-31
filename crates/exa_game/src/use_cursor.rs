//! The use cursor: an item's use begun from the item panels and ended by
//! the next mouse press.
//!
//! **The begin.** The item panel's double click on an entry asks the item's
//! double-click class ([`player_controls::panel_use_class`]). Class 0 on an
//! item that has a container section opens that section in the container
//! popup. Class 2 closes the card and calls the game's use hook with the
//! item, its entry word, its slot (from one) and the panel. The controls
//! install that hook at start-up: [`player_controls::use_begin`] opens the
//! GUI item a class-0
//! kind-0x11 section names (not wired: the readable's window), or, while
//! play is on, lifts the use cursor (the controls' state word
//! [`player_controls::USE_STATE`], the shared cursor mode
//! [`player_controls::USE_CURSOR_MODE`]) and holds the mouse for the world
//! view.
//!
//! **The end.** The world view hands its pending callback
//! [`player_controls::USE_END_EVENT`] before it serves the next mouse event,
//! so the next press of any button ends the cursor
//! ([`player_controls::use_end`]); the press is taken (the answer's bit 0)
//! and neither the hand nor the panels see it. The dispatcher runs only for
//! the left button [guess: the desk word's bit 0], so a right press puts the
//! cursor down without a use. The dispatcher reads what is under the
//! pointer, in the original's order: over the world view, a door (its key
//! unlocks it when latched; another key rattles it), a character (a liquid
//! is poured), a placed item (cleaned or repaired); over a window, the
//! character view (a liquid) or an item panel's entry
//! ([`player_controls::use_release`]). Exarust applies the unlock, the rattle
//! and a pour on the body (a salve fills its heal pool, an elixir sets the
//! potion). A cleaning or repair kit plays its sound and lowers the item's
//! dirt or wear over two seconds.
//!
//! The debug flag `--use-item ID@FRAME` puts the item into the player's pack
//! and calls the use hook for it at that frame after the load, as its
//! double click would.
//!
//! Evidence: the controls' use hook, its cursor callback, the item panel's
//! mouse handler and double-click class, and the use dispatcher
//! (`context/re/player-control-hand.md`).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_assets::Game;
use exa_items::possession::Possessions;
use exa_physics::body::DoorBody;
use exa_physics::mechanism_sound::{self, MechanismSounded};
use exa_ui::ScriptedPointer;

use crate::doll_view::DollItems;
use crate::gui_screens::GameScreens;
use crate::hand::{ObjectIds, OtherBodies, PickSources};
use crate::panel_live::{Panel, PointerOverGui, container, entry_at, open_panels, panel_at};
use crate::player::{ChaseCamera, PlayerBody};
use crate::player_controls::{
    self, DoorLock, PlaySession, UseBegin, UseCursor, UseOver, UseRelease,
};

/// The game's use hook called by an item panel's double click: the item,
/// its entry word, the entry's slot from one, and the panel.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseHook {
    pub item: u32,
    pub entry: u32,
    pub slot: i32,
    pub panel: Panel,
}

/// A use the dispatcher ran: its branch, the item used and what it was
/// used on (the picked body, or none for a window).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct UseReleased {
    pub release: UseRelease,
    /// The item the cursor was holding.
    pub item: u32,
    pub target: Option<Entity>,
    /// The item the use lands on: a placed object's id, or the panel entry's
    /// id. Zero when the release has no item under it.
    pub subject: u32,
    /// The panel entry under the pointer, when the use landed on one.
    pub subject_entry: Option<(Panel, usize)>,
}

/// `--use-item ID@FRAME`: the item put into the player's pack and its use
/// hook called at that frame after the load.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UseScript(pub Option<(u32, u32)>);

impl UseScript {
    pub fn parse(text: &str) -> Option<Self> {
        let (id, frame) = text.split_once('@')?;
        Some(Self(Some((
            crate::gui_drop::parse_id(id)?,
            frame.trim().parse().ok()?,
        ))))
    }
}

/// The world classification the dispatcher tests for a picked body: a door
/// leaf is 4, a living or dead character 0x10, a placed item 3 (bit 2) and
/// anything else 0.
pub fn classification(character: bool, door: bool, placed_item: bool) -> u8 {
    if door {
        4
    } else if character {
        0x10
    } else if placed_item {
        3
    } else {
        0
    }
}

/// The picked door's lock for the dispatcher: a latched leaf is locked, and
/// its key matches when the door section names the used item.
pub fn door_lock(latched: bool, key: u32, item: u32) -> DoorLock {
    if latched {
        DoorLock::Locked {
            key_matches: key == item,
        }
    } else {
        DoorLock::Open
    }
}

/// The item's operative class and kind, from its record.
fn operative_of(items: &DollItems, id: u32) -> Option<(u8, u8, u32)> {
    items
        .get(id)
        .and_then(|item| item.operative.as_ref())
        .map(|o| (o.class, o.kind, o.target))
}

/// The pointer in physical window pixels.
fn pointer(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<IVec2> {
    let window = windows.single().ok()?;
    window
        .cursor_position()
        .map(|p| (p * window.scale_factor()).as_ivec2())
}

/// The item panel's double click: a second left press within the double
/// click's time and distance on the same entry asks its double-click class.
/// Class 0 on a container opens its popup; class 2 closes the card and
/// calls the use hook.
#[allow(clippy::too_many_arguments)]
fn panel_double_click(
    real: Res<Time<Real>>,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cursor: Res<UseCursor>,
    game: Option<Res<Game>>,
    mut items: ResMut<DollItems>,
    mut screens: ResMut<GameScreens>,
    mut last: Local<Option<(IVec2, u64, Panel, usize)>>,
    mut hooks: MessageWriter<UseHook>,
    mut containers: MessageWriter<crate::gui_screens::ContainerRequest>,
    mut divination: MessageWriter<crate::div_card_screen::OpenDivination>,
    mut levels: MessageWriter<crate::flow::ReadNamedLevel>,
) {
    if !buttons.just_pressed(MouseButton::Left) || cursor.took_press {
        return;
    }
    let Some(at) = pointer(&windows) else {
        return;
    };
    let now = real.elapsed().as_millis() as u64;
    let panels = open_panels(&screens);
    let Some((panel, local)) = panel_at(&panels, at) else {
        *last = None;
        return;
    };
    let Some(panel_container) = container(&screens, panel) else {
        return;
    };
    let Some(index) = entry_at(panel_container, local) else {
        *last = None;
        return;
    };
    let double = last.is_some_and(|(was, ms, p, i)| {
        p == panel
            && i == index
            && player_controls::is_double_click((was.x, was.y), ms, (at.x, at.y), now)
    });
    if !double {
        *last = Some((at, now, panel, index));
        return;
    }
    *last = None;
    let entry = panel_container.entries[index];
    items.ensure(game.as_deref());
    let operative = operative_of(&items, entry.id).map(|(class, kind, _)| (class, kind));
    let use_class = player_controls::panel_use_class(operative, entry.flags);
    let is_container = items
        .get(entry.id)
        .is_some_and(|item| item.container.is_some());
    // Class 0 of a kind other than the GUI item runs its use before the
    // class is applied. Kind 5 closes the card and opens the divination
    // reading with the zero argument that use passes. Kind 7 asks to load
    // the level its header names.
    if operative.is_some_and(|(class, kind)| class == 0 && kind == player_controls::TAKE_POWERS) {
        let _ = screens.card.close(now as u32);
        divination.write(crate::div_card_screen::OpenDivination { seed: 0 });
    }
    // Kind 7 asks to load the level the header names. The card stays, and a
    // container section still opens afterwards.
    if operative.is_some_and(|(class, kind)| class == 0 && kind == player_controls::TAKE_NAME_HOOK)
    {
        levels.write(crate::flow::ReadNamedLevel { id: entry.id });
    }
    match crate::item_container::double_click(use_class, is_container) {
        crate::item_container::DoubleClick::UseItem => {
            let _ = screens.card.close(now as u32);
            hooks.write(UseHook {
                item: entry.id,
                entry: entry.id,
                slot: index as i32 + 1,
                panel,
            });
        }
        // Class 0 on a container opens its record. The popup is the same one
        // a placed chest uses; this item has no placed object, so the record
        // is the item's own section.
        crate::item_container::DoubleClick::OpenContainer => {
            containers.write(crate::gui_screens::ContainerRequest {
                object: Entity::PLACEHOLDER,
                id: entry.id,
                pointer: at,
            });
        }
        crate::item_container::DoubleClick::CloseCard => {
            let _ = screens.card.close(now as u32);
        }
        crate::item_container::DoubleClick::Nothing => {}
    }
}

/// The debug use: at its frame after the load the item goes into the
/// player's pack record and the use hook is called for it.
fn scripted_use(
    script: Res<UseScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut players: Query<&mut Possessions, With<PlayerBody>>,
    mut hooks: MessageWriter<UseHook>,
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
        info!("use script: no player at frame {frame}");
        return;
    };
    p.carried.push(exa_items::possession::Carried {
        id,
        flags: 0,
        x: 0,
        y: 0,
    });
    info!("use script: the use hook for {id:#x} at frame {frame}");
    hooks.write(UseHook {
        item: id,
        entry: id,
        slot: p.carried.len() as i32,
        panel: Panel::PlayerPack,
    });
}

/// The use hook ([`player_controls::use_begin`]).
fn begin_use(
    mut hooks: MessageReader<UseHook>,
    game: Option<Res<Game>>,
    mut items: ResMut<DollItems>,
    session: Option<Res<PlaySession>>,
    mut cursor: ResMut<UseCursor>,
) {
    for hook in hooks.read() {
        items.ensure(game.as_deref());
        let (class, kind, target) = operative_of(&items, hook.item).unwrap_or((0xff, 0xff, 0));
        let playing = session.as_deref().is_some_and(|s| s.playing);
        match player_controls::use_begin(class, kind, target, playing) {
            UseBegin::OpenGui { item } => {
                info!(
                    "use: {:#x} opens the GUI item {item:#x} (no window for it yet)",
                    hook.item
                );
            }
            UseBegin::Cursor => {
                info!(
                    "use: the cursor is up for {:#x} (class {class}, kind {kind}, slot {})",
                    hook.item, hook.slot
                );
                cursor.lift(hook.item, hook.entry, hook.slot, hook.panel.number(), kind);
            }
            UseBegin::Nothing => info!("use: {:#x} begins nothing, play is off", hook.item),
        }
    }
}

/// What the dispatcher reads under the pointer.
#[derive(bevy::ecs::system::SystemParam)]
struct UseUnder<'w, 's> {
    scripted: Res<'w, ScriptedPointer>,
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    cameras: Query<'w, 's, (&'static Camera, &'static GlobalTransform), With<ChaseCamera>>,
    sources: PickSources<'w>,
    bodies: Query<'w, 's, OtherBodies, Without<PlayerBody>>,
    doors: Query<'w, 's, &'static mut DoorBody>,
    objects: Query<'w, 's, ObjectIds>,
    over: Option<Res<'w, PointerOverGui>>,
    screens: Res<'w, GameScreens>,
}

/// The use cursor's end on the next mouse press (see the module notes).
fn end_use(
    buttons: Res<ButtonInput<MouseButton>>,
    time: Option<Res<Time<Virtual>>>,
    mut cursor: ResMut<UseCursor>,
    mut under: UseUnder,
    mut released: MessageWriter<UseReleased>,
    mut sounds: MessageWriter<MechanismSounded>,
) {
    cursor.took_press = false;
    if !cursor.up() {
        return;
    }
    let pressed = [MouseButton::Left, MouseButton::Right, MouseButton::Middle]
        .into_iter()
        .any(|b| buttons.just_pressed(b));
    if !pressed {
        return;
    }
    let (item, kind) = (cursor.item, cursor.kind);
    let Some(end) = cursor.end(
        player_controls::USE_END_EVENT,
        buttons.just_pressed(MouseButton::Left),
    ) else {
        return;
    };
    info!(
        "use: the cursor ends for {item:#x} (dispatch {}, answer {:#x})",
        end.dispatch, end.answer
    );
    if !end.dispatch {
        return;
    }
    let at = pointer(&under.windows);
    let over_gui = under.over.as_deref().is_some_and(|o| o.0);
    let mut target = None;
    let mut door = DoorLock::None;
    let mut item_id = 0;
    let mut subject = 0;
    let mut subject_entry = None;
    let over = if over_gui {
        let in_view = at.zip(crate::doll_view::open_view(&under.screens));
        if in_view.is_some_and(|(at, view)| view.contains(at.x, at.y)) {
            UseOver::CharacterView
        } else {
            let panels = open_panels(&under.screens);
            let entry = at
                .and_then(|at| panel_at(&panels, at))
                .and_then(|(panel, local)| {
                    let c = container(&under.screens, panel)?;
                    let index = entry_at(c, local)?;
                    subject_entry = Some((panel, index));
                    Some(c.entries[index].id)
                });
            match entry {
                Some(id) => {
                    subject = id;
                    UseOver::ItemWindow {
                        same_item: id == item,
                    }
                }
                None => UseOver::OtherGui,
            }
        }
    } else {
        let ray = crate::hand::cursor_ray(&under.scripted, &under.windows, &under.cameras);
        let picked = crate::hand::pick_with(
            ray,
            &under.sources.physics,
            &under.sources.stand_ins,
            &under.sources.collider_nodes,
            &under.bodies,
            None,
        );
        info!("use: the dispatcher's pick: {:?}", picked.map(|p| p.hit));
        let class = match picked.map(|p| p.hit) {
            Some(crate::hand::Hit::Character { body, .. }) => {
                target = Some(body);
                classification(true, false, false)
            }
            Some(crate::hand::Hit::Prop { body, .. }) => {
                target = Some(body);
                if let Ok(leaf) = under.doors.get(body) {
                    door = door_lock(leaf.door.latched, leaf.door.record.key, item);
                    classification(false, true, false)
                } else {
                    item_id = crate::hand::object_ids(body, &under.objects).0;
                    subject = item_id;
                    classification(false, false, item_id != 0)
                }
            }
            _ => 0,
        };
        UseOver::World(player_controls::use_target(class))
    };
    let release = player_controls::use_release(over, kind, door, item_id);
    info!("use: {item:#x} released over {over:?}: {release:?}");
    if release == UseRelease::UnlockDoor
        && let Some(body) = target
    {
        let now = time.map_or(0, |t| t.elapsed().as_millis() as i32);
        let partner = under.doors.get(body).ok().and_then(|leaf| leaf.partner);
        let unlocked = match partner.and_then(|p| under.doors.get_many_mut([body, p]).ok()) {
            Some([mut leaf, mut other]) => Some((
                leaf.door.unlock(Some(&mut other.door), now),
                leaf.door.record,
                leaf.handle_point(),
            )),
            None => under.doors.get_mut(body).ok().map(|mut leaf| {
                let sound = leaf.door.unlock(None, now);
                (sound, leaf.door.record, leaf.handle_point())
            }),
        };
        // The unlock plays the door's `LkO` at its handle.
        if let Some((sound, record, handle)) = unlocked {
            sounds.write(mechanism_sound::lock_cue(
                &record,
                mechanism_sound::UNLOCKED,
                handle,
                sound,
            ));
        }
    }
    // Another item on a latched door rattles it: its `LkT`.
    if release == UseRelease::RattleDoor
        && let Some(leaf) = target.and_then(|body| under.doors.get(body).ok())
    {
        sounds.write(mechanism_sound::lock_cue(
            &leaf.door.record,
            mechanism_sound::LOCKED,
            leaf.handle_point(),
            mechanism_sound::RATTLE,
        ));
    }
    released.write(UseReleased {
        release,
        item,
        target,
        subject,
        subject_entry,
    });
}

/// A pour the release named, on the picked body or, over the character view,
/// on the player. A salve fills the heal pool and an elixir sets the potion,
/// when the body takes remedies; either kind marks the panel entry it came
/// from. The elixir's pour plays at the body.
#[allow(clippy::too_many_arguments)]
fn apply_pour(
    mut released: MessageReader<UseReleased>,
    mut bodies: Query<(
        &mut exa_combat::Fighter,
        &mut exa_combat::StateFlags,
        &GlobalTransform,
    )>,
    players: Query<Entity, With<PlayerBody>>,
    cursor: Res<UseCursor>,
    game: Option<Res<Game>>,
    mut items: ResMut<DollItems>,
    mut screens: ResMut<GameScreens>,
    clock: Option<Res<exa_combat::remedy::WorldClock>>,
    time: Option<Res<Time<Virtual>>>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
    mut roll: Local<Option<crate::arena::Mt19937>>,
) {
    let now_ms = time.as_ref().map_or(0, |t| t.elapsed().as_millis() as u32);
    let world_s = exa_combat::remedy::world_seconds(clock.as_deref(), now_ms);
    items.ensure(game.as_deref());
    for release in released.read() {
        if release.release != UseRelease::Pour {
            continue;
        }
        let Some(body) = release.target.or_else(|| players.iter().next()) else {
            continue;
        };
        let Ok((mut fighter, mut flags, transform)) = bodies.get_mut(body) else {
            continue;
        };
        let value = items
            .get(cursor.item)
            .and_then(|item| item.operative.as_ref())
            .map_or(0, |operative| operative.value);
        let entry_flags = panel_entry_flags(&screens, cursor.panel, cursor.slot);
        let source = if cursor.slot > 0 {
            exa_combat::remedy::ItemSource::Inventory(cursor.slot as usize)
        } else {
            exa_combat::remedy::ItemSource::Direct
        };
        let applied = fighter.take_remedy(
            &mut flags,
            exa_combat::remedy::ItemUse {
                kind: cursor.kind,
                value,
                flags: entry_flags.unwrap_or(0),
                source,
            },
            world_s,
            || {
                roll.get_or_insert_with(|| crate::arena::Mt19937::new(5489))
                    .unit()
            },
        );
        if let Some(flags) = entry_flags
            && applied.item_flags != flags
        {
            mark_panel_entry(&mut screens, cursor.panel, cursor.slot, applied.item_flags);
        }
        if let Some(pour) = applied.pour {
            sounds.write(exa_audio::PointSounded {
                name: exa_combat::remedy::POUR_SAMPLE.into(),
                position: transform.translation(),
                volume: pour.volume,
                pitch: pour.pitch,
                flags: 0,
            });
        }
    }
}

/// The panel entry a use came from, when the cursor names one.
fn panel_entry_flags(screens: &GameScreens, panel_number: u32, slot: i32) -> Option<u32> {
    if slot <= 0 {
        return None;
    }
    let panel = open_panels(screens)
        .into_iter()
        .find(|panel| panel.0.number() == panel_number)?;
    container(screens, panel.0)?
        .entries
        .get(slot as usize - 1)
        .map(|entry| entry.flags)
}

/// Writes the spent flags back onto that entry.
fn mark_panel_entry(screens: &mut GameScreens, panel_number: u32, slot: i32, flags: u32) {
    let Some(panel) = open_panels(screens)
        .into_iter()
        .find(|panel| panel.0.number() == panel_number)
    else {
        return;
    };
    let Some(container) = crate::panel_live::container_mut(screens, panel.0) else {
        return;
    };
    if let Some(entry) = container.entries.get_mut((slot as usize).wrapping_sub(1)) {
        entry.flags = flags;
    }
}

/// A kind `0x10` use whose entry is the operative's target replaces that
/// entry with the operative's second word, then drops the used entry.
#[allow(clippy::too_many_arguments)]
fn transfer_gui(
    mut released: MessageReader<UseReleased>,
    mut screens: ResMut<GameScreens>,
    game: Option<Res<Game>>,
    mut items: ResMut<DollItems>,
    cursor: Res<UseCursor>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
    players: Query<&GlobalTransform, With<PlayerBody>>,
) {
    items.ensure(game.as_deref());
    for release in released.read() {
        if release.release != UseRelease::TransferGui {
            continue;
        }
        let Some((panel, index)) = release.subject_entry else {
            continue;
        };
        let Some(operative) = items.get(release.item).and_then(|item| item.operative) else {
            continue;
        };
        if operative.kind != 0x10 || release.subject != operative.target {
            continue;
        }
        let replaced = {
            let Some(container) = crate::panel_live::container_mut(&mut screens, panel) else {
                continue;
            };
            let half = container.icon_size >> 1;
            let icon = container
                .entries
                .get(index)
                .map(|entry| entry.icon)
                .unwrap_or(0);
            let icon_size = container.icon_size;
            let client = container.client;
            crate::item_container::replace_entry(
                &mut container.entries,
                &mut container.used,
                index,
                &crate::item_container::Replacement {
                    id: operative.second,
                    icon,
                    flags: 0,
                    bulk: &|id| {
                        catalogue
                            .as_deref()
                            .and_then(|catalogue| catalogue.bulk(id))
                            .map(i32::from)
                            .unwrap_or(0)
                    },
                    half: (half, half),
                    icon_size,
                    client,
                },
            )
        };
        if replaced.is_none() {
            continue;
        }
        if cursor.slot > 0 {
            let source = cursor.slot as usize - 1;
            if let Some(source_panel) = open_panels(&screens)
                .into_iter()
                .find(|open| open.0.number() == cursor.panel)
                .map(|open| open.0)
                && let Some(source_container) =
                    crate::panel_live::container_mut(&mut screens, source_panel)
                && (source_panel != panel || source != index)
            {
                let id = source_container.entries.get(source).map(|entry| entry.id);
                let bulk = id
                    .and_then(|id| {
                        catalogue
                            .as_deref()
                            .and_then(|catalogue| catalogue.bulk(id))
                    })
                    .map(i32::from)
                    .unwrap_or(0);
                let _ = source_container.remove_at(source, bulk);
            }
        }
        if let Some(name) = player_controls::use_sound_name(&operative) {
            let position = players
                .iter()
                .next()
                .map(|transform| transform.translation())
                .unwrap_or(Vec3::ZERO);
            sounds.write(exa_audio::PointSounded {
                name,
                position,
                volume: 0.3,
                pitch: 1.0,
                flags: 0x400,
            });
        }
    }
}

/// The panel entry a use landed on takes the new record id.
fn rewrite_entry(screens: &mut GameScreens, panel: Panel, index: usize, from: u32, to: u32) {
    let Some(container) = crate::panel_live::container_mut(screens, panel) else {
        return;
    };
    if let Some(entry) = container.entries.get_mut(index)
        && entry.id == from
    {
        entry.id = to;
    }
}

/// The dirt and wear bytes a kit reads. `None` is a missing section. An
/// apparel section, even at dirt 0, hides the weapon's dirt: the original
/// reads the body first and does not fall through.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct KitBytes {
    apparel_dirt: Option<u8>,
    weapon_dirt: Option<u8>,
    apparel_wear: Option<u8>,
    weapon_wear: Option<u8>,
    material: u8,
}

fn kit_bytes_from(stored: exa_formats::objdb::StoredCondition) -> KitBytes {
    KitBytes {
        apparel_dirt: stored.apparel_dirt,
        weapon_dirt: stored.weapon_dirt,
        apparel_wear: stored.apparel_wear,
        weapon_wear: stored.weapon_wear,
        material: stored.material,
    }
}

/// The loaded object database. A kit reads and writes the record the item
/// id names, so a local copy keeps its own dirt and wear.
#[derive(Resource, Debug, Default)]
struct LiveObjects {
    db: Option<exa_formats::objdb::ObjectDb>,
    tried: bool,
}

impl LiveObjects {
    fn ensure(&mut self, game: Option<&Game>) {
        if self.tried {
            return;
        }
        self.tried = true;
        let Some(game) = game else {
            return;
        };
        let Some(pack) = game.0.pack(exa_assets::PackKind::Resource) else {
            return;
        };
        let Some(bytes) = pack.get("objdb.rdb") else {
            return;
        };
        let strings = pack.get("objstrings.rdb").unwrap_or_default();
        self.db = exa_formats::objdb::ObjectDb::parse(bytes, strings).ok();
    }

    /// A marked template used on an item becomes its own local record. An
    /// unmarked id, or a database that is not loaded, comes back unchanged.
    fn instantiate(&mut self, id: u32) -> u32 {
        self.db.as_mut().map(|db| db.instantiate(id)).unwrap_or(id)
    }
}

fn kit_bytes(item: &exa_formats::objdb::Item) -> KitBytes {
    KitBytes {
        apparel_dirt: item
            .appearance
            .map(|appearance| appearance.dirt)
            .or(item.has_body.then_some(0)),
        weapon_dirt: item.weapon.as_ref().map(|weapon| weapon.dirt),
        apparel_wear: item.appearance.map(|appearance| appearance.damage),
        weapon_wear: item.weapon.as_ref().map(|weapon| weapon.damage),
        material: item.equip_lock,
    }
}

/// The dirt byte a clean reads: the apparel section's when the body has
/// one, otherwise the weapon's.
fn clean_dirt(bytes: KitBytes) -> Option<u8> {
    bytes.apparel_dirt.or(bytes.weapon_dirt)
}

/// The wear byte a repair reads. A weapon section replaces the apparel's.
fn repair_wear(bytes: KitBytes) -> Option<u8> {
    bytes.weapon_wear.or(bytes.apparel_wear)
}

fn write_kit_byte(bytes: &mut KitBytes, section: u8, kind: u8, value: u8) {
    let apparel = section == player_controls::APPAREL_SECTION;
    if kind == player_controls::CLEAN {
        if apparel {
            bytes.apparel_dirt = Some(value);
        } else {
            bytes.weapon_dirt = Some(value);
        }
    } else if apparel {
        bytes.apparel_wear = Some(value);
    } else {
        bytes.weapon_wear = Some(value);
    }
}

/// Dirt and wear the kits have started changing, by item id. Seeded from
/// the item record the first time a kit lands on it.
#[derive(Resource, Debug, Default)]
struct KitConditions {
    bytes: std::collections::HashMap<u32, KitBytes>,
    work: player_controls::TimedList,
}

/// A cleaning or repair kit the release named. The timed byte is stored on
/// [`KitConditions`] and written down each frame; the sound plays at once.
#[allow(clippy::too_many_arguments)]
fn apply_kit(
    mut released: MessageReader<UseReleased>,
    mut kits: ResMut<KitConditions>,
    mut objects: ResMut<LiveObjects>,
    players: Query<&GlobalTransform, With<PlayerBody>>,
    targets: Query<&GlobalTransform>,
    game: Option<Res<Game>>,
    mut items: ResMut<DollItems>,
    cursor: Res<UseCursor>,
    mut screens: ResMut<GameScreens>,
    time: Option<Res<Time<Virtual>>>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
    mut roll: Local<Option<crate::arena::Mt19937>>,
) {
    let now_ms = time.as_ref().map_or(0, |t| t.elapsed().as_millis() as u32);
    let game = game.as_deref();
    objects.ensure(game);
    items.ensure(game);
    for release in released.read() {
        let kind = match release.release {
            UseRelease::CleanWorld | UseRelease::CleanGui => player_controls::CLEAN,
            UseRelease::RepairWorld | UseRelease::RepairGui => player_controls::REPAIR,
            _ => continue,
        };
        let Some(subject) = (release.subject != 0).then_some(release.subject) else {
            continue;
        };
        let bytes = *kits.bytes.entry(subject).or_insert_with(|| {
            objects
                .db
                .as_ref()
                .and_then(|db| db.stored_condition(subject))
                .map(kit_bytes_from)
                .or_else(|| items.get(subject).map(kit_bytes))
                .unwrap_or_default()
        });
        if kind == player_controls::REPAIR
            && objects
                .db
                .as_ref()
                .and_then(|db| db.stored_condition(subject))
                .is_some_and(|stored| stored.repair_refused)
        {
            continue;
        }
        let at_point = matches!(release.release, UseRelease::CleanWorld);
        let active = match kind {
            player_controls::CLEAN => {
                clean_dirt(bytes).is_some_and(|dirt| dirt > player_controls::CLEAN_TO)
            }
            _ => repair_wear(bytes)
                .is_some_and(|wear| player_controls::repair_timed(wear, bytes.material).is_some()),
        };
        let mut subject = subject;
        if active {
            let copy = objects.instantiate(subject);
            if copy != subject {
                if let Some((panel, index)) = release.subject_entry {
                    rewrite_entry(&mut screens, panel, index, subject, copy);
                }
                if let Some(bytes) = kits.bytes.remove(&subject) {
                    kits.bytes.insert(copy, bytes);
                }
                subject = copy;
            }
        }
        let draw = if active {
            roll.get_or_insert_with(|| crate::arena::Mt19937::new(5489))
                .unit() as f32
        } else {
            0.0
        };
        let use_ = player_controls::use_on(
            kind,
            0,
            false,
            0,
            repair_wear(bytes),
            clean_dirt(bytes),
            bytes.material,
            now_ms,
            draw,
            at_point,
        );
        if let Some(timed) = use_.outcome.timed() {
            kits.work.push(player_controls::TimedItem {
                timed,
                item: subject,
            });
        }
        if use_.marks != 0
            && let Some(flags) = panel_entry_flags(&screens, cursor.panel, cursor.slot)
        {
            mark_panel_entry(&mut screens, cursor.panel, cursor.slot, flags | use_.marks);
        }
        if let Some(sound) = use_.sound {
            let name = match sound.kind {
                player_controls::UseSoundKind::Clean => "CleanItem01",
                player_controls::UseSoundKind::Repair => "ReprSphereA1",
                player_controls::UseSoundKind::Pour => continue,
            };
            let position = if at_point {
                release
                    .target
                    .and_then(|entity| targets.get(entity).ok())
                    .map(|transform| transform.translation())
            } else {
                players
                    .iter()
                    .next()
                    .map(|transform| transform.translation())
            };
            sounds.write(exa_audio::PointSounded {
                name: name.into(),
                position: position.unwrap_or(Vec3::ZERO),
                volume: sound.volume,
                pitch: sound.pitch,
                flags: 0,
            });
        }
    }
}

/// One frame of the kits' timed bytes. A value is written only while it
/// lowers the section's byte or arrives, apparel first.
fn tick_kits(
    time: Option<Res<Time<Virtual>>>,
    mut kits: ResMut<KitConditions>,
    mut objects: ResMut<LiveObjects>,
) {
    let now_ms = time.as_ref().map_or(0, |t| t.elapsed().as_millis() as u32);
    let writes = settle_kits(&mut kits, now_ms);
    if let Some(db) = objects.db.as_mut() {
        for (id, section, kind, value) in writes {
            let apparel = section == player_controls::APPAREL_SECTION;
            let dirt = kind == player_controls::CLEAN;
            db.write_condition_byte(id, apparel, dirt, value);
        }
    }
}

fn settle_kits(kits: &mut KitConditions, now_ms: u32) -> Vec<(u32, u8, u8, u8)> {
    let KitConditions { bytes, work } = kits;
    let writes = work.tick_kind(now_ms, |id, section, kind| {
        bytes.get(&id).and_then(|bytes| match (section, kind) {
            (player_controls::APPAREL_SECTION, player_controls::CLEAN) => bytes.apparel_dirt,
            (player_controls::WEAPON_SECTION, player_controls::CLEAN) => bytes.weapon_dirt,
            (player_controls::APPAREL_SECTION, _) => bytes.apparel_wear,
            _ => bytes.weapon_wear,
        })
    });
    for &(id, section, kind, value) in &writes {
        if let Some(bytes) = bytes.get_mut(&id) {
            write_kit_byte(bytes, section, kind, value);
        }
    }
    writes
}

/// The use cursor's end, the panels' double click and the hook: the
/// world's and the panels' own press paths run after it.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UseCursorSystems;

/// The use cursor.
pub struct UseCursorPlugin;

impl Plugin for UseCursorPlugin {
    fn build(&self, app: &mut App) {
        mechanism_sound::register(app);
        app.init_resource::<UseCursor>()
            .init_resource::<UseScript>()
            .init_resource::<DollItems>()
            .init_resource::<KitConditions>()
            .init_resource::<LiveObjects>()
            .add_message::<UseHook>()
            .add_message::<UseReleased>()
            .add_systems(
                Update,
                (
                    end_use,
                    apply_pour,
                    apply_kit,
                    transfer_gui,
                    tick_kits,
                    panel_double_click,
                    scripted_use,
                    begin_use,
                )
                    .chain()
                    .in_set(UseCursorSystems)
                    .after(crate::panel_live::track_pointer)
                    .before(crate::panel_live::PanelInput)
                    .before(crate::hand::HandSystems),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_controls::UseTarget;

    #[test]
    fn the_dispatcher_classifies_its_pick_in_order() {
        assert_eq!(classification(false, true, true), 4);
        assert_eq!(classification(true, false, false), 0x10);
        assert_eq!(classification(false, false, true), 3);
        assert_eq!(classification(false, false, false), 0);
        assert_eq!(
            player_controls::use_target(classification(false, true, false)),
            UseTarget::Door
        );
        assert_eq!(player_controls::use_target(3), UseTarget::Item);
        assert_eq!(player_controls::use_target(0), UseTarget::Nothing);
    }

    #[test]
    fn a_latched_door_takes_its_own_key() {
        assert_eq!(
            door_lock(true, 0xb, 0xb),
            DoorLock::Locked { key_matches: true }
        );
        assert_eq!(
            door_lock(true, 0xb, 0x5a),
            DoorLock::Locked { key_matches: false }
        );
        assert_eq!(door_lock(false, 0xb, 0xb), DoorLock::Open);
    }

    #[test]
    fn the_use_script_reads_its_item_and_frame() {
        assert_eq!(
            UseScript::parse("0xb@200"),
            Some(UseScript(Some((0xb, 200))))
        );
        assert_eq!(UseScript::parse("0xb"), None);
    }

    fn app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(PlaySession {
                playing: true,
                ..PlaySession::default()
            })
            .init_resource::<UseCursor>()
            .init_resource::<DollItems>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_message::<UseHook>()
            .add_systems(Update, begin_use);
        app
    }

    /// The hook lifts the cursor while play is on (an item with no record
    /// reads as no GUI item) and begins nothing while it is off.
    #[test]
    fn the_use_hook_lifts_the_cursor_while_playing() {
        let mut app = app();
        app.world_mut().write_message(UseHook {
            item: 0xb,
            entry: 0xb,
            slot: 2,
            panel: Panel::PlayerPack,
        });
        app.update();
        let cursor = *app.world().resource::<UseCursor>();
        assert!(cursor.up());
        assert_eq!((cursor.item, cursor.slot), (0xb, 2));
        assert_eq!(cursor.cursor_mode, player_controls::USE_CURSOR_MODE);

        let mut app = self::app();
        app.world_mut().resource_mut::<PlaySession>().playing = false;
        app.world_mut().write_message(UseHook {
            item: 0xb,
            entry: 0xb,
            slot: 2,
            panel: Panel::PlayerPack,
        });
        app.update();
        assert!(!app.world().resource::<UseCursor>().up());
    }

    /// A salve poured on a body that takes remedies adds 0.2 to its heal pool.
    #[test]
    fn a_salve_fills_the_bodys_heal_pool() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<UseCursor>()
            .init_resource::<DollItems>()
            .init_resource::<GameScreens>()
            .add_message::<UseReleased>()
            .add_message::<exa_audio::PointSounded>()
            .add_systems(Update, apply_pour);
        let mut fighter = exa_combat::Fighter::default();
        fighter.stats.rules = exa_formats::character_stats::BodyRules(
            exa_formats::character_stats::BodyRules::TAKES_REMEDIES,
        );
        let body = app
            .world_mut()
            .spawn((
                fighter,
                exa_combat::StateFlags::default(),
                GlobalTransform::default(),
            ))
            .id();
        app.world_mut().resource_mut::<UseCursor>().kind = player_controls::POUR;
        app.world_mut().write_message(UseReleased {
            release: UseRelease::Pour,
            item: 1,
            target: Some(body),
            subject: 0,
            subject_entry: None,
        });
        app.update();
        let pool = app
            .world()
            .get::<exa_combat::Fighter>(body)
            .unwrap()
            .health
            .heal_pool;
        assert_eq!(pool, 0.2);
    }

    /// A clean starts at the apparel dirt and, once the two seconds are
    /// spent, leaves it at 0x20. The weapon byte is written on the same pass
    /// because the apparel write happened.
    #[test]
    fn a_clean_lowers_dirty_apparel_to_0x20() {
        let mut kits = KitConditions::default();
        kits.bytes.insert(
            7,
            KitBytes {
                apparel_dirt: Some(0x80),
                weapon_dirt: Some(0x80),
                ..KitBytes::default()
            },
        );
        kits.work.push(player_controls::TimedItem {
            timed: player_controls::Timed {
                kind: player_controls::CLEAN,
                from: 0x80,
                to: player_controls::CLEAN_TO,
                started_ms: 0,
            },
            item: 7,
        });
        settle_kits(&mut kits, 0);
        assert_eq!(kits.bytes[&7].apparel_dirt, Some(0x80));
        settle_kits(&mut kits, 2_000);
        assert_eq!(kits.bytes[&7].apparel_dirt, Some(player_controls::CLEAN_TO));
        assert_eq!(kits.bytes[&7].weapon_dirt, Some(player_controls::CLEAN_TO));
        assert!(kits.work.entries.is_empty());
    }
}

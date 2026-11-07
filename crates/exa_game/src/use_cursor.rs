//! The use cursor: an item's use begun from the item panels and ended by
//! the next mouse press.
//!
//! **The begin.** The item panel's double click on an entry asks the item's
//! double-click class ([`player_controls::panel_use_class`]). Class 0 on an
//! item that has a container section opens that section in the container
//! popup. Class 2 closes the card and calls the game's use hook with the
//! item, its entry word, its slot (from one) and the panel. The controls
//! install that hook at start-up: [`player_controls::use_begin`] lifts the
//! GUI item a class-0 kind-0x11 section's second word names onto the panel
//! drag when its icon reserves, or, while
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
use exa_assets::live_objects::LiveObjects;
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

/// A panel drag ended. `accepted` is a drop onto a panel; anything else,
/// including a release outside every panel, is a cancel.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiDragEnded {
    pub accepted: bool,
    pub id: u32,
}

/// The kind-17 lift: the use item, the slot it came from (from one) and its
/// panel. `panel` is `None` when no such lift is up.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GuiItemLift {
    /// The kind-17 item whose operative the accept reads.
    pub item: u32,
    /// The second word, which is the id on the drag.
    pub dragged: u32,
    pub slot: i32,
    pub panel: Option<Panel>,
}

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

/// `--kit-use clean|repair:TARGET@FRAME,...` (debug): at each frame after
/// the load, a cleaning brush or repair sphere is released on the pack
/// entry that names `TARGET`, as a GUI use of the kit over the inventory
/// would release it ([`UseRelease::CleanGui`] / [`UseRelease::RepairGui`]).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct KitScript(pub Vec<KitUse>);

/// One scripted kit release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KitUse {
    pub repair: bool,
    pub target: u32,
    pub frame: u32,
}

impl KitScript {
    pub fn parse(text: &str) -> Option<Self> {
        text.split(',')
            .map(|one| {
                let (kind, rest) = one.split_once(':')?;
                let repair = match kind.trim() {
                    "repair" => true,
                    "clean" => false,
                    _ => return None,
                };
                let (target, frame) = rest.split_once('@')?;
                Some(KitUse {
                    repair,
                    target: crate::gui_drop::parse_id(target)?,
                    frame: frame.trim().parse().ok()?,
                })
            })
            .collect::<Option<Vec<_>>>()
            .map(Self)
    }
}

/// The scripted kit releases ([`KitScript`]): the shipped brush (`0x34d`)
/// or repair sphere (`0x48c`) on the player's pack entry naming the target.
fn scripted_kit(
    script: Res<KitScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    players: Query<&Possessions, With<PlayerBody>>,
    mut released: MessageWriter<UseReleased>,
) {
    let Some(last) = script.0.iter().map(|kit| kit.frame).max() else {
        return;
    };
    if loading.is_some() || *frames > last {
        return;
    }
    *frames += 1;
    for kit in script.0.iter().filter(|kit| kit.frame == *frames) {
        let Some(pack) = players.iter().next() else {
            info!("kit script: no player at frame {}", kit.frame);
            continue;
        };
        let entry = pack
            .carried
            .iter()
            .filter(|carried| carried.id != 0)
            .position(|carried| carried.id == kit.target);
        let Some(entry) = entry else {
            info!("kit script: the pack holds no {:#x}", kit.target);
            continue;
        };
        info!(
            "kit script: {} on pack entry {entry} ({:#x}) at frame {}",
            if kit.repair { "repair" } else { "clean" },
            kit.target,
            kit.frame
        );
        released.write(UseReleased {
            release: if kit.repair {
                UseRelease::RepairGui
            } else {
                UseRelease::CleanGui
            },
            item: if kit.repair { 0x48c } else { 0x34d },
            target: None,
            subject: kit.target,
            subject_entry: Some((Panel::PlayerPack, entry)),
        });
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
        .map(|o| (o.class, o.kind, o.second))
}

/// The pointer in physical window pixels.
fn pointer(windows: &Query<&Window, With<PrimaryWindow>>) -> Option<IVec2> {
    let window = windows.single().ok()?;
    window
        .cursor_position()
        .map(|p| (p * window.scale_factor()).as_ivec2())
}

/// What a panel entry's double click writes: its use hook, a container
/// popup, the divination reading and the named level's load.
#[derive(bevy::ecs::system::SystemParam)]
struct DoubleClickOut<'w> {
    hooks: MessageWriter<'w, UseHook>,
    containers: MessageWriter<'w, crate::gui_screens::ContainerRequest>,
    divination: MessageWriter<'w, crate::div_card_screen::OpenDivination>,
    levels: MessageWriter<'w, crate::flow::ReadNamedLevel>,
}

/// `--double-click ID@FRAME` (debug): at that frame after the load the item
/// is put into the player's pack and its entry's double click is run, as a
/// second left press on it in the inventory would.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DoubleClickScript(pub Option<(u32, u32)>);

impl DoubleClickScript {
    pub fn parse(text: &str) -> Option<Self> {
        UseScript::parse(text).map(|s| Self(s.0))
    }
}

/// The scripted double click's entry, once the item is in the pack: its id
/// and its slot from one.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
struct PendingDoubleClick(Option<(u32, i32)>);

/// Puts the scripted item into the pack and asks for its double click.
fn scripted_double_click(
    script: Res<DoubleClickScript>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut players: Query<&mut Possessions, With<PlayerBody>>,
    mut pending: ResMut<PendingDoubleClick>,
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
        info!("double click script: no player at frame {frame}");
        return;
    };
    p.carried.push(exa_items::possession::Carried {
        id,
        flags: 0,
        x: 0,
        y: 0,
    });
    info!("double click script: {id:#x} double clicked in the pack at frame {frame}");
    pending.0 = Some((id, p.carried.len() as i32));
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
    mut pending: ResMut<PendingDoubleClick>,
    mut out: DoubleClickOut,
) {
    let now = real.elapsed().as_millis() as u64;
    if let Some((id, slot)) = pending.0.take() {
        let at = pointer(&windows).unwrap_or_default();
        run_double_click(
            &mut items,
            game.as_deref(),
            &mut screens,
            &mut out,
            (id, 0),
            slot,
            Panel::PlayerPack,
            at,
            now,
        );
    }
    if !buttons.just_pressed(MouseButton::Left) || cursor.took_press {
        return;
    }
    let Some(at) = pointer(&windows) else {
        return;
    };
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
    run_double_click(
        &mut items,
        game.as_deref(),
        &mut screens,
        &mut out,
        (entry.id, entry.flags),
        index as i32 + 1,
        panel,
        at,
        now,
    );
}

/// An entry's double click: the item's use class picks what it does, after
/// a class-0 use of kind 5 or 7 has run.
#[allow(clippy::too_many_arguments)]
fn run_double_click(
    items: &mut DollItems,
    game: Option<&Game>,
    screens: &mut GameScreens,
    out: &mut DoubleClickOut,
    (id, flags): (u32, u32),
    slot: i32,
    panel: Panel,
    at: IVec2,
    now: u64,
) {
    items.ensure(game);
    let operative = operative_of(items, id).map(|(class, kind, _)| (class, kind));
    let use_class = player_controls::panel_use_class(operative, flags);
    let is_container = items.get(id).is_some_and(|item| item.container.is_some());
    info!("double click: {id:#x} use {operative:?}, class {use_class}");
    // Class 0 of a kind other than the GUI item runs its use before the
    // class is applied. Kind 5 closes the card and opens the divination
    // reading with the zero argument that use passes. Kind 7 asks to load
    // the level its header names.
    if operative.is_some_and(|(class, kind)| class == 0 && kind == player_controls::TAKE_DIVINATION)
    {
        let _ = screens.card.close(now as u32);
        info!("double click: {id:#x} opens the divination reading");
        out.divination
            .write(crate::div_card_screen::OpenDivination { seed: 0 });
    }
    // Kind 7 asks to load the level the header names. The card stays, and a
    // container section still opens afterwards.
    if operative.is_some_and(|(class, kind)| class == 0 && kind == player_controls::TAKE_NAME_HOOK)
    {
        out.levels.write(crate::flow::ReadNamedLevel { id });
    }
    match crate::item_container::double_click(use_class, is_container) {
        crate::item_container::DoubleClick::UseItem => {
            let _ = screens.card.close(now as u32);
            out.hooks.write(UseHook {
                item: id,
                entry: id,
                slot,
                panel,
            });
        }
        // Class 0 on a container opens its record. The popup is the same one
        // a placed chest uses; this item has no placed object, so the record
        // is the item's own section.
        crate::item_container::DoubleClick::OpenContainer => {
            out.containers.write(crate::gui_screens::ContainerRequest {
                object: Entity::PLACEHOLDER,
                id,
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
#[allow(clippy::too_many_arguments)]
fn begin_use(
    mut hooks: MessageReader<UseHook>,
    game: Option<Res<Game>>,
    mut items: ResMut<DollItems>,
    session: Option<Res<PlaySession>>,
    mut cursor: ResMut<UseCursor>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut icons: Option<ResMut<crate::item_panel_draw::PanelIcons>>,
    mut drag: Option<ResMut<crate::panel_live::PanelDrag>>,
    mut lift: Option<ResMut<GuiItemLift>>,
) {
    for hook in hooks.read() {
        items.ensure(game.as_deref());
        let (class, kind, second) = operative_of(&items, hook.item).unwrap_or((0xff, 0xff, 0));
        let playing = session.as_deref().is_some_and(|s| s.playing);
        match player_controls::use_begin(class, kind, second, playing) {
            UseBegin::OpenGui { item } => {
                let Some(icons) = icons.as_deref_mut() else {
                    if let Some(lift) = lift.as_deref_mut() {
                        lift.panel = None;
                    }
                    info!("use: {item:#x} has no icon manager, so it is not lifted");
                    continue;
                };
                let Some(drag) = drag.as_deref_mut() else {
                    info!("use: {item:#x} has no drag, so it is not lifted");
                    continue;
                };
                let (handle, half) = icons.reserve(item, 0, catalogue.as_deref());
                if handle == 0 {
                    info!("use: {item:#x} did not reserve an icon, so it is not lifted");
                    continue;
                }
                if drag.icon != 0 && drag.icon != handle {
                    icons.release(drag.icon);
                }
                let bulk = catalogue
                    .as_deref()
                    .and_then(|catalogue| catalogue.bulk(item))
                    .map(i32::from)
                    .unwrap_or(0);
                *drag = crate::panel_live::PanelDrag {
                    source: Some(hook.panel),
                    icon: handle,
                    grab: crate::item_container::Cell::new(half.0, half.1),
                    id: item,
                    flags: 0,
                    bulk,
                };
                if let Some(lift) = lift.as_deref_mut() {
                    *lift = GuiItemLift {
                        item: hook.item,
                        dragged: item,
                        slot: hook.slot,
                        panel: Some(hook.panel),
                    };
                }
                info!("use: {:#x} lifts the GUI item {item:#x}", hook.item);
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
    fixed: ResMut<'w, exa_physics::sim_app::FixedCommands>,
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
        // The unlock is a write to the door's simulation data: posted for
        // the next step ([`exa_physics::sim_app::FixedCommands::write`]).
        let fixed = &mut under.fixed;
        let unlocked = match partner.and_then(|p| {
            under
                .doors
                .get_many_mut([body, p])
                .ok()
                .map(|leaves| (p, leaves))
        }) {
            Some((p, mut leaves)) => {
                let sound = fixed.write_many([body, p], &mut leaves, move |[leaf, other]| {
                    leaf.door.unlock(Some(&mut other.door), now)
                });
                let [leaf, _] = leaves;
                Some((sound, leaf.door.record, leaf.handle_point()))
            }
            None => under.doors.get_mut(body).ok().map(|mut leaf| {
                let sound = fixed.write(body, &mut leaf, move |leaf| leaf.door.unlock(None, now));
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
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut icons: Option<ResMut<crate::item_panel_draw::PanelIcons>>,
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
            if let Some(icons) = icons.as_deref_mut() {
                refresh_spent_icon(
                    &mut screens,
                    icons,
                    catalogue.as_deref(),
                    cursor.panel,
                    cursor.slot,
                    cursor.item,
                    applied.item_flags,
                );
            }
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
/// The spent mark changes the entry's icon. The new slot is reserved with
/// the marked flags; the old slot is released when it differs.
fn refresh_spent_icon(
    screens: &mut GameScreens,
    icons: &mut crate::item_panel_draw::PanelIcons,
    catalogue: Option<&exa_items::catalogue::ItemCatalogue>,
    panel_number: u32,
    slot: i32,
    item: u32,
    flags: u32,
) {
    if slot <= 0 {
        return;
    }
    let (handle, _) = icons.reserve(item, flags, catalogue);
    if handle == 0 {
        return;
    }
    let Some(panel) = open_panels(screens)
        .into_iter()
        .find(|panel| panel.0.number() == panel_number)
        .map(|panel| panel.0)
    else {
        icons.release(handle);
        return;
    };
    let Some(container) = crate::panel_live::container_mut(screens, panel) else {
        icons.release(handle);
        return;
    };
    let Some(entry) = container.entries.get_mut(slot as usize - 1) else {
        icons.release(handle);
        return;
    };
    let old = entry.icon;
    entry.icon = handle;
    if old != 0 && old != handle {
        icons.release(old);
    }
}

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
    mut icons: Option<ResMut<crate::item_panel_draw::PanelIcons>>,
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
        // The original acquires the replacement's icon first and refuses the
        // replace when that acquire fails, including when there is no icon
        // manager to acquire from.
        let Some((icon, half)) = icons
            .as_deref_mut()
            .map(|icons| icons.reserve(operative.second, 0, catalogue.as_deref()))
        else {
            continue;
        };
        if icon == 0 {
            continue;
        }
        let reserved = Some((icon, half));
        let replaced = {
            let Some(container) = crate::panel_live::container_mut(&mut screens, panel) else {
                if icon != 0
                    && let Some(icons) = icons.as_deref_mut()
                {
                    icons.release(icon);
                }
                continue;
            };
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
                    half,
                    icon_size,
                    client,
                },
            )
        };
        let Some(old_icon) = replaced else {
            if let Some((handle, _)) = reserved
                && handle != 0
                && let Some(icons) = icons.as_deref_mut()
            {
                icons.release(handle);
            }
            continue;
        };
        if let Some((handle, _)) = reserved
            && old_icon != 0
            && old_icon != handle
            && let Some(icons) = icons.as_deref_mut()
        {
            icons.release(old_icon);
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
                if let Some(entry) = source_container.remove_at(source, bulk)
                    && entry.icon != 0
                    && let Some(icons) = icons.as_deref_mut()
                {
                    icons.release(entry.icon);
                }
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

/// A kit's copy `to` of the item `from` takes the item's place in the
/// player's pack: the pack entry the use landed on when it still names
/// `from` (the panel lists the pack's non-empty entries in order), else the
/// first entry naming it. The original's pack panel is the record's own
/// list, so its rewrite is the record's.
fn rewrite_carried(possessions: &mut Possessions, entry: Option<usize>, from: u32, to: u32) {
    let at = entry
        .and_then(|index| {
            possessions
                .carried
                .iter()
                .enumerate()
                .filter(|(_, carried)| carried.id != 0)
                .nth(index)
                .filter(|(_, carried)| carried.id == from)
                .map(|(at, _)| at)
        })
        .or_else(|| possessions.carried.iter().position(|c| c.id == from));
    if let Some(at) = at {
        possessions.carried[at].id = to;
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
    mut live: ResMut<LiveObjects>,
    players: Query<&GlobalTransform, With<PlayerBody>>,
    mut packs: Query<&mut Possessions, With<PlayerBody>>,
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
    if let Some(game) = game {
        live.ensure_base(&game.0);
    }
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
            live.db()
                .stored_condition(subject)
                .map(kit_bytes_from)
                .or_else(|| items.get(subject).map(kit_bytes))
                .unwrap_or_default()
        });
        if kind == player_controls::REPAIR
            && live
                .db()
                .stored_condition(subject)
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
            // A marked template used on becomes its own local record, under
            // the first id the level's local tier leaves free.
            let copy = live.db_mut().instantiate(subject);
            if copy != subject {
                if let Some((panel, index)) = release.subject_entry {
                    rewrite_entry(&mut screens, panel, index, subject, copy);
                }
                if let Some(bytes) = kits.bytes.remove(&subject) {
                    kits.bytes.insert(copy, bytes);
                }
                let entry = release
                    .subject_entry
                    .filter(|(panel, _)| *panel == Panel::PlayerPack)
                    .map(|(_, index)| index);
                if let Some(mut pack) = packs.iter_mut().next() {
                    rewrite_carried(&mut pack, entry, subject, copy);
                }
                // The panels, the card and the worn figure read the copy
                // under its own id from the next sync on.
                items.sync(&live);
                info!("kit: item {subject:#x} becomes its own record {copy:#x}");
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
/// lowers the section's byte or arrives, apparel first. Each write goes
/// into the live database's record; the item records the panels and the
/// worn figure read ([`DollItems`]) and the card's sentence (which reads
/// the wear byte) are read again from it.
#[allow(clippy::too_many_arguments)]
fn tick_kits(
    time: Option<Res<Time<Virtual>>>,
    mut kits: ResMut<KitConditions>,
    mut live: ResMut<LiveObjects>,
    mut items: ResMut<DollItems>,
    game: Option<Res<Game>>,
    mut factories: Option<ResMut<crate::shop_stock::ShopFactories>>,
    mut refreshed: MessageWriter<crate::item_card::CardRefresh>,
) {
    let now_ms = time.as_ref().map_or(0, |t| t.elapsed().as_millis() as u32);
    let writes = settle_kits(&mut kits, now_ms);
    if writes.is_empty() {
        return;
    }
    for (id, section, kind, value) in writes {
        let apparel = section == player_controls::APPAREL_SECTION;
        let dirt = kind == player_controls::CLEAN;
        let written = live.write_condition(id, apparel, dirt, value);
        debug!(
            "kit write: item {id:#x} {} {} = {value:#x} (stored {written})",
            if apparel { "apparel" } else { "weapon" },
            if dirt { "dirt" } else { "wear" }
        );
        // A repair's write brings the copy up to its template at its new
        // wear, then refreshes an open card of the item.
        if written && !dirt {
            let copied = repair_record(
                &mut live,
                id,
                apparel,
                game.as_deref(),
                factories.as_deref_mut(),
            );
            debug!("kit repair: item {id:#x} copied from its template: {copied}");
            refreshed.write(crate::item_card::CardRefresh { item: id });
        }
    }
    items.sync(&live);
}

/// The repair's copy pass over the live record `id` names
/// ([`exa_items::refresh::repair_copy`]): only a copy (an id with a tier
/// tag) whose template, its header's parent word, resolves. A weapon's
/// factory stat pass reads `Factories.rpk` through the shop's factory
/// cache. Returns whether the record was rewritten.
fn repair_record(
    live: &mut LiveObjects,
    id: u32,
    apparel: bool,
    game: Option<&Game>,
    factories: Option<&mut crate::shop_stock::ShopFactories>,
) -> bool {
    use exa_items::refresh::{Repaired, repair_copy};
    if id & 0xF000_0000 == 0 {
        return false;
    }
    let db = live.db();
    let Some(record) = db.find(id) else {
        return false;
    };
    let parent = record.parent_word();
    let Some(template) = db.find(parent) else {
        debug!("kit repair: item {id:#x} names template {parent:#x}, which is not there");
        return false;
    };
    let template = template.payload().to_vec();
    let mut payload = record.payload().to_vec();
    let repaired = if apparel {
        Repaired::Apparel
    } else {
        Repaired::Weapon
    };
    let copied = repair_copy(&mut payload, &template, repaired, |name, body| {
        if let Some(factories) = factories {
            factories.pass(game, name, body);
        }
    });
    debug!("kit repair: item {id:#x} from template {parent:#x}: copy pass {copied}");
    if payload.as_slice() == live.db().find(id).map_or(&[][..], |r| r.payload()) {
        return false;
    }
    live.set_payload(id, &payload)
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
        exa_physics::sim_app::add_fixed_commands(app);
        mechanism_sound::register(app);
        app.init_resource::<UseCursor>()
            .init_resource::<UseScript>()
            .init_resource::<DoubleClickScript>()
            .init_resource::<PendingDoubleClick>()
            .init_resource::<KitScript>()
            .init_resource::<DollItems>()
            .init_resource::<KitConditions>()
            .init_resource::<crate::shop_stock::ShopFactories>()
            .add_message::<crate::item_card::CardRefresh>()
            .init_resource::<LiveObjects>()
            .init_resource::<GuiItemLift>()
            .add_message::<UseHook>()
            .add_message::<UseReleased>()
            .add_systems(
                Update,
                (
                    end_use,
                    apply_pour,
                    scripted_kit,
                    apply_kit,
                    transfer_gui,
                    tick_kits,
                    scripted_double_click,
                    panel_double_click,
                    scripted_use,
                    begin_use,
                )
                    .chain()
                    .in_set(UseCursorSystems)
                    .after(crate::panel_live::track_pointer)
                    .before(crate::panel_live::PanelInput)
                    .before(crate::hand::HandSystems),
            )
            .add_systems(Update, settle_gui_lift.after(crate::panel_live::PanelInput));
    }
}

/// An accepted drop of a kind-17 lift replaces the slot it came from with
/// the operative's target, and plays the operative's sound. A cancel leaves
/// that slot alone.
#[allow(clippy::too_many_arguments)]
fn settle_gui_lift(
    mut ended: MessageReader<GuiDragEnded>,
    mut lift: ResMut<GuiItemLift>,
    mut screens: ResMut<GameScreens>,
    items: Res<DollItems>,
    mut icons: ResMut<crate::item_panel_draw::PanelIcons>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut sounds: MessageWriter<exa_audio::PointSounded>,
    players: Query<&GlobalTransform, With<PlayerBody>>,
) {
    for end in ended.read() {
        let Some(panel) = lift.panel else {
            continue;
        };
        if end.id != lift.dragged {
            continue;
        }
        let item = lift.item;
        let slot = lift.slot;
        lift.panel = None;
        if !end.accepted || slot <= 0 {
            continue;
        }
        let Some(operative) = items.get(item).and_then(|item| item.operative) else {
            continue;
        };
        if operative.class != 0 || operative.kind != player_controls::GUI_ITEM_KIND {
            continue;
        }
        let (icon, half) = icons.reserve(operative.target, 0, catalogue.as_deref());
        if icon == 0 {
            continue;
        }
        let index = slot as usize - 1;
        let replaced = {
            let Some(container) = crate::panel_live::container_mut(&mut screens, panel) else {
                icons.release(icon);
                continue;
            };
            let icon_size = container.icon_size;
            let client = container.client;
            crate::item_container::replace_entry(
                &mut container.entries,
                &mut container.used,
                index,
                &crate::item_container::Replacement {
                    id: operative.target,
                    icon,
                    flags: 0,
                    bulk: &|id| {
                        catalogue
                            .as_deref()
                            .and_then(|catalogue| catalogue.bulk(id))
                            .map(i32::from)
                            .unwrap_or(0)
                    },
                    half,
                    icon_size,
                    client,
                },
            )
        };
        let Some(old) = replaced else {
            icons.release(icon);
            continue;
        };
        if old != 0 && old != icon {
            icons.release(old);
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

    /// A kit's copy takes its template's place in the pack: the entry the
    /// use landed on (counting non-empty entries), else the first naming it.
    #[test]
    fn a_kit_copy_replaces_its_pack_entry() {
        let mut pack = Possessions::default();
        for id in [0, 7, 9, 7] {
            pack.carried.push(exa_items::possession::Carried {
                id,
                flags: 0,
                x: 0,
                y: 0,
            });
        }
        rewrite_carried(&mut pack, Some(2), 7, 0x8000_0010);
        let ids: Vec<u32> = pack.carried.iter().map(|c| c.id).collect();
        assert_eq!(ids, [0, 7, 9, 0x8000_0010]);
        rewrite_carried(&mut pack, Some(1), 7, 0x8000_0011);
        let ids: Vec<u32> = pack.carried.iter().map(|c| c.id).collect();
        assert_eq!(ids, [0, 0x8000_0011, 9, 0x8000_0010]);
        rewrite_carried(&mut pack, None, 9, 0x8000_0012);
        assert_eq!(pack.carried[2].id, 0x8000_0012);
        rewrite_carried(&mut pack, None, 5, 1);
        assert_eq!(pack.carried.len(), 4);
    }

    #[test]
    fn the_kit_script_reads_its_kind_target_and_frame() {
        assert_eq!(
            KitScript::parse("repair:0x8000003@120"),
            Some(KitScript(vec![KitUse {
                repair: true,
                target: 0x8000003,
                frame: 120
            }]))
        );
        let two = KitScript::parse("clean:12@5,repair:1@9").unwrap();
        assert_eq!(two.0.len(), 2);
        assert!(!two.0[0].repair && two.0[1].repair);
        assert_eq!(KitScript::parse("polish:12@5"), None);
        assert_eq!(KitScript::parse("clean:12"), None);
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

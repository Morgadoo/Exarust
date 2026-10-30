//! The use cursor: an item's use begun from the item panels and ended by
//! the next mouse press.
//!
//! **The begin.** The item panel's double click on an entry asks the item's
//! double-click class ([`player_controls::panel_use_class`]); class 2
//! closes the card and calls the game's use hook with the item, its entry
//! word, its slot (from one) and the panel. The controls install that hook
//! at start-up: [`player_controls::use_begin`] opens the GUI item a class-0
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
//! ([`player_controls::use_release`]). Exarust applies the unlock; the other
//! branches are reported in [`UseReleased`] for their own consumers.
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
use crate::item_container::PanelEvent;
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
    pub item: u32,
    pub target: Option<Entity>,
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
/// click's time and distance on the same entry asks its double-click class;
/// class 2 closes the card and calls the use hook.
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
    let event = crate::item_container::panel_event(
        0x8001,
        Some(index),
        true,
        false,
        use_class,
        None,
        panel_container.icon_size,
        false,
    );
    if event == PanelEvent::UseItem {
        let _ = screens.card.close(now as u32);
        hooks.write(UseHook {
            item: entry.id,
            entry: entry.id,
            slot: index as i32 + 1,
            panel,
        });
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
                    entry_at(c, local).map(|i| c.entries[i].id)
                });
            match entry {
                Some(id) => UseOver::ItemWindow {
                    same_item: id == item,
                },
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
    });
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
            .add_message::<UseHook>()
            .add_message::<UseReleased>()
            .add_systems(
                Update,
                (end_use, panel_double_click, scripted_use, begin_use)
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
}

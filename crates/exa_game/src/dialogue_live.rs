//! The dialogue in play: whom the Talk key talks to, the reply panel's
//! pointer, and the paint of the reply panel and the speech bubbles.
//!
//! **Whom Talk talks to.** The Talk key's talk routine asks for its target
//! first and does nothing without one. The target is one of the player's
//! nearby characters (the list the player behaviour gathers every tick: the
//! characters of the player's room and of the rooms its exits lead to, seen,
//! within 30 m, the sixteen nearest; see [`exa_ai::contacts::nearby`]). Of
//! those not collapsing or dead (condition bit 0x800), the one whose chest
//! point shows nearest the cursor on screen wins: each chest is taken
//! through the camera's rigid inverse view and its viewport projection (the
//! speech bubbles' projection), and its squared pixel distance to the
//! cursor must be below the best so far, which starts at the largest float
//! the original stores (`0x7f7fc99e`), so there is no range on screen; an
//! unordered distance never wins. There is no facing test. Exarust asks
//! only on the frame the Talk key is pressed, which is when the original
//! asks too.
//!
//! **The panel.** [`exa_ui::dialogue_window::DialogueWindow`] mirrors the
//! engine's panel each frame; the pointer's moves hover its bands, and a
//! left click released over a band picks it ([`exa_script::PickReply`]),
//! as the panel's input answers the click. The pointer over the shown panel
//! counts as over the GUI ([`crate::panel_live::PointerOverGui`]), so the
//! world's own press and click paths leave it. The engine opening the panel
//! shows the form with the base form's 256-tick fade on the GUI (real-time)
//! clock, and closing it starts the hide fade; the draw runs at the fade's
//! alpha, handed to the line skin's palette as both scales.
//!
//! **The gamepad.** The pad's command word reaches the dialogue as the
//! player controls' pad routine reads it: with the reply panel open and the
//! power bar closed, Select (`0x10000`) picks the highlighted entry, else a
//! step down the list (`0x80`, the move stick's vertical axis under −0.1)
//! moves the highlight forward, else a step up (`0x40`) moves it back. The
//! pad's Talk (`0x20000`) closes an open panel, as the keyboard's does, and
//! otherwise talks to the character the target query finds in the pad mode
//! ([`exa_script::PadTalk`]), whose panel highlights its first entry. The
//! reader turns the stick into those steps while the panel is open.
//!
//! **The paint.** After the open screens, every showing bubble is measured
//! and placed (a character's over its body position lifted 120 cm, the
//! narrator's 75 % down the screen) and drawn box first, text after
//! ([`exa_ui::speech`]); then the panel's box and rows. Both use the main
//! GUI font of the window's interface level. Nothing runs while there is no
//! bubble and the panel is hidden.
//!
//! Evidence: the Talk key's talk routine and its target query, the bubble
//! placement and frame, and the panel's input and draw routines (REA on
//! `Exanima.exe` 0.9.5).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ai::brain::CharacterId;
use exa_combat::Condition;
use exa_physics::body::Ragdoll;
use exa_script::Conversations;
use exa_script::bubbles::{Camera as BubbleCamera, Speaker, View, project};
use exa_ui::ScriptedPointer;
use exa_ui::dialogue_choices::ChoicePalette;
use exa_ui::dialogue_window::DialogueWindow;
use exa_ui::gui_draw::{GuiFrame, GuiFrames, GuiLayer, GuiTextures, LayerTexture, text_batch};
use exa_ui::speech::{self, BubbleLayer};
use exa_world::room_view::LevelRooms;

use crate::bindings::{Action, Bindings, virtual_key};
use crate::hotkeys::{DialogueFocus, HotkeySet};
use crate::player::{ChaseCamera, PlayerBody};

/// The best squared distance the target query starts from: the float the
/// original stores, just under the largest finite one.
pub const NO_TARGET: f32 = f32::from_bits(0x7f7f_c99e);
/// The condition bit that keeps a character out of the query: collapsing,
/// and so also dead (0xA00).
pub const COLLAPSING: u32 = 0x800;
/// A body without a balance yet shows its bubble this high over its root
/// (m) [guess: the original's body position is at the pelvis, which a
/// standing human carries about this high].
pub const BODY_HEIGHT: f32 = 0.9;

/// The character the Talk key talks to: of `candidates` (who, condition
/// word, the chest's screen point), the first whose squared distance to
/// `cursor` is below every earlier one's and below [`NO_TARGET`], skipping
/// collapsing and dead ones. An unordered distance never wins.
pub fn talk_target<T>(
    cursor: [f32; 2],
    candidates: impl IntoIterator<Item = (T, u32, [f32; 2])>,
) -> Option<T> {
    let mut best = NO_TARGET;
    let mut out = None;
    for (who, condition, at) in candidates {
        if condition & COLLAPSING != 0 {
            continue;
        }
        let dx = at[0] - cursor[0];
        let dy = at[1] - cursor[1];
        let distance = dy * dy + dx * dx;
        // The original's ordered compare: an unordered distance falls
        // through.
        if best > distance {
            best = distance;
            out = Some(who);
        }
    }
    out
}

/// The player's nearby characters, as the player behaviour's gather keeps
/// them: of `others` (who and position, cm), those standing in the player's
/// room or in a room one of its exits leads to, seen from the player,
/// within 30 m (3-D), the sixteen nearest. Without the level's rooms every
/// character is in reach of the room walk. [guess: a position in no room
/// counts as in reach, as [`exa_ai::rooms::in_sight`] takes it as seen.]
pub fn nearby_characters(
    level: Option<&LevelRooms>,
    player_cm: Vec3,
    others: &[(Entity, Vec3)],
) -> Vec<Entity> {
    let rooms: Option<Vec<i32>> = level.and_then(|level| {
        let here = exa_ai::rooms::level_room(&level.rooms, player_cm);
        let room = u16::try_from(here).ok()?;
        let mut rooms = vec![here];
        rooms.extend(
            exa_ai::rooms::room_exits(&level.rooms, &level.tracker, room)
                .iter()
                .map(|exit| exit.room - 1),
        );
        Some(rooms)
    });
    let around: Vec<(u64, f32, bool)> = others
        .iter()
        .filter(|(_, at)| {
            let (Some(level), Some(rooms)) = (level, rooms.as_ref()) else {
                return true;
            };
            let room = exa_ai::rooms::level_room(&level.rooms, *at);
            room < 0 || rooms.contains(&room)
        })
        .map(|(who, at)| {
            (
                who.to_bits(),
                (*at - player_cm).length_squared(),
                exa_ai::rooms::in_sight(level, player_cm, *at),
            )
        })
        .collect();
    exa_ai::contacts::nearby(&around)
        .into_iter()
        .filter_map(|(id, _)| Entity::try_from_bits(id))
        .collect()
}

/// The camera as the bubble placement and the target query read it: the
/// rigid inverse of its world transform (positions in cm) and its
/// projection's two scales.
pub fn bubble_camera(eye: &GlobalTransform, clip_from_view: Mat4) -> BubbleCamera {
    let (_, rotation, translation) = eye.to_scale_rotation_translation();
    let right = rotation * Vec3::X;
    let up = rotation * Vec3::Y;
    let back = rotation * Vec3::Z;
    let eye_cm = translation * 100.0;
    BubbleCamera {
        inverse_view: [
            right.x, right.y, right.z, up.x, up.y, up.z, back.x, back.y, back.z, eye_cm.x,
            eye_cm.y, eye_cm.z,
        ],
        projection: [clip_from_view.x_axis.x, clip_from_view.y_axis.y],
    }
}

/// The reply panel as the running game shows it.
#[derive(Resource, Debug, Default)]
pub struct ReplyPanel {
    pub window: DialogueWindow,
    /// The pointer where the panel last saw it (window pixels).
    pointer: Option<IVec2>,
}

/// What the pad's frame asks of the open reply panel, in the order the
/// player controls' pad routine tests the bits: Select picks, else a step
/// down moves the highlight forward, else a step up moves it back. Nothing
/// while the power bar is open (its own steps take the stick).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadPanel {
    Pick,
    Forward,
    Back,
}

/// The panel command of the pad's newly pressed `bits`, with the power bar
/// open or not.
pub fn pad_panel(bits: u32, power_bar: bool) -> Option<PadPanel> {
    use crate::gamepad::command;
    if power_bar {
        return None;
    }
    if bits & command::USE != 0 {
        Some(PadPanel::Pick)
    } else if bits & command::STEP_DOWN != 0 {
        Some(PadPanel::Forward)
    } else if bits & command::STEP_UP != 0 {
        Some(PadPanel::Back)
    } else {
        None
    }
}

/// The pad's newly pressed command bits this frame, when a gamepad drives
/// the player.
fn pad_pressed(
    pad: Option<&crate::gamepad::PlayPad>,
    status: Option<&crate::gamepad::PadStatus>,
) -> u32 {
    match (pad, status) {
        (Some(pad), Some(status)) if status.active() => pad.frame().pressed,
        _ => 0,
    }
}

/// The window's physical size, the pointer in physical pixels and the
/// interface unit.
fn window_frame(
    window: &Window,
    scripted: Option<&ScriptedPointer>,
) -> (IVec2, Option<IVec2>, i32) {
    let size = IVec2::new(
        window.physical_width() as i32,
        window.physical_height() as i32,
    );
    let pointer = scripted
        .map_or_else(|| window.cursor_position(), |s| s.position(window))
        .map(|p| (p * window.scale_factor()).as_ivec2());
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    (size, pointer, unit.max(1))
}

/// The main GUI font of the interface level `unit` picks, with its atlas
/// size when the atlas is loaded.
fn main_font(unit: i32, textures: &GuiTextures) -> (exa_ui::gui_font::GuiFont, Option<(u16, u16)>) {
    let level = exa_ui::gui_font::level_for_unit(unit);
    crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures)
}

/// On the frame the Talk key is pressed with no dialogue open, finds the
/// character it talks to and puts it in [`DialogueFocus`].
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn focus_talk_target(
    keys: Res<ButtonInput<KeyCode>>,
    bindings: Res<Bindings>,
    conversations: Option<Res<Conversations>>,
    level: Option<Res<LevelRooms>>,
    scripted: Option<Res<ScriptedPointer>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    players: Query<(Entity, &GlobalTransform), With<PlayerBody>>,
    characters: Query<
        (
            Entity,
            &GlobalTransform,
            Option<&Ragdoll>,
            Option<&Condition>,
        ),
        (With<CharacterId>, Without<PlayerBody>),
    >,
    mut focus: ResMut<DialogueFocus>,
    ids: Query<&CharacterId>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
) {
    let talk = bindings.key(Action::Talk) & 0xFFFF;
    let pressed = keys
        .get_just_pressed()
        .filter_map(|&k| virtual_key(k))
        .any(|key| key & 0xFF == talk)
        || pad_pressed(pad.as_deref(), pad_status.as_deref()) & crate::gamepad::command::TALK != 0;
    if !pressed
        || conversations
            .as_deref()
            .is_some_and(|c| c.engine.panel.open)
    {
        return;
    }
    focus.0 = None;
    let (Ok(window), Ok((camera, eye)), Some((_, player))) =
        (windows.single(), cameras.single(), players.iter().next())
    else {
        return;
    };
    let (size, pointer, _) = window_frame(window, scripted.as_deref());
    let Some(pointer) = pointer else {
        return;
    };
    let player_cm = player.translation() * 100.0;
    let others: Vec<(Entity, Vec3)> = characters
        .iter()
        .map(|(e, gt, _, _)| (e, gt.translation() * 100.0))
        .collect();
    let nearby = nearby_characters(level.as_deref(), player_cm, &others);
    let view = View {
        width: size.x,
        height: size.y,
        glyph_height: 0,
        camera: None,
    };
    let lens = bubble_camera(eye, camera.clip_from_view());
    let candidates = nearby.iter().filter_map(|&e| {
        let (_, gt, body, condition) = characters.get(e).ok()?;
        let chest = exa_combat::stance::chest_point(gt, body) * 100.0;
        Some((
            e,
            condition.map_or(0, |c| c.0),
            project(&lens, &view, chest.to_array()),
        ))
    });
    let cursor = [pointer.x as f32, pointer.y as f32];
    let target = talk_target(cursor, candidates);
    focus.0 = target.and_then(|e| ids.get(e).ok()).map(|id| id.0);
    info!(
        "talk: {} nearby, target {:?} (character {:?})",
        nearby.len(),
        target,
        focus.0
    );
}

/// The pad's Talk: an open panel closes (the keyboard Talk's leave), and
/// otherwise the focused character is talked to in the pad mode.
fn pad_talk(
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    conversations: Option<Res<Conversations>>,
    focus: Res<DialogueFocus>,
    mut talk: MessageWriter<exa_script::PadTalk>,
    mut leave: MessageWriter<exa_script::LeaveDialogue>,
) {
    if pad_pressed(pad.as_deref(), pad_status.as_deref()) & crate::gamepad::command::TALK == 0 {
        return;
    }
    if conversations
        .as_deref()
        .is_some_and(|c| c.engine.panel.open)
    {
        info!("dialogue: pad Talk leaves");
        leave.write(exa_script::LeaveDialogue);
    } else if let Some(who) = focus.0 {
        info!("dialogue: pad Talk to character {who:#x}");
        talk.write(exa_script::PadTalk(who));
    }
}

/// Mirrors the engine's panel into the reply panel, follows the pointer
/// over it and turns a click on a band into a pick.
#[allow(clippy::too_many_arguments)]
fn panel_input(
    time: Res<Time<Virtual>>,
    real: Res<Time<Real>>,
    conversations: Option<Res<Conversations>>,
    textures: Option<Res<GuiTextures>>,
    scripted: Option<Res<ScriptedPointer>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    buttons: Res<ButtonInput<MouseButton>>,
    pad: Option<Res<crate::gamepad::PlayPad>>,
    pad_status: Option<Res<crate::gamepad::PadStatus>>,
    casters: Query<&crate::thaum::cast::PowerCaster, With<PlayerBody>>,
    mut panel: ResMut<ReplyPanel>,
    mut over: ResMut<crate::panel_live::PointerOverGui>,
    mut pick: MessageWriter<exa_script::PickReply>,
) {
    let Some(conversations) = conversations else {
        return;
    };
    let engine_panel = &conversations.engine.panel;
    if !engine_panel.open && !panel.window.open {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    // The base form's fades run on the GUI clock, the system's milliseconds.
    let gui_now = real.elapsed().as_millis() as u32;
    let (size, pointer, unit) = window_frame(window, scripted.as_deref());
    let empty = GuiTextures::default();
    let (font, _) = main_font(unit, textures.as_deref().unwrap_or(&empty));
    let ReplyPanel {
        window: reply,
        pointer: last,
    } = &mut *panel;
    if engine_panel.open {
        reply.create(size.x, size.y);
    }
    let was_open = reply.open;
    let changed = reply.sync(engine_panel, &font, unit, size.y, now);
    if reply.open && !was_open {
        reply.show(gui_now);
    } else if !reply.open && was_open {
        reply.hide(gui_now);
    }
    if changed || was_open != reply.open {
        info!(
            "dialogue panel: {} with {} entries at {now} ms (GUI clock {gui_now}, pad mode {}, highlight {})",
            if reply.open { "shown" } else { "hidden" },
            reply.view.entries.len(),
            engine_panel.by_pad,
            reply.view.hover
        );
    }
    if !reply.open {
        *last = None;
        return;
    }
    let power_bar = casters.iter().any(|c| c.bar_open);
    match pad_panel(
        pad_pressed(pad.as_deref(), pad_status.as_deref()),
        power_bar,
    ) {
        Some(PadPanel::Pick) => {
            if let Some(n) = reply.highlighted() {
                info!("dialogue: pad Select picks reply {}", n + 1);
                pick.write(exa_script::PickReply(n));
            }
        }
        Some(step) => {
            reply.step_highlight(step == PadPanel::Forward);
            info!(
                "dialogue: pad step {:?} highlights {}",
                step, reply.view.hover
            );
        }
        None => {}
    }
    let Some(at) = pointer else {
        return;
    };
    // The hover follows the pointer's moves, and a new list under a mouse
    // panel's pointer; a pad panel's new list keeps the first entry its
    // layout highlighted until the pointer moves.
    if *last != Some(at) || (changed && !engine_panel.by_pad) {
        *last = Some(at);
        reply.point(at.x, at.y, unit);
    }
    let inside = reply.contains(at.x, at.y);
    over.0 |= inside;
    if inside
        && buttons.just_released(MouseButton::Left)
        && let Some(n) = reply.click()
    {
        info!("dialogue: reply {} clicked", n + 1);
        pick.write(exa_script::PickReply(n));
    }
}

/// Paints the frame's bubbles and the reply panel after the open screens.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn paint_dialogue(
    time: Res<Time<Virtual>>,
    real: Res<Time<Real>>,
    conversations: Option<ResMut<Conversations>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &GlobalTransform), With<ChaseCamera>>,
    characters: Query<(&CharacterId, &GlobalTransform, Option<&Ragdoll>)>,
    menu: Option<Res<crate::menu_screen::MenuScreen>>,
    mut panel: ResMut<ReplyPanel>,
    mut frames: ResMut<GuiFrames>,
) {
    let Some(mut conversations) = conversations else {
        return;
    };
    if conversations.engine.bubbles.list.is_empty() && !panel.window.visible {
        return;
    }
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let now = time.elapsed().as_millis() as u32;
    let gui_now = real.elapsed().as_millis() as u32;
    let (size, _, unit) = window_frame(window, None);
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let (font, atlas) = main_font(unit, textures);
    let Some(atlas) = atlas else {
        return;
    };
    let mut frame = GuiFrame::new();
    if !conversations.engine.bubbles.list.is_empty() {
        let view = View {
            width: size.x,
            height: size.y,
            glyph_height: font.glyph.1,
            camera: cameras
                .single()
                .ok()
                .map(|(camera, eye)| bubble_camera(eye, camera.clip_from_view())),
        };
        // A character's bubble hangs over its body position (the pelvis
        // height the body is anchored at).
        let head = |speaker: Speaker| {
            let Speaker::Character(id) = speaker else {
                return None;
            };
            let (_, gt, body) = characters.iter().find(|(c, ..)| c.0 == id)?;
            let at = body
                .and_then(|b| b.drive.as_ref())
                .and_then(|d| d.anchor())
                .unwrap_or(gt.translation() + Vec3::Y * BODY_HEIGHT);
            Some((at * 100.0).to_array())
        };
        let bubbles = &mut conversations.engine.bubbles;
        speech::layout(bubbles, &font, unit, &view, now, head);
        let draws = speech::bubble_draws(bubbles, unit, font.glyph.1);
        if !draws.is_empty() {
            for layer in speech::layers(&draws, unit, &font, atlas) {
                frame.push(match layer {
                    BubbleLayer::Box(batch) => GuiLayer::new(batch, LayerTexture::None),
                    BubbleLayer::Text(batch) => {
                        GuiLayer::new(batch, LayerTexture::Named(font.texture))
                    }
                });
            }
        }
    }
    // The form's fade: while one runs its alpha is the skin's two scales
    // (the line skin's palette alphas) and the box's alpha; after the draw
    // the scales are back at 1.
    if let Some(alpha) = panel.window.fade_alpha(gui_now) {
        let palette = ChoicePalette::of_skin(&exa_ui::line_skin::shown_palette(alpha));
        if alpha < 1.0 {
            debug!("dialogue panel: fade alpha {alpha} at GUI clock {gui_now}");
        }
        let paint = panel
            .window
            .paint(&font, &palette, unit, size.y, alpha, now);
        frame.push(GuiLayer::new(paint.box_batch(unit), LayerTexture::None));
        for (quads, ink) in [
            (&paint.normal, paint.normal_ink),
            (&paint.highlight, paint.highlight_ink),
        ] {
            if !quads.is_empty() {
                frame.push(GuiLayer::new(
                    text_batch(&font, quads, ink, atlas),
                    LayerTexture::Named(font.texture),
                ));
            }
        }
    }
    if !frame.is_empty() {
        frames.push(frame);
    }
}

pub struct DialogueLivePlugin;

impl Plugin for DialogueLivePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ReplyPanel>()
            .init_resource::<DialogueFocus>()
            .init_resource::<crate::panel_live::PointerOverGui>()
            .init_resource::<GuiFrames>()
            .add_message::<exa_script::PickReply>()
            .add_message::<exa_script::PadTalk>()
            .add_message::<exa_script::LeaveDialogue>()
            .add_systems(
                Update,
                (
                    focus_talk_target
                        .before(HotkeySet)
                        .after(crate::player::cursor_ground),
                    pad_talk.after(focus_talk_target),
                    panel_input
                        .after(crate::panel_live::track_pointer)
                        .before(crate::hand::HandSystems),
                    paint_dialogue.after(crate::gui_screens::draw_screens),
                ),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The nearest chest on screen wins, wherever it is; a collapsing or
    /// dead character is skipped; a tie keeps the first; NaN never wins.
    #[test]
    fn the_talk_target_is_the_chest_nearest_the_cursor() {
        let cursor = [640.0, 360.0];
        let picked = talk_target(
            cursor,
            [
                (1, 0, [900.0, 360.0]),
                (2, 0, [700.0, 400.0]),
                (3, 0x800, [640.0, 360.0]),
                (4, 0xA00, [641.0, 360.0]),
                (5, 0, [f32::NAN, 360.0]),
            ],
        );
        assert_eq!(picked, Some(2));
        // No range on screen: a chest at the far edge still answers.
        assert_eq!(talk_target(cursor, [(7, 0, [-5000.0, 9000.0])]), Some(7));
        // Two at the same distance: the first stays.
        assert_eq!(
            talk_target(cursor, [(1, 0, [650.0, 360.0]), (2, 0, [630.0, 360.0])]),
            Some(1)
        );
        // Beyond the start value (an infinite distance) nothing answers.
        assert_eq!(talk_target(cursor, [(1, 0, [f32::MAX, 0.0])]), None);
        assert_eq!(talk_target::<u32>(cursor, []), None);
        // The stored float is 3.4e38.
        assert_eq!(NO_TARGET, 3.4e38);
    }

    /// The pad routine's order: Select first, then a step down (forward),
    /// then a step up; nothing while the power bar is open.
    #[test]
    fn the_pad_bits_drive_the_panel_in_the_routines_order() {
        use crate::gamepad::command::{STEP_DOWN, STEP_UP, TALK, USE};
        assert_eq!(pad_panel(USE | STEP_DOWN, false), Some(PadPanel::Pick));
        assert_eq!(
            pad_panel(STEP_DOWN | STEP_UP, false),
            Some(PadPanel::Forward)
        );
        assert_eq!(pad_panel(STEP_UP, false), Some(PadPanel::Back));
        assert_eq!(pad_panel(TALK, false), None);
        assert_eq!(pad_panel(USE, true), None);
        assert_eq!(pad_panel(0, false), None);
    }

    /// Without the level's rooms, the nearby list is every seen character
    /// within 30 m, the sixteen nearest by squared distance.
    #[test]
    fn the_nearby_list_keeps_sixteen_within_thirty_metres() {
        let mut others: Vec<(Entity, Vec3)> = (0..20u32)
            .map(|i| {
                (
                    Entity::from_raw_u32(i + 1).unwrap(),
                    Vec3::new(100.0 * (20 - i) as f32, 0.0, 0.0),
                )
            })
            .collect();
        others.push((
            Entity::from_raw_u32(99).unwrap(),
            Vec3::new(3001.0, 0.0, 0.0),
        ));
        let near = nearby_characters(None, Vec3::ZERO, &others);
        assert_eq!(near.len(), 16);
        // Nearest first: the one at 100 cm (index 19, entity 20).
        assert_eq!(near[0], Entity::from_raw_u32(20).unwrap());
        assert!(!near.contains(&Entity::from_raw_u32(99).unwrap()));
        assert!(!near.contains(&Entity::from_raw_u32(1).unwrap()));
    }

    /// The camera's rigid inverse and scales: a camera at (0, 1.6, 5) m
    /// looking down −z puts a point straight ahead at the screen's middle,
    /// and one higher up nearer the top.
    #[test]
    fn the_bubble_camera_projects_through_the_cameras_frame() {
        let eye = GlobalTransform::from_translation(Vec3::new(0.0, 1.6, 5.0));
        // Bevy's perspective with a 90° vertical field: y scale 1, x scale
        // over the aspect.
        let mut clip = Mat4::IDENTITY;
        clip.x_axis.x = 9.0 / 16.0;
        clip.y_axis.y = 1.0;
        let camera = bubble_camera(&eye, clip);
        assert_eq!(camera.inverse_view[9..], [0.0, 160.0, 500.0]);
        assert!((camera.projection[1] - 1.0).abs() < 1e-6);
        assert!((camera.projection[0] - 9.0 / 16.0).abs() < 1e-6);
        let view = View {
            width: 1280,
            height: 720,
            glyph_height: 16,
            camera: Some(camera),
        };
        let ahead = project(&camera, &view, [0.0, 160.0, 0.0]);
        assert_eq!(ahead, [640.0, 360.0]);
        // 100 cm up at 500 cm: 360 − 0.5·720·100/500 = 288.
        let up = project(&camera, &view, [0.0, 260.0, 0.0]);
        assert!((up[1] - 288.0).abs() < 1e-3, "{up:?}");
    }
}

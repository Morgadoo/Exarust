//! The end-of-session menu drawn over the game: the rows, scale and no-row
//! flag of [`crate::flow::EndMenu`], through the GUI draw path and with the
//! main menu's text program on the selected row.
//!
//! The original builds this as its end-session form. Exarust has no form
//! class for it, so the rows are laid out centred under a dark backdrop:
//! the row advance and the menu font come from the menu's row scale times
//! the form's item scale, and the first baseline is the client's middle
//! less half the rows. That placement is Exarust's presentation (a guess);
//! the rows, their order, the scales and what each row asks for are the
//! original's ([`crate::flow::end_menu`], `docs/UI.md`, Death and the end
//! menu).
//!
//! The rows run on the release over them or on Enter. Their actions call
//! the flow: the next practice fight, the story's respawn or exit, a
//! practice restart and the leaving of any other session. The fit against
//! the original's form is [`crate::flow::Step`]s, so the world, camera and
//! save parts remain the host's.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::gui_draw::{GuiFrame, GuiFrames, GuiTextures, LayerTexture, menu_batch, text_batch};
use exa_ui::gui_font::FONTBASE_24;
use exa_ui::gui_text::{MenuFont, menu_font_scale};
use exa_ui::menu::{EndSessionAction, EndSessionInput, dispatch_end_session, menu_row_scale};
use exa_ui::skin::{SkinBatch, SkinInk};

use crate::flow::{Action, EndMenu, Flow, FlowSteps, Mode, Step};
use crate::menu_pages::{row_advance, row_h};

/// The backdrop's darkness, shared with the main menu.
pub const SHADE: u8 = 210;
/// The selected row's ink.
pub const SELECTED_INK: SkinInk = [255, 255, 255, 255];
/// A row that is not selected.
pub const ROW_INK: SkinInk = [255, 255, 255, 255];

/// What an end-menu row asks for. This is the end-session form's dispatch:
/// it branches on the game mode word first, then on the 1-based row, and
/// only then (for the first row) on the menu number.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EndAction {
    /// Practice's NEXT FIGHT: the next fight behind the transition.
    NextFight,
    /// The reconstructed story player continues (the respawn).
    StoryContinue,
    /// The story's end menu exits.
    StoryExit,
    /// Practice's RESTART: the prelude closes and the session leaves to the
    /// menu form (the practice page) behind the transition.
    Restart,
    /// The arena's first row: the arena unit's continue.
    ArenaContinue,
    /// The second row in practice and the arena (EXIT, LOBBY): back to the
    /// title behind the transition.
    QuitToMenu,
    /// A scripted mode's registered handler, with the clicked 1-based row.
    /// The original calls the handler with the row and clears the form's
    /// clicked item; the menu itself is the script's to close.
    Scripted { handler: usize, item: i32 },
}

/// The scripted modes whose end-menu handler a story script registered, in
/// registration order: the original's table of mode words, each entry
/// carrying the handler the click calls. Exarust's script crate is empty, so
/// nothing registers one yet; the table and its dispatch are here for the
/// scripts that will.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct EndHandlers {
    /// The registered mode words, in order of registration.
    pub modes: Vec<u32>,
}

impl EndHandlers {
    /// Registers a scripted mode: the original appends its word and handler
    /// (a mode already present is registered again).
    pub fn register(&mut self, mode: u32) {
        self.modes.push(mode);
    }

    /// The handler index for a mode: the first matching entry.
    #[must_use]
    pub fn handler_for(&self, mode: u32) -> Option<usize> {
        self.modes.iter().position(|&registered| registered == mode)
    }
}

/// A scripted mode's end-menu handler ran: its index and the clicked row.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptedEnd {
    pub handler: usize,
    pub item: i32,
}

/// The action of the 1-based `row` on `menu` in `mode`, or `None` where the
/// dispatch does nothing: a row past the ones it handles (the arena's third
/// row among them) and a scripted mode no script registered.
///
/// The decision is the end-session menu's own click dispatch
/// ([`exa_ui::menu::dispatch_end_session`]).
pub fn end_action(menu: &EndMenu, mode: Mode, row: usize, handlers: &[u32]) -> Option<EndAction> {
    match end_dispatch(menu, mode, row, 0, handlers)? {
        EndSessionAction::PracticeRestart => Some(EndAction::Restart),
        EndSessionAction::NextPracticeFight { .. } => Some(EndAction::NextFight),
        EndSessionAction::BackToMenu => Some(EndAction::QuitToMenu),
        EndSessionAction::StoryRespawn => Some(EndAction::StoryContinue),
        EndSessionAction::StoryQuit => Some(EndAction::StoryExit),
        EndSessionAction::ArenaContinue => Some(EndAction::ArenaContinue),
        EndSessionAction::Registered { handler, item } => {
            Some(EndAction::Scripted { handler, item })
        }
    }
}

/// The end-session click for the 1-based `row` with the practice ladder at
/// `fight` and `handlers` the registered scripted modes.
pub fn end_dispatch(
    menu: &EndMenu,
    mode: Mode,
    row: usize,
    fight: i32,
    handlers: &[u32],
) -> Option<EndSessionAction> {
    let input = EndSessionInput {
        mode: mode.word(),
        item: i32::try_from(row).unwrap_or(i32::MAX),
        menu: i32::try_from(menu.kind).unwrap_or(-1),
        fights: crate::practice::PRACTICE_FIGHTS,
        fight,
    };
    dispatch_end_session(input, handlers).action
}

/// One painted end-menu row: its place, text and ink.
#[derive(Clone, Debug, PartialEq)]
pub struct EndRowPaint {
    pub x: i32,
    /// The painted width, for the hit test.
    pub width: i32,
    /// The row's baseline.
    pub baseline: i32,
    /// The row's top, for the hit test.
    pub top: i32,
    pub text: String,
    pub ink: SkinInk,
    pub selected: bool,
}

/// The rows' layout on a `width` × `height` client.
#[derive(Clone, Debug, PartialEq)]
pub struct EndMenuLayout {
    pub rows: Vec<EndRowPaint>,
    /// The menu font's ascent at the form's scale.
    pub ascent: i32,
    /// The band under the ascent that takes the pointer.
    pub band_inset: i32,
}

impl EndMenuLayout {
    /// The 0-based row under `(x, y)`, using the same strict band as the
    /// main menu.
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        self.rows.iter().position(|row| {
            let width = row.width;
            let left = row.x;
            left < x
                && x < left + width
                && row.top < y
                && y < row.top + (self.ascent - self.band_inset)
        })
    }
}

/// The layout of `menu`'s rows.
pub fn end_layout(
    width: i32,
    height: i32,
    menu: &EndMenu,
    selected: Option<usize>,
) -> EndMenuLayout {
    let row_scale = menu_row_scale(height) * menu.scale;
    let font = MenuFont::at_scale(menu_font_scale(row_scale));
    let advance = row_advance(row_h(height) * menu.scale);
    let count = menu.rows.len() as i32;
    let first_baseline = height / 2 - (advance * count) / 2 + font.ascent;
    let mut rows = Vec::with_capacity(menu.rows.len());
    for (index, label) in menu.rows.iter().enumerate() {
        let text = (*label).to_string();
        let text_width = font.row_width(text.as_bytes()).max(1);
        rows.push(EndRowPaint {
            x: (width - text_width) / 2,
            width: text_width,
            baseline: first_baseline + index as i32 * advance,
            top: first_baseline + index as i32 * advance - font.ascent,
            text,
            ink: if selected == Some(index) {
                SELECTED_INK
            } else {
                ROW_INK
            },
            selected: selected == Some(index),
        });
    }
    EndMenuLayout {
        rows,
        ascent: font.ascent,
        band_inset: font.origin.1,
    }
}

/// The menu's screen state.
#[derive(Resource, Debug, Default, Clone)]
pub struct EndMenuScreen {
    pub open: bool,
    pub menu: Option<EndMenu>,
    /// The selected row, 0-based; `None` when the menu starts with no row
    /// selected (the arena's).
    pub selected: Option<usize>,
    pub pressed: Option<usize>,
    /// The arena's no-row-selected flag.
    pub unselected: bool,
    /// A registered scripted handler the last confirm asked for; the update
    /// system sends it as a [`ScriptedEnd`] and clears it.
    pub scripted: Option<ScriptedEnd>,
}

impl EndMenuScreen {
    /// Shows `menu`; an unselected menu starts on no row, every other on its
    /// first.
    pub fn show(&mut self, menu: EndMenu) {
        self.open = true;
        self.selected = if menu.unselected { None } else { Some(0) };
        self.unselected = menu.unselected;
        self.pressed = None;
        self.menu = Some(menu);
        self.scripted = None;
    }

    /// Hides the menu.
    pub fn close(&mut self) {
        self.open = false;
        self.menu = None;
        self.pressed = None;
        self.selected = None;
        self.scripted = None;
    }

    /// Moves the selection by `delta`, wrapping from no selection into the
    /// rows.
    pub fn move_selection(&mut self, delta: i32) {
        let Some(menu) = self.menu else {
            return;
        };
        let count = menu.rows.len() as i32;
        if count == 0 {
            self.selected = None;
            return;
        }
        self.selected = Some(match self.selected {
            Some(selected) => (selected as i32 + delta).clamp(0, count - 1) as usize,
            None if delta > 0 => 0,
            None => (count - 1) as usize,
        });
    }

    /// Runs the selected row, returning the flow's steps. A row the dispatch
    /// handles closes the menu first; a registered scripted handler is left
    /// for its script to close (the original only calls the handler). A row
    /// the dispatch does not handle leaves the menu as it is.
    pub fn confirm(&mut self, flow: &mut Flow, handlers: &[u32]) -> Vec<Step> {
        let Some(menu) = self.menu else {
            return Vec::new();
        };
        let Some(selected) = self.selected else {
            return Vec::new();
        };
        let Some(action) = end_action(&menu, flow.mode, selected + 1, handlers) else {
            return Vec::new();
        };
        if let EndAction::Scripted { handler, item } = action {
            self.scripted = Some(ScriptedEnd { handler, item });
            self.pressed = None;
            return Vec::new();
        }
        self.close();
        // Each row passes the original's music-fade argument: the next
        // fight fades the music only into the last fight (whose boss track
        // replaces it); the other rows that leave always fade it.
        match action {
            EndAction::NextFight => {
                let into_last = matches!(
                    end_dispatch(
                        &menu,
                        flow.mode,
                        selected + 1,
                        flow.practice.fight,
                        handlers
                    ),
                    Some(EndSessionAction::NextPracticeFight { last: true })
                );
                flow.transition(Action::NextPracticeFight, into_last)
            }
            EndAction::StoryContinue => flow.continue_after_death(),
            EndAction::StoryExit => flow.story_exit(),
            EndAction::Restart => {
                // The prelude closes first. The original also makes two
                // calls (into the experience system and the GUI unit) whose
                // roles are not identified; they are not modelled.
                let mut steps = vec![Step::ClosePrelude];
                steps.extend(flow.transition(Action::LeaveToMenuForm, true));
                steps
            }
            EndAction::ArenaContinue => {
                // The arena unit's continue has no live session bridge yet;
                // the stand-in leaves to the title.
                flow.transition(Action::QuitToMenu, true)
            }
            EndAction::QuitToMenu => flow.transition(Action::QuitToMenu, true),
            EndAction::Scripted { .. } => Vec::new(),
        }
    }
}

pub struct EndMenuScreenPlugin;

impl Plugin for EndMenuScreenPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EndMenuScreen>()
            .init_resource::<EndHandlers>()
            .add_message::<ScriptedEnd>()
            .add_systems(
                Update,
                (update, draw)
                    .chain()
                    .after(crate::gui_screens::draw_screens),
            );
    }
}

/// Reads the pointer and keys while the menu is open.
#[allow(clippy::too_many_arguments)]
fn update(
    mut screen: ResMut<EndMenuScreen>,
    mut flow: Option<ResMut<Flow>>,
    handlers: Option<Res<EndHandlers>>,
    mut scripts: MessageWriter<ScriptedEnd>,
    mut steps: ResMut<FlowSteps>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    if !screen.open {
        return;
    }
    let registered = handlers.as_ref().map_or(&[][..], |h| &h.modes[..]);
    if keys.just_pressed(KeyCode::ArrowDown) {
        screen.move_selection(1);
    }
    if keys.just_pressed(KeyCode::ArrowUp) {
        screen.move_selection(-1);
    }
    if keys.just_pressed(KeyCode::Enter)
        && let Some(flow) = flow.as_mut()
    {
        let out = screen.confirm(flow, registered);
        if let Some(scripted) = screen.scripted.take() {
            scripts.write(scripted);
        }
        steps.steps.extend(out);
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as i32;
    let height = window.physical_height() as i32;
    let Some(menu) = screen.menu else {
        return;
    };
    let layout = end_layout(width, height, &menu, screen.selected);
    let Some(cursor) = window.cursor_position() else {
        return;
    };
    let scale = window.scale_factor();
    let x = (cursor.x * scale) as i32;
    let y = (cursor.y * scale) as i32;
    let hit = layout.hit(x, y);
    if let Some(row) = hit {
        screen.selected = Some(row);
    }
    if mouse.just_pressed(MouseButton::Left) {
        screen.pressed = hit;
    }
    if mouse.just_released(MouseButton::Left)
        && let Some(pressed) = screen.pressed.take()
        && hit == Some(pressed)
        && let Some(flow) = flow.as_mut()
    {
        let out = screen.confirm(flow, registered);
        if let Some(scripted) = screen.scripted.take() {
            scripts.write(scripted);
        }
        steps.steps.extend(out);
    }
}

/// Draws the menu into the GUI path's frame queue.
fn draw(
    screen: Res<EndMenuScreen>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
) {
    if !screen.open {
        return;
    }
    let Some(menu) = screen.menu else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let width = window.physical_width() as i32;
    let height = window.physical_height() as i32;
    if width <= 0 || height <= 0 {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    frames.push(end_menu_frame(
        width,
        height,
        &menu,
        screen.selected,
        textures,
    ));
}

/// The end menu's frame through the GUI draw path.
pub fn end_menu_frame(
    width: i32,
    height: i32,
    menu: &EndMenu,
    selected: Option<usize>,
    textures: &GuiTextures,
) -> GuiFrame {
    let mut frame = GuiFrame::new();
    // The backdrop.
    let mut backdrop = SkinBatch::new();
    backdrop.select(1, 0, false);
    backdrop.set_ink([8, 8, 10, SHADE]);
    backdrop.begin();
    backdrop.vertex(0.0, 0.0);
    backdrop.vertex(0.0, height as f32);
    backdrop.vertex(width as f32, height as f32);
    backdrop.vertex(width as f32, 0.0);
    backdrop.end_quads();
    frame.push(exa_ui::gui_draw::GuiLayer::new(
        backdrop,
        LayerTexture::None,
    ));
    let layout = end_layout(width, height, menu, selected);
    let row_scale = menu_row_scale(height) * menu.scale;
    if textures.get(exa_ui::gui_text::MENU_FONT_TEXTURE).is_some() {
        let font = MenuFont::at_scale(menu_font_scale(row_scale));
        let ordered = layout
            .rows
            .iter()
            .filter(|row| !row.selected)
            .chain(layout.rows.iter().filter(|row| row.selected));
        for row in ordered {
            let (quads, _) = font.draw_line(row.text.as_bytes(), row.x, row.baseline);
            let batch = menu_batch(&quads, row.ink, row.selected);
            frame.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                LayerTexture::Named(exa_ui::gui_text::MENU_FONT_TEXTURE),
            ));
        }
    } else if let Some(texture) = textures.get(FONTBASE_24.texture) {
        let size = (texture.width as u16, texture.height as u16);
        let ordered = layout
            .rows
            .iter()
            .filter(|row| !row.selected)
            .chain(layout.rows.iter().filter(|row| row.selected));
        for row in ordered {
            let text = row.text.as_bytes();
            let x = (width - FONTBASE_24.text_width(text)) / 2;
            let quads = exa_ui::gui_text::draw_line(&FONTBASE_24, text, x, row.baseline);
            let batch = text_batch(&FONTBASE_24, &quads, row.ink, size);
            frame.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                LayerTexture::Named(FONTBASE_24.texture),
            ));
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::{MENU_NEXT, MENU_RECONSTRUCTED, MENU_WON, Mode, end_menu};
    use exa_ui::gui_draw::GuiTexture;

    fn textures() -> GuiTextures {
        let mut textures = GuiTextures::default();
        let blank = vec![255u8; 448 * 896 * 4];
        textures.textures.insert(
            exa_ui::gui_text::MENU_FONT_TEXTURE.into(),
            GuiTexture::from_top_down(448, 896, &blank).unwrap(),
        );
        textures
    }

    #[test]
    fn the_end_menus_ask_for_the_original_actions() {
        let next = end_menu(MENU_NEXT, Mode::Practice);
        assert_eq!(
            end_action(&next, Mode::Practice, 1, &[]),
            Some(EndAction::NextFight)
        );
        let practice = end_menu(MENU_WON, Mode::Practice);
        assert_eq!(
            end_action(&practice, Mode::Practice, 1, &[]),
            Some(EndAction::Restart)
        );
        assert_eq!(
            end_action(&practice, Mode::Practice, 2, &[]),
            Some(EndAction::QuitToMenu)
        );
        let reconstructed = end_menu(MENU_RECONSTRUCTED, Mode::Story);
        assert_eq!(
            end_action(&reconstructed, Mode::Story, 1, &[]),
            Some(EndAction::StoryContinue)
        );
        assert_eq!(
            end_action(&reconstructed, Mode::Story, 2, &[]),
            Some(EndAction::StoryExit)
        );
        // The story's one-row EXIT menu exits the story.
        let won = end_menu(MENU_WON, Mode::Story);
        assert_eq!(
            end_action(&won, Mode::Story, 1, &[]),
            Some(EndAction::StoryExit)
        );
        let arena = end_menu(crate::flow::MENU_ARENA, Mode::Arena);
        assert_eq!(
            end_action(&arena, Mode::Arena, 1, &[]),
            Some(EndAction::ArenaContinue)
        );
        assert_eq!(
            end_action(&arena, Mode::Arena, 2, &[]),
            Some(EndAction::QuitToMenu)
        );
        // The arena's third row and a scripted mode's rows do nothing here.
        assert_eq!(end_action(&arena, Mode::Arena, 3, &[]), None);
        assert_eq!(end_action(&won, Mode::Other(0x20), 1, &[]), None);
    }

    #[test]
    fn a_registered_scripted_mode_hands_its_row_to_the_handler() {
        let won = end_menu(MENU_WON, Mode::Story);
        let mut handlers = EndHandlers::default();
        assert_eq!(handlers.handler_for(0x20), None);
        assert_eq!(
            end_action(&won, Mode::Other(0x20), 1, &handlers.modes),
            None
        );
        handlers.register(0x20);
        // A second registration of the same word is kept; the first wins.
        handlers.register(0x20);
        assert_eq!(handlers.modes, [0x20, 0x20]);
        assert_eq!(handlers.handler_for(0x20), Some(0));
        assert_eq!(
            end_action(&won, Mode::Other(0x20), 1, &handlers.modes),
            Some(EndAction::Scripted {
                handler: 0,
                item: 1
            })
        );
        // The confirm hands the row over and leaves the menu to the script.
        let mut flow = Flow::default();
        flow.mode = Mode::Other(0x20);
        let mut screen = EndMenuScreen::default();
        screen.show(won);
        assert!(screen.confirm(&mut flow, &handlers.modes).is_empty());
        assert_eq!(
            screen.scripted,
            Some(ScriptedEnd {
                handler: 0,
                item: 1
            })
        );
        assert!(screen.open);
    }

    #[test]
    fn restart_leaves_to_the_menu_form_behind_the_transition() {
        let mut flow = Flow::default();
        flow.mode = Mode::Practice;
        flow.phase = crate::flow::Phase::Ended;
        let mut screen = EndMenuScreen::default();
        screen.show(end_menu(MENU_WON, Mode::Practice));
        let steps = screen.confirm(&mut flow, &[]);
        assert_eq!(steps, vec![Step::ClosePrelude, Step::Transition]);
        assert!(!screen.open);
        assert_eq!(flow.phase, crate::flow::Phase::Transition);
        // At black the session leaves to the menu form with its theme.
        let steps = crate::flow::run_to_black(&mut flow);
        assert!(steps.iter().any(|step| matches!(step, Step::ShowMenu(_))));
        assert!(steps.iter().any(|step| matches!(step, Step::Music { .. })));
        // An unhandled row keeps the menu.
        let mut flow = Flow::default();
        flow.mode = Mode::Arena;
        screen.show(end_menu(crate::flow::MENU_ARENA, Mode::Arena));
        screen.selected = Some(2);
        assert!(screen.confirm(&mut flow, &[]).is_empty());
        assert!(screen.open);
    }

    #[test]
    fn the_rows_are_centred_and_the_selected_row_draws_last() {
        let menu = end_menu(MENU_RECONSTRUCTED, Mode::Story);
        let layout = end_layout(1600, 1080, &menu, Some(1));
        assert_eq!(layout.rows.len(), 2);
        let selected = layout.rows.last().unwrap();
        assert!(selected.selected);
        assert_eq!(selected.text, "EXIT");
        // Both labels are centred around the client's middle.
        let first = &layout.rows[0];
        assert!(first.x > 0 && first.x < 1600);
        // A hit uses the strict bands and the row's own width.
        assert_eq!(layout.hit(first.x + 1, first.top + 1), Some(0));
        assert_eq!(layout.hit(0, first.top + 1), None);
    }

    #[test]
    fn the_frame_draws_the_selected_row_with_the_menu_text_program() {
        let menu = end_menu(MENU_RECONSTRUCTED, Mode::Story);
        let frame = end_menu_frame(1600, 1080, &menu, Some(1), &textures());
        assert_eq!(frame.layers.len(), 3);
        assert_eq!(frame.layers[0].batch.mode, exa_ui::skin::SkinMode::Solid);
        assert_eq!(
            frame.layers.last().unwrap().batch.mode,
            exa_ui::skin::SkinMode::MenuText
        );
    }

    #[test]
    fn an_unselected_menu_starts_on_no_row() {
        let arena = end_menu(crate::flow::MENU_ARENA, Mode::Arena);
        let mut screen = EndMenuScreen::default();
        screen.show(arena);
        assert_eq!(screen.selected, None);
        screen.move_selection(1);
        assert_eq!(screen.selected, Some(0));
        screen.move_selection(-1);
        assert_eq!(screen.selected, Some(0));
        screen.close();
        assert!(!screen.open);
    }
}

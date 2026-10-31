//! The running arena arsenal: the company's shared stores as five item
//! columns beside an inventory, painted through the GUI draw path.
//!
//! The form is the original's: a column frame 972 wide (236 + 4 per
//! column, plus the 12-unit margin) and 644 tall (564 in the compact
//! layout), its five column controls 236 × 628 (548 compact) each, the
//! embedded inventory to its left, and the control row on the line the
//! layout centres on the form. The columns' places come from
//! [`exa_items::arsenal::layout`]: their x is the column step from the
//! first shown column, the inventory key's toggle is placed after the five
//! template buttons (the layout overwrites its first place) and the scroll
//! arrows sit at the frame's ends.
//!
//! A column is the same item container the pack is: its child frame takes
//! the pack's style and attribute bits (0x80, 0x10 and 0x100000) and its
//! entries draw through the icon panel. The eight controls come from the
//! form's control template: each 44 × 44 design pixels (11 units), the two
//! arrows, five template buttons and the inventory toggle. The template
//! names pictures (6 for the arrows, 1–5 for the template buttons, 7 for
//! the toggle), not captions; they are drawn here as line-skin buttons
//! with stand-in captions [guess: the pictures are not traced]. The
//! inventory key shows or hides the embedded inventory; Escape closes the
//! arsenal.
//!
//! The hub's Arsenal row sends [`OpenArsenal`]. Not wired: no arena store
//! fills the columns
//! ([`ArsenalWindow::set_column`]); the drop rules stay the tested
//! `exa_items::arsenal::drop_in` and `column_hook`.
//!
//! Evidence: the arsenal view's constructor, layout routine, close, key,
//! scroll and drop handlers and the item container's constructor.

use bevy::prelude::*;

use exa_items::arsenal::{
    self, ColumnPlace, Key, Layout, TOGGLE_CONTROL, arsenal_controls, arsenal_form,
};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::skin::SkinPalette;

use crate::gui_paint::{self, Frame, Painted};
use crate::item_container::Container;
use crate::item_panel_draw::PanelIcons;

/// The form's title [guess: the constructor sets no caption that was
/// traced; its call with 7 loads the eight-entry control template, controls
/// 0 to 7].
pub const TITLE: &str = "Arsenal";
/// The form's style: the base constructor's centred-title bits.
pub const STYLE: u32 = gui_paint::SCREEN_STYLE;
/// The two scroll arrows' stand-in captions (the template's picture 6).
pub const LEFT_ARROW: &str = "<";
pub const RIGHT_ARROW: &str = ">";
/// The five template buttons' stand-in captions (pictures 1 to 5).
pub const TEMPLATE_LABELS: [&str; 5] = ["1", "2", "3", "4", "5"];
/// The inventory toggle's stand-in caption (picture 7).
pub const TOGGLE_LABEL: &str = "Inventory";
/// A control's side in design pixels (at unit 4), as the template sizes
/// all eight.
pub const CONTROL_SIDE: i32 = 44;

/// A key event the arsenal handles. Codes are the pooled input's translated
/// keys; flag bit 2 marks a release (which the arsenal ignores).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalKey {
    pub code: u32,
    pub flags: u32,
}

/// A scroll button pressed, as the control's event mask (bit 2 set, the
/// low word `0x1002` left or `0x2002` right).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalScroll(pub u32);

/// The hub's Arsenal row asks the window to open.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct OpenArsenal;

/// The live arsenal window.
#[derive(Resource, Debug, Clone, PartialEq)]
pub struct ArsenalWindow {
    pub open: bool,
    /// The set whose columns are shown (−1 for none).
    pub set: i32,
    pub scroll: i32,
    pub inventory_shown: bool,
    pub compact: bool,
    pub unit: i32,
    /// The form's place and size on the screen.
    pub bounds: GuiRect,
    /// The five columns' item panels.
    pub columns: [Container; 5],
    /// The embedded inventory's pack.
    pub pack: Container,
    /// The inventory key (the letter `I` by default).
    pub inventory_key: u32,
}

impl Default for ArsenalWindow {
    fn default() -> Self {
        let width = arsenal::FRAME_WIDTH + (arsenal::INVENTORY_SHARE + arsenal::MARGIN) * 4;
        let height = arsenal::FRAME_HEIGHT + 24 * 4;
        Self {
            open: false,
            set: -1,
            scroll: 0,
            inventory_shown: true,
            compact: false,
            unit: 4,
            bounds: GuiRect::new(0, 0, width, height),
            columns: std::array::from_fn(|_| Container::default()),
            pack: Container::default(),
            inventory_key: arsenal::INVENTORY_KEY,
        }
    }
}

impl ArsenalWindow {
    /// The embedded inventory's height, as its constructor sizes it in the
    /// arsenal ([`exa_ui::game_gui::inventory_height`]).
    pub fn inventory_height(&self) -> i32 {
        exa_ui::game_gui::inventory_height(self.compact, true)
    }

    /// Opens the window for a set. The columns start empty; the arena's
    /// store fills them ([`Self::set_column`]).
    pub fn open(&mut self, set: i32) {
        self.open = true;
        self.set = set;
        self.scroll = 0;
        self.inventory_shown = true;
        self.compact = false;
        for column in &mut self.columns {
            *column = Container::default();
        }
    }

    /// Closes the window. The arena answers the close with the set's store
    /// write (`exa_items::arsenal::arsenal_close`).
    pub fn close(&mut self) {
        self.open = false;
    }

    /// The window's layout for its form size and inventory height.
    pub fn layout(&self) -> Layout {
        arsenal::layout(
            self.bounds.width(),
            self.bounds.height(),
            self.bounds.width(),
            self.inventory_height(),
            self.unit,
            self.inventory_shown,
            self.scroll,
        )
    }

    /// The form's constructor words for the current compact flag.
    pub fn form(&self) -> arsenal::ArsenalForm {
        arsenal_form(self.compact)
    }

    /// The control row for a layout.
    pub fn controls(&self, layout: &Layout) -> arsenal::ArsenalControls {
        arsenal_controls(
            layout,
            self.unit,
            self.inventory_shown,
            self.form().last_control,
        )
    }

    /// The visible columns' places inside the column frame, with the shown
    /// flag the layout gives them.
    pub fn column_places(&self, layout: &Layout) -> [(bool, i32, ColumnPlace); arsenal::COLUMNS] {
        let design = arsenal::column_places(self.compact);
        std::array::from_fn(|i| {
            let (shown, x) = layout.columns[i];
            (shown, x, design[i])
        })
    }

    /// A key press. Escape closes; the inventory key shows or hides the
    /// inventory; anything else, and any release, does nothing.
    pub fn key(&mut self, code: u32, flags: u32) {
        match arsenal::key(code, flags, self.inventory_key) {
            Key::Close => self.close(),
            Key::ToggleInventory => self.inventory_shown = !self.inventory_shown,
            Key::None => {}
        }
    }

    /// A scroll button's press: the layout clamps the new scroll.
    pub fn scroll_press(&mut self, mask: u32) {
        let scroll = arsenal::scroll_by(mask, self.scroll);
        self.scroll = scroll;
        self.scroll = self.layout().scroll;
    }

    /// Sets one column's panel (for an arena store to fill; none does yet).
    pub fn set_column(&mut self, column: usize, items: Container) {
        if let Some(slot) = self.columns.get_mut(column) {
            *slot = items;
        }
    }
}

/// The form's painted frame and the geometry its children need.
#[derive(Debug, Default)]
pub struct ArsenalPaint {
    pub painted: Painted,
    /// The column frame's client rectangle.
    pub frame_client: GuiRect,
    /// The column frame's own rectangle.
    pub frame_rect: GuiRect,
    /// The visible columns' rectangles, in screen coordinates.
    pub columns: Vec<GuiRect>,
    /// The control buttons' rectangles by control index, in screen
    /// coordinates.
    pub buttons: Vec<(usize, GuiRect)>,
}

/// Paints the arsenal form: the main frame, the column frame child, the
/// visible columns, the embedded inventory and the control row.
/// `textures` supplies the font; with no font the frame's skin still draws
/// and the captions are skipped.
pub fn paint_arsenal(window: &ArsenalWindow, textures: &GuiTextures) -> ArsenalPaint {
    let palette = palette();
    let level = exa_ui::gui_font::level_for_unit(window.unit);
    let (font, size) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let layout = window.layout();
    let mut frame = Frame::new(window.bounds, window.unit, TITLE);
    frame.style = STYLE;
    let mut painted = gui_paint::paint_frame(&frame, &palette, &font, size);
    let origin = (window.bounds.left, window.bounds.top);
    // The column frame child.
    let frame_rect = GuiRect::new(
        origin.0 + layout.frame.0,
        origin.1 + layout.frame.1,
        layout.frame_width,
        window.form().height,
    );
    let child = gui_paint::paint_child(
        frame_rect,
        window.unit,
        &palette,
        gui_paint::PACK_ATTRIBUTES,
        gui_paint::PACK_STYLE,
        0,
        None,
    );
    let frame_client = child.client;
    for layer in child.layers {
        painted.push(layer);
    }
    // The visible columns, in the frame's client.
    let mut columns = Vec::new();
    for (shown, x, place) in window.column_places(&layout) {
        if !shown {
            continue;
        }
        let rect = GuiRect::new(
            frame_client.left + x,
            frame_client.top,
            place.width,
            place.height.min(frame_client.height()),
        );
        columns.push(rect);
        let column = gui_paint::paint_child(
            rect,
            window.unit,
            &palette,
            gui_paint::PACK_ATTRIBUTES,
            gui_paint::PACK_STYLE,
            0,
            None,
        );
        for layer in column.layers {
            painted.push(layer);
        }
    }
    // The embedded inventory to the left of the frame.
    if window.inventory_shown {
        let inventory = GuiRect::new(
            origin.0 + layout.inventory.0,
            origin.1 + layout.inventory.1,
            arsenal::INVENTORY_SHARE * window.unit,
            window.inventory_height(),
        );
        let panel = gui_paint::paint_panel(
            inventory,
            window.unit,
            &palette,
            gui_paint::PACK_ATTRIBUTES,
            gui_paint::PACK_STYLE,
        );
        for layer in panel.layers {
            painted.push(layer);
        }
    }
    // The control row.
    let controls = window.controls(&layout);
    let mut buttons = Vec::new();
    let mut push_button = |index: usize, at: [i16; 2], label: &str, painted: &mut Painted| {
        let rect = GuiRect::new(
            origin.0 + i32::from(at[0]),
            origin.1 + i32::from(at[1]),
            CONTROL_SIDE * window.unit / 4,
            CONTROL_SIDE * window.unit / 4,
        );
        buttons.push((index, rect));
        let button = gui_paint::paint_line_button(
            rect,
            gui_paint::ButtonWords {
                // [guess: the template's style words are not mapped onto
                // the line skin's]
                style: 0x40,
                state: 0,
                group: 0,
            },
            window.unit,
            &palette,
            1.0,
            &font,
            size,
            label,
        );
        for layer in button.layers {
            painted.push(layer);
        }
    };
    push_button(0, controls.left_arrow, LEFT_ARROW, &mut painted);
    push_button(1, controls.right_arrow, RIGHT_ARROW, &mut painted);
    // The template row runs through the form's last control, the
    // inventory toggle; the toggle is that last row entry (its own earlier
    // place was overwritten), so it is drawn once.
    for (index, at) in &controls.template {
        let label = if *index == TOGGLE_CONTROL {
            TOGGLE_LABEL
        } else {
            TEMPLATE_LABELS
                .get(index.saturating_sub(2))
                .copied()
                .unwrap_or("")
        };
        push_button(*index, *at, label, &mut painted);
    }
    ArsenalPaint {
        painted,
        frame_client,
        frame_rect,
        columns,
        buttons,
    }
}

/// The skin's palette at full fade.
fn palette() -> SkinPalette {
    gui_paint::palette()
}

/// The arsenal's icon layers: one per visible column with entries, clipped
/// to the column's client. `icons` is the item manager's cache and atlas.
pub fn column_layers(
    window: &ArsenalWindow,
    paint: &ArsenalPaint,
    icons: &mut PanelIcons,
) -> Vec<exa_ui::gui_draw::GuiLayer> {
    let mut layers = Vec::new();
    for (at, rect) in paint.columns.iter().enumerate() {
        let Some(column) = window.columns.get(at) else {
            continue;
        };
        if column.entries.is_empty() {
            continue;
        }
        if let Some(mut layer) = icons.layer(column) {
            gui_paint::offset_batch(&mut layer.batch, rect.left, rect.top);
            layer.clip = Some([rect.left, rect.top, rect.width(), rect.height()]);
            layers.push(layer);
        }
    }
    layers
}

/// Sends the arsenal's key and scroll presses to the open window and draws
/// it into the shared GUI frames after the screens.
/// Opens the window when the hub asks. The set stays 0 until a store fills it.
fn open_arsenal(mut window: ResMut<ArsenalWindow>, mut open: MessageReader<OpenArsenal>) {
    if open.read().next().is_some() {
        window.open(0);
        info!("arsenal window opens");
    }
}

fn arsenal_live(
    mut window: ResMut<ArsenalWindow>,
    mut keys: MessageReader<ArsenalKey>,
    mut scrolls: MessageReader<ArsenalScroll>,
    mut icons: Option<ResMut<PanelIcons>>,
    mut frames: ResMut<GuiFrames>,
    textures: Option<Res<GuiTextures>>,
) {
    for key in keys.read() {
        if window.open {
            window.key(key.code, key.flags);
        }
    }
    for scroll in scrolls.read() {
        if window.open {
            window.scroll_press(scroll.0);
        }
    }
    if !window.open {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let ArsenalPaint {
        mut painted,
        columns,
        frame_client,
        frame_rect,
        buttons,
    } = paint_arsenal(&window, textures);
    let geometry = ArsenalPaint {
        painted: Painted::default(),
        frame_client,
        frame_rect,
        columns,
        buttons,
    };
    if let Some(icons) = icons.as_deref_mut() {
        for layer in column_layers(&window, &geometry, icons) {
            painted.push(layer);
        }
    }
    painted.submit(&mut frames);
}

/// The arsenal window's plugin.
pub struct ArsenalViewPlugin;

impl Plugin for ArsenalViewPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ArsenalWindow>()
            .init_resource::<GuiFrames>()
            .add_message::<ArsenalKey>()
            .add_message::<ArsenalScroll>()
            .add_message::<OpenArsenal>()
            .add_systems(
                Update,
                (open_arsenal, arsenal_live)
                    .chain()
                    .after(crate::gui_screens::draw_screens)
                    .after(crate::item_icon_render::IconRenderSystems),
            );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_items::arsenal::{COLUMNS, FRAME_HEIGHT, FRAME_WIDTH, INVENTORY_KEY};

    fn window() -> ArsenalWindow {
        let mut window = ArsenalWindow {
            bounds: GuiRect::new(0, 0, 1000, 700),
            unit: 4,
            ..ArsenalWindow::default()
        };
        window.open(0);
        window
    }

    #[test]
    fn the_layout_places_the_columns_and_the_inventory() {
        let window = window();
        let layout = window.layout();
        // The arsenal's embedded inventory is 590 tall: (700 - 590)/2 -
        // 8*4 = 23.
        assert_eq!(window.inventory_height(), 590);
        assert_eq!(layout.top, 23);
        // The visible columns: (1000 - 12*4 - 61*4) / (60*4) = 2.
        assert_eq!(layout.visible, 2);
        // left = (1000 - (60*2 + 3 + 61)*4)/2 = 132; the frame follows the
        // inventory, the inventory sits four units before the left edge.
        assert_eq!(layout.left, 132);
        assert_eq!(layout.frame, (132 + 61 * 4, 12 * 4 + 23));
        assert_eq!(layout.inventory, (132 - 4 * 4, 12 * 4 + 23));
        assert_eq!(layout.frame_width, 3 * 4 + 60 * 4 * 2);
        // Two shown columns, the first at 0 and the second a step on.
        assert_eq!(layout.columns[0], (true, 0));
        assert_eq!(layout.columns[1], (true, 60 * 4));
        assert_eq!(layout.columns[2], (false, 60 * 4 * 2));
        // The left arrow is enabled only once the scroll is past zero.
        assert!(!layout.left_arrow);
        assert!(layout.right_arrow);
    }

    #[test]
    fn the_control_row_runs_through_the_last_control() {
        let window = window();
        let layout = window.layout();
        let controls = window.controls(&layout);
        // The template row starts at control 2 and ends at the form's last
        // control (7): six entries.
        assert_eq!(controls.template.len(), 6);
        assert_eq!(controls.template[0].0, 2);
        assert_eq!(controls.template[5].0, TOGGLE_CONTROL);
        assert_eq!(controls.template[0].1[1], layout.top as i16);
        // The toggle keeps the template row's place, not its first place.
        assert_eq!(controls.toggle, controls.template[5].1);
        // The arrows sit at the form's ends on the same row.
        assert_eq!(controls.left_arrow[1], layout.top as i16);
        assert_eq!(controls.left_arrow[0] as i32, layout.frame.0 - 2 * 4);
        assert_eq!(
            controls.right_arrow[0] as i32,
            (layout.visible * 60 - 6) * 4 + layout.frame.0
        );
    }

    #[test]
    fn escape_closes_and_the_inventory_key_toggles() {
        let mut window = window();
        window.key(0x1b, 0);
        assert!(!window.open);
        window.open = true;
        assert!(window.inventory_shown);
        window.key(INVENTORY_KEY, 0);
        assert!(!window.inventory_shown);
        window.key(INVENTORY_KEY, 0);
        assert!(window.inventory_shown);
        // Releases do nothing, and another key does nothing.
        window.key(0x1b, 2);
        assert!(window.open);
        window.key(0x41, 0);
        assert!(window.open);
    }

    #[test]
    fn scrolling_stops_at_the_ends() {
        let mut window = window();
        // Two columns fit: the scroll runs up to three.
        for _ in 0..5 {
            window.scroll_press(0x2002);
        }
        assert_eq!(window.scroll, 3);
        window.scroll_press(0x1002);
        assert_eq!(window.scroll, 2);
        for _ in 0..4 {
            window.scroll_press(0x1002);
        }
        assert_eq!(window.scroll, 0);
        // Without the scroll event's bit (2) the mask changes nothing.
        window.scroll_press(0x2000);
        assert_eq!(window.scroll, 0);
    }

    #[test]
    fn the_form_words_are_the_constructors() {
        let window = window();
        let form = window.form();
        assert_eq!(form.width, FRAME_WIDTH);
        assert_eq!(form.height, FRAME_HEIGHT);
        assert!(form.inventory_shown);
        assert_eq!(form.last_control, 7);
        assert_eq!(window.columns.len(), COLUMNS);
        // A column frame's initial size at the default unit.
        let places = window.column_places(&window.layout());
        assert_eq!((places[0].2.width, places[0].2.height), (236, 628));
        assert_eq!(places[1].2.x, 240);
    }

    #[test]
    fn the_paint_puts_every_part_where_the_layout_says() {
        let window = window();
        let textures = GuiTextures::default();
        let paint = paint_arsenal(&window, &textures);
        assert!(!paint.painted.layers.is_empty());
        assert_eq!(paint.columns.len(), 2);
        let layout = window.layout();
        assert_eq!(
            paint.frame_rect,
            GuiRect::new(
                layout.frame.0,
                layout.frame.1,
                layout.frame_width,
                FRAME_HEIGHT
            )
        );
        // The columns start at the frame's client.
        assert!(paint.columns[0].left >= paint.frame_client.left);
        // Two arrows and the six-entry template row, whose last entry is
        // the inventory toggle.
        assert_eq!(paint.buttons.len(), 8);
        assert_eq!(paint.buttons.iter().filter(|(i, _)| *i == 0).count(), 1);
        assert_eq!(
            paint
                .buttons
                .iter()
                .filter(|(i, _)| (2..=TOGGLE_CONTROL).contains(i))
                .count(),
            6
        );
    }
}

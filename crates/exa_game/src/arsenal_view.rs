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
//! inventory key (the player's bound one) shows or hides the embedded
//! inventory; Escape closes the arsenal, and so does a right release over
//! its form ([`arsenal_input`] hands the open window the frame's pressed
//! keys and that release).
//!
//! The hub's Arsenal row sends [`OpenArsenal`]. The open loads the first
//! shelf's five boxes into the columns ([`load_shelf`], from the arena's
//! stores, [`crate::save_live::ArenaStores`]); a shelf button stores the
//! columns into the shown shelf's boxes and loads the shelf it names, the
//! arrows scroll and the toggle shows or hides the inventory (a left
//! release over a control, [`ArsenalButton`]); the close stores the columns
//! back. The drop rules stay the tested `exa_items::arsenal::drop_in` and
//! `column_hook`; the embedded inventory's pack is not bound.
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

use crate::arsenal as arsenal_store;
use crate::gui_paint::{self, Painted};
use crate::item_container::Container;
use crate::item_panel_draw::PanelIcons;

/// The form's title: the constructor sets no caption (its call with 7
/// loads the eight-entry control template, controls 0 to 7, whose first
/// label reads "ARSENAL"), so the committed title bar has no text.
pub const TITLE: &str = "";
/// The form's style: the constructor sets no style bits of its own.
pub const STYLE: u32 = gui_paint::ARSENAL_WORDS.style;
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

/// A mouse event on the arsenal's form, as its event mask (`0x2002` a
/// right release).
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalMouse(pub u32);

/// A left release over one of the form's eight controls (its index in the
/// control template, [`exa_items::arsenal::CONTROLS`]): the control's
/// callback runs.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArsenalButton(pub usize);

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
    /// The eight controls' state words (the down bits of the shelf radio
    /// group and of the inventory toggle).
    pub states: [u32; arsenal::LAST_CONTROL + 1],
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
            states: arsenal::CONTROLS.map(|control| control.state),
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
        self.states = arsenal::CONTROLS.map(|control| control.state);
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
            Key::ToggleInventory => self.show_inventory(!self.inventory_shown),
            Key::None => {}
        }
    }

    /// Shows or hides the embedded inventory; the toggle control's down
    /// bits follow ([`exa_items::arsenal::toggle_flags`]).
    pub fn show_inventory(&mut self, shown: bool) {
        self.inventory_shown = shown;
        self.states[TOGGLE_CONTROL] = arsenal::toggle_flags(self.states[TOGGLE_CONTROL], shown);
    }

    /// A click on control `index`: the control's state changes
    /// ([`exa_items::arsenal::click_states`]) and its callback runs. The
    /// arrows scroll one column (the left one only while past zero) and lay
    /// out; the toggle shows the inventory while it is down. A shelf
    /// button that is down returns the shelf it names, for the caller's
    /// [`load_shelf`].
    pub fn click(&mut self, index: usize) -> Option<usize> {
        let record = arsenal::CONTROLS.get(index)?;
        arsenal::click_states(&mut self.states, index);
        match record.action {
            arsenal::ControlAction::ScrollLeft => self.scroll_press(0x1002),
            arsenal::ControlAction::ScrollRight => self.scroll_press(0x2002),
            arsenal::ControlAction::ToggleInventory => {
                self.inventory_shown = self.states[index] & arsenal::DOWN != 0;
            }
            arsenal::ControlAction::Shelf => {
                if self.states[index] & arsenal::DOWN != 0 {
                    return Some(record.value as usize);
                }
            }
        }
        None
    }

    /// A scroll button's press: the layout clamps the new scroll.
    pub fn scroll_press(&mut self, mask: u32) {
        let scroll = arsenal::scroll_by(mask, self.scroll);
        self.scroll = scroll;
        self.scroll = self.layout().scroll;
    }

    /// Sets one column's panel directly (the shelf load fills them from the
    /// stores, [`load_shelf`]).
    pub fn set_column(&mut self, column: usize, items: Container) {
        if let Some(slot) = self.columns.get_mut(column) {
            *slot = items;
        }
    }
}

/// A shelf box's entries as the list a column loads: the item, its
/// flags and its place in sixteenths of a unit.
fn box_children(entries: &[arsenal_store::ArsenalEntry]) -> Vec<crate::item_container::ListChild> {
    entries
        .iter()
        .map(|entry| crate::item_container::ListChild {
            id: entry.item,
            flags: entry.flags,
            x: entry.first as i16,
            y: entry.second as i16,
        })
        .collect()
}

/// A column's list back as a shelf box's entries.
fn box_entries(children: &[crate::item_container::ListChild]) -> Vec<arsenal_store::ArsenalEntry> {
    children
        .iter()
        .map(|child| arsenal_store::ArsenalEntry {
            item: child.id,
            flags: child.flags,
            first: child.x as u16,
            second: child.y as u16,
        })
        .collect()
}

/// Stores the five columns into the shown shelf's boxes (column `b` into
/// box `b`), when a shelf is shown.
pub fn store_columns(window: &ArsenalWindow, stores: &mut arsenal_store::Arsenal) {
    let Some(shelf) = usize::try_from(window.set)
        .ok()
        .and_then(|set| stores.shelves.get_mut(set))
    else {
        return;
    };
    for (column, entries) in window.columns.iter().zip(shelf.iter_mut()) {
        *entries = box_entries(&column.save_list());
    }
}

/// The shelf load of a shelf button that went down: the columns go into
/// the shown shelf's boxes, the shown shelf becomes `shelf`, and each
/// column gives its icons back and loads its box of the new shelf. Every
/// entry is kept, with the icon slot the item manager gives it (none
/// without one), so a store written back loses nothing. [The original
/// then sorts a box once when its dirty byte is set; nothing Exarust runs
/// sets one, so no sort runs.]
pub fn load_shelf(
    window: &mut ArsenalWindow,
    stores: &mut arsenal_store::Arsenal,
    shelf: usize,
    mut icons: Option<&mut PanelIcons>,
    catalogue: Option<&exa_items::catalogue::ItemCatalogue>,
) {
    if shelf >= arsenal_store::SHELVES {
        return;
    }
    store_columns(window, stores);
    window.set = shelf as i32;
    let places = arsenal::column_places(window.compact);
    let vscale = if window.compact {
        arsenal::COMPACT_COLUMN_SCALE
    } else {
        1.0
    };
    for (index, column) in window.columns.iter_mut().enumerate() {
        if let Some(icons) = icons.as_deref_mut() {
            icons.release_container(column);
        }
        let place = places[index];
        column.client = (place.width, place.height);
        column.vscale = vscale;
        column.green_mask = place.green;
        column.yellow_mask = place.yellow;
        let children = box_children(&stores.shelves[shelf][index]);
        let slots: Vec<usize> = children
            .iter()
            .map(|child| {
                icons
                    .as_deref_mut()
                    .map_or(0, |icons| icons.reserve(child.id, child.flags, catalogue).0)
            })
            .collect();
        let mut next = slots.into_iter();
        let icons_ref = icons.as_deref();
        column.load_list(
            &children,
            |id, _| {
                Some((
                    next.next().unwrap_or(0),
                    crate::body_inventory::item_bulk(id, catalogue),
                ))
            },
            |slot| icons_ref.and_then(|icons| icons.half(slot)),
        );
    }
    info!(
        "arsenal: shelf {shelf} loaded, {} items over its five boxes",
        window.columns.iter().map(|c| c.entries.len()).sum::<usize>()
    );
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
    // The game skin's form draw with the words the show commits: the
    // arsenal is a base form, so top-level it is titled, shadowed and
    // corner-framed; its column frame (style 0x10), its columns (item
    // panels) and the embedded inventory are held.
    let (mut outer, committed) = gui_paint::paint_top_form(
        window.bounds,
        window.unit,
        &palette,
        1.0,
        gui_paint::ARSENAL_WORDS,
        Some((TITLE, &font, size)),
    );
    let client = GuiRect::new(
        window.bounds.left + gui_paint::BORDER,
        window.bounds.top + gui_paint::BORDER,
        window.bounds.width() - 2 * gui_paint::BORDER,
        window.bounds.height() - 2 * gui_paint::BORDER,
    );
    let mut painted = Painted {
        client,
        ..Painted::default()
    };
    painted.layers.extend(outer.under());
    let origin = (window.bounds.left, window.bounds.top);
    let held = |rect: GuiRect, owner: GuiRect, owner_attributes: u32, words| {
        gui_paint::held_layers(
            rect,
            owner,
            owner_attributes,
            window.unit,
            &palette,
            1.0,
            words,
        )
    };
    // The column frame child.
    let frame_rect = GuiRect::new(
        origin.0 + layout.frame.0,
        origin.1 + layout.frame.1,
        layout.frame_width,
        window.form().height,
    );
    painted.layers.extend(held(
        frame_rect,
        client,
        committed.attributes,
        gui_paint::COLUMN_FRAME_WORDS,
    ));
    let frame_client = GuiRect::new(
        frame_rect.left + 4,
        frame_rect.top + 4,
        frame_rect.width() - 8,
        frame_rect.height() - 8,
    );
    let frame_words = gui_paint::commit_words(
        gui_paint::COLUMN_FRAME_WORDS.style,
        gui_paint::COLUMN_FRAME_WORDS.attributes,
        false,
    );
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
        painted.layers.extend(held(
            rect,
            frame_client,
            frame_words.attributes,
            gui_paint::ITEM_PANEL_WORDS,
        ));
    }
    // The embedded inventory to the left of the frame.
    if window.inventory_shown {
        let inventory = GuiRect::new(
            origin.0 + layout.inventory.0,
            origin.1 + layout.inventory.1,
            arsenal::INVENTORY_SHARE * window.unit,
            window.inventory_height(),
        );
        painted.layers.extend(held(
            inventory,
            client,
            committed.attributes,
            gui_paint::EMBEDDED_INVENTORY_WORDS,
        ));
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
    painted.layers.extend(outer.over());
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

/// Opens the window when the hub asks: the player's controls go off
/// first, as the original's arsenal open does, and the open loads the
/// first shelf (its button starts down) from the arena's stores; with no
/// stores the columns stay empty and no shelf is shown, so the close
/// writes nothing back.
fn open_arsenal(
    mut window: ResMut<ArsenalWindow>,
    mut open: MessageReader<OpenArsenal>,
    control: Option<ResMut<crate::player_controls::PlayerControl>>,
    (mut stores, mut icons, catalogue): ArsenalStores<'_>,
) {
    if open.read().next().is_some() {
        if let Some(mut control) = control {
            crate::controls_live::switch_controls(&mut control, true, "arsenal opens");
        }
        window.open(-1);
        if let Some(stores) = stores.as_deref_mut() {
            load_shelf(
                &mut window,
                &mut stores.arsenal,
                0,
                icons.as_deref_mut(),
                catalogue.as_deref(),
            );
        }
        info!("arsenal window opens");
    }
}

/// The arena's stores the columns show, the item manager's icons and the
/// item catalogue.
type ArsenalStores<'w> = (
    Option<ResMut<'w, crate::save_live::ArenaStores>>,
    Option<ResMut<'w, PanelIcons>>,
    Option<Res<'w, exa_items::catalogue::ItemCatalogue>>,
);

/// The open arsenal's input as the window manager hands it to the form:
/// each key pressed this frame (its virtual-key code; the inventory key is
/// the player's bound one) and a right release over the form.
pub fn arsenal_input(
    window: Res<ArsenalWindow>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    bindings: Option<Res<crate::bindings::Bindings>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut out: MessageWriter<ArsenalKey>,
    (mut mouse, mut clicks): (MessageWriter<ArsenalMouse>, MessageWriter<ArsenalButton>),
) {
    if !window.open {
        return;
    }
    let inventory = bindings
        .as_deref()
        .map(|b| b.key(crate::bindings::Action::Inventory));
    if let Some(keys) = keys.as_deref() {
        for &key in keys.get_just_pressed() {
            let Some(code) = crate::bindings::virtual_key(key) else {
                continue;
            };
            // The key slot compares with the bound inventory key: the
            // bound code is handed on as the window's own.
            let code = match inventory {
                Some(bound) if bound == code => window.inventory_key,
                _ => code,
            };
            out.write(ArsenalKey { code, flags: 0 });
        }
    }
    if buttons
        .as_deref()
        .is_some_and(|b| b.just_released(MouseButton::Right))
    {
        let over = windows.single().ok().and_then(|w| {
            w.cursor_position()
                .map(|p| (p * w.scale_factor()).as_ivec2())
        });
        if over.is_some_and(|p| window.bounds.contains(p.x, p.y)) {
            mouse.write(ArsenalMouse(0x2002));
        }
    }
    // A left release over a control runs its callback.
    if buttons
        .as_deref()
        .is_some_and(|b| b.just_released(MouseButton::Left))
        && let Some(point) = windows.single().ok().and_then(|w| {
            w.cursor_position()
                .map(|p| (p * w.scale_factor()).as_ivec2())
        })
    {
        // The control drawn last is on top where two overlap.
        let paint = paint_arsenal(&window, &GuiTextures::default());
        if let Some(&(index, _)) = paint
            .buttons
            .iter()
            .rev()
            .find(|(_, rect)| rect.contains(point.x, point.y))
        {
            clicks.write(ArsenalButton(index));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn arsenal_live(
    mut window: ResMut<ArsenalWindow>,
    mut keys: MessageReader<ArsenalKey>,
    mut mouse: MessageReader<ArsenalMouse>,
    mut scrolls: MessageReader<ArsenalScroll>,
    mut clicks: MessageReader<ArsenalButton>,
    (mut stores, mut icons, catalogue): ArsenalStores<'_>,
    mut frames: ResMut<GuiFrames>,
    textures: Option<Res<GuiTextures>>,
    mut control: Option<ResMut<crate::player_controls::PlayerControl>>,
    mut pad: Option<ResMut<crate::pad_menu::PadMenu>>,
) {
    let was_open = window.open;
    for key in keys.read() {
        if window.open {
            window.key(key.code, key.flags);
        }
    }
    // The form's mouse slot: a right release closes, as a key's close
    // does.
    for event in mouse.read() {
        if window.open && arsenal::closes_on_release(event.0) {
            window.close();
            info!("arsenal: a right release closes it");
        }
    }
    for scroll in scrolls.read() {
        if window.open {
            window.scroll_press(scroll.0);
        }
    }
    // The controls' callbacks: the arrows, the shelf buttons and the
    // inventory toggle.
    for click in clicks.read() {
        if !window.open {
            continue;
        }
        if let Some(shelf) = window.click(click.0)
            && let Some(stores) = stores.as_deref_mut()
        {
            load_shelf(
                &mut window,
                &mut stores.arsenal,
                shelf,
                icons.as_deref_mut(),
                catalogue.as_deref(),
            );
        }
    }
    // The close (by a key or a right release): the player's controls come
    // back and the shown shelf's columns are stored, as the original's
    // close does in that order.
    if was_open && !window.open {
        if let Some(control) = control.as_deref_mut() {
            crate::controls_live::switch_controls(control, false, "arsenal closes");
        }
        let close = arsenal::arsenal_close(window.set, control.is_some(), stores.is_some());
        if close.save_columns
            && let Some(stores) = stores.as_deref_mut()
        {
            store_columns(&window, &mut stores.arsenal);
            info!("arsenal: shelf {} stored at the close", window.set);
        }
        let mut columns = std::mem::take(&mut window.columns);
        if let Some(icons) = icons.as_deref_mut() {
            for column in &mut columns {
                icons.release_container(column);
            }
        }
        window.columns = columns;
    }
    // The pad's word goes to the view's key slot: the scroll stick's left
    // and right steps scroll the columns as the buttons do; any other word
    // falls to the base form, whose answer the dispatch leaves alone (the
    // slot hands back the word, not the base form's 0x40).
    if let Some(key) = pad
        .as_deref_mut()
        .and_then(|pad| pad.take(crate::pad_menu::PadForm::Arsenal))
        && window.open
        && key.key & 2 != 0
    {
        window.scroll_press(key.key);
        info!(
            "arsenal: the pad's word {:#x} scrolls to {}",
            key.key, window.scroll
        );
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
            .add_message::<ArsenalMouse>()
            .add_message::<ArsenalScroll>()
            .add_message::<ArsenalButton>()
            .add_message::<OpenArsenal>()
            .add_systems(
                Update,
                (open_arsenal, arsenal_input, arsenal_live)
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

    /// The pad's scroll stick reaches the live window's key slot: right
    /// scrolls the columns on, left back; a navigation word does nothing.
    #[test]
    fn the_pads_scroll_words_scroll_the_live_window() {
        use crate::pad_menu::{PadForm, PadMenu};
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(ArsenalViewPlugin)
            .init_resource::<PadMenu>()
            .insert_resource(window());
        let word = |app: &mut App, key: u32| {
            app.world_mut().resource_mut::<PadMenu>().dispatch(
                crate::gamepad::MenuFrame {
                    key,
                    ..Default::default()
                },
                Some(PadForm::Arsenal.id()),
                true,
            );
            app.update();
            app.world().resource::<ArsenalWindow>().scroll
        };
        assert_eq!(word(&mut app, 0x2002), 1);
        assert_eq!(word(&mut app, 0x2002), 2);
        assert_eq!(word(&mut app, 0x1002), 1);
        assert_eq!(word(&mut app, 0x2001), 1);
    }

    /// The open arsenal takes the keys and a right release over it: the
    /// inventory key flips the embedded inventory, Escape closes, and so
    /// does a right release over the form; one elsewhere does not.
    #[test]
    fn the_keys_and_a_right_release_reach_the_live_window() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(ArsenalViewPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<crate::bindings::Bindings>()
            .insert_resource(window());
        let primary = app
            .world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow))
            .id();
        let press = |app: &mut App, key: KeyCode| {
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.reset_all();
            keys.press(key);
            app.update();
        };
        press(&mut app, KeyCode::KeyI);
        assert!(!app.world().resource::<ArsenalWindow>().inventory_shown);
        press(&mut app, KeyCode::KeyI);
        assert!(app.world().resource::<ArsenalWindow>().inventory_shown);
        press(&mut app, KeyCode::Escape);
        assert!(!app.world().resource::<ArsenalWindow>().open);
        let release = |app: &mut App, at: (f64, f64)| {
            app.world_mut().resource_mut::<ArsenalWindow>().open = true;
            app.world_mut()
                .get_mut::<Window>(primary)
                .unwrap()
                .set_physical_cursor_position(Some(bevy::math::DVec2::new(at.0, at.1)));
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            buttons.reset_all();
            buttons.press(MouseButton::Right);
            buttons.release(MouseButton::Right);
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .reset_all();
            app.update();
            app.world().resource::<ArsenalWindow>().open
        };
        assert!(!release(&mut app, (500.0, 300.0)));
        assert!(release(&mut app, (1100.0, 300.0)));
    }

    fn stores() -> crate::save_live::ArenaStores {
        let mut stores = crate::save_live::ArenaStores::default();
        let entry = |item| arsenal_store::ArsenalEntry {
            item,
            flags: 0,
            first: 0x28,
            second: 4,
        };
        stores.arsenal.shelves[0][0].push(entry(11));
        stores.arsenal.shelves[1][2].push(entry(22));
        stores
    }

    fn live_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .add_plugins(ArsenalViewPlugin)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<crate::bindings::Bindings>()
            .insert_resource(window())
            .insert_resource(stores());
        let primary = app
            .world_mut()
            .spawn((Window::default(), bevy::window::PrimaryWindow))
            .id();
        (app, primary)
    }

    fn column_items(app: &App, column: usize) -> Vec<u32> {
        app.world().resource::<ArsenalWindow>().columns[column]
            .entries
            .iter()
            .map(|entry| entry.id)
            .collect()
    }

    /// The open loads the first shelf's boxes into the columns; the second
    /// shelf button stores them back and loads its own shelf; the close
    /// stores the shown shelf, so the stores come out as they went in.
    #[test]
    fn the_open_loads_the_first_shelf_and_a_shelf_button_loads_its_own() {
        let (mut app, _) = live_app();
        app.world_mut().write_message(OpenArsenal);
        app.update();
        let window = app.world().resource::<ArsenalWindow>();
        assert!(window.open);
        assert_eq!(window.set, 0);
        assert_eq!(column_items(&app, 0), vec![11]);
        assert!(column_items(&app, 2).is_empty());
        app.world_mut()
            .write_message(ArsenalButton(arsenal::FIRST_SHELF_CONTROL + 1));
        app.update();
        let window = app.world().resource::<ArsenalWindow>();
        assert_eq!(window.set, 1);
        assert_eq!(window.states[arsenal::FIRST_SHELF_CONTROL] & arsenal::DOWN, 0);
        assert!(column_items(&app, 0).is_empty());
        assert_eq!(column_items(&app, 2), vec![22]);
        let stored = &app.world().resource::<crate::save_live::ArenaStores>().arsenal;
        assert_eq!(stored.shelves[0][0][0].item, 11);
        assert_eq!(
            (stored.shelves[0][0][0].first, stored.shelves[0][0][0].second),
            (0x28, 4)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        assert!(!app.world().resource::<ArsenalWindow>().open);
        assert_eq!(
            app.world().resource::<crate::save_live::ArenaStores>().clone(),
            stores()
        );
    }

    /// A left release over the right arrow scrolls one column; the left
    /// arrow scrolls back; the toggle hides the inventory and shows it again.
    #[test]
    fn the_arrows_and_the_toggle_answer_a_left_release() {
        let (mut app, primary) = live_app();
        app.world_mut().write_message(OpenArsenal);
        app.update();
        let click = |app: &mut App, index: usize| {
            let paint = paint_arsenal(
                app.world().resource::<ArsenalWindow>(),
                &GuiTextures::default(),
            );
            let at = paint
                .buttons
                .iter()
                .position(|(at, _)| *at == index)
                .unwrap();
            let rect = paint.buttons[at].1;
            // A point of the control no later (upper) control covers.
            let y = rect.top + rect.height() / 2;
            let x = (rect.left..rect.right)
                .find(|&x| {
                    paint.buttons[at + 1..]
                        .iter()
                        .all(|(_, upper)| !upper.contains(x, y))
                })
                .unwrap();
            let centre = bevy::math::DVec2::new(f64::from(x), f64::from(y));
            app.world_mut()
                .get_mut::<Window>(primary)
                .unwrap()
                .set_physical_cursor_position(Some(centre));
            let mut buttons = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            buttons.reset_all();
            buttons.press(MouseButton::Left);
            buttons.release(MouseButton::Left);
            app.update();
        };
        click(&mut app, 1);
        assert_eq!(app.world().resource::<ArsenalWindow>().scroll, 1);
        click(&mut app, 0);
        assert_eq!(app.world().resource::<ArsenalWindow>().scroll, 0);
        click(&mut app, TOGGLE_CONTROL);
        assert!(!app.world().resource::<ArsenalWindow>().inventory_shown);
        click(&mut app, TOGGLE_CONTROL);
        assert!(app.world().resource::<ArsenalWindow>().inventory_shown);
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

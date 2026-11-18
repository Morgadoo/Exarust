//! The running game's in-game screens: the inventory and journal the
//! player's keys open, kept as forms on a [`GuiTask`] so their focus follows
//! the window manager. The show rules, tags, places and the journal's
//! entries are [`exa_ui::game_gui`]'s; the keys come through the player's
//! hotkey handler ([`crate::hotkeys`]), which applies its own permissions
//! and sends a [`ScreenRequest`].
//!
//! The inventory and journal are painted through the GUI draw path
//! ([`crate::gui_paint`]): their frames, the item pack's panel and the
//! control records the constructors append (labels and buttons, placed by
//! the control layout pass). The journal's New, Erase and entry list (its
//! drop-down list of titles) and the inventory's page buttons (the set
//! swap) run their record callbacks. The forms are base forms ([`exa_ui::base_form::BaseForm`]) on a rooted task
//! the pointer drives ([`crate::form_input`]): a title press drags a window,
//! a release over its close mark closes it, a pressed window draws on top,
//! the frame marks and the records take their hover inks and a record that
//! asks for it shows the default tooltip. Entering combat mode closes
//! every screen and the power tree, Control Mind's handover the container
//! family ([`sweep_screens`]); a game save first saves the open screens
//! ([`save_open_screens_for_saves`]). Escape closing the focused
//! screen is Exarust's (the original closes them through their frame marks
//! and the keys again). The skills key's screen
//! and the power tree are outside this unit; the technique selector opens
//! only from the unit's power slot, a control no form uses in 0.9.5.
//!
//! Rules: `docs/UI.md` (The in-game GUI unit).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::base_form::BaseForm;
use exa_ui::form::{ClientRect, UNPLACED, place_unset};
use exa_ui::game_gui::{
    CONTAINER_FAMILY, GAME_SCREEN_FAMILY, GameScreen, INVENTORY_PLACE, INVENTORY_TITLE,
    InventoryInput, JOURNAL_TEXT, JOURNAL_TITLE, Journal, JournalInput, LOOT_PLACE, LOOT_TITLE,
    OpenContainers, ScreenAction, forms_with_tag, inventory_place, inventory_screen,
    journal_screen, loot_screen, tagged_container,
};
use exa_ui::gui_controls::{Control, LayoutClient, layout_records};
use exa_ui::gui_core::{FormId, GuiRect, GuiTask, NO_CONTROL};
use exa_ui::gui_draw::{GuiFrames, GuiTextures};
use exa_ui::skin::SkinBatch;

use crate::body_inventory;
use crate::gui_paint::{self, LineFormAt};
use crate::item_container::{
    CardWindow, Container, ContainerChild, ContainerRecord, POPUP_BORDER, POPUP_PANEL_BORDER,
    PopupPlan, open_container, popup_form_size, popup_icon_height, popup_icon_place, popup_place,
};
use crate::item_panel_draw::PanelIcons;
use crate::menu_screen::MenuScreen;

/// A screen the player's keys ask for.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScreenRequest {
    pub screen: GameScreen,
    /// The player is not out of action (not collapsing).
    pub player_ready: bool,
    /// The player's condition is not the dead mode 0xa00.
    pub player_alive: bool,
    /// The body a loot window is asked for (its entity index), if any; for
    /// the inventory, the character whose inventory opens (`None` is the
    /// player's own).
    pub target: Option<i32>,
    /// The body's record flag word, for the loot window's show rule.
    pub flags: u32,
    /// An inventory that is already the active form closes. The inventory
    /// key asks for that; a script's `ShowInventory` does not.
    pub toggle: bool,
}

impl ScreenRequest {
    /// A request for a screen that is not a body's loot window.
    pub fn screen(screen: GameScreen, player_ready: bool, player_alive: bool) -> Self {
        Self {
            screen,
            player_ready,
            player_alive,
            target: None,
            flags: 0,
            toggle: true,
        }
    }
}

/// One open screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpenScreen {
    pub kind: GameScreen,
    pub form: FormId,
}

/// An open loot window: the body it shows, its pack and what that body
/// wears and holds. The panels are the item container's (the pack's
/// 228 × 180 panel at the client's bottom, the equipment panel above it).
#[derive(Debug, Clone, PartialEq)]
pub struct LootWindow {
    pub form: FormId,
    /// The body's record id (its entity index in Exarust).
    pub character: i32,
    /// The body's record flag word, as it was when the window opened.
    pub flags: u32,
    pub pack: Container,
    /// What the body wears and holds.
    pub worn: Container,
}

impl LootWindow {
    /// An empty window's panels for a GUI unit.
    fn new(form: FormId, character: i32, flags: u32, unit: i32) -> Self {
        let mut pack = Container::form(unit);
        pack.owner = Some(character as u32);
        let mut worn = Container::form(unit);
        worn.owner = Some(character as u32);
        Self {
            form,
            character,
            flags,
            pack,
            worn,
        }
    }
}

/// A request to open a placed container's popup: the double click's
/// interaction on an object whose record has a container section.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContainerRequest {
    /// The placed object carrying the container section
    /// ([`exa_world::ObjectSections`]).
    pub object: Entity,
    /// The placed object's database id, tier tag included.
    pub id: u32,
    /// The pointer when the request was made, physical pixels.
    pub pointer: IVec2,
}

/// An open container popup: the card frame hosting the container's item
/// panel, bound to the container's record.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerPopup {
    pub form: FormId,
    /// The container's tagged database id (the popup's record id field).
    pub id: u32,
    /// The placed object whose record the panel saves back into.
    pub object: Entity,
    /// The record's header fields, which the save keeps.
    pub stored: ContainerRecord,
    pub panel: Container,
    /// The panel's size and top inside the popup's client.
    pub plan: PopupPlan,
    /// The container's own icon (the card's icon handle) and the icon
    /// area's height.
    pub icon: usize,
    pub icon_height: i32,
}

impl ContainerPopup {
    /// The popup's client: its bounds inside its border.
    pub fn client(bounds: GuiRect) -> GuiRect {
        GuiRect::new(
            bounds.left + POPUP_BORDER,
            bounds.top + POPUP_BORDER,
            bounds.width() - 2 * POPUP_BORDER,
            bounds.height() - 2 * POPUP_BORDER,
        )
    }

    /// The item panel's rectangle: at the client's left, the plan's top
    /// down, the plan's size (its own border included).
    pub fn panel_rect(&self, bounds: GuiRect) -> GuiRect {
        let client = Self::client(bounds);
        GuiRect::new(
            client.left + exa_ui::popups::CONTAINER_INSET,
            client.top + self.plan.inner_top,
            self.plan.inner.0,
            self.plan.inner.1,
        )
    }
}

/// A container record as the panel loads it, from the object's section.
pub fn container_record(section: &exa_formats::containers::Container) -> ContainerRecord {
    ContainerRecord {
        size: (section.width, section.height),
        capacity: section.capacity,
        max_item: section.max_item,
        children: section
            .children
            .iter()
            .map(|child| ContainerChild {
                id: child.id,
                flags: child.flags,
                x: child.x,
                y: child.y,
            })
            .collect(),
    }
}

/// Writes a saved record's children back into the object's section, as the
/// close's save rewrites the record's container section.
pub fn store_record(section: &mut exa_formats::containers::Container, record: &ContainerRecord) {
    section.children = record
        .children
        .iter()
        .map(|child| exa_formats::containers::Child {
            id: child.id,
            flags: child.flags,
            x: child.x,
            y: child.y,
        })
        .collect();
}

/// The class of the inventory's second record: the item GUI's rating-dots
/// control (the first of the five it registers, sized 0 × 24), which the
/// inventory fills with ten times the shown body's encumbrance
/// ([`crate::doll_view`]).
pub const RATING_DOTS_CLASS: u16 = 0xac01;

/// The unit the layout pass is given (it reads it only for a first record's
/// unit anchor and for trackbars, which these screens have none of).
const LAYOUT_UNIT: i32 = 4;

/// The record's y for the full-height inventory: the constructor writes
/// one of three values by the form's height variant (0x54 in the arsenal,
/// 0x5e under the compact flag, 0x62 otherwise).
const INVENTORY_LABEL_ROW: i32 = 0x62;

/// A button record: class 1, the style word the journal and inventory
/// constructors share, and its caption.
fn button_record(x: i32, y: i32, w: i32, text: &str) -> Control {
    let mut control = Control::new(1, x, y, w, 0);
    control.style = 0x40_3320;
    control.text = text.as_bytes().to_vec();
    control
}

/// The inventory constructor's four records, as stored (the constructor
/// passes the table's last index, 3): the Encumbrance label, the rating
/// dots beside it and the two page buttons, Primary and Secondary, in
/// radio group 1. The fill lights one of them
/// ([`exa_ui::game_gui::inventory_tab_states`]; the table stores Primary
/// lit). Both buttons carry the same record callback: the one turned on
/// swaps the shown character's weapon sets
/// ([`exa_ui::game_gui::page_button_swaps`]).
fn inventory_records() -> Vec<Control> {
    let mut label = Control::new(5, 4, INVENTORY_LABEL_ROW, 12, 28);
    label.style = 0;
    label.text = b"Encumbrance:".to_vec();
    let dots = Control::new(RATING_DOTS_CLASS, 100, -0x8000, 12, 28);
    let [primary_state, secondary_state] = exa_ui::game_gui::inventory_tab_states(0);
    // Primary's x is stored as a plain 4 and its y as an anchor of eight
    // rows below the record above; Secondary follows it on its row, four
    // after it.
    let mut primary = button_record(4, -0x7ff8, 108, "Primary");
    primary.group = 1;
    primary.state = primary_state;
    primary.reports = true;
    let mut secondary = button_record(-0x7ff7, -0x8000, 108, "Secondary");
    secondary.group = 1;
    secondary.state = secondary_state;
    secondary.reports = true;
    vec![label, dots, primary, secondary]
}

/// The height the inventory constructor leaves the character view short of
/// the form's own: the view is the form's width less two borders by its
/// height less this.
pub const DOLL_HEIGHT_RESERVE: i32 = 0x104;
/// The character view's border (the constructor's `2`), which a rescale
/// keeps (a border under 4 is not scaled).
pub const DOLL_BORDER: i32 = 2;

/// A screen form's design at unit 4: its size, place and border (its
/// client inset), its child forms (client-local rectangles, each with its
/// border) and its records (laid out in the client).
pub(crate) struct ScreenDesign {
    pub(crate) size: (i32, i32),
    pub(crate) place: (i32, i32),
    pub(crate) border: i32,
    pub(crate) style: u32,
    pub(crate) children: Vec<(GuiRect, i32)>,
    pub(crate) records: Vec<Control>,
}

/// A screen form's geometry at a GUI unit, form-local: its design rescaled
/// from unit 4 as showing a form does
/// ([`exa_ui::gui_rescale::rescale_form`]): a border of 4 or more becomes
/// `round(unit · border / 4)` (smaller ones stay), the client and the
/// children's clients scale by `/ 4 · unit`, the places likewise; the
/// records follow the record rescale.
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenLayout {
    pub unit: i32,
    /// The form's width and height.
    pub size: (i32, i32),
    /// The constructor's place, scaled with the form [guess: the show
    /// places the form after its rescale]. An unset coordinate stays unset.
    pub place: (i32, i32),
    /// The form's border (its client inset).
    pub border: i32,
    /// The child forms, form-local.
    pub children: Vec<GuiRect>,
    /// The records, form-local.
    pub controls: Vec<Control>,
}

impl ScreenLayout {
    pub(crate) fn rescaled(unit: i32, design: ScreenDesign) -> Self {
        use exa_ui::gui_rescale::{FormPlace, RecordPlace, rescale_form};
        let inset = |w: i32, h: i32, b: i32| ClientRect {
            left: b,
            top: b,
            right: w - b,
            bottom: h - b,
        };
        let form_place =
            |rect: GuiRect, border: i32, anchors: u32, records: &[Control]| FormPlace {
                x: rect.left,
                y: rect.top,
                w: rect.width(),
                h: rect.height(),
                border,
                design_border: 0,
                title: 0,
                client: inset(rect.width(), rect.height(), border),
                content_height: 0,
                minimum: (0, 0),
                anchors,
                keep_size: false,
                title_only: false,
                container: false,
                children: Vec::new(),
                records: records
                    .iter()
                    .map(|c| RecordPlace {
                        x: c.x as i16,
                        y: c.y as i16,
                        w: c.w as i16,
                        h: c.h as i16,
                        style: c.style,
                        form_bottom: false,
                    })
                    .collect(),
            };
        let unset = |v: i32| v == exa_ui::form::UNPLACED;
        let (px, py) = design.place;
        let mut place = form_place(
            GuiRect::new(
                if unset(px) { 0 } else { px },
                if unset(py) { 0 } else { py },
                design.size.0,
                design.size.1,
            ),
            design.border,
            design.style,
            &design.records,
        );
        place.children = design
            .children
            .iter()
            .map(|&(rect, border)| form_place(rect, border, 0, &[]))
            .collect();
        let mut records = design.records;
        if unit != 4 && unit > 0 {
            let mut realign = |f: &mut FormPlace| f.client = inset(f.w, f.h, f.border);
            let screen = (1 << 20, 1 << 20);
            rescale_form(
                &mut place,
                4,
                unit,
                screen,
                screen,
                6 * unit - gui_paint::BORDER,
                &mut realign,
            );
            for (c, r) in records.iter_mut().zip(&place.records) {
                c.x = i32::from(r.x);
                c.y = i32::from(r.y);
                c.w = i32::from(r.w);
                c.h = i32::from(r.h);
            }
        }
        let border = place.border;
        for record in &mut records {
            record.x += border;
            record.y += border;
        }
        Self {
            unit,
            size: (place.w, place.h),
            place: (
                if unset(px) { px } else { place.x },
                if unset(py) { py } else { place.y },
            ),
            border,
            children: place
                .children
                .iter()
                .map(|c| GuiRect::new(c.x + border, c.y + border, c.w, c.h))
                .collect(),
            controls: records,
        }
    }

    /// The form's client inside `bounds`.
    pub fn client(&self, bounds: GuiRect) -> GuiRect {
        GuiRect::new(
            bounds.left + self.border,
            bounds.top + self.border,
            bounds.width() - self.border * 2,
            bounds.height() - self.border * 2,
        )
    }

    /// Child `index`'s rectangle on the screen.
    pub fn child_at(&self, index: usize, bounds: GuiRect) -> GuiRect {
        let rect = self
            .children
            .get(index)
            .copied()
            .unwrap_or(GuiRect::new(0, 0, 0, 0));
        GuiRect::new(
            rect.left + bounds.left,
            rect.top + bounds.top,
            rect.width(),
            rect.height(),
        )
    }

    /// The records on the screen.
    pub fn controls_at(&self, bounds: GuiRect) -> Vec<Control> {
        let mut controls = self.controls.clone();
        for control in &mut controls {
            control.x += bounds.left;
            control.y += bounds.top;
        }
        controls
    }

    /// The inventory at `unit`: 244 × 644 at (8, 8), border 8, the
    /// character view (the width less two borders by the height less
    /// 0x104, border 2) at the client's top, the pack (228 × 180, border
    /// 2) at its bottom and the three records laid out in the client.
    pub fn inventory(unit: i32) -> Self {
        let (width, height) = GameScreen::Inventory.size().unwrap_or((244, 644));
        let border = gui_paint::INVENTORY_SPACING;
        let client = (width - border * 2, height - border * 2);
        let mut records = inventory_records();
        layout_records(
            &mut records,
            LayoutClient {
                inner_right: client.0,
                border: gui_paint::BORDER,
                bottom: client.1,
            },
            LAYOUT_UNIT,
        );
        Self::rescaled(
            unit,
            ScreenDesign {
                size: (width, height),
                place: INVENTORY_PLACE,
                border,
                style: gui_paint::SCREEN_STYLE,
                children: vec![
                    (
                        GuiRect::new(0, 0, client.0, height - DOLL_HEIGHT_RESERVE),
                        DOLL_BORDER,
                    ),
                    (
                        GuiRect::new(
                            0,
                            client.1 - gui_paint::PACK.1,
                            gui_paint::PACK.0,
                            gui_paint::PACK.1,
                        ),
                        DOLL_BORDER,
                    ),
                ],
                records,
            },
        )
    }

    /// The loot window at `unit`: 244 × 440 at x 400 (y unset), the base
    /// border 2, the pack (228 × 180) at the client's bottom and the
    /// equipment panel above it ending four pixels before the pack, both
    /// with border 2 [guess: the panels' borders are the item container's
    /// 2, as the inventory's pack].
    pub fn loot(unit: i32) -> Self {
        let (width, height) = GameScreen::Loot.size().unwrap_or((244, 440));
        let border = gui_paint::FORM_SPACING;
        let client_height = height - border * 2;
        let pack_top = client_height - exa_ui::game_gui::LOOT_PACK_HEIGHT;
        Self::rescaled(
            unit,
            ScreenDesign {
                size: (width, height),
                place: LOOT_PLACE,
                border,
                style: gui_paint::SCREEN_STYLE,
                children: vec![
                    (
                        GuiRect::new(
                            0,
                            pack_top,
                            exa_items::char_loot::PACK_WIDTH,
                            exa_ui::game_gui::LOOT_PACK_HEIGHT,
                        ),
                        DOLL_BORDER,
                    ),
                    (
                        GuiRect::new(
                            0,
                            0,
                            exa_items::char_loot::PACK_WIDTH,
                            (pack_top - exa_ui::game_gui::LOOT_PANEL_GAP).max(0),
                        ),
                        DOLL_BORDER,
                    ),
                ],
                records: Vec::new(),
            },
        )
    }

    /// The journal at `unit`: 244 × 518 (place unset), the base border 2,
    /// its text box (228 × 412 at (0, 88) in the client, no border) and its
    /// New and Erase records laid out in the client.
    pub fn journal(unit: i32) -> Self {
        let (width, height) = GameScreen::Journal.size().unwrap_or((244, 518));
        let border = gui_paint::FORM_SPACING;
        let mut records = journal_records();
        layout_records(
            &mut records,
            LayoutClient {
                inner_right: width - border * 2,
                border: gui_paint::BORDER,
                bottom: height - border * 2,
            },
            LAYOUT_UNIT,
        );
        Self::rescaled(
            unit,
            ScreenDesign {
                size: (width, height),
                place: (UNPLACED, UNPLACED),
                border,
                style: 0x3000,
                children: vec![(
                    GuiRect::new(
                        JOURNAL_TEXT.left,
                        JOURNAL_TEXT.top,
                        JOURNAL_TEXT.width(),
                        JOURNAL_TEXT.height(),
                    ),
                    0,
                )],
                records,
            },
        )
    }
}

/// The inventory's geometry at a GUI unit, form-local ([`ScreenLayout`]
/// with the character view and the pack named).
#[derive(Debug, Clone, PartialEq)]
pub struct InventoryLayout {
    pub unit: i32,
    /// The form's width and height.
    pub size: (i32, i32),
    /// The constructor's place (8, 8), scaled with the form.
    pub place: (i32, i32),
    /// The form's border (its client inset).
    pub border: i32,
    /// The character view and the item pack, form-local.
    pub doll: GuiRect,
    pub pack: GuiRect,
    /// The records, form-local.
    pub controls: Vec<Control>,
}

impl InventoryLayout {
    /// The layout at `unit`; unit 4 is the constructor's design, with the
    /// client's top where the show's client measure puts it under the title
    /// ([`InventoryLayout::title_shift`]): the character view and the
    /// records sit that much lower, the pack stays at the client's bottom.
    pub fn at(unit: i32) -> Self {
        let layout = ScreenLayout::inventory(unit);
        let shift = Self::title_shift(layout.size, layout.border, unit);
        let mut doll = layout.children[0];
        doll.top += shift;
        doll.bottom += shift;
        let mut controls = layout.controls;
        for control in &mut controls {
            control.y += shift;
        }
        Self {
            unit,
            size: layout.size,
            place: layout.place,
            border: layout.border,
            doll,
            pack: layout.children[1],
            controls,
        }
    }

    /// How far the title moves the client's top below the border: the
    /// line skin's client measure ([`exa_ui::line_skin::line_client`]) of
    /// the committed words starts a titled top-level form at two units and
    /// the DPI border metric (2), where the constructor's layout counted
    /// from the border (two units).
    pub fn title_shift(size: (i32, i32), border: i32, unit: i32) -> i32 {
        let words =
            gui_paint::commit_words(gui_paint::SCREEN_STYLE, gui_paint::FORM_ATTRIBUTES, true);
        let client = exa_ui::line_skin::line_client(
            size.0,
            size.1,
            border,
            words.style,
            words.attributes,
            unit,
        );
        (client.top - border).max(0)
    }

    /// The form's client measure (form-local), as the hit test reads it.
    pub fn measured_client(&self) -> ClientRect {
        let words =
            gui_paint::commit_words(gui_paint::SCREEN_STYLE, gui_paint::FORM_ATTRIBUTES, true);
        exa_ui::line_skin::line_client(
            self.size.0,
            self.size.1,
            self.border,
            words.style,
            words.attributes,
            self.unit,
        )
    }

    /// The form's client inside `bounds`.
    pub fn client(&self, bounds: GuiRect) -> GuiRect {
        GuiRect::new(
            bounds.left + self.border,
            bounds.top + self.border,
            bounds.width() - self.border * 2,
            bounds.height() - self.border * 2,
        )
    }

    fn moved(rect: GuiRect, bounds: GuiRect) -> GuiRect {
        GuiRect::new(
            rect.left + bounds.left,
            rect.top + bounds.top,
            rect.width(),
            rect.height(),
        )
    }

    /// The character view's rectangle on the screen.
    pub fn doll_at(&self, bounds: GuiRect) -> GuiRect {
        Self::moved(self.doll, bounds)
    }

    /// The pack's rectangle on the screen.
    pub fn pack_at(&self, bounds: GuiRect) -> GuiRect {
        Self::moved(self.pack, bounds)
    }

    /// The records on the screen.
    pub fn controls_at(&self, bounds: GuiRect) -> Vec<Control> {
        let mut controls = self.controls.clone();
        for control in &mut controls {
            control.x += bounds.left;
            control.y += bounds.top;
        }
        controls
    }
}

/// The journal's text box (the original's text box child of the journal
/// form, [`exa_ui::text_box::TextBox`]): its text, caret, selection and
/// scroll, and the held form the window manager hits, focuses and scrolls
/// (its scroll bar and the wheel). While the window manager's focus is on
/// it, the keys type into it ([`journal_typing`]).
#[derive(Debug, Clone)]
pub struct JournalBox {
    pub text: exa_ui::text_box::TextBox,
    /// The held form in [`GameScreens::task`], a child of the journal.
    pub form: FormId,
    /// The held form's committed style word (its caret bit) and state word
    /// (its caret-lit bit), and the GUI's shared caret blink counter.
    pub style: u32,
    pub state: u32,
    pub blink: u32,
    /// The box had the window manager's focus last frame.
    pub focused: bool,
    /// A left press on the text: where and when, until the drag threshold
    /// or the release.
    pub press: Option<((i32, i32), u32)>,
}

impl JournalBox {
    /// A screen point in the box's scrolled client coordinates: the form's
    /// local point taken into its content ([`exa_ui::form::FormFrame`]: less
    /// the client's inset, plus the drawn scroll's offset).
    fn content_point(&self, task: &GuiTask, at: (i32, i32)) -> Option<(i32, i32)> {
        let form = task.form(self.form)?.base()?;
        let frame = exa_ui::form::FormFrame {
            x: form.bounds.left,
            y: form.bounds.top,
            client: form.client,
            content: (form.client.width(), self.text.content_height),
            scroll: (0.0, self.text.scroll),
            attributes: form.attributes,
        };
        Some(frame.local_to_content((at.0 - form.bounds.left, at.1 - form.bounds.top)))
    }

    /// Whether a screen point is on the box's client (not its scroll bar).
    fn over_text(&self, task: &GuiTask, at: (i32, i32)) -> bool {
        task.form(self.form)
            .and_then(|f| f.base())
            .is_some_and(|form| {
                let (lx, ly) = (at.0 - form.bounds.left, at.1 - form.bounds.top);
                lx >= form.client.left
                    && lx < form.client.right
                    && ly >= form.client.top
                    && ly < form.client.bottom
            })
    }

    /// The caret's restart (the form's caret-lit bit set and the blink
    /// counter zeroed), as a press or a key does it.
    fn light_caret(&mut self) {
        self.state |= exa_ui::form::state::CARET_LIT;
        self.blink = 0;
        self.text.focused = true;
    }
}

/// The journal constructor's three records, as stored (the opener passes
/// the table's last index, 2): New, Erase placed after it by an anchor,
/// and the entry list, a combo box on the next row whose caption is the
/// selected entry's title. All three have callbacks (add, erase, the
/// list's change). The text box is a child form, not a record.
fn journal_records() -> Vec<Control> {
    let mut new = button_record(4, 6, 104, "New");
    new.reports = true;
    let mut erase = button_record(-0x7ff9, -0x8000, 104, "Erase");
    erase.reports = true;
    let height = exa_ui::controls::extended_class(exa_ui::controls::COMBO_BOX)
        .map_or(0, |class| class.size.1);
    let mut list = Control::new(
        exa_ui::controls::COMBO_BOX,
        4,
        -0x7ff9,
        exa_ui::game_gui::JOURNAL_LIST_WIDTH,
        height,
    );
    list.style = exa_ui::game_gui::JOURNAL_LIST_STYLE;
    list.text = exa_ui::game_gui::NEW_ENTRY.as_bytes().to_vec();
    list.reports = true;
    vec![new, erase, list]
}

/// The window manager's word for a left press (the left button's bit and
/// the press bit 0x1000).
const JOURNAL_PRESS: u32 = 0x1001;

/// The journal's records: New (0), Erase (1) and the entry list (2).
pub const JOURNAL_NEW: usize = 0;
pub const JOURNAL_ERASE: usize = 1;
pub const JOURNAL_LIST: usize = 2;

/// The control records a screen's form carries, laid out by the control
/// layout pass inside the form's client (inset by `spacing`) and moved to
/// window coordinates. None of these records is a trackbar or the first
/// record's unit anchor, so the pass does not depend on the GUI unit.
pub fn screen_controls(kind: GameScreen, bounds: GuiRect) -> Vec<Control> {
    let (mut records, spacing) = match kind {
        GameScreen::Inventory => (inventory_records(), gui_paint::INVENTORY_SPACING),
        GameScreen::Journal => (journal_records(), gui_paint::FORM_SPACING),
        _ => return Vec::new(),
    };
    let client = GuiRect::new(
        bounds.left + spacing,
        bounds.top + spacing,
        bounds.width() - spacing * 2,
        bounds.height() - spacing * 2,
    );
    layout_records(
        &mut records,
        LayoutClient {
            inner_right: client.width(),
            border: gui_paint::BORDER,
            bottom: client.height(),
        },
        LAYOUT_UNIT,
    );
    for record in &mut records {
        record.x += client.left;
        record.y += client.top;
    }
    records
}

/// The `--debug-gui-form` flag: opens one empty form through the skin's
/// opening so the draw path can be checked by eye.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct DebugGuiForm {
    pub enabled: bool,
}

/// The open screens, the window manager that focuses them and the session
/// data behind them.
#[derive(Resource)]
pub struct GameScreens {
    pub task: GuiTask,
    pub open: Vec<OpenScreen>,
    pub journal: Journal,
    pub containers: OpenContainers,
    /// The journal text box's text, as the selected entry's store takes it
    /// (the box's bytes through the ANSI code page).
    pub journal_text: String,
    /// The open journal's text box.
    pub journal_box: Option<JournalBox>,
    /// The player's inventory place, kept from its last close (the
    /// original keeps one; its writer is outside this unit).
    pub inventory_place: Option<(i32, i32)>,
    /// The GUI unit the open inventory was laid out at: its form, panels
    /// and records follow [`InventoryLayout::at`] for it.
    pub inventory_unit: i32,
    /// The GUI unit the open journal was laid out at.
    pub journal_unit: i32,
    /// The player's character record id (Exarust uses the entity index).
    pub player: i32,
    /// The character the open inventory is bound to (its record id): the
    /// player's own, or another character's that the interaction or a
    /// script's `ShowInventory` opened. The pack, the character view and
    /// the drops onto it all read this character.
    pub inventory_owner: i32,
    /// The body of [`Self::inventory_owner`] (the player's body for the
    /// player's own inventory), set when the window opens.
    pub inventory_body: Option<Entity>,
    /// The open inventory's character view shows a body bound by a Loyalty
    /// or possession marker (the original's fill sets the view's mind
    /// window flag from [`crate::mind::casts::is_bound`]): a drag out of
    /// its pack may then break the hold and close the window.
    pub inventory_mind_window: bool,
    /// The inventory's item pack, drawn through the panel's icon path.
    pub pack: Container,
    /// The description card the panel asked for.
    pub card: CardWindow,
    /// The open loot windows, one per body.
    pub loot: Vec<LootWindow>,
    /// The open container popups.
    pub popups: Vec<ContainerPopup>,
    /// Records the closed popups saved, waiting to be written back into
    /// their objects' container sections.
    pub saved_containers: Vec<(Entity, u32, ContainerRecord)>,
    /// Icon handles freed by a closed panel, waiting for the icon manager
    /// to drop their references (the destroy's sweep).
    pub released_icons: Vec<usize>,
    /// The crystal-drain sources the panels' drops filed (the AR crystal
    /// dropped into a container files kind 2 with its id, into a
    /// character's pack kind 3), as `(kind, id)`, waiting for the game's
    /// drain to take them ([`crate::crystal_drain`]).
    pub drain_sources: Vec<(u32, u32)>,
    /// The default tooltip the task's tick showed, while it is up.
    pub tooltip: Option<crate::form_input::ShownTooltip>,
    /// A page button (Primary or Secondary) of the open inventory turned
    /// on this frame: its record callback asks the shown character to
    /// swap its weapon sets ([`swap_from_page_button`]).
    pub page_swap: bool,
    /// The journal's entry list as the selection writes it into the list
    /// record: its titles (in the ANSI code page) and the 1-based row.
    pub journal_titles: Vec<Vec<u8>>,
    pub journal_row: i32,
    /// The journal list's drop-down list while it is open over the
    /// journal.
    pub journal_list: Option<JournalList>,
    /// The journal list's own editing (an editable combo box): its edit
    /// bytes, whether it holds the GUI's key focus and its caret's blink.
    pub journal_combo: JournalCombo,
}

/// The journal list's editing: the edit bytes the combo box keeps (made
/// when a press, a key or the focus first reaches it), whether the box
/// holds the GUI's key focus (the focus word of the journal's form names
/// it: its state's focus bit 0x400 and the form's caret bit follow), and
/// the caret's blink counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct JournalCombo {
    pub view: Option<exa_ui::text_edit::EditView>,
    pub focused: bool,
    pub blink: u32,
}

/// The journal list's open drop-down list and its place on the screen.
#[derive(Debug, Clone, PartialEq)]
pub struct JournalList {
    pub list: exa_ui::drop_down::DropDownList,
    pub rect: exa_ui::settings::Rect,
}

impl Default for GameScreens {
    fn default() -> Self {
        // The task's root form: the screen the windows hang from (the
        // original's task form); a point on no window hits it.
        let mut task = GuiTask::new((800, 600));
        task.add(Box::new(exa_ui::gui_core::PlainForm::new(
            GuiRect::new(0, 0, 1 << 20, 1 << 20),
            4,
        )));
        Self {
            task,
            open: Vec::new(),
            journal: Journal::default(),
            containers: OpenContainers::default(),
            journal_text: String::new(),
            journal_box: None,
            inventory_place: None,
            inventory_unit: 4,
            journal_unit: 4,
            player: 0,
            inventory_owner: 0,
            inventory_body: None,
            inventory_mind_window: false,
            pack: Container::default(),
            card: CardWindow::default(),
            loot: Vec::new(),
            popups: Vec::new(),
            saved_containers: Vec::new(),
            released_icons: Vec::new(),
            drain_sources: Vec::new(),
            tooltip: None,
            page_swap: false,
            journal_titles: Vec::new(),
            journal_row: 0,
            journal_list: None,
            journal_combo: JournalCombo::default(),
        }
    }
}

impl GameScreens {
    pub fn find(&self, kind: GameScreen) -> Option<OpenScreen> {
        self.open.iter().copied().find(|screen| screen.kind == kind)
    }

    pub fn is_open(&self, kind: GameScreen) -> bool {
        self.find(kind).is_some()
    }

    /// The screen the window manager focuses.
    pub fn focused(&self) -> Option<GameScreen> {
        let active = self.task.root_of(self.task.active?);
        self.open
            .iter()
            .find(|screen| screen.form == active)
            .map(|screen| screen.kind)
    }

    /// The window manager's focus is on the journal's text box, which then
    /// takes the keys.
    pub fn journal_box_focused(&self) -> bool {
        self.journal_box
            .as_ref()
            .is_some_and(|b| self.task.active == Some(b.form))
    }

    /// Whether an in-game screen is open on the task: any form whose tag
    /// has the 0xce100000 family bits (the inventory, a loot window, the
    /// journal, a container popup). The controls' idle turn asks it
    /// ([`exa_ui::game_gui::has_game_screen`]).
    pub fn has_game_screen(&self) -> bool {
        let tags: Vec<u32> = self.open.iter().filter_map(|s| s.kind.tag()).collect();
        exa_ui::game_gui::has_game_screen(&tags)
    }

    /// The window manager's focus is on a game screen (a window or its
    /// child, the pointer over it): a key goes to that form's key slot and
    /// climbs its parents to the task, never reaching the game view's key
    /// handler (the player's hotkeys). With the focus on the root (the
    /// pointer over the view) the keys are the view's.
    pub fn keys_to_window(&self) -> bool {
        matches!(self.task.active, Some(form) if Some(form) != self.task.root)
    }

    fn root_client(&self) -> ClientRect {
        ClientRect {
            left: 0,
            top: 0,
            right: self.task.screen.0,
            bottom: self.task.screen.1,
        }
    }

    /// The words and border a screen's form is built with.
    fn screen_words(&self, kind: GameScreen) -> (gui_paint::FormWords, i32) {
        match kind {
            GameScreen::Inventory => (
                gui_paint::FormWords {
                    style: gui_paint::SCREEN_STYLE,
                    attributes: gui_paint::FORM_ATTRIBUTES,
                },
                self.inventory_layout().border,
            ),
            GameScreen::Journal => (gui_paint::JOURNAL_WORDS, gui_paint::FORM_SPACING),
            GameScreen::Container => (gui_paint::CONTAINER_WORDS, POPUP_BORDER),
            _ => (gui_paint::LOOT_WORDS, gui_paint::FORM_SPACING),
        }
    }

    /// A screen's records at `bounds`, at the unit its layout keeps.
    fn screen_records(&self, kind: GameScreen, bounds: GuiRect) -> Vec<Control> {
        match kind {
            GameScreen::Inventory => self.inventory_layout().controls_at(bounds),
            GameScreen::Journal => ScreenLayout::journal(self.journal_unit).controls_at(bounds),
            _ => screen_controls(kind, bounds),
        }
    }

    /// The GUI's rescale walk over the open screens (the level switch's
    /// pass over the root's children, each form scaled from its old unit to
    /// `unit`): a screen at another unit takes its place scaled as a free
    /// form's (`v / old · unit`), its layout's size, client and records at
    /// the new unit, and its item panels run their class's rescale
    /// ([`Container::rescale`]: each entry's place `round(unit · place /
    /// old)`). The journal's text box is laid out again. Container popups
    /// keep the unit they opened at (their size follows their container's
    /// record). Returns how many screens were rescaled.
    pub fn rescale_open(&mut self, unit: i32) -> usize {
        let unit = unit.max(1);
        let mut count = 0;
        for screen in self.open.clone() {
            if screen.kind == GameScreen::TechniqueSelector {
                continue;
            }
            let Some((old, bounds)) = self
                .task
                .form(screen.form)
                .and_then(|f| f.base())
                .map(|f| (f.unit, f.bounds))
            else {
                continue;
            };
            if old == unit || old <= 0 {
                continue;
            }
            let scale = |v: i32| v / old * unit;
            let size = match screen.kind {
                GameScreen::Inventory => {
                    self.inventory_unit = unit;
                    self.pack.rescale(old, unit);
                    self.pack.unit = unit;
                    self.pack.icon_size = exa_items::icon_cache::icon_size(unit);
                    let layout = self.inventory_layout();
                    self.pack.client = (layout.pack.width(), layout.pack.height());
                    layout.size
                }
                GameScreen::Journal => {
                    self.journal_unit = unit;
                    ScreenLayout::journal(unit).size
                }
                GameScreen::Container => {
                    // The popup follows the walk's scale (`v / old · unit`)
                    // for its icon area, its panel is laid out from the
                    // record's cells at the new unit and its items move by
                    // the panel's own rescale [guess: the popup's class has
                    // no rescale of its own, so its plan is rebuilt].
                    let Some(popup) = self.popups.iter_mut().find(|p| p.form == screen.form) else {
                        continue;
                    };
                    popup.icon_height = scale(popup.icon_height);
                    let Some(plan) = open_container(
                        false,
                        Some(popup.stored.size),
                        unit,
                        POPUP_PANEL_BORDER,
                        popup.icon_height,
                    ) else {
                        continue;
                    };
                    popup.plan = plan;
                    popup.panel.rescale(old, unit);
                    popup.panel.unit = unit;
                    popup.panel.icon_size = exa_items::icon_cache::icon_size(unit);
                    popup.panel.client = (
                        plan.inner.0 - 2 * POPUP_PANEL_BORDER,
                        plan.inner.1 - 2 * POPUP_PANEL_BORDER,
                    );
                    popup_form_size(plan, unit, POPUP_BORDER)
                }
                _ => {
                    if let Some(window) = self.loot.iter_mut().find(|w| w.form == screen.form) {
                        for panel in [&mut window.pack, &mut window.worn] {
                            panel.rescale(old, unit);
                            panel.unit = unit;
                            panel.icon_size = exa_items::icon_cache::icon_size(unit);
                        }
                    }
                    ScreenLayout::loot(unit).size
                }
            };
            let bounds = GuiRect::new(scale(bounds.left), scale(bounds.top), size.0, size.1);
            let (words, border) = self.screen_words(screen.kind);
            let committed = gui_paint::commit_words(words.style, words.attributes, true);
            let client = exa_ui::line_skin::line_client(
                bounds.width(),
                bounds.height(),
                border,
                committed.style,
                committed.attributes,
                unit,
            );
            let records = self.screen_records(screen.kind, bounds);
            if let Some(form) = self.task.form_mut(screen.form).and_then(|f| f.base_mut()) {
                form.bounds = bounds;
                form.in_owner = (bounds.left, bounds.top);
                form.unit = unit;
                form.client = client;
                form.records = records;
                form.hover = 0;
                form.held = 0;
                form.over = false;
                form.repeat_at = None;
            }
            self.task.rescale_snaps(screen.form);
            if screen.kind == GameScreen::Journal {
                if let Some(text_box) = self.journal_box.take() {
                    self.task.close_form(text_box.form);
                }
                self.open_journal_box(screen.form);
                self.journal_selected();
            }
            count += 1;
        }
        count
    }

    fn push(&mut self, kind: GameScreen, bounds: GuiRect, unit: i32) -> FormId {
        let root = self.task.root;
        let (words, border) = self.screen_words(kind);
        let unit = unit.max(1);
        let committed = gui_paint::commit_words(words.style, words.attributes, true);
        // The base form's show: a window with a parent is raised in it, and
        // a centring style re-centres it in the parent's client after the
        // layout (no game screen's style centres in 0.9.5: their places
        // come from their openers).
        let plan = exa_ui::form::show_plan(exa_ui::form::ShowInput {
            has_parent: root.is_some(),
            raise: true,
            y_unset: bounds.top == UNPLACED,
            style: words.style,
            adopt_title: false,
            parent_not_root: false,
            unit,
        });
        let bounds = if plan.recentre_x || plan.recentre_y {
            let (x, y) = exa_ui::form::show_place(
                bounds.left,
                bounds.top,
                bounds.width(),
                bounds.height(),
                words.style,
                self.root_client(),
            );
            GuiRect::new(x, y, bounds.width(), bounds.height())
        } else {
            bounds
        };
        let controls = self.screen_records(kind, bounds);
        let client = exa_ui::line_skin::line_client(
            bounds.width(),
            bounds.height(),
            border,
            committed.style,
            committed.attributes,
            unit,
        );
        let mut form = BaseForm::new(
            bounds,
            unit,
            (committed.style, committed.attributes, committed.state),
            client,
        );
        form.parent = root;
        form.records = controls;
        form.tag = kind.tag().unwrap_or(0);
        let form = self.task.add(Box::new(form));
        // The show's snap rebuilds (the parent's, the form's, its snap
        // containers').
        self.task.show_snaps(form);
        self.open.push(OpenScreen { kind, form });
        // The show's activation raises the window; the key focus stays
        // with the form under the pointer.
        if plan.raise {
            self.task.activate(form);
        }
        form
    }

    /// The hover word of an open screen's form (its frame mark under the
    /// pointer, for the skin's hover inks).
    pub fn hover_mark(&self, form: FormId) -> i32 {
        self.task
            .form(form)
            .and_then(|f| f.base())
            .map_or(0, |f| f.hover)
    }

    /// The open inventory's layout, at the unit it was opened at.
    pub fn inventory_layout(&self) -> InventoryLayout {
        InventoryLayout::at(self.inventory_unit)
    }

    /// The journal opener's text box: a held form at the layout's child
    /// (no border, the vertical scroll bar) and the box laid out in its
    /// client at the journal's unit ([`exa_ui::game_gui::journal_text_box`]).
    fn open_journal_box(&mut self, journal: FormId) {
        let layout = ScreenLayout::journal(self.journal_unit);
        let Some(bounds) = self.task.form(journal).map(|f| f.bounds()) else {
            return;
        };
        let rect = layout.child_at(0, bounds);
        let words = gui_paint::JOURNAL_BOX_WORDS;
        let committed = gui_paint::commit_words(words.style, words.attributes, false);
        let client = gui_paint::scrolling_client(rect, 0, words.attributes);
        let mut form = BaseForm::new(
            rect,
            self.journal_unit,
            (committed.style, committed.attributes, committed.state),
            client,
        );
        form.parent = Some(journal);
        let owner = layout.client(bounds);
        form.in_owner = (rect.left - owner.left, rect.top - owner.top);
        form.owner_client = (owner.width(), owner.height());
        form.scroll = Some(exa_ui::base_form::FormScroll {
            fraction: 0.0,
            step: 0.0,
            content: 0,
        });
        let form = self.task.add(Box::new(form));
        // The box's show rebuilds its parent's snap slots: the journal's
        // client is cut around the box.
        self.task.show_snaps(form);
        let fonts =
            exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(self.journal_unit));
        let text =
            exa_ui::game_gui::journal_text_box(self.journal_unit, fonts.main, fonts.small, client);
        self.journal_box = Some(JournalBox {
            text,
            form,
            style: committed.style,
            state: committed.state,
            blink: 0,
            focused: false,
            press: None,
        });
    }

    /// The journal's select (its open, the list's pick, New and Erase): the
    /// list's caption is the selected entry's title and the text box shows
    /// its text, laid out and scrolled to the top.
    pub fn journal_selected(&mut self) {
        let state = self.journal.list_state();
        self.journal_text = state.text.clone();
        // The list record keeps the row, the count and every title for its
        // drop-down list.
        self.journal_row = state.selected;
        self.journal_titles = state
            .titles
            .iter()
            .map(|title| crate::os_clipboard::text_to_ansi(title))
            .collect();
        debug_assert_eq!(self.journal_titles.len() as i32, state.count);
        if let Some(form) = self
            .find(GameScreen::Journal)
            .and_then(|screen| self.task.form_mut(screen.form))
            .and_then(|f| f.base_mut())
            && let Some(list) = form.records.get_mut(JOURNAL_LIST)
        {
            list.text = crate::os_clipboard::text_to_ansi(&state.title);
        }
        // The list's new text starts its edit bytes over.
        self.journal_combo.view = None;
        if let Some(text_box) = self.journal_box.as_mut() {
            text_box
                .text
                .set_text(&crate::os_clipboard::text_to_ansi(&state.text), None);
        }
        self.sync_journal_box();
    }

    /// A journal record's callback: New stores the text, appends a "New
    /// Entry" and selects it, then focuses the third control (the list);
    /// Erase removes the selected entry (or blanks the only one) and
    /// selects what is left. A landed release on the list (the combo box)
    /// opens its drop-down list of the entries' titles
    /// ([`GameScreens::open_journal_list`]).
    pub fn journal_record(&mut self, record: usize) {
        match record {
            JOURNAL_LIST => self.open_journal_list(),
            JOURNAL_NEW => {
                let text = self.journal_text.clone();
                self.journal.add_entry(&text);
                self.journal_selected();
                if let Some(screen) = self.find(GameScreen::Journal) {
                    self.task
                        .focus_set(Some(screen.form), exa_ui::game_gui::JOURNAL_FOCUS);
                }
                // The add's focus moves the GUI's key focus to the list
                // with no answer bits: its whole title is selected, so the
                // first typed key replaces it.
                self.journal_combo_focus(true, 0);
                info!("gui: journal entry {} added", self.journal.entries.len());
            }
            JOURNAL_ERASE => {
                self.journal.delete_entry();
                self.journal_selected();
                info!(
                    "gui: journal entry erased, {} left",
                    self.journal.entries.len()
                );
            }
            _ => {}
        }
    }

    /// The journal list's drop-down list opens under the list record with
    /// the titles the selection stored and its row chosen
    /// ([`crate::option_forms::open_drop_list`]).
    pub fn open_journal_list(&mut self) {
        if self.journal_titles.is_empty() {
            return;
        }
        let Some(owner) = self
            .find(GameScreen::Journal)
            .and_then(|screen| self.task.form(screen.form))
            .and_then(|f| f.base())
            .and_then(|f| f.records.get(JOURNAL_LIST))
            .map(|r| (r.x, r.y, r.w, r.h))
        else {
            return;
        };
        let (list, rect) = crate::option_forms::open_drop_list(
            self.journal_titles.clone(),
            owner,
            self.journal_unit.max(1),
            self.task.screen,
            self.journal_row,
        );
        info!(
            "gui: the journal's entry list opens with {} titles at {rect:?}",
            list.items.len()
        );
        self.journal_list = Some(JournalList { list, rect });
        self.journal_list_bits(true);
    }

    /// The list's open marks the combo box (state 0x60) until the list's
    /// close clears it.
    fn journal_list_bits(&mut self, open: bool) {
        if let Some(list) = self
            .find(GameScreen::Journal)
            .and_then(|screen| self.task.form_mut(screen.form))
            .and_then(|f| f.base_mut())
            .and_then(|f| f.records.get_mut(JOURNAL_LIST))
        {
            if open {
                list.state |= crate::option_forms::LIST_OPEN;
            } else {
                list.state &= !crate::option_forms::LIST_OPEN;
            }
        }
    }

    /// A row picked in the journal's drop-down list (1-based): the list
    /// record's callback, not editing, stores the text box's text into the
    /// selected entry and selects the picked one
    /// ([`exa_ui::game_gui::Journal::list_changed`]), whose title and text
    /// the list and the box then show.
    pub fn journal_pick(&mut self, row: i32) {
        let text = self.journal_text.clone();
        self.journal.list_changed(false, "", row, &text);
        self.journal_selected();
        info!(
            "gui: journal entry {} of {} picked",
            self.journal.current + 1,
            self.journal.entries.len()
        );
    }

    /// The pointer over the journal's open drop-down list, which takes it
    /// while it is up (its form sits over the others): see
    /// [`crate::option_forms::drive_drop_list`]. Returns whether the list
    /// took the frame.
    pub fn journal_list_pointer(&mut self, frame: &crate::form_input::PointerFrame) -> bool {
        let Some(mut open) = self.journal_list.take() else {
            return false;
        };
        let at = frame.at.unwrap_or((i32::MIN, i32::MIN));
        let (keep, row) = crate::option_forms::drive_drop_list(
            &mut open.list,
            open.rect,
            at,
            frame.pressed,
            frame.released,
        );
        if keep {
            self.journal_list = Some(open);
        } else {
            self.journal_list_bits(false);
        }
        if let Some(row) = row {
            self.journal_pick(row);
        }
        true
    }

    /// The journal list's record (the combo box), as the journal's form
    /// holds it.
    fn journal_list_record(&self) -> Option<Control> {
        self.find(GameScreen::Journal)
            .and_then(|screen| self.task.form(screen.form))
            .and_then(|f| f.base())
            .and_then(|f| f.records.get(JOURNAL_LIST))
            .cloned()
    }

    /// The main font of the journal's GUI level: its advances place the
    /// list's caret, as they lay out its drawn run.
    fn journal_font(&self) -> exa_ui::gui_font::GuiFont {
        *exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(self.journal_unit)).main
    }

    /// The journal list's edit bytes, made with its creation's when no
    /// press, key or focus reached it yet.
    fn journal_view(&mut self, text: &[u8]) -> exa_ui::text_edit::EditView {
        *self
            .journal_combo
            .view
            .get_or_insert_with(|| exa_ui::field_edit::created_view(text))
    }

    /// The journal's form focus change (the form's focus slot): from the
    /// list (`focused`) to nothing, or from nothing to the list with the
    /// focus word's answer bits. The list's state takes or loses its
    /// focus bit (0x400) and the form's caret bit follows; a focus that
    /// comes with no answer bits (New's, not a press's) is the focus
    /// message with word 0, which selects the list's whole text.
    fn journal_combo_focus(&mut self, focused: bool, bits: u32) {
        if focused == self.journal_combo.focused {
            return;
        }
        let Some(text) = self.journal_list_record().map(|r| r.text) else {
            return;
        };
        let code = JOURNAL_LIST as u32 + 1;
        let (current, new) = if focused {
            (0, code | (bits & 0x7f_0000))
        } else {
            (code, 0)
        };
        let mut view = self.journal_view(&text);
        if let Some(form) = self
            .find(GameScreen::Journal)
            .and_then(|screen| self.task.form_mut(screen.form))
            .and_then(|f| f.base_mut())
        {
            let mut states: Vec<u32> = form.records.iter().map(|r| r.state).collect();
            if let Some(change) =
                exa_ui::form::focus_change(&mut states, &mut form.state, current, new)
                && let Some((_, 0)) = change.focus
            {
                exa_ui::field_edit::focus_gained(&text, &mut view);
            }
            for (r, state) in form.records.iter_mut().zip(states) {
                r.state = state;
            }
        }
        self.journal_combo.view = Some(view);
        self.journal_combo.focused = focused;
        self.journal_combo.blink = 0;
        debug!("gui: the journal list's key focus: {focused}");
    }

    /// The journal list's pointer after the window manager's frame. A left
    /// press the list held in its edit state (its press answered 0x15,
    /// which moves the key focus to it) focuses it and drops the anchor
    /// at the caret under the pointer; any other press takes the focus
    /// off it. With the button held on it the caret follows the pointer
    /// and the selection grows from the anchor (0x30). While it has the
    /// focus its caret blinks (the record's caret style).
    pub fn journal_combo_pointer(&mut self, frame: &crate::form_input::PointerFrame, held: bool) {
        let Some(c) = self.journal_list_record() else {
            return;
        };
        let held_by_list = self
            .find(GameScreen::Journal)
            .and_then(|screen| self.task.form(screen.form))
            .and_then(|f| f.base())
            .is_some_and(|f| f.held == JOURNAL_LIST as i32 + 1);
        let at = frame.at;
        let editing = exa_ui::controls::combo_press(c.state) == exa_ui::controls::ComboPress::Edit;
        if frame.pressed {
            if held_by_list && editing {
                self.journal_combo_focus(
                    true,
                    exa_ui::controls::combo_cursor(
                        c.style,
                        at.map_or(0, |a| a.0) as u32,
                        (c.x, c.y, c.w, c.h),
                        c.margin,
                        self.journal_unit,
                    ),
                );
                if let Some(at) = at {
                    self.journal_caret(&c, at.0, true);
                }
            } else {
                self.journal_combo_focus(false, 0);
            }
        } else if held
            && held_by_list
            && self.journal_combo.focused
            && let Some(at) = at
        {
            self.journal_caret(&c, at.0, false);
        }
        if self.journal_combo.focused
            && let Some(form) = self
                .find(GameScreen::Journal)
                .and_then(|screen| self.task.form_mut(screen.form))
                .and_then(|f| f.base_mut())
        {
            exa_ui::form::caret_blink(c.style, &mut self.journal_combo.blink, &mut form.state);
        }
    }

    /// The list's caret under a pointer `x`: a press (0x40) drops the
    /// anchor with it, a drag (0x30) moves the caret only; the run then
    /// scrolls to keep it in view.
    fn journal_caret(&mut self, c: &Control, x: i32, press: bool) {
        let font = self.journal_font();
        let unit = self.journal_unit;
        let mut view = self.journal_view(&c.text);
        let run = exa_ui::controls::run_width(&c.text, view.first, view.length, &font.advances);
        let origin = exa_ui::controls::combo_text_origin(
            (c.x, c.y, c.w, c.h),
            c.margin,
            unit,
            c.style,
            run,
            font.ascent,
            font.glyph.1,
        );
        let span = exa_ui::controls::combo_text_span(c.w, c.margin, unit);
        if press {
            exa_ui::field_edit::press_caret(&font, &c.text, &mut view, origin.x, x, span);
        } else {
            exa_ui::field_edit::drag_caret(&font, &c.text, &mut view, origin.x, x, span);
        }
        self.journal_combo.view = Some(view);
        if let Some(form) = self
            .find(GameScreen::Journal)
            .and_then(|screen| self.task.form_mut(screen.form))
            .and_then(|f| f.base_mut())
        {
            form.state |= exa_ui::form::state::CARET_LIT;
        }
        self.journal_combo.blink = 0;
    }

    /// Whether the journal list holds the key focus while the window
    /// manager's focus is on the journal: the keys are then the list's
    /// key message (0xa0).
    pub fn journal_combo_focused(&self) -> bool {
        self.journal_combo.focused
            && self.focused() == Some(GameScreen::Journal)
            && !self.journal_box_focused()
    }

    /// A key message for the focused journal list: the combo box's key
    /// handler through the edit unit ([`exa_ui::field_edit::combo_key`]:
    /// Enter calls the owner with 0x300000, any other key edits with the
    /// box's limit and span and, the list having the notify style, calls
    /// it with 0x200000), then the list's callback with those bits: the
    /// edit state's arm renames the selected entry and its title in the
    /// list's titles ([`exa_ui::game_gui::Journal::list_changed`]).
    /// Answers the clipboard request.
    pub fn journal_combo_key(
        &mut self,
        key: exa_ui::field_edit::FieldKey,
        clipboard: Option<&[u8]>,
    ) -> Option<exa_ui::text_edit::EditAction> {
        let c = self.journal_list_record()?;
        let font = self.journal_font();
        let mut view = self.journal_view(&c.text);
        let mut text = c.text.clone();
        let effect = exa_ui::field_edit::combo_key(
            &font,
            exa_ui::field_edit::FieldWords {
                text: &mut text,
                view: &mut view,
                style: c.style,
                max_len: 255,
                has_callback: c.reports,
            },
            c.w,
            c.margin,
            self.journal_unit,
            key,
            clipboard,
        );
        self.journal_combo.view = Some(view);
        self.journal_combo.blink = 0;
        if let Some(form) = self
            .find(GameScreen::Journal)
            .and_then(|screen| self.task.form_mut(screen.form))
            .and_then(|f| f.base_mut())
        {
            form.state |= exa_ui::form::state::CARET_LIT;
            if let Some(list) = form.records.get_mut(JOURNAL_LIST) {
                list.text = text.clone();
            }
        }
        if let Some(bits) = effect.call {
            self.journal_list_changed(bits, &text);
        }
        effect.clipboard
    }

    /// The journal list's callback with the state bits its handler set:
    /// in the edit state (0x200000) the list's text becomes the selected
    /// entry's title and its row in the list's titles; otherwise the pick
    /// arm ([`GameScreens::journal_pick`]).
    fn journal_list_changed(&mut self, bits: u32, text: &[u8]) {
        if bits & exa_ui::field_edit::COMBO_EDITED == 0 {
            self.journal_pick(self.journal_row);
            return;
        }
        let title = crate::os_clipboard::ansi_to_text(text);
        let text_now = self.journal_text.clone();
        self.journal
            .list_changed(true, &title, self.journal_row, &text_now);
        if let Some(row) = self.journal_titles.get_mut(self.journal.current) {
            *row = text.to_vec();
        }
        debug!(
            "gui: journal entry {} renamed {title:?}",
            self.journal.current + 1
        );
    }

    /// Keeps the journal's text box on its journal (a title drag moves the
    /// journal; the box's place in it stays) and hands the box's lines to
    /// its form's scroll bar: the content height and one line's step, and
    /// the box's scroll target as the bar's fraction.
    pub fn sync_journal_box(&mut self) {
        let Some(journal) = self.find(GameScreen::Journal) else {
            return;
        };
        let Some(bounds) = self.task.form(journal.form).map(|f| f.bounds()) else {
            return;
        };
        let rect = ScreenLayout::journal(self.journal_unit).child_at(0, bounds);
        let Some(text_box) = self.journal_box.as_ref() else {
            return;
        };
        let (content, step, target) = (
            text_box.text.content_height,
            text_box.text.scroll_step,
            text_box.text.scroll_target,
        );
        if let Some(form) = self.task.form_mut(text_box.form).and_then(|f| f.base_mut()) {
            if form.bounds != rect {
                let (dx, dy) = (rect.left - form.bounds.left, rect.top - form.bounds.top);
                form.bounds = rect;
                form.in_owner = (form.in_owner.0 + dx, form.in_owner.1 + dy);
            }
            if let Some(scroll) = form.scroll.as_mut() {
                scroll.content = content;
                scroll.step = step;
                scroll.fraction = target;
            }
        }
    }

    /// The journal box's pointer after the window manager's frame: the
    /// focus moving onto it or off it (the box's focus slot and the form's
    /// focus change), a left press on its text placing the caret, a held
    /// press past the drag threshold starting the box's drag, the release
    /// ending it, the scroll bar's or the wheel's fraction becoming the
    /// box's scroll target, and the frame's tick (the caret's blink and the
    /// drag's follow).
    pub fn journal_pointer(
        &mut self,
        frame: &crate::form_input::PointerFrame,
        held: bool,
        shift: bool,
    ) {
        let focused_now = self.journal_box_focused();
        let task = &mut self.task;
        let Some(text_box) = self.journal_box.as_mut() else {
            return;
        };
        let events = task
            .form_mut(text_box.form)
            .and_then(|f| f.base_mut())
            .map(|f| std::mem::take(&mut f.events))
            .unwrap_or_default();
        for event in events {
            if let exa_ui::base_form::BaseEvent::Scrolled(f) = event {
                let delta = f - text_box.text.scroll_target;
                text_box.text.scroll_by(delta);
            }
        }
        if focused_now != text_box.focused {
            text_box.focused = focused_now;
            let word = if focused_now { u32::MAX } else { 0 };
            let current = if focused_now { 0 } else { u32::MAX };
            text_box.text.set_focus(word);
            exa_ui::form::focus_change(&mut [], &mut text_box.state, current, word);
            text_box.blink = 0;
        }
        let mods = exa_ui::text_box::Modifiers { shift, ctrl: false };
        if let Some(at) = frame.at {
            if frame.pressed && focused_now && text_box.over_text(task, at) {
                if let Some((x, y)) = text_box.content_point(task, at) {
                    text_box.text.press(x, y, JOURNAL_PRESS, false, mods);
                    text_box.light_caret();
                    text_box.press = Some((at, frame.now));
                }
            } else if held
                && let Some((from, stamp)) = text_box.press
                && !text_box.text.dragging
            {
                let elapsed = frame.now.wrapping_sub(stamp) as i32;
                let limit = exa_ui::gui_core::drag_threshold(elapsed, task.pointer.limit);
                if (at.0 - from.0).abs() > limit || (at.1 - from.1).abs() > limit {
                    let (x, y) = text_box.content_point(task, at).unwrap_or((0, 0));
                    text_box.text.press(
                        x,
                        y,
                        JOURNAL_PRESS | exa_ui::text_box::pointer::DRAG,
                        false,
                        mods,
                    );
                }
            }
            if text_box.text.dragging
                && let Some((x, y)) = text_box.content_point(task, at)
            {
                text_box.text.tick(x, y, frame.now);
            }
        }
        if (frame.released || !held) && (text_box.press.take().is_some() || text_box.text.dragging)
        {
            text_box.text.release();
        }
        if text_box.focused {
            exa_ui::form::caret_blink(text_box.style, &mut text_box.blink, &mut text_box.state);
            text_box.text.focused = text_box.state & exa_ui::form::state::CARET_LIT != 0;
        }
        text_box.text.ease_scroll();
    }

    /// A key for the focused journal box: `key` the original's key code
    /// (or, `typed`, a typed byte). The text is stored as the selected
    /// entry's when the box changed; Tab answers true (the box's callback
    /// turns combat mode on).
    pub fn journal_key(
        &mut self,
        key: u32,
        typed: bool,
        mods: exa_ui::text_box::Modifiers,
        clipboard: &mut dyn exa_ui::text_box::Clipboard,
    ) -> bool {
        let Some(text_box) = self.journal_box.as_mut() else {
            return false;
        };
        let events = text_box.text.key(key, typed, mods, clipboard);
        let mut tab = false;
        for event in events {
            match event {
                exa_ui::text_box::TextBoxEvent::Changed => {
                    text_box.light_caret();
                    self.journal_text =
                        crate::os_clipboard::ansi_to_text(text_box.text.text.as_bytes());
                }
                exa_ui::text_box::TextBoxEvent::Tab => tab = true,
                _ => {}
            }
        }
        self.sync_journal_box();
        tab
    }

    /// The player's inventory, as the inventory key asks: an active one
    /// closes.
    pub fn open_inventory(
        &mut self,
        player_ready: bool,
        player_alive: bool,
        unit: i32,
    ) -> ScreenAction {
        let player = self.player;
        self.show_inventory(player_ready, player_alive, unit, true, player)
    }

    /// The body whose possessions and look the open inventory shows: the
    /// bound character's, else `player` (a window opened without its
    /// opener's binding, as tests do).
    pub fn inventory_bound(&self, player: Option<Entity>) -> Option<Entity> {
        self.inventory_body.or(player)
    }

    /// The pack panel of the player's own open inventory, as the game
    /// unit's crystal drain asks the character view for it: the first open
    /// view whose character is the player answers its pack, and none is
    /// answered while no inventory is open or the open one shows another
    /// character.
    pub fn player_pack_mut(&mut self) -> Option<&mut Container> {
        (self.inventory_character()? == self.player).then_some(&mut self.pack)
    }

    /// The character the open inventory shows, if one is open.
    pub fn inventory_character(&self) -> Option<i32> {
        self.find(GameScreen::Inventory)
            .map(|_| self.inventory_owner)
    }

    /// A character's inventory (`owner` its record id; the player's own or
    /// another's): the show rule, then a new one is registered as an open
    /// container under the character's tag and placed by
    /// [`inventory_place`]: the player's at its remembered place or its
    /// constructor's (8, 8), another character's at the screen's right
    /// edge less its width and four units. `toggle` closes an inventory
    /// that is already the active form; a script's `ShowInventory` passes
    /// false and leaves it.
    ///
    /// The original keeps one inventory window per character (its finder
    /// matches the window's character), so the player's and a companion's
    /// can be open together. Exarust has one inventory window: asking for
    /// another character's closes the open one first [Exarust stand-in].
    pub fn show_inventory(
        &mut self,
        player_ready: bool,
        player_alive: bool,
        unit: i32,
        toggle: bool,
        owner: i32,
    ) -> ScreenAction {
        let mut open = self.find(GameScreen::Inventory);
        if open.is_some() && self.inventory_owner != owner && player_ready && player_alive {
            self.close(GameScreen::Inventory);
            open = None;
        }
        let action = inventory_screen(InventoryInput {
            player_ready,
            player_mode_ok: player_alive,
            open: open.is_some(),
            filling: false,
            // The opener's test is the base form's: the window is its
            // task's topmost child, whatever the pointer hovers.
            active: open.is_some_and(|screen| self.task.is_active(screen.form)),
            toggle,
        });
        match (action, open) {
            (ScreenAction::Opened, _) => {
                self.inventory_unit = unit.max(1);
                self.inventory_owner = owner;
                let layout = self.inventory_layout();
                let (width, height) = layout.size;
                let (x, y) = inventory_place(
                    owner == self.player,
                    self.inventory_place,
                    self.task.screen.0,
                    width,
                    unit,
                    layout.place,
                );
                // The open keeps the window on the screen (the GUI's
                // clamp: x within the root's width, y at least a unit).
                let (x, y) =
                    exa_ui::gui_core::clamp_form(x, y, width, unit.max(1), self.task.screen.0);
                // The pack is built for the unit: its icons are forty units
                // and its saved places sixteenths of one (the open binds
                // the carried list after this).
                self.pack.unit = self.inventory_unit;
                self.pack.icon_size = exa_items::icon_cache::icon_size(self.inventory_unit);
                self.pack.client = (layout.pack.width(), layout.pack.height());
                self.pack.owner = Some(owner as u32);
                // The pack's linked panel is the character view: a drag
                // onto the figure leaves the pack without losing its bulk.
                self.pack.linked = Some(crate::doll_view::DOLL_PANEL);
                self.containers.open(tagged_container(owner));
                self.push(
                    GameScreen::Inventory,
                    GuiRect::new(x, y, width, height),
                    self.inventory_unit,
                );
                // The new inventory's show ends with the forced pointer
                // update: the focus goes to the form under the pointer.
                self.task.refresh_pointer();
            }
            (ScreenAction::Focused, Some(screen)) => {
                self.task.activate(screen.form);
            }
            (ScreenAction::Closed, _) => {
                self.close(GameScreen::Inventory);
            }
            _ => {}
        }
        action
    }

    /// The journal: the show rule, then a new one at its remembered place
    /// or the unset place in the screen (4, 36), on its first entry.
    pub fn open_journal(&mut self, player_ready: bool, player_alive: bool) -> ScreenAction {
        self.open_journal_at(player_ready, player_alive, 4)
    }

    /// [`GameScreens::open_journal`] laid out at the GUI unit `unit`
    /// ([`ScreenLayout::journal`]).
    pub fn open_journal_at(
        &mut self,
        player_ready: bool,
        player_alive: bool,
        unit: i32,
    ) -> ScreenAction {
        // The opener searches the task's children by the journal's tag (a
        // hidden journal is not found) and closes it only when it is the
        // topmost child.
        let open = self
            .task
            .find_by_tag(exa_ui::game_gui::JOURNAL_SCREEN)
            .and_then(|form| self.open.iter().copied().find(|s| s.form == form));
        let action = journal_screen(JournalInput {
            player_ready,
            player_mode_ok: player_alive,
            open: open.is_some(),
            active: open.is_some_and(|screen| self.task.is_active(screen.form)),
        });
        match (action, open) {
            (ScreenAction::Opened, _) => {
                self.journal_unit = unit.max(1);
                let (width, height) = ScreenLayout::journal(self.journal_unit).size;
                let (x, y) = self.journal.place.unwrap_or_else(|| {
                    place_unset(
                        UNPLACED,
                        UNPLACED,
                        width,
                        height,
                        0x3000,
                        self.root_client(),
                    )
                });
                let (x, y) = exa_ui::gui_core::clamp_form(
                    x,
                    y,
                    width,
                    self.journal_unit,
                    self.task.screen.0,
                );
                self.journal.ensure_entry();
                self.journal.select(0);
                self.journal_text = self.journal.entries[0].text.clone();
                let form = self.push(
                    GameScreen::Journal,
                    GuiRect::new(x, y, width, height),
                    self.journal_unit,
                );
                self.open_journal_box(form);
                self.journal_selected();
                // The opener ends with the forced pointer update: the focus
                // goes to the form under the pointer.
                self.task.refresh_pointer();
            }
            (ScreenAction::Focused, Some(screen)) => {
                self.task.activate(screen.form);
            }
            (ScreenAction::Closed, _) => {
                self.close(GameScreen::Journal);
            }
            _ => {}
        }
        action
    }

    /// The loot window for a body: refused for a body whose record flag
    /// word has its second byte 3; an existing window on that body is
    /// activated, else a new one opens at x 400 with an unset y, enters the
    /// open-container registry under the body's tagged id, and takes an
    /// empty pack and equipment panel until the body's items are bound.
    /// Another body's window is not reused, as the original matches the
    /// window's character as well as its tag.
    pub fn open_loot(&mut self, character: i32, flags: u32, unit: i32) -> ScreenAction {
        let existing = self
            .loot
            .iter()
            .find(|window| window.character == character)
            .map(|window| window.form);
        let action = loot_screen(flags, existing.is_some());
        match action {
            ScreenAction::Focused => {
                if let Some(form) = existing {
                    self.task.activate(form);
                }
            }
            ScreenAction::Opened => {
                let layout = ScreenLayout::loot(unit.max(1));
                let (width, height) = layout.size;
                let (x, y) = place_unset(
                    layout.place.0,
                    layout.place.1,
                    width,
                    height,
                    gui_paint::SCREEN_STYLE,
                    self.root_client(),
                );
                self.containers.open(tagged_container(character));
                let form = self.push(
                    GameScreen::Loot,
                    GuiRect::new(x, y, width, height),
                    unit.max(1),
                );
                self.loot
                    .push(LootWindow::new(form, character, flags, unit.max(1)));
            }
            _ => {}
        }
        action
    }

    /// The open popup of a container, if any.
    pub fn popup(&self, id: u32) -> Option<&ContainerPopup> {
        self.popups.iter().find(|popup| popup.id == id)
    }

    /// Opens a container's popup, as the original's opener does: a popup
    /// already open on the same container is brought forward (`Focused`);
    /// otherwise the popup is registered as an open container, its panel
    /// sized from the record ([`open_container`] with the panel's border)
    /// six units plus the icon area below its top, the form sized around
    /// it ([`popup_form_size`]) and placed on the pointer
    /// ([`popup_place`]). The panel then loads the record's children
    /// (`resolve` gives each child's icon and bulk, as
    /// [`Container::load_record`] takes it). `icon` is the container's own
    /// icon handle and `icon_half` its half height.
    #[allow(clippy::too_many_arguments)]
    pub fn open_container_popup<F>(
        &mut self,
        object: Entity,
        id: u32,
        record: &ContainerRecord,
        pointer: (i32, i32),
        unit: i32,
        icon: usize,
        icon_half: i32,
        resolve: F,
    ) -> ScreenAction
    where
        F: FnMut(u32, u32) -> Option<(usize, i32)>,
    {
        if id == 0 {
            return ScreenAction::Refused;
        }
        if let Some(form) = self.popup(id).map(|popup| popup.form) {
            self.task.focus_set(Some(form), NO_CONTROL);
            return ScreenAction::Focused;
        }
        let mut panel = Container::form(unit);
        let icon_height = popup_icon_height(icon_half, panel.icon_size);
        let Some(plan) = open_container(
            false,
            Some(record.size),
            unit,
            POPUP_PANEL_BORDER,
            icon_height,
        ) else {
            return ScreenAction::Refused;
        };
        self.containers.open(id as i32);
        let size = popup_form_size(plan, unit, POPUP_BORDER);
        let (x, y) = popup_place(pointer, size, self.task.screen.0, unit);
        let form = self.push(
            GameScreen::Container,
            GuiRect::new(x, y, size.0, size.1),
            unit,
        );
        panel.client = (
            plan.inner.0 - 2 * POPUP_PANEL_BORDER,
            plan.inner.1 - 2 * POPUP_PANEL_BORDER,
        );
        panel.load_record(id, record, resolve);
        self.popups.push(ContainerPopup {
            form,
            id,
            object,
            stored: ContainerRecord {
                children: Vec::new(),
                ..record.clone()
            },
            panel,
            plan,
            icon,
            icon_height,
        });
        ScreenAction::Opened
    }

    /// Closes the popup of a container, whichever window is focused.
    pub fn close_popup(&mut self, id: u32) -> bool {
        let Some(form) = self.popup(id).map(|popup| popup.form) else {
            return false;
        };
        let Some(index) = self.open.iter().position(|screen| screen.form == form) else {
            return false;
        };
        self.close_at(index)
    }

    /// The open loot window on a body, if any.
    pub fn loot_window(&self, character: i32) -> Option<&LootWindow> {
        self.loot
            .iter()
            .find(|window| window.character == character)
    }

    /// Binds a body's items to its loot window: the pack's entries and what
    /// it wears and holds. The original loads both panels from the body's
    /// record when the window is created; the running game has no body
    /// inventory yet, so the caller supplies them (an empty body leaves the
    /// panels as they are).
    pub fn bind_loot(&mut self, character: i32, pack: Container, worn: Container) -> bool {
        let Some(window) = self
            .loot
            .iter_mut()
            .find(|window| window.character == character)
        else {
            return false;
        };
        window.pack = pack;
        window.worn = worn;
        true
    }

    /// The bodies whose loot window the deferred close takes, in the open
    /// windows' order (the original walks the GUI's forms): a window whose
    /// body's mark 0x80000 is set or whose condition is no longer dead
    /// (other than 0xa00). `bodies` gives each body's (id, flags, mode); a
    /// window whose body is not listed stays.
    pub fn loot_windows_to_close(&self, bodies: &[(i32, u32, u32)]) -> Vec<i32> {
        self.open
            .iter()
            .filter(|screen| screen.kind == GameScreen::Loot)
            .filter_map(|screen| self.loot.iter().find(|w| w.form == screen.form))
            .filter_map(|window| {
                bodies
                    .iter()
                    .find(|(character, _, _)| *character == window.character)
            })
            .filter(|(_, flags, mode)| exa_ui::game_gui::closes_loot_window(*flags, *mode))
            .map(|(character, _, _)| *character)
            .collect()
    }

    /// Closes a screen: its form closes and the task deletes it; the
    /// journal stores its text (its save) and the screens remember their
    /// place; the inventory leaves the container registry; a loot window
    /// leaves it under its body's id (the original also saves the body's
    /// pack, which has no Exarust store yet).
    /// The focus falls to the last screen left. The focused window of the
    /// kind is closed when there are several (the loot windows of two
    /// bodies).
    pub fn close(&mut self, kind: GameScreen) -> bool {
        let focused = self.task.active.map(|form| self.task.root_of(form));
        let index = self
            .open
            .iter()
            .position(|screen| screen.kind == kind && Some(screen.form) == focused)
            .or_else(|| self.open.iter().position(|screen| screen.kind == kind));
        let Some(index) = index else {
            return false;
        };
        self.close_at(index)
    }

    /// Closes one open screen by its position in the open list.
    fn close_at(&mut self, index: usize) -> bool {
        let screen = self.open.remove(index);
        let place = self.task.form(screen.form).map(|form| {
            let b = form.bounds();
            (b.left, b.top)
        });
        // The base form's close: the window's child forms (the journal's
        // text box) close with it and the window manager forgets them.
        self.task.close_form(screen.form);
        match screen.kind {
            GameScreen::Journal => {
                self.journal_box = None;
                self.journal_list = None;
                self.journal_combo = JournalCombo::default();
                self.journal.store_text(&self.journal_text);
                self.journal.place = place;
            }
            GameScreen::Inventory => {
                self.containers
                    .close(tagged_container(self.inventory_owner));
                // Only the player's own window remembers its place.
                if self.inventory_owner == self.player {
                    self.inventory_place = place;
                }
                // The panel's destroy releases every icon reference before
                // its entry array goes.
                let unit = self.pack.unit;
                let mut released = Vec::new();
                self.pack.release_all_icons(|icon| released.push(icon));
                self.released_icons.extend(released);
                self.pack = Container::form(unit);
                self.inventory_body = None;
                self.inventory_mind_window = false;
            }
            GameScreen::Loot => {
                if let Some(window) = self
                    .loot
                    .iter()
                    .position(|window| window.form == screen.form)
                    .map(|index| self.loot.remove(index))
                {
                    self.containers.close(tagged_container(window.character));
                    let (mut pack, mut worn) = (window.pack, window.worn);
                    let mut released = Vec::new();
                    pack.release_all_icons(|icon| released.push(icon));
                    worn.release_all_icons(|icon| released.push(icon));
                    self.released_icons.extend(released);
                }
            }
            GameScreen::Container => {
                // The container close: the game forgets the open container,
                // the panel saves its entries into the record (releasing
                // their icons), its id is cleared and the card's close
                // releases the container's own icon.
                if let Some(mut popup) = self
                    .popups
                    .iter()
                    .position(|popup| popup.form == screen.form)
                    .map(|index| self.popups.remove(index))
                {
                    let close = exa_ui::popups::container_close(false);
                    if close.notify_game {
                        self.containers.close(popup.id as i32);
                    }
                    if close.save_contents
                        && let Some(saved) = popup.panel.save_record(&popup.stored)
                    {
                        self.saved_containers.push((popup.object, popup.id, saved));
                    }
                    let mut released = Vec::new();
                    popup.panel.release_all_icons(|icon| released.push(icon));
                    if close.clear_id {
                        popup.panel.container_id = 0;
                    }
                    if close.run_base && exa_ui::popups::desc_close(popup.icon != 0) {
                        released.push(popup.icon);
                    }
                    self.released_icons.extend(released);
                }
            }
            _ => {}
        }
        // The focus went to the form under the pointer: the window's hide
        // ends with the forced pointer update (`GuiTask::close_form`).
        true
    }

    /// The screen whose window is its task's active child (the topmost by
    /// the openers' test, [`GuiTask::is_active`]).
    pub fn active_screen(&self) -> Option<GameScreen> {
        self.open
            .iter()
            .rev()
            .find(|screen| self.task.is_active(screen.form))
            .map(|screen| screen.kind)
    }

    /// Closes the focused screen (the one under the pointer), else the
    /// active one, if any (Escape; Exarust's).
    pub fn close_focused(&mut self) -> bool {
        let focused = self.task.active.map(|form| self.task.root_of(form));
        let index = self
            .open
            .iter()
            .position(|screen| Some(screen.form) == focused)
            .or_else(|| {
                self.open
                    .iter()
                    .rposition(|screen| self.task.is_active(screen.form))
            });
        index.is_some_and(|index| self.close_at(index))
    }

    /// Closes the loot window of one body (the deferred close), whichever
    /// window is focused.
    pub fn close_loot(&mut self, character: i32) -> bool {
        let Some(form) = self
            .loot
            .iter()
            .find(|window| window.character == character)
            .map(|window| window.form)
        else {
            return false;
        };
        let Some(index) = self.open.iter().position(|screen| screen.form == form) else {
            return false;
        };
        self.close_at(index)
    }

    /// Closes the container and journal family (the inventory, container,
    /// journal and loot windows), front to back.
    pub fn close_container_family(&mut self) -> usize {
        self.close_tagged(CONTAINER_FAMILY, true)
    }

    /// Saves what the open screens hold, front to back, without closing
    /// them (what every game save does first): an open container popup
    /// saves its panel into its record (queued in
    /// [`Self::saved_containers`]) and the journal stores its text. The
    /// inventory's and a loot window's loaded panels ask for their save
    /// too, which the save system writes into the bodies' carried lists
    /// ([`body_inventory::store_pack`]). Returns the saves asked for.
    pub fn save_open_screens(&mut self) -> Vec<exa_ui::game_gui::SaveAction> {
        use exa_ui::game_gui::{SaveAction, save_action};
        let mut actions = Vec::new();
        for index in 0..self.open.len() {
            let screen = self.open[index];
            let Some(tag) = screen.kind.tag() else {
                continue;
            };
            let loaded = match screen.kind {
                GameScreen::Inventory => self.pack.loaded,
                GameScreen::Loot => self
                    .loot
                    .iter()
                    .find(|window| window.form == screen.form)
                    .is_some_and(|window| window.pack.loaded),
                _ => false,
            };
            let Some(action) = save_action(tag, loaded) else {
                continue;
            };
            match action {
                SaveAction::Container => {
                    if let Some(popup) = self.popups.iter().find(|p| p.form == screen.form)
                        && let Some(saved) = popup.panel.save_record(&popup.stored)
                    {
                        self.saved_containers.push((popup.object, popup.id, saved));
                    }
                }
                SaveAction::Journal => self.journal.store_text(&self.journal_text),
                SaveAction::Inventory | SaveAction::Loot => {}
            }
            actions.push(action);
        }
        actions
    }

    /// Closes every in-game screen (the whole 0xce1 family), back to front:
    /// what entering combat mode and the session's ends do.
    pub fn close_game_screens(&mut self) -> usize {
        self.close_tagged(GAME_SCREEN_FAMILY, false)
    }

    /// Closes the open screens whose tag has all of `family`'s bits, in
    /// the order [`forms_with_tag`] gives, each by its own form (the
    /// original calls each form's close in turn).
    fn close_tagged(&mut self, family: u32, front_to_back: bool) -> usize {
        let tags: Vec<u32> = self
            .open
            .iter()
            .map(|screen| screen.kind.tag().unwrap_or(0))
            .collect();
        let forms: Vec<FormId> = forms_with_tag(&tags, family, front_to_back)
            .into_iter()
            .map(|i| self.open[i].form)
            .collect();
        let mut closed = 0;
        for form in forms {
            if let Some(index) = self.open.iter().position(|screen| screen.form == form)
                && self.close_at(index)
            {
                closed += 1;
            }
        }
        closed
    }
}

/// Which screens a GUI sweep closes before the power screen's part.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenSweep {
    /// The container and journal family; the shown power bar falls too
    /// (Control Mind's handover and hand-back).
    ContainerFamily,
    /// Every in-game screen; the bar stays (entering combat mode).
    Every,
    /// Every in-game screen, and the shown power bar falls too (the spirit
    /// form's start).
    EveryAndBar,
}

impl ScreenSweep {
    /// The bar bit the sweep hands the power screen's part.
    pub fn bar_mask(self) -> u8 {
        match self {
            ScreenSweep::ContainerFamily | ScreenSweep::EveryAndBar => 1,
            ScreenSweep::Every => 0,
        }
    }
}

/// The GUI unit's screen sweeps: the screens of `sweep` close, then the
/// power tree closes (or, with the sweep's bar bit, the shown bar falls)
/// through [`crate::power_screen::sweep_forms`]. Run on the world holding
/// the screens, else after the fixed step on the frame's.
pub fn sweep_screens(world: &mut World, sweep: ScreenSweep) {
    if let Some(mut screens) = world.get_resource_mut::<GameScreens>() {
        let closed = match sweep {
            ScreenSweep::ContainerFamily => screens.close_container_family(),
            ScreenSweep::Every | ScreenSweep::EveryAndBar => screens.close_game_screens(),
        };
        if closed > 0 {
            info!("GUI sweep {sweep:?}: {closed} screen(s) closed");
        }
        crate::power_screen::sweep_forms(world, sweep.bar_mask());
    } else if let Some(mut frame) = world.get_resource_mut::<exa_physics::sim_app::FrameCommands>()
    {
        frame.send(move |world: &mut World| sweep_screens(world, sweep));
    } else {
        crate::power_screen::sweep_forms(world, sweep.bar_mask());
    }
}

pub struct GameScreensPlugin;

/// Gives the live object database the item factory's pass its migrations
/// run (a level's local tier, a save's world tier), when the game's data is
/// open as the plugin is built.
fn install_factory_hook(app: &mut App) {
    let Some(game) = app
        .world()
        .get_resource::<exa_assets::Game>()
        .map(|g| g.0.clone())
    else {
        return;
    };
    app.init_resource::<exa_assets::live_objects::LiveObjects>();
    let mut live = app
        .world_mut()
        .resource_mut::<exa_assets::live_objects::LiveObjects>();
    if live.factory().is_none() {
        live.set_factory(crate::shop_stock::factory_hook(game));
    }
}

impl Plugin for GameScreensPlugin {
    fn build(&self, app: &mut App) {
        install_factory_hook(app);
        crate::gui_keys::ensure(app);
        app.init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .init_resource::<DebugGuiForm>()
            .init_resource::<DebugOpenContainer>()
            .init_resource::<DebugOpenLoot>()
            .init_resource::<GuiFrames>()
            // The item panel's container popups read and write the item
            // records; init_resource keeps the table another plugin made.
            .init_resource::<crate::doll_view::DollItems>()
            .init_resource::<exa_assets::live_objects::LiveObjects>()
            .add_message::<ScreenRequest>()
            .add_message::<exa_script::ScriptEffect>()
            .add_message::<ContainerRequest>()
            .add_systems(
                Update,
                (
                    sync_screen_size.after(crate::parchment::open_parchment),
                    rescale_open_screens,
                    register_level_items,
                    crate::container_fill::fill_level_containers,
                    show_script_inventory,
                    debug_open_loot,
                    open_requested_screens.after(crate::gui_drop::run_gui_drops),
                    debug_open_container.after(store_saved_containers),
                    open_requested_containers,
                )
                    .chain()
                    .after(crate::hotkeys::HotkeySet)
                    .before(crate::player::player_combat)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .init_resource::<DebugRecordHint>()
            .add_message::<bevy::input::mouse::MouseWheel>()
            .add_systems(
                Update,
                screens_pointer
                    .after(open_requested_screens)
                    .after(close_on_right_release)
                    .after(crate::player::cursor_look)
                    .before(draw_screens)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                swap_from_page_button
                    .after(screens_pointer)
                    .before(exa_items::catalogue::swap_weapon_sets)
                        .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                draw_screens
                    .after(open_requested_screens)
                    .after(crate::item_icon_render::IconRenderSystems)
                        .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                (close_on_right_release
                    .after(close_changed_loot_windows), close_changed_loot_windows)
                    .after(open_requested_screens)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_message::<bevy::window::WindowFocused>()
            .add_systems(Update, deactivate_on_focus_loss
                .after(screens_keys)
                .before(screens_pointer)
                .in_set(exa_physics::FramePhase::Gui))
            .add_message::<bevy::input::keyboard::KeyboardInput>()
            .add_systems(
                Update,
                (journal_typing.after(crate::item_card::figure_cards), screens_keys)
                    .before(crate::hotkeys::HotkeySet)
                    .before(open_requested_screens)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                inventory_follows_the_set
                    .after(open_requested_containers)
                    .after(store_saved_containers)
                    .after(exa_items::catalogue::swap_weapon_sets)
                    .after(open_requested_screens)
                    .before(draw_screens)
                    .in_set(exa_physics::FramePhase::Gui),
            )
            .add_systems(
                Update,
                (release_closed_icons
                    .after(swap_from_page_button), store_saved_containers
                    .after(release_closed_icons))
                    .after(close_on_right_release)
                    .after(close_changed_loot_windows)
                    .in_set(exa_physics::FramePhase::Gui),
            );
    }
}

/// The inventory's page buttons (records 2 and 3, Primary and Secondary).
const PAGE_BUTTONS: [usize; 2] = [2, 3];

/// The page buttons' record callback, after the pointer's press: the
/// character the inventory shows is asked to swap its weapon sets, as the
/// swap key asks ([`exa_items::possession::Possessions::swap_request`]): a
/// busy body (state bits 0x10202) or an item drag in flight refuses it,
/// and with its inventory open the sets swap at once. The open window
/// then lights the other button and loads its pack again
/// ([`inventory_follows_the_set`]).
#[allow(clippy::type_complexity)]
pub(crate) fn swap_from_page_button(
    mut commands: Commands,
    mut screens: ResMut<GameScreens>,
    drag: Option<Res<crate::panel_live::PanelDrag>>,
    players: Query<Entity, With<crate::player::PlayerBody>>,
    bodies: Query<Option<&exa_combat::Condition>, With<exa_items::possession::Possessions>>,
) {
    if !std::mem::take(&mut screens.page_swap) {
        return;
    }
    let Some(body) = screens
        .find(GameScreen::Inventory)
        .and_then(|_| screens.inventory_bound(players.iter().next()))
    else {
        return;
    };
    let Ok(condition) = bodies.get(body) else {
        return;
    };
    let word = condition.map_or(0, |c| c.0);
    let dragging = drag.is_some_and(|d| d.source.is_some());
    let answer = exa_items::possession::Possessions::swap_request(word, dragging, true);
    info!("gui: the page button's swap for {body:?}: {answer:?}");
    if answer == exa_items::possession::SwapRequest::Now {
        commands
            .entity(body)
            .insert(exa_items::catalogue::SwapRequested);
    }
}

/// The open inventory follows its character's weapon set. The fill lights
/// the page button of the set the record has out
/// ([`exa_ui::game_gui::inventory_fill`]'s tab states), and a swap of the
/// shown body's sets while the window is open lights the other one and
/// loads the pack again from the record, whose held marks the swap moved
/// (the original's swap does both through the body's open view). The
/// pack's old icon references are given back before the load takes new
/// ones.
#[allow(clippy::type_complexity)]
pub(crate) fn inventory_follows_the_set(
    mut screens: ResMut<GameScreens>,
    mut icons: Option<ResMut<PanelIcons>>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    bodies: Query<&exa_items::possession::Possessions>,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
    mut seen: Local<Option<(Entity, bool)>>,
) {
    let Some(open) = screens.find(GameScreen::Inventory) else {
        *seen = None;
        return;
    };
    let Some((body, p)) = screens
        .inventory_bound(player_bodies.iter().next())
        .and_then(|body| bodies.get(body).ok().map(|p| (body, p)))
    else {
        return;
    };
    let second = exa_items::inventory_window::second_tab(p);
    let states = exa_ui::game_gui::inventory_tab_states(p.flags);
    if let Some(base) = screens
        .task
        .form_mut(open.form)
        .and_then(|form| form.base_mut())
    {
        for (record, state) in PAGE_BUTTONS.into_iter().zip(states) {
            if let Some(button) = base.records.get_mut(record)
                && button.state != state
            {
                button.state = state;
            }
        }
    }
    if let Some((was_body, was)) = *seen
        && was_body == body
        && was != second
        && let Some(icons) = icons.as_deref_mut()
    {
        icons.release_container(&mut screens.pack);
        let loaded =
            body_inventory::bind_inventory_pack(&mut screens.pack, p, icons, catalogue.as_deref());
        info!(
            "GUI: the shown body's sets swapped ({} set out): the pack loads {loaded} entries again",
            if second { "second" } else { "first" }
        );
    }
    *seen = Some((body, second));
}

/// The application's deactivation (the window procedure's focus loss): the
/// window managers' held buttons release, their pointer shape and capture
/// clear, and their focused forms deselect with the focus on their root,
/// remembered ([`exa_ui::gui_core::GuiTask::focus_lost`]); the focus's
/// return goes back to the remembered form
/// ([`exa_ui::gui_core::GuiTask::focus_regained`]). The in-game screens, the
/// hub's windows, the main menu and the power tree hang from their own
/// tasks in Exarust; each is told, as the original's one GUI is.
pub(crate) fn deactivate_on_focus_loss(
    mut focus: MessageReader<bevy::window::WindowFocused>,
    screens: Option<ResMut<GameScreens>>,
    hub: Option<ResMut<crate::hub_windows::HubWindows>>,
    menu: Option<ResMut<crate::menu_screen::MenuRouter>>,
    tree: Option<ResMut<crate::power_screen::PowerRouter>>,
) {
    // The last focus message of the frame decides (a loss then a gain in
    // one frame runs both, in order).
    let events: Vec<bool> = focus.read().map(|event| event.focused).collect();
    if events.is_empty() {
        return;
    }
    let mut tasks: Vec<&mut exa_ui::gui_core::GuiTask> = Vec::new();
    let (mut screens, mut hub, mut menu, mut tree) = (screens, hub, menu, tree);
    if let Some(screens) = screens.as_mut() {
        tasks.push(&mut screens.task);
    }
    if let Some(hub) = hub.as_mut() {
        tasks.push(&mut hub.task);
    }
    if let Some(menu) = menu.as_mut() {
        tasks.push(&mut menu.task);
    }
    if let Some(tree) = tree.as_mut() {
        tasks.push(&mut tree.task);
    }
    for focused in events {
        for task in tasks.iter_mut() {
            if focused {
                task.focus_regained();
            } else {
                task.focus_lost();
            }
        }
    }
}

/// Opens the popups the double click asked for: the item card's request is
/// closed first (a pending card is cancelled and a shown one fades out),
/// then the object's container section is loaded into a new popup at the
/// pointer. An object without the section opens nothing.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_requested_containers(
    mut requests: MessageReader<ContainerRequest>,
    objects: Query<&exa_world::ObjectSections>,
    mut items: ResMut<crate::doll_view::DollItems>,
    game: Option<Res<exa_assets::Game>>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut icons: Option<ResMut<PanelIcons>>,
    mut card: Option<ResMut<crate::item_card::CardView>>,
    time: Option<Res<Time>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
) {
    items.ensure(game.as_deref());
    for request in requests.read() {
        let placed = objects
            .get(request.object)
            .ok()
            .and_then(|sections| sections.container.clone());
        let from_item = (request.object == Entity::PLACEHOLDER)
            .then(|| {
                items
                    .get(request.id)
                    .and_then(|item| item.container.clone())
            })
            .flatten();
        let Some(section) = placed.or(from_item) else {
            continue;
        };
        let Some(icons) = icons.as_deref_mut() else {
            continue;
        };
        screens.card.timer.cancel();
        if let Some(shown) = card.as_deref_mut().and_then(|card| card.shown.as_mut()) {
            let now = time
                .as_deref()
                .map_or(0, |t| t.elapsed().as_millis() as u32);
            shown.fade.fade_out(now);
        }
        let unit = windows
            .single()
            .ok()
            .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
            .unwrap_or(4);
        let record = container_record(&section);
        let (icon, (_, icon_half)) = icons.reserve(request.id, 0, catalogue.as_deref());
        let catalogue = catalogue.as_deref();
        let action = screens.open_container_popup(
            request.object,
            request.id,
            &record,
            (request.pointer.x, request.pointer.y),
            unit,
            icon,
            icon_half,
            |id, flags| {
                // An empty slot (id 0, the fill's `0x100`) has no item to
                // list. [guess: the panel's resolver finds no record for id
                // 0, as its database lookup does]
                if id == 0 {
                    return None;
                }
                // A child the acquire gives no icon is not listed, as the
                // list load skips a failed resolve.
                let (slot, _) = icons.reserve(id, flags, catalogue);
                if slot == 0 && catalogue.is_some() {
                    return None;
                }
                let bulk = catalogue.and_then(|c| c.bulk(id)).map_or(0, i32::from);
                Some((slot, bulk))
            },
        );
        if action != ScreenAction::Opened && icon != 0 {
            icons.release(icon);
        }
        // The open's last step keeps every loaded child inside the panel,
        // each by its icon's half extents, as the original clamps after
        // the record load.
        if action == ScreenAction::Opened
            && let Some(popup) = screens.popups.iter_mut().find(|p| p.id == request.id)
        {
            popup.panel.clamp_all(|slot| icons.half(slot));
        }
        debug!("GUI: container {:#x} {action:?}", request.id);
        // Each child's id, model and pixel rect, for the smoke runs.
        if action == ScreenAction::Opened
            && let Some(popup) = screens.popup(request.id)
            && let Some(form) = screens.task.form(popup.form)
        {
            let client = panel_client(popup.panel_rect(form.bounds()));
            for entry in &popup.panel.entries {
                let model = catalogue
                    .and_then(|c| c.model(entry.id))
                    .map(|m| m.model.as_str());
                debug!(
                    "GUI: container child {:#x} flags {:#x} model {model:?} icon slot {} \
                     rendered {} at ({}, {}) {}x{} px",
                    entry.id,
                    entry.flags,
                    entry.icon,
                    icons.rendered(entry.icon),
                    client.left + entry.cell.x,
                    client.top + entry.cell.y,
                    popup.panel.icon_size,
                    popup.panel.icon_size
                );
            }
        }
    }
}

/// Exarust's debug helper (`--open-container FRAME`): this many frames after
/// the level load has finished and a player is present, the placed container nearest the player is
/// opened in its popup at the window's centre, as the double click would.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugOpenContainer(pub Option<u32>);

/// Runs [`DebugOpenContainer`]: counts frames once a player is present and,
/// at the frame asked for, requests the nearest container's popup.
fn debug_open_container(
    debug: Option<Res<DebugOpenContainer>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    objects: Query<(
        Entity,
        &exa_world::PlacedItem,
        &exa_world::ObjectSections,
        &GlobalTransform,
    )>,
    windows: Query<&Window, With<PrimaryWindow>>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut requests: MessageWriter<ContainerRequest>,
) {
    let Some(at) = debug.and_then(|debug| debug.0) else {
        return;
    };
    // The frames count once the level load (and so the containers' fill)
    // has finished.
    if loading.is_some() {
        return;
    }
    let Some(player) = players.iter().next() else {
        return;
    };
    *frames += 1;
    if *frames != at {
        return;
    }
    let feet = player.translation();
    let nearest = objects
        .iter()
        .filter(|(_, _, sections, _)| sections.container.is_some())
        .min_by(|a, b| {
            let da = a.3.translation().distance_squared(feet);
            let db = b.3.translation().distance_squared(feet);
            da.total_cmp(&db)
        });
    let Some((object, placed, sections, transform)) = nearest else {
        info!("open-container: no placed container in the level");
        return;
    };
    let pointer = windows
        .single()
        .map(|window| {
            IVec2::new(
                window.physical_width() as i32 / 2,
                window.physical_height() as i32 / 2,
            )
        })
        .unwrap_or(IVec2::new(640, 360));
    let (cells, items) = sections
        .container
        .as_ref()
        .map_or(((0, 0), 0), |c| ((c.width, c.height), c.children.len()));
    info!(
        "open-container: {:#x} at {:.2} m, {}x{} cells, {items} items",
        placed.tagged_id,
        transform.translation().distance(feet),
        cells.0,
        cells.1
    );
    requests.write(ContainerRequest {
        object,
        id: placed.tagged_id,
        pointer,
    });
}

/// Exarust's debug helper (`--open-loot FRAME`): this many frames after the
/// level load has finished and a player is present, the loot window of the
/// dead body nearest the player opens, as the double click on it would.
#[derive(Resource, Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DebugOpenLoot(pub Option<u32>);

/// Runs [`DebugOpenLoot`]: counts frames once a player is present and, at
/// the frame asked for, requests the nearest dead body's loot window.
pub(crate) fn debug_open_loot(
    debug: Option<Res<DebugOpenLoot>>,
    players: Query<&GlobalTransform, With<crate::player::Player>>,
    bodies: Query<(Entity, &exa_physics::body::Ragdoll, &exa_combat::Condition)>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut frames: Local<u32>,
    mut requests: MessageWriter<ScreenRequest>,
) {
    let Some(at) = debug.and_then(|debug| debug.0) else {
        return;
    };
    if loading.is_some() {
        return;
    }
    let Some(player) = players.iter().next() else {
        return;
    };
    *frames += 1;
    if *frames != at {
        return;
    }
    let feet = player.translation();
    let distance = |ragdoll: &exa_physics::body::Ragdoll| {
        ragdoll.parts.first().map_or(f32::MAX, |part| {
            Vec3::from(part.transform().translation).distance(feet)
        })
    };
    let nearest = bodies
        .iter()
        .filter(|(_, _, condition)| condition.is_dead())
        .min_by(|a, b| distance(a.1).total_cmp(&distance(b.1)));
    let Some((body, ragdoll, _)) = nearest else {
        info!("open-loot: no dead body in the level");
        return;
    };
    info!("open-loot: body {body:?} at {:.2} m", distance(ragdoll));
    requests.write(ScreenRequest {
        screen: GameScreen::Loot,
        player_ready: true,
        player_alive: true,
        target: Some(body.index().index() as i32),
        flags: 0,
        toggle: true,
    });
}

/// Hands the live database's tagged records to the item catalogue once it
/// is loaded, and again whenever the database changes: the local tier (the
/// loaded level's own records and the copies made in it) and the world
/// tier, each under its tagged ids, so their icons render, their bulk
/// counts and their cards read their own bytes. A record written in place
/// is read again alone (a template's, or its tier's).
pub(crate) fn register_level_items(
    live: Option<Res<exa_assets::live_objects::LiveObjects>>,
    catalogue: Option<ResMut<exa_items::catalogue::ItemCatalogue>>,
    mut seen: Local<Option<u64>>,
) {
    use exa_assets::live_objects::Changes;
    use exa_formats::objdb::{TAG_MASK, Tier};
    let (Some(live), Some(mut catalogue)) = (live, catalogue) else {
        return;
    };
    if catalogue.is_empty() {
        *seen = None;
        return;
    }
    let changes = seen.map_or(Changes::All, |seen| live.changes_since(seen));
    let tiers: Vec<Tier> = match changes {
        Changes::None => return,
        Changes::All => vec![Tier::Local, Tier::World],
        Changes::Records(ids) => {
            let mut tiers = Vec::new();
            for id in ids {
                if id & TAG_MASK == 0 {
                    if let Some(record) = live.db().base.find(id) {
                        let base = &live.db().base;
                        catalogue.refresh_record(id, record.payload(), |parent| {
                            base.find(parent).map(|r| r.payload())
                        });
                    }
                } else if !tiers.contains(&Tier::of(id)) {
                    tiers.push(Tier::of(id));
                }
            }
            tiers
        }
    };
    for tier in tiers {
        register_tier(&live, &mut catalogue, tier);
    }
    *seen = Some(live.revision());
}

/// Hands one tier of the live database to the item catalogue: its items'
/// models and bulk and its records' cards, under their tagged ids.
pub(crate) fn register_tier(
    live: &exa_assets::live_objects::LiveObjects,
    catalogue: &mut exa_items::catalogue::ItemCatalogue,
    tier: exa_formats::objdb::Tier,
) {
    let store = live.db().store(tier);
    let items: Vec<exa_formats::objdb::Item> =
        store.records().filter_map(|r| r.item().ok()).collect();
    let records: Vec<(u32, Vec<u8>)> = store
        .records()
        .map(|r| (r.id, r.payload().to_vec()))
        .collect();
    catalogue.set_tier_items(tier.tag(), &items);
    catalogue.set_tier_cards(tier.tag(), &records, Some(live.db().library(tier)));
    debug!(
        "item catalogue: {} {:?} records from the live database",
        records.len(),
        tier
    );
}

/// Writes the records the closed popups saved back into their objects'
/// container sections.
fn store_saved_containers(
    mut screens: ResMut<GameScreens>,
    mut objects: Query<&mut exa_world::ObjectSections>,
    mut items: ResMut<crate::doll_view::DollItems>,
    mut live: ResMut<exa_assets::live_objects::LiveObjects>,
) {
    store_containers(&mut screens, &mut objects, Some(&mut items), &mut live);
}

/// Writes the queued container records into their objects' sections (or,
/// for a bag inside an item, the item's record).
fn store_containers(
    screens: &mut GameScreens,
    objects: &mut Query<&mut exa_world::ObjectSections>,
    mut items: Option<&mut crate::doll_view::DollItems>,
    live: &mut exa_assets::live_objects::LiveObjects,
) {
    for (object, id, record) in screens.saved_containers.drain(..) {
        if object == Entity::PLACEHOLDER {
            if let Some(items) = items.as_deref_mut() {
                items.store_container(live, id, &record);
            }
            continue;
        }
        if let Ok(mut sections) = objects.get_mut(object)
            && let Some(section) = sections.container.as_mut()
        {
            store_record(section, &record);
        }
    }
}

/// A game save first saves the open screens ([`GameScreens::save_open_screens`]):
/// when a save to a new slot or a writing save step is asked for this
/// frame, the open popups' contents go into their records (and from there
/// into the level section the save writes) and the journal keeps its text.
#[allow(clippy::too_many_arguments)]
pub fn save_open_screens_for_saves<const AT_QUIT: bool>(
    mut new_slots: MessageReader<crate::save_live::SaveToNewSlot>,
    mut steps: MessageReader<crate::save_live::SaveStep>,
    screens: Option<ResMut<GameScreens>>,
    mut objects: Query<&mut exa_world::ObjectSections>,
    mut items: Option<ResMut<crate::doll_view::DollItems>>,
    (live, quit): (
        Option<ResMut<exa_assets::live_objects::LiveObjects>>,
        Option<Res<crate::save_live::QuitAutosave>>,
    ),
    mut bodies: Query<(Entity, &mut exa_items::possession::Possessions)>,
) {
    let new_slot = new_slots.read().count() > 0;
    let writes = steps.read().any(|step| {
        !matches!(
            step,
            crate::save_live::SaveStep::Drop | crate::save_live::SaveStep::NewSlot
        )
    });
    // The frame's and the quit's instances both read every frame; only
    // the one for the app's state saves ([`crate::save_live::quitting`]).
    if (!new_slot && !writes) || crate::save_live::quitting(quit) != AT_QUIT {
        return;
    }
    let (Some(mut screens), Some(mut live)) = (screens, live) else {
        return;
    };
    if screens.open.is_empty() {
        return;
    }
    let actions = screens.save_open_screens();
    info!("save: the open screens save first: {actions:?}");
    store_containers(&mut screens, &mut objects, items.as_deref_mut(), &mut live);
    // The inventory's and the loot windows' loaded packs save into their
    // characters' carried lists.
    use exa_ui::game_gui::SaveAction;
    if actions.contains(&SaveAction::Inventory)
        && let Some(body) = screens.inventory_body
        && let Ok((_, mut possessions)) = bodies.get_mut(body)
    {
        let written = body_inventory::store_pack(&mut possessions, &screens.pack);
        info!("save: the inventory's pack writes {written} entries into {body:?}'s list");
    }
    if actions.contains(&SaveAction::Loot) {
        for window in screens.loot.iter().filter(|window| window.pack.loaded) {
            let Some((_, mut possessions)) = bodies
                .iter_mut()
                .find(|(entity, _)| entity.index().index() as i32 == window.character)
            else {
                continue;
            };
            let written = body_inventory::store_pack(&mut possessions, &window.pack);
            info!(
                "save: the loot window's pack writes {written} entries into character {}'s list",
                window.character
            );
        }
    }
}

/// Gives the icon manager back the handles of the panels the screens
/// closed this frame.
pub(crate) fn release_closed_icons(
    mut screens: ResMut<GameScreens>,
    mut icons: ResMut<PanelIcons>,
) {
    for handle in screens.released_icons.drain(..) {
        icons.release(handle);
    }
}

/// Keeps the task's screen size on the window's physical size.
fn sync_screen_size(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
) {
    if let Ok(window) = windows.single() {
        let size = (
            window.physical_width() as i32,
            window.physical_height() as i32,
        );
        if size != screens.task.screen {
            screens.task.resize(size);
        }
    }
}

/// The GUI unit follows the window's height (the level switch); a change
/// runs the rescale walk over the open screens
/// ([`GameScreens::rescale_open`]).
pub(crate) fn rescale_open_screens(
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let unit = exa_ui::scale::gui_scale(window.physical_height(), 0) as i32;
    let stale = screens.open.iter().any(|screen| {
        screen.kind != GameScreen::TechniqueSelector
            && screens
                .task
                .form(screen.form)
                .and_then(|f| f.base())
                .is_some_and(|f| f.unit != unit.max(1))
    });
    if stale {
        let count = screens.rescale_open(unit);
        if count > 0 {
            info!("GUI: {count} open screens rescale to unit {unit}");
        }
    }
}

/// A script's `ShowInventory` opens the named character's inventory (the
/// actor index clamped to 0 or more), without the toggle: an inventory
/// that is already the active form is left. The player's own body opens
/// the player's inventory; another character's opens that character's,
/// under the same show rule (the player ready and not dead).
fn show_script_inventory(
    mut effects: MessageReader<exa_script::ScriptEffect>,
    players: Query<
        (&exa_ai::brain::CharacterId, Option<&exa_combat::Condition>),
        With<crate::player::PlayerBody>,
    >,
    characters: Query<(Entity, &exa_ai::brain::CharacterId), Without<crate::player::PlayerBody>>,
    mut screens: MessageWriter<ScreenRequest>,
) {
    let Some((id, condition)) = players.iter().next() else {
        return;
    };
    let word = condition.copied().unwrap_or_default();
    for effect in effects.read() {
        let exa_script::dialogue::Effect::ShowInventory(actor) = effect.0 else {
            continue;
        };
        let target = if actor == id.0 {
            None
        } else {
            match characters.iter().find(|(_, c)| c.0 == actor) {
                Some((body, _)) => Some(body.index().index() as i32),
                None => {
                    debug!("ShowInventory of {actor}: no such character");
                    continue;
                }
            }
        };
        info!("ShowInventory of {actor}: the inventory of {target:?} (None: the player's)");
        screens.write(ScreenRequest {
            screen: GameScreen::Inventory,
            player_ready: !word.dying(),
            player_alive: !word.is_dead(),
            target,
            flags: 0,
            toggle: false,
        });
    }
}

/// Opens the screens the hotkeys asked for; Escape closes the focused one.
/// Nothing happens while the main menu is open.
#[allow(clippy::too_many_arguments)]
pub(crate) fn open_requested_screens(
    keys: Res<ButtonInput<KeyCode>>,
    mut requests: MessageReader<ScreenRequest>,
    players: Query<Entity, With<crate::player::Player>>,
    player_bodies: Query<Entity, With<crate::player::PlayerBody>>,
    possessions: Query<(Entity, &exa_items::possession::Possessions)>,
    catalogue: Option<Res<exa_items::catalogue::ItemCatalogue>>,
    mut icons: Option<ResMut<PanelIcons>>,
    menu: Option<Res<MenuScreen>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    minds: Query<&crate::mind::casts::MindBody>,
    mut screens: ResMut<GameScreens>,
    (mut roll, mut rtl): (
        ResMut<crate::thaum::live::FrameEffectRoll>,
        ResMut<crate::thaum::live::FrameEffectDraws>,
    ),
) {
    if menu.is_some_and(|menu| menu.open) {
        requests.clear();
        return;
    }
    // The unit the forms are shown at, as the painter reads it.
    let unit = windows
        .single()
        .ok()
        .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
        .unwrap_or(4);
    if let Some(entity) = players.iter().next() {
        screens.player = entity.index().index() as i32;
    }
    for request in requests.read() {
        if !request.player_ready || !request.player_alive {
            continue;
        }
        let action = match request.screen {
            GameScreen::Inventory => {
                // The character the window binds: the player's own body, or
                // the body the request names.
                let body = match request.target {
                    None => player_bodies.iter().next(),
                    Some(target) => {
                        let body = possessions
                            .iter()
                            .find(|(entity, _)| entity.index().index() as i32 == target)
                            .map(|(entity, _)| entity);
                        if body.is_none() {
                            continue;
                        }
                        body
                    }
                };
                let owner = match request.target {
                    None => screens.player,
                    Some(target) => target,
                };
                let action = screens.show_inventory(
                    request.player_ready,
                    request.player_alive,
                    unit,
                    request.toggle,
                    owner,
                );
                if action == ScreenAction::Opened {
                    screens.inventory_body = body;
                    // The fill's mind window: a shown body bound by a
                    // Loyalty or possession marker.
                    screens.inventory_mind_window = body
                        .and_then(|body| minds.get(body).ok())
                        .is_some_and(crate::mind::casts::is_bound);
                    if screens.inventory_mind_window {
                        info!("GUI: the inventory shows a bound body: its mind window is set");
                    }
                }
                if owner != screens.player {
                    info!("GUI: the inventory of character {owner}: {action:?}");
                }
                // The pack is the character's own carried list, as the
                // original's bind loads it when the inventory opens, and
                // what the character wears counts against its capacity.
                if action == ScreenAction::Opened
                    && let Some(body) = body
                    && let Ok((_, possessions)) = possessions.get(body)
                    && let Some(icons) = icons.as_deref_mut()
                {
                    let loaded = body_inventory::bind_inventory_pack(
                        &mut screens.pack,
                        possessions,
                        icons,
                        catalogue.as_deref(),
                    );
                    info!(
                        "GUI: the inventory pack binds {loaded} entries, {} of its capacity used with {} worn items",
                        screens.pack.used,
                        possessions.worn.len()
                    );
                }
                action
            }
            GameScreen::Journal => {
                screens.open_journal_at(request.player_ready, request.player_alive, unit)
            }
            GameScreen::Loot => match request.target {
                Some(target) => {
                    let entity = possessions
                        .iter()
                        .find(|(entity, _)| entity.index().index() as i32 == target)
                        .map(|(entity, _)| entity);
                    // The window's show rule reads the body's record flag
                    // word: the profession word (byte 1 the race; race 3
                    // refuses), which the body's possessions keep.
                    let flags = entity
                        .and_then(|entity| possessions.get(entity).ok())
                        .map_or(request.flags, |(_, body)| body.flags);
                    let action = screens.open_loot(target, flags, unit);
                    if action == ScreenAction::Opened
                        && let Some(entity) = entity
                        && let Ok((_, body)) = possessions.get(entity)
                        && let Some(bounds) = screens
                            .loot
                            .iter()
                            .rev()
                            .find(|window| window.character == target)
                            .and_then(|window| screens.task.form(window.form))
                            .map(|form| form.bounds())
                        && let Some(icons) = icons.as_deref_mut()
                    {
                        let (pack_rect, equipment) = loot_panel_rects_at(bounds, unit);
                        let mut pack = Container::form(unit);
                        pack.owner = Some(target as u32);
                        pack.client = (pack_rect.width(), pack_rect.height());
                        body_inventory::bind_pack(&mut pack, body, icons, catalogue.as_deref());
                        let client = panel_client(equipment);
                        let mut worn = Container::form(unit);
                        worn.owner = Some(target as u32);
                        worn.client = (client.width(), client.height());
                        body_inventory::bind_equipment(
                            &mut worn,
                            body,
                            icons,
                            catalogue.as_deref(),
                            (client.width(), client.height()),
                            roll.word(),
                            Some(&mut rtl.0.0),
                        );
                        screens.bind_loot(target, pack, worn);
                    }
                    action
                }
                None => continue,
            },
            _ => continue,
        };
        debug!("GUI: {:?} {action:?}", request.screen);
    }
    if keys.just_pressed(KeyCode::Escape) && screens.close_focused() {
        debug!("GUI: the focused screen closes");
    }
}

/// `--debug-hint TEXT`: the inventory's Primary button carries this
/// tooltip text (the C string a button record keeps for its tooltip),
/// so the default tooltip can be checked by eye. No retail record of the
/// running game's screens is known to carry one.
#[derive(Resource, Debug, Clone, Default)]
pub struct DebugRecordHint(pub Option<String>);

/// The screens' window manager driven by the pointer: the hover, the
/// focus and the frame marks follow it, the left button presses and
/// releases through the forms' own rules (a title press drags the window,
/// a release over the close mark closes it, the scroll arrows repeat), the
/// wheel scrolls the focused form, and the task's tick shows the default
/// tooltip ([`crate::form_input`]). A moved player's inventory keeps its
/// place for the next open when it closes.
#[allow(clippy::too_many_arguments)]
fn screens_pointer(
    mouse: Option<Res<ButtonInput<MouseButton>>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut wheel: MessageReader<bevy::input::mouse::MouseWheel>,
    windows: Query<&Window, With<PrimaryWindow>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    time: Res<Time<Real>>,
    menu: Option<Res<MenuScreen>>,
    hint: Res<DebugRecordHint>,
    mut screens: ResMut<GameScreens>,
    mut timers: Option<ResMut<crate::frame_timers::FrameTimers>>,
    taken: Option<Res<crate::player::CaptureAnswer>>,
    controls: Option<Res<crate::player_controls::PlayerControl>>,
) {
    let delta = crate::form_input::wheel_delta(&mut wheel);
    if screens.open.is_empty() || menu.is_some_and(|menu| menu.open) {
        return;
    }
    let (Some(mouse), Ok(window)) = (mouse, windows.single()) else {
        return;
    };
    if let Some(text) = hint.0.as_deref()
        && let Some(screen) = screens.find(GameScreen::Inventory)
        && let Some(form) = screens
            .task
            .form_mut(screen.form)
            .and_then(|f| f.base_mut())
        && let Some(primary) = form.records.iter_mut().find(|r| r.class == 1)
        && primary.hint.is_empty()
    {
        primary.hint = text.as_bytes().to_vec();
    }
    let now = time.elapsed().as_millis() as u32;
    // The middle button reaches the GUI only while the controls are off.
    let gui_middle = controls.is_none_or(|c| c.off);
    let mut frame =
        crate::form_input::read_pointer(window, scripted.as_deref(), &mouse, delta, now).with_keys(
            &mouse,
            keys.as_deref(),
            gui_middle,
        );
    // The controls' mouse capture took the frame's mouse: the desk skips
    // its mouse-move processing (the hover and a drag's start).
    if taken.is_some_and(|t| t.0) {
        frame.at = None;
    }
    let screens = &mut *screens;
    screens.sync_journal_box();
    // The journal's open drop-down list is over the windows and takes the
    // pointer while it is up.
    if screens.journal_list_pointer(&frame) {
        // The press that opened the list held nothing: its release still
        // clears the window manager's buttons.
        if frame.released {
            screens.task.release(1);
        }
        return;
    }
    let tick = crate::form_input::drive(&mut screens.task, &frame, timers.as_deref_mut());
    // The journal list's press, drag and caret, before the records'
    // callbacks (New's focus moves to the list after its press).
    screens.journal_combo_pointer(&frame, mouse.pressed(MouseButton::Left));
    crate::form_input::ShownTooltip::follow(
        &mut screens.tooltip,
        &tick,
        frame.at,
        &mut screens.task,
        now,
    );
    let forms: Vec<(GameScreen, FormId)> = screens.open.iter().map(|s| (s.kind, s.form)).collect();
    for (kind, form) in forms {
        let events = screens
            .task
            .form_mut(form)
            .and_then(|f| f.base_mut())
            .map(|f| std::mem::take(&mut f.events))
            .unwrap_or_default();
        for event in events {
            match event {
                exa_ui::base_form::BaseEvent::Moved(x, y) => {
                    info!("gui: {kind:?} moved to {x}, {y}");
                }
                exa_ui::base_form::BaseEvent::Close => {
                    info!("gui: {kind:?} closed by its close mark");
                    if let Some(index) = screens.open.iter().position(|s| s.form == form) {
                        screens.close_at(index);
                    }
                }
                exa_ui::base_form::BaseEvent::Changed(record) if kind == GameScreen::Journal => {
                    screens.journal_record(record);
                }
                exa_ui::base_form::BaseEvent::Changed(record)
                    if kind == GameScreen::Inventory && PAGE_BUTTONS.contains(&record) =>
                {
                    // The page buttons' callback: the button that turned on
                    // asks for the swap; the one turned off does nothing.
                    let state = screens
                        .task
                        .form(form)
                        .and_then(|f| f.base())
                        .and_then(|f| f.records.get(record))
                        .map_or(0, |r| r.state);
                    if exa_ui::game_gui::page_button_swaps(state) {
                        info!("gui: inventory page button {record} asks for the set swap");
                        screens.page_swap = true;
                    }
                }
                _ => {}
            }
        }
    }
    let shift = keys
        .as_deref()
        .is_some_and(|k| k.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]));
    screens.journal_pointer(&frame, mouse.pressed(MouseButton::Left), shift);
}

/// The frame's keys for the focused game screen ([`GameScreens::keys_to_window`]):
/// each key-down (with the window procedure's mask word) and typed
/// character goes to the focused form's key slot
/// ([`exa_ui::gui_core::GuiTask::key`]): a Tab moves a window's focus among
/// its tab stops, a record's key message (0xa0) answers 0 for every record
/// class the game screens hold, and the key climbs to the task: the root
/// task's shortcuts ([`crate::gui_keys::TaskKeys`]). Escape is Exarust's
/// close of the focused screen ([`GameScreens::close_focused`]), which
/// takes it. The journal's text box has its own key slot
/// ([`journal_typing`]).
pub fn screens_keys(
    frame: Res<crate::gui_keys::GuiKeyFrame>,
    mut screens: ResMut<GameScreens>,
    mut climbed: Option<ResMut<crate::gui_keys::TaskKeys>>,
    mut clipboard: Option<ResMut<crate::os_clipboard::SystemClipboard>>,
) {
    if frame.keys.is_empty() || !screens.keys_to_window() || screens.journal_box_focused() {
        return;
    }
    for key in &frame.keys {
        // The journal list with the key focus takes every key but Tab
        // (the form's own) as its key message, and answers 3.
        if screens.journal_combo_focused()
            && (key.flags & exa_ui::gui_keys::TYPED != 0 || key.key != 9)
        {
            if let Some(press) = crate::option_forms::field_key_of(*key) {
                let pasted =
                    (press.ctrl && !press.typed && press.byte == exa_ui::text_edit::key::PASTE)
                        .then(|| clipboard.as_deref_mut().and_then(|c| c.read()))
                        .flatten();
                let action = screens.journal_combo_key(press, pasted.as_deref());
                if let Some(
                    exa_ui::text_edit::EditAction::Copy(bytes)
                    | exa_ui::text_edit::EditAction::Cut(bytes),
                ) = action
                    && let Some(c) = clipboard.as_deref_mut()
                {
                    c.put(&bytes);
                }
            }
            continue;
        }
        let route = screens.task.key(key.key, key.flags);
        debug!(
            "gui: key {:#x} (word {:#x}) on the game screens: {route:?}",
            key.key, key.flags
        );
        if route == exa_ui::gui_core::KeyRoute::Task
            && key.key != u32::from(crate::commands::MENU_TOGGLE_KEY)
            && let Some(climbed) = climbed.as_deref_mut()
        {
            climbed.forward(*key);
        }
    }
}

/// The text box's key code for a key-down (the virtual-key codes its key
/// slot switches on).
fn text_box_key(code: KeyCode) -> Option<u32> {
    use exa_ui::text_box as tb;
    Some(match code {
        KeyCode::Backspace => tb::KEY_BACK,
        KeyCode::Tab => tb::KEY_TAB,
        KeyCode::Enter | KeyCode::NumpadEnter => tb::KEY_RETURN,
        KeyCode::PageUp => tb::KEY_PRIOR,
        KeyCode::PageDown => tb::KEY_NEXT,
        KeyCode::End => tb::KEY_END,
        KeyCode::Home => tb::KEY_HOME,
        KeyCode::ArrowLeft => tb::KEY_LEFT,
        KeyCode::ArrowUp => tb::KEY_UP,
        KeyCode::ArrowRight => tb::KEY_RIGHT,
        KeyCode::ArrowDown => tb::KEY_DOWN,
        KeyCode::Delete => tb::KEY_DELETE,
        KeyCode::KeyC => tb::KEY_C,
        KeyCode::KeyV => tb::KEY_V,
        KeyCode::KeyX => tb::KEY_X,
        _ => return None,
    })
}

/// The keys of the frame while the window manager's focus is on the
/// journal's text box (the pointer over it: the focus follows the
/// pointer): each key-down goes to the box's key slot, then the character
/// it typed as a typed byte (not with Ctrl; through the ANSI code page, a
/// character the page lacks dropped). Tab is the box's callback: combat
/// mode on, whose entry closes the screens. Runs before the player's
/// hotkeys, which the box's focus keeps from the keys
/// ([`GameScreens::journal_box_focused`]).
pub fn journal_typing(
    mut typed: MessageReader<bevy::input::keyboard::KeyboardInput>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    mut screens: ResMut<GameScreens>,
    mut clipboard: Option<ResMut<crate::os_clipboard::SystemClipboard>>,
    mut steps: Option<ResMut<crate::flow::FlowSteps>>,
) {
    use bevy::input::keyboard::Key;
    let events: Vec<_> = typed.read().cloned().collect();
    if !screens.journal_box_focused() {
        return;
    }
    let held = |codes: [KeyCode; 2]| keys.as_deref().is_some_and(|k| k.any_pressed(codes));
    let mods = exa_ui::text_box::Modifiers {
        shift: held([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        ctrl: held([KeyCode::ControlLeft, KeyCode::ControlRight]),
    };
    let mut presses: Vec<(u32, bool)> = Vec::new();
    let mut sent = Vec::new();
    for event in events.iter().filter(|e| e.state.is_pressed()) {
        if let Some(code) =
            text_box_key(event.key_code).filter(|&k| mods.ctrl || !matches!(k, 0x43 | 0x56 | 0x58))
        {
            sent.push(event.key_code);
            presses.push((code, false));
        }
        if mods.ctrl {
            continue;
        }
        let text = match &event.logical_key {
            Key::Character(text) => text.to_string(),
            Key::Space => " ".to_string(),
            _ => continue,
        };
        for c in text.chars() {
            if let Some(byte) = crate::os_clipboard::char_ansi(c).filter(|&b| b >= 0x20) {
                presses.push((u32::from(byte), true));
            }
        }
    }
    // Scripted key-downs (`--press`) come without a keyboard message: a
    // letter, digit or space also types its character (capital with
    // Shift), as the system's character message would.
    for code in keys.iter().flat_map(|k| k.get_just_pressed()) {
        if sent.contains(code) {
            continue;
        }
        if let Some(key) =
            text_box_key(*code).filter(|&k| mods.ctrl || !matches!(k, 0x43 | 0x56 | 0x58))
        {
            presses.push((key, false));
        } else if events.is_empty()
            && !mods.ctrl
            && let Some(byte) = scripted_char(*code, mods.shift)
        {
            presses.push((u32::from(byte), true));
        }
    }
    let mut none = NoClipboard;
    for (key, is_typed) in presses {
        let board: &mut dyn exa_ui::text_box::Clipboard = match clipboard.as_deref_mut() {
            Some(board) => board,
            None => &mut none,
        };
        if screens.journal_key(key, is_typed, mods, board) {
            info!("gui: Tab in the journal's text box turns combat mode on");
            if let Some(steps) = steps.as_mut() {
                steps.steps.push(crate::flow::Step::CombatMode(true));
            }
        }
    }
}

/// The character a scripted key-down types: a letter (capital with
/// Shift), a digit or a space.
fn scripted_char(code: KeyCode, shift: bool) -> Option<u8> {
    let letters = [
        KeyCode::KeyA,
        KeyCode::KeyB,
        KeyCode::KeyC,
        KeyCode::KeyD,
        KeyCode::KeyE,
        KeyCode::KeyF,
        KeyCode::KeyG,
        KeyCode::KeyH,
        KeyCode::KeyI,
        KeyCode::KeyJ,
        KeyCode::KeyK,
        KeyCode::KeyL,
        KeyCode::KeyM,
        KeyCode::KeyN,
        KeyCode::KeyO,
        KeyCode::KeyP,
        KeyCode::KeyQ,
        KeyCode::KeyR,
        KeyCode::KeyS,
        KeyCode::KeyT,
        KeyCode::KeyU,
        KeyCode::KeyV,
        KeyCode::KeyW,
        KeyCode::KeyX,
        KeyCode::KeyY,
        KeyCode::KeyZ,
    ];
    let digits = [
        KeyCode::Digit0,
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ];
    if let Some(i) = letters.iter().position(|&k| k == code) {
        return Some(if shift { b'A' } else { b'a' } + i as u8);
    }
    if let Some(i) = digits.iter().position(|&k| k == code) {
        return Some(b'0' + i as u8);
    }
    (code == KeyCode::Space).then_some(b' ')
}

/// No system clipboard: copies are dropped and pastes find nothing.
struct NoClipboard;

impl exa_ui::text_box::Clipboard for NoClipboard {
    fn set(&mut self, _text: &[u8]) {}

    fn get(&mut self) -> Option<Vec<u8>> {
        None
    }
}

/// A right-button release over a screen that has the handler closes it
/// (the inventory's and the loot window's mouse routines); a release over
/// another form or outside leaves the screens alone.
pub(crate) fn close_on_right_release(
    buttons: Option<Res<ButtonInput<MouseButton>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut screens: ResMut<GameScreens>,
    card: Option<Res<crate::item_card::CardView>>,
    scripted: Option<Res<exa_ui::ScriptedPointer>>,
    icons: Option<Res<PanelIcons>>,
) {
    let Some(buttons) = buttons else {
        return;
    };
    if !buttons.just_released(MouseButton::Right) {
        return;
    }
    let Ok(window) = windows.single() else {
        return;
    };
    // The scripted pointer when one is set (as the forms read it).
    let pointer = match scripted.as_deref() {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    };
    let Some(pointer) = pointer else {
        return;
    };
    let pointer = pointer * window.scale_factor();
    // A right press over an entry is the panel's card request; it must not
    // also close the screen. The next right release outside the card is
    // the card's own close.
    if card.as_deref().is_some_and(|card| card.active(&screens))
        || crate::item_card::entry_under(&screens, icons.as_deref(), pointer.as_ivec2())
    {
        return;
    }
    // A child form (the journal's text box) hands the right-button word
    // to its parent: the screen holding the focus closes.
    let active = screens.task.active.map(|form| screens.task.root_of(form));
    let Some(screen) = screens
        .open
        .iter()
        .find(|screen| Some(screen.form) == active)
        .copied()
    else {
        return;
    };
    if !exa_ui::game_gui::right_release_closes(screen.kind) {
        return;
    }
    let Some(form) = screens.task.form(screen.form) else {
        return;
    };
    if form.bounds().contains(pointer.x as i32, pointer.y as i32) {
        screens.close(screen.kind);
    }
}

/// The dead powers' loot close: Raise Dead and Command Dead close the first
/// loot window open on the body they raise, as the original's routine walks
/// the GUI task's forms for the loot tag and the body.
pub fn close_loot_on_body(world: &mut World, body: Entity) {
    let Some(mut screens) = world.get_resource_mut::<GameScreens>() else {
        return;
    };
    let character = body.index().index() as i32;
    if screens.close_loot(character) {
        info!("raise dead: the loot window on {body:?} closes");
    }
}

/// The GUI callback a Loyalty or Command Dead mark posts as it ends on its
/// body: the task's root takes the focus, and an inventory window open on
/// the body (the original looks it up by its owner and its visible byte)
/// closes.
pub fn release_marked_body(world: &mut World, body: Entity) {
    let Some(mut screens) = world.get_resource_mut::<GameScreens>() else {
        return;
    };
    if let Some(root) = screens.task.root {
        screens.task.activate_with(root, NO_CONTROL);
    }
    if screens.inventory_body == Some(body)
        && screens.is_open(GameScreen::Inventory)
        && screens.close(GameScreen::Inventory)
    {
        info!("mind: the mark on {body:?} ends: its inventory closes");
    }
}

/// Closes the loot windows the original's deferred close takes: a window
/// whose body's state word has `0x80000`, or whose condition is no longer
/// the dead mode `0xa00`. Exarust keeps no copy of the body's state word, so
/// the mark half reads 0 [not modelled]; a window whose body is no longer in
/// the world is left alone.
fn close_changed_loot_windows(
    bodies: Query<(Entity, &exa_combat::Condition)>,
    mut screens: ResMut<GameScreens>,
) {
    if screens.loot.is_empty() {
        return;
    }
    let live: Vec<(i32, u32, u32)> = screens
        .loot
        .iter()
        .filter_map(|window| {
            bodies
                .iter()
                .find(|(entity, _)| entity.index().index() as i32 == window.character)
                .map(|(_, condition)| (window.character, 0, condition.0))
        })
        .collect();
    for character in screens.loot_windows_to_close(&live) {
        screens.close_loot(character);
    }
}

/// The font texture's size, for the glyph quads' sample step.
fn font_texture(textures: &GuiTextures, font: &exa_ui::gui_font::GuiFont) -> Option<(u16, u16)> {
    textures
        .get(font.texture)
        .map(|texture| (texture.width as u16, texture.height as u16))
}

/// The font a screen uses: the level's main font when its texture is
/// installed. Else the level's metrics (glyph quad, advances, line height,
/// which the layout and the controls' sizes are built from) are kept and the
/// glyph cells are sampled from the base font's atlas, the only GUI font the
/// packs ship. The size returned then is the atlas size the level's glyph
/// quad would span one base glyph cell of, so [`exa_ui::gui_text::font_uv`]
/// gives the base cell's sample step while the quad keeps the level's size.
pub(crate) fn screen_font(
    level_font: &exa_ui::gui_font::GuiFont,
    textures: &GuiTextures,
) -> (exa_ui::gui_font::GuiFont, Option<(u16, u16)>) {
    exa_ui::gui_font::font_with_fallback(
        level_font,
        font_texture(textures, level_font),
        font_texture(textures, &exa_ui::gui_font::FONTBASE_24),
    )
}

/// Paints the control records a screen's form carries: buttons with their
/// caption and labels as text. The unlocated control is not drawn.
///
/// `combo` is the combo box's edit bytes and its form's caret bit, when
/// it keeps them (the journal list).
#[allow(clippy::too_many_arguments)]
fn paint_controls(
    controls: &[Control],
    unit: i32,
    palette: &exa_ui::skin::SkinPalette,
    font: &exa_ui::gui_font::GuiFont,
    size: Option<(u16, u16)>,
    glyphs: Option<&exa_ui::GuiLevelState>,
    combo: Option<(Option<exa_ui::text_edit::EditView>, bool)>,
    painted: &mut gui_paint::Painted,
) {
    for control in controls {
        let rect = GuiRect::new(control.x, control.y, control.w, control.h);
        let text = String::from_utf8_lossy(&control.text);
        let layers = match control.class {
            // The combo box: the skin's panel behind it, then its own front
            // (the arrow and the caption).
            exa_ui::controls::COMBO_BOX => {
                let mut layers =
                    gui_paint::paint_panel(rect, unit, palette, gui_paint::FORM_ATTRIBUTES, 0);
                layers.layers.extend(
                    gui_paint::paint_combo_front(
                        rect,
                        gui_paint::ComboWords {
                            style: control.style,
                            state: control.state,
                            pad: control.margin,
                            text: &control.text,
                            view: combo.and_then(|(view, _)| view),
                            caret_lit: combo.is_some_and(|(_, lit)| lit),
                        },
                        unit,
                        palette,
                        font,
                        size,
                        glyphs,
                    )
                    .layers,
                );
                layers
            }
            1 => gui_paint::paint_line_button(
                rect,
                gui_paint::ButtonWords {
                    style: control.style,
                    state: control.state,
                    group: control.group,
                },
                unit,
                palette,
                1.0,
                font,
                size,
                &text,
            ),
            5 => gui_paint::paint_label(rect, control.style, unit, palette, font, size, &text),
            _ => continue,
        };
        for layer in layers.layers {
            painted.push(layer);
        }
    }
}

/// The inventory's pack panel at the design unit 4, as its painter places
/// it: the client's bottom, 228 × 180 ([`InventoryLayout::pack_at`] for
/// another unit).
pub fn inventory_pack_panel(bounds: GuiRect) -> GuiRect {
    InventoryLayout::at(4).pack_at(bounds)
}

/// A loot window's pack and equipment panels at the design unit 4, as its
/// painter places them: the pack at the client's bottom (228 × 180) and the
/// equipment panel above it, ending four units before the pack.
pub fn loot_panel_rects(bounds: GuiRect) -> (GuiRect, GuiRect) {
    loot_panel_rects_at(bounds, 4)
}

/// A loot window's pack and equipment panels at the unit the window was
/// opened at ([`ScreenLayout::loot`]).
pub fn loot_panel_rects_at(bounds: GuiRect, unit: i32) -> (GuiRect, GuiRect) {
    let layout = ScreenLayout::loot(unit);
    (layout.child_at(0, bounds), layout.child_at(1, bounds))
}

/// A panel's inner client, the four-unit inset its painter uses for the
/// entries' cells.
pub fn panel_client(rect: GuiRect) -> GuiRect {
    GuiRect::new(
        rect.left + 4,
        rect.top + 4,
        rect.width() - 8,
        rect.height() - 8,
    )
}

/// The inventory's frame, its character view's panel, the item pack's panel
/// with its entries' icons (when the panel's icon atlas holds them), the
/// Encumbrance label, its rating dots and the Primary button record, laid
/// out at the unit the window was opened at ([`InventoryLayout`]). When the
/// view draws its figure, the canvas is cleared over the view so the
/// figure's node shows through, and the colour the canvas had at the
/// view's centre comes back as the backdrop the view's camera clears to.
#[allow(clippy::too_many_arguments)]
fn paint_inventory(
    bounds: GuiRect,
    layout: &InventoryLayout,
    unit: i32,
    textures: &GuiTextures,
    items: &Container,
    icons: Option<&mut PanelIcons>,
    doll: Option<&crate::doll_view::DollPaint>,
    hover_mark: i32,
    records: Option<&[Control]>,
) -> (gui_paint::Painted, Option<[u8; 4]>) {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) = screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    // The game's skin draws a form through the line skin's form draw, with
    // the words the show commits: the inventory is a top-level form (its
    // shadow, backdrop, title and corner frame), the character view and
    // the pack are held children (no backdrop; the panel frame on their
    // edges, curled where they leave the client's edges).
    let client = GuiRect::new(
        bounds.left + layout.border,
        bounds.top + layout.border,
        bounds.width() - layout.border * 2,
        bounds.height() - layout.border * 2,
    );
    let mut painted = gui_paint::Painted {
        client,
        ..Default::default()
    };
    let outer_words =
        gui_paint::commit_words(gui_paint::SCREEN_STYLE, gui_paint::FORM_ATTRIBUTES, true);
    let mut outer = gui_paint::paint_line_form(
        LineFormAt {
            hover_mark,
            ..LineFormAt::top(bounds)
        },
        unit,
        &palette,
        1.0,
        outer_words,
        Some((INVENTORY_TITLE, &font, size)),
    );
    painted.layers.extend(outer.under());
    let child = |rect: GuiRect, attributes: u32| {
        gui_paint::paint_line_form(
            LineFormAt::child(rect, client, outer_words.attributes),
            unit,
            &palette,
            1.0,
            gui_paint::commit_words(gui_paint::PACK_STYLE, attributes, false),
            None,
        )
    };
    let doll_rect = layout.doll_at(bounds);
    let mut doll_form = child(doll_rect, gui_paint::DOLL_ATTRIBUTES);
    painted.layers.extend(doll_form.under());
    let view = crate::doll_view::view_rect(doll_rect, DOLL_BORDER);
    let backdrop = doll.filter(|doll| doll.punch).map(|_| {
        let centre = ((view.left + view.right) / 2, (view.top + view.bottom) / 2);
        let backdrop = canvas_colour_at(&painted.layers, textures, centre);
        painted.push(clear_layer(view));
        backdrop
    });
    // The view's draw callback for its shown item: the icon at the view's
    // lower-left corner, clipped to the view.
    let mut icons = icons;
    if let (Some(doll), Some(icons)) = (doll, icons.as_deref_mut())
        && doll.shown_icon != 0
        && let Some((x, y)) = crate::doll_view::shown_icon_origin(
            icons,
            doll.shown_icon,
            unit,
            view.height(),
            items.icon_size,
        )
    {
        let mut own = Container::form(unit);
        own.icon_size = items.icon_size;
        own.entries.push(crate::item_container::Entry {
            cell: crate::item_container::Cell::new(x, y),
            id: 0,
            icon: doll.shown_icon,
            flags: 0,
            drag: 0,
        });
        if let Some(mut layer) = icons.layer(&own) {
            gui_paint::offset_batch(&mut layer.batch, view.left, view.top);
            layer.clip = Some([view.left, view.top, view.width(), view.height()]);
            painted.push(layer);
        }
    }
    painted.layers.extend(doll_form.over());
    let pack = layout.pack_at(bounds);
    let mut pack_form = child(pack, gui_paint::PACK_ATTRIBUTES);
    painted.layers.extend(pack_form.under());
    let panel_client = panel_client(pack);
    // The entries' cells are the panel's own coordinates: the icons are
    // moved to its client area and clipped to it.
    if let Some(icons) = icons
        && let Some(mut layer) = icons.layer(items)
    {
        gui_paint::offset_batch(&mut layer.batch, panel_client.left, panel_client.top);
        layer.clip = Some([
            panel_client.left,
            panel_client.top,
            panel_client.width(),
            panel_client.height(),
        ]);
        painted.push(layer);
    }
    painted.layers.extend(pack_form.over());
    painted.layers.extend(outer.over());
    // The form's own records, with the hover bits the pointer gave them.
    let controls = records.map_or_else(|| layout.controls_at(bounds), <[Control]>::to_vec);
    paint_controls(
        &controls,
        unit,
        &palette,
        &font,
        size,
        None,
        None,
        &mut painted,
    );
    // The rating dots draw in the label group's seventh ink.
    if let Some(doll) = doll
        && let (Some(value), Some(sheet)) = (doll.dots, doll.sheet)
        && let Some(record) = controls
            .iter()
            .find(|control| control.class == RATING_DOTS_CLASS)
        && let Some(layer) = crate::doll_view::rating_dots_layer(
            sheet,
            value,
            (record.x, record.y, record.h),
            unit,
            palette.text[6],
        )
    {
        painted.push(layer);
    }
    (painted, backdrop)
}

/// The colour the layers leave at one window pixel, straight alpha: the
/// layers rasterised into a one-pixel canvas moved onto that pixel.
fn canvas_colour_at(
    layers: &[exa_ui::gui_draw::GuiLayer],
    textures: &GuiTextures,
    at: (i32, i32),
) -> [u8; 4] {
    let mut frame = exa_ui::gui_draw::GuiFrame::new();
    for layer in layers {
        let mut layer = layer.clone();
        gui_paint::offset_batch(&mut layer.batch, -at.0, -at.1);
        if let Some(clip) = &mut layer.clip {
            clip[0] -= at.0;
            clip[1] -= at.1;
        }
        frame.push(layer);
    }
    let pixel = exa_ui::gui_draw::rasterise(&[frame], textures, 1, 1);
    [pixel[0], pixel[1], pixel[2], pixel[3]]
}

/// A layer that clears the canvas over `rect` (blending off, a transparent
/// ink), so what is under the canvas shows there.
pub(crate) fn clear_layer(rect: GuiRect) -> exa_ui::gui_draw::GuiLayer {
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    batch.blend = exa_ui::gui_draw::BLEND_REPLACE;
    batch.set_ink([0, 0, 0, 0]);
    batch.begin();
    batch.vertex(rect.left as f32, rect.top as f32);
    batch.vertex(rect.left as f32, rect.bottom as f32);
    batch.vertex(rect.right as f32, rect.bottom as f32);
    batch.vertex(rect.right as f32, rect.top as f32);
    batch.end_quads();
    exa_ui::gui_draw::GuiLayer::new(batch, exa_ui::gui_draw::LayerTexture::None)
}

/// What the journal paints inside its frame: its records as its form
/// holds them (else as laid out), its text box (else the selected entry's
/// text in a box built for the paint), the box form's hover word and the
/// GUI level's glyph atlas (the list's arrow).
pub(crate) struct JournalPaint<'a> {
    pub journal: &'a Journal,
    pub layout: &'a ScreenLayout,
    pub records: Option<&'a [Control]>,
    pub text_box: Option<&'a JournalBox>,
    pub box_hover: i32,
    pub glyphs: Option<&'a exa_ui::GuiLevelState>,
    /// The list's edit bytes and the form's caret bit.
    pub combo: Option<(Option<exa_ui::text_edit::EditView>, bool)>,
}

/// The journal's frame, its records (New, Erase and the entry list), and
/// its text box: the held form's panel frame and scroll bar, then the
/// text through the box's own draw (selection, glyphs in the small font,
/// moved up by the drawn scroll and kept inside the client) and the caret
/// while it is lit.
fn paint_journal(
    bounds: GuiRect,
    paint: JournalPaint<'_>,
    unit: i32,
    textures: &GuiTextures,
    hover_mark: i32,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let fonts = exa_ui::gui_font::gui_level(level);
    let (font, size) = screen_font(fonts.main, textures);
    let layout = paint.layout;
    // The game skin's form draw with the words the show commits: a
    // top-level form, so titled, shadowed and corner-framed.
    let mut painted = gui_paint::Painted {
        client: layout.client(bounds),
        ..Default::default()
    };
    let (mut form, committed) = gui_paint::paint_top_form_marked(
        bounds,
        unit,
        &palette,
        1.0,
        gui_paint::JOURNAL_WORDS,
        Some((JOURNAL_TITLE, &font, size)),
        hover_mark,
    );
    painted.layers.extend(form.under());
    let controls = match paint.records {
        Some(records) => records.to_vec(),
        None => layout.controls_at(bounds),
    };
    paint_controls(
        &controls,
        unit,
        &palette,
        &font,
        size,
        paint.glyphs,
        paint.combo,
        &mut painted,
    );
    let rect = layout.child_at(0, bounds);
    let fallback;
    let (text_box, fraction) = match paint.text_box {
        Some(journal_box) => (&journal_box.text, journal_box.text.scroll_target),
        None => {
            let client =
                gui_paint::scrolling_client(rect, 0, gui_paint::JOURNAL_BOX_WORDS.attributes);
            let mut text_box =
                exa_ui::game_gui::journal_text_box(layout.unit, fonts.main, fonts.small, client);
            let text = paint
                .journal
                .entries
                .get(paint.journal.current)
                .map(|entry| crate::os_clipboard::text_to_ansi(&entry.text))
                .unwrap_or_default();
            text_box.set_text(&text, None);
            fallback = text_box;
            (&fallback, 0.0)
        }
    };
    let mut frame = gui_paint::paint_held_scrolling_in(
        rect,
        0,
        layout.client(bounds),
        committed.attributes,
        unit,
        &palette,
        1.0,
        gui_paint::JOURNAL_BOX_WORDS,
        fraction,
        paint.box_hover,
    );
    painted.layers.extend(frame.under());
    let client = text_box.client;
    let origin = (rect.left + client.left, rect.top + client.top);
    let (text_font, text_size) = screen_font(text_box.font, textures);
    let inks = exa_ui::text_layout::TextInks {
        text: palette.text[6],
        tag_alpha: 255,
        selection: palette.highlight[2],
        slant: font.slant,
    };
    // The drawn scroll's offset, as the form's content point takes it.
    let shift = exa_ui::form::FormFrame {
        x: rect.left,
        y: rect.top,
        client,
        content: (client.width(), text_box.content_height),
        scroll: (0.0, text_box.scroll),
        attributes: gui_paint::JOURNAL_BOX_WORDS.attributes,
    }
    .scroll_offset()
    .1;
    let (top, bottom) = (origin.1, origin.1 + client.height());
    let draw = text_box.draw(&inks, &|_| None);
    if !draw.selections.is_empty() {
        let mut batch = SkinBatch::new();
        batch.select(1, 0, false);
        for (r, ink) in &draw.selections {
            let y0 = (origin.1 + r[1] - shift).max(top);
            let y1 = (origin.1 + r[1] + r[3] - shift).min(bottom);
            if y1 <= y0 {
                continue;
            }
            let (x0, x1) = ((origin.0 + r[0]) as f32, (origin.0 + r[0] + r[2]) as f32);
            batch.set_ink(*ink);
            batch.begin();
            batch.vertex(x0, y0 as f32);
            batch.vertex(x0, y1 as f32);
            batch.vertex(x1, y1 as f32);
            batch.vertex(x1, y0 as f32);
            batch.end_quads();
        }
        painted.push(exa_ui::gui_draw::GuiLayer::new(
            batch,
            exa_ui::gui_draw::LayerTexture::None,
        ));
    }
    if let Some(texture) = text_size {
        let inside = |glyph: &exa_ui::text_layout::GlyphQuad| {
            let mut glyph = *glyph;
            glyph.x += origin.0;
            glyph.y += origin.1 - shift;
            (glyph.y >= top && glyph.y + text_font.line_height <= bottom).then_some(glyph)
        };
        let mut glyphs: Vec<_> = draw.glyphs.iter().filter_map(inside).collect();
        if let Some(caret) = text_box.caret_draw(&inks, &|_| None) {
            let quad = exa_ui::text_layout::GlyphQuad {
                byte: exa_ui::text_layout::CARET_GLYPH,
                x: caret.x,
                y: caret.y,
                slant: caret.slant,
                ink: inks.text,
            };
            glyphs.extend(inside(&quad));
        }
        if !glyphs.is_empty() {
            let batch = exa_ui::gui_draw::glyph_batch(&text_font, &glyphs, texture, 1.0);
            painted.push(exa_ui::gui_draw::GuiLayer::new(
                batch,
                exa_ui::gui_draw::LayerTexture::Named(text_font.texture),
            ));
        }
    }
    painted.layers.extend(frame.over());
    painted.layers.extend(form.over());
    painted
}

/// The loot window's frame and its two panels: the pack at the client's
/// bottom (228 × 180) and the equipment panel above it, ending four units
/// before the pack. Each panel draws its entries' icons when the icon cache
/// holds them, clipped to the panel's client.
fn paint_loot(
    bounds: GuiRect,
    unit: i32,
    textures: &GuiTextures,
    window: &LootWindow,
    mut icons: Option<&mut PanelIcons>,
    hover_mark: i32,
) -> gui_paint::Painted {
    let palette = gui_paint::palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, size) = screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let layout = ScreenLayout::loot(window.pack.unit.max(1));
    // The game skin's form draw: the window is top-level (titled,
    // shadowed, corner-framed), its two item panels are held children
    // framed on their edges.
    let client = layout.client(bounds);
    let mut painted = gui_paint::Painted {
        client,
        ..Default::default()
    };
    let (mut outer, outer_words) = gui_paint::paint_top_form_marked(
        bounds,
        unit,
        &palette,
        1.0,
        gui_paint::LOOT_WORDS,
        Some((LOOT_TITLE, &font, size)),
        hover_mark,
    );
    painted.layers.extend(outer.under());
    let (pack, equipment) = (layout.child_at(0, bounds), layout.child_at(1, bounds));
    let mut paint_panel_with =
        |rect: GuiRect, container: &Container, icons: &mut Option<&mut PanelIcons>| {
            let mut panel = gui_paint::paint_held_form(
                rect,
                client,
                outer_words.attributes,
                unit,
                &palette,
                1.0,
                gui_paint::ITEM_PANEL_WORDS,
            );
            painted.layers.extend(panel.under());
            let panel_client = panel_client(rect);
            if let Some(icons) = icons.as_deref_mut()
                && let Some(mut layer) = icons.layer(container)
            {
                gui_paint::offset_batch(&mut layer.batch, panel_client.left, panel_client.top);
                layer.clip = Some([
                    panel_client.left,
                    panel_client.top,
                    panel_client.width(),
                    panel_client.height(),
                ]);
                painted.push(layer);
            }
            painted.layers.extend(panel.over());
        };
    paint_panel_with(pack, &window.pack, &mut icons);
    paint_panel_with(equipment, &window.worn, &mut icons);
    painted.layers.extend(outer.over());
    painted
}

/// A container popup: the card's frame (no title, the popup's eight-pixel
/// border), the container's icon over the icon area and the item panel
/// with its entries' icons, clipped to the panel's client.
fn paint_container_popup(
    bounds: GuiRect,
    unit: i32,
    popup: &ContainerPopup,
    mut icons: Option<&mut PanelIcons>,
    hover_mark: i32,
) -> gui_paint::Painted {
    // A shown form's draw hands its fade (1) to the skin, which raises the
    // palette's alpha ramps (form group 170, the others 125).
    let palette = exa_ui::line_skin::shown_palette(1.0);
    // The game skin's form draw with the words the show commits: the popup
    // is top-level (shadowed, corner-framed; titled but with no title
    // string, and its 0x800000 bit keeps the title strip off), the item
    // panel a held child framed on its edges.
    let client = ContainerPopup::client(bounds);
    let mut painted = gui_paint::Painted {
        client,
        ..Default::default()
    };
    let (mut form, form_words) = gui_paint::paint_top_form_marked(
        bounds,
        unit,
        &palette,
        1.0,
        gui_paint::CONTAINER_WORDS,
        None,
        hover_mark,
    );
    painted.layers.extend(form.under());
    if let Some(icons) = icons.as_deref_mut() {
        let (x, y) = popup_icon_place(
            bounds.width(),
            popup.icon_height,
            unit,
            popup.panel.icon_size,
        );
        let mut own = Container::form(unit);
        own.entries.push(crate::item_container::Entry {
            cell: crate::item_container::Cell::new(x, y),
            id: popup.id,
            icon: popup.icon,
            flags: 0,
            drag: 0,
        });
        if let Some(mut layer) = icons.layer(&own) {
            gui_paint::offset_batch(&mut layer.batch, client.left, client.top);
            painted.push(layer);
        }
    }
    let panel_rect = popup.panel_rect(bounds);
    let mut panel = gui_paint::paint_held_form(
        panel_rect,
        client,
        form_words.attributes,
        unit,
        &palette,
        1.0,
        gui_paint::ITEM_PANEL_WORDS,
    );
    painted.layers.extend(panel.under());
    let panel_client = panel_client(panel_rect);
    if let Some(icons) = icons
        && let Some(mut layer) = icons.layer(&popup.panel)
    {
        gui_paint::offset_batch(&mut layer.batch, panel_client.left, panel_client.top);
        layer.clip = Some([
            panel_client.left,
            panel_client.top,
            panel_client.width(),
            panel_client.height(),
        ]);
        painted.push(layer);
    }
    painted.layers.extend(panel.over());
    painted.layers.extend(form.over());
    painted
}

/// Draws the open screens and the debug form into the GUI's queue.
#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_screens(
    screens: Res<GameScreens>,
    mut icons: ResMut<PanelIcons>,
    debug: Res<DebugGuiForm>,
    menu: Option<Res<MenuScreen>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut frames: ResMut<GuiFrames>,
    mut doll: Option<ResMut<crate::doll_view::InventoryDoll>>,
    level: Option<Res<exa_ui::GuiLevelState>>,
) {
    frames.clear();
    if menu.is_some_and(|menu| menu.open) {
        return;
    }
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let unit = windows
        .single()
        .ok()
        .map(|window| exa_ui::scale::gui_scale(window.physical_height(), 0) as i32)
        .unwrap_or(4);
    // The windows draw in the task's order, the last raised on top.
    let order = screens.task.draw_order();
    let mut open = screens.open.clone();
    open.sort_by_key(|screen| order.iter().position(|&id| id == screen.form));
    for screen in &open {
        let Some(form) = screens.task.form(screen.form) else {
            continue;
        };
        let bounds = form.bounds();
        let hover = screens.hover_mark(screen.form);
        let records = form.base().map(|f| f.records.clone());
        let painted = match screen.kind {
            GameScreen::Inventory => {
                let paint = doll.as_deref().map(|doll| crate::doll_view::DollPaint {
                    punch: doll.scene.is_some(),
                    shown_icon: doll.shown.map_or(0, |shown| shown.icon),
                    dots: doll.dots,
                    sheet: doll.sheet.as_ref(),
                });
                let (painted, backdrop) = paint_inventory(
                    bounds,
                    &screens.inventory_layout(),
                    unit,
                    textures,
                    &screens.pack,
                    Some(&mut icons),
                    paint.as_ref(),
                    hover,
                    records.as_deref(),
                );
                if let (Some(backdrop), Some(doll)) = (backdrop, doll.as_deref_mut())
                    && doll.backdrop != backdrop
                {
                    doll.backdrop = backdrop;
                }
                painted
            }
            GameScreen::Journal => {
                let text_box = screens.journal_box.as_ref();
                paint_journal(
                    bounds,
                    JournalPaint {
                        journal: &screens.journal,
                        layout: &ScreenLayout::journal(screens.journal_unit),
                        records: records.as_deref(),
                        text_box,
                        box_hover: text_box.map_or(0, |b| screens.hover_mark(b.form)),
                        glyphs: level.as_deref(),
                        combo: Some((
                            screens.journal_combo.view,
                            screens
                                .task
                                .form(screen.form)
                                .and_then(|f| f.base())
                                .is_some_and(|f| f.state & exa_ui::form::state::CARET_LIT != 0),
                        )),
                    },
                    unit,
                    textures,
                    hover,
                )
            }
            GameScreen::Loot => match screens.loot.iter().find(|w| w.form == screen.form) {
                Some(window) => paint_loot(bounds, unit, textures, window, Some(&mut icons), hover),
                None => continue,
            },
            GameScreen::Container => match screens.popups.iter().find(|p| p.form == screen.form) {
                Some(popup) => paint_container_popup(bounds, unit, popup, Some(&mut icons), hover),
                None => continue,
            },
            _ => continue,
        };
        painted.submit(&mut frames);
    }
    // The journal's drop-down list over the windows.
    if let Some(open) = screens.journal_list.as_ref() {
        let list_unit = screens.journal_unit.max(1);
        let level = exa_ui::gui_font::level_for_unit(list_unit);
        let (font, size) = screen_font(exa_ui::gui_font::gui_level(level).main, textures);
        let mut painted = gui_paint::Painted::default();
        crate::option_forms::paint_drop_list(
            &mut painted,
            &open.list,
            open.rect,
            (list_unit, 1.0),
            &gui_paint::palette(),
            (&font, size),
        );
        painted.submit(&mut frames);
    }
    // The default tooltip over the windows.
    if let Some(mut tip) = screens.tooltip.clone() {
        let level = exa_ui::gui_font::level_for_unit(unit);
        let (font, atlas) = screen_font(exa_ui::gui_font::gui_level(level).main, textures);
        let screen = screens.task.screen;
        let palette = gui_paint::palette();
        if let Some(layers) = tip.paint(screens.task.clock, unit, screen, &font, atlas, &palette) {
            let mut painted = gui_paint::Painted::default();
            for layer in layers {
                painted.push(layer);
            }
            painted.submit(&mut frames);
        }
    }
    if debug.enabled {
        let rect = GuiRect::new(100, 100, 302, 202);
        gui_paint::paint_debug_form(rect, unit, textures).submit(&mut frames);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn client_of(bounds: GuiRect, spacing: i32) -> (i32, i32) {
        (bounds.left + spacing, bounds.top + spacing)
    }

    #[test]
    fn the_inventory_records_lay_out_where_the_constructor_places_them() {
        let bounds = GuiRect::new(8, 8, 244, 644);
        let (left, top) = client_of(bounds, gui_paint::INVENTORY_SPACING);
        let controls = screen_controls(GameScreen::Inventory, bounds);
        assert_eq!(controls.len(), 4);
        let place = |c: &Control| (c.x - left, c.y - top, c.w, c.h);
        assert_eq!(controls[0].class, 5);
        assert_eq!((place(&controls[0]).0, place(&controls[0]).1), (4, 386));
        assert_eq!(controls[1].class, RATING_DOTS_CLASS);
        // The rating dots sit on the label's row, 100 in, 12 × 28: their
        // 22-pixel dots draw 3 below its top.
        assert_eq!(place(&controls[1]), (100, 386, 12, 28));
        assert_eq!(
            crate::doll_view::dot_top(controls[1].y, controls[1].h, 22) - top,
            389
        );
        assert_eq!(controls[2].class, 1);
        assert_eq!(place(&controls[2]), (4, 416, 108, 24));
        // Secondary follows Primary on its row, four after it; both are
        // in group 1 and Primary is lit.
        assert_eq!(place(&controls[3]), (116, 416, 108, 24));
        assert_eq!((controls[2].group, controls[3].group), (1, 1));
        assert_eq!(
            (controls[2].state & 0x60, controls[3].state & 0x60),
            (0x60, 0)
        );
    }

    /// At unit 4 the layout is the constructor's design: 244 × 644 at
    /// (8, 8), the character view 228 × 384 at the client's top, the pack
    /// 228 × 180 at its bottom, the records where the layout pass puts them;
    /// the title moves the client's top two pixels below the border (two
    /// units and the border metric 2), so the view and the records sit two
    /// lower.
    #[test]
    fn the_inventory_layout_at_unit_four_is_the_constructors() {
        let layout = InventoryLayout::at(4);
        assert_eq!(layout.size, (244, 644));
        assert_eq!(layout.place, (8, 8));
        assert_eq!(layout.border, 8);
        assert_eq!(
            InventoryLayout::title_shift(layout.size, layout.border, 4),
            2
        );
        assert_eq!(layout.doll, GuiRect::new(8, 10, 228, 384));
        assert_eq!(layout.pack, GuiRect::new(8, 456, 228, 180));
        assert_eq!(layout.measured_client().top, 10);
        let bounds = GuiRect::new(8, 8, 244, 644);
        assert_eq!(layout.pack_at(bounds), inventory_pack_panel(bounds));
        let design = screen_controls(GameScreen::Inventory, bounds);
        let laid = layout.controls_at(bounds);
        for (a, b) in design.iter().zip(&laid) {
            assert_eq!((a.x, a.y + 2, a.w, a.h), (b.x, b.y, b.w, b.h));
        }
    }

    /// At unit 8 (a 1440-pixel window) the form is rescaled from the
    /// design: the border 8 becomes 16, the 228 × 628 client 456 × 1256,
    /// the children keep their 2-pixel borders around doubled clients
    /// (224 × 380 → 448 × 760, 224 × 176 → 448 × 352) and the pack's top
    /// 448 → 896. The records stay between the view and the pack.
    #[test]
    fn the_inventory_layout_scales_with_the_gui_unit() {
        let layout = InventoryLayout::at(8);
        assert_eq!(layout.size, (488, 1288));
        assert_eq!(layout.place, (16, 16));
        assert_eq!(layout.border, 16);
        assert_eq!(layout.doll, GuiRect::new(16, 18, 452, 764));
        assert_eq!(layout.pack, GuiRect::new(16, 912, 452, 356));
        let label = &layout.controls[0];
        let button = &layout.controls[2];
        assert!(label.y >= layout.doll.bottom, "{label:?}");
        assert!(button.y > label.y);
        assert!(button.y + button.h <= layout.pack.top, "{button:?}");
        // Unit 5 (the user's 1014-pixel retail window): border 10, client
        // 285 × 785.
        let layout = InventoryLayout::at(5);
        assert_eq!(layout.size, (305, 805));
        assert_eq!(layout.border, 10);
    }

    /// The loot window and the journal rescale the same way: at unit 4 they
    /// are their design, at unit 8 their 2-pixel borders stay and their
    /// clients and children scale by `/ 4 · 8` (the journal's 514-pixel
    /// client truncates to 1024).
    #[test]
    fn the_loot_window_and_journal_scale_with_the_gui_unit() {
        let loot = ScreenLayout::loot(4);
        assert_eq!(loot.size, (244, 440));
        assert_eq!(loot.place, LOOT_PLACE);
        let bounds = GuiRect::new(400, 36, 244, 440);
        assert_eq!(
            loot_panel_rects_at(bounds, 4),
            (
                GuiRect::new(402, 294, 228, 180),
                GuiRect::new(402, 38, 228, 252)
            )
        );
        let loot = ScreenLayout::loot(8);
        assert_eq!(loot.size, (484, 876));
        assert_eq!(loot.place, (800, UNPLACED));
        assert_eq!(loot.border, 2);
        assert_eq!(loot.children[0], GuiRect::new(2, 514, 452, 356));
        assert_eq!(loot.children[1], GuiRect::new(2, 2, 452, 500));
        let journal = ScreenLayout::journal(4);
        assert_eq!(journal.size, (244, 518));
        assert_eq!(journal.children[0], GuiRect::new(2, 90, 228, 412));
        let bounds = GuiRect::new(4, 36, 244, 518);
        let design = screen_controls(GameScreen::Journal, bounds);
        for (a, b) in design.iter().zip(&journal.controls_at(bounds)) {
            assert_eq!((a.x, a.y, a.w, a.h), (b.x, b.y, b.w, b.h));
        }
        let journal = ScreenLayout::journal(8);
        assert_eq!(journal.size, (484, 1028));
        assert_eq!(journal.place, (UNPLACED, UNPLACED));
        assert_eq!(journal.children[0], GuiRect::new(2, 178, 456, 824));
        // The open takes the unit: a journal opened at unit 8 is 484 wide.
        let mut screens = GameScreens::default();
        screens.task.screen = (2560, 1440);
        screens.open_journal_at(true, true, 8);
        let form = screens.find(GameScreen::Journal).unwrap().form;
        assert_eq!(screens.task.form(form).unwrap().bounds().width(), 484);
        screens.open_loot(7, 0, 8);
        let window = screens.loot_window(7).unwrap();
        assert_eq!(window.pack.unit, 8);
        let bounds = screens.task.form(window.form).unwrap().bounds();
        assert_eq!((bounds.left, bounds.width()), (800, 484));
    }

    /// The openers keep a new window on the screen (the GUI's clamp): a
    /// remembered journal place past the right edge comes back flush with
    /// it, one above the top comes down to a unit.
    #[test]
    fn a_new_journal_is_clamped_to_the_screen() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1280, 720);
        screens.journal.place = Some((1200, -30));
        screens.open_journal_at(true, true, 4);
        let form = screens.find(GameScreen::Journal).unwrap().form;
        let bounds = screens.task.form(form).unwrap().bounds();
        assert_eq!((bounds.left, bounds.top), (1280 - 244, 4));
        assert_eq!(bounds.width(), 244);
    }

    #[test]
    fn the_journal_records_lay_out_where_the_constructor_places_them() {
        let bounds = GuiRect::new(4, 36, 244, 518);
        let (left, top) = client_of(bounds, gui_paint::FORM_SPACING);
        let controls = screen_controls(GameScreen::Journal, bounds);
        assert_eq!(controls.len(), 3);
        let place = |c: &Control| (c.x - left, c.y - top, c.w, c.h);
        assert_eq!(place(&controls[0]), (4, 20, 104, 24));
        assert_eq!(place(&controls[1]), (120, 20, 104, 24));
        // The list on the row below the buttons (anchor 0x8007).
        assert_eq!(place(&controls[2]), (4, 52, 220, 24));
    }

    #[test]
    fn the_inventory_opens_focuses_and_toggles() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.player = 7;
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Opened);
        let form = screens.find(GameScreen::Inventory).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(8, 8, 244, 644)
        );
        // The new window comes to the front; the key focus stays with the
        // form under the pointer (the forced pointer update ends the open).
        assert_eq!(screens.active_screen(), Some(GameScreen::Inventory));
        assert_eq!(screens.focused(), None);
        assert!(screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Closed);
        assert!(!screens.is_open(GameScreen::Inventory));
        assert!(!screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.inventory_place, Some((8, 8)));
        assert_eq!(
            screens.open_inventory(false, true, 4),
            ScreenAction::Refused
        );
    }

    /// Another character's inventory: the same show rule, registered under
    /// that character's tag, placed at the screen's right edge less its
    /// width and four units; its close does not move the player's
    /// remembered place. Asking for the player's own closes it first
    /// (Exarust keeps one inventory window).
    #[test]
    fn another_characters_inventory_opens_at_the_right_edge() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.player = 7;
        screens.inventory_place = Some((30, 40));
        assert_eq!(
            screens.show_inventory(true, true, 4, true, 9),
            ScreenAction::Opened
        );
        assert_eq!(screens.inventory_character(), Some(9));
        let form = screens.find(GameScreen::Inventory).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(1600 - 244 - 16, 8, 244, 644)
        );
        assert!(screens.containers.is_open(0x2000_0009));
        assert_eq!(screens.pack.owner, Some(9));
        // A script's call leaves the active window; the interaction's
        // toggles it shut.
        assert_eq!(
            screens.show_inventory(true, true, 4, false, 9),
            ScreenAction::Refused
        );
        assert_eq!(
            screens.show_inventory(true, true, 4, true, 9),
            ScreenAction::Closed
        );
        assert!(!screens.containers.is_open(0x2000_0009));
        assert_eq!(screens.inventory_place, Some((30, 40)));
        // The companion's, then the player's own: the first closes.
        screens.show_inventory(true, true, 4, true, 9);
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Opened);
        assert_eq!(screens.inventory_character(), Some(7));
        assert!(!screens.containers.is_open(0x2000_0009));
        assert!(screens.containers.is_open(0x2000_0007));
        let form = screens.find(GameScreen::Inventory).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(30, 40, 244, 644)
        );
    }

    /// The inventory's fill sets the view's mind window for a shown body a
    /// Loyalty or possession marker binds, and not for a free one.
    #[test]
    fn the_inventory_of_a_bound_body_sets_its_mind_window() {
        use crate::mind::casts::{LOYALTY_MARK, MindBody, MindClass, MindPower};
        let run = |bound: bool| {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<GameScreens>()
                .add_message::<ScreenRequest>()
                .init_resource::<crate::thaum::live::FrameEffectRoll>()
                .init_resource::<crate::thaum::live::FrameEffectDraws>()
                .add_systems(Update, open_requested_screens);
            let mut mind = MindBody::new(9);
            if bound {
                mind.powers.push(MindPower::new(
                    LOYALTY_MARK,
                    MindClass::LoyaltyMark(crate::thaum::power::mind::LoyaltyTarget::new(1)),
                ));
            }
            let body = app
                .world_mut()
                .spawn((exa_items::possession::Possessions::default(), mind))
                .id();
            app.world_mut().write_message(ScreenRequest {
                screen: GameScreen::Inventory,
                player_ready: true,
                player_alive: true,
                target: Some(body.index().index() as i32),
                flags: 0,
                toggle: true,
            });
            app.update();
            let screens = app.world().resource::<GameScreens>();
            assert!(screens.is_open(GameScreen::Inventory));
            screens.inventory_mind_window
        };
        assert!(run(true));
        assert!(!run(false));
    }

    /// The open inventory lights the page button of the set its body has
    /// out, and a swap while it is open lights the other one and loads the
    /// pack again with the held marks the swap moved.
    #[test]
    fn the_inventory_follows_the_shown_bodys_weapon_set() {
        use exa_items::possession::{Carried, HELD, Hand, Possessions, SECOND_SET};
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<exa_items::catalogue::ItemCatalogue>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(
                Update,
                (
                    open_requested_screens,
                    exa_items::catalogue::swap_weapon_sets,
                    inventory_follows_the_set,
                )
                    .chain(),
            );
        let carried = |id: u32, flags: u32| Carried {
            id,
            flags,
            x: 0,
            y: 0,
        };
        let body = app
            .world_mut()
            .spawn((
                crate::player::PlayerBody::new(0.0),
                Possessions {
                    flags: SECOND_SET,
                    out: [Hand { id: 5, flags: 0 }, Hand::default()],
                    stowed: [Hand { id: 6, flags: 0 }, Hand::default()],
                    carried: vec![carried(5, HELD), carried(6, 0)],
                    ..Default::default()
                },
            ))
            .id();
        app.world_mut().resource_mut::<GameScreens>().task.screen = (1600, 900);
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        app.update();
        let states = |app: &App| {
            let screens = app.world().resource::<GameScreens>();
            let form = screens.find(GameScreen::Inventory).unwrap().form;
            let base = screens.task.form(form).unwrap().base().unwrap();
            [base.records[2].state, base.records[3].state]
        };
        let held = |app: &App| {
            let screens = app.world().resource::<GameScreens>();
            let mut held: Vec<u32> = screens
                .pack
                .entries
                .iter()
                .filter(|e| e.flags & HELD != 0)
                .map(|e| e.id)
                .collect();
            held.sort_unstable();
            held
        };
        // The second set is out: Secondary is lit.
        assert_eq!(states(&app), [0, 0x60]);
        assert_eq!(held(&app), [5]);
        app.world_mut()
            .entity_mut(body)
            .insert(exa_items::catalogue::SwapRequested);
        app.update();
        assert_eq!(states(&app), [0x60, 0]);
        assert_eq!(held(&app), [6]);
    }

    #[test]
    fn closing_a_panel_queues_its_icon_handles() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_inventory(true, true, 4);
        screens.pack.entries = vec![
            crate::item_container::Entry {
                icon: 3,
                ..Default::default()
            },
            crate::item_container::Entry {
                icon: 7,
                ..Default::default()
            },
        ];
        assert!(screens.close(GameScreen::Inventory));
        assert_eq!(screens.released_icons, vec![3, 7]);
        // The destroyed panel keeps no entries.
        assert!(screens.pack.entries.is_empty());
        // A loot window queues both of its panels' handles.
        screens.open_loot(7, 0, 4);
        screens.loot[0].pack.entries = vec![crate::item_container::Entry {
            icon: 4,
            ..Default::default()
        }];
        screens.loot[0].worn.entries = vec![crate::item_container::Entry {
            icon: 5,
            ..Default::default()
        }];
        assert!(screens.close(GameScreen::Loot));
        assert_eq!(screens.released_icons, vec![3, 7, 4, 5]);
    }

    #[test]
    fn an_inactive_screen_is_focused_not_closed() {
        let mut screens = GameScreens::default();
        screens.open_inventory(true, true, 4);
        screens.open_journal(true, true);
        assert_eq!(screens.active_screen(), Some(GameScreen::Journal));
        assert_eq!(screens.open_inventory(true, true, 4), ScreenAction::Focused);
        assert_eq!(screens.active_screen(), Some(GameScreen::Inventory));
        assert_eq!(screens.close_container_family(), 2);
        assert!(screens.open.is_empty());
    }

    #[test]
    fn entering_combat_closes_every_game_screen() {
        let mut screens = GameScreens::default();
        screens.open_inventory(true, true, 4);
        screens.open_journal(true, true);
        assert_eq!(screens.open_loot(7, 0, 4), ScreenAction::Opened);
        assert_eq!(screens.open_loot(9, 0, 4), ScreenAction::Opened);
        assert_eq!(screens.open.len(), 4);
        assert_eq!(screens.close_game_screens(), 4);
        assert!(screens.open.is_empty());
        assert!(screens.loot.is_empty());
        assert_eq!(screens.task.active, None);
        // The journal's close stored its text and place on the way.
        assert!(screens.journal.place.is_some());
        assert_eq!(screens.close_game_screens(), 0);
    }

    #[test]
    fn a_save_saves_the_open_screens_without_closing_them() {
        use exa_ui::game_gui::SaveAction;
        let mut screens = GameScreens::default();
        assert!(screens.save_open_screens().is_empty());
        screens.open_journal(true, true);
        screens.open_inventory(true, true, 4);
        screens.journal_text = "The well is dry.".into();
        // The inventory's unloaded panel asks for nothing; the journal
        // stores its text.
        assert_eq!(screens.save_open_screens(), vec![SaveAction::Journal]);
        assert_eq!(screens.journal.entries[0].text, "The well is dry.");
        assert_eq!(screens.open.len(), 2);
        screens.pack.loaded = true;
        assert_eq!(
            screens.save_open_screens(),
            vec![SaveAction::Journal, SaveAction::Inventory]
        );
        assert!(screens.saved_containers.is_empty());
    }

    /// A game save writes the open inventory's loaded pack into its body's
    /// carried list, in the panel's order and at its places; a carried item
    /// the panel does not show stays first.
    #[test]
    fn a_save_writes_the_open_packs_into_the_carried_lists() {
        use exa_items::possession::{Carried, Possessions};
        let mut app = App::new();
        app.init_resource::<GameScreens>()
            .init_resource::<exa_assets::live_objects::LiveObjects>()
            .add_message::<crate::save_live::SaveToNewSlot>()
            .add_message::<crate::save_live::SaveStep>()
            .add_systems(Update, save_open_screens_for_saves::<false>);
        let carried = |id, x| Carried {
            id,
            flags: 0,
            x,
            y: 0,
        };
        let body = app
            .world_mut()
            .spawn(Possessions {
                carried: vec![carried(10, 0), carried(11, 0), carried(12, 0)],
                ..Default::default()
            })
            .id();
        {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            screens.open_inventory(true, true, 4);
            screens.inventory_body = Some(body);
            screens.pack.loaded = true;
            for (id, x) in [(11, 64), (10, 128)] {
                screens.pack.entries.push(crate::item_container::Entry {
                    cell: crate::item_container::Cell::new(x, 0),
                    id,
                    ..Default::default()
                });
            }
        }
        app.world_mut()
            .write_message(crate::save_live::SaveStep::Save {
                drop_on_low_health: false,
            });
        app.update();
        let list = &app.world().get::<Possessions>(body).unwrap().carried;
        let pack = app.world().resource::<GameScreens>().pack.save_list();
        assert_eq!(
            list.iter().map(|c| c.id).collect::<Vec<_>>(),
            vec![12, 11, 10]
        );
        assert_eq!((list[1].x, list[1].y), (pack[0].x, pack[0].y));
        assert_eq!((list[2].x, list[2].y), (pack[1].x, pack[1].y));
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        );
    }

    #[test]
    fn the_sweeps_close_the_screens_in_the_screens_world() {
        let mut world = World::new();
        let mut screens = GameScreens::default();
        screens.open_inventory(true, true, 4);
        screens.open_journal(true, true);
        world.insert_resource(screens);
        sweep_screens(&mut world, ScreenSweep::ContainerFamily);
        assert!(world.resource::<GameScreens>().open.is_empty());
        let mut screens = world.resource_mut::<GameScreens>();
        screens.open_journal(true, true);
        sweep_screens(&mut world, ScreenSweep::Every);
        assert!(world.resource::<GameScreens>().open.is_empty());
        assert_eq!(ScreenSweep::ContainerFamily.bar_mask(), 1);
        assert_eq!(ScreenSweep::Every.bar_mask(), 0);
        // A world without the screens (or the frame's queue) is left alone.
        sweep_screens(&mut World::new(), ScreenSweep::Every);
    }

    #[test]
    fn the_popup_takes_the_container_constructors_words() {
        // The card's style (0x3080) with the panel bit cleared, the
        // attribute word written over; the show commits the corner frame
        // and the title and shadow bits of a top-level form.
        let words = gui_paint::CONTAINER_WORDS;
        assert_eq!((words.style, words.attributes), (0x3000, 0x83_0023));
        let committed = gui_paint::commit_words(words.style, words.attributes, true);
        assert_eq!((committed.style, committed.attributes), (0x3010, 0x83_0063));
        assert_eq!(POPUP_BORDER, 8);
    }

    #[test]
    fn a_container_opened_by_the_live_request_keeps_its_children_inside() {
        let mut app = App::new();
        app.init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<crate::doll_view::DollItems>()
            .add_message::<ContainerRequest>()
            .add_systems(Update, open_requested_containers);
        app.world_mut().resource_mut::<GameScreens>().task.screen = (1280, 720);
        // A child stored far outside its 50 by 30 unit container.
        let object = app
            .world_mut()
            .spawn(exa_world::ObjectSections {
                placement: 0,
                usable: None,
                use_script: None,
                container: Some(exa_formats::containers::Container {
                    width: 50,
                    height: 30,
                    capacity: 400,
                    max_item: 60,
                    children: vec![exa_formats::containers::Child {
                        id: 9,
                        flags: 0,
                        x: 8000,
                        y: -4000,
                    }],
                }),
            })
            .id();
        let id = exa_formats::objdb::LOCAL | 4;
        app.world_mut().write_message(ContainerRequest {
            object,
            id,
            pointer: IVec2::new(640, 400),
        });
        app.update();
        let screens = app.world().resource::<GameScreens>();
        let popup = screens.popup(id).expect("the popup opened");
        let icons = app.world().resource::<PanelIcons>();
        let entry = popup.panel.entries[0];
        let half = icons.half(entry.icon).unwrap();
        let centre = (
            entry.cell.x + popup.panel.icon_size / 2,
            entry.cell.y + popup.panel.icon_size / 2,
        );
        // The open's clamp keeps the icon's centre at least its half
        // extents inside the client on each side.
        assert_eq!(centre.0, popup.panel.client.0 - half.0);
        assert_eq!(centre.1, half.1);
    }

    #[test]
    fn the_container_popup_opens_on_the_pointer_and_saves_on_close() {
        use crate::item_container::Cell;
        let mut screens = GameScreens::default();
        screens.task.screen = (1280, 720);
        let object = Entity::from_bits(42);
        let id = exa_formats::objdb::LOCAL | 4;
        let record = ContainerRecord {
            size: (50, 30),
            capacity: 400,
            max_item: 60,
            children: vec![ContainerChild {
                id: 9,
                flags: 0x2000_0000,
                x: 800,
                y: 400,
            }],
        };
        let mut slots = 10..;
        let action =
            screens.open_container_popup(object, id, &record, (640, 400), 4, 3, 70, |_, flags| {
                // The resolver sees the flags' low 28 bits.
                assert_eq!(flags, 0);
                Some((slots.next().unwrap(), 5))
            });
        assert_eq!(action, ScreenAction::Opened);
        assert_eq!(screens.active_screen(), Some(GameScreen::Container));
        assert!(screens.containers.is_open(id as i32));
        let popup = screens.popup(id).unwrap();
        let bounds = screens.task.form(popup.form).unwrap().bounds();
        assert_eq!(bounds, GuiRect::new(528, 124, 224, 308));
        assert_eq!(
            popup.panel_rect(bounds),
            GuiRect::new(536, 132 + 164, 208, 128)
        );
        assert_eq!(popup.panel.client, (200, 120));
        assert_eq!((popup.panel.capacity, popup.panel.used), (400, 5));
        assert_eq!(popup.panel.entries[0].cell, Cell::new(40, 20));
        assert_eq!(popup.panel.entries[0].flags, 0x2000_0000);
        // The same container again only comes forward.
        assert_eq!(
            screens.open_container_popup(object, id, &record, (0, 0), 4, 3, 70, |_, _| None),
            ScreenAction::Focused
        );
        // Moving the entry and closing saves it back, in 1/80 units.
        screens.popups[0].panel.entries[0].cell = Cell::new(8, 4);
        assert!(screens.close_popup(id));
        assert!(screens.popups.is_empty());
        assert!(!screens.containers.is_open(id as i32));
        let (saved_object, saved_id, saved) = screens.saved_containers.pop().unwrap();
        assert_eq!(saved_object, object);
        assert_eq!(saved_id, id);
        assert_eq!(
            (saved.size, saved.capacity, saved.max_item),
            ((50, 30), 400, 60)
        );
        assert_eq!(
            saved.children,
            vec![ContainerChild {
                id: 9,
                flags: 0x2000_0000,
                x: 160,
                y: 80,
            }]
        );
        assert_eq!(screens.released_icons, vec![10, 3]);
        // The section takes the saved children.
        let mut section = exa_formats::containers::Container {
            width: 50,
            height: 30,
            capacity: 400,
            max_item: 60,
            children: Vec::new(),
        };
        store_record(&mut section, &saved);
        assert_eq!(container_record(&section), saved);
        // A container without an id opens nothing.
        assert_eq!(
            screens.open_container_popup(object, 0, &record, (0, 0), 4, 0, 70, |_, _| None),
            ScreenAction::Refused
        );
    }

    #[test]
    fn the_loot_window_opens_per_body_and_remembers_its_panels() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        assert_eq!(screens.open_loot(7, 0, 4), ScreenAction::Opened);
        let form = screens.loot_window(7).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(400, 36, 244, 440)
        );
        assert_eq!(screens.active_screen(), Some(GameScreen::Loot));
        assert!(screens.containers.is_open(0x2000_0007));
        // The same body again focuses its window; another body opens a
        // second one and keeps the first.
        assert_eq!(screens.open_loot(7, 0, 4), ScreenAction::Focused);
        assert_eq!(screens.open_loot(8, 0, 4), ScreenAction::Opened);
        assert_eq!(screens.loot.len(), 2);
        assert!(screens.containers.is_open(0x2000_0008));
        assert_eq!(screens.active_screen(), Some(GameScreen::Loot));
        // The active window (body 8) closes; body 7's stays open.
        assert!(screens.close_focused());
        assert!(!screens.containers.is_open(0x2000_0008));
        assert!(screens.containers.is_open(0x2000_0007));
        assert_eq!(screens.loot.len(), 1);
        // A body whose flag word's second byte is 3 is refused.
        assert_eq!(screens.open_loot(9, 0x300, 4), ScreenAction::Refused);
        assert!(screens.loot_window(9).is_none());
    }

    #[test]
    fn the_loot_panels_bind_the_bodys_items() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_loot(7, 0, 4);
        let mut pack = Container::form(4);
        pack.entries = vec![crate::item_container::Entry {
            cell: crate::item_container::Cell::new(10, 20),
            id: 7,
            icon: 1,
            flags: 0,
            drag: 0,
        }];
        let mut worn = Container::form(4);
        worn.entries = vec![crate::item_container::Entry {
            cell: crate::item_container::Cell::new(30, 40),
            id: 8,
            icon: 1,
            flags: 0,
            drag: 0,
        }];
        assert!(screens.bind_loot(7, pack, worn));
        assert_eq!(screens.loot_window(7).unwrap().pack.entries[0].id, 7);
        assert_eq!(screens.loot_window(7).unwrap().worn.entries[0].id, 8);
        assert!(!screens.bind_loot(99, Container::default(), Container::default()));
    }

    #[test]
    fn a_loot_window_closes_when_the_body_is_no_longer_dead() {
        let mut app = App::new();
        app.add_systems(Update, close_changed_loot_windows);
        let body = app.world_mut().spawn(exa_combat::Condition::DEAD).id();
        let target = body.index().index() as i32;
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        assert_eq!(screens.open_loot(target, 0, 4), ScreenAction::Opened);
        app.insert_resource(screens);
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .loot_window(target)
                .is_some()
        );
        app.world_mut()
            .get_mut::<exa_combat::Condition>(body)
            .unwrap()
            .0 = exa_combat::Condition::NORMAL.0;
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .loot_window(target)
                .is_none()
        );
    }

    /// A script's `ShowInventory` of the player opens the inventory and
    /// does not close it again. Another character, and a dead player, open
    /// nothing.
    #[test]
    fn a_script_opens_the_players_inventory_without_closing_it() {
        fn opened(actor: u32, condition: exa_combat::Condition) -> bool {
            let mut app = App::new();
            app.init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<GameScreens>()
                .init_resource::<PanelIcons>()
                .add_message::<ScreenRequest>()
                .add_message::<exa_script::ScriptEffect>()
                .init_resource::<crate::thaum::live::FrameEffectRoll>()
                .init_resource::<crate::thaum::live::FrameEffectDraws>()
                .add_systems(
                    Update,
                    (show_script_inventory, open_requested_screens).chain(),
                );
            app.world_mut().spawn((
                crate::player::PlayerBody::new(0.0),
                exa_ai::brain::CharacterId(7),
                condition,
            ));
            app.world_mut().write_message(exa_script::ScriptEffect(
                exa_script::dialogue::Effect::ShowInventory(actor),
            ));
            app.update();
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        }
        assert!(opened(7, exa_combat::Condition::NORMAL));
        assert!(!opened(9, exa_combat::Condition::NORMAL));
        assert!(!opened(7, exa_combat::Condition::DEAD));

        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .add_message::<exa_script::ScriptEffect>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(
                Update,
                (show_script_inventory, open_requested_screens).chain(),
            );
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_ai::brain::CharacterId(7),
            exa_combat::Condition::NORMAL,
        ));
        let ask = |app: &mut App| {
            app.world_mut().write_message(exa_script::ScriptEffect(
                exa_script::dialogue::Effect::ShowInventory(7),
            ));
            app.update();
        };
        ask(&mut app);
        ask(&mut app);
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        );
    }

    /// The inventory's bind counts what the player wears against the pack:
    /// the used capacity is the carried items' bulk plus the worn items'
    /// (the record's worn-weight rule), read from the live possessions.
    #[test]
    fn the_inventory_bind_charges_the_pack_with_the_worn_bulk() {
        use exa_formats::objdb::{Item, LOCAL};
        // Each copy weighs as the template its header names (10 + its id).
        let item = |id: u32, bulk: u8| Item {
            model: format!("item{id}"),
            equip_lock: bulk,
            parent: if id < 10 { LOCAL | (id + 10) } else { 0 },
            ..Item::parse(id, &[]).expect("an empty item")
        };
        let mut catalogue = exa_items::catalogue::ItemCatalogue::default();
        catalogue.set_level_items(&[
            item(1, 0),
            item(2, 0),
            item(3, 0),
            item(11, 3),
            item(12, 5),
            item(13, 7),
        ]);
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .insert_resource(catalogue)
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_items::possession::Possessions {
                carried: vec![exa_items::possession::Carried {
                    id: LOCAL | 1,
                    flags: 0,
                    x: 0x28,
                    y: 4,
                }],
                worn: vec![
                    exa_formats::characters::ItemRef::new(LOCAL | 2, 0),
                    exa_formats::characters::ItemRef::new(LOCAL | 3, 0),
                ],
                ..Default::default()
            },
        ));
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert_eq!(screens.pack.entries.len(), 1);
        assert_eq!(screens.pack.used, 3 + 5 + 7);
    }

    /// The openers' toggles read the draw order, not the hover focus: the
    /// journal on top closes on its key even while the pointer focuses the
    /// inventory under it; the inventory's key then raises it (the
    /// activation), and only a second press closes it. The journal is
    /// found by its tag on the task's children.
    #[test]
    fn the_toggles_close_the_topmost_window_and_raise_the_others() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_items::possession::Possessions::default(),
        ));
        let ask = |app: &mut App, screen: GameScreen| {
            app.world_mut()
                .write_message(ScreenRequest::screen(screen, true, true));
            app.update();
        };
        ask(&mut app, GameScreen::Inventory);
        ask(&mut app, GameScreen::Journal);
        let (inventory, journal) = {
            let screens = app.world().resource::<GameScreens>();
            let inventory = screens.find(GameScreen::Inventory).unwrap().form;
            let journal = screens.find(GameScreen::Journal).unwrap().form;
            assert_eq!(
                screens.task.find_by_tag(exa_ui::game_gui::JOURNAL_SCREEN),
                Some(journal)
            );
            assert!(screens.task.is_active(journal));
            assert!(!screens.task.is_active(inventory));
            (inventory, journal)
        };
        // The pointer's hover focuses the inventory under the journal.
        app.world_mut()
            .resource_mut::<GameScreens>()
            .task
            .focus_set(Some(inventory), NO_CONTROL);
        assert_eq!(
            app.world().resource::<GameScreens>().focused(),
            Some(GameScreen::Inventory)
        );
        // The inventory's key raises the inventory over the journal.
        ask(&mut app, GameScreen::Inventory);
        {
            let screens = app.world().resource::<GameScreens>();
            assert!(screens.is_open(GameScreen::Inventory));
            assert!(screens.task.is_active(inventory));
            assert_eq!(screens.task.draw_order().last(), Some(&inventory));
        }
        // The journal's key now raises the journal (it is not on top) ...
        ask(&mut app, GameScreen::Journal);
        assert!(
            app.world()
                .resource::<GameScreens>()
                .task
                .is_active(journal)
        );
        // ... and closes it once it is, whatever the hover focuses.
        app.world_mut()
            .resource_mut::<GameScreens>()
            .task
            .focus_set(Some(inventory), NO_CONTROL);
        ask(&mut app, GameScreen::Journal);
        let screens = app.world().resource::<GameScreens>();
        assert!(!screens.is_open(GameScreen::Journal));
        assert!(screens.is_open(GameScreen::Inventory));
        assert_eq!(
            screens.task.find_by_tag(exa_ui::game_gui::JOURNAL_SCREEN),
            None
        );
    }

    /// Losing the system's focus moves the window manager's focus off the
    /// journal's text box to the root (control -1), remembering the box; a
    /// later focus to no form goes back to it.
    #[test]
    fn the_focus_loss_parks_the_focus_on_the_root_and_remembers_the_box() {
        let mut app = App::new();
        app.init_resource::<GameScreens>()
            .add_message::<bevy::window::WindowFocused>()
            .add_systems(Update, deactivate_on_focus_loss);
        let window = app.world_mut().spawn_empty().id();
        let text_box = {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            screens.open_journal(true, true);
            let text_box = screens.journal_box.as_ref().unwrap().form;
            // The pointer rests on the box: its hover focuses it.
            let b = screens.task.form(text_box).unwrap().bounds();
            screens.task.screen = (4096, 4096);
            screens
                .task
                .update(false, ((b.left + b.right) / 2, (b.top + b.bottom) / 2));
            assert!(screens.journal_box_focused());
            text_box
        };
        app.world_mut().write_message(bevy::window::WindowFocused {
            window,
            focused: false,
        });
        app.update();
        {
            let screens = app.world().resource::<GameScreens>();
            assert!(!screens.journal_box_focused());
            assert_eq!(screens.task.active, screens.task.root);
            assert_eq!(screens.task.active_control, -1);
            assert_eq!(screens.task.remembered.map(|r| r.0), Some(text_box));
        }
        // The focus's return is a focus to no form: the box is focused
        // again.
        app.world_mut().write_message(bevy::window::WindowFocused {
            window,
            focused: true,
        });
        app.update();
        assert!(app.world().resource::<GameScreens>().journal_box_focused());
    }

    /// The main menu's and the power tree's tasks are told about the focus
    /// loss too: their focused control deselects and the root keeps the
    /// focus with no control.
    #[test]
    fn the_focus_loss_reaches_the_menu_and_the_tree_tasks() {
        let mut app = App::new();
        app.add_message::<bevy::window::WindowFocused>()
            .add_systems(
                Startup,
                (
                    crate::menu_screen::setup_router,
                    crate::power_screen::setup_router,
                ),
            )
            .add_systems(Update, deactivate_on_focus_loss);
        app.update();
        let window = app.world_mut().spawn_empty().id();
        {
            let world = app.world_mut();
            // The pointer rests on each task's root.
            let mut menu = world.resource_mut::<crate::menu_screen::MenuRouter>();
            let root = menu.task.root;
            menu.task.screen = (4096, 4096);
            menu.task.update(false, (1, 1));
            menu.task.focus_set(root, 2);
            assert_eq!(menu.task.active_control, 2);
            let mut tree = world.resource_mut::<crate::power_screen::PowerRouter>();
            let root = tree.task.root;
            tree.task.screen = (4096, 4096);
            tree.task.update(false, (1, 1));
            tree.task.focus_set(root, 1);
        }
        app.world_mut().write_message(bevy::window::WindowFocused {
            window,
            focused: false,
        });
        app.update();
        let world = app.world();
        let menu = &world.resource::<crate::menu_screen::MenuRouter>().task;
        assert_eq!((menu.active, menu.active_control), (menu.root, -1));
        let tree = &world.resource::<crate::power_screen::PowerRouter>().task;
        assert_eq!((tree.active, tree.active_control), (tree.root, -1));
    }

    /// The journal's key closes it through the base form's close: its text
    /// box closes with it, and the window manager forgets both (the focus,
    /// the pointer's target, the capture and the remembered focus no longer
    /// name them); the focus falls to the window left.
    #[test]
    fn the_journals_show_cuts_its_snap_slots_around_the_text_box() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_items::possession::Possessions::default(),
        ));
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Journal, true, true));
        app.update();
        let screens = app.world().resource::<GameScreens>();
        let journal = screens.find(GameScreen::Journal).unwrap().form;
        let text_box = screens.journal_box.as_ref().unwrap().form;
        let journal = screens.task.form(journal).and_then(|f| f.base()).unwrap();
        let text_box = screens.task.form(text_box).and_then(|f| f.base()).unwrap();
        // The box's client in the journal's client coordinates.
        let (ox, oy) = (
            journal.bounds.left + journal.client.left,
            journal.bounds.top + journal.client.top,
        );
        let inner = [
            text_box.bounds.left - ox + text_box.client.left,
            text_box.bounds.top - oy + text_box.client.top,
            text_box.bounds.left - ox + text_box.client.right,
            text_box.bounds.top - oy + text_box.client.bottom,
        ];
        assert!(!journal.snaps.is_empty());
        assert_eq!(journal.state & 0x2_0000, 0);
        for slot in &journal.snaps {
            let apart = slot[2] <= inner[0]
                || inner[2] <= slot[0]
                || slot[3] <= inner[1]
                || inner[3] <= slot[1];
            assert!(apart, "slot {slot:?} overlaps the text box {inner:?}");
        }
        // The box holds no form of its own: it is no snap container with
        // children, and its own slots (no snapping child) are empty.
        assert!(text_box.snaps.is_empty());
    }

    #[test]
    fn closing_the_journal_closes_its_text_box_and_clears_the_references() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_items::possession::Possessions::default(),
        ));
        let ask = |app: &mut App, screen: GameScreen| {
            app.world_mut()
                .write_message(ScreenRequest::screen(screen, true, true));
            app.update();
        };
        ask(&mut app, GameScreen::Inventory);
        ask(&mut app, GameScreen::Journal);
        let (inventory, journal, text_box) = {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            let inventory = screens.find(GameScreen::Inventory).unwrap().form;
            let journal = screens.find(GameScreen::Journal).unwrap().form;
            let text_box = screens.journal_box.as_ref().unwrap().form;
            screens.task.focus_set(Some(journal), NO_CONTROL);
            screens.task.focus_set(Some(text_box), NO_CONTROL);
            screens.task.pointer.target = Some(text_box);
            screens.task.pointer.capture = Some(text_box);
            assert_eq!(screens.task.remembered.map(|r| r.0), Some(journal));
            (inventory, journal, text_box)
        };
        ask(&mut app, GameScreen::Journal);
        let screens = app.world().resource::<GameScreens>();
        assert!(!screens.is_open(GameScreen::Journal));
        assert!(screens.task.form(journal).is_none());
        assert!(screens.task.form(text_box).is_none());
        assert!(screens.journal_box.is_none());
        // The hide's forced pointer update: the focus goes to the form under
        // the pointer, none here (no pointer was read yet).
        assert_eq!(screens.task.active, None);
        assert!(screens.task.is_active(inventory));
        assert_eq!(screens.task.pointer.target, None);
        assert_eq!(screens.task.pointer.capture, None);
        assert!(
            screens
                .task
                .remembered
                .is_none_or(|(f, _)| f != journal && f != text_box)
        );
        assert_eq!(screens.task.draw_order().len(), 2);
    }

    /// A window grown past a height step changes the GUI unit, and the
    /// rescale walk takes the open screens with it: the inventory's place
    /// scales as a free form's, its size, client and records are its
    /// layout's at the new unit and its pack's entries rescale; the
    /// journal and its text box follow.
    #[test]
    fn a_unit_change_rescales_the_open_screens() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_message::<ScreenRequest>()
            .add_systems(
                Update,
                (
                    sync_screen_size,
                    rescale_open_screens,
                    open_requested_screens,
                )
                    .chain(),
            );
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_items::possession::Possessions::default(),
        ));
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1280, 720),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        for screen in [GameScreen::Inventory, GameScreen::Journal] {
            app.world_mut()
                .write_message(ScreenRequest::screen(screen, true, true));
            app.update();
        }
        let inventory = {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            assert_eq!(screens.inventory_unit, 4);
            screens.pack.entries.push(crate::item_container::Entry {
                cell: crate::item_container::Cell::new(40, 10),
                id: 0x30,
                ..Default::default()
            });
            screens.find(GameScreen::Inventory).unwrap().form
        };
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set_physical_resolution(1920, 1080);
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert_eq!(screens.inventory_unit, 6);
        let layout = InventoryLayout::at(6);
        let form = screens.task.form(inventory).and_then(|f| f.base()).unwrap();
        assert_eq!(form.unit, 6);
        assert_eq!(
            form.bounds,
            GuiRect::new(12, 12, layout.size.0, layout.size.1)
        );
        assert_eq!(form.records, layout.controls_at(form.bounds));
        assert_eq!(
            screens.pack.entries[0].cell,
            crate::item_container::Cell::new(60, 15)
        );
        assert_eq!(screens.pack.unit, 6);
        assert_eq!(screens.journal_unit, 6);
        let text_box = screens.journal_box.as_ref().unwrap().form;
        assert_eq!(
            screens
                .task
                .form(text_box)
                .and_then(|f| f.base())
                .unwrap()
                .unit,
            6
        );
        // One text box: the old one closed with the walk.
        assert_eq!(screens.task.draw_order().len(), 4);
    }

    /// The window's size reaches the task as the level switch takes it:
    /// the pointer's drag distance follows the height (a
    /// five-hundred-and-fortieth, at least 2).
    #[test]
    fn the_drag_distance_follows_the_window_height() {
        let mut app = App::new();
        app.init_resource::<GameScreens>()
            .add_systems(Update, sync_screen_size);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(3840, 2160),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.update();
        let task = &app.world().resource::<GameScreens>().task;
        assert_eq!((task.screen, task.pointer.limit), ((3840, 2160), 4));
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set_physical_resolution(1280, 720);
        app.update();
        let task = &app.world().resource::<GameScreens>().task;
        assert_eq!((task.screen, task.pointer.limit), ((1280, 720), 2));
    }

    /// A container popup follows a unit change too: its panel is laid out
    /// from the record's cells at the new unit, its items' places scale,
    /// and its form takes the new size at its scaled place.
    #[test]
    fn a_unit_change_rescales_an_open_container_popup() {
        let mut app = App::new();
        app.init_resource::<GameScreens>()
            .add_systems(Update, (sync_screen_size, rescale_open_screens).chain());
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1280, 720),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.update();
        let record = ContainerRecord {
            size: (50, 30),
            capacity: 400,
            max_item: 60,
            children: Vec::new(),
        };
        let form = {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            let id = exa_formats::objdb::LOCAL | 4;
            let action = screens.open_container_popup(
                Entity::from_bits(42),
                id,
                &record,
                (640, 400),
                4,
                3,
                70,
                |_, _| None,
            );
            assert_eq!(action, ScreenAction::Opened);
            let popup = screens.popups.last_mut().unwrap();
            popup.panel.entries.push(crate::item_container::Entry {
                cell: crate::item_container::Cell::new(40, 10),
                id: 9,
                ..Default::default()
            });
            popup.form
        };
        let before = app
            .world()
            .resource::<GameScreens>()
            .task
            .form(form)
            .and_then(|f| f.base())
            .unwrap()
            .bounds;
        app.world_mut()
            .get_mut::<Window>(window)
            .unwrap()
            .resolution
            .set_physical_resolution(1920, 1080);
        app.update();
        let screens = app.world().resource::<GameScreens>();
        let popup = &screens.popups[0];
        assert_eq!(popup.panel.unit, 6);
        assert_eq!(
            popup.panel.entries[0].cell,
            crate::item_container::Cell::new(60, 15)
        );
        let plan = open_container(
            false,
            Some(record.size),
            6,
            POPUP_PANEL_BORDER,
            popup.icon_height,
        )
        .unwrap();
        assert_eq!(popup.plan, plan);
        let size = popup_form_size(plan, 6, POPUP_BORDER);
        let form = screens.task.form(form).and_then(|f| f.base()).unwrap();
        assert_eq!(form.unit, 6);
        assert_eq!(
            form.bounds,
            GuiRect::new(before.left / 4 * 6, before.top / 4 * 6, size.0, size.1)
        );
    }

    #[test]
    fn the_inventory_request_binds_the_players_carried_list() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_items::possession::Possessions {
                carried: vec![exa_items::possession::Carried {
                    id: 0x30,
                    flags: 0,
                    x: 0x28,
                    y: 4,
                }],
                ..Default::default()
            },
        ));
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert_eq!(screens.pack.entries.len(), 1);
        assert_eq!(screens.pack.entries[0].id, 0x30);
        // The panel's icon half extents carry the GUI unit, so the top
        // clamp lifts the saved y of 1 to 4, as the icon's own halves do.
        assert_eq!(
            screens.pack.entries[0].cell,
            crate::item_container::Cell::new(10, 4)
        );
    }

    #[test]
    fn the_loot_request_binds_the_bodys_record_and_its_condition_flags() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        let record = exa_formats::characters::CharacterRecord {
            hands: [
                exa_formats::characters::ItemRef::new(0x11, 0),
                exa_formats::characters::ItemRef::new(0x12, 0),
                exa_formats::characters::ItemRef::default(),
                exa_formats::characters::ItemRef::default(),
            ],
            worn: vec![exa_formats::characters::ItemRef::new(0xa0, 0)],
            inventory: vec![exa_formats::characters::Carried {
                item: 0x30,
                flags: 0,
                extra: 0,
            }],
            race: 1,
            ..exa_formats::characters::CharacterRecord::default()
        };
        let entity = app
            .world_mut()
            .spawn((
                exa_items::possession::Possessions::of_record(&record),
                exa_combat::Condition::DEAD,
            ))
            .id();
        let target = entity.index().index() as i32;
        // The request's flags are stale; the body's record flag word (its
        // profession word, race 1 here) is what the show rule reads.
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Loot,
            player_ready: true,
            player_alive: true,
            target: Some(target),
            flags: 0x300,
            toggle: true,
        });
        app.update();
        let screens = app.world().resource::<GameScreens>();
        let window = screens.loot_window(target).unwrap();
        assert_eq!(window.flags, 0x100);
        assert_eq!(window.pack.entries.len(), 1);
        assert_eq!(window.pack.entries[0].id, 0x30);
        // Worn first, then only the shields in the hands: with no item
        // catalogue nothing in a hand is known to be a shield.
        let ids: Vec<u32> = window.worn.entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![0xa0]);
        // The scatter drew the worn item's spot (x, then y) from the
        // frame's engine generator, and its run-time seed restarted the
        // frame's run-time stream.
        let mut roll = app
            .world_mut()
            .resource_mut::<crate::thaum::live::FrameEffectRoll>();
        assert_eq!(*roll.word(), 0x0808_8406);
        assert_eq!(
            app.world()
                .resource::<crate::thaum::live::FrameEffectDraws>()
                .0
                .0,
            crate::arena::Mt19937::new(crate::item_container::DOLL_SEED)
        );
    }

    /// A body carrying a shipped item and an id no record holds: the loot
    /// request's pack lists only the item the acquire gives an icon, and
    /// the other is reported as unresolved (shipped records, skipped
    /// without the game).
    #[test]
    fn the_loot_pack_leaves_out_an_item_without_a_model() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let Ok(data) = exa_assets::GameData::open(&dir) else {
            return;
        };
        let Some(db) = data
            .pack(exa_assets::PackKind::Resource)
            .and_then(|pack| pack.get("objdb.rdb"))
            .and_then(|bytes| exa_formats::rdb::Database::parse(bytes).ok())
        else {
            return;
        };
        let catalogue = exa_items::catalogue::ItemCatalogue::of_database(&db);
        let shipped = exa_formats::objdb::items(&db)
            .into_iter()
            .map(|item| item.id)
            .find(|&id| catalogue.model(id).is_some())
            .expect("a shipped item with a model");
        let unknown = 0x0F_FFF2;
        assert!(catalogue.model(unknown).is_none());
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .insert_resource(catalogue)
            .add_message::<ScreenRequest>()
            .add_systems(Update, open_requested_screens);
        let record = exa_formats::characters::CharacterRecord {
            inventory: [shipped, unknown]
                .into_iter()
                .map(|item| exa_formats::characters::Carried {
                    item,
                    flags: 0,
                    extra: 0,
                })
                .collect(),
            race: 1,
            ..exa_formats::characters::CharacterRecord::default()
        };
        let entity = app
            .world_mut()
            .spawn((
                exa_items::possession::Possessions::of_record(&record),
                exa_combat::Condition::DEAD,
            ))
            .id();
        let target = entity.index().index() as i32;
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Loot,
            player_ready: true,
            player_alive: true,
            target: Some(target),
            flags: 0,
            toggle: true,
        });
        app.update();
        let screens = app.world().resource::<GameScreens>();
        let window = screens.loot_window(target).unwrap();
        let ids: Vec<u32> = window.pack.entries.iter().map(|e| e.id).collect();
        assert_eq!(ids, vec![shipped]);
        assert!(window.pack.entries[0].icon != 0);
        assert!(
            crate::engine_log::overlay_lines()
                .iter()
                .any(|line| line.contains("Unable to resolve item: 000FFFF2")),
            "{:?}",
            crate::engine_log::overlay_lines()
        );
    }

    #[test]
    fn a_body_of_race_three_refuses_the_loot_window() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .init_resource::<PanelIcons>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        let record = exa_formats::characters::CharacterRecord {
            race: 3,
            ..exa_formats::characters::CharacterRecord::default()
        };
        let entity = app
            .world_mut()
            .spawn((
                exa_items::possession::Possessions::of_record(&record),
                exa_combat::Condition::DEAD,
            ))
            .id();
        let target = entity.index().index() as i32;
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Loot,
            player_ready: true,
            player_alive: true,
            target: Some(target),
            flags: 0,
            toggle: true,
        });
        app.update();
        // 0x300 & 0xff00 == 0x300: the show rule refuses.
        assert!(
            app.world()
                .resource::<GameScreens>()
                .loot_window(target)
                .is_none()
        );
    }

    #[test]
    fn the_loot_form_draws_its_two_panels() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        app.world_mut().write_message(ScreenRequest {
            screen: GameScreen::Loot,
            player_ready: true,
            player_alive: true,
            target: Some(7),
            flags: 0,
            toggle: true,
        });
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert!(screens.loot_window(7).is_some());
        // The loot form has no control records; its paint is the frame and
        // the two item panels.
        assert!(screen_controls(GameScreen::Loot, GuiRect::new(400, 36, 244, 440)).is_empty());
        let window = screens.loot_window(7).unwrap();
        let painted = paint_loot(
            GuiRect::new(400, 36, 244, 440),
            4,
            &GuiTextures::default(),
            window,
            None,
            0,
        );
        // The window's shadow and backdrop, the two held panels' frames
        // (no backdrop of their own) and the window's corner frame.
        assert_eq!(painted.layers.len(), 5);
        // The pack panel's frame lies on its edges, at the client's left
        // and its bottom less 180; the equipment panel above it ends four
        // units before. A corner not flush with the client's edges curls
        // two units outside.
        let client = painted.client;
        assert_eq!(
            (client.left, client.top, client.width(), client.height()),
            (402, 38, 240, 436)
        );
        let corner = |layer: usize| {
            painted.layers[layer]
                .batch
                .vertices
                .iter()
                .map(|v| (v.x, v.y))
                .fold((f32::MAX, f32::MAX), |a, b| (a.0.min(b.0), a.1.min(b.1)))
        };
        let near = |value: f32, edge: i32| (edge - 8..=edge).contains(&(value as i32));
        let pack = corner(2);
        assert!(near(pack.0, client.left), "{pack:?}");
        assert!(near(pack.1, client.bottom - 180), "{pack:?}");
        let equipment = corner(3);
        assert!(near(equipment.0, client.left), "{equipment:?}");
        assert!(near(equipment.1, client.top), "{equipment:?}");
        // The window's backdrop covers the panels (held, they have none).
        let textures = GuiTextures::default();
        for at in [(500, 100), (500, 400)] {
            assert_eq!(
                canvas_colour_at(&painted.layers, &textures, at),
                [0, 0, 0, 170],
                "{at:?}"
            );
        }
    }

    #[test]
    fn the_deferred_close_takes_the_windows_of_dead_bodies_only() {
        let mut screens = GameScreens::default();
        screens.task.screen = (1600, 900);
        screens.open_loot(7, 0, 4);
        screens.open_loot(8, 0, 4);
        // 7 is still dead and unmarked; 8 is no longer dead; 9 has no window.
        assert_eq!(
            screens.loot_windows_to_close(&[(7, 0, 0xa00), (8, 0, 0), (9, 0x80000, 0xa00)]),
            vec![8]
        );
        assert_eq!(
            screens.loot_windows_to_close(&[(7, 0x80000, 0xa00)]),
            vec![7]
        );
        // Both close in the windows' order, whatever the bodies' order.
        assert_eq!(
            screens.loot_windows_to_close(&[(8, 0, 0), (7, 0, 0)]),
            vec![7, 8]
        );
    }

    #[test]
    fn the_journal_opens_and_remembers() {
        let mut screens = GameScreens::default();
        assert_eq!(screens.open_journal(true, true), ScreenAction::Opened);
        let form = screens.find(GameScreen::Journal).unwrap().form;
        assert_eq!(
            screens.task.form(form).unwrap().bounds(),
            GuiRect::new(4, 36, 244, 518)
        );
        screens.journal_text = "a note".to_string();
        assert_eq!(screens.open_journal(true, true), ScreenAction::Closed);
        assert_eq!(screens.journal.entries[0].text, "a note");
        assert_eq!(screens.journal.place, Some((4, 36)));
        assert_eq!(screens.open_journal(false, true), ScreenAction::Refused);
    }

    #[test]
    fn the_draw_path_runs_for_three_frames_on_an_empty_app() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        for _ in 0..3 {
            app.update();
        }
        assert!(app.world().resource::<GuiFrames>().frames.is_empty());
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        for _ in 0..3 {
            app.update();
        }
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        assert!(frames.primitives() > 0);
    }

    #[test]
    fn an_inventory_request_submits_a_frame_with_the_skins_modes() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        // The shadow first, reaching eight units out from the form; then
        // the backdrop from the form's corner.
        let shadow = &frames.frames[0].layers[0];
        let first = &shadow.batch.vertices_of(&shadow.batch.primitives[0])[0];
        assert_eq!((first.x, first.y), (8.0 - 32.0, 8.0 - 32.0));
        let layer = &frames.frames[0].layers[1];
        assert!(!layer.batch.primitives.is_empty());
        let first = &layer.batch.vertices_of(&layer.batch.primitives[0])[0];
        assert_eq!((first.x, first.y), (8.0, 8.0));
        assert!(layer.batch.primitives.iter().any(|primitive| {
            matches!(
                primitive.mode,
                exa_ui::skin::SkinMode::Solid
                    | exa_ui::skin::SkinMode::Rounded
                    | exa_ui::skin::SkinMode::Skin
            )
        }));
    }

    #[test]
    fn a_pack_entry_with_an_icon_draws_through_the_panel_program() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        {
            let mut icons = app.world_mut().resource_mut::<PanelIcons>();
            let mut image = exa_ui::icon_atlas::IconImage::new(160, 160, 4);
            image.pixels.fill(255);
            let cell = icons.atlas.add(&image, 7);
            icons.cache.entries = vec![exa_items::icon_cache::Entry {
                id: 7,
                cell,
                variant: 0,
                half_width: 84,
                half_height: 84,
                refs: 1,
            }];
        }
        {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            screens.pack.icon_size = 160;
            screens.pack.entries = vec![crate::item_container::Entry {
                cell: crate::item_container::Cell::new(10, 20),
                id: 7,
                icon: 1,
                flags: 0,
                drag: 0,
            }];
        }
        let request = ScreenRequest::screen(GameScreen::Inventory, true, true);
        app.world_mut().write_message(request);
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        let panel: Vec<_> = frames.frames[0]
            .layers
            .iter()
            .filter(|layer| layer.batch.mode == exa_ui::skin::SkinMode::HltCol)
            .collect();
        assert_eq!(panel.len(), 1);
        assert_eq!(panel[0].batch.primitives.len(), 1);
        // The cell is placed inside the pack panel's client area and the
        // layer is clipped to it.
        let clip = panel[0].clip.unwrap();
        let first = panel[0].batch.vertices_of(&panel[0].batch.primitives[0])[0];
        assert_eq!(first.x, (clip[0] + 10) as f32);
        assert!(first.y >= clip[1] as f32);
    }

    /// Rasterises the open inventory with the installed skin and fonts at a
    /// window's size and unit; `EXA_GUI_DUMP` names a raw RGBA file (the
    /// width and height in its name) to look at by eye. Skipped without an
    /// installation.
    fn rasterise_inventory(width: u32, height: u32) -> Option<(Vec<u8>, GuiRect)> {
        use exa_ui::gui_draw::{GuiTexture, GuiTextures};
        let dir = exa_assets::locate_install(None)?;
        let game = exa_assets::Game(std::sync::Arc::new(exa_assets::GameData::open(&dir).ok()?));
        let unit = exa_ui::scale::gui_scale(height, 0) as i32;
        let level = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(unit));
        let mut textures = GuiTextures::default();
        for name in [
            exa_ui::gui_draw::SKIN_TEXTURE,
            level.main.texture,
            level.small.texture,
            exa_ui::gui_font::FONTBASE_24.texture,
        ] {
            if let Ok(image) = game.load_rgba(name)
                && let Some(texture) =
                    GuiTexture::from_bottom_up(image.width, image.height, &image.pixels)
            {
                textures.textures.insert(name.to_string(), texture);
            }
        }
        let mut screens = GameScreens::default();
        screens.task.screen = (width as i32, height as i32);
        screens.open_inventory(true, true, unit);
        let form = screens.find(GameScreen::Inventory)?.form;
        let bounds = screens.task.form(form)?.bounds();
        let layout = screens.inventory_layout();
        let (painted, _) = paint_inventory(
            bounds,
            &layout,
            unit,
            &textures,
            &screens.pack,
            None,
            None,
            0,
            None,
        );
        let mut frames = GuiFrames::default();
        painted.submit(&mut frames);
        let pixels = exa_ui::gui_draw::rasterise(&frames.frames, &textures, width, height);
        if let Ok(path) = std::env::var("EXA_GUI_DUMP") {
            let _ = std::fs::write(path, &pixels);
        }
        Some((pixels, bounds))
    }

    /// With the view drawing its figure, the canvas is cleared over the
    /// view (224 × 380 inside the doll panel's 2-pixel border) and the
    /// colour it had there comes back as the camera's backdrop; the rating
    /// dots add one layer of their quads.
    #[test]
    fn the_inventory_clears_the_view_and_returns_its_backdrop() {
        let bounds = GuiRect::new(8, 8, 244, 644);
        let layout = InventoryLayout::at(4);
        let textures = GuiTextures::default();
        let cell = vec![255u8; 64 * 64 * 4];
        let sheet = crate::doll_view::DotSheet::of_cells(&[cell.clone(), cell], 64, 22).unwrap();
        let paint = crate::doll_view::DollPaint {
            punch: true,
            shown_icon: 0,
            dots: Some(2.5),
            sheet: Some(&sheet),
        };
        let pack = Container::default();
        let (plain, none) =
            paint_inventory(bounds, &layout, 4, &textures, &pack, None, None, 0, None);
        assert_eq!(none, None);
        let (painted, backdrop) = paint_inventory(
            bounds,
            &layout,
            4,
            &textures,
            &pack,
            None,
            Some(&paint),
            0,
            None,
        );
        // The clearing layer and the dots' layer.
        assert_eq!(painted.layers.len(), plain.layers.len() + 2);
        let view = crate::doll_view::view_rect(layout.doll_at(bounds), DOLL_BORDER);
        assert_eq!(
            (view.left, view.top, view.width(), view.height()),
            (18, 20, 224, 380)
        );
        let centre = ((view.left + view.right) / 2, (view.top + view.bottom) / 2);
        assert_eq!(
            canvas_colour_at(&plain.layers, &textures, centre),
            backdrop.unwrap()
        );
        // The form's translucent black backdrop (the held view has none).
        assert_eq!(backdrop, Some([0, 0, 0, 170]));
        assert_eq!(canvas_colour_at(&painted.layers, &textures, centre), [0; 4]);
    }

    /// The inventory paints its frame and its panels over the whole window,
    /// not only the label and button records.
    #[test]
    fn the_inventory_paints_a_visible_frame_and_panels() {
        let Some((pixels, bounds)) = rasterise_inventory(2560, 1440) else {
            return;
        };
        let covered = |rect: GuiRect| {
            let mut count = 0usize;
            let mut total = 0usize;
            for y in rect.top.max(0)..rect.bottom.min(1440) {
                for x in rect.left.max(0)..rect.right.min(2560) {
                    total += 1;
                    if pixels[(y as usize * 2560 + x as usize) * 4 + 3] > 0 {
                        count += 1;
                    }
                }
            }
            (count, total)
        };
        let (count, total) = covered(bounds);
        eprintln!("inventory {bounds:?}: {count} of {total} pixels drawn");
        assert!(count * 2 > total, "{count} of {total}");
        let pack = InventoryLayout::at(8).pack_at(bounds);
        let (count, total) = covered(pack);
        assert!(count * 2 > total, "pack {count} of {total}");
    }

    #[test]
    fn without_the_level_font_the_base_atlas_is_sampled_at_the_levels_size() {
        use exa_ui::gui_draw::{GuiTexture, GuiTextures};
        let level = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(4)).main;
        let mut textures = GuiTextures::default();
        // Nothing installed: no size, the base name.
        let (font, size) = screen_font(level, &textures);
        assert_eq!(font.texture, "fontbase24r");
        assert_eq!(size, None);
        textures.textures.insert(
            "fontbase24r".into(),
            GuiTexture::from_top_down(1024, 1024, &vec![0u8; 1024 * 1024 * 4]).unwrap(),
        );
        let (font, size) = screen_font(level, &textures);
        // The level's quad, advances and line height stay.
        assert_eq!(font.glyph, level.glyph);
        assert_eq!(font.line_height, level.line_height);
        assert_eq!(font.advances, level.advances);
        // The sample step is still the base glyph's: 36 and 48 texels.
        let (du, dv) = exa_ui::gui_text::font_uv(&font, size.unwrap()).unwrap();
        assert!((du - 36.0 / 1024.0).abs() < 1e-4, "{du}");
        assert!((dv - 48.0 / 1024.0).abs() < 1e-4, "{dv}");
        // With the level's own texture the level font is used as is.
        textures
            .textures
            .insert(level.texture.into(), GuiTexture::solid([255; 4]));
        let (font, size) = screen_font(level, &textures);
        assert_eq!(font.texture, level.texture);
        assert_eq!(size, Some((1, 1)));
    }

    #[test]
    fn the_journals_selected_text_reaches_the_font_layer() {
        use exa_ui::gui_draw::{GuiTexture, GuiTextures, LayerTexture};
        let mut textures = GuiTextures::default();
        let fonts = exa_ui::gui_font::gui_level(exa_ui::gui_font::level_for_unit(4));
        // The text box writes in the level's small font.
        let font = fonts.small;
        for texture in [fonts.main.texture, fonts.small.texture] {
            textures
                .textures
                .insert(texture.into(), GuiTexture::solid([255; 4]));
        }
        textures.textures.insert(
            exa_ui::gui_draw::SKIN_TEXTURE.into(),
            GuiTexture::solid([10, 20, 30, 255]),
        );
        let journal = Journal {
            entries: vec![
                exa_ui::game_gui::JournalEntry {
                    id: 0,
                    title: "First".into(),
                    text: "A note".into(),
                },
                exa_ui::game_gui::JournalEntry {
                    id: 0,
                    title: "Second".into(),
                    text: "Another note".into(),
                },
            ],
            current: 1,
            place: None,
        };
        let painted = paint_journal(
            GuiRect::new(4, 36, 244, 518),
            JournalPaint {
                journal: &journal,
                layout: &ScreenLayout::journal(4),
                records: None,
                text_box: None,
                box_hover: 0,
                glyphs: None,
                combo: None,
            },
            4,
            &textures,
            0,
        );
        let font_layers: Vec<_> = painted
            .layers
            .iter()
            .filter(
                |layer| matches!(layer.texture, LayerTexture::Named(name) if name == font.texture),
            )
            .collect();
        assert!(!font_layers.is_empty());
        let glyphs: usize = font_layers
            .iter()
            .map(|layer| {
                layer
                    .batch
                    .primitives
                    .iter()
                    .map(|p| p.count)
                    .sum::<usize>()
            })
            .sum();
        assert!(glyphs >= "Another note".len() * 4);
        // Two buttons and the title are also drawn.
        assert!(painted.layers.len() >= 4);
    }

    #[test]
    fn a_journal_with_two_entries_draws_the_selected_text() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .add_plugins(GameScreensPlugin);
        {
            let mut screens = app.world_mut().resource_mut::<GameScreens>();
            screens.journal.entries = vec![
                exa_ui::game_gui::JournalEntry {
                    id: 0,
                    title: "First".into(),
                    text: "A note".into(),
                },
                exa_ui::game_gui::JournalEntry {
                    id: 0,
                    title: "Second".into(),
                    text: "Another note".into(),
                },
            ];
            screens.journal.current = 1;
        }
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Journal, true, true));
        app.update();
        let frames = app.world().resource::<GuiFrames>();
        assert_eq!(frames.frames.len(), 1);
        let primitives: usize = frames.frames[0]
            .layers
            .iter()
            .map(|layer| layer.batch.primitives.len())
            .sum();
        assert!(primitives > 0);
    }

    /// An app with the screens' plugin, a 1600 × 900 window (GUI unit 5),
    /// a scripted pointer, the flow's step list and an open journal.
    fn journal_app() -> (App, Entity) {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<MouseButton>>()
            .init_resource::<crate::flow::FlowSteps>()
            .init_resource::<exa_ui::ScriptedPointer>()
            .add_plugins(GameScreensPlugin);
        let window = app
            .world_mut()
            .spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1600, 900),
                    ..default()
                },
                PrimaryWindow,
            ))
            .id();
        app.update();
        app.world_mut()
            .write_message(ScreenRequest::screen(GameScreen::Journal, true, true));
        app.update();
        (app, window)
    }

    /// The journal box's client centre and a point `dy` pixels below its
    /// top-left corner (screen pixels).
    fn journal_box_points(app: &App) -> ((f32, f32), (f32, f32)) {
        let screens = app.world().resource::<GameScreens>();
        let form = screens
            .task
            .form(screens.journal_box.as_ref().unwrap().form)
            .unwrap()
            .base()
            .unwrap();
        let (x0, y0) = (
            form.bounds.left + form.client.left,
            form.bounds.top + form.client.top,
        );
        (
            (
                (x0 + form.client.width() / 2) as f32,
                (y0 + form.client.height() / 2) as f32,
            ),
            (x0 as f32 + 1.0, y0 as f32 + 2.0),
        )
    }

    /// One frame with a mouse button pressed (or released); the frame's
    /// press and release marks are then cleared, as the input plugin does.
    fn button(app: &mut App, which: MouseButton, down: bool) {
        {
            let mut mouse = app.world_mut().resource_mut::<ButtonInput<MouseButton>>();
            if down {
                mouse.press(which);
            } else {
                mouse.release(which);
            }
        }
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<MouseButton>>()
            .clear();
    }

    fn point(app: &mut App, at: (f32, f32)) {
        app.world_mut().resource_mut::<exa_ui::ScriptedPointer>().0 = Some(Vec2::new(at.0, at.1));
    }

    fn type_text(app: &mut App, window: Entity, text: &str) {
        use bevy::input::ButtonState;
        use bevy::input::keyboard::{Key, KeyboardInput};
        for c in text.chars() {
            app.world_mut().write_message(KeyboardInput {
                key_code: KeyCode::KeyA,
                logical_key: Key::Character(c.to_string().into()),
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        }
        app.update();
    }

    fn press_key(app: &mut App, window: Entity, code: KeyCode, key: bevy::input::keyboard::Key) {
        use bevy::input::ButtonState;
        app.world_mut()
            .write_message(bevy::input::keyboard::KeyboardInput {
                key_code: code,
                logical_key: key,
                state: ButtonState::Pressed,
                text: None,
                repeat: false,
                window,
            });
        app.update();
    }

    /// The journal's text box takes the keys while the window manager's
    /// focus is on it (the pointer over it): typed letters go in at the
    /// caret, Backspace and Enter edit, and the player's hotkeys (J here)
    /// are not reached. The close stores the typed text in the entry.
    #[test]
    fn the_journal_text_box_takes_typing_while_focused() {
        use bevy::input::keyboard::Key;
        let (mut app, window) = journal_app();
        let (centre, _) = journal_box_points(&app);
        // Away from the box the keys are not the box's.
        point(&mut app, (1500.0, 800.0));
        app.update();
        assert!(!app.world().resource::<GameScreens>().journal_box_focused());
        type_text(&mut app, window, "x");
        assert_eq!(app.world().resource::<GameScreens>().journal_text, "");
        point(&mut app, centre);
        app.update();
        assert!(app.world().resource::<GameScreens>().journal_box_focused());
        type_text(&mut app, window, "Wellj");
        press_key(&mut app, window, KeyCode::Backspace, Key::Backspace);
        press_key(&mut app, window, KeyCode::Enter, Key::Enter);
        type_text(&mut app, window, "dry");
        let screens = app.world().resource::<GameScreens>();
        assert_eq!(screens.journal_text, "Well\r\ndry");
        assert!(
            screens.is_open(GameScreen::Journal),
            "J typed, not the hotkey"
        );
        let text_box = &screens.journal_box.as_ref().unwrap().text;
        assert_eq!(text_box.caret, 9);
        assert_eq!(text_box.lines, 2);
        // The right release over the journal closes it, storing the text.
        button(&mut app, MouseButton::Right, true);
        button(&mut app, MouseButton::Right, false);
        let screens = app.world().resource::<GameScreens>();
        assert!(!screens.is_open(GameScreen::Journal));
        assert!(screens.journal_box.is_none());
        assert_eq!(screens.journal.entries[0].text, "Well\r\ndry");
    }

    /// A key with a game screen under the pointer goes to that window's
    /// key slot and climbs to the task: the game view's key handler (the
    /// player's hotkeys) does not hear it, so J over the journal leaves it
    /// open. With the pointer on the view the same key is the hotkey and
    /// closes the journal.
    #[test]
    fn a_key_over_a_game_screen_never_reaches_the_players_hotkeys() {
        use bevy::input::keyboard::Key;
        let (mut app, window) = journal_app();
        app.init_resource::<crate::bindings::Bindings>()
            .init_resource::<crate::hand::Hand>()
            .add_plugins(crate::hotkeys::HotkeyPlugin);
        app.world_mut().spawn((
            crate::player::PlayerBody::new(0.0),
            exa_combat::Condition::NORMAL,
        ));
        let (form, title) = {
            let screens = app.world().resource::<GameScreens>();
            let journal = screens.find(GameScreen::Journal).unwrap().form;
            let b = screens.task.form(journal).unwrap().bounds();
            (
                journal,
                ((b.left + b.right) as f32 / 2.0, b.top as f32 + 6.0),
            )
        };
        point(&mut app, title);
        app.update();
        {
            let screens = app.world().resource::<GameScreens>();
            assert_eq!(screens.task.active, Some(form));
            assert!(screens.keys_to_window());
            assert!(!screens.journal_box_focused());
        }
        let j = |app: &mut App| {
            app.world_mut()
                .resource_mut::<ButtonInput<KeyCode>>()
                .press(KeyCode::KeyJ);
            press_key(app, window, KeyCode::KeyJ, Key::Character("j".into()));
            let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
            keys.release(KeyCode::KeyJ);
            keys.clear();
            app.update();
        };
        j(&mut app);
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Journal)
        );
        point(&mut app, (1590.0, 890.0));
        app.update();
        assert!(!app.world().resource::<GameScreens>().keys_to_window());
        j(&mut app);
        assert!(
            !app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Journal)
        );
    }

    /// Tab in the journal's text box is its callback: combat mode on.
    #[test]
    fn tab_in_the_journal_box_turns_combat_mode_on() {
        use bevy::input::keyboard::Key;
        let (mut app, window) = journal_app();
        let (centre, _) = journal_box_points(&app);
        point(&mut app, centre);
        app.update();
        press_key(&mut app, window, KeyCode::Tab, Key::Tab);
        let steps = &app.world().resource::<crate::flow::FlowSteps>().steps;
        assert!(steps.contains(&crate::flow::Step::CombatMode(true)));
    }

    /// A left press on the text puts the caret under the pointer and
    /// lights it; the caret then blinks every fifth frame and is drawn
    /// while lit.
    #[test]
    fn a_press_on_the_journal_text_places_the_caret() {
        let (mut app, window) = journal_app();
        let (centre, corner) = journal_box_points(&app);
        point(&mut app, centre);
        app.update();
        type_text(&mut app, window, "abc");
        point(&mut app, corner);
        app.update();
        button(&mut app, MouseButton::Left, true);
        {
            let screens = app.world().resource::<GameScreens>();
            let text_box = screens.journal_box.as_ref().unwrap();
            assert_eq!(text_box.text.caret, 0, "the press before the first byte");
            assert!(text_box.text.focused);
        }
        button(&mut app, MouseButton::Left, false);
        let mut lit = Vec::new();
        for _ in 0..10 {
            app.update();
            lit.push(
                app.world()
                    .resource::<GameScreens>()
                    .journal_box
                    .as_ref()
                    .unwrap()
                    .text
                    .focused,
            );
        }
        assert!(lit.contains(&true) && lit.contains(&false), "{lit:?}");
    }

    /// The inventory's Secondary page button, pressed and released, asks
    /// the shown body for the set swap (its inventory is open, so at
    /// once); pressing the lit button again asks for nothing, and a busy
    /// body (a switch in progress) refuses the swap.
    #[test]
    fn the_inventory_page_buttons_swap_the_sets() {
        fn app_with(condition: exa_combat::Condition) -> (App, Entity) {
            let mut app = App::new();
            app.add_plugins(MinimalPlugins)
                .init_resource::<ButtonInput<KeyCode>>()
                .init_resource::<ButtonInput<MouseButton>>()
                .init_resource::<crate::flow::FlowSteps>()
                .init_resource::<exa_ui::ScriptedPointer>()
                .add_plugins(GameScreensPlugin);
            app.world_mut().spawn((
                Window {
                    resolution: bevy::window::WindowResolution::new(1600, 900),
                    ..default()
                },
                PrimaryWindow,
            ));
            let body = app
                .world_mut()
                .spawn((crate::player::PlayerBody::new(0.0), condition))
                .id();
            app.update();
            app.world_mut()
                .write_message(ScreenRequest::screen(GameScreen::Inventory, true, true));
            app.update();
            (app, body)
        }
        let click = |app: &mut App, record: usize| {
            let at = {
                let screens = app.world().resource::<GameScreens>();
                let form = screens.find(GameScreen::Inventory).unwrap().form;
                let r = &screens.task.form(form).unwrap().base().unwrap().records[record];
                ((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32)
            };
            point(app, at);
            app.update();
            button(app, MouseButton::Left, true);
            button(app, MouseButton::Left, false);
        };
        let swapping = |app: &App, body: Entity| {
            app.world()
                .entity(body)
                .contains::<exa_items::catalogue::SwapRequested>()
        };
        let (mut app, body) = app_with(exa_combat::Condition::NORMAL);
        click(&mut app, PAGE_BUTTONS[0]);
        assert!(!swapping(&app, body), "the lit Primary asks for nothing");
        click(&mut app, PAGE_BUTTONS[1]);
        assert!(swapping(&app, body), "Secondary asks for the swap");
        let (mut app, body) = app_with(exa_combat::Condition(0x200));
        click(&mut app, PAGE_BUTTONS[1]);
        assert!(!swapping(&app, body), "a busy body refuses it");
    }

    /// New stores the text, adds a "New Entry" and shows it empty; Erase
    /// removes the selected entry and shows the one left.
    #[test]
    fn the_journal_new_and_erase_buttons() {
        let (mut app, window) = journal_app();
        let (centre, _) = journal_box_points(&app);
        point(&mut app, centre);
        app.update();
        type_text(&mut app, window, "first");
        let release_over = |app: &mut App, record: usize| {
            let at = {
                let screens = app.world().resource::<GameScreens>();
                let form = screens.find(GameScreen::Journal).unwrap().form;
                let r = &screens.task.form(form).unwrap().base().unwrap().records[record];
                ((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32)
            };
            point(app, at);
            app.update();
            button(app, MouseButton::Left, true);
            button(app, MouseButton::Left, false);
        };
        release_over(&mut app, JOURNAL_NEW);
        {
            let screens = app.world().resource::<GameScreens>();
            assert_eq!(screens.journal.entries.len(), 2);
            assert_eq!(screens.journal.entries[0].text, "first");
            assert_eq!(screens.journal.current, 1);
            assert_eq!(screens.journal_text, "");
            let form = screens.find(GameScreen::Journal).unwrap().form;
            let list = &screens.task.form(form).unwrap().base().unwrap().records[JOURNAL_LIST];
            assert_eq!(list.text, b"New Entry");
        }
        release_over(&mut app, JOURNAL_ERASE);
        let screens = app.world().resource::<GameScreens>();
        assert_eq!(screens.journal.entries.len(), 1);
        assert_eq!(screens.journal.current, 0);
        assert_eq!(screens.journal_text, "first");
        assert_eq!(
            screens.journal_box.as_ref().unwrap().text.text.as_bytes(),
            b"first"
        );
    }

    /// The journal's entry list: a press on its arrow opens its drop-down
    /// list of the titles with the selected row chosen; picking the first row
    /// stores the text typed into the second entry and shows the first.
    #[test]
    fn the_journal_list_drops_down_and_picks_an_entry() {
        let (mut app, window) = journal_app();
        let (centre, _) = journal_box_points(&app);
        point(&mut app, centre);
        app.update();
        type_text(&mut app, window, "first");
        let release_over = |app: &mut App, record: usize| {
            let at = {
                let screens = app.world().resource::<GameScreens>();
                let form = screens.find(GameScreen::Journal).unwrap().form;
                let r = &screens.task.form(form).unwrap().base().unwrap().records[record];
                ((r.x + r.w / 2) as f32, (r.y + r.h / 2) as f32)
            };
            point(app, at);
            app.update();
            button(app, MouseButton::Left, true);
            button(app, MouseButton::Left, false);
        };
        release_over(&mut app, JOURNAL_NEW);
        point(&mut app, centre);
        app.update();
        type_text(&mut app, window, "second");
        // The list's arrow (its text part is the edit state).
        let arrow = {
            let screens = app.world().resource::<GameScreens>();
            let form = screens.find(GameScreen::Journal).unwrap().form;
            let r = &screens.task.form(form).unwrap().base().unwrap().records[JOURNAL_LIST];
            ((r.x + r.w - 3) as f32, (r.y + r.h / 2) as f32)
        };
        point(&mut app, arrow);
        app.update();
        button(&mut app, MouseButton::Left, true);
        button(&mut app, MouseButton::Left, false);
        let (rect, row_y) = {
            let screens = app.world().resource::<GameScreens>();
            assert_eq!(screens.journal_row, 2);
            let open = screens.journal_list.as_ref().expect("the list opens");
            assert_eq!(
                open.list.items,
                vec![b"New Entry".to_vec(), b"New Entry".to_vec()]
            );
            assert_eq!(open.list.selected, 2, "the selected entry's row");
            let y = (open.rect.y..open.rect.y + open.rect.h)
                .find(|&y| open.list.row_at(y - open.rect.y) == 1)
                .unwrap();
            (open.rect, y + 2)
        };
        point(&mut app, ((rect.x + rect.w / 2) as f32, row_y as f32));
        app.update();
        button(&mut app, MouseButton::Left, true);
        button(&mut app, MouseButton::Left, false);
        let screens = app.world().resource::<GameScreens>();
        assert!(screens.journal_list.is_none(), "the pick closes the list");
        assert_eq!(screens.journal.current, 0);
        assert_eq!(screens.journal.entries[1].text, "second");
        assert_eq!(screens.journal_text, "first");
        assert_eq!(screens.journal_row, 1);
        assert_eq!(
            screens.journal_box.as_ref().unwrap().text.text.as_bytes(),
            b"first"
        );
    }

    /// The journal's entry list is an editable combo box: New gives it the
    /// key focus with its whole title selected, so typing renames the
    /// entry and its row in the list's titles (the list's edit-state
    /// callback); a press on its arrow opens the drop-down list and takes
    /// the focus off it; a press on its text holds it, places the caret
    /// under the pointer and focuses it again.
    #[test]
    fn the_journal_list_renames_its_entry_as_it_is_typed() {
        let (mut app, window) = journal_app();
        let record = |app: &App, i: usize| {
            let screens = app.world().resource::<GameScreens>();
            let form = screens.find(GameScreen::Journal).unwrap().form;
            screens.task.form(form).unwrap().base().unwrap().records[i].clone()
        };
        let click = |app: &mut App, at: (i32, i32)| {
            point(app, (at.0 as f32, at.1 as f32));
            app.update();
            button(app, MouseButton::Left, true);
            button(app, MouseButton::Left, false);
        };
        let new = record(&app, JOURNAL_NEW);
        click(&mut app, (new.x + new.w / 2, new.y + new.h / 2));
        {
            let screens = app.world().resource::<GameScreens>();
            assert!(screens.journal_combo.focused, "New focuses the list");
            let view = screens.journal_combo.view.unwrap();
            assert_eq!((view.anchor, view.caret), (0, 9), "the whole title");
        }
        assert_ne!(record(&app, JOURNAL_LIST).state & 0x400, 0);
        type_text(&mut app, window, "Camp");
        {
            let screens = app.world().resource::<GameScreens>();
            assert_eq!(screens.journal.entries[1].title, "Camp");
            assert_eq!(screens.journal.entries[0].title, "New Entry");
            assert_eq!(screens.journal_titles[1], b"Camp");
        }
        assert_eq!(record(&app, JOURNAL_LIST).text, b"Camp");
        // The arrow opens the list with the new title.
        let list = record(&app, JOURNAL_LIST);
        let arrow = (list.x + list.w - 3, list.y + list.h / 2);
        click(&mut app, arrow);
        {
            let screens = app.world().resource::<GameScreens>();
            let open = screens
                .journal_list
                .as_ref()
                .expect("the arrow opens the list");
            assert_eq!(
                open.list.items,
                vec![b"New Entry".to_vec(), b"Camp".to_vec()]
            );
            assert!(!screens.journal_combo.focused);
        }
        assert_eq!(record(&app, JOURNAL_LIST).state & 0x60, 0x60, "open marks");
        // A press off the list closes it; a press on the text's left
        // places the caret before the first byte.
        click(&mut app, (5, 5));
        assert!(app.world().resource::<GameScreens>().journal_list.is_none());
        assert_eq!(
            record(&app, JOURNAL_LIST).state & 0x60,
            0,
            "the close clears them"
        );
        let text_at = (list.x + list.margin + 1, list.y + list.h / 2);
        point(&mut app, (text_at.0 as f32, text_at.1 as f32));
        app.update();
        assert_eq!(
            record(&app, JOURNAL_LIST).state & 0xf_0000,
            0x2_0000,
            "the text part is the edit state"
        );
        button(&mut app, MouseButton::Left, true);
        button(&mut app, MouseButton::Left, false);
        {
            let screens = app.world().resource::<GameScreens>();
            assert!(screens.journal_combo.focused);
            assert_eq!(screens.journal_combo.view.unwrap().caret, 0);
            assert!(
                screens.journal_list.is_none(),
                "the edit press opens nothing"
            );
        }
        type_text(&mut app, window, "X");
        assert_eq!(
            app.world().resource::<GameScreens>().journal.entries[1].title,
            "XCamp"
        );
    }

    #[test]
    fn the_requests_open_and_escape_closes() {
        let mut app = App::new();
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<GameScreens>()
            .add_message::<ScreenRequest>()
            .init_resource::<crate::thaum::live::FrameEffectRoll>()
            .init_resource::<crate::thaum::live::FrameEffectDraws>()
            .add_systems(Update, open_requested_screens);
        let request = |screen| ScreenRequest::screen(screen, true, true);
        app.world_mut()
            .write_message(request(GameScreen::Inventory));
        app.update();
        assert!(
            app.world()
                .resource::<GameScreens>()
                .is_open(GameScreen::Inventory)
        );
        app.world_mut().write_message(request(GameScreen::Journal));
        app.update();
        assert_eq!(
            app.world().resource::<GameScreens>().active_screen(),
            Some(GameScreen::Journal)
        );
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::Escape);
        app.update();
        let screens = app.world().resource::<GameScreens>();
        assert!(!screens.is_open(GameScreen::Journal));
        assert_eq!(screens.active_screen(), Some(GameScreen::Inventory));
    }
}

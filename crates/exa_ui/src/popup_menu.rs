//! The popup menu (`TGUIMenu`): a list of items six units tall in a one-pixel border,
//! separators three units tall, each item with an optional check mark, a
//! shortcut label right-aligned ("Shift+Ctrl+Alt+" and the key's system
//! name) and an optional submenu marked by a right chevron. Hovering an
//! item opens its submenu at once (there is no hover delay): the submenu
//! is placed at the menu's right edge level with the item and flips to
//! the left or upward when it would leave the screen. A press on an
//! enabled item with an action runs the action and closes the whole
//! chain; a radio item becomes the one checked item of its menu, a check
//! item hands its new state to the action.
//!
//! The retail game never opens a menu: only the dormant editor taskbar
//! creates one. These types have no live caller in Exarust and are kept,
//! tested, for completeness. Rules: `docs/UI.md` (Popup menus).

use crate::gui_font::GuiFont;
use crate::gui_text::measure;
use crate::skin::PaletteInk;

/// The style word the constructor gives the menu's form.
pub const STYLE: u32 = 0x10;
/// The attribute word the constructor gives the menu's form.
pub const ATTRIBUTES: u32 = 0x0002_1060;
/// The menu's border in pixels.
pub const BORDER: i32 = 1;
/// An item's height in GUI units.
pub const ROW_UNITS: i32 = 6;

/// Item flags.
pub mod item {
    /// A separator line (no caption, half an item tall).
    pub const SEPARATOR: u32 = 0x1;
    /// Greyed: no highlight fill of its own and a press does nothing.
    pub const DISABLED: u32 = 0x2;
    /// A check item: a press hands the menu the inverse of its checked
    /// state (the item's own flag is left as it is).
    pub const CHECK: u32 = 0x8;
    /// A radio item: a press checks it and unchecks the menu's others.
    pub const RADIO: u32 = 0x10;
    /// Checked; the tick is drawn only for a check item.
    pub const CHECKED: u32 = 0x100;
}

/// Shortcut modifier bits (the shortcut word's low byte).
pub mod modifier {
    pub const SHIFT: u16 = 0x20;
    pub const CTRL: u16 = 0x40;
    pub const ALT: u16 = 0x80;
}

/// The pointer bit a press closes the menu on.
pub const PRESS_CLOSES: u32 = 0x2;
/// The release word a release closes the menu on.
pub const RELEASE_CLOSES: u32 = 0x200;
/// What a press reports to the form dispatcher.
pub const PRESS_REPLY: u32 = 0x40;
/// What a release reports to the form dispatcher.
pub const RELEASE_REPLY: u32 = 0x80;
/// A pointer move that needs a redraw.
pub const REDRAW: u32 = 3;
/// The opener button's state bits a closing menu clears (lit, pressed).
pub const OPENER_BITS: u32 = 0x60;

/// One menu item: caption, flags, shortcut word (modifiers in the low
/// byte, the virtual-key code in the high byte; none when the high byte is
/// zero), the action it runs and the items of the submenu it opens.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuItem {
    pub caption: Vec<u8>,
    pub flags: u32,
    pub shortcut: u16,
    /// The owner's action id; `None` for an item that only opens a
    /// submenu or does nothing.
    pub action: Option<u32>,
    /// The submenu's items; `None` for no submenu.
    pub submenu: Option<Vec<MenuItem>>,
}

impl MenuItem {
    /// A plain item running `action`.
    pub fn new(caption: &[u8], action: u32) -> Self {
        MenuItem {
            caption: caption.to_vec(),
            action: Some(action),
            ..MenuItem::default()
        }
    }

    /// A separator.
    pub fn separator() -> Self {
        MenuItem {
            flags: item::SEPARATOR,
            ..MenuItem::default()
        }
    }

    fn is_separator(&self) -> bool {
        self.flags & item::SEPARATOR == item::SEPARATOR
    }

    fn is_disabled(&self) -> bool {
        self.flags & item::DISABLED == item::DISABLED
    }

    fn has_shortcut(&self) -> bool {
        self.shortcut >> 8 != 0
    }
}

/// The name Windows' key-name call gives the key whose virtual-key code
/// is `vk` after the code is mapped to its scan code without the extended
/// bit, as the original asks for it: so the arrows, Home, End, Page Up,
/// Page Down, Insert and Delete are named as their numeric-pad twins, and
/// Num Lock as Pause. Empty for a key without a name. The names are those
/// of the US layout (from the layout's name table, not observed in the
/// game, which never shows a shortcut).
pub fn system_key_name(vk: u8) -> String {
    let name = match vk {
        0x08 => "Backspace",
        0x09 => "Tab",
        0x0c => "Num 5",
        0x0d => "Enter",
        0x10 | 0xa0 => "Shift",
        0xa1 => "Right Shift",
        0x11 | 0xa2 | 0xa3 => "Ctrl",
        0x12 | 0xa4 | 0xa5 => "Alt",
        0x13 | 0x90 => "Pause",
        0x14 => "Caps Lock",
        0x1b => "Esc",
        0x20 => "Space",
        0x21 | 0x69 => "Num 9",
        0x22 | 0x63 => "Num 3",
        0x23 | 0x61 => "Num 1",
        0x24 | 0x67 => "Num 7",
        0x25 | 0x64 => "Num 4",
        0x26 | 0x68 => "Num 8",
        0x27 | 0x66 => "Num 6",
        0x28 | 0x62 => "Num 2",
        0x2c => "Sys Req",
        0x2d | 0x60 => "Num 0",
        0x2e | 0x6e => "Num Del",
        0x65 => "Num 5",
        0x6a => "Num *",
        0x6b => "Num +",
        0x6d => "Num -",
        0x6f | 0xbf => "/",
        0x91 => "Scroll Lock",
        0xba => ";",
        0xbb => "=",
        0xbc => ",",
        0xbd => "-",
        0xbe => ".",
        0xc0 => "`",
        0xdb => "[",
        0xdc | 0xe2 => "\\",
        0xdd => "]",
        0xde => "'",
        0x30..=0x39 | 0x41..=0x5a => return char::from(vk).to_string(),
        0x70..=0x7b => return format!("F{}", vk - 0x6f),
        _ => "",
    };
    name.to_string()
}

/// A shortcut's label: "Shift+", then "Ctrl+", then "Alt+" for the
/// modifier bits that are set, then the key's system name.
pub fn shortcut_label(shortcut: u16) -> Vec<u8> {
    let mut out = Vec::new();
    if shortcut & modifier::SHIFT != 0 {
        out.extend_from_slice(b"Shift+");
    }
    if shortcut & modifier::CTRL != 0 {
        out.extend_from_slice(b"Ctrl+");
    }
    if shortcut & modifier::ALT != 0 {
        out.extend_from_slice(b"Alt+");
    }
    out.extend_from_slice(system_key_name((shortcut >> 8) as u8).as_bytes());
    out
}

/// Places a popup of size `size` at `at` on a screen of size `screen`:
/// when it would leave the screen's right edge it moves to `flip.0` (or,
/// with a negative `flip.0` or one that leaves the screen too, to two
/// pixels inside the right edge); when it fits, a negative `flip.0` puts
/// it at 0 instead (as the original does). When it would leave the bottom
/// edge its bottom goes to `flip.1`. The original's general popup
/// placement, shared by every popup form.
pub fn place_popup(
    size: (i32, i32),
    screen: (i32, i32),
    flip: (i32, i32),
    at: (i32, i32),
) -> (i32, i32) {
    let (w, h) = size;
    let wide = i64::from(screen.0);
    let x = if wide < i64::from(w) + i64::from(at.0) {
        if flip.0 < 0 || wide < i64::from(w) + i64::from(flip.0) {
            screen.0 - w - 2
        } else {
            flip.0
        }
    } else if flip.0 < 0 {
        0
    } else {
        at.0
    };
    let y = if i64::from(screen.1) < i64::from(h) + i64::from(at.1) {
        flip.1 - h
    } else {
        at.1
    };
    (x, y)
}

/// One open menu.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PopupMenu {
    pub unit: i32,
    pub items: Vec<MenuItem>,
    /// Screen position.
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub border: i32,
    /// The client rectangle's top, right and bottom.
    pub client_top: i32,
    pub client_right: i32,
    pub client_bottom: i32,
    /// The hovered item, 1-based; 0 for none.
    pub hovered: i32,
    /// The value a check item's press leaves for its action: 1 when the
    /// item was unchecked, 0 when it was checked.
    pub value: i32,
    /// The action runs on the opener's owner rather than on the menu.
    pub targeted: bool,
    /// The taskbar button that opened the menu, if any.
    pub opener: Option<u32>,
    /// While set, closing this menu leaves its parent open and a release
    /// does not close it.
    pub keep_parent: bool,
    /// The form's state word; layout clears [`crate::form::state::LAYOUT`].
    pub state: u32,
}

impl PopupMenu {
    /// The constructor's menu: border 1, the style and attribute words
    /// above, and `items`.
    pub fn new(unit: i32, items: Vec<MenuItem>) -> Self {
        PopupMenu {
            unit,
            items,
            border: BORDER,
            state: crate::form::state::LAYOUT,
            ..PopupMenu::default()
        }
    }

    fn row(&self) -> i32 {
        ROW_UNITS * self.unit
    }

    fn separator(&self) -> i32 {
        (ROW_UNITS >> 1) * self.unit
    }

    fn item_height(&self, item: &MenuItem) -> i32 {
        if item.is_separator() {
            self.separator()
        } else {
            self.row()
        }
    }

    /// The layout: height two units plus every item's, width the widest
    /// caption (with its shortcut label and eight units between them)
    /// plus sixteen units and both borders. `font` is the font selected
    /// when the layout runs.
    pub fn layout(&mut self, font: &GuiFont) {
        let u = self.unit;
        let mut width = 0;
        let mut height = u << 1;
        for it in &self.items {
            if it.is_separator() {
                height += self.separator();
            } else {
                height += self.row();
                let mut w = measure(font, &it.caption);
                if it.has_shortcut() {
                    w += measure(font, &shortcut_label(it.shortcut)) + u * 8;
                }
                width = width.max(w);
            }
        }
        self.height = height;
        self.width = self.border * 2 + width + u * 16;
        self.client_right = self.width - self.border;
        self.client_top = self.border;
        self.client_bottom = self.height - self.border;
        self.state &= !crate::form::state::LAYOUT;
    }

    /// The item under a point in the menu's own coordinates (1-based, 0
    /// for none) and the top of the band it was found in. A band runs
    /// from its top to its bottom inclusive, the first top one pixel above
    /// the first unit; the point must be strictly inside the client's left
    /// and right.
    pub fn item_at(&self, x: i32, y: i32) -> (i32, i32) {
        let mut top = self.unit - 1;
        if self.border < x && x < self.client_right {
            for (i, it) in self.items.iter().enumerate() {
                let h = self.item_height(it);
                if top <= y && i64::from(y) <= i64::from(top) + i64::from(h) {
                    return (i as i32 + 1, top);
                }
                top += h;
            }
        }
        (0, top)
    }

    /// Whether a screen point is on the menu (edges included).
    pub fn contains(&self, x: i32, y: i32) -> bool {
        let (lx, ly) = (x - self.x, y - self.y);
        (0..=self.width).contains(&lx) && (0..=self.height).contains(&ly)
    }
}

/// What a menu chain asks of its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuEvent {
    /// Run an item's action, on the opener's owner (`targeted`) or the
    /// menu; `value` is what a check item left.
    Action {
        action: u32,
        targeted: bool,
        value: i32,
    },
    /// A menu of the chain closed (its level when it closed).
    Closed(usize),
    /// The taskbar button that opened a closing menu is to clear its
    /// [`OPENER_BITS`].
    ReleaseOpener(u32),
    /// The pointer now goes to the menu at this level.
    Capture(usize),
}

/// A menu and the submenus open from it, root first: level `k + 1` is
/// the submenu of level `k`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MenuChain {
    pub menus: Vec<PopupMenu>,
    pub screen: (i32, i32),
    /// The menu the pointer goes to.
    pub captured: usize,
}

impl MenuChain {
    /// Opens `menu` under a taskbar button whose screen rectangle is
    /// (x, y, h): one pixel right of and below its corner, flipped above
    /// its bottom when it does not fit below. The menu's actions run on
    /// the button's owner. Returns the chain.
    pub fn open_under(
        mut menu: PopupMenu,
        font: &GuiFont,
        button: (i32, i32, i32),
        opener: u32,
        screen: (i32, i32),
    ) -> Self {
        let (bx, by, bh) = button;
        menu.layout(font);
        menu.opener = Some(opener);
        menu.targeted = true;
        let (x, y) = place_popup(
            (menu.width, menu.height),
            screen,
            (bx + 1, bh + by - 1),
            (bx + 1, by + 1),
        );
        menu.x = x;
        menu.y = y;
        MenuChain {
            menus: vec![menu],
            screen,
            captured: 0,
        }
    }

    /// The modal variant: placed the same way, the opener recorded but
    /// the actions run on the menu itself, the menu focused and run
    /// modally by the form base (not modelled here).
    pub fn open_modal_under(
        menu: PopupMenu,
        font: &GuiFont,
        button: (i32, i32, i32),
        opener: u32,
        screen: (i32, i32),
    ) -> Self {
        let mut chain = Self::open_under(menu, font, button, opener, screen);
        chain.menus[0].targeted = false;
        chain
    }

    /// Closes the menu at `level`: the submenus open from it close first
    /// (deepest first), then, unless the menu keeps its parent, the parent
    /// chain (root first), then the menu itself. Each closing menu
    /// releases its opener button.
    pub fn close(&mut self, level: usize) -> Vec<MenuEvent> {
        let mut events = Vec::new();
        if level >= self.menus.len() {
            return events;
        }
        let opener = self.menus[level].opener;
        if let Some(button) = opener {
            events.push(MenuEvent::ReleaseOpener(button));
        }
        for child in (level + 1..self.menus.len()).rev() {
            if let Some(button) = self.menus[child].opener {
                events.push(MenuEvent::ReleaseOpener(button));
            }
            events.push(MenuEvent::Closed(child));
        }
        let keep = self.menus[level].keep_parent;
        self.menus.truncate(level + 1);
        if level > 0 && !keep {
            for parent in 0..level {
                if let Some(button) = self.menus[parent].opener {
                    events.push(MenuEvent::ReleaseOpener(button));
                }
                events.push(MenuEvent::Closed(parent));
            }
            events.push(MenuEvent::Closed(level));
            self.menus.clear();
        } else {
            events.push(MenuEvent::Closed(level));
            self.menus.truncate(level);
        }
        if self.captured >= self.menus.len() {
            self.captured = self.menus.len().saturating_sub(1);
        }
        events
    }

    /// Hands the pointer to the menu at `target` when the screen point is
    /// on it; the menu at `from` keeps its parent while it lets go.
    fn hand_over(&mut self, from: usize, target: usize, x: i32, y: i32) -> Option<MenuEvent> {
        if self.menus.get(target)?.contains(x, y) {
            self.menus[from].keep_parent = true;
            self.captured = target;
            self.menus[from].keep_parent = false;
            Some(MenuEvent::Capture(target))
        } else {
            None
        }
    }

    /// A pointer move over the captured menu, at a screen point. On a new
    /// item the open submenu closes (leaving this menu open) and the
    /// item's submenu, if it has one and is enabled, opens at once, laid
    /// out in `font`. Off the items the hover clears and the submenu
    /// closes, unless the point is on the submenu, which then takes the
    /// pointer; otherwise the parent takes it when the point is on it.
    /// Returns the reply ([`REDRAW`] or 0) and the events.
    pub fn pointer_move(&mut self, font: &GuiFont, x: i32, y: i32) -> (u32, Vec<MenuEvent>) {
        let level = self.captured;
        let mut events = Vec::new();
        let Some(menu) = self.menus.get(level) else {
            return (0, events);
        };
        let (hit, top) = menu.item_at(x - menu.x, y - menu.y);
        if hit == 0 {
            let reply = if menu.hovered != 0 { REDRAW } else { 0 };
            let child = level + 1;
            if child < self.menus.len()
                && let Some(e) = self.hand_over(level, child, x, y)
            {
                events.push(e);
                return (reply, events);
            }
            self.menus[level].hovered = 0;
            if child < self.menus.len() {
                self.menus[child].keep_parent = true;
                events.extend(self.close(child));
            }
            if level > 0
                && let Some(e) = self.hand_over(level, level - 1, x, y)
            {
                events.push(e);
            }
            return (reply, events);
        }
        if hit == menu.hovered {
            return (0, events);
        }
        self.menus[level].hovered = hit;
        if level + 1 < self.menus.len() {
            self.menus[level + 1].keep_parent = true;
            events.extend(self.close(level + 1));
        }
        let menu = &self.menus[level];
        let it = &menu.items[hit as usize - 1];
        if let (Some(items), false) = (&it.submenu, it.is_disabled()) {
            let u = menu.unit;
            let mut child = PopupMenu::new(u, items.clone());
            child.layout(font);
            let flip = (
                menu.x + menu.border,
                u * 2 + menu.row() * hit + menu.y + menu.border,
            );
            let at = (menu.x + menu.width - u, menu.y + top + menu.border - u);
            let (cx, cy) = place_popup((child.width, child.height), self.screen, flip, at);
            child.x = cx;
            child.y = cy;
            self.menus.push(child);
        }
        (REDRAW, events)
    }

    /// A press on the captured menu. The [`PRESS_CLOSES`] bit, or no
    /// hovered item, closes the chain. An enabled hovered item with an
    /// action: a radio item becomes its menu's only checked radio item, a
    /// check item leaves the inverse of its checked state, the action
    /// runs and the chain closes. Anything else does nothing.
    pub fn press(&mut self, buttons: u32) -> (u32, Vec<MenuEvent>) {
        let level = self.captured;
        let mut events = Vec::new();
        let Some(menu) = self.menus.get_mut(level) else {
            return (PRESS_REPLY, events);
        };
        if buttons & PRESS_CLOSES != 0 || menu.hovered < 1 {
            events.extend(self.close(level));
            return (PRESS_REPLY, events);
        }
        let index = menu.hovered as usize - 1;
        let it = menu.items[index].clone();
        let Some(action) = it.action else {
            return (PRESS_REPLY, events);
        };
        if it.is_disabled() {
            return (PRESS_REPLY, events);
        }
        if it.flags & item::RADIO != 0 {
            for other in menu.items.iter_mut() {
                if other.flags & item::RADIO != 0 {
                    other.flags &= !item::CHECKED;
                }
            }
            menu.items[index].flags |= item::CHECKED;
        } else if it.flags & item::CHECK != 0 {
            menu.value = i32::from(it.flags & item::CHECKED == 0);
        }
        events.push(MenuEvent::Action {
            action,
            targeted: menu.targeted,
            value: menu.value,
        });
        events.extend(self.close(level));
        (PRESS_REPLY, events)
    }

    /// A release on the captured menu: the [`RELEASE_CLOSES`] word closes
    /// the chain unless the menu is keeping its parent.
    pub fn release(&mut self, word: u32) -> (u32, Vec<MenuEvent>) {
        let level = self.captured;
        let keep = self.menus.get(level).is_none_or(|m| m.keep_parent);
        if word == RELEASE_CLOSES && !keep {
            return (RELEASE_REPLY, self.close(level));
        }
        (RELEASE_REPLY, Vec::new())
    }
}

/// How a menu glyph is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GlyphInk {
    /// Plain, tinted with the palette ink.
    Tinted(PaletteInk),
    /// The atlas's outlined draw at full strength (the hovered item).
    Outlined,
}

/// Which glyph of the small atlas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuGlyphKind {
    /// The right chevron of an item with a submenu.
    Submenu,
    /// The tick of a checked check item.
    Tick,
}

/// One glyph draw.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuGlyph {
    pub kind: MenuGlyphKind,
    pub at: (f32, f32),
    pub ink: GlyphInk,
}

/// How a text run is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInk {
    Tinted(PaletteInk),
    /// The skin's hot-text draw (the hovered item).
    Hot,
}

/// One text run: its bytes, pen (left, bottom) and ink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuText {
    pub text: Vec<u8>,
    pub pen: (i32, i32),
    pub ink: TextInk,
}

/// The hovered item's highlight: a vertical gradient fill (corners
/// x0, y0, x1, y1 and its top and bottom inks), a frame inside it (with
/// its two inks and strength 0.5) and, for an enabled item, a shade box
/// (x, y, w, h).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuHighlight {
    pub fill: [i32; 4],
    pub fill_inks: (PaletteInk, PaletteInk),
    pub frame: [i32; 4],
    pub frame_inks: (PaletteInk, PaletteInk),
    pub shade: Option<[i32; 4]>,
}

/// Where a menu's parts are drawn, in its own coordinates.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MenuDraw {
    pub highlight: Option<MenuHighlight>,
    /// Separator lines: a bevelled band (x0, y0, x1, y1) in the form inks
    /// 1, 2, 4 and 5.
    pub separators: Vec<[i32; 4]>,
    pub glyphs: Vec<MenuGlyph>,
    pub texts: Vec<MenuText>,
}

impl PopupMenu {
    /// The menu's draw in three passes, as the original's: the hovered
    /// item's highlight and the separators, then the chevrons and ticks,
    /// then the captions and shortcut labels in `font` (the main font). A
    /// disabled caption is drawn twice, embossed: form ink 6, then form
    /// ink 0 one pixel up and left.
    pub fn draw(&self, font: &GuiFont) -> MenuDraw {
        let u = self.unit;
        let row = self.row();
        let sep = self.separator();
        let right = self.client_right;
        let mut out = MenuDraw::default();
        let mut y = u;
        for (i, it) in self.items.iter().enumerate() {
            if it.is_separator() {
                let mid = (sep >> 1) + y;
                out.separators
                    .push([u - 1, mid - 2, right + u - (u << 1), mid]);
                y += sep;
                continue;
            }
            if self.hovered - 1 == i as i32 {
                let fill = [u - 2, y - 2, right + u - (u << 1) + 1, y + row];
                let frame = [u, y, right + u - (u << 1) - 1, row + y - 2];
                out.highlight = Some(if it.is_disabled() {
                    MenuHighlight {
                        fill,
                        fill_inks: (PaletteInk::FormHot(5), PaletteInk::FormHot(0)),
                        frame,
                        frame_inks: (PaletteInk::Form(4), PaletteInk::Form(1)),
                        shade: None,
                    }
                } else {
                    MenuHighlight {
                        fill,
                        fill_inks: (PaletteInk::FormHot(6), PaletteInk::FormHot(0)),
                        frame,
                        frame_inks: (PaletteInk::Form(4), PaletteInk::Form(0)),
                        shade: Some([u, y, right - (u << 1) - 1, row - 2]),
                    }
                });
            }
            y += row;
        }
        let gx = self.width - u * 4 - 4;
        let mut gy = (u >> 1) + u * 2 - 1;
        for (i, it) in self.items.iter().enumerate() {
            if it.is_separator() {
                gy += sep;
                continue;
            }
            let hot = self.hovered - 1 == i as i32;
            if it.submenu.is_some() {
                if it.is_disabled() {
                    out.glyphs.push(MenuGlyph {
                        kind: MenuGlyphKind::Submenu,
                        at: (gx as f32, gy as f32),
                        ink: GlyphInk::Tinted(PaletteInk::Text(3)),
                    });
                    out.glyphs.push(MenuGlyph {
                        kind: MenuGlyphKind::Submenu,
                        at: (gx as f32 - 1.0, gy as f32 - 0.5),
                        ink: GlyphInk::Tinted(PaletteInk::Text(0)),
                    });
                } else {
                    out.glyphs.push(MenuGlyph {
                        kind: MenuGlyphKind::Submenu,
                        at: (gx as f32, gy as f32),
                        ink: if hot {
                            GlyphInk::Outlined
                        } else {
                            GlyphInk::Tinted(PaletteInk::Text(6))
                        },
                    });
                }
            }
            let ticked = item::CHECK | item::CHECKED;
            if it.flags & ticked == ticked && !it.is_disabled() {
                out.glyphs.push(MenuGlyph {
                    kind: MenuGlyphKind::Tick,
                    at: (u as f32, gy as f32),
                    ink: if hot {
                        GlyphInk::Outlined
                    } else {
                        GlyphInk::Tinted(PaletteInk::Text(6))
                    },
                });
            }
            gy += row;
        }
        let tx = u * 5;
        let mut ty = ((i64::from(font.ascent) + i64::from(row) + 1) as u64 >> 1) as i32 + u;
        for (i, it) in self.items.iter().enumerate() {
            if it.is_separator() {
                ty += sep;
                continue;
            }
            let label = it.has_shortcut().then(|| shortcut_label(it.shortcut));
            let lx = label
                .as_ref()
                .map(|l| self.client_right - measure(font, l) - u * 6);
            let mut runs: Vec<(Vec<u8>, i32)> = vec![(it.caption.clone(), tx)];
            if let (Some(l), Some(x)) = (label, lx) {
                runs.push((l, x));
            }
            for (text, x) in runs {
                if it.is_disabled() {
                    out.texts.push(MenuText {
                        text: text.clone(),
                        pen: (x, ty),
                        ink: TextInk::Tinted(PaletteInk::Form(6)),
                    });
                    out.texts.push(MenuText {
                        text,
                        pen: (x - 1, ty - 1),
                        ink: TextInk::Tinted(PaletteInk::Form(0)),
                    });
                } else if self.hovered - 1 == i as i32 {
                    out.texts.push(MenuText {
                        text,
                        pen: (x, ty),
                        ink: TextInk::Hot,
                    });
                } else {
                    out.texts.push(MenuText {
                        text,
                        pen: (x, ty),
                        ink: TextInk::Tinted(PaletteInk::Text(6)),
                    });
                }
            }
            ty += row;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_08;

    fn items() -> Vec<MenuItem> {
        vec![
            MenuItem {
                shortcut: 0x4f00 | modifier::CTRL,
                ..MenuItem::new(b"Open", 1)
            },
            MenuItem::separator(),
            MenuItem {
                submenu: Some(vec![MenuItem::new(b"A", 7), MenuItem::new(b"B", 8)]),
                ..MenuItem::new(b"Recent", 2)
            },
            MenuItem {
                flags: item::DISABLED,
                ..MenuItem::new(b"Save", 3)
            },
        ]
    }

    fn laid_out() -> PopupMenu {
        let mut m = PopupMenu::new(4, items());
        m.layout(&RAYFORM_08);
        m
    }

    #[test]
    fn shortcut_labels_name_modifiers_then_the_key() {
        assert_eq!(shortcut_label(0x4f40), b"Ctrl+O");
        assert_eq!(shortcut_label(0x7020 | 0x40 | 0x80), b"Shift+Ctrl+Alt+F1");
        assert_eq!(shortcut_label(0x2540), b"Ctrl+Num 4");
        assert_eq!(shortcut_label(0x2e00), b"Num Del");
        assert_eq!(shortcut_label(0x0780), b"Alt+");
    }

    #[test]
    fn the_layout_sums_rows_and_takes_the_widest_item() {
        let m = laid_out();
        // 2u + 3 rows of 6u + a separator of 3u.
        assert_eq!(m.height, 8 + 72 + 12);
        // "Open" 34 + "Ctrl+O" 42 + 8u = 108 is the widest.
        assert_eq!(measure(&RAYFORM_08, b"Open"), 34);
        assert_eq!(measure(&RAYFORM_08, b"Ctrl+O"), 42);
        assert_eq!(m.width, 2 + 108 + 64);
        assert_eq!(
            (m.client_top, m.client_right, m.client_bottom),
            (1, 173, 91)
        );
        assert_eq!(m.state & crate::form::state::LAYOUT, 0);
    }

    #[test]
    fn bands_include_both_edges_from_one_pixel_above_the_first_unit() {
        let m = laid_out();
        assert_eq!(m.item_at(10, 3), (1, 3));
        assert_eq!(m.item_at(10, 27), (1, 3));
        assert_eq!(m.item_at(10, 28), (2, 27));
        assert_eq!(m.item_at(10, 39), (2, 27));
        assert_eq!(m.item_at(10, 40), (3, 39));
        assert_eq!(m.item_at(1, 10).0, 0);
        assert_eq!(m.item_at(173, 10).0, 0);
        assert_eq!(m.item_at(10, 2).0, 0);
    }

    #[test]
    fn popups_flip_left_and_up_at_the_screen_edges() {
        assert_eq!(
            place_popup((100, 50), (800, 600), (5, 5), (10, 20)),
            (10, 20)
        );
        assert_eq!(
            place_popup((100, 50), (800, 600), (650, 5), (750, 20)),
            (650, 20)
        );
        assert_eq!(
            place_popup((100, 50), (800, 600), (750, 5), (750, 20)),
            (698, 20)
        );
        assert_eq!(
            place_popup((100, 50), (800, 600), (-1, 5), (10, 20)),
            (0, 20)
        );
        assert_eq!(
            place_popup((100, 50), (800, 600), (5, 590), (10, 560)),
            (10, 540)
        );
    }

    #[test]
    fn a_menu_opens_under_its_button() {
        let chain = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 30, 20),
            9,
            (800, 600),
        );
        assert_eq!((chain.menus[0].x, chain.menus[0].y), (101, 31));
        assert!(chain.menus[0].targeted);
        let low = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 560, 20),
            9,
            (800, 600),
        );
        assert_eq!(low.menus[0].y, 579 - 92);
    }

    #[test]
    fn hovering_an_item_opens_its_submenu_at_once() {
        let mut chain = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 30, 20),
            9,
            (800, 600),
        );
        let (reply, events) = chain.pointer_move(&RAYFORM_08, 101 + 10, 31 + 45);
        assert_eq!((reply, events), (REDRAW, vec![]));
        assert_eq!(chain.menus[0].hovered, 3);
        assert_eq!(chain.menus.len(), 2);
        let sub = &chain.menus[1];
        // At the menu's right edge less a unit, level with the band's top.
        assert_eq!((sub.x, sub.y), (101 + 174 - 4, 31 + 39 + 1 - 4));
        // The same item again changes nothing.
        assert_eq!(chain.pointer_move(&RAYFORM_08, 111, 77).0, 0);
        // Another item closes the submenu but not the menu.
        let (_, events) = chain.pointer_move(&RAYFORM_08, 111, 31 + 10);
        assert_eq!(events, vec![MenuEvent::Closed(1)]);
        assert_eq!(chain.menus.len(), 1);
        assert_eq!(chain.menus[0].hovered, 1);
    }

    #[test]
    fn moving_onto_the_submenu_hands_it_the_pointer() {
        let mut chain = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 30, 20),
            9,
            (800, 600),
        );
        chain.pointer_move(&RAYFORM_08, 111, 76);
        let (sx, sy) = (chain.menus[1].x, chain.menus[1].y);
        let (reply, events) = chain.pointer_move(&RAYFORM_08, sx + 10, sy + 10);
        assert_eq!(reply, REDRAW);
        assert_eq!(events, vec![MenuEvent::Capture(1)]);
        assert_eq!(chain.captured, 1);
        assert_eq!(chain.menus[0].hovered, 3);
    }

    #[test]
    fn a_press_runs_the_action_and_closes_the_chain() {
        let mut chain = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 30, 20),
            9,
            (800, 600),
        );
        chain.pointer_move(&RAYFORM_08, 111, 41);
        let (reply, events) = chain.press(0x2001);
        assert_eq!(reply, PRESS_REPLY);
        assert_eq!(
            events,
            vec![
                MenuEvent::Action {
                    action: 1,
                    targeted: true,
                    value: 0
                },
                MenuEvent::ReleaseOpener(9),
                MenuEvent::Closed(0),
            ]
        );
        assert!(chain.menus.is_empty());
    }

    #[test]
    fn disabled_items_ignore_presses_and_radio_items_check_alone() {
        let mut m = PopupMenu::new(4, items());
        m.items[0].flags = item::RADIO | item::CHECKED;
        m.items[2].flags = item::RADIO;
        m.items[2].submenu = None;
        let mut chain = MenuChain::open_under(m, &RAYFORM_08, (100, 30, 20), 9, (800, 600));
        chain.menus[0].hovered = 4;
        assert_eq!(chain.press(1).1, vec![]);
        chain.menus[0].hovered = 3;
        let events = chain.press(1).1;
        assert_eq!(
            events[0],
            MenuEvent::Action {
                action: 2,
                targeted: true,
                value: 0
            }
        );
    }

    #[test]
    fn a_check_item_hands_over_its_inverse() {
        let mut m = PopupMenu::new(
            4,
            vec![MenuItem {
                flags: item::CHECK,
                ..MenuItem::new(b"Grid", 5)
            }],
        );
        m.layout(&RAYFORM_08);
        let mut chain = MenuChain {
            menus: vec![m],
            screen: (800, 600),
            captured: 0,
        };
        chain.menus[0].hovered = 1;
        assert_eq!(
            chain.press(1).1[0],
            MenuEvent::Action {
                action: 5,
                targeted: false,
                value: 1
            }
        );
    }

    #[test]
    fn leaving_a_submenu_for_nothing_closes_it_and_keeps_the_parent() {
        let mut chain = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 30, 20),
            9,
            (800, 600),
        );
        chain.pointer_move(&RAYFORM_08, 111, 76);
        let (reply, events) = chain.pointer_move(&RAYFORM_08, 700, 500);
        assert_eq!(reply, REDRAW);
        assert_eq!(events, vec![MenuEvent::Closed(1)]);
        assert_eq!(chain.menus.len(), 1);
        assert_eq!(chain.menus[0].hovered, 0);
    }

    #[test]
    fn a_release_closes_unless_the_parent_is_kept() {
        let mut chain = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 30, 20),
            9,
            (800, 600),
        );
        assert_eq!(chain.release(0x100), (RELEASE_REPLY, vec![]));
        chain.menus[0].keep_parent = true;
        assert_eq!(chain.release(RELEASE_CLOSES).1, vec![]);
        chain.menus[0].keep_parent = false;
        assert_eq!(
            chain.release(RELEASE_CLOSES).1,
            vec![MenuEvent::ReleaseOpener(9), MenuEvent::Closed(0)]
        );
    }

    #[test]
    fn closing_a_submenu_closes_the_whole_chain() {
        let mut chain = MenuChain::open_under(
            PopupMenu::new(4, items()),
            &RAYFORM_08,
            (100, 30, 20),
            9,
            (800, 600),
        );
        chain.pointer_move(&RAYFORM_08, 111, 76);
        assert_eq!(
            chain.close(1),
            vec![
                MenuEvent::ReleaseOpener(9),
                MenuEvent::Closed(0),
                MenuEvent::Closed(1)
            ]
        );
        assert!(chain.menus.is_empty());
    }

    #[test]
    fn the_draw_places_highlight_glyphs_and_text() {
        let mut m = laid_out();
        m.hovered = 1;
        let d = m.draw(&RAYFORM_08);
        let h = d.highlight.unwrap();
        assert_eq!(h.fill, [2, 2, 170, 28]);
        assert_eq!(h.frame, [4, 4, 168, 26]);
        assert_eq!(h.shade, Some([4, 4, 164, 22]));
        assert_eq!(
            h.fill_inks,
            (PaletteInk::FormHot(6), PaletteInk::FormHot(0))
        );
        // The separator band sits mid-way in its 12 pixels.
        assert_eq!(d.separators, vec![[3, 32, 169, 34]]);
        // The chevron of "Recent" at the right, 8 units in.
        assert_eq!(
            d.glyphs,
            vec![MenuGlyph {
                kind: MenuGlyphKind::Submenu,
                at: (154.0, 45.0),
                ink: GlyphInk::Tinted(PaletteInk::Text(6)),
            }]
        );
        // Captions at 5 units; the pen's bottom centres the ascent in the row.
        assert_eq!(
            d.texts[0],
            MenuText {
                text: b"Open".to_vec(),
                pen: (20, 23),
                ink: TextInk::Hot
            }
        );
        assert_eq!(d.texts[1].pen, (173 - 42 - 24, 23));
        assert_eq!(d.texts[2].pen, (20, 59));
        assert_eq!(d.texts[3].ink, TextInk::Tinted(PaletteInk::Form(6)));
        assert_eq!(d.texts[4].pen, (19, 82));
    }

    #[test]
    fn a_disabled_hovered_item_has_no_shade_and_a_tick_needs_check_and_checked() {
        let mut m = laid_out();
        m.items[0].flags = item::CHECK | item::CHECKED;
        m.hovered = 4;
        let d = m.draw(&RAYFORM_08);
        let h = d.highlight.unwrap();
        assert_eq!(h.shade, None);
        assert_eq!(h.fill_inks.0, PaletteInk::FormHot(5));
        assert_eq!(h.fill[1], 62);
        assert!(d.glyphs.contains(&MenuGlyph {
            kind: MenuGlyphKind::Tick,
            at: (4.0, 9.0),
            ink: GlyphInk::Tinted(PaletteInk::Text(6)),
        }));
    }
}

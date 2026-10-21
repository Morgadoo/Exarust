//! The extended GUI controls (`RF_GUI_Controls_Ex`): the text field, combo
//! box, colour swatch, spinner, progress bar, separator, image and texture
//! views, caption, trackbar and scroll bar, their default sizes, spans,
//! values, presses, keys and draw geometry, plus the menu/drop-down row
//! metrics, list extents and radio/check activation. The unit-derived
//! `3U|1` quirk is kept.

/// The default trackbar knob width (20).
pub const DEFAULT_SPAN: i32 = 20;
/// The default trackbar pad (2).
pub const DEFAULT_PAD: i32 = 2;
/// The list row height in units (6).
pub const ROW_UNITS: i32 = 6;
/// The list's visible row cap (16).
pub const VISIBLE_ROWS: i32 = 16;

/// The three usable-text spans. `span_c` uses `3U|1`, not `3U+1`.
pub fn text_span_a(w: i32, pad: i32, unit: i32) -> i32 {
    w - 2 * pad - 2 * unit
}

pub fn text_span_b(w: i32, pad: i32, unit: i32) -> i32 {
    w - ((4 * unit) | 1) - 2 * pad - unit
}

pub fn text_span_c(w: i32, pad: i32, unit: i32) -> i32 {
    w - ((3 * unit) | 1) - 2 * pad - 2 * unit
}

/// A trackbar's geometry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Track {
    pub unit: i32,
    pub pad: i32,
    pub span: i32,
}

impl Track {
    /// The knob travel: `w − 2·pad − span`.
    pub fn travel(self, w: i32) -> i32 {
        w - 2 * self.pad - self.span
    }

    /// The value at a cursor x: `(cursor − x − pad − span/2) / travel`,
    /// clamped to 0..1 and quantised to `steps` when positive. The clamp is
    /// the original's ordered `0 ≤ f` / `1 < f` pair, so a NaN falls to 0.
    pub fn value_at(self, x: i32, w: i32, cursor: i32, steps: i32) -> f32 {
        let num = cursor - x - self.pad - self.span / 2;
        let den = self.travel(w);
        let mut f = (num as f64 / den as f64) as f32;
        if 0.0 <= f {
            if 1.0 < f {
                f = 1.0;
            }
        } else {
            f = 0.0;
        }
        if steps > 0 {
            let q = (steps as f32 * f).round_ties_even() as i32;
            f = (q as f64 / steps as f64) as f32;
        }
        f
    }

    /// The thumb's left edge: `x + pad + rint(travel·value)`.
    pub fn thumb_left(self, x: i32, w: i32, value: f32) -> i32 {
        x + self.pad + (self.travel(w) as f32 * value).round_ties_even() as i32
    }

    /// The value caption's centre within the knob's slot.
    #[allow(clippy::too_many_arguments)]
    pub fn caption(
        self,
        x: i32,
        y: i32,
        h: i32,
        value: f32,
        w: i32,
        text_w: i32,
        line_h: i32,
    ) -> (i32, i32) {
        let tx = x + self.pad + (self.travel(w) as f32 * value).round_ties_even() as i32;
        let cx = tx + (self.span - 1 - text_w) / 2;
        let cy = y + (h + line_h + 2) / 2;
        (cx, cy)
    }
}

/// A menu/drop-down list's metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListMetrics {
    pub unit: i32,
    pub inset: i32,
}

impl ListMetrics {
    pub fn row(self) -> i32 {
        ROW_UNITS * self.unit
    }

    pub fn separator(self) -> i32 {
        (ROW_UNITS / 2) * self.unit
    }
}

/// The 1-based row at client `y`, clamped to `1..=count`.
pub fn item_at(metrics: ListMetrics, y: i32, top: i32, count: i32) -> i32 {
    if count <= 0 {
        return 0;
    }
    let pitch = metrics.unit * ROW_UNITS;
    let mut index = 1 + ((y as i64 - top as i64) / pitch as i64) as i32;
    if index > count {
        index = count;
    } else if index < 1 {
        index = 1;
    }
    index
}

/// The next selection: ±1, clamped to `1..=count` (never wraps).
pub fn step_selection(current: i32, direction: i32, count: i32) -> i32 {
    (current + direction).clamp(1, count.max(1))
}

/// A list's extent, content height and optional page fraction.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListExtent {
    pub height: i32,
    pub content: i32,
    pub page: Option<f32>,
}

/// A list's extent: up to 16 rows fit; beyond that the height is
/// `2·inset + 16·row` and the page fraction is `1/(count−16)`.
pub fn list_extent(metrics: ListMetrics, count: i32) -> ListExtent {
    let row = metrics.row();
    if count > VISIBLE_ROWS {
        ListExtent {
            height: 2 * metrics.inset + row * VISIBLE_ROWS,
            content: row * count,
            page: Some(1.0 / (count - VISIBLE_ROWS) as f32),
        }
    } else {
        ListExtent {
            height: row * count + 2 * metrics.inset,
            content: row * count,
            page: None,
        }
    }
}

/// One item's measurement input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemMeasure {
    pub text_width: i32,
    pub shortcut_width: i32,
    pub has_shortcut: bool,
    pub separator: bool,
}

/// The content size: separator rows are half height; shortcuts inset by
/// `8·unit`; the width adds `2·inset + 16·unit`.
pub fn item_extent(metrics: ListMetrics, items: &[ItemMeasure]) -> (i32, i32) {
    let mut height = 2 * metrics.unit;
    let mut widest = 0;
    for item in items {
        if item.separator {
            height += metrics.separator();
            continue;
        }
        height += metrics.row();
        let mut width = item.text_width;
        if item.has_shortcut {
            width += item.shortcut_width + 8 * metrics.unit;
        }
        widest = widest.max(width);
    }
    (2 * metrics.inset + widest + 16 * metrics.unit, height)
}

/// A menu item's flags: 1 separator, 2 disabled, 8 checkable, 0x10 radio
/// group, 0x100 checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuItem {
    pub flags: u32,
}

/// The activation's result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Activation {
    None,
    Radio(usize),
    Check(u8),
}

/// Applies radio/check activation to the items.
pub fn activate(items: &mut [MenuItem], index: usize) -> Activation {
    let Some(item) = items.get(index) else {
        return Activation::None;
    };
    if item.flags & 0x10 != 0 {
        for other in items.iter_mut() {
            if other.flags & 0x10 != 0 {
                other.flags &= !0x100;
            }
        }
        items[index].flags |= 0x100;
        return Activation::Radio(index);
    }
    if item.flags & 0x8 != 0 {
        let next = if item.flags & 0x100 != 0 { 0 } else { 1 };
        return Activation::Check(next);
    }
    Activation::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_text_spans() {
        assert_eq!(text_span_a(100, 2, 4), 88);
        assert_eq!(text_span_b(100, 2, 4), 75);
        assert_eq!(text_span_c(100, 2, 4), 75);
        assert_eq!(text_span_c(100, 2, 5), 71);
        assert_eq!(text_span_c(100, 2, 6), 65);
    }

    #[test]
    fn the_trackbar_value_and_thumb() {
        let t = Track {
            unit: 4,
            pad: 2,
            span: 20,
        };
        assert_eq!(t.travel(100), 76);
        assert_eq!(t.value_at(0, 100, 50, 0), 0.5);
        assert!((t.value_at(0, 100, 61, 0) - 49.0 / 76.0).abs() < 1e-6);
        assert_eq!(t.value_at(0, 100, 0, 0), 0.0);
        assert_eq!(t.value_at(0, 100, 100, 0), 1.0);
        // A zero travel makes 0/0; the ordered clamp sends NaN to 0.
        assert_eq!(t.value_at(0, 24, 12, 0), 0.0);
        assert_eq!(t.value_at(0, 100, 61, 4), 0.75);
        assert_eq!(t.thumb_left(0, 100, 0.5), 40);
        assert_eq!(t.value_at(0, 100, 40 + 10, 0), 0.5);
        assert_eq!(t.caption(0, 10, 20, 0.5, 100, 10, 16), (44, 29));
    }

    #[test]
    fn the_list_rows_and_navigation() {
        let m = ListMetrics { unit: 4, inset: 3 };
        assert_eq!(m.row(), 24);
        assert_eq!(m.separator(), 12);
        assert_eq!(item_at(m, 30, 3, 10), 2);
        assert_eq!(item_at(m, 26, 3, 10), 1);
        assert_eq!(item_at(m, 10_000, 3, 10), 10);
        assert_eq!(step_selection(1, -1, 10), 1);
        assert_eq!(step_selection(10, 1, 10), 10);
        assert_eq!(step_selection(5, 1, 10), 6);
    }

    #[test]
    fn the_list_extent_and_item_size() {
        let m = ListMetrics { unit: 4, inset: 3 };
        let e = list_extent(m, 20);
        assert_eq!(e.height, 390);
        assert_eq!(e.content, 480);
        assert_eq!(e.page, Some(0.25));
        assert_eq!(list_extent(m, 16).page, None);
        assert_eq!(list_extent(m, 10).height, 246);

        let items = [
            ItemMeasure {
                text_width: 40,
                shortcut_width: 0,
                has_shortcut: false,
                separator: false,
            },
            ItemMeasure {
                text_width: 0,
                shortcut_width: 0,
                has_shortcut: false,
                separator: true,
            },
            ItemMeasure {
                text_width: 10,
                shortcut_width: 30,
                has_shortcut: true,
                separator: false,
            },
        ];
        assert_eq!(item_extent(m, &items), (142, 68));
    }

    #[test]
    fn the_radio_and_check_activation() {
        let mut items = [
            MenuItem { flags: 0x10 },
            MenuItem {
                flags: 0x10 | 0x100,
            },
            MenuItem { flags: 0 },
        ];
        assert_eq!(activate(&mut items, 0), Activation::Radio(0));
        assert_eq!(items[0].flags & 0x100, 0x100);
        assert_eq!(items[1].flags & 0x100, 0);
        let mut item = [MenuItem { flags: 0x8 | 0x100 }];
        assert_eq!(activate(&mut item, 0), Activation::Check(0));
        let mut item = [MenuItem { flags: 0x8 }];
        assert_eq!(activate(&mut item, 0), Activation::Check(1));
        assert_eq!(activate(&mut items, 9), Activation::None);
    }
}

/// The control label's aligned position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelAt {
    pub x: i32,
    pub y: i32,
}

/// Aligns a control label. `avail` is the usable text
/// width, `base` the font base and `line_h` its line height.
#[allow(clippy::too_many_arguments)]
pub fn align_label(
    x: i32,
    y: i32,
    h: i32,
    pad: i32,
    unit: i32,
    text_w: i32,
    avail: i32,
    line_h: i32,
    base: i32,
    style: u32,
) -> LabelAt {
    // The original switches on 1, 3, 4 and 5; a 2 falls through to the
    // default, so only those four alignments exist.
    let lx = match style & 0x7000 {
        0x1000 => x - text_w - unit,
        0x3000 => (avail - 1 - text_w) / 2 + pad + x + unit,
        0x4000 => x + avail - text_w + unit,
        0x5000 => avail + x + unit,
        _ => x + pad + (unit + 1) / 2,
    };
    let ly = match style & 0x700 {
        0x100 => y - unit / 2,
        0x300 => (line_h + h + 2) / 2 + y,
        0x400 => y + h + base - 1,
        0x500 => y + h,
        _ => y + (unit + 1) / 2,
    };
    LabelAt { x: lx, y: ly }
}

#[cfg(test)]
mod label_tests {
    use super::*;

    #[test]
    fn the_horizontal_styles() {
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x1000).x,
            76
        );
        // 0x2000 is not a case: the original falls through to the default.
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x2000).x,
            104
        );
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x3000).x,
            145
        );
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x4000).x,
            184
        );
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x5000).x,
            204
        );
        assert_eq!(align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0).x, 104);
    }

    #[test]
    fn the_vertical_styles() {
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x100).y, 98);
        // 0x200 is not a case either.
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x200).y, 102);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x300).y, 119);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x400).y, 133);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x500).y, 120);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0).y, 102);
    }
}

/// The unit-derived text inset: `3 × unit | 1`, the value the original
/// caches at start-up and adds to a control's border.
pub fn text_inset(unit: i32) -> i32 {
    (3 * unit) | 1
}

/// The extended control classes, in the order the unit registers them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtendedClass {
    pub id: u16,
    pub name: &'static str,
    /// The record's default size (w, h).
    pub size: (i32, i32),
}

/// The text field class id.
pub const TEXT_FIELD: u16 = 17;
/// The combo box class id.
pub const COMBO_BOX: u16 = 18;
/// The colour swatch class id.
pub const COLOUR_SWATCH: u16 = 23;
/// The numeric spinner class id.
pub const SPINNER: u16 = 16;
/// The progress bar class id.
pub const PROGRESS_BAR: u16 = 20;
/// The separator class id.
pub const SEPARATOR: u16 = 19;
/// The image class id.
pub const IMAGE: u16 = 24;
/// The texture view class id.
pub const TEXTURE_VIEW: u16 = 21;
/// The trackbar class id.
pub const TRACKBAR: u16 = 22;
/// The caption class id.
pub const CAPTION: u16 = 25;
/// The scroll bar class id.
pub const SCROLL_BAR: u16 = 32;

/// The eleven classes, in registration order.
pub const EXTENDED_CLASSES: [ExtendedClass; 11] = [
    ExtendedClass {
        id: TEXT_FIELD,
        name: "text field",
        size: (80, 24),
    },
    ExtendedClass {
        id: COMBO_BOX,
        name: "combo box",
        size: (80, 24),
    },
    ExtendedClass {
        id: COLOUR_SWATCH,
        name: "colour swatch",
        size: (0, 24),
    },
    ExtendedClass {
        id: SPINNER,
        name: "spinner",
        size: (60, 20),
    },
    ExtendedClass {
        id: PROGRESS_BAR,
        name: "progress bar",
        size: (0, 24),
    },
    ExtendedClass {
        id: SEPARATOR,
        name: "separator",
        size: (0, 3),
    },
    ExtendedClass {
        id: IMAGE,
        name: "image",
        size: (0, 0),
    },
    ExtendedClass {
        id: TEXTURE_VIEW,
        name: "texture view",
        size: (0, 0),
    },
    ExtendedClass {
        id: TRACKBAR,
        name: "trackbar",
        size: (0, 28),
    },
    ExtendedClass {
        id: CAPTION,
        name: "caption",
        size: (0, 20),
    },
    ExtendedClass {
        id: SCROLL_BAR,
        name: "scroll bar",
        size: (0, 16),
    },
];

/// The extended class with `id`.
pub fn extended_class(id: u16) -> Option<&'static ExtendedClass> {
    EXTENDED_CLASSES.iter().find(|class| class.id == id)
}

/// The text field's usable span: `w − 2·pad − 2·unit`.
pub fn text_field_span(w: i32, pad: i32, unit: i32) -> i32 {
    text_span_a(w, pad, unit)
}

/// The combo box's usable span: `w − (4·unit|1) − 2·pad − unit`.
pub fn combo_text_span(w: i32, pad: i32, unit: i32) -> i32 {
    text_span_b(w, pad, unit)
}

/// The spinner's usable span: `w − (3·unit|1) − 2·pad − 2·unit`.
pub fn spinner_text_span(w: i32, pad: i32, unit: i32) -> i32 {
    text_span_c(w, pad, unit)
}

/// The trackbar's default height (`20` when the record gives none) and pad
/// (`2`), set when the control is created.
pub fn trackbar_defaults(height: i32) -> (i32, i32) {
    (if height == 0 { 20 } else { height }, 2)
}

/// The scroll bar's pad (`2`), set when the control is created.
pub fn scrollbar_defaults() -> i32 {
    2
}

/// What a text field's key or pointer did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFieldEvent {
    /// The key was swallowed.
    None,
    /// The text or the selection changed.
    Changed,
    /// Enter committed the text.
    Submitted,
    /// The field lost the selection or focus.
    FocusLost,
    /// The key asks for the on-screen keyboard.
    Keyboard,
}

/// A single-line text field: the bytes, the selection anchor and caret, the
/// visible start and the maximum length. The editing rules are the text
/// edit unit's, re-exposed for the controls that host one.
#[derive(Debug, Clone, PartialEq)]
pub struct TextField {
    pub text: Vec<u8>,
    /// The selection anchor (0 to `text.len()`).
    pub anchor: usize,
    /// The caret (0 to `text.len()`).
    pub caret: usize,
    /// The first byte drawn.
    pub view_start: usize,
    /// The longest text the field accepts; the constructor turns 0 into
    /// 255, as the original's init does.
    pub max_len: usize,
    pub editable: bool,
}

impl TextField {
    /// A field around `text`.
    pub fn new(text: &[u8], max_len: usize, editable: bool) -> Self {
        // The original's init turns a zero limit into 255.
        let max_len = if max_len == 0 { 255 } else { max_len };
        let text = text.to_vec();
        let caret = text.len();
        Self {
            text,
            anchor: caret,
            caret,
            view_start: 0,
            max_len,
            editable,
        }
    }

    /// The selected byte range, low to high.
    pub fn selection(&self) -> (usize, usize) {
        if self.anchor <= self.caret {
            (self.anchor, self.caret)
        } else {
            (self.caret, self.anchor)
        }
    }

    /// Replaces the selection (or inserts at the caret) with `bytes`.
    pub fn insert(&mut self, bytes: &[u8]) {
        if !self.editable {
            return;
        }
        let (low, high) = self.selection();
        if self.max_len != 0 {
            let room = self.max_len.saturating_sub(self.text.len() - (high - low));
            let take = bytes.len().min(room);
            self.text.splice(low..high, bytes[..take].iter().copied());
            self.caret = low + take;
        } else {
            self.text.splice(low..high, bytes.iter().copied());
            self.caret = low + bytes.len();
        }
        self.anchor = self.caret;
    }

    /// Removes the selection, or the byte on `side` of the caret (`-1`
    /// before, `1` after).
    pub fn erase(&mut self, side: i32) {
        if !self.editable {
            return;
        }
        let (low, high) = self.selection();
        if low != high {
            self.text.drain(low..high);
            self.caret = low;
            self.anchor = low;
            return;
        }
        if side < 0 && self.caret > 0 {
            self.caret -= 1;
            self.text.remove(self.caret);
            self.anchor = self.caret;
        } else if side > 0 && self.caret < self.text.len() {
            self.text.remove(self.caret);
            self.anchor = self.caret;
        }
    }

    /// Handles a key. Printable bytes come as their Latin-1 code.
    pub fn key(&mut self, key: i32, shift: bool) -> TextFieldEvent {
        if !self.editable {
            return TextFieldEvent::None;
        }
        match key {
            13 => return TextFieldEvent::Submitted,
            8 => {
                self.erase(-1);
                return TextFieldEvent::Changed;
            }
            46 => {
                self.erase(1);
                return TextFieldEvent::Changed;
            }
            37 => {
                if self.caret > 0 {
                    self.caret -= 1;
                }
                if !shift {
                    self.anchor = self.caret;
                }
                return TextFieldEvent::Changed;
            }
            39 => {
                if self.caret < self.text.len() {
                    self.caret += 1;
                }
                if !shift {
                    self.anchor = self.caret;
                }
                return TextFieldEvent::Changed;
            }
            36 => {
                self.caret = 0;
                if !shift {
                    self.anchor = 0;
                }
                return TextFieldEvent::Changed;
            }
            35 => {
                self.caret = self.text.len();
                if !shift {
                    self.anchor = self.caret;
                }
                return TextFieldEvent::Changed;
            }
            256 => return TextFieldEvent::Keyboard,
            _ => {}
        }
        if (32..=255).contains(&key) {
            self.insert(&[key as u8]);
            return TextFieldEvent::Changed;
        }
        TextFieldEvent::None
    }

    /// Places the caret at `column`, as a press does: an anchor when not
    /// dragging, a selection end when it is.
    pub fn place(&mut self, column: usize, drag: bool) {
        self.caret = column.min(self.text.len());
        if !drag {
            self.anchor = self.caret;
        }
    }

    /// The byte column under `x`, from the text's own origin and advances.
    pub fn hit(&self, x: i32, origin: i32, advances: &[i16]) -> usize {
        let mut at = self.view_start.min(self.text.len());
        let mut left = x - origin;
        while at < self.text.len() {
            let advance = advances
                .get(usize::from(self.text[at]))
                .copied()
                .unwrap_or(1) as i32;
            if left < advance / 2 {
                break;
            }
            left -= advance;
            at += 1;
        }
        at
    }

    /// The field's first visible byte when the caret moved past its span.
    /// A guess: the original clips the text to the span and redraws, the
    /// scrolled start is not traced.
    pub fn scroll_to_caret(&mut self, origin: i32, span: i32, advances: &[i16]) {
        let mut width = 0;
        for &byte in &self.text[self.view_start.min(self.text.len())..self.caret] {
            width += advances.get(usize::from(byte)).copied().unwrap_or(1) as i32;
        }
        while width > span && self.view_start < self.caret {
            let byte = self.text[self.view_start];
            width -= advances.get(usize::from(byte)).copied().unwrap_or(1) as i32;
            self.view_start += 1;
        }
        let _ = origin;
    }
}

/// A drop-down list's placement under its combo box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropDownPlace {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub row: i32,
    pub count: i32,
}

/// A combo box: a selected item, its list and its open state.
#[derive(Debug, Clone, PartialEq)]
pub struct ComboBox {
    pub items: Vec<Vec<u8>>,
    /// The selected item, 1-based; 0 is none.
    pub selected: i32,
    pub open: bool,
    pub max_len: usize,
}

impl ComboBox {
    pub fn new(items: Vec<Vec<u8>>, selected: i32) -> Self {
        Self {
            items,
            selected,
            open: false,
            max_len: 255,
        }
    }

    /// The selected item's bytes, empty when none is selected.
    pub fn text(&self) -> &[u8] {
        usize::try_from(self.selected)
            .ok()
            .and_then(|at| at.checked_sub(1))
            .and_then(|at| self.items.get(at))
            .map(|text| &text[..text.len().min(self.max_len)])
            .unwrap_or(&[])
    }

    /// Selects a 1-based item, clamped to the list; returns whether it
    /// changed.
    pub fn choose(&mut self, item: i32) -> bool {
        if self.items.is_empty() {
            return false;
        }
        let chosen = item.clamp(1, self.items.len() as i32);
        if chosen == self.selected {
            return false;
        }
        self.selected = chosen;
        true
    }

    /// Opens the list under `rect`; `None` for an empty list.
    pub fn open_below(
        &mut self,
        rect: (i32, i32, i32, i32),
        metrics: ListMetrics,
        window_h: i32,
    ) -> Option<DropDownPlace> {
        if self.items.is_empty() {
            return None;
        }
        let count = self.items.len() as i32;
        let extent = list_extent(metrics, count);
        let row = metrics.row();
        let mut y = rect.1 + rect.3 + 1;
        if y + extent.height > window_h {
            // Guess: the list flips above when it would leave the window.
            y = rect.1 - extent.height;
        }
        self.open = true;
        Some(DropDownPlace {
            x: rect.0,
            y,
            w: rect.2,
            h: extent.height,
            row,
            count,
        })
    }
}

/// A numeric spinner's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spinner {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    /// Digits after the point; 0 with `integer` formats as an integer.
    pub precision: i32,
    /// The control's style bit stores an integer rather than a float.
    pub integer: bool,
    /// The edit is in progress (the caret is in the text zone).
    pub editing: bool,
}

impl Default for Spinner {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 0.0,
            step: 0.0,
            precision: 0,
            integer: false,
            editing: false,
        }
    }
}

/// What setting a spinner's value did.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinnerSet {
    pub changed: bool,
    /// The text named a value outside the range and was clamped.
    pub clamped: bool,
}

/// A spinner's text zone or one of its two step halves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerPart {
    Text,
    Up,
    Down,
}

impl Spinner {
    /// The init rule: a zero step becomes 1, a zero maximum 100, and a
    /// value under the minimum is raised to it.
    pub fn init(&mut self) {
        if self.step == 0.0 {
            self.step = 1.0;
        }
        if self.max == 0.0 {
            self.max = 100.0;
        }
        if self.value < self.min {
            self.value = self.min;
        }
    }

    /// Sets the value from a typed number with the original's ordered
    /// clamp: `min ≤ v ≤ max`, a NaN falling to the minimum.
    pub fn set(&mut self, value: f32, report: bool) -> SpinnerSet {
        let mut v = value;
        if self.min <= v {
            if self.max <= v && v != self.max {
                v = self.max;
            }
        } else {
            v = self.min;
        }
        let changed = (v - self.value).abs() > 1e-8;
        let clamped = v != value;
        if changed {
            self.value = v;
        }
        let _ = report;
        SpinnerSet { changed, clamped }
    }

    /// Steps the value up or down by one step, clamped to the range.
    pub fn step(&mut self, up: bool) -> bool {
        let next = if up {
            self.value + self.step
        } else {
            self.value - self.step
        };
        let mut v = next;
        if self.min <= v {
            if self.max <= v && v != self.max {
                v = self.max;
            }
        } else {
            v = self.min;
        }
        if v != self.value {
            self.value = v;
            true
        } else {
            false
        }
    }

    /// The shown text: the value with the configured digits.
    pub fn text(&self) -> String {
        if self.integer {
            format!("{}", self.value as i32)
        } else {
            format!("{:.*}", self.precision.max(0) as usize, self.value)
        }
    }

    /// The part under (`x`, `y`) within `rect` and `span` (the usable text
    /// width).
    pub fn part(&self, x: i32, y: i32, rect: (i32, i32, i32, i32), span: i32) -> SpinnerPart {
        if x < rect.0 + rect.2 - span {
            SpinnerPart::Text
        } else if y < rect.1 + rect.3 / 2 {
            SpinnerPart::Up
        } else {
            SpinnerPart::Down
        }
    }
}

/// The spinner's auto-repeat pixel step: `round(unit × 96 / ((max − min) /
/// step))`, clamped to `unit/2 .. 8·unit`.
pub fn spinner_repeat_step(unit: i32, min: f32, max: f32, step: f32) -> i32 {
    let range = (max - min) / step;
    let raw = unit as f32 * 96.0 / range;
    if raw.is_nan() {
        unit / 2
    } else {
        (raw.round_ties_even() as i32).clamp(unit / 2, unit * 8)
    }
}

/// A progress bar's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressBar {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub precision: i32,
    pub integer: bool,
}

impl Default for ProgressBar {
    fn default() -> Self {
        Self {
            value: 0.0,
            min: 0.0,
            max: 0.0,
            step: 0.0,
            precision: 0,
            integer: false,
        }
    }
}

impl ProgressBar {
    /// The init rule: a zero step becomes 1, a zero maximum 100, a value
    /// under the minimum raised to it.
    pub fn init(&mut self) {
        if self.step == 0.0 {
            self.step = 1.0;
        }
        if self.max == 0.0 {
            self.max = 100.0;
        }
        if self.value < self.min {
            self.value = self.min;
        }
    }

    /// Sets the value; a negative value becomes zero (NaN stays, as the
    /// original's ordered test leaves it).
    pub fn set(&mut self, value: f32) -> bool {
        let mut v = value;
        if v < 0.0 {
            v = 0.0;
        }
        if v != self.value {
            self.value = v;
            true
        } else {
            false
        }
    }

    /// The shown text: the value times a hundred, one digit, then `%`.
    pub fn text(&self) -> String {
        // The original formats this text with a fixed one digit.
        format!("{:.1}%", self.value * 100.0)
    }

    /// The filled width: `round(width × value)`.
    pub fn fill(&self, width: i32) -> i32 {
        (width as f32 * self.value).round_ties_even() as i32
    }
}

/// The separator's gap when it carries text: one line height a byte.
pub fn separator_gap(font_height: i32, text_len: i32) -> i32 {
    font_height.saturating_mul(text_len)
}

/// The separator's line rectangles. Text leaves a gap at the centre, or at
/// the right when the alignment's 0x2000 bit is set.
pub fn separator_lines(
    rect: (i32, i32, i32, i32),
    gap: i32,
    right: bool,
    unit: i32,
) -> Vec<(i32, i32, i32, i32)> {
    let (x, y, w, h) = rect;
    if gap <= 0 {
        return vec![(x, y, w, h)];
    }
    if right {
        // The line keeps its right edge and starts after the gap and a unit.
        return vec![(x + gap + unit, y, w - gap - unit, h)];
    }
    let first = (w - gap) / 2 - 1;
    vec![(x, y, first, h), (x + w - first, y, first, h)]
}

/// An image view: the normalised point picked by a press.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePick {
    pub u: f32,
    pub v: f32,
}

/// The image's inset rectangle.
pub fn image_rect(rect: (i32, i32, i32, i32), pad: i32) -> (i32, i32, i32, i32) {
    (
        rect.0 + pad,
        rect.1 + pad,
        rect.2 - 2 * pad,
        rect.3 - 2 * pad,
    )
}

/// The normalised point under a press in an image view. `u` is across,
/// `v` is up (the original measures from the bottom), both clamped to 0..1
/// with NaN falling to 0.
pub fn image_pick(rect: (i32, i32, i32, i32), pad: i32, x: i32, y: i32) -> ImagePick {
    let inner_w = rect.2 - pad;
    let inner_h = rect.3 - pad;
    let clamp = |value: f32| {
        if 0.0 <= value {
            if 1.0 < value { 1.0 } else { value }
        } else {
            0.0
        }
    };
    let u = if inner_w == 0 {
        f32::NAN
    } else {
        (x - rect.0 - pad) as f32 / inner_w as f32
    };
    let v = if inner_h == 0 {
        f32::NAN
    } else {
        1.0 - (y - rect.1 - pad) as f32 / inner_h as f32
    };
    ImagePick {
        u: clamp(u),
        v: clamp(v),
    }
}

/// A texture view's centred rectangle for a texture of `tex` within `rect`
/// and `pad`.
pub fn texture_rect(
    rect: (i32, i32, i32, i32),
    _pad: i32,
    tex: (i32, i32),
) -> (i32, i32, i32, i32) {
    (
        rect.0 + (rect.2 - tex.0) / 2,
        rect.1 + (rect.3 - tex.1) / 2,
        tex.0,
        tex.1,
    )
}

/// A caption's alignment: the original draws its text with vertical style
/// 0x300 (a line centred in the box) and the horizontal style's own bit.
pub fn caption_align(style: u32) -> u32 {
    (style & 0x7000) | 0x300
}

/// A colour swatch's press result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwatchPress {
    Ignored,
    /// The owner's callback.
    Callback,
    /// Open the colour picker (the original's 0x105 result).
    Picker,
}

/// The colour swatch's press rule: the picker bit opens the picker, the
/// callback bit calls the owner, anything else does nothing.
pub fn swatch_press(flags: u32) -> SwatchPress {
    if flags & 0x4000 != 0 {
        SwatchPress::Picker
    } else if flags & 0x2000 != 0 {
        SwatchPress::Callback
    } else {
        SwatchPress::Ignored
    }
}

/// The colour drag's payload code.
pub const COLOUR_DRAG_CODE: u32 = 0xcd0c;

/// Whether a drag asking for `code` is accepted: true for the colour code.
pub fn swatch_accepts(code: u32) -> bool {
    code == COLOUR_DRAG_CODE
}

/// A scroll bar's thumb: its width (`4 × unit + 1`) and left offset.
pub fn scroll_thumb(unit: i32, pad: i32, w: i32, value: f32) -> (i32, i32) {
    let width = 4 * unit + 1;
    let offset = ((w - 2 * pad - width) as f32 * value).round_ties_even() as i32;
    (width, offset)
}

/// The scroll bar's value at `x`: `(x − left − pad − thumb/2) / travel`,
/// clamped with the ordered pair so NaN falls to 0.
pub fn scroll_value(unit: i32, x0: i32, pad: i32, w: i32, x: i32) -> f32 {
    let thumb = 4 * unit + 1;
    let num = x - x0 - pad - thumb / 2;
    let den = w - 2 * pad - thumb;
    let mut value = (num as f64 / den as f64) as f32;
    if 0.0 <= value {
        if 1.0 < value {
            value = 1.0;
        }
    } else {
        value = 0.0;
    }
    value
}

/// The scroll bar's current-value label: the value, no digits.
pub fn scroll_label(value: f32) -> String {
    format!("{}", value as i32)
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn the_extended_classes_and_defaults_are_in_registration_order() {
        assert_eq!(extended_class(TEXT_FIELD).unwrap().size, (80, 24));
        assert_eq!(extended_class(COMBO_BOX).unwrap().size, (80, 24));
        assert_eq!(extended_class(COLOUR_SWATCH).unwrap().size, (0, 24));
        assert_eq!(extended_class(SPINNER).unwrap().size, (60, 20));
        assert_eq!(extended_class(PROGRESS_BAR).unwrap().size, (0, 24));
        assert_eq!(extended_class(SEPARATOR).unwrap().size, (0, 3));
        assert_eq!(extended_class(IMAGE).unwrap().size, (0, 0));
        assert_eq!(extended_class(TEXTURE_VIEW).unwrap().size, (0, 0));
        assert_eq!(extended_class(TRACKBAR).unwrap().size, (0, 28));
        assert_eq!(extended_class(CAPTION).unwrap().size, (0, 20));
        assert_eq!(extended_class(SCROLL_BAR).unwrap().size, (0, 16));
        assert_eq!(extended_class(1), None);
        assert_eq!(text_inset(4), 13);
        assert_eq!(text_inset(5), 15);
        assert_eq!(text_field_span(100, 2, 4), 88);
        assert_eq!(combo_text_span(100, 2, 4), 75);
        assert_eq!(spinner_text_span(100, 2, 4), 75);
        assert_eq!(trackbar_defaults(0), (20, 2));
        assert_eq!(trackbar_defaults(24), (24, 2));
        assert_eq!(scrollbar_defaults(), 2);
    }

    fn advances() -> Vec<i16> {
        let mut table = vec![1i16; 256];
        table[usize::from(b'A')] = 10;
        table[usize::from(b'B')] = 10;
        table
    }

    #[test]
    fn a_text_field_hits_edits_and_limits_its_bytes() {
        let mut field = TextField::new(b"abc", 0, true);
        assert_eq!(field.caret, 3);
        assert_eq!(field.key(256, false), TextFieldEvent::Keyboard);
        assert_eq!(field.key(13, false), TextFieldEvent::Submitted);
        assert_eq!(field.key(8, false), TextFieldEvent::Changed);
        assert_eq!(field.text, b"ab");
        field.place(1, false);
        assert_eq!(field.key(b'Z' as i32, false), TextFieldEvent::Changed);
        assert_eq!(field.text, b"aZb");
        assert_eq!(field.caret, 2);
        // Shift-left keeps the anchor, so typing replaces the selection.
        let mut selected = TextField::new(b"abc", 0, true);
        selected.place(1, false);
        selected.key(37, true);
        assert_eq!(selected.selection(), (0, 1));
        selected.insert(b"X");
        assert_eq!(selected.text, b"Xbc");
        assert_eq!(selected.caret, 1);
        // A max length clips the insert; zero becomes the original's 255.
        let mut limited = TextField::new(b"ab", 3, true);
        limited.insert(b"CD");
        assert_eq!(limited.text, b"abC");
        let mut default_limited = TextField::new(b"", 0, true);
        default_limited.insert(&[b'x'; 300]);
        assert_eq!(default_limited.text.len(), 255);
        // Read-only fields take nothing.
        let mut read_only = TextField::new(b"ab", 0, false);
        assert_eq!(read_only.key(b'X' as i32, false), TextFieldEvent::None);
        assert_eq!(read_only.text, b"ab");
    }

    #[test]
    fn a_text_field_hit_takes_the_byte_whose_middle_is_right() {
        let mut field = TextField::new(b"AB", 0, true);
        assert_eq!(field.hit(4, 0, &advances()), 0);
        assert_eq!(field.hit(5, 0, &advances()), 1);
        assert_eq!(field.hit(14, 0, &advances()), 1);
        assert_eq!(field.hit(20, 0, &advances()), 2);
        assert_eq!(field.hit(99, 0, &advances()), 2);
        field.place(2, true);
        assert_eq!(field.selection(), (2, 2));
    }

    #[test]
    fn a_combo_box_chooses_and_opens_its_list() {
        let mut combo = ComboBox::new(vec![b"one".to_vec(), b"two".to_vec()], 1);
        assert_eq!(combo.text(), b"one");
        assert!(combo.choose(2));
        assert_eq!(combo.text(), b"two");
        assert!(!combo.choose(9));
        assert_eq!(combo.selected, 2);
        let place = combo
            .open_below((10, 20, 80, 24), ListMetrics { unit: 4, inset: 3 }, 200)
            .unwrap();
        assert_eq!((place.x, place.y, place.w), (10, 45, 80));
        assert_eq!(place.h, 54);
        assert_eq!(place.row, 24);
        let flipped = combo
            .open_below((10, 160, 80, 24), ListMetrics { unit: 4, inset: 3 }, 200)
            .unwrap();
        assert_eq!(flipped.y, 106);
        let mut empty = ComboBox::new(Vec::new(), 0);
        assert_eq!(
            empty.open_below((0, 0, 10, 10), ListMetrics { unit: 4, inset: 3 }, 100),
            None
        );
        assert!(!empty.open);
    }

    #[test]
    fn a_spinner_clamps_steps_and_formats() {
        let mut spinner = Spinner {
            value: -5.0,
            ..Spinner::default()
        };
        spinner.init();
        assert_eq!(spinner.step, 1.0);
        assert_eq!(spinner.max, 100.0);
        assert_eq!(spinner.value, 0.0);

        let set = spinner.set(150.0, true);
        assert!(set.changed);
        assert!(set.clamped);
        assert_eq!(spinner.value, 100.0);
        let nan = spinner.set(f32::NAN, true);
        assert!(nan.changed);
        assert_eq!(spinner.value, 0.0);
        assert!(!spinner.step(false));
        spinner.value = 1.0;
        assert!(spinner.step(false));
        assert_eq!(spinner.value, 0.0);

        let mut precise = Spinner {
            value: 2.6,
            min: 0.0,
            max: 10.0,
            step: 0.5,
            precision: 2,
            ..Spinner::default()
        };
        precise.init();
        assert_eq!(precise.text(), "2.60");
        precise.precision = 0;
        assert_eq!(precise.text(), "3");

        let integer = Spinner {
            value: 3.9,
            integer: true,
            ..Spinner::default()
        };
        assert_eq!(integer.text(), "3");

        let rect = (0, 0, 60, 20);
        let span = spinner_text_span(60, 2, 4);
        assert_eq!(span, 35);
        assert_eq!(spinner.part(10, 5, rect, span), SpinnerPart::Text);
        assert_eq!(spinner.part(40, 5, rect, span), SpinnerPart::Up);
        assert_eq!(spinner.part(40, 15, rect, span), SpinnerPart::Down);
        assert_eq!(spinner_repeat_step(4, 0.0, 100.0, 1.0), 4);
        // The clamp keeps a tiny step at half a unit and a huge one at 8.
        assert_eq!(spinner_repeat_step(4, 0.0, 1000.0, 1.0), 2);
        assert_eq!(spinner_repeat_step(4, 0.0, 1.0, 1.0), 32);
    }

    #[test]
    fn a_progress_bar_starts_at_zero_and_fills_by_its_value() {
        let mut bar = ProgressBar {
            value: -1.0,
            ..ProgressBar::default()
        };
        bar.init();
        assert_eq!(bar.step, 1.0);
        assert_eq!(bar.max, 100.0);
        assert_eq!(bar.value, 0.0);
        assert!(!bar.set(-2.0));
        assert!(bar.set(0.5));
        assert_eq!(bar.text(), "50.0%");
        assert_eq!(bar.fill(100), 50);
        assert_eq!(bar.fill(101), 50);
        let mut whole = ProgressBar {
            precision: 0,
            ..ProgressBar::default()
        };
        whole.value = 0.25;
        assert_eq!(whole.text(), "25.0%");
        let mut negative = ProgressBar {
            min: -5.0,
            value: -10.0,
            ..ProgressBar::default()
        };
        negative.init();
        assert_eq!(negative.value, -5.0);
        assert!(negative.set(f32::NAN));
        assert!(negative.value.is_nan());
    }

    #[test]
    fn a_separator_leaves_a_gap_for_its_text() {
        assert_eq!(separator_gap(16, 2), 32);
        assert_eq!(
            separator_lines((0, 0, 100, 3), 0, false, 4),
            vec![(0, 0, 100, 3)]
        );
        assert_eq!(
            separator_lines((0, 0, 100, 3), 32, false, 4),
            vec![(0, 0, 33, 3), (67, 0, 33, 3)]
        );
        assert_eq!(
            separator_lines((0, 0, 100, 3), 32, true, 4),
            vec![(36, 0, 64, 3)]
        );
    }

    #[test]
    fn an_image_view_picks_its_normalised_point() {
        let rect = (10, 20, 100, 80);
        assert_eq!(image_rect(rect, 2), (12, 22, 96, 76));
        let pick = image_pick(rect, 2, 12, 22);
        assert_eq!((pick.u, pick.v), (0.0, 1.0));
        let pick = image_pick(rect, 2, 61, 61);
        assert_eq!(pick.u, 0.5);
        assert_eq!(pick.v, 0.5);
        let pick = image_pick(rect, 2, 200, 200);
        assert_eq!((pick.u, pick.v), (1.0, 0.0));
        // A zero inner extent is NaN, and the ordered clamp makes it 0.
        let pick = image_pick((0, 0, 2, 2), 2, 0, 0);
        assert_eq!((pick.u, pick.v), (0.0, 0.0));
    }

    #[test]
    fn a_texture_view_centres_its_texture() {
        assert_eq!(
            texture_rect((10, 20, 100, 80), 2, (40, 30)),
            (40, 45, 40, 30)
        );
        assert_eq!(texture_rect((0, 0, 10, 10), 0, (20, 20)), (-5, -5, 20, 20));
    }

    #[test]
    fn a_colour_swatch_opens_the_picker_or_calls_back() {
        assert_eq!(swatch_press(0x4000), SwatchPress::Picker);
        assert_eq!(swatch_press(0x2000), SwatchPress::Callback);
        assert_eq!(swatch_press(0), SwatchPress::Ignored);
        assert!(swatch_accepts(COLOUR_DRAG_CODE));
        assert!(!swatch_accepts(COLOUR_DRAG_CODE + 1));
    }

    #[test]
    fn a_scroll_bar_maps_its_thumb() {
        assert_eq!(scroll_thumb(4, 2, 100, 0.0), (17, 0));
        assert_eq!(scroll_thumb(4, 2, 100, 0.5), (17, 40));
        assert_eq!(scroll_thumb(4, 2, 100, 1.0), (17, 79));
        assert_eq!(scroll_value(4, 0, 2, 100, 10), 0.0);
        assert_eq!(scroll_value(4, 0, 2, 100, 89), 1.0);
        // Zero travel gives NaN, which the ordered clamp sends to 0.
        assert_eq!(scroll_value(4, 0, 2, 21, 10), 0.0);
        assert_eq!(scroll_label(42.9), "42");
    }

    #[test]
    fn a_text_and_combo_field_trace_their_inner_parts() {
        assert_eq!(text_border((10, 20, 80, 24)), (11, 21, 78, 22));
        assert_eq!(combo_arrow((10, 20, 80, 24), 4), (10 + 80 - 16, 20 + 5));
    }

    #[test]
    fn a_combo_press_opens_or_edits() {
        assert_eq!(combo_press(false, true, true), ComboPress::Open);
        assert_eq!(combo_press(false, false, true), ComboPress::Edit);
        assert_eq!(combo_press(true, false, true), ComboPress::Edit);
        assert_eq!(combo_press(false, true, false), ComboPress::Ignored);
    }

    #[test]
    fn a_spinner_splits_its_buttons_and_filters_its_bytes() {
        let buttons = spinner_buttons((0, 0, 60, 20), 35);
        assert_eq!(buttons.text, (0, 0, 25, 20));
        assert_eq!(buttons.up, (25, 0, 35, 10));
        assert_eq!(buttons.down, (25, 10, 35, 10));

        let mut field = TextField::new(b"", 0, true);
        assert!(spinner_accepts(&field, b'5'));
        assert!(!spinner_accepts(&field, b'x'));
        assert!(spinner_accepts(&field, b'-'));
        field.insert(b"5");
        field.place(1, false);
        assert!(!spinner_accepts(&field, b'-'));
        field.insert(b".");
        field.place(1, false);
        assert!(!spinner_accepts(&field, b'.'));

        let mut spinner = Spinner {
            value: 1.0,
            min: 0.0,
            max: 2.0,
            step: 1.0,
            ..Spinner::default()
        };
        let mut field = TextField::new(b"", 0, true);
        assert_eq!(
            spinner_key(&mut spinner, &field, 0x26),
            SpinnerKey::Stepped(true)
        );
        assert_eq!(spinner.value, 2.0);
        assert_eq!(
            spinner_key(&mut spinner, &field, 0x28),
            SpinnerKey::Stepped(false)
        );
        assert_eq!(spinner.value, 1.0);
        assert_eq!(
            spinner_key(&mut spinner, &field, b'7' as i32),
            SpinnerKey::Edited
        );
        assert_eq!(spinner_key(&mut spinner, &field, 13), SpinnerKey::Committed);
        assert_eq!(
            spinner_key(&mut spinner, &field, b'x' as i32),
            SpinnerKey::Ignored
        );
        field.insert(b"1");
    }

    #[test]
    fn a_scroll_bar_labels_its_current_value() {
        let labels = scroll_labels(b"volume", b"%", 0.5, 0.0, 100.0);
        assert_eq!(labels.text, b"volume");
        assert_eq!(labels.unit, b"%");
        assert_eq!(labels.value, "50");
        assert_eq!(colour_drag_rect(20, 10), (0, 0, 20, 10));
    }

    #[test]
    fn a_caption_uses_the_centred_vertical_style() {
        assert_eq!(caption_align(0x1000), 0x1300);
        assert_eq!(caption_align(0), 0x300);
    }
}

/// A text control's inner border: the rectangle one pixel inside the
/// control, as the text field's draw-back pass traces it.
pub fn text_border(rect: (i32, i32, i32, i32)) -> (i32, i32, i32, i32) {
    (rect.0 + 1, rect.1 + 1, rect.2 - 2, rect.3 - 2)
}

/// A combo box's drop-down arrow: its left edge and top, from the class's
/// `(4·unit|1) − unit/2 + 1` width and `(unit+1) − (unit&1)` height.
pub fn combo_arrow(rect: (i32, i32, i32, i32), unit: i32) -> (i32, i32) {
    let width = ((4 * unit) | 1) - unit / 2 + 1;
    let height = (unit + 1) - (unit & 1);
    (rect.0 + rect.2 - width, rect.1 + height)
}

/// What a combo box's press did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboPress {
    Ignored,
    /// The button opened the list.
    Open,
    /// The press landed in the text and moves the caret.
    Edit,
}

/// The combo box's press rule: while open the text takes the caret, while
/// closed the arrow opens the list.
pub fn combo_press(open: bool, on_arrow: bool, button_left: bool) -> ComboPress {
    if !button_left {
        return ComboPress::Ignored;
    }
    if open || on_arrow {
        if open {
            ComboPress::Edit
        } else {
            ComboPress::Open
        }
    } else {
        ComboPress::Edit
    }
}

/// A spinner's two step buttons and text zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpinnerButtons {
    pub text: (i32, i32, i32, i32),
    pub up: (i32, i32, i32, i32),
    pub down: (i32, i32, i32, i32),
}

/// Splits a spinner into its text zone and its up and down halves.
pub fn spinner_buttons(rect: (i32, i32, i32, i32), span: i32) -> SpinnerButtons {
    let (x, y, w, h) = rect;
    let text_w = w - span;
    SpinnerButtons {
        text: (x, y, text_w, h),
        up: (x + text_w, y, span, h / 2),
        down: (x + text_w, y + h / 2, span, h - h / 2),
    }
}

/// Whether the spinner takes a typed byte: a digit, a dot unless one is
/// already at the caret, or a minus only at an empty caret.
pub fn spinner_accepts(field: &TextField, byte: u8) -> bool {
    if byte == b'.' {
        field.text.get(field.caret).copied() != Some(b'.')
    } else if byte == b'-' {
        field.caret == 0 && field.anchor == 0
    } else {
        byte.is_ascii_digit()
    }
}

/// What a spinner's key did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerKey {
    Ignored,
    /// A byte was accepted by the text edit unit.
    Edited,
    /// Enter committed the typed value.
    Committed,
    /// The up or down key stepped the value.
    Stepped(bool),
}

/// A spinner's key: up and down step, Enter commits, the filter admits the
/// numeric bytes and the rest goes to the text edit unit.
pub fn spinner_key(spinner: &mut Spinner, field: &TextField, key: i32) -> SpinnerKey {
    match key {
        0x26 => {
            if spinner.step(true) {
                SpinnerKey::Stepped(true)
            } else {
                SpinnerKey::Ignored
            }
        }
        0x28 => {
            if spinner.step(false) {
                SpinnerKey::Stepped(false)
            } else {
                SpinnerKey::Ignored
            }
        }
        13 => SpinnerKey::Committed,
        _ if (0..=255).contains(&key) && spinner_accepts(field, key as u8) => SpinnerKey::Edited,
        _ => SpinnerKey::Ignored,
    }
}

/// The scroll bar's three captions: its own text at the left, its unit text
/// at the right and the current value in the middle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollLabels {
    pub text: Vec<u8>,
    pub unit: Vec<u8>,
    pub value: String,
}

/// The scroll bar's captions; the value is `(max − min) × value + min`,
/// rounded to a whole number.
pub fn scroll_labels(text: &[u8], suffix: &[u8], value: f32, min: f32, max: f32) -> ScrollLabels {
    let shown = (max - min) * value + min;
    ScrollLabels {
        text: text.to_vec(),
        unit: suffix.to_vec(),
        value: format!("{:.0}", shown),
    }
}

/// A colour drag payload's draw rectangle, relative to the control.
pub fn colour_drag_rect(w: i32, h: i32) -> (i32, i32, i32, i32) {
    (0, 0, w, h)
}

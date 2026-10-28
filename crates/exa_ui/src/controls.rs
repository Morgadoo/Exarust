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

    /// The value at a cursor x: `(cursor − x − pad − span/2) / travel` in
    /// double precision, then below 0 is 0 and above 1 is 1; a NaN (zero
    /// travel) passes both, as the original's unordered compares let it.
    /// With `steps` the value is quantised: `steps × f` rounded half to even
    /// in 64 bits (a NaN to the indefinite integer), divided back in double
    /// precision.
    pub fn value_at(self, x: i32, w: i32, cursor: i32, steps: i32) -> f32 {
        let num = cursor - x - self.pad - self.span / 2;
        let den = self.travel(w);
        // `f32::clamp` leaves a NaN alone, as the unordered compares do.
        let mut f = ((num as f64 / den as f64) as f32).clamp(0.0, 1.0);
        if steps > 0 {
            let q = round_to_i64(steps as f32 * f);
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
        // A zero travel makes 0/0, which passes both clamps; quantised, it
        // rounds to the indefinite integer.
        assert!(t.value_at(0, 24, 12, 0).is_nan());
        assert_eq!(t.value_at(0, 24, 12, 4), (i64::MIN as f64 / 4.0) as f32);
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
        0x300 => ((line_h + h + 2) >> 1) + y,
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
        // The centre's halving is a shift: −3 halves to −2.
        assert_eq!(align_label(0, 0, -21, 2, 4, 0, 0, 16, 14, 0x300).y, -2);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0).y, 102);
    }
}

/// `CVTSS2SI` into 64 bits: rounds half to even; NaN and values outside
/// the range give `i64::MIN`, the instruction's "integer indefinite". The
/// controls convert this way and keep the low 32 bits where they want an
/// `int`, so an indefinite result reads as 0 there.
pub fn round_to_i64(value: f32) -> i64 {
    let rounded = value.round_ties_even();
    if rounded.is_nan() || rounded < i64::MIN as f32 || rounded >= 9.223_372e18 {
        i64::MIN
    } else {
        rounded as i64
    }
}

/// The spinner's button width, `3 × unit | 1`: the unit-change hook the
/// unit installs (ahead of the previous hook, which it then calls) caches
/// it for every spinner.
pub fn spinner_button_width(unit: i32) -> i32 {
    (3 * unit) | 1
}

/// An extended control class as the unit registers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtendedClass {
    pub id: u16,
    pub name: &'static str,
    /// The record's default size (w, h); 0 keeps the form's.
    pub size: (i32, i32),
    /// Whether the class has a creation hook.
    pub has_init: bool,
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

const fn class(id: u16, name: &'static str, size: (i32, i32), has_init: bool) -> ExtendedClass {
    ExtendedClass {
        id,
        name,
        size,
        has_init,
    }
}

/// The eleven classes, in registration order, with their records' default
/// sizes and whether a creation hook is set.
pub const EXTENDED_CLASSES: [ExtendedClass; 11] = [
    class(TEXT_FIELD, "text field", (80, 24), true),
    class(COMBO_BOX, "combo box", (80, 24), true),
    class(COLOUR_SWATCH, "colour swatch", (0, 24), false),
    class(SPINNER, "spinner", (60, 20), true),
    class(PROGRESS_BAR, "progress bar", (0, 24), false),
    class(SEPARATOR, "separator", (0, 3), false),
    class(IMAGE, "image", (0, 0), false),
    class(TEXTURE_VIEW, "texture view", (0, 0), false),
    class(TRACKBAR, "trackbar", (0, 28), true),
    class(CAPTION, "caption", (0, 20), false),
    class(SCROLL_BAR, "scroll bar", (0, 16), true),
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

/// The spinner's usable span: `w − button − 2·pad − 2·unit`, the button
/// [`spinner_button_width`].
pub fn spinner_text_span(w: i32, pad: i32, unit: i32) -> i32 {
    text_span_c(w, pad, unit)
}

/// The trackbar's creation hook: a knob width of 0 becomes 20, and the pad
/// is 2. Returns `(span, pad)`.
pub fn trackbar_defaults(span: i32) -> (i32, i32) {
    (if span == 0 { DEFAULT_SPAN } else { span }, DEFAULT_PAD)
}

/// The trackbar's knob width after the unit changes from `old_unit`:
/// `unit × (span / old_unit)` in double precision, rounded half to even.
pub fn trackbar_rescale(span: i32, old_unit: i32, unit: i32) -> i32 {
    let scaled = f64::from(unit) * (f64::from(span) / f64::from(old_unit));
    if scaled.is_nan() || scaled.abs() >= 9.2e18 {
        0
    } else {
        scaled.round_ties_even() as i64 as i32
    }
}

/// The scroll bar's creation hook sets its pad to 2.
pub fn scrollbar_defaults() -> i32 {
    DEFAULT_PAD
}

/// A text control's edit state: the selection anchor and caret, and the
/// drawn run (`start` to `end`), all byte positions.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EditState {
    pub anchor: u8,
    pub caret: u8,
    pub end: u8,
    pub start: u8,
}

impl EditState {
    /// Selects the whole text (the anchor at 0, the caret at the end), as
    /// a control that gains the focus does.
    pub fn select_all(&mut self, len: usize) {
        self.anchor = 0;
        self.caret = len as u8;
    }

    /// The reset message: the run starts at 0 and ends at the text's end,
    /// with the anchor and caret there too.
    pub fn reset(&mut self, len: usize) {
        self.start = 0;
        self.end = len as u8;
        self.anchor = self.end;
        self.caret = self.end;
    }

    /// The caret position under a pointer `x` already made relative to the
    /// text's origin ([`align_label`]): walking the drawn run from `start`,
    /// the first byte whose half advance (an unsigned shift) passes the
    /// remaining offset, else `end`; below 1 is 0, and past the text's
    /// length is its length. `advances` holds the font's per-byte widths.
    pub fn hit(&self, text: &[u8], x: i32, advances: &[i16]) -> u8 {
        let mut left = x;
        let mut at = i32::from(self.end);
        let mut index = i32::from(self.start);
        while index < i32::from(self.end) {
            let byte = text.get(index as usize).copied().unwrap_or(0);
            let advance = advances.get(usize::from(byte)).copied().unwrap_or(0);
            let half = ((i32::from(advance) as u32) >> 1) as i32;
            if left < half {
                at = index;
                break;
            }
            left -= i32::from(advance);
            index += 1;
        }
        let at = at.max(0) as usize;
        at.min(text.len()) as u8
    }
}

/// What a key does in a text field (its handler's key message). The
/// editing itself belongs to the text edit unit the field calls.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFieldKey {
    /// Enter with an owner callback: it is called with the "submitted" bit
    /// set; with `release` the field also gives the focus up.
    Submit { release: bool },
    /// A digits-only field drops a typed byte that is not a digit.
    Dropped,
    /// An editable field passes the key to the text edit unit with its
    /// length limit, then calls the owner when `notify`.
    Edit { max_len: i32, notify: bool },
    /// A read-only field passes the key to the text edit unit's moves only
    /// (with a 255-byte limit).
    Move,
}

/// The text field's style bits the key handler reads.
pub mod text_style {
    /// The bits that make a field editable (all three).
    pub const EDITABLE: u32 = 0xd;
    /// The owner is called after every edit.
    pub const NOTIFY: u32 = 0x100_0000;
    /// Enter gives the focus up.
    pub const RELEASE: u32 = 0x200_0000;
    /// Typed bytes must be digits.
    pub const DIGITS: u32 = 0x400_0000;
}

/// The text field's key rule. `typed` is the message's "typed byte" flag;
/// `has_callback` whether the owner gave one.
pub fn text_field_key(style: u32, key: u32, typed: bool, has_callback: bool) -> TextFieldKey {
    if key == 0x0d && has_callback {
        return TextFieldKey::Submit {
            release: style & text_style::RELEASE != 0,
        };
    }
    if style & text_style::EDITABLE == text_style::EDITABLE {
        let byte = key as u8;
        if style & text_style::DIGITS != 0 && typed && !byte.is_ascii_digit() {
            return TextFieldKey::Dropped;
        }
        return TextFieldKey::Edit {
            max_len: 0,
            notify: style & text_style::NOTIFY != 0 && has_callback,
        };
    }
    TextFieldKey::Move
}

/// A single-line text field: its bytes, edit state and limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextField {
    pub text: Vec<u8>,
    pub edit: EditState,
    /// The longest text the field takes; creation turns 0 into 255.
    pub max_len: i32,
    pub style: u32,
}

impl TextField {
    /// A field as created: a zero limit becomes 255.
    pub fn new(text: &[u8], max_len: i32, style: u32) -> Self {
        let text = text.to_vec();
        Self {
            edit: EditState {
                end: text.len() as u8,
                ..EditState::default()
            },
            text,
            max_len: if max_len == 0 { 255 } else { max_len },
            style,
        }
    }

    /// The pointer-shape query: a text cursor that takes the focus.
    pub const CURSOR: u32 = 0x801_0000;

    /// A press places the caret under `x` (relative to the text origin).
    pub fn press(&mut self, x: i32, advances: &[i16]) {
        self.edit.caret = self.edit.hit(&self.text, x, advances);
    }

    /// A left press while the field's state is the drag state (`0x10000`)
    /// also drops the anchor at the caret.
    pub fn press_anchor(&mut self, x: i32, advances: &[i16]) {
        self.press(x, advances);
        self.edit.anchor = self.edit.caret;
    }

    /// A key: [`text_field_key`] with the field's own limit.
    pub fn key(&self, key: u32, typed: bool, has_callback: bool) -> TextFieldKey {
        match text_field_key(self.style, key, typed, has_callback) {
            TextFieldKey::Edit { notify, .. } => TextFieldKey::Edit {
                max_len: self.max_len,
                notify,
            },
            TextFieldKey::Move => TextFieldKey::Move,
            other => other,
        }
    }

    /// The focus message: a field that has the focus selects all of it.
    pub fn focus(&mut self, focused: bool) {
        if focused {
            self.edit.select_all(self.text.len());
        }
    }
}

/// What a text field's message does. The message numbers and the owner
/// callbacks are the original's; the text edit unit's own editing stays
/// outside this model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFieldMessage {
    /// The back draw: the inner frame from [`text_border`] (if any).
    DrawBack {
        border: Option<(i32, i32, i32, i32)>,
    },
    /// The front draw: the text edit run.
    DrawFront { start: u8, end: u8 },
    /// The pointer query's word.
    Pointer(u32),
    /// The control redraws.
    Redraw,
    /// A drag or press placed the caret (the edit hit).
    Caret(u8),
    /// A left press in the edit state also dropped the anchor.
    Anchor(u8),
    /// Enter with an owner callback; `release` gives the focus up.
    Submit { release: bool },
    /// A typed byte the digits-only style drops.
    Dropped,
    /// A key the text edit unit takes with its limit.
    Edit { max_len: i32, notify: bool },
    /// A read-only field's move.
    Move,
    /// The field lost the focus with the reset bit: the run starts at 0 and
    /// the caret at the text's end.
    Reset,
    /// The set message stored new text.
    SetText,
    /// The Steam on-screen keyboard was asked for.
    OnScreenKeyboard,
}

impl TextField {
    /// Handles a message. `word` is the message's first argument and
    /// `flags` its second; `x` is the pointer's offset from the text's
    /// origin for drags and presses. The key's `typed` flag comes from the
    /// flags' bit 2, as the original's does.
    pub fn message(
        &mut self,
        kind: u32,
        word: u32,
        flags: u32,
        x: i32,
        advances: &[i16],
        has_callback: bool,
    ) -> TextFieldMessage {
        match kind {
            1 => TextFieldMessage::DrawBack {
                border: text_border((0, 0, 0, 0), self.style),
            },
            2 => TextFieldMessage::DrawFront {
                start: self.edit.start,
                end: self.edit.end,
            },
            0x10 => TextFieldMessage::Pointer(Self::CURSOR),
            0x20 | 0x50 => TextFieldMessage::Redraw,
            0x30 => TextFieldMessage::Caret(self.edit.hit(&self.text, x, advances)),
            0x40 => {
                if word & 7 == 1 && self.style & 0xf_0000 == 0x1_0000 {
                    self.press_anchor(x, advances);
                    TextFieldMessage::Anchor(self.edit.caret)
                } else {
                    TextFieldMessage::Redraw
                }
            }
            0xa0 => {
                let typed = flags & 2 != 0;
                match self.key(word, typed, has_callback) {
                    TextFieldKey::Submit { release } => TextFieldMessage::Submit { release },
                    TextFieldKey::Dropped => TextFieldMessage::Dropped,
                    TextFieldKey::Edit { max_len, notify } => {
                        TextFieldMessage::Edit { max_len, notify }
                    }
                    TextFieldKey::Move => TextFieldMessage::Move,
                }
            }
            0xb0 => {
                if word == 0 && self.style & 0x400 != 0 {
                    self.edit.reset(self.text.len());
                    TextFieldMessage::Reset
                } else {
                    TextFieldMessage::Redraw
                }
            }
            0xc0 => {
                if word == 0x100 {
                    TextFieldMessage::OnScreenKeyboard
                } else {
                    TextFieldMessage::Redraw
                }
            }
            0x200 => {
                let mut text = word.to_le_bytes().to_vec();
                while text.last() == Some(&0) {
                    text.pop();
                }
                text.truncate(usize::try_from(self.max_len).unwrap_or(0));
                self.text = text;
                self.edit.reset(self.text.len());
                TextFieldMessage::SetText
            }
            0x300 => TextFieldMessage::DrawFront {
                start: self.edit.start,
                end: self.edit.end,
            },
            0x400 => {
                self.edit.reset(self.text.len());
                TextFieldMessage::Reset
            }
            _ => TextFieldMessage::Redraw,
        }
    }
}

/// What a combo box's message does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboBoxMessage {
    DrawBack,
    DrawFront,
    Pointer(u32),
    Redraw,
    Caret(u8),
    Anchor(u8),
    /// The drop-down list opens (or its drop-down draw runs).
    Open,
    /// Enter with an owner callback: the box enters its edit state.
    Submit,
    /// The selected item was taken (see [`ComboBox::choose`]).
    Chosen,
    /// The focus loss put the selected item's text back.
    Restored,
    /// The set, refresh or reset messages.
    Refresh,
}

impl ComboBox {
    /// Handles a message. `word` is the message's first argument and `x`
    /// the pointer's offset for drags and presses.
    #[allow(clippy::too_many_arguments)]
    pub fn message(
        &mut self,
        kind: u32,
        word: u32,
        _flags: u32,
        x: i32,
        advances: &[i16],
        style: u32,
        state: u32,
    ) -> ComboBoxMessage {
        match kind {
            1 => ComboBoxMessage::DrawBack,
            2 => ComboBoxMessage::DrawFront,
            0x10 => ComboBoxMessage::Pointer(combo_cursor(style, word, (0, 0, 0, 0), 0, 0)),
            0x20 | 0x50 => ComboBoxMessage::Redraw,
            0x30 => ComboBoxMessage::Caret(self.edit.hit(&self.text, x, advances)),
            0x40 => {
                if word & 7 == 1 {
                    if state & 0xf_0000 == 0x2_0000 {
                        self.edit.anchor = self.edit.hit(&self.text, x, advances);
                        self.edit.caret = self.edit.anchor;
                        ComboBoxMessage::Anchor(self.edit.caret)
                    } else {
                        ComboBoxMessage::Open
                    }
                } else {
                    ComboBoxMessage::Redraw
                }
            }
            0xa0 => {
                if word == 0x0d {
                    ComboBoxMessage::Submit
                } else {
                    ComboBoxMessage::Redraw
                }
            }
            0xb0 => {
                if word == 0 && state & 0x400 != 0 {
                    if let Some(item) = self.item(self.selected) {
                        self.text = item.to_vec();
                    }
                    self.edit.reset(self.text.len());
                    ComboBoxMessage::Restored
                } else {
                    ComboBoxMessage::Redraw
                }
            }
            0xc0 => {
                if word == 0x100 {
                    ComboBoxMessage::Open
                } else {
                    ComboBoxMessage::Redraw
                }
            }
            0x200 => {
                if self.choose(word) {
                    ComboBoxMessage::Chosen
                } else {
                    ComboBoxMessage::Refresh
                }
            }
            0x300 => ComboBoxMessage::Refresh,
            0x400 => {
                self.edit.reset(self.text.len());
                ComboBoxMessage::Refresh
            }
            _ => ComboBoxMessage::Redraw,
        }
    }
}

/// The text field's inner frame, drawn after the skin when the style has
/// any of the `0xf0` frame bits: from one pixel in at the left and two at
/// the top to one in at the right and two at the bottom, as
/// `(left, top, right, bottom)`.
pub fn text_border(rect: (i32, i32, i32, i32), style: u32) -> Option<(i32, i32, i32, i32)> {
    if style & 0xf0 == 0 {
        return None;
    }
    let (x, y, w, h) = rect;
    let (y, h) = (y + 1, h - 2);
    Some((x + 1, y + 1, w + x - 1, h + y - 1))
}

/// A drop-down list's placement under its owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DropDownPlace {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// The list was cut short and scrolls (its `0x200` style bit).
    pub scrolls: bool,
}

/// Places a drop-down list under its owner (the owner's rectangle in
/// screen space). When the list's height `h` does not fit below, whole
/// `row`s that fit (inside `2·pad`) are counted below and above: with
/// fewer than six below and fewer than half again as many as above, it
/// opens above with the rows above, else it is cut to the rows below.
/// Either cut that is shorter than `h` scrolls. The half is the unsigned
/// shift of the row count, as the original's (a negative count halves to
/// a huge number).
pub fn drop_down_place(
    owner: (i32, i32, i32, i32),
    h: i32,
    pad: i32,
    row: i32,
    screen_h: i32,
) -> DropDownPlace {
    let (x, top, w, owner_h) = owner;
    let mut place = DropDownPlace {
        x,
        y: owner_h + top,
        w,
        h,
        scrolls: false,
    };
    let space = screen_h - place.y;
    if space < place.h && row != 0 {
        let below = (i64::from(space) - i64::from(pad << 1)) / i64::from(row);
        let above = ((i64::from(top) - i64::from(pad << 1)) / i64::from(row)) as i32;
        let half = (((below as u64) >> 1) as u32 & 0x7fff_ffff) as i64;
        let below32 = below as i32;
        if below32 < 6 && i64::from(below32) + half < i64::from(above) {
            let cut = above * row + pad * 2;
            if cut < place.h {
                place.scrolls = true;
                place.h = cut;
            }
            place.y = top - place.h;
        } else {
            let cut = below32 * row + pad * 2;
            if cut < place.h {
                place.scrolls = true;
            }
            place.h = cut;
        }
    }
    place
}

/// A combo box: its items, the 1-based selected one and its text limit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComboBox {
    pub items: Vec<Vec<u8>>,
    pub selected: i32,
    pub max_len: i32,
    pub text: Vec<u8>,
    /// The edit run and caret.
    pub edit: EditState,
}

impl ComboBox {
    /// A combo box as created: the selected item's text, a zero limit
    /// turned into 255.
    pub fn new(items: Vec<Vec<u8>>, selected: i32, max_len: i32) -> Self {
        let mut combo = Self {
            items,
            selected,
            max_len: if max_len == 0 { 255 } else { max_len },
            text: Vec::new(),
            edit: EditState::default(),
        };
        if let Some(item) = combo.item(selected) {
            combo.edit.end = item.len() as u8;
        }
        if let Some(item) = combo.item(selected) {
            combo.text = item.to_vec();
        }
        combo
    }

    fn item(&self, selected: i32) -> Option<&[u8]> {
        let at = usize::try_from(selected).ok()?.checked_sub(1)?;
        self.items.get(at).map(Vec::as_slice)
    }

    /// The set message: 0 (or an empty list) clears the text and reports
    /// nothing; otherwise the item, capped at the last, becomes the text,
    /// cut to the limit, and the owner is told (unless the caller asked for
    /// quiet). Returns whether it took an item.
    pub fn choose(&mut self, item: u32) -> bool {
        if item == 0 || self.items.is_empty() {
            self.text.clear();
            return false;
        }
        let count = self.items.len() as i64;
        self.selected = if i64::from(item) > count {
            count as i32
        } else {
            item as i32
        };
        let mut text = self.item(self.selected).unwrap_or(&[]).to_vec();
        text.truncate(usize::try_from(self.max_len).unwrap_or(0));
        self.text = text;
        true
    }
}

/// The combo box's pointer-shape query: an editable box shows the text
/// cursor left of its arrow (`x < left + w − (4·unit|1) − pad`), else none.
pub fn combo_cursor(style: u32, x: u32, rect: (i32, i32, i32, i32), pad: i32, unit: i32) -> u32 {
    let limit =
        i64::from(rect.0) + (i64::from(rect.2) - i64::from((4 * unit) | 1)) - i64::from(pad);
    if style & text_style::EDITABLE == text_style::EDITABLE && i64::from(x) < limit {
        0x802_0000
    } else {
        0
    }
}

/// What a combo box's left press does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComboPress {
    /// The box is in its edit state (`0x20000`): the press drops the
    /// anchor at the caret.
    Edit,
    /// Otherwise the press opens the drop-down list.
    Open,
}

/// The combo box's left-press rule, from its state bits (`0xf0000`).
pub fn combo_press(state: u32) -> ComboPress {
    if state & 0xf_0000 == 0x2_0000 {
        ComboPress::Edit
    } else {
        ComboPress::Open
    }
}

/// The combo box's arrow origin: `(4·unit|1) − unit/2 + 1` in from the
/// right edge and `(unit + 1) − (unit & 1)` down; no arrow with the style's
/// `0x8000000` bit.
pub fn combo_arrow(rect: (i32, i32, i32, i32), unit: i32, style: u32) -> Option<(i32, i32)> {
    if style & 0x800_0000 != 0 {
        return None;
    }
    let width = (((4 * unit) | 1) - (unit >> 1)) + 1;
    let height = (unit + 1) - (unit & 1);
    Some((rect.2 + rect.0 - width, height + rect.1))
}

/// What a colour swatch's left press does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwatchPress {
    Ignored,
    /// The owner's callback (result 1).
    Callback,
    /// A colour drag starts (result `0x105`).
    Drag(ColourDrag),
}

/// The colour drag's payload code.
pub const COLOUR_DRAG_CODE: u32 = 0xcd0c;

/// The colour a swatch drags: the colour, the press's offset into the
/// swatch and the swatch's inner size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColourDrag {
    pub colour: u32,
    pub offset: (i32, i32),
    pub size: (i32, i32),
}

/// The swatch's left press: with the `0x4000` modifier bit the colour is
/// dragged; else with `0x2000` the owner is called; else nothing.
pub fn swatch_press(
    flags: u32,
    colour: u32,
    press: (i32, i32),
    rect: (i32, i32, i32, i32),
    pad: i32,
) -> SwatchPress {
    if flags & 0x4000 != 0 {
        SwatchPress::Drag(ColourDrag {
            colour,
            offset: (press.0 - rect.0, press.1 - rect.1),
            size: (rect.2 - pad * 2, rect.3 - pad * 2),
        })
    } else if flags & 0x2000 != 0 {
        SwatchPress::Callback
    } else {
        SwatchPress::Ignored
    }
}

/// Whether a swatch accepts a dragged payload: the colour code only.
pub fn swatch_accepts(code: u32) -> bool {
    code == COLOUR_DRAG_CODE
}

/// The dragged colour's rectangle under the cursor, drawn at alpha `0xa0`.
pub fn colour_drag_rect(cursor: (i32, i32), drag: &ColourDrag) -> (i32, i32, i32, i32) {
    (
        cursor.0 - drag.offset.0,
        cursor.1 - drag.offset.1,
        drag.size.0,
        drag.size.1,
    )
}

/// A numeric spinner: its value, range and step, in float or (with the
/// integer style) 32-bit integer form.
#[derive(Debug, Clone, PartialEq)]
pub struct Spinner {
    pub value: f32,
    pub min: f32,
    pub max: f32,
    pub step: f32,
    pub integer: bool,
    /// The typed text no longer shows the value (the `0x40000000` bit).
    pub dirty: bool,
    /// The decimals the value formats with (the record's `0x58` word).
    pub decimals: i32,
    /// The drawn text.
    pub text: Vec<u8>,
}

impl Spinner {
    /// The creation hook: a zero step becomes 1 (1.0, or the integer 1),
    /// a zero maximum becomes 100.0 (written as a float even for an integer
    /// spinner, whose maximum then reads as `0x42c80000`), and a float
    /// value under the minimum rises to it.
    pub fn init(&mut self) {
        if self.step == 0.0 {
            self.step = if self.integer { f32::from_bits(1) } else { 1.0 };
        }
        if self.max == 0.0 {
            self.max = 100.0;
        }
        if !self.integer && self.value < self.min {
            self.value = self.min;
        }
        self.text = self.formatted();
    }

    /// The text the value formats to: `decimals` places for a float
    /// spinner, or the 32-bit integer bits for an integer spinner. The
    /// original's number formatter is outside the pack; Rust's formatting
    /// stands in for it.
    pub fn formatted(&self) -> Vec<u8> {
        let text = if self.integer {
            (self.value.to_bits() as i32).to_string()
        } else {
            format!("{:.*}", self.decimals.max(0) as usize, self.value)
        };
        text.into_bytes()
    }

    /// The drawn text, reformatted from the value.
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.formatted()).into_owned()
    }

    /// The set message. The message carries one 32-bit word: an integer
    /// spinner stores it as is when its bits differ. A float spinner reads
    /// it as a float when `float_word` (the message's flag bit 0), else as
    /// an integer converted to a float; clamps below the minimum, else
    /// above the maximum, with the original's ordered compares (a NaN
    /// passes both); and lands only when it moves by more than 1e-8 (the
    /// single-precision difference compared in double). Returns whether
    /// the value changed; a caller's reset of the edit state is separate.
    pub fn set_value(&mut self, word: u32, float_word: bool) -> bool {
        if self.integer {
            if word != self.value.to_bits() {
                self.value = f32::from_bits(word);
                self.text = self.formatted();
                return true;
            }
            return false;
        }
        let mut value = if float_word {
            f32::from_bits(word)
        } else {
            word as i32 as f32
        };
        if value < self.min {
            value = self.min;
        } else if value > self.max {
            value = self.max;
        }
        if f64::from((value - self.value).abs()) > 1e-8 {
            self.value = value;
            self.text = self.formatted();
            return true;
        }
        false
    }

    /// Commits a typed float: below the minimum takes the minimum, above
    /// the maximum the maximum, NaN neither (the ordered compares skip
    /// it). Unless `reformat`, a value the clamp moved marks the text
    /// dirty. The value changes when it moves by more than 1e-8 (in double
    /// precision), so a NaN never lands. Returns whether it changed.
    pub fn commit(&mut self, typed: f32, reformat: bool) -> bool {
        let mut value = typed;
        if value < self.min {
            value = self.min;
        } else if value > self.max {
            value = self.max;
        }
        if !reformat && value != typed {
            self.dirty = true;
        }
        if f64::from((value - self.value).abs()) > 1e-8 {
            self.value = value;
            return true;
        }
        false
    }
}

/// Where a spinner's left press lands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerPress {
    /// In the text with the `0x8000` modifier: select all.
    SelectAll,
    /// In the text: the anchor drops at the caret (result `0x15`).
    Anchor,
    /// The upper half of the buttons steps up (result `0x1f`).
    Up,
    /// The lower half steps down.
    Down,
}

/// The spinner's press rule: left of the button column is the text; the
/// buttons split at half the height (an unsigned shift).
pub fn spinner_press(
    rect: (i32, i32, i32, i32),
    button: i32,
    x: i32,
    y: i32,
    flags: u32,
) -> SpinnerPress {
    let (left, top, w, h) = rect;
    if i64::from(x) < i64::from(left) + i64::from(w) - i64::from(button) {
        if flags & 0x8000 != 0 {
            SpinnerPress::SelectAll
        } else {
            SpinnerPress::Anchor
        }
    } else if i64::from(y) - i64::from(top) < i64::from(((h as u32) >> 1) as i32) {
        SpinnerPress::Up
    } else {
        SpinnerPress::Down
    }
}

/// The spinner's pointer-shape query: the text cursor over the text (that
/// takes the focus when editable), else the up or down button's.
/// The message's `x` and `y` are unsigned, as the original compares them.
pub fn spinner_cursor(rect: (i32, i32, i32, i32), button: i32, style: u32, x: u32, y: u32) -> u32 {
    let (left, top, w, h) = rect;
    if i64::from(x) < i64::from(left) + i64::from(w) - i64::from(button) {
        if style & text_style::EDITABLE == text_style::EDITABLE {
            0x801_0000
        } else {
            0x1_0000
        }
    } else if i64::from(y) < i64::from(((h as u32) >> 1) as i32) + i64::from(top) {
        0x2_0000
    } else {
        0x4_0000
    }
}

/// What a key does in a spinner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpinnerKey {
    /// A typed byte the filter drops.
    Dropped,
    /// A typed dot over the dot at the anchor steps the anchor and caret
    /// past it instead.
    SkipDot,
    /// Up (`0x26`) adds a step, Down (`0x28`) takes one.
    Step(bool),
    /// Enter reformats a dirty text, commits and selects all.
    Commit,
    /// Anything else goes to the text edit unit (255 bytes), marks the
    /// text edited and, with the style's `0x2000000` bit, commits at once.
    Edit {
        commit: bool,
    },
    /// Ctrl+V pastes (32 bytes at most), Ctrl+C copies.
    Paste,
    Copy,
    /// Other control keys.
    Ignored,
}

/// The spinner's key rule. `flags` holds the message's modifier bits:
/// `0x40` Ctrl, `0x80` Alt, `2` a typed byte. A typed byte passes only as
/// a digit, a dot, or a minus while the anchor or the caret is at 0.
pub fn spinner_key(text: &[u8], edit: EditState, style: u32, key: u32, flags: u32) -> SpinnerKey {
    let typed = flags & 2 != 0;
    if flags & 0x40 != 0 && !typed {
        return match key {
            0x56 => SpinnerKey::Paste,
            0x43 => SpinnerKey::Copy,
            _ => SpinnerKey::Ignored,
        };
    }
    if flags & 0xc0 != 0 {
        return SpinnerKey::Ignored;
    }
    if typed {
        match key as u8 {
            b'.' => {
                let at = usize::from(edit.anchor);
                if at < text.len() && text[at] == b'.' {
                    return SpinnerKey::SkipDot;
                }
            }
            b'-' => {
                if edit.anchor != 0 && edit.caret != 0 {
                    return SpinnerKey::Dropped;
                }
            }
            byte if !byte.is_ascii_digit() => return SpinnerKey::Dropped,
            _ => {}
        }
    }
    match key {
        0x26 => SpinnerKey::Step(true),
        0x28 => SpinnerKey::Step(false),
        0x0d => SpinnerKey::Commit,
        _ => SpinnerKey::Edit {
            commit: style & 0x200_0000 != 0,
        },
    }
}

/// The spinner's drag step in pixels: `96·unit / ((max − min) / step)`
/// rounded half to even (in 64 bits, keeping the low 32), at most
/// `8·unit`, then at least `unit / 2`; a NaN or infinite quotient rounds to
/// the indefinite integer, whose low half 0 lands on the floor.
pub fn spinner_repeat_step(unit: i32, min: f32, max: f32, step: f32) -> i32 {
    let mut pixels = round_to_i64((96 * unit) as f32 / ((max - min) / step)) as i32;
    if unit << 3 < pixels {
        pixels = unit << 3;
    }
    if pixels < unit >> 1 {
        pixels = unit >> 1;
    }
    pixels
}

/// The spinner's drawn parts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpinnerButtons {
    /// The field's frame: left, top, right, bottom.
    pub field: (i32, i32, i32, i32),
    /// The up and down buttons: left, top, width, height.
    pub up: (i32, i32, i32, i32),
    pub down: (i32, i32, i32, i32),
    /// The arrows' origins and width (`(unit − 1)·2 | 1`).
    pub up_arrow: (i32, i32),
    pub down_arrow: (i32, i32),
    pub arrow_width: i32,
}

/// Lays a spinner's draw out: the field up to the button column, two
/// buttons of half the height plus an overlap (1, or 2 from a unit of 5)
/// starting that overlap above the top, and the arrows `(unit + 1) / 2`
/// into each.
pub fn spinner_buttons(rect: (i32, i32, i32, i32), button: i32, unit: i32) -> SpinnerButtons {
    let (x, y, w, h) = rect;
    let overlap = if unit < 5 { 1 } else { 2 };
    let half = (((h as u32) >> 1) as i32) + overlap;
    let column = x + w - button;
    let inset = (unit + 1) >> 1;
    SpinnerButtons {
        field: (x, y, column, h + y),
        up: (column, y - overlap, button, half),
        down: (column, (y + half) - overlap, button, half),
        up_arrow: (column, y + inset),
        down_arrow: (column, y + half + inset),
        arrow_width: ((unit - 1) * 2) | 1,
    }
}

/// Whether the swatch's draw message draws its face: state bit
/// `0x1000000`.
pub fn swatch_draw(state: u32) -> bool {
    state & 0x100_0000 != 0
}

/// The separator lets the pointer through (`0x800000`).
pub const SEPARATOR_POINTER: u32 = PASS_THROUGH;

/// The image's two corners through the parent's origin: the top-left and
/// the bottom-right, as the corner routine hands them to the parent's
/// conversion.
pub fn image_corners(rect: (i32, i32, i32, i32), origin: (i32, i32)) -> ((i32, i32), (i32, i32)) {
    (
        (rect.0 + origin.0, rect.1 + origin.1),
        (rect.0 + rect.2 + origin.0, rect.1 + rect.3 + origin.1),
    )
}

/// A progress bar's value (a fraction, shown as a percentage).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressBar {
    pub value: f32,
}

impl ProgressBar {
    /// The set message: a negative value becomes 0 (−0 and NaN stay, by
    /// the original's `≤ 0 and ≠ 0` test); the owner is told every time.
    pub fn set(&mut self, value: f32) {
        // The original's `≤ 0 and ≠ 0` is a plain `< 0`.
        self.value = if value < 0.0 { 0.0 } else { value };
    }

    /// The caption: the value times a hundred with one decimal, then `%`.
    /// The number formatter is outside the evidence; Rust's fixed-point
    /// formatting stands in for it.
    pub fn text(&self) -> String {
        format!("{:.1}%", self.value * 100.0)
    }

    /// The filled width: `w × value`, rounded half to even.
    pub fn fill(&self, w: i32) -> i32 {
        round_to_i64(w as f32 * self.value) as i32
    }

    /// The caption draws only on a bar taller than five units.
    pub fn shows_text(h: i32, unit: i32) -> bool {
        i64::from(unit) * 5 < i64::from(h)
    }

    /// The creation helper's text: the value as a number (`decimals`
    /// places) then `%`.
    pub fn text_with(&self, decimals: i32) -> String {
        format!("{:.*}%", decimals.max(0) as usize, self.value * 100.0)
    }
}

/// A progress bar's drawn rectangles, each left, top, right, bottom.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProgressRects {
    /// The empty part (drawn at 0.4 alpha), when the fill is short of the
    /// width.
    pub rest: Option<(i32, i32, i32, i32)>,
    /// The filled bar and its highlight, when the fill is positive.
    pub bar: Option<(i32, i32, i32, i32)>,
    pub highlight: Option<(i32, i32, i32, i32)>,
}

/// Lays a progress bar's draw out from its fill ([`ProgressBar::fill`]).
pub fn progress_rects(rect: (i32, i32, i32, i32), pad: i32, fill: i32) -> ProgressRects {
    let (x, y, w, h) = rect;
    ProgressRects {
        rest: (fill < w).then_some((fill + x - 1, y + 1, w + x - 1, h + y - 1)),
        bar: (fill > 0).then_some((
            pad + x + 2,
            pad + y + 2,
            (x + fill) - pad - 2,
            (h + y) - pad - 2,
        )),
        highlight: (fill > 0).then_some((
            pad + x + 1,
            pad + y + 1,
            (x + fill) - pad - 1,
            (y + h) - pad - 1,
        )),
    }
}

/// The separator's text gap: the font's character width (its `0x18`
/// metric) times the text's length.
pub fn separator_gap(char_width: i32, text_len: i32) -> i32 {
    char_width.wrapping_mul(text_len)
}

/// The separator's line rectangles (left, top, width, height). With no
/// text the line spans the control, raised by half its height (an unsigned
/// shift), or with a positive line count `lines` lowered by half of
/// `unit × lines`. With text, a right-aligned (`0x2000`) separator keeps
/// only the part right of the text and a unit; any other leaves the text
/// in the middle, each side `(w − gap) / 2 − 1`.
pub fn separator_lines(
    rect: (i32, i32, i32, i32),
    gap: i32,
    style: u32,
    unit: i32,
    lines: i32,
) -> Vec<(i32, i32, i32, i32)> {
    let (x, y, w, h) = rect;
    if gap < 1 {
        let top = if lines < 1 {
            y - ((h as u32) >> 1) as i32
        } else {
            (((i64::from(unit) * i64::from(lines)) as u64 >> 1) as i32) + y
        };
        return vec![(x, top, w, h)];
    }
    if style & 0x7000 == 0x2000 {
        let width = (w - gap) - unit;
        return vec![((x + w) - width, y, width, h)];
    }
    let width = ((((i64::from(w) - i64::from(gap)) as u64) >> 1) as i32) - 1;
    vec![(x, y, width, h), ((x + w) - width, y, width, h)]
}

/// The separator, image and caption pointer query result that lets
/// presses through to what is under them.
pub const PASS_THROUGH: u32 = 0x80_0000;

/// An image view's picked point: across, and up from the bottom.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePick {
    pub u: f32,
    pub v: f32,
}

/// The point a drag over an image view picks. `x` and `y` are the
/// message's coordinates, from which only the pad is taken (the original
/// does not take the control's origin off); each fraction is divided in
/// double precision by the size less one pad, `v` turned to count up, and
/// clamped below 0 and above 1. A NaN (a zero extent with a zero offset)
/// passes both clamps.
pub fn image_pick(w: i32, h: i32, pad: i32, x: i32, y: i32) -> ImagePick {
    // `f32::clamp` leaves a NaN alone, as the unordered compares do.
    let clamp = |value: f32| value.clamp(0.0, 1.0);
    let u = (f64::from(x - pad) / f64::from(w - pad)) as f32;
    let v = (1.0 - f64::from(y - pad) / f64::from(h - pad)) as f32;
    ImagePick {
        u: clamp(u),
        v: clamp(v),
    }
}

/// The image's inset rectangle (the pad in from each side), where the
/// image and its fading shade draw.
pub fn image_rect(rect: (i32, i32, i32, i32), pad: i32) -> (i32, i32, i32, i32) {
    (
        rect.0 + pad,
        pad + rect.1,
        rect.2 - pad * 2,
        rect.3 - pad * 2,
    )
}

/// Where a texture view draws its texture's frame: centred by the
/// texture's cell size, each half truncated toward zero.
pub fn texture_origin(rect: (i32, i32, i32, i32), cell: i32) -> (i32, i32) {
    (rect.0 + (rect.2 - cell) / 2, rect.1 + (rect.3 - cell) / 2)
}

/// A caption's draw and pointer rules.
pub mod caption {
    /// The skin draws behind the caption only with a frame bit (`0xf0`).
    pub fn draws_back(style: u32) -> bool {
        style & 0xf0 != 0
    }

    /// The text wraps over lines with either `0x3000` bit.
    pub fn wraps(style: u32) -> bool {
        style & 0x3000 != 0
    }

    /// A caption without a callback lets the pointer through.
    pub fn pointer(has_callback: bool) -> u32 {
        if has_callback { 0 } else { super::PASS_THROUGH }
    }

    /// An enabled caption calls its owner on the `0x2001` left press.
    pub fn press(disabled: bool, flags: u32, has_callback: bool) -> bool {
        !disabled && flags == 0x2001 && has_callback
    }
}

/// A scroll bar's thumb: its width (`4·unit + 1`) and its offset from
/// `x + pad`, `(w − 2·pad − width) × value` rounded half to even.
pub fn scroll_thumb(unit: i32, pad: i32, w: i32, value: f32) -> (i32, i32) {
    let width = unit * 4 + 1;
    let offset = round_to_i64(((w - (pad << 1)) - width) as f32 * value) as i32;
    (width, offset)
}

/// The scroll bar's value at `x`: `(x − left − pad − width/2) / travel` in
/// double precision, clamped below 0 and above 1; a NaN passes both.
pub fn scroll_value(unit: i32, left: i32, pad: i32, w: i32, x: i32) -> f32 {
    let width = (unit * 4 + 1) as u32;
    let num = i64::from(x - left) - i64::from(pad) - i64::from((width >> 1) as i32);
    let den = (i64::from(w) - i64::from(pad << 1)) - i64::from(width as i32);
    // `f32::clamp` leaves a NaN alone, as the unordered compares do.
    ((num as f64 / den as f64) as f32).clamp(0.0, 1.0)
}

/// The scroll bar's three captions: its own text at the left, its suffix
/// at the right (alignment `0x4000`) and the value, `(max − min) × value +
/// min` with no decimals, in the middle (`0x3000`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollLabels {
    pub text: Vec<u8>,
    pub suffix: Vec<u8>,
    pub value: String,
}

/// The scroll bar's captions. The number formatter is outside the
/// evidence; Rust's fixed-point formatting stands in for it.
pub fn scroll_labels(text: &[u8], suffix: &[u8], value: f32, min: f32, max: f32) -> ScrollLabels {
    ScrollLabels {
        text: text.to_vec(),
        suffix: suffix.to_vec(),
        value: format!("{:.0}", (max - min) * value + min),
    }
}

/// The scroll bar's and trackbar's left press: an enabled control re-sends
/// the press as a drag (`0x30`) and adds `5` to its result.
pub fn drag_press(disabled: bool, flags: u32) -> bool {
    !disabled && flags & 7 == 1
}

/// The spinner's drag state and the values it remembers when a drag starts.
///
/// The state word's `0x0f00_0000` field holds the drag's phase: 0 is idle
/// (a press places the caret), `0x0100_0000`/`0x0200_0000` are the up/down
/// hot halves, `0x0300_0000` is a live drag and `0x0306_0000` is a drag
/// that has captured the mouse.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpinnerDrag {
    pub state: u32,
    /// The cursor's y when the drag began.
    pub origin_y: i32,
    /// The value when the drag began.
    pub origin_value: f32,
    /// The integer value's bits when the drag began (the integer path
    /// remembers the raw bits).
    pub origin_bits: u32,
}

impl Default for SpinnerDrag {
    fn default() -> Self {
        Self {
            state: 0,
            origin_y: 0,
            origin_value: 0.0,
            origin_bits: 0,
        }
    }
}

/// What a spinner's drag message does.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpinnerDragAction {
    /// Nothing.
    None,
    /// A press left of the button column places the caret.
    Text,
    /// Commit a float value through the control's set message.
    SetFloat(f32),
    /// Commit an integer value (its bits) through the set message, flagged
    /// as an integer.
    SetInt(u32),
    /// The pointer passed the unit's travel: capture the mouse and become
    /// the live drag. The two bounds are the original's clamp limits; its
    /// two other region fields hold the sentinel `0x8000_0001`.
    Capture { low: i32, high: i32 },
    /// The hot half changed (the control redraws).
    Hot,
}

/// The spinner's drag message. `cursor` is the GUI's cursor position and
/// `press` the message's. The state starts idle; a press in the text
/// places the caret. Once the text's drag state is on, moving past the
/// unit's travel captures the mouse over the value's whole reach, and
/// further moves step the value by `spinner_repeat_step` pixels each.
#[allow(clippy::too_many_arguments)]
pub fn spinner_drag(
    drag: &mut SpinnerDrag,
    spinner: &Spinner,
    rect: (i32, i32, i32, i32),
    _button: i32,
    unit: i32,
    style: u32,
    press: (i32, i32),
    cursor: (i32, i32),
) -> SpinnerDragAction {
    let pixels = spinner_repeat_step(unit, spinner.min, spinner.max, spinner.step);
    if drag.state & 0x0f00_0000 == 0 {
        return SpinnerDragAction::Text;
    }
    if drag.state & 0x0300_0000 == 0x0300_0000 {
        if spinner.integer {
            let moves = cvt_div(drag.origin_y - cursor.1, pixels);
            let step = spinner.step.to_bits();
            let value = (drag.origin_bits as u64)
                .wrapping_add((moves as i64 as u64).wrapping_mul(u64::from(step)));
            return SpinnerDragAction::SetInt(value as u32);
        }
        let moves = cvt_div(drag.origin_y - cursor.1, pixels) as f32;
        return SpinnerDragAction::SetFloat(moves * spinner.step + drag.origin_value);
    }
    // The text drag: a travel past the unit captures the mouse over the
    // value's whole reach.
    let travel = (cursor.1 - press.1).unsigned_abs() as i32;
    if travel > unit {
        let (low, high) = if spinner.integer {
            let max = spinner.max.to_bits();
            let min = spinner.min.to_bits();
            let step = spinner.step.to_bits();
            let to_left = if max <= 0x186a0 {
                cursor.1 - cvt_div_u(pixels, max.wrapping_sub(drag.origin_bits), step) - pixels
            } else {
                i32::MIN
            };
            let to_right = if min <= 0x186a0 {
                cursor.1 + pixels + cvt_div_u(pixels, drag.origin_bits.wrapping_sub(min), step)
            } else {
                i32::MAX
            };
            (to_left, to_right)
        } else {
            let to_left = cursor.1
                - (pixels as f32 * ((spinner.max - spinner.value) / spinner.step)).round_ties_even()
                    as i32
                - pixels;
            let to_right = cursor.1
                + (pixels as f32 * ((spinner.value - spinner.min) / spinner.step)).round_ties_even()
                    as i32
                + pixels;
            (to_left, to_right)
        };
        drag.state |= 0x0306_0000;
        drag.origin_y = cursor.1;
        drag.origin_value = spinner.value;
        drag.origin_bits = spinner.value.to_bits();
        return SpinnerDragAction::Capture { low, high };
    }
    // The hot halves: the up half marks 0x0100_0000 when the style wants
    // a hot button, the lower half 0x0200_0000. Other bits are kept.
    let (top, h) = (rect.1, rect.3);
    let mut bit = 0;
    if cursor.1 >= top && cursor.1 < top + (((h as u32) >> 1) as i32) {
        if style & 0x0400_0000 != 0 {
            bit = 0x0100_0000;
        }
    } else if style & 0x0800_0000 != 0 {
        bit = 0x0200_0000;
    }
    let state = (drag.state & 0xfcff_ffff) | bit;
    if state != drag.state {
        drag.state = state;
        return SpinnerDragAction::Hot;
    }
    SpinnerDragAction::None
}

/// `round(a / b)` on the i32 operands, half to even; the quotient is taken
/// in double precision as the original's conversion does.
fn cvt_div(a: i32, b: i32) -> i32 {
    let quotient = f64::from(a) / f64::from(b);
    let rounded = quotient.round_ties_even();
    if rounded.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// `round(pixels × (numerator / step))` on the unsigned difference of two
/// integer values, as the integer drag reads them.
fn cvt_div_u(pixels: i32, numerator: u32, step: u32) -> i32 {
    let quotient = f64::from(pixels) * (f64::from(numerator) / f64::from(step));
    let rounded = quotient.round_ties_even();
    if rounded.is_nan() || !(-2_147_483_648.0..2_147_483_648.0).contains(&rounded) {
        i32::MIN
    } else {
        rounded as i32
    }
}

/// A spinner's drawn parts, including each button's press and hot state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpinnerDraw {
    pub buttons: SpinnerButtons,
    /// The field's two-ink face: its inks are the control's panel pair.
    pub field_face: bool,
    /// The inner one-pixel inset rectangle.
    pub inner: (i32, i32, i32, i32),
    /// The up button is pressed (`0x0100_0000`) and hot (`0x10`).
    pub up_pressed: bool,
    pub down_pressed: bool,
    pub hot: bool,
    /// The arrows' state words: 1 for the pressed style bit `0x20000`,
    /// 0 otherwise, or `0xffff_ffff` while the control is hot.
    pub up_arrow_state: i32,
    pub down_arrow_state: i32,
}

/// Lays out the spinner's draw exactly as its draw routine does: a field to
/// the button column (its inner rectangle one pixel in), two half-height
/// buttons with the 1- or 2-pixel overlap, and the arrows at
/// `(unit + 1) / 2` into each. The up button's pressed bit is `0x1000000`,
/// the down button's `0x2000000`; both take the hot bit `0x10`. Without the
/// hot bit the up arrow's state is 1 on style `0x20000` and the down
/// arrow's 1 on style `0x40000`; with the hot bit both are `0xffff_ffff`.
pub fn spinner_draw(
    rect: (i32, i32, i32, i32),
    button: i32,
    unit: i32,
    style: u32,
    state: u32,
) -> SpinnerDraw {
    let (x, y, w, h) = rect;
    let buttons = spinner_buttons(rect, button, unit);
    let hot = state & 0x10 != 0;
    let up_arrow_state = if hot {
        -1
    } else if style & 0x0002_0000 != 0 {
        1
    } else {
        0
    };
    let down_arrow_state = if hot {
        -1
    } else if style & 0x0004_0000 != 0 {
        1
    } else {
        0
    };
    SpinnerDraw {
        buttons,
        field_face: true,
        inner: (x + 1, y + 1, (x + w - button) - 1, (y + h) - 1),
        up_pressed: state & 0x0100_0000 != 0,
        down_pressed: state & 0x0200_0000 != 0,
        hot,
        up_arrow_state,
        down_arrow_state,
    }
}

/// A progress bar's draw: its empty part (0.4 alpha), its four-stop bar and
/// its one-ink highlight, each a rectangle in screen space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProgressDraw {
    pub rest: Option<(i32, i32, i32, i32)>,
    pub bar: Option<(i32, i32, i32, i32)>,
    pub highlight: Option<(i32, i32, i32, i32)>,
    /// The single-precision alpha the skin's rectangle call takes.
    pub rest_alpha: f32,
}

/// Lays out the progress bar's draw from its fill. The empty part is drawn
/// at alpha 0.4; the bar is four stops and the highlight one ink. The
/// rectangles are the ones [`progress_rects`] already computes.
pub fn progress_draw(rect: (i32, i32, i32, i32), pad: i32, value: f32) -> ProgressDraw {
    let fill = round_to_i64((rect.2 as f32) * value) as i32;
    let rects = progress_rects(rect, pad, fill);
    ProgressDraw {
        rest: rects.rest,
        bar: rects.bar,
        highlight: rects.highlight,
        rest_alpha: 0.4,
    }
}

/// A separator's line draw: the line rectangles and the four inks in the
/// order the skin takes them (top, left, right, bottom). Style bit 0x40
/// swaps the two ink sets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeparatorDraw {
    pub lines: Vec<(i32, i32, i32, i32)>,
    /// The four skin colour slots.
    pub inks: [u8; 4],
}

/// Lays out the separator's lines and inks. The line rects are
/// [`separator_lines`]; the normal inks are slots `0x9c, 0xa0, 0xac, 0xb4`,
/// and style bit 0x40 reads `0xb8, 0xb0, 0xa0, 0x9c`.
pub fn separator_draw(
    rect: (i32, i32, i32, i32),
    gap: i32,
    style: u32,
    unit: i32,
    lines: i32,
) -> SeparatorDraw {
    let inks = if style & 0x40 != 0 {
        [0xb8, 0xb0, 0xa0, 0x9c]
    } else {
        [0x9c, 0xa0, 0xac, 0xb4]
    };
    SeparatorDraw {
        lines: separator_lines(rect, gap, style, unit, lines),
        inks,
    }
}

/// The unit text inset the unit-change hook caches: `3·unit | 1`, the
/// width the spinner's button column and the separator's text gap use. The
/// hook runs ahead of the previous one, which it then calls.
pub fn text_inset(unit: i32) -> i32 {
    (3 * unit) | 1
}

/// A text field's draw-through plan: the inner frame, the label's origin
/// and the run the text edit unit draws.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextFieldDraw {
    /// The inner frame when the style has any `0xf0` bit.
    pub border: Option<(i32, i32, i32, i32)>,
    /// The aligned text origin.
    pub origin: LabelAt,
    /// The drawn run's first byte and one past its last.
    pub start: u8,
    pub end: u8,
}

/// Lays out a text field's draw-through: its inner frame, the label origin
/// from the style and the measured width, and the run its edit state keeps.
#[allow(clippy::too_many_arguments)]
pub fn text_field_draw(
    rect: (i32, i32, i32, i32),
    style: u32,
    edit: EditState,
    text_width: i32,
    line_h: i32,
    base: i32,
    pad: i32,
    unit: i32,
) -> TextFieldDraw {
    let (x, y, w, h) = rect;
    let avail = text_span_a(w, pad, unit);
    TextFieldDraw {
        border: text_border(rect, style),
        origin: align_label(x, y, h, pad, unit, text_width, avail, line_h, base, style),
        start: edit.start,
        end: edit.end,
    }
}

/// A scroll bar's three caption runs: their origins and alignment styles.
/// The text keeps its origin, the suffix takes `0x4000` and the value
/// `0x3000`; the style word's own alignment bits are cleared first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollCaptionRun {
    pub x: i32,
    pub y: i32,
    pub style: u32,
}

/// Places the scroll bar's three captions. Each is measured by the caller;
/// the text and suffix keep the control's style with the alignment bits
/// cleared, and the value is centred.
#[allow(clippy::too_many_arguments)]
pub fn scroll_caption_runs(
    rect: (i32, i32, i32, i32),
    pad: i32,
    unit: i32,
    text_width: i32,
    suffix_width: i32,
    value_width: i32,
    line_h: i32,
    base: i32,
    style: u32,
) -> [ScrollCaptionRun; 3] {
    let (x, y, w, h) = rect;
    let plain = style & !0x7000;
    let avail = text_span_a(w, pad, unit);
    let text = align_label(x, y, h, pad, unit, text_width, avail, line_h, base, plain);
    let suffix = align_label(
        x,
        y,
        h,
        pad,
        unit,
        suffix_width,
        avail,
        line_h,
        base,
        plain | 0x4000,
    );
    let value = align_label(
        x,
        y,
        h,
        pad,
        unit,
        value_width,
        avail,
        line_h,
        base,
        plain | 0x3000,
    );
    [
        ScrollCaptionRun {
            x: text.x,
            y: text.y,
            style: plain,
        },
        ScrollCaptionRun {
            x: suffix.x,
            y: suffix.y,
            style: plain | 0x4000,
        },
        ScrollCaptionRun {
            x: value.x,
            y: value.y,
            style: plain | 0x3000,
        },
    ]
}

/// A combo box's draw: its arrow (when shown) and which text path runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ComboDraw {
    pub arrow: Option<(i32, i32)>,
    /// The arrow's colour slot: `0xd4` when the style bit 0x10 is set,
    /// `0xe0` otherwise.
    pub arrow_ink: u8,
    /// The arrow was drawn through the texture path (`0xe0` method) rather
    /// than the flat one (`0xd0`).
    pub arrow_textured: bool,
    /// The text goes through the text-edit draw (edit state `0x10000`)
    /// rather than the plain draw-through.
    pub editing: bool,
    /// The text's colour slot: `0xe4` for the plain path.
    pub text_ink: u8,
}

/// Lays out a combo box's draw from its style and state words.
pub fn combo_draw(rect: (i32, i32, i32, i32), unit: i32, style: u32, state: u32) -> ComboDraw {
    let arrow = combo_arrow(rect, unit, style);
    let editing = state & 0xf_0000 == 0x1_0000;
    ComboDraw {
        arrow,
        arrow_ink: if state & 0x10 != 0 { 0xd4 } else { 0xe0 },
        arrow_textured: editing,
        editing,
        text_ink: 0xe4,
    }
}

/// A trackbar's draw: its track and thumb rectangles and the knob's travel.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackDraw {
    pub track: (i32, i32, i32, i32),
    /// The single-precision alpha the skin's rectangle call takes.
    pub track_alpha: f32,
    pub thumb: (i32, i32, i32, i32),
    pub pressed: bool,
    /// The knob's offset from `x + pad`.
    pub offset: i32,
}

/// Lays out a trackbar's draw. The track is inset by two pixels all round
/// and drawn at alpha 0.6; the knob is `knob` wide at
/// `x + pad + rint((w − 2·pad − knob) × value)`, one pad inside the height.
pub fn track_draw(
    rect: (i32, i32, i32, i32),
    pad: i32,
    knob: i32,
    value: f32,
    state: u32,
) -> TrackDraw {
    let (x, y, w, h) = rect;
    let travel = (w - (pad << 1)) - knob;
    let offset = round_to_i64(travel as f32 * value) as i32;
    TrackDraw {
        track: (x + 2, y + 2, (x + w) - 2, (y + h) - 2),
        track_alpha: 0.6,
        thumb: (x + pad + offset, y + pad, knob, (h - (pad << 1)).max(0)),
        pressed: state & 0x10 != 0,
        offset,
    }
}

/// A scroll bar's draw: the track, the thumb and the value offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollDraw {
    pub track: (i32, i32, i32, i32),
    /// The single-precision alpha the skin's rectangle call takes.
    pub track_alpha: f32,
    pub thumb: (i32, i32, i32, i32),
    pub pressed: bool,
}

/// Lays out the scroll bar's track and thumb. The track is inset by two
/// pixels; the thumb is `4·unit + 1` wide at `x + pad + rint(spare ×
/// value)`, one pad inside the height.
pub fn scroll_draw(
    rect: (i32, i32, i32, i32),
    unit: i32,
    pad: i32,
    value: f32,
    state: u32,
) -> ScrollDraw {
    let (x, y, w, h) = rect;
    let (width, offset) = scroll_thumb(unit, pad, w, value);
    ScrollDraw {
        track: (x + 2, y + 2, (x + w) - 2, (y + h) - 2),
        track_alpha: 0.6,
        thumb: (x + pad + offset, y + pad, width, (h - (pad << 1)).max(0)),
        pressed: state & 0x10 != 0,
    }
}

/// The image view's fading shade: the inset rectangle with a transparent
/// top edge (grey 128) and an opaque bottom.
pub fn image_shade(rect: (i32, i32, i32, i32), pad: i32) -> (i32, i32, i32, i32) {
    image_rect(rect, pad)
}

/// The ink an image's shade takes at its top and bottom: 128, 128, 128 at
/// alpha 0 and 255.
pub const IMAGE_SHADE_TOP: [u8; 4] = [0x80, 0x80, 0x80, 0x00];
pub const IMAGE_SHADE_BOTTOM: [u8; 4] = [0x80, 0x80, 0x80, 0xff];

/// A texture view's draw: either the filled inset rectangle in the stored
/// colour, or the texture's cell centred in the control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureDraw {
    Fill {
        rect: (i32, i32, i32, i32),
        ink: [u8; 4],
    },
    Cell {
        origin: (i32, i32),
        cell: i32,
    },
}

/// Lays out a texture view's draw. With no texture it fills its inset
/// rectangle in its stored ink; with one it centres the texture's cell,
/// each half truncated toward zero.
pub fn texture_draw(rect: (i32, i32, i32, i32), pad: i32, cell: Option<(i32, i32)>) -> TextureDraw {
    match cell {
        Some((cell, _ink)) => TextureDraw::Cell {
            origin: (rect.0 + (rect.2 - cell) / 2, rect.1 + (rect.3 - cell) / 2),
            cell,
        },
        None => TextureDraw::Fill {
            rect: image_rect(rect, pad),
            ink: [0, 0, 0, 0xff],
        },
    }
}

/// A caption's draw: whether the skin draws behind it and whether the text
/// wraps with the style's 0x3000 bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CaptionDraw {
    pub back: bool,
    pub wraps: bool,
}

/// Lays out a caption's draw.
pub fn caption_draw(style: u32) -> CaptionDraw {
    CaptionDraw {
        back: caption::draws_back(style),
        wraps: caption::wraps(style),
    }
}

/// The colour drag's draw ink: the dragged colour at alpha 0xa0.
pub fn colour_drag_ink(colour: u32) -> [u8; 4] {
    let [red, green, blue, _] = colour.to_le_bytes();
    [red, green, blue, 0xa0]
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn the_classes_register_in_order_with_their_defaults() {
        assert_eq!(extended_class(TEXT_FIELD).unwrap().size, (80, 24));
        assert_eq!(extended_class(SPINNER).unwrap().size, (60, 20));
        assert_eq!(extended_class(TRACKBAR).unwrap().size, (0, 28));
        assert_eq!(extended_class(SCROLL_BAR).unwrap().size, (0, 16));
        assert_eq!(extended_class(SEPARATOR).unwrap().size, (0, 3));
        let hooked: Vec<u16> = EXTENDED_CLASSES
            .iter()
            .filter(|class| class.has_init)
            .map(|class| class.id)
            .collect();
        assert_eq!(
            hooked,
            [TEXT_FIELD, COMBO_BOX, SPINNER, TRACKBAR, SCROLL_BAR]
        );
        assert_eq!(extended_class(1), None);
        assert_eq!(spinner_button_width(4), 13);
        assert_eq!(spinner_button_width(5), 15);
        assert_eq!(text_field_span(100, 2, 4), 88);
        assert_eq!(combo_text_span(100, 2, 4), 75);
        assert_eq!(spinner_text_span(100, 2, 4), 75);
        // The trackbar's creation hook defaults the knob width, not the
        // height.
        assert_eq!(trackbar_defaults(0), (20, 2));
        assert_eq!(trackbar_defaults(24), (24, 2));
        assert_eq!(trackbar_rescale(20, 4, 8), 40);
        assert_eq!(trackbar_rescale(21, 4, 6), 32);
        assert_eq!(scrollbar_defaults(), 2);
        assert_eq!(round_to_i64(2.5), 2);
        assert_eq!(round_to_i64(3.5), 4);
        assert_eq!(round_to_i64(f32::NAN), i64::MIN);
        assert_eq!(round_to_i64(1e30), i64::MIN);
    }

    fn advances() -> Vec<i16> {
        let mut table = vec![1i16; 256];
        table[usize::from(b'A')] = 10;
        table[usize::from(b'B')] = 10;
        table
    }

    #[test]
    fn an_edit_hit_takes_the_byte_whose_half_advance_passes() {
        let text = b"AB";
        let edit = EditState {
            end: 2,
            ..EditState::default()
        };
        assert_eq!(edit.hit(text, 4, &advances()), 0);
        assert_eq!(edit.hit(text, 5, &advances()), 1);
        assert_eq!(edit.hit(text, 14, &advances()), 1);
        assert_eq!(edit.hit(text, 15, &advances()), 2);
        assert_eq!(edit.hit(text, 99, &advances()), 2);
        let scrolled = EditState { start: 1, ..edit };
        assert_eq!(scrolled.hit(text, 4, &advances()), 1);
        // A run past the text clamps to its length.
        let long = EditState { end: 5, ..edit };
        assert_eq!(long.hit(text, 99, &advances()), 2);

        let mut state = edit;
        state.select_all(2);
        assert_eq!((state.anchor, state.caret), (0, 2));
        state.reset(3);
        assert_eq!(
            state,
            EditState {
                anchor: 3,
                caret: 3,
                end: 3,
                start: 0
            }
        );
    }

    #[test]
    fn a_text_fields_keys_submit_filter_or_edit() {
        use text_style::*;
        assert_eq!(
            text_field_key(EDITABLE, 0x0d, false, true),
            TextFieldKey::Submit { release: false }
        );
        assert_eq!(
            text_field_key(EDITABLE | RELEASE, 0x0d, false, true),
            TextFieldKey::Submit { release: true }
        );
        assert_eq!(
            text_field_key(EDITABLE, 0x0d, false, false),
            TextFieldKey::Edit {
                max_len: 0,
                notify: false
            }
        );
        assert_eq!(
            text_field_key(EDITABLE | DIGITS, u32::from(b'x'), true, false),
            TextFieldKey::Dropped
        );
        assert_eq!(
            text_field_key(EDITABLE | DIGITS, u32::from(b'5'), true, false),
            TextFieldKey::Edit {
                max_len: 0,
                notify: false
            }
        );
        // The digit filter reads typed bytes only.
        assert!(matches!(
            text_field_key(EDITABLE | DIGITS, 0x78, false, false),
            TextFieldKey::Edit { .. }
        ));
        assert_eq!(
            text_field_key(EDITABLE | NOTIFY, u32::from(b'a'), true, true),
            TextFieldKey::Edit {
                max_len: 0,
                notify: true
            }
        );
        assert_eq!(
            text_field_key(0x5, u32::from(b'a'), true, true),
            TextFieldKey::Move
        );

        let mut field = TextField::new(b"AB", 0, EDITABLE);
        assert_eq!(field.max_len, 255);
        assert_eq!(
            field.key(u32::from(b'c'), true, false),
            TextFieldKey::Edit {
                max_len: 255,
                notify: false
            }
        );
        field.focus(true);
        assert_eq!((field.edit.anchor, field.edit.caret), (0, 2));
        field.edit.end = 2;
        field.press_anchor(5, &advances());
        assert_eq!((field.edit.anchor, field.edit.caret), (1, 1));
        assert_eq!(text_border((10, 20, 80, 24), 0x10), Some((11, 22, 89, 42)));
        assert_eq!(text_border((10, 20, 80, 24), 0), None);
    }

    #[test]
    fn a_drop_down_opens_below_cut_or_above() {
        // 176 pixels below fit seven 24-pixel rows inside the pad.
        assert_eq!(
            drop_down_place((10, 100, 80, 24), 200, 2, 24, 300),
            DropDownPlace {
                x: 10,
                y: 124,
                w: 80,
                h: 172,
                scrolls: true
            }
        );
        // No row below and ten above: it opens above at its full height.
        assert_eq!(
            drop_down_place((10, 250, 80, 24), 200, 2, 24, 300),
            DropDownPlace {
                x: 10,
                y: 50,
                w: 80,
                h: 200,
                scrolls: false
            }
        );
        assert_eq!(drop_down_place((10, 100, 80, 24), 200, 2, 24, 1000).h, 200);
        // A negative row count halves (unsigned) to a huge number, so the
        // list is cut below to a negative height.
        assert_eq!(
            drop_down_place((0, 300, 80, 40), 200, 2, 24, 300),
            DropDownPlace {
                x: 0,
                y: 340,
                w: 80,
                h: -20,
                scrolls: true
            }
        );
    }

    #[test]
    fn a_combo_box_takes_its_item_and_opens_or_edits() {
        let items = vec![b"one".to_vec(), b"two".to_vec()];
        let mut combo = ComboBox::new(items.clone(), 1, 0);
        assert_eq!((combo.text.as_slice(), combo.max_len), (&b"one"[..], 255));
        assert!(combo.choose(2));
        assert_eq!(combo.text, b"two");
        assert!(combo.choose(9));
        assert_eq!(combo.selected, 2);
        assert!(!combo.choose(0));
        assert!(combo.text.is_empty());
        let mut short = ComboBox::new(items, 0, 2);
        assert!(short.text.is_empty());
        short.choose(1);
        assert_eq!(short.text, b"on");

        assert_eq!(combo_cursor(0xd, 10, (0, 0, 80, 24), 2, 4), 0x802_0000);
        assert_eq!(combo_cursor(0xd, 61, (0, 0, 80, 24), 2, 4), 0);
        assert_eq!(combo_cursor(0, 10, (0, 0, 80, 24), 2, 4), 0);
        assert_eq!(combo_press(0x2_0000), ComboPress::Edit);
        assert_eq!(combo_press(0x1_0000), ComboPress::Open);
        assert_eq!(combo_arrow((10, 20, 80, 24), 4, 0), Some((74, 25)));
        assert_eq!(combo_arrow((0, 0, 80, 24), 5, 0), Some((60, 5)));
        assert_eq!(combo_arrow((10, 20, 80, 24), 4, 0x800_0000), None);
    }

    #[test]
    fn a_swatch_drags_its_colour_or_calls_back() {
        let rect = (10, 20, 50, 24);
        let drag = ColourDrag {
            colour: 0xff00ff,
            offset: (20, 20),
            size: (46, 20),
        };
        assert_eq!(
            swatch_press(0x4000, 0xff00ff, (30, 40), rect, 2),
            SwatchPress::Drag(drag)
        );
        assert_eq!(
            swatch_press(0x6000, 0xff00ff, (30, 40), rect, 2),
            SwatchPress::Drag(drag)
        );
        assert_eq!(
            swatch_press(0x2000, 0, (30, 40), rect, 2),
            SwatchPress::Callback
        );
        assert_eq!(swatch_press(0, 0, (30, 40), rect, 2), SwatchPress::Ignored);
        assert_eq!(colour_drag_rect((100, 100), &drag), (80, 80, 46, 20));
        assert!(swatch_accepts(COLOUR_DRAG_CODE));
        assert!(!swatch_accepts(COLOUR_DRAG_CODE + 1));
    }

    fn spinner() -> Spinner {
        Spinner {
            value: 0.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            integer: false,
            dirty: false,
            decimals: 1,
            text: b"0.0".to_vec(),
        }
    }

    #[test]
    fn a_spinner_inits_and_commits_with_ordered_clamps() {
        let mut fresh = Spinner {
            value: -5.0,
            max: 0.0,
            step: 0.0,
            ..spinner()
        };
        fresh.init();
        assert_eq!((fresh.step, fresh.max, fresh.value), (1.0, 100.0, 0.0));
        // An integer spinner's step is the integer 1 and its maximum the
        // float 100's bits; its value is not raised.
        let mut whole = Spinner {
            value: -5.0,
            max: 0.0,
            step: 0.0,
            integer: true,
            ..spinner()
        };
        whole.init();
        assert_eq!(whole.step.to_bits(), 1);
        assert_eq!(whole.max.to_bits(), 0x42c8_0000);
        assert_eq!(whole.value, -5.0);

        let mut value = spinner();
        assert!(value.commit(150.0, false));
        assert_eq!(value.value, 100.0);
        assert!(value.dirty);
        // A NaN passes both clamps and never lands, but marks the text.
        let mut nan = spinner();
        nan.value = 50.0;
        assert!(!nan.commit(f32::NAN, false));
        assert_eq!(nan.value, 50.0);
        assert!(nan.dirty);
        assert!(!nan.commit(50.0, false));
        let mut quiet = spinner();
        quiet.value = 100.0;
        assert!(quiet.commit(-3.0, true));
        assert_eq!(quiet.value, 0.0);
        assert!(!quiet.dirty);
    }

    #[test]
    fn a_text_field_dispatches_its_messages() {
        let mut field = TextField::new(b"AB", 0, text_style::EDITABLE);
        let advances = advances();
        assert_eq!(
            field.message(1, 0, 0, 0, &advances, false),
            TextFieldMessage::DrawBack { border: None }
        );
        assert_eq!(
            field.message(0x10, 0, 0, 0, &advances, false),
            TextFieldMessage::Pointer(0x801_0000)
        );
        assert_eq!(
            field.message(0x30, 0, 0, 5, &advances, false),
            TextFieldMessage::Caret(1)
        );
        assert_eq!(
            field.message(0xa0, 0x0d, 0, 0, &advances, true),
            TextFieldMessage::Submit { release: false }
        );
        field.style |= text_style::DIGITS;
        assert_eq!(
            field.message(0xa0, u32::from(b'x'), 2, 0, &advances, false),
            TextFieldMessage::Dropped
        );
        field.style |= 0x400;
        assert_eq!(
            field.message(0xb0, 0, 0, 0, &advances, false),
            TextFieldMessage::Reset
        );
        assert_eq!((field.edit.start, field.edit.caret), (0, 2));
        assert_eq!(
            field.message(
                0x200,
                u32::from_le_bytes(*b"Q\0\0\0"),
                0,
                0,
                &advances,
                false
            ),
            TextFieldMessage::SetText
        );
        assert_eq!(field.text, b"Q");
    }

    #[test]
    fn a_combo_box_dispatches_its_messages() {
        let items = vec![b"one".to_vec(), b"two".to_vec()];
        let mut combo = ComboBox::new(items, 1, 0);
        let advances = advances();
        assert_eq!(
            combo.message(0x40, 1, 0, 0, &advances, 0, 0),
            ComboBoxMessage::Open
        );
        assert_eq!(
            combo.message(0x40, 1, 0, 10, &advances, 0xd, 0x2_0000),
            ComboBoxMessage::Anchor(3)
        );
        assert_eq!(
            combo.message(0xa0, 0x0d, 0, 0, &advances, 0, 0),
            ComboBoxMessage::Submit
        );
        assert_eq!(
            combo.message(0x200, 2, 0, 0, &advances, 0, 0),
            ComboBoxMessage::Chosen
        );
        assert_eq!(combo.text, b"two");
        combo.text = b"t".to_vec();
        assert_eq!(
            combo.message(0xb0, 0, 0, 0, &advances, 0, 0x400),
            ComboBoxMessage::Restored
        );
        assert_eq!(combo.text, b"two");
    }

    #[test]
    fn a_spinner_formats_and_takes_a_set_value() {
        let mut value = spinner();
        value.value = 12.26;
        assert_eq!(value.text(), "12.3");
        value.value = 12.0;
        value.text = value.formatted();
        assert!(value.set_value(150.0f32.to_bits(), true));
        assert_eq!(value.value, 100.0);
        assert_eq!(value.text, b"100.0");
        // An integer word is converted, not reinterpreted.
        assert!(value.set_value(3, false));
        assert_eq!(value.value, 3.0);
        assert!(!value.set_value(3.0f32.to_bits(), true));
        assert!(!value.set_value(f32::NAN.to_bits(), true));
        let mut whole = spinner();
        whole.integer = true;
        whole.value = f32::from_bits(7);
        whole.text = whole.formatted();
        assert!(whole.set_value(9, false));
        assert!(!whole.set_value(9, true));
        assert_eq!(whole.value.to_bits(), 9);
        assert_eq!(whole.text, b"9");
        assert_eq!(ProgressBar { value: 0.25 }.text_with(2), "25.00%");
    }

    #[test]
    fn a_spinner_presses_its_text_or_halves_and_filters_keys() {
        let rect = (0, 0, 60, 20);
        assert_eq!(spinner_press(rect, 13, 10, 5, 0), SpinnerPress::Anchor);
        assert_eq!(
            spinner_press(rect, 13, 10, 5, 0x8000),
            SpinnerPress::SelectAll
        );
        assert_eq!(spinner_press(rect, 13, 47, 5, 0), SpinnerPress::Up);
        assert_eq!(spinner_press(rect, 13, 50, 10, 0), SpinnerPress::Down);
        assert_eq!(spinner_cursor(rect, 13, 0xd, 10, 5), 0x801_0000);
        assert_eq!(spinner_cursor(rect, 13, 0, 10, 5), 0x1_0000);
        assert_eq!(spinner_cursor(rect, 13, 0, 50, 5), 0x2_0000);
        assert_eq!(spinner_cursor(rect, 13, 0, 50, 15), 0x4_0000);
        // An unsigned "negative" x is far right, on the buttons.
        assert_eq!(spinner_cursor(rect, 13, 0, u32::MAX, 15), 0x4_0000);

        let at = |anchor, caret| EditState {
            anchor,
            caret,
            ..EditState::default()
        };
        let typed = |byte: u8, edit| spinner_key(b"1.5", edit, 0, u32::from(byte), 2);
        assert_eq!(typed(b'.', at(1, 1)), SpinnerKey::SkipDot);
        assert_eq!(typed(b'.', at(0, 0)), SpinnerKey::Edit { commit: false });
        assert_eq!(typed(b'-', at(1, 1)), SpinnerKey::Dropped);
        assert_eq!(typed(b'-', at(0, 1)), SpinnerKey::Edit { commit: false });
        assert_eq!(typed(b'x', at(0, 0)), SpinnerKey::Dropped);
        let key = |code, flags| spinner_key(b"", at(0, 0), 0x200_0000, code, flags);
        assert_eq!(key(0x26, 0), SpinnerKey::Step(true));
        assert_eq!(key(0x28, 0), SpinnerKey::Step(false));
        assert_eq!(key(0x0d, 0), SpinnerKey::Commit);
        assert_eq!(key(u32::from(b'7'), 2), SpinnerKey::Edit { commit: true });
        assert_eq!(key(0x56, 0x40), SpinnerKey::Paste);
        assert_eq!(key(0x43, 0x40), SpinnerKey::Copy);
        assert_eq!(key(0x41, 0x40), SpinnerKey::Ignored);
        assert_eq!(key(0x26, 0x80), SpinnerKey::Ignored);
        assert_eq!(key(0x56, 0x42), SpinnerKey::Ignored);

        assert_eq!(spinner_repeat_step(4, 0.0, 100.0, 1.0), 4);
        assert_eq!(spinner_repeat_step(4, 0.0, 1000.0, 1.0), 2);
        assert_eq!(spinner_repeat_step(4, 0.0, 1.0, 1.0), 32);
        // A zero range divides to infinity: the indefinite integer's low
        // half is 0, which the floor lifts to half a unit.
        assert_eq!(spinner_repeat_step(4, 0.0, 0.0, 1.0), 2);

        let buttons = spinner_buttons(rect, 13, 4);
        assert_eq!(buttons.field, (0, 0, 47, 20));
        assert_eq!(buttons.up, (47, -1, 13, 11));
        assert_eq!(buttons.down, (47, 10, 13, 11));
        assert_eq!((buttons.up_arrow, buttons.down_arrow), ((47, 2), (47, 13)));
        assert_eq!(buttons.arrow_width, 7);
        assert_eq!(spinner_buttons(rect, 15, 5).up.1, -2);
    }

    #[test]
    fn a_progress_bar_sets_fills_and_lays_out() {
        let mut bar = ProgressBar { value: 0.0 };
        bar.set(-2.0);
        assert_eq!(bar.value, 0.0);
        bar.set(-0.0);
        assert!(bar.value.is_sign_negative());
        bar.set(f32::NAN);
        assert!(bar.value.is_nan());
        bar.set(0.5);
        assert_eq!(bar.text(), "50.0%");
        assert_eq!(bar.fill(100), 50);
        assert_eq!(bar.fill(101), 50);
        assert_eq!(bar.fill(103), 52);
        assert!(ProgressBar::shows_text(21, 4));
        assert!(!ProgressBar::shows_text(20, 4));
        let rects = progress_rects((10, 20, 100, 24), 2, 50);
        assert_eq!(rects.rest, Some((59, 21, 109, 43)));
        assert_eq!(rects.bar, Some((14, 24, 56, 40)));
        assert_eq!(rects.highlight, Some((13, 23, 57, 41)));
        assert_eq!(progress_rects((10, 20, 100, 24), 2, 0).bar, None);
        assert_eq!(progress_rects((10, 20, 100, 24), 2, 100).rest, None);
    }

    #[test]
    fn a_separator_lines_up_around_its_text() {
        assert_eq!(separator_gap(8, 4), 32);
        assert_eq!(
            separator_lines((0, 10, 100, 3), 0, 0, 4, 0),
            vec![(0, 9, 100, 3)]
        );
        assert_eq!(
            separator_lines((0, 10, 100, 3), 0, 0, 4, 2),
            vec![(0, 14, 100, 3)]
        );
        assert_eq!(
            separator_lines((0, 10, 100, 3), 32, 0, 4, 0),
            vec![(0, 10, 33, 3), (67, 10, 33, 3)]
        );
        assert_eq!(
            separator_lines((0, 10, 100, 3), 32, 0x2000, 4, 0),
            vec![(36, 10, 64, 3)]
        );
        assert_eq!(separator_lines((0, 10, 100, 3), 32, 0x3000, 4, 0).len(), 2);
    }

    #[test]
    fn an_image_picks_and_a_texture_centres() {
        let pick = image_pick(100, 80, 2, 2, 2);
        assert_eq!((pick.u, pick.v), (0.0, 1.0));
        let pick = image_pick(100, 80, 2, 51, 41);
        assert_eq!((pick.u, pick.v), (0.5, 0.5));
        let pick = image_pick(100, 80, 2, 200, 200);
        assert_eq!((pick.u, pick.v), (1.0, 0.0));
        // 0/0 passes both clamps; x/0 is infinite and clamps to 1.
        assert!(image_pick(2, 80, 2, 2, 2).u.is_nan());
        assert_eq!(image_pick(2, 80, 2, 3, 2).u, 1.0);
        assert_eq!(image_rect((10, 20, 100, 80), 2), (12, 22, 96, 76));
        assert_eq!(texture_origin((10, 20, 100, 80), 40), (40, 40));
        assert_eq!(texture_origin((0, 0, 10, 10), 21), (-5, -5));
    }

    #[test]
    fn a_caption_passes_the_pointer_without_a_callback() {
        assert!(caption::draws_back(0x10));
        assert!(!caption::draws_back(0));
        assert!(caption::wraps(0x1000));
        assert!(!caption::wraps(0x4000));
        assert_eq!(caption::pointer(false), PASS_THROUGH);
        assert_eq!(caption::pointer(true), 0);
        assert!(caption::press(false, 0x2001, true));
        assert!(!caption::press(true, 0x2001, true));
        assert!(!caption::press(false, 0x2002, true));
    }

    #[test]
    fn the_spinner_draw_lays_out_its_field_and_buttons() {
        let draw = spinner_draw((10, 20, 60, 20), 13, 4, 0, 0);
        assert_eq!(draw.buttons.field, (10, 20, 57, 40));
        assert_eq!(draw.inner, (11, 21, 56, 39));
        assert_eq!(draw.buttons.up, (57, 19, 13, 11));
        assert_eq!(draw.buttons.down, (57, 30, 13, 11));
        assert_eq!(
            (draw.buttons.up_arrow, draw.buttons.down_arrow),
            ((57, 22), (57, 33))
        );
        assert_eq!(draw.buttons.arrow_width, 7);
        assert!(!draw.up_pressed && !draw.down_pressed && !draw.hot);
        assert_eq!((draw.up_arrow_state, draw.down_arrow_state), (0, 0));
        let marked = spinner_draw((0, 0, 60, 20), 13, 5, 0x6_0000, 0x100_0010);
        assert!(marked.hot && marked.up_pressed);
        assert_eq!(marked.up_arrow_state, -1);
        assert_eq!(marked.buttons.up.1, -2);
        // Style bit 0x20000 marks the up arrow when the control is not hot.
        let up = spinner_draw((0, 0, 60, 20), 13, 4, 0x2_0000, 0);
        assert_eq!((up.up_arrow_state, up.down_arrow_state), (1, 0));
    }

    #[test]
    fn a_swatch_separator_and_image_expose_their_small_rules() {
        assert!(swatch_draw(0x100_0000));
        assert!(!swatch_draw(0));
        assert_eq!(SEPARATOR_POINTER, PASS_THROUGH);
        assert_eq!(
            image_corners((10, 20, 100, 80), (5, 7)),
            ((15, 27), (115, 107))
        );
    }

    #[test]
    fn a_progress_bar_draws_its_rest_bar_and_highlight() {
        let draw = progress_draw((10, 20, 100, 24), 2, 0.5);
        assert_eq!(draw.rest, Some((59, 21, 109, 43)));
        assert_eq!(draw.bar, Some((14, 24, 56, 40)));
        assert_eq!(draw.highlight, Some((13, 23, 57, 41)));
        assert_eq!(draw.rest_alpha, 0.4);
        assert_eq!(progress_draw((0, 0, 100, 24), 2, 0.0).bar, None);
        assert_eq!(progress_draw((0, 0, 100, 24), 2, 1.0).rest, None);
    }

    #[test]
    fn a_separator_swaps_its_ink_set() {
        let draw = separator_draw((0, 10, 100, 3), 32, 0, 4, 0);
        assert_eq!(draw.lines, vec![(0, 10, 33, 3), (67, 10, 33, 3)]);
        assert_eq!(draw.inks, [0x9c, 0xa0, 0xac, 0xb4]);
        assert_eq!(
            separator_draw((0, 0, 10, 3), 0, 0x40, 4, 0).inks,
            [0xb8, 0xb0, 0xa0, 0x9c]
        );
    }

    #[test]
    fn a_combo_box_draws_its_arrow_and_picks_the_text_path() {
        let draw = combo_draw((10, 20, 80, 24), 4, 0, 0);
        assert_eq!(draw.arrow, Some((74, 25)));
        assert_eq!(draw.arrow_ink, 0xe0);
        assert!(!draw.arrow_textured && !draw.editing);
        let editing = combo_draw((0, 0, 80, 24), 4, 0, 0x1_0010);
        assert!(editing.editing && editing.arrow_textured);
        assert_eq!(editing.arrow_ink, 0xd4);
        assert_eq!(combo_draw((0, 0, 80, 24), 4, 0x800_0000, 0).arrow, None);
    }

    #[test]
    fn a_track_and_a_scroll_bar_lay_out_their_tracks_and_thumbs() {
        let track = track_draw((0, 0, 100, 20), 2, 20, 0.5, 0);
        assert_eq!(track.track, (2, 2, 98, 18));
        assert_eq!(track.thumb, (40, 2, 20, 16));
        assert_eq!((track.offset, track.track_alpha), (38, 0.6));
        let pressed = track_draw((0, 0, 100, 20), 2, 20, 0.0, 0x10);
        assert!(pressed.pressed);
        assert_eq!(pressed.thumb, (2, 2, 20, 16));
        let scroll = scroll_draw((0, 0, 100, 80), 4, 2, 0.0, 0);
        assert_eq!(scroll.track, (2, 2, 98, 78));
        assert_eq!(scroll.thumb, (2, 2, 17, 76));
        assert_eq!(
            scroll_draw((0, 0, 100, 80), 4, 2, 0.5, 0).thumb,
            (42, 2, 17, 76)
        );
    }

    #[test]
    fn an_image_texture_and_colour_drag_draw_their_parts() {
        assert_eq!(image_shade((10, 20, 100, 80), 2), (12, 22, 96, 76));
        assert_eq!(IMAGE_SHADE_TOP, [0x80, 0x80, 0x80, 0]);
        assert_eq!(IMAGE_SHADE_BOTTOM, [0x80, 0x80, 0x80, 0xff]);
        assert_eq!(
            texture_draw((10, 20, 100, 80), 2, None),
            TextureDraw::Fill {
                rect: (12, 22, 96, 76),
                ink: [0, 0, 0, 0xff]
            }
        );
        assert_eq!(
            texture_draw((10, 20, 100, 80), 2, Some((40, 0))),
            TextureDraw::Cell {
                origin: (40, 40),
                cell: 40
            }
        );
        assert_eq!(colour_drag_ink(0x1234_56ff), [0xff, 0x56, 0x34, 0xa0]);
        assert_eq!(
            caption_draw(0x1000),
            CaptionDraw {
                back: false,
                wraps: true
            }
        );
        assert_eq!(
            caption_draw(0x10),
            CaptionDraw {
                back: true,
                wraps: false
            }
        );
    }

    #[test]
    fn the_spinner_drag_places_steps_captures_and_lights_halves() {
        let spinner = Spinner {
            value: 10.0,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            integer: false,
            dirty: false,
            decimals: 1,
            text: Vec::new(),
        };
        let rect = (0, 0, 60, 20);
        let mut drag = SpinnerDrag::default();
        assert_eq!(
            spinner_drag(&mut drag, &spinner, rect, 13, 4, 0, (10, 10), (10, 10)),
            SpinnerDragAction::Text
        );
        // The text drag: a move past the unit captures over the value's reach.
        drag.state = 0x0100_0000;
        assert_eq!(
            spinner_drag(&mut drag, &spinner, rect, 13, 4, 0, (10, 100), (10, 110)),
            SpinnerDragAction::Capture {
                low: -254,
                high: 154
            }
        );
        assert_eq!(drag.state, 0x0306_0000);
        assert_eq!((drag.origin_y, drag.origin_value), (110, 10.0));
        // And a live drag steps by the 4-pixel repeat.
        assert_eq!(
            spinner_drag(&mut drag, &spinner, rect, 13, 4, 0, (10, 100), (10, 88)),
            SpinnerDragAction::SetFloat(16.0)
        );
        // Integer spinners add the step's bits to the value's bits.
        let mut whole = spinner.clone();
        whole.integer = true;
        whole.step = f32::from_bits(1);
        let mut drag = SpinnerDrag {
            state: 0x0300_0000,
            origin_y: 100,
            origin_value: 10.0,
            origin_bits: 10,
        };
        assert_eq!(
            spinner_drag(&mut drag, &whole, rect, 13, 4, 0, (0, 0), (0, 88)),
            SpinnerDragAction::SetInt(16)
        );
        // The hot halves: upper takes 0x01000000 under style 0x4000000.
        let mut drag = SpinnerDrag {
            state: 0x0200_0000,
            ..SpinnerDrag::default()
        };
        assert_eq!(
            spinner_drag(
                &mut drag,
                &spinner,
                rect,
                13,
                4,
                0x400_0000,
                (10, 5),
                (10, 5)
            ),
            SpinnerDragAction::Hot
        );
        assert_eq!(drag.state, 0x0100_0000);
        assert_eq!(
            spinner_drag(
                &mut drag,
                &spinner,
                rect,
                13,
                4,
                0x400_0000,
                (10, 15),
                (10, 15)
            ),
            SpinnerDragAction::Hot
        );
        assert_eq!(drag.state, 0);
    }

    #[test]
    fn a_text_field_draws_its_frame_and_aligned_run() {
        assert_eq!(text_inset(4), 13);
        assert_eq!(text_inset(5), 15);
        let draw = text_field_draw(
            (10, 20, 80, 24),
            0x1000,
            EditState {
                start: 1,
                end: 4,
                ..EditState::default()
            },
            30,
            16,
            14,
            2,
            4,
        );
        assert_eq!(draw.border, None);
        assert_eq!(draw.origin.x, 10 - 30 - 4);
        assert_eq!((draw.start, draw.end), (1, 4));
        assert_eq!(
            text_field_draw(
                (10, 20, 80, 24),
                0x10,
                EditState::default(),
                0,
                16,
                14,
                2,
                4
            )
            .border,
            Some((11, 22, 89, 42))
        );
    }

    #[test]
    fn the_scroll_captions_take_their_three_alignments() {
        let runs = scroll_caption_runs((0, 0, 100, 16), 2, 4, 20, 10, 12, 16, 14, 0x4000);
        assert_eq!(runs[0].style, 0);
        assert_eq!(runs[0].x, 2 + (4 + 1) / 2);
        assert_eq!(runs[1].style, 0x4000);
        assert_eq!(runs[2].style, 0x3000);
        // The value is centred in the usable span.
        let avail = text_span_a(100, 2, 4);
        assert_eq!(runs[2].x, (avail - 1 - 12) / 2 + 2 + 4);
    }

    #[test]
    fn a_scroll_bar_maps_its_thumb_and_labels() {
        assert_eq!(scroll_thumb(4, 2, 100, 0.0), (17, 0));
        assert_eq!(scroll_thumb(4, 2, 100, 0.5), (17, 40));
        assert_eq!(scroll_thumb(4, 2, 100, 1.0), (17, 79));
        assert_eq!(scroll_thumb(4, 2, 100, f32::NAN), (17, 0));
        assert_eq!(scroll_value(4, 0, 2, 100, 10), 0.0);
        assert_eq!(scroll_value(4, 0, 2, 100, 89), 1.0);
        assert_eq!(scroll_value(4, 0, 2, 100, 200), 1.0);
        assert_eq!(scroll_value(4, 0, 2, 100, 0), 0.0);
        // Zero travel: 0/0 passes the clamps, 1/0 clamps to 1.
        assert!(scroll_value(4, 0, 2, 21, 10).is_nan());
        assert_eq!(scroll_value(4, 0, 2, 21, 11), 1.0);
        let labels = scroll_labels(b"volume", b"%", 0.5, 0.0, 100.0);
        assert_eq!(
            (labels.text.as_slice(), labels.suffix.as_slice()),
            (&b"volume"[..], &b"%"[..])
        );
        assert_eq!(labels.value, "50");
        assert!(drag_press(false, 1));
        assert!(!drag_press(true, 1));
        assert!(!drag_press(false, 2));
    }
}

//! The GUI form's own layout metrics and base-class rules (`TGUIForm`):
//! skin measures, the 302×402 default form, anchoring, the client
//! rectangle and both scrollbars, the unset and shown places, the sibling
//! layers, the active-form tests, maximise and restore, the runtime resize,
//! the 256-tick show/hide fade, the point conversions, the hit test with
//! its control slop, hover, focus, Tab traversal, the caret, the wheel and
//! held-arrow scrolls, the frame codes a press, drag and release act on,
//! the magnetic snap slots and the controls' rescale. The row spacing of
//! the main menu is not modelled here; the binary's `70·0.5·round(H/135)`
//! does not fit its own list and needs a screenshot to reconcile.
//!
//! Three words describe a form: the style word places an unplaced form and
//! anchors it in a resize; the attribute word selects the frame parts
//! (title, scrollbars, snapping) and the sibling layer; the state word
//! carries the layout bookkeeping, the maximised bit and the caret. A
//! control record has its own style word (anchors, tab stop, caret) and
//! state word (disabled, focus, hover bits). Positions and sizes are
//! physical pixels. Rules: `docs/UI.md` (The form base class).

/// A GUI unit and the DPI border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormMetrics {
    pub unit: i32,
    pub border: i32,
}

impl FormMetrics {
    /// The horizontal scrollbar's smallest thumb: a fixed 9 at every GUI
    /// unit (the vertical scrollbar's is `2·unit + 1`).
    pub const MIN_THUMB: i32 = 9;

    /// `6·unit − border` (the skin's base measure).
    pub fn base(self) -> i32 {
        6 * self.unit - self.border
    }

    /// `4·unit + 1` (the close button).
    pub fn close(self) -> i32 {
        4 * self.unit + 1
    }

    /// `4·unit − 1` (the scroll rail).
    pub fn rail(self) -> i32 {
        4 * self.unit - 1
    }

    /// `(unit >> 1) + close` (the tab).
    pub fn tab(self) -> i32 {
        (self.unit >> 1) + self.close()
    }
}

/// The DPI border: `max(2, trunc(dpi/135))`.
pub fn border_for_dpi(dpi: i32) -> i32 {
    (dpi / 135).max(2)
}

/// The form constructor's defaults.
pub const DEFAULT_WIDTH: i32 = 302;
pub const DEFAULT_HEIGHT: i32 = 402;
pub const DEFAULT_BORDER: i32 = 2;
pub const DEFAULT_CONTENT: (i32, i32) = (300, 800);
/// The horizontal scroll step; the layout recomputes it from the overflow.
pub const DEFAULT_SCROLL_STEP: f32 = 0.025;
/// The vertical scroll step. Nothing in the form base recomputes it; the
/// wheel and the held arrows move by it.
pub const DEFAULT_VERTICAL_STEP: f32 = 0.25;
/// The smallest page the grips resize to: 76 high and 90 wide (the
/// constructor's two words in field order; the resize reads the second as
/// the width and the first as the height).
pub const PAGE_MIN: (i32, i32) = (76, 90);
/// The constructor's attribute word: a close mark, a client fill and the
/// 0x30000 bits.
pub const DEFAULT_ATTRIBUTES: u32 = 0x30022;
/// The constructor's state word: laid out and the style pending.
pub const DEFAULT_STATE: u32 = 0x3000;
/// An unplaced coordinate (the constructor's sentinel).
pub const UNPLACED: i32 = -987_654_321;
/// The scroll rail the layout uses (four units less one; 15 at unit 4).
pub const RAIL_AT_UNIT_FOUR: i32 = 15;

/// An anchor's axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Anchor {
    /// Default: 4 px on X, 36 px on Y.
    Default,
    /// Centred in the parent.
    Center,
    /// The far edge, 4 px in.
    Far,
}

/// Resolves an unset position against the parent's span. `parent` is the
/// parent's client extent (right−left or bottom−top); `size` is the
/// child's.
pub fn anchor(position: Option<i32>, parent: i32, size: i32, axis_y: bool, mode: Anchor) -> i32 {
    if let Some(position) = position {
        return position;
    }
    match mode {
        Anchor::Default => {
            if axis_y {
                36
            } else {
                4
            }
        }
        Anchor::Center => {
            let span = parent - size;
            if axis_y {
                // The original uses a logical shift here; for the normal
                // non-negative case this is a floor.
                span >> 1
            } else {
                let sign = (span as u32) >> 31;
                (span + sign as i32) >> 1
            }
        }
        Anchor::Far => parent - size - 4,
    }
}

/// A form's client rectangle: `left` is the border, `top` the title's
/// bottom (or the border), `right` and `bottom` inside the scrollbars.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClientRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl ClientRect {
    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }
}

/// The form's client rectangle from its size, border and attribute word.
/// `title_text` is the skin's title-bar height, `scroll_inset` the
/// vertical scrollbar's width, `title_span` the form's own title spacing
/// (0 without the style word's 0xc0 bits), `rail` the skin rail and
/// `border_metric` the DPI border (2).
#[allow(clippy::too_many_arguments)]
pub fn client_rect(
    width: i32,
    height: i32,
    border: i32,
    style: u32,
    attributes: u32,
    title_text: i32,
    scroll_inset: i32,
    title_span: i32,
    rail: i32,
    border_metric: i32,
) -> ClientRect {
    let top = if attributes & attribute::TITLE != 0 {
        if attributes & 0x80_0030 == 0x20 {
            title_text + border_metric
        } else {
            title_text + title_span
        }
    } else {
        border
    };
    let right = if attributes & attribute::VSCROLL != 0 {
        width - border - scroll_inset
    } else {
        width - border
    };
    let bottom = if attributes & attribute::HSCROLL != 0 {
        (height - border - rail) - 1 - 2 * border_metric
    } else {
        height - border
    };
    let _ = style;
    ClientRect {
        left: border,
        top,
        right,
        bottom,
    }
}

/// The horizontal scrollbar's metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollMetrics {
    pub step: f32,
    pub thumb: i32,
    pub thumb_x: i32,
}

/// The horizontal scrollbar for a form of `width` pixels, `content_w`
/// content, the given border, vertical scrollbar inset and rail at scroll
/// fraction `f`: the step is `10 / (content − client right − border)` (0
/// without room to scroll), the thumb `round(track · client / content)`
/// (ties to even), [`FormMetrics::MIN_THUMB`] when below it, else at most
/// the track, and placed at `border + rail + round((track − thumb) · f)`.
pub fn scroll_metrics(
    width: i32,
    content_w: i32,
    border: i32,
    scroll_inset: i32,
    rail: i32,
    f: f32,
) -> ScrollMetrics {
    let right = width - border - scroll_inset;
    let cw = right - border;
    let over = content_w - right;
    let step = if over <= border {
        0.0
    } else {
        (10.0 / f64::from(over - border)) as f32
    };
    let track = cw - 2 * rail;
    let mut thumb = cvtsd2si(f64::from(track) * (f64::from(cw) / f64::from(content_w)));
    if thumb < FormMetrics::MIN_THUMB {
        thumb = FormMetrics::MIN_THUMB;
    } else if track < thumb {
        thumb = track;
    }
    let x = border + rail + cvtss2si((track - thumb) as f32 * f);
    ScrollMetrics {
        step,
        thumb,
        thumb_x: x,
    }
}

/// Sets the horizontal scroll fraction: clamped to 0..1 with ordered
/// compares (a NaN passes), snapped to whole steps when `snap` (the
/// attribute word's 0x400), stored as 0 when the content fits. The thumb is
/// placed from the snapped fraction, except while the thumb itself is held
/// (`thumb_held`), when it follows the unsnapped one; the no-overflow 0
/// does not move it. Returns the stored fraction and the thumb's x.
#[allow(clippy::too_many_arguments)]
pub fn set_scroll_x(
    f: f32,
    step: f32,
    snap: bool,
    thumb_held: bool,
    width: i32,
    content_w: i32,
    border: i32,
    scroll_inset: i32,
    rail: i32,
    thumb: i32,
) -> (f32, i32) {
    let mut placed = ordered_clamp(f);
    let mut stored = placed;
    if snap {
        stored = cvtss2si64(placed / step) as f32 * step;
        if !thumb_held {
            placed = stored;
        }
    }
    let cw = (width - border - scroll_inset) - border;
    if content_w <= cw {
        stored = 0.0;
    }
    let track = cw - 2 * rail;
    let x = cvtss2si((track - thumb) as f32 * placed) + border + rail;
    (stored, x)
}

/// The label's bracket style: 0 plain, 1 `[t]`, 2 `[[t]]`.
pub fn styled_label(style: u32, text: &str) -> String {
    match style {
        0 => text.to_string(),
        1 => format!("[{text}]"),
        _ => format!("[[{text}]]"),
    }
}

/// The selected row's nudge: `unit / 3`.
pub fn selected_nudge(unit: i32) -> i32 {
    unit / 3
}

/// The form class's shared ease `(3 − 2t)t²`, in the instructions' order:
/// `t²` is taken first, then `3 − 2t` times it.
pub fn ease(t: f32) -> f32 {
    (3.0 - 2.0 * t) * (t * t)
}

/// `CVTSS2SI` into a 64-bit register, keeping the low 32 bits: rounds to
/// nearest, ties to even; a NaN or out-of-range value gives the indefinite
/// integer, whose low half is 0.
pub fn cvtss2si(v: f32) -> i32 {
    cvtss2si64(v) as i32
}

/// `CVTSS2SI` into a 64-bit register.
pub fn cvtss2si64(v: f32) -> i64 {
    let r = v.round_ties_even();
    if r.is_nan() || !(-9.223_372e18..9.223_372e18).contains(&r) {
        i64::MIN
    } else {
        r as i64
    }
}

/// `CVTSD2SI` into a 64-bit register, keeping the low 32 bits.
pub fn cvtsd2si(v: f64) -> i32 {
    let r = v.round_ties_even();
    if r.is_nan() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&r) {
        0
    } else {
        (r as i64) as i32
    }
}

/// Clamps a scroll fraction with the original's ordered comparisons: a
/// NaN passes through both, which `f32::clamp` would not do.
#[allow(clippy::manual_clamp)]
fn ordered_clamp(v: f32) -> f32 {
    if 0.0 <= v {
        if 1.0 < v { 1.0 } else { v }
    } else if v.is_nan() {
        v
    } else {
        0.0
    }
}

/// A form's style word: where an unplaced form goes and how a resize
/// anchors it.
pub mod style {
    /// Centre the unplaced x in the parent's client.
    pub const CENTRE_X: u32 = 0x800;
    /// Centre the unplaced y in the parent's client.
    pub const CENTRE_Y: u32 = 0x8000;
    /// The x anchor bits: `RIGHT` alone anchors to the right edge, both
    /// bits stretch the form with its parent.
    pub const X_MASK: u32 = 0x3_0000;
    pub const RIGHT: u32 = 0x2_0000;
    /// The y anchor bits: `BOTTOM` alone anchors to the bottom edge, both
    /// bits stretch.
    pub const Y_MASK: u32 = 0xc_0000;
    pub const BOTTOM: u32 = 0x8_0000;
    /// A child the parent's hit test passes over.
    pub const NO_HIT: u32 = 2;
    /// A press on the form's client takes the press-through bit 0x10.
    pub const PRESS_THROUGH: u32 = 8;
    /// The form's caret blinks (when no control has the focus).
    pub const CARET: u32 = 4;
}

/// A form's attribute word.
pub mod attribute {
    /// A title bar.
    pub const TITLE: u32 = 1;
    /// A client fill (and a snap container: its children's free space is
    /// cut into slots).
    pub const CLIENT_FILL: u32 = 0x20;
    /// A form that takes part in its parent's snap slots (either bit).
    pub const SNAPS: u32 = 0x30;
    /// A horizontal scrollbar along the bottom.
    pub const HSCROLL: u32 = 0x100;
    /// A vertical scrollbar along the right.
    pub const VSCROLL: u32 = 0x200;
    /// The scroll fractions snap to whole steps.
    pub const SNAP_SCROLL: u32 = 0x400;
    /// The vertical scroll offset is not applied to a point taken to the
    /// screen.
    pub const LOCK_SCROLL: u32 = 0x800;
    /// The sibling layer kept on top.
    pub const UPPER_LAYER: u32 = 0x1000;
    /// The sibling layer kept at the bottom.
    pub const LOWER_LAYER: u32 = 0x2000;
    /// The form maximises to the screen.
    pub const MAXIMISED: u32 = 0x1_0000;
    /// The x placement bit a maximise of a free form clears.
    pub const FREE_MAXIMISE: u32 = 0x4_0000;
}

/// A form's state word.
pub mod state {
    /// The form is maximised (the low two bits are 2).
    pub const MAXIMISED: u32 = 2;
    /// The parent is not a task.
    pub const NOT_TASK_CHILD: u32 = 4;
    /// The caret is lit.
    pub const CARET_LIT: u32 = 0x10;
    /// The client rectangle is out of date.
    pub const LAYOUT: u32 = 0x1000;
    /// The style and attribute words are to be applied again.
    pub const RESTYLE: u32 = 0x2000;
    /// The form has been placed in its parent.
    pub const PLACED: u32 = 0x4000;
}

/// A control record's style bits the form reads.
pub mod record_style {
    /// The record is passed over by the hit test.
    pub const NO_HIT: u32 = 2;
    /// The record's caret blinks while it has the focus.
    pub const CARET: u32 = 4;
    /// A Tab stop.
    pub const TAB_STOP: u32 = 8;
    /// The record ignores keys.
    pub const NO_KEYS: u32 = 0x10_0000;
}

/// A control record's state bits the form reads or writes.
pub mod record_state {
    pub const DISABLED: u32 = 0x10;
    /// Hidden and disabled together: the record is not drawn.
    pub const NOT_DRAWN: u32 = 0x110;
    /// The record has the focus.
    pub const FOCUSED: u32 = 0x400;
    /// The pointer's hover bits.
    pub const HOVER: u32 = 0xf_0000;
}

/// The form constructor's fields.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormDefaults {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub border: i32,
    pub content: (i32, i32),
    /// The horizontal and vertical scroll steps.
    pub step: f32,
    pub vertical_step: f32,
    /// The smallest page (height, width).
    pub min_page: (i32, i32),
    pub attributes: u32,
    pub state: u32,
    pub visible: bool,
    /// A close marks the form as closing, so a second close does nothing.
    pub closes_once: bool,
    /// The last control index (none).
    pub last_control: i32,
}

/// The original's form constructor defaults.
pub fn form_defaults() -> FormDefaults {
    FormDefaults {
        x: UNPLACED,
        y: UNPLACED,
        width: DEFAULT_WIDTH,
        height: DEFAULT_HEIGHT,
        border: DEFAULT_BORDER,
        content: DEFAULT_CONTENT,
        step: DEFAULT_SCROLL_STEP,
        vertical_step: DEFAULT_VERTICAL_STEP,
        min_page: PAGE_MIN,
        attributes: DEFAULT_ATTRIBUTES,
        state: DEFAULT_STATE,
        visible: true,
        closes_once: true,
        last_control: -1,
    }
}

/// A child form as a parent's tag searches see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaggedForm {
    pub tag: u32,
    pub visible: bool,
}

/// Finds a visible child by its type tag among a task's children (one
/// level, first first). A tag of 0 finds nothing.
pub fn find_form_by_tag(children: &[TaggedForm], tag: u32) -> Option<usize> {
    if tag == 0 {
        return None;
    }
    children
        .iter()
        .position(|child| child.tag == tag && child.visible)
}

/// The child the focus-by-tag routine activates: the first with the tag,
/// visible or not. A tag of 0 finds nothing.
pub fn focus_by_tag(tags: &[u32], tag: u32) -> Option<usize> {
    if tag == 0 {
        return None;
    }
    tags.iter().position(|candidate| *candidate == tag)
}

/// Applies a style and attribute word to a form's `state`: a form whose
/// parent is not a task gets state bit 4; a maximising form then clears
/// its attribute bit 0x20000 when it has that bit, or else gains style
/// 0x10 and attributes 0x41; the restyle bit clears and the layout bit
/// sets. Returns the style, attribute and state words.
pub fn set_style(style: u32, attributes: u32, state: u32, parent_is_task: bool) -> (u32, u32, u32) {
    let mut style = style;
    let mut attributes = attributes;
    let mut state = state;
    if !parent_is_task {
        state |= state::NOT_TASK_CHILD;
    }
    if attributes & attribute::MAXIMISED != 0 {
        if state & state::NOT_TASK_CHILD != 0 {
            attributes &= !0x2_0000;
        } else {
            style |= 0x10;
            attributes |= 0x41;
        }
    }
    state = (state & !state::RESTYLE) | state::LAYOUT;
    (style, attributes, state)
}

/// Resolves a form's unset place in its parent's client rectangle: a
/// centred x is `(right − left − width) / 2` (truncating), a right-anchored
/// x `right − width − 4`, else 4; a centred y is `(bottom − top − height)
/// >> 1` (a floor), a bottom-anchored y `bottom − height − 4`, else 36.
pub fn place_unset(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    style: u32,
    parent: ClientRect,
) -> (i32, i32) {
    let x = if x != UNPLACED {
        x
    } else if style & style::CENTRE_X != 0 {
        ((i64::from(parent.right) - i64::from(parent.left) - i64::from(width)) / 2) as i32
    } else if style & style::X_MASK == style::RIGHT {
        parent.right - width - 4
    } else {
        4
    };
    let y = if y != UNPLACED {
        y
    } else if style & style::CENTRE_Y != 0 {
        ((i64::from(parent.bottom) - i64::from(parent.top) - i64::from(height)) >> 1) as i32
    } else if style & style::Y_MASK == style::BOTTOM {
        parent.bottom - height - 4
    } else {
        36
    };
    (x, y)
}

/// A form of the show's eager layout: its place and size, its style and the
/// client rectangle its children are placed in (in its own coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct UnsetForm {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub style: u32,
    pub client: ClientRect,
    pub children: Vec<UnsetForm>,
}

/// Resolves every unset place in a tree, depth first as the original's
/// layout does: a centred x is `(client width − width) / 2` (truncating), a
/// right-anchored x `client right − width − 4`, else 4; a centred y is
/// `(client height − height) >> 1`, a bottom-anchored y
/// `client bottom − height − 4`, else 36. The form's own client is its
/// children's parent. Answers how many forms were visited.
pub fn layout_unset(form: &mut UnsetForm) -> usize {
    let parent = form.client;
    let mut visited = 1;
    for child in &mut form.children {
        let (x, y) = place_unset(
            child.x,
            child.y,
            child.width,
            child.height,
            child.style,
            parent,
        );
        child.x = x;
        child.y = y;
        visited += layout_unset(child);
    }
    visited
}

/// The place a shown form takes after its layout: a centred style is
/// re-centred in the parent's client every time, both axes truncating.
pub fn show_place(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    style: u32,
    parent: ClientRect,
) -> (i32, i32) {
    let x = if style & style::CENTRE_X != 0 {
        ((i64::from(parent.right) - i64::from(parent.left) - i64::from(width)) / 2) as i32
    } else {
        x
    };
    let y = if style & style::CENTRE_Y != 0 {
        ((i64::from(parent.bottom) - i64::from(parent.top) - i64::from(height)) / 2) as i32
    } else {
        y
    };
    (x, y)
}

/// The skin measures a form reads when it sets its controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlSkin {
    /// The left/right frame's stretch.
    pub span_x: i32,
    /// The top/bottom frame's stretch.
    pub span_y: i32,
    /// The title's height.
    pub title: i32,
    /// The scrollbars' inset.
    pub scroll: i32,
}

/// A form's client spans after its controls are set or it is restored from
/// a maximise: the border (kept at least the frame's two stretches), the
/// border remembered, the client's right and bottom edges and the title's
/// height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpanMetrics {
    pub border: i32,
    /// The remembered border; the original keeps the first border that
    /// covers the stretches and restores it on later calls.
    pub saved_border: i32,
    pub span_x: i32,
    pub span_y: i32,
    pub right: i32,
    pub bottom: i32,
    pub title: i32,
    /// The horizontal scrollbar's metrics are recomputed (attribute 0x100)
    /// and the vertical's (0x200).
    pub horizontal_scroll: bool,
    pub vertical_scroll: bool,
}

/// The client spans of a form whose controls are set: the frame's stretches
/// from the style word (0x30 across, 0xc0 down) are summed (in 64 bits)
/// and a border below that sum becomes the sum; otherwise the first border
/// is remembered in `saved_border` and later calls restore it. The
/// client's right edge is the width less the new border and, with a
/// vertical scrollbar (attribute 0x200), the skin's scroll inset; the
/// title's height is the skin's plus either the border metric 2 (attribute
/// bits 0x20 without 0x800000) or the frame's top stretch, or the new
/// border without a title (attribute 1); the bottom edge is the height less
/// the new border and, with a horizontal scrollbar (0x100), the scroll
/// inset. The original recomputes the scrollbars before it writes the new
/// bottom edge, so a caller wanting that order passes the previous bottom
/// to the scrollbar metrics.
pub fn control_spans(
    width: i32,
    height: i32,
    border: i32,
    style: u32,
    attributes: u32,
    saved_border: i32,
    skin: ControlSkin,
) -> SpanMetrics {
    let span_x = if style & 0x30 != 0 { skin.span_x } else { 0 };
    let span_y = if style & 0xc0 != 0 { skin.span_y } else { 0 };
    let (border, saved_border) = if i64::from(span_x) + i64::from(span_y) > i64::from(border) {
        (span_x.wrapping_add(span_y), saved_border)
    } else if saved_border == 0 {
        (border, border)
    } else {
        (saved_border, saved_border)
    };
    let right = if attributes & attribute::VSCROLL != 0 {
        (width - border) - skin.scroll
    } else {
        width - border
    };
    let title = if attributes & attribute::TITLE != 0 {
        skin.title
            + if attributes & 0x80_0030 == 0x20 {
                2
            } else {
                span_y
            }
    } else {
        border
    };
    let bottom = if attributes & attribute::HSCROLL != 0 {
        (height - border) - skin.scroll
    } else {
        height - border
    };
    SpanMetrics {
        border,
        saved_border,
        span_x,
        span_y,
        right,
        bottom,
        title,
        horizontal_scroll: attributes & attribute::HSCROLL != 0,
        vertical_scroll: attributes & attribute::VSCROLL != 0,
    }
}

/// What setting a form's controls does with the table it was given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlTable {
    /// An empty table (a negative count).
    Empty,
    /// The table is copied; the form owns the copy.
    Copy,
    /// The table is adopted as it is; the form does not own it.
    Adopt,
}

/// What setting a form's controls does, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetControlsPlan {
    /// The old owned table is freed first.
    pub free_old: bool,
    /// The hovered control (slot 47) is cleared.
    pub clear_hover: bool,
    pub table: ControlTable,
    /// The record count after the call (the original stores -1 first).
    pub count: i32,
    /// The style is applied again (state 0x2000).
    pub restyle: bool,
    /// The client spans are recomputed (state 0x1000, not placed): the
    /// form's own client from its skin's frame stretches.
    pub spans: bool,
    /// The client area metrics are recomputed instead (state 0x1000 and
    /// 0x4000): the title, scrollbar insets and border.
    pub metrics: bool,
    /// Each record's owner pointer is set to the form.
    pub assign_owner: bool,
    /// The control set-up pass runs over the table.
    pub setup: bool,
    /// The records are rescaled to the GUI unit (state 0x4000).
    pub rescale: bool,
}

/// The form state that decides what [`set_controls`] does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetControlsInput {
    /// The form owns its old table and has one (a non-null table).
    pub owns_old: bool,
    /// The old table's stored count, which the free is given.
    pub old_count: i32,
    /// The hovered control (0 none).
    pub hover: i32,
    pub adopt: bool,
    pub new_count: i32,
    pub state: u32,
}

/// Setting the control table: the old owned table is freed, the hover is
/// cleared, an adopted table is taken as given (its count too, negative or
/// not), else a negative count leaves no table and any other is copied
/// (owned); a restyle and a client-span or client-area pass
/// follow by the state word; every record's owner becomes the form, the
/// set-up pass runs, and a placed form rescales its records to the GUI
/// unit.
pub fn set_controls(input: SetControlsInput) -> SetControlsPlan {
    // The adopt flag is tested before the count's sign.
    let table = if input.adopt {
        ControlTable::Adopt
    } else if input.new_count < 0 {
        ControlTable::Empty
    } else {
        ControlTable::Copy
    };
    let count = if !input.adopt && input.new_count < 0 {
        -1
    } else {
        input.new_count
    };
    SetControlsPlan {
        free_old: input.owns_old,
        clear_hover: input.hover != 0,
        table,
        count,
        restyle: input.state & state::RESTYLE != 0,
        spans: input.state & state::LAYOUT != 0 && input.state & state::PLACED == 0,
        metrics: input.state & state::LAYOUT != 0 && input.state & state::PLACED != 0,
        // The owner loop runs over records 0 to the stored count.
        assign_owner: count >= 0,
        setup: true,
        rescale: input.state & state::PLACED != 0,
    }
}

/// The copy [`set_controls`] makes of a caller's table
/// ([`ControlTable::Copy`]): a new table of the records `0..=last`, each
/// copied whole (its text too), and the new last index. The original
/// reads `last + 1` records whatever the source holds; a source shorter
/// than that gives what it has, and the index follows it (Exarust's
/// guard). A negative `last` copies nothing.
pub fn copy_control_table(
    source: &[crate::gui_controls::Control],
    last: i32,
) -> (Vec<crate::gui_controls::Control>, i32) {
    let take = usize::try_from(i64::from(last) + 1)
        .unwrap_or(0)
        .min(source.len());
    let mut table = Vec::with_capacity(take);
    let last = crate::gui_controls::append_records(&mut table, -1, &source[..take]);
    (table, last)
}

/// One control drawing pass: the records to draw (index and message) and
/// the blend word the pass is drawn with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPass {
    /// The GUI blend word set before the pass (the original sets 0x10 for
    /// the back pass and 2 for the front one).
    pub blend: u32,
    /// The records drawn, in their stored order, with the message: 1 for
    /// the back pass and 2 for the front one.
    pub draws: Vec<(usize, u32)>,
}

/// The two control passes: the back pass over every drawn record, then the
/// front pass over the same records in the same order. A record whose state
/// has both 0x100 and 0x10 (hidden and disabled) is skipped by both.
pub fn control_passes(states: &[u32]) -> [ControlPass; 2] {
    let drawn = drawn_records(states);
    [
        ControlPass {
            blend: 0x10,
            draws: drawn.iter().map(|&i| (i, 1)).collect(),
        },
        ControlPass {
            blend: 2,
            draws: drawn.iter().map(|&i| (i, 2)).collect(),
        },
    ]
}

/// What drawing a form does with its fade.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormDraw {
    /// The form and its controls are drawn.
    pub paint: bool,
    /// The alpha the fade step sets for the draw (mode 0), if any.
    pub alpha: Option<f32>,
    /// The GUI alpha is set back to 1 after the draw (a fading form in
    /// mode 0).
    pub reset_alpha: bool,
}

/// The draw plan of a form at `now`: no fade draws as it is; a fade that
/// has not ended draws at the alpha its mode sets (mode 0 only); a hide
/// that has ended draws nothing, a show that has ended draws at full
/// alpha, and a form that was fading in mode 0 has its alpha reset after
/// the draw.
pub fn form_draw(fade: &mut FormFade, now: u32) -> FormDraw {
    let was_fading = fade.state != 0;
    let reset = was_fading && fade.mode == 0;
    match fade.step(now) {
        FadeStep::Close => FormDraw {
            paint: false,
            alpha: None,
            reset_alpha: false,
        },
        FadeStep::Draw(alpha) => FormDraw {
            paint: true,
            alpha,
            reset_alpha: reset,
        },
    }
}

/// What showing a form does, in order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShowPlan {
    /// A form with no parent joins the root task.
    pub attach_to_root: bool,
    /// The call asked to raise the form in its parent.
    pub raise: bool,
    /// The task takes the shown form's title when its own is empty.
    pub adopt_title: bool,
    /// The unplaced y becomes 36 before the layout.
    pub default_y: bool,
    /// The layout runs at unit 4, then the controls are rescaled to the
    /// unit (only when it is not 4).
    pub rescale_controls: bool,
    /// Re-centre x or y from the style word read before the layout.
    pub recentre_x: bool,
    pub recentre_y: bool,
    /// Rebuild the parent's snap slots (a parent that is not the root).
    pub snap_parent: bool,
}

/// The show's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowInput {
    pub has_parent: bool,
    pub raise: bool,
    pub y_unset: bool,
    pub style: u32,
    /// The form is its task's active child and the task's title is empty.
    pub adopt_title: bool,
    /// The form's parent is not the root.
    pub parent_not_root: bool,
    pub unit: i32,
}

/// The show's rule: join the root (or raise), adopt the task's empty
/// title, give an unset y 36 unless the style centres or bottom-anchors
/// it, lay out at unit 4 and rescale, re-centre, then rebuild the snap
/// slots of the parent (not the root), the form and each child that
/// [`snaps_on_show`], and redraw.
pub fn show_plan(input: ShowInput) -> ShowPlan {
    ShowPlan {
        attach_to_root: !input.has_parent,
        raise: input.has_parent && input.raise,
        adopt_title: input.adopt_title,
        default_y: input.y_unset && input.style & 0xc_8000 == 0,
        rescale_controls: input.unit != 4,
        recentre_x: input.style & style::CENTRE_X != 0,
        recentre_y: input.style & style::CENTRE_Y != 0,
        snap_parent: input.parent_not_root,
    }
}

/// Whether a child's snap slots are rebuilt when its parent is shown: it
/// is a snap container (attribute 0x20) with children.
pub fn snaps_on_show(attributes: u32, children: usize) -> bool {
    attributes & attribute::CLIENT_FILL != 0 && children > 0
}

/// The order the show's propagation pass visits a form tree: each form,
/// then its children in order (depth first). `children[i]` lists form
/// `i`'s children.
pub fn propagation_order(children: &[Vec<usize>], root: usize) -> Vec<usize> {
    let mut order = Vec::new();
    let mut stack = vec![root];
    while let Some(form) = stack.pop() {
        order.push(form);
        if let Some(kids) = children.get(form) {
            stack.extend(kids.iter().rev().copied());
        }
    }
    order
}

/// A form's place, size and words as the maximise record saves them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowRecord {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub border: i32,
    /// The form's second border word (its title spacing reserve).
    pub spacing: i32,
    pub right: i32,
    pub top: i32,
    pub bottom: i32,
    pub style: u32,
    pub attributes: u32,
    /// The GUI unit at the time.
    pub unit: i32,
}

/// A form after a maximise or restore.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WindowPlace {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub border: i32,
    pub spacing: i32,
    pub style: u32,
    pub attributes: u32,
    pub state: u32,
}

/// Maximises a form to the screen's client (`screen` is the root's right
/// and bottom): the record saves the form, the place becomes 0, 0, the
/// style takes the four anchor bits; a free form (attribute 0x40000)
/// also loses its border, spacing, attributes 0x4f and style 0xf0, the
/// others only attributes 0xc000. The state's low bits become 2, then the
/// form is resized (see [`resize_form`]).
pub fn maximise(form: WindowRecord, state: u32, screen: (i32, i32)) -> WindowPlace {
    let mut place = WindowPlace {
        x: 0,
        y: 0,
        width: screen.0,
        height: screen.1,
        border: form.border,
        spacing: form.spacing,
        style: form.style | 0xf_0000,
        attributes: form.attributes,
        state: (state & !3) | state::MAXIMISED,
    };
    if form.attributes & attribute::FREE_MAXIMISE == 0 {
        place.attributes &= !0xc000;
    } else {
        place.border = 0;
        place.spacing = 0;
        place.attributes &= 0xffff_3fb0;
        place.style &= !0xf0;
    }
    place
}

/// Restores a maximised form from its record at the current `unit`: each
/// coordinate and size is the saved one divided by the saved unit
/// (truncating) times the unit, the border `round(unit · border / saved
/// unit)` (ties to even); the spacing and words come back and the state's
/// low bits clear. `None` for a record whose unit is 0 (the original
/// divides by it).
pub fn restore(saved: WindowRecord, unit: i32, state: u32) -> Option<WindowPlace> {
    let su = saved.unit;
    if su == 0 {
        return None;
    }
    Some(WindowPlace {
        x: unit.wrapping_mul(saved.x / su),
        y: unit.wrapping_mul(saved.y / su),
        width: (i64::from(unit) * (i64::from(saved.width) / i64::from(su))) as i32,
        height: (i64::from(unit) * (i64::from(saved.height) / i64::from(su))) as i32,
        border: cvtsd2si(f64::from(unit) * (f64::from(saved.border) / f64::from(su))),
        spacing: saved.spacing,
        style: saved.style,
        attributes: saved.attributes,
        state: state & !3,
    })
}

/// The word the GUI's window-state hook is told a form was maximised with.
pub const WINDOW_HOOK_MAXIMISED: u32 = 2;
/// The word the window-state hook is told a form was restored with.
pub const WINDOW_HOOK_RESTORED: u32 = 0;

/// One step of a maximise or a restore, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowStep {
    /// The form is made visible first.
    Show,
    /// The maximise record is allocated and the form saved into it
    /// ([`WindowRecord`]), the unit last.
    SaveRecord,
    /// The GUI's window-state hook, when one is installed, is called with
    /// the form, the word and a cleared answer byte.
    StateHook(u32),
    /// The place and words of [`maximise`] or [`restore`] are written.
    Place,
    /// The state word's low bits are set to maximised (2) or cleared.
    SetState(u32),
    /// The runtime resize ([`resize_form`]) to the new size.
    Resize,
    /// The record is freed.
    FreeRecord,
    /// The control spans are refreshed ([`control_spans`]).
    ControlSpans,
    /// The form's layout-changed virtual.
    Changed,
    Redraw,
}

/// A maximise's steps: show, save the record, tell the hook (before the
/// form changes), write the place and words, set the state, resize to the
/// screen's client, then the layout-changed virtual and the redraw.
pub fn maximise_steps(hook: bool) -> Vec<WindowStep> {
    let mut steps = vec![WindowStep::Show, WindowStep::SaveRecord];
    if hook {
        steps.push(WindowStep::StateHook(WINDOW_HOOK_MAXIMISED));
    }
    steps.extend([
        WindowStep::Place,
        WindowStep::SetState(state::MAXIMISED),
        WindowStep::Resize,
        WindowStep::Changed,
        WindowStep::Redraw,
    ]);
    steps
}

/// A restore's steps: show; with a saved record, write the place and words
/// back, resize to the saved size, free the record and refresh the control
/// spans; tell the hook (whether or not a record was kept), clear the
/// state's low bits, then the layout-changed virtual and the redraw.
pub fn restore_steps(saved: bool, hook: bool) -> Vec<WindowStep> {
    let mut steps = vec![WindowStep::Show];
    if saved {
        steps.extend([
            WindowStep::Place,
            WindowStep::Resize,
            WindowStep::FreeRecord,
            WindowStep::ControlSpans,
        ]);
    }
    if hook {
        steps.push(WindowStep::StateHook(WINDOW_HOOK_RESTORED));
    }
    steps.extend([
        WindowStep::SetState(0),
        WindowStep::Changed,
        WindowStep::Redraw,
    ]);
    steps
}

/// The destination when a top-level form is raised among the root's
/// children, from their attribute words in draw order (the last is the
/// topmost): a lower-layer form goes to the first child not in the lower
/// layer (one less when that is above it; 0 when all are), an upper-layer
/// form to the top, any other below the upper layer.
pub fn raise_top_level_index(attributes: &[u32], moving: usize) -> usize {
    raise_index(attributes, moving, 0)
}

/// The destination when a child is raised within its parent. The rule is
/// [`raise_top_level_index`]'s, except that the lower-layer search starts
/// at the second child.
pub fn raise_child_index(attributes: &[u32], moving: usize) -> usize {
    raise_index(attributes, moving, 1)
}

fn raise_index(attributes: &[u32], moving: usize, first: usize) -> usize {
    let count = attributes.len();
    let Some(&own) = attributes.get(moving) else {
        return moving;
    };
    if own & attribute::LOWER_LAYER != 0 {
        let mut target = (first..count)
            .find(|&i| attributes[i] & attribute::LOWER_LAYER == 0)
            .unwrap_or(0);
        if moving < target {
            target -= 1;
        }
        target
    } else if own & attribute::UPPER_LAYER != 0 {
        count - 1
    } else {
        (0..count)
            .rev()
            .find(|&i| attributes[i] & attribute::UPPER_LAYER == 0)
            .unwrap_or(0)
    }
}

/// A child in a parent's draw order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sibling {
    pub id: u32,
    pub attributes: u32,
    /// The form this one belongs to (a popup's opener).
    pub owner: Option<u32>,
}

/// Raises the child `id` within its parent ([`raise_child_index`]), then
/// raises every child below it that it owns, each pulling its owner's
/// place down by one. Returns whether the child was found.
pub fn raise_child(children: &mut Vec<Sibling>, id: u32) -> bool {
    let Some(moving) = children.iter().position(|c| c.id == id) else {
        return false;
    };
    let attributes: Vec<u32> = children.iter().map(|c| c.attributes).collect();
    let mut target = raise_child_index(&attributes, moving);
    let child = children.remove(moving);
    children.insert(target.min(children.len()), child);
    let mut index = 0;
    while index < target {
        if children[index].owner == Some(id) {
            let owned = children[index].id;
            raise_child(children, owned);
            target -= 1;
        } else {
            index += 1;
        }
    }
    true
}

/// Raises a top-level form among the root's children
/// ([`raise_top_level_index`]); owned forms are not moved.
pub fn raise_top_level(children: &mut Vec<Sibling>, id: u32) -> bool {
    let Some(moving) = children.iter().position(|c| c.id == id) else {
        return false;
    };
    let attributes: Vec<u32> = children.iter().map(|c| c.attributes).collect();
    let target = raise_top_level_index(&attributes, moving);
    let child = children.remove(moving);
    children.insert(target.min(children.len()), child);
    true
}

/// Where a new child goes in its parent's draw order: a lower-layer child
/// just above the last lower-layer child (at the bottom when there is
/// none), an upper-layer child at the end, any other just below the
/// trailing upper-layer children.
pub fn add_child_index(attributes: &[u32], new: u32) -> usize {
    let count = attributes.len();
    if new & attribute::LOWER_LAYER != 0 {
        (0..count)
            .rev()
            .find(|&i| attributes[i] & attribute::LOWER_LAYER != 0)
            .map_or(0, |i| i + 1)
    } else if new & attribute::UPPER_LAYER != 0 {
        count
    } else {
        (0..count)
            .rev()
            .find(|&i| attributes[i] & attribute::UPPER_LAYER == 0)
            .map_or(0, |i| i + 1)
    }
}

/// Whether a form is its task's active one. A form whose attribute word is
/// exactly 0x1000 is active when it is the task's first child; any other
/// when it is the last child outside the upper layer. Visibility is not
/// looked at.
pub fn is_active(task_children: &[u32], form: usize) -> bool {
    let Some(&own) = task_children.get(form) else {
        return false;
    };
    if own == attribute::UPPER_LAYER {
        return form == 0;
    }
    (0..task_children.len())
        .rev()
        .find(|&i| task_children[i] & attribute::UPPER_LAYER == 0)
        == Some(form)
}

/// Moves an activated task to the end of the recently-active list (a
/// task not in the list is left out).
pub fn touch_task(list: &mut Vec<u32>, task: u32) {
    if let Some(at) = list.iter().position(|t| *t == task) {
        let id = list.remove(at);
        list.push(id);
    }
}

/// The root's child the focus falls back to when the active task closes:
/// the last child that is a task and visible; `None` means the root.
pub fn fallback_focus(children: &[(bool, bool)]) -> Option<usize> {
    (0..children.len())
        .rev()
        .find(|&i| children[i].0 && children[i].1)
}

/// What closing a form does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ClosePlan {
    /// Mark the form as closing (and raise the GUI's "a form closed" flag).
    pub mark_closing: bool,
    /// Tell the GUI main that its tracked form closes (message 0x800).
    pub notify_main: bool,
    pub hide_tooltip: bool,
    /// The form is its task's active child: the callback runs, the task's
    /// active child clears and the task closes instead.
    pub close_task: bool,
    /// Otherwise: detach from the task, close the task's children owned by
    /// this form and this form's children (those not closing yet), hide,
    /// run the callback and redraw.
    pub close_owned: bool,
    pub run_callback: bool,
    pub redraw: bool,
}

/// The close's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CloseInput {
    /// The form is already closing.
    pub closing: bool,
    /// The form closes once (the constructor sets it).
    pub closes_once: bool,
    /// The GUI main tracks this form.
    pub tracked_by_main: bool,
    pub is_tooltip: bool,
    pub is_active_child: bool,
    pub has_callback: bool,
}

/// The close's rule. A form already closing does nothing; the result is
/// always 1.
pub fn close_plan(input: CloseInput) -> ClosePlan {
    if input.closing {
        return ClosePlan::default();
    }
    let mut plan = ClosePlan {
        mark_closing: input.closes_once,
        notify_main: input.tracked_by_main,
        hide_tooltip: input.is_tooltip,
        run_callback: input.has_callback,
        ..ClosePlan::default()
    };
    if input.is_active_child {
        plan.close_task = true;
    } else {
        plan.close_owned = true;
        plan.redraw = true;
    }
    plan
}

/// The style bit whose forms restore the controller's action set when they
/// close (a Steam Input call, platform glue in Exarust).
pub const STYLE_CONTROLLER_ACTIONS: u32 = 0x20;

/// One form as the close cascade sees it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CloseNode {
    pub style: u32,
    pub closing: bool,
    /// The form marks itself closing (the constructor's choice).
    pub closes_once: bool,
    /// The form whose popup this one is.
    pub owner: Option<usize>,
    /// The form's task (itself for a task).
    pub task: usize,
    /// The form's children, in their list's order.
    pub children: Vec<usize>,
    pub has_callback: bool,
    /// For a task: its active child.
    pub active_child: Option<usize>,
    /// For a task: the child it remembers to refocus.
    pub remembered: Option<usize>,
}

/// One step of a close cascade, in the order the original takes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloseStep {
    /// The controller's action set is restored (style bit 0x20); this runs
    /// even for a form already closing.
    ControllerActions(usize),
    /// The GUI's closing flag is raised (a form that closes once).
    ClosingFlag,
    Callback(usize),
    /// The task of an active child is closed in its place (the task class's
    /// own close).
    CloseTask(usize),
    Hide(usize),
    Redraw,
}

/// Closes `form` and what it takes with it. A form already closing does
/// nothing more than the controller step. The task's active child runs its
/// callback, clears the task's active child and closes the task. Any other
/// form clears the task's remembered child when it is this one, then walks
/// the task's children (the count read once, before the walk) closing each
/// not yet closing whose owner is this form, then its own children not yet
/// closing, then hides, runs its callback and redraws. Closing never
/// removes a form from a list, so the walks see every entry; a form that
/// does not close once can be closed again by a later walk, as in the
/// original. The GUI main's notice and the tooltip's hide are
/// [`close_plan`]'s. A malformed tree that links back into itself stops
/// after [`CLOSE_DEPTH`] nested closes instead of recursing for ever.
pub fn close_cascade(nodes: &mut [CloseNode], form: usize, steps: &mut Vec<CloseStep>) {
    close_nested(nodes, form, steps, 0);
}

/// How deep a close cascade nests before it stops (Exarust's guard).
pub const CLOSE_DEPTH: usize = 64;

fn close_nested(nodes: &mut [CloseNode], form: usize, steps: &mut Vec<CloseStep>, depth: usize) {
    if depth >= CLOSE_DEPTH {
        return;
    }
    let Some(node) = nodes.get(form) else {
        return;
    };
    if node.style & STYLE_CONTROLLER_ACTIONS != 0 {
        steps.push(CloseStep::ControllerActions(form));
    }
    if node.closing {
        return;
    }
    if node.closes_once {
        nodes[form].closing = true;
        steps.push(CloseStep::ClosingFlag);
    }
    let task = nodes[form].task;
    let callback = nodes[form].has_callback;
    if task < nodes.len() && nodes[task].active_child == Some(form) {
        if callback {
            steps.push(CloseStep::Callback(form));
        }
        nodes[task].active_child = None;
        steps.push(CloseStep::CloseTask(task));
        return;
    }
    if task < nodes.len() {
        if nodes[task].remembered == Some(form) {
            nodes[task].remembered = None;
        }
        let count = nodes[task].children.len();
        for index in 0..count {
            let Some(&child) = nodes[task].children.get(index) else {
                break;
            };
            if nodes
                .get(child)
                .is_some_and(|c| !c.closing && c.owner == Some(form))
            {
                close_nested(nodes, child, steps, depth + 1);
            }
        }
    }
    let count = nodes[form].children.len();
    for index in 0..count {
        let Some(&child) = nodes[form].children.get(index) else {
            break;
        };
        if nodes.get(child).is_some_and(|c| !c.closing) {
            close_nested(nodes, child, steps, depth + 1);
        }
    }
    steps.push(CloseStep::Hide(form));
    if callback {
        steps.push(CloseStep::Callback(form));
    }
    steps.push(CloseStep::Redraw);
}

/// What changing a form's visibility does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct VisibilityPlan {
    pub changed: bool,
    /// The task follows its active child's visibility.
    pub task_follows: bool,
    /// Showing: the form is activated.
    pub activate: bool,
    /// Hiding: the hide hook runs, the form's hidden virtual is told, the
    /// GUI's references to it clear and the screen redraws.
    pub clear_references: bool,
}

/// The visibility rule from the current and wanted visibility.
pub fn visibility_plan(visible: bool, wanted: bool, is_active_child: bool) -> VisibilityPlan {
    if visible == wanted {
        return VisibilityPlan::default();
    }
    VisibilityPlan {
        changed: true,
        task_follows: is_active_child,
        activate: wanted && !is_active_child,
        clear_references: !wanted && !is_active_child,
    }
}

/// The destroy's work: the before-destroy hook when the caller frees the
/// object, the children, the control records when the form owns them (each
/// record's text), the snap slots, the name, the focus when it was this
/// form (back to the root) and the maximise record.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DestroyPlan {
    pub before_hook: bool,
    pub children_freed: usize,
    pub records_freed: usize,
    pub focus_to_root: bool,
    pub frees_object: bool,
}

/// Plans a destroy from the form's counts and its ownership.
pub fn destroy_plan(
    children: usize,
    last_control: i32,
    owns_records: bool,
    focused: bool,
    caller_frees: bool,
) -> DestroyPlan {
    DestroyPlan {
        before_hook: caller_frees,
        children_freed: children,
        records_freed: if owns_records {
            (last_control + 1).max(0) as usize
        } else {
            0
        },
        focus_to_root: focused,
        frees_object: caller_frees,
    }
}

/// A control record for the runtime resize: its 16-bit place and size and
/// its style word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResizeControl {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
    pub style: u32,
    /// Its owner's client rectangle after the resize, for the centred
    /// placements.
    pub client: ClientRect,
}

/// A form for the runtime resize: its place, size, style, children and
/// records.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResizeNode {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub style: u32,
    pub has_parent: bool,
    pub children: Vec<ResizeNode>,
    pub controls: Vec<ResizeControl>,
}

/// Resizes a form in place: a form with a parent that is right- or
/// bottom-anchored keeps its far edge; a child that stretches takes the
/// change and is resized itself, a right/bottom-anchored child moves by
/// it, and a child that did not stretch is re-centred by its style
/// (truncating). The records follow the same rules in 16-bit arithmetic,
/// the centred ones in their owner's client. The original then calls the
/// form's layout and, when it has snap slots, rebuilds them (the caller's).
pub fn resize_form(form: &mut ResizeNode, new_width: i32, new_height: i32) {
    let (old_w, old_h) = (form.width, form.height);
    if form.has_parent {
        if form.style & style::X_MASK == style::RIGHT {
            form.x = form.width + form.x - new_width;
        }
        if form.style & style::Y_MASK == style::BOTTOM {
            form.y = form.height + form.y - new_height;
        }
    }
    for child in &mut form.children {
        let mut stretched = false;
        let child_w = if child.style & style::X_MASK == style::X_MASK {
            stretched = true;
            child.width + new_width - old_w
        } else {
            if child.style & style::X_MASK == style::RIGHT {
                child.x = child.x + new_width - old_w;
            }
            child.width
        };
        let child_h = if child.style & style::Y_MASK == style::Y_MASK {
            stretched = true;
            child.height + new_height - old_h
        } else {
            if child.style & style::Y_MASK == style::BOTTOM {
                child.y = child.y + new_height - old_h;
            }
            child.height
        };
        if stretched {
            resize_form(child, child_w, child_h);
        } else {
            if child.style & style::CENTRE_X != 0 {
                child.x = ((i64::from(new_width) - i64::from(child.width)) / 2) as i32;
            }
            if child.style & style::CENTRE_Y != 0 {
                child.y = ((i64::from(new_height) - i64::from(child.height)) / 2) as i32;
            }
        }
    }
    form.width = new_width;
    form.height = new_height;
    let (dw, dh) = (new_width as i16, new_height as i16);
    let (ow, oh) = (old_w as i16, old_h as i16);
    for c in &mut form.controls {
        let mut stretched = false;
        let w = if c.style & style::X_MASK == style::X_MASK {
            stretched = true;
            c.width.wrapping_add(dw).wrapping_sub(ow)
        } else {
            if c.style & style::X_MASK == style::RIGHT {
                c.x = c.x.wrapping_add(dw).wrapping_sub(ow);
            }
            c.width
        };
        let h = if c.style & style::Y_MASK == style::Y_MASK {
            stretched = true;
            c.height.wrapping_add(dh).wrapping_sub(oh)
        } else {
            if c.style & style::Y_MASK == style::BOTTOM {
                c.y = c.y.wrapping_add(dh).wrapping_sub(oh);
            }
            c.height
        };
        if stretched {
            c.width = w;
            c.height = h;
        } else {
            if c.style & style::CENTRE_X != 0 {
                c.x = ((i64::from(c.client.right) - i64::from(c.client.left) - i64::from(c.width))
                    / 2) as i16;
            }
            if c.style & style::CENTRE_Y != 0 {
                c.y = ((i64::from(c.client.bottom) - i64::from(c.client.top) - i64::from(c.height))
                    / 2) as i16;
            }
        }
    }
}

/// The 1-based control code's index (its low 16 bits less one), when the
/// code is positive and the index is at most `last`. A code whose low half
/// is 0 gives none.
pub fn control_index(code: i32, last: i32) -> Option<usize> {
    if code <= 0 {
        return None;
    }
    let index = (code & 0xffff) - 1;
    (index >= 0 && index <= last).then_some(index as usize)
}

/// A form's control ids and its children, for the find-by-id search.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ControlTree {
    pub ids: Vec<u16>,
    pub children: Vec<ControlTree>,
}

/// Finds the first record with `id`: the form's own records first, then
/// each child's tree depth first. Returns the child path and the record.
pub fn find_control(tree: &ControlTree, id: u16) -> Option<(Vec<usize>, usize)> {
    if let Some(record) = tree.ids.iter().position(|candidate| *candidate == id) {
        return Some((Vec::new(), record));
    }
    tree.children.iter().enumerate().find_map(|(i, child)| {
        find_control(child, id).map(|(mut path, record)| {
            path.insert(0, i);
            (path, record)
        })
    })
}

/// A message sent to a control by its id: the three values the control's
/// handler takes after the record (the message code and its two words).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlMessage {
    pub message: u32,
    pub first: u32,
    pub second: u32,
}

/// Sends a message to the first record with `id`, anywhere in the tree
/// (the original's find-by-code followed by the control's message slot).
/// Without a record the original's default control takes the message and
/// ignores it, so the answer is none.
pub fn send_control(
    tree: &ControlTree,
    id: u16,
    message: ControlMessage,
) -> Option<(Vec<usize>, usize, ControlMessage)> {
    find_control(tree, id).map(|(path, record)| (path, record, message))
}

/// The centre of a form's client for a control code: none for a code
/// below 1 other than -1; otherwise the client's spans over two,
/// truncating.
pub fn content_centre(code: i32, client: ClientRect) -> Option<(i32, i32)> {
    if code != -1 && code < 1 {
        return None;
    }
    Some((
        ((i64::from(client.right) - i64::from(client.left)) / 2) as i32,
        ((i64::from(client.bottom) - i64::from(client.top)) / 2) as i32,
    ))
}

/// A form's geometry as the point conversions read it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormFrame {
    pub x: i32,
    pub y: i32,
    pub client: ClientRect,
    pub content: (i32, i32),
    pub scroll: (f32, f32),
    pub attributes: u32,
}

impl FormFrame {
    /// The scroll offset: each axis' overflow over the client times its
    /// fraction, rounded with `CVTSS2SI`.
    pub fn scroll_offset(&self) -> (i32, i32) {
        let ox = i64::from(self.content.0)
            - (i64::from(self.client.right) - i64::from(self.client.left));
        let oy = i64::from(self.content.1)
            - (i64::from(self.client.bottom) - i64::from(self.client.top));
        (
            cvtss2si(ox as f32 * self.scroll.0),
            cvtss2si(oy as f32 * self.scroll.1),
        )
    }

    /// A form-local point in content coordinates: less the client's inset,
    /// plus the scroll offset.
    pub fn local_to_content(&self, (x, y): (i32, i32)) -> (i32, i32) {
        let (sx, sy) = self.scroll_offset();
        (sx + (x - self.client.left), sy + (y - self.client.top))
    }

    /// The control-at-point helper's conversion: plus the client's inset
    /// and the scroll offset.
    pub fn offset_point(&self, (x, y): (i32, i32)) -> (i32, i32) {
        let (sx, sy) = self.scroll_offset();
        (sx + x + self.client.left, sy + y + self.client.top)
    }
}

/// A root point in a form's own coordinates: less the form's place, then
/// through each ancestor below the root (innermost first) less its place
/// and inset and plus its scroll offset.
pub fn root_to_local(point: (i32, i32), form: (i32, i32), ancestors: &[FormFrame]) -> (i32, i32) {
    let (mut x, mut y) = (point.0 - form.0, point.1 - form.1);
    for a in ancestors {
        let (sx, sy) = a.scroll_offset();
        x = sx + ((x - a.x) - a.client.left);
        y = sy + ((y - a.y) - a.client.top);
    }
    (x, y)
}

/// Takes a form-local point to the root through the form and its
/// ancestors below the root (innermost first): x adds the border and the
/// place; y adds the client top, less the vertical scroll offset unless
/// the attribute word locks it, then the place. The horizontal scroll is
/// not applied.
pub fn local_to_screen(mut point: (i32, i32), forms: &[FormFrame]) -> (i32, i32) {
    for form in forms {
        point.0 = form.client.left + point.0 + form.x;
        if form.attributes & attribute::LOCK_SCROLL != 0 {
            point.1 += form.client.top;
        } else {
            point.1 = (point.1 + form.client.top) - form.scroll_offset().1;
        }
        point.1 += form.y;
    }
    point
}

/// A control record as the form's hover, focus, Tab and hit test see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitControl {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
    pub style: u32,
    pub state: u32,
    /// The record's answer to the hit query: its 0x7f0000 bits pass back
    /// to the form, its high byte is the cursor, and 0x800000 lets the
    /// point through.
    pub answer: u32,
}

/// A form for the hit test.
#[derive(Debug, Clone, PartialEq)]
pub struct HitNode {
    pub id: u32,
    pub frame: FormFrame,
    pub width: i32,
    pub height: i32,
    pub visible: bool,
    pub style: u32,
    /// The answer of the skin's frame hit test for points off the client
    /// (0 for none).
    pub frame_code: u32,
    pub children: Vec<HitNode>,
    pub controls: Vec<HitControl>,
}

/// The hit test's context: the skin's cursor (-2: the controls do not set
/// it), the control slop (two units, off in some modes), the unit and
/// whether a modal task elsewhere blocks this form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitContext {
    pub cursor: i32,
    pub slop: bool,
    pub unit: i32,
    pub blocked: bool,
}

/// What a hit test found: the form, its code and the cursor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TreeHit {
    pub form: Option<u32>,
    /// A 1-based control code with its answer bits, -1 for the client, a
    /// frame code, or 0 for nothing.
    pub code: u32,
    pub cursor: i32,
}

/// Whether a point hits a control's rectangle. Without the slop the
/// rectangle's edges are inclusive; with it, a point within two units of
/// it (the squared distance strictly below the squared radius) hits.
pub fn control_hit(x: i32, y: i32, rect: (i32, i32, i32, i32), slop: bool, unit: i32) -> bool {
    let (left, top, right, bottom) = rect;
    if !slop {
        return !(x < left || right < x || y < top || bottom < y);
    }
    let dx = if x < left {
        left - x
    } else if right < x {
        x - right
    } else {
        0
    };
    let dy = if y < top {
        top - y
    } else if bottom < y {
        y - bottom
    } else {
        0
    };
    let r = i64::from(unit) * 2 * i64::from(unit) * 2;
    let d = i64::from(dy) * i64::from(dy) + i64::from(dx) * i64::from(dx);
    r != d && d <= r
}

/// The cursor shape a resize frame code asks for (1 horizontal, 2
/// vertical, 3 and 4 the diagonals), or `None`.
pub fn resize_cursor(code: u32) -> Option<i32> {
    match code {
        0x8000_0011 | 0x8000_0012 => Some(2),
        0x8000_0014 | 0x8000_0018 => Some(1),
        0x8000_0015 | 0x8000_001a => Some(3),
        0x8000_0016 | 0x8000_0019 => Some(4),
        _ => None,
    }
}

/// The base form's hit test at a form-local point. Off the client the
/// skin's frame code answers. On it, the visible children not marked
/// [`style::NO_HIT`] are tried last first, each when the content point is
/// strictly inside it; then the records, first without the slop and then
/// (when the slop is on) with it, each when it is neither disabled nor
/// [`record_style::NO_HIT`] and its answer does not pass the point
/// through. A record answers its 1-based index with its answer's 0x7f0000
/// bits (0x10000 when it has none) and its cursor. A client hit with no
/// record is -1, or 0 for a form whose attribute word has no 0x30 bits.
pub fn hit_test(node: &HitNode, x: i32, y: i32, ctx: HitContext) -> TreeHit {
    if ctx.blocked {
        return TreeHit::default();
    }
    let client = node.frame.client;
    let mut code = 0u32;
    let mut content = (x, y);
    if client.left <= x && x < client.right && client.top <= y && y < client.bottom {
        content = node.frame.local_to_content((x, y));
        for child in node.children.iter().rev() {
            let (cx, cy) = content;
            let inside = child.frame.x < cx
                && child.frame.y < cy
                && i64::from(cx) < i64::from(child.frame.x) + i64::from(child.width)
                && i64::from(cy) < i64::from(child.height) + i64::from(child.frame.y);
            if child.visible && child.style & style::NO_HIT == 0 && inside {
                let hit = hit_test(child, cx - child.frame.x, cy - child.frame.y, ctx);
                if hit.code != 0 {
                    return hit;
                }
            }
        }
        code = u32::MAX;
    }
    let mut cursor = ctx.cursor;
    if code == 0 {
        code = node.frame_code;
    }
    if ctx.cursor != -2
        && let Some(shape) = resize_cursor(code)
    {
        cursor = shape;
    }
    if code == u32::MAX {
        let passes: &[bool] = if ctx.slop { &[false, true] } else { &[false] };
        for &slop in passes {
            for (i, c) in node.controls.iter().enumerate() {
                let rect = (
                    i32::from(c.x),
                    i32::from(c.y),
                    i32::from(c.width) + i32::from(c.x),
                    i32::from(c.height) + i32::from(c.y),
                );
                if c.state & record_state::DISABLED != 0
                    || c.style & record_style::NO_HIT != 0
                    || !control_hit(content.0, content.1, rect, slop, ctx.unit)
                    || c.answer & 0x80_0000 != 0
                {
                    continue;
                }
                let mut answer = c.answer;
                if answer & 0x7f_0000 == 0 {
                    answer |= 0x1_0000;
                }
                if ctx.cursor != -2 {
                    cursor = (answer >> 24) as i32;
                    if cursor == 0 {
                        cursor = ctx.cursor;
                    }
                }
                return TreeHit {
                    form: Some(node.id),
                    code: (i as u32 + 1) | (answer & 0x7f_0000),
                    cursor,
                };
            }
        }
        if node.frame.attributes & attribute::SNAPS == 0 {
            code = 0;
        }
    }
    TreeHit {
        form: Some(node.id),
        code,
        cursor,
    }
}

/// Whether a code is one of the frame's close-band codes the hover redraws
/// for: -21, -20, -14, -13, -10, -9, -6, -5, -4, and -8 when the skin's
/// first flag bit is set.
pub fn is_close_edge(code: i32, thin_frame: bool) -> bool {
    match code {
        -21..=-20 | -14..=-13 | -10..=-9 | -6..=-4 => true,
        -8 => thin_frame,
        _ => false,
    }
}

/// A hover change on a form's records: the old record's hover bits clear
/// and it is sent the leave (message 0x20 with 0); a new valid record
/// takes the word's hover bits and is sent message 0x20 with its 0x7f0000
/// bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HoverChange {
    pub leave: Option<usize>,
    pub enter: Option<(usize, u32)>,
    /// The frame's close band was or is hovered: the result is 3.
    pub edge_redraw: bool,
}

/// Moves the hover from the stored word `old` to `new`, updating the
/// records' state bits. The form stores `new` afterwards.
pub fn hover_change(states: &mut [u32], old: u32, new: u32, thin_frame: bool) -> HoverChange {
    let last = states.len() as i32 - 1;
    let mut change = HoverChange::default();
    if let Some(i) = control_index(old as i32, last) {
        states[i] &= !record_state::HOVER;
        change.leave = Some(i);
    }
    if (new as i32) < 1 {
        change.edge_redraw =
            is_close_edge(old as i32, thin_frame) || is_close_edge(new as i32, thin_frame);
    } else if let Some(i) = control_index(new as i32, last) {
        states[i] |= new & record_state::HOVER;
        change.enter = Some((i, new & 0x7f_0000));
    }
    change
}

/// A focus change on a form's records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FocusChange {
    /// The old record loses the focus bit and is sent message 0xb0 with 0.
    pub blur: Option<usize>,
    /// The new record gains it and is sent message 0xb0 with the word's
    /// 0x7f0000 bits.
    pub focus: Option<(usize, u32)>,
}

/// Moves the focus from `current` to `new` (1-based words, 0 none): `None`
/// when they are equal (result 0); otherwise the result is 3 with the
/// records' answers, the form's caret bit is set for any non-zero word
/// and cleared for 0, and the caret's blink counter restarts.
pub fn focus_change(
    states: &mut [u32],
    form_state: &mut u32,
    current: u32,
    new: u32,
) -> Option<FocusChange> {
    if new == current {
        return None;
    }
    let last = states.len() as i32 - 1;
    let mut change = FocusChange::default();
    if let Some(i) = control_index(current as i32, last) {
        states[i] &= !record_state::FOCUSED;
        change.blur = Some(i);
    }
    if new == 0 {
        *form_state &= !state::CARET_LIT;
    } else {
        if let Some(i) = control_index(new as i32, last) {
            states[i] |= record_state::FOCUSED;
            change.focus = Some((i, new & 0x7f_0000));
        }
        *form_state |= state::CARET_LIT;
    }
    Some(change)
}

/// The caret blink: while the focused record's style (or the form's own,
/// when no record has the focus) has the caret bit, the shared counter
/// counts frames and every fifth flips the form's caret bit. Returns
/// whether the form redraws.
pub fn caret_blink(caret_style: u32, counter: &mut u32, form_state: &mut u32) -> bool {
    if caret_style & style::CARET == 0 {
        return false;
    }
    *counter += 1;
    if *counter > 4 {
        *counter = 0;
        *form_state ^= state::CARET_LIT;
        return true;
    }
    false
}

/// A record's Tab bits: its style's tab stop and its state's disabled bit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlTab {
    pub tab_stop: bool,
    pub disabled: bool,
}

/// The record Tab moves the focus to from the focused one: forward, the
/// next enabled tab stop, else (wrapping) the first tab stop before it;
/// backward (Shift, the key flags' 0x10), the previous enabled tab stop,
/// else the last tab stop after it. The wrap does not look at the
/// disabled bit. A form with no focused record passes the key on.
pub fn tab_target(controls: &[ControlTab], current: usize, backward: bool) -> Option<usize> {
    if current >= controls.len() {
        return None;
    }
    let ok = |i: usize| controls[i].tab_stop && !controls[i].disabled;
    if !backward {
        ((current + 1)..controls.len())
            .find(|&i| ok(i))
            .or_else(|| (0..current).find(|&i| controls[i].tab_stop))
    } else {
        (0..current).rev().find(|&i| ok(i)).or_else(|| {
            ((current + 1)..controls.len())
                .rev()
                .find(|&i| controls[i].tab_stop)
        })
    }
}

/// What the base form's key handler did with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormKey {
    /// Tab moved the focus to this record.
    Focus(usize),
    /// The focused record took the key (its answer had both low bits); the
    /// GUI is asked to redraw.
    Taken(usize),
    /// Nobody took it: it goes on to the parent form's key slot.
    Parent,
    /// The key ends here.
    Nothing,
}

/// The base form's key handler. With a focused record (a positive focus
/// code naming a record): Tab moves the focus to [`tab_target`] (Shift,
/// the key flags' 0x10, goes backward), and a Tab with nowhere to go ends
/// there; any other key is offered to the focused record (message 0xa0),
/// whose `answer` with both low bits set takes it. A key nobody took goes
/// to the parent's key slot unless its flags have a low bit (a repeat, 1,
/// or a typed character, 2).
pub fn form_key(
    key: u32,
    flags: u32,
    focus: i32,
    controls: &[ControlTab],
    answer: impl FnOnce(usize) -> u32,
) -> FormKey {
    let last = controls.len() as i32 - 1;
    if last >= 0
        && let Some(index) = control_index(focus, last)
    {
        if key == 9 {
            return tab_target(controls, index, flags & 0x10 != 0)
                .map_or(FormKey::Nothing, FormKey::Focus);
        }
        if answer(index) & 3 == 3 {
            return FormKey::Taken(index);
        }
    }
    if flags & 3 == 0 {
        FormKey::Parent
    } else {
        FormKey::Nothing
    }
}

/// Where a key, press or drag goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// The record at this index, with the message.
    Control(usize, u32),
    /// The form's parent (the form's flag word has 0x80).
    Parent,
    /// The payload's own form (a drop's payload code).
    Payload,
    Nothing,
}

/// A key sent to a control code: a valid record gets message 0x2000 unless
/// it ignores keys; an invalid code goes to the parent when the form
/// passes codes up.
pub fn route_key(
    code: u32,
    last: i32,
    passes_up: bool,
    record_style: impl Fn(usize) -> u32,
) -> Route {
    match control_index((code & 0xffff) as i32, last) {
        Some(i) if record_style(i) & self::record_style::NO_KEYS == 0 => Route::Control(i, 0x2000),
        Some(_) => Route::Nothing,
        None if passes_up => Route::Parent,
        None => Route::Nothing,
    }
}

/// A press sent to a control code: the drop payload code 0x80000000 goes
/// to the payload's form; a valid record gets message 0x3000; an invalid
/// code goes to the parent when the form passes codes up.
pub fn route_press(code: u32, last: i32, passes_up: bool) -> Route {
    if code == 0x8000_0000 {
        return Route::Payload;
    }
    match control_index((code & 0xffff) as i32, last) {
        Some(i) => Route::Control(i, 0x3000),
        None if passes_up => Route::Parent,
        None => Route::Nothing,
    }
}

/// A drag sent to the held record: message 0x4000 when the held code names
/// a record and the drag word is not `i32::MIN`.
pub fn route_drag(held: u32, last: i32, word: i32) -> Route {
    if word == i32::MIN {
        return Route::Nothing;
    }
    let index = (held & 0xffff) as i32 - 1;
    if index >= 0 && index <= last {
        Route::Control(index as usize, 0x4000)
    } else {
        Route::Nothing
    }
}

/// A pointer move sent to the held record: message 0x30 with the form-local
/// point packed as x in the low half and y in the high half (both 16-bit).
/// Only a positive held code whose low 16 bits less one are not past the
/// last record is sent; the record is the whole code less one.
pub fn held_move(held: i32, last: i32, point: (i32, i32)) -> Option<(usize, u32, i32)> {
    if held <= 0 || i64::from(held & 0xffff) - 1 > i64::from(last) {
        return None;
    }
    let packed = ((point.0 & 0xffff) as u32 | ((point.1 & 0xffff) as u32) << 16) as i32;
    Some((held as usize - 1, 0x30, packed))
}

/// The place a title-bar drag asks for: the grab saved at the press plus
/// the pointer's move. The form's x and y are not clamped by the move (the
/// pointer was clamped before it reached the form).
pub fn title_drag(grab: (i32, i32), pointer: (i32, i32), task: (i32, i32)) -> (i32, i32) {
    (
        grab.0.wrapping_add(pointer.0).wrapping_sub(task.0),
        grab.1.wrapping_add(pointer.1).wrapping_sub(task.1),
    )
}

/// The scroll a wheel turn sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WheelScroll {
    Vertical(f32),
    Horizontal(f32),
    None,
}

/// The wheel: a form with a vertical scrollbar moves its vertical fraction
/// by `round(delta / 120)` vertical steps, else one with a horizontal
/// scrollbar its horizontal fraction by `round(−delta / 120)` horizontal
/// steps (both turn the same way); the conversion is in double, ties to
/// even. The setters then clamp and snap.
pub fn wheel_scroll(
    attributes: u32,
    delta: i32,
    scroll: (f32, f32),
    steps: (f32, f32),
) -> WheelScroll {
    if attributes & attribute::VSCROLL != 0 {
        let notches = cvtsd2si64(f64::from(delta) * 0.008_333_333_333_333_333) as f32;
        WheelScroll::Vertical(scroll.1 - notches * steps.1)
    } else if attributes & attribute::HSCROLL != 0 {
        let notches = cvtsd2si64(f64::from(delta) * -0.008_333_333_333_333_333) as f32;
        WheelScroll::Horizontal(notches * steps.0 + scroll.0)
    } else {
        WheelScroll::None
    }
}

fn cvtsd2si64(v: f64) -> i64 {
    let r = v.round_ties_even();
    if r.is_nan() || !(-9.223_372_036_854_776e18..9.223_372_036_854_776e18).contains(&r) {
        i64::MIN
    } else {
        r as i64
    }
}

/// What a held scroll arrow repeats each tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Repeat {
    Vertical(f32),
    Horizontal(f32),
    /// A held record (1-based code) is sent message 0x100.
    Control(usize),
    None,
}

/// The repeat of a held frame code while the pointer is over it: -10 and
/// -9 the vertical arrows (down and up a vertical step), -13 and -14 the
/// horizontal ones (left and right a horizontal step); a held record gets
/// its tick.
pub fn repeat_scroll(held: i32, over: bool, scroll: (f32, f32), steps: (f32, f32)) -> Repeat {
    if held >= 1 {
        return Repeat::Control(held as usize - 1);
    }
    if !over {
        return Repeat::None;
    }
    match held {
        -10 => Repeat::Vertical(scroll.1 + steps.1),
        -9 => Repeat::Vertical(scroll.1 - steps.1),
        -13 => Repeat::Horizontal(scroll.0 - steps.0),
        -14 => Repeat::Horizontal(scroll.0 + steps.0),
        _ => Repeat::None,
    }
}

/// The vertical scrollbar's track and thumb.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerticalScroll {
    /// The track's top and bottom.
    pub start: i32,
    pub end: i32,
    pub thumb: i32,
    pub thumb_y: i32,
    /// The stored fraction (snapped when the attribute word asks).
    pub fraction: f32,
}

/// The vertical scrollbar's layout: the track runs from the client's top
/// plus the rail to the form's height less its border and the rail; the
/// thumb is `round(track · client / content)` (ties to even), `2·unit + 1`
/// when not above `2·unit`, at most the track; a snapping form snaps the
/// stored fraction (to 0 for a non-positive step); the thumb sits at
/// `start + round((track − thumb) · f)`.
#[allow(clippy::too_many_arguments)]
pub fn vertical_scroll_metrics(
    height: i32,
    border: i32,
    client: ClientRect,
    content_h: i32,
    rail: i32,
    unit: i32,
    attributes: u32,
    fraction: f32,
    step: f32,
) -> VerticalScroll {
    let start = client.top + rail;
    let end = (height - border) - rail;
    let track = end - start;
    let mut thumb =
        cvtsd2si(f64::from(track) * (f64::from(client.bottom - client.top) / f64::from(content_h)));
    if unit * 2 < thumb {
        if track < thumb {
            thumb = track;
        }
    } else {
        thumb = unit * 2 + 1;
    }
    let mut f = fraction;
    if attributes & attribute::SNAP_SCROLL != 0 {
        f = if step <= 0.0 {
            0.0
        } else {
            cvtss2si64(f / step) as f32 * step
        };
    }
    let thumb_y = start + cvtss2si((i64::from(track) - i64::from(thumb)) as f32 * f);
    VerticalScroll {
        start,
        end,
        thumb,
        thumb_y,
        fraction: f,
    }
}

/// Sets the vertical scroll fraction: nothing happens when the clamped
/// value equals the stored one (`None`); otherwise it is snapped when the
/// attribute word asks, stored as 0 when the content fits the client, and
/// the thumb is placed from the stored fraction. Returns the fraction and
/// the thumb's y.
#[allow(clippy::too_many_arguments)]
pub fn set_scroll_y(
    f: f32,
    current: f32,
    step: f32,
    attributes: u32,
    client: ClientRect,
    content_h: i32,
    track: (i32, i32),
    thumb: i32,
) -> Option<(f32, i32)> {
    let f = ordered_clamp(f);
    if f == current {
        return None;
    }
    let mut stored = if attributes & attribute::SNAP_SCROLL != 0 {
        cvtss2si64(f / step) as f32 * step
    } else {
        f
    };
    if i64::from(content_h) <= i64::from(client.bottom) - i64::from(client.top) {
        stored = 0.0;
    }
    let y =
        cvtss2si(((i64::from(track.1) - i64::from(track.0)) - i64::from(thumb)) as f32 * stored)
            + track.0;
    Some((stored, y))
}

/// The fraction a dragged horizontal thumb asks for: the pointer's form x
/// less the rail and border and the grab offset, over the track left for
/// the thumb (in double).
pub fn drag_thumb_x(
    x: i32,
    grab: i32,
    border: i32,
    client_right: i32,
    rail: i32,
    thumb: i32,
) -> f32 {
    let x = x - (rail + border);
    let span = f64::from((client_right - border) + rail * -2) - f64::from(thumb);
    ((f64::from(x) - f64::from(grab)) / span) as f32
}

/// The fraction a dragged vertical thumb asks for: the pointer's form y
/// less the track's start and the grab offset, over the track left for the
/// thumb; 0 when the thumb fills the track.
pub fn drag_thumb_y(y: i32, grab: i32, track: (i32, i32), thumb: i32) -> f32 {
    let span = track.1 - track.0;
    if thumb < span {
        ((f64::from(y - track.0) - f64::from(grab)) / f64::from(span - thumb)) as f32
    } else {
        0.0
    }
}

/// A form's place and size during a grip drag, and the minimum page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GripDrag {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    /// The grab saved by the press: the form's x (left grip) or width
    /// (right grip), and its y (top grip) or height (bottom grip).
    pub grab: (i32, i32),
    /// The pointer's move since the press.
    pub moved: (i32, i32),
    /// The smallest width and height.
    pub min: (i32, i32),
}

/// The new place and size a grip drag asks for. The frame codes'
/// 0x80000011 bits drag the left edge (the x stays at 0 or more and the
/// width at least the minimum, moving the x back), 0x80000012 the right
/// edge, 0x80000014 the top and 0x80000018 the bottom, each tested as a
/// bit set so a corner moves two. Returns the place and the size the form
/// is resized to.
pub fn grip_drag(code: u32, d: GripDrag) -> (i32, i32, i32, i32) {
    let (mut x, mut y) = (d.x, d.y);
    let width = if code & 0x8000_0011 == 0x8000_0011 {
        let old = d.x;
        x = (d.moved.0 + d.grab.0).max(0);
        let mut w = (old + d.width) - x;
        if w < d.min.0 {
            x = (x - d.min.0) + w;
            w = d.min.0;
        }
        w
    } else if code & 0x8000_0012 == 0x8000_0012 {
        (d.moved.0 + d.grab.0).max(d.min.0)
    } else {
        d.width
    };
    let height = if code & 0x8000_0014 == 0x8000_0014 {
        let old = d.y;
        y = (d.moved.1 + d.grab.1).max(0);
        let mut h = (d.height + old) - y;
        if h < d.min.1 {
            y = (y - d.min.1) + h;
            h = d.min.1;
        }
        h
    } else if code & 0x8000_0018 == 0x8000_0018 {
        (d.moved.1 + d.grab.1).max(d.min.1)
    } else {
        d.height
    };
    (x, y, width, height)
}

/// What a press on a form's hovered code does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PressPlan {
    /// The record or frame code now held (0 none).
    pub held: i32,
    /// The grab the drag will read (see [`GripDrag`]).
    pub grab: Option<(i32, i32)>,
    /// The thumb jumps under the pointer, grabbed at its middle.
    pub thumb_jump: bool,
    /// A scroll arrow starts its repeat.
    pub start_repeat: bool,
    /// The press is sent to the hovered record (message 0x40).
    pub to_record: bool,
    pub result: u32,
}

/// The press's inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PressInput {
    /// The hovered code and the press word (buttons and flags).
    pub hover: i32,
    pub word: u16,
    pub last: i32,
    pub place: (i32, i32),
    pub size: (i32, i32),
    /// The form's flag word forbids moving it (0x20).
    pub fixed: bool,
    /// The pointer's offset into the horizontal and vertical thumbs, and
    /// their lengths.
    pub thumb_offset: (i32, i32),
    pub thumb: (i32, i32),
    pub style: u32,
}

/// The press rule on the form's own codes. A hovered record takes the
/// press unless the word has 0x8000 (the record's answer then decides the
/// held code). Otherwise only a left press acts: a grip saves its grab, a
/// title press (-3) the place unless the form is fixed, a thumb (-12, -8)
/// its offset (or jumps when the pointer is off it), an arrow or page
/// (-14…-13, -10…-9) starts the repeat; other codes hold nothing. A
/// form whose style has 8 adds 0x10 to a client press (-1).
pub fn press_plan(input: PressInput) -> PressPlan {
    let mut plan = PressPlan::default();
    if control_index(input.hover, input.last).is_some() {
        if input.word & 0x8000 != 0 {
            return plan;
        }
        plan.to_record = true;
        return plan;
    }
    if input.word & 0xefff != 1 {
        return client_press(plan, input);
    }
    plan.result = 5;
    plan.held = input.hover;
    let code = input.hover;
    let hover = code as u32;
    if (0x8000_0011..=0x8000_001a).contains(&hover) {
        let gx = if hover & 0x8000_0011 == 0x8000_0011 {
            input.place.0
        } else if hover & 0x8000_0012 == 0x8000_0012 {
            input.size.0
        } else {
            0
        };
        let gy = if hover & 0x8000_0014 == 0x8000_0014 {
            input.place.1
        } else if hover & 0x8000_0018 == 0x8000_0018 {
            input.size.1
        } else {
            0
        };
        plan.grab = Some((gx, gy));
    } else if code == -20 {
    } else if (-14..=-13).contains(&code) || (-10..=-9).contains(&code) {
        plan.start_repeat = true;
        plan.result = 0xd;
    } else if code == -12 || code == -8 {
        let (offset, thumb) = if code == -12 {
            (input.thumb_offset.0, input.thumb.0)
        } else {
            (input.thumb_offset.1, input.thumb.1)
        };
        if offset < 0 || thumb < offset {
            plan.grab = Some(((thumb as u32 >> 1) as i32, 0));
            plan.thumb_jump = true;
        } else {
            plan.grab = Some((offset, 0));
        }
    } else if code == -3 {
        if input.fixed {
            plan.held = 0;
            plan.result = 0;
            return plan;
        }
        plan.grab = Some(input.place);
    } else if !(-6..=-4).contains(&code) {
        plan.held = 0;
        plan.result = 0;
    }
    client_press(plan, input)
}

fn client_press(mut plan: PressPlan, input: PressInput) -> PressPlan {
    if plan.result & 0x10 == 0 && input.style & style::PRESS_THROUGH != 0 && input.hover == -1 {
        plan.result |= 0x10;
    }
    plan
}

/// What a release does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseAction {
    /// The held record is sent message 0x50 with the landing flag.
    Record(usize),
    /// The maximise mark (-6): a maximised form restores, another
    /// maximises.
    Restore,
    Maximise,
    /// The hide mark (-5).
    Hide,
    /// The close mark (-4).
    Close,
    /// A snapping form snaps its fraction when a thumb (-12, -8) lets go.
    SnapHorizontal,
    SnapVertical,
}

/// The release rule: a held record gets the release; a held frame mark
/// acts only while the pointer is still over it; a snapping form snaps the
/// released thumb's fraction. The held code then clears (result 0x80).
pub fn release_actions(
    held: i32,
    over: bool,
    last: i32,
    attributes: u32,
    maximised: bool,
) -> Vec<ReleaseAction> {
    if held >= 1 && held <= last + 1 {
        return vec![ReleaseAction::Record(held as usize - 1)];
    }
    let mut actions = Vec::new();
    if over {
        match held {
            -6 if maximised => actions.push(ReleaseAction::Restore),
            -6 => actions.push(ReleaseAction::Maximise),
            -5 => actions.push(ReleaseAction::Hide),
            -4 => actions.push(ReleaseAction::Close),
            _ => {}
        }
    }
    if attributes & attribute::SNAP_SCROLL != 0 {
        match held {
            -12 => actions.push(ReleaseAction::SnapHorizontal),
            -8 => actions.push(ReleaseAction::SnapVertical),
            _ => {}
        }
    }
    actions
}

/// A form's 256-tick show/hide fade. Mode 0 sets the GUI's alpha while the
/// form draws; mode 1 computes the eased value and discards it, so the
/// form draws at full alpha (the instructions keep the value in a
/// register no call reads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormFade {
    /// 0 none, 1 showing, 2 hiding.
    pub state: u8,
    /// The GUI clock at which the fade ends.
    pub end: u32,
    pub mode: u32,
}

/// The fade's length in GUI clock ticks.
pub const FADE_TICKS: u32 = 256;

/// One fade step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FadeStep {
    /// Draw, with this alpha set (`None`: the alpha is left alone).
    Draw(Option<f32>),
    /// The hide ended: the form is not drawn and closes.
    Close,
}

impl FormFade {
    /// Starts showing a hidden form at `now` unless it is already showing.
    /// Returns whether the form is to be made visible. The form is raised
    /// among the root's children either way.
    pub fn start_show(&mut self, now: u32, visible: bool) -> bool {
        if self.state != 1 && !visible {
            self.end = FADE_TICKS.wrapping_add(now);
            self.state = 1;
            return true;
        }
        false
    }

    /// Starts hiding at `now` (the clock the hide reads) unless already
    /// hiding. Returns whether it started.
    pub fn start_hide(&mut self, now: u32) -> bool {
        if self.state != 2 {
            self.state = 2;
            self.end = now.wrapping_add(FADE_TICKS);
            return true;
        }
        false
    }

    /// The step at `now`. An idle form draws as it is; a fade whose end is
    /// not after `now` stops (a hide closes the form).
    pub fn step(&mut self, now: u32) -> FadeStep {
        if self.state == 0 {
            return FadeStep::Draw(None);
        }
        if self.end <= now {
            let hiding = self.state == 2;
            self.state = 0;
            return if hiding {
                FadeStep::Close
            } else {
                FadeStep::Draw(None)
            };
        }
        let left = f64::from(self.end - now) / f64::from(FADE_TICKS);
        let alpha = match (self.state, self.mode) {
            (1, 0) => Some((1.0 - left) as f32),
            (2, 0) => Some(left as f32),
            _ => None,
        };
        FadeStep::Draw(alpha)
    }
}

/// The records the control draw visits: every record whose state does not
/// have both 0x100 and 0x10. The back pass draws them all, then the front
/// pass again in order.
pub fn drawn_records(states: &[u32]) -> Vec<usize> {
    (0..states.len())
        .filter(|&i| states[i] & record_state::NOT_DRAWN != record_state::NOT_DRAWN)
        .collect()
}

/// A record for the controls' rescale from an old unit to the current one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RescaleRecord {
    pub x: i16,
    pub y: i16,
    pub width: i16,
    pub height: i16,
    pub style: u32,
    /// The owner's width less its spacing reserve, and its client right
    /// and bottom.
    pub owner_span: i32,
    pub owner_right: i32,
    pub owner_bottom: i32,
}

/// Rescales a record from unit `old` to `unit`, as the form does when its
/// controls are set on a placed form: the width is `width / old · unit`
/// (16-bit, made even by adding one); a stretched record then takes the
/// owner's span, a right-anchored one keeps its distance from the owner's
/// right edge in units, any other scales its x. The y is moved to the
/// centre of the old row (`old` when the height is above eight old units,
/// else half of `height − 4·old`), scaled (bottom-anchored ones from the
/// owner's bottom), the height scaled and made even when above `old`, and
/// moved back out of the new row. A zero `old` leaves the record (the
/// original would divide by it).
pub fn rescale_record(r: &mut RescaleRecord, old: i32, unit: i32) {
    if old == 0 {
        return;
    }
    let u = unit as i16;
    let scale = |v: i16| ((i32::from(v) / old) as i16).wrapping_mul(u);
    let even = |v: i16| if v & 1 == 1 { v.wrapping_add(1) } else { v };
    r.width = even(scale(r.width));
    if r.style & style::X_MASK == style::X_MASK {
        r.width = r.width.wrapping_add(r.owner_span as i16);
    } else if r.style & style::X_MASK == style::RIGHT {
        let right = r.owner_right;
        r.x = (right as i16).wrapping_sub(
            (((i64::from(right) - i64::from(r.x)) / i64::from(old)) as i16).wrapping_mul(u),
        );
    } else {
        r.x = scale(r.x);
    }
    let half = |h: i16, n: i32| ((i64::from(h) - i64::from(n << 2)) as u64 >> 1) as i16;
    r.y = if old * 8 < i32::from(r.height) {
        r.y.wrapping_add(old as i16)
    } else {
        r.y.wrapping_add(half(r.height, old))
    };
    if r.style & style::Y_MASK == style::BOTTOM {
        let bottom = r.owner_bottom;
        r.y = (bottom as i16).wrapping_sub(
            (((i64::from(bottom) - i64::from(r.y)) / i64::from(old)) as i16).wrapping_mul(u),
        );
    } else {
        r.y = scale(r.y);
    }
    if old < i32::from(r.height) {
        r.height = even(scale(r.height));
    }
    r.y = if unit * 8 < i32::from(r.height) {
        r.y.wrapping_sub(u)
    } else {
        r.y.wrapping_sub(half(r.height, unit))
    };
}

/// A child for the snap slots: its place, client rectangle, frame spacing,
/// attribute word and visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SnapChild {
    pub x: i32,
    pub y: i32,
    pub client: ClientRect,
    pub spacing: i32,
    pub attributes: u32,
    pub visible: bool,
}

/// Inserts `value` into a sorted edge list unless present.
fn insert_edge(edges: &mut Vec<i32>, value: i32) {
    if let Err(at) = edges.binary_search(&value) {
        edges.insert(at, value);
    }
}

/// The magnetic snap slots of a container: `None` when the container is
/// not one (attribute 0x20 clear: its slots are left as they were). The
/// edges are the container client's 0 and spans and each visible snapping
/// child's client rectangle widened by its spacing; the cells between them
/// that no child covers are cut into slots, scanning rows top down: from a
/// free cell the run right and the run down are measured, and the longer
/// one (the row on a tie) becomes a slot one cell thick. Each slot is
/// `[left, top, right, bottom]` in the container's client coordinates.
pub fn snap_slots(
    attributes: u32,
    client: ClientRect,
    children: &[SnapChild],
) -> Option<Vec<[i32; 4]>> {
    if attributes & attribute::CLIENT_FILL == 0 {
        return None;
    }
    let mut xs = vec![0, client.right - client.left];
    let mut ys = vec![0, client.bottom - client.top];
    let boxes: Vec<[i32; 4]> = children
        .iter()
        .filter(|c| c.attributes & attribute::SNAPS != 0 && c.visible)
        .map(|c| {
            [
                (c.x + c.client.left) - c.spacing,
                c.client.right + c.x + c.spacing,
                (c.client.top + c.y) - c.spacing,
                c.y + c.client.bottom + c.spacing,
            ]
        })
        .collect();
    for b in &boxes {
        insert_edge(&mut xs, b[0]);
        insert_edge(&mut xs, b[1]);
        insert_edge(&mut ys, b[2]);
        insert_edge(&mut ys, b[3]);
    }
    let (nx, ny) = (xs.len() - 1, ys.len() - 1);
    if nx * ny < 2 {
        return Some(Vec::new());
    }
    let mut grid = vec![0u8; nx * ny];
    let at = |list: &[i32], v: i32| list.iter().position(|&e| e == v).unwrap_or(0);
    for b in &boxes {
        let (i1, i2) = (at(&xs, b[0]), at(&xs, b[1]));
        let (j1, j2) = (at(&ys, b[2]), at(&ys, b[3]));
        for j in j1..j2 {
            for i in i1..i2 {
                grid[i + j * nx] = 1;
            }
        }
    }
    let mut slots = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            if grid[i + j * nx] != 0 {
                continue;
            }
            let mut e = i;
            while e + 1 < nx && grid[e + 1 + j * nx] == 0 {
                e += 1;
            }
            let mut f = j;
            while f + 1 < ny && grid[i + (f + 1) * nx] == 0 {
                f += 1;
            }
            if e - i < f - j {
                for row in j..=f {
                    grid[i + row * nx] = 2;
                }
                slots.push([xs[i], ys[j], xs[i + 1], ys[f + 1]]);
            } else {
                for col in i..=e {
                    grid[col + j * nx] = 2;
                }
                slots.push([xs[i], ys[j], xs[e + 1], ys[j + 1]]);
            }
        }
    }
    Some(slots)
}

#[cfg(test)]
#[path = "form_test.rs"]
mod tests;

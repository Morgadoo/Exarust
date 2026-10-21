//! The GUI form's own layout metrics and base-class rules (`TGUIForm`):
//! skin measures, the 302×402 default form, anchoring, the client
//! rectangle, the scrollbars, the sibling order, showing and hiding with
//! the 256 ms fade, keyboard traversal, the wheel and the close edges.
//! The row spacing of the main menu is not modelled here; the binary's
//! `70·0.5·round(H/135)` does not fit its own list and needs a screenshot
//! to reconcile.
//!
//! The style word (`+0x5c`) says where an unplaced form goes and how it is
//! layered; the attribute word (`+0x64`) selects the frame parts; the state
//! word (`+0x60`) carries the layout bookkeeping. Positions and sizes are
//! physical pixels.

/// A GUI unit and the DPI border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormMetrics {
    pub unit: i32,
    pub border: i32,
}

impl FormMetrics {
    /// The minimum scrollbar thumb.
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
pub const DEFAULT_SCROLL_STEP: f32 = 0.025;
pub const PAGE_MIN: (i32, i32) = (76, 90);

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

/// A form's client rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// The form's client rectangle from its size, border and the style word
/// (`+0x64`). `parent_top` is the parent's `+0x10`, `parent_base` its
/// `+0x14`, `parent_extra` its `+0xc` (or 0), `rail` the skin rail and
/// `border_metric` the DPI border (2).
#[allow(clippy::too_many_arguments)]
pub fn client_rect(
    width: i32,
    height: i32,
    border: i32,
    style: u32,
    parent_top: i32,
    parent_base: i32,
    parent_extra: i32,
    rail: i32,
    border_metric: i32,
) -> ClientRect {
    let top = if style & 1 != 0 {
        if style & 0x80_0030 == 0x20 {
            parent_top + border_metric
        } else {
            parent_top + parent_extra
        }
    } else {
        border
    };
    let right = if style & 0x200 != 0 {
        width - border - parent_base
    } else {
        width - border
    };
    let bottom = if style & 0x100 != 0 {
        height - border - rail - 1 - 2 * border_metric
    } else {
        height - border
    };
    ClientRect {
        left: border,
        top,
        right,
        bottom,
    }
}

/// The scrollbar's metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollMetrics {
    pub step: f32,
    pub thumb: i32,
    pub thumb_x: i32,
}

/// The scrollbar for a form of `width` pixels, `content_w` content, the
/// given border and rail at scroll fraction `f`.
pub fn scroll_metrics(width: i32, content_w: i32, border: i32, rail: i32, f: f32) -> ScrollMetrics {
    let cw = width - 2 * border;
    let overflow = content_w - width;
    let step = if overflow > 0 {
        (10.0 / overflow as f64) as f32
    } else {
        0.0
    };
    let track = cw - 2 * rail;
    let mut thumb = ((cw as f64 / content_w as f64) * track as f64).round_ties_even() as i32;
    if thumb < FormMetrics::MIN_THUMB {
        thumb = FormMetrics::MIN_THUMB;
    } else if track < thumb {
        thumb = track;
    }
    let x = border + rail + ((track - thumb) as f32 * f) as i32;
    ScrollMetrics {
        step,
        thumb,
        thumb_x: x,
    }
}

/// Sets the scroll fraction: clamped to 0..1, optionally snapped to whole
/// steps, forced to 0 when there is no overflow. Returns the fraction and
/// thumb x.
#[allow(clippy::too_many_arguments)]
pub fn set_scroll_x(
    mut f: f32,
    step: f32,
    snap: bool,
    width: i32,
    content_w: i32,
    border: i32,
    rail: i32,
    thumb: i32,
) -> (f32, i32) {
    f = ordered_clamp(f);
    if snap && step > 0.0 {
        // The original's CVTSS2SI rounds to nearest, ties to even.
        f = (f / step).round_ties_even() * step;
    }
    let cw = width - 2 * border;
    if cw >= content_w {
        f = 0.0;
    }
    let track = cw - 2 * rail;
    let x = border + rail + ((track - thumb) as f32 * f) as i32;
    (f, x)
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

/// An unplaced coordinate (the constructor's sentinel).
pub const UNPLACED: i32 = -987_654_321;

/// A form's style word (`+0x5c`). The layering meaning of [`ON_TOP`] and
/// [`AT_BACK`] is read from the sibling ordering rules; the words
/// themselves are the original's.
pub mod style {
    /// Centre the unplaced x in the parent's client.
    pub const CENTRE_X: u32 = 0x800;
    /// Centre the unplaced y in the parent's client.
    pub const CENTRE_Y: u32 = 0x8000;
    /// Put an unplaced form at the right edge, four pixels in.
    pub const RIGHT: u32 = 0x20000;
    /// Put an unplaced form at the bottom edge, four pixels in.
    pub const BOTTOM: u32 = 0x80000;
    /// The x placement bits; `RIGHT` alone picks the right edge.
    pub const X_MASK: u32 = 0x30_000;
    /// The y placement bits; `BOTTOM` alone picks the bottom edge.
    pub const Y_MASK: u32 = 0xc_0000;
    /// A child kept at the bottom of the draw order when raised.
    pub const LOWER_LAYER: u32 = 0x2000;
    /// A child kept at the top of the draw order when raised.
    pub const UPPER_LAYER: u32 = 0x1000;
    /// A child whose own frame is not hit (`style & 2`).
    pub const NO_FRAME_HIT: u32 = 2;
}

/// A form's attribute word (`+0x64`).
pub mod attribute {
    /// Draw a title bar.
    pub const TITLE: u32 = 1;
    /// A close mark.
    pub const CLOSE: u32 = 2;
    /// A large mark.
    pub const LARGE: u32 = 4;
    /// A small mark.
    pub const SMALL: u32 = 8;
    /// A page hidden by the state word's bit 1.
    pub const PAGE: u32 = 0x10;
    /// A client fill.
    pub const CLIENT_FILL: u32 = 0x20;
    /// A rounded shell and outline frame.
    pub const ROUNDED: u32 = 0x40;
    /// A vertical scrollbar.
    pub const VSCROLL: u32 = 0x100;
    /// A horizontal scrollbar.
    pub const HSCROLL: u32 = 0x200;
    /// The scroll fraction snaps to whole steps.
    pub const SNAP_SCROLL: u32 = 0x400;
    /// The scroll offset is locked at zero.
    pub const LOCK_SCROLL: u32 = 0x800;
    /// A form skipped when the task looks for its active child.
    pub const NOT_ACTIVE: u32 = 0x1000;
    /// The tree is laid out in a single pass.
    pub const EAGER_LAYOUT: u32 = 0x2000;
}

/// A form's state word (`+0x60`).
pub mod state {
    /// The layout has run.
    pub const LAID_OUT: u32 = 0x1000;
    /// The form is placed in its parent.
    pub const PLACED: u32 = 0x4000;
    /// The form holds the focus (a control's caret lit).
    pub const FOCUSED: u32 = 0x10;
    /// The form draws a flat frame.
    pub const FLAT: u32 = 0x1;
    /// No gutters.
    pub const NO_GUTTERS: u32 = 0x4;
}

/// The rectangle a hit test needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl FormRect {
    pub fn new(left: i32, top: i32, width: i32, height: i32) -> Self {
        Self {
            left,
            top,
            right: left + width,
            bottom: top + height,
        }
    }

    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }

    /// Strictly inside: every edge is outside.
    pub fn strictly_inside(self, x: i32, y: i32) -> bool {
        self.left < x && x < self.right && self.top < y && y < self.bottom
    }

    /// Within `slop` pixels of the rectangle, measured from the nearest
    /// point (the original's control hit slop). A point exactly on the
    /// radius is not inside (the original's `!=` guard).
    pub fn near(self, x: i32, y: i32, slop: i32) -> bool {
        let dx = if x < self.left {
            self.left - x
        } else if self.right < x {
            x - self.right
        } else {
            0
        };
        let dy = if y < self.top {
            self.top - y
        } else if self.bottom < y {
            y - self.bottom
        } else {
            0
        };
        let r2 = (slop as i64) * (slop as i64);
        let d2 = (dx as i64) * (dx as i64) + (dy as i64) * (dy as i64);
        d2 < r2
    }
}

/// Resolves a form's unset place against its parent's client rectangle.
/// `parent` is `None` for a form attached to the root.
pub fn place_unset(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    style: u32,
    parent: Option<FormRect>,
) -> (i32, i32) {
    let Some(parent) = parent else {
        return (x, y);
    };
    let x = if x == UNPLACED {
        if style & style::CENTRE_X != 0 {
            // Signed division truncates toward zero, as the original's idiv.
            (parent.width() - width) / 2
        } else if style & style::X_MASK == style::RIGHT {
            parent.width() - width - 4
        } else {
            4
        }
    } else {
        x
    };
    let y = if y == UNPLACED {
        if style & style::CENTRE_Y != 0 {
            // The original shifts the span arithmetically (a floor).
            (parent.height() - height) >> 1
        } else if style & style::Y_MASK == style::BOTTOM {
            parent.height() - height - 4
        } else {
            36
        }
    } else {
        y
    };
    (x, y)
}

/// The place a form takes when it is shown: a centred style is re-centred
/// in the parent's client every time (the original's show rule).
pub fn show_place(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    style: u32,
    parent: FormRect,
) -> (i32, i32) {
    let x = if style & style::CENTRE_X != 0 {
        (parent.width() - width) / 2
    } else {
        x
    };
    let y = if style & style::CENTRE_Y != 0 {
        (parent.height() - height) / 2
    } else {
        y
    };
    (x, y)
}

/// The destination index for raising the child at `moving`, given the
/// children's style words in draw order (the last is the topmost). Pinned
/// children (`ON_TOP`) stay above the others; children at the back stay
/// below them.
pub fn raise_index(styles: &[u32], moving: usize) -> usize {
    let count = styles.len();
    if count == 0 || moving >= count {
        return moving;
    }
    let moving_style = styles[moving];
    let mut target;
    if moving_style & style::LOWER_LAYER != 0 {
        // The first child that is not pinned; 0 when all are pinned.
        target = 0;
        for (index, s) in styles.iter().enumerate().take(count.saturating_sub(1)) {
            target = index;
            if s & style::LOWER_LAYER == 0 {
                break;
            }
            target = 0;
        }
        if moving < target {
            target -= 1;
        }
    } else if moving_style & style::UPPER_LAYER != 0 {
        target = count - 1;
    } else {
        // The last child from the end that is not at the back; 0 if all are.
        target = 0;
        for index in (0..count).rev() {
            target = index;
            if styles[index] & style::UPPER_LAYER == 0 {
                break;
            }
            target = 0;
        }
    }
    target
}

/// Moves `ids[moving]` to the destination [`raise_index`] gives and returns
/// the destination. The ids are the window manager's form ids.
pub fn raise_child(ids: &mut Vec<u32>, styles: &[u32], moving: usize) -> usize {
    let target = raise_index(styles, moving);
    if moving < ids.len() {
        let id = ids.remove(moving);
        ids.insert(target.min(ids.len()), id);
    }
    target
}

/// The index of the topmost visible child in draw order: the last child
/// whose attribute word does not mark it inactive.
pub fn topmost_child(attributes: &[u32], visible: &[bool]) -> Option<usize> {
    (0..attributes.len()).rev().find(|&i| {
        attributes[i] & attribute::NOT_ACTIVE == 0 && visible.get(i).copied().unwrap_or(true)
    })
}

/// Whether a form is the active one: the topmost visible sibling.
pub fn is_active(form: usize, attributes: &[u32], visible: &[bool]) -> bool {
    topmost_child(attributes, visible) == Some(form)
}

/// A form's 256 ms show/hide fade. The mode is 0 for a linear ramp and 1
/// for the smooth `(3 − 2t)t²` ramp; a form may set either.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FormFade {
    /// 0 none, 1 showing, 2 hiding.
    pub state: u8,
    /// The clock tick at which the fade ends.
    pub end: i64,
    /// The ramp: 0 linear, 1 eased.
    pub mode: u8,
}

/// The fade's length in clock ticks (the original's 256 ms tick count).
pub const FADE_TICKS: i64 = 256;

impl FormFade {
    /// Starts showing at `now`; the form is fully visible at `now + 256`.
    pub fn start_show(&mut self, now: i64) {
        if self.state != 1 {
            self.state = 1;
            self.end = now + FADE_TICKS;
        }
    }

    /// Starts hiding at `now`; the form is gone at `now + 256`.
    pub fn start_hide(&mut self, now: i64) {
        if self.state != 2 {
            self.state = 2;
            self.end = now + FADE_TICKS;
        }
    }

    /// The global GUI alpha at `now`. `None` when a hiding fade has ended
    /// (the caller closes the form); `Some(1.0)` when idle or shown.
    pub fn step(&mut self, now: i64) -> Option<f32> {
        if self.state == 0 {
            return Some(1.0);
        }
        let hiding = self.state == 2;
        let remaining = (self.end - now).max(0) as f64;
        if self.end <= now {
            self.state = 0;
            return if hiding { None } else { Some(1.0) };
        }
        let t = (remaining / FADE_TICKS as f64) as f32;
        let eased = (3.0 - 2.0 * t) * t * t;
        Some(match (self.state, self.mode) {
            (1, 0) => 1.0 - t,
            (1, _) => 1.0 - eased,
            (2, 0) => t,
            _ => eased,
        })
    }
}

/// The vertical scrollbar's metrics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VerticalScroll {
    pub step: f32,
    pub thumb: i32,
    pub thumb_y: i32,
}

/// The vertical scrollbar for a form `height` pixels tall, `content_h`
/// content, the given border and rail at fraction `f`.
pub fn vertical_scroll_metrics(
    height: i32,
    content_h: i32,
    border: i32,
    rail: i32,
    f: f32,
) -> VerticalScroll {
    let ch = height - 2 * border;
    let overflow = content_h - height;
    let step = if overflow > 0 {
        (10.0 / overflow as f64) as f32
    } else {
        0.0
    };
    let track = ch - 2 * rail;
    let mut thumb = ((ch as f64 / content_h as f64) * track as f64).round_ties_even() as i32;
    if thumb < FormMetrics::MIN_THUMB {
        thumb = FormMetrics::MIN_THUMB;
    } else if track < thumb {
        thumb = track;
    }
    let y = border + rail + ((track - thumb) as f32 * f) as i32;
    VerticalScroll {
        step,
        thumb,
        thumb_y: y,
    }
}

/// Clamps a scroll fraction with the original's ordered comparisons: a
/// NaN passes through both, which `f32::clamp` would not do.
#[allow(clippy::manual_clamp)]
fn ordered_clamp(v: f32) -> f32 {
    if v < 0.0 {
        0.0
    } else if 1.0 < v {
        1.0
    } else {
        v
    }
}

/// The scroll fraction after `set_scroll_y`: clamped, optionally snapped to
/// whole steps (ties to even, as `CVTSS2SI` does), and forced to zero when
/// the content fits. Returns the fraction and the thumb's y.
#[allow(clippy::too_many_arguments)]
pub fn set_scroll_y(
    f: f32,
    step: f32,
    snap: bool,
    height: i32,
    content_h: i32,
    border: i32,
    rail: i32,
    thumb: i32,
) -> (f32, i32) {
    let mut value = ordered_clamp(f);
    if snap && step > 0.0 {
        value = (value / step).round_ties_even() * step;
    }
    let ch = height - 2 * border;
    if content_h <= ch {
        value = 0.0;
    }
    let track = ch - 2 * rail;
    let y = border + rail + ((track - thumb) as f32 * value) as i32;
    (value, y)
}

/// The wheel's effect on the vertical scroll: `delta` in wheel units, each
/// an eighth of a step less a hundredth of a step (the original's double
/// `0.0083333…` conversion, ties to even).
pub fn wheel_scroll(delta: i32, step: f32, current: f32, horizontal: bool) -> f32 {
    let v = (delta as f64
        * if horizontal {
            -0.008_333_333_333_333_333
        } else {
            0.008_333_333_333_333_333
        })
    .round_ties_even() as f32;
    current - v * step
}

/// The scroll a held scroll arrow or page repeats: `word` is the held
/// control's code, the base is the fraction when the drag began, the steps
/// the form's. The original names -10/-9 for the vertical arrows and
/// -13/-14 for the horizontal ones.
pub fn repeat_scroll(
    word: i32,
    base_x: f32,
    x: f32,
    y: f32,
    step_x: f32,
    step_y: f32,
) -> (f32, f32) {
    match word {
        -10 => (x, y + step_y),
        -9 => (x, y - step_y),
        -13 => (base_x - step_x, y),
        -14 => (base_x + step_x, y),
        _ => (x, y),
    }
}

/// A control's tab state bits: `focusable` is the record state's 8 and
/// `skip` its 0x10.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlTab {
    pub focusable: bool,
    pub skip: bool,
}

/// The next focusable control for Tab: forward from `current + 1`, then
/// round to the first focusable one; backward the other way. The wrap
/// ignores the skip bit, as the original's does.
pub fn tab_target(
    controls: &[ControlTab],
    current: Option<usize>,
    backward: bool,
) -> Option<usize> {
    let count = controls.len();
    if count == 0 {
        return None;
    }
    match (current, backward) {
        (Some(current), false) => {
            for (index, c) in controls.iter().enumerate().skip(current + 1) {
                if c.focusable && !c.skip {
                    return Some(index);
                }
            }
            (0..current).find(|&i| controls[i].focusable)
        }
        (Some(current), true) => {
            for (index, c) in controls.iter().enumerate().take(current).rev() {
                if c.focusable && !c.skip {
                    return Some(index);
                }
            }
            ((current + 1)..count)
                .rev()
                .find(|&i| controls[i].focusable)
        }
        (None, false) => (0..count).find(|&i| controls[i].focusable && !controls[i].skip),
        (None, true) => (0..count)
            .rev()
            .find(|&i| controls[i].focusable && !controls[i].skip),
    }
}

/// The wheel's control: a 1-based record code within the form's controls.
pub fn wheel_control(word: i32, last_control: i32) -> Option<usize> {
    let index = (word & 0xffff) - 1;
    if index < 0 || last_control < index {
        None
    } else {
        Some(index as usize)
    }
}

/// The caret blink: while the focused control (or form) has state bit 4,
/// the form's lit bit toggles every fifth tick.
pub fn caret_blink(caret: bool, counter: &mut u32, lit: &mut bool) -> bool {
    if !caret {
        return false;
    }
    *counter += 1;
    if *counter > 4 {
        *counter = 0;
        *lit = !*lit;
        return true;
    }
    false
}

/// Whether a close-band code is a close edge. [`attribute::SMALL`]'s skin
/// flag decides -8 (the original reads the skin's first field bit).
pub fn is_close_edge(code: i32, thin_frame: bool) -> bool {
    match code {
        -21..=-19 | -14..=-13 | -10..=-9 | -6..=-4 => true,
        -8 => thin_frame,
        _ => false,
    }
}

/// The cursor shape a resize code asks for (1 horizontal, 2 vertical,
/// 3 diagonal one way, 4 the other), or `None`.
pub fn resize_cursor(code: u32) -> Option<i32> {
    match code {
        0x8000_0011 | 0x8000_0012 => Some(2),
        0x8000_0014 | 0x8000_0018 => Some(1),
        0x8000_0015 | 0x8000_001a => Some(3),
        0x8000_0016 | 0x8000_0019 => Some(4),
        _ => None,
    }
}

/// One control rectangle for a hit test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HitControl {
    pub id: i32,
    pub rect: FormRect,
    pub state: u32,
    /// The control's own hit answer: its high byte is the cursor, its
    /// 0x7f0000 bits pass back to the form; a 0x800000 bit lets the point
    /// through to the form.
    pub answer: u32,
}

/// A form for the base hit test: its rectangle, children in draw order and
/// its controls in content coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct HitNode {
    pub id: u32,
    pub rect: FormRect,
    pub visible: bool,
    pub style: u32,
    pub unit: i32,
    /// The content's width and height (the client plus the scroll overflow).
    pub content: (i32, i32),
    pub client: ClientRect,
    pub scroll_x: f32,
    pub scroll_y: f32,
    pub children: Vec<HitNode>,
    pub controls: Vec<HitControl>,
    /// The skin's default cursor shape.
    pub cursor: i32,
}

impl HitNode {
    /// The point in the form's content coordinates.
    pub fn content_point(&self, x: i32, y: i32) -> (i32, i32) {
        let cw = self.client.width();
        let ch = self.client.height();
        let sx = ((self.content.0 - cw) as f32 * self.scroll_x) as i32;
        let sy = ((self.content.1 - ch) as f32 * self.scroll_y) as i32;
        (sx + (x - self.client.left), sy + (y - self.client.top))
    }
}

/// What a hit test found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TreeHit {
    pub form: Option<u32>,
    pub control: i32,
    pub cursor: i32,
    /// The control's answer bits, or 0 for the form itself.
    pub answer: u32,
}

/// The base form's hit test: the deepest visible child first (the last
/// child at the top), then the controls in content coordinates. A point
/// outside every form is `None`.
pub fn hit_test_tree(node: &HitNode, x: i32, y: i32) -> Option<TreeHit> {
    hit_node(node, x - node.rect.left, y - node.rect.top)
}

/// `lx` and `ly` are relative to the node's own top-left corner.
fn hit_node(node: &HitNode, lx: i32, ly: i32) -> Option<TreeHit> {
    if !node.visible || lx < 0 || ly < 0 || lx >= node.rect.width() || ly >= node.rect.height() {
        return None;
    }
    let (cx, cy) = node.content_point(lx, ly);
    for child in node.children.iter().rev() {
        if !child.visible || child.style & style::NO_FRAME_HIT != 0 {
            continue;
        }
        let rx = cx - child.rect.left;
        let ry = cy - child.rect.top;
        if child.rect.strictly_inside(cx, cy)
            && let Some(hit) = hit_node(child, rx, ry)
        {
            return Some(hit);
        }
    }
    for control in &node.controls {
        if control.state & 0x10 != 0 || control.state & 0x2 != 0 {
            continue;
        }
        if control.answer & 0x80_0000 != 0 {
            continue;
        }
        if control.rect.near(cx, cy, 2 * node.unit) {
            let mut answer = control.answer;
            if answer & 0x7f_0000 == 0 {
                answer |= 0x1_0000;
            }
            let cursor = if answer >> 24 == 0 {
                node.cursor
            } else {
                (answer >> 24) as i32
            };
            return Some(TreeHit {
                form: Some(node.id),
                control: control.id,
                cursor,
                answer,
            });
        }
    }
    Some(TreeHit {
        form: Some(node.id),
        control: 0,
        cursor: node.cursor,
        answer: 0,
    })
}

/// The magnetic snap slots around the visible sibling forms: sorted unique
/// edge coordinates (the forms' frames widened by `grip`) and the free
/// rectangles between them, up to 2000. Each slot is `(x1, y1, x2, y2)`.
pub struct SnapForm {
    pub rect: FormRect,
    pub visible: bool,
    pub style: u32,
    /// The form's title-bar length (the original's `+0x38`).
    pub grip: i32,
}

pub fn snap_slots(forms: &[SnapForm]) -> Vec<(i32, i32, i32, i32)> {
    let mut xs: Vec<i32> = Vec::new();
    let mut ys: Vec<i32> = Vec::new();
    let push = |list: &mut Vec<i32>, value: i32| {
        if let Err(at) = list.binary_search(&value) {
            list.insert(at, value);
        }
    };
    let mut boxes = Vec::new();
    for form in forms {
        if !form.visible || form.style & 0x30 == 0 {
            continue;
        }
        let x1 = form.rect.left - form.grip;
        let x2 = form.rect.right + form.grip;
        let y1 = form.rect.top - form.grip;
        let y2 = form.rect.bottom + form.grip;
        push(&mut xs, x1);
        push(&mut xs, x2);
        push(&mut ys, y1);
        push(&mut ys, y2);
        boxes.push((x1, x2, y1, y2));
    }
    let (nx, ny) = (xs.len(), ys.len());
    if nx * ny < 2 {
        return Vec::new();
    }
    let mut grid = vec![false; nx * ny];
    for (x1, x2, y1, y2) in boxes {
        let Some(i1) = xs.iter().position(|&x| x == x1) else {
            continue;
        };
        let Some(i2) = xs.iter().position(|&x| x == x2) else {
            continue;
        };
        let Some(j1) = ys.iter().position(|&y| y == y1) else {
            continue;
        };
        let Some(j2) = ys.iter().position(|&y| y == y2) else {
            continue;
        };
        for j in j1..j2 {
            for i in i1..i2 {
                grid[j * nx + i] = true;
            }
        }
    }
    let mut slots = Vec::new();
    for j in 0..ny.saturating_sub(1) {
        for i in 0..nx.saturating_sub(1) {
            if grid[j * nx + i] {
                continue;
            }
            let mut xe = i;
            while xe + 1 < nx - 1 && !grid[j * nx + xe + 1] {
                xe += 1;
            }
            let mut ye = j;
            while ye + 1 < ny - 1 && !grid[(ye + 1) * nx + i] {
                ye += 1;
            }
            if xe - i < ye - j {
                for y in j..ye {
                    grid[y * nx + i] = true;
                }
            } else {
                for x in i..xe {
                    grid[j * nx + x] = true;
                }
            }
            slots.push((xs[i], ys[j], xs[xe + 1], ys[ye + 1]));
            if slots.len() >= 2000 {
                return slots;
            }
        }
    }
    slots
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_skin_measures() {
        let m = FormMetrics { unit: 5, border: 2 };
        assert_eq!((m.base(), m.close(), m.rail(), m.tab()), (28, 21, 19, 23));
        assert_eq!(
            FormMetrics {
                unit: 12,
                border: 2
            }
            .base(),
            70
        );
        assert_eq!(border_for_dpi(96), 2);
        assert_eq!(border_for_dpi(405), 3);
    }

    #[test]
    fn the_form_defaults() {
        assert_eq!(
            (
                DEFAULT_WIDTH,
                DEFAULT_HEIGHT,
                DEFAULT_BORDER,
                DEFAULT_CONTENT,
                DEFAULT_SCROLL_STEP,
                PAGE_MIN
            ),
            (302, 402, 2, (300, 800), 0.025, (76, 90))
        );
    }

    #[test]
    fn the_client_rectangle() {
        let r = client_rect(302, 402, 2, 0, 0, 0, 0, 15, 2);
        assert_eq!(
            r,
            ClientRect {
                left: 2,
                top: 2,
                right: 300,
                bottom: 400
            }
        );
        assert_eq!((r.width(), r.height()), (298, 398));
        let r = client_rect(302, 402, 2, 0x200, 0, 7, 0, 15, 2);
        assert_eq!(r.right, 293);
        let r = client_rect(302, 402, 2, 0x100, 0, 0, 0, 15, 2);
        assert_eq!(r.bottom, 400 - 15 - 1 - 4);
    }

    #[test]
    fn the_anchors() {
        assert_eq!(anchor(None, 0, 100, false, Anchor::Default), 4);
        assert_eq!(anchor(None, 0, 100, true, Anchor::Default), 36);
        assert_eq!(anchor(None, 800, 100, false, Anchor::Center), 350);
        assert_eq!(anchor(None, 800, 100, false, Anchor::Far), 696);
        assert_eq!(anchor(Some(12), 800, 100, false, Anchor::Far), 12);
    }

    #[test]
    fn the_scrollbar_metrics() {
        let s = scroll_metrics(404, 1200, 2, 19, 0.0);
        assert!((s.step - 10.0 / 796.0).abs() < 1e-7);
        assert_eq!(s.thumb, 121);
        assert_eq!(s.thumb_x, 21);
        let s = scroll_metrics(404, 1200, 2, 19, 1.0);
        assert_eq!(s.thumb_x, 262);
    }

    #[test]
    fn the_scroll_fraction_and_snap() {
        let (f, x) = set_scroll_x(0.5, 0.1, false, 404, 1200, 2, 19, 121);
        assert_eq!(f, 0.5);
        assert_eq!(x, 21 + 120);
        // 0.55 / 0.1 is exactly 5.5 in single precision; ties go to even.
        let (f, _) = set_scroll_x(0.55, 0.1, true, 404, 1200, 2, 19, 121);
        assert_eq!(f, 0.6);
        let (f, _) = set_scroll_x(0.45, 0.1, true, 404, 1200, 2, 19, 121);
        assert_eq!(f, 0.4);
        // Ties go to even: 1.5 steps round to 2, 0.5 to 0.
        let (f, _) = set_scroll_x(0.75, 0.5, true, 404, 1200, 2, 19, 121);
        assert_eq!(f, 1.0);
        let (f, _) = set_scroll_x(0.25, 0.5, true, 404, 1200, 2, 19, 121);
        assert_eq!(f, 0.0);
        let (f, x) = set_scroll_x(0.5, 0.1, false, 404, 400, 2, 19, 121);
        assert_eq!(f, 0.0);
        assert_eq!(x, 21);
    }

    #[test]
    fn the_vertical_scrollbar() {
        let s = vertical_scroll_metrics(404, 1200, 2, 19, 0.0);
        assert_eq!((s.thumb, s.thumb_y), (121, 21));
        assert!((s.step - 10.0 / 796.0).abs() < 1e-7);
        let (f, y) = set_scroll_y(1.0, s.step, false, 404, 1200, 2, 19, s.thumb);
        assert_eq!((f, y), (1.0, 262));
        let (f, _) = set_scroll_y(0.25, 0.5, true, 404, 1200, 2, 19, 121);
        assert_eq!(f, 0.0);
        let (f, _) = set_scroll_y(f32::NAN, 0.0, false, 404, 1200, 2, 19, 121);
        assert!(f.is_nan());
    }

    #[test]
    fn the_wheel_and_repeat() {
        // 120 wheel units is one step.
        assert!((wheel_scroll(120, 0.1, 0.5, false) - 0.4).abs() < 1e-6);
        assert!((wheel_scroll(-120, 0.1, 0.5, false) - 0.6).abs() < 1e-6);
        assert!((wheel_scroll(120, 0.1, 0.5, true) - 0.6).abs() < 1e-6);
        // Half a step rounds to even, so 60 units move nothing.
        assert!((wheel_scroll(60, 0.1, 0.5, false) - 0.5).abs() < 1e-6);
        assert!((wheel_scroll(180, 0.1, 0.5, false) - 0.3).abs() < 1e-6);
        let (x, y) = repeat_scroll(-10, 0.0, 0.2, 0.3, 0.05, 0.1);
        assert_eq!((x, y), (0.2, 0.4));
        let (x, y) = repeat_scroll(-13, 0.7, 0.2, 0.3, 0.05, 0.1);
        assert_eq!((x, y), (0.65, 0.3));
        assert_eq!(repeat_scroll(-3, 0.7, 0.2, 0.3, 0.05, 0.1), (0.2, 0.3));
    }

    #[test]
    fn the_tab_order() {
        let none = ControlTab::default();
        let can = ControlTab {
            focusable: true,
            skip: false,
        };
        let skipped = ControlTab {
            focusable: true,
            skip: true,
        };
        let controls = [can, skipped, can, can];
        assert_eq!(tab_target(&controls, Some(0), false), Some(2));
        assert_eq!(tab_target(&controls, Some(3), false), Some(0));
        assert_eq!(tab_target(&controls, Some(0), true), Some(3));
        assert_eq!(tab_target(&controls, Some(2), true), Some(0));
        assert_eq!(tab_target(&[none, none], None, false), None);
        assert_eq!(tab_target(&[skipped], None, false), None);
        assert_eq!(tab_target(&[], Some(0), false), None);
    }

    #[test]
    fn the_wheel_control_and_caret() {
        assert_eq!(wheel_control(3, 5), Some(2));
        assert_eq!(wheel_control(0x7f0003, 5), Some(2));
        assert_eq!(wheel_control(0, 5), None);
        assert_eq!(wheel_control(6, 5), Some(5));
        assert_eq!(wheel_control(7, 5), None);
        let mut counter = 0;
        let mut lit = true;
        for _ in 0..4 {
            assert!(!caret_blink(true, &mut counter, &mut lit));
        }
        assert!(caret_blink(true, &mut counter, &mut lit));
        assert!(!lit);
        assert!(!caret_blink(false, &mut counter, &mut lit));
        assert_eq!(counter, 0);
    }

    #[test]
    fn the_close_edges_and_cursors() {
        assert!(is_close_edge(-20, false));
        assert!(is_close_edge(-14, false));
        assert!(is_close_edge(-9, false));
        assert!(is_close_edge(-5, false));
        assert!(!is_close_edge(-8, false));
        assert!(is_close_edge(-8, true));
        assert!(!is_close_edge(-7, true));
        assert!(!is_close_edge(-3, true));
        assert_eq!(resize_cursor(0x8000_0011), Some(2));
        assert_eq!(resize_cursor(0x8000_0012), Some(2));
        assert_eq!(resize_cursor(0x8000_0014), Some(1));
        assert_eq!(resize_cursor(0x8000_0018), Some(1));
        assert_eq!(resize_cursor(0x8000_0015), Some(3));
        assert_eq!(resize_cursor(0x8000_0016), Some(4));
        assert_eq!(resize_cursor(0x8000_0019), Some(4));
        assert_eq!(resize_cursor(0x8000_001a), Some(3));
        assert_eq!(resize_cursor(0x8000_0017), None);
    }

    #[test]
    fn the_labels_and_nudge() {
        assert_eq!(styled_label(0, "EXIT"), "EXIT");
        assert_eq!(styled_label(1, "EXIT"), "[EXIT]");
        assert_eq!(styled_label(2, "EXIT"), "[[EXIT]]");
        assert_eq!(selected_nudge(5), 1);
        assert_eq!(selected_nudge(8), 2);
        assert_eq!(selected_nudge(12), 4);
    }

    #[test]
    fn the_unset_place() {
        let parent = FormRect::new(10, 20, 800, 600);
        assert_eq!(
            place_unset(UNPLACED, UNPLACED, 200, 100, 0, Some(parent)),
            (4, 36)
        );
        assert_eq!(
            place_unset(
                UNPLACED,
                UNPLACED,
                200,
                100,
                style::CENTRE_X | style::CENTRE_Y,
                Some(parent)
            ),
            (300, 250)
        );
        assert_eq!(
            place_unset(UNPLACED, UNPLACED, 200, 100, style::RIGHT, Some(parent)),
            (596, 36)
        );
        assert_eq!(
            place_unset(UNPLACED, UNPLACED, 200, 100, style::BOTTOM, Some(parent)),
            (4, 496)
        );
        // The x centring truncates and the y centring floors.
        let odd = FormRect::new(0, 0, 801, 501);
        assert_eq!(
            place_unset(
                UNPLACED,
                UNPLACED,
                200,
                100,
                style::CENTRE_X | style::CENTRE_Y,
                Some(odd)
            ),
            (300, 200)
        );
        assert_eq!(place_unset(7, 8, 200, 100, 0, Some(parent)), (7, 8));
        assert_eq!(
            place_unset(UNPLACED, UNPLACED, 200, 100, 0, None),
            (UNPLACED, UNPLACED)
        );
    }

    #[test]
    fn showing_recentres() {
        let parent = FormRect::new(0, 0, 800, 600);
        assert_eq!(
            show_place(13, 17, 200, 100, style::CENTRE_X | style::CENTRE_Y, parent),
            (300, 250)
        );
        assert_eq!(show_place(13, 17, 200, 100, 0, parent), (13, 17));
    }

    #[test]
    fn the_sibling_order() {
        let s = |a: &[u32], i: usize| raise_index(a, i);
        assert_eq!(s(&[0, 0, 0], 0), 2);
        assert_eq!(s(&[0, 0, 0], 2), 2);
        assert_eq!(s(&[0, style::UPPER_LAYER, 0], 0), 2);
        assert_eq!(s(&[0, style::UPPER_LAYER, 0], 1), 2);
        assert_eq!(s(&[style::LOWER_LAYER, 0, 0], 1), 2);
        assert_eq!(s(&[0, style::LOWER_LAYER, 0], 1), 0);
        assert_eq!(s(&[0, style::LOWER_LAYER, style::LOWER_LAYER], 1), 0);
        assert_eq!(s(&[0, style::LOWER_LAYER, style::LOWER_LAYER], 2), 0);
        let mut ids = vec![10, 11, 12];
        assert_eq!(raise_child(&mut ids, &[0, 0, 0], 0), 2);
        assert_eq!(ids, vec![11, 12, 10]);
        let mut ids = vec![10, 11, 12];
        assert_eq!(raise_child(&mut ids, &[0, 0, 0], 1), 2);
        assert_eq!(ids, vec![10, 12, 11]);
        assert_eq!(topmost_child(&[0, 0, 0], &[true, true, true]), Some(2));
        assert_eq!(
            topmost_child(&[0, attribute::NOT_ACTIVE, 0], &[true, true, true]),
            Some(2)
        );
        assert_eq!(topmost_child(&[attribute::NOT_ACTIVE], &[true]), None);
        assert!(is_active(2, &[0, 0, 0], &[true, true, true]));
    }

    #[test]
    fn the_form_fade() {
        let mut fade = FormFade::default();
        assert_eq!(fade.step(10), Some(1.0));
        fade.start_show(1000);
        assert_eq!(fade.step(1000), Some(0.0));
        assert!((fade.step(1128).unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(fade.step(1256), Some(1.0));
        assert_eq!(fade.state, 0);
        let mut fade = FormFade {
            mode: 1,
            ..FormFade::default()
        };
        fade.start_show(0);
        // 1 - (3 - 2·0.25)·0.25² = 1 - 0.15625.
        assert!((fade.step(192).unwrap() - 0.84375).abs() < 1e-6);
        fade.start_hide(1000);
        assert!((fade.step(1128).unwrap() - 0.5).abs() < 1e-6);
        assert_eq!(fade.step(1256), None);
        assert_eq!(fade.state, 0);
    }

    fn leaf(id: u32, rect: FormRect, controls: Vec<HitControl>) -> HitNode {
        HitNode {
            id,
            rect,
            visible: true,
            style: 0,
            unit: 5,
            content: (rect.width(), rect.height()),
            client: ClientRect {
                left: 0,
                top: 0,
                right: rect.width(),
                bottom: rect.height(),
            },
            scroll_x: 0.0,
            scroll_y: 0.0,
            children: Vec::new(),
            controls,
            cursor: 3,
        }
    }

    #[test]
    fn the_hit_test() {
        let control = HitControl {
            id: 1,
            rect: FormRect::new(10, 10, 100, 20),
            state: 0,
            answer: 0,
        };
        let root = leaf(7, FormRect::new(0, 0, 800, 600), vec![control]);
        let hit = hit_test_tree(&root, 50, 20).unwrap();
        assert_eq!((hit.form, hit.control, hit.cursor), (Some(7), 1, 3));
        assert_eq!(hit.answer, 0x1_0000);
        // Two units of slop around the control.
        assert_eq!(hit_test_tree(&root, 111, 20).unwrap().control, 1);
        assert_eq!(hit_test_tree(&root, 121, 20).unwrap().control, 0);
        assert_eq!(hit_test_tree(&root, -1, 20), None);
        let passing = HitNode {
            controls: vec![HitControl {
                answer: 0x80_0000,
                ..control
            }],
            ..root.clone()
        };
        assert_eq!(hit_test_tree(&passing, 50, 20).unwrap().control, 0);
        let shaped = HitNode {
            controls: vec![HitControl {
                answer: 7 << 24,
                ..control
            }],
            ..root
        };
        assert_eq!(hit_test_tree(&shaped, 50, 20).unwrap().cursor, 7);
    }

    #[test]
    fn the_hit_test_descends_into_children() {
        let child = leaf(
            2,
            FormRect::new(100, 100, 200, 100),
            vec![HitControl {
                id: 4,
                rect: FormRect::new(5, 5, 50, 10),
                state: 0,
                answer: 0,
            }],
        );
        let root = HitNode {
            children: vec![child],
            ..leaf(1, FormRect::new(0, 0, 800, 600), Vec::new())
        };
        let hit = hit_test_tree(&root, 110, 110).unwrap();
        assert_eq!((hit.form, hit.control), (Some(2), 4));
        // A child's edge is outside it, so the root answers there.
        let hit = hit_test_tree(&root, 100, 150).unwrap();
        assert_eq!((hit.form, hit.control), (Some(1), 0));
    }

    #[test]
    fn the_snap_slots() {
        let form = |left, top| SnapForm {
            rect: FormRect::new(left, top, 100, 100),
            visible: true,
            style: 0x30,
            grip: 4,
        };
        assert_eq!(snap_slots(&[]), Vec::new());
        assert_eq!(
            snap_slots(&[form(0, 0), form(200, 0)]),
            vec![(104, -4, 196, 104)]
        );
        // A hidden or non-moving form contributes no edges.
        let hidden = SnapForm {
            visible: false,
            ..form(0, 0)
        };
        assert_eq!(snap_slots(&[hidden]), Vec::new());
        let fixed = SnapForm {
            style: 0,
            ..form(0, 0)
        };
        assert_eq!(snap_slots(&[fixed]), Vec::new());
    }
}

//! The GUI form's own layout metrics (`TGUIForm`): skin measures, the
//! 302×402 default form, anchoring, the client rectangle and the scrollbar.
//! The row spacing of the main menu is not modelled here; the binary's
//! `70·0.5·round(H/135)` does not fit its own list and needs a screenshot
//! to reconcile.

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
    let mut thumb = ((cw as f64 / content_w as f64) * track as f64) as i32;
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
    f = f.clamp(0.0, 1.0);
    if snap && step > 0.0 {
        f = (f / step).trunc() * step;
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
        assert_eq!(s.thumb, 120);
        assert_eq!(s.thumb_x, 21);
        let s = scroll_metrics(404, 1200, 2, 19, 1.0);
        assert_eq!(s.thumb_x, 263);
    }

    #[test]
    fn the_scroll_fraction_and_snap() {
        let (f, x) = set_scroll_x(0.5, 0.1, false, 404, 1200, 2, 19, 120);
        assert_eq!(f, 0.5);
        assert_eq!(x, 21 + 121);
        let (f, _) = set_scroll_x(0.55, 0.1, true, 404, 1200, 2, 19, 120);
        assert!((f - 0.5).abs() < 1e-6);
        let (f, x) = set_scroll_x(0.5, 0.1, false, 404, 400, 2, 19, 120);
        assert_eq!(f, 0.0);
        assert_eq!(x, 21);
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
}

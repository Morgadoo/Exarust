//! The extended GUI controls' geometry (`RF_GUI_Controls_Ex`): the trackbar
//! value/thumb map, the menu/drop-down row metrics, list extents and
//! radio/check activation. The unit-derived `3U|1` quirk is kept.

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
    /// clamped to 0..1 and quantised to `steps` when positive. NaN passes
    /// both clamps, as the original's `jp`.
    pub fn value_at(self, x: i32, w: i32, cursor: i32, steps: i32) -> f32 {
        let num = cursor - x - self.pad - self.span / 2;
        let den = self.travel(w);
        let mut f = (num as f64 / den as f64) as f32;
        if !f.is_nan() {
            f = f.clamp(0.0, 1.0);
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
    let lx = match style & 0x7000 {
        0x1000 => x - text_w - unit,
        0x2000 => x + pad + unit + (avail - 1 - text_w) / 2,
        0x3000 => x + avail - text_w + unit,
        0x4000 => x + avail + unit,
        _ => x + pad + (unit + 1) / 2,
    };
    let ly = match style & 0x700 {
        0x100 => y - unit / 2,
        0x200 => y + (h + line_h + 2) / 2,
        0x300 => y + h + base - 1,
        0x400 => y + h,
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
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x2000).x,
            145
        );
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x3000).x,
            184
        );
        assert_eq!(
            align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0x4000).x,
            204
        );
        assert_eq!(align_label(100, 100, 20, 2, 4, 20, 100, 16, 14, 0).x, 104);
    }

    #[test]
    fn the_vertical_styles() {
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x100).y, 98);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x200).y, 119);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x300).y, 133);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0x400).y, 120);
        assert_eq!(align_label(0, 100, 20, 2, 4, 0, 0, 16, 14, 0).y, 102);
    }
}

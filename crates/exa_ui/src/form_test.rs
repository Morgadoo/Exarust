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
fn the_constructor_defaults() {
    let d = form_defaults();
    assert_eq!(
        (d.width, d.height, d.border, d.content, d.min_page),
        (302, 402, 2, (300, 800), (76, 90))
    );
    assert_eq!((d.attributes, d.state), (0x30022, 0x3000));
    assert_eq!((d.x, d.y), (UNPLACED, UNPLACED));
    assert_eq!((d.step, d.vertical_step), (0.025, 0.25));
    assert!(d.visible && d.closes_once);
    assert_eq!(d.last_control, -1);
}

#[test]
fn the_client_rectangle() {
    let r = client_rect(302, 402, 2, 0, 0, 0, 0, 0, 15, 2);
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
    // The vertical scrollbar narrows the right, the horizontal one lifts
    // the bottom by the rail, one and two borders.
    assert_eq!(
        client_rect(302, 402, 2, 0, attribute::VSCROLL, 0, 7, 0, 15, 2).right,
        293
    );
    assert_eq!(
        client_rect(302, 402, 2, 0, attribute::HSCROLL, 0, 0, 0, 15, 2).bottom,
        400 - 15 - 1 - 4
    );
    assert_eq!(client_rect(302, 402, 2, 0, 1, 6, 0, 18, 15, 2).top, 24);
    assert_eq!(client_rect(302, 402, 2, 0, 0x21, 6, 0, 18, 15, 2).top, 8);
}

#[test]
fn the_control_spans_keep_the_border_over_the_stretches() {
    let skin = ControlSkin {
        span_x: 8,
        span_y: 9,
        title: 20,
        scroll: 7,
    };
    // No stretches: the border 2 is kept and remembered; the client is the
    // form less the border, and without a title the top is the border.
    let m = control_spans(302, 402, 2, 0, 0, 0, skin);
    assert_eq!(
        (m.border, m.saved_border, m.right, m.bottom, m.title),
        (2, 2, 300, 400, 2)
    );
    assert!(!m.horizontal_scroll && !m.vertical_scroll);
    // The frame's stretch (style 0x30 across, 0xc0 down) above the border
    // becomes the border, and nothing is remembered.
    let wide = control_spans(302, 402, 2, 0x30, 0, 0, skin);
    assert_eq!(
        (
            wide.border,
            wide.saved_border,
            wide.span_x,
            wide.right,
            wide.title
        ),
        (8, 0, 8, 294, 8)
    );
    // A remembered border is restored over the one given.
    let restored = control_spans(302, 402, 5, 0, 0, 2, skin);
    assert_eq!(
        (restored.border, restored.saved_border, restored.right),
        (2, 2, 300)
    );
    // The scrollbars inset the client and the title takes the skin's
    // height plus the top stretch (or the border metric 2 for 0x20).
    let bars = control_spans(
        302,
        402,
        2,
        0xc0,
        attribute::VSCROLL | attribute::HSCROLL | 1,
        0,
        skin,
    );
    assert_eq!(
        (bars.border, bars.right, bars.bottom, bars.title),
        (9, 286, 386, 29)
    );
    assert!(bars.horizontal_scroll && bars.vertical_scroll);
    let thin = control_spans(302, 402, 2, 0xc0, 0x21, 0, skin);
    assert_eq!(thin.title, 22);
}

#[test]
fn setting_controls_copies_or_adopts_the_table() {
    let input = SetControlsInput {
        owns_old: false,
        old_count: -1,
        hover: 2,
        adopt: false,
        new_count: 3,
        state: state::RESTYLE | state::LAYOUT,
    };
    let plan = set_controls(input);
    assert_eq!(
        plan,
        SetControlsPlan {
            free_old: false,
            clear_hover: true,
            table: ControlTable::Copy,
            count: 3,
            restyle: true,
            spans: true,
            metrics: false,
            assign_owner: true,
            setup: true,
            rescale: false,
        }
    );
    // A placed form recomputes the client area and rescales the records.
    let placed = set_controls(SetControlsInput {
        state: state::LAYOUT | state::PLACED,
        ..input
    });
    assert!(placed.metrics && placed.rescale && !placed.spans);
    // A negative count leaves no table; an adopted table is not copied.
    let none = set_controls(SetControlsInput {
        new_count: -1,
        state: 0,
        ..input
    });
    assert_eq!(
        (none.table, none.count, none.assign_owner),
        (ControlTable::Empty, -1, false)
    );
    assert_eq!(
        set_controls(SetControlsInput {
            adopt: true,
            new_count: 2,
            state: 0,
            ..input
        })
        .table,
        ControlTable::Adopt
    );
    // The adopt flag wins over a negative count, which is kept as given.
    let adopted = set_controls(SetControlsInput {
        adopt: true,
        new_count: -1,
        state: 0,
        ..input
    });
    assert_eq!(
        (adopted.table, adopted.count, adopted.assign_owner),
        (ControlTable::Adopt, -1, false)
    );
    // A stored count of 0 still has its record's owner set.
    assert!(
        set_controls(SetControlsInput {
            new_count: 0,
            ..input
        })
        .assign_owner
    );
    // The old owned table is freed, an empty one included.
    assert!(
        set_controls(SetControlsInput {
            owns_old: true,
            old_count: 0,
            ..input
        })
        .free_old
    );
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
fn the_conversions_round_as_the_instructions() {
    assert_eq!(cvtss2si(2.5), 2);
    assert_eq!(cvtss2si(3.5), 4);
    assert_eq!(cvtss2si(-2.5), -2);
    assert_eq!(cvtss2si(f32::NAN), 0);
    assert_eq!(cvtsd2si(120.5), 120);
    assert!((ease(0.25) - 0.15625).abs() < 1e-7);
    assert_eq!((ease(0.0), ease(1.0)), (0.0, 1.0));
}

#[test]
fn the_horizontal_scrollbar() {
    // Client 400, track 400 − 38 = 362, thumb round(362 · 400/1200 =
    // 120.67) = 121.
    let s = scroll_metrics(404, 1200, 2, 0, 19, 0.0);
    assert!((s.step - 10.0 / 796.0).abs() < 1e-7);
    assert_eq!((s.thumb, s.thumb_x), (121, 21));
    assert_eq!(scroll_metrics(404, 1200, 2, 0, 19, 1.0).thumb_x, 262);
    // A vertical scrollbar insets the client's right edge: the step's
    // denominator and the thumb follow it.
    let s = scroll_metrics(404, 1200, 2, 7, 19, 0.5);
    assert_eq!(s.thumb, 116);
    assert!((s.step - 10.0 / 803.0).abs() < 1e-7);
    assert_eq!(s.thumb_x, 141);
    // A thumb below 9 becomes 9 at any unit (round(362 · 400/100000) = 1).
    assert_eq!(scroll_metrics(404, 100_000, 2, 0, 19, 0.0).thumb, 9);
}

#[test]
fn the_horizontal_scroll_setter() {
    // 241 · 0.5 = 120.5 rounds to the even 120.
    assert_eq!(
        set_scroll_x(0.5, 0.1, false, false, 404, 1200, 2, 0, 19, 121),
        (0.5, 141)
    );
    // 0.55 / 0.1 is 5.5 in single precision: six steps, and the thumb
    // follows the snapped 0.6 (144.6 → 145).
    assert_eq!(
        set_scroll_x(0.55, 0.1, true, false, 404, 1200, 2, 0, 19, 121),
        (0.6, 166)
    );
    // A held thumb follows the unsnapped fraction (132.55 → 133).
    assert_eq!(
        set_scroll_x(0.55, 0.1, true, true, 404, 1200, 2, 0, 19, 121),
        (0.6, 154)
    );
    // Content that fits stores 0 but leaves the thumb where it was asked.
    assert_eq!(
        set_scroll_x(0.5, 0.1, false, false, 404, 400, 2, 0, 19, 121),
        (0.0, 141)
    );
    let (f, x) = set_scroll_x(f32::NAN, 0.1, false, false, 404, 1200, 2, 0, 19, 121);
    assert!(f.is_nan());
    assert_eq!(x, 21);
    assert_eq!(
        set_scroll_x(-1.0, 0.1, false, false, 404, 1200, 2, 0, 19, 121).0,
        0.0
    );
    // A vertical scrollbar narrows the client: the same half fraction
    // places the 116-pixel thumb at 141.
    assert_eq!(
        set_scroll_x(0.5, 0.1, false, false, 404, 1200, 2, 7, 19, 116),
        (0.5, 141)
    );
}

fn tall_client() -> ClientRect {
    ClientRect {
        left: 2,
        top: 26,
        right: 385,
        bottom: 402,
    }
}

#[test]
fn the_vertical_scrollbar() {
    // Track 26 + 15 = 41 to 404 − 2 − 15 = 387: 346; thumb round(346 ·
    // 376/1200 = 108.41) = 108; 41 + 238 · 0.5.
    let v = vertical_scroll_metrics(404, 2, tall_client(), 1200, 15, 4, 0, 0.5, 0.25);
    assert_eq!((v.start, v.end, v.thumb, v.thumb_y), (41, 387, 108, 160));
    // A thumb not above two units becomes 2·unit + 1.
    let v = vertical_scroll_metrics(404, 2, tall_client(), 100_000, 15, 4, 0, 0.0, 0.25);
    assert_eq!(v.thumb, 9);
    // A snapping form snaps the stored fraction (2.4 steps → 2).
    let v = vertical_scroll_metrics(404, 2, tall_client(), 1200, 15, 4, 0x400, 0.6, 0.25);
    assert_eq!(v.fraction, 0.5);
    let v = vertical_scroll_metrics(404, 2, tall_client(), 1200, 15, 4, 0x400, 0.6, 0.0);
    assert_eq!(v.fraction, 0.0);
}

#[test]
fn the_vertical_scroll_setter() {
    let c = tall_client();
    assert_eq!(
        set_scroll_y(0.5, 0.5, 0.25, 0, c, 1200, (41, 387), 108),
        None
    );
    assert_eq!(
        set_scroll_y(0.5, 0.0, 0.25, 0, c, 1200, (41, 387), 108),
        Some((0.5, 160))
    );
    assert_eq!(
        set_scroll_y(0.6, 0.0, 0.25, 0x400, c, 1200, (41, 387), 108),
        Some((0.5, 160))
    );
    assert_eq!(
        set_scroll_y(0.5, 0.0, 0.25, 0, c, 300, (41, 387), 108),
        Some((0.0, 41))
    );
}

#[test]
fn the_wheel_turns_both_bars_the_same_way() {
    let steps = (0.1, 0.25);
    assert_eq!(
        wheel_scroll(attribute::VSCROLL, 120, (0.3, 0.5), steps),
        WheelScroll::Vertical(0.25)
    );
    // Half a notch rounds to the even 0; one and a half to 2.
    assert_eq!(
        wheel_scroll(attribute::VSCROLL, 60, (0.3, 0.5), steps),
        WheelScroll::Vertical(0.5)
    );
    assert_eq!(
        wheel_scroll(attribute::VSCROLL, 180, (0.3, 0.5), steps),
        WheelScroll::Vertical(0.0)
    );
    let WheelScroll::Horizontal(x) = wheel_scroll(attribute::HSCROLL, 120, (0.3, 0.5), steps)
    else {
        panic!("a horizontal scroll");
    };
    assert!((x - 0.2).abs() < 1e-6);
    assert_eq!(wheel_scroll(0, 120, (0.3, 0.5), steps), WheelScroll::None);
}

#[test]
fn the_held_arrows_repeat() {
    let (scroll, steps) = ((0.2, 0.3), (0.05, 0.1));
    assert_eq!(
        repeat_scroll(-10, true, scroll, steps),
        Repeat::Vertical(0.4)
    );
    assert_eq!(
        repeat_scroll(-9, true, scroll, steps),
        Repeat::Vertical(0.3 - 0.1)
    );
    assert_eq!(
        repeat_scroll(-13, true, scroll, steps),
        Repeat::Horizontal(0.15)
    );
    assert_eq!(
        repeat_scroll(-14, true, scroll, steps),
        Repeat::Horizontal(0.25)
    );
    assert_eq!(repeat_scroll(-10, false, scroll, steps), Repeat::None);
    assert_eq!(repeat_scroll(3, false, scroll, steps), Repeat::Control(2));
}

#[test]
fn the_scroll_timer_moves_one_step_a_tick() {
    let (scroll, steps) = ((0.2, 0.5), (0.05, 0.25));
    // One step per 120 ms; a short tick moves nothing.
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL, 120, scroll, steps),
        ScrollBy::Vertical(0.25)
    );
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL, 119, scroll, steps),
        ScrollBy::Vertical(0.25)
    );
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL, 59, scroll, steps),
        ScrollBy::Vertical(0.5)
    );
    // Ties go to even: 0.5 tick is none, 1.5 ticks are two, 2.5 are two.
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL, 60, scroll, steps),
        ScrollBy::Vertical(0.5)
    );
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL, 180, scroll, steps),
        ScrollBy::Vertical(0.0)
    );
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL, 300, scroll, steps),
        ScrollBy::Vertical(0.0)
    );
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL, 420, scroll, steps),
        ScrollBy::Vertical(-0.5)
    );
    // The horizontal timer runs the other way.
    let ScrollBy::Horizontal(x) = scroll_by_elapsed(attribute::HSCROLL, 120, scroll, steps) else {
        panic!("a horizontal scroll");
    };
    assert!((x - 0.15).abs() < 1e-7);
    assert_eq!(scroll_by_elapsed(0, 1000, scroll, steps), ScrollBy::None);
    // The vertical bar wins when a form has both.
    assert_eq!(
        scroll_by_elapsed(attribute::VSCROLL | attribute::HSCROLL, 120, scroll, steps),
        ScrollBy::Vertical(0.25)
    );
}

#[test]
fn the_thumb_drags() {
    // 50 − 17 = 33 less the grab 5, over 398 − 30 − 121 = 247.
    assert!((drag_thumb_x(50, 5, 2, 400, 15, 121) - 28.0 / 247.0).abs() < 1e-7);
    assert!((drag_thumb_y(100, 10, (41, 387), 108) - 49.0 / 238.0).abs() < 1e-7);
    assert_eq!(drag_thumb_y(100, 10, (41, 387), 346), 0.0);
}

#[test]
fn the_tab_order() {
    let ok = ControlTab {
        tab_stop: true,
        disabled: false,
    };
    let off = ControlTab {
        tab_stop: true,
        disabled: true,
    };
    let none = ControlTab::default();
    let controls = [ok, off, ok, ok];
    assert_eq!(tab_target(&controls, 0, false), Some(2));
    assert_eq!(tab_target(&controls, 3, false), Some(0));
    assert_eq!(tab_target(&controls, 0, true), Some(3));
    assert_eq!(tab_target(&controls, 2, true), Some(0));
    // The wrap takes a disabled tab stop.
    assert_eq!(tab_target(&[off, none], 1, false), Some(0));
    assert_eq!(tab_target(&[none, none], 0, false), None);
    assert_eq!(tab_target(&[], 0, false), None);
}

#[test]
fn the_close_band() {
    for code in [-21, -20, -14, -13, -10, -9, -6, -5, -4] {
        assert!(is_close_edge(code, false), "{code}");
    }
    for code in [-22, -19, -15, -12, -11, -7, -3, -1, 0, 4] {
        assert!(!is_close_edge(code, true), "{code}");
    }
    assert!(!is_close_edge(-8, false));
    assert!(is_close_edge(-8, true));
}

#[test]
fn the_resize_cursors() {
    assert_eq!(resize_cursor(0x8000_0011), Some(2));
    assert_eq!(resize_cursor(0x8000_0012), Some(2));
    assert_eq!(resize_cursor(0x8000_0014), Some(1));
    assert_eq!(resize_cursor(0x8000_0018), Some(1));
    assert_eq!(resize_cursor(0x8000_0015), Some(3));
    assert_eq!(resize_cursor(0x8000_001a), Some(3));
    assert_eq!(resize_cursor(0x8000_0016), Some(4));
    assert_eq!(resize_cursor(0x8000_0019), Some(4));
    assert_eq!(resize_cursor(0x8000_0013), None);
    assert_eq!(resize_cursor(0x8000_0017), None);
}

fn record(x: i16, y: i16, w: i16, h: i16) -> HitControl {
    HitControl {
        x,
        y,
        width: w,
        height: h,
        style: 0,
        state: 0,
        answer: 0,
    }
}

fn node(id: u32, controls: Vec<HitControl>) -> HitNode {
    let client = ClientRect {
        left: 2,
        top: 20,
        right: 298,
        bottom: 198,
    };
    HitNode {
        id,
        frame: FormFrame {
            x: 0,
            y: 0,
            client,
            content: (296, 178),
            scroll: (0.0, 0.0),
            attributes: attribute::CLIENT_FILL,
        },
        width: 300,
        height: 200,
        visible: true,
        style: 0,
        frame_code: 0x8000_0011,
        children: Vec::new(),
        controls,
    }
}

const CTX: HitContext = HitContext {
    cursor: 3,
    slop: true,
    unit: 4,
    blocked: false,
};

#[test]
fn the_hit_test_on_records() {
    let root = node(7, vec![record(10, 10, 100, 20)]);
    // Local (50, 40) is content (48, 20).
    let hit = hit_test(&root, 50, 40, CTX);
    assert_eq!((hit.form, hit.code, hit.cursor), (Some(7), 0x1_0001, 3));
    // One past the right edge hits only with the slop (1² < 8²).
    assert_eq!(hit_test(&root, 113, 40, CTX).code, 0x1_0001);
    let exact = HitContext { slop: false, ..CTX };
    assert_eq!(hit_test(&root, 113, 40, exact).code, u32::MAX);
    // Nine past it is outside the slop (81 > 64).
    assert_eq!(hit_test(&root, 121, 40, CTX).code, u32::MAX);
    // An exact hit on a later record beats a slop hit on an earlier one.
    let pair = node(7, vec![record(0, 0, 10, 10), record(12, 0, 10, 10)]);
    // Content (11, 5) is in the slop of the first only; (12, 5) is on the
    // second and in the slop of the first.
    assert_eq!(hit_test(&pair, 13, 25, CTX).code, 0x1_0001);
    assert_eq!(hit_test(&pair, 14, 25, CTX).code, 0x1_0002);
}

#[test]
fn the_hit_test_answers_and_frame() {
    let shaped = node(
        7,
        vec![HitControl {
            answer: 0x0503_0000,
            ..record(10, 10, 100, 20)
        }],
    );
    let hit = hit_test(&shaped, 50, 40, CTX);
    assert_eq!((hit.code, hit.cursor), (0x3_0001, 5));
    let passing = node(
        7,
        vec![HitControl {
            answer: 0x80_0000,
            ..record(10, 10, 100, 20)
        }],
    );
    assert_eq!(hit_test(&passing, 50, 40, CTX).code, u32::MAX);
    let disabled = node(
        7,
        vec![HitControl {
            state: record_state::DISABLED,
            ..record(10, 10, 100, 20)
        }],
    );
    assert_eq!(hit_test(&disabled, 50, 40, CTX).code, u32::MAX);
    // A form without the 0x30 attribute bits lets the client through.
    let mut plain = node(7, Vec::new());
    plain.frame.attributes = 0;
    assert_eq!(hit_test(&plain, 50, 40, CTX).code, 0);
    // Off the client the frame answers, with its resize cursor unless the
    // skin's cursor is -2.
    let root = node(7, Vec::new());
    let hit = hit_test(&root, 1, 5, CTX);
    assert_eq!((hit.code, hit.cursor), (0x8000_0011, 2));
    let fixed = HitContext { cursor: -2, ..CTX };
    assert_eq!(hit_test(&root, 1, 5, fixed).cursor, -2);
    let blocked = HitContext {
        blocked: true,
        ..CTX
    };
    assert_eq!(hit_test(&root, 50, 40, blocked), TreeHit::default());
}

#[test]
fn the_hit_test_descends_into_children() {
    let mut child = node(2, Vec::new());
    child.frame.x = 100;
    child.frame.y = 50;
    child.frame.client = ClientRect {
        left: 0,
        top: 0,
        right: 50,
        bottom: 40,
    };
    child.width = 50;
    child.height = 40;
    let mut root = node(1, Vec::new());
    root.children.push(child);
    // Content (110, 60) is strictly inside the child.
    let hit = hit_test(&root, 112, 80, CTX);
    assert_eq!((hit.form, hit.code), (Some(2), u32::MAX));
    // Its left edge is outside it.
    let hit = hit_test(&root, 102, 80, CTX);
    assert_eq!((hit.form, hit.code), (Some(1), u32::MAX));
    // A scrolled parent: 103 of overflow at one half is 51.5, rounded to
    // the even 52.
    root.frame.content = (296 + 103, 178);
    root.frame.scroll = (0.5, 0.0);
    let hit = hit_test(&root, 112 - 52, 80, CTX);
    assert_eq!(hit.form, Some(2));
}

#[test]
fn the_point_conversions() {
    let frame = FormFrame {
        x: 10,
        y: 20,
        client: ClientRect {
            left: 2,
            top: 24,
            right: 300,
            bottom: 424,
        },
        content: (298, 500),
        scroll: (0.0, 0.5),
        attributes: 0,
    };
    // y: (5 + 24) − round(100 · 0.5) + 20.
    assert_eq!(local_to_screen((5, 5), &[frame]), (17, -1));
    let locked = FormFrame {
        attributes: attribute::LOCK_SCROLL,
        ..frame
    };
    assert_eq!(local_to_screen((5, 5), &[locked]), (17, 49));
    assert_eq!(local_to_screen((7, 8), &[]), (7, 8));
    assert_eq!(root_to_local((17, -1), (0, 0), &[frame]), (5, 5));
    assert_eq!(frame.local_to_content((5, 5)), (3, 31));
    assert_eq!(frame.offset_point((5, 5)), (7, 79));
    let odd = FormFrame {
        content: (298, 503),
        ..frame
    };
    assert_eq!(odd.scroll_offset(), (0, 52));
}

#[test]
fn the_hover_moves_the_records_bits() {
    let mut states = [0u32; 3];
    let c = hover_change(&mut states, 0, 0x3_0002, false);
    assert_eq!(
        (c.leave, c.enter, c.edge_redraw),
        (None, Some((1, 0x3_0000)), false)
    );
    assert_eq!(states[1], 0x3_0000);
    let c = hover_change(&mut states, 0x3_0002, 0, false);
    assert_eq!((c.leave, c.enter, c.edge_redraw), (Some(1), None, false));
    assert_eq!(states[1], 0);
    assert!(hover_change(&mut states, -4i32 as u32, 0, false).edge_redraw);
    assert!(hover_change(&mut states, 0, -20i32 as u32, false).edge_redraw);
    assert!(!hover_change(&mut states, 0, -3i32 as u32, false).edge_redraw);
}

#[test]
fn the_focus_moves_the_records_bits() {
    let mut states = [0u32; 3];
    let mut form = 0;
    let c = focus_change(&mut states, &mut form, 0, 2).unwrap();
    assert_eq!((c.blur, c.focus), (None, Some((1, 0))));
    assert_eq!((states[1], form), (0x400, 0x10));
    assert_eq!(focus_change(&mut states, &mut form, 2, 2), None);
    let c = focus_change(&mut states, &mut form, 2, 0).unwrap();
    assert_eq!((c.blur, c.focus, states[1], form), (Some(1), None, 0, 0));
    // A code past the records still lights the form's caret bit.
    let c = focus_change(&mut states, &mut form, 0, 9).unwrap();
    assert_eq!((c.focus, form), (None, 0x10));
}

#[test]
fn the_caret_blinks_every_fifth_frame() {
    let mut counter = 0;
    let mut form = 0x10;
    for _ in 0..4 {
        assert!(!caret_blink(style::CARET, &mut counter, &mut form));
    }
    assert!(caret_blink(style::CARET, &mut counter, &mut form));
    assert_eq!((counter, form), (0, 0));
    assert!(!caret_blink(0, &mut counter, &mut form));
    assert_eq!(counter, 0);
}

fn parent() -> ClientRect {
    ClientRect {
        left: 2,
        top: 24,
        right: 798,
        bottom: 600,
    }
}

#[test]
fn the_unset_place() {
    let p = parent();
    assert_eq!(place_unset(UNPLACED, UNPLACED, 200, 100, 0, p), (4, 36));
    assert_eq!(
        place_unset(
            UNPLACED,
            UNPLACED,
            200,
            100,
            style::CENTRE_X | style::CENTRE_Y,
            p
        ),
        (298, 238)
    );
    // The far edges are the client's right and bottom.
    assert_eq!(
        place_unset(UNPLACED, UNPLACED, 200, 100, style::RIGHT, p),
        (594, 36)
    );
    assert_eq!(
        place_unset(UNPLACED, UNPLACED, 200, 100, style::BOTTOM, p),
        (4, 496)
    );
    assert_eq!(place_unset(7, 8, 200, 100, style::RIGHT, p), (7, 8));
    // The x centre truncates, the y centre floors: −125 >> 1 is −63.
    assert_eq!(
        place_unset(
            UNPLACED,
            UNPLACED,
            901,
            701,
            style::CENTRE_X | style::CENTRE_Y,
            p
        ),
        (-52, -63)
    );
    // Showing re-centres with both axes truncating.
    assert_eq!(
        show_place(13, 17, 901, 701, style::CENTRE_X | style::CENTRE_Y, p),
        (-52, -62)
    );
    assert_eq!(show_place(13, 17, 200, 100, 0, p), (13, 17));
}

#[test]
fn the_style_words() {
    assert_eq!(set_style(0, 0, 0, true), (0, 0, 0x1000));
    assert_eq!(set_style(0, 0, 0, false), (0, 0, 0x1004));
    assert_eq!(set_style(0, 0x1_0000, 0, true), (0x10, 0x1_0041, 0x1000));
    assert_eq!(set_style(0, 0x3_0000, 0, false), (0, 0x1_0000, 0x1004));
    // A state that already has bit 4 keeps it under a task too.
    assert_eq!(set_style(0, 0x3_0000, 0x2004, true), (0, 0x1_0000, 0x1004));
}

#[test]
fn the_sibling_layers() {
    let (l, u) = (attribute::LOWER_LAYER, attribute::UPPER_LAYER);
    // The top level searches the lower layer from the first child, a
    // child raise from the second.
    assert_eq!(raise_top_level_index(&[0, l, 0], 1), 0);
    assert_eq!(raise_child_index(&[0, l, 0], 1), 1);
    assert_eq!(raise_top_level_index(&[l, l, 0], 0), 1);
    assert_eq!(raise_top_level_index(&[0, u, 0], 0), 2);
    assert_eq!(raise_top_level_index(&[0, u, 0], 1), 2);
    assert_eq!(raise_top_level_index(&[0, 0, u], 0), 1);
    let sib = |id, owner| Sibling {
        id,
        attributes: 0,
        owner,
    };
    // Raising a form takes the popups it owns above it.
    let mut kids = vec![sib(1, None), sib(2, Some(3)), sib(3, None)];
    assert!(raise_child(&mut kids, 3));
    assert_eq!(kids.iter().map(|k| k.id).collect::<Vec<_>>(), vec![1, 3, 2]);
    let mut kids = vec![sib(1, None), sib(2, Some(1)), sib(3, None)];
    assert!(raise_top_level(&mut kids, 1));
    assert_eq!(kids.iter().map(|k| k.id).collect::<Vec<_>>(), vec![2, 3, 1]);
    assert!(!raise_child(&mut kids, 9));
    assert_eq!(add_child_index(&[l, 0, u], 0), 2);
    assert_eq!(add_child_index(&[l, 0, u], l), 1);
    assert_eq!(add_child_index(&[l, 0, u], u), 3);
    assert_eq!(add_child_index(&[0, 0], l), 0);
    assert_eq!(add_child_index(&[u, u], 0), 0);
}

#[test]
fn the_active_form() {
    let u = attribute::UPPER_LAYER;
    assert!(is_active(&[0, 0, u], 1));
    assert!(!is_active(&[0, 0, u], 0));
    // A form whose word is exactly the upper layer is active first.
    assert!(is_active(&[u, 0], 0));
    assert!(!is_active(&[u | 1, 0], 0));
    let mut list = vec![1, 2, 3];
    touch_task(&mut list, 1);
    assert_eq!(list, vec![2, 3, 1]);
    touch_task(&mut list, 9);
    assert_eq!(list, vec![2, 3, 1]);
    assert_eq!(
        fallback_focus(&[(true, true), (false, true), (true, false)]),
        Some(0)
    );
    assert_eq!(fallback_focus(&[(true, false)]), None);
}

#[test]
fn the_tag_searches() {
    let kids = [
        TaggedForm {
            tag: 5,
            visible: false,
        },
        TaggedForm {
            tag: 5,
            visible: true,
        },
    ];
    assert_eq!(find_form_by_tag(&kids, 5), Some(1));
    assert_eq!(find_form_by_tag(&kids, 0), None);
    assert_eq!(focus_by_tag(&[5, 5], 5), Some(0));
    assert_eq!(focus_by_tag(&[0], 0), None);
}

#[test]
fn maximising_and_restoring() {
    let record = WindowRecord {
        x: 10,
        y: 20,
        width: 302,
        height: 402,
        border: 2,
        spacing: 3,
        right: 300,
        top: 2,
        bottom: 400,
        style: 0x80,
        attributes: 0x3_c022,
        unit: 4,
    };
    let m = maximise(record, 0x1001, (1600, 900));
    assert_eq!((m.x, m.y, m.width, m.height), (0, 0, 1600, 900));
    assert_eq!(
        (m.style, m.attributes, m.state),
        (0xf_0080, 0x3_0022, 0x1002)
    );
    assert_eq!((m.border, m.spacing), (2, 3));
    let free = maximise(
        WindowRecord {
            attributes: 0x7_c06f,
            ..record
        },
        0,
        (1600, 900),
    );
    assert_eq!((free.border, free.spacing), (0, 0));
    assert_eq!((free.attributes, free.style), (0x7_0020, 0xf_0000));
    // Each coordinate is divided by the saved unit before the multiply.
    let r = restore(record, 8, 0x1002).unwrap();
    assert_eq!(
        (r.x, r.y, r.width, r.height, r.border),
        (16, 40, 600, 800, 4)
    );
    assert_eq!((r.spacing, r.style, r.state), (3, 0x80, 0x1000));
    let r = restore(record, 4, 0).unwrap();
    assert_eq!((r.x, r.y, r.width, r.height), (8, 20, 300, 400));
    // round(1 · 3/2) is the even 2.
    let odd = WindowRecord {
        border: 3,
        unit: 2,
        ..record
    };
    assert_eq!(restore(odd, 1, 0).unwrap().border, 2);
    assert_eq!(restore(WindowRecord { unit: 0, ..record }, 4, 0), None);
}

fn resize_child(width: i32, style: u32) -> ResizeNode {
    ResizeNode {
        x: 10,
        y: 20,
        width,
        height: 40,
        style,
        has_parent: true,
        children: Vec::new(),
        controls: Vec::new(),
    }
}

#[test]
fn the_runtime_resize() {
    let client = ClientRect {
        left: 2,
        top: 0,
        right: 398,
        bottom: 250,
    };
    let mut form = ResizeNode {
        x: 100,
        y: 50,
        width: 300,
        height: 200,
        style: style::RIGHT | style::BOTTOM,
        has_parent: true,
        children: vec![
            resize_child(100, style::X_MASK | style::Y_MASK),
            resize_child(101, style::CENTRE_X),
            resize_child(501, style::CENTRE_X),
        ],
        controls: vec![
            ResizeControl {
                x: 10,
                y: 10,
                width: 80,
                height: 20,
                style: style::RIGHT,
                client,
            },
            ResizeControl {
                x: 100,
                y: 100,
                width: 80,
                height: 20,
                style: style::CENTRE_X,
                client,
            },
        ],
    };
    resize_form(&mut form, 400, 250);
    assert_eq!((form.x, form.y, form.width, form.height), (0, 0, 400, 250));
    assert_eq!((form.children[0].width, form.children[0].height), (200, 90));
    // (400 − 101) / 2 and (400 − 501) / 2 truncate: 149 and −50.
    assert_eq!(form.children[1].x, 149);
    assert_eq!(form.children[2].x, -50);
    assert_eq!((form.controls[0].x, form.controls[0].width), (110, 80));
    assert_eq!((form.controls[1].x, form.controls[1].y), (158, 100));
    let mut root = ResizeNode {
        has_parent: false,
        ..resize_child(100, style::RIGHT)
    };
    resize_form(&mut root, 200, 40);
    assert_eq!(root.x, 10);
}

#[test]
fn the_control_codes_and_lookup() {
    assert_eq!(control_index(3, 5), Some(2));
    assert_eq!(control_index(0x7f_0003, 5), Some(2));
    assert_eq!(control_index(0x1_0000, 5), None);
    assert_eq!(control_index(0, 5), None);
    assert_eq!(control_index(-3, 5), None);
    assert_eq!(control_index(7, 5), None);
    let tree = ControlTree {
        ids: vec![4, 9],
        children: vec![ControlTree {
            ids: vec![12],
            children: vec![ControlTree {
                ids: vec![15],
                children: Vec::new(),
            }],
        }],
    };
    assert_eq!(find_control(&tree, 9), Some((vec![], 1)));
    assert_eq!(find_control(&tree, 15), Some((vec![0, 0], 0)));
    assert_eq!(find_control(&tree, 13), None);
    let client = ClientRect {
        left: 2,
        top: 24,
        right: 301,
        bottom: 425,
    };
    assert_eq!(content_centre(-1, client), Some((149, 200)));
    assert_eq!(content_centre(3, client), Some((149, 200)));
    assert_eq!(content_centre(0, client), None);
    assert_eq!(content_centre(-2, client), None);
}

#[test]
fn the_routes() {
    assert_eq!(route_key(3, 5, false, |_| 0), Route::Control(2, 0x2000));
    assert_eq!(
        route_key(3, 5, true, |_| record_style::NO_KEYS),
        Route::Nothing
    );
    assert_eq!(route_key(9, 5, true, |_| 0), Route::Parent);
    assert_eq!(route_key(9, 5, false, |_| 0), Route::Nothing);
    assert_eq!(route_press(0x8000_0000, 5, true), Route::Payload);
    assert_eq!(route_press(1, 5, false), Route::Control(0, 0x3000));
    assert_eq!(route_press(0, 5, true), Route::Parent);
    assert_eq!(route_drag(3, 5, 0), Route::Control(2, 0x4000));
    assert_eq!(route_drag(3, 5, i32::MIN), Route::Nothing);
    assert_eq!(route_drag(0, 5, 0), Route::Nothing);
    // A held record takes pointer moves as message 0x30 with x low and y
    // high; frame codes and out-of-range codes take nothing.
    assert_eq!(held_move(2, 3, (5, 7)), Some((1, 0x30, 5 | (7 << 16))));
    assert_eq!(
        held_move(2, 3, (-1, 2)),
        Some((1, 0x30, 0xffff | (2 << 16)))
    );
    assert_eq!(held_move(4, 3, (5, 7)), Some((3, 0x30, 5 | (7 << 16))));
    assert_eq!(held_move(5, 3, (5, 7)), None);
    assert_eq!(held_move(0, 3, (5, 7)), None);
    // Dragging the title bar moves the form by the pointer's move.
    assert_eq!(title_drag((100, 50), (120, 80), (10, 20)), (210, 110));
}

fn grip() -> GripDrag {
    GripDrag {
        x: 100,
        y: 50,
        width: 300,
        height: 200,
        grab: (100, 50),
        moved: (-20, 10),
        min: (90, 76),
    }
}

#[test]
fn the_grip_drags() {
    // The top-left corner sets both bit groups.
    assert_eq!(grip_drag(0x8000_0015, grip()), (80, 60, 320, 190));
    let right = GripDrag {
        grab: (300, 0),
        moved: (-250, 0),
        ..grip()
    };
    assert_eq!(grip_drag(0x8000_0012, right), (100, 50, 90, 200));
    // A left edge dragged past the minimum moves the x back.
    let shrink = GripDrag {
        moved: (250, 0),
        ..grip()
    };
    assert_eq!(grip_drag(0x8000_0011, shrink), (310, 50, 90, 200));
    let up = GripDrag {
        moved: (0, -100),
        ..grip()
    };
    assert_eq!(grip_drag(0x8000_0014, up), (100, 0, 300, 250));
}

fn press(hover: i32, word: u16) -> PressInput {
    PressInput {
        hover,
        word,
        last: 2,
        place: (100, 50),
        size: (300, 200),
        fixed: false,
        thumb_offset: (5, 25),
        thumb: (20, 20),
        style: 0,
    }
}

#[test]
fn the_press_on_the_frame() {
    assert!(press_plan(press(2, 1)).to_record);
    assert_eq!(press_plan(press(2, 0x8001)), PressPlan::default());
    let p = press_plan(press(0x8000_0015u32 as i32, 1));
    assert_eq!(
        (p.held, p.grab, p.result),
        (0x8000_0015u32 as i32, Some((100, 50)), 5)
    );
    let p = press_plan(press(0x8000_001au32 as i32, 1));
    assert_eq!(p.grab, Some((300, 200)));
    let p = press_plan(press(-12, 1));
    assert_eq!((p.grab, p.thumb_jump), (Some((5, 0)), false));
    // Off the thumb, it jumps under the pointer, grabbed at its middle.
    let p = press_plan(press(-8, 1));
    assert_eq!((p.grab, p.thumb_jump), (Some((10, 0)), true));
    let p = press_plan(press(-9, 1));
    assert_eq!((p.start_repeat, p.result), (true, 0xd));
    let p = press_plan(PressInput {
        fixed: true,
        ..press(-3, 1)
    });
    assert_eq!((p.held, p.result), (0, 0));
    assert_eq!(press_plan(press(-3, 1)).grab, Some((100, 50)));
    let p = press_plan(PressInput {
        style: style::PRESS_THROUGH,
        ..press(-1, 1)
    });
    assert_eq!((p.held, p.result), (0, 0x10));
    assert_eq!(press_plan(press(-6, 1)).held, -6);
    assert_eq!(press_plan(press(-3, 2)).held, 0);
}

#[test]
fn the_release() {
    assert_eq!(
        release_actions(2, false, 5, 0, false),
        vec![ReleaseAction::Record(1)]
    );
    assert_eq!(
        release_actions(-6, true, 5, 0, true),
        vec![ReleaseAction::Restore]
    );
    assert_eq!(
        release_actions(-6, true, 5, 0, false),
        vec![ReleaseAction::Maximise]
    );
    assert!(release_actions(-6, false, 5, 0, false).is_empty());
    assert_eq!(
        release_actions(-5, true, 5, 0, false),
        vec![ReleaseAction::Hide]
    );
    assert_eq!(
        release_actions(-4, true, 5, 0, false),
        vec![ReleaseAction::Close]
    );
    assert_eq!(
        release_actions(-12, false, 5, 0x400, false),
        vec![ReleaseAction::SnapHorizontal]
    );
    assert_eq!(
        release_actions(-8, true, 5, 0x400, false),
        vec![ReleaseAction::SnapVertical]
    );
}

#[test]
fn the_form_fade() {
    let mut fade = FormFade::default();
    assert_eq!(fade.step(10), FadeStep::Draw(None));
    assert!(fade.start_show(1000, false));
    assert_eq!(fade.end, 1256);
    assert_eq!(fade.step(1000), FadeStep::Draw(Some(0.0)));
    assert_eq!(fade.step(1128), FadeStep::Draw(Some(0.5)));
    assert_eq!(fade.step(1256), FadeStep::Draw(None));
    assert_eq!(fade.state, 0);
    // A visible form does not fade in.
    assert!(!fade.start_show(2000, true));
    // The eased mode sets no alpha.
    let mut eased = FormFade {
        mode: 1,
        ..FormFade::default()
    };
    eased.start_show(0, false);
    assert_eq!(eased.step(64), FadeStep::Draw(None));
    assert!(fade.start_hide(1000));
    assert!(!fade.start_hide(1100));
    assert_eq!(fade.step(1064), FadeStep::Draw(Some(0.75)));
    assert_eq!(fade.step(1256), FadeStep::Close);
}

#[test]
fn the_drawn_records() {
    assert_eq!(
        drawn_records(&[0, 0x110, 0x100, 0x10, 0x111]),
        vec![0, 2, 3]
    );
    let [back, front] = control_passes(&[0, 0x110, 0x100]);
    assert_eq!((back.blend, front.blend), (0x10, 2));
    assert_eq!(back.draws, vec![(0, 1), (2, 1)]);
    assert_eq!(front.draws, vec![(0, 2), (2, 2)]);
}

#[test]
fn a_form_draws_through_its_fade() {
    // No fade: drawn as it is.
    let mut fade = FormFade::default();
    assert_eq!(
        form_draw(&mut fade, 100),
        FormDraw {
            paint: true,
            alpha: None,
            reset_alpha: false
        }
    );
    // Showing in mode 0: half way at 128 ticks, and the alpha is reset
    // after the draw.
    let mut fade = FormFade {
        state: 1,
        end: 256,
        mode: 0,
    };
    let draw = form_draw(&mut fade, 128);
    assert_eq!(draw.alpha, Some(0.5));
    assert!(draw.paint && draw.reset_alpha);
    // Hiding that has ended draws nothing.
    let mut fade = FormFade {
        state: 2,
        end: 512,
        mode: 0,
    };
    assert!(!form_draw(&mut fade, 600).paint);
    // A showing fade that has ended draws at full alpha and resets.
    let mut fade = FormFade {
        state: 1,
        end: 512,
        mode: 0,
    };
    let draw = form_draw(&mut fade, 600);
    assert_eq!(
        (draw.paint, draw.alpha, draw.reset_alpha),
        (true, None, true)
    );
}

fn rescale(x: i16, y: i16, w: i16, h: i16, style: u32) -> RescaleRecord {
    RescaleRecord {
        x,
        y,
        width: w,
        height: h,
        style,
        owner_span: 0,
        owner_right: 300,
        owner_bottom: 400,
    }
}

#[test]
fn the_records_rescale() {
    // w: 81/4 = 20, ·8 = 160; y: 20 + (24 − 16)/2 = 24, /4·8 = 48;
    // h: 24/4·8 = 48; y − (48 − 32)/2 = 40.
    let mut r = rescale(10, 20, 81, 24, 0);
    rescale_record(&mut r, 4, 8);
    assert_eq!((r.x, r.y, r.width, r.height), (16, 40, 160, 48));
    // An odd width is made even.
    let mut r = rescale(0, 0, 4, 24, 0);
    rescale_record(&mut r, 4, 3);
    assert_eq!(r.width, 4);
    // A right-anchored record keeps its distance from the owner's right.
    let mut r = rescale(200, 20, 80, 24, style::RIGHT);
    rescale_record(&mut r, 4, 8);
    assert_eq!(r.x, 100);
    let mut r = rescale(1, 2, 3, 4, 0);
    rescale_record(&mut r, 0, 8);
    assert_eq!((r.x, r.y, r.width, r.height), (1, 2, 3, 4));
}

#[test]
fn the_show_plan() {
    let base = ShowInput {
        has_parent: false,
        raise: true,
        y_unset: true,
        style: 0,
        adopt_title: false,
        parent_not_root: false,
        unit: 4,
    };
    let plan = show_plan(base);
    assert!(plan.attach_to_root && !plan.raise && plan.default_y);
    assert!(!plan.rescale_controls && !plan.snap_parent);
    let plan = show_plan(ShowInput {
        has_parent: true,
        style: style::CENTRE_X,
        parent_not_root: true,
        unit: 5,
        ..base
    });
    assert!(plan.raise && plan.recentre_x && !plan.recentre_y);
    assert!(plan.rescale_controls && plan.snap_parent);
    // Only the y centring and anchors keep an unset y from 36.
    assert!(plan.default_y);
    assert!(
        !show_plan(ShowInput {
            style: style::BOTTOM,
            ..base
        })
        .default_y
    );
    assert!(snaps_on_show(0x20, 1));
    assert!(!snaps_on_show(0x20, 0));
    assert!(!snaps_on_show(0x10, 3));
    assert_eq!(
        propagation_order(&[vec![1, 2], vec![3], vec![], vec![]], 0),
        vec![0, 1, 3, 2]
    );
}

#[test]
fn the_client_areas_refresh_parent_before_child() {
    // A root with two children, the first with one: the pass measures the
    // root, then each child in order, descending before moving right.
    let tree = [vec![1, 2], vec![3], vec![], vec![]];
    let mut visited = Vec::new();
    let measured = refresh_client_areas(&tree, 0, |form| {
        visited.push(form);
        ClientRect {
            left: form as i32 + 1,
            top: form as i32 + 2,
            right: form as i32 + 3,
            bottom: form as i32 + 4,
        }
    });
    assert_eq!(visited, vec![0, 1, 3, 2]);
    assert_eq!(measured.len(), 4);
    assert_eq!(
        measured[2],
        (
            3,
            ClientRect {
                left: 4,
                top: 5,
                right: 6,
                bottom: 7
            }
        )
    );
    // A leaf-only tree measures the start form once.
    assert_eq!(
        refresh_client_areas(&[vec![]], 0, |_| ClientRect::default()),
        vec![(0, ClientRect::default())]
    );
}

#[test]
fn the_close_visibility_and_destroy_plans() {
    let base = CloseInput {
        closing: false,
        closes_once: true,
        tracked_by_main: false,
        is_tooltip: false,
        is_active_child: false,
        has_callback: true,
    };
    let plan = close_plan(base);
    assert!(plan.mark_closing && plan.close_owned && plan.run_callback && plan.redraw);
    let plan = close_plan(CloseInput {
        is_active_child: true,
        ..base
    });
    assert!(plan.close_task && !plan.close_owned && !plan.redraw);
    assert_eq!(
        close_plan(CloseInput {
            closing: true,
            ..base
        }),
        ClosePlan::default()
    );
    assert!(
        !close_plan(CloseInput {
            closes_once: false,
            ..base
        })
        .mark_closing
    );
    assert_eq!(
        visibility_plan(true, true, false),
        VisibilityPlan::default()
    );
    let plan = visibility_plan(true, false, false);
    assert!(plan.changed && plan.clear_references && !plan.activate);
    let plan = visibility_plan(false, true, true);
    assert!(plan.task_follows && !plan.activate);
    assert!(visibility_plan(false, true, false).activate);
    let plan = destroy_plan(2, 4, true, true, true);
    assert_eq!(
        plan,
        DestroyPlan {
            before_hook: true,
            children_freed: 2,
            records_freed: 5,
            focus_to_root: true,
            frees_object: true,
        }
    );
    assert_eq!(destroy_plan(0, -1, true, false, false).records_freed, 0);
}

#[test]
fn the_snap_slots() {
    let client = ClientRect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };
    assert_eq!(snap_slots(0x20, client, &[]), Some(Vec::new()));
    let child = SnapChild {
        x: 20,
        y: 20,
        client: ClientRect {
            left: 0,
            top: 0,
            right: 30,
            bottom: 30,
        },
        spacing: 0,
        attributes: 0x10,
        visible: true,
    };
    // Edges 0, 20, 50, 100 both ways; the middle cell is covered.
    assert_eq!(
        snap_slots(0x20, client, &[child]),
        Some(vec![
            [0, 0, 100, 20],
            [0, 20, 20, 100],
            [50, 20, 100, 100],
            [20, 50, 50, 100],
        ])
    );
    assert_eq!(snap_slots(0x10, client, &[child]), None);
    let hidden = SnapChild {
        visible: false,
        ..child
    };
    assert_eq!(snap_slots(0x20, client, &[hidden]), Some(Vec::new()));
}

#[test]
fn the_eager_layout_resolves_unset_places_down_the_tree() {
    let client = ClientRect {
        left: 0,
        top: 0,
        right: 100,
        bottom: 100,
    };
    let mut tree = UnsetForm {
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        style: 0,
        client,
        children: vec![
            // Centred both ways: (100 − 20) / 2 = 40.
            UnsetForm {
                x: UNPLACED,
                y: UNPLACED,
                width: 20,
                height: 20,
                style: style::CENTRE_X | style::CENTRE_Y,
                client,
                children: Vec::new(),
            },
            // Right and bottom: 100 − 30 − 4 = 66.
            UnsetForm {
                x: UNPLACED,
                y: UNPLACED,
                width: 30,
                height: 30,
                style: style::RIGHT | style::BOTTOM,
                client,
                children: vec![UnsetForm {
                    // In the child's own 100 × 100 client: the default 4, 36.
                    x: UNPLACED,
                    y: UNPLACED,
                    width: 10,
                    height: 10,
                    style: 0,
                    client,
                    children: Vec::new(),
                }],
            },
            // Neither: 4, 36.
            UnsetForm {
                x: UNPLACED,
                y: UNPLACED,
                width: 10,
                height: 10,
                style: 0,
                client,
                children: Vec::new(),
            },
        ],
    };
    assert_eq!(layout_unset(&mut tree), 5);
    assert_eq!((tree.children[0].x, tree.children[0].y), (40, 40));
    assert_eq!((tree.children[1].x, tree.children[1].y), (66, 66));
    assert_eq!(
        (
            tree.children[1].children[0].x,
            tree.children[1].children[0].y
        ),
        (4, 36)
    );
    assert_eq!((tree.children[2].x, tree.children[2].y), (4, 36));
    // A set place is kept.
    tree.children[2].x = 12;
    tree.children[2].y = 13;
    assert_eq!(layout_unset(&mut tree), 5);
    assert_eq!((tree.children[2].x, tree.children[2].y), (12, 13));
}

#[test]
fn a_control_message_finds_a_nested_record_or_answers_none() {
    let tree = ControlTree {
        ids: vec![7],
        children: vec![ControlTree {
            ids: vec![2, 9],
            children: vec![ControlTree {
                ids: vec![11],
                children: Vec::new(),
            }],
        }],
    };
    let message = ControlMessage {
        message: 0x40,
        first: 1,
        second: 2,
    };
    assert_eq!(
        send_control(&tree, 11, message),
        Some((vec![0, 0], 0, message))
    );
    assert_eq!(send_control(&tree, 2, message), Some((vec![0], 0, message)));
    assert_eq!(send_control(&tree, 7, message), Some((vec![], 0, message)));
    assert_eq!(send_control(&tree, 8, message), None);
}

#[test]
fn the_close_cascade_takes_owned_popups_and_children() {
    // 0 is the task; 1 its active child; 2 a window; 3 a popup of 2 in the
    // task; 4 a popup of 1; 5 a child of 2 already closing; 6 a child of 2.
    let task = CloseNode {
        children: vec![1, 2, 3, 4],
        active_child: Some(1),
        remembered: Some(2),
        ..CloseNode::default()
    };
    let form = |owner: Option<usize>, children: Vec<usize>| CloseNode {
        closes_once: true,
        owner,
        children,
        has_callback: true,
        ..CloseNode::default()
    };
    let mut nodes = vec![
        task,
        form(None, vec![]),
        CloseNode {
            style: STYLE_CONTROLLER_ACTIONS,
            ..form(None, vec![5, 6])
        },
        form(Some(2), vec![]),
        form(Some(1), vec![]),
        CloseNode {
            closing: true,
            ..form(None, vec![])
        },
        form(None, vec![]),
    ];
    let mut steps = Vec::new();
    close_cascade(&mut nodes, 2, &mut steps);
    use CloseStep::*;
    assert_eq!(
        steps,
        [
            ControllerActions(2),
            ClosingFlag,
            // The task's popup owned by 2, then 2's own child not closing.
            ClosingFlag,
            Hide(3),
            Callback(3),
            Redraw,
            ClosingFlag,
            Hide(6),
            Callback(6),
            Redraw,
            Hide(2),
            Callback(2),
            Redraw,
        ]
    );
    assert_eq!(nodes[0].remembered, None);
    assert!(!nodes[4].closing);
    // A form already closing only restores the controller's actions.
    let mut again = Vec::new();
    close_cascade(&mut nodes, 2, &mut again);
    assert_eq!(again, [ControllerActions(2)]);
    // The active child runs its callback and closes the task instead.
    let mut active = Vec::new();
    close_cascade(&mut nodes, 1, &mut active);
    assert_eq!(active, [ClosingFlag, Callback(1), CloseTask(0)]);
    assert_eq!(nodes[0].active_child, None);
    assert!(
        !nodes[4].closing,
        "the task closes its popups, not the child"
    );
    // A form that does not close once is taken again by a later walk, and
    // a tree that links into itself stops at the depth guard.
    let mut cycle = vec![CloseNode {
        children: vec![0],
        ..CloseNode::default()
    }];
    cycle[0].task = 1;
    let mut looped = Vec::new();
    close_cascade(&mut cycle, 0, &mut looped);
    assert_eq!(looped.iter().filter(|s| **s == Redraw).count(), CLOSE_DEPTH);
    // An index past the tree closes nothing.
    let mut none = Vec::new();
    close_cascade(&mut cycle, 9, &mut none);
    assert!(none.is_empty());
}

#[test]
fn the_maximise_and_restore_steps_and_hook_words() {
    use WindowStep::*;
    assert_eq!(
        maximise_steps(true),
        [
            Show,
            SaveRecord,
            StateHook(2),
            Place,
            SetState(2),
            Resize,
            Changed,
            Redraw
        ]
    );
    assert!(!maximise_steps(false).contains(&StateHook(2)));
    assert_eq!(
        restore_steps(true, true),
        [
            Show,
            Place,
            Resize,
            FreeRecord,
            ControlSpans,
            StateHook(0),
            SetState(0),
            Changed,
            Redraw
        ]
    );
    // Without a record the hook is still told, and nothing moves.
    assert_eq!(
        restore_steps(false, true),
        [Show, StateHook(0), SetState(0), Changed, Redraw]
    );
    assert_eq!(
        restore_steps(false, false),
        [Show, SetState(0), Changed, Redraw]
    );
}

#[test]
fn the_set_controls_copy_takes_records_up_to_the_last_index() {
    use crate::gui_controls::Control;
    let mut source = vec![Control::new(1, 0, 0, 10, 10), Control::new(5, 4, 4, 20, 8)];
    source[1].text = b"Label".to_vec();
    source.push(Control::new(2, 0, 0, 1, 1));
    // The last index 1 copies two records, the text with them.
    let (table, last) = copy_control_table(&source, 1);
    assert_eq!(last, 1);
    assert_eq!(table, source[..2]);
    // A negative index copies nothing; a source short of the index gives
    // what it holds.
    assert_eq!(copy_control_table(&source, -1), (Vec::new(), -1));
    let (table, last) = copy_control_table(&source, 9);
    assert_eq!((table.len(), last), (3, 2));
}

#[test]
fn the_base_form_routes_a_key() {
    let stop = ControlTab {
        tab_stop: true,
        disabled: false,
    };
    let plain = ControlTab::default();
    let controls = [stop, plain, stop];
    // Tab from the first record (focus code 1) to the next tab stop; Shift
    // goes back, wrapping to the last stop.
    assert_eq!(form_key(9, 0, 1, &controls, |_| 0), FormKey::Focus(2));
    assert_eq!(form_key(9, 0x10, 1, &controls, |_| 0), FormKey::Focus(2));
    assert_eq!(form_key(9, 0, 3, &controls, |_| 0), FormKey::Focus(0));
    // A Tab with no stop to go to ends there.
    assert_eq!(form_key(9, 0, 1, &[stop], |_| 0), FormKey::Nothing);
    // Another key: the focused record takes it when its answer has both
    // low bits, else it goes up.
    assert_eq!(form_key(0x41, 0, 2, &controls, |_| 7), FormKey::Taken(1));
    assert_eq!(form_key(0x41, 0, 2, &controls, |_| 1), FormKey::Parent);
    // No focus (code 0, or past the records) or no records: up, unless the
    // key repeats or was typed.
    assert_eq!(form_key(0x41, 0, 0, &controls, |_| 3), FormKey::Parent);
    assert_eq!(form_key(0x41, 0, 9, &controls, |_| 3), FormKey::Parent);
    assert_eq!(form_key(9, 0, 1, &[], |_| 3), FormKey::Parent);
    assert_eq!(form_key(0x41, 1, 0, &controls, |_| 3), FormKey::Nothing);
    assert_eq!(form_key(0x41, 2, 2, &controls, |_| 0), FormKey::Nothing);
}

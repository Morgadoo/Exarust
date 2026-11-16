use super::*;
use crate::gui_core::{GuiTask, PlainForm};

/// A corner-framed, titled window with a close mark (the inventory's
/// committed words) at 100, 50, 200 × 300, unit 4.
fn window() -> BaseForm {
    let client = ClientRect {
        left: 8,
        top: 24,
        right: 192,
        bottom: 292,
    };
    BaseForm::new(
        GuiRect::new(100, 50, 200, 300),
        4,
        (0x3010, 0x3_0063, 0),
        client,
    )
}

/// A held text page with the vertical bar: 100 × 200 at 20, 30 in a
/// 300 × 300 owner, client 2..80 × 2..198, content 600 high, step 0.1.
fn page() -> BaseForm {
    let mut page = BaseForm::new(
        GuiRect::new(20, 30, 100, 200),
        4,
        (0x8c, 0x3_0632, 4),
        ClientRect {
            left: 2,
            top: 2,
            right: 80,
            bottom: 198,
        },
    );
    page.in_owner = (10, 10);
    page.owner_client = (300, 300);
    page.scroll = Some(FormScroll {
        fraction: 0.0,
        step: 0.1,
        content: 600,
    });
    page
}

fn task_with(form: BaseForm) -> (GuiTask, FormId) {
    let mut task = GuiTask::new((800, 600));
    let root = task.add(Box::new(PlainForm::new(GuiRect::new(0, 0, 800, 600), 4)));
    let mut form = form;
    form.parent = Some(root);
    let id = task.add(Box::new(form));
    (task, id)
}

fn base(task: &GuiTask, id: FormId) -> &BaseForm {
    task.form(id).and_then(FormHost::base).unwrap()
}

#[test]
fn the_line_skin_marks_the_title_band_the_close_corner_and_the_bar() {
    let w = window();
    // The wide title band: the close corner within six units of the
    // right and of the top, the band above six units.
    assert_eq!(w.hit_local(195, 3), -4);
    assert_eq!(w.hit_local(100, 3), -3);
    assert_eq!(w.hit_local(178, 22), -4);
    assert_eq!(w.hit_local(100, 22), -3);
    // Inside the client with no record: -1 (the 0x30 bits are set).
    assert_eq!(w.hit_local(100, 100), -1);
    let p = page();
    let rail = p.rail().unwrap();
    // The held page's rail: four units in from its client (the top is not
    // within a unit of the owner's), thumb two units.
    assert_eq!((rail.start, rail.end, rail.side), (18, 182, 8));
    assert_eq!(p.hit_local(90, 10), -9);
    assert_eq!(p.hit_local(90, 100), -8);
    assert_eq!(p.hit_local(90, 190), -10);
}

#[test]
fn a_title_drag_moves_the_window_and_the_close_mark_closes_it() {
    let (mut task, id) = task_with(window());
    task.update(false, (200, 52));
    assert_eq!(base(&task, id).hover, -3);
    task.press(1);
    task.update(false, (260, 90));
    assert_eq!(base(&task, id).bounds.left, 160);
    assert_eq!(base(&task, id).bounds.top, 88);
    task.release(1);
    assert!(base(&task, id).events.contains(&BaseEvent::Moved(160, 88)));
    // The close corner is now at the moved form's right.
    task.update(false, (357, 90));
    assert_eq!(base(&task, id).hover, -4);
    task.press(1);
    task.release(1);
    assert!(base(&task, id).events.contains(&BaseEvent::Close));
}

#[test]
fn the_arrows_repeat_the_thumb_drags_and_the_wheel_steps() {
    let (mut task, id) = task_with(page());
    task.clock = 1000;
    task.update(false, (20 + 90, 30 + 190));
    if let Some(f) = task.form_mut(id).and_then(FormHost::base_mut) {
        f.clock = 1000;
    }
    task.press(1);
    // The press steps once, then the timer waits seven ticks.
    assert_eq!(base(&task, id).scroll.unwrap().fraction, 0.1);
    let f = task.form_mut(id).and_then(FormHost::base_mut).unwrap();
    f.tick(1699);
    assert_eq!(f.scroll.unwrap().fraction, 0.1);
    f.tick(1700);
    assert!((f.scroll.unwrap().fraction - 0.2).abs() < 1e-6);
    // A late frame takes one step and re-arms a tick after itself.
    f.tick(1950);
    assert!((f.scroll.unwrap().fraction - 0.3).abs() < 1e-6);
    assert_eq!(f.repeat_at, Some(2050));
    task.release(1);
    // The wheel goes to the focused form: one notch up is a step back.
    task.wheel(120);
    let fraction = base(&task, id).scroll.unwrap().fraction;
    assert!((fraction - 0.2).abs() < 1e-6, "{fraction}");
    // A press on the rail off the thumb grabs its middle and jumps.
    task.update(false, (20 + 90, 30 + 150));
    task.press(1);
    let f = base(&task, id);
    let rail = f.rail().unwrap();
    assert_eq!(f.held, -8);
    assert_eq!(f.grab.0, 4);
    // The jump's fraction (150 − 18 − 4) / 156 snaps to the step (the
    // page has 0x400): 0.8, the thumb at 18 + round(156 · 0.8).
    assert!((f.scroll.unwrap().fraction - 0.8).abs() < 1e-6);
    assert_eq!(rail.thumb, 143);
    task.release(1);
}

#[test]
fn a_hovered_button_with_text_asks_for_its_tooltip() {
    let mut w = window();
    let mut button = Control::new(1, 120, 120, 80, 24);
    button.text = b"Primary".to_vec();
    button.hint = b"Hint".to_vec();
    w.records.push(button);
    let (mut task, id) = task_with(w);
    task.clock = 10;
    task.update(false, (130, 130));
    assert_eq!(task.tooltip.form, Some(id));
    let state = base(&task, id).records[0].state;
    assert_eq!(state & 0xf_0000, 0x1_0000);
    task.clock = 500;
    let tick = task.tick();
    let show = tick.show.unwrap();
    assert_eq!(show.record, 0);
    assert_eq!(task.form(id).unwrap().tooltip_text(show.record), b"Hint");
}

#[test]
fn a_press_raises_the_window_under_the_layer_rule() {
    let mut task = GuiTask::new((800, 600));
    let root = task.add(Box::new(PlainForm::new(GuiRect::new(0, 0, 800, 600), 4)));
    let mut a = window();
    a.parent = Some(root);
    let mut b = window();
    b.parent = Some(root);
    b.move_to(150, 60);
    let a = task.add(Box::new(a));
    let b = task.add(Box::new(b));
    assert_eq!(task.draw_order(), vec![root, a, b]);
    // The lower window's title band, where the upper one does not reach (a
    // press on a client without records holds nothing and raises nothing).
    task.update(false, (120, 52));
    task.press(1);
    task.release(1);
    assert_eq!(task.draw_order(), vec![root, b, a]);
}

/// The add-child places a window by its layer: an upper-layer window stays
/// above windows added after it, a lower-layer one goes under them; a
/// child form draws with its parent wherever the parent is raised; the
/// toggle's active test reads the draw order outside the upper layer, and
/// the tag search finds the visible window with the tag.
#[test]
fn an_added_window_takes_its_layers_place_and_the_tag_search_finds_it() {
    let mut task = GuiTask::new((800, 600));
    let root = task.add(Box::new(PlainForm::new(GuiRect::new(0, 0, 800, 600), 4)));
    let child_of_root = |attributes: u32, tag: u32| {
        let mut w = window();
        w.parent = Some(root);
        w.attributes |= attributes;
        w.tag = tag;
        w
    };
    let top = task.add(Box::new(child_of_root(form::attribute::UPPER_LAYER, 0)));
    let a = task.add(Box::new(child_of_root(0, 0xce1c_2030)));
    let low = task.add(Box::new(child_of_root(form::attribute::LOWER_LAYER, 0)));
    let b = task.add(Box::new(child_of_root(0, 0xce1c_2010)));
    let mut held = window();
    held.parent = Some(a);
    let held = task.add(Box::new(held));
    assert_eq!(task.draw_order(), vec![root, low, a, held, b, top]);
    assert!(task.is_active(b));
    assert!(!task.is_active(a));
    // The upper-layer form whose word is that layer alone is active as the
    // first child only; this one has the window's other bits.
    assert!(!task.is_active(top));
    assert_eq!(task.find_by_tag(0xce1c_2030), Some(a));
    assert_eq!(task.find_by_tag(0), None);
    task.activate(a);
    assert_eq!(task.draw_order(), vec![root, low, b, a, held, top]);
    assert!(task.is_active(a));
    // The activation with the control 0 leaves the key focus alone; a
    // nonzero control focuses the form with it.
    assert_eq!(task.active, None);
    task.activate_with(b, 2);
    assert_eq!((task.active, task.active_control), (Some(b), 2));
    assert!(task.is_active(b));
    task.form_mut(a).unwrap().base_mut().unwrap().visible = false;
    assert_eq!(task.find_by_tag(0xce1c_2030), None);
}

/// The base form's focus routine with no form: the remembered focus while
/// it is visible and not the focus, else the root with control -1; a focus
/// change remembers the visible form it leaves.
#[test]
fn a_focus_to_no_form_returns_to_the_remembered_one_or_the_root() {
    let mut task = GuiTask::new((800, 600));
    let root = task.add(Box::new(PlainForm::new(GuiRect::new(0, 0, 800, 600), 4)));
    let mut a = window();
    a.parent = Some(root);
    let mut b = window();
    b.parent = Some(root);
    let a = task.add(Box::new(a));
    let b = task.add(Box::new(b));
    task.focus_form(Some(a), 0);
    task.focus_form(Some(b), 0);
    assert_eq!(task.remembered, Some((a, 0)));
    task.focus_form(None, 0);
    assert_eq!(task.active, Some(a));
    assert_eq!(task.remembered, Some((b, 0)));
    // A hidden remembered form is passed over for the root.
    task.form_mut(b).unwrap().base_mut().unwrap().visible = false;
    task.focus_form(None, 0);
    assert_eq!((task.active, task.active_control), (Some(root), -1));
    // The deactivation parks the focus on the root, remembering the form.
    task.focus_form(Some(a), 0);
    task.deactivate();
    assert_eq!((task.active, task.active_control), (Some(root), -1));
    assert_eq!(task.remembered, Some((a, 0)));
}

/// The release reaches a held slider with the landing bit even when the
/// pointer is let go off it (the record tests the point itself), so the
/// slider reports its change; a button released off itself does not fire.
#[test]
fn a_slider_let_go_off_its_track_still_reports_the_change() {
    let mut w = window();
    let mut slider = Control::new(4, 120, 120, 80, 12);
    slider.reports = true;
    w.records.push(slider);
    let mut button = Control::new(1, 120, 200, 80, 24);
    button.reports = true;
    w.records.push(button);
    let (mut task, id) = task_with(w);
    task.update(false, (130, 125));
    task.press(1);
    task.update(false, (130, 180));
    task.release(1);
    let events = &base(&task, id).events;
    assert!(events.contains(&BaseEvent::Changed(0)), "{events:?}");
    task.form_mut(id)
        .unwrap()
        .base_mut()
        .unwrap()
        .events
        .clear();
    task.update(false, (130, 210));
    task.press(1);
    task.update(false, (130, 250));
    task.release(1);
    let events = &base(&task, id).events;
    assert!(!events.contains(&BaseEvent::Changed(1)), "{events:?}");
}

/// The close slot's cascade on the window manager: a window's child form
/// closes before it, both are deleted, and the tooltip request, the drop
/// they source and the modal slot on them clear; a sibling is untouched.
#[test]
fn closing_a_window_closes_its_children_and_sweeps_the_slots() {
    let mut task = GuiTask::new((800, 600));
    let root = task.add(Box::new(PlainForm::new(GuiRect::new(0, 0, 800, 600), 4)));
    let mut a = window();
    a.parent = Some(root);
    let a = task.add(Box::new(a));
    let mut child = window();
    child.parent = Some(a);
    let child = task.add(Box::new(child));
    let mut other = window();
    other.parent = Some(root);
    let other = task.add(Box::new(other));
    task.focus_set(Some(child), 0);
    (task.pointer.x, task.pointer.y) = (700, 500);
    task.tooltip.form = Some(child);
    task.drop.source = Some(a);
    task.drop.kind = 3;
    task.modal = Some(child);
    assert_eq!(task.close_form(a), 2);
    assert!(task.form(a).is_none() && task.form(child).is_none());
    assert!(task.form(other).is_some());
    assert_eq!(task.tooltip.form, None);
    assert_eq!(task.drop.kind, 0);
    assert_eq!(task.modal, None);
    // The sweep hands the focus up the tree, and each hide ends with the
    // forced pointer update: the focus is the form under the pointer, the
    // root.
    assert_eq!(task.active, Some(root));
    assert_eq!(task.draw_order(), vec![root, other]);
}

/// A press activates its window through the child raise rule: a pressed
/// lower-layer window stays under the plain ones, a pressed plain window
/// goes on top of them.
#[test]
fn a_press_raises_by_the_child_rule() {
    let mut task = GuiTask::new((800, 600));
    let root = task.add(Box::new(PlainForm::new(GuiRect::new(0, 0, 800, 600), 4)));
    let mut low = window();
    low.parent = Some(root);
    low.attributes |= form::attribute::LOWER_LAYER;
    let low = task.add(Box::new(low));
    let mut plain = window();
    plain.parent = Some(root);
    plain.move_to(300, 50);
    let plain = task.add(Box::new(plain));
    assert_eq!(task.draw_order(), vec![root, low, plain]);
    // The lower window's title band, left of the plain one.
    task.update(false, (120, 52));
    task.press(1);
    task.release(1);
    assert_eq!(task.draw_order(), vec![root, low, plain]);
    assert_eq!(task.active, Some(low));
}

/// A 100 × 100 snap container at 50, 40 (the base attribute word, client
/// 2..98) holding a 20 × 20 child placed at 10, 10 in its client (the
/// child's own client 2..18, no title spacing).
fn snap_pair(child_visible: bool) -> (GuiTask, FormId, FormId) {
    let client = ClientRect {
        left: 2,
        top: 2,
        right: 98,
        bottom: 98,
    };
    let (mut task, outer) = task_with(BaseForm::new(
        GuiRect::new(50, 40, 100, 100),
        4,
        (0, 0x3_0022, 0x2_0000),
        client,
    ));
    let mut child = BaseForm::new(
        GuiRect::new(62, 52, 20, 20),
        4,
        (0, 0x3_0022, 0),
        ClientRect {
            left: 2,
            top: 2,
            right: 18,
            bottom: 18,
        },
    );
    child.parent = Some(outer);
    child.visible = child_visible;
    let child = task.add(Box::new(child));
    (task, outer, child)
}

#[test]
fn a_shown_child_cuts_its_parents_snap_slots_around_it() {
    let (mut task, outer, child) = snap_pair(true);
    task.show_snaps(child);
    // The child's client widened by no spacing is 12..28 on both axes in
    // the container's client (0..96): the top row, the left column, the
    // right column and the cell under the child (the scan's tie takes the
    // row).
    assert_eq!(
        base(&task, outer).snaps,
        vec![
            [0, 0, 96, 12],
            [0, 12, 12, 96],
            [28, 12, 96, 96],
            [12, 28, 28, 96]
        ]
    );
    // A snap container's rebuild clears its state bit 0x20000.
    assert_eq!(base(&task, outer).state & 0x2_0000, 0);
    // The rescale walk rebuilds a form that has slots, and leaves one
    // without any alone.
    if let Some(f) = task.form_mut(outer).and_then(FormHost::base_mut) {
        f.snaps.clear();
    }
    task.rescale_snaps(outer);
    assert!(base(&task, outer).snaps.is_empty());
}

#[test]
fn a_hidden_child_or_a_plain_container_leaves_no_snap_slots() {
    // A hidden child takes no part: the one cell left is no slot.
    let (mut task, outer, child) = snap_pair(false);
    task.show_snaps(child);
    assert!(base(&task, outer).snaps.is_empty());
    // A container without the 0x20 bit keeps its slots and its state.
    let (mut task, outer, child) = snap_pair(true);
    if let Some(f) = task.form_mut(outer).and_then(FormHost::base_mut) {
        f.attributes &= !0x20;
        f.snaps = vec![[1, 2, 3, 4]];
    }
    task.show_snaps(child);
    assert_eq!(base(&task, outer).snaps, vec![[1, 2, 3, 4]]);
    assert_eq!(base(&task, outer).state & 0x2_0000, 0x2_0000);
}

#[test]
fn a_base_form_hits_its_children_inside_their_edges_and_only_when_they_answer() {
    let (mut task, outer, child) = snap_pair(true);
    // The child's client spans it, so only the parent's edge test decides.
    if let Some(f) = task.form_mut(child).and_then(FormHost::base_mut) {
        f.client = ClientRect {
            left: 0,
            top: 0,
            right: 20,
            bottom: 20,
        };
    }
    // On the child's client (no record): the child answers -1.
    assert_eq!(task.hit(70, 60).0, Some(child));
    assert_eq!(task.hit(70, 60).1, -1);
    // The child's left and top edges are outside it for the parent's test.
    assert_eq!(task.hit(62, 60).0, Some(outer));
    assert_eq!(task.hit(70, 52).0, Some(outer));
    // Its right edge is outside too; the last pixel in is inside.
    assert_eq!(task.hit(82, 60).0, Some(outer));
    assert_eq!(task.hit(81, 60).0, Some(child));
    // A child answering 0 (no 0x30 attribute bits) lets the parent answer.
    let (mut task, outer, child) = snap_pair(true);
    if let Some(f) = task.form_mut(child).and_then(FormHost::base_mut) {
        f.attributes &= !0x30;
    }
    assert_eq!(task.hit(70, 60).0, Some(outer));
    assert_eq!(task.hit(70, 60).1, -1);
    // A child with the no-hit style bit is passed over.
    let (mut task, outer, child) = snap_pair(true);
    if let Some(f) = task.form_mut(child).and_then(FormHost::base_mut) {
        f.style |= crate::form::style::NO_HIT;
    }
    assert_eq!(task.hit(70, 60).0, Some(outer));
}

/// A window with three buttons in a row, the first and the last tab stops.
fn tab_window() -> BaseForm {
    let mut w = window();
    for i in 0..3 {
        let mut button = Control::new(1, 120 + i * 50, 120, 40, 24);
        if i != 1 {
            button.style |= form::record_style::TAB_STOP;
        }
        w.records.push(button);
    }
    w
}

#[test]
fn the_key_slot_tabs_among_the_tab_stops_with_the_key_masks_shift() {
    use crate::gui_core::KeyRoute;
    use crate::gui_keys::KeyMask;
    let (mut task, id) = task_with(tab_window());
    task.update(false, (130, 130));
    assert_eq!(task.active, Some(id));
    assert_eq!(base(&task, id).hover & 0xffff, 1);
    let mut keys = KeyMask::default();
    let tab = keys.key_down(9);
    assert_eq!(task.key(tab.key, tab.flags), KeyRoute::Focus(id, 2));
    assert_eq!(base(&task, id).hover, 3);
    assert_eq!(task.active_control, 3);
    keys.key_up(9);
    // Shift with Tab: the key-down's mask has Shift as 0x20, and the base
    // form steps back only on 0x10, so it wraps forward to the first stop.
    keys.key_down(0x10);
    let shift_tab = keys.key_down(9);
    assert_eq!(shift_tab.flags, 0x20);
    assert_eq!(
        task.key(shift_tab.key, shift_tab.flags),
        KeyRoute::Focus(id, 0)
    );
    // The word 0x10 (a repeat after a key-up with Shift held) steps back.
    assert_eq!(task.key(9, 0x10), KeyRoute::Focus(id, 2));
}

#[test]
fn a_key_no_record_takes_climbs_to_the_task_unless_it_repeats_or_was_typed() {
    use crate::gui_core::KeyRoute;
    let (mut task, id) = task_with(tab_window());
    task.update(false, (130, 130));
    // A button answers the key message with 0: the key goes to the parent,
    // the task.
    assert_eq!(task.key(0x41, 0), KeyRoute::Task);
    assert_eq!(task.key(0x41, 1), KeyRoute::Ended(id));
    assert_eq!(task.key(0x61, 2), KeyRoute::Ended(id));
    // Off every record (focus code -1): no Tab, the key goes up.
    task.update(false, (280, 300));
    assert_eq!(task.active, Some(id));
    assert_eq!(task.key(9, 0), KeyRoute::Task);
    // Off the window the root has the focus: the key is the root's own.
    task.update(false, (10, 10));
    assert_eq!(task.key(9, 0), KeyRoute::Root);
}

#[test]
fn a_child_forms_key_goes_to_its_parents_key_slot() {
    use crate::gui_core::KeyRoute;
    let (mut task, id) = task_with(tab_window());
    let mut child = BaseForm::new(
        GuiRect::new(150, 200, 60, 60),
        4,
        (0, 0x3_0000, 0),
        ClientRect {
            left: 0,
            top: 0,
            right: 60,
            bottom: 60,
        },
    );
    child.parent = Some(id);
    let child = task.add(Box::new(child));
    task.focus_form(Some(child), 0);
    // The child has no record: Tab goes to the window's key slot, whose
    // focused record (selected here) steps to the next stop.
    task.form_mut(id).unwrap().select(1);
    assert_eq!(task.key(9, 0), KeyRoute::Focus(id, 2));
    assert_eq!(task.active, Some(id));
}

/// The record classes the base form's key message reaches answer it 0:
/// none of the button, check box, image button, trackbar, label, icon
/// button or picture handlers has an arm for 0xa0.
#[test]
fn the_base_record_classes_answer_the_key_message_0() {
    for class in 1..=7u16 {
        let mut records = vec![Control::new(class, 0, 0, 40, 20)];
        let mut events = Vec::new();
        let answer = gui_controls::dispatch(
            &mut records,
            0,
            Message {
                message: message::KEY_DOWN,
                param: 0x41,
                ..Message::default()
            },
            4,
            &mut events,
        );
        assert_eq!(answer, 0, "class {class}");
        assert!(events.is_empty());
    }
}

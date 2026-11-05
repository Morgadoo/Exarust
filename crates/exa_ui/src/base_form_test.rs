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
    // Four units down: the close corner within four units of the right.
    assert_eq!(w.hit_local(195, 3), -4);
    assert_eq!(w.hit_local(100, 3), -3);
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

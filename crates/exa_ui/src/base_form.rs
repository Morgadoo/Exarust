//! The base form as a live window under the line skin: the form class's
//! hit test, hover, press, drag, release, scroll-arrow repeat and wheel
//! (`TGUIForm`'s pointer slots) run on one form, so the window manager
//! ([`crate::gui_core::GuiTask`]) can move it by its title bar, close it by
//! its close mark and scroll it by its bar's arrows, thumb and the wheel.
//!
//! The rules are [`crate::form`]'s (`press_plan`, `title_drag`,
//! `drag_thumb_y`, `release_actions`, `repeat_scroll`, `wheel_scroll`,
//! `hover_change`); this module holds the state they act on and the line
//! skin's two hooks the base form calls through its skin:
//!
//! - **Frame hit.** Off the client the line skin's hit test answers
//!   ([`crate::skin::hit_line`]): -4 the close corner and -3 the title band
//!   (a corner-framed form, four units down), then, for a form with the
//!   vertical bar, -9 above the rail's start, -8 on the rail and -10 below
//!   its end, right of the client.
//! - **Scroll rail.** The base form's vertical metrics hand the bar to the
//!   skin, which stores the rail's start and end and a thumb two units long
//!   ([`crate::line_skin::refresh_line_scroll`]); the thumb's hit, press
//!   and drag read that rail.
//!
//! The press on a thumb grabs the pointer's offset into it, or (off it)
//! grabs its middle and runs the drag at once. A press on an arrow steps
//! once, lights the over flag and restarts the GUI timer seven ticks ahead
//! (`REPEAT_FIRST`); each later tick (`REPEAT_TICK`) repeats the step while
//! the pointer stays on the arrow. A drag on any other held code only keeps
//! that over flag. Records keep root coordinates here, as the game's screens
//! lay them out.

use crate::form::{
    self, ClientRect, PressInput, ReleaseAction, Repeat, WheelScroll, attribute, record_state,
};
use crate::gui_controls::{self, Control, ControlEvent, Message, message};
use crate::gui_core::{FormHost, FormId, GuiRect};
use crate::line_skin::{LineScroll, LineScrollPlace, refresh_line_scroll};
use crate::skin::{HitPlace, hit_line};

/// One GUI timer tick, in clock units: the timer runs at an interval of
/// one, which the timer table scales by 100.
pub const REPEAT_TICK: u32 = 100;
/// A press on an arrow restarts the timer seven ticks ahead (its interval
/// less −6).
pub const REPEAT_FIRST: u32 = 7 * REPEAT_TICK;

/// A form's vertical scroll: the stored fraction, one step's share and the
/// content height the fraction runs over.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormScroll {
    pub fraction: f32,
    pub step: f32,
    pub content: i32,
}

/// What the form's pointer handling asks its owner to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BaseEvent {
    /// The close mark was released over (-4).
    Close,
    /// The hide mark (-5).
    Hide,
    /// The title drag moved the form to this place.
    Moved(i32, i32),
    /// The vertical fraction changed.
    Scrolled(f32),
    /// A record reported a change (its callback).
    Changed(usize),
}

/// A live base form.
#[derive(Debug, Clone, PartialEq)]
pub struct BaseForm {
    pub parent: Option<FormId>,
    /// The form's rectangle on the screen.
    pub bounds: GuiRect,
    /// The form's place in its owner's client and that client's size (the
    /// scroll refresh measures a held form against them).
    pub in_owner: (i32, i32),
    pub owner_client: (i32, i32),
    pub unit: i32,
    /// The committed frame style, attribute and state words.
    pub style: u32,
    pub attributes: u32,
    pub state: u32,
    /// The client rectangle, form-local.
    pub client: ClientRect,
    /// The records, in root coordinates.
    pub records: Vec<Control>,
    pub scroll: Option<FormScroll>,
    pub visible: bool,
    /// The form's flag word forbids moving it.
    pub fixed: bool,
    /// The hover word: a record code (1-based, with its answer bits) or a
    /// frame mark.
    pub hover: i32,
    /// The held code, the grab its press saved and whether the pointer is
    /// still over it.
    pub held: i32,
    pub grab: (i32, i32),
    pub over: bool,
    /// The task's pointer and press point, as last handed over.
    pub pointer: (i32, i32),
    pub press_point: (i32, i32),
    /// The GUI clock, set by the owner each frame.
    pub clock: u32,
    /// When the held arrow repeats next.
    pub repeat_at: Option<u32>,
    pub doomed: bool,
    pub events: Vec<BaseEvent>,
}

impl BaseForm {
    /// A top-level form at `bounds` with the committed words and the client
    /// inset `client` (form-local).
    pub fn new(bounds: GuiRect, unit: i32, words: (u32, u32, u32), client: ClientRect) -> Self {
        Self {
            parent: None,
            bounds,
            in_owner: (bounds.left, bounds.top),
            owner_client: (0, 0),
            unit,
            style: words.0,
            attributes: words.1,
            state: words.2,
            client,
            records: Vec::new(),
            scroll: None,
            visible: true,
            fixed: false,
            hover: 0,
            held: 0,
            grab: (0, 0),
            over: false,
            pointer: (0, 0),
            press_point: (0, 0),
            clock: 0,
            repeat_at: None,
            doomed: false,
            events: Vec::new(),
        }
    }

    fn width(&self) -> i32 {
        self.bounds.width()
    }

    fn height(&self) -> i32 {
        self.bounds.height()
    }

    /// The scroll rail the line skin stores for a form with the vertical
    /// bar: its start, end and thumb (form-local).
    pub fn rail(&self) -> Option<LineScroll> {
        let scroll = self.scroll?;
        if self.attributes & attribute::VSCROLL == 0 {
            return None;
        }
        Some(refresh_line_scroll(
            LineScrollPlace {
                style: self.state,
                x: self.in_owner.0,
                y: self.in_owner.1,
                w: self.width(),
                h: self.height(),
                client_top: self.client.top,
                client_bottom: self.client.bottom,
                owner_left: 0,
                owner_top: 0,
                owner_right: self.owner_client.0,
                owner_bottom: self.owner_client.1,
                fraction: scroll.fraction,
            },
            self.unit,
        ))
    }

    /// A root point in the form's own coordinates.
    pub fn local(&self, (x, y): (i32, i32)) -> (i32, i32) {
        (x - self.bounds.left, y - self.bounds.top)
    }

    /// The line skin's frame code at a form-local point (off the client).
    pub fn frame_code(&self, x: i32, y: i32) -> i32 {
        let rail = self.rail();
        hit_line(
            HitPlace {
                flags: self.style,
                control: self.attributes,
                right: self.width(),
                left: self.client.right,
                top: rail.map_or(0, |r| r.start),
                bottom: rail.map_or(0, |r| r.end),
            },
            x,
            y,
            self.unit,
            false,
        )
    }

    /// The record a code names (its low word less one), if any.
    pub fn record_index(&self, code: i32) -> Option<usize> {
        form::control_index(code & 0xffff, self.records.len() as i32 - 1).filter(|_| code > 0)
    }

    /// The form's hit test at a form-local point: on the client, the
    /// records on their rectangles and then within two units (each enabled,
    /// hit and not passing the point through), answering the 1-based index
    /// with the answer's bits (0x10000 when it has none); a client point
    /// with no record is -1 (0 for a form without the 0x30 attribute bits);
    /// off the client the skin's frame code.
    pub fn hit_local(&self, x: i32, y: i32) -> i32 {
        let c = self.client;
        if !(c.left <= x && x < c.right && c.top <= y && y < c.bottom) {
            return self.frame_code(x, y);
        }
        let (rx, ry) = (x + self.bounds.left, y + self.bounds.top);
        let mut records = self.records.clone();
        for slop in [false, true] {
            for i in 0..records.len() {
                let r = &records[i];
                let rect = (r.x, r.y, r.x + r.w, r.y + r.h);
                if r.state & record_state::DISABLED != 0
                    || r.style & form::record_style::NO_HIT != 0
                    || !form::control_hit(rx, ry, rect, slop, self.unit)
                {
                    continue;
                }
                let mut events = Vec::new();
                let answer = gui_controls::dispatch(
                    &mut records,
                    i,
                    Message {
                        message: message::QUERY,
                        ..Message::default()
                    },
                    self.unit,
                    &mut events,
                );
                if answer & 0x80_0000 != 0 {
                    continue;
                }
                let bits = if answer & 0x7f_0000 == 0 {
                    0x1_0000
                } else {
                    answer & 0x7f_0000
                };
                return (i as i32 + 1) | bits as i32;
            }
        }
        if self.attributes & attribute::SNAPS == 0 {
            0
        } else {
            -1
        }
    }

    fn send(&mut self, i: usize, m: Message) -> u32 {
        let mut events = Vec::new();
        let result = gui_controls::dispatch(&mut self.records, i, m, self.unit, &mut events);
        for event in events {
            if let ControlEvent::Changed(i) = event {
                self.events.push(BaseEvent::Changed(i));
            }
        }
        result
    }

    /// Sets the vertical fraction through the base form's setter (clamped,
    /// snapped when the attribute word asks, 0 when the content fits).
    pub fn set_fraction(&mut self, f: f32) -> bool {
        let Some(scroll) = self.scroll else {
            return false;
        };
        let rail = self.rail();
        let track = rail.map_or((0, 0), |r| (r.start, r.end));
        let thumb = rail.map_or(0, |r| r.side);
        let Some((stored, _)) = form::set_scroll_y(
            f,
            scroll.fraction,
            scroll.step,
            self.attributes,
            self.client,
            scroll.content,
            track,
            thumb,
        ) else {
            return false;
        };
        if let Some(s) = self.scroll.as_mut() {
            s.fraction = stored;
        }
        self.events.push(BaseEvent::Scrolled(stored));
        true
    }

    /// One step of the held code's repeat ([`form::repeat_scroll`]).
    fn repeat_step(&mut self) {
        let Some(scroll) = self.scroll else {
            return;
        };
        match form::repeat_scroll(
            self.held,
            self.over,
            (0.0, scroll.fraction),
            (0.0, scroll.step),
        ) {
            Repeat::Vertical(f) => {
                self.set_fraction(f);
            }
            Repeat::Control(i) if i < self.records.len() => {
                self.send(
                    i,
                    Message {
                        message: 0x100,
                        ..Message::default()
                    },
                );
            }
            _ => {}
        }
    }

    /// The GUI timer: once due (an unsigned compare with the clock) a held
    /// arrow repeats one step and the timer re-arms one tick after `now`;
    /// a late frame takes one step, not the ticks it missed.
    pub fn tick(&mut self, now: u32) {
        self.clock = now;
        if let Some(at) = self.repeat_at
            && now >= at
        {
            self.repeat_step();
            self.repeat_at = Some(now.wrapping_add(REPEAT_TICK));
        }
    }

    /// Moves the form (and its records) to `place` on the screen.
    pub fn move_to(&mut self, x: i32, y: i32) {
        let (dx, dy) = (x - self.bounds.left, y - self.bounds.top);
        if dx == 0 && dy == 0 {
            return;
        }
        self.bounds = GuiRect::new(x, y, self.width(), self.height());
        self.in_owner = (self.in_owner.0 + dx, self.in_owner.1 + dy);
        for r in &mut self.records {
            r.x += dx;
            r.y += dy;
        }
        self.events.push(BaseEvent::Moved(x, y));
    }

    /// The drag slot at a root point: a title press moves the form, a
    /// thumb press moves the fraction, any other held code keeps whether
    /// the pointer is over it.
    pub fn drag_to(&mut self, at: (i32, i32)) {
        let (lx, ly) = self.local(at);
        match self.held {
            -3 => {
                let (x, y) = form::title_drag(self.grab, at, self.press_point);
                self.move_to(x, y);
            }
            -8 => {
                if let Some(rail) = self.rail() {
                    let f = form::drag_thumb_y(ly, self.grab.0, (rail.start, rail.end), rail.side);
                    self.set_fraction(f);
                }
            }
            held if held != 0 => {
                self.over = self.hit_local(lx, ly) == held;
            }
            _ => {}
        }
    }
}

impl FormHost for BaseForm {
    fn parent(&self) -> Option<FormId> {
        self.parent
    }

    fn bounds(&self) -> GuiRect {
        self.bounds
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hit_test(&self, x: i32, y: i32) -> i32 {
        let (lx, ly) = self.local((x, y));
        self.hit_local(lx, ly)
    }

    /// The hover slot: the old record's hover bits clear and it hears the
    /// leave, the new one takes the code's bits and is asked for its hint
    /// (a 0x100 answer asks the task for its tooltip); a frame mark is
    /// kept for the skin's hover inks.
    fn select(&mut self, code: i32) -> u32 {
        if code == self.hover {
            return 0;
        }
        let mut states: Vec<u32> = self.records.iter().map(|r| r.state).collect();
        let change = form::hover_change(&mut states, self.hover as u32, code as u32, false);
        for (r, s) in self.records.iter_mut().zip(states) {
            r.state = s;
        }
        let mut result = 3;
        if let Some(i) = change.leave {
            result |= self.send(
                i,
                Message {
                    message: message::HINT,
                    ..Message::default()
                },
            );
        }
        if let Some((i, bits)) = change.enter {
            result |= self.send(
                i,
                Message {
                    message: message::HINT,
                    param: bits,
                    over: true,
                    ..Message::default()
                },
            );
        }
        self.hover = code;
        result
    }

    fn selected_control(&self) -> i32 {
        self.hover
    }

    fn set_pointer(&mut self, at: (i32, i32), press: (i32, i32)) {
        self.pointer = at;
        self.press_point = press;
    }

    /// The press slot ([`form::press_plan`]): a hovered record takes the
    /// press; a left press on the frame holds its code and saves the grab.
    fn press(&mut self, word: u32) -> u32 {
        let (_, ly) = self.local(self.pointer);
        let rail = self.rail();
        let plan = form::press_plan(PressInput {
            hover: self.hover,
            word: word as u16,
            last: self.records.len() as i32 - 1,
            place: (self.in_owner.0, self.in_owner.1),
            size: (self.width(), self.height()),
            fixed: self.fixed,
            thumb_offset: (0, rail.map_or(0, |r| ly - r.thumb)),
            thumb: (0, rail.map_or(0, |r| r.side)),
            style: self.style,
        });
        if plan.to_record {
            let Some(i) = self.record_index(self.hover) else {
                return 0;
            };
            let (lx, _) = self.local(self.pointer);
            let x = lx + self.bounds.left - self.records[i].x;
            let result = self.send(
                i,
                Message {
                    message: message::PRESS,
                    param: if self.records[i].class == 5 {
                        gui_controls::LABEL_CLICK
                    } else {
                        x as u32
                    },
                    extra: word & 7,
                    over: true,
                    ..Message::default()
                },
            );
            if result != 0 {
                self.held = self.hover;
                self.over = true;
            }
            return result;
        }
        self.held = plan.held;
        if let Some(grab) = plan.grab {
            self.grab = grab;
        }
        if plan.start_repeat {
            self.over = true;
            self.repeat_step();
            self.repeat_at = Some(self.clock.wrapping_add(REPEAT_FIRST));
        }
        if plan.thumb_jump {
            self.drag_to(self.pointer);
        }
        if plan.result & 5 == 5 {
            self.over = true;
        }
        plan.result
    }

    fn drag_move(&mut self, x: i32, y: i32) -> u32 {
        if let Some(i) = self.record_index(self.held)
            && self.records[i].class == 4
        {
            let param = (x - self.records[i].x) as u32;
            return self.send(
                i,
                Message {
                    message: message::HOVER,
                    param,
                    over: true,
                    ..Message::default()
                },
            );
        }
        self.drag_to((x, y));
        0
    }

    /// The release slot ([`form::release_actions`]): a held record hears
    /// it, a held mark acts while the pointer is still over it; the held
    /// code and the repeat clear.
    fn release(&mut self, _word: u32) -> u32 {
        let (lx, ly) = self.local(self.pointer);
        if self.held != 0 {
            self.over = self.hit_local(lx, ly) == self.held;
        }
        let actions = form::release_actions(
            self.held,
            self.over,
            self.records.len() as i32 - 1,
            self.attributes,
            false,
        );
        for action in actions {
            match action {
                ReleaseAction::Record(i) => {
                    let over = self.over;
                    self.send(
                        i,
                        Message {
                            message: message::RELEASE,
                            param: if over { gui_controls::RELEASE_LANDS } else { 0 },
                            extra: 1,
                            over,
                            ..Message::default()
                        },
                    );
                }
                ReleaseAction::Close => self.events.push(BaseEvent::Close),
                ReleaseAction::Hide => self.events.push(BaseEvent::Hide),
                ReleaseAction::SnapVertical => {
                    if let Some(s) = self.scroll {
                        self.set_fraction(s.fraction);
                    }
                }
                _ => {}
            }
        }
        self.held = 0;
        self.over = false;
        self.repeat_at = None;
        0x83
    }

    fn wheel(&mut self, delta: i32) -> u32 {
        let Some(scroll) = self.scroll else {
            return 0;
        };
        match form::wheel_scroll(
            self.attributes,
            delta,
            (0.0, scroll.fraction),
            (0.0, scroll.step),
        ) {
            WheelScroll::Vertical(f) => {
                self.set_fraction(f);
                3
            }
            _ => 0,
        }
    }

    fn attributes(&self) -> u32 {
        self.attributes
    }

    fn hint_record(&self, control: i32) -> i32 {
        self.record_index(control).map_or(-1, |i| i as i32)
    }

    fn tooltip_text(&self, record: i32) -> Vec<u8> {
        usize::try_from(record)
            .ok()
            .and_then(|i| self.records.get(i))
            .map_or_else(Vec::new, |r| r.tooltip_text().to_vec())
    }

    fn close(&mut self, _arg: i32) {
        self.doomed = true;
    }

    fn doomed(&self) -> bool {
        self.doomed
    }

    fn base(&self) -> Option<&BaseForm> {
        Some(self)
    }

    fn base_mut(&mut self) -> Option<&mut BaseForm> {
        Some(self)
    }
}

#[cfg(test)]
#[path = "base_form_test.rs"]
mod tests;

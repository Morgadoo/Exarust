//! A form built from one constructor's 0x68-byte record array: the
//! records laid out at unit 4 ([`gui_controls::layout_records`]), the form,
//! its child panes and its records rescaled to the live unit
//! ([`gui_rescale::rescale_form`]) and placed by the form's placement word,
//! as showing a form does. The pointer goes through the control classes'
//! own messages ([`gui_controls::dispatch`]); a record whose callback ran
//! is reported by index.
//!
//! This is the Settings form's route ([`crate::settings`]) made general
//! for the forms that keep one record list and, at most, plain child panes
//! (the arena's tables): `docs/UI.md` (Arena company windows).

use crate::creation_forms::{FormRecord, control};
use crate::form::{self, ClientRect};
use crate::gui_controls::{
    self, Control, ControlEvent, ITEM_CLASSES, LayoutClient, Message, layout_records, message,
    state,
};
use crate::gui_rescale::{FormPlace, RecordPlace, rescale_form};
use crate::settings::{HIDDEN, HOVER_BITS, HOVERED, Rect, SEPARATOR_CLASS};

/// The placement word's centring bits: across (0x800) and down (0x8000).
pub const CENTRE_X: u32 = 0x800;
pub const CENTRE_Y: u32 = 0x8000;

/// A form's constructor words: its design size at unit 4, its border, its
/// placement word (the `0xb800` the arena's windows OR in), its attribute
/// word, its title, its record array and its plain child panes (form-local
/// client rectangles at unit 4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormSpec {
    pub size: (i32, i32),
    pub border: i32,
    pub placement: u32,
    pub attributes: u32,
    pub title: &'static str,
    pub records: &'static [FormRecord],
    pub children: &'static [Rect],
}

/// A record as a laid-out control: [`control`], with the trackbar's steps
/// and an item-GUI class's registered height when the record has none
/// (the item unit's classes are in the registry the layout reads).
pub fn record_control(r: &FormRecord) -> Control {
    let mut c = control(r);
    c.steps = r.steps;
    if let Some(class) = ITEM_CLASSES.iter().find(|class| class.id == r.class) {
        if c.w == 0 {
            c.w = class.size.0;
        }
        if c.h == 0 {
            c.h = class.size.1;
        }
    }
    c
}

/// The client of a form `w` × `h` with `border` all round (no title bar,
/// no scrollbar: the attribute words of these forms carry neither).
fn client_of(w: i32, h: i32, border: i32) -> ClientRect {
    form::client_rect(w, h, border, 0, 0, 0, 0, 0, 15, border)
}

fn place_of(rect: Rect, border: i32, anchors: u32, records: &[Control]) -> FormPlace {
    FormPlace {
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        border,
        design_border: 0,
        title: 0,
        client: client_of(rect.w, rect.h, border),
        content_height: 0,
        minimum: (90, 76),
        anchors,
        keep_size: false,
        title_only: false,
        container: false,
        children: Vec::new(),
        records: records
            .iter()
            .map(|c| RecordPlace {
                x: c.x as i16,
                y: c.y as i16,
                w: c.w as i16,
                h: c.h as i16,
                style: c.style,
                form_bottom: false,
            })
            .collect(),
    }
}

/// The laid-out form at a unit: its place on the screen, its child panes
/// (client-local) and its records (client-local).
#[derive(Debug, Clone, PartialEq)]
pub struct FormGeometry {
    pub unit: i32,
    pub border: i32,
    pub form: Rect,
    pub children: Vec<Rect>,
    pub controls: Vec<Control>,
}

/// Builds `spec` for a `screen` at `unit`: the records laid out at unit 4
/// in the form's client, the form, its panes and its records rescaled when
/// the unit is not 4, the trackbars resynced, then the form placed: centred
/// across (truncating) and down (a floor) by the placement word's bits,
/// else at its own (rescaled) place.
pub fn geometry(spec: &FormSpec, screen: (i32, i32), unit: i32) -> FormGeometry {
    let rect = Rect {
        x: 0,
        y: 0,
        w: spec.size.0,
        h: spec.size.1,
    };
    let client = client_of(rect.w, rect.h, spec.border);
    let mut controls: Vec<Control> = spec.records.iter().map(record_control).collect();
    layout_records(
        &mut controls,
        LayoutClient {
            inner_right: client.width(),
            border: spec.border,
            bottom: client.height(),
        },
        4,
    );
    let mut place = place_of(rect, spec.border, spec.placement, &controls);
    place.children = spec
        .children
        .iter()
        .map(|&child| place_of(child, crate::settings::BORDER, 0, &[]))
        .collect();
    let mut border = spec.border;
    if unit != 4 {
        let mut realign = |f: &mut FormPlace| f.client = client_of(f.w, f.h, f.border);
        rescale_form(
            &mut place,
            4,
            unit,
            screen,
            screen,
            6 * unit - spec.border,
            &mut realign,
        );
        for (c, r) in controls.iter_mut().zip(&place.records) {
            c.x = i32::from(r.x);
            c.y = i32::from(r.y);
            c.w = i32::from(r.w);
            c.h = i32::from(r.h);
        }
        border = place.border;
        resync_trackbars(&mut controls, unit);
    }
    let mut form = Rect {
        x: place.x,
        y: place.y,
        w: place.w,
        h: place.h,
    };
    if spec.placement & CENTRE_X != 0 {
        form.x = (screen.0 - form.w) / 2;
    }
    if spec.placement & CENTRE_Y != 0 {
        form.y = (screen.1 - form.h) >> 1;
    }
    let children = place
        .children
        .iter()
        .map(|p| Rect {
            x: p.x,
            y: p.y,
            w: p.w,
            h: p.h,
        })
        .collect();
    FormGeometry {
        unit,
        border,
        form,
        children,
        controls,
    }
}

/// A rescaled trackbar's knob: a knob bar's is `4·unit + 2`, a stepped bar
/// resyncs its knob from its width (as [`crate::settings`] does).
fn resync_trackbars(controls: &mut [Control], unit: i32) {
    let mut events = Vec::new();
    for i in 0..controls.len() {
        if controls[i].class != 4 {
            continue;
        }
        if controls[i].style & gui_controls::style::KIND == 0 {
            controls[i].knob = unit * 4 + 2;
        }
        gui_controls::dispatch(
            controls,
            i,
            Message {
                message: message::RESYNC,
                ..Message::default()
            },
            unit,
            &mut events,
        );
    }
}

/// Whether a record takes the pointer: shown, enabled, without the not-hit
/// style bit 2, and not one that lets the pointer through (a rule, or a
/// label without a callback).
fn takes_hits(c: &Control) -> bool {
    c.state & HIDDEN != HIDDEN
        && c.state & state::DISABLED == 0
        && c.style & 2 == 0
        && c.class != SEPARATOR_CLASS
        && !(c.class == 5 && !c.reports)
}

/// An open record form: its spec, geometry, the coin controls' amounts
/// (the word at 0x38, which a coin control keeps as an integer) and the
/// record the pointer holds and hovers.
#[derive(Debug, Clone, PartialEq)]
pub struct RecordForm {
    pub spec: FormSpec,
    pub screen: (i32, i32),
    pub geometry: FormGeometry,
    /// The amount word of every record (0 for those that are not coins).
    pub amounts: Vec<u32>,
    /// The form's title as drawn (the constructor's, until a refresh
    /// changes it).
    pub title: String,
    held: Option<usize>,
    hovered: Option<usize>,
}

impl RecordForm {
    /// Builds and places the form.
    pub fn open(spec: &FormSpec, screen: (i32, i32), unit: i32) -> Self {
        Self {
            spec: *spec,
            screen,
            geometry: geometry(spec, screen, unit),
            amounts: spec.records.iter().map(|r| r.amount).collect(),
            title: spec.title.to_string(),
            held: None,
            hovered: None,
        }
    }

    pub fn unit(&self) -> i32 {
        self.geometry.unit
    }

    pub fn controls(&self) -> &[Control] {
        &self.geometry.controls
    }

    /// Rebuilds the form for another screen or unit, keeping every
    /// record's state, value and text.
    pub fn resize(&mut self, screen: (i32, i32), unit: i32) {
        if screen == self.screen && unit == self.unit() {
            return;
        }
        let mut g = geometry(&self.spec, screen, unit);
        for (n, o) in g.controls.iter_mut().zip(&self.geometry.controls) {
            n.state = o.state;
            n.value = o.value;
            n.text = o.text.clone();
        }
        self.geometry = g;
        self.screen = screen;
        self.held = None;
        self.hovered = None;
    }

    /// The form's rectangle on the screen.
    pub fn rect(&self) -> Rect {
        self.geometry.form
    }

    /// The records' origin on the screen: the client's top left.
    pub fn origin(&self) -> (i32, i32) {
        let b = self.geometry.border;
        (self.geometry.form.x + b, self.geometry.form.y + b)
    }

    /// A child pane on the screen.
    pub fn child(&self, index: usize) -> Option<Rect> {
        let (ox, oy) = self.origin();
        self.geometry.children.get(index).map(|c| Rect {
            x: ox + c.x,
            y: oy + c.y,
            ..*c
        })
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.geometry.form.contains(x, y)
    }

    /// Sets a record's text.
    pub fn set_text(&mut self, i: usize, text: &str) {
        if let Some(c) = self.geometry.controls.get_mut(i) {
            c.text = text.as_bytes().to_vec();
        }
    }

    /// Sets or clears state bits of a record.
    pub fn set_bits(&mut self, i: usize, bits: u32, on: bool) {
        if let Some(c) = self.geometry.controls.get_mut(i) {
            if on {
                c.state |= bits;
            } else {
                c.state &= !bits;
            }
        }
    }

    /// Sets a record's whole state word, keeping its hover bits.
    pub fn set_state(&mut self, i: usize, word: u32) {
        if let Some(c) = self.geometry.controls.get_mut(i) {
            c.state = (c.state & HOVER_BITS) | (word & !HOVER_BITS);
        }
    }

    /// Sets a coin control's amount.
    pub fn set_amount(&mut self, i: usize, amount: u32) {
        if let Some(a) = self.amounts.get_mut(i) {
            *a = amount;
        }
    }

    /// The record under a screen point: on its rectangle, edges included,
    /// then within two units.
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        if !self.contains(x, y) {
            return None;
        }
        let (ox, oy) = self.origin();
        let (x, y) = (x - ox, y - oy);
        let unit = self.unit();
        for slop in [false, true] {
            for (i, c) in self.geometry.controls.iter().enumerate() {
                if takes_hits(c)
                    && form::control_hit(x, y, (c.x, c.y, c.x + c.w, c.y + c.h), slop, unit)
                {
                    return Some(i);
                }
            }
        }
        None
    }

    fn send(&mut self, i: usize, m: Message) -> Vec<usize> {
        let unit = self.unit();
        let mut events = Vec::new();
        gui_controls::dispatch(&mut self.geometry.controls, i, m, unit, &mut events);
        events
            .into_iter()
            .filter_map(|event| match event {
                ControlEvent::Changed(i) => Some(i),
                _ => None,
            })
            .collect()
    }

    /// A left press: the record under the pointer is held and pressed (a
    /// trackbar takes the pointer's x, a label its click word).
    pub fn press(&mut self, x: i32, y: i32) -> Vec<usize> {
        let Some(i) = self.hit(x, y) else {
            return Vec::new();
        };
        self.held = Some(i);
        let (ox, _) = self.origin();
        let param = if self.geometry.controls[i].class == 5 {
            gui_controls::LABEL_CLICK
        } else {
            (x - ox) as u32
        };
        self.send(
            i,
            Message {
                message: message::PRESS,
                param,
                extra: 1,
                over: true,
                ..Message::default()
            },
        )
    }

    /// The pointer moved: a held trackbar follows it, a held record lights
    /// while the pointer is over it; otherwise the hover bits move from the
    /// record left to the record under the pointer.
    pub fn pointer(&mut self, x: i32, y: i32) -> Vec<usize> {
        let hit = self.hit(x, y);
        if let Some(i) = self.held {
            let (ox, _) = self.origin();
            let param = if self.geometry.controls[i].class == 4 {
                (x - ox) as u32
            } else {
                0
            };
            return self.send(
                i,
                Message {
                    message: message::HOVER,
                    param,
                    over: hit == Some(i),
                    ..Message::default()
                },
            );
        }
        if hit != self.hovered {
            if let Some(c) = self.hovered.and_then(|i| self.geometry.controls.get_mut(i)) {
                c.state &= !HOVER_BITS;
            }
            if let Some(c) = hit.and_then(|i| self.geometry.controls.get_mut(i)) {
                c.state = (c.state & !HOVER_BITS) | HOVERED;
            }
            for (target, over) in [(self.hovered, false), (hit, true)] {
                if let Some(i) = target
                    && self.geometry.controls.get(i).is_some_and(|c| c.class != 4)
                {
                    self.send(
                        i,
                        Message {
                            message: message::HOVER,
                            over,
                            ..Message::default()
                        },
                    );
                }
            }
            self.hovered = hit;
        }
        Vec::new()
    }

    /// The left button's release: the held record takes it, landing when
    /// the pointer is still over it.
    pub fn release(&mut self, x: i32, y: i32) -> Vec<usize> {
        let Some(i) = self.held.take() else {
            return Vec::new();
        };
        let over = self.hit(x, y) == Some(i);
        self.send(
            i,
            Message {
                message: message::RELEASE,
                param: gui_controls::RELEASE_LANDS,
                extra: 1,
                over,
                ..Message::default()
            },
        )
    }

    /// Whether a record is held by the pointer.
    pub fn is_held(&self) -> bool {
        self.held.is_some()
    }
}

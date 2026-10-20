//! The basic form controls (`RF_GUI_Controls`): the seven control classes
//! a form's 0x68-byte records name (button, check box, image button,
//! trackbar, label, icon button, picture), their default sizes and styles,
//! and how each answers the form's messages: hover lights a control, a
//! release over it toggles it (a radio group keeps one on; a three-state
//! box goes off, on, partly on), a trackbar takes its value from the
//! pointer or steps by keys and the stick. Rules: `docs/UI.md` (Controls).

/// A control class as the form's control registry holds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlClass {
    pub id: u16,
    pub name: &'static str,
    /// The default size when a record gives none (w, h).
    pub size: (i32, i32),
    /// The style bits every control of the class has.
    pub style: u32,
    /// The class draws a picture (its records' last flag).
    pub picture: bool,
}

/// The seven classes, in the order the original registers them.
pub const CLASSES: [ControlClass; 7] = [
    ControlClass {
        id: 1,
        name: "button",
        size: (80, 24),
        style: 0x0040_3300,
        picture: false,
    },
    ControlClass {
        id: 2,
        name: "check box",
        size: (16, 16),
        style: 0x00c0_5320,
        picture: false,
    },
    ControlClass {
        id: 3,
        name: "image button",
        size: (28, 28),
        style: 0x0040_0080,
        picture: false,
    },
    ControlClass {
        id: 5,
        name: "label",
        size: (0, 20),
        style: 0x2300,
        picture: false,
    },
    ControlClass {
        id: 4,
        name: "trackbar",
        size: (0, 16),
        style: 0x0040_2120,
        picture: false,
    },
    ControlClass {
        id: 6,
        name: "icon button",
        size: (28, 28),
        style: 0,
        picture: true,
    },
    ControlClass {
        id: 7,
        name: "picture",
        size: (28, 28),
        style: 0,
        picture: true,
    },
];

/// The class with `id`.
pub fn class(id: u16) -> Option<&'static ControlClass> {
    CLASSES.iter().find(|c| c.id == id)
}

/// Style bits.
pub mod style {
    /// A toggle (a lone box flips; a radio member may turn off).
    pub const TOGGLE: u32 = 0x0080_0000;
    /// A three-state check box, or a label with a background.
    pub const THREE_STATE: u32 = 0x1000_0000;
    /// A button drawn in the small font.
    pub const SMALL_FONT: u32 = 0x0010_0000;
    /// A button the right button does not press.
    pub const LEFT_ONLY: u32 = 0x0020_0000;
    /// A trackbar's kind (0: knob with a 2-pixel margin; this: a 5-pixel
    /// marker).
    pub const KIND: u32 = 0x3000_0000;
}

/// State bits.
pub mod state {
    /// The mouse button that pressed it (1 left, 2 right).
    pub const BUTTON: u32 = 0x7;
    /// Disabled.
    pub const DISABLED: u32 = 0x10;
    /// On (checked, pressed).
    pub const ON: u32 = 0x20;
    /// Lit (the pointer is over it, or it is on).
    pub const LIT: u32 = 0x40;
    /// A three-state box's partial state.
    pub const PARTIAL: u32 = 0x80;
    /// Its change is being reported.
    pub const CHANGING: u32 = 0x10_0000;
    /// A trackbar's activation key is held.
    pub const HELD: u32 = 0x20_0000;
}

/// The messages a control answers.
pub mod message {
    pub const DRAW_BACK: u32 = 0x1;
    pub const DRAW_FRONT: u32 = 0x2;
    pub const QUERY: u32 = 0x10;
    pub const HINT: u32 = 0x20;
    pub const HOVER: u32 = 0x30;
    pub const PRESS: u32 = 0x40;
    pub const RELEASE: u32 = 0x50;
    pub const KEY: u32 = 0xc0;
    pub const SET: u32 = 0x200;
    pub const RESYNC: u32 = 0x300;
}

/// A release that lands (the press's flag).
pub const RELEASE_LANDS: u32 = 0x1_0000;
/// The activation key.
pub const ACTIVATE: u32 = 0x100;
/// A label's click.
pub const LABEL_CLICK: u32 = 0x2001;

/// The result code a toggle answers.
pub const TOGGLED: u32 = 0x83;

/// A control: the parts of its record the classes read.
#[derive(Debug, Clone, PartialEq)]
pub struct Control {
    pub class: u16,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    /// A trackbar's margin either side.
    pub margin: i32,
    /// A radio group (0: none).
    pub group: i32,
    pub text: Vec<u8>,
    pub style: u32,
    pub state: u32,
    /// A trackbar's value, 0 to 1.
    pub value: f32,
    /// A trackbar's knob width.
    pub knob: i32,
    /// A trackbar's steps (0: continuous).
    pub steps: i32,
    /// An icon button's icon and atlas cell.
    pub icon: i32,
    /// The control reports changes (its callback is set).
    pub reports: bool,
    /// An image button's right-press has its own handler.
    pub alt_press: bool,
}

impl Control {
    pub fn new(class: u16, x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            class,
            x,
            y,
            w,
            h,
            margin: 0,
            group: 0,
            text: Vec::new(),
            style: self::class(class).map_or(0, |c| c.style),
            state: 0,
            value: 0.0,
            knob: 0,
            steps: 0,
            icon: 0,
            reports: false,
            alt_press: false,
        }
    }

    fn is_on(&self) -> bool {
        self.state & state::ON != 0
    }
}

/// What a message did besides its code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlEvent {
    /// The control at this index changed.
    Changed(usize),
    /// An image button's right-press handler ran.
    AltPress(usize),
}

/// Mirrors the on bit into the lit bit.
fn light_as_on(s: u32) -> u32 {
    ((s & state::ON) << 1) | (s & !state::LIT)
}

/// Toggles control `i` of a form's `controls`: a lone toggle flips, a radio
/// member turns the group's others off (each reporting) and itself on
/// (staying on when pressed again unless it is a toggle); the lit bit
/// follows, and the control reports its change.
pub fn toggle(controls: &mut [Control], i: usize, events: &mut Vec<ControlEvent>) -> u32 {
    let group = controls[i].group;
    if group == 0 {
        if controls[i].style & style::TOGGLE == style::TOGGLE {
            controls[i].state ^= state::ON;
        }
    } else if controls[i].is_on() {
        if controls[i].style & style::TOGGLE != style::TOGGLE {
            return TOGGLED;
        }
        controls[i].state &= !state::ON;
    } else {
        for (k, c) in controls.iter_mut().enumerate() {
            if c.group == group && c.is_on() {
                c.state &= !(state::ON | state::LIT);
                if c.reports {
                    events.push(ControlEvent::Changed(k));
                }
            }
        }
        controls[i].state |= state::ON;
    }
    let c = &mut controls[i];
    c.state = light_as_on(c.state) | state::CHANGING;
    if c.reports {
        events.push(ControlEvent::Changed(i));
    }
    c.state &= !state::CHANGING;
    TOGGLED
}

/// A three-state box's press: off → on → on and partial → off.
pub fn cycle(c: &mut Control, i: usize, events: &mut Vec<ControlEvent>) -> u32 {
    if c.is_on() {
        if c.state & state::PARTIAL == 0 {
            c.state |= state::PARTIAL;
        } else {
            c.state &= !(state::ON | state::PARTIAL);
        }
    } else {
        c.state |= state::ON;
        c.state &= !state::PARTIAL;
    }
    c.state = light_as_on(c.state);
    if c.reports {
        events.push(ControlEvent::Changed(i));
    }
    TOGGLED
}

/// The pointer's hover: a control that is not on lights while the pointer
/// is over it; 3 when that changed.
pub fn hover(c: &mut Control, over: bool) -> u32 {
    if c.is_on() {
        return 0;
    }
    if (c.state & state::LIT != 0) != over {
        if over {
            c.state |= state::LIT;
        } else {
            c.state &= !state::LIT;
        }
        return 3;
    }
    0
}

/// A trackbar's value at the pointer's `x`: its offset past the control's
/// left, the margin and half the knob, over the travel (the width less
/// both margins and the knob), clamped to 0..1 (not a number gives 0) and
/// quantised to the steps; 3 when it changed.
pub fn track_to(c: &mut Control, x: i32, i: usize, events: &mut Vec<ControlEvent>) -> u32 {
    let num = i64::from(x - c.x) - i64::from(c.margin) - i64::from((c.knob as u32 >> 1) as i32);
    let den = i64::from(c.w) - i64::from(c.margin << 1) - i64::from(c.knob);
    let mut f = (num as f64 / den as f64) as f32;
    // Not a number fails the first test, as in the original.
    if f.partial_cmp(&0.0).is_none_or(|o| o.is_lt()) {
        f = 0.0;
    } else if f > 1.0 {
        f = 1.0;
    }
    if c.steps > 0 {
        f = crate::gui_font::round_i32(f64::from(c.steps as f32 * f)) as f32 / c.steps as f32;
    }
    if f != c.value {
        c.value = f;
        if c.reports {
            events.push(ControlEvent::Changed(i));
        }
        return 3;
    }
    0
}

/// A trackbar's key or stick word: the activation key holds it; a stepped
/// bar moves a step left (`0x1000`) or right (`0x2000`) by key (`& 1`) or
/// stick (`& 2`); a continuous bar moves by the stick's axes in percent
/// (`0x10`: the first, `0x20`: the second).
pub fn track_key(
    c: &mut Control,
    word: u32,
    axes: (f32, f32),
    i: usize,
    events: &mut Vec<ControlEvent>,
) {
    match word {
        ACTIVATE => c.state ^= state::HELD,
        0x200 => c.state &= !state::HELD,
        _ => {
            let mut f = c.value;
            if c.steps < 1 {
                // The axis is read as a float and the sum is taken in
                // double precision, as the original's.
                if word & 0x10 != 0 {
                    f = (f64::from(axes.0) * 0.01 + f64::from(f)) as f32;
                } else if word & 0x20 != 0 {
                    f = (f64::from(axes.1) * 0.01 + f64::from(f)) as f32;
                }
            } else {
                let d = 1.0 / c.steps as f32;
                let source = if word & 1 != 0 {
                    0xf001
                } else if word & 2 != 0 {
                    0xf002
                } else {
                    0
                };
                if source != 0 {
                    match word & source {
                        w if w == (source & 0x0fff) | 0x1000 => f = c.value - d,
                        w if w == (source & 0x0fff) | 0x2000 => f = c.value + d,
                        _ => {}
                    }
                }
            }
            let f = f.clamp(0.0, 1.0);
            if f != c.value {
                c.value = f;
                if c.reports {
                    events.push(ControlEvent::Changed(i));
                }
            }
        }
    }
}

/// Sets a trackbar's steps and its value to `at` of them; the knob is
/// the inner width over one more than the steps, at least 4 units.
pub fn set_steps(c: &mut Control, steps: i32, at: i32, unit: i32) {
    c.steps = steps;
    if steps < 1 {
        c.value = 0.0;
        return;
    }
    c.value = at as f32 / steps as f32;
    c.knob = crate::gui_font::round_i32(f64::from(c.w - (c.margin << 1)) / f64::from(steps + 1));
    if c.knob < unit * 4 {
        c.knob = unit << 2;
    }
}

/// A trackbar's set-up: a knob bar gets a knob of 4 units and 2 pixels and
/// a 2-pixel margin, a marker bar a 5-pixel marker; a stepped bar then
/// sizes its knob.
pub fn init_trackbar(c: &mut Control, unit: i32) {
    match c.style & style::KIND {
        0 => {
            if c.knob == 0 {
                c.knob = unit * 4 + 2;
            }
            c.margin = 2;
        }
        style::THREE_STATE if c.knob == 0 => c.knob = 5,
        _ => {}
    }
    if c.steps > 0 {
        let at = crate::gui_font::round_i32(f64::from(c.steps as f32 * c.value));
        set_steps(c, c.steps, at, unit);
    }
}

/// A message's words and where the pointer is.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Message {
    pub message: u32,
    /// The first word: the pointer's x for a hover or press, the release's
    /// landing flag, a key's word, a set's value.
    pub param: u32,
    /// The second word: the pressing button (1 left, 2 right), or a
    /// set's flags (`0x10`: do not report).
    pub extra: u32,
    /// The pointer is on the control.
    pub over: bool,
    /// The x components of the two stick vectors (floats), for a
    /// trackbar's keys.
    pub axes: (f32, f32),
}

/// A message to control `i` of a form's `controls`. Returns the
/// original's result code; drawing messages answer 0 here (see the draw
/// helpers).
pub fn dispatch(
    controls: &mut [Control],
    i: usize,
    m: Message,
    unit: i32,
    events: &mut Vec<ControlEvent>,
) -> u32 {
    let Message {
        message,
        param,
        extra: button,
        over,
        axes,
    } = m;
    let class = controls[i].class;
    let disabled = controls[i].state & state::DISABLED != 0;
    match (class, message) {
        (_, 0) => 0,
        (4, message::HOVER) => track_to(&mut controls[i], param as i32, i, events),
        (5, message::QUERY) => {
            if controls[i].reports {
                0
            } else {
                0x80_0000
            }
        }
        (5, message::HINT) => u32::from(controls[i].reports) * 3,
        (5, message::PRESS) => {
            if !disabled && param == LABEL_CLICK && controls[i].reports {
                events.push(ControlEvent::Changed(i));
                1
            } else {
                0
            }
        }
        (5, _) => 0,
        (1 | 3 | 6 | 7, message::HINT) => {
            let c = &controls[i];
            if c.state & 0xf_0000 == 0x1_0000 && !c.text.is_empty() {
                0x103
            } else {
                3
            }
        }
        (_, message::HINT) => 3,
        (_, message::HOVER) => hover(&mut controls[i], over),
        (4, message::PRESS) => {
            if !disabled && button == 1 {
                track_to(&mut controls[i], param as i32, i, events) | 5
            } else {
                0
            }
        }
        (1, message::PRESS) => {
            if controls[i].style & style::LEFT_ONLY == 0 || button != 2 {
                hover(&mut controls[i], over) | 5
            } else {
                0
            }
        }
        (3, message::PRESS) => {
            if disabled {
                0
            } else if controls[i].alt_press && button == 2 {
                5
            } else {
                hover(&mut controls[i], over) | 5
            }
        }
        (_, message::PRESS) => {
            if !disabled && button == 1 {
                hover(&mut controls[i], over) | 5
            } else {
                0
            }
        }
        (4, message::RELEASE) => {
            if param & RELEASE_LANDS != 0 {
                controls[i].state |= state::CHANGING;
                if controls[i].reports {
                    events.push(ControlEvent::Changed(i));
                }
                controls[i].state &= !state::CHANGING;
            }
            0
        }
        (_, message::RELEASE) => {
            if param & RELEASE_LANDS == 0 || !over {
                return 0;
            }
            if class == 3 && controls[i].alt_press && button == 2 {
                events.push(ControlEvent::AltPress(i));
                return 0;
            }
            if class == 6 || class == 7 {
                return toggle(controls, i, events);
            }
            controls[i].state |= button & state::BUTTON;
            let code = if class == 2 && controls[i].style & style::THREE_STATE != 0 {
                cycle(&mut controls[i], i, events)
            } else {
                toggle(controls, i, events)
            };
            controls[i].state &= !state::BUTTON;
            code
        }
        (4, message::KEY) => {
            track_key(&mut controls[i], param, axes, i, events);
            0
        }
        (6 | 7, message::KEY) => 0,
        (_, message::KEY) => {
            if param == ACTIVATE {
                toggle(controls, i, events);
            }
            0
        }
        (2, message::SET) => {
            if param == 0 {
                controls[i].state &= !(state::ON | state::LIT);
            } else {
                controls[i].state |= state::ON | state::LIT;
            }
            0
        }
        (4, message::SET) => {
            let c = &mut controls[i];
            c.value = if c.steps < 1 {
                f32::from_bits(param)
            } else {
                param as f32 / c.steps as f32
            };
            if button & 0x10 == 0 && c.reports {
                events.push(ControlEvent::Changed(i));
            }
            3
        }
        (4, message::RESYNC) => {
            let c = &mut controls[i];
            if c.steps > 0 {
                let at = crate::gui_font::round_i32(f64::from(c.steps as f32 * c.value));
                set_steps(c, c.steps, at, unit);
            }
            0
        }
        _ => 0,
    }
}

/// Where a check box draws its tick: centred in the box (the tick cell is
/// 4 units; a box smaller than that pushes it far off, as the original's
/// unsigned halving does). A plain box shows it while on or lit, a
/// three-state box while partly on.
pub fn tick_at(c: &Control, unit: i32) -> Option<(f32, f32)> {
    let shown = if c.style & style::THREE_STATE == 0 {
        c.state & (state::ON | state::LIT) != 0
    } else {
        c.state & state::PARTIAL != 0
    };
    shown.then(|| {
        let half = |span: i32| ((i64::from(span) - i64::from(unit << 2)) as u64 >> 1) as f64;
        (
            (f64::from(c.x) + half(c.w)) as f32,
            (f64::from(c.y) + half(c.h)) as f32,
        )
    })
}

/// Where a check box's label starts: five units right of the box.
pub fn label_x(c: &Control, unit: i32) -> i32 {
    c.x + unit * 5
}

/// Where an icon (`cell` pixels) is centred in a control.
pub fn icon_at(c: &Control, cell: i32) -> (i32, i32) {
    (c.x + (c.w - cell) / 2, c.y + (c.h - cell) / 2)
}

/// A picture's rectangle: its texture's size over 12, in units, centred
/// in the control.
pub fn picture_rect(c: &Control, texture: (u16, u16), unit: i32) -> [i32; 4] {
    let w = i32::from(texture.0 / 12) * unit;
    let h = i32::from(texture.1 / 12) * unit;
    [c.x + (c.w - w) / 2, c.y + (c.h - h) / 2, w, h]
}

/// The colour slot an icon button tints its icon with: disabled, lit or
/// plain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconInk {
    Disabled,
    Lit,
    Plain,
}

/// An icon button's tint.
pub fn icon_ink(c: &Control) -> IconInk {
    if c.state & state::DISABLED == state::DISABLED {
        IconInk::Disabled
    } else if c.state & state::LIT == state::LIT {
        IconInk::Lit
    } else {
        IconInk::Plain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_classes_and_their_defaults() {
        assert_eq!(CLASSES.map(|c| c.id), [1, 2, 3, 5, 4, 6, 7]);
        assert_eq!(class(1).map(|c| c.size), Some((80, 24)));
        assert_eq!(
            class(4).map(|c| (c.size, c.style)),
            Some(((0, 16), 0x40_2120))
        );
        assert_eq!(class(2).map(|c| c.style), Some(0xc0_5320));
        assert!(class(7).is_some_and(|c| c.picture));
        assert!(class(9).is_none());
        // The settings form's sizes agree for the classes both know.
        for id in [1, 2, 3, 4, 5] {
            assert_eq!(class(id).map(|c| c.size), Some(crate::settings::skin(id)));
        }
    }

    #[test]
    fn a_lone_toggle_flips_and_lights() {
        let mut c = vec![Control::new(2, 0, 0, 16, 16)];
        c[0].reports = true;
        let mut ev = Vec::new();
        assert_eq!(toggle(&mut c, 0, &mut ev), TOGGLED);
        assert_eq!(
            c[0].state & (state::ON | state::LIT),
            state::ON | state::LIT
        );
        toggle(&mut c, 0, &mut ev);
        assert_eq!(c[0].state, 0);
        assert_eq!(ev, vec![ControlEvent::Changed(0); 2]);
        // A button without the toggle style does not flip.
        let mut b = vec![Control::new(1, 0, 0, 80, 24)];
        b[0].style &= !style::TOGGLE;
        toggle(&mut b, 0, &mut ev);
        assert_eq!(b[0].state, 0);
    }

    #[test]
    fn a_radio_group_keeps_one_on() {
        let mut c: Vec<_> = (0..3)
            .map(|_| {
                let mut r = Control::new(2, 0, 0, 16, 16);
                r.group = 4;
                r.style &= !style::TOGGLE;
                r.reports = true;
                r
            })
            .collect();
        let mut ev = Vec::new();
        toggle(&mut c, 0, &mut ev);
        toggle(&mut c, 2, &mut ev);
        assert_eq!(
            c.iter().map(Control::is_on).collect::<Vec<_>>(),
            [false, false, true]
        );
        assert_eq!(
            ev,
            vec![
                ControlEvent::Changed(0),
                ControlEvent::Changed(0),
                ControlEvent::Changed(2)
            ]
        );
        // Pressing the one that is on keeps it on.
        ev.clear();
        assert_eq!(toggle(&mut c, 2, &mut ev), TOGGLED);
        assert!(c[2].is_on() && ev.is_empty());
    }

    #[test]
    fn a_three_state_box_cycles() {
        let mut c = Control::new(2, 0, 0, 16, 16);
        c.style |= style::THREE_STATE;
        let mut ev = Vec::new();
        cycle(&mut c, 0, &mut ev);
        assert_eq!(c.state & 0xe0, state::ON | state::LIT);
        cycle(&mut c, 0, &mut ev);
        assert_eq!(c.state & 0xe0, state::ON | state::LIT | state::PARTIAL);
        cycle(&mut c, 0, &mut ev);
        assert_eq!(c.state & 0xe0, 0);
    }

    #[test]
    fn hover_lights_what_is_off() {
        let mut c = Control::new(1, 0, 0, 80, 24);
        assert_eq!(hover(&mut c, true), 3);
        assert_eq!(hover(&mut c, true), 0);
        assert_eq!(hover(&mut c, false), 3);
        c.state = state::ON;
        assert_eq!(hover(&mut c, true), 0);
    }

    #[test]
    fn the_trackbar_follows_the_pointer() {
        let mut c = Control::new(4, 10, 0, 100, 16);
        init_trackbar(&mut c, 5);
        // A 22-pixel knob, a 2-pixel margin: travel 74 from 10 + 2 + 11.
        assert_eq!((c.knob, c.margin), (22, 2));
        let mut ev = Vec::new();
        assert_eq!(track_to(&mut c, 23, 0, &mut ev), 0);
        assert_eq!(track_to(&mut c, 60, 0, &mut ev), 3);
        assert!((c.value - 37.0 / 74.0).abs() < 1e-6);
        track_to(&mut c, 500, 0, &mut ev);
        assert_eq!(c.value, 1.0);
        // Stepped: four steps of a quarter, the knob 96 / 5 at least 20.
        set_steps(&mut c, 4, 1, 5);
        assert_eq!((c.value, c.knob), (0.25, 20));
        track_to(&mut c, 10 + 2 + 10 + 57, 0, &mut ev);
        assert_eq!(c.value, 0.75);
    }

    #[test]
    fn trackbar_keys_and_the_stick() {
        let mut c = Control::new(4, 0, 0, 100, 16);
        c.reports = true;
        set_steps(&mut c, 4, 2, 5);
        let mut ev = Vec::new();
        track_key(&mut c, 0x2001, (0.0, 0.0), 0, &mut ev);
        assert_eq!(c.value, 0.75);
        track_key(&mut c, 0x1002, (0.0, 0.0), 0, &mut ev);
        assert_eq!(c.value, 0.5);
        track_key(&mut c, 0x1001, (0.0, 0.0), 0, &mut ev);
        track_key(&mut c, 0x1001, (0.0, 0.0), 0, &mut ev);
        track_key(&mut c, 0x1001, (0.0, 0.0), 0, &mut ev);
        assert_eq!(c.value, 0.0);
        assert_eq!(ev.len(), 4);
        let mut s = Control::new(4, 0, 0, 100, 16);
        track_key(&mut s, 0x10, (30.0, -50.0), 0, &mut ev);
        assert!((s.value - 0.3).abs() < 1e-6);
        track_key(&mut s, ACTIVATE, (0.0, 0.0), 0, &mut ev);
        assert_ne!(s.state & state::HELD, 0);
    }

    #[test]
    fn messages_by_class() {
        let mut c = vec![
            Control::new(2, 0, 0, 16, 16),
            Control::new(5, 0, 20, 60, 20),
        ];
        let mut ev = Vec::new();
        // A left press lights the box; the release over it toggles.
        assert_eq!(
            dispatch(
                &mut c,
                0,
                Message {
                    message: message::PRESS,
                    param: 0,
                    extra: 1,
                    over: true,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            3 | 5
        );
        assert_eq!(
            dispatch(
                &mut c,
                0,
                Message {
                    message: message::RELEASE,
                    param: RELEASE_LANDS,
                    extra: 1,
                    over: true,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            TOGGLED
        );
        assert_eq!(c[0].state & state::BUTTON, 0);
        assert!(c[0].is_on());
        // A right press does not.
        assert_eq!(
            dispatch(
                &mut c,
                0,
                Message {
                    message: message::PRESS,
                    param: 0,
                    extra: 2,
                    over: true,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            0
        );
        assert_eq!(
            dispatch(
                &mut c,
                0,
                Message {
                    message: message::SET,
                    param: 0,
                    extra: 0,
                    over: false,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            0
        );
        assert!(!c[0].is_on());
        // A label without a callback says so, and is not clickable.
        assert_eq!(
            dispatch(
                &mut c,
                1,
                Message {
                    message: message::QUERY,
                    param: 0,
                    extra: 0,
                    over: false,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            0x80_0000
        );
        assert_eq!(
            dispatch(
                &mut c,
                1,
                Message {
                    message: message::PRESS,
                    param: LABEL_CLICK,
                    extra: 1,
                    over: true,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            0
        );
        c[1].reports = true;
        // A trackbar's press takes its value from the pointer.
        let mut bar = vec![Control::new(4, 0, 0, 100, 16)];
        init_trackbar(&mut bar[0], 5);
        assert_eq!(
            dispatch(
                &mut bar,
                0,
                Message {
                    message: message::PRESS,
                    param: 50,
                    extra: 1,
                    over: true,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            3 | 5
        );
        // A button with text and a hint asks for its tooltip.
        let mut b = vec![Control::new(1, 0, 0, 80, 24)];
        b[0].text = b"Go".to_vec();
        b[0].state = 0x1_0000;
        assert_eq!(
            dispatch(
                &mut b,
                0,
                Message {
                    message: message::HINT,
                    ..Default::default()
                },
                5,
                &mut ev
            ),
            0x103
        );
        assert_eq!(
            dispatch(
                &mut c,
                1,
                Message {
                    message: message::PRESS,
                    param: LABEL_CLICK,
                    extra: 1,
                    over: true,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            1
        );
        let mut t = vec![Control::new(4, 0, 0, 100, 16)];
        t[0].steps = 4;
        assert_eq!(
            dispatch(
                &mut t,
                0,
                Message {
                    message: message::SET,
                    param: 3,
                    extra: 0,
                    over: false,
                    axes: (0.0, 0.0)
                },
                5,
                &mut ev
            ),
            3
        );
        assert_eq!(t[0].value, 0.75);
        let mut img = vec![Control::new(3, 0, 0, 28, 28)];
        img[0].alt_press = true;
        dispatch(
            &mut img,
            0,
            Message {
                message: message::RELEASE,
                param: RELEASE_LANDS,
                extra: 2,
                over: true,
                axes: (0.0, 0.0),
            },
            5,
            &mut ev,
        );
        assert_eq!(ev.last(), Some(&ControlEvent::AltPress(0)));
    }

    #[test]
    fn tick_label_icon_and_picture_places() {
        let mut c = Control::new(2, 10, 20, 30, 30);
        assert_eq!(tick_at(&c, 5), None);
        c.state = state::ON;
        assert_eq!(tick_at(&c, 5), Some((15.0, 25.0)));
        assert_eq!(label_x(&c, 5), 35);
        assert_eq!(icon_at(&c, 20), (15, 25));
        assert_eq!(picture_rect(&c, (120, 60), 3), [10, 27, 30, 15]);
        c.state = state::LIT | state::DISABLED;
        assert_eq!(icon_ink(&c), IconInk::Disabled);
    }
}

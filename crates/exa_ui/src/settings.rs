//! The Settings form (`TSGSettingDlg`): its three 0x68-byte control record
//! arrays (the tabs and buttons, the Settings page and the Controls page),
//! laid out and rescaled by the GUI's own passes, its two child pages and
//! how it answers the pointer (`docs/UI.md`, Settings form).
//!
//! The form is 434 × 536 at unit 4, centred (style 0xb810), with the
//! upper-layer attribute; the Settings page is a child at 0, 64 of the
//! form's width less 8 by 408, the Controls page a scrolled child of the
//! width less 16 by 416 over 676 of content, placed by the page switch.
//! The records are laid out at unit 4 ([`gui_controls::layout_records`])
//! and the form, its children and records rescaled to the live unit
//! ([`gui_rescale::rescale_form`]), as showing a form does. [`FormState`]
//! holds the laid-out controls and routes the pointer through the control
//! classes' own messages ([`gui_controls::dispatch`]); what a change asks
//! of the game is a [`FormEvent`].
//!
//! The option rows' behaviour is here too: the row ids, what each slider
//! step or checkbox writes ([`on_slider`], [`on_toggle`]), what the rows show
//! when the page opens ([`populate`]) and the labels from
//! [`exa_formats::hw_config`]. The form's four buttons and two tabs are
//! [`form_action`].

use exa_formats::hw_config::{self, Options, Scale};

use crate::controls::extended_class;
use crate::form::{self, ClientRect, WheelScroll};
use crate::gui_controls::{
    self, Control, ControlEvent, LayoutClient, Message, layout_records, message, state,
};
use crate::gui_rescale::{FormPlace, RecordPlace, rescale_form};

/// The design dialog size at unit 4.
pub const DESIGN: (i32, i32) = (434, 536);
/// The Settings page's height at unit 4.
pub const SETTINGS_HEIGHT: i32 = 408;
/// The Controls page's height at unit 4 (before the page switch sets it).
pub const CONTROLS_HEIGHT: i32 = 416;
/// The Controls page's content height at unit 4.
pub const CONTROLS_CONTENT: i32 = 676;
/// The form's style word: centred across (0x800) and down (0x8000), with
/// the centred-title frame bits 0x3000 and 0x10.
pub const FORM_STYLE: u32 = 0xb810;
/// The form's attribute word: the base constructor's 0x30022 with the
/// upper-layer bit 0x1000 the Settings constructor sets.
pub const FORM_ATTRIBUTES: u32 = 0x3_1022;
/// A child page's attribute word: the base constructor's.
pub const CHILD_ATTRIBUTES: u32 = 0x3_0022;
/// The Controls page's attribute word: the base word with 0x80 and the
/// vertical scrollbar (0x200) its constructor sets.
pub const CONTROLS_ATTRIBUTES: u32 = 0x3_0022 | 0x280;
/// The Controls page's style bits (0x88), which its constructor sets.
pub const CONTROLS_STYLE: u32 = 0x88;
/// The forms' border, which is also their client inset (no title bar).
pub const BORDER: i32 = 2;
/// The vertical scrollbar's width taken off a scrolled client's right: the
/// rail (15 at unit 4) and two pixels, as the form background draws the
/// bar's frame 17 pixels in. A guess: the client's scroll inset was not
/// traced.
pub const SCROLL_INSET: i32 = 17;
/// The form's vertical scroll step (the base constructor's), which the
/// wheel moves the Controls page by per notch.
pub const SCROLL_STEP: f32 = 0.25;
/// The hidden state the page switch gives a record (0x100 with the
/// disabled bit); the control draw skips it.
pub const HIDDEN: u32 = 0x110;
/// The form's title.
pub const TITLE: &str = "Settings";
/// A record's hover bits.
pub const HOVER_BITS: u32 = 0xf_0000;
/// The hover word a hovered record takes (it answers no bits of its own).
pub const HOVERED: u32 = 0x1_0000;
/// The video memory readout when the driver gives no figure.
pub const UNKNOWN: &str = "Unknown";

/// The Controls page's power-tree help text, exactly as the original's
/// text box holds it: `[tcol=RRGGBB]` spans in the highlight colour, and
/// `\r\r` between its three paragraphs.
pub const HELP: &str = "Hold [tcol=FFE399]Powers[/tcol] to access the power tree.\r\rPress [tcol=FFE399]Use Power[/tcol] with the power bar open to use a power directly without changing which is active.\r\rYou can toggle in and out of dialogue mode without ending the conversation by pressing [tcol=FFE399]Talk[/tcol].";

/// The help box's rectangle inside the Controls page at `unit`, for a page
/// client `width` wide: the constructor's 2, 48, width less 4 units and 50
/// units, scaled by whole units.
pub fn help_box(width: i32, unit: i32) -> Rect {
    Rect {
        x: 2 * unit,
        y: 48 * unit,
        w: width - 4 * unit,
        h: 50 * unit,
    }
}

/// A rectangle in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    /// Whether the point is inside (edges at the left and top included).
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x && y >= self.y && x < self.x + self.w && y < self.y + self.h
    }
}

/// A control record, as the original's arrays store it: the class and id
/// words, the place and size (16-bit, anchors 0x8000…0x800a read signed),
/// the radio group, the text, the style and state words, the step count
/// and whether a callback is set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub class: u16,
    pub id: u16,
    /// Pixels, or an anchor code 0x8000..0x800a.
    pub x: i32,
    /// 4-px grid units, or an anchor code.
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub group: u16,
    pub text: Option<&'static str>,
    pub style: u32,
    pub state: u32,
    pub steps: i32,
    /// The record has a callback: the control reports its changes.
    pub reports: bool,
}

const fn base(class: u16, id: u16, x: i32, y: i32, w: i32, h: i32) -> Record {
    Record {
        class,
        id,
        x,
        y,
        w,
        h,
        group: 0,
        text: None,
        style: 0,
        state: 0,
        steps: 0,
        reports: false,
    }
}

const fn label(id: u16, x: i32, y: i32, w: i32, text: Option<&'static str>, style: u32) -> Record {
    Record {
        text,
        style,
        ..base(5, id, x, y, w, 0)
    }
}

#[allow(clippy::too_many_arguments)]
const fn button(
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    group: u16,
    text: &'static str,
    style: u32,
    state: u32,
) -> Record {
    Record {
        group,
        text: Some(text),
        style,
        state,
        reports: true,
        ..base(1, 0, x, y, w, h)
    }
}

const fn rule(x: i32, y: i32, w: i32) -> Record {
    base(SEPARATOR_CLASS, 0, x, y, w, 0)
}

const fn track(id: u16, steps: i32) -> Record {
    Record {
        style: 0x40_5320,
        steps,
        reports: true,
        ..base(4, id, 168, 0x8000, 140, 0)
    }
}

const fn check(id: u16, x: i32, y: i32, w: i32, text: &'static str) -> Record {
    Record {
        text: Some(text),
        reports: true,
        ..base(2, id, x, y, w, 0)
    }
}

/// The separator's class id (an extended control).
pub const SEPARATOR_CLASS: u16 = crate::controls::SEPARATOR;

/// The default skin sizes `(w, h)` by class.
pub fn skin(class: u16) -> (i32, i32) {
    match class {
        0x01 => (80, 24),
        0x02 => (16, 16),
        0x03 => (28, 28),
        0x04 => (0, 16),
        0x05 => (0, 20),
        0x13 => (0, 3),
        0x10 => (60, 20),
        0x11 | 0x12 => (80, 24),
        _ => (0, 0),
    }
}

/// The tab and button array: the SETTINGS and CONTROLS tabs (radio group
/// 1), the Keyboard and Controller tabs (group 2, hidden without a
/// controller), the two rules, and Defaults, Apply and Cancel. The
/// constructor sets records 0 to 8.
pub const TABS: [Record; 9] = [
    button(12, 5, 192, 32, 1, "SETTINGS", 0x40_3320, 0x60),
    button(212, 0x8000, 192, 32, 1, "CONTROLS", 0x40_3320, 0),
    button(12, 0x8006, 192, 0, 2, "Keyboard", 0x40_3320, 0x170),
    button(212, 0x8000, 192, 0, 2, "Controller", 0x40_3300, 0x110),
    rule(16, 0x8008, 400),
    rule(16, 116, 400),
    button(40, 0x8009, 100, 0, 0, "Defaults", 0x2040_3300, 0),
    button(0x8006, 0x8000, 104, 0, 0, "Apply", 0x2040_3300, 0),
    button(0x8006, 0x8000, 100, 0, 0, "Cancel", 0x2040_3300, 0),
];

/// The Settings page's array (27 records): the device and memory readouts,
/// a rule, then each option's label and slider or checkbox.
pub const SETTINGS: [Record; 27] = [
    label(0, 16, 0, 60, Some("Video Device: "), 0),
    label(1, 168, 0x8000, 248, Some(UNKNOWN), 0x2000_2380),
    label(0, 16, 0x8008, 60, Some("Video Memory: "), 0),
    label(2, 168, 0x8000, 248, None, 0x2380),
    rule(16, 0x8009, 400),
    label(0, 16, 0x8009, 60, Some("Audio volume: "), 0),
    track(3, 0),
    check(4, 320, 0x8000, 56, "Music"),
    label(0, 16, 0x8006, 60, Some("Super sampling: "), 0),
    track(5, 3),
    label(0, 16, 0x8007, 60, Some("Shading quality: "), 0),
    track(6, 3),
    label(0, 16, 0x8007, 60, Some("Texture detail: "), 0),
    track(7, 1),
    label(0, 16, 0x8008, 60, Some("Texture quality: "), 0),
    track(8, 3),
    label(0, 16, 0x8007, 60, Some("Shadow detail: "), 0),
    track(9, 1),
    label(0, 16, 0x8008, 60, Some("Shadow quality: "), 0),
    track(11, 3),
    label(0, 16, 0x8007, 60, Some("HW antialiasing: "), 0),
    track(12, 3),
    check(13, 72, 0x8007, 80, "Vertical sync "),
    check(14, 228, 0x8000, 80, "Show framerate"),
    check(15, 72, 0x8008, 80, "Window mode"),
    check(16, 228, 0x8000, 80, "Show gameplay tips"),
    check(17, 148, 0x8008, 100, "Early Checkpoints"),
];

/// One Controls binding: a label row and its key cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub label: &'static str,
    /// The original's action code for the row: the key cell's control id,
    /// one more than the row's slot in the saved key table.
    pub action: u16,
    /// The row's slot in the player's 23-entry binding table
    /// (`exa_game::bindings`), through the original's conversion from the
    /// saved table.
    pub slot: Option<usize>,
    /// The template's key text (the shipped table's key may differ).
    pub key: &'static str,
    /// The row's y in design pixels: the first row's 2 grid units plus
    /// the anchors' rows (7, or 8 and 9 between the groups), times 4.
    pub y: i32,
    /// Whether the row has a key cell.
    pub keyed: bool,
}

const fn bind(label: &'static str, action: u16, slot: usize, key: &'static str, y: i32) -> Binding {
    Binding {
        label,
        action,
        slot: Some(slot),
        key,
        y,
        keyed: true,
    }
}

/// The Controls bindings: 22 label rows, each with a key cell (the record
/// table's 44 records are label and key-cell pairs; every key cell opens the
/// key-read screen). The primary mouse button's slot 0 has no row.
pub const BINDINGS: [Binding; 22] = [
    bind("Forward", 1, 1, "W", 8),
    bind("Back", 2, 3, "S", 36),
    bind("Left", 3, 2, "A", 64),
    bind("Right", 4, 4, "D", 92),
    bind("Run/Sprint", 5, 6, "Shift", 120),
    bind("Crouch", 6, 5, "X", 148),
    bind("Orbit Camera", 7, 8, "", 180),
    bind("Cursor Look", 8, 9, "SPACE", 208),
    bind("Toggle Combat", 0xA, 12, "TAB", 240),
    bind("Alternate Attack", 0xE, 7, "Alt", 268),
    bind("Switch Equipped", 0xF, 14, "R", 296),
    bind("Flip weapon", 0x17, 15, "F", 324),
    bind("Interact", 9, 10, "Ctrl", 352),
    bind("Inventory", 0xB, 11, "I", 384),
    bind("Skills", 0xC, 13, "K", 412),
    bind("Journal", 0xD, 16, "J", 440),
    bind("Talk", 0x10, 17, "T", 468),
    bind("Use Power", 0x15, 20, "", 504),
    bind("Power Bar", 0x11, 18, "", 532),
    bind("Previous Power", 0x13, 22, "", 560),
    bind("Next Power", 0x14, 21, "", 588),
    bind("Power Tree", 0x12, 19, "P", 616),
];

/// The key cells' style word: the alternate label front (0x20000000) with
/// a frame (0x80), centred (0x3000) and the text centred down (0x300).
pub const KEY_CELL_STYLE: u32 = 0x2000_3380;

/// The Controls page's 44 records: each row's label (x 80, 68 wide, no
/// callback) and its key cell (x 232, 100 wide, the row's action as its id,
/// the key-read screen's opener as its callback), on the row's grid line.
pub fn controls_records() -> Vec<Record> {
    let mut out = Vec::with_capacity(BINDINGS.len() * 2);
    for b in &BINDINGS {
        let y = b.y / 4;
        out.push(label(0, 0x50, y, 68, Some(b.label), 0));
        let key = (!b.key.is_empty()).then_some(b.key);
        out.push(Record {
            reports: true,
            ..label(b.action, 0xe8, y, 100, key, KEY_CELL_STYLE)
        });
    }
    out
}

/// A stored 16-bit word read signed, as the layout pass reads anchors.
fn word(v: i32) -> i32 {
    i32::from(v as u16 as i16)
}

/// A record as a control: its words copied, its anchors read signed, its
/// style left 0 when the record has none so the layout gives the class's.
/// An extended class (the separator) takes its registered default height,
/// which the basic layout pass does not know.
pub fn control(r: &Record) -> Control {
    let mut c = Control::new(r.class, word(r.x), word(r.y), r.w, r.h);
    c.group = i32::from(r.group);
    c.text = r.text.unwrap_or("").as_bytes().to_vec();
    c.style = r.style;
    c.state = r.state;
    c.steps = r.steps;
    c.reports = r.reports;
    if c.h == 0
        && let Some(class) = extended_class(r.class)
    {
        c.h = class.size.1;
    }
    c
}

/// Lays `records` out at unit 4 inside a client `client` wide and high.
pub fn lay_out(records: &[Record], client: (i32, i32)) -> Vec<Control> {
    let mut controls: Vec<Control> = records.iter().map(control).collect();
    layout_records(
        &mut controls,
        LayoutClient {
            inner_right: client.0,
            border: BORDER,
            bottom: client.1,
        },
        4,
    );
    controls
}

/// A form's client from its size: the border all round, and the scrollbar
/// off the right of a scrolled one ([`form::client_rect`] without a title).
fn client_of(w: i32, h: i32, scrolled: bool) -> ClientRect {
    let attributes = if scrolled {
        CONTROLS_ATTRIBUTES
    } else {
        CHILD_ATTRIBUTES
    };
    form::client_rect(w, h, BORDER, 0, attributes, 0, SCROLL_INSET, 0, 15, BORDER)
}

fn place_of(rect: Rect, anchors: u32, content: i32, records: &[Control]) -> FormPlace {
    let scrolled = content > 0;
    FormPlace {
        x: rect.x,
        y: rect.y,
        w: rect.w,
        h: rect.h,
        border: BORDER,
        design_border: 0,
        title: 0,
        client: client_of(rect.w, rect.h, scrolled),
        content_height: content,
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

fn take_places(controls: &mut [Control], place: &FormPlace) {
    for (c, r) in controls.iter_mut().zip(&place.records) {
        c.x = i32::from(r.x);
        c.y = i32::from(r.y);
        c.w = i32::from(r.w);
        c.h = i32::from(r.h);
    }
}

/// Re-sizes the trackbars' knobs after a rescale: a knob bar's is
/// `4·unit + 2` (as its set-up gives it; the rescale of a basic trackbar's
/// knob was not traced), and a stepped bar resyncs its knob from its
/// width.
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

/// The laid-out form at a unit: its place on the screen, its two pages
/// (form-local, the Controls page as the constructor places it), the
/// Controls page's content height and the three record lists (each in its
/// own form's client coordinates).
#[derive(Debug, Clone, PartialEq)]
pub struct Geometry {
    pub unit: i32,
    pub form: Rect,
    pub settings_page: Rect,
    pub controls_page: Rect,
    pub controls_content: i32,
    pub main: Vec<Control>,
    pub settings: Vec<Control>,
    pub controls: Vec<Control>,
}

/// Builds the form for a `screen` (width, height) at `unit`: the records
/// laid out at unit 4, the form, its pages and their records rescaled to
/// the unit when it is not 4, then the form centred (truncating across,
/// a floor down).
pub fn geometry(screen: (i32, i32), unit: i32) -> Geometry {
    let form_rect = Rect {
        x: 0,
        y: 0,
        w: DESIGN.0,
        h: DESIGN.1,
    };
    let settings_rect = Rect {
        x: 0,
        y: 64,
        w: DESIGN.0 - 8,
        h: SETTINGS_HEIGHT,
    };
    let controls_rect = Rect {
        x: 0,
        y: 56,
        w: DESIGN.0 - 16,
        h: CONTROLS_HEIGHT,
    };
    let main_client = client_of(form_rect.w, form_rect.h, false);
    let settings_client = client_of(settings_rect.w, settings_rect.h, false);
    let controls_client = client_of(controls_rect.w, controls_rect.h, true);
    let mut main = lay_out(&TABS, (main_client.width(), main_client.height()));
    let mut settings = lay_out(
        &SETTINGS,
        (settings_client.width(), settings_client.height()),
    );
    let mut controls = lay_out(
        &controls_records(),
        (controls_client.width(), controls_client.height()),
    );
    let mut place = place_of(form_rect, FORM_STYLE, 0, &main);
    place.children = vec![
        place_of(settings_rect, 0, 0, &settings),
        place_of(controls_rect, CONTROLS_STYLE, CONTROLS_CONTENT, &controls),
    ];
    if unit != 4 {
        // Only the Controls page carries a content height: it is the one
        // with a vertical scrollbar.
        let mut realign = |f: &mut FormPlace| f.client = client_of(f.w, f.h, f.content_height > 0);
        rescale_form(
            &mut place,
            4,
            unit,
            screen,
            screen,
            6 * unit - BORDER,
            &mut realign,
        );
        take_places(&mut main, &place);
        take_places(&mut settings, &place.children[0]);
        take_places(&mut controls, &place.children[1]);
        resync_trackbars(&mut settings, unit);
    }
    let rect_of = |p: &FormPlace| Rect {
        x: p.x,
        y: p.y,
        w: p.w,
        h: p.h,
    };
    let mut form = rect_of(&place);
    form.x = (screen.0 - form.w) / 2;
    form.y = (screen.1 - form.h) >> 1;
    Geometry {
        unit,
        form,
        settings_page: rect_of(&place.children[0]),
        controls_page: rect_of(&place.children[1]),
        controls_content: place.children[1].content_height,
        main,
        settings,
        controls,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(c: &Control) -> (i32, i32, i32, i32) {
        (c.x, c.y, c.w, c.h)
    }

    #[test]
    fn the_skin_sizes() {
        assert_eq!(skin(1), (80, 24));
        assert_eq!(skin(2), (16, 16));
        assert_eq!(skin(3), (28, 28));
        assert_eq!(skin(4), (0, 16));
        assert_eq!(skin(5), (0, 20));
        assert_eq!(skin(0x13), (0, 3));
        assert_eq!(SEPARATOR_CLASS, 0x13);
    }

    #[test]
    fn the_records_lay_out_at_unit_four() {
        let g = geometry((1280, 720), 4);
        assert_eq!(
            g.form,
            Rect {
                x: 423,
                y: 92,
                w: 434,
                h: 536
            }
        );
        // A 32-high tab moves up by 8; a 24-high button by 4.
        assert_eq!(rect(&g.main[0]), (12, 12, 192, 32));
        assert_eq!(rect(&g.main[1]), (212, 12, 192, 32));
        // The rules are 3 high: half of (3 − 16) through the unsigned shift
        // moves them 7 down.
        assert_eq!(rect(&g.main[4]), (16, 91, 400, 3));
        assert_eq!(rect(&g.main[5]), (16, 471, 400, 3));
        // Defaults six rows below the second rule, Apply and Cancel 16
        // pixels after the button before.
        assert_eq!(rect(&g.main[6]), (40, 484, 100, 24));
        assert_eq!(rect(&g.main[7]), (156, 484, 104, 24));
        assert_eq!(rect(&g.main[8]), (276, 484, 100, 24));
        // The Settings page: readouts, the volume slider and Music.
        assert_eq!(rect(&g.settings[0]), (16, -2, 60, 20));
        assert_eq!(rect(&g.settings[1]), (168, -2, 248, 20));
        assert_eq!(rect(&g.settings[5]), (16, 74, 60, 20));
        assert_eq!(rect(&g.settings[6]), (168, 76, 140, 16));
        assert_eq!(rect(&g.settings[7]), (320, 76, 56, 16));
        assert_eq!(rect(&g.settings[26]), (148, 384, 100, 16));
        // The volume slider is a knob bar: knob 18, margin 2 widened by its
        // frame bit 0x20 to 3; a stepped one sizes its knob from its width.
        assert_eq!((g.settings[6].knob, g.settings[6].margin), (18, 3));
        assert_eq!(g.settings[9].knob, 34);
        assert_eq!(g.settings[13].knob, 68);
        // The Controls page: the first and last rows.
        assert_eq!(rect(&g.controls[0]), (80, 6, 68, 20));
        assert_eq!(rect(&g.controls[1]), (232, 6, 100, 20));
        assert_eq!(rect(&g.controls[43]), (232, 614, 100, 20));
        assert_eq!(
            g.settings_page,
            Rect {
                x: 0,
                y: 64,
                w: 426,
                h: 408
            }
        );
        assert_eq!(
            g.controls_page,
            Rect {
                x: 0,
                y: 56,
                w: 418,
                h: 416
            }
        );
        assert_eq!(g.controls_content, 676);
    }

    #[test]
    fn the_form_rescales_through_its_client() {
        let g = geometry((1920, 1080), 6);
        // The client (430 × 532) scales by whole units: 107 and 133 of them.
        assert_eq!(
            g.form,
            Rect {
                x: 637,
                y: 139,
                w: 646,
                h: 802
            }
        );
        assert_eq!(rect(&g.main[0]), (18, 18, 288, 48));
        assert_eq!(
            g.settings_page,
            Rect {
                x: 0,
                y: 96,
                w: 634,
                h: 610
            }
        );
        assert_eq!(g.controls_page.w, 615);
        assert_eq!(g.controls_content, 1014);
        assert_eq!(g.settings[6].knob, 26);
    }
}

/// A switched page: what is visible and where the Controls page goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PageSwitch {
    pub page: u8,
    pub settings_visible: bool,
    pub controls_visible: bool,
    /// The Controls page's y and height (unchanged on page 0).
    pub controls_y: Option<(i32, i32)>,
}

/// The page switch: page 0 shows the Settings page; page 1 the Controls
/// page at a y of 14 units, or 21 when the controller tabs show (with Steam
/// Input, which Exarust does not have), its height 118 units less its y.
pub fn switch_page(page: u8, controller_tabs: bool, unit: i32) -> Option<PageSwitch> {
    match page {
        0 => Some(PageSwitch {
            page,
            settings_visible: true,
            controls_visible: false,
            controls_y: None,
        }),
        1 => {
            let y = unit * if controller_tabs { 21 } else { 14 };
            Some(PageSwitch {
                page,
                settings_visible: false,
                controls_visible: true,
                controls_y: Some((y, unit * 118 - y)),
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod page_tests {
    use super::*;

    #[test]
    fn the_page_switch() {
        let settings = switch_page(0, false, 4).unwrap();
        assert!(settings.settings_visible && !settings.controls_visible);
        assert_eq!(settings.controls_y, None);
        let controls = switch_page(1, false, 4).unwrap();
        assert!(!controls.settings_visible && controls.controls_visible);
        assert_eq!(controls.controls_y, Some((56, 416)));
        assert_eq!(switch_page(1, true, 4).unwrap().controls_y, Some((84, 388)));
        assert_eq!(
            switch_page(1, false, 6).unwrap().controls_y,
            Some((84, 624))
        );
        assert!(switch_page(2, false, 4).is_none());
    }
}

/// An editable row of the Settings page.
///
/// The ids are the tab dialog's control records: the two readouts are 1 and
/// 2, then the sliders and checkboxes in the order the page shows them (id
/// 10 is unused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Volume,
    Music,
    Supersample,
    ShadingQuality,
    TextureDetail,
    TextureQuality,
    ShadowDetail,
    ShadowQuality,
    Multisample,
    VerticalSync,
    ShowFps,
    WindowMode,
    ShowTips,
    Checkpoints,
}

impl Setting {
    /// The row's control id.
    pub fn id(self) -> u16 {
        match self {
            Setting::Volume => 3,
            Setting::Music => 4,
            Setting::Supersample => 5,
            Setting::ShadingQuality => 6,
            Setting::TextureDetail => 7,
            Setting::TextureQuality => 8,
            Setting::ShadowDetail => 9,
            Setting::ShadowQuality => 11,
            Setting::Multisample => 12,
            Setting::VerticalSync => 13,
            Setting::ShowFps => 14,
            Setting::WindowMode => 15,
            Setting::ShowTips => 16,
            Setting::Checkpoints => 17,
        }
    }

    /// The row with control id `id`, if it is editable.
    pub fn from_id(id: u16) -> Option<Self> {
        Some(match id {
            3 => Setting::Volume,
            4 => Setting::Music,
            5 => Setting::Supersample,
            6 => Setting::ShadingQuality,
            7 => Setting::TextureDetail,
            8 => Setting::TextureQuality,
            9 => Setting::ShadowDetail,
            11 => Setting::ShadowQuality,
            12 => Setting::Multisample,
            13 => Setting::VerticalSync,
            14 => Setting::ShowFps,
            15 => Setting::WindowMode,
            16 => Setting::ShowTips,
            17 => Setting::Checkpoints,
            _ => return None,
        })
    }

    /// Whether the row is a checkbox rather than a slider.
    pub fn is_checkbox(self) -> bool {
        matches!(
            self,
            Setting::Music
                | Setting::VerticalSync
                | Setting::ShowFps
                | Setting::WindowMode
                | Setting::ShowTips
                | Setting::Checkpoints
        )
    }
}

/// What a row shows when the Settings page opens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowState {
    /// The slider's position, 0 to 1. Checkboxes and a slider the original
    /// leaves at its record's value have none.
    pub fraction: Option<f32>,
    /// Whether a checkbox is checked.
    pub checked: bool,
    /// Whether the row takes input. Hardware anti-aliasing is disabled while
    /// super-sampling is off.
    pub enabled: bool,
    /// The slider's label, with the original's leading space.
    pub text: Option<&'static str>,
}

impl RowState {
    const fn slider(fraction: f32, text: Option<&'static str>) -> Self {
        Self {
            fraction: Some(fraction),
            checked: false,
            enabled: true,
            text,
        }
    }

    const fn check(checked: bool) -> Self {
        Self {
            fraction: None,
            checked,
            enabled: true,
            text: None,
        }
    }
}

/// The rows as the page opens, in row order.
///
/// The volume, music, super-sampling and window-mode rows show the live
/// options; the others show the page's draft. Hardware anti-aliasing is
/// enabled only while the live super-sampling is off, showing the draft's
/// samples; while it is on the row is disabled and keeps its position.
pub fn populate(live: &Options, options: &Options) -> [(Setting, RowState); 14] {
    let row = |setting: Setting| match setting {
        Setting::Volume => RowState::slider(live.volume_fraction(), None),
        Setting::Music => RowState::check(live.music),
        Setting::Supersample => {
            RowState::slider(live.supersample_fraction(), live.supersample_text())
        }
        Setting::ShadingQuality => {
            RowState::slider(options.shading_fraction(), options.shading_text())
        }
        Setting::TextureDetail => RowState::slider(
            options.texture_detail_fraction(),
            options.texture_detail_text(),
        ),
        Setting::TextureQuality => RowState::slider(
            options.texture_quality_fraction(),
            options.texture_quality_text(),
        ),
        Setting::ShadowDetail => RowState::slider(
            options.shadow_detail_fraction(),
            options.shadow_detail_text(),
        ),
        Setting::ShadowQuality => RowState::slider(
            options.shadow_quality_fraction(),
            options.shadow_quality_text(),
        ),
        Setting::Multisample if live.supersample > 0 => RowState {
            fraction: None,
            checked: false,
            enabled: false,
            text: None,
        },
        Setting::Multisample => RowState {
            fraction: Some(options.multisample_fraction()),
            checked: false,
            enabled: true,
            text: options.multisample_text(),
        },
        Setting::VerticalSync => RowState::check(options.vertical_sync),
        Setting::ShowFps => RowState::check(options.show_fps),
        Setting::WindowMode => RowState::check(live.window_mode),
        Setting::ShowTips => RowState::check(options.show_tips),
        Setting::Checkpoints => RowState::check(options.checkpoints),
    };
    [
        Setting::Volume,
        Setting::Music,
        Setting::Supersample,
        Setting::ShadingQuality,
        Setting::TextureDetail,
        Setting::TextureQuality,
        Setting::ShadowDetail,
        Setting::ShadowQuality,
        Setting::Multisample,
        Setting::VerticalSync,
        Setting::ShowFps,
        Setting::WindowMode,
        Setting::ShowTips,
        Setting::Checkpoints,
    ]
    .map(|setting| (setting, row(setting)))
}

/// Where a control's change goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// The running game changes at once as well as the draft (volume,
    /// texture detail and quality, vertical sync, the frame rate, the tips,
    /// checkpoints).
    Live,
    /// Only the screen's draft changes; Apply pushes it.
    Draft,
    /// The change is remembered but not written to the settings file (the
    /// Music checkbox mutes the session; the original writes the startup
    /// value).
    SessionOnly,
    /// The row is disabled and ignores input.
    Ignored,
}

/// A slider step: writes its value into `options` and says where it goes.
///
/// The steps are the original's: volume × 1.5; super-sampling `round(3·f)`,
/// one more above 1; shading `round(3·f) + 3`; texture detail `round(f) + 2`;
/// texture quality `round(3·f) + 2`; shadow detail `round(f) + 4`; shadow
/// quality `round(3·f) + 1`; hardware anti-aliasing `1 << round(3·f)`.
/// Rounding is to even, as the processor's conversion does.
///
/// The super-sampling slider also disables the hardware anti-aliasing row
/// when its level is above 0 (and enables it at 0), setting that row to 0
/// through a reported set ([`FormState`]); the row's own step then writes
/// the draft's samples, disabled or not. The volume, texture-detail and texture-quality sliders change
/// the live options too; the other quality sliders change the draft only.
pub fn on_slider(options: &mut Options, setting: Setting, fraction: f32) -> Effect {
    match setting {
        Setting::Volume => {
            options.volume = hw_config::volume_from_fraction(fraction);
            Effect::Live
        }
        Setting::Supersample => {
            options.supersample = hw_config::supersample_from_fraction(fraction);
            Effect::Draft
        }
        Setting::ShadingQuality => {
            options.shading_quality = hw_config::shading_from_fraction(fraction);
            Effect::Draft
        }
        Setting::TextureDetail => {
            options.texture_detail = hw_config::texture_detail_from_fraction(fraction);
            Effect::Live
        }
        Setting::TextureQuality => {
            options.texture_quality = hw_config::texture_quality_from_fraction(fraction);
            Effect::Live
        }
        Setting::ShadowDetail => {
            options.shadow_detail = hw_config::shadow_detail_from_fraction(fraction);
            Effect::Draft
        }
        Setting::ShadowQuality => {
            options.shadow_quality = hw_config::shadow_quality_from_fraction(fraction);
            Effect::Draft
        }
        // The callback writes whatever the slider holds: a disabled row
        // takes no pointer input, but the super-sampling slider's reported
        // set of 0 still reaches it.
        Setting::Multisample => {
            options.multisample = hw_config::multisample_from_fraction(fraction);
            Effect::Draft
        }
        Setting::Music
        | Setting::VerticalSync
        | Setting::ShowFps
        | Setting::WindowMode
        | Setting::ShowTips
        | Setting::Checkpoints => Effect::Ignored,
    }
}

/// A checkbox click: writes it into `options` and says where it goes.
///
/// The Music checkbox only mutes the running session; the original's setting
/// writes the startup music value and the page reopens from it.
pub fn on_toggle(options: &mut Options, setting: Setting, checked: bool) -> Effect {
    match setting {
        Setting::Music => Effect::SessionOnly,
        Setting::VerticalSync => {
            options.vertical_sync = checked;
            Effect::Live
        }
        Setting::ShowFps => {
            options.show_fps = checked;
            Effect::Live
        }
        Setting::WindowMode => {
            options.window_mode = checked;
            Effect::Draft
        }
        Setting::ShowTips => {
            options.show_tips = checked;
            Effect::Live
        }
        Setting::Checkpoints => {
            options.checkpoints = checked;
            Effect::Live
        }
        _ => Effect::Ignored,
    }
}

/// The aim-speed row's number (`round(f × 99) + 1`) and the fraction it
/// writes: the slider changes the live aim speed.
pub fn set_aim_speed(options: &mut Options, fraction: f32) -> i32 {
    options.aim_speed = fraction;
    options.aim_speed_text()
}

/// The text of a scale the controller-backed dropdown would show, for the
/// pages that reuse the renderer's labels.
pub fn scale_text(scale: Scale, value: i32) -> Option<&'static str> {
    scale.text(value)
}

/// A Settings or Controls form action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormAction {
    /// The SETTINGS tab (page 0).
    SettingsTab,
    /// The CONTROLS tab (page 1).
    ControlsTab,
    /// The Keyboard tab (hidden without Steam Input): the keyboard view and
    /// the controller mode off.
    KeyboardView,
    /// The Controller tab (hidden without Steam Input): the controller view
    /// and the controller mode on.
    ControllerView,
    /// The Defaults button. On the Settings page: run the benchmark (or take
    /// the Deck preset), push its quality and refill the rows. On the
    /// Controls page: the shipped keys back into the page's copy, or, in
    /// controller mode, the saved aim speed back into the live one.
    Defaults,
    /// Apply: store the settings and close.
    Apply,
    /// Cancel: push the saved options back and close.
    Cancel,
}

/// The action of the tab/button at `index` in [`TABS`]. The two rule records
/// are not form actions.
pub fn form_action(index: usize) -> Option<FormAction> {
    Some(match index {
        0 => FormAction::SettingsTab,
        1 => FormAction::ControlsTab,
        2 => FormAction::KeyboardView,
        3 => FormAction::ControllerView,
        6 => FormAction::Defaults,
        7 => FormAction::Apply,
        8 => FormAction::Cancel,
        _ => return None,
    })
}

/// The page an action shows, for the two page tabs.
pub fn action_page(action: FormAction) -> Option<u8> {
    match action {
        FormAction::SettingsTab => Some(0),
        FormAction::ControlsTab => Some(1),
        _ => None,
    }
}

/// The controller mode a controller-view tab selects; `None` for the other
/// actions.
pub fn controller_view_tab(action: FormAction) -> Option<bool> {
    match action {
        FormAction::KeyboardView => Some(false),
        FormAction::ControllerView => Some(true),
        _ => None,
    }
}

/// The label a slider shows for the draft's value, as each slider's
/// callback writes it after a step (the super-sampling level, the quality
/// levels and the sample count); the volume slider has none.
pub fn draft_text(setting: Setting, o: &Options) -> Option<&'static str> {
    match setting {
        Setting::Supersample => o.supersample_text(),
        Setting::ShadingQuality => o.shading_text(),
        Setting::TextureDetail => o.texture_detail_text(),
        Setting::TextureQuality => o.texture_quality_text(),
        Setting::ShadowDetail => o.shadow_detail_text(),
        Setting::ShadowQuality => o.shadow_quality_text(),
        Setting::Multisample => o.multisample_text(),
        _ => None,
    }
}

/// The index in [`SETTINGS`] of the record with control id `id`.
pub fn settings_index(id: u16) -> Option<usize> {
    SETTINGS.iter().position(|r| r.id == id && r.class != 5)
}

/// Which of the form's record lists a control is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    /// The form's own records: the tabs, rules and buttons.
    Main,
    Settings,
    Controls,
}

/// What a control's change asks of the game.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FormEvent {
    /// A tab turned on: the form shows the page (already switched).
    Page(u8),
    /// Defaults, Apply or Cancel.
    Action(FormAction),
    /// A slider moved to this fraction.
    Slider(Setting, f32),
    /// A checkbox changed (its lit bit, which the callbacks read).
    Toggle(Setting, bool),
    /// A Controls key cell was clicked: the key-read screen opens for the
    /// row ([`BINDINGS`] index).
    Rebind(usize),
}

/// The open Settings form: its geometry, the page in view, the Controls
/// page's place and scroll, and the record the pointer holds and hovers.
#[derive(Debug, Clone, PartialEq)]
pub struct FormState {
    pub screen: (i32, i32),
    pub geometry: Geometry,
    pub page: u8,
    /// The Controls page as the page switch placed it (form-local).
    pub controls_page: Rect,
    /// The Controls page's vertical scroll fraction.
    pub scroll: f32,
    /// Whether the Keyboard and Controller tabs show (they need Steam Input).
    pub controller_tabs: bool,
    /// Whether the Controls page shows its controller view (the Controller
    /// tab lit) rather than the keyboard one.
    pub controller_view: bool,
    held: Option<(Part, usize)>,
    hovered: Option<(Part, usize)>,
}

/// Whether a record takes the pointer: shown, enabled, without the
/// not-hit style bit 2, and not one that lets the pointer through (a rule,
/// or a label without a callback).
fn takes_hits(c: &Control) -> bool {
    c.state & HIDDEN != HIDDEN
        && c.state & state::DISABLED == 0
        && c.style & 2 == 0
        && c.class != SEPARATOR_CLASS
        && !(c.class == 5 && !c.reports)
}

impl FormState {
    /// Opens the form on its Settings page.
    pub fn open(screen: (i32, i32), unit: i32) -> Self {
        let geometry = geometry(screen, unit);
        let mut form = Self {
            screen,
            controls_page: geometry.controls_page,
            geometry,
            page: 0,
            scroll: 0.0,
            controller_tabs: false,
            controller_view: false,
            held: None,
            hovered: None,
        };
        form.switch(0);
        form
    }

    pub fn unit(&self) -> i32 {
        self.geometry.unit
    }

    /// Rebuilds the form for another screen size or unit, keeping its page,
    /// scroll and every record's state, value and text.
    pub fn resize(&mut self, screen: (i32, i32), unit: i32) {
        let mut g = geometry(screen, unit);
        let keep = |new: &mut [Control], old: &[Control]| {
            for (n, o) in new.iter_mut().zip(old) {
                n.state = o.state;
                n.value = o.value;
                n.text = o.text.clone();
            }
        };
        keep(&mut g.main, &self.geometry.main);
        keep(&mut g.settings, &self.geometry.settings);
        keep(&mut g.controls, &self.geometry.controls);
        self.controls_page = g.controls_page;
        self.geometry = g;
        self.screen = screen;
        self.held = None;
        self.hovered = None;
        self.switch(self.page);
    }

    /// The page switch: page 0 shows the rules and hides the Keyboard and
    /// Controller tabs; page 1 hides the rules and places the Controls page
    /// ([`switch_page`]). With Steam Input the view tabs show on page 1 and
    /// light the view in use (the keyboard one by default); without it they
    /// stay hidden, so the controller view is never shown.
    pub fn switch(&mut self, page: u8) {
        let Some(switch) = switch_page(page, self.controller_tabs, self.unit()) else {
            return;
        };
        self.page = page;
        let main = &mut self.geometry.main;
        main[0].state &= !HIDDEN;
        main[1].state &= !HIDDEN;
        if page == 0 {
            main[4].state = 0;
            main[5].state = 0;
            main[2].state = HIDDEN;
            main[3].state = HIDDEN;
        } else {
            main[4].state = HIDDEN;
            main[5].state = HIDDEN;
            if self.controller_tabs {
                main[2].state &= !HIDDEN;
                main[3].state &= !HIDDEN;
                self.select_controller_view(self.controller_view);
            } else {
                main[2].state = HIDDEN;
                main[3].state = HIDDEN;
            }
        }
        if let Some((y, h)) = switch.controls_y {
            self.controls_page.y = y;
            self.controls_page.h = h;
        }
    }

    /// The Keyboard (`false`) or Controller (`true`) view tab: lights the one
    /// and darkens the other, and re-places the Controls page (the controller
    /// tabs push it 21 units down instead of 14).
    pub fn select_controller_view(&mut self, controller: bool) {
        self.controller_view = controller;
        let main = &mut self.geometry.main;
        for (record, on) in [(2, !controller), (3, controller)] {
            main[record].state &= !(state::ON | state::LIT);
            if on {
                main[record].state |= state::ON | state::LIT;
            }
        }
        let switch = switch_page(1, self.controller_tabs, self.unit());
        if self.page == 1
            && let Some((y, h)) = switch.and_then(|switch| switch.controls_y)
        {
            self.controls_page.y = y;
            self.controls_page.h = h;
        }
    }

    /// The form's record origin on the screen (its client's top left).
    pub fn origin(&self) -> (i32, i32) {
        (self.geometry.form.x + BORDER, self.geometry.form.y + BORDER)
    }

    /// The page in view and its record list.
    pub fn page_part(&self) -> Part {
        if self.page == 0 {
            Part::Settings
        } else {
            Part::Controls
        }
    }

    /// The page in view on the screen.
    pub fn page_screen(&self) -> Rect {
        let (ox, oy) = self.origin();
        let page = if self.page == 0 {
            self.geometry.settings_page
        } else {
            self.controls_page
        };
        Rect {
            x: ox + page.x,
            y: oy + page.y,
            ..page
        }
    }

    /// The page's client on the screen.
    pub fn page_client(&self) -> Rect {
        let page = self.page_screen();
        let client = client_of(page.w, page.h, self.page == 1);
        Rect {
            x: page.x + client.left,
            y: page.y + client.top,
            w: client.width(),
            h: client.height(),
        }
    }

    /// The Controls page's scroll offset: the overflow times the fraction,
    /// rounded as `CVTSS2SI` does.
    pub fn scroll_offset(&self) -> i32 {
        if self.page != 1 {
            return 0;
        }
        let overflow = self.geometry.controls_content - self.page_client().h;
        if overflow <= 0 {
            0
        } else {
            form::cvtss2si(overflow as f32 * self.scroll)
        }
    }

    /// The help box's place inside the Controls page (page-local), from that
    /// page's client width: [`help_box`].
    pub fn help_box(&self) -> Rect {
        let page = self.controls_page;
        let client = client_of(page.w, page.h, true);
        help_box(client.width(), self.unit())
    }

    /// The Controls page's scrollbar track and thumb (page-local).
    pub fn scrollbar(&self) -> form::VerticalScroll {
        let page = self.controls_page;
        form::vertical_scroll_metrics(
            page.h,
            BORDER,
            client_of(page.w, page.h, true),
            self.geometry.controls_content,
            4 * self.unit() - 1,
            self.unit(),
            CONTROLS_ATTRIBUTES,
            self.scroll,
            SCROLL_STEP,
        )
    }

    /// Where a part's records are measured from on the screen.
    pub fn part_origin(&self, part: Part) -> (i32, i32) {
        match part {
            Part::Main => self.origin(),
            _ => {
                let client = self.page_client();
                (client.x, client.y - self.scroll_offset())
            }
        }
    }

    pub fn controls(&self, part: Part) -> &[Control] {
        match part {
            Part::Main => &self.geometry.main,
            Part::Settings => &self.geometry.settings,
            Part::Controls => &self.geometry.controls,
        }
    }

    fn controls_mut(&mut self, part: Part) -> &mut Vec<Control> {
        match part {
            Part::Main => &mut self.geometry.main,
            Part::Settings => &mut self.geometry.settings,
            Part::Controls => &mut self.geometry.controls,
        }
    }

    /// Whether a screen point is on the form.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.geometry.form.contains(x, y)
    }

    /// The record under a screen point. The page in view is tried first
    /// (a visible child before the records); off its client it answers
    /// none. Records are hit on their rectangles, edges included, then
    /// within two units.
    pub fn hit(&self, x: i32, y: i32) -> Option<(Part, usize)> {
        let part = if self.page_screen().contains(x, y) {
            if !self.page_client().contains(x, y) {
                return None;
            }
            self.page_part()
        } else {
            Part::Main
        };
        let (ox, oy) = self.part_origin(part);
        let (x, y) = (x - ox, y - oy);
        let unit = self.unit();
        let controls = self.controls(part);
        for slop in [false, true] {
            for (i, c) in controls.iter().enumerate() {
                if takes_hits(c)
                    && form::control_hit(x, y, (c.x, c.y, c.x + c.w, c.y + c.h), slop, unit)
                {
                    return Some((part, i));
                }
            }
        }
        None
    }

    fn send(&mut self, part: Part, i: usize, m: Message) -> Vec<FormEvent> {
        let unit = self.unit();
        let mut events = Vec::new();
        gui_controls::dispatch(self.controls_mut(part), i, m, unit, &mut events);
        self.answer(part, events)
    }

    /// Turns the controls' change reports into the callbacks' events.
    fn answer(&mut self, part: Part, events: Vec<ControlEvent>) -> Vec<FormEvent> {
        let mut out = Vec::new();
        for event in events {
            let ControlEvent::Changed(i) = event else {
                continue;
            };
            match part {
                Part::Main => {
                    let Some(action) = form_action(i) else {
                        continue;
                    };
                    match action_page(action) {
                        // The page tabs' callbacks act only on turning on.
                        Some(page) => {
                            if self.geometry.main[i].state & state::ON != 0 {
                                self.switch(page);
                                out.push(FormEvent::Page(page));
                            }
                        }
                        // The controller-view tabs do too.
                        None if matches!(
                            action,
                            FormAction::KeyboardView | FormAction::ControllerView
                        ) =>
                        {
                            if self.geometry.main[i].state & state::ON != 0 {
                                out.push(FormEvent::Action(action));
                            }
                        }
                        None => out.push(FormEvent::Action(action)),
                    }
                }
                Part::Settings => {
                    let Some(setting) = Setting::from_id(SETTINGS[i].id) else {
                        continue;
                    };
                    let c = &self.geometry.settings[i];
                    if c.class == 4 {
                        let value = c.value;
                        out.push(FormEvent::Slider(setting, value));
                        if setting == Setting::Supersample {
                            out.extend(self.supersample_changed(value));
                        }
                    } else {
                        out.push(FormEvent::Toggle(setting, c.state & state::LIT != 0));
                    }
                }
                Part::Controls => {
                    if i % 2 == 1 {
                        out.push(FormEvent::Rebind(i / 2));
                    }
                }
            }
        }
        out
    }

    /// The super-sampling slider's callback: its level is the step count
    /// times the value rounded, one more above 1; a level of 1 or more sets
    /// the anti-aliasing slider to 0 (a reported set, whose callback writes
    /// the draft) and disables it, a level of 0 enables it.
    fn supersample_changed(&mut self, value: f32) -> Vec<FormEvent> {
        let Some(i) = settings_index(Setting::Multisample.id()) else {
            return Vec::new();
        };
        let mut level = form::cvtss2si(3.0 * value);
        if level > 1 {
            level += 1;
        }
        if level < 1 {
            self.geometry.settings[i].state &= !state::DISABLED;
            return Vec::new();
        }
        let out = self.send(
            Part::Settings,
            i,
            Message {
                message: message::SET,
                ..Message::default()
            },
        );
        self.geometry.settings[i].state |= state::DISABLED;
        out
    }

    /// A left press: the record under the pointer is held and pressed (a
    /// trackbar takes the pointer's x, a label its click word).
    pub fn press(&mut self, x: i32, y: i32) -> Vec<FormEvent> {
        let Some((part, i)) = self.hit(x, y) else {
            return Vec::new();
        };
        self.held = Some((part, i));
        let (ox, _) = self.part_origin(part);
        let param = if self.controls(part)[i].class == 5 {
            gui_controls::LABEL_CLICK
        } else {
            (x - ox) as u32
        };
        self.send(
            part,
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

    /// The pointer moved: a held trackbar follows it; a held record lights
    /// while the pointer is over it; otherwise the record under the pointer
    /// lights and the one it left darkens. Trackbars take no hover (their
    /// move message is the drag).
    pub fn pointer(&mut self, x: i32, y: i32) -> Vec<FormEvent> {
        let hit = self.hit(x, y);
        if let Some((part, i)) = self.held {
            let (ox, _) = self.part_origin(part);
            let class = self.controls(part)[i].class;
            let param = if class == 4 { (x - ox) as u32 } else { 0 };
            return self.send(
                part,
                i,
                Message {
                    message: message::HOVER,
                    param,
                    over: hit == Some((part, i)),
                    ..Message::default()
                },
            );
        }
        if hit != self.hovered {
            // The form's hover moves the hover bits (0x10000 for a record
            // that answers no bits of its own) from the old record to the
            // new one; the skin's thumb, box and caption read them.
            if let Some((part, i)) = self.hovered
                && let Some(c) = self.controls_mut(part).get_mut(i)
            {
                c.state &= !HOVER_BITS;
            }
            if let Some((part, i)) = hit
                && let Some(c) = self.controls_mut(part).get_mut(i)
            {
                c.state = (c.state & !HOVER_BITS) | HOVERED;
            }
            for (target, over) in [(self.hovered, false), (hit, true)] {
                if let Some((part, i)) = target
                    && self.controls(part).get(i).is_some_and(|c| c.class != 4)
                {
                    self.send(
                        part,
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
    pub fn release(&mut self, x: i32, y: i32) -> Vec<FormEvent> {
        let Some((part, i)) = self.held.take() else {
            return Vec::new();
        };
        let over = self.hit(x, y) == Some((part, i));
        self.send(
            part,
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

    /// The wheel over the Controls page moves its scroll by whole notches
    /// of [`SCROLL_STEP`], clamped to 0..1. Returns whether it moved.
    pub fn wheel(&mut self, x: i32, y: i32, delta: i32) -> bool {
        if self.page != 1 || !self.page_screen().contains(x, y) {
            return false;
        }
        let WheelScroll::Vertical(f) = form::wheel_scroll(
            CONTROLS_ATTRIBUTES,
            delta,
            (0.0, self.scroll),
            (0.0, SCROLL_STEP),
        ) else {
            return false;
        };
        let f = f.clamp(0.0, 1.0);
        let moved = f != self.scroll;
        self.scroll = f;
        moved
    }

    /// Fills the Settings rows as the page opens (and Defaults refills
    /// it): each slider's value and label, each set checkbox lit and on,
    /// the anti-aliasing row enabled or, while super-sampling is on, set to
    /// 0 through a reported set and disabled. Returns that set's events.
    pub fn fill(&mut self, rows: &[(Setting, RowState)]) -> Vec<FormEvent> {
        let mut out = Vec::new();
        for (setting, row) in rows {
            let Some(i) = settings_index(setting.id()) else {
                continue;
            };
            if setting.is_checkbox() {
                if row.checked {
                    self.geometry.settings[i].state |= state::ON | state::LIT;
                }
                continue;
            }
            let c = &mut self.geometry.settings[i];
            if let Some(f) = row.fraction {
                c.value = f;
            }
            if let Some(text) = row.text {
                c.text = text.as_bytes().to_vec();
            }
            if row.enabled {
                c.state &= !state::DISABLED;
            } else {
                out.extend(self.send(
                    Part::Settings,
                    i,
                    Message {
                        message: message::SET,
                        ..Message::default()
                    },
                ));
                self.geometry.settings[i].state |= state::DISABLED;
            }
        }
        out
    }

    /// A slider's label after its callback, from the draft.
    pub fn set_text(&mut self, setting: Setting, draft: &Options) {
        if let (Some(i), Some(text)) = (settings_index(setting.id()), draft_text(setting, draft)) {
            self.geometry.settings[i].text = text.as_bytes().to_vec();
        }
    }

    /// The Controls key cells' names, in [`BINDINGS`] order.
    pub fn set_keys(&mut self, names: &[String]) {
        for (row, name) in names.iter().enumerate() {
            if let Some(c) = self.geometry.controls.get_mut(row * 2 + 1) {
                c.text = name.as_bytes().to_vec();
            }
        }
    }

    /// The video device and memory readouts: the renderer's name (the Deck's
    /// fixed one on a Deck) and the driver's video memory, "Unknown" when it
    /// reports none, as the original's form creation fills them.
    pub fn set_device(&mut self, renderer: Option<&str>, steam_deck: bool, megabytes: u32) {
        let device = hw_config::device_text(renderer, steam_deck);
        self.geometry.settings[1].text = device.into_bytes();
        let memory = hw_config::memory_text(megabytes);
        self.geometry.settings[3].text = memory.into_bytes();
    }

    /// Whether a record is held (a press not yet released).
    pub fn holding(&self) -> bool {
        self.held.is_some()
    }
}

#[cfg(test)]
mod form_tests {
    use super::*;

    const SCREEN: (i32, i32) = (1280, 720);

    fn click(form: &mut FormState, x: i32, y: i32) -> Vec<FormEvent> {
        let mut events = form.press(x, y);
        events.extend(form.release(x, y));
        events
    }

    #[test]
    fn the_form_opens_on_the_settings_page() {
        let form = FormState::open(SCREEN, 4);
        assert_eq!(form.page, 0);
        assert_eq!(form.origin(), (425, 94));
        assert_eq!(
            form.page_screen(),
            Rect {
                x: 425,
                y: 158,
                w: 426,
                h: 408
            }
        );
        assert_eq!(
            form.page_client(),
            Rect {
                x: 427,
                y: 160,
                w: 422,
                h: 404
            }
        );
        assert_eq!(form.geometry.main[2].state, HIDDEN);
        assert_eq!(form.geometry.main[3].state, HIDDEN);
        assert_eq!(form.geometry.main[4].state, 0);
        // The hidden Keyboard tab takes no hit.
        assert_eq!(form.hit(445, 154), None);
        // SETTINGS is on; CONTROLS is a hit.
        assert_eq!(form.hit(647, 116), Some((Part::Main, 1)));
    }

    #[test]
    fn the_controls_tab_switches_the_page() {
        let mut form = FormState::open(SCREEN, 4);
        assert_eq!(form.press(647, 116), vec![]);
        assert_eq!(form.release(647, 116), vec![FormEvent::Page(1)]);
        assert_eq!(form.page, 1);
        assert_eq!(form.geometry.main[4].state, HIDDEN);
        assert_eq!(form.geometry.main[0].state & state::ON, 0);
        assert_eq!(
            form.controls_page,
            Rect {
                x: 0,
                y: 56,
                w: 418,
                h: 416
            }
        );
        assert_eq!(
            form.page_client(),
            Rect {
                x: 427,
                y: 152,
                w: 397,
                h: 412
            }
        );
        // Clicking the tab that is on does nothing.
        assert_eq!(click(&mut form, 647, 116), vec![]);
        // The first key cell opens the key-read screen for its row.
        assert_eq!(form.press(664, 163), vec![FormEvent::Rebind(0)]);
        form.release(664, 163);
        // A row label lets the pointer through.
        assert_eq!(form.hit(512, 163), None);
        // SETTINGS goes back.
        assert_eq!(click(&mut form, 437, 116), vec![FormEvent::Page(0)]);
    }

    #[test]
    fn the_wheel_scrolls_the_controls_page() {
        let mut form = FormState::open(SCREEN, 4);
        form.switch(1);
        assert!(form.wheel(600, 300, -120));
        assert_eq!(form.scroll, 0.25);
        // The overflow is 676 − 412 = 264.
        assert_eq!(form.scroll_offset(), 66);
        assert!(form.wheel(600, 300, 240));
        assert_eq!(form.scroll, 0.0);
        // The thumb: round(382 · 412 / 676) = 233 on a track from 17.
        let bar = form.scrollbar();
        assert_eq!(
            (bar.start, bar.end, bar.thumb, bar.thumb_y),
            (17, 399, 233, 17)
        );
        // Off the page the wheel does nothing.
        assert!(!form.wheel(10, 10, -120));
    }

    #[test]
    fn a_slider_follows_the_pointer_and_reports() {
        let mut form = FormState::open(SCREEN, 4);
        // The volume bar's travel is 140 − 2·3 − 18 = 116; half of it is
        // 58 past the margin and half the knob.
        let x = 427 + 168 + 3 + 9 + 58;
        assert_eq!(
            form.press(x, 244),
            vec![FormEvent::Slider(Setting::Volume, 0.5)]
        );
        assert_eq!(
            form.pointer(x + 116, 400),
            vec![FormEvent::Slider(Setting::Volume, 1.0)]
        );
        assert_eq!(
            form.release(x + 116, 400),
            vec![FormEvent::Slider(Setting::Volume, 1.0)]
        );
        // Super-sampling to its top step sets the anti-aliasing slider to 0
        // and disables it.
        let i = settings_index(Setting::Multisample.id()).unwrap();
        form.geometry.settings[i].value = 1.0;
        let ss = &form.geometry.settings[9];
        let y = 160 + ss.y + 8;
        let events = form.press(427 + 168 + 140, y);
        assert_eq!(
            events,
            vec![
                FormEvent::Slider(Setting::Supersample, 1.0),
                FormEvent::Slider(Setting::Multisample, 0.0)
            ]
        );
        assert_ne!(form.geometry.settings[i].state & state::DISABLED, 0);
        assert_eq!(form.geometry.settings[i].value, 0.0);
    }

    #[test]
    fn checkboxes_and_buttons() {
        let mut form = FormState::open(SCREEN, 4);
        assert_eq!(
            click(&mut form, 427 + 324, 160 + 80),
            vec![FormEvent::Toggle(Setting::Music, true)]
        );
        assert_eq!(
            click(&mut form, 427 + 324, 160 + 80),
            vec![FormEvent::Toggle(Setting::Music, false)]
        );
        assert_eq!(
            click(&mut form, 586, 583),
            vec![FormEvent::Action(FormAction::Apply)]
        );
        assert_eq!(
            click(&mut form, 475, 583),
            vec![FormEvent::Action(FormAction::Defaults)]
        );
        assert_eq!(
            click(&mut form, 706, 583),
            vec![FormEvent::Action(FormAction::Cancel)]
        );
        // A release away from the button does not land.
        form.press(586, 583);
        assert_eq!(form.release(10, 10), vec![]);
    }

    #[test]
    fn the_rows_fill_and_the_labels_follow_the_draft() {
        let mut form = FormState::open(SCREEN, 4);
        let live = Options {
            supersample: 3,
            ..Options::default()
        };
        let rows = populate(&live, &live);
        let events = form.fill(&rows);
        assert_eq!(events, vec![FormEvent::Slider(Setting::Multisample, 0.0)]);
        assert_eq!(form.geometry.settings[9].value, 2.0 / 3.0);
        assert_eq!(form.geometry.settings[9].text, b" Medium".to_vec());
        assert_ne!(form.geometry.settings[7].state & state::ON, 0);
        let draft = Options {
            shadow_quality: 2,
            ..live
        };
        form.set_text(Setting::ShadowQuality, &draft);
        assert_eq!(
            form.geometry.settings[19].text,
            draft.shadow_quality_text().unwrap().as_bytes().to_vec()
        );
        form.set_keys(&["Q".to_string()]);
        assert_eq!(form.geometry.controls[1].text, b"Q".to_vec());
        form.set_device(Some("GPU"), false, 8192);
        assert_eq!(form.geometry.settings[1].text, b"GPU".to_vec());
        assert_eq!(form.geometry.settings[3].text, b"8192 MB".to_vec());
        // No figure: "Unknown"; a Deck names itself.
        form.set_device(None, true, 0);
        assert_eq!(form.geometry.settings[1].text, b"Valve Steam Deck".to_vec());
        assert_eq!(form.geometry.settings[3].text, b"Unknown".to_vec());
    }

    #[test]
    fn the_hovered_record_takes_the_hover_bits() {
        let mut form = FormState::open(SCREEN, 4);
        form.pointer(647, 116);
        assert_eq!(form.geometry.main[1].state & HOVER_BITS, HOVERED);
        // Moving to the volume slider moves the bits.
        form.pointer(427 + 200, 244);
        assert_eq!(form.geometry.main[1].state & HOVER_BITS, 0);
        assert_eq!(form.geometry.settings[6].state & HOVER_BITS, HOVERED);
        // A trackbar takes no hover message: its value is unchanged.
        assert_eq!(form.geometry.settings[6].value, 0.0);
    }

    /// The Keyboard and Controller tabs show only with Steam Input; clicking
    /// the one that turns on reports its action, the one already on nothing.
    #[test]
    fn the_controller_tabs_report_their_view() {
        let mut form = FormState::open(SCREEN, 4);
        form.controller_tabs = true;
        form.switch(1);
        // With Steam Input the view pair shows below the page pair, the
        // keyboard view lit.
        assert_eq!(form.geometry.main[2].state & HIDDEN, 0);
        assert_ne!(form.geometry.main[2].state & state::ON, 0);
        assert_eq!(form.geometry.main[3].state & state::ON, 0);
        // At unit 4 the view tabs sit 40 pixels below the page tabs: x 12
        // and 212 from the client, y 52, 192 wide and 24 high.
        let controller = (437 + 212 + 20, 94 + 52 + 10);
        assert_eq!(
            click(&mut form, controller.0, controller.1),
            vec![FormEvent::Action(FormAction::ControllerView)]
        );
        assert_ne!(form.geometry.main[3].state & state::ON, 0);
        assert_eq!(form.geometry.main[2].state & state::ON, 0);
        // Clicking it again (still on) does nothing; the Keyboard tab
        // reports the other view.
        assert_eq!(click(&mut form, controller.0, controller.1), vec![]);
        let keyboard = (437 + 12 + 20, 94 + 52 + 10);
        assert_eq!(
            click(&mut form, keyboard.0, keyboard.1),
            vec![FormEvent::Action(FormAction::KeyboardView)]
        );
        // The Keyboard tab lights again; the page stays 21 units down.
        assert_ne!(form.geometry.main[2].state & state::ON, 0);
        assert_eq!(form.geometry.main[3].state & state::ON, 0);
        assert_eq!(form.controls_page.y, 84);
        assert_eq!(form.controls_page.h, 388);
        // Going back to the Settings page hides the view pair.
        form.switch(0);
        assert_eq!(form.geometry.main[2].state, HIDDEN);
        assert_eq!(form.geometry.main[3].state, HIDDEN);
        // Selecting the controller view by hand lights its tab.
        form.switch(1);
        form.select_controller_view(true);
        assert_ne!(form.geometry.main[3].state & state::ON, 0);
        assert_eq!(form.geometry.main[2].state & state::ON, 0);
    }

    #[test]
    fn the_help_box_sits_inside_the_controls_page() {
        // At unit 4 the page's client is 397 wide: the box at 8, 192, 381
        // wide and 200 high.
        let form = FormState::open(SCREEN, 4);
        assert_eq!(
            form.help_box(),
            Rect {
                x: 8,
                y: 192,
                w: 381,
                h: 200
            }
        );
        // At unit 6 the Controls page's client is 594 wide: 12, 288, 570 by
        // 300 (the scrollbar and borders take the rest of its 615).
        let form = FormState::open((1920, 1080), 6);
        assert_eq!(
            form.help_box(),
            Rect {
                x: 12,
                y: 288,
                w: 570,
                h: 300
            }
        );
        // The text is the original's, three paragraphs with its three
        // highlighted key names.
        assert_eq!(HELP.matches("[tcol=FFE399]").count(), 3);
        assert_eq!(HELP.matches("[/tcol]").count(), 3);
        assert_eq!(HELP.matches("\r\r").count(), 2);
        assert!(HELP.starts_with("Hold [tcol=FFE399]Powers[/tcol]"));
        assert!(HELP.ends_with("pressing [tcol=FFE399]Talk[/tcol]."));
    }

    #[test]
    fn a_resize_keeps_the_page_and_the_rows() {
        let mut form = FormState::open(SCREEN, 4);
        form.switch(1);
        form.geometry.settings[6].value = 0.25;
        form.resize((1920, 1080), 6);
        assert_eq!(form.page, 1);
        assert_eq!(form.controls_page.y, 84);
        assert_eq!(form.controls_page.h, 624);
        assert_eq!(form.geometry.settings[6].value, 0.25);
        assert_eq!(form.geometry.form.w, 646);
    }
}

#[cfg(test)]
mod screen_tests {
    use super::*;

    #[test]
    fn the_row_ids() {
        assert_eq!(Setting::Volume.id(), 3);
        assert_eq!(Setting::ShadowQuality.id(), 11);
        assert_eq!(Setting::Checkpoints.id(), 17);
        assert_eq!(Setting::from_id(10), None);
        assert_eq!(Setting::from_id(11), Some(Setting::ShadowQuality));
        assert!(Setting::Music.is_checkbox());
        assert!(!Setting::Volume.is_checkbox());
    }

    #[test]
    fn the_page_opens_on_the_original_values() {
        let live = Options {
            supersample: 3,
            shading_quality: 5,
            texture_detail: 2,
            texture_quality: 5,
            shadow_detail: 5,
            shadow_quality: 2,
            multisample: 8,
            vertical_sync: false,
            show_fps: true,
            window_mode: false,
            show_tips: false,
            checkpoints: false,
            volume: 0.75,
            ..Options::default()
        };
        let draft = live;
        let rows = populate(&live, &draft);
        // The live volume over the 1.5 maximum.
        assert_eq!(rows[0], (Setting::Volume, RowState::slider(0.5, None)));
        assert_eq!(rows[1], (Setting::Music, RowState::check(true)));
        assert_eq!(rows[2].1.text, Some(" Medium"));
        assert_eq!(rows[3].1.text, Some(" High"));
        assert_eq!(rows[4].1.text, Some(" Low"));
        assert_eq!(rows[5].1.text, Some(" Very high"));
        assert_eq!(rows[6].1.text, Some(" High"));
        assert_eq!(rows[7].1.text, Some(" Low"));
        // Super-sampling is on, so the anti-aliasing row is disabled and keeps
        // its record's position: it has no opening value.
        assert_eq!(
            rows[8],
            (
                Setting::Multisample,
                RowState {
                    fraction: None,
                    checked: false,
                    enabled: false,
                    text: None
                }
            )
        );
        // Vertical sync is not reset when the page opens.
        assert_eq!(rows[9], (Setting::VerticalSync, RowState::check(false)));
        assert_eq!(rows[10], (Setting::ShowFps, RowState::check(true)));
        assert_eq!(rows[11], (Setting::WindowMode, RowState::check(false)));
        assert_eq!(rows[12], (Setting::ShowTips, RowState::check(false)));
        assert_eq!(rows[13], (Setting::Checkpoints, RowState::check(false)));
    }

    #[test]
    fn the_anti_aliasing_row_is_enabled_without_super_sampling() {
        let live = Options::default();
        let draft = Options {
            multisample: 4,
            ..live
        };
        let rows = populate(&live, &draft);
        assert!(rows[8].1.enabled);
        assert_eq!(rows[8].1.fraction, Some(2.0 / 3.0));
        assert_eq!(rows[8].1.text, Some(" 4x"));
        // The live options, not the draft, decide it and fill the super-
        // sampling, volume, music and window rows.
        let draft = Options {
            supersample: 4,
            volume: 0.3,
            music: false,
            window_mode: false,
            ..live
        };
        let rows = populate(&live, &draft);
        assert!(rows[8].1.enabled);
        assert_eq!(rows[2].1.text, Some(" Off"));
        assert_eq!(rows[0].1.fraction, Some(2.0 / 3.0));
        assert!(rows[1].1.checked);
        assert!(rows[11].1.checked);
    }

    #[test]
    fn a_slider_writes_its_step_and_says_where_it_goes() {
        let mut o = Options::default();
        assert_eq!(on_slider(&mut o, Setting::Volume, 0.5), Effect::Live);
        assert_eq!(o.volume, 0.75);

        assert_eq!(
            on_slider(&mut o, Setting::Supersample, 2.0 / 3.0),
            Effect::Draft
        );
        assert_eq!(o.supersample, 3);
        assert_eq!(
            on_slider(&mut o, Setting::ShadingQuality, 1.0 / 3.0),
            Effect::Draft
        );
        assert_eq!(o.shading_quality, 4);
        // The texture sliders also set the live options at once.
        assert_eq!(on_slider(&mut o, Setting::TextureDetail, 1.0), Effect::Live);
        assert_eq!(o.texture_detail, 3);
        assert_eq!(
            on_slider(&mut o, Setting::TextureQuality, 2.0 / 3.0),
            Effect::Live
        );
        assert_eq!(o.texture_quality, 4);
        assert_eq!(on_slider(&mut o, Setting::ShadowDetail, 1.0), Effect::Draft);
        assert_eq!(o.shadow_detail, 5);
        assert_eq!(
            on_slider(&mut o, Setting::ShadowQuality, 2.0 / 3.0),
            Effect::Draft
        );
        assert_eq!(o.shadow_quality, 3);

        // The anti-aliasing step writes the samples even with super-sampling
        // on: the set of 0 the super-sampling slider sends lands as 0.
        let mut off = Options::default();
        assert_eq!(
            on_slider(&mut off, Setting::Multisample, 1.0),
            Effect::Draft
        );
        assert_eq!(off.multisample, 8);
        on_slider(&mut off, Setting::Supersample, 1.0 / 3.0);
        assert_eq!(
            on_slider(&mut off, Setting::Multisample, 0.0),
            Effect::Draft
        );
        assert_eq!(off.multisample, 0);
    }

    #[test]
    fn a_checkbox_writes_its_flag_and_says_where_it_goes() {
        let mut o = Options::default();
        assert_eq!(
            on_toggle(&mut o, Setting::VerticalSync, false),
            Effect::Live
        );
        assert!(!o.vertical_sync);
        assert_eq!(on_toggle(&mut o, Setting::ShowFps, true), Effect::Live);
        assert!(o.show_fps);
        assert_eq!(on_toggle(&mut o, Setting::WindowMode, false), Effect::Draft);
        assert!(!o.window_mode);
        assert_eq!(on_toggle(&mut o, Setting::ShowTips, false), Effect::Live);
        assert!(!o.show_tips);
        assert_eq!(on_toggle(&mut o, Setting::Checkpoints, false), Effect::Live);
        assert!(!o.checkpoints);
        // Music only mutes the session: the option keeps the startup value.
        assert_eq!(
            on_toggle(&mut o, Setting::Music, false),
            Effect::SessionOnly
        );
        assert!(o.music);
        assert_eq!(on_toggle(&mut o, Setting::Volume, true), Effect::Ignored);
    }

    #[test]
    fn the_aim_speed_row() {
        let mut o = Options::default();
        assert_eq!(set_aim_speed(&mut o, 0.29), 30);
        assert_eq!(o.aim_speed, 0.29);
        assert_eq!(set_aim_speed(&mut o, 0.0), 1);
        assert_eq!(set_aim_speed(&mut o, 1.0), 100);
    }

    #[test]
    fn the_controls_rows_use_their_slots_once() {
        let mut slots: Vec<usize> = BINDINGS.iter().filter_map(|b| b.slot).collect();
        slots.sort_unstable();
        slots.dedup();
        // The page has no row for the primary mouse button (slot 0); every
        // other slot appears once.
        assert_eq!(slots, (1..23).collect::<Vec<usize>>());
        assert_eq!(BINDINGS[0].slot, Some(1)); // Forward
        assert_eq!(BINDINGS[1].slot, Some(3)); // Back
        assert_eq!(BINDINGS[6].slot, Some(8)); // Orbit Camera
        assert_eq!(BINDINGS[17].slot, Some(20)); // Use Power (Q)
        assert_eq!(BINDINGS[18].slot, Some(18)); // Power Bar (C)
        assert_eq!(BINDINGS[21].slot, Some(19)); // Power Tree (P)
        assert!(BINDINGS.iter().all(|b| b.keyed));
        // The key cells' ids are one more than their saved-table slots, so
        // they are 1 to 23 without 22 (the saved table's unused slot).
        let mut ids: Vec<u16> = BINDINGS.iter().map(|b| b.action).collect();
        ids.sort_unstable();
        assert_eq!(ids, (1..22).chain(23..24).collect::<Vec<u16>>());
    }

    #[test]
    fn the_form_actions() {
        assert_eq!(form_action(0), Some(FormAction::SettingsTab));
        assert_eq!(form_action(1), Some(FormAction::ControlsTab));
        assert_eq!(form_action(2), Some(FormAction::KeyboardView));
        assert_eq!(form_action(3), Some(FormAction::ControllerView));
        assert_eq!(form_action(4), None);
        assert_eq!(form_action(6), Some(FormAction::Defaults));
        assert_eq!(form_action(7), Some(FormAction::Apply));
        assert_eq!(form_action(8), Some(FormAction::Cancel));
        assert_eq!(form_action(9), None);
        assert_eq!(action_page(FormAction::SettingsTab), Some(0));
        assert_eq!(action_page(FormAction::ControlsTab), Some(1));
        assert_eq!(action_page(FormAction::Apply), None);
        assert_eq!(controller_view_tab(FormAction::KeyboardView), Some(false));
        assert_eq!(controller_view_tab(FormAction::ControllerView), Some(true));
        assert_eq!(controller_view_tab(FormAction::Defaults), None);
    }
}

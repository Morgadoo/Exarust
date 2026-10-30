//! The GUI's window manager (`RF_GUI_Main`): the focus and its tooltip
//! request, the modal form, the pointer's hover, press, double click, drag
//! threshold, capture and release, the drag-and-drop hand-over, the cursor
//! shape, the level switch's derived lengths, the title-bar grips, a form's
//! placement and clamp, a toggle's group switch, the control-record
//! wrappers and the message box's construction.
//!
//! A form is anything implementing [`FormHost`]: the manager only needs the
//! virtual methods the original calls (hit test, select, press, release,
//! drag move, drop, close). The forms' own classes (the original's
//! `TGUIForm`) belong to the forms; [`PlainForm`] is a basic record over
//! [`crate::gui_controls`] for tests and simple screens. The root's hit test
//! descends the form tree, the last child first; that descent is the
//! forms' rule and is modelled here only so a task can stand alone.
//!
//! Times are the GUI clock's 32-bit stamps; "none" for a control is
//! `i32::MIN`, as the original stores it. Rules: `docs/UI.md` (Window
//! manager).

use crate::form::Anchor;
use crate::gui_controls::{
    Control, ControlEvent, Message, dispatch, message, state as control_state,
};
use crate::gui_rescale::{MessageBoxLayout, message_box, message_box_layout};

/// A form's identity in a [`GuiTask`].
pub type FormId = u32;

/// The control value meaning "none".
pub const NO_CONTROL: i32 = i32::MIN;

/// The tooltip's delay, and how far a tooltip request is back-dated when a
/// tooltip is already showing (so the next one shows at once).
pub const TOOLTIP_DELAY: u32 = 400;

/// Two presses of the same button within this many clock units, at most
/// one pixel apart, are a double click.
pub const DOUBLE_CLICK: u32 = 450;

/// The active control the forced idle step stores, so the next focus
/// always reselects.
pub const RESELECT: i32 = -21;

/// The rectangle of a form, in root pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuiRect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Default for GuiRect {
    fn default() -> Self {
        Self::new(0, 0, 0, 0)
    }
}

impl GuiRect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            left: x,
            top: y,
            right: x + w,
            bottom: y + h,
        }
    }

    pub fn width(self) -> i32 {
        self.right - self.left
    }

    pub fn height(self) -> i32 {
        self.bottom - self.top
    }

    /// Whether a point is inside; the far edges are outside.
    pub fn contains(self, x: i32, y: i32) -> bool {
        self.left <= x && x < self.right && self.top <= y && y < self.bottom
    }
}

/// The form methods the window manager calls (the original's virtual
/// slots). Every result is the original's bit word: bits 1 and 2 together
/// ask for a redraw, 0x100 on a select marks a control with a tooltip.
pub trait FormHost: Send + Sync {
    /// The parent in the form tree, if any.
    fn parent(&self) -> Option<FormId>;
    /// The form's absolute rectangle.
    fn bounds(&self) -> GuiRect;
    fn is_visible(&self) -> bool {
        true
    }
    /// A root point in the form's own coordinates, for the modal form's
    /// hit test (the forms decide; identity by default).
    fn to_local(&self, x: i32, y: i32) -> (i32, i32) {
        (x, y)
    }
    /// The control code under a point: a positive 1-based control, 0 or a
    /// negative edge code.
    fn hit_test(&self, _x: i32, _y: i32) -> i32 {
        0
    }
    /// The cursor shape the hit test reports with a code.
    fn hit_cursor(&self, _code: i32) -> i32 {
        0
    }
    /// Selects a control (0 deselects).
    fn select(&mut self, _control: i32) -> u32 {
        0
    }
    /// The selected control.
    fn selected_control(&self) -> i32 {
        0
    }
    /// Focused with the control -1: the form's own response.
    fn focus_without_control(&mut self) -> u32 {
        0
    }
    /// A press word (the button bits with 0x1000 for a press, 0x8000 for
    /// the second press of a double click, 0x2000 for a click's release and
    /// 0x4000 for a drag start).
    fn press(&mut self, _word: u32) -> u32 {
        0
    }
    /// A release word, for the form holding a press or the capture.
    fn release(&mut self, _word: u32) -> u32 {
        0
    }
    /// The pointer moved while this form holds the capture; bit 0x80 ends
    /// the capture.
    fn drag_move(&mut self, _x: i32, _y: i32) -> u32 {
        0
    }
    /// A drag-and-drop payload of `kind` is over `control`; bit 1 accepts
    /// it, 0x200 keeps the hover.
    fn drop_over(&mut self, _kind: u32, _control: i32) -> u32 {
        0
    }
    /// The payload is dropped on `control`.
    fn drop(&mut self, _kind: u32, _control: i32) {}
    /// The payload's source hears that no one took it.
    fn drop_cancelled(&mut self, _kind: u32) {}
    /// The form is told to close.
    fn close(&mut self, _arg: i32) {}
    /// The form is marked for deletion.
    fn doomed(&self) -> bool {
        false
    }
    /// The gamepad's own focus test for a control (the original asks the
    /// Steam input layer when it drives the pointer).
    fn gamepad_focusable(&self, _control: i32) -> bool {
        true
    }
    /// The selected control's state asks that leaving the form drops the
    /// focus (state bit 0x400), in gamepad mode.
    fn drops_focus(&self, _control: i32) -> bool {
        false
    }
    /// The newly selected control's style asks for its form's focus (style
    /// bit 0x800000), in gamepad mode.
    fn takes_focus(&self, _control: i32) -> bool {
        false
    }
    /// The record whose tooltip a control shows, or a negative value for
    /// the form's own hint.
    fn hint_record(&self, _control: i32) -> i32 {
        -1
    }
    /// The tooltip class a record asks for (its 0x8000 message); 0 or less
    /// is the default tooltip.
    fn tooltip_class(&self, _record: i32) -> i32 {
        0
    }
}

/// A basic form: an absolute rectangle, a parent, its controls and the
/// control message dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainForm {
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub visible: bool,
    pub controls: Vec<Control>,
    /// The selected control, 1-based; 0 for none.
    pub selected: i32,
    pub unit: i32,
    pub events: Vec<ControlEvent>,
}

impl PlainForm {
    pub fn new(bounds: GuiRect, unit: i32) -> Self {
        Self {
            parent: None,
            bounds,
            visible: true,
            controls: Vec::new(),
            selected: 0,
            unit,
            events: Vec::new(),
        }
    }

    /// The 1-based control whose rectangle holds the point, or 0.
    pub fn control_at(&self, x: i32, y: i32) -> i32 {
        for (index, control) in self.controls.iter().enumerate() {
            if GuiRect::new(control.x, control.y, control.w, control.h).contains(x, y) {
                return index as i32 + 1;
            }
        }
        0
    }

    fn send(&mut self, message: u32, param: u32) -> u32 {
        let Ok(index) = usize::try_from(self.selected - 1) else {
            return 0;
        };
        if self.selected < 1 || index >= self.controls.len() {
            return 0;
        }
        dispatch(
            &mut self.controls,
            index,
            Message {
                message,
                param,
                extra: param & 7,
                over: true,
                axes: (0.0, 0.0),
            },
            self.unit,
            &mut self.events,
        )
    }
}

impl FormHost for PlainForm {
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
        self.control_at(x, y)
    }

    fn select(&mut self, control: i32) -> u32 {
        if self.selected == control {
            return 0;
        }
        self.selected = control;
        3
    }

    fn selected_control(&self) -> i32 {
        self.selected
    }

    fn press(&mut self, word: u32) -> u32 {
        self.send(message::PRESS, word)
    }

    fn release(&mut self, word: u32) -> u32 {
        self.send(message::RELEASE, word)
    }
}

/// The lengths the GUI level switch derives from the unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiMetrics {
    /// The unit the level ends with.
    pub unit: i32,
    /// The title bar: six units less two.
    pub title: i32,
    /// A title-bar control's width (and the scroll rail): four units and
    /// one.
    pub control: i32,
    /// The step between title-bar controls: half a unit (shifted) and a
    /// control.
    pub control_step: i32,
    /// A resize grip: four units less one.
    pub grip: i32,
    /// A quarter unit.
    pub quarter: f32,
}

impl GuiMetrics {
    /// The lengths for a plain unit.
    pub fn at(unit: i32) -> Self {
        let control = unit * 4 + 1;
        Self {
            unit,
            title: unit * 6 - 2,
            control,
            control_step: (unit >> 1) + control,
            grip: unit * 4 - 1,
            quarter: unit as f32 * 0.25,
        }
    }

    /// The lengths the level switch sets for a level (clamped to 0 through
    /// 5). The lengths come from level + 3, or 8 at level 4, before the
    /// switch on the level picks the fonts; level 5 then raises the unit to
    /// 12, so its other lengths stay those of unit 8.
    pub fn for_level(level: i32) -> Self {
        let level = level.clamp(0, 5);
        let first = if level == 4 { 8 } else { level + 3 };
        let mut metrics = Self::at(first);
        if level == 5 {
            metrics.unit = 12;
        }
        metrics
    }
}

/// The pointer's drag distance at a screen height: the height times the
/// original's double literal `0.001851851851851852` (one five-hundred-and-
/// fortieth), rounded half to even in double precision, at least 2.
pub fn drag_limit(screen_height: i32) -> i32 {
    let limit = (f64::from(screen_height) * 0.001851851851851_852).round_ties_even() as i32;
    limit.max(2)
}

/// The pointer's drag threshold: during the first 191 clock units after a
/// press, `(200 − elapsed) / 20` limits (a truncating division), then the
/// limit itself.
pub fn drag_threshold(elapsed: i32, limit: i32) -> i32 {
    if elapsed < 0xbf {
        ((200 - i64::from(elapsed)) / 20) as i32 * limit
    } else {
        limit
    }
}

/// The fields of a form the grip and title-bar rectangles read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EdgeForm {
    pub width: i32,
    pub height: i32,
    /// The border inset.
    pub border: i32,
    /// Two more client fields the top-right and bottom-left-of-right grips
    /// start from (their meaning beyond this use is not established).
    pub top: i32,
    pub inner_right: i32,
    /// The style word: bit 2 adds a control before the -6 button, bits 2
    /// and 8 move the -5 button.
    pub style: u32,
}

/// The rectangle `[left, top, right, bottom]` of a grip or title-bar code
/// in the form's own pixels, or `None` for a code the helper does not know.
/// The grips (-14, -13, -10, -9) are `grip` squares inside the border; the
/// title-bar controls (-6, -5, -4) are `control` wide from the right edge
/// less a unit, stepped left by `control_step`, from `unit − 1` down to
/// `title − unit + 3`.
pub fn edge_rect(form: EdgeForm, metrics: GuiMetrics, code: i32) -> Option<[i32; 4]> {
    let GuiMetrics {
        unit,
        title,
        control,
        control_step,
        grip,
        ..
    } = metrics;
    let square = |x: i32, y: i32| [x, y, x + grip, y + grip];
    let bottom = (form.height - form.border) - grip;
    let right = (form.width - form.border) - grip;
    let button = |x: i32| [x, unit - 1, x + control, (title - unit) + 3];
    let first = (form.width - control) - unit;
    Some(match code {
        -14 => square(form.inner_right - grip, bottom),
        -13 => square(form.border, bottom),
        -10 => square(right, bottom),
        -9 => square(right, form.top),
        -6 => button(if form.style & 2 == 2 {
            first - control_step
        } else {
            first
        }),
        -5 => button(match form.style & 10 {
            10 => first - control_step * 2,
            0 => first,
            _ => first - control_step,
        }),
        -4 => button(first),
        _ => return None,
    })
}

/// A child form's placement request: the x and y it would like, and the
/// fallbacks when the form does not fit the screen there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceRequest {
    pub desired_x: i32,
    /// The x tried when the desired one overflows (negative: none).
    pub fallback_x: i32,
    pub desired_y: i32,
    /// The bottom the form is lifted above when the desired y overflows.
    pub fallback_bottom: i32,
}

/// Places a `width` × `height` form on a `screen`: the desired x when the
/// form fits there (0 when the fallback x is negative); otherwise the
/// fallback x when that fits, else the screen's right edge less the width
/// and two. The desired y when the form fits, else the fallback bottom
/// less the height.
pub fn place_form(
    screen: (i32, i32),
    width: i32,
    height: i32,
    request: PlaceRequest,
) -> (i32, i32) {
    let fits = |x: i32| i64::from(width) + i64::from(x) <= i64::from(screen.0);
    let x = if !fits(request.desired_x) {
        if request.fallback_x < 0 || !fits(request.fallback_x) {
            (screen.0 - width) - 2
        } else {
            request.fallback_x
        }
    } else if request.fallback_x < 0 {
        0
    } else {
        request.desired_x
    };
    let y = if i64::from(screen.1) < i64::from(height) + i64::from(request.desired_y) {
        request.fallback_bottom - height
    } else {
        request.desired_y
    };
    (x, y)
}

/// Keeps a form on the screen: a negative x becomes 0, an x whose right
/// edge passes the screen moves the form flush with it, and the y is at
/// least a unit. The size is not touched.
pub fn clamp_form(x: i32, y: i32, width: i32, unit: i32, screen_width: i32) -> (i32, i32) {
    let x = if x < 0 {
        0
    } else if i64::from(screen_width) < i64::from(width) + i64::from(x) {
        screen_width - width
    } else {
        x
    };
    (x, y.max(unit))
}

/// Switches a toggle control on or off. Off clears its on bit. On first
/// clears the on and lit bits of every control of its group that is on
/// (itself included), reporting each unless `quiet`, then sets its on bit.
/// The lit bit then follows the on bit, and the control reports itself
/// unless `quiet`. A control of group 0 has no group.
pub fn set_control_on(
    controls: &mut [Control],
    index: usize,
    on: bool,
    quiet: bool,
    events: &mut Vec<ControlEvent>,
) {
    let Some(control) = controls.get(index) else {
        return;
    };
    let group = control.group;
    if on {
        if group != 0 {
            for (other, control) in controls.iter_mut().enumerate() {
                if control.group == group && control.state & control_state::ON != 0 {
                    control.state &= !(control_state::ON | control_state::LIT);
                    if !quiet && control.reports {
                        events.push(ControlEvent::Changed(other));
                    }
                }
            }
        }
        controls[index].state |= control_state::ON;
    } else {
        controls[index].state &= !control_state::ON;
    }
    let control = &mut controls[index];
    control.state =
        (control.state & !control_state::LIT) | ((control.state & control_state::ON) << 1);
    if !quiet && control.reports {
        events.push(ControlEvent::Changed(index));
    }
}

/// What showing a control did besides its own state: the other controls of
/// its group that were hidden, and the state word's low byte after the
/// call with the lit bit masked off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlVisibility {
    /// The other controls of the group whose shown and lit bits cleared.
    pub hidden: usize,
    /// The control's state's low byte, with the lit bit masked off.
    pub state_byte: u8,
}

/// Shows or hides a control (the original's visibility helper). Hiding
/// clears its shown bit (the lit bit then follows it down). Showing first
/// clears the shown and lit bits of every control of its group that is
/// shown (the control itself included; a group of 0 has none), reporting
/// each unless `quiet`, then sets its own shown bit; the lit bit then
/// follows the shown bit. Unless `quiet`, the control itself reports too.
/// Returns the other controls hidden and the state's low byte (lit masked
/// off).
pub fn set_control_visible(
    controls: &mut [Control],
    index: usize,
    show: bool,
    quiet: bool,
    events: &mut Vec<ControlEvent>,
) -> ControlVisibility {
    let Some(control) = controls.get(index) else {
        return ControlVisibility::default();
    };
    let group = control.group;
    let mut hidden = 0;
    if !show {
        controls[index].state &= !control_state::ON;
    } else {
        if group != 0 {
            for (other, control) in controls.iter_mut().enumerate() {
                if control.group == group && control.state & control_state::ON != 0 {
                    control.state &= !(control_state::ON | control_state::LIT);
                    if !quiet && control.reports {
                        events.push(ControlEvent::Changed(other));
                    }
                    if other != index {
                        hidden += 1;
                    }
                }
            }
        }
        controls[index].state |= control_state::ON;
    }
    let control = &mut controls[index];
    control.state =
        (control.state & !control_state::LIT) | ((control.state & control_state::ON) << 1);
    let state_byte = (control.state & !control_state::LIT) as u8;
    if !quiet && control.reports {
        events.push(ControlEvent::Changed(index));
    }
    ControlVisibility { hidden, state_byte }
}

/// Where a message box's result goes: the box stores its parent's result
/// field and tells the parent to close.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MessageBoxButton {
    /// The value stored: 1 for Ok, 0 for Cancel or the close mark.
    pub result: i32,
    /// The parent form hears the close message.
    pub closes: bool,
    /// The box is destroyed (the close mark's own message).
    pub destroys: bool,
}

/// The Ok button's answer (message box callback): store 1, close the
/// parent.
pub const MESSAGE_OK: MessageBoxButton = MessageBoxButton {
    result: 1,
    closes: true,
    destroys: false,
};

/// The Cancel button's answer: store 0, close the parent.
pub const MESSAGE_CANCEL: MessageBoxButton = MessageBoxButton {
    result: 0,
    closes: true,
    destroys: false,
};

/// The close mark's answer: store 0, destroy the box through the parent.
pub const MESSAGE_CLOSE: MessageBoxButton = MessageBoxButton {
    result: 0,
    closes: false,
    destroys: true,
};

/// The three callbacks a message box's buttons and close mark run.
pub fn message_box_button(ok: bool, close_mark: bool) -> MessageBoxButton {
    if close_mark {
        MESSAGE_CLOSE
    } else if ok {
        MESSAGE_OK
    } else {
        MESSAGE_CANCEL
    }
}

/// Sets a control's text and tells it (message 0x400).
pub fn set_control_text(
    controls: &mut [Control],
    index: usize,
    text: &[u8],
    unit: i32,
    events: &mut Vec<ControlEvent>,
) -> u32 {
    let Some(control) = controls.get_mut(index) else {
        return 0;
    };
    control.text = text.to_vec();
    dispatch(
        controls,
        index,
        Message {
            message: 0x400,
            ..Default::default()
        },
        unit,
        events,
    )
}

/// Sends a control the set message (0x200) with a value and flags; `marked`
/// adds flag bit 1, as the original's second wrapper does. Returns 0x200.
pub fn set_control(
    controls: &mut [Control],
    index: usize,
    value: u32,
    flags: u32,
    marked: bool,
    unit: i32,
    events: &mut Vec<ControlEvent>,
) -> u32 {
    if index < controls.len() {
        dispatch(
            controls,
            index,
            Message {
                message: message::SET,
                param: value,
                extra: flags | u32::from(marked),
                ..Default::default()
            },
            unit,
            events,
        );
    }
    message::SET
}

/// The tooltip a focus change asks for: the form and control focused with
/// a 0x100 select result, and the clock stamp its delay counts from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooltipRequest {
    pub form: Option<FormId>,
    pub control: i32,
    pub stamp: u32,
}

impl Default for TooltipRequest {
    fn default() -> Self {
        Self {
            form: None,
            control: 0,
            stamp: u32::MAX,
        }
    }
}

/// A tooltip the tick shows: the form, the record its text comes from (or
/// the form's own hint when negative) and its class (0 or less: the
/// default tooltip).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooltipShow {
    pub form: FormId,
    pub record: i32,
    pub class: i32,
}

/// The drag-and-drop hand-over: a payload kind (0: none), its source
/// form and control, the form under the pointer and the form and control
/// that accepted it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DropState {
    pub kind: u32,
    pub source: Option<FormId>,
    pub source_control: i32,
    pub under: Option<FormId>,
    pub target: Option<FormId>,
    pub target_control: i32,
    /// Bit 4 shows the no-drop cursor; bit 0x10 marks a cancelled drop.
    pub flags: u32,
    /// A drag image follows the pointer (every move redraws).
    pub image: bool,
}

/// The pointer's state in the task.
#[derive(Debug, Clone, PartialEq)]
pub struct PointerState {
    pub x: i32,
    pub y: i32,
    /// Where the press (or the capture) started; the drag threshold and the
    /// double click measure from here.
    pub press_x: i32,
    pub press_y: i32,
    /// Where the last button press happened.
    pub click_x: i32,
    pub click_y: i32,
    /// The form and control under the pointer.
    pub target: Option<FormId>,
    pub target_control: i32,
    /// The held press word: the button bits, 0x100 once the drag threshold
    /// is passed, and the press's flags.
    pub buttons: u32,
    /// The form holding the pointer capture.
    pub capture: Option<FormId>,
    /// The drag distance ([`drag_limit`]).
    pub limit: i32,
    /// The last press's word and stamp, for the double click.
    pub last_word: u16,
    pub press_stamp: u32,
    /// The buttons a double click consumed: their release is swallowed.
    pub consumed: u16,
    /// A form took the press: the release goes to it rather than clicking.
    pub held: bool,
    /// The next update re-reads the pointer even when it did not move.
    pub force: bool,
    /// The system pointer is captured.
    pub os_captured: bool,
}

impl Default for PointerState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            press_x: 0,
            press_y: 0,
            click_x: 0,
            click_y: 0,
            target: None,
            target_control: 0,
            buttons: 0,
            capture: None,
            limit: 2,
            last_word: 0,
            press_stamp: 0,
            consumed: 0,
            held: false,
            force: false,
            os_captured: false,
        }
    }
}

/// The window manager.
pub struct GuiTask {
    forms: Vec<Option<Box<dyn FormHost>>>,
    /// The root form, the first added (the original's 800 × 600 task form).
    pub root: Option<FormId>,
    pub screen: (i32, i32),
    /// The focused form and its control.
    pub active: Option<FormId>,
    pub active_control: i32,
    /// The modal form, which takes every hit and closes on a press outside
    /// it.
    pub modal: Option<FormId>,
    pub tooltip: TooltipRequest,
    /// The tooltip form is visible (fading out included).
    pub tooltip_visible: bool,
    /// The tooltip is up, and when it hides by itself.
    pub tooltip_showing: bool,
    pub tooltip_hide_at: u32,
    /// The clock a hidden tooltip finishes fading out at.
    pub tooltip_fade_until: Option<u32>,
    /// The GUI clock, set by the caller each frame.
    pub clock: u32,
    /// A gamepad drives the pointer (the focus asks the gamepad layer).
    pub gamepad: bool,
    pub pointer: PointerState,
    pub drop: DropState,
    /// The cursor shape last set.
    pub cursor: i32,
    /// A redraw was asked for.
    pub redraw: bool,
}

impl Default for GuiTask {
    fn default() -> Self {
        Self::new((800, 600))
    }
}

impl GuiTask {
    pub fn new(screen: (i32, i32)) -> Self {
        Self {
            forms: Vec::new(),
            root: None,
            screen,
            active: None,
            active_control: NO_CONTROL,
            modal: None,
            tooltip: TooltipRequest::default(),
            tooltip_visible: false,
            tooltip_showing: false,
            tooltip_hide_at: 0,
            tooltip_fade_until: None,
            clock: 0,
            gamepad: false,
            pointer: PointerState::default(),
            drop: DropState::default(),
            cursor: -1,
            redraw: false,
        }
    }

    /// Adds a form and returns its id; the first form becomes the root.
    pub fn add(&mut self, form: Box<dyn FormHost>) -> FormId {
        self.forms.push(Some(form));
        let id = self.forms.len() as FormId;
        if self.root.is_none() {
            self.root = Some(id);
        }
        id
    }

    pub fn form(&self, id: FormId) -> Option<&dyn FormHost> {
        self.forms
            .get((id as usize).wrapping_sub(1))
            .and_then(|form| form.as_deref())
    }

    pub fn form_mut(&mut self, id: FormId) -> Option<&mut (dyn FormHost + 'static)> {
        self.forms
            .get_mut((id as usize).wrapping_sub(1))
            .and_then(|form| form.as_deref_mut())
    }

    /// The form's root ancestor: the walk up stops at the first form whose
    /// parent is the root (or has none).
    pub fn root_of(&self, id: FormId) -> FormId {
        let mut current = id;
        for _ in 0..self.forms.len() {
            match self.form(current).and_then(FormHost::parent) {
                Some(parent) if Some(parent) != self.root && parent != current => current = parent,
                _ => break,
            }
        }
        current
    }

    fn select(&mut self, form: FormId, control: i32) -> u32 {
        self.form_mut(form).map_or(0, |f| f.select(control))
    }

    fn selected(&self, form: FormId) -> i32 {
        self.form(form).map_or(0, FormHost::selected_control)
    }

    /// Sets the cursor shape; the system is told only of a change.
    pub fn set_cursor(&mut self, shape: i32) -> bool {
        if shape == self.cursor {
            return false;
        }
        self.cursor = shape;
        true
    }

    /// The root's hit test: the deepest visible form under a point (the
    /// last child first), its control code and cursor. A point off the
    /// screen hits nothing, with the code 0 and the cursor -1.
    pub fn hit_root(&self, x: i32, y: i32) -> (Option<FormId>, i32, i32) {
        if x < 0 || self.screen.0 <= x || y < 0 || self.screen.1 <= y {
            return (None, 0, -1);
        }
        let Some(root) = self.root else {
            return (None, 0, -1);
        };
        let form = self.hit_in(root, x, y).unwrap_or(root);
        let code = self.form(form).map_or(0, |f| f.hit_test(x, y));
        let cursor = self.form(form).map_or(0, |f| f.hit_cursor(code));
        (Some(form), code, cursor)
    }

    fn hit_in(&self, form: FormId, x: i32, y: i32) -> Option<FormId> {
        let children: Vec<FormId> = (1..=self.forms.len() as FormId)
            .filter(|&id| {
                self.form(id)
                    .is_some_and(|f| f.parent() == Some(form) && f.is_visible())
            })
            .collect();
        for child in children.into_iter().rev() {
            if let Some(found) = self.hit_in(child, x, y) {
                return Some(found);
            }
            if self.form(child).is_some_and(|f| f.bounds().contains(x, y)) {
                return Some(child);
            }
        }
        None
    }

    /// The hit test the pointer uses: the modal form, given the point in its
    /// own coordinates, else the root's.
    pub fn hit(&self, x: i32, y: i32) -> (Option<FormId>, i32, i32) {
        if let Some(modal) = self.modal {
            let Some(form) = self.form(modal) else {
                return (Some(modal), 0, 0);
            };
            let (lx, ly) = form.to_local(x, y);
            let code = form.hit_test(lx, ly);
            return (Some(modal), code, form.hit_cursor(code));
        }
        self.hit_root(x, y)
    }

    /// The form under the stored pointer (the root's hit test, no modal).
    pub fn form_at_pointer(&self) -> Option<FormId> {
        self.hit_root(self.pointer.x, self.pointer.y).0
    }

    /// The focus change. While a modal form is set, only it (or no form)
    /// may take the focus; another form clears it. A new form deselects the
    /// old one (the gamepad first drops the focus if the old control asks)
    /// and starts with no control; a control the form has not selected is
    /// selected (the gamepad then lets a control take its form's focus); the
    /// control -1 adds the form's own response. When something changed and
    /// the result has 0x100, a tooltip is requested, stamped now, or 400
    /// earlier when a tooltip already shows and the result lacks 0x200.
    /// Returns the select results.
    pub fn focus_set(&mut self, form: Option<FormId>, control: i32) -> u32 {
        if self.modal.is_some() && form.is_some() && form != self.modal {
            self.active = None;
            self.active_control = NO_CONTROL;
            return 0;
        }
        let mut result = 0;
        let mut changed = false;
        if form != self.active {
            if let Some(previous) = self.active {
                if self.gamepad {
                    self.gamepad_leave(previous);
                }
                result = self.select(previous, 0) | 1;
            }
            self.active_control = NO_CONTROL;
            changed = true;
            self.active = form;
        }
        if let Some(active) = self.active {
            if control != self.active_control {
                changed = true;
            }
            let mut stored = self.active_control;
            if control != self.selected(active) {
                result |= self.select(active, control) | 1;
                stored = control;
                if self.gamepad {
                    self.gamepad_enter(active);
                }
            }
            self.active_control = stored;
            if control == -1 {
                result |= self
                    .form_mut(active)
                    .map_or(0, |f| f.focus_without_control());
            }
            if changed && result & 0x100 == 0x100 {
                let stamp = if !self.tooltip_visible || result & 0x200 != 0 {
                    self.clock
                } else {
                    self.clock.wrapping_sub(TOOLTIP_DELAY)
                };
                self.tooltip = TooltipRequest {
                    form: self.active,
                    control: self.active_control,
                    stamp,
                };
            }
        }
        if result & 3 == 3 {
            self.redraw = true;
        }
        result
    }

    /// The gamepad's leave check: a selected control whose state asks for
    /// it drops the focus altogether.
    fn gamepad_leave(&mut self, form: FormId) {
        let selected = self.selected(form);
        if selected > 0 && self.form(form).is_some_and(|f| f.drops_focus(selected)) {
            // The forms' own focus call; it does not check the gamepad again.
            self.gamepad = false;
            self.focus_set(None, 0);
            self.gamepad = true;
        }
    }

    /// The gamepad's enter check: a selected control whose style asks for
    /// it gives its form the focus.
    fn gamepad_enter(&mut self, form: FormId) {
        let selected = self.selected(form);
        if selected > 0 && self.form(form).is_some_and(|f| f.takes_focus(selected)) {
            self.focus_set(Some(form), selected);
        }
    }

    /// Hit tests a point and focuses what it finds, setting the cursor; on
    /// nothing, the focus goes to no form with control 1 and the cursor is
    /// the hit test's (-1 off the screen).
    pub fn hit_and_focus(&mut self, x: i32, y: i32) -> HitResult {
        let (form, code, cursor) = self.hit(x, y);
        let control = if form.is_none() { 1 } else { code };
        let result = self.focus_set(form, control);
        self.set_cursor(cursor);
        HitResult {
            form,
            control,
            cursor,
            result,
        }
    }

    /// Hit tests the stored pointer into the pointer's target; on nothing,
    /// the target is no form with control 1 and the cursor 0.
    fn hit_target(&mut self) {
        let (form, code, cursor) = self.hit(self.pointer.x, self.pointer.y);
        match form {
            Some(form) => {
                self.pointer.target = Some(form);
                self.pointer.target_control = code;
                self.set_cursor(cursor);
            }
            None => {
                self.pointer.target = None;
                self.pointer.target_control = 1;
                self.set_cursor(0);
            }
        }
    }

    /// Focuses the pointer's target, unless the gamepad refuses it.
    fn focus_target(&mut self) -> bool {
        if self.gamepad {
            let allowed = self.pointer.target.is_some_and(|form| {
                self.form(form)
                    .is_some_and(|f| f.gamepad_focusable(self.pointer.target_control))
            });
            if !allowed {
                return false;
            }
        }
        self.focus_set(self.pointer.target, self.pointer.target_control);
        true
    }

    /// Changes the form under a drop: the old one deselects, the new one
    /// starts with no control, then selects `control` when it differs.
    fn set_drop_hover(&mut self, form: Option<FormId>, control: i32) -> u32 {
        let mut result = 0;
        if form != self.pointer.target {
            if let Some(old) = self.pointer.target {
                result = self.select(old, 0) | 1;
            }
            self.pointer.target = form;
            self.pointer.target_control = NO_CONTROL;
        }
        if let Some(target) = self.pointer.target
            && control != self.pointer.target_control
        {
            result |= self.select(target, control) | 1;
            self.pointer.target_control = control;
        }
        if result & 3 == 3 {
            self.redraw = true;
        }
        result
    }

    /// The pointer update. Unless `force`, the system pointer `at` is read
    /// and nothing happens when it has not moved. With a button held and no
    /// drag flag yet, a move past the threshold sets 0x100 and starts the
    /// drag. Without a capture, an idle pointer hit tests and focuses its
    /// target, and a drag start sends the target a 0x4000 press (a 0x100
    /// answer makes it a drop source). A capture gets the move, ending the
    /// capture on 0x80; during a drop the form under the pointer is asked
    /// to accept it. Returns whether the pointer moved.
    pub fn update(&mut self, force: bool, at: (i32, i32)) -> bool {
        let mut force = force || self.pointer.force;
        self.pointer.force = false;
        if self.gamepad {
            force = false;
        }
        let mut moved = false;
        if !force {
            if at == (self.pointer.x, self.pointer.y) {
                return false;
            }
            if self.cursor == -2 || self.drop.image {
                self.redraw = true;
            }
            (self.pointer.x, self.pointer.y) = at;
            moved = true;
        }
        let mut start_drag = false;
        if self.pointer.buttons & 7 == 0 {
            self.pointer.buttons &= !0x100;
        } else if self.pointer.buttons & 0x6100 == 0 {
            let elapsed = self.clock.wrapping_sub(self.pointer.press_stamp) as i32;
            let threshold = i64::from(drag_threshold(elapsed, self.pointer.limit));
            let dx = (i64::from(self.pointer.x) - i64::from(self.pointer.press_x)).abs();
            let dy = (i64::from(self.pointer.y) - i64::from(self.pointer.press_y)).abs();
            if dx > threshold || dy > threshold {
                self.pointer.buttons |= 0x100;
                start_drag = true;
            }
        }
        let Some(capture) = self.pointer.capture else {
            if self.pointer.buttons & 7 == 0 {
                self.hit_target();
                self.focus_target();
            } else if !self.pointer.held && start_drag && self.pointer.target.is_some() {
                self.drop.under = None;
                let result = self.press_start((self.pointer.buttons & 0xffff) | 0x4000);
                if result & 0x100 != 0 {
                    self.drop.source = self.pointer.target;
                    self.drop.source_control = self.pointer.target_control;
                    self.drop.target = self.pointer.target;
                    self.drop.target_control = self.pointer.target_control;
                    self.pointer.target = None;
                }
            }
            return moved;
        };
        let mut result = 0;
        if self.drop.kind == 0 {
            let (x, y) = (self.pointer.x, self.pointer.y);
            result = self.form_mut(capture).map_or(0, |f| f.drag_move(x, y));
            if result & 0x80 == 0x80 {
                self.release(0);
                self.set_capture(None);
            }
        } else {
            let (under, code, _) = self.hit_root(self.pointer.x, self.pointer.y);
            let mut cursor = 0xb;
            self.drop.target = None;
            self.drop.under = under;
            if let Some(under) = under {
                let kind = self.drop.kind;
                result = self.form_mut(under).map_or(0, |f| f.drop_over(kind, code));
                if result & 1 != 0 {
                    self.drop.target = Some(under);
                    self.drop.target_control = code;
                    cursor = 0;
                }
            }
            if self.drop.flags & 4 != 0 {
                cursor = -2;
            }
            result |= if result & 0x201 == 0 {
                self.set_drop_hover(None, 0)
            } else {
                self.set_drop_hover(self.drop.target, self.drop.target_control)
            };
            self.set_cursor(cursor);
        }
        if result & 3 == 3 {
            self.redraw = true;
        }
        moved
    }

    /// The press dispatch: the capture, or the target (selecting its
    /// control first), gets the press word. Answering 0, the target goes
    /// back to the focus's control (or none). Otherwise, unless it answers
    /// 0x40, the form holds the press, comes to the front, and with bits 1
    /// and 4 takes the capture; else it goes back as above.
    fn press_start(&mut self, word: u32) -> u32 {
        let form = match self.pointer.capture {
            Some(capture) => capture,
            None => {
                let Some(target) = self.pointer.target else {
                    return 0;
                };
                self.select(target, self.pointer.target_control);
                target
            }
        };
        let result = self.form_mut(form).map_or(0, |f| f.press(word));
        let restore = |task: &mut Self, target: FormId| {
            let control = if Some(target) == task.active {
                task.active_control
            } else {
                0
            };
            task.select(target, control);
        };
        if result == 0 {
            if let Some(target) = self.pointer.target {
                restore(self, target);
            }
            return 0;
        }
        if result & 3 == 3 {
            self.redraw = true;
        }
        if result & 0x40 != 0x40 {
            self.pointer.held = true;
            if let Some(target) = self.pointer.target {
                let control = if result & 0x10 == 0x10 {
                    self.selected(target)
                } else {
                    0
                };
                self.bring_to_front(target, control);
                if result & 5 == 5 {
                    self.pointer.capture = Some(target);
                    self.pointer.target_control = NO_CONTROL;
                    self.pointer.os_captured = true;
                } else {
                    restore(self, target);
                }
            }
        }
        result
    }

    /// Brings a form to the front of its siblings with a control focused
    /// (the forms' activation; the order of the form list is its z-order).
    fn bring_to_front(&mut self, form: FormId, control: i32) {
        self.focus_set(Some(form), control);
    }

    /// A button press `word` (the button bits). A second button while a
    /// capture holds the first cancels it through the release. During a
    /// drop, the press ends it. A press outside the modal form closes the
    /// modal form. A press of the same word within the double-click time,
    /// at most one pixel from the last, sends 0x8000 and then, if that is
    /// answered 0 or 0x10, the plain 0x1000; otherwise the double click
    /// consumes the button. The result decides the hold and the drop
    /// source.
    pub fn press(&mut self, word: u16) {
        let word32 = u32::from(word);
        if self.pointer.buttons & 7 != 0 && self.pointer.capture.is_some() {
            self.pointer.held = true;
            self.release((self.pointer.buttons & 0xffff) as u16 | 0x800);
            self.pointer.held = false;
            return;
        }
        self.pointer.buttons = 0;
        if self.drop.kind != 0 {
            self.end_drop(if word & 2 == 0 { word32 } else { 0x800 });
            if let Some(capture) = self.pointer.capture.take() {
                self.select(capture, 0);
            }
            self.hit_and_focus(self.pointer.x, self.pointer.y);
            return;
        }
        self.hit_target();
        if word & 7 != 0 {
            self.pointer.click_x = self.pointer.x;
            self.pointer.click_y = self.pointer.y;
        }
        self.hide_tooltip();
        if let Some(capture) = self.pointer.capture {
            self.pointer.target = Some(capture);
        }
        if let Some(modal) = self.modal
            && self.pointer.target != Some(modal)
        {
            self.pointer.target = None;
            self.pointer.target_control = NO_CONTROL;
            if let Some(f) = self.form_mut(modal) {
                f.close(0);
            }
            self.modal = None;
            self.update(true, (self.pointer.x, self.pointer.y));
            return;
        }
        let near = |a: i32, b: i32| (i64::from(a) - i64::from(b)).abs() < 2;
        let result = if u64::from(self.clock.wrapping_sub(self.pointer.press_stamp))
            < u64::from(DOUBLE_CLICK)
            && word == self.pointer.last_word
            && near(self.pointer.x, self.pointer.press_x)
            && near(self.pointer.y, self.pointer.press_y)
        {
            let second = self.press_start(word32 | 0x8000);
            if second & !0x10 == 0 {
                self.press_start(word32 | 0x1000)
            } else {
                self.pointer.consumed = word & 7;
                second
            }
        } else {
            self.pointer.last_word = word;
            self.pointer.press_stamp = self.clock;
            self.press_start(word32 | 0x1000)
        };
        if result != 0x40 {
            if result & 0x100 != 0 {
                self.drop.source = self.pointer.target;
                self.drop.source_control = self.pointer.target_control;
                self.pointer.target = None;
            }
            self.pointer.buttons = word32;
            if result == 0 || result == 0x10 {
                self.pointer.held = false;
            }
            if self.pointer.buttons & 0x4000 == 0 {
                self.pointer.press_x = self.pointer.x;
                self.pointer.press_y = self.pointer.y;
            }
        }
    }

    /// A button release `word`. A button a double click consumed is
    /// swallowed. A release of the held buttons ends a drop; a press nobody
    /// held and that did not drag clicks (0x2000), clearing the target when
    /// answered; a held press goes to its target's release; a capture gets
    /// the release, keeps the capture on 0x40 and drops it on 0x80. The
    /// pointer then hit tests and focuses, falling back to the focus's own
    /// form and control. A release of other buttons does nothing.
    pub fn release(&mut self, word: u16) {
        if word & self.pointer.consumed != 0 {
            self.pointer.buttons = 0;
            self.pointer.consumed = 0;
            return;
        }
        if self.pointer.buttons & 7 != u32::from(word & 7) {
            return;
        }
        let word32 = u32::from(word);
        if self.drop.kind != 0 {
            self.end_drop(word32);
        }
        if !self.pointer.held {
            if self.pointer.buttons & 0x100 != 0x100
                && self.press_start((self.pointer.buttons & 0xffff) | 0x2000) != 0
            {
                self.pointer.target = None;
            }
        } else if let Some(capture) = self.pointer.capture.take() {
            let result = self.form_mut(capture).map_or(0, |f| f.release(word32));
            if result & 0x80 == 0x80 {
                if self.pointer.capture.is_none() {
                    self.set_capture(None);
                }
            } else if result & 0x40 == 0x40 {
                self.pointer.capture = Some(capture);
            }
            self.select(capture, 0);
            if result & 3 == 3 {
                self.redraw = true;
            }
        } else if let Some(target) = self.pointer.target {
            let result = self.form_mut(target).map_or(0, |f| f.release(word32));
            if result & 3 == 3 {
                self.redraw = true;
            }
        }
        self.pointer.buttons = 0;
        self.hit_target();
        if !self.focus_target() {
            self.focus_set(self.active, self.active_control);
        }
    }

    /// Ends a drop: a word with 0x800 cancels it (no target control and the
    /// cancelled flag); with a target the payload is dropped there, else its
    /// source hears of the cancel. The source gets its control back and
    /// deselects, and the drop clears.
    pub fn end_drop(&mut self, word: u32) {
        if word & 0x800 != 0 {
            self.drop.target_control = NO_CONTROL;
            self.drop.flags |= 0x10;
        }
        let kind = self.drop.kind;
        match self.drop.target.take() {
            Some(target) => {
                let control = self.drop.target_control;
                if let Some(f) = self.form_mut(target) {
                    f.drop(kind, control);
                }
                self.pointer.target_control = 0;
            }
            None => {
                if let Some(source) = self.drop.source
                    && let Some(f) = self.form_mut(source)
                {
                    f.drop_cancelled(kind);
                }
            }
        }
        if let Some(source) = self.drop.source.take() {
            let control = self.drop.source_control;
            self.select(source, control);
            self.select(source, 0);
        }
        self.drop = DropState::default();
        self.redraw = true;
    }

    /// Hands the capture to `form`: the previous holder hears a 0x200
    /// release. Dropping it re-reads the pointer and frees the system
    /// pointer; taking it marks the press point and captures the system
    /// pointer. The drag flag clears either way.
    pub fn set_capture(&mut self, form: Option<FormId>) {
        if let Some(previous) = self.pointer.capture
            && let Some(f) = self.form_mut(previous)
        {
            f.release(0x200);
        }
        self.pointer.capture = form;
        if form.is_none() {
            self.update(true, (self.pointer.x, self.pointer.y));
            self.pointer.os_captured = false;
        } else {
            self.pointer.press_x = self.pointer.x;
            self.pointer.press_y = self.pointer.y;
            self.pointer.os_captured = true;
        }
        self.pointer.buttons &= !0x100;
    }

    /// The forced idle step: the active control becomes [`RESELECT`] and the
    /// pointer is re-read.
    pub fn reselect(&mut self) {
        self.active_control = RESELECT;
        self.update(true, (self.pointer.x, self.pointer.y));
    }

    /// Clears the tooltip request (its stamp back to the far future).
    pub fn clear_tooltip(&mut self) {
        self.tooltip = TooltipRequest::default();
    }

    /// Hides the tooltip: the request's form and control clear (its stamp
    /// stays); a showing tooltip stops and fades out over 256 clock units.
    pub fn hide_tooltip(&mut self) {
        self.tooltip.form = None;
        self.tooltip.control = 0;
        if self.tooltip_showing {
            self.tooltip_showing = false;
            self.tooltip_fade_until = Some(self.clock.wrapping_add(256));
            self.redraw = true;
        }
    }

    /// The task's tick: a tooltip request whose delay has passed (an
    /// unsigned 64-bit comparison) shows its tooltip when the focus is
    /// still on it, and is then cleared; a showing tooltip hides once its
    /// time is past; flagged forms are deleted.
    pub fn tick(&mut self) -> TickResult {
        let mut out = TickResult::default();
        if let Some(form) = self.tooltip.form
            && u64::from(TOOLTIP_DELAY) + u64::from(self.tooltip.stamp) < u64::from(self.clock)
        {
            if self.active == Some(form) && self.active_control == self.tooltip.control {
                let record = self
                    .form(form)
                    .map_or(-1, |f| f.hint_record(self.tooltip.control));
                let class = if record < 0 {
                    0
                } else {
                    self.form(form).map_or(0, |f| f.tooltip_class(record))
                };
                out.show = Some(TooltipShow {
                    form,
                    record,
                    class,
                });
            }
            self.clear_tooltip();
        }
        if self.tooltip_showing && self.tooltip_hide_at < self.clock {
            self.hide_tooltip();
            out.hide = true;
        }
        out.deleted = self.delete_doomed();
        out
    }

    /// Deletes the forms marked for deletion; returns how many.
    pub fn delete_doomed(&mut self) -> usize {
        let mut count = 0;
        for slot in self.forms.iter_mut() {
            if slot.as_ref().is_some_and(|form| form.doomed()) {
                *slot = None;
                count += 1;
            }
        }
        count
    }

    /// The shutdown: every top-level form not marked for deletion is told
    /// to close, then the marked forms are deleted.
    pub fn shutdown(&mut self) {
        let root = self.root;
        for slot in self.forms.iter_mut().flatten() {
            if slot.parent() == root && root.is_some() && !slot.doomed() {
                slot.close(0);
            }
        }
        self.delete_doomed();
    }

    /// Whether `form` is the modal form.
    pub fn is_modal(&self, form: FormId) -> bool {
        self.modal == Some(form)
    }
}

/// What a hit test found and focused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HitResult {
    pub form: Option<FormId>,
    pub control: i32,
    pub cursor: i32,
    pub result: u32,
}

/// What a tick asks the caller to do with the tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TickResult {
    pub show: Option<TooltipShow>,
    pub hide: bool,
    pub deleted: usize,
}

/// Makes `form` the modal form (the original only stores it).
pub fn set_modal(task: &mut GuiTask, form: Option<FormId>) {
    task.modal = form;
}

/// The registered tooltip classes: each registration returns a 1-based
/// index, which a record's 0x8000 message answers to pick its tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TooltipClasses {
    pub count: i32,
}

impl TooltipClasses {
    pub fn register(&mut self) -> i32 {
        self.count += 1;
        self.count
    }

    /// Whether an index names a registered class (0 or less is the default
    /// tooltip).
    pub fn get(&self, index: i32) -> Option<i32> {
        (index >= 1 && index <= self.count).then_some(index)
    }
}

/// The registered form factories: each registration appends a factory and
/// returns its 1-based index, which a tooltip record's hint message answers
/// to build its tooltip.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FormFactories {
    pub count: i32,
}

impl FormFactories {
    /// Appends a factory, returning its 1-based index.
    pub fn register(&mut self) -> i32 {
        self.count += 1;
        self.count
    }

    /// The index of a registered factory (0 or less is none).
    pub fn get(&self, index: i32) -> Option<i32> {
        (index >= 1 && index <= self.count).then_some(index)
    }

    /// Clears every registration (the task's shutdown frees the array).
    pub fn clear(&mut self) {
        self.count = 0;
    }
}

/// A message box's construction: its layout, controls and flags.
#[derive(Debug, Clone, PartialEq)]
pub struct MessageBoxPlan {
    pub layout: MessageBoxLayout,
    /// The buttons' controls, in create order (Ok, then Cancel).
    pub controls: Vec<Control>,
    /// The label's control.
    pub label: Control,
    /// The bits the box adds to its form's style word.
    pub style: u32,
    /// The state word's bit 0x1000, and the close bit 2 left set unless
    /// the box asks for no close button.
    pub keeps_close: bool,
    pub modal: bool,
    pub top_quarter: bool,
}

/// Message-box flag: placed a quarter down the screen.
pub const MESSAGE_HIGH: u32 = message_box::HIGH;
/// Message-box flag: the close button's state bit is cleared.
pub const MESSAGE_NO_CLOSE: u32 = 0x20;
/// Message-box flag: the wide frame.
pub const MESSAGE_FRAME: u32 = 0x40;
/// Message-box flag: the box becomes the modal form.
pub const MESSAGE_MODAL: u32 = 0x80;

/// The result a message box's button stores: Ok 1, Cancel 0.
pub fn message_result(ok: bool) -> i32 {
    i32::from(ok)
}

/// Builds a message box from its text's pixel width and the original's
/// button flags (see [`crate::gui_rescale::message_box`]).
pub fn message_box_plan(
    text: &[u8],
    text_width: i32,
    buttons: u32,
    screen_height: i32,
    unit: i32,
    border: i32,
) -> MessageBoxPlan {
    let layout = message_box_layout(text_width, buttons, screen_height, border, unit);
    let mut controls = Vec::new();
    if buttons & message_box::OK != 0 {
        let mut ok = Control::new(1, 0, 0, 80, 0);
        ok.text = b"Ok".to_vec();
        controls.push(ok);
    }
    if buttons & message_box::CANCEL != 0 {
        let mut cancel = Control::new(1, 0, 0, 80, 0);
        cancel.text = b"Cancel".to_vec();
        controls.push(cancel);
    }
    for (index, control) in controls.iter_mut().enumerate() {
        control.y = 13;
        control.x = layout.buttons.get(index).map_or(0, |(_, x, _, _)| *x);
    }
    let mut label = Control::new(5, 12, 4, layout.label.2, 0);
    label.text = text.to_vec();
    label.style = 0x0003_3300;
    MessageBoxPlan {
        layout,
        controls,
        label,
        style: 0x808
            | if buttons & MESSAGE_FRAME != 0 {
                0x3000
            } else {
                0
            },
        keeps_close: buttons & MESSAGE_NO_CLOSE == 0,
        modal: buttons & MESSAGE_MODAL != 0,
        top_quarter: buttons & MESSAGE_HIGH != 0,
    }
}

/// A form's anchor for a placement request: the original's default is 4
/// pixels on each axis, a centre is the parent's spare half.
pub fn anchored(parent: i32, size: i32, axis_y: bool, anchor: Anchor) -> i32 {
    crate::form::anchor(None, parent, size, axis_y, anchor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn rect(x: i32, y: i32, w: i32, h: i32) -> GuiRect {
        GuiRect::new(x, y, w, h)
    }

    #[test]
    fn rectangles_hold_their_interior_and_exclude_far_edges() {
        let r = rect(10, 20, 100, 50);
        assert!(r.contains(10, 20));
        assert!(r.contains(109, 69));
        assert!(!r.contains(110, 69));
        assert!(!r.contains(10, 70));
        assert_eq!((r.width(), r.height()), (100, 50));
    }

    #[test]
    fn the_drag_threshold_shrinks_from_the_press() {
        assert_eq!(drag_threshold(0, 10), 100);
        assert_eq!(drag_threshold(1, 10), 90);
        assert_eq!(drag_threshold(20, 10), 90);
        assert_eq!(drag_threshold(181, 10), 0);
        assert_eq!(drag_threshold(190, 10), 0);
        assert_eq!(drag_threshold(191, 10), 10);
        // Before the press (a clock that went back) the division still
        // truncates: (200 + 5) / 20 is 10.
        assert_eq!(drag_threshold(-5, 3), 30);
        // One pixel per 540 of height, at least 2.
        assert_eq!(drag_limit(1080), 2);
        assert_eq!(drag_limit(2160), 4);
        assert_eq!(drag_limit(1890), 4);
        assert_eq!(drag_limit(600), 2);
    }

    #[test]
    fn a_child_is_placed_on_the_screen() {
        let screen = (800, 600);
        let request = PlaceRequest {
            desired_x: 10,
            fallback_x: 10,
            desired_y: 20,
            fallback_bottom: 590,
        };
        assert_eq!(place_form(screen, 100, 80, request), (10, 20));
        let tight = PlaceRequest {
            desired_x: 780,
            fallback_x: 700,
            desired_y: 550,
            fallback_bottom: 590,
        };
        assert_eq!(place_form(screen, 100, 80, tight), (700, 510));
        let none = PlaceRequest {
            desired_x: 900,
            fallback_x: -1,
            desired_y: 20,
            fallback_bottom: 590,
        };
        assert_eq!(place_form(screen, 100, 80, none), (698, 20));
        // A fitting desired x with a negative fallback is the left edge.
        let left = PlaceRequest {
            desired_x: 10,
            fallback_x: -1,
            desired_y: 20,
            fallback_bottom: 590,
        };
        assert_eq!(place_form(screen, 100, 80, left).0, 0);
        // Exactly at the edge still fits.
        let edge = PlaceRequest {
            desired_x: 700,
            fallback_x: 0,
            desired_y: 520,
            fallback_bottom: 590,
        };
        assert_eq!(place_form(screen, 100, 80, edge), (700, 520));
    }

    #[test]
    fn the_grips_and_title_controls_come_from_the_lengths() {
        let form = EdgeForm {
            width: 200,
            height: 150,
            border: 3,
            top: 24,
            inner_right: 120,
            style: 0,
        };
        let m = GuiMetrics::at(4);
        // Controls are 17 wide from 200 − 17 − 4 = 179, from 3 to 21.
        assert_eq!(edge_rect(form, m, -4), Some([179, 3, 196, 21]));
        assert_eq!(edge_rect(form, m, -5).unwrap()[0], 179);
        let both = EdgeForm { style: 10, ..form };
        assert_eq!(edge_rect(both, m, -5).unwrap()[0], 179 - 38);
        let one = EdgeForm { style: 8, ..form };
        assert_eq!(edge_rect(one, m, -5).unwrap()[0], 179 - 19);
        let restore = EdgeForm { style: 2, ..form };
        assert_eq!(edge_rect(restore, m, -6).unwrap()[0], 179 - 19);
        // Grips are 15 squares inside the border.
        assert_eq!(edge_rect(form, m, -10), Some([182, 132, 197, 147]));
        assert_eq!(edge_rect(form, m, -13), Some([3, 132, 18, 147]));
        assert_eq!(edge_rect(form, m, -14), Some([105, 132, 120, 147]));
        assert_eq!(edge_rect(form, m, -9), Some([182, 24, 197, 39]));
        assert_eq!(edge_rect(form, m, -7), None);
        // At unit 5 the controls are 21 wide and step by 23.
        let m = GuiMetrics::at(5);
        assert_eq!(edge_rect(both, m, -5).unwrap()[0], 200 - 21 - 5 - 46);
    }

    #[test]
    fn metrics_follow_the_unit_and_the_level() {
        let m = GuiMetrics::at(5);
        assert_eq!(
            (m.title, m.control, m.control_step, m.grip),
            (28, 21, 23, 19)
        );
        assert_eq!(m.quarter.to_bits(), 1.25f32.to_bits());
        assert_eq!(GuiMetrics::for_level(0).unit, 3);
        assert_eq!(GuiMetrics::for_level(4), GuiMetrics::at(8));
        // Level 5's lengths are unit 8's; only the unit is 12.
        let five = GuiMetrics::for_level(5);
        assert_eq!(five.unit, 12);
        assert_eq!((five.title, five.control, five.grip), (46, 33, 31));
        assert_eq!(five.quarter, 2.0);
        assert_eq!(GuiMetrics::for_level(9), five);
    }

    #[test]
    fn a_form_is_clamped_to_the_screen() {
        assert_eq!(clamp_form(-5, 1, 100, 4, 800), (0, 4));
        assert_eq!(clamp_form(750, 20, 100, 4, 800), (700, 20));
        assert_eq!(clamp_form(700, 20, 100, 4, 800), (700, 20));
        assert_eq!(clamp_form(10, 20, 2, 4, 800), (10, 20));
    }

    #[test]
    fn switching_a_toggle_on_turns_its_group_off() {
        let mut controls = vec![
            Control::new(2, 0, 0, 16, 16),
            Control::new(2, 0, 0, 16, 16),
            Control::new(2, 0, 0, 16, 16),
        ];
        for control in controls.iter_mut().take(2) {
            control.group = 1;
            control.reports = true;
        }
        controls[2].group = 2;
        controls[2].state = control_state::ON;
        controls[0].state = control_state::ON | control_state::LIT;
        let mut events = Vec::new();
        set_control_on(&mut controls, 1, true, false, &mut events);
        assert_eq!(
            controls[0].state & (control_state::ON | control_state::LIT),
            0
        );
        assert_eq!(
            controls[1].state & (control_state::ON | control_state::LIT),
            control_state::ON | control_state::LIT
        );
        // Another group is left alone.
        assert_eq!(controls[2].state, control_state::ON);
        assert_eq!(
            events,
            vec![ControlEvent::Changed(0), ControlEvent::Changed(1)]
        );
        // Switching an on control on again clears and reports it first.
        events.clear();
        set_control_on(&mut controls, 1, true, false, &mut events);
        assert_eq!(
            events,
            vec![ControlEvent::Changed(1), ControlEvent::Changed(1)]
        );
        // Quietly off: the lit bit follows, nothing is reported.
        events.clear();
        set_control_on(&mut controls, 1, false, true, &mut events);
        assert_eq!(
            controls[1].state & (control_state::ON | control_state::LIT),
            0
        );
        assert!(events.is_empty());
    }

    /// What a test form was sent.
    #[derive(Default)]
    struct Log {
        selects: Vec<i32>,
        presses: Vec<u32>,
        releases: Vec<u32>,
        moves: usize,
        closes: usize,
    }

    /// A form that records the calls it gets.
    struct Recorder {
        parent: Option<FormId>,
        bounds: GuiRect,
        selected: i32,
        /// What a select answers, and what a press answers.
        select_result: u32,
        press_result: u32,
        release_result: u32,
        move_result: u32,
        log: Arc<Mutex<Log>>,
    }

    impl Recorder {
        fn new(bounds: GuiRect, log: &Arc<Mutex<Log>>) -> Self {
            Self {
                parent: None,
                bounds,
                selected: 0,
                select_result: 3,
                press_result: 0x13,
                release_result: 3,
                move_result: 0,
                log: Arc::clone(log),
            }
        }
    }

    impl FormHost for Recorder {
        fn parent(&self) -> Option<FormId> {
            self.parent
        }
        fn bounds(&self) -> GuiRect {
            self.bounds
        }
        fn hit_test(&self, x: i32, y: i32) -> i32 {
            if self.bounds.contains(x, y) { 1 } else { 0 }
        }
        fn hit_cursor(&self, _code: i32) -> i32 {
            2
        }
        fn select(&mut self, control: i32) -> u32 {
            self.selected = control;
            self.log.lock().unwrap().selects.push(control);
            self.select_result
        }
        fn selected_control(&self) -> i32 {
            self.selected
        }
        fn press(&mut self, word: u32) -> u32 {
            self.log.lock().unwrap().presses.push(word);
            self.press_result
        }
        fn release(&mut self, word: u32) -> u32 {
            self.log.lock().unwrap().releases.push(word);
            self.release_result
        }
        fn drag_move(&mut self, _x: i32, _y: i32) -> u32 {
            self.log.lock().unwrap().moves += 1;
            self.move_result
        }
        fn close(&mut self, _arg: i32) {
            self.log.lock().unwrap().closes += 1;
        }
    }

    fn task_with(recorder: impl FnOnce(&mut Recorder)) -> (GuiTask, FormId, Arc<Mutex<Log>>) {
        let mut task = GuiTask::new((800, 600));
        let log = Arc::new(Mutex::new(Log::default()));
        let mut form = Recorder::new(rect(0, 0, 800, 600), &log);
        recorder(&mut form);
        let id = task.add(Box::new(form));
        (task, id, log)
    }

    #[test]
    fn the_focus_selects_against_the_forms_own_selection() {
        let (mut task, id, log) = task_with(|_| {});
        assert_eq!(task.focus_set(Some(id), 1), 3);
        assert_eq!((task.active, task.active_control), (Some(id), 1));
        // The form already has control 1: no select, and nothing new.
        assert_eq!(task.focus_set(Some(id), 1), 0);
        // A control the form has selected is stored only when it reselects:
        // with the form already on 2, the stored control stays 1.
        task.form_mut(id).unwrap().select(2);
        assert_eq!(task.focus_set(Some(id), 2), 0);
        assert_eq!(task.active_control, 1);
        assert_eq!(log.lock().unwrap().selects, vec![1, 2]);
        // The control -1 always selects when the form is elsewhere.
        task.focus_set(Some(id), -1);
        assert_eq!(task.active_control, -1);
    }

    #[test]
    fn a_tooltip_is_requested_by_a_hinted_select_and_shown_after_400() {
        let (mut task, id, _log) = task_with(|f| f.select_result = 0x103);
        task.clock = 1000;
        task.focus_set(Some(id), 1);
        assert_eq!(
            task.tooltip,
            TooltipRequest {
                form: Some(id),
                control: 1,
                stamp: 1000
            }
        );
        task.clock = 1400;
        assert_eq!(task.tick().show, None);
        assert_eq!(task.tooltip.form, Some(id));
        task.clock = 1401;
        let shown = task.tick().show.unwrap();
        assert_eq!((shown.form, shown.record, shown.class), (id, -1, 0));
        assert_eq!(task.tooltip, TooltipRequest::default());
        // A tooltip already showing back-dates the next request by 400.
        task.tooltip_visible = true;
        task.form_mut(id).unwrap().select(0);
        task.focus_set(Some(id), 2);
        assert_eq!(task.tooltip.stamp, 1001);
        // A showing tooltip hides once its time is past, then fades for
        // 256.
        task.tooltip_showing = true;
        task.tooltip_hide_at = 1401;
        assert!(!task.tick().hide);
        task.tooltip_hide_at = 1400;
        assert!(task.tick().hide);
        assert!(!task.tooltip_showing);
        assert_eq!(task.tooltip_fade_until, Some(1657));
        // A press hides it too, keeping the request's stamp.
        task.tooltip_showing = true;
        task.update(false, (5, 5));
        let stamp = task.tooltip.stamp;
        task.press(1);
        assert!(!task.tooltip_showing);
        assert_eq!(task.tooltip.stamp, stamp);
    }

    #[test]
    fn a_moved_focus_drops_the_tooltip_it_requested() {
        let (mut task, id, log) = task_with(|f| f.select_result = 0x103);
        let other = task.add(Box::new(Recorder::new(rect(0, 0, 10, 10), &log)));
        task.focus_set(Some(id), 1);
        task.focus_set(Some(other), 1);
        task.focus_set(Some(id), 3);
        task.tooltip.form = Some(other);
        task.tooltip.control = 1;
        task.clock = 5000;
        assert_eq!(task.tick().show, None);
        assert_eq!(task.tooltip.form, None);
    }

    #[test]
    fn a_modal_form_takes_the_hit_and_blocks_the_others() {
        let (mut task, root, log) = task_with(|_| {});
        let modal = task.add(Box::new(Recorder::new(rect(100, 100, 200, 100), &log)));
        set_modal(&mut task, Some(modal));
        assert_eq!(task.hit(150, 150), (Some(modal), 1, 2));
        assert_eq!(task.hit(700, 500), (Some(modal), 0, 2));
        // Focusing another form while modal clears the focus.
        task.focus_set(Some(root), 1);
        assert_eq!((task.active, task.active_control), (None, NO_CONTROL));
        // The modal form is always the pointer's target, so only a press
        // aimed elsewhere (here by a capture) closes it.
        task.update(false, (700, 500));
        assert_eq!(task.pointer.target, Some(modal));
        task.pointer.capture = Some(root);
        task.press(1);
        assert_eq!(task.modal, None);
        assert_eq!(log.lock().unwrap().closes, 1);
    }

    #[test]
    fn off_the_screen_nothing_is_hit() {
        let (mut task, _id, _log) = task_with(|_| {});
        assert_eq!(task.hit_root(-1, 10), (None, 0, -1));
        assert_eq!(task.hit_root(800, 10), (None, 0, -1));
        // The hit test's cursor is -1: the system pointer hides there.
        let hit = task.hit_and_focus(10, 600);
        assert_eq!((hit.form, hit.control, hit.cursor), (None, 1, -1));
        assert_eq!(task.cursor, -1);
        // The pointer update's own hit test stores the no-target cursor 0.
        task.update(false, (10, 600));
        assert_eq!((task.pointer.target, task.cursor), (None, 0));
    }

    #[test]
    fn the_last_child_is_hit_first() {
        let (mut task, root, log) = task_with(|_| {});
        let mut below = Recorder::new(rect(10, 10, 100, 100), &log);
        below.parent = Some(root);
        let below = task.add(Box::new(below));
        let mut above = Recorder::new(rect(50, 50, 100, 100), &log);
        above.parent = Some(root);
        let above = task.add(Box::new(above));
        assert_eq!(task.hit_root(60, 60).0, Some(above));
        assert_eq!(task.hit_root(20, 20).0, Some(below));
        assert_eq!(task.hit_root(500, 500).0, Some(root));
        assert_eq!(task.root_of(above), above);
    }

    #[test]
    fn a_press_and_release_click_through_the_held_form() {
        let (mut task, id, log) = task_with(|_| {});
        task.update(false, (100, 100));
        assert_eq!(task.pointer.target, Some(id));
        task.clock = 10;
        task.press(1);
        // 0x13: the form holds the press; 0x10 keeps its control.
        assert!(task.pointer.held);
        assert_eq!(task.pointer.buttons, 1);
        assert_eq!((task.pointer.press_x, task.pointer.press_y), (100, 100));
        task.release(1);
        assert_eq!(task.pointer.buttons, 0);
        let log = log.lock().unwrap();
        assert_eq!(log.presses, vec![0x1001]);
        assert_eq!(log.releases, vec![1]);
    }

    #[test]
    fn an_unheld_press_clicks_on_the_release() {
        let (mut task, _id, log) = task_with(|f| f.press_result = 0);
        task.update(false, (100, 100));
        task.press(1);
        assert!(!task.pointer.held);
        task.release(1);
        assert_eq!(log.lock().unwrap().presses, vec![0x1001, 0x2001]);
        // A release of another button does nothing.
        task.press(1);
        task.release(2);
        assert_eq!(task.pointer.buttons, 1);
    }

    #[test]
    fn a_second_press_within_the_window_is_a_double_click() {
        let (mut task, _id, log) = task_with(|f| f.press_result = 0);
        task.update(false, (50, 50));
        task.clock = 1000;
        task.press(1);
        task.release(1);
        task.clock = 1449;
        task.press(1);
        // 0x8000 is answered 0, so the plain press follows.
        assert_eq!(
            log.lock().unwrap().presses,
            vec![0x1001, 0x2001, 0x8001, 0x1001]
        );
        task.release(1);
        // After the window it is a single press again.
        task.clock = 3000;
        task.press(1);
        assert_eq!(log.lock().unwrap().presses.last(), Some(&0x1001));
    }

    #[test]
    fn an_answered_double_click_consumes_its_release() {
        let (mut task, _id, log) = task_with(|f| f.press_result = 0x40);
        task.update(false, (50, 50));
        task.press(1);
        // A 0x40 answer leaves the press point where it was: the second
        // press is near only once the point is the pointer's.
        assert_eq!(task.pointer.press_x, 0);
        task.press(1);
        assert_eq!(task.pointer.consumed, 0);
        (task.pointer.press_x, task.pointer.press_y) = (50, 50);
        task.press(1);
        assert_eq!(task.pointer.consumed, 1);
        task.release(1);
        assert_eq!(task.pointer.consumed, 0);
        assert!(log.lock().unwrap().releases.is_empty());
    }

    #[test]
    fn a_drag_waits_out_the_press_threshold() {
        let (mut task, _id, log) = task_with(|f| f.press_result = 0);
        task.pointer.limit = 2;
        task.update(false, (100, 100));
        task.clock = 0;
        task.press(1);
        // Right after the press the threshold is (200 / 20) · 2 = 20.
        task.update(false, (120, 100));
        assert_eq!(task.pointer.buttons & 0x100, 0);
        task.update(false, (121, 100));
        assert_eq!(task.pointer.buttons & 0x100, 0x100);
        // The drag start's word carries the held word with its new 0x100.
        assert_eq!(log.lock().unwrap().presses, vec![0x1001, 0x4101]);
    }

    #[test]
    fn a_captured_form_gets_the_moves_until_it_lets_go() {
        let (mut task, id, log) = task_with(|f| {
            f.press_result = 0x15;
            f.move_result = 0;
        });
        task.update(false, (10, 10));
        task.press(1);
        assert_eq!(task.pointer.capture, Some(id));
        assert!(task.pointer.os_captured);
        task.update(false, (20, 20));
        assert_eq!(log.lock().unwrap().moves, 1);
        task.release(1);
        // The release answered 3: the capture is gone.
        assert_eq!(task.pointer.capture, None);
    }

    #[test]
    fn the_reselect_step_forces_a_select() {
        let (mut task, id, log) = task_with(|_| {});
        task.update(false, (10, 10));
        assert_eq!(task.active_control, 1);
        task.form_mut(id).unwrap().select(0);
        task.reselect();
        assert_eq!(task.active_control, 1);
        assert_eq!(log.lock().unwrap().selects, vec![1, 0, 1]);
    }

    #[test]
    fn the_message_box_plan_follows_the_layout() {
        let plan = message_box_plan(
            b"A question",
            100,
            message_box::OK | message_box::CANCEL,
            768,
            4,
            16,
        );
        assert_eq!(plan.layout.size, (352, 104));
        assert_eq!(plan.layout.label, (12, 4, 296));
        assert_eq!(plan.controls.len(), 2);
        assert_eq!(
            (plan.controls[0].x, plan.controls[0].y, plan.controls[0].w),
            (72, 13, 80)
        );
        assert_eq!(plan.controls[1].text, b"Cancel".to_vec());
        assert_eq!((plan.controls[1].x, plan.controls[1].y), (168, 13));
        assert_eq!(plan.label.x, 12);
        assert!(plan.keeps_close);
        assert!(!plan.modal);
        assert_eq!((message_result(true), message_result(false)), (1, 0));
    }

    #[test]
    fn the_modal_flag_and_quarter_top_come_from_the_buttons() {
        let plan = message_box_plan(
            b"Go?",
            100,
            message_box::OK | MESSAGE_HIGH | MESSAGE_MODAL | MESSAGE_NO_CLOSE,
            768,
            5,
            2,
        );
        assert!(plan.modal && plan.top_quarter && !plan.keeps_close);
        assert_eq!(plan.controls.len(), 1);
        assert_eq!(plan.layout.y, (768 - 104) / 4 / 5 * 4);
    }

    #[test]
    fn tooltip_classes_count_from_one() {
        let mut classes = TooltipClasses::default();
        assert_eq!(classes.register(), 1);
        assert_eq!(classes.register(), 2);
        assert_eq!(classes.get(2), Some(2));
        assert_eq!(classes.get(0), None);
        assert_eq!(classes.get(3), None);
    }

    #[test]
    fn showing_a_control_clears_its_group_and_follows_the_shown_bit() {
        let mut controls = vec![
            Control::new(2, 0, 0, 16, 16),
            Control::new(2, 0, 0, 16, 16),
            Control::new(2, 0, 0, 16, 16),
        ];
        for control in controls.iter_mut() {
            control.group = 1;
            control.reports = true;
        }
        controls[0].state = control_state::ON | control_state::LIT;
        controls[2].group = 2;
        controls[2].state = control_state::ON | control_state::LIT;
        let mut events = Vec::new();
        let shown = set_control_visible(&mut controls, 1, true, false, &mut events);
        // The previous member of the group hides; the other group stays.
        assert_eq!(shown.hidden, 1);
        assert_eq!(
            controls[0].state & (control_state::ON | control_state::LIT),
            0
        );
        assert_eq!(
            controls[1].state & (control_state::ON | control_state::LIT),
            control_state::ON | control_state::LIT
        );
        assert_eq!(
            controls[2].state & (control_state::ON | control_state::LIT),
            control_state::ON | control_state::LIT
        );
        assert_eq!(
            events,
            vec![ControlEvent::Changed(0), ControlEvent::Changed(1)]
        );
        // The returned byte is the new shown bit, with the lit bit masked
        // off: 0x20 and the restored low bits, not 0x60.
        assert_eq!(shown.state_byte, 0x20);
        // A quiet show reports nothing but still hides.
        events.clear();
        let quiet = set_control_visible(&mut controls, 0, true, true, &mut events);
        assert_eq!(quiet.hidden, 1);
        assert!(events.is_empty());
        // Hiding clears the shown bit; the lit bit follows it, so it
        // clears too.
        let hidden = set_control_visible(&mut controls, 0, false, false, &mut events);
        assert_eq!(
            controls[0].state & (control_state::ON | control_state::LIT),
            0
        );
        assert_eq!(hidden.state_byte, 0);
        assert_eq!(events, vec![ControlEvent::Changed(0)]);
        // A control of no group hides nobody.
        controls[2].group = 0;
        assert_eq!(
            set_control_visible(&mut controls, 2, true, false, &mut events).hidden,
            0
        );
    }

    #[test]
    fn a_message_box_button_stores_its_result_and_closes_the_parent() {
        assert_eq!(message_box_button(true, false), MESSAGE_OK);
        assert_eq!(message_box_button(false, false), MESSAGE_CANCEL);
        assert_eq!(message_box_button(true, true), MESSAGE_CLOSE);
        assert_eq!(MESSAGE_OK.result, 1);
        assert_eq!(MESSAGE_CANCEL.result, 0);
        let cols = [MESSAGE_OK, MESSAGE_CANCEL];
        assert_eq!(cols.map(|m| m.closes), [true, true]);
        let close = MESSAGE_CLOSE;
        assert_eq!((close.closes, close.destroys), (false, true));
    }

    #[test]
    fn form_factories_count_from_one_until_cleared() {
        let mut factories = FormFactories::default();
        assert_eq!(factories.register(), 1);
        assert_eq!(factories.register(), 2);
        assert_eq!(factories.get(2), Some(2));
        assert_eq!(factories.get(0), None);
        assert_eq!(factories.get(3), None);
        factories.clear();
        assert_eq!(factories.get(1), None);
        assert_eq!(factories.register(), 1);
    }

    #[test]
    fn a_double_click_uses_an_unsigned_clock_difference() {
        let (mut task, _id, log) = task_with(|f| f.press_result = 0);
        task.update(false, (50, 50));
        task.clock = 400;
        task.press(1);
        task.release(1);
        // A clock wrap makes the difference huge as unsigned, so this is a
        // single press even though it looks negative as a signed number.
        task.clock = 100;
        task.press(1);
        assert_eq!(log.lock().unwrap().presses, vec![0x1001, 0x2001, 0x1001]);
        // Just inside the window it is a double click again.
        task.release(1);
        task.clock = 401;
        task.press(1);
        assert_eq!(
            log.lock().unwrap().presses,
            vec![0x1001, 0x2001, 0x1001, 0x2001, 0x8001, 0x1001]
        );
    }

    #[test]
    fn doomed_forms_are_deleted_and_the_rest_close_on_shutdown() {
        struct Doomed;
        impl FormHost for Doomed {
            fn parent(&self) -> Option<FormId> {
                Some(1)
            }
            fn bounds(&self) -> GuiRect {
                GuiRect::default()
            }
            fn doomed(&self) -> bool {
                true
            }
        }
        let (mut task, root, log) = task_with(|_| {});
        let mut child = Recorder::new(rect(0, 0, 10, 10), &log);
        child.parent = Some(root);
        let child = task.add(Box::new(child));
        let doomed = task.add(Box::new(Doomed));
        assert_eq!(task.tick().deleted, 1);
        assert!(task.form(doomed).is_none());
        task.shutdown();
        assert_eq!(log.lock().unwrap().closes, 1);
        assert!(task.form(child).is_some());
    }
}

//! The GUI's window manager (`RF_GUI_Main`): the form tree and its z-order,
//! the active and modal forms, focus changes and their fallback, the
//! pointer's press, drag and release state machine, hit testing, control
//! message routing, the tooltip and the system cursor, and the message
//! box's construction.
//!
//! A form is anything implementing [`FormHost`]: the manager only needs its
//! parent, bounds, hit test and the callbacks the original's virtual methods
//! provide. [`PlainForm`] is the basic record (bounds, controls, the control
//! message dispatch of [`crate::gui_controls`]) used by the running app.
//! Rules: `docs/UI.md` (Forms, Controls).

use crate::form::Anchor;
use crate::gui_controls::{Control, ControlEvent, Message, dispatch, state as control_state};
use crate::gui_rescale::{MessageBoxLayout, message_box, message_box_layout};

/// A form's identity in a [`GuiTask`].
pub type FormId = u32;

/// The rectangle of a form or a grip, in root pixels.
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

    pub fn intersects(self, other: Self) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }
}

/// The form callbacks the window manager uses. A form's own class supplies
/// these; the manager never draws.
pub trait FormHost: Send + Sync {
    /// The parent in the form tree, if any.
    fn parent(&self) -> Option<FormId>;
    /// The form's absolute rectangle.
    fn bounds(&self) -> GuiRect;
    /// Modal forms take every pointer event (the original's message boxes).
    fn is_modal(&self) -> bool {
        false
    }
    fn is_visible(&self) -> bool {
        true
    }
    /// The control or grip code under a root point: a positive 1-based
    /// control index, a negative grip code, or -1 for none.
    fn hit_test(&self, _x: i32, _y: i32) -> i32 {
        -1
    }
    /// The cursor shape a hit code asks for.
    fn hit_cursor(&self, _code: i32) -> i32 {
        0
    }
    /// Changes the selected control; the original's focus-changed result
    /// bits (3: redraw; 0x100: a focus was granted).
    fn focus_changed(&mut self, _control: i32) -> u32 {
        0
    }
    fn selected_control(&self) -> i32 {
        -1
    }
    /// A key word while the form is focused.
    fn key(&mut self, _word: u32) -> u32 {
        0
    }
    /// The focus-key event (the form's focus-key callback).
    fn key_focus(&mut self, _form: Option<FormId>, _control: i32) -> u32 {
        0
    }
    /// A pointer message (the form's pointer callback).
    fn pointer(&mut self, _message: u32, _param: u32, _button: u16) -> u32 {
        0
    }
    /// A drag message: the move and end callbacks.
    fn drag(&mut self, _message: u32, _x: i32, _y: i32) -> u32 {
        0
    }
    /// A per-frame tick.
    fn tick(&mut self, _now: i32) {}
    /// The result of a closing message box, when the form is one.
    fn message_result(&self) -> Option<i32> {
        None
    }
}

/// A basic form: an absolute rectangle, a parent, its controls and the
/// control message dispatch.
#[derive(Debug, Clone, PartialEq)]
pub struct PlainForm {
    pub id: FormId,
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub modal: bool,
    pub visible: bool,
    pub controls: Vec<Control>,
    /// The selected control, 1-based; -1 for none.
    pub selected: i32,
    /// A message box's chosen button, once closed.
    pub result: Option<i32>,
    pub events: Vec<ControlEvent>,
}

impl PlainForm {
    pub fn new(id: FormId, bounds: GuiRect) -> Self {
        Self {
            id,
            parent: None,
            bounds,
            modal: false,
            visible: true,
            controls: Vec::new(),
            selected: -1,
            result: None,
            events: Vec::new(),
        }
    }

    /// The 1-based control whose rectangle holds the point.
    pub fn control_at(&self, x: i32, y: i32) -> i32 {
        for (index, control) in self.controls.iter().enumerate() {
            if GuiRect::new(control.x, control.y, control.w, control.h).contains(x, y) {
                return index as i32 + 1;
            }
        }
        -1
    }

    fn selected_index(&self) -> Option<usize> {
        (self.selected >= 1).then(|| self.selected as usize - 1)
    }
}

impl FormHost for PlainForm {
    fn parent(&self) -> Option<FormId> {
        self.parent
    }

    fn bounds(&self) -> GuiRect {
        self.bounds
    }

    fn is_modal(&self) -> bool {
        self.modal
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn hit_test(&self, x: i32, y: i32) -> i32 {
        if self.bounds.contains(x, y) {
            self.control_at(x, y)
        } else {
            -1
        }
    }

    fn focus_changed(&mut self, control: i32) -> u32 {
        self.selected = control;
        3
    }

    fn selected_control(&self) -> i32 {
        self.selected
    }

    fn pointer(&mut self, message: u32, param: u32, button: u16) -> u32 {
        let Some(index) = self.selected_index() else {
            return 0;
        };
        let over = true;
        dispatch(
            &mut self.controls,
            index,
            Message {
                message,
                param,
                extra: u32::from(button),
                over,
                axes: (0.0, 0.0),
            },
            4,
            &mut self.events,
        )
    }

    fn key(&mut self, word: u32) -> u32 {
        let Some(index) = self.selected_index() else {
            return 0;
        };
        dispatch(
            &mut self.controls,
            index,
            Message {
                message: crate::gui_controls::message::KEY,
                param: word,
                ..Default::default()
            },
            4,
            &mut self.events,
        )
    }

    fn message_result(&self) -> Option<i32> {
        self.result
    }
}

/// The GUI's derived metrics at a scale level, as its level switch sets
/// them. The title-bar controls, the scroll rail, the tab and the grip are
/// all built from these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GuiMetrics {
    pub unit: i32,
    /// `6·unit − 2`.
    pub title_height: i32,
    /// `4·unit + 1`.
    pub rail: i32,
    /// `4·unit − 1`.
    pub grip: i32,
    /// `(unit >> 1) + rail`.
    pub tab: i32,
    /// The window-control strip's width.
    pub control_width: i32,
    /// The unit divided by four, as a float (the font's slant step).
    pub quarter: f32,
}

impl GuiMetrics {
    pub fn at(unit: i32) -> Self {
        let rail = unit * 4 + 1;
        Self {
            unit,
            title_height: unit * 6 - 2,
            rail,
            grip: unit * 4 - 1,
            tab: (unit >> 1) + rail,
            control_width: 17,
            quarter: unit as f32 * 0.25,
        }
    }
}

/// The rectangle of a title-bar control or a grip code, as the original's
/// edge helper returns it: corners `[left, top, right, bottom]`, or `None`
/// for a code it does not know.
pub fn edge_rect(form: GuiRect, style: u32, unit: i32, code: i32) -> Option<[i32; 4]> {
    let metrics = GuiMetrics::at(unit);
    let left = form.left;
    let top = form.top;
    let right = form.right;
    let bottom = form.bottom;
    let rect = match code {
        -14 => {
            let x = right - metrics.grip;
            let y = (bottom - left) - metrics.grip;
            [x, y, x + metrics.grip, y + metrics.grip]
        }
        -13 => {
            let x = left;
            let y = (bottom - top) - metrics.grip;
            [x, y, x + metrics.grip, y + metrics.grip]
        }
        -10 => {
            let x = (right - left) - metrics.grip;
            let y = (bottom - top) - metrics.grip;
            [x, y, x + metrics.grip, y + metrics.grip]
        }
        -9 => {
            let x = (right - left) - metrics.grip;
            let y = top;
            [x, y, x + metrics.grip, y + metrics.grip]
        }
        -6 => {
            let x = if style & 2 != 0 {
                ((right - metrics.control_width) - unit) - ((unit >> 1) + metrics.control_width)
            } else {
                (right - metrics.control_width) - unit
            };
            [
                x,
                unit - 1,
                x + metrics.control_width,
                (metrics.title_height - unit) + 3,
            ]
        }
        -5 => {
            let x = if style & 10 == 10 {
                ((right - metrics.control_width) - unit) - ((unit >> 1) + metrics.control_width) * 2
            } else if style & 10 == 0 {
                (right - metrics.control_width) - unit
            } else {
                ((right - metrics.control_width) - unit) - ((unit >> 1) + metrics.control_width)
            };
            [
                x,
                unit - 1,
                x + metrics.control_width,
                (metrics.title_height - unit) + 3,
            ]
        }
        -4 => {
            let x = (right - metrics.control_width) - unit;
            [
                x,
                unit - 1,
                x + metrics.control_width,
                (metrics.title_height - unit) + 3,
            ]
        }
        _ => return None,
    };
    Some(rect)
}

/// Places a child form in its parent: `request`'s desired x when it and a
/// minimum fit, else the parent's right edge less two; the desired y when
/// it fits, else the bottom limit less the form's height.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaceRequest {
    /// The x the caller would like; negative means "left edge".
    pub desired_x: i32,
    /// A minimum x accepted while the preferred one does not fit.
    pub min_x: i32,
    /// The bottom limit.
    pub bottom: i32,
    /// The y the caller would like.
    pub desired_y: i32,
}

/// Places a `width` × `height` form inside `parent_client`.
pub fn place_form(
    parent_client: GuiRect,
    width: i32,
    height: i32,
    request: PlaceRequest,
) -> (i32, i32) {
    let x = if parent_client.right < width + request.desired_x {
        if request.min_x < 0 || parent_client.right < width + request.min_x {
            (parent_client.right - width) - 2
        } else {
            request.min_x
        }
    } else if request.min_x < 0 {
        0
    } else {
        request.desired_x
    };
    let y = if parent_client.bottom < parent_client.top + request.desired_y + height {
        request.bottom - height
    } else {
        request.desired_y
    };
    (x, y)
}

/// The pointer's state: its position, the buttons, where and when it
/// pressed, its hover target, and the drag in progress.
#[derive(Debug, Clone, PartialEq)]
pub struct PointerState {
    pub x: i32,
    pub y: i32,
    /// The OS's button word (bit 1 left, bit 2 right).
    pub buttons: u16,
    /// The button that started the press.
    pub pressed: u16,
    pub press_x: i32,
    pub press_y: i32,
    pub press_stamp: i32,
    /// The button word a double click consumes.
    pub consumed: u16,
    /// The form and control the pointer is over.
    pub target: Option<(FormId, i32)>,
    /// The form being dragged, its offset and the button word.
    pub drag: Option<DragState>,
    /// A drag was begun with the 0x100 flag (the button may be down).
    pub dragging: bool,
    /// Hover reporting is suppressed after a drag started.
    pub suppress_hover: bool,
    /// The drag threshold's base distance.
    pub drag_limit: i32,
    /// The last press for the double-click test.
    pub last_press: Option<(u16, i32, i32, i32)>,
    pub hover_changed: bool,
}

/// A drag in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DragState {
    pub form: FormId,
    pub offset_x: i32,
    pub offset_y: i32,
    pub button: u16,
}

impl Default for PointerState {
    fn default() -> Self {
        Self {
            x: 0,
            y: 0,
            buttons: 0,
            pressed: 0,
            press_x: 0,
            press_y: 0,
            press_stamp: 0,
            consumed: 0,
            target: None,
            drag: None,
            dragging: false,
            suppress_hover: false,
            drag_limit: 10,
            last_press: None,
            hover_changed: false,
        }
    }
}

/// The pointer's drag threshold: the base distance until a quarter second
/// of the press is over, shrinking by a twentieth of the remaining time
/// each twentieth of a second.
pub fn drag_threshold(elapsed: i32, limit: i32) -> i32 {
    if elapsed < 0xbf {
        ((200 - elapsed) / 20) * limit
    } else {
        limit
    }
}

/// The tooltip: a hint waiting for its delay, its position and its text.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tooltip {
    pub request: Option<(FormId, i32)>,
    pub hint: Option<Vec<u8>>,
    pub shown_at: Option<i32>,
    pub position: (i32, i32),
}

/// The original waits a quarter second (a guess: the tooltip's own unit is
/// outside this group) before showing a hint.
pub const TOOLTIP_DELAY: i32 = 450;

impl Tooltip {
    /// A hint asked for at `now`, or `None` to clear it.
    pub fn set(&mut self, hint: Option<Vec<u8>>, request: (FormId, i32), now: i32, at: (i32, i32)) {
        if self.hint != hint || self.request != Some(request) {
            self.hint = hint;
            self.request = Some(request);
            self.shown_at = None;
            self.position = at;
            if self.hint.is_none() {
                self.request = None;
            }
        }
        self.update(now);
    }

    /// Shows the pending hint once its delay is over.
    pub fn update(&mut self, now: i32) {
        if self.hint.is_none() {
            self.shown_at = None;
            return;
        }
        if self.shown_at.is_none() {
            self.shown_at = Some(now);
        }
    }

    /// Whether the tooltip is drawn: after the delay.
    pub fn visible(&self, now: i32) -> bool {
        self.hint.is_some()
            && self
                .shown_at
                .is_some_and(|stamp| stamp.wrapping_add(TOOLTIP_DELAY) <= now)
    }

    pub fn clear(&mut self) {
        self.request = None;
        self.hint = None;
        self.shown_at = None;
    }
}

/// Shows or hides a control: hiding clears its shown bit; showing clears
/// every other control of the same class's shown and lit bits (reporting
/// each, unless `no_callback`), then sets this one's shown and lit bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ControlVisibility {
    pub shown: bool,
    /// The other controls that were hidden.
    pub hidden: usize,
}

/// Sets control `index`'s shown state as the original's visibility helper.
pub fn set_control_visible(
    controls: &mut [Control],
    index: usize,
    show: bool,
    no_callback: bool,
    events: &mut Vec<ControlEvent>,
) -> ControlVisibility {
    let Some(control) = controls.get(index) else {
        return ControlVisibility::default();
    };
    let class = control.class;
    let group = control.group;
    if !show {
        controls[index].state &= !control_state::SHOWN;
        return ControlVisibility::default();
    }
    let mut hidden = 0;
    for (other, control) in controls.iter_mut().enumerate() {
        if other != index
            && control.class == class
            && control.state & control_state::SHOWN != 0
            && (group == 0 || control.group == group || control.group == 0)
        {
            control.state &= !(control_state::SHOWN | control_state::LIT);
            if !no_callback && control.reports {
                events.push(ControlEvent::Changed(other));
            }
            hidden += 1;
        }
    }
    controls[index].state |= control_state::SHOWN;
    let lit = (controls[index].state & control_state::ON) << 1;
    controls[index].state = (controls[index].state & !control_state::LIT) | lit;
    ControlVisibility {
        shown: true,
        hidden,
    }
}

/// Clamps a form's place and size to the screen: `x` to 0, the width so
/// the right edge fits (less two), and `y` to at least a unit.
pub fn clamp_form(x: i32, y: i32, width: i32, unit: i32, screen_width: i32) -> (i32, i32, i32) {
    let x = if x < 0 {
        0
    } else if screen_width < x + width {
        (screen_width - width) - 2
    } else {
        x
    };
    let width = width.max(unit);
    let y = y.max(unit);
    (x, y, width)
}

/// The pointer grab record: which form held it and whether the system
/// pointer is captured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PointerGrab {
    pub form: Option<FormId>,
    pub captured: bool,
}

impl PointerGrab {
    /// Grabs for `form`, releasing the previous holder through its callback.
    /// Returns the previous holder.
    pub fn grab(&mut self, form: Option<FormId>, offset: (i32, i32)) -> Option<FormId> {
        let previous = self.form;
        self.form = form;
        self.captured = form.is_some();
        let _ = offset;
        previous
    }
}

/// The system cursor: its shape, captures and visibility. The world cursor
/// is drawn by [`crate::cursor`]; this is the system pointer's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CursorState {
    pub shape: i32,
    pub captures: u32,
}

impl CursorState {
    /// Sets the shape, reporting whether it changed (the original only
    /// tells the OS when it differs).
    pub fn set_shape(&mut self, shape: i32) -> bool {
        if self.shape == shape {
            return false;
        }
        self.shape = shape;
        true
    }

    /// Takes the pointer capture; the system pointer is hidden meanwhile.
    pub fn capture(&mut self) {
        self.captures = self.captures.saturating_add(1);
    }

    pub fn release(&mut self) {
        self.captures = self.captures.saturating_sub(1);
    }

    pub fn visible(&self) -> bool {
        self.captures == 0
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

/// The window manager.
pub struct GuiTask {
    forms: Vec<Box<dyn FormHost>>,
    next_id: FormId,
    /// The top-level form (the original's 800 × 600 root).
    pub root: Option<FormId>,
    /// The form with the keyboard focus.
    pub active: Option<FormId>,
    /// Its selected control, 1-based (-1 none).
    pub active_control: i32,
    /// The modal form, which takes every pointer event.
    pub modal: Option<FormId>,
    /// The focus to restore when a modal closes.
    pub fallback: Option<(FormId, i32)>,
    /// When the fallback stops being used, on the original's clock.
    pub fallback_until: i64,
    /// Focus changes are checked against the input device this frame.
    pub suppress_focus: bool,
    pub screen: (i32, i32),
    pub pointer: PointerState,
    pub tooltip: Tooltip,
    pub cursor: CursorState,
    /// The drag that started before a press was forwarded.
    pub press_target: Option<(FormId, i32)>,
    /// A redraw is wanted (the original's result bits 3).
    pub redraw: u32,
    /// The drag target before a press began, to release it later.
    pub pressed_drag: Option<FormId>,
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
            next_id: 1,
            root: None,
            active: None,
            active_control: -1,
            modal: None,
            fallback: None,
            fallback_until: 0,
            suppress_focus: false,
            screen,
            pointer: PointerState::default(),
            tooltip: Tooltip::default(),
            cursor: CursorState::default(),
            press_target: None,
            redraw: 0,
            pressed_drag: None,
        }
    }

    /// Adds a form and returns its id; the first form becomes the root.
    pub fn add(&mut self, mut form: Box<dyn FormHost>) -> FormId {
        let id = self.next_id;
        self.next_id += 1;
        let _ = form.focus_changed(-1);
        self.forms.push(form);
        if self.root.is_none() {
            self.root = Some(id);
        }
        id
    }

    /// The form with `id`.
    pub fn form(&self, id: FormId) -> Option<&dyn FormHost> {
        self.forms.get((id - 1) as usize).map(|f| f.as_ref())
    }

    pub fn form_mut(&mut self, id: FormId) -> Option<&mut (dyn FormHost + '_)> {
        match self.forms.get_mut((id - 1) as usize) {
            Some(form) => Some(form.as_mut()),
            None => None,
        }
    }

    /// The form's root: its ancestor whose parent is none.
    pub fn root_of(&self, id: FormId) -> FormId {
        let mut current = id;
        let mut guard = 0;
        while let Some(parent) = self.form(current).and_then(FormHost::parent) {
            if parent == current || guard > self.forms.len() {
                break;
            }
            current = parent;
            guard += 1;
        }
        current
    }

    /// The hit test across the tree, modal first: the form and its control
    /// code under a point.
    pub fn hit_test(&self, x: i32, y: i32) -> (Option<FormId>, i32) {
        if let Some(modal) = self.modal {
            let code = self.form(modal).map_or(-1, |f| f.hit_test(x, y));
            return (Some(modal), code);
        }
        let Some(root) = self.root else {
            return (None, -1);
        };
        let form = self.hit_in(root, x, y).or(Some(root));
        let code = form
            .and_then(|id| self.form(id))
            .map_or(-1, |f| f.hit_test(x, y));
        (form, code)
    }

    /// The deepest visible form under a point, starting at `form`. The last
    /// child in the list is drawn on top and is tested first.
    fn hit_in(&self, form: FormId, x: i32, y: i32) -> Option<FormId> {
        let candidates: Vec<FormId> = self
            .forms
            .iter()
            .enumerate()
            .filter(|(_, f)| f.parent() == Some(form) && f.is_visible())
            .map(|(i, _)| i as FormId + 1)
            .collect();
        for child in candidates.into_iter().rev() {
            if let Some(found) = self.hit_in(child, x, y) {
                return Some(found);
            }
            if self.form(child).is_some_and(|f| f.bounds().contains(x, y)) {
                return Some(child);
            }
        }
        None
    }

    /// The form and control under a point, with the cursor shape.
    pub fn hit_cursor(&self, x: i32, y: i32) -> (Option<FormId>, i32, i32) {
        let (form, code) = self.hit_test(x, y);
        let cursor = form
            .and_then(|id| self.form(id))
            .map_or(0, |f| f.hit_cursor(code));
        (form, code, cursor)
    }

    /// The focus change (the original's `set focus`): the current form and
    /// control are left, the new one selected, and the original's result
    /// bits returned (1: something changed, 3: redraw, 0x100: a focus was
    /// granted, 0x200: it was not keyboard-driven).
    pub fn focus_set(&mut self, form: Option<FormId>, control: i32) -> u32 {
        let allowed = self.modal.is_none() || form.is_none() || form == self.modal;
        if !allowed {
            self.active = None;
            self.active_control = -1;
            return 0;
        }
        let mut changed = false;
        let mut result = 0;
        if form != self.active {
            if let Some(previous) = self.active {
                let granted = if !self.suppress_focus {
                    self.check_focus_grant(previous)
                } else {
                    0
                };
                if let Some(active) = self.form_mut(previous) {
                    result |= active.focus_changed(0);
                }
                result |= granted;
                result |= 1;
            }
            self.active_control = -1;
            changed = true;
            self.active = form;
        }
        if let Some(active) = self.active {
            let mut granted = 0;
            if control != self.active_control {
                if let Some(f) = self.form_mut(active) {
                    let r = f.focus_changed(control);
                    result |= r;
                    result |= 1;
                }
                self.active_control = control;
                changed = true;
                if !self.suppress_focus {
                    granted = self.check_focus_grant(active);
                }
            }
            result |= granted;
            if control == -1 {
                // The original's focus-key event.
                if let Some(f) = self.form_mut(active) {
                    result |= f.key_focus(Some(active), -1);
                }
            }
            if changed && result & 0x100 == 0x100 {
                self.fallback = Some((active, self.active_control));
                if !self.suppress_focus || result & 0x200 != 0 {
                    self.fallback_until = 0;
                } else {
                    self.fallback_until = -400;
                }
            }
        }
        if result & 3 == 3 {
            self.redraw |= 3;
        }
        result
    }

    /// Whether the newly focused control grants keyboard focus (the
    /// original asks the form to check its control's flags). Basic forms
    /// grant it.
    fn check_focus_grant(&mut self, form: FormId) -> u32 {
        if self.form(form).is_some() { 0x100 } else { 0 }
    }

    /// Restores the remembered fallback focus.
    pub fn restore_fallback(&mut self) -> u32 {
        let Some((form, control)) = self.fallback.take() else {
            return 0;
        };
        self.focus_set(Some(form), control)
    }

    /// Whether the remembered fallback is still usable at `now`.
    pub fn fallback_valid(&self, now: i64) -> bool {
        self.fallback.is_some() && self.fallback_until < now
    }

    /// Translates a point into the modal form's coordinates. Forms are kept
    /// in root coordinates here, so the point is unchanged.
    pub fn to_form(&self, _form: FormId, x: i32, y: i32) -> (i32, i32) {
        (x, y)
    }

    /// The pointer's movement (the original's update): the hover or drag
    /// target, the drag threshold and the delayed click.
    pub fn pointer_move(&mut self, now: i32, x: i32, y: i32, buttons: u16) -> bool {
        if self.pointer.x == x && self.pointer.y == y && self.pointer.buttons == buttons {
            return false;
        }
        self.pointer.x = x;
        self.pointer.y = y;
        self.pointer.buttons = buttons;
        let mut redraw = false;
        let mut start_drag = false;
        if buttons & 7 == 0 {
            self.pointer.dragging = false;
        } else if buttons & 0x6100 == 0 {
            let elapsed = now.wrapping_sub(self.pointer.press_stamp);
            let threshold = drag_threshold(elapsed, self.pointer.drag_limit);
            let dx = (x - self.pointer.press_x).abs();
            let dy = (y - self.pointer.press_y).abs();
            if dx as i64 > i64::from(threshold) || dy as i64 > i64::from(threshold) {
                self.pointer.dragging = true;
                start_drag = true;
            }
        }
        if self.pointer.drag.is_some() {
            let drag = self.pointer.drag.expect("checked");
            let moved = self.form_mut(drag.form).map_or(0, |f| f.drag(0x148, x, y));
            if moved & 0x80 == 0x80 {
                self.end_drag(now, 0);
            }
        } else if buttons & 7 == 0 {
            let (form, code, cursor) = self.hit_cursor(x, y);
            self.cursor.set_shape(cursor);
            redraw |= self.hover(form, code);
            if self.focus_set(form, code) & 3 == 3 {
                redraw = true;
            }
            self.tooltip_hint(now, form, code);
        } else if start_drag
            && !self.pointer.suppress_hover
            && let Some((form, code)) = self.pointer.target
        {
            self.pointer.dragging = true;
            let result = self.drag_start(form, code, 0x4000 | u32::from(buttons & 7), x, y);
            if result & 0x100 != 0 {
                self.press_target = Some((form, code));
                self.pointer.target = None;
            }
        }
        if redraw {
            self.redraw |= 3;
        }
        self.pointer.hover_changed = redraw;
        redraw
    }

    /// Sends the hover message and reports whether it changed anything.
    fn hover(&mut self, form: Option<FormId>, code: i32) -> bool {
        let previous = self.pointer.target;
        let next = form.filter(|_| code >= 0).map(|f| (f, code));
        self.pointer.target = next;
        if let Some((id, control)) = previous
            && let Some(f) = self.form_mut(id)
            && f.selected_control() == control
        {
            let r = f.pointer(crate::gui_controls::message::HOVER, 0, 1);
            if r & 3 == 3 {
                return true;
            }
        }
        if let Some((id, control)) = next
            && let Some(f) = self.form_mut(id)
        {
            let r = f.pointer(crate::gui_controls::message::HOVER, control as u32, 1);
            if r & 3 == 3 {
                return true;
            }
        }
        previous != next
    }

    /// Asks the form for the hint at a control and arms the tooltip.
    fn tooltip_hint(&mut self, now: i32, form: Option<FormId>, code: i32) {
        let Some((form, code)) = form.filter(|_| code >= 0).map(|f| (f, code)) else {
            self.tooltip.clear();
            return;
        };
        let hint = self
            .form_mut(form)
            .map_or(0, |f| f.pointer(crate::gui_controls::message::HINT, 0, 1));
        if hint & 0x100 != 0 {
            self.tooltip.set(
                Some(Vec::new()),
                (form, code),
                now,
                (self.pointer.x, self.pointer.y),
            );
        } else {
            self.tooltip.clear();
        }
    }

    /// The press: forwards the button to the hovered form, or starts a
    /// double click when the second press comes within 450 units and two
    /// pixels of the first.
    pub fn pointer_press(&mut self, now: i32, button: u16) -> u32 {
        if self.pointer.buttons & 7 != 0
            && let Some((form, _)) = self.pointer.target
        {
            self.pointer.suppress_hover = true;
            self.pressed_drag = Some(form);
            let buttons = self.pointer.buttons;
            let param = u32::from(buttons) | 0x800;
            if let Some(f) = self.form_mut(form) {
                f.pointer(0x50, param, buttons);
            }
            self.pointer.suppress_hover = false;
            return 0;
        }
        self.pointer.buttons = 0;
        let (form, code, cursor) = self.hit_cursor(self.pointer.x, self.pointer.y);
        self.cursor.set_shape(cursor);
        self.pointer.press_x = self.pointer.x;
        self.pointer.press_y = self.pointer.y;
        self.tooltip.clear();
        if let Some(modal) = self.modal
            && form != Some(modal)
        {
            self.active = None;
            self.active_control = -1;
            if let Some(f) = self.form_mut(modal) {
                f.key_focus(None, 0);
            }
            self.modal = None;
            return 0;
        }
        let double = self.pointer.last_press.is_some_and(|(b, x, y, stamp)| {
            b == button
                && now.wrapping_sub(stamp) < 450
                && (self.pointer.x - x).abs() < 2
                && (self.pointer.y - y).abs() < 2
        });
        let Some(form) = form else {
            return 0;
        };
        let result = if double {
            let down = self.send_pointer(form, code, 0x40, u32::from(button) | 0x8000);
            if down & !0x10 == 0 {
                self.send_pointer(form, code, 0x40, u32::from(button) | 0x1000)
            } else {
                self.pointer.consumed = button & 7;
                down
            }
        } else {
            self.pointer.last_press = Some((button, self.pointer.x, self.pointer.y, now));
            self.send_pointer(form, code, 0x40, u32::from(button) | 0x1000)
        };
        if result != 0x40 {
            if result & 0x100 != 0 {
                self.press_target = Some((form, code));
            }
            self.pointer.buttons = button & 7;
            if result == 0 || result == 0x10 {
                self.pointer.suppress_hover = false;
            }
            if self.pointer.buttons & 0x4000 == 0 {
                self.pointer.press_x = self.pointer.x;
                self.pointer.press_y = self.pointer.y;
            }
        }
        result
    }

    /// Sends a pointer message to a form's control.
    fn send_pointer(&mut self, form: FormId, code: i32, message: u32, param: u32) -> u32 {
        if code < 0 {
            return 0;
        }
        let button = (param & 7) as u16;
        self.form_mut(form).map_or(0, |f| {
            if f.selected_control() != code {
                f.focus_changed(code);
            }
            f.pointer(message, param, button)
        })
    }

    /// The release: completes a press on the hovered form, ends a drag, or
    /// closes a modal.
    pub fn pointer_release(&mut self, _now: i32, button: u16) -> u32 {
        if self.pointer.consumed & button != 0 {
            self.pointer.buttons = 0;
            self.pointer.consumed = 0;
            return 0;
        }
        let result;
        if self.pointer.buttons & 7 == button & 7 {
            if let Some(modal) = self.modal {
                if self.pointer.target.is_some() {
                    let r = self.send_pointer(
                        modal,
                        self.pointer.target.unwrap().1,
                        0x50,
                        u32::from(button),
                    );
                    result = r;
                } else {
                    result = 0;
                }
            } else if !self.pointer.suppress_hover {
                let (form, code) = self.hit_test(self.pointer.x, self.pointer.y);
                result = match form {
                    Some(form) => self.send_pointer(form, code, 0x50, u32::from(button) | 0x2000),
                    None => 0,
                };
            } else if self.pointer.drag.is_none() {
                let (form, code) = self.hit_test(self.pointer.x, self.pointer.y);
                result = match form {
                    Some(form) => self.send_pointer(form, code, 0x50, u32::from(button) | 0x140),
                    None => 0,
                };
            } else {
                self.end_drag(_now, button);
                result = 0;
            }
            self.pointer.buttons = 0;
            let (form, code, cursor) = self.hit_cursor(self.pointer.x, self.pointer.y);
            self.cursor.set_shape(cursor);
            self.focus_set(form, code);
            self.tooltip.clear();
            if form.is_none() {
                self.pointer.target = None;
            }
        } else {
            self.pointer.buttons = 0;
            self.pointer.consumed = 0;
            result = 0;
        }
        result
    }

    /// Begins a drag at the form's control: the form's drag start callback
    /// and the pressed target.
    fn drag_start(&mut self, form: FormId, code: i32, param: u32, x: i32, y: i32) -> u32 {
        let result = self
            .form_mut(form)
            .map_or(0, |f| f.pointer(0x138, param, (param & 7) as u16));
        if result == 0 {
            if let Some(f) = self.form_mut(form) {
                f.focus_changed(if param & 0x10 != 0 { code } else { 0 });
            }
        } else if result & 0x40 != 0x40 {
            self.pointer.suppress_hover = true;
            if result & 0x10 != 0 {
                if let Some(f) = self.form_mut(form) {
                    f.focus_changed(code);
                }
            } else if let Some(f) = self.form_mut(form) {
                f.focus_changed(0);
            }
            if result & 5 == 5 {
                self.pointer.drag = Some(DragState {
                    form,
                    offset_x: x - self.form(form).map_or(0, |f| f.bounds().left),
                    offset_y: y - self.form(form).map_or(0, |f| f.bounds().top),
                    button: (param & 7) as u16,
                });
            } else {
                let active = self.active;
                let control = self.active_control;
                if let Some(active) = active
                    && let Some(f) = self.form_mut(active)
                {
                    f.focus_changed(control);
                }
            }
        }
        result
    }

    /// Ends the drag: the form's drag end callback and the release.
    pub fn end_drag(&mut self, _now: i32, button: u16) {
        if let Some(drag) = self.pointer.drag.take() {
            let (x, y) = (self.pointer.x, self.pointer.y);
            if let Some(f) = self.form_mut(drag.form) {
                f.drag(0xe8, x, y);
            }
            self.press_target = None;
            self.pointer.suppress_hover = false;
            self.pointer.buttons &= !button;
        }
    }

    /// Cancels the pointer's press and drag state (the original's cancel).
    pub fn cancel_pointer(&mut self) {
        self.pointer.drag = None;
        self.pointer.dragging = false;
        self.pointer.suppress_hover = false;
        self.pointer.buttons = 0;
        self.pointer.consumed = 0;
        self.press_target = None;
        self.pointer.target = None;
        self.tooltip.clear();
        self.redraw |= 3;
    }

    /// The pointer update's forced idle step: the cursor snaps to its
    /// neutral shape and the pointer is re-hit-tested.
    pub fn reset_pointer(&mut self, now: i32) {
        self.cursor.set_shape(-0x15);
        let (x, y, buttons) = (self.pointer.x, self.pointer.y, self.pointer.buttons);
        self.pointer.x = x.wrapping_sub(1);
        self.pointer_move(now, x, y, buttons);
    }

    /// Routes a key word to the active form.
    pub fn key(&mut self, word: u32) -> u32 {
        let Some(active) = self.active else {
            return 0;
        };
        let result = self.form_mut(active).map_or(0, |f| f.key(word));
        if result & 3 == 3 {
            self.redraw |= 3;
        }
        result
    }

    /// The task's tick: the pointer default, the modal and fallback focus,
    /// and the form ticks.
    pub fn tick(&mut self, now: i32) {
        if self.fallback_valid(i64::from(now)) {
            self.restore_fallback();
        }
        for form in self.forms.iter_mut() {
            form.tick(now);
        }
        self.tooltip.update(now);
    }

    /// A message box's button result: the parent's remembered result and
    /// the modal's close.
    pub fn close_modal(&mut self, result: i32) {
        if let Some(modal) = self.modal.take() {
            if let Some(f) = self.form_mut(modal) {
                let _ = f.key_focus(None, result);
            }
            self.active = self.fallback.map(|f| f.0);
            self.active_control = self.fallback.map_or(-1, |f| f.1);
        }
        self.redraw |= 3;
    }

    /// The tooltip's draw place: its top-left corner at the request point.
    pub fn tooltip_at(&self, now: i32) -> Option<(i32, i32)> {
        self.tooltip.visible(now).then_some(self.tooltip.position)
    }

    /// Whether `form` is the active modal form.
    pub fn is_modal(&self, form: FormId) -> bool {
        self.modal == Some(form)
    }

    /// The hit-and-focus step (the original's pointer re-hit test): the hit
    /// form and code, the cursor shape and the focus result.
    pub fn hit_and_focus(&mut self, x: i32, y: i32) -> HitResult {
        let (form, code, cursor) = self.hit_cursor(x, y);
        let result = self.focus_set(form, code);
        self.cursor.set_shape(cursor);
        HitResult {
            form,
            control: code,
            cursor,
            result,
        }
    }

    /// Forgets the remembered fallback focus.
    pub fn reset_fallback(&mut self) {
        self.fallback = None;
        self.fallback_until = 0;
    }

    /// The pointer's grab: the previous holder and the capture state.
    pub fn grab(&mut self, form: Option<FormId>) -> Option<FormId> {
        let previous = self.pressed_drag;
        self.pressed_drag = form;
        if previous.is_some() && form.is_none() {
            self.cursor.release();
        } else if previous.is_none() && form.is_some() {
            self.cursor.capture();
        }
        previous
    }
}

/// The form factory registry (the original's registered form constructors):
/// each registration returns a 1-based index.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FactoryRegistry {
    pub next: i32,
}

impl FactoryRegistry {
    pub fn register(&mut self) -> i32 {
        self.next += 1;
        self.next
    }

    /// The factory for an index, when it was registered.
    pub fn get(&self, index: i32) -> Option<i32> {
        (index >= 1 && index <= self.next).then_some(index)
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
    /// The form's style word.
    pub style: u32,
    /// The form's state word.
    pub state: u32,
    pub modal: bool,
    pub top_quarter: bool,
}

/// Message-box flag: placed a quarter down the screen.
pub const MESSAGE_HIGH: u32 = 0x10;
/// Message-box flag: the form keeps its plain edges.
pub const MESSAGE_NO_CLOSE: u32 = 0x20;
/// Message-box flag: the wide frame.
pub const MESSAGE_FRAME: u32 = 0x40;
/// Message-box flag: the box becomes the active modal form.
pub const MESSAGE_MODAL: u32 = 0x80;

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
        state: 0x1000
            | if buttons & MESSAGE_NO_CLOSE != 0 {
                0
            } else {
                2
            },
        modal: buttons & MESSAGE_MODAL != 0,
        top_quarter: buttons & MESSAGE_HIGH != 0,
    }
}

/// Makes `form` the modal form and focuses it.
pub fn set_modal(task: &mut GuiTask, form: FormId) {
    task.modal = Some(form);
    task.focus_set(Some(form), -1);
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
        assert!(r.intersects(rect(50, 50, 100, 100)));
        assert!(!r.intersects(rect(110, 0, 10, 10)));
    }

    #[test]
    fn the_drag_threshold_shrinks_from_the_press() {
        assert_eq!(drag_threshold(0, 10), 100);
        assert_eq!(drag_threshold(1, 10), 90);
        assert_eq!(drag_threshold(20, 10), 90);
        assert_eq!(drag_threshold(190, 10), 0);
        assert_eq!(drag_threshold(191, 10), 10);
        assert_eq!(drag_threshold(1000, 10), 10);
    }

    #[test]
    fn a_child_is_placed_inside_its_parent() {
        let parent = rect(0, 0, 800, 600);
        let request = PlaceRequest {
            desired_x: 10,
            min_x: 10,
            bottom: 590,
            desired_y: 20,
        };
        assert_eq!(place_form(parent, 100, 80, request), (10, 20));
        let tight = PlaceRequest {
            desired_x: 780,
            min_x: 700,
            bottom: 590,
            desired_y: 550,
        };
        assert_eq!(place_form(parent, 100, 80, tight), (700, 510));
        let left = PlaceRequest {
            desired_x: 900,
            min_x: -1,
            bottom: 590,
            desired_y: 20,
        };
        assert_eq!(place_form(parent, 100, 80, left), (698, 20));
        let left_edge = PlaceRequest {
            desired_x: 10,
            min_x: -1,
            bottom: 590,
            desired_y: 20,
        };
        assert_eq!(place_form(parent, 100, 80, left_edge).0, 0);
    }

    #[test]
    fn the_title_controls_come_from_the_units() {
        let form = rect(0, 0, 200, 150);
        let close = edge_rect(form, 0, 4, -4).unwrap();
        assert_eq!(close, [200 - 17 - 4, 3, 200 - 4, 21]);
        let max = edge_rect(form, 0, 4, -5).unwrap();
        assert_eq!(max[0], 179);
        let both = edge_rect(form, 10, 4, -5).unwrap();
        assert_eq!(both[0], 179 - 38);
        let restore = edge_rect(form, 2, 4, -6).unwrap();
        assert_eq!(restore[0], 179 - 19);
        assert_eq!(edge_rect(form, 0, 4, -7), None);
        let grip = edge_rect(form, 0, 4, -10).unwrap();
        assert_eq!(grip, [200 - 15, 150 - 15, 200, 150]);
    }

    #[test]
    fn metrics_follow_the_unit() {
        let m = GuiMetrics::at(5);
        assert_eq!(
            (m.title_height, m.rail, m.grip, m.tab, m.control_width),
            (28, 21, 19, 23, 17)
        );
        assert_eq!(m.quarter.to_bits(), 1.25f32.to_bits());
        let m = GuiMetrics::at(4);
        assert_eq!((m.title_height, m.rail, m.grip, m.tab), (22, 17, 15, 19));
    }

    #[test]
    fn the_tooltip_waits_its_delay() {
        let mut t = Tooltip::default();
        t.set(Some(b"Go".to_vec()), (1, 2), 100, (30, 40));
        assert!(!t.visible(100));
        assert!(!t.visible(549));
        assert!(t.visible(550));
        assert_eq!(t.position, (30, 40));
        t.clear();
        assert!(!t.visible(1000));
    }

    #[test]
    fn the_cursor_tracks_shape_and_capture() {
        let mut c = CursorState::default();
        assert!(c.set_shape(3));
        assert!(!c.set_shape(3));
        assert!(c.visible());
        c.capture();
        c.capture();
        assert!(!c.visible());
        c.release();
        assert!(!c.visible());
        c.release();
        assert!(c.visible());
        c.release();
        assert_eq!(c.captures, 0);
    }

    /// What a test form was sent.
    #[derive(Default)]
    struct Log {
        messages: Vec<(u32, u32, u16)>,
        keys: Vec<u32>,
        drags: Vec<u32>,
    }

    /// A form that records the messages it is sent.
    struct Recorder {
        parent: Option<FormId>,
        bounds: GuiRect,
        selected: i32,
        grants: bool,
        log: Arc<Mutex<Log>>,
    }

    impl FormHost for Recorder {
        fn parent(&self) -> Option<FormId> {
            self.parent
        }
        fn bounds(&self) -> GuiRect {
            self.bounds
        }
        fn hit_test(&self, x: i32, y: i32) -> i32 {
            if self.bounds.contains(x, y) { 1 } else { -1 }
        }
        fn hit_cursor(&self, _code: i32) -> i32 {
            2
        }
        fn focus_changed(&mut self, control: i32) -> u32 {
            self.selected = control;
            3 | if self.grants { 0x100 } else { 0 }
        }
        fn selected_control(&self) -> i32 {
            self.selected
        }
        fn key(&mut self, word: u32) -> u32 {
            self.log.lock().unwrap().keys.push(word);
            3
        }
        fn pointer(&mut self, message: u32, param: u32, button: u16) -> u32 {
            self.log
                .lock()
                .unwrap()
                .messages
                .push((message, param, button));
            if message == crate::gui_controls::message::PRESS {
                0x103
            } else {
                0
            }
        }
        fn drag(&mut self, message: u32, _x: i32, _y: i32) -> u32 {
            self.log.lock().unwrap().drags.push(message);
            0
        }
    }

    fn task_with_recorder() -> (GuiTask, FormId, Arc<Mutex<Log>>) {
        let mut task = GuiTask::new((800, 600));
        let log = Arc::new(Mutex::new(Log::default()));
        let form = Recorder {
            parent: None,
            bounds: rect(0, 0, 800, 600),
            selected: -1,
            grants: true,
            log: Arc::clone(&log),
        };
        let id = task.add(Box::new(form));
        (task, id, log)
    }

    #[test]
    fn focusing_routes_the_focus_and_remembers_the_grant() {
        let (mut task, id, _log) = task_with_recorder();
        task.suppress_focus = false;
        let r = task.focus_set(Some(id), 1);
        assert_eq!(r & 0x100, 0x100);
        assert_eq!(task.active, Some(id));
        assert_eq!(task.active_control, 1);
        assert_eq!(task.fallback, Some((id, 1)));
        // Leaving the control sends the form's focus change to 0.
        task.focus_set(Some(id), 2);
        let recorder = task.form(id).unwrap();
        assert_eq!(recorder.selected_control(), 2);
    }

    #[test]
    fn a_modal_form_takes_the_hit_and_blocks_the_others() {
        let (mut task, _root, log) = task_with_recorder();
        let modal = Recorder {
            parent: None,
            bounds: rect(100, 100, 200, 100),
            selected: -1,
            grants: true,
            log,
        };
        let modal = task.add(Box::new(modal));
        task.modal = Some(modal);
        assert_eq!(task.hit_test(150, 150), (Some(modal), 1));
        assert_eq!(task.hit_test(700, 500), (Some(modal), -1));
        // Focusing another form while modal is refused and clears focus.
        task.focus_set(task.root, 1);
        assert_eq!(task.active, None);
    }

    #[test]
    fn a_pointer_move_hovers_then_presses_and_releases() {
        let (mut task, id, log) = task_with_recorder();
        task.pointer_move(0, 100, 100, 0);
        assert_eq!(task.pointer.target, Some((id, 1)));
        let r = task.pointer_press(10, 1);
        assert_eq!(r, 0x103);
        assert_eq!(task.pointer.buttons, 1);
        task.pointer_release(20, 1);
        assert_eq!(task.pointer.buttons, 0);
        let log = log.lock().unwrap();
        let presses = log
            .messages
            .iter()
            .filter(|m| m.0 == crate::gui_controls::message::PRESS)
            .count();
        let releases = log
            .messages
            .iter()
            .filter(|m| m.0 == crate::gui_controls::message::RELEASE)
            .count();
        assert_eq!((presses, releases), (1, 1));
    }

    #[test]
    fn a_second_press_within_the_window_is_a_double_click() {
        let (mut task, _id, log) = task_with_recorder();
        task.pointer_move(0, 50, 50, 0);
        task.pointer_press(0, 1);
        task.pointer.buttons = 0;
        task.pointer_press(100, 1);
        let presses = |log: &Log| {
            log.messages
                .iter()
                .filter(|m| m.0 == crate::gui_controls::message::PRESS)
                .count()
        };
        assert_eq!(presses(&log.lock().unwrap()), 2);
        // A third press, after the window, is a single click again.
        task.pointer.buttons = 0;
        task.pointer_press(1000, 1);
        assert_eq!(presses(&log.lock().unwrap()), 3);
    }

    #[test]
    fn a_drag_waits_out_the_press_threshold() {
        let (mut task, _id, _log) = task_with_recorder();
        task.pointer_move(0, 100, 100, 0);
        task.pointer_press(0, 1);
        // Immediately a small move does not drag; a large one does.
        task.pointer_move(1, 105, 100, 1);
        assert!(!task.pointer.dragging);
        // After 200 units the threshold is the plain limit.
        task.pointer_move(200, 150, 100, 1);
        assert!(task.pointer.dragging);
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
        assert_eq!(plan.state, 0x1002);
        assert!(!plan.modal);
    }

    #[test]
    fn the_modal_flag_and_quarter_top_come_from_the_buttons() {
        let plan = message_box_plan(
            b"Go?",
            100,
            message_box::OK | MESSAGE_HIGH | MESSAGE_MODAL,
            768,
            5,
            2,
        );
        assert!(plan.modal && plan.top_quarter);
        assert_eq!(plan.controls.len(), 1);
        assert_eq!(plan.layout.y, (768 - 104) / 4 / 5 * 4);
    }

    #[test]
    fn a_key_reaches_the_active_form() {
        let (mut task, id, log) = task_with_recorder();
        task.focus_set(Some(id), 1);
        assert_eq!(task.key(0x2001), 3);
        assert_eq!(log.lock().unwrap().keys, vec![0x2001]);
    }

    #[test]
    fn showing_a_control_hides_its_classmates() {
        let mut controls = vec![Control::new(2, 0, 0, 16, 16), Control::new(2, 0, 0, 16, 16)];
        controls[0].state = control_state::SHOWN;
        controls[0].reports = true;
        controls[1].state = control_state::SHOWN | control_state::LIT;
        controls[1].reports = true;
        let mut events = Vec::new();
        let change = set_control_visible(&mut controls, 1, true, false, &mut events);
        assert_eq!(
            change,
            ControlVisibility {
                shown: true,
                hidden: 1
            }
        );
        assert_eq!(controls[0].state & control_state::SHOWN, 0);
        // The shown bit is also the on bit; the lit bit follows it.
        assert_eq!(
            controls[1].state & (control_state::SHOWN | control_state::LIT),
            control_state::SHOWN | control_state::LIT
        );
        assert_eq!(events.len(), 1);
        let change = set_control_visible(&mut controls, 1, false, true, &mut events);
        assert!(!change.shown);
        assert_eq!(controls[1].state & control_state::SHOWN, 0);
    }

    #[test]
    fn a_form_is_clamped_to_the_screen() {
        assert_eq!(clamp_form(-5, 1, 100, 4, 800), (0, 4, 100));
        assert_eq!(clamp_form(750, 20, 100, 4, 800), (698, 20, 100));
        assert_eq!(clamp_form(10, 20, 2, 4, 800), (10, 20, 4));
        assert_eq!(clamp_form(10, 20, 100, 4, 800), (10, 20, 100));
    }

    #[test]
    fn the_pointer_grab_records_its_holder() {
        let mut grab = PointerGrab::default();
        assert_eq!(grab.grab(Some(3), (10, 20)), None);
        assert!(grab.captured);
        assert_eq!(grab.grab(None, (0, 0)), Some(3));
        assert!(!grab.captured);
    }

    #[test]
    fn a_hit_and_focus_reports_the_form_control_and_cursor() {
        let (mut task, id, _log) = task_with_recorder();
        let hit = task.hit_and_focus(100, 100);
        assert_eq!((hit.form, hit.control, hit.cursor), (Some(id), 1, 2));
        assert_eq!(task.cursor.shape, 2);
        assert_eq!(task.active, Some(id));
        assert!(!task.is_modal(id));
    }

    #[test]
    fn factories_and_fallbacks_reset() {
        let mut factories = FactoryRegistry::default();
        assert_eq!(factories.register(), 1);
        assert_eq!(factories.register(), 2);
        assert_eq!(factories.get(2), Some(2));
        assert_eq!(factories.get(0), None);
        let (mut task, id, _log) = task_with_recorder();
        task.focus_set(Some(id), 1);
        assert!(task.fallback.is_some());
        task.reset_fallback();
        assert!(task.fallback.is_none());
    }
}

//! The pointer, buttons and wheel of one frame fed to a window manager
//! whose forms are base forms ([`exa_ui::base_form::BaseForm`]), and the
//! default tooltip it shows.
//!
//! A frame's pointer moves the task's pointer ([`GuiTask::update`]: the
//! hover and the focus follow it, a held form gets the drag), the buttons'
//! presses and releases go to [`GuiTask::press`] and [`GuiTask::release`]
//! (the original's words: left 1, right 2, middle 4; a press adds Ctrl
//! 0x40, Shift 0x20 and Alt 0x80, so a Ctrl press on a title does not drag;
//! the middle button only while the player's controls are off), the wheel to
//! the focused form ([`GuiTask::wheel`], 120 a notch). On the frames the
//! GUI timer comes due ([`crate::frame_timers::FrameTimers`], every 100 ms)
//! every base form runs its repeat and the task's tick shows a requested
//! tooltip once its 400-unit delay has passed, hides one past its time and
//! deletes closed forms; a press that starts an arrow's repeat pushes the
//! GUI timer 700 ms out, as the original's form does. The clock is the
//! frame's real time in milliseconds.
//!
//! The tooltip ([`ShownTooltip`]) is the default one: the record's text in
//! the GUI level's main font, its box sized and placed by
//! [`exa_ui::tooltips::layout`] on the pointer, fading in over 256 units,
//! up for 2500 and fading out ([`exa_ui::tooltips::ToolTipState`]), drawn
//! on the skin's rounded fill (alpha 153 at full strength).

use bevy::input::mouse::{MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use exa_ui::base_form::{BaseEvent, BaseForm};
use exa_ui::gui_core::{FormId, GuiRect, GuiTask, PlainForm, TickResult};
use exa_ui::gui_draw::{GuiLayer, LayerTexture};
use exa_ui::skin::SkinBatch;
use exa_ui::tooltips::{ToolTipState, draw as tooltip_draw, layout as tooltip_layout};

/// One frame's pointer input, in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PointerFrame {
    pub at: Option<(i32, i32)>,
    /// The left button's press and release this frame.
    pub pressed: bool,
    pub released: bool,
    /// The right button's press and release (the original's word 2).
    pub right_pressed: bool,
    pub right_released: bool,
    /// The middle button's press and release (word 4), already gated by
    /// [`read_pointer`]: the GUI hears it only while the player's controls
    /// are off.
    pub middle_pressed: bool,
    pub middle_released: bool,
    /// The press words' modifier bits: Ctrl 0x40, Shift 0x20, Alt 0x80.
    pub modifiers: u16,
    /// The wheel's turn this frame, 120 a notch (up positive).
    pub wheel: i32,
    pub now: u32,
}

/// The modifier bits a button press carries, as the window procedure
/// builds them: Ctrl 0x40, Shift 0x20 and Alt 0x80.
pub fn press_modifiers(ctrl: bool, shift: bool, alt: bool) -> u16 {
    u16::from(ctrl) << 6 | u16::from(shift) << 5 | u16::from(alt) << 7
}

/// The modifiers held now, as [`press_modifiers`] words them.
pub fn held_modifiers(keys: Option<&ButtonInput<KeyCode>>) -> u16 {
    let Some(keys) = keys else {
        return 0;
    };
    press_modifiers(
        keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]),
        keys.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]),
        keys.any_pressed([KeyCode::AltLeft, KeyCode::AltRight]),
    )
}

/// The wheel's messages summed into the original's units (120 a notch).
pub fn wheel_delta(wheel: &mut MessageReader<MouseWheel>) -> i32 {
    wheel
        .read()
        .map(|w| match w.unit {
            MouseScrollUnit::Line => w.y * 120.0,
            MouseScrollUnit::Pixel => w.y,
        })
        .sum::<f32>() as i32
}

/// The frame's pointer: the scripted one when set, else the window's, in
/// physical pixels.
pub fn read_pointer(
    window: &Window,
    scripted: Option<&exa_ui::ScriptedPointer>,
    mouse: &ButtonInput<MouseButton>,
    wheel: i32,
    now: u32,
) -> PointerFrame {
    let at = match scripted {
        Some(scripted) => scripted.position(window),
        None => window.cursor_position(),
    }
    .map(|p| {
        let p = p * window.scale_factor();
        (p.x as i32, p.y as i32)
    });
    PointerFrame {
        at,
        pressed: mouse.just_pressed(MouseButton::Left),
        released: mouse.just_released(MouseButton::Left),
        right_pressed: mouse.just_pressed(MouseButton::Right),
        right_released: mouse.just_released(MouseButton::Right),
        middle_pressed: false,
        middle_released: false,
        modifiers: 0,
        wheel,
        now,
    }
}

impl PointerFrame {
    /// Adds the keyboard's modifiers and the middle button: the window
    /// procedure sends the middle button's press and release to the GUI
    /// only while the player's controls are off (`gui_middle`); with the
    /// controls on, the controls' mouse hook takes that button.
    pub fn with_keys(
        mut self,
        mouse: &ButtonInput<MouseButton>,
        keys: Option<&ButtonInput<KeyCode>>,
        gui_middle: bool,
    ) -> Self {
        self.modifiers = held_modifiers(keys);
        if gui_middle {
            self.middle_pressed = mouse.just_pressed(MouseButton::Middle);
            self.middle_released = mouse.just_released(MouseButton::Middle);
        }
        self
    }
}

/// A window manager of base forms, with its root (the screen-sized task
/// form the original's forms hang from).
pub struct FormTask {
    pub task: GuiTask,
    pub root: FormId,
}

impl std::fmt::Debug for FormTask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FormTask")
            .field("root", &self.root)
            .field("active", &self.task.active)
            .finish()
    }
}

impl Default for FormTask {
    fn default() -> Self {
        Self::new()
    }
}

impl FormTask {
    pub fn new() -> Self {
        let mut task = GuiTask::new((1 << 20, 1 << 20));
        let root = task.add(Box::new(PlainForm::new(
            GuiRect::new(0, 0, 1 << 20, 1 << 20),
            4,
        )));
        Self { task, root }
    }

    /// Adds a base form under the root (or under `parent`) and runs its
    /// show's snap rebuilds ([`GuiTask::show_snaps`]): a child shown in a
    /// form cuts that form's snap slots around it.
    pub fn add(&mut self, mut form: BaseForm, parent: Option<FormId>) -> FormId {
        form.parent = Some(parent.unwrap_or(self.root));
        let id = self.task.add(Box::new(form));
        self.task.show_snaps(id);
        id
    }

    pub fn base(&self, id: FormId) -> Option<&BaseForm> {
        self.task.form(id).and_then(|f| f.base())
    }

    pub fn base_mut(&mut self, id: FormId) -> Option<&mut BaseForm> {
        self.task.form_mut(id).and_then(|f| f.base_mut())
    }

    /// Takes the events a form's pointer handling raised.
    pub fn take_events(&mut self, id: FormId) -> Vec<BaseEvent> {
        self.base_mut(id)
            .map(|f| std::mem::take(&mut f.events))
            .unwrap_or_default()
    }

    /// Runs one frame: see [`drive`].
    pub fn drive(
        &mut self,
        frame: &PointerFrame,
        timers: Option<&mut crate::frame_timers::FrameTimers>,
    ) -> TickResult {
        drive(&mut self.task, frame, timers)
    }
}

/// Feeds one frame to a task: the clock, the pointer's move, the
/// buttons' presses (with the modifier bits) and releases and the wheel; then, when the GUI timer came
/// due (every frame without a timer table), the forms' repeat and the
/// task's tick. A press that started an arrow's repeat reschedules the GUI
/// timer [`crate::frame_timers::GUI_TIMER_ARROW_PAUSE`] units out.
pub fn drive(
    task: &mut GuiTask,
    frame: &PointerFrame,
    mut timers: Option<&mut crate::frame_timers::FrameTimers>,
) -> TickResult {
    task.clock = frame.now;
    for id in task.draw_order() {
        if let Some(form) = task.form_mut(id).and_then(|f| f.base_mut()) {
            form.clock = frame.now;
        }
    }
    if let Some(at) = frame.at {
        task.update(false, at);
    }
    // The window procedure's button messages: a press carries its button
    // (left 1, right 2, middle 4) with the modifier bits; a release carries
    // the button alone.
    for (down, button) in [
        (frame.pressed, 1u16),
        (frame.right_pressed, 2),
        (frame.middle_pressed, 4),
    ] {
        if !down {
            continue;
        }
        let before = repeat_times(task);
        task.press(button | frame.modifiers);
        let started = repeat_times(task)
            .into_iter()
            .any(|(id, at)| at.is_some() && !before.contains(&(id, at)));
        if started && let Some(timers) = timers.as_deref_mut() {
            timers.reschedule(timers.gui, crate::frame_timers::GUI_TIMER_ARROW_PAUSE);
            info!(
                "gui timer: an arrow's repeat pushes the GUI timer 700 ms out at {} ms",
                timers.now_ms()
            );
        }
    }
    for (up, button) in [
        (frame.released, 1u16),
        (frame.right_released, 2),
        (frame.middle_released, 4),
    ] {
        if up {
            task.release(button);
        }
    }
    if frame.wheel != 0 {
        task.wheel(frame.wheel);
    }
    if !crate::frame_timers::gui_tick(timers.as_deref()) {
        return TickResult::default();
    }
    for id in task.draw_order() {
        if let Some(form) = task.form_mut(id).and_then(|f| f.base_mut()) {
            form.tick(frame.now);
        }
    }
    task.tick()
}

/// Every base form's next repeat time.
fn repeat_times(task: &GuiTask) -> Vec<(FormId, Option<u32>)> {
    task.draw_order()
        .into_iter()
        .filter_map(|id| {
            task.form(id)
                .and_then(|f| f.base())
                .map(|f| (id, f.repeat_at))
        })
        .collect()
}

/// The tooltip background's rounded corner, in the rounded fill's radius
/// words (four units each).
pub const TOOLTIP_CORNER: i32 = 2;

/// The default tooltip on the screen.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ShownTooltip {
    pub text: Vec<u8>,
    /// The pointer it was shown at.
    pub at: (i32, i32),
    pub state: ToolTipState,
}

impl ShownTooltip {
    /// The tick's answer: a tooltip to show starts (its text from the
    /// record), a hide fades the shown one out; a tooltip shown longer
    /// than its time hides by itself.
    pub fn follow(
        shown: &mut Option<ShownTooltip>,
        tick: &TickResult,
        at: Option<(i32, i32)>,
        task: &mut GuiTask,
        now: u32,
    ) {
        if let Some(show) = tick.show {
            let mut tip = shown.take().unwrap_or_default();
            tip.text = task
                .form(show.form)
                .map(|f| f.tooltip_text(show.record))
                .unwrap_or_default();
            tip.at = at.unwrap_or(tip.at);
            tip.state.show(now);
            task.tooltip_showing = true;
            task.tooltip_hide_at = tip.state.hide_at;
            info!(
                "tooltip: {:?} at {:?}",
                String::from_utf8_lossy(&tip.text),
                tip.at
            );
            *shown = Some(tip);
        }
        if let Some(tip) = shown.as_mut()
            && !task.tooltip_showing
            && tip.state.showing
        {
            tip.state.hide(now);
        }
    }

    /// The tooltip's layers at `now`, or `None` once it has faded out.
    pub fn paint(
        &mut self,
        now: u32,
        unit: i32,
        screen: (i32, i32),
        font: &exa_ui::gui_font::GuiFont,
        atlas: Option<(u16, u16)>,
        palette: &exa_ui::skin::SkinPalette,
    ) -> Option<Vec<GuiLayer>> {
        let frame = self.state.animate(now);
        if frame.close {
            return None;
        }
        let width = font.text_width(&self.text);
        let layout = tooltip_layout(width, font.line_height, self.at, screen);
        let draw = tooltip_draw(layout, frame.alpha);
        let mut layers = Vec::new();
        // The background: the skin's rounded fill (the rounded program's
        // black with the corner fade) over the box from (1, 1), one pixel
        // short of its size, its corner two units grown.
        let mut back = SkinBatch::new();
        exa_ui::skin::rounded_fill_fixed(
            &mut back,
            exa_ui::skin::GuiRect::sized(
                (layout.x + 1) as f32,
                (layout.y + 1) as f32,
                (layout.width - 1) as f32,
                (layout.height - 1) as f32,
            ),
            unit,
            TOOLTIP_CORNER,
            draw.background_alpha,
        );
        layers.push(GuiLayer::new(back, LayerTexture::None));
        if let Some(atlas) = atlas {
            let mut ink = palette.text[6];
            ink[3] = (f32::from(ink[3]) * frame.alpha).round() as u8;
            layers.push(crate::gui_paint::caption_layer(
                font,
                atlas,
                &self.text,
                (layout.x + draw.text_at[0], layout.y + draw.text_at[1]),
                ink,
            ));
        }
        Some(layers)
    }
}

#[cfg(test)]
mod button_tests {
    use super::*;
    use exa_ui::form::ClientRect;

    /// A titled window at 100, 50, 200 × 300 (the inventory's words).
    fn window() -> BaseForm {
        BaseForm::new(
            GuiRect::new(100, 50, 200, 300),
            4,
            (0x3010, 0x3_0063, 0),
            ClientRect {
                left: 8,
                top: 24,
                right: 192,
                bottom: 292,
            },
        )
    }

    fn frame(at: (i32, i32)) -> PointerFrame {
        PointerFrame {
            at: Some(at),
            now: 1000,
            ..PointerFrame::default()
        }
    }

    /// The window procedure's words: only a plain left press drags a
    /// title; the same press with Ctrl (0x41) or the right button (2) holds
    /// nothing, as the base form's press rule takes only the word 1.
    #[test]
    fn only_a_plain_left_press_drags_a_title() {
        for (left, right, modifiers, moves) in [
            (true, false, 0, true),
            (true, false, press_modifiers(true, false, false), false),
            (false, true, 0, false),
        ] {
            let mut forms = FormTask::new();
            let id = forms.add(window(), None);
            forms.drive(&frame((200, 52)), None);
            let mut press = frame((200, 52));
            press.pressed = left;
            press.right_pressed = right;
            press.modifiers = modifiers;
            forms.drive(&press, None);
            forms.drive(&frame((260, 90)), None);
            let mut release = frame((260, 90));
            release.released = left;
            release.right_released = right;
            forms.drive(&release, None);
            let left_at = forms.base(id).unwrap().bounds.left;
            assert_eq!(left_at == 160, moves, "{left} {right} {modifiers:#x}");
        }
        assert_eq!(press_modifiers(true, true, true), 0xe0);
    }

    /// The middle button reaches the GUI only while the controls are off.
    #[test]
    fn the_middle_button_is_the_guis_only_with_the_controls_off() {
        let mut mouse = ButtonInput::<MouseButton>::default();
        mouse.press(MouseButton::Middle);
        let mut keys = ButtonInput::<KeyCode>::default();
        keys.press(KeyCode::ShiftLeft);
        let on = PointerFrame::default().with_keys(&mouse, Some(&keys), false);
        assert!(!on.middle_pressed);
        assert_eq!(on.modifiers, 0x20);
        let off = PointerFrame::default().with_keys(&mouse, Some(&keys), true);
        assert!(off.middle_pressed);
    }
}

#[cfg(test)]
mod tooltip_tests {
    use super::*;

    /// The tooltip's background is the skin's rounded fill: the rounded
    /// program's quad grown by two corner words (eight units) around the
    /// box from (1, 1), at alpha 153 once faded in; the text follows.
    #[test]
    fn the_tooltip_back_is_the_skins_rounded_fill() {
        let mut tip = ShownTooltip {
            text: b"Hint".to_vec(),
            at: (200, 200),
            state: ToolTipState::default(),
        };
        tip.state.show(0);
        let font = exa_ui::gui_font::RAYFORM_08;
        let palette = crate::gui_paint::palette();
        let layers = tip
            .paint(1000, 4, (800, 600), &font, Some((256, 256)), &palette)
            .unwrap();
        let back = &layers[0].batch;
        let primitive = &back.primitives[0];
        assert!(primitive.rounded_scale.is_some());
        assert_eq!(back.vertices[0].ink[3], 153);
        let layout = tooltip_layout(
            font.text_width(b"Hint"),
            font.line_height,
            (200, 200),
            (800, 600),
        );
        assert_eq!(back.vertices[0].x, (layout.x + 1 - 32) as f32);
        assert_eq!(back.vertices[0].y, (layout.y + 1 - 32) as f32);
        assert_eq!(layers.len(), 2);
    }
}

//! The pointer, buttons and wheel of one frame fed to a window manager
//! whose forms are base forms ([`exa_ui::base_form::BaseForm`]), and the
//! default tooltip it shows.
//!
//! A frame's pointer moves the task's pointer ([`GuiTask::update`]: the
//! hover and the focus follow it, a held form gets the drag), the left
//! button's press and release go to [`GuiTask::press`] and
//! [`GuiTask::release`] (the original's left button word 1), the wheel to
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
//! with the translucent background (alpha 153 at full strength).

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
    pub pressed: bool,
    pub released: bool,
    /// The wheel's turn this frame, 120 a notch (up positive).
    pub wheel: i32,
    pub now: u32,
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
        wheel,
        now,
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

    /// Adds a base form under the root (or under `parent`).
    pub fn add(&mut self, mut form: BaseForm, parent: Option<FormId>) -> FormId {
        form.parent = Some(parent.unwrap_or(self.root));
        self.task.add(Box::new(form))
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

/// Feeds one frame to a task: the clock, the pointer's move, the left
/// button's press and release and the wheel; then, when the GUI timer came
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
    if frame.pressed {
        let before = repeat_times(task);
        task.press(1);
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
    if frame.released {
        task.release(1);
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
        let mut back = SkinBatch::new();
        back.select(1, 0, false);
        back.set_ink([0, 0, 0, draw.background_alpha]);
        back.begin();
        let (x0, y0) = (layout.x as f32, layout.y as f32);
        let (x1, y1) = (
            (layout.x + layout.width) as f32,
            (layout.y + layout.height) as f32,
        );
        back.vertex(x0, y0);
        back.vertex(x0, y1);
        back.vertex(x1, y1);
        back.vertex(x1, y0);
        back.end_quads();
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

//! The gamepad's menu dispatch, as the original's Steam Input layer runs it
//! (reproduced on Bevy's gamepads, DEC-2): which form receives the menu
//! set's key words, how the focus stack follows the forms that show and
//! hide, the switch between the menu set and the gameplay layer, and what
//! happens to a word the form leaves unanswered.
//!
//! A form that shows pushes itself on the focus stack ([`FocusStack`]) with
//! its set word and the control it focuses; the push activates the menu set
//! and primes its reader. While the stack holds a form the menu reader
//! ([`crate::gamepad::MenuPad`]) runs each frame and hands the top form its
//! key word with the navigation cursor ([`PadMenu::take`]); the gameplay
//! sets read nothing. A form that hides pops itself; the last pop brings
//! the gameplay layer back and primes it. The main menu's constructor
//! installs the menu toggle as the exit hook, so `MenuExit` toggles the
//! menu over whatever form has the focus ([`PadMenu::exit`]).
//!
//! The form's key slot answers each word; [`UNANSWERED`] (the base form's
//! answer) hands it to [`fallback`]: the form's back slot for `MenuBak` on a
//! form that closes on it, the directional pick for a navigation step, or
//! the focused control's key message for the rest.
//!
//! Exarust's forms are not one class tree, so the stack's ids name the live
//! forms ([`PadForm`]) and each form's own input system takes its word.

use bevy::prelude::*;

use crate::gamepad::{
    FocusChange, FocusEntry, FocusStack, MenuFrame, MenuPad, PlayPad, SetRegistry,
};

/// The live forms that push themselves on the focus stack when they show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PadForm {
    /// The main menu (title, arena and in-game pages): pushed by its show
    /// with the set word 0 and no control; in pad mode its selection
    /// starts on the first row.
    MainMenu,
    /// The settings dialog: pushed by its constructor with the set word 0
    /// and control 1.
    Settings,
    /// The manual (the two-pane help screen): pushed with the set word
    /// 0x4000, fast scroll repeats.
    Manual,
    /// The skill screen: pushed by its open routine with the set word 0.
    Skills,
    /// The arena's arsenal view: pushed by its open routine with 0.
    Arsenal,
    /// The arena hub's windows (board, manage, loadout, records, status,
    /// shop and their dialogs): each pushed by its constructor with 0.
    HubWindow,
}

impl PadForm {
    /// Every form, in the order a frame that shows several pushes them
    /// (the lower layers first).
    pub const ALL: [PadForm; 6] = [
        PadForm::MainMenu,
        PadForm::HubWindow,
        PadForm::Arsenal,
        PadForm::Skills,
        PadForm::Settings,
        PadForm::Manual,
    ];

    /// The form's id on the focus stack.
    pub const fn id(self) -> u64 {
        match self {
            PadForm::MainMenu => 1,
            PadForm::Settings => 2,
            PadForm::Manual => 3,
            PadForm::Skills => 4,
            PadForm::Arsenal => 5,
            PadForm::HubWindow => 6,
        }
    }

    /// The form an id names.
    pub fn from_id(id: u64) -> Option<Self> {
        Self::ALL.into_iter().find(|form| form.id() == id)
    }

    /// The set word the form's show pushes: bit 0x2000 fast navigation
    /// repeats, 0x4000 fast scroll repeats, 0x8000 with a low byte a
    /// registered gameplay set (none of the live forms asks for one).
    pub const fn flags(self) -> u32 {
        match self {
            PadForm::Manual => 0x4000,
            _ => 0,
        }
    }

    /// The control the push focuses in pad mode (0: none, the GUI focus is
    /// cleared instead).
    pub const fn control(self) -> i32 {
        match self {
            PadForm::Settings => 1,
            _ => 0,
        }
    }
}

/// One frame's menu key word for the top form, with the reader's cursors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MenuKey {
    pub form: PadForm,
    /// The word: newly pressed `MenuSel` (0x100), `MenuBak` (0x200) and
    /// `MenuExit` (0x400) bits, a direction step (0x?001 navigation,
    /// 0x?002 scroll) and the analog bits (0x10, 0x20).
    pub key: u32,
    /// The navigation cursor (y down), scaled by the frame's sixteenths of
    /// a millisecond while the stick is past its analog threshold.
    pub cursor: Vec2,
    pub scroll_cursor: Vec2,
}

/// The menu dispatch's state between the pad reader and the forms.
#[derive(Resource, Debug, Default)]
pub struct PadMenu {
    key: Option<MenuKey>,
    /// `MenuExit` was newly pressed this frame on a focused form while the
    /// main menu's exit hook is installed: the menu toggles, as F1 does.
    pub exit: bool,
    /// The pad drives the GUI (the controller mode, copied at each
    /// dispatch; a show while it is on focuses the form's first control).
    pub pad_mode: bool,
    /// The GUI root's gamepad flag: set by a push or a dispatch in pad
    /// mode, cleared by the last pop. The forms' window managers copy it
    /// into their task ([`exa_ui::gui_core::GuiTask::gamepad`]).
    pub gui_gamepad: bool,
    /// The forms the last sync found shown, in push order.
    shown: Vec<PadForm>,
    /// Forms pushed by the last sync in pad mode, whose shows focus their
    /// first control or row (the main menu's selection, the dialog's
    /// control).
    focused: Vec<PadForm>,
}

impl PadMenu {
    /// Takes this frame's key word when `form` is the one it is for.
    pub fn take(&mut self, form: PadForm) -> Option<MenuKey> {
        if self.key.is_some_and(|key| key.form == form) {
            self.key.take()
        } else {
            None
        }
    }

    /// This frame's key word, without taking it.
    pub fn peek(&self) -> Option<MenuKey> {
        self.key
    }

    /// Whether the last sync pushed `form` in pad mode (its show then
    /// focuses its first control); taking it clears the request.
    pub fn take_focus_request(&mut self, form: PadForm) -> bool {
        match self.focused.iter().position(|f| *f == form) {
            Some(at) => {
                self.focused.remove(at);
                true
            }
            None => false,
        }
    }

    /// The forms shown at the last sync, in push order.
    pub fn shown(&self) -> &[PadForm] {
        &self.shown
    }

    /// Hands the menu reader's frame to the top form: a word goes to the
    /// stack's top form and, with the controller mode on, turns the GUI's
    /// pad flag on; the exit hook's frame raises [`Self::exit`].
    pub fn dispatch(&mut self, frame: MenuFrame, top: Option<u64>, controller: bool) {
        self.key = None;
        self.exit = frame.exit_hook;
        self.pad_mode = controller;
        if frame.key == 0 {
            return;
        }
        if controller {
            self.gui_gamepad = true;
        }
        self.key = top.and_then(PadForm::from_id).map(|form| MenuKey {
            form,
            key: frame.key,
            cursor: frame.cursor,
            scroll_cursor: frame.scroll_cursor,
        });
    }
}

/// The base form's key slot answer: the word was not used.
pub const UNANSWERED: u32 = 0x40;

/// What becomes of a word after the form's key slot answered it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fallback {
    /// The form used it (any answer but [`UNANSWERED`]), or nothing takes
    /// it.
    Answered,
    /// `MenuBak` on a form whose attribute word asks for it (bit 2): the
    /// form's back slot (it closes).
    Back,
    /// A navigation step: the directional pick of the next control
    /// ([`crate::gamepad::pick_control`]).
    Pick,
    /// The focused control's key message (0xc0) with this word.
    Control(u32),
}

/// The dispatch's handling of an unanswered word. A form whose focused
/// control takes the keys (the form's bit 0x40, set when that control asks
/// for the arrows) hands it the whole word; otherwise `MenuBak` (0x200)
/// goes to the back slot when `back_closes`, a navigation step (bit 1) to
/// the pick, and anything else to the focused control with only its
/// select, back, scroll-step and direction bits (`& 0xf322`).
pub fn fallback(answer: u32, key: u32, control_keys: bool, back_closes: bool) -> Fallback {
    if answer != UNANSWERED {
        return Fallback::Answered;
    }
    if control_keys {
        return Fallback::Control(key);
    }
    if key & 0x200 != 0 {
        return if back_closes {
            Fallback::Back
        } else {
            Fallback::Answered
        };
    }
    if key & 1 != 0 {
        Fallback::Pick
    } else {
        Fallback::Control(key & 0xf322)
    }
}

/// Whether each live form is shown, read before the pad.
#[derive(bevy::ecs::system::SystemParam)]
pub struct ShownForms<'w> {
    menu: Option<Res<'w, crate::menu_screen::MenuScreen>>,
    settings: Option<Res<'w, crate::settings_form::SettingsForm>>,
    manual: Option<Res<'w, crate::help_live::ManualScreen>>,
    skills: Option<Res<'w, crate::skill_screen::SkillUi>>,
    arsenal: Option<Res<'w, crate::arsenal_view::ArsenalWindow>>,
    hub: Option<Res<'w, crate::hub_windows::HubWindows>>,
}

impl ShownForms<'_> {
    fn is_shown(&self, form: PadForm) -> bool {
        match form {
            PadForm::MainMenu => self.menu.as_ref().is_some_and(|m| m.open),
            PadForm::Settings => self.settings.as_ref().is_some_and(|s| s.is_open()),
            PadForm::Manual => self.manual.as_ref().is_some_and(|m| m.is_open()),
            PadForm::Skills => self.skills.as_ref().is_some_and(|s| s.screen.is_some()),
            PadForm::Arsenal => self.arsenal.as_ref().is_some_and(|a| a.open),
            PadForm::HubWindow => self.hub.as_ref().is_some_and(|h| h.is_open()),
        }
    }
}

/// The sets' side of the focus stack: the pad reader's clock and the
/// readers a change primes.
pub struct FocusSets<'a> {
    pub now_ms: i64,
    pub registry: &'a mut SetRegistry,
    pub menu: &'a mut MenuPad,
    pub play: &'a mut PlayPad,
}

impl FocusSets<'_> {
    /// Activates the set a push or a pop names: the menus (always priming
    /// the menu reader), a named gameplay set (its activation callback
    /// primes the gameplay reader), or the gameplay layer's restore after
    /// the last pop (primed when it comes back).
    pub fn apply(&mut self, change: FocusChange) {
        match change {
            FocusChange::Menus => {
                self.registry.apply(change);
                self.menu.activate(self.now_ms);
            }
            FocusChange::Gameplay(_) => {
                self.registry.apply(change);
                self.play.activate_set(self.now_ms);
            }
            FocusChange::Restore => {
                if self.registry.apply(change) {
                    self.play.activate_set(self.now_ms);
                }
            }
            FocusChange::None => {}
        }
    }
}

/// Pushes `form` as its show does: refused past eight forms; in pad mode
/// the GUI's pad flag goes on and the form's first control is focused; the
/// push's set is activated.
pub fn push_form(stack: &mut FocusStack, pad: &mut PadMenu, sets: &mut FocusSets, form: PadForm) {
    let change = stack.push(FocusEntry {
        form: form.id(),
        flags: form.flags(),
    });
    if change == FocusChange::None {
        return;
    }
    if pad.pad_mode {
        pad.gui_gamepad = true;
        pad.focused.push(form);
    }
    sets.apply(change);
}

/// Pops `form` as its hide does: the last pop clears the GUI's pad flag
/// and restores the gameplay layer; otherwise the new top's set comes back.
pub fn pop_form(stack: &mut FocusStack, pad: &mut PadMenu, sets: &mut FocusSets, form: PadForm) {
    if !stack.contains(form.id()) {
        return;
    }
    let change = stack.pop(form.id());
    if !stack.focused() {
        pad.gui_gamepad = false;
    }
    pad.focused.retain(|f| *f != form);
    sets.apply(change);
}

/// Follows the forms' shows and hides: a form hidden since the last frame
/// pops, a newly shown one pushes (in [`PadForm::ALL`] order).
#[allow(clippy::too_many_arguments)]
pub fn sync_focus(
    time: Res<Time<Real>>,
    forms: ShownForms,
    mut stack: ResMut<FocusStack>,
    mut registry: ResMut<SetRegistry>,
    mut menu: ResMut<MenuPad>,
    mut play: ResMut<PlayPad>,
    mut pad: ResMut<PadMenu>,
) {
    let shown: Vec<PadForm> = PadForm::ALL
        .into_iter()
        .filter(|form| forms.is_shown(*form))
        .collect();
    if shown == pad.shown {
        return;
    }
    let mut sets = FocusSets {
        now_ms: time.elapsed().as_millis() as i64,
        registry: &mut registry,
        menu: &mut menu,
        play: &mut play,
    };
    let before = std::mem::take(&mut pad.shown);
    for form in before.iter().filter(|form| !shown.contains(form)) {
        pop_form(&mut stack, &mut pad, &mut sets, *form);
        info!("pad: {form:?} leaves the focus stack");
    }
    for form in shown.iter().filter(|form| !before.contains(form)) {
        push_form(&mut stack, &mut pad, &mut sets, *form);
        info!(
            "pad: {form:?} takes the focus (stack {}, set {:?})",
            stack.depth(),
            sets.registry.active()
        );
    }
    pad.shown = shown;
}

#[cfg(test)]
#[path = "pad_menu_test.rs"]
mod tests;

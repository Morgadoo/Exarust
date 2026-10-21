//! The top-level GUI layer (`TGUITask`): a form with no border that
//! stretches with the root, owns the forms of one screen layer, carries its
//! own keyboard-shortcut table, dims itself when it is not the active layer,
//! runs per-frame callbacks and hit tests its forms from the top down.
//!
//! The rules come from the original's layer constructor and destructor, its
//! close (which leaves the task list), its frame slot, its notice word, its
//! shortcut table and best-match dispatch, its draw and hit-test slots, and
//! the three focus hooks the form base class and the task switch call.
//! [`TaskFocus`] keeps the two records those hooks read: the layer's main
//! form and the focus it saved when another layer was activated.

use crate::form::{DEFAULT_HEIGHT, DEFAULT_WIDTH, style};
use crate::gui_core::{FormId, GuiRect};

/// The key the shortcut scan never looks up: the Shift virtual key (0x10).
/// It goes straight to the parent layer, as an unmatched key does.
pub const NO_SHORTCUT_KEY: u8 = 0x10;

/// Shortcut modifier bits, as the input layer keeps the held modifiers:
/// Shift, Control and Alt.
pub mod modifier {
    pub const SHIFT: u8 = 0x20;
    pub const CONTROL: u8 = 0x40;
    pub const ALT: u8 = 0x80;
}

/// The pair of GUI alphas a layer sets before drawing its forms. The
/// active layer and the root set 1.0 and 1.0; every other layer 0.75 and
/// 0.5. The first is the GUI's draw alpha (the graphics unit also keeps it
/// as the byte `round(a · 255)`); the second is handed to the renderer
/// with it, and what the renderer does with it is not established here.
pub const ACTIVE_ALPHA: (f32, f32) = (1.0, 1.0);
pub const INACTIVE_ALPHA: (f32, f32) = (0.75, 0.5);

/// The layer's style word after its constructor: both anchor masks
/// (`0xf0000`), so the layer stretches with the root in x and y.
pub const LAYER_STYLE: u32 = style::X_MASK | style::Y_MASK;

/// The layer's attribute word after its constructor: none (no title bar,
/// no client fill, no scrollbars), in place of the form default.
pub const LAYER_ATTRIBUTES: u32 = 0;

/// The layer's border after its constructor (the form default is 2).
pub const LAYER_BORDER: i32 = 0;

/// One shortcut-table entry: a key, the modifiers it needs and the action
/// word it runs. The original's records are 16 bytes (key byte, modifier
/// byte, handler pointer); the handler is an action word here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shortcut {
    pub key: u8,
    pub modifiers: u8,
    pub action: u32,
}

impl Shortcut {
    pub fn new(key: u8, modifiers: u8, action: u32) -> Self {
        Self {
            key,
            modifiers,
            action,
        }
    }
}

/// The modifier score of a shortcut's required modifiers against the
/// modifiers held: −1 when a modifier the entry needs is not held, else one
/// point per modifier it needs. Held modifiers the entry does not need cost
/// nothing.
pub fn modifier_score(required: u8, held: u8) -> i32 {
    let mut score = 0;
    for bit in [modifier::SHIFT, modifier::CONTROL, modifier::ALT] {
        if required & bit != 0 {
            if held & bit == 0 {
                return -1;
            }
            score += 1;
        }
    }
    score
}

/// What a layer's key slot did with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutcome {
    /// A shortcut fired: its table index and action word.
    Shortcut { index: usize, action: u32 },
    /// No shortcut fired; the key goes on to the parent layer's key slot.
    Forward,
    /// Nothing happens: a key whose state word has either of its low two
    /// bits set, or an unmatched key on the root layer.
    Ignored,
}

/// A callback run on every layer frame. It receives the layer and answers
/// whether it stays in the list.
type Callback = Box<dyn FnMut(FormId) -> bool + Send + Sync>;

/// The callback list a layer runs each frame (the original's generic list
/// of its system unit: append, run, free).
#[derive(Default)]
pub struct CallbackList {
    callbacks: Vec<Callback>,
}

impl std::fmt::Debug for CallbackList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackList")
            .field("len", &self.callbacks.len())
            .finish()
    }
}

impl CallbackList {
    /// Appends a callback.
    pub fn add(&mut self, callback: impl FnMut(FormId) -> bool + Send + Sync + 'static) {
        self.callbacks.push(Box::new(callback));
    }

    /// Empties the list.
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }

    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }

    /// Runs the callbacks in order with `arg`. One that answers `false` is
    /// removed and the later ones close up, so the next one runs at the
    /// same index.
    pub fn run(&mut self, arg: FormId) {
        let mut index = 0;
        while index < self.callbacks.len() {
            if (self.callbacks[index])(arg) {
                index += 1;
            } else {
                self.callbacks.remove(index);
            }
        }
    }
}

/// One form owned by a layer, as the layer's hit test and draw see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedForm {
    pub id: FormId,
    pub bounds: GuiRect,
    pub visible: bool,
    /// The form's style word; [`style::NO_HIT`] keeps it out of the
    /// layer's hit test.
    pub style: u32,
}

impl OwnedForm {
    pub fn new(id: FormId, bounds: GuiRect) -> Self {
        Self {
            id,
            bounds,
            visible: true,
            style: 0,
        }
    }
}

/// The two focus records of a layer: its main form (the first form
/// created on it) and the form and control it saved when another layer was
/// activated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskFocus {
    /// The layer's main form: closing it closes the layer.
    pub main: Option<FormId>,
    /// The focused form and control saved when the layer was left.
    pub saved: Option<(FormId, i32)>,
}

impl TaskFocus {
    /// The leave hook: when another layer is activated and the focused
    /// form belongs to this layer, the focused form and control are saved.
    pub fn save(&mut self, focused: FormId, control: i32, focused_on_this_layer: bool) {
        if focused_on_this_layer {
            self.saved = Some((focused, control));
        }
    }

    /// The form-close hook: the main form clears the main record and asks
    /// for the layer to close (`true`); the saved form clears the saved
    /// record.
    pub fn form_closed(&mut self, form: FormId) -> bool {
        if Some(form) == self.main {
            self.main = None;
            return true;
        }
        if self.saved.is_some_and(|(saved, _)| saved == form) {
            self.saved = None;
        }
        false
    }

    /// The return hook, when the layer is activated again: the focus goes
    /// to the saved form with its control when that form still shows; a
    /// saved form that no longer shows is forgotten and the focus goes to
    /// the main form, else to the layer itself, with the control −1.
    pub fn restore(&mut self, layer: FormId, saved_visible: bool) -> (FormId, i32) {
        if let Some((saved, control)) = self.saved {
            if saved_visible {
                return (saved, control);
            }
            self.saved = None;
        }
        (self.main.unwrap_or(layer), -1)
    }
}

/// A top-level layer: its forms, its shortcut table, its frame callbacks,
/// its notice word and bounds, and the flags that place it in the task
/// list.
pub struct TaskLayer {
    /// The layer's own form id.
    pub id: FormId,
    pub bounds: GuiRect,
    pub forms: Vec<OwnedForm>,
    pub shortcuts: Vec<Shortcut>,
    pub callbacks: CallbackList,
    /// The word [`TaskLayer::raise`] ORs bits into. Its readers are outside
    /// the layer class.
    pub notices: u32,
    /// Whether this layer is the last non-root layer activated (it draws at
    /// full strength).
    pub active: bool,
    /// Whether this layer is the root task (never dimmed, never forwards).
    pub root: bool,
    /// The layer's flag that keeps its forms out of the hit test (set
    /// outside the layer class).
    pub hit_disabled: bool,
    /// The layer left the task list.
    pub closed: bool,
    pub focus: TaskFocus,
}

impl TaskLayer {
    /// A layer at the origin, the root's size when there is a root (else
    /// the form default size), with no border, [`LAYER_STYLE`] and
    /// [`LAYER_ATTRIBUTES`]. The original also parents every layer to the
    /// root when one exists and makes the layer its own task.
    pub fn new(id: FormId, root_size: Option<(i32, i32)>) -> Self {
        let (width, height) = root_size.unwrap_or((DEFAULT_WIDTH, DEFAULT_HEIGHT));
        Self {
            id,
            bounds: GuiRect::new(0, 0, width, height),
            forms: Vec::new(),
            shortcuts: Vec::new(),
            callbacks: CallbackList::default(),
            notices: 0,
            active: false,
            root: false,
            hit_disabled: false,
            closed: false,
            focus: TaskFocus::default(),
        }
    }

    /// Appends a shortcut entry and returns its table index.
    pub fn add_shortcut(&mut self, key: u8, modifiers: u8, action: u32) -> usize {
        self.shortcuts.push(Shortcut::new(key, modifiers, action));
        self.shortcuts.len() - 1
    }

    /// The best shortcut for a key and the modifiers held: each entry for
    /// the key scores its [`modifier_score`] plus one, so an entry with no
    /// required modifier scores 1 and one missing a modifier 0. The highest
    /// score above zero wins; a tie keeps the earlier entry.
    pub fn best_shortcut(&self, key: u8, held: u8) -> Option<(usize, i32)> {
        let mut best = None;
        let mut best_score = 0;
        for (index, entry) in self.shortcuts.iter().enumerate() {
            if entry.key != key {
                continue;
            }
            let score = modifier_score(entry.modifiers, held) + 1;
            if best_score < score {
                best_score = score;
                best = Some((index, score));
            }
        }
        best
    }

    /// The key slot. A key whose state has either low bit set does nothing.
    /// Any other key but Shift looks up the best shortcut and fires it.
    /// Shift and a key with no shortcut go to the parent's key slot, unless
    /// this is the root. The original also records the layer as the current
    /// task before calling the handler with the layer.
    pub fn key(&self, key: u8, held_modifiers: u8, state: u32) -> KeyOutcome {
        if state & 3 != 0 {
            return KeyOutcome::Ignored;
        }
        if key != NO_SHORTCUT_KEY
            && let Some((index, _)) = self.best_shortcut(key, held_modifiers)
        {
            return KeyOutcome::Shortcut {
                index,
                action: self.shortcuts[index].action,
            };
        }
        if self.root {
            KeyOutcome::Ignored
        } else {
            KeyOutcome::Forward
        }
    }

    /// The alphas the layer's draw sets: full for the active layer and the
    /// root, dimmed for every other layer.
    pub fn draw_alpha(&self) -> (f32, f32) {
        if self.active || self.root {
            ACTIVE_ALPHA
        } else {
            INACTIVE_ALPHA
        }
    }

    /// The forms to draw, bottom to top: every visible one, in list order.
    pub fn draw_order(&self) -> Vec<FormId> {
        self.forms
            .iter()
            .filter(|form| form.visible)
            .map(|form| form.id)
            .collect()
    }

    /// The layer's hit test: nothing while [`TaskLayer::hit_disabled`];
    /// else the forms from the top down, a form only when it is visible,
    /// its style lacks [`style::NO_HIT`] and the point lies inside its
    /// rectangle, far edges included. `hit` receives the form and the point
    /// in its coordinates; the first non-zero code ends the search.
    pub fn hit(
        &self,
        x: i32,
        y: i32,
        mut hit: impl FnMut(FormId, i32, i32) -> i32,
    ) -> Option<(FormId, i32)> {
        if self.hit_disabled {
            return None;
        }
        for form in self.forms.iter().rev() {
            if !form.visible || form.style & style::NO_HIT != 0 {
                continue;
            }
            let left = i64::from(form.bounds.left);
            let top = i64::from(form.bounds.top);
            let (x64, y64) = (i64::from(x), i64::from(y));
            if x64 < left
                || y64 < top
                || x64 > left + i64::from(form.bounds.width())
                || y64 > top + i64::from(form.bounds.height())
            {
                continue;
            }
            let code = hit(form.id, x - form.bounds.left, y - form.bounds.top);
            if code != 0 {
                return Some((form.id, code));
            }
        }
        None
    }

    /// ORs bits into the layer's notice word.
    pub fn raise(&mut self, word: u32) {
        self.notices |= word;
    }

    /// Reads and clears the notice word (Exarust's reader; the original's
    /// readers are outside the layer class).
    pub fn take_notices(&mut self) -> u32 {
        std::mem::take(&mut self.notices)
    }

    /// Adds a form on top and returns its id. The first form added becomes
    /// the layer's main form (the form base class's constructor rule).
    pub fn add_form(&mut self, form: OwnedForm) -> FormId {
        if self.forms.is_empty() {
            self.focus.main = Some(form.id);
        }
        self.forms.push(form);
        form.id
    }

    /// A form of the layer closes: the focus records that name it are
    /// cleared and the form leaves the list; closing the main form closes
    /// the layer too. Returns whether the form was on the layer.
    pub fn remove_form(&mut self, form: FormId) -> bool {
        if self.focus.form_closed(form) {
            self.close();
        }
        let before = self.forms.len();
        self.forms.retain(|owned| owned.id != form);
        self.forms.len() != before
    }

    /// The layer's destructor: the callback list and the shortcut table go
    /// before the form base class frees the forms. (The original also frees
    /// one more owned object whose class is outside the layer.)
    pub fn destroy(&mut self) {
        self.callbacks.clear();
        self.shortcuts.clear();
        self.forms.clear();
        self.focus = TaskFocus::default();
        self.closed = true;
    }

    /// The close slot: after the form base class's close the layer leaves
    /// the task list (the later layers move down one place).
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// The frame slot: the layer becomes the current task and its callbacks
    /// run in order with the layer.
    pub fn update(&mut self) {
        self.callbacks.run(self.id);
    }

    /// The return hook (see [`TaskFocus::restore`]): the form and control to
    /// focus when the layer is activated again.
    pub fn return_focus(&mut self, saved_visible: bool) -> (FormId, i32) {
        self.focus.restore(self.id, saved_visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer() -> TaskLayer {
        TaskLayer::new(1, Some((800, 600)))
    }

    #[test]
    fn modifier_scores_count_needed_modifiers_and_refuse_missing_ones() {
        assert_eq!(modifier_score(0, 0), 0);
        assert_eq!(modifier_score(0, 0xff), 0);
        assert_eq!(modifier_score(modifier::SHIFT, 0), -1);
        assert_eq!(modifier_score(modifier::SHIFT, modifier::SHIFT), 1);
        assert_eq!(
            modifier_score(modifier::SHIFT | modifier::CONTROL, modifier::SHIFT),
            -1
        );
        assert_eq!(
            modifier_score(
                modifier::SHIFT | modifier::CONTROL,
                modifier::SHIFT | modifier::CONTROL | modifier::ALT
            ),
            2
        );
        assert_eq!(modifier_score(modifier::ALT, modifier::ALT), 1);
        assert_eq!(modifier_score(modifier::ALT, modifier::SHIFT), -1);
    }

    #[test]
    fn the_best_modifier_match_wins_and_a_tie_keeps_the_first() {
        let mut task = layer();
        let plain = task.add_shortcut(b'a', 0, 1);
        let shift = task.add_shortcut(b'a', modifier::SHIFT, 2);
        let both = task.add_shortcut(b'a', modifier::SHIFT | modifier::CONTROL, 3);
        // Shift and Control held: the two-modifier entry scores 2 + 1.
        assert_eq!(
            task.best_shortcut(b'a', modifier::SHIFT | modifier::CONTROL),
            Some((both, 3))
        );
        // Shift and Alt held: the Shift entry (2) beats the plain one (1);
        // the two-modifier entry lacks Control and scores 0.
        assert_eq!(
            task.best_shortcut(b'a', modifier::SHIFT | modifier::ALT),
            Some((shift, 2))
        );
        // Nothing held: the plain entry still scores 1 and wins.
        assert_eq!(task.best_shortcut(b'a', 0), Some((plain, 1)));
        // A different key has no entry.
        assert_eq!(task.best_shortcut(b'b', 0xff), None);
        // Equal scores keep the first entry.
        task.add_shortcut(b'c', modifier::SHIFT, 4);
        task.add_shortcut(b'c', modifier::SHIFT, 5);
        assert_eq!(task.best_shortcut(b'c', modifier::SHIFT), Some((3, 2)));
    }

    #[test]
    fn the_key_slot_fires_the_best_shortcut_or_forwards() {
        let mut task = layer();
        task.add_shortcut(b'k', modifier::CONTROL, 0x42);
        task.add_shortcut(b'p', 0, 0x43);
        task.add_shortcut(NO_SHORTCUT_KEY, 0, 0x44);
        assert_eq!(
            task.key(b'k', modifier::CONTROL, 0),
            KeyOutcome::Shortcut {
                index: 0,
                action: 0x42
            }
        );
        // An entry without modifiers fires with or without modifiers held.
        assert_eq!(
            task.key(b'p', modifier::ALT, 0),
            KeyOutcome::Shortcut {
                index: 1,
                action: 0x43
            }
        );
        // Control not held: the entry scores 0, the key is forwarded.
        assert_eq!(task.key(b'k', 0, 0), KeyOutcome::Forward);
        // Shift is never looked up, even with an entry for it: forwarded.
        assert_eq!(task.key(NO_SHORTCUT_KEY, 0, 0), KeyOutcome::Forward);
        // Either low state bit: nothing at all.
        assert_eq!(task.key(b'k', modifier::CONTROL, 1), KeyOutcome::Ignored);
        assert_eq!(task.key(b'k', modifier::CONTROL, 2), KeyOutcome::Ignored);
        // The root has nobody to forward to, but its shortcuts fire.
        task.root = true;
        assert_eq!(task.key(b'k', 0, 0), KeyOutcome::Ignored);
        assert_eq!(task.key(NO_SHORTCUT_KEY, 0, 0), KeyOutcome::Ignored);
        assert!(matches!(
            task.key(b'p', 0, 0),
            KeyOutcome::Shortcut { index: 1, .. }
        ));
    }

    #[test]
    fn only_the_active_and_root_layers_draw_full_strength() {
        let mut task = layer();
        assert_eq!(task.draw_alpha(), (0.75, 0.5));
        task.active = true;
        assert_eq!(task.draw_alpha(), (1.0, 1.0));
        task.active = false;
        task.root = true;
        assert_eq!(task.draw_alpha(), (1.0, 1.0));
    }

    #[test]
    fn forms_are_drawn_visible_only_and_hit_from_the_top_down() {
        let mut task = layer();
        let a = task.add_form(OwnedForm::new(10, GuiRect::new(0, 0, 100, 100)));
        let b = task.add_form(OwnedForm::new(11, GuiRect::new(50, 50, 100, 100)));
        task.add_form(OwnedForm::new(12, GuiRect::new(200, 200, 10, 10)));
        task.forms[2].visible = false;
        assert_eq!(task.draw_order(), vec![a, b]);
        // The top form gets the point first, in its own coordinates; it
        // answers 0, so the one below is asked.
        let mut asked = Vec::new();
        assert_eq!(
            task.hit(60, 60, |form, x, y| {
                asked.push((form, x, y));
                if form == b { 0 } else { 7 }
            }),
            Some((a, 7))
        );
        assert_eq!(asked, vec![(b, 10, 10), (a, 60, 60)]);
        // The far edges are inside: (100, 100) still hits the first form.
        assert_eq!(
            task.hit(100, 100, |form, _, _| i32::from(form == a)),
            Some((a, 1))
        );
        // A hidden form is passed over.
        assert_eq!(task.hit(205, 205, |_, _, _| 1), None);
        // A form whose style has the no-hit bit is passed over.
        task.forms[1].style = style::NO_HIT;
        assert_eq!(task.hit(60, 60, |_, _, _| 1), Some((a, 1)));
        // The layer's own flag keeps every form out of the hit test.
        task.hit_disabled = true;
        assert_eq!(task.hit(10, 10, |_, _, _| 1), None);
    }

    #[test]
    fn raising_a_word_accumulates_until_read() {
        let mut task = layer();
        task.raise(0x40);
        task.raise(0x2);
        assert_eq!(task.notices, 0x42);
        assert_eq!(task.take_notices(), 0x42);
        assert_eq!(task.take_notices(), 0);
    }

    #[test]
    fn frames_run_the_callbacks_and_drop_those_that_answer_false() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        let mut task = layer();
        let count = Arc::new(AtomicU32::new(0));
        let (first, second, third) = (count.clone(), count.clone(), count.clone());
        // Runs once, then leaves the list.
        task.callbacks.add(move |layer| {
            assert_eq!(layer, 1);
            first.fetch_add(1, Ordering::SeqCst);
            false
        });
        // The callback after it closes up and still runs in the same frame.
        task.callbacks.add(move |_| {
            second.fetch_add(10, Ordering::SeqCst);
            true
        });
        task.callbacks.add(move |_| {
            third.fetch_add(100, Ordering::SeqCst);
            true
        });
        task.update();
        assert_eq!(count.load(Ordering::SeqCst), 111);
        assert_eq!(task.callbacks.len(), 2);
        task.update();
        assert_eq!(count.load(Ordering::SeqCst), 221);
        task.callbacks.clear();
        assert!(task.callbacks.is_empty());
    }

    #[test]
    fn the_focus_hooks_save_restore_and_clear() {
        let mut task = layer();
        let main = task.add_form(OwnedForm::new(5, GuiRect::new(0, 0, 10, 10)));
        task.add_form(OwnedForm::new(6, GuiRect::new(0, 0, 10, 10)));
        assert_eq!(task.focus.main, Some(main));
        // Leaving the layer with the focus on another layer's form saves
        // nothing; with the focus on this layer's form 6, control 3, it is
        // saved.
        task.focus.save(40, 2, false);
        assert_eq!(task.focus.saved, None);
        task.focus.save(6, 3, true);
        assert_eq!(task.focus.saved, Some((6, 3)));
        // Coming back while form 6 shows: the saved focus returns and stays.
        assert_eq!(task.return_focus(true), (6, 3));
        assert_eq!(task.focus.saved, Some((6, 3)));
        // Form 6 no longer shows: it is forgotten, the main form gets −1.
        assert_eq!(task.return_focus(false), (main, -1));
        assert_eq!(task.focus.saved, None);
        // Closing the saved form forgets the saved record only.
        task.focus.save(6, 1, true);
        assert!(task.remove_form(6));
        assert_eq!(task.focus.saved, None);
        assert!(!task.closed);
        // Closing the main form closes the layer; the layer itself then
        // takes the returned focus.
        assert!(task.remove_form(main));
        assert!(task.closed);
        assert_eq!(task.focus.main, None);
        assert_eq!(task.return_focus(false), (1, -1));
        assert!(!task.remove_form(main));
    }

    #[test]
    fn the_layer_constructor_matches_the_original() {
        let mut task = layer();
        assert_eq!(task.bounds, GuiRect::new(0, 0, 800, 600));
        assert_eq!(LAYER_STYLE, 0x000f_0000);
        assert_eq!(LAYER_ATTRIBUTES, 0);
        assert_eq!(LAYER_BORDER, 0);
        // The root layer, built before any root exists, keeps the form
        // default size.
        assert_eq!(
            TaskLayer::new(2, None).bounds,
            GuiRect::new(0, 0, DEFAULT_WIDTH, DEFAULT_HEIGHT)
        );
        assert!(task.shortcuts.is_empty());
        assert_eq!(task.notices, 0);
        assert!(!task.active && !task.root && !task.hit_disabled);
        assert!(!task.closed);
        task.close();
        assert!(task.closed);
        task.add_form(OwnedForm::new(3, GuiRect::new(0, 0, 1, 1)));
        task.add_shortcut(b'a', modifier::SHIFT, 1);
        task.callbacks.add(|_| true);
        task.destroy();
        assert!(task.forms.is_empty() && task.shortcuts.is_empty());
        assert!(task.callbacks.is_empty());
    }
}

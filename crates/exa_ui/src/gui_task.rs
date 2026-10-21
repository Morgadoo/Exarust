//! The top-level GUI layer (`TGUITask`): a full-screen, unframed form that
//! owns the forms of one screen layer, carries its own keyboard-shortcut
//! table, dims itself when it is not the active layer, runs per-frame
//! callbacks and hit tests its forms from the top down.
//!
//! The rules come from the original's layer constructor, its shortcut table
//! and best-match dispatch, its draw and hit-test slots, its close (which
//! leaves the task list), its raise word and the three focus hooks the form
//! base class calls. [`TaskFocus`] keeps the focus stack the hooks describe.

use crate::gui_core::{FormId, GuiRect};

/// The key a layer never treats as a shortcut (Tab belongs to the focus).
pub const NO_SHORTCUT_KEY: u8 = 0x10;

/// Shortcut modifier bits: Shift, Control and Alt.
pub mod modifier {
    pub const SHIFT: u8 = 0x20;
    pub const CONTROL: u8 = 0x40;
    pub const ALT: u8 = 0x80;
}

/// The opacity pair a layer draws itself with: its own alpha and the alpha
/// of its own ink. The active layer and the root task draw at full
/// strength; every other layer at 0.75 and 0.5.
pub const ACTIVE_ALPHA: (f32, f32) = (1.0, 1.0);
pub const INACTIVE_ALPHA: (f32, f32) = (0.75, 0.5);

/// The word the original's draw leaves on the skin at the start of a
/// layer: 16, the mode that draws plain flat quads.
pub const LAYER_DRAW_MODE: u32 = 0x10;

/// The layer's style word after its constructor: `0xf0000` (a full-screen
/// page with the small marks and no title).
pub const LAYER_STYLE: u32 = 0x000f_0000;

/// The state word bit that keeps a layer out of the hit test.
pub const NO_HIT: u32 = 0x2;

/// One shortcut-table entry: a key, the modifiers it needs and the action
/// word it runs. The original's records are 16 bytes; the handler is a
/// function pointer there and an action word here.
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

/// The best-match score of a shortcut's modifiers against the modifiers
/// held: −1 when a modifier the entry needs is not held, else one point per
/// modifier it needs. Held modifiers the entry does not need cost nothing,
/// so an entry with no required modifier never scores above zero and never
/// fires.
pub fn modifier_score(required: u8, held: u8) -> i32 {
    let mut score = 0;
    if required & modifier::SHIFT != 0 {
        if held & modifier::SHIFT == 0 {
            return -1;
        }
        score = 1;
    }
    if required & modifier::CONTROL != 0 {
        if held & modifier::CONTROL == 0 {
            return -1;
        }
        score += 1;
    }
    if required & modifier::ALT != 0 && held & modifier::ALT == 0 {
        return -1;
    }
    if required & modifier::ALT != 0 {
        score += 1;
    }
    score
}

/// What a layer's key slot did with a key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyOutcome {
    /// A shortcut fired: its table index and action word.
    Shortcut { index: usize, action: u32 },
    /// No shortcut matched; the key goes on to the parent layer.
    Forward,
    /// The key is not a shortcut key at all: Tab, or a key with the state
    /// word's low two bits set (a repeat or a key held down).
    Ignored,
}

/// A callback run on every layer frame. The original's list is the
/// generic callback list of its system unit: add, clear, call.
type Callback = Box<dyn FnMut() + Send + Sync>;

/// The order-preserving callback list a layer ticks each frame.
#[derive(Default)]
pub struct CallbackList {
    callbacks: Vec<Option<Callback>>,
}

impl std::fmt::Debug for CallbackList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallbackList")
            .field("len", &self.callbacks.len())
            .finish()
    }
}

impl CallbackList {
    /// Adds a callback; its index is the handle `remove` takes.
    pub fn add(&mut self, callback: impl FnMut() + Send + Sync + 'static) -> usize {
        self.callbacks.push(Some(Box::new(callback)));
        self.callbacks.len() - 1
    }

    /// Removes a callback by handle (a gap is kept, as the original's
    /// removal leaves the list's other entries in place).
    pub fn remove(&mut self, index: usize) {
        if let Some(slot) = self.callbacks.get_mut(index) {
            *slot = None;
        }
    }

    /// Empties the list.
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }

    /// How many callbacks the list holds, gaps included.
    pub fn len(&self) -> usize {
        self.callbacks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.callbacks.is_empty()
    }

    /// Runs every live callback in order.
    pub fn run(&mut self) {
        for callback in self.callbacks.iter_mut().flatten() {
            callback();
        }
    }
}

/// One form owned by a layer, as the layer's hit test and draw see it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OwnedForm {
    pub id: FormId,
    pub bounds: GuiRect,
    pub visible: bool,
    /// The form's state word; bit 1 keeps it out of the layer's hit test.
    pub state: u32,
    /// Its style word; the layer's draw asks nothing of it.
    pub style: u32,
}

impl OwnedForm {
    pub fn new(id: FormId, bounds: GuiRect) -> Self {
        Self {
            id,
            bounds,
            visible: true,
            state: 0,
            style: 0,
        }
    }
}

/// The focus stack the original's form base class keeps on a layer: the
/// form a newly activated form remembers, the form it asks for focus when
/// it closes, and the stored form and control to give the focus back to.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct TaskFocus {
    /// The form that holds the focus and the control it had selected.
    pub current: Option<(FormId, i32)>,
    /// The form and control an activated form saved to restore later.
    pub saved: Option<(FormId, i32)>,
    /// The form a closing form hands the focus to.
    pub next: Option<FormId>,
}

impl TaskFocus {
    /// A form gained the focus: it stores the focus it replaced in its own
    /// saved record and becomes the current form with `control`.
    pub fn activate(&mut self, form: FormId, control: i32) {
        self.saved = self.current;
        self.current = Some((form, control));
    }

    /// A form is freed: one that was the closing form's next focus clears
    /// it (and tells the form to give the focus up), one that was the saved
    /// focus clears the saved record.
    pub fn closed(&mut self, form: FormId) -> bool {
        if Some(form) == self.next {
            self.next = None;
            self.current = None;
            return true;
        }
        if self.saved.is_some_and(|(saved, _)| saved == form) {
            self.saved = None;
        }
        false
    }

    /// The closing form gives the focus back: the saved form when it still
    /// shows (with its saved control), else the next form (or the form
    /// itself) with the control −1.
    pub fn restore(&mut self, form: FormId, saved_visible: bool) -> (Option<FormId>, i32) {
        if saved_visible && let Some((saved, control)) = self.saved {
            self.current = Some((saved, control));
            return (Some(saved), control);
        }
        self.saved = None;
        let target = self.next.unwrap_or(form);
        self.current = Some((target, -1));
        (Some(target), -1)
    }
}

/// A top-level layer: its forms, its shortcut table, its frame callbacks,
/// its raise word, its bounds and the flags that place it in the task list.
pub struct TaskLayer {
    /// The layer's own form id.
    pub id: FormId,
    pub parent: Option<FormId>,
    pub bounds: GuiRect,
    pub forms: Vec<OwnedForm>,
    pub shortcuts: Vec<Shortcut>,
    pub callbacks: CallbackList,
    /// The word raised by [`TaskLayer::raise`], cleared by the owner.
    pub notices: u32,
    /// Whether this layer is the current input layer.
    pub active: bool,
    /// Whether this layer is the root task (never dimmed, never forwards).
    pub root: bool,
    /// The form state bit [`NO_HIT`]: the layer does not hit test its forms.
    pub state: u32,
    /// How many frames the layer has run.
    pub frames: u64,
    /// The layer left the task list.
    pub closed: bool,
    pub focus: TaskFocus,
}

impl TaskLayer {
    /// A layer at the screen's origin: full screen, no frame, no shortcut,
    /// a zero raise word, the original's layer style and state clear.
    pub fn new(id: FormId, screen: (i32, i32)) -> Self {
        Self {
            id,
            parent: None,
            bounds: GuiRect::new(0, 0, screen.0, screen.1),
            forms: Vec::new(),
            shortcuts: Vec::new(),
            callbacks: CallbackList::default(),
            notices: 0,
            active: false,
            root: false,
            state: 0,
            frames: 0,
            closed: false,
            focus: TaskFocus::default(),
        }
    }

    /// Adds a shortcut entry and returns its table index.
    pub fn add_shortcut(&mut self, key: u8, modifiers: u8, action: u32) -> usize {
        self.shortcuts.push(Shortcut::new(key, modifiers, action));
        self.shortcuts.len() - 1
    }

    /// The best shortcut for a key and the modifiers held: the qualifying
    /// entry with the highest modifier score (the first one on a tie, as
    /// the original scans forward and only replaces a strictly better
    /// score). Entries that need more modifiers than are held are skipped.
    pub fn best_shortcut(&self, key: u8, held: u8) -> Option<(usize, i32)> {
        let mut best = None;
        let mut best_score = 0;
        for (index, entry) in self.shortcuts.iter().enumerate() {
            if entry.key != key {
                continue;
            }
            let score = modifier_score(entry.modifiers, held);
            if best_score < score {
                best_score = score;
                best = Some((index, score));
            }
        }
        best
    }

    /// The key slot: a layer key that is not Tab and not a repeat
    /// (`state & 3 == 0`) looks up the best shortcut; one that scores above
    /// zero fires. Otherwise a non-root layer forwards the key to its
    /// parent. The original also records the layer as the current task and
    /// calls the handler with the layer as its argument.
    pub fn key(&self, key: u8, held_modifiers: u8, state: u32) -> KeyOutcome {
        if state & 3 != 0 || key == NO_SHORTCUT_KEY {
            return KeyOutcome::Ignored;
        }
        if let Some((index, score)) = self.best_shortcut(key, held_modifiers)
            && score > 0
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

    /// The layer's draw alpha: the active layer and the root draw at full
    /// strength, every other layer dimmed.
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

    /// The layer's hit test: forms from the top down, a form only when it
    /// is visible, its state word does not carry [`NO_HIT`] and the point
    /// lies inside its rectangle. `hit` receives the form and the point in
    /// its coordinates. Returns the form and the code it answered.
    pub fn hit(
        &self,
        x: i32,
        y: i32,
        mut hit: impl FnMut(FormId, i32, i32) -> i32,
    ) -> Option<(FormId, i32)> {
        if self.state & NO_HIT != 0 {
            return None;
        }
        for form in self.forms.iter().rev() {
            if !form.visible || form.state & NO_HIT != 0 {
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

    /// Raises a word on the layer (the original's redraw-notification
    /// slot): the bits accumulate until the owner reads them.
    pub fn raise(&mut self, word: u32) {
        self.notices |= word;
    }

    /// Reads and clears the raise word.
    pub fn take_notices(&mut self) -> u32 {
        std::mem::take(&mut self.notices)
    }

    /// Adds a form on top and returns its id.
    pub fn add_form(&mut self, form: OwnedForm) -> FormId {
        self.forms.push(form);
        form.id
    }

    /// Removes a form from the layer (the close slot left the task list);
    /// also clears the focus records that named it.
    pub fn remove_form(&mut self, form: FormId) -> bool {
        self.focus.closed(form);
        let before = self.forms.len();
        self.forms.retain(|owned| owned.id != form);
        self.forms.len() != before
    }

    /// The layer's destructor: every owned form, shortcut and callback goes
    /// (the original frees its form list, shortcut array and callback list).
    pub fn destroy(&mut self) {
        self.forms.clear();
        self.shortcuts.clear();
        self.callbacks.clear();
        self.focus = TaskFocus::default();
        self.notices = 0;
        self.closed = true;
    }

    /// The close slot: the layer leaves the task list.
    pub fn close(&mut self) {
        self.closed = true;
    }

    /// The frame's per-frame slot: the layer becomes the current task and
    /// its callbacks run in order.
    pub fn update(&mut self) {
        self.frames += 1;
        self.callbacks.run();
    }

    /// The form's own saved focus and its next-focus form, as the three
    /// hooks of the form base class see them. Returns the form to select
    /// and the control, if the form should give the focus back.
    pub fn close_focus(&mut self, form: FormId, saved_visible: bool) -> (Option<FormId>, i32) {
        self.focus.restore(form, saved_visible)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layer() -> TaskLayer {
        TaskLayer::new(1, (800, 600))
    }

    #[test]
    fn modifier_scores_count_held_modifiers_and_refuse_missing_ones() {
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
    }

    #[test]
    fn the_best_modifier_match_wins_and_a_tie_keeps_the_first() {
        let mut task = layer();
        task.add_shortcut(b'a', 0, 1);
        let plain = task.add_shortcut(b'a', modifier::SHIFT, 2);
        let both = task.add_shortcut(b'a', modifier::SHIFT | modifier::CONTROL, 3);
        // With Shift and Control held the both-modifier entry scores 2.
        assert_eq!(
            task.best_shortcut(b'a', modifier::SHIFT | modifier::CONTROL),
            Some((both, 2))
        );
        // With Shift and Alt held the Shift entry wins (1) over the plain
        // one, which never scores above zero.
        assert_eq!(
            task.best_shortcut(b'a', modifier::SHIFT | modifier::ALT),
            Some((plain, 1))
        );
        // With nothing held no entry qualifies.
        assert_eq!(task.best_shortcut(b'a', 0), None);
        // A different key has no entry.
        assert_eq!(task.best_shortcut(b'b', 0xff), None);
        // Equal scores keep the first entry.
        task.add_shortcut(b'c', modifier::SHIFT, 4);
        task.add_shortcut(b'c', modifier::SHIFT, 5);
        assert_eq!(task.best_shortcut(b'c', modifier::SHIFT), Some((3, 1)));
    }

    #[test]
    fn the_key_slot_fires_the_best_shortcut_or_forwards() {
        let mut task = layer();
        task.add_shortcut(b'k', modifier::CONTROL, 0x42);
        assert_eq!(
            task.key(b'k', modifier::CONTROL, 0),
            KeyOutcome::Shortcut {
                index: 0,
                action: 0x42
            }
        );
        // No modifier held: nothing fires, a non-root layer forwards.
        assert_eq!(task.key(b'k', 0, 0), KeyOutcome::Forward);
        // Tab is never a shortcut; a repeat is ignored.
        assert_eq!(task.key(NO_SHORTCUT_KEY, 0xff, 0), KeyOutcome::Ignored);
        assert_eq!(task.key(b'k', modifier::CONTROL, 1), KeyOutcome::Ignored);
        // The root has nobody to forward to.
        task.root = true;
        assert_eq!(task.key(b'k', 0, 0), KeyOutcome::Ignored);
    }

    #[test]
    fn only_the_active_and_root_layers_draw_full_strength() {
        let mut task = layer();
        assert_eq!(task.draw_alpha(), INACTIVE_ALPHA);
        task.active = true;
        assert_eq!(task.draw_alpha(), ACTIVE_ALPHA);
        task.active = false;
        task.root = true;
        assert_eq!(task.draw_alpha(), ACTIVE_ALPHA);
    }

    #[test]
    fn forms_are_drawn_visible_only_and_hit_from_the_top_down() {
        let mut task = layer();
        let a = task.add_form(OwnedForm::new(10, GuiRect::new(0, 0, 100, 100)));
        let b = task.add_form(OwnedForm::new(11, GuiRect::new(50, 50, 100, 100)));
        let c = task.add_form(OwnedForm::new(12, GuiRect::new(200, 200, 10, 10)));
        task.forms[2].visible = false;
        assert_eq!(task.draw_order(), vec![a, b]);
        // The top form gets the point first; it answers 0, so the one below
        // is asked.
        assert_eq!(
            task.hit(60, 60, |form, _, _| if form == b { 0 } else { 7 }),
            Some((a, 7))
        );
        assert_eq!(task.hit(210, 210, |_, _, _| 1), None);
        // A form whose state has bit 1 is skipped.
        task.forms[1].state = NO_HIT;
        assert_eq!(task.hit(60, 60, |_, _, _| 1), Some((a, 1)));
        // The whole layer can be kept out of the hit test.
        task.state = NO_HIT;
        assert_eq!(task.hit(10, 10, |_, _, _| 1), None);
        assert_eq!(c, 12);
    }

    #[test]
    fn raising_a_word_accumulates_until_read() {
        let mut task = layer();
        task.raise(0x40);
        task.raise(0x2);
        assert_eq!(task.take_notices(), 0x42);
        assert_eq!(task.take_notices(), 0);
    }

    #[test]
    fn frames_run_the_callbacks_and_closing_leaves_the_list() {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicU32, Ordering};
        let mut task = layer();
        let count = Arc::new(AtomicU32::new(0));
        let first = count.clone();
        let second = count.clone();
        let a = task.callbacks.add(move || {
            first.fetch_add(1, Ordering::SeqCst);
        });
        task.callbacks.add(move || {
            second.fetch_add(10, Ordering::SeqCst);
        });
        task.update();
        task.update();
        assert_eq!(task.frames, 2);
        assert_eq!(count.load(Ordering::SeqCst), 22);
        task.callbacks.remove(a);
        task.update();
        assert_eq!(count.load(Ordering::SeqCst), 32);
        let id = task.add_form(OwnedForm::new(20, GuiRect::new(0, 0, 1, 1)));
        assert!(task.remove_form(id));
        assert!(!task.remove_form(id));
        assert!(task.forms.is_empty());
        task.callbacks.clear();
        assert!(task.callbacks.is_empty());
    }

    #[test]
    fn the_focus_hooks_save_restore_and_clear() {
        let mut focus = TaskFocus::default();
        focus.activate(5, 3);
        assert_eq!(focus.current, Some((5, 3)));
        focus.activate(6, 1);
        assert_eq!(focus.saved, Some((5, 3)));
        // The saved form still shows: it takes the focus back with its
        // saved control.
        assert_eq!(focus.restore(6, true), (Some(5), 3));
        // It is gone: the next form (or the closing form) gets control −1.
        focus.next = Some(9);
        assert_eq!(focus.restore(6, false), (Some(9), -1));
        // Freeing the next-focus form clears it and drops the focus.
        focus.closed(9);
        assert_eq!(focus.next, None);
        assert_eq!(focus.current, None);
        // Freeing the saved form just forgets the saved record.
        focus.saved = Some((7, 2));
        focus.closed(7);
        assert_eq!(focus.saved, None);
    }

    #[test]
    fn the_layer_constructor_matches_the_original() {
        let task = layer();
        assert_eq!(task.bounds, GuiRect::new(0, 0, 800, 600));
        assert_eq!(LAYER_STYLE, 0x000f_0000);
        assert_eq!(LAYER_DRAW_MODE, 0x10);
        assert_eq!(task.shortcuts.len(), 0);
        assert_eq!(task.notices, 0);
        assert!(!task.active && !task.root);
    }
}

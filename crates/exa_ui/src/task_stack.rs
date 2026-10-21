//! The GUI task stack: an editor task registered as a layer that removes
//! itself on close, moved to the top when focused, and the active task the
//! player controls read.
//!
//! Each stack entry pairs a task interface (its menu items) with the layer
//! form created for it. Pushing appends at the top. Focusing a task moves
//! its pair to the last slot and publishes its interface. Removing a layer
//! frees its entry and, when it was the active one, clears the published
//! interface back to the root view. The task's menu items become 0x68-byte
//! control records, the first one losing the `0x8009` id it starts with.
//!
//! Evidence: the task interface's destroy and menu-item append and the
//! taskbar's remove, activate, refresh, push, init and finaliser routines
//! (`docs/UI.md`, the GUI taskbar).

/// The id and type of a task menu record.
pub const MENU_RECORD_TYPE: u16 = 0xab02;
pub const MENU_RECORD_ID: u16 = 0x8009;
/// The record's x word.
pub const MENU_RECORD_X: u16 = 2;
/// The record's height word.
pub const MENU_RECORD_HEIGHT: u16 = 2;

/// One task menu item: its label and callback.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMenuItem {
    pub name: String,
    pub callback: u64,
}

/// A task menu record (the 0x68-byte control record the taskbar appends).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskRecord {
    pub kind: u16,
    pub id: u16,
    pub x: u16,
    pub height: u16,
    pub text: String,
    pub callback: u64,
}

/// A task interface (`TTaskInterface`): the task's menu items.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskInterface {
    pub items: Vec<TaskMenuItem>,
}

impl TaskInterface {
    /// The destroy slot: releases every item's name.
    pub fn destroy(&mut self) {
        self.items.clear();
    }

    /// The menu-item slot: appends the task's items to `records` and
    /// returns the new count. A run that starts at zero clears the first
    /// record's `0x8009` id.
    pub fn append_records(&self, records: &mut Vec<TaskRecord>) -> usize {
        let start = records.len();
        for item in &self.items {
            records.push(TaskRecord {
                kind: MENU_RECORD_TYPE,
                id: MENU_RECORD_ID,
                x: MENU_RECORD_X,
                height: MENU_RECORD_HEIGHT,
                text: item.name.clone(),
                callback: item.callback,
            });
        }
        let count = records.len();
        if count > 0 && start == 0 {
            records[0].id = 0;
        }
        count
    }

    /// The dormant slot: no.
    pub fn accepts_none(&self) -> bool {
        false
    }
}

/// One stack entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskEntry {
    /// The layer's form handle.
    pub form: u64,
    /// The task interface's handle.
    pub interface: u64,
}

/// The active task published to the player controls and the AI.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ActiveTask {
    pub interface: u64,
    /// The task's view object (its `+0x170` field).
    pub view: u64,
    pub form: u64,
}

/// The task stack (`RF_GUI_Taskbar`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskStack {
    pub entries: Vec<TaskEntry>,
    pub active: ActiveTask,
}

impl TaskStack {
    /// The push: appends the task and makes its layer remove itself on
    /// close. Returns the layer form's handle.
    pub fn push(&mut self, form: u64, interface: u64) -> u64 {
        self.entries.push(TaskEntry { form, interface });
        form
    }

    /// The activation hook: a focused layer moves to the top and its task
    /// is published.
    pub fn activate(&mut self, form: u64, view: u64) -> bool {
        let entry = self
            .entries
            .iter()
            .position(|e| e.form == form)
            .map(|at| self.entries.remove(at));
        if let Some(entry) = entry {
            let interface = entry.interface;
            self.entries.push(entry);
            self.active.interface = interface;
            self.active.view = view;
            if self.active.form != form {
                self.active.form = form;
            }
            return true;
        }
        self.active.view = view;
        false
    }

    /// The remove-on-close: takes the layer out and, when it was active,
    /// clears the published task back to the root.
    pub fn remove(&mut self, form: u64, root: u64) -> bool {
        let Some(at) = self.entries.iter().position(|e| e.form == form) else {
            return false;
        };
        self.entries.remove(at);
        if self.active.form == form {
            self.active = ActiveTask {
                interface: 0,
                view: 0,
                form: root,
            };
        }
        true
    }

    /// Refreshes the active view from the active form.
    pub fn refresh(&mut self, view: u64) {
        self.active.view = view;
    }

    /// The finaliser.
    pub fn finish(&mut self) {
        self.entries.clear();
    }
}

/// The form field the task's view comes from.
pub const TASK_VIEW_FIELD: u64 = 0x170;

#[cfg(test)]
mod tests {
    use super::*;

    fn menu(name: &str, callback: u64) -> TaskMenuItem {
        TaskMenuItem {
            name: name.into(),
            callback,
        }
    }

    #[test]
    fn the_menu_items_append_as_records() {
        let mut task = TaskInterface {
            items: vec![menu("One", 1), menu("Two", 2)],
        };
        let mut records = Vec::new();
        assert_eq!(task.append_records(&mut records), 2);
        assert_eq!(records[0].kind, 0xab02);
        assert_eq!(records[0].id, 0);
        assert_eq!(records[0].x, 2);
        assert_eq!(records[0].height, 2);
        assert_eq!(records[0].text, "One");
        assert_eq!(records[1].id, 0x8009);
        // Appending again keeps the existing first record untouched.
        let mut later = vec![TaskRecord {
            kind: 9,
            id: 9,
            x: 9,
            height: 9,
            text: String::new(),
            callback: 0,
        }];
        assert_eq!(task.append_records(&mut later), 3);
        assert_eq!(later[0].id, 9);
        assert_eq!(later[1].id, 0x8009);
        assert!(!task.accepts_none());
        task.destroy();
        assert!(task.items.is_empty());
    }

    #[test]
    fn focusing_moves_a_task_to_the_top() {
        let mut stack = TaskStack::default();
        stack.push(1, 11);
        stack.push(2, 22);
        stack.push(3, 33);
        assert!(stack.activate(1, 100));
        assert_eq!(stack.entries.last().unwrap().form, 1);
        assert_eq!(stack.active.interface, 11);
        assert_eq!(stack.active.view, 100);
        assert_eq!(stack.active.form, 1);
        assert!(!stack.activate(9, 5));
        assert_eq!(stack.active.view, 5);
    }

    #[test]
    fn removing_the_active_task_clears_it() {
        let mut stack = TaskStack::default();
        stack.push(1, 11);
        stack.push(2, 22);
        stack.activate(2, 50);
        assert!(stack.remove(2, 77));
        assert_eq!(
            stack.active,
            ActiveTask {
                interface: 0,
                view: 0,
                form: 77
            }
        );
        assert!(!stack.remove(2, 77));
        assert_eq!(stack.entries.len(), 1);
        stack.finish();
        assert!(stack.entries.is_empty());
    }
}

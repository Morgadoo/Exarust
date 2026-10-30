//! The dialogue window: the reply panel as the conversation engine fills it.
//!
//! The engine keeps the entries — the replies of a running conversation,
//! the topics of every dialogue the two characters can hold, and the
//! `[Other]` toggle — this module turns them into the panel's view. Each
//! text is measured at the panel's width less eight units, the `[Other]`
//! entry is appended unmeasured (its line count stays whatever the fresh
//! slot holds, zero), and the height is laid out once per change so a
//! running height animation is not restarted every frame.
//!
//! The panel's geometry, hit test, height animation and draw data are
//! [`crate::dialogue_choices`]; the engine's entries and their selection
//! are [`exa_script::dialogue`]'s.
//!
//! Evidence: the add-entry, `[Other]` and height routines of the dialogue
//! unit (REA on `Exanima.exe` 0.9.5).

use exa_script::bubbles::Speaker;
use exa_script::dialogue::{Choice, Panel};

use crate::dialogue_choices::{ChoicesView, PANEL_WIDTH};
use crate::gui_font::GuiFont;

/// The `[Other]` entry's topic word.
pub const OTHER_TOPIC: u32 = 0xFFFF;

/// The reply panel of the running conversation, synced from the engine.
#[derive(Debug, Clone, Default)]
pub struct DialogueWindow {
    pub view: ChoicesView,
    /// The panel is up with entries to show.
    pub open: bool,
    /// The narrator is the partner: its height is taken at once and its
    /// minimum is 17 units rather than 30.
    pub narrator: bool,
    /// The engine entries the view was built from, to notice a change.
    entries: Vec<Choice>,
    /// The narrator flag the view was laid out with.
    laid_out_as_narrator: bool,
}

impl DialogueWindow {
    /// A closed window.
    pub fn new() -> Self {
        Self::default()
    }

    /// Syncs the panel: opens it when the engine has entries, closes it
    /// (and clears the view) otherwise. The entry list is rebuilt, measured
    /// and laid out only when it changed. Returns whether it changed.
    pub fn sync(
        &mut self,
        panel: &Panel,
        font: &GuiFont,
        unit: i32,
        parent_height: i32,
        now: u32,
    ) -> bool {
        let narrator = panel.partner == Some(Speaker::Narrator);
        if !panel.open || panel.entries.is_empty() {
            self.narrator = narrator;
            self.entries.clear();
            self.view.entries.clear();
            self.open = false;
            return false;
        }
        self.open = true;
        self.narrator = narrator;
        let changed = self.entries != panel.entries || self.laid_out_as_narrator != narrator;
        if changed {
            self.view.entries.clear();
            for choice in &panel.entries {
                if choice.topic == OTHER_TOPIC {
                    self.view.push_other();
                } else {
                    let text = String::from_utf8_lossy(&choice.text);
                    self.view
                        .push(&text, choice.topic, choice.branch, font, PANEL_WIDTH, unit);
                }
            }
            self.view.layout(font, unit, parent_height, narrator, now);
            self.view.dirty = false;
            self.entries = panel.entries.clone();
            self.laid_out_as_narrator = narrator;
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_08 as FONT;

    fn choice(text: &str, topic: u32, branch: usize) -> Choice {
        Choice {
            text: text.as_bytes().to_vec(),
            topic,
            branch,
        }
    }

    fn panel(entries: Vec<Choice>, partner: Option<Speaker>) -> Panel {
        Panel {
            open: true,
            partner,
            entries,
            ..Panel::default()
        }
    }

    #[test]
    fn the_entries_are_measured_and_the_other_toggle_is_appended() {
        let mut window = DialogueWindow::new();
        let p = panel(
            vec![choice("One", 0, 2), choice("[Other]", OTHER_TOPIC, 0)],
            Some(Speaker::Character(5)),
        );
        assert!(window.sync(&p, &FONT, 4, 600, 1000));
        assert!(window.open && !window.narrator);
        assert_eq!(window.view.entries.len(), 2);
        let e = &window.view.entries[0];
        assert_eq!((e.lines, e.topic, e.branch, e.other), (1, 0, 2, false));
        let o = &window.view.entries[1];
        // The toggle is appended without measuring its text.
        assert_eq!((o.lines, o.topic, o.other), (0, OTHER_TOPIC, true));
        // 8 + (16·1 + 4) + (16·0 + 4) = 32, under the 120 minimum.
        assert_eq!(window.view.target, 120);
        assert_eq!(window.view.anim_end, 1500);
    }

    #[test]
    fn a_narrator_panel_takes_its_height_at_once() {
        let mut window = DialogueWindow::new();
        let p = panel(vec![choice("One", 1, 0)], Some(Speaker::Narrator));
        assert!(window.sync(&p, &FONT, 4, 600, 1000));
        assert!(window.narrator);
        // 8 + 20 = 28, under the narrator's 17-unit minimum (68).
        assert_eq!(
            (window.view.height, window.view.base, window.view.target),
            (68, 68, 68)
        );
        assert_eq!(window.view.y, 600 - 68 - 32);
        assert_eq!(window.view.anim_end, 0);
    }

    #[test]
    fn an_unchanged_panel_keeps_its_animation_and_a_closed_one_clears_it() {
        let mut window = DialogueWindow::new();
        let mut p = panel(vec![choice("One", 0, 0)], Some(Speaker::Character(5)));
        assert!(window.sync(&p, &FONT, 4, 600, 1000));
        // The same list again changes nothing: the animation clock stays.
        assert!(!window.sync(&p, &FONT, 4, 600, 1200));
        assert_eq!(window.view.anim_end, 1500);
        // Another line rebuilds and restarts the animation.
        p.entries.push(choice("Two", 3, 9));
        assert!(window.sync(&p, &FONT, 4, 600, 2000));
        assert_eq!(window.view.entries.len(), 2);
        assert_eq!(window.view.anim_end, 2500);
        assert_eq!(window.view.entries[1].topic, 3);
        assert_eq!(window.view.entries[1].branch, 9);
        // The engine closes the panel: the window clears.
        p.open = false;
        assert!(!window.sync(&p, &FONT, 4, 600, 3000));
        assert!(!window.open);
        assert!(window.view.entries.is_empty());
    }
}

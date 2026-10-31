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
//! The panel is one form, made the first time the player talks: 400 by 120,
//! centred across its parent and seated 120 + 32 pixels above the parent's
//! bottom. It is shown while the engine's panel is open, also while its list
//! is empty (a conversation step empties the list and the pick but leaves
//! the form up, so the box stays while the partner speaks), and hidden when
//! the engine closes it. The engine's pick (its entry number from one) is
//! the panel's pick: when it turns on, the panel holds it for a second.
//!
//! **The fade.** The constructor leaves the form hidden, so the talk's show
//! starts the base form's show fade (256 GUI clock ticks, mode 0) and the
//! close starts its hide fade; while one runs the draw hands its alpha to
//! the skin as both scales (the box at that alpha times 160, the inks'
//! alphas scaled with the palette) and sets them back to 1 after. A show
//! asked for while the form is still visible (inside a hide fade) does not
//! restart the fade, as the original's show tests the visible flag; the
//! hide then ends and the form goes. The rows the form held stay through
//! the hide fade ([`DialogueWindow::fade_alpha`]).
//!
//! **The gamepad.** A panel the gamepad's Talk opened highlights its first
//! entry on the open and on every layout of a non-empty list. The pad's
//! vertical steps move the highlight (the form's hover itself, which the
//! pointer also sets): down the list while it is below the count, from none
//! to the first; up while it is above the first. Select picks the
//! highlighted entry when it is one of the list's ([`DialogueWindow::step_highlight`],
//! [`DialogueWindow::highlighted`]).
//!
//! A frame's draw ([`DialogueWindow::paint`]) eases the height, then draws
//! a rounded box of radius two at alpha 160 and, while the list has entries,
//! each row's "N." and its wrapped text in the main GUI font: the hovered
//! or held row in the highlight ink (its alpha raised by half again), the
//! others in the label ink, both fading with the held pick.
//!
//! The panel's geometry, hit test, height animation and draw data are
//! [`crate::dialogue_choices`]; the engine's entries and their selection
//! are [`exa_script::dialogue`]'s.
//!
//! Evidence: the talk routine (the panel's creation and placement), the
//! panel's constructor, add-entry, `[Other]`, height, hit test, input and
//! draw routines, and the conversation step that empties the list (REA on
//! `Exanima.exe` 0.9.5).

use exa_script::bubbles::Speaker;
use exa_script::dialogue::{Choice, Panel};

use crate::dialogue_choices::{
    BOX_RADIUS, CREATE_GAP, ChoicePalette, ChoicesView, FontMetrics, PANEL_HEIGHT, PANEL_WIDTH,
};
use crate::form::{FadeStep, FormFade};
use crate::gui_font::GuiFont;
use crate::gui_text::{TextQuad, draw_line, draw_wrapped};
use crate::skin::{GuiRect, SkinBatch, rounded_fill_fixed};

/// The `[Other]` entry's topic word.
pub const OTHER_TOPIC: u32 = 0xFFFF;

/// The reply panel of the running conversation, synced from the engine.
#[derive(Debug, Clone, Default)]
pub struct DialogueWindow {
    pub view: ChoicesView,
    /// The form is shown (the engine's panel is open).
    pub open: bool,
    /// The narrator is the partner: its height is taken at once and its
    /// minimum is 17 units rather than 30.
    pub narrator: bool,
    /// The form has been made (the first talk places it).
    pub created: bool,
    /// The form's x in its parent, set when it is made.
    pub x: i32,
    /// The engine entries the view was built from, to notice a change.
    entries: Vec<Choice>,
    /// The narrator flag the view was laid out with.
    laid_out_as_narrator: bool,
    /// The engine's pick the view last mirrored.
    chosen: usize,
    /// The base form's show and hide fade (GUI clock).
    pub fade: FormFade,
    /// The form's visible flag: set by the show, cleared when a hide fade
    /// ends.
    pub visible: bool,
}

/// One frame of the panel's draw: the box, then the rows' glyphs in their
/// two inks.
#[derive(Debug, Clone, Default)]
pub struct PanelPaint {
    /// The box: x, y, width, height.
    pub rect: [i32; 4],
    pub box_alpha: u8,
    /// The rows drawn in the ordinary ink and in the highlight ink.
    pub normal: Vec<TextQuad>,
    pub highlight: Vec<TextQuad>,
    /// The two inks, as `[r, g, b, a]`.
    pub normal_ink: [u8; 4],
    pub highlight_ink: [u8; 4],
}

impl PanelPaint {
    /// The box as one rounded-fill batch.
    pub fn box_batch(&self, unit: i32) -> SkinBatch {
        let mut batch = SkinBatch::new();
        let [x, y, w, h] = self.rect;
        rounded_fill_fixed(
            &mut batch,
            GuiRect::sized(x as f32, y as f32, w as f32, h as f32),
            unit,
            BOX_RADIUS,
            self.box_alpha,
        );
        batch
    }
}

impl DialogueWindow {
    /// A closed window.
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes the form if it has not been made: 400 by 120, centred across
    /// the parent and 32 pixels plus its height above the parent's bottom.
    /// Later talks reuse it where it is.
    pub fn create(&mut self, parent_width: i32, parent_height: i32) {
        if self.created {
            return;
        }
        self.created = true;
        self.x = (parent_width - PANEL_WIDTH) / 2;
        self.view.height = PANEL_HEIGHT;
        self.view.y = parent_height - PANEL_HEIGHT - CREATE_GAP;
    }

    /// Syncs the panel: shown while the engine's panel is open (an empty
    /// list leaves only the box), hidden and cleared when it closes. The
    /// entry list is rebuilt, measured and laid out only when it changed;
    /// the engine's pick turning on starts the panel's one-second hold.
    /// Returns whether the list changed.
    pub fn sync(
        &mut self,
        panel: &Panel,
        font: &GuiFont,
        unit: i32,
        parent_height: i32,
        now: u32,
    ) -> bool {
        let narrator = panel.partner == Some(Speaker::Narrator);
        self.narrator = narrator;
        let opening = panel.open && !self.open;
        self.open = panel.open;
        if !panel.open {
            // The close only hides the form: its rows stay for the hide
            // fade, and the next open rebuilds them.
            return false;
        }
        let changed =
            opening || self.entries != panel.entries || self.laid_out_as_narrator != narrator;
        if changed {
            self.view.entries.clear();
            for choice in &panel.entries {
                if choice.topic == OTHER_TOPIC {
                    self.view.push_other();
                } else {
                    self.view.push(
                        &choice.text,
                        choice.topic,
                        choice.branch,
                        font,
                        PANEL_WIDTH,
                        unit,
                    );
                }
            }
            self.view.layout(font, unit, parent_height, narrator, now);
            self.view.dirty = false;
            self.entries = panel.entries.clone();
            self.laid_out_as_narrator = narrator;
            // The layout of a list with entries highlights the first when
            // the gamepad opened the panel.
            if panel.by_pad && !self.view.entries.is_empty() {
                self.view.hover = 1;
            }
        }
        if panel.chosen != self.chosen {
            self.chosen = panel.chosen;
            if panel.chosen > 0 {
                self.view.pick(panel.chosen - 1, now);
            } else {
                self.view.picked = 0;
            }
        }
        changed
    }

    /// The pointer moved to `(x, y)` (parent pixels): over the panel the hit
    /// test's entry number from one becomes the hover, elsewhere the hover
    /// clears. Returns whether the pointer is over the shown panel.
    pub fn point(&mut self, x: i32, y: i32, unit: i32) -> bool {
        let inside = self.contains(x, y);
        self.view.hover = if inside {
            self.view.hit(-1, true, y - self.view.y, unit)
        } else {
            0
        };
        inside
    }

    /// Whether `(x, y)` lies on the shown panel.
    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.open
            && x >= self.x
            && x < self.x + PANEL_WIDTH
            && y >= self.view.y
            && y < self.view.y + self.view.height
    }

    /// A left click released over the panel: the hovered entry it picks
    /// (from zero), as the panel's input answers a click.
    pub fn click(&self) -> Option<usize> {
        self.view.input(0x2001, 0).select
    }

    /// The gamepad's step: `forward` moves the highlight down the list while
    /// it is below the count (from none to the first), otherwise up while it
    /// is above the first. Nothing moves while the panel is closed.
    pub fn step_highlight(&mut self, forward: bool) {
        if !self.open {
            return;
        }
        let hover = self.view.hover;
        if forward {
            if hover < self.view.entries.len() as i32 {
                self.view.hover = (hover + 1).max(1);
            }
        } else if hover > 1 {
            self.view.hover = hover - 1;
        }
    }

    /// The entry the gamepad's Select picks (from zero): the highlighted
    /// one, when it is one of the list's.
    pub fn highlighted(&self) -> Option<usize> {
        let hover = self.view.hover;
        (hover > 0 && hover <= self.view.entries.len() as i32).then(|| hover as usize - 1)
    }

    /// The talk's show at GUI clock `now`: a hidden form starts its show
    /// fade and becomes visible; a visible one (inside its hide fade) keeps
    /// the fade it has.
    pub fn show(&mut self, now: u32) {
        if self.fade.start_show(now, self.visible) {
            self.visible = true;
        }
    }

    /// The close's hide at GUI clock `now`: the hide fade starts unless one
    /// runs.
    pub fn hide(&mut self, now: u32) {
        self.fade.start_hide(now);
    }

    /// The alpha the form draws at, at GUI clock `now`, or `None` when it is
    /// not drawn (hidden, or its hide fade has just ended, which hides it).
    /// Outside a fade the alpha is 1.
    pub fn fade_alpha(&mut self, now: u32) -> Option<f32> {
        if !self.visible {
            return None;
        }
        match self.fade.step(now) {
            FadeStep::Close => {
                self.visible = false;
                None
            }
            FadeStep::Draw(alpha) => Some(alpha.unwrap_or(1.0)),
        }
    }

    /// One frame of the draw at `now`: the height eased toward its target
    /// (the panel moving to stay above the parent's bottom), the box at the
    /// interface alpha `ui_alpha`, and the rows while the list has entries.
    #[allow(clippy::too_many_arguments)]
    pub fn paint(
        &mut self,
        font: &GuiFont,
        palette: &ChoicePalette,
        unit: i32,
        parent_height: i32,
        ui_alpha: f32,
        now: u32,
    ) -> PanelPaint {
        self.view.step_height(now, parent_height, unit);
        let colours = self.view.colours(palette, now);
        let mut out = PanelPaint {
            rect: [self.x, self.view.y, PANEL_WIDTH, self.view.height],
            box_alpha: (ChoicesView::box_alpha(ui_alpha) & 0xff) as u8,
            normal_ink: colours.normal.to_le_bytes(),
            highlight_ink: colours.highlight.to_le_bytes(),
            ..PanelPaint::default()
        };
        let metrics = FontMetrics {
            line_height: font.line_height,
            text_offset: font.glyph.1,
        };
        for row in self.view.rows(PANEL_WIDTH, unit, &metrics, now) {
            let quads = if row.highlighted {
                &mut out.highlight
            } else {
                &mut out.normal
            };
            quads.extend(draw_line(
                font,
                row.prefix.as_bytes(),
                self.x + row.prefix_at[0],
                self.view.y + row.prefix_at[1],
            ));
            let [bx, by, bw, bh] = row.body;
            quads.extend(
                draw_wrapped(
                    font,
                    &row.text,
                    self.x + bx,
                    self.view.y + by,
                    bw,
                    bh,
                    0,
                    false,
                )
                .quads,
            );
        }
        out
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
        // A step empties the list: the form stays up with its box.
        p.entries.clear();
        assert!(window.sync(&p, &FONT, 4, 600, 2600));
        assert!(window.open);
        assert!(window.view.entries.is_empty());
        // The engine closes the panel: the window hides, keeping what it
        // shows for the hide fade.
        p.entries.push(choice("Three", 0, 1));
        window.sync(&p, &FONT, 4, 600, 2700);
        p.open = false;
        assert!(!window.sync(&p, &FONT, 4, 600, 3000));
        assert!(!window.open);
        assert_eq!(window.view.entries.len(), 1);
        // The next open rebuilds even an identical list.
        p.open = true;
        assert!(window.sync(&p, &FONT, 4, 600, 4000));
        assert_eq!(window.view.anim_end, 4500);
    }

    /// A pad-opened panel highlights its first entry on every layout; the
    /// pad's steps move the highlight within the list and Select takes it.
    #[test]
    fn the_pad_highlight_starts_on_the_first_entry_and_steps() {
        let mut window = DialogueWindow::new();
        let mut p = panel(
            vec![
                choice("One", 0, 0),
                choice("Two", 0, 1),
                choice("Three", 0, 2),
            ],
            Some(Speaker::Character(5)),
        );
        p.by_pad = true;
        window.sync(&p, &FONT, 4, 600, 0);
        assert_eq!(window.view.hover, 1);
        assert_eq!(window.highlighted(), Some(0));
        window.step_highlight(false);
        assert_eq!(window.view.hover, 1, "the first stays first");
        for want in [2, 3, 3] {
            window.step_highlight(true);
            assert_eq!(window.view.hover, want);
        }
        assert_eq!(window.highlighted(), Some(2));
        window.step_highlight(false);
        assert_eq!(window.view.hover, 2);
        // A new list goes back to the first.
        p.entries.pop();
        window.sync(&p, &FONT, 4, 600, 100);
        assert_eq!(window.view.hover, 1);
        // From the pointer's "none" (or the form's -1 above the bands) a
        // step down takes the first; Select has nothing there.
        window.view.hover = -1;
        assert_eq!(window.highlighted(), None);
        window.step_highlight(true);
        assert_eq!(window.view.hover, 1);
        // An empty list keeps the hover; a mouse-opened panel never sets it.
        p.entries.clear();
        window.view.hover = 0;
        window.sync(&p, &FONT, 4, 600, 200);
        assert_eq!(window.view.hover, 0);
        let mut mouse = panel(vec![choice("One", 0, 0)], Some(Speaker::Character(5)));
        mouse.by_pad = false;
        let mut other = DialogueWindow::new();
        other.sync(&mouse, &FONT, 4, 600, 0);
        assert_eq!(other.view.hover, 0);
        // Closed, the steps do nothing.
        mouse.open = false;
        other.sync(&mouse, &FONT, 4, 600, 10);
        other.step_highlight(true);
        assert_eq!(other.view.hover, 0);
    }

    /// The show fades the form in over 256 ticks, the hide out over 256
    /// and then hides it; a show inside the hide fade does not restart it.
    #[test]
    fn the_form_fades_in_and_out() {
        let mut window = DialogueWindow::new();
        assert_eq!(window.fade_alpha(0), None, "made hidden");
        window.show(1000);
        assert_eq!(window.fade_alpha(1000), Some(0.0));
        assert_eq!(window.fade_alpha(1064), Some(0.25));
        assert_eq!(window.fade_alpha(1256), Some(1.0));
        assert_eq!(window.fade_alpha(5000), Some(1.0));
        // A second show while visible changes nothing.
        window.show(5000);
        assert_eq!(window.fade_alpha(5001), Some(1.0));
        window.hide(6000);
        assert_eq!(window.fade_alpha(6064), Some(0.75));
        // A show inside the hide fade: still visible, so no new fade.
        window.show(6100);
        assert_eq!(window.fade_alpha(6192), Some(0.25));
        assert_eq!(window.fade_alpha(6256), None);
        assert!(!window.visible);
        // Hidden now, the next show fades in again.
        window.show(7000);
        assert_eq!(window.fade_alpha(7128), Some(0.5));
    }

    /// The first talk makes the form: 400 × 120, centred on a 1280 × 720
    /// parent, its top 720 − 120 − 32 = 568; the first list then eases the
    /// height from those 120 pixels.
    #[test]
    fn the_first_talk_places_the_form() {
        let mut window = DialogueWindow::new();
        window.create(1280, 720);
        assert_eq!(
            (window.x, window.view.y, window.view.height),
            (440, 568, 120)
        );
        // Made once: a later talk on another parent leaves it in place.
        window.create(800, 600);
        assert_eq!((window.x, window.view.y), (440, 568));
        let lines: Vec<Choice> = (0..6)
            .map(|i| {
                choice(
                    "aaaa aaaa aaaa aaaa aaaa aaaa aaaa aaaa aaaa aaaa aaaa",
                    i,
                    0,
                )
            })
            .collect();
        let p = panel(lines, Some(Speaker::Character(5)));
        window.sync(&p, &FONT, 4, 720, 1000);
        // 8 + 6·(16·2 + 4) = 224 above the 120 minimum; the base is the
        // 120 the form was made with.
        assert_eq!((window.view.base, window.view.target), (120, 224));
        // Half way through the 500 ms the eased share is a half: 172.
        window.view.step_height(1250, 720, 4);
        assert_eq!(window.view.height, 172);
        assert_eq!(window.view.y, 720 - 172 - 32);
    }

    /// The engine's pick starts the one-second hold; a step's reset to
    /// zero drops it, and the next pick of the same number starts it again.
    #[test]
    fn the_engines_pick_is_held_for_a_second() {
        let mut window = DialogueWindow::new();
        let mut p = panel(
            vec![choice("One", 0, 0), choice("Two", 0, 1)],
            Some(Speaker::Character(5)),
        );
        window.sync(&p, &FONT, 4, 600, 1000);
        p.chosen = 2;
        window.sync(&p, &FONT, 4, 600, 1100);
        assert_eq!((window.view.picked, window.view.pick_end), (2, 2100));
        // Unchanged: the hold keeps its end.
        window.sync(&p, &FONT, 4, 600, 1200);
        assert_eq!(window.view.pick_end, 2100);
        p.chosen = 0;
        window.sync(&p, &FONT, 4, 600, 1300);
        assert_eq!(window.view.picked, 0);
        p.chosen = 2;
        window.sync(&p, &FONT, 4, 600, 1400);
        assert_eq!((window.view.picked, window.view.pick_end), (2, 2400));
    }

    /// The pointer's hover is the hit test's band under it, the click picks
    /// the hovered entry, and off the panel the hover clears.
    #[test]
    fn the_pointer_hovers_and_clicks_the_bands() {
        let mut window = DialogueWindow::new();
        window.create(1280, 720);
        let p = panel(
            vec![choice("One", 0, 0), choice("Two", 0, 1)],
            Some(Speaker::Character(5)),
        );
        window.sync(&p, &FONT, 4, 720, 0);
        // The panel spans x 440..840 and y 568..688 (no step yet).
        assert!(!window.point(439, 600, 4));
        assert_eq!(window.view.hover, 0);
        assert_eq!(window.click(), None);
        // The first band is 4..24 below the top, the second 24..44.
        assert!(window.point(500, 568 + 10, 4));
        assert_eq!(window.view.hover, 1);
        assert_eq!(window.click(), Some(0));
        window.point(500, 568 + 30, 4);
        assert_eq!(window.click(), Some(1));
        // Above the first band and below the last: the form's own answer.
        window.point(500, 568 + 2, 4);
        assert_eq!(window.view.hover, -1);
        assert_eq!(window.click(), None);
        window.point(500, 568 + 60, 4);
        assert_eq!(window.click(), None);
    }

    /// The draw: the box at 160, the rows "N." a unit in and the text six
    /// units in, the hovered row in the highlight ink with its alpha raised
    /// by half again; an empty list draws only the box.
    #[test]
    fn the_paint_draws_the_box_and_the_rows() {
        let palette = ChoicePalette {
            highlight: 0x6400_80ff,
            normal: 0xdc10_2030,
            normal_while_picked: 0x8000_0001,
        };
        let mut window = DialogueWindow::new();
        window.create(1280, 720);
        let p = panel(
            vec![choice("One", 0, 0), choice("Two", 0, 1)],
            Some(Speaker::Character(5)),
        );
        window.sync(&p, &FONT, 4, 720, 0);
        window.point(500, 568 + 10, 4);
        // Past the animation the 120-pixel target holds: no change.
        let paint = window.paint(&FONT, &palette, 4, 720, 1.0, 600);
        assert_eq!(paint.rect, [440, 568, 400, 120]);
        assert_eq!(paint.box_alpha, 160);
        assert_eq!(paint.highlight_ink, [0xff, 0x80, 0x00, 0x96]);
        assert_eq!(paint.normal_ink, [0x30, 0x20, 0x10, 0xdc]);
        // "1." and "One" highlighted, "2." and "Two" not.
        assert_eq!(paint.highlight.len(), 5);
        assert_eq!(paint.normal.len(), 5);
        // The number sits a unit in, its bottom a line and a unit down.
        let first = &paint.highlight[0];
        assert_eq!(first.rect[0], 444);
        assert_eq!(first.rect[1] + first.rect[3], 568 + 16 + 4);
        // The text starts six units in.
        assert_eq!(paint.highlight[2].rect[0], 440 + 24);
        assert!(!paint.box_batch(4).vertices.is_empty());
        // An empty list (a conversation step) leaves the box alone.
        let mut empty = p.clone();
        empty.entries.clear();
        window.sync(&empty, &FONT, 4, 720, 700);
        let paint = window.paint(&FONT, &palette, 4, 720, 1.0, 700);
        assert!(paint.normal.is_empty() && paint.highlight.is_empty());
        assert_eq!(paint.rect[3], 120);
    }
}

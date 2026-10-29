//! The dormant game-object appearance editor's control handlers: the
//! sliders, check boxes, choice lists and colour cells that edit an item's
//! look record (four base bytes and eight twelve-byte layers), the colour
//! palette popup's placement and the labels the sliders show.
//!
//! The retail game never opens this editor (no code path builds its
//! form); these types have no live caller in Exarust and are kept, tested,
//! for completeness. The editor's form construction, preview rebuild and
//! item-container refresh are not reproduced. Rules: `docs/UI.md`
//! (Object editor).

use exa_formats::text::{format_float, format_hex};

/// The short value names the variation sliders show, by step.
pub const STEP_NAMES: [&str; 6] = ["0.0", "0.5", "1.0", "2.0", "3.0", "4.0"];
/// The first entry of every choice list the editor fills.
pub const RANDOM_CHOICE: &str = "<Random>";
/// The second entry of the preset list.
pub const NONE_CHOICE: &str = "None";
/// The number of look layers.
pub const LAYERS: usize = 8;
/// The control state bit of a checked check box.
pub const CHECKED: u32 = 0x40;
/// The control state bit of a control that is being set by the editor
/// itself (its handler does nothing).
pub const SETTING: u32 = 0x10_0000;
/// The palette popup's columns are this many cells tall.
pub const PALETTE_ROWS: u8 = 8;
/// A palette cell's size in GUI units.
pub const PALETTE_CELL_UNITS: u32 = 3;

/// One look layer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LookLayer {
    /// Bit 0: the layer takes part in the presets; bit 1: a second switch.
    pub flags: u8,
    pub amount: u8,
    pub material: u8,
    pub variant: u8,
    pub colour: u8,
}

/// The look record the editor edits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Look {
    /// Three 0 to 255 levels and a byte of three variation steps (bits
    /// 0-2 and 3-5 from 0 to 5, bits 6-7 from 0 to 3).
    pub base: [u8; 4],
    pub layers: [LookLayer; LAYERS],
}

/// What a handler asks of the editor around it.
#[derive(Debug, Clone, PartialEq)]
pub enum EditorEvent {
    /// The control's label is set to this text.
    Label(String),
    /// The look changed: rebuild the preview.
    Refresh,
    /// The look changed through the layer controls: rebuild the preview
    /// from the look record.
    Rebuild,
    /// The preset list is to be filled again.
    Presets,
    /// The material list is to be filled again for this material.
    Materials(u8),
    /// The item state's level setter runs with this value (the setter is
    /// outside this unit).
    Level(f32),
    /// The item state's balance setter runs with this value.
    Balance(f32),
    /// Control 7 (the seed field) shows this value.
    Seed(i32),
    /// The open item container's preview is refreshed.
    Container,
}

/// The editor's state the handlers read and write.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ObjectEditor {
    pub look: Look,
    /// The layer the layer controls edit.
    pub layer: usize,
    /// Positive while the editor fills its own controls: the handlers then
    /// leave the look alone.
    pub filling: i32,
    pub seed: i32,
    pub seeds: (i32, i32),
    /// The preset byte: 0xff random, 0 none, else a preset id.
    pub preset: u8,
    /// The ids of the preset list's entries after "<Random>" and "None".
    pub presets: Vec<u8>,
    /// The ids of the material list's entries after "<Random>".
    pub materials: Vec<u8>,
    /// An item container is open beside the editor.
    pub container: bool,
}

fn step(value: f32, steps: f32) -> u8 {
    (value * steps).round_ties_even() as i64 as u8
}

fn level(value: f32) -> u8 {
    (value * 255.0).round_ties_even() as i64 as u8
}

impl ObjectEditor {
    fn guarded(&self) -> bool {
        self.filling >= 1
    }

    fn current(&mut self) -> &mut LookLayer {
        let i = self.layer.min(LAYERS - 1);
        &mut self.look.layers[i]
    }

    /// The first level slider: the label is four times the value with two
    /// decimals; the byte is the value at 255 steps.
    pub fn first_level(&mut self, value: f32, state: u32) -> Vec<EditorEvent> {
        if state & SETTING != 0 {
            return Vec::new();
        }
        let mut events = vec![EditorEvent::Label(format_float(value * 4.0, 2, true))];
        if !self.guarded() {
            self.look.base[0] = level(value);
            events.push(EditorEvent::Refresh);
        }
        events
    }

    /// The second and third level sliders (`which` 1 or 2): the byte at
    /// 255 steps, written even while the editor fills its controls.
    pub fn other_level(&mut self, which: usize, value: f32, state: u32) -> Vec<EditorEvent> {
        if state & SETTING != 0 || !(1..=2).contains(&which) {
            return Vec::new();
        }
        self.look.base[which] = level(value);
        vec![EditorEvent::Refresh]
    }

    /// The three variation sliders: `slot` 0 and 1 take six steps (the
    /// labels "Var " and " Var " with the step's name), slot 2 four steps
    /// (" Var "), packed into the fourth base byte.
    pub fn variation(&mut self, slot: u8, value: f32, state: u32) -> Vec<EditorEvent> {
        if state & SETTING != 0 {
            return Vec::new();
        }
        let (steps, prefix, mask, shift) = match slot {
            0 => (5.0, "Var ", 0xf8u8, 0),
            1 => (5.0, " Var ", 0xc7, 3),
            _ => (3.0, " Var ", 0x3f, 6),
        };
        let s = step(value, steps);
        let name = STEP_NAMES.get(usize::from(s)).copied().unwrap_or("");
        let mut events = vec![EditorEvent::Label(format!("{prefix}{name}"))];
        if !self.guarded() {
            let b = &mut self.look.base[3];
            *b = (*b & mask) | (s << shift);
            events.push(EditorEvent::Refresh);
        }
        events
    }

    /// The layer selector (1-based value).
    pub fn select_layer(&mut self, value: f32) -> Vec<EditorEvent> {
        self.layer = ((value.round_ties_even() as i64) - 1).clamp(0, LAYERS as i64 - 1) as usize;
        vec![EditorEvent::Rebuild]
    }

    /// The layer's amount.
    pub fn amount(&mut self, value: f32) -> Vec<EditorEvent> {
        if self.guarded() {
            return Vec::new();
        }
        self.current().amount = value.round_ties_even() as i64 as u8;
        vec![EditorEvent::Rebuild]
    }

    /// The layer's first switch: bit 0 follows the check box; the preset
    /// list is filled again.
    pub fn first_switch(&mut self, state: u32) -> Vec<EditorEvent> {
        let l = self.current();
        if state & CHECKED == 0 {
            l.flags &= !1;
        } else {
            l.flags |= 1;
        }
        vec![EditorEvent::Rebuild, EditorEvent::Presets]
    }

    /// The layer's second switch: bit 1 follows the check box.
    pub fn second_switch(&mut self, state: u32) -> Vec<EditorEvent> {
        let l = self.current();
        if state & CHECKED == 0 {
            l.flags &= !2;
        } else {
            l.flags |= 2;
        }
        vec![EditorEvent::Rebuild]
    }

    /// The layer's material choice: the variant goes back to 0, the
    /// material list is filled again and, for a preset layer, the presets.
    pub fn material(&mut self, choice: i32) -> Vec<EditorEvent> {
        if self.guarded() {
            return Vec::new();
        }
        let l = self.current();
        l.material = choice as u8;
        l.variant = 0;
        let preset_layer = l.flags & 1 != 0;
        let mut events = vec![EditorEvent::Materials(choice as u8), EditorEvent::Refresh];
        if preset_layer {
            events.push(EditorEvent::Presets);
        }
        events
    }

    /// The layer's variant choice (1-based): the first entry is random
    /// (0), else the material list's id. The byte is stored even while the
    /// editor fills its controls; only the refresh waits.
    pub fn variant(&mut self, choice: i32) -> Vec<EditorEvent> {
        let id = if choice < 2 {
            0
        } else {
            self.materials
                .get(choice as usize - 2)
                .copied()
                .unwrap_or(0)
        };
        let guarded = self.guarded();
        let l = self.current();
        if id != l.variant {
            l.variant = id;
            if !guarded {
                return vec![EditorEvent::Refresh];
            }
        }
        Vec::new()
    }

    /// Sets the layer's colour: the colour cell shows it as two hex
    /// digits.
    pub fn set_colour(&mut self, colour: u8) -> Vec<EditorEvent> {
        self.current().colour = colour;
        vec![
            EditorEvent::Label(format_hex(i64::from(colour), 2)),
            EditorEvent::Refresh,
        ]
    }

    /// A random colour: rows 5 to 11 of the palette's eight-cell columns,
    /// cells 0 to 4. `random(n)` is the engine's generator (0 to n - 1).
    pub fn random_colour(&mut self, mut random: impl FnMut(i32) -> i32) -> Vec<EditorEvent> {
        let column = random(7);
        let cell = random(5);
        self.set_colour(((column + 5) * 8 + cell) as u8)
    }

    /// The level and balance controls: the item state's setters get the
    /// value less one, and twice the value less a half.
    pub fn state_level(value: f32) -> EditorEvent {
        EditorEvent::Level(value - 1.0)
    }

    pub fn state_balance(value: f32) -> EditorEvent {
        EditorEvent::Balance((value - 0.5) * 2.0)
    }

    /// The seed field.
    pub fn set_seed(&mut self, value: i32) -> Vec<EditorEvent> {
        self.seed = value;
        vec![EditorEvent::Refresh]
    }

    /// A new random seed, shown in control 7.
    pub fn random_seed(&mut self, mut random: impl FnMut(i32) -> i32) -> Vec<EditorEvent> {
        self.seed = random(0x7fff_ffff);
        vec![EditorEvent::Seed(self.seed), EditorEvent::Refresh]
    }

    /// The preset choice (1-based): random, none, then the list's ids.
    pub fn choose_preset(&mut self, choice: i32) -> Vec<EditorEvent> {
        self.preset = match choice {
            1 => 0xff,
            2 => 0,
            c => self
                .presets
                .get((c - 3).max(0) as usize)
                .copied()
                .unwrap_or(0),
        };
        if self.container {
            vec![EditorEvent::Container]
        } else {
            Vec::new()
        }
    }

    /// Fills the preset list: the class's presets, then those of the
    /// first preset layer's variant. Returns the list's entries' ids and
    /// the 1-based entry to select: 1 for random, else the entry *before*
    /// the matching preset (the original's index is one short), 2 when
    /// none matches.
    pub fn fill_presets(&mut self, class: &[u8], layer: &[u8]) -> (Vec<u8>, i32) {
        self.presets = class.iter().chain(layer.iter()).copied().collect();
        if self.preset == 0xff {
            return (self.presets.clone(), 1);
        }
        let select = self
            .presets
            .iter()
            .position(|&p| p == self.preset)
            .map_or(2, |i| i as i32 + 2);
        (self.presets.clone(), select)
    }

    /// The seed pair: both take the value and the look's random-seed flag
    /// clears; an open container refreshes.
    pub fn set_seeds(&mut self, value: i32) -> Vec<EditorEvent> {
        self.seeds = (value, value);
        let mut events = vec![EditorEvent::Refresh];
        if self.container {
            events.push(EditorEvent::Container);
        }
        events
    }

    /// Two new random seeds; control 7 shows the first.
    pub fn random_seeds(&mut self, mut random: impl FnMut(i32) -> i32) -> Vec<EditorEvent> {
        let a = random(0x7fff_ffff);
        let b = random(0x7fff_ffff);
        self.seeds = (a, b);
        vec![EditorEvent::Seed(a)]
    }
}

/// The colour palette popup's position: its columns are eight cells tall,
/// so the current colour sits at column `colour / 8`, row `colour % 8`;
/// the popup is placed so that cell's centre lies under the pointer, or,
/// for a cell anchored to its control, under the control's bottom centre.
pub fn palette_origin(colour: u8, unit: i32, border: i32, anchor: (i32, i32)) -> (i32, i32) {
    let cell = PALETTE_CELL_UNITS * unit as u32;
    let dx = (u32::from(colour) >> 3) * cell + (cell >> 1);
    let dy = u32::from(colour % PALETTE_ROWS) * cell + (cell >> 1);
    (
        anchor.0 - (dx as i32 + border),
        anchor.1 - (dy as i32 + border),
    )
}

/// A control's bottom centre, the anchor of an anchored colour cell.
pub fn control_anchor(x: i16, y: i16, w: i16, h: i16) -> (i32, i32) {
    (i32::from(x) + i32::from(w) / 2, i32::from(h) + i32::from(y))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn level_sliders_write_bytes_and_labels() {
        let mut e = ObjectEditor::default();
        assert_eq!(
            e.first_level(0.5, 0),
            vec![EditorEvent::Label("2.00".into()), EditorEvent::Refresh]
        );
        assert_eq!(e.look.base[0], 128);
        e.filling = 1;
        assert_eq!(
            e.first_level(0.25, 0),
            vec![EditorEvent::Label("1.00".into())]
        );
        assert_eq!(e.look.base[0], 128);
        e.other_level(2, 1.0, 0);
        assert_eq!(e.look.base[2], 255);
        assert_eq!(e.first_level(0.1, SETTING), vec![]);
    }

    #[test]
    fn variation_steps_pack_into_the_fourth_byte() {
        let mut e = ObjectEditor::default();
        assert_eq!(
            e.variation(0, 0.6, 0)[0],
            EditorEvent::Label("Var 2.0".into())
        );
        assert_eq!(e.look.base[3], 3);
        assert_eq!(
            e.variation(1, 1.0, 0)[0],
            EditorEvent::Label(" Var 4.0".into())
        );
        assert_eq!(e.look.base[3], 3 | (5 << 3));
        assert_eq!(
            e.variation(2, 0.5, 0)[0],
            EditorEvent::Label(" Var 1.0".into())
        );
        assert_eq!(e.look.base[3], 3 | (5 << 3) | (2 << 6));
    }

    #[test]
    fn layer_controls_edit_the_selected_layer() {
        let mut e = ObjectEditor::default();
        e.select_layer(3.0);
        assert_eq!(e.layer, 2);
        e.amount(7.4);
        e.first_switch(CHECKED);
        assert_eq!(
            e.material(4),
            vec![
                EditorEvent::Materials(4),
                EditorEvent::Refresh,
                EditorEvent::Presets
            ]
        );
        e.materials = vec![9, 11];
        assert_eq!(e.variant(3), vec![EditorEvent::Refresh]);
        assert_eq!(e.variant(3), vec![]);
        e.second_switch(CHECKED);
        e.first_switch(0);
        assert_eq!(
            e.look.layers[2],
            LookLayer {
                flags: 2,
                amount: 7,
                material: 4,
                variant: 11,
                colour: 0
            }
        );
    }

    #[test]
    fn a_variant_is_stored_while_filling_but_not_refreshed() {
        let mut e = ObjectEditor {
            filling: 1,
            materials: vec![5],
            ..ObjectEditor::default()
        };
        assert_eq!(e.variant(2), vec![]);
        assert_eq!(e.look.layers[0].variant, 5);
        assert_eq!(e.material(3), vec![]);
    }

    #[test]
    fn colours_show_as_two_hex_digits() {
        let mut e = ObjectEditor::default();
        assert_eq!(e.set_colour(0x2a)[0], EditorEvent::Label("2A".into()));
        let mut rolls = [3, 4].into_iter();
        e.random_colour(|_| rolls.next().unwrap());
        assert_eq!(e.look.layers[0].colour, (3 + 5) * 8 + 4);
    }

    #[test]
    fn presets_select_one_entry_short_of_the_match() {
        let mut e = ObjectEditor {
            preset: 0xff,
            ..ObjectEditor::default()
        };
        assert_eq!(e.fill_presets(&[4, 6], &[9]), (vec![4, 6, 9], 1));
        e.preset = 9;
        assert_eq!(e.fill_presets(&[4, 6], &[9]).1, 4);
        e.preset = 1;
        assert_eq!(e.fill_presets(&[4, 6], &[9]).1, 2);
        e.choose_preset(4);
        assert_eq!(e.preset, 6);
        e.choose_preset(1);
        assert_eq!(e.preset, 0xff);
        e.container = true;
        assert_eq!(e.choose_preset(2), vec![EditorEvent::Container]);
        assert_eq!(e.preset, 0);
    }

    #[test]
    fn state_setters_and_seeds() {
        assert_eq!(ObjectEditor::state_level(3.0), EditorEvent::Level(2.0));
        assert_eq!(ObjectEditor::state_balance(0.75), EditorEvent::Balance(0.5));
        let mut e = ObjectEditor::default();
        assert_eq!(e.random_seed(|n| n - 1)[0], EditorEvent::Seed(0x7fff_fffe));
        let mut rolls = [10, 20].into_iter();
        assert_eq!(
            e.random_seeds(|_| rolls.next().unwrap()),
            vec![EditorEvent::Seed(10)]
        );
        assert_eq!(e.seeds, (10, 20));
        e.set_seeds(5);
        assert_eq!(e.seeds, (5, 5));
    }

    #[test]
    fn the_palette_centres_the_current_cell() {
        // Unit 4: cells of 12; colour 0x13 is column 2, row 3.
        assert_eq!(
            palette_origin(0x13, 4, 2, (300, 200)),
            (300 - (24 + 6 + 2), 200 - (36 + 6 + 2))
        );
        assert_eq!(control_anchor(10, 20, 31, 16), (25, 36));
    }
}

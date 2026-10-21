//! A company emblem chosen by name.
//!
//! The name `klean` selects background 52 and design 81, and stores the four
//! layer bytes 7, 0, 0 and 6. A picture name keeps a leading 0 and writes the
//! index's tens and ones after it, so 52 is `emblembkg052` and 81 is
//! `emblemdes081`. Any other name selects nothing. A candidate's listed price
//! was not read.
//!
//! Evidence: the name compare, the two picture loaders, and the four bytes
//! stored with that name. The editor itself is [`exa_ui::EmblemEditor`].

/// Which picture an index names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmblemKind {
    Background,
    Design,
}

/// The pictures and layer bytes a name selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Emblem {
    pub background: u32,
    pub design: u32,
    /// Four bytes stored with the preset. What each byte selects was not read.
    pub layers: [u8; 4],
}

/// The emblem for `name`, if that name is a preset.
pub fn preset_emblem(name: &str) -> Option<Emblem> {
    if name == "klean" {
        Some(Emblem {
            background: 52,
            design: 81,
            layers: [7, 0, 0, 6],
        })
    } else {
        None
    }
}

/// Picture name. The hundreds digit stays 0. The next two are the index
/// divided by 10 and the remainder.
pub fn emblem_texture(kind: EmblemKind, index: u32) -> String {
    let stem = match kind {
        EmblemKind::Background => "emblembkg",
        EmblemKind::Design => "emblemdes",
    };
    let tens = b'0'.wrapping_add((index / 10) as u8);
    let ones = b'0'.wrapping_add((index % 10) as u8);
    format!("{stem}0{}{}", tens as char, ones as char)
}

/// The editor on a company's emblem.
pub fn editor_for(company: &crate::career::Company) -> exa_ui::EmblemEditor {
    let mut editor = exa_ui::EmblemEditor::new();
    editor.set_record(company.emblem);
    editor
}

/// Saves an editor's emblem back to the company.
pub fn save_to(company: &mut crate::career::Company, editor: &exa_ui::EmblemEditor) {
    company.emblem = editor.record();
}

/// The library the emblem editor opens and its pack extension.
pub const EMBLEMS_LIBRARY: &str = "Emblems";
pub const EMBLEMS_PACK: &str = "Emblems.rpk";
/// The new-career dialog's labels.
pub const COMPANY_NAME_TITLE: &str = "Company Name";
pub const COMPANY_NAME_PROMPT: &str = "Please name your company";
/// The message box's icon id the prompt uses.
pub const COMPANY_NAME_ICON: u32 = 0xc1;
/// The company-create form's colour (opaque black).
pub const COMPANY_FORM_COLOUR: [u8; 4] = [0, 0, 0, 255];
/// The action set the company dialog switches to.
pub const COMPANY_ACTION_SET: i32 = 2;

/// What confirming the company name does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanyConfirm {
    /// The name is empty: show the message box and change nothing.
    MissingName,
    /// The name is taken: store it and the emblem, and open the
    /// character screen.
    Named { name: String, layers: [u8; 4] },
}

/// Confirms a company name (the name's bytes are kept whole).
pub fn confirm_company(name: &str, editor: &exa_ui::EmblemEditor) -> CompanyConfirm {
    if name.is_empty() {
        CompanyConfirm::MissingName
    } else {
        CompanyConfirm::Named {
            name: name.to_string(),
            layers: editor.record().layers.to_le_bytes(),
        }
    }
}

/// Where the company dialog is centred: screen width over two, and the
/// 370-scaled line down from the 1024 frame.
pub fn company_place(screen: (i32, i32)) -> [i32; 2] {
    let scale = (f64::from(screen.1) / 135.0).round_ties_even() * 0.125;
    let top = (f64::from(screen.1) - (scale as f32) as f64 * 1024.0).round_ties_even() as i32 / 2;
    [screen.0 / 2, (scale * 370.0).round_ties_even() as i32 + top]
}

/// The emblem form's "remove layer" callback: takes the last layer off
/// and disables the button when none is left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerRemove {
    pub removed: bool,
    pub disabled: bool,
}

pub fn remove_layer(count: &mut i32) -> LayerRemove {
    if *count > 0 {
        *count -= 1;
    }
    LayerRemove {
        removed: *count >= 0 && *count < i32::MAX,
        disabled: *count < 1,
    }
}

/// The emblem form's mask callback: sets or clears a record word's bits.
pub fn toggle_mask(word: &mut u32, value: u32, set: bool) {
    if set {
        *word |= value;
    } else {
        *word &= !value;
    }
}

/// The four layer bytes the "klean" preset stores, and its pictures.
pub const KLEAN_LAYERS: [u8; 4] = [7, 0, 0, 6];
pub const KLEAN_BACKGROUND: u32 = 52;
pub const KLEAN_DESIGN: u32 = 81;

/// Applies the preset to an editor when the typed name is exactly `klean`.
pub fn apply_klean(name: &str, editor: &mut exa_ui::EmblemEditor) -> bool {
    if name != "klean" {
        return false;
    }
    editor.record.background = KLEAN_BACKGROUND;
    editor.record.design = KLEAN_DESIGN;
    editor.record.layers = u32::from_le_bytes(KLEAN_LAYERS);
    editor.set_background(KLEAN_BACKGROUND as i32);
    editor.set_design(KLEAN_DESIGN as i32);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_company_emblem_round_trips_through_the_editor() {
        let mut company = crate::career::Company::new("Ash");
        let mut editor = editor_for(&company);
        assert_eq!(editor.record(), exa_ui::EmblemRecord::default());
        editor.record.background = 49;
        editor.record.design = 79;
        editor.record.layers = exa_ui::emblem_form::MIRROR_X;
        save_to(&mut company, &editor);
        assert_eq!(company.emblem.background, 49);
        assert_eq!(company.emblem.design, 79);
        assert_eq!(company.emblem.layers, 0x10);
        assert_eq!(editor_for(&company).record().design, 79);
    }

    #[test]
    fn klean_selects_background_52_and_design_81() {
        let emblem = preset_emblem("klean").unwrap();
        assert_eq!(emblem.background, 52);
        assert_eq!(emblem.design, 81);
        assert_eq!(emblem.layers, [7, 0, 0, 6]);
        assert_eq!(
            emblem_texture(EmblemKind::Background, emblem.background),
            "emblembkg052"
        );
        assert_eq!(
            emblem_texture(EmblemKind::Design, emblem.design),
            "emblemdes081"
        );
        assert!(preset_emblem("Klean").is_none());
        assert!(preset_emblem("").is_none());
    }
}

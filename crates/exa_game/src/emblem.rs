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

/// The emblem texture loader's flags and its name/flags pair.
pub const EMBLEM_TEXTURE_FLAGS: u32 = 0x70133;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmblemTexture {
    pub name: String,
    pub flags: u32,
}

/// The loader's name and flags: `emblembkg0NN` or `emblemdes0NN`.
pub fn emblem_texture_request(kind: EmblemKind, index: u32) -> EmblemTexture {
    EmblemTexture {
        name: emblem_texture(kind, index),
        flags: EMBLEM_TEXTURE_FLAGS,
    }
}

/// A callback's value write: the form's per-index word and the shared
/// emblem record's word both take `value`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerWrite {
    pub form_word: i32,
    pub record_word: i32,
}

pub fn write_layer_value(index: usize, value: i32) -> LayerWrite {
    let _ = index;
    LayerWrite {
        form_word: value,
        record_word: value,
    }
}

/// The layer list's label: the one-based layer number.
pub fn layer_label(index: i32) -> String {
    format!("{}", index.wrapping_add(1))
}

/// A picture picker's selected index: the label's text parsed, one below
/// the one-based control.
pub fn picture_index(text: i32) -> i32 {
    text.wrapping_sub(1)
}

/// The layer picker's index: the control's number plus the form's own
/// offset.
pub fn layer_index(control: i32, form_offset: i32) -> i32 {
    control.wrapping_add(form_offset)
}

/// The colour control's value: the form's base plus the control's delta.
pub fn colour_value(base: i32, delta: i32) -> i32 {
    base.wrapping_add(delta)
}

/// The new-career dialog's close: hides the dialog and returns to the
/// main menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanyClose {
    pub close_dialog: bool,
    pub main_menu: bool,
}

pub fn close_company_to_menu(has_dialog: bool) -> CompanyClose {
    CompanyClose {
        close_dialog: has_dialog,
        main_menu: true,
    }
}

/// The close after the company is named: closes the dialog, resets the
/// generator, releases the textures and the procedural materials, and
/// returns to mode 2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanyDone {
    pub close_dialog: bool,
    pub reset_generator: bool,
    pub release_textures: bool,
    pub release_materials: bool,
    pub mode: i32,
}

pub fn company_named() -> CompanyDone {
    CompanyDone {
        close_dialog: true,
        reset_generator: true,
        release_textures: true,
        release_materials: true,
        mode: 2,
    }
}

/// The fade wrapper the named company runs behind: the transition calls
/// the named-company step.
pub fn named_company_step() -> CompanyDone {
    company_named()
}

/// The company-create dialog's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanyCreate {
    pub colour: [u8; 4],
    pub layout: usize,
    pub emblem_flags: u32,
}

pub fn company_create() -> CompanyCreate {
    CompanyCreate {
        colour: COMPANY_FORM_COLOUR,
        layout: 1,
        emblem_flags: 0x8800,
    }
}

/// The dialog's paint: a black quad over the whole form before the base
/// paint.
pub fn company_paint(width: i32, height: i32) -> [[f32; 2]; 4] {
    [
        [0.0, 0.0],
        [0.0, height as f32],
        [width as f32, height as f32],
        [width as f32, 0.0],
    ]
}

/// The emblem unit's init is empty and its finaliser releases the stored
/// name and the shader's tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmblemGlobals {
    pub name: String,
    pub shader_record: bool,
    pub layout: bool,
}

pub fn emblem_finalise(mut globals: EmblemGlobals) -> EmblemGlobals {
    globals.name.clear();
    globals.shader_record = false;
    globals.layout = false;
    globals
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
    // The preset stores the two pictures directly; the selector's clamps
    // do not apply to it.
    editor.record.background = KLEAN_BACKGROUND;
    editor.record.design = KLEAN_DESIGN;
    editor.record.layers = u32::from_le_bytes(KLEAN_LAYERS);
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
        assert_eq!(
            emblem_texture_request(EmblemKind::Design, 81),
            EmblemTexture {
                name: "emblemdes081".into(),
                flags: 0x70133
            }
        );
        assert_eq!(write_layer_value(2, 5).record_word, 5);
        assert_eq!(layer_label(0), "1");
        assert_eq!(picture_index(4), 3);
        assert_eq!(layer_index(2, 5), 7);
        assert_eq!(colour_value(10, 2), 12);
        assert!(close_company_to_menu(true).main_menu);
        assert_eq!(company_named().mode, 2);
        assert!(named_company_step().reset_generator);
        assert_eq!(company_create().emblem_flags, 0x8800);
        assert_eq!(company_paint(10, 20)[2], [10.0, 20.0]);
        assert_eq!(
            emblem_finalise(EmblemGlobals {
                name: "x".into(),
                shader_record: true,
                layout: true
            })
            .name,
            ""
        );

        // The company name and the dialog's place.
        let mut editor = exa_ui::EmblemEditor::new();
        assert!(apply_klean("klean", &mut editor));
        assert_eq!(editor.record().background, 52);
        assert_eq!(editor.record().layers, u32::from_le_bytes(KLEAN_LAYERS));
        assert!(!apply_klean("other", &mut editor));
        assert_eq!(confirm_company("", &editor), CompanyConfirm::MissingName);
        assert_eq!(
            confirm_company("Ash", &editor),
            CompanyConfirm::Named {
                name: "Ash".into(),
                layers: KLEAN_LAYERS
            }
        );
        let mut count = 2;
        assert_eq!(
            remove_layer(&mut count),
            LayerRemove {
                removed: true,
                disabled: false
            }
        );
        count = 0;
        assert!(remove_layer(&mut count).disabled);
        let mut word = 0;
        toggle_mask(&mut word, 0x10, true);
        toggle_mask(&mut word, 0x4, true);
        assert_eq!(word, 0x14);
        toggle_mask(&mut word, 0x10, false);
        assert_eq!(word, 0x4);
        assert_eq!(company_place((1920, 1080)), [960, 370 + 28]);
    }
}

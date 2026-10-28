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
    use exa_formats::procedural::{EMBLEM_BACKGROUND_STEM, EMBLEM_DESIGN_STEM, emblem_picture};
    let stem = match kind {
        EmblemKind::Background => EMBLEM_BACKGROUND_STEM,
        EmblemKind::Design => EMBLEM_DESIGN_STEM,
    };
    emblem_picture(stem, index)
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

/// The library the emblem editor opens: this name joined with a data path,
/// then [`EMBLEMS_EXTENSION`].
pub const EMBLEMS_LIBRARY: &str = "Emblems";
pub const EMBLEMS_EXTENSION: &str = ".rpk";
/// The new-career dialog's message-box title and prompt.
pub const COMPANY_NAME_TITLE: &str = "Company Name";
pub const COMPANY_NAME_PROMPT: &str = "Please name your company";
/// The word the prompt passes to the message box after the two texts (its
/// role is not traced).
pub const COMPANY_NAME_BOX_WORD: u32 = 0xc1;
/// The company-create form's colour (opaque black).
pub const COMPANY_FORM_COLOUR: [u8; 4] = [0, 0, 0, 255];

/// What confirming the company name does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompanyConfirm {
    /// The name is empty: show the message box and change nothing.
    MissingName,
    /// The name is kept whole, the emblem record is stored beside it,
    /// character creation starts and the fade runs [`company_named`] as its
    /// step.
    Named {
        name: String,
        emblem: exa_ui::EmblemRecord,
    },
}

/// Confirms a company name.
pub fn confirm_company(name: &str, editor: &exa_ui::EmblemEditor) -> CompanyConfirm {
    if name.is_empty() {
        CompanyConfirm::MissingName
    } else {
        CompanyConfirm::Named {
            name: name.to_string(),
            emblem: editor.record(),
        }
    }
}

/// Where the new-career screen puts the mouse cursor: half the screen width
/// across, and the 370-scaled line down from the 1024 frame (the scale is
/// the height over 135, rounded to even, in eighths). The cursor is moved
/// there through the window's client-to-screen conversion.
pub fn company_cursor(screen: (i32, i32)) -> [i32; 2] {
    let height = screen.1;
    let scale = ((f64::from(height) / 135.0).round_ties_even() * 0.125) as f32;
    let top = (height as f32 - scale * 1024.0).round_ties_even() as i64 / 2;
    let line = (370.0f32 * scale).round_ties_even() as i64 + top;
    [(screen.0 as u32 >> 1) as i32, line as i32]
}

/// The emblem form's "remove layer" button: takes the last layer off while
/// there is one, then disables the button once fewer than one is left (the
/// test runs even when nothing was removed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayerRemove {
    pub removed: bool,
    pub disable_button: bool,
}

pub fn remove_layer(count: &mut i32) -> LayerRemove {
    let removed = *count > 0;
    if removed {
        *count -= 1;
    }
    LayerRemove {
        removed,
        disable_button: *count < 1,
    }
}

/// The picture buttons: a button's tag added to the form's current
/// background or design index gives the index the form is asked to take.
pub fn step_picture(current: i32, tag: i32) -> i32 {
    current.wrapping_add(tag)
}

/// The emblem texture loader's flags.
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

/// Backing out of the new-career dialog: closes the dialog when there is one
/// and runs the front-end step (traced only as far as setting a flag and
/// bringing the world GUI back).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanyClose {
    pub close_dialog: bool,
    pub front_end_step: bool,
}

pub fn close_company_back(has_dialog: bool) -> CompanyClose {
    CompanyClose {
        close_dialog: has_dialog,
        front_end_step: true,
    }
}

/// The step the fade runs once the company is named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanyDone {
    pub close_dialog: bool,
    /// The character-creation reset called with 0.
    pub reset_character_create: bool,
    pub release_textures: bool,
    pub release_procedural_materials: bool,
    /// The value the game-mode global takes.
    pub mode: i32,
}

pub fn company_named() -> CompanyDone {
    CompanyDone {
        close_dialog: true,
        reset_character_create: true,
        release_textures: true,
        release_procedural_materials: true,
        mode: 2,
    }
}

/// The company-create dialog's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompanyCreate {
    pub colour: [u8; 4],
    /// The control table's high index (so two controls).
    pub controls_high: usize,
    /// Bits OR-ed into the embedded emblem form's flag word.
    pub emblem_flags_or: u32,
}

pub fn company_create() -> CompanyCreate {
    CompanyCreate {
        colour: COMPANY_FORM_COLOUR,
        controls_high: 1,
        emblem_flags_or: 0x8800,
    }
}

/// The dialog's paint: a quad over the whole form in the form's colour
/// (texture corners 0,0, 0,1, 1,1, 1,0) before the base paint.
pub fn company_paint(width: i32, height: i32) -> [[f32; 2]; 4] {
    [
        [0.0, 0.0],
        [0.0, height as f32],
        [width as f32, height as f32],
        [width as f32, 0.0],
    ]
}

/// The four layer bytes the "klean" preset stores, and its pictures.
pub const KLEAN_LAYERS: [u8; 4] = [7, 0, 0, 6];

/// Applies the preset to an editor when the typed name is exactly `klean`.
/// The original also needs the entry's changed flag to be set before it
/// compares.
pub fn apply_klean(name: &str, editor: &mut exa_ui::EmblemEditor) -> bool {
    let Some(preset) = preset_emblem(name) else {
        return false;
    };
    // The preset stores the two pictures directly; the selector's clamps
    // do not apply to it.
    editor.record.background = preset.background;
    editor.record.design = preset.design;
    editor.record.layers = u32::from_le_bytes(preset.layers);
    true
}

/// The editor's preview as a draw-path layer: a quad over
/// [`exa_ui::preview_quad`], filled by the emblem shader's own body
/// ([`exa_ui::emblem_sample`]) at the quad's size. The four pictures are
/// looked up by the record's names and the two shared layers. `None` when
/// any layer is not loaded.
pub fn preview_layer(
    editor: &exa_ui::EmblemEditor,
    textures: &exa_ui::gui_draw::GuiTextures,
    unit: i32,
) -> Option<exa_ui::gui_draw::GuiLayer> {
    let preview = editor.preview(|index| {
        // The palette table is outside the pack; the index is used as a
        // grey ramp until the palette file is read [guess].
        [index, index, index]
    });
    let names = &preview.textures;
    let layers = [
        textures.get(&names[0])?,
        textures.get(&names[1])?,
        textures.get(&names[2])?,
        textures.get(&names[3])?,
    ];
    let rect = exa_ui::preview_quad(exa_ui::emblem_form::FORM_WIDTH, unit);
    let size = rect[2] - rect[0];
    let image = exa_ui::preview_image(&preview, layers, size)?;
    let mut batch = exa_ui::skin::SkinBatch::new();
    batch.select(2, 0, true);
    batch.blend = exa_ui::gui_draw::BLEND_ALPHA;
    batch.tex_col = Some(1.0);
    batch.set_ink([255; 4]);
    batch.begin();
    batch.set_uv(0.0, 1.0);
    batch.vertex(rect[0] as f32, rect[1] as f32);
    batch.set_uv(0.0, 0.0);
    batch.vertex(rect[0] as f32, rect[3] as f32);
    batch.set_uv(1.0, 0.0);
    batch.vertex(rect[2] as f32, rect[3] as f32);
    batch.set_uv(1.0, 1.0);
    batch.vertex(rect[2] as f32, rect[1] as f32);
    batch.end_quads();
    Some(exa_ui::gui_draw::GuiLayer::new(
        batch,
        exa_ui::gui_draw::LayerTexture::Owned(image),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_draws_the_emblem_shader_into_a_layer() {
        let editor = exa_ui::EmblemEditor::new();
        let mut textures = exa_ui::gui_draw::GuiTextures::default();
        let solid = exa_ui::gui_draw::GuiTexture::solid([255, 255, 255, 255]);
        for name in exa_ui::preview_textures(&editor.record()) {
            textures.textures.insert(name, solid.clone());
        }
        let layer = preview_layer(&editor, &textures, 4).unwrap();
        assert_eq!(layer.batch.primitives.len(), 1);
        let exa_ui::gui_draw::LayerTexture::Owned(image) = &layer.texture else {
            panic!("the preview carries its rendered image");
        };
        assert_eq!(image.width, 256);
        // Missing pictures draw nothing.
        let empty = exa_ui::gui_draw::GuiTextures::default();
        assert!(preview_layer(&editor, &empty, 4).is_none());
    }

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
        assert_eq!(step_picture(52, -1), 51);
        assert_eq!(step_picture(i32::MAX, 1), i32::MIN);
        assert!(close_company_back(true).close_dialog);
        assert!(!close_company_back(false).close_dialog);
        assert_eq!(company_named().mode, 2);
        assert_eq!(company_create().emblem_flags_or, 0x8800);
        assert_eq!(company_paint(10, 20)[2], [10.0, 20.0]);

        let mut editor = exa_ui::EmblemEditor::new();
        assert!(apply_klean("klean", &mut editor));
        assert_eq!(editor.record().background, 52);
        assert_eq!(editor.record().design, 81);
        assert_eq!(editor.record().layers, u32::from_le_bytes(KLEAN_LAYERS));
        assert!(!apply_klean("other", &mut editor));
        assert_eq!(confirm_company("", &editor), CompanyConfirm::MissingName);
        assert_eq!(
            confirm_company("Ash", &editor),
            CompanyConfirm::Named {
                name: "Ash".into(),
                emblem: editor.record()
            }
        );
        // The last layer comes off while there is one; the button turns
        // off once fewer than one is left, even when nothing came off.
        let mut count = 2;
        assert_eq!(
            remove_layer(&mut count),
            LayerRemove {
                removed: true,
                disable_button: false
            }
        );
        assert_eq!(count, 1);
        assert!(remove_layer(&mut count).disable_button);
        assert_eq!(count, 0);
        count = 0;
        assert_eq!(
            remove_layer(&mut count),
            LayerRemove {
                removed: false,
                disable_button: true
            }
        );
        count = -3;
        assert!(!remove_layer(&mut count).removed);
        assert_eq!(count, -3);
        // 1080 high: scale 1, the frame leaves 56 over, so the line is
        // 370 + 28; 720 high: 720/135 rounds to 5, scale 0.625, top 40,
        // line 231 + 40.
        assert_eq!(company_cursor((1920, 1080)), [960, 398]);
        assert_eq!(company_cursor((1280, 720)), [640, 271]);
    }
}

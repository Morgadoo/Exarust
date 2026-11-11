//! Painting a form that keeps the draw skin's own default instance.
//!
//! A form made without an owner takes the draw skin's default instance and
//! keeps it unless its maker sets another (the game's screens set the line
//! skin or the textured skin). The colour picker's constructor sets none,
//! so the picker draws with the draw skin's own methods and palette
//! ([`exa_ui::skin::shown_base_palette`]):
//!
//! * the form draw: the default background
//!   ([`exa_ui::skin::draw_form_background`]) with the words the show
//!   commits (the title bar, the rounded shell and the close mark), its
//!   two title blocks, then the records in the client, which starts under
//!   the title bar ([`exa_ui::skin::BASE_TITLED_CLIENT_TOP`]);
//! * a button: the skin's shell ([`exa_ui::skin::draw_control_shell`]),
//!   then its caption through the skin's label slot
//!   ([`exa_ui::skin::LabelVariant::Slot`]);
//! * a trackbar: the style frames, the skin's slider
//!   ([`exa_ui::skin::draw_control_slider`]: a marker trackbar's striped
//!   fill from the record's colour list and the unit-wide bar at its
//!   value), then its caption through the plain label draw.
//!
//! The other records (the spinners, the swatches and the colour field)
//! draw through their classes' own draws in the same palette.
//!
//! Evidence: the picker's constructor (no skin set, owner none), the base
//! form constructor (the default instance first, an owner's skin after),
//! the default instance's maker and scheme record, the button, check box
//! and trackbar classes' draw messages (the skin slots they call), the
//! skin's slider, shell and default background, read from their assembly.

use exa_ui::form::FormMetrics;
use exa_ui::gui_controls::{Control, paint_back};
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiLayer, LayerTexture, SKIN_TEXTURE};
use exa_ui::gui_font::GuiFont;
use exa_ui::record_form::RecordForm;
use exa_ui::skin::{
    ControlLabelPlace, FormBackgroundPlace, GuiRect as SkinRect, LabelVariant, SkinBatch, SkinInk,
    SkinPalette, draw_control_label, draw_form_background, glow_text,
};

use crate::gui_paint::{self, Painted, caption_layer};
use crate::hub_windows::Look;

/// The button record class.
const BUTTON: u16 = 1;
/// The check box record class.
const CHECK_BOX: u16 = 2;
/// The trackbar record class.
const TRACKBAR: u16 = 4;
/// A record's hidden state bits.
const HIDDEN: u32 = 0x110;

/// The look a base-skinned form draws with: the hub's, its palette the
/// default instance's at the same fade.
pub(crate) fn base_look<'a>(look: &Look<'a>) -> Look<'a> {
    Look {
        palette: exa_ui::skin::shown_base_palette(look.fade),
        fade: look.fade,
        unit: look.unit,
        textures: look.textures,
        glyphs: look.glyphs,
    }
}

/// The default background's place for a form at its committed words.
fn background_place(
    form: &RecordForm,
    font: &GuiFont,
    words: gui_paint::LineFormWords,
) -> FormBackgroundPlace {
    let r = form.rect();
    let border = form.geometry.border;
    let unit = form.unit();
    let (_, oy) = form.origin();
    FormBackgroundPlace {
        rect: SkinRect::sized(0.0, 0.0, r.w as f32, r.h as f32),
        inset: border,
        client_top: oy - r.y,
        client_right: r.w - border,
        client_bottom: r.h - border,
        state: words.state,
        flags: words.attributes,
        frame_style: words.style,
        margin: border,
        bar: FormMetrics { unit, border }.base(),
        has_lines: false,
        fill: None,
        vertical: None,
        horizontal: None,
        title_width: font.text_width(form.title.as_bytes()),
        title_icon: false,
        line_height: font.line_height,
        hover_code: 0,
        pressed_code: 0,
        button_down: false,
    }
}

/// Paints the form's default background and its title blocks, with the
/// words the show commits for a form whose owner is the task.
pub(crate) fn paint_base_frame(
    painted: &mut Painted,
    form: &RecordForm,
    palette: &SkinPalette,
    font: &GuiFont,
    font_texture: Option<(u16, u16)>,
) {
    let words = gui_paint::commit_words(form.spec.placement, form.spec.attributes, true);
    let r = form.rect();
    let mut batch = SkinBatch::new();
    let drawn = draw_form_background(
        &mut batch,
        background_place(form, font, words),
        form.unit(),
        palette,
    );
    gui_paint::offset_batch(&mut batch, r.x, r.y);
    painted.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
    if let (Some(title), Some(texture)) = (drawn.title, font_texture) {
        for block in title {
            painted.push(caption_layer(
                font,
                texture,
                form.title.as_bytes(),
                (r.x + block.x, r.y + block.y),
                block.ink,
            ));
        }
    }
}

/// A record's caption place for the skin's label draws.
fn label_place(c: &Control, rect: GuiRect, font: &GuiFont) -> ControlLabelPlace {
    ControlLabelPlace {
        rect: SkinRect::sized(
            rect.left as f32,
            rect.top as f32,
            rect.width() as f32,
            rect.height() as f32,
        ),
        inset: c.margin,
        style: c.style,
        state: c.state,
        offset: 0,
        text_width: font.text_width(&c.text),
        line_height: font.line_height,
        base: font.ascent,
        text_len: c.text.len(),
        step: 1,
    }
}

/// Pushes a label draw's text calls: one call in its ink, the shadow
/// pair, or the glowing text and its four copies under the additive blend.
fn push_label(
    painted: &mut Painted,
    draw: &exa_ui::skin::ControlLabelDraw,
    text: &[u8],
    palette: &SkinPalette,
    font: &GuiFont,
    texture: (u16, u16),
) {
    let text = &text[..draw.keep.min(text.len())];
    if let Some(shadow) = draw.shadow {
        for (x, y, ink) in shadow {
            painted.push(caption_layer(font, texture, text, (x, y), ink));
        }
        return;
    }
    if draw.coloured {
        let glow = glow_text(draw.x, draw.y, palette);
        painted.push(caption_layer(
            font,
            texture,
            text,
            (draw.x, draw.y),
            glow.text_ink,
        ));
        for at in glow.glow_places {
            let mut layer = caption_layer(font, texture, text, at, glow.glow_ink);
            layer.batch.blend = glow.blend;
            for primitive in &mut layer.batch.primitives {
                primitive.blend = glow.blend;
            }
            painted.push(layer);
        }
        return;
    }
    if let Some(ink) = draw.ink {
        painted.push(caption_layer(font, texture, text, (draw.x, draw.y), ink));
    }
}

/// Paints one record as the draw skin's own methods draw it, at `origin`
/// (the client's top left on the screen): its back through the skin's
/// slots in its class's order, then its caption. `stripes` is a marker
/// trackbar's colour list. Returns false for a class this draw leaves to
/// the class's own front draw.
pub(crate) fn paint_base_control(
    painted: &mut Painted,
    c: &Control,
    origin: (i32, i32),
    look: &Look,
    stripes: &[SkinInk],
) -> bool {
    if !matches!(c.class, BUTTON | CHECK_BOX | TRACKBAR) {
        return false;
    }
    if c.state & HIDDEN == HIDDEN {
        return true;
    }
    let mut at = c.clone();
    at.x += origin.0;
    at.y += origin.1;
    let rect = GuiRect::new(at.x, at.y, at.w, at.h);
    let mut batch = SkinBatch::new();
    paint_back(&mut batch, &at, look.unit, &look.palette, stripes);
    painted.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
    if c.text.is_empty() {
        return true;
    }
    let (font, size) = crate::hub_windows::screen_font(look.textures, look.unit);
    let Some(texture) = size else {
        return true;
    };
    let variant = match c.class {
        // The button's front calls the skin's label slot.
        BUTTON => LabelVariant::Slot,
        // The check box and the trackbar call the plain label draw.
        _ => LabelVariant::Plain,
    };
    let draw = draw_control_label(
        label_place(&at, rect, &font),
        variant,
        look.palette.text[6],
        &look.palette,
        look.unit,
    );
    push_label(painted, &draw, &c.text, &look.palette, &font, texture);
    true
}

/// Paints a record form that keeps the draw skin's default instance (the
/// colour picker): the default background and title, then every record in
/// the client in its class's draw, a marker trackbar's stripes from the
/// picker's colour lists. Returns the layers its records add.
pub(crate) fn paint_base_form(
    painted: &mut Painted,
    form: &RecordForm,
    look: &Look,
    extras: &crate::hub_forms::FormExtras,
) {
    let base = base_look(look);
    let (font, size) = crate::hub_windows::screen_font(base.textures, base.unit);
    paint_base_frame(painted, form, &base.palette, &font, size);
    let origin = form.origin();
    let text = crate::settings_form::Text { font: &font, size };
    for (i, c) in form.controls().iter().enumerate() {
        let stripes = extras
            .picker
            .and_then(|picker| crate::option_forms::picker_gradient(picker, i))
            .unwrap_or_default();
        if paint_base_control(painted, c, origin, &base, &stripes) {
            continue;
        }
        let rect = GuiRect::new(origin.0 + c.x, origin.1 + c.y, c.w, c.h);
        let drawn = crate::option_forms::paint_extended(
            painted,
            c,
            rect,
            &base,
            crate::option_forms::FieldDraw {
                focused: extras.focus == Some(i) || extras.key_focus == Some(i),
                view: extras.edits.iter().find(|(r, _)| *r == i).map(|(_, v)| *v),
                caret_lit: extras.caret_lit,
            },
        );
        if !drawn {
            crate::settings_form::paint_control(
                c,
                origin,
                base.unit,
                &base.palette,
                &text,
                None,
                painted,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ui::gui_draw::GuiTextures;

    fn look(textures: &GuiTextures) -> Look<'_> {
        Look {
            palette: exa_ui::line_skin::shown_palette(1.0),
            fade: 1.0,
            unit: 4,
            textures,
            glyphs: None,
        }
    }

    #[test]
    fn the_base_look_takes_the_default_instance_palette() {
        let textures = GuiTextures::default();
        let base = base_look(&look(&textures));
        assert_eq!(base.palette, exa_ui::skin::shown_base_palette(1.0));
        assert_eq!(base.unit, 4);
    }

    #[test]
    fn a_marker_trackbar_draws_its_stripes_and_value_bar() {
        let textures = GuiTextures::default();
        let base = base_look(&look(&textures));
        let mut c = Control::new(TRACKBAR, 0, 0, 136, 16);
        c.style = 0x1002_1320;
        c.value = 0.5;
        c.text = b"R:".to_vec();
        let stripes = [[0, 0, 0, 255], [255, 0, 0, 255]];
        let mut painted = Painted::default();
        assert!(paint_base_control(
            &mut painted,
            &c,
            (10, 20),
            &base,
            &stripes
        ));
        // The back is one layer (frames, the striped fill and the bar).
        let back = &painted.layers[0].batch;
        assert!(!back.vertices.is_empty());
        // The stripe runs from the first colour at the trough's left.
        assert!(
            back.vertices
                .iter()
                .any(|v| v.x == 11.0 && v.ink == [0, 0, 0, 255])
        );
    }

    #[test]
    fn a_button_draws_the_shell_and_other_classes_are_left() {
        let textures = GuiTextures::default();
        let base = base_look(&look(&textures));
        let mut button = Control::new(BUTTON, 0, 0, 100, 24);
        button.margin = 2;
        let mut painted = Painted::default();
        assert!(paint_base_control(
            &mut painted,
            &button,
            (0, 0),
            &base,
            &[]
        ));
        assert_eq!(painted.layers.len(), 1);
        let spinner = Control::new(0x10, 0, 0, 40, 16);
        assert!(!paint_base_control(
            &mut painted,
            &spinner,
            (0, 0),
            &base,
            &[]
        ));
    }
}

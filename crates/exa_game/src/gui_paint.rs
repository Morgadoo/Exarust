//! Painting the in-game screens' frames, titles and controls through the
//! draw skin's own emitters.
//!
//! The forms' words and geometry are the original's: the line skin's
//! palette ([`exa_ui::line_skin::line_palette`]), the default background
//! ([`exa_ui::skin::draw_form_background`]) with its title placement from
//! [`exa_ui::skin::dispatch_form`], the button face
//! ([`exa_ui::skin::draw_button_face`]) and the text quads of
//! [`exa_ui::gui_text`] and [`exa_ui::text_box::TextBox::draw`]. The text
//! and icon quads are separate layers sampling the font's atlas through
//! [`exa_ui::gui_draw::text_batch`].
//!
//! The inventory and journal forms draw with the constructors' own words:
//! the ordinary form attributes, the centred-title frame style and, for the
//! item pack, its container's attribute word. Neither screen's constructor
//! sets the title bit, so the frames draw no title bar (the title string the
//! forms carry is unused until a caller sets the bit). The item icons wait
//! for the item renderer; the character view and its preview bars are not
//! drawn.

use exa_ui::form::FormMetrics;
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTextures, LayerTexture, SKIN_TEXTURE, text_batch};
use exa_ui::gui_font::GuiFont;
use exa_ui::gui_text::draw_line;
use exa_ui::line_skin::{LINE_SCHEME, line_palette};
use exa_ui::skin::{
    ControlLabelPlace, DispatchPlace, FormBackgroundPlace, FormChromePlace, GuiRect as SkinRect,
    LabelVariant, SkinBatch, SkinPalette, dispatch_form, draw_button_face, draw_control_label,
    draw_form_background, draw_form_chrome, form_rect,
};

/// The base form constructor's attribute word: a close mark, a client fill
/// and the two layer bits.
pub const FORM_ATTRIBUTES: u32 = 0x30022;
/// The journal constructor sets one more bit on top of the base word.
pub const JOURNAL_ATTRIBUTES: u32 = FORM_ATTRIBUTES | 0x8000;
/// The item pack's attribute word: the base word with the two bits the
/// inventory constructor sets on the pack (0x10 and 0x100000).
pub const PACK_ATTRIBUTES: u32 = FORM_ATTRIBUTES | 0x10 | 0x10_0000;
/// The pack's frame style: the bit the inventory constructor sets on it.
pub const PACK_STYLE: u32 = 0x80;
/// The inventory and journal frame style: both constructors set the
/// centred-title bits (0x3000).
pub const SCREEN_STYLE: u32 = 0x3000;
/// The form's border (the skin's inset).
pub const BORDER: i32 = 2;
/// The inventory form's spacing, which its constructor sets (the base
/// constructor's is 2, which the journal keeps).
pub const INVENTORY_SPACING: i32 = 8;
/// The base constructor's spacing.
pub const FORM_SPACING: i32 = 2;
/// The item pack's size (from the inventory constructor).
pub const PACK: (i32, i32) = (228, 180);

/// A form frame to paint: its place, words and title.
#[derive(Debug, Clone, PartialEq)]
pub struct Frame {
    pub rect: GuiRect,
    pub unit: i32,
    pub title: String,
    pub attributes: u32,
    pub style: u32,
    pub state: u32,
    pub spacing: i32,
}

impl Frame {
    /// A frame from its place and title, with the base constructor's words
    /// and spacing.
    pub fn new(rect: GuiRect, unit: i32, title: &str) -> Self {
        Self {
            rect,
            unit,
            title: title.to_string(),
            attributes: FORM_ATTRIBUTES,
            style: SCREEN_STYLE,
            state: 0,
            spacing: FORM_SPACING,
        }
    }

    /// The client rectangle inside the frame's spacing.
    pub fn client(&self) -> GuiRect {
        GuiRect::new(
            self.rect.left + self.spacing,
            self.rect.top + self.spacing,
            self.rect.width() - self.spacing * 2,
            self.rect.height() - self.spacing * 2,
        )
    }

    /// The title font line's baseline, as the skin places it.
    pub fn baseline(&self, font: &GuiFont) -> i32 {
        ((i64::from(BORDER) + i64::from(font.line_height) + 25) >> 1) as i32
    }
}

/// A screen's painted layers and the geometry its content needs.
#[derive(Debug, Default)]
pub struct Painted {
    pub layers: Vec<GuiLayer>,
    /// The frame's client rectangle.
    pub client: GuiRect,
}

impl Painted {
    pub fn push(&mut self, layer: GuiLayer) {
        self.layers.push(layer);
    }

    /// Fills the frame's draw queue.
    pub fn submit(self, frames: &mut GuiFrames) {
        let mut frame = exa_ui::gui_draw::GuiFrame::new();
        for layer in self.layers {
            frame.push(layer);
        }
        frames.push(frame);
    }
}

pub(crate) fn offset_batch(batch: &mut SkinBatch, x: i32, y: i32) {
    for vertex in &mut batch.vertices {
        vertex.x += x as f32;
        vertex.y += y as f32;
    }
}

/// The title's two text blocks, as two font layers.
fn title_layers(
    frame: &Frame,
    palette: &SkinPalette,
    font: &GuiFont,
    font_texture: Option<(u16, u16)>,
    dispatch: &exa_ui::skin::FormDispatch,
) -> Vec<GuiLayer> {
    let Some(title) = dispatch.title else {
        return Vec::new();
    };
    let Some(texture) = font_texture else {
        return Vec::new();
    };
    let bytes = frame.title.as_bytes();
    let blocks = [
        (title.x - 1, title.baseline, palette.form[1]),
        (title.x, title.baseline + 1, palette.text[6]),
    ];
    let mut out = Vec::new();
    for (x, y, ink) in blocks {
        let quads = draw_line(font, bytes, frame.rect.left + x, frame.rect.top + y);
        let batch = text_batch(font, &quads, ink, texture);
        out.push(GuiLayer::new(batch, LayerTexture::Named(font.texture)));
    }
    out
}

/// The form's `DispatchPlace`: the place the skin's opening reads.
fn dispatch_place(frame: &Frame, font: &GuiFont) -> DispatchPlace {
    DispatchPlace {
        flags_a: frame.state,
        flags_b: frame.attributes,
        flags_c: 0,
        flags_d: frame.style,
        title_width: font.text_width(frame.title.as_bytes()),
        title_byte: 0,
        baseline: frame.baseline(font),
        titled: true,
        close_mark: -4,
        width: frame.rect.width(),
        height: frame.rect.height(),
        at_x: frame.rect.left,
        at_y: frame.rect.top,
        view_x: 0,
        view_y: 0,
        span_x: 0,
        span_y: 0,
        limit_x: 0,
        limit_y: 0,
        along_x: 0,
        along_y: 0,
        count: 0,
        edge_x: 0,
        edge_y: 0,
        room_far_x: 0,
        room_near_x: 0,
        room_far_y: 0,
        room_near_y: 0,
        owner_flags: 0,
        grip_x: 0,
        grip_y: 0,
        seat_y: 0,
        post_y: 0,
    }
}

/// Paints a form's frame, chrome and title. `unit` sets the skin's measures.
pub fn paint_frame(
    frame: &Frame,
    palette: &SkinPalette,
    font: &GuiFont,
    font_texture: Option<(u16, u16)>,
) -> Painted {
    let unit = frame.unit;
    let metrics = FormMetrics {
        unit,
        border: BORDER,
    };
    let rect = SkinRect::sized(
        0.0,
        0.0,
        frame.rect.width() as f32,
        frame.rect.height() as f32,
    );
    let width = frame.rect.width();
    let height = frame.rect.height();
    let place = FormBackgroundPlace {
        rect,
        inset: BORDER,
        client_top: frame.spacing,
        client_right: width - frame.spacing,
        client_bottom: height - frame.spacing,
        state: frame.state,
        flags: frame.attributes,
        frame_style: frame.style,
        margin: BORDER,
        bar: metrics.base(),
        has_lines: false,
        fill: None,
        vertical: None,
        horizontal: None,
        title_width: font.text_width(frame.title.as_bytes()),
        title_icon: false,
        line_height: font.line_height,
        hover_code: 0,
        pressed_code: 0,
        button_down: false,
    };
    let mut batch = SkinBatch::new();
    draw_form_background(&mut batch, place, unit, palette);
    offset_batch(&mut batch, frame.rect.left, frame.rect.top);
    let dispatch = dispatch_form(unit, 0, dispatch_place(frame, font), &[]);
    let mut painted = Painted {
        client: frame.client(),
        ..Painted::default()
    };
    painted.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
    for layer in title_layers(frame, palette, font, font_texture, &dispatch) {
        painted.push(layer);
    }
    painted
}

/// Paints a child panel's frame (the item pack's form).
pub fn paint_panel(
    rect: GuiRect,
    unit: i32,
    palette: &SkinPalette,
    attributes: u32,
    style: u32,
) -> Painted {
    paint_child(rect, unit, palette, attributes, style, 4, None)
}

/// Paints a child form's frame with its client's top inset and, for a form
/// with a vertical scrollbar (attribute 0x200), the thumb's place and length
/// in the form's own coordinates.
pub fn paint_child(
    rect: GuiRect,
    unit: i32,
    palette: &SkinPalette,
    attributes: u32,
    style: u32,
    client_top: i32,
    vertical: Option<exa_ui::skin::FormScroll>,
) -> Painted {
    let metrics = FormMetrics {
        unit,
        border: BORDER,
    };
    let skin_rect = SkinRect::sized(0.0, 0.0, rect.width() as f32, rect.height() as f32);
    let place = FormBackgroundPlace {
        rect: skin_rect,
        inset: BORDER,
        client_top,
        client_right: rect.width() - BORDER,
        client_bottom: rect.height() - BORDER,
        state: 0,
        flags: attributes,
        frame_style: style,
        margin: BORDER,
        bar: metrics.base(),
        has_lines: false,
        fill: None,
        vertical,
        horizontal: None,
        title_width: 0,
        title_icon: false,
        line_height: 20,
        hover_code: 0,
        pressed_code: 0,
        button_down: false,
    };
    let mut batch = SkinBatch::new();
    draw_form_background(&mut batch, place, unit, palette);
    offset_batch(&mut batch, rect.left, rect.top);
    let mut painted = Painted {
        client: GuiRect::new(
            rect.left + 4,
            rect.top + 4,
            rect.width() - 8,
            rect.height() - 8,
        ),
        ..Painted::default()
    };
    painted.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
    painted
}

/// Where a control's caption starts: the skin's own placement from the
/// control's style (its x from bits 0x7000, its baseline from bits 0x700).
pub(crate) fn caption_origin(
    rect: GuiRect,
    style: u32,
    font: &GuiFont,
    text: &[u8],
    palette: &SkinPalette,
    unit: i32,
) -> (i32, i32) {
    let draw = draw_control_label(
        ControlLabelPlace {
            rect: SkinRect::sized(
                rect.left as f32,
                rect.top as f32,
                rect.width() as f32,
                rect.height() as f32,
            ),
            inset: BORDER,
            style,
            state: 0,
            offset: 0,
            text_width: font.text_width(text),
            line_height: font.line_height,
            base: font.ascent,
            text_len: text.len(),
            step: 1,
        },
        LabelVariant::Plain,
        palette.text[6],
        palette,
        unit,
    );
    (draw.x, draw.y)
}

pub(crate) fn caption_layer(
    font: &GuiFont,
    font_texture: (u16, u16),
    text: &[u8],
    origin: (i32, i32),
    ink: [u8; 4],
) -> GuiLayer {
    let quads = draw_line(font, text, origin.0, origin.1);
    GuiLayer::new(
        text_batch(font, &quads, ink, font_texture),
        LayerTexture::Named(font.texture),
    )
}

/// Paints a button record: the skin's face and its caption, placed from
/// the record's style word.
#[allow(clippy::too_many_arguments)]
pub fn paint_button(
    rect: GuiRect,
    style: u32,
    unit: i32,
    palette: &SkinPalette,
    font: &GuiFont,
    font_texture: Option<(u16, u16)>,
    label: &str,
    pressed: bool,
    hot: bool,
) -> Painted {
    let mut batch = SkinBatch::new();
    draw_button_face(
        &mut batch,
        SkinRect::sized(
            rect.left as f32,
            rect.top as f32,
            rect.width() as f32,
            rect.height() as f32,
        ),
        pressed,
        hot,
        palette,
    );
    let mut painted = Painted::default();
    painted.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
    if let Some(texture) = font_texture {
        let text = label.as_bytes();
        let origin = caption_origin(rect, style, font, text, palette, unit);
        painted.push(caption_layer(font, texture, text, origin, palette.text[6]));
    }
    painted
}

/// Paints a label record: its text at the place its style word gives.
pub fn paint_label(
    rect: GuiRect,
    style: u32,
    unit: i32,
    palette: &SkinPalette,
    font: &GuiFont,
    font_texture: Option<(u16, u16)>,
    label: &str,
) -> Painted {
    paint_label_ink(rect, style, unit, palette, font, font_texture, label, None)
}

/// Paints a label record with a caller's ink; `None` is the skin's own text
/// colour. The roster's name cells use their role's ink this way.
#[allow(clippy::too_many_arguments)]
pub fn paint_label_ink(
    rect: GuiRect,
    style: u32,
    unit: i32,
    palette: &SkinPalette,
    font: &GuiFont,
    font_texture: Option<(u16, u16)>,
    label: &str,
    ink: Option<[u8; 4]>,
) -> Painted {
    let mut painted = Painted::default();
    if let Some(texture) = font_texture {
        let text = label.as_bytes();
        let origin = caption_origin(rect, style, font, text, palette, unit);
        painted.push(caption_layer(
            font,
            texture,
            text,
            origin,
            ink.unwrap_or(palette.text[6]),
        ));
    }
    painted
}

/// The frame and title of a form, as the debug flag opens it.
pub fn paint_debug_form(rect: GuiRect, unit: i32, textures: &GuiTextures) -> Painted {
    let palette = line_palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, font_texture) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    // The debug form sets the title bit itself so that the title path shows.
    let mut frame = Frame::new(rect, unit, "GUI Form");
    frame.attributes |= 1;
    paint_frame(&frame, &palette, &font, font_texture)
}

/// The line skin's palette (a convenience for the screen painters).
pub fn palette() -> SkinPalette {
    line_palette()
}

/// The line skin's scheme, for the callers that need its colours.
pub const SCHEME: exa_ui::skin::SkinScheme = LINE_SCHEME;

/// Draws the client's fill (a flat translucent black) alone, for a screen
/// that has no pack panel.
pub fn paint_client_fill(rect: GuiRect, palette: &SkinPalette) -> Painted {
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    batch.set_ink(palette.form[3]);
    batch.begin();
    batch.vertex(rect.left as f32, rect.top as f32);
    batch.vertex(rect.left as f32, rect.bottom as f32);
    batch.vertex(rect.right as f32, rect.bottom as f32);
    batch.vertex(rect.right as f32, rect.top as f32);
    batch.end_quads();
    let mut painted = Painted::default();
    painted.push(GuiLayer::new(batch, LayerTexture::None));
    painted
}

/// Re-export the chrome painter so a screen with its own frame still uses
/// the skin's rule.
pub fn paint_chrome(
    batch: &mut SkinBatch,
    rect: GuiRect,
    attributes: u32,
    style: u32,
    unit: i32,
    palette: &SkinPalette,
) {
    draw_form_chrome(
        batch,
        FormChromePlace {
            rect: SkinRect::sized(
                rect.left as f32,
                rect.top as f32,
                rect.width() as f32,
                rect.height() as f32,
            ),
            frame_style: style,
            flags: attributes,
            state: 0,
            margin: BORDER as f32,
            bar: FormMetrics {
                unit,
                border: BORDER,
            }
            .base() as f32,
            client_left: (rect.left + BORDER) as f32,
            client_top: (rect.top + BORDER) as f32,
            client_right: (rect.right - BORDER) as f32,
            client_bottom: (rect.bottom - BORDER) as f32,
            fill: None,
            has_lines: false,
        },
        palette,
    );
}

/// A form's control-code rectangle, through the skin's own switch.
pub fn control_rect(rect: GuiRect, code: i32, unit: i32, attributes: u32) -> Option<[i32; 4]> {
    let mut out = [0; 4];
    form_rect(
        exa_ui::skin::FormRectPlace {
            width: rect.width(),
            height: rect.height(),
            unit,
            inset: BORDER,
            client_top: rect.top + BORDER,
            right: rect.right - BORDER,
            bottom: rect.bottom - BORDER,
            flags: attributes,
        },
        code,
        &mut out,
    )
    .then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ui::game_gui::inventory_height;
    use exa_ui::gui_draw::{GuiFrame, GuiTextures, rasterise};

    fn textures() -> GuiTextures {
        let font = &exa_ui::gui_font::RAYFORM_10;
        let mut textures = GuiTextures::default();
        textures.textures.insert(
            font.texture.into(),
            exa_ui::gui_draw::GuiTexture::solid([255; 4]),
        );
        textures.textures.insert(
            SKIN_TEXTURE.into(),
            exa_ui::gui_draw::GuiTexture::solid([10, 20, 30, 255]),
        );
        textures
    }

    fn frame_of(painted: &Painted) -> GuiFrame {
        let mut gui = GuiFrame::new();
        for layer in &painted.layers {
            gui.push(layer.clone());
        }
        gui
    }

    #[test]
    fn an_inventory_frame_draws_no_title_without_the_title_bit() {
        let palette = palette();
        let font = &exa_ui::gui_font::RAYFORM_10;
        let rect = GuiRect::new(8, 8, 244, inventory_height(false, false));
        let mut frame = Frame::new(rect, 4, "Inventory");
        frame.spacing = INVENTORY_SPACING;
        let painted = paint_frame(&frame, &palette, font, Some((256, 256)));
        assert_eq!(painted.client, GuiRect::new(16, 16, 228, 628));
        // The constructor's word has no title bit: one skin layer, no text.
        assert_eq!(painted.layers.len(), 1);
        assert!(!painted.layers[0].batch.primitives.is_empty());
        let pixels = rasterise(&[frame_of(&painted)], &textures(), 260, 660);
        assert!(pixels.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn the_title_bit_draws_the_title_text() {
        let palette = palette();
        let font = &exa_ui::gui_font::RAYFORM_10;
        let mut frame = Frame::new(GuiRect::new(0, 0, 244, 100), 4, "Title");
        frame.attributes |= 1;
        let painted = paint_frame(&frame, &palette, font, Some((256, 256)));
        assert_eq!(painted.layers.len(), 3);
    }

    #[test]
    fn the_pack_panel_takes_the_constructor_size_and_place() {
        let rect = GuiRect::new(8, 8, 244, inventory_height(false, false));
        let mut frame = Frame::new(rect, 4, "Inventory");
        frame.spacing = INVENTORY_SPACING;
        let client = frame.client();
        let pack = GuiRect::new(client.left, client.bottom - PACK.1, PACK.0, PACK.1);
        assert_eq!((pack.left, pack.top), (16, 464));
        assert_eq!((pack.width(), pack.height()), (228, 180));
        assert_eq!(PACK_ATTRIBUTES, 0x130032);
    }

    #[test]
    fn a_button_caption_is_placed_by_its_style_word() {
        let palette = palette();
        let font = &exa_ui::gui_font::RAYFORM_10;
        let rect = GuiRect::new(20, 40, 108, 24);
        let (x, y) = caption_origin(rect, 0x40_3320, font, b"Primary", &palette, 4);
        // Style 0x3000 centres the text; 0x300 puts its baseline half a
        // line below the middle.
        assert_eq!(x, 20 + (108 - 1 - font.text_width(b"Primary")) / 2);
        assert_eq!(y, 40 + (24 + font.line_height + 2) / 2);
        let painted = paint_button(
            rect,
            0x40_3320,
            4,
            &palette,
            font,
            Some((256, 256)),
            "Primary",
            false,
            false,
        );
        assert_eq!(painted.layers.len(), 2);
    }

    #[test]
    fn a_label_record_draws_text_only() {
        let palette = palette();
        let font = &exa_ui::gui_font::RAYFORM_10;
        let rect = GuiRect::new(4, 386, 12, 28);
        let painted = paint_label(
            rect,
            0x2300,
            4,
            &palette,
            font,
            Some((256, 256)),
            "Encumbrance:",
        );
        assert_eq!(painted.layers.len(), 1);
        assert!(
            paint_label(rect, 0x2300, 4, &palette, font, None, "x")
                .layers
                .is_empty()
        );
    }
}

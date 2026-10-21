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
//! Every guess is labelled: the shipped inventory and journal attribute
//! words do not set the title bit (0x1), while their forms give a title
//! string, so the screens set it to draw the frame and title the task
//! names. The item icons themselves wait for the item renderer; the pack
//! panel, its entry geometry and the character view's bars are drawn.

use exa_ui::form::FormMetrics;
use exa_ui::gui_core::GuiRect;
use exa_ui::gui_draw::{GuiFrames, GuiLayer, GuiTextures, LayerTexture, SKIN_TEXTURE, text_batch};
use exa_ui::gui_font::GuiFont;
use exa_ui::gui_text::draw_line;
use exa_ui::hud::{FrameTexture, HudBars, layout};
use exa_ui::line_skin::{LINE_SCHEME, line_palette};
use exa_ui::skin::{
    DispatchPlace, FormBackgroundPlace, FormChromePlace, GuiRect as SkinRect, SkinBatch,
    SkinPalette, dispatch_form, draw_button_face, draw_form_background, draw_form_chrome,
    fade_rect, form_rect,
};

/// The constructor's attribute word for an ordinary game form: a close
/// mark, a client fill and the two layer bits.
pub const FORM_ATTRIBUTES: u32 = 0x30022;
/// The title bit the screens add (a guess; see the module note).
pub const TITLE_BIT: u32 = 0x1;
/// The form's border (the skin's inset).
pub const BORDER: i32 = 2;
/// The inventory and journal forms' spacing: their controls are laid out
/// inside a client inset by eight pixels.
pub const SCREEN_SPACING: i32 = 8;
/// The item pack's size (from the inventory constructor).
pub const PACK: (i32, i32) = (228, 180);
/// The character view's width and the height the inventory gives away
/// below it (0x104 = 260).
pub const VIEW_WIDTH: i32 = 228;
pub const VIEW_BOTTOM: i32 = 260;

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
    /// A frame from its place and title. The title bit is added (guess).
    pub fn new(rect: GuiRect, unit: i32, title: &str) -> Self {
        Self {
            rect,
            unit,
            title: title.to_string(),
            attributes: FORM_ATTRIBUTES | TITLE_BIT,
            style: 0,
            state: 0,
            spacing: SCREEN_SPACING,
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

fn offset_batch(batch: &mut SkinBatch, x: i32, y: i32) {
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
    let metrics = FormMetrics {
        unit,
        border: BORDER,
    };
    let skin_rect = SkinRect::sized(0.0, 0.0, rect.width() as f32, rect.height() as f32);
    let place = FormBackgroundPlace {
        rect: skin_rect,
        inset: BORDER,
        client_top: 4,
        client_right: rect.width() - BORDER,
        client_bottom: rect.height() - BORDER,
        state: 0,
        flags: attributes,
        frame_style: style,
        margin: BORDER,
        bar: metrics.base(),
        has_lines: false,
        fill: None,
        vertical: None,
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

/// Paints a button: the skin's face and a centred caption.
pub fn paint_button(
    rect: GuiRect,
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
        let baseline = rect.top + ((font.line_height + rect.height()) as f32 * 0.5) as i32
            - (font.line_height - font.ascent);
        let (quads, _) = exa_ui::gui_text::draw_line_centred(
            font,
            label.as_bytes(),
            rect.left + 2,
            baseline,
            rect.width() - 4,
        );
        let batch = text_batch(font, &quads, palette.text[6], texture);
        painted.push(GuiLayer::new(batch, LayerTexture::Named(font.texture)));
    }
    painted
}

/// The vertical wash behind the character view, from the constructor's two
/// grey ends (0.5 and 0.75).
pub fn paint_view_background(rect: GuiRect) -> Painted {
    let mut batch = SkinBatch::new();
    batch.select(1, 0, false);
    fade_rect(
        &mut batch,
        SkinRect::sized(
            rect.left as f32,
            rect.top as f32,
            rect.width() as f32,
            rect.height() as f32,
        ),
        [191, 191, 191, 255],
        [128, 128, 128, 255],
    );
    let mut painted = Painted::default();
    painted.push(GuiLayer::new(batch, LayerTexture::None));
    painted
}

/// The inventory's preview bars, painted by the HUD's own painter into a
/// canvas and submitted as a texture layer. `shares` is
/// [`exa_ui::game_gui::preview_shares`]'s pair of words.
pub fn paint_preview_bars(rect: GuiRect, unit: i32, shares: (f32, f32)) -> Painted {
    let bars = HudBars {
        active: true,
        combat: false,
        yellow: shares.0,
        red: shares.1,
        max: 1.0,
        focus: 1.0,
        focus_cap: 1.0,
        shield: false,
        stamina: 1.0,
    };
    // The character view's callback stores each preview bar as 40 × 4
    // pixels; the place is a guess (the view draws the pair at the bottom
    // of its panel, centred with a four-pixel gap).
    let bar_width = 40;
    let gap = 4;
    let centre = rect.left + rect.width() / 2;
    let y = rect.bottom - 12;
    let bars_layout = layout(
        rect.width().max(0) as u32,
        rect.height().max(0) as u32,
        unit as u32,
    );
    let bars_layout = exa_ui::hud::Layout {
        health_x: centre - bar_width - gap / 2,
        focus_x: centre + gap / 2,
        y,
        width: bar_width,
        height: 4,
        ..bars_layout
    };
    let mut canvas = exa_ui::hud::paint(
        &bars,
        &bars_layout,
        (1.0, 1.0),
        &FrameTexture::flat(),
        rect.height().max(0) as u32,
    );
    canvas.x += rect.left;
    canvas.y += rect.top;
    let bytes = canvas.to_srgba8();
    let texture = exa_ui::gui_draw::GuiTexture::from_top_down(
        canvas.width as u32,
        canvas.height as u32,
        &bytes,
    );
    let mut painted = Painted::default();
    if let Some(texture) = texture {
        let mut batch = SkinBatch::new();
        batch.select(2, 0, true);
        batch.blend = exa_ui::gui_draw::BLEND_ALPHA;
        batch.tex_col = Some(1.0);
        batch.set_ink([255, 255, 255, 255]);
        let (x, y) = (canvas.x as f32, canvas.y as f32);
        let (w, h) = (canvas.width as f32, canvas.height as f32);
        batch.set_uv(0.0, 0.0);
        batch.vertex(x, y);
        batch.set_uv(0.0, 1.0);
        batch.vertex(x, y + h);
        batch.set_uv(1.0, 1.0);
        batch.vertex(x + w, y + h);
        batch.set_uv(1.0, 0.0);
        batch.vertex(x + w, y);
        batch.end_quads();
        painted.push(GuiLayer::new(batch, LayerTexture::Owned(texture)));
    }
    painted
}

/// The frame and title of a form, as the debug flag opens it.
pub fn paint_debug_form(rect: GuiRect, unit: i32, textures: &GuiTextures) -> Painted {
    let palette = line_palette();
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, font_texture) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let frame = Frame::new(rect, unit, "GUI Form");
    paint_frame(&frame, &palette, font, font_texture)
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
    use exa_ui::game_gui::{inventory_height, preview_shares};
    use exa_ui::gui_draw::{GuiFrame, GuiTextures, rasterise};

    #[test]
    fn an_inventory_frame_draws_a_title_and_a_pack_panel() {
        let palette = palette();
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
        let rect = GuiRect::new(8, 8, 244, inventory_height(false, false));
        let frame = Frame::new(rect, 4, "Inventory");
        let painted = paint_frame(&frame, &palette, font, Some((256, 256)));
        assert_eq!(painted.client, GuiRect::new(16, 16, 228, 628));
        let primitives: usize = painted
            .layers
            .iter()
            .map(|layer| layer.batch.primitives.len())
            .sum();
        assert!(primitives > 0);
        let title: usize = painted
            .layers
            .iter()
            .skip(1)
            .map(|layer| layer.batch.primitives.len())
            .sum();
        assert!(title >= 1);
        let mut gui = GuiFrame::new();
        for layer in &painted.layers {
            gui.push(layer.clone());
        }
        let pixels = rasterise(&[gui], &textures, 260, 660);
        assert!(pixels.iter().any(|byte| *byte != 0));
    }

    #[test]
    fn the_pack_panel_takes_the_constructor_size_and_place() {
        let rect = GuiRect::new(8, 8, 244, inventory_height(false, false));
        let frame = Frame::new(rect, 4, "Inventory");
        let client = frame.client();
        let pack = GuiRect::new(client.left, client.bottom - PACK.1, PACK.0, PACK.1);
        assert_eq!((pack.left, pack.top), (16, 464));
        assert_eq!((pack.width(), pack.height()), (228, 180));
        let view = GuiRect::new(
            client.left,
            client.top,
            VIEW_WIDTH,
            frame.rect.height() - VIEW_BOTTOM,
        );
        assert_eq!(view.height(), 384);
    }

    #[test]
    fn the_preview_bars_use_the_health_words_over_65535() {
        let shares = preview_shares(32768, 65535);
        let rect = GuiRect::new(0, 0, 228, 384);
        let painted = paint_preview_bars(rect, 4, shares);
        assert_eq!(painted.layers.len(), 1);
        assert!(matches!(painted.layers[0].texture, LayerTexture::Owned(_)));
        let primitive = &painted.layers[0].batch.primitives[0];
        assert_eq!(primitive.mode, exa_ui::skin::SkinMode::Textured);
        let vertices = painted.layers[0].batch.vertices_of(primitive);
        assert!(vertices[0].x >= 0.0 && vertices[0].x < 228.0);
        assert!(vertices[0].y >= 0.0 && vertices[0].y < 384.0);
        assert!(vertices[2].x - vertices[0].x >= 40.0);
    }
}

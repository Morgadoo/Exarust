//! The GUI draw path: a frame's [`SkinBatch`] (and the text and icon quads
//! the forms emit) submitted in the original's order.
//!
//! A frame is a list of [`GuiLayer`]s. Every layer carries one [`SkinBatch`]
//! and the texture its textured primitives sample; a layer's primitives are
//! drawn in submission order, and the layers in the order they were pushed.
//! The skin's three programs and the subclass program are reproduced as the
//! skin unit binds them (`docs/UI.md`, Skin):
//!
//! - **Solid** (mode 1): the vertex's ink.
//! - **Textured** (mode 2 or 5): the texture sampled at the vertex's
//!   coordinates, its colour mixed by `TexCol` and multiplied by the ink.
//! - **Rounded** (mode 3): the vertex's ink with the alpha the `Scale`
//!   uniform cuts out of the interpolated corner coordinate.
//! - **Skin** (mode 0x10): whatever the skin's subclass answers when asked.
//!   The line skin the game screens use answers the flat program, so its
//!   primitives draw the vertex's ink. The HUD's stone skin answers
//!   [`HUD_STONE_MODE`] and multiplies the ink by the stone, sampled in
//!   screen space, one texel a pixel, from the window's bottom.
//!
//! The blend word is the original's: the high byte's low nibble names the
//! source factor and the low nibble the destination factor, each an index
//! into a ten-entry table (zero, one, source alpha, one minus source alpha,
//! source colour, one minus source colour, destination alpha, one minus
//! destination alpha, destination colour, one minus destination colour); the
//! word 0x100 draws with blending off. The table and the exceptional word
//! are the original's blend setter. Everything is drawn at the window's
//! physical pixels, as the original draws its interface.
//!
//! The renderer is a CPU rasteriser into one window-sized RGBA image shown
//! over the 3-D view through an [`ImageNode`]. It blends in sRGB, as the
//! original's interface does, then writes straight-alpha pixels for Bevy to
//! blend in linear light (the HUD's [`crate::blend`] note applies here too).
//! A window without a `Game` resource draws nothing; a texture the
//! installation does not have is logged once and left out, never a panic.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::Game;

use crate::gui_font::GuiFont;
use crate::icon_atlas::IconQuad;
use crate::skin::{GuiDraw, GuiPrimitive, GuiVertex, SkinBatch, SkinInk, SkinMode};
use crate::text_layout::GlyphQuad;

/// The skin's atlas (`guiskin.rfi`).
pub const SKIN_TEXTURE: &str = "guiskin";
/// The mode the HUD's stone skin answers when the skin asks its subclass for
/// the mode of a primitive drawn in the subclass' own mode.
pub const HUD_STONE_MODE: u32 = 4;
/// The blend word that draws a control normally: source alpha over one
/// minus source alpha.
pub const BLEND_ALPHA: u16 = 0x203;
/// The blend word that draws additively: source alpha plus the destination.
pub const BLEND_ADDITIVE: u16 = 0x201;
/// The blend word that draws premultiplied: one over one minus source alpha.
pub const BLEND_PREMULTIPLIED: u16 = 0x103;
/// The blend word that draws a replacement with blending off.
pub const BLEND_REPLACE: u16 = 0x100;

/// One blend factor of the original's table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendFactor {
    Zero,
    One,
    SourceColour,
    OneMinusSourceColour,
    SourceAlpha,
    OneMinusSourceAlpha,
    DestinationAlpha,
    OneMinusDestinationAlpha,
    DestinationColour,
    OneMinusDestinationColour,
}

/// The original's blend factor table: ten entries, the GL factors in the
/// order zero, one, source alpha, one minus source alpha, source colour,
/// one minus source colour, destination alpha, one minus destination alpha,
/// destination colour, one minus destination colour. An index past the
/// table names no factor and draws as zero.
pub fn blend_factor(index: u32) -> BlendFactor {
    match index & 0xf {
        1 => BlendFactor::One,
        2 => BlendFactor::SourceAlpha,
        3 => BlendFactor::OneMinusSourceAlpha,
        4 => BlendFactor::SourceColour,
        5 => BlendFactor::OneMinusSourceColour,
        6 => BlendFactor::DestinationAlpha,
        7 => BlendFactor::OneMinusDestinationAlpha,
        8 => BlendFactor::DestinationColour,
        9 => BlendFactor::OneMinusDestinationColour,
        _ => BlendFactor::Zero,
    }
}

/// The source and destination factors a blend word names: the high byte's
/// low nibble is the source, the low nibble the destination.
pub fn blend_factors(blend: u16) -> (BlendFactor, BlendFactor) {
    (
        blend_factor(u32::from(blend >> 8)),
        blend_factor(u32::from(blend)),
    )
}

/// The blend word whose factors are `(source, destination)`; the inverse of
/// [`blend_factors`] for the table's entries.
pub fn blend_word(source: BlendFactor, destination: BlendFactor) -> u16 {
    fn index(factor: BlendFactor) -> u16 {
        match factor {
            BlendFactor::Zero => 0,
            BlendFactor::One => 1,
            BlendFactor::SourceAlpha => 2,
            BlendFactor::OneMinusSourceAlpha => 3,
            BlendFactor::SourceColour => 4,
            BlendFactor::OneMinusSourceColour => 5,
            BlendFactor::DestinationAlpha => 6,
            BlendFactor::OneMinusDestinationAlpha => 7,
            BlendFactor::DestinationColour => 8,
            BlendFactor::OneMinusDestinationColour => 9,
        }
    }
    index(source) << 8 | index(destination)
}

/// A decoded texture the draw path samples.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuiTexture {
    pub width: u32,
    pub height: u32,
    /// Straight RGBA, row 0 at the top.
    pub pixels: Vec<[u8; 4]>,
}

impl GuiTexture {
    /// A texture from raw RGBA bytes whose rows already run top-down (a
    /// CPU-painted canvas).
    pub fn from_top_down(width: u32, height: u32, bytes: &[u8]) -> Option<Self> {
        let count = (width as usize).checked_mul(height as usize)?;
        let bytes = bytes.get(..count.checked_mul(4)?)?;
        Some(Self {
            width,
            height,
            pixels: bytes.as_chunks::<4>().0.to_vec(),
        })
    }

    /// A texture from raw RGBA bytes whose rows run bottom-up (the `.rfi`
    /// decoder's order) converted to the top-down order the GUI uses.
    pub fn from_bottom_up(width: u32, height: u32, bytes: &[u8]) -> Option<Self> {
        let count = (width as usize).checked_mul(height as usize)?;
        let pixels = bytes.get(..count.checked_mul(4)?)?;
        let mut out = vec![[0u8; 4]; count];
        for y in 0..height as usize {
            let source = (height as usize - 1 - y) * width as usize;
            let target = y * width as usize;
            for x in 0..width as usize {
                let i = (source + x) * 4;
                out[target + x] = [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]];
            }
        }
        Some(Self {
            width,
            height,
            pixels: out,
        })
    }

    /// The texel at `(x, y)`, wrapped like the original's `GL_REPEAT`.
    pub fn texel(&self, x: i64, y: i64) -> [u8; 4] {
        if self.width == 0 || self.height == 0 || self.pixels.is_empty() {
            return [0; 4];
        }
        let w = i64::from(self.width);
        let h = i64::from(self.height);
        let px = x.rem_euclid(w) as usize;
        let py = y.rem_euclid(h) as usize;
        self.pixels
            .get(py * self.width as usize + px)
            .copied()
            .unwrap_or([0; 4])
    }

    /// The texel a normalized coordinate samples with nearest-neighbour
    /// filtering (the original's GUI sampler).
    pub fn sample(&self, u: f32, v: f32) -> [u8; 4] {
        if !u.is_finite() || !v.is_finite() || self.width == 0 || self.height == 0 {
            return [0; 4];
        }
        let x = (u * self.width as f32).floor();
        let y = (v * self.height as f32).floor();
        let x = if x.is_finite() {
            x as i64
        } else {
            return [0; 4];
        };
        let y = if y.is_finite() {
            y as i64
        } else {
            return [0; 4];
        };
        self.texel(x, y)
    }

    /// A solid texture of one texel, for tests and placeholders.
    pub fn solid(ink: [u8; 4]) -> Self {
        Self {
            width: 1,
            height: 1,
            pixels: vec![ink],
        }
    }
}

/// The textures the interface samples, by the name the skin unit uses.
#[derive(Resource, Default)]
pub struct GuiTextures {
    pub textures: HashMap<String, GuiTexture>,
    /// Names already reported missing, so the error is logged once.
    pub missing: Vec<String>,
}

impl GuiTextures {
    /// The texture `name`, if it has been loaded.
    pub fn get(&self, name: &str) -> Option<&GuiTexture> {
        self.textures.get(name)
    }
}

/// The texture a layer's textured primitives sample.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerTexture {
    /// No texture: the solid and rounded programs only.
    None,
    /// A texture by its name in the resource pack (no extension).
    Named(&'static str),
    /// A texture the caller built (a CPU-painted canvas, for one).
    Owned(GuiTexture),
}

/// One submitted layer: a batch, its texture and the clip it draws in.
#[derive(Debug, Clone, PartialEq)]
pub struct GuiLayer {
    pub batch: SkinBatch,
    pub texture: LayerTexture,
    /// Clip rectangle `[x, y, width, height]` in window pixels. Layers with
    /// no clip use the window.
    pub clip: Option<[i32; 4]>,
}

impl GuiLayer {
    /// A layer over the whole window.
    pub fn new(batch: SkinBatch, texture: LayerTexture) -> Self {
        Self {
            batch,
            texture,
            clip: None,
        }
    }

    /// The layer clipped to `rect`.
    pub fn clipped(mut self, rect: [i32; 4]) -> Self {
        self.clip = Some(rect);
        self
    }
}

/// One frame: its layers, drawn in order.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GuiFrame {
    pub layers: Vec<GuiLayer>,
}

impl GuiFrame {
    /// An empty frame.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends a layer.
    pub fn push(&mut self, layer: GuiLayer) {
        self.layers.push(layer);
    }

    /// Whether the frame would draw nothing.
    pub fn is_empty(&self) -> bool {
        self.layers
            .iter()
            .all(|layer| layer.batch.primitives.is_empty() || layer.batch.vertices.is_empty())
    }
}

/// The frames a frame's systems submit. The GUI's draw systems clear it
/// before pushing, and the renderer takes it each frame.
#[derive(Resource, Default, Debug, Clone)]
pub struct GuiFrames {
    pub frames: Vec<GuiFrame>,
}

impl GuiFrames {
    /// Removes every frame.
    pub fn clear(&mut self) {
        self.frames.clear();
    }

    /// Adds a frame.
    pub fn push(&mut self, frame: GuiFrame) {
        self.frames.push(frame);
    }

    /// The total number of primitives across every layer.
    pub fn primitives(&self) -> usize {
        self.frames
            .iter()
            .flat_map(|frame| frame.layers.iter())
            .map(|layer| layer.batch.primitives.len())
            .sum()
    }
}

/// Builds a textured quad batch from a font's line quads: one primitive per
/// line, normal alpha blending and the ink as the tint. `texture` is the
/// atlas's size, whose cell a quad's corner plus the font's sample step
/// spans.
pub fn text_batch(
    font: &GuiFont,
    quads: &[crate::gui_text::TextQuad],
    ink: SkinInk,
    texture: (u16, u16),
) -> SkinBatch {
    let (du, dv) = crate::gui_text::font_uv(font, texture).unwrap_or((0.0, 0.0));
    let mut batch = SkinBatch::new();
    batch.select(2, 0, true);
    batch.blend = BLEND_ALPHA;
    batch.tex_col = Some(1.0);
    batch.begin();
    for quad in quads {
        let [x, top, w, h] = quad.rect;
        let (u0, v0) = quad.cell;
        batch.set_ink(ink);
        batch.set_uv(u0, v0 + dv);
        batch.vertex(x as f32, top as f32);
        batch.set_uv(u0, v0);
        batch.vertex(x as f32, (top + h) as f32);
        batch.set_uv(u0 + du, v0);
        batch.vertex((x + w) as f32, (top + h) as f32);
        batch.set_uv(u0 + du, v0 + dv);
        batch.vertex((x + w) as f32, top as f32);
    }
    batch.end_quads();
    batch
}

/// Builds a textured quad batch from a wrapped text draw's glyph quads (the
/// text box's form), in the draw's order, with each glyph's own ink.
pub fn glyph_batch(
    font: &GuiFont,
    quads: &[GlyphQuad],
    texture: (u16, u16),
    slant: f32,
) -> SkinBatch {
    let (du, dv) = crate::gui_text::font_uv(font, texture).unwrap_or((0.0, 0.0));
    let mut batch = SkinBatch::new();
    batch.select(2, 0, true);
    batch.blend = BLEND_ALPHA;
    batch.tex_col = Some(1.0);
    batch.begin();
    for quad in quads {
        let (u0, v0) = crate::gui_text::glyph_cell(quad.byte);
        let x = quad.x as f32;
        let y = quad.y as f32;
        let w = font.glyph.0 as f32;
        let h = font.glyph.1 as f32;
        let lean = quad.slant * slant;
        batch.set_ink(quad.ink);
        batch.set_uv(u0, v0 + dv);
        batch.vertex(x + lean.max(0.0), y);
        batch.set_uv(u0, v0);
        batch.vertex(x, y + h);
        batch.set_uv(u0 + du, v0);
        batch.vertex(x + w, y + h);
        batch.set_uv(u0 + du, v0 + dv);
        batch.vertex(x + w + lean.max(0.0), y);
    }
    batch.end_quads();
    batch
}

/// Builds one textured batch from icon quads. Every quad keeps its own blend
/// and ink; a quad without an ink uses white.
pub fn icon_batch(quads: &[IconQuad], ink: SkinInk) -> SkinBatch {
    let mut batch = SkinBatch::new();
    batch.select(2, 0, true);
    batch.tex_col = Some(1.0);
    for quad in quads {
        batch.blend = if quad.blend == 0 {
            BLEND_ALPHA
        } else {
            (quad.blend & 0xffff) as u16
        };
        batch.set_ink(quad.ink.unwrap_or(ink));
        batch.begin();
        for corner in &quad.corners {
            batch.set_uv(corner[2], corner[3]);
            batch.vertex(corner[0], corner[1]);
        }
        batch.end_quads();
    }
    batch
}

/// One decoded GUI layer ready to rasterise: its primitives and the texels
/// they sample.
struct RasterLayer<'a> {
    batch: &'a SkinBatch,
    texture: Option<&'a GuiTexture>,
    clip: [i32; 4],
}

/// The GUI renderer's canvas: the image it writes and the entity that shows
/// it.
#[derive(Resource, Default)]
pub struct GuiCanvas {
    image: Option<Handle<Image>>,
    entity: Option<Entity>,
    size: (u32, u32),
    /// The frame counter the canvas was last drawn at.
    drawn: u64,
    /// The frames the canvas image shows. A screen that paints the same
    /// frames again keeps the image instead of rasterising and uploading a
    /// window-sized one every frame.
    last: Vec<GuiFrame>,
}

/// Draws every submitted frame over the 3-D view, once a frame.
pub struct GuiDrawPlugin;

impl Plugin for GuiDrawPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GuiFrames>()
            .init_resource::<GuiTextures>()
            .init_resource::<GuiCanvas>()
            .add_systems(Startup, load_gui_textures)
            .add_systems(PostUpdate, render_frames);
    }
}

/// Loads the skin's atlas and the main and small fonts of the current GUI
/// level. A missing texture is reported once and left out.
fn load_gui_textures(
    game: Option<Res<Game>>,
    scale: Option<Res<crate::scale::GuiScale>>,
    mut textures: ResMut<GuiTextures>,
) {
    let Some(game) = game else {
        return;
    };
    let unit = scale.as_deref().map_or(4, |scale| scale.scale as i32);
    let level = crate::gui_font::level_for_unit(unit);
    let fonts = crate::gui_font::gui_level(level);
    for name in [
        SKIN_TEXTURE,
        fonts.main.texture,
        fonts.small.texture,
        crate::gui_font::FONTBASE_24.texture,
    ] {
        if textures.textures.contains_key(name) || textures.missing.iter().any(|n| n == name) {
            continue;
        }
        match game.load_rgba(name) {
            Ok(image) => {
                let Some(texture) =
                    GuiTexture::from_bottom_up(image.width, image.height, &image.pixels)
                else {
                    continue;
                };
                textures.textures.insert(name.to_string(), texture);
            }
            Err(error) => {
                error!("GUI texture {name:?}: {error}");
                textures.missing.push(name.to_string());
            }
        }
    }
}

fn blend_channel(source: u8, destination: u8, source_factor: f32, destination_factor: f32) -> f32 {
    (f32::from(source) * source_factor + f32::from(destination) * destination_factor)
        .clamp(0.0, 255.0)
}

fn factor_of(factor: BlendFactor, source: [f32; 4], destination: [f32; 4]) -> [f32; 3] {
    let colour = |value: [f32; 4]| [value[0], value[1], value[2]];
    let alpha = |value: f32| [value; 3];
    let one_minus = |value: [f32; 3]| [1.0 - value[0], 1.0 - value[1], 1.0 - value[2]];
    match factor {
        BlendFactor::Zero => alpha(0.0),
        BlendFactor::One => alpha(1.0),
        BlendFactor::SourceColour => colour(source),
        BlendFactor::OneMinusSourceColour => one_minus(colour(source)),
        BlendFactor::SourceAlpha => alpha(source[3]),
        BlendFactor::OneMinusSourceAlpha => alpha(1.0 - source[3]),
        BlendFactor::DestinationAlpha => alpha(destination[3]),
        BlendFactor::OneMinusDestinationAlpha => alpha(1.0 - destination[3]),
        BlendFactor::DestinationColour => colour(destination),
        BlendFactor::OneMinusDestinationColour => one_minus(colour(destination)),
    }
}

/// The colour a primitive's shader gives a fragment before blending.
fn shade(
    mode: SkinMode,
    ink: [f32; 4],
    uv: (f32, f32),
    primitive: &GuiPrimitive,
    texture: Option<&GuiTexture>,
    viewport: (f32, f32),
    place: (f32, f32),
) -> [f32; 4] {
    match mode {
        SkinMode::Solid | SkinMode::None => ink,
        SkinMode::Rounded => {
            let scale = primitive.rounded_scale.unwrap_or([0.0; 4]);
            let sx = (0.5 - uv.0).abs();
            let sy = (0.5 - uv.1).abs();
            let tx = (sx - scale[0]) * scale[2];
            let ty = (sy - scale[1]) * scale[3];
            let length = (tx.max(0.0) * tx.max(0.0) + ty.max(0.0) * ty.max(0.0)).sqrt();
            let mut a = (1.0 - length).max(0.0);
            a *= a;
            a *= a;
            [ink[0], ink[1], ink[2], ink[3] * a]
        }
        SkinMode::Textured => {
            let Some(texture) = texture else {
                return [0.0; 4];
            };
            let texel = texture.sample(uv.0, uv.1);
            let tex = [
                f32::from(texel[0]) / 255.0,
                f32::from(texel[1]) / 255.0,
                f32::from(texel[2]) / 255.0,
                f32::from(texel[3]) / 255.0,
            ];
            let tex_col = primitive.tex_col.unwrap_or(1.0);
            [
                ink[0] * (tex[0] * tex_col + 1.0 - tex_col),
                ink[1] * (tex[1] * tex_col + 1.0 - tex_col),
                ink[2] * (tex[2] * tex_col + 1.0 - tex_col),
                ink[3] * tex[3],
            ]
        }
        SkinMode::Skin if primitive.subclass == HUD_STONE_MODE => {
            let Some(texture) = texture else {
                return [0.0; 4];
            };
            // The stone is sampled in screen space, one texel a pixel, from
            // the window's bottom: stored row (height - 1 - y), column x,
            // each moved by the texture offset (kept in 512ths).
            let column = (place.0 + primitive.texture_shift[0] * 512.0).floor();
            let row = (viewport.1 - place.1 + primitive.texture_shift[1] * 512.0).floor();
            let texel = texture.texel(column as i64, texture.height as i64 - 1 - row as i64);
            [
                ink[0] * f32::from(texel[0]) / 255.0,
                ink[1] * f32::from(texel[1]) / 255.0,
                ink[2] * f32::from(texel[2]) / 255.0,
                ink[3] * f32::from(texel[3]) / 255.0,
            ]
        }
        // Any other answer of the subclass is the flat program: the line
        // skin game screens use binds it and draws the vertex's ink.
        SkinMode::Skin => ink,
    }
}

/// Whether a barycentric weight is inside its edge: past it, or on it when
/// the triangle owns the edge.
fn inside(weight: f32, owns_edge: bool) -> bool {
    weight > 0.0 || (weight == 0.0 && owns_edge)
}

/// One primitive's vertices as triangles: (indices into the vertex slice).
fn triangles(draw: GuiDraw, count: usize) -> Vec<[usize; 3]> {
    let mut out = Vec::new();
    match draw {
        GuiDraw::Quads => {
            let mut i = 0;
            while i + 3 < count {
                out.push([i, i + 1, i + 2]);
                out.push([i, i + 2, i + 3]);
                i += 4;
            }
        }
        GuiDraw::Triangles => {
            let mut i = 0;
            while i + 2 < count {
                out.push([i, i + 1, i + 2]);
                i += 3;
            }
        }
        GuiDraw::Lines => {}
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn raster_primitive(
    batch: &SkinBatch,
    primitive: &GuiPrimitive,
    texture: Option<&GuiTexture>,
    clip: [i32; 4],
    canvas: &mut [f32],
    width: usize,
    height: usize,
) {
    let vertices = batch.vertices_of(primitive);
    if vertices.is_empty() {
        return;
    }
    let viewport = (width as f32, height as f32);
    // A batch that never named a word draws with the GUI's steady-state one.
    let blend = if primitive.blend == 0 {
        BLEND_ALPHA
    } else {
        primitive.blend
    };
    let (source, destination) = blend_factors(blend);
    let disabled = blend & 0x3f0f == BLEND_REPLACE & 0x3f0f;
    let vertex_at = |i: usize| -> [f32; 4] {
        let v: &GuiVertex = &vertices[i];
        [
            v.ink[0] as f32 / 255.0,
            v.ink[1] as f32 / 255.0,
            v.ink[2] as f32 / 255.0,
            v.ink[3] as f32 / 255.0,
        ]
    };
    let mut plot = |x: i32, y: i32, ink: [f32; 4], uv: (f32, f32)| {
        if x < clip[0]
            || y < clip[1]
            || x >= clip[0] + clip[2]
            || y >= clip[1] + clip[3]
            || x < 0
            || y < 0
            || x as usize >= width
            || y as usize >= height
        {
            return;
        }
        let fragment = shade(
            primitive.mode,
            ink,
            uv,
            primitive,
            texture,
            viewport,
            (x as f32 + 0.5, y as f32 + 0.5),
        );
        let index = (y as usize * width + x as usize) * 4;
        let Some(pixel) = canvas.get_mut(index..index + 4) else {
            return;
        };
        let old = [pixel[0], pixel[1], pixel[2], pixel[3]];
        if disabled {
            pixel.copy_from_slice(&fragment);
            return;
        }
        let sf = factor_of(source, fragment, old);
        let df = factor_of(destination, fragment, old);
        // The visible result is the original's glBlendFunc over the pixel
        // below. The alpha channel is the overlay's coverage, not a GL blend
        // target (the original draws on the window, whose alpha is unused),
        // so it accumulates the source's coverage.
        for channel in 0..3 {
            pixel[channel] = blend_channel(
                (fragment[channel] * 255.0) as u8,
                (old[channel] * 255.0) as u8,
                sf[channel],
                df[channel],
            ) / 255.0;
        }
        pixel[3] = (fragment[3] + old[3] * (1.0 - fragment[3])).clamp(0.0, 1.0);
    };
    if primitive.draw == GuiDraw::Lines {
        let mut i = 0;
        while i + 1 < vertices.len() {
            let a = vertices[i];
            let b = vertices[i + 1];
            let steps = ((b.x - a.x).abs().max((b.y - a.y).abs()) as i32).max(1);
            for step in 0..=steps {
                let t = step as f32 / steps as f32;
                let x = (a.x + (b.x - a.x) * t).round() as i32;
                let y = (a.y + (b.y - a.y) * t).round() as i32;
                let ink = [
                    a.ink[0] as f32 / 255.0 + (b.ink[0] as f32 - a.ink[0] as f32) / 255.0 * t,
                    a.ink[1] as f32 / 255.0 + (b.ink[1] as f32 - a.ink[1] as f32) / 255.0 * t,
                    a.ink[2] as f32 / 255.0 + (b.ink[2] as f32 - a.ink[2] as f32) / 255.0 * t,
                    a.ink[3] as f32 / 255.0 + (b.ink[3] as f32 - a.ink[3] as f32) / 255.0 * t,
                ];
                plot(x, y, ink, (a.u + (b.u - a.u) * t, a.v + (b.v - a.v) * t));
            }
            i += 2;
        }
        return;
    }
    for [i0, i1, i2] in triangles(primitive.draw, vertices.len()) {
        let (v0, v1, v2) = (vertices[i0], vertices[i1], vertices[i2]);
        let min_x = v0.x.min(v1.x).min(v2.x).floor().max(clip[0] as f32) as i32;
        let max_x =
            v0.x.max(v1.x)
                .max(v2.x)
                .ceil()
                .min((clip[0] + clip[2] - 1) as f32) as i32;
        let min_y = v0.y.min(v1.y).min(v2.y).floor().max(clip[1] as f32) as i32;
        let max_y =
            v0.y.max(v1.y)
                .max(v2.y)
                .ceil()
                .min((clip[1] + clip[3] - 1) as f32) as i32;
        let area = (v1.x - v0.x) * (v2.y - v0.y) - (v2.x - v0.x) * (v1.y - v0.y);
        if area.abs() < f32::EPSILON {
            continue;
        }
        // For each edge, whether this triangle owns a pixel centre lying on
        // it: the edge, walked with the inside on the same side for both
        // triangles that share it, points down or along the row leftwards.
        let (first, second) = if area > 0.0 { (v1, v2) } else { (v2, v1) };
        let owned = |a: &GuiVertex, b: &GuiVertex| b.y > a.y || (b.y == a.y && b.x < a.x);
        let owns = [
            owned(&first, &second),
            owned(
                if area > 0.0 { &v2 } else { &v0 },
                if area > 0.0 { &v0 } else { &v2 },
            ),
            owned(
                if area > 0.0 { &v0 } else { &v1 },
                if area > 0.0 { &v1 } else { &v0 },
            ),
        ];
        let i0ink = vertex_at(i0);
        let i1ink = vertex_at(i1);
        let i2ink = vertex_at(i2);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let w0 = ((v1.x - px) * (v2.y - py) - (v2.x - px) * (v1.y - py)) / area;
                let w1 = ((v2.x - px) * (v0.y - py) - (v0.x - px) * (v2.y - py)) / area;
                let w2 = ((v0.x - px) * (v1.y - py) - (v1.x - px) * (v0.y - py)) / area;
                // A pixel centre exactly on an edge belongs to one of the
                // two triangles that share it, as the hardware's fill rule
                // has it, so a translucent quad blends its diagonal once.
                if !(inside(w0, owns[0]) && inside(w1, owns[1]) && inside(w2, owns[2])) {
                    continue;
                }
                let ink = [
                    i0ink[0] * w0 + i1ink[0] * w1 + i2ink[0] * w2,
                    i0ink[1] * w0 + i1ink[1] * w1 + i2ink[1] * w2,
                    i0ink[2] * w0 + i1ink[2] * w1 + i2ink[2] * w2,
                    i0ink[3] * w0 + i1ink[3] * w1 + i2ink[3] * w2,
                ];
                let uv = (
                    v0.u * w0 + v1.u * w1 + v2.u * w2,
                    v0.v * w0 + v1.v * w1 + v2.v * w2,
                );
                plot(x, y, ink, uv);
            }
        }
    }
}

/// Rasterises the submitted frames into a straight-alpha RGBA image.
pub fn rasterise(frames: &[GuiFrame], textures: &GuiTextures, width: u32, height: u32) -> Vec<u8> {
    let mut canvas = vec![0.0f32; width as usize * height as usize * 4];
    let window = [0, 0, width as i32, height as i32];
    let mut layers = Vec::new();
    for frame in frames {
        for layer in &frame.layers {
            let texture = match &layer.texture {
                LayerTexture::None => None,
                LayerTexture::Named(name) => textures.get(name),
                LayerTexture::Owned(texture) => Some(texture),
            };
            layers.push(RasterLayer {
                batch: &layer.batch,
                texture,
                clip: layer.clip.unwrap_or(window),
            });
        }
    }
    for layer in &layers {
        for primitive in &layer.batch.primitives {
            raster_primitive(
                layer.batch,
                primitive,
                layer.texture,
                layer.clip,
                &mut canvas,
                width as usize,
                height as usize,
            );
        }
    }
    // The original's blending leaves premultiplied colour in a transparent
    // target; the image node blends straight alpha, so unpremultiply.
    let mut out = vec![0u8; width as usize * height as usize * 4];
    for (i, pixel) in canvas.as_chunks::<4>().0.iter().enumerate() {
        let alpha = pixel[3];
        let index = i * 4;
        if alpha <= 0.0 {
            out[index + 3] = 0;
            continue;
        }
        for channel in 0..3 {
            out[index + channel] =
                ((pixel[channel] / alpha) * 255.0).round().clamp(0.0, 255.0) as u8;
        }
        out[index + 3] = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    out
}

fn render_frames(
    mut commands: Commands,
    mut frames: ResMut<GuiFrames>,
    textures: Res<GuiTextures>,
    mut canvas: ResMut<GuiCanvas>,
    images: Option<ResMut<Assets<Image>>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let Some(mut images) = images else {
        frames.clear();
        return;
    };
    if frames.frames.is_empty() || frames.frames.iter().all(GuiFrame::is_empty) {
        if !frames.frames.is_empty() {
            frames.clear();
        }
        if let Some(entity) = canvas.entity
            && let Ok((_, _, mut visibility)) = nodes.get_mut(entity)
        {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let Ok(window) = windows.single() else {
        frames.clear();
        return;
    };
    let width = window.physical_width();
    let height = window.physical_height();
    if width == 0 || height == 0 {
        frames.clear();
        return;
    }
    if canvas.size == (width, height) && canvas.image.is_some() && canvas.last == frames.frames {
        frames.clear();
        if let Some(entity) = canvas.entity
            && let Ok((_, _, mut visibility)) = nodes.get_mut(entity)
        {
            *visibility = Visibility::Inherited;
        }
        return;
    }
    let pixels = rasterise(&frames.frames, &textures, width, height);
    canvas.last = std::mem::take(&mut frames.frames);
    let image = Image::new(
        Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    canvas.size = (width, height);
    canvas.drawn = canvas.drawn.wrapping_add(1);
    match canvas.image.as_ref() {
        Some(handle) => {
            if let Some(mut existing) = images.get_mut(handle) {
                *existing = image;
            } else {
                let handle = images.add(image);
                canvas.image = Some(handle.clone());
                if let Some(entity) = canvas.entity
                    && let Ok((mut image_node, _, _)) = nodes.get_mut(entity)
                {
                    image_node.image = handle;
                }
            }
        }
        None => {
            let handle = images.add(image);
            canvas.image = Some(handle.clone());
            let logical = (
                width as f32 / window.scale_factor(),
                height as f32 / window.scale_factor(),
            );
            let entity = commands
                .spawn((
                    Name::new("GuiDraw"),
                    ImageNode {
                        image: handle,
                        ..default()
                    },
                    Node {
                        position_type: PositionType::Absolute,
                        left: Val::Px(0.0),
                        top: Val::Px(0.0),
                        width: Val::Px(logical.0),
                        height: Val::Px(logical.1),
                        ..default()
                    },
                    GlobalZIndex(200),
                    Visibility::Inherited,
                ))
                .id();
            canvas.entity = Some(entity);
        }
    }
    if let Some(entity) = canvas.entity
        && let Ok((_, mut node, mut visibility)) = nodes.get_mut(entity)
    {
        node.width = Val::Px(width as f32 / window.scale_factor());
        node.height = Val::Px(height as f32 / window.scale_factor());
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin::GuiVertex;

    fn vertex(x: f32, y: f32, ink: [u8; 4]) -> GuiVertex {
        GuiVertex {
            x,
            y,
            u: 0.0,
            v: 0.0,
            ink,
        }
    }

    fn solid_frame(vertices: Vec<GuiVertex>, blend: u16) -> GuiFrame {
        let mut batch = SkinBatch::new();
        batch.mode = SkinMode::Solid;
        batch.blend = blend;
        let first = 0;
        let count = vertices.len();
        batch.vertices = vertices;
        batch.primitives.push(GuiPrimitive {
            mode: SkinMode::Solid,
            blend,
            draw: GuiDraw::Quads,
            texture_shift: [0.0; 2],
            rounded_scale: None,
            tex_col: None,
            subclass: 0,
            first,
            count,
        });
        let mut frame = GuiFrame::new();
        frame.push(GuiLayer::new(batch, LayerTexture::None));
        frame
    }

    #[test]
    fn the_blend_words_are_the_originals() {
        assert_eq!(
            blend_factors(0x203),
            (BlendFactor::SourceAlpha, BlendFactor::OneMinusSourceAlpha)
        );
        assert_eq!(
            blend_factors(0x201),
            (BlendFactor::SourceAlpha, BlendFactor::One)
        );
        assert_eq!(
            blend_factors(0x103),
            (BlendFactor::One, BlendFactor::OneMinusSourceAlpha)
        );
        assert_eq!(blend_factors(0x100), (BlendFactor::One, BlendFactor::Zero));
        assert_eq!(
            blend_word(BlendFactor::SourceAlpha, BlendFactor::One),
            0x201
        );
        // Index past the table's ten entries names no factor.
        assert_eq!(blend_factor(10), BlendFactor::Zero);
        assert_eq!(blend_factor(15), BlendFactor::Zero);
    }

    #[test]
    fn the_factor_table_has_ten_entries_in_the_originals_order() {
        use BlendFactor::*;
        let table = [
            Zero,
            One,
            SourceAlpha,
            OneMinusSourceAlpha,
            SourceColour,
            OneMinusSourceColour,
            DestinationAlpha,
            OneMinusDestinationAlpha,
            DestinationColour,
            OneMinusDestinationColour,
        ];
        for (index, factor) in table.into_iter().enumerate() {
            assert_eq!(blend_factor(index as u32), factor, "entry {index}");
            assert_eq!(
                blend_word(factor, Zero),
                (index as u16) << 8,
                "entry {index}"
            );
        }
    }

    #[test]
    fn a_colour_factor_weights_each_channel_by_its_own_value() {
        // Source colour over destination colour: each channel multiplies the
        // two, so a red source over a half-red, full-blue pixel keeps only
        // the red channel.
        let base = solid_frame(
            vec![
                vertex(0.0, 0.0, [128, 0, 255, 255]),
                vertex(0.0, 2.0, [128, 0, 255, 255]),
                vertex(2.0, 2.0, [128, 0, 255, 255]),
                vertex(2.0, 0.0, [128, 0, 255, 255]),
            ],
            BLEND_REPLACE,
        );
        let mut second = solid_frame(
            vec![
                vertex(0.0, 0.0, [255, 255, 0, 255]),
                vertex(0.0, 2.0, [255, 255, 0, 255]),
                vertex(2.0, 2.0, [255, 255, 0, 255]),
                vertex(2.0, 0.0, [255, 255, 0, 255]),
            ],
            blend_word(BlendFactor::Zero, BlendFactor::SourceColour),
        );
        let mut frame = base;
        frame.layers.append(&mut second.layers);
        let pixels = rasterise(&[frame], &GuiTextures::default(), 2, 2);
        assert_eq!(&pixels[0..4], &[128, 0, 0, 255]);
    }

    #[test]
    fn a_primitive_without_a_blend_word_draws_with_the_steady_state_one() {
        let frame = solid_frame(
            vec![
                vertex(0.0, 0.0, [10, 20, 30, 255]),
                vertex(0.0, 2.0, [10, 20, 30, 255]),
                vertex(2.0, 2.0, [10, 20, 30, 255]),
                vertex(2.0, 0.0, [10, 20, 30, 255]),
            ],
            0,
        );
        let pixels = rasterise(&[frame], &GuiTextures::default(), 2, 2);
        assert_eq!(&pixels[0..4], &[10, 20, 30, 255]);
    }

    #[test]
    fn a_solid_quad_fills_its_pixels_in_srgb() {
        let frame = solid_frame(
            vec![
                vertex(0.0, 0.0, [255, 0, 0, 255]),
                vertex(0.0, 2.0, [255, 0, 0, 255]),
                vertex(2.0, 2.0, [255, 0, 0, 255]),
                vertex(2.0, 0.0, [255, 0, 0, 255]),
            ],
            BLEND_REPLACE,
        );
        let textures = GuiTextures::default();
        let pixels = rasterise(&[frame], &textures, 3, 3);
        let at = |x: usize, y: usize| {
            let i = (y * 3 + x) * 4;
            [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
        };
        assert_eq!(at(1, 1), [255, 0, 0, 255]);
        assert_eq!(at(2, 2), [0, 0, 0, 0]);
    }

    #[test]
    fn an_additive_quad_adds_to_what_is_below() {
        let base = solid_frame(
            vec![
                vertex(0.0, 0.0, [0, 0, 255, 255]),
                vertex(0.0, 2.0, [0, 0, 255, 255]),
                vertex(2.0, 2.0, [0, 0, 255, 255]),
                vertex(2.0, 0.0, [0, 0, 255, 255]),
            ],
            BLEND_REPLACE,
        );
        let mut second = solid_frame(
            vec![
                vertex(0.0, 0.0, [255, 0, 0, 128]),
                vertex(0.0, 2.0, [255, 0, 0, 128]),
                vertex(2.0, 2.0, [255, 0, 0, 128]),
                vertex(2.0, 0.0, [255, 0, 0, 128]),
            ],
            BLEND_ADDITIVE,
        );
        let mut frame = base;
        frame.layers.append(&mut second.layers);
        let pixels = rasterise(&[frame], &GuiTextures::default(), 2, 2);
        // Red added at half alpha over the blue, once even on the quad's
        // diagonal: (128, 0, 255).
        assert_eq!(&pixels[0..4], &[128, 0, 255, 255]);
        assert_eq!(&pixels[12..16], &[128, 0, 255, 255]);
    }

    #[test]
    fn the_textured_mode_applies_tex_col_before_the_ink() {
        let texture = GuiTexture {
            width: 2,
            height: 1,
            pixels: vec![[128, 128, 128, 255], [255, 255, 255, 255]],
        };
        let mut textures = GuiTextures::default();
        textures.textures.insert("t".into(), texture);
        let mut batch = SkinBatch::new();
        batch.mode = SkinMode::Textured;
        batch.blend = BLEND_REPLACE;
        batch.tex_col = Some(0.5);
        batch.set_ink([200, 100, 50, 255]);
        batch.set_uv(0.0, 0.5);
        batch.vertex(0.0, 0.0);
        batch.set_uv(1.0, 0.5);
        batch.vertex(2.0, 0.0);
        batch.set_uv(1.0, 0.5);
        batch.vertex(2.0, 1.0);
        batch.set_uv(0.0, 0.5);
        batch.vertex(0.0, 1.0);
        batch.end_quads();
        let mut frame = GuiFrame::new();
        frame.push(GuiLayer::new(batch, LayerTexture::Named("t")));
        let pixels = rasterise(&[frame], &textures, 2, 1);
        // Grey 128/255 times 0.5 plus 0.5 is 0.75; so 200 * 0.75 = 150.
        assert_eq!(&pixels[0..4], &[150, 75, 38, 255]);
    }

    fn skin_frame(subclass: u32, shift: [f32; 2]) -> GuiFrame {
        let mut batch = SkinBatch::new();
        batch.mode = SkinMode::Skin;
        batch.subclass_mode = subclass;
        batch.blend = BLEND_REPLACE;
        batch.texture_shift = shift;
        batch.set_ink([200, 100, 50, 255]);
        batch.vertex(0.0, 0.0);
        batch.vertex(0.0, 4.0);
        batch.vertex(4.0, 4.0);
        batch.vertex(4.0, 0.0);
        batch.end_quads();
        let mut frame = GuiFrame::new();
        frame.push(GuiLayer::new(batch, LayerTexture::Named(SKIN_TEXTURE)));
        frame
    }

    /// A 4x4 texture whose stored rows (bottom first) are 10, 20, 30 and 40.
    fn striped_textures() -> GuiTextures {
        let mut bytes = Vec::new();
        for row in [10u8, 20, 30, 40] {
            for _ in 0..4 {
                bytes.extend_from_slice(&[row, row, row, 255]);
            }
        }
        let mut textures = GuiTextures::default();
        textures.textures.insert(
            SKIN_TEXTURE.into(),
            GuiTexture::from_bottom_up(4, 4, &bytes).unwrap(),
        );
        textures
    }

    #[test]
    fn a_subclass_that_answers_the_flat_program_draws_the_ink() {
        // The line skin's answer is the flat program: the texture is not
        // sampled, so even a texture the installation lacks changes nothing.
        let frame = skin_frame(1, [0.0; 2]);
        let pixels = rasterise(&[frame], &GuiTextures::default(), 4, 4);
        assert_eq!(&pixels[0..4], &[200, 100, 50, 255]);
        let frame = skin_frame(0, [0.0; 2]);
        let pixels = rasterise(&[frame], &striped_textures(), 4, 4);
        assert_eq!(&pixels[0..4], &[200, 100, 50, 255]);
    }

    #[test]
    fn the_stone_is_sampled_from_the_windows_bottom_one_texel_a_pixel() {
        // Window height 4: the bottom pixel row (y = 3) is stored row 0
        // (value 10), the top row (y = 0) is stored row 3 (value 40).
        let pixels = rasterise(
            &[skin_frame(HUD_STONE_MODE, [0.0; 2])],
            &striped_textures(),
            4,
            4,
        );
        let red = |x: usize, y: usize| pixels[(y * 4 + x) * 4];
        let expected = |value: u32| (200.0 * value as f32 / 255.0).round() as u8;
        assert_eq!(red(0, 3), expected(10));
        assert_eq!(red(0, 2), expected(20));
        assert_eq!(red(0, 1), expected(30));
        assert_eq!(red(0, 0), expected(40));
    }

    #[test]
    fn the_stone_offset_moves_the_sampled_row_in_512ths() {
        // One texel up the stone (1/512 of the texture) moves each pixel one
        // stored row on.
        let shift = [0.0, 1.0 / 512.0];
        let pixels = rasterise(
            &[skin_frame(HUD_STONE_MODE, shift)],
            &striped_textures(),
            4,
            4,
        );
        assert_eq!(pixels[(3 * 4) * 4], (200.0f32 * 20.0 / 255.0).round() as u8);
    }

    #[test]
    fn the_rounded_mode_cuts_the_alpha_at_the_corners() {
        let mut batch = SkinBatch::new();
        batch.mode = SkinMode::Rounded;
        batch.blend = BLEND_REPLACE;
        batch.rounded_scale = Some(crate::skin::rounded_scale(
            crate::skin::GuiRect::sized(0.0, 0.0, 10.0, 10.0),
            2.0,
        ));
        batch.set_ink([10, 20, 30, 255]);
        batch.set_uv(0.0, 0.0);
        batch.vertex(0.0, 0.0);
        batch.set_uv(0.0, 1.0);
        batch.vertex(0.0, 10.0);
        batch.set_uv(1.0, 1.0);
        batch.vertex(10.0, 10.0);
        batch.set_uv(1.0, 0.0);
        batch.vertex(10.0, 0.0);
        batch.end_quads();
        let mut frame = GuiFrame::new();
        frame.push(GuiLayer::new(batch, LayerTexture::None));
        let pixels = rasterise(&[frame], &GuiTextures::default(), 10, 10);
        // The centre keeps the ink; the top-left corner is cut away.
        assert_eq!(
            &pixels[(5 * 10 + 5) * 4..(5 * 10 + 5) * 4 + 4],
            &[10, 20, 30, 255]
        );
        assert!(pixels[3] < 20);
    }

    #[test]
    fn a_layer_is_clipped_to_its_rectangle() {
        let frame = {
            let mut frame = solid_frame(
                vec![
                    vertex(0.0, 0.0, [255, 255, 255, 255]),
                    vertex(0.0, 4.0, [255, 255, 255, 255]),
                    vertex(4.0, 4.0, [255, 255, 255, 255]),
                    vertex(4.0, 0.0, [255, 255, 255, 255]),
                ],
                BLEND_REPLACE,
            );
            frame.layers[0].clip = Some([1, 1, 2, 2]);
            frame
        };
        let pixels = rasterise(&[frame], &GuiTextures::default(), 4, 4);
        let at = |x: usize, y: usize| pixels[(y * 4 + x) * 4 + 3];
        assert_eq!(at(0, 0), 0);
        assert_eq!(at(1, 1), 255);
        assert_eq!(at(2, 2), 255);
        assert_eq!(at(3, 3), 0);
    }

    #[test]
    fn text_quads_sample_the_cells_far_row_at_the_top() {
        let font = &crate::gui_font::RAYFORM_10;
        let quads = crate::gui_text::draw_line(font, b"A", 4, 24);
        let batch = text_batch(font, &quads, [255, 255, 255, 255], (256, 256));
        assert_eq!(batch.primitives.len(), 1);
        let primitive = &batch.primitives[0];
        assert_eq!(primitive.mode, SkinMode::Textured);
        let vertices = batch.vertices_of(primitive);
        assert_eq!((vertices[0].x, vertices[0].y), (4.0, 4.0));
        assert_eq!((vertices[1].x, vertices[1].y), (4.0, 24.0));
        assert_eq!((vertices[2].x, vertices[2].y), (19.0, 24.0));
        assert_eq!(vertices[0].u, vertices[1].u);
        assert!(vertices[0].v > vertices[1].v);
    }

    #[test]
    fn the_plugin_runs_without_a_game_window_or_image_assets() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(GuiDrawPlugin);
        for _ in 0..3 {
            app.update();
        }
        assert!(app.world().resource::<GuiFrames>().frames.is_empty());
    }

    #[test]
    fn the_same_frames_keep_the_canvas_image_instead_of_drawing_again() {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins).add_plugins(GuiDrawPlugin);
        app.init_resource::<Assets<Image>>();
        app.world_mut().spawn((
            Window {
                resolution: bevy::window::WindowResolution::new(8, 8),
                ..default()
            },
            PrimaryWindow,
        ));
        let frame = solid_frame(
            vec![
                vertex(0.0, 0.0, [255, 0, 0, 255]),
                vertex(0.0, 4.0, [255, 0, 0, 255]),
                vertex(4.0, 4.0, [255, 0, 0, 255]),
                vertex(4.0, 0.0, [255, 0, 0, 255]),
            ],
            BLEND_ALPHA,
        );
        app.world_mut()
            .resource_mut::<GuiFrames>()
            .push(frame.clone());
        app.update();
        let drawn = app.world().resource::<GuiCanvas>().drawn;
        assert_eq!(drawn, 1);
        app.world_mut().resource_mut::<GuiFrames>().push(frame);
        app.update();
        assert_eq!(app.world().resource::<GuiCanvas>().drawn, drawn);
        assert!(app.world().resource::<GuiFrames>().frames.is_empty());
        let changed = solid_frame(
            vec![
                vertex(0.0, 0.0, [0, 255, 0, 255]),
                vertex(0.0, 4.0, [0, 255, 0, 255]),
                vertex(4.0, 4.0, [0, 255, 0, 255]),
                vertex(4.0, 0.0, [0, 255, 0, 255]),
            ],
            BLEND_ALPHA,
        );
        app.world_mut().resource_mut::<GuiFrames>().push(changed);
        app.update();
        assert_eq!(app.world().resource::<GuiCanvas>().drawn, drawn + 1);
    }

    #[test]
    fn missing_textures_draw_nothing_instead_of_panicking() {
        let mut batch = SkinBatch::new();
        batch.mode = SkinMode::Textured;
        batch.blend = BLEND_REPLACE;
        batch.set_ink([255, 255, 255, 255]);
        batch.vertex(0.0, 0.0);
        batch.vertex(0.0, 1.0);
        batch.vertex(1.0, 1.0);
        batch.vertex(1.0, 0.0);
        batch.end_quads();
        let mut frame = GuiFrame::new();
        frame.push(GuiLayer::new(batch, LayerTexture::Named("absent")));
        let pixels = rasterise(&[frame], &GuiTextures::default(), 2, 2);
        assert!(pixels.iter().all(|byte| *byte == 0));
    }
}

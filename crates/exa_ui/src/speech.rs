//! Speech bubbles: the frame's measurement, placement and draw.
//!
//! Each line is wrapped at 320 pixels (80 units), its width narrowed to the
//! widest wrapped line and its height the line count times the font's line
//! height less two; it is then placed (a narrator line centred 75 % down the
//! screen, a character line on its raised head) and kept four units inside
//! the screen. A bubble is drawn as a rounded box of radius three, its alpha
//! 128 per 1.0 of the fade (the text 160), and white text wrapped at the
//! bubble's width starting four units below the box's top, less the glyph
//! height and half a unit. The narrator's box is grown by four units on
//! every side. Waiting bubbles are neither placed nor drawn.
//!
//! The bubble list, its fades, the placement rule and the projection are
//! [`exa_script::bubbles`]; this module measures text with the GUI font and
//! turns the result into the GUI draw path's batches.
//!
//! Evidence: the bubble create/frame/draw routines of the conversation unit
//! and the rounded-fill, wrapped-text and text-colour draw calls they make
//! (REA on `Exanima.exe` 0.9.5).

use exa_script::bubbles::{Bubbles, Phase, Speaker};

use crate::form::cvtss2si;
use crate::gui_draw::text_batch;
use crate::gui_font::GuiFont;
use crate::gui_text::{self, TextQuad};
use crate::skin::{GuiRect, SkinBatch, rounded_fill_fixed};

/// The box's corner radius, in units (the original passes 3).
pub const BOX_RADIUS: i32 = 3;
/// The box's alpha at full fade (the original multiplies by 128).
pub const BOX_ALPHA: f32 = 128.0;
/// The text's alpha at full fade.
pub const TEXT_ALPHA: f32 = 160.0;
/// How far the box's top edge is above the first line's top, in units.
pub const TEXT_TOP: i32 = 4;
/// The narrator's box grows this many units on each side.
pub const NARRATOR_MARGIN: i32 = 4;
/// The wrapped text draw's height limit (the original passes 1000).
pub const TEXT_HEIGHT: i32 = 1000;

/// One bubble's draw data for a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BubbleDraw {
    pub speaker: Speaker,
    /// The box: x, y, width, height.
    pub rect: [i32; 4],
    /// The box's alpha byte.
    pub box_alpha: u8,
    /// Where the wrapped text starts (x and the first line's top).
    pub text_at: [i32; 2],
    /// The wrapped text's width.
    pub width: i32,
    pub text_alpha: u8,
    pub text: Vec<u8>,
}

/// The draw data of every showing bubble, oldest first. Waiting bubbles are
/// left out; the caller places them first with [`Bubbles::frame`].
pub fn bubble_draws(bubbles: &Bubbles, unit: i32, glyph_height: i32) -> Vec<BubbleDraw> {
    let mut out = Vec::new();
    for bubble in &bubbles.list {
        if bubble.phase == Phase::Waiting {
            continue;
        }
        let narrator = bubble.speaker == Speaker::Narrator;
        let margin = if narrator { unit } else { 0 };
        let rect = [
            bubble.x - margin,
            bubble.y - margin,
            bubble.width + margin * 2,
            bubble.height + margin * 2,
        ];
        out.push(BubbleDraw {
            speaker: bubble.speaker,
            rect,
            box_alpha: (cvtss2si(bubble.alpha * BOX_ALPHA) & 0xff) as u8,
            text_at: [
                bubble.x,
                bubble
                    .y
                    .wrapping_add(unit.wrapping_mul(TEXT_TOP))
                    .wrapping_sub(glyph_height)
                    .wrapping_sub(unit >> 1),
            ],
            width: bubble.width,
            text_alpha: (cvtss2si(bubble.alpha * TEXT_ALPHA) & 0xff) as u8,
            text: bubble.text.clone(),
        });
    }
    out
}

/// The boxes of a frame as one rounded-fill batch (radius 3; the narrator's
/// box was already grown by its margin).
pub fn box_batch(draws: &[BubbleDraw], unit: i32) -> SkinBatch {
    let mut batch = SkinBatch::new();
    for draw in draws {
        let [x, y, w, h] = draw.rect;
        rounded_fill_fixed(
            &mut batch,
            GuiRect::sized(x as f32, y as f32, w as f32, h as f32),
            unit,
            BOX_RADIUS,
            draw.box_alpha,
        );
    }
    batch
}

/// The glyph quads of one bubble's text: white glyphs wrapped at the
/// bubble's width, hanging from the line top it was given.
pub fn text_quads(font: &GuiFont, draw: &BubbleDraw) -> Vec<TextQuad> {
    gui_text::draw_wrapped(
        font,
        &draw.text,
        draw.text_at[0],
        draw.text_at[1],
        draw.width,
        TEXT_HEIGHT,
        0,
        false,
    )
    .quads
}

/// The draw layers of a frame in draw order: each bubble's box, then its
/// text, bubble by bubble (the original draws them interleaved, so an
/// earlier bubble's text sits under a later bubble's box).
pub enum BubbleLayer {
    Box(SkinBatch),
    Text(SkinBatch),
}

/// Builds the frame's layers in draw order.
pub fn layers(
    draws: &[BubbleDraw],
    unit: i32,
    font: &GuiFont,
    texture: (u16, u16),
) -> Vec<BubbleLayer> {
    let mut out = Vec::new();
    for draw in draws {
        let [x, y, w, h] = draw.rect;
        let mut batch = SkinBatch::new();
        rounded_fill_fixed(
            &mut batch,
            GuiRect::sized(x as f32, y as f32, w as f32, h as f32),
            unit,
            BOX_RADIUS,
            draw.box_alpha,
        );
        out.push(BubbleLayer::Box(batch));
        out.push(BubbleLayer::Text(text_layer(font, draw, texture)));
    }
    out
}

/// One bubble's text as a textured batch in the bubble's alpha.
pub fn text_layer(font: &GuiFont, draw: &BubbleDraw, texture: (u16, u16)) -> SkinBatch {
    let quads = text_quads(font, draw);
    text_batch(font, &quads, [255, 255, 255, draw.text_alpha], texture)
}

/// Measures and places the bubbles for a draw frame at `now`: the width
/// fit and layout use `font`, the placement uses `view` and the head
/// positions.
pub fn layout(
    bubbles: &mut Bubbles,
    font: &GuiFont,
    unit: i32,
    view: &exa_script::bubbles::View,
    now: u32,
    head: impl FnMut(Speaker) -> Option<[f32; 3]>,
) {
    bubbles.layout(font.line_height, |text, width| {
        gui_text::fit_width(font, text, width, unit)
    });
    bubbles.frame(now, Some(view), head);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gui_font::RAYFORM_08 as FONT;
    use exa_script::bubbles::View;

    #[test]
    fn a_line_is_wrapped_at_eighty_units_and_placed() {
        let mut bubbles = Bubbles::default();
        bubbles.say(Speaker::Narrator, None, b"aaa", 0, None);
        // 3 bytes: one line, narrowed to its 21-pixel width; height 16 − 2.
        bubbles.layout(FONT.line_height, |text, width| {
            gui_text::fit_width(&FONT, text, width, 4)
        });
        let b = &bubbles.list[0];
        assert_eq!((b.width, b.lines, b.height), (21, 1, 14));
        let view = View {
            width: 800,
            height: 600,
            glyph_height: FONT.glyph.1,
            camera: None,
        };
        bubbles.frame(300, Some(&view), |_| None);
        let b = &bubbles.list[0];
        // (800 − 21) / 2 = 389; 16 + 450 − 14 = 452.
        assert_eq!((b.x, b.y), (389, 452));
        let draw = bubble_draws(&bubbles, 4, FONT.glyph.1);
        assert_eq!(draw.len(), 1);
        // At full fade the box is 128 and the text 160; the narrator's box
        // is grown by 4 and the text starts 16 − 16 − 2 above the box's y.
        assert_eq!(draw[0].rect, [385, 448, 29, 22]);
        assert_eq!((draw[0].box_alpha, draw[0].text_alpha), (128, 160));
        assert_eq!(draw[0].text_at, [389, 450]);
    }

    #[test]
    fn the_alpha_bytes_round_to_even_and_waiting_lines_are_skipped() {
        let mut bubbles = Bubbles::default();
        // Two lines by the same speaker: the second waits.
        bubbles.say(Speaker::Character(1), None, b"one", 0, None);
        bubbles.say(Speaker::Character(1), None, b"two", 0, None);
        for b in &mut bubbles.list {
            b.width = 20;
            b.height = 14;
            b.lines = 1;
        }
        // Half the fade in: 64 and 80.
        bubbles.list[0].alpha = 0.5;
        let draw = bubble_draws(&bubbles, 4, 16);
        assert_eq!(draw.len(), 1);
        assert_eq!((draw[0].box_alpha, draw[0].text_alpha), (64, 80));
        // 64.5 rounds to even (64), not up; the text's 80.625 rounds up.
        bubbles.list[0].alpha = 64.5 / 128.0;
        let draw = bubble_draws(&bubbles, 4, 16);
        assert_eq!((draw[0].box_alpha, draw[0].text_alpha), (64, 81));
        // 80.5 for the text rounds to even (80).
        bubbles.list[0].alpha = 80.5 / 160.0;
        let draw = bubble_draws(&bubbles, 4, 16);
        assert_eq!(draw[0].text_alpha, 80);
        // A character's box is not grown.
        assert_eq!(draw[0].rect, [0, 0, 20, 14]);
    }

    #[test]
    fn the_box_layer_is_a_rounded_fill_and_the_text_wraps() {
        let mut bubbles = Bubbles::default();
        bubbles.say(Speaker::Character(1), None, b"aaa bbb", 0, None);
        bubbles.layout(FONT.line_height, |text, width| {
            gui_text::fit_width(&FONT, text, width, 4)
        });
        bubbles.frame(
            0,
            Some(&View {
                width: 800,
                height: 600,
                glyph_height: 16,
                camera: None,
            }),
            |_| Some([0.0, 0.0, 0.0]),
        );
        let draws = bubble_draws(&bubbles, 4, 16);
        // No head position and no camera leave the bubble at the origin,
        // which the margin clamp pulls to (4, 4): 49 wide, 14 tall.
        assert_eq!(draws[0].rect, [4, 4, 53, 14]);
        let batch = box_batch(&draws, 4);
        assert!(!batch.vertices.is_empty());
        assert_eq!(batch.primitives.len(), 1);
        // The rounded program grows the rect by radius·unit·4 = 48.
        assert!(batch.rounded_scale.is_some());
        assert_eq!(batch.vertices[0].x, 4.0 - 48.0);
        let quads = text_quads(&FONT, &draws[0]);
        assert_eq!(quads.len(), 7);
        // 7 bytes at 7 pixels each: x 4, 11, 18, ... and one line top.
        assert_eq!(quads[0].rect[0], 4);
        assert_eq!(quads[1].rect[0], 11);
        assert_eq!(draws[0].text_at, [4, 2]);
        assert!(quads.iter().all(|q| q.rect[1] == draws[0].text_at[1]));
        let text = text_layer(&FONT, &draws[0], (256, 256));
        assert_eq!(text.primitives.len(), 1);
        // Two bubbles draw box, text, box, text, never all boxes first.
        let mut more = draws.clone();
        more.push(draws[0].clone());
        let layers = layers(&more, 4, &FONT, (256, 256));
        assert_eq!(layers.len(), 4);
        assert!(matches!(layers[0], BubbleLayer::Box(_)));
        assert!(matches!(layers[1], BubbleLayer::Text(_)));
        assert!(matches!(layers[2], BubbleLayer::Box(_)));
        assert!(matches!(layers[3], BubbleLayer::Text(_)));
    }
}

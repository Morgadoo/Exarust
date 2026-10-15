//! The game's bitmap font (`fontbase24r`): a 16 × 16 grid of Latin-1 glyphs
//! in an A8 texture, one 64-pixel cell per code, each glyph stored upside
//! down in its cell. Glyphs are proportional: a glyph's advance is its inked
//! width plus a fixed gap. Rules: `docs/UI.md` (Fonts).

use exa_formats::rfi::Rfi;

/// The base UI font's texture.
pub const BASE_FONT: &str = "fontbase24r";
/// Cells per row and column.
const GRID: usize = 16;
/// Gap between glyphs, in atlas pixels.
const GAP: usize = 4;
/// The advance of a space, in atlas pixels.
const SPACE: usize = 18;

/// A glyph's inked columns within its cell.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Span {
    left: usize,
    width: usize,
}

/// A decoded font atlas, glyphs turned upright.
#[derive(Clone, Debug)]
pub struct GameFont {
    cell: usize,
    /// Coverage, `GRID · cell` square, row-major, top row first.
    alpha: Vec<u8>,
    spans: Vec<Span>,
}

/// A line of text drawn as coverage.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rendered {
    pub width: usize,
    pub height: usize,
    pub alpha: Vec<u8>,
}

impl GameFont {
    /// Reads the atlas from its texture; `None` when it is not a square A8
    /// grid.
    pub fn from_rfi(rfi: &Rfi) -> Option<Self> {
        let size = rfi.header.width as usize;
        if size == 0 || size != rfi.header.height as usize || !size.is_multiple_of(GRID) {
            return None;
        }
        let rgba = rfi.decode_rgba8().ok()?;
        let alpha: Vec<u8> = rgba.iter().skip(3).step_by(4).copied().collect();
        Self::from_alpha(size, alpha)
    }

    /// Builds the font from a square coverage atlas as stored (glyphs upside
    /// down in their cells).
    pub fn from_alpha(size: usize, mut alpha: Vec<u8>) -> Option<Self> {
        if size == 0 || !size.is_multiple_of(GRID) || alpha.len() != size * size {
            return None;
        }
        let cell = size / GRID;
        for row in 0..GRID {
            let top = row * cell;
            for y in 0..cell / 2 {
                let (a, b) = (top + y, top + cell - 1 - y);
                for x in 0..size {
                    alpha.swap(a * size + x, b * size + x);
                }
            }
        }
        let spans = (0..GRID * GRID)
            .map(|code| {
                let (cx, cy) = ((code % GRID) * cell, (code / GRID) * cell);
                let inked = |x: usize| (0..cell).any(|y| alpha[(cy + y) * size + cx + x] > 0);
                match (
                    (0..cell).find(|&x| inked(x)),
                    (0..cell).rfind(|&x| inked(x)),
                ) {
                    (Some(l), Some(r)) => Span {
                        left: l,
                        width: r + 1 - l,
                    },
                    _ => Span::default(),
                }
            })
            .collect();
        Some(Self { cell, alpha, spans })
    }

    /// The height of a line, in atlas pixels.
    pub fn line_height(&self) -> usize {
        self.cell
    }

    fn code(c: char) -> usize {
        let code = c as u32;
        if code < (GRID * GRID) as u32 {
            code as usize
        } else {
            '?' as usize
        }
    }

    fn advance(&self, code: usize) -> usize {
        match self.spans[code].width {
            0 => SPACE,
            w => w + GAP,
        }
    }

    /// Draws `text` on one line, at atlas resolution.
    pub fn render(&self, text: &str) -> Rendered {
        let width = text
            .chars()
            .map(|c| self.advance(Self::code(c)))
            .sum::<usize>()
            .max(1);
        let height = self.cell;
        let size = self.cell * GRID;
        let mut alpha = vec![0u8; width * height];
        let mut pen = 0;
        for c in text.chars() {
            let code = Self::code(c);
            let span = self.spans[code];
            let (cx, cy) = (
                (code % GRID) * self.cell + span.left,
                (code / GRID) * self.cell,
            );
            for y in 0..height {
                let src = (cy + y) * size + cx;
                let dst = y * width + pen;
                alpha[dst..dst + span.width].copy_from_slice(&self.alpha[src..src + span.width]);
            }
            pen += self.advance(code);
        }
        Rendered {
            width,
            height,
            alpha,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 32-pixel atlas (2-pixel cells) with '1' inked in its left column,
    /// on the cell's stored (upside-down) bottom row.
    fn atlas() -> GameFont {
        let size = 32;
        let mut alpha = vec![0u8; size * size];
        let code = '1' as usize;
        let (cx, cy) = ((code % 16) * 2, (code / 16) * 2);
        alpha[(cy + 1) * size + cx] = 200;
        GameFont::from_alpha(size, alpha).unwrap()
    }

    #[test]
    fn glyphs_are_turned_upright_and_measured() {
        let font = atlas();
        let line = font.render("1");
        assert_eq!((line.width, line.height), (1 + GAP, 2));
        assert_eq!(line.alpha[0], 200);
        assert_eq!(line.alpha[line.width], 0);
    }

    #[test]
    fn blank_and_unknown_characters_advance() {
        let font = atlas();
        assert_eq!(font.render(" ").width, SPACE);
        assert_eq!(font.render("€").width, SPACE);
    }

    #[test]
    fn rejects_odd_atlases() {
        assert!(GameFont::from_alpha(30, vec![0; 900]).is_none());
        assert!(GameFont::from_alpha(32, vec![0; 10]).is_none());
    }
}

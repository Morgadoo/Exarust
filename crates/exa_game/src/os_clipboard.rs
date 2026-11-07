//! The OS layer's clipboard text, as the original's text controls use it.
//!
//! The original's put empties the clipboard and offers the control's bytes
//! twice: as they are (the ANSI text format, with a terminating zero) and
//! converted from the ANSI code page to UTF-16 (the Unicode format). It
//! reports success when either buffer was allocated. Its read takes the
//! ANSI text format only (Windows converts other applications' Unicode
//! text to it) and copies it up to the first zero byte; no text there
//! means nothing to paste. Copy, cut and paste are asked for by the text
//! box and the single-line edit (`exa_ui::text_box`, `exa_ui::text_edit`).
//!
//! The ANSI code page is Windows-1252 here (the Western default the game
//! ships for). The system clipboard holds UTF-8: a put decodes the bytes
//! from Windows-1252, and a read encodes each character back, a character
//! the code page lacks becoming `?` (Windows' default character; its
//! best-fit substitutions, such as `ā` to `a`, are not reproduced). The
//! clipboard is opened on first use, so a run that never copies or pastes
//! never connects to it. Rules: `docs/UI.md` (Text boxes).

use std::sync::Mutex;

use bevy::prelude::*;

/// The characters Windows-1252 gives the bytes 0x80–0x9F. The five bytes
/// the code page leaves undefined (0x81, 0x8D, 0x8F, 0x90, 0x9D) convert
/// to the control characters of the same number, as Windows converts them.
const CP1252_HIGH: [char; 32] = [
    '\u{20AC}', '\u{0081}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{008D}', '\u{017D}', '\u{008F}',
    '\u{0090}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{009D}', '\u{017E}', '\u{0178}',
];

/// Windows' default character for one the code page cannot hold.
pub const DEFAULT_CHAR: u8 = b'?';

/// The character a Windows-1252 byte stands for.
pub fn ansi_char(byte: u8) -> char {
    match byte {
        0x80..=0x9f => CP1252_HIGH[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

/// A control's bytes as the clipboard's Unicode text: every byte through
/// the code page, a zero byte included (the original converts the whole
/// length it was given).
pub fn ansi_to_text(bytes: &[u8]) -> String {
    bytes.iter().copied().map(ansi_char).collect()
}

/// The Windows-1252 byte for a character, when the code page holds it.
pub fn char_ansi(c: char) -> Option<u8> {
    let code = u32::from(c);
    if code < 0x80 || (0xa0..=0xff).contains(&code) {
        return Some(code as u8);
    }
    CP1252_HIGH
        .iter()
        .position(|&high| high == c)
        .map(|i| 0x80 + i as u8)
}

/// The clipboard's text as the ANSI bytes the original's read returns: up
/// to the first zero, each character through the code page, `?` for one it
/// lacks.
pub fn text_to_ansi(text: &str) -> Vec<u8> {
    text.chars()
        .take_while(|&c| c != '\0')
        .map(|c| char_ansi(c).unwrap_or(DEFAULT_CHAR))
        .collect()
}

/// The system clipboard, opened on first use.
#[derive(Resource, Default)]
pub struct SystemClipboard {
    board: Mutex<Option<arboard::Clipboard>>,
    /// The clipboard could not be opened (no display server): every put
    /// fails and every read is empty, as the original's do when it cannot
    /// open the clipboard.
    unavailable: bool,
}

impl SystemClipboard {
    fn with<T>(&mut self, f: impl FnOnce(&mut arboard::Clipboard) -> Option<T>) -> Option<T> {
        if self.unavailable {
            return None;
        }
        let board = self.board.get_mut().unwrap_or_else(|e| e.into_inner());
        if board.is_none() {
            match arboard::Clipboard::new() {
                Ok(opened) => *board = Some(opened),
                Err(error) => {
                    warn!("clipboard: cannot be opened: {error}");
                    self.unavailable = true;
                    return None;
                }
            }
        }
        board.as_mut().and_then(f)
    }

    /// Puts a control's bytes on the clipboard; false when it could not.
    pub fn put(&mut self, bytes: &[u8]) -> bool {
        let text = ansi_to_text(bytes);
        self.with(|board| board.set_text(text).ok()).is_some()
    }

    /// The clipboard's text as ANSI bytes; `None` when it holds no text.
    pub fn read(&mut self) -> Option<Vec<u8>> {
        self.with(|board| board.get_text().ok())
            .map(|text| text_to_ansi(&text))
    }
}

impl exa_ui::text_box::Clipboard for SystemClipboard {
    fn set(&mut self, text: &[u8]) {
        self.put(text);
    }

    fn get(&mut self) -> Option<Vec<u8>> {
        self.read()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_control_bytes_go_out_through_windows_1252() {
        assert_eq!(ansi_to_text(b"Ab 1"), "Ab 1");
        assert_eq!(
            ansi_to_text(&[0x80, 0x93, 0x94, 0xe9]),
            "\u{20AC}\u{201C}\u{201D}é"
        );
        // The undefined bytes become the control characters.
        assert_eq!(ansi_to_text(&[0x81, 0x9d]), "\u{0081}\u{009D}");
        // Every byte round-trips.
        let all: Vec<u8> = (1..=255).collect();
        assert_eq!(text_to_ansi(&ansi_to_text(&all)), all);
    }

    #[test]
    fn the_read_stops_at_a_zero_and_marks_what_the_code_page_lacks() {
        assert_eq!(
            text_to_ansi("caf\u{e9} \u{20AC}5"),
            b"caf\xe9 \x805".to_vec()
        );
        assert_eq!(text_to_ansi("ok\0rest"), b"ok".to_vec());
        assert_eq!(text_to_ansi("\u{0101}\u{4e2d}"), b"??".to_vec());
        assert_eq!(text_to_ansi(""), Vec::<u8>::new());
    }
}

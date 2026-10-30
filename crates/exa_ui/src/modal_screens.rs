//! The modal screens: the message box, the shader warm-up screen and the
//! key-capture screen.
//!
//! A message box confirms on Enter or Space (and on a click of its first
//! button) and cancels on Escape or either other button, with result −1.
//! The shader warm-up screen fades its text in over 200 clock units, steps
//! the renderer's 29-row program warm-up one row per frame, and fades out
//! once the warm-up answers that it has no row left. The key-capture screen fades in the same way, warns "ALREADY IN
//! USE" for 1000 units when a key is bound elsewhere, and closes on a
//! captured key or Escape.
//!
//! Both screens paint their text in the main font, centred, in the
//! palette's text colour under the fade.
//!
//! Evidence: the message box's key and click handlers, the warm-up screen's
//! per-frame task and paint, and the key screen's register, key, mouse and
//! per-frame routines (`docs/UI.md`).

use crate::gui_font::GuiFont;
use crate::gui_text::{TextQuad, draw_line, measure};

/// The key codes the message box and key screen use.
pub const KEY_ESCAPE: u32 = 0x1b;
pub const KEY_ENTER: u32 = 0x0d;
pub const KEY_SPACE: u32 = 0x20;
/// An event's release bit.
pub const RELEASE: u16 = 2;

/// A message box's state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MessageBox {
    /// The result the box closes with; −1 is a cancel.
    pub result: i32,
    pub closed: bool,
}

impl MessageBox {
    /// What the key handler did: closed or not.
    pub fn key(&mut self, code: u32, flags: u16) -> bool {
        if flags & RELEASE != 0 {
            return false;
        }
        if code < KEY_ENTER {
            return false;
        }
        if code == KEY_ENTER || code == KEY_SPACE {
            self.closed = true;
            return true;
        }
        if code == KEY_ESCAPE {
            self.result = -1;
            self.closed = true;
            return true;
        }
        false
    }

    /// What a click on a button with mask `mask` did. `0x100` is the
    /// confirming button; `0x600` cancels; any other mask does nothing.
    pub fn click(&mut self, mask: u32) -> bool {
        if mask & 0x100 != 0 {
            self.closed = true;
            return true;
        }
        if mask & 0x600 != 0 {
            self.result = -1;
            self.closed = true;
            return true;
        }
        false
    }
}

/// The clock units from `start` to `now`, signed so that a clock behind its
/// start gives a negative span (the original subtracts in 64 bits and
/// converts the result as a signed number).
fn elapsed(now: u32, start: u32) -> f64 {
    (i64::from(now) - i64::from(start)) as f64
}

/// The fade's byte: `t · 128` in single precision, converted with
/// `CVTSS2SI` and kept in a byte (so a negative level wraps).
fn fade_byte(t: f32) -> u8 {
    crate::form::cvtss2si64(t * SCREEN_ALPHA) as u8
}

/// The centring both screens use: the span less the text, worked in 64
/// bits and shifted right by one, its low 32 bits kept.
fn centre(span: i32, text: i32) -> i32 {
    (((i64::from(span) - i64::from(text)) as u64) >> 1) as i32
}

/// The two fade lengths the small screens use, clock units.
pub const SCREEN_FADE_MS: u32 = 200;
/// The multiplier that turns a fade level into the byte the screens store.
pub const SCREEN_ALPHA: f32 = 128.0;
/// How long the key screen's warning stays before the prompt returns.
pub const IN_USE_MS: u32 = 1000;
/// The draw-skin state both screens' paints select before their text.
pub const SCREEN_DRAW_STATE: u32 = 2;
/// The blend word both screens' paints set (source alpha over one minus
/// source alpha).
pub const SCREEN_BLEND: u32 = 0x203;
/// The rows of the renderer's warm-up table: the warm-up routine compiles
/// row `index` and answers true while `index` is below this; at this index
/// it runs its finished hook and answers false.
pub const WARM_ROWS: i32 = 29;

/// The warm-up routine's answer for `index` (see [`WARM_ROWS`]): one row a
/// call, true while there was a row to compile.
pub fn warm_row(index: i32) -> bool {
    index < WARM_ROWS
}

/// The warm-up screen's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheScreenForm {
    pub flags: u32,
    pub form_flags: u32,
    pub digit: i32,
    /// The form's colour word (opaque black); the fade writes its alpha
    /// byte.
    pub colour: u32,
}

pub fn cache_screen_form() -> CacheScreenForm {
    CacheScreenForm {
        flags: 0x1020,
        form_flags: 0xf0008,
        digit: 0,
        colour: 0xff00_0000,
    }
}

/// The shader warm-up screen's phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WarmScreenPhase {
    /// Fading in.
    #[default]
    FadingIn,
    /// Stepping the program cache.
    Warming,
    /// Fading out.
    FadingOut,
}

/// What one frame of a small screen asks of its host.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ScreenStep {
    /// The form closes (the warm-up screen at the end of its fade-out).
    pub close: bool,
    /// The form takes the focus again (while it fades in).
    pub focus: bool,
    /// The GUI task is asked to repaint (message `0x23`).
    pub repaint: bool,
}

/// The shader warm-up screen's state (`TCacheShaderScreen`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheShaderScreen {
    pub phase: WarmScreenPhase,
    /// The clock the phase started at.
    pub start: u32,
    /// The warm-up row the next frame compiles.
    pub next: i32,
    /// The fade's level, 0…128: the alpha byte of the form's colour.
    pub alpha: u8,
}

impl CacheShaderScreen {
    /// Opens the screen at `now`: the constructor stamps the clock and
    /// clears the fade byte; the opener shows the form, focuses it and
    /// adds [`Self::step`] to the per-frame list.
    pub fn new(now: u32) -> Self {
        Self {
            phase: WarmScreenPhase::FadingIn,
            start: now,
            next: 0,
            alpha: 0,
        }
    }

    /// One frame (the per-frame task, which always stays registered).
    ///
    /// Fading in, the level is the elapsed time over 200 in double,
    /// narrowed; at 1 or more the phase moves on. Each frame of the fade
    /// repaints and refocuses. Warming, `warm(index)` runs the renderer's
    /// warm-up for row `index` ([`warm_row`] gives its answer); a false
    /// answer starts the fade-out at `now`, and the row counter rises
    /// either way. Fading out, the level is 1 less the elapsed fraction and
    /// the form closes once it falls under 0.001.
    pub fn step(&mut self, now: u32, mut warm: impl FnMut(i32) -> bool) -> ScreenStep {
        match self.phase {
            WarmScreenPhase::FadingIn => {
                let mut t = (elapsed(now, self.start) / 200.0) as f32;
                if t >= 1.0 {
                    self.phase = WarmScreenPhase::Warming;
                    t = 1.0;
                }
                self.alpha = fade_byte(t);
                ScreenStep {
                    close: false,
                    focus: true,
                    repaint: true,
                }
            }
            WarmScreenPhase::Warming => {
                if !warm(self.next) {
                    self.phase = WarmScreenPhase::FadingOut;
                    self.start = now;
                }
                self.next = self.next.wrapping_add(1);
                ScreenStep::default()
            }
            WarmScreenPhase::FadingOut => {
                let mut t = (1.0 - elapsed(now, self.start) / 200.0) as f32;
                let mut close = false;
                // The narrowed level is compared in double.
                if f64::from(t) < 0.001 {
                    close = true;
                    t = 0.0;
                }
                self.alpha = fade_byte(t);
                ScreenStep {
                    close,
                    focus: false,
                    repaint: true,
                }
            }
        }
    }

    /// The screen does not take a hit.
    pub fn hit(&self) -> i32 {
        -1
    }

    /// The destroy takes the per-frame step out of the list and runs the
    /// base destroy: both always happen.
    pub fn destroy(&self) -> (bool, bool) {
        (true, true)
    }

    /// The text's origin and the colour's alpha. The origin centres the
    /// text across and the font's line down (64-bit differences shifted
    /// right, low halves kept); the alpha is the fade byte times the
    /// palette colour's alpha over 128.
    pub fn draw(
        &self,
        width: i32,
        height: i32,
        line_height: i32,
        text_width: i32,
        palette_alpha: u8,
    ) -> ([i32; 2], u8) {
        (
            [centre(width, text_width), centre(height, line_height)],
            screen_colour_alpha(self.alpha, palette_alpha),
        )
    }

    /// The paint's text in Exarust's draw path: [`WARMING_TEXT`] in the
    /// main font at [`Self::draw`]'s origin, in the palette's text colour
    /// with the faded alpha.
    pub fn paint(&self, font: &GuiFont, size: (i32, i32), palette_colour: u32) -> ScreenText {
        screen_text(font, size, WARMING_TEXT, self.alpha, palette_colour)
    }

    /// The message's string slot is released.
    pub fn release_text(&self) -> bool {
        true
    }
}

/// The shader warm-up text.
pub const WARMING_TEXT: &str = "Applying new settings...";
/// The key screen's prompts.
pub const PRESS_A_KEY: &str = "PRESS A KEY";
pub const ALREADY_IN_USE: &str = "ALREADY IN USE";

/// The alpha a small screen's colour takes: the form's alpha byte times
/// the palette colour's, over 128 in double, converted with `CVTSD2SI`
/// and stored in a byte (so a product above 32767 wraps).
pub fn screen_colour_alpha(colour_alpha: u8, palette_alpha: u8) -> u8 {
    let product = u64::from(colour_alpha) * u64::from(palette_alpha);
    crate::form::cvtsd2si(product as f64 * 0.0078125) as u8
}

/// A small screen's painted text: the glyph quads and the colour word they
/// draw in (the palette's red, green and blue under the faded alpha, alpha
/// in the top byte), after [`SCREEN_DRAW_STATE`] and [`SCREEN_BLEND`].
#[derive(Debug, Clone, PartialEq)]
pub struct ScreenText {
    pub quads: Vec<TextQuad>,
    pub colour: u32,
}

/// Lays `text` out as both screens' paints do: measured with the main
/// font, centred across the form and down by the font's line height.
fn screen_text(
    font: &GuiFont,
    size: (i32, i32),
    text: &str,
    alpha: u8,
    palette_colour: u32,
) -> ScreenText {
    let bytes = text.as_bytes();
    let x = centre(size.0, measure(font, bytes));
    let y = centre(size.1, font.line_height);
    let alpha = screen_colour_alpha(alpha, (palette_colour >> 24) as u8);
    ScreenText {
        quads: draw_line(font, bytes, x, y),
        colour: (palette_colour & 0x00ff_ffff) | (u32::from(alpha) << 24),
    }
}

/// How many binding entries the key screen compares a new key against.
pub const BINDING_SLOTS: usize = 23;

/// The key screen's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadKeyForm {
    pub flags: u32,
    pub form_flags: u32,
    pub digit: i32,
    /// The form's colour word (opaque black); the fade writes its alpha
    /// byte.
    pub colour: u32,
    /// The form's left edge (0: the constructor centres it in the window).
    pub left: i32,
    /// The fade byte the constructor starts the form at (nothing shown).
    pub fade: u8,
}

pub fn read_key_form() -> ReadKeyForm {
    ReadKeyForm {
        flags: 0x1020,
        form_flags: 0xf0008,
        digit: 0,
        colour: 0xff00_0000,
        left: 0,
        fade: 0,
    }
}

/// The key screen's phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ReadKeyPhase {
    #[default]
    FadingIn,
    Ready,
    /// A conflicting key was seen: the warning shows for 1000 units.
    InUse,
}

/// What the key screen did with an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyAnswer {
    /// Nothing happened (a release, a key without a key-state word, an
    /// unmapped mouse button).
    Ignored,
    /// The code was stored in the action's entry and the form closes; the
    /// controls form's control with the action's id is relabelled with the
    /// key's name.
    Captured(u32),
    /// The code is bound to another action: the warning is raised and the
    /// form stays open.
    InUse,
    /// Escape: the form closes and nothing is stored.
    Closed,
}

/// The key-capture screen (`TReadKeyScreen`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadKeyScreen {
    /// The action being rebound: the clicked key cell's control id, one
    /// more than its entry in the page's table.
    pub action: u32,
    pub phase: ReadKeyPhase,
    /// The clock the fade-in started at.
    pub start: u32,
    /// The clock the "already in use" warning was raised at.
    pub warned_at: u32,
    pub alpha: u8,
}

impl ReadKeyScreen {
    /// The opener: the screen for the key cell `action`, its fade stamped
    /// at `now`; the opener also adds [`Self::step`] to the per-frame list
    /// and installs [`Self::mouse`] as the window's mouse hook.
    pub fn new(action: u32, now: u32) -> Self {
        Self {
            action,
            phase: ReadKeyPhase::FadingIn,
            start: now,
            warned_at: 0,
            alpha: 0,
        }
    }

    /// The prompt the draw shows.
    pub fn prompt(&self) -> &'static str {
        if self.phase == ReadKeyPhase::InUse {
            ALREADY_IN_USE
        } else {
            PRESS_A_KEY
        }
    }

    /// The key-capture registration: a key already in any of the first 23
    /// entries other than entry `action − 1` raises the warning at `now`
    /// and fails; otherwise the key is stored in entry `action − 1` and the
    /// capture succeeds (the original writes the word before the table for
    /// action 0; Exarust writes nothing there). Actions count from one.
    pub fn register(&mut self, bindings: &mut [u32], action: u32, key: u32, now: u32) -> bool {
        let own = i64::from(action as i32) - 1;
        let found = bindings
            .iter()
            .take(BINDING_SLOTS)
            .enumerate()
            .any(|(i, bound)| own != i as i64 && *bound == key);
        if found {
            self.phase = ReadKeyPhase::InUse;
            self.warned_at = now;
            return false;
        }
        if let Some(slot) = usize::try_from(own).ok().and_then(|i| bindings.get_mut(i)) {
            *slot = key;
        }
        true
    }

    fn capture(&mut self, bindings: &mut [u32], key: u32, now: u32) -> KeyAnswer {
        if self.register(bindings, self.action, key, now) {
            KeyAnswer::Captured(key)
        } else {
            KeyAnswer::InUse
        }
    }

    /// The mouse hook: buttons 4, 8 and 16 (the middle and the two extra
    /// buttons) map to the codes `0x8004`, `0x8005` and `0x8006`; every
    /// other button, the primary and secondary among them, is ignored.
    pub fn mouse(&mut self, bindings: &mut [u32], button: u32, now: u32) -> KeyAnswer {
        let key = match button {
            0..=3 => return KeyAnswer::Ignored,
            4 => 0x8004,
            8 => 0x8005,
            0x10 => 0x8006,
            _ => return KeyAnswer::Ignored,
        };
        self.capture(bindings, key, now)
    }

    /// A keyboard or pad event. A release does nothing; Escape closes the
    /// form; otherwise `scan`, the window's last key-state word, gives the
    /// code `(scan >> 16) & 0xfff` (the scan code and its extended bit),
    /// and a bare extended bit (`0x100`) becomes `(code & 0x3fff) | 0x4000`.
    /// A zero word does nothing.
    pub fn key(
        &mut self,
        bindings: &mut [u32],
        code: u32,
        flags: u16,
        scan: u32,
        now: u32,
    ) -> KeyAnswer {
        if flags & RELEASE != 0 {
            return KeyAnswer::Ignored;
        }
        if code == KEY_ESCAPE {
            return KeyAnswer::Closed;
        }
        if scan == 0 {
            return KeyAnswer::Ignored;
        }
        let mut key = (scan >> 16) & 0xfff;
        if key == 0x100 {
            key = (code & 0x3fff) | 0x4000;
        }
        self.capture(bindings, key, now)
    }

    /// One frame's fade and warning timer (the per-frame task, which stays
    /// registered). Under a whole fade the form refocuses; at the end the
    /// first phase becomes ready. A warning older than 1000 units clears
    /// (the sum in 64 bits, a strict compare). Every frame repaints.
    pub fn step(&mut self, now: u32) -> ScreenStep {
        let mut t = (elapsed(now, self.start) / 200.0) as f32;
        let mut focus = false;
        if t >= 1.0 {
            t = 1.0;
            if self.phase == ReadKeyPhase::FadingIn {
                self.phase = ReadKeyPhase::Ready;
            }
        } else {
            focus = true;
        }
        if self.phase == ReadKeyPhase::InUse
            && u64::from(self.warned_at) + u64::from(IN_USE_MS) < u64::from(now)
        {
            self.phase = ReadKeyPhase::Ready;
        }
        self.alpha = fade_byte(t);
        ScreenStep {
            close: false,
            focus,
            repaint: true,
        }
    }

    /// The draw: the prompt centred across and down by the font's line,
    /// with the fade byte times the palette alpha over 128.
    pub fn draw(
        &self,
        width: i32,
        height: i32,
        line_height: i32,
        text_width: i32,
        palette_alpha: u8,
    ) -> ([i32; 2], &'static str, u8) {
        (
            [centre(width, text_width), centre(height, line_height)],
            self.prompt(),
            screen_colour_alpha(self.alpha, palette_alpha),
        )
    }

    /// The paint's text in Exarust's draw path (see
    /// [`CacheShaderScreen::paint`]), with the current prompt.
    pub fn paint(&self, font: &GuiFont, size: (i32, i32), palette_colour: u32) -> ScreenText {
        screen_text(font, size, self.prompt(), self.alpha, palette_colour)
    }

    /// The hit test misses.
    pub fn hit(&self) -> i32 {
        -1
    }

    /// The destroy takes the per-frame step out of the list, clears the
    /// window's mouse hook and runs the base destroy.
    pub fn destroy(&self) -> (bool, bool) {
        (true, true)
    }

    /// The prompt's string slot is released.
    pub fn release(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_message_box_keys_and_clicks() {
        let mut m = MessageBox::default();
        assert!(!m.key(KEY_ENTER, RELEASE));
        assert!(!m.key(u32::from(b'a'), 0));
        assert!(m.key(KEY_ENTER, 0));
        assert!(m.closed && m.result == 0);
        let mut m = MessageBox::default();
        assert!(m.key(KEY_ESCAPE, 0));
        assert_eq!(m.result, -1);
        let mut m = MessageBox::default();
        assert!(m.key(KEY_SPACE, 0));
        assert_eq!(m.result, 0);
        let mut m = MessageBox::default();
        assert!(m.click(0x600));
        assert_eq!(m.result, -1);
        let mut m = MessageBox::default();
        assert!(m.click(0x100));
        assert_eq!(m.result, 0);
        assert!(m.click(0x400));
        assert_eq!(m.result, -1);
    }

    #[test]
    fn the_warm_screen_fades_in_steps_twenty_nine_rows_and_fades_out() {
        let mut s = CacheShaderScreen::new(1000);
        let step = s.step(1100, warm_row);
        assert!(step.focus && step.repaint && !step.close);
        assert_eq!(s.alpha, 64);
        s.step(1200, warm_row);
        assert_eq!((s.phase, s.alpha), (WarmScreenPhase::Warming, 128));
        // One row a frame: rows 0 to 28 compile, the 30th call (row 29)
        // answers false and starts the fade-out at its clock.
        let mut frames = 0;
        while s.phase == WarmScreenPhase::Warming {
            let step = s.step(1300 + frames, warm_row);
            assert_eq!(step, ScreenStep::default());
            frames += 1;
        }
        assert_eq!(frames, 30);
        assert_eq!((s.next, s.start), (30, 1329));
        assert!(!s.step(1429, warm_row).close);
        assert_eq!(s.alpha, 64);
        // 1 − 199/200 = 0.005 is not under 0.001; 1 − 200/200 is.
        assert!(!s.step(1528, warm_row).close);
        assert_eq!(s.alpha, 1);
        let last = s.step(1529, warm_row);
        assert!(last.close && last.repaint);
        assert_eq!(s.alpha, 0);
        assert!(warm_row(28) && !warm_row(29));
    }

    #[test]
    fn the_key_screen_warns_about_an_existing_binding() {
        let mut screen = ReadKeyScreen::new(3, 0);
        assert_eq!(screen.warned_at, 0);
        let mut binds = [0x11u32, 0x12, 0x13, 0];
        assert!(screen.register(&mut binds, 3, 0x14, 0));
        // Actions count from one: action 3 owns entry 2.
        assert_eq!(binds, [0x11, 0x12, 0x14, 0]);
        assert!(!screen.register(&mut binds, 3, 0x11, 500));
        // The warning has its own clock; the fade clock is untouched.
        assert_eq!(
            (screen.phase, screen.warned_at, screen.start),
            (ReadKeyPhase::InUse, 500, 0)
        );
        assert_eq!(screen.prompt(), ALREADY_IN_USE);
        assert_eq!(
            screen.mouse(&mut binds, 8, 600),
            KeyAnswer::Captured(0x8005)
        );
        // The primary, secondary and unknown buttons are ignored.
        for button in [1, 2, 3, 0x20] {
            assert_eq!(screen.mouse(&mut binds, button, 600), KeyAnswer::Ignored);
        }
        assert_eq!(
            screen.key(&mut binds, 0x41, 0, (0x100 << 16) | 7, 700),
            KeyAnswer::Captured(0x4041)
        );
        // A bound key warns; release and a zero word capture nothing;
        // Escape closes the form.
        assert_eq!(
            screen.key(&mut binds, 0x41, 0, 0x11 << 16, 700),
            KeyAnswer::InUse
        );
        assert_eq!(
            screen.key(&mut binds, 0x41, RELEASE, 7 << 16, 700),
            KeyAnswer::Ignored
        );
        assert_eq!(screen.key(&mut binds, 0x41, 0, 0, 700), KeyAnswer::Ignored);
        assert_eq!(
            screen.key(&mut binds, KEY_ESCAPE, 0, 7 << 16, 700),
            KeyAnswer::Closed
        );
        // The scan code keeps its extended bit: 0x11d for a right control.
        let mut table = [0u32; 23];
        assert_eq!(
            screen.key(&mut table, 0x11, 0, 0x011d_0001, 0),
            KeyAnswer::Captured(0x11d)
        );
        assert_eq!(table[2], 0x11d);
    }

    #[test]
    fn the_key_screen_compares_against_twenty_three_entries() {
        let mut screen = ReadKeyScreen::new(1, 0);
        let mut binds = [0u32; 24];
        binds[22] = 0x55;
        assert!(!screen.register(&mut binds, 1, 0x55, 0));
        binds[22] = 0;
        binds[23] = 0x55;
        assert!(screen.register(&mut binds, 1, 0x55, 0));
        assert_eq!(binds[0], 0x55);
        // Action 0 compares against every entry and stores nothing.
        let mut binds = [7u32; 23];
        assert!(screen.register(&mut binds, 0, 9, 0));
        assert_eq!(binds, [7; 23]);
    }

    #[test]
    fn a_clock_behind_the_start_wraps_the_fade_byte() {
        let mut s = CacheShaderScreen::new(1000);
        s.step(900, warm_row);
        // Minus half of 128, stored in a byte.
        assert_eq!(s.alpha, 192);
    }

    #[test]
    fn the_key_screen_fades_and_clears_the_warning() {
        let mut s = ReadKeyScreen::new(1, 0);
        assert!(s.step(100).focus);
        assert_eq!((s.alpha, s.phase), (64, ReadKeyPhase::FadingIn));
        let step = s.step(200);
        assert!(!step.focus && step.repaint && !step.close);
        assert_eq!((s.alpha, s.phase), (128, ReadKeyPhase::Ready));
        s.phase = ReadKeyPhase::InUse;
        s.warned_at = 1000;
        s.step(1100);
        assert_eq!(s.alpha, 128);
        assert_eq!(s.phase, ReadKeyPhase::InUse);
        // 1000 + 1000 < 2000 is false: the warning stays one more unit.
        s.step(2000);
        assert_eq!(s.phase, ReadKeyPhase::InUse);
        s.step(2001);
        assert_eq!(s.phase, ReadKeyPhase::Ready);
    }

    #[test]
    fn the_forms_keep_their_constructor_values() {
        let c = cache_screen_form();
        assert_eq!((c.flags, c.form_flags, c.digit), (0x1020, 0xf0008, 0));
        assert_eq!(c.colour, 0xff000000);
        let mut warm = CacheShaderScreen::new(0);
        warm.step(200, warm_row);
        assert_eq!(warm.hit(), -1);
        assert_eq!(warm.destroy(), (true, true));
        assert!(warm.release_text());
        // The alpha is the fade byte (128) times the palette's over 128.
        assert_eq!(warm.draw(400, 200, 20, 100, 255), ([150, 90], 255));
        assert_eq!(warm.draw(400, 200, 20, 100, 64).1, 64);
        // An odd negative margin rounds down, not toward zero.
        assert_eq!(warm.draw(99, 20, 20, 100, 255).0, [-1, 0]);
        let r = read_key_form();
        assert_eq!((r.flags, r.colour), (0x1020, 0xff000000));
        assert_eq!((r.form_flags, r.left, r.fade), (0xf0008, 0, 0));
        let mut screen = ReadKeyScreen::new(2, 0);
        assert_eq!(screen.hit(), -1);
        assert!(screen.release());
        // Before the first frame the fade byte is 0: nothing shows.
        assert_eq!(
            screen.draw(400, 200, 20, 80, 255),
            ([160, 90], PRESS_A_KEY, 0)
        );
        screen.step(100);
        assert_eq!(screen.draw(400, 200, 20, 80, 255).2, 128);
    }

    #[test]
    fn the_screen_colour_alpha_rounds_half_to_even() {
        assert_eq!(screen_colour_alpha(128, 255), 255);
        assert_eq!(screen_colour_alpha(64, 255), 128);
        // 1 · 64 / 128 = 0.5 rounds to 0; 3 · 64 / 128 = 1.5 to 2.
        assert_eq!(screen_colour_alpha(1, 64), 0);
        assert_eq!(screen_colour_alpha(3, 64), 2);
        // A product past 32767 wraps in the byte.
        assert_eq!(screen_colour_alpha(255, 255), 252);
        assert_eq!(screen_colour_alpha(0, 200), 0);
    }

    #[test]
    fn the_paints_centre_their_text_in_the_palette_colour() {
        let font = &crate::gui_font::FONTBASE_24;
        let mut s = CacheShaderScreen::new(0);
        s.step(100, warm_row);
        let text = s.paint(font, (800, 600), 0x80c0_ffee);
        let width = measure(font, WARMING_TEXT.as_bytes());
        assert_eq!(text.quads.len(), WARMING_TEXT.len());
        assert_eq!(text.quads[0].pen, (800 - width) >> 1);
        // Fade 64 times palette alpha 0x80 over 128 is 64.
        assert_eq!(text.colour, 0x40c0_ffee);
        let key = ReadKeyScreen::new(1, 0);
        assert_eq!(key.paint(font, (800, 600), 0xff00_0000).quads.len(), 11);
    }
}

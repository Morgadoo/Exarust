//! The modal screens: the message box, the shader warm-up screen and the
//! key-capture screen.
//!
//! A message box confirms on Enter or Space (and on a click of its first
//! button) and cancels on Escape or either other button, with result −1.
//! The shader warm-up screen fades its text in over 200 clock units, steps
//! the program cache one entry per frame, and fades out when the cache is
//! warm. The key-capture screen fades in the same way, warns "ALREADY IN
//! USE" for 1000 units when a key is bound elsewhere, and closes on a
//! captured key or Escape.
//!
//! Evidence: the message box's key and click handlers, the warm-up screen's
//! per-frame task and paint, and the key screen's register, key, mouse and
//! per-frame routines (`docs/UI.md`).

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
    /// confirming button; `0x600` cancels. The event is always consumed.
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

/// The two fade lengths the small screens use, clock units.
pub const SCREEN_FADE_MS: u32 = 200;
/// The multiplier that turns a fade level into the byte the screens store.
pub const SCREEN_ALPHA: f32 = 128.0;
/// How long the key screen's warning stays before the prompt returns.
pub const IN_USE_MS: u32 = 1000;

/// The warm-up screen's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheScreenForm {
    pub flags: u32,
    pub form_flags: u32,
    pub digit: i32,
    /// The colour the screen paints behind its text (opaque black).
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

/// The shader warm-up screen's state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CacheShaderScreen {
    pub phase: WarmScreenPhase,
    /// The clock the phase started at.
    pub start: u32,
    /// The cache entry the next frame steps.
    pub next: i32,
    /// The fade's level, 0…128.
    pub alpha: u8,
}

impl CacheShaderScreen {
    /// Opens the screen at `now`.
    pub fn new(now: u32) -> Self {
        Self {
            phase: WarmScreenPhase::FadingIn,
            start: now,
            next: 0,
            alpha: 0,
        }
    }

    /// One frame. `warm(index)` answers whether cache entry `index` is
    /// already compiled (false means it was compiled this frame). Returns
    /// whether the form should close.
    pub fn step(&mut self, now: u32, mut warm: impl FnMut(i32) -> bool) -> bool {
        match self.phase {
            WarmScreenPhase::FadingIn => {
                let mut t = (u64::from(now) - u64::from(self.start)) as f64 / 200.0;
                if t >= 1.0 {
                    self.phase = WarmScreenPhase::Warming;
                    t = 1.0;
                }
                self.alpha = (t as f32 * SCREEN_ALPHA).round_ties_even() as u8;
                false
            }
            WarmScreenPhase::Warming => {
                if !warm(self.next) {
                    self.phase = WarmScreenPhase::FadingOut;
                    self.start = now;
                }
                self.next = self.next.wrapping_add(1);
                false
            }
            WarmScreenPhase::FadingOut => {
                let mut t = 1.0 - (u64::from(now) - u64::from(self.start)) as f64 / 200.0;
                let mut close = false;
                if t < 0.001 {
                    close = true;
                    t = 0.0;
                }
                self.alpha = (t as f32 * SCREEN_ALPHA).round_ties_even() as u8;
                close
            }
        }
    }
}

impl CacheShaderScreen {
    /// The screen does not take a hit.
    pub fn hit(&self) -> i32 {
        -1
    }

    /// The destroy registers its per-frame task and runs the base.
    pub fn destroy(&self) -> (bool, bool) {
        (true, true)
    }

    /// The text's centred origin and the colour's alpha.
    pub fn draw(
        &self,
        width: i32,
        line_height_view_top: i32,
        text_width: i32,
        palette_alpha: u8,
    ) -> ([i32; 2], u8) {
        (
            [(width - text_width) / 2, (line_height_view_top) / 2],
            screen_colour_alpha(255, palette_alpha),
        )
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

/// The alpha a small screen's colour takes: the colour's own alpha times
/// the palette's, truncated after dividing by 128. The original converts
/// through a double and truncates toward zero.
pub fn screen_colour_alpha(colour_alpha: u8, palette_alpha: u8) -> u8 {
    let product = u64::from(colour_alpha) * u64::from(palette_alpha);
    ((product as f64 * 0.0078125) as i64) as u8
}

/// The key screen's constructor values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadKeyForm {
    pub flags: u32,
    pub form_flags: u32,
    pub digit: i32,
    /// The colour the prompt draws in (opaque black).
    pub colour: u32,
}

pub fn read_key_form() -> ReadKeyForm {
    ReadKeyForm {
        flags: 0x1020,
        form_flags: 0xf0008,
        digit: 0,
        colour: 0xff00_0000,
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

/// The key-capture screen (`TReadKeyScreen`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadKeyScreen {
    /// The action being rebound (the control id).
    pub action: u32,
    pub phase: ReadKeyPhase,
    pub start: u32,
    pub alpha: u8,
}

impl ReadKeyScreen {
    pub fn new(action: u32, now: u32) -> Self {
        Self {
            action,
            phase: ReadKeyPhase::FadingIn,
            start: now,
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

    /// The key-capture registration: a key already bound to another action
    /// (index ≠ action − 1) raises the warning and fails; otherwise the key
    /// is stored in the table at `action` and the capture succeeds.
    pub fn register(&mut self, bindings: &mut [u32], action: u32, key: u32, now: u32) -> bool {
        let action_index = action as usize;
        let mut found = false;
        for (i, bound) in bindings.iter().enumerate().take(22) {
            if action.wrapping_sub(1) != i as u32 && *bound == key {
                found = true;
                break;
            }
        }
        if found {
            self.phase = ReadKeyPhase::InUse;
            self.start = now;
            return false;
        }
        if action_index < bindings.len() {
            bindings[action_index] = key;
        }
        true
    }

    /// A mouse button: 4, 8 and 16 map to the original's synthetic codes.
    /// Returns whether the capture succeeded (and the form closes).
    pub fn mouse(&mut self, bindings: &mut [u32], button: u32, now: u32) -> Option<u32> {
        let key = match button {
            4 => 0x8004,
            8 => 0x8005,
            0x10 => 0x8006,
            _ => return None,
        };
        self.register(bindings, self.action, key, now)
            .then_some(key)
    }

    /// A keyboard or pad event. `scan` is the game's key-state word (0 when
    /// there is none); a scan with the `0x100` code family becomes the
    /// `0x4000` form. Returns the key when the capture succeeded.
    pub fn key(
        &mut self,
        bindings: &mut [u32],
        code: u32,
        flags: u16,
        scan: u32,
        now: u32,
    ) -> Option<u32> {
        if flags & RELEASE != 0 {
            return None;
        }
        if code == KEY_ESCAPE {
            return None;
        }
        if scan == 0 {
            return None;
        }
        let mut key = (scan >> 16) & 0xfff;
        if key == 0x100 {
            key = (code & 0x3fff) | 0x4000;
        }
        self.register(bindings, self.action, key, now)
            .then_some(key)
    }

    /// One frame's fade and warning timer. Returns whether the form should
    /// close (never: the screen closes on a capture or Escape).
    pub fn step(&mut self, now: u32) -> bool {
        let mut t = (u64::from(now) - u64::from(self.start)) as f64 / 200.0;
        if t >= 1.0 {
            t = 1.0;
            if self.phase == ReadKeyPhase::FadingIn {
                self.phase = ReadKeyPhase::Ready;
            }
        }
        if self.phase == ReadKeyPhase::InUse
            && u64::from(self.start) + u64::from(IN_USE_MS) < u64::from(now)
        {
            self.phase = ReadKeyPhase::Ready;
        }
        self.alpha = (t as f32 * SCREEN_ALPHA).round_ties_even() as u8;
        false
    }
}

impl ReadKeyScreen {
    /// The draw: the prompt centred across and down by the font's line.
    pub fn draw(
        &self,
        width: i32,
        height: i32,
        line_height: i32,
        text_width: i32,
        palette_alpha: u8,
    ) -> ([i32; 2], &'static str, u8) {
        (
            [(width - text_width) / 2, (height - line_height) / 2],
            self.prompt(),
            screen_colour_alpha(255, palette_alpha),
        )
    }

    /// The hit test misses.
    pub fn hit(&self) -> i32 {
        -1
    }

    /// The destroy registers the per-frame hook and clears its globals.
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
    fn the_warm_screen_fades_in_steps_and_fades_out() {
        let mut s = CacheShaderScreen::new(1000);
        assert!(!s.step(1100, |_| true));
        assert_eq!(s.alpha, 64);
        assert!(!s.step(1200, |_| true));
        assert_eq!((s.phase, s.alpha), (WarmScreenPhase::Warming, 128));
        // The first two entries are already warm; the third compiles.
        assert!(!s.step(1300, |i| i < 2));
        assert!(!s.step(1300, |i| i < 2));
        assert_eq!(s.next, 2);
        assert!(!s.step(1400, |i| i < 2));
        assert_eq!((s.phase, s.next), (WarmScreenPhase::FadingOut, 3));
        assert!(!s.step(1500, |_| true));
        assert_eq!(s.alpha, 64);
        assert!(!s.step(1599, |_| true));
        assert!(s.step(1600, |_| true));
        assert_eq!(s.alpha, 0);
    }

    #[test]
    fn the_key_screen_warns_about_an_existing_binding() {
        let mut screen = ReadKeyScreen::new(3, 0);
        let mut binds = [0x11u32, 0x12, 0x13, 0];
        assert!(screen.register(&mut binds, 3, 0x14, 0));
        assert_eq!(binds[3], 0x14);
        assert!(!screen.register(&mut binds, 3, 0x11, 500));
        assert_eq!((screen.phase, screen.start), (ReadKeyPhase::InUse, 500));
        assert_eq!(screen.prompt(), ALREADY_IN_USE);
        assert_eq!(screen.mouse(&mut binds, 8, 600), Some(0x8005));
        assert_eq!(screen.mouse(&mut binds, 1, 600), None);
        assert_eq!(
            screen.key(&mut binds, 0x41, 0, (0x100 << 16) | 7, 700),
            Some(0x4041)
        );
        // Release and Escape capture nothing.
        assert_eq!(screen.key(&mut binds, 0x41, RELEASE, 7 << 16, 700), None);
        assert_eq!(screen.key(&mut binds, KEY_ESCAPE, 0, 7 << 16, 700), None);
    }

    #[test]
    fn the_key_screen_fades_and_clears_the_warning() {
        let mut s = ReadKeyScreen::new(1, 0);
        s.step(100);
        assert_eq!((s.alpha, s.phase), (64, ReadKeyPhase::FadingIn));
        s.step(200);
        assert_eq!((s.alpha, s.phase), (128, ReadKeyPhase::Ready));
        s.phase = ReadKeyPhase::InUse;
        s.start = 1000;
        s.step(1100);
        assert_eq!(s.phase, ReadKeyPhase::InUse);
        s.step(2001);
        assert_eq!(s.phase, ReadKeyPhase::Ready);
        // A fresh warning's 1000 units are measured from its own start.
        s.phase = ReadKeyPhase::InUse;
        s.start = 9000;
        s.step(9500);
        assert_eq!(s.phase, ReadKeyPhase::InUse);
        s.step(10_001);
        assert_eq!(s.phase, ReadKeyPhase::Ready);
    }

    #[test]
    fn the_forms_keep_their_constructor_values() {
        let c = cache_screen_form();
        assert_eq!((c.flags, c.form_flags, c.digit), (0x1020, 0xf0008, 0));
        assert_eq!(c.colour, 0xff000000);
        let mut warm = CacheShaderScreen::new(0);
        warm.step(200, |_| true);
        assert_eq!(warm.hit(), -1);
        assert_eq!(warm.destroy(), (true, true));
        assert!(warm.release_text());
        assert_eq!(warm.draw(400, 200, 100, 255).0, [150, 100]);
        let r = read_key_form();
        assert_eq!((r.flags, r.colour), (0x1020, 0xff000000));
        let screen = ReadKeyScreen::new(2, 0);
        assert_eq!(screen.hit(), -1);
        assert!(screen.release());
        let (at, text, alpha) = screen.draw(400, 200, 20, 80, 255);
        assert_eq!(at, [160, 90]);
        assert_eq!(text, PRESS_A_KEY);
        assert_eq!(alpha, 252);
    }

    #[test]
    fn the_screen_colour_alpha_truncates() {
        assert_eq!(screen_colour_alpha(255, 255), 252);
        assert_eq!(screen_colour_alpha(255, 64), 127);
        assert_eq!(screen_colour_alpha(128, 255), 255);
        assert_eq!(screen_colour_alpha(0, 200), 0);
    }
}

//! The start-up splash screen: the two BME logos, their fades and the
//! hand-over to the main menu.
//!
//! The screen is a small state machine on the real clock. It waits a second
//! while it warms up the renderer's programs, fades the first logo in over
//! 600 ms, holds it two seconds, shows the second logo with the
//! `BME Critters` sound, holds it 200 ms while the interface settles, fades
//! it out over 300 ms, waits for the warm-up to finish, hands over to the
//! main menu, waits a second, draws the menu's theme and fades the whole
//! screen to the game over a second. Every logo is centred and scaled by
//! `round(height/135)·0.125`, the same quantised scale the menu uses.
//!
//! The textures are loaded by [`Splash::new`]'s caller; this module holds
//! their sizes and the state machine, not the drawn quads. [`SplashPlugin`]
//! can run it (nothing starts it by default: Exarust's launcher opens a level
//! view rather than the original's menu).
//!
//! Evidence: the splash screen's load, draw and destroy routines and their
//! state words.

use bevy::prelude::*;
use exa_assets::Game;
use exa_formats::rfi::Rfi;

use crate::flow::Viewport;

/// The first logo's texture.
pub const LOGO_1: &str = "bmelogol01.rfi";
/// The second logo's texture.
pub const LOGO_2: &str = "bmelogol02.rfi";
/// The sound that plays with the second logo.
pub const HAND_OVER_SOUND: &str = "BME Critters";
/// Its volume.
pub const SOUND_VOLUME: f32 = 0.2;
/// Its priority.
pub const SOUND_PRIORITY: u32 = 0x4060;

/// How long the renderer's programs warm up before the first fade.
pub const WARM_UP_MS: u32 = 1000;
/// How long the first logo stays fully shown.
pub const LOGO_HOLD_MS: u32 = 2000;
/// How long the second logo stays before its fade.
pub const SECOND_HOLD_MS: u32 = 200;
/// How long the screen waits after the hand-over before it fades to the game.
pub const MENU_HOLD_MS: u32 = 1000;
/// The fade-in rate per millisecond, as a double.
pub const FADE_IN_RATE: f64 = 0.001_666_666_666_666_666_8;
/// The fade-out rate per millisecond, as a double.
pub const FADE_OUT_RATE: f64 = 0.003_333_333_333_333_333_5;
/// The final fade-to-game rate per millisecond, as a double.
pub const MENU_FADE_RATE: f64 = 0.001;
/// The fade value below which the screen closes: the smallest float above
/// zero that the original's fade rate can produce in one step.
pub const FADE_EPSILON: f32 = 1.192_089_982_282_595_9e-07;
/// The size quantisation the logos share with the menu.
pub const SCALE_STEP: f64 = 0.125;
/// The client height one scale step stands for.
pub const SCALE_DIVISOR: f64 = 135.0;

/// One logo's stored size, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Logo {
    pub width: u16,
    pub height: u16,
}

/// A drawn logo's place, in client pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogoRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl LogoRect {
    /// The rectangle's right and bottom edges.
    pub fn right(&self) -> f32 {
        self.x + self.width
    }

    /// The rectangle's bottom edge.
    pub fn bottom(&self) -> f32 {
        self.y + self.height
    }
}

/// The splash's state word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum State {
    WarmUp,
    FadeIn,
    SecondLogo,
    SecondHold,
    FadeOut,
    Settle,
    HandOver,
    MenuWait,
    Menu,
    MenuFade,
    FadeToGame,
}

impl State {
    /// The state a word names.
    pub const fn from_word(word: u8) -> Self {
        match word {
            1 => State::FadeIn,
            2 => State::SecondLogo,
            3 => State::SecondHold,
            4 => State::FadeOut,
            5 => State::Settle,
            6 => State::HandOver,
            7 => State::MenuWait,
            8 => State::Menu,
            9 => State::MenuFade,
            10 => State::FadeToGame,
            _ => State::WarmUp,
        }
    }
}

/// What one splash tick asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SplashFrame {
    /// A renderer warm-up pass runs this frame.
    pub warm_up: bool,
    /// The logo to draw and where.
    pub logo: Option<(usize, LogoRect)>,
    pub alpha: u8,
    pub darkness: f32,
    pub fade: f32,
    /// The sound to start, at this volume.
    pub sound: Option<(&'static str, f32)>,
    /// The hand-over callback runs (the main menu is built and shown).
    pub menu: bool,
    /// The menu's theme starts.
    pub theme: bool,
    /// The screen closes (the form is destroyed).
    pub closed: bool,
}

/// The splash screen.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Splash {
    /// The state word (0 to 10).
    pub state: u8,
    /// The state's start time.
    pub start_ms: u32,
    /// The current stage's stamp.
    pub stamp_ms: u32,
    /// The form's overall alpha.
    pub alpha: u8,
    /// The screen's darkening.
    pub darkness: f32,
    /// The logo's fade.
    pub fade: f32,
    /// The warm-up frame counter.
    pub frame: u32,
    /// The busy count the splash holds; the game's loading counter.
    pub busy: i32,
    /// The menu's theme plays on the hand-over.
    pub theme_on_menu: bool,
    pub logos: [Logo; 2],
}

impl Splash {
    /// Loads the splash at `now`. It holds one busy count; the original
    /// raises the counter until the menu has faded in.
    pub fn new(now: u32, logos: [Logo; 2]) -> Self {
        Self {
            state: 0,
            start_ms: now,
            stamp_ms: 0,
            alpha: 255,
            darkness: 0.0,
            fade: 0.0,
            frame: 0,
            busy: 1,
            theme_on_menu: false,
            logos,
        }
    }

    /// The state.
    pub fn phase(&self) -> State {
        State::from_word(self.state)
    }

    /// One tick at `now`. `programs_ready` is the renderer's warm-up answer:
    /// the settle state waits for it.
    pub fn tick(&mut self, now: u32, viewport: Viewport, programs_ready: bool) -> SplashFrame {
        let mut out = SplashFrame::default();
        match self.phase() {
            State::WarmUp => {
                if u64::from(self.start_ms) + u64::from(WARM_UP_MS) < u64::from(now) {
                    self.state = 1;
                    self.stamp_ms = now;
                } else {
                    out.warm_up = true;
                    self.frame = self.frame.wrapping_add(1);
                }
                self.alpha = 255;
                self.fade = 1.0;
            }
            State::FadeIn => {
                self.fade = fade_in(now.wrapping_sub(self.stamp_ms));
                if 1.0 < self.fade {
                    self.fade = 1.0;
                }
                self.alpha = alpha_of(self.fade);
                out.logo = Some((0, logo_rect(viewport, self.logos[0])));
                if u64::from(self.stamp_ms) + u64::from(LOGO_HOLD_MS) < u64::from(now) {
                    self.state = 2;
                }
            }
            State::SecondLogo => {
                self.darkness = 1.0;
                out.sound = Some((HAND_OVER_SOUND, SOUND_VOLUME));
                self.alpha = 255;
                out.logo = Some((1, logo_rect(viewport, self.logos[1])));
                self.state = 3;
                self.stamp_ms = now;
            }
            State::SecondHold => {
                out.logo = Some((1, logo_rect(viewport, self.logos[1])));
                if u64::from(self.stamp_ms) + u64::from(SECOND_HOLD_MS) < u64::from(now) {
                    self.state = 4;
                    self.stamp_ms = now;
                } else {
                    out.warm_up = true;
                }
            }
            State::FadeOut => {
                self.fade = 1.0 - fade_out(now.wrapping_sub(self.stamp_ms));
                if self.fade <= 0.0 {
                    self.fade = 0.0;
                    self.start_ms = now;
                    self.state = 5;
                    self.stamp_ms = now;
                }
                self.alpha = alpha_of(self.fade);
                out.logo = Some((1, logo_rect(viewport, self.logos[1])));
            }
            State::Settle => {
                if programs_ready {
                    self.state = 6;
                } else {
                    out.warm_up = true;
                    self.frame = self.frame.wrapping_add(1);
                }
            }
            State::HandOver => {
                out.menu = true;
                self.alpha = 255;
                self.state = 7;
            }
            State::MenuWait => {
                if u64::from(self.start_ms) + u64::from(MENU_HOLD_MS) < u64::from(now) {
                    self.state = 8;
                    self.darkness = 0.0;
                    self.release_busy();
                }
            }
            State::Menu => {
                self.state = 9;
                self.start_ms = now;
                self.stamp_ms = now;
            }
            State::MenuFade => {
                self.state = 10;
                if self.theme_on_menu {
                    out.theme = true;
                }
                self.release_busy();
                self.start_ms = now;
                self.stamp_ms = now;
                out.menu = true;
            }
            State::FadeToGame => {
                self.fade = 1.0 - menu_fade(now.wrapping_sub(self.stamp_ms));
                if self.fade < FADE_EPSILON {
                    self.darkness = 1.0;
                    self.fade = 0.0;
                    out.closed = true;
                } else {
                    self.darkness = 1.0 - self.fade;
                    self.alpha = alpha_of(self.fade);
                }
            }
        }
        out.alpha = self.alpha;
        out.darkness = self.darkness;
        out.fade = self.fade;
        out
    }

    /// The busy count's release, once: it never goes under zero.
    fn release_busy(&mut self) {
        if self.busy > 0 {
            self.busy -= 1;
        }
    }
}

/// The logo's place: the scale is `round(height/135)·0.125`, the logo's
/// stored size times it, centred on the client.
pub fn logo_rect(viewport: Viewport, logo: Logo) -> LogoRect {
    let steps = round_even(f64::from(viewport.height) * (1.0 / SCALE_DIVISOR));
    let scale = (steps as f64 * SCALE_STEP) as f32;
    let width = f32::from(logo.width) * scale;
    let height = f32::from(logo.height) * scale;
    LogoRect {
        x: (viewport.width as f32 - width) * 0.5,
        y: (viewport.height as f32 - height) * 0.5,
        width,
        height,
    }
}

/// A double rounded to the nearest integer, ties to even (the processor's
/// default conversion).
fn round_even(value: f64) -> i64 {
    if !value.is_finite() || value >= 9.223_372e18 || value < -9.223_372e18 {
        i64::MIN
    } else {
        value.round_ties_even() as i64
    }
}

/// The alpha the fade gives: the fade times 255, converted with the
/// processor's rounding and kept as a byte.
fn alpha_of(fade: f32) -> u8 {
    let value = fade * 255.0;
    if !value.is_finite() || value >= 9.223_372e18 || value < -9.223_372e18 {
        0
    } else {
        value.round_ties_even() as i64 as u8
    }
}

/// The fade-in value after `elapsed` milliseconds: the elapsed time times
/// the rate in double precision, kept in single.
fn fade_in(elapsed: u32) -> f32 {
    (f64::from(elapsed) * FADE_IN_RATE) as f32
}

/// The fade-out value after `elapsed` milliseconds.
fn fade_out(elapsed: u32) -> f32 {
    (f64::from(elapsed) * FADE_OUT_RATE) as f32
}

/// The final fade after `elapsed` milliseconds.
fn menu_fade(elapsed: u32) -> f32 {
    (f64::from(elapsed) * MENU_FADE_RATE) as f32
}

/// The splash's running form.
#[derive(Resource, Default)]
pub struct SplashScreen(pub Option<Splash>);

impl SplashScreen {
    /// Starts the splash at `now`, if it is not already up.
    pub fn begin(&mut self, now: u32, logos: [Logo; 2]) {
        if self.0.is_none() {
            self.0 = Some(Splash::new(now, logos));
        }
    }

    /// Stops the splash.
    pub fn stop(&mut self) {
        self.0 = None;
    }

    /// Whether the splash is up.
    pub fn is_active(&self) -> bool {
        self.0.is_some()
    }
}

/// The two logos' sizes, read once from the game's textures.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SplashLogos(pub [Logo; 2]);

impl Default for SplashLogos {
    fn default() -> Self {
        // A placeholder until the game's textures are read.
        Self([
            Logo {
                width: 512,
                height: 128,
            },
            Logo {
                width: 512,
                height: 128,
            },
        ])
    }
}

/// Reads the two logos' sizes from the game's textures.
pub fn read_logos(game: &Game) -> Option<[Logo; 2]> {
    let read = |name: &str| {
        let rfi = Rfi::parse(game.0.texture(name)?).ok()?;
        let width = u16::try_from(rfi.header.width).ok()?;
        let height = u16::try_from(rfi.header.height).ok()?;
        Some(Logo { width, height })
    };
    Some([read(LOGO_1)?, read(LOGO_2)?])
}

/// The splash's plugin: it reads the logo sizes, and ticks the screen when
/// one was started. Nothing starts it by default; the launcher opens a level
/// view, not the original's menu flow.
pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashScreen>()
            .init_resource::<SplashLogos>()
            .add_systems(Startup, load_logos)
            .add_systems(Update, tick_splash);
    }
}

fn load_logos(mut logos: ResMut<SplashLogos>, game: Option<Res<Game>>) {
    if let Some(game) = game.as_ref()
        && let Some(read) = read_logos(game)
    {
        logos.0 = read;
    }
}

fn tick_splash(
    mut screen: ResMut<SplashScreen>,
    logos: Res<SplashLogos>,
    time: Res<Time<Real>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let Some(splash) = screen.0.as_mut() else {
        return;
    };
    let viewport = windows
        .single()
        .ok()
        .map(|window| Viewport {
            width: window.physical_width(),
            height: window.physical_height(),
        })
        .unwrap_or(Viewport {
            width: logos.0[0].width.into(),
            height: logos.0[0].height.into(),
        });
    let now = time.elapsed().as_millis() as u32;
    let frame = splash.tick(now, viewport, true);
    if let Some((name, volume)) = frame.sound {
        info!("splash: {name} at {volume}");
    }
    if frame.menu {
        info!("splash: handing over to the main menu");
    }
    if frame.closed {
        info!("splash: closed");
        screen.0 = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VIEW: Viewport = Viewport {
        width: 1600,
        height: 1080,
    };

    fn logos() -> [Logo; 2] {
        [
            Logo {
                width: 512,
                height: 128,
            },
            Logo {
                width: 256,
                height: 64,
            },
        ]
    }

    #[test]
    fn the_logos_are_centred_and_scaled_by_135ths() {
        let rect = logo_rect(VIEW, logos()[0]);
        // 1080/135 = 8 steps, times 0.125: scale 1; 512×128 centred.
        assert_eq!(rect.width, 512.0);
        assert_eq!(rect.height, 128.0);
        assert_eq!(rect.x, 544.0);
        assert_eq!(rect.y, 476.0);
        assert_eq!(rect.right(), 1056.0);
        assert_eq!(rect.bottom(), 604.0);
        let small = logo_rect(
            Viewport {
                width: 800,
                height: 453,
            },
            logos()[1],
        );
        // 453/135 = 3.3556, round to 3 steps, times 0.125 = 0.375.
        assert_eq!(small.width, 96.0);
        assert_eq!(small.height, 24.0);
        assert_eq!(small.x, 352.0);
        assert_eq!(small.y, 214.5);
        // A zero-height client has no scale.
        let empty = logo_rect(
            Viewport {
                width: 100,
                height: 0,
            },
            logos()[0],
        );
        assert_eq!(empty.width, 0.0);
        assert_eq!(empty.x, 50.0);
    }

    #[test]
    fn the_splash_waits_a_second_then_fades_in() {
        let mut splash = Splash::new(1000, logos());
        assert_eq!(splash.phase(), State::WarmUp);
        let frame = splash.tick(1000, VIEW, true);
        assert!(frame.warm_up);
        assert_eq!(frame.alpha, 255);
        assert_eq!(frame.fade, 1.0);
        // Strictly more than a second: the fade starts at 2001 ms.
        let frame = splash.tick(2000, VIEW, true);
        assert!(frame.warm_up);
        assert_eq!(splash.phase(), State::WarmUp);
        let frame = splash.tick(2001, VIEW, true);
        assert_eq!(splash.phase(), State::FadeIn);
        assert_eq!(splash.stamp_ms, 2001);
        assert!(frame.logo.is_none());
        let frame = splash.tick(2002, VIEW, true);
        assert_eq!(frame.logo.unwrap().0, 0);
        let frame = splash.tick(2101, VIEW, true);
        // 100 ms at 1/600 a millisecond is 0.16666667; times 255 is 42.5,
        // which rounds half to even: 42.
        assert_eq!(frame.fade, 0.166_666_67);
        assert_eq!(frame.alpha, 42);
        // 600 ms is full white: 255.
        let frame = splash.tick(2601, VIEW, true);
        assert_eq!(frame.fade, 1.0);
        assert_eq!(frame.alpha, 255);
        // The first logo holds until 2000 ms after the fade started.
        assert_eq!(splash.phase(), State::FadeIn);
        let frame = splash.tick(4601, VIEW, true);
        assert_eq!(splash.phase(), State::SecondLogo);
        // The first logo still draws on the tick that changes state, but the
        // sound and the second logo draw are the next tick's.
        assert_eq!(frame.logo.unwrap().0, 0);
        let frame = splash.tick(4602, VIEW, true);
        assert_eq!(frame.sound, Some((HAND_OVER_SOUND, SOUND_VOLUME)));
        assert_eq!(frame.darkness, 1.0);
        assert_eq!(frame.logo.unwrap().0, 1);
    }

    #[test]
    fn the_second_logo_holds_fades_and_settles() {
        let mut splash = Splash::new(0, logos());
        splash.state = 3;
        splash.start_ms = 5000;
        splash.stamp_ms = 5200;
        splash.tick(5200, VIEW, true);
        assert_eq!(splash.phase(), State::SecondHold);
        splash.tick(5400, VIEW, true);
        assert_eq!(splash.phase(), State::SecondHold);
        let frame = splash.tick(5401, VIEW, true);
        assert_eq!(splash.phase(), State::FadeOut);
        assert!(!frame.warm_up);
        // 100 ms of the 300 ms fade: 1 − 0.33333334.
        let frame = splash.tick(5501, VIEW, true);
        assert!((frame.fade - 0.666_666_7).abs() < 1e-6);
        assert_eq!(frame.alpha, 170);
        // 300 ms: zero, and the settle starts.
        let frame = splash.tick(5701, VIEW, true);
        assert_eq!(splash.phase(), State::Settle);
        assert_eq!(frame.alpha, 0);
        assert_eq!(splash.start_ms, 5701);
        // A warm-up that is not finished keeps it there; then it hands over.
        let frame = splash.tick(5900, VIEW, false);
        assert!(frame.warm_up);
        assert_eq!(splash.phase(), State::Settle);
        let frame = splash.tick(5901, VIEW, true);
        assert!(!frame.menu);
        assert_eq!(splash.phase(), State::HandOver);
        let frame = splash.tick(5902, VIEW, true);
        assert!(frame.menu);
        assert_eq!(splash.alpha, 255);
        assert_eq!(splash.phase(), State::MenuWait);
        // The hand-over's second counts from the fade-out's end.
        let frame = splash.tick(6702, VIEW, true);
        assert_eq!(splash.phase(), State::Menu);
        assert_eq!(splash.darkness, 0.0);
        assert_eq!(frame.darkness, 0.0);
        assert_eq!(splash.busy, 0);
        let frame = splash.tick(6703, VIEW, true);
        assert!(!frame.menu);
        assert_eq!(splash.phase(), State::MenuFade);
        assert_eq!(splash.busy, 0);
        let frame = splash.tick(6704, VIEW, true);
        assert!(frame.menu);
        assert_eq!(splash.phase(), State::FadeToGame);
    }

    #[test]
    fn the_last_fade_closes_the_screen() {
        let mut splash = Splash::new(0, logos());
        splash.state = 10;
        splash.stamp_ms = 10_000;
        splash.fade = 1.0;
        let frame = splash.tick(10_100, VIEW, true);
        // 1 − 0.1 = 0.9; darkness 0.1; alpha 230 (229.5 rounds to even 230).
        assert!((frame.fade - 0.9).abs() < 1e-6);
        assert!((frame.darkness - 0.1).abs() < 1e-6);
        assert_eq!(frame.alpha, 230);
        let frame = splash.tick(10_999, VIEW, true);
        assert!(!frame.closed);
        // 0.001 × 255 = 0.255, which rounds to 0.
        assert_eq!(frame.alpha, 0);
        // At 11 000 the fade is exactly 0, under the epsilon: it closes.
        let frame = splash.tick(11_000, VIEW, true);
        assert!(frame.closed);
        assert_eq!(splash.darkness, 1.0);
        assert_eq!(splash.fade, 0.0);
    }

    #[test]
    fn the_state_words_and_the_screen_resource() {
        for (word, state) in [
            (0, State::WarmUp),
            (1, State::FadeIn),
            (2, State::SecondLogo),
            (3, State::SecondHold),
            (4, State::FadeOut),
            (5, State::Settle),
            (6, State::HandOver),
            (7, State::MenuWait),
            (8, State::Menu),
            (9, State::MenuFade),
            (10, State::FadeToGame),
        ] {
            assert_eq!(State::from_word(word), state);
        }
        let mut screen = SplashScreen::default();
        assert!(!screen.is_active());
        screen.begin(10, logos());
        assert!(screen.is_active());
        screen.begin(20, logos());
        assert_eq!(screen.0.unwrap().start_ms, 10);
        screen.stop();
        assert!(!screen.is_active());
    }

    #[test]
    fn the_alpha_rounds_to_even_and_the_logo_sizes_are_read() {
        assert_eq!(alpha_of(0.0), 0);
        assert_eq!(alpha_of(1.0), 255);
        // 0.5·255 = 127.5, halfway: to even is 128.
        assert_eq!(alpha_of(0.5), 128);
        // 0.1·255 = 25.5: to even is 26.
        assert_eq!(alpha_of(0.1), 26);
        assert_eq!(alpha_of(f32::NAN), 0);
        assert_eq!(fade_in(600), (600.0f64 * FADE_IN_RATE) as f32);
        assert_eq!(fade_in(600), 1.0);
    }
}

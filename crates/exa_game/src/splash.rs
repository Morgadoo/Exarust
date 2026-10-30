//! The start-up splash screen: the two BME logos, their fades and the
//! hand-over to the main menu.
//!
//! The screen is a small state machine. Every tick it first draws its black
//! backdrop over the whole client at the backdrop's alpha, then runs its
//! state: it waits a second while it warms up the renderer's programs, fades
//! the first logo in from black over 600 ms (the logo's grey level rises; the
//! backdrop stays opaque), holds it two seconds, shows the second logo with
//! the `BME Critters` sound, holds it 200 ms while the warm-up goes on, fades
//! it to black over 300 ms, finishes the warm-up, runs the hand-over (the
//! main menu is built behind the splash), waits a second, asks for the menu's
//! theme and fades the backdrop out over a second. Every logo is centred and
//! scaled by `round(height/135)·0.125`, the same quantised scale the menu
//! uses.
//!
//! Two clocks are read, as in the original: the frame's time (sampled when
//! the frame starts; every comparison and elapsed time uses it) and the
//! system clock (read when a stage is stamped). An elapsed time is their
//! signed 64-bit difference, so a stamp taken after the frame started gives a
//! small negative time, not a wrapped one.
//!
//! The splash also drives two global fades: the screen-transition audio gain
//! (silent from the load until the second logo, silent again after the menu
//! wait, then rising with the last fade) and the marker outline fade (1, the
//! outlines hidden, during the warm-up; falling to 0 with the last fade).
//!
//! The form's hit-test callback and its destroy order are
//! [`exa_ui::menu::splash_bounds`] and [`exa_ui::menu::destroy_splash`].
//! [`SplashPlugin`] runs and draws the screen when the launcher asks for it
//! (`--splash`); the hand-over opens the main menu screen.
//!
//! Evidence: the splash screen's load, state machine, logo draw and create
//! routines, their state words and constants.

use bevy::audio::{AudioPlayer, PlaybackSettings, Volume};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_assets::{ColorSpace, Game};
use exa_formats::rfi::Rfi;

use crate::flow::{Step, Viewport, menu_theme};

/// The first logo's texture.
pub const LOGO_1: &str = "bmelogol01.rfi";
/// The second logo's texture.
pub const LOGO_2: &str = "bmelogol02.rfi";
/// The sound that plays with the second logo.
pub const HAND_OVER_SOUND: &str = "BME Critters";
/// Its volume.
pub const SOUND_VOLUME: f32 = 0.2;
/// Its pitch.
pub const SOUND_PITCH: f32 = 1.0;
/// Its priority.
pub const SOUND_PRIORITY: u32 = 0x4060;

/// The form flags the splash's constructor gives it (style 0).
pub const FORM_FLAGS: u32 = 0x1020;
/// The form's background ink: opaque black.
pub const BACKGROUND: [u8; 4] = [0, 0, 0, 255];

/// The sound front end's play record: the origin it plays from ([0, 0, 0]),
/// the volume and pitch, and the priority word. Volume and pitch are single
/// precision; the priority is the original's own word and has no Bevy
/// counterpart.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SplashSound {
    pub name: &'static str,
    pub position: [f32; 3],
    pub volume: f32,
    pub pitch: f32,
    pub priority: u32,
}

/// The hand-over sound the load resolves by name and the second logo plays.
pub const HAND_OVER: SplashSound = SplashSound {
    name: HAND_OVER_SOUND,
    position: [0.0, 0.0, 0.0],
    volume: SOUND_VOLUME,
    pitch: SOUND_PITCH,
    priority: SOUND_PRIORITY,
};

/// What destroying the splash releases: its two logo textures, its sound
/// and the warm-up pass counter (the original also clears a unit flag).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplashDestroy {
    pub logos: bool,
    pub sound: bool,
    pub pass_counter: bool,
}

/// How long the renderer's programs warm up before the first fade.
pub const WARM_UP_MS: u32 = 1000;
/// How long the first logo stays from the start of its fade.
pub const LOGO_HOLD_MS: u32 = 2000;
/// How long the second logo stays before its fade.
pub const SECOND_HOLD_MS: u32 = 200;
/// How long the screen waits after the fade-out before it starts the menu.
pub const MENU_HOLD_MS: u32 = 1000;
/// The fade-in rate per millisecond, as a double (1/600).
pub const FADE_IN_RATE: f64 = 0.001_666_666_666_666_666_8;
/// The fade-out rate per millisecond, as a double (1/300).
pub const FADE_OUT_RATE: f64 = 0.003_333_333_333_333_333_5;
/// The final fade's rate per millisecond, as a double.
pub const MENU_FADE_RATE: f64 = 0.001;
/// The final fade's value below which the screen closes.
pub const FADE_EPSILON: f32 = 1.192_089_982_282_595_9e-07;
/// The size quantisation the logos share with the menu.
pub const SCALE_STEP: f64 = 0.125;
/// The reciprocal of the client height one scale step stands for (1/135).
pub const SCALE_PER_PIXEL: f64 = 0.007_407_407_407_407_408;

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
    /// The rectangle's right edge.
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
    /// The state a word names. Words past 10 match no state: the tick only
    /// draws the backdrop.
    pub const fn from_word(word: u8) -> Option<Self> {
        Some(match word {
            0 => State::WarmUp,
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
            _ => return None,
        })
    }
}

/// What one splash tick asks for.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SplashFrame {
    /// The backdrop's alpha this tick (the backdrop is drawn first, black).
    pub alpha: u8,
    /// A renderer warm-up pass runs this tick, with this pass number.
    pub warm_up: Option<u32>,
    /// The logo to draw, where, and its grey level (the logo is drawn at
    /// `(tint, tint, tint, 255)`).
    pub logo: Option<(usize, LogoRect, u8)>,
    /// The screen-transition audio gain after the tick.
    pub audio_fade: f32,
    /// The marker outline fade after the tick.
    pub marker_fade: f32,
    /// The sound to start.
    pub sound: bool,
    /// The hand-over runs: its callback (the main menu is built and shown)
    /// and the form is raised over the menu.
    pub hand_over: bool,
    /// The main menu's form is brought to the front (the menu stage).
    pub focus_menu: bool,
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
    /// The load's (and later stages') start, on either clock.
    pub start_ms: u32,
    /// The current stage's stamp, on the system clock.
    pub stamp_ms: u32,
    /// The backdrop's alpha.
    pub alpha: u8,
    /// The screen-transition audio gain.
    pub audio_fade: f32,
    /// The marker outline fade.
    pub marker_fade: f32,
    /// The warm-up pass counter (reset by the load).
    pub frame: u32,
    /// The busy count the splash holds: the game's loading counter.
    pub busy: i32,
    /// The menu's theme plays at the menu stage.
    pub theme_on_menu: bool,
    pub logos: [Logo; 2],
}

impl Splash {
    /// The splash as its creator and its load leave it, at the system clock
    /// `clock_ms`: the backdrop opaque, the audio gain silent, one busy count
    /// held (the counter is raised only when it is below one; the caller's
    /// counter is taken as zero) and the warm-up counter cleared. The boot's
    /// creator asks for the menu theme.
    pub fn new(clock_ms: u32, logos: [Logo; 2]) -> Self {
        Self {
            state: 0,
            start_ms: clock_ms,
            stamp_ms: 0,
            alpha: 255,
            audio_fade: 0.0,
            marker_fade: 0.0,
            frame: 0,
            busy: 1,
            theme_on_menu: true,
            logos,
        }
    }

    /// The state.
    pub fn phase(&self) -> Option<State> {
        State::from_word(self.state)
    }

    /// One tick. `frame_ms` is the frame's time, `clock_ms` the system clock
    /// read for a stamp; `programs_ready` is the settle stage's warm-up
    /// answer (the pass reports that nothing is left to build).
    pub fn tick(
        &mut self,
        frame_ms: u32,
        clock_ms: u32,
        viewport: Viewport,
        programs_ready: bool,
    ) -> SplashFrame {
        let mut out = SplashFrame {
            alpha: self.alpha,
            ..Default::default()
        };
        let passed =
            |since: u32, hold: u32| u64::from(since) + u64::from(hold) < u64::from(frame_ms);
        match self.phase() {
            Some(State::WarmUp) => {
                if passed(self.start_ms, WARM_UP_MS) {
                    self.state = 1;
                    self.stamp_ms = clock_ms;
                } else {
                    out.warm_up = Some(self.warm_up_pass());
                }
                self.alpha = 255;
                self.marker_fade = 1.0;
            }
            Some(State::FadeIn) => {
                let mut fade = (elapsed(frame_ms, self.stamp_ms) * FADE_IN_RATE) as f32;
                if 1.0 < fade {
                    fade = 1.0;
                }
                out.logo = Some((0, logo_rect(viewport, self.logos[0]), grey(fade)));
                if passed(self.stamp_ms, LOGO_HOLD_MS) {
                    self.state = 2;
                }
            }
            Some(State::SecondLogo) => {
                self.audio_fade = 1.0;
                out.sound = true;
                out.logo = Some((1, logo_rect(viewport, self.logos[1]), 255));
                self.state = 3;
                self.stamp_ms = clock_ms;
            }
            Some(State::SecondHold) => {
                out.logo = Some((1, logo_rect(viewport, self.logos[1]), 255));
                if passed(self.stamp_ms, SECOND_HOLD_MS) {
                    self.state = 4;
                    self.stamp_ms = clock_ms;
                } else {
                    out.warm_up = Some(self.warm_up_pass());
                }
            }
            Some(State::FadeOut) => {
                let mut fade = (1.0 - elapsed(frame_ms, self.stamp_ms) * FADE_OUT_RATE) as f32;
                if fade <= 0.0 {
                    fade = 0.0;
                    self.start_ms = frame_ms;
                    self.state = 5;
                    self.stamp_ms = clock_ms;
                }
                out.logo = Some((1, logo_rect(viewport, self.logos[1]), grey(fade)));
            }
            Some(State::Settle) => {
                // The pass runs every tick of the stage; the stage ends on
                // the tick whose pass has nothing left to build.
                out.warm_up = Some(self.frame);
                if programs_ready {
                    self.state = 6;
                }
                self.frame = self.frame.wrapping_add(1);
            }
            Some(State::HandOver) => {
                out.hand_over = true;
                self.alpha = 255;
                self.state = 7;
            }
            Some(State::MenuWait) => {
                if passed(self.start_ms, MENU_HOLD_MS) {
                    self.state = 8;
                    self.audio_fade = 0.0;
                    self.release_busy();
                }
            }
            Some(State::Menu) => {
                self.state = 9;
                self.start_ms = frame_ms;
                self.stamp_ms = clock_ms;
            }
            Some(State::MenuFade) => {
                self.state = 10;
                out.theme = self.theme_on_menu;
                self.release_busy();
                self.start_ms = frame_ms;
                self.stamp_ms = clock_ms;
                out.focus_menu = true;
            }
            Some(State::FadeToGame) => {
                let fade = (1.0 - elapsed(frame_ms, self.stamp_ms) * MENU_FADE_RATE) as f32;
                // A NaN fade keeps the screen up, as the original's compare.
                if fade < FADE_EPSILON {
                    self.audio_fade = 1.0;
                    self.marker_fade = 0.0;
                    out.closed = true;
                } else {
                    self.marker_fade = fade;
                    self.audio_fade = 1.0 - fade;
                    self.alpha = round_to_i64(fade * 255.0) as u8;
                }
            }
            None => {}
        }
        out.audio_fade = self.audio_fade;
        out.marker_fade = self.marker_fade;
        out
    }

    /// One warm-up pass: the counter's value for this pass, then counted.
    fn warm_up_pass(&mut self) -> u32 {
        let pass = self.frame;
        self.frame = self.frame.wrapping_add(1);
        pass
    }

    /// The busy count's release: it never goes under zero.
    fn release_busy(&mut self) {
        if self.busy > 0 {
            self.busy -= 1;
        }
    }

    /// The destroy: the two logo textures and the sound record go, and the
    /// warm-up pass counter is cleared. The caller drops the screen.
    pub fn destroy(&self) -> SplashDestroy {
        SplashDestroy {
            logos: true,
            sound: true,
            pass_counter: true,
        }
    }
}

/// The frame's time less a stamp: the signed 64-bit difference of the two
/// 32-bit clock words, converted to double.
fn elapsed(frame_ms: u32, stamp_ms: u32) -> f64 {
    (i64::from(frame_ms) - i64::from(stamp_ms)) as f64
}

/// The logo's place: the scale is `round(height/135)·0.125` (the steps in
/// double, rounded half to even, times the step in double, kept in single),
/// the logo's stored size times it, centred on the client.
pub fn logo_rect(viewport: Viewport, logo: Logo) -> LogoRect {
    let steps = round_double(f64::from(viewport.height as i32) * SCALE_PER_PIXEL);
    let scale = (steps as f64 * SCALE_STEP) as f32;
    let width = f32::from(logo.width) * scale;
    let height = f32::from(logo.height) * scale;
    LogoRect {
        x: (viewport.width as i32 as f32 - width) * 0.5,
        y: (viewport.height as i32 as f32 - height) * 0.5,
        width,
        height,
    }
}

/// A double converted to an integer with the processor's default rounding:
/// nearest, ties to even, and the integer-indefinite value for a NaN or an
/// out-of-range result.
fn round_double(value: f64) -> i64 {
    if value.is_nan()
        || value >= 9.223_372_036_854_775_808e18
        || value < -9.223_372_036_854_775_808e18
    {
        i64::MIN
    } else {
        value.round_ties_even() as i64
    }
}

/// A float converted the same way.
fn round_to_i64(value: f32) -> i64 {
    exa_ui::engine_math::round_i64(value)
}

/// The logo's grey level for a fade: the fade times 255, converted with the
/// processor's rounding, kept as its low byte.
fn grey(fade: f32) -> u8 {
    round_to_i64(fade * 255.0) as u8
}

/// The splash's running form.
#[derive(Resource, Default)]
pub struct SplashScreen(pub Option<Splash>);

impl SplashScreen {
    /// Starts the splash at `clock_ms`, if it is not already up.
    pub fn begin(&mut self, clock_ms: u32, logos: [Logo; 2]) {
        if self.0.is_none() {
            self.0 = Some(Splash::new(clock_ms, logos));
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

/// Whether the launcher shows the splash at start-up. Off unless the app
/// inserts it set: Exarust's launcher opens a view of its own, and the
/// splash covers its first five seconds.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SplashAtStart(pub bool);

/// Above the interface, the HUD and the fades; under the cursor.
const Z: i32 = 950;

#[derive(Component)]
struct SplashBackdrop;

#[derive(Component)]
struct SplashLogo;

/// The two logos' images, loaded when the splash starts.
#[derive(Resource)]
struct SplashImages([Handle<Image>; 2]);

/// The splash's plugin: with [`SplashAtStart`] set it starts the screen at
/// start-up, ticks it on the real clock, draws its backdrop and logos, and
/// opens the main menu screen at the hand-over.
pub struct SplashPlugin;

impl Plugin for SplashPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SplashScreen>()
            .init_resource::<SplashAtStart>()
            .add_systems(Startup, start)
            .add_systems(Update, tick_splash);
    }
}

/// The real clock in milliseconds, as the splash's 32-bit clock words.
pub(crate) fn clock_ms(time: &Time<Real>) -> u32 {
    time.elapsed().as_millis() as u32
}

fn start(
    mut commands: Commands,
    at_start: Res<SplashAtStart>,
    mut screen: ResMut<SplashScreen>,
    game: Option<Res<Game>>,
    mut images: ResMut<Assets<Image>>,
    time: Res<Time<Real>>,
) {
    if !at_start.0 {
        return;
    }
    let Some(game) = game else {
        return;
    };
    let Some(logos) = read_logos(&game) else {
        warn!("splash: the BME logos are missing");
        return;
    };
    let load = |name: &str| game.load_texture(name, ColorSpace::Srgb, None);
    let (first, second) = match (load(LOGO_1), load(LOGO_2)) {
        (Ok(first), Ok(second)) => (first, second),
        (Err(e), _) | (_, Err(e)) => {
            warn!("splash: {e}");
            return;
        }
    };
    commands.insert_resource(SplashImages([images.add(first), images.add(second)]));
    commands
        .spawn((
            SplashBackdrop,
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.0),
                height: percent(100.0),
                ..default()
            },
            BackgroundColor(Color::BLACK),
            GlobalZIndex(Z),
        ))
        .with_child((
            SplashLogo,
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            // The original maps the texture's first row to the quad's
            // bottom edge (v = 0 at y + h): the stored image is bottom-up.
            ImageNode {
                flip_y: true,
                ..default()
            },
            Visibility::Hidden,
        ));
    screen.begin(clock_ms(&time), logos);
}

type BackdropParts<'a> = (Entity, &'a mut BackgroundColor);
type LogoParts<'a> = (&'a mut Node, &'a mut ImageNode, &'a mut Visibility);

#[allow(clippy::too_many_arguments)]
fn tick_splash(
    mut commands: Commands,
    mut screen: ResMut<SplashScreen>,
    images: Option<Res<SplashImages>>,
    time: Res<Time<Real>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut backdrops: Query<BackdropParts, With<SplashBackdrop>>,
    mut logos: Query<LogoParts, With<SplashLogo>>,
    mut menu: Option<ResMut<crate::menu_screen::MenuScreen>>,
    mut flow: Option<ResMut<crate::flow::Flow>>,
    mut steps: Option<ResMut<crate::flow::FlowSteps>>,
    game: Option<Res<Game>>,
    mut bank: Option<ResMut<exa_audio::SoundBank>>,
    mut sources: Option<ResMut<Assets<AudioSource>>>,
) {
    let Some(splash) = screen.0.as_mut() else {
        return;
    };
    let Ok(window) = windows.single() else {
        return;
    };
    let viewport = Viewport {
        width: window.physical_width(),
        height: window.physical_height(),
    };
    // The frame's time and the stamp's clock are the same real clock here:
    // Bevy samples it once a frame.
    let now = clock_ms(&time);
    let frame = splash.tick(now, now, viewport, true);
    if frame.sound {
        if let (Some(game), Some(bank), Some(sources)) =
            (game.as_deref(), bank.as_mut(), sources.as_mut())
            && let Some(clip) = bank.get(game, sources, HAND_OVER_SOUND)
        {
            commands.spawn((
                AudioPlayer(clip),
                PlaybackSettings {
                    volume: Volume::Linear(HAND_OVER.volume),
                    speed: HAND_OVER.pitch,
                    ..default()
                },
            ));
        }
        debug!(
            "splash: {} at {} (priority {:#x})",
            HAND_OVER.name, HAND_OVER.volume, HAND_OVER.priority
        );
    }
    if frame.hand_over {
        // The hand-over callback: the title page is built and shown under
        // the splash.
        if let Some(flow) = flow.as_mut() {
            let after = flow.after_splash();
            for step in &after {
                debug!("game flow: {step:?}");
            }
            if let Some(steps) = steps.as_mut() {
                steps.steps.extend(after);
            }
        }
        if let Some(menu) = menu.as_mut() {
            menu.open();
        }
    }
    if frame.theme {
        let theme = menu_theme(flow.as_deref().is_some_and(|flow| flow.alternative_music));
        if let Some(steps) = steps.as_mut() {
            steps.steps.push(Step::Music {
                name: theme.name,
                volume: theme.volume,
                restart: true,
            });
        }
        debug!("splash: the menu theme starts");
    }
    let scale = window.scale_factor();
    for (_, mut colour) in &mut backdrops {
        colour.0 = Color::srgba_u8(0, 0, 0, frame.alpha);
    }
    for (mut node, mut image, mut visibility) in &mut logos {
        match (frame.logo, images.as_ref()) {
            (Some((index, rect, tint)), Some(images)) => {
                node.left = px(rect.x / scale);
                node.top = px(rect.y / scale);
                node.width = px(rect.width / scale);
                node.height = px(rect.height / scale);
                image.image = images.0[index].clone();
                image.color = Color::srgb_u8(tint, tint, tint);
                *visibility = Visibility::Inherited;
            }
            _ => *visibility = Visibility::Hidden,
        }
    }
    if frame.closed {
        debug!("splash: closed");
        if let Some(splash) = screen.0.as_ref() {
            splash.destroy();
        }
        screen.0 = None;
        for (entity, _) in &backdrops {
            commands.entity(entity).despawn();
        }
        commands.remove_resource::<SplashImages>();
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

    /// A tick where the frame's time and the stamp's clock agree.
    fn tick(splash: &mut Splash, now: u32, ready: bool) -> SplashFrame {
        splash.tick(now, now, VIEW, ready)
    }

    #[test]
    fn the_hand_over_sound_record_and_the_destroy_are_pinned() {
        assert_eq!(HAND_OVER.name, "BME Critters");
        assert_eq!(HAND_OVER.position, [0.0, 0.0, 0.0]);
        assert_eq!(HAND_OVER.volume, 0.2);
        assert_eq!(HAND_OVER.pitch, 1.0);
        assert_eq!(HAND_OVER.priority, 0x4060);
        assert_eq!(FORM_FLAGS, 0x1020);
        assert_eq!(BACKGROUND, [0, 0, 0, 255]);
        let splash = Splash::new(0, logos());
        assert_eq!(
            splash.destroy(),
            SplashDestroy {
                logos: true,
                sound: true,
                pass_counter: true
            }
        );
    }

    #[test]
    fn the_logos_are_centred_and_scaled_by_135ths() {
        // The shipped double is the nearest one to 1/135.
        assert_eq!(SCALE_PER_PIXEL, 1.0 / 135.0);
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
        // A client of 1215 is exactly 9 steps: scale 1.125.
        let nine = logo_rect(
            Viewport {
                width: 100,
                height: 1215,
            },
            logos()[0],
        );
        assert_eq!(nine.width, 576.0);
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
    fn the_splash_warms_up_a_second_then_fades_the_logo_in_from_black() {
        let mut splash = Splash::new(1000, logos());
        assert_eq!(splash.audio_fade, 0.0);
        assert_eq!(splash.phase(), Some(State::WarmUp));
        let frame = tick(&mut splash, 1000, true);
        assert_eq!(frame.warm_up, Some(0));
        assert_eq!(frame.alpha, 255);
        assert_eq!(frame.marker_fade, 1.0);
        // Strictly more than a second: the fade starts at 2001 ms.
        let frame = tick(&mut splash, 2000, true);
        assert_eq!(frame.warm_up, Some(1));
        assert_eq!(splash.phase(), Some(State::WarmUp));
        let frame = tick(&mut splash, 2001, true);
        assert_eq!(splash.phase(), Some(State::FadeIn));
        assert_eq!(splash.stamp_ms, 2001);
        assert_eq!(frame.warm_up, None);
        assert!(frame.logo.is_none());
        let frame = tick(&mut splash, 2101, true);
        // 100 ms at 1/600 a millisecond is 0.16666667; times 255 is 42.5,
        // which rounds half to even: grey 42. The backdrop stays opaque.
        let (index, _, grey) = frame.logo.unwrap();
        assert_eq!((index, grey), (0, 42));
        assert_eq!(frame.alpha, 255);
        // 600 ms is full white.
        let frame = tick(&mut splash, 2601, true);
        assert_eq!(frame.logo.unwrap().2, 255);
        // The first logo holds until 2000 ms after the fade started.
        assert_eq!(splash.phase(), Some(State::FadeIn));
        let frame = tick(&mut splash, 4002, true);
        assert_eq!(splash.phase(), Some(State::SecondLogo));
        // The first logo still draws on the tick that changes state; the
        // sound and the second logo are the next tick's.
        assert_eq!(frame.logo.unwrap().0, 0);
        assert!(!frame.sound);
        let frame = tick(&mut splash, 4003, true);
        assert!(frame.sound);
        assert_eq!(frame.audio_fade, 1.0);
        assert_eq!(frame.logo.unwrap().0, 1);
        assert_eq!(frame.logo.unwrap().2, 255);
    }

    #[test]
    fn a_stamp_after_the_frame_started_is_a_negative_time() {
        // The stamp's clock ran 3 ms past the frame's time: the next frame,
        // 1 ms later, is 2 ms before the stamp, not a wrapped 4294967294 ms
        // (which would clamp the fade to white). The fade is −2/600; times
        // 255 it is −0.85, which converts to −1, whose low byte is 255: the
        // original draws that one frame white too, by way of the byte.
        let mut splash = Splash::new(0, logos());
        splash.tick(1001, 1004, VIEW, true);
        assert_eq!(splash.stamp_ms, 1004);
        let frame = splash.tick(1002, 1005, VIEW, true);
        assert_eq!(frame.logo.unwrap().2, 255);
        // 1 ms before the stamp: −0.425 converts to 0, black.
        let frame = splash.tick(1003, 1005, VIEW, true);
        assert_eq!(frame.logo.unwrap().2, 0);
    }

    #[test]
    fn the_second_logo_holds_fades_to_black_and_settles() {
        let mut splash = Splash::new(0, logos());
        splash.state = 3;
        splash.start_ms = 5000;
        splash.stamp_ms = 5200;
        splash.frame = 7;
        // The hold keeps warming up, counting its passes.
        let frame = tick(&mut splash, 5200, true);
        assert_eq!(frame.warm_up, Some(7));
        assert_eq!(splash.phase(), Some(State::SecondHold));
        tick(&mut splash, 5400, true);
        assert_eq!(splash.frame, 9);
        let frame = tick(&mut splash, 5401, true);
        assert_eq!(splash.phase(), Some(State::FadeOut));
        assert_eq!(frame.warm_up, None);
        // 100 ms of the 300 ms fade: 1 − 1/3 in double, 0.6666667 in single,
        // grey 170 (169.99999 rounds to 170).
        let frame = tick(&mut splash, 5501, true);
        assert_eq!(frame.logo.unwrap().2, 170);
        assert_eq!(frame.alpha, 255);
        // 300 ms: black, and the settle starts.
        let frame = tick(&mut splash, 5701, true);
        assert_eq!(splash.phase(), Some(State::Settle));
        assert_eq!(frame.logo.unwrap().2, 0);
        assert_eq!(splash.start_ms, 5701);
        // The settle's pass runs every tick, and counts, until it reports
        // nothing left to build.
        let frame = tick(&mut splash, 5800, false);
        assert_eq!(frame.warm_up, Some(9));
        assert_eq!(splash.phase(), Some(State::Settle));
        let frame = tick(&mut splash, 5801, true);
        assert_eq!(frame.warm_up, Some(10));
        assert_eq!(splash.frame, 11);
        assert!(!frame.hand_over);
        assert_eq!(splash.phase(), Some(State::HandOver));
        let frame = tick(&mut splash, 5802, true);
        assert!(frame.hand_over);
        assert_eq!(splash.phase(), Some(State::MenuWait));
    }

    #[test]
    fn the_menu_waits_a_second_then_starts_its_theme() {
        let mut splash = Splash::new(0, logos());
        splash.state = 7;
        splash.start_ms = 5701;
        splash.audio_fade = 1.0;
        // The second counts from the fade-out's end.
        tick(&mut splash, 6701, true);
        assert_eq!(splash.phase(), Some(State::MenuWait));
        let frame = tick(&mut splash, 6702, true);
        assert_eq!(splash.phase(), Some(State::Menu));
        assert_eq!(frame.audio_fade, 0.0);
        assert_eq!(splash.busy, 0);
        let frame = tick(&mut splash, 6703, true);
        assert!(!frame.focus_menu);
        assert_eq!(splash.phase(), Some(State::MenuFade));
        assert_eq!(splash.busy, 0);
        let frame = tick(&mut splash, 6704, true);
        assert!(frame.focus_menu);
        assert!(frame.theme);
        assert_eq!(splash.phase(), Some(State::FadeToGame));
        assert_eq!(splash.stamp_ms, 6704);
        // A splash made without the theme request starts none.
        let mut quiet = Splash::new(0, logos());
        quiet.state = 9;
        quiet.theme_on_menu = false;
        assert!(!tick(&mut quiet, 1, true).theme);
    }

    #[test]
    fn the_last_fade_uncovers_the_menu_and_closes_the_screen() {
        let mut splash = Splash::new(0, logos());
        splash.state = 10;
        splash.stamp_ms = 10_000;
        let frame = tick(&mut splash, 10_100, true);
        // 1 − 0.1 = 0.9; the audio gain 0.1; the backdrop's alpha 229.5,
        // rounded half to even, 230; it is drawn on the next tick.
        assert_eq!(frame.marker_fade, 0.9);
        assert!((frame.audio_fade - 0.1).abs() < 1e-6);
        assert_eq!(frame.alpha, 255);
        assert_eq!(splash.alpha, 230);
        assert_eq!(tick(&mut splash, 10_100, true).alpha, 230);
        let frame = tick(&mut splash, 10_999, true);
        assert!(!frame.closed);
        // 0.001 × 255 = 0.255, which rounds to 0.
        assert_eq!(splash.alpha, 0);
        // At 11 000 the fade is exactly 0, under the epsilon: it closes.
        let frame = tick(&mut splash, 11_000, true);
        assert!(frame.closed);
        assert_eq!(splash.audio_fade, 1.0);
        assert_eq!(splash.marker_fade, 0.0);
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
            assert_eq!(State::from_word(word), Some(state));
        }
        // A word past the table only draws the backdrop.
        let mut stray = Splash::new(0, logos());
        stray.state = 11;
        let frame = tick(&mut stray, 5000, true);
        assert_eq!(frame.alpha, 255);
        assert!(frame.logo.is_none() && frame.warm_up.is_none());
        let mut screen = SplashScreen::default();
        assert!(!screen.is_active());
        screen.begin(10, logos());
        assert!(screen.is_active());
        screen.begin(20, logos());
        assert_eq!(screen.0.unwrap().start_ms, 10);
        screen.stop();
        assert!(!screen.is_active());
    }

    /// The installed logo sizes, when `EXANIMA_DIR` names the game; skipped
    /// otherwise.
    #[test]
    fn the_installed_logos_are_readable() {
        let Ok(root) = std::env::var("EXANIMA_DIR") else {
            return;
        };
        let Ok(data) = exa_assets::GameData::open(root) else {
            return;
        };
        let game = Game(std::sync::Arc::new(data));
        let logos = read_logos(&game).expect("the splash logos");
        assert!(logos[0].width > 0);
        assert!(logos[0].height > 0);
        assert!(logos[1].width > 0);
        assert!(logos[1].height > 0);
    }

    #[test]
    fn the_grey_level_rounds_to_even() {
        assert_eq!(grey(0.0), 0);
        assert_eq!(grey(1.0), 255);
        // 0.5·255 = 127.5, halfway: to even is 128.
        assert_eq!(grey(0.5), 128);
        // 0.1·255 = 25.5 in single precision: to even is 26.
        assert_eq!(grey(0.1), 26);
        // A NaN converts to the integer indefinite, whose low byte is 0.
        assert_eq!(grey(f32::NAN), 0);
        assert_eq!((600.0 * FADE_IN_RATE) as f32, 1.0);
    }
}

//! The loading screen: a dark page with what is being built and a progress
//! bar, shown while a level loads over several frames. The app owns the load;
//! it sets [`LoadingStatus`] each frame and removes it when the level is in,
//! which takes the screen down.
//!
//! While the screen is up, every frame is timed: a frame longer than
//! [`LONG_FRAME`] is logged as a warning with the stage it was spent in, and
//! each stage's time is logged at debug level when it ends. The frames of
//! the [`SETTLE`] time after the load (or after the start, when nothing
//! loads) are watched the same way, and their longest is logged.

use std::time::{Duration, Instant};

use bevy::camera::CameraOutputMode;
use bevy::prelude::*;
use bevy::render::render_resource::BlendState;
use bevy::window::PrimaryWindow;
use exa_assets::Game;
use exa_formats::rfi::Rfi;

use crate::font::{BASE_FONT, GameFont};
use crate::fps::text_image;

/// What the load is doing and how far it is (0–1). Present while loading.
#[derive(Resource, Clone, Debug, Default)]
pub struct LoadingStatus {
    pub title: String,
    pub stage: String,
    pub fraction: f32,
}

/// Window pixels per atlas pixel for the loading text.
const SCALE: f32 = 0.6;
const BACKGROUND: Color = Color::srgb(0.02, 0.02, 0.025);
const TEXT: Color = Color::srgb(0.72, 0.68, 0.6);
const BAR_TRACK: Color = Color::srgb(0.12, 0.11, 0.1);
const BAR_FILL: Color = Color::srgb(0.62, 0.48, 0.28);
const BAR_WIDTH: f32 = 420.0;
const BAR_HEIGHT: f32 = 6.0;

#[derive(Component)]
struct LoadingScreen;

#[derive(Component)]
struct LoadingText;

#[derive(Component)]
struct LoadingBar;

#[derive(Resource)]
struct LoadingFont(Option<GameFont>);

/// The loading text's parts, and the filter that keeps it apart from the bar.
type TextParts = (
    &'static mut DrawnText,
    &'static mut ImageNode,
    &'static mut Node,
);
type TextOnly = (With<LoadingText>, Without<LoadingBar>);

/// The text last drawn, so the image is rebuilt only when it changes.
#[derive(Component, Default)]
struct DrawnText(String);

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<FrameClock>()
        .add_systems(PreStartup, start_clock)
        .add_systems(First, time_frames)
        .add_systems(Last, end_main)
        .add_systems(
            Update,
            (
                show.run_if(resource_added::<LoadingStatus>),
                update.run_if(resource_exists::<LoadingStatus>),
                hide.run_if(resource_removed::<LoadingStatus>),
            )
                .chain(),
        );
}

/// A frame longer than this is logged, while loading and for [`SETTLE`]
/// after. The window answers the desktop only between frames: GNOME offers
/// to force-quit a window that has not answered for about five seconds,
/// and well before that a frame shows as a stall.
pub const LONG_FRAME: Duration = Duration::from_millis(250);

/// How long the frames after a load (or after the start, when nothing
/// loads) are watched: the first frames of a scene compile its pipelines
/// and upload its textures.
pub const SETTLE: Duration = Duration::from_secs(10);

/// The frames' timing: when the frame now running started and the load
/// stage it works on (none outside a load), and the totals of the stage, of
/// the whole load and of the frames after it.
#[derive(Resource, Default)]
struct FrameClock {
    startup: Option<Instant>,
    frame: Option<(Instant, Option<String>)>,
    stage: Option<Span>,
    load: Option<Span>,
    after: Option<Span>,
    /// When the main world's part of the frame ended; the rest of a frame
    /// waits for the render world and the window system.
    main_end: Option<Instant>,
}

/// Time spent in a stretch of frames.
struct Span {
    name: String,
    started: Instant,
    frames: u32,
    longest: Duration,
    longest_in: String,
}

impl Span {
    fn new(name: &str, started: Instant) -> Self {
        Self {
            name: name.to_string(),
            started,
            frames: 0,
            longest: Duration::ZERO,
            longest_in: String::new(),
        }
    }

    fn add(&mut self, frame: Duration, stage: &str) {
        self.frames += 1;
        if frame > self.longest {
            self.longest = frame;
            self.longest_in = stage.to_string();
        }
    }
}

/// The stage a status names, for the log.
fn stage_name(status: &LoadingStatus) -> &str {
    if status.stage.is_empty() {
        "starting"
    } else {
        &status.stage
    }
}

/// Notes when the startup systems begin, where a load starts.
fn start_clock(mut clock: ResMut<FrameClock>) {
    clock.startup = Some(Instant::now());
}

/// Notes when the main world's systems are done with the frame.
fn end_main(mut clock: ResMut<FrameClock>) {
    clock.main_end = Some(Instant::now());
}

/// Times the frame that just ended. A loading frame belongs to the stage
/// the status named when it started, as the status names what the next
/// step builds.
fn time_frames(mut clock: ResMut<FrameClock>, status: Option<Res<LoadingStatus>>) {
    let now = Instant::now();
    let clock = &mut *clock;
    let ended = match clock.frame.take() {
        Some(frame) => Some(frame),
        // The startup systems, where a load begins, run before the first
        // frame's `First`: they count as the first frame, the load's when
        // one began.
        None => clock.startup.take().map(|at| {
            if status.is_none() {
                clock.after = Some(Span::new("start", at));
            }
            (at, status.is_some().then(|| "starting".to_string()))
        }),
    };
    let main_end = clock.main_end.take();
    if let Some((started, stage)) = ended {
        let frame = now - started;
        let main = main_end.map_or(frame, |end| end.saturating_duration_since(started));
        match stage {
            Some(stage) => {
                if frame > LONG_FRAME {
                    warn!(
                        "loading: a {} ms frame in stage \"{stage}\" (main world {} ms)",
                        frame.as_millis(),
                        main.as_millis()
                    );
                }
                let load = clock.load.get_or_insert_with(|| Span::new("load", started));
                load.add(frame, &stage);
                let span = clock
                    .stage
                    .get_or_insert_with(|| Span::new(&stage, started));
                span.add(frame, &stage);
            }
            None => {
                if let Some(after) = clock.after.as_mut() {
                    after.add(frame, "");
                    if frame > LONG_FRAME {
                        warn!(
                            "a {} ms frame {:.1?} after the {} (main world {} ms)",
                            frame.as_millis(),
                            started - after.started,
                            after.name,
                            main.as_millis()
                        );
                    }
                }
            }
        }
    }
    // The watch ends after its time, or when a new load begins.
    if clock
        .after
        .as_ref()
        .is_some_and(|a| status.is_some() || now - a.started >= SETTLE)
        && let Some(after) = clock.after.take()
    {
        info!(
            "the {:.1?} after the {}: {} frames, longest {} ms",
            now - after.started,
            after.name,
            after.frames,
            after.longest.as_millis()
        );
    }
    let next = status.as_deref().map(stage_name);
    if clock
        .stage
        .as_ref()
        .is_some_and(|s| Some(s.name.as_str()) != next)
        && let Some(span) = clock.stage.take()
    {
        debug!(
            "loading stage \"{}\": {:.1?} over {} frames, longest {} ms",
            span.name,
            now - span.started,
            span.frames,
            span.longest.as_millis()
        );
    }
    if next.is_none()
        && let Some(load) = clock.load.take()
    {
        info!(
            "loading took {:.1?} over {} frames, longest {} ms (\"{}\")",
            now - load.started,
            load.frames,
            load.longest.as_millis(),
            load.longest_in
        );
        clock.after = Some(Span::new("load", now));
    }
    clock.frame = Some((now, next.map(str::to_string)));
}

fn show(
    mut commands: Commands,
    game: Option<Res<Game>>,
    cameras: Query<(), (With<LoadingScreen>, With<Camera>)>,
    mut screens: Query<&mut Visibility, (With<LoadingScreen>, Without<Camera>)>,
) {
    if !cameras.is_empty() {
        // A later load: the screen was hidden, not despawned.
        for mut visibility in &mut screens {
            *visibility = Visibility::Inherited;
        }
        return;
    }
    let font = game.and_then(|game| {
        game.0
            .texture(BASE_FONT)
            .and_then(|b| Rfi::parse(b).ok())
            .and_then(|rfi| GameFont::from_rfi(&rfi))
    });
    commands.insert_resource(LoadingFont(font));
    // Its own camera: the level's camera does not exist until the load
    // ends. It stays after the load and draws the rest of the interface
    // (the frame rate) over the game's view, without clearing it: as the
    // highest-ordered camera it is the interface's camera, and Bevy draws no
    // interface when that camera is off. The interface is drawn into the
    // camera's own texture, cleared to transparent black, so its colours
    // there are already multiplied by their opacity: it goes over the game's
    // view premultiplied. (Bevy's default for a later camera blends it as
    // straight alpha, which multiplied every translucent pixel twice.)
    commands.spawn((
        LoadingScreen,
        Camera2d,
        Camera {
            order: 10,
            clear_color: ClearColorConfig::None,
            output_mode: CameraOutputMode::Write {
                blend_state: Some(BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                clear_color: ClearColorConfig::Default,
            },
            ..default()
        },
    ));
    commands
        .spawn((
            LoadingScreen,
            Node {
                width: percent(100.0),
                height: percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                row_gap: px(18.0),
                ..default()
            },
            BackgroundColor(BACKGROUND),
            GlobalZIndex(100),
        ))
        .with_children(|page| {
            page.spawn((
                LoadingText,
                DrawnText::default(),
                Node::default(),
                ImageNode {
                    color: TEXT,
                    ..default()
                },
            ));
            page.spawn((
                Node {
                    width: px(BAR_WIDTH),
                    height: px(BAR_HEIGHT),
                    ..default()
                },
                BackgroundColor(BAR_TRACK),
            ))
            .with_child((
                LoadingBar,
                Node {
                    width: percent(0.0),
                    height: percent(100.0),
                    ..default()
                },
                BackgroundColor(BAR_FILL),
            ));
        });
}

fn update(
    status: Res<LoadingStatus>,
    font: Option<Res<LoadingFont>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut images: ResMut<Assets<Image>>,
    mut texts: Query<TextParts, TextOnly>,
    mut bars: Query<&mut Node, With<LoadingBar>>,
) {
    for mut bar in &mut bars {
        bar.width = percent(status.fraction.clamp(0.0, 1.0) * 100.0);
    }
    let Some(font) = font.as_ref().and_then(|f| f.0.as_ref()) else {
        return;
    };
    let line = if status.stage.is_empty() {
        status.title.clone()
    } else {
        format!("{}  -  {}", status.title, status.stage)
    };
    let unit = windows.single().map_or(1.0, |w| 1.0 / w.scale_factor());
    for (mut drawn, mut image, mut node) in &mut texts {
        if drawn.0 == line {
            continue;
        }
        let rendered = font.render(&line);
        node.width = px(rendered.width as f32 * SCALE * unit);
        node.height = px(rendered.height as f32 * SCALE * unit);
        image.image = images.add(text_image(rendered.width, rendered.height, &rendered.alpha));
        drawn.0 = line.clone();
    }
}

fn hide(mut screens: Query<&mut Visibility, (With<LoadingScreen>, Without<Camera>)>) {
    for mut visibility in &mut screens {
        *visibility = Visibility::Hidden;
    }
}

//! Grid view (`exarust grid`): one window split into four quadrants, each
//! showing its own game.
//!
//! An Exarust world holds one game (the level, the player, the rooms, the
//! GI and the audio are world resources), so each quadrant's game runs in
//! a child process of its own, in offscreen mode ([`crate::offscreen`]):
//! it draws into an image instead of a window and publishes each frame to a
//! shared-memory file. The host is a small Bevy app laid out as Bevy's
//! split-screen example: one camera per quadrant, its [`Viewport`] set from
//! the window's physical size and recomputed on every resize, and in each
//! camera's render layer a sprite showing the child's latest frame, with a
//! label and a status over it (interface nodes aimed at that camera).
//!
//! The host launches the children one at a time, each once the GPU has the
//! memory a game needs ([`GpuMemory`], from `nvidia-smi`) and the previous
//! one has shown its first frame. It forwards the keyboard, the mouse
//! buttons, the wheel, the mouse motion and the cursor (in the child's
//! pixels) to the active quadrant, the one last clicked, over the child's
//! standard input. A child that exits shows its exit status (and restarts
//! with `--restart`, or on Enter while its quadrant is active).
//! `exarust grid-swap <n> <scene>` asks a running host to replace one
//! quadrant's game.

use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::FileExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::camera::{ClearColorConfig, ScalingMode, Viewport};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use bevy::window::{CursorMoved, PrimaryWindow, WindowFocused, WindowResized};

use crate::offscreen::{ForwardedInput, FrameHeader, HEADER_LEN};

/// GPU memory a full game run needs before it is launched (MiB), as
/// `context/tools/gpu_run.sh` asks.
const DEFAULT_GPU_NEED_MB: u32 = 4600;
/// Seconds an exited child waits before `--restart` launches it again.
const RESTART_DELAY_S: f32 = 3.0;
/// Seconds after which a child that has shown no frame no longer holds
/// back the next launch.
const LAUNCH_TIMEOUT_S: f32 = 120.0;

/// One scripted host input (`--inject`), counted in host frames once every
/// quadrant shows its scene.
#[derive(Clone, Debug, PartialEq)]
enum Inject {
    /// Move the cursor to this window point (logical pixels).
    Cursor(Vec2),
    /// Move the cursor there and click the left button.
    Click(Vec2),
    Press(crate::Press),
}

#[derive(Clone, Debug)]
struct GridConfig {
    size: (u32, u32),
    game: Option<String>,
    scenes: Vec<String>,
    child_args: Vec<String>,
    child_fps: u32,
    gpu_need_mb: u32,
    restart: bool,
    settings: bool,
    screenshot: Option<PathBuf>,
    frames: u32,
    inject: Vec<(Inject, std::ops::Range<u32>)>,
    socket: Option<PathBuf>,
    log_dir: PathBuf,
}

fn grid_usage() -> AppExit {
    eprintln!(
        "usage: exarust grid [--size WxH] [--game <dir>] [--child-args \"...\"] [--child-fps n] \
         [--gpu-need MiB] [--restart] [--settings] [--screenshot out.png] [--frames n] \
         [--inject \"...\"] [--socket path] [--log-dir dir] <scene1> [scene2] [scene3] [scene4]\n\
         \x20      exarust grid-swap [--socket path] <1-4> <scene>\n\
         \x20      exarust grid-shot [--socket path] <file.png>"
    );
    AppExit::from_code(2)
}

/// The arguments a quadrant's scene runs with: `play <scene>`, or the
/// words of a scene that holds spaces as they stand (`"level exanima01
/// --all-rooms"`).
fn scene_args(scene: &str) -> Vec<String> {
    if scene.contains(char::is_whitespace) {
        scene.split_whitespace().map(str::to_string).collect()
    } else {
        vec!["play".into(), scene.into()]
    }
}

/// Reads `WHAT@from[-to];...`, where WHAT is `click:X,Y`, `cursor:X,Y`, a
/// `--press` key name or a mouse button (LMB, RMB, MMB).
fn parse_inject(list: &str) -> Option<Vec<(Inject, std::ops::Range<u32>)>> {
    list.split(';')
        .filter(|s| !s.trim().is_empty())
        .map(|entry| {
            let (what, frames) = entry.trim().rsplit_once('@')?;
            let (from, to) = match frames.split_once('-') {
                Some((from, to)) => (from.parse().ok()?, to.parse().ok()?),
                None => {
                    let at: u32 = frames.parse().ok()?;
                    (at, at.checked_add(1)?)
                }
            };
            let point = |p: &str| {
                let (x, y) = p.split_once(',')?;
                Some(Vec2::new(x.trim().parse().ok()?, y.trim().parse().ok()?))
            };
            let inject = if let Some(p) = what.strip_prefix("click:") {
                Inject::Click(point(p)?)
            } else if let Some(p) = what.strip_prefix("cursor:") {
                Inject::Cursor(point(p)?)
            } else {
                Inject::Press(
                    crate::button_named(what)
                        .map(crate::Press::Button)
                        .or_else(|| crate::key_named(what).map(crate::Press::Key))?,
                )
            };
            (to > from).then_some((inject, from..to))
        })
        .collect()
}

fn default_socket() -> PathBuf {
    std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
        .join("exarust-grid.sock")
}

fn parse_config(args: &[String]) -> Result<GridConfig, String> {
    let mut config = GridConfig {
        size: (2560, 1440),
        game: None,
        scenes: Vec::new(),
        child_args: Vec::new(),
        child_fps: 60,
        gpu_need_mb: DEFAULT_GPU_NEED_MB,
        restart: false,
        settings: false,
        screenshot: None,
        frames: 600,
        inject: Vec::new(),
        socket: Some(default_socket()),
        log_dir: std::env::temp_dir().join(format!("exarust-grid-{}", std::process::id())),
    };
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().cloned().ok_or(format!("{a}: missing value"));
        match a.as_str() {
            "--size" => {
                let v = value()?;
                config.size = v
                    .split_once('x')
                    .and_then(|(w, h)| Some((w.parse().ok()?, h.parse().ok()?)))
                    .filter(|&(w, h): &(u32, u32)| w >= 2 && h >= 2)
                    .ok_or(format!("--size: cannot read {v:?} (WxH)"))?;
            }
            "--game" => config.game = Some(value()?),
            "--child-args" => {
                config
                    .child_args
                    .extend(value()?.split_whitespace().map(str::to_string));
            }
            "--child-fps" => {
                let v = value()?;
                config.child_fps = v.parse().map_err(|_| format!("--child-fps: {v:?}"))?;
            }
            "--gpu-need" => {
                let v = value()?;
                config.gpu_need_mb = v.parse().map_err(|_| format!("--gpu-need: {v:?}"))?;
            }
            "--restart" => config.restart = true,
            "--settings" => config.settings = true,
            "--screenshot" => config.screenshot = Some(value()?.into()),
            "--frames" => {
                let v = value()?;
                config.frames = v.parse().map_err(|_| format!("--frames: {v:?}"))?;
            }
            "--inject" => {
                let v = value()?;
                config
                    .inject
                    .extend(parse_inject(&v).ok_or(format!("--inject: cannot read {v:?}"))?);
            }
            "--socket" => config.socket = Some(value()?.into()),
            "--no-socket" => config.socket = None,
            "--log-dir" => config.log_dir = value()?.into(),
            s if s.starts_with("--") => return Err(format!("unknown grid option {s}")),
            scene => config.scenes.push(scene.to_string()),
        }
    }
    if config.scenes.is_empty() || config.scenes.len() > 4 {
        return Err("give one to four scenes".into());
    }
    Ok(config)
}

/// `exarust grid ...`: the host.
pub fn run(args: &[String]) -> AppExit {
    let config = match parse_config(args) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("grid: {e}");
            return grid_usage();
        }
    };
    if let Err(e) = std::fs::create_dir_all(&config.log_dir) {
        eprintln!("grid: cannot create {}: {e}", config.log_dir.display());
        return AppExit::from_code(2);
    }
    let listener = config.socket.as_ref().and_then(|path| {
        if UnixStream::connect(path).is_ok() {
            eprintln!(
                "grid: another grid host answers on {}; grid-swap is off (pass --socket)",
                path.display()
            );
            return None;
        }
        let _ = std::fs::remove_file(path);
        match UnixListener::bind(path) {
            Ok(l) => {
                let _ = l.set_nonblocking(true);
                Some(l)
            }
            Err(e) => {
                eprintln!("grid: cannot listen on {}: {e}", path.display());
                None
            }
        }
    });
    let gpu = GpuMemory::start();
    let unattended = config.screenshot.is_some();
    let mut app = App::new();
    app.insert_resource(bevy::winit::WinitSettings {
        focused_mode: bevy::winit::UpdateMode::Continuous,
        unfocused_mode: bevy::winit::UpdateMode::Continuous,
    })
    .insert_resource(ClearColor(Color::BLACK))
    .add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "Exarust grid".into(),
            name: Some("exarust".into()),
            resolution: config.size.into(),
            ..default()
        }),
        ..default()
    }))
    .insert_resource(GridHost {
        config,
        quads: Vec::new(),
        active: 0,
        gpu,
        listener,
        held_keys: HashSet::new(),
        held_buttons: HashSet::new(),
        cursor: None,
        cursor_inside: false,
        ready_frames: 0,
        unattended,
        last_report: Instant::now(),
    })
    .add_systems(Startup, setup)
    .add_systems(
        Update,
        (
            set_camera_viewports,
            supervise,
            pull_frames,
            scripted_input,
            forward_input,
            serve_swaps,
            show_status,
            screenshot_when_ready,
        )
            .chain(),
    )
    .add_systems(Last, stop_children_on_exit);
    app.run()
}

/// `exarust grid-swap [--socket path] <n> <scene>`: asks the running host
/// to replace quadrant n's game.
pub fn swap(args: &[String]) -> AppExit {
    let (socket, rest) = match socket_and_rest(args) {
        Some(v) => v,
        None => return grid_usage(),
    };
    let [n, scene @ ..] = rest.as_slice() else {
        return grid_usage();
    };
    if scene.is_empty() || !matches!(n.as_str(), "1" | "2" | "3" | "4") {
        return grid_usage();
    }
    send_command(&socket, &format!("swap {n} {}", scene.join(" ")))
}

/// `exarust grid-shot [--socket path] <file.png>`: asks the running host
/// to save its composed window.
pub fn shot(args: &[String]) -> AppExit {
    let (socket, rest) = match socket_and_rest(args) {
        Some(v) => v,
        None => return grid_usage(),
    };
    let [file] = rest.as_slice() else {
        return grid_usage();
    };
    let path = std::path::absolute(file).unwrap_or_else(|_| file.into());
    send_command(&socket, &format!("shot {}", path.display()))
}

/// The `--socket` option and the other arguments of a host command.
fn socket_and_rest(args: &[String]) -> Option<(PathBuf, Vec<String>)> {
    let mut socket = default_socket();
    let mut rest = Vec::new();
    let mut it = args.iter();
    while let Some(a) = it.next() {
        if a == "--socket" {
            socket = it.next()?.into();
        } else {
            rest.push(a.clone());
        }
    }
    Some((socket, rest))
}

/// Sends one command line to the host and prints its answer.
fn send_command(socket: &PathBuf, line: &str) -> AppExit {
    let mut stream = match UnixStream::connect(socket) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("grid: no grid host on {}: {e}", socket.display());
            return AppExit::from_code(1);
        }
    };
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
    if let Err(e) = writeln!(stream, "{line}") {
        eprintln!("grid: {e}");
        return AppExit::from_code(1);
    }
    let mut reply = String::new();
    let _ = BufReader::new(stream).read_line(&mut reply);
    let reply = reply.trim();
    println!("{reply}");
    if reply.starts_with("ok") {
        AppExit::Success
    } else {
        AppExit::from_code(1)
    }
}

/// The GPU's memory as `nvidia-smi` last reported it, polled every second
/// on a thread of its own: total used and free, and each process's use.
#[derive(Clone, Default)]
struct GpuMemory(Arc<Mutex<Option<GpuReading>>>);

#[derive(Clone, Debug, Default)]
struct GpuReading {
    used_mb: u32,
    free_mb: u32,
    by_pid: Vec<(u32, u32)>,
}

impl GpuMemory {
    fn start() -> Self {
        let memory = Self::default();
        let shared = memory.0.clone();
        std::thread::spawn(move || {
            loop {
                let reading = read_gpu();
                let missing = reading.is_none();
                if let Ok(mut slot) = shared.lock() {
                    *slot = reading;
                }
                if missing {
                    // No nvidia-smi: launches go ahead unchecked.
                    return;
                }
                std::thread::sleep(Duration::from_secs(1));
            }
        });
        memory
    }

    fn get(&self) -> Option<GpuReading> {
        self.0.lock().ok()?.clone()
    }
}

/// The adapter `WGPU_ADAPTER_NAME` names (else the one with the most free
/// memory): its used and free MiB, and its processes' use.
fn read_gpu() -> Option<GpuReading> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=index,name,memory.used,memory.free",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let wanted = std::env::var("WGPU_ADAPTER_NAME")
        .ok()
        .map(|n| n.to_lowercase());
    let text = String::from_utf8_lossy(&out.stdout);
    let gpus: Vec<(String, String, u32, u32)> = text
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split(',').map(str::trim).collect();
            Some((
                f.first()?.to_string(),
                f.get(1)?.to_lowercase(),
                f.get(2)?.parse().ok()?,
                f.get(3)?.parse().ok()?,
            ))
        })
        .collect();
    let gpu = gpus
        .iter()
        .filter(|g| wanted.as_ref().is_none_or(|w| g.1.contains(w.as_str())))
        .max_by_key(|g| g.3)
        .or_else(|| gpus.iter().max_by_key(|g| g.3))?;
    let apps = Command::new("nvidia-smi")
        .args([
            "-i",
            &gpu.0,
            "--query-compute-apps=pid,used_memory",
            "--format=csv,noheader,nounits",
        ])
        .output()
        .ok();
    let by_pid = apps
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| {
                    let (pid, mb) = l.split_once(',')?;
                    Some((pid.trim().parse().ok()?, mb.trim().parse().ok()?))
                })
                .collect()
        })
        .unwrap_or_default();
    Some(GpuReading {
        used_mb: gpu.2,
        free_mb: gpu.3,
        by_pid,
    })
}

/// A running child: the process and the line writer feeding its input.
struct Running {
    child: Child,
    input: Sender<String>,
    launched: Instant,
}

/// One quadrant: its game, its frames and its entities.
struct Quadrant {
    /// The arguments its game runs with (after the offscreen ones).
    args: Vec<String>,
    /// The words shown in its label.
    title: String,
    run: Option<Running>,
    /// Why the last game ended, and when.
    exited: Option<(String, Instant)>,
    frame_path: PathBuf,
    log_path: PathBuf,
    reader: Option<File>,
    header: Option<FrameHeader>,
    scratch: Vec<u8>,
    image: Handle<Image>,
    image_size: UVec2,
    /// The child's frame counter and the time of the last rate sample.
    rate_from: Option<(u64, Instant)>,
    fps: f32,
    camera: Entity,
    sprite: Entity,
    border: Entity,
    label: Entity,
    status: Entity,
    status_text: String,
}

impl Quadrant {
    /// A game is up and has shown a frame past its loading screen.
    fn shows_scene(&self) -> bool {
        self.run.is_some()
            && self
                .header
                .is_some_and(|h| h.flags & FrameHeader::LOADING == 0)
    }

    /// A game launched and has not yet shown a frame (and is not stuck).
    fn starting(&self) -> bool {
        self.run.as_ref().is_some_and(|r| {
            self.header.is_none() && r.launched.elapsed().as_secs_f32() < LAUNCH_TIMEOUT_S
        })
    }

    fn send(&self, input: &ForwardedInput) {
        if let (Some(run), Some(line)) = (&self.run, input.encode()) {
            let _ = run.input.send(line);
        }
    }

    fn stop(&mut self, why: &str) {
        if let Some(mut run) = self.run.take() {
            let _ = run.child.kill();
            let _ = run.child.wait();
            self.exited = Some((why.to_string(), Instant::now()));
        }
        self.reader = None;
        self.header = None;
        self.rate_from = None;
        self.fps = 0.0;
    }
}

#[derive(Resource)]
struct GridHost {
    config: GridConfig,
    quads: Vec<Quadrant>,
    /// The quadrant that takes the keyboard and the mouse.
    active: usize,
    gpu: GpuMemory,
    listener: Option<UnixListener>,
    held_keys: HashSet<KeyCode>,
    held_buttons: HashSet<MouseButton>,
    /// The cursor over the window (physical pixels).
    cursor: Option<Vec2>,
    cursor_inside: bool,
    /// Host frames since every quadrant shows its scene.
    ready_frames: u32,
    unattended: bool,
    last_report: Instant,
}

impl Drop for GridHost {
    fn drop(&mut self) {
        for q in &mut self.quads {
            q.stop("host closed");
            let _ = std::fs::remove_file(&q.frame_path);
        }
        if let (Some(_), Some(path)) = (&self.listener, &self.config.socket) {
            let _ = std::fs::remove_file(path);
        }
    }
}

impl GridHost {
    /// The quadrant size: half the window on each axis.
    fn child_size(&self) -> UVec2 {
        UVec2::new(self.config.size.0 / 2, self.config.size.1 / 2)
    }

    /// Lets go of everything held in the active quadrant.
    fn release_held(&mut self) {
        let q = &self.quads[self.active];
        for code in self.held_keys.drain() {
            q.send(&ForwardedInput::Key {
                code,
                logical: Key::Unidentified(NativeKey::Unidentified),
                pressed: false,
                repeat: false,
                text: None,
            });
        }
        for button in self.held_buttons.drain() {
            q.send(&ForwardedInput::Button {
                button,
                pressed: false,
            });
        }
        if self.cursor_inside {
            q.send(&ForwardedInput::CursorLeft);
            self.cursor_inside = false;
        }
    }
}

/// The quadrant (0 top-left, 1 top-right, 2 bottom-left, 3 bottom-right)
/// of a camera, as the split-screen example's `CameraPosition`.
#[derive(Component)]
struct CameraPosition {
    pos: UVec2,
}

fn blank_image(size: UVec2) -> Image {
    let mut image = Image::new_fill(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[16, 16, 16, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::linear();
    image
}

fn fixed_projection(size: UVec2) -> Projection {
    Projection::Orthographic(OrthographicProjection {
        scaling_mode: ScalingMode::Fixed {
            width: size.x as f32,
            height: size.y as f32,
        },
        ..OrthographicProjection::default_2d()
    })
}

fn setup(
    mut commands: Commands,
    mut host: ResMut<GridHost>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let size = host.child_size();
    let quarter = windows.single().map(|w| w.physical_size() / 2).ok();
    let pid = std::process::id();
    let scenes = host.config.scenes.clone();
    for index in 0..4 {
        let scene = scenes.get(index).cloned();
        let pos = UVec2::new(index as u32 % 2, index as u32 / 2);
        let layer = RenderLayers::layer(index + 1);
        let camera = commands
            .spawn((
                Name::new(format!("_GridCamera{}", index + 1)),
                Camera2d,
                Camera {
                    order: index as isize,
                    clear_color: ClearColorConfig::Custom(Color::srgb(0.06, 0.06, 0.06)),
                    viewport: quarter.map(|q| Viewport {
                        physical_position: pos * q,
                        physical_size: q,
                        ..default()
                    }),
                    ..default()
                },
                fixed_projection(size),
                layer.clone(),
                CameraPosition { pos },
            ))
            .id();
        let image = images.add(blank_image(size));
        let sprite = commands
            .spawn((
                Sprite {
                    image: image.clone(),
                    custom_size: Some(size.as_vec2()),
                    ..default()
                },
                layer,
            ))
            .id();
        let mut label = Entity::PLACEHOLDER;
        let mut status = Entity::PLACEHOLDER;
        let border = commands
            .spawn((
                UiTargetCamera(camera),
                Node {
                    width: percent(100),
                    height: percent(100),
                    border: UiRect::all(px(2)),
                    flex_direction: FlexDirection::Column,
                    justify_content: JustifyContent::SpaceBetween,
                    ..default()
                },
                BorderColor::all(Color::NONE),
            ))
            .with_children(|root| {
                label = root
                    .spawn((
                        Text::new(""),
                        TextFont::from_font_size(16.0),
                        TextColor(Color::srgb(1.0, 0.85, 0.4)),
                        TextShadow::default(),
                        Node {
                            margin: px(8).all(),
                            ..default()
                        },
                    ))
                    .id();
                status = root
                    .spawn((
                        Text::new(""),
                        TextFont::from_font_size(22.0),
                        TextColor(Color::WHITE),
                        TextShadow::default(),
                        Node {
                            margin: px(8).all(),
                            align_self: AlignSelf::Center,
                            ..default()
                        },
                    ))
                    .id();
            })
            .id();
        let args = scene.as_deref().map(scene_args).unwrap_or_default();
        let log_path = host
            .config
            .log_dir
            .join(format!("quadrant-{}.log", index + 1));
        host.quads.push(Quadrant {
            title: args.join(" "),
            args,
            run: None,
            exited: scene.is_none().then(|| ("empty".into(), Instant::now())),
            frame_path: PathBuf::from(format!("/dev/shm/exarust-grid-{pid}-{}.rgba", index + 1)),
            log_path,
            reader: None,
            header: None,
            scratch: Vec::new(),
            image,
            image_size: size,
            rate_from: None,
            fps: 0.0,
            camera,
            sprite,
            border,
            label,
            status,
            status_text: String::new(),
        });
    }
    info!(
        "grid: {} scenes, quadrants of {}x{}, child logs in {}",
        scenes.len(),
        size.x,
        size.y,
        host.config.log_dir.display()
    );
}

/// The split-screen example's viewport system: a quarter of the window's
/// physical size per camera, at its place in the 2 × 2 grid.
fn set_camera_viewports(
    windows: Query<&Window>,
    mut window_resized_reader: MessageReader<WindowResized>,
    mut query: Query<(&CameraPosition, &mut Camera)>,
) {
    for window_resized in window_resized_reader.read() {
        let Ok(window) = windows.get(window_resized.window) else {
            continue;
        };
        let size = window.physical_size() / 2;
        for (camera_position, mut camera) in &mut query {
            camera.viewport = Some(Viewport {
                physical_position: camera_position.pos * size,
                physical_size: size,
                ..default()
            });
        }
    }
}

/// Launches a child for `q` with the host's offscreen arguments.
fn launch(config: &GridConfig, q: &mut Quadrant, size: UVec2) -> std::io::Result<Running> {
    let _ = std::fs::remove_file(&q.frame_path);
    let log = File::create(&q.log_path)?;
    let exe = std::env::current_exe()?;
    let mut command = Command::new(exe);
    if let Some(game) = &config.game {
        command.arg("--game").arg(game);
    }
    command
        .arg("--offscreen")
        .arg(&q.frame_path)
        .arg("--size")
        .arg(format!("{}x{}", size.x, size.y));
    if !config.settings {
        command.arg("--no-settings");
    }
    if config.child_fps > 0 && !config.child_args.iter().any(|a| a == "--max-fps") {
        command.arg("--max-fps").arg(config.child_fps.to_string());
    }
    command
        .args(&config.child_args)
        .args(&q.args)
        .env_remove("EXA_WINDOW_AT")
        .stdin(Stdio::piped())
        .stdout(log.try_clone()?)
        .stderr(log);
    let mut child = command.spawn()?;
    let mut stdin = child.stdin.take().ok_or(std::io::ErrorKind::BrokenPipe)?;
    let (input, lines) = channel::<String>();
    // Its own writer, so a child that stops reading never stalls the host.
    std::thread::spawn(move || {
        for line in lines {
            if writeln!(stdin, "{line}").is_err() {
                return;
            }
        }
    });
    Ok(Running {
        child,
        input,
        launched: Instant::now(),
    })
}

/// The last line of a child's log, for its exit status.
fn last_log_line(path: &PathBuf) -> Option<String> {
    let mut file = File::open(path).ok()?;
    let len = file.seek(SeekFrom::End(0)).ok()?;
    file.seek(SeekFrom::Start(len.saturating_sub(4096))).ok()?;
    let mut tail = String::new();
    file.read_to_string(&mut tail).ok()?;
    let line = tail.lines().rev().find(|l| !l.trim().is_empty())?;
    // Without the log's colour codes.
    let mut clean = String::new();
    let mut escape = false;
    for c in line.chars() {
        match (escape, c) {
            (false, '\u{1b}') => escape = true,
            (true, c) if c.is_ascii_alphabetic() => escape = false,
            (true, _) => {}
            (false, c) => clean.push(c),
        }
    }
    Some(clean.chars().take(140).collect())
}

/// Watches the children: notes exits, restarts, and launches waiting
/// quadrants one at a time once the GPU has room.
fn supervise(mut host: ResMut<GridHost>, keys: Res<ButtonInput<KeyCode>>) {
    let host = &mut *host;
    let size = host.child_size();
    let gpu = host.gpu.get();
    let need = host.config.gpu_need_mb;
    for (index, q) in host.quads.iter_mut().enumerate() {
        let Some(run) = &mut q.run else { continue };
        if let Ok(Some(status)) = run.child.try_wait() {
            let why = match status.code() {
                Some(code) => format!("exited with code {code}"),
                None => format!("ended by {status}"),
            };
            let tail = last_log_line(&q.log_path).unwrap_or_default();
            info!("grid: quadrant {} {why}: {tail}", index + 1);
            q.run = None;
            q.stop(&why);
            q.exited = Some((format!("{why}\n{tail}"), Instant::now()));
        }
    }
    // Enter restarts the active quadrant's ended game.
    let active = host.active;
    if keys.just_pressed(KeyCode::Enter)
        && host.quads[active].run.is_none()
        && !host.quads[active].args.is_empty()
    {
        host.quads[active].exited = None;
    }
    if host.config.restart {
        for q in &mut host.quads {
            if q.run.is_none()
                && !q.args.is_empty()
                && q.exited
                    .as_ref()
                    .is_some_and(|(_, at)| at.elapsed().as_secs_f32() >= RESTART_DELAY_S)
            {
                q.exited = None;
            }
        }
    }
    let starting = host.quads.iter().position(Quadrant::starting);
    // Memory the games still loading are yet to take.
    let outstanding: u32 = host
        .quads
        .iter()
        .filter(|q| q.run.is_some() && !q.shows_scene())
        .filter_map(|q| {
            let run = q.run.as_ref()?;
            if run.launched.elapsed().as_secs_f32() >= LAUNCH_TIMEOUT_S {
                return None;
            }
            let used = gpu.as_ref().and_then(|g| {
                g.by_pid
                    .iter()
                    .find(|(pid, _)| *pid == run.child.id())
                    .map(|p| p.1)
            });
            Some(need.saturating_sub(used.unwrap_or(0)))
        })
        .sum();
    let config = host.config.clone();
    for (index, q) in host.quads.iter_mut().enumerate() {
        if q.run.is_some() || q.exited.is_some() || q.args.is_empty() {
            continue;
        }
        if let Some(other) = starting {
            q.status_text = format!("waiting for quadrant {} to start", other + 1);
            break;
        }
        if let Some(g) = &gpu
            && need > 0
            && g.free_mb < need + outstanding
        {
            q.status_text = format!(
                "waiting for GPU memory: {} MiB free, {} MiB wanted",
                g.free_mb,
                need + outstanding
            );
            break;
        }
        match launch(&config, q, size) {
            Ok(run) => {
                info!(
                    "grid: quadrant {} launched (pid {}): {}",
                    index + 1,
                    run.child.id(),
                    q.title
                );
                q.run = Some(run);
            }
            Err(e) => {
                q.exited = Some((format!("cannot launch: {e}"), Instant::now()));
            }
        }
        // One launch a frame; the next waits for this one's first frame.
        break;
    }
}

/// Copies each child's newest frame into its quadrant's image.
fn pull_frames(
    mut host: ResMut<GridHost>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<&mut Sprite>,
    mut projections: Query<&mut Projection>,
) {
    let host = &mut *host;
    for q in &mut host.quads {
        if q.run.is_none() {
            continue;
        }
        if q.reader.is_none() {
            q.reader = File::open(&q.frame_path).ok();
        }
        let Some(file) = &q.reader else { continue };
        let mut bytes = [0; HEADER_LEN as usize];
        if file.read_exact_at(&mut bytes, 0).is_err() {
            continue;
        }
        let Some(header) = FrameHeader::from_bytes(&bytes) else {
            continue;
        };
        if header.seq % 2 == 1
            || q.header.is_some_and(|h| h.frame == header.frame)
            || header.width == 0
            || header.height == 0
        {
            continue;
        }
        q.scratch.resize(header.pixel_len(), 0);
        if file.read_exact_at(&mut q.scratch, HEADER_LEN).is_err()
            || file.read_exact_at(&mut bytes, 0).is_err()
            || FrameHeader::from_bytes(&bytes).is_none_or(|h| h.seq != header.seq)
        {
            // Torn: the child wrote a newer frame meanwhile.
            continue;
        }
        let size = UVec2::new(header.width, header.height);
        if size != q.image_size {
            q.image = images.add(blank_image(size));
            q.image_size = size;
            if let Ok(mut sprite) = sprites.get_mut(q.sprite) {
                sprite.image = q.image.clone();
                sprite.custom_size = Some(size.as_vec2());
            }
            if let Ok(mut projection) = projections.get_mut(q.camera) {
                *projection = fixed_projection(size);
            }
        }
        if let Some(mut image) = images.get_mut(&q.image) {
            match &mut image.data {
                Some(data) => std::mem::swap(data, &mut q.scratch),
                None => image.data = Some(std::mem::take(&mut q.scratch)),
            }
        }
        let now = Instant::now();
        match q.rate_from {
            None => q.rate_from = Some((header.frame, now)),
            Some((from, at)) => {
                let secs = now.duration_since(at).as_secs_f32();
                if secs >= 1.0 {
                    q.fps = (header.frame - from) as f32 / secs;
                    q.rate_from = Some((header.frame, now));
                }
            }
        }
        q.header = Some(header);
    }
}

/// The quadrant whose viewport holds a window point (physical pixels), and
/// the point in that child's pixels.
fn quadrant_at(
    point: Vec2,
    cameras: &Query<(&Camera, &CameraPosition)>,
    sizes: &[UVec2],
) -> Option<(usize, Vec2)> {
    cameras.iter().find_map(|(camera, position)| {
        let viewport = camera.viewport.as_ref()?;
        let origin = viewport.physical_position.as_vec2();
        let extent = viewport.physical_size.as_vec2();
        let local = point - origin;
        if local.x < 0.0 || local.y < 0.0 || local.x >= extent.x || local.y >= extent.y {
            return None;
        }
        let index = (position.pos.x + position.pos.y * 2) as usize;
        let child = sizes.get(index)?.as_vec2();
        Some((index, local / extent * child))
    })
}

/// `--inject`: scripted host input, written as the window's own messages
/// so that it takes the same route as the user's.
#[allow(clippy::too_many_arguments)]
fn scripted_input(
    host: Res<GridHost>,
    windows: Query<(Entity, &Window), With<PrimaryWindow>>,
    mut moved: MessageWriter<CursorMoved>,
    mut buttons: MessageWriter<MouseButtonInput>,
    mut keys: MessageWriter<KeyboardInput>,
) {
    if host.config.inject.is_empty() {
        return;
    }
    let Ok((window, _)) = windows.single() else {
        return;
    };
    let frame = host.ready_frames;
    for (inject, over) in &host.config.inject {
        let (start, end) = (frame == over.start, frame == over.end);
        if !start && !end {
            continue;
        }
        let state = if start {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };
        match inject {
            Inject::Cursor(p) | Inject::Click(p) => {
                if start {
                    info!("grid: injected cursor at {p}");
                    moved.write(CursorMoved {
                        window,
                        position: *p,
                        delta: None,
                    });
                }
                if matches!(inject, Inject::Click(_)) {
                    buttons.write(MouseButtonInput {
                        button: MouseButton::Left,
                        state,
                        window,
                    });
                }
            }
            Inject::Press(crate::Press::Button(button)) => {
                buttons.write(MouseButtonInput {
                    button: *button,
                    state,
                    window,
                });
            }
            Inject::Press(crate::Press::Key(code)) => {
                if start {
                    info!("grid: injected key {code:?}");
                }
                keys.write(KeyboardInput {
                    key_code: *code,
                    logical_key: Key::Unidentified(NativeKey::Unidentified),
                    state,
                    text: None,
                    repeat: false,
                    window,
                });
            }
        }
    }
}

/// Sends the window's input to the active quadrant: a button press makes
/// the quadrant under the cursor the active one first.
#[allow(clippy::too_many_arguments)]
fn forward_input(
    mut host: ResMut<GridHost>,
    windows: Query<&Window, With<PrimaryWindow>>,
    cameras: Query<(&Camera, &CameraPosition)>,
    mut moved: MessageReader<CursorMoved>,
    mut buttons: MessageReader<MouseButtonInput>,
    mut keys: MessageReader<KeyboardInput>,
    mut wheel: MessageReader<MouseWheel>,
    mut motion: MessageReader<MouseMotion>,
    mut focus: MessageReader<WindowFocused>,
) {
    let host = &mut *host;
    let scale = windows.single().map(|w| w.scale_factor()).unwrap_or(1.0);
    let sizes: Vec<UVec2> = host.quads.iter().map(|q| q.image_size).collect();
    let mut cursor_changed = false;
    for m in moved.read() {
        host.cursor = Some(m.position * scale);
        cursor_changed = true;
    }
    for f in focus.read() {
        if !f.focused {
            host.release_held();
        }
    }
    for b in buttons.read() {
        let pressed = b.state == ButtonState::Pressed;
        if pressed
            && host.held_buttons.is_empty()
            && let Some((index, _)) = host.cursor.and_then(|c| quadrant_at(c, &cameras, &sizes))
            && index != host.active
        {
            host.release_held();
            host.active = index;
            info!("grid: quadrant {} is active", index + 1);
            cursor_changed = true;
        }
        if cursor_changed {
            send_cursor(host, &cameras, &sizes);
            cursor_changed = false;
        }
        if pressed {
            host.held_buttons.insert(b.button);
        } else if !host.held_buttons.remove(&b.button) {
            continue;
        }
        host.quads[host.active].send(&ForwardedInput::Button {
            button: b.button,
            pressed,
        });
    }
    if cursor_changed {
        send_cursor(host, &cameras, &sizes);
    }
    let q = &host.quads[host.active];
    for k in keys.read() {
        let pressed = k.state == ButtonState::Pressed;
        if pressed {
            host.held_keys.insert(k.key_code);
        } else {
            host.held_keys.remove(&k.key_code);
        }
        q.send(&ForwardedInput::Key {
            code: k.key_code,
            logical: k.logical_key.clone(),
            pressed,
            repeat: k.repeat,
            text: k.text.as_ref().map(ToString::to_string),
        });
    }
    for w in wheel.read() {
        q.send(&ForwardedInput::Wheel {
            line: w.unit == MouseScrollUnit::Line,
            x: w.x,
            y: w.y,
        });
    }
    for m in motion.read() {
        if host.cursor_inside || !host.held_buttons.is_empty() {
            q.send(&ForwardedInput::Motion(m.delta));
        }
    }
}

/// Sends the cursor to the active quadrant in its pixels, or that it left.
fn send_cursor(host: &mut GridHost, cameras: &Query<(&Camera, &CameraPosition)>, sizes: &[UVec2]) {
    let q = &host.quads[host.active];
    match host.cursor.and_then(|c| quadrant_at(c, cameras, sizes)) {
        Some((index, at)) if index == host.active => {
            q.send(&ForwardedInput::Cursor(at));
            host.cursor_inside = true;
        }
        _ if host.cursor_inside => {
            q.send(&ForwardedInput::CursorLeft);
            host.cursor_inside = false;
        }
        _ => {}
    }
}

/// Answers `grid-swap` and `grid-shot`: `swap <n> <scene>` replaces
/// quadrant n's game, `shot <file>` saves the composed window.
fn serve_swaps(mut commands: Commands, mut host: ResMut<GridHost>) {
    let Some(listener) = &host.listener else {
        return;
    };
    let Ok((stream, _)) = listener.accept() else {
        return;
    };
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
    let mut line = String::new();
    let mut reader = BufReader::new(&stream);
    let _ = reader.read_line(&mut line);
    let mut words = line.split_whitespace();
    let reply = match (
        words.next(),
        words.next().and_then(|n| n.parse::<usize>().ok()),
    ) {
        (Some("swap"), Some(n @ 1..=4)) => {
            let scene: Vec<&str> = words.collect();
            if scene.is_empty() {
                "error: no scene".to_string()
            } else {
                let scene = scene.join(" ");
                let q = &mut host.quads[n - 1];
                q.stop("swapped");
                q.args = scene_args(&scene);
                q.title = q.args.join(" ");
                q.exited = None;
                info!("grid: quadrant {n} swapped to {}", q.title);
                format!("ok: quadrant {n} now runs {}", q.title)
            }
        }
        (Some("shot"), _) if line.trim().len() > 5 => {
            let path = PathBuf::from(line.trim()[5..].trim());
            info!("grid: screenshot to {}", path.display());
            commands
                .spawn(Screenshot::primary_window())
                .observe(save_to_disk(path.clone()));
            format!("ok: saving {}", path.display())
        }
        _ => format!("error: cannot read {:?}", line.trim()),
    };
    let mut stream = &stream;
    let _ = writeln!(stream, "{reply}");
}

/// Labels, status lines and the active quadrant's border; a log line of
/// each quadrant's frame rate and the GPU's memory every five seconds.
fn show_status(
    mut host: ResMut<GridHost>,
    mut texts: Query<&mut Text>,
    mut borders: Query<&mut BorderColor>,
) {
    let host = &mut *host;
    let active = host.active;
    for (index, q) in host.quads.iter_mut().enumerate() {
        let state = if q.args.is_empty() {
            "empty".to_string()
        } else if let Some((why, at)) = &q.exited {
            let mut s = why.clone();
            if host.config.restart {
                let left = (RESTART_DELAY_S - at.elapsed().as_secs_f32()).max(0.0);
                s.push_str(&format!("\nrestarting in {left:.0} s"));
            } else if index == active {
                s.push_str("\nEnter restarts it");
            } else {
                s.push_str("\nclick here, then Enter restarts it");
            }
            s
        } else if q.run.is_none() {
            q.status_text.clone()
        } else if q.header.is_none() {
            "starting".to_string()
        } else {
            String::new()
        };
        let loading = q.run.is_some()
            && q.header
                .is_some_and(|h| h.flags & FrameHeader::LOADING != 0);
        let label = format!(
            "{}  {}{}{}",
            index + 1,
            q.title,
            if q.fps > 0.0 {
                format!("  {:.0} fps", q.fps)
            } else {
                String::new()
            },
            if loading { "  loading" } else { "" }
        );
        if let Ok(mut text) = texts.get_mut(q.label)
            && text.0 != label
        {
            text.0 = label;
        }
        if let Ok(mut text) = texts.get_mut(q.status)
            && text.0 != state
        {
            text.0 = state;
        }
        if let Ok(mut border) = borders.get_mut(q.border) {
            let color = if index == active {
                Color::srgb(1.0, 0.75, 0.2)
            } else {
                Color::NONE
            };
            if border.top != color {
                *border = BorderColor::all(color);
            }
        }
    }
    if host.last_report.elapsed().as_secs_f32() >= 5.0 {
        host.last_report = Instant::now();
        let rates: Vec<String> = host
            .quads
            .iter()
            .enumerate()
            .map(|(i, q)| format!("q{} {:.1}", i + 1, q.fps))
            .collect();
        let memory = host.gpu.get().map(|g| {
            let ours: Vec<String> = host
                .quads
                .iter()
                .filter_map(|q| {
                    let pid = q.run.as_ref()?.child.id();
                    let mb = g.by_pid.iter().find(|p| p.0 == pid)?.1;
                    Some(format!("{pid}:{mb}"))
                })
                .collect();
            format!(
                "GPU {} MiB used, {} MiB free; children {}",
                g.used_mb,
                g.free_mb,
                if ours.is_empty() {
                    "-".to_string()
                } else {
                    ours.join(" ")
                }
            )
        });
        info!(
            "grid: fps {}; {}",
            rates.join(" "),
            memory.unwrap_or_else(|| "no nvidia-smi".into())
        );
    }
}

/// Counts the frames since every quadrant shows its scene (or its game
/// has ended) and, with
/// `--screenshot`, saves the composed window after `--frames` of them and
/// exits.
fn screenshot_when_ready(
    mut commands: Commands,
    mut host: ResMut<GridHost>,
    mut exit: MessageWriter<AppExit>,
) {
    let ready = host
        .quads
        .iter()
        .all(|q| q.args.is_empty() || q.shows_scene() || (q.run.is_none() && q.exited.is_some()));
    if !ready {
        return;
    }
    if host.ready_frames == 0 {
        info!("grid: every quadrant shows its scene");
    }
    host.ready_frames += 1;
    if !host.unattended {
        return;
    }
    let Some(path) = host.config.screenshot.clone() else {
        return;
    };
    if host.ready_frames == host.config.frames {
        let rates: Vec<String> = host
            .quads
            .iter()
            .enumerate()
            .map(|(i, q)| format!("q{} {:.1}", i + 1, q.fps))
            .collect();
        info!(
            "grid: screenshot to {} ({})",
            path.display(),
            rates.join(" ")
        );
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if host.ready_frames == host.config.frames + 30 {
        exit.write(AppExit::Success);
    }
}

/// Ends every child when the host exits.
fn stop_children_on_exit(mut exits: MessageReader<AppExit>, mut host: ResMut<GridHost>) {
    if exits.read().next().is_none() {
        return;
    }
    for q in &mut host.quads {
        q.stop("host closed");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scenes_become_child_arguments() {
        assert_eq!(scene_args("arena"), ["play", "arena"]);
        assert_eq!(
            scene_args("level exanima01  --all-rooms"),
            ["level", "exanima01", "--all-rooms"]
        );
    }

    #[test]
    fn grid_options_are_read() {
        let args: Vec<String> = [
            "--size",
            "1920x1080",
            "--child-args",
            "--no-gi --opponent",
            "--restart",
            "exanima01",
            "level arenao3",
        ]
        .map(String::from)
        .into();
        let config = parse_config(&args).unwrap();
        assert_eq!(config.size, (1920, 1080));
        assert_eq!(config.child_args, ["--no-gi", "--opponent"]);
        assert!(config.restart);
        assert_eq!(config.scenes, ["exanima01", "level arenao3"]);
        assert!(parse_config(&[]).is_err());
        let five: Vec<String> = ["a", "b", "c", "d", "e"].map(String::from).into();
        assert!(parse_config(&five).is_err());
        assert!(parse_config(&["--size".into(), "big".into(), "a".into()]).is_err());
    }

    #[test]
    fn injections_are_read() {
        assert_eq!(
            parse_inject("click:1900,300@10; F3@20-25;cursor:5,6@30;RMB@40"),
            Some(vec![
                (Inject::Click(Vec2::new(1900.0, 300.0)), 10..11),
                (Inject::Press(crate::Press::Key(KeyCode::F3)), 20..25),
                (Inject::Cursor(Vec2::new(5.0, 6.0)), 30..31),
                (
                    Inject::Press(crate::Press::Button(MouseButton::Right)),
                    40..41
                ),
            ])
        );
        for bad in ["click:1@2", "F3", "Nope@3", "F3@5-5"] {
            assert_eq!(parse_inject(bad), None, "{bad}");
        }
    }
}

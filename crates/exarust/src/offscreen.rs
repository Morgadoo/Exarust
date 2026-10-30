//! Offscreen child mode (`--offscreen <file>`), one quadrant of the grid
//! view (`exarust grid`, [`crate::grid`]).
//!
//! The game runs as it always does, but without an OS window: winit is
//! left out and Bevy's schedule runner drives the frames, so nothing is
//! presented and no frame waits on a compositor. The primary [`Window`]
//! entity still exists, at the quadrant's size, so every system that reads
//! the window (its size, the cursor, the focus) sees an ordinary one. Each
//! camera that would draw to that window draws into one image instead
//! ([`OffscreenTarget`]), the interface camera among them, so the scene,
//! the HUD and the menus all land in it. The image is read back every
//! frame and written to a shared-memory file the host maps into its
//! quadrant ([`FrameHeader`]).
//!
//! Input comes from the host over the child's standard input, one event a
//! line ([`ForwardedInput`]): it is injected as the window's own keyboard,
//! mouse button, wheel, cursor and motion messages, so the game reads it as
//! if winit had delivered it. Standard input closing (the host gone) ends
//! the game.

use std::fs::File;
use std::io::BufRead;
use std::os::unix::fs::FileExt;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};

use bevy::asset::RenderAssetUsages;
use bevy::camera::{CameraUpdateSystems, RenderTarget, ShadowLodOrigin};
use bevy::input::ButtonState;
use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
use bevy::input::mouse::{MouseButtonInput, MouseMotion, MouseScrollUnit, MouseWheel};
use bevy::input::touch::TouchPhase;
use bevy::math::DVec2;
use bevy::prelude::*;
use bevy::reflect::enums::{DynamicEnum, DynamicVariant};
use bevy::reflect::{FromReflect, ReflectRef};
use bevy::render::gpu_readback::{Readback, ReadbackComplete};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use bevy::ui::IsDefaultUiCamera;
use bevy::window::{CursorLeft, CursorMoved, PrimaryWindow, WindowRef};

/// Bytes before the pixels in the frame file.
pub const HEADER_LEN: u64 = 64;
/// The frame file's first eight bytes.
pub const MAGIC: &[u8; 8] = b"EXAGRID1";

/// The frame file's header: `MAGIC`, a sequence number that is odd while
/// the pixels are being written, the frame's size, its counter and its
/// [`flags`](FrameHeader::LOADING). The
/// pixels follow at [`HEADER_LEN`], RGBA8 (sRGB), rows top to bottom with
/// no padding.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameHeader {
    pub seq: u64,
    pub width: u32,
    pub height: u32,
    pub frame: u64,
    pub flags: u32,
}

impl FrameHeader {
    /// The frame shows the loading screen.
    pub const LOADING: u32 = 1;

    pub fn to_bytes(self) -> [u8; HEADER_LEN as usize] {
        let mut b = [0; HEADER_LEN as usize];
        b[..8].copy_from_slice(MAGIC);
        b[8..16].copy_from_slice(&self.seq.to_le_bytes());
        b[16..20].copy_from_slice(&self.width.to_le_bytes());
        b[20..24].copy_from_slice(&self.height.to_le_bytes());
        b[24..32].copy_from_slice(&self.frame.to_le_bytes());
        b[32..36].copy_from_slice(&self.flags.to_le_bytes());
        b
    }

    /// The header in `b`, if it starts with [`MAGIC`].
    pub fn from_bytes(b: &[u8]) -> Option<Self> {
        if b.len() < 36 || &b[..8] != MAGIC {
            return None;
        }
        let u64_at = |at: usize| u64::from_le_bytes(b[at..at + 8].try_into().unwrap_or_default());
        let u32_at = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap_or_default());
        Some(Self {
            seq: u64_at(8),
            width: u32_at(16),
            height: u32_at(20),
            frame: u64_at(24),
            flags: u32_at(32),
        })
    }

    /// The pixel bytes the header's size names.
    pub fn pixel_len(&self) -> usize {
        self.width as usize * self.height as usize * 4
    }
}

/// One input event the host forwards, in the child's pixels.
#[derive(Clone, Debug, PartialEq)]
pub enum ForwardedInput {
    Key {
        code: KeyCode,
        logical: Key,
        pressed: bool,
        repeat: bool,
        text: Option<String>,
    },
    Button {
        button: MouseButton,
        pressed: bool,
    },
    /// The cursor at this point of the child's window (physical pixels).
    Cursor(Vec2),
    /// The cursor left the child's quadrant.
    CursorLeft,
    Motion(Vec2),
    Wheel {
        line: bool,
        x: f32,
        y: f32,
    },
}

/// The name of a field-less enum variant (a [`KeyCode`], a named [`Key`],
/// a [`MouseButton`]).
fn unit_variant_name<T: Reflect>(value: &T) -> Option<String> {
    match value.reflect_ref() {
        ReflectRef::Enum(e) if e.field_len() == 0 => Some(e.variant_name().to_string()),
        _ => None,
    }
}

/// The field-less variant called `name`.
fn unit_variant<T: FromReflect>(name: &str) -> Option<T> {
    T::from_reflect(&DynamicEnum::new(name.to_string(), DynamicVariant::Unit))
}

fn hex(text: &str) -> String {
    text.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(text: &str) -> Option<String> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

impl ForwardedInput {
    /// The event as one line of the host-to-child protocol (no newline),
    /// or `None` for keys the protocol cannot name.
    pub fn encode(&self) -> Option<String> {
        let flag = |b: bool| if b { "1" } else { "0" };
        Some(match self {
            Self::Key {
                code,
                logical,
                pressed,
                repeat,
                text,
            } => {
                let logical = match logical {
                    Key::Character(c) => format!("c:{}", hex(c)),
                    other => match unit_variant_name(other) {
                        Some(name) => format!("n:{name}"),
                        None => "u".into(),
                    },
                };
                let text = text.as_deref().map(hex).unwrap_or_else(|| "-".into());
                format!(
                    "k {} {} {} {logical} {text}",
                    unit_variant_name(code)?,
                    flag(*pressed),
                    flag(*repeat)
                )
            }
            Self::Button { button, pressed } => {
                let name = match button {
                    MouseButton::Other(n) => format!("o{n}"),
                    other => unit_variant_name(other)?,
                };
                format!("b {name} {}", flag(*pressed))
            }
            Self::Cursor(p) => format!("c {} {}", p.x, p.y),
            Self::CursorLeft => "l".into(),
            Self::Motion(d) => format!("m {} {}", d.x, d.y),
            Self::Wheel { line, x, y } => format!("w {} {x} {y}", flag(*line)),
        })
    }

    /// Reads one protocol line.
    pub fn decode(line: &str) -> Option<Self> {
        let mut it = line.split_ascii_whitespace();
        let kind = it.next()?;
        let mut next = || it.next();
        let flag = |s: Option<&str>| match s? {
            "1" => Some(true),
            "0" => Some(false),
            _ => None,
        };
        Some(match kind {
            "k" => {
                let code = unit_variant::<KeyCode>(next()?)?;
                let pressed = flag(next())?;
                let repeat = flag(next())?;
                let logical = match next()? {
                    "u" => Key::Unidentified(NativeKey::Unidentified),
                    l => {
                        if let Some(c) = l.strip_prefix("c:") {
                            Key::Character(unhex(c)?.into())
                        } else {
                            unit_variant::<Key>(l.strip_prefix("n:")?)?
                        }
                    }
                };
                let text = match next()? {
                    "-" => None,
                    t => Some(unhex(t)?),
                };
                Self::Key {
                    code,
                    logical,
                    pressed,
                    repeat,
                    text,
                }
            }
            "b" => {
                let name = next()?;
                let button = match name.strip_prefix('o') {
                    Some(n) if n.chars().all(|c| c.is_ascii_digit()) && !n.is_empty() => {
                        MouseButton::Other(n.parse().ok()?)
                    }
                    _ => unit_variant::<MouseButton>(name)?,
                };
                Self::Button {
                    button,
                    pressed: flag(next())?,
                }
            }
            "c" => Self::Cursor(Vec2::new(next()?.parse().ok()?, next()?.parse().ok()?)),
            "l" => Self::CursorLeft,
            "m" => Self::Motion(Vec2::new(next()?.parse().ok()?, next()?.parse().ok()?)),
            "w" => Self::Wheel {
                line: flag(next())?,
                x: next()?.parse().ok()?,
                y: next()?.parse().ok()?,
            },
            _ => return None,
        })
    }
}

/// The offscreen run: the frame file and the image the window's cameras
/// draw into instead of the window.
pub struct OffscreenPlugin {
    pub path: PathBuf,
    pub size: UVec2,
}

/// The image every camera that would draw to the primary window draws
/// into.
#[derive(Resource, Clone)]
pub struct OffscreenTarget {
    pub image: Handle<Image>,
    pub size: UVec2,
}

/// The frame file and what has been written to it.
#[derive(Resource)]
struct FrameFile {
    file: File,
    seq: u64,
    frame: u64,
    sized: bool,
    pixels: Vec<u8>,
}

/// Lines from the host, read on their own thread.
#[derive(Resource)]
struct InputLines(Mutex<Receiver<Option<ForwardedInput>>>);

impl Plugin for OffscreenPlugin {
    fn build(&self, app: &mut App) {
        let file = match File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
        {
            Ok(f) => f,
            Err(e) => {
                error!("offscreen: cannot open {}: {e}", self.path.display());
                std::process::exit(2);
            }
        };
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let stdin = std::io::stdin();
            for line in stdin.lock().lines() {
                let Ok(line) = line else { break };
                match ForwardedInput::decode(&line) {
                    Some(input) => {
                        if send.send(Some(input)).is_err() {
                            return;
                        }
                    }
                    None => eprintln!("offscreen: cannot read input line {line:?}"),
                }
            }
            let _ = send.send(None);
        });
        let size = self.size;
        app.insert_resource(FrameFile {
            file,
            seq: 0,
            frame: 0,
            sized: false,
            pixels: Vec::new(),
        })
        .insert_resource(InputLines(Mutex::new(receive)))
        .add_systems(
            PreStartup,
            move |mut commands: Commands, mut images: ResMut<Assets<Image>>| {
                let image = images.add(target_image(size));
                commands
                    .spawn((
                        Name::new("_OffscreenReadback"),
                        Readback::texture(image.clone()),
                    ))
                    .observe(write_frame);
                commands.insert_resource(OffscreenTarget { image, size });
            },
        )
        .add_systems(PreUpdate, inject_input.before(bevy::input::InputSystems))
        .add_systems(
            PostUpdate,
            retarget_cameras
                .before(CameraUpdateSystems)
                .before(bevy::ui::UiSystems::Prepare),
        );
    }
}

/// The image the window's cameras draw into: the window's size, sRGB
/// RGBA8 (Bevy's default view format), readable back.
fn target_image(size: UVec2) -> Image {
    let mut image = Image::new_uninit(
        Extent3d {
            width: size.x.max(1),
            height: size.y.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::RENDER_ATTACHMENT
        | TextureUsages::COPY_SRC
        | TextureUsages::COPY_DST;
    image
}

/// A camera, where it draws and the markers Bevy picks among a window's
/// cameras.
type WindowCamera = (
    Entity,
    &'static Camera,
    &'static mut RenderTarget,
    Has<IsDefaultUiCamera>,
    Has<ShadowLodOrigin>,
);

/// Points every camera drawing to the primary window at the offscreen
/// image, makes the highest-ordered of them the interface's camera and the
/// lowest-numbered the shadows' LOD origin, as Bevy picks them among a
/// window's cameras.
fn retarget_cameras(
    mut commands: Commands,
    target: Option<Res<OffscreenTarget>>,
    primary: Query<Entity, With<PrimaryWindow>>,
    mut cameras: Query<WindowCamera>,
) {
    let Some(target) = target else { return };
    let primary = primary.single().ok();
    let ours = |t: &RenderTarget| matches!(t, RenderTarget::Image(i) if i.handle == target.image);
    for (_, _, mut render_target, ..) in &mut cameras {
        let to_window = match &*render_target {
            RenderTarget::Window(WindowRef::Primary) => true,
            RenderTarget::Window(WindowRef::Entity(w)) => Some(*w) == primary,
            _ => false,
        };
        if to_window {
            *render_target = RenderTarget::Image(target.image.clone().into());
        }
    }
    // Bevy's point and spot shadows take their LOD origin from the
    // lowest-numbered camera drawing to a window; the same camera here.
    let lod_origin = cameras
        .iter()
        .filter(|(_, _, t, _, _)| ours(t))
        .map(|(e, ..)| e)
        .min();
    let interface = cameras
        .iter()
        .filter(|(_, _, t, ..)| ours(t))
        .max_by_key(|(e, c, ..)| (c.order, *e))
        .map(|(e, ..)| e);
    for (entity, _, render_target, marked, origin) in &cameras {
        let wanted = Some(entity) == interface;
        if wanted && !marked {
            commands.entity(entity).insert(IsDefaultUiCamera);
        } else if marked && !wanted && ours(render_target) {
            commands.entity(entity).remove::<IsDefaultUiCamera>();
        }
        let wanted = Some(entity) == lod_origin;
        if wanted && !origin {
            commands.entity(entity).insert(ShadowLodOrigin);
        } else if origin && !wanted && ours(render_target) {
            commands.entity(entity).remove::<ShadowLodOrigin>();
        }
    }
}

/// Writes a read-back frame to the frame file: rows unpadded, alpha opaque
/// (the window's compositor ignores it; the host's quad would not).
fn write_frame(
    readback: On<ReadbackComplete>,
    target: Res<OffscreenTarget>,
    loading: Option<Res<exa_ui::LoadingStatus>>,
    mut out: ResMut<FrameFile>,
) {
    let (w, h) = (target.size.x as usize, target.size.y as usize);
    let row = w * 4;
    let padded = bevy::render::renderer::RenderDevice::align_copy_bytes_per_row(row);
    if readback.data.len() < padded * h.saturating_sub(1) + row {
        return;
    }
    let out = &mut *out;
    out.pixels.resize(row * h, 0);
    for y in 0..h {
        out.pixels[y * row..(y + 1) * row]
            .copy_from_slice(&readback.data[y * padded..y * padded + row]);
    }
    for px in out.pixels.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    if !out.sized {
        if let Err(e) = out.file.set_len(HEADER_LEN + (row * h) as u64) {
            error!("offscreen: cannot size the frame file: {e}");
            return;
        }
        out.sized = true;
    }
    out.frame += 1;
    let mut header = FrameHeader {
        seq: out.seq + 1,
        width: w as u32,
        height: h as u32,
        frame: out.frame,
        flags: if loading.is_some() {
            FrameHeader::LOADING
        } else {
            0
        },
    };
    // Odd while the pixels change, so a reader drops a torn frame.
    let written = out
        .file
        .write_all_at(&header.to_bytes(), 0)
        .and_then(|()| out.file.write_all_at(&out.pixels, HEADER_LEN));
    header.seq += 1;
    out.seq = header.seq;
    if let Err(e) = written.and_then(|()| out.file.write_all_at(&header.to_bytes(), 0)) {
        error!("offscreen: cannot write the frame: {e}");
    }
}

/// Feeds the host's events to Bevy's input as the window's own messages.
#[allow(clippy::too_many_arguments)]
fn inject_input(
    lines: Res<InputLines>,
    mut windows: Query<(Entity, &mut Window), With<PrimaryWindow>>,
    mut keys: MessageWriter<KeyboardInput>,
    mut buttons: MessageWriter<MouseButtonInput>,
    mut moved: MessageWriter<CursorMoved>,
    mut left: MessageWriter<CursorLeft>,
    mut motion: MessageWriter<MouseMotion>,
    mut wheel: MessageWriter<MouseWheel>,
    mut exit: MessageWriter<AppExit>,
) {
    let Ok((window_entity, mut window)) = windows.single_mut() else {
        return;
    };
    let Ok(receiver) = lines.0.lock() else { return };
    let state = |pressed| {
        if pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        }
    };
    while let Ok(input) = receiver.try_recv() {
        let Some(input) = input else {
            info!("offscreen: the host closed the input; exiting");
            exit.write(AppExit::Success);
            return;
        };
        match input {
            ForwardedInput::Key {
                code,
                logical,
                pressed,
                repeat,
                text,
            } => {
                if !repeat {
                    info!("offscreen: forwarded key {code:?} pressed={pressed}");
                }
                keys.write(KeyboardInput {
                    key_code: code,
                    logical_key: logical,
                    state: state(pressed),
                    text: text.map(Into::into),
                    repeat,
                    window: window_entity,
                });
            }
            ForwardedInput::Button { button, pressed } => {
                info!("offscreen: forwarded button {button:?} pressed={pressed}");
                buttons.write(MouseButtonInput {
                    button,
                    state: state(pressed),
                    window: window_entity,
                });
            }
            ForwardedInput::Cursor(position) => {
                let delta = window.cursor_position().map(|old| position - old);
                window.set_physical_cursor_position(Some(DVec2::new(
                    position.x as f64,
                    position.y as f64,
                )));
                moved.write(CursorMoved {
                    window: window_entity,
                    position,
                    delta,
                });
            }
            ForwardedInput::CursorLeft => {
                window.set_physical_cursor_position(None);
                left.write(CursorLeft {
                    window: window_entity,
                });
            }
            ForwardedInput::Motion(delta) => {
                motion.write(MouseMotion { delta });
            }
            ForwardedInput::Wheel { line, x, y } => {
                wheel.write(MouseWheel {
                    unit: if line {
                        MouseScrollUnit::Line
                    } else {
                        MouseScrollUnit::Pixel
                    },
                    x,
                    y,
                    window: window_entity,
                    phase: TouchPhase::Moved,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forwarded_input_round_trips() {
        let events = [
            ForwardedInput::Key {
                code: KeyCode::KeyW,
                logical: Key::Character("w".into()),
                pressed: true,
                repeat: false,
                text: Some("w".into()),
            },
            ForwardedInput::Key {
                code: KeyCode::F3,
                logical: Key::F3,
                pressed: false,
                repeat: true,
                text: None,
            },
            ForwardedInput::Key {
                code: KeyCode::Space,
                logical: Key::Character(" ".into()),
                pressed: true,
                repeat: false,
                text: Some(" ".into()),
            },
            ForwardedInput::Button {
                button: MouseButton::Right,
                pressed: true,
            },
            ForwardedInput::Button {
                button: MouseButton::Other(7),
                pressed: false,
            },
            ForwardedInput::Cursor(Vec2::new(640.5, 12.0)),
            ForwardedInput::CursorLeft,
            ForwardedInput::Motion(Vec2::new(-3.0, 4.25)),
            ForwardedInput::Wheel {
                line: true,
                x: 0.0,
                y: -1.0,
            },
        ];
        for event in events {
            let line = event.encode().unwrap();
            assert_eq!(ForwardedInput::decode(&line), Some(event), "{line}");
        }
        for bad in [
            "",
            "k",
            "k NoSuchKey 1 0 u -",
            "b Left 2",
            "c 1",
            "z",
            "k KeyA 1 0 c:zz -",
        ] {
            assert_eq!(ForwardedInput::decode(bad), None, "{bad}");
        }
    }

    #[test]
    fn frame_headers_round_trip() {
        let header = FrameHeader {
            seq: 42,
            width: 1280,
            height: 720,
            frame: 9,
            flags: FrameHeader::LOADING,
        };
        let bytes = header.to_bytes();
        assert_eq!(FrameHeader::from_bytes(&bytes), Some(header));
        assert_eq!(header.pixel_len(), 1280 * 720 * 4);
        assert_eq!(FrameHeader::from_bytes(&bytes[..16]), None);
        assert_eq!(FrameHeader::from_bytes(&[0; 64]), None);
    }
}

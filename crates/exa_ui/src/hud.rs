//! The in-game HUD: the health and focus bars at the bottom centre of the
//! game's view and the block-stamina gauge above them, fading in and out as
//! the original's. The game fills [`HudBars`] every frame; this module paints
//! the bars on the CPU as the original draws them (gradients, frames,
//! shadows, all blended in gamma space) and shows them as one image. Rules:
//! `docs/UI.md` (Health bars).

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use exa_assets::Game;
use exa_formats::rfi::Rfi;

use crate::LoadingStatus;
use crate::blend::Canvas;
use crate::scale::GuiScale;

/// The stone texture the original multiplies the bars' frame lines by.
pub const GUITEX: &str = "guitex";

/// What the HUD shows, set by the game every frame.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct HudBars {
    /// A player to show the HUD for, in the game's view.
    pub active: bool,
    /// Combat mode: holds the bars in.
    pub combat: bool,
    /// Yellow (current) and red (lasting) health, and their maximum.
    pub yellow: f32,
    pub red: f32,
    pub max: f32,
    /// Focus and the capacity it regenerates to (0–1).
    pub focus: f32,
    pub focus_cap: f32,
    /// A shield is held: in combat mode the block-stamina gauge shows.
    pub shield: bool,
    /// Block stamina (0–1).
    pub stamina: f32,
}

impl Default for HudBars {
    fn default() -> Self {
        Self {
            active: false,
            combat: false,
            yellow: 0.25,
            red: 0.25,
            max: 0.25,
            focus: 1.0,
            focus_cap: 1.0,
            shield: false,
            stamina: 1.0,
        }
    }
}

/// The systems that draw the HUD; the game sets [`HudBars`] and
/// [`crate::Heartbeat`] before them.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DrawHud;

/// A bar segment's colours: top, middle (upper half's end), middle (lower
/// half's start) and bottom, RGBA.
pub type Stops = [[u8; 4]; 4];

/// Health up to the yellow value.
pub const YELLOW: Stops = [
    [240, 230, 200, 240],
    [120, 85, 15, 210],
    [130, 80, 0, 200],
    [210, 180, 30, 240],
];
/// Health between yellow and red: damage that will heal.
pub const BROWN: Stops = [
    [180, 160, 145, 200],
    [70, 50, 10, 160],
    [70, 40, 0, 128],
    [100, 80, 50, 200],
];
/// Health beyond red: lasting damage.
pub const RED: Stops = [
    [220, 160, 130, 220],
    [100, 25, 5, 210],
    [90, 0, 0, 190],
    [140, 60, 15, 220],
];
/// Focus.
pub const BLUE: Stops = [
    [150, 180, 230, 240],
    [20, 60, 160, 210],
    [0, 30, 80, 200],
    [40, 120, 255, 240],
];
/// Focus spent, up to the capacity.
pub const SPENT: Stops = [
    [135, 140, 150, 200],
    [30, 40, 50, 160],
    [5, 10, 40, 128],
    [70, 90, 120, 200],
];
/// Focus capacity held back (by lasting powers).
pub const RESERVED: Stops = [
    [180, 170, 220, 220],
    [60, 20, 100, 210],
    [30, 0, 70, 190],
    [100, 40, 200, 220],
];
/// Block stamina left.
pub const STAMINA: Stops = [
    [255, 255, 255, 240],
    [90, 90, 90, 210],
    [70, 70, 70, 200],
    [180, 180, 180, 240],
];
/// Block stamina used.
pub const STAMINA_USED: Stops = [
    [160, 160, 160, 200],
    [30, 30, 30, 160],
    [20, 20, 20, 128],
    [80, 80, 80, 200],
];
/// The frames' two line colours (outer, inner) before the stone texture,
/// and their opacity at full fade.
pub const FRAME_OUTER: [u8; 3] = [129, 134, 135];
pub const FRAME_INNER: [u8; 3] = [28, 30, 30];
pub const FRAME_ALPHA: f32 = 239.0;
/// The drop shadow's and the inner shadow's opacity at full fade.
pub const SHADOW_ALPHA: f32 = 220.0;
pub const INNER_SHADOW_ALPHA: f32 = 240.0;
/// Red health at or below this draws no yellow-damage segment.
const RED_FLOOR: f32 = 0.01;
/// Opacities at or below this draw nothing.
const INVISIBLE: f32 = 1.2e-7;

/// Where the HUD goes in a window (physical pixels, y down).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub gui: i32,
    /// The health and focus bars' left edges, their top and width.
    pub health_x: i32,
    pub focus_x: i32,
    pub y: i32,
    pub width: i32,
    /// The bars' fill reaches row y + height (from y + 2).
    pub height: i32,
    /// The stamina gauge's left edge, top and height.
    pub gauge_x: i32,
    pub gauge_y: i32,
    pub gauge_height: i32,
}

/// The HUD's layout in a window at a GUI scale: bars 48 × scale wide, 52 ×
/// scale apart, from 50 × scale left of the centre, 5 × scale above the
/// bottom; the gauge centred 9 × scale above the bottom.
pub fn layout(window_width: u32, window_height: u32, gui: u32) -> Layout {
    let (h, g) = (window_height as i32, gui as i32);
    let centre = (window_width >> 1) as i32;
    let health_x = centre - 50 * g;
    Layout {
        gui: g,
        health_x,
        focus_x: health_x + 52 * g,
        y: h - 5 * g,
        width: 48 * g,
        height: 4 * g - 4,
        gauge_x: centre - 24 * g,
        gauge_y: h - 9 * g,
        gauge_height: 2 * g,
    }
}

impl Layout {
    /// A rectangle (x0, y0, x1, y1) around a bar: its frame and shadow.
    fn bar_rect(&self, x: i32) -> (i32, i32, i32, i32) {
        (x - 2, self.y, x + self.width + 2, self.y + 4 * self.gui - 2)
    }

    fn gauge_rect(&self) -> (i32, i32, i32, i32) {
        let top = self.gauge_y - 1;
        (
            self.gauge_x - 2,
            top,
            self.gauge_x + self.width + 2,
            top + self.gauge_height + 4,
        )
    }

    /// The canvas the HUD is painted on: from the left bar's shadow to the
    /// right bar's, from the gauge's frame to the window's bottom.
    pub fn canvas(&self, window_height: u32) -> Canvas {
        let shadow = 2 * self.gui;
        let x = self.health_x - 2 - shadow;
        let right = self.focus_x + self.width + 2 + shadow;
        let y = self.gauge_y - 1;
        Canvas::new(
            x,
            y,
            (right - x).max(0) as usize,
            (window_height as i32 - y).max(0) as usize,
        )
    }
}

/// The original's fade for the HUD: 500 ms in while shown, then a 200 ms
/// hold and 2 s out (game time).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Fade {
    pub opacity: f32,
    rising: bool,
    end: i64,
}

const RISE_MS: f32 = 500.0;
const FALL_MS: f32 = 2000.0;
const HOLD_AND_FALL_MS: i64 = 2200;

impl Fade {
    /// The opacity at `now` (ms) when the HUD is or is not shown this frame.
    pub fn step(&mut self, shown: bool, now: i64) -> f32 {
        if shown {
            if !self.rising {
                self.end = now + ((1.0 - self.opacity) * RISE_MS).round_ties_even() as i64;
                self.rising = true;
            }
            self.opacity = if self.end < now {
                1.0
            } else {
                1.0 - (self.end - now) as f32 / RISE_MS
            };
        } else {
            if self.rising {
                self.rising = false;
                self.end = if self.opacity < 1.0 {
                    now + (self.opacity * FALL_MS).round_ties_even() as i64
                } else {
                    now + HOLD_AND_FALL_MS
                };
            }
            self.opacity = if self.end < now {
                0.0
            } else if now < self.end - FALL_MS as i64 {
                1.0
            } else {
                (self.end - now) as f32 / FALL_MS
            };
        }
        self.opacity
    }
}

/// The stone texture of the frame lines, sampled one texel per window pixel
/// with rows counted from the window's bottom, tiled.
#[derive(Clone, Debug)]
pub struct FrameTexture {
    size: usize,
    rgb: Vec<[f32; 3]>,
}

impl FrameTexture {
    /// From the installation's texture; `None` unless it is square.
    pub fn from_rfi(rfi: &Rfi) -> Option<Self> {
        let (w, h) = (rfi.header.width as usize, rfi.header.height as usize);
        let rgba = rfi.decode_rgba8().ok()?;
        (w == h && w > 0 && rgba.len() == w * h * 4).then(|| Self {
            size: w,
            rgb: rgba
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| [p[0], p[1], p[2]].map(|c| c as f32 / 255.0))
                .collect(),
        })
    }

    /// A flat grey, the stone's mean, when the texture is missing.
    pub fn flat() -> Self {
        Self {
            size: 1,
            rgb: vec![[113.0 / 255.0; 3]],
        }
    }

    /// The texel under a window pixel; stored row 0 is the window's bottom.
    pub fn at(&self, x: i32, y: i32, window_height: i32) -> [f32; 3] {
        let n = self.size as i32;
        let col = x.rem_euclid(n) as usize;
        let row = (window_height - 1 - y).rem_euclid(n) as usize;
        self.rgb[row * self.size + col]
    }

    /// The mean colour (0–1).
    pub fn mean(&self) -> [f32; 3] {
        let n = self.rgb.len() as f32;
        let sum = self
            .rgb
            .iter()
            .fold([0.0; 3], |s, c| [s[0] + c[0], s[1] + c[1], s[2] + c[2]]);
        sum.map(|s| s / n)
    }
}

/// Paints the HUD at the bars' fade `bars_fade` and the gauge's fade
/// `gauge_fade`, in the original's order: both frames, both drop shadows,
/// the health bar, the focus bar, then the gauge with its frame on top.
pub fn paint(
    hud: &HudBars,
    layout: &Layout,
    (bars_fade, gauge_fade): (f32, f32),
    texture: &FrameTexture,
    window_height: u32,
) -> Canvas {
    let mut canvas = layout.canvas(window_height);
    let h = window_height as i32;
    let g = layout.gui;
    if bars_fade > INVISIBLE {
        let fade = bars_fade;
        let line = (FRAME_ALPHA * fade).round_ties_even() / 255.0;
        let (health, focus) = (
            layout.bar_rect(layout.health_x),
            layout.bar_rect(layout.focus_x),
        );
        frame(&mut canvas, health, line, texture, h);
        frame(&mut canvas, focus, line, texture, h);
        let shadow = (SHADOW_ALPHA * fade).round_ties_even() / 255.0;
        drop_shadow(&mut canvas, health, 2 * g, shadow);
        drop_shadow(&mut canvas, focus, 2 * g, shadow);
        let fill = fade * fade;
        let inner = (INNER_SHADOW_ALPHA * fade).round_ties_even() / 255.0;
        let share = |v: f32| if hud.max > 0.0 { v / hud.max } else { 0.0 };
        let red = if hud.red <= RED_FLOOR {
            0.0
        } else {
            share(hud.red)
        };
        let bar = Bar {
            x: layout.health_x,
            y: layout.y,
            width: layout.width,
            height: layout.height,
        };
        bar.fill(
            &mut canvas,
            share(hud.yellow).max(0.0),
            red,
            [&YELLOW, &BROWN, &RED],
            false,
            fill,
        );
        inner_shadow(
            &mut canvas,
            bar.x,
            bar.y + 1,
            bar.width,
            bar.height,
            g,
            inner,
        );
        let bar = Bar {
            x: layout.focus_x,
            ..bar
        };
        bar.fill(
            &mut canvas,
            hud.focus,
            hud.focus_cap,
            [&BLUE, &SPENT, &RESERVED],
            true,
            fill,
        );
        inner_shadow(
            &mut canvas,
            bar.x,
            bar.y + 1,
            bar.width,
            bar.height,
            g,
            inner,
        );
    }
    if gauge_fade > INVISIBLE {
        let fade = gauge_fade;
        let bar = Bar {
            x: layout.gauge_x,
            y: layout.gauge_y,
            width: layout.width,
            height: layout.gauge_height,
        };
        bar.fill(
            &mut canvas,
            hud.stamina,
            1.0,
            [&STAMINA, &STAMINA_USED, &STAMINA_USED],
            false,
            fade * fade,
        );
        let inner = (INNER_SHADOW_ALPHA * fade).round_ties_even() / 255.0;
        inner_shadow(
            &mut canvas,
            bar.x,
            bar.y + 1,
            bar.width,
            bar.height,
            g / 2 + 1,
            inner,
        );
        let line = (FRAME_ALPHA * fade).round_ties_even() / 255.0;
        frame(&mut canvas, layout.gauge_rect(), line, texture, h);
    }
    canvas
}

/// A bar's fill area: rows y + 2 .. y + height, columns x .. x + width.
#[derive(Clone, Copy)]
struct Bar {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

impl Bar {
    /// Three segments: the first up to `a` of the width, the second to `b`,
    /// the third to the end, the pixel where the first ends mixed by how
    /// much of it the first covers. `guard`: the first is drawn only when
    /// it is at least a pixel wide (the focus bar). `alpha` scales the
    /// colours' opacities.
    fn fill(
        &self,
        canvas: &mut Canvas,
        a: f32,
        b: f32,
        segments: [&Stops; 3],
        guard: bool,
        alpha: f32,
    ) {
        let (x, w) = (self.x as f32, self.width as f32);
        let end_a = w * a + x;
        let end_b = w * b + x;
        let i = end_a as i32;
        let covered = 1.0 - (end_a - i as f32);
        let (top, bottom) = (self.y + 2, self.y + self.height);
        let scaled =
            |s: &Stops| s.map(|[r, g, b, a]| [r, g, b, (a as f32 * alpha).round_ties_even() as u8]);
        if !guard || self.x < i {
            gradient(canvas, x, i as f32, top, bottom, &scaled(segments[0]));
        }
        if i < self.x + self.width {
            // The original draws this pixel as a one-pixel line at its
            // right edge.
            let mixed = mix(segments[0], segments[1], covered);
            gradient(
                canvas,
                i as f32,
                i as f32 + 1.0,
                top,
                bottom,
                &scaled(&mixed),
            );
            gradient(
                canvas,
                (i + 1) as f32,
                end_b,
                top,
                bottom,
                &scaled(segments[1]),
            );
            gradient(canvas, end_b, x + w, top, bottom, &scaled(segments[2]));
        }
    }
}

/// Stops mixed from `a` towards `b` by `t`, truncated as the original's.
fn mix(a: &Stops, b: &Stops, t: f32) -> Stops {
    let mut out = *a;
    for (k, stop) in out.iter_mut().enumerate() {
        for (c, v) in stop.iter_mut().enumerate() {
            let (p, q) = (a[k][c] as f32, b[k][c] as f32);
            *v = (p + (q - p) * t).clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// A quad from x0 to x1 (either order; a pixel is in when its centre is),
/// rows y0 .. y1: the upper half shades from the first stop to the second,
/// the lower half from the third to the fourth.
fn gradient(canvas: &mut Canvas, x0: f32, x1: f32, y0: i32, y1: i32, stops: &Stops) {
    let (lo, hi) = (x0.min(x1), x0.max(x1));
    let (first, last) = ((lo - 0.5).ceil() as i32, (hi - 0.5).ceil() as i32);
    if first >= last || y1 <= y0 {
        return;
    }
    let mid = (y0 + y1) as f32 / 2.0;
    for y in y0..y1 {
        let yc = y as f32 + 0.5;
        let (from, to, t) = if yc < mid {
            (stops[0], stops[1], (yc - y0 as f32) / (mid - y0 as f32))
        } else {
            (stops[2], stops[3], (yc - mid) / (y1 as f32 - mid))
        };
        let lerp = |c: usize| from[c] as f32 + (to[c] as f32 - from[c] as f32) * t;
        let colour = [lerp(0), lerp(1), lerp(2)].map(|v| v / 255.0);
        let alpha = lerp(3) / 255.0;
        for x in first..last {
            canvas.blend(x, y, colour, alpha);
        }
    }
}

/// The two-line frame of a rectangle: its border pixels in the outer
/// colour, the ring one pixel in in the inner colour, each times the stone
/// texture under it.
fn frame(
    canvas: &mut Canvas,
    (x0, y0, x1, y1): (i32, i32, i32, i32),
    alpha: f32,
    texture: &FrameTexture,
    window_height: i32,
) {
    for (inset, colour) in [(0, FRAME_OUTER), (1, FRAME_INNER)] {
        let (x0, y0, x1, y1) = (x0 + inset, y0 + inset, x1 - inset, y1 - inset);
        if x1 <= x0 || y1 <= y0 {
            continue;
        }
        let colour = colour.map(|c| c as f32 / 255.0);
        for y in y0..y1 {
            let edge = y == y0 || y == y1 - 1;
            let columns: Vec<i32> = if edge {
                (x0..x1).collect()
            } else if x1 - 1 > x0 {
                vec![x0, x1 - 1]
            } else {
                vec![x0]
            };
            for x in columns {
                let t = texture.at(x, y, window_height);
                let shade = [colour[0] * t[0], colour[1] * t[1], colour[2] * t[2]];
                canvas.blend(x, y, shade, alpha);
            }
        }
    }
}

/// A soft black shadow `spread` pixels wide around a rectangle (not under
/// it): opacity alpha × (1 − d / spread)⁴, d the distance from the
/// rectangle (round at the corners).
fn drop_shadow(
    canvas: &mut Canvas,
    (x0, y0, x1, y1): (i32, i32, i32, i32),
    spread: i32,
    alpha: f32,
) {
    if spread <= 0 {
        return;
    }
    let f = spread as f32;
    for y in (y0 - spread)..(y1 + spread) {
        for x in (x0 - spread)..(x1 + spread) {
            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
            let dx = (x0 as f32 - cx).max(cx - x1 as f32).max(0.0);
            let dy = (y0 as f32 - cy).max(cy - y1 as f32).max(0.0);
            if dx == 0.0 && dy == 0.0 {
                continue;
            }
            let k = (1.0 - (dx * dx + dy * dy).sqrt() / f).max(0.0);
            canvas.blend(x, y, [0.0; 3], alpha * k * k * k * k);
        }
    }
}

/// A black shade inside a rectangle from its edges: alpha at the edge, none
/// `radius` pixels in (at most half the height).
fn inner_shadow(
    canvas: &mut Canvas,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    radius: i32,
    alpha: f32,
) {
    let r = radius.min(height / 2) as f32;
    if r <= 0.0 {
        return;
    }
    let (left, top, right, bottom) = (x as f32, y as f32, (x + width) as f32, (y + height) as f32);
    for py in y..y + height {
        for px in x..x + width {
            let (cx, cy) = (px as f32 + 0.5, py as f32 + 0.5);
            let d = (cx - left).min(right - cx).min(cy - top).min(bottom - cy);
            if d < r {
                canvas.blend(px, py, [0.0; 3], alpha * (1.0 - d / r));
            }
        }
    }
}

/// The frame texture, when loaded.
#[derive(Resource)]
struct Texture(FrameTexture);

/// The HUD node.
#[derive(Component)]
struct HudNode;

/// Fades, last frame's values and the painted image.
#[derive(Resource, Default)]
struct HudState {
    bars: Fade,
    gauge: Fade,
    /// Last frame's yellow health and focus.
    last: Option<(f32, f32)>,
    painted: Option<(UVec2, u32, HudBars, f32, f32)>,
    origin: IVec2,
    size: UVec2,
    image: Option<Handle<Image>>,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<HudBars>()
        .init_resource::<HudState>()
        .add_systems(Startup, spawn)
        .add_systems(Update, draw.in_set(DrawHud));
}

/// Under the world cursor, over the heartbeat border.
const Z: i32 = 900;

fn spawn(mut commands: Commands, game: Option<Res<Game>>) {
    let texture = game.as_ref().and_then(|g| {
        g.0.texture(GUITEX)
            .and_then(|b| Rfi::parse(b).ok())
            .and_then(|rfi| FrameTexture::from_rfi(&rfi))
    });
    if texture.is_none() {
        warn!("{GUITEX} missing: HUD frames drawn flat");
    }
    commands.insert_resource(Texture(texture.unwrap_or_else(FrameTexture::flat)));
    commands.spawn((
        Name::new("Hud"),
        HudNode,
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::default(),
        GlobalZIndex(Z),
        Visibility::Hidden,
    ));
}

#[allow(clippy::too_many_arguments)]
fn draw(
    hud: Res<HudBars>,
    gui: Res<GuiScale>,
    time: Res<Time<Virtual>>,
    loading: Option<Res<LoadingStatus>>,
    texture: Option<Res<Texture>>,
    mut state: ResMut<HudState>,
    mut images: ResMut<Assets<Image>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut nodes: Query<(&mut Node, &mut ImageNode, &mut Visibility), With<HudNode>>,
) {
    let (Ok(window), Ok((mut node, mut sprite, mut visibility)), Some(texture)) =
        (windows.single(), nodes.single_mut(), texture)
    else {
        return;
    };
    let hide = |visibility: &mut Visibility| {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    };
    if !hud.active || loading.is_some() {
        // Without a player the original draws nothing and keeps no state.
        state.bars = Fade::default();
        state.gauge = Fade::default();
        state.last = None;
        hide(&mut visibility);
        return;
    }
    let now = time.elapsed().as_millis() as i64;
    // Shown while in combat mode, in a frame where the yellow health
    // changed or the focus dropped. The values compared against are taken
    // only in a frame that shows the bars, as the original's show rule
    // stores them (a rise of the focus while hidden is not remembered).
    let changed = state
        .last
        .is_some_and(|(yellow, focus)| hud.yellow != yellow || hud.focus < focus);
    let shown = hud.combat || changed;
    let bars_fade = state.bars.step(shown, now);
    let gauge_fade = state.gauge.step(hud.combat && hud.shield, now);
    if shown || state.last.is_none() {
        state.last = Some((hud.yellow, hud.focus));
    }
    if bars_fade <= INVISIBLE && gauge_fade <= INVISIBLE {
        hide(&mut visibility);
        return;
    }
    let size = UVec2::new(window.physical_width(), window.physical_height());
    let key = (size, gui.scale, *hud, bars_fade, gauge_fade);
    if state.painted != Some(key) {
        let layout = layout(size.x, size.y, gui.scale);
        let canvas = paint(&hud, &layout, (bars_fade, gauge_fade), &texture.0, size.y);
        let mut image = Image::new(
            Extent3d {
                width: canvas.width as u32,
                height: canvas.height as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            canvas.to_srgba8(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        if let Some(handle) = state.image.clone()
            && let Some(mut slot) = images.get_mut(&handle)
        {
            *slot = image;
        } else {
            state.image = Some(images.add(image));
        }
        state.origin = IVec2::new(canvas.x, canvas.y);
        state.size = UVec2::new(canvas.width as u32, canvas.height as u32);
        state.painted = Some(key);
        debug!("HUD painted: fade {bars_fade:.3}, gauge {gauge_fade:.3}");
    }
    if let Some(handle) = &state.image
        && sprite.image != *handle
    {
        sprite.image = handle.clone();
    }
    // Physical pixels, as the original draws whatever the scale factor.
    let unit = 1.0 / window.scale_factor();
    node.left = px(state.origin.x as f32 * unit);
    node.top = px(state.origin.y as f32 * unit);
    node.width = px(state.size.x as f32 * unit);
    node.height = px(state.size.y as f32 * unit);
    if *visibility != Visibility::Inherited {
        *visibility = Visibility::Inherited;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The user's retail window: 1600 × 1014, GUI scale 5.
    fn retail() -> Layout {
        layout(1600, 1014, 5)
    }

    fn close(a: [f32; 4], b: [f32; 4], within: f32) -> bool {
        a.iter().zip(b).all(|(p, q)| (p - q).abs() <= within)
    }

    #[test]
    fn the_bars_sit_at_the_bottom_centre() {
        let l = retail();
        // Measured on the retail screenshot: fills at x 550–789 and
        // 810–1049, rows 991–1004 (y + 2 .. y + 16).
        assert_eq!((l.health_x, l.focus_x, l.y, l.width), (550, 810, 989, 240));
        assert_eq!(l.height, 16);
        assert_eq!((l.gauge_x, l.gauge_y, l.gauge_height), (680, 969, 10));
        // Exarust's default window: the same scale, 114 px higher.
        let d = layout(1600, 900, 5);
        assert_eq!((d.health_x, d.y), (550, 875));
        // Scale 4 (a 720-pixel-tall window).
        let s = layout(1280, 720, 4);
        assert_eq!(
            (s.health_x, s.focus_x, s.y, s.width, s.height),
            (440, 648, 700, 192, 12)
        );
        // The canvas spans both shadows, the gauge's frame to the bottom.
        let c = l.canvas(1014);
        assert_eq!((c.x, c.y, c.width, c.height), (538, 968, 524, 46));
    }

    #[test]
    fn fades_in_in_half_a_second_and_out_in_two() {
        let mut f = Fade::default();
        assert_eq!(f.step(false, 0), 0.0);
        // Combat mode from 1000 ms.
        assert_eq!(f.step(true, 1000), 0.0);
        assert!((f.step(true, 1250) - 0.5).abs() < 1e-6);
        assert_eq!(f.step(true, 1500), 1.0);
        assert_eq!(f.step(true, 5000), 1.0);
        // Out of combat mode at 6000: held for 200 ms, then 2 s down.
        assert_eq!(f.step(false, 6000), 1.0);
        assert_eq!(f.step(false, 6199), 1.0);
        assert!((f.step(false, 7200) - 0.5).abs() < 1e-6);
        assert_eq!(f.step(false, 8200), 0.0);
        assert_eq!(f.step(false, 9000), 0.0);
        // Back in half-way down: the rise takes the rest, 250 ms.
        let mut f = Fade::default();
        f.step(true, 0);
        f.step(true, 600);
        f.step(false, 1000);
        assert!((f.step(false, 2200) - 0.5).abs() < 1e-6);
        assert!((f.step(true, 2300) - 0.5).abs() < 1e-6);
        assert_eq!(f.step(true, 2550), 1.0);
        // Dropping out half-way up falls from there with no hold.
        let mut f = Fade::default();
        f.step(true, 0);
        assert!((f.step(true, 250) - 0.5).abs() < 1e-6);
        assert!((f.step(false, 260) - 0.5).abs() < 1e-6);
        assert!((f.step(false, 760) - 0.25).abs() < 1e-6);
        // A single frame of change shows nothing.
        let mut f = Fade::default();
        assert_eq!(f.step(true, 100), 0.0);
        assert_eq!(f.step(false, 116), 0.0);
        assert_eq!(f.step(false, 400), 0.0);
    }

    fn hurt() -> HudBars {
        // The live retail values next to the reference screenshot.
        HudBars {
            active: true,
            combat: true,
            yellow: 0.176,
            red: 0.226,
            max: 0.25,
            ..default()
        }
    }

    fn painted(hud: &HudBars, fade: f32) -> Canvas {
        paint(hud, &retail(), (fade, 0.0), &FrameTexture::flat(), 1014)
    }

    /// A pixel's colour as 0–255 and its opacity as 0–255.
    fn bytes(c: &Canvas, x: i32, y: i32) -> [f32; 4] {
        c.get(x, y).unwrap().map(|v| v * 255.0)
    }

    #[test]
    fn health_shows_yellow_then_brown_then_red() {
        let c = painted(&hurt(), 1.0);
        let row = 998; // the fill's middle row, clear of the inner shadow
        // Yellow 0.704 of 240 ends at 718.96: 718 is the mixed pixel.
        let yellow = bytes(&c, 700, row);
        let brown = bytes(&c, 740, row);
        let red = bytes(&c, 780, row);
        // Lower half's first row: 1/14 of the way from the middle stop to
        // the bottom one.
        let lower = |s: &Stops| {
            let t = 0.5 / 7.0;
            [0, 1, 2, 3].map(|k| s[2][k] as f32 + (s[3][k] as f32 - s[2][k] as f32) * t)
        };
        assert!(close(yellow, lower(&YELLOW), 0.01), "{yellow:?}");
        assert!(close(brown, lower(&BROWN), 0.01), "{brown:?}");
        assert!(close(red, lower(&RED), 0.01), "{red:?}");
        // The mixed pixel: 96 % yellow.
        let mixed = bytes(&c, 718, row);
        let end = 240.0 * (0.176f32 / 0.25) + 550.0;
        assert!((end - 718.96).abs() < 1e-3);
        let m = mix(&YELLOW, &BROWN, 1.0 - (end - end.trunc()));
        assert!(close(mixed, lower(&m), 0.01), "{mixed:?}");
        assert_eq!(bytes(&c, 717, row), yellow);
        assert_eq!(bytes(&c, 719, row), brown);
        // Red 0.904 of 240 ends at 766.96: 766 is still brown.
        assert_eq!(bytes(&c, 766, row), brown);
        assert_eq!(bytes(&c, 767, row), red);
        // The last pixels are under the inner shadow (5 px).
        assert_eq!(bytes(&c, 784, row), red);
        assert!(bytes(&c, 789, row)[0] < red[0] / 4.0);
        // Rows: the fill from y + 2 to y + 15.
        assert!(bytes(&c, 700, 1004)[3] > 200.0);
        assert!(bytes(&c, 700, 991)[3] > 200.0);
    }

    #[test]
    fn full_health_and_focus_fill_their_bars() {
        let full = HudBars {
            active: true,
            combat: true,
            ..default()
        };
        let c = painted(&full, 1.0);
        let lower = |s: &Stops| {
            let t = 0.5 / 7.0;
            [0, 1, 2, 3].map(|k| s[2][k] as f32 + (s[3][k] as f32 - s[2][k] as f32) * t)
        };
        for x in [560, 700, 780] {
            assert!(close(bytes(&c, x, 998), lower(&YELLOW), 0.01));
        }
        // Focus 1 of 1: blue to the end, no spent or reserved part.
        for x in [820, 1000, 1039] {
            assert!(close(bytes(&c, x, 998), lower(&BLUE), 0.01), "{x}");
        }
        // Focus half spent, capacity 0.75: blue, then grey, then purple.
        let spent = HudBars {
            focus: 0.5,
            focus_cap: 0.75,
            ..full
        };
        let c = painted(&spent, 1.0);
        assert!(close(bytes(&c, 900, 998), lower(&BLUE), 0.01));
        assert!(close(bytes(&c, 960, 998), lower(&SPENT), 0.01));
        assert!(close(bytes(&c, 1040, 998), lower(&RESERVED), 0.01));
        // Dead: no yellow, brown to the red.
        let dead = HudBars {
            yellow: 0.0,
            red: 0.1,
            ..full
        };
        let c = painted(&dead, 1.0);
        assert!(close(bytes(&c, 560, 998), lower(&BROWN), 0.01));
        assert!(close(bytes(&c, 700, 998), lower(&RED), 0.01));
    }

    #[test]
    fn frames_shadows_and_edges_are_where_the_original_draws_them() {
        let c = painted(&hurt(), 1.0);
        let stone = 113.0 / 255.0;
        let line = |col: [u8; 3]| {
            let [r, g, b] = col.map(|v| v as f32 * stone);
            [r, g, b, FRAME_ALPHA]
        };
        // Outer lines at x − 2 and x + w + 1, rows y and y + 17; inner
        // lines one pixel in (column 549 is clear of the inner shadow).
        assert!(close(bytes(&c, 548, 995), line(FRAME_OUTER), 0.01));
        assert!(close(bytes(&c, 791, 995), line(FRAME_OUTER), 0.01));
        assert!(close(bytes(&c, 700, 989), line(FRAME_OUTER), 0.01));
        assert!(close(bytes(&c, 700, 1006), line(FRAME_OUTER), 0.01));
        assert!(close(bytes(&c, 549, 995), line(FRAME_INNER), 0.01));
        assert!(close(bytes(&c, 790, 995), line(FRAME_INNER), 0.01));
        assert!(close(bytes(&c, 808, 995), line(FRAME_OUTER), 0.01));
        assert!(close(bytes(&c, 1051, 995), line(FRAME_OUTER), 0.01));
        // The drop shadow: just above the frame, half a pixel out:
        // 220 × 0.95⁴; three pixels out 220 × 0.75⁴; ten out nothing.
        let above = bytes(&c, 700, 988);
        assert!(
            close(above, [0.0, 0.0, 0.0, 220.0 * 0.95f32.powi(4)], 0.01),
            "{above:?}"
        );
        let three = bytes(&c, 700, 986);
        assert!((three[3] - 220.0 * 0.75f32.powi(4)).abs() < 0.01);
        assert_eq!(bytes(&c, 700, 978)[3], 0.0);
        // Round at the corners: diagonally 3 px out each way.
        let corner = bytes(&c, 544, 985)[3];
        let r = (3.5f32 * 3.5 * 2.0).sqrt() / 10.0;
        assert!((corner - 220.0 * (1.0 - r).powi(4)).abs() < 0.01);
        // The inner shadow darkens the fill's edges: the top fill row,
        // 1.5 px in, under 240 × 0.7 of black.
        let top = bytes(&c, 700, 991)[3];
        let fill = YELLOW[0][3] as f32 / 255.0; // about the top stop
        let shade = 240.0 / 255.0 * 0.7;
        assert!(
            (top / 255.0 - (shade + fill * (1.0 - shade))).abs() < 0.01,
            "{top}"
        );
        // Nothing between the bars' frames but their shadows' overlap.
        let gap = bytes(&c, 800, 995);
        assert_eq!([gap[0], gap[1], gap[2]], [0.0; 3]);
        assert!(gap[3] > 0.0);
        // No gauge without a shield.
        assert_eq!(bytes(&c, 700, 975)[3], 0.0);
    }

    #[test]
    fn the_fade_thins_fills_by_its_square() {
        let half = painted(&hurt(), 0.5);
        let full = painted(&hurt(), 1.0);
        // Stops at round(alpha × 0.25); the middle row of yellow.
        let a = bytes(&half, 700, 998)[3];
        let want = YELLOW[2][3] as f32 * 0.25
            + (YELLOW[3][3] as f32 - YELLOW[2][3] as f32) * 0.25 * 0.5 / 7.0;
        assert!((a - want).abs() < 0.6, "{a} {want}");
        // Frame lines and shadows by the fade itself.
        assert!((bytes(&half, 548, 995)[3] - 120.0).abs() < 0.01);
        assert!(bytes(&half, 700, 988)[3] < bytes(&full, 700, 988)[3]);
        // At zero nothing is drawn.
        let none = painted(&hurt(), 0.0);
        assert!(none.pixels.iter().all(|p| p[3] == 0.0));
    }

    #[test]
    fn the_stamina_gauge_shows_above_with_a_shield() {
        let guarded = HudBars {
            shield: true,
            stamina: 0.5,
            ..hurt()
        };
        let c = paint(&guarded, &retail(), (1.0, 1.0), &FrameTexture::flat(), 1014);
        // The gauge: x 680–919, rows 971–978 (y + 2 .. y + 10); half used.
        // Its inner shadow (3 px) leaves its middle rows clear only at
        // the centre columns, so test the colour order by brightness.
        let left = bytes(&c, 700, 975);
        let right = bytes(&c, 900, 975);
        assert!(left[0] > right[0] + 40.0, "{left:?} {right:?}");
        // Its frame: outer at x − 2, rows y − 1 and y + 12.
        let stone = 113.0 / 255.0;
        let outer = FRAME_OUTER.map(|v| v as f32 * stone);
        assert!(close(
            bytes(&c, 678, 974),
            [outer[0], outer[1], outer[2], FRAME_ALPHA],
            0.01
        ));
        assert!(bytes(&c, 700, 968)[3] > 200.0);
        assert!(bytes(&c, 700, 981)[3] > 200.0);
    }

    #[test]
    fn the_stone_is_sampled_from_the_bottom_tiled() {
        let tex = FrameTexture {
            size: 2,
            rgb: vec![[0.0; 3], [0.25; 3], [0.5; 3], [1.0; 3]],
        };
        // Window 10 tall: row 9 is stored row 0.
        assert_eq!(tex.at(0, 9, 10), [0.0; 3]);
        assert_eq!(tex.at(1, 9, 10), [0.25; 3]);
        assert_eq!(tex.at(0, 8, 10), [0.5; 3]);
        assert_eq!(tex.at(3, 8, 10), [1.0; 3]);
        assert_eq!(tex.at(-1, 7, 10), [0.25; 3]);
        assert_eq!(tex.mean(), [0.4375; 3]);
    }

    /// The installed stone texture is 512 pixels square, blue-grey with a
    /// mean near (111, 113, 117) (skipped without `EXANIMA_DIR`).
    #[test]
    fn the_installed_stone_texture() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let rfi = Rfi::parse(game.texture(GUITEX).unwrap()).unwrap();
        assert_eq!((rfi.header.width, rfi.header.height), (512, 512));
        let tex = FrameTexture::from_rfi(&rfi).unwrap();
        let mean = tex.mean().map(|v| v * 255.0);
        for (m, want) in mean.iter().zip([111.5, 112.7, 117.4]) {
            assert!((m - want).abs() < 2.0, "{mean:?}");
        }
    }
}

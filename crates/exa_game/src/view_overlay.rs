//! The 3D viewport's two overlays, live: the frame-rate readout in the
//! top-right corner and the version line in the bottom-left corner, drawn
//! over the interface as the original's viewport draw slot ends. Each line
//! is rasterised by the GUI's own rasteriser into a picture of the line's
//! box only, again only when its text or place changes.
//!
//! The original's viewport keeps a frame-rate flag and the game keeps a
//! title flag. The readout is drawn while the frame-rate flag is set and the
//! title flag is clear; the version line while the title flag is set:
//! - the title flag is set where the game enters its title (the return to
//!   the title and the splash's hand-over) and cleared where a session
//!   starts or continues;
//! - the frame-rate flag takes the ShowFPS setting where a session starts
//!   and whenever the setting changes, and is cleared by the return to the
//!   title (the session's leave clears it).
//!
//! The readout counts frames only while it is drawn; once more than 250 ms
//! have passed since the last reading, it reads the frames over the elapsed
//! seconds ([`exa_ui::viewport::FpsCounter`]). Both lines are drawn in the
//! GUI level's main font over a one-pixel black shadow at half alpha, up and
//! left; the readout's ink is the skin's seventh label ink at alpha 0xa0,
//! the version's that ink as it is.
//!
//! Rules: `docs/UI.md` (Frame rate, The 3D viewport).

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use exa_ui::gui_draw::{GuiFrame, GuiLayer, GuiTextures, LayerTexture, text_batch};
use exa_ui::viewport::{
    BORDER, FPS_ALPHA, FpsCounter, OVERLAY_SHADOW, VERSION, fps_place, version_place,
};

/// The label ink both overlays take: the skin's seventh.
pub const OVERLAY_INK: usize = 6;

/// The viewport's frame-rate flag and the game's title flag.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewOverlays {
    /// The viewport's frame-rate flag (the ShowFPS setting while a session
    /// runs).
    pub fps: bool,
    /// The game is at its title: the version line shows and the readout
    /// does not.
    pub title: bool,
}

impl Default for ViewOverlays {
    fn default() -> Self {
        Self {
            fps: true,
            title: false,
        }
    }
}

impl ViewOverlays {
    /// The readout is drawn.
    pub fn shows_fps(&self) -> bool {
        self.fps && !self.title
    }

    /// The game enters its title: the version line shows, and the readout's
    /// flag is clear (the session's leave, or the viewport's constructor at
    /// start-up, left it so).
    pub fn enter_title(&mut self) {
        self.title = true;
        self.fps = false;
    }

    /// A session starts or continues: the title flag clears and the
    /// readout's flag takes the ShowFPS setting.
    pub fn start_session(&mut self, show_fps: bool) {
        self.title = false;
        self.fps = show_fps;
    }
}

/// The readout's counter and text.
#[derive(Resource, Debug, Clone, Default)]
pub struct FpsReadout(pub FpsCounter);

impl FpsReadout {
    /// One frame of the viewport's draw at `now` (ms): counted while the
    /// readout is drawn; otherwise the text is cleared and the count and
    /// clock stand where they were, as the original's do.
    pub fn frame(&mut self, shown: bool, now: u32) {
        if shown {
            self.0.frame(now);
        } else {
            self.0.text.clear();
        }
    }
}

/// One line and its shadow, as the overlays draw it: the shadow's quads a
/// pixel up and left of the line's.
pub struct OverlayLine {
    pub shadow: Vec<exa_ui::gui_text::TextQuad>,
    pub line: Vec<exa_ui::gui_text::TextQuad>,
}

/// Lays out `text` with its baseline at `y` and its left at `x`.
pub fn overlay_line(font: &exa_ui::gui_font::GuiFont, text: &str, x: i32, y: i32) -> OverlayLine {
    OverlayLine {
        shadow: exa_ui::gui_text::draw_line(font, text.as_bytes(), x - 1, y - 1),
        line: exa_ui::gui_text::draw_line(font, text.as_bytes(), x, y),
    }
}

/// The readout's place in a view `width` pixels wide: its right end the
/// border twice and a unit inside the view, its width the main font's gap
/// per byte, its baseline a unit and a line height down.
pub fn fps_line_place(
    width: i32,
    unit: i32,
    font: &exa_ui::gui_font::GuiFont,
    text: &str,
) -> (i32, i32) {
    fps_place(
        width - BORDER,
        BORDER,
        unit,
        font.gap,
        font.line_height,
        text.len(),
    )
}

/// The version line's place in a view `height` pixels tall: a unit in from
/// the left, its baseline the border and a unit up from the bottom.
pub fn version_line_place(height: i32, unit: i32) -> (i32, i32) {
    version_place(height - BORDER, unit)
}

/// The overlays' layer: over the interface, the splash and the transition
/// screen (the original draws them last in its viewport's draw), under the
/// cursor.
pub const OVERLAY_Z: i32 = 970;

pub struct ViewOverlayPlugin;

impl Plugin for ViewOverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewOverlays>()
            .init_resource::<FpsReadout>()
            .add_systems(Startup, spawn_overlay_node)
            .add_systems(PostUpdate, (follow_setting, draw_overlays).chain());
    }
}

/// The node the overlay line is drawn into. The readout and the version
/// line never show together (the title flag hides the one and shows the
/// other), so one node carries both. Only the line's own box is
/// rasterised, and only when the line or its place changes, so the window's
/// GUI canvas is left alone.
#[derive(Component, Default)]
struct OverlayNode {
    drawn: Option<DrawnLine>,
}

/// What the node shows: the text, its place, ink and unit, and the window.
#[derive(Clone, PartialEq)]
struct DrawnLine {
    text: String,
    place: (i32, i32),
    ink: [u8; 4],
    unit: i32,
    window: (u32, u32),
}

fn spawn_overlay_node(mut commands: Commands) {
    commands.spawn((
        Name::new("ViewOverlay"),
        Node {
            position_type: PositionType::Absolute,
            ..default()
        },
        ImageNode::default(),
        GlobalZIndex(OVERLAY_Z),
        Visibility::Hidden,
        OverlayNode::default(),
    ));
}

/// The ShowFPS setting's setter writes the viewport's flag at once.
fn follow_setting(show: Option<Res<exa_ui::ShowFps>>, mut overlays: ResMut<ViewOverlays>) {
    if let Some(show) = show
        && show.is_changed()
    {
        overlays.fps = show.0;
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_overlays(
    time: Res<Time<Real>>,
    overlays: Res<ViewOverlays>,
    mut readout: ResMut<FpsReadout>,
    gui: Option<Res<exa_ui::scale::GuiScale>>,
    textures: Option<Res<GuiTextures>>,
    windows: Query<&Window, With<PrimaryWindow>>,
    images: Option<ResMut<Assets<Image>>>,
    mut nodes: Query<(&mut OverlayNode, &mut ImageNode, &mut Node, &mut Visibility)>,
) {
    let now = time.elapsed().as_millis() as u32;
    let shows_fps = overlays.shows_fps();
    readout.frame(shows_fps, now);
    let Ok((mut overlay, mut image, mut node, mut visibility)) = nodes.single_mut() else {
        return;
    };
    let text = if shows_fps {
        readout.0.text.clone()
    } else if overlays.title {
        VERSION.to_string()
    } else {
        String::new()
    };
    let (Ok(window), Some(mut images)) = (windows.single(), images) else {
        return;
    };
    if text.is_empty() {
        overlay.drawn = None;
        *visibility = Visibility::Hidden;
        return;
    }
    let (width, height) = (window.physical_width(), window.physical_height());
    let unit = gui.map_or_else(|| exa_ui::scale::gui_scale(height, 0), |gui| gui.scale) as i32;
    let empty = GuiTextures::default();
    let textures = textures.as_deref().unwrap_or(&empty);
    let level = exa_ui::gui_font::level_for_unit(unit);
    let (font, atlas) =
        crate::gui_screens::screen_font(exa_ui::gui_font::gui_level(level).main, textures);
    let Some(atlas) = atlas else {
        *visibility = Visibility::Hidden;
        return;
    };
    let label = exa_ui::line_skin::shown_palette(1.0).text[OVERLAY_INK];
    let (place, ink) = if shows_fps {
        (
            fps_line_place(width as i32, unit, &font, &text),
            [label[0], label[1], label[2], FPS_ALPHA],
        )
    } else {
        (version_line_place(height as i32, unit), label)
    };
    let want = DrawnLine {
        text,
        place,
        ink,
        unit,
        window: (width, height),
    };
    if overlay.drawn.as_ref() == Some(&want) {
        return;
    }
    let line = overlay_line(&font, &want.text, place.0, place.1);
    let Some((origin, size)) = line_box(&line) else {
        return;
    };
    let mut frame = GuiFrame::new();
    for (quads, ink) in [(&line.shadow, OVERLAY_SHADOW), (&line.line, ink)] {
        let moved: Vec<_> = quads
            .iter()
            .map(|quad| {
                let mut quad = *quad;
                quad.rect[0] -= origin.0;
                quad.rect[1] -= origin.1;
                quad
            })
            .collect();
        frame.push(GuiLayer::new(
            text_batch(&font, &moved, ink, atlas),
            LayerTexture::Named(font.texture),
        ));
    }
    let pixels = exa_ui::gui_draw::rasterise(&[frame], textures, size.0, size.1);
    let picture = Image::new(
        bevy::render::render_resource::Extent3d {
            width: size.0,
            height: size.1,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        pixels,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    // A fresh picture each time, as the GUI canvas's first does: the old
    // one is dropped with its handle.
    image.image = images.add(picture);
    let scale = window.scale_factor();
    node.left = Val::Px(origin.0 as f32 / scale);
    node.top = Val::Px(origin.1 as f32 / scale);
    node.width = Val::Px(size.0 as f32 / scale);
    node.height = Val::Px(size.1 as f32 / scale);
    *visibility = Visibility::Inherited;
    debug!(
        "view overlay: {:?} at {origin:?} size {size:?} (unit {unit})",
        want.text
    );
    overlay.drawn = Some(want);
}

/// The box a line and its shadow cover, in window pixels: its top-left
/// corner and size; none for a line with no glyph.
fn line_box(line: &OverlayLine) -> Option<((i32, i32), (u32, u32))> {
    let quads = line.shadow.iter().chain(&line.line);
    let (mut left, mut top, mut right, mut bottom) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for quad in quads {
        let [x, y, w, h] = quad.rect;
        left = left.min(x);
        top = top.min(y);
        right = right.max(x + w);
        bottom = bottom.max(y + h);
    }
    (right > left && bottom > top)
        .then(|| ((left, top), ((right - left) as u32, (bottom - top) as u32)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_ui::gui_font::RAYFORM_16;

    #[test]
    fn the_title_hides_the_readout_and_shows_the_version() {
        let mut overlays = ViewOverlays::default();
        assert!(overlays.shows_fps());
        overlays.enter_title();
        assert!(overlays.title && !overlays.shows_fps());
        // The setting turned on at the title sets the flag, but the title
        // still hides the readout.
        overlays.fps = true;
        assert!(!overlays.shows_fps());
        overlays.start_session(false);
        assert!(!overlays.title && !overlays.shows_fps());
        overlays.start_session(true);
        assert!(overlays.shows_fps());
    }

    #[test]
    fn the_readout_counts_only_while_drawn() {
        let mut readout = FpsReadout::default();
        readout.frame(true, 100);
        readout.frame(true, 200);
        readout.frame(true, 300);
        // 3 frames over 0.3 s.
        assert_eq!(readout.0.text, "10.00 FPS");
        readout.frame(false, 400);
        assert!(readout.0.text.is_empty());
        assert_eq!(readout.0.since, 300);
        // Shown again: the count went on from where it stood.
        readout.frame(true, 500);
        assert!(readout.0.text.is_empty());
        readout.frame(true, 600);
        // 2 frames over 0.3 s since the last reading.
        assert_eq!(readout.0.text, "6.67 FPS");
    }

    #[test]
    fn the_places_follow_the_view() {
        // A 2560 x 1440 view at unit 8 with the 16-point font (gap 17,
        // line 36): "60.00 FPS" is nine bytes.
        let text = "60.00 FPS";
        assert_eq!(
            fps_line_place(2560, 8, &RAYFORM_16, text),
            (2560 - 8 - 8 - 8 - 17 * 9, 8 + 36)
        );
        assert_eq!(version_line_place(1440, 8), (8, 1440 - 8 - 8));
    }

    #[test]
    fn the_box_covers_the_line_and_its_shadow() {
        let line = overlay_line(&RAYFORM_16, "V", 10, 50);
        let ((x, y), (w, h)) = line_box(&line).unwrap();
        assert_eq!((x, y), (9, 50 - 1 - RAYFORM_16.glyph.1));
        assert_eq!(
            (w, h),
            (RAYFORM_16.glyph.0 as u32 + 1, RAYFORM_16.glyph.1 as u32 + 1)
        );
        assert!(line_box(&overlay_line(&RAYFORM_16, "", 10, 50)).is_none());
    }

    #[test]
    fn the_shadow_sits_a_pixel_up_and_left() {
        let line = overlay_line(&RAYFORM_16, "V", 10, 50);
        assert_eq!(line.shadow[0].rect[0] + 1, line.line[0].rect[0]);
        assert_eq!(line.shadow[0].rect[1] + 1, line.line[0].rect[1]);
    }
}

//! The in-game world cursor: a flat sprite from `cursors.rfi` drawn over the
//! game's view at the pointer's position, in place of the system pointer.
//! The game picks its shape and says whether combat mode is on
//! ([`WorldCursor`]); this module draws it as the original does. Rules:
//! `docs/UI.md` (World cursor).

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::{CursorOptions, PrimaryWindow};
use exa_assets::Game;
use exa_formats::rfi::Rfi;

use crate::LoadingStatus;
use crate::scale::GuiScale;

/// The cursor atlas: 2 columns × 4 rows of square frames, coverage only.
pub const CURSORS: &str = "cursors";
/// Frames per row and per column.
const COLUMNS: usize = 2;
const ROWS: usize = 4;
/// The original's cursor size setting (a factor on the GUI scale).
const CURSOR_SIZE: f32 = 1.0;
/// The largest sprite, in physical pixels.
const MAX_SIZE: u32 = 128;
/// In combat mode: red, with a translucent fill.
pub const COMBAT_COLOUR: [u8; 4] = [255, 76, 76, 180];
/// Outside combat mode: amber.
pub const EXPLORE_COLOUR: [u8; 4] = [255, 205, 76, 255];
/// The pointer frame is turned this far counter-clockwise (degrees) and
/// moved right and down by these fractions of the sprite's size, so its tip
/// sits on the pointer's position.
const POINTER_TURN: f32 = 33.0;
const POINTER_SHIFT: (f64, f64) = (0.275, 0.4);
/// The outline: four black copies one physical pixel left, up, down and
/// right, drawn under the sprite.
const OUTLINE: [(f32, f32); 4] = [(-1.0, 0.0), (0.0, -1.0), (0.0, 1.0), (1.0, 0.0)];
/// Over the rest of the interface, under nothing.
const Z: i32 = 1000;
/// The original blends its interface in gamma space (sRGB values); Bevy
/// blends in linear light. The opacities are remapped through this power
/// so that the blend comes out as the original's (see [`fill_opacity`] and
/// [`outline_opacity`]).
const BLEND_GAMMA: f32 = 2.2;

/// The cursor's shapes, by their frame in the atlas.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum CursorShape {
    /// A disc with a bright rim: the cursor in combat mode.
    #[default]
    Ring,
    /// One chevron: walking to the cursor.
    Chevron,
    /// Two chevrons: running to the cursor (beyond 3 m).
    DoubleChevron,
    /// An arrowhead, tilted as a system pointer.
    Pointer,
    /// A four-armed curved cross: nothing to do there.
    Cross,
    /// Four chevrons pointing in.
    Crosshair,
}

impl CursorShape {
    pub const ALL: [Self; 6] = [
        Self::Ring,
        Self::Chevron,
        Self::DoubleChevron,
        Self::Pointer,
        Self::Cross,
        Self::Crosshair,
    ];

    /// The shape's frame in the atlas as stored: (row, column), the first
    /// stored row first. The stored rows run from the bottom of the drawn
    /// sprite to its top.
    pub fn frame(self) -> (usize, usize) {
        match self {
            Self::Ring => (3, 0),
            Self::Chevron => (2, 0),
            Self::DoubleChevron => (1, 0),
            Self::Pointer => (0, 0),
            Self::Cross => (1, 1),
            Self::Crosshair => (0, 1),
        }
    }

    /// Whether the sprite is turned so that its top points away from the
    /// player on screen.
    pub fn follows_player(self) -> bool {
        matches!(self, Self::Ring | Self::Chevron | Self::DoubleChevron)
    }
}

/// What the game shows with the world cursor this frame.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct WorldCursor {
    /// The game's view is up with a player to steer: the cursor replaces
    /// the system pointer.
    pub active: bool,
    pub shape: CursorShape,
    /// Combat mode: red, else amber.
    pub combat: bool,
    /// The player on screen (logical pixels), which the ring and chevrons
    /// turn away from.
    pub player: Option<Vec2>,
}

/// A fixed pointer position (logical window pixels) that stands in for the
/// mouse's, for unattended runs.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct ScriptedPointer(pub Option<Vec2>);

/// The player has captured the mouse (Cursor Look): the system pointer and
/// the world cursor stay hidden until it is released. The game sets this;
/// the cursor draw obeys it.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CursorCapture(pub bool);

impl ScriptedPointer {
    /// Where the pointer is in the window: the scripted position, else the
    /// mouse's while it is over the window.
    pub fn position(&self, window: &Window) -> Option<Vec2> {
        self.0.or_else(|| window.cursor_position())
    }
}

/// The system that draws the cursor; the game sets [`WorldCursor`] before it.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct DrawWorldCursor;

/// The sprite's size in physical pixels at a GUI scale: 24 at 4, 30 at 5.
pub fn cursor_size(gui_scale: u32) -> u32 {
    let size = ((3 * gui_scale) as f32 * CURSOR_SIZE).round_ties_even() as u32 * 2;
    size.min(MAX_SIZE)
}

/// The cursor's colour: red in combat mode (opaque once a shape other than
/// the ring or the pointer has been drawn in it), amber outside it.
pub fn colour(combat: bool, opaque_red: bool) -> [u8; 4] {
    match (combat, opaque_red) {
        (true, false) => COMBAT_COLOUR,
        (true, true) => [COMBAT_COLOUR[0], COMBAT_COLOUR[1], COMBAT_COLOUR[2], 255],
        (false, _) => EXPLORE_COLOUR,
    }
}

/// The outline's opacity for a colour's: its square.
pub fn outline_alpha(alpha: u8) -> u8 {
    let a = alpha as f32 / 255.0;
    (a * a * 255.0).round() as u8
}

/// The opacity (0–1) that blends a colour in linear light as the original
/// blends it in gamma space with `coverage` × `alpha` (both 0–255): exact
/// over black, close over the game's dark floors. It is the product of the
/// two remapped, so the coverage goes in the image and the alpha in the
/// tint.
pub fn fill_opacity(coverage: u8, alpha: u8) -> f32 {
    (coverage as f32 / 255.0 * alpha as f32 / 255.0).powf(BLEND_GAMMA)
}

/// The opacity (0–1) that darkens in linear light as the original's black
/// outline darkens in gamma space with `coverage` × `alpha`: exact over any
/// background.
pub fn outline_opacity(coverage: u8, alpha: u8) -> f32 {
    1.0 - (1.0 - coverage as f32 / 255.0 * alpha as f32 / 255.0).powf(BLEND_GAMMA)
}

/// The sprite's clockwise turn (rad, screen y down) and its centre's shift
/// from the pointer (physical pixels) for a shape of `size` pixels; `away`
/// is the screen direction from the player to the pointer.
pub fn placement(shape: CursorShape, size: u32, away: Option<Vec2>) -> (f32, Vec2) {
    match shape {
        CursorShape::Pointer => {
            let shift = Vec2::new(
                (size as f64 * POINTER_SHIFT.0).round() as f32,
                (size as f64 * POINTER_SHIFT.1).round() as f32,
            );
            (-POINTER_TURN.to_radians(), shift)
        }
        s if s.follows_player() => {
            let turn = away
                .and_then(|d| d.try_normalize())
                .map_or(0.0, |d| d.x.atan2(-d.y));
            (turn, Vec2::ZERO)
        }
        _ => (0.0, Vec2::ZERO),
    }
}

/// The decoded atlas: coverage as stored, two frames wide and four tall.
#[derive(Clone, Debug)]
pub struct CursorAtlas {
    cell: usize,
    alpha: Vec<u8>,
}

impl CursorAtlas {
    /// Reads the atlas from its texture; `None` unless it is two square
    /// frames wide and four tall.
    pub fn from_rfi(rfi: &Rfi) -> Option<Self> {
        let rgba = rfi.decode_rgba8().ok()?;
        let alpha = rgba.iter().skip(3).step_by(4).copied().collect();
        Self::from_alpha(rfi.header.width as usize, rfi.header.height as usize, alpha)
    }

    /// Builds the atlas from stored coverage, row-major.
    pub fn from_alpha(width: usize, height: usize, alpha: Vec<u8>) -> Option<Self> {
        let cell = width / COLUMNS;
        if cell == 0 || width != cell * COLUMNS || height != cell * ROWS {
            return None;
        }
        (alpha.len() == width * height).then_some(Self { cell, alpha })
    }

    /// A frame's size as stored, in pixels.
    pub fn cell(&self) -> usize {
        self.cell
    }

    /// A shape's coverage upright as drawn (the stored rows bottom to top),
    /// resampled to `size` pixels square.
    pub fn frame(&self, shape: CursorShape, size: usize) -> Vec<u8> {
        let (row, column) = shape.frame();
        let width = self.cell * COLUMNS;
        let mut cell = Vec::with_capacity(self.cell * self.cell);
        for y in (0..self.cell).rev() {
            let start = (row * self.cell + y) * width + column * self.cell;
            cell.extend_from_slice(&self.alpha[start..start + self.cell]);
        }
        resample(&cell, self.cell, size)
    }
}

/// Resamples a square coverage image from `from` to `to` pixels a side,
/// each new pixel the mean of the area it covers.
pub fn resample(alpha: &[u8], from: usize, to: usize) -> Vec<u8> {
    if from == 0 || to == 0 {
        return Vec::new();
    }
    // Each new pixel's share of the old ones along one axis.
    let weights: Vec<Vec<(usize, f32)>> = (0..to)
        .map(|i| {
            let (a, b) = (
                i as f32 * from as f32 / to as f32,
                (i + 1) as f32 * from as f32 / to as f32,
            );
            (a.floor() as usize..(b.ceil() as usize).min(from))
                .map(|j| {
                    let w = (b.min(j as f32 + 1.0) - a.max(j as f32)).max(0.0);
                    (j, w / (b - a))
                })
                .collect()
        })
        .collect();
    let mut rows = vec![0.0f32; from * to];
    for y in 0..from {
        for (x, share) in weights.iter().enumerate() {
            rows[y * to + x] = share
                .iter()
                .map(|&(j, w)| alpha[y * from + j] as f32 * w)
                .sum();
        }
    }
    let mut out = vec![0u8; to * to];
    for (y, share) in weights.iter().enumerate() {
        for x in 0..to {
            let v: f32 = share.iter().map(|&(j, w)| rows[j * to + x] * w).sum();
            out[y * to + x] = v.round().clamp(0.0, 255.0) as u8;
        }
    }
    out
}

/// The atlas, when the installation has one.
#[derive(Resource)]
struct Atlas(Option<CursorAtlas>);

/// The frames at the current size (physical pixels): their coverage, the
/// sprite's images, and the outline's images by outline alpha.
#[derive(Resource, Default)]
struct Frames {
    size: u32,
    coverage: Vec<(CursorShape, Vec<u8>)>,
    fill: Vec<(CursorShape, Handle<Image>)>,
    outline: Vec<((CursorShape, u8), Handle<Image>)>,
}

impl Frames {
    /// Cuts the frames from the atlas at `size` pixels.
    fn rebuild(&mut self, atlas: &CursorAtlas, size: u32, images: &mut Assets<Image>) {
        for (_, h) in self
            .fill
            .drain(..)
            .chain(self.outline.drain(..).map(|(k, h)| (k.0, h)))
        {
            images.remove(&h);
        }
        self.size = size;
        self.coverage = CursorShape::ALL
            .iter()
            .map(|&shape| (shape, atlas.frame(shape, size as usize)))
            .collect();
        self.fill = self
            .coverage
            .iter()
            .map(|(shape, alpha)| {
                let image = coverage_image(size, alpha, |c| fill_opacity(c, 255));
                (*shape, images.add(image))
            })
            .collect();
    }

    fn fill(&self, shape: CursorShape) -> Handle<Image> {
        self.fill
            .iter()
            .find(|(s, _)| *s == shape)
            .map(|(_, h)| h.clone())
            .unwrap_or_default()
    }

    /// The outline's image for a shape and an outline alpha, made the first
    /// time it is asked for.
    fn outline(
        &mut self,
        shape: CursorShape,
        alpha: u8,
        images: &mut Assets<Image>,
    ) -> Handle<Image> {
        if let Some((_, h)) = self.outline.iter().find(|(k, _)| *k == (shape, alpha)) {
            return h.clone();
        }
        let Some((_, coverage)) = self.coverage.iter().find(|(s, _)| *s == shape) else {
            return Handle::default();
        };
        let image = coverage_image(self.size, coverage, |c| outline_opacity(c, alpha));
        let handle = images.add(image);
        self.outline.push(((shape, alpha), handle.clone()));
        handle
    }
}

/// One of the five copies of the sprite: an outline copy's offset, or the
/// sprite itself.
#[derive(Component, Clone, Copy)]
struct Part(Option<(f32, f32)>);

/// Whether the red has turned opaque.
#[derive(Resource, Default)]
struct OpaqueRed(bool);

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<WorldCursor>()
        .init_resource::<ScriptedPointer>()
        .init_resource::<CursorCapture>()
        .init_resource::<Frames>()
        .init_resource::<OpaqueRed>()
        .add_systems(Startup, spawn)
        .add_systems(Update, draw.in_set(DrawWorldCursor));
}

fn spawn(mut commands: Commands, game: Option<Res<Game>>) {
    let Some(game) = game else { return };
    let atlas = game
        .0
        .texture(CURSORS)
        .and_then(|b| Rfi::parse(b).ok())
        .and_then(|rfi| CursorAtlas::from_rfi(&rfi));
    if atlas.is_none() {
        warn!("{CURSORS} missing: no world cursor");
    }
    commands.insert_resource(Atlas(atlas));
    for part in OUTLINE.map(Some).into_iter().chain([None]) {
        commands.spawn((
            Name::new("WorldCursor"),
            Part(part),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            ImageNode::default(),
            UiTransform::default(),
            GlobalZIndex(Z + part.is_none() as i32),
            Visibility::Hidden,
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn draw(
    cursor: Res<WorldCursor>,
    scripted: Res<ScriptedPointer>,
    capture: Res<CursorCapture>,
    gui: Res<GuiScale>,
    atlas: Option<Res<Atlas>>,
    loading: Option<Res<LoadingStatus>>,
    mut frames: ResMut<Frames>,
    mut opaque: ResMut<OpaqueRed>,
    mut images: ResMut<Assets<Image>>,
    mut windows: Query<(&Window, &mut CursorOptions), With<PrimaryWindow>>,
    mut parts: Query<(
        &Part,
        &mut Node,
        &mut ImageNode,
        &mut UiTransform,
        &mut Visibility,
    )>,
) {
    let Ok((window, mut options)) = windows.single_mut() else {
        return;
    };
    let atlas = atlas.as_ref().and_then(|a| a.0.as_ref());
    let at = scripted.position(window);
    let shown = cursor.active && loading.is_none() && atlas.is_some() && at.is_some();
    // Cursor Look owns the pointer: hide both the system and the world
    // cursor until it is released.
    let visible = shown && !capture.0;
    if options.visible == visible {
        options.visible = !visible;
    }
    if capture.0 {
        for (.., mut visibility) in &mut parts {
            *visibility = Visibility::Hidden;
        }
        return;
    }
    let (Some(atlas), Some(at), true) = (atlas, at, shown) else {
        for (.., mut visibility) in &mut parts {
            *visibility = Visibility::Hidden;
        }
        return;
    };
    let size = cursor_size(gui.scale);
    if frames.size != size {
        info!(
            "world cursor: {size} px at GUI scale {} ({} px tall window)",
            gui.scale,
            window.physical_height()
        );
        frames.rebuild(atlas, size, &mut images);
    }
    // Logical pixels per physical pixel: the original draws in physical
    // pixels whatever the display's scale factor.
    let unit = 1.0 / window.scale_factor();
    let away = cursor.player.map(|p| at - p);
    let (turn, shift) = placement(cursor.shape, size, away);
    let centre = at + shift * unit;
    let rgba = colour(cursor.combat, opaque.0);
    let [r, g, b] = [rgba[0], rgba[1], rgba[2]].map(|c| c as f32 / 255.0);
    let tint = Color::srgba(r, g, b, fill_opacity(255, rgba[3]));
    let side = size as f32 * unit;
    let fill = frames.fill(cursor.shape);
    let edge = frames.outline(cursor.shape, outline_alpha(rgba[3]), &mut images);
    for (part, mut node, mut sprite, mut transform, mut visibility) in &mut parts {
        let offset = part.0.map_or(Vec2::ZERO, |(x, y)| Vec2::new(x, y) * unit);
        let corner = centre + offset - Vec2::splat(side / 2.0);
        node.left = px(corner.x);
        node.top = px(corner.y);
        node.width = px(side);
        node.height = px(side);
        let (image, color) = match part.0 {
            Some(_) => (&edge, Color::BLACK),
            None => (&fill, tint),
        };
        if sprite.image != *image {
            sprite.image = image.clone();
        }
        sprite.color = color;
        transform.rotation = Rot2::radians(turn);
        *visibility = Visibility::Inherited;
    }
    // The original turns its red opaque for good once any shape but the
    // ring or the pointer is drawn in combat mode (from the next frame).
    if cursor.combat && !matches!(cursor.shape, CursorShape::Ring | CursorShape::Pointer) {
        opaque.0 = true;
    }
}

/// White with an opacity per pixel from its coverage; the node's colour
/// tints it.
fn coverage_image(size: u32, coverage: &[u8], opacity: impl Fn(u8) -> f32) -> Image {
    let data = coverage
        .iter()
        .flat_map(|&c| [255, 255, 255, (opacity(c) * 255.0).round() as u8])
        .collect();
    Image::new(
        Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_size_follows_the_gui_scale() {
        // 3 × scale × size factor, rounded, doubled, at most 128.
        assert_eq!(cursor_size(3), 18);
        assert_eq!(cursor_size(4), 24);
        assert_eq!(cursor_size(5), 30);
        assert_eq!(cursor_size(12), 72);
        assert_eq!(cursor_size(30), 128);
    }

    #[test]
    fn red_in_combat_amber_outside() {
        assert_eq!(colour(true, false), [255, 76, 76, 180]);
        assert_eq!(colour(true, true), [255, 76, 76, 255]);
        assert_eq!(colour(false, false), [255, 205, 76, 255]);
        assert_eq!(colour(false, true), [255, 205, 76, 255]);
        // The outline: the colour's opacity squared.
        assert_eq!(outline_alpha(180), 127);
        assert_eq!(outline_alpha(255), 255);
    }

    #[test]
    fn opacities_blend_as_in_gamma_space() {
        let srgb = |l: f32| l.powf(1.0 / BLEND_GAMMA);
        let lin = |s: f32| s.powf(BLEND_GAMMA);
        // The red at full coverage over black: 180/255 of its colour.
        let o = fill_opacity(255, 180);
        assert!((srgb(o) - 180.0 / 255.0).abs() < 1e-4);
        assert_eq!(fill_opacity(0, 255), 0.0);
        assert_eq!(fill_opacity(255, 255), 1.0);
        // The image and the tint multiply.
        let product = fill_opacity(128, 255) * fill_opacity(255, 180);
        assert!((product - fill_opacity(128, 180)).abs() < 1e-5);
        // The red outline (alpha 127) at full coverage over a floor of 100
        // leaves 100 × (1 − 127/255) in gamma space, whatever the floor.
        for floor in [30.0f32, 100.0, 200.0] {
            let o = outline_opacity(255, 127);
            let out = srgb(lin(floor / 255.0) * (1.0 - o)) * 255.0;
            assert!(
                (out - floor * (1.0 - 127.0 / 255.0)).abs() < 0.05,
                "{floor}: {out}"
            );
        }
        assert_eq!(outline_opacity(0, 255), 0.0);
        assert_eq!(outline_opacity(255, 255), 1.0);
    }

    #[test]
    fn frames_sit_where_the_atlas_has_them() {
        assert_eq!(CursorShape::Ring.frame(), (3, 0));
        assert_eq!(CursorShape::Chevron.frame(), (2, 0));
        assert_eq!(CursorShape::DoubleChevron.frame(), (1, 0));
        assert_eq!(CursorShape::Pointer.frame(), (0, 0));
        assert_eq!(CursorShape::Cross.frame(), (1, 1));
        assert_eq!(CursorShape::Crosshair.frame(), (0, 1));
        let turned: Vec<_> = CursorShape::ALL
            .into_iter()
            .filter(|s| s.follows_player())
            .collect();
        assert_eq!(
            turned,
            [
                CursorShape::Ring,
                CursorShape::Chevron,
                CursorShape::DoubleChevron
            ]
        );
    }

    #[test]
    fn the_pointer_is_tilted_with_its_tip_on_the_position() {
        let (turn, shift) = placement(CursorShape::Pointer, 30, Some(Vec2::X));
        assert!((turn.to_degrees() + 33.0).abs() < 1e-4);
        assert_eq!(shift, Vec2::new(8.0, 12.0));
        // The tip, half a sprite above the centre before the turn, lands
        // within a pixel of the pointer's position.
        let tip = Rot2::radians(turn) * Vec2::new(0.0, -15.0) + shift;
        assert!(tip.length() < 1.0, "{tip}");
        assert_eq!(
            placement(CursorShape::Cross, 30, Some(Vec2::X)),
            (0.0, Vec2::ZERO)
        );
    }

    #[test]
    fn chevrons_point_away_from_the_player() {
        // Pointer to the right of the player: the sprite's top turns right.
        let (turn, shift) = placement(CursorShape::Chevron, 30, Some(Vec2::new(5.0, 0.0)));
        assert!((turn - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        assert_eq!(shift, Vec2::ZERO);
        // Pointer below the player: upside down.
        let (turn, _) = placement(CursorShape::DoubleChevron, 30, Some(Vec2::new(0.0, 3.0)));
        assert!((turn.abs() - std::f32::consts::PI).abs() < 1e-5);
        // Above: upright; on the player: upright.
        assert_eq!(
            placement(CursorShape::Ring, 30, Some(Vec2::new(0.0, -4.0))).0,
            0.0
        );
        assert_eq!(placement(CursorShape::Ring, 30, Some(Vec2::ZERO)).0, 0.0);
    }

    /// A 4 × 8 atlas (2-pixel frames) whose stored pixels are numbered.
    fn atlas() -> CursorAtlas {
        CursorAtlas::from_alpha(4, 8, (0..32).collect()).unwrap()
    }

    #[test]
    fn frames_are_cut_and_turned_upright() {
        let atlas = atlas();
        assert_eq!(atlas.cell(), 2);
        // The ring: stored rows 6 and 7, left column, bottom row first.
        assert_eq!(atlas.frame(CursorShape::Ring, 2), [28, 29, 24, 25]);
        // The cross: stored rows 2 and 3, right column.
        assert_eq!(atlas.frame(CursorShape::Cross, 2), [14, 15, 10, 11]);
        assert_eq!(atlas.frame(CursorShape::Pointer, 2), [4, 5, 0, 1]);
        assert!(CursorAtlas::from_alpha(4, 6, vec![0; 24]).is_none());
        assert!(CursorAtlas::from_alpha(4, 8, vec![0; 31]).is_none());
        assert!(CursorAtlas::from_alpha(0, 0, vec![]).is_none());
    }

    #[test]
    fn resampling_averages_the_area() {
        let alpha = [
            0, 100, 200, 40, 10, 50, 0, 60, 255, 255, 0, 0, 255, 255, 0, 0,
        ];
        assert_eq!(resample(&alpha, 4, 2), [40, 75, 255, 0]);
        assert_eq!(resample(&alpha, 4, 4), alpha);
        // 3 to 2: each new pixel covers one and a half old ones.
        let third = [90, 0, 30, 90, 0, 30, 90, 0, 30];
        assert_eq!(resample(&third, 3, 2), [60, 20, 60, 20]);
    }

    /// The installed atlas has the six shapes where the original reads
    /// them, and two empty frames (skipped without `EXANIMA_DIR`).
    #[test]
    fn the_installed_atlas_has_six_shapes() {
        let Some(dir) = exa_assets::locate_install(None) else {
            return;
        };
        let game = exa_assets::GameData::open(&dir).unwrap();
        let rfi = Rfi::parse(game.texture(CURSORS).unwrap()).unwrap();
        assert_eq!((rfi.header.width, rfi.header.height), (256, 512));
        let atlas = CursorAtlas::from_rfi(&rfi).unwrap();
        assert_eq!(atlas.cell(), 128);
        let inked = |row: usize, column: usize| {
            (0..128)
                .flat_map(|y| (0..128).map(move |x| (y, x)))
                .filter(|&(y, x)| atlas.alpha[(row * 128 + y) * 256 + column * 128 + x] > 0)
                .count()
        };
        for shape in CursorShape::ALL {
            let (row, column) = shape.frame();
            assert!(inked(row, column) > 2000, "{shape:?}");
        }
        assert_eq!(inked(2, 1), 0);
        assert_eq!(inked(3, 1), 0);
        // Drawn upright, the pointer's arrowhead and the chevrons have
        // their tips at the top: the top rows are narrower than the rows
        // further down.
        for shape in [
            CursorShape::Pointer,
            CursorShape::Chevron,
            CursorShape::DoubleChevron,
        ] {
            let frame = atlas.frame(shape, 128);
            let span = |y: usize| {
                let inked: Vec<usize> = (0..128).filter(|&x| frame[y * 128 + x] > 16).collect();
                inked.last().map_or(0, |last| last + 1 - inked[0])
            };
            let top = (0..128).find(|&y| span(y) > 0).unwrap();
            assert!(
                span(top + 2) * 3 < span(top + 30),
                "{shape:?}: top at row {top}"
            );
        }
        // The ring is round: as wide as it is tall.
        let ring = atlas.frame(CursorShape::Ring, 128);
        let rows = (0..128).filter(|&y| (0..128).any(|x| ring[y * 128 + x] > 16));
        let columns = (0..128).filter(|&x| (0..128).any(|y| ring[y * 128 + x] > 16));
        assert!((rows.count() as i32 - columns.count() as i32).abs() <= 2);
    }
}

//! The offscreen icon renderer (`FLX_IconRender`): a private scene and 3-D
//! view that draw an item or a character into a render target twice the
//! icon's size, then read the pixels back into an RGBA image the icon atlas
//! uploads.
//!
//! The private scene is created with scene mode 1, an ambient multiplier
//! of 100 (a stored 1.0 times 100) and a second
//! scene value of 0.5 (not traced further). Its environment record is the
//! icon preset, whose ambient is `(0.35, 0.36, 0.37)` (the record's other
//! fields are not in the evidence), and the record's four words after it
//! are then set to `(0, 0, 0, 1)` ([`GLOBAL_LIGHT`]). The view takes a 30°
//! field of view and a 10-unit (10 cm) near plane; it also stores 0.8 into
//! one view field and passes `0x3f4ccc01` (just under 0.8) to a view
//! setter, which Exarust reads as the bloom (a guess), and clears one view
//! flag, read as the sharpen (a guess). The render target is twice the
//! icon's width and height: its colour buffer is `GL_SRGB8`, or
//! `GL_SRGB8_ALPHA8` when transparency is wanted (characters cut out, the
//! view's own format word then RGBA8), with a `GL_DEPTH_COMPONENT` depth
//! buffer. The destination image is RGBA8 with flags `0x2000_0000` at the
//! icon's own size.
//!
//! The renderer keeps one instance and reuses it: a new icon size only
//! resizes the image, the render target and the view. Character creation
//! walks its list of object nodes, rendering one per step (a worker drives
//! the same step), and stops the list when it is done. The render-and-read
//! into the destination image is the view's capture call; the readback's
//! row order ([`IconPixels`], top row first) is Exarust's assumption, and
//! `exa_ui::icon_atlas::IconImage::from_icon_pixels` flips it into the
//! atlas's bottom-up rows. Nothing in the game spawns this renderer yet;
//! the item renderer (`exa_items::item_renderer`) is the live icon path.
//!
//! Rules: `docs/RENDERING.md` (Icons).

use bevy::prelude::*;

/// The private view's field of view, degrees.
pub const FOV_DEGREES: f32 = 30.0;
/// The view's near plane in engine units (centimetres): 10, i.e. 10 cm.
pub const NEAR_ENGINE_UNITS: f32 = 10.0;
/// The render target is this many times the icon's side.
pub const TARGET_SCALE: u32 = 2;
/// The view value read as the bloom strength (a guess: the set-up stores
/// 0.8 there).
pub const BLOOM: f32 = 0.8;
/// The view flag read as the sharpen (a guess: the set-up clears it).
pub const SHARPEN: bool = false;
/// The scene's ambient multiplier: the preset ambient times 100.
pub const AMBIENT_MULTIPLIER: f32 = 100.0;
/// The icon preset's ambient colour.
pub const AMBIENT_COLOUR: [f32; 3] = [0.35, 0.36, 0.37];
/// The four environment words the set-up writes after applying the preset
/// (their reader is not traced).
pub const GLOBAL_LIGHT: [f32; 4] = [0.0, 0.0, 0.0, 1.0];
/// The destination image's format word (RGBA8) and flags.
pub const DESTINATION_FORMAT: u32 = 0x0100_E608;
pub const DESTINATION_FLAGS: u32 = 0x2000_0000;
/// The view's own format word when it renders a transparent icon.
pub const VIEW_FORMAT_TRANSPARENT: u32 = 0x0100_E608;
/// The depth renderbuffer's internal format (`GL_DEPTH_COMPONENT`).
pub const DEPTH_FORMAT: u32 = 0x1902;

/// The colour renderbuffer's internal format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColourTarget {
    /// `GL_SRGB8`: an opaque icon.
    Srgb8,
    /// `GL_SRGB8_ALPHA8`: an icon with a cut-out.
    Srgb8Alpha8,
}

impl ColourTarget {
    /// The renderbuffer format the icon's transparency selects: characters
    /// (transparent) take alpha, items do not.
    pub fn for_icon(transparent: bool) -> Self {
        if transparent {
            Self::Srgb8Alpha8
        } else {
            Self::Srgb8
        }
    }

    /// The GL internal-format word.
    pub fn word(self) -> u32 {
        match self {
            Self::Srgb8 => 0x8C41,
            Self::Srgb8Alpha8 => 0x8C43,
        }
    }
}

/// A created icon renderer's fixed set-up: private scene, view and target
/// sizes. The renderer is reused between icons.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct IconRender {
    /// The icon's side in pixels (the destination image's side).
    pub icon_size: u32,
    /// The character cut-out target.
    pub transparent: bool,
}

impl IconRender {
    /// The renderer for an icon of `icon_size` pixels, as the first icon
    /// creates it: a private scene and view at twice the size, with the
    /// icon preset applied.
    pub fn new(icon_size: u32, transparent: bool) -> Self {
        Self {
            icon_size,
            transparent,
        }
    }

    /// The render target's side: twice the icon's.
    pub fn target_side(&self) -> u32 {
        self.icon_size * TARGET_SCALE
    }

    /// The colour renderbuffer format.
    pub fn colour_target(&self) -> ColourTarget {
        ColourTarget::for_icon(self.transparent)
    }

    /// The destination image's format word and flags.
    pub fn destination(&self) -> (u32, u32) {
        (DESTINATION_FORMAT, DESTINATION_FLAGS)
    }

    /// A resized icon: only the sizes change; the scene and view stay.
    pub fn resize(&mut self, icon_size: u32) {
        self.icon_size = icon_size;
    }

    /// The private view's camera set-up, in metres: the field of view
    /// converted from degrees, the near plane from centimetres, and the
    /// target's size.
    pub fn camera(&self) -> IconCamera {
        IconCamera {
            fov_radians: FOV_DEGREES.to_radians(),
            near_metres: NEAR_ENGINE_UNITS * 0.01,
            bloom: BLOOM,
            sharpen: SHARPEN,
            width: self.target_side(),
            height: self.target_side(),
        }
    }
}

/// A private icon view's camera values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconCamera {
    pub fov_radians: f32,
    pub near_metres: f32,
    pub bloom: f32,
    pub sharpen: bool,
    pub width: u32,
    pub height: u32,
}

/// A rendered icon's pixels, tightly packed RGBA8, top row first as the
/// framebuffer read returns them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconPixels {
    pub side: u32,
    pub rgba: Vec<u8>,
}

impl IconPixels {
    /// The pixels as the icon atlas stores them: rows from the bottom up.
    pub fn flip_rows(&self) -> Vec<u8> {
        let stride = self.side as usize * 4;
        let mut out = Vec::with_capacity(self.rgba.len());
        for row in (0..self.side as usize).rev() {
            let start = row * stride;
            out.extend_from_slice(&self.rgba[start..start + stride]);
        }
        out
    }
}

/// One icon's render step: which object in the renderer's list is drawn,
/// and whether the list is finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconStep {
    /// Draw the object at this 0-based index.
    Draw(usize),
    /// Every object has been drawn; clean the private scene up.
    Done,
}

/// The renderer's walk over its object list. The original starts at index
/// −1 and increments before each step, so the first step draws object 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconWalk {
    pub next: i32,
    pub count: usize,
}

impl IconWalk {
    pub fn new(count: usize) -> Self {
        Self { next: -1, count }
    }

    /// The next step. A single synchronous icon draws once; an asynchronous
    /// one walks the list and runs [`IconStep::Done`] after the last.
    pub fn step(&mut self) -> IconStep {
        self.next += 1;
        if self.next < self.count as i32 {
            IconStep::Draw(self.next as usize)
        } else {
            IconStep::Done
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_opaque_icon_renders_twice_its_size_in_srgb() {
        let render = IconRender::new(64, false);
        assert_eq!(render.target_side(), 128);
        assert_eq!(render.colour_target(), ColourTarget::Srgb8);
        assert_eq!(render.colour_target().word(), 0x8C41);
        assert_eq!(render.destination(), (0x0100_E608, 0x2000_0000));
        let camera = render.camera();
        assert_eq!(camera.width, 128);
        assert_eq!(camera.height, 128);
        assert_eq!(camera.fov_radians, 30.0f32.to_radians());
        assert!((camera.near_metres - 0.1).abs() < 1e-6);
        assert_eq!(camera.bloom, 0.8);
        assert!(!camera.sharpen);
    }

    #[test]
    fn a_character_icon_cuts_out_with_an_alpha_target() {
        let render = IconRender::new(128, true);
        assert_eq!(render.target_side(), 256);
        assert_eq!(render.colour_target(), ColourTarget::Srgb8Alpha8);
        assert_eq!(render.colour_target().word(), 0x8C43);
        assert_eq!(DEPTH_FORMAT, 0x1902);
    }

    #[test]
    fn a_resize_keeps_the_scene_and_changes_only_the_sizes() {
        let mut render = IconRender::new(64, false);
        render.resize(200);
        assert_eq!(render.icon_size, 200);
        assert_eq!(render.target_side(), 400);
        assert_eq!(render.camera().width, 400);
    }

    #[test]
    fn the_walk_draws_every_object_once_and_then_finishes() {
        let mut walk = IconWalk::new(3);
        assert_eq!(walk.step(), IconStep::Draw(0));
        assert_eq!(walk.step(), IconStep::Draw(1));
        assert_eq!(walk.step(), IconStep::Draw(2));
        assert_eq!(walk.step(), IconStep::Done);
        assert_eq!(walk.step(), IconStep::Done);
    }

    #[test]
    fn readback_rows_flip() {
        let side = 2;
        let rgba = vec![
            1, 0, 0, 255, 2, 0, 0, 255, // top
            3, 0, 0, 255, 4, 0, 0, 255, // bottom
        ];
        let pixels = IconPixels { side, rgba };
        assert_eq!(
            pixels.flip_rows(),
            vec![3, 0, 0, 255, 4, 0, 0, 255, 1, 0, 0, 255, 2, 0, 0, 255]
        );
    }
}

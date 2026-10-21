//! The off-screen icon renderer: a private scene, a view and two
//! renderbuffers at twice the icon size, with the full-size and the final
//! RGBA images the item GUI samples.
//!
//! The icon size is a global that starts at 160 pixels; the renderbuffers
//! and the first image are twice that and the second image is the icon
//! size. A renderbuffer holds the depth (`GL_DEPTH_COMPONENT`, attached as
//! the depth attachment) and one the colour (sRGB with alpha, or without
//! while the alpha global is clear; attached as the first colour
//! attachment). Each image is the raw RGBA payload behind a 32-byte header.
//!
//! The scene's sun points along (0.46, 0.47, 0.48) and has the colour
//! (0.75, 0.75, 0.75, 0.5); the view's camera sits at (0, 0, 350) looking
//! at the origin, 40 away, with the setting 10 after the distance.
//!
//! Evidence: the renderer's constructor, its image allocator and its
//! destroy (`docs/RENDERING.md`, item icons).

/// The icon size global's start value, in pixels.
pub const ICON_SIZE: i32 = 160;
/// The render size: twice the icon size.
pub const BUFFER_SIZE: i32 = ICON_SIZE * 2;
/// The image header's size.
pub const IMAGE_HEADER: u32 = 0x20;
/// The image's texture format and flag word.
pub const IMAGE_FORMAT: u32 = 0x0100_e608;
pub const IMAGE_FLAGS: u32 = 0x2000_0000;
/// The header word the allocator sets to 1.
pub const IMAGE_PLANES: u16 = 1;
/// The header word the allocator sets to `0x1000` (its role is not
/// traced).
pub const IMAGE_WORD: u16 = 0x1000;
/// The framebuffer's depth and first colour attachment points.
pub const DEPTH_ATTACHMENT: u32 = 0x8d00;
pub const COLOUR_ATTACHMENT: u32 = 0x8ce0;
/// The depth renderbuffer's format (`GL_DEPTH_COMPONENT`).
pub const DEPTH_FORMAT: u32 = 0x1902;
/// The colour renderbuffer's format while the alpha global is set
/// (`GL_SRGB8_ALPHA8`), and without it (`GL_SRGB8`).
pub const COLOUR_FORMAT_ALPHA: u32 = 0x8c43;
pub const COLOUR_FORMAT: u32 = 0x8c41;

/// One RGBA image header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconImage {
    pub width: u16,
    pub height: u16,
    /// The header word set to 1.
    pub planes: u16,
    /// The format word.
    pub format: u32,
    /// The flag word.
    pub flags: u32,
    /// The header word set to `0x1000`.
    pub word: u16,
    /// Where the payload starts.
    pub data_offset: u32,
    /// The payload's size in bytes.
    pub data_size: i32,
    /// The whole allocation: the payload plus the header.
    pub allocation: i64,
}

/// The image for a side: the 32-byte header, then `side × side × 4` bytes
/// of payload. The allocation is worked in 64 bits, the payload size in 32.
pub fn image(side: i32) -> IconImage {
    let area = side.wrapping_mul(side);
    IconImage {
        width: side as u16,
        height: side as u16,
        planes: IMAGE_PLANES,
        format: IMAGE_FORMAT,
        flags: IMAGE_FLAGS,
        word: IMAGE_WORD,
        data_offset: IMAGE_HEADER,
        data_size: area.wrapping_shl(2),
        allocation: (i64::from(area) << 2) + i64::from(IMAGE_HEADER),
    }
}

/// The renderbuffers and images a renderer makes for an icon of `icon`
/// pixels (the global's value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocations {
    /// The side of both renderbuffers.
    pub buffer_side: i32,
    /// The colour renderbuffer's format.
    pub colour_format: u32,
    /// The full-size image (twice the icon).
    pub full: IconImage,
    /// The final image (the icon's size).
    pub final_image: IconImage,
}

/// The allocations for an icon size and the alpha global.
pub fn allocations(icon: i32, alpha: bool) -> Allocations {
    let buffer_side = icon.wrapping_shl(1);
    Allocations {
        buffer_side,
        colour_format: if alpha {
            COLOUR_FORMAT_ALPHA
        } else {
            COLOUR_FORMAT
        },
        full: image(buffer_side),
        final_image: image(icon),
    }
}

/// The renderer's private scene settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconScene {
    /// The sun's direction (fourth component 0).
    pub sun_direction: [f32; 3],
    /// The sun's colour (red, green, blue, and a fourth component, 0.5).
    pub sun_colour: [f32; 4],
    /// The scene value set to 100 times a global scale (1).
    pub scene_value: f32,
    /// The scene value set to 0.5.
    pub scene_half: f32,
    /// The environment value set to 4 (its role is not traced).
    pub environment_value: f32,
}

/// The icon renderer's scene.
pub fn scene() -> IconScene {
    IconScene {
        sun_direction: [0.46, 0.47, 0.48],
        sun_colour: [0.75, 0.75, 0.75, 0.5],
        scene_value: 100.0,
        scene_half: 0.5,
        environment_value: 4.0,
    }
}

/// The renderer's view settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconView {
    /// The camera's distance setting.
    pub distance: f32,
    /// The camera setting after it (10).
    pub setting: f32,
    /// The camera position.
    pub position: [f32; 3],
    /// The look-at target.
    pub target: [f32; 3],
    /// A view value set to 0.8 (its role is not traced).
    pub view_factor: f32,
    /// A view value set to 0.
    pub view_zero: f32,
    /// The view's size: twice the icon.
    pub size: (i32, i32),
}

/// The icon renderer's view for an icon size.
pub fn view(icon: i32) -> IconView {
    let side = icon.wrapping_shl(1);
    IconView {
        distance: 40.0,
        setting: 10.0,
        position: [0.0, 0.0, 350.0],
        target: [0.0, 0.0, 0.0],
        view_factor: 0.8,
        view_zero: 0.0,
        size: (side, side),
    }
}

/// What the destroy does, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyOrder {
    /// A positive count first runs the object's release hook.
    pub release_hook: bool,
    /// The view, the scene and both images are freed.
    pub frees_parts: bool,
    /// A non-zero count ends with the instance's free.
    pub frees_instance: bool,
}

/// The destroy for a signed count.
pub fn destroy(count: i64) -> DestroyOrder {
    DestroyOrder {
        release_hook: count > 0,
        frees_parts: true,
        frees_instance: count != 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buffers_are_twice_the_icon() {
        let a = allocations(160, true);
        assert_eq!(a.buffer_side, 320);
        assert_eq!(a.colour_format, 0x8c43);
        assert_eq!(allocations(160, false).colour_format, 0x8c41);
        assert_eq!((a.full.width, a.full.height), (320, 320));
        assert_eq!((a.final_image.width, a.final_image.height), (160, 160));
        assert_eq!(a.final_image.allocation, 160 * 160 * 4 + 32);
        assert_eq!(a.full.allocation, 320 * 320 * 4 + 32);
        assert_eq!(DEPTH_FORMAT, 0x1902);
        // A different icon size scales both.
        assert_eq!(allocations(96, true).buffer_side, 192);
    }

    #[test]
    fn the_image_header_matches_the_allocator() {
        let i = image(320);
        assert_eq!((i.width, i.height, i.planes), (320, 320, 1));
        assert_eq!(i.format, 0x100e608);
        assert_eq!(i.flags, 0x20000000);
        assert_eq!(i.word, 0x1000);
        assert_eq!(i.data_offset, 32);
        assert_eq!(i.data_size, 320 * 320 * 4);
    }

    #[test]
    fn the_scene_and_view_keep_the_constructor_numbers() {
        let s = scene();
        assert_eq!(s.sun_direction, [0.46, 0.47, 0.48]);
        assert_eq!(s.sun_colour, [0.75, 0.75, 0.75, 0.5]);
        assert_eq!(
            (s.scene_value, s.scene_half, s.environment_value),
            (100.0, 0.5, 4.0)
        );
        let v = view(160);
        assert_eq!(v.position, [0.0, 0.0, 350.0]);
        assert_eq!(v.target, [0.0, 0.0, 0.0]);
        assert_eq!((v.distance, v.setting, v.view_factor), (40.0, 10.0, 0.8));
        assert_eq!(v.size, (320, 320));
    }

    #[test]
    fn the_destroy_follows_the_count() {
        let d = destroy(1);
        assert!(d.release_hook && d.frees_parts && d.frees_instance);
        let d = destroy(0);
        assert!(!d.release_hook && d.frees_parts && !d.frees_instance);
        let d = destroy(-1);
        assert!(!d.release_hook && d.frees_instance);
    }
}

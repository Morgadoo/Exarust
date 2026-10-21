//! The off-screen icon renderer: a private scene, a camera and a
//! framebuffer at twice the icon size, with the full-size and the final
//! RGBA images the item GUI samples.
//!
//! The icon is 160 pixels, so the render buffers and the first image are
//! 320 square and the final image is 160 square. Each image is the raw
//! RGBA payload plus a 32-byte header: width and height as 16-bit words,
//! the texture format `0x100E608`, the flag word `0x20000000`, the data
//! offset of 32 and the payload's size.
//!
//! The scene's sun points along (0.46, 0.47, 0.48) with the colour
//! (0.75, 0.75, 0.75); the camera sits 40 from the target, looks at the
//! origin from (0, 0, 350), has exposure 10 and the 0.8 depth range. The
//! environment's sun distance is 100 and its field 0.75's neighbour is
//! 0.5. [The field names are not established; the values are the ones the
//! constructor stores.]
//!
//! Evidence: the renderer's constructor, its image allocator and its
//! destroy (`docs/RENDERING.md`, item icons).

/// The icon's size in pixels, and the doubled render size.
pub const ICON_SIZE: i32 = 160;
pub const BUFFER_SIZE: i32 = ICON_SIZE * 2;
/// The image header's size.
pub const IMAGE_HEADER: u32 = 0x20;
/// The image's texture format and flag word.
pub const IMAGE_FORMAT: u32 = 0x0100_e608;
pub const IMAGE_FLAGS: u32 = 0x2000_0000;
/// The flag word at the image's depth/size fields.
pub const IMAGE_DEPTH_FLAGS: u16 = 0x1000;
/// The framebuffer's colour and depth attachment formats.
pub const COLOUR_ATTACHMENT: u32 = 0x8ce0;
pub const DEPTH_ATTACHMENT: u32 = 0x8d00;
/// The depth renderbuffer's formats: 16-bit component, or 24-bit with
/// stencil when the global picks it.
pub const DEPTH_FORMAT: u32 = 0x1902;
pub const DEPTH_STENCIL_FORMAT: u32 = 0x8c43;

/// One RGBA image buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconImage {
    pub width: u16,
    pub height: u16,
    /// The format word.
    pub format: u32,
    /// The flag word.
    pub flags: u32,
    /// The flag at the image's size group.
    pub depth_flags: u16,
    /// Where the payload starts.
    pub data_offset: u32,
    /// The payload's size in bytes.
    pub data_size: i32,
}

/// The image for a size: width × height × 4 bytes of payload after the
/// 32-byte header.
pub fn image(width: i32, height: i32) -> IconImage {
    IconImage {
        width: width as u16,
        height: height as u16,
        format: IMAGE_FORMAT,
        flags: IMAGE_FLAGS,
        depth_flags: IMAGE_DEPTH_FLAGS,
        data_offset: IMAGE_HEADER,
        data_size: width.wrapping_mul(height).wrapping_mul(4),
    }
}

/// The allocations a renderer makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Allocations {
    /// The colour renderbuffer's size.
    pub colour: i32,
    /// The depth renderbuffer's size.
    pub depth: i32,
    /// The depth format: 16-bit or 24-bit with stencil.
    pub depth_format: u32,
    /// The full-size image (twice the icon).
    pub full: IconImage,
    /// The final image (the icon's size).
    pub final_image: IconImage,
    /// The byte count of each allocation.
    pub buffer_bytes: i32,
    pub full_bytes: i32,
    pub final_bytes: i32,
}

/// The allocations for a renderer whose depth uses the stencil format.
pub fn allocations(depth_stencil: bool) -> Allocations {
    Allocations {
        colour: BUFFER_SIZE,
        depth: BUFFER_SIZE,
        depth_format: if depth_stencil {
            DEPTH_STENCIL_FORMAT
        } else {
            DEPTH_FORMAT
        },
        full: image(BUFFER_SIZE, BUFFER_SIZE),
        final_image: image(ICON_SIZE, ICON_SIZE),
        buffer_bytes: BUFFER_SIZE * BUFFER_SIZE * 4 + IMAGE_HEADER as i32,
        full_bytes: BUFFER_SIZE * BUFFER_SIZE * 4 + IMAGE_HEADER as i32,
        final_bytes: ICON_SIZE * ICON_SIZE * 4 + IMAGE_HEADER as i32,
    }
}

/// The renderer's private scene settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconScene {
    /// The sun's direction, normalised by the engine.
    pub sun_direction: [f32; 3],
    /// The sun's colour.
    pub sun_colour: [f32; 3],
    /// The sun distance the constructor stores (100).
    pub sun_distance: f32,
    /// The environment field the constructor sets to 0.5.
    pub field: f32,
    /// The environment's inner value set to 4.
    pub strength: f32,
}

/// The icon renderer's scene.
pub fn scene() -> IconScene {
    IconScene {
        sun_direction: [0.46, 0.47, 0.48],
        sun_colour: [0.75, 0.75, 0.75],
        sun_distance: 100.0,
        field: 0.5,
        strength: 4.0,
    }
}

/// The renderer's camera settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconCamera {
    /// The camera's distance from its target.
    pub distance: f32,
    /// The camera position.
    pub position: [f32; 3],
    /// The look-at target.
    pub target: [f32; 3],
    /// The exposure.
    pub exposure: f32,
    /// The near/far setting the constructor clears.
    pub range: f32,
    /// The two view fields set to 1 and 0.
    pub fixed: bool,
    pub offset: f32,
    /// The image size the view renders at.
    pub size: (i32, i32),
}

/// The icon renderer's camera, its target at the origin.
pub fn camera() -> IconCamera {
    IconCamera {
        distance: 40.0,
        position: [0.0, 0.0, 350.0],
        target: [0.0, 0.0, 0.0],
        exposure: 10.0,
        range: 0.8,
        fixed: true,
        offset: 0.0,
        size: (BUFFER_SIZE, BUFFER_SIZE),
    }
}

/// What the renderer releases, in the original's order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DestroyOrder {
    pub camera: bool,
    pub scene: bool,
    pub full_image: bool,
    pub final_image: bool,
    pub object: bool,
}

/// A destroy with a positive reference count frees all of it.
pub fn destroy(count: i64) -> DestroyOrder {
    DestroyOrder {
        camera: true,
        scene: true,
        full_image: true,
        final_image: true,
        object: count > 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buffers_are_twice_the_icon() {
        let a = allocations(false);
        assert_eq!((a.colour, a.depth), (320, 320));
        assert_eq!(a.depth_format, DEPTH_FORMAT);
        assert_eq!(allocations(true).depth_format, DEPTH_STENCIL_FORMAT);
        assert_eq!((a.full.width, a.full.height), (320, 320));
        assert_eq!((a.final_image.width, a.final_image.height), (160, 160));
        assert_eq!(a.final_bytes, 160 * 160 * 4 + 32);
        assert_eq!(a.full_bytes, 320 * 320 * 4 + 32);
        assert_eq!(a.buffer_bytes, a.full_bytes);
    }

    #[test]
    fn the_image_header_matches_the_allocator() {
        let i = image(320, 160);
        assert_eq!((i.width, i.height), (320, 160));
        assert_eq!(i.format, 0x100e608);
        assert_eq!(i.flags, 0x20000000);
        assert_eq!(i.depth_flags, 0x1000);
        assert_eq!(i.data_offset, 32);
        assert_eq!(i.data_size, 320 * 160 * 4);
    }

    #[test]
    fn the_scene_and_camera_keep_the_constructor_numbers() {
        let s = scene();
        assert_eq!(s.sun_direction, [0.46, 0.47, 0.48]);
        assert_eq!(s.sun_colour, [0.75, 0.75, 0.75]);
        assert_eq!((s.sun_distance, s.field, s.strength), (100.0, 0.5, 4.0));
        let c = camera();
        assert_eq!(c.position, [0.0, 0.0, 350.0]);
        assert_eq!(c.target, [0.0, 0.0, 0.0]);
        assert_eq!((c.distance, c.exposure, c.range), (40.0, 10.0, 0.8));
        assert_eq!(c.size, (320, 320));
        assert!(c.fixed && c.offset == 0.0);
    }

    #[test]
    fn the_destroy_releases_everything() {
        let d = destroy(1);
        assert!(d.camera && d.scene && d.full_image && d.final_image && d.object);
        assert!(!destroy(0).object);
    }
}

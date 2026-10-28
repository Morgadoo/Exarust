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
//! at the origin, with a 40° field of view and 10 in the field after it
//! (read as the near plane, in centimetres).
//!
//! Evidence: the renderer's constructor, its image allocator and its
//! destroy (`docs/RENDERING.md`, item icons).

/// The icon size global's start value, in pixels.
pub const ICON_SIZE: i32 = 160;
/// The alpha global's start value: set, so the screen-quad pass draws the
/// full render into the final image and the downsample and halo do not run.
pub const ALPHA_GLOBAL: bool = true;
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
    /// The view's field of view in degrees (the character view writes 30
    /// to the same field).
    pub fov_degrees: f32,
    /// The view field after it (10) [guess: the near plane, centimetres].
    pub near: f32,
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
        fov_degrees: 40.0,
        near: 10.0,
        position: [0.0, 0.0, 350.0],
        target: [0.0, 0.0, 0.0],
        view_factor: 0.8,
        view_zero: 0.0,
        size: (side, side),
    }
}

/// The position the render gives the icon view's camera node before the
/// scene fit, 500 along +z, with its target left at the origin: the fit
/// moves the camera along this direction from the scene's centre.
pub const CAMERA_SHIFT: [f32; 3] = [0.0, 0.0, 500.0];

/// The scene fit's camera distance: the engine's frame-the-scene routine
/// given the scene box's half extents (`half[0]`, `half[1]` across the
/// view, `half[2]` along it), the framing distance and the view's field of
/// view in degrees.
///
/// The larger of the two cross halves is taken (the second only when the
/// first is below it, an ordered compare), and the depth weight is 1 for
/// the first and 0.5 for the second. While the framing is above that half
/// (ordered), the half is bent toward the framing:
/// `r = h / F` and `h = F · ((r · 0.11 + 1) − 1 / (r · 8 + 1))`, with
/// `1 / (r · 8 + 1)` in single precision and the rest in double. The
/// distance is `h / tan(fov · π / 360)` in double, narrowed, plus the depth
/// half times its weight: `half[2] · w + distance` in single precision.
pub fn frame_distance(half: [f32; 3], framing: f32, fov_degrees: f32) -> f32 {
    let (mut h, mut weight) = (half[0], 1.0f32);
    if half[0] < half[1] {
        h = half[1];
        weight = 0.5;
    }
    if framing > h {
        let r = h / framing;
        let lift = 1.0f32 / (r * 8.0 + 1.0);
        let bent = (f64::from(r) * 0.11 + 1.0) - f64::from(lift);
        h = (f64::from(framing) * bent) as f32;
    }
    let tan = (f64::from(fov_degrees) * std::f64::consts::PI / 360.0).tan();
    let distance = (f64::from(h) / tan) as f32;
    half[2] * weight + distance
}

/// A 3 × 4 row-major transform: three rows of `[x, y, z, w]`.
pub type Transform3 = [[f32; 4]; 3];

/// A point's projection through a [`Transform3`]. The engine's packed
/// transform sums the x and y terms and the z and translation terms
/// separately, then adds the two pairs.
pub fn project(transform: &Transform3, point: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for (row, value) in out.iter_mut().enumerate() {
        let m = &transform[row];
        *value = (point[0] * m[0] + point[1] * m[1]) + (point[2] * m[2] + m[3]);
    }
    out
}

/// The span of projected points: the largest minus the smallest x and y
/// (in single precision), each widened by 0.0001 in double precision and
/// narrowed back to single.
/// `None` for no points.
pub fn projected_extents(points: &[[f32; 3]], transform: &Transform3) -> Option<(f32, f32)> {
    let first = project(transform, *points.first()?);
    let (mut min_x, mut max_x) = (first[0], first[0]);
    let (mut min_y, mut max_y) = (first[1], first[1]);
    for point in &points[1..] {
        let p = project(transform, *point);
        // The compare order is the machine's: below the minimum moves the
        // minimum, otherwise above the maximum moves the maximum. An
        // unordered value moves neither.
        if p[0] < min_x {
            min_x = p[0];
        } else if p[0] > max_x {
            max_x = p[0];
        }
        if p[1] < min_y {
            min_y = p[1];
        } else if p[1] > max_y {
            max_y = p[1];
        }
    }
    Some(span(max_x, min_x, max_y, min_y))
}

/// The span of a box's eight corners: `centre ± half` per axis, projected.
pub fn projected_box_extents(
    centre: [f32; 3],
    half: [f32; 3],
    transform: &Transform3,
) -> (f32, f32) {
    let mut low = [0.0f32; 3];
    let mut high = [0.0f32; 3];
    for axis in 0..3 {
        low[axis] = centre[axis] - half[axis];
        high[axis] = centre[axis] + half[axis];
    }
    let mut corners = [[0.0f32; 3]; 8];
    let mut at = 0;
    for x in [low[0], high[0]] {
        for y in [low[1], high[1]] {
            for z in [low[2], high[2]] {
                corners[at] = [x, y, z];
                at += 1;
            }
        }
    }
    projected_extents(&corners, transform).unwrap_or((0.0001, 0.0001))
}

/// The exact subtraction and widening of the extents helper: the
/// difference is taken in single precision, only the widening in double.
fn span(max_x: f32, min_x: f32, max_y: f32, min_y: f32) -> (f32, f32) {
    let width = max_x - min_x;
    let height = max_y - min_y;
    (
        (f64::from(width) + 0.0001) as f32,
        (f64::from(height) + 0.0001) as f32,
    )
}

/// The depth the icon scene's bounding box spans: its extent along the
/// camera's axis (the transform's second row; the icon camera sits 500 up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DepthBounds {
    /// The smallest and largest projected depth.
    pub min: f32,
    pub max: f32,
    /// Their midpoint (the box centre's second component).
    pub centre: f32,
    /// Half their difference (the box half extent's second component).
    pub half: f32,
    /// The length of the box's half extents with the new depth half:
    /// `sqrt((x² + z²) + half²)`, the engine's vector length order.
    pub radius: f32,
}

/// One kept mesh node of the icon scene: its vertices and its matrix.
#[derive(Debug, Clone, Copy)]
pub struct SceneMesh<'a> {
    pub vertices: &'a [[f32; 3]],
    pub transform: Transform3,
}

/// The scene bounds the icon camera frames. `meshes` are the scene's mesh
/// nodes the original keeps (the mesh class, not both withheld bits), in
/// node order; `seed` is the first kept node's stored depth (a node field
/// both bounds start from before any vertex is seen); `half_xz` is the
/// box's other two half extents, which the radius includes. With no kept
/// node the original reads uninitialised bounds; this returns zeros there.
/// The original then adds the node's offset to the centre into a second
/// vector, which is not modelled.
pub fn scene_bounds(seed: f32, meshes: &[SceneMesh<'_>], half_xz: (f32, f32)) -> DepthBounds {
    if meshes.is_empty() {
        return DepthBounds {
            min: 0.0,
            max: 0.0,
            centre: 0.0,
            half: 0.0,
            radius: 0.0,
        };
    }
    let (mut min, mut max) = (seed, seed);
    for mesh in meshes {
        let row = &mesh.transform[1];
        for p in mesh.vertices {
            // The depth row in the machine's order: ((x + y) + z) + w.
            let depth = p[0] * row[0] + p[1] * row[1] + p[2] * row[2] + row[3];
            // Above the maximum moves the maximum, otherwise below the
            // minimum moves the minimum; an unordered value moves neither.
            if depth > max {
                max = depth;
            } else if depth < min {
                min = depth;
            }
        }
    }
    let centre = (min + max) * 0.5;
    let half = (max - min) * 0.5;
    let (hx, hz) = half_xz;
    DepthBounds {
        min,
        max,
        centre,
        half,
        radius: ((hx * hx + hz * hz) + half * half).sqrt(),
    }
}

/// The buffers and images the resize hook gives an icon of `icon` pixels:
/// both renderbuffers are twice the icon, the full image is the buffer's
/// side and the final image the icon's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resize {
    pub buffer_side: i32,
    pub alpha_colour_format: u32,
    pub full: IconImage,
    pub final_image: IconImage,
}

/// The resize for a GUI unit and the alpha global (the icon size is
/// [`crate::icon_cache::icon_size`]).
pub fn resize(unit: i32, alpha: bool) -> Resize {
    let icon = crate::icon_cache::icon_size(unit);
    let a = allocations(icon, alpha);
    Resize {
        buffer_side: a.buffer_side,
        alpha_colour_format: a.colour_format,
        full: a.full,
        final_image: a.final_image,
    }
}

/// One icon render's plan: what the render routine does with the item and
/// the numbers it frames the camera with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderPlan {
    /// The framing distance (20, 30 or 70) and sun.
    pub framing: crate::item_gui::Framing,
    /// The object is turned (−0.1, 0.1, 0.5) before it renders.
    pub turned: bool,
    /// The clothing material named with a leading `*` is hidden.
    pub hide_star_materials: bool,
    /// The node's meshes are subdivided first.
    pub subdivide: bool,
    /// The projected width and height after the screen scale.
    pub screen_extents: (f32, f32),
    /// The screen-quad pass draws the full render into the final image
    /// (while the alpha global is set). The post-process call itself always
    /// runs.
    pub screen_pass: bool,
    /// The full-size render is filtered down to the final image (rather
    /// than the final image being the full one); only without the screen
    /// pass.
    pub downsample: bool,
    /// The halo runs on the final image; only without the screen pass.
    pub halo: bool,
    /// The halo is the highlighted one.
    pub halo_highlight: bool,
}

/// The scale the extents take: the final image's width times the view's
/// scale, over the distance between the camera and its target, halved.
pub fn extent_scale(icon_pixels: i32, view_scale: f32, distance_squared: f32) -> f32 {
    (icon_pixels as f32 * view_scale) / distance_squared.sqrt() * 0.5
}

/// The icon render plan for an item: the framing by its variant, the
/// scene's bounds and the projected extents it frames. `extents` is the
/// item's own projected span; `model_flags` decides the material hiding;
/// `close` is the manager's close-icon flag; `alpha` is the alpha global,
/// which picks the screen-quad pass over the downsample and halo.
#[allow(clippy::too_many_arguments)]
pub fn render_plan(
    variant: u32,
    close: bool,
    extents: (f32, f32),
    model_flags: u32,
    icon_pixels: i32,
    view_scale: f32,
    distance_squared: f32,
    alpha: bool,
) -> RenderPlan {
    let framing = crate::item_gui::framing(variant, close);
    let scale = extent_scale(icon_pixels, view_scale, distance_squared);
    RenderPlan {
        framing,
        turned: framing.turned,
        hide_star_materials: model_flags & 0x5f == 9,
        subdivide: variant & 0x8000_0000 != 0,
        screen_extents: (extents.0 * scale, extents.1 * scale),
        screen_pass: alpha,
        downsample: !alpha && downsamples(icon_pixels, icon_pixels.wrapping_shl(1)),
        halo: !alpha,
        halo_highlight: variant & crate::icon_cache::variant::HIGHLIGHT != 0,
    }
}

/// Whether the render filters the full image down: the final image's side
/// is smaller than the full one's. Equal sides make the final image the
/// full one instead.
pub fn downsamples(final_side: i32, full_side: i32) -> bool {
    final_side < full_side
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
        assert_eq!((v.fov_degrees, v.near, v.view_factor), (40.0, 10.0, 0.8));
        assert_eq!(v.size, (320, 320));
    }

    #[test]
    fn the_scene_fit_bends_small_items_toward_the_framing() {
        // 5 cm across at the 70 cm framing: r = 1/14, the half becomes
        // 70 · ((r · 0.11 + 1) − 1/(8r + 1)) = 26.0045, over tan(20°), plus
        // the depth half at weight 1.
        assert!((frame_distance([5.0, 2.0, 1.0], 70.0, 40.0) - 72.446_91).abs() < 1.0e-3);
        // The second cross half wins only when the first is below it, and
        // halves the depth weight; a half above the framing is not bent.
        assert!((frame_distance([2.0, 50.0, 10.0], 30.0, 40.0) - 142.373_87).abs() < 1.0e-3);
        // Equal halves keep the first and its full depth weight.
        let equal = frame_distance([0.5, 0.5, 0.5], 0.3, 40.0);
        assert!((equal - (0.5 / 20.0f64.to_radians().tan() + 0.5) as f32).abs() < 1.0e-5);
        // An unordered half takes neither branch: the framing is not above
        // it, so the distance is unordered too, never a panic.
        assert!(frame_distance([f32::NAN, 1.0, 1.0], 70.0, 40.0).is_nan());
    }

    const IDENTITY: Transform3 = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ];

    #[test]
    fn the_extents_span_the_projected_points() {
        let points = [[1.0, 2.0, 3.0], [-1.0, 4.0, 0.0]];
        let (w, h) = projected_extents(&points, &IDENTITY).unwrap();
        assert_eq!((w, h), (2.0001, 2.0001));
        // 2 x − 3 y = the projection: x 2·1+5 and 2·(−1)+5, y 3·1−1 and
        // 3·(−1)−1.
        let scaled: Transform3 = [
            [2.0, 0.0, 0.0, 5.0],
            [0.0, 3.0, 0.0, -1.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let points = [[1.0, 1.0, 0.0], [-1.0, -1.0, 0.0]];
        let (w, h) = projected_extents(&points, &scaled).unwrap();
        assert_eq!((w, h), (4.0001, 6.0001));
        // An unordered value moves neither bound, as the machine's ordered
        // compares skip it.
        let points = [[0.0, 0.0, 0.0], [f32::NAN, f32::NAN, 0.0], [2.0, 3.0, 0.0]];
        assert_eq!(
            projected_extents(&points, &IDENTITY).unwrap(),
            (2.0001, 3.0001)
        );
        assert!(projected_extents(&[], &IDENTITY).is_none());
    }

    #[test]
    fn the_box_corners_are_its_eight_corners() {
        // A box's centre ± half, its x halved by the transform.
        let tf: Transform3 = [
            [0.5, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let (w, h) = projected_box_extents([1.0, 0.0, 0.0], [2.0, 3.0, 1.0], &tf);
        assert_eq!((w, h), (2.0001, 6.0001));
        // A single corner (a zero box) still gets the widening.
        let (w, h) = projected_box_extents([4.0, 5.0, 6.0], [0.0; 3], &IDENTITY);
        assert_eq!((w, h), (0.0001, 0.0001));
    }

    #[test]
    fn the_projection_pairs_its_terms() {
        // (1e8 + 1) + (−1e8 + 1): the pairs round to 1e8 and −1e8, so the
        // sum is 0, where a left-to-right sum would give 1.
        let tf: Transform3 = [
            [1.0, 1.0, 1.0, 1.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        assert_eq!(project(&tf, [1.0e8, 1.0, -1.0e8])[0], 0.0);
    }

    #[test]
    fn the_scene_bounds_frame_the_camera_axis() {
        let verts = [[1.0, 1.0, 1.0], [1.0, 3.0, 1.0], [9.0, -1.0, 9.0]];
        let mesh = SceneMesh {
            vertices: &verts,
            transform: IDENTITY,
        };
        let b = scene_bounds(1.0, &[mesh], (0.0, 0.0));
        assert_eq!(
            (b.min, b.max, b.centre, b.half, b.radius),
            (-1.0, 3.0, 1.0, 2.0, 2.0)
        );
        // The radius takes the box's other half extents: sqrt(9 + 4).
        let b = scene_bounds(1.0, &[mesh], (3.0, 0.0));
        assert_eq!(b.radius, 13.0f32.sqrt());
        // The seed bounds both ends before the first vertex.
        let b = scene_bounds(10.0, &[mesh], (0.0, 0.0));
        assert_eq!((b.min, b.max), (-1.0, 10.0));
        assert_eq!(scene_bounds(1.0, &[], (0.0, 0.0)).half, 0.0);
        // Each node projects through its own matrix's depth row.
        let tilted: Transform3 = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 2.0, 0.0, 1.0],
            [0.0, 0.0, 1.0, 0.0],
        ];
        let verts = [[0.0, 1.0, 0.0], [0.0, 2.0, 0.0]];
        let mesh = SceneMesh {
            vertices: &verts,
            transform: tilted,
        };
        let b = scene_bounds(3.0, &[mesh], (0.0, 0.0));
        assert_eq!((b.min, b.max, b.centre, b.half), (3.0, 5.0, 4.0, 1.0));
    }

    #[test]
    fn the_screen_scale_keeps_the_float_order() {
        assert_eq!(extent_scale(160, 1.0, 100.0), 8.0);
        assert_eq!(extent_scale(160, 1.0, 400.0), 4.0);
        assert_eq!(extent_scale(80, 2.0, 400.0), 4.0);
        assert!(downsamples(160, 320));
        assert!(!downsamples(160, 160));
    }

    #[test]
    fn the_render_plan_follows_the_variant() {
        use crate::icon_cache::variant as v;
        let p = render_plan(
            v::APPAREL | v::WEAPON | v::HIGHLIGHT,
            true,
            (2.0, 1.0),
            9,
            160,
            1.0,
            400.0,
            false,
        );
        assert_eq!(p.framing.distance, 20.0);
        assert_eq!(p.framing.sun, (-0.16, 0.6516));
        assert!(p.turned && p.hide_star_materials && p.halo_highlight);
        assert!(!p.subdivide);
        assert!(!p.screen_pass && p.downsample && p.halo);
        // 2 × 4.0 and 1 × 4.0.
        assert_eq!(p.screen_extents, (8.0, 4.0));
        // Apparel alone: 30, the other sun.
        let p = render_plan(v::APPAREL, false, (1.0, 1.0), 0, 160, 1.0, 400.0, true);
        // The alpha global's screen pass replaces the downsample and halo.
        assert!(p.screen_pass && !p.downsample && !p.halo);
        assert_eq!(p.framing.distance, 30.0);
        assert_eq!(p.framing.sun, (-0.21, 0.7516));
        assert!(!p.turned && !p.hide_star_materials);
        // Neither: 70, and the subdivision bit.
        let p = render_plan(0x8000_0000, false, (1.0, 1.0), 0, 160, 1.0, 400.0, false);
        assert_eq!(p.framing.distance, 70.0);
        assert!(p.subdivide);
    }

    #[test]
    fn the_resize_is_twice_the_icon() {
        let r = resize(4, true);
        assert_eq!(r.buffer_side, 320);
        assert_eq!(r.alpha_colour_format, 0x8c43);
        assert_eq!((r.full.width, r.final_image.width), (320, 160));
        let r = resize(2, false);
        assert_eq!(
            (r.buffer_side, r.full.width, r.final_image.width),
            (160, 160, 80)
        );
        assert_eq!(r.alpha_colour_format, 0x8c41);
        assert_eq!(CAMERA_SHIFT, [0.0, 500.0, 0.0]);
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

//! Shadow-map plumbing: the size each light's map gets from the shadow
//! detail setting and the light's own offsets, the two map classes
//! (directional depth map or cube), the caster programs, the cube map pool
//! the renderer hands out at draw time, the cull-face state of the caster
//! pass and the matrices the directional map builds.
//!
//! The original has two places that size a cube map. The allocator picks
//! the class by the light's kind (kind 2 the directional `TShadowMapDirect`,
//! anything else the cube class) and sizes it from a table indexed by the
//! shadow detail plus the light's quality offset; the cube class then
//! halves the table value, shifts it by the light's reduction level and
//! caps it by the hardware's maximum. The cube map's own draw routine sizes
//! the map again when it first renders, from a table of its own with
//! different clamping, and takes its map out of a pool of equally sized
//! maps. The caster programs write distance to the light as depth, with an
//! optional alpha test or an optional dithered fade.
//!
//! Exarust renders the cube maps through Bevy (`cube_shadows`) at its fixed
//! top size and the sun through Bevy's shadow map; nothing here drives them
//! yet. The rules are the sizes, classes, programs and pool arithmetic the
//! original uses, pinned by tests, for the settings changes and the map
//! sizes to follow.
//!
//! Rules: `docs/RENDERING.md` (Lights, shadows).

use bevy::prelude::*;
use exa_formats::rfc::reciprocal_sqrt;

/// The user's default shadow detail (the original's stored setting).
pub const DEFAULT_SHADOW_DETAIL: i32 = 4;
/// The value the allocator substitutes for a user detail above
/// [`DEFAULT_SHADOW_DETAIL`].
pub const TOP_SHADOW_DETAIL: i32 = 5;
/// The highest table index; the draw-time sizing clamps the user's detail
/// plus the light's offset to it.
pub const MAX_SHADOW_QUALITY: i32 = 6;
/// The hardware's maximum face size in the shipped data. The game
/// overwrites this from its hardware configuration.
pub const SHIPPED_FACE_CAP: i32 = 256;
/// The mask of all six cube faces, the state of a map that needs a redraw.
pub const ALL_FACES: u32 = 0x3F;

/// The directional table size for quality index `q`; an index outside 1..=6
/// takes the first entry's value.
fn direct_table_size(q: i32) -> i32 {
    match q {
        1 => 0x100,
        2 => 0x200,
        3 => 0x400,
        4 => 0x800,
        5 => 0x1000,
        6 => 0x2000,
        _ => 0x800,
    }
}

/// The cube table size for quality index `q`, before the halving; an index
/// outside 1..=6 takes the middle entry's value.
fn cube_table_size(q: i32) -> i32 {
    match q {
        1 => 0x40,
        2 => 0x80,
        3 => 0x100,
        4 => 0x200,
        5 => 0x400,
        6 => 0x800,
        _ => 0x200,
    }
}

/// What kind of shadow map a light gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowKind {
    /// One depth map (light kind 2, the sun).
    Directional,
    /// Six cube faces (every other light kind).
    Cube,
}

/// The shadow map a light allocates: its class and the face size the
/// hardware allows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowAllocation {
    pub kind: ShadowKind,
    /// Face width and height, texels.
    pub size: u32,
}

/// The allocator's quality index: a detail of 0 takes the user's setting
/// (a setting above 4 counts as 5), then the light's quality offset is
/// added. A non-zero detail is used as given. Nothing clamps the sum.
pub fn allocation_quality(detail: i32, light_offset: i32, user_detail: i32) -> i32 {
    let base = if detail != 0 {
        detail
    } else if user_detail > DEFAULT_SHADOW_DETAIL {
        TOP_SHADOW_DETAIL
    } else {
        user_detail
    };
    base.wrapping_add(light_offset)
}

/// The allocation a light of `kind` gets for quality index `q`, or `None`
/// at index 0 (the original returns no map). An index outside the table,
/// negative or above 6, still allocates a map of the table's fallback
/// size. Directional maps take the table value directly; cube maps halve
/// it and shift it right by the light's reduction level `shift` (taken
/// modulo 32). Both are then capped at the signed `hardware_cap`.
pub fn shadow_allocation(
    kind: ShadowKind,
    q: i32,
    shift: u32,
    hardware_cap: i32,
) -> Option<ShadowAllocation> {
    if q == 0 {
        return None;
    }
    let size = match kind {
        ShadowKind::Directional => direct_table_size(q),
        ShadowKind::Cube => (cube_table_size(q) >> 1) >> (shift & 0x1F),
    };
    Some(ShadowAllocation {
        kind,
        size: size.min(hardware_cap).max(0) as u32,
    })
}

/// The original's allocator: the quality index from a light's detail and
/// offsets, then the map of the matching class.
pub fn allocate_shadow(
    kind: ShadowKind,
    detail: i32,
    light_offset: i32,
    shift: u32,
    user_detail: i32,
    hardware_cap: i32,
) -> Option<ShadowAllocation> {
    let q = allocation_quality(detail, light_offset, user_detail);
    shadow_allocation(kind, q, shift, hardware_cap)
}

/// What a cube map's draw routine sets up when it first renders and has no
/// pooled map yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubeDrawSettings {
    /// The quality index: the light's offset plus the renderer's own detail
    /// when it has one, else the user's setting, clamped to 6 in that case.
    pub quality: i32,
    /// The value the light keeps for its lighting pass: `0.005·quality` for
    /// indices up to 5 (double precision, stored as single), else 0.025.
    pub light_offset: f32,
    /// Set when the user's detail is above 4.
    pub high_detail: bool,
    /// The face size: the cube table value halved, with no shift and no
    /// hardware cap.
    pub face_size: u32,
}

/// The draw-time sizing of a cube map. `renderer_detail` is the renderer's
/// own shadow detail (0 when it has none, which takes the user's setting).
pub fn cube_draw_settings(
    light_offset: i32,
    renderer_detail: i32,
    user_detail: i32,
) -> CubeDrawSettings {
    let quality = if renderer_detail == 0 {
        (i64::from(light_offset) + i64::from(user_detail)).min(i64::from(MAX_SHADOW_QUALITY)) as i32
    } else {
        light_offset.wrapping_add(renderer_detail)
    };
    CubeDrawSettings {
        quality,
        light_offset: cube_light_offset(quality),
        high_detail: user_detail > DEFAULT_SHADOW_DETAIL,
        face_size: (cube_table_size(quality) >> 1) as u32,
    }
}

/// The offset a cube light keeps for quality index `q`: `0.005·q` in double
/// precision for indices up to 5 (negative ones included), else 0.025.
pub fn cube_light_offset(q: i32) -> f32 {
    if q < 6 {
        (f64::from(q) * 0.005) as f32
    } else {
        0.025
    }
}

/// The map-size fields a shadow map carries, as the original's base-class
/// slot sets them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct ShadowMapSize {
    pub width: u32,
    pub height: u32,
}

impl ShadowMapSize {
    /// The base slot: stores both dimensions and returns itself.
    pub fn set(&mut self, width: u32, height: u32) -> &mut Self {
        self.width = width;
        self.height = height;
        self
    }
}

/// The directional map's fixed settings: near plane 10, far plane 2 750,
/// its initial needed/dirty mask `0x3F` (all six faces), its own texture, and
/// the polygon offset its first render starts from (factor 1, units 100) —
/// the render recomputes both before it draws, so these only matter to a map
/// that has not rendered yet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectShadowSettings {
    pub size: ShadowMapSize,
    pub near: f32,
    pub far: f32,
    pub faces: u32,
    /// The map holds its own texture rather than one from the shared pool.
    pub own_texture: bool,
    /// The polygon offset the render's first pass would start from.
    pub polygon_factor: f32,
    pub polygon_units: f32,
}

impl DirectShadowSettings {
    /// The constructor for a `width` × `height` map.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: ShadowMapSize { width, height },
            near: 10.0,
            far: 2_750.0,
            faces: ALL_FACES,
            own_texture: true,
            polygon_factor: 1.0,
            polygon_units: 100.0,
        }
    }
}

/// How a shadow map's texels are filtered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowFilter {
    Nearest,
    Linear,
}

/// How a shadow map's texels wrap past the edge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShadowWrap {
    ClampToEdge,
    Repeat,
    MirroredRepeat,
}

/// How the map is sampled: its magnification and minification filters, its
/// wrap on both axes and whether the depth is compared as it is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowSampler {
    pub min: ShadowFilter,
    pub mag: ShadowFilter,
    pub wrap_s: ShadowWrap,
    pub wrap_t: ShadowWrap,
    /// The texture is compared against the reference depth when sampled
    /// (a compare sampler) rather than read as a plain depth value.
    pub compare: bool,
}

/// The sun map's sampler as its creation sets it: the creation's filter mode
/// word, `clamp-to-edge` on both axes and comparison on.
pub const SUN_SHADOW_SAMPLER: ShadowSampler = ShadowSampler {
    min: ShadowFilter::Linear,
    mag: ShadowFilter::Linear,
    wrap_s: ShadowWrap::ClampToEdge,
    wrap_t: ShadowWrap::ClampToEdge,
    compare: true,
};

/// The filters a texture creation's mode word picks, as the original's
/// sampler helper does: bit 5 takes the linear minification filter, and bit 4
/// the linear magnification filter. Bits 6 and 7 select mipmapped variants of
/// the same filter (nearest or linear), which a shadow map never uses.
pub fn sampler_filters(mode: u32) -> (ShadowFilter, ShadowFilter) {
    let min = if mode & 0x20 != 0 {
        ShadowFilter::Linear
    } else {
        ShadowFilter::Nearest
    };
    let mag = if mode & 0x10 != 0 {
        ShadowFilter::Linear
    } else {
        ShadowFilter::Nearest
    };
    (min, mag)
}

/// The size of a shadow map's depth texels, in bits: the sun map's creation
/// asks for a 16-bit depth texture.
pub const SUN_SHADOW_DEPTH_BITS: u32 = 16;

/// Which face the caster pass culls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CullFace {
    Back,
    Front,
}

/// The caster pass's remembered cull face. A caster with the front-cull flag
/// draws with front faces culled, every other caster with back faces culled;
/// the state remembers the last choice so an unchanged flag costs nothing.
/// The cube pass resets it to back culling when it starts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CasterCull {
    front: bool,
}

impl CasterCull {
    /// A cube pass begins: back faces are culled and the flag cleared.
    pub fn begin_pass(&mut self) -> CullFace {
        self.front = false;
        CullFace::Back
    }

    /// A caster is about to draw. Returns the face to switch to when the
    /// flag changed, `None` when the current setting already fits.
    pub fn set(&mut self, front: bool) -> Option<CullFace> {
        if front == self.front {
            return None;
        }
        self.front = front;
        Some(if front {
            CullFace::Front
        } else {
            CullFace::Back
        })
    }

    /// A cube pass ends: back faces are culled again. The remembered flag is
    /// left as it was; the next pass clears it.
    pub fn end_pass(&self) -> CullFace {
        CullFace::Back
    }

    /// Whether front faces are culled now.
    pub fn front(&self) -> bool {
        self.front
    }
}

/// Which caster program a draw uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CasterProgram {
    /// Texture coordinates are read and an alpha test discards below ½.
    pub alpha_test: bool,
    /// The fragment writes radial distance to the light as depth.
    pub writes_distance: bool,
    /// A dithered fade pattern discards fragments by the caster's fade.
    pub dithered: bool,
}

/// The six caster programs the original's builder makes.
pub mod caster {
    use super::CasterProgram;

    /// Position only, no fragment work.
    pub const DEPTH: CasterProgram = CasterProgram {
        alpha_test: false,
        writes_distance: false,
        dithered: false,
    };
    /// Position and texture coordinates, alpha discarded below ½.
    pub const DEPTH_CUT: CasterProgram = CasterProgram {
        alpha_test: true,
        writes_distance: false,
        dithered: false,
    };
    /// Distance to the light written as depth.
    pub const DISTANCE: CasterProgram = CasterProgram {
        alpha_test: false,
        writes_distance: true,
        dithered: false,
    };
    /// Distance and texture coordinates, alpha discarded below ½.
    pub const DISTANCE_CUT: CasterProgram = CasterProgram {
        alpha_test: true,
        writes_distance: true,
        dithered: false,
    };
    /// Distance plus a dither discard by the caster's fade.
    pub const DISTANCE_FADE: CasterProgram = CasterProgram {
        alpha_test: false,
        writes_distance: true,
        dithered: true,
    };
}

/// Selects the caster program, as the original's program builder does:
/// texture coordinates and the alpha test only for an alpha-tested draw,
/// distance only for a cube face, and the dithered stage only for a cube
/// face without an alpha test. An alpha-tested draw ignores the fade flag.
pub fn caster_program(alpha_test: bool, cube_face: bool, faded: bool) -> CasterProgram {
    CasterProgram {
        alpha_test,
        writes_distance: cube_face,
        dithered: cube_face && !alpha_test && faded,
    }
}

/// The dithered fade caster's fragment stage, in WGSL: a screen-space hash
/// of the fragment's coordinates, discarding below the caster's fade value,
/// then the distance to the light scaled by the light's depth factor. The
/// original reads the window coordinates from the bottom left; WGSL's start
/// at the top left, so the pattern comes out mirrored vertically.
pub const DITHER_FADE_WGSL: &str = r#"
struct CasterLight {
    // xyz: the light's position, w: the factor that turns distance to depth.
    position: vec4<f32>,
    fade: f32,
}
@group(0) @binding(0) var<uniform> light: CasterLight;

@fragment
fn fade_distance(
    @builtin(position) frag: vec4<f32>,
    @location(0) world: vec4<f32>,
) -> @builtin(frag_depth) f32 {
    let a = fract(frag.x * 3.555713358 + frag.y * 0.309269245);
    if a < light.fade {
        discard;
    }
    return distance(light.position.xyz, world.xyz) * light.position.w;
}
"#;

/// A shared cube depth texture: the texture handle, its size and how many
/// maps hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PooledCubeTex {
    pub handle: u32,
    pub size: u32,
    pub references: u32,
}

/// The shared cube depth textures, reference-counted by size as the
/// original's pool keeps them: a request for a size already present takes
/// the existing entry and raises its count; a last release frees the entry
/// by moving the last one into its place. The original reaches this pool
/// only for a map whose own-texture flag is clear, and its constructor sets
/// that flag, so the shipped game never uses it.
#[derive(Clone, Debug, Default)]
pub struct CubeTexPool {
    entries: Vec<PooledCubeTex>,
    next_handle: u32,
}

impl CubeTexPool {
    /// An empty pool.
    pub fn new() -> Self {
        Self::default()
    }

    /// The live entries.
    pub fn entries(&self) -> &[PooledCubeTex] {
        &self.entries
    }

    /// A cube texture of `size` texels: the first entry of that size (its
    /// count raised) or a new entry (count 1, a fresh handle). Returns the
    /// handle.
    pub fn acquire(&mut self, size: u32) -> u32 {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.size == size) {
            entry.references += 1;
            return entry.handle;
        }
        self.next_handle += 1;
        let handle = self.next_handle;
        self.entries.push(PooledCubeTex {
            handle,
            size,
            references: 1,
        });
        handle
    }

    /// Drops a reference to the texture `handle`; at zero the texture is
    /// freed and the last entry moved into its slot.
    pub fn release(&mut self, handle: u32) {
        let Some(index) = self.entries.iter().position(|e| e.handle == handle) else {
            return;
        };
        self.entries[index].references = self.entries[index].references.saturating_sub(1);
        if self.entries[index].references < 1 {
            self.entries.swap_remove(index);
        }
    }

    /// The entry of a size, if the pool holds one.
    pub fn get(&self, size: u32) -> Option<&PooledCubeTex> {
        self.entries.iter().find(|e| e.size == size)
    }
}

/// One pooled cube map: its face size and the light that holds it now.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PooledCubeMap {
    pub size: u32,
    pub owner: Option<usize>,
}

/// What handing out a pooled map did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CubeMapGrant {
    /// The pool slot the light now holds.
    pub slot: usize,
    /// The light that lost the slot, whose faces are all due a redraw and
    /// which must forget the slot.
    pub evicted: Option<usize>,
}

/// The cube maps the renderer hands to lights at draw time, grouped by face
/// size. A light that first draws takes the first map of its size that has
/// no owner or whose owner is not in use; that owner loses it. With none
/// free a new map is made. A change of the shadow detail drops every map.
#[derive(Clone, Debug, Default)]
pub struct CubeMapPool {
    maps: Vec<PooledCubeMap>,
    user_detail: i32,
}

impl CubeMapPool {
    /// An empty pool for the user's shadow detail.
    pub fn new(user_detail: i32) -> Self {
        Self {
            maps: Vec::new(),
            user_detail,
        }
    }

    /// The user's shadow detail the draw-time sizing reads.
    pub fn user_detail(&self) -> i32 {
        self.user_detail
    }

    /// The maps in the pool.
    pub fn maps(&self) -> &[PooledCubeMap] {
        &self.maps
    }

    /// Gives `light` a map of `size` texels. `in_use` tells whether an
    /// owner still has faces to show (its second face mask is non-empty).
    pub fn assign(
        &mut self,
        light: usize,
        size: u32,
        in_use: impl Fn(usize) -> bool,
    ) -> CubeMapGrant {
        let mut found = None;
        for (index, map) in self.maps.iter().enumerate() {
            if map.size != size {
                continue;
            }
            match map.owner {
                Some(owner) if in_use(owner) => continue,
                owner => {
                    found = Some((index, owner));
                    break;
                }
            }
        }
        let (slot, evicted) = found.unwrap_or_else(|| {
            self.maps.push(PooledCubeMap { size, owner: None });
            (self.maps.len() - 1, None)
        });
        self.maps[slot].owner = Some(light);
        CubeMapGrant { slot, evicted }
    }

    /// A light is gone: its map keeps its size and loses its owner.
    pub fn release_owner(&mut self, slot: usize) {
        if let Some(map) = self.maps.get_mut(slot) {
            map.owner = None;
        }
    }

    /// The shadow detail changed: every owner is told to redraw all faces
    /// and forget its map, and the pool empties. Returns the owners.
    pub fn set_detail(&mut self, user_detail: i32) -> Vec<usize> {
        self.user_detail = user_detail;
        let owners = self.maps.iter().filter_map(|m| m.owner).collect();
        self.maps.clear();
        owners
    }
}

/// An engine vector normalised as the original does: the squared length
/// summed as `y² + (x² + z²)`, its reciprocal square root taken from the
/// processor's estimate (about 12 bits), a zero length leaving a zero
/// vector.
fn engine_normalize(v: Vec3) -> Vec3 {
    let sum = v.y * v.y + (v.x * v.x + v.z * v.z);
    let scale = if sum == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(sum)
    };
    v * scale
}

/// `a × b` in the original's component order.
fn engine_cross(a: Vec3, b: Vec3) -> Vec3 {
    Vec3::new(
        a.y * b.z - a.z * b.y,
        b.x * a.z - a.x * b.z,
        b.y * a.x - a.y * b.x,
    )
}

/// The view matrix of a directional shadow map, as the original builds it
/// from the light's position, its forward axis and an up vector. The rows
/// of the rotation are `normalize(cross(forward, up))`, `normalize(cross(
/// first, forward))` and the forward axis; the translation takes the light's
/// position to the origin, each component summed as `z + (y + x)`. The
/// normalisations use the processor's reciprocal square root estimate, so
/// the axes are unit length to about 1.5 parts in 4 096.
pub fn shadow_view(position: Vec3, forward: Vec3, up: Vec3) -> Mat4 {
    let first = engine_normalize(engine_cross(forward, up));
    let second = engine_normalize(engine_cross(first, forward));
    let third = forward;
    let along = |axis: Vec3| -(axis.z * position.z + (axis.y * position.y + axis.x * position.x));
    Mat4::from_cols(
        Vec4::new(first.x, second.x, third.x, 0.0),
        Vec4::new(first.y, second.y, third.y, 0.0),
        Vec4::new(first.z, second.z, third.z, 0.0),
        Vec4::new(along(first), along(second), along(third), 1.0),
    )
}

/// The orthographic projection of a directional map over the box
/// `min..max`, as the original fills it: `2/(max − min)` on the diagonal
/// and `−(max + min)/(max − min)` for the translation, per axis, in single
/// precision.
pub fn shadow_ortho(min: Vec3, max: Vec3) -> Mat4 {
    let inv = |a: f32, b: f32| 2.0 / (b - a);
    let shift = |a: f32, b: f32| -(b + a) / (b - a);
    Mat4::from_cols_array(&[
        inv(min.x, max.x),
        0.0,
        0.0,
        0.0,
        0.0,
        inv(min.y, max.y),
        0.0,
        0.0,
        0.0,
        0.0,
        inv(min.z, max.z),
        0.0,
        shift(min.x, max.x),
        shift(min.y, max.y),
        shift(min.z, max.z),
        1.0,
    ])
}

/// The cube face's projection, as the original builds it: 90° vertical, an
/// aspect of 1, the near plane (10 when the map is created, 3 when it
/// renders) and the far plane at the light's radius.
pub fn cube_projection(radius: f32, near: f32) -> PerspectiveProjection {
    PerspectiveProjection {
        fov: 90f32.to_radians(),
        aspect_ratio: 1.0,
        near,
        far: radius,
        ..default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(kind: ShadowKind, q: i32, shift: u32, cap: i32) -> Option<u32> {
        shadow_allocation(kind, q, shift, cap).map(|a| a.size)
    }

    #[test]
    fn the_shadow_detail_and_offsets_pick_the_size() {
        // The user's detail 4 with no offset, cube: 0x200 halved is 256.
        let q = allocation_quality(0, 0, DEFAULT_SHADOW_DETAIL);
        assert_eq!(q, 4);
        assert_eq!(
            shadow_allocation(ShadowKind::Cube, q, 0, 8192),
            Some(ShadowAllocation {
                kind: ShadowKind::Cube,
                size: 256
            })
        );
        // Directional at index 4 is 0x800, unless the hardware caps it.
        assert_eq!(size(ShadowKind::Directional, 4, 0, 8192), Some(2048));
        assert_eq!(
            size(ShadowKind::Directional, 4, 0, SHIPPED_FACE_CAP),
            Some(256)
        );
        // The cap is a signed minimum, so a cap that is no power of two
        // applies as given.
        assert_eq!(size(ShadowKind::Cube, 6, 0, 100), Some(100));
        // A user detail above 4 counts as 5; a +1 light offset reaches 6.
        // Cube 0x800 halved is 1024, the light's shift of 1 makes it 512.
        let q = allocation_quality(0, 1, 9);
        assert_eq!(q, 6);
        assert_eq!(size(ShadowKind::Cube, q, 1, 8192), Some(512));
        // The shift is taken modulo 32.
        assert_eq!(size(ShadowKind::Cube, 6, 33, 8192), Some(512));
        // Index 0 means no map.
        assert_eq!(shadow_allocation(ShadowKind::Cube, 0, 0, 8192), None);
        assert_eq!(
            allocate_shadow(ShadowKind::Directional, 0, 0, 0, 0, 8192),
            None
        );
        // A light offset that cancels the detail also gives no map.
        assert_eq!(allocate_shadow(ShadowKind::Cube, 2, -2, 0, 4, 8192), None);
    }

    #[test]
    fn nothing_clamps_the_allocation_index() {
        // A non-zero detail is used as given, whatever its size.
        assert_eq!(allocation_quality(8, 0, 4), 8);
        // Indices outside 1..=6 still allocate, at the fallback sizes:
        // 0x800 directional and 0x200 halved for a cube.
        assert_eq!(size(ShadowKind::Directional, 8, 0, 8192), Some(0x800));
        assert_eq!(size(ShadowKind::Cube, 8, 0, 8192), Some(0x100));
        assert_eq!(size(ShadowKind::Directional, -2, 0, 8192), Some(0x800));
        assert_eq!(size(ShadowKind::Cube, -2, 0, 8192), Some(0x100));
        // A negative user detail is used as it stands.
        assert_eq!(allocation_quality(0, 0, -2), -2);
        // The user detail 4 stays 4; only larger values become 5.
        assert_eq!(allocation_quality(0, 0, 4), 4);
        assert_eq!(allocation_quality(0, 0, 5), 5);
        assert_eq!(allocation_quality(0, 0, 99), 5);
    }

    #[test]
    fn the_cube_draw_sizes_its_own_map() {
        // The shipped setting 4: index 4, offset 0.02, 0x200 halved.
        let d = cube_draw_settings(0, 0, 4);
        assert_eq!(d.quality, 4);
        assert!((d.light_offset - 0.02).abs() < 1e-7);
        assert!(!d.high_detail);
        assert_eq!(d.face_size, 256);
        // A user detail of 5 raises the flag and makes 512 faces; a light
        // offset past the top clamps to index 6 with the 0.025 offset.
        let d = cube_draw_settings(0, 0, 5);
        assert_eq!((d.quality, d.face_size), (5, 512));
        assert!(d.high_detail);
        let d = cube_draw_settings(3, 0, 5);
        assert_eq!((d.quality, d.face_size), (6, 1024));
        assert_eq!(d.light_offset, 0.025);
        // The renderer's own detail replaces the user's and is not clamped;
        // an index past the table falls back to 0x200 halved, the offset
        // stays at 0.025.
        let d = cube_draw_settings(4, 3, 4);
        assert_eq!((d.quality, d.face_size), (7, 256));
        assert_eq!(d.light_offset, 0.025);
        // A negative index multiplies through (a negative offset); a zero
        // index falls back to the same size as the default.
        let d = cube_draw_settings(-4, 3, 4);
        assert_eq!((d.quality, d.face_size), (-1, 256));
        assert!((d.light_offset + 0.005).abs() < 1e-8);
        let d = cube_draw_settings(0, 0, 0);
        assert_eq!((d.quality, d.face_size, d.light_offset), (0, 256, 0.0));
        // The draw sizing has no shift and no hardware cap.
        assert_eq!(cube_draw_settings(1, 0, 5).face_size, 1024);
    }

    #[test]
    fn cube_light_offsets_are_five_thousandths_per_step() {
        assert_eq!(cube_light_offset(0), 0.0);
        assert!((cube_light_offset(4) - 0.02).abs() < 1e-7);
        assert!((cube_light_offset(5) - 0.025).abs() < 1e-7);
        assert_eq!(cube_light_offset(6), 0.025);
        assert_eq!(cube_light_offset(99), 0.025);
    }

    #[test]
    fn a_cube_face_looks_ninety_degrees_over_the_lights_radius() {
        let p = cube_projection(6.7, 0.03);
        assert_eq!(p.fov, std::f32::consts::FRAC_PI_2);
        assert_eq!(p.aspect_ratio, 1.0);
        assert_eq!(p.near, 0.03);
        assert_eq!(p.far, 6.7);
        // The creation's near plane is 10 engine units (10 cm).
        assert_eq!(cube_projection(670.0, 10.0).near, 10.0);
    }

    #[test]
    fn the_base_map_size_slot_stores_both_dimensions() {
        let mut size = ShadowMapSize::default();
        size.set(512, 256);
        assert_eq!(
            size,
            ShadowMapSize {
                width: 512,
                height: 256
            }
        );
        let direct = DirectShadowSettings::new(4096, 4096);
        assert_eq!(direct.near, 10.0);
        assert_eq!(direct.far, 2_750.0);
        assert_eq!(direct.faces, ALL_FACES);
        // The map owns its texture, and its first render starts from the
        // constructor's offset seeds before recomputing both.
        assert!(direct.own_texture);
        assert_eq!(direct.polygon_factor, 1.0);
        assert_eq!(direct.polygon_units, 100.0);
    }

    #[test]
    fn the_sun_map_is_a_linear_clamped_compare_texture() {
        // The creation's mode word 0x33: linear minification (bit 5) and
        // linear magnification (bit 4).
        assert_eq!(
            sampler_filters(0x33),
            (ShadowFilter::Linear, ShadowFilter::Linear)
        );
        // Neither bit: nearest both ways.
        assert_eq!(
            sampler_filters(0x00),
            (ShadowFilter::Nearest, ShadowFilter::Nearest)
        );
        // The mipmapped variants (bits 6 and 7) keep the filter's family: a
        // nearest minification stays nearest, a linear one linear.
        assert_eq!(
            sampler_filters(0x20),
            (ShadowFilter::Linear, ShadowFilter::Nearest)
        );
        assert_eq!(
            sampler_filters(0xD0),
            (ShadowFilter::Nearest, ShadowFilter::Linear)
        );
        // The sun's map: linear, clamped both ways, compared as it is read,
        // and 16 bits of depth.
        assert_eq!(
            SUN_SHADOW_SAMPLER,
            ShadowSampler {
                min: ShadowFilter::Linear,
                mag: ShadowFilter::Linear,
                wrap_s: ShadowWrap::ClampToEdge,
                wrap_t: ShadowWrap::ClampToEdge,
                compare: true,
            }
        );
        assert_eq!(SUN_SHADOW_DEPTH_BITS, 16);
    }

    #[test]
    fn the_caster_cull_switches_only_on_a_change() {
        let mut cull = CasterCull::default();
        assert_eq!(cull.begin_pass(), CullFace::Back);
        // The first back-cull caster changes nothing.
        assert_eq!(cull.set(false), None);
        assert_eq!(cull.set(true), Some(CullFace::Front));
        assert_eq!(cull.set(true), None);
        assert!(cull.front());
        assert_eq!(cull.set(false), Some(CullFace::Back));
        // The pass end culls back faces but leaves the flag; the next pass
        // clears it.
        assert_eq!(cull.set(true), Some(CullFace::Front));
        assert_eq!(cull.end_pass(), CullFace::Back);
        assert!(cull.front());
        assert_eq!(cull.begin_pass(), CullFace::Back);
        assert!(!cull.front());
    }

    #[test]
    fn caster_programs_follow_the_builders_flags() {
        assert_eq!(caster_program(false, false, false), caster::DEPTH);
        assert_eq!(caster_program(true, false, false), caster::DEPTH_CUT);
        assert_eq!(caster_program(false, true, false), caster::DISTANCE);
        assert_eq!(caster_program(true, true, false), caster::DISTANCE_CUT);
        assert_eq!(caster_program(false, true, true), caster::DISTANCE_FADE);
        // The fade stage exists only for a distance writer without an alpha
        // test; the other draws ignore the flag.
        assert_eq!(caster_program(true, true, true), caster::DISTANCE_CUT);
        assert_eq!(caster_program(false, false, true), caster::DEPTH);
        assert_eq!(caster_program(true, false, true), caster::DEPTH_CUT);
        assert!(DITHER_FADE_WGSL.contains("discard"));
        assert!(DITHER_FADE_WGSL.contains("3.555713358"));
        assert!(DITHER_FADE_WGSL.contains("0.309269245"));
    }

    #[test]
    fn the_cube_texture_pool_references_entries_by_size() {
        let mut pool = CubeTexPool::new();
        let a = pool.acquire(256);
        assert_eq!(pool.entries().len(), 1);
        assert_eq!(pool.get(256).unwrap().references, 1);
        let b = pool.acquire(256);
        assert_eq!(a, b);
        assert_eq!(pool.get(256).unwrap().references, 2);
        let c = pool.acquire(512);
        assert_ne!(a, c);
        assert_eq!(pool.entries().len(), 2);
        pool.release(a);
        assert_eq!(pool.get(256).unwrap().references, 1);
        pool.release(a);
        assert!(pool.get(256).is_none());
        // The 512 entry moved into the freed slot.
        assert_eq!(pool.entries().len(), 1);
        assert_eq!(pool.entries()[0].size, 512);
        // An unknown handle changes nothing.
        pool.release(999);
        assert_eq!(pool.entries().len(), 1);
    }

    #[test]
    fn the_cube_map_pool_hands_maps_of_equal_size_to_idle_owners() {
        let mut pool = CubeMapPool::new(4);
        assert_eq!(pool.user_detail(), 4);
        // Nothing of the size exists: a new map.
        let first = pool.assign(10, 256, |_| false);
        assert_eq!(
            first,
            CubeMapGrant {
                slot: 0,
                evicted: None
            }
        );
        // An owner still in use keeps its map; the next light gets another.
        let second = pool.assign(11, 256, |light| light == 10);
        assert_eq!((second.slot, second.evicted), (1, None));
        // A different size never matches.
        let other = pool.assign(12, 512, |_| false);
        assert_eq!((other.slot, other.evicted), (2, None));
        // With every owner idle the first map of the size is taken over and
        // its owner told.
        let stolen = pool.assign(13, 256, |_| false);
        assert_eq!((stolen.slot, stolen.evicted), (0, Some(10)));
        assert_eq!(pool.maps()[0].owner, Some(13));
        // A destroyed light leaves its map without an owner, which is then
        // taken without eviction.
        pool.release_owner(1);
        let reused = pool.assign(14, 256, |_| true);
        assert_eq!((reused.slot, reused.evicted), (1, None));
        // A detail change drops everything and names the owners.
        let mut owners = pool.set_detail(5);
        owners.sort_unstable();
        assert_eq!(owners, vec![12, 13, 14]);
        assert!(pool.maps().is_empty());
        assert_eq!(pool.user_detail(), 5);
    }

    fn assert_close(a: Vec3, b: Vec3, tolerance: f32) {
        assert!((a - b).abs().max_element() <= tolerance, "{a} vs {b}");
    }

    #[test]
    fn the_shadow_view_rows_are_the_axes_and_takes_the_light_to_the_origin() {
        let tolerance = 1e-3;
        let position = Vec3::new(1.0, 2.0, 3.0);
        let view = shadow_view(position, -Vec3::Z, Vec3::Y);
        // Looking down −Z with +Y up is a diagonal basis (the estimate of
        // the reciprocal root leaves the axes within 1.5/4096 of unit).
        assert_close(view.x_axis.truncate(), Vec3::new(1.0, 0.0, 0.0), tolerance);
        assert_close(view.y_axis.truncate(), Vec3::new(0.0, 1.0, 0.0), tolerance);
        assert_close(view.z_axis.truncate(), Vec3::new(0.0, 0.0, -1.0), tolerance);
        assert_close(view.transform_point3(position), Vec3::ZERO, 2e-3);
        // A side view: forward +X, up +Z gives the rows (0, −1, 0), (0, 0, 1)
        // and (1, 0, 0), so the point one unit ahead lands at z = 1.
        let side = shadow_view(Vec3::ZERO, Vec3::X, Vec3::Z);
        assert_close(
            side.transform_point3(Vec3::X),
            Vec3::new(0.0, 0.0, 1.0),
            tolerance,
        );
        assert_close(
            side.transform_point3(Vec3::Y),
            Vec3::new(-1.0, 0.0, 0.0),
            tolerance,
        );
        assert_close(
            side.transform_point3(Vec3::Z),
            Vec3::new(0.0, 1.0, 0.0),
            tolerance,
        );
        // The matrix columns are the rows' components, not the axes.
        assert!((side.x_axis.z - 1.0).abs() < tolerance);
        assert!((side.y_axis.x + 1.0).abs() < tolerance);
        assert!((side.z_axis.y - 1.0).abs() < tolerance);
    }

    #[test]
    fn a_zero_axis_stays_zero_instead_of_becoming_not_a_number() {
        // Forward parallel to up gives a zero first axis.
        let view = shadow_view(Vec3::ONE, Vec3::Y, Vec3::Y);
        assert!(view.to_cols_array().iter().all(|v| v.is_finite()));
        assert_eq!(view.x_axis, Vec4::ZERO);
    }

    #[test]
    fn the_orthographic_projection_maps_the_box_to_the_unit_cube() {
        let min = Vec3::new(-2.0, 1.0, 10.0);
        let max = Vec3::new(6.0, 5.0, 2750.0);
        let m = shadow_ortho(min, max);
        assert!((m.x_axis.x - 0.25).abs() < 1e-6);
        assert!((m.y_axis.y - 0.5).abs() < 1e-6);
        assert!((m.z_axis.z - 2.0 / 2740.0).abs() < 1e-7);
        assert!((m.w_axis.x + 0.5).abs() < 1e-6);
        assert!((m.w_axis.y + 1.5).abs() < 1e-6);
        assert!((m.w_axis.z + 2760.0 / 2740.0).abs() < 1e-6);
        assert_eq!(m.w_axis.w, 1.0);
        let p = m.project_point3((min + max) * 0.5);
        assert!((p.x).abs() < 1e-6 && (p.y).abs() < 1e-6 && (p.z).abs() < 1e-6);
        let corner = m.project_point3(max);
        assert!((corner.x - 1.0).abs() < 1e-6);
        assert!((corner.y - 1.0).abs() < 1e-6);
        assert!((corner.z - 1.0).abs() < 1e-6);
    }
}

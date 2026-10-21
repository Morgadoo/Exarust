//! Shadow-map plumbing: the size each light's map gets from the shadow
//! detail setting and the light's own offsets, the two map classes
//! (directional depth map or cube), the caster programs, the pooled cube
//! depth textures and the matrices the directional map builds.
//!
//! The original picks the class by the light's kind: kind 2 allocates the
//! directional (`TShadowMapDirect`) class, anything else the cube class.
//! Sizes come from a table indexed by the shadow detail plus the light's
//! quality offset, then halved and shifted for cubes, and capped by the
//! hardware's maximum. The caster programs write distance to the light as
//! depth, optionally discarding alpha-tested texels and optionally a
//! dithered fade pattern.
//!
//! Exarust renders the cube maps through Bevy (`cube_shadows`) at its fixed
//! top size and the sun through Bevy's shadow map; the rules here are the
//! sizes, classes, programs and pool arithmetic the original uses, pinned
//! by tests, for the settings changes and the map sizes to follow.
//!
//! Rules: `docs/RENDERING.md` (Shadows).

use bevy::prelude::*;

/// The user's default shadow detail (the original's stored setting).
pub const DEFAULT_SHADOW_DETAIL: i32 = 4;
/// The detail the original treats as its top for the settings (larger
/// stored values clamp to this).
pub const TOP_SHADOW_DETAIL: i32 = 5;
/// The highest table index (the original clamps the combined index to 6).
pub const MAX_SHADOW_QUALITY: i32 = 6;
/// The hardware's maximum face size in the shipped data. The game
/// overwrites this from its hardware configuration; Exarust's pool is its
/// own constant.
pub const MAX_SHADOW_FACE: u32 = 256;

/// The directional map's sizes for quality index 0..6 (the original's
/// table; index 0 is the unset default).
const DIRECT_SIZES: [u32; 7] = [0x800, 0x100, 0x200, 0x400, 0x800, 0x1000, 0x2000];
/// The cube map's sizes for quality index 0..6, before the halving and the
/// light's shift.
const CUBE_SIZES: [u32; 7] = [0x200, 0x40, 0x80, 0x100, 0x200, 0x400, 0x800];

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

/// The combined quality index: a detail of 0 takes the user's setting
/// (clamped to the top), then the light's quality offset is added and the
/// sum clamped to [`MAX_SHADOW_QUALITY`].
pub fn shadow_quality(detail: i32, light_offset: i32, user_detail: i32) -> i32 {
    let mut q = if detail == 0 {
        user_detail.clamp(0, TOP_SHADOW_DETAIL)
    } else {
        detail
    };
    q += light_offset;
    q.min(MAX_SHADOW_QUALITY)
}

/// The allocation a light of `kind` gets for quality index `q`, or `None`
/// at index 0 (the original returns no map). Directional maps take the
/// table value directly; cube maps halve it and shift it right by the
/// light's reduction level `shift`. Both are capped at `max_face`.
pub fn shadow_allocation(
    kind: ShadowKind,
    q: i32,
    shift: u32,
    max_face: u32,
) -> Option<ShadowAllocation> {
    if q <= 0 {
        return None;
    }
    let index = (q as usize).min(MAX_SHADOW_QUALITY as usize);
    let size = match kind {
        ShadowKind::Directional => DIRECT_SIZES[index],
        ShadowKind::Cube => (CUBE_SIZES[index] >> 1) >> (shift & 0x1F),
    }
    .min(max_face);
    Some(ShadowAllocation { kind, size })
}

/// The original's allocator: the quality index from a light's kind, detail
/// and offsets, then the map of the matching class.
pub fn allocate_shadow(
    kind: ShadowKind,
    detail: i32,
    light_offset: i32,
    shift: u32,
    user_detail: i32,
    max_face: u32,
) -> Option<ShadowAllocation> {
    let q = shadow_quality(detail, light_offset, user_detail);
    shadow_allocation(kind, q, shift, max_face)
}

/// The cube light's fade distance for quality index `q`: `0.005·q` in
/// double precision for indices up to 5, else 0.025 (also the value for a
/// quality above the table).
pub fn cube_fade_distance(q: i32) -> f32 {
    if q < 6 {
        (f64::from(q) * 0.005) as f32
    } else {
        0.025
    }
}

/// The map-size fields a shadow map carries (`+0x78`, `+0x7C`), as the
/// original's base-class slot sets them.
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
/// and its initial needed/dirty mask `0x3F` (all six faces).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DirectShadowSettings {
    pub size: ShadowMapSize,
    pub near: f32,
    pub far: f32,
    pub faces: u32,
}

impl DirectShadowSettings {
    /// The constructor for a `width` × `height` map.
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: ShadowMapSize { width, height },
            near: 10.0,
            far: 2_750.0,
            faces: 0x3F,
        }
    }
}

/// A dither-discard state: the original switches the fragment program
/// between its fade variant (0x404) and the plain one (0x405), remembering
/// the last choice so an unchanged flag costs nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DitherState {
    enabled: bool,
}

impl DitherState {
    /// Sets the fade variant and reports whether a program switch is due
    /// (the flag changed).
    pub fn set(&mut self, enabled: bool) -> bool {
        if enabled == self.enabled {
            return false;
        }
        self.enabled = enabled;
        true
    }

    /// Whether the fade variant is current.
    pub fn enabled(&self) -> bool {
        self.enabled
    }
}

/// Which caster program a draw uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CasterProgram {
    /// Texture coordinates are read and an alpha test discards below ½.
    pub alpha_test: bool,
    /// The fragment writes radial distance to the light as depth.
    pub writes_distance: bool,
    /// A dithered fade pattern discards fragments by the light's fade.
    pub dithered: bool,
}

/// The caster programs the original compiles: a plain depth pass, a
/// distance writer, and each with an alpha test and a dithered fade.
pub mod caster {
    use super::CasterProgram;

    /// Position only, no fragment work.
    pub const DEPTH: CasterProgram = CasterProgram {
        alpha_test: false,
        writes_distance: false,
        dithered: false,
    };
    /// Distance to the light written as depth.
    pub const DISTANCE: CasterProgram = CasterProgram {
        alpha_test: false,
        writes_distance: true,
        dithered: false,
    };
    /// Position and texture coordinates, alpha discarded below ½.
    pub const DEPTH_CUT: CasterProgram = CasterProgram {
        alpha_test: true,
        writes_distance: false,
        dithered: false,
    };
    /// Distance and texture coordinates, alpha discarded below ½.
    pub const DISTANCE_CUT: CasterProgram = CasterProgram {
        alpha_test: true,
        writes_distance: true,
        dithered: false,
    };
    /// Distance plus a dither discard by the light's fade.
    pub const DISTANCE_FADE: CasterProgram = CasterProgram {
        alpha_test: false,
        writes_distance: true,
        dithered: true,
    };
    /// Distance, texture coordinates, alpha and dither discards.
    pub const DISTANCE_CUT_FADE: CasterProgram = CasterProgram {
        alpha_test: true,
        writes_distance: true,
        dithered: true,
    };
}

/// Selects the caster program, as the original's program builder does:
/// texture coordinates only for an alpha-tested draw, distance only for a
/// cube face, and the dithered fragment stage when asked.
pub fn caster_program(alpha_test: bool, cube_face: bool, dithered: bool) -> CasterProgram {
    CasterProgram {
        alpha_test,
        writes_distance: cube_face,
        dithered: cube_face && dithered,
    }
}

/// The dither pattern of the fade fragment stage, in WGSL: a screen-space
/// hash from the fragment's coordinates, discarding below the fade value.
pub const DITHER_FADE_WGSL: &str = r#"
// fragment of the dithered fade caster
@fragment
fn dithered_fade(@builtin(position) at: vec4<f32>, fade: f32, depth: f32) -> @location(0) f32 {
    let a = fract(at.x * 3.555713358 + at.y * 0.309269245);
    if a < fade {
        discard;
    }
    return depth;
}
"#;

/// A cube depth texture pool entry: the texture size and how many lights
/// hold it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PooledCubeTex {
    pub size: u32,
    pub references: u32,
}

/// The shared cube depth textures, by face size, reference-counted as the
/// original's pool keeps them: a request for a size already present takes
/// the existing entry and raises its count; a last release frees the entry
/// by swapping the last one into its place.
#[derive(Clone, Debug, Default)]
pub struct CubeTexPool {
    entries: Vec<PooledCubeTex>,
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
    /// count raised, the original's returned index), or a new entry (count
    /// 1).
    pub fn acquire(&mut self, size: u32) -> usize {
        if let Some(index) = self.entries.iter().position(|e| e.size == size) {
            self.entries[index].references += 1;
            return index;
        }
        self.entries.push(PooledCubeTex {
            size,
            references: 1,
        });
        self.entries.len() - 1
    }

    /// Drops a reference to entry `index`; at zero the texture is freed and
    /// the last entry moved into its slot.
    pub fn release(&mut self, index: usize) {
        let Some(entry) = self.entries.get_mut(index) else {
            return;
        };
        entry.references = entry.references.saturating_sub(1);
        if entry.references < 1 {
            self.entries.swap_remove(index);
        }
    }

    /// Drops every texture, as a shadow-detail change does.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// The entry of a size, if the pool holds one.
    pub fn get(&self, size: u32) -> Option<&PooledCubeTex> {
        self.entries.iter().find(|e| e.size == size)
    }
}

/// The view matrix of a directional shadow map, as the original builds it
/// from the light's position, its forward axis and an up vector: the
/// columns are `normalize(cross(forward, up))`, `normalize(cross(first,
/// forward))` and the forward axis, with the translation that takes the
/// light's position to the origin.
pub fn shadow_view(position: Vec3, forward: Vec3, up: Vec3) -> Mat4 {
    let first = forward.cross(up).normalize_or_zero();
    let second = first.cross(forward).normalize_or_zero();
    let third = forward;
    let translation = Vec3::new(
        -first.dot(position),
        -second.dot(position),
        -third.dot(position),
    );
    Mat4::from_cols(
        first.extend(0.0),
        second.extend(0.0),
        third.extend(0.0),
        translation.extend(1.0),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shadow_detail_and_offsets_pick_the_size() {
        // The user's detail 4 with no offset, cube: 0x200/2 = 256.
        let q = shadow_quality(0, 0, DEFAULT_SHADOW_DETAIL);
        assert_eq!(q, 4);
        assert_eq!(
            shadow_allocation(ShadowKind::Cube, q, 0, 8192),
            Some(ShadowAllocation {
                kind: ShadowKind::Cube,
                size: 256
            })
        );
        // Directional, index 4: 0x800, capped at the shipped maximum 256.
        assert_eq!(
            shadow_allocation(ShadowKind::Directional, 4, 0, 8192),
            Some(ShadowAllocation {
                kind: ShadowKind::Directional,
                size: 2048
            })
        );
        // Capped by the hardware's shipped maximum of 256.
        assert_eq!(
            shadow_allocation(ShadowKind::Directional, 4, 0, MAX_SHADOW_FACE).map(|a| a.size),
            Some(MAX_SHADOW_FACE)
        );
        // Detail 5 with a +1 light offset clamps to index 6: cube 0x800/2
        // = 1024, then the light's shift of 1 halves it to 512.
        let q = shadow_quality(0, 1, TOP_SHADOW_DETAIL);
        assert_eq!(q, 6);
        assert_eq!(
            shadow_allocation(ShadowKind::Cube, q, 1, 8192).map(|a| a.size),
            Some(512)
        );
        // The largest size never exceeds the hardware maximum.
        assert_eq!(
            shadow_allocation(ShadowKind::Cube, 6, 0, MAX_SHADOW_FACE).map(|a| a.size),
            Some(MAX_SHADOW_FACE)
        );
        // Index 0 means no map.
        assert_eq!(shadow_allocation(ShadowKind::Cube, 0, 0, 8192), None);
        assert_eq!(
            allocate_shadow(ShadowKind::Directional, 0, 0, 0, 0, 8192),
            None
        );
        // An oversized user detail clamps to the top.
        assert_eq!(shadow_quality(0, 0, 99), TOP_SHADOW_DETAIL);
    }

    #[test]
    fn cube_fade_distances_are_five_thousandths_per_step() {
        assert_eq!(cube_fade_distance(0), 0.0);
        assert!((cube_fade_distance(4) - 0.02).abs() < 1e-7);
        assert!((cube_fade_distance(5) - 0.025).abs() < 1e-7);
        assert_eq!(cube_fade_distance(6), 0.025);
        assert_eq!(cube_fade_distance(99), 0.025);
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
        assert_eq!(direct.faces, 0x3F);
    }

    #[test]
    fn the_dither_state_switches_only_on_a_change() {
        let mut dither = DitherState::default();
        assert!(!dither.enabled());
        assert!(dither.set(true));
        assert!(!dither.set(true));
        assert!(dither.set(false));
        assert!(!dither.set(false));
        assert!(!dither.enabled());
    }

    #[test]
    fn caster_programs_take_the_alpha_and_distance_stages() {
        assert_eq!(
            caster_program(false, false, false),
            CasterProgram {
                alpha_test: false,
                writes_distance: false,
                dithered: false
            }
        );
        assert_eq!(caster_program(true, true, true), caster::DISTANCE_CUT_FADE);
        assert_eq!(caster_program(true, false, true), caster::DEPTH_CUT);
        assert_eq!(caster_program(false, true, true), caster::DISTANCE_FADE);
        // A depth pass never dithers (the fade stage is only in the cube
        // program).
        assert!(!caster_program(false, false, true).dithered);
        assert!(DITHER_FADE_WGSL.contains("discard"));
    }

    #[test]
    fn the_cube_texture_pool_references_entries_by_size() {
        let mut pool = CubeTexPool::new();
        let a = pool.acquire(256);
        assert_eq!(pool.entries().len(), 1);
        assert_eq!(pool.entries()[a].references, 1);
        let b = pool.acquire(256);
        assert_eq!(a, b);
        assert_eq!(pool.get(256).unwrap().references, 2);
        let c = pool.acquire(512);
        assert_eq!(c, 1);
        assert_eq!(pool.entries().len(), 2);
        pool.release(a);
        assert_eq!(pool.get(256).unwrap().references, 1);
        pool.release(a);
        assert!(pool.get(256).is_none());
        assert_eq!(pool.entries().len(), 1);
        // The 512 entry moved into the freed slot.
        assert_eq!(pool.entries()[0].size, 512);
        pool.clear();
        assert!(pool.entries().is_empty());
    }

    #[test]
    fn the_shadow_view_takes_the_light_to_the_origin() {
        let view = shadow_view(Vec3::new(1.0, 2.0, 3.0), -Vec3::Z, Vec3::Y);
        // Looking down −Z with +Y up is the identity basis; the translation
        // is the negated position in that basis.
        assert_eq!(view.x_axis, Vec4::new(1.0, 0.0, 0.0, 0.0));
        assert_eq!(view.y_axis, Vec4::new(0.0, 1.0, 0.0, 0.0));
        assert_eq!(view.z_axis, Vec4::new(0.0, 0.0, -1.0, 0.0));
        assert_eq!(view.w_axis, Vec4::new(-1.0, -2.0, 3.0, 1.0));
        assert_eq!(view.transform_point3(Vec3::new(1.0, 2.0, 3.0)), Vec3::ZERO);
        // A side axis crosses the forward and up vectors.
        let side = shadow_view(Vec3::ZERO, Vec3::X, Vec3::Z);
        assert_eq!(side.x_axis, Vec4::new(0.0, -1.0, 0.0, 0.0));
        assert_eq!(side.y_axis, Vec4::new(0.0, 0.0, 1.0, 0.0));
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

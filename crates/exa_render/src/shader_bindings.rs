//! Where a compiled surface program's uniforms and samplers sit: the texture
//! units the original gives each map slot, the uniform-block bindings, the
//! sampler uniforms it looks up once per program, and the shadow-tap upload
//! window.
//!
//! The engine fills a small record per program: its `program` handle, a
//! pointer to six cached uniform locations (`ObjMat`, `LqdLvl`, `Light`,
//! `SdwSmp`, `VXGIPos`, `PassIdx`), and it binds `Global` to block 0 and
//! `Material` to block 1. The texture units are packed from 0 up in the
//! order the maps were declared, except that `EnvMap`, `VoxelGI`, `FrmBuf`
//! and the shadow cubes sit on the fixed units 5, 6, 7 and 8 up. Because the
//! original binds by location, a map the program optimised away is a no-op
//! (`glUniform1i(-1, …)`), which this module models as `None`.
//!
//! The order and the conditions are those of the engine's uniform-location
//! pass; Exarust's own renderer binds through Bevy, so the table is kept for
//! comparison and tests (`docs/RENDERING.md`, Shader builder).

use crate::shader_program::MAX_LIGHTS;
use crate::shader_template::Flags;

/// The uniform-block bindings the engine sets for every surface program.
pub const GLOBAL_BLOCK_BINDING: u32 = 0;
/// See [`GLOBAL_BLOCK_BINDING`].
pub const MATERIAL_BLOCK_BINDING: u32 = 1;

/// The fixed texture units of the environment and shadow samplers.
pub const ENVIRONMENT_UNIT: u32 = 5;
/// The voxel-GI volume's unit.
pub const VOXEL_GI_UNIT: u32 = 6;
/// The screen-copy unit (`FrmBuf`, used by refraction).
pub const FRAME_BUFFER_UNIT: u32 = 7;
/// The first shadow cube's unit; the maps follow one another.
pub const FIRST_SHADOW_UNIT: u32 = 8;

/// A material word's sampler bits (the same values the templates read).
mod sampler {
    pub const DIFFUSE: u32 = 0x20_000;
    pub const SPECULAR: u32 = 0x4_0000;
    pub const EMISSIVE: u32 = 0x8_0000;
    pub const NORMAL: u32 = 0x20_0000;
    pub const OPACITY: u32 = 0x40_0000;
    pub const HEIGHT: u32 = 0x80_0000;
    pub const GLOSS: u32 = 0x100_0000;
    pub const DIRT: u32 = 0x800_0000;
    pub const COLOUR_SLOTS: u32 = 0x48_0000;
    pub const LIT: u32 = 0x7;
    pub const LIT_OR_SPECULAR: u32 = 0x3;
    pub const BLEND: u32 = 0x200;
}

/// A shader word's displacement bit.
const DISPLACEMENT: u32 = 0x100;

/// The texture unit of every sampler a program can declare.
///
/// `None` means the engine did not have a location for that sampler (the
/// program dropped it), so its unit was never assigned. The map slots are
/// numbered from 0 in declaration order; the fixed units can collide with
/// them, as the original's can (a lit material with a height map puts
/// `DspTex` on unit 5, where `EnvMap` also sits).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TextureUnits {
    pub diffuse: Option<u32>,
    pub emissive: Option<u32>,
    pub specular: Option<u32>,
    pub normal: Option<u32>,
    pub gloss: Option<u32>,
    pub opacity: Option<u32>,
    pub displacement: Option<u32>,
    pub dirt: Option<u32>,
    pub environment: Option<u32>,
    pub voxel_gi: Option<u32>,
    pub frame_buffer: Option<u32>,
    /// The shadow maps, one per gathered light, in slot order.
    pub shadows: [Option<u32>; MAX_LIGHTS],
}

/// The uniform locations the engine caches for a program.
///
/// The original stores handles (or `-1` when the program has no such
/// uniform); `true` here means the lookup found one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct UniformLocations {
    /// `ObjMat`, the per-object environment word and the light count.
    pub object: bool,
    /// `LqdLvl`, the liquid surface's level.
    pub liquid_level: bool,
    /// `Light`, the gathered lights' records.
    pub lights: bool,
    /// `SdwSmp`, the shadow-tap offsets.
    pub shadow_samples: bool,
    /// `VXGIPos`, the GI volume's corner.
    pub voxel_gi_position: bool,
    /// `PassIdx`, the super-sampling pass.
    pub pass_index: bool,
}

/// Everything bound for one program: the sampler units and the cached
/// uniform locations.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ProgramBindings {
    pub units: TextureUnits,
    pub locations: UniformLocations,
}

/// The bindings of a program built for `flags` with `lights` gathered light
/// slots.
///
/// The material word decides which maps exist, the shader word whether the
/// height map also provides displacement shadows, the environment word
/// whether the GI volume is bound, and the object word whether the pass
/// index is uploaded.
pub fn program_bindings(flags: &Flags, lights: usize) -> ProgramBindings {
    let material = flags.material;
    let mut units = TextureUnits::default();
    // The colour slot: the diffuse map when the material has one, else the
    // opacity map when that is the only colour-using map. Both claim unit 0.
    let mut next = 0u32;
    if material & sampler::DIFFUSE == 0 {
        if material & sampler::COLOUR_SLOTS == sampler::OPACITY {
            units.opacity = Some(next);
            next += 1;
        }
    } else {
        units.diffuse = Some(next);
        next += 1;
    }
    if material & sampler::EMISSIVE != 0 {
        units.emissive = Some(next);
        next += 1;
    }
    if material & sampler::SPECULAR != 0 {
        units.specular = Some(next);
        next += 1;
    }
    if material & sampler::NORMAL != 0 {
        units.normal = Some(next);
        next += 1;
    }
    if material & sampler::GLOSS != 0 {
        units.gloss = Some(next);
        next += 1;
    }
    if material & sampler::HEIGHT != 0 || flags.shader & DISPLACEMENT != 0 {
        units.displacement = Some(next);
        next += 1;
    }
    if material & sampler::DIRT != 0 {
        // The dirt map does not advance the counter: it takes the number
        // after the height map's, which the next test also uses.
        units.dirt = Some(next);
    }
    if material & sampler::LIT != 0 {
        units.environment = Some(ENVIRONMENT_UNIT);
    }
    let volume = material & sampler::LIT_OR_SPECULAR != 0 || flags.environment & 0x10 != 0;
    if volume {
        units.voxel_gi = Some(VOXEL_GI_UNIT);
    }
    if material & 0x4 != 0 {
        units.frame_buffer = Some(FRAME_BUFFER_UNIT);
    }
    for (i, slot) in units
        .shadows
        .iter_mut()
        .enumerate()
        .take(lights.min(MAX_LIGHTS))
    {
        *slot = Some(FIRST_SHADOW_UNIT + i as u32);
    }

    ProgramBindings {
        units,
        locations: UniformLocations {
            // `ObjMat` is always looked up, whatever the flags.
            object: true,
            liquid_level: material & sampler::BLEND == 0x200,
            lights: lights > 0,
            shadow_samples: volume,
            voxel_gi_position: flags.environment & 0x10 != 0,
            pass_index: flags.object & 0x1000 != 0,
        },
    }
}

/// How many floats of the `SdwSmp` array a sub-frame uploads, and where in
/// the table its first tap is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowSampleUpload {
    /// The first tap's index in the table (three floats a tap).
    pub first_tap: usize,
    /// The number of floats written.
    pub floats: usize,
}

/// The `SdwSmp` window of a sub-frame.
///
/// A super-sampled program (`object & 0x1000`) keeps four taps per output
/// pixel and uploads all four; a plain one uploads the frame's taps. The
/// window starts at the pass's own taps, three floats apart. A program with
/// no tap still uploads one entry (four in quad mode), as the original's
/// count clamp does.
pub fn shadow_sample_upload(
    shadow_samples: u32,
    quad_render: bool,
    pass: u32,
) -> ShadowSampleUpload {
    let first_tap = (pass.wrapping_mul(shadow_samples)).wrapping_mul(3) as usize;
    let floats = if shadow_samples < 1 {
        if quad_render { 4 } else { 1 }
    } else if quad_render {
        (shadow_samples as usize) << 2
    } else {
        shadow_samples as usize
    };
    ShadowSampleUpload { first_tap, floats }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flags(material: u32, shader: u32, object: u32, environment: u32) -> Flags {
        Flags {
            material,
            shader,
            object,
            environment,
        }
    }

    #[test]
    fn a_lit_textured_surface_packs_its_maps_from_zero() {
        let f = flags(
            0x7 | 0x20_000 | 0x8_0000 | 0x4_0000 | 0x20_0000 | 0x100_0000,
            0,
            0x1000,
            0x10,
        );
        let b = program_bindings(&f, 0);
        assert_eq!(b.units.diffuse, Some(0));
        assert_eq!(b.units.emissive, Some(1));
        assert_eq!(b.units.specular, Some(2));
        assert_eq!(b.units.normal, Some(3));
        assert_eq!(b.units.gloss, Some(4));
        assert_eq!(b.units.environment, Some(5));
        assert_eq!(b.units.voxel_gi, Some(6));
        // The specular bit (`0x4`) binds the screen copy, as the engine's
        // refraction path does.
        assert_eq!(b.units.frame_buffer, Some(FRAME_BUFFER_UNIT));
        assert_eq!(b.units.displacement, None);
        assert_eq!(b.units.dirt, None);
        assert_eq!(b.units.shadows, [None; MAX_LIGHTS]);
        assert!(b.locations.object);
        assert!(b.locations.shadow_samples);
        assert!(b.locations.voxel_gi_position);
        assert!(b.locations.pass_index);
        assert!(!b.locations.liquid_level);
        assert!(!b.locations.lights);
    }

    #[test]
    fn an_opacity_map_alone_takes_unit_zero() {
        let b = program_bindings(&flags(0x7 | 0x40_0000, 0, 0, 0), 0);
        assert_eq!(b.units.opacity, Some(0));
        assert_eq!(b.units.diffuse, None);
        // The opacity slot needs the colour-slot word to be exactly the
        // opacity bit: an emissive bit in the same word drops it.
        let b = program_bindings(&flags(0x7 | 0x40_0000 | 0x8_0000, 0, 0, 0), 0);
        assert_eq!(b.units.opacity, None);
        assert_eq!(b.units.emissive, Some(0));
    }

    #[test]
    fn the_dirt_map_reuses_the_height_units_number() {
        let f = flags(0x7 | 0x20_000 | 0x80_0000 | 0x800_0000, 0, 0, 0);
        let b = program_bindings(&f, 0);
        assert_eq!(b.units.diffuse, Some(0));
        assert_eq!(b.units.displacement, Some(1));
        assert_eq!(b.units.dirt, Some(2));
        // A height map without the diffuse slot still claims unit 0.
        let b = program_bindings(&flags(0x7 | 0x80_0000 | 0x800_0000, 0, 0, 0), 0);
        assert_eq!(b.units.displacement, Some(0));
        assert_eq!(b.units.dirt, Some(1));
    }

    #[test]
    fn displacement_samples_bind_the_height_map_without_a_height_layer() {
        let b = program_bindings(&flags(0x7, DISPLACEMENT, 0, 0), 0);
        assert_eq!(b.units.displacement, Some(0));
    }

    #[test]
    fn shadows_follow_on_fixed_units_and_their_location_needs_lights() {
        let b = program_bindings(&flags(0x7, 0, 0, 0), 3);
        assert_eq!(b.units.shadows[0], Some(8));
        assert_eq!(b.units.shadows[1], Some(9));
        assert_eq!(b.units.shadows[2], Some(10));
        assert_eq!(b.units.shadows[3], None);
        assert!(b.locations.lights);
        // Six is the most the array holds.
        let b = program_bindings(&flags(0x7, 0, 0, 0), 9);
        assert_eq!(b.units.shadows[5], Some(13));
        assert_eq!(b.units.shadows.iter().filter(|s| s.is_some()).count(), 6);
    }

    #[test]
    fn the_shadow_sample_location_needs_a_lit_or_gi_program() {
        // An unlit emissive-only program declares no SdwSmp.
        let b = program_bindings(&flags(0x8, 0, 0, 0), 0);
        assert!(!b.locations.shadow_samples);
        assert_eq!(b.units.environment, None);
        // The GI volume alone is enough.
        let b = program_bindings(&flags(0x8, 0, 0, 0x10), 0);
        assert!(b.locations.shadow_samples);
        assert_eq!(b.units.voxel_gi, Some(VOXEL_GI_UNIT));
        // The specular bit alone binds the environment cube but neither the
        // GI volume nor the tap uniform: those need `material & 3`.
        let b = program_bindings(&flags(0x4, 0, 0, 0), 0);
        assert!(!b.locations.shadow_samples);
        assert_eq!(b.units.environment, Some(ENVIRONMENT_UNIT));
        assert_eq!(b.units.voxel_gi, None);
    }

    #[test]
    fn the_liquid_level_location_follows_the_blend_word() {
        assert!(
            !program_bindings(&flags(0, 0, 0, 0), 0)
                .locations
                .liquid_level
        );
        assert!(
            program_bindings(&flags(0x200, 0, 0, 0), 0)
                .locations
                .liquid_level
        );
        // The test is against the whole two-bit field, not just one bit.
        assert!(
            program_bindings(&flags(0x300, 0, 0, 0), 0)
                .locations
                .liquid_level
        );
        assert!(
            !program_bindings(&flags(0x100, 0, 0, 0), 0)
                .locations
                .liquid_level
        );
    }

    #[test]
    fn the_pass_index_location_needs_the_quadrant_mode() {
        assert!(!program_bindings(&flags(0, 0, 0, 0), 0).locations.pass_index);
        assert!(
            program_bindings(&flags(0, 0, 0x1000, 0), 0)
                .locations
                .pass_index
        );
    }

    #[test]
    fn sample_uploads_cover_the_pass_window() {
        let plain = shadow_sample_upload(2, false, 0);
        assert_eq!(
            plain,
            ShadowSampleUpload {
                first_tap: 0,
                floats: 2
            }
        );
        let second = shadow_sample_upload(2, false, 1);
        assert_eq!(second.first_tap, 6);
        let quad = shadow_sample_upload(2, true, 0);
        assert_eq!(quad.floats, 8);
        let quad_later = shadow_sample_upload(2, true, 3);
        assert_eq!(quad_later.first_tap, 18);
        // A program with no taps uploads one entry, four in quad mode.
        assert_eq!(shadow_sample_upload(0, false, 0).floats, 1);
        assert_eq!(shadow_sample_upload(0, true, 0).floats, 4);
    }
}

//! Smoke and mist: the original's `shadesmk_f` draw, the material type it
//! takes over, and the rules of its program.
//!
//! The smoke unit's start-up hook rewrites the third slot of the material
//! type table in place: type 2 gets the smoke draw, a key routine and the
//! mask `0x400008` ([`crate::material_state::BUILTIN_TYPES`]). The hook
//! runs with the program's unit initialisation, before the factory's reset
//! copies the table, so in 0.9.5 every type-2 record draws here; the only
//! one shipped is the wraith's `wraithskin` (its skin and robe).
//!
//! The key routine keeps the render state's material and shader words
//! under [`STATE_KEY_MASK`] and adds the shading-quality bits. The draw
//! sets the object word from the state's (plus `VrtCol` while the colour
//! attribute is on and `VrtDlt` for a deformed mesh), builds
//! `shading_v` + `shadesmk_f` once per object and environment word pair
//! ([`ProgramCache`]; a failed build is kept as −1 and the mesh is then
//! skipped), sets `UVOfst` ([`smoke_uv_offset`]) and draws the mesh; with
//! the supersampled quad passes on it repeats the draw in each pass once
//! per sub-view ([`pass_draws`]). The program's alpha is
//! [`smoke_alpha`] over a black colour.
//!
//! Exarust draws a type-2 record with the surface material's smoke branch
//! ([`crate::material::flags2::SMOKE`], `exa.wesl` and `shadesmk.wesl`).
//! The shipped program uses identifiers only some words declare
//! ([`program_builds`]); under the level's distance fog it never builds, so
//! the smoke draws nothing there.
//!
//! Rules: `docs/RENDERING.md` (Smoke and mist).

use bevy::prelude::*;

use crate::shader_program::{environment, object, shader};

/// The bits of the render state's material word (low half) and shader word
/// (high half) the smoke's key routine keeps: `MatEmi`, `TexEmi` and
/// `TexOpc`; the class nibble, `AlphaTst`, the blend-mode bit, `UseNrm` and
/// `TngtNrm`.
pub const STATE_KEY_MASK: u64 = 0x0003_C00F_0048_0008;
/// The texture scroll's cycle, milliseconds.
pub const SCROLL_PERIOD_MS: u64 = 80_000;
/// The owner node's scale the scroll starts from: the node's default (the
/// wraith's body is not rescaled).
pub const NODE_SCALE: f32 = 1.0;

/// A cached smoke program: the object and environment words it was built
/// for, and the program id (0 while the build runs, `-1` when it failed).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CachedProgram {
    pub state: u64,
    pub program: i32,
}

/// The smoke program cache: one entry per object and environment word pair,
/// created on first use. The original's entry also holds the program's
/// uniform locations, which Exarust's material binding supplies instead.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProgramCache {
    entries: Vec<CachedProgram>,
}

impl ProgramCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Finds the program for `state`, or builds it with `build` and stores
    /// the result. A failed build is stored as `-1` and not retried.
    pub fn get_or_build(&mut self, state: u64, build: impl FnOnce() -> Option<i32>) -> i32 {
        if let Some(entry) = self.entries.iter().find(|e| e.state == state) {
            return entry.program;
        }
        let program = build().unwrap_or(-1);
        self.entries.push(CachedProgram { state, program });
        program
    }

    /// The cached program for a state, if any.
    pub fn get(&self, state: u64) -> Option<i32> {
        self.entries
            .iter()
            .find(|e| e.state == state)
            .map(|e| e.program)
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Frees every cached program.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// How many times one smoke mesh is drawn in each supersampling pass: once
/// per sub-view the pass draws, each with that sub-view's viewport and
/// matrix (the level's first-pass count, at most four; one without
/// supersampling). Exarust's supersampled camera draws the scene once.
pub fn pass_draws(level: crate::view::SupersampleLevel) -> u32 {
    level.subviews().min(crate::view::SUBVIEWS_PER_PASS) as u32
}

/// The key routine's words from a render state's material word (low half)
/// and shader word (high half), before the shading-quality bits.
pub fn smoke_state_key(state: u64) -> u64 {
    state & STATE_KEY_MASK
}

/// The smoke program's words: the material and shader words under the key
/// mask, the shader word with the builder's shading-quality bits added as
/// they are (`ShadeQual` from quality 4 up, `FastLgt` up to quality 4).
pub fn program_words(material: u32, shader_word: u32, quality_bits: u32) -> (u32, u32) {
    let key = smoke_state_key(u64::from(material) | (u64::from(shader_word) << 32));
    (key as u32, (key >> 32) as u32 | quality_bits)
}

/// Whether the shipped `shadesmk_f` builds for these words. Its normal
/// include reads `vrNrm`, which the generated interface declares only with
/// `UseNrm`; it reads `vrCol.a`, declared only with `VrtCol`; and its fog
/// include's distance-fog branch (fog mode 1) reads `amb` and `b`, which
/// the program never declares. The other fog modes and the fog-free one
/// build (`exa-tool shader --smoke` expands the shipped text and lists
/// what it leaves undeclared). Exarust applies the fog rule; the wraith's
/// skin is taken to carry a colour stream, as every character's body does
/// once built (their stored meshes have none, yet wounds paint its alpha).
pub fn program_builds(shader_word: u32, object_word: u32, environment_word: u32) -> bool {
    shader_word & shader::USE_NORMAL != 0
        && object_word & object::VERTEX_COLOR != 0
        && environment_word & environment::FOG_MODE_MASK != 1
}

/// `UVOfst` at scene time `time_ms` under a parent node of scale `scale`.
pub fn smoke_uv_offset(scale: f32, time_ms: u32) -> Vec3 {
    let phase = f64::from(time_ms % SCROLL_PERIOD_MS as u32) / SCROLL_PERIOD_MS as f64;
    let t = (f64::from(scale) + phase) as f32;
    Vec3::new(0.5, 0.5 + t, t)
}

/// The program's alpha (mirrors `shadesmk.wesl`): `|N·V|·1.5 − 1` plus the
/// opacity term (the product of the two scrolled opacity reads, or 0.25
/// without an opacity texture), then `(a − 1 + vertex alpha)·0.8`. The
/// colour is black, so over the record's premultiplied blend the surface
/// darkens what is behind it by that alpha.
pub fn smoke_alpha(ndv: f32, opacity: f32, colour_alpha: f32) -> f32 {
    let a = ndv * 1.5 - 1.0 + opacity;
    (a - 1.0 + colour_alpha) * 0.8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offset_scrolls_once_each_80_seconds() {
        assert_eq!(smoke_uv_offset(1.0, 0), Vec3::new(0.5, 1.5, 1.0));
        assert_eq!(smoke_uv_offset(1.0, 40_000), Vec3::new(0.5, 2.0, 1.5));
        assert_eq!(smoke_uv_offset(1.0, 80_000), smoke_uv_offset(1.0, 0));
        assert_eq!(smoke_uv_offset(0.0, 20_000), Vec3::new(0.5, 0.75, 0.25));
        assert_eq!(smoke_uv_offset(1.0, 120_000), smoke_uv_offset(1.0, 40_000));
        // The sum is in double precision before the single-precision
        // offset is formed.
        let t = (f64::from(0.1f32) + 1.0 / 80_000.0) as f32;
        assert_eq!(smoke_uv_offset(0.1, 1), Vec3::new(0.5, 0.5 + t, t));
    }

    #[test]
    fn the_state_key_keeps_the_selected_bits_only() {
        assert_eq!(smoke_state_key(0), 0);
        assert_eq!(smoke_state_key(u64::MAX), STATE_KEY_MASK);
        assert_eq!(smoke_state_key(0x8000), 0);
        assert_eq!(smoke_state_key(0x8), 0x8);
        assert_eq!(
            smoke_state_key(0x0002_0000_0040_0000),
            0x0002_0000_0040_0000
        );
    }

    /// The wraith's skin: material word `0x400808` (emission, its opacity
    /// map and the fall-off), shader word `0x10002` (class 2 with `UseNrm`
    /// from the fall-off). The key drops the fall-off; the quality lift
    /// joins the shader word.
    #[test]
    fn the_wraiths_program_words() {
        assert_eq!(
            program_words(0x0040_0808, 0x0001_0002, 0x20_0000),
            (0x0040_0008, 0x0021_0002)
        );
        // Below shading quality 5 the builder's word carries `FastLgt`.
        assert_eq!(program_words(0x8, 0x2, 0x200).1, 0x202);
    }

    #[test]
    fn the_program_builds_only_with_its_inputs_declared_and_no_distance_fog() {
        let shd = 0x0021_0002;
        let obj = object::VERTEX_COLOR;
        assert!(program_builds(shd, obj, 0x10));
        // Fade to black (fog mode 2) declares what it reads.
        assert!(program_builds(shd, obj, 0x12));
        // Distance fog reads `amb` and `b`.
        assert!(!program_builds(shd, obj, 0x11));
        // `vrNrm` needs `UseNrm`; `vrCol` needs a colour stream.
        assert!(!program_builds(0x0020_0002, obj, 0x10));
        assert!(!program_builds(shd, 0, 0x10));
    }

    #[test]
    fn the_alpha_follows_the_facing_ratio_and_the_opacity() {
        // Facing the eye with a full opacity term and a white vertex: past
        // one, which the target clamps.
        assert!((smoke_alpha(1.0, 1.0, 1.0) - 1.2).abs() < 1e-6);
        // Without an opacity texture.
        assert!((smoke_alpha(1.0, 0.25, 1.0) - 0.6).abs() < 1e-6);
        // Edge on, a full opacity term leaves nothing; the quarter falls
        // below zero.
        assert!(smoke_alpha(0.0, 1.0, 1.0).abs() < 1e-6);
        assert!((smoke_alpha(0.0, 0.25, 1.0) + 0.6).abs() < 1e-6);
        // The vertex alpha lowers it one for one, × 0.8.
        assert!((smoke_alpha(1.0, 1.0, 0.5) - 0.8).abs() < 1e-6);
    }

    #[test]
    fn a_program_is_cached_per_state_and_a_failure_is_not_retried() {
        let mut cache = ProgramCache::new();
        let mut builds = 0;
        assert_eq!(
            cache.get_or_build(7, || {
                builds += 1;
                Some(42)
            }),
            42
        );
        assert_eq!(
            cache.get_or_build(7, || {
                builds += 1;
                Some(99)
            }),
            42
        );
        assert_eq!(builds, 1);
        assert_eq!(
            cache.get_or_build(9, || {
                builds += 1;
                None
            }),
            -1
        );
        assert_eq!(cache.get(9), Some(-1));
        assert_eq!(cache.get_or_build(9, || Some(100)), -1);
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
    }

    #[test]
    fn a_pass_draws_the_mesh_once_per_sub_view() {
        use crate::view::SupersampleLevel;
        assert_eq!(pass_draws(SupersampleLevel(0)), 1);
        assert_eq!(pass_draws(SupersampleLevel(1)), 2);
        assert_eq!(pass_draws(SupersampleLevel(3)), 4);
        // Eight sub-views take two passes of four draws each.
        assert_eq!(pass_draws(SupersampleLevel(4)), 4);
    }

    /// The shader library parses and validates as plain WGSL.
    #[test]
    fn the_smoke_library_validates() {
        let source = include_str!("shadesmk.wesl");
        let module = naga::front::wgsl::parse_str(source).expect("shadesmk.wesl parses");
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .expect("shadesmk.wesl validates");
    }
}

//! Smoke and mist: the original's `shadesmk_f` draw, which sits between the
//! divination and thaumaturgy units.
//!
//! The draw compiles `shading_v` + `shadesmk_f` once per state key and
//! caches the program, locates its `UVOfst` uniform, then draws the mesh.
//! The texture offset is animated on an 80-second cycle:
//! `UVOfst = (0.5, node-scale + (now mod 80 000) / 80 000 + 0.5)` in
//! seconds, where the node scale enters the vertical component. With the
//! supersampling quad pass count above zero the draw is repeated once per
//! quadrant, each with its own view and viewport (the original's
//! per-quadrant matrices); Exarust's supersampled scene camera already
//! draws every quadrant, so one draw per quadrant is what its pass does.
//!
//! The program's state key is the material state's selected bits:
//! `state & 0x0003_C00F_0048_0008`. Exarust runs one WESL program and
//! carries the key in the uniform; the cache still keys programs the way
//! the original does and is pinned by tests.
//!
//! Rules: `docs/RENDERING.md` (shader builder and effect draws).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::mesh::MeshVertexBufferLayoutRef;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MaterialPlugin};
use bevy::prelude::*;
use bevy::render::render_resource::{
    AsBindGroup, RenderPipelineDescriptor, ShaderType, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

pub struct SmokePlugin;

impl Plugin for SmokePlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "shadesmk.wesl");
        app.add_plugins(MaterialPlugin::<SmokeMaterial>::default())
            .init_resource::<SmokeProgramCache>()
            .add_systems(Update, scroll_smoke);
    }
}

/// The bits of a material state that select a smoke program.
pub const STATE_KEY_MASK: u64 = 0x0003_C00F_0048_0008;
/// The texture offset's cycle, milliseconds (80 s).
pub const SCROLL_PERIOD_MS: u64 = 80_000;
/// The horizontal offset the original always uses.
pub const UV_OFFSET_X: f32 = 0.5;
/// The extra half the original adds to the vertical offset.
pub const UV_OFFSET_BIAS: f32 = 0.5;

/// The texture offset at `time_ms` for a node of scale `scale`:
/// `(0.5, scale + (time_ms mod 80000)/80000 + 0.5)`.
pub fn smoke_uv_offset(scale: f32, time_ms: u64) -> Vec2 {
    let phase = (time_ms % SCROLL_PERIOD_MS) as f64 / SCROLL_PERIOD_MS as f64;
    Vec2::new(UV_OFFSET_X, scale + phase as f32 + UV_OFFSET_BIAS)
}

/// How many times the original draws the mesh: one per supersample
/// quadrant (its quad pass count is the supersample level, so a level of 3
/// is four draws).
pub fn smoke_quadrants(supersample_level: u32) -> u32 {
    supersample_level + 1
}

/// The smoke program's state key from a material state word.
pub fn smoke_state_key(state: u64) -> u64 {
    state & STATE_KEY_MASK
}

/// One cached smoke program.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SmokeProgram {
    pub key: u64,
    /// The registry index of the shader program; Exarust's program pool
    /// hands out ids, and 0 means it is the first.
    pub program: u32,
}

/// The compiled-program cache, one entry per state key, freed as a unit.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct SmokeProgramCache {
    entries: Vec<SmokeProgram>,
    next_program: u32,
}

impl SmokeProgramCache {
    /// The cached program of a key, compiling and recording one on a miss
    /// (the original builds `shading_v` + `shadesmk_f` and looks up
    /// `UVOfst`).
    pub fn get_or_insert(&mut self, key: u64) -> SmokeProgram {
        if let Some(program) = self.entries.iter().find(|p| p.key == key) {
            return *program;
        }
        let program = SmokeProgram {
            key,
            program: self.next_program,
        };
        self.next_program = self.next_program.wrapping_add(1);
        self.entries.push(program);
        program
    }

    /// The number of cached programs.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = &SmokeProgram> + '_ {
        self.entries.iter()
    }

    /// Frees every program (the unit's cleanup hook).
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_program = 0;
    }
}

/// A smoke/mist mesh: its material, the node scale the texture offset adds
/// and the state key its program is cached under.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct SmokeMist {
    pub material: Handle<SmokeMaterial>,
    /// The node's scale (`+0x1ac`), added to the texture's vertical offset.
    pub scale: f32,
    pub state_key: u64,
}

/// The smoke shader's parameters.
#[derive(Clone, Copy, Debug, PartialEq, ShaderType)]
pub struct SmokeParams {
    pub uv_offset: Vec2,
    /// The tint, alpha its opacity.
    pub tint: Vec4,
    /// The cached program's state key (Exarust runs one program, so this
    /// only distinguishes states).
    pub state_key: u32,
    pub pad: u32,
}

impl Default for SmokeParams {
    fn default() -> Self {
        Self {
            uv_offset: Vec2::ZERO,
            tint: Vec4::ONE,
            state_key: 0,
            pad: 0,
        }
    }
}

/// A smoke or mist surface: an unlit, two-sided, alpha-blended draw with no
/// depth writes, as the original's blended class has.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct SmokeMaterial {
    #[uniform(0)]
    pub params: SmokeParams,
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

fn smoke_shader() -> ShaderRef {
    ShaderRef::Path(
        AssetPath::from_path_buf(embedded_path!("shadesmk.wesl")).with_source("embedded"),
    )
}

impl Material for SmokeMaterial {
    fn fragment_shader() -> ShaderRef {
        smoke_shader()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Blend
    }

    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        // Smoke draws both faces.
        descriptor.primitive.cull_mode = None;
        Ok(())
    }
}

/// Advances each smoke mesh's texture offset on the 80-second cycle.
fn scroll_smoke(
    time: Res<Time>,
    mist: Query<&SmokeMist>,
    mut materials: ResMut<Assets<SmokeMaterial>>,
) {
    let now = time.elapsed().as_millis() as u64;
    for smoke in &mist {
        if let Some(mut material) = materials.get_mut(&smoke.material) {
            material.params.uv_offset = smoke_uv_offset(smoke.scale, now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_texture_offset_scrolls_once_each_80_seconds() {
        assert_eq!(smoke_uv_offset(1.0, 0), Vec2::new(0.5, 1.5));
        assert_eq!(smoke_uv_offset(1.0, 40_000), Vec2::new(0.5, 2.0));
        assert_eq!(smoke_uv_offset(1.0, 80_000), Vec2::new(0.5, 1.5));
        assert_eq!(smoke_uv_offset(0.0, 20_000), Vec2::new(0.5, 0.75));
        assert_eq!(smoke_uv_offset(1.0, 120_000), smoke_uv_offset(1.0, 40_000));
    }

    #[test]
    fn the_state_key_keeps_the_selected_bits_only() {
        assert_eq!(smoke_state_key(0), 0);
        assert_eq!(smoke_state_key(STATE_KEY_MASK), STATE_KEY_MASK);
        assert_eq!(smoke_state_key(u64::MAX), STATE_KEY_MASK);
        assert_eq!(smoke_state_key(0x8000), 0);
        assert_eq!(smoke_state_key(0x10), 0x0);
        assert_eq!(smoke_state_key(0x8), 0x8);
    }

    #[test]
    fn the_program_cache_compiles_once_per_key_and_frees() {
        let mut cache = SmokeProgramCache::default();
        assert!(cache.is_empty());
        let first = cache.get_or_insert(7);
        assert_eq!(first.program, 0);
        assert_eq!(cache.get_or_insert(7), first);
        let second = cache.get_or_insert(9);
        assert_eq!(second.program, 1);
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.get_or_insert(7).program, 0);
    }

    #[test]
    fn the_draw_repeats_for_each_supersample_quadrant() {
        assert_eq!(smoke_quadrants(0), 1);
        assert_eq!(smoke_quadrants(3), 4);
        assert_eq!(smoke_quadrants(7), 8);
    }
}

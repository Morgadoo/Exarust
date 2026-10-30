//! The GPU benchmark's workload: sixteen full-screen quads drawn with the
//! original's own fragment program into a 1024² target, sampling a 512²
//! texture whose bytes are all 0x3f.
//!
//! The original binds a 1024² RGB8 colour target, uploads a 512² RGB8
//! texture filled with one value (its 0x3f bytes; wgpu has no three-channel
//! format, so both are RGBA8 here and the program reads only `.rgb`), sets the texture's wrap to
//! repeat and its filters to linear, compiles a fragment shader that reads
//! the texture at four offsets with square roots, dot products, lengths,
//! powers, sines and cosines, and draws sixteen full-screen quads a frame
//! through it (the vertex stage is the GUI's built-in screen quad). This
//! module is that program: [`BenchmarkMaterial`] is the textured material it
//! binds and [`SOURCE_BYTES`] the fill. Rules: `docs/UI.md` (GPU benchmark).

use bevy::asset::{AssetPath, embedded_asset, embedded_path};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;
use bevy::sprite_render::{Material2d, Material2dPlugin};

/// The benchmark source texture's side, texels (512).
pub const SOURCE_SIZE: u32 = 512;
/// The byte every channel of the source texture holds (the original fills
/// the upload buffer with `FillChar(…, 0x3f)`).
pub const SOURCE_BYTE: u8 = 0x3f;
/// The benchmark target's side, texels (1024).
pub const TARGET_SIZE: u32 = 1024;
/// How many full-screen quads the original draws a measured frame (it draws
/// [`WARMUP_QUADS`] a warm-up frame).
pub const QUADS: usize = exa_formats::hw_config::RUN_QUADS as usize;
/// How many full-screen quads the original draws a warm-up frame.
pub const WARMUP_QUADS: usize = exa_formats::hw_config::WARMUP_QUADS as usize;

/// The fragment program's source, as compiled, for tests and the docs.
pub const FRAGMENT_SOURCE: &str = include_str!("benchmark.wesl");

/// The plugin: the material and its embedded shader.
pub struct BenchmarkPlugin;

impl Plugin for BenchmarkPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "benchmark.wesl");
        app.add_plugins(Material2dPlugin::<BenchmarkMaterial>::default());
    }
}

/// The benchmark's material: the source texture and the original's
/// fragment program.
#[derive(Asset, TypePath, AsBindGroup, Clone, Debug)]
pub struct BenchmarkMaterial {
    /// The original's projection uniform, 2.0 in its first float; its
    /// program declares the block and never reads it.
    #[uniform(0)]
    pub projection: Vec4,
    /// The 512² source texture, sampled with repeat addressing.
    #[texture(1)]
    #[sampler(2)]
    pub texture: Handle<Image>,
}

impl BenchmarkMaterial {
    /// The material over a source texture, with the original's 2.0
    /// projection.
    pub fn new(texture: Handle<Image>) -> Self {
        Self {
            projection: Vec4::new(2.0, 0.0, 0.0, 0.0),
            texture,
        }
    }
}

impl Material2d for BenchmarkMaterial {
    fn fragment_shader() -> ShaderRef {
        ShaderRef::Path(
            AssetPath::from_path_buf(embedded_path!("benchmark.wesl")).with_source("embedded"),
        )
    }
}

/// The source texture's pixels: [`SOURCE_SIZE`]² RGBA8 texels whose every
/// byte is [`SOURCE_BYTE`], as the original's one-byte fill of its RGB8
/// upload gives (the padded alpha is never read).
pub fn source_pixels() -> Vec<u8> {
    vec![SOURCE_BYTE; (SOURCE_SIZE * SOURCE_SIZE * 4) as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_source_texture_is_one_value() {
        let pixels = source_pixels();
        assert_eq!(pixels.len(), 512 * 512 * 4);
        assert!(pixels.iter().all(|byte| *byte == 0x3f));
        assert_eq!(SOURCE_BYTE, 0x3f);
        assert_eq!(SOURCE_SIZE, 512);
        assert_eq!(TARGET_SIZE, 1024);
    }

    /// The two texture reads the program makes before its first rebind are at
    /// the plain uv and 0.2341 to the right, as the original's source states.
    #[test]
    fn the_fragment_program_is_the_originals() {
        for literal in [
            "var t = in.uv;",
            "textureSample(texture, texture_sampler, in.uv)",
            "t.x = t.x + 0.2341;",
            "t.y = t.y - 0.1476;",
            "t.x = t.x - 0.3768;",
            "t.y = t.y + 0.4276;",
            "1.2134",
            "2.2134",
            "3.734",
        ] {
            assert!(
                FRAGMENT_SOURCE.contains(literal),
                "the program lost {literal}"
            );
        }
    }

    /// The program is valid WGSL (and so valid WESL, which this file has no
    /// imports for): the shader compiler's own front end accepts it.
    #[test]
    fn the_fragment_program_is_valid_wgsl() {
        let module = naga::front::wgsl::parse_str(FRAGMENT_SOURCE).expect("the program parses");
        let mut validator = naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        );
        validator.validate(&module).expect("the program validates");
        // The material's bindings: the projection uniform, the texture and
        // its sampler, at group 2 bindings 0, 1 and 2.
        let bindings: Vec<(u32, u32)> = module
            .global_variables
            .iter()
            .filter_map(|(_, variable)| variable.binding.as_ref())
            .map(|binding| (binding.group, binding.binding))
            .collect();
        assert_eq!(
            bindings,
            vec![(2, 0), (2, 1), (2, 2)],
            "the material's bindings moved"
        );
    }
}

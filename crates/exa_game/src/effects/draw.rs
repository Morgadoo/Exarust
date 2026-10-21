//! The draw descriptions of the force effects.
//!
//! The original issues OpenGL calls from a handful of effect programs. The
//! calls themselves are platform glue, but the state they set is data: the
//! program pair, the uniforms, the bound textures and the pass flags. This
//! module reproduces that data so the renderer can consume it.
//!
//! - The **force geometry** programs are `effectforce_v` with
//!   `effectforce_f` for blast, bolt and wave and with `effecttorrent_f`
//!   for torrent. They take `ObjMat` (the effect's frame times the view's
//!   matrix), `TexOfs` (the texture offset) and `EmiTex` on unit 0. The
//!   blast's offset is (0, 0, 1); the torrent's comes from its two scroll
//!   words.
//! - The **GI lights** carry their positions, colour blocks and radius.
//!
//! Rules and evidence: `Exanima.exe`'s effect geometry and emissive code and
//! the shader sources; [`docs/THAUMATURGY.md`](../../docs/THAUMATURGY.md).

use crate::effects::blast::BlastLightSet;
use crate::effects::torrent::TorrentLightSet;

/// The force geometry's vertex program.
pub const FORCE_VERTEX: &str = "effectforce_v";
/// The force geometry's fragment program for the blast family.
pub const FORCE_FRAGMENT: &str = "effectforce_f";
/// The force geometry's fragment program for the torrent.
pub const TORRENT_FRAGMENT: &str = "effecttorrent_f";
/// The GI light program.
pub const GI_LIGHT: &str = "gi_light";

/// How a draw blends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blend {
    /// The original's additive `SRC_ALPHA`, `ONE`.
    Add,
    /// The original's premultiplied `ONE`, `ONE_MINUS_SRC_ALPHA`.
    Alpha,
    /// No blending.
    Opaque,
}

/// One uniform value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UniformValue {
    Float(f32),
    Vec2([f32; 2]),
    Vec3([f32; 3]),
    Vec4([f32; 4]),
    Mat4([[f32; 4]; 4]),
}

/// A texture bound to a unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureBinding {
    pub name: &'static str,
    pub unit: u32,
}

/// One draw call the renderer can issue.
#[derive(Debug, Clone, PartialEq)]
pub struct DrawCall {
    pub vertex: &'static str,
    pub fragment: &'static str,
    pub uniforms: Vec<(&'static str, UniformValue)>,
    pub textures: Vec<TextureBinding>,
    pub blend: Blend,
    pub depth_write: bool,
    pub depth_test: bool,
    pub cull: bool,
    /// The view pass the call belongs to (0 when there is one).
    pub view: usize,
}

/// Multiplies two row-major 4 × 4 matrices, the original's row-by-column
/// sums in their instruction order.
pub fn mat4_multiply(left: [[f32; 4]; 4], right: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for row in 0..4 {
        for column in 0..4 {
            out[row][column] = left[row][0] * right[0][column]
                + left[row][1] * right[1][column]
                + left[row][2] * right[2][column]
                + left[row][3] * right[3][column];
        }
    }
    out
}

/// The force geometry draw descriptions: one per view pass, each with the
/// effect's frame times the view's matrix and the texture offset. The
/// geometry holds every chunk (blast) or stream (torrent) in one mesh.
pub fn force_geometry_draws(
    vertex: &'static str,
    fragment: &'static str,
    views: &[[[f32; 4]; 4]],
    frame: [[f32; 4]; 4],
    tex_ofs: [f32; 3],
    blend: Blend,
) -> Vec<DrawCall> {
    views
        .iter()
        .enumerate()
        .map(|(view, matrix)| DrawCall {
            vertex,
            fragment,
            uniforms: vec![
                ("ObjMat", UniformValue::Mat4(mat4_multiply(frame, *matrix))),
                ("TexOfs", UniformValue::Vec3(tex_ofs)),
            ],
            textures: vec![TextureBinding {
                name: "EmiTex",
                unit: 0,
            }],
            blend,
            depth_write: false,
            depth_test: true,
            cull: false,
            view,
        })
        .collect()
}

/// The blast family's draws.
pub fn blast_geometry_draws(
    views: &[[[f32; 4]; 4]],
    frame: [[f32; 4]; 4],
    tex_ofs: [f32; 3],
) -> Vec<DrawCall> {
    force_geometry_draws(
        FORCE_VERTEX,
        FORCE_FRAGMENT,
        views,
        frame,
        tex_ofs,
        Blend::Add,
    )
}

/// The torrent's draws.
pub fn torrent_geometry_draws(
    views: &[[[f32; 4]; 4]],
    frame: [[f32; 4]; 4],
    tex_ofs: [f32; 3],
) -> Vec<DrawCall> {
    force_geometry_draws(
        FORCE_VERTEX,
        TORRENT_FRAGMENT,
        views,
        frame,
        tex_ofs,
        Blend::Add,
    )
}

/// The blast's GI light as a draw: its position, radius and colour blocks
/// (one per chunk), with no texture.
pub fn blast_light_draw(set: &BlastLightSet) -> DrawCall {
    let mut colours = Vec::with_capacity(set.blocks.len() * 3);
    for block in &set.blocks {
        colours.extend_from_slice(block);
    }
    DrawCall {
        vertex: GI_LIGHT,
        fragment: GI_LIGHT,
        uniforms: vec![
            (
                "LightPos",
                UniformValue::Vec3([set.position.x, set.position.y, set.position.z]),
            ),
            ("LightRadius", UniformValue::Float(set.radius)),
            ("LightColours", UniformValue::Vec3(mean3(&colours))),
        ],
        textures: Vec::new(),
        blend: Blend::Add,
        depth_write: false,
        depth_test: false,
        cull: false,
        view: 0,
    }
}

/// The torrent's GI light as a draw.
pub fn torrent_light_draw(set: &TorrentLightSet) -> DrawCall {
    let mut colours = Vec::with_capacity(set.blocks.len() * 3);
    for block in &set.blocks {
        colours.extend_from_slice(block);
    }
    DrawCall {
        vertex: GI_LIGHT,
        fragment: GI_LIGHT,
        uniforms: vec![
            (
                "LightPos",
                UniformValue::Vec3([set.position.x, set.position.y, set.position.z]),
            ),
            ("LightStrength", UniformValue::Float(set.strength)),
            ("LightColours", UniformValue::Vec3(mean3(&colours))),
        ],
        textures: Vec::new(),
        blend: Blend::Add,
        depth_write: false,
        depth_test: false,
        cull: false,
        view: 0,
    }
}

/// The mean of a flat rgb array; an empty array is black.
fn mean3(values: &[f32]) -> [f32; 3] {
    if values.is_empty() {
        return [0.0; 3];
    }
    let mut sum = [0.0f32; 3];
    let count = values.len() / 3;
    for block in values.as_chunks::<3>().0 {
        sum[0] += block[0];
        sum[1] += block[1];
        sum[2] += block[2];
    }
    [
        sum[0] / count as f32,
        sum[1] / count as f32,
        sum[2] / count as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3;

    fn identity() -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    #[test]
    fn a_matrix_product_is_row_by_column() {
        let mut scale = identity();
        scale[0][0] = 2.0;
        scale[3][0] = 3.0;
        let moved = mat4_multiply(scale, identity());
        assert_eq!(moved[0][0], 2.0);
        assert_eq!(moved[3][0], 3.0);
        let mut other = identity();
        other[1][3] = 5.0;
        let product = mat4_multiply(scale, other);
        assert_eq!(product[0][3], 0.0);
        assert_eq!(product[1][3], 5.0);
    }

    #[test]
    fn force_draws_are_one_a_view() {
        let views = [identity(), identity()];
        let draws = blast_geometry_draws(&views, identity(), [0.0, 0.0, 1.0]);
        assert_eq!(draws.len(), 2);
        assert_eq!(draws[0].vertex, FORCE_VERTEX);
        assert_eq!(draws[0].fragment, FORCE_FRAGMENT);
        assert_eq!(draws[0].view, 0);
        assert_eq!(draws[1].view, 1);
        assert!(draws[0].uniforms.iter().any(|(name, value)| {
            *name == "TexOfs" && *value == UniformValue::Vec3([0.0, 0.0, 1.0])
        }));
        let torrent = torrent_geometry_draws(&views, identity(), [0.0, 0.2, 0.4]);
        assert_eq!(torrent[0].fragment, TORRENT_FRAGMENT);
    }

    #[test]
    fn the_spirit_screen_and_lights_are_described() {
        let blast = BlastLightSet {
            position: Vec3::ZERO,
            radius: 5000.0,
            blocks: vec![[0.3, 0.0, 0.0], [0.0, 0.0, 0.0]],
        };
        let draw = blast_light_draw(&blast);
        assert!(draw.textures.is_empty());
        assert!(
            draw.uniforms.iter().any(
                |(name, value)| *name == "LightRadius" && *value == UniformValue::Float(5000.0)
            )
        );
        let torrent = TorrentLightSet {
            position: Vec3::ZERO,
            strength: 2000.0,
            blocks: vec![[0.1, 0.0, 0.0]],
        };
        let draw = torrent_light_draw(&torrent);
        assert!(
            draw.uniforms
                .iter()
                .any(|(name, value)| *name == "LightStrength"
                    && *value == UniformValue::Float(2000.0))
        );
    }
}

//! The force effects' render nodes: which shader each class builds, which
//! uniforms it binds, the flags its geometry carries, and the per-instance
//! texture row and fade the shared effect renderer computes.
//!
//! Every force geometry is a geometry node with one sub-object (kind
//! 35044), built with flags `0x0400_1000` and extra `0x400`; the shared
//! effect renderer reports four texture layers and, for each supersample
//! pass, offsets the texture row by the node's level of detail
//! ([`effect_row`]) before it draws the pass.
//!
//! The classes and their programs:
//!
//! | class | vertex | fragment |
//! |---|---|---|
//! | force (base) | — | — |
//! | barrier | `effectbarrier_v` | `effectbarrier_f` |
//! | bubble | `effectbarrier_v` | `effectbubble_f` |
//! | barricade | `effectfbrcd_v` | `effectfbrcd_f` |
//! | strikes | `effectshell_v` | `effectfrcstrk_f` |
//! | tendril | `effectforce_v` | `effectsmoke_f` |
//! | blast (and bolt and wave) | `effectforce_v` | `effectforce_f` |
//! | torrent | `effectforce_v` | `effecttorrent_f` |
//! | effect (generic) | — | — |
//!
//! Each supersample pass uploads its own object matrix to `ObjMat`: pass 0
//! with the renderer's current viewport, then every later pass after its
//! whole-pixel viewport shift ([`force_uploads`]).
//!
//! The barrier, bubble and barricade bind `ObjMat`, `ViewPos`, `Fade`,
//! `TexPos`, `ColPt` and `EmiTex`; the bubble and barricade add `TexOfs`
//! (the barrier has none); the barricade adds its `BExt`; the strikes add
//! `LclMat`, `FdPln`, `ColourInt`, `TexScale` and `Gradient`; the tendril
//! binds only `ObjMat`, `TexOfs` and `EmiTex`, its offset `(0, a, b)` from
//! its effect; the blast and the torrent bind the same three, their offsets
//! `(0, r, s)` (the first chunk's radius and the setup's texture scale) and
//! `(0, scroll, 1)` from their effects. The tendril, blast and torrent
//! draw their passes on the 0.002 row step ([`force_row`]).
//!
//! Evidence: the classes' geometry constructors and program setups and the
//! shared effect renderer's instance loop, read from `Exanima.exe`.

/// The flags a force geometry node is built with.
pub const NODE_FLAGS: u32 = 0x0400_1000;
/// The extra word a force geometry's sub-object carries.
pub const SUB_OBJECT_EXTRA: u32 = 0x400;
/// The texture layers the shared effect renderer reports.
pub const LAYERS: i32 = 4;
/// The sub-object's kind word.
pub const SUB_OBJECT_KIND: i32 = 35_044;
/// The pass count the row step divides by (1 in the original's data; a
/// renderer setting may change it at run time).
pub const ROW_PASSES: f32 = 1.0;
/// The tendril's, blast's and torrent's row step, 0.002.
pub const FORCE_ROW_STEP: f32 = 0.002;

/// A force geometry class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// The abstract base all force geometries derive from.
    Force,
    /// The barrier's half dome with its impact ripples.
    Barrier,
    /// The bubble's full sphere.
    Bubble,
    /// The barricade's wall.
    Barricade,
    /// The force strikes' ribbon.
    Strikes,
    /// The displacement tendril.
    Tendril,
    /// The force blast's chunks (the bolt and the wave share it).
    Blast,
    /// The force torrent's streams.
    Torrent,
    /// A plain effect geometry.
    Effect,
}

/// A geometry class's shader program and uniforms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Geometry {
    pub vertex: Option<&'static str>,
    pub fragment: Option<&'static str>,
    pub uniforms: &'static [&'static str],
}

impl Class {
    /// The class's program, as its constructor builds it.
    pub fn geometry(self) -> Geometry {
        match self {
            Class::Force | Class::Effect => Geometry {
                vertex: None,
                fragment: None,
                uniforms: &[],
            },
            Class::Barrier => Geometry {
                vertex: Some("effectbarrier_v"),
                fragment: Some("effectbarrier_f"),
                uniforms: &["ObjMat", "ViewPos", "Fade", "TexPos", "ColPt", "EmiTex"],
            },
            Class::Bubble => Geometry {
                vertex: Some("effectbarrier_v"),
                fragment: Some("effectbubble_f"),
                uniforms: &[
                    "ObjMat", "TexOfs", "ViewPos", "Fade", "TexPos", "ColPt", "EmiTex",
                ],
            },
            Class::Barricade => Geometry {
                vertex: Some("effectfbrcd_v"),
                fragment: Some("effectfbrcd_f"),
                uniforms: &[
                    "ObjMat", "TexOfs", "ViewPos", "BExt", "Fade", "TexPos", "ColPt", "EmiTex",
                ],
            },
            Class::Strikes => Geometry {
                vertex: Some("effectshell_v"),
                fragment: Some("effectfrcstrk_f"),
                uniforms: &[
                    "ObjMat",
                    "LclMat",
                    "TexOfs",
                    "ViewPos",
                    "Fade",
                    "TexPos",
                    "ColPt",
                    "FdPln",
                    "ColourInt",
                    "TexScale",
                    "Gradient",
                    "EmiTex",
                ],
            },
            Class::Tendril => Geometry {
                vertex: Some("effectforce_v"),
                fragment: Some("effectsmoke_f"),
                uniforms: &["ObjMat", "TexOfs", "EmiTex"],
            },
            Class::Blast => Geometry {
                vertex: Some("effectforce_v"),
                fragment: Some("effectforce_f"),
                uniforms: &["ObjMat", "TexOfs", "EmiTex"],
            },
            Class::Torrent => Geometry {
                vertex: Some("effectforce_v"),
                fragment: Some("effecttorrent_f"),
                uniforms: &["ObjMat", "TexOfs", "EmiTex"],
            },
        }
    }
}

/// A supersample pass's texture row: `((pass + frame + 0.5)·step)/passes +
/// start − step/2`, the pass and frame counters added as integers.
pub fn instance_row(pass: i32, frame: i32, start: f32, step: f32) -> f32 {
    (((i64::from(pass) + i64::from(frame)) as f32 + 0.5) * step) / ROW_PASSES + start - step * 0.5
}

/// The shared renderer's pass row: the node's level of detail is the step.
pub fn effect_row(pass: i32, frame: i32, start: f32, lod: f32) -> f32 {
    instance_row(pass, frame, start, lod)
}

/// The tendril's, blast's and torrent's pass row, step 0.002 (its half,
/// 0.001, as the original's literal). The counter and its 0.5 are single
/// precision; the step, the divisor, the start and the half step are
/// applied in double and the sum narrowed, as the material passes do.
pub fn force_row(pass: i32, frame: i32, start: f32) -> f32 {
    let counter = (i64::from(pass) + i64::from(frame)) as f32 + 0.5;
    (f64::from(counter) * FORCE_ROW_STEP_LITERAL / f64::from(ROW_PASSES) + f64::from(start) - 0.001)
        as f32
}

/// The 0.002 step as the double literal the material passes multiply by.
const FORCE_ROW_STEP_LITERAL: f64 = 0.002;

/// The object matrix a supersample pass draws with: the node's own matrix
/// times the pass's texture matrix, each row `((a₀·b₀ + a₁·b₁) + (a₂·b₂ +
/// a₃·b₃))` — the two halves summed before each other, as the original's
/// matrix product does.
pub fn pass_matrix(node: [[f32; 4]; 4], pass: [[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut out = [[0.0f32; 4]; 4];
    for (row, out_row) in node.iter().zip(out.iter_mut()) {
        for (lane, value) in out_row.iter_mut().enumerate() {
            *value = (row[0] * pass[0][lane] + row[1] * pass[1][lane])
                + (row[2] * pass[2][lane] + row[3] * pass[3][lane]);
        }
    }
    out
}

/// One supersample pass of a force geometry: the texture row the shared
/// pass writes into the pass's matrix and the object matrix the geometry
/// uploads before drawing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassDraw {
    pub row: f32,
    pub matrix: [[f32; 4]; 4],
}

/// The passes a force geometry draws in a frame: for each pass the texture
/// row ([`force_row`]) and the object matrix [`pass_matrix`] makes of the
/// node's matrix and the pass's texture matrix.
pub fn force_passes(
    node: [[f32; 4]; 4],
    pass_matrices: &[[[f32; 4]; 4]],
    frame: i32,
    start: f32,
) -> Vec<PassDraw> {
    pass_matrices
        .iter()
        .enumerate()
        .map(|(pass, texture)| PassDraw {
            row: force_row(pass as i32, frame, start),
            matrix: pass_matrix(node, *texture),
        })
        .collect()
}

/// One object-matrix upload of a force geometry's supersample pass: the
/// matrix written to `ObjMat`, the pass it belongs to and the viewport the
/// draw sets for it. The first pass is uploaded before any viewport is set
/// (the renderer's current view), every later pass after its own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjMatUpload {
    pub pass: i32,
    /// `ObjMat`: the node's matrix times the pass's texture matrix.
    pub matrix: [[f32; 4]; 4],
    /// The whole-pixel viewport the pass draws in (zero for the first).
    pub viewport: [i32; 2],
    /// The first pass, drawn before the viewport walk.
    pub before_viewports: bool,
}

/// The object matrices a force geometry uploads in a frame, in the
/// renderer's order: pass 0 with the current viewport, then each later pass
/// after its own viewport shift.
pub fn force_uploads(node: [[f32; 4]; 4], pass_matrices: &[[[f32; 4]; 4]]) -> Vec<ObjMatUpload> {
    pass_matrices
        .iter()
        .enumerate()
        .map(|(pass, texture)| ObjMatUpload {
            pass: pass as i32,
            matrix: pass_matrix(node, *texture),
            viewport: if pass == 0 {
                [0, 0]
            } else {
                exa_render::subframe::viewport_shift(pass as u32)
            },
            before_viewports: pass == 0,
        })
        .collect()
}

/// The `Fade` a dome geometry draws at: `max(1 − alpha, 0)` while the
/// effect is under state 3, the alpha itself from 3 on.
pub fn dome_fade(state: u32, alpha: f32) -> f32 {
    if state < 3 {
        let left = 1.0 - alpha;
        if left <= 0.0 { 0.0 } else { left }
    } else {
        alpha
    }
}

/// The barricade's draw alpha: growing, `(1 − share)²`; standing, 0;
/// shrinking, `share²`; all squared again by the draw.
pub fn barricade_alpha(state: i32, share: f32) -> f32 {
    let f = if state == 1 {
        let e = 1.0 - share;
        let e = if e <= 0.0 { 0.0 } else { e };
        e * e
    } else if (state as u32) < 3 {
        0.0
    } else {
        share * share
    };
    f * f
}

/// The strikes' `Fade`: `1 − fade`.
pub fn strikes_alpha(fade: f32) -> f32 {
    1.0 - fade
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_node_flags_are_the_originals() {
        assert_eq!(NODE_FLAGS, 0x0400_1000);
        assert_eq!(SUB_OBJECT_EXTRA, 0x400);
        assert_eq!(LAYERS, 4);
    }

    #[test]
    fn each_class_builds_its_program() {
        assert_eq!(Class::Barrier.geometry().fragment, Some("effectbarrier_f"));
        assert_eq!(Class::Bubble.geometry().fragment, Some("effectbubble_f"));
        assert_eq!(Class::Barricade.geometry().fragment, Some("effectfbrcd_f"));
        assert_eq!(Class::Strikes.geometry().vertex, Some("effectshell_v"));
        assert_eq!(
            Class::Tendril.geometry().uniforms,
            &["ObjMat", "TexOfs", "EmiTex"]
        );
        assert_eq!(Class::Force.geometry().vertex, None);
        assert_eq!(Class::Blast.geometry().fragment, Some("effectforce_f"));
        assert_eq!(Class::Torrent.geometry().fragment, Some("effecttorrent_f"));
        assert_eq!(
            Class::Torrent.geometry().vertex,
            Class::Tendril.geometry().vertex
        );
        // The bubble and barrier share the vertex program.
        assert_eq!(
            Class::Barrier.geometry().vertex,
            Class::Bubble.geometry().vertex
        );
    }

    #[test]
    fn the_force_row_steps_in_double_precision() {
        // The counter is formed in single precision, then the 0.002 step,
        // the pass divisor, the start and the 0.001 half step are applied
        // in double and the sum narrowed: 0.5·0.002 + 0.25 − 0.001 is
        // exactly 0.25, where the same all in single gives 0x3e7fffff.
        assert_eq!(force_row(0, 0, 0.25).to_bits(), 0x3e80_0000);
        let single = ((0.5f32 * 0.002) / 1.0 + 0.25) - 0.001;
        assert_eq!(single.to_bits(), 0x3e7f_ffff);
    }

    #[test]
    fn a_pass_product_pairs_its_halves() {
        // (1e8 + 1) + (−1e8 + 1) is 0, where left-to-right would give 1.
        let node = [[1.0e8, 1.0, -1.0e8, 1.0]; 4];
        let pass = [[1.0; 4]; 4];
        let out = pass_matrix(node, pass);
        assert_eq!(out[0][0], 0.0);
        assert_ne!(out[0][0], 1.0);
    }

    #[test]
    fn each_pass_uploads_its_object_matrix() {
        let node = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let passes = [
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 2.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        ];
        let uploads = force_uploads(node, &passes);
        assert_eq!(uploads.len(), 2);
        // Pass 0 draws with the current view and the node's own matrix.
        assert_eq!(uploads[0].pass, 0);
        assert!(uploads[0].before_viewports);
        assert_eq!(uploads[0].viewport, [0, 0]);
        assert_eq!(uploads[0].matrix, node);
        // Pass 1 takes the sub-frame's viewport and the product.
        assert_eq!(uploads[1].pass, 1);
        assert!(!uploads[1].before_viewports);
        assert_eq!(uploads[1].viewport, exa_render::subframe::viewport_shift(1));
        assert_eq!(uploads[1].matrix, pass_matrix(node, passes[1]));
        assert_eq!(uploads[1].matrix[0][0], 2.0);
        assert_eq!(uploads[1].matrix[1][1], 2.0);
    }

    #[test]
    fn the_pass_rows_step_by_the_level_of_detail() {
        // Step 1, base 0: rows 0.0, 1.0, 2.0.
        assert_eq!(effect_row(0, 0, 0.0, 1.0), 0.0);
        assert_eq!(effect_row(1, 0, 0.0, 1.0), 1.0);
        // Step 0.5, frame 10, start 0.25: ((11 + 0.5)·0.5) + 0.25 − 0.25.
        assert_eq!(effect_row(1, 10, 0.25, 0.5), 5.75);
        // The force geometries' 0.002 step: rows 0 and 0.002.
        assert!(force_row(0, 0, 0.0).abs() < 1e-7);
        assert!((force_row(1, 0, 0.0) - 0.002).abs() < 1e-7);
    }

    #[test]
    fn the_dome_fades_flip_at_state_three() {
        assert_eq!(dome_fade(1, 0.25), 0.75);
        assert_eq!(dome_fade(2, 0.25), 0.75);
        assert_eq!(dome_fade(3, 0.25), 0.25);
        assert_eq!(dome_fade(4, 0.25), 0.25);
        assert_eq!(dome_fade(1, 1.5), 0.0);
    }

    #[test]
    fn the_barricade_alpha_is_a_fourth_power() {
        assert!((barricade_alpha(1, 0.5) - 0.0625).abs() < 1e-6);
        assert_eq!(barricade_alpha(0, 1.0), 0.0);
        assert!((barricade_alpha(3, 0.5) - 0.0625).abs() < 1e-6);
        assert!((barricade_alpha(4, 1.0) - 1.0).abs() < 1e-6);
        assert_eq!(strikes_alpha(0.25), 0.75);
    }

    #[test]
    fn a_pass_multiplies_the_node_matrix_on_the_right() {
        let node = [
            [1.0, 0.0, 0.0, 10.0],
            [0.0, 1.0, 0.0, 20.0],
            [0.0, 0.0, 1.0, 30.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let pass = [
            [2.0, 0.0, 0.0, 0.0],
            [0.0, 3.0, 0.0, 0.0],
            [0.0, 0.0, 4.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let out = pass_matrix(node, pass);
        assert_eq!(out[0], [2.0, 0.0, 0.0, 10.0]);
        assert_eq!(out[1], [0.0, 3.0, 0.0, 20.0]);
        assert_eq!(out[2], [0.0, 0.0, 4.0, 30.0]);
        // The halves sum before each other: (a₀b₀ + a₁b₁) + (a₂b₂ + a₃b₃).
        let scaled = [
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
            [0.5, 0.25, 0.125, 1.0],
        ];
        let texture = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let out = pass_matrix(scaled, texture);
        assert_eq!(out[3], [0.5, 0.25, 0.125, 1.0]);
    }

    #[test]
    fn a_force_geometry_draws_one_matrix_a_pass() {
        let node = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let pass = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let draws = force_passes(node, &[pass, pass], 0, 0.0);
        assert_eq!(draws.len(), 2);
        assert_eq!(draws[0].row, 0.0);
        assert!((draws[1].row - 0.002).abs() < 1e-7);
        assert_eq!(draws[0].matrix, node);
    }
}

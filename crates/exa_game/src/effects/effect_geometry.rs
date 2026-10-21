//! The force effects' render nodes: which shader each class builds, which
//! uniforms it binds, the flags its geometry carries, and the per-instance
//! texture row and fade the shared effect renderer computes.
//!
//! Every force geometry is a `TGeometry` with one sub-object, built with
//! flags `0x0400_1000` and extra `0x400`; the shared effect renderer gives a
//! node four texture layers and fills each instance's matrix and light row
//! from the node's light radius.
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
//! | effect (generic) | — | — |
//!
//! The barrier, bubble and barricade binds `ObjMat`, `ViewPos`, `TexPos`,
//! `ColPt` and `EmiTex`; the bubble and barricade add `TexOfs`; the
//! barricade adds its `BExt`; the strikes add `LclMat`, `FdPln`,
//! `ColourInt`, `TexScale` and `Gradient`; the tendril binds only `ObjMat`,
//! `TexOfs` and `EmiTex`.
//!
//! Evidence: the classes' geometry constructors and program setups and the
//! shared effect renderer's instance loop, read from `Exanima.exe`.

/// The flags a force geometry node is built with.
pub const NODE_FLAGS: u32 = 0x0400_1000;
/// The extra word a force geometry's sub-object carries.
pub const SUB_OBJECT_EXTRA: u32 = 0x400;
/// The textures the shared effect renderer gives each node.
pub const LAYERS: i32 = 4;
/// The texture height the row blend divides by (one, as the renderer
/// stores it).
pub const TEXTURE_ROWS: f32 = 1.0;
/// The tendril's row step, 0.002.
pub const TENDRIL_ROW_STEP: f32 = 0.002;

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
                uniforms: &["ObjMat", "ViewPos", "TexPos", "ColPt", "EmiTex"],
            },
            Class::Bubble => Geometry {
                vertex: Some("effectbarrier_v"),
                fragment: Some("effectbubble_f"),
                uniforms: &["ObjMat", "TexOfs", "ViewPos", "TexPos", "ColPt", "EmiTex"],
            },
            Class::Barricade => Geometry {
                vertex: Some("effectfbrcd_v"),
                fragment: Some("effectfbrcd_f"),
                uniforms: &[
                    "ObjMat", "TexOfs", "ViewPos", "BExt", "TexPos", "ColPt", "EmiTex",
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
        }
    }
}

/// The per-instance texture row: `((index + base + 0.5)·step)/rows + start −
/// step/2`.
pub fn instance_row(index: i32, base: f32, start: f32, step: f32) -> f32 {
    ((index as f32 + base + 0.5) * step) / TEXTURE_ROWS + start - step * 0.5
}

/// The shared renderer's instance row for an effect: the node's light
/// radius is the step and the stored row its start.
pub fn effect_row(index: i32, base: f32, start: f32, light_radius: f32) -> f32 {
    instance_row(index, base, start, light_radius)
}

/// The tendril's instance row, step 0.002.
pub fn tendril_row(index: i32, base: f32, start: f32) -> f32 {
    instance_row(index, base, start, TENDRIL_ROW_STEP)
}

/// The fade a dome geometry draws at: `1 − alpha` while the effect is under
/// state 3, the alpha itself from 3 on.
pub fn dome_fade(state: u32, alpha: f32) -> f32 {
    if state < 3 { 1.0 - alpha } else { alpha }
}

/// The barricade's draw alpha: growing, `(1 − share)²`; standing, 0;
/// shrinking, `share²`; all squared again by the draw.
pub fn barricade_alpha(state: i32, share: f32) -> f32 {
    let f = if state == 1 {
        let e = (1.0 - share).clamp(0.0, 1.0);
        e * e
    } else if state < 3 {
        0.0
    } else {
        share * share
    };
    f * f
}

/// The strikes' draw alpha: `1 − fade`.
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
        // The bubble and barrier share the vertex program.
        assert_eq!(
            Class::Barrier.geometry().vertex,
            Class::Bubble.geometry().vertex
        );
    }

    #[test]
    fn the_instance_rows_step_by_the_light_radius() {
        // Step 1, base 0: rows 0.0, 1.0, 2.0.
        assert_eq!(effect_row(0, 0.0, 0.0, 1.0), 0.0);
        assert_eq!(effect_row(1, 0.0, 0.0, 1.0), 1.0);
        // Step 0.5, base 10, start 0.25: ((11 + 0.5)·0.5) + 0.25 − 0.25.
        assert_eq!(effect_row(1, 10.0, 0.25, 0.5), 5.75);
        // The tendril's 0.002 step: rows 0 and 0.002.
        assert!(tendril_row(0, 0.0, 0.0).abs() < 1e-7);
        assert!((tendril_row(1, 0.0, 0.0) - 0.002).abs() < 1e-7);
    }

    #[test]
    fn the_dome_fades_flip_at_state_three() {
        assert_eq!(dome_fade(1, 0.25), 0.75);
        assert_eq!(dome_fade(2, 0.25), 0.75);
        assert_eq!(dome_fade(3, 0.25), 0.25);
        assert_eq!(dome_fade(4, 0.25), 0.25);
    }

    #[test]
    fn the_barricade_alpha_is_a_fourth_power() {
        assert!((barricade_alpha(1, 0.5) - 0.0625).abs() < 1e-6);
        assert_eq!(barricade_alpha(0, 1.0), 0.0);
        assert!((barricade_alpha(3, 0.5) - 0.0625).abs() < 1e-6);
        assert!((barricade_alpha(4, 1.0) - 1.0).abs() < 1e-6);
        assert_eq!(strikes_alpha(0.25), 0.75);
    }
}

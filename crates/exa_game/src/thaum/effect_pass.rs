//! The four layered draw passes the effect materials run: the generic effect
//! material, the barrier and aura shader, the tendril material and the
//! displacement shell.
//!
//! The shared renderer keeps one 4×4 matrix a supersample layer (four
//! layers). Before a pass draws, each layer `0..=passes` gets its matrix's
//! third diagonal term (the depth term; [`crate::effects::effect_geometry`]
//! calls it the texture row) set from layer 0's own value, the layer's
//! matrices are multiplied again, and after the pass every layer's term is
//! put back. The terms differ only in their step and offset:
//!
//! | pass | arithmetic | step | offset |
//! |---|---|---|---|
//! | effect material, tendril | double | the literal 0.002 (its half the literal 0.001) | 0.5 |
//! | barrier and aura shader | single | the sub-object's level of detail | 0.5 |
//! | displacement shell | single | the sub-object's level of detail | 1.5 |
//!
//! so a term is `((pass + frame + offset) × step) / divisor + base −
//! step × 0.5`, the pass and frame counters added as 64-bit integers before
//! the conversion; the divisor is a global integer holding 1.
//!
//! A pass draws nothing while the global pass count is zero or less (the
//! shell's own node then draws its first entry point itself, see
//! [`crate::effects::shell::own_node_draw`]). Otherwise it sets layer 0's
//! colour and draws the first entry point, then for each layer `1..=passes`
//! sets that layer's colour and draws the layer entry point. The shader and
//! shell passes bind a time uniform (`base + layer × step`) and, while the
//! display switch bit 0x40 is set, a brightness uniform `(light − layer ×
//! (1 / divisor)) × scale`; the shader pass alone binds a texture
//! coordinate `(layer + frame + 0.5) / divisor − 0.5` when its location
//! exists. The effect material and tendril passes bind neither.
//!
//! Evidence: the four material draw callbacks, read from `Exanima.exe`
//! instruction by instruction (the effect material's and tendril's depth
//! term is computed in double precision, the others in single).

use crate::effects::effect_geometry::{
    FORCE_ROW_STEP, LAYERS, ROW_PASSES, force_row, instance_row,
};

/// Which material's pass is being planned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// The generic effect material (`_ForceMat01`).
    Material,
    /// The barrier and aura shader.
    Shader,
    /// The tendril material (`_TDispMat01`).
    Tendril,
    /// The displacement shell material.
    Shell,
}

/// The message the tendril material raises when its geometry is not the
/// effect class it was registered for.
pub const INVALID_BINDING: &str = "Invalid effect geometry binding.";

/// Checks that the node's geometry is the registered effect class. The
/// original looks the class up in its registry and raises
/// [`INVALID_BINDING`] when it is missing; the pass then runs anyway.
pub fn check_geometry_binding(registered: bool) -> Result<(), &'static str> {
    if registered {
        Ok(())
    } else {
        Err(INVALID_BINDING)
    }
}

/// The parameters one pass plan is computed from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassParams {
    /// The global supersample pass count; zero or less draws nothing.
    pub passes: i32,
    /// The shared renderer's frame counter.
    pub frame: i32,
    /// Layer 0's depth term before the pass.
    pub base: f32,
    /// The sub-object's level of detail, the shader and shell passes' step.
    pub lod: f32,
    /// The time uniform's base and step.
    pub time_base: f32,
    pub time_step: f32,
    /// The brightness uniform's base and scale.
    pub light_base: f32,
    pub light_scale: f32,
    /// The display switch bit 0x40 that enables the brightness uniform.
    pub light_enabled: bool,
    /// The shader's texture-coordinate uniform has a location.
    pub coord_bound: bool,
    /// The colours the renderer keeps for its four layers.
    pub colours: [[f32; 4]; LAYERS as usize],
}

impl Default for PassParams {
    fn default() -> Self {
        Self {
            passes: 0,
            frame: 0,
            base: 0.0,
            lod: 0.0,
            time_base: 0.0,
            time_step: 0.0,
            light_base: 1.0,
            light_scale: 1.0,
            light_enabled: true,
            coord_bound: true,
            colours: [[1.0; 4]; LAYERS as usize],
        }
    }
}

/// Which of the two draw entry points a pass uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawStage {
    /// The first entry point, with layer 0.
    First,
    /// The layer entry point, with one supersample layer.
    Layer,
}

/// One draw of a material pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassDraw {
    /// The layer index; zero is the first draw.
    pub layer: i32,
    pub stage: DrawStage,
    /// The layer matrix's depth term during the pass.
    pub depth: f32,
    /// The layer's colour, set before the draw.
    pub colour: [f32; 4],
    /// The texture coordinate the shader pass binds at unit 1.
    pub coord: Option<f32>,
    /// The time uniform.
    pub time: Option<f32>,
    /// The brightness uniform.
    pub light: Option<f32>,
}

impl PassKind {
    /// Whether the pass binds the time and brightness uniforms.
    fn binds_light(self) -> bool {
        matches!(self, Self::Shader | Self::Shell)
    }
}

/// One layer's depth term during the pass.
pub fn depth(kind: PassKind, layer: i32, frame: i32, base: f32, lod: f32) -> f32 {
    match kind {
        PassKind::Material | PassKind::Tendril => literal_depth(layer, frame, base),
        PassKind::Shader => instance_row(layer, frame, base, lod),
        PassKind::Shell => {
            let counter = (i64::from(layer) + i64::from(frame)) as f32;
            ((counter + 1.5) * lod) / ROW_PASSES + base - lod * 0.5
        }
    }
}

/// The effect material's and tendril's depth term: the counter and its 0.5
/// in single precision, then the step, divisor, base and half step in
/// double, narrowed at the end ([`force_row`]).
pub fn literal_depth(layer: i32, frame: i32, base: f32) -> f32 {
    force_row(layer, frame, base)
}

/// The depth terms every layer `0..=passes` takes for the pass; a pass count
/// below zero sets none, and the shell pass sets none unless it draws.
pub fn depths(kind: PassKind, params: &PassParams) -> Vec<f32> {
    if params.passes < 0 || (kind == PassKind::Shell && params.passes < 1) {
        return Vec::new();
    }
    (0..=params.passes)
        .map(|layer| depth(kind, layer, params.frame, params.base, params.lod))
        .collect()
}

/// The pass plan: the first draw, then one draw for each supersample layer.
/// A pass count of zero or less draws nothing.
pub fn plan(kind: PassKind, params: &PassParams) -> Vec<PassDraw> {
    if params.passes < 1 {
        return Vec::new();
    }
    let mut draws = vec![draw(kind, 0, DrawStage::First, params)];
    for layer in 1..=params.passes {
        draws.push(draw(kind, layer, DrawStage::Layer, params));
    }
    draws
}

fn draw(kind: PassKind, layer: i32, stage: DrawStage, params: &PassParams) -> PassDraw {
    // The renderer keeps four layers; a larger count reads past them in the
    // original, so the last layer's colour stands in.
    let colour = params.colours[(layer as usize).min(params.colours.len() - 1)];
    let counter = (i64::from(layer) + i64::from(params.frame)) as f32;
    let coord = (kind == PassKind::Shader && params.coord_bound)
        .then(|| (counter + 0.5) / ROW_PASSES - 0.5);
    let time = kind
        .binds_light()
        .then_some(layer as f32 * params.time_step + params.time_base);
    let light = (kind.binds_light() && params.light_enabled).then(|| {
        if stage == DrawStage::First {
            params.light_base * params.light_scale
        } else {
            (params.light_base - layer as f32 * (1.0 / ROW_PASSES)) * params.light_scale
        }
    });
    PassDraw {
        layer,
        stage,
        depth: depth(kind, layer, params.frame, params.base, params.lod),
        colour,
        coord,
        time,
        light,
    }
}

const _: () = assert!(FORCE_ROW_STEP == 0.002);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_literal_depth_is_computed_in_double() {
        // ((0 + 10) + 0.5) · 0.002 + 0.5 − 0.001 in double, then narrowed.
        let expected = (10.5_f64 * 0.002 + 0.5 - 0.001) as f32;
        assert_eq!(literal_depth(0, 10, 0.5), expected);
        assert_eq!(depth(PassKind::Material, 0, 10, 0.5, 9.0), expected);
        assert_eq!(depth(PassKind::Tendril, 0, 10, 0.5, 9.0), expected);
        // The single-precision order would round the other way here:
        // 0.72 in single, 0.71999997 in double.
        let single = ((10.5_f32 * 0.002) / 1.0 + 0.7) - 0.001;
        assert_eq!(single, 0.72);
        assert_eq!(literal_depth(0, 10, 0.7), 0.719_999_97);
        assert_ne!(literal_depth(0, 10, 0.7), single);
    }

    #[test]
    fn the_shader_and_shell_terms_step_by_the_level_of_detail() {
        // Shader: ((layer + 10) + 0.5) · 0.25 + 0.5 − 0.125.
        assert_eq!(depth(PassKind::Shader, 0, 10, 0.5, 0.25), 3.0);
        assert_eq!(depth(PassKind::Shader, 2, 10, 0.5, 0.25), 3.5);
        // Shell adds 1.5 instead of 0.5.
        assert_eq!(depth(PassKind::Shell, 0, 10, 0.5, 0.25), 3.25);
        assert_eq!(depth(PassKind::Shell, 1, 10, 0.5, 0.25), 3.5);
        // A frame counter at the integer limit still adds as 64-bit.
        assert_eq!(
            depth(PassKind::Shader, 1, i32::MAX, 0.0, 1.0),
            2_147_483_648.0
        );
    }

    #[test]
    fn a_pass_count_of_zero_draws_nothing() {
        let params = PassParams {
            passes: 0,
            frame: 7,
            ..PassParams::default()
        };
        for kind in [
            PassKind::Material,
            PassKind::Shader,
            PassKind::Tendril,
            PassKind::Shell,
        ] {
            assert!(plan(kind, &params).is_empty());
        }
        // Layer 0's term is still set for the three non-shell passes.
        assert_eq!(depths(PassKind::Shader, &params).len(), 1);
        assert!(depths(PassKind::Shell, &params).is_empty());
        assert!(
            depths(
                PassKind::Material,
                &PassParams {
                    passes: -1,
                    ..params
                }
            )
            .is_empty()
        );
    }

    #[test]
    fn the_shader_layers_bind_colour_time_brightness_and_coord() {
        let colours = [
            [1.0, 1.0, 1.0, 1.0],
            [0.5, 0.25, 0.0, 1.0],
            [0.25, 0.0, 0.0, 0.5],
            [0.0, 0.0, 0.0, 0.0],
        ];
        let params = PassParams {
            passes: 2,
            frame: 0,
            lod: 1.0,
            time_base: 2.0,
            time_step: 0.5,
            light_base: 1.0,
            light_scale: 2.0,
            colours,
            ..PassParams::default()
        };
        let draws = plan(PassKind::Shader, &params);
        assert_eq!(draws.len(), 3);
        assert_eq!(draws[0].stage, DrawStage::First);
        assert_eq!(draws[0].colour, [1.0, 1.0, 1.0, 1.0]);
        assert_eq!(draws[1].stage, DrawStage::Layer);
        assert_eq!(draws[1].colour, [0.5, 0.25, 0.0, 1.0]);
        assert_eq!(draws[2].colour, [0.25, 0.0, 0.0, 0.5]);
        let times: Vec<_> = draws.iter().map(|d| d.time).collect();
        assert_eq!(times, vec![Some(2.0), Some(2.5), Some(3.0)]);
        // (1 − layer · 1/1) · 2: the divisor, not the pass count.
        assert_eq!(draws[0].light, Some(2.0));
        assert_eq!(draws[1].light, Some(0.0));
        assert_eq!(draws[2].light, Some(-2.0));
        // (layer + frame + 0.5) / 1 − 0.5.
        assert_eq!(draws[0].coord, Some(0.0));
        assert_eq!(draws[1].coord, Some(1.0));
        assert_eq!(draws[2].coord, Some(2.0));
        let dark = plan(
            PassKind::Shader,
            &PassParams {
                light_enabled: false,
                coord_bound: false,
                ..params
            },
        );
        assert!(dark.iter().all(|d| d.light.is_none() && d.coord.is_none()));
    }

    #[test]
    fn the_shell_binds_time_and_brightness_but_no_coord() {
        let params = PassParams {
            passes: 1,
            time_base: 1.0,
            time_step: 1.0,
            ..PassParams::default()
        };
        let draws = plan(PassKind::Shell, &params);
        assert_eq!(draws.len(), 2);
        assert!(draws.iter().all(|d| d.coord.is_none()));
        assert_eq!(draws[1].time, Some(2.0));
        assert_eq!(draws[1].light, Some(0.0));
    }

    #[test]
    fn the_material_and_tendril_passes_bind_only_colours() {
        let params = PassParams {
            passes: 3,
            frame: 1,
            lod: 4.0,
            ..PassParams::default()
        };
        for kind in [PassKind::Material, PassKind::Tendril] {
            let draws = plan(kind, &params);
            assert_eq!(draws.len(), 4);
            assert!(
                draws
                    .iter()
                    .all(|d| d.time.is_none() && d.light.is_none() && d.coord.is_none())
            );
            // The level of detail does not change the literal term.
            assert_eq!(draws[1].depth, (2.5_f64 * 0.002 - 0.001) as f32);
        }
    }

    #[test]
    fn an_unregistered_geometry_is_reported() {
        assert_eq!(check_geometry_binding(true), Ok(()));
        assert_eq!(
            check_geometry_binding(false),
            Err("Invalid effect geometry binding.")
        );
    }
}

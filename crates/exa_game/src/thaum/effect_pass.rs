//! The four layered draw passes the effect materials run: the generic effect
//! material, the barrier and aura shader, the tendril material and the
//! displacement shell.
//!
//! Each pass draws once with the node's own colour and then once for every
//! supersample pass, changing the texture row and the layer's colour before
//! each. The draws differ only in where the row starts:
//!
//! | pass | row step | first pass offset |
//! |---|---|---|
//! | effect material, tendril | 0.002 | 0.5 |
//! | barrier and aura shader | the node's speed (+0xf8) | 0.5 |
//! | displacement shell | the node's speed (+0xf8) | 1.5 |
//!
//! so a row is `((pass + frame + offset) × step) / passes + base − step ×
//! 0.5`, with the pass and frame counters added as integers before the
//! conversion and the divisor held at 1. The shader and shell passes also
//! bind a per-layer time (`base + pass × step`, the time uniform) and a
//! per-layer brightness (`(light − pass / passes) × scale`); the shader
//! pass binds a texture coordinate `(pass + frame + 0.5) / passes − 0.5` at
//! texture unit 1. The effect material and tendril passes set the layer's
//! colour only; their first draw keeps the colour already bound.
//!
//! Evidence: the four material draw callbacks and the shared renderer's
//! supersample loop, read from `Exanima.exe`.

/// Which material's pass is being planned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PassKind {
    /// The generic effect material (`_ForceMat01` and the displacement
    /// material).
    Material,
    /// The barrier and aura shader.
    Shader,
    /// The tendril material.
    Tendril,
    /// The displacement shell material.
    Shell,
}

/// The material pass's row step.
pub const MATERIAL_STEP: f32 = 0.002;
/// The divisor the original keeps in a global; it holds 1.
pub const PASS_DIVISOR: f32 = 1.0;

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
    /// The supersample pass count; zero or less draws only the first pass.
    pub passes: i32,
    /// The shared renderer's frame counter.
    pub frame: i32,
    /// The texture row the effect starts at.
    pub base: f32,
    /// The node's texture speed (the shell and aura read it from the
    /// effect at +0xf8 the node belongs to; the other passes ignore it).
    pub speed: f32,
    /// The time uniform's base and step.
    pub time_base: f32,
    pub time_step: f32,
    /// The brightness uniform's base and scale.
    pub light_base: f32,
    pub light_scale: f32,
    /// The colours the renderer sets before each layer, in layer order.
    pub colours: [[f32; 4]; 5],
}

impl Default for PassParams {
    fn default() -> Self {
        Self {
            passes: 0,
            frame: 0,
            base: 0.0,
            speed: 0.0,
            time_base: 0.0,
            time_step: 0.0,
            light_base: 1.0,
            light_scale: 1.0,
            colours: [[1.0; 4]; 5],
        }
    }
}

/// Which of the two draw entry points a pass uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawStage {
    /// The node's own pass, before the layer loop.
    First,
    /// One supersample layer.
    Layer,
}

/// One draw of a material pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PassDraw {
    /// The layer index; zero is the first pass.
    pub layer: i32,
    pub stage: DrawStage,
    /// The texture row the pass sets.
    pub row: f32,
    /// The texture coordinate the shader pass binds at unit 1 (0 when the
    /// pass binds none).
    pub coord: f32,
    /// The layer's colour; the first pass keeps the colour already bound.
    pub colour: Option<[f32; 4]>,
    /// The time uniform (0 when the pass binds none).
    pub time: f32,
    /// The brightness uniform (0 when the pass binds none).
    pub light: f32,
}

impl PassKind {
    /// The row step the pass uses for a node speed of `speed`.
    pub fn step(self, speed: f32) -> f32 {
        match self {
            Self::Material | Self::Tendril => MATERIAL_STEP,
            Self::Shader | Self::Shell => speed,
        }
    }

    /// The offset the pass adds to the layer and frame counters.
    pub fn offset(self) -> f32 {
        match self {
            Self::Material | Self::Shader | Self::Tendril => 0.5,
            Self::Shell => 1.5,
        }
    }

    /// Whether the pass binds the time and brightness uniforms.
    fn binds_light(self) -> bool {
        matches!(self, Self::Shader | Self::Tendril | Self::Shell)
    }

    /// Whether the pass binds the texture coordinate at unit 1.
    fn binds_coord(self) -> bool {
        matches!(self, Self::Shader | Self::Shell)
    }
}

/// One pass's row, the counters added as integers before the conversion.
pub fn row(kind: PassKind, layer: i32, frame: i32, base: f32, speed: f32) -> f32 {
    let step = kind.step(speed);
    let counter = (i64::from(layer) + i64::from(frame)) as f32;
    (counter + kind.offset()) * step / PASS_DIVISOR + base - step * 0.5
}

/// The pass plan: the first draw, then one draw for each supersample pass.
/// A pass count of zero or less leaves the first draw alone.
pub fn plan(kind: PassKind, params: &PassParams) -> Vec<PassDraw> {
    let mut draws = vec![draw(kind, 0, DrawStage::First, params)];
    if params.passes < 1 {
        return draws;
    }
    for layer in 1..=params.passes {
        draws.push(draw(kind, layer, DrawStage::Layer, params));
    }
    draws
}

fn draw(kind: PassKind, layer: i32, stage: DrawStage, params: &PassParams) -> PassDraw {
    let first = stage == DrawStage::First;
    let colour = if first {
        None
    } else {
        params
            .colours
            .get(layer as usize)
            .copied()
            .or_else(|| params.colours.last().copied())
    };
    let passes = params.passes.max(1) as f32;
    let coord = if kind.binds_coord() {
        ((i64::from(layer) + i64::from(params.frame)) as f32 + 0.5) / passes - 0.5
    } else {
        0.0
    };
    let time = if kind.binds_light() {
        params.time_base + layer as f32 * params.time_step
    } else {
        0.0
    };
    let light = if kind.binds_light() {
        (params.light_base - layer as f32 / passes) * params.light_scale
    } else {
        0.0
    };
    PassDraw {
        layer,
        stage,
        row: row(kind, layer, params.frame, params.base, params.speed),
        coord,
        colour,
        time,
        light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_add_the_counters_before_the_conversion() {
        // Material: ((layer + 10) + 0.5) · 0.002 + 0.5 − 0.001.
        assert!((row(PassKind::Material, 0, 10, 0.5, 0.0) - 0.52).abs() < 1e-6);
        assert!((row(PassKind::Material, 1, 10, 0.5, 0.0) - 0.522).abs() < 1e-6);
        // Tendril shares the material's 0.002 step.
        assert_eq!(
            row(PassKind::Tendril, 2, 10, 0.5, 3.0),
            row(PassKind::Material, 2, 10, 0.5, 3.0)
        );
        // Shader: ((layer + 10) + 0.5) · 0.25 + 0.5 − 0.125.
        assert_eq!(row(PassKind::Shader, 0, 10, 0.5, 0.25), 3.0);
        assert_eq!(row(PassKind::Shader, 2, 10, 0.5, 0.25), 3.5);
        // Shell adds 1.5 instead of 0.5 and subtracts half a step.
        assert_eq!(row(PassKind::Shell, 0, 10, 0.5, 0.25), 3.25);
        assert_eq!(row(PassKind::Shell, 1, 10, 0.5, 0.25), 3.5);
        // A frame counter beyond the integer range still adds as integers.
        assert!(row(PassKind::Shader, 0, i32::MAX, 0.0, 1.0).is_finite());
    }

    #[test]
    fn a_pass_count_of_zero_draws_only_the_first_pass() {
        let params = PassParams {
            passes: 0,
            frame: 7,
            base: 0.25,
            speed: 0.5,
            ..PassParams::default()
        };
        let draws = plan(PassKind::Shader, &params);
        assert_eq!(draws.len(), 1);
        assert_eq!(draws[0].stage, DrawStage::First);
        assert_eq!(draws[0].colour, None);
        assert_eq!(draws[0].row, ((7.5 * 0.5) + 0.25 - 0.25));
        assert_eq!(draws[0].coord, (7.5 / 1.0) - 0.5);
        assert_eq!(draws[0].light, (1.0 - 0.0) * 1.0);
        assert_eq!(draws[0].time, 0.0);
        assert!(
            plan(
                PassKind::Material,
                &PassParams {
                    passes: -1,
                    ..params
                }
            )
            .len()
                == 1
        );
    }

    #[test]
    fn each_layer_sets_its_colour_time_and_brightness() {
        let colours = [
            [1.0, 1.0, 1.0, 1.0],
            [0.5, 0.25, 0.0, 1.0],
            [0.25, 0.0, 0.0, 0.5],
            [0.0, 0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0, 0.0],
        ];
        let params = PassParams {
            passes: 2,
            frame: 0,
            base: 0.0,
            speed: 1.0,
            time_base: 2.0,
            time_step: 0.5,
            light_base: 1.0,
            light_scale: 2.0,
            colours,
        };
        let draws = plan(PassKind::Shader, &params);
        assert_eq!(draws.len(), 3);
        assert_eq!(draws[1].stage, DrawStage::Layer);
        assert_eq!(draws[1].colour, Some([0.5, 0.25, 0.0, 1.0]));
        assert_eq!(draws[2].colour, Some([0.25, 0.0, 0.0, 0.5]));
        assert_eq!(draws[1].time, 2.5);
        assert_eq!(draws[2].time, 3.0);
        // (1 − layer/2) · 2.
        assert_eq!(draws[1].light, 1.0);
        assert_eq!(draws[2].light, 0.0);
        // The coord: (layer + frame + 0.5) / passes − 0.5.
        assert_eq!(draws[0].coord, -0.25);
        assert_eq!(draws[1].coord, 0.25);
        assert_eq!(draws[2].coord, 0.75);
    }

    #[test]
    fn the_material_pass_binds_no_light_or_coord() {
        let params = PassParams {
            passes: 2,
            frame: 1,
            base: 0.0,
            speed: 4.0,
            ..PassParams::default()
        };
        let draws = plan(PassKind::Material, &params);
        assert_eq!(draws.len(), 3);
        assert!(
            draws
                .iter()
                .all(|draw| draw.time == 0.0 && draw.light == 0.0)
        );
        assert!(draws.iter().all(|draw| draw.coord == 0.0));
        // The speed does not change the material's row.
        assert_eq!(draws[1].row, ((2.5f32 * 0.002) - 0.001));
    }
    #[test]
    fn an_unregistered_geometry_is_reported() {
        assert_eq!(check_geometry_binding(true), Ok(()));
        assert_eq!(
            check_geometry_binding(false),
            Err("Invalid effect geometry binding.")
        );
        assert_eq!(INVALID_BINDING, "Invalid effect geometry binding.");
    }
}

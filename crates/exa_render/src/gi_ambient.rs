//! The voxel GI's shading in the surface programs: how the volume's light
//! at a surface becomes its ambient, indoors and under an open sky. The
//! shaders (`shading.wesl`: `gi_ambient`, `gi_ambient_outdoor`) mirror these
//! functions; the tests pin the original's constants and their order.
//! Rules: docs/RENDERING.md (Voxel GI).

use bevy::math::{Vec3, Vec4};

/// `Ambient.w` indoors: the environment cube's strength in reflections.
pub const AMBIENT_W_INDOOR: f32 = 8.0;
/// `Ambient.w` under an open sky.
pub const AMBIENT_W_OUTDOOR: f32 = 3.0;
/// The level ambient's scale in `Ambient.xyz` with voxel GI.
pub const GI_AMBIENT_SCALE: f32 = 15.0;
/// The environment cube's mip the outdoor ambient reads around the normal.
pub const OUTDOOR_CUBE_LOD: f32 = 7.0;
/// The outdoor ambient's factor on the cube, per unit of `Ambient.w`.
pub const OUTDOOR_CUBE_SCALE: f32 = 5.0;

/// The indoor lift of the light `a` (its length): faint light is raised,
/// (1 − a)²·0.03/(a + 0.016) + (0.47·a + 0.53).
pub fn indoor_lift(a: f32) -> f32 {
    let b = 1.0 - a;
    (b * b * 0.03) / (a + 0.016) + (a * 0.47 + 0.53)
}

/// The outdoor lift of the light `a`: (1 − a)²·0.08/(a + 0.035) + a.
pub fn outdoor_lift(a: f32) -> f32 {
    let b = 1.0 - a;
    (b * b * 0.08) / (a + 0.035) + a
}

/// The outdoor openness: with shadow taps the averaged openness `occ` is
/// raised to 1 − (1 − occ)·a·32 where the light `a` is faint (it is never
/// lowered), so faint sky light is not darkened by the occupancy.
pub fn outdoor_openness(occ: f32, a: f32) -> f32 {
    (1.0 - (1.0 - occ) * a * 32.0).max(occ)
}

/// The indoor ambient from a GI sample `g` (averaged light, openness) and
/// `Ambient.xyz` (`ambient`): light × (ambient × lift × openness).
pub fn indoor(g: Vec4, ambient: Vec3) -> Vec3 {
    let light = g.truncate();
    let b = indoor_lift(light.length());
    light * (ambient * b * g.w)
}

/// The GI's shading under an open sky.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Outdoor {
    /// The ambient: [`Outdoor::light`] × the cube × `Ambient.w`·5.
    pub ambient: Vec3,
    /// The light shaded before the cube factor (the original's `vgi`): the
    /// reflections and the fog take it for the ambient.
    pub light: Vec3,
}

/// The outdoor ambient from a GI sample `g`, `Ambient.xyz` (`ambient`), the
/// environment cube's colour around the normal at mip 7 (`cube`) and
/// `Ambient.w` (`ambient_w`).
pub fn outdoor(g: Vec4, ambient: Vec3, cube: Vec3, ambient_w: f32) -> Outdoor {
    let light = g.truncate();
    let a = light.length();
    let occ = outdoor_openness(g.w, a);
    let b = outdoor_lift(a);
    let light = light * (ambient * b * occ);
    Outdoor {
        ambient: light * (cube * (ambient_w * OUTDOOR_CUBE_SCALE)),
        light,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() <= 1e-5 * b.abs().max(1.0)
    }

    #[test]
    fn the_outdoor_lift_takes_the_original_constants() {
        // (1 − a)²·0.08/(a + 0.035) + a.
        assert!(close(outdoor_lift(0.0), 0.08 / 0.035));
        assert!(close(outdoor_lift(1.0), 1.0));
        assert!(close(outdoor_lift(0.1), 0.81 * 0.08 / 0.135 + 0.1));
        assert!(close(outdoor_lift(0.5), 0.25 * 0.08 / 0.535 + 0.5));
    }

    #[test]
    fn the_indoor_lift_keeps_its_constants() {
        assert!(close(indoor_lift(0.0), 0.03 / 0.016 + 0.53));
        assert!(close(indoor_lift(1.0), 1.0));
        assert!(close(indoor_lift(0.1), 0.81 * 0.03 / 0.116 + 0.577));
    }

    #[test]
    fn the_outdoor_lift_has_no_floor() {
        // Indoors the lift keeps 0.53 + 0.47·a; outdoors mid light is
        // lifted less.
        for a in [0.1, 0.3, 0.6] {
            assert!(outdoor_lift(a) < indoor_lift(a), "{a}");
        }
        assert!(close(outdoor_lift(0.3), 0.49 * 0.08 / 0.335 + 0.3));
    }

    #[test]
    fn faint_light_opens_the_openness_outdoors() {
        // 1 − (1 − occ)·a·32, never below occ: where the light is faint
        // the openness stays near 1, where it is strong it falls to occ.
        assert_eq!(outdoor_openness(0.5, 0.0), 1.0);
        assert!(close(outdoor_openness(0.5, 0.01), 1.0 - 0.5 * 0.01 * 32.0));
        assert_eq!(outdoor_openness(0.5, 1.0), 0.5);
        assert_eq!(outdoor_openness(1.0, 0.3), 1.0);
    }

    #[test]
    fn the_outdoor_ambient_is_the_light_times_the_cube_times_fifteen() {
        let g = Vec4::new(0.03, 0.04, 0.0, 0.8);
        let ambient = Vec3::new(0.2, 0.3, 0.4);
        let cube = Vec3::new(0.1, 0.2, 0.3);
        let o = outdoor(g, ambient, cube, AMBIENT_W_OUTDOOR);
        let a = 0.05f32;
        let occ = (1.0 - 0.2 * a * 32.0f32).max(0.8);
        let b = 0.95 * 0.95 * 0.08 / (a + 0.035) + a;
        let light = Vec3::new(0.03, 0.04, 0.0) * (ambient * b * occ);
        assert!((o.light - light).abs().max_element() < 1e-6);
        let want = light * cube * 15.0;
        assert!((o.ambient - want).abs().max_element() < 1e-6);
    }

    #[test]
    fn the_indoor_ambient_is_the_lifted_light_times_the_level() {
        let g = Vec4::new(0.0, 0.1, 0.0, 0.5);
        let ambient = Vec3::splat(2.0);
        let want = 0.1 * 2.0 * indoor_lift(0.1) * 0.5;
        assert!(close(indoor(g, ambient).y, want));
    }

    #[test]
    fn the_shader_mirrors_the_constants() {
        let src = include_str!("shading.wesl");
        assert!(src.contains("(b0 * b0 * 0.08) / (a + 0.035) + a"));
        assert!(src.contains("max(1.0 - (1.0 - g.a) * a * 32.0, g.a)"));
        assert!(src.contains("0.03 / (a + 0.016) + (a * 0.47 + 0.53)"));
        assert!(src.contains("AMBIENT_W_OUTDOOR * 5.0"));
        assert!(src.contains("const AMBIENT_W_OUTDOOR: f32 = 3.0;"));
        assert!(src.contains("const REFLECTION_STRENGTH: f32 = 8.0;"));
        assert!(src.contains("const GI_SCALE: f32 = 15.0;"));
        let exa = include_str!("exa.wesl");
        assert!(exa.contains("environment_map, environment_sampler, n, 7.0"));
        assert_eq!(OUTDOOR_CUBE_LOD, 7.0);
        assert_eq!(AMBIENT_W_INDOOR, 8.0);
        assert_eq!(GI_AMBIENT_SCALE, 15.0);
        // The material's open-sky flag matches the shader's.
        assert!(exa.contains(&format!(
            "const OUTDOOR: u32 = {}u;",
            crate::material::flags::OUTDOOR
        )));
    }
}

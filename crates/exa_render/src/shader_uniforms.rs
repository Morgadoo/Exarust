//! The original's uniform blocks: the 252-byte `Global` block every program
//! reads, the 112-byte `Material` block of a draw and the three `vec4`s per
//! light.
//!
//! These are the CPU-side values the engine's shader builder uploads. The
//! renderer has its own `ExaParams` (`crate::material`) with the same
//! quantities in Bevy's layout; this module keeps the original layout and
//! arithmetic for comparison and tests (`docs/RENDERING.md`, Shader builder).

use exa_formats::material::{Material, TextureSlot};

/// A field of a std140 block: its name, byte offset and size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LayoutField {
    pub name: &'static str,
    pub offset: usize,
    pub bytes: usize,
}

/// `Global`, std140, 252 bytes, in the order the builder writes it.
pub const GLOBAL_FIELDS: &[LayoutField] = &[
    LayoutField {
        name: "ViewPos",
        offset: 0,
        bytes: 16,
    },
    LayoutField {
        name: "ViewSiz",
        offset: 16,
        bytes: 8,
    },
    LayoutField {
        name: "ViewPrj",
        offset: 24,
        bytes: 8,
    },
    LayoutField {
        name: "CamMat",
        offset: 32,
        bytes: 64,
    },
    LayoutField {
        name: "SdwMat",
        offset: 96,
        bytes: 64,
    },
    LayoutField {
        name: "VXGIScl",
        offset: 160,
        bytes: 16,
    },
    LayoutField {
        name: "Ambient",
        offset: 176,
        bytes: 16,
    },
    LayoutField {
        name: "TimeDlt",
        offset: 192,
        bytes: 8,
    },
    LayoutField {
        name: "ViewRcp",
        offset: 200,
        bytes: 8,
    },
    LayoutField {
        name: "FogColr",
        offset: 208,
        bytes: 16,
    },
    LayoutField {
        name: "FogPos",
        offset: 224,
        bytes: 16,
    },
    LayoutField {
        name: "BufrPos",
        offset: 240,
        bytes: 8,
    },
    LayoutField {
        name: "Wetness",
        offset: 248,
        bytes: 4,
    },
];

/// `Material`, std140, 112 bytes.
pub const MATERIAL_FIELDS: &[LayoutField] = &[
    LayoutField {
        name: "MatDif",
        offset: 0,
        bytes: 16,
    },
    LayoutField {
        name: "MatSpc",
        offset: 16,
        bytes: 16,
    },
    LayoutField {
        name: "MatEmi",
        offset: 32,
        bytes: 16,
    },
    LayoutField {
        name: "MatDrt",
        offset: 48,
        bytes: 16,
    },
    LayoutField {
        name: "MatDsp",
        offset: 64,
        bytes: 8,
    },
    LayoutField {
        name: "DrtUVs",
        offset: 72,
        bytes: 8,
    },
    LayoutField {
        name: "OpcFll",
        offset: 80,
        bytes: 16,
    },
    LayoutField {
        name: "DspSdw",
        offset: 96,
        bytes: 16,
    },
];

/// Bytes of the `Global` block.
pub const GLOBAL_BYTES: usize = 252;
/// Bytes of the `Material` block.
pub const MATERIAL_BYTES: usize = 112;
/// Bytes of one light's three `vec4`s.
pub const LIGHT_BYTES: usize = 48;
/// Bytes the six `vec4`s of the `Light` array occupy.
pub const LIGHT_ARRAY_BYTES: usize = LIGHT_BYTES * crate::shader_program::MAX_LIGHTS;

/// The scene and view values that fill `Global`.
#[derive(Clone, Copy, Debug)]
pub struct ViewInputs {
    /// The camera's eye, world cm.
    pub eye: [f32; 3],
    /// The frame's time delta, written into `ViewPos.w`.
    pub frame_delta: f32,
    /// The window's width and height, texels.
    pub viewport: [u32; 2],
    /// Multisampled/supersampled views write one less than the viewport size.
    pub multisampled: bool,
    /// The vertical field of view, radians.
    pub fov_y: f32,
    /// The camera's view-projection matrix, column-major.
    pub camera_from_world: [[f32; 4]; 4],
    /// The sun's world-to-shadow matrix, column-major.
    pub world_to_shadow: [[f32; 4]; 4],
    /// The GI volume's texture extent in voxels: across X, up, across Z.
    pub gi_extent: [u32; 3],
    /// The GI volume's voxel size, cm (20).
    pub gi_voxel_size: f32,
    /// Shadow taps this sub-frame takes (`SdwSmps`).
    pub shadow_samples: u32,
    /// The level's main ambient, linear.
    pub ambient_linear: [f32; 3],
    /// The irradiance scale: 15 with voxel GI.
    pub ambient_scale: f32,
    /// The ambient cube's scale (`Ambient.w`): 8 indoors, 3 outdoors.
    pub ambient_w: f32,
    /// The exposure (1 in play).
    pub exposure: f32,
    /// The frame time, written to `TimeDlt`.
    pub time_delta: f32,
    /// The fog colour, linear.
    pub fog_colour: [f32; 3],
    /// The fog distance in cm; `FogColr.w` is its inverse.
    pub fog_distance: f32,
    /// `FogPos`: the fog plane's position. The builder computes it from the
    /// camera's near and far points; the rule was not decoded [guess].
    pub fog_position: [f32; 4],
    /// `BufrPos`: the frame-copy offset used by screen-space refraction.
    pub buffer_position: [f32; 2],
    /// The environment's wetness.
    pub wetness: f32,
}

impl Default for ViewInputs {
    fn default() -> Self {
        Self {
            eye: [0.0; 3],
            frame_delta: 0.0,
            viewport: [1920, 1080],
            multisampled: false,
            fov_y: 60f32.to_radians(),
            camera_from_world: [[0.0; 4]; 4],
            world_to_shadow: [[0.0; 4]; 4],
            gi_extent: [256, 64, 256],
            gi_voxel_size: 20.0,
            shadow_samples: 2,
            ambient_linear: [0.0; 3],
            ambient_scale: 15.0,
            ambient_w: 8.0,
            exposure: 1.0,
            time_delta: 0.0,
            fog_colour: [0.0; 3],
            fog_distance: 50_100.0,
            fog_position: [0.0; 4],
            buffer_position: [0.0; 2],
            wetness: 0.0,
        }
    }
}

/// The `Global` block's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalUniforms {
    pub view_pos: [f32; 4],
    pub view_size: [f32; 2],
    pub view_proj: [f32; 2],
    pub cam_mat: [[f32; 4]; 4],
    pub sdw_mat: [[f32; 4]; 4],
    pub voxel_gi_scale: [f32; 4],
    pub ambient: [f32; 4],
    pub time_delta: [f32; 2],
    pub view_reciprocal: [f32; 2],
    pub fog_colour: [f32; 4],
    pub fog_position: [f32; 4],
    pub buffer_position: [f32; 2],
    pub wetness: f32,
}

impl GlobalUniforms {
    /// Fill the block from the frame's view and environment.
    pub fn from_view(view: &ViewInputs) -> Self {
        let width = view.viewport[0].max(1);
        let height = view.viewport[1].max(1);
        let size = if view.multisampled {
            [
                width.saturating_sub(1) as f32,
                height.saturating_sub(1) as f32,
            ]
        } else {
            [width as f32, height as f32]
        };
        let aspect = width as f32 / height as f32;
        let half = (view.fov_y * 0.5).tan();
        let voxel = view.gi_voxel_size.max(f32::EPSILON);
        // World x maps to texture s (the x extent), world z to texture t (the
        // z extent) and world y to texture r (the vertical extent); the
        // material shaders swizzle the lookup with `.xzy`.
        let gi_scale = [
            1.0 / (view.gi_extent[0].max(1) as f32 * voxel),
            1.0 / (view.gi_extent[2].max(1) as f32 * voxel),
            1.0 / (view.gi_extent[1].max(1) as f32 * voxel),
            1.0 / view.shadow_samples.max(1) as f32,
        ];
        let ambient = [
            view.ambient_linear[0] * view.exposure * view.ambient_scale,
            view.ambient_linear[1] * view.exposure * view.ambient_scale,
            view.ambient_linear[2] * view.exposure * view.ambient_scale,
            view.ambient_w,
        ];
        Self {
            view_pos: [view.eye[0], view.eye[1], view.eye[2], view.frame_delta],
            view_size: size,
            // `ViewPrj` is the projection's half extents at 500 cm [inf].
            view_proj: [half * 500.0, aspect * half * 500.0],
            cam_mat: view.camera_from_world,
            sdw_mat: view.world_to_shadow,
            voxel_gi_scale: gi_scale,
            ambient,
            // The builder writes (Δt, −Δt).
            time_delta: [view.time_delta, -view.time_delta],
            view_reciprocal: [1.0 / width as f32, 1.0 / height as f32],
            fog_colour: [
                view.fog_colour[0],
                view.fog_colour[1],
                view.fog_colour[2],
                1.0 / view.fog_distance.max(f32::EPSILON),
            ],
            fog_position: view.fog_position,
            buffer_position: view.buffer_position,
            wetness: view.wetness,
        }
    }

    /// The block's bytes, little-endian f32, at the std140 offsets.
    pub fn to_bytes(&self) -> [u8; GLOBAL_BYTES] {
        let mut out = [0u8; GLOBAL_BYTES];
        put(&mut out, 0, &self.view_pos);
        put(&mut out, 16, &self.view_size);
        put(&mut out, 24, &self.view_proj);
        put(&mut out, 32, &flatten(&self.cam_mat));
        put(&mut out, 96, &flatten(&self.sdw_mat));
        put(&mut out, 160, &self.voxel_gi_scale);
        put(&mut out, 176, &self.ambient);
        put(&mut out, 192, &self.time_delta);
        put(&mut out, 200, &self.view_reciprocal);
        put(&mut out, 208, &self.fog_colour);
        put(&mut out, 224, &self.fog_position);
        put(&mut out, 240, &self.buffer_position);
        put(&mut out, 248, &[self.wetness]);
        out
    }
}

/// Which program's material values are being packed: the UBO writer's
/// three cases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UniformProgram {
    /// `shading_f`.
    Legacy,
    /// `shadeopt_f`.
    Optimized,
    /// `shadesss_f`.
    Subsurface,
}

/// The values around a material the UBO writer needs.
#[derive(Clone, Copy, Debug)]
pub struct MaterialInputs {
    pub program: UniformProgram,
    /// The exposure (1 in play).
    pub exposure: f32,
    /// The blend word's source factor is ONE (the diffuse is pre-scaled by
    /// the opacity).
    pub premultiplied: bool,
    /// The diffuse texture's size, for `DrtUVs`.
    pub diffuse_texture: Option<(u32, u32)>,
    /// The parallax texel scale (X, Y), from the height layer.
    pub height_texels: Option<(u16, u16)>,
    /// The refraction strength (`0x58`).
    pub refraction: f32,
    /// The supersampling pass, which offsets the displacement taps.
    pub subpass: u32,
}

impl Default for MaterialInputs {
    fn default() -> Self {
        Self {
            program: UniformProgram::Optimized,
            exposure: 1.0,
            premultiplied: false,
            diffuse_texture: None,
            height_texels: None,
            refraction: 0.0,
            subpass: 0,
        }
    }
}

/// The `Material` block's values.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MaterialUniforms {
    pub mat_dif: [f32; 4],
    pub mat_spc: [f32; 4],
    pub mat_emi: [f32; 4],
    pub mat_drt: [f32; 4],
    pub mat_dsp: [f32; 2],
    pub drt_uvs: [f32; 2],
    pub opc_fll: [f32; 4],
    pub dsp_sdw: [f32; 4],
}

impl MaterialUniforms {
    /// Pack a record as the original's material writer does.
    pub fn from_material(material: &Material, inputs: &MaterialInputs) -> Self {
        let legacy = material.kind & 0xFF == 0;
        let scale = if legacy { 1.5 } else { 1.0 };
        let subsurface = if inputs.program == UniformProgram::Subsurface {
            material.subsurface
        } else {
            [0.0; 4]
        };
        let diffuse = material.diffuse;
        let linear_diffuse = [
            crate::material::linear(diffuse[0] * scale),
            crate::material::linear(diffuse[1] * scale),
            crate::material::linear(diffuse[2] * scale),
        ];
        let premultiply = if inputs.premultiplied {
            material.opacity
        } else {
            1.0
        };
        let mut mat_dif = [
            linear_diffuse[0] * premultiply * (1.0 - subsurface[0]),
            linear_diffuse[1] * premultiply * (1.0 - subsurface[1]),
            linear_diffuse[2] * premultiply * (1.0 - subsurface[2]),
            material.opacity,
        ];
        // Blended surfaces with an opacity fall-off lose its floor from the
        // colour's alpha: (f + 1)·f.
        if material.falloff_amount.abs() > 0.01 && material.class & 0xF == 3 {
            let f = (-1.8 * material.falloff_exponent).exp2();
            mat_dif[3] -= (f + 1.0) * f;
        }

        let specular = [
            crate::material::linear(material.specular[0]) * material.specular_level,
            crate::material::linear(material.specular[1]) * material.specular_level,
            crate::material::linear(material.specular[2]) * material.specular_level,
        ];
        let has_specular = material.flags & 0x4 != 0;
        let specular_scale =
            if inputs.program == UniformProgram::Optimized && material.flags & 0x40000 != 0 {
                3.0
            } else {
                1.0
            };
        let mut gloss = material.glossiness;
        if legacy && gloss < 0.01 {
            gloss = 0.01;
        }
        let mat_spc = [
            if has_specular {
                specular[0] * specular_scale
            } else {
                0.0
            },
            if has_specular {
                specular[1] * specular_scale
            } else {
                0.0
            },
            if has_specular {
                specular[2] * specular_scale
            } else {
                0.0
            },
            gloss,
        ];

        let lit = material.flags & 7 != 0;
        let emissive = [
            crate::material::linear(material.emissive[0]),
            crate::material::linear(material.emissive[1]),
            crate::material::linear(material.emissive[2]),
        ];
        let emissive_scale = if lit && material.layer(TextureSlot::Emissive).is_some() {
            2.0
        } else {
            1.0
        };
        let mat_emi = if inputs.program == UniformProgram::Subsurface {
            [
                linear_diffuse[0] * subsurface[0],
                linear_diffuse[1] * subsurface[1],
                linear_diffuse[2] * subsurface[2],
                subsurface[3] + 1.0,
            ]
        } else {
            [
                emissive[0] * inputs.exposure * emissive_scale,
                emissive[1] * inputs.exposure * emissive_scale,
                emissive[2] * inputs.exposure * emissive_scale,
                if lit {
                    material.emissive[3]
                } else {
                    material.opacity
                },
            ]
        };

        let blood = crate::shader_program::dirt_mode(material) == 4;
        let mat_drt = if blood {
            [0.45, 0.03, 0.02, 0.5]
        } else {
            [0.0, 0.0, 0.0, 2.0 - material.dirt_amount]
        };

        let drt_uvs = match inputs.diffuse_texture {
            Some((w, h)) if w != 0 && h != 0 && w >= h => [w as f32 / h as f32, 1.0],
            Some((w, h)) if w != 0 && h != 0 => [1.0, h as f32 / w as f32],
            _ => [1.0, 1.0],
        };
        let mat_dsp = match inputs.height_texels {
            Some((x, y)) => [f32::from(x) * 3e-5, f32::from(y) * 3e-5],
            None => [0.0, 0.0],
        };

        let f = (-1.8 * material.falloff_exponent).exp2();
        let inverted = material.extra & 0x100 != 0;
        let sign = if inverted { -1.0 } else { 1.0 };
        let term = (f + 1.0) * material.falloff_amount * sign;
        let opc_fll = [
            if term < 0.0 { 1.0 } else { 0.0 },
            1.8 * material.falloff_exponent * sign,
            term.abs(),
            inputs.refraction,
        ];
        let dsp_sdw = [
            material.displacement_step * 0.004,
            1.0 + 0.125 * inputs.subpass as f32,
            material.displacement_bias * 0.05,
            0.0,
        ];

        Self {
            mat_dif,
            mat_spc,
            mat_emi,
            mat_drt,
            mat_dsp,
            drt_uvs,
            opc_fll,
            dsp_sdw,
        }
    }

    pub fn to_bytes(&self) -> [u8; MATERIAL_BYTES] {
        let mut out = [0u8; MATERIAL_BYTES];
        put(&mut out, 0, &self.mat_dif);
        put(&mut out, 16, &self.mat_spc);
        put(&mut out, 32, &self.mat_emi);
        put(&mut out, 48, &self.mat_drt);
        put(&mut out, 64, &self.mat_dsp);
        put(&mut out, 72, &self.drt_uvs);
        put(&mut out, 80, &self.opc_fll);
        put(&mut out, 96, &self.dsp_sdw);
        out
    }
}

/// The parallax texel scale from a height layer: `P = round(50·amount)`,
/// `Y = round(P·2048/height)`, `X = round(Y·height/width)`.
///
/// The engine's round helper floors; a NaN amount gives zero. The values are
/// stored as 16-bit texel counts before the uniform scales them by 3·10⁻⁵.
pub fn parallax_texels(amount: f32, width: u32, height: u32) -> (u16, u16) {
    if width == 0 || height == 0 || !amount.is_finite() {
        return (0, 0);
    }
    let depth = (50.0 * amount).floor();
    let y = (depth * 2048.0 / height as f32).floor();
    let x = (y * height as f32 / width as f32).floor();
    (
        x.clamp(0.0, f32::from(u16::MAX)) as u16,
        y.clamp(0.0, f32::from(u16::MAX)) as u16,
    )
}

/// One light's three `vec4`s: position/depth scale, colour/attenuation and
/// filter radius/vertical shaping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightUniform {
    pub position: [f32; 4],
    pub colour: [f32; 4],
    pub shape: [f32; 4],
}

/// A light's inputs.
#[derive(Clone, Copy, Debug)]
pub struct LightInputs {
    /// The light's position in cm, or the direction it shines along for a
    /// directional light.
    pub position: [f32; 3],
    /// The stored sRGB colour.
    pub colour: [f32; 3],
    /// The record's intensity.
    pub intensity: f32,
    /// The multiplier, which only scales the GI injection (1 for the
    /// shader's colour), and the per-light dimmer (0 normally).
    pub dimmer: f32,
    /// The radius in cm.
    pub radius: f32,
    /// The shadow softness.
    pub softness: f32,
    /// The shadow map's depth scale (the cube map's far/near term).
    pub depth_scale: f32,
    /// The shadow taps this sub-frame takes.
    pub shadow_samples: u32,
    /// The exposure.
    pub exposure: f32,
    /// `FastLgt`: the filter radius falls to 0.003 instead of 0.005.
    pub fast_light: bool,
    /// The vertical shaping `(y, z, w)` when the light record has it.
    pub shaping: Option<[f32; 3]>,
}

impl Default for LightInputs {
    fn default() -> Self {
        Self {
            position: [0.0; 3],
            colour: [1.0; 3],
            intensity: 1.0,
            dimmer: 0.0,
            radius: 100.0,
            softness: 0.5,
            depth_scale: 1.0,
            shadow_samples: 1,
            exposure: 1.0,
            fast_light: false,
            shaping: None,
        }
    }
}

/// The reciprocal of π doubled, the light colour's normalisation.
const TWO_OVER_PI: f32 = 0.636_619_75;

impl LightUniform {
    /// A point or spot light: `position.w` is the cube map's depth scale,
    /// `colour.w` is `0.8/radius` (the attenuation's inverse reach).
    pub fn point(input: &LightInputs) -> Self {
        let scale = TWO_OVER_PI / input.shadow_samples.max(1) as f32
            * (1.0 - input.dimmer)
            * input.exposure;
        let colour = [
            crate::material::linear(input.colour[0]) * input.intensity * scale,
            crate::material::linear(input.colour[1]) * input.intensity * scale,
            crate::material::linear(input.colour[2]) * input.intensity * scale,
            if input.radius > 0.0 {
                0.8 / input.radius
            } else {
                0.0
            },
        ];
        let filter = if input.fast_light { 0.003 } else { 0.005 };
        Self {
            position: [
                input.position[0],
                input.position[1],
                input.position[2],
                input.depth_scale,
            ],
            colour,
            shape: [
                (input.softness + 1.0) * filter,
                input.shaping.unwrap_or([1.0; 3])[0],
                input.shaping.unwrap_or([1.0; 3])[1],
                input.shaping.unwrap_or([1.0; 3])[2],
            ],
        }
    }

    /// A directional light: `position.xyz` is the direction, its `w` the
    /// depth scale, and the filter radius is half the softness's 0.005 step.
    pub fn directional(input: &LightInputs) -> Self {
        let scale = TWO_OVER_PI / input.shadow_samples.max(1) as f32
            * (1.0 - input.dimmer)
            * input.exposure;
        let filter = if input.fast_light { 0.003 } else { 0.005 };
        Self {
            position: [
                input.position[0],
                input.position[1],
                input.position[2],
                input.depth_scale,
            ],
            colour: [
                crate::material::linear(input.colour[0]) * input.intensity * scale,
                crate::material::linear(input.colour[1]) * input.intensity * scale,
                crate::material::linear(input.colour[2]) * input.intensity * scale,
                if input.radius > 0.0 {
                    0.8 / input.radius
                } else {
                    0.0
                },
            ],
            shape: [
                input.softness * 0.5 * filter,
                input.shaping.unwrap_or([1.0; 3])[0],
                input.shaping.unwrap_or([1.0; 3])[1],
                input.shaping.unwrap_or([1.0; 3])[2],
            ],
        }
    }

    /// Twelve floats in the light array's order.
    pub fn to_array(self) -> [f32; 12] {
        let mut out = [0.0; 12];
        out[0..4].copy_from_slice(&self.position);
        out[4..8].copy_from_slice(&self.colour);
        out[8..12].copy_from_slice(&self.shape);
        out
    }
}

fn put(out: &mut [u8], offset: usize, values: &[f32]) {
    for (i, value) in values.iter().enumerate() {
        let at = offset + i * 4;
        if let Some(slot) = out.get_mut(at..at + 4) {
            slot.copy_from_slice(&value.to_le_bytes());
        }
    }
}

fn flatten(matrix: &[[f32; 4]; 4]) -> [f32; 16] {
    let mut out = [0.0; 16];
    for (i, row) in matrix.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(row);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material() -> Material {
        Material {
            diffuse: [0.5, 0.25, 0.125, 1.0],
            specular: [1.0, 0.5, 0.25, 1.0],
            emissive: [0.0, 0.0, 0.0, 1.0],
            opacity: 0.8,
            glossiness: 0.9,
            specular_level: 0.5,
            class: 0x100,
            flags: 0x2 | 0x4 | 0x8 | 0x20000 | 0x40000,
            kind: 1,
            ..Material::default()
        }
    }

    fn close(a: f32, b: f32) {
        assert!((a - b).abs() < 1e-5, "{a} != {b}");
    }

    #[test]
    fn global_layout_matches_the_std140_block() {
        assert_eq!(GLOBAL_FIELDS[0].name, "ViewPos");
        assert_eq!(GLOBAL_FIELDS[0].offset, 0);
        assert_eq!(GLOBAL_FIELDS[5].offset, 160);
        assert_eq!(GLOBAL_FIELDS[6].offset, 176);
        assert_eq!(GLOBAL_FIELDS[12].offset, 248);
        assert_eq!(GLOBAL_FIELDS[12].bytes, 4);
        assert_eq!(
            GLOBAL_FIELDS[12].offset + GLOBAL_FIELDS[12].bytes,
            GLOBAL_BYTES
        );
        assert_eq!(MATERIAL_FIELDS[4].offset, 64);
        assert_eq!(MATERIAL_FIELDS[7].offset, 96);
        assert_eq!(
            MATERIAL_FIELDS[7].offset + MATERIAL_FIELDS[7].bytes,
            MATERIAL_BYTES
        );
    }

    #[test]
    fn global_values_follow_the_builder() {
        let view = ViewInputs {
            eye: [100.0, 200.0, -300.0],
            frame_delta: 0.012,
            viewport: [1600, 900],
            multisampled: true,
            shadow_samples: 2,
            ambient_linear: [0.1, 0.2, 0.4],
            ambient_scale: 15.0,
            exposure: 1.0,
            ambient_w: 8.0,
            time_delta: 0.5,
            fog_colour: [0.2, 0.3, 0.4],
            fog_distance: 1000.0,
            ..ViewInputs::default()
        };
        let u = GlobalUniforms::from_view(&view);
        assert_eq!(u.view_pos, [100.0, 200.0, -300.0, 0.012]);
        assert_eq!(u.view_size, [1599.0, 899.0]);
        assert_eq!(u.voxel_gi_scale[0], 1.0 / (256.0 * 20.0));
        assert_eq!(u.voxel_gi_scale[1], 1.0 / (256.0 * 20.0));
        assert_eq!(u.voxel_gi_scale[2], 1.0 / (64.0 * 20.0));
        assert_eq!(u.voxel_gi_scale[3], 0.5);
        assert_eq!(u.ambient, [1.5, 3.0, 6.0, 8.0]);
        assert_eq!(u.time_delta, [0.5, -0.5]);
        assert_eq!(u.view_reciprocal, [1.0 / 1600.0, 1.0 / 900.0]);
        close(u.fog_colour[3], 0.001);
        let bytes = u.to_bytes();
        assert_eq!(bytes.len(), 252);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 100.0);
        assert_eq!(f32::from_le_bytes(bytes[240..244].try_into().unwrap()), 0.0);
        assert_eq!(f32::from_le_bytes(bytes[248..252].try_into().unwrap()), 0.0);
    }

    #[test]
    fn material_values_follow_the_ubo_writer() {
        let m = material();
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        close(u.mat_dif[0], crate::material::linear(0.5));
        close(u.mat_dif[1], crate::material::linear(0.25));
        close(u.mat_dif[3], 0.8);
        // The PBR program with a specular texture triples the specular level.
        close(u.mat_spc[0], 1.5);
        close(u.mat_spc[3], 0.9);
        // No emissive layer: the colour stays as stored (zero here).
        close(u.mat_emi[3], 1.0);
        // No dirt mode and no dirt layer: MatDrt.w = 2 − 0x30.
        close(u.mat_drt[3], 2.0);
        close(u.mat_dsp[0], 0.0);
        assert_eq!(u.drt_uvs, [1.0, 1.0]);
        // No fall-off: the packed sign, exponent, amount and refraction.
        assert_eq!(u.opc_fll, [0.0, 0.0, -0.0, 0.0]);
        assert_eq!(u.to_bytes().len(), 112);
    }

    #[test]
    fn opacity_falloff_packs_sign_and_half_precision_floor() {
        let mut m = material();
        m.falloff_amount = 1.0;
        m.falloff_exponent = 2.0;
        m.extra = 0x100;
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        let f = (-1.8f32 * 2.0).exp2();
        close(u.opc_fll[0], 1.0);
        close(u.opc_fll[1], -3.6);
        close(u.opc_fll[2], f + 1.0);
        let e = 2f32.powf((1.0 - 1.0) * -3.6) * (f + 1.0);
        close(e, f + 1.0);
        let e = 2f32.powf((1.0 - 0.0) * -3.6) * (f + 1.0);
        assert!((e - 0.089).abs() < 0.001);
    }

    #[test]
    fn subsurface_packs_the_scatter_colour_and_blur() {
        let mut m = material();
        m.kind = 4;
        m.subsurface = [1.0, 0.8, 0.46, 1.0];
        let u = MaterialUniforms::from_material(
            &m,
            &MaterialInputs {
                program: UniformProgram::Subsurface,
                ..MaterialInputs::default()
            },
        );
        close(u.mat_dif[0], crate::material::linear(0.5) * (1.0 - 1.0));
        assert_eq!(u.mat_dif[0], 0.0);
        close(u.mat_emi[0], crate::material::linear(0.5));
        close(u.mat_emi[1], crate::material::linear(0.25) * 0.8);
        close(u.mat_emi[3], 2.0);
    }

    #[test]
    fn parallax_texels_follow_the_layer_amount() {
        // 0.6 on the menu floor: P = 30, Y = round(30·2048/H), X = Y·H/W.
        assert_eq!(parallax_texels(0.6, 1024, 1024), (60, 60));
        assert_eq!(parallax_texels(0.6, 1024, 2048), (60, 30));
        assert_eq!(parallax_texels(0.0, 1024, 1024), (0, 0));
        assert_eq!(parallax_texels(f32::NAN, 1024, 1024), (0, 0));
    }

    #[test]
    fn torch_light_values_match_the_original() {
        let input = LightInputs {
            position: [0.55, 52.97, 0.0],
            colour: [1.0, 0.68, 0.43],
            intensity: 3.58,
            radius: 671.0,
            softness: 2.0,
            shadow_samples: 2,
            ..LightInputs::default()
        };
        let light = LightUniform::point(&input);
        let expected = 3.58 * 0.636_619_75 / 2.0;
        close(light.colour[0], expected);
        close(light.colour[1], crate::material::linear(0.68) * expected);
        close(light.colour[2], crate::material::linear(0.43) * expected);
        close(light.colour[3], 0.8 / 671.0);
        close(light.shape[0], 0.015);
        assert_eq!(light.shape[1..], [1.0, 1.0, 1.0]);
        assert_eq!(light.position, [0.55, 52.97, 0.0, 1.0]);
        // A directional light's filter is half the softness step.
        let sun = LightUniform::directional(&LightInputs {
            softness: 0.5,
            ..LightInputs::default()
        });
        close(sun.shape[0], 0.5 * 0.5 * 0.005);
    }
}

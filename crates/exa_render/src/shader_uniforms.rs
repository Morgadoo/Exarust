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

/// The three `vec4`s of one light, at their offsets in the `Light` array
/// element (48 bytes, three times the six-slot stride).
pub const LIGHT_FIELDS: &[LayoutField] = &[
    LayoutField {
        name: "Light.position",
        offset: 0,
        bytes: 16,
    },
    LayoutField {
        name: "Light.colour",
        offset: 16,
        bytes: 16,
    },
    LayoutField {
        name: "Light.shape",
        offset: 32,
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
    /// The frame's clock: `ViewPos.w` and `TimeDlt.x` are this value after
    /// the exposure.
    pub time_delta: f32,
    /// The fog colour, linear.
    pub fog_colour: [f32; 3],
    /// The fog distance in cm; `FogColr.w` is its inverse.
    pub fog_distance: f32,
    /// The shadow camera's eye, world cm: `FogPos` is the midpoint of it and
    /// the view's eye.
    pub sun_eye: [f32; 3],
    /// `BufrPos`: the frame-copy offset used by screen-space refraction.
    pub buffer_position: [f32; 2],
    /// The environment's wetness.
    pub wetness: f32,
    /// The super-sampling factor (the setting's 1, 2, 3, 4, 8 or 15):
    /// `TimeDlt.y` is `−exposure` over it.
    pub supersample_factor: u32,
}

impl Default for ViewInputs {
    fn default() -> Self {
        Self {
            eye: [0.0; 3],
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
            sun_eye: [0.0; 3],
            buffer_position: [0.0; 2],
            wetness: 0.0,
            supersample_factor: 4,
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
        // `FogPos` is the half-way point of the eye and the shadow camera's
        // eye, as the engine's vec3 lerp builds it: `a + (b − a)·0.5` per
        // component, in single precision; `w` is never written.
        let fog_position = [
            view.eye[0] + (view.sun_eye[0] - view.eye[0]) * 0.5,
            view.eye[1] + (view.sun_eye[1] - view.eye[1]) * 0.5,
            view.eye[2] + (view.sun_eye[2] - view.eye[2]) * 0.5,
            0.0,
        ];
        Self {
            view_pos: [
                view.eye[0],
                view.eye[1],
                view.eye[2],
                view.time_delta * view.exposure,
            ],
            view_size: size,
            // `ViewPrj` is the projection's half extents at 500 cm [inf].
            view_proj: [half * 500.0, aspect * half * 500.0],
            cam_mat: view.camera_from_world,
            sdw_mat: view.world_to_shadow,
            voxel_gi_scale: gi_scale,
            ambient,
            // `TimeDlt` is (the frame's clock, −1/factor), both after the
            // exposure.
            time_delta: [
                view.time_delta * view.exposure,
                -(1.0 / view.supersample_factor.max(1) as f32) * view.exposure,
            ],
            view_reciprocal: [1.0 / width as f32, 1.0 / height as f32],
            fog_colour: [
                view.fog_colour[0],
                view.fog_colour[1],
                view.fog_colour[2],
                1.0 / view.fog_distance.max(f32::EPSILON),
            ],
            fog_position,
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
    /// The program class nibble of the shader word: the writer triples the
    /// specular colour of a record with a specular map when its bit 0 is
    /// set (classes 1, 3, 5 and 7).
    pub class: u32,
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
    /// The shader word's `DispSdw`: the displacement taps and their scale
    /// are written only when the program has displacement shadows.
    pub displacement_shadows: bool,
    /// The material word's `0x300` mode bits: a translucent (`0x100`) or
    /// refracting (`0x200`) record's emissive colour is not scaled by the
    /// exposure.
    pub material_mode: u32,
    /// The material record's dirt tint (`+0x248` of the runtime record),
    /// written to `MatDrt` in blood mode 4. A record starts with zero; the
    /// weapon-blood spawner writes
    /// [`WEAPON_BLOOD_TINT`](WEAPON_BLOOD_TINT) and the hair program's
    /// record carries [`HAIR_BLOOD_TINT`](HAIR_BLOOD_TINT) [the hair record's
    /// value is a guess: the hair shader's note states its colour].
    pub dirt_tint: Option<[f32; 4]>,
}

/// The dirt tint a spawned weapon's first material gets when it can bleed:
/// the item spawner writes `(1.0, 0.06, 0.04, 0.3)` at the record's
/// `+0x248` beside dirt mode 4 and the `0x100` render flag.
pub const WEAPON_BLOOD_TINT: [f32; 4] = [1.0, 0.06, 0.04, 0.3];

/// The dirt tint the hair program's blood uses (`MatDrt.rgb`): its shader
/// note gives `(0.45, 0.03, 0.02)`. The fourth lane is not read by the
/// program [guess].
pub const HAIR_BLOOD_TINT: [f32; 4] = [0.45, 0.03, 0.02, 0.5];

impl Default for MaterialInputs {
    fn default() -> Self {
        Self {
            program: UniformProgram::Optimized,
            class: 1,
            exposure: 1.0,
            premultiplied: false,
            diffuse_texture: None,
            height_texels: None,
            refraction: 0.0,
            subpass: 0,
            displacement_shadows: false,
            material_mode: 0,
            dirt_tint: None,
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
        // An unlit record draws emission only: the factory zeroes the
        // diffuse's colour (the alpha stays its opacity).
        if crate::shader_program::feature_key(material) & 3 == 0 {
            mat_dif[0] = 0.0;
            mat_dif[1] = 0.0;
            mat_dif[2] = 0.0;
        }
        // A surface with an opacity fall-off loses its floor from the
        // colour's alpha: (f + 1)·f. (The factory's test for blended states
        // compares a four-bit field with 0x100 and never holds, so every
        // fall-off takes this path.)
        if material.falloff_amount.abs() > 0.01 {
            let f = (-1.8 * material.falloff_exponent).exp2();
            mat_dif[3] -= (f + 1.0) * f;
        }

        let specular = [
            crate::material::linear(material.specular[0]) * material.specular_level,
            crate::material::linear(material.specular[1]) * material.specular_level,
            crate::material::linear(material.specular[2]) * material.specular_level,
        ];
        // The factory keeps the specular colour times its level for a record
        // with the specular bit, the unscaled colour for a record with only
        // a reflection cube, and zero otherwise.
        let specular = if material.flags & 0x4 != 0 {
            specular
        } else if material.flags & 0x10_0000 != 0 {
            [
                crate::material::linear(material.specular[0]),
                crate::material::linear(material.specular[1]),
                crate::material::linear(material.specular[2]),
            ]
        } else {
            [0.0; 3]
        };
        let specular_scale = if inputs.class & 1 != 0
            && material.flags & 0x40000 != 0
            && material.layer(TextureSlot::Specular).is_some()
        {
            3.0
        } else {
            1.0
        };
        let mut gloss = material.glossiness;
        if legacy && gloss < 0.01 {
            gloss = 0.01;
        }
        let mat_spc = [
            specular[0] * specular_scale,
            specular[1] * specular_scale,
            specular[2] * specular_scale,
            gloss,
        ];

        let lit = material.flags & 7 != 0;
        let emissive = [
            crate::material::linear(material.emissive[0]),
            crate::material::linear(material.emissive[1]),
            crate::material::linear(material.emissive[2]),
        ];
        // The factory doubles the emissive only when the plug-in-masked word
        // keeps `MatEmi` (`0x8`) beside the lit or specular bits, and the
        // emissive layer is there.
        let emissive_key = crate::shader_program::feature_key(material);
        let emissive_scale =
            if emissive_key & 0x8 != 0 && lit && material.layer(TextureSlot::Emissive).is_some() {
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
            // The writer scales the emissive by the exposure only for a
            // record without the material word's translucent or refracting
            // mode bits; those copy the stored colour straight through.
            let emissive_exposure = if inputs.material_mode & 0x300 == 0 {
                inputs.exposure
            } else {
                1.0
            };
            [
                emissive[0] * emissive_exposure * emissive_scale,
                emissive[1] * emissive_exposure * emissive_scale,
                emissive[2] * emissive_exposure * emissive_scale,
                if lit {
                    material.emissive[3]
                } else {
                    material.opacity
                },
            ]
        };

        // The dirt block. Only a dirt mode writes it: mode 4 (blood) copies
        // the material record's dirt tint into `MatDrt` and leaves `DrtUVs`;
        // any other mode stores `2 − the record's dirt strength` in
        // `MatDrt.w` and the diffuse map's aspect in `DrtUVs`. The engine's
        // block lives on the drawing state, so a field it skips keeps the
        // previous draw's value; Exarust writes zero there. The strength and
        // the tint are the record's own fields; Exarust's material record
        // carries the strength and the caller passes the tint.
        let dirt = crate::shader_program::dirt_mode(material);
        let (mat_drt, drt_uvs) = match dirt {
            0 => ([0.0; 4], [0.0, 0.0]),
            4 => (inputs.dirt_tint.unwrap_or([0.0; 4]), [0.0, 0.0]),
            _ => (
                [0.0, 0.0, 0.0, 2.0 - material.dirt_amount],
                match inputs.diffuse_texture {
                    // The stored width over height in double, narrowed; a
                    // ratio below one (not NaN) turns into `(1, 1/ratio)`.
                    Some((w, h)) => {
                        let aspect = (f64::from(w) / f64::from(h)) as f32;
                        if 1.0 > aspect {
                            [1.0, 1.0 / aspect]
                        } else {
                            [aspect, 1.0]
                        }
                    }
                    None => [1.0, 1.0],
                },
            ),
        };
        let mat_dsp = match inputs.height_texels {
            Some((x, y)) => [(f64::from(x) * 3e-5) as f32, (f64::from(y) * 3e-5) as f32],
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
        let dsp_sdw = if inputs.displacement_shadows {
            [
                (f64::from(material.displacement_step) * 0.004) as f32,
                1.0 + 0.125 * inputs.subpass as f32,
                (f64::from(material.displacement_bias) * 0.05) as f32,
                0.0,
            ]
        } else {
            [0.0; 4]
        };

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

/// The texture a record's refraction draws the screen copy from (unit 7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefractionMap {
    /// The built-in background refraction, for a nearly opaque record.
    BkgRfr01,
    /// The built-in background refraction, for an opacity above 0.85.
    BkgRfr02,
    /// The scene's own refraction texture, low field (`+0x6C`).
    SceneLow,
    /// The scene's own refraction texture, high field (`+0x64`).
    SceneHigh,
}

/// The scene's refraction pair: the texture the engine binds for a nearly
/// opaque surface (`low`, `+0x6C`) and for a clearer one (`high`, `+0x64`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneRefraction {
    pub low: u32,
    pub high: u32,
}

/// The texture the material writer binds to the screen-copy unit when it
/// updates a material.
///
/// Only a record in the liquid mode (the material word's `0x300` bits equal
/// `0x200`) binds a refraction background: without the scene's own
/// refraction texture it loads the built-in `BkgRfr01` for an opacity up to
/// 0.85 (a NaN takes this branch) and `BkgRfr02` above it; with one, the
/// scene's low and high fields. Any other record binds the scene's low field
/// when it has an opacity map and the scene has the texture.
pub fn refraction_map(
    material_word: u32,
    opacity: f32,
    scene: Option<SceneRefraction>,
) -> Option<RefractionMap> {
    if material_word & 0x300 == 0x200 {
        if scene.is_some() {
            return Some(if opacity > 0.85 {
                RefractionMap::SceneHigh
            } else {
                RefractionMap::SceneLow
            });
        }
        return Some(if opacity > 0.85 {
            RefractionMap::BkgRfr02
        } else {
            RefractionMap::BkgRfr01
        });
    }
    if material_word & 0x40_0000 != 0 && scene.is_some() {
        return Some(RefractionMap::SceneLow);
    }
    None
}

/// The parallax texel scale from a height layer: `P = round(50·amount)`,
/// `Y = round(P·(2048/height))`, `X = round((height/width)·Y)`.
///
/// The depth is rounded in single precision, the two scales in double, all
/// to nearest with ties to even (the conversions' default mode), and each
/// stored as a 16-bit texel count before the uniform scales it by 3·10⁻⁵. A
/// NaN or a zero size gives zero, as the conversion's "indefinite" result
/// truncated to 16 bits does. (A one-texel-tall height map turns parallax
/// off; that is the caller's test.)
pub fn parallax_texels(amount: f32, width: u32, height: u32) -> (u16, u16) {
    fn round(value: f64) -> u16 {
        if value.is_finite() && value.abs() < 9.2e18 {
            value.round_ties_even() as i64 as u16
        } else {
            0
        }
    }
    let depth = round(f64::from(amount * 50.0));
    let (w, h) = (f64::from(width as u16), f64::from(height as u16));
    let y = round(f64::from(depth) * (2048.0 / h));
    let x = round(h / w * f64::from(y));
    (x, y)
}

/// Whether a height layer still displaces. The engine drops the height
/// map's material bit (`0x800000`) and the shader's displacement bit
/// (`0x100`) for a one-texel-tall map, and stores the texel scales as zero;
/// any other height keeps them. The stored height is 16 bits.
pub fn height_layer_enabled(height: u32) -> bool {
    (height & 0xFFFF) != 1
}

/// A spot light's vertical shaping terms from its two stored values `q` and
/// `p`: `y = ½·sin(qπ/2)` (`3 − 3/cos(qπ/2)` for a negative `q`), `z = p + 1`
/// and `w = 1/(2^z − 1)`.
pub fn vertical_shaping(q: f32, p: f32) -> [f32; 3] {
    let angle = q * std::f32::consts::FRAC_PI_2;
    let y = if q >= 0.0 {
        0.5 * angle.sin()
    } else {
        3.0 - 3.0 / angle.cos()
    };
    let z = p + 1.0;
    let w = 1.0 / (z.exp2() - 1.0);
    [y, z, w]
}

/// One light's three `vec4`s: position/depth scale, colour/attenuation and
/// filter radius/vertical shaping.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightUniform {
    pub position: [f32; 4],
    pub colour: [f32; 4],
    pub shape: [f32; 4],
}

impl Default for LightUniform {
    /// A slot that has never been filled.
    fn default() -> Self {
        Self {
            position: [0.0; 4],
            colour: [0.0; 4],
            shape: [0.0; 4],
        }
    }
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
        let normalise = TWO_OVER_PI / input.shadow_samples.max(1) as f32;
        let dim = (1.0 - input.dimmer) * input.exposure;
        let colour = [
            crate::material::linear(input.colour[0]) * input.intensity * normalise * dim,
            crate::material::linear(input.colour[1]) * input.intensity * normalise * dim,
            crate::material::linear(input.colour[2]) * input.intensity * normalise * dim,
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
        let normalise = TWO_OVER_PI / input.shadow_samples.max(1) as f32;
        let dim = (1.0 - input.dimmer) * input.exposure;
        let filter = if input.fast_light { 0.003 } else { 0.005 };
        Self {
            position: [
                input.position[0],
                input.position[1],
                input.position[2],
                input.depth_scale,
            ],
            colour: [
                crate::material::linear(input.colour[0]) * input.intensity * normalise * dim,
                crate::material::linear(input.colour[1]) * input.intensity * normalise * dim,
                crate::material::linear(input.colour[2]) * input.intensity * normalise * dim,
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
            layers: vec![exa_formats::material::TextureLayer {
                slot: TextureSlot::Specular,
                texture: "s".into(),
                parameter: 0,
                amount: 0.0,
            }],
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
        assert_eq!(LIGHT_FIELDS[0].offset, 0);
        assert_eq!(LIGHT_FIELDS[2].offset + LIGHT_FIELDS[2].bytes, LIGHT_BYTES);
        assert_eq!(
            LIGHT_BYTES * crate::shader_program::MAX_LIGHTS,
            LIGHT_ARRAY_BYTES
        );
    }

    #[test]
    fn global_values_follow_the_builder() {
        let view = ViewInputs {
            eye: [100.0, 200.0, -300.0],
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
            sun_eye: [300.0, 500.0, -100.0],
            supersample_factor: 4,
            ..ViewInputs::default()
        };
        let u = GlobalUniforms::from_view(&view);
        // `ViewPos.w` and `TimeDlt.x` are the same frame clock.
        assert_eq!(u.view_pos, [100.0, 200.0, -300.0, 0.5]);
        assert_eq!(u.view_size, [1599.0, 899.0]);
        assert_eq!(u.voxel_gi_scale[0], 1.0 / (256.0 * 20.0));
        assert_eq!(u.voxel_gi_scale[1], 1.0 / (256.0 * 20.0));
        assert_eq!(u.voxel_gi_scale[2], 1.0 / (64.0 * 20.0));
        assert_eq!(u.voxel_gi_scale[3], 0.5);
        assert_eq!(u.ambient, [1.5, 3.0, 6.0, 8.0]);
        // `TimeDlt` is the frame's clock after the exposure and −1 over the
        // super-sampling factor, also after the exposure.
        assert_eq!(u.time_delta, [0.5, -0.25]);
        // `FogPos` is the eye + (shadow eye − eye)·0.5; `w` is untouched.
        assert_eq!(u.fog_position, [200.0, 350.0, -200.0, 0.0]);
        assert_eq!(u.view_reciprocal, [1.0 / 1600.0, 1.0 / 900.0]);
        close(u.fog_colour[3], 0.001);
        let bytes = u.to_bytes();
        assert_eq!(bytes.len(), 252);
        assert_eq!(f32::from_le_bytes(bytes[0..4].try_into().unwrap()), 100.0);
        assert_eq!(f32::from_le_bytes(bytes[240..244].try_into().unwrap()), 0.0);
        assert_eq!(f32::from_le_bytes(bytes[248..252].try_into().unwrap()), 0.0);
    }

    #[test]
    fn the_global_time_pairs_follow_the_exposure_and_supersampling() {
        let view = ViewInputs {
            eye: [1.0, 2.0, 3.0],
            time_delta: 0.25,
            exposure: 2.0,
            supersample_factor: 8,
            ..ViewInputs::default()
        };
        let u = GlobalUniforms::from_view(&view);
        assert_eq!(u.view_pos[3], 0.5);
        assert_eq!(u.time_delta, [0.5, -0.25]);
        // The factor is floored at one rather than dividing by zero.
        let u = GlobalUniforms::from_view(&ViewInputs {
            supersample_factor: 0,
            exposure: 1.0,
            ..ViewInputs::default()
        });
        assert_eq!(u.time_delta[1], -1.0);
        // The midpoint is formed as a + (b − a)·0.5, not (a + b)·0.5: with
        // values where the two orders differ the engine's lerp keeps the
        // narrow rounding.
        let u = GlobalUniforms::from_view(&ViewInputs {
            eye: [0.1, 0.0, 0.0],
            sun_eye: [0.3, 0.0, 0.0],
            ..ViewInputs::default()
        });
        assert_eq!(u.fog_position[0], 0.1 + (0.3 - 0.1) * 0.5);
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
        // No dirt mode: the writer skips the dirt block (zero here).
        assert_eq!(u.mat_drt, [0.0; 4]);
        close(u.mat_dsp[0], 0.0);
        assert_eq!(u.drt_uvs, [0.0, 0.0]);
        // No displacement shadows: the tap block stays zero too.
        assert_eq!(u.dsp_sdw, [0.0; 4]);
        // No fall-off: the packed sign, exponent, amount and refraction.
        assert_eq!(u.opc_fll, [0.0, 0.0, -0.0, 0.0]);
        assert_eq!(u.to_bytes().len(), 112);
    }

    #[test]
    fn the_dirt_block_follows_the_dirt_mode() {
        // Modes 1–3 store the strength and the diffuse map's aspect.
        for extra in [0x0u32, 0x1000, 0x2000] {
            let mut m = material();
            m.extra = extra;
            m.layers.push(exa_formats::material::TextureLayer {
                slot: TextureSlot::Dirt,
                texture: "dirt".into(),
                parameter: 0,
                amount: 0.0,
            });
            m.dirt_amount = 0.25;
            let inputs = MaterialInputs {
                diffuse_texture: Some((512, 256)),
                ..MaterialInputs::default()
            };
            let u = MaterialUniforms::from_material(&m, &inputs);
            assert_eq!(u.mat_drt, [0.0, 0.0, 0.0, 1.75]);
            assert_eq!(u.drt_uvs, [2.0, 1.0]);
            let inputs = MaterialInputs {
                diffuse_texture: Some((256, 512)),
                ..MaterialInputs::default()
            };
            let u = MaterialUniforms::from_material(&m, &inputs);
            assert_eq!(u.drt_uvs, [1.0, 2.0]);
            // A zero-width map's ratio 0 still takes the `1/ratio` branch;
            // the NaN of an empty map fails `1 > ratio` and is kept.
            let inputs = MaterialInputs {
                diffuse_texture: Some((0, 4)),
                ..MaterialInputs::default()
            };
            let u = MaterialUniforms::from_material(&m, &inputs);
            assert_eq!(u.drt_uvs, [1.0, f32::INFINITY]);
            let inputs = MaterialInputs {
                diffuse_texture: Some((0, 0)),
                ..MaterialInputs::default()
            };
            let u = MaterialUniforms::from_material(&m, &inputs);
            assert!(u.drt_uvs[0].is_nan());
            assert_eq!(u.drt_uvs[1], 1.0);
        }
        // Blood (mode 4) takes the record's dirt tint and does not write the
        // uv block (zero here).
        let mut m = material();
        m.extra = 0x4000;
        m.dirt_amount = 0.25;
        let u = MaterialUniforms::from_material(
            &m,
            &MaterialInputs {
                diffuse_texture: Some((512, 256)),
                dirt_tint: Some(WEAPON_BLOOD_TINT),
                ..MaterialInputs::default()
            },
        );
        assert_eq!(u.mat_drt, [1.0, 0.06, 0.04, 0.3]);
        assert_eq!(u.drt_uvs, [0.0, 0.0]);
        // A record that has not had a tint written carries zero there.
        let u = MaterialUniforms::from_material(
            &m,
            &MaterialInputs {
                diffuse_texture: Some((512, 256)),
                ..MaterialInputs::default()
            },
        );
        assert_eq!(u.mat_drt, [0.0; 4]);
        // The hair program's record tint.
        let u = MaterialUniforms::from_material(
            &m,
            &MaterialInputs {
                dirt_tint: Some(HAIR_BLOOD_TINT),
                ..MaterialInputs::default()
            },
        );
        assert_eq!(u.mat_drt, [0.45, 0.03, 0.02, 0.5]);
        // A mode the factory does not name writes nothing.
        let mut m = material();
        m.extra = 0x3000;
        m.dirt_amount = 0.5;
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        assert_eq!(u.mat_drt, [0.0; 4]);
    }

    #[test]
    fn displacement_and_parallax_values_use_the_writers_precision() {
        let mut m = material();
        m.displacement_step = 0.7;
        m.displacement_bias = 0.3;
        let inputs = MaterialInputs {
            displacement_shadows: true,
            subpass: 3,
            height_texels: Some((100, 250)),
            ..MaterialInputs::default()
        };
        let u = MaterialUniforms::from_material(&m, &inputs);
        assert_eq!(u.dsp_sdw[0], (f64::from(0.7f32) * 0.004) as f32);
        assert_eq!(u.dsp_sdw[1], 1.375);
        assert_eq!(u.dsp_sdw[2], (f64::from(0.3f32) * 0.05) as f32);
        // The texel counts are widened to double, scaled and narrowed.
        assert_eq!(
            u.mat_dsp,
            [(100.0f64 * 3e-5) as f32, (250.0f64 * 3e-5) as f32]
        );
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
        // Opaque or blended, the fall-off's floor leaves the alpha.
        close(u.mat_dif[3], 0.8 - (f + 1.0) * f);
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
    fn the_emissive_doubles_only_with_the_masked_bit_and_the_layer() {
        let mut m = material();
        m.flags = 0x3 | 0x8 | 0x8_0000;
        m.emissive = [0.5, 0.5, 0.5, 1.0];
        m.layers.push(exa_formats::material::TextureLayer {
            slot: TextureSlot::Emissive,
            texture: "e".into(),
            parameter: 0,
            amount: 0.0,
        });
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        close(u.mat_emi[0], crate::material::linear(0.5) * 2.0);
        // The bit the plug-in mask keeps is `MatEmi`; without it the layer
        // does not double the colour.
        m.flags = 0x3 | 0x8_0000;
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        close(u.mat_emi[0], crate::material::linear(0.5));
    }

    #[test]
    fn a_reflection_only_record_keeps_its_unscaled_specular() {
        let mut m = material();
        // Lit and a reflection cube, but no specular bit: the factory takes
        // the colour without the specular level.
        m.flags = 0x2 | 0x10_0000;
        m.specular = [0.5, 0.25, 0.125, 1.0];
        m.specular_level = 0.5;
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        close(u.mat_spc[0], crate::material::linear(0.5));
        close(u.mat_spc[1], crate::material::linear(0.25));
        close(u.mat_spc[2], crate::material::linear(0.125));
        // Without either bit the colour is zero.
        m.flags = 0x2;
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        assert_eq!(u.mat_spc[0..3], [0.0, 0.0, 0.0]);
    }

    #[test]
    fn an_unlit_record_keeps_only_its_emission() {
        let mut m = material();
        // The lit bits are 0x3 in the record's flags; clearing them makes
        // the factory zero the diffuse colour and keep its alpha.
        m.flags = 0x8 | 0x20000;
        m.emissive = [0.5, 0.5, 0.5, 0.25];
        m.opacity = 0.75;
        let u = MaterialUniforms::from_material(&m, &MaterialInputs::default());
        assert_eq!(u.mat_dif[0..3], [0.0, 0.0, 0.0]);
        assert_eq!(u.mat_dif[3], 0.75);
        close(u.mat_emi[0], crate::material::linear(0.5));
        // The emissive alpha of an unlit record is its opacity.
        assert_eq!(u.mat_emi[3], 0.75);
    }

    #[test]
    fn a_translucent_or_refracting_record_keeps_its_emissive_unexposed() {
        let mut m = material();
        m.emissive = [0.5, 0.25, 0.125, 0.75];
        let inputs = MaterialInputs {
            exposure: 2.0,
            ..MaterialInputs::default()
        };
        let u = MaterialUniforms::from_material(&m, &inputs);
        close(u.mat_emi[0], crate::material::linear(0.5) * 2.0);
        // The material word's mode bits 0x100 (translucent) and 0x200
        // (refracting) skip the exposure.
        for mode in [0x100u32, 0x200] {
            let u = MaterialUniforms::from_material(
                &m,
                &MaterialInputs {
                    material_mode: mode,
                    ..inputs
                },
            );
            close(u.mat_emi[0], crate::material::linear(0.5));
            close(u.mat_emi[1], crate::material::linear(0.25));
        }
        // Bits outside the word's 0x300 field do not count.
        let u = MaterialUniforms::from_material(
            &m,
            &MaterialInputs {
                material_mode: 0x400,
                ..inputs
            },
        );
        close(u.mat_emi[0], crate::material::linear(0.5) * 2.0);
    }

    #[test]
    fn refraction_picks_the_background_or_the_scenes_own_texture() {
        let scene = SceneRefraction { low: 11, high: 12 };
        // The liquid mode with no scene texture loads the built-in pair; a
        // NaN opacity takes the low branch, as the original's compare does.
        assert_eq!(
            refraction_map(0x200, 0.5, None),
            Some(RefractionMap::BkgRfr01)
        );
        assert_eq!(
            refraction_map(0x200, 0.85, None),
            Some(RefractionMap::BkgRfr01)
        );
        assert_eq!(
            refraction_map(0x200, 0.8501, None),
            Some(RefractionMap::BkgRfr02)
        );
        assert_eq!(
            refraction_map(0x200, f32::NAN, None),
            Some(RefractionMap::BkgRfr01)
        );
        // With the scene's own refraction texture both fields are used.
        assert_eq!(
            refraction_map(0x200, 0.4, Some(scene)),
            Some(RefractionMap::SceneLow)
        );
        assert_eq!(
            refraction_map(0x200, 0.9, Some(scene)),
            Some(RefractionMap::SceneHigh)
        );
        // Any other record takes the scene's low field only with an opacity
        // map, and nothing without the scene texture.
        assert_eq!(
            refraction_map(0x7 | 0x40_0000, 1.0, Some(scene)),
            Some(RefractionMap::SceneLow)
        );
        assert_eq!(refraction_map(0x7, 1.0, Some(scene)), None);
        assert_eq!(refraction_map(0x7 | 0x40_0000, 1.0, None), None);
        // The translucent bit 0x100 is not the refracting mode.
        assert_eq!(refraction_map(0x100, 0.5, None), None);
    }

    #[test]
    fn parallax_texels_follow_the_layer_amount() {
        // 0.6 on the menu floor: P = 30, Y = round(30·2048/H), X = Y·H/W.
        assert_eq!(parallax_texels(0.6, 1024, 1024), (60, 60));
        assert_eq!(parallax_texels(0.6, 1024, 2048), (60, 30));
        assert_eq!(parallax_texels(0.0, 1024, 1024), (0, 0));
        assert_eq!(parallax_texels(f32::NAN, 1024, 1024), (0, 0));
        assert_eq!(parallax_texels(0.6, 0, 0), (0, 0));
        // Ties go to even: 0.25·50 = 12.5 → 12, and 12·2048/1536 = 16.
        assert_eq!(parallax_texels(0.25, 1536, 1536), (16, 16));
        // 0.31·50 = 15.5 → 16: rounding, not flooring.
        assert_eq!(parallax_texels(0.31, 1024, 1024).1, 32);
        // A one-texel-tall height map turns the whole height layer off.
        assert!(!height_layer_enabled(1));
        assert!(height_layer_enabled(0));
        assert!(height_layer_enabled(2));
    }

    #[test]
    fn vertical_shaping_matches_the_light_record() {
        // q = 1 straight down: y = ½, z = 1, w = 1.
        assert_eq!(vertical_shaping(1.0, 0.0), [0.5, 1.0, 1.0]);
        // q = 0: no vertical narrowing.
        assert_eq!(vertical_shaping(0.0, 0.0), [0.0, 1.0, 1.0]);
        // p = 1: z = 2, w = 1/3.
        let [_, z, w] = vertical_shaping(1.0, 1.0);
        assert_eq!(z, 2.0);
        assert!((w - 1.0 / 3.0).abs() < 1e-6);
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

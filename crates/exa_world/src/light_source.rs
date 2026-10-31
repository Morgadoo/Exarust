//! Light-source nodes: the state the original keeps beside a light's colour
//! and radius, and what a lever or the flicker timer does with it.
//!
//! A light starts with its base intensity, a copy of it as the current
//! intensity, a zero scale and the constructor's animation parameters
//! (amplitude 0.1, speed 0.5, emissive share 0.5); a body with flag
//! `0x10000` or `0x20000` stores its own ([`exa_formats::rfc::LightAnimation`]).
//! Activation copies the base intensity into the current one and picks the
//! light's shadow-quality offset from the low two flag bits (0, +1, −1, −2).
//!
//! **Lever**: the scale becomes the lever's value, the intensity
//! `base × scale`, and the parent mesh's emissive share `scale × emissive`.
//!
//! **Flicker** (flag `0x10000` without `0x20000`), once a frame: a term
//! random-walks by the frame's step (speed × frame milliseconds × 0.01),
//! drifting down above a scale of 0.9, up below −0.9 and either way
//! between; the scale then moves by the term (×0.1 per millisecond and a
//! second draw) and is held to ±1. The intensity is
//! `(scale·amplitude·0.1 + 1)·base` and the parent mesh's emissive share
//! `scale × emissive + 1`. **Fade** (flag `0x20000`): the scale climbs by
//! the step while the term is positive, else falls by it; on reaching one
//! (or the amplitude, falling) the intensity settles there and flag
//! `0x10000` clears. The steps are in double precision and the draws come
//! from the engine's `× 0x08088405 + 1` generator, as the original's.
//!
//! The frame milliseconds are the scene's frame time, held to 0…16.667 ms.
//! The share the parent mesh gets is applied to its first part's emissive
//! colour (the original writes a scale into that sub-object; its draw-side
//! reader is not traced, so scaling the emissive colour is Exarust's
//! reading). The part gets its own material copy first, so other meshes
//! sharing the material keep theirs.
//!
//! Rules: `docs/RENDERING.md` (lights), `docs/formats/RFC.md` (light node).

use bevy::prelude::*;

use exa_formats::rfc::Light;

/// Light flag (`0x10000`): the light flickers, or fades while [`FADE`] is
/// set (a fade clears it on arrival).
pub const FLICKER: u32 = 0x1_0000;
/// Light flag (`0x20000`): the light fades instead of flickering.
pub const FADE: u32 = 0x2_0000;
/// The constructor's flicker amplitude.
pub const DEFAULT_AMPLITUDE: f32 = 0.1;
/// The constructor's flicker and fade speed.
pub const DEFAULT_SPEED: f32 = 0.5;
/// The constructor's emissive share.
pub const DEFAULT_EMISSIVE: f32 = 0.5;
/// The longest frame the scene's frame time reports, milliseconds.
pub const FRAME_MS_CAP: f32 = 16.666_666;

/// The state a light node keeps beside its colour and radius.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct LightSource {
    /// The light's body flags: [`FLICKER`], [`FADE`] and the shadow
    /// quality's two low bits.
    pub flags: u32,
    /// The stored intensity, which a lever and the flicker scale.
    pub base_intensity: f32,
    /// The current intensity.
    pub intensity: f32,
    /// The scale a lever sets and the flicker moves.
    pub scale: f32,
    /// The flicker's random-walk term.
    pub flicker: f32,
    /// The flicker amplitude (the level a fade falls to).
    pub amplitude: f32,
    /// The flicker and fade speed.
    pub speed: f32,
    /// The share of the scale the parent mesh's emissive takes.
    pub emissive: f32,
    /// The shadow-quality offset activation picks: 0, 1, −1 or −2.
    pub shadow_offset: i32,
    /// The parent mesh's emissive scale the last lever or flicker step set
    /// (1 until one runs).
    pub fixture_scale: f32,
    /// The pending-change bits a lever or fade step sets (`0xB`).
    pub dirty: u32,
    /// The record's multiplier (1 as constructed): the GI colour and the
    /// light node's own colour scale by it.
    pub multiplier: f32,
    /// The light's reach in metres, as loaded.
    pub radius: f32,
    /// The reach the last update reported; a change of more than 1 marks
    /// the shadow links for a rebuild.
    pub last_radius: f32,
    /// The running colour sum: a derive adds the second colour to it, and
    /// the update replaces it by the direct colour.
    pub offset: [f32; 3],
    /// The light's colour, sRGB as stored (white as constructed).
    pub colour: [f32; 3],
}

impl LightSource {
    /// A light node from its record's flags and intensity, with the
    /// constructor's defaults.
    pub fn new(flags: u32, intensity: f32) -> Self {
        Self {
            flags,
            base_intensity: intensity,
            intensity,
            scale: 0.0,
            flicker: 0.0,
            amplitude: DEFAULT_AMPLITUDE,
            speed: DEFAULT_SPEED,
            emissive: DEFAULT_EMISSIVE,
            shadow_offset: 0,
            fixture_scale: 1.0,
            dirty: 0,
            multiplier: 1.0,
            radius: 0.0,
            last_radius: 0.0,
            offset: [0.0; 3],
            colour: [1.0; 3],
        }
    }

    /// The light node as loaded: the record's flags and intensity, and its
    /// animation parameters when it stores them.
    pub fn of(light: &Light) -> Self {
        let mut source = Self::new(light.flags, light.intensity);
        source.radius = light.radius;
        source.colour = light.color;
        source.multiplier = light.multiplier;
        if let Some(animation) = light.animation {
            source.amplitude = animation.amplitude;
            source.speed = animation.speed;
            source.emissive = animation.emissive;
        }
        source
    }

    /// Activation: the current intensity becomes the base and the
    /// shadow-quality offset comes from the flags' low two bits.
    pub fn activate(&mut self) {
        self.intensity = self.base_intensity;
        self.shadow_offset = shadow_quality_offset(self.flags);
    }

    /// Whether the flicker timer runs for the light.
    pub fn animates(&self) -> bool {
        self.flags & (FLICKER | FADE) != 0
    }

    /// A lever's scale: `intensity = base × scale`, the parent mesh's
    /// emissive scale `scale × emissive`. Returns that emissive scale.
    pub fn set_scale(&mut self, scale: f32) -> f32 {
        self.scale = scale;
        self.intensity = self.base_intensity * self.scale;
        self.fixture_scale = self.scale * self.emissive;
        self.dirty |= 0xB;
        self.fixture_scale
    }

    /// One frame of the flicker timer at `frame_ms` (the scene's frame
    /// time), drawing from `random` (the engine's generator) only on the
    /// flicker path, twice, in the original's order. A light whose flags
    /// hold neither bit is not stepped by the original; calling this on one
    /// runs the flicker path.
    pub fn flicker_step(&mut self, random: &mut impl FnMut() -> f32, frame_ms: f32) {
        let step = f64::from(self.speed * frame_ms) * 0.01;
        if self.flags & FADE == 0 {
            let step = step as f32;
            let scale = f64::from(self.scale);
            // Double-precision compares; a NaN scale takes the walk.
            if scale >= 0.9 {
                let r = random();
                self.flicker -= r * step;
            } else if scale <= -0.9 {
                let r = random();
                self.flicker += r * step;
            } else {
                let r = random();
                self.flicker += 2.0 * step * (r - 0.5);
            }
            let r = random();
            let moved = f64::from(frame_ms) * 0.1 * f64::from(r * self.flicker) + scale;
            // A NaN scale is kept.
            self.scale = (moved as f32).clamp(-1.0, 1.0);
            self.fixture_scale = self.scale * self.emissive + 1.0;
            let gain = f64::from(self.scale * self.amplitude) * 0.1 + 1.0;
            self.intensity = (gain * f64::from(self.base_intensity)) as f32;
        } else {
            // A NaN term falls.
            if self.flicker > 0.0 {
                self.scale = (step + f64::from(self.scale)) as f32;
                if self.scale >= 1.0 {
                    self.intensity = self.base_intensity;
                    self.flags &= !FLICKER;
                } else {
                    self.intensity = self.base_intensity * self.scale;
                }
            } else {
                self.scale = (f64::from(self.scale) - step) as f32;
                if self.scale <= self.amplitude {
                    self.intensity = self.amplitude * self.base_intensity;
                    self.flags &= !FLICKER;
                } else {
                    self.intensity = self.base_intensity * self.scale;
                }
            }
            self.fixture_scale = self.scale * self.emissive;
            self.dirty |= 0xB;
        }
    }
}

/// One update's derived colours, in the original's single precision (the
/// colours linearised where the light's workflow linearises them).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct LightUpdate {
    /// The direct colour: the light's colour scaled by its intensity.
    pub colour: [f32; 3],
    /// The second colour slot: the direct colour, or black with node flag
    /// [`NODE_NO_SECOND_COLOUR`].
    pub second: [f32; 3],
    /// The running sum after the update: the direct colour.
    pub offset: [f32; 3],
    /// The sum's Rec. 601 luma ([`exa_formats::colour::luma`]).
    pub luma: f32,
    /// The GI colour: the direct colour times [`LightSource::multiplier`],
    /// or black with node flag [`NODE_NO_GI`].
    pub gi: [f32; 3],
    /// The light node's own colour: the colour linearised, times
    /// `√intensity · multiplier · 3` (linear workflow) or
    /// `√(intensity · multiplier) · 0.6` (legacy, the 0.6 in double).
    pub node_colour: [f32; 3],
    /// The reach the shadows use: the radius, or zero while the multiplier
    /// is below 2⁻²³.
    pub radius: f32,
    /// The reach moved by more than one since the last update, so the
    /// shadow links are due a rebuild.
    pub radius_changed: bool,
}

impl LightSource {
    /// The direct colour: linearised and times the intensity in the linear
    /// workflow, else the stored colour times `√intensity · 0.75`. A NaN
    /// intensity passes through.
    pub fn direct_colour(&self, linear: bool) -> [f32; 3] {
        if linear {
            let c = exa_render::material::linear_rgb(self.colour) * self.intensity;
            [c.x, c.y, c.z]
        } else {
            let scale = self.intensity.sqrt() * 0.75;
            self.colour.map(|c| c * scale)
        }
    }

    /// The light node's own colour: the colour linearised, then times
    /// `√intensity · multiplier · 3` in the linear workflow or
    /// `√(intensity · multiplier) · 0.6` (the 0.6 in double) in the legacy
    /// one. Activation and the clone copy compute it the same way.
    pub fn node_colour(&self, linear: bool) -> [f32; 3] {
        let c = exa_render::material::linear_rgb(self.colour);
        let scale = if linear {
            self.intensity.sqrt() * self.multiplier * 3.0
        } else {
            (f64::from((self.intensity * self.multiplier).sqrt()) * 0.6) as f32
        };
        [c.x * scale, c.y * scale, c.z * scale]
    }

    /// The derive step: the direct colour, then the second colour (black
    /// with node flag [`NODE_NO_SECOND_COLOUR`], which also leaves the
    /// running sum alone; otherwise the sum takes the second colour).
    /// Returns the direct and second colours.
    pub fn derive(&mut self, node_flags: u32, linear: bool) -> ([f32; 3], [f32; 3]) {
        let colour = self.direct_colour(linear);
        if node_flags & NODE_NO_SECOND_COLOUR != 0 {
            return (colour, [0.0; 3]);
        }
        self.offset = [
            self.offset[0] + colour[0],
            self.offset[1] + colour[1],
            self.offset[2] + colour[2],
        ];
        (colour, colour)
    }

    /// The light's update: an intensity under 2⁻²³ becomes zero (an
    /// ordered compare, so a NaN stays), the derive runs, the running sum
    /// becomes the direct colour and its luma is taken, the GI colour and
    /// the node's own colour follow, and the reach is the radius (zero
    /// while the multiplier is under 2⁻²³; a NaN multiplier keeps the
    /// radius). Every update sets pending bit 4; a reach that moved by more
    /// than 1 (an unordered difference does not) is stored and sets bit 2.
    pub fn update(&mut self, node_flags: u32, linear: bool) -> LightUpdate {
        if self.intensity < TINY {
            self.intensity = 0.0;
        }
        let (colour, second) = self.derive(node_flags, linear);
        self.offset = colour;
        let luma = exa_formats::colour::luma(self.offset);
        let gi = if node_flags & NODE_NO_GI != 0 {
            [0.0; 3]
        } else {
            colour.map(|c| c * self.multiplier)
        };
        let node_colour = self.node_colour(linear);
        let radius = if self.multiplier < TINY {
            0.0
        } else {
            self.radius
        };
        let radius_changed = (radius - self.last_radius).abs() > 1.0;
        if radius_changed {
            self.last_radius = radius;
            self.dirty |= 2;
        }
        self.dirty |= 4;
        LightUpdate {
            colour,
            second,
            offset: self.offset,
            luma,
            gi,
            node_colour,
            radius,
            radius_changed,
        }
    }
}

/// The shadow-quality offset the low two flag bits select: 00 → 0,
/// 01 → +1, 10 → −1, 11 → −2.
pub fn shadow_quality_offset(flags: u32) -> i32 {
    match flags & 3 {
        0 => 0,
        1 => 1,
        2 => -1,
        _ => -2,
    }
}

/// Node flag (`flags2`) `0x10000`: activation and the update give the light
/// no GI colour ([`DerivedLight::gi_colour`], [`LightUpdate::gi`]).
pub const NODE_NO_GI: u32 = 0x1_0000;
/// Node flag (`flags2`) `0x20000`: activation and the update zero the
/// light's second colour slot ([`DerivedLight::second_colour`],
/// [`LightUpdate::second`]), and the derive leaves the running sum alone.
pub const NODE_NO_SECOND_COLOUR: u32 = 0x2_0000;
/// Node flag (`flags2`) `0x100000`: activation copies it into a byte of the
/// light and the alias sync into each alias (its reader is not traced; the
/// light constructor sets it).
pub const NODE_FLAG_BIT: u32 = 0x10_0000;
/// The far plane an unranged light uses (the original's `3.4e38`).
pub const UNRANGED_FAR: f32 = f32::from_bits(0x7f7f_c99e);
/// The engine's small-value threshold: an intensity below it becomes 0.
const TINY: f32 = f32::from_bits(0x33ff_ffd7);

/// The light values activation derives from a record on the original's
/// modern-lighting path (its GI setting on, the one Exarust renders): the
/// linear colour times the current intensity, its Rec. 601 luminance, the
/// GI colour, the light-node colour and the falloff.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedLight {
    /// The current intensity after activation's clamp (below the engine's
    /// epsilon it becomes 0; a NaN stays).
    pub intensity: f32,
    /// The linear colour multiplied by the intensity (the shader's direct
    /// colour source).
    pub colour: [f32; 3],
    /// The second colour slot: [`DerivedLight::colour`], or zero with node
    /// flag [`NODE_NO_SECOND_COLOUR`] (its reader is not traced).
    pub second_colour: [f32; 3],
    /// The colour the voxel GI injects: the direct colour times the
    /// record's multiplier, zero with node flag [`NODE_NO_GI`].
    pub gi_colour: [f32; 3],
    /// The light node's own colour: the linear colour times
    /// `√intensity · multiplier · 3` (single precision, in that order).
    pub inject_colour: [f32; 3],
    /// The scale of [`DerivedLight::inject_colour`].
    pub inject_scale: f32,
    /// Rec. 601 luminance of [`DerivedLight::colour`]
    /// ([`exa_formats::colour::luma`]).
    pub luminance: f32,
    /// The shadow's far plane and the light node's bounding radius: the
    /// radius when ranged, else [`UNRANGED_FAR`].
    pub reach: f32,
    /// The falloff coefficient `0.8 / radius` (`1 / radius` in single
    /// precision, the product in double) when ranged, else 0: the shader's
    /// `(1 − d · k)⁴`. The original's radius is in centimetres; Exarust's
    /// is in metres, so `k` is per metre.
    pub attenuation: f32,
    /// The light's shadow-quality offset (`0`, `+1`, `−1` or `−2`).
    pub shadow_offset: i32,
    /// The vertical shaping's `y` when the light has a spot, else 1.
    pub shaping_y: f32,
    /// The vertical shaping's `z` when the light has a spot, else 1.
    pub shaping_z: f32,
}

impl DerivedLight {
    /// Derives a light's render values from its record, its node's
    /// `flags2` word and its current intensity. Ranged means the record's
    /// [`Light::ranged`] (the node flag and a radius above zero).
    pub fn of(light: &Light, node_flags: u32, intensity: f32) -> Self {
        // An ordered compare: a NaN intensity is kept.
        let intensity = if intensity < TINY { 0.0 } else { intensity };
        let linear = exa_render::material::linear_rgb(light.color);
        let colour = [
            linear.x * intensity,
            linear.y * intensity,
            linear.z * intensity,
        ];
        let (shaping_y, shaping_z) = crate::vertical_shaping(light);
        let inject_scale = intensity.sqrt() * light.multiplier * 3.0;
        let gi_colour = if node_flags & NODE_NO_GI != 0 {
            [0.0; 3]
        } else {
            colour.map(|c| c * light.multiplier)
        };
        let second_colour = if node_flags & NODE_NO_SECOND_COLOUR != 0 {
            [0.0; 3]
        } else {
            colour
        };
        Self {
            intensity,
            colour,
            second_colour,
            gi_colour,
            inject_colour: [
                linear.x * inject_scale,
                linear.y * inject_scale,
                linear.z * inject_scale,
            ],
            inject_scale,
            luminance: exa_formats::colour::luma(colour),
            reach: if light.ranged {
                light.radius
            } else {
                UNRANGED_FAR
            },
            attenuation: if light.ranged {
                (f64::from(1.0 / light.radius) * 0.8) as f32
            } else {
                0.0
            },
            shadow_offset: shadow_quality_offset(light.flags),
            shaping_y,
            shaping_z,
        }
    }
}

/// The values a light's alias sync copies onto each alias: the node word's
/// bits 16–23 (the alias keeps its other bits), the derived direct colour
/// (into two slots), the GI and second colours, the luminance, the
/// falloff, the softness, the [`NODE_FLAG_BIT`] and the reach. The
/// original also copies the light's world matrix and a frame stamp, which
/// Exarust's alias nodes take from their own transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AliasValues {
    /// The node word bits 16–23 the alias takes.
    pub node_flags: u32,
    pub colour: [f32; 3],
    pub second_colour: [f32; 3],
    pub gi_colour: [f32; 3],
    pub luminance: f32,
    pub attenuation: f32,
    pub softness: f32,
    pub flag_bit: bool,
    pub reach: f32,
}

impl AliasValues {
    /// The alias copy of a light's derived values.
    pub fn of(derived: &DerivedLight, node_flags: u32, softness: f32) -> Self {
        Self {
            node_flags: node_flags & 0x00FF_0000,
            colour: derived.colour,
            second_colour: derived.second_colour,
            gi_colour: derived.gi_colour,
            luminance: derived.luminance,
            attenuation: derived.attenuation,
            softness,
            flag_bit: node_flags & NODE_FLAG_BIT != 0,
            reach: derived.reach,
        }
    }

    /// An alias node word after the sync: bits 16–23 from the light, the
    /// rest the alias's own.
    pub fn alias_word(&self, alias_word: u32) -> u32 {
        self.node_flags | alias_word & 0xFF00_FFFF
    }
}

/// The scene's frame time for the flicker: whole milliseconds since the
/// last frame, held to 0…[`FRAME_MS_CAP`].
pub fn frame_ms(delta_ms: u128) -> f32 {
    let ms = delta_ms as f32;
    if f64::from(ms) > 16.666_666_666_666_668 {
        FRAME_MS_CAP
    } else {
        ms.max(0.0)
    }
}

/// The engine's generator the flicker draws from: `x ← x·0x08088405 + 1`,
/// a draw `x·2⁻³²` rounded to single precision. The original shares one
/// generator across the engine; the flicker keeps a stream of its own.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FlickerRng(pub u32);

impl FlickerRng {
    /// The next draw in `[0, 1)`.
    pub fn draw(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(0x0808_8405).wrapping_add(1);
        (f64::from(self.0) * f64::from_bits(0x3DF0_0000_0000_0000)) as f32
    }
}

/// Applies a kind 6 lever's fade to the light under it.
fn fade_from_levers(
    mut fades: MessageReader<exa_physics::body::LightFade>,
    mut lights: Query<&mut LightSource>,
) {
    for fade in fades.read() {
        if let Ok(mut source) = lights.get_mut(fade.light) {
            source.set_scale(fade.scale);
        }
    }
}

pub struct LightSourcePlugin;

impl Plugin for LightSourcePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlickerRng>()
            .add_systems(FixedUpdate, fade_from_levers)
            .add_systems(Update, flicker_lights)
            .add_systems(PostUpdate, apply_light_state);
    }
}

/// The parent mesh's first part a light's flicker or lever drives, with the
/// part's own material copy and the emissive colour it started with.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct LightFixture {
    pub part: Entity,
    pub base: Vec4,
}

/// One flicker step for every animating light, at the frame's time.
fn flicker_lights(
    time: Res<Time>,
    mut rng: ResMut<FlickerRng>,
    mut lights: Query<&mut LightSource>,
) {
    let ms = frame_ms(time.delta().as_millis());
    for mut source in &mut lights {
        if !source.animates() {
            continue;
        }
        source.flicker_step(&mut || rng.draw(), ms);
    }
}

/// Moves a changed light's intensity into its point light, and its
/// fixture scale into the parent mesh's first part (only once a lever or
/// the flicker has run: a light at rest leaves its mesh as loaded).
fn apply_light_state(
    mut commands: Commands,
    lights: Query<(Entity, &LightSource, &ChildOf, Option<&LightFixture>), Changed<LightSource>>,
    parents: Query<&crate::node_parts::NodeParts>,
    mut handles: Query<&mut MeshMaterial3d<exa_render::material::ExaMaterial>>,
    mut point: Query<&mut PointLight>,
    mut materials: ResMut<Assets<exa_render::material::ExaMaterial>>,
) {
    for (entity, source, parent, fixture) in &lights {
        if let Ok(mut light) = point.get_mut(entity) {
            light.intensity = source.intensity.max(0.0) * crate::LUMENS_PER_INTENSITY;
        }
        if fixture.is_none() && source.dirty == 0 && source.fixture_scale == 1.0 {
            continue;
        }
        let fixture = match fixture {
            Some(fixture) => fixture.clone(),
            None => {
                let Some(&part) = parents
                    .get(parent.parent())
                    .ok()
                    .and_then(|p| p.parts.first())
                else {
                    continue;
                };
                let Ok(mut handle) = handles.get_mut(part) else {
                    continue;
                };
                let Some(own) = materials.get(&handle.0).cloned() else {
                    continue;
                };
                let base = own.params.emissive;
                handle.0 = materials.add(own);
                let fixture = LightFixture { part, base };
                commands.entity(entity).insert(fixture.clone());
                fixture
            }
        };
        let Ok(handle) = handles.get(fixture.part) else {
            continue;
        };
        if let Some(mut material) = materials.get_mut(&handle.0) {
            material.params.emissive =
                (fixture.base.truncate() * source.fixture_scale).extend(fixture.base.w);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_takes_the_base_intensity_and_the_offset() {
        let mut s = LightSource::new(0, 3.0);
        s.intensity = 0.0;
        s.activate();
        assert_eq!(s.intensity, 3.0);
        assert_eq!(s.shadow_offset, 0);
        for (bits, offset) in [(0, 0), (1, 1), (2, -1), (3, -2)] {
            assert_eq!(shadow_quality_offset(bits), offset);
        }
        assert_eq!(shadow_quality_offset(0xFFFF_FFFE), -1);
    }

    #[test]
    fn the_constructor_defaults_and_the_records_parameters() {
        let s = LightSource::new(0, 2.0);
        assert_eq!((s.base_intensity, s.intensity, s.scale), (2.0, 2.0, 0.0));
        assert_eq!((s.amplitude, s.speed, s.emissive), (0.1, 0.5, 0.5));
        assert!(!s.animates());
        assert!(LightSource::new(FLICKER, 1.0).animates());
        let light = Light {
            flags: FLICKER,
            color: [1.0; 3],
            multiplier: 1.0,
            intensity: 4.0,
            radius: 5.0,
            softness: 0.25,
            spot: None,
            ranged: true,
            animation: Some(exa_formats::rfc::LightAnimation {
                amplitude: 0.3,
                speed: 2.0,
                emissive: 0.75,
            }),
        };
        let s = LightSource::of(&light);
        assert_eq!((s.amplitude, s.speed, s.emissive), (0.3, 2.0, 0.75));
        assert_eq!(s.base_intensity, 4.0);
    }

    /// A kind 6 lever's fade sets its own light's scale and leaves another
    /// light as it was.
    #[test]
    fn a_kind_6_fade_sets_only_the_linked_light() {
        let mut app = App::new();
        app.add_message::<exa_physics::body::LightFade>()
            .add_systems(FixedUpdate, fade_from_levers);
        let linked = app.world_mut().spawn(LightSource::new(0, 4.0)).id();
        let other = app.world_mut().spawn(LightSource::new(0, 4.0)).id();
        app.world_mut().write_message(exa_physics::body::LightFade {
            light: linked,
            scale: 1.0,
        });
        app.world_mut().run_schedule(FixedUpdate);
        let linked = app.world().get::<LightSource>(linked).unwrap();
        let other = app.world().get::<LightSource>(other).unwrap();
        assert_eq!((linked.scale, linked.intensity), (1.0, 4.0));
        assert_eq!((other.scale, other.intensity), (0.0, 4.0));
    }

    #[test]
    fn a_lever_scales_the_intensity_and_the_fixture() {
        let mut s = LightSource::new(0, 4.0);
        assert_eq!(s.set_scale(0.25), 0.125);
        assert_eq!(s.intensity, 1.0);
        assert_eq!(s.dirty & 0xB, 0xB);
        s.set_scale(0.0);
        assert_eq!((s.intensity, s.fixture_scale), (0.0, 0.0));
    }

    #[test]
    fn the_flicker_walk_draws_twice_in_double_precision() {
        let mut s = LightSource::new(FLICKER, 2.0);
        let mut draws = vec![1.0f32, 0.5].into_iter();
        s.flicker_step(&mut || draws.next().unwrap(), 10.0);
        // step = f32(f64(0.5·10)·0.01) = 0.05; the walk adds
        // 2·0.05·(1 − 0.5) = 0.05; the scale moves by 10·0.1·(0.5·0.05).
        let step = (f64::from(0.5f32 * 10.0) * 0.01) as f32;
        let term = 2.0 * step * 0.5;
        assert_eq!(s.flicker, term);
        let scale = (10.0 * 0.1 * f64::from(0.5 * term)) as f32;
        assert_eq!(s.scale, scale);
        let gain = f64::from(scale * 0.1) * 0.1 + 1.0;
        assert_eq!(s.intensity, (gain * 2.0) as f32);
        assert_eq!(s.fixture_scale, scale * 0.5 + 1.0);
        // The flicker path leaves the pending bits alone.
        assert_eq!(s.dirty, 0);
    }

    #[test]
    fn the_walk_drifts_back_inside_the_band_and_clamps() {
        // At 0.9 in double precision the f32 0.9 is below the bound: the
        // walk is taken, not the downward drift.
        let mut s = LightSource::new(FLICKER, 1.0);
        s.scale = 0.9;
        let mut draws = [1.0f32, 0.0].into_iter();
        s.flicker_step(&mut || draws.next().unwrap(), 16.0);
        assert!(s.flicker > 0.0);
        // At 0.95 the term drifts down.
        let mut s = LightSource::new(FLICKER, 1.0);
        s.scale = 0.95;
        let mut draws = [1.0f32, 0.0].into_iter();
        s.flicker_step(&mut || draws.next().unwrap(), 16.0);
        assert!(s.flicker < 0.0);
        assert_eq!(s.scale, 0.95);
        // Below −0.9 it drifts up.
        let mut s = LightSource::new(FLICKER, 1.0);
        s.scale = -0.95;
        let mut draws = [1.0f32, 0.0].into_iter();
        s.flicker_step(&mut || draws.next().unwrap(), 16.0);
        assert!(s.flicker > 0.0);
        // A large term clamps the scale to one.
        let mut s = LightSource::new(FLICKER, 1.0);
        s.flicker = 500.0;
        let mut draws = [0.0f32, 1.0].into_iter();
        s.flicker_step(&mut || draws.next().unwrap(), 16.0);
        assert_eq!(s.scale, 1.0);
        assert_eq!(s.fixture_scale, 1.5);
    }

    #[test]
    fn a_fading_light_draws_nothing_and_clears_its_flag() {
        let mut s = LightSource::new(FADE | FLICKER, 2.0);
        s.flicker = 1.0;
        let mut drawn = 0;
        s.flicker_step(
            &mut || {
                drawn += 1;
                1.0
            },
            10.0,
        );
        assert_eq!(drawn, 0);
        assert_eq!(s.scale, 0.05);
        assert_eq!(s.intensity, 2.0 * 0.05);
        assert_eq!(s.dirty, 0xB);
        s.scale = 0.99;
        s.flicker_step(&mut || 0.0, 10.0);
        assert_eq!(s.intensity, 2.0);
        assert_eq!(s.flags & FLICKER, 0);
        // Falling, it settles at the amplitude.
        let mut t = LightSource::new(FADE | FLICKER, 1.0);
        t.scale = 0.12;
        t.flicker = -1.0;
        t.flicker_step(&mut || 0.0, 10.0);
        assert_eq!(t.intensity, 0.1);
        assert_eq!(t.flags & FLICKER, 0);
        assert_eq!(t.fixture_scale, t.scale * 0.5);
    }

    #[test]
    fn the_frame_time_is_held_to_one_sixtieth_of_a_second() {
        assert_eq!(frame_ms(0), 0.0);
        assert_eq!(frame_ms(16), 16.0);
        assert_eq!(frame_ms(17), FRAME_MS_CAP);
        assert_eq!(frame_ms(250), FRAME_MS_CAP);
    }

    #[test]
    fn the_generator_steps_and_scales_by_two_to_the_minus_32() {
        let mut rng = FlickerRng(0);
        assert_eq!(rng.draw(), (1.0f64 / 4_294_967_296.0) as f32);
        assert_eq!(rng.0, 1);
        assert_eq!(
            rng.draw(),
            (f64::from(0x0808_8406u32) / 4_294_967_296.0) as f32
        );
    }

    fn light(flags: u32, intensity: f32, radius: f32, ranged: bool) -> Light {
        Light {
            flags,
            color: [1.0, 0.5, 0.25],
            multiplier: 1.0,
            intensity,
            radius,
            softness: 0.25,
            spot: None,
            ranged,
            animation: None,
        }
    }

    #[test]
    fn derived_light_values_follow_the_activation() {
        let l = light(1, 4.0, 2.0, true);
        let derived = DerivedLight::of(&l, exa_formats::rfc::RANGED, 4.0);
        let c = exa_render::material::linear_rgb([1.0, 0.5, 0.25]);
        assert_eq!(derived.colour, [c.x * 4.0, c.y * 4.0, c.z * 4.0]);
        assert_eq!(derived.second_colour, derived.colour);
        assert_eq!(derived.gi_colour, derived.colour);
        // √4 · 1 · 3 = 6.
        assert_eq!(derived.inject_scale, 6.0);
        assert_eq!(derived.inject_colour, [c.x * 6.0, c.y * 6.0, c.z * 6.0]);
        assert_eq!(derived.reach, 2.0);
        // 1/2 = 0.5 in single precision, × 0.8 in double = 0.4.
        assert_eq!(derived.attenuation, 0.4);
        assert_eq!(derived.shadow_offset, 1);
        assert_eq!((derived.shaping_y, derived.shaping_z), (1.0, 1.0));
        assert_eq!(derived.luminance, exa_formats::colour::luma(derived.colour));
        // An unranged light reaches the far plane with no falloff.
        let derived = DerivedLight::of(&light(0, 1.0, 0.0, false), 0, 1.0);
        assert_eq!(derived.reach, UNRANGED_FAR);
        assert_eq!(derived.reach, 3.4e38);
        assert_eq!(derived.attenuation, 0.0);
        // A 3 m radius: 1/3 rounds in single precision before the double
        // multiply.
        let derived = DerivedLight::of(&light(0, 1.0, 3.0, true), 0, 1.0);
        assert_eq!(derived.attenuation, (f64::from(1.0f32 / 3.0) * 0.8) as f32);
        // The GI colour carries the multiplier; node flag 0x10000 zeroes it,
        // 0x20000 the second slot. The body's flicker bit does neither.
        let mut l = light(FLICKER, 2.0, 1.0, true);
        l.multiplier = 0.5;
        assert_eq!(DerivedLight::of(&l, 0, 2.0).gi_colour, [c.x, c.y, c.z]);
        assert_eq!(DerivedLight::of(&l, NODE_NO_GI, 2.0).gi_colour, [0.0; 3]);
        let d = DerivedLight::of(&l, NODE_NO_SECOND_COLOUR, 2.0);
        assert_eq!(d.second_colour, [0.0; 3]);
        assert_eq!(d.gi_colour, [c.x, c.y, c.z]);
    }

    #[test]
    fn a_tiny_intensity_becomes_zero_and_a_nan_stays() {
        let l = light(0, 0.0, 1.0, true);
        let d = DerivedLight::of(&l, 0, 1.0e-8);
        assert_eq!(d.intensity, 0.0);
        assert_eq!(d.colour, [0.0; 3]);
        assert_eq!(d.inject_scale, 0.0);
        let d = DerivedLight::of(&l, 0, -2.0);
        assert_eq!(d.intensity, 0.0);
        let d = DerivedLight::of(&l, 0, f32::NAN);
        assert!(d.intensity.is_nan() && d.inject_scale.is_nan());
        // The threshold itself is kept.
        assert_eq!(DerivedLight::of(&l, 0, TINY).intensity, TINY);
    }

    #[test]
    fn an_alias_takes_the_derived_values_and_the_flag_byte() {
        let l = light(0, 2.0, 4.0, true);
        let node = 0x0012_3456 | NODE_FLAG_BIT;
        let derived = DerivedLight::of(&l, node, 2.0);
        let values = AliasValues::of(&derived, node, 0.25);
        assert_eq!(values.node_flags, 0x0012_0000 | NODE_FLAG_BIT);
        assert_eq!(values.alias_word(0xAB00_00CD), 0xAB12_00CD | NODE_FLAG_BIT);
        assert!(values.flag_bit);
        assert_eq!(values.colour, derived.colour);
        assert_eq!(values.gi_colour, derived.gi_colour);
        assert_eq!(values.luminance, derived.luminance);
        assert_eq!(values.attenuation, derived.attenuation);
        assert_eq!(values.reach, 4.0);
        assert_eq!(values.softness, 0.25);
    }

    #[test]
    fn the_derived_colours_scale_by_the_intensity_and_root() {
        let mut s = LightSource::new(0, 4.0);
        s.colour = [1.0, 0.5, 0.25];
        // Legacy: √4 · 0.75 = 1.5 on every channel.
        assert_eq!(s.direct_colour(false), [1.5, 0.75, 0.375]);
        // Linear: linearise each channel first, then times 4.
        let c = exa_render::material::linear_rgb([1.0, 0.5, 0.25]);
        assert_eq!(s.direct_colour(true), [c.x * 4.0, c.y * 4.0, c.z * 4.0]);
        // The node colour: the linearised colour times √(4·0.5) · 0.6 with
        // the 0.6 in double (legacy), or √4 · 0.5 · 3 (linear).
        s.multiplier = 0.5;
        let scale = (f64::from((4.0f32 * 0.5).sqrt()) * 0.6) as f32;
        assert_eq!(
            s.node_colour(false),
            [c.x * scale, c.y * scale, c.z * scale]
        );
        assert_eq!(s.node_colour(true), [c.x * 3.0, c.y * 3.0, c.z * 3.0]);
        // The record's multiplier is loaded with the light.
        let light = Light {
            flags: 0,
            color: [1.0, 0.5, 0.25],
            multiplier: 0.1,
            intensity: 60.0,
            radius: 6.3,
            softness: 0.0,
            spot: None,
            ranged: true,
            animation: None,
        };
        let s = LightSource::of(&light);
        assert_eq!((s.multiplier, s.radius, s.colour), (0.1, 6.3, light.color));
        // As constructed the light is white with multiplier 1.
        let s = LightSource::new(0, 1.0);
        assert_eq!((s.colour, s.multiplier), ([1.0; 3], 1.0));
    }

    #[test]
    fn the_node_flags_black_out_the_second_and_gi_colours() {
        // The gates read the node's flags word, not the body's flicker and
        // fade bits: a flickering body keeps both colours.
        let mut s = LightSource::new(FLICKER | FADE, 1.0);
        let (colour, second) = s.derive(0, false);
        assert_eq!(colour, [0.75; 3]);
        assert_eq!(second, colour);
        assert_eq!(s.offset, colour);
        // Node flag 0x20000: a black second colour and the sum untouched.
        let mut s = LightSource::new(0, 1.0);
        let (_, second) = s.derive(NODE_NO_SECOND_COLOUR, false);
        assert_eq!(second, [0.0; 3]);
        assert_eq!(s.offset, [0.0; 3]);
        // Node flag 0x10000: a black GI colour; otherwise the direct colour
        // times the multiplier.
        s.multiplier = 0.5;
        assert_eq!(s.update(NODE_NO_GI, false).gi, [0.0; 3]);
        assert_eq!(s.update(0, false).gi, [0.375; 3]);
    }

    #[test]
    fn the_update_clamps_the_intensity_and_reports_the_reach() {
        let mut s = LightSource::new(0, 1.0);
        s.colour = [0.5, 0.25, 0.0];
        s.intensity = 1e-9;
        s.radius = 10.0;
        let update = s.update(0, false);
        // Under 2^-23 the intensity is zero, so the direct colour is black.
        assert_eq!(s.intensity, 0.0);
        assert_eq!(update.colour, [0.0; 3]);
        assert_eq!(update.offset, [0.0; 3]);
        assert_eq!(update.luma, 0.0);
        // The reach moves from 0 to 10: over one, so bit 2 and bit 4.
        assert_eq!(update.radius, 10.0);
        assert!(update.radius_changed);
        assert_eq!(s.dirty & 0x6, 0x6);
        assert_eq!(s.last_radius, 10.0);
        // A second update with the same reach sets only bit 4.
        s.dirty = 0;
        assert!(!s.update(0, false).radius_changed);
        assert_eq!(s.dirty, 4);
        // A zero multiplier reports a zero reach even with a radius.
        s.intensity = 1.0;
        s.multiplier = 0.0;
        let update = s.update(0, false);
        assert_eq!(update.radius, 0.0);
        assert!(update.radius_changed);
        // The luma sums red, green, then blue: (x·0.299 + y·0.587) + z·0.114.
        s.multiplier = 1.0;
        s.colour = [1.0, 0.5, 0.25];
        s.radius = 0.5;
        s.last_radius = 0.5;
        let update = s.update(0, true);
        let c = s.direct_colour(true);
        assert_eq!(update.luma, (c[0] * 0.299 + c[1] * 0.587) + c[2] * 0.114);
        // A NaN multiplier takes the radius, as the unordered compare does.
        s.multiplier = f32::NAN;
        assert_eq!(s.update(0, true).radius, 0.5);
        // A NaN intensity is kept by the ordered clamp.
        s.intensity = f32::NAN;
        s.update(0, true);
        assert!(s.intensity.is_nan());
    }
}

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
        }
    }

    /// The light node as loaded: the record's flags and intensity, and its
    /// animation parameters when it stores them.
    pub fn of(light: &Light) -> Self {
        let mut source = Self::new(light.flags, light.intensity);
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

/// Light flag (`0x40000`): the light has a finite reach (otherwise it is
/// treated as unbounded).
pub const RANGED: u32 = 0x4_0000;
/// The falloff's far end, in radii: the shader reaches zero at 1.25 R.
pub const REACH_RADII: f32 = 1.25;
/// The far plane an unranged light uses (the original's `3.4e38`).
pub const UNRANGED_FAR: f32 = 3.4e38;

/// The light values activation derives from a record: the linear colour
/// times the current intensity, its Rec. 601 luminance, the GI's injection
/// radius and the shadow parameters.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DerivedLight {
    /// The linear colour multiplied by the current intensity (the shader's
    /// direct colour source).
    pub colour: [f32; 3],
    /// The colour the voxel GI injects (`+0x1C8`): the direct colour times
    /// the record's multiplier, zero for a light with the flicker flag.
    pub gi_colour: [f32; 3],
    /// The light node's `+0x278` colour: the linear sRGB colour times
    /// `√intensity · multiplier · 3`.
    pub inject_colour: [f32; 3],
    /// The scale of [`DerivedLight::inject_colour`].
    pub inject_scale: f32,
    /// Rec. 601 luminance of [`DerivedLight::colour`].
    pub luminance: f32,
    /// The point light's falloff end: 1.25 radii when ranged, else the
    /// unranged far plane.
    pub reach: f32,
    /// The shadow filter's strength: `0.8 / radius` when ranged with a
    /// positive radius, else 0.
    pub shadow_strength: f32,
    /// The light's shadow-quality offset (`0`, `+1`, `−1` or `−2`).
    pub shadow_offset: i32,
    /// The vertical shaping's `y` when the light has a spot, else 1.
    pub shaping_y: f32,
    /// The vertical shaping's `z` when the light has a spot, else 1.
    pub shaping_z: f32,
}

/// Rec. 601 luminance in double precision, summed as the converter does:
/// `b·0.114 + (g·0.587 + r·0.299)`.
pub fn luminance(rgb: [f32; 3]) -> f32 {
    let sum = f64::from(rgb[1]) * 0.587 + f64::from(rgb[0]) * 0.299;
    (f64::from(rgb[2]) * 0.114 + sum) as f32
}

impl DerivedLight {
    /// Derives a light's render values from its record and current
    /// intensity.
    pub fn of(light: &Light, intensity: f32) -> Self {
        let linear = exa_render::material::linear_rgb(light.color);
        let colour = [
            linear.x * intensity,
            linear.y * intensity,
            linear.z * intensity,
        ];
        let (shaping_y, shaping_z) = crate::vertical_shaping(light);
        let ranged = light.flags & RANGED != 0 || light.ranged;
        let inject_scale = intensity.max(0.0).sqrt() * light.multiplier * 3.0;
        let gi = if light.flags & FLICKER != 0 {
            [0.0; 3]
        } else {
            [
                colour[0] * light.multiplier,
                colour[1] * light.multiplier,
                colour[2] * light.multiplier,
            ]
        };
        Self {
            colour,
            gi_colour: gi,
            inject_colour: [
                linear.x * inject_scale,
                linear.y * inject_scale,
                linear.z * inject_scale,
            ],
            inject_scale,
            luminance: luminance(colour),
            reach: if ranged {
                light.radius * REACH_RADII
            } else {
                UNRANGED_FAR
            },
            shadow_strength: if ranged && light.radius > 0.0 {
                ((1.0 / light.radius) as f64 * 0.8) as f32
            } else {
                0.0
            },
            shadow_offset: shadow_quality_offset(light.flags),
            shaping_y,
            shaping_z,
        }
    }
}

/// The values copied to a light's aliases: the record's flags, colour,
/// current intensity, radius, softness, spot, ranged bit, scale and the
/// shadow-quality offset.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AliasValues {
    pub flags: u32,
    pub colour: [f32; 3],
    pub intensity: f32,
    pub radius: f32,
    pub softness: f32,
    pub spot: Option<[f32; 2]>,
    pub ranged: bool,
    pub scale: f32,
    pub shadow_offset: i32,
}

impl AliasValues {
    /// The alias copy of a light with its current state.
    pub fn of(light: &Light, source: &LightSource) -> Self {
        Self {
            flags: light.flags,
            colour: light.color,
            intensity: source.intensity,
            radius: light.radius,
            softness: light.softness,
            spot: light.spot,
            ranged: light.ranged,
            scale: source.scale,
            shadow_offset: source.shadow_offset,
        }
    }

    /// Copies the light's values onto every alias. The flags keep their low
    /// byte (the original copies only the top byte) and the scale and spot
    /// follow. Returns how many aliases were written.
    pub fn sync(&self, aliases: &mut [AliasValues]) -> usize {
        for alias in aliases.iter_mut() {
            alias.flags = self.flags & 0xFF00_0000 | alias.flags & 0x00FF_FFFF;
            alias.colour = self.colour;
            alias.intensity = self.intensity;
            alias.radius = self.radius;
            alias.softness = self.softness;
            alias.spot = self.spot;
            alias.ranged = self.ranged;
            alias.scale = self.scale;
            alias.shadow_offset = self.shadow_offset;
        }
        aliases.len()
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

pub struct LightSourcePlugin;

impl Plugin for LightSourcePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlickerRng>()
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
    fn derived_light_values_follow_the_modern_gi_and_shadow_rules() {
        let l = light(1, 4.0, 2.0, true);
        let derived = DerivedLight::of(&l, 4.0);
        let c = exa_render::material::linear_rgb([1.0, 0.5, 0.25]);
        assert_eq!(derived.colour, [c.x * 4.0, c.y * 4.0, c.z * 4.0]);
        assert_eq!(derived.gi_colour, derived.colour);
        assert_eq!(derived.inject_scale, 4.0f32.sqrt() * 3.0);
        assert_eq!(
            derived.inject_colour,
            [
                c.x * derived.inject_scale,
                c.y * derived.inject_scale,
                c.z * derived.inject_scale
            ]
        );
        assert_eq!(derived.reach, 2.5);
        assert_eq!(derived.shadow_strength, 0.4);
        assert_eq!(derived.shadow_offset, 1);
        assert_eq!((derived.shaping_y, derived.shaping_z), (1.0, 1.0));
        let expected = (f64::from(derived.colour[2]) * 0.114
            + (f64::from(derived.colour[1]) * 0.587 + f64::from(derived.colour[0]) * 0.299))
            as f32;
        assert_eq!(derived.luminance, expected);
        // An unranged light reaches the far plane and has no shadow
        // strength; a zero radius does not divide.
        let l = light(0, 1.0, 0.0, false);
        let derived = DerivedLight::of(&l, 1.0);
        assert_eq!(derived.reach, UNRANGED_FAR);
        assert_eq!(derived.shadow_strength, 0.0);
        // The GI colour carries the multiplier; a flicker flag zeroes it.
        let mut l = light(0, 2.0, 1.0, true);
        l.multiplier = 0.5;
        assert_eq!(DerivedLight::of(&l, 2.0).gi_colour, [c.x, c.y, c.z]);
        l.flags = FLICKER;
        assert_eq!(DerivedLight::of(&l, 2.0).gi_colour, [0.0; 3]);
    }

    #[test]
    fn alias_sync_copies_the_light_and_keeps_the_alias_low_flag_byte() {
        let mut source = LightSource::new(1, 2.0);
        source.activate();
        source.set_scale(0.5);
        let l = light(0x0100_0001, 2.0, 4.0, true);
        let values = AliasValues::of(&l, &source);
        let mut aliases = [AliasValues {
            flags: 0x0000_00AB,
            ..values
        }];
        assert_eq!(values.sync(&mut aliases), 1);
        assert_eq!(aliases[0].flags, 0x0100_00AB);
        assert_eq!(aliases[0].intensity, 1.0);
        assert_eq!(aliases[0].scale, 0.5);
        assert_eq!(aliases[0].shadow_offset, 1);
        assert_eq!(aliases[0].colour, [1.0, 0.5, 0.25]);
    }
}

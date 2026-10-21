//! Light-source nodes: the fields the original keeps beside a light's
//! colour and radius, and what a lever or the flicker timer does to them.
//!
//! A light node starts with its base intensity (`+0x2c8`), a copy of it as
//! the current intensity, and the flicker state the constructor gives it:
//! depth 0.5, rate 0.1, lever emissive 0.5. Activation copies the base
//! intensity into the current one and sets the light's shadow-quality
//! offset from the low two flag bits (0, +1, −1, −2).
//!
//! **Lever** (`0x1000525c0`): sets the intensity scale, `base × scale`; a
//! light whose parent is a drawable node also scales its first sub-object's
//! material emissive by `scale × lever-emissive` and the node's emissive
//! scale to the same. It marks the light dirty and recomputes.
//!
//! **Flicker** (`0x100052710`): without flag `0x20000` it random-walks a
//! flicker term and a scale, then sets the intensity to
//! `(scale·rate·0.1 + 1)·base` and the fixture's emissive to
//! `scale·lever-emissive + 1`. With the flag it fades: the scale moves
//! towards the rate (when the flicker term is at or below zero) or towards
//! one (when above), and the flag clears once it arrives. The random draws
//! are in the original's order: one for the walk, one for the return, from
//! the engine's uniform generator.
//!
//! **Aliases** (`0x1000521f0`): a light registered in other sectors keeps a
//! copy per sector; syncing copies the flags' high byte, the frame, the
//! derived colours, the fade amount, the softness and the shaping word.
//!
//! The derived colours themselves are built by `crate::light_bundle`; this
//! module keeps the state those copies read and the two timer paths.
//! Rules: `docs/RENDERING.md` (pass order and lights), `docs/formats/RFC.md`
//! (light node).

use bevy::prelude::*;

use crate::particles::{Draws, Rng};
use exa_formats::rfc::Light;

/// Light flag (`0x10000`): the light flickers.
pub const FLICKER: u32 = 0x1_0000;
/// Light flag (`0x20000`): the light fades instead of flickering; cleared
/// when the fade arrives.
pub const FADE: u32 = 0x2_0000;
/// The constructor's flicker depth (`+0x2cc`).
pub const DEFAULT_DEPTH: f32 = 0.5;
/// The constructor's flicker rate (`+0x2d0`).
pub const DEFAULT_RATE: f32 = 0.1;
/// The constructor's lever emissive scale (`+0x2d4`).
pub const DEFAULT_LEVER_EMISSIVE: f32 = 0.5;
/// The scene's flicker scale (`+0x1dd4` of the scene environment in the
/// original). Its value is not traced; a level's environment may scale it.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct FlickerScale(pub f32);

impl Default for FlickerScale {
    fn default() -> Self {
        Self(1.0)
    }
}

/// The state a light node keeps beside its colour and radius.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct LightSource {
    /// The light flags (`+0x20c`): [`FLICKER`], [`FADE`] and the shadow
    /// quality's two low bits.
    pub flags: u32,
    /// The stored intensity (`+0x2c8`), which a lever scales.
    pub base_intensity: f32,
    /// The current intensity (`+0x220`).
    pub intensity: f32,
    /// The intensity scale (`+0x2d8`): the lever's, moved by the flicker.
    pub scale: f32,
    /// The lever's emissive scale (`+0x2d4`).
    pub lever_emissive: f32,
    /// The flicker depth (`+0x2cc`) and rate (`+0x2d0`).
    pub flicker_depth: f32,
    pub flicker_rate: f32,
    /// The flicker term (`+0x2dc`).
    pub flicker: f32,
    /// The shadow-quality offset (`+0x23c`): 0, 1, −1 or −2.
    pub shadow_offset: i32,
    /// The node's emissive scale (`+0x244`), which a lever or flicker sets
    /// on its fixture.
    pub emissive_scale: f32,
    /// The pending-change word (`+0x268`); bit `0x2` means the radius
    /// changed, bit `0x4` that the light needs a recompute.
    pub dirty: u32,
    /// The light's record colour (sRGB), softness and radius.
    pub colour: [f32; 3],
    pub softness: f32,
    pub radius: f32,
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
            lever_emissive: DEFAULT_LEVER_EMISSIVE,
            flicker_depth: DEFAULT_DEPTH,
            flicker_rate: DEFAULT_RATE,
            flicker: 0.0,
            shadow_offset: 0,
            emissive_scale: 1.0,
            dirty: 0,
            colour: [1.0; 3],
            softness: 0.25,
            radius: 1.0,
        }
    }

    /// The light node as loaded: base intensity from the record.
    pub fn of(light: &Light) -> Self {
        Self {
            colour: light.color,
            softness: light.softness,
            radius: light.radius,
            ..Self::new(light.flags, light.intensity)
        }
    }

    /// Activation: the current intensity becomes the base, the
    /// shadow-quality offset comes from the flags' low two bits, and the
    /// pending-change word clears.
    pub fn activate(&mut self) {
        self.intensity = self.base_intensity;
        self.shadow_offset = shadow_quality_offset(self.flags);
        self.dirty = 0;
    }

    /// Whether the light flickers or fades.
    pub fn animates(&self) -> bool {
        self.flags & (FLICKER | FADE) != 0
    }

    /// A lever's intensity scale: `intensity = base × scale`; a fixture's
    /// emissive scale becomes `scale × lever-emissive`. Returns the emissive
    /// scale (the original's `+0xf4` write).
    pub fn set_scale(&mut self, scale: f32) -> f32 {
        self.scale = scale;
        self.intensity = self.base_intensity * self.scale;
        self.emissive_scale = self.scale * self.lever_emissive;
        self.dirty |= 0xB;
        self.emissive_scale
    }

    /// One flicker tick, drawing from `random` (`FnMut` returning the
    /// engine's uniform `[0, 1)`) at the scene's flicker scale. Returns
    /// whether the light changed. The two random draws are made in the
    /// original's order, and only on the flicker path.
    // The ordered compares are the original's: a NaN scale keeps its value
    // (clamp would return NaN or panic).
    #[allow(clippy::manual_clamp)]
    pub fn flicker_step(&mut self, random: &mut impl FnMut() -> f32, scene: f32) -> bool {
        if self.flags & FADE == 0 {
            let step = self.flicker_depth * scene * 0.01;
            if self.scale < 0.9 {
                if -0.9 < self.scale {
                    let r = random();
                    self.flicker += step * 2.0 * (r - 0.5);
                } else {
                    let r = random();
                    self.flicker += r * step;
                }
            } else {
                let r = random();
                self.flicker -= r * step;
            }
            let r = random();
            let mut scale = self.scale + scene * 0.1 * r * self.flicker;
            if scale > 1.0 {
                scale = 1.0;
            } else if scale < -1.0 {
                scale = -1.0;
            }
            self.scale = scale;
            self.emissive_scale = self.scale * self.lever_emissive + 1.0;
            self.intensity = (self.scale * self.flicker_rate * 0.1 + 1.0) * self.base_intensity;
            self.dirty |= 0xB;
            true
        } else {
            let step = self.flicker_depth * scene * 0.01;
            if self.flicker <= 0.0 {
                let scale = self.scale - step;
                self.scale = scale;
                if self.flicker_rate <= scale && scale != self.flicker_rate {
                    self.intensity = self.base_intensity * self.scale;
                } else {
                    self.intensity = self.flicker_rate * self.base_intensity;
                    self.flags &= !FLICKER;
                }
            } else {
                let scale = self.flicker_depth * scene * 0.01 + self.scale;
                self.scale = scale;
                if scale < 1.0 {
                    self.intensity = self.base_intensity * self.scale;
                } else {
                    self.intensity = self.base_intensity;
                    self.flags &= !FLICKER;
                }
            }
            self.emissive_scale = self.scale * self.lever_emissive;
            self.dirty |= 0xB;
            true
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

/// A light's shadow detail after its offset: the scene's setting plus the
/// light's offset, held to the engine's 1..=5 range.
pub fn shadow_quality_level(quality: i32, offset: i32) -> i32 {
    (quality + offset).clamp(1, 5)
}

/// The frame and derived light a per-sector alias carries.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct LightAlias {
    /// The source light node.
    pub source: Entity,
    /// The copied flags' high byte, replacing the alias's own.
    pub flags_high: u32,
    /// The source's frame.
    pub world: Mat4,
    /// The derived colours: the linear source colour × intensity (`+0x1d8`),
    /// the range colour (`+0x1c8`) and the fade colour (`+0x1e8`).
    pub colour: [f32; 3],
    pub range: [f32; 3],
    pub fade: [f32; 3],
    /// The luminance (`+0x208`) and fade amount (`+0x1f8`).
    pub luminance: f32,
    pub fade_amount: f32,
    /// The softness (`+0x22c`) and the shaping word (`+0x1d8` of the
    /// material-side copy).
    pub softness: f32,
    pub shaping: f32,
    /// The light's bounding radius (`+0x2c`).
    pub radius: f32,
    /// The shadow-quality offset (`+0x2c8` area) and the light's flags.
    pub shadow_offset: i32,
    pub animates: bool,
}

impl LightAlias {
    /// A copy of `source`'s state. `colour` is the linear source colour
    /// times its intensity; `range` and `fade` are the derived colours the
    /// source computes; `world` is the source node's world frame.
    #[allow(clippy::too_many_arguments)]
    pub fn copy_of(
        source: Entity,
        source_flags: u32,
        world: Mat4,
        colour: Vec3,
        range: Vec3,
        fade: Vec3,
        luminance: f32,
        fade_amount: f32,
        softness: f32,
        shaping: f32,
        radius: f32,
        shadow_offset: i32,
    ) -> Self {
        Self {
            source,
            flags_high: source_flags & 0xFF00_0000,
            world,
            colour: colour.to_array(),
            range: range.to_array(),
            fade: fade.to_array(),
            luminance,
            fade_amount,
            softness,
            shaping,
            radius,
            shadow_offset,
            animates: source_flags & (FLICKER | FADE) != 0,
        }
    }
}

/// The generated random stream of the flicker timers.
#[derive(Resource)]
pub struct FlickerRng(pub Rng);

impl Default for FlickerRng {
    fn default() -> Self {
        Self(Rng::new(0x5EED_1234))
    }
}

pub struct LightSourcePlugin;

impl Plugin for LightSourcePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FlickerScale>()
            .init_resource::<FlickerRng>()
            .add_systems(Update, flicker_lights)
            .add_systems(
                PostUpdate,
                (prepare_fixtures, apply_light_state, sync_aliases).chain(),
            );
    }
}

/// Gives every new light the emissive parts of its parent as its fixture.
fn prepare_fixtures(
    mut commands: Commands,
    new: Query<(Entity, &ChildOf), Added<LightSource>>,
    children: Query<&Children>,
    parts: Query<&MeshMaterial3d<exa_render::material::ExaMaterial>>,
    assets: Res<Assets<exa_render::material::ExaMaterial>>,
) {
    for (light, parent) in &new {
        let mut fixture = Vec::new();
        if let Ok(list) = children.get(parent.parent()) {
            for &child in list {
                let Ok(handle) = parts.get(child) else {
                    continue;
                };
                let Some(material) = assets.get(handle) else {
                    continue;
                };
                if material.params.emissive.truncate() != Vec3::ZERO {
                    fixture.push((child, material.params.emissive));
                }
            }
        }
        if !fixture.is_empty() {
            commands.entity(light).insert(LightFixture(fixture));
        }
    }
}

/// The parts a light's lever or flicker drives: its parent's direct
/// children with a glowing material, and the material's base emissive.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct LightFixture(pub Vec<(Entity, Vec4)>);

/// Moves a light's current intensity into its point light.
fn apply_light_state(
    lights: Query<(Entity, &LightSource), Changed<LightSource>>,
    fixtures: Query<&LightFixture>,
    materials: Query<&MeshMaterial3d<exa_render::material::ExaMaterial>>,
    mut point: Query<&mut PointLight>,
    mut assets: ResMut<Assets<exa_render::material::ExaMaterial>>,
) {
    for (entity, source) in &lights {
        if let Ok(mut light) = point.get_mut(entity) {
            light.intensity = source.intensity.max(0.0) * crate::LUMENS_PER_INTENSITY;
        }
        let Ok(fixture) = fixtures.get(entity) else {
            continue;
        };
        let scale = source.emissive_scale;
        for &(part, base) in &fixture.0 {
            let Ok(handle) = materials.get(part) else {
                continue;
            };
            materials::set_emissive(&mut assets, handle, base.truncate() * scale, base.w);
        }
    }
}

/// Sets a material's emissive colour and alpha in place.
mod materials {
    use super::*;

    pub fn set_emissive(
        assets: &mut Assets<exa_render::material::ExaMaterial>,
        handle: &MeshMaterial3d<exa_render::material::ExaMaterial>,
        rgb: Vec3,
        alpha: f32,
    ) {
        if let Some(mut material) = assets.get_mut(handle) {
            material.params.emissive = rgb.extend(alpha);
        }
    }
}

/// One flicker tick for every animating light.
fn flicker_lights(
    scale: Res<FlickerScale>,
    mut rng: ResMut<FlickerRng>,
    mut lights: Query<&mut LightSource>,
) {
    for mut source in &mut lights {
        if !source.animates() {
            continue;
        }
        let mut random = || rng.0.unit() as f32;
        source.flicker_step(&mut random, scale.0);
    }
}

/// Copies a light's state to its per-sector aliases.
fn sync_aliases(
    sources: Query<(Entity, &LightSource, &GlobalTransform), Changed<LightSource>>,
    mut aliases: Query<&mut LightAlias>,
) {
    for (entity, source, at) in &sources {
        let linear = exa_render::material::linear_rgb(source.colour) * source.intensity.max(0.0);
        let fade = if source.flags & FADE != 0 {
            Vec3::ZERO
        } else {
            linear
        };
        let luminance = 0.299 * linear.x + 0.587 * linear.y + 0.114 * linear.z;
        for mut alias in &mut aliases {
            if alias.source != entity {
                continue;
            }
            alias.flags_high = source.flags & 0xFF00_0000;
            alias.world = at.to_matrix();
            alias.colour = linear.to_array();
            alias.range = linear.to_array();
            alias.fade = fade.to_array();
            alias.luminance = luminance;
            alias.fade_amount = if source.flags & FADE != 0 { 1.0 } else { 0.0 };
            alias.softness = source.softness;
            alias.shadow_offset = source.shadow_offset;
            alias.animates = source.animates();
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
        assert_eq!(shadow_quality_level(4, 1), 5);
        assert_eq!(shadow_quality_level(4, -2), 2);
        assert_eq!(shadow_quality_level(5, 1), 5);
        assert_eq!(shadow_quality_level(1, -2), 1);
    }

    #[test]
    fn the_constructor_defaults_are_the_originals() {
        let s = LightSource::new(0, 2.0);
        assert_eq!((s.base_intensity, s.intensity, s.scale), (2.0, 2.0, 0.0));
        assert_eq!(s.flicker_depth, 0.5);
        assert_eq!(s.flicker_rate, 0.1);
        assert_eq!(s.lever_emissive, 0.5);
        assert_eq!(s.emissive_scale, 1.0);
        assert!(!s.animates());
        assert!(LightSource::new(FLICKER, 1.0).animates());
    }

    #[test]
    fn a_lever_scales_the_intensity_and_the_fixture() {
        let mut s = LightSource::new(0, 4.0);
        assert_eq!(s.set_scale(0.25), 0.125);
        assert_eq!(s.intensity, 1.0);
        assert_eq!(s.emissive_scale, 0.125);
        assert_eq!(s.dirty & 0xB, 0xB);
        s.set_scale(0.0);
        assert_eq!(s.intensity, 0.0);
        assert_eq!(s.emissive_scale, 0.0);
    }

    #[test]
    fn the_flicker_walk_draws_twice_and_clamps() {
        let mut s = LightSource::new(FLICKER, 2.0);
        let mut draws = vec![1.0f32, 0.5].into_iter();
        let changed = s.flicker_step(&mut || draws.next().unwrap_or(0.5), 1.0);
        assert!(changed);
        // step = 0.5·1·0.01 = 0.005; first draw 1.0 adds 0.005·2·0.5 =
        // 0.005; second draw 0.5 adds 1·0.1·0.5·0.005 = 0.00025.
        assert!((s.flicker - 0.005).abs() < 1e-6);
        assert!((s.scale - 0.000_25).abs() < 1e-6);
        assert!((s.intensity - 2.0 * (1.0 + 0.000_25 * 0.1 * 0.1)).abs() < 1e-6);
        assert!((s.emissive_scale - (0.000_25 * 0.5 + 1.0)).abs() < 1e-6);
        assert_eq!(s.dirty & 0xB, 0xB);
    }

    #[test]
    fn a_flicker_above_the_band_walks_down_and_clamps_to_one() {
        let mut s = LightSource::new(FLICKER, 1.0);
        s.scale = 0.95;
        let mut draws = [1.0f32, 1.0].into_iter();
        s.flicker_step(&mut || draws.next().unwrap_or(0.0), 1.0);
        // The walk subtracts 0.005 from the term, then the return adds
        // 1·0.1·1·0.005 = 0.0005 to the scale: still above the band, not
        // clamped.
        assert!((s.flicker + 0.005).abs() < 1e-6);
        assert!((s.scale - 0.9495).abs() < 1e-6);
        s.scale = 500.0;
        s.flicker = 500.0;
        let mut draws = [0.0f32, 1.0].into_iter();
        s.flicker_step(&mut || draws.next().unwrap_or(0.0), 1.0);
        assert_eq!(s.scale, 1.0);
    }

    #[test]
    fn a_fading_light_draws_nothing_and_clears_its_flag() {
        let mut s = LightSource::new(FADE | FLICKER, 2.0);
        s.scale = 0.0;
        s.flicker = 1.0;
        let mut draws = Vec::new();
        s.flicker_step(
            &mut || {
                draws.push(1.0);
                1.0
            },
            1.0,
        );
        assert!(draws.is_empty());
        assert!((s.scale - 0.005).abs() < 1e-6);
        s.scale = 1.1;
        s.flicker_step(&mut || 0.0, 1.0);
        assert_eq!(s.intensity, 2.0);
        assert_eq!(s.flags & FLICKER, 0);
        // The other direction: the scale falls toward the rate, and the
        // flag clears when it arrives.
        let mut t = LightSource::new(FADE | FLICKER, 1.0);
        t.scale = 0.1;
        t.flicker = -1.0;
        t.flicker_step(&mut || 0.0, 1.0);
        assert_eq!(t.intensity, 0.1 * 1.0);
        assert_eq!(t.flags & FLICKER, 0);
    }

    #[test]
    fn an_alias_copies_the_flags_high_byte_and_the_frame() {
        let mut a = LightAlias::copy_of(
            Entity::from_raw_u32(7).unwrap(),
            0xAB10_0000,
            Mat4::IDENTITY,
            Vec3::new(1.0, 2.0, 3.0),
            Vec3::new(0.1, 0.2, 0.3),
            Vec3::new(0.0, 0.0, 0.0),
            1.5,
            1.0,
            0.25,
            1.0,
            12.0,
            0,
        );
        assert_eq!(a.flags_high, 0xAB00_0000);
        assert_eq!(a.colour, [1.0, 2.0, 3.0]);
        assert_eq!(a.softness, 0.25);
        assert!(!a.animates);
        // The high byte replaces the alias's own while its low bits stay.
        let own = 0x0012_3456u32;
        let merged = (own & 0x00FF_FFFF) | a.flags_high;
        assert_eq!(merged, 0xAB12_3456);
        a.animates = true;
        assert!(a.animates);
    }
}

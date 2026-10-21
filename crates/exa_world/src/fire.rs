//! Fires: the original's fire presets, which turn a fire marker (a torch's
//! `_Fire`, a brazier's flame, a burning item) into particle emitters and
//! the light they drive.
//!
//! A fire's size picks a tier: the small template up to 0.35, a
//! medium-small pair up to 0.5, a medium pair up to 1, and above that a
//! large pair built at half the size. Flames above the small tier also get
//! a second, offset `_FireBas` emitter for the fire's bed. The templates,
//! their colour keys and their lights are the original's records; the keys
//! here are the linear pipeline's, with the RGB of each table's first `n`
//! entries scaled by 0.7 at load (rounded half to even; the alpha is
//! untouched).
//! Rules: `docs/RENDERING.md` (Fires).

use bevy::prelude::*;

use crate::particles::{Descriptor, Emitter, Key, Keys, LightDesc, Rng, Source, TICK_HZ};

pub struct FirePlugin;

impl Plugin for FirePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, flicker);
    }
}

/// The small template's light colour (sRGB); the larger templates' differ
/// only slightly.
const TORCH_LIGHT: [f32; 3] = [1.0, 0.68, 0.43];
/// A fire light's shadow softness before its jitter.
const SOFTNESS: f32 = 2.0;

/// Fires up to this size use the small template, and above the second its
/// radius grows by a quarter (both compared in double precision).
const SMALL_UP_TO: f64 = 0.35;
const SMALL_WIDENED_FROM: f64 = 0.2;
const MEDIUM_SMALL_UP_TO: f32 = 0.5;
const MEDIUM_UP_TO: f32 = 1.0;

/// The tables before the load-time RGB scaling, for the tests.
#[cfg(test)]
const RAW_SMALL_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [32, 13, 3, 16],
    [64, 23, 5, 32],
    [192, 127, 67, 32],
    [255, 160, 94, 64],
    [170, 120, 60, 64],
    [90, 30, 7, 64],
    [48, 12, 3, 80],
    [0, 0, 0, 48],
    [0, 0, 0, 0],
];
#[cfg(test)]
const RAW_MEDIUM_SMALL_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [64, 33, 10, 16],
    [128, 65, 20, 32],
    [200, 140, 80, 64],
    [140, 70, 30, 64],
    [120, 60, 12, 80],
    [68, 24, 0, 80],
    [34, 5, 0, 32],
    [0, 0, 0, 8],
    [0, 0, 0, 0],
];
#[cfg(test)]
const RAW_MEDIUM_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [64, 23, 15, 32],
    [100, 60, 42, 32],
    [120, 70, 30, 64],
    [98, 50, 15, 64],
    [80, 20, 5, 64],
    [48, 10, 0, 80],
    [8, 0, 0, 48],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
];
#[cfg(test)]
const RAW_LARGE_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [120, 75, 32, 64],
    [125, 60, 6, 80],
    [64, 38, 15, 80],
    [40, 15, 5, 64],
    [16, 2, 0, 32],
    [0, 0, 0, 16],
    [0, 0, 0, 0],
];
#[cfg(test)]
const RAW_BASE_KEYS: &[Key] = &[[0, 0, 0, 0], [180, 100, 50, 0], [0, 0, 0, 0], [0, 0, 0, 0]];

/// The small fire's colour keys. The first two and the last keys are black
/// with no opacity.
const SMALL_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [22, 9, 2, 16],
    [45, 16, 4, 32],
    [134, 89, 47, 32],
    [178, 112, 66, 64],
    [119, 84, 42, 64],
    [63, 21, 5, 64],
    [34, 8, 2, 80],
    [0, 0, 0, 48],
    [0, 0, 0, 0],
];
/// The 0.35–0.5 flame's keys.
const MEDIUM_SMALL_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [45, 23, 7, 16],
    [90, 46, 14, 32],
    [140, 98, 56, 64],
    [98, 49, 21, 64],
    [84, 42, 8, 80],
    [48, 17, 0, 80],
    [24, 4, 0, 32],
    [0, 0, 0, 8],
    [0, 0, 0, 0],
];
/// The 0.5–1 flame's keys.
const MEDIUM_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [45, 16, 10, 32],
    [70, 42, 29, 32],
    [84, 49, 21, 64],
    [69, 35, 10, 64],
    [56, 14, 4, 64],
    [34, 7, 0, 80],
    [6, 0, 0, 48],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
];
/// The large flame's keys.
const LARGE_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [84, 52, 22, 64],
    [88, 42, 4, 80],
    [45, 27, 10, 80],
    [28, 10, 4, 64],
    [11, 1, 0, 32],
    [0, 0, 0, 16],
    [0, 0, 0, 0],
];
/// The 0.35–0.5 bed's keys (three, so five entries): no alpha, so the bed
/// adds light without dimming what is behind it, brightest two thirds of
/// the way through its life. This table is not part of the load-time RGB
/// scaling.
const SMALL_BASE_KEYS: &[Key] = &[
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [200, 120, 50, 0],
    [0, 0, 0, 0],
    [0, 0, 0, 0],
];
/// The 0.5–1 and large beds' keys (two, so four entries), additive in the
/// same way and brightest half way through their life.
const BASE_KEYS: &[Key] = &[[0, 0, 0, 0], [126, 70, 35, 0], [0, 0, 0, 0], [0, 0, 0, 0]];

/// The small fire's template: a torch or a burning item (size 0.2).
pub const SMALL: Descriptor = Descriptor {
    rate: 250,
    life: 1.25,
    half_size: 10.0,
    speed: 0.0,
    retention: 0.0,
    growth: 2.0,
    stretch: 0.6,
    spawn_radius: 35.0,
    spread: 0.0,
    buoyancy: -1.25,
    centring: 0.0007,
    pool: 0,
    flags: 0x420C_0B01,
    keys: Keys {
        count: 9,
        entries: SMALL_KEYS,
    },
    light: LightDesc {
        colour: TORCH_LIGHT,
        intensity: 8.0,
        radius: 1500.0,
        flicker: 1.0,
    },
    sound: Some("FireS02"),
    volume: 0.0,
    density: 450,
    dose: 0.0,
};

/// The 0.35–0.5 flame.
const MEDIUM_SMALL: Descriptor = Descriptor {
    rate: 40,
    life: 0.7,
    half_size: 18.0,
    speed: 10.0,
    retention: 0.0,
    growth: 2.0,
    stretch: 0.0,
    spawn_radius: 15.0,
    spread: 0.0,
    buoyancy: -0.5,
    centring: 0.0005,
    pool: 0,
    flags: 0x1000_0B01,
    keys: Keys {
        count: 9,
        entries: MEDIUM_SMALL_KEYS,
    },
    light: LightDesc {
        colour: [1.0, 0.7, 0.46],
        intensity: 8.0,
        radius: 1500.0,
        flicker: 1.0,
    },
    sound: Some("FireS01"),
    volume: 0.5,
    density: 0,
    dose: 0.2,
};

/// The 0.35–0.5 bed.
const MEDIUM_SMALL_BASE: Descriptor = Descriptor {
    rate: 100,
    life: 0.5,
    half_size: 16.0,
    speed: 0.0,
    retention: 0.5,
    growth: 1.0,
    stretch: 0.0,
    spawn_radius: 100.0,
    spread: 0.0,
    buoyancy: -0.05,
    centring: 0.001,
    pool: 0,
    flags: 0x1000_0A00,
    keys: Keys {
        count: 3,
        entries: SMALL_BASE_KEYS,
    },
    light: LightDesc::NONE,
    sound: None,
    volume: 0.0,
    density: 0,
    dose: 0.0,
};

/// The 0.5–1 flame.
const MEDIUM: Descriptor = Descriptor {
    rate: 80,
    life: 0.65,
    half_size: 30.0,
    speed: 0.0,
    retention: 0.0,
    growth: 1.0,
    stretch: 0.75,
    spawn_radius: 30.0,
    spread: 0.0,
    buoyancy: -1.5,
    centring: 0.0002,
    pool: 0,
    flags: 0x4000_0A05,
    keys: Keys {
        count: 9,
        entries: MEDIUM_KEYS,
    },
    light: LightDesc {
        colour: [1.0, 0.7, 0.45],
        intensity: 12.0,
        radius: 1600.0,
        flicker: 1.0,
    },
    sound: Some("FireS02"),
    volume: 0.1,
    density: 0,
    dose: 1.0,
};

/// The 0.5–1 bed.
const MEDIUM_BASE: Descriptor = Descriptor {
    rate: 80,
    life: 0.5,
    half_size: 25.0,
    speed: 0.0,
    retention: 0.5,
    growth: 1.0,
    stretch: 0.0,
    spawn_radius: 45.0,
    spread: 0.0,
    buoyancy: -0.33,
    centring: 0.0,
    pool: 0,
    flags: 0x1000_0A00,
    keys: Keys {
        count: 2,
        entries: BASE_KEYS,
    },
    light: LightDesc::NONE,
    sound: None,
    volume: 0.0,
    density: 0,
    dose: 0.0,
};

/// The large flame (used at half the fire's size).
const LARGE: Descriptor = Descriptor {
    rate: 70,
    life: 0.6,
    half_size: 50.0,
    speed: 0.0,
    retention: 1.0,
    growth: 1.0,
    stretch: 0.6,
    spawn_radius: 45.0,
    spread: 0.0,
    buoyancy: -1.5,
    centring: 0.0005,
    pool: 0,
    flags: 0x4000_0B05,
    keys: Keys {
        count: 7,
        entries: LARGE_KEYS,
    },
    light: LightDesc {
        colour: TORCH_LIGHT,
        intensity: 12.0,
        radius: 1500.0,
        flicker: 1.0,
    },
    sound: Some("FireL01"),
    volume: 0.6,
    density: 0,
    dose: 1.0,
};

/// The large bed.
const LARGE_BASE: Descriptor = Descriptor {
    rate: 100,
    life: 0.5,
    half_size: 25.0,
    speed: 0.0,
    retention: 0.5,
    growth: 1.0,
    stretch: 0.0,
    spawn_radius: 50.0,
    spread: 0.0,
    buoyancy: -0.33,
    centring: 0.0,
    pool: 0,
    flags: 0x1000_0A00,
    keys: Keys {
        count: 2,
        entries: BASE_KEYS,
    },
    light: LightDesc::NONE,
    sound: None,
    volume: 0.0,
    density: 0,
    dose: 0.0,
};

/// One emitter of a fire preset.
#[derive(Debug, Clone)]
pub struct EmitterSpec {
    /// The node's name.
    pub name: &'static str,
    /// The template, already scaled.
    pub template: Descriptor,
    /// The size the template was scaled by.
    pub template_size: f32,
    /// The emitter's size field: the fire size, or twice it for a bed.
    pub size: f32,
    /// The emitter's height below the fire node (cm).
    pub lift: f32,
    /// The spawn ring: source count and radius (cm).
    pub ring: Option<(usize, f32)>,
    /// The particle texture (`_FirePrt01` or `_FirePrt02`).
    pub texture: &'static str,
}

/// A fire's light.
#[derive(Debug, Clone, Copy)]
pub struct LightSpec {
    pub desc: LightDesc,
    /// The light's height above the fire node (cm).
    pub lift: f32,
    /// The radius multiplier (1.25 for a small fire above size 0.2).
    pub radius_scale: f32,
}

/// A fire built from its size: the flame, its bed (above the small tier)
/// and the light.
#[derive(Debug, Clone)]
pub struct Preset {
    pub size: f32,
    pub flame: EmitterSpec,
    pub base: Option<EmitterSpec>,
    pub light: LightSpec,
}

/// The fire for a size, as the original's `CreateFire` builds it.
pub fn preset(size: f32) -> Preset {
    // The original takes whatever float the marker holds; keep a finite,
    // positive size so the templates stay usable.
    let size = if size.is_finite() && size > 0.0 {
        size
    } else {
        0.01
    };
    if size <= MEDIUM_UP_TO {
        if size < MEDIUM_SMALL_UP_TO {
            if f64::from(size) <= SMALL_UP_TO {
                small(size)
            } else {
                medium_small(size)
            }
        } else {
            medium(size)
        }
    } else {
        // Above 1 the large pair is built at half the size.
        large(size * 0.5)
    }
}

/// The small template's light: its radius widens by a quarter above 0.2,
/// compared in double precision, so a torch's stored 0.2 does widen.
fn small_light(desc: LightDesc, size: f32) -> LightSpec {
    let radius_scale = if f64::from(size) > SMALL_WIDENED_FROM {
        1.25
    } else {
        1.0
    };
    LightSpec {
        desc,
        lift: 5.0,
        radius_scale,
    }
}

fn small(size: f32) -> Preset {
    let template = SMALL.scaled(size);
    Preset {
        size,
        flame: EmitterSpec {
            name: "_Fire",
            template,
            template_size: size,
            size,
            lift: 0.0,
            ring: None,
            texture: "_FirePrt02",
        },
        base: None,
        light: small_light(template.light, size),
    }
}

fn medium_small(size: f32) -> Preset {
    let template = MEDIUM_SMALL.scaled(size);
    let base = MEDIUM_SMALL_BASE.scaled(size * 0.5);
    Preset {
        size,
        flame: EmitterSpec {
            name: "_FireM",
            template,
            template_size: size,
            size,
            lift: 0.0,
            ring: Some((count(size, 5), size * 40.0)),
            texture: "_FirePrt01",
        },
        base: Some(EmitterSpec {
            name: "_FireBas",
            template: base,
            template_size: size * 0.5,
            size: size * 2.0,
            lift: -size * 15.0,
            ring: None,
            texture: "_FirePrt01",
        }),
        light: LightSpec {
            desc: template.light,
            lift: 40.0,
            radius_scale: 1.0,
        },
    }
}

fn medium(size: f32) -> Preset {
    // Unlike the 0.35–0.5 bed, this bed is scaled by the whole size.
    let template = MEDIUM.scaled(size);
    let base = MEDIUM_BASE.scaled(size);
    Preset {
        size,
        flame: EmitterSpec {
            name: "_FireL",
            template,
            template_size: size,
            size,
            lift: 0.0,
            ring: Some((count(size, 4), size * 40.0)),
            texture: "_FirePrt01",
        },
        base: Some(EmitterSpec {
            name: "_FireBas",
            template: base,
            template_size: size,
            size: size * 2.0,
            lift: -size * 20.0,
            ring: None,
            texture: "_FirePrt01",
        }),
        light: LightSpec {
            desc: template.light,
            lift: 50.0,
            radius_scale: 1.0,
        },
    }
}

fn large(size: f32) -> Preset {
    // The large fires are one-shot: both emitters stop once their ring is
    // empty (the builder sets the flag on the emitters).
    let mut template = LARGE.scaled(size);
    template.flags |= crate::particles::flags::ONE_SHOT;
    let mut base = LARGE_BASE.scaled(size);
    base.flags |= crate::particles::flags::ONE_SHOT;
    Preset {
        size,
        flame: EmitterSpec {
            name: "_FireL",
            template,
            template_size: size,
            size: size * 2.0,
            lift: 0.0,
            ring: Some((count(size, 4), size * 40.0)),
            texture: "_FirePrt01",
        },
        base: Some(EmitterSpec {
            name: "_FireBas",
            template: base,
            template_size: size,
            size: size * 2.0,
            lift: -size * 40.0,
            ring: None,
            texture: "_FirePrt01",
        }),
        light: LightSpec {
            desc: template.light,
            lift: 40.0,
            radius_scale: 1.0,
        },
    }
}

/// The ring's source count: `round(√size · n)`, at least one.
fn count(size: f32, n: usize) -> usize {
    (size.sqrt() * n as f32).round_ties_even().max(1.0) as usize
}

/// The intensity, radius (m) and lift (m) of a fire's light, as
/// `RENDERING.md` tabulates them.
pub fn fire_light(size: f32) -> (f32, f32, f32) {
    let p = preset(size);
    (
        p.light.desc.intensity,
        p.light.desc.radius * p.light.radius_scale / 100.0,
        p.light.lift / 100.0,
    )
}

/// A burning item's size from its object record: percent · 0.01, clamped
/// to [0.25, 10].
pub fn operative_size(percent: u16) -> f32 {
    (f32::from(percent) * 0.01).clamp(0.25, 10.0)
}

/// A fire marker's size: the four bytes the model stores, read as an f32
/// unchanged.
pub fn marker_size(bytes: [u8; 4]) -> f32 {
    f32::from_le_bytes(bytes)
}

/// The keys as the linear pipeline loads them: the RGB of each of the
/// table's first `count` keys goes to a float (×1/255), is scaled by 0.7,
/// back to a byte (×255, rounded half to even, saturated); the alpha is
/// untouched.
pub fn load_scale(entries: &[Key], count: usize) -> Vec<Key> {
    const SCALE: f32 = 0.7;
    let channel = |b: u8| {
        let v = f32::from(b) * (1.0 / 255.0);
        (v * SCALE * 255.0).round_ties_even().clamp(0.0, 255.0) as u8
    };
    entries
        .iter()
        .enumerate()
        .map(|(i, key)| {
            if i < count {
                [channel(key[0]), channel(key[1]), channel(key[2]), key[3]]
            } else {
                *key
            }
        })
        .collect()
}

/// A fire's light and its emitter's light values.
#[derive(Component, Debug, Clone)]
pub struct Fire {
    /// The intensity the light has this frame.
    pub(crate) intensity: f32,
    /// The shadow softness this frame.
    pub(crate) softness: f32,
    /// The fire's smoothed density.
    pub(crate) density: f32,
    /// Ticks the fire has run.
    pub(crate) ticks: u32,
}

impl Fire {
    /// Whether the fire has burnt long enough for its flame and light to
    /// have built up (two seconds).
    pub fn settled(&self) -> bool {
        self.ticks as f32 >= 2.0 * TICK_HZ
    }
}

/// A fire emitter waiting for its particle texture once the assets load.
#[derive(Component, Debug, Clone)]
pub struct FireFlame {
    pub texture: &'static str,
}

/// Spawns a fire of `size` under its node: its light, and one entity per
/// emitter (the flame, and above the small tier its bed). The texture is
/// attached when the assets load ([`crate::flame::FlamePlugin`]).
pub fn spawn_fire(commands: &mut Commands, node: Entity, size: f32) {
    let p = preset(size);
    let specs: Vec<&EmitterSpec> = std::iter::once(&p.flame).chain(p.base.iter()).collect();
    for (index, spec) in specs.into_iter().enumerate() {
        let sources = match spec.ring {
            Some((n, radius)) if n > 1 => {
                let mut rng = Rng::new(0x5EED_0001 ^ node.index_u32().wrapping_mul(2_654_435_761));
                Source::ring(n, radius, &mut rng)
            }
            _ => vec![Source::at_node()],
        };
        let emitter = Emitter::new(spec.template, spec.size, sources);
        let mut entity = if index == 0 {
            let light = p.light;
            let bundle = crate::light_bundle(&exa_formats::rfc::Light {
                flags: 0,
                color: light.desc.colour,
                multiplier: 1.0,
                intensity: light.desc.intensity,
                radius: light.desc.radius * light.radius_scale / 100.0,
                softness: SOFTNESS,
                spot: None,
                ranged: true,
            });
            commands.spawn((
                Name::new(spec.name),
                bundle,
                Transform::from_xyz(0.0, light.lift / 100.0, 0.0),
                Fire {
                    intensity: 0.0,
                    softness: SOFTNESS,
                    density: 0.0,
                    ticks: 0,
                },
                crate::flame::Flame::new(emitter, light.lift, spec.texture, seed(node, index)),
                ChildOf(node),
            ))
        } else {
            commands.spawn((
                Name::new(spec.name),
                Transform::from_xyz(0.0, spec.lift / 100.0, 0.0),
                crate::flame::Flame::new(emitter, 0.0, spec.texture, seed(node, index)),
                ChildOf(node),
            ))
        };
        entity.insert(FireFlame {
            texture: spec.texture,
        });
    }
}

/// A flame's random seed, distinct per node and emitter.
fn seed(node: Entity, index: usize) -> u32 {
    (node.index_u32() ^ (index as u32).wrapping_mul(0x9E37_79B9)) | 1
}

/// Sets each fire light from its emitter's state, computed at the tick.
fn flicker(mut fires: Query<(&Fire, &mut PointLight)>) {
    for (fire, mut light) in &mut fires {
        light.intensity = crate::LUMENS_PER_INTENSITY * fire.intensity.max(0.0);
        light.radius = crate::shadow_radius(fire.softness);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_torch_fire() {
        let (base, radius, lift) = fire_light(0.2);
        assert!((base - 3.578).abs() < 1e-3);
        // 1500 cm × √0.2 × 1.25: a torch's 0.2 is widened.
        assert!((radius - 8.385).abs() < 1e-3);
        assert!((lift - 0.05).abs() < 1e-6);
    }

    #[test]
    fn the_tiers_and_their_lights() {
        // 0.35 and below: the small template, radius widened above 0.2.
        let p = preset(0.35);
        assert_eq!(p.flame.template_size, 0.35);
        assert_eq!(p.light.radius_scale, 1.25);
        assert_eq!(p.flame.ring, None);
        assert!(p.base.is_none());
        // Just above 0.35: the medium-small pair, 40 cm light.
        let p = preset(0.36);
        let (n, radius) = p.flame.ring.expect("a ring");
        assert_eq!(n, 3);
        assert!((radius - 14.4).abs() < 1e-5);
        assert_eq!(p.flame.name, "_FireM");
        assert!(p.base.is_some());
        assert_eq!(p.light.lift, 40.0);
        assert_eq!(p.light.radius_scale, 1.0);
        assert!((p.light.desc.radius - 1500.0 * 0.36f32.sqrt()).abs() < 1e-4);
        // 0.5–1: the medium pair, 50 cm light, 1600 cm radius.
        let p = preset(0.8);
        assert_eq!(p.flame.template_size, 0.8);
        assert_eq!(p.flame.name, "_FireL");
        assert_eq!(p.light.lift, 50.0);
        assert!((p.light.desc.radius - 1600.0 * 0.8f32.sqrt()).abs() < 1e-3);
        // Above 1: the large pair at half the size.
        let p = preset(2.0);
        assert_eq!(p.flame.template_size, 1.0);
        assert!((p.flame.size - 2.0).abs() < 1e-6);
        assert!((p.base.as_ref().unwrap().lift + 40.0).abs() < 1e-6);
        assert!((p.light.desc.intensity - 12.0).abs() < 1e-6);
        assert!(p.flame.template.flags & crate::particles::flags::ONE_SHOT != 0);
        assert!(p.base.as_ref().unwrap().template.flags & crate::particles::flags::ONE_SHOT != 0);
    }

    #[test]
    fn the_bed_offsets_and_sizes() {
        let p = preset(0.41);
        let base = p.base.expect("a bed");
        assert_eq!(base.name, "_FireBas");
        assert!((base.lift + 0.41 * 15.0).abs() < 1e-5);
        assert!((base.size - 0.82).abs() < 1e-5);
        assert!((base.template_size - 0.205).abs() < 1e-6);
        // The 0.5–1 bed is scaled by the whole size.
        let p = preset(0.8);
        assert!((p.base.expect("a bed").template_size - 0.8).abs() < 1e-6);
        let p = preset(3.0);
        let base = p.base.expect("a bed");
        assert!((base.lift + 1.5 * 40.0).abs() < 1e-4);
        assert!((base.size - 3.0).abs() < 1e-6);
    }

    #[test]
    fn the_rings_draw_their_angles_first() {
        let p = preset(0.8);
        let (n, radius) = p.flame.ring.expect("a ring");
        assert_eq!(n, 4); // round(√0.8 · 4) = round(3.58) = 4
        assert!((radius - 32.0).abs() < 1e-6);
        let mut rng = Rng::new(3);
        let sources = Source::ring(n, radius, &mut rng);
        assert_eq!(sources.len(), 4);
        for s in &sources {
            assert!((16.0..=32.0).contains(&s.offset.length()));
        }
    }

    #[test]
    fn a_burning_item_size() {
        assert_eq!(operative_size(0), 0.25);
        assert_eq!(operative_size(20), 0.25);
        assert!((operative_size(41) - 0.41).abs() < 1e-6);
        assert_eq!(operative_size(2000), 10.0);
        assert!((marker_size(0.41f32.to_le_bytes()) - 0.41).abs() < 1e-6);
    }

    #[test]
    fn a_bad_size_does_not_panic() {
        for size in [f32::NAN, f32::INFINITY, -5.0, 0.0] {
            let p = preset(size);
            assert!(p.flame.template.half_size > 0.0);
        }
    }

    #[test]
    fn the_keys_are_the_load_time_ones() {
        assert_eq!(load_scale(RAW_SMALL_KEYS, 9), SMALL_KEYS);
        assert_eq!(load_scale(RAW_MEDIUM_SMALL_KEYS, 9), MEDIUM_SMALL_KEYS);
        assert_eq!(load_scale(RAW_MEDIUM_KEYS, 9), MEDIUM_KEYS);
        assert_eq!(load_scale(RAW_LARGE_KEYS, 7), LARGE_KEYS);
        assert_eq!(load_scale(RAW_BASE_KEYS, 2), BASE_KEYS);
        // 160 · 0.7 = 112 exactly, once through the byte-to-float scale.
        assert_eq!(SMALL_KEYS[5], [178, 112, 66, 64]);
        assert_eq!(SMALL_KEYS[9], [0, 0, 0, 48]);
    }

    #[test]
    fn the_base_is_additive() {
        assert!(BASE_KEYS.iter().all(|k| k[3] == 0));
        assert!(SMALL_BASE_KEYS.iter().all(|k| k[3] == 0));
    }

    #[test]
    fn the_key_tables_are_consistent() {
        for keys in [
            SMALL.keys,
            MEDIUM_SMALL.keys,
            MEDIUM_SMALL_BASE.keys,
            MEDIUM.keys,
            MEDIUM_BASE.keys,
            LARGE.keys,
            LARGE_BASE.keys,
        ] {
            assert_eq!(keys.entries.len() as i32, keys.count + 2);
        }
    }

    #[test]
    fn the_beds_peak_inside_their_life() {
        // Even slots: key k at age (k − 1)/n.
        let bed = MEDIUM_BASE.keys;
        assert_eq!(bed.sample(0.0, false), [0, 0, 0, 0]);
        assert_eq!(bed.sample(1.0, false), [126, 70, 35, 0]);
        let small = MEDIUM_SMALL_BASE.keys;
        assert_eq!(small.sample(2.0, false), [200, 120, 50, 0]);
        assert_eq!(small.sample(1.0, false), [0, 0, 0, 0]);
    }
}

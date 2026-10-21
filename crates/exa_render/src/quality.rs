//! The renderer options the settings screen changes: light shadow-map size,
//! hardware anti-aliasing and super-sampling.
//!
//! The original pushes a changed shadow quality into the live renderer and
//! rebuilds its shaders; an anti-aliasing or super-sampling change needs the
//! render targets rebuilt. Exarust keeps the options in resources and its
//! renderer follows them: the point lights' cube faces and the sun's map take
//! the size the shadow quality gives, the scene camera takes the requested
//! sample count while the scene is not super-sampled, and the scene's
//! super-sampling target is torn down or rebuilt as it is turned off or on.
//!
//! The other options have no Exarust feature to drive: shading quality has
//! one shader, texture quality and detail no streaming, the GI is already
//! 8-bit (the original's low-precision format), and hardware multi-sampling
//! does not apply while the scene is super-sampled (the original disables its
//! control then too). Rules: `docs/UI.md` (Settings options).

use bevy::light::{DirectionalLightShadowMap, PointLightShadowMap};
use bevy::prelude::*;

use crate::supersample::{SceneCamera, SuperSampling};

/// The settings screen's shadow quality, 1 to 5 (4 shipped).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShadowQuality(pub i32);

impl Default for ShadowQuality {
    fn default() -> Self {
        Self(4)
    }
}

/// The hardware anti-aliasing sample count: 0, 1, 2, 4 or 8 (0 shipped).
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MultisampleSetting(pub i32);

pub struct QualityPlugin;

impl Plugin for QualityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ShadowQuality>()
            .init_resource::<MultisampleSetting>()
            .add_systems(PostUpdate, (sync_shadow_maps, sync_multisample));
    }
}

/// The point-light cube face size and the sun map size for a shadow quality.
///
/// The original's shadow-quality setter changes the light shadow maps and
/// leaves the detail to its own setter. Exarust's shipped sizes (512 and
/// 4096) are the level-4 (default) sizes; the other levels halve or double
/// them around it, the sun map capped at 4096 (a guess: the exact sizes were
/// not traced).
pub fn shadow_map_sizes(quality: i32) -> (usize, usize) {
    let point = match quality.clamp(1, 5) {
        1 => 64,
        2 => 128,
        3 => 256,
        5 => 1024,
        _ => 512,
    };
    let sun = (point * 8).min(4096);
    (point, sun)
}

fn sync_shadow_maps(
    quality: Res<ShadowQuality>,
    mut point: ResMut<PointLightShadowMap>,
    mut sun: ResMut<DirectionalLightShadowMap>,
) {
    if !quality.is_changed() {
        return;
    }
    let (point_size, sun_size) = shadow_map_sizes(quality.0);
    if point.size != point_size {
        point.size = point_size;
    }
    if sun.size != sun_size {
        sun.size = sun_size;
    }
}

/// The MSAA setting for a sample count: 2, 4 and 8 map to their Bevy modes,
/// anything else turns it off.
pub fn msaa_for(samples: i32) -> Msaa {
    match samples {
        2 => Msaa::Sample2,
        4 => Msaa::Sample4,
        8 => Msaa::Sample8,
        _ => Msaa::Off,
    }
}

fn sync_multisample(
    setting: Res<MultisampleSetting>,
    sampling: Res<SuperSampling>,
    mut cameras: Query<&mut Msaa, SceneCamera>,
) {
    if !setting.is_changed() && !sampling.is_changed() {
        return;
    }
    let msaa = if sampling.0 {
        Msaa::Off
    } else {
        msaa_for(setting.0)
    };
    for mut current in &mut cameras {
        if *current != msaa {
            *current = msaa;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shadow_map_sizes_follow_the_quality() {
        assert_eq!(shadow_map_sizes(4), (512, 4096));
        assert_eq!(shadow_map_sizes(3), (256, 2048));
        assert_eq!(shadow_map_sizes(2), (128, 1024));
        assert_eq!(shadow_map_sizes(1), (64, 512));
        // Above the shipped level the sun map stays 4096.
        assert_eq!(shadow_map_sizes(5), (1024, 4096));
        // Out-of-range levels clamp to the ends.
        assert_eq!(shadow_map_sizes(0), (64, 512));
        assert_eq!(shadow_map_sizes(99), (1024, 4096));
    }

    #[test]
    fn the_sample_counts() {
        assert_eq!(msaa_for(0), Msaa::Off);
        assert_eq!(msaa_for(1), Msaa::Off);
        assert_eq!(msaa_for(2), Msaa::Sample2);
        assert_eq!(msaa_for(4), Msaa::Sample4);
        assert_eq!(msaa_for(8), Msaa::Sample8);
        assert_eq!(msaa_for(3), Msaa::Off);
    }
}

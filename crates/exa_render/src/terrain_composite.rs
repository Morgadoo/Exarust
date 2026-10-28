//! Where the terrain's offscreen composite sets live in the renderer.
//!
//! The original bakes each selected 6 m block into a colour, tangent-space
//! normal, gloss and half-size parallax-height texture, at 1024² for the 21
//! high-resolution sets and 512² for the 43 low ones, and draws those
//! textures instead of stacking layers. Exarust's terrain shader stacks the
//! layers per pixel, so the composite content is already there; this module
//! hosts the sets themselves as render-target images of the original's
//! formats and sizes, so a bake pass can own them. The live selector hands
//! its set assignments over through [`TerrainCompositeHost::sync`].
//!
//! A set's images are created the first time the selector holds it and
//! dropped when the set goes back to its pool. `EXA_TERRAIN_SETS=0` turns
//! the allocation off for memory-tight hosts; the selector's rules run
//! either way.
//!
//! Evidence: the composite set allocator and the set copies (see
//! `exa_world::terrain::composite`). The texture formats are the OpenGL
//! internal formats the original's sets are created with: RGB8 colour,
//! RG8 normal, R8 gloss and R8 half-size displacement. wgpu cannot render to
//! a three-channel target, so the colour target is RGBA8 with its alpha
//! unused (an adaptation, not a change of the composite's content).

use bevy::asset::RenderAssetUsages;
use bevy::image::{Image, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat, TextureUsages};
use std::collections::HashMap;

/// A composite set's side at `level`: `32 << level`. Levels outside 0..=15
/// have no size.
pub fn set_size(level: i32) -> u32 {
    if (0..=15).contains(&level) {
        TEXTURE_BASE << level
    } else {
        0
    }
}

/// Side of a level's set at level 0 (the original's stored 32).
pub const TEXTURE_BASE: u32 = 32;

/// Which of the set's four targets the compositor creates.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetFeatures {
    pub normal: bool,
    pub gloss: bool,
    pub displacement: bool,
}

impl Default for SetFeatures {
    fn default() -> Self {
        Self {
            normal: true,
            gloss: true,
            displacement: true,
        }
    }
}

/// The images of one hosted set, in the original's allocation order.
#[derive(Clone, Debug, PartialEq)]
pub struct HostedSet {
    pub level: i32,
    pub colour: Handle<Image>,
    pub normal: Option<Handle<Image>>,
    pub gloss: Option<Handle<Image>>,
    /// Half the colour target's side.
    pub displacement: Option<Handle<Image>>,
}

impl HostedSet {
    /// The image handles that exist, in allocation order.
    pub fn handles(&self) -> Vec<Handle<Image>> {
        let mut out = vec![self.colour.clone()];
        out.extend(self.normal.clone());
        out.extend(self.gloss.clone());
        out.extend(self.displacement.clone());
        out
    }
}

/// The composite set pool's render targets, by pool id (0..=20 high, 21..=63
/// low).
#[derive(Resource, Debug)]
pub struct TerrainCompositeHost {
    sets: HashMap<u32, HostedSet>,
    enabled: bool,
}

impl Default for TerrainCompositeHost {
    fn default() -> Self {
        Self {
            sets: HashMap::new(),
            enabled: std::env::var("EXA_TERRAIN_SETS").map_or(true, |v| v != "0"),
        }
    }
}

impl TerrainCompositeHost {
    /// Whether set images are created at all.
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// Turns allocation on or off. Assignments are still accepted while off;
    /// no image is created.
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.sets.clear();
        }
    }

    /// The hosted sets.
    pub fn sets(&self) -> &HashMap<u32, HostedSet> {
        &self.sets
    }

    pub fn len(&self) -> usize {
        self.sets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sets.is_empty()
    }

    /// The set identified by a pool id, once hosted.
    pub fn set(&self, id: u32) -> Option<&HostedSet> {
        self.sets.get(&id)
    }

    /// Creates a set's images, in the original's order and formats: colour
    /// RGB8, normal RG8, gloss R8 and displacement R8 at half the side. The
    /// images are render targets and sample linearly. An id already hosted
    /// at the same level keeps its images.
    pub fn allocate(
        &mut self,
        images: &mut Assets<Image>,
        id: u32,
        level: i32,
        features: SetFeatures,
    ) -> Option<&HostedSet> {
        if self.sets.contains_key(&id) {
            return self.sets.get(&id);
        }
        let side = set_size(level);
        if side == 0 || !self.enabled {
            return None;
        }
        let mut target = |side: u32, format: TextureFormat| target_image(images, side, format);
        let colour = target(side, TextureFormat::Rgba8Unorm);
        let normal = features
            .normal
            .then(|| target(side, TextureFormat::Rg8Unorm));
        let gloss = features.gloss.then(|| target(side, TextureFormat::R8Unorm));
        let displacement = features
            .displacement
            .then(|| target((side >> 1).max(1), TextureFormat::R8Unorm));
        Some(self.sets.entry(id).or_insert(HostedSet {
            level,
            colour,
            normal,
            gloss,
            displacement,
        }))
    }

    /// Drops a set's images (the set went back to its pool).
    pub fn release(&mut self, id: u32) {
        self.sets.remove(&id);
    }

    /// Reconciles the hosted sets with the selector's assignments: `held`
    /// lists every set a block holds as `(id, level, features)` in selection
    /// order. Missing sets are created at most `budget` at a time so a frame
    /// never makes many render targets at once; sets no block holds any more
    /// are released. Returns the ids created this call.
    pub fn sync(
        &mut self,
        images: &mut Assets<Image>,
        held: &[(u32, i32, SetFeatures)],
        budget: usize,
    ) -> Vec<u32> {
        let keep: std::collections::HashSet<u32> = held.iter().map(|&(id, _, _)| id).collect();
        let stale: Vec<u32> = self
            .sets
            .keys()
            .copied()
            .filter(|id| !keep.contains(id))
            .collect();
        for id in stale {
            self.release(id);
        }
        let mut made = Vec::new();
        if !self.enabled {
            return made;
        }
        for &(id, level, features) in held {
            if made.len() >= budget {
                break;
            }
            if self.sets.contains_key(&id) {
                continue;
            }
            if self.allocate(images, id, level, features).is_some() {
                made.push(id);
            }
        }
        made
    }
}

/// A render-target image of `side`² with the given format, sampled linearly.
fn target_image(images: &mut Assets<Image>, side: u32, format: TextureFormat) -> Handle<Image> {
    let mut image = Image::new_uninit(
        Extent3d {
            width: side.max(1),
            height: side.max(1),
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        format,
        RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
    );
    image.texture_descriptor.usage = TextureUsages::TEXTURE_BINDING
        | TextureUsages::RENDER_ATTACHMENT
        | TextureUsages::COPY_SRC
        | TextureUsages::COPY_DST;
    image.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor::linear());
    images.add(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn host() -> TerrainCompositeHost {
        let mut host = TerrainCompositeHost::default();
        host.set_enabled(true);
        host
    }

    #[test]
    fn a_set_is_created_at_the_originals_sizes_and_formats() {
        let mut host = host();
        let mut images = Assets::<Image>::default();
        let set = host
            .allocate(&mut images, 0, 5, SetFeatures::default())
            .expect("a level-5 set")
            .clone();
        // A low set is 512² with a 256² displacement target.
        let low = host
            .allocate(&mut images, 21, 4, SetFeatures::default())
            .unwrap()
            .clone();
        assert_eq!(set.level, 5);
        let size = |handle: &Handle<Image>| {
            let image = images.get(handle).unwrap();
            (
                image.texture_descriptor.size.width,
                image.texture_descriptor.size.height,
                image.texture_descriptor.format,
            )
        };
        assert_eq!(size(&set.colour), (1024, 1024, TextureFormat::Rgba8Unorm));
        assert_eq!(
            size(set.normal.as_ref().unwrap()),
            (1024, 1024, TextureFormat::Rg8Unorm)
        );
        assert_eq!(
            size(set.gloss.as_ref().unwrap()),
            (1024, 1024, TextureFormat::R8Unorm)
        );
        assert_eq!(
            size(set.displacement.as_ref().unwrap()),
            (512, 512, TextureFormat::R8Unorm)
        );
        let image = images.get(&set.colour).unwrap();
        assert!(
            image
                .texture_descriptor
                .usage
                .contains(TextureUsages::RENDER_ATTACHMENT)
        );
        assert!(
            image
                .texture_descriptor
                .usage
                .contains(TextureUsages::TEXTURE_BINDING)
        );
        assert_eq!(size(&low.colour), (512, 512, TextureFormat::Rgba8Unorm));
        assert_eq!(
            size(low.displacement.as_ref().unwrap()),
            (256, 256, TextureFormat::R8Unorm)
        );
        assert_eq!(host.len(), 2);
        assert_eq!(set_size(0), 32);
        assert_eq!(set_size(5), 1024);
        assert_eq!(set_size(-1), 0);
        assert_eq!(set_size(16), 0);
    }

    #[test]
    fn disabled_features_leave_their_targets_out() {
        let mut host = host();
        let mut images = Assets::<Image>::default();
        let set = host
            .allocate(
                &mut images,
                7,
                4,
                SetFeatures {
                    normal: false,
                    gloss: true,
                    displacement: false,
                },
            )
            .unwrap();
        assert!(set.normal.is_none());
        assert!(set.gloss.is_some());
        assert!(set.displacement.is_none());
        assert_eq!(set.handles().len(), 2);
    }

    #[test]
    fn the_sync_creates_a_budget_a_frame_and_releases_stale_sets() {
        let mut host = host();
        let mut images = Assets::<Image>::default();
        let held = [
            (0, 5, SetFeatures::default()),
            (21, 4, SetFeatures::default()),
            (1, 5, SetFeatures::default()),
        ];
        // One new set a frame.
        assert_eq!(host.sync(&mut images, &held, 1), [0]);
        assert_eq!(host.len(), 1);
        assert_eq!(host.sync(&mut images, &held, 1), [21]);
        assert_eq!(host.sync(&mut images, &held, 1), [1]);
        // Nothing new now.
        assert!(host.sync(&mut images, &held, 1).is_empty());
        // A set no block holds goes back.
        let fewer = [held[0]];
        assert!(host.sync(&mut images, &fewer, 1).is_empty());
        assert!(host.set(21).is_none() && host.set(1).is_none());
        assert!(host.set(0).is_some());
        // Disabling drops everything.
        host.set_enabled(false);
        assert!(host.is_empty());
        assert!(host.sync(&mut images, &held, 1).is_empty());
        assert!(host.is_empty());
    }
}

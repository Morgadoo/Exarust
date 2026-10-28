//! The per-draw light slots: which of an object's linked lights are uploaded
//! to its surface program, in which order, and the three `vec4`s each one
//! fills.
//!
//! The engine keeps six slots per drawing state. A draw gathers up to six
//! lights from the object's link list in list order, rejects a link with no
//! id or a weight below 0.004, and then orders them: lights without the node
//! flags' shadow bits (`0x300`) first, then the shadowed ones, each keeping
//! the slot it had last frame where it can. A light that stays in its slot
//! only has its softness radius (for a shaped light) and, when the scene's
//! light scale changes, its depth scale refreshed; a light new to the slot
//! has all three `vec4`s written. The environment word's two bits per slot
//! carry the kind: `0x200` for a light whose node has no shadow bits (the
//! sun and, in the engine, the unshadowed lights) and `0x100` for a
//! cube-shadowed one.
//!
//! The values are the ones `docs/RENDERING.md` (Shader builder) and the
//! light-record notes derive: the colour is the record's linear colour times
//! its intensity, `2/π`, the shadow-tap count, `(1 − dimmer)` and the
//! exposure; `w` is `0.8/radius`; the shape is the softness's filter radius
//! and the vertical shaping; the position's `w` is the cube map's depth
//! scale. Exarust's WESL lights read Bevy's clustered light data, so this
//! module is the original's own fill, kept tested; the renderer's light
//! lists already choose the six lights (`exa_world::shadow_links`).

use crate::shader_program::MAX_LIGHTS;
use crate::shader_uniforms::LightUniform;

/// How a light's record is treated when its slots are filled.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SlotKind {
    /// An ordinary light: `(softness + 1) · filter` radius, depth scale from
    /// the radius and the scene's light scale.
    #[default]
    Plain,
    /// A sun-like light (`R+0x288 == 2`): `softness · 0.5 · 0.005` radius and
    /// a depth scale the light record itself supplies.
    Sun,
    /// A shaped light (`R+0x288 == 3`): its radius is refreshed even when it
    /// keeps its slot.
    Shaped,
}

/// A light as the object's link list and the light record present it.
#[derive(Clone, Copy, Debug)]
pub struct LinkedLight {
    /// The link's selection weight.
    pub weight: f32,
    /// The link's id word; a zero ends the list.
    pub id: u32,
    /// The node's shadow bits (`flags & 0x300`): 0 for a light without a
    /// shadow map, `0x100` for a cube-shadowed one and `0x200` for a
    /// cube-shadowed one with vertical shaping. A light with any bit set
    /// sorts after the others.
    pub shadow_bits: u32,
    pub kind: SlotKind,
    /// The record's direct colour: its linear sRGB colour times its current
    /// intensity.
    pub direct: [f32; 3],
    /// `0.8/radius` when the light has range, else 0.
    pub attenuation: f32,
    /// The record's shadow softness.
    pub softness: f32,
    /// The vertical shaping `(y, z, w)` when the node's flags say `0x200`,
    /// else `(1, 1, 1)`.
    pub shaping: [f32; 3],
    /// The light's bounding radius (the cube map's far plane).
    pub far: f32,
    /// The sun-like light's own depth scale.
    pub depth_scale: f32,
    /// The light record's dimmer (`0x234`).
    pub dimmer: f32,
}

impl Default for LinkedLight {
    fn default() -> Self {
        Self {
            weight: 1.0,
            id: 1,
            shadow_bits: 0x100,
            kind: SlotKind::Plain,
            direct: [1.0, 1.0, 1.0],
            attenuation: 0.0,
            softness: 0.0,
            shaping: [1.0, 1.0, 1.0],
            far: 100.0,
            depth_scale: 1.0,
            dimmer: 0.0,
        }
    }
}

impl LinkedLight {
    /// Whether the node has a shadow map: the light sorts after the
    /// unshadowed ones.
    pub fn shadowed(&self) -> bool {
        self.shadow_bits & 0x300 != 0
    }

    /// The environment word's two bits for this light's slot.
    pub fn slot_kind(&self) -> u32 {
        if self.shadowed() { 0x100 } else { 0x200 }
    }

    /// The vertical shaping the slot takes: only a light whose shadow bits
    /// are exactly `0x200` has it.
    pub fn slot_shaping(&self) -> [f32; 3] {
        if self.shadow_bits == 0x200 {
            self.shaping
        } else {
            [1.0, 1.0, 1.0]
        }
    }
}

/// The scene values a light fill reads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlotScene {
    /// The shadow taps this sub-frame takes (`SdwSmps`), at least 1.
    pub shadow_samples: u32,
    /// The exposure (`state` +0x5b4, 1 in play).
    pub exposure: f32,
    /// The scene's light scale, the object's `+0x228`; a change refreshes
    /// every slot's depth scale.
    pub scale: f32,
    /// The quality scale (`0x1002ae820`, 1 in play).
    pub quality_scale: f32,
    /// The shader word's `FastLgt` bit: the filter radius uses 0.003.
    pub fast_light: bool,
    /// `ShadeQual`'s scene part: the environment word without the light
    /// slots (`state +0x20 & 0xFF`).
    pub base_environment: u32,
}

impl Default for SlotScene {
    fn default() -> Self {
        Self {
            shadow_samples: 1,
            exposure: 1.0,
            scale: 1.0,
            quality_scale: 1.0,
            fast_light: false,
            base_environment: 0,
        }
    }
}

const LINK_WEIGHT_MIN: f32 = 0.004;

/// The six light slots of a drawing state.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct DrawLightSlots {
    scene: SlotScene,
    /// The environment word with this draw's light bits.
    pub environment: u32,
    /// The lights uploaded, 0 through 6.
    pub count: usize,
    /// The three `vec4`s of each used slot.
    pub uniforms: [LightUniform; MAX_LIGHTS],
    /// The light id in each slot.
    ids: [Option<u32>; MAX_LIGHTS],
    /// The scene scale each slot's depth scale was computed at.
    scales: [f32; MAX_LIGHTS],
    /// The slot each light last occupied (the light record's own index).
    indices: Vec<(u32, i32)>,
    /// A slot was (re)filled or its depth scale changed since the records
    /// were uploaded.
    pub stale: bool,
}

/// Twelve zero floats, for a slot that has never been filled.
const EMPTY_UNIFORM: LightUniform = LightUniform {
    position: [0.0; 4],
    colour: [0.0; 4],
    shape: [0.0; 4],
};

impl DrawLightSlots {
    pub fn new(scene: SlotScene) -> Self {
        Self {
            scene,
            environment: scene.base_environment & 0xFF,
            count: 0,
            uniforms: [EMPTY_UNIFORM; MAX_LIGHTS],
            ids: [None; MAX_LIGHTS],
            scales: [f32::NAN; MAX_LIGHTS],
            indices: Vec::new(),
            stale: false,
        }
    }

    /// The scene values; changing them takes effect on the next fill.
    pub fn scene(&self) -> SlotScene {
        self.scene
    }

    /// Replace the scene values.
    pub fn set_scene(&mut self, scene: SlotScene) {
        self.scene = scene;
    }

    /// The slot a light occupies, or -1.
    fn index_of(&self, id: u32) -> i32 {
        self.indices
            .iter()
            .find(|(light, _)| *light == id)
            .map_or(-1, |(_, index)| *index)
    }

    fn set_index(&mut self, id: u32, index: i32) {
        if let Some(entry) = self.indices.iter_mut().find(|(light, _)| *light == id) {
            entry.1 = index;
        } else {
            self.indices.push((id, index));
        }
    }

    /// Empty every slot, as the engine's per-frame reset does: the
    /// environment word loses its twelve light bits and the gathered count
    /// goes to zero. The recorded uniforms are left as they were.
    pub fn clear(&mut self) {
        self.environment = self.scene.base_environment & 0xFF;
        for i in 0..MAX_LIGHTS {
            if let Some(id) = self.ids[i].take() {
                self.set_index(id, -1);
            }
            self.uniforms[i] = EMPTY_UNIFORM;
            self.scales[i] = f32::NAN;
        }
        self.count = 0;
    }

    /// The filter radius a light's softness gets: `Q · 0.005`, or `Q ·
    /// 0.003` under `FastLgt`, both in double precision and narrowed.
    fn filter_radius(&self) -> f32 {
        let base = if self.scene.fast_light { 0.003 } else { 0.005 };
        (f64::from(self.scene.quality_scale) * base) as f32
    }

    /// The position's `w` of a light: the sun's own value, else `(0.97 +
    /// softness)/(far · scene scale)` in single precision.
    fn depth_scale(&self, light: &LinkedLight) -> f32 {
        match light.kind {
            SlotKind::Sun => light.depth_scale,
            SlotKind::Plain | SlotKind::Shaped => {
                (0.97 + light.softness) / (light.far * self.scene.scale)
            }
        }
    }

    /// Fill the slots from the object's light list. Returns the number of
    /// lights uploaded.
    pub fn fill(&mut self, lights: &[LinkedLight]) -> usize {
        let mut gathered: Vec<&LinkedLight> = Vec::new();
        let limit = lights.len().min(MAX_LIGHTS);
        for light in lights.iter().take(limit) {
            if light.id == 0 {
                break;
            }
            if light.weight < LINK_WEIGHT_MIN {
                break;
            }
            gathered.push(light);
        }

        // The unshadowed lights take the first slots, the shadowed ones the
        // rest. A light keeps the slot it had last frame where it can; the
        // rest fill the first empty slot of their group.
        let unshadowed = gathered.iter().filter(|l| !l.shadowed()).count();
        let mut ordered: [Option<usize>; MAX_LIGHTS] = [None; MAX_LIGHTS];
        for (index, light) in gathered.iter().enumerate() {
            if light.shadowed() {
                continue;
            }
            let slot = self.index_of(light.id);
            if (0..unshadowed as i32).contains(&slot) {
                ordered[slot as usize] = Some(index);
            } else {
                self.set_index(light.id, -1);
            }
        }
        for (index, light) in gathered.iter().enumerate() {
            if light.shadowed() || self.index_of(light.id) >= 0 {
                continue;
            }
            if let Some(slot) = ordered[..unshadowed].iter().position(Option::is_none) {
                ordered[slot] = Some(index);
            }
        }
        for (index, light) in gathered.iter().enumerate() {
            if !light.shadowed() {
                continue;
            }
            let slot = self.index_of(light.id);
            if slot >= unshadowed as i32 && (slot as usize) < gathered.len() {
                ordered[slot as usize] = Some(index);
            } else {
                self.set_index(light.id, -1);
            }
        }
        for (index, light) in gathered.iter().enumerate() {
            if !light.shadowed() || self.index_of(light.id) >= 0 {
                continue;
            }
            if unshadowed < MAX_LIGHTS
                && let Some(slot) = ordered[unshadowed..].iter().position(Option::is_none)
            {
                ordered[unshadowed + slot] = Some(index);
            }
        }

        self.environment = self.scene.base_environment & 0xFF;
        let filter = self.filter_radius();
        let mut count = 0;
        for (slot, source) in ordered.iter().enumerate() {
            let Some(index) = *source else { break };
            count += 1;
            let light = gathered[index];
            let kind = light.slot_kind();
            if self.ids[slot] == Some(light.id) {
                if light.kind == SlotKind::Shaped {
                    self.uniforms[slot].shape[0] = (light.softness + 1.0) * filter;
                }
                // `comiss` treats an unordered compare as different, so a
                // NaN scale always refreshes.
                if !(self.scene.scale == self.scales[slot]) {
                    self.uniforms[slot].position[3] = self.depth_scale(light);
                    self.scales[slot] = self.scene.scale;
                    self.stale = true;
                }
            } else {
                if let Some(old) = self.ids[slot] {
                    self.set_index(old, -1);
                }
                self.ids[slot] = Some(light.id);
                self.set_index(light.id, slot as i32);
                let taps = self.scene.shadow_samples.max(1);
                let normalise = (0.636_619_772_367_581_4f64 / f64::from(taps)) as f32;
                let dim = (1.0 - light.dimmer) * self.scene.exposure;
                self.uniforms[slot].colour = [
                    light.direct[0] * normalise * dim,
                    light.direct[1] * normalise * dim,
                    light.direct[2] * normalise * dim,
                    light.attenuation,
                ];
                self.uniforms[slot].position[3] = self.depth_scale(light);
                self.uniforms[slot].shape[0] = if light.kind == SlotKind::Sun {
                    ((light.softness * 0.5) as f64 * 0.005) as f32
                } else {
                    (light.softness + 1.0) * filter
                };
                self.uniforms[slot].shape[1..4].copy_from_slice(&light.slot_shaping());
                self.scales[slot] = self.scene.scale;
                self.stale = true;
            }
            self.environment |= kind << (2 * slot as u32);
        }
        self.count = count;
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::material::linear;
    use crate::shader_uniforms::vertical_shaping;

    fn close(a: f32, b: f32) {
        assert!((a - b).abs() <= 1e-5, "{a} != {b}");
    }

    fn light(id: u32, shadowed: bool) -> LinkedLight {
        LinkedLight {
            id,
            shadow_bits: if shadowed { 0x100 } else { 0 },
            ..LinkedLight::default()
        }
    }

    /// The first light of `exanima01` from the light-record notes.
    fn ceiling_light() -> LinkedLight {
        LinkedLight {
            weight: 1.0,
            id: 7,
            shadow_bits: 0x200,
            kind: SlotKind::Plain,
            direct: [
                linear(0.808) * 60.0,
                linear(0.808) * 60.0,
                linear(0.808) * 60.0,
            ],
            attenuation: 0.8 / 630.0,
            softness: 0.5,
            shaping: vertical_shaping(0.0, 15.0),
            far: 630.0,
            depth_scale: 1.0,
            dimmer: 0.0,
        }
    }

    #[test]
    fn the_first_ceiling_light_fills_the_three_vec4s() {
        let scene = SlotScene::default();
        let mut slots = DrawLightSlots::new(scene);
        assert_eq!(slots.fill(&[ceiling_light()]), 1);
        let u = slots.uniforms[0];
        close(u.colour[0], ceiling_light().direct[0] * 0.636_619_75);
        // The light-record notes' 37.05·2/π ≈ 23.59.
        assert!((u.colour[0] - 23.586).abs() < 0.01);
        close(u.colour[3], 0.8 / 630.0);
        close(u.position[3], 1.47 / 630.0);
        close(u.shape[0], 0.0075);
        assert_eq!(u.shape[1..4], vertical_shaping(0.0, 15.0));
        assert_eq!(slots.environment & !0xFF, 0x100);
        assert!(slots.stale);
        assert_eq!(slots.count, 1);
    }

    #[test]
    fn the_colour_divides_by_the_shadow_taps_and_the_dimmer() {
        let mut light = ceiling_light();
        light.dimmer = 0.25;
        let scene = SlotScene {
            shadow_samples: 2,
            exposure: 2.0,
            ..SlotScene::default()
        };
        let mut slots = DrawLightSlots::new(scene);
        slots.fill(&[light]);
        let expected = light.direct[0] * (0.636_619_772_367_581_4f64 / 2.0) as f32 * 1.5;
        close(slots.uniforms[0].colour[0], expected);
    }

    #[test]
    fn a_fast_light_gets_the_narrower_filter() {
        let mut slots = DrawLightSlots::new(SlotScene {
            fast_light: true,
            ..SlotScene::default()
        });
        slots.fill(&[ceiling_light()]);
        close(slots.uniforms[0].shape[0], 1.5 * 0.003);
        // The sun's own radius is not scaled by the quality setting at all.
        let sun = LinkedLight {
            kind: SlotKind::Sun,
            softness: 0.5,
            depth_scale: 0.75,
            ..light(9, false)
        };
        slots.fill(&[sun]);
        close(slots.uniforms[0].shape[0], 0.25 * 0.005);
        close(slots.uniforms[0].position[3], 0.75);
        assert_eq!(slots.environment & !0xFF, 0x200);
    }

    #[test]
    fn unshadowed_lights_take_the_first_slots_in_list_order() {
        let mut slots = DrawLightSlots::new(SlotScene::default());
        let lights = [
            light(1, true),
            light(2, false),
            light(3, true),
            light(4, false),
        ];
        assert_eq!(slots.fill(&lights), 4);
        // Unshadowed 2 then 4, shadowed 1 then 3.
        let order = |i| slots.ids[i];
        assert_eq!(
            [order(0), order(1), order(2), order(3)],
            [Some(2), Some(4), Some(1), Some(3)]
        );
        // Two bits a slot: 0x200, 0x200, 0x100, 0x100.
        assert_eq!(
            slots.environment & !0xFF,
            0x200 | (0x200 << 2) | (0x100 << 4) | (0x100 << 6)
        );
    }

    #[test]
    fn a_light_that_keeps_its_slot_only_refreshes_what_changed() {
        let mut slots = DrawLightSlots::new(SlotScene::default());
        slots.fill(&[light(1, false), light(2, false)]);
        let before = slots.uniforms;
        slots.stale = false;
        // Nothing changed: no new uniforms, no stale flag.
        slots.fill(&[light(1, false), light(2, false)]);
        assert_eq!(slots.uniforms, before);
        assert!(!slots.stale);
        // A shaped light's radius is refreshed in place. The engine does not
        // mark the light records stale for that alone.
        let shaped = LinkedLight {
            kind: SlotKind::Shaped,
            softness: 2.0,
            ..light(1, false)
        };
        slots.fill(&[shaped, light(2, false)]);
        assert!(!slots.stale);
        close(slots.uniforms[0].shape[0], 3.0 * 0.005);
        // Only the shape changed; the colour is the old one.
        assert_eq!(slots.uniforms[0].colour, before[0].colour);
    }

    #[test]
    fn a_new_scene_scale_refreshes_the_depth_scale() {
        let mut slots = DrawLightSlots::new(SlotScene::default());
        slots.fill(&[ceiling_light()]);
        close(slots.uniforms[0].position[3], 1.47 / 630.0);
        slots.stale = false;
        slots.set_scene(SlotScene {
            scale: 2.0,
            ..SlotScene::default()
        });
        slots.fill(&[ceiling_light()]);
        close(slots.uniforms[0].position[3], 1.47 / 1260.0);
        assert!(slots.stale);
        // A NaN scale is a change, as the unordered compare makes it.
        slots.stale = false;
        slots.set_scene(SlotScene {
            scale: f32::NAN,
            ..SlotScene::default()
        });
        slots.fill(&[ceiling_light()]);
        assert!(slots.stale);
    }

    #[test]
    fn a_replaced_light_fills_its_slot_again() {
        let mut slots = DrawLightSlots::new(SlotScene::default());
        slots.fill(&[light(1, false)]);
        slots.stale = false;
        slots.fill(&[light(2, false)]);
        assert!(slots.stale);
        assert_eq!(slots.ids[0], Some(2));
        assert_eq!(slots.index_of(1), -1);
        assert_eq!(slots.index_of(2), 0);
    }

    #[test]
    fn a_light_keeps_its_slot_between_reorders() {
        let mut slots = DrawLightSlots::new(SlotScene::default());
        slots.fill(&[light(1, false), light(2, false)]);
        // The list order turns around; each light keeps its slot.
        slots.fill(&[light(2, false), light(1, false)]);
        assert_eq!([slots.ids[0], slots.ids[1]], [Some(1), Some(2)]);
    }

    #[test]
    fn a_zero_id_or_a_faint_link_ends_the_list() {
        let mut slots = DrawLightSlots::new(SlotScene::default());
        let faint = LinkedLight {
            weight: 0.003,
            ..light(2, false)
        };
        assert_eq!(slots.fill(&[light(1, false), faint, light(3, false)]), 1);
        // A NaN weight is not below the threshold, as the unordered compare
        // skips the rejection.
        let nan = LinkedLight {
            weight: f32::NAN,
            ..light(2, false)
        };
        assert_eq!(slots.fill(&[nan, light(3, false)]), 2);
        let zero = LinkedLight {
            id: 0,
            ..light(1, false)
        };
        assert_eq!(slots.fill(&[light(4, false), zero, light(5, false)]), 1);
    }

    #[test]
    fn at_most_six_lights_are_gathered() {
        let mut slots = DrawLightSlots::new(SlotScene::default());
        let lights: Vec<_> = (1..=8).map(|i| light(i, false)).collect();
        assert_eq!(slots.fill(&lights), 6);
        assert_eq!(slots.ids[5], Some(6));
        assert_eq!(
            slots.environment & !0xFF,
            0x200 | (0x200 << 2) | (0x200 << 4) | (0x200 << 6) | (0x200 << 8) | (0x200 << 10)
        );
    }

    #[test]
    fn clearing_releases_every_slot_and_keeps_the_scene_word() {
        let mut slots = DrawLightSlots::new(SlotScene {
            base_environment: 0x15,
            ..SlotScene::default()
        });
        slots.fill(&[light(1, false), light(2, true)]);
        assert_eq!(slots.environment & !0xFF, 0x200 | (0x100 << 2));
        slots.clear();
        assert_eq!(slots.environment, 0x15);
        assert_eq!(slots.count, 0);
        assert_eq!(slots.ids, [None; MAX_LIGHTS]);
        assert_eq!(slots.index_of(1), -1);
        assert_eq!(slots.index_of(2), -1);
    }
}

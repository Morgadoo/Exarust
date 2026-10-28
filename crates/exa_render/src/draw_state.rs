//! The drawing state's per-draw decisions: which object and environment
//! words a draw starts from, whether it gathers lights or clears them, how
//! many sub-frames it renders and what each one uploads, and the material
//! overrides the frame applies (the room fade and the emissive scale).
//!
//! The engine keeps one drawing state per view. A draw first rebuilds the
//! object word from the frame's word plus the streams the geometry has
//! (vertex colours and deformed vertices), gathers at most six lights from
//! the object's list unless the object has none or the material is unlit,
//! and keys its program on the material and shader words plus that object
//! and environment pair. Then it uploads the light array, the material
//! block after an emissive or fade change, the GI corner and the liquid
//! level, and renders the object's super-sampling sub-frames, each with its
//! own shadow-tap window and sub-frame clock. With super-sampling off the
//! engine takes the single-pass variant instead: one matrix, one tap
//! window, no sub-frame clock; the terrain and grass draws use it too.
//!
//! Exarust's renderer lets Bevy draw, so this module is the original's own
//! fill, kept tested beside [`crate::shader_cache`]; the live path draws
//! through `exa_render::material` and `exa_world`'s light lists
//! (`docs/RENDERING.md`, Shader builder).

use crate::light_slots::{DrawLightSlots, LinkedLight, SlotScene};
use crate::shader_bindings::{ShadowSampleUpload, shadow_sample_upload};
use crate::shader_cache::ProgramWords;
use crate::shader_program::draw_passes;

/// The threshold above which a changed fade alpha rewrites the material
/// block: `0x33FFFFD7`, the engine's 2⁻²³-like epsilon.
pub const FADE_EPSILON: f32 = f32::from_bits(0x33FF_FFD7);

/// A geometry record's word bit `0x8000`: the node is animated, so the
/// draw uploads a vertex-delta stream and the engine changes its cull state.
pub const GEOMETRY_VERTEX_DELTA: u32 = 0x8000;

/// A geometry record's word bit `0x10`: the object's clock advances with
/// each super-sampling sub-frame.
pub const GEOMETRY_SUBFRAME_CLOCK: u32 = 0x10;

/// A geometry record's flag word bit `0x10`: the material's emissive is
/// rescaled for this draw.
pub const EMISSIVE_PER_DRAW: u32 = 0x10;

/// An object's frame word bit `0x2000_0000`: the diffuse alpha is
/// the frame's fade, `1 − fade`.
pub const FRAME_FADE: u32 = 0x2000_0000;

/// An object's frame word bit `0x2_0000`: the scene's emissive scale is
/// used instead of the geometry record's.
pub const FRAME_EMISSIVE_SCALE: u32 = 0x2_0000;

/// Which of the two per-draw variants a draw takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawPath {
    /// The surface path: one program with the scene's super-sampling
    /// sub-frames, each uploading its own shadow taps.
    Supersampled,
    /// The single-pass path the engine takes when the scene draws no
    /// super-sampling sub-frames (the terrain and grass draws use it too):
    /// one draw, the taps uploaded once, and no sub-frame clock.
    Plain,
}

/// The variant a scene's sub-frame count selects: the single-pass path with
/// at most one sub-frame (super-sampling off), the super-sampling path
/// otherwise.
pub fn draw_path(passes: u32) -> DrawPath {
    if passes <= 1 {
        DrawPath::Plain
    } else {
        DrawPath::Supersampled
    }
}

/// The geometry record's own values a draw reads.
#[derive(Clone, Copy, Debug)]
pub struct GeometryRecord {
    /// The record's word: [`GEOMETRY_VERTEX_DELTA`] and
    /// [`GEOMETRY_SUBFRAME_CLOCK`].
    pub word: u32,
    /// The record's flag word: [`EMISSIVE_PER_DRAW`].
    pub flags: u32,
}

/// One object of a draw.
#[derive(Clone, Copy, Debug)]
pub struct DrawObject<'a> {
    /// The material word the factory built for the object.
    pub material: u32,
    /// The shader word the factory built.
    pub shader: u32,
    /// The builder's frame object word: the shadow-tap count less one from
    /// bit 24 and the quad bit when super-sampling is on.
    pub frame_object: u32,
    /// The object's frame word.
    pub frame: u32,
    /// The frame's fade, 1 gone.
    pub fade: f32,
    /// The material block's diffuse alpha the material writer stored: the
    /// fade rewrite compares against it.
    pub diffuse_alpha: f32,
    /// The geometry record's values.
    pub geometry: GeometryRecord,
    /// The object's own per-draw emissive scale.
    pub emissive: f32,
    /// The mesh has a vertex-colour stream.
    pub vertex_colors: bool,
    /// The object's light links, in list order; `None` for no list.
    pub lights: Option<&'a [LinkedLight]>,
}

impl Default for DrawObject<'_> {
    fn default() -> Self {
        Self {
            material: 0,
            shader: 0,
            frame_object: 0,
            frame: 0,
            fade: 0.0,
            diffuse_alpha: 1.0,
            geometry: GeometryRecord { word: 0, flags: 0 },
            emissive: 1.0,
            vertex_colors: false,
            lights: None,
        }
    }
}

/// The frame values every draw of a view reads.
#[derive(Clone, Copy, Debug)]
pub struct DrawScene {
    /// The light fill's scene values.
    pub slot: SlotScene,
    /// The scene's super-sampling sub-frame count (1, 2 or 4), the value
    /// that also picks the draw path.
    pub passes: u32,
    /// The frame's sub-frame index for the single-pass path's tap window.
    pub frame_pass: u32,
    /// The sun-shadow mode: 3 halves the passes of a
    /// [`HALF_PASSES`](crate::shader_program::HALF_PASSES) object.
    pub sun_shadow_mode: u32,
    /// The scene's emissive scale, used by an object whose frame word has
    /// [`FRAME_EMISSIVE_SCALE`].
    pub emissive_scale: f32,
    /// The first sub-frame's clock and the step between sub-frames.
    pub subframe_base: f32,
    pub subframe_step: f32,
}

impl Default for DrawScene {
    fn default() -> Self {
        Self {
            slot: SlotScene::default(),
            passes: 4,
            frame_pass: 0,
            sun_shadow_mode: 0,
            emissive_scale: 1.0,
            subframe_base: 0.0,
            subframe_step: 0.0,
        }
    }
}

/// One sub-frame of a draw.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Subframe {
    /// The sub-frame's index, the `PassIdx` uniform of a quad render.
    pub index: u32,
    /// The `SdwSmp` window this sub-frame uploads.
    pub taps: ShadowSampleUpload,
    /// The object's clock for this sub-frame, for an animated object whose
    /// geometry word has [`GEOMETRY_SUBFRAME_CLOCK`]; `None` otherwise.
    pub time: Option<f32>,
}

/// What a draw does, in the order it does it.
#[derive(Clone, Debug)]
pub struct DrawPlan {
    /// The object word: the frame's word plus the streams the geometry has.
    pub object_word: u32,
    /// The environment word after the light gather.
    pub environment: u32,
    /// Lights gathered, 0 through 6.
    pub lights: usize,
    /// The light array is uploaded (a slot was refilled or its depth scale
    /// changed, or the draw bound another program than the last one).
    pub upload_lights: bool,
    /// The material block is rewritten with this diffuse alpha (the room
    /// fade), when the object's frame word asks for it and it moved.
    pub fade_alpha: Option<f32>,
    /// The material block's emissive is rewritten with this scale.
    pub emissive_scale: Option<f32>,
    /// The `LqdLvl` uniform is uploaded (the material word's refracting bit).
    pub liquid_level: bool,
    /// The `VXGIPos` uniform is uploaded (the environment has the GI
    /// volume).
    pub gi_position: bool,
    /// The sub-frames to render; the plain path has one.
    pub subframes: Vec<Subframe>,
    /// The taps the plain path uploads once, at its current frame's index.
    pub plain_taps: Option<ShadowSampleUpload>,
}

impl DrawPlan {
    /// The program cache key of the draw.
    pub fn words(&self, object: &DrawObject<'_>) -> ProgramWords {
        ProgramWords {
            object: self.object_word,
            environment: self.environment,
            material: object.material,
            shader: object.shader,
        }
    }
}

/// The drawing state a view keeps across its draws: the six light slots and
/// their fill state.
#[derive(Clone, Debug, Default)]
pub struct DrawState {
    slots: DrawLightSlots,
}

impl DrawState {
    pub fn new(scene: SlotScene) -> Self {
        Self {
            slots: DrawLightSlots::new(scene),
        }
    }

    /// The light slots as the last draw left them.
    pub fn slots(&self) -> &DrawLightSlots {
        &self.slots
    }

    /// Replace the scene values.
    pub fn set_scene(&mut self, scene: SlotScene) {
        self.slots.set_scene(scene);
    }

    /// The scene begin (`0x1001ff3c0`): empty every light slot and drop the
    /// environment word's light bits, keeping the uploaded uniforms.
    pub fn begin_scene(&mut self) {
        self.slots.clear();
    }

    /// Whether the light records must be re-uploaded before the next draw.
    pub fn lights_stale(&self) -> bool {
        self.slots.stale
    }

    /// Take the stale flag, as a light upload does.
    pub fn take_lights_stale(&mut self) -> bool {
        std::mem::take(&mut self.slots.stale)
    }

    /// Prepare one draw: gather or clear its lights, rebuild its object
    /// word, decide the material overrides and list its sub-frames.
    ///
    /// The stale flags are consumed: a plan that uploads the light array
    /// clears the flag, as the engine's upload does.
    pub fn prepare(
        &mut self,
        object: &DrawObject<'_>,
        scene: &DrawScene,
        path: DrawPath,
        program_changed: bool,
    ) -> DrawPlan {
        self.slots.set_scene(scene.slot);
        let lit = object.material & 6 != 0;
        let gathered = match object.lights {
            Some(lights) if lit => self.slots.fill(lights),
            _ => {
                self.slots.clear();
                0
            }
        };
        let mut object_word = object.frame_object;
        if object.vertex_colors {
            object_word |= crate::shader_program::object::VERTEX_COLOR;
        }
        if object.geometry.word & GEOMETRY_VERTEX_DELTA != 0 {
            object_word |= crate::shader_program::object::VERTEX_DELTA;
        }
        let environment = self.slots.environment;
        // The plain path uploads the lights only for a lit material; the
        // surface path whenever any light is gathered.
        // The light records are re-uploaded when a slot changed or the
        // draw bound another program than the last one (the drawing
        // state's single stale flag).
        let upload_lights = match path {
            DrawPath::Supersampled => gathered > 0,
            DrawPath::Plain => lit && gathered > 0,
        } && (self.slots.stale || program_changed);
        if upload_lights {
            self.slots.stale = false;
        }
        let fade_alpha = if object.frame & FRAME_FADE != 0 {
            let alpha = 1.0 - object.fade;
            // `comiss` treats an unordered compare as not above, so a NaN
            // difference never rewrites the block.
            if FADE_EPSILON < (object.diffuse_alpha - alpha).abs() {
                Some(alpha)
            } else {
                None
            }
        } else {
            None
        };
        let emissive_scale =
            if object.material & 8 != 0 && object.geometry.flags & EMISSIVE_PER_DRAW != 0 {
                Some(if object.frame & FRAME_EMISSIVE_SCALE != 0 {
                    scene.emissive_scale
                } else {
                    object.emissive
                })
            } else {
                None
            };
        let quad = object_word & crate::shader_program::object::QUAD_RENDER != 0;
        let samples = scene.slot.shadow_samples;
        let passes = match path {
            DrawPath::Supersampled => {
                draw_passes(scene.passes, object.frame, scene.sun_shadow_mode)
            }
            DrawPath::Plain => 1,
        };
        let mut subframes = Vec::with_capacity(passes as usize);
        let mut plain_taps = None;
        for index in 0..passes {
            let taps = if path == DrawPath::Plain {
                shadow_sample_upload(samples, quad, scene.frame_pass)
            } else {
                shadow_sample_upload(samples, quad, index)
            };
            if path == DrawPath::Plain {
                plain_taps = Some(taps);
            }
            let time = (path == DrawPath::Supersampled
                && object.geometry.word & GEOMETRY_SUBFRAME_CLOCK != 0)
                .then_some(scene.subframe_base + index as f32 * scene.subframe_step);
            subframes.push(Subframe { index, taps, time });
        }
        DrawPlan {
            object_word,
            environment,
            lights: gathered,
            upload_lights,
            fade_alpha,
            emissive_scale,
            liquid_level: object.material & 0x200 != 0,
            gi_position: environment & 0x10 != 0,
            subframes,
            plain_taps,
        }
    }
}

/// The shutter time of one super-sampling sub-frame for a node that blurs
/// its motion (the node's motion-blur flag `0x20`): the sub-frame's slot
/// `index + first` (the frame's first sub-frame index) is centred with
/// `+ 0.5`, scaled by the node's shutter `duration` over the sample factor,
/// moved to the frame clock and back by half the shutter. Single precision
/// in the engine's order: `((i + 0.5)·d / factor + clock) − d·0.5`.
/// [The node field read as a shutter length and the added frame index are
/// named from how the formula uses them; the arithmetic is exact.]
pub fn motion_blur_time(index: u32, first: i32, duration: f32, factor: u32, clock: f32) -> f32 {
    let slot = (i64::from(index) + i64::from(first)) as f32;
    ((slot + 0.5) * duration / factor as f32 + clock) - duration * 0.5
}

/// The world matrix an effect draw uses, from the original's parent-transform
/// pass: an object without its own matrix takes its parent's, unless its
/// material word has bit `0x20`; an object with one keeps it. The pass also
/// sets the chosen node's sub-frame clock; this returns the matrix only.
pub fn effect_world_matrix(
    own: Option<[[f32; 4]; 4]>,
    parent: Option<[[f32; 4]; 4]>,
    material: u32,
) -> Option<[[f32; 4]; 4]> {
    match (own, parent) {
        (Some(matrix), _) => Some(matrix),
        (None, Some(matrix)) if material & 0x20 == 0 => Some(matrix),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::light_slots::{LinkedLight, SlotKind};
    use crate::shader_program::HALF_PASSES;

    fn light(id: u32) -> LinkedLight {
        LinkedLight {
            id,
            shadow_bits: 0x100,
            kind: SlotKind::Plain,
            ..LinkedLight::default()
        }
    }

    fn scene() -> DrawScene {
        DrawScene {
            slot: SlotScene {
                shadow_samples: 2,
                exposure: 1.0,
                scale: 1.0,
                light_limit: 6,
                ..SlotScene::default()
            },
            passes: 4,
            ..DrawScene::default()
        }
    }

    fn object() -> DrawObject<'static> {
        DrawObject {
            material: 0x7 | 0x20_000,
            shader: 0x1,
            frame_object: (2 - 1) << 24 | 0x1000,
            ..DrawObject::default()
        }
    }

    #[test]
    fn the_pass_count_picks_the_single_pass_path() {
        assert_eq!(draw_path(0), DrawPath::Plain);
        assert_eq!(draw_path(1), DrawPath::Plain);
        assert_eq!(draw_path(2), DrawPath::Supersampled);
        assert_eq!(draw_path(4), DrawPath::Supersampled);
    }

    #[test]
    fn a_supersampled_draw_lists_a_subframe_per_pass_with_its_tap_window() {
        let lights = [light(1), light(2)];
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.lights = Some(&lights);
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(
            plan.object_word,
            (1 << 24) | crate::shader_program::object::QUAD_RENDER
        );
        assert_eq!(plan.lights, 2);
        // The light bits sit two per slot from bit 8. The low byte is the
        // scene's own (zero in this scene).
        assert_eq!(plan.environment >> 8, 0x5);
        assert_eq!(plan.environment & 0xFF, 0);
        assert!(plan.upload_lights);
        assert!(!state.lights_stale());
        assert_eq!(plan.subframes.len(), 4);
        // Two taps per pass, four per tap in quadrant mode: eight taps,
        // each pass starting one pass's worth on.
        assert_eq!(
            plan.subframes[0].taps,
            ShadowSampleUpload {
                first_tap: 0,
                taps: 8
            }
        );
        assert_eq!(
            plan.subframes[1].taps,
            ShadowSampleUpload {
                first_tap: 2,
                taps: 8
            }
        );
        assert_eq!(
            plan.subframes[3].taps,
            ShadowSampleUpload {
                first_tap: 6,
                taps: 8
            }
        );
        assert!(plan.gi_position == (plan.environment & 0x10 != 0));
        assert!(plan.fade_alpha.is_none());
        assert!(plan.emissive_scale.is_none());
        assert!(!plan.liquid_level);
        assert!(plan.plain_taps.is_none());
        // The program key uses the gathered environment word.
        let words = plan.words(&object);
        assert_eq!(words.object, plan.object_word);
        assert_eq!(words.environment, plan.environment);
    }

    #[test]
    fn a_draw_without_a_list_or_on_an_unlit_material_clears_the_lights() {
        let mut state = DrawState::new(scene().slot);
        let lights = [light(1)];
        // No light list: the slots are cleared.
        let mut object = object();
        object.lights = Some(&lights);
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.lights, 1);
        object.lights = None;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.lights, 0);
        assert_eq!(plan.environment & 0xFF, 0);
        // An unlit material ignores the list.
        object.lights = Some(&lights);
        object.material = 0x8;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.lights, 0);
        assert_eq!(plan.environment & 0xFF, 0);
    }

    #[test]
    fn a_program_change_reuploads_the_lights() {
        let lights = [light(1)];
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.lights = Some(&lights);
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert!(plan.upload_lights);
        // A second draw with the same slots and the same program leaves the
        // light records alone.
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert!(!plan.upload_lights);
        // Binding another program marks the drawing state stale.
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, true);
        assert!(plan.upload_lights);
        // With no lights there is nothing to upload even on a program
        // change.
        object.lights = None;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, true);
        assert_eq!(plan.lights, 0);
        assert!(!plan.upload_lights);
    }

    #[test]
    fn the_plain_path_uploads_one_window_and_no_subframe_clock() {
        let lights = [light(1)];
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.lights = Some(&lights);
        let plan = state.prepare(&object, &scene(), DrawPath::Plain, false);
        assert_eq!(plan.subframes.len(), 1);
        assert_eq!(plan.subframes[0].index, 0);
        assert_eq!(
            plan.plain_taps,
            Some(ShadowSampleUpload {
                first_tap: 0,
                taps: 8
            })
        );
        // An unlit material's plain draw uploads no lights.
        object.material = 0x8;
        let plan = state.prepare(&object, &scene(), DrawPath::Plain, false);
        assert_eq!(plan.lights, 0);
        assert!(!plan.upload_lights);
    }

    #[test]
    fn the_ninth_subframe_index_still_carries_the_quadrant_mode() {
        // A non-supersampled program uploads the frame's taps.
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.frame_object = (2 - 1) << 24;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(
            plan.subframes[0].taps,
            ShadowSampleUpload {
                first_tap: 0,
                taps: 2
            }
        );
        assert_eq!(
            plan.subframes[3].taps,
            ShadowSampleUpload {
                first_tap: 6,
                taps: 2
            }
        );
    }

    #[test]
    fn an_animated_object_gets_a_clock_per_subframe() {
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.geometry.word = GEOMETRY_SUBFRAME_CLOCK;
        let mut scene = scene();
        scene.subframe_base = 0.25;
        scene.subframe_step = 0.125;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.subframes[0].time, Some(0.25));
        assert_eq!(plan.subframes[2].time, Some(0.25 + 2.0 * 0.125));
        // The plain path draws once and sets no sub-frame clock.
        let plan = state.prepare(&object, &scene, DrawPath::Plain, false);
        assert_eq!(plan.subframes[0].time, None);
        // A static object has none.
        object.geometry.word = 0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert!(plan.subframes.iter().all(|s| s.time.is_none()));
    }

    #[test]
    fn the_object_word_takes_the_streams_the_geometry_has() {
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.vertex_colors = true;
        object.geometry.word = GEOMETRY_VERTEX_DELTA;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_ne!(
            plan.object_word & crate::shader_program::object::VERTEX_COLOR,
            0
        );
        assert_ne!(
            plan.object_word & crate::shader_program::object::VERTEX_DELTA,
            0
        );
    }

    #[test]
    fn the_fade_rewrites_only_after_the_epsilon() {
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.frame = FRAME_FADE;
        object.fade = 0.5;
        object.diffuse_alpha = 0.5;
        // The target 1 − fade equals the stored alpha: nothing is uploaded.
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.fade_alpha, None);
        object.diffuse_alpha = 0.5 - 1e-4;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.fade_alpha, Some(0.5));
        // One float below the target is still under the epsilon; the
        // epsilon itself is the engine's constant, one step above that.
        object.diffuse_alpha = f32::from_bits(0.5f32.to_bits() - 1);
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.fade_alpha, None);
        object.diffuse_alpha = 0.5 - FADE_EPSILON;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.fade_alpha, Some(0.5));
        // A NaN stored alpha never rewrites (the original's compare is not
        // an ordered `<`).
        object.diffuse_alpha = f32::NAN;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.fade_alpha, None);
        // Without the frame bit no fade is applied.
        object.frame = 0;
        object.diffuse_alpha = 0.0;
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.fade_alpha, None);
    }

    #[test]
    fn the_emissive_scale_comes_from_the_object_or_the_scene() {
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.material = 0x8;
        object.geometry.flags = EMISSIVE_PER_DRAW;
        object.emissive = 0.25;
        let mut scene = scene();
        scene.emissive_scale = 2.0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.emissive_scale, Some(0.25));
        object.frame = FRAME_EMISSIVE_SCALE;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.emissive_scale, Some(2.0));
        // Only a record with the emissive bit is rescaled.
        object.material = 0x7;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.emissive_scale, None);
        // And only a geometry whose flags ask for it.
        object.material = 0x8;
        object.geometry.flags = 0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.emissive_scale, None);
    }

    #[test]
    fn the_liquid_level_and_the_gi_corner_follow_the_words() {
        let mut state = DrawState::new(SlotScene::default());
        let mut object = object();
        object.material = 0x7 | 0x200;
        let mut scene = scene();
        scene.slot.base_environment = 0x10;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert!(plan.liquid_level);
        assert!(plan.gi_position);
        object.material = 0x7;
        scene.slot.base_environment = 0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert!(!plan.liquid_level);
        assert!(!plan.gi_position);
    }

    #[test]
    fn the_half_passes_bit_halves_the_subframes_only_under_sun_mode_three() {
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.frame = HALF_PASSES;
        let mut scene = scene();
        scene.sun_shadow_mode = 3;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.subframes.len(), 2);
        scene.sun_shadow_mode = 2;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.subframes.len(), 4);
        // A single pass stays one: the engine halves the last index, not
        // the count.
        scene.passes = 1;
        scene.sun_shadow_mode = 3;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.subframes.len(), 1);
    }

    #[test]
    fn motion_blur_spreads_the_subframes_over_the_shutter() {
        // Four samples over a 0.1 s shutter at clock 2: the slots sit at
        // 1.9625, 1.9875, 2.0125 and 2.0375 (each step one quarter shutter).
        let times: Vec<f32> = (0..4)
            .map(|i| motion_blur_time(i, 0, 0.1, 4, 2.0))
            .collect();
        assert_eq!(times[0], ((0.5f32 * 0.1) / 4.0 + 2.0) - 0.05);
        assert!((times[0] - 1.9625).abs() < 1e-6);
        assert!((times[3] - 2.0375).abs() < 1e-6);
        // The frame's first sub-frame index shifts the slot.
        assert_eq!(
            motion_blur_time(0, 2, 0.1, 4, 2.0),
            motion_blur_time(2, 0, 0.1, 4, 2.0)
        );
        // No shutter: every sub-frame is the clock.
        assert_eq!(motion_blur_time(3, 0, 0.0, 4, 2.0), 2.0);
    }

    #[test]
    fn an_effect_takes_its_parent_matrix_unless_the_material_forbids_it() {
        let own = [[1.0; 4]; 4];
        let parent = [[2.0; 4]; 4];
        assert_eq!(effect_world_matrix(Some(own), Some(parent), 0), Some(own));
        assert_eq!(effect_world_matrix(None, Some(parent), 0), Some(parent));
        assert_eq!(effect_world_matrix(None, Some(parent), 0x20), None);
        assert_eq!(effect_world_matrix(None, None, 0), None);
    }

    #[test]
    fn the_scene_begin_empties_the_slots_and_keeps_the_uniforms() {
        let lights = [light(1)];
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.lights = Some(&lights);
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert_eq!(plan.lights, 1);
        state.begin_scene();
        assert_eq!(state.slots().count, 0);
        assert_eq!(state.slots().environment & 0xFF, 0);
    }
}

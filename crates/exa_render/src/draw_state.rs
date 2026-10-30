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

/// A draw object's flag word bit `0x8000`: the node is animated, so the draw
/// uploads a vertex-delta stream and the engine changes its cull state.
pub const OBJECT_VERTEX_DELTA: u32 = 0x8000;

/// A draw object's flag word bit `0x10`: the node's animation clock advances
/// with each super-sampling sub-frame.
pub const OBJECT_SUBFRAME_CLOCK: u32 = 0x10;

/// A draw object's flag word bit `0x800000`: the node takes the specular
/// voxel-GI lookup.
pub const OBJECT_GI_SPECULAR: u32 = 0x800000;

/// A geometry record's flag word bit `0x10`: the material's emissive is
/// rescaled for this draw.
pub const EMISSIVE_PER_DRAW: u32 = 0x10;

/// A geometry record's flag word bit `0x20`: the draw rewrites each
/// sub-frame projection's depth lane with the geometry's shutter offset.
pub const MOTION_BLUR: u32 = 0x20;

/// An object's frame word bit `0x2000_0000`: the diffuse alpha is
/// the frame's fade, `1 − fade`.
pub const FRAME_FADE: u32 = 0x2000_0000;

/// A material word bit (`TexDif`, the diffuse map): a draw's per-draw
/// emissive scale is the scene's when the material has this bit, else the
/// geometry record's own.
pub const MATERIAL_DIFFUSE_MAP: u32 = 0x2_0000;

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
    /// The record's flag word: [`EMISSIVE_PER_DRAW`] and [`MOTION_BLUR`].
    pub flags: u32,
    /// The per-draw emissive scale (the record's `0xf4` field).
    pub emissive: f32,
    /// The geometry's shutter length (the record's `0xf8` field), the range
    /// of the sub-frame depth offsets.
    pub shutter: f32,
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
    /// The render object's flag word (its `0xc` field):
    /// [`OBJECT_SUBFRAME_CLOCK`], [`OBJECT_VERTEX_DELTA`] and
    /// [`OBJECT_GI_SPECULAR`].
    pub object_flags: u32,
    /// The object's world matrix (its `0x40` field), row-major.
    pub world: [[f32; 4]; 4],
    /// The liquid-level point in the object's space (its `0x390` field).
    pub liquid_point: [f32; 3],
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
            geometry: GeometryRecord {
                flags: 0,
                emissive: 1.0,
                shutter: 0.0,
            },
            object_flags: 0,
            world: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
            liquid_point: [0.0; 3],
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
    /// The scene's emissive scale, used by an object whose render-flag word
    /// has [`EMISSIVE_SCENE_SCALE`].
    pub emissive_scale: f32,
    /// The view's sub-frame clock: the frame's time, the sample count and the
    /// accumulation quarter.
    pub clock: SubframeClock,
    /// The render state's bit 2 (the engine's global drawing word): every
    /// draw's object word gets the vertex-colour bit and the shaders' colour
    /// stream is declared. The bit is a global option outside this pack; it
    /// is false in play [guess].
    pub force_vertex_colors: bool,
}

/// The inputs of the view's per-sub-frame animation clocks.
#[derive(Clone, Copy, Debug)]
pub struct SubframeClock {
    /// The frame's time (the view's `0x1dd4` field), the shutter length.
    pub frame_time: f32,
    /// The super-sampling setting's sample count.
    pub samples: u32,
    /// The accumulation quarter being drawn.
    pub quarter: u32,
}

impl Default for SubframeClock {
    fn default() -> Self {
        Self {
            frame_time: 0.0,
            samples: 1,
            quarter: 0,
        }
    }
}

impl SubframeClock {
    /// The step between two samples' clocks.
    pub fn step(&self) -> f32 {
        crate::subframe::animation_step(self.frame_time, self.samples)
    }

    /// The first sample's clock in this quarter.
    pub fn base(&self) -> f32 {
        crate::subframe::animation_base(self.quarter, self.frame_time, self.samples)
    }

    /// The sub-frame's animation clock.
    pub fn time(&self, subframe: u32) -> f32 {
        crate::subframe::animation_clock(subframe, self.quarter, self.frame_time, self.samples)
    }
}

impl DrawScene {
    /// Point the scene at one of a super-sampling setting's quarters: the
    /// pass count, the clock's sample count and quarter, and the view's
    /// sun-shadow mode for that quarter.
    pub fn set_quarter(&mut self, setting: u32, quarter: u32) {
        let samples = crate::subframe::sample_count(setting);
        self.passes = crate::subframe::passes(setting);
        self.clock.samples = samples;
        self.clock.quarter = quarter;
        self.sun_shadow_mode = crate::subframe::shadow_mode(setting, quarter);
    }
}

impl Default for DrawScene {
    fn default() -> Self {
        Self {
            slot: SlotScene::default(),
            passes: 4,
            frame_pass: 0,
            sun_shadow_mode: 0,
            emissive_scale: 1.0,
            clock: SubframeClock::default(),
            force_vertex_colors: false,
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
    /// The object's animation clock for this sub-frame, for an object whose
    /// flag word has [`OBJECT_SUBFRAME_CLOCK`]; `None` otherwise.
    pub time: Option<f32>,
    /// The whole-pixel viewport shift of the sub-frame.
    pub viewport_shift: [i32; 2],
    /// The shutter length of a geometry with [`MOTION_BLUR`]: the sub-frame's
    /// projection depth lane is rewritten to
    /// `subframe::depth_lane(base, slot, duration, samples)` and the
    /// composite rebuilt (`Subframes::apply_depth_offset`).
    pub depth_shutter: Option<f32>,
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
    /// The `LqdLvl` value (the liquid point's world height), for a material
    /// word with the refracting bit on the super-sampled path.
    pub liquid_level: Option<f32>,
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

    /// The scene begin: empty every light slot and drop the
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
        // The drawing state keeps one "uniforms are stale" flag: binding
        // another program raises it and leaves it raised even when this draw
        // has no lights to upload, so the next lit draw does.
        self.slots.stale |= program_changed;
        let lit = object.material & 6 != 0;
        let gathered = match object.lights {
            Some(lights) if lit => self.slots.fill(lights),
            _ => {
                self.slots.clear();
                0
            }
        };
        let mut object_word = object.frame_object;
        if object.vertex_colors || scene.force_vertex_colors {
            object_word |= crate::shader_program::object::VERTEX_COLOR;
        }
        if object.object_flags & OBJECT_VERTEX_DELTA != 0 {
            object_word |= crate::shader_program::object::VERTEX_DELTA;
        }
        let environment = self.slots.environment;
        // The plain path uploads the lights only for a lit material; the
        // surface path whenever any light is gathered. Both upload when the
        // state's stale flag is raised.
        let upload_lights = match path {
            DrawPath::Supersampled => gathered > 0,
            DrawPath::Plain => lit && gathered > 0,
        } && self.slots.stale;
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
                Some(if object.material & MATERIAL_DIFFUSE_MAP != 0 {
                    scene.emissive_scale
                } else {
                    object.geometry.emissive
                })
            } else {
                None
            };
        // The liquid point's world height, uploaded only by the
        // super-sampled path.
        let liquid_level = (path == DrawPath::Supersampled && object.material & 0x200 != 0)
            .then(|| crate::subframe::transform_point(&object.world, object.liquid_point)[1]);
        let quad = object_word & crate::shader_program::object::QUAD_RENDER != 0;
        let samples = scene.slot.shadow_samples;
        let passes = match path {
            DrawPath::Supersampled => {
                draw_passes(scene.passes, object.frame, scene.sun_shadow_mode)
            }
            DrawPath::Plain => 1,
        };
        let depth_shutter = (path == DrawPath::Supersampled
            && object.geometry.flags & MOTION_BLUR != 0)
            .then_some(object.geometry.shutter);
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
                && object.object_flags & OBJECT_SUBFRAME_CLOCK != 0)
                .then(|| scene.clock.time(index));
            subframes.push(Subframe {
                index,
                taps,
                time,
                viewport_shift: crate::subframe::viewport_shift(index),
                depth_shutter,
            });
        }
        DrawPlan {
            object_word,
            environment,
            lights: gathered,
            upload_lights,
            fade_alpha,
            emissive_scale,
            liquid_level,
            gi_position: environment & 0x10 != 0,
            subframes,
            plain_taps,
        }
    }
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
        assert_eq!(plan.liquid_level, None);
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
        object.object_flags = OBJECT_SUBFRAME_CLOCK;
        let mut scene = scene();
        scene.clock = SubframeClock {
            frame_time: 0.1,
            samples: 4,
            quarter: 0,
        };
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.subframes[0].time, Some(scene.clock.time(0)));
        assert_eq!(plan.subframes[2].time, Some(scene.clock.time(2)));
        // Four samples over the 0.1 s frame put the clocks at -0.0375,
        // -0.0125, 0.0125 and 0.0375.
        assert!((plan.subframes[0].time.unwrap() + 0.0375).abs() < 1e-7);
        assert!((plan.subframes[3].time.unwrap() - 0.0375).abs() < 1e-7);
        // The plain path draws once and sets no sub-frame clock.
        let plan = state.prepare(&object, &scene, DrawPath::Plain, false);
        assert_eq!(plan.subframes[0].time, None);
        // A static object has none.
        object.object_flags = 0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert!(plan.subframes.iter().all(|s| s.time.is_none()));
        // The next quarter continues the clock sequence.
        scene.clock.quarter = 1;
        object.object_flags = OBJECT_SUBFRAME_CLOCK;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert!((plan.subframes[0].time.unwrap() - 0.0625).abs() < 1e-7);
    }

    #[test]
    fn the_object_word_takes_the_streams_the_geometry_has() {
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.vertex_colors = true;
        object.object_flags = OBJECT_VERTEX_DELTA;
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
        object.geometry.emissive = 0.25;
        let mut scene = scene();
        scene.emissive_scale = 2.0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.emissive_scale, Some(0.25));
        object.material = 0x8 | MATERIAL_DIFFUSE_MAP;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.emissive_scale, Some(2.0));
        // Only a record with the emissive bit is rescaled.
        object.material = 0x7 | MATERIAL_DIFFUSE_MAP;
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
        object.world = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [10.0, 20.0, 30.0, 1.0],
        ];
        object.liquid_point = [1.0, 2.0, 3.0];
        let mut scene = scene();
        scene.slot.base_environment = 0x10;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        // The liquid point's world height.
        assert_eq!(plan.liquid_level, Some(22.0));
        assert!(plan.gi_position);
        // The plain path never uploads the liquid level.
        let plan = state.prepare(&object, &scene, DrawPath::Plain, false);
        assert_eq!(plan.liquid_level, None);
        object.material = 0x7;
        scene.slot.base_environment = 0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert_eq!(plan.liquid_level, None);
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
    fn a_scene_takes_its_quarter_from_the_setting() {
        let mut scene = DrawScene::default();
        scene.set_quarter(3, 0);
        assert_eq!(scene.passes, 4);
        assert_eq!(scene.clock.samples, 4);
        assert_eq!(scene.clock.quarter, 0);
        assert_eq!(scene.sun_shadow_mode, 3);
        scene.set_quarter(2, 0);
        assert_eq!(scene.passes, 3);
        assert_eq!(scene.clock.samples, 3);
        scene.set_quarter(7, 3);
        assert_eq!(scene.passes, 4);
        assert_eq!(scene.clock.samples, 66);
        assert_eq!(scene.clock.quarter, 3);
        assert_eq!(scene.sun_shadow_mode, 2);
    }

    #[test]
    fn a_motion_blurred_geometry_carries_its_shutter_length() {
        let mut state = DrawState::new(scene().slot);
        let mut object = object();
        object.geometry.flags = MOTION_BLUR;
        object.geometry.shutter = 0.1;
        let mut scene = scene();
        scene.clock = SubframeClock {
            frame_time: 0.1,
            samples: 4,
            quarter: 0,
        };
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert!(plan.subframes.iter().all(|s| s.depth_shutter == Some(0.1)));
        // The lane's value: (((i + 0.5)·0.1)/4 + base) − 0.05.
        let base = -1.0;
        assert_eq!(
            crate::subframe::depth_lane(base, 0, 0.1, 4),
            ((0.5f32 * 0.1) / 4.0 + base) - 0.05
        );
        // A geometry without the flag, and the plain path, take no shutter.
        object.geometry.flags = 0;
        let plan = state.prepare(&object, &scene, DrawPath::Supersampled, false);
        assert!(plan.subframes.iter().all(|s| s.depth_shutter.is_none()));
        object.geometry.flags = MOTION_BLUR;
        let plan = state.prepare(&object, &scene, DrawPath::Plain, false);
        assert!(plan.subframes.iter().all(|s| s.depth_shutter.is_none()));
    }

    #[test]
    fn a_render_state_that_forces_vertex_colours_sets_the_object_bit() {
        let mut state = DrawState::new(scene().slot);
        let mut scene = scene();
        scene.force_vertex_colors = true;
        let plan = state.prepare(&object(), &scene, DrawPath::Supersampled, false);
        assert_ne!(
            plan.object_word & crate::shader_program::object::VERTEX_COLOR,
            0
        );
        scene.force_vertex_colors = false;
        let plan = state.prepare(&object(), &scene, DrawPath::Supersampled, false);
        assert_eq!(
            plan.object_word & crate::shader_program::object::VERTEX_COLOR,
            0
        );
    }

    #[test]
    fn the_subframes_carry_their_viewport_shifts() {
        let mut state = DrawState::new(scene().slot);
        let plan = state.prepare(&object(), &scene(), DrawPath::Supersampled, false);
        let shifts: Vec<[i32; 2]> = plan.subframes.iter().map(|s| s.viewport_shift).collect();
        assert_eq!(shifts, vec![[0, 0], [1, 0], [0, 1], [1, 1]]);
        let plan = state.prepare(&object(), &scene(), DrawPath::Plain, false);
        assert_eq!(plan.subframes[0].viewport_shift, [0, 0]);
    }

    #[test]
    fn a_program_change_with_no_lights_keeps_the_uniforms_stale() {
        let mut state = DrawState::new(scene().slot);
        let lights = [light(1)];
        let mut object = object();
        // A program change with no light list gathers nothing, so nothing is
        // uploaded and the stale flag stays raised.
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, true);
        assert_eq!(plan.lights, 0);
        assert!(!plan.upload_lights);
        assert!(state.lights_stale());
        // The next lit draw uploads.
        object.lights = Some(&lights);
        let plan = state.prepare(&object, &scene(), DrawPath::Supersampled, false);
        assert!(plan.upload_lights);
        assert!(!state.lights_stale());
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

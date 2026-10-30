//! The flame drawn along a lit blade.
//!
//! A burning held item carries a flame row: a line from 30 to 115 cm along
//! the item's own +Y, drawn with the `_FlameMat01` shader and lit by a warm
//! light at the middle of the line. Igniting sets the fade-in bit; the row
//! fades in by 0.02 a tick to one and clears the bit. Extinguishing sets the
//! fade-out bit; the row loses 0.015 a tick and is removed once it is below
//! 0.01 (the fades are taken in double precision). The ribbon's lagging
//! points follow the line's ends, each tick closing a fifth of the gap to a
//! point 20 cm above its end, so the flame leans up and trails the swing.
//! The light's flicker is a soft-bounded random walk scaled by 0.06, with a
//! separate 0.87–1.15 softness walk; both run only while the row has a
//! light, and the loop sound's pitch walk only while it has a sound.
//!
//! The carried torch is not a flame row: it uses its model's own fire
//! emitter (`crate::torch`). A flame row needs a held item whose ability
//! word is a burning one, which Exarust's item records do not carry yet.
//!
//! Sizes are the original's centimetres and its 83 Hz ticks.

use bevy::prelude::*;

use crate::arena::Mt19937;
use crate::effects::{Frame, round_i32};
use crate::thaum::engine::{length, normalize};

/// One tick's lerp of the original's helper: `a + (b − a)·t`.
fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
    a + (b - a) * t
}

/// The flame starts this far along the item's local +Y.
pub const START_CM: f32 = 30.0;
/// And ends here.
pub const END_CM: f32 = 115.0;

/// Fade in per tick while lit, a double.
pub const FADE_IN_PER_TICK: f64 = 0.02;
/// Fade out per tick while extinguishing, a double.
pub const FADE_OUT_PER_TICK: f64 = 0.015;
/// The lit flag: the row is fading in.
pub const FADING_IN: u32 = 1;
/// The extinguish flag: the row is fading out.
pub const EXTINGUISHING: u32 = 2;
/// The removed flag.
pub const REMOVED: u32 = 4;
/// An extinguished row leaves the scene once its fade, widened to double,
/// is below this.
pub const REMOVE_BELOW: f64 = 0.01;

/// The lagging points aim this far above the line's ends, on the up axis.
pub const TIP_RISE_CM: f32 = 20.0;
/// And close this share of the gap each tick.
pub const TIP_SMOOTHING: f32 = 0.2;
/// The speed's smoothing: a tenth of the new move, nine tenths of the old
/// speed, in double precision.
pub const SPEED_NEW: f64 = 0.1;
pub const SPEED_OLD: f64 = 0.9;

/// The light's colour, sRGB.
pub const LIGHT_COLOUR: [f32; 3] = [1.0, 0.68, 0.43];
/// Its radius and cast softness.
pub const LIGHT_RADIUS_CM: f32 = 800.0;
pub const LIGHT_SOFTNESS: f32 = 4.0;
/// Intensity at a full fade before the flicker.
pub const LIGHT_BASE_INTENSITY: f32 = 5.0;
/// The intensity the light object is created with.
pub const LIGHT_START_INTENSITY: f32 = 0.01;
/// The walk's share of the intensity, a double.
pub const LIGHT_WALK_SCALE: f64 = 0.06;
/// How far the walk moves its speed each tick.
pub const LIGHT_FLICKER_STEP: f32 = 1.0;
/// The softness walk's step and bounds (compared in double precision).
pub const JITTER_STEP: f64 = 0.125;
pub const JITTER_MIN: f32 = 0.87;
pub const JITTER_MAX: f32 = 1.15;
/// The swing follows the speed by this share, a double.
pub const SWING_SCALE: f64 = 0.3;
/// The flags the row raises on its light each frame.
pub const LIGHT_UPDATE_FLAGS: u32 = 0xb;

/// The loop sound (`FireS02`) follows the row's speed and fade. It starts
/// at volume 0 and pitch 1.
pub const LOOP_SOUND: &str = "FireS02";
pub const SOUND_VOLUME_SCALE: f64 = 0.001;
pub const SOUND_VOLUME_MIN: f32 = 0.03;
pub const SOUND_VOLUME_MAX: f32 = 1.0;
pub const SOUND_VOLUME_GAIN: f32 = 2.0;
/// The loop's pitch walk, its bounds and the 0.8 the stored pitch is then
/// multiplied by, all in double precision.
pub const PITCH_STEP: f64 = 0.005;
pub const PITCH_MIN: f32 = 0.85;
pub const PITCH_MAX: f32 = 1.15;
pub const PITCH_SCALE: f64 = 0.8;

/// The sizzle of a blade being lit: volume 0.3, pitch 0.8–1.0.
pub const SIZZLE_SOUND: &str = "FireSizzleA";
pub const SIZZLE_VOLUME: f32 = 0.3;
pub const SIZZLE_PITCH_BASE: f32 = 0.8;
pub const SIZZLE_PITCH_SPAN: f32 = 0.2;

/// The item render node's flag while its flame row burns.
pub const LIT_RENDER_FLAG: u32 = 0x800;
/// The body byte set on ignition.
pub const BODY_LIT_BYTE: bool = true;
/// The moveset modifier a lit item switches on.
pub const FLAMING_MODIFIER: u32 = 0x1002_0000;
/// The item ability type this path serves.
pub const BURNING_ABILITY: u32 = 3;
/// The ability word's low five bits of a burning item.
pub const BURNING_KIND: u32 = 4;
/// An item with this much fuel left (or more) no longer burns.
pub const FUEL_EMPTY: u8 = 0xfe;

/// The row's object kind.
pub const ROW_KIND: u32 = 0x2120_0000;
/// A row is found through the top twelve bits of its kind.
pub const ROW_MASK: u32 = 0xfff0_0000;
/// Extinguishing finds the flame child through the top byte.
pub const EXTINGUISH_MASK: u32 = 0xff00_0000;
pub const EXTINGUISH_MATCH: u32 = 0x2100_0000;
/// The light's kind, flags and emitter flag bits, as created.
pub const LIGHT_KIND: i32 = 3;
pub const LIGHT_FLAGS: u32 = 0x0014_0100;
pub const LIGHT_EMITTER_FLAGS: u32 = 0x9000;
pub const LIGHT_EXTRA_ONE: bool = true;
pub const LIGHT_EXTRA_TWO: u32 = 1;

/// The mesh level of detail: 200 over the squared camera distance, at
/// least one.
pub const LOD_BASE: f32 = 200.0;

/// A row's fade also drives the attached item's emissive sub-object: the
/// first whose material name starts with `*`, whose emissive amount is set
/// and whose material flag is raised.
pub const EMISSIVE_PREFIX: char = '*';
pub const EMISSIVE_FLAG: u32 = 0x10;

/// The index of the first emissive sub-object, if the item has one.
pub fn emissive_part(names: &[&str]) -> Option<usize> {
    names
        .iter()
        .position(|name| name.starts_with(EMISSIVE_PREFIX))
}

/// The shader's two colours.
pub const ELE_COLOUR: [f32; 3] = [0.65, 0.4, 1.0];
pub const EDGE_COLOUR: [f32; 3] = [0.08, 0.14, 0.2];
/// The row's material and texture.
pub const MATERIAL: &str = "_FlameMat01";
/// The texture animation runs over 50 s, in double precision.
pub const ANIM_PERIOD_MS: u32 = 50_000;
pub const ANIM_STEP: f64 = 0.000_02;
/// The ribbon is built from this many sections.
pub const SECTIONS: i32 = 16;
/// The width of one section, as a share of the line's length over the
/// section count.
pub const SECTION_WIDTH: f32 = 1.5;
/// A section's texture step along the ribbon.
pub const SECTION_U: f32 = 0.025;
/// The vertex colours of each four-vertex quad's corners, RGBA8: two clear
/// black, then two white with zero alpha. The fragment squares the first
/// channel, so the pair tells the ribbon's two faces apart.
pub const QUAD_COLOURS: [[u8; 4]; 4] = [
    [0, 0, 0, 0],
    [0, 0, 0, 0],
    [255, 255, 255, 0],
    [255, 255, 255, 0],
];

/// The ribbon's colour buffer: [`QUAD_COLOURS`] repeated for every quad.
pub fn ribbon_colours() -> Vec<[u8; 4]> {
    (0..SECTIONS * 2).flat_map(|_| QUAD_COLOURS).collect()
}

/// The shader's vertex attributes, in the original's declaration order.
pub const SHADER_ATTRIBUTES: [&str; 3] = ["vPos", "vTex", "vCol"];
/// The texture units `EmiTex` and `GrdTex` bind to.
pub const EMISSIVE_UNIT: i32 = 0;
pub const GRADIENT_UNIT: i32 = 1;
/// The direction toward the lagging line is scaled by this before its
/// length is taken; the scale cancels in the normalisation but keeps the
/// original's rounding.
pub const LAG_DIRECTION_SCALE: f32 = 0.05;
/// The ribbon's cross-section wobble: this share of the interpolated line's
/// length (taken in double and rounded once), times one less the
/// direction's face-on weight.
pub const WOBBLE_SCALE: f64 = 0.2;

/// The original's dot product: `(x·x + y·y) + z·z`.
fn dot(a: Vec3, b: Vec3) -> f32 {
    (a.x * b.x + a.y * b.y) + a.z * b.z
}

/// The ribbon's index list: each section's two quads of four vertices,
/// split into two triangles `(0, 1, 3)` and `(3, 1, 2)`.
pub fn ribbon_indices() -> Vec<u16> {
    (0..(SECTIONS * 2) as u16)
        .flat_map(|q| {
            let b = q * 4;
            [b, b + 1, b + 3, b + 3, b + 1, b + 2]
        })
        .collect()
}

/// The ribbon's texture coordinates: per section, a quad from the
/// section's step back by one step and one forward, each from v 0 to 1.
pub fn ribbon_uvs() -> Vec<[f32; 2]> {
    (0..SECTIONS)
        .flat_map(|i| {
            let u = i as f32 * SECTION_U;
            [
                [u, 0.0],
                [u, 1.0],
                [u - SECTION_U, 1.0],
                [u - SECTION_U, 0.0],
                [u, 0.0],
                [u, 1.0],
                [u + SECTION_U, 1.0],
                [u + SECTION_U, 0.0],
            ]
        })
        .collect()
}

/// What a held item's fire state asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeldFire {
    /// Nothing changes.
    None,
    /// Light the blade.
    Ignite,
    /// Put it out.
    Extinguish,
}

/// The answer of the per-tick held-item check: whether the item was found
/// with its record, and what to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeldTick {
    pub found: bool,
    pub action: HeldFire,
}

/// A held item's ability section: its type word and the fuel byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemAbility {
    pub kind: u32,
    pub fuel: u8,
}

/// Whether the item has fire left: its low five bits are four and the fuel
/// byte is below 254.
pub fn burns(ability: ItemAbility) -> bool {
    ability.kind & 0x1f == BURNING_KIND && ability.fuel < FUEL_EMPTY
}

/// The row attached to a body: whether one exists and whether it is on its
/// way out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AttachedRow {
    pub exists: bool,
    pub extinguishing: bool,
}

impl AttachedRow {
    pub const NONE: Self = Self {
        exists: false,
        extinguishing: false,
    };
}

/// The per-tick check of one held item. A missing item or record does
/// nothing. A burning item with no live row is lit; a row on its way out is
/// replaced. A cold item puts a live row out.
pub fn held_tick(ability: Option<ItemAbility>, row: AttachedRow) -> HeldTick {
    let Some(ability) = ability else {
        return HeldTick {
            found: false,
            action: HeldFire::None,
        };
    };
    let live = row.exists && !row.extinguishing;
    let action = if burns(ability) {
        if live {
            HeldFire::None
        } else {
            HeldFire::Ignite
        }
    } else if live {
        HeldFire::Extinguish
    } else {
        HeldFire::None
    };
    HeldTick {
        found: true,
        action,
    }
}

/// The attachment a new flame row gets when an item is lit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ignition {
    /// The segment's ends, in the item's own space.
    pub start_cm: Vec3,
    pub end_cm: Vec3,
    /// The row's flags: fading in.
    pub flags: u32,
    /// The item render node's lit flag, and the body byte ignition sets.
    pub render_lit: bool,
    pub body_flag: bool,
    /// The moveset modifier.
    pub modifier: u32,
}

/// What lighting a held item creates.
pub fn ignition() -> Ignition {
    Ignition {
        start_cm: Vec3::new(0.0, START_CM, 0.0),
        end_cm: Vec3::new(0.0, END_CM, 0.0),
        flags: FADING_IN,
        render_lit: true,
        body_flag: BODY_LIT_BYTE,
        modifier: FLAMING_MODIFIER,
    }
}

/// The first flame row among a body's children, and whether the row it
/// finds is on its way out. A row of any other kind is skipped. The scan
/// stops at the first row: a live one blocks a new one, an extinguishing
/// one is replaced.
pub fn find_row(children: &[(u32, u32)]) -> AttachedRow {
    for &(kind, flags) in children {
        if kind & ROW_MASK == ROW_KIND {
            return AttachedRow {
                exists: true,
                extinguishing: flags & EXTINGUISHING != 0,
            };
        }
    }
    AttachedRow::NONE
}

/// The index of the first flame child an extinguish finds: the kind's top
/// byte is 0x21.
pub fn find_extinguish_row(children: &[(u32, u32)]) -> Option<usize> {
    children
        .iter()
        .position(|&(kind, _)| kind & EXTINGUISH_MASK == EXTINGUISH_MATCH)
}

/// The light a row shows this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowLight {
    pub intensity: f32,
    pub softness: f32,
    pub radius_cm: f32,
    pub colour: [f32; 3],
}

/// The loop sound's volume and pitch this frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowSound {
    pub volume: f32,
    pub pitch: f32,
}

/// The per-tick state of a flame row.
#[derive(Debug, Clone)]
pub struct FlameRow {
    pub flags: u32,
    /// Fade, 0..1.
    pub fade: f32,
    /// The segment's ends in the item's own space (cm).
    pub start_local: Vec3,
    pub end_local: Vec3,
    /// Their world positions this tick and last.
    pub start: Vec3,
    pub end: Vec3,
    pub prev_start: Vec3,
    pub prev_end: Vec3,
    /// The lagging points the ribbon follows, and their previous state.
    pub lag_start: Vec3,
    pub lag_end: Vec3,
    pub older_lag_start: Vec3,
    pub older_lag_end: Vec3,
    /// The ends the ribbon interpolates between over a frame: the pair
    /// shifted out last and the pair before it (start, end).
    pub ribbon_prev: [Vec3; 2],
    pub ribbon_old: [Vec3; 2],
    /// Whether the row has a light and a loop sound: the light's walks and
    /// the sound's pitch walk run only with them.
    pub has_light: bool,
    pub has_sound: bool,
    /// Smoothed speed of the ends, cm per tick.
    pub speed: f32,
    /// The light's random walk, speed and its softness jitter.
    pub walk: f32,
    pub swing: f32,
    pub jitter: f32,
    /// The loop sound's pitch: the value the sound is started with (one)
    /// and the per-frame walk stores.
    pub pitch: f32,
    /// The seeded stream the light and sound walks draw from.
    pub rng: Mt19937,
}

impl FlameRow {
    /// A row with the given segment and flags.
    pub fn new(start_local: Vec3, end_local: Vec3, flags: u32, seed: u32) -> Self {
        Self {
            flags,
            fade: 0.0,
            start_local,
            end_local,
            start: Vec3::ZERO,
            end: Vec3::ZERO,
            prev_start: Vec3::ZERO,
            prev_end: Vec3::ZERO,
            lag_start: Vec3::ZERO,
            lag_end: Vec3::ZERO,
            older_lag_start: Vec3::ZERO,
            older_lag_end: Vec3::ZERO,
            ribbon_prev: [Vec3::ZERO; 2],
            ribbon_old: [Vec3::ZERO; 2],
            has_light: true,
            has_sound: true,
            speed: 0.0,
            walk: 0.0,
            swing: 0.0,
            jitter: 0.0,
            pitch: 1.0,
            rng: Mt19937::new(seed),
        }
    }

    /// The flame a freshly lit item creates.
    pub fn lit(seed: u32) -> Self {
        let light = ignition();
        Self::new(light.start_cm, light.end_cm, light.flags, seed)
    }

    /// Attach the row to its item: the ends from the item's frame, and
    /// every remembered and lagging point set to them.
    pub fn attach(&mut self, frame: &Frame) {
        self.start = frame.point(self.start_local);
        self.end = frame.point(self.end_local);
        self.prev_start = self.start;
        self.prev_end = self.end;
        self.lag_start = self.start;
        self.older_lag_start = self.lag_start;
        self.lag_end = self.end;
        self.older_lag_end = self.lag_end;
    }

    /// The pair shift before a tick: the ribbon's older ends take the
    /// previous ones, and the previous ones take the line's ends.
    pub fn shift_ribbon(&mut self) {
        self.ribbon_old = self.ribbon_prev;
        self.ribbon_prev = [self.start, self.end];
    }

    /// The ribbon's ends `t` of the way through the frame.
    pub fn ribbon_ends(&self, t: f32) -> (Vec3, Vec3) {
        (
            lerp(self.ribbon_old[0], self.ribbon_prev[0], t),
            lerp(self.ribbon_old[1], self.ribbon_prev[1], t),
        )
    }

    /// The row's bounds `t` of the way through the frame: the middle of the
    /// line (where its light and sound sit) and half the ribbon's length.
    pub fn bounds(&self, t: f32) -> (Vec3, f32) {
        let (a, b) = self.ribbon_ends(t);
        let d = a - b;
        let length = ((d.x * d.x + d.z * d.z) + d.y * d.y).sqrt();
        (lerp(self.start, self.end, 0.5), length * 0.5)
    }

    /// The row's ribbon this frame: eight vertices a section, from the
    /// interpolated pair of ends toward the interpolated lagging line, seen
    /// from `camera_cm`. The first four carry the section's two faces swept
    /// one way, the next four the same pair swept the other.
    ///
    /// The original builds this in single precision; a zero direction or a
    /// camera on the line divides by zero and gives a not-a-number, as the
    /// original's does.
    pub fn ribbon_positions(&self, camera_cm: Vec3, t: f32) -> Vec<[f32; 3]> {
        let (pair_start, pair_end) = self.ribbon_ends(t);
        let line = length(pair_start - pair_end);
        let width = line / SECTIONS as f32 * SECTION_WIDTH;
        let lag_start = lerp(self.older_lag_start, self.lag_start, t);
        let lag_end = lerp(self.older_lag_end, self.lag_end, t);
        let delta_start = (lag_start - pair_start) * LAG_DIRECTION_SCALE;
        let delta_end = (lag_end - pair_end) * LAG_DIRECTION_SCALE;
        let wobble_span = (f64::from(line) * WOBBLE_SCALE) as f32;
        let mut out = Vec::with_capacity((SECTIONS * 8) as usize);
        for i in 0..SECTIONS {
            let section = (i as f32 + 0.5) / SECTIONS as f32;
            let point = lerp(pair_start, pair_end, section);
            let mut direction = lerp(delta_start, delta_end, section);
            let direction_length = length(direction);
            direction *= 1.0 / direction_length;
            let to_camera = normalize(point - camera_cm);
            let across = direction.cross(to_camera);
            let side = to_camera.cross(across);
            let across_length = length(across);
            let half_width = width / across_length;
            let face = across * half_width;
            let weight = 1.0 - dot(direction, to_camera).abs();
            let wobble = (direction_length * wobble_span) * weight;
            let left = point + side * -half_width;
            let right = point + side * (half_width * 2.0 + wobble);
            let near_left = left - face;
            let near_right = right - face;
            let far_right = right + face;
            let far_left = left + face;
            for v in [
                left, right, near_right, near_left, left, right, far_right, far_left,
            ] {
                out.push(v.to_array());
            }
        }
        out
    }

    /// The row's mesh this frame: the positions from [`Self::ribbon_positions`],
    /// the fixed texture coordinates, colours and indices, in metres for the
    /// play renderer.
    pub fn ribbon_mesh(&self, camera_cm: Vec3, t: f32) -> Mesh {
        use bevy::asset::RenderAssetUsages;
        use bevy::mesh::{Indices, Mesh, PrimitiveTopology};

        let positions: Vec<[f32; 3]> = self
            .ribbon_positions(camera_cm, t)
            .into_iter()
            .map(|p| [p[0] * 0.01, p[1] * 0.01, p[2] * 0.01])
            .collect();
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, ribbon_uvs());
        // The vertex stage's `vCol · 0.003921568`: an RGBA8 byte over 255.
        let colours: Vec<[f32; 4]> = ribbon_colours()
            .into_iter()
            .map(|c| {
                [
                    f32::from(c[0]) * 0.003_921_568,
                    f32::from(c[1]) * 0.003_921_568,
                    f32::from(c[2]) * 0.003_921_568,
                    f32::from(c[3]) * 0.003_921_568,
                ]
            })
            .collect();
        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colours);
        mesh.insert_indices(Indices::U16(ribbon_indices()));
        mesh
    }

    /// Whether the row has been removed.
    pub fn removed(&self) -> bool {
        self.flags & REMOVED != 0
    }

    /// One tick: fade, the ends from the attachment, the lag, the speed
    /// and, with a light, its walks. `random` holds the walk, swing and
    /// jitter units, in that draw order (drawn only with a light).
    pub fn tick(&mut self, attach: Option<&Frame>, random: [f64; 3]) -> RowLight {
        if self.removed() {
            return self.light();
        }
        if self.flags & EXTINGUISHING != 0 {
            self.fade = (f64::from(self.fade) - FADE_OUT_PER_TICK) as f32;
            if self.fade < 0.0 {
                self.fade = 0.0;
            }
        } else if self.flags & FADING_IN != 0 {
            self.fade = (f64::from(self.fade) + FADE_IN_PER_TICK) as f32;
            if self.fade > 1.0 {
                self.fade = 1.0;
                self.flags &= !FADING_IN;
            }
        }
        self.prev_start = self.start;
        self.prev_end = self.end;
        if let Some(attach) = attach {
            self.start = attach.point(self.start_local);
            self.end = attach.point(self.end_local);
        }
        self.older_lag_start = self.lag_start;
        self.older_lag_end = self.lag_end;
        self.lag_start = lag_step(self.start, self.lag_start);
        self.lag_end = lag_step(self.end, self.lag_end);
        // The original measures squared distances and smooths them as they
        // are; the larger is kept unless the first is a NaN.
        let a = crate::effects::distance_squared(self.prev_start, self.start);
        let b = crate::effects::distance_squared(self.prev_end, self.end);
        let moved = if a < b { b } else { a };
        self.speed = (f64::from(moved) * SPEED_NEW + f64::from(self.speed) * SPEED_OLD) as f32;

        if self.has_light {
            self.walk = walk_step(self.walk, self.swing, random[0], LIGHT_FLICKER_STEP);
            self.swing =
                (f64::from(self.walk) * SWING_SCALE * random[1] + f64::from(self.swing)) as f32;
            // A zero-initialised jitter clamps to 0.87 on the first tick
            // (guess: the class does not write the field elsewhere).
            self.jitter = jitter_step(self.jitter, random[2]);
        }
        self.light()
    }

    /// The light from the current fade and walks: the intensity in double
    /// precision, `fade · ((swing · 0.06 + 1) · base)`.
    pub fn light(&self) -> RowLight {
        let gain =
            (f64::from(self.swing) * LIGHT_WALK_SCALE + 1.0) * f64::from(LIGHT_BASE_INTENSITY);
        RowLight {
            intensity: (f64::from(self.fade) * gain) as f32,
            softness: LIGHT_SOFTNESS * self.jitter,
            radius_cm: LIGHT_RADIUS_CM,
            colour: LIGHT_COLOUR,
        }
    }

    /// One frame: remove a faded-out row (its flags become the removed
    /// flag alone) and, with a sound, step the loop's pitch. `pitch_unit` is
    /// drawn only with a sound.
    pub fn frame(&mut self, pitch_unit: f64) -> Option<RowSound> {
        if self.removed() {
            return None;
        }
        if self.flags & EXTINGUISHING != 0 && f64::from(self.fade) < REMOVE_BELOW {
            self.flags = REMOVED;
            return None;
        }
        if !self.has_sound {
            return None;
        }
        let mut pitch = ((pitch_unit - 0.5) * PITCH_STEP + f64::from(self.pitch)) as f32;
        if f64::from(pitch) < 0.85 {
            pitch = PITCH_MIN;
        } else if f64::from(pitch) > 1.15 {
            pitch = PITCH_MAX;
        }
        self.pitch = (f64::from(pitch) * PITCH_SCALE) as f32;
        Some(self.sound())
    }

    /// The loop sound's volume and pitch from the current speed and fade.
    pub fn sound(&self) -> RowSound {
        let mut volume = (f64::from(self.speed) * SOUND_VOLUME_SCALE) as f32;
        if volume > SOUND_VOLUME_MAX {
            volume = SOUND_VOLUME_MAX;
        } else if f64::from(volume) < 0.03 {
            volume = SOUND_VOLUME_MIN;
        }
        volume = volume * self.fade * SOUND_VOLUME_GAIN;
        if self.flags & FADING_IN != 0 {
            volume = (volume + 0.5) - (self.fade - 0.5).abs();
        }
        RowSound {
            volume,
            pitch: self.pitch,
        }
    }
}

/// One lagging point: close a fifth of the gap to a point 20 cm above the
/// target on the up axis.
fn lag_step(target: Vec3, lag: Vec3) -> Vec3 {
    let mut delta = target - lag;
    delta.y += TIP_RISE_CM;
    lag + delta * TIP_SMOOTHING
}

/// The light's soft-bounded speed walk. Its arithmetic is in double, as
/// the original's, then narrowed back to a float.
pub fn walk_step(walk: f32, swing: f32, unit: f64, step: f32) -> f32 {
    let next = if 1.0 < swing || 1.0 < walk {
        f64::from(walk) - f64::from(step) * unit
    } else if -1.0 > swing || -1.0 > walk {
        f64::from(step) * unit + f64::from(walk)
    } else {
        // The one is added to the step in single precision.
        f64::from(1.0 + step) * (unit - 0.5) + f64::from(walk)
    };
    next as f32
}

/// The softness jitter's walk, compared with 0.87 and 1.15 in double
/// precision and clamped to their single-precision values.
pub fn jitter_step(jitter: f32, unit: f64) -> f32 {
    let jitter = ((unit - 0.5) * JITTER_STEP + f64::from(jitter)) as f32;
    if f64::from(jitter) < 0.87 {
        JITTER_MIN
    } else if f64::from(jitter) > 1.15 {
        JITTER_MAX
    } else {
        jitter
    }
}

/// The world-space corner a camera-facing ribbon build starts from, for
/// the play renderer. The original builds 16 sections of four vertices.
pub fn section_count() -> i32 {
    SECTIONS
}

/// The width of a section: 1.5 times the line over the section count.
pub fn section_width(line: f32) -> f32 {
    line / SECTIONS as f32 * SECTION_WIDTH
}

/// A row component, stepped by [`tick_rows`] on the fixed tick and finished
/// by [`frame_rows`].
#[derive(Component, Debug, Clone)]
pub struct Flame(FlameRow);

impl Flame {
    pub fn new(start_local: Vec3, end_local: Vec3, flags: u32, seed: u32) -> Self {
        Self(FlameRow::new(start_local, end_local, flags, seed))
    }

    pub fn row(&self) -> &FlameRow {
        &self.0
    }
}

/// The attach frame of a row from its world transform: centimetres in,
/// centimetres out, so the basis keeps its scale and only the translation
/// turns from metres to centimetres.
pub fn attach_frame(transform: &GlobalTransform) -> Frame {
    let (scale, rotation, translation) = transform.to_scale_rotation_translation();
    Frame {
        x: rotation * Vec3::X * scale.x,
        y: rotation * Vec3::Y * scale.y,
        z: rotation * Vec3::Z * scale.z,
        at: translation * 100.0,
    }
}

/// A row that has taken its first ends from its item.
#[derive(Component)]
pub struct Attached;

#[allow(clippy::type_complexity)]
pub fn tick_rows(
    mut commands: Commands,
    mut rows: Query<(Entity, &mut Flame, Option<&GlobalTransform>, Has<Attached>)>,
) {
    for (entity, mut flame, transform, attached) in &mut rows {
        let frame = transform.map(attach_frame);
        let row = &mut flame.0;
        if !attached && let Some(frame) = &frame {
            row.attach(frame);
            commands.entity(entity).insert(Attached);
        }
        // The pair shift runs before the tick (guess: its caller is outside
        // this group).
        row.shift_ribbon();
        let random = if row.has_light {
            [row.rng.unit(), row.rng.unit(), row.rng.unit()]
        } else {
            [0.5; 3]
        };
        row.tick(frame.as_ref(), random);
    }
}

/// One frame of each row: a faded-out row leaves the scene; a live one
/// steps its loop's pitch and shows its light.
pub fn frame_rows(
    mut commands: Commands,
    mut rows: Query<(Entity, &mut Flame, Option<&mut PointLight>)>,
) {
    for (entity, mut flame, light) in &mut rows {
        let row = &mut flame.0;
        if row.removed() {
            continue;
        }
        let unit = if row.has_sound { row.rng.unit() } else { 0.5 };
        row.frame(unit);
        if row.removed() {
            commands.entity(entity).despawn();
            continue;
        }
        if let Some(mut light) = light {
            let shown = row.light();
            light.intensity = shown.intensity.max(0.0) * exa_world::LUMENS_PER_INTENSITY;
            light.radius = exa_world::shadow_radius(shown.softness);
        }
    }
}

/// The path a row's light adds, as the play renderer takes it.
pub fn light_record() -> exa_formats::rfc::Light {
    exa_formats::rfc::Light {
        flags: LIGHT_FLAGS,
        color: LIGHT_COLOUR,
        multiplier: 1.0,
        intensity: LIGHT_START_INTENSITY,
        radius: LIGHT_RADIUS_CM / 100.0,
        softness: LIGHT_SOFTNESS,
        spot: None,
        ranged: true,
        animation: None,
    }
}

/// The sizzle's pitch for a random unit: `u·0.2 + 0.8` in double
/// precision.
pub fn sizzle_pitch(unit: f64) -> f32 {
    (unit * f64::from(SIZZLE_PITCH_SPAN) + f64::from(SIZZLE_PITCH_BASE)) as f32
}

/// The mesh level of detail the play renderer stores for the row: 200
/// over the squared camera distance (cm), raised to one.
pub fn level_of_detail(camera: Vec3, at: Vec3) -> f32 {
    crate::effects::level_of_detail(LOD_BASE, camera, at)
}

/// The sizzle of a hit: volume 0.3 and a pitch from 0.8 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sizzle {
    pub volume: f32,
    pub pitch: f32,
}

/// One sizzle from a random unit.
pub fn sizzle(unit: f64) -> Sizzle {
    Sizzle {
        volume: SIZZLE_VOLUME,
        pitch: sizzle_pitch(unit),
    }
}

/// Put out the first flame row a body carries: set its extinguish bit.
pub fn extinguish(children: &mut [(u32, u32)]) -> Option<usize> {
    let index = find_extinguish_row(children)?;
    children[index].1 |= EXTINGUISHING;
    Some(index)
}

/// The item render flags with its lit flag raised.
pub fn lit_render(flags: u32) -> u32 {
    flags | LIT_RENDER_FLAG
}

/// The shader's `Anim` values this frame: the rolling texture offset, its
/// negative twin, and the fade's other half.
pub fn texture_animation(now_ms: u32, fade: f32) -> [f32; 3] {
    let offset = (f64::from(now_ms % ANIM_PERIOD_MS) * ANIM_STEP) as f32;
    [offset, offset * -2.0, (1.0 - fade) * 0.5]
}

/// The millisecond timestamps a row's phase boundaries land on.
pub fn deadline_ms(now_ms: u32, seconds: f32) -> u32 {
    now_ms.wrapping_add(round_i32(seconds * 1000.0) as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_lit_row_fades_in_by_two_hundredths_and_clears_its_flag() {
        let mut row = FlameRow::lit(1);
        assert_eq!(row.fade, 0.0);
        let light = row.tick(None, [0.5, 0.5, 0.5]);
        assert!((light.intensity - 0.1).abs() < 1e-6);
        assert_eq!(row.fade, 0.02);
        // The double 0.02 added to the stored float and narrowed back does
        // not reach one: 50 ticks leave 0.99999964 and the fifty-first
        // passes the clamp.
        for _ in 0..49 {
            row.tick(None, [0.5, 0.5, 0.5]);
        }
        assert_eq!(row.fade.to_bits(), 0.999_999_64f32.to_bits());
        assert_eq!(row.flags & FADING_IN, FADING_IN);
        row.tick(None, [0.5, 0.5, 0.5]);
        assert_eq!(row.fade, 1.0);
        assert_eq!(row.flags & FADING_IN, 0);
        row.tick(None, [0.5, 0.5, 0.5]);
        assert_eq!(row.fade, 1.0);
    }

    #[test]
    fn an_extinguished_row_fades_out_and_leaves_below_one_percent() {
        let mut row = FlameRow::lit(1);
        row.fade = 1.0;
        row.flags = EXTINGUISHING;
        let mut ticks = 0;
        while row.fade > 0.0 {
            row.tick(None, [0.5, 0.5, 0.5]);
            ticks += 1;
            assert!(ticks < 200);
        }
        assert_eq!(ticks, 67);
        assert!(!row.removed());
        assert!(row.frame(0.5).is_none());
        assert!(row.removed());
        // The flags become the removed flag alone.
        assert_eq!(row.flags, REMOVED);
        // 66 ticks leave 0.0100007, still at the removal line in double
        // precision: the row is drawn one more frame.
        let mut row = FlameRow::lit(1);
        row.fade = 1.0;
        row.flags = EXTINGUISHING;
        for _ in 0..66 {
            row.tick(None, [0.5, 0.5, 0.5]);
        }
        assert!((row.fade - 0.010_000_7).abs() < 1e-7, "{}", row.fade);
        assert!(row.frame(0.5).is_some());
        // A float 0.01 is below the double 0.01 and removes the row.
        row.fade = 0.01;
        assert!(row.frame(0.5).is_none());
    }

    #[test]
    fn the_lag_closes_a_fifth_of_the_gap_to_twenty_centimetres_up() {
        let mut row = FlameRow::lit(7);
        row.start = Vec3::new(10.0, 0.0, 0.0);
        row.end = Vec3::new(10.0, 100.0, 0.0);
        row.lag_start = Vec3::ZERO;
        row.lag_end = Vec3::ZERO;
        row.tick(None, [0.5, 0.5, 0.5]);
        assert_eq!(row.lag_start, Vec3::new(2.0, 4.0, 0.0));
        assert_eq!(row.lag_end, Vec3::new(2.0, 24.0, 0.0));
        assert_eq!(row.older_lag_start, Vec3::ZERO);
        row.tick(None, [0.5, 0.5, 0.5]);
        assert!((row.lag_start.x - 3.6).abs() < 1e-5);
        assert!((row.lag_start.y - 7.2).abs() < 1e-5);
        assert_eq!(row.older_lag_start, Vec3::new(2.0, 4.0, 0.0));
    }

    #[test]
    fn an_attached_row_starts_at_rest_on_its_item() {
        let mut row = FlameRow::lit(7);
        let frame = Frame {
            at: Vec3::new(5.0, 0.0, 0.0),
            ..Frame::IDENTITY
        };
        row.attach(&frame);
        assert_eq!(row.start, Vec3::new(5.0, 30.0, 0.0));
        assert_eq!(row.end, Vec3::new(5.0, 115.0, 0.0));
        assert_eq!((row.prev_start, row.prev_end), (row.start, row.end));
        assert_eq!((row.lag_start, row.lag_end), (row.start, row.end));
        assert_eq!(
            (row.older_lag_start, row.older_lag_end),
            (row.start, row.end)
        );
        // Still on the next tick: no speed.
        row.tick(Some(&frame), [0.5, 0.5, 0.5]);
        assert_eq!(row.speed, 0.0);
    }

    #[test]
    fn the_ribbon_interpolates_the_shifted_ends() {
        let mut row = FlameRow::lit(7);
        row.start = Vec3::new(0.0, 0.0, 0.0);
        row.end = Vec3::new(0.0, 10.0, 0.0);
        row.shift_ribbon();
        row.start = Vec3::new(4.0, 0.0, 0.0);
        row.end = Vec3::new(4.0, 10.0, 0.0);
        row.shift_ribbon();
        assert_eq!(row.ribbon_old, [Vec3::ZERO, Vec3::new(0.0, 10.0, 0.0)]);
        let (a, b) = row.ribbon_ends(0.5);
        assert_eq!(
            (a, b),
            (Vec3::new(2.0, 0.0, 0.0), Vec3::new(2.0, 10.0, 0.0))
        );
        let (centre, radius) = row.bounds(0.5);
        assert_eq!((centre, radius), (Vec3::new(4.0, 5.0, 0.0), 5.0));
        let indices = ribbon_indices();
        assert_eq!(indices.len(), 192);
        assert_eq!(&indices[..12], &[0, 1, 3, 3, 1, 2, 4, 5, 7, 7, 5, 6]);
        let uvs = ribbon_uvs();
        assert_eq!(uvs.len(), 128);
        assert_eq!(uvs[2], [-0.025, 1.0]);
        assert_eq!(uvs[14], [0.05, 1.0]);
        assert_eq!(QUAD_COLOURS[2], [255, 255, 255, 0]);
        let colours = ribbon_colours();
        assert_eq!(colours.len(), 128);
        assert_eq!(&colours[..4], &QUAD_COLOURS);
        assert_eq!(&colours[128 - 4..], &QUAD_COLOURS);
    }

    #[test]
    fn a_row_without_a_light_does_not_walk() {
        let mut row = FlameRow::lit(1);
        row.has_light = false;
        row.tick(None, [1.0, 1.0, 1.0]);
        assert_eq!((row.walk, row.swing, row.jitter), (0.0, 0.0, 0.0));
        row.has_sound = false;
        assert!(row.frame(1.0).is_none());
        assert_eq!(row.pitch, 1.0);
    }

    #[test]
    fn the_ribbon_sections_sweep_between_the_pair_and_the_lag() {
        let mut row = FlameRow::lit(7);
        // The pair's ends after the frame's half step: 0 and 8 cm up. The
        // lagging end has swung 320 cm along +x, so a section's direction
        // is +x and the section at 0.25 cm is level with the camera.
        row.ribbon_old = [Vec3::ZERO, Vec3::ZERO];
        row.ribbon_prev = [Vec3::ZERO, Vec3::new(0.0, 16.0, 0.0)];
        row.older_lag_start = Vec3::ZERO;
        row.lag_start = Vec3::ZERO;
        row.older_lag_end = Vec3::new(320.0, 8.0, 0.0);
        row.lag_end = Vec3::new(320.0, 8.0, 0.0);
        let positions = row.ribbon_positions(Vec3::new(0.0, 0.25, -1000.0), 0.5);
        assert_eq!(positions.len(), 128);
        // Section 0's point is (0, 0.25, 0); its direction (1, 0, 0), the
        // line 8 cm, the width 0.75 and the wobble 0.5·1.6.
        let want = [
            [-0.75, 0.25, 0.0],
            [2.3, 0.25, 0.0],
            [2.3, 1.0, 0.0],
            [-0.75, 1.0, 0.0],
            [-0.75, 0.25, 0.0],
            [2.3, 0.25, 0.0],
            [2.3, -0.5, 0.0],
            [-0.75, -0.5, 0.0],
        ];
        for (got, want) in positions[..8].iter().zip(want) {
            let got = Vec3::from_array(*got);
            assert!(
                (got - Vec3::from_array(want)).length() < 2e-3,
                "{got} != {want:?}"
            );
        }
        // A camera on the line makes the cross product zero; the original
        // divides by zero there and the vertices are not numbers.
        let broken = row.ribbon_positions(Vec3::new(0.0, 0.25, 0.0), 0.5);
        assert!(!broken[0][0].is_finite());
    }

    #[test]
    fn the_ribbon_mesh_carries_the_built_positions_and_fixed_attributes() {
        let mut row = FlameRow::lit(7);
        row.ribbon_prev = [Vec3::ZERO, Vec3::new(0.0, 16.0, 0.0)];
        let mesh = row.ribbon_mesh(Vec3::new(0.0, 8.0, -1000.0), 1.0);
        let positions = mesh.attribute(Mesh::ATTRIBUTE_POSITION).unwrap();
        assert_eq!(positions.len(), 128);
        assert_eq!(mesh.attribute(Mesh::ATTRIBUTE_UV_0).unwrap().len(), 128);
        assert_eq!(mesh.attribute(Mesh::ATTRIBUTE_COLOR).unwrap().len(), 128);
        assert_eq!(mesh.indices().unwrap().len(), 192);
        // The mesh is in metres: the line runs from 0 to 0.16 m.
        if let bevy::mesh::VertexAttributeValues::Float32x3(p) = positions {
            assert!(p.iter().flatten().all(|v| v.abs() < 1.0));
            assert!(p.iter().any(|v| (v[1] - 0.16).abs() < 0.02));
        } else {
            panic!("positions are not floats");
        }
    }

    #[test]
    fn the_light_walks_within_its_soft_bounds() {
        assert_eq!(walk_step(0.0, 0.0, 0.5, 1.0), 0.0);
        assert_eq!(walk_step(0.0, 0.0, 1.0, 1.0), 1.0);
        assert_eq!(walk_step(1.5, 0.0, 0.25, 1.0), 1.25);
        assert_eq!(walk_step(-1.5, 0.0, 0.25, 1.0), -1.25);
        assert_eq!(walk_step(0.0, 2.0, 0.25, 1.0), -0.25);
        assert_eq!(jitter_step(0.0, 0.0), JITTER_MIN);
        assert_eq!(jitter_step(1.0, 0.0), 0.9375);
        assert_eq!(jitter_step(1.14, 1.0), JITTER_MAX);
        assert_eq!(jitter_step(1.0, 1.0), 1.0625);
    }

    #[test]
    fn the_speed_is_the_smoothed_squared_move_of_the_ends() {
        let mut row = FlameRow::lit(7);
        row.start_local = Vec3::new(3.0, 4.0, 0.0);
        row.end_local = Vec3::ZERO;
        row.tick(Some(&Frame::IDENTITY), [0.5, 0.5, 0.5]);
        assert_eq!(row.speed, 2.5);
        row.tick(Some(&Frame::IDENTITY), [0.5, 0.5, 0.5]);
        assert_eq!(row.speed, 2.25);
    }

    #[test]
    fn a_flickering_light_is_the_fade_walk_and_base() {
        let mut row = FlameRow::lit(1);
        row.fade = 1.0;
        row.flags = 0;
        row.jitter = 1.0;
        row.swing = 0.0;
        let light = row.tick(None, [0.5, 0.5, 0.5]);
        assert_eq!(light.intensity, LIGHT_BASE_INTENSITY);
        assert_eq!(light.softness, 4.0);
        assert_eq!(light.radius_cm, 800.0);
        assert_eq!(light.colour, [1.0, 0.68, 0.43]);
        row.swing = 0.5;
        assert!((row.light().intensity - 5.15).abs() < 1e-5);
        row.jitter = JITTER_MIN;
        assert_eq!(row.light().softness, 3.48);
    }

    #[test]
    fn the_loop_volume_follows_speed_and_fade() {
        let mut row = FlameRow::lit(1);
        row.fade = 1.0;
        row.flags = 0;
        row.speed = 0.0;
        let sound = row.sound();
        assert_eq!(sound.volume, 0.06);
        row.speed = 500.0;
        assert_eq!(row.sound().volume, 1.0);
        row.speed = 100.0;
        assert!((row.sound().volume - 0.2).abs() < 1e-6);
        row.flags = FADING_IN;
        assert!((row.sound().volume - 0.2).abs() < 1e-6);
        row.fade = 0.5;
        assert_eq!(row.sound().volume, 0.6);
    }

    #[test]
    fn the_loop_pitch_clamps_then_operates_at_eight_tenths() {
        let mut row = FlameRow::lit(1);
        row.pitch = 1.0;
        let sound = row.frame(0.5).unwrap();
        assert_eq!(sound.pitch, 0.8);
        let mut row = FlameRow::lit(1);
        row.pitch = 1.0;
        assert!((row.frame(1.0).unwrap().pitch - 0.802).abs() < 1e-6);
        let mut row = FlameRow::lit(1);
        row.pitch = 1.0;
        row.frame(0.0);
        row.frame(0.0);
        assert_eq!(row.pitch, 0.68);
    }

    #[test]
    fn an_item_burns_only_while_its_kind_and_fuel_allow() {
        assert!(burns(ItemAbility {
            kind: BURNING_KIND,
            fuel: 0
        }));
        assert!(burns(ItemAbility {
            kind: 0x24,
            fuel: 0xfd
        }));
        assert!(!burns(ItemAbility {
            kind: BURNING_KIND,
            fuel: FUEL_EMPTY
        }));
        assert!(!burns(ItemAbility { kind: 5, fuel: 0 }));
        assert_eq!(BURNING_ABILITY, 3);
    }

    #[test]
    fn a_held_item_lights_once_and_goes_out_when_cold() {
        let hot = Some(ItemAbility {
            kind: BURNING_KIND,
            fuel: 1,
        });
        assert!(!held_tick(None, AttachedRow::NONE).found);
        assert_eq!(held_tick(hot, AttachedRow::NONE).action, HeldFire::Ignite);
        assert_eq!(
            held_tick(
                hot,
                AttachedRow {
                    exists: true,
                    extinguishing: false
                }
            )
            .action,
            HeldFire::None
        );
        assert_eq!(
            held_tick(
                hot,
                AttachedRow {
                    exists: true,
                    extinguishing: true
                }
            )
            .action,
            HeldFire::Ignite
        );
        let cold = Some(ItemAbility { kind: 5, fuel: 1 });
        assert_eq!(
            held_tick(
                cold,
                AttachedRow {
                    exists: true,
                    extinguishing: false
                }
            )
            .action,
            HeldFire::Extinguish
        );
        assert_eq!(
            held_tick(
                cold,
                AttachedRow {
                    exists: true,
                    extinguishing: true
                }
            )
            .action,
            HeldFire::None
        );
    }

    #[test]
    fn a_lit_item_hangs_its_flame_from_thirty_to_115_centimetres() {
        let light = ignition();
        assert_eq!(light.start_cm, Vec3::new(0.0, 30.0, 0.0));
        assert_eq!(light.end_cm, Vec3::new(0.0, 115.0, 0.0));
        assert_eq!(light.flags, FADING_IN);
        assert!(light.render_lit && light.body_flag);
        assert_eq!(light.modifier, 0x1002_0000);
        assert_eq!(sizzle_pitch(0.0), 0.8);
        assert_eq!(sizzle_pitch(1.0), 1.0);
        assert_eq!(SIZZLE_VOLUME, 0.3);
        assert_eq!(LOOP_SOUND, "FireS02");
        assert_eq!(MATERIAL, "_FlameMat01");
        assert_eq!(SHADER_ATTRIBUTES, ["vPos", "vTex", "vCol"]);
        assert_eq!((EMISSIVE_UNIT, GRADIENT_UNIT), (0, 1));
        assert_eq!(
            exa_render::flame_row::GRADIENT_PALETTE[4],
            [255, 230, 200, 64]
        );
        assert_eq!(emissive_part(&["steel", "*glow", "*other"]), Some(1));
        assert_eq!(emissive_part(&["steel"]), None);
        assert_eq!(EMISSIVE_FLAG, 0x10);
        assert_eq!(ELE_COLOUR, [0.65, 0.4, 1.0]);
        assert_eq!(EDGE_COLOUR, [0.08, 0.14, 0.2]);
        assert_eq!(section_count(), 16);
        assert_eq!(section_width(100.0), 9.375);
    }

    #[test]
    fn a_row_is_found_by_its_kind_and_extinguish_uses_the_top_byte() {
        let rows = [(0x1111_0000, 0), (ROW_KIND, 0), (ROW_KIND, EXTINGUISHING)];
        assert_eq!(
            find_row(&rows),
            AttachedRow {
                exists: true,
                extinguishing: false
            }
        );
        assert_eq!(find_row(&rows[..1]), AttachedRow::NONE);
        assert_eq!(find_extinguish_row(&rows), Some(1));
        assert_eq!(find_extinguish_row(&[(0x1111_0000, 0)]), None);
        assert_eq!(ROW_KIND & EXTINGUISH_MASK, EXTINGUISH_MATCH);
    }

    #[test]
    fn the_row_light_record_is_warm_and_ranged() {
        let light = light_record();
        assert_eq!(light.color, LIGHT_COLOUR);
        assert_eq!(light.radius, 8.0);
        assert_eq!(light.softness, 4.0);
        assert_eq!(light.intensity, 0.01);
        assert!(light.ranged);
        assert_eq!(LIGHT_FLAGS, 0x0014_0100);
        assert_eq!(LIGHT_EMITTER_FLAGS, 0x9000);
        assert_eq!(LIGHT_KIND, 3);
        assert_eq!(LIGHT_EXTRA_TWO, 1);
        assert_eq!(deadline_ms(100, 0.4), 500);
        assert_eq!(LIT_RENDER_FLAG, 0x800);
        assert_eq!(lit_render(0), 0x800);
        assert_eq!(level_of_detail(Vec3::ZERO, Vec3::new(0.5, 0.0, 0.0)), 200.0);
        assert_eq!(level_of_detail(Vec3::ZERO, Vec3::X), 200.0);
        assert_eq!(level_of_detail(Vec3::ZERO, Vec3::new(0.0, 0.0, 4.0)), 12.5);
        assert_eq!(texture_animation(0, 1.0), [0.0, -0.0, 0.0]);
        assert_eq!(texture_animation(25_000, 0.0), [0.5, -1.0, 0.5]);
        assert_eq!(texture_animation(50_000, 1.0), [0.0, -0.0, 0.0]);
        let sizzle = sizzle(0.5);
        assert_eq!(sizzle.volume, 0.3);
        assert!((sizzle.pitch - 0.9).abs() < 1e-6);
        let mut children = vec![(0x1111_0000, 0), (ROW_KIND, 0)];
        assert_eq!(extinguish(&mut children), Some(1));
        assert_eq!(children[1].1, EXTINGUISHING);
    }

    #[test]
    fn a_row_attach_frame_is_in_centimetres() {
        let transform = GlobalTransform::from_xyz(1.0, 2.0, 3.0);
        let frame = attach_frame(&transform);
        assert_eq!(frame.at, Vec3::new(100.0, 200.0, 300.0));
        // A local centimetre stays a centimetre.
        assert_eq!(frame.x, Vec3::X);
        assert_eq!(
            frame.point(Vec3::new(0.0, 30.0, 0.0)),
            Vec3::new(100.0, 230.0, 300.0)
        );
    }
}

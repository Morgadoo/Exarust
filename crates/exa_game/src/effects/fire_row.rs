//! The flame drawn along a lit blade.
//!
//! A burning held item carries a flame row: a line from 30 to 115 cm along
//! the item's own +Y, drawn with the `_FlameMat01` shader and lit by a warm
//! light at the middle of the line. Igniting sets the fade-in bit; the row
//! fades in by 0.02 a tick to one and clears the bit. Extinguishing sets the
//! fade-out bit; the row loses 0.015 a tick and is removed once it is below
//! 0.01. The line's ends lag behind the item (the tip's points rise 4 cm a
//! tick and close a fifth of the gap), and the light's flicker is a
//! soft-bounded random walk scaled by 0.06, with a separate 0.87–1.15
//! softness walk.
//!
//! The same row is what a lit torch-like effect is built from, but the
//! carried torch uses its model's own fire emitter instead.
//!
//! Sizes are the original's centimetres and its 83 Hz ticks.

use bevy::prelude::*;

use crate::arena::Mt19937;
use crate::effects::{Frame, round_i32};

/// The flame starts this far along the item's local +Y.
pub const START_CM: f32 = 30.0;
/// And ends here.
pub const END_CM: f32 = 115.0;

/// Fade in per tick while lit.
pub const FADE_IN_PER_TICK: f32 = 0.02;
/// Fade out per tick while extinguishing.
pub const FADE_OUT_PER_TICK: f32 = 0.015;
/// The lit flag: the row is fading in.
pub const FADING_IN: u32 = 1;
/// The extinguish flag: the row is fading out.
pub const EXTINGUISHING: u32 = 2;
/// The removed flag.
pub const REMOVED: u32 = 4;
/// An extinguished row leaves the scene once its fade is below this.
pub const REMOVE_BELOW: f32 = 0.01;

/// The tip's points rise this much per tick (a fifth of it: 4 cm).
pub const TIP_RISE_CM: f32 = 20.0;
/// And close this share of the gap to the true end each tick.
pub const TIP_SMOOTHING: f32 = 0.2;

/// The light's colour, sRGB.
pub const LIGHT_COLOUR: [f32; 3] = [1.0, 0.68, 0.43];
/// Its radius and cast softness.
pub const LIGHT_RADIUS_CM: f32 = 800.0;
pub const LIGHT_SOFTNESS: f32 = 4.0;
/// Intensity at a full fade before the flicker.
pub const LIGHT_BASE_INTENSITY: f32 = 5.0;
/// The intensity the light object is created with.
pub const LIGHT_START_INTENSITY: f32 = 0.01;
/// The walk's share of the intensity.
pub const LIGHT_WALK_SCALE: f32 = 0.06;
/// How far the walk moves its speed each tick.
pub const LIGHT_FLICKER_STEP: f32 = 1.0;
/// The softness walk's step and bounds.
pub const JITTER_STEP: f32 = 0.125;
pub const JITTER_MIN: f32 = 0.87;
pub const JITTER_MAX: f32 = 1.15;
/// The swing follows the speed by this share.
pub const SWING_SCALE: f32 = 0.3;

/// The loop sound (`FireS02`) follows the row's speed and fade.
pub const LOOP_SOUND: &str = "FireS02";
pub const SOUND_VOLUME_SCALE: f32 = 0.001;
pub const SOUND_VOLUME_MIN: f32 = 0.03;
pub const SOUND_VOLUME_MAX: f32 = 1.0;
pub const SOUND_VOLUME_GAIN: f32 = 2.0;
/// The loop's pitch walk and its 0.8 operating scale.
pub const PITCH_STEP: f32 = 0.005;
pub const PITCH_MIN: f32 = 0.85;
pub const PITCH_MAX: f32 = 1.15;
pub const PITCH_SCALE: f32 = 0.8;

/// The sizzle of a blade being lit: volume 0.3, pitch 0.8–1.0.
pub const SIZZLE_SOUND: &str = "FireSizzleA";
pub const SIZZLE_VOLUME: f32 = 0.3;
pub const SIZZLE_PITCH_BASE: f32 = 0.8;
pub const SIZZLE_PITCH_SPAN: f32 = 0.2;

/// The body's render flag while it holds a lit item.
pub const BODY_LIT_FLAG: u32 = 0x800;
/// The moveset modifier a lit item switches on.
pub const FLAMING_MODIFIER: u32 = 0x1002_0000;
/// The item flag byte set on ignition.
pub const ITEM_LIT: bool = true;

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

/// The shader's two colours.
pub const ELE_COLOUR: [f32; 3] = [0.65, 0.4, 1.0];
pub const EDGE_COLOUR: [f32; 3] = [0.08, 0.14, 0.2];
/// The row's material and texture.
pub const MATERIAL: &str = "_FlameMat01";
/// The texture animation runs over 50 s.
pub const ANIM_PERIOD_MS: u32 = 50_000;
pub const ANIM_STEP: f32 = 0.000_02;
/// The ribbon is built from this many sections.
pub const SECTIONS: i32 = 16;
/// The width of one section, as a share of the line.
pub const SECTION_WIDTH: f32 = 1.5;

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
    /// The body's render flag and the item's lit byte.
    pub body_lit: bool,
    pub item_lit: bool,
    /// The moveset modifier.
    pub modifier: u32,
}

/// What lighting a held item creates.
pub fn ignition() -> Ignition {
    Ignition {
        start_cm: Vec3::new(0.0, START_CM, 0.0),
        end_cm: Vec3::new(0.0, END_CM, 0.0),
        flags: FADING_IN,
        body_lit: true,
        item_lit: ITEM_LIT,
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
    /// Smoothed speed of the ends, cm per tick.
    pub speed: f32,
    /// The light's random walk, speed and its softness jitter.
    pub walk: f32,
    pub swing: f32,
    pub jitter: f32,
    /// The loop sound's pitch.
    pub pitch: f32,
    /// The seeded stream the light and sound walks draw from.
    pub rng: Mt19937,
}

impl FlameRow {
    /// A row with the given segment, lit by default.
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

    /// Whether the row has been removed.
    pub fn removed(&self) -> bool {
        self.flags & REMOVED != 0
    }

    /// One tick: fade, the ends from the attachment, the tip lag, the speed
    /// and the light's walk. `random` holds the walk, swing and jitter
    /// units, in that draw order.
    pub fn tick(&mut self, attach: Option<&Frame>, random: [f32; 3]) -> RowLight {
        if self.removed() {
            return self.light();
        }
        if self.flags & EXTINGUISHING != 0 {
            self.fade -= FADE_OUT_PER_TICK;
            if self.fade < 0.0 {
                self.fade = 0.0;
            }
        } else if self.flags & FADING_IN != 0 {
            self.fade += FADE_IN_PER_TICK;
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
        // The original measures squared distances and smooths them as they are.
        let moved = (self.prev_start - self.start)
            .length_squared()
            .max((self.prev_end - self.end).length_squared());
        self.speed = moved * 0.1 + self.speed * 0.9;

        self.walk = walk_step(self.walk, self.swing, random[0], LIGHT_FLICKER_STEP);
        self.swing = (f64::from(self.walk) * f64::from(SWING_SCALE) * f64::from(random[1])
            + f64::from(self.swing)) as f32;
        self.jitter = jitter_step(self.jitter, random[2]);
        self.light()
    }

    /// The light from the current fade and walks.
    pub fn light(&self) -> RowLight {
        RowLight {
            intensity: self.fade * (self.swing * LIGHT_WALK_SCALE + 1.0) * LIGHT_BASE_INTENSITY,
            softness: self.jitter * LIGHT_SOFTNESS,
            radius_cm: LIGHT_RADIUS_CM,
            colour: LIGHT_COLOUR,
        }
    }

    /// One frame: remove a faded-out row and step the loop's pitch.
    pub fn frame(&mut self, pitch_unit: f32) -> Option<RowSound> {
        if self.removed() {
            return None;
        }
        if self.flags & EXTINGUISHING != 0 && self.fade < REMOVE_BELOW {
            self.flags |= REMOVED;
            return None;
        }
        let mut pitch =
            ((f64::from(pitch_unit) - 0.5) * f64::from(PITCH_STEP) + f64::from(self.pitch)) as f32;
        pitch = if pitch >= PITCH_MIN {
            if pitch > PITCH_MAX { PITCH_MAX } else { pitch }
        } else {
            PITCH_MIN
        };
        self.pitch = pitch * PITCH_SCALE;
        Some(self.sound())
    }

    /// The loop sound's volume and pitch from the current speed and fade.
    pub fn sound(&self) -> RowSound {
        let mut volume = self.speed * SOUND_VOLUME_SCALE;
        if volume <= SOUND_VOLUME_MAX {
            if volume < SOUND_VOLUME_MIN {
                volume = SOUND_VOLUME_MIN;
            }
        } else {
            volume = SOUND_VOLUME_MAX;
        }
        volume *= self.fade * SOUND_VOLUME_GAIN;
        if self.flags & FADING_IN != 0 {
            volume = (volume + 0.5) - (self.fade - 0.5).abs();
        }
        RowSound {
            volume,
            pitch: self.pitch,
        }
    }
}

/// One lagging point: rise a fifth of 20 cm and close a fifth of the gap.
fn lag_step(target: Vec3, lag: Vec3) -> Vec3 {
    let mut delta = target - lag;
    delta.z += TIP_RISE_CM;
    lag + delta * TIP_SMOOTHING
}

/// The light's soft-bounded speed walk. Its arithmetic is in double, as
/// the original's, then narrowed back to a float.
pub fn walk_step(walk: f32, swing: f32, unit: f32, step: f32) -> f32 {
    let (walk, unit, step) = (f64::from(walk), f64::from(unit), f64::from(step));
    let next = if f64::from(swing) > 1.0 || walk > 1.0 {
        walk - step * unit
    } else if f64::from(swing) < -1.0 || walk < -1.0 {
        walk + step * unit
    } else {
        (step + 1.0) * (unit - 0.5) + walk
    };
    next as f32
}

/// The softness jitter's walk, clamped to 0.87–1.15.
pub fn jitter_step(jitter: f32, unit: f32) -> f32 {
    let jitter = ((f64::from(unit) - 0.5) * f64::from(JITTER_STEP) + f64::from(jitter)) as f32;
    if jitter >= JITTER_MIN {
        if jitter > JITTER_MAX {
            JITTER_MAX
        } else {
            jitter
        }
    } else {
        JITTER_MIN
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

/// The attach frame of a row from its world transform, in centimetres.
pub fn attach_frame(transform: &GlobalTransform) -> Frame {
    let (scale, rotation, translation) = transform.to_scale_rotation_translation();
    Frame {
        x: rotation * Vec3::X * scale.x * 100.0,
        y: rotation * Vec3::Y * scale.y * 100.0,
        z: rotation * Vec3::Z * scale.z * 100.0,
        at: translation * 100.0,
    }
}

pub fn tick_rows(mut rows: Query<(&mut Flame, Option<&GlobalTransform>)>) {
    for (mut flame, transform) in &mut rows {
        let frame = transform.map(attach_frame);
        let row = &mut flame.0;
        let random = [
            row.rng.unit() as f32,
            row.rng.unit() as f32,
            row.rng.unit() as f32,
        ];
        row.tick(frame.as_ref(), random);
    }
}

#[allow(clippy::type_complexity)]
pub fn frame_rows(mut rows: Query<(&mut Flame, Option<&mut PointLight>)>) {
    for (mut flame, light) in &mut rows {
        let row = &mut flame.0;
        let unit = row.rng.unit() as f32;
        if row.frame(unit).is_none() {
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
    }
}

/// The sizzle's pitch for a random unit: 0.8 + 0.2·u.
pub fn sizzle_pitch(unit: f32) -> f32 {
    SIZZLE_PITCH_BASE + unit * SIZZLE_PITCH_SPAN
}

/// The mesh level of detail the play renderer stores for the row.
pub fn level_of_detail(distance: f32) -> f32 {
    LOD_BASE / (distance * distance).max(1.0)
}

/// The sizzle of a hit: volume 0.3 and a pitch from 0.8 to 1.0.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sizzle {
    pub volume: f32,
    pub pitch: f32,
}

/// One sizzle from a random unit.
pub fn sizzle(unit: f32) -> Sizzle {
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

/// The body render flags with its lit flag raised.
pub fn light_body(flags: u32) -> u32 {
    flags | BODY_LIT_FLAG
}

/// The shader's `Anim` values this frame: the rolling texture offset, its
/// negative twin, and the fade's other half.
pub fn texture_animation(now_ms: u32, fade: f32) -> [f32; 3] {
    let offset = (now_ms % ANIM_PERIOD_MS) as f32 * ANIM_STEP;
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
        assert_eq!(row.fade, FADE_IN_PER_TICK);
        // 0.02 does not add to exactly one in single precision: 50 ticks
        // leave 0.9999996 and the fifty-first passes the clamp.
        for _ in 0..49 {
            row.tick(None, [0.5, 0.5, 0.5]);
        }
        assert_eq!(row.fade.to_bits(), 0.999_999_6f32.to_bits());
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
        // 66 ticks leave 0.0099999, already below the removal line.
        assert!(row.fade < REMOVE_BELOW);
        assert!(row.frame(0.5).is_none());
        assert!(row.removed());
    }

    #[test]
    fn the_tip_lags_and_rises_four_centimetres_a_tick() {
        let mut row = FlameRow::lit(7);
        row.start = Vec3::new(10.0, 0.0, 0.0);
        row.end = Vec3::new(10.0, 0.0, 100.0);
        row.lag_start = Vec3::ZERO;
        row.lag_end = Vec3::ZERO;
        row.tick(None, [0.5, 0.5, 0.5]);
        assert_eq!(row.lag_start, Vec3::new(2.0, 0.0, 4.0));
        assert_eq!(row.lag_end, Vec3::new(2.0, 0.0, 24.0));
        row.tick(None, [0.5, 0.5, 0.5]);
        assert!((row.lag_start.x - 3.6).abs() < 1e-5);
        assert!((row.lag_start.z - 7.2).abs() < 1e-5);
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
        assert!(light.body_lit && light.item_lit);
        assert_eq!(light.modifier, 0x1002_0000);
        assert_eq!(sizzle_pitch(0.0), 0.8);
        assert_eq!(sizzle_pitch(1.0), 1.0);
        assert_eq!(SIZZLE_VOLUME, 0.3);
        assert_eq!(LOOP_SOUND, "FireS02");
        assert_eq!(MATERIAL, "_FlameMat01");
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
        assert_eq!(BODY_LIT_FLAG, 0x800);
        assert_eq!(light_body(0), 0x800);
        assert_eq!(level_of_detail(0.5), 200.0);
        assert_eq!(level_of_detail(1.0), 200.0);
        assert_eq!(level_of_detail(4.0), 12.5);
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
        assert_eq!(frame.x, Vec3::new(100.0, 0.0, 0.0));
        assert_eq!(crate::effects::TICK_MS, 12);
    }
}

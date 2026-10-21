//! The character-creation pickers (`TTonePicker`, `THairColPicker`): a
//! field whose picked point becomes a colour, drawn as coloured quads, with
//! the picker base's marker.
//!
//! - **Tone picker.** A 160 × 160 field drawn as a 2 × 2 checkerboard of
//!   quads over the 3 × 3 tone colours the original builds at start-up
//!   ([`exa_assets::human_skin::tone_field_bytes`]). The picked point's
//!   colour is the creator's skin tone in linear light
//!   ([`exa_assets::human_skin::tone_linear`]); a refresh hands it to the
//!   owner's callback when there is one, else tints the character's skin
//!   materials with it and raises task notice 7.
//! - **Hair picker.** A 160 × 112 field: three columns blending the hair
//!   hue run left to right, repeated in three rows, under a vertical shade
//!   from black to white. The point's colour is the column blend scaled by
//!   `v·½ + ¼`, the marker a grey of `140·v`. With a hair material attached
//!   a refresh also re-shades it through the hair picker's colour routine
//!   (ported on the character generator's side) and raises notice 7.
//!
//! Evidence: both constructors, the press slots, the tone colour, refresh
//! and skin apply, the hair refresh, set, attach and child hooks, the two
//! field draws and the start-up routine that fills the tone table, read
//! from their assembly, with the picker base's pick ([`pick_point`]). The
//! rest of the picker base (`TPickerForm`: the marker draw and the gamepad
//! nudge) and the hair routine that shades the material are not here.

use exa_assets::human_skin::{tone_blend_target, tone_field_bytes, tone_linear};

/// The task notice a picker raises when it changed the character.
pub const PICKER_NOTICE: u32 = 7;
/// What the press slots add to the pick's answer on the field.
pub const PRESS_ANSWER: u32 = 5;
/// What the picker base's pick answers.
pub const PICK_ANSWER: u32 = 3;

/// The hair picker's colour table, sixteen stored colours in rows of four:
/// black, the hue run (dark red, blonde, reddish, dark red), grey and
/// white. The picker blends within the hue run; a point at the field's
/// right edge reads the grey after it with weight 0, and the original
/// reads on unchecked for points the set slot puts outside 0–1.
pub const HAIR_TABLE: [[u8; 3]; 16] = [
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [0, 0, 0],
    [164, 98, 48],
    [170, 130, 82],
    [152, 75, 54],
    [164, 98, 48],
    [128, 128, 128],
    [128, 128, 128],
    [128, 128, 128],
    [128, 128, 128],
    [255, 255, 255],
    [255, 255, 255],
    [255, 255, 255],
    [255, 255, 255],
];
/// Where the hue run starts in [`HAIR_TABLE`].
const HUE_RUN: usize = 4;
/// The marker's grey per unit of the point's `v`.
pub const HAIR_MARKER_SCALE: f32 = 140.0;
/// Where the hair picker's attached control sits below the picker.
pub const ATTACHED_DROP: i32 = 0x78;
/// The attached control's height.
pub const ATTACHED_HEIGHT: i32 = 0x28;
/// The attached control's starting value (the hair routine's third input).
pub const ATTACHED_VALUE: f32 = 0.25;

/// The three form words the tone picker's constructor sets on top of the
/// form's own (the hair picker inherits them).
pub const PICKER_LAYOUT_BITS: u32 = 0x80;
pub const PICKER_STATE_BITS: u32 = 0x40_0010;
pub const PICKER_STYLE_BITS: u32 = 0x400;

/// The quarter-way blend the tone apply uses toward
/// [`tone_blend_target`] on a material whose level is under 0.1.
const TONE_BLEND: f32 = 0.25;

/// A picker's picked point, both coordinates nominally 0–1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickPoint {
    pub u: f32,
    pub v: f32,
}

/// One drawn vertex: its colour and position in the field.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FieldVertex {
    pub colour: [u8; 3],
    pub at: (f32, f32),
}

/// One tone cell: six vertices, two triangles.
pub type FieldQuad = [FieldVertex; 6];

/// The picker base's pick (`TPickerForm`): a point in the field's client
/// coordinates becomes `u = x/width` and `v′ = 1 − y/height`, each worked
/// in double, narrowed and held to 0–1 (a NaN passes). When both differ
/// from the stored point by less than the base's epsilon (just under
/// 2⁻²³) nothing happens;
/// otherwise the point takes `u` and `1 − v′` and the picker refreshes.
/// Answers whether the point moved.
pub fn pick_point(point: &mut PickPoint, x: i32, y: i32, size: (i32, i32)) -> bool {
    let hold = |c: f32| {
        if c < 0.0 {
            0.0
        } else if c > 1.0 {
            1.0
        } else {
            c
        }
    };
    let u = hold((f64::from(x) / f64::from(size.0)) as f32);
    let flipped = hold((1.0 - f64::from(y) / f64::from(size.1)) as f32);
    let epsilon = f32::from_bits(0x33ff_ffd7);
    if (point.u - u).abs() < epsilon && ((point.v - 1.0) + flipped).abs() < epsilon {
        return false;
    }
    point.u = u;
    point.v = 1.0 - flipped;
    true
}

/// The engine's byte blend `a + (b − a)·t`, rounded to even and saturated.
fn byte_mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [0, 1, 2].map(|i| {
        let (a, b) = (f32::from(a[i]), f32::from(b[i]));
        exa_formats::pixel::round_f32(a + (b - a) * t).clamp(0, 255) as u8
    })
}

/// The engine's byte scale `c·s`, rounded to even and saturated.
fn byte_scale(c: [u8; 3], s: f32) -> [u8; 3] {
    c.map(|c| exa_formats::pixel::round_f32(f32::from(c) * s).clamp(0, 255) as u8)
}

/// The engine's colour blend `a + (b − a)·t` per channel.
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [0, 1, 2].map(|i| a[i] + (b[i] - a[i]) * t)
}

/// How a picker change reached the character.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToneTarget {
    /// The owner's callback heard the colour; no notice is raised.
    Callback,
    /// The skin materials were tinted and notice 7 raised.
    Skin,
}

/// The tone picker.
#[derive(Debug, Clone, PartialEq)]
pub struct TonePicker {
    pub point: PickPoint,
    /// The field's size in pixels.
    pub size: (i32, i32),
    pub layout: u32,
    pub state: u32,
    pub style: u32,
    /// The colour the last refresh produced, in linear light.
    pub colour: [f32; 3],
    /// Whether the owner set a colour callback.
    pub callback: bool,
}

impl TonePicker {
    /// The constructor: a 160 × 160 field, its form words and the point at
    /// the middle. The colour stays zero until the first refresh.
    pub fn new() -> Self {
        Self {
            point: PickPoint { u: 0.5, v: 0.5 },
            size: (0xa0, 0xa0),
            layout: PICKER_LAYOUT_BITS,
            state: PICKER_STATE_BITS,
            style: PICKER_STYLE_BITS,
            colour: [0.0; 3],
            callback: false,
        }
    }

    /// The refresh slot: the point's colour, then the owner's callback or
    /// the skin apply.
    pub fn refresh(&mut self) -> ToneTarget {
        self.colour = tone_linear(self.point.u, self.point.v);
        if self.callback {
            ToneTarget::Callback
        } else {
            ToneTarget::Skin
        }
    }

    /// The colour a skin material of the character takes from the apply:
    /// on a material whose level (compared in double) is under 0.1 the
    /// colour moves a quarter of the way to [`tone_blend_target`];
    /// otherwise, and for a NaN level, it is the colour itself.
    pub fn skin_colour(&self, level: f32) -> [f32; 3] {
        if f64::from(level) < 0.1 {
            mix(self.colour, tone_blend_target(), TONE_BLEND)
        } else {
            self.colour
        }
    }

    /// The press slot: a press over the field itself (no control under the
    /// pointer, `at` its client position) picks the point, refreshing when
    /// it moved, and answers the pick's answer with [`PRESS_ANSWER`]; any
    /// other press answers what the form base did.
    pub fn press(&mut self, at: Option<(i32, i32)>, base_answer: u32) -> u32 {
        let Some((x, y)) = at else {
            return base_answer;
        };
        if pick_point(&mut self.point, x, y, self.size) {
            self.refresh();
        }
        PICK_ANSWER | PRESS_ANSWER
    }

    /// The field's four cells, a 2 × 2 grid over the 3 × 3 tone colours at
    /// half the field per cell. A cell whose row and column add up odd is
    /// split top-left/bottom-left/top-right, top-right/bottom-left/
    /// bottom-right; an even cell top-left/bottom-left/bottom-right,
    /// bottom-right/top-right/top-left.
    pub fn quads(&self) -> [FieldQuad; 4] {
        let bytes = tone_field_bytes();
        let half_w = self.size.0 as f32 * 0.5;
        let half_h = self.size.1 as f32 * 0.5;
        let mut quads = [[FieldVertex {
            colour: [0; 3],
            at: (0.0, 0.0),
        }; 6]; 4];
        for row in 0..2usize {
            for column in 0..2usize {
                let vertex = |r: usize, c: usize| FieldVertex {
                    colour: bytes[r][c],
                    at: (c as f32 * half_w, r as f32 * half_h),
                };
                let (tl, tr) = (vertex(row, column), vertex(row, column + 1));
                let (bl, br) = (vertex(row + 1, column), vertex(row + 1, column + 1));
                quads[row * 2 + column] = if (row + column) % 2 == 1 {
                    [tl, bl, tr, tr, bl, br]
                } else {
                    [tl, bl, br, br, tr, tl]
                };
            }
        }
        quads
    }
}

impl Default for TonePicker {
    fn default() -> Self {
        Self::new()
    }
}

/// The control a hair picker attaches below itself (its third input).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AttachedControl {
    /// x, y, width and height.
    pub place: (i32, i32, i32, i32),
    /// The control's value: the hair routine's third input.
    pub value: f32,
    /// The control shows the picker's colour record and calls the
    /// picker's child refresh on a change.
    pub linked: bool,
}

/// The hair-colour picker.
#[derive(Debug, Clone, PartialEq)]
pub struct HairColPicker {
    pub point: PickPoint,
    /// The picker's place.
    pub at: (i32, i32),
    /// The field's size: 160 wide, 112 tall.
    pub size: (i32, i32),
    /// The colour of the picked point in bytes, alpha 255.
    pub colour: [u8; 4],
    /// The marker's grey, alpha 255.
    pub marker: [u8; 4],
    /// The control attached below the picker.
    pub attached: Option<AttachedControl>,
    /// Whether a hair material is attached (re-shaded on refresh).
    pub material: bool,
    /// Whether the owner hears a change.
    pub callback: bool,
}

impl HairColPicker {
    /// The constructor: the tone picker's, then a 160 × 112 field, the
    /// point at (¼, ½), opaque colour and marker records and a refresh.
    pub fn new(at: (i32, i32)) -> Self {
        let mut picker = Self {
            point: PickPoint { u: 0.25, v: 0.5 },
            at,
            size: (0xa0, 0x70),
            colour: [0, 0, 0, 255],
            marker: [0, 0, 0, 255],
            attached: None,
            material: false,
            callback: false,
        };
        picker.refresh();
        picker
    }

    /// The colour at `u` before the value scale: the hue run blended at
    /// `3u`, the index truncated. Points outside the table (`u` under
    /// −4/3 or past 11/3) are held to its ends, where the original reads
    /// whatever follows.
    pub fn hue_at(u: f32) -> [u8; 3] {
        let scaled = u * 3.0;
        let index = scaled as i32;
        let fraction = scaled - index as f32;
        let at = (index + HUE_RUN as i32).clamp(0, HAIR_TABLE.len() as i32 - 2) as usize;
        byte_mix(HAIR_TABLE[at], HAIR_TABLE[at + 1], fraction)
    }

    /// The refresh: the colour is the hue at `u` scaled by `v·½ + ¼`, the
    /// marker grey `140·v` rounded to even (its low byte). Answers whether
    /// the attached hair material was re-shaded (and notice 7 raised).
    pub fn refresh(&mut self) -> bool {
        let scale = self.point.v * 0.5 + 0.25;
        let [r, g, b] = byte_scale(Self::hue_at(self.point.u), scale);
        self.colour = [r, g, b, 255];
        let grey = exa_formats::pixel::round_f32(HAIR_MARKER_SCALE * self.point.v) as u8;
        self.marker = [grey, grey, grey, 255];
        self.material
    }

    /// The hair routine's three inputs for the material: the point's `u`
    /// and `v` and the attached control's value. The material then takes
    /// the routine's diffuse, its specular × 1.3 (both sRGB-decoded when
    /// the engine's linear lighting flag is set), a copy of the specular
    /// and the specular scaled by the material's own level.
    pub fn material_inputs(&self) -> Option<(f32, f32, f32)> {
        if !self.material {
            return None;
        }
        let value = self.attached.map_or(0.0, |a| a.value);
        Some((self.point.u, self.point.v, value))
    }

    /// The refresh slot: the refresh, then the owner's callback when set.
    /// Answers whether the owner is called.
    pub fn refresh_with(&mut self) -> bool {
        self.refresh();
        self.callback
    }

    /// The set slot: the point and the attached control's value as given
    /// (no clamp), then the refresh.
    pub fn set(&mut self, point: PickPoint, value: f32) -> bool {
        self.point = point;
        if let Some(attached) = self.attached.as_mut() {
            attached.value = value;
        }
        self.refresh()
    }

    /// Attaches the control below the picker: it shows the picker's colour
    /// record, calls the picker's child refresh on a change, sits at the
    /// picker's x, 120 below its y, as wide as the picker and 40 tall, and
    /// starts at ¼.
    pub fn attach(&mut self) -> AttachedControl {
        let control = AttachedControl {
            place: (
                self.at.0,
                self.at.1 + ATTACHED_DROP,
                self.size.0,
                ATTACHED_HEIGHT,
            ),
            value: ATTACHED_VALUE,
            linked: true,
        };
        self.attached = Some(control);
        control
    }

    /// The attached control's change hook: the control's value, then the
    /// picker's refresh and the owner's callback. Answers whether the owner
    /// is called.
    pub fn child_refresh(&mut self, value: f32) -> bool {
        if let Some(attached) = self.attached.as_mut() {
            attached.value = value;
        }
        self.refresh_with()
    }

    /// The press slot, as the tone picker's (a moved point runs the
    /// refresh slot, [`HairColPicker::refresh_with`]).
    pub fn press(&mut self, at: Option<(i32, i32)>, base_answer: u32) -> u32 {
        let Some((x, y)) = at else {
            return base_answer;
        };
        if pick_point(&mut self.point, x, y, self.size) {
            self.refresh_with();
        }
        PICK_ANSWER | PRESS_ANSWER
    }

    /// The field's cells: three columns of three rows, each a quad whose
    /// left edge takes the column's hue and right edge the next one's (all
    /// rows alike). The cell size is a third of the field, in double,
    /// narrowed.
    pub fn cells(&self) -> Vec<[FieldVertex; 4]> {
        let cell_w = (f64::from(self.size.0) * (1.0 / 3.0)) as f32;
        let cell_h = (f64::from(self.size.1) * (1.0 / 3.0)) as f32;
        let mut out = Vec::with_capacity(9);
        for row in 0..3usize {
            let (y0, y1) = (row as f32 * cell_h, (row + 1) as f32 * cell_h);
            for column in 0..3usize {
                let (x0, x1) = (column as f32 * cell_w, (column + 1) as f32 * cell_w);
                let (left, right) = (
                    HAIR_TABLE[HUE_RUN + column],
                    HAIR_TABLE[HUE_RUN + column + 1],
                );
                let vertex = |colour, at| FieldVertex { colour, at };
                out.push([
                    vertex(left, (x0, y0)),
                    vertex(left, (x0, y1)),
                    vertex(right, (x1, y1)),
                    vertex(right, (x1, y0)),
                ]);
            }
        }
        out
    }

    /// The shade drawn over the field (in the draw's second blend mode):
    /// black along the top, white at 1.2 times the field's height (in
    /// double, narrowed), across the whole width.
    pub fn shade_quad(&self) -> [FieldVertex; 4] {
        let height = (f64::from(self.size.1) * 1.2) as f32;
        let width = self.size.0 as f32;
        [
            FieldVertex {
                colour: [0, 0, 0],
                at: (width, 0.0),
            },
            FieldVertex {
                colour: [0, 0, 0],
                at: (0.0, 0.0),
            },
            FieldVertex {
                colour: [255, 255, 255],
                at: (0.0, height),
            },
            FieldVertex {
                colour: [255, 255, 255],
                at: (width, height),
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_field_is_drawn_from_the_start_up_bytes() {
        // An independent single-precision emulation of the start-up
        // routine (value lowered, raised to 1.5 with the fast ln/exp,
        // HSV to RGB, × 255).
        assert_eq!(
            tone_field_bytes(),
            [
                [[104, 86, 68], [123, 104, 89], [123, 111, 105]],
                [[64, 52, 41], [92, 74, 64], [123, 99, 86]],
                [[30, 24, 20], [64, 47, 39], [98, 74, 61]],
            ]
        );
        let picker = TonePicker::new();
        let quads = picker.quads();
        let bytes = tone_field_bytes();
        // Cell (0, 0) is even: top-left, bottom-left, bottom-right first.
        assert_eq!(quads[0][0].colour, bytes[0][0]);
        assert_eq!(quads[0][2].colour, bytes[1][1]);
        assert_eq!(quads[0][2].at, (80.0, 80.0));
        assert_eq!(quads[0][4].colour, bytes[0][1]);
        // Cell (0, 1) is odd: its third vertex is the top-right.
        assert_eq!(quads[1][2].colour, bytes[0][2]);
        assert_eq!(quads[1][2].at, (160.0, 0.0));
        assert_eq!(quads[3][5].at, (80.0, 80.0));
    }

    #[test]
    fn the_tone_picker_colour_is_the_skin_tone_before_encoding() {
        let mut picker = TonePicker::new();
        assert_eq!(
            (picker.size, picker.point, picker.colour),
            ((160, 160), PickPoint { u: 0.5, v: 0.5 }, [0.0; 3])
        );
        assert_eq!(
            (picker.layout, picker.state, picker.style),
            (0x80, 0x40_0010, 0x400)
        );
        assert_eq!(picker.refresh(), ToneTarget::Skin);
        assert_eq!(picker.colour, tone_linear(0.5, 0.5));
        picker.callback = true;
        assert_eq!(picker.refresh(), ToneTarget::Callback);
        // A material under 0.1 is pulled a quarter of the way to the pale
        // tone; others and NaN take the colour.
        let pale = tone_blend_target();
        let pulled = picker.skin_colour(0.0);
        assert_eq!(
            pulled[0],
            picker.colour[0] + (pale[0] - picker.colour[0]) * 0.25
        );
        assert_eq!(picker.skin_colour(0.1), picker.colour);
        assert_eq!(picker.skin_colour(f32::NAN), picker.colour);
        assert_eq!(picker.press(None, 0x10), 0x10);
        picker.callback = false;
        assert_eq!(picker.press(Some((40, 120)), 0x10), 7);
        assert_eq!(picker.point, PickPoint { u: 0.25, v: 0.75 });
        assert_eq!(picker.colour, tone_linear(0.25, 0.75));
        // Out-of-field points are held to the edges.
        assert!(pick_point(&mut picker.point, -100, 1000, picker.size));
        assert_eq!(picker.point, PickPoint { u: 0.0, v: 1.0 });
        // The same point again does not move it.
        assert!(!pick_point(&mut picker.point, -100, 1000, picker.size));
    }

    #[test]
    fn the_hair_colour_is_the_column_blend_scaled_by_the_value() {
        // u = ¼: 3u = 0.75 between the dark red and the blonde:
        // 164 + 6·0.75 = 168.5 → 168 (even), 98 + 32·0.75 = 122,
        // 48 + 34·0.75 = 73.5 → 74.
        assert_eq!(HairColPicker::hue_at(0.25), [168, 122, 74]);
        assert_eq!(HairColPicker::hue_at(0.0), [164, 98, 48]);
        // The right edge reads the last hue with weight 0 toward the grey.
        assert_eq!(HairColPicker::hue_at(1.0), [164, 98, 48]);
        // Past the run the grey takes over, and black before it.
        assert_eq!(HairColPicker::hue_at(4.0 / 3.0), [128, 128, 128]);
        assert_eq!(HairColPicker::hue_at(-1.0), [0, 0, 0]);
        let mut picker = HairColPicker::new((10, 20));
        assert_eq!(
            (picker.size, picker.point.u, picker.point.v),
            ((160, 112), 0.25, 0.5)
        );
        // Scale ½·½ + ¼ = ½: 84, 61, 37.
        assert_eq!(picker.colour, [84, 61, 37, 255]);
        assert_eq!(picker.marker, [70, 70, 70, 255]);
        assert!(!picker.refresh());
        assert_eq!(picker.material_inputs(), None);
        // The set slot does not clamp and needs no attached control.
        picker.set(PickPoint { u: 0.5, v: 1.0 }, 0.9);
        // 3u = 1.5: 170 − 18·½ = 161, 130 − 55·½ = 102.5 → 102, 82 − 28·½
        // = 68, scaled by ¾: 120.75 → 121, 76.5 → 76, 51.
        assert_eq!(picker.colour, [121, 76, 51, 255]);
        assert_eq!(picker.marker[0], 140);
        // Attaching places the control 120 below, picker-wide, 40 tall.
        let control = picker.attach();
        assert_eq!(control.place, (10, 140, 160, 40));
        assert_eq!(control.value, 0.25);
        picker.material = true;
        assert!(picker.set(PickPoint { u: 0.0, v: 0.0 }, 0.6));
        assert_eq!(picker.material_inputs(), Some((0.0, 0.0, 0.6)));
        picker.callback = true;
        assert!(picker.child_refresh(0.3));
        assert_eq!(picker.attached.unwrap().value, 0.3);
    }

    #[test]
    fn the_hair_field_draws_three_columns_and_a_shade() {
        let picker = HairColPicker::new((0, 0));
        let cells = picker.cells();
        assert_eq!(cells.len(), 9);
        assert_eq!(cells[0][0].colour, [164, 98, 48]);
        assert_eq!(cells[0][3].colour, [170, 130, 82]);
        assert_eq!(cells[8][2].colour, [164, 98, 48]);
        let third = (160.0f64 * (1.0 / 3.0)) as f32;
        assert_eq!(cells[0][3].at, (third, 0.0));
        let quad = picker.shade_quad();
        assert_eq!(quad[0].at, (160.0, 0.0));
        assert_eq!(quad[2].at, (0.0, (112.0f64 * 1.2) as f32));
        assert_eq!((quad[1].colour, quad[3].colour), ([0; 3], [255; 3]));
    }
}

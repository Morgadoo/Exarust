//! The character-creation pickers (`THairColPicker`, `TTonePicker`): a
//! field whose picked point becomes a colour, a marker and a small preview.
//!
//! The tone picker is a 2 × 2 grid of quads over a 3 × 3 table of skin
//! tones; the picked point bilinearly interpolates the table, the channel
//! divisors of 0.8, 0.651 and 0.573 and the 0.6 scale are the original's.
//! The hair picker is a 3 × 3 grid whose columns run through a four-colour
//! table, the picked column interpolated and the value scaling the colour,
//! with a grey marker whose level is 140 times the value.
//!
//! The table values are the bytes and floats the original's pickers read;
//! the helper that turns the tone table into floats is outside the pack, so
//! the bilinear form is a guess pinned to the original's table. Evidence:
//! both constructors, the tone field's pick and draw, the hair picker's
//! refresh, set and draw, and the task notification word 7.

use crate::colour_picker::{hsv_to_rgb, rgb_to_hsv};

/// The tone table's 3 × 3 colours, red green blue, raw bytes read as
/// floats.
pub const TONE_TABLE: [[[f32; 3]; 3]; 3] = [
    [
        [170.0, 143.0, 132.0],
        [198.0, 154.0, 118.0],
        [173.0, 122.0, 85.0],
    ],
    [
        [199.0, 156.0, 101.0],
        [140.0, 118.0, 96.0],
        [125.0, 90.0, 65.0],
    ],
    [[134.0, 104.0, 62.0], [90.0, 70.0, 46.0], [62.0, 50.0, 30.0]],
];
/// The per-channel divisors the tone picker applies.
pub const TONE_DIVISOR: [f32; 3] = [0.8, 0.651, 0.573];
/// The scale the tone colour takes after the division.
pub const TONE_SCALE: f32 = 0.6;

/// The tone field's 3 × 3 vertex colours, drawn as four quads.
pub const TONE_VERTICES: [[u8; 3]; 9] = [
    [170, 145, 121],
    [198, 198, 154],
    [118, 173, 122],
    [85, 140, 118],
    [96, 125, 90],
    [65, 134, 131],
    [62, 90, 70],
    [46, 60, 50],
    [40, 0, 0],
];

/// The hair table's three rows of four colours: each row is a horizontal
/// run, the field interpolating left to right.
pub const HAIR_TABLE: [[[u8; 3]; 4]; 3] = [
    [[164, 98, 48], [170, 130, 82], [152, 75, 54], [164, 98, 48]],
    [
        [128, 128, 128],
        [128, 128, 128],
        [128, 128, 128],
        [128, 128, 128],
    ],
    [
        [128, 128, 128],
        [255, 255, 255],
        [255, 255, 255],
        [255, 255, 255],
    ],
];
/// The marker's brightness per unit of value.
pub const HAIR_MARKER_SCALE: f32 = 140.0;
/// The task notification a hair-picker refresh raises.
pub const HAIR_CHANGED_WORD: u32 = 7;

/// A picker's picked point, both coordinates in 0–1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PickPoint {
    pub u: f32,
    pub v: f32,
}

/// The linear blend of two colours at `t`.
fn blend(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// The bilinear tone colour of a point: the point scaled by two, split into
/// a cell (0 or 1) and its fraction, the four table corners blended across
/// and then down, each channel divided by its divisor and the result scaled
/// by 0.6. The result stays in the table's raw 0–255 scale.
pub fn tone_rgb(u: f32, v: f32) -> [f32; 3] {
    let mut x = u * 2.0;
    let right = x > 1.0;
    if right {
        x -= 1.0;
    }
    let mut y = v * 2.0;
    let down = y > 1.0;
    if down {
        y -= 1.0;
    }
    let (column, row) = (usize::from(right), usize::from(down));
    let top = blend(TONE_TABLE[row][column], TONE_TABLE[row][column + 1], x);
    let bottom = blend(
        TONE_TABLE[row + 1][column],
        TONE_TABLE[row + 1][column + 1],
        x,
    );
    let mixed = blend(top, bottom, y);
    [
        mixed[0] / TONE_DIVISOR[0] * TONE_SCALE,
        mixed[1] / TONE_DIVISOR[1] * TONE_SCALE,
        mixed[2] / TONE_DIVISOR[2] * TONE_SCALE,
    ]
}

/// One drawn vertex: its colour and position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FieldVertex {
    pub colour: [u8; 3],
    pub at: (i32, i32),
}

/// One quad of a picker field: six vertices, two triangles.
pub type FieldQuad = [FieldVertex; 6];

/// The marker: where it sits and which way it points (the original's picker
/// base draws it), with the hair picker's grey level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PickerMarker {
    pub at: (i32, i32),
    /// The marker's grey, red green and blue equal.
    pub grey: u8,
}

/// The tone picker.
#[derive(Debug, Clone, PartialEq)]
pub struct TonePicker {
    pub point: PickPoint,
    /// The field's size in pixels (160 square).
    pub size: (i32, i32),
    /// The form's style word after the constructor (0x80 and 0x400).
    pub style: u32,
    /// The form's state word after the constructor.
    pub state: u32,
    /// The colour the field last produced, in the table's raw scale.
    pub colour: [f32; 3],
    pub shown: bool,
}

impl TonePicker {
    /// The constructor: a 160 × 160 field with its point at the middle and
    /// the first refresh.
    pub fn new() -> Self {
        let mut picker = Self {
            point: PickPoint { u: 0.5, v: 0.5 },
            size: (0xa0, 0xa0),
            style: 0x80 | 0x400,
            state: 0x40_0010,
            colour: [0.0; 3],
            shown: false,
        };
        picker.refresh();
        picker
    }

    /// Recomputes the colour from the point.
    pub fn refresh(&mut self) {
        self.colour = tone_rgb(self.point.u, self.point.v);
    }

    /// The pick slot: a point in the field's coordinates becomes the
    /// point, held to 0–1, and the colour follows. The original divides by
    /// the field's size and clamps through the picker base. A NaN passes
    /// the clamp.
    pub fn pick(&mut self, x: i32, y: i32) -> [f32; 3] {
        self.point.u = (x as f32 / self.size.0 as f32).clamp(0.0, 1.0);
        self.point.v = (y as f32 / self.size.1 as f32).clamp(0.0, 1.0);
        self.refresh();
        self.colour
    }

    /// The four quads of the field's 2 × 2 grid, each over the 3 × 3
    /// vertex table. The vertices are the top-left, bottom-left and
    /// bottom-right colours in one triangle and the bottom-right, top-right
    /// and top-left in the other, alternating for the checkerboard, at
    /// half the field per cell.
    pub fn quads(&self) -> [FieldQuad; 4] {
        let half_w = (self.size.0 as f32 * 0.5) as i32;
        let half_h = (self.size.1 as f32 * 0.5) as i32;
        let mut quads = [[FieldVertex {
            colour: [0; 3],
            at: (0, 0),
        }; 6]; 4];
        let mut slot = 0;
        for row in 0..2usize {
            for column in 0..2usize {
                let (x0, y0) = (column as i32 * half_w, row as i32 * half_h);
                let (x1, y1) = (x0 + half_w, y0 + half_h);
                let top_left = TONE_VERTICES[row * 3 + column];
                let top_right = TONE_VERTICES[row * 3 + column + 1];
                let bottom_left = TONE_VERTICES[(row + 1) * 3 + column];
                let bottom_right = TONE_VERTICES[(row + 1) * 3 + column + 1];
                let vertex = |colour, at| FieldVertex { colour, at };
                quads[slot] = if (row + column) % 2 == 0 {
                    [
                        vertex(top_left, (x0, y0)),
                        vertex(bottom_left, (x0, y1)),
                        vertex(bottom_right, (x1, y1)),
                        vertex(bottom_right, (x1, y1)),
                        vertex(top_right, (x1, y0)),
                        vertex(top_left, (x0, y0)),
                    ]
                } else {
                    [
                        vertex(top_left, (x0, y0)),
                        vertex(bottom_left, (x0, y1)),
                        vertex(top_right, (x1, y0)),
                        vertex(top_right, (x1, y0)),
                        vertex(bottom_left, (x0, y1)),
                        vertex(bottom_right, (x1, y1)),
                    ]
                };
                slot += 1;
            }
        }
        quads
    }

    /// The press slot: a press on the field itself (the pointer's control
    /// code −1) picks the point and answers 5, the picker's hold word.
    pub fn press(&mut self, x: i32, y: i32) -> ([f32; 3], u32) {
        (self.pick(x, y), 5)
    }

    /// Where the marker sits: the point over the field's size.
    pub fn marker(&self) -> PickerMarker {
        PickerMarker {
            at: (
                (self.point.u * self.size.0 as f32) as i32,
                (self.point.v * self.size.1 as f32) as i32,
            ),
            grey: 0,
        }
    }
}

impl Default for TonePicker {
    fn default() -> Self {
        Self::new()
    }
}

/// The preview node a hair picker attaches to: the colour it shows, the
/// owner and callback marks, its place and its version.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HairPreviewNode {
    pub colour: [u8; 3],
    pub owner: bool,
    pub callback: bool,
    pub place: (i32, i32, i32, i32),
    pub version: f32,
}

impl Default for HairPreviewNode {
    fn default() -> Self {
        Self {
            colour: [0; 3],
            owner: false,
            callback: false,
            place: (0, 0, 0, 0),
            version: 0.0,
        }
    }
}

/// The hair-colour picker.
#[derive(Debug, Clone, PartialEq)]
pub struct HairColPicker {
    pub point: PickPoint,
    /// The field's size: 160 wide, 112 tall.
    pub size: (i32, i32),
    /// The shading version the character generator passes (the picker base's
    /// third argument).
    pub version: f32,
    /// The colour of the picked point, scaled by the value, in bytes.
    pub colour: [u8; 3],
    /// The marker's grey.
    pub grey: u8,
    /// The style and state words after the constructor.
    pub style: u32,
    pub state: u32,
    /// Whether the owner hears a change (the original's +0x180 callback).
    pub callback: bool,
    /// The task notification the last refresh raised.
    pub notice: u32,
    pub shown: bool,
}

impl HairColPicker {
    /// The constructor: the tone base at 160 square, moved to 160 × 112,
    /// the point at (0.25, 0.5) and the first refresh. The value sets the
    /// marker grey and the field's flag byte.
    pub fn new() -> Self {
        let mut picker = Self {
            point: PickPoint { u: 0.25, v: 0.5 },
            size: (0xa0, 0x70),
            version: 0.5,
            colour: [0; 3],
            grey: 0,
            style: 0x80 | 0x400,
            state: 0x40_0010,
            callback: true,
            notice: 0,
            shown: false,
        };
        picker.refresh();
        picker
    }

    /// The colour at a point of the table before the value scale: the
    /// column is `floor(u · 3)`, the blend across it to the next column,
    /// each byte rounded to nearest even.
    pub fn table_colour(u: f32) -> [u8; 3] {
        let scaled = u * 3.0;
        let index = (scaled as i32).clamp(0, 2) as usize;
        let fraction = scaled - index as f32;
        let a = HAIR_TABLE[0][index];
        let b = HAIR_TABLE[0][index + 1];
        let mut out = [0u8; 3];
        for channel in 0..3 {
            let value =
                f32::from(a[channel]) + (f32::from(b[channel]) - f32::from(a[channel])) * fraction;
            out[channel] = value.round_ties_even() as u8;
        }
        out
    }

    /// Recomputes the colour and the marker: the table colour scaled by
    /// `v · 0.5 + 0.25`, the marker's grey `round(v · 140)`. The owner is
    /// told and the task notified with word 7 when the picker has one.
    pub fn refresh(&mut self) {
        let base = Self::table_colour(self.point.u);
        let scale = self.point.v * 0.5 + 0.25;
        for (out, channel) in self.colour.iter_mut().zip(base) {
            *out = (f32::from(channel) * scale).round_ties_even() as u8;
        }
        self.grey = (self.point.v * HAIR_MARKER_SCALE).round_ties_even() as u8;
        if self.callback {
            self.notice |= HAIR_CHANGED_WORD;
        }
    }

    /// The pick slot: a point in the field becomes the point (held to 0–1)
    /// and the colour follows.
    pub fn pick(&mut self, x: i32, y: i32) -> [u8; 3] {
        self.point.u = (x as f32 / self.size.0 as f32).clamp(0.0, 1.0);
        self.point.v = (y as f32 / self.size.1 as f32).clamp(0.0, 1.0);
        self.refresh();
        self.colour
    }

    /// The press slot: a press on the field picks the point and answers 5.
    pub fn press(&mut self, x: i32, y: i32) -> ([u8; 3], u32) {
        (self.pick(x, y), 5)
    }

    /// The refresh callback choice: when the owner has a callback it hears
    /// the new colour; otherwise the marker node is updated in place. The
    /// task is notified with word 7 either way.
    pub fn refresh_with(&mut self, owner: Option<&mut dyn FnMut([u8; 3])>) {
        self.refresh();
        if let Some(callback) = owner {
            callback(self.colour);
        }
    }

    /// The child callback: the picker refreshes and tells its owner (the
    /// original's +0x198 hook).
    pub fn child_refresh(&mut self, owner: Option<&mut dyn FnMut([u8; 3])>) {
        self.refresh_with(owner);
    }

    /// Attaches the character generator's version: it becomes the picker's
    /// shading version and the refresh runs.
    pub fn attach(&mut self, version: f32) -> [u8; 3] {
        self.version = version;
        self.refresh();
        self.colour
    }

    /// Attaches a preview node: the node takes the picker's colour, the
    /// picker as its owner, the child callback and the version 0.25, and
    /// its place follows the picker's (x, y, width, 0x28 tall). The node's
    /// callback then hears the picker's colour.
    pub fn attach_node(&mut self, node: &mut HairPreviewNode) -> [u8; 3] {
        node.colour = self.colour;
        node.owner = true;
        node.callback = true;
        node.place = (self.size.0 / 2, self.size.1 / 2, 0x28, 0x28);
        node.version = 0.25;
        self.version = 0.25;
        self.refresh();
        node.colour = self.colour;
        self.colour
    }

    /// The set slot: a point and the shading version, then a refresh.
    pub fn set(&mut self, point: PickPoint, version: f32) -> [u8; 3] {
        self.point = PickPoint {
            u: point.u.clamp(0.0, 1.0),
            v: point.v.clamp(0.0, 1.0),
        };
        self.version = version;
        self.refresh();
        self.colour
    }

    /// The marker's place.
    pub fn marker(&self) -> PickerMarker {
        PickerMarker {
            at: (
                (self.point.u * self.size.0 as f32) as i32,
                (self.point.v * self.size.1 as f32) as i32,
            ),
            grey: self.grey,
        }
    }

    /// The field's cells: three columns of three rows. Each cell's left
    /// edge takes the column's table colour, its right edge the next
    /// column's; all nine cells repeat the same three colour pairs.
    pub fn cells(&self) -> Vec<[FieldVertex; 4]> {
        let cell_w = (self.size.0 as f64 / 3.0) as i32;
        let cell_h = (self.size.1 as f64 / 3.0) as i32;
        let mut out = Vec::with_capacity(9);
        for row in 0..3usize {
            let y0 = row as i32 * cell_h;
            let y1 = (row as i32 + 1) * cell_h;
            for column in 0..3usize {
                let x0 = column as i32 * cell_w;
                let x1 = (column as i32 + 1) * cell_w;
                let left = HAIR_TABLE[0][column];
                let right = HAIR_TABLE[0][column + 1];
                out.push([
                    FieldVertex {
                        colour: left,
                        at: (x0, y0),
                    },
                    FieldVertex {
                        colour: left,
                        at: (x0, y1),
                    },
                    FieldVertex {
                        colour: right,
                        at: (x1, y1),
                    },
                    FieldVertex {
                        colour: right,
                        at: (x1, y0),
                    },
                ]);
            }
        }
        out
    }

    /// The darkening quad drawn over the field: black at the top and white
    /// at 1.2 times the field's height, across the whole width.
    pub fn shade_quad(&self) -> FieldQuad {
        let height = (self.size.1 as f32 * 1.2) as i32;
        let width = self.size.0;
        [
            FieldVertex {
                colour: [0, 0, 0],
                at: (width, 0),
            },
            FieldVertex {
                colour: [0, 0, 0],
                at: (0, 0),
            },
            FieldVertex {
                colour: [255, 255, 255],
                at: (0, height),
            },
            FieldVertex {
                colour: [255, 255, 255],
                at: (0, height),
            },
            FieldVertex {
                colour: [255, 255, 255],
                at: (width, height),
            },
            FieldVertex {
                colour: [255, 255, 255],
                at: (width, 0),
            },
        ]
    }
}

impl Default for HairColPicker {
    fn default() -> Self {
        Self::new()
    }
}

/// The two colour buffers the hair picker's shading routine produces from
/// its point and the character generator's version: the first a lighter
/// highlight and the second the shaded hair colour. The routine blends the
/// column's table colours, folds in the value and the version, and hands
/// both through the engine's colour helpers. The helpers' exact transfer
/// (the linear/sRGB pair and the brightness adjustment) is outside the
/// pack; they are modelled here as a clamp to 0–1, a channel scale,
/// `rgb_to_hsv`, a saturation of 1.5 and a value from the version, and the
/// numbers are pinned to those guesses.
pub fn hair_shading(u: f32, v: f32, version: f32) -> ([f32; 3], [f32; 3]) {
    let scaled = u * 3.0;
    let index = (scaled as i32).clamp(0, 2) as usize;
    let fraction = scaled - index as f32;
    let a = HAIR_TABLE[0][index];
    let b = HAIR_TABLE[0][index + 1];
    let mixed = [
        (f32::from(a[0]) + (f32::from(b[0]) - f32::from(a[0])) * fraction) / 255.0,
        (f32::from(a[1]) + (f32::from(b[1]) - f32::from(a[1])) * fraction) / 255.0,
        (f32::from(a[2]) + (f32::from(b[2]) - f32::from(a[2])) * fraction) / 255.0,
    ];
    let value = v + 0.1;
    // The brightness adjustment's first pass.
    let adjusted = [
        mixed[0] * value * value * 0.1,
        mixed[1] * value * value * 0.1,
        mixed[2] * value * value * 0.1,
    ];
    let grey = 0.75 * value * 0.3;
    let t = version * 0.8 + 0.2;
    let blended = [
        adjusted[0] + (grey - adjusted[0]) * t,
        adjusted[1] + (grey - adjusted[1]) * t,
        adjusted[2] + (grey - adjusted[2]) * t,
    ];
    let first = {
        let scale = value * 0.4 + 0.3;
        [
            (blended[0] * scale * 2.0).clamp(0.0, 1.0),
            (blended[1] * scale * 2.0).clamp(0.0, 1.0),
            (blended[2] * scale * 2.0).clamp(0.0, 1.0),
        ]
    };
    let mut hsv = rgb_to_hsv([
        blended[0].clamp(0.0, 1.0),
        blended[1].clamp(0.0, 1.0),
        blended[2].clamp(0.0, 1.0),
    ]);
    hsv[1] *= 1.5;
    hsv[2] = ((value * 0.9 + 0.3) - version * 0.1) * 0.8;
    let second = hsv_to_rgb(hsv);
    (first, second)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tone_point_bilinearly_interpolates_the_table() {
        // Exactly 0.5 is the end of the first cell (the split is strictly
        // above one): the middle row's first table colour.
        let middle = tone_rgb(0.5, 0.5);
        let raw = [140.0, 118.0, 96.0];
        let expected = [
            raw[0] / 0.8 * 0.6,
            raw[1] / 0.651 * 0.6,
            raw[2] / 0.573 * 0.6,
        ];
        for (a, b) in middle.iter().zip(expected) {
            assert!((a - b).abs() < 1e-5, "{middle:?}");
        }
        // The top-left corner is the first table colour.
        let corner = tone_rgb(0.0, 0.0);
        assert!((corner[0] - 170.0 / 0.8 * 0.6).abs() < 1e-5);
        assert!((corner[1] - 143.0 / 0.651 * 0.6).abs() < 1e-5);
        assert!((corner[2] - 132.0 / 0.573 * 0.6).abs() < 1e-5);
        // The bottom-right corner is the last.
        let corner = tone_rgb(1.0, 1.0);
        assert_eq!(
            corner,
            [62.0 / 0.8 * 0.6, 50.0 / 0.651 * 0.6, 30.0 / 0.573 * 0.6]
        );
        // A point in the second cell uses its own corners.
        let point = tone_rgb(0.75, 0.25);
        let a = [159.0, 121.0, 91.0];
        assert!((point[0] - a[0] / 0.8 * 0.6).abs() < 1e-5, "{point:?}");
    }

    #[test]
    fn the_tone_picker_picks_and_draws_the_checker() {
        let mut picker = TonePicker::new();
        assert_eq!(
            (picker.size, picker.point.u, picker.point.v),
            ((160, 160), 0.5, 0.5)
        );
        assert_eq!((picker.style, picker.state), (0x480, 0x40_0010));
        let colour = picker.pick(40, 120);
        assert!((picker.point.u - 0.25).abs() < 1e-6);
        assert!((picker.point.v - 0.75).abs() < 1e-6);
        assert_eq!(colour, tone_rgb(0.25, 0.75));
        // Out-of-field points clamp.
        picker.pick(-100, 1000);
        assert_eq!(picker.point.u, 0.0);
        assert_eq!(picker.point.v, 1.0);
        let marker = picker.marker();
        assert_eq!(marker.at, (0, 160));
        let quads = picker.quads();
        assert_eq!(
            quads[0][0],
            FieldVertex {
                colour: TONE_VERTICES[0],
                at: (0, 0)
            }
        );
        // The first cell splits from its top-left to its bottom-right.
        assert_eq!(quads[0][2].colour, TONE_VERTICES[4]);
        assert_eq!(quads[0][2].at, (80, 80));
        assert_eq!(quads[1][0].colour, TONE_VERTICES[1]);
        assert_eq!(quads[3][0].colour, TONE_VERTICES[4]);
        assert_eq!(quads[3][5].at, (80, 80));
    }

    #[test]
    fn the_hair_colour_is_the_column_blend_scaled_by_the_value() {
        // u = 0.25 is three quarters through the first third: from (164,
        // 98, 48) three quarters to (170, 130, 82) is (169, 122, 74).
        assert_eq!(HairColPicker::table_colour(0.25), [168, 122, 74]);
        assert_eq!(HairColPicker::table_colour(0.0), [164, 98, 48]);
        assert_eq!(HairColPicker::table_colour(1.0), [164, 98, 48]);
        let mut picker = HairColPicker::new();
        assert_eq!(
            (picker.size, picker.point.u, picker.point.v),
            ((160, 112), 0.25, 0.5)
        );
        // Scale 0.5 × 0.5 + 0.25 = 0.5.
        assert_eq!(picker.colour, [84, 61, 37]);
        assert_eq!(picker.grey, 70);
        assert_eq!(picker.notice, HAIR_CHANGED_WORD);
        // Another point: u = 0.5 is the second third's start, v = 1 scales
        // by 0.75.
        let colour = picker.pick(80, 112);
        assert_eq!(HairColPicker::table_colour(0.5), [161, 102, 68]);
        assert_eq!(colour, [121, 76, 51]);
        assert_eq!(picker.grey, 140);
        assert_eq!(picker.marker().at, (80, 112));
        // The set slot takes a point and a version.
        picker.set(PickPoint { u: 0.0, v: 0.0 }, 0.9);
        assert_eq!(picker.colour, [41, 24, 12]);
        assert_eq!(picker.version, 0.9);
        // A callback-less picker raises no notice.
        picker.callback = false;
        picker.notice = 0;
        picker.refresh();
        assert_eq!(picker.notice, 0);
        let cells = picker.cells();
        assert_eq!(cells.len(), 9);
        assert_eq!(cells[0][0].colour, HAIR_TABLE[0][0]);
        assert_eq!(cells[0][3].colour, HAIR_TABLE[0][1]);
        assert_eq!((cells[0][0].at, cells[0][3].at), ((0, 0), (53, 0)));
        // Attaching a node copies the colour, the two marks, the place and
        // the version.
        let mut node = HairPreviewNode::default();
        let colour = picker.attach_node(&mut node);
        assert_eq!(node.colour, colour);
        assert!(node.owner && node.callback);
        assert_eq!(node.place, (80, 56, 0x28, 0x28));
        assert_eq!((node.version, picker.version), (0.25, 0.25));
        // The shading's two buffers: the table's first colour at u = 0
        // with v = 0.5 and version 0.5.
        let (light, shaded) = hair_shading(0.0, 0.5, 0.5);
        assert_eq!(light, [0.09748208, 0.093456864, 0.09040745]);
        assert_eq!(shaded, [0.632, 0.5928554, 0.5632003]);
        let quad = picker.shade_quad();
        assert_eq!(quad[1].at, (0, 0));
        assert_eq!(quad[2].at, (0, 134));
        assert_eq!(quad[0].colour, [0, 0, 0]);
        assert_eq!(quad[4].colour, [255, 255, 255]);
    }
}

//! The divination card reading: three distinct cards out of the twenty-two
//! on the sheet, dealt in, turned over one after another and held, each
//! over its drop shadow.
//!
//! Evidence: the reading's routines in the retail image, read from the
//! instructions, with the engine's ease (`(3 − 2t)·t²`), lerp and
//! normalisation they call. The sheet `divcardsa1.rfi` is a six-column,
//! four-row grid; card `c` takes cell `(c + 2) mod 6` of row `(c + 2) /
//! 6`, the back the cell between 1/6 and 1/3 of the top row; a second
//! texture set reads the paper mask from the sheet's first column. A card
//! is `unit/6 · 480 · 0.25` wide and `unit/6 · 800 · 0.25` tall (half
//! sizes). The reading darkens the screen to alpha 128 over a second,
//! deals the cards over 600 ms each (rising by the ease to 400 · unit/6),
//! turns them over 900 ms each from 1.8 s (a half turn about an axis that
//! swings from (0.3, 0.9, 0.2) toward (0, 1, 0)), and holds from 4.5 s.
//! `PprMov` sounds at each deal step and at the first three turns. Times
//! are milliseconds, positions pixels.

use crate::engine_math::{reciprocal_sqrt, round_i64};

/// The card sheet.
pub const SHEET: &str = "divcardsa1.rfi";
/// The sound the deal and the turns play (volume 1, pitch 1).
pub const SOUND: &str = "PprMov";
/// The number of cards on the sheet.
pub const CARDS: u32 = 22;
/// The sheet's columns.
pub const SHEET_COLUMNS: i32 = 6;
/// The first card takes this cell of the sheet.
pub const FIRST_CELL: i32 = 2;
/// The card's sizes before the unit scale (`× 0.25`, over 6).
pub const CARD_WIDTH: i32 = 480;
pub const CARD_HEIGHT: i32 = 800;
/// The shadow outline's chamfer before the unit scale (`× 0.5`, over 6).
pub const CHAMFER: f64 = 28.0;
/// The fade's full alpha, reached after a second.
pub const FADE_MS: u32 = 1_000;
pub const FADE_ALPHA: f32 = 128.0;
/// The deal runs up to here; the turns up to the hold.
pub const DEAL_END_MS: u32 = 1_800;
pub const HOLD_MS: u32 = 4_500;
/// One card's deal and one card's turn.
pub const DEAL_STEP_MS: u32 = 600;
pub const FLIP_STEP_MS: u32 = 900;
/// A dealt card rests at this y (before the unit/6 scale); it rises from
/// `−300` by `700 · ease`.
pub const REST_Y: f32 = 400.0;
pub const START_Y: f32 = -300.0;
pub const RISE: f32 = 700.0;
/// The shadow's alpha and its band (four units).
pub const SHADOW_ALPHA: f32 = 200.0;
pub const SHADOW_UNITS: i32 = 4;
/// A shadow corner's bisector is lengthened by this.
pub const JOIN_SCALE: f32 = 1.1;
/// The paper mask's texture coordinates (the four corners: top left,
/// bottom left, bottom right, top right on the sheet).
pub const MASK: [[f32; 2]; 4] = [
    [0.000_347_222_23, 0.999_687_5],
    [0.000_347_222_23, 0.750_312_5],
    [0.166_319_44, 0.750_312_5],
    [0.166_319_44, 0.999_687_5],
];
/// The turn's axis at its start and at its end (the start normalised when
/// the form is built).
pub const TURN_AXIS_START: [f32; 3] = [0.3, 0.9, 0.2];
pub const TURN_AXIS_END: [f32; 3] = [0.0, 1.0, 0.0];
/// The shader's light direction, normalised at build.
pub const LIGHT: [f32; 3] = [0.0, 0.0, 1.0];

/// The form's style and flag bits.
pub const FORM_STYLE: u32 = 0xf_8800;
pub const FORM_FLAGS: u32 = 0x8_1000;
/// The paper shader's program slot.
pub const SHADER_PROGRAM: u32 = 0x35;
/// The reading's two shaders.
pub const CARD_VERTEX_SHADER: &str = "in vec3 vPos;\r\nin vec2 vTex;\r\nin vec2 vTx2;\r\nin vec4 vCol;\r\nout vec2 vrTex;\r\nout vec2 vrTx2;\r\nout vec4 vrCol;\r\nuniform mat4 MPrj;\r\nuniform mat4 MObj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrTx2 = vTx2;\r\nvrCol = vCol * 0.003921568;\r\n}";
pub const CARD_FRAGMENT_SHADER: &str = "in vec2 vrTex;\r\nin vec2 vrTx2;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform sampler2D Tex0;\r\nvoid main() {\r\nvec3 c = texture(Tex0, vrTex).rgb;\r\nc = c * (0.2 + abs(par0.z) * 0.8);\r\nFragColor.rgb = vrCol.rgb * c;\r\nfloat a = texture(Tex0, vrTx2).g;\r\nFragColor.a = vrCol.a * vrCol.a * a;\r\n}";

/// A reading that has just been opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub cards: [u32; 3],
    pub start_ms: u32,
    /// The phase word: 1 dealing, 2 turning, 3 held.
    pub phase: i32,
    /// The last sound step.
    pub step: u32,
}

/// Opens a reading at `now`: three distinct cards from `random(22)`.
pub fn open(now: u32, random: &mut impl FnMut(u32) -> u32) -> Reading {
    Reading {
        cards: deal_cards(random),
        start_ms: now,
        phase: phase::DEAL,
        step: 0,
    }
}

/// The reading's phase words.
pub mod phase {
    pub const DEAL: i32 = 1;
    pub const FLIP: i32 = 2;
    pub const HOLD: i32 = 3;
}

/// Deals three distinct cards: each is redrawn from 0..21 until it differs
/// from every earlier card.
pub fn deal_cards(random: &mut impl FnMut(u32) -> u32) -> [u32; 3] {
    let mut cards = [0u32; 3];
    for i in 0..3 {
        loop {
            let card = random(CARDS);
            if !cards[..i].contains(&card) {
                cards[i] = card;
                break;
            }
        }
    }
    cards
}

/// The engine's ease: `(3 − 2t)·t·t`.
pub fn ease(t: f32) -> f32 {
    (3.0 - t * 2.0) * t * t
}

/// The darkening's alpha byte: 128 past a second, else `elapsed / 1000 ·
/// 128` (a double rounded half to even).
pub fn fade_alpha(elapsed: u32) -> u8 {
    let alpha = if elapsed > FADE_MS {
        FADE_ALPHA
    } else {
        ((f64::from(elapsed) / 1_000.0) * 128.0).round_ties_even() as f32
    };
    (round_i64(alpha * 1.0) & 0xff) as u8
}

/// The paint's clock step: the phase it enters, whether `PprMov` sounds,
/// and the time the card loop measures from (the turns' own clock from
/// 1.8 s). The deal's steps are 600 ms; entering the turns resets the
/// step, whose steps are 900 ms and sound only for the first three.
pub fn clock_step(reading: &mut Reading, elapsed: u32) -> (bool, u32) {
    if elapsed <= DEAL_END_MS {
        let step = elapsed / DEAL_STEP_MS + 1;
        let play = step != reading.step;
        reading.step = step;
        (play, elapsed)
    } else if elapsed <= HOLD_MS {
        if reading.phase != phase::FLIP {
            reading.step = 0;
        }
        reading.phase = phase::FLIP;
        let local = elapsed - DEAL_END_MS;
        let step = local / FLIP_STEP_MS + 1;
        let play = step != reading.step && step < 4;
        reading.step = step;
        (play, local)
    } else {
        reading.phase = phase::HOLD;
        (false, elapsed)
    }
}

/// A card's half sizes in pixels: `(unit / 6) · 480 · 0.25` and the same
/// with 800, in doubles.
pub fn card_size(unit: i32) -> (f32, f32) {
    let sixth = f64::from(unit) / 6.0;
    (
        (sixth * f64::from(CARD_WIDTH) * 0.25) as f32,
        (sixth * f64::from(CARD_HEIGHT) * 0.25) as f32,
    )
}

/// The shadow outline: the card's box with its four corners chamfered by
/// `(unit / 6) · 28 · 0.5`, from the top edge's left end round to its
/// right end.
pub fn shadow_outline(unit: i32) -> [[f32; 2]; 8] {
    let (w, h) = card_size(unit);
    let c = (f64::from(unit) / 6.0 * CHAMFER * 0.5) as f32;
    [
        [-w + c, -h],
        [-w, -h + c],
        [-w, h - c],
        [-w + c, h],
        [w - c, h],
        [w, h - c],
        [w, -h + c],
        [w - c, -h],
    ]
}

/// A card's centre at card-loop time `local` in the given phase: x from
/// three card widths a slot and the form's half width (shifted), y from
/// the rise (scaled by unit/6). `None` for a card whose deal has not
/// begun (the loop stops there).
pub fn card_place(
    index: usize,
    form_width: u32,
    unit: i32,
    phase: i32,
    local: u32,
) -> Option<(f32, f32)> {
    let (w, _) = card_size(unit);
    let x = w * 3.0 * (index as i64 - 1) as f32 + (form_width >> 1) as f32;
    let start = index as i64 * i64::from(DEAL_STEP_MS);
    let y = if phase == phase::DEAL {
        if i64::from(local) < start {
            return None;
        }
        if (index as i64 + 1) * i64::from(DEAL_STEP_MS) < i64::from(local) {
            REST_Y
        } else {
            let t = ((i64::from(local) - start) as f64 / 600.0) as f32;
            ease(t) * RISE - 300.0
        }
    } else {
        REST_Y
    };
    Some((x, (f64::from(unit) / 6.0 * f64::from(y)) as f32))
}

/// A turning card's eased share at turn-clock `local`, or `None` before or
/// after its turn.
pub fn turn_share(index: usize, local: u32) -> Option<f32> {
    let start = index as i64 * i64::from(FLIP_STEP_MS);
    let local = i64::from(local);
    if local < start || (index as i64 + 1) * i64::from(FLIP_STEP_MS) < local {
        return None;
    }
    Some(ease(((local - start) as f64 / 900.0) as f32))
}

/// The engine's normalisation: times the processor's approximate
/// reciprocal square root of `(y² + 0) + (x² + z²)`; zero stays zero.
pub fn normalize(v: [f32; 3]) -> [f32; 3] {
    let sum = (v[1] * v[1] + 0.0) + (v[0] * v[0] + v[2] * v[2]);
    let scale = if sum == 0.0 {
        0.0
    } else {
        reciprocal_sqrt(sum)
    };
    [scale * v[0], scale * v[1], scale * v[2]]
}

/// The turn at eased share `share`: the axis `normalize(start + (end −
/// start) · share)` and the angle `share · π`.
pub fn turn(share: f32) -> ([f32; 3], f32) {
    let a = normalize(TURN_AXIS_START);
    let b = TURN_AXIS_END;
    let axis = [
        a[0] + (b[0] - a[0]) * share,
        a[1] + (b[1] - a[1]) * share,
        a[2] + (b[2] - a[2]) * share,
    ];
    (normalize(axis), share * std::f32::consts::PI)
}

/// One vertex of a card quad: its corner, its sheet coordinates and its
/// mask coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardVertex {
    pub at: [f32; 2],
    pub tex: [f32; 2],
    pub mask: [f32; 2],
}

/// The face of card `card` with half sizes `x`, `y` (a negative `x`
/// mirrors it): the cell's left column at `x`, its right at `−x`.
pub fn face_quad(card: u32, x: f32, y: f32) -> [CardVertex; 4] {
    let cell = card as i64 + i64::from(FIRST_CELL);
    let row = cell / 6;
    let column = cell % 6;
    let u0 = (column as f64 / 6.0) as f32;
    let u1 = ((column + 1) as f64 / 6.0) as f32;
    let v0 = 1.0 - (row + 1) as f32 * 0.25;
    let v1 = 1.0 - row as f32 * 0.25;
    [
        CardVertex {
            at: [x, -y],
            tex: [u0, v1],
            mask: MASK[0],
        },
        CardVertex {
            at: [x, y],
            tex: [u0, v0],
            mask: MASK[1],
        },
        CardVertex {
            at: [-x, y],
            tex: [u1, v0],
            mask: MASK[2],
        },
        CardVertex {
            at: [-x, -y],
            tex: [u1, v1],
            mask: MASK[3],
        },
    ]
}

/// The back with half sizes `x`, `y`: the sheet's cell from 1/6 to 1/3 of
/// the top row, its left at `−x`.
pub fn back_quad(x: f32, y: f32) -> [CardVertex; 4] {
    let (left, right) = (f32::from_bits(0x3e2a_aaab), f32::from_bits(0x3eaa_aaab));
    [
        CardVertex {
            at: [-x, -y],
            tex: [left, 1.0],
            mask: MASK[0],
        },
        CardVertex {
            at: [-x, y],
            tex: [left, 0.75],
            mask: MASK[1],
        },
        CardVertex {
            at: [x, y],
            tex: [right, 0.75],
            mask: MASK[2],
        },
        CardVertex {
            at: [x, -y],
            tex: [right, 1.0],
            mask: MASK[3],
        },
    ]
}

/// A 3 × 3 rotation, the engine's layout: a point maps to `x·m0 + y·m3 +
/// z·m6` across, `x·m1 + y·m4 + z·m7` down.
pub type Rotation = [f32; 9];
pub const IDENTITY: Rotation = [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0];

fn rotate(m: &Rotation, p: [f32; 2]) -> [f32; 2] {
    [
        p[0] * m[0] + p[1] * m[3] + 0.0 * m[6],
        p[0] * m[1] + p[1] * m[4] + 0.0 * m[7],
    ]
}

/// One shadow vertex: its place and its alpha byte (the colour is black).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowVertex {
    pub at: [f32; 2],
    pub alpha: u8,
}

/// The card's drop shadow as a triangle list: a fan over the rotated
/// outline at alpha 200, then for each edge a band four units wide fading
/// to 0 outward and a corner join along the two edges' half-way normal
/// lengthened by 1.1. The band points away from the card when the
/// rotation keeps its z axis, inward when it turns it over (`m8 ≤ 0`).
pub fn shadow_mesh(outline: &[[f32; 2]; 8], m: &Rotation, unit: i32) -> Vec<ShadowVertex> {
    let band = ((i64::from(unit)) << 2) as f32;
    let alpha = round_i64(1.0 * SHADOW_ALPHA) as u8;
    let mut out = Vec::new();
    let push = |out: &mut Vec<ShadowVertex>, at: [f32; 2], alpha: u8| {
        out.push(ShadowVertex { at, alpha });
    };
    let first = rotate(m, outline[0]);
    let mut previous = rotate(m, outline[1]);
    for point in outline.iter().skip(2) {
        let point = rotate(m, *point);
        push(&mut out, first, alpha);
        push(&mut out, point, alpha);
        push(&mut out, previous, alpha);
        previous = point;
    }
    let scale = |v: [f32; 2]| {
        let length = (v[1] * v[1] + v[0] * v[0]).sqrt();
        let s = if m[8] <= 0.0 {
            -band / length
        } else {
            band / length
        };
        [v[0] * s, v[1] * s]
    };
    let a = rotate(m, outline[6]);
    let mut b = rotate(m, outline[7]);
    let mut normal = scale([a[1] - b[1], b[0] - a[0]]);
    for point in outline {
        let p = rotate(m, *point);
        let before = normal;
        normal = scale([b[1] - p[1], p[0] - b[0]]);
        let out_b = [b[0] + normal[0], b[1] + normal[1]];
        let out_p = [p[0] + normal[0], p[1] + normal[1]];
        push(&mut out, b, alpha);
        push(&mut out, p, alpha);
        push(&mut out, out_p, 0);
        push(&mut out, out_p, 0);
        push(&mut out, out_b, 0);
        push(&mut out, b, alpha);
        let join = [
            (normal[0] + (before[0] - normal[0]) * 0.5) * JOIN_SCALE,
            (normal[1] + (before[1] - normal[1]) * 0.5) * JOIN_SCALE,
        ];
        push(&mut out, b, alpha);
        push(&mut out, out_b, 0);
        push(&mut out, [b[0] + join[0], b[1] + join[1]], 0);
        push(&mut out, [b[0] + join[0], b[1] + join[1]], 0);
        push(&mut out, [b[0] + before[0], b[1] + before[1]], 0);
        push(&mut out, b, alpha);
        b = p;
    }
    out
}

/// One step of the paint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardDraw {
    /// The full-form darkening at this alpha.
    Fade { alpha: u8 },
    /// `PprMov` sounds.
    Sound,
    /// Card `index` is placed at `x`, `y`.
    Place { index: usize, x: f32, y: f32 },
    /// Its shadow under the identity rotation.
    Shadow { index: usize },
    /// Its shadow and quads under the turn about `axis` by `angle`.
    Turn {
        index: usize,
        axis: [f32; 3],
        angle: f32,
    },
    /// Its back.
    Back { index: usize },
    /// Its face; `mirrored` draws it with the half width negated (a card
    /// lying face up after its turn).
    Face {
        index: usize,
        card: u32,
        mirrored: bool,
    },
}

/// One paint at `elapsed` since the reading opened: the darkening, the
/// sound step, then each card in turn until one whose deal has not begun.
/// A dealing card shows its shadow and back; a card waiting to turn the
/// same; a turning card its turned shadow, back and face; a turned or
/// held card its shadow and its face mirrored.
pub fn draw_order(
    reading: &mut Reading,
    form_width: u32,
    unit: i32,
    elapsed: u32,
) -> Vec<CardDraw> {
    let mut draw = vec![CardDraw::Fade {
        alpha: fade_alpha(elapsed),
    }];
    let (play, local) = clock_step(reading, elapsed);
    if play {
        draw.push(CardDraw::Sound);
    }
    for (index, &card) in reading.cards.iter().enumerate() {
        let Some((x, y)) = card_place(index, form_width, unit, reading.phase, local) else {
            break;
        };
        draw.push(CardDraw::Place { index, x, y });
        match reading.phase {
            phase::HOLD => {
                draw.push(CardDraw::Shadow { index });
                draw.push(CardDraw::Face {
                    index,
                    card,
                    mirrored: true,
                });
            }
            phase::FLIP => {
                let start = index as i64 * i64::from(FLIP_STEP_MS);
                if i64::from(local) < start {
                    draw.push(CardDraw::Shadow { index });
                    draw.push(CardDraw::Back { index });
                } else if let Some(share) = turn_share(index, local) {
                    let (axis, angle) = turn(share);
                    draw.push(CardDraw::Turn { index, axis, angle });
                    draw.push(CardDraw::Back { index });
                    draw.push(CardDraw::Face {
                        index,
                        card,
                        mirrored: false,
                    });
                } else {
                    draw.push(CardDraw::Shadow { index });
                    draw.push(CardDraw::Face {
                        index,
                        card,
                        mirrored: true,
                    });
                }
            }
            _ => {
                draw.push(CardDraw::Shadow { index });
                draw.push(CardDraw::Back { index });
            }
        }
    }
    draw
}

/// The mouse rule: the right button (mask bit 1) closes the reading and
/// returns 3; anything else goes to the base handler.
pub fn mouse(mask: u16) -> Option<i32> {
    if mask & 2 != 0 { Some(3) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_cards_are_dealt_distinctly() {
        let mut draws = [5u32, 5, 9, 5, 9, 7].into_iter();
        let cards = deal_cards(&mut |n| {
            assert_eq!(n, 22);
            draws.next().unwrap()
        });
        assert_eq!(cards, [5, 9, 7]);
        let mut next = 0;
        let reading = open(500, &mut |_| {
            next += 1;
            next
        });
        assert_eq!(reading.cards, [1, 2, 3]);
        assert_eq!(reading.start_ms, 500);
        assert_eq!(reading.phase, phase::DEAL);
        assert!(CARD_FRAGMENT_SHADER.contains("uniform sampler2D Tex0;"));
    }

    #[test]
    fn the_ease_is_the_engines_smooth_step() {
        assert_eq!(ease(0.0), 0.0);
        assert_eq!(ease(0.5), 0.5);
        assert_eq!(ease(1.0), 1.0);
        assert_eq!(ease(0.25), 0.15625);
    }

    #[test]
    fn the_darkening_rises_to_128_over_a_second() {
        assert_eq!(fade_alpha(0), 0);
        assert_eq!(fade_alpha(500), 64);
        // 4 ms: 0.512 rounds to 1; 3 ms: 0.384 to 0.
        assert_eq!(fade_alpha(4), 1);
        assert_eq!(fade_alpha(3), 0);
        assert_eq!(fade_alpha(1_000), 128);
        assert_eq!(fade_alpha(5_000), 128);
    }

    #[test]
    fn the_clock_sounds_each_deal_step_and_the_first_three_turns() {
        let mut reading = reading([0, 1, 2]);
        assert_eq!(clock_step(&mut reading, 0), (true, 0));
        assert_eq!(clock_step(&mut reading, 599), (false, 599));
        assert_eq!(clock_step(&mut reading, 600), (true, 600));
        assert_eq!(clock_step(&mut reading, 1_800), (true, 1_800));
        assert_eq!(reading.step, 4);
        // The turns reset the step: their first sounds.
        assert_eq!(clock_step(&mut reading, 1_801), (true, 1));
        assert_eq!(reading.phase, phase::FLIP);
        assert_eq!(clock_step(&mut reading, 2_700), (true, 900));
        assert_eq!(clock_step(&mut reading, 3_600), (true, 1_800));
        assert_eq!(clock_step(&mut reading, 4_500), (false, 2_700));
        assert_eq!(reading.step, 4);
        assert_eq!(clock_step(&mut reading, 4_501), (false, 4_501));
        assert_eq!(reading.phase, phase::HOLD);
    }

    fn reading(cards: [u32; 3]) -> Reading {
        Reading {
            cards,
            start_ms: 0,
            phase: phase::DEAL,
            step: 0,
        }
    }

    #[test]
    fn cards_are_twenty_by_thirty_three_units_and_chamfered() {
        let (w, h) = card_size(6);
        assert_eq!((w, h), (120.0, 200.0));
        let (w, h) = card_size(4);
        assert_eq!(w, 80.0);
        assert!((h - 133.333_33).abs() < 1e-4);
        let outline = shadow_outline(6);
        assert_eq!(outline[0], [-106.0, -200.0]);
        assert_eq!(outline[1], [-120.0, -186.0]);
        assert_eq!(outline[4], [106.0, 200.0]);
        assert_eq!(outline[7], [106.0, -200.0]);
    }

    #[test]
    fn a_card_rises_by_the_ease_and_rests_at_four_hundred() {
        // Unit 6: card 0 at 600 / 2 − 360, y scaled by 1.
        assert_eq!(card_place(0, 601, 6, phase::DEAL, 0), Some((-60.0, -300.0)));
        assert_eq!(card_place(0, 601, 6, phase::DEAL, 300), Some((-60.0, 50.0)));
        assert_eq!(
            card_place(0, 601, 6, phase::DEAL, 601),
            Some((-60.0, 400.0))
        );
        // Card 1 has not begun at 599: the paint stops.
        assert_eq!(card_place(1, 601, 6, phase::DEAL, 599), None);
        assert_eq!(
            card_place(1, 601, 6, phase::DEAL, 600),
            Some((300.0, -300.0))
        );
        assert_eq!(card_place(2, 600, 6, phase::FLIP, 0), Some((660.0, 400.0)));
        // Unit 3 halves the y.
        assert_eq!(card_place(2, 600, 3, phase::HOLD, 0), Some((480.0, 200.0)));
    }

    #[test]
    fn a_turn_swings_its_axis_over_a_half_turn() {
        assert_eq!(turn_share(0, 0), Some(0.0));
        assert_eq!(turn_share(0, 450), Some(0.5));
        assert_eq!(turn_share(0, 901), None);
        assert_eq!(turn_share(1, 899), None);
        let (axis, angle) = turn(1.0);
        assert!((axis[1] - 1.0).abs() < 1e-3 && axis[0].abs() < 1e-6);
        assert_eq!(angle, std::f32::consts::PI);
        let (axis, angle) = turn(0.0);
        let length = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
        assert!((length - 1.0).abs() < 1e-3);
        assert!(axis[0] > 0.3 && axis[1] > 0.9);
        assert_eq!(angle, 0.0);
        assert_eq!(normalize([0.0; 3]), [0.0; 3]);
    }

    #[test]
    fn the_face_and_back_take_their_sheet_cells() {
        // Card 4 is cell 6: the second row's first column.
        let face = face_quad(4, 10.0, 20.0);
        assert_eq!(face[0].at, [10.0, -20.0]);
        assert_eq!(face[0].tex, [0.0, 0.75]);
        assert_eq!(face[1].tex, [0.0, 0.5]);
        assert_eq!(face[2].at, [-10.0, 20.0]);
        assert_eq!(face[2].tex[0], (1.0f64 / 6.0) as f32);
        assert_eq!(face[3].mask, MASK[3]);
        // Card 0 is cell 2 of the top row.
        let face = face_quad(0, 10.0, 20.0);
        assert_eq!(face[0].tex, [(2.0f64 / 6.0) as f32, 1.0]);
        let back = back_quad(10.0, 20.0);
        assert_eq!(back[0].at, [-10.0, -20.0]);
        assert_eq!(back[0].tex, [1.0 / 6.0, 1.0]);
        assert_eq!(back[2].tex, [1.0 / 3.0, 0.75]);
    }

    #[test]
    fn the_shadow_is_a_fan_with_a_fading_band_and_joins() {
        let outline = shadow_outline(6);
        let mesh = shadow_mesh(&outline, &IDENTITY, 6);
        // Six fan triangles, then twelve vertices per edge.
        assert_eq!(mesh.len(), 18 + 8 * 12);
        assert_eq!(
            mesh[0],
            ShadowVertex {
                at: outline[0],
                alpha: 200
            }
        );
        assert_eq!(mesh[1].at, outline[2]);
        assert_eq!(mesh[2].at, outline[1]);
        // The first band runs from the last point to the first, along the
        // top edge: its normal points up (−y) by four units (24 px).
        let band = &mesh[18..30];
        assert_eq!(band[0].at, outline[7]);
        assert_eq!(band[1].at, outline[0]);
        assert_eq!(
            band[2],
            ShadowVertex {
                at: [outline[0][0], -224.0],
                alpha: 0
            }
        );
        assert_eq!(band[4].at, [outline[7][0], -224.0]);
        // Turned over, the band points the other way.
        let mut flipped = IDENTITY;
        flipped[8] = -1.0;
        let mesh = shadow_mesh(&outline, &flipped, 6);
        assert_eq!(mesh[18 + 2].at, [outline[0][0], -176.0]);
    }

    #[test]
    fn the_paint_follows_the_phases() {
        let mut reading = reading([3, 8, 13]);
        let draw = draw_order(&mut reading, 600, 6, 300);
        assert_eq!(draw[0], CardDraw::Fade { alpha: 38 });
        assert_eq!(draw[1], CardDraw::Sound);
        assert_eq!(
            draw[2],
            CardDraw::Place {
                index: 0,
                x: -60.0,
                y: 50.0
            }
        );
        assert_eq!(draw[3], CardDraw::Shadow { index: 0 });
        assert_eq!(draw[4], CardDraw::Back { index: 0 });
        // The second card has not begun.
        assert_eq!(draw.len(), 5);
        // Mid-turn of the first card, the others waiting.
        let draw = draw_order(&mut reading, 600, 6, 1_800 + 450);
        assert!(matches!(draw[3], CardDraw::Turn { index: 0, .. }));
        assert_eq!(draw[4], CardDraw::Back { index: 0 });
        assert_eq!(
            draw[5],
            CardDraw::Face {
                index: 0,
                card: 3,
                mirrored: false
            }
        );
        assert_eq!(draw[7], CardDraw::Shadow { index: 1 });
        assert_eq!(draw[8], CardDraw::Back { index: 1 });
        // Held: every card face up, mirrored.
        let draw = draw_order(&mut reading, 600, 6, 5_000);
        assert_eq!(draw.len(), 1 + 9);
        assert_eq!(
            draw[3],
            CardDraw::Face {
                index: 0,
                card: 3,
                mirrored: true
            }
        );
        assert_eq!(mouse(2), Some(3));
        assert_eq!(mouse(1), None);
    }
}

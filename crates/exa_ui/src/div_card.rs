//! The divination card reading: three distinct cards out of the twenty-two
//! on the sheet, dealt in, flipped one after another and held.
//!
//! Evidence: the reading's open and paint routines in the retail image. The
//! sheet `divcardsa1.rfi` holds a six-column, four-row grid; a card at index
//! `i` takes the cell `(i + 2) mod 6` of row `(i + 2) / 6`, inset by one
//! pixel. Three card indices are drawn from 0 to 21, each redrawn until it
//! differs from every earlier one. A card is twenty units wide and
//! thirty-three and a third tall. The reading fades in over one second,
//! deals the cards in over 600 ms each, flips them over 900 ms each and
//! holds from 4.5 s; a sound fires at the start of each step. Times are
//! milliseconds, positions pixels.

/// The card sheet.
pub const SHEET: &str = "divcardsa1.rfi";
/// The domain icon sheet.
pub const ICON_SHEET: &str = "thmmndico1.rfi";
/// The sound the deal and the flips play.
pub const SOUND: &str = "PprMov";
/// The number of cards on the sheet.
pub const CARDS: u32 = 0x16;
/// The sheet's columns and rows.
pub const SHEET_COLUMNS: i32 = 6;
pub const SHEET_ROWS: i32 = 4;
/// The first card takes this cell of the sheet.
pub const FIRST_CELL: i32 = 2;
/// A card's width in units.
pub const CARD_WIDTH_UNITS: f64 = 480.0 * 0.25 / 6.0;
/// A card's height in units.
pub const CARD_HEIGHT_UNITS: f64 = 800.0 * 0.25 / 6.0;
/// The shadow's half thickness in units.
pub const SHADOW_HALF_UNITS: f64 = 28.0 * 0.5 / 6.0;
/// The fade-in takes this long.
pub const FADE_MS: u32 = 1_000;
/// The fade's full alpha.
pub const FADE_ALPHA: f32 = 128.0;
/// The deal phase ends and the flip phase starts here.
pub const FLIP_START_MS: u32 = 1_800;
/// The hold starts here.
pub const HOLD_MS: u32 = 4_500;
/// One card's deal.
pub const DEAL_STEP_MS: u32 = 600;
/// One card's flip.
pub const FLIP_STEP_MS: u32 = 900;
/// A dealt card's resting y.
pub const REST_Y: f32 = 400.0;
/// A card's starting y.
pub const START_Y: f32 = -300.0;
/// The rise's amplitude.
pub const RISE: f32 = 700.0;
/// The paper burst's width in units (4.6).
pub const BURST_UNITS: f64 = 4.6;
/// The burst's second, wider copy's scale.
pub const BURST_SCALE: f32 = 1.1;
/// The paper masks' first texture column.
pub const MASK_X: f32 = 0.166_319_44;
/// The mask's inset x.
pub const MASK_INSET_X: f32 = 0.000_347_222_23;
/// The mask's top and bottom.
pub const MASK_TOP: f32 = 0.999_687_5;
pub const MASK_BOTTOM: f32 = 0.750_312_5;

/// The form's style and flag words.
pub const FORM_STYLE: u32 = 0xf_8800;
pub const FORM_FLAGS: u32 = 0x8_1000;
/// The paper shader's program slot.
pub const SHADER_PROGRAM: u32 = 0x35;
/// The card quad shader's texture unit.
pub const CARD_TEXTURE_UNIT: u32 = 0;
/// The reading's two shaders.
pub const CARD_VERTEX_SHADER: &str = "in vec3 vPos;\r\nin vec2 vTex;\r\nin vec2 vTx2;\r\nin vec4 vCol;\r\nout vec2 vrTex;\r\nout vec2 vrTx2;\r\nout vec4 vrCol;\r\nuniform mat4 MPrj;\r\nuniform mat4 MObj;\r\nvoid main() {\r\ngl_Position = MPrj * vec4(vPos,1.0);\r\nvrTex = vTex;\r\nvrTx2 = vTx2;\r\nvrCol = vCol * 0.003921568;\r\n}";
pub const CARD_FRAGMENT_SHADER: &str = "in vec2 vrTex;\r\nin vec2 vrTx2;\r\nin vec4 vrCol;\r\nout vec4 FragColor;\r\nuniform vec4 par0;\r\nuniform sampler2D Tex0;\r\nvoid main() {\r\nvec3 c = texture(Tex0, vrTex).rgb;\r\nc = c * (0.2 + abs(par0.z) * 0.8);\r\nFragColor.rgb = vrCol.rgb * c;\r\nfloat a = texture(Tex0, vrTx2).g;\r\nFragColor.a = vrCol.a * vrCol.a * a;\r\n}";

/// A reading that has just been opened: its three cards, its material and
/// the moment it starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Reading {
    pub cards: [u32; 3],
    pub sheet: &'static str,
    pub sound: &'static str,
    pub start_ms: u32,
}

/// Opens a reading: three distinct cards, the sheet and sound, starting at
/// `now`.
pub fn open(now: u32, random: &mut impl FnMut(u32) -> u32) -> Reading {
    Reading {
        cards: deal_cards(random),
        sheet: SHEET,
        sound: SOUND,
        start_ms: now,
    }
}

/// The reading's phase.
pub mod phase {
    /// The deal.
    pub const DEAL: i32 = 1;
    /// The flips.
    pub const FLIP: i32 = 2;
    /// The held reading.
    pub const HOLD: i32 = 3;
}

/// Deals three distinct cards out of the sheet. The card at each index is
/// redrawn from 0..21 until it differs from every earlier card.
pub fn deal_cards(random: &mut impl FnMut(u32) -> u32) -> [u32; 3] {
    let mut cards = [0u32; 3];
    for i in 0..3 {
        loop {
            let card = random(CARDS);
            if cards[..i].contains(&card) {
                continue;
            }
            cards[i] = card;
            break;
        }
    }
    cards
}

/// The fade-in alpha: 128 after a second, else the elapsed share of a second
/// times 128, rounded to even, masked to a byte.
pub fn fade_alpha(elapsed: u32) -> u8 {
    let alpha = if elapsed > FADE_MS {
        FADE_ALPHA
    } else {
        ((f64::from(elapsed) / 1_000.0) * 128.0).round_ties_even() as f32
    };
    (alpha.round_ties_even() as i64 & 0xff) as u8
}

/// The phase at an elapsed time.
pub fn phase_at(elapsed: u32) -> i32 {
    if elapsed > HOLD_MS {
        phase::HOLD
    } else if elapsed > FLIP_START_MS {
        phase::FLIP
    } else {
        phase::DEAL
    }
}

/// The sound step at an elapsed time: the stored step and whether it just
/// changed. In the deal the steps change every 600 ms; in the flips every
/// 900 ms, only the first four steps sounding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundStep {
    pub step: u32,
    pub play: bool,
}

/// The sound step a paint stores.
pub fn sound_step(elapsed: u32, stored: u32) -> SoundStep {
    match phase_at(elapsed) {
        phase::DEAL => {
            let step = elapsed / DEAL_STEP_MS + 1;
            SoundStep {
                step,
                play: step != stored,
            }
        }
        phase::FLIP => {
            let step = (elapsed - FLIP_START_MS) / FLIP_STEP_MS + 1;
            SoundStep {
                step,
                play: step != stored && step < 4,
            }
        }
        _ => SoundStep {
            step: stored,
            play: false,
        },
    }
}

/// The card size in pixels.
pub fn card_size(unit: i32) -> (f32, f32) {
    (
        (f64::from(unit) * CARD_WIDTH_UNITS) as f32,
        (f64::from(unit) * CARD_HEIGHT_UNITS) as f32,
    )
}

/// The sheet cell a card shows: the sub-rectangle's two texture corners.
pub fn sheet_cell(card: u32) -> [[f32; 2]; 2] {
    let cell = card as i32 + FIRST_CELL;
    let row = cell / SHEET_COLUMNS;
    let column = cell % SHEET_COLUMNS;
    let u = column as f32 / SHEET_COLUMNS as f32;
    let u2 = (column + 1) as f32 / SHEET_COLUMNS as f32;
    let v = 1.0 - (row + 1) as f32 * 0.25;
    let v2 = 1.0 - row as f32 * 0.25;
    [[u, v], [u2, v2]]
}

/// A card's position at an elapsed time: its centre x from the form's
/// width and its centre y from the deal's rise, or 400 once it rests.
pub fn card_place(index: usize, form_width: i32, unit: i32, elapsed: u32) -> (f32, f32) {
    let (width, _) = card_size(unit);
    let x = width * 3.0 * (index as f32 - 1.0) + form_width as f32 / 2.0;
    let y = if phase_at(elapsed) != phase::DEAL {
        REST_Y
    } else {
        let start = index as u32 * DEAL_STEP_MS;
        if start > elapsed {
            // The card has not started: the paint returns before it.
            return (x, START_Y);
        }
        if (index as u32 + 1) * DEAL_STEP_MS < elapsed {
            REST_Y
        } else {
            let t = (f64::from(elapsed - start) / f64::from(DEAL_STEP_MS)) as f32;
            quarter_rise(t)
        }
    };
    (x, y)
}

/// The deal's rise: `sin(t · π/2)·700 − 300` (the original's quartern-turn
/// sine helper, which the resting y of 400 completes at t = 1).
pub fn quarter_rise(t: f32) -> f32 {
    (f64::from(t) * std::f64::consts::FRAC_PI_2).sin() as f32 * RISE + START_Y
}

/// A card's deal has begun.
pub fn deal_started(index: usize, elapsed: u32) -> bool {
    (index as u32) * DEAL_STEP_MS <= elapsed
}

/// A card's flip progress at an elapsed time: 0 before it, 1 after it, and
/// the quarter-turn sine in between.
pub fn flip_progress(index: usize, elapsed: u32) -> f32 {
    let start = index as u32 * FLIP_STEP_MS;
    if elapsed <= start {
        0.0
    } else if elapsed >= start + FLIP_STEP_MS {
        1.0
    } else {
        let t = (f64::from(elapsed - start) / f64::from(FLIP_STEP_MS)) as f32;
        (f64::from(t) * std::f64::consts::FRAC_PI_2).sin() as f32
    }
}

/// The flip's scale on x: `cos(progress · π)`, from 1 through 0 to −1.
pub fn flip_scale(progress: f32) -> f32 {
    (f64::from(progress) * std::f64::consts::PI).cos() as f32
}

/// The card's eight outline points in its own space: an octagon with the
/// two long sides and the two short ones chamfered by 28 units.
pub fn shadow_outline(unit: i32) -> [[f32; 2]; 8] {
    let (width, height) = card_size(unit);
    let chamfer = (f64::from(unit) * SHADOW_HALF_UNITS) as f32;
    [
        [-width + chamfer, -height],
        [-width, -height + chamfer],
        [-width, height - chamfer],
        [-width + chamfer, height],
        [width - chamfer, height],
        [width, height - chamfer],
        [width, -height + chamfer],
        [width - chamfer, -height],
    ]
}

/// The paper burst's width in pixels.
pub fn burst_width(unit: i32) -> f32 {
    (f64::from(unit) * BURST_UNITS) as f32
}

/// A quad: four corners and the two texture-coordinate sets.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CardQuad {
    pub corners: [[f32; 2]; 4],
    pub tex: [[f32; 2]; 4],
    pub tex_alpha: [[f32; 2]; 4],
}

/// The card's face: the four corners at ±the card size and the sheet cell's
/// coordinates. The two x coordinates carry the flip (the sign bit makes
/// the quad face the other way).
pub fn face_quad(index: usize, unit: i32, flip_scale: f32) -> CardQuad {
    let (width, height) = card_size(unit);
    let (w, h) = (width * flip_scale, height);
    let cells = sheet_cell(index as u32);
    CardQuad {
        corners: [[-w, h], [-w, -h], [w, -h], [w, h]],
        tex: [
            [cells[0][0], cells[1][1]],
            [cells[0][0], cells[0][1]],
            [cells[1][0], cells[0][1]],
            [cells[1][0], cells[1][1]],
        ],
        tex_alpha: [
            [MASK_INSET_X, MASK_TOP],
            [MASK_INSET_X, MASK_BOTTOM],
            [MASK_X, MASK_BOTTOM],
            [MASK_X, MASK_TOP],
        ],
    }
}

/// The card's back: the same box, the mask's first column of the sheet.
pub fn back_quad(index: usize, unit: i32, flip_scale: f32) -> CardQuad {
    let (width, height) = card_size(unit);
    let (w, h) = (width * flip_scale, height);
    let _ = index;
    CardQuad {
        corners: [[-w, h], [-w, -h], [w, -h], [w, h]],
        // 1/6 to 1/3 of the sheet, the top quarter.
        tex: [
            [1.0 / 6.0, 1.0],
            [1.0 / 6.0, 0.75],
            [1.0 / 3.0, 0.75],
            [1.0 / 3.0, 1.0],
        ],
        tex_alpha: [
            [MASK_INSET_X, MASK_TOP],
            [MASK_INSET_X, MASK_BOTTOM],
            [MASK_X, MASK_BOTTOM],
            [MASK_X, MASK_TOP],
        ],
    }
}

/// The paint's steps, in order. The sheet's texture and the paper shader
/// wrap the whole run.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CardDraw {
    BindSheet,
    Fade {
        alpha: u8,
    },
    /// The octagonal paper burst around card `index` at `y`.
    Paper {
        index: usize,
        card: u32,
        x: f32,
        y: f32,
    },
    /// The card's back.
    Back {
        index: usize,
        card: u32,
        x: f32,
        y: f32,
    },
    /// The card's face.
    Face {
        index: usize,
        card: u32,
        x: f32,
        y: f32,
    },
    Clear,
}

/// The whole paint: every card's burst, back and face while the deal or the
/// flips run, faces alone once held. A card not yet dealt is skipped.
pub fn draw_order(cards: [u32; 3], form_width: i32, unit: i32, elapsed: u32) -> Vec<CardDraw> {
    let mut draw = vec![
        CardDraw::BindSheet,
        CardDraw::Fade {
            alpha: fade_alpha(elapsed),
        },
    ];
    let phase = phase_at(elapsed);
    for (index, &card) in cards.iter().enumerate() {
        if phase == phase::DEAL && !deal_started(index, elapsed) {
            break;
        }
        let (x, y) = card_place(index, form_width, unit, elapsed);
        draw.push(CardDraw::Paper { index, card, x, y });
        if phase != phase::HOLD {
            draw.push(CardDraw::Back { index, card, x, y });
        }
        draw.push(CardDraw::Face { index, card, x, y });
    }
    draw.push(CardDraw::Clear);
    draw
}

/// The mouse rule: a click (mask bit 1) closes the reading and returns 3;
/// anything else goes to the base handler (0).
pub fn mouse(mask: u16) -> Option<i32> {
    if mask & 2 != 0 { Some(3) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_cards_are_dealt_distinctly() {
        // First draws 5, 5 again (rejected), 9, 5 again (rejected), 7.
        let mut draws = [5u32, 5, 9, 5, 7].into_iter();
        let cards = deal_cards(&mut |_| draws.next().unwrap());
        assert_eq!(cards, [5, 9, 7]);
        assert_eq!(CARDS, 22);
        assert_eq!(SHEET, "divcardsa1.rfi");
        assert_eq!(ICON_SHEET, "thmmndico1.rfi");
        assert_eq!(SOUND, "PprMov");
    }

    #[test]
    fn opening_loads_the_sheet_the_sound_and_three_cards() {
        let mut draws = [0u32, 1, 2].into_iter();
        let reading = open(500, &mut |_| draws.next().unwrap());
        assert_eq!(reading.cards, [0, 1, 2]);
        assert_eq!(reading.sheet, "divcardsa1.rfi");
        assert_eq!(reading.sound, "PprMov");
        assert_eq!(reading.start_ms, 500);
        assert_eq!(FORM_STYLE, 0xf_8800);
        assert_eq!(FORM_FLAGS, 0x8_1000);
        assert!(CARD_FRAGMENT_SHADER.contains("uniform sampler2D Tex0;"));
        assert!(CARD_VERTEX_SHADER.contains("uniform mat4 MObj;"));
    }

    #[test]
    fn the_fade_rises_to_128_over_a_second() {
        assert_eq!(fade_alpha(0), 0);
        assert_eq!(fade_alpha(500), 64);
        assert_eq!(fade_alpha(999), 128);
        assert_eq!(fade_alpha(1_000), 128);
        assert_eq!(fade_alpha(5_000), 128);
    }

    #[test]
    fn the_phases_and_their_sound_steps() {
        assert_eq!(phase_at(0), phase::DEAL);
        assert_eq!(phase_at(1_800), phase::DEAL);
        assert_eq!(phase_at(1_801), phase::FLIP);
        assert_eq!(phase_at(4_500), phase::FLIP);
        assert_eq!(phase_at(4_501), phase::HOLD);
        assert_eq!(
            sound_step(0, 0),
            SoundStep {
                step: 1,
                play: true
            }
        );
        assert_eq!(
            sound_step(599, 1),
            SoundStep {
                step: 1,
                play: false
            }
        );
        assert_eq!(
            sound_step(600, 1),
            SoundStep {
                step: 2,
                play: true
            }
        );
        // Flips: 900 ms steps, at most four sounds.
        assert_eq!(
            sound_step(1_801, 3),
            SoundStep {
                step: 1,
                play: true
            }
        );
        assert_eq!(
            sound_step(2_700, 1),
            SoundStep {
                step: 2,
                play: true
            }
        );
        assert_eq!(
            sound_step(3_600, 2),
            SoundStep {
                step: 3,
                play: true
            }
        );
        assert_eq!(
            sound_step(4_500, 3),
            SoundStep {
                step: 4,
                play: false
            }
        );
        assert_eq!(
            sound_step(4_500, 4),
            SoundStep {
                step: 4,
                play: false
            }
        );
    }

    #[test]
    fn cards_are_twenty_by_thirty_three_and_a_third_units() {
        assert_eq!(card_size(6), (120.0, 200.0));
        assert!((card_size(4).0 - 80.0).abs() < 1e-6);
        assert!((card_size(4).1 - 133.333_33).abs() < 1e-3);
        assert_eq!(sheet_cell(0), [[1.0 / 3.0, 0.75], [0.5, 1.0]]);
        assert_eq!(sheet_cell(3), [[5.0 / 6.0, 0.75], [1.0, 1.0]]);
        assert_eq!(sheet_cell(4), [[0.0, 0.5], [1.0 / 6.0, 0.75]]);
        assert_eq!(sheet_cell(21), [[5.0 / 6.0, 0.0], [1.0, 0.25]]);
    }

    #[test]
    fn a_card_rises_and_rests_on_its_own_step() {
        // Card 0 begins at once; card 1 at 600 ms; card 2 at 1 200 ms.
        assert_eq!(card_place(0, 600, 4, 0).1, START_Y);
        assert_eq!(card_place(0, 600, 4, 300).1, quarter_rise(0.5));
        assert_eq!(card_place(0, 600, 4, 600).1, REST_Y);
        assert_eq!(card_place(1, 600, 4, 599).1, START_Y);
        assert!(deal_started(1, 600));
        assert!(!deal_started(1, 599));
        assert_eq!(card_place(2, 600, 4, 1_199).1, START_Y);
        // x is three widths apart around the centre.
        assert_eq!(card_place(0, 1_000, 4, 1_800).0, 260.0);
        assert_eq!(card_place(1, 1_000, 4, 1_800).0, 500.0);
        assert_eq!(card_place(2, 1_000, 4, 1_800).0, 740.0);
        assert!((quarter_rise(1.0) - 400.0).abs() < 1e-4);
    }

    #[test]
    fn a_flip_turns_the_card_through_a_half_turn() {
        assert_eq!(flip_progress(0, 0), 0.0);
        assert!((flip_progress(0, 450) - 0.707_106_77).abs() < 1e-6);
        assert_eq!(flip_progress(0, 900), 1.0);
        assert_eq!(flip_scale(0.0), 1.0);
        assert!(flip_scale(1.0) < -0.999);
        assert_eq!(flip_progress(1, 900), 0.0);
        assert!((flip_progress(1, 1_350) - 0.707_106_77).abs() < 1e-6);
        assert_eq!(flip_progress(1, 1_800), 1.0);
    }

    #[test]
    fn the_outline_is_a_chamfered_octagon() {
        let points = shadow_outline(6);
        // Width 120, height 200, chamfer 14.
        assert_eq!(
            points,
            [
                [-106.0, -200.0],
                [-120.0, -186.0],
                [-120.0, 186.0],
                [-106.0, 200.0],
                [106.0, 200.0],
                [120.0, 186.0],
                [120.0, -186.0],
                [106.0, -200.0]
            ]
        );
        assert_eq!(burst_width(5), 23.0);
        assert_eq!(BURST_SCALE, 1.1);
    }

    #[test]
    fn the_face_uses_the_cards_cell_and_the_mask_strip() {
        let face = face_quad(0, 6, 1.0);
        assert_eq!(
            face.corners,
            [
                [-120.0, 200.0],
                [-120.0, -200.0],
                [120.0, -200.0],
                [120.0, 200.0]
            ]
        );
        assert_eq!(face.tex[0], [1.0 / 3.0, 1.0]);
        assert_eq!(face.tex[2], [0.5, 0.75]);
        assert_eq!(face.tex_alpha[0], [MASK_INSET_X, MASK_TOP]);
        let back = back_quad(2, 6, -0.5);
        assert_eq!(back.corners[0], [60.0, 200.0]);
        assert_eq!(back.tex[1], [1.0 / 6.0, 0.75]);
        assert_eq!(back.tex[2], [1.0 / 3.0, 0.75]);
    }

    #[test]
    fn the_paint_deals_then_adds_backs_and_faces() {
        let deal = draw_order([0, 1, 2], 600, 4, 0);
        assert_eq!(deal[0], CardDraw::BindSheet);
        assert_eq!(deal[1], CardDraw::Fade { alpha: 0 });
        assert_eq!(
            deal[2],
            CardDraw::Paper {
                index: 0,
                card: 0,
                x: 60.0,
                y: -300.0
            }
        );
        assert_eq!(deal.len(), 2 + 3 + 1);
        // At 600 the second card joins; both keep their backs.
        let later = draw_order([0, 1, 2], 600, 4, 600);
        assert_eq!(later.len(), 2 + 2 * 3 + 1);
        let all = draw_order([0, 1, 2], 600, 4, 1_200);
        assert_eq!(all.len(), 2 + 3 * 3 + 1);
        // Held: no backs, just the three faces.
        let held = draw_order([0, 1, 2], 600, 4, 4_501);
        assert_eq!(held.len(), 2 + 3 * 2 + 1);
        assert!(matches!(held[2], CardDraw::Paper { .. }));
        assert!(matches!(held[3], CardDraw::Face { .. }));
        assert_eq!(held.last(), Some(&CardDraw::Clear));
    }

    #[test]
    fn a_click_closes_the_reading() {
        assert_eq!(mouse(2), Some(3));
        assert_eq!(mouse(3), Some(3));
        assert_eq!(mouse(0), None);
        assert_eq!(mouse(1), None);
    }
}

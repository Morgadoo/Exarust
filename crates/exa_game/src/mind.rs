//! The Mind power's spirit form, as `TPwrSpiritForm` runs it
//! (`0x1001e1690`): a value charged to 1, held, or faded to 0, published as
//! a strength and a smooth body fade.
//!
//! - **Charge** (state 3): the value rises by the elapsed milliseconds over
//!   30 s. It stops at 1 and becomes held (state 1).
//! - **Hold** (state 1): the value is 1.
//! - **Fade** (state 2): the value falls by the elapsed milliseconds over
//!   30 s. Below 0.002 it drops to 0 and the form ends (state 0).
//! - **Publish**: in the fade the strength is `1000·value`. Otherwise the
//!   smooth value is `1 − (1 − value)²`, the strength is
//!   `(1 − smooth)·rate + 1000·smooth`, and the body vector scales by
//!   `1 − value`.

/// Milliseconds for the spirit form to charge or fade by one.
pub const SPIRIT_SPAN_MS: f64 = 30_000.0;
/// The strength at a full value.
pub const SPIRIT_STRENGTH: f32 = 1000.0;
/// The value below which a fade ends.
pub const SPIRIT_FLOOR: f64 = 0.002;

/// The spirit form's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiritForm {
    /// 0 ended, 1 held, 2 fading, 3 charging.
    pub state: i32,
    pub value: f32,
    pub stamp: i64,
}

/// One tick's published values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpiritStep {
    pub state: i32,
    pub value: f32,
    pub strength: f32,
    /// The smooth fade, absent while fading.
    pub smooth: Option<f32>,
    /// The body vector's scale, absent while fading.
    pub vector_scale: Option<f32>,
    /// Whether the form still runs.
    pub active: bool,
}

/// Steps the spirit form by the clock and publishes its strength.
pub fn step_spirit_form(form: &mut SpiritForm, clock: i64, rate: f32) -> SpiritStep {
    let mut active = true;
    match form.state {
        1 => form.value = 1.0,
        2 => {
            let elapsed = (clock - form.stamp) as f64 / SPIRIT_SPAN_MS;
            form.value -= elapsed as f32;
            if f64::from(form.value) < SPIRIT_FLOOR {
                form.state = 0;
                form.value = 0.0;
                active = false;
            }
        }
        3 => {
            let elapsed = (clock - form.stamp) as f64 / SPIRIT_SPAN_MS;
            form.value += elapsed as f32;
            if form.value > 1.0 {
                form.value = 1.0;
                form.state = 1;
            }
        }
        _ => {}
    }
    form.stamp = clock;
    if !active {
        return SpiritStep {
            state: form.state,
            value: form.value,
            strength: 0.0,
            smooth: None,
            vector_scale: None,
            active: false,
        };
    }
    if form.state == 2 {
        return SpiritStep {
            state: form.state,
            value: form.value,
            strength: SPIRIT_STRENGTH * form.value,
            smooth: None,
            vector_scale: None,
            active: true,
        };
    }
    let smooth = 1.0 - (1.0 - form.value) * (1.0 - form.value);
    SpiritStep {
        state: form.state,
        value: form.value,
        strength: (1.0 - smooth) * rate + SPIRIT_STRENGTH * smooth,
        smooth: Some(smooth),
        vector_scale: Some(1.0 - form.value),
        active: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn form(state: i32, value: f32, stamp: i64) -> SpiritForm {
        SpiritForm {
            state,
            value,
            stamp,
        }
    }

    #[test]
    fn a_held_form_publishes_full_strength() {
        let mut held = form(1, 0.3, 0);
        let step = step_spirit_form(&mut held, 100, 100.0);
        assert_eq!(step.state, 1);
        assert_eq!(step.value, 1.0);
        assert_eq!(step.strength, 1000.0);
        assert_eq!(step.smooth, Some(1.0));
        assert_eq!(step.vector_scale, Some(0.0));
        assert!(step.active);
    }

    #[test]
    fn a_fade_falls_over_thirty_seconds_and_ends() {
        let mut fading = form(2, 1.0, 0);
        let half = step_spirit_form(&mut fading, 15_000, 100.0);
        assert_eq!(half.state, 2);
        assert!((half.value - 0.5).abs() < 1e-6);
        assert_eq!(half.strength, 500.0);
        assert_eq!(half.smooth, None);

        let mut gone = form(2, 1.0, 0);
        let end = step_spirit_form(&mut gone, 30_000, 100.0);
        assert!(!end.active);
        assert_eq!(end.state, 0);
        assert_eq!(end.value, 0.0);
    }

    #[test]
    fn a_charge_rises_and_clamps_at_one() {
        let mut charging = form(3, 0.0, 0);
        let half = step_spirit_form(&mut charging, 15_000, 100.0);
        assert_eq!(half.state, 3);
        assert!((half.value - 0.5).abs() < 1e-6);
        assert_eq!(half.smooth, Some(0.75));
        assert!((half.strength - 775.0).abs() < 1e-3);

        let full = step_spirit_form(&mut charging, 40_000, 100.0);
        assert_eq!(full.state, 1);
        assert_eq!(full.value, 1.0);
    }
}

/// The mask and sense powers' control state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MindEffect {
    /// 2 means the effect is being removed; any other value keeps it on.
    pub state: i32,
}

/// The mask's flag result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MindMaskStep {
    pub state: i32,
    pub active: bool,
    pub flags: u32,
}

/// The sense's trigger result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindSenseStep {
    pub state: i32,
    pub active: bool,
    pub triggered: bool,
}

/// Steps the mind mask: an event 2 starts its removal; otherwise state 2
/// clears the `0x10000` body flag and an active mask sets it.
pub fn step_mind_mask(effect: &mut MindEffect, event: i32, flags: u32) -> MindMaskStep {
    if event == 2 {
        effect.state = 2;
    }
    if effect.state == 2 {
        MindMaskStep {
            state: effect.state,
            active: false,
            flags: flags & !0x10000,
        }
    } else {
        MindMaskStep {
            state: effect.state,
            active: true,
            flags: flags | 0x10000,
        }
    }
}

/// Steps the mind sense: an event 2 starts its removal; in state 2 it
/// triggers while the stored value is above zero.
pub fn step_mind_sense(effect: &mut MindEffect, event: i32, value: f32) -> MindSenseStep {
    if event == 2 {
        effect.state = 2;
    }
    if effect.state == 2 {
        MindSenseStep {
            state: effect.state,
            active: true,
            triggered: value > 0.0,
        }
    } else {
        MindSenseStep {
            state: effect.state,
            active: false,
            triggered: false,
        }
    }
}

#[cfg(test)]
mod effect_tests {
    use super::*;

    #[test]
    fn the_mask_sets_and_clears_its_body_flag() {
        let mut effect = MindEffect { state: 0 };
        let on = step_mind_mask(&mut effect, 0, 0);
        assert!(on.active);
        assert_eq!(on.flags, 0x10000);

        let off = step_mind_mask(&mut effect, 2, 0x30000);
        assert!(!off.active);
        assert_eq!(off.state, 2);
        assert_eq!(off.flags, 0x20000);
    }

    #[test]
    fn the_sense_triggers_only_while_removing_with_a_positive_value() {
        let mut effect = MindEffect { state: 0 };
        let idle = step_mind_sense(&mut effect, 0, 5.0);
        assert!(!idle.active && !idle.triggered);

        let ending = step_mind_sense(&mut effect, 2, 5.0);
        assert!(ending.active && ending.triggered);

        let quiet = step_mind_sense(&mut effect, 2, 0.0);
        assert!(quiet.active);
        assert!(!quiet.triggered);
    }
}

/// The mind-sight overlay's phase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindPhase {
    Idle,
    Shown,
    FadingOut,
    FadingIn,
}

/// The mind-sight overlay's state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindOverlay {
    pub phase: MindPhase,
    pub alpha: f32,
    pub stamp_ms: u32,
    pub end_ms: u32,
}

/// Advances the overlay: shown forces alpha 1 and starts fading out past
/// its end; the 2 s fades move alpha by the elapsed milliseconds and the
/// fade-out ends below 0.001. Returns whether it still draws.
pub fn advance_mind_overlay(overlay: &mut MindOverlay, now_ms: u32) -> bool {
    match overlay.phase {
        MindPhase::Idle => return false,
        MindPhase::Shown => {
            overlay.alpha = 1.0;
            if overlay.end_ms < now_ms {
                overlay.phase = MindPhase::FadingOut;
            }
        }
        MindPhase::FadingOut => {
            let elapsed = now_ms.wrapping_sub(overlay.stamp_ms) as f64 / 2000.0;
            let alpha = overlay.alpha - elapsed as f32;
            if f64::from(alpha) >= 0.001 {
                overlay.alpha = alpha;
            } else {
                overlay.phase = MindPhase::Idle;
                overlay.alpha = 0.0;
                overlay.stamp_ms = now_ms;
                return false;
            }
        }
        MindPhase::FadingIn => {
            let elapsed = now_ms.wrapping_sub(overlay.stamp_ms) as f64 / 2000.0;
            let mut alpha = overlay.alpha + elapsed as f32;
            if alpha > 1.0 {
                alpha = 1.0;
                overlay.phase = MindPhase::Shown;
            }
            overlay.alpha = alpha;
        }
    }
    overlay.stamp_ms = now_ms;
    true
}

/// The pick radius of a mind-sight node: `n · scale · 0.5 · 40 / width`.
pub fn mind_pick_radius(n: f32, scale: f32, width: f32) -> f32 {
    n * scale * 0.5 * 40.0 / width
}

/// Picks the nearest candidate whose normalized distance is at most 1.
/// Each candidate carries its centre x, y and pick radius.
pub fn pick_mind_target(candidates: &[(f32, f32, f32)], x: f32, y: f32) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (index, &(cx, cy, radius)) in candidates.iter().enumerate() {
        if radius <= 0.0 {
            continue;
        }
        let distance = ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt() / radius;
        if distance <= 1.0 && best.is_none_or(|(kept, _)| distance < kept) {
            best = Some((distance, index));
        }
    }
    best.map(|(_, index)| index)
}

#[cfg(test)]
mod sight_tests {
    use super::*;

    #[test]
    fn the_overlay_fades_in_and_out_over_two_seconds() {
        let mut shown = MindOverlay {
            phase: MindPhase::Shown,
            alpha: 0.0,
            stamp_ms: 0,
            end_ms: 1000,
        };
        assert!(advance_mind_overlay(&mut shown, 500));
        assert_eq!(shown.alpha, 1.0);
        assert_eq!(shown.phase, MindPhase::Shown);
        assert!(advance_mind_overlay(&mut shown, 1001));
        assert_eq!(shown.phase, MindPhase::FadingOut);

        let mut fading = MindOverlay {
            phase: MindPhase::FadingOut,
            alpha: 1.0,
            stamp_ms: 0,
            end_ms: 0,
        };
        assert!(advance_mind_overlay(&mut fading, 1000));
        assert!((fading.alpha - 0.5).abs() < 1e-6);
        assert!(!advance_mind_overlay(&mut fading, 3000));
        assert_eq!(fading.phase, MindPhase::Idle);

        let mut rising = MindOverlay {
            phase: MindPhase::FadingIn,
            alpha: 0.0,
            stamp_ms: 0,
            end_ms: 0,
        };
        assert!(advance_mind_overlay(&mut rising, 1000));
        assert!((rising.alpha - 0.5).abs() < 1e-6);
        assert!(advance_mind_overlay(&mut rising, 3000));
        assert_eq!(rising.alpha, 1.0);
        assert_eq!(rising.phase, MindPhase::Shown);
    }

    #[test]
    fn the_pick_radius_scales_with_n_and_the_view() {
        assert_eq!(mind_pick_radius(1.0, 1.0, 40.0), 0.5);
        assert_eq!(mind_pick_radius(2.0, 1.0, 40.0), 1.0);
    }

    #[test]
    fn the_nearest_inside_candidate_wins() {
        let candidates = [(0.0, 0.0, 10.0), (6.0, 0.0, 10.0), (100.0, 0.0, 10.0)];
        assert_eq!(pick_mind_target(&candidates, 5.0, 0.0), Some(1));
        assert_eq!(pick_mind_target(&candidates, 50.0, 0.0), None);
        assert_eq!(pick_mind_target(&[(0.0, 0.0, 0.0)], 0.0, 0.0), None);
    }
}

/// The body values the sight's node weights read, at `+0x78`, `+0x974`,
/// `+0x978`, `+0x7b4`, `+0x7c` and `+0x74`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindBodyValues {
    pub v78: f32,
    pub v974: f32,
    pub v978: f32,
    pub v7b4: f32,
    pub v7c: f32,
    pub v74: f32,
}

/// The sight's three node weights: the threshold, the maximum and the
/// ratio.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindWeights {
    pub threshold: f32,
    pub maximum: f32,
    pub ratio: f32,
}

/// The sight's node weights, before the fade: `A = 0.2·v78`,
/// `B = 0.2·(v974 + v978 − v7b4)`. A marked body or `A > 1` forces all
/// three to 1. Otherwise `A` is floored at 0, `B` clamped to 0..1, the
/// maximum is their larger, the ratio `A/(A+B)`, and the threshold the
/// largest of `maximum + 0.2`, `0.6·v7c` and `0.015·v74`, capped at 1.
pub fn mind_track_weights(body: &MindBodyValues, marked: bool) -> MindWeights {
    let a = 0.2 * body.v78;
    let b = 0.2 * (body.v974 + body.v978 - body.v7b4);
    if marked || a > 1.0 {
        return MindWeights {
            threshold: 1.0,
            maximum: 1.0,
            ratio: 1.0,
        };
    }
    let a = a.max(0.0);
    let b = b.clamp(0.0, 1.0);
    let maximum = a.max(b);
    let ratio = if a + b > 0.0 { a / (a + b) } else { 0.0 };
    let threshold = (maximum + 0.2)
        .max(0.6 * body.v7c)
        .max(0.015 * body.v74)
        .min(1.0);
    MindWeights {
        threshold,
        maximum,
        ratio,
    }
}

/// The three colours a sight entry blends, at `+0x08`, `+0x0c` and `+0x10`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindEntryColors {
    pub first: f32,
    pub second: f32,
    pub third: f32,
}

/// Blends the entry's three values toward the weights by `2^(−dt/1000)`;
/// a step of a second or more uses one half.
pub fn blend_mind_weights(
    entry: &mut MindEntryColors,
    target: MindWeights,
    dt_ms: u32,
) -> MindEntryColors {
    let k = if dt_ms >= 1000 {
        0.5
    } else {
        2.0f32.powf(-(dt_ms as f32) / 1000.0)
    };
    entry.first += (target.threshold - entry.first) * k;
    entry.second += (target.maximum - entry.second) * k;
    entry.third += (target.ratio - entry.third) * k;
    *entry
}

#[cfg(test)]
mod weight_tests {
    use super::*;

    fn body(v78: f32, v974: f32, v978: f32, v7b4: f32, v7c: f32, v74: f32) -> MindBodyValues {
        MindBodyValues {
            v78,
            v974,
            v978,
            v7b4,
            v7c,
            v74,
        }
    }

    #[test]
    fn a_marked_body_forces_full_weights() {
        let weights = mind_track_weights(&body(0.0, 0.0, 0.0, 0.0, 0.0, 0.0), true);
        assert_eq!(weights.threshold, 1.0);
        assert_eq!(weights.maximum, 1.0);
        assert_eq!(weights.ratio, 1.0);
    }

    #[test]
    fn the_weights_follow_the_body_values() {
        let weights = mind_track_weights(&body(0.5, 0.5, 0.5, 0.0, 0.0, 0.0), false);
        assert!((weights.maximum - 0.2).abs() < 1e-6);
        assert!((weights.ratio - 1.0 / 3.0).abs() < 1e-6);
        assert!((weights.threshold - 0.4).abs() < 1e-6);

        let strong = mind_track_weights(&body(10.0, 0.0, 0.0, 0.0, 0.0, 0.0), false);
        assert_eq!(strong.maximum, 1.0);
        assert_eq!(strong.ratio, 1.0);
    }

    #[test]
    fn the_entry_blends_by_a_half_over_a_second() {
        let mut entry = MindEntryColors {
            first: 0.0,
            second: 0.0,
            third: 0.0,
        };
        blend_mind_weights(
            &mut entry,
            MindWeights {
                threshold: 1.0,
                maximum: 0.5,
                ratio: 0.25,
            },
            1000,
        );
        assert_eq!(entry.first, 0.5);
        assert_eq!(entry.second, 0.25);
        assert_eq!(entry.third, 0.125);
    }
}

/// The sight quad's basis in view space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightBasis {
    pub u: [f32; 3],
    pub v: [f32; 3],
    pub eye: [f32; 3],
    pub dir: [f32; 3],
    pub blend: f32,
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length == 0.0 {
        return [0.0; 3];
    }
    [v[0] / length, v[1] / length, v[2] / length]
}

fn mat3_rows(rows: [[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        rows[0][0] * v[0] + rows[1][0] * v[1] + rows[2][0] * v[2],
        rows[0][1] * v[0] + rows[1][1] * v[1] + rows[2][1] * v[2],
        rows[0][2] * v[0] + rows[1][2] * v[1] + rows[2][2] * v[2],
    ]
}

/// The sight quad's basis: the eye anchors at the origin plus
/// `(anchor_a + 15)·m1 + anchor_b·m2`, the direction runs to the camera,
/// and the two axes blend the cross products by `1 − |view·dir.x|` before
/// scaling by 0.025.
pub fn sight_billboard(
    rows: [[f32; 3]; 3],
    origin: [f32; 3],
    anchor_a: f32,
    anchor_b: f32,
    camera: [f32; 3],
    view: [[f32; 3]; 3],
) -> SightBasis {
    let eye = [
        origin[0] + (anchor_a + 15.0) * rows[1][0] + anchor_b * rows[2][0],
        origin[1] + (anchor_a + 15.0) * rows[1][1] + anchor_b * rows[2][1],
        origin[2] + (anchor_a + 15.0) * rows[1][2] + anchor_b * rows[2][2],
    ];
    let dir = normalize([eye[0] - camera[0], eye[1] - camera[1], eye[2] - camera[2]]);
    let d = mat3_rows(view, dir);
    let blend = 1.0 - d[0].abs();
    let a = normalize(cross(rows[2], dir));
    let b = normalize(cross(a, dir));
    let c = normalize(cross(rows[0], dir));
    let e = normalize(cross(c, dir));
    let mix = |from: [f32; 3], to: [f32; 3]| {
        normalize([
            from[0] + (to[0] - from[0]) * blend,
            from[1] + (to[1] - from[1]) * blend,
            from[2] + (to[2] - from[2]) * blend,
        ])
    };
    let u = mix(a, e);
    let v = mix(b, c);
    SightBasis {
        u: [
            mat3_rows(view, u)[0] * 0.025,
            mat3_rows(view, u)[1] * 0.025,
            mat3_rows(view, u)[2] * 0.025,
        ],
        v: [
            mat3_rows(view, v)[0] * 0.025,
            mat3_rows(view, v)[1] * 0.025,
            mat3_rows(view, v)[2] * 0.025,
        ],
        eye: [-d[0], -d[1], -d[2]],
        dir,
        blend,
    }
}

#[cfg(test)]
mod basis_tests {
    use super::*;

    #[test]
    fn the_quad_axes_are_orthogonal_and_scaled() {
        let identity = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
        let basis = sight_billboard(identity, [0.0; 3], 0.0, 0.0, [0.0, 0.0, -10.0], identity);
        let u_len = (basis.u[0].powi(2) + basis.u[1].powi(2) + basis.u[2].powi(2)).sqrt();
        let v_len = (basis.v[0].powi(2) + basis.v[1].powi(2) + basis.v[2].powi(2)).sqrt();
        assert!((u_len - 0.025).abs() < 1e-6);
        assert!((v_len - 0.025).abs() < 1e-6);
        let dot = basis.u[0] * basis.v[0] + basis.u[1] * basis.v[1] + basis.u[2] * basis.v[2];
        assert!(dot.abs() < 1e-6);
        assert!(basis.blend >= 0.0 && basis.blend <= 1.0);
        assert!((basis.eye[2] + 10.0 / 325.0f32.sqrt()).abs() < 1e-6);
    }
}

/// One sight entry's style values: the fade, the kind and the three
/// blended values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindEntryStyle {
    pub fade: f32,
    pub kind: i32,
    pub first: f32,
    pub second: f32,
    pub third: f32,
}

/// The entry's shader colours and alphas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindColors {
    pub colour0: [f32; 3],
    pub colour1: [f32; 3],
    pub colour2: [f32; 3],
    pub alpha: f32,
    pub alpha2: f32,
    /// The quad's own alpha: the overlay's, less the entry's fade.
    pub quad_alpha: f32,
}

/// The per-entry colours: a fixed {0.1, 0.4, 0.6} and {0.8, 0.6, 0},
/// a third blended from {0.4, 0, 0} to {0.31, 0.78, 0} by the third value,
/// the two 0.1 + 0.6·(1 − value) alphas and the overlay alpha less the
/// entry fade (when positive).
pub fn mind_entry_colors(style: &MindEntryStyle, overlay_alpha: f32) -> MindColors {
    if style.kind == 1 {
        return MindColors {
            colour0: [0.3, 0.3, 0.3],
            colour1: [0.5, 0.5, 0.6],
            colour2: [0.3, 0.3, 0.3],
            alpha: 1.0,
            alpha2: 0.5,
            quad_alpha: if style.fade > 0.0 {
                overlay_alpha - style.fade
            } else {
                overlay_alpha
            },
        };
    }
    let third = style.third.clamp(0.0, 1.0);
    MindColors {
        colour0: [0.1, 0.4, 0.6],
        colour1: [0.8, 0.6, 0.0],
        colour2: [0.4 + (0.31 - 0.4) * third, 0.0 + (0.78 - 0.0) * third, 0.0],
        alpha: (0.1 + 0.6 * (1.0 - f64::from(style.first))) as f32,
        alpha2: (0.1 + 0.6 * (1.0 - f64::from(style.second))) as f32,
        quad_alpha: if style.fade > 0.0 {
            overlay_alpha - style.fade
        } else {
            overlay_alpha
        },
    }
}

#[cfg(test)]
mod color_tests {
    use super::*;

    fn style(first: f32, second: f32, third: f32, fade: f32) -> MindEntryStyle {
        MindEntryStyle {
            fade,
            kind: 0,
            first,
            second,
            third,
        }
    }

    #[test]
    fn the_entry_colours_follow_the_blended_values() {
        let colors = mind_entry_colors(&style(0.0, 0.0, 0.0, 0.0), 1.0);
        assert_eq!(colors.colour0, [0.1, 0.4, 0.6]);
        assert_eq!(colors.colour1, [0.8, 0.6, 0.0]);
        assert_eq!(colors.colour2, [0.4, 0.0, 0.0]);
        assert!((colors.alpha - 0.7).abs() < 1e-6);
        assert!((colors.alpha2 - 0.7).abs() < 1e-6);
        assert_eq!(colors.quad_alpha, 1.0);
    }

    #[test]
    fn full_values_darken_the_alphas() {
        let colors = mind_entry_colors(&style(1.0, 1.0, 1.0, 0.2), 1.0);
        assert!((colors.alpha - 0.1).abs() < 1e-6);
        assert!((colors.alpha2 - 0.1).abs() < 1e-6);
        assert_eq!(colors.colour2, [0.31, 0.78, 0.0]);
        assert!((colors.quad_alpha - 0.8).abs() < 1e-6);
    }

    #[test]
    fn a_kind_one_entry_uses_the_pale_palette() {
        let mut pale = style(0.0, 0.0, 0.0, 0.0);
        pale.kind = 1;
        let colors = mind_entry_colors(&pale, 1.0);
        assert_eq!(colors.colour0, [0.3, 0.3, 0.3]);
        assert_eq!(colors.colour1, [0.5, 0.5, 0.6]);
    }
}

/// Whether a sight tick is due: at least four milliseconds since the last
/// one.
pub fn mind_track_due(now_ms: u32, stamp_ms: u32) -> bool {
    now_ms.wrapping_sub(stamp_ms) >= 4
}

/// The dwell window of a sight node: 3000 ms when its dwell value is above
/// 1, 5000 ms when its body is marked, and none otherwise.
pub fn mind_dwell_window(dwell: i32, marked: bool) -> Option<u32> {
    if dwell > 1 {
        Some(3000)
    } else if marked {
        Some(5000)
    } else {
        None
    }
}

/// The dwell fade and whether the node is lost: the elapsed time over the
/// window, lost once the clock passes `seen + window`.
pub fn mind_dwell_fade(now_ms: u32, seen_ms: u32, window_ms: u32) -> (f32, bool) {
    if window_ms == 0 {
        return (1.0, true);
    }
    let elapsed = now_ms.wrapping_sub(seen_ms) as f32 / window_ms as f32;
    (elapsed, now_ms > seen_ms.wrapping_add(window_ms))
}

#[cfg(test)]
mod dwell_tests {
    use super::*;

    #[test]
    fn a_tick_is_due_after_four_milliseconds() {
        assert!(!mind_track_due(0, 0));
        assert!(!mind_track_due(3, 0));
        assert!(mind_track_due(4, 0));
        assert!(!mind_track_due(10, 8));
    }

    #[test]
    fn the_window_follows_the_dwell_value_and_the_mark() {
        assert_eq!(mind_dwell_window(2, false), Some(3000));
        assert_eq!(mind_dwell_window(1, true), Some(5000));
        assert_eq!(mind_dwell_window(0, false), None);
    }

    #[test]
    fn the_fade_runs_over_the_window_and_is_lost_after_it() {
        assert_eq!(mind_dwell_fade(1500, 0, 3000), (0.5, false));
        assert_eq!(mind_dwell_fade(3001, 0, 3000), (1.0003333, true));
        assert_eq!(mind_dwell_fade(0, 0, 0), (1.0, true));
    }
}

/// The loyalty power's state: whether it is applying and the target id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltyState {
    pub state: i32,
    pub target: Option<u32>,
}

/// One loyalty step's outcome: the removal request (2) or 0, whether the
/// effect was applied, whether it was removed, and the effect marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltyStep {
    pub state: i32,
    pub result: i32,
    pub applied: bool,
    pub removed: bool,
    pub marker: Option<u32>,
}

/// The effect marker the loyalty apply records (`0x10010a`).
pub const LOYALTY_MARKER: u32 = 0x10_010a;

/// Steps the loyalty power: event 2 on an applying state asks for removal
/// (2), otherwise it starts removal (state 2). While applying, a missing or
/// marked target moves to state 2; a valid one applies with the marker and
/// returns to state 0. In removal it removes the target and asks for 2.
pub fn step_loyalty(
    loyalty: &mut LoyaltyState,
    event: i32,
    target_valid: bool,
    target_marked: bool,
) -> LoyaltyStep {
    if event == 2 {
        if loyalty.state == 1 {
            return LoyaltyStep {
                state: loyalty.state,
                result: 2,
                applied: false,
                removed: false,
                marker: None,
            };
        }
        loyalty.state = 2;
    }
    if loyalty.state == 1 {
        if !target_valid || target_marked {
            loyalty.state = 2;
            return LoyaltyStep {
                state: loyalty.state,
                result: 0,
                applied: false,
                removed: false,
                marker: None,
            };
        }
        loyalty.state = 0;
        return LoyaltyStep {
            state: loyalty.state,
            result: 0,
            applied: true,
            removed: false,
            marker: Some(LOYALTY_MARKER),
        };
    }
    if loyalty.state == 2 {
        loyalty.state = 2;
        return LoyaltyStep {
            state: loyalty.state,
            result: 2,
            applied: false,
            removed: true,
            marker: None,
        };
    }
    LoyaltyStep {
        state: loyalty.state,
        result: 0,
        applied: false,
        removed: false,
        marker: None,
    }
}

#[cfg(test)]
mod loyalty_tests {
    use super::*;

    #[test]
    fn an_applying_loyalty_with_a_valid_target_applies() {
        let mut loyalty = LoyaltyState {
            state: 1,
            target: Some(7),
        };
        let step = step_loyalty(&mut loyalty, 0, true, false);
        assert!(step.applied);
        assert!(!step.removed);
        assert_eq!(step.marker, Some(LOYALTY_MARKER));
        assert_eq!(step.state, 0);
    }

    #[test]
    fn a_marked_or_missing_target_starts_removal() {
        let mut loyalty = LoyaltyState {
            state: 1,
            target: Some(7),
        };
        let step = step_loyalty(&mut loyalty, 0, false, false);
        assert!(!step.applied);
        assert_eq!(step.state, 2);

        let mut marked = LoyaltyState {
            state: 1,
            target: Some(7),
        };
        assert_eq!(step_loyalty(&mut marked, 0, true, true).state, 2);
    }

    #[test]
    fn event_two_asks_for_removal_then_removes() {
        let mut loyalty = LoyaltyState {
            state: 1,
            target: Some(7),
        };
        let asked = step_loyalty(&mut loyalty, 2, true, false);
        assert_eq!(asked.result, 2);
        assert_eq!(asked.state, 1);

        let mut removing = LoyaltyState {
            state: 2,
            target: Some(7),
        };
        let removed = step_loyalty(&mut removing, 0, true, false);
        assert!(removed.removed);
        assert_eq!(removed.result, 2);
    }
}

/// The command-dead power's state: whether it is applying and the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDeadState {
    pub state: i32,
    pub target: Option<u32>,
}

/// One command-dead step's outcome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandDeadStep {
    pub state: i32,
    pub result: i32,
    pub applied: bool,
    pub removed: bool,
    pub marker: Option<u32>,
}

/// The effect marker the command-dead apply records (`0x10010d`).
pub const COMMAND_DEAD_MARKER: u32 = 0x10_010d;

/// Steps the command-dead power: event 2 on an applying state asks for
/// removal (2), otherwise it starts removal. While applying, a missing
/// target moves to state 2; a valid one applies with the marker, sets the
/// target's raised flag and returns to state 0. In removal it removes the
/// marked effect, clears the target's flag and returns 2.
pub fn step_command_dead(
    dead: &mut CommandDeadState,
    event: i32,
    target_valid: bool,
) -> CommandDeadStep {
    if event == 2 {
        if dead.state == 1 {
            return CommandDeadStep {
                state: dead.state,
                result: 2,
                applied: false,
                removed: false,
                marker: None,
            };
        }
        dead.state = 2;
    }
    if dead.state == 1 {
        if !target_valid {
            dead.state = 2;
            return CommandDeadStep {
                state: dead.state,
                result: 0,
                applied: false,
                removed: false,
                marker: None,
            };
        }
        dead.state = 0;
        return CommandDeadStep {
            state: dead.state,
            result: 0,
            applied: true,
            removed: false,
            marker: Some(COMMAND_DEAD_MARKER),
        };
    }
    if dead.state == 2 {
        return CommandDeadStep {
            state: dead.state,
            result: 2,
            applied: false,
            removed: true,
            marker: None,
        };
    }
    CommandDeadStep {
        state: dead.state,
        result: 0,
        applied: false,
        removed: false,
        marker: None,
    }
}

#[cfg(test)]
mod command_dead_tests {
    use super::*;

    #[test]
    fn a_valid_target_applies_the_marker() {
        let mut dead = CommandDeadState {
            state: 1,
            target: Some(9),
        };
        let step = step_command_dead(&mut dead, 0, true);
        assert!(step.applied);
        assert_eq!(step.marker, Some(COMMAND_DEAD_MARKER));
        assert_eq!(step.state, 0);
    }

    #[test]
    fn a_missing_target_moves_to_removal() {
        let mut dead = CommandDeadState {
            state: 1,
            target: Some(9),
        };
        let step = step_command_dead(&mut dead, 0, false);
        assert!(!step.applied);
        assert_eq!(step.state, 2);
        assert_eq!(step.result, 0);
    }

    #[test]
    fn event_two_and_removal_ask_for_two() {
        let mut dead = CommandDeadState {
            state: 1,
            target: Some(9),
        };
        assert_eq!(step_command_dead(&mut dead, 2, true).result, 2);

        let mut removing = CommandDeadState {
            state: 2,
            target: Some(9),
        };
        let step = step_command_dead(&mut removing, 0, true);
        assert!(step.removed);
        assert_eq!(step.result, 2);
    }
}

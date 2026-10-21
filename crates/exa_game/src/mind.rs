//! The Mind power's spirit form, as `TPwrSpiritForm` runs it: a value
//! charged to 1, held, or faded to 0, published as a strength, a screen
//! fade and a body vector fade.
//!
//! - **Charge** (state 3): the value rises by the elapsed milliseconds over
//!   1.2 s. It stops at 1 and becomes held (state 1).
//! - **Hold** (state 1): the value is 1.
//! - **Fade** (state 2): the value falls by the elapsed milliseconds over
//!   1.2 s. Below 0.001 it drops to 0 and the form ends (state 0).
//! - **Publish**: in the fade the strength is `1000·value`. Otherwise the
//!   smooth value is `1 − (1 − value)²` (the screen's fade and the body
//!   vector's scale is `1 − smooth`), the strength's own smooth is
//!   `1 − (1 − smooth)²`, and the strength is
//!   `(1 − smooth2)·rate + 1000·smooth2`.

/// Milliseconds for the spirit form to charge or fade by one.
pub const SPIRIT_SPAN_MS: f64 = 1_200.0;
/// The strength at a full value.
pub const SPIRIT_STRENGTH: f32 = 1000.0;
/// The value below which a fade ends.
pub const SPIRIT_FLOOR: f64 = 0.001;

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
    /// The screen effect's fade, absent while fading.
    pub screen: Option<f32>,
    /// The smooth fade the strength follows, absent while fading.
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
        _ => active = false,
    }
    // A fade that ends, or a form that has ended, publishes nothing and
    // keeps its stamp.
    if !active {
        return SpiritStep {
            state: form.state,
            value: form.value,
            strength: 0.0,
            screen: None,
            smooth: None,
            vector_scale: None,
            active: false,
        };
    }
    form.stamp = clock;
    if form.state == 2 {
        return SpiritStep {
            state: form.state,
            value: form.value,
            strength: SPIRIT_STRENGTH * form.value,
            screen: None,
            smooth: None,
            vector_scale: None,
            active: true,
        };
    }
    let screen = 1.0 - (1.0 - form.value) * (1.0 - form.value);
    let smooth = 1.0 - (1.0 - screen) * (1.0 - screen);
    SpiritStep {
        state: form.state,
        value: form.value,
        strength: (1.0 - smooth) * rate + SPIRIT_STRENGTH * smooth,
        screen: Some(screen),
        smooth: Some(smooth),
        vector_scale: Some(1.0 - screen),
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
        assert_eq!(step.screen, Some(1.0));
        assert_eq!(step.smooth, Some(1.0));
        assert_eq!(step.vector_scale, Some(0.0));
        assert!(step.active);
    }

    #[test]
    fn a_fade_falls_over_one_point_two_seconds_and_ends() {
        let mut fading = form(2, 1.0, 0);
        let half = step_spirit_form(&mut fading, 600, 100.0);
        assert_eq!(half.state, 2);
        assert!((half.value - 0.5).abs() < 1e-6);
        assert_eq!(half.strength, 500.0);
        assert_eq!(half.smooth, None);

        let mut gone = form(2, 1.0, 0);
        let end = step_spirit_form(&mut gone, 1_200, 100.0);
        assert!(!end.active);
        assert_eq!(end.state, 0);
        assert_eq!(end.value, 0.0);
        // The ended form keeps the stamp of its last running step and
        // publishes nothing after.
        assert_eq!(gone.stamp, 0);
        assert!(!step_spirit_form(&mut gone, 5_000, 100.0).active);
        assert_eq!(gone.stamp, 0);
    }

    #[test]
    fn a_charge_rises_and_clamps_at_one() {
        let mut charging = form(3, 0.0, 0);
        let half = step_spirit_form(&mut charging, 600, 100.0);
        assert_eq!(half.state, 3);
        assert!((half.value - 0.5).abs() < 1e-6);
        assert_eq!(half.screen, Some(0.75));
        assert_eq!(half.smooth, Some(0.9375));
        assert!((half.vector_scale.unwrap() - 0.25).abs() < 1e-6);
        assert!((half.strength - 943.75).abs() < 1e-3);

        let full = step_spirit_form(&mut charging, 1_500, 100.0);
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

/// Picks the nearest candidate whose normalized distance is below 1. Each
/// candidate carries its centre x, y and pick radius; a radius of zero or
/// less never picks.
pub fn pick_mind_target(candidates: &[(f32, f32, f32)], x: f32, y: f32) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    for (index, &(cx, cy, radius)) in candidates.iter().enumerate() {
        // A NaN radius (or a negative one) never picks; the original's
        // ordered test falls through.
        if !radius.is_finite() || radius <= 0.0 {
            continue;
        }
        let distance = ((x - cx) * (x - cx) + (y - cy) * (y - cy)).sqrt() / radius;
        if distance < 1.0 && best.is_none_or(|(kept, _)| distance < kept) {
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
    /// The first colour's alpha, also the quad's own alpha before the fade.
    pub alpha: f32,
    /// The first blend's threshold, the second colour's alpha.
    pub alpha2: f32,
    /// The second blend's threshold, the third colour's alpha.
    pub alpha3: f32,
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
            colour1: [0.3, 0.3, 0.3],
            colour2: [0.5, 0.5, 0.6],
            // The first colour's alpha is the overlay's, less the entry's
            // fade (the quad's alpha below).
            alpha: overlay_alpha,
            alpha2: 1.0,
            alpha3: 0.5,
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
        // A guess: the original never writes the third colour's alpha on this
        // path, so its value is stack garbage. The kind-1 palette's 0.5 is
        // the only value the original ever leaves there deliberately.
        alpha3: 0.5,
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
        assert_eq!(colors.colour1, [0.3, 0.3, 0.3]);
        assert_eq!(colors.colour2, [0.5, 0.5, 0.6]);
        assert_eq!((colors.alpha2, colors.alpha3), (1.0, 0.5));
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

/// The record kinds the mind powers care about: a living character
/// (record types 1 to 3), an undead one (4 and 5), or neither.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindRecord {
    Living,
    Undead,
    Other,
}

/// Classifies a character record's type word.
pub fn mind_record(record_type: u32) -> MindRecord {
    match record_type {
        1..=3 => MindRecord::Living,
        4..=5 => MindRecord::Undead,
        _ => MindRecord::Other,
    }
}

/// Which cast a mind row runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MindCast {
    Sense,
    SensePersistent,
    Mask,
    MaskPersistent,
    SpiritForm,
    Calm,
    Enrage,
    Confusion,
    LoyaltyEnd,
    LoyaltyStart,
    Suspicion,
    Possession,
    CommandDead,
    Fear,
    Embolden,
    RaiseDead,
}

/// One row of the mind power table: its id, focus cost and cast.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindPowerRow {
    pub id: u32,
    pub cost: f32,
    pub cast: MindCast,
}

/// The mind power table, in the original's order. A sustained row's id has
/// bit 16 set and a persistent row's bit 17.
pub const MIND_POWERS: [MindPowerRow; 16] = [
    MindPowerRow {
        id: 0x101,
        cost: 0.13,
        cast: MindCast::Sense,
    },
    MindPowerRow {
        id: 0x20101,
        cost: 0.27,
        cast: MindCast::SensePersistent,
    },
    MindPowerRow {
        id: 0x102,
        cost: 0.12,
        cast: MindCast::Mask,
    },
    MindPowerRow {
        id: 0x20102,
        cost: 0.24,
        cast: MindCast::MaskPersistent,
    },
    MindPowerRow {
        id: 0x10106,
        cost: 0.025,
        cast: MindCast::SpiritForm,
    },
    MindPowerRow {
        id: 0x103,
        cost: 0.14,
        cast: MindCast::Calm,
    },
    MindPowerRow {
        id: 0x104,
        cost: 0.16,
        cast: MindCast::Enrage,
    },
    MindPowerRow {
        id: 0x105,
        cost: 0.17,
        cast: MindCast::Confusion,
    },
    MindPowerRow {
        id: 0x109,
        cost: 0.19,
        cast: MindCast::LoyaltyEnd,
    },
    MindPowerRow {
        id: 0x2010A,
        cost: 0.29,
        cast: MindCast::LoyaltyStart,
    },
    MindPowerRow {
        id: 0x10B,
        cost: 0.19,
        cast: MindCast::Suspicion,
    },
    MindPowerRow {
        id: 0x2010C,
        cost: 0.35,
        cast: MindCast::Possession,
    },
    MindPowerRow {
        id: 0x2010D,
        cost: 0.25,
        cast: MindCast::CommandDead,
    },
    MindPowerRow {
        id: 0x107,
        cost: 0.18,
        cast: MindCast::Fear,
    },
    MindPowerRow {
        id: 0x108,
        cost: 0.18,
        cast: MindCast::Embolden,
    },
    MindPowerRow {
        id: 0x10F,
        cost: 0.30,
        cast: MindCast::RaiseDead,
    },
];

/// The row whose id matches under the table's mask, if any.
pub fn mind_row(asked: u32) -> Option<&'static MindPowerRow> {
    let id = asked & 0x0e0f_ffff;
    MIND_POWERS.iter().find(|row| row.id == id)
}

/// The return codes a mind cast shares with the power tree: refused,
/// spent without an effect, applied, or ended.
pub const MIND_REFUSED: i32 = -1;
pub const MIND_FAILED: i32 = 0;
pub const MIND_APPLIED: i32 = 1;
pub const MIND_ENDED: i32 = 2;

/// A sound a mind cast plays and its volume.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindSound {
    pub name: &'static str,
    pub volume: f32,
}

pub const SOUND_A01: MindSound = MindSound {
    name: "ThmMnd_A01",
    volume: 0.3,
};
pub const SOUND_B01: MindSound = MindSound {
    name: "ThmMnd_B01",
    volume: 0.3,
};
pub const SOUND_B02: MindSound = MindSound {
    name: "ThmMnd_B02",
    volume: 0.2,
};
pub const SOUND_C01: MindSound = MindSound {
    name: "ThmMnd_C01",
    volume: 0.2,
};
pub const SOUND_C02: MindSound = MindSound {
    name: "ThmMnd_C02",
    volume: 0.1,
};
pub const SOUND_C03: MindSound = MindSound {
    name: "ThmMnd_C03",
    volume: 0.2,
};
pub const SOUND_CTRL: MindSound = MindSound {
    name: "ThmMnd_Ctrl_01",
    volume: 0.15,
};
pub const SOUND_E01: MindSound = MindSound {
    name: "ThmMnd_E01",
    volume: 0.15,
};
pub const SOUND_E02: MindSound = MindSound {
    name: "ThmMnd_E02",
    volume: 0.1,
};
pub const SOUND_F01: MindSound = MindSound {
    name: "ThmMnd_F01",
    volume: 0.2,
};
pub const SOUND_R01: MindSound = MindSound {
    name: "ThmMnd_R01",
    volume: 0.1,
};
pub const SOUND_R01_LOUD: MindSound = MindSound {
    name: "ThmMnd_R01",
    volume: 0.2,
};
pub const SOUND_R02: MindSound = MindSound {
    name: "ThmMnd_R02",
    volume: 0.05,
};
pub const SOUND_S01: MindSound = MindSound {
    name: "ThmMnd_S01",
    volume: 0.15,
};
pub const SOUND_T01: MindSound = MindSound {
    name: "ThmMnd_T01",
    volume: 0.1,
};
pub const SPIRIT_MUSIC: &str = "Msc_SprtRealm";

/// The effect marker of the loyalty bind (a start).
pub const LOYALTY_BIND: u32 = 0x10_010a;
/// The effect marker of the loyalty release.
pub const LOYALTY_RELEASE: u32 = 0x10_0109;
/// The second loyalty-family marker a body can carry.
pub const LOYALTY_OTHER: u32 = 0x10_010c;
/// The effect marker of the command over the dead.
pub const COMMAND_BIND: u32 = 0x10_010d;

/// The mind powers' body flag bits.
pub const MIND_MASKED: u32 = 0x1_0000;
pub const MIND_ENRAGED: u32 = 0x8_0000;
pub const MIND_CONFUSED: u32 = 0x4_0000;
pub const MIND_SUSPICIOUS: u32 = 0x10_0000;
pub const MIND_FEARFUL: u32 = 0x8_0000;
pub const SPIRIT_FORM: u32 = 0x100_0000;
/// The status bit a raised body carries.
pub const UNDEAD_RAISED: u32 = 0x10;
/// The technique bit that resists possession.
pub const POSSESSION_RESIST: u32 = 0x1_0000;
/// The repeat-cost technique.
pub const REPEAT_TECHNIQUE: u32 = 0x80_0000;
/// The sustain-cost technique.
pub const SUSTAIN_TECHNIQUE: u32 = 0x10_0000;
/// The reserve technique that refunds a quarter of a restored power.
pub const MIND_RESERVE_TECHNIQUE: u32 = 0x40_0000;
/// The body flags that mark a corpse (dying or dead).
pub const CORPSE_FLAGS: u32 = 0xa00;
/// The character flag that refuses a body to the dead powers.
pub const NO_RAISE: u32 = 0x40;
/// The mind-sight windows: a marked body 5000 ms, another tracked body
/// 3000 ms.
pub const SIGHT_MARKED_WINDOW_MS: u32 = 5000;
pub const SIGHT_WINDOW_MS: u32 = 3000;
/// The sense's longest duration, and the sense overlay's fade.
pub const SENSE_SPAN_MS: i32 = 4000;
/// A mind glow's life.
pub const MIND_GLOW_MS: u32 = 3000;

/// One recorded concern of a body: its size and its two decay terms.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindConcern {
    pub value: f32,
    /// The first decay term, moved into the value on a calm.
    pub decay_a: f32,
    /// The second decay term.
    pub decay_b: f32,
}

/// A relation entry towards another body, as the loyalty powers write it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindRelation {
    /// The other body's id.
    pub body: u32,
    /// The relation's value.
    pub value: f32,
    /// The relation's first weight.
    pub weight_a: f32,
    /// The relation's second weight.
    pub weight_b: f32,
}

/// One effect object a body carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindSlot {
    pub id: u32,
    /// 1 applying, 2 removing, 0 done.
    pub state: i32,
    /// Who owns the effect.
    pub owner: Option<u32>,
    /// Whom the effect is about.
    pub target: Option<u32>,
    /// The entry's kind for the sight.
    pub kind: i32,
    /// The effect's first level.
    pub level: f32,
    /// The effect's second level.
    pub level2: f32,
    /// The sense's end time, in the game clock.
    pub end_ms: i32,
    /// The loyalty relation level.
    pub relation: f32,
}

/// A body's mind-side state: focus, the values the powers move, its
/// relations and the effect objects it carries.
#[derive(Debug, Clone, PartialEq)]
pub struct MindState {
    pub id: u32,
    pub focus: f32,
    pub record_type: u32,
    /// The corpse bits (`CORPSE_FLAGS` when dying or dead).
    pub body_flags: u32,
    /// The mind-effect flags (`MIND_MASKED`, `SPIRIT_FORM`, …).
    pub effect_flags: u32,
    /// The character status word.
    pub status: u32,
    /// The technique word that resists possession.
    pub resistance: u32,
    /// The character flags (a body with `NO_RAISE` refuses the dead powers).
    pub character_flags: u32,
    /// The power tree's active count, as the persistent casts read it.
    pub power_count: i32,
    pub concerns: Vec<MindConcern>,
    /// The aggression value.
    pub aggression: f32,
    /// The fear value.
    pub fear: f32,
    /// The courage value.
    pub courage: f32,
    /// The calm factor.
    pub calm: f32,
    pub relations: Vec<MindRelation>,
    pub effects: Vec<MindSlot>,
    /// The two mask slots the table keeps per body.
    pub mask_slots: [Option<u32>; 2],
    /// The id the body is casting, for the repeat-cost technique.
    pub casting: u32,
    /// The focus held back for restored persistent powers.
    pub reserved: f32,
    /// The spirit-form screen's state (0 none, 1 held, 2 fading, 3 showing).
    pub spirit_screen: i32,
}

impl MindState {
    /// A body with the given id, focus and record type.
    pub fn new(id: u32, focus: f32, record_type: u32) -> Self {
        Self {
            id,
            focus,
            record_type,
            body_flags: 0,
            effect_flags: 0,
            status: 0,
            resistance: 0,
            character_flags: 0,
            power_count: 0,
            concerns: Vec::new(),
            aggression: 0.0,
            fear: 0.0,
            courage: 0.0,
            calm: 0.0,
            relations: Vec::new(),
            effects: Vec::new(),
            mask_slots: [None; 2],
            casting: 0,
            reserved: 0.0,
            spirit_screen: 0,
        }
    }

    /// The first effect with this id.
    pub fn effect(&self, id: u32) -> Option<&MindSlot> {
        self.effects.iter().find(|slot| slot.id == id)
    }

    /// The first effect with this id, mutably.
    pub fn effect_mut(&mut self, id: u32) -> Option<&mut MindSlot> {
        self.effects.iter_mut().find(|slot| slot.id == id)
    }

    /// The first effect with this id, or a new inert one.
    pub fn effect_or_insert(&mut self, id: u32) -> &mut MindSlot {
        if let Some(index) = self.effects.iter().position(|slot| slot.id == id) {
            &mut self.effects[index]
        } else {
            self.effects.push(MindSlot {
                id,
                state: 0,
                owner: None,
                target: None,
                kind: 0,
                level: 0.0,
                level2: 0.0,
                end_ms: 0,
                relation: 0.0,
            });
            let last = self.effects.len() - 1;
            &mut self.effects[last]
        }
    }
}

/// What a mind cast did besides changing the bodies.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindOutcome {
    pub code: i32,
    pub sound: Option<MindSound>,
    /// The sense overlay's duration, when the cast starts one.
    pub overlay_ms: Option<i32>,
    /// The focus the cast spent.
    pub spent: f32,
}

impl MindOutcome {
    fn refused() -> Self {
        Self {
            code: MIND_REFUSED,
            sound: None,
            overlay_ms: None,
            spent: 0.0,
        }
    }

    fn silent(code: i32) -> Self {
        Self {
            code,
            sound: None,
            overlay_ms: None,
            spent: 0.0,
        }
    }

    fn with(mut self, sound: MindSound) -> Self {
        self.sound = Some(sound);
        self
    }

    fn spending(mut self, caster: &mut MindState, amount: f32) -> Self {
        caster.focus -= amount;
        self.spent = amount;
        self
    }
}

/// Whether focus reaches a cost times a factor, the original's ordered
/// double compare: a NaN cost or focus passes.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn focus_reaches(focus: f32, cost: f32, factor: f64) -> bool {
    !(f64::from(cost) * factor > f64::from(focus))
}

/// Whether focus reaches a cost, the original's ordered single compare: a
/// NaN passes.
#[allow(clippy::neg_cmp_op_on_partial_ord)]
fn focus_covers(focus: f32, cost: f32) -> bool {
    !(focus < cost)
}

/// The duration a limited mind effect lasts: the longest span when focus
/// covers the cost, else the focus's share of it, rounded to nearest even.
pub fn mind_duration(focus: f32, cost: f32, span: i32) -> i32 {
    if focus_covers(focus, cost) {
        span
    } else {
        ((focus / cost) * span as f32).round_ties_even() as i32
    }
}

/// The cost a cast is priced at. An instant power repeated with the repeat
/// technique costs half; a sustained one is its table cost over the tick
/// and half again with the sustain technique; a persistent one keeps its
/// table cost. `casting` is the id the body is already casting.
pub fn mind_cost(id: u32, table_cost: f32, tick_ms: u32, techniques: u32, casting: u32) -> f32 {
    let row = id & 0x002f_ffff;
    if row & 0x3_0000 == 0 {
        if techniques & REPEAT_TECHNIQUE != 0 && casting == id {
            return table_cost * 0.5;
        }
        table_cost
    } else if row & 0x3_0000 == 0x1_0000 {
        let mut cost = (f32::from(tick_ms as u16) * table_cost) / 1000.0;
        if techniques & SUSTAIN_TECHNIQUE != 0 {
            cost *= 0.5;
        }
        cost
    } else {
        table_cost
    }
}

/// The focus a restored persistent mind power takes back: its table cost,
/// or three quarters with the reserve technique.
pub fn restore_mind_power(id: u32, state: &mut MindState) -> Option<f32> {
    let row = MIND_POWERS.iter().find(|row| row.id == id)?;
    if !matches!(
        row.cast,
        MindCast::SensePersistent | MindCast::MaskPersistent
    ) {
        return None;
    }
    let held = if state.resistance & MIND_RESERVE_TECHNIQUE != 0 {
        row.cost * 0.75
    } else {
        row.cost
    };
    state.reserved += held;
    Some(row.cost)
}

/// Ends the loyalty and release effects a target carries: a bind whose
/// owner is the target, or a release whose target is.
pub fn mind_end_effects(state: &mut MindState, target: u32) -> bool {
    for slot in &mut state.effects {
        let id = slot.id & 0x0010_ffff;
        if id == LOYALTY_BIND && slot.owner == Some(target) {
            slot.state = 2;
            return true;
        }
        if id == LOYALTY_RELEASE && slot.target == Some(target) {
            slot.state = 2;
            return true;
        }
    }
    false
}

/// Ends the loyalty-family effects whose owner test fails.
pub fn mind_end_orphans(state: &mut MindState, owner_ok: impl Fn(&MindSlot) -> bool) -> bool {
    for slot in &mut state.effects {
        let id = slot.id & 0x0010_ffff;
        if (id == LOYALTY_BIND || id == LOYALTY_OTHER) && !owner_ok(slot) {
            slot.state = 2;
            return true;
        }
    }
    false
}

/// Whether a body carries a loyalty-family effect.
pub fn mind_has_binding(state: &MindState) -> bool {
    state.effects.iter().any(|slot| {
        let id = slot.id & 0x0010_ffff;
        id == LOYALTY_BIND || id == LOYALTY_OTHER
    })
}

/// Ends the spirit-form and possession effects a body owns.
pub fn mind_end_spirit_effects(state: &mut MindState) {
    for slot in &mut state.effects {
        let id = slot.id & 0x0010_ffff;
        if (id == 0x106 || id == 0x10c) && slot.owner == Some(state.id) {
            slot.state = 2;
            return;
        }
    }
}

/// The calm cast: every concern loses up to ten into four times its decay,
/// the aggression drops ten, and the fear and courage follow, floored at
/// zero (a NaN is left).
pub fn mind_calm(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return MindOutcome::refused();
    }
    let mut outcome = MindOutcome::silent(MIND_APPLIED).with(SOUND_C01);
    outcome = outcome.spending(caster, cost);
    if mind_record(target.record_type) != MindRecord::Living {
        return MindOutcome {
            code: MIND_FAILED,
            ..outcome
        };
    }
    for concern in &mut target.concerns {
        if concern.decay_a <= 10.0 {
            concern.value += concern.decay_a * 3.0;
            concern.decay_a = 0.0;
        } else {
            concern.decay_a -= 10.0;
            concern.value += 30.0;
        }
        if concern.decay_b <= 10.0 {
            concern.value += concern.decay_b * 3.0;
            concern.decay_b = 0.0;
        } else {
            concern.decay_b -= 10.0;
            concern.value += 30.0;
        }
    }
    target.aggression -= 10.0;
    if target.aggression < 0.0 {
        target.aggression = 0.0;
    }
    target.fear -= 50.0;
    if target.fear < 0.0 {
        target.fear = 0.0;
    }
    target.courage -= 1.0;
    if target.courage < 0.0 {
        target.courage = 0.0;
    }
    target.calm = 1.0;
    outcome
}

/// The shared gate of the short-lived enchantments: living target, focus
/// of a third the cost; an undead target spends the cost quietly.
fn enchant_gate(
    caster: &mut MindState,
    target: &MindState,
    cost: f32,
    dead_sound: MindSound,
) -> Option<MindOutcome> {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return Some(MindOutcome::refused());
    }
    match mind_record(target.record_type) {
        MindRecord::Living => None,
        MindRecord::Undead => Some(
            MindOutcome::silent(MIND_FAILED)
                .with(dead_sound)
                .spending(caster, cost),
        ),
        MindRecord::Other => Some(MindOutcome::refused()),
    }
}

/// Enrage: fifty points of aggression and a marked body.
pub fn mind_enrage(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if let Some(outcome) = enchant_gate(caster, target, cost, SOUND_R01) {
        return outcome;
    }
    let mut outcome = MindOutcome::silent(MIND_APPLIED).with(SOUND_R01_LOUD);
    outcome = outcome.spending(caster, cost);
    let owner = caster.id;
    let slot = target.effect_or_insert(0x104);
    slot.state = 1;
    slot.level = 1.0;
    slot.target = Some(owner);
    target.aggression += 50.0;
    target.effect_flags |= MIND_ENRAGED;
    outcome
}

/// How the confusion cast clears a body's concerns and adds fifty of
/// aggression.
pub fn mind_confusion(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if let Some(outcome) = enchant_gate(caster, target, cost, SOUND_C02) {
        return outcome;
    }
    let mut outcome = MindOutcome::silent(MIND_APPLIED).with(SOUND_C02);
    outcome = outcome.spending(caster, cost);
    let owner = caster.id;
    let slot = target.effect_or_insert(0x105);
    slot.state = 1;
    slot.level = 0.9;
    slot.level2 = 1.0;
    slot.target = Some(owner);
    target.aggression += 50.0;
    for concern in &mut target.concerns {
        concern.value = 0.0;
        concern.decay_a = 0.0;
        concern.decay_b = 0.0;
    }
    target.effect_flags |= MIND_CONFUSED;
    outcome
}

/// Fear: twenty points of the fear value, capped at forty.
pub fn mind_fear(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if let Some(outcome) = enchant_gate(caster, target, cost, SOUND_F01) {
        return outcome;
    }
    let mut outcome = MindOutcome::silent(MIND_APPLIED).with(SOUND_F01);
    outcome = outcome.spending(caster, cost);
    target.fear += 20.0;
    if target.fear > 40.0 {
        target.fear = 40.0;
    }
    outcome
}

/// Embolden: fifty points of courage, capped at eighty.
pub fn mind_embolden(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return MindOutcome::refused();
    }
    match mind_record(target.record_type) {
        MindRecord::Living => {}
        MindRecord::Undead => {
            if focus_reaches(caster.focus, cost, 0.3) {
                return MindOutcome::silent(MIND_FAILED)
                    .with(SOUND_C03)
                    .spending(caster, cost);
            }
            return MindOutcome::refused();
        }
        MindRecord::Other => return MindOutcome::refused(),
    }
    let mut outcome = MindOutcome::silent(MIND_APPLIED).with(SOUND_C03);
    outcome = outcome.spending(caster, cost);
    target.courage += 50.0;
    if target.courage > 80.0 {
        target.courage = 80.0;
    }
    outcome
}

/// Suspicion: a marked body and its effect, whose id carries the
/// suspicious bit.
pub fn mind_suspicion(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if !focus_covers(caster.focus, cost) {
        return MindOutcome::refused();
    }
    let id = 0x10B;
    match mind_record(target.record_type) {
        MindRecord::Undead => {
            return MindOutcome::silent(MIND_FAILED)
                .with(SOUND_S01)
                .spending(caster, cost);
        }
        MindRecord::Other => return MindOutcome::refused(),
        MindRecord::Living => {}
    }
    let masked = id | MIND_SUSPICIOUS;
    let owner = caster.id;
    let slot = target.effect_or_insert(masked);
    slot.state = 1;
    slot.target = Some(owner);
    MindOutcome::silent(MIND_APPLIED)
        .with(SOUND_S01)
        .spending(caster, cost)
}

/// The loyalty start: a bind on the target, a relation of thirty to the
/// caster, and a power object on the target.
pub fn mind_loyalty_start(
    caster: &mut MindState,
    target: &mut MindState,
    cost: f32,
    relation: f32,
) -> MindOutcome {
    match mind_record(target.record_type) {
        MindRecord::Undead => {
            if focus_reaches(caster.focus, cost, 0.3) {
                return MindOutcome::silent(MIND_FAILED)
                    .with(SOUND_T01)
                    .spending(caster, cost);
            }
            return MindOutcome::refused();
        }
        MindRecord::Other => return MindOutcome::refused(),
        MindRecord::Living => {}
    }
    if let Some(slot) = caster.effect(LOYALTY_BIND) {
        if slot.owner.is_some_and(|owner| owner != target.id) {
            return MindOutcome::refused();
        }
        if let Some(slot) = caster.effect_mut(LOYALTY_BIND) {
            slot.state = 2;
        }
        return MindOutcome::silent(MIND_ENDED).with(SOUND_T01);
    }
    let mut count = caster.power_count;
    for slot in &caster.effects {
        if slot.id & 0x2_0000 != 0 {
            count -= 1;
        }
    }
    if count < 1 || !focus_covers(caster.focus, cost) {
        return MindOutcome::refused();
    }
    let scaled = (relation + 100.0) * 0.005;
    let owner = caster.id;
    let target_id = target.id;
    {
        let slot = caster.effect_or_insert(LOYALTY_BIND);
        slot.state = 1;
        slot.owner = Some(target_id);
        slot.relation = scaled * scaled * 100.0;
    }
    if let Some(entry) = caster
        .relations
        .iter_mut()
        .find(|entry| entry.body == target_id)
    {
        entry.weight_a = 0.0;
        entry.value = 30.0;
        entry.weight_b = 0.0;
    }
    target.effects.push(MindSlot {
        id: 0x2010A,
        state: 1,
        owner: Some(owner),
        target: None,
        kind: 0,
        level: cost,
        level2: 0.0,
        end_ms: 0,
        relation: 0.0,
    });
    MindOutcome::silent(MIND_APPLIED)
        .with(SOUND_T01)
        .spending(caster, cost)
}

/// The loyalty release: an effect that clears half the target's relation
/// weights and sets the relation to thirty.
pub fn mind_loyalty_end(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return MindOutcome::refused();
    }
    match mind_record(target.record_type) {
        MindRecord::Undead => {
            return MindOutcome::silent(MIND_FAILED)
                .with(SOUND_T01)
                .spending(caster, cost);
        }
        MindRecord::Other => return MindOutcome::refused(),
        MindRecord::Living => {}
    }
    if caster
        .effect(LOYALTY_BIND)
        .is_some_and(|slot| slot.owner == Some(target.id))
    {
        return MindOutcome::refused();
    }
    let id = 0x109 | 0x10_0000;
    let owner = caster.id;
    let slot = caster.effect_or_insert(id);
    slot.state = 1;
    slot.owner = Some(owner);
    slot.target = Some(target.id);
    if let Some(entry) = caster
        .relations
        .iter_mut()
        .find(|entry| entry.body == target.id)
    {
        entry.weight_a *= 0.5;
        entry.value = 30.0;
        entry.weight_b *= 0.5;
    }
    MindOutcome::silent(MIND_APPLIED)
        .with(SOUND_T01)
        .spending(caster, cost)
}

/// The sense: a limited sense effect and the overlay's duration. The
/// undead answer with the cost spent and no effect.
pub fn mind_sense(caster: &mut MindState, cost: f32, now_ms: u32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return MindOutcome::refused();
    }
    let duration = mind_duration(caster.focus, cost, SENSE_SPAN_MS);
    if caster.effect(0x101 | 0x2_0000).is_some() {
        return MindOutcome::refused();
    }
    let slot = caster.effect_or_insert(0x101);
    slot.state = 1;
    slot.level = 0.9;
    slot.level2 = 1.0;
    slot.end_ms = now_ms.wrapping_add(duration as u32) as i32;
    let outcome = MindOutcome::silent(MIND_APPLIED).with(SOUND_A01);
    let outcome = outcome.spending(caster, cost);
    MindOutcome {
        overlay_ms: Some(duration),
        ..outcome
    }
}

/// The persistent sense: a second cast ends the running one.
pub fn mind_sense_persistent(caster: &mut MindState, cost: f32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return MindOutcome::refused();
    }
    if caster.power_count < 1 {
        return MindOutcome::refused();
    }
    if caster.effect(0x20101).is_some() {
        if let Some(slot) = caster.effect_mut(0x20101) {
            slot.state = 2;
        }
        return MindOutcome::silent(MIND_ENDED).with(SOUND_E01);
    }
    let mut count = caster.power_count;
    for slot in &caster.effects {
        if slot.id & 0x2_0000 != 0 {
            count -= 1;
            if count < 1 {
                return MindOutcome::refused();
            }
        }
    }
    if !focus_covers(caster.focus, cost) {
        return MindOutcome::refused();
    }
    if let Some(slot) = caster.effect_mut(0x101) {
        slot.state |= 2;
    }
    let owner = caster.id;
    caster.effects.push(MindSlot {
        id: 0x20101,
        state: 1,
        owner: Some(owner),
        target: None,
        kind: 0,
        level: 1.0,
        level2: 0.0,
        end_ms: 0,
        relation: 0.0,
    });
    MindOutcome::silent(MIND_APPLIED)
        .with(SOUND_A01)
        .spending(caster, cost)
}

/// The mask: a body flag and a limited effect. Neither mask slot may hold
/// the power.
pub fn mind_mask(caster: &mut MindState, cost: f32, caster_word: u32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return MindOutcome::refused();
    }
    let masked = 0x102 | 0x2_0000;
    if caster.mask_slots.iter().flatten().any(|&id| id == masked) {
        return MindOutcome::refused();
    }
    let _duration = mind_duration(caster.focus, cost, SENSE_SPAN_MS);
    if caster.effect(masked).is_some() {
        return MindOutcome::refused();
    }
    let slot = caster.effect_or_insert(0x102);
    slot.state = 1;
    slot.level = 0.9;
    slot.level2 = 1.0;
    slot.target = Some(caster_word);
    caster.effect_flags |= MIND_MASKED;
    MindOutcome::silent(MIND_APPLIED)
        .with(SOUND_B01)
        .spending(caster, cost)
}

/// The persistent mask: a second cast ends the running one.
pub fn mind_mask_persistent(caster: &mut MindState, cost: f32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3) {
        return MindOutcome::refused();
    }
    if caster.power_count < 1 {
        return MindOutcome::refused();
    }
    if caster.effect(0x20102).is_some() {
        if let Some(slot) = caster.effect_mut(0x20102) {
            slot.state = 2;
        }
        return MindOutcome::silent(MIND_ENDED).with(SOUND_E01);
    }
    let mut count = caster.power_count;
    for slot in &caster.effects {
        if slot.id & 0x2_0000 != 0 {
            count -= 1;
            if count < 1 {
                return MindOutcome::refused();
            }
        }
    }
    if !focus_covers(caster.focus, cost) {
        return MindOutcome::refused();
    }
    let masked = 0x20102;
    if let Some(slot) = caster.effect_mut(0x102 | 0x2_0000) {
        slot.state |= 2;
    }
    caster.effects.push(MindSlot {
        id: masked,
        state: 1,
        owner: Some(caster.id),
        target: None,
        kind: 0,
        level: 0.9,
        level2: 0.8,
        end_ms: 0,
        relation: 0.0,
    });
    caster.effect_flags |= MIND_MASKED;
    MindOutcome::silent(MIND_APPLIED)
        .with(SOUND_B01)
        .spending(caster, cost)
}

/// The spirit form: a screen effect on the caster and a form effect on the
/// body, while focus stays above 0.05. A second cast ends it.
pub fn mind_spirit_form(caster: &mut MindState, cost: f32) -> MindOutcome {
    if caster.focus < 0.05 {
        return MindOutcome::refused();
    }
    let id = 0x10106;
    if caster.effect(id | 0x2_0000).is_some() {
        return MindOutcome::refused();
    }
    if caster.effect(id).is_some() {
        if caster.spirit_screen == 3 {
            return MindOutcome::refused();
        }
        if let Some(slot) = caster.effect_mut(id) {
            slot.state = 2;
        }
        return MindOutcome::silent(MIND_ENDED);
    }
    if caster.spirit_screen == 0 {
        caster.spirit_screen = 3;
    }
    let owner = caster.id;
    caster.effects.push(MindSlot {
        id,
        state: 1,
        owner: Some(owner),
        target: None,
        kind: 0,
        level: cost,
        level2: 0.0,
        end_ms: 0,
        relation: 0.0,
    });
    caster.effect_flags |= SPIRIT_FORM;
    MindOutcome::silent(MIND_APPLIED)
}

/// The possession: an effect on the caster that holds the target's body.
/// The undead and a resistant target spend half the cost and fail.
pub fn mind_possession(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    let id = 0x2010C;
    if caster.effect(id).is_some() {
        let state = caster.effect(id).map(|slot| slot.state);
        if state == Some(1) {
            return MindOutcome::refused();
        }
        if let Some(slot) = caster.effect_mut(id) {
            slot.state = 2;
        }
        return MindOutcome::silent(MIND_ENDED).with(SOUND_E01);
    }
    let mut count = caster.power_count;
    for slot in &caster.effects {
        if slot.id & 0xffff == 0x106 {
            count = 0;
        } else if slot.id & 0x2_0000 != 0 {
            count -= 1;
        }
    }
    if count < 1 || !focus_covers(caster.focus, cost) {
        return MindOutcome::refused();
    }
    match mind_record(target.record_type) {
        MindRecord::Living => {
            if target.resistance & POSSESSION_RESIST != 0 {
                return MindOutcome::silent(MIND_FAILED)
                    .with(SOUND_E02)
                    .spending(caster, cost * 0.5);
            }
            let owner = caster.id;
            caster.effects.push(MindSlot {
                id,
                state: 1,
                owner: Some(owner),
                target: Some(target.id),
                kind: 0,
                level: cost,
                level2: 0.0,
                end_ms: 0,
                relation: 0.0,
            });
            // The success is free: the effect drains its cost while it runs.
            MindOutcome::silent(MIND_APPLIED).with(SOUND_CTRL)
        }
        MindRecord::Undead => MindOutcome::silent(MIND_FAILED)
            .with(SOUND_E02)
            .spending(caster, cost * 0.5),
        MindRecord::Other => MindOutcome::refused(),
    }
}

/// The raise dead: a corpse with the corpse bits, brought back without a
/// bind. The undead answer with the cost spent.
pub fn mind_raise_dead(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if !focus_reaches(caster.focus, cost, 0.3)
        || target.status & UNDEAD_RAISED != 0
        || target.character_flags & NO_RAISE != 0
    {
        return MindOutcome::refused();
    }
    match mind_record(target.record_type) {
        MindRecord::Undead => MindOutcome::silent(MIND_FAILED)
            .with(SOUND_R02)
            .spending(caster, cost),
        MindRecord::Living => {
            if target.body_flags & CORPSE_FLAGS != CORPSE_FLAGS || !focus_covers(caster.focus, cost)
            {
                return MindOutcome::refused();
            }
            revive(target);
            MindOutcome::silent(MIND_APPLIED)
                .with(SOUND_R02)
                .spending(caster, cost)
        }
        MindRecord::Other => MindOutcome::refused(),
    }
}

/// The command over the dead: a corpse with the corpse bits, bound to the
/// caster through a command effect.
pub fn mind_command_dead(caster: &mut MindState, target: &mut MindState, cost: f32) -> MindOutcome {
    if target.status & UNDEAD_RAISED != 0 || target.character_flags & NO_RAISE != 0 {
        return MindOutcome::refused();
    }
    match mind_record(target.record_type) {
        MindRecord::Undead => {
            if focus_reaches(caster.focus, cost, 0.3) {
                return MindOutcome::silent(MIND_FAILED)
                    .with(SOUND_R02)
                    .spending(caster, cost);
            }
            return MindOutcome::refused();
        }
        MindRecord::Living => {}
        MindRecord::Other => return MindOutcome::refused(),
    }
    if target.body_flags & CORPSE_FLAGS != CORPSE_FLAGS {
        // Not a corpse: a second command on a body the caster already
        // commands ends the command.
        if caster
            .effect(COMMAND_BIND)
            .is_some_and(|slot| slot.owner == Some(target.id))
        {
            if let Some(slot) = caster.effect_mut(COMMAND_BIND) {
                slot.state = 2;
            }
            return MindOutcome::silent(MIND_ENDED).with(SOUND_E01);
        }
        return MindOutcome::silent(MIND_FAILED);
    }
    let mut count = caster.power_count;
    for slot in &caster.effects {
        if slot.id & 0x2_0000 != 0 {
            count -= 1;
            if count < 1 {
                return MindOutcome::refused();
            }
        }
    }
    if !focus_covers(caster.focus, cost) {
        return MindOutcome::refused();
    }
    revive(target);
    let owner = caster.id;
    let target_id = target.id;
    {
        let slot = target.effect_or_insert(COMMAND_BIND);
        slot.state = 1;
        slot.owner = Some(owner);
        slot.target = Some(target_id);
    }
    if let Some(entry) = caster
        .relations
        .iter_mut()
        .find(|entry| entry.body == target_id)
    {
        entry.weight_a = 0.0;
        entry.value = 30.0;
        entry.weight_b = 0.0;
    }
    caster.effects.push(MindSlot {
        id: 0x2010D,
        state: 1,
        owner: Some(owner),
        target: Some(target_id),
        kind: 0,
        level: cost,
        level2: 0.0,
        end_ms: 0,
        relation: 0.0,
    });
    MindOutcome::silent(MIND_APPLIED)
        .with(SOUND_R02)
        .spending(caster, cost)
}

/// What a revival changes on a body: its condition word, the movement and
/// strength it keeps, and the condition word's slow tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Revival {
    pub condition: u32,
    pub strength: f32,
    pub slow_tick: i32,
    pub speed: f32,
    pub strength_scale: f32,
    pub restored: f32,
}

/// Brings a corpse back: the raised condition word, a slower strength, the
/// speed and the slow tick.
pub fn revive(state: &mut MindState) -> Revival {
    let strength = 0.2f32;
    let condition = if strength <= 0.0 { 2 } else { 8 };
    state.status |= if strength <= 0.0 {
        UNDEAD_RAISED | 2
    } else {
        8
    };
    let strength_scale = if strength <= 0.0 {
        1.25
    } else {
        1.25 - strength * strength * 0.25
    };
    let slow_tick = (((1.0 - strength * 0.25) as f64 * 1660.0) / 83.0).round_ties_even() as i32;
    Revival {
        condition,
        strength,
        slow_tick,
        speed: 1.0 - strength * 0.1,
        strength_scale,
        restored: strength,
    }
}

/// A mind glow's state: when it started and how bright it is.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindGlow {
    pub start_ms: Option<u32>,
    pub alpha: f32,
}

/// Steps a mind glow: it fades in over a second, then halves over each of
/// the next two, and ends after three.
pub fn step_mind_glow(glow: &mut MindGlow, now_ms: u32) -> bool {
    if glow.start_ms.is_none() {
        glow.start_ms = Some(now_ms);
    }
    let start = glow.start_ms.unwrap_or(now_ms);
    let elapsed = now_ms.wrapping_sub(start);
    if elapsed < MIND_GLOW_MS {
        let seconds = elapsed as f32 / 1000.0;
        glow.alpha = if seconds <= 1.0 {
            1.0 - seconds
        } else {
            (seconds - 1.0) * 0.5
        };
        true
    } else {
        false
    }
}

/// The mind-sight overlay's screen constants: its square texture, the
/// sphere's segment count and radius, and the noise's two octaves.
pub const MIND_SIGHT_TEXTURE: u32 = 0x100;
pub const MIND_SIGHT_SEGMENTS: u32 = 0x10;
pub const MIND_SIGHT_RADIUS: f32 = 40.0;
pub const MIND_SIGHT_NOISE_OCTAVES: u32 = 4;
pub const MIND_SIGHT_NOISE_SCALE: f32 = 0.5;

/// The spirit screen's texture side and its noise scale.
pub const SPIRIT_TEXTURE: u32 = 0x200;
pub const SPIRIT_NOISE_SCALE: f32 = 0.8;

/// A tracked body on the mind sight.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightNode {
    pub body: u32,
    pub kind: i32,
    pub seen_ms: u32,
    pub fade: f32,
    pub weights: MindEntryColors,
}

/// One body offered to the sight this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SightCandidate {
    pub id: u32,
    pub state: u32,
    pub effect_flags: u32,
    pub record_type: u32,
    pub values: MindBodyValues,
    /// The body carries a mark that forces full weights.
    pub marked: bool,
}

/// The mind sight's tracked bodies.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MindSight {
    pub nodes: Vec<SightNode>,
    pub stamp_ms: u32,
}

/// The entry kind the sight gives a record: 2 for a living body, 1 for an
/// undead one, 0 for neither.
pub fn sight_kind(record_type: u32) -> i32 {
    match mind_record(record_type) {
        MindRecord::Living => 2,
        MindRecord::Undead => 1,
        MindRecord::Other => 0,
    }
}

/// Advances the mind sight one step: a tick every four milliseconds, the
/// fade over the dwell window, a new node's weights snapped, an old one's
/// blended by `2^(−dt/1000)`.
pub fn step_mind_sight(sight: &mut MindSight, bodies: &[SightCandidate], now_ms: u32) -> bool {
    if !mind_track_due(now_ms, sight.stamp_ms) {
        return false;
    }
    let elapsed = now_ms.wrapping_sub(sight.stamp_ms);
    sight.stamp_ms = now_ms;
    for body in bodies {
        if body.effect_flags & 0x10 != 0 {
            continue;
        }
        let dead = mind_record(body.record_type) == MindRecord::Undead;
        let mut tracked = true;
        if let Some(node) = sight.nodes.iter_mut().find(|node| node.body == body.id) {
            let window = if body.state < 2 {
                if dead {
                    Some(SIGHT_MARKED_WINDOW_MS)
                } else {
                    None
                }
            } else {
                Some(SIGHT_WINDOW_MS)
            };
            match window {
                Some(window) if node.seen_ms != 0 && now_ms > node.seen_ms.wrapping_add(window) => {
                    tracked = false;
                }
                Some(_) if node.seen_ms == 0 => {
                    node.seen_ms = now_ms;
                    node.fade = 0.0;
                }
                Some(window) => {
                    node.fade = now_ms.wrapping_sub(node.seen_ms) as f32 / window as f32;
                }
                None => {}
            }
        }
        if tracked {
            let target = mind_track_weights(&body.values, body.marked);
            if let Some(node) = sight.nodes.iter_mut().find(|node| node.body == body.id) {
                let mut colors = node.weights;
                colors = blend_mind_weights(&mut colors, target, elapsed);
                node.weights = colors;
            } else {
                sight.nodes.push(SightNode {
                    body: body.id,
                    kind: sight_kind(body.record_type),
                    seen_ms: 0,
                    fade: 0.0,
                    weights: MindEntryColors {
                        first: target.threshold,
                        second: target.maximum,
                        third: target.ratio,
                    },
                });
            }
        } else if let Some(index) = sight.nodes.iter().position(|node| node.body == body.id) {
            sight.nodes.remove(index);
        }
    }
    true
}

/// The sight entry a node makes.
pub fn sight_style(node: &SightNode) -> MindEntryStyle {
    MindEntryStyle {
        fade: node.fade,
        kind: node.kind,
        first: node.weights.first,
        second: node.weights.second,
        third: node.weights.third,
    }
}

#[cfg(test)]
mod power_tests {
    use super::*;

    #[test]
    fn the_table_matches_the_recorded_rows() {
        assert_eq!(MIND_POWERS.len(), 16);
        assert_eq!(MIND_POWERS[0].id, 0x101);
        assert_eq!(MIND_POWERS[0].cost, 0.13);
        assert_eq!(MIND_POWERS[0].cast, MindCast::Sense);
        assert_eq!(MIND_POWERS[4].id, 0x10106);
        assert_eq!(MIND_POWERS[12].cast, MindCast::CommandDead);
        assert_eq!(MIND_POWERS[15].cost, 0.3);
        assert_eq!(mind_row(0x10_0101).map(|row| row.id), Some(0x101));
    }

    #[test]
    fn a_repeated_instant_power_costs_half_and_a_sustained_one_the_tick() {
        assert_eq!(mind_cost(0x103, 0.14, 12, 0, 0), 0.14);
        assert_eq!(mind_cost(0x103, 0.14, 12, REPEAT_TECHNIQUE, 0x103), 0.07);
        assert_eq!(mind_cost(0x103, 0.14, 12, REPEAT_TECHNIQUE, 0x104), 0.14);
        let tick = mind_cost(0x10106, 0.025, 12, 0, 0);
        assert_eq!(tick, 12.0f32 * 0.025 / 1000.0);
        assert_eq!(
            mind_cost(0x10106, 0.025, 12, SUSTAIN_TECHNIQUE, 0),
            tick * 0.5
        );
        assert_eq!(
            mind_cost(0x2010A, 0.29, 12, REPEAT_TECHNIQUE, 0x2010A),
            0.29
        );
    }

    #[test]
    fn a_restored_sense_holds_its_cost() {
        let mut state = MindState::new(1, 1.0, 0);
        assert_eq!(restore_mind_power(0x20101, &mut state), Some(0.27));
        assert_eq!(restore_mind_power(0x20102, &mut state), Some(0.24));
        assert_eq!(restore_mind_power(0x103, &mut state), None);
        state.resistance = MIND_RESERVE_TECHNIQUE;
        assert_eq!(restore_mind_power(0x20101, &mut state), Some(0.27));
        assert!((state.reserved - 0.27 - 0.24 - 0.2025).abs() < 1e-6);
    }
}

#[cfg(test)]
mod cast_tests {
    use super::*;

    fn ready(focus: f32) -> MindState {
        let mut state = MindState::new(1, focus, 0);
        state.power_count = 1;
        state
    }

    fn living(id: u32) -> MindState {
        MindState::new(id, 1.0, 2)
    }

    #[test]
    fn calm_lowers_concerns_and_values() {
        let mut caster = ready(1.0);
        let mut target = living(7);
        target.concerns = vec![
            MindConcern {
                value: 5.0,
                decay_a: 3.0,
                decay_b: 20.0,
            },
            MindConcern {
                value: 1.0,
                decay_a: 0.0,
                decay_b: 0.0,
            },
        ];
        target.aggression = 15.0;
        target.fear = 60.0;
        target.courage = 2.0;
        let outcome = mind_calm(&mut caster, &mut target, 0.14);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(outcome.sound, Some(SOUND_C01));
        assert!((caster.focus - 0.86).abs() < 1e-6);
        assert_eq!(target.concerns[0].value, 5.0 + 9.0 + 30.0);
        assert_eq!(target.concerns[0].decay_a, 0.0);
        assert_eq!(target.concerns[0].decay_b, 10.0);
        assert_eq!(target.aggression, 5.0);
        assert_eq!(target.fear, 10.0);
        assert_eq!(target.courage, 1.0);
        assert_eq!(target.calm, 1.0);
    }

    #[test]
    fn a_third_the_cost_gates_the_casts() {
        let mut poor = ready(0.04);
        let mut target = living(7);
        assert_eq!(mind_enrage(&mut poor, &mut target, 0.16).code, MIND_REFUSED);
        assert_eq!(poor.focus, 0.04);
        let mut rich = ready(1.0);
        assert_eq!(mind_enrage(&mut rich, &mut target, 0.16).code, MIND_APPLIED);
        assert_eq!(target.aggression, 50.0);
        assert_eq!(target.effect_flags, MIND_ENRAGED);
        assert!((rich.focus - 0.84).abs() < 1e-6);
    }

    #[test]
    fn the_undead_casts_spend_and_fail() {
        let mut caster = ready(1.0);
        let mut undead = MindState::new(9, 1.0, 4);
        let outcome = mind_fear(&mut caster, &mut undead, 0.18);
        assert_eq!(outcome.code, MIND_FAILED);
        assert!((caster.focus - 0.82).abs() < 1e-6);
        assert_eq!(undead.fear, 0.0);
        let mut other = MindState::new(9, 1.0, 6);
        assert_eq!(mind_fear(&mut caster, &mut other, 0.18).code, MIND_REFUSED);
    }

    #[test]
    fn fear_and_embolden_cap_their_values() {
        let mut caster = ready(2.0);
        let mut target = living(7);
        target.fear = 30.0;
        mind_fear(&mut caster, &mut target, 0.18);
        assert_eq!(target.fear, 40.0);
        target.courage = 50.0;
        mind_embolden(&mut caster, &mut target, 0.18);
        assert_eq!(target.courage, 80.0);
    }

    #[test]
    fn confusion_clears_the_concerns() {
        let mut caster = ready(1.0);
        let mut target = living(7);
        target.concerns.push(MindConcern {
            value: 20.0,
            decay_a: 5.0,
            decay_b: 1.0,
        });
        let outcome = mind_confusion(&mut caster, &mut target, 0.17);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(target.concerns[0].value, 0.0);
        assert_eq!(target.aggression, 50.0);
        assert_eq!(target.effect_flags, MIND_CONFUSED);
    }

    #[test]
    fn suspicion_marks_and_keeps_its_effect() {
        let mut caster = ready(1.0);
        let mut target = living(7);
        let outcome = mind_suspicion(&mut caster, &mut target, 0.19);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert!(target.effect(0x10B | MIND_SUSPICIOUS).is_some());
        assert_eq!(target.effect_flags, 0);
    }

    #[test]
    fn loyalty_binds_and_releases() {
        let mut caster = ready(1.0);
        caster.relations.push(MindRelation {
            body: 7,
            value: 0.0,
            weight_a: 4.0,
            weight_b: 2.0,
        });
        let mut target = living(7);
        let outcome = mind_loyalty_start(&mut caster, &mut target, 0.29, 20.0);
        assert_eq!(outcome.code, MIND_APPLIED);
        let bind = caster.effect(LOYALTY_BIND).expect("a bind");
        assert_eq!(bind.owner, Some(7));
        assert!((bind.relation - 36.0).abs() < 1e-5);
        assert_eq!(caster.relations[0].value, 30.0);
        assert!(target.effect(0x2010A).is_some());
        assert!((caster.focus - 0.71).abs() < 1e-6);

        let mut other = MindState::new(8, 1.0, 2);
        assert_eq!(
            mind_loyalty_start(&mut caster, &mut other, 0.29, 0.0).code,
            MIND_REFUSED
        );
        let ended = mind_loyalty_start(&mut caster, &mut target, 0.29, 0.0);
        assert_eq!(ended.code, MIND_ENDED);
        assert_eq!(caster.effect(LOYALTY_BIND).map(|slot| slot.state), Some(2));
    }

    #[test]
    fn the_loyalty_end_halves_the_weights() {
        let mut caster = ready(1.0);
        caster.relations.push(MindRelation {
            body: 7,
            value: 5.0,
            weight_a: 4.0,
            weight_b: 2.0,
        });
        let mut target = living(7);
        let outcome = mind_loyalty_end(&mut caster, &mut target, 0.19);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(caster.relations[0].weight_a, 2.0);
        assert_eq!(caster.relations[0].weight_b, 1.0);
        assert_eq!(caster.relations[0].value, 30.0);
        let effect = caster.effect(0x100109).expect("a release");
        assert_eq!(effect.target, Some(7));
    }

    #[test]
    fn the_sense_runs_for_four_seconds_or_the_focus_share() {
        let mut caster = ready(1.0);
        let outcome = mind_sense(&mut caster, 0.13, 1000);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(outcome.overlay_ms, Some(4000));
        let slot = caster.effect(0x101).expect("a sense");
        assert_eq!(slot.end_ms, 5000);
        let mut poor = ready(0.1);
        let outcome = mind_sense(&mut poor, 0.13, 0);
        assert_eq!(outcome.overlay_ms, Some(3077));
        assert!((poor.focus + 0.03).abs() < 1e-6);
    }

    #[test]
    fn a_second_persistent_sense_ends_the_first() {
        let mut caster = ready(1.0);
        assert_eq!(mind_sense_persistent(&mut caster, 0.27).code, MIND_APPLIED);
        assert!(caster.effect(0x20101).is_some());
        assert_eq!(mind_sense_persistent(&mut caster, 0.27).code, MIND_ENDED);
        assert_eq!(caster.effect(0x20101).map(|slot| slot.state), Some(2));
    }

    #[test]
    fn the_mask_sets_and_clears_the_body_flag() {
        let mut caster = ready(1.0);
        let outcome = mind_mask(&mut caster, 0.12, 42);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(caster.effect_flags, MIND_MASKED);
        let slot = caster.effect(0x102).expect("a mask");
        assert_eq!(slot.target, Some(42));
        assert_eq!(mind_mask(&mut caster, 0.12, 42).code, MIND_APPLIED);
        assert_eq!(caster.effects.len(), 1);
        let mut other = ready(1.0);
        assert_eq!(mind_mask(&mut other, 0.12, 42).code, MIND_APPLIED);
    }

    #[test]
    fn the_persistent_mask_ends_and_reapplies() {
        let mut caster = ready(1.0);
        assert_eq!(mind_mask_persistent(&mut caster, 0.24).code, MIND_APPLIED);
        assert_eq!(caster.effect(0x20102).map(|slot| slot.level2), Some(0.8));
        assert_eq!(mind_mask_persistent(&mut caster, 0.24).code, MIND_ENDED);
    }

    #[test]
    fn possession_resists_or_takes_the_body() {
        let mut caster = ready(1.0);
        let mut target = living(7);
        target.resistance = POSSESSION_RESIST;
        let outcome = mind_possession(&mut caster, &mut target, 0.35);
        assert_eq!(outcome.code, MIND_FAILED);
        assert_eq!(outcome.sound, Some(SOUND_E02));
        assert!((caster.focus - 0.825).abs() < 1e-6);

        let mut caster = ready(1.0);
        let mut target = living(7);
        let outcome = mind_possession(&mut caster, &mut target, 0.35);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(caster.focus, 1.0);
        let slot = caster.effect(0x2010C).expect("a possession");
        assert_eq!(slot.target, Some(7));
        assert_eq!(slot.state, 1);
    }

    #[test]
    fn the_dead_are_raised_and_commanded() {
        let mut caster = ready(1.0);
        let mut target = living(7);
        target.body_flags = CORPSE_FLAGS;
        let outcome = mind_raise_dead(&mut caster, &mut target, 0.3);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(target.status & 8, 8);
        let revival = revive(&mut target);
        assert_eq!(revival.condition, 8);
        assert!((revival.strength_scale - 1.24).abs() < 1e-6);
        assert_eq!(revival.slow_tick, 19);

        let mut caster = ready(1.0);
        let mut corpse = living(9);
        corpse.body_flags = CORPSE_FLAGS;
        let outcome = mind_command_dead(&mut caster, &mut corpse, 0.25);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert!(corpse.effect(COMMAND_BIND).is_some());
        assert!(caster.effect(0x2010D).is_some());
    }

    #[test]
    fn a_raised_body_refuses_the_dead_powers() {
        let mut caster = ready(1.0);
        let mut target = living(7);
        target.status = UNDEAD_RAISED;
        assert_eq!(
            mind_raise_dead(&mut caster, &mut target, 0.3).code,
            MIND_REFUSED
        );
        target.status = 8;
        target.character_flags = NO_RAISE;
        assert_eq!(
            mind_command_dead(&mut caster, &mut target, 0.25).code,
            MIND_REFUSED
        );
    }

    #[test]
    fn the_end_helpers_clear_the_right_effects() {
        let mut state = MindState::new(1, 1.0, 2);
        state.effects.push(MindSlot {
            id: LOYALTY_BIND,
            state: 1,
            owner: Some(7),
            target: None,
            kind: 0,
            level: 0.0,
            level2: 0.0,
            end_ms: 0,
            relation: 0.0,
        });
        state.effects.push(MindSlot {
            id: LOYALTY_RELEASE,
            state: 1,
            owner: Some(1),
            target: Some(7),
            kind: 0,
            level: 0.0,
            level2: 0.0,
            end_ms: 0,
            relation: 0.0,
        });
        state.effects.push(MindSlot {
            id: 0x10106,
            state: 1,
            owner: Some(1),
            target: None,
            kind: 0,
            level: 0.0,
            level2: 0.0,
            end_ms: 0,
            relation: 0.0,
        });
        assert!(mind_has_binding(&state));
        assert!(mind_end_effects(&mut state, 7));
        assert_eq!(state.effects[0].state, 2);
        mind_end_spirit_effects(&mut state);
        assert_eq!(state.effects[2].state, 2);
    }

    #[test]
    fn the_glow_fades_in_and_ends() {
        let mut glow = MindGlow {
            start_ms: None,
            alpha: 0.0,
        };
        assert!(step_mind_glow(&mut glow, 100));
        assert_eq!(glow.alpha, 1.0);
        assert!(step_mind_glow(&mut glow, 1600));
        assert!((glow.alpha - 0.25).abs() < 1e-6);
        assert!(!step_mind_glow(&mut glow, 3100));
    }

    #[test]
    fn the_sight_tracks_dwells_and_drops() {
        let mut sight = MindSight::default();
        let values = MindBodyValues {
            v78: 5.0,
            v974: 0.0,
            v978: 0.0,
            v7b4: 0.0,
            v7c: 0.0,
            v74: 0.0,
        };
        let body = SightCandidate {
            id: 5,
            state: 2,
            effect_flags: 0,
            record_type: 2,
            values,
            marked: false,
        };
        assert!(!step_mind_sight(&mut sight, &[body], 0));
        assert!(step_mind_sight(&mut sight, &[body], 4));
        assert_eq!(sight.nodes.len(), 1);
        assert_eq!(sight.nodes[0].kind, 2);
        assert_eq!(sight.nodes[0].weights.first, 1.0);
        assert!(step_mind_sight(&mut sight, &[body], 1500));
        assert_eq!(sight.nodes[0].fade, 0.0);
        assert!(step_mind_sight(&mut sight, &[body], 3000));
        assert!((sight.nodes[0].fade - 0.5).abs() < 1e-6);
        assert!(step_mind_sight(&mut sight, &[body], 4501));
        assert!(sight.nodes.is_empty());
    }
}

/// Starts the mind-sight overlay: a fresh start registers its frame callback
/// and sets the shown state; a later one extends its end.
pub fn start_mind_overlay(overlay: &mut MindOverlay, now_ms: u32, duration_ms: u32) -> bool {
    let registered = overlay.phase == MindPhase::Idle;
    if overlay.phase as u32 & 1 == 0 {
        overlay.phase = MindPhase::FadingIn;
        overlay.end_ms = now_ms.wrapping_add(duration_ms);
    } else {
        overlay.end_ms = overlay.end_ms.wrapping_add(duration_ms);
    }
    if overlay.phase as u32 & 2 == 0 {
        overlay.alpha = 0.0;
    }
    if duration_ms == 0 {
        overlay.end_ms = u32::MAX;
    }
    overlay.stamp_ms = now_ms;
    registered
}

/// Clears the mind sight: the overlay, its end and every body's entries.
pub fn reset_mind_sight(overlay: &mut MindOverlay, sight: &mut MindSight) {
    *overlay = MindOverlay {
        phase: MindPhase::Idle,
        alpha: 0.0,
        stamp_ms: 0,
        end_ms: 0,
    };
    *sight = MindSight::default();
}

/// A mind glow's birth: it starts at full strength and stamps its clock on
/// its first tick.
pub fn mind_glow_add() -> MindGlow {
    MindGlow {
        start_ms: None,
        alpha: 1.0,
    }
}

/// One body offered to the mind-target picker.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindCandidate {
    pub id: u32,
    /// The body's own class word; only class 1 is a character mind-target.
    pub body_type: i32,
    /// The body's effect flags (`0x10` keeps it out of the pick).
    pub effect_flags: u32,
    pub dead: bool,
    /// Whether the player's view can see it; Sense Minds bypasses this.
    pub visible: bool,
    /// The projected centre and its depth, as the view reports them.
    pub screen: [f32; 3],
    /// The node's scale, multiplied into the pick radius.
    pub scale: f32,
}

/// The viewport's side used by the pick radius: `n · scale · 0.5 · 40 /
/// depth`.
pub const PICK_VIEW: f32 = 40.0;

/// Picks the nearest character on screen, within a cone that widens with
/// the field of view and shrinks with depth. `include_dead` lets the dead
/// powers pick a corpse; `sense_active` looks through walls.
pub fn pick_mind_candidate(
    candidates: &[MindCandidate],
    screen_x: f32,
    screen_y: f32,
    n: f32,
    include_dead: bool,
    sense_active: bool,
) -> Option<usize> {
    let mut best: Option<(f32, usize)> = None;
    let mut threshold = 1.0f32;
    for (index, candidate) in candidates.iter().enumerate() {
        if candidate.body_type != 1
            || candidate.effect_flags & 0x10 != 0
            || (candidate.dead && !include_dead)
            || (!sense_active && !candidate.visible)
        {
            continue;
        }
        let radius = n * candidate.scale * 0.5 * PICK_VIEW / candidate.screen[2];
        let dx = candidate.screen[0] - screen_x;
        let dy = candidate.screen[1] - screen_y;
        let distance = (dy * dy + dx * dx).sqrt();
        if distance <= radius {
            let ratio = distance / radius;
            if ratio < threshold {
                best = Some((ratio, index));
                threshold = ratio;
            }
        }
    }
    best.map(|(_, index)| index)
}

/// The scene a mind cast reads: the body it picked and the relation the
/// loyalty power uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MindScene {
    pub now_ms: u32,
    /// The caster's relation to the picked body, for the loyalty powers.
    pub relation: f32,
    /// The caster's own word the mask stores.
    pub caster_word: u32,
}

/// Casts a mind row: the target-requiring casts get the picked body, the
/// self casts ignore it.
pub fn mind_cast(
    caster: &mut MindState,
    target: Option<&mut MindState>,
    row: &MindPowerRow,
    cost: f32,
    scene: MindScene,
) -> MindOutcome {
    match row.cast {
        MindCast::Sense => mind_sense(caster, cost, scene.now_ms),
        MindCast::SensePersistent => mind_sense_persistent(caster, cost),
        MindCast::Mask => mind_mask(caster, cost, scene.caster_word),
        MindCast::MaskPersistent => mind_mask_persistent(caster, cost),
        MindCast::SpiritForm => mind_spirit_form(caster, cost),
        MindCast::Calm => match target {
            Some(target) => mind_calm(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::Enrage => match target {
            Some(target) => mind_enrage(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::Confusion => match target {
            Some(target) => mind_confusion(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::LoyaltyEnd => match target {
            Some(target) => mind_loyalty_end(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::LoyaltyStart => match target {
            Some(target) => mind_loyalty_start(caster, target, cost, scene.relation),
            None => MindOutcome::refused(),
        },
        MindCast::Suspicion => match target {
            Some(target) => mind_suspicion(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::Possession => match target {
            Some(target) => mind_possession(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::CommandDead => match target {
            Some(target) => mind_command_dead(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::Fear => match target {
            Some(target) => mind_fear(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::Embolden => match target {
            Some(target) => mind_embolden(caster, target, cost),
            None => MindOutcome::refused(),
        },
        MindCast::RaiseDead => match target {
            Some(target) => mind_raise_dead(caster, target, cost),
            None => MindOutcome::refused(),
        },
    }
}

#[cfg(test)]
mod cast_dispatch_tests {
    use super::*;

    #[test]
    fn the_screen_overlay_starts_and_extends() {
        let mut overlay = MindOverlay {
            phase: MindPhase::Idle,
            alpha: 0.0,
            stamp_ms: 0,
            end_ms: 0,
        };
        assert!(start_mind_overlay(&mut overlay, 1000, 4000));
        assert_eq!(overlay.phase, MindPhase::FadingIn);
        assert_eq!(overlay.end_ms, 5000);
        assert_eq!(overlay.alpha, 0.0);
        assert!(!start_mind_overlay(&mut overlay, 2000, 4000));
        assert_eq!(overlay.end_ms, 9000);
        assert_eq!(overlay.stamp_ms, 2000);
        let mut forever = MindOverlay {
            phase: MindPhase::Idle,
            alpha: 0.0,
            stamp_ms: 0,
            end_ms: 0,
        };
        start_mind_overlay(&mut forever, 0, 0);
        assert_eq!(forever.end_ms, u32::MAX);
    }

    #[test]
    fn the_mind_pick_takes_the_nearest_visible_character() {
        let candidates = [
            MindCandidate {
                id: 4,
                body_type: 1,
                effect_flags: 0,
                dead: false,
                visible: false,
                screen: [4.5, 0.0, 10.0],
                scale: 1.0,
            },
            MindCandidate {
                id: 5,
                body_type: 1,
                effect_flags: 0,
                dead: false,
                visible: true,
                screen: [6.0, 0.0, 10.0],
                scale: 1.0,
            },
            MindCandidate {
                id: 6,
                body_type: 2,
                effect_flags: 0,
                dead: false,
                visible: true,
                screen: [0.0, 0.0, 10.0],
                scale: 1.0,
            },
        ];
        assert_eq!(
            pick_mind_candidate(&candidates, 5.0, 0.0, 1.0, false, false),
            Some(1)
        );
        assert_eq!(
            pick_mind_candidate(&candidates, 5.0, 0.0, 1.0, false, true),
            Some(0)
        );
        assert_eq!(
            pick_mind_candidate(&candidates, 15.0, 0.0, 1.0, false, true),
            None
        );
    }

    #[test]
    fn the_dead_pick_takes_a_corpse_and_the_pick_ignores_ghosts() {
        let corpse = MindCandidate {
            id: 7,
            body_type: 1,
            effect_flags: 0,
            dead: true,
            visible: true,
            screen: [0.0, 0.0, 10.0],
            scale: 1.0,
        };
        assert_eq!(
            pick_mind_candidate(&[corpse], 0.0, 0.0, 1.0, false, false),
            None
        );
        assert_eq!(
            pick_mind_candidate(&[corpse], 0.0, 0.0, 1.0, true, false),
            Some(0)
        );
        let hidden = MindCandidate {
            effect_flags: 0x10,
            ..corpse
        };
        assert_eq!(
            pick_mind_candidate(&[hidden], 0.0, 0.0, 1.0, true, true),
            None
        );
    }

    #[test]
    fn a_row_casts_on_its_target_or_on_the_caster() {
        let mut caster = MindState::new(1, 1.0, 0);
        caster.power_count = 1;
        let mut target = MindState::new(7, 1.0, 2);
        let row = mind_row(0x104).expect("enrage");
        let scene = MindScene {
            now_ms: 0,
            relation: 0.0,
            caster_word: 0,
        };
        let outcome = mind_cast(&mut caster, Some(&mut target), row, row.cost, scene);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert_eq!(target.aggression, 50.0);
        let sense = mind_row(0x101).expect("sense");
        let outcome = mind_cast(&mut caster, None, sense, sense.cost, scene);
        assert_eq!(outcome.code, MIND_APPLIED);
        assert!(outcome.overlay_ms.is_some());
        let refused = mind_cast(&mut caster, None, row, row.cost, scene);
        assert_eq!(refused.code, MIND_REFUSED);
    }
}

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

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

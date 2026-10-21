//! The worker unit's weighted three-plane sum, which the original's
//! image-noise generator calls: three float planes combined into an output
//! plane with three weights.
//!
//! The original walks `width × height` samples and computes
//! `plane0·w0 + plane1·w1 + plane2·w2` in single precision, adding the first
//! two products before the third.
//! It fills the last sample too (the loop counter starts at −1 and is
//! pre-incremented), which is what the slice loop here does.

/// Sums three float planes into `out` with weights `weights`, over the
/// shortest of the four lengths (the original's callers size all four from
/// the same width and height). Nothing calls it yet: the image-noise
/// generator that uses it in the original is not ported.
pub fn weighted_planes(planes: [&[f32]; 3], out: &mut [f32], weights: [f32; 3]) {
    let samples = out
        .iter_mut()
        .zip(planes[0].iter().zip(planes[1]).zip(planes[2]));
    for (out, ((&a, &b), &c)) in samples {
        *out = a * weights[0] + b * weights[1] + c * weights[2];
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_three_planes_with_the_original_order() {
        let a = [1.0f32, 2.0, 3.0];
        let b = [10.0f32, 20.0, 30.0];
        let c = [100.0f32, 200.0, 300.0];
        let mut out = [0.0f32; 3];
        weighted_planes([&a, &b, &c], &mut out, [0.5, 0.25, 0.125]);
        assert_eq!(out, [0.5 + 2.5 + 12.5, 1.0 + 5.0 + 25.0, 1.5 + 7.5 + 37.5]);
    }

    #[test]
    fn an_empty_plane_writes_nothing_and_a_short_plane_stops_the_sum() {
        let mut out: [f32; 0] = [];
        weighted_planes([&[], &[], &[]], &mut out, [1.0, 0.0, 0.0]);
        assert_eq!(out, []);
        let mut two = [9.0f32; 2];
        weighted_planes(
            [&[1.0, 1.0], &[2.0], &[3.0, 3.0]],
            &mut two,
            [1.0, 1.0, 1.0],
        );
        assert_eq!(two, [6.0, 9.0]);
    }
}

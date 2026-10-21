//! The worker unit's weighted three-plane sum, used by the image-noise
//! generator: one float plane per RGB channel (or per sample set), combined
//! into an output plane with three weights.
//!
//! The original walks `width × height` samples and computes
//! `plane0·w0 + plane1·w1 + plane2·w2` in single precision, in that order.
//! It fills the last sample too (the loop counter starts at −1 and is
//! pre-incremented), which is what the slice loop here does.

/// Sums three equally long float planes into `out` with weights `weights`.
///
/// # Panics
///
/// Panics when the planes or `out` have different lengths. The caller sizes
/// them from the same width and height, as the original does.
pub fn weighted_planes(planes: [&[f32]; 3], out: &mut [f32], weights: [f32; 3]) {
    assert_eq!(out.len(), planes[0].len(), "plane 0 length");
    assert_eq!(out.len(), planes[1].len(), "plane 1 length");
    assert_eq!(out.len(), planes[2].len(), "plane 2 length");
    for i in 0..out.len() {
        out[i] = planes[0][i] * weights[0] + planes[1][i] * weights[1] + planes[2][i] * weights[2];
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
    fn an_empty_plane_writes_nothing_and_mismatches_are_caught() {
        let mut out: [f32; 0] = [];
        weighted_planes([&[], &[], &[]], &mut out, [1.0, 0.0, 0.0]);
        assert_eq!(out, []);
        let mut one = [0.0f32];
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            weighted_planes([&[1.0], &[], &[]], &mut one, [1.0, 1.0, 1.0]);
        }));
        assert!(result.is_err());
    }
}

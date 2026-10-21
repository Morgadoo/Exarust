//! The parameters of the GI sample-point generator, as the original's
//! voxeliser object holds them: a 40 cm lattice spacing, the 4.0 and 0.4
//! tuning values and one work pass. The generator itself builds the lattice
//! and its work buffers; the sample points it produces are the level's GI
//! points (`exa_formats::rfc`, the GI block in `gi_volume`).
//!
//! Units are the original's: centimetres.

/// The lattice spacing and tuning values of the GI sample-point generator.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voxeliser {
    /// Lattice spacing, centimetres.
    pub spacing: f32,
    /// First tuning value.
    pub tuning: f32,
    /// Second tuning value.
    pub tolerance: f32,
    /// Work passes (the constructor stores 1).
    pub passes: i32,
}

impl Default for Voxeliser {
    /// The constructor's values: spacing 40, tuning 4.0 and 0.4, one pass.
    fn default() -> Self {
        Self {
            spacing: 40.0,
            tuning: 4.0,
            tolerance: 0.4,
            passes: 1,
        }
    }
}

impl Voxeliser {
    /// The lattice spacing in metres.
    pub fn spacing_metres(&self) -> f32 {
        self.spacing / 100.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_voxeliser_starts_with_the_original_numbers() {
        let v = Voxeliser::default();
        assert_eq!(v.spacing, 40.0);
        assert_eq!(v.tuning, 4.0);
        assert_eq!(v.tolerance, 0.4);
        assert_eq!(v.passes, 1);
        assert_eq!(v.spacing_metres(), 0.4);
    }
}

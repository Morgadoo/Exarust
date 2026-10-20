//! The dynamics volume's voxel summary (the original's
//! `RF_Dynamics_Volumes` helpers): a float-keyed quicksort and the mass and
//! centre of a padded voxel count grid.

/// A volume's centre and mass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumeSummary {
    pub centre: [f32; 3],
    pub mass: f32,
}

/// The mass returned for a volume with no filled cell (0.1).
pub const EMPTY_MASS: f32 = 0.1;

/// Sorts entries by their float key with the original's two-way quicksort:
/// a middle pivot, values below it from the left and above it from the
/// right, then both sides.
pub fn sort_by_key(entries: &mut [(f32, u32)]) {
    if entries.len() > 1 {
        quicksort(entries, 0, entries.len() as isize - 1);
    }
}

fn quicksort(entries: &mut [(f32, u32)], low: isize, high: isize) {
    let mut i = low;
    let mut j = high;
    let pivot = entries[((low + high) / 2) as usize].0;
    loop {
        while entries[i as usize].0 < pivot {
            i += 1;
        }
        while entries[j as usize].0 > pivot {
            j -= 1;
        }
        if i <= j {
            entries.swap(i as usize, j as usize);
            i += 1;
            j -= 1;
        }
        if i > j {
            break;
        }
    }
    if low < j {
        quicksort(entries, low, j);
    }
    if i < high {
        quicksort(entries, i, high);
    }
}

/// Summarises a voxel volume. `dims` is the unpadded cell count per axis;
/// the count grid is padded by one cell on each side, so it holds
/// `(dims + 2)` cells per axis. Each filled cell contributes its centre
/// `(i, j, k) - 0.5` to the centre sum and one to the count; the centre is
/// `(sum/count/dim - 0.5)·scale` and the mass is the count times the cell
/// volume `Π(2·scale/dim)`.
pub fn summarise(dims: [i32; 3], scale: [f32; 3], counts: &[i32]) -> VolumeSummary {
    let stride_y = dims[1] + 2;
    let stride_z = dims[2] + 2;
    let mut sum = [0.0f32; 3];
    let mut filled = 0i32;
    for i in 0..=dims[0] {
        for j in 0..=dims[1] {
            for k in 0..=dims[2] {
                let index = ((i * stride_y + j) * stride_z + k) as usize;
                if counts.get(index).copied().unwrap_or(0) > 0 {
                    sum[0] += i as f32 - 0.5;
                    sum[1] += j as f32 - 0.5;
                    sum[2] += k as f32 - 0.5;
                    filled += 1;
                }
            }
        }
    }
    if filled == 0 {
        return VolumeSummary {
            centre: [0.0; 3],
            mass: EMPTY_MASS,
        };
    }
    let count = filled as f32;
    let centre = [
        (sum[0] / count / dims[0] as f32 - 0.5) * scale[0],
        (sum[1] / count / dims[1] as f32 - 0.5) * scale[1],
        (sum[2] / count / dims[2] as f32 - 0.5) * scale[2],
    ];
    let cell = (scale[0] * 2.0 / dims[0] as f32)
        * (scale[1] * 2.0 / dims[1] as f32)
        * (scale[2] * 2.0 / dims[2] as f32);
    VolumeSummary {
        centre,
        mass: count * cell,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entries_sort_by_key() {
        let mut entries = [(3.0, 0), (1.0, 1), (2.0, 2), (1.0, 3)];
        sort_by_key(&mut entries);
        let keys: Vec<f32> = entries.iter().map(|(key, _)| *key).collect();
        assert_eq!(keys, vec![1.0, 1.0, 2.0, 3.0]);
        assert!(entries.iter().any(|(_, value)| *value == 3));
    }

    #[test]
    fn a_unit_volume_summarises_its_centre_and_mass() {
        let mut counts = vec![0; 27];
        counts[13] = 1;
        let summary = summarise([1, 1, 1], [1.0, 1.0, 1.0], &counts);
        assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
        assert!((summary.mass - 8.0).abs() < 1e-6);
    }

    #[test]
    fn an_empty_volume_has_the_fallback_mass() {
        let summary = summarise([1, 1, 1], [1.0, 1.0, 1.0], &[0; 27]);
        assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
        assert_eq!(summary.mass, EMPTY_MASS);
    }

    #[test]
    fn the_centre_follows_the_filled_cell() {
        let mut counts = vec![0; 27];
        counts[13] = 1;
        let summary = summarise([1, 1, 1], [2.0, 4.0, 6.0], &counts);
        assert_eq!(summary.centre, [0.0, 0.0, 0.0]);
        assert!((summary.mass - 8.0 * 48.0).abs() < 1e-5);
    }
}

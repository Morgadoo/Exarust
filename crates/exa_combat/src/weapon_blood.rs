//! The weapon's blood list, as the original keeps it (`TWeaponBlood`):
//! one spot for each source that has drawn blood on the weapon. A spot
//! stores the source, how much blood it has left (0 through 1), and the
//! two terms the stain needs from the weapon's length: the length over 80
//! and the inverse square root of 80 over the length. The draw
//! (`exa_render::stain`) uses the amount; this is the original's own list.
//!
//! - **Adding a source** ([`WeaponBlood::add`]): a source carrying
//!   `0x400000` adds nothing and returns 0. Otherwise the source takes the
//!   first slot whose source is gone, or the list grows by one. Its two
//!   lengths sum, the scale is that sum × 0.0125 (in double precision) and
//!   the inverse is the square root of 80 over the sum (single precision).
//!   The amount and the tag start at 0. The return is the 1-based index.
//! - **Releasing a part's spot** ([`release_part`]): a body that is not a
//!   limb (a held weapon) and still holds a 1-based slot releases it and
//!   takes 0; a limb's slot is left alone.
//! - **Clearing the count** ([`WeaponBlood::clear_count`]) drops every spot;
//!   the array memory is reused on the next add.
//! - **Clearing a spot** ([`WeaponBlood::clear`]) drops its source, amount
//!   and scale; the slot is then free.
//! - **Adding blood** ([`WeaponBlood::add_amount`]) raises the spot's amount
//!   by the delta and keeps it within 0 through 1.
//!
//! Rules: `docs/RENDERING.md` (Blood on weapons).

/// The stain's span in centimetres (the original's float 80.0).
pub const SPOT_SPAN: f32 = 80.0;
/// One over the span, as the original uses it (its double 0.0125).
pub const SPOT_INV_SPAN: f64 = 0.0125;

/// One source's spot in the weapon's blood list.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WeaponBloodSpot {
    /// The source (the original stores a pointer; 0 is free).
    pub source: Option<usize>,
    /// The blood this source has left, 0 through 1.
    pub amount: f32,
    /// The source's two lengths over 80 (the stain's offset span).
    pub scale: f32,
    /// The square root of 80 over the source's two lengths.
    pub inv: f32,
    /// The original's spare word at `0x14`.
    pub tag: i32,
}

/// The weapon's blood list.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct WeaponBlood {
    pub spots: Vec<WeaponBloodSpot>,
}

impl WeaponBlood {
    /// Drops every spot (the original's count at `0x8` set to 0).
    pub fn clear_count(&mut self) {
        self.spots.clear();
    }

    /// Adds a source with two lengths (the weapon's two halves). A hidden
    /// source (bit `0x400000`) adds nothing and returns 0. Returns the
    /// spot's 1-based index, as the original does.
    pub fn add(&mut self, source: usize, length_a: f32, length_b: f32, hidden: bool) -> usize {
        if hidden {
            return 0;
        }
        let length = length_a + length_b;
        let scale = (f64::from(length) * SPOT_INV_SPAN) as f32;
        let inv = (SPOT_SPAN / length).sqrt();
        let index = match self.spots.iter().position(|spot| spot.source.is_none()) {
            Some(index) => index,
            None => {
                self.spots.push(WeaponBloodSpot::default());
                self.spots.len() - 1
            }
        };
        let spot = &mut self.spots[index];
        spot.source = Some(source);
        spot.amount = 0.0;
        spot.scale = scale;
        spot.inv = inv;
        spot.tag = 0;
        index + 1
    }

    /// Clears the spot at the original's 1-based `index`: its source,
    /// amount and scale.
    pub fn clear(&mut self, index: usize) {
        if let Some(spot) = index.checked_sub(1).and_then(|i| self.spots.get_mut(i)) {
            spot.source = None;
            spot.amount = 0.0;
            spot.scale = 0.0;
        }
    }

    /// Releases the spot a part leaves the world with: only a body that is
    /// not a limb (a held weapon) releases a 1-based `slot`, and the part's
    /// slot becomes 0. A limb keeps its slot.
    pub fn release_part(&mut self, part_is_limb: bool, slot: usize) -> usize {
        if !part_is_limb && slot > 0 {
            self.clear(slot);
            0
        } else {
            slot
        }
    }

    /// Adds blood to the spot at the original's 1-based `index`, keeping the
    /// amount within 0 through 1.
    pub fn add_amount(&mut self, index: usize, delta: f32) {
        if let Some(spot) = index.checked_sub(1).and_then(|i| self.spots.get_mut(i)) {
            spot.amount = (spot.amount + delta).clamp(0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_source_takes_a_spot_with_its_stain_terms() {
        let mut blood = WeaponBlood::default();
        assert_eq!(blood.add(7, 40.0, 40.0, false), 1);
        let spot = blood.spots[0];
        assert_eq!(spot.source, Some(7));
        assert_eq!(spot.amount, 0.0);
        assert_eq!(spot.scale, 1.0);
        assert_eq!(spot.inv, 1.0);
        assert_eq!(spot.tag, 0);

        assert_eq!(blood.add(8, 10.0, 30.0, false), 2);
        let spot = blood.spots[1];
        assert_eq!(spot.scale, 0.5);
        assert!((spot.inv - 2.0f32.sqrt()).abs() < 1e-6);
    }

    #[test]
    fn a_hidden_source_adds_nothing() {
        let mut blood = WeaponBlood::default();
        assert_eq!(blood.add(7, 40.0, 40.0, true), 0);
        assert!(blood.spots.is_empty());
    }

    #[test]
    fn a_cleared_spot_is_reused_and_counts_clamp() {
        let mut blood = WeaponBlood::default();
        blood.add(7, 40.0, 40.0, false);
        blood.add(8, 40.0, 40.0, false);

        blood.add_amount(1, 0.4);
        assert!((blood.spots[0].amount - 0.4).abs() < 1e-6);
        blood.add_amount(1, 0.8);
        assert_eq!(blood.spots[0].amount, 1.0);
        blood.add_amount(1, -2.0);
        assert_eq!(blood.spots[0].amount, 0.0);

        blood.clear(1);
        assert_eq!(blood.spots[0].source, None);
        assert_eq!(blood.spots[0].scale, 0.0);
        assert_eq!(blood.add(9, 40.0, 40.0, false), 1);
        assert_eq!(blood.spots.len(), 2);

        blood.clear_count();
        assert!(blood.spots.is_empty());
    }

    #[test]
    fn a_leaving_weapon_releases_its_spot_but_a_limb_does_not() {
        let mut blood = WeaponBlood::default();
        blood.add(7, 40.0, 40.0, false);
        assert_eq!(blood.release_part(true, 1), 1);
        assert_eq!(blood.spots[0].source, Some(7));
        assert_eq!(blood.release_part(false, 1), 0);
        assert_eq!(blood.spots[0].source, None);
        // A part with no slot keeps 0.
        assert_eq!(blood.release_part(false, 0), 0);
    }
}

//! The weapons' blood list, as the original keeps it (`TWeaponBlood`, one
//! for the world): one spot for each weapon that has drawn blood. A spot
//! stores the weapon, how much blood it has left (0 through 1), and the
//! two terms the stain needs from the weapon's length: the length over 80
//! and the inverse square root of 80 over the length. The draw
//! (`exa_render::stain`) uses the amount. The running game keeps the list
//! as a resource: a weapon's first bleeding wound registers it, each wound
//! raises its spot, and the weapon mirrors its spot's amount and index in
//! [`crate::wounds::BladeBlood`] ([`crate::wounds::BladeBlood::draw_listed`]).
//!
//! - **World reset** ([`clear_at_world_reset`]): the original's world reset
//!   (a new session, a save load, an arena round's start or end, a scripted
//!   jump, a quit) drops every spot, and the bodies it then rebuilds hold
//!   no spot and no blood. Exarust keeps the player's body and possessions
//!   across it, so it also clears each surviving weapon's blood and spot.
//!   A door's level change keeps the list, the weapon and its stain.
//! - **A weapon that leaves the world** ([`release_gone_blades`]) releases
//!   its spot, as the original's item body does when its node is destroyed.
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
//!   by the delta and keeps it within 0 through 1 ([`add_blood`]).
//!
//! Rules: `docs/RENDERING.md` (Blood on weapons).

use bevy::prelude::*;
use exa_world::level_lifecycle::WorldReset;

use crate::wounds::BladeBlood;

/// The stain's span in centimetres (the original's float 80.0).
pub const SPOT_SPAN: f32 = 80.0;
/// One over the span, as the original uses it (its double 0.0125).
pub const SPOT_INV_SPAN: f64 = 0.0125;

/// The blood `amount` holds after `delta` more, as the original keeps it:
/// the sum, then 1 when 1 is below it, else 0 when 0 is above it. A NaN
/// stays NaN and −0 stays −0 (both compares fail for them).
pub fn add_blood(amount: f32, delta: f32) -> f32 {
    // `clamp` makes the same two compares (strictly below 0, strictly
    // above 1) and lets NaN and −0 through, as the original does.
    (amount + delta).clamp(0.0, 1.0)
}

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

/// The weapons' blood list.
#[derive(Resource, Debug, Clone, Default, PartialEq)]
pub struct WeaponBlood {
    pub spots: Vec<WeaponBloodSpot>,
}

impl WeaponBlood {
    /// The amount of the spot at the original's 1-based `index`, none for
    /// an index the list does not hold.
    pub fn amount(&self, index: usize) -> Option<f32> {
        index
            .checked_sub(1)
            .and_then(|i| self.spots.get(i))
            .map(|spot| spot.amount)
    }

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
            spot.amount = add_blood(spot.amount, delta);
        }
    }
}

/// The world reset drops every spot and, since Exarust keeps the bodies
/// the original rebuilds clean, every weapon's blood and spot.
pub fn clear_at_world_reset(
    mut resets: MessageReader<WorldReset>,
    mut list: ResMut<WeaponBlood>,
    mut blades: Query<(Entity, &mut BladeBlood)>,
) {
    if resets.read().count() == 0 {
        return;
    }
    let spots = list.spots.len();
    list.clear_count();
    let mut cleared = 0;
    for (blade, mut blood) in &mut blades {
        if *blood != BladeBlood::default() {
            info!(
                "world reset: weapon {blade:?} drops its blood {:.3} (spot {})",
                blood.0, blood.1
            );
            *blood = BladeBlood::default();
            cleared += 1;
        }
    }
    info!("world reset: the weapons' blood list drops {spots} spots, {cleared} weapons cleaned");
}

/// A weapon that leaves the world releases its spot: the list's spot whose
/// source it is goes free (a body that is not a limb).
pub fn release_gone_blades(mut gone: RemovedComponents<BladeBlood>, mut list: ResMut<WeaponBlood>) {
    for blade in gone.read() {
        let source = blade.to_bits() as usize;
        if let Some(index) = list.spots.iter().position(|s| s.source == Some(source)) {
            list.release_part(false, index + 1);
            debug!(
                "weapon {blade:?} left the world: blood spot {} released",
                index + 1
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A weapon's first blood registers it; a second wound raises the same
    /// spot; another weapon takes the next spot. A world reset drops the
    /// list and cleans both weapons, and the next blood registers afresh.
    /// A despawned weapon frees its spot for the next.
    #[test]
    fn struck_blades_are_listed_and_cleared_at_a_world_reset() {
        let mut app = App::new();
        app.init_resource::<WeaponBlood>()
            .add_message::<WorldReset>()
            .add_systems(Update, (clear_at_world_reset, release_gone_blades));
        let a = app.world_mut().spawn(BladeBlood::default()).id();
        let b = app.world_mut().spawn(BladeBlood::default()).id();
        let draw = |app: &mut App, blade: Entity, size: f32| {
            app.world_mut()
                .resource_scope(|world, mut list: Mut<WeaponBlood>| {
                    let mut blood = world.get_mut::<BladeBlood>(blade).unwrap();
                    blood.draw_listed(&mut list, blade, 80.0, size);
                });
        };
        draw(&mut app, a, 0.4);
        draw(&mut app, a, 0.4);
        draw(&mut app, b, 0.2);
        let blood = |app: &App, e: Entity| *app.world().get::<BladeBlood>(e).unwrap();
        assert_eq!(blood(&app, a), BladeBlood(0.4, 1));
        assert_eq!(blood(&app, b), BladeBlood(0.1, 2));
        let list = app.world().resource::<WeaponBlood>();
        assert_eq!(list.spots[0].source, Some(a.to_bits() as usize));
        assert_eq!(list.spots[0].scale, 1.0);
        assert_eq!(list.amount(2), Some(0.1));

        app.world_mut().write_message(WorldReset);
        app.update();
        assert!(app.world().resource::<WeaponBlood>().spots.is_empty());
        assert_eq!(blood(&app, a), BladeBlood::default());
        assert_eq!(blood(&app, b), BladeBlood::default());
        draw(&mut app, b, 0.2);
        assert_eq!(blood(&app, b), BladeBlood(0.1, 1));

        draw(&mut app, a, 0.2);
        app.world_mut().despawn(b);
        app.update();
        let list = app.world().resource::<WeaponBlood>();
        assert_eq!(list.spots[0].source, None);
        assert_eq!(list.spots[1].source, Some(a.to_bits() as usize));
        let c = app.world_mut().spawn(BladeBlood::default()).id();
        draw(&mut app, c, 0.2);
        assert_eq!(blood(&app, c).1, 1);
    }

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

    /// The sum is kept within 0 through 1; NaN and −0 pass through.
    #[test]
    fn blood_is_kept_within_zero_and_one() {
        assert_eq!(add_blood(0.75, 0.2285), 0.978_5);
        assert_eq!(add_blood(0.9, 0.2285), 1.0);
        assert_eq!(add_blood(0.1, -0.3), 0.0);
        assert!(add_blood(0.2, f32::NAN).is_nan());
        let z = add_blood(-0.0, -0.0);
        assert!(z == 0.0 && z.is_sign_negative());
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

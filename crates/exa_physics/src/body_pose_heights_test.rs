//! The samplers' heights, pinned to the original's numbers.
//! (Included from body_pose_heights.rs.)
use super::*;

fn aim() -> PelvisAim {
    PelvisAim {
        walking: false,
        off_feet: false,
        getting_up: false,
        unconscious: false,
        run: 0.0,
        scale: 1.0,
        stepping: false,
        crouch: 0.0,
        progress: 1.0,
        factor: 1.0,
    }
}

/// The stand height is the root over the lowest posed contact less the
/// sole; the reach the root less half the clip heights' sum.
#[test]
fn the_stand_height_is_the_root_over_the_lowest_contact() {
    let contacts = [
        ContactHeight {
            clip: 8.0,
            posed: 9.0,
        },
        ContactHeight {
            clip: 12.0,
            posed: 7.0,
        },
    ];
    let h = stand_heights(100.0, &contacts, 4.5, false);
    assert_eq!(h.stand, 100.0 - (7.0 - 4.5));
    assert_eq!(h.reach, 100.0 - 20.0 * 0.5);
    // No contacts: the lowest stays at 5000, over the root, so 0.
    let none = stand_heights(100.0, &[], 4.5, false);
    assert_eq!(none.stand, 0.0);
    assert_eq!(none.reach, 100.0);
    // A lowest at the root gives 0, just under it the difference.
    let at = [ContactHeight {
        clip: 0.0,
        posed: 104.5,
    }];
    assert_eq!(stand_heights(100.0, &at, 4.5, false).stand, 0.0);
}

/// A NaN contact is never the lowest; a NaN root subtracts.
#[test]
fn a_nan_contact_is_skipped_and_a_nan_root_subtracts() {
    let contacts = [
        ContactHeight {
            clip: 1.0,
            posed: f32::NAN,
        },
        ContactHeight {
            clip: 1.0,
            posed: 10.0,
        },
    ];
    assert_eq!(stand_heights(100.0, &contacts, 0.0, false).stand, 90.0);
    assert!(
        stand_heights(f32::NAN, &contacts, 0.0, false)
            .stand
            .is_nan()
    );
}

/// Hovering, both heights are the root less 10.
#[test]
fn a_hovering_body_stands_ten_under_its_root() {
    let h = stand_heights(
        80.0,
        &[ContactHeight {
            clip: 1.0,
            posed: 1.0,
        }],
        4.0,
        true,
    );
    assert_eq!(
        h,
        StandHeights {
            stand: 70.0,
            reach: 70.0
        }
    );
}

/// The placement's walking target adds the pelvis's posed height and
/// 3 − 3·run; otherwise the margin (2·scale, 5 when not stepping),
/// shortened when crouched, and the Unconscious clip drops the stand.
#[test]
fn the_placement_target_follows_the_condition() {
    let walking = PelvisAim {
        walking: true,
        run: 0.5,
        ..aim()
    };
    assert_eq!(
        placement_pelvis_target(95.0, 100.0, &walking),
        (3.0 - 1.5) + (95.0 + 100.0)
    );
    assert_eq!(
        placement_pelvis_target(95.0, 100.0, &aim()),
        95.0 + 2.0 + 5.0
    );
    let stepping = PelvisAim {
        stepping: true,
        scale: 1.2,
        ..aim()
    };
    assert_eq!(placement_pelvis_target(95.0, 0.0, &stepping), 95.0 + 2.4);
    let low = PelvisAim {
        crouch: 1.0,
        ..aim()
    };
    assert_eq!(
        placement_pelvis_target(95.0, 0.0, &low),
        (f64::from(102.0f32) * 0.7) as f32
    );
    // The crouch counts only on the feet.
    let lying = PelvisAim {
        off_feet: true,
        ..low
    };
    assert_eq!(placement_pelvis_target(95.0, 0.0, &lying), 102.0);
    let out = PelvisAim {
        unconscious: true,
        factor: 2.0,
        ..aim()
    };
    assert_eq!(placement_pelvis_target(95.0, 0.0, &out), 14.0);
}

/// The crouch compares in double: the float 0.95 is over the double 0.95.
#[test]
fn the_crouch_compares_in_double() {
    let edge = PelvisAim {
        crouch: 0.95,
        ..aim()
    };
    // 0.95f32 widens to 0.949999988…, under the double 0.95.
    assert_eq!(placement_pelvis_target(93.0, 0.0, &edge), 100.0);
    let over = PelvisAim {
        crouch: f32::from_bits(0.95f32.to_bits() + 1),
        ..aim()
    };
    assert_eq!(placement_pelvis_target(93.0, 0.0, &over), 70.0);
}

/// The living target eases: walking by 0.1 + 0.1·run toward stand +
/// 2·run + 3, otherwise by 0.1 toward the margin; crouched it is set.
#[test]
fn the_living_target_eases_toward_the_margin() {
    let walking = PelvisAim {
        walking: true,
        run: 0.5,
        ..aim()
    };
    let r = 0.5f64 * 0.1;
    let want = ((0.1 + r) * f64::from(95.0f32 + 4.0) + (0.9 - r) * 80.0) as f32;
    assert_eq!(living_pelvis_target(80.0, 95.0, None, &walking), want);
    let want = (f64::from(102.0f32) * 0.1 + 80.0 * 0.9) as f32;
    assert_eq!(living_pelvis_target(80.0, 95.0, None, &aim()), want);
    let low = PelvisAim {
        crouch: 1.0,
        stepping: true,
        ..aim()
    };
    assert_eq!(
        living_pelvis_target(80.0, 95.0, None, &low),
        (f64::from(97.0f32) * 0.8) as f32
    );
    // Hovering: the root less 10, times the factor.
    let hover = PelvisAim {
        factor: 0.5,
        ..aim()
    };
    assert_eq!(living_pelvis_target(80.0, 95.0, Some(110.0), &hover), 50.0);
}

/// A get-up under a fifth of its clip starts the target from nothing.
#[test]
fn an_early_get_up_clears_the_living_target() {
    let rising = PelvisAim {
        getting_up: true,
        off_feet: true,
        progress: 0.1,
        ..aim()
    };
    assert_eq!(
        living_pelvis_target(80.0, 95.0, None, &rising),
        (f64::from(7.0f32) * 0.1) as f32
    );
    let later = PelvisAim {
        progress: 0.2,
        ..rising
    };
    // 0.2f32 widens over the double 0.2: not early.
    assert_eq!(
        living_pelvis_target(80.0, 95.0, None, &later),
        (f64::from(102.0f32) * 0.1 + 80.0 * 0.9) as f32
    );
    let nan = PelvisAim {
        progress: f32::NAN,
        ..rising
    };
    assert_eq!(
        living_pelvis_target(80.0, 95.0, None, &nan),
        (f64::from(102.0f32) * 0.1 + 80.0 * 0.9) as f32
    );
}

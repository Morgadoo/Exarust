//! The pace of a swing, as the original's attack routine sets it: an
//! attack's clip clock runs at a rate its weapon gives, slower at the ends
//! of the swing and faster mid-swing, and the attack ends into the stance
//! with a blend of twice the weapon's recovery. A fighter's [`Swing`] is
//! its held weapon's, its fists' when it holds none, read from the weapon
//! record's swing speed (W15), swing shaping (W18) and stance blend (W19).
//! Rules: `docs/formats/RSQ.md` (Motion playback).

use bevy::prelude::*;

use crate::Blade;
use crate::attack::{Attacker, Slot};
use crate::damage::WeaponRecord;

/// The swing's rate: speed × g × 1.125 × the clip clock's base, g = 1 −
/// 0.833·modulation·(d + d²), d = |progress − 0.5|.
const SHAPE: f32 = 0.833;
const MID_SWING: f32 = 0.5;
const RATE: f32 = 1.125;
/// A stab is faster: g ×1.2, and early in it (progress under 0.4) also ×
/// (1.2·progress + 0.5).
const STAB_SPEED: f32 = 1.2;
const STAB_EARLY: f32 = 0.4;
const STAB_RAMP: f32 = 1.2;
const STAB_FROM: f32 = 0.5;
/// Under this share of its most energy a fighter's swing stops; under its
/// most the rate becomes the share squared.
const SWING_ENERGY: f32 = 0.8;
/// The stab's action index.
const STAB: usize = 0x0B;

/// The pace of a fighter's swings.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Swing {
    /// The weapon's swing speed (W15).
    pub speed: f32,
    /// How much slower the swing is at its ends: the weapon's swing shaping
    /// (W18; 1 − its balance unencumbered).
    pub modulation: f32,
    /// The weapon's stance blend (W19, frames): an attack ends into the
    /// stance with twice it as blend.
    pub recovery: u32,
}

impl Swing {
    /// A human's fists: weightless and balanced, so fast and even, with
    /// no recovery (their record's swing speed is (0.15 + 1.05) × 1.2).
    pub const FISTS: Swing = Swing {
        speed: 1.44,
        modulation: 0.0,
        recovery: 0,
    };

    /// The pace of a weapon, from its record as held: its swing speed
    /// (W15), its swing shaping (W18) and its stance blend (W19).
    pub fn of(record: &WeaponRecord) -> Swing {
        Swing {
            speed: record.swing_speed,
            modulation: record.swing_shaping,
            recovery: record.blend.max(0) as u32,
        }
    }

    /// Frames an attack's clip clock advances this tick, at the original's
    /// `progress` (clock less blend over the last key's time) of the clip
    /// `action`, with the clock's `base` rate and `energy` the fighter's
    /// share of its most: speed × g × 1.125 × base, or nothing under 0.8
    /// of its energy and the share squared under all of it.
    pub fn frames(&self, action: Option<usize>, progress: f32, base: f32, energy: f32) -> f32 {
        let d = (progress - MID_SWING).abs();
        let mut g = if progress <= 0.0 {
            1.0 - SHAPE * self.modulation
        } else {
            1.0 - SHAPE * self.modulation * (d + d * d)
        };
        if action == Some(STAB) {
            if progress < STAB_EARLY {
                g *= STAB_RAMP * progress + STAB_FROM;
            }
            g *= STAB_SPEED;
        }
        let rate = self.speed * g * RATE * base;
        if energy < SWING_ENERGY {
            0.0
        } else if energy < 1.0 {
            energy * energy
        } else {
            rate
        }
    }
}

/// Gives each wielder the swing of its primary striker (once a frame; the
/// game adds it beside the character ticks that read it): the blade in
/// the held slot its last attack or its equip made primary, the first
/// before any (an unarmed fighter's right fist).
pub fn pace_swings(
    mut commands: Commands,
    blades: Query<&Blade>,
    attackers: Query<&Attacker>,
    mut swings: Query<&mut Swing>,
) {
    for blade in &blades {
        let primary = attackers
            .get(blade.wielder)
            .ok()
            .and_then(|a| a.primary)
            .unwrap_or(Slot::First);
        if blade.slot != primary {
            continue;
        }
        let swing = Swing::of(&blade.record);
        match swings.get_mut(blade.wielder) {
            Ok(mut s) => {
                if *s != swing {
                    *s = swing;
                }
            }
            Err(_) => {
                if let Ok(mut e) = commands.get_entity(blade.wielder) {
                    e.insert(swing);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::weapon_record::{Holder, Lengths, Section};
    use exa_formats::weapon_stats::{StatBytes, at};

    /// A sword weighing 0.3 with balance 0.6: speed 1.155, modulation 0.4,
    /// recovery round(7) = 7 (a 14-frame blend into the stance); the swing
    /// runs 0.87 frames a tick at its start (progress 0), 0.97 at its end,
    /// 1.30 mid-swing.
    #[test]
    fn a_sword_paces_its_swing() {
        let mut stats = StatBytes::default();
        stats.set(at::WEIGHT, 30);
        stats.set(at::BALANCE, 60);
        let sword = WeaponRecord::of_one_part(0x11, &stats, false, 90.0);
        let swing = Swing::of(&sword);
        assert!((swing.speed - 1.155).abs() < 1e-5);
        assert!((swing.modulation - 0.4).abs() < 1e-5);
        assert_eq!(swing.recovery, 7);
        let at = |p: f32| swing.frames(Some(5), p, 1.0, 1.0);
        assert!((at(0.0) - 1.155 * (1.0 - 0.833 * 0.4) * 1.125).abs() < 1e-5);
        assert!((at(0.0) - 0.8664).abs() < 1e-3);
        assert!((at(1.0) - 0.9746).abs() < 1e-3);
        assert!((at(0.5) - 1.2994).abs() < 1e-3);
        // d = 0.25: 1 − 0.833·0.4·0.3125.
        assert!((at(0.25) - 1.155 * 0.895_875 * 1.125).abs() < 1e-5);
    }

    /// Fists: speed 1.44, an even swing at 1.62 frames a tick, no recovery.
    #[test]
    fn fists_swing_fast_and_even() {
        let record = WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(8.0, 9.66),
            &Holder::AVERAGE,
        );
        let fists = Swing::of(&record);
        assert!((fists.speed - Swing::FISTS.speed).abs() < 1e-6);
        assert!((fists.speed - 1.44).abs() < 1e-6);
        assert_eq!((fists.modulation, fists.recovery), (0.0, 0));
        for p in [0.0, 0.3, 0.5, 0.9] {
            assert!((fists.frames(Some(5), p, 1.0, 1.0) - 1.62).abs() < 1e-5);
        }
    }

    /// A stab is 1.2 times faster, and slower early: 0.5 + 1.2p of it under
    /// 40 % of the clip.
    #[test]
    fn a_stab_starts_slowly() {
        let even = Swing {
            speed: 1.0,
            modulation: 0.0,
            recovery: 0,
        };
        let stab = |p: f32| even.frames(Some(STAB), p, 1.0, 1.0);
        assert!((stab(0.1) - 1.125 * 1.2 * 0.62).abs() < 1e-5);
        assert!((stab(0.5) - 1.125 * 1.2).abs() < 1e-5);
    }

    /// A tired fighter's swing: stopped under 80 % of its energy, the share
    /// squared under all of it (in place of the weapon's rate), and
    /// scaled by the clock's base.
    #[test]
    fn energy_slows_the_swing() {
        let s = Swing::FISTS;
        assert_eq!(s.frames(Some(5), 0.5, 1.0, 0.79), 0.0);
        assert!((s.frames(Some(5), 0.5, 1.0, 0.9) - 0.81).abs() < 1e-6);
        assert!((s.frames(Some(5), 0.5, 1.5, 1.0) - 1.62 * 1.5).abs() < 1e-5);
    }

    /// A fighter swings with its primary striker: the right fist before
    /// any attack, the left after an other-side attack made it primary (a
    /// Harvester's left claw swings slower than its right fist).
    #[test]
    fn the_primary_striker_paces_the_swing() {
        let mut app = App::new();
        app.add_systems(Update, pace_swings);
        let fighter = app.world_mut().spawn(Attacker::default()).id();
        let hand = |section: &Section| {
            WeaponRecord::build(section, false, Lengths::hand(8.0, 9.66), &Holder::AVERAGE)
        };
        let (fist, claw) = (hand(&Section::FISTS), hand(&Section::HEAVY_CLAWS));
        app.world_mut()
            .spawn(Blade::new(fighter, 0.18, fist).in_slot(Slot::First));
        app.world_mut()
            .spawn(Blade::new(fighter, 0.18, claw).in_slot(Slot::Second));
        app.update();
        let swing = |app: &App| *app.world().get::<Swing>(fighter).unwrap();
        assert_eq!(swing(&app), Swing::of(&fist));
        app.world_mut()
            .get_mut::<Attacker>(fighter)
            .unwrap()
            .primary = Some(Slot::Second);
        app.update();
        assert_eq!(swing(&app), Swing::of(&claw));
        assert_ne!(Swing::of(&claw), Swing::of(&fist));
    }
}

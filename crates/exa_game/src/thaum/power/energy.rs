//! The Energy powers' active classes: Shock's charge and release, and Shock
//! Weapons' arcs on the held weapons.
//!
//! - **Shock** ([`Shock`]): the first control tick starts cast motion 0x27
//!   and plays the charge. Each later tick adds the charge rate; at or below
//!   one it spends the tick's focus, and past one it fires the arc burst
//!   once, clears the hand's arcs and plays the release. A second charge
//!   past one answers 2 (the power is done). A control event carrying bit 1
//!   or 3 ends it, clearing the hand's arcs first.
//! - **Shock Weapons** ([`ShockWeapons`]): the first tick starts cast
//!   motion 0x25; ending clears each armed weapon's arcs and both body
//!   callbacks. The weapon changes themselves are
//!   [`super::super::energy::ArmedWeapons`].
//!
//! Evidence: the two classes' control handlers, read from `Exanima.exe`.

use super::PowerBase;
use crate::thaum::casting::{CastBody, start_cast};

/// The Energy power ids the classes run.
pub const SHOCK: u32 = 0x401;
pub const SHOCK_WEAPONS: u32 = 0x2040B;

/// The cast motions they start.
pub const SHOCK_MOTION: u32 = 0x27;
pub const SHOCK_WEAPONS_MOTION: u32 = 0x25;

/// The sounds the shock plays.
pub const CHARGE_SOUND: &str = "thmnrg_chrga";
pub const RELEASE_SOUND: &str = "thmnrg_shock";

/// Shock's phase: charging.
pub const CHARGE: i32 = 0;
/// Shock's phase: the burst has been released.
pub const FIRED: i32 = 1;
/// Shock's phase: cleared without a burst.
pub const CLEARED: i32 = -1;

/// How long Shock Weapons keeps the arcs' character bits.
pub const WEAPON_ARC_ON: i32 = 1;

/// What a Shock tick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShockStep {
    pub result: i32,
    /// The first tick's cast motion was taken.
    pub motion: bool,
    /// The tick spends its focus.
    pub drain: bool,
    /// The burst fires this tick.
    pub fired: bool,
    /// The hand's arcs are cleared.
    pub cleared: bool,
    /// The release sound plays.
    pub release_sound: bool,
}

/// Shock: a charge that fires one arc burst.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Shock {
    pub state: i32,
    /// The charge amount.
    pub charge: f32,
    /// What one tick adds.
    pub rate: f32,
    /// What one charging tick spends.
    pub focus_tick: f32,
    /// Charging, fired or cleared.
    pub phase: i32,
}

impl Shock {
    /// A charge that would take `steps` ticks at `rate` and spends
    /// `focus_tick` a tick.
    pub fn new(rate: f32, focus_tick: f32) -> Self {
        Self {
            state: super::READY,
            charge: 0.0,
            rate,
            focus_tick,
            phase: CHARGE,
        }
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ShockStep {
        let quiet = ShockStep {
            result: 0,
            motion: false,
            drain: false,
            fired: false,
            cleared: false,
            release_sound: false,
        };
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let cleared = self.phase == CHARGE;
            if cleared {
                self.phase = CLEARED;
            }
            return ShockStep {
                result: 2,
                cleared,
                ..quiet
            };
        }
        if self.state == super::READY {
            self.state = super::RUNNING;
            return ShockStep {
                motion: true,
                ..quiet
            };
        }
        self.charge += self.rate;
        if self.charge <= 1.0 {
            if self.phase == CHARGE {
                super::drain(body, self.focus_tick);
                return ShockStep {
                    drain: true,
                    ..quiet
                };
            }
            return quiet;
        }
        self.charge = 0.0;
        if self.phase == CHARGE {
            self.phase = FIRED;
            return ShockStep {
                fired: true,
                cleared: true,
                release_sound: true,
                ..quiet
            };
        }
        ShockStep { result: 2, ..quiet }
    }
}

/// What a Shock Weapons tick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShockWeaponsStep {
    pub result: i32,
    pub released: bool,
    pub motion: bool,
    /// Every armed weapon's arcs are cleared.
    pub cleared: bool,
}

/// Shock Weapons: arcs on the held weapons.
#[derive(Debug, Clone, PartialEq)]
pub struct ShockWeapons {
    pub state: i32,
    pub held: f32,
    /// How many weapons carry arcs.
    pub count: usize,
}

impl ShockWeapons {
    /// The HUD word of a running power.
    pub fn hud(&self) -> super::HudState {
        super::HudState::active()
    }

    pub fn new() -> Self {
        Self {
            state: super::READY,
            held: 0.0,
            count: 0,
        }
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ShockWeaponsStep {
        if event & 0xa != 0 {
            self.state = super::ENDING;
        }
        if self.state == super::ENDING {
            let released = 0.0 < self.held;
            if released {
                PowerBase {
                    id: SHOCK_WEAPONS,
                    state: self.state,
                    held: self.held,
                }
                .release(body);
            }
            self.count = 0;
            return ShockWeaponsStep {
                result: 2,
                released,
                motion: false,
                cleared: true,
            };
        }
        if self.state == super::READY {
            let motion = start_cast(body, SHOCK_WEAPONS_MOTION, SHOCK_WEAPONS);
            self.state = super::RUNNING;
            return ShockWeaponsStep {
                result: 0,
                released: false,
                motion,
                cleared: false,
            };
        }
        ShockWeaponsStep {
            result: 0,
            released: false,
            motion: false,
            cleared: false,
        }
    }
}

impl Default for ShockWeapons {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shock_charges_fires_once_and_then_ends() {
        let mut body = CastBody::default();
        let mut shock = Shock::new(0.5, 0.1);
        let first = shock.control(0, &mut body);
        assert!(first.motion);
        let half = shock.control(0, &mut body);
        assert!(half.drain);
        assert!((shock.charge - 0.5).abs() < 1e-6);
        let second = shock.control(0, &mut body);
        assert!(!second.fired);
        let burst = shock.control(0, &mut body);
        assert!(burst.fired);
        assert!(burst.cleared);
        assert_eq!(shock.phase, FIRED);
        assert_eq!(shock.charge, 0.0);
        // The next charge cycle runs dry without a second burst and ends.
        for _ in 0..4 {
            if shock.control(0, &mut body).result == 2 {
                return;
            }
        }
        panic!("the shock did not end");
    }

    #[test]
    fn a_cancelled_shock_clears_its_hand_arcs() {
        let mut body = CastBody::default();
        let mut shock = Shock::new(0.5, 0.1);
        shock.control(0, &mut body);
        let end = shock.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.cleared);
        assert_eq!(shock.phase, CLEARED);
    }

    #[test]
    fn shock_weapons_starts_a_motion_and_clears_its_arcs() {
        let mut body = CastBody::default();
        let mut weapons = ShockWeapons::new();
        weapons.held = 0.3;
        body.spent = 0.3;
        assert!(weapons.control(0, &mut body).motion);
        assert_eq!(weapons.state, super::super::RUNNING);
        assert_eq!(weapons.control(0, &mut body).result, 0);
        let end = weapons.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.cleared && end.released);
        assert_eq!(body.spent, 0.0);
    }
}

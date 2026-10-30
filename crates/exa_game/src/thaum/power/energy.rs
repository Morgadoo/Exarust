//! The Energy powers' active classes: Shock's charge and release, and Shock
//! Weapons' arcs on the held weapons.
//!
//! - **Shock** ([`Shock`]): the cast has already started motion 0x27; the
//!   first control tick installs the cast hook (−1 to every event; a call
//!   while charging, a focus shortfall's included, clears the hand's arcs
//!   so the shock never fires) and plays the charge (volume 0.2, pitch
//!   `0.9 + 0.2·roll`). Each later tick adds the charge rate; not
//!   past one (or not a number) it spends the tick's focus while charging,
//!   and past one it fires the arc burst once ([`super::super::arcs::shock_burst`]:
//!   ten tendrils of fourteen points, size 10, growth 0.25, lift 0.5, light
//!   1, shock strength 0.05),
//!   clears the hand's arcs, the cast hook and the cast id, and plays the
//!   release (volume 1). A second
//!   charge past one answers 2 (the power is done). A control event carrying
//!   bit 1 or 3 ends it, clearing the hand's arcs first when it never fired,
//!   and stops the cast motion.
//! - **Shock Weapons** ([`ShockWeapons`]): the first tick starts cast
//!   motion 0x25, with a hook answering 0 to bits 0x29; ending clears each armed weapon's arcs and both body
//!   callbacks. The weapon changes themselves are
//!   [`super::super::energy::ArmedWeapons`].
//!
//! Evidence: the two classes' control handlers, read from `Exanima.exe`.

use super::PowerBase;
use crate::thaum::casting::{CastBody, CastHook, start_cast};

pub use crate::thaum::energy::{SHOCK, SHOCK_MOTION, SHOCK_WEAPONS};

/// The cast motion Shock Weapons starts.
pub const SHOCK_WEAPONS_MOTION: u32 = 0x25;

/// Shock's cast hook: −1 to every event; called while the shock charges,
/// it clears the hand's arcs and the shock never fires.
pub const SHOCK_HOOK: CastHook = CastHook::NEVER_KEEPS;
/// Shock Weapons' cast hook: 0 to an event carrying bits 0x29.
pub const SHOCK_WEAPONS_HOOK: CastHook = CastHook { keeps: 0x29 };

/// The sounds the shock plays, and their volumes.
pub use crate::thaum::energy::{CHARGE_SOUND, SHOCK_SOUND as RELEASE_SOUND};
pub const CHARGE_VOLUME: f32 = 0.2;
pub const RELEASE_VOLUME: f32 = 1.0;

/// The pitch both shock sounds take: `0.9 + 0.2·roll` in double.
pub fn shock_pitch(roll: f64) -> f32 {
    (roll * 0.2 + 0.9) as f32
}

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
    /// The first tick: the cast hook is installed and the charge sound
    /// plays.
    pub charge_sound: bool,
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
    /// The caster's cast hook is this shock's.
    pub hooked: bool,
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
            hooked: false,
        }
    }

    /// The hook's call: a charging shock is cleared.
    fn hook_called(&mut self) -> bool {
        self.hooked = false;
        let cleared = self.phase == CHARGE;
        if cleared {
            self.phase = CLEARED;
        }
        cleared
    }

    /// One control tick.
    pub fn control(&mut self, event: i32, body: &mut CastBody) -> ShockStep {
        let quiet = ShockStep {
            result: 0,
            charge_sound: false,
            drain: false,
            fired: false,
            cleared: false,
            release_sound: false,
        };
        let mut quiet = quiet;
        if self.hooked && (body.casting != SHOCK || body.cast_hook != Some(SHOCK_HOOK)) {
            quiet.cleared = self.hook_called();
        }
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
                cleared: cleared || quiet.cleared,
                ..quiet
            };
        }
        if self.state == super::READY {
            self.state = super::RUNNING;
            body.cast_hook = Some(SHOCK_HOOK);
            self.hooked = true;
            return ShockStep {
                charge_sound: true,
                ..quiet
            };
        }
        self.charge += self.rate;
        // The ordered compare: a charge that is not a number keeps
        // charging.
        let past_one = self.charge.partial_cmp(&1.0) == Some(std::cmp::Ordering::Greater);
        if !past_one {
            if self.phase == CHARGE {
                if super::drain(body, self.focus_tick) && self.hooked {
                    quiet.cleared = self.hook_called();
                }
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
            self.hooked = false;
            body.cast_hook = None;
            body.casting = 0;
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
            if motion {
                body.cast_hook = Some(SHOCK_WEAPONS_HOOK);
            }
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
        assert!(first.charge_sound);
        body.casting = SHOCK;
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
        assert_eq!(body.casting, 0);
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
        body.casting = SHOCK;
        let end = shock.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.cleared);
        assert_eq!(shock.phase, CLEARED);
    }

    #[test]
    fn a_shock_hook_called_while_charging_clears_it() {
        let mut body = CastBody::default();
        let mut shock = Shock::new(0.5, 0.1);
        shock.control(0, &mut body);
        assert_eq!(body.cast_hook, Some(SHOCK_HOOK));
        body.casting = SHOCK;
        assert!(start_cast(&mut body, 0x22, 0x201), "the hook answers -1");
        let step = shock.control(0, &mut body);
        assert!(step.cleared && !step.drain);
        assert_eq!(shock.phase, CLEARED);
        let rest: Vec<_> = (0..2).map(|_| shock.control(0, &mut body)).collect();
        assert!(rest.iter().all(|step| !step.fired && !step.drain));
        assert_eq!(rest[1].result, 2, "no burst, then done");

        let mut body = CastBody {
            focus: 0.15,
            ..CastBody::default()
        };
        let mut short = Shock::new(0.5, 0.1);
        short.control(0, &mut body);
        body.casting = SHOCK;
        let step = short.control(0, &mut body);
        assert!(step.drain && step.cleared);
        assert_eq!((body.casting, body.cast_hook), (0, None));
    }

    #[test]
    fn shock_weapons_starts_a_motion_and_clears_its_arcs() {
        let mut body = CastBody::default();
        let mut weapons = ShockWeapons::new();
        weapons.held = 0.3;
        body.reserved = 0.3;
        assert!(weapons.control(0, &mut body).motion);
        assert_eq!(body.cast_hook, Some(SHOCK_WEAPONS_HOOK));
        assert_eq!(weapons.state, super::super::RUNNING);
        assert_eq!(weapons.control(0, &mut body).result, 0);
        let end = weapons.control(2, &mut body);
        assert_eq!(end.result, 2);
        assert!(end.cleared && end.released);
        assert_eq!(body.reserved, 0.0);
    }
}

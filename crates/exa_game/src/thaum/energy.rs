//! The Energy domain's casts, as the original's Energy unit routines run
//! them: Shock and shock weapons, their front ends, AI evaluators and what
//! each makes. The arcs themselves are [`super::arcs`].
//!
//! - **Table** ([`ENERGY_POWERS`]): shock 0x401 at 0.10 and shock weapons
//!   0x2040B at 0.30. The front ends price as the other domains' do, a
//!   sustained cost over 1000 rather than times 0.001, and the player's
//!   front end has no status test.
//! - **Shock**: pressing it while it charges releases it; it needs 1.2
//!   times its cost in focus (in double precision) and the state gate; then
//!   cast motion 0x27 starts and, when it does, the shock is made (the cast
//!   returns 1 either way). Its charge lasts the cast clip's length over
//!   1200 ms and costs the cost times that times 1.2; its hand arcs
//!   ([`super::arcs::hand_arcs`]) ride the right hand, the charge plays
//!   `thmnrg_chrga` and the release `thmnrg_shock`.
//! - **Shock weapons**: a persistent power that needs a weapon in hand and
//!   a free slot; it puts arcs ([`super::arcs::weapon_arcs`]) on each held
//!   weapon, at most four, holds its cost, and follows weapon changes: a
//!   weapon put away stops respawning its arcs and lets them end; a weapon
//!   taken while fewer than four carry arcs gains them. A blade that strikes
//!   a body shocks its character by 0.01 and sets a spark burst
//!   ([`super::arcs::spark_burst`]) at the contact, on the struck part.
//! - **AI**: shock 3 with 1.2 times the cost in focus, facing within 1, the
//!   target within 5 m and on its feet; shock weapons as the other
//!   persistent self powers ([`super::casting::persistent_advice`]).
//!
//! Evidence: the Energy table and AI table, the two casts, the front ends,
//! the shock and weapon creators and the weapon-change hook, read from
//! `Exanima.exe` (`context/re/thaum-casts.md`).

use super::casting::{
    AiView, CastBody, ID_MASK, OFF_FEET, TickScale, WEAPON_IN_HAND, advice, body_gate, code,
    free_slots, persistent_advice, price, start_cast, within,
};

/// One row of the Energy table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnergyRow {
    pub id: u32,
    pub cost: f32,
}

pub const SHOCK: u32 = 0x401;
pub const SHOCK_WEAPONS: u32 = 0x2040B;

/// The Energy table.
pub const ENERGY_POWERS: [EnergyRow; 2] = [
    EnergyRow {
        id: SHOCK,
        cost: 0.1,
    },
    EnergyRow {
        id: SHOCK_WEAPONS,
        cost: 0.3,
    },
];

/// The cast motion Shock plays.
pub const SHOCK_MOTION: u32 = 0x27;
pub const CHARGE_SOUND: &str = "thmnrg_chrga";
pub const SHOCK_SOUND: &str = "thmnrg_shock";
/// Shock weapons carry arcs on this many weapons at most.
pub const MOST_WEAPONS: usize = 4;

/// What an Energy cast makes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EnergyStart {
    /// The shock, priced at `cost`, when its cast motion started.
    Shock { cost: f32 },
    /// Arcs on the held weapons, holding `cost`.
    ShockWeapons { cost: f32 },
}

/// One Energy cast of a priced row.
pub fn cast_energy(row: &EnergyRow, cost: f32, body: &mut CastBody) -> (i32, Option<EnergyStart>) {
    if row.id == SHOCK {
        if body.casting == row.id {
            body.release = true;
            return (code::RELEASED, None);
        }
        if f64::from(cost) * 1.2 > f64::from(body.focus) {
            return (code::REFUSED, None);
        }
        let gate = body_gate(body);
        if gate < 0 {
            return (gate, None);
        }
        let started = start_cast(body, SHOCK_MOTION, row.id);
        return (
            code::STARTED,
            started.then_some(EnergyStart::Shock { cost }),
        );
    }
    let free = match free_slots(body, row.id) {
        Ok(free) => free,
        Err(ended) => return (ended, None),
    };
    if free < 1 {
        return (code::REFUSED, None);
    }
    if body.casting == row.id {
        body.release = true;
        return (code::RELEASED, None);
    }
    if body.weapon_class & WEAPON_IN_HAND == 0 || !within(cost, body.focus) {
        return (code::REFUSED, None);
    }
    let gate = body_gate(body);
    if gate < 0 {
        return (gate, None);
    }
    (code::STARTED, Some(EnergyStart::ShockWeapons { cost }))
}

/// The row for `asked` under the front ends' mask.
pub fn energy_row(asked: u32) -> Option<&'static EnergyRow> {
    ENERGY_POWERS.iter().find(|row| asked & ID_MASK == row.id)
}

/// The player's Energy front end.
pub fn energy_front_end(asked: u32, body: &mut CastBody) -> (i32, Option<EnergyStart>) {
    let Some(row) = energy_row(asked) else {
        return (code::REFUSED, None);
    };
    let cost = price(row.id, row.cost, asked, body, TickScale::OverThousand);
    cast_energy(row, cost, body)
}

/// The AI's Energy front end.
pub fn energy_ai_front_end(asked: u32, view: &AiView) -> u32 {
    let Some(row) = energy_row(asked) else {
        return advice::UNAVAILABLE;
    };
    let cost = price(
        row.id,
        row.cost,
        asked,
        &view.caster,
        TickScale::OverThousand,
    );
    if row.id == SHOCK {
        shock_advice(cost, view)
    } else {
        persistent_advice(row.id, cost, view)
    }
}

/// Shock: 3 when the AI should cast it.
pub fn shock_advice(cost: f32, view: &AiView) -> u32 {
    let use_it = f64::from(cost) * 1.2 <= f64::from(view.caster.focus)
        && view.facing <= 1.0
        && view.distance_sq <= 250_000.0
        && view.target_condition & OFF_FEET == 0;
    if use_it { 3 } else { 0 }
}

/// A shock's charge: its length (the cast clip's milliseconds over 1200)
/// and its cost (the cost times that times 1.2).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShockCharge {
    pub length: f32,
    pub cost: f32,
}

/// The shock's charge from its cast clip's length.
pub fn shock_charge(clip_ms: u32, cost: f32) -> ShockCharge {
    let length = (f64::from(clip_ms) / 1200.0) as f32;
    ShockCharge {
        length,
        cost: (f64::from(cost * length) * 1.2) as f32,
    }
}

/// The charge object the Shock creator instantiates: its owner, the cast
/// flag (1), the base cost it was cast with, its charge and hand arcs, the
/// charge-up sound it looks up and the release sound it keeps. The original
/// also adds it to the owner's active powers with the flags `0x401`, and
/// tells the player controls first when the owner is the player; neither is
/// modelled here.
#[derive(Debug, Clone, PartialEq)]
pub struct ShockPower {
    pub owner: u64,
    /// The object's cast flag (1).
    pub cast_flag: i32,
    /// The cost the cast was made with, before the charge scales it.
    pub base_cost: f32,
    pub charge: ShockCharge,
    /// The hand arcs the charge object creates (`0x201`, 20 tendrils of 5
    /// segments at the hand's −15, −5 cm).
    pub arcs: super::arcs::ArcParams,
    /// The charge-up sound it looks up (`thmnrg_chrga`).
    pub sound: &'static str,
    /// The release sound it keeps (`thmnrg_shock`).
    pub release_sound: &'static str,
}

/// Creates the Shock's charge object from the owner, the cast clip's length
/// and the power's base cost.
pub fn shock_power(owner: u64, clip_ms: u32, base_cost: f32) -> ShockPower {
    ShockPower {
        owner,
        cast_flag: 1,
        base_cost,
        charge: shock_charge(clip_ms, base_cost),
        arcs: super::arcs::hand_arcs(),
        sound: CHARGE_SOUND,
        release_sound: SHOCK_SOUND,
    }
}

/// What shock weapons do when the held weapons change.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeaponChange {
    /// That weapon's arcs stop respawning and end.
    End(u64),
    /// That weapon gains arcs.
    Arm(u64),
    /// Nothing.
    Keep,
}

/// The weapons shock weapons has armed.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ArmedWeapons {
    pub weapons: Vec<u64>,
}

impl ArmedWeapons {
    /// Arms each held weapon in turn (the creator arms every one).
    pub fn arm_all(held: &[u64]) -> Self {
        Self {
            weapons: held.to_vec(),
        }
    }

    /// A weapon put away (`taken` false) or taken.
    pub fn change(&mut self, weapon: u64, taken: bool) -> WeaponChange {
        if !taken {
            if let Some(k) = self.weapons.iter().position(|&w| w == weapon) {
                self.weapons.swap_remove(k);
            }
            return WeaponChange::End(weapon);
        }
        if self.weapons.len() < MOST_WEAPONS {
            self.weapons.push(weapon);
            return WeaponChange::Arm(weapon);
        }
        WeaponChange::Keep
    }
}

/// The shock a struck body's character takes from an armed blade.
pub const BLADE_SHOCK: f32 = super::arcs::WEAPON_SHOCK;

#[cfg(test)]
mod tests {
    use super::super::casting::ActivePower;
    use super::*;

    #[test]
    fn shock_needs_its_motion_to_be_made() {
        let mut body = CastBody {
            focus: 0.1201,
            ..CastBody::default()
        };
        let (result, start) = energy_front_end(SHOCK, &mut body);
        assert_eq!(result, code::STARTED);
        assert_eq!(start, Some(EnergyStart::Shock { cost: 0.1 }));
        assert_eq!(body.cast_motion, Some(SHOCK_MOTION));
        assert_eq!(energy_front_end(SHOCK, &mut body).0, code::RELEASED);

        let mut held = CastBody {
            focus: 1.0,
            cast_hook: Some(0),
            ..CastBody::default()
        };
        assert_eq!(energy_front_end(SHOCK, &mut held), (code::STARTED, None));
        let mut short = CastBody {
            focus: 0.12,
            ..CastBody::default()
        };
        assert_eq!(energy_front_end(SHOCK, &mut short).0, code::REFUSED);
    }

    #[test]
    fn shock_weapons_need_a_blade_and_a_slot() {
        let mut fists = CastBody::default();
        assert_eq!(energy_front_end(SHOCK_WEAPONS, &mut fists).0, code::REFUSED);
        let mut armed = CastBody {
            weapon_class: 0x20,
            ..CastBody::default()
        };
        assert_eq!(
            energy_front_end(SHOCK_WEAPONS, &mut armed),
            (code::STARTED, Some(EnergyStart::ShockWeapons { cost: 0.3 }))
        );
        armed.powers.push(ActivePower {
            id: SHOCK_WEAPONS,
            state: 1,
        });
        assert_eq!(energy_front_end(SHOCK_WEAPONS, &mut armed).0, code::ENDED);
        assert_eq!(armed.powers[0].state, 2);
    }

    #[test]
    fn the_charge_follows_the_clip() {
        let charge = shock_charge(1200, 0.1);
        assert_eq!(charge.length, 1.0);
        assert_eq!(charge.cost, (f64::from(0.1f32) * 1.2) as f32);
        let half = shock_charge(600, 0.1);
        assert_eq!(half.length, 0.5);
    }

    #[test]
    fn the_ai_shocks_within_five_metres() {
        let mut view = AiView {
            caster: CastBody {
                focus: 0.1201,
                ..CastBody::default()
            },
            facing: 1.0,
            distance_sq: 250_000.0,
            ..AiView::default()
        };
        assert_eq!(energy_ai_front_end(SHOCK, &view), 3);
        view.distance_sq = 250_001.0;
        assert_eq!(energy_ai_front_end(SHOCK, &view), 0);
        view.facing = 0.5;
        view.caster.focus = 0.6;
        assert_eq!(energy_ai_front_end(SHOCK_WEAPONS, &view), advice::USE);
        assert_eq!(energy_ai_front_end(0x402, &view), advice::UNAVAILABLE);
    }

    #[test]
    fn at_most_four_weapons_carry_arcs() {
        let mut armed = ArmedWeapons::arm_all(&[1, 2, 3]);
        assert_eq!(armed.change(4, true), WeaponChange::Arm(4));
        assert_eq!(armed.change(5, true), WeaponChange::Keep);
        assert_eq!(armed.change(2, false), WeaponChange::End(2));
        assert_eq!(armed.weapons, vec![1, 4, 3]);
        assert_eq!(armed.change(5, true), WeaponChange::Arm(5));
    }

    #[test]
    fn the_shock_creator_builds_its_charge_object() {
        let power = shock_power(42, 600, 0.1);
        assert_eq!(power.owner, 42);
        assert_eq!(power.cast_flag, 1);
        assert_eq!(power.base_cost, 0.1);
        // 600 ms is half the charge: 0.5 long, costing 0.1·0.5·1.2.
        assert_eq!(power.charge.length, 0.5);
        assert!((power.charge.cost - 0.06).abs() < 1e-6);
        assert_eq!(power.sound, "thmnrg_chrga");
        assert_eq!(power.release_sound, "thmnrg_shock");
        // The hand arcs: 20 tendrils, 5 segments, life 10, size 2, scale
        // 0.25, growth 0.25, light 48.
        let arcs = power.arcs;
        assert_eq!((arcs.count, arcs.segments, arcs.life), (20, 5, 10));
        assert_eq!((arcs.size, arcs.scale, arcs.growth), (2.0, 0.25, 0.25));
        assert_eq!(arcs.light, 48.0);
        assert_eq!(
            super::super::arcs::HAND_ARCS_OFFSET,
            bevy::math::Vec3::new(-15.0, -5.0, 0.0)
        );
    }
}

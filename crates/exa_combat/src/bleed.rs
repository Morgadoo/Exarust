//! Bleeding: the damage-over-time a wound tally becomes on the body tick.
//!
//! A positive tally raises the bleeding level by 0.03 of itself (the product
//! in double, the sum in single), capped at 20 when the level compares above
//! 20. Yellow loses the level times 0.00004, and a level above 1 takes
//! (level times 0.002) squared off red, both in double. Yellow then scales
//! by the red left, or becomes 0 when the red the tick began with does not
//! compare above the least red. The regeneration rate clears. Yellow under 0
//! zeroes both bars and starts the collapse. Without a positive tally the
//! level falls by 0.1, floored at 0, and the health stays. Either way the
//! tally clears. A tick whose tally and level are not positive does not run.
//! The down bit or the dying bit (down, getting up, dormant, collapsing,
//! dead) stops the tick and leaves the tally. The groan and the burn sample
//! are not played.
//!
//! The character tick's upkeep stage runs this ([`bleed_upkeep`]), after red
//! regeneration and before burning.

use bevy::prelude::*;

use crate::collapse;

/// The level gains this share of a positive tally, in double, then added in
/// single precision.
const LEVEL_GAIN: f64 = 0.03;
/// The level is capped here when it compares above it.
const LEVEL_CAP: f32 = 20.0;
/// Yellow loses the level times this. Red loses (level times [`RED_RATE`])²
/// once the level compares above 1.
const YELLOW_LOSS: f64 = 0.000_04;
const RED_RATE: f64 = 0.002;
/// Without a positive tally the level falls by this, in double.
const LEVEL_FALL: f64 = 0.1;
/// The red the tick began with must compare above this before yellow scales
/// by the red left. The bits are the original's least red.
const RED_LEAST: f32 = f32::from_bits(0x33ff_ffd7);
/// Down bit or dying bit: getting up and dormant keep the down bit, and
/// collapsing and dead hold the dying bit. The tick changes nothing.
const STOP: u32 = 0xa00;

/// What one bleeding tick writes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BleedStep {
    pub yellow: f32,
    pub red: f32,
    pub level: f32,
    /// Always 0: the tick cleared the tally.
    pub tally: f32,
    pub collapse: bool,
    /// A positive tally cleared the regeneration rate.
    pub clears_regen: bool,
}

/// One bleeding tick. `None` when the tally and the level are not positive,
/// or when `condition` holds a [`STOP`] bit: the caller keeps the tally.
pub fn bleed_tick(
    condition: u32,
    yellow: f32,
    red: f32,
    level: f32,
    tally: f32,
) -> Option<BleedStep> {
    if !(tally > 0.0 || level > 0.0) {
        return None;
    }
    if condition & STOP != 0 {
        return None;
    }
    if tally > 0.0 {
        let mut level = level + (f64::from(tally) * LEVEL_GAIN) as f32;
        if level > LEVEL_CAP {
            level = LEVEL_CAP;
        }
        let old_red = red;
        let mut yellow = (f64::from(yellow) - f64::from(level) * YELLOW_LOSS) as f32;
        let mut red = red;
        if level > 1.0 {
            let loss = f64::from(level) * RED_RATE;
            red = (f64::from(red) - loss * loss) as f32;
        }
        if old_red > RED_LEAST {
            // The quotient is the left factor, as the original's multiply.
            #[allow(clippy::assign_op_pattern)]
            {
                yellow = (red / old_red) * yellow;
            }
        } else {
            yellow = 0.0;
        }
        let mut collapse = false;
        if yellow < 0.0 {
            yellow = 0.0;
            red = 0.0;
            collapse = true;
        }
        return Some(BleedStep {
            yellow,
            red,
            level,
            tally: 0.0,
            collapse,
            clears_regen: true,
        });
    }
    let mut level = (f64::from(level) - LEVEL_FALL) as f32;
    if level < 0.0 {
        level = 0.0;
    }
    Some(BleedStep {
        yellow,
        red,
        level,
        tally: 0.0,
        collapse: false,
        clears_regen: false,
    })
}

/// Writes one bleeding tick onto the fighter's health, level and tally.
/// A tick that does not run leaves them as they are.
pub fn apply_bleed(
    health: &mut crate::Health,
    condition: u32,
    level: &mut f32,
    tally: &mut f32,
) -> Option<BleedStep> {
    let step = bleed_tick(condition, health.yellow, health.red, *level, *tally)?;
    health.yellow = step.yellow;
    health.red = step.red;
    if step.clears_regen {
        health.rate = 0.0;
    }
    *level = step.level;
    *tally = step.tally;
    Some(step)
}

/// The ticked body's bleeding, in its upkeep stage: the level, the tally,
/// the health and the regeneration rate, and a collapse when yellow falls
/// below 0. The blend takes the body's muscle energy.
#[allow(clippy::type_complexity)]
pub fn bleed_upkeep(
    mut commands: Commands,
    ticked: Res<exa_physics::body::TickedCharacter>,
    mut fighters: Query<(
        &mut crate::Fighter,
        &mut crate::Condition,
        &mut crate::StateFlags,
        &mut exa_anim::Animator,
        Option<&mut exa_physics::body::Ragdoll>,
        Option<&mut crate::collapse::Hands>,
    )>,
) {
    let Ok((mut fighter, mut condition, mut flags, mut anim, mut body, mut hands)) =
        fighters.get_mut(ticked.body)
    else {
        return;
    };
    let mut level = fighter.bleed_level;
    let mut tally = fighter.bleeding.get();
    let Some(step) = apply_bleed(&mut fighter.health, condition.0, &mut level, &mut tally) else {
        return;
    };
    fighter.bleed_level = level;
    if tally == 0.0 {
        fighter.bleeding.take();
    }
    let energy = body
        .as_ref()
        .and_then(|b| b.drive.as_ref())
        .map_or(1.0, |d| d.energy);
    if step.collapse
        && collapse::start_collapse(
            &mut condition,
            &mut flags,
            &mut anim,
            body.as_deref_mut(),
            energy,
            hands.as_deref_mut(),
        )
    {
        commands
            .entity(ticked.body)
            .insert(collapse::Collapse::default())
            .remove::<exa_physics::body::Fallen>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A tally of 1 raises the level to the single-precision 0.03 and takes
    /// that times 0.00004 off yellow. A tally past the cap stops at 20 and
    /// scales yellow by the red left. No tally takes 0.1 off the level.
    #[test]
    fn the_bleed_is_the_originals() {
        let step = bleed_tick(0, 0.9, 1.0, 0.0, 1.0).unwrap();
        assert_eq!(step.level.to_bits(), 0.03f32.to_bits());
        assert_eq!(step.yellow.to_bits(), 0x3f66_6652);
        assert_eq!(step.red, 1.0);
        assert_eq!(step.tally, 0.0);
        assert!(step.clears_regen);
        assert!(!step.collapse);

        let step = bleed_tick(0, 0.9, 1.0, 0.0, 1000.0).unwrap();
        assert_eq!(step.level, 20.0);
        let loss = (f64::from(20.0f32) * 0.002).powi(2);
        let red = (f64::from(1.0f32) - loss) as f32;
        assert_eq!(step.red, red);
        let yellow = (f64::from(0.9f32) - f64::from(20.0f32) * 0.000_04) as f32;
        assert_eq!(step.yellow, (red / 1.0) * yellow);

        let step = bleed_tick(0, 1.0, 1.0, 0.25, 0.0).unwrap();
        assert_eq!(
            step.level.to_bits(),
            ((f64::from(0.25f32) - 0.1) as f32).to_bits()
        );
        assert_eq!((step.yellow, step.red), (1.0, 1.0));
        assert!(!step.clears_regen);
        assert_eq!(step.tally, 0.0);
    }

    /// Yellow under 0 collapses. A red at the least red zeroes yellow
    /// without a collapse. Down, getting up and dying keep the tally.
    #[test]
    fn the_bleed_collapses_and_stops() {
        let step = bleed_tick(0, 0.0005, 1.0, 0.0, 500.0).unwrap();
        assert!(step.collapse);
        assert_eq!((step.yellow, step.red), (0.0, 0.0));

        let least = f32::from_bits(0x33ff_ffd7);
        let step = bleed_tick(0, 0.5, least, 0.0, 1.0).unwrap();
        assert_eq!(step.yellow, 0.0);
        assert!(!step.collapse);
        assert!(step.red > 0.0);

        for condition in [0x200u32, 0x300, 0x600, 0x800, 0xa00] {
            assert!(bleed_tick(condition, 0.9, 1.0, 0.4, 1.0).is_none());
        }
        assert!(bleed_tick(0, 0.9, 1.0, 0.0, 0.0).is_none());
        assert!(bleed_tick(0, 0.9, 1.0, 0.0, -1.0).is_none());
        let step = bleed_tick(0, 0.9, 1.0, 0.2, -1.0).unwrap();
        assert_eq!(step.tally, 0.0);
        assert!(step.level < 0.2);
    }

    /// A NaN tally is not positive, so only a positive level runs, and a
    /// NaN level is not floored.
    #[test]
    fn a_nan_tally_is_not_positive() {
        assert!(bleed_tick(0, 1.0, 1.0, 0.0, f32::NAN).is_none());
        let step = bleed_tick(0, 1.0, 1.0, f32::NAN, 1.0).unwrap();
        assert!(step.level.is_nan());
        assert!(step.yellow.is_nan());
        assert!(!step.collapse);
        assert_eq!(step.tally, 0.0);
        let step = bleed_tick(0, 1.0, 1.0, f32::NAN, 0.0);
        assert!(step.is_none());
    }

    /// The upkeep stage spends the ticked fighter's tally and clears its rate.
    #[test]
    fn the_upkeep_spends_the_tally() {
        let mut app = App::new();
        app.add_systems(Update, bleed_upkeep);
        let mut fighter = crate::Fighter::default();
        fighter.health.rate = 1.0;
        fighter.bleeding.add(1, 1.0);
        let body = app
            .world_mut()
            .spawn((
                fighter,
                crate::Condition::NORMAL,
                crate::StateFlags::default(),
                exa_anim::Animator::new("Default", "Idle"),
            ))
            .id();
        app.insert_resource(exa_physics::body::TickedCharacter {
            body,
            character: None,
        });
        app.update();
        let fighter = app.world().get::<crate::Fighter>(body).unwrap();
        assert_eq!(fighter.health.rate, 0.0);
        assert_eq!(fighter.bleed_level.to_bits(), 0.03f32.to_bits());
        assert_eq!(fighter.bleeding.get(), 0.0);
        assert!(fighter.health.yellow < 0.25);
        assert!(app.world().get::<crate::collapse::Collapse>(body).is_none());
    }
}

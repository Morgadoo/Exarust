//! Burning: the damage-over-time a fire particle puts on a body. Each tick
//! the burn's strength raises a timer by 0.03 of itself (up to 20, or takes
//! 0.1 off it when the strength is gone); the body's yellow health falls by
//! the timer × 0.00004, its red by (timer × 0.002)², and yellow is then
//! scaled by the red's new share of its old value. A timer over 1 makes a
//! sound once the clock is 800 ms past the one drawn after the last. The
//! next clock is the game clock plus a draw below 800. A timer of 19 or
//! under groans at volume 0.3 + 0.2×the second draw and pitch 0.8 + 0.2×the
//! first. Over 19, a burn sample plays those levels with the character's
//! gains folded in double, and without a sample those two draws are
//! skipped. Yellow under 0, after red has scaled it,
//! collapses the body (yellow and red 0). A red already near 0 takes all of
//! yellow. A tick that still has strength clears the regeneration rate.
//! The body's condition bit 0x200 (down) and 0x800 (dying) stop it. The
//! character tick's upkeep stage runs this ([`burn_upkeep`]).
//!
//! The original keeps these as the burning body's strength, timer and sound
//! clock, and the yellow and red flame levels; [`Burn`] holds the first three.
//! Evidence: the burning routine's assembly and the damage model notes.
//! Constants are the original's.

use bevy::prelude::*;

use crate::collapse;

/// The timer gains this share of the strength a tick, up to 20; gone, it
/// loses 0.1.
const TIMER_GAIN: f64 = 0.03;
const TIMER_MOST: f32 = 20.0;
const TIMER_FALL: f64 = 0.1;
/// Yellow loses timer × this, red (timer × 0.002)².
const YELLOW_LOSS: f64 = 0.000_04;
const RED_LOSS: f64 = 0.002;
/// The red below which yellow is taken whole instead of scaled.
const RED_LEAST: f32 = 1.192_09e-7;
/// A sound waits this many ms past the clock the last one drew. The next
/// clock adds a draw below this many ms.
pub const BURN_SOUND_MS: u32 = 800;
/// The timer over which the burn takes red and may sound.
const SOUND_FROM: f32 = 1.0;
/// A timer over this groans no longer: it plays the burn's own sample.
const LOUD_FROM: f32 = 19.0;
/// Both sound levels are `base + 0.2 × draw`, in double, then narrowed.
const DRAW_SPAN: f64 = 0.2;
const VOLUME_BASE: f64 = 0.3;
const PITCH_BASE: f64 = 0.8;

/// A body's burn.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Burn {
    /// The burn's strength this tick.
    pub strength: f32,
    /// The timer it builds.
    pub timer: f32,
    /// The next sound's earliest game clock.
    pub next_sound: u32,
}

impl Default for Burn {
    fn default() -> Self {
        Self {
            strength: 0.0,
            timer: 0.0,
            next_sound: 0,
        }
    }
}

impl Burn {
    /// A fire's dose on one of the body's parts, as the particle
    /// body-contact pass hands it to the part: it adds to this tick's
    /// strength when the part's bone index is above zero, so a held weapon
    /// (negative) or a boneless body (0) adds nothing. A wound never adds.
    pub fn add_dose(&mut self, bone: i32, dose: f32) {
        if bone > 0 {
            self.strength += dose;
        }
    }
}

/// What one burning tick did: the body's new yellow and red, whether the
/// crackle sounds (its game clock), and whether the body collapses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurnStep {
    pub yellow: f32,
    pub red: f32,
    pub sound_at: Option<u64>,
    pub collapse: bool,
    /// The tick still had strength, so the regeneration rate is cleared.
    pub clears_regen: bool,
    /// The crackle this tick asked for, when the clock was due and a sound
    /// was formed. A loud tick with no sample still moves the clock and
    /// leaves this empty.
    pub crackle: Option<Crackle>,
}

/// The levels a burning tick gives its crackle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Crackle {
    pub volume: f32,
    pub pitch: f32,
    /// The timer is 19 or under: a groan at these levels, before the voice
    /// applies the character's gains. Over 19 the burn's own sample, and
    /// the levels already include [`BurnGains`].
    pub groan: bool,
}

/// The character gains a loud crackle folds in. The volume is
/// `(volume × volume2) × (0.3 + 0.2·draw)`. The pitch is
/// `pitch × (0.8 + 0.2·draw)`. Both products are in double and narrowed
/// once. A groan does not use these.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BurnGains {
    pub volume: f32,
    pub volume2: f32,
    pub pitch: f32,
}

impl Default for BurnGains {
    fn default() -> Self {
        Self {
            volume: 1.0,
            volume2: 1.0,
            pitch: 1.0,
        }
    }
}

/// Whether this tick has the burn's own sample, and the gains a loud
/// crackle folds in.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BurnSound {
    pub loud_sample: bool,
    pub gains: BurnGains,
}

/// One burning tick of a body not off its feet (`condition` & 0xA00 clear),
/// with `yellow`, `red`, the burn and the game clock `now_ms`. `roll` is
/// the shared generator: a due sound draws its levels, then a number below
/// 800 for the next clock. A tick that still has strength reports
/// [`BurnStep::clears_regen`]. [`BurnSound::loud_sample`] is the burn's own
/// sound; without it a timer over 19 still moves the clock and forms no
/// crackle.
pub fn burn_tick(
    condition: u32,
    yellow: f32,
    red: f32,
    burn: &mut Burn,
    now_ms: u64,
    roll: &mut crate::Roll,
    sound: BurnSound,
) -> Option<BurnStep> {
    if condition & 0xa00 != 0 {
        return None;
    }
    let mut sound_at = None;
    let mut crackle = None;
    if burn.strength > 0.0 {
        // The original works these in double and narrows each result.
        burn.timer += (f64::from(burn.strength) * TIMER_GAIN) as f32;
        if burn.timer > TIMER_MOST {
            burn.timer = TIMER_MOST;
        }
        let timer = f64::from(burn.timer);
        let mut yellow = (f64::from(yellow) - timer * YELLOW_LOSS) as f32;
        let old_red = red;
        let mut red = red;
        if burn.timer > SOUND_FROM {
            let loss = timer * RED_LOSS;
            red = (f64::from(red) - loss * loss) as f32;
            if u64::from(burn.next_sound) + u64::from(BURN_SOUND_MS) < now_ms {
                sound_at = Some(now_ms);
                crackle = crackle_of(burn.timer, roll, sound);
                let spread = roll.below(BURN_SOUND_MS);
                burn.next_sound = (now_ms as u32).wrapping_add(spread);
            }
        }
        // The share of red left scales yellow, by the red the tick began
        // with: none worth dividing by takes all of it.
        if old_red > RED_LEAST {
            yellow *= red / old_red;
        } else {
            yellow = 0.0;
        }
        let mut collapse = false;
        if yellow < 0.0 {
            yellow = 0.0;
            red = 0.0;
            collapse = true;
        }
        burn.strength = 0.0;
        return Some(BurnStep {
            yellow,
            red,
            sound_at,
            collapse,
            clears_regen: true,
            crackle,
        });
    }
    burn.timer = (f64::from(burn.timer) - TIMER_FALL) as f32;
    if burn.timer < 0.0 {
        burn.timer = 0.0;
    }
    burn.strength = 0.0;
    Some(BurnStep {
        yellow,
        red,
        sound_at,
        collapse: false,
        clears_regen: false,
        crackle,
    })
}

/// The crackle for a timer that is already due. A timer over 19 with no
/// sample draws nothing. Otherwise two unit draws, then the caller draws
/// the clock.
fn crackle_of(timer: f32, roll: &mut crate::Roll, sound: BurnSound) -> Option<Crackle> {
    // A NaN timer is not over 19, so it groans, as the original's compare does.
    let groan = timer.partial_cmp(&LOUD_FROM) != Some(std::cmp::Ordering::Greater);
    if !groan && !sound.loud_sample {
        return None;
    }
    // The first draw is the pitch's, the second the volume's, as the
    // original calls its generator.
    let pitch_draw = roll.next();
    let volume_draw = roll.next();
    if groan {
        Some(Crackle {
            volume: (f64::from(volume_draw) * DRAW_SPAN + VOLUME_BASE) as f32,
            pitch: (f64::from(pitch_draw) * DRAW_SPAN + PITCH_BASE) as f32,
            groan: true,
        })
    } else {
        let volume = ((f64::from(sound.gains.volume) * f64::from(sound.gains.volume2))
            * (f64::from(volume_draw) * DRAW_SPAN + VOLUME_BASE)) as f32;
        let base = f64::from(pitch_draw) * DRAW_SPAN + PITCH_BASE;
        let pitch = (f64::from(sound.gains.pitch) * base) as f32;
        Some(Crackle {
            volume,
            pitch,
            groan: false,
        })
    }
}

/// Writes one burning tick onto `health`. A tick that still has strength
/// clears the regeneration rate. A body that is down or dying is left as
/// it is.
pub fn apply_burn(
    health: &mut crate::Health,
    condition: u32,
    burn: &mut Burn,
    now_ms: u64,
    roll: &mut crate::Roll,
    sound: BurnSound,
) -> Option<BurnStep> {
    let step = burn_tick(
        condition,
        health.yellow,
        health.red,
        burn,
        now_ms,
        roll,
        sound,
    )?;
    health.yellow = step.yellow;
    health.red = step.red;
    if step.clears_regen {
        health.rate = 0.0;
    }
    Some(step)
}

/// The ticked body's burn, in its upkeep stage: health, the regeneration
/// rate, and a collapse when yellow falls below 0. The blend takes the
/// body's muscle energy (the collapse start reads that energy against 1.2).
#[allow(clippy::type_complexity)]
pub fn burn_upkeep(
    mut commands: Commands,
    ticked: Res<exa_physics::body::TickedCharacter>,
    ticks: Option<Res<exa_physics::body::BodyTicks>>,
    mut fighters: Query<(
        &mut crate::Fighter,
        &mut crate::Condition,
        &mut crate::StateFlags,
        &mut exa_anim::Animator,
        Option<&mut exa_physics::body::Ragdoll>,
        Option<&mut crate::collapse::Hands>,
        &mut Burn,
    )>,
    mut roll: Local<crate::Roll>,
) {
    let Ok((mut fighter, mut condition, mut flags, mut anim, mut body, mut hands, mut burn)) =
        fighters.get_mut(ticked.body)
    else {
        return;
    };
    let now = ticks.as_ref().map_or(0, |t| t.millis());
    let Some(step) = apply_burn(
        &mut fighter.health,
        condition.0,
        &mut burn,
        now,
        &mut roll,
        BurnSound::default(),
    ) else {
        return;
    };
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

    fn plain(condition: u32, yellow: f32, red: f32, burn: &mut Burn, now: u64) -> Option<BurnStep> {
        burn_tick(
            condition,
            yellow,
            red,
            burn,
            now,
            &mut crate::Roll::default(),
            BurnSound::default(),
        )
    }

    /// A strength of 1 raises the timer to 0.03 and takes 1.2e-6 off
    /// yellow; a timer over 1 takes (timer·0.002)² off red and scales
    /// yellow; no strength takes 0.1 off the timer.
    #[test]
    fn the_burn_is_the_originals() {
        let mut b = Burn {
            strength: 1.0,
            ..Burn::default()
        };
        let step = plain(0, 0.9, 1.0, &mut b, 0).unwrap();
        assert!((b.timer - 0.03).abs() < 1e-7);
        assert!((step.yellow - (0.9 - 0.03 * 0.000_04)).abs() < 1e-6);
        assert_eq!(step.red, 1.0);
        assert_eq!(b.strength, 0.0);
        let mut b = Burn {
            strength: 1000.0,
            timer: 1.5,
            ..Burn::default()
        };
        let mut roll = crate::Roll::default();
        let step = burn_tick(0, 1.0, 1.0, &mut b, 1000, &mut roll, BurnSound::default()).unwrap();
        assert_eq!(b.timer, 20.0);
        let red_loss = (20.0f32 * 0.002).powi(2);
        assert!((step.red - (1.0 - red_loss)).abs() < 1e-6);
        assert!(step.sound_at == Some(1000));
        assert!(step.crackle.is_none());
        // Timer 20 and no sample: only the clock's draw below 800. Seed 0
        // steps to 1, and the high half of that times 800 is 0.
        let mut once = crate::Roll::default();
        assert_eq!(once.below(BURN_SOUND_MS), 0);
        assert_eq!(roll.0, once.0);
        assert_eq!(b.next_sound, 1000);
        // Too soon after the draw: 1400 + 800 is not yet past 2000.
        let mut quiet = Burn {
            strength: 1000.0,
            timer: 1.5,
            next_sound: 1400,
        };
        let mut held = crate::Roll(7);
        let step = burn_tick(
            0,
            1.0,
            1.0,
            &mut quiet,
            2000,
            &mut held,
            BurnSound::default(),
        )
        .unwrap();
        assert_eq!(step.sound_at, None);
        assert_eq!(quiet.next_sound, 1400);
        assert_eq!(held.0, 7);
        let mut b = Burn {
            strength: 0.0,
            timer: 0.25,
            ..Burn::default()
        };
        let step = plain(0, 1.0, 1.0, &mut b, 0).unwrap();
        assert!((b.timer - 0.15).abs() < 1e-6);
        assert_eq!((step.yellow, step.red), (1.0, 1.0));
    }

    /// A yellow under 0 collapses (both 0); a down or dying body takes no
    /// burn.
    #[test]
    fn the_burn_collapses_and_stops() {
        let mut b = Burn {
            strength: 0.001,
            timer: 19.9,
            ..Burn::default()
        };
        let step = plain(0, 0.0005, 1.0, &mut b, 0).unwrap();
        assert!(step.collapse);
        assert_eq!((step.yellow, step.red), (0.0, 0.0));
        // A red the tick began with near 0 takes yellow whole, without a
        // collapse, even when the timer's loss already left yellow below 0.
        let mut b = Burn {
            strength: 1.0,
            timer: 19.0,
            ..Burn::default()
        };
        let step = plain(0, 0.0001, 0.0, &mut b, 0).unwrap();
        assert!(!step.collapse);
        assert_eq!(step.yellow, 0.0);
        let mut b = Burn::default();
        assert!(plain(0x200, 1.0, 1.0, &mut b, 0).is_none());
        assert!(plain(0x800, 1.0, 1.0, &mut b, 0).is_none());
    }

    /// A tick that still has strength clears the regeneration rate. A tick
    /// whose strength is gone leaves the rate, and a down body leaves both.
    #[test]
    fn a_damaging_burn_clears_the_regeneration_rate() {
        let mut health = crate::Health {
            yellow: 0.9,
            red: 1.0,
            max: 1.0,
            rate: 1.0,
            heal_pool: 0.0,
        };
        let mut burn = Burn {
            strength: 1.0,
            ..Burn::default()
        };
        let step = apply_burn(
            &mut health,
            0,
            &mut burn,
            0,
            &mut crate::Roll::default(),
            BurnSound::default(),
        )
        .unwrap();
        assert!(step.clears_regen);
        assert_eq!(health.rate, 0.0);
        assert!(health.yellow < 0.9);

        let mut health = crate::Health {
            yellow: 1.0,
            red: 1.0,
            max: 1.0,
            rate: 0.5,
            heal_pool: 0.0,
        };
        let mut burn = Burn {
            strength: 0.0,
            timer: 0.25,
            ..Burn::default()
        };
        let step = apply_burn(
            &mut health,
            0,
            &mut burn,
            0,
            &mut crate::Roll::default(),
            BurnSound::default(),
        )
        .unwrap();
        assert!(!step.clears_regen);
        assert_eq!(health.rate, 0.5);
        assert_eq!(health.yellow, 1.0);

        let mut health = crate::Health {
            yellow: 1.0,
            red: 1.0,
            max: 1.0,
            rate: 0.5,
            heal_pool: 0.0,
        };
        let mut burn = Burn {
            strength: 1.0,
            ..Burn::default()
        };
        assert!(
            apply_burn(
                &mut health,
                0x200,
                &mut burn,
                0,
                &mut crate::Roll::default(),
                BurnSound::default(),
            )
            .is_none()
        );
        assert_eq!(health.rate, 0.5);
        assert_eq!(burn.strength, 1.0);
    }

    /// The upkeep stage writes that cleared rate onto the ticked fighter.
    #[test]
    fn the_upkeep_clears_the_ticked_fighters_rate() {
        let mut app = App::new();
        app.add_systems(Update, burn_upkeep);
        let mut fighter = crate::Fighter::default();
        fighter.health.rate = 1.0;
        let body = app
            .world_mut()
            .spawn((
                fighter,
                crate::Condition::NORMAL,
                crate::StateFlags::default(),
                exa_anim::Animator::new("Default", "Idle"),
                Burn {
                    strength: 1.0,
                    ..Burn::default()
                },
            ))
            .id();
        app.insert_resource(exa_physics::body::TickedCharacter {
            body,
            character: None,
        });
        app.update();
        let health = app.world().get::<crate::Fighter>(body).unwrap().health;
        assert_eq!(health.rate, 0.0);
        assert!(health.yellow < 0.25);
        assert_eq!(app.world().get::<Burn>(body).unwrap().strength, 0.0);
        assert!(app.world().get::<crate::collapse::Collapse>(body).is_none());
    }

    /// Only a part with a bone index above zero adds a fire's dose.
    #[test]
    fn only_a_boned_part_takes_a_dose() {
        let mut burn = Burn::default();
        burn.add_dose(0, 1.0);
        burn.add_dose(-1, 1.0);
        assert_eq!(burn.strength, 0.0);
        burn.add_dose(3, 0.25);
        burn.add_dose(4, 0.5);
        assert_eq!(burn.strength, 0.75);
    }

    /// A timer of 19 groans at volume 0.3 + 0.2×the second draw and pitch
    /// 0.8 + 0.2×the first, then the clock takes a draw below 800. Over 19 a
    /// sample folds the gains in double: volume `(volume × volume2) × base`,
    /// pitch `pitch × base`.
    #[test]
    fn a_crackle_draws_its_volume_and_pitch() {
        let mut burn = Burn {
            strength: 1.0e-6,
            timer: 19.0,
            ..Burn::default()
        };
        let mut roll = crate::Roll::default();
        let step = burn_tick(
            0,
            1.0,
            1.0,
            &mut burn,
            1_000,
            &mut roll,
            BurnSound::default(),
        )
        .unwrap();
        assert!(burn.timer <= 19.0);
        let mut expect = crate::Roll::default();
        let pitch_draw = expect.next();
        let volume_draw = expect.next();
        let spread = expect.below(BURN_SOUND_MS);
        let crackle = step.crackle.unwrap();
        assert!(crackle.groan);
        assert_eq!(crackle.volume, (f64::from(volume_draw) * 0.2 + 0.3) as f32);
        assert_eq!(crackle.pitch, (f64::from(pitch_draw) * 0.2 + 0.8) as f32);
        assert_eq!(burn.next_sound, 1_000u32.wrapping_add(spread));
        assert_eq!(roll.0, expect.0);

        let gains = BurnGains {
            volume: 2.0,
            volume2: 5.0,
            pitch: 3.0,
        };
        let mut burn = Burn {
            strength: 1000.0,
            timer: 19.0,
            ..Burn::default()
        };
        let mut roll = crate::Roll::default();
        let step = burn_tick(
            0,
            1.0,
            1.0,
            &mut burn,
            1_000,
            &mut roll,
            BurnSound {
                loud_sample: true,
                gains,
            },
        )
        .unwrap();
        assert!(burn.timer > 19.0);
        let mut expect = crate::Roll::default();
        let pitch_draw = expect.next();
        let volume_draw = expect.next();
        let _ = expect.below(BURN_SOUND_MS);
        let crackle = step.crackle.unwrap();
        assert!(!crackle.groan);
        assert_eq!(
            crackle.volume,
            ((f64::from(gains.volume) * f64::from(gains.volume2))
                * (f64::from(volume_draw) * 0.2 + 0.3)) as f32
        );
        let base = f64::from(pitch_draw) * 0.2 + 0.8;
        assert_eq!(crackle.pitch, (f64::from(gains.pitch) * base) as f32);
        assert_eq!(roll.0, expect.0);
    }
}

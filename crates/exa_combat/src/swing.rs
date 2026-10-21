//! The swing clock, as the original's attack routine runs it every attack
//! tick: an attack's clip clock runs at a rate its weapon gives, slower at
//! the ends of the swing and faster mid-swing; a swing strength scales it
//! while the swing stalls (released, or struck mid-swing) and the stance
//! returns when the stall ends; holding through 80 % of the clip chains the
//! other side's attack for a fighter with Remise; a strike window opens at
//! 30 % of the clip (40 % for the stab, 50 % for the overhead) and the
//! overhead's and stab's close late in theirs. A fighter's [`Swing`] is its
//! primary striker's, read from the weapon record's swing speed (W15),
//! handling (W16), recoil (W17), swing shaping (W18) and stance blend
//! (W19); its [`SwingState`] is the swing in progress. Rules:
//! `docs/PLAYER.md` (Combat mode) and `docs/formats/RSQ.md` (Motion
//! playback).
//!
//! Times are milliseconds of the game clock (12 ms a body tick); the
//! original's expected swing time and stall lengths assume 80 ticks a
//! second, as it does.

use bevy::prelude::*;
use exa_anim::{Animator, Ease, Motions};
use exa_physics::body::{BODY_HZ, BodyTicks, Ragdoll, TickedCharacter};

use crate::attack::{AttackMotion, Attacker, Side, Slot};
use crate::condition::OFF_BALANCE_BELOW;
use crate::damage::{Outcome, WeaponRecord};
use crate::stance::Stance;
use crate::{Attacking, Blade, Condition, Fighter};

/// The swing's rate: speed × g × 1.125 × the clip clock's base, g = 1 −
/// 0.833·modulation·(d + d²), d = |progress − 0.5|, or 1 − 0.833·modulation
/// before the clip's first key.
const SHAPE: f64 = 0.833;
const MID_SWING: f32 = 0.5;
const RATE: f32 = 1.125;
/// g at either end of the swing: 1 − 0.62475·modulation (0.833 × 0.75).
const END_SHAPE: f64 = 0.624_75;
/// A stab is faster: g ×1.2, and early in it (progress under 0.4) also ×
/// (1.2·progress + 0.5).
const STAB_SPEED: f64 = 1.2;
const STAB_EARLY: f64 = 0.4;
const STAB_FROM: f64 = 0.5;
/// Under this share of its most energy a fighter's swing stops; under its
/// most the rate becomes the share squared.
const SWING_ENERGY: f64 = 0.8;
/// The original's expected swing time and stall lengths assume this many
/// ticks a second.
const ASSUMED_HZ: f32 = 80.0;
const MS: f32 = 1000.0;
/// A released swing's strength drains by max(1 − 0.05·|v|, 0.1)/W17 × 0.03
/// a tick, at least 0.01 (v the striker's tracked speed, cm a tick).
const RELEASE_SPEED: f64 = 0.05;
const RELEASE_FLOOR: f64 = 0.1;
const RELEASE_SCALE: f64 = 0.03;
const RELEASE_MIN: f64 = 0.01;
/// A released swing may be renewed (Feint) after (2f + 1) ×
/// (round(100·W17) + 100) ms, f = smoothstep(progress / 0.7) up to 0.7 of
/// the clip, else 1.
const RENEW_PER_RECOIL: f32 = 100.0;
const RENEW_BASE: i64 = 100;
const RENEW_RAMP_TO: f64 = 0.7;
/// 1/0.7, as the original stores it.
const RENEW_RAMP: f64 = 1.428_571_428_571_428_6;
/// A struck attacker's swing stalls by 4·D·f − (1 − D)·min(0.005·s, 1)·W17
/// a tick (D the daze, f its second factor, s the striker's tracked speed
/// squared), when that is at least 0.001; it lasts 80·P/stall ms at most.
const DAZE_STALL: f32 = 4.0;
const DAZE_REACH: f64 = 0.005;
const DAZE_STALL_FROM: f64 = 0.001;
const DAZE_STALL_LENGTH: f32 = 80.0;
/// The daze's second factor is 0.25 over the fighter's most health (1 for a
/// human).
const DAZE_HEALTH: f32 = 0.25;
/// Below this the original counts a strength as none.
const EPSILON: f32 = 1.192_09e-7;
/// The window region: the trackers run past 30 % of a clip, or within the
/// first 25 % of a chained one (its carried window).
const REGION_FROM: f64 = 0.3;
const CARRIED_UNTIL: f32 = 0.25;
/// Past 70 % of the clip in the region the swing is late.
const LATE_FROM: f32 = 0.7;
/// The strike window opens past these shares of the clip.
const OPEN_SIDE: f32 = 0.3;
const OPEN_STAB: f32 = 0.4;
const OPEN_OVERHEAD: f32 = 0.5;
/// The overhead's window closes past 90 % unarmed (weapon class 0) and
/// 95 % otherwise, or at once past 85 % once the swing no longer falls
/// faster than 3 cm a tick.
const CLOSE_OVERHEAD_EMPTY: f32 = 0.9;
const CLOSE_OVERHEAD: f32 = 0.95;
const OVERHEAD_LATE: f32 = 0.85;
const OVERHEAD_FALL: f32 = -3.0;
const ONE_HUNDREDTH: f32 = 0.01;
/// The stab's window closes past 80 %, or past 95 % while the thrust
/// still drives on at 2.5 cm a tick or more along the aim.
const CLOSE_STAB: f32 = 0.8;
const CLOSE_STAB_DRIVEN: f32 = 0.95;
const THRUST_DRIVE: f32 = 2.5;
/// Holding through 80 % of a side attack chains the other side's.
const CHAIN_AT: f32 = 0.8;
/// A chain starts with round(6·W18) + 2 frames of blend.
const CHAIN_BLEND_PER_SHAPE: f32 = 6.0;
const CHAIN_BLEND_BASE: u32 = 2;
/// The striker tracker: up to 80 % of the clip a speed is capped at 50 cm a
/// tick, after it at the slot's best; the slot moves a quarter of the way
/// toward it each tick; its squared speed is kept under 1000.
const TRACK_CAP: f32 = 50.0;
const TRACK_LATE: f32 = 0.8;
const TRACK_SHARE: f32 = 0.25;
const TRACK_MAX2: f32 = 1000.0;
/// While attacking the body turns at (1 − 0.6·enc)·(1 − 0.25·progress)·B·
/// handling·P/scale a tick, B the base turn rate (240° a second: 4π/3
/// over 83 ticks, in double precision).
const TURN_ENCUMBRANCE: f64 = 0.6;
const TURN_PROGRESS: f32 = 0.25;
const BASE_TURN: f32 = (4.188_790_204_786_391_f64 / 83.0) as f32;
/// Off its balance (under 0.6) the body tick takes its falls branch, not
/// the attack branch: an attack's clip then runs 1.25 × base a tick.
const OFF_BALANCE_RATE: f32 = 1.25;
/// Weapon class bit of the fists, claws and two weapons, and the claws'
/// class.
const ONE_HAND_STRIKES: u8 = 0x80;
const CLAWS: u8 = 0xC0;
/// Centimetres per metre.
const CM: f32 = 100.0;

/// The original's attack state word, kept with a fighter's swing.
pub mod flags {
    /// The strike window opened this swing.
    pub const WINDOW_OPENED: u32 = 0x1;
    /// The swing stalls: its strength drains.
    pub const STALLING: u32 = 0x2;
    /// A blow landed and the swing cleaves on.
    pub const CLEAVING: u32 = 0x4;
    /// The swing was released (its request dropped).
    pub const RELEASED: u32 = 0x10;
    /// The striker recoiled from a block.
    pub const RECOILED: u32 = 0x20;
    /// Past 70 % of the clip in the window region.
    pub const LATE: u32 = 0x80;
    /// A chained (Remise) swing, its window carried over.
    pub const CONTINUATION: u32 = 0x1_0000;
    /// A chained swing past 25 % of its clip: no further chain.
    pub const CONTINUED: u32 = 0x2_0000;
    /// Kept while the fighter is not attacking (with the released and
    /// recoiled bits).
    pub const KEPT_IDLE: u32 = 0x4_0030;
    /// The fighter lacks Remise: no chain.
    pub const NO_CHAIN: u32 = 0x8_0000;
    /// What a wound sets on the striker's swing: cleaving and recoiled, with
    /// the swing's clash (0x100), block (0x200) and wound (0x400) bits.
    pub const WOUNDED: u32 = 0x724;
    /// A shield's sound rang this swing (the block's bit of
    /// [`WOUNDED`]): its sound and cooldown draw come once a swing.
    pub const SHIELD_RUNG: u32 = 0x200;
    /// The weapon's hit sound rang on a weapon or a shield this swing.
    pub const WEAPON_RUNG: u32 = 0x1000;
    /// Held back after a wound by the edge: stalling, released, chained
    /// past a quarter or without Remise; such a swing follows the wound
    /// through instead of cleaving on.
    pub const HOLDS_BACK: u32 = 0xA_0012;
    /// No release over a recoil, a landed blow or an earlier release.
    pub const NO_RELEASE: u32 = CLEAVING | RELEASED | RECOILED;
    /// No chain while released, stalling or chained, or without Remise.
    pub const NO_REMISE: u32 = STALLING | RELEASED | CONTINUED | NO_CHAIN;
}

/// The pace of a fighter's swings: its primary striker's.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct Swing {
    /// The weapon's swing speed (W15).
    pub speed: f32,
    /// How much slower the swing is at its ends: the weapon's swing shaping
    /// (W18; 1 − its balance unencumbered).
    pub modulation: f32,
    /// The weapon's recoil (W17): a released or struck swing stalls by it.
    pub recoil: f32,
    /// The weapon's handling (W16): the body's turn while attacking.
    pub handling: f32,
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
        recoil: 0.1,
        handling: 0.8,
        recovery: 0,
    };

    /// The pace of a weapon, from its record as held.
    pub fn of(record: &WeaponRecord) -> Swing {
        Swing {
            speed: record.swing_speed,
            modulation: record.swing_shaping,
            recoil: record.recoil,
            handling: record.handling,
            recovery: record.blend.max(0) as u32,
        }
    }

    /// g at `progress`: 1 − 0.833·modulation·(d + d²), d = |progress −
    /// 0.5|, or 1 − 0.833·modulation at or before the clip's first key.
    fn shape(&self, progress: f32) -> f32 {
        let m = f64::from(self.modulation);
        if progress <= 0.0 {
            (1.0 - m * SHAPE) as f32
        } else {
            let d = (progress - MID_SWING).abs();
            let d = d + d * d;
            (1.0 - m * (f64::from(d) * SHAPE)) as f32
        }
    }

    /// Frames an attack's clip clock advances this tick before any stall or
    /// tiredness, at the original's `progress` (clock less blend over the
    /// last key's time), with the clock's `base` rate and the character's
    /// `feral` level (the record's, 0..1): speed × g × 1.125 ×
    /// base, a `stab` ×1.2 (and early in it × (1.2·progress + 0.5)), a
    /// feral character's × ((1 − f)/2 + f·smoothstep(progress) + 1/2).
    pub fn rate(&self, stab: bool, progress: f32, base: f32, feral: f32) -> f32 {
        let mut g = self.shape(progress);
        if stab {
            if f64::from(progress) < STAB_EARLY {
                g = ((f64::from(progress) * STAB_SPEED + STAB_FROM) * f64::from(g)) as f32;
            }
            g = (f64::from(g) * STAB_SPEED) as f32;
        }
        if feral > 0.0 {
            let ramp = if progress > 0.0 {
                smoothstep(progress) * feral + 0.5
            } else {
                0.5
            };
            g *= (1.0 - feral) * 0.5 + ramp;
        }
        self.speed * g * RATE * base
    }

    /// Frames an attack's clip clock advances this tick with nothing
    /// stalling it, at the fighter's `energy` share of its most: the
    /// [`Swing::rate`] of the clip `action` (an action index), or tired
    /// ([`paced`]).
    pub fn frames(&self, action: Option<usize>, progress: f32, base: f32, energy: f32) -> f32 {
        let stab = action == Some(AttackMotion::Stab.index());
        paced(self.rate(stab, progress, base, 0.0), energy)
    }

    /// The milliseconds the swing is expected to take from `progress` to
    /// its end, a clip whose last key is at `last_key` frames, its clock's
    /// `base` rate (at 80 ticks a second, by the mean g over what is left,
    /// as the original reckons it): the cap of every stall.
    pub fn expected_ms(&self, progress: f32, last_key: f32, base: f32) -> u64 {
        let ends = (1.0 - f64::from(self.modulation) * END_SHAPE) as f32;
        let d = (progress - MID_SWING).abs();
        let d = d * d + d;
        let now = (1.0 - f64::from(self.modulation) * (f64::from(d) * SHAPE)) as f32;
        let mean = if progress >= MID_SWING {
            (now + ends) * 0.5
        } else {
            ((MID_SWING - progress) * (now + 1.0) + (ends + 1.0) * 0.5) / (1.0 - progress) * 0.5
        };
        let per_tick = self.speed * mean * RATE * base;
        let ms = (1.0 - progress) * last_key / (ASSUMED_HZ * per_tick) * MS;
        whole(ms.max(0.0))
    }

    /// How fast a released swing's strength drains a tick, from `speed2`,
    /// its striker's tracked speed squared (cm² a tick²).
    pub fn release_stall(&self, speed2: f32) -> f32 {
        let s = if speed2 > EPSILON {
            (1.0 - f64::from(speed2.sqrt()) * RELEASE_SPEED) as f32
        } else {
            1.0
        };
        let s = if f64::from(s) < RELEASE_FLOOR {
            RELEASE_FLOOR as f32
        } else {
            s
        };
        let s = (f64::from(s / self.recoil) * RELEASE_SCALE) as f32;
        if f64::from(s) < RELEASE_MIN {
            RELEASE_MIN as f32
        } else {
            s
        }
    }

    /// How long (ms) after its release a swing of `motion` (none: not an
    /// attack's clip) released at `progress` may be renewed by Feint;
    /// `continued` past the carried window of a chain.
    pub fn renew_after_ms(
        &self,
        progress: f32,
        motion: Option<AttackMotion>,
        continued: bool,
    ) -> u64 {
        let f = if f64::from(progress) > RENEW_RAMP_TO || motion.is_none() || continued {
            1.0
        } else {
            smoothstep((f64::from(progress) * RENEW_RAMP) as f32)
        };
        let base = whole_signed(RENEW_PER_RECOIL * self.recoil) + RENEW_BASE;
        whole((f * 2.0 + 1.0) * base as f32)
    }

    /// The blend (frames) a chained (Remise) attack starts with:
    /// round(6·W18) + 2.
    pub fn chain_blend(&self) -> u32 {
        whole(self.modulation * CHAIN_BLEND_PER_SHAPE) as u32 + CHAIN_BLEND_BASE
    }

    /// The body's turn a tick (rad) while it attacks, at `progress` with
    /// swing `strength` P, its `encumbrance` and body `scale`:
    /// (1 − 0.6·enc)·(1 − 0.25·progress)·B·handling·P / scale.
    pub fn turn_rate(&self, progress: f32, strength: f32, encumbrance: f32, scale: f32) -> f32 {
        let turn = (1.0 - progress * TURN_PROGRESS) * (BASE_TURN * self.handling) * strength;
        ((1.0 - f64::from(encumbrance) * TURN_ENCUMBRANCE) * f64::from(turn) / f64::from(scale))
            as f32
    }
}

/// The daze's second factor for a struck attacker's stall: 0.25 over its
/// most health, halved under a potion (×1 for every blow until Security,
/// a perk, scales it).
pub fn daze_factor(max_health: f32, potion: bool) -> f32 {
    let f = DAZE_HEALTH / max_health.max(f32::EPSILON);
    if potion { f * 0.5 } else { f }
}

/// The original's smoothstep, (3 − 2x)·x², unclamped.
fn smoothstep(x: f32) -> f32 {
    (3.0 - 2.0 * x) * x * x
}

/// A float rounded as the original rounds (to the nearest, ties to even),
/// never negative.
fn whole(x: f32) -> u64 {
    whole_signed(x).max(0) as u64
}

fn whole_signed(x: f32) -> i64 {
    x.round_ties_even() as i64
}

/// A swing `rate` paced by the fighter's `energy` share of its most: none
/// under 0.8 (compared in double precision), the share squared in place
/// of the rate under 1.
pub fn paced(rate: f32, energy: f32) -> f32 {
    if f64::from(energy) < SWING_ENERGY {
        0.0
    } else if energy < 1.0 {
        energy * energy
    } else {
        rate
    }
}

/// A striker's tracked velocity through the window region, as the
/// original's swing tracker keeps it per held item.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Tracker {
    /// The smoothed velocity (cm a tick).
    pub velocity: Vec3,
    /// Its speed squared, at most the latest speed's and 1000.
    pub speed2: f32,
    /// The best speed squared up to 80 % of the clip, then only shrinking.
    pub best: f32,
}

impl Tracker {
    /// One tick with the striker's tip velocity `v` (cm a tick) at
    /// `progress`.
    pub fn track(&mut self, v: Vec3, progress: f32) {
        let mut v = v;
        let mut s = v.length_squared();
        if progress <= TRACK_LATE {
            if s > TRACK_CAP * TRACK_CAP {
                v *= TRACK_CAP / s.sqrt();
                s = TRACK_CAP * TRACK_CAP;
            }
        } else if s > self.best {
            v *= (self.best / s).sqrt();
            s = self.best;
        }
        self.velocity += (v - self.velocity) * TRACK_SHARE;
        self.speed2 = self.velocity.length_squared();
        if progress <= TRACK_LATE || self.speed2 < self.best {
            self.best = self.speed2;
        }
        if s < self.speed2 {
            self.speed2 = s;
        }
        if self.speed2 > TRACK_MAX2 {
            self.speed2 = TRACK_MAX2;
        }
    }

    /// Cleared: outside the window region, and when an attack starts.
    pub fn reset(&mut self) {
        self.velocity = Vec3::ZERO;
        self.speed2 = 0.0;
    }
}

/// Where a fighter's clip stands at a moment: what a release, a daze stall
/// and the swing routine read of it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SwingAt {
    /// The game clock (ms).
    pub now_ms: u64,
    /// The attack whose clip plays, if any.
    pub motion: Option<AttackMotion>,
    /// The clip's progress, (clock − blend) / its last key's time.
    pub progress: f32,
    /// The clip's last key's time (frames).
    pub last_key: f32,
    /// The clip clock's base rate (the race's).
    pub base: f32,
}

impl SwingAt {
    /// Where `anim`'s clip stands at `now_ms`, its clock's base rate
    /// `base`; none when its clip is not in `motions`.
    pub fn of(now_ms: u64, anim: &Animator, motions: &Motions, base: f32) -> Option<Self> {
        let clip = motions.clip(anim, &anim.action)?;
        Some(Self {
            now_ms,
            motion: AttackMotion::of_action(&anim.action).filter(|_| !anim.looping),
            progress: anim.clip_progress(clip),
            last_key: clip.keys.last()?.time,
            base,
        })
    }
}

/// What the swing routine reads of a fighter's attack tick.
#[derive(Clone, Copy, Debug)]
pub struct SwingTick {
    /// Where its clip stands.
    pub at: SwingAt,
    pub motion: AttackMotion,
    /// The character's feral level (0..1).
    pub feral: f32,
    /// Its energy's share of its most.
    pub energy: f32,
    pub weapon_class: u8,
    /// Each held slot's striker tip velocity (cm a tick), if it has one.
    pub strikers: [Option<Vec3>; 2],
    /// The horizontal direction it aims along (unit).
    pub aim: Vec3,
}

/// What a swing tick asks of the fighter's clip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwingEvent {
    /// The attack plays on.
    Plays,
    /// Remise: the other side's attack chains on.
    Chains,
    /// The stall is over: the stance returns.
    Ends,
}

/// The swing in progress on a fighter (on the body model root): its state
/// word ([`flags`]), its strength P and how fast a stall drains it, when
/// the stall ends and when a released swing may be renewed, the clip rate
/// this tick, its strikers' window and trackers.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct SwingState {
    pub flags: u32,
    /// The swing strength P: 1 at every start; a stall drains it.
    pub strength: f32,
    /// How much P loses a tick while the swing stalls.
    pub stall: f32,
    /// The game clock (ms) at which the stall ends and the stance returns;
    /// before it a released swing keeps the executor busy.
    pub end_ms: u64,
    /// The game clock (ms) from which Feint may renew a released swing.
    pub renew_ms: u64,
    /// Frames the attack's clip clock advances this tick.
    pub rate: f32,
    /// Each held slot's striker has the strike window open.
    pub open: [bool; 2],
    /// Each held slot's striker tracker.
    pub tracker: [Tracker; 2],
    /// The recoil's rest, kept until the stance returns: its blend and the
    /// arm stiffness the stance starts with.
    pub recoil: Option<RecoilRest>,
}

impl Default for SwingState {
    fn default() -> Self {
        Self {
            flags: 0,
            strength: 1.0,
            stall: 0.0,
            end_ms: 0,
            renew_ms: 0,
            rate: 0.0,
            open: [false; 2],
            tracker: [Tracker::default(); 2],
            recoil: None,
        }
    }
}

impl SwingState {
    /// A fresh attack starts: full strength, no stall, the window shut,
    /// the trackers cleared; no chain without Remise (`remise`) for a side
    /// attack.
    pub fn start(&mut self, side: bool, remise: bool) {
        self.flags = if side && !remise { flags::NO_CHAIN } else { 0 };
        self.strength = 1.0;
        self.stall = 0.0;
        self.open = [false; 2];
        self.recoil = None;
        for t in &mut self.tracker {
            t.reset();
        }
    }

    /// A chained (Remise) attack starts: full strength, no stall, its
    /// window carried over.
    pub fn chain(&mut self) {
        self.flags |= flags::CONTINUATION;
        self.strength = 1.0;
        self.stall = 0.0;
        self.recoil = None;
    }

    pub fn has(&self, mask: u32) -> bool {
        self.flags & mask != 0
    }

    /// The executor is busy until the stall ends.
    pub fn busy(&self, now_ms: u64) -> bool {
        now_ms < self.end_ms
    }

    /// The faster tracked speed squared of the two strikers: the first's,
    /// or the second's when larger (an attacking fighter always holds
    /// something).
    fn tracked2(&self) -> f32 {
        self.tracker[0].speed2.max(self.tracker[1].speed2)
    }

    /// The request was dropped during the attack (`t`): the swing stalls
    /// from full strength by [`Swing::release_stall`], until the stance
    /// returns after 1000/(80 × stall) ms, at most the expected rest of the
    /// swing; Feint may renew it after [`Swing::renew_after_ms`].
    pub fn release(&mut self, swing: &Swing, t: &SwingAt) {
        self.strength = 1.0;
        self.recoil = None;
        self.flags |= flags::STALLING | flags::RELEASED;
        self.stall = swing.release_stall(self.tracker[0].speed2);
        let lasts = whole(MS / (ASSUMED_HZ * self.stall));
        let expected = swing.expected_ms(t.progress, t.last_key, t.base);
        self.end_ms = t.now_ms + lasts.min(expected);
        self.renew_ms =
            t.now_ms + swing.renew_after_ms(t.progress, t.motion, self.has(flags::CONTINUED));
    }

    /// The attacker was dazed by `daze` (D, at most 0.9) with the daze's
    /// second factor `factor` ([`daze_factor`]) during the attack (`t`):
    /// r = 4·D·max(f, 0) − (1 − D)·min(0.005·s, 1)·W17 starts the stall
    /// (or adds to it, up to 1) when at least 0.001; the stance returns
    /// after 80·P/stall ms, at most the expected rest of the swing. Returns
    /// the stall added.
    pub fn daze_stall(
        &mut self,
        swing: &Swing,
        t: &SwingAt,
        daze: f32,
        factor: f32,
    ) -> Option<f32> {
        if self.strength < EPSILON {
            return None;
        }
        self.recoil = None;
        let reach = ((f64::from(self.tracked2()) * DAZE_REACH) as f32).min(1.0);
        let r = daze * DAZE_STALL * factor.max(0.0) - (1.0 - daze) * (reach * swing.recoil);
        if f64::from(r) < DAZE_STALL_FROM {
            return None;
        }
        if self.has(flags::STALLING) {
            self.stall = (self.stall + r).min(1.0);
        } else {
            self.stall = r;
            self.flags |= flags::STALLING;
        }
        let lasts = whole(self.strength / self.stall * DAZE_STALL_LENGTH);
        let expected = swing.expected_ms(t.progress, t.last_key, t.base);
        self.end_ms = t.now_ms + lasts.min(expected);
        Some(r)
    }

    /// One attack tick of the swing routine, in the original's order: the
    /// trackers in the window region (late past 70 %); the rate; a chained
    /// swing's carried window closing past 25 %; the overhead's and stab's
    /// windows closing; the window opening (with the fists, claws or two
    /// weapons only the motion's hand, which becomes the primary striker);
    /// Remise past 80 % unless released, stalling, chained or without it;
    /// the stall draining P (the stance returns once its end has passed),
    /// the rate × P; tiredness ([`paced`]).
    pub fn tick(&mut self, swing: &Swing, t: &SwingTick, primary: &mut Option<Slot>) -> SwingEvent {
        let p = t.at.progress;
        let region = if t.motion.is_side() && self.has(flags::CONTINUATION) {
            p < CARRIED_UNTIL
        } else {
            f64::from(p) > REGION_FROM
        };
        if region {
            if p > LATE_FROM {
                self.flags |= flags::LATE;
            }
            for (tracker, v) in self.tracker.iter_mut().zip(t.strikers) {
                if let Some(v) = v {
                    tracker.track(v, p);
                }
            }
        } else {
            for tracker in &mut self.tracker {
                tracker.reset();
            }
        }
        let mut rate = swing.rate(t.motion == AttackMotion::Stab, p, t.at.base, t.feral);
        if self.has(flags::CONTINUATION) && p > CARRIED_UNTIL {
            self.flags = flags::CONTINUED;
            self.open = [false; 2];
        }
        let opens_at = match t.motion {
            AttackMotion::Overhead => {
                let closes_at = if t.weapon_class == 0 {
                    CLOSE_OVERHEAD_EMPTY
                } else if p <= OVERHEAD_LATE || self.tracker[0].velocity.y <= OVERHEAD_FALL {
                    CLOSE_OVERHEAD
                } else {
                    p - ONE_HUNDREDTH
                };
                if closes_at < p {
                    self.open = [false; 2];
                }
                OPEN_OVERHEAD
            }
            AttackMotion::Stab => {
                if p > CLOSE_STAB {
                    let drive = self.tracker[0].velocity.dot(t.aim);
                    if drive < THRUST_DRIVE || p > CLOSE_STAB_DRIVEN {
                        self.open = [false; 2];
                    }
                }
                OPEN_STAB
            }
            _ => OPEN_SIDE,
        };
        if opens_at < p && !self.has(flags::WINDOW_OPENED) {
            self.flags |= flags::WINDOW_OPENED;
            if t.weapon_class & ONE_HAND_STRIKES == 0 {
                self.open = [true; 2];
            } else {
                let (hand, both) = match t.motion {
                    AttackMotion::Overhead => (Slot::First, t.weapon_class == CLAWS),
                    AttackMotion::Main(_) | AttackMotion::Stab => (Slot::First, false),
                    AttackMotion::Other(_) => (Slot::Second, false),
                };
                *primary = Some(hand);
                self.open[hand.index()] = true;
                if both {
                    self.open[Slot::Second.index()] = true;
                }
            }
        }
        let mut event = SwingEvent::Plays;
        if !self.has(flags::NO_REMISE) && t.motion.is_side() && p > CHAIN_AT {
            event = SwingEvent::Chains;
        }
        if self.has(flags::STALLING) {
            self.strength = (self.strength - self.stall).max(0.0);
            if self.end_ms < t.at.now_ms {
                // The stance's start shuts the window, as every clip start.
                self.flags = 0;
                self.open = [false; 2];
                event = SwingEvent::Ends;
            }
            rate *= self.strength;
        }
        rate = paced(rate, t.energy);
        self.rate = rate;
        event
    }
}

/// The rate of an attack's clip for a body of `balance` and clock `base`
/// when it is off its balance (under 0.6, double precision): its falls
/// branch runs instead of the attack branch and the clip runs 1.25 × base;
/// none on its balance.
pub fn off_balance_rate(balance: f32, base: f32) -> Option<f32> {
    (f64::from(balance) < OFF_BALANCE_BELOW).then_some(OFF_BALANCE_RATE * base)
}

/// Whether a body of `condition` takes the falls branch off its balance,
/// whatever its condition word says otherwise: only death, a collapse, a
/// shock and a get-up have branches that come first (as the posture
/// decision, [`crate::condition::posture`], orders them).
fn falls_off_balance(condition: Condition) -> bool {
    !(condition.dying()
        || condition.has(Condition::SHOCK_MASK)
        || condition == Condition::GETTING_UP)
}

/// The side a chain from `motion` goes on: the other one.
fn chained_side(motion: AttackMotion) -> Side {
    match motion {
        AttackMotion::Main(_) => Side::Other,
        _ => Side::Main,
    }
}

/// Each body tick, in the ticked character's attack stage (after its
/// posture stage, before its health bookkeeping and its clip's advance),
/// the attack branch of an attacking fighter (its condition attacking, its
/// attack's clip playing, on its balance): the last attack tick stamped,
/// then the swing routine ([`SwingState::tick`]) with its strikers' tip
/// velocities; a Remise chain starts the other side's attack, a stall's
/// end its stance as it is now. Off its balance (under 0.6, double
/// precision) the falls branch runs instead, also for an attack started
/// there under another condition, and the clip runs 1.25 × base.
#[allow(clippy::type_complexity)]
pub(crate) fn run_swings(
    mut commands: Commands,
    ticked: Res<TickedCharacter>,
    ticks: Res<BodyTicks>,
    motions: Option<Res<Motions>>,
    blades: Query<&Blade>,
    mut fighters: Query<(
        &mut Animator,
        &mut Attacker,
        &mut SwingState,
        &mut Attacking,
        &Condition,
        &Fighter,
        Option<&Swing>,
        Option<&mut Ragdoll>,
        &GlobalTransform,
        Option<&Stance>,
    )>,
) {
    let Some(motions) = motions else { return };
    let now = ticks.millis();
    let entity = ticked.body;
    let Ok((
        mut anim,
        mut attacker,
        mut state,
        mut attacking,
        condition,
        fighter,
        swing,
        mut body,
        gt,
        stance,
    )) = fighters.get_mut(entity)
    else {
        return;
    };
    if anim.looping || AttackMotion::of_action(&anim.action).is_none() {
        return;
    }
    // The body's clip-rate base, the falls branch's rate and its energy, read
    // before its frame is handed on.
    let (base, off_balance, energy) = body.as_deref().map_or((1.0, None, 1.0), |b| {
        let drive = b.drive.as_ref();
        let base = drive.map_or(1.0, |d| d.race.clip_rate);
        (
            base,
            drive.and_then(|d| off_balance_rate(d.balance().value, base)),
            drive.map_or(1.0, |d| d.energy_share()),
        )
    });
    // The falls branch sets the clip's rate whatever the condition (an
    // attack started off balance is not attacking yet), so the clip never
    // runs at a rate left from an earlier swing.
    if falls_off_balance(*condition)
        && let Some(rate) = off_balance
    {
        state.rate = rate;
        return;
    }
    if *condition != Condition::ATTACKING {
        return;
    }
    let Some(at) = SwingAt::of(now, &anim, &motions, base) else {
        return;
    };
    attacker.last_tick_ms = Some(now);
    let swing = swing.copied().unwrap_or(Swing::FISTS);
    let mut strikers = [None; 2];
    for blade in blades.iter().filter(|b| b.wielder == entity) {
        strikers[blade.slot.index()] = Some(blade.tip_velocity * CM / BODY_HZ);
    }
    let facing = gt.rotation() * Vec3::Z;
    let t = SwingTick {
        at,
        motion: attacking.motion,
        feral: fighter.stats.feral,
        energy,
        weapon_class: attacking.weapon_class,
        strikers,
        aim: facing.with_y(0.0).normalize_or_zero(),
    };
    match state.tick(&swing, &t, &mut attacker.primary) {
        SwingEvent::Plays => {}
        SwingEvent::Chains => {
            let side = chained_side(attacking.motion);
            let motion = side.motion(attacker.pick(side, false));
            let stance = crate::stance_of(&anim);
            anim.play_once(motion.action(), &stance);
            anim.blended(swing.chain_blend(), Ease::Smooth);
            state.chain();
            attacking.motion = motion;
            attacking.struck.clear();
            debug!(
                "{entity:?} chains {} at {now} ms (blend {})",
                motion.action(),
                swing.chain_blend()
            );
        }
        SwingEvent::Ends => {
            // Its stance as it is now (the clip's follow-up without one). A
            // recoil's rest starts it: its own blend and both arms' stiffness.
            let stance = stance.map_or_else(|| crate::stance_of(&anim), |s| s.action().to_string());
            let rest = state.take_recoil();
            let blend = rest.map_or(2 * swing.recovery, |r| r.blend);
            anim.start(&stance, true, None, blend, Ease::Smooth);
            if let Some(rest) = rest
                && let Some(body) = body.as_deref_mut()
            {
                body.set_arm_stiffness(rest.stiffness);
            }
            commands.entity(entity).remove::<Attacking>();
            debug!(
                "{entity:?} stall over at {now} ms: {stance} (blend {blend}{})",
                rest.map_or("", |_| ", recoil")
            );
        }
    }
}

/// A class word of exactly this value is two weapons. A recoil and a
/// follow-through compare the whole word, and skip the rest of the write
/// when it matches and a continuation is already set, or the clock widens
/// above 0.4 and a counter starts.
const DUAL_WIELD: u32 = 0x90;
/// The clock compare is a double. A float 0.4 widens to slightly more.
const COUNTER_FROM: f64 = 0.4;
/// Both arm stiffnesses a recoil writes, and the held scale passed with them.
const ARM_STIFFNESS: f32 = 0.2;
const HELD_SCALE: f32 = 1.0;
const RECOIL_BLEND: u32 = 40;
const RECOIL_TAIL_MS: i32 = 200;
/// Drive is widened, multiplied by this double, narrowed, then by 400.
const DRIVE_SHARE: f64 = 0.1;
const END_SCALE: f32 = 400.0;
/// Gain, recoil and speed are widened, then multiplied by this double.
const FOLLOW_SHARE: f64 = 0.15;
/// One minus the strength is widened, then multiplied by this double.
const STALL_SHARE: f64 = 0.05;
/// A body zone that is not strictly above this sets the strength to 0.
const ZONE_FLOOR: f32 = f32::from_bits(0x33ff_ffd7);

/// What a blocked striker brings to a recoil.
#[derive(Clone, Copy, Debug)]
pub struct RecoilStrike {
    /// The weapon class word. Two weapons are [`DUAL_WIELD`].
    pub class_word: u32,
    pub flags: u32,
    /// The attack clock, 0 to 1.
    pub clock: f32,
    /// An immediate counter is starting.
    pub counter: bool,
    /// Strike speed times the weapon's recoil.
    pub drive: f32,
    /// The game clock, in milliseconds.
    pub now_ms: i32,
}

/// The stall end, the stance blend and the arm stiffness a recoil writes
/// when the dual-wield skip does not take it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RecoilRest {
    pub end_ms: i32,
    pub blend: u32,
    /// Both arms.
    pub stiffness: f32,
    /// The held scale passed with the two stiffnesses.
    pub held: f32,
}

/// Strength and stall rate are always one half, and the released bit is
/// always cleared. [`RecoilRest`] is absent when two weapons skip the rest.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Recoil {
    pub strength: f32,
    pub stall: f32,
    pub flags: u32,
    pub rest: Option<RecoilRest>,
}

/// A blocked striker recoils. Strength and the stall rate become 0.5 and
/// the released bit clears. Two weapons that are already continuing, or
/// whose clock widens above 0.4 while a counter starts, stop there. Any
/// other swing sets stalling, cleaving and recoiled, and the stance returns
/// after the widened drive times 0.1 times 400, rounded half to even, plus
/// 200 ms.
pub fn recoil(strike: &RecoilStrike) -> Recoil {
    let cleared = strike.flags & !flags::RELEASED;
    if dual_wield_holds(
        strike.class_word,
        strike.flags,
        strike.clock,
        strike.counter,
    ) {
        return Recoil {
            strength: 0.5,
            stall: 0.5,
            flags: cleared,
            rest: None,
        };
    }
    let scaled = (f64::from(strike.drive) * DRIVE_SHARE) as f32;
    Recoil {
        strength: 0.5,
        stall: 0.5,
        flags: cleared | flags::STALLING | flags::CLEAVING | flags::RECOILED,
        rest: Some(RecoilRest {
            end_ms: (scaled * END_SCALE).round_ties_even() as i32 + strike.now_ms + RECOIL_TAIL_MS,
            blend: RECOIL_BLEND,
            stiffness: ARM_STIFFNESS,
            held: HELD_SCALE,
        }),
    }
}

/// What a wound brings to the striker's follow-through.
#[derive(Clone, Copy, Debug)]
pub struct FollowStrike {
    pub class_word: u32,
    pub flags: u32,
    pub clock: f32,
    pub counter: bool,
    /// The caller's gain. A hard blow and a light one choose different values.
    pub gain: f32,
    /// The weapon's recoil.
    pub recoil: f32,
    /// Strike speed, centimetres a tick.
    pub speed: f32,
    /// Body zone, or how far into the tip the edge sits.
    pub zone: f32,
    /// The blow landed on the edge.
    pub edge: bool,
    pub now_ms: i32,
}

/// The strength, stall, flags and end a follow-through writes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FollowThrough {
    pub strength: f32,
    pub stall: f32,
    pub flags: u32,
    pub end_ms: i32,
}

/// The striker follows a wound through. Two weapons that are continuing, or
/// whose clock widens above 0.4 while a counter starts, write nothing.
/// Otherwise the scale is the widened gain times recoil times speed times
/// 0.15, floored at 0. A body zone at or under the floor sets strength 0,
/// and a larger zone uses its square root times the scale. An edge uses
/// one minus the zone, times the scale, plus the zone. Strength caps at 1.
/// The stall is the widened one minus strength times 0.05. Stalling and
/// cleaving are set. The end rounds the scale times 400 half to even (the
/// conversion in the default rounding) and adds now.
pub fn follow_through(strike: &FollowStrike) -> Option<FollowThrough> {
    if dual_wield_holds(
        strike.class_word,
        strike.flags,
        strike.clock,
        strike.counter,
    ) {
        return None;
    }
    let scaled = follow_scale(strike.gain, strike.recoil, strike.speed);
    let mut strength = if strike.edge {
        (1.0_f32 - strike.zone) * scaled + strike.zone
    } else if strike.zone > ZONE_FLOOR {
        strike.zone.sqrt() * scaled
    } else {
        0.0
    };
    if strength > 1.0 {
        strength = 1.0;
    }
    Some(FollowThrough {
        strength,
        stall: (f64::from(1.0_f32 - strength) * STALL_SHARE) as f32,
        flags: strike.flags | flags::STALLING | flags::CLEAVING,
        end_ms: (scaled * END_SCALE).round_ties_even() as i32 + strike.now_ms,
    })
}

fn dual_wield_holds(class_word: u32, flags: u32, clock: f32, counter: bool) -> bool {
    class_word == DUAL_WIELD
        && ((flags & flags::CONTINUATION) != 0 || f64::from(clock) > COUNTER_FROM && counter)
}

fn follow_scale(gain: f32, recoil: f32, speed: f32) -> f32 {
    let speed_scaled = f64::from(speed) * FOLLOW_SHARE;
    let shaped = f64::from(recoil) * speed_scaled;
    let scaled = (f64::from(gain) * shaped) as f32;
    if scaled < 0.0 { 0.0 } else { scaled }
}

/// A striker at least this large (character size, compared in double
/// precision) neither recoils from a block nor bogs down in a body hit.
pub const LARGE_STRIKER: f64 = 1.4;
/// A body-of-the-weapon blow at least this hard cleaves through.
const CLEAVES_FROM: f32 = 2.0;
/// The follow-through gain of a large striker's body hit and of an edge
/// hit held back.
const HELD_GAIN: f32 = 0.5;

/// The striker whose swing a blow met, as the damage routine reads it.
#[derive(Clone, Copy, Debug)]
pub struct Striker {
    /// The attacker's weapon class word.
    pub class_word: u32,
    /// Its character size.
    pub size: f32,
    /// The swing clock at the blow.
    pub clock: f32,
    /// The held slot of the striker that dealt it.
    pub slot: Slot,
    /// The game clock, ms.
    pub now_ms: i32,
}

impl SwingState {
    /// The striker's swing after it wounded: the released bit cleared and
    /// [`flags::WOUNDED`] set, the striker's window shut; then a striker
    /// under [`LARGE_STRIKER`] follows a body-of-the-weapon blow through
    /// with gain 1 − I/2 (cleaving on from an impact of 2), and an edge
    /// blow at 0.5 when it is held back ([`flags::HOLDS_BACK`]), else
    /// cleaves on; a larger one follows a body hit through at 0.5 and
    /// cleaves on from the edge ([`follow_through`]).
    pub fn after_wound(&mut self, blow: &Outcome, recoil: f32, striker: &Striker) {
        self.flags = (self.flags & !flags::RELEASED) | flags::WOUNDED;
        self.open[striker.slot.index()] = false;
        let gain = if f64::from(striker.size) < LARGE_STRIKER {
            if blow.central {
                (CLEAVES_FROM > blow.hit_impact).then_some(1.0 - blow.hit_impact * 0.5)
            } else {
                (self.flags & flags::HOLDS_BACK != 0).then_some(HELD_GAIN)
            }
        } else {
            blow.central.then_some(HELD_GAIN)
        };
        let Some(gain) = gain else {
            self.flags |= flags::CLEAVING;
            return;
        };
        if let Some(f) = follow_through(&FollowStrike {
            class_word: striker.class_word,
            flags: self.flags,
            clock: striker.clock,
            counter: false,
            gain,
            recoil,
            speed: blow.speed,
            zone: blow.zone,
            edge: blow.edge,
            now_ms: striker.now_ms,
        }) {
            self.strength = f.strength;
            self.stall = f.stall;
            self.flags = f.flags;
            self.end_ms = f.end_ms.max(0) as u64;
        }
    }

    /// The striker's swing recoiling from a blow at `speed` (cm a tick)
    /// with a weapon of recoil W17 `weapon_recoil`: its window shut, then
    /// [`recoil`] with the drive speed·W17. Returns the recoil's rest (its
    /// stance blend and arm stiffness) when it takes one.
    pub fn recoil_from(
        &mut self,
        speed: f32,
        weapon_recoil: f32,
        striker: &Striker,
    ) -> Option<RecoilRest> {
        self.open[striker.slot.index()] = false;
        let r = recoil(&RecoilStrike {
            class_word: striker.class_word,
            flags: self.flags,
            clock: striker.clock,
            counter: false,
            drive: speed * weapon_recoil,
            now_ms: striker.now_ms,
        });
        self.strength = r.strength;
        self.stall = r.stall;
        self.flags = r.flags;
        self.recoil = r.rest;
        if let Some(rest) = r.rest {
            self.end_ms = rest.end_ms.max(0) as u64;
        }
        r.rest
    }

    /// Takes the recoil's rest, leaving none: the stance that a stall's end
    /// starts uses its blend and gives the arms its stiffness.
    pub fn take_recoil(&mut self) -> Option<RecoilRest> {
        self.recoil.take()
    }
    /// A Force effect breaks its attack, as the original's interrupt does:
    /// the swing stalls with full strength and no drain, cleaving and
    /// recoiled, its window shut, and the stance returns 600 ms on. The
    /// original also takes 50·clock off the attack reach; Exarust keeps no
    /// attack reach.
    pub fn force_interrupt(&mut self, now_ms: u64) {
        self.flags |= flags::STALLING | flags::CLEAVING | flags::RECOILED;
        self.strength = 1.0;
        self.stall = 0.0;
        self.end_ms = now_ms + 600;
        self.open = [false; 2];
        self.recoil = None;
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
#[path = "swing_test.rs"]
mod tests;

//! The original's attack executor, which the player and the AI both go
//! through: when a fighter may start an attack (enough energy, the cooldown
//! since its last attack's last tick, or a released swing's overhead and
//! Feint while it stalls), which attack a standing request starts, which
//! clip a side's attack picks, and which of the fighter's held strikers
//! strike in it. Rules and constants: `docs/PLAYER.md` (Combat mode).
//!
//! Times are milliseconds of the game clock (12 ms a body tick,
//! [`exa_physics::body::BodyTicks::millis`]).

use bevy::prelude::*;
use exa_anim::Animator;

use crate::damage::WeaponRecord;
use crate::{Attacking, Blade};

/// The cooldown (ms) while the fighter has no primary striker: since its
/// hands were emptied (the fists are equipped), until its first attack
/// picks one.
pub const FIRST_COOLDOWN_MS: u64 = 400;
/// With a primary striker the cooldown is 300 ms × its recoil (W17),
/// rounded to the nearest (ties to even), plus 300 ms: 330 ms for the
/// fists.
const COOLDOWN_PER_RECOIL: f32 = 300.0;
const COOLDOWN_BASE_MS: u64 = 300;
/// A fighter attacks only with at least this share of its most energy.
pub const ATTACK_ENERGY: f64 = 0.8;
/// The stab's facing gate: the primary weapon's long axis, in the body's
/// frame, may point back by a forward component f ≤ 0 only while
/// round(−15·f) stays under 6 (f above −0.367).
const STAB_BACK_STEPS: f32 = 15.0;
const STAB_BACK_LIMIT: i64 = 6;
/// Weapon class bit of the fists, claws and two weapons: their attacks
/// strike with one hand, the motion's.
const ONE_HAND_STRIKES: u8 = 0x80;
/// Weapon class of the claws (fists of a clawed race): their overhead
/// strikes with both hands.
const CLAWS: u8 = 0xC0;
/// The hands bits of a weapon class: a thrusting (held) weapon.
const HANDS: u8 = 0x30;
/// A released swing's request turns into an overhead within this long (ms)
/// of the side attack's start.
const OVERHEAD_AFTER_RELEASE_MS: u64 = 300;
/// Feint's renewal of a released swing: an overhead or a stab starts with
/// this blend (frames); a side attack with its own.
const FEINT_BLEND: u32 = 10;
/// The attack clips' action indices: the main side's from 5, the other
/// side's from 8, the stab's and the overhead's.
const MAIN_FROM: usize = 5;
const OTHER_FROM: usize = 8;
const STAB: usize = 0x0B;
const OVERHEAD: usize = 0x0C;

/// The cooldown (ms) between attacks: [`FIRST_COOLDOWN_MS`] without a
/// primary striker, else from its recoil `primary_recoil` (W17).
pub fn cooldown_ms(primary_recoil: Option<f32>) -> u64 {
    match primary_recoil {
        None => FIRST_COOLDOWN_MS,
        Some(recoil) => {
            (COOLDOWN_PER_RECOIL * recoil).round_ties_even().max(0.0) as u64 + COOLDOWN_BASE_MS
        }
    }
}

/// Whether a fighter with `energy` of its most `max` may attack: at least
/// 80 % of it, compared in double precision as the original does.
pub fn has_energy(energy: f32, max: f32) -> bool {
    f64::from(energy) >= f64::from(max) * ATTACK_ENERGY
}

/// Whether a stab may start with the primary weapon's long axis at
/// `forward`, its forward component in the body's frame (−1..=1).
pub fn stab_faces(forward: f32) -> bool {
    let back = if forward <= 0.0 {
        (-forward * STAB_BACK_STEPS).round_ties_even() as i64
    } else {
        0
    };
    back < STAB_BACK_LIMIT
}

/// Whether a fighter of weapon class `weapon_class` may ask for a thrust
/// (Alt): only with a held weapon, never with the fists.
pub fn may_thrust(weapon_class: u8) -> bool {
    weapon_class & HANDS != 0
}

/// An attack's motion: its clip, by the original's motion numbers 5 to 12.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackMotion {
    /// Attack Mid, Down or Up (0, 1, 2): the main side.
    Main(usize),
    /// Attack Combo 1, 2 or 3 (0, 1, 2): the other side.
    Other(usize),
    Stab,
    Overhead,
}

/// The main side's clips and the other side's, by pick.
const MAIN_SIDE: [&str; 3] = ["Attack Mid", "Attack Down", "Attack Up"];
const OTHER_SIDE: [&str; 3] = ["Attack Combo 1", "Attack Combo 2", "Attack Combo 3"];

impl AttackMotion {
    /// The clip it plays.
    pub fn action(self) -> &'static str {
        match self {
            Self::Main(i) => MAIN_SIDE[i.min(2)],
            Self::Other(i) => OTHER_SIDE[i.min(2)],
            Self::Stab => "Stab",
            Self::Overhead => "Overhead Swing",
        }
    }

    /// The motion of a clip, if it is an attack's.
    pub fn of_action(action: &str) -> Option<Self> {
        if let Some(i) = MAIN_SIDE.iter().position(|&a| a == action) {
            return Some(Self::Main(i));
        }
        if let Some(i) = OTHER_SIDE.iter().position(|&a| a == action) {
            return Some(Self::Other(i));
        }
        match action {
            "Stab" => Some(Self::Stab),
            "Overhead Swing" => Some(Self::Overhead),
            _ => None,
        }
    }

    /// A side attack (Attack Mid … Attack Combo 3).
    pub fn is_side(self) -> bool {
        matches!(self, Self::Main(_) | Self::Other(_))
    }

    /// The motion's action index (the original's motion number, 5–12).
    pub fn index(self) -> usize {
        match self {
            Self::Main(i) => MAIN_FROM + i.min(2),
            Self::Other(i) => OTHER_FROM + i.min(2),
            Self::Stab => STAB,
            Self::Overhead => OVERHEAD,
        }
    }
}

/// An attack's side: the main one (Attack Mid, Down, Up) or the other
/// (Attack Combo 1–3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Main,
    Other,
}

impl Side {
    /// The side's attack with pick `i` (0–2).
    pub fn motion(self, i: usize) -> AttackMotion {
        match self {
            Side::Main => AttackMotion::Main(i),
            Side::Other => AttackMotion::Other(i),
        }
    }

    fn index(self) -> usize {
        match self {
            Side::Main => 0,
            Side::Other => 1,
        }
    }
}

/// Which held item a striker is: the first (the main hand's weapon, or the
/// right fist) or the second (the left fist, or a second weapon).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Slot {
    #[default]
    First,
    Second,
}

impl Slot {
    /// 0 for the first, 1 for the second.
    pub fn index(self) -> usize {
        match self {
            Slot::First => 0,
            Slot::Second => 1,
        }
    }
}

/// Whether a striker in held slot `slot` (`armed`: a weapon, not a fist or
/// claw) strikes in an attack `motion` of a fighter of weapon class
/// `weapon_class`. With the fists, claws or two weapons (class bit 0x80)
/// only the motion's hand strikes: the first for the main side, the stab
/// and the overhead (both claws in a clawed overhead), the second for the
/// other side. Otherwise every held weapon strikes, and the fists are not
/// held.
pub fn strikes(slot: Slot, armed: bool, weapon_class: u8, motion: AttackMotion) -> bool {
    if weapon_class & ONE_HAND_STRIKES == 0 {
        return armed;
    }
    match motion {
        AttackMotion::Overhead => slot == Slot::First || weapon_class == CLAWS,
        AttackMotion::Main(_) | AttackMotion::Stab => slot == Slot::First,
        AttackMotion::Other(_) => slot == Slot::Second,
    }
}

/// Whether a striker (`armed`: a weapon, not a fist or claw) of a fighter
/// of weapon class `weapon_class` is one of its held items, which strike
/// when their window opens: every weapon, and the fists or claws only when
/// they are what it holds (class bit 0x80).
pub fn is_striker(armed: bool, weapon_class: u8) -> bool {
    armed || weapon_class & ONE_HAND_STRIKES != 0
}

/// The held slot an attack `motion` makes the primary striker (whose recoil
/// sets the next cooldown): the second for the other side with the fists,
/// claws or two weapons, else the first.
pub fn primary_slot(weapon_class: u8, motion: AttackMotion) -> Slot {
    if weapon_class & ONE_HAND_STRIKES != 0 && matches!(motion, AttackMotion::Other(_)) {
        Slot::Second
    } else {
        Slot::First
    }
}

/// What a standing attack request asks of the executor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Request {
    /// A double click's: the overhead.
    pub overhead: bool,
    /// Alt with a held weapon: the stab.
    pub thrust: bool,
    /// The press came with the cursor left of the facing: the other side.
    pub other_side: bool,
}

/// What the executor starts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackKind {
    /// One of the main side's attacks.
    Main,
    /// One of the other side's.
    Other,
    Overhead,
    Stab,
}

/// What the executor reads of a fighter besides its request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Busy {
    /// The game clock (ms).
    pub now_ms: u64,
    /// The game clock of its last attack tick; `None` before its first.
    pub last_tick_ms: Option<u64>,
    /// Its cooldown ([`cooldown_ms`]).
    pub cooldown_ms: u64,
    /// The attack it plays, if any.
    pub playing: Option<AttackMotion>,
    /// Its swing was released (its request dropped mid-swing).
    pub released: bool,
    /// The game clock until which its released swing stalls.
    pub stall_until_ms: u64,
    /// The game clock from which Feint may renew its released swing.
    pub renew_ms: u64,
    /// The game clock its last side attack started at.
    pub side_started_ms: Option<u64>,
    /// It has Feint.
    pub feint: bool,
}

/// What the executor starts: the attack and, for a stab or an overhead,
/// its blend (frames; a side attack's is its own, [`start_blend`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Start {
    pub kind: AttackKind,
    pub blend: u32,
}

impl Start {
    fn of(kind: AttackKind, blend: u32) -> Option<Self> {
        Some(Self { kind, blend })
    }
}

/// The executor: the attack a standing `request` starts, given the
/// fighter's state `at`. While a swing stalls only a released swing's
/// request starts anything: the overhead (no blend) within 300 ms of the
/// side attack's start, else, with Feint once its renewal time has come,
/// a stab, an overhead (both with a blend of 10) or a side attack. Free, a
/// released swing's request gives the overhead within those 300 ms too,
/// and Feint waives the cooldown once its renewal time has come (before
/// it nothing starts); otherwise nothing until the cooldown has passed
/// since the last attack tick, then an overhead unless a stab or an
/// overhead plays, a stab unless a stab plays, else a side attack unless a
/// side attack plays. A stab still has to pass the facing gate
/// ([`stab_faces`]).
pub fn execute(request: Request, at: Busy) -> Option<Start> {
    let soon_after_side = at
        .side_started_ms
        .is_some_and(|t| at.now_ms < t + OVERHEAD_AFTER_RELEASE_MS);
    if at.now_ms < at.stall_until_ms {
        if !at.released {
            return None;
        }
        if soon_after_side {
            return Start::of(AttackKind::Overhead, 0);
        }
        if !at.feint || at.renew_ms > at.now_ms {
            return None;
        }
        return if request.thrust {
            Start::of(AttackKind::Stab, FEINT_BLEND)
        } else if request.overhead {
            Start::of(AttackKind::Overhead, FEINT_BLEND)
        } else if request.other_side {
            Start::of(AttackKind::Other, 0)
        } else {
            Start::of(AttackKind::Main, 0)
        };
    }
    let mut cooldown = at.cooldown_ms;
    if at.released {
        if soon_after_side {
            return Start::of(AttackKind::Overhead, 0);
        }
        if at.feint {
            if at.now_ms < at.renew_ms {
                return None;
            }
            cooldown = 0;
        }
    }
    if at
        .last_tick_ms
        .is_some_and(|last| last + cooldown > at.now_ms)
    {
        return None;
    }
    let kind = if request.overhead {
        if matches!(
            at.playing,
            Some(AttackMotion::Stab | AttackMotion::Overhead)
        ) {
            return None;
        }
        AttackKind::Overhead
    } else if request.thrust {
        if at.playing == Some(AttackMotion::Stab) {
            return None;
        }
        AttackKind::Stab
    } else if at.playing.is_some_and(AttackMotion::is_side) {
        return None;
    } else if request.other_side {
        AttackKind::Other
    } else {
        AttackKind::Main
    };
    Start::of(kind, 0)
}

/// A fighter's executor state (on the body model root).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attacker {
    /// The game clock (ms) of the last tick an attack played; `None` before
    /// its first.
    pub last_tick_ms: Option<u64>,
    /// The held slot of its primary striker: set by every attack, and by
    /// equipping a weapon; none since its hands were emptied.
    pub primary: Option<Slot>,
    /// The game clock (ms) its last side attack started at.
    pub side_started_ms: Option<u64>,
    /// Its generator for the attack picks (Exarust's own instance of the
    /// original's 32-bit generator).
    seed: u32,
    /// Each side's last pick, 0 until the first (as the original's zeroed
    /// fields, so a side's first pick is never its first clip).
    last_picks: [usize; 2],
    /// Stance class 1–6, copied from the held moveset when the hands
    /// change. 5 (unarmed) until then, so a swing before the first equip
    /// still sets both arms.
    pub stance_class: u8,
}

impl Default for Attacker {
    fn default() -> Self {
        Self {
            last_tick_ms: None,
            primary: None,
            side_started_ms: None,
            seed: 0,
            last_picks: [0; 2],
            stance_class: 5,
        }
    }
}

impl Attacker {
    /// One of `side`'s three attacks at random, never the one it picked
    /// last, by the original's 32-bit generator; `crouched`, always the
    /// first (Attack Mid or Combo 1), which then counts as its last pick.
    pub fn pick(&mut self, side: Side, crouched: bool) -> usize {
        let last = &mut self.last_picks[side.index()];
        if crouched {
            *last = 0;
            return 0;
        }
        loop {
            self.seed = self.seed.wrapping_mul(0x0808_8405).wrapping_add(1);
            let i = ((u64::from(self.seed) * 3) >> 32) as usize;
            if i != *last {
                *last = i;
                return i;
            }
        }
    }

    /// The record of its primary striker among `blades`: its own blade in
    /// its primary slot; none without a primary.
    pub fn primary_record<'a>(
        &self,
        fighter: Entity,
        blades: impl IntoIterator<Item = &'a Blade>,
    ) -> Option<&'a WeaponRecord> {
        let slot = self.primary?;
        blades
            .into_iter()
            .find(|b| b.wielder == fighter && b.slot == slot)
            .map(|b| &b.record)
    }

    /// Its cooldown (ms), from the recoil of its primary striker among
    /// `blades` (those it wields).
    pub fn cooldown_ms<'a>(
        &self,
        fighter: Entity,
        blades: impl IntoIterator<Item = &'a Blade>,
    ) -> u64 {
        cooldown_ms(self.primary_record(fighter, blades).map(|r| r.recoil))
    }
}

/// The other side's attacks start with a blend of round((20 − W19) × 0.6)
/// frames, W19 the new primary striker's stance blend.
const OTHER_SIDE_BLEND_FROM: i32 = 20;
const OTHER_SIDE_BLEND_SCALE: f64 = 0.6;

/// The blend (frames) an attack `motion` starts with, eased smoothly: the
/// main side's none; the other side's round((20 − W19) × 0.6), ties to
/// even, `primary_blend` the stance blend W19 of the striker the attack
/// made primary (12 frames for the fists); the stab's and the overhead's
/// `given` by the executor (10 for Feint's, else none).
pub fn start_blend(motion: AttackMotion, primary_blend: Option<i32>, given: u32) -> u32 {
    match motion {
        AttackMotion::Other(_) => {
            let w = OTHER_SIDE_BLEND_FROM - primary_blend.unwrap_or(0);
            (f64::from(w) * OTHER_SIDE_BLEND_SCALE)
                .round_ties_even()
                .max(0.0) as u32
        }
        AttackMotion::Main(_) => 0,
        AttackMotion::Stab | AttackMotion::Overhead => given,
    }
}

/// The sound a free main-side start plays at the fighter's hip, unless the
/// body's quiet byte is set: two uniform draws, the first for the pitch
/// (0.2·u + 0.8) / √scale, the second for the gain 0.4·v + 0.6, each
/// worked in double precision and stored as single (the executor's
/// sequence). The busy branch's Feint start plays none. Returns
/// (gain, pitch). Not yet played by the running game: the sound handle
/// the original passes is not identified.
pub fn main_start_sound(first: f32, second: f32, size_root: f32) -> (f32, f32) {
    let pitch = ((f64::from(first) * 0.2 + 0.8) / f64::from(size_root)) as f32;
    let gain = (f64::from(second) * 0.4 + 0.6) as f32;
    (gain, pitch)
}

/// The attack a fighter plays: its attack's motion while its clip plays
/// once.
pub fn playing(attacking: Option<&Attacking>, anim: &Animator) -> Option<AttackMotion> {
    attacking.filter(|_| !anim.looping).map(|a| a.motion)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Empty hands are stance class 5 until a held moveset copies its own.
    #[test]
    fn a_new_attacker_is_unarmed() {
        assert_eq!(Attacker::default().stance_class, 5);
    }

    /// The main-side start's sound: the first draw sets the pitch, the
    /// second the gain, in double precision.
    #[test]
    fn the_main_start_sound_takes_pitch_then_gain() {
        let (gain, pitch) = main_start_sound(0.5, 0.25, 2.0);
        assert_eq!(pitch, ((0.5f64 * 0.2 + 0.8) / 2.0) as f32);
        assert_eq!(gain, (0.25f64 * 0.4 + 0.6) as f32);
        assert!((pitch - 0.45).abs() < 1e-7 && (gain - 0.7).abs() < 1e-7);
    }
    use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};
    use exa_physics::body::BodyTicks;

    /// The human fist's record: 17.66 cm across the hand.
    fn fist() -> WeaponRecord {
        WeaponRecord::build(
            &Section::FISTS,
            false,
            Lengths::hand(-8.0, 9.66),
            &Holder::AVERAGE,
        )
    }

    /// 400 ms without a primary striker; with the fist (recoil 0.1) 330 ms,
    /// with the arena sword's recoil 0.42 426 ms; 300·r rounds to the
    /// nearest, ties to even (0.4175 → 125.25 → 125, 0.375 → 112.5 → 112).
    #[test]
    fn the_cooldown_follows_the_primary_recoil() {
        assert!((fist().recoil - 0.1).abs() < 1e-6, "{:?}", fist());
        assert_eq!(cooldown_ms(None), 400);
        assert_eq!(cooldown_ms(Some(fist().recoil)), 330);
        assert_eq!(cooldown_ms(Some(0.42)), 426);
        assert_eq!(cooldown_ms(Some(0.4175)), 425);
        assert_eq!(cooldown_ms(Some(0.375)), 412);
        // The game clock: 330 ms are 27.5 ticks of 12 ms, so the 28th tick.
        assert_eq!(BodyTicks(28).millis(), 336);
        assert!(BodyTicks(27).millis() < 330 && BodyTicks(28).millis() >= 330);
    }

    /// The gate is 80 % of the most energy: a human (most 1) attacks at 0.8
    /// and above, not at 0.79; a feral of most 0.95 from 0.76.
    #[test]
    fn attacks_need_80_percent_of_the_energy() {
        assert!(has_energy(1.0, 1.0));
        assert!(has_energy(0.8, 1.0));
        assert!(!has_energy(0.799, 1.0));
        assert!(!has_energy(0.05, 1.0));
        assert!(has_energy(0.76, 0.95));
        assert!(!has_energy(0.759, 0.95));
    }

    /// A blade pointing forward or level stabs; back by f = −0.36 (round
    /// 5.4 = 5) still, by −0.37 (5.55 → 6) not; −0.3667 is 5.5, which
    /// rounds to 6.
    #[test]
    fn the_stab_needs_the_blade_ahead() {
        assert!(stab_faces(1.0));
        assert!(stab_faces(0.0));
        assert!(stab_faces(-0.36));
        assert!(!stab_faces(-0.37));
        assert!(!stab_faces(-1.0));
        assert!(may_thrust(0x10) && may_thrust(0x20) && may_thrust(0x30));
        assert!(!may_thrust(0x80) && !may_thrust(0xC0) && !may_thrust(0));
    }

    /// A free fighter (nothing stalling, nothing released) at `now_ms`
    /// whose last attack ticked at `last` with cooldown `cooldown`, playing
    /// `playing`.
    fn free(now_ms: u64, last: Option<u64>, cooldown: u64, playing: Option<AttackMotion>) -> Busy {
        Busy {
            now_ms,
            last_tick_ms: last,
            cooldown_ms: cooldown,
            playing,
            ..Busy::default()
        }
    }

    fn kind(start: Option<Start>) -> Option<AttackKind> {
        start.map(|s| s.kind)
    }

    /// Nothing within the cooldown since the last attack tick; at it a side
    /// attack, never over another side attack; an overhead over a side
    /// attack but not over a stab or an overhead; a stab not over a stab.
    /// None of them with a blend.
    #[test]
    fn the_executor_waits_out_the_cooldown() {
        let main = Request::default();
        let other = Request {
            other_side: true,
            ..main
        };
        assert_eq!(
            execute(main, free(1000, None, 400, None)),
            Some(Start {
                kind: AttackKind::Main,
                blend: 0
            })
        );
        assert_eq!(
            kind(execute(other, free(1000, None, 400, None))),
            Some(AttackKind::Other)
        );
        assert_eq!(execute(main, free(1329, Some(1000), 330, None)), None);
        assert_eq!(
            kind(execute(main, free(1330, Some(1000), 330, None))),
            Some(AttackKind::Main)
        );
        let mid = Some(AttackMotion::Main(0));
        assert_eq!(execute(main, free(5000, Some(1000), 330, mid)), None);
        assert_eq!(execute(other, free(5000, Some(1000), 330, mid)), None);
        let overhead = Request {
            overhead: true,
            ..main
        };
        assert_eq!(
            execute(overhead, free(5000, Some(1000), 330, mid)),
            Some(Start {
                kind: AttackKind::Overhead,
                blend: 0
            })
        );
        for busy in [AttackMotion::Stab, AttackMotion::Overhead] {
            assert_eq!(
                execute(overhead, free(5000, Some(1000), 330, Some(busy))),
                None
            );
        }
        // The overhead waits for the cooldown too.
        assert_eq!(execute(overhead, free(1200, Some(1000), 330, None)), None);
        let thrust = Request {
            thrust: true,
            ..main
        };
        assert_eq!(
            kind(execute(thrust, free(5000, Some(1000), 330, mid))),
            Some(AttackKind::Stab)
        );
        assert_eq!(
            execute(
                thrust,
                free(5000, Some(1000), 330, Some(AttackMotion::Stab))
            ),
            None
        );
    }

    /// While a released swing stalls (until 2000 ms), a new press within
    /// 300 ms of the side attack's start (at 1000 ms) gives the overhead
    /// with no blend, whatever it asks; later nothing without Feint, and
    /// with Feint nothing before its renewal (1500 ms), then a stab or an
    /// overhead with a blend of 10, or a side attack with its own. A swing
    /// that stalls unreleased (a struck one) keeps the executor busy.
    #[test]
    fn a_released_swing_gives_the_overhead_and_feint() {
        let stalling = Busy {
            now_ms: 1299,
            last_tick_ms: Some(1296),
            cooldown_ms: 330,
            playing: Some(AttackMotion::Main(1)),
            released: true,
            stall_until_ms: 2000,
            renew_ms: 1500,
            side_started_ms: Some(1000),
            feint: false,
        };
        let main = Request::default();
        assert_eq!(
            execute(main, stalling),
            Some(Start {
                kind: AttackKind::Overhead,
                blend: 0
            })
        );
        let late = Busy {
            now_ms: 1300,
            ..stalling
        };
        assert_eq!(execute(main, late), None);
        let feint = Busy {
            feint: true,
            ..late
        };
        assert_eq!(execute(main, feint), None);
        let renewed = Busy {
            now_ms: 1500,
            ..feint
        };
        let overhead = Request {
            overhead: true,
            ..main
        };
        let thrust = Request {
            thrust: true,
            ..main
        };
        let other = Request {
            other_side: true,
            ..main
        };
        assert_eq!(
            execute(overhead, renewed),
            Some(Start {
                kind: AttackKind::Overhead,
                blend: 10
            })
        );
        assert_eq!(
            execute(thrust, renewed),
            Some(Start {
                kind: AttackKind::Stab,
                blend: 10
            })
        );
        assert_eq!(
            execute(other, renewed),
            Some(Start {
                kind: AttackKind::Other,
                blend: 0
            })
        );
        assert_eq!(kind(execute(main, renewed)), Some(AttackKind::Main));
        // Struck mid-swing (a stall without the release): busy.
        let struck = Busy {
            released: false,
            ..renewed
        };
        assert_eq!(execute(main, struck), None);
        assert_eq!(
            execute(
                overhead,
                Busy {
                    now_ms: 1100,
                    ..struck
                }
            ),
            None
        );
    }

    /// Once the released swing's stall is over: within 300 ms of the side
    /// attack's start a press still gives the overhead (before the
    /// cooldown); after them Feint waives the cooldown from its renewal
    /// time on (nothing before it), without Feint the cooldown holds.
    #[test]
    fn feint_waives_the_cooldown_after_a_release() {
        let released = Busy {
            now_ms: 1250,
            last_tick_ms: Some(1200),
            cooldown_ms: 330,
            playing: None,
            released: true,
            stall_until_ms: 1200,
            renew_ms: 1400,
            side_started_ms: Some(1000),
            feint: false,
        };
        let main = Request::default();
        assert_eq!(kind(execute(main, released)), Some(AttackKind::Overhead));
        let later = Busy {
            now_ms: 1400,
            ..released
        };
        assert_eq!(execute(main, later), None);
        assert_eq!(
            kind(execute(
                main,
                Busy {
                    now_ms: 1530,
                    ..later
                }
            )),
            Some(AttackKind::Main)
        );
        let feint = Busy {
            feint: true,
            ..later
        };
        assert_eq!(kind(execute(main, feint)), Some(AttackKind::Main));
        assert_eq!(
            execute(
                main,
                Busy {
                    now_ms: 1399,
                    ..feint
                }
            ),
            None
        );
    }

    #[test]
    fn the_picks_follow_the_original_generator() {
        // × 0x08088405 + 1 from 0, clip = state × 3 >> 32, drawn again
        // while it equals the last pick (0 at first).
        let mut a = Attacker::default();
        let picks: Vec<usize> = (0..8).map(|_| a.pick(Side::Main, false)).collect();
        assert_eq!(picks, [2, 0, 2, 0, 1, 0, 1, 0]);
        assert_eq!(a.seed, 303_092_965);
        // Crouched: Attack Mid, which then is the side's last pick.
        assert_eq!(a.pick(Side::Main, true), 0);
        assert_eq!(a.last_picks[0], 0);
        assert_eq!(a.seed, 303_092_965);
        assert_eq!(Side::Other.motion(2), AttackMotion::Other(2));
        assert_eq!(AttackMotion::Main(2).index(), 7);
        assert_eq!(AttackMotion::Other(0).index(), 8);
        assert_eq!(AttackMotion::Stab.index(), 0x0B);
        assert_eq!(AttackMotion::Overhead.index(), 0x0C);
    }

    #[test]
    fn a_side_never_repeats_its_last_attack() {
        let mut a = Attacker::default();
        let picks: Vec<usize> = (0..200).map(|_| a.pick(Side::Main, false)).collect();
        // The first pick is never the first clip: the last pick starts at 0.
        assert_ne!(picks[0], 0);
        assert!(picks.windows(2).all(|w| w[0] != w[1]));
        for i in 0..3 {
            let n = picks.iter().filter(|&&p| p == i).count();
            assert!((50..=84).contains(&n), "clip {i}: {n} of 200");
        }
        // Each side keeps its own last pick.
        assert_ne!(a.pick(Side::Other, false), 0);
    }

    /// Fists strike with the motion's hand: the right in the main side,
    /// the left in the other side, the right in the overhead; clawed hands
    /// both in the overhead; a held weapon always, and fists beside it
    /// never.
    #[test]
    fn the_motion_picks_the_striking_hand() {
        use AttackMotion::*;
        let fists = 0x80;
        for (motion, first, second) in [
            (Main(0), true, false),
            (Main(2), true, false),
            (Other(1), false, true),
            (Overhead, true, false),
            (Stab, true, false),
        ] {
            assert_eq!(
                strikes(Slot::First, false, fists, motion),
                first,
                "{motion:?}"
            );
            assert_eq!(
                strikes(Slot::Second, false, fists, motion),
                second,
                "{motion:?}"
            );
        }
        assert!(strikes(Slot::Second, false, 0xC0, Overhead));
        assert!(!strikes(Slot::Second, false, 0xC0, Main(0)));
        for motion in [Main(0), Other(0), Overhead, Stab] {
            assert!(strikes(Slot::First, true, 0x10, motion));
            assert!(!strikes(Slot::First, false, 0x10, motion));
            assert!(!strikes(Slot::Second, false, 0x10, motion));
        }
        assert_eq!(primary_slot(fists, Other(0)), Slot::Second);
        assert_eq!(primary_slot(fists, Main(1)), Slot::First);
        assert_eq!(primary_slot(fists, Overhead), Slot::First);
        assert_eq!(primary_slot(0x10, Other(0)), Slot::First);
    }

    #[test]
    fn motions_name_their_clips() {
        for motion in [
            AttackMotion::Main(0),
            AttackMotion::Main(1),
            AttackMotion::Main(2),
            AttackMotion::Other(0),
            AttackMotion::Other(2),
            AttackMotion::Stab,
            AttackMotion::Overhead,
        ] {
            assert_eq!(AttackMotion::of_action(motion.action()), Some(motion));
        }
        assert_eq!(AttackMotion::Main(1).action(), "Attack Down");
        assert_eq!(AttackMotion::Other(2).action(), "Attack Combo 3");
        assert_eq!(AttackMotion::of_action("Cmbt - Idle"), None);
    }

    /// The cooldown reads the recoil of the fighter's own blade in its
    /// primary slot; with none, 400 ms.
    #[test]
    fn the_cooldown_reads_the_primary_blade() {
        let mut world = World::new();
        let (me, other) = (world.spawn_empty().id(), world.spawn_empty().id());
        let mut sword = fist();
        sword.recoil = 0.42;
        let blades = [
            Blade::new(other, 0.9, sword),
            Blade::new(me, 0.18, fist()).in_slot(Slot::Second),
        ];
        let mut a = Attacker::default();
        assert_eq!(a.cooldown_ms(me, &blades), 400);
        a.primary = Some(Slot::Second);
        assert_eq!(a.cooldown_ms(me, &blades), 330);
        a.primary = Some(Slot::First);
        assert_eq!(a.cooldown_ms(me, &blades), 400);
        assert_eq!(a.cooldown_ms(other, &blades), 426);
    }

    /// The main side starts with no blend; the other side with
    /// round((20 − W19) × 0.6): 12 frames with the fists (W19 0), 8 with a
    /// sword of W19 7 (7.8), 12 with none; the stab and the overhead with
    /// the executor's (10 from Feint), the side attacks never with it.
    #[test]
    fn the_other_side_starts_with_a_blend() {
        use AttackMotion::*;
        assert_eq!(fist().blend, 0);
        for m in [Main(0), Main(2), Stab, Overhead] {
            assert_eq!(start_blend(m, Some(7), 0), 0, "{m:?}");
        }
        assert_eq!(start_blend(Other(1), Some(fist().blend), 10), 12);
        assert_eq!(start_blend(Other(0), Some(7), 0), 8);
        assert_eq!(start_blend(Other(2), None, 0), 12);
        assert_eq!(start_blend(Other(0), Some(20), 0), 0);
        assert_eq!(start_blend(Main(1), Some(7), 10), 0);
        assert_eq!(start_blend(Stab, Some(7), 10), 10);
        assert_eq!(start_blend(Overhead, None, 10), 10);
    }
}

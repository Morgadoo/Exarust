//! The original's attack executor, which the player and the AI both go
//! through: when a fighter may start an attack (enough energy, the cooldown
//! since its last attack's last tick), which attack a standing request
//! starts, and which of the fighter's held strikers strike in it. Rules and
//! constants: `docs/PLAYER.md` (Combat mode).
//!
//! Times are milliseconds of the game clock (12 ms a body tick,
//! [`BodyTicks::millis`]).

use bevy::prelude::*;
use exa_anim::Animator;
use exa_physics::body::BodyTicks;

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
}

/// Which held item a striker is: the first (the main hand's weapon, or the
/// right fist) or the second (the left fist, or a second weapon).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Slot {
    #[default]
    First,
    Second,
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

/// The executor: the attack a standing `request` starts at `now_ms`, given
/// the game clock of the fighter's last attack tick, its cooldown and the
/// attack it plays (`None` when it plays none). Nothing until the cooldown
/// has passed since the last attack tick; then an overhead unless a stab
/// or an overhead plays, a stab unless a stab plays, else a side attack
/// unless a side attack plays. A stab still has to pass the facing gate
/// ([`stab_faces`]).
pub fn execute(
    request: Request,
    now_ms: u64,
    last_tick_ms: Option<u64>,
    cooldown_ms: u64,
    playing: Option<AttackMotion>,
) -> Option<AttackKind> {
    if last_tick_ms.is_some_and(|last| last + cooldown_ms > now_ms) {
        return None;
    }
    if request.overhead {
        return (!matches!(playing, Some(AttackMotion::Stab | AttackMotion::Overhead)))
            .then_some(AttackKind::Overhead);
    }
    if request.thrust {
        return (playing != Some(AttackMotion::Stab)).then_some(AttackKind::Stab);
    }
    if playing.is_some_and(AttackMotion::is_side) {
        return None;
    }
    Some(if request.other_side {
        AttackKind::Other
    } else {
        AttackKind::Main
    })
}

/// A fighter's executor state (on the body model root).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Attacker {
    /// The game clock (ms) of the last tick an attack played; `None` before
    /// its first.
    pub last_tick_ms: Option<u64>,
    /// The held slot of its primary striker: set by every attack, and by
    /// equipping a weapon; none since its hands were emptied.
    pub primary: Option<Slot>,
}

impl Attacker {
    /// Its cooldown (ms), from the recoil of its primary striker among
    /// `blades` (those it wields).
    pub fn cooldown_ms<'a>(
        &self,
        fighter: Entity,
        blades: impl IntoIterator<Item = &'a Blade>,
    ) -> u64 {
        let recoil = self.primary.and_then(|slot| {
            blades
                .into_iter()
                .find(|b| b.wielder == fighter && b.slot == slot)
                .map(|b| b.record.recoil)
        });
        cooldown_ms(recoil)
    }
}

/// The attack a fighter plays: its attack's motion while its clip plays
/// once.
pub fn playing(attacking: Option<&Attacking>, anim: &Animator) -> Option<AttackMotion> {
    attacking.filter(|_| !anim.looping).map(|a| a.motion)
}

/// Each body tick, before the character ticks: every fighter whose attack
/// plays stamps the game clock as its last attack tick, which its
/// cooldown counts from.
pub(crate) fn stamp_attack_ticks(
    ticks: Res<BodyTicks>,
    mut fighters: Query<(&Animator, &mut Attacker), With<Attacking>>,
) {
    let now = ticks.millis();
    for (anim, mut attacker) in &mut fighters {
        if !anim.looping {
            attacker.last_tick_ms = Some(now);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use exa_formats::weapon_record::{Holder, Lengths, Section, WeaponRecord};

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

    /// Nothing within the cooldown since the last attack tick; at it a side
    /// attack, never over another side attack; an overhead over a side
    /// attack but not over a stab or an overhead; a stab not over a stab.
    #[test]
    fn the_executor_waits_out_the_cooldown() {
        let main = Request::default();
        let other = Request {
            other_side: true,
            ..main
        };
        assert_eq!(execute(main, 1000, None, 400, None), Some(AttackKind::Main));
        assert_eq!(
            execute(other, 1000, None, 400, None),
            Some(AttackKind::Other)
        );
        assert_eq!(execute(main, 1329, Some(1000), 330, None), None);
        assert_eq!(
            execute(main, 1330, Some(1000), 330, None),
            Some(AttackKind::Main)
        );
        let mid = Some(AttackMotion::Main(0));
        assert_eq!(execute(main, 5000, Some(1000), 330, mid), None);
        assert_eq!(execute(other, 5000, Some(1000), 330, mid), None);
        let overhead = Request {
            overhead: true,
            ..main
        };
        assert_eq!(
            execute(overhead, 5000, Some(1000), 330, mid),
            Some(AttackKind::Overhead)
        );
        for busy in [AttackMotion::Stab, AttackMotion::Overhead] {
            assert_eq!(execute(overhead, 5000, Some(1000), 330, Some(busy)), None);
        }
        // The overhead waits for the cooldown too.
        assert_eq!(execute(overhead, 1200, Some(1000), 330, None), None);
        let thrust = Request {
            thrust: true,
            ..main
        };
        assert_eq!(
            execute(thrust, 5000, Some(1000), 330, mid),
            Some(AttackKind::Stab)
        );
        assert_eq!(
            execute(thrust, 5000, Some(1000), 330, Some(AttackMotion::Stab)),
            None
        );
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

    /// While its attack clip plays, each tick stamps the game clock; once
    /// the clip is back in its stance the stamp stays at its last tick.
    #[test]
    fn the_last_attack_tick_is_stamped_every_attack_tick() {
        let mut app = App::new();
        app.insert_resource(BodyTicks(100))
            .add_systems(Update, stamp_attack_ticks);
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        anim.play_once("Attack Mid", "Cmbt - Idle");
        let fighter = app
            .world_mut()
            .spawn((anim, Attacker::default(), Attacking::default()))
            .id();
        app.update();
        let stamp = |app: &App| app.world().get::<Attacker>(fighter).unwrap().last_tick_ms;
        assert_eq!(stamp(&app), Some(1200));
        app.world_mut().resource_mut::<BodyTicks>().0 = 130;
        app.update();
        assert_eq!(stamp(&app), Some(1560));
        app.world_mut()
            .get_mut::<Animator>(fighter)
            .unwrap()
            .play_loop("Cmbt - Idle");
        app.world_mut().resource_mut::<BodyTicks>().0 = 140;
        app.update();
        assert_eq!(stamp(&app), Some(1560));
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
}

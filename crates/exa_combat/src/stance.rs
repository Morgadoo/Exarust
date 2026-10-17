//! The combat stance: the looping "home" clip a fighter returns to, as the
//! original keeps it on every character. Every clip a fighter plays once
//! ends in it, and a balanced fighter standing in another looping clip
//! switches to it ([`take_stance`]). Who chooses it: the player's
//! behaviour in and out of combat mode ([`player_stance`]), a duellist by
//! its distance to its target ([`duellist_stance`]), and a dazed fighter's
//! body, which takes the Defence stance while its energy is more than 0.05
//! under its most. Each change starts the stance's clip from where the
//! body is with the original's blend and ease: 30 frames, smooth, through
//! the helper ([`set_stance`]); 30, linear, by the posture switch; 40 or
//! twice the primary striker's stance blend, smooth, by a duellist
//! ([`take_ai_stance`]); round((1.05 − E) × 300), smooth, into the daze's
//! Defence stance ([`daze_blend`]). Rules: `docs/PLAYER.md` (The combat
//! stance).
//!
//! Not yet: the Defence stance and Rebalance a fighter takes from its
//! balance or its tone (C11); parries, whose clips are stances while they
//! last (L10).

use bevy::prelude::*;
use exa_anim::{Animator, Ease};
use exa_physics::body::{CharacterTicks, Fallen, Ragdoll};

use crate::fighter::DEATH;
use crate::{Attacking, Condition, Fighter, attack};

/// The looping clip a fighter returns to.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Stance {
    /// Out of combat.
    #[default]
    Idle,
    /// The guard: the fists up before the chest and chin unarmed, the
    /// weapon at mid-chest armed.
    Defence,
    /// The combat idle: knees bent, the hands (or the weapon) low.
    CombatIdle,
}

impl Stance {
    /// The clip (an action of the fighter's moveset).
    pub fn action(self) -> &'static str {
        match self {
            Stance::Idle => "Idle",
            Stance::Defence => "Defence stance",
            Stance::CombatIdle => "Cmbt - Idle",
        }
    }

    /// The stance whose clip is `action`.
    pub fn of_action(action: &str) -> Option<Self> {
        [Stance::Idle, Stance::Defence, Stance::CombatIdle]
            .into_iter()
            .find(|s| s.action() == action)
    }
}

/// In combat mode the player stands in the combat idle with its target
/// nearer than this (m, chest to chest), times the target's body scale.
pub const CLOSE: f32 = 3.5;
/// ... or this long (s) after its last attack's last tick.
pub const AFTER_ATTACK: f32 = 3.0;
/// The player's target is the best within this distance (m).
pub const TARGET_RANGE: f32 = 5.0;
/// A duellist guards at this many times its reach or more.
pub const GUARD_REACHES: f32 = 4.0;
/// An unarmed human's reach (m): the race's own.
pub const UNARMED_REACH: f32 = 0.5;
/// Readiness rises this much per tick in the combat idle.
pub const READINESS_RATE: f32 = 0.004;
/// The posture switch needs a balance and a muscle tone of at least these.
const POSTURE_BALANCE: f32 = 0.8;
const POSTURE_TONE: f32 = 0.85;
/// The helper's start of a new stance: 30 frames of blend, eased smoothly.
const HELPER_BLEND: u32 = 30;
/// The posture switch's start: 30 frames, linear.
const POSTURE_BLEND: u32 = 30;
/// A duellist starts its stance with 40 frames of blend without a primary
/// striker, else twice its stance blend (W19), eased smoothly.
const AI_BLEND: u32 = 40;
/// The daze's Defence stance starts with round((1.05 − E) × 300) frames of
/// blend, E the energy, eased smoothly.
const DAZE_BLEND_FROM: f64 = 1.05;
const DAZE_BLEND_SCALE: f64 = 300.0;
/// Where a body's chest is (m above its feet) before its balance has
/// measured it.
const CHEST_HEIGHT: f32 = 1.3;
/// Looping clips the posture switch leaves alone: the combat idle (the
/// original's body tick handles it in its own branch, which skips the
/// switch), the walk and the run (each its own branch too), and the
/// backpedal and combat walk of Exarust's interim footwork.
const KEPT: [&str; 5] = ["Cmbt - Idle", "Walk", "Run", "Backpedal", "Combat Walk"];

pub struct StancePlugin;

/// The body tick's stance part ([`take_stance`]): the daze's Defence stance
/// comes first in the original's body tick, so a fall decided in the same
/// tick (after this set) replaces the daze's start.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct StanceTick;

impl Plugin for StancePlugin {
    fn build(&self, app: &mut App) {
        // The body tick's part: before each living body's clip advances.
        app.add_systems(
            FixedUpdate,
            take_stance.in_set(StanceTick).before(CharacterTicks),
        );
    }
}

/// The player's stance, chosen every behaviour tick: Idle out of combat
/// mode; in combat mode the Defence stance while dying (yellow health
/// under 0.01), the combat idle with its target nearer than 3.5 m times
/// the target's body scale or within 3 s of its last attack's last tick,
/// unless it is `dazed` (its energy more than 0.05 under its most), else
/// the Defence stance. `target` is the target's distance (m) and body
/// scale, `since_attack` the seconds since that tick.
///
/// Not yet: a parrying player keeps its stance (no parries).
pub fn player_stance(
    combat: bool,
    yellow: f32,
    dazed: bool,
    target: Option<(f32, f32)>,
    since_attack: Option<f32>,
) -> Stance {
    if !combat {
        Stance::Idle
    } else if yellow < DEATH {
        Stance::Defence
    } else if !dazed
        && (target.is_some_and(|(d, scale)| d < CLOSE * scale)
            || since_attack.is_some_and(|t| t < AFTER_ATTACK))
    {
        Stance::CombatIdle
    } else {
        Stance::Defence
    }
}

/// The player's target among `candidates` (each with its chest point): the
/// one within 5 m of `from` with the best (5 − d)·(cos + 0.5), the angle
/// measured from `aim` (a horizontal unit vector, the cursor's direction),
/// so none further than 120° off it. Returns it with its distance.
pub fn pick_target<T: Copy>(
    from: Vec3,
    aim: Vec3,
    candidates: impl IntoIterator<Item = (T, Vec3)>,
) -> Option<(T, f32)> {
    let mut best: Option<(T, f32, f32)> = None;
    for (who, chest) in candidates {
        let to = chest - from;
        let d = to.length();
        if d > TARGET_RANGE || d < 1e-4 {
            continue;
        }
        let score = (TARGET_RANGE - d) * (to.dot(aim) / d + 0.5);
        if score > best.map_or(0.0, |b| b.2) {
            best = Some((who, d, score));
        }
    }
    best.map(|(who, d, _)| (who, d))
}

/// A duellist's stance in combat: the Defence stance at 4 reaches or more
/// from its target (m, chest to chest), else the combat idle.
pub fn duellist_stance(distance: f32, reach: f32) -> Stance {
    if distance >= GUARD_REACHES * reach {
        Stance::Defence
    } else {
        Stance::CombatIdle
    }
}

/// Sets a fighter's stance as the original's helper does: when the clip
/// playing is the old stance's, the new stance starts at once, blending
/// 30 frames from where the body is, eased smoothly.
pub fn set_stance(stance: &mut Stance, anim: &mut Animator, new: Stance) {
    if new != *stance && anim.looping && anim.action == stance.action() {
        anim.start(new.action(), true, None, HELPER_BLEND, Ease::Smooth);
    }
    *stance = new;
}

/// The blend (frames) a duellist starts its stance with: 40 without a
/// primary striker, else twice the primary's stance blend `primary_blend`
/// (W19).
pub fn ai_stance_blend(primary_blend: Option<i32>) -> u32 {
    primary_blend.map_or(AI_BLEND, |w| 2 * w.max(0) as u32)
}

/// A standing duellist's stance, as the original's AI starts it on every
/// pass: its clip starts unless it plays already, blending
/// [`ai_stance_blend`] frames, and the blend from the start's capture is
/// eased smoothly either way.
pub fn take_ai_stance(anim: &mut Animator, stance: Stance, primary_blend: Option<i32>) {
    if anim.action != stance.action() || !anim.looping {
        let blend = ai_stance_blend(primary_blend);
        anim.start(stance.action(), true, None, blend, Ease::Smooth);
    }
    anim.ease = Ease::Smooth;
}

/// The blend (frames) the daze's Defence stance starts with at `energy`:
/// round((1.05 − E) × 300), ties to even (45 at 0.9, 129 at 0.62).
pub fn daze_blend(energy: f32) -> u32 {
    ((DAZE_BLEND_FROM - f64::from(energy)) * DAZE_BLEND_SCALE)
        .round_ties_even()
        .max(0.0) as u32
}

/// The daze's part in the body tick, for a fighter whose energy is more
/// than 0.05 under its most: its stance becomes the Defence stance, and
/// unless it is down (`fallen`: Exarust's fall, or a helpless condition)
/// or attacking, the Defence stance starts if it does not play already,
/// blending [`daze_blend`] frames, and the blend from the start's capture
/// is eased smoothly either way.
pub fn take_daze(
    stance: &mut Stance,
    anim: &mut Animator,
    energy: f32,
    fallen: bool,
    attacking: bool,
) {
    *stance = Stance::Defence;
    if fallen || attacking {
        return;
    }
    if anim.action != Stance::Defence.action() {
        let blend = daze_blend(energy);
        anim.start(Stance::Defence.action(), true, None, blend, Ease::Smooth);
    }
    anim.ease = Ease::Smooth;
}

/// Whether a balanced fighter switches to its stance: a looping clip other
/// than the stance's and not one of those the switch leaves alone, a
/// balance of 0.8 or more and a muscle tone of 0.85 or more.
pub fn switches_posture(anim: &Animator, stance: Stance, balance: f32, tone: f32) -> bool {
    anim.looping
        && anim.action != stance.action()
        && !KEPT.contains(&anim.action.as_str())
        && balance >= POSTURE_BALANCE
        && tone >= POSTURE_TONE
}

/// One tick of combat readiness: it rises by 0.004 up to 1 while the
/// stance and the clip are both the combat idle, and drops to 0 at once
/// otherwise. (The original also raises it while parrying and while an
/// attack is ready to chain; neither exists yet.)
pub fn readiness_step(readiness: f32, stance: Stance, action: &str) -> f32 {
    if stance == Stance::CombatIdle && action == Stance::CombatIdle.action() {
        (readiness + READINESS_RATE).min(1.0)
    } else {
        0.0
    }
}

/// Where a body's chest is (m): its balance's chest point, or a standing
/// chest's height over its root before the balance measured it.
pub fn chest_point(root: &GlobalTransform, body: Option<&Ragdoll>) -> Vec3 {
    body.and_then(|b| b.drive.as_ref())
        .and_then(|d| d.balance().chest())
        .unwrap_or(root.translation() + Vec3::Y * CHEST_HEIGHT)
}

/// The body tick's part of the stance, for every fighter that has one: its
/// readiness, the daze's Defence stance ([`take_daze`]), the clip it plays
/// once ending in its stance, and the posture switch (30 frames, linear).
#[allow(clippy::type_complexity)]
fn take_stance(
    mut bodies: Query<(
        &mut Stance,
        &mut Animator,
        Option<&mut Fighter>,
        Option<&Condition>,
        Option<&Ragdoll>,
        Option<&Attacking>,
        Has<Fallen>,
    )>,
) {
    for (mut stance, mut anim, fighter, condition, body, attacking, fallen) in &mut bodies {
        let condition = condition.copied().unwrap_or_default();
        // A collapsing or dead fighter keeps nothing of its stance.
        if condition.dying() {
            continue;
        }
        if let Some(mut f) = fighter {
            let g = readiness_step(f.readiness, *stance, &anim.action);
            if f.readiness != g {
                f.readiness = g;
            }
        }
        if let Some(drive) = body.and_then(|b| b.drive.as_ref())
            && drive.dazed()
        {
            // The daze leaves a helpless or attacking fighter's clip alone
            // (by its condition), and Exarust's fallen one.
            let down = fallen || condition.helpless();
            let attacking =
                attack::playing(attacking, &anim).is_some() || condition == Condition::ATTACKING;
            let was = *stance;
            take_daze(&mut stance, &mut anim, drive.energy, down, attacking);
            if was != *stance {
                debug!("dazed at {:.2}: {:?}", drive.energy, *stance);
            }
        }
        let stance = *stance;
        // A clip played once that ends in a stance (an attack, a get-up)
        // ends in the stance the fighter holds when it ends; the falls keep
        // ending in their lying clip.
        if !anim.looping
            && anim
                .then
                .as_deref()
                .is_some_and(|t| Stance::of_action(t).is_some_and(|s| s != stance))
        {
            anim.then = Some(stance.action().into());
        }
        if fallen {
            continue;
        }
        let (balance, tone) = match body {
            Some(b) => match &b.drive {
                Some(d) => (d.balance().value, d.tone),
                // Limp: no muscles, no posture.
                None => continue,
            },
            None => (1.0, 1.0),
        };
        if switches_posture(&anim, stance, balance, tone) {
            debug!("{} -> {}", anim.action, stance.action());
            anim.start(stance.action(), true, None, POSTURE_BLEND, Ease::Linear);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The player's rule against the live retail trace: in combat mode the
    /// opponent at 383–388 cm still gives the Defence stance and 350–356 cm
    /// the combat idle (a human's scale is 1); out of combat always Idle.
    #[test]
    fn the_player_guards_until_the_target_comes_within_350_cm() {
        let human = |d| Some((d, 1.0));
        for d in [3.88, 3.83, 3.51, 3.5] {
            assert_eq!(
                player_stance(true, 0.25, false, human(d), None),
                Stance::Defence
            );
        }
        for d in [3.49, 3.0, 0.9] {
            assert_eq!(
                player_stance(true, 0.25, false, human(d), None),
                Stance::CombatIdle
            );
        }
        // No target: the guard.
        assert_eq!(
            player_stance(true, 0.25, false, None, None),
            Stance::Defence
        );
        // A body twice as big is met twice as far.
        assert_eq!(
            player_stance(true, 0.25, false, Some((6.9, 2.0)), None),
            Stance::CombatIdle
        );
        for target in [None, human(1.0), human(9.0)] {
            assert_eq!(
                player_stance(false, 0.25, false, target, Some(0.0)),
                Stance::Idle
            );
        }
    }

    /// Within 3 s of its last attack's last tick the player stays in the
    /// combat idle, wherever the target is; dying (yellow under 0.01) it
    /// takes the Defence stance even with the target close.
    #[test]
    fn three_seconds_after_an_attack_and_dying() {
        for t in [0.0, 1.5, 2.99] {
            assert_eq!(
                player_stance(true, 0.25, false, None, Some(t)),
                Stance::CombatIdle
            );
        }
        assert_eq!(
            player_stance(true, 0.25, false, None, Some(3.0)),
            Stance::Defence
        );
        assert_eq!(
            player_stance(true, 0.009, false, Some((1.0, 1.0)), Some(0.0)),
            Stance::Defence
        );
        assert_eq!(
            player_stance(true, 0.011, false, Some((1.0, 1.0)), None),
            Stance::CombatIdle
        );
    }

    /// A dazed player (energy more than 0.05 under its most) guards even
    /// with its target close or just after an attack; out of combat it
    /// stays Idle.
    #[test]
    fn a_dazed_player_guards() {
        let close = Some((1.0, 1.0));
        assert_eq!(
            player_stance(true, 0.25, true, close, None),
            Stance::Defence
        );
        assert_eq!(
            player_stance(true, 0.25, true, None, Some(0.5)),
            Stance::Defence
        );
        assert_eq!(player_stance(false, 0.25, true, close, None), Stance::Idle);
        assert_eq!(
            player_stance(true, 0.25, false, close, None),
            Stance::CombatIdle
        );
    }

    /// The target is the best (5 − d)(cos + 0.5) within 5 m: one 120° or
    /// more off the cursor's direction is never picked, a near one beside
    /// the cursor beats a far one in line with it.
    #[test]
    fn the_target_is_picked_by_distance_and_angle() {
        let at = |x: f32, z: f32| Vec3::new(x, 1.3, z);
        let from = at(0.0, 0.0);
        let aim = Vec3::Z;
        assert_eq!(pick_target(from, aim, [(1, at(0.0, 3.0))]), Some((1, 3.0)));
        // Behind, and 121° off: no target; 119° off: picked.
        assert_eq!(pick_target(from, aim, [(1, at(0.0, -2.0))]), None);
        let off = |deg: f32| at(2.0 * deg.to_radians().sin(), 2.0 * deg.to_radians().cos());
        assert_eq!(pick_target(from, aim, [(1, off(121.0))]), None);
        assert_eq!(
            pick_target(from, aim, [(1, off(119.0))]).map(|t| t.0),
            Some(1)
        );
        // Beyond 5 m: none.
        assert_eq!(pick_target(from, aim, [(1, at(0.0, 5.01))]), None);
        // 4 m ahead scores 1·1.5; 2 m at 60° scores 3·1.0.
        let side = 60f32.to_radians();
        let near = at(2.0 * side.sin(), 2.0 * side.cos());
        let picked = pick_target(from, aim, [(1, at(0.0, 4.0)), (2, near)]).unwrap();
        assert_eq!(picked.0, 2);
        assert!((picked.1 - 2.0).abs() < 1e-5);
    }

    /// A duellist guards at 4 reaches or more: a human with a sword of
    /// 1.2 m reach from 4.8 m, unarmed (0.5 m) from 2 m.
    #[test]
    fn the_duellist_guards_beyond_four_reaches() {
        assert_eq!(duellist_stance(4.8, 1.2), Stance::Defence);
        assert_eq!(duellist_stance(4.79, 1.2), Stance::CombatIdle);
        assert_eq!(duellist_stance(2.0, UNARMED_REACH), Stance::Defence);
        assert_eq!(duellist_stance(1.99, UNARMED_REACH), Stance::CombatIdle);
    }

    /// The helper starts the new stance only when the old one's clip plays.
    #[test]
    fn setting_the_stance_starts_it_from_the_old_one() {
        let mut anim = Animator::new("Default", "Idle");
        let mut stance = Stance::Idle;
        set_stance(&mut stance, &mut anim, Stance::Defence);
        assert_eq!(
            (stance, anim.action.as_str()),
            (Stance::Defence, "Defence stance")
        );
        // From where the body is, over 30 frames past the first key,
        // smoothly.
        assert_eq!((anim.blend, anim.ease), (30.0, Ease::Smooth));
        assert!(anim.looping && anim.pending.is_some());
        // Walking: the stance changes, the clip does not.
        anim.play_loop("Combat Walk");
        set_stance(&mut stance, &mut anim, Stance::CombatIdle);
        assert_eq!(
            (stance, anim.action.as_str()),
            (Stance::CombatIdle, "Combat Walk")
        );
    }

    /// The posture switch: from Idle or the guard to the stance when
    /// balanced, never from the combat idle (so a fighter stays there when
    /// its target leaves), the walk, the run or the interim footwork, nor
    /// below a balance of 0.8 or a tone of 0.85, nor during a clip played
    /// once.
    #[test]
    fn the_posture_switch_skips_the_combat_idle() {
        let anim = |a: &str| Animator::new("Default", a);
        assert!(switches_posture(&anim("Idle"), Stance::Defence, 1.0, 1.0));
        assert!(switches_posture(
            &anim("Defence stance"),
            Stance::CombatIdle,
            0.8,
            0.85
        ));
        assert!(!switches_posture(
            &anim("Cmbt - Idle"),
            Stance::Defence,
            1.0,
            1.0
        ));
        for a in ["Walk", "Run", "Backpedal", "Combat Walk"] {
            assert!(
                !switches_posture(&anim(a), Stance::Defence, 1.0, 1.0),
                "{a}"
            );
        }
        assert!(!switches_posture(&anim("Idle"), Stance::Defence, 0.79, 1.0));
        assert!(!switches_posture(&anim("Idle"), Stance::Defence, 1.0, 0.84));
        assert!(!switches_posture(&anim("Idle"), Stance::Idle, 1.0, 1.0));
        let mut attacking = anim("Defence stance");
        attacking.play_once("Attack Mid", "Defence stance");
        assert!(!switches_posture(&attacking, Stance::CombatIdle, 1.0, 1.0));
    }

    /// A duellist's stance starts with 40 frames of blend without a primary
    /// striker, twice its stance blend with one (none for the fists), eased
    /// smoothly; playing already, it only eases the blend smoothly.
    #[test]
    fn a_duellist_starts_its_stance_by_its_primary() {
        assert_eq!(ai_stance_blend(None), 40);
        assert_eq!(ai_stance_blend(Some(7)), 14);
        assert_eq!(ai_stance_blend(Some(0)), 0);
        let mut anim = Animator::new("One Handed", "Combat Walk");
        take_ai_stance(&mut anim, Stance::Defence, Some(7));
        assert_eq!(anim.action, "Defence stance");
        assert_eq!((anim.blend, anim.ease), (14.0, Ease::Smooth));
        anim.pending = None;
        anim.ease = Ease::Linear;
        take_ai_stance(&mut anim, Stance::Defence, None);
        assert_eq!((anim.blend, anim.ease), (14.0, Ease::Smooth));
        assert!(anim.pending.is_none());
    }

    /// The daze's blend: round((1.05 − E) × 300), 45 frames at 0.9, 129 at
    /// 0.62, 300 at the floor of 0.05, 30 at 0.95.
    #[test]
    fn the_daze_blends_by_the_energy_left() {
        assert_eq!(daze_blend(0.9), 45);
        assert_eq!(daze_blend(0.62), 129);
        assert_eq!(daze_blend(0.05), 300);
        assert_eq!(daze_blend(0.95), 30);
    }

    /// Dazed, a fighter's stance becomes the Defence stance and its clip
    /// starts it (blend by the energy, smooth) unless it plays it already;
    /// down or attacking, only the stance changes.
    #[test]
    fn a_dazed_fighter_takes_the_defence_stance() {
        let mut stance = Stance::CombatIdle;
        let mut anim = Animator::new("Default", "Cmbt - Idle");
        take_daze(&mut stance, &mut anim, 0.62, false, false);
        assert_eq!(stance, Stance::Defence);
        assert_eq!(anim.action, "Defence stance");
        assert_eq!((anim.blend, anim.ease), (129.0, Ease::Smooth));
        assert!(anim.looping && anim.pending.is_some());
        // Next tick: no restart, the ease kept smooth.
        anim.pending = None;
        anim.time = 5.0;
        anim.ease = Ease::Linear;
        take_daze(&mut stance, &mut anim, 0.63, false, false);
        assert_eq!(
            (anim.time, anim.blend, anim.ease),
            (5.0, 129.0, Ease::Smooth)
        );
        for (fallen, attacking) in [(true, false), (false, true)] {
            let mut stance = Stance::CombatIdle;
            let mut anim = Animator::new("Default", "Attack Mid");
            anim.looping = false;
            take_daze(&mut stance, &mut anim, 0.62, fallen, attacking);
            assert_eq!(stance, Stance::Defence);
            assert_eq!(anim.action, "Attack Mid");
        }
    }

    /// Readiness: 0.004 a tick in the combat idle (0 to 1 in 250 ticks, 3 s
    /// at 83 Hz), 0 at once in any other stance or clip.
    #[test]
    fn readiness_rises_in_the_combat_idle_only() {
        let mut g = 0.0;
        for _ in 0..125 {
            g = readiness_step(g, Stance::CombatIdle, "Cmbt - Idle");
        }
        assert!((g - 0.5).abs() < 1e-4, "{g}");
        for _ in 0..200 {
            g = readiness_step(g, Stance::CombatIdle, "Cmbt - Idle");
        }
        assert_eq!(g, 1.0);
        assert_eq!(readiness_step(g, Stance::Defence, "Cmbt - Idle"), 0.0);
        assert_eq!(readiness_step(g, Stance::CombatIdle, "Attack Mid"), 0.0);
        assert_eq!(readiness_step(g, Stance::CombatIdle, "Combat Walk"), 0.0);
    }

    fn ticked() -> App {
        let mut app = App::new();
        app.add_systems(Update, take_stance);
        app
    }

    /// The body tick part: Idle switches to the guard; the guard to the
    /// combat idle when the stance changes; the combat idle stays when the
    /// stance turns back to the guard; an attack started for the old
    /// stance ends in the new one; a fall keeps its lying clip.
    #[test]
    fn fighters_take_their_stance_in_the_body_tick() {
        let mut app = ticked();
        let id = app
            .world_mut()
            .spawn((
                Stance::Defence,
                Animator::new("Default", "Idle"),
                Fighter::default(),
            ))
            .id();
        let action = |app: &App| app.world().get::<Animator>(id).unwrap().action.clone();
        app.update();
        assert_eq!(action(&app), "Defence stance");
        // The posture switch blends 30 frames, linear.
        let anim = app.world().get::<Animator>(id).unwrap();
        assert_eq!((anim.blend, anim.ease), (30.0, Ease::Linear));
        *app.world_mut().get_mut::<Stance>(id).unwrap() = Stance::CombatIdle;
        app.update();
        assert_eq!(action(&app), "Cmbt - Idle");
        *app.world_mut().get_mut::<Stance>(id).unwrap() = Stance::Defence;
        for _ in 0..10 {
            app.update();
        }
        assert_eq!(action(&app), "Cmbt - Idle");
        app.world_mut()
            .get_mut::<Animator>(id)
            .unwrap()
            .play_once("Attack Mid", "Cmbt - Idle");
        app.update();
        let anim = app.world().get::<Animator>(id).unwrap();
        assert_eq!(anim.then.as_deref(), Some("Defence stance"));
        app.world_mut()
            .get_mut::<Animator>(id)
            .unwrap()
            .play_once("Fall Backward", "Unconcious 1");
        app.update();
        let anim = app.world().get::<Animator>(id).unwrap();
        assert_eq!(anim.then.as_deref(), Some("Unconcious 1"));
    }

    /// The daze in the body tick: a fighter whose energy is 0.62 takes the
    /// Defence stance from the combat idle, blending 129 frames smoothly;
    /// once it has its wits back and its stance is the combat idle again,
    /// the posture switch blends back over 30 frames, linear.
    #[test]
    fn a_dazed_body_guards_and_blends_back() {
        let mut app = ticked();
        let mut body = Ragdoll::new(&[]).with_drive();
        body.drive.as_mut().unwrap().energy = 0.62;
        let id = app
            .world_mut()
            .spawn((
                Stance::CombatIdle,
                Animator::new("Default", "Cmbt - Idle"),
                Fighter::default(),
                body,
            ))
            .id();
        app.update();
        let anim = app.world().get::<Animator>(id).unwrap();
        assert_eq!(anim.action, "Defence stance");
        assert_eq!((anim.blend, anim.ease), (129.0, Ease::Smooth));
        assert_eq!(*app.world().get::<Stance>(id).unwrap(), Stance::Defence);
        let mut r = app.world_mut().get_mut::<Ragdoll>(id).unwrap();
        r.drive.as_mut().unwrap().energy = 1.0;
        *app.world_mut().get_mut::<Stance>(id).unwrap() = Stance::CombatIdle;
        app.update();
        let anim = app.world().get::<Animator>(id).unwrap();
        assert_eq!(anim.action, "Cmbt - Idle");
        assert_eq!((anim.blend, anim.ease), (30.0, Ease::Linear));
    }

    /// Readiness rises in the body tick while the combat idle plays in that
    /// stance, and a dead fighter is left as it is.
    #[test]
    fn readiness_ticks_with_the_body() {
        let mut app = ticked();
        let id = app
            .world_mut()
            .spawn((
                Stance::CombatIdle,
                Animator::new("Default", "Cmbt - Idle"),
                Fighter::default(),
            ))
            .id();
        for _ in 0..10 {
            app.update();
        }
        let g = app.world().get::<Fighter>(id).unwrap().readiness;
        assert!((g - 0.04).abs() < 1e-5, "{g}");
        *app.world_mut().get_mut::<Condition>(id).unwrap() = Condition::DEAD;
        *app.world_mut().get_mut::<Stance>(id).unwrap() = Stance::Defence;
        app.update();
        let f = app.world().get::<Fighter>(id).unwrap();
        assert!((f.readiness - 0.04).abs() < 1e-5);
        assert_eq!(
            app.world().get::<Animator>(id).unwrap().action,
            "Cmbt - Idle"
        );
    }
}

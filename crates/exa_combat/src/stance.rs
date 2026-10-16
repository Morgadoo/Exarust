//! The combat stance: the looping "home" clip a fighter returns to, as the
//! original keeps it on every character. Every clip a fighter plays once
//! ends in it, and a balanced fighter standing in another looping clip
//! switches to it ([`take_stance`]). Who chooses it: the player's
//! behaviour in and out of combat mode ([`player_stance`]), a duellist by
//! its distance to its target ([`duellist_stance`]). Rules:
//! `docs/PLAYER.md` (The combat stance).
//!
//! Not yet: the dazed fighter's Defence stance and its pose hold, and the
//! blends of every stance change (C10); the Defence stance and Rebalance a
//! fighter takes from its balance or its tone (C11); parries, whose clips
//! are stances while they last (L10).

use bevy::prelude::*;
use exa_anim::Animator;
use exa_physics::body::{CharacterTicks, Fallen, Ragdoll};

use crate::Fighter;
use crate::damage::DEATH;

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
/// Where a body's chest is (m above its feet) before its balance has
/// measured it.
const CHEST_HEIGHT: f32 = 1.3;
/// Looping clips the posture switch leaves alone: the combat idle (the
/// original's body tick handles it in its own branch, which skips the
/// switch), the walk and the run (each its own branch too), and the
/// backpedal and combat walk of Exarust's interim footwork.
const KEPT: [&str; 5] = ["Cmbt - Idle", "Walk", "Run", "Backpedal", "Combat Walk"];

pub struct StancePlugin;

impl Plugin for StancePlugin {
    fn build(&self, app: &mut App) {
        // The body tick's part: before each living body's clip advances.
        app.add_systems(FixedUpdate, take_stance.before(CharacterTicks));
    }
}

/// The player's stance, chosen every behaviour tick: Idle out of combat
/// mode; in combat mode the Defence stance while dying (yellow health
/// under 0.01), the combat idle with its target nearer than 3.5 m times
/// the target's body scale or within 3 s of its last attack's last tick,
/// else the Defence stance. `target` is the target's distance (m) and
/// body scale, `since_attack` the seconds since that tick.
///
/// Not yet: a parrying player keeps its stance (no parries), and a dazed
/// one (energy more than 0.05 under its cap) takes the Defence stance
/// with C10's pose hold, without which it would raise its guard at every
/// dazing blow.
pub fn player_stance(
    combat: bool,
    yellow: f32,
    target: Option<(f32, f32)>,
    since_attack: Option<f32>,
) -> Stance {
    if !combat {
        Stance::Idle
    } else if yellow < DEATH {
        Stance::Defence
    } else if target.is_some_and(|(d, scale)| d < CLOSE * scale)
        || since_attack.is_some_and(|t| t < AFTER_ATTACK)
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
/// playing is the old stance's, the new stance starts at once.
pub fn set_stance(stance: &mut Stance, anim: &mut Animator, new: Stance) {
    if new != *stance && anim.looping && anim.action == stance.action() {
        anim.play_loop(new.action());
    }
    *stance = new;
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
/// readiness, the clip it plays once ending in its stance, and the posture
/// switch.
#[allow(clippy::type_complexity)]
fn take_stance(
    mut bodies: Query<(
        &Stance,
        &mut Animator,
        Option<&mut Fighter>,
        Option<&Ragdoll>,
        Has<Fallen>,
    )>,
) {
    for (&stance, mut anim, fighter, body, fallen) in &mut bodies {
        if let Some(mut f) = fighter {
            if f.down {
                continue;
            }
            let g = readiness_step(f.readiness, stance, &anim.action);
            if f.readiness != g {
                f.readiness = g;
            }
        }
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
            anim.play_loop(stance.action());
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
            assert_eq!(player_stance(true, 0.25, human(d), None), Stance::Defence);
        }
        for d in [3.49, 3.0, 0.9] {
            assert_eq!(
                player_stance(true, 0.25, human(d), None),
                Stance::CombatIdle
            );
        }
        // No target: the guard.
        assert_eq!(player_stance(true, 0.25, None, None), Stance::Defence);
        // A body twice as big is met twice as far.
        assert_eq!(
            player_stance(true, 0.25, Some((6.9, 2.0)), None),
            Stance::CombatIdle
        );
        for target in [None, human(1.0), human(9.0)] {
            assert_eq!(player_stance(false, 0.25, target, Some(0.0)), Stance::Idle);
        }
    }

    /// Within 3 s of its last attack's last tick the player stays in the
    /// combat idle, wherever the target is; dying (yellow under 0.01) it
    /// takes the Defence stance even with the target close.
    #[test]
    fn three_seconds_after_an_attack_and_dying() {
        for t in [0.0, 1.5, 2.99] {
            assert_eq!(player_stance(true, 0.25, None, Some(t)), Stance::CombatIdle);
        }
        assert_eq!(player_stance(true, 0.25, None, Some(3.0)), Stance::Defence);
        assert_eq!(
            player_stance(true, 0.009, Some((1.0, 1.0)), Some(0.0)),
            Stance::Defence
        );
        assert_eq!(
            player_stance(true, 0.011, Some((1.0, 1.0)), None),
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
        app.world_mut().get_mut::<Fighter>(id).unwrap().down = true;
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

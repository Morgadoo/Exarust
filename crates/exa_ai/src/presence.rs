//! An AI character's presence in the scene: the original takes a character
//! out of the scene when it leaves the activation range and puts it back,
//! reset, when it comes into range again.
//!
//! - **Range** ([`in_range`]): the character's sector is live and its body
//!   lies strictly within the activation radius of the centre (a squared
//!   distance in single precision, `(dx² + dy²) + dz²`, a NaN out of range).
//!   The start of play centres the area on the player with 40 m, the end
//!   puts it back at the origin with 200 m, and while play runs the player
//!   controller moves the centre to the player's body every tick.
//! - **Presence** ([`Presence::step`]): each AI tick, a body neither
//!   collapsing nor dead (condition bit 0x800) and not lying dormant
//!   (state flag 0x10) first waits out its hold (80 ticks after a change).
//!   Out of range a present character starts leaving; in range a dormant
//!   one starts entering. Leaving ends dormant and entering ends present
//!   once the scene has moved its node ([`Presence::settle`]).
//! - **Entering** resets the body: its condition clears, it takes its
//!   stance, its stepping, run, moving and gait flags and velocities are
//!   zeroed, its balance, tone and energy return to 1, and its cast hook is
//!   asked event −1 and cleared with its power ([`Entered`]).
//! - Only a present character's body moves under its AI; a dormant or
//!   entering one is out of the scene ([`OutOfScene`]): the solver, its
//!   character tick and the drawing leave it.
//!
//! Exarust keeps no live sector per character, so every sector reads live,
//! and the scene's node move completes on the next AI tick. The entering
//! body's re-placement, its part handles and the node flag word are not
//! reproduced.
//!
//! Evidence: the AI tick's range check, the scene's detach and attach
//! callbacks, the AI manager frame, the body reset and the player
//! controller's centre, read from `Exanima.exe`.

use bevy::prelude::*;
use exa_anim::Animator;
use exa_combat::Condition;
use exa_combat::condition::StateFlags;
use exa_combat::stance::{self, Stance};
use exa_physics::body::locomotion::{Locomotion, Locomotor};
use exa_physics::body::{OutOfScene, Ragdoll};

use exa_combat::Fighter;

use crate::Duellist;
use crate::combatant::Combatant;

/// The squared activation radius (cm²) outside play: 200 m.
pub const OPEN_RADIUS2: f32 = 400_000_000.0;
/// The ticks a presence change holds before the next check.
pub const HOLD_TICKS: u32 = 80;

/// Where the AI characters are in the scene: the centre (cm) and the
/// squared radius (cm²) of the activation range.
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct ActivationArea {
    pub centre_cm: Vec3,
    pub radius2: f32,
}

impl Default for ActivationArea {
    fn default() -> Self {
        Self {
            centre_cm: Vec3::ZERO,
            radius2: OPEN_RADIUS2,
        }
    }
}

/// Where a character stands toward the scene.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PresenceState {
    /// Out of the scene.
    Dormant = 0,
    /// In the scene.
    #[default]
    Present = 1,
    /// Its node is leaving the scene.
    Leaving = 2,
    /// Its node is entering the scene.
    Entering = 4,
}

/// A presence change the range check starts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Change {
    Leave,
    Enter,
}

/// An AI character's presence and its hold.
#[derive(Component, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Presence {
    pub state: PresenceState,
    /// Ticks left before the range is checked again.
    pub hold: u32,
}

impl Presence {
    /// One AI tick's range check.
    pub fn step(&mut self, in_range: bool) -> Option<Change> {
        if self.hold > 0 {
            self.hold -= 1;
            return None;
        }
        let change = match (in_range, self.state) {
            (false, PresenceState::Present) => {
                self.state = PresenceState::Leaving;
                Change::Leave
            }
            (true, PresenceState::Dormant) => {
                self.state = PresenceState::Entering;
                Change::Enter
            }
            _ => return None,
        };
        self.hold = HOLD_TICKS;
        Some(change)
    }

    /// The scene's node move ends: leaving becomes dormant, entering
    /// present.
    pub fn settle(&mut self) {
        self.state = match self.state {
            PresenceState::Leaving => PresenceState::Dormant,
            PresenceState::Entering => PresenceState::Present,
            state => state,
        };
    }

    /// Its body is in the scene.
    pub fn in_scene(&self) -> bool {
        matches!(self.state, PresenceState::Present | PresenceState::Leaving)
    }

    /// Its AI moves its body.
    pub fn moves(&self) -> bool {
        self.state == PresenceState::Present
    }
}

/// What the level-leave reset did.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Reentry {
    /// The lasting health came back to the current pool.
    pub healed: bool,
    /// The contacts and the target cleared.
    pub cleared: bool,
    /// A roaming AI was reset: it left combat and was put down again.
    pub left_combat: bool,
    /// Where the body was put, when a roaming AI was moved.
    pub moved_to: Option<Vec3>,
}

/// The span a planned wander path must beat (cm², 150 cm) before a
/// resetting roam walks it.
pub const REENTRY_SPAN_CM2: f32 = 22_500.0;
/// The rounds of wander-and-home plans a resetting roam makes.
pub const REENTRY_ROUNDS: u32 = 2;
/// The lasting health a reset needs to restore the current pool from it.
pub const HEAL_FLOOR: f32 = 0.01;

/// The reset the original gives every AI character when the level is
/// written out as the player leaves it, so the character is not found as
/// it was left. A collapsed or dead body (`out`) or a dormant one
/// (`dormant`) is left alone. Otherwise the body's current health returns
/// to its lasting pool when that is over [`HEAL_FLOOR`], and every contact
/// and the target are forgotten. A roaming AI that is not a statue is then
/// put down along freshly planned wander paths, twice: a wander endpoint
/// whose path spans over [`REENTRY_SPAN_CM2`] becomes the body's goal, and
/// from there a home-side endpoint over the same span; the body ends at the
/// last goal, if any, and leaves combat. `wander` and `home` return a
/// planned endpoint and the squared ground span from the path's first point
/// to it, or none when the level has no room to draw one from.
///
/// Exarust does not write levels on leaving, so nothing calls this yet.
#[allow(clippy::too_many_arguments)]
pub fn reenter(
    out: bool,
    dormant: bool,
    roaming: bool,
    statue: bool,
    health: &mut Fighter,
    contacts: &mut Combatant,
    wander: &mut dyn FnMut() -> Option<(Vec3, f32)>,
    home: &mut dyn FnMut() -> Option<(Vec3, f32)>,
) -> Reentry {
    let mut result = Reentry::default();
    if out || dormant {
        return result;
    }
    if HEAL_FLOOR < health.health.red {
        health.health.yellow = health.health.red;
        result.healed = true;
    }
    contacts.reset();
    result.cleared = true;
    if !roaming || statue {
        return result;
    }
    let mut goal = None;
    for _ in 0..REENTRY_ROUNDS {
        if let Some((point, span)) = wander()
            && REENTRY_SPAN_CM2 < span
        {
            goal = Some(point);
            if let Some((point, span)) = home()
                && REENTRY_SPAN_CM2 < span
            {
                goal = Some(point);
            }
        }
    }
    result.moved_to = goal;
    result.left_combat = true;
    result
}

/// Whether a body at squared distance `distance2` (cm²) from the centre is
/// in the activation range of squared radius `radius2`.
pub fn in_range(sector_live: bool, distance2: f32, radius2: f32) -> bool {
    sector_live && distance2 < radius2
}

/// The squared distance (cm²) between two points, summed as the original
/// sums it.
pub fn distance2(a: Vec3, b: Vec3) -> f32 {
    let d = a - b;
    (d.x * d.x + d.y * d.y) + d.z * d.z
}

/// An AI character entered the scene: the game asks its cast hook event −1
/// and clears its cast.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entered(pub Entity);

/// Each body tick, every AI character's presence: the node moves the last
/// tick started end, then the range check, with the reset of an entering
/// body.
#[allow(clippy::type_complexity)]
pub fn check_presence(
    mut commands: Commands,
    area: Res<ActivationArea>,
    roots: Query<&GlobalTransform>,
    mut characters: Query<
        (
            Entity,
            &mut Presence,
            &ChildOf,
            &mut Condition,
            Option<&StateFlags>,
            Option<&mut Animator>,
            Option<&Stance>,
            Option<&mut Locomotion>,
            Option<&mut Locomotor>,
            Option<&mut Ragdoll>,
        ),
        With<Duellist>,
    >,
    mut entered: MessageWriter<Entered>,
) {
    for (
        entity,
        mut presence,
        parent,
        mut condition,
        flags,
        anim,
        stance,
        locomotion,
        locomotor,
        ragdoll,
    ) in &mut characters
    {
        let was = presence.state;
        presence.settle();
        if presence.state != was {
            show(&mut commands, entity, presence.in_scene());
        }
        if condition.0 & Condition::COLLAPSING.0 != 0
            || flags.is_some_and(|f| f.has(StateFlags::DORMANT))
        {
            continue;
        }
        let Ok(root) = roots.get(parent.parent()) else {
            continue;
        };
        let near = in_range(
            true,
            distance2(root.translation() * 100.0, area.centre_cm),
            area.radius2,
        );
        if presence.step(near) == Some(Change::Enter) {
            *condition = Condition::NORMAL;
            if let Some(mut anim) = anim {
                stance::take_ai_stance(&mut anim, stance.copied().unwrap_or_default(), None);
            }
            if let Some(mut locomotion) = locomotion {
                locomotion.moving = false;
                locomotion.stepping = None;
                locomotion.flags = 0;
                locomotion.free_walk = false;
                locomotion.slow_turn = false;
                locomotion.goal = Vec3::ZERO;
                locomotion.look_target = None;
                locomotion.look_until = 0;
            }
            if let Some(mut locomotor) = locomotor {
                *locomotor = Locomotor {
                    yaw: locomotor.yaw,
                    stance_facing: locomotor.stance_facing,
                    frame_yaw: locomotor.frame_yaw,
                    ..Locomotor::default()
                };
            }
            if let Some(mut ragdoll) = ragdoll {
                ragdoll.reset_presence();
            }
            entered.write(Entered(entity));
            show(&mut commands, entity, false);
        }
    }
}

/// Puts a character's body into the scene or takes it out.
fn show(commands: &mut Commands, entity: Entity, in_scene: bool) {
    if in_scene {
        commands
            .entity(entity)
            .remove::<OutOfScene>()
            .insert(Visibility::Inherited);
    } else {
        commands
            .entity(entity)
            .insert((OutOfScene, Visibility::Hidden));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_range_is_strict_and_a_nan_is_out() {
        assert!(in_range(true, 399_999_980.0, OPEN_RADIUS2));
        assert!(!in_range(true, OPEN_RADIUS2, OPEN_RADIUS2));
        assert!(!in_range(true, f32::NAN, OPEN_RADIUS2));
        assert!(!in_range(false, 0.0, OPEN_RADIUS2));
        assert_eq!(
            distance2(Vec3::new(3.0, 4.0, 12.0), Vec3::ZERO),
            (9.0 + 16.0) + 144.0
        );
    }

    /// A duellist 300 m out leaves the scene; once the area reaches it, it
    /// comes back after its hold, reset.
    #[test]
    fn a_character_out_of_range_leaves_the_scene_and_comes_back_reset() {
        let mut app = App::new();
        app.init_resource::<ActivationArea>()
            .add_message::<Entered>()
            .add_systems(FixedUpdate, check_presence);
        let root = app
            .world_mut()
            .spawn(GlobalTransform::from_translation(Vec3::new(
                300.0, 0.0, 0.0,
            )))
            .id();
        let foe = app
            .world_mut()
            .spawn((
                Duellist::new(root),
                Condition::CASTING,
                Animator::new("Default", "Idle"),
                ChildOf(root),
            ))
            .id();
        let tick = |app: &mut App| app.world_mut().run_schedule(FixedUpdate);
        tick(&mut app);
        let presence = *app.world().get::<Presence>(foe).unwrap();
        assert_eq!(presence.state, PresenceState::Leaving);
        assert!(
            app.world().get::<OutOfScene>(foe).is_none(),
            "still in the scene"
        );
        tick(&mut app);
        assert!(app.world().get::<OutOfScene>(foe).is_some());
        assert_eq!(
            app.world().get::<Visibility>(foe),
            Some(&Visibility::Hidden)
        );

        app.world_mut().resource_mut::<ActivationArea>().centre_cm = Vec3::new(30_000.0, 0.0, 0.0);
        for _ in 0..79 {
            tick(&mut app);
        }
        assert_eq!(
            app.world().get::<Presence>(foe).unwrap().state,
            PresenceState::Dormant,
            "the hold"
        );
        tick(&mut app);
        assert_eq!(
            app.world().get::<Presence>(foe).unwrap().state,
            PresenceState::Entering
        );
        assert_eq!(
            *app.world().get::<Condition>(foe).unwrap(),
            Condition::NORMAL
        );
        let sent: Vec<Entered> = app
            .world_mut()
            .resource_mut::<Messages<Entered>>()
            .drain()
            .collect();
        assert_eq!(sent, vec![Entered(foe)]);
        tick(&mut app);
        assert!(app.world().get::<Presence>(foe).unwrap().moves());
        assert!(app.world().get::<OutOfScene>(foe).is_none());
    }

    /// Entering resets the body's stepping, run, moving and gait flags and
    /// its velocities, and sets balance, tone and energy to 1.
    #[test]
    fn entering_resets_the_body() {
        let mut app = App::new();
        app.init_resource::<ActivationArea>()
            .add_message::<Entered>()
            .add_systems(FixedUpdate, check_presence);
        let root = app.world_mut().spawn(GlobalTransform::default()).id();
        let mut body = Ragdoll::new(&[]).with_drive();
        {
            let drive = body.drive.as_mut().unwrap();
            drive.tone = 0.2;
            drive.energy = 0.3;
            drive.balance_mut().value = 0.4;
        }
        let foe = app
            .world_mut()
            .spawn((
                Duellist::new(root),
                Condition::NORMAL,
                Animator::new("Default", "Idle"),
                ChildOf(root),
                body,
                Locomotion {
                    moving: true,
                    stepping: Some(Vec3::X),
                    flags: 0x4,
                    free_walk: true,
                    ..Locomotion::default()
                },
                Locomotor {
                    speed: 3.0,
                    run: 0.5,
                    drive: Vec3::X,
                    going: Vec3::X,
                    ..Locomotor::default()
                },
                Presence {
                    state: PresenceState::Dormant,
                    hold: 0,
                },
            ))
            .id();
        app.world_mut().run_schedule(FixedUpdate);
        assert_eq!(
            app.world().get::<Presence>(foe).unwrap().state,
            PresenceState::Entering
        );
        let body = app.world().get::<Ragdoll>(foe).unwrap();
        let drive = body.drive.as_ref().unwrap();
        assert_eq!(
            (drive.tone, drive.energy, drive.balance().value),
            (1.0, 1.0, 1.0)
        );
        let locomotion = app.world().get::<Locomotion>(foe).unwrap();
        assert!(!locomotion.moving && locomotion.stepping.is_none());
        assert_eq!((locomotion.flags, locomotion.free_walk), (0, false));
        let locomotor = app.world().get::<Locomotor>(foe).unwrap();
        assert_eq!(
            (
                locomotor.speed,
                locomotor.run,
                locomotor.drive,
                locomotor.going
            ),
            (0.0, 0.0, Vec3::ZERO, Vec3::ZERO)
        );
    }

    /// The level-leave reset heals the current pool from the lasting one,
    /// clears the contacts, and moves a roaming AI along its plans twice,
    /// leaving combat; a statue stays where it is and stays in combat; a
    /// collapsed or dormant body is not touched.
    #[test]
    fn the_level_leave_reset_heals_clears_and_moves_a_roam() {
        let mut fighter = Fighter::default();
        fighter.health.yellow = 0.2;
        fighter.health.red = 0.6;
        let mut contacts = Combatant::default();
        let target = contacts.contact_index(Entity::PLACEHOLDER);
        contacts.contacts[target].concern = 3.0;
        let mut calls = 0;
        let mut wander = || {
            calls += 1;
            Some((Vec3::new(calls as f32 * 100.0, 0.0, 0.0), 30_000.0))
        };
        let mut home = || Some((Vec3::new(0.0, 0.0, 500.0), 10.0));
        let done = reenter(
            false,
            false,
            true,
            false,
            &mut fighter,
            &mut contacts,
            &mut wander,
            &mut home,
        );
        assert_eq!(fighter.health.yellow, 0.6);
        assert!(done.healed && done.cleared && done.left_combat);
        assert!(contacts.contacts.is_empty());
        // Two rounds of wander, each over the span (100 cm, then 200 cm);
        // the home point is under the span, so the last wander wins.
        assert_eq!(calls, 2);
        assert_eq!(done.moved_to, Some(Vec3::new(200.0, 0.0, 0.0)));
        // A home point over the span wins its round.
        let mut fighter = Fighter::default();
        let mut contacts = Combatant::default();
        let mut wander = || Some((Vec3::new(100.0, 0.0, 0.0), 22_501.0));
        let mut home = || Some((Vec3::new(0.0, 0.0, 500.0), 22_501.0));
        let done = reenter(
            false,
            false,
            true,
            false,
            &mut fighter,
            &mut contacts,
            &mut wander,
            &mut home,
        );
        assert_eq!(done.moved_to, Some(Vec3::new(0.0, 0.0, 500.0)));
        // A statue does not move or leave combat; a short health pool does
        // not heal.
        let mut fighter = Fighter::default();
        fighter.health.yellow = 0.0;
        fighter.health.red = 0.01;
        let mut contacts = Combatant::default();
        let mut never = || None;
        let mut home = || -> Option<(Vec3, f32)> { None };
        let done = reenter(
            false,
            false,
            true,
            true,
            &mut fighter,
            &mut contacts,
            &mut never,
            &mut home,
        );
        assert_eq!(
            done,
            Reentry {
                healed: false,
                cleared: true,
                left_combat: false,
                moved_to: None
            }
        );
        assert_eq!(fighter.health.yellow, 0.0);
        // A collapsed or dormant body keeps everything.
        for (out, dormant) in [(true, false), (false, true)] {
            let mut fighter = Fighter::default();
            fighter.health.yellow = 0.2;
            fighter.health.red = 0.6;
            let mut contacts = Combatant::default();
            contacts.contact_index(Entity::PLACEHOLDER);
            let done = reenter(
                out,
                dormant,
                true,
                false,
                &mut fighter,
                &mut contacts,
                &mut never,
                &mut home,
            );
            assert_eq!(done, Reentry::default());
            assert_eq!(fighter.health.yellow, 0.2);
            assert_eq!(contacts.contacts.len(), 1);
        }
    }

    #[test]
    fn a_change_holds_eighty_ticks() {
        let mut p = Presence::default();
        assert_eq!(p.step(true), None);
        assert_eq!(p.step(false), Some(Change::Leave));
        assert_eq!((p.state, p.hold), (PresenceState::Leaving, 80));
        p.settle();
        assert_eq!(p.state, PresenceState::Dormant);
        for _ in 0..80 {
            assert_eq!(p.step(true), None);
        }
        assert_eq!(p.step(true), Some(Change::Enter));
        assert!(!p.in_scene());
        p.settle();
        assert!(p.moves());
    }
}

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
//!   stance, and its cast hook is asked event −1 and cleared with its power
//!   ([`Entered`]).
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
use exa_physics::body::OutOfScene;

use crate::Duellist;

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
        ),
        With<Duellist>,
    >,
    mut entered: MessageWriter<Entered>,
) {
    for (entity, mut presence, parent, mut condition, flags, anim, stance) in &mut characters {
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

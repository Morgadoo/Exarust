//! The target pass's turn: one brain a tick senses, admits and classifies
//! its contacts, the one that has waited longest.
//!
//! - **Rotation** ([`rotate_pass_turn`]): at the start of each tick, before
//!   any brain's AI tick, the turn passes to the brain the last tick named
//!   the longest waiter; the waiter and its wait are cleared, and the tick
//!   counter the waits are measured on steps by one.
//! - **Claim** ([`PassTurn::claim`]): each present brain's AI tick, after
//!   the integration's head, asks for the turn. The brain whose turn it is
//!   stamps the counter as its last pass and runs the target pass (its
//!   perception included). Every other present brain measures its wait,
//!   the counter less its last pass, and becomes the next tick's brain when
//!   that wait is strictly longer than the longest so far this tick, so an
//!   equal wait keeps the earlier brain. A brain that never had a pass has
//!   waited the whole counter.
//! - A brain that is not present (its presence state other than in the
//!   scene) neither runs the pass nor waits, and one knocked out, collapsed
//!   or dead leaves its AI tick before the integration, so it does not
//!   either. The brain whose turn it is does not wait on its own tick, so a
//!   lone brain has a pass every other tick, and `n` brains each have one
//!   every `n` ticks.
//! - Between its passes a brain still drifts its morale and integrates its
//!   contacts every tick: the concern follows the concern target its last
//!   pass left, the hostility and the noticed level drift, and the aim
//!   point follows the foe.
//!
//! The waits are counted in body ticks: the brains' tick
//! ([`crate::BrainTick`]) runs in the fixed step once per body tick, as the
//! original runs its AI from every body tick, and the counter is the game
//! loop's step counter as this tick's AI reads it: the body ticks run so
//! far plus this one (the original's hook list runs before the counter
//! advances, and the brains claim their turns after). Without a body clock
//! (unit tests) the counter steps once per rotation.
//!
//! Evidence: the game loop's pre-tick hook list (the rotation, which also
//! rotates the route planner's turn, [`crate::route::PlanTurn`]), the step
//! counter it advances, and the contact integration's present-state branch
//! (the turn test, the stamp and the wait), read from `Exanima.exe`.

use bevy::prelude::*;
use exa_physics::body::BodyTicks;

use crate::route::PlanTurn;

/// The target pass's turn: whose pass runs this tick, who has waited
/// longest so far, and each brain's last pass, on the tick counter
/// [`PlanTurn::clock`]. Without this resource every brain runs its pass
/// every tick (unit tests of a single brain).
#[derive(Resource, Debug, Clone, Default, PartialEq, Eq)]
pub struct PassTurn {
    pub turn: PlanTurn,
    /// How long the brain whose turn it is had waited when it was named.
    pub waited: u32,
}

impl PassTurn {
    /// The start of a tick: the counter steps and the longest waiter of the
    /// last tick takes the turn. Returns the brain whose turn it is.
    pub fn rotate(&mut self) -> Option<u64> {
        let next = self.turn.clock.wrapping_add(1);
        self.rotate_to(next)
    }

    /// The start of a tick on the game loop's step counter `counter` (the
    /// value this tick's brains stamp and measure their waits on): the
    /// longest waiter of the last tick takes the turn.
    pub fn rotate_to(&mut self, counter: u32) -> Option<u64> {
        self.waited = self.turn.longest_wait;
        self.turn.clock = counter;
        self.turn.next_turn()
    }

    /// A present brain asks for the turn on this tick: the brain whose turn
    /// it is stamps its pass and gets `true`; another records its wait.
    pub fn claim(&mut self, brain: u64) -> bool {
        let clock = self.turn.clock;
        self.turn.turn(brain, clock)
    }

    /// Whether `brain` runs the target pass this tick.
    pub fn is_turn(&self, brain: u64) -> bool {
        self.turn.current == Some(brain)
    }

    /// Forgets a brain that left the scene.
    pub fn forget(&mut self, brain: u64) {
        self.turn.last.retain(|(id, _)| *id != brain);
        if self.turn.current == Some(brain) {
            self.turn.current = None;
        }
        if self.turn.longest == Some(brain) {
            self.turn.longest = None;
            self.turn.longest_wait = 0;
        }
    }
}

/// Passes the target pass's turn once per body tick, before the perception
/// and the AI tick, on the step counter this tick's brains read (without a
/// body clock, once per run). With the `exa_ai::pass_turn` debug log on, it
/// names the brain whose turn it is.
pub fn rotate_pass_turn(
    ticks: Option<Res<BodyTicks>>,
    turn: Option<ResMut<PassTurn>>,
    mut rotated: Local<Option<u64>>,
) {
    let Some(mut turn) = turn else {
        return;
    };
    if let Some(ticks) = ticks.as_deref() {
        if *rotated == Some(ticks.0) {
            return;
        }
        *rotated = Some(ticks.0);
    }
    let current = match ticks.as_deref() {
        // The step counter after this tick's advance, as the brains read it.
        Some(ticks) => turn.rotate_to(ticks.0.wrapping_add(1) as u32),
        None => turn.rotate(),
    };
    if let Some(brain) = current {
        debug!(
            "body tick {}: the target pass's turn goes to {:?}, which waited {} ticks",
            turn.turn.clock,
            Entity::try_from_bits(brain).unwrap_or(Entity::PLACEHOLDER),
            turn.waited
        );
    } else {
        debug!(
            "body tick {}: no brain waited, no target pass",
            turn.turn.clock
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One tick: the rotation, then each present brain's claim in order.
    /// Returns the brain that ran its pass.
    fn tick(turn: &mut PassTurn, brains: &[u64]) -> Option<u64> {
        turn.rotate();
        let mut ran = None;
        for &b in brains {
            if turn.claim(b) {
                assert!(ran.is_none(), "two passes on one tick");
                ran = Some(b);
            }
        }
        ran
    }

    /// Three brains take the pass in turn, one a tick, in the order they
    /// first waited: the first tick names the first brain (an equal wait
    /// keeps the earlier one) and runs no pass.
    #[test]
    fn three_brains_take_the_pass_in_turn() {
        let mut turn = PassTurn::default();
        let brains = [10, 20, 30];
        let ran: Vec<_> = (0..8).map(|_| tick(&mut turn, &brains)).collect();
        assert_eq!(
            ran,
            [
                None,
                Some(10),
                Some(20),
                Some(30),
                Some(10),
                Some(20),
                Some(30),
                Some(10)
            ]
        );
        // Named on the tick before its pass, the brain had waited two
        // ticks: it runs every third tick.
        assert_eq!(turn.waited, 2);
    }

    /// A lone brain does not wait on its own tick, so it has a pass every
    /// other tick.
    #[test]
    fn a_lone_brain_has_a_pass_every_other_tick() {
        let mut turn = PassTurn::default();
        let ran: Vec<_> = (0..6).map(|_| tick(&mut turn, &[7])).collect();
        assert_eq!(ran, [None, Some(7), None, Some(7), None, Some(7)]);
    }

    /// A brain that never had a pass has waited the whole counter, so a
    /// newcomer jumps the queue ahead of brains that had one.
    #[test]
    fn a_newcomer_has_waited_the_whole_counter() {
        let mut turn = PassTurn::default();
        for _ in 0..5 {
            tick(&mut turn, &[1, 2]);
        }
        // Brain 2 ran on tick 5, so brain 1 runs on tick 6; a newcomer
        // claiming on tick 6 has waited 6 ticks, beats brain 2's one tick
        // and runs on tick 7, before brain 2.
        assert_eq!(tick(&mut turn, &[1, 2, 3]), Some(1));
        assert_eq!(tick(&mut turn, &[1, 2, 3]), Some(3));
        assert_eq!(tick(&mut turn, &[1, 2, 3]), Some(2));
    }

    /// A brain that stops claiming (not present, or knocked out) is not
    /// named, and when the named brain does not claim its tick no pass
    /// runs; the others' waits go on.
    #[test]
    fn a_brain_that_does_not_claim_loses_its_turn() {
        let mut turn = PassTurn::default();
        tick(&mut turn, &[1, 2]);
        assert_eq!(turn.turn.current, None);
        // Brain 1 is named; on its tick it is out and only brain 2 claims.
        assert_eq!(tick(&mut turn, &[2]), None);
        assert_eq!(tick(&mut turn, &[2]), Some(2));
        assert_eq!(tick(&mut turn, &[1, 2]), None);
        assert_eq!(tick(&mut turn, &[1, 2]), Some(1));
    }

    /// The waits are unsigned differences of a wrapping counter.
    #[test]
    fn the_wait_wraps_with_the_counter() {
        let mut turn = PassTurn::default();
        turn.turn.clock = u32::MAX - 1;
        turn.turn.last = vec![(1, u32::MAX - 3), (2, u32::MAX - 2)];
        tick(&mut turn, &[1, 2]);
        assert_eq!(turn.turn.longest, Some(1));
        assert_eq!(tick(&mut turn, &[1, 2]), Some(1));
        assert_eq!(turn.turn.clock, 0);
        assert_eq!(turn.turn.last_plan(1), Some(0));
    }

    /// With a body clock the rotation runs once per body tick and the
    /// counter is the step counter this tick's brains read: the body ticks
    /// run so far plus this one; a second run on the same body tick does
    /// nothing.
    #[test]
    fn the_counter_is_the_body_tick() {
        let mut app = App::new();
        app.init_resource::<PassTurn>()
            .insert_resource(BodyTicks(41))
            .add_systems(Update, rotate_pass_turn);
        app.update();
        assert_eq!(app.world().resource::<PassTurn>().turn.clock, 42);
        app.update();
        assert_eq!(app.world().resource::<PassTurn>().turn.clock, 42);
        app.world_mut().resource_mut::<BodyTicks>().0 = 45;
        app.update();
        assert_eq!(app.world().resource::<PassTurn>().turn.clock, 46);
    }

    /// A forgotten brain leaves the queue.
    #[test]
    fn a_forgotten_brain_leaves_the_queue() {
        let mut turn = PassTurn::default();
        tick(&mut turn, &[1, 2]);
        turn.forget(1);
        assert_eq!(turn.turn.longest, None);
        tick(&mut turn, &[2]);
        assert_eq!(turn.turn.last_plan(1), None);
    }
}

//! The brain's lifecycle hooks and the idle driver's object goals: what
//! happens to a brain when it is removed, destroyed or disabled, the script
//! object a role's init asks the factory table for, the scripted action hook
//! the idle states call first, the investigation of a disturbance, and the
//! room walk the idle driver plans.
//!
//! - **Script objects** ([`role_object`]): the factory table lists object
//!   kinds by id; the first id that matches makes the object with that id
//!   and the brain as its owner; an unknown id makes nothing.
//! - **Scripted action** ([`scripted_action`]): with no action running, a
//!   pending id is started when its table entry's start callback succeeds
//!   (the pending id clears either way); with one running, its tick callback
//!   is called and a false return clears it. Returns whether one runs.
//! - **Removal** ([`removed`]) stops the idle activity, then the base
//!   removal.
//! - **Destroy** ([`destroy`]): cancels a pending activation, leaves the
//!   party when the character follows, frees its path, and finalises.
//! - **Disable** ([`disabled`]): clears the AI's animation parameter and
//!   leaves the party when the character follows.
//! - **Investigation** ([`investigate`]): the idle states sample the
//!   pathfinder's disturbance at the body's 600 cm cell (a key of cell x
//!   and z). The same cell with a value within 0.02 of the last one does
//!   nothing (a value over 0.5 stamps the cell and returns); a changed cell
//!   or value stores it, and a value over 0.3 plans a walk to the noise (goal
//!   mode 4, forced) and enters the walk state. The pathfinder's disturbance
//!   is the constant 0 in 0.9.5, so the branch never fires.
//! - **Room walk** ([`plan_next_cell`]): the idle driver asks for the next
//!   room: it marks the current cell visited, and with an unvisited exit
//!   draws one at random, else marks the cell exhausted and draws one of the
//!   visited exits; the chosen cell becomes the walk goal at its exit point.
//!
//! Evidence: the original's removal, destroy and disable slot handlers, its
//! script object factory, its scripted action hook, its investigation and
//! its idle driver's room choice.

/// A script object kind the role init factory can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptKind(pub u32);

/// A script object the factory made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScriptObject {
    /// The object's kind id (`+8`).
    pub id: u32,
    /// The brain that owns it (`+0x10`).
    pub owner: u64,
}

/// The factory table's entry for a kind: its id and constructor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Factory {
    pub id: u32,
    pub kind: ScriptKind,
}

/// The script object a role's init asks for: the first table entry whose id
/// is `id`, owned by `owner`, or none for an unknown id.
pub fn role_object(table: &[Factory], id: u32, owner: u64) -> Option<ScriptObject> {
    table
        .iter()
        .find(|entry| entry.id == id)
        .map(|_| ScriptObject { id, owner })
}

/// One scripted action's callbacks: the start, which says whether it took,
/// and the tick, which says whether it keeps going.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub id: u32,
    pub start_ok: bool,
    pub tick_ok: bool,
}

/// The scripted action hook: `pending` is the id an idle state asked to
/// start, `running` the one under way. With no action running the pending
/// id is looked up and started when its start callback takes; the pending
/// id clears whether it took or not. With one running, its tick callback
/// is called and a false return clears it. Returns the running action and
/// whether one runs.
pub fn scripted_action(
    pending: Option<u32>,
    running: Option<u32>,
    actions: &[Action],
) -> (Option<u32>, bool) {
    if let Some(current) = running {
        let keeps = actions
            .iter()
            .find(|a| a.id == current)
            .is_some_and(|a| a.tick_ok);
        return (keeps.then_some(current), keeps);
    }
    if let Some(wanted) = pending
        && let Some(action) = actions.iter().find(|a| a.id == wanted)
        && action.start_ok
    {
        return (Some(wanted), true);
    }
    (None, false)
}

/// What removing a brain does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Removed {
    /// It had an idle activity, which is stopped.
    pub stopped_idle: bool,
}

/// Removing a brain stops its idle activity, then runs the base removal.
pub fn removed(idle_activity: bool) -> Removed {
    Removed {
        stopped_idle: idle_activity,
    }
}

/// What destroying a brain does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Destroyed {
    /// A pending activation was cancelled.
    pub cancelled: bool,
    /// The character left the party.
    pub left_party: bool,
    /// Its path was freed.
    pub freed_path: bool,
}

/// Destroys a brain: a body whose presence is entering (a pending
/// activation) is cancelled, a follower leaves the party, and its path is
/// freed.
pub fn destroy(entering: bool, following: bool, has_path: bool) -> Destroyed {
    Destroyed {
        cancelled: entering,
        left_party: following,
        freed_path: has_path,
    }
}

/// What disabling a brain does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Disabled {
    /// The AI animation parameter cleared.
    pub cleared_animation: bool,
    /// The character left the party.
    pub left_party: bool,
}

/// Disabling a brain clears its AI animation parameter when one is set,
/// and a follower leaves the party.
pub fn disabled(has_animation: bool, following: bool) -> Disabled {
    Disabled {
        cleared_animation: has_animation,
        left_party: following,
    }
}

/// The disturbance inside the value's own fluctuation: the same cell whose
/// value moved by less than this does not restart the walk.
pub const SAME_DISTURBANCE: f32 = 0.02;
/// The disturbance the walk ignores (the stamp-only branch).
pub const QUIET: f32 = 0.5;
/// The disturbance that starts a walk to the noise.
pub const LOUD: f32 = 0.3;

/// The investigation's memory: the cell it last sampled and the value it
/// kept.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Investigation {
    pub cell: Option<(i32, i32)>,
    pub kept: f32,
}

/// What an investigation tick asks the caller to do.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Investigated {
    /// The walk state was entered: the caller plans a forced walk to the
    /// noise (goal mode 4) and walks it.
    pub walking: bool,
    /// The cell was stamped at its goal (the quiet branch).
    pub stamped: bool,
}

/// One investigation tick at the body's `cell` with the pathfinder's
/// disturbance `value`: the same cell and a value within
/// [`SAME_DISTURBANCE`] of the kept one does nothing beyond stamping a
/// value over [`QUIET`]; a changed cell or value stores it, and a value over
/// [`LOUD`] enters the walk to the noise. Returns what to do.
pub fn investigate(
    state: &mut Investigation,
    cell: (i32, i32),
    value: f32,
    now_ms: u32,
    noise_at_ms: &mut u32,
) -> Investigated {
    let same_cell = state.cell == Some(cell);
    if same_cell && (value - state.kept).abs() < SAME_DISTURBANCE {
        if value <= QUIET {
            return Investigated {
                walking: false,
                stamped: false,
            };
        }
        // The original stamps the body's footprint at this point.
        return Investigated {
            walking: false,
            stamped: true,
        };
    }
    state.cell = Some(cell);
    if LOUD < value {
        state.kept = value;
        *noise_at_ms = now_ms;
        return Investigated {
            walking: true,
            stamped: false,
        };
    }
    Investigated {
        walking: false,
        stamped: false,
    }
}

/// A room exit the idle driver may walk to: the cell it leads to and its
/// point (cm).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExitCell {
    pub cell: i32,
    pub point_cm: bevy::math::Vec3,
}

/// The visited-cell words the idle driver keeps: bit 24 mark a visited
/// cell, bit 25 an exhausted one (a cell whose exits were all visited).
pub const VISITED: u32 = 0x0100_0000;
/// The cell is exhausted: all its exits were visited.
pub const EXHAUSTED: u32 = 0x0200_0000;

fn draw(roll: &mut u32, count: usize) -> usize {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    if count == 0 {
        return 0;
    }
    ((u64::from(*roll) * count as u64) >> 32) as usize
}

/// The idle driver's next room: the current cell's `exits` (in the room
/// grid's order, each with the cell it leads to), the cell it came from
/// (`previous`), the exits to avoid (a bit per exit index,
/// `excluded`), and the per-cell visited words. The current cell is marked
/// visited. Exits that lead to the previous cell or to an exhausted cell
/// are skipped; with none of them left (and the current cell not yet
/// exhausted) the walk goes back through the exit to the previous cell, and
/// the current cell is marked exhausted when every exit was skipped. With
/// exits left, one unvisited (and not avoided) is drawn at random, or, when
/// all are visited, one of the remaining exits. Returns the chosen exit and
/// the cell the walk leads to.
pub fn plan_next_cell(
    current: i32,
    previous: i32,
    exits: &[ExitCell],
    excluded: u32,
    words: &mut [u32],
    roll: &mut u32,
) -> Option<(ExitCell, i32)> {
    let mark = |words: &mut [u32], cell: i32, bit: u32| {
        if cell < 0 || words.len() <= cell as usize {
            return;
        }
        words[cell as usize] |= bit;
    };
    let at = |words: &[u32], cell: i32| -> u32 {
        if cell < 0 || words.len() <= cell as usize {
            0
        } else {
            words[cell as usize]
        }
    };
    mark(words, current, VISITED);
    let blocked =
        |e: &ExitCell| -> bool { e.cell == previous || at(words, e.cell) & EXHAUSTED != 0 };
    let fresh: Vec<usize> = exits
        .iter()
        .enumerate()
        .filter(|(_, e)| !blocked(e))
        .map(|(i, _)| i)
        .collect();
    let chosen =
        if fresh.iter().all(|&i| excluded & (1 << i) != 0) && at(words, current) & EXHAUSTED == 0 {
            if fresh.is_empty() {
                mark(words, current, EXHAUSTED);
            }
            exits.iter().position(|e| e.cell == previous)
        } else {
            let unvisited: Vec<usize> = fresh
                .iter()
                .copied()
                .filter(|&i| excluded & (1 << i) == 0 && at(words, exits[i].cell) & VISITED == 0)
                .collect();
            if unvisited.is_empty() {
                if fresh.is_empty() {
                    return None;
                }
                Some(fresh[draw(roll, fresh.len())])
            } else {
                Some(unvisited[draw(roll, unvisited.len())])
            }
        };
    let chosen = chosen?;
    let exit = exits[chosen];
    Some((exit, exit.cell))
}

/// The goal the idle driver puts an object to rest at: the body's own
/// position with no object, else the object's point 100 cm below its
/// origin; stop 50 cm, radius 100 cm, mode 7.
pub fn object_goal(
    body_cm: bevy::math::Vec3,
    object_cm: Option<bevy::math::Vec3>,
) -> (bevy::math::Vec3, i32, f32) {
    let point = object_cm.map_or(body_cm, |at| {
        bevy::math::Vec3::new(at.x, at.y - 100.0, at.z)
    });
    (point, 50, 100.0)
}

/// The steps of the idle driver's go-to-object state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum GoTo {
    /// Plan a forced walk to the next room's exit (fails back to state 1).
    #[default]
    Plan,
    /// Walk the route; on an arrival within 10 cm the wait starts, else
    /// plan again.
    Walk,
    /// Turn in place until the wait's timer passes.
    Wait,
}

/// The go-to-object state's clock and state.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GoToObject {
    pub step: GoTo,
    /// The wait's deadline (ms).
    pub until_ms: u32,
}

/// The draw for the wait after a go-to-object arrival: 1000 + a draw of
/// 4000 ms.
pub fn go_to_wait(roll: &mut u32, now_ms: u32) -> u32 {
    *roll = roll.wrapping_mul(0x0808_8405).wrapping_add(1);
    let step = ((u64::from(*roll) * 4000) >> 32) as u32;
    now_ms.wrapping_add(1000).wrapping_add(step)
}

/// One tick of the go-to-object state: state 1 asks for a forced plan; a
/// planned route becomes state 2; state 2's walk ends within 10 cm of the
/// goal, which draws the wait ([`go_to_wait`]) and becomes state 3, else
/// asks for a new plan; state 3 stands until the deadline. Returns whether
/// the caller should plan.
pub fn go_to_object(
    state: &mut GoToObject,
    plan: bool,
    stopped: bool,
    distance_cm: f32,
    now_ms: u32,
    roll: &mut u32,
) -> bool {
    match state.step {
        GoTo::Plan => {
            if !plan {
                return true;
            }
            state.step = GoTo::Walk;
            false
        }
        GoTo::Walk => {
            if !stopped {
                return false;
            }
            if distance_cm <= 10.0 {
                state.step = GoTo::Wait;
                state.until_ms = go_to_wait(roll, now_ms);
                false
            } else {
                state.step = GoTo::Plan;
                true
            }
        }
        GoTo::Wait => {
            if state.until_ms < now_ms {
                state.step = GoTo::Plan;
            }
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec3;

    #[test]
    fn the_factory_makes_the_first_matching_kind() {
        let table = [
            Factory {
                id: 4,
                kind: ScriptKind(9),
            },
            Factory {
                id: 7,
                kind: ScriptKind(2),
            },
        ];
        assert_eq!(
            role_object(&table, 7, 0xabc),
            Some(ScriptObject {
                id: 7,
                owner: 0xabc
            })
        );
        assert_eq!(role_object(&table, 8, 1), None);
    }

    #[test]
    fn the_scripted_action_starts_and_runs() {
        let actions = [
            Action {
                id: 3,
                start_ok: true,
                tick_ok: false,
            },
            Action {
                id: 5,
                start_ok: true,
                tick_ok: true,
            },
            Action {
                id: 6,
                start_ok: false,
                tick_ok: true,
            },
        ];
        // Nothing pending: nothing runs.
        assert_eq!(scripted_action(None, None, &actions), (None, false));
        // A start that takes runs until its tick fails.
        assert_eq!(scripted_action(Some(3), None, &actions), (Some(3), true));
        assert_eq!(scripted_action(None, Some(3), &actions), (None, false));
        // A start that fails leaves nothing.
        assert_eq!(scripted_action(Some(6), None, &actions), (None, false));
        // A running action keeps.
        assert_eq!(scripted_action(None, Some(5), &actions), (Some(5), true));
        // A pending id never replaces a running one.
        assert_eq!(scripted_action(Some(5), Some(5), &actions), (Some(5), true));
    }

    #[test]
    fn the_lifecycle_hooks_report_what_they_clear() {
        assert_eq!(removed(true), Removed { stopped_idle: true });
        assert_eq!(
            destroy(true, true, true),
            Destroyed {
                cancelled: true,
                left_party: true,
                freed_path: true
            }
        );
        assert_eq!(
            disabled(true, true),
            Disabled {
                cleared_animation: true,
                left_party: true
            }
        );
        assert_eq!(destroy(false, false, false), Destroyed::default());
    }

    /// A disturbance starts at 0.3 (strictly over) and keeps its cell's
    /// value within 0.02; a changed cell or value stores it again.
    #[test]
    fn the_investigation_walks_to_a_loud_noise() {
        let mut state = Investigation::default();
        let mut stamp = 0;
        assert!(!investigate(&mut state, (2, 3), 0.3, 10, &mut stamp).walking);
        let out = investigate(&mut state, (2, 3), 0.31, 20, &mut stamp);
        assert!(out.walking);
        assert_eq!(stamp, 20);
        assert_eq!(state.kept, 0.31);
        // The same cell and value: nothing.
        assert!(!investigate(&mut state, (2, 3), 0.315, 30, &mut stamp).walking);
        // A quiet value over 0.5 only stamps; at 0.5 exactly it does not.
        let mut state = Investigation {
            cell: Some((1, 1)),
            kept: 0.9,
        };
        assert!(investigate(&mut state, (1, 1), 0.9, 40, &mut stamp).stamped);
        // A changed value stores it and does not walk at 0.5 or 0.3.
        assert!(!investigate(&mut state, (1, 1), 0.5, 50, &mut stamp).stamped);
        assert!(!investigate(&mut state, (1, 1), 0.3, 60, &mut stamp).walking);
        // A kept 0.5 has no stamp; a kept 0.55 does.
        let mut state = Investigation {
            cell: Some((1, 1)),
            kept: 0.5,
        };
        assert!(!investigate(&mut state, (1, 1), 0.5, 70, &mut stamp).stamped);
        state.kept = 0.55;
        assert!(investigate(&mut state, (1, 1), 0.55, 80, &mut stamp).stamped);
    }

    /// The room walk marks the current cell visited, draws an unvisited
    /// exit, and falls back to any exit (marking the cell exhausted) once
    /// all are visited.
    #[test]
    fn the_room_walk_draws_an_unvisited_exit_and_goes_back_when_stuck() {
        let exits = [
            ExitCell {
                cell: 5,
                point_cm: Vec3::new(100.0, 0.0, 0.0),
            },
            ExitCell {
                cell: 6,
                point_cm: Vec3::new(200.0, 0.0, 0.0),
            },
            ExitCell {
                cell: 3,
                point_cm: Vec3::new(300.0, 0.0, 0.0),
            },
        ];
        let mut words = vec![0u32; 10];
        let mut roll = 0u32;
        // Roll 1: 1·2 >> 32 = 0, so the first unvisited exit is drawn.
        let (exit, cell) = plan_next_cell(4, 3, &exits, 0, &mut words, &mut roll).unwrap();
        assert_eq!((exit.cell, cell), (5, 5));
        assert_eq!(words[4] & VISITED, VISITED);
        // Mark cell 5 visited: only 6 is fresh.
        words[5] |= VISITED;
        let (exit, cell) = plan_next_cell(4, 3, &exits, 0, &mut words, &mut roll).unwrap();
        assert_eq!((exit.cell, cell), (6, 6));
        // Both visited and not exhausted: an exit is still drawn.
        words[6] |= VISITED;
        let (exit, _) = plan_next_cell(4, 3, &exits, 0, &mut words, &mut roll).unwrap();
        assert!(exit.cell == 5 || exit.cell == 6);
        // Both exhausted: the walk goes back through the exit to the
        // previous cell and the current cell is marked exhausted.
        words[5] |= EXHAUSTED;
        words[6] |= EXHAUSTED;
        let (exit, cell) = plan_next_cell(4, 3, &exits, 0, &mut words, &mut roll).unwrap();
        assert_eq!((exit.cell, cell), (3, 3));
        assert_eq!(words[4] & EXHAUSTED, EXHAUSTED);
        // No exits: nothing.
        assert!(plan_next_cell(4, 3, &[], 0, &mut words, &mut roll).is_none());
        // An avoided exit is not drawn while another is fresh.
        let mut words = vec![0u32; 10];
        let mut roll = 0u32;
        let (exit, _) = plan_next_cell(4, 3, &exits, 1, &mut words, &mut roll).unwrap();
        assert_eq!(exit.cell, 6);
    }

    #[test]
    fn the_object_goal_drops_one_hundred() {
        assert_eq!(
            object_goal(Vec3::ZERO, Some(Vec3::new(10.0, 250.0, 30.0))),
            (Vec3::new(10.0, 150.0, 30.0), 50, 100.0)
        );
        assert_eq!(
            object_goal(Vec3::new(1.0, 2.0, 3.0), None),
            (Vec3::new(1.0, 2.0, 3.0), 50, 100.0)
        );
    }

    /// A planned go-to-object walks, waits on an arrival within 10 cm, and
    /// plans again when the walk ended short.
    #[test]
    fn the_go_to_object_state_walks_and_waits() {
        let mut state = GoToObject::default();
        let mut roll = 0u32;
        assert!(go_to_object(&mut state, false, false, 0.0, 0, &mut roll));
        assert!(!go_to_object(&mut state, true, false, 0.0, 0, &mut roll));
        assert_eq!(state.step, GoTo::Walk);
        assert!(go_to_object(&mut state, false, true, 50.0, 10, &mut roll));
        assert_eq!(state.step, GoTo::Plan);
        assert!(go_to_object(&mut state, false, false, 0.0, 10, &mut roll));
        assert!(!go_to_object(&mut state, true, false, 0.0, 10, &mut roll));
        assert!(!go_to_object(&mut state, false, true, 10.0, 20, &mut roll));
        assert_eq!(state.step, GoTo::Wait);
        assert_eq!(state.until_ms, 1020);
        // The wait holds at its deadline and passes one millisecond after.
        assert!(!go_to_object(
            &mut state, false, false, 0.0, 1020, &mut roll
        ));
        assert_eq!(state.step, GoTo::Wait);
        assert!(!go_to_object(
            &mut state, false, false, 0.0, 1021, &mut roll
        ));
        assert_eq!(state.step, GoTo::Plan);
    }
}

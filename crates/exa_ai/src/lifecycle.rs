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
    /// The brain that owns it.
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
    excluded: &mut u32,
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
    let chosen = if fresh.iter().all(|&i| *excluded & (1 << i) != 0)
        && at(words, current) & EXHAUSTED == 0
    {
        if fresh.is_empty() {
            mark(words, current, EXHAUSTED);
        }
        exits.iter().position(|e| e.cell == previous)
    } else {
        let unvisited: Vec<usize> = fresh
            .iter()
            .copied()
            .filter(|&i| *excluded & (1 << i) == 0 && at(words, exits[i].cell) & VISITED == 0)
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

/// A room object the idle driver can walk to and use.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoomObject {
    /// Its point (cm).
    pub point_cm: bevy::math::Vec3,
    /// Its item record is searchable and not fixed (the original's
    /// `0x20` flag), its kind is not 5 and its value is not negative.
    pub usable: bool,
    /// Its reach in the item's unit, at most 0.7 to be used.
    pub reach: f32,
    /// Its use flag (`0x528` bit 8) is already set.
    pub used: bool,
}

/// Whether the body stands within 250 cm (flat, squared 62 500) of the
/// midpoint between the exit point `a` and the next one `b`.
pub fn near_exit(body_cm: bevy::math::Vec3, a: bevy::math::Vec3, b: bevy::math::Vec3) -> bool {
    let mid = (a + b) * 0.5;
    let dx = mid.x - body_cm.x;
    let dz = mid.z - body_cm.z;
    dz * dz + dx * dx <= 62_500.0
}

/// One tick of the use-object check: an object within 0.7 of its reach
/// that has not been used is used. Returns (a use fired, whether the
/// state is done): a used object, or one out of reach, leaves the state
/// done; an unused one in reach fires and is not done.
pub fn use_object(object: RoomObject) -> (bool, bool) {
    if object.used || 0.7 < object.reach {
        return (false, true);
    }
    (true, false)
}

/// The first usable object of the current room: searchable, not fixed, not
/// searched, its kind not 5, its value not negative.
pub fn find_object(objects: &[RoomObject]) -> Option<RoomObject> {
    objects.iter().copied().find(|o| o.usable)
}

/// The idle driver's state: its step (1–6), the wait's deadline, the exits
/// to avoid (a bit per exit index) and the room it set out for.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Explore {
    pub step: i32,
    pub until_ms: u32,
    pub excluded: u32,
    pub target_cell: i32,
}

/// What the idle driver asks the caller to do this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Drive {
    /// Turn the body in place (the out-of-combat turn).
    Turn,
    /// Plan a route with the room-walk callback (`forced` for a new room,
    /// not forced for an object in the same room).
    Plan { forced: bool },
    /// Walk the planned route.
    Walk,
    /// Test whether the body is near the exit.
    NearExit,
    /// Use the object at the exit.
    UseObject,
    /// Look for a usable object in the room.
    FindObject,
}

/// What the driver reads this tick.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DriveInput {
    pub now_ms: u32,
    /// The exit the walk set out for (its index in the room's list).
    pub exit_index: u32,
    /// The plan was started.
    pub planned: bool,
    /// The walk routine still walks.
    pub walking: bool,
    /// The body is within 250 cm of the exit's midpoint.
    pub near_exit: bool,
    /// The use check is done (the object was used, or is out of reach).
    pub use_done: bool,
    /// A usable object was found in the room.
    pub found: bool,
    /// The body reached the room it set out for.
    pub same_cell: bool,
    /// The planned path's flat length (cm).
    pub path_length_cm: f32,
}

impl Default for DriveInput {
    fn default() -> Self {
        Self {
            now_ms: 0,
            exit_index: 0,
            planned: false,
            walking: false,
            near_exit: false,
            use_done: false,
            found: false,
            same_cell: false,
            path_length_cm: 0.0,
        }
    }
}

/// One tick of the idle driver (the go-to-object state machine), after the
/// investigation, alert and attend checks: state 1 turns until its wait
/// passes, then asks for a forced room plan; state 2 plans, and on a plan
/// walks (state 3) with a wait of the path's length × 20 ms plus 2 s;
/// state 3 walks while its wait lasts, then tests the exit: near it the
/// body moves on to the use state (4), otherwise it plans again (2) and
/// remembers the exit to avoid. State 4 turns and uses the object, then
/// plans the way home (5); a use that does not come within ten seconds
/// plans again (2). State 5 plans a forced home route, and state 6, at the
/// room, looks for an object before it walks and, when the walk stops,
/// plans again with a 1–4 s wait. Returns the action for the caller.
pub fn drive(state: &mut Explore, input: &DriveInput, roll: &mut u32) -> Drive {
    match state.step {
        1 => {
            if state.until_ms < input.now_ms {
                state.step = 2;
            }
            Drive::Turn
        }
        2 => {
            if input.planned {
                state.step = 3;
                state.until_ms = input
                    .now_ms
                    .wrapping_add((input.path_length_cm * 20.0).round_ties_even() as u32)
                    .wrapping_add(2000);
            }
            Drive::Plan { forced: true }
        }
        3 => {
            if state.until_ms <= input.now_ms && input.walking {
                Drive::Walk
            } else if input.near_exit {
                state.step = 4;
                state.until_ms = input.now_ms;
                Drive::NearExit
            } else {
                state.step = 2;
                state.excluded |= 1 << (input.exit_index & 31);
                Drive::NearExit
            }
        }
        4 => {
            if input.use_done {
                state.step = 5;
            } else if state.until_ms.wrapping_add(10_000) < input.now_ms {
                state.excluded |= 1 << (input.exit_index & 31);
                state.target_cell = -1;
                state.step = 2;
            }
            Drive::UseObject
        }
        5 => {
            if input.planned {
                state.step = 6;
            }
            Drive::Plan { forced: true }
        }
        6 => {
            if input.same_cell {
                if input.found {
                    return Drive::FindObject;
                }
                if input.planned {
                    state.step = 3;
                    state.until_ms = input
                        .now_ms
                        .wrapping_add((input.path_length_cm * 20.0).round_ties_even() as u32)
                        .wrapping_add(2000);
                    return Drive::Plan { forced: false };
                }
            }
            if !input.walking {
                state.step = 2;
                state.until_ms = go_to_wait(roll, input.now_ms);
            }
            Drive::Walk
        }
        _ => {
            state.step = 1;
            Drive::Turn
        }
    }
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
        let (exit, cell) = plan_next_cell(4, 3, &exits, &mut 0, &mut words, &mut roll).unwrap();
        assert_eq!((exit.cell, cell), (5, 5));
        assert_eq!(words[4] & VISITED, VISITED);
        // Mark cell 5 visited: only 6 is fresh.
        words[5] |= VISITED;
        let (exit, cell) = plan_next_cell(4, 3, &exits, &mut 0, &mut words, &mut roll).unwrap();
        assert_eq!((exit.cell, cell), (6, 6));
        // Both visited and not exhausted: an exit is still drawn.
        words[6] |= VISITED;
        let (exit, _) = plan_next_cell(4, 3, &exits, &mut 0, &mut words, &mut roll).unwrap();
        assert!(exit.cell == 5 || exit.cell == 6);
        // Both exhausted: the walk goes back through the exit to the
        // previous cell and the current cell is marked exhausted.
        words[5] |= EXHAUSTED;
        words[6] |= EXHAUSTED;
        let (exit, cell) = plan_next_cell(4, 3, &exits, &mut 0, &mut words, &mut roll).unwrap();
        assert_eq!((exit.cell, cell), (3, 3));
        assert_eq!(words[4] & EXHAUSTED, EXHAUSTED);
        // No exits: nothing.
        assert!(plan_next_cell(4, 3, &[], &mut 0, &mut words, &mut roll).is_none());
        // An avoided exit is not drawn while another is fresh.
        let mut words = vec![0u32; 10];
        let mut roll = 0u32;
        let (exit, _) = plan_next_cell(4, 3, &exits, &mut 1, &mut words, &mut roll).unwrap();
        assert_eq!(exit.cell, 6);
    }

    /// The exit midpoint is 250 cm out at most; the use fires within 0.7
    /// once; the first usable object is found.
    #[test]
    fn the_exit_use_and_object_checks() {
        assert!(near_exit(
            Vec3::new(250.0, 0.0, 0.0),
            Vec3::new(500.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0)
        ));
        assert!(!near_exit(
            Vec3::new(501.0, 0.0, 0.0),
            Vec3::new(500.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 0.0)
        ));
        let object = RoomObject {
            point_cm: Vec3::ZERO,
            usable: true,
            reach: 0.7,
            used: false,
        };
        assert_eq!(use_object(object), (true, false));
        assert_eq!(
            use_object(RoomObject {
                used: true,
                ..object
            }),
            (false, true)
        );
        assert_eq!(
            use_object(RoomObject {
                reach: 0.71,
                ..object
            }),
            (false, true)
        );
        assert_eq!(find_object(&[]), None);
        assert_eq!(
            find_object(&[
                RoomObject {
                    usable: false,
                    ..object
                },
                object
            ]),
            Some(object)
        );
    }

    /// The driver turns, plans a room, walks and tests the exit, uses the
    /// object and plans home; a short walk plans again.
    #[test]
    fn the_idle_driver_walks_a_room_and_uses_its_object() {
        let mut state = Explore {
            step: 1,
            until_ms: 100,
            ..Explore::default()
        };
        let mut roll = 0u32;
        let at = |now, planned, walking, near, used, found, same, length| DriveInput {
            now_ms: now,
            exit_index: 2,
            planned,
            walking,
            near_exit: near,
            use_done: used,
            found,
            same_cell: same,
            path_length_cm: length,
        };
        assert_eq!(
            drive(
                &mut state,
                &at(100, false, false, false, false, false, false, 0.0),
                &mut roll
            ),
            Drive::Turn
        );
        assert_eq!(state.step, 1);
        assert_eq!(
            drive(
                &mut state,
                &at(101, false, false, false, false, false, false, 0.0),
                &mut roll
            ),
            Drive::Turn
        );
        assert_eq!(state.step, 2);
        assert_eq!(
            drive(
                &mut state,
                &at(110, false, false, false, false, false, false, 0.0),
                &mut roll
            ),
            Drive::Plan { forced: true }
        );
        assert_eq!(
            drive(
                &mut state,
                &at(120, true, false, false, false, false, false, 300.0),
                &mut roll
            ),
            Drive::Plan { forced: true }
        );
        assert_eq!(state.step, 3);
        // 300 cm × 20 ms + 2000 = 8000 ms after 120.
        assert_eq!(state.until_ms, 8120);
        assert_eq!(
            drive(
                &mut state,
                &at(8121, false, true, false, false, false, false, 0.0),
                &mut roll
            ),
            Drive::Walk
        );
        // Out of the exit's reach: plan again and avoid it.
        assert_eq!(
            drive(
                &mut state,
                &at(9000, false, false, false, false, false, false, 0.0),
                &mut roll
            ),
            Drive::NearExit
        );
        assert_eq!(state.step, 2);
        assert_eq!(state.excluded, 4);
        // Plan, arrive, use, and plan home.
        state.step = 3;
        state.until_ms = 0;
        assert_eq!(
            drive(
                &mut state,
                &at(20_000, false, false, true, false, false, false, 0.0),
                &mut roll
            ),
            Drive::NearExit
        );
        assert_eq!(state.step, 4);
        assert_eq!(
            drive(
                &mut state,
                &at(20_001, false, false, false, true, false, false, 0.0),
                &mut roll
            ),
            Drive::UseObject
        );
        assert_eq!(state.step, 5);
        assert_eq!(
            drive(
                &mut state,
                &at(20_100, true, false, false, false, false, false, 0.0),
                &mut roll
            ),
            Drive::Plan { forced: true }
        );
        assert_eq!(state.step, 6);
        // At the room with an object: find it.
        assert_eq!(
            drive(
                &mut state,
                &at(21_000, false, false, false, false, true, true, 0.0),
                &mut roll
            ),
            Drive::FindObject
        );
        // A walk that stops plans again with a 1–4 s wait.
        state.step = 6;
        assert_eq!(
            drive(
                &mut state,
                &at(22_000, false, false, false, false, false, true, 0.0),
                &mut roll
            ),
            Drive::Walk
        );
        assert_eq!(state.step, 2);
        assert_eq!(state.until_ms, 23_000);
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

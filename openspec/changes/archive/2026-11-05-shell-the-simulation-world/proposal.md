## Why

Stage 2 of moving the fixed step onto its own thread (`plan-the-simulation-thread`). Before any simulation data can move, the simulation needs a world of its own, a thread to run it on, and the hand-offs the original uses: work handed to the simulation thread runs at the top of its next pass, in order; the renderer reads what the last step published. This change builds that shell and runs it in lockstep with the fixed step, so the game behaves exactly as before.

## What Changes

- `exa_physics::sim_app`: `SimShell`, a Bevy `SubApp` with its own world and `SimStep` schedule, on a thread named `exa-sim` or inline (`--sim-inline`).
- `SimInbox` (frame to simulation): commands run on the simulation's world at the start of the next step, in post order; `spawn_linked` / `despawn_linked` create and remove an object named by a `SimId` in both worlds.
- `SimOutbox` (simulation to frame): events run on the frame's world after the step that sent them, in send order.
- `SnapshotBuffer` (latest-wins) and `Snapshots` (the frame's last two) carry each step's `SimSnapshot` (tick, game time, linked objects).
- `SimIds` and `SimLinks`: an object's id and each world's map from ids to entities.
- Lockstep: a `SimLockstep` schedule before `FixedFirst` in every pass of `FixedMain` runs one simulation step and waits; `LockstepCheck` counts steps against the snapshots' ticks and is logged every 1000 steps.
- The simulation's world holds only its clock: no game data moves yet.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost.

Evidence: the original's physics thread body read with REA for `plan-the-simulation-thread` (the deferred-call ring drained at the top of each pass; the published last tick and time). Parity: a body test runs the same shoved, muscle-driven body with no shell, an inline shell and a threaded shell and compares every pose bit for bit; joint traces through the grid.

## Out of scope

- Moving any fixed-step system or data into the simulation's world (stage 3 onwards).
- Running the simulation free of the frame (stage 6); the thread's priority.
- The character controller's internals (`cl/charmove`).

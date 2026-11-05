## Why

Exarust runs the fixed 83 Hz step inside each frame on the main thread. In exanima01 a tick costs about 11.5 ms, so a frame settles at about 10 ms of frame work plus four ticks (about 18 FPS) and game time slows to about 71 Hz. The original runs every scene's simulation on its own time-critical thread, one 12 ms step at a time, and rendering is not on that thread. The owner approved moving Exarust's fixed step onto its own thread. Bevy cannot run two schedules on one world at once, so the move needs the simulation in a world of its own; that is a staged refactor, and this change is its first stage: the original's driver, a map of the data the move has to hand across, and the design.

## What Changes

- `exa_physics::step_driver::StepDriver`: the original's step clock (32-bit wrapping milliseconds, a step due once the time since the base reaches the next step's end, the backlog dropped by moving the base when a step is more than four steps late, a wait one millisecond short of the next step and at most 100 ms). It is not yet wired into the frame loop: the simulation thread of a later stage drives it.
- `exarust … --sim-boundary <file>`: once the level has loaded and the fixed step has run 200 more ticks, writes which component and resource types the fixed step's schedules share with the frame's schedules, by direction (simulation to frame, frame to simulation, written by both, read by both), with each side's readers and writers, and the whole-world systems of each side.
- The design and the stages that follow (context artifact `2026-10-30-simthread`).
- No rule of the simulation changes, and the game runs as before.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost.

Evidence: the original's physics thread body, read with REA (pseudo-code checked against the assembly): the deferred-call ring drained at the top of each pass, the due test and backlog drop, the wait, the pause's sleep, the published last tick and time. Bevy 0.20's sources for the schedule and access APIs.

## Out of scope

- The simulation sub-app, its thread, the inbox and the snapshots (stages 2 to 6 of the design).
- Any change to the frame-coupled fixed loop's pacing.
- The character controller's internals (`cl/charmove`).

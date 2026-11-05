## Why

Stage 4 of the simulation thread, coupled writers (`hits-in-the-simulation`), after the grid runs were made repeatable (`repeat-scripted-grid-runs`). Two more frame writers of simulation data: the golem alarm's zone check, and the hotkeys' grab release on entering combat.

## What Changes

- `check_alarm_zones` runs in `SimFrameStart` (`Early`), after the rise and its restore: the original's AI manager frame checks the trigger zones after its rise, as a hook its simulation thread's step calls. The alarm's volumes, state and check points are bridged.
- Entering combat by hotkey posts the release of the hand's grab (`FixedCommands`); the hand's own state changes in the frame at once.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Evidence: the manager frame (REA, `rise-in-the-simulation`: a hook the simulation thread's step calls) runs the rise and then the zones (`exa_ai::manager`'s notes on the original). No rule changes.

## Out of scope

- The hand's press, hold and reach with the GUI's lifts and drops (they need the frame's input captured once a frame with its edges kept across frames that run no tick, and their whole chain moved with the object clicks that read it in the same frame), the flow's steps with the alarm return, and the use cursor's end: next increments.

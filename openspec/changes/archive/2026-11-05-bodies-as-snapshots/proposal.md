## Why

Stage 4 of the simulation thread, after `alarm-zones-and-hotkeys-in-the-simulation`. The bodies were still copied both ways because the frame wrote them: the hand, the GUI drop's lift, the use cursor's unlock, the flow's steps and the alarm return. REA settled where the hand belongs: the original runs it on its main thread per input event and writes the bodies directly, so the hand stays in the frame and its writes are handed over instead of moving the chain.

## What Changes

- `FixedCommands::write` (with `write_pair` and `write_many`): a frame write to simulation data applies to the frame's copy at once, without a change mark, and is queued for the simulation's copy at the start of the next step, before the frame's other changes. In one world it is a plain write.
- The hand's press, hold and reach, the GUI drop's lift, the use cursor's unlock, the flow's knock-out, revive, wake and pose steps, and the alarm return's resets, tones and sleep scales write that way.
- `bridge_snapshot`: a component copied out when the simulation changes it and in only when the frame inserts it; a frame change not posted is not sent and is warned of once. `Ragdoll`, `DoorBody`, `LeverBody` and `PlateBody` become snapshots. Cloths stay copied in only.
- The hand logs its state, presses, lifts and drags at debug level (`RUST_LOG=exa_game::hand=debug`).

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the simulation thread.

Evidence: the left press's link creation appends to the body's link array with no lock and no deferred call (REA, `context/re/sim-thread-input.md`); the simulation thread reads the array at its next step. Exarust lands the write at the next step: a step's difference at most. No rule changes.

## Out of scope

- Narrower scans of the copied types; the simulation running free of the lockstep (stage 6).

## Why

Stage 4 of the simulation thread, second group (`snapshot-the-bodies-and-cloths`). Frame systems still write simulation data directly (21 writers of bodies alone); with the simulation free of the frame (stage 6) each such write would race it. The writes have to reach the fixed step as ordered work instead.

## What Changes

- `FixedCommands` (`exa_physics::sim_app`): a frame system posts a write (a closure, or a cached one-shot system with its input); `FixedCommandsStep`, after the lockstep and before `FixedFirst`, runs the queue on the world that runs the fixed step: the frame's in one world, the simulation's behind the bridge (the queue is lent for the step). Both modes apply a posted write at the same point.
- The lockstep schedule leads the fixed order, so no fixed-step schedule runs in the frame before the bridge moves it.
- Writers posted: `--fell-player`, `--undead-opponent`, `--mechanism-poke`, the door toggle, going limp (the body's muscles; the frame's own records change at once), finished limb trees, the hand's door and lever releases.
- `--sim-digest` also digests the bridged frame's copy (`<file>.frame.tsv`).

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the fixed tick's cost and the simulation thread.

Evidence: no engine rule is added. Parity with `--sim-digest` and duel joint traces against one world; the bridge's semantics test now posts a write and finds it alike in both modes.

## Out of scope

- The coupled writers, which need their decide/apply split designed with their same-frame readers and structural changes: the hand's press, hold and reach and the GUI's lifts and drops, hotkeys, use and flow steps, the golem alarm, the rise (with `restore_revived`), the hit chain (`react_to_hits`, `open_bites`).
- Snapshots in place of copies out; narrower scans; freeing the thread (stage 6).

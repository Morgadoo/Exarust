## Why

The arena duel stopped repeating from run to run after round 69: two one-world runs differed by 1.1–1.6 degrees mean and 16–25 degrees max over their first 100 traced ticks, against 0.010 / 0.41 before, so no lane could tell a regression from noise.

## What Changes

- Under `--fixed-frames` an offscreen grid quadrant drops the host's forwarded cursor, motion, keys, buttons and wheel (it still exits when the host closes): only the scene's scripted input reaches the game.

Phase 3 (bodies: skeleton, physics, control) in docs/PLAN.md: the parity checks of the simulation thread.

Evidence: the step digest of two one-world runs (`--sim-digest`, `EXA_DIGEST_DUMP`) parts first in the idle watch's pointer (`IVec2(0, 3)` against `IVec2(0, 0)`), then the cursor's ground point, the player's steering and locomotion, then the bodies: the grid host forwards its real mouse to the active quadrant, and the player's aim follows the cursor's ground point. Round 69's footwork (pelvis lead, stance clamp) turns the body to that aim, so a resting host mouse now moves bodies. No engine rule changes: the original has no forwarded input; this is the test harness.

## Out of scope

- The load-time spawn of held items (a weapon attached a few ticks apart between runs), which still separates runs after their first hundred ticks.

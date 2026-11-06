## Why

Stage 6 step 3 (docs/PLAN.md Phase 3, the simulation thread). The original's simulation always runs on its own
thread. Free mode now holds lockstep parity (scripted runs stay in lockstep), takes the debug step and speed keys,
and draws as evenly as lockstep where the step fits its 12 ms (arena walk: drawn speed off its median 10.2% free vs
13.6% lockstep; camera-relative shake p95 1.79 vs 1.39 cm). Where the step is late, the original holds its render
fraction at 2 too (REA: both render-fraction variants clamp there), so the drawn pose holds the same way.

## What Changes

- The game runs the simulation free of the frame by default; `--sim-lockstep` (and `--fixed-frames`,
  `--sim-inline`, `--no-sim-bridge`) step it in lockstep; `--sim-free` is kept for old command lines.
- The bodies' impact sounds walk the pair touches once a step, not once a body.
- `--draw-trace` writes the body ticks the frame has seen.

Rule change: none in lockstep (traces identical to main); the default run is not tick-identical to lockstep by
design.

## Out of scope

- The solver's own cost (pairs, part frames, cloth): another lane's physics.

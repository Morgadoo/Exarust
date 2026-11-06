## Why

Stage 6 toward step 3 (docs/PLAN.md Phase 3, the simulation thread). Free mode (round 88) is opt-in; before it can be
the default, scripted runs must stay repeatable, Exarust's debug step and speed keys must reach the free
simulation, and the step must get cheaper (exanima01's step took 25-33 ms in a busy grid, far over the 12 ms
budget, so game time ran at about 30 steps a second).

Evidence: REA of Exanima.exe 0.9.5: the simulation loop reads the real millisecond clock unscaled (no speed in the
loop); the scaled-clock render variant is called only from the renderer's multi-sample pass (sub-frame offsets
across the exposure), with a scene time scale; the render fraction is held at 2 when the frame clock is more than a
step past the published time (both variants), so a late step holds the drawn pose in the original too. Profiles of
the simulation thread (perf, flat: call graphs are not available without privileges).

## What Changes

- `--fixed-frames` keeps lockstep even with `--sim-free`.
- Free mode: the pause key holds the simulation, the step key takes single steps while held (each moves its clock
  on by a step), the speed keys scale its clock (`SimPause::step`, `set_speed`); the step's stamp carries the clock
  and its rate so the frame's fraction follows.
- The moved schedules run on the single-threaded executor on the simulation's own thread (the shared task pool made
  a step wait for the frame's and the renderer's work); `EXA_SIM_EXECUTOR=multi` restores the pool.
- The body grid keeps the cells of bodies that did not change since its last rebuild.
- `EXA_SIM_TIMING=1`: each moved schedule's time a step, logged every 1000 steps.

Rule change: none (lockstep traces of every body identical to main in the arena duel, exanima01 and exanima06).

## Out of scope

- Free mode as the default (its drawn-speed evenness is still measured against lockstep's).
- The solver's own work (pairs, part frames, cloth): another lane's physics.

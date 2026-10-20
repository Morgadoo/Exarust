# Proposal

## Why

A review of the salvaged locomotion and clip-clock work against the
original's sampler and locomotion routine found two rules read through the
wrong gate. The living sampler weights the Run overlay on the Walk by the
run blend itself whenever the sprint latch is kept, and by 0.7 of it
otherwise: it reads the latch bit alone. Exarust also required the stance
level above 0.9 and a speed above 3.5 cm a tick there, the gates that only
pick the sprint gait row. And the stop distance took the sprint row with
the latch and the speed but without the stance level, which the original's
row choice also tests. Phase 3 (motion, milestone M16, the run overlay) and
Phase 2/6's locomotion (lane D change M1).

## What Changes

- The overlay weight: the run blend with the sprint latch kept, whatever
  the stance level and the speed; else 0.7 × the run blend, in double
  precision as the original multiplies it.
- The stop distance's sprint row: the latch, a stance level above 0.9
  (double precision) and over 3.5 cm a tick, as the clip clock's sprint
  row already is.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `motion-playback`: the Run overlay's weight.
- `player-movement`: the locomotion request's stop distance row.

## Impact

- `exa_game::limp` (`Pace::latch`, `overlay_share`), `exa_game::stages`
  (sampler and movement), `exa_physics::body::locomotion::locomote`.
- `docs/PLAYER.md`, `docs/formats/RSQ.md`, `crates/exa_game/README.md`.
- Evidence: REA on the living sampler's overlay step (a test of the
  request's latch bit alone before the slerp, the 0.7 a double) and the
  locomotion routine's gait-row choice (latch, level above 0.9 compared in
  double precision, speed above 3.5 cm a tick). No format field changes,
  so the corpus step is a skip.
- Out of scope: the sprint row's other columns in the capsule, the run
  blend's bleed by the alignment and vertical travel, a gamepad sprint.

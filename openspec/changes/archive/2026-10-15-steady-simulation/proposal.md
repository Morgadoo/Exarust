## Why

Play mode ran at about 6 FPS with exanima01's 39 characters. A profile
showed the fixed schedule in a spiral: slow frames owed more fixed steps,
each step swept every character with the character controller (about
5.7 ms), and the next frame was slower still. The original ticks its
simulation at 83 Hz and drops its backlog once it is more than 4 steps
behind. Exarust stepped at 120 Hz and caught up to 250 ms. Characters were
also drawn at their last fixed step, so at 37 FPS a frame covered 2 or 3
steps, and the player's motion stuttered visibly.

## What Changes

- The fixed step runs at the original's 83 Hz. A slow frame catches up at
  most 4 steps (48 ms of game time), as the original drops its backlog.
- A grounded character not trying to move is not swept.
- Characters are drawn between their last two fixed steps. A transform moved
  elsewhere (a teleport) replaces the simulated position.
- A limp body still for 2 s (every particle slower than 1 cm/s) sleeps
  until shoved. Its contacts are with fixed geometry only, so it would not
  have moved.

## Capabilities

### New Capabilities
(none)

### Modified Capabilities
- `physical-body`: fixed rate, catch-up limit, drawing between steps,
  sleeping limp bodies.

## Impact

- `exa_physics` (`lib.rs`, `body.rs`), `docs/ARCHITECTURE.md`,
  `docs/PLAN.md`, `docs/PERFORMANCE.md`.

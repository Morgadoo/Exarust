# Proposal

## Why

After some walking, play mode fell to 10–15 FPS. The profiler showed the
fixed step at 4 ticks a frame, and a trace named the character controller:
when the player pushed into furniture, each tick's capsule sweep against
the crates' and tables' meshes cost several milliseconds (up to 22). The
player's look-ahead cast a ray at hip height against the level, which
misses crates below the hip and passes under table tops, so the player
kept pushing. The original's look-ahead walks the pathfinder grid, where
furniture blocks cells, and stops the character in front of it.

## What Changes

- The player's look-ahead walks the navigation grid from the standing
  point toward the look-ahead goal (75 cm + 6·v², 70 cm back); a blocked
  goal within 100 cm stops the character. The hip ray stays only for
  levels without a grid.
- The frame profiler counts the character controllers swept in a frame.

## Capabilities

### Modified Capabilities

- `player-movement`: the look-ahead uses the grid.

## Impact

- `exa_game::player` (`look_blocked`), `exarust::profiler`.
- `docs/PLAYER.md`, `docs/PERFORMANCE.md`.
- Evidence: `context/re/master-movement.md` §4.2 (look-ahead goal, grid line
  walk clamped at the first blocking cell, blocked within 100 cm) and §8.
- Out of scope: the brake toward the clamped goal (the character stops at
  once, as Exarust's look-ahead already did), NPCs' obstacle check.

## Phase

Phase 3, player movement; Phase 8, performance.

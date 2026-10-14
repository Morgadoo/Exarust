# Proposal

## Why

Players started in a stand-in cell near the grid centre. The original fixes
the start of a new game, the arena hub and arena matches, where fighters face
each other across a ring.

## What Changes

- `exa_world::start`: the new-game start in `exanima01`, the arena hub and
  practice arena starts, arena centres and the fighters' ring.
- Play mode starts the player there with the original's facing; the
  opponent takes the second ring slot.
- Docs: starts, the character (`CAC00001`) and placed-object (`DBCE`) layouts
  for the next changes, and `locales.rdb` as the level list.

Out of scope: arrival markers of level transitions, spawning the placed
characters and objects.

## Capabilities

### New Capabilities

### Modified Capabilities
- `player-movement`: start positions.

## Impact

`crates/exa_world`, `crates/exarust`, docs.

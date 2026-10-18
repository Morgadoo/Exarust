# Proposal

## Why

Phase 6 still sends a duellist in a straight line at its own speed, and the navigation grid has no planner. `docs/PLAN.md` §7 asks for NPC control on the player's inputs, with look-ahead and pathfinding.

## What Changes

- A search on the grid returns a path of cell centres that goes around blocked cells.
- The look-ahead is 75 + 6·v² cm forward, 45 + 6·v² in combat, and 70 cm back. The player's wall probe uses it.
- An NPC order is a heading and a desired velocity, the same fields the player writes, at the player's walk or jog speed. The duellist approaches by that order along the straight path to its target.

## Capabilities

### New Capabilities

- `npc-steering`: the order that follows a path.

### Modified Capabilities

- `navigation-grid`: the search and the look-ahead.

## Impact

- `exa_world::nav::find_path` and `look_ahead`, `exa_ai::follow`, the duellist and the player's wall probe.
- `docs/PLAN.md`, `docs/NAVIGATION.md`.
- Evidence: the movement notes' pathfinder and NPC driver, already recorded from the original. No format field changes, so the corpus step is a skip.
- Out of scope: line-walk clamping, footprints, perception, and the original attack choice.

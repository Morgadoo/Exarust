# Proposal

## Why

Phase 2 still treats every door as open, draws the next room past the doorway, and never changes level. `docs/PLAN.md` §7 lists portal clipping, doors' open state and level transitions next.

## What Changes

- A shut door does not load the room behind it. An open door still does.
- A point in the next room is shown only when the view crosses the doorway. A shut door shows nothing through it. Openings are 82 × 245 cm, 115 × 290 cm, or 200 × 340 cm.
- A passage loads the level its id names in the level list. An unknown id keeps the current level.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `room-visibility`: door state, portal clip and level passages.

## Impact

- `exa_world::rooms`: `set_open`, `seen_through`, `portal_size`, `enter_level`.
- `docs/PLAN.md`, `docs/RENDERING.md`.
- Evidence: the grid notes' portal sizes and the level-transition marker, already recorded from the original. No format field changes, so the corpus step is a skip.
- Out of scope: drawing the lid into light cube faces, and light passing through portals.

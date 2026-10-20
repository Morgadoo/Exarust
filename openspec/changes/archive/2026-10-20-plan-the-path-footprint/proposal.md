# Proposal

## Why

The pathfinder keeps a growing list of footprint rectangles: a new footprint joins the first rectangle whose padded union is smaller than the two padded areas, and otherwise it is appended, with the capacity grown by eight. Exarust models a separate mover queue that keeps sixteen and drops the oldest, not this list.

## What Changes

- Merge into the first rectangle when the padded union is smaller than the two padded areas.
- Otherwise append, growing the capacity by eight when the length has reached it.
- A busy planner leaves the list alone.
- The padded area is `(x1 − x0 + 6) × (y1 − y0 + 6)`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the footprint rectangle planner.

## Impact

- `exa_world::nav::mover`, from `plan_footprint`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 929-byte method `0x1000c2fc0`: the guard at `0x1e0`, the merge loop over the list at `0xd0` with count `0xc8` and capacity `0xcc`, the `+6` padded areas, the `+8` growth and the append. No new corpus files.
- Out of scope: the transform and the ring slot the bounds come from, already modelled by `path_extents`.

## Phase

Phase 4, navigation. The footprint rectangle planner.

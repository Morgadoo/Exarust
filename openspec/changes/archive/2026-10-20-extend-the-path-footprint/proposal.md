# Proposal

## Why

The pathfinder's footprint extent routine advances a 16-slot ring, derives a rotated radius per axis and stores the clamped low and high grid bounds. Exarust does not model it.

## What Changes

- The slot advances by one modulo 16.
- Axis 0 takes local values 0, 3 and 6; axis 1 takes 2, 5 and 8, each absolute value weighted by the matching extent and summed.
- The low bound is half the axis size plus the local value less the radius, over the inverse cell, minus 0.5, rounded, at least 3.
- The high bound adds the radius, adds 0.5, rounds and caps at the axis size less 4.

## Capabilities

### New Capabilities

### Modified Capabilities

- `navigation-grid`: the footprint extents.

## Impact

- `exa_world::nav`, from `path_extents`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 413-byte extent method and its two columns and clamps. No new corpus files.
- Out of scope: the transform that builds the local matrix.

## Phase

Phase 4, navigation. The footprint extents.

# Proposal

## Why

The morph controller scans its transformed points and keeps the index and distance of the nearest to a given point, starting from the maximum float. Exarust has the setup and the single-slot update but not the scan.

## What Changes

- Finding scans the points and keeps the smallest distance to the given point.
- The index and distance are returned; NaN distances do not replace.
- An empty list returns −1 and the maximum float.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the nearest-index scan.

## Impact

- `exa_assets::morph_controller`, from `nearest_morph_index`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 1% to 2%.
- Evidence: the 192-byte nearest search: the point count and list, the transform by the owner's frame, the distance helper, the start `0xffffffff` with the maximum float, and the index and distance stores. No new corpus files.
- Out of scope: the transform and distance bodies.

## Phase

Phase 5, characters. The nearest morph index.

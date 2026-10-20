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
- Evidence: the 192-byte `0x1000db790`: the count `0x68`, the points `0x70`, the transform `0x1000190b0` with `0x38 + 0x64`, the distance `0x100019ae0`, the start `0xffffffff` with the maximum float `0x1002ab6d0`, and the index and distance stores. No new corpus files.
- Out of scope: the transform and distance bodies.

## Phase

Phase 5, characters. The nearest morph index.

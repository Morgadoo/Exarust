# Proposal

## Why

A morph controller stores its target count and copies two arrays, and keeps the nearest morph point by distance. Exarust has the physique morphs but not the controller's setup or its nearest update.

## What Changes

- Setup stores the count, allocates both arrays and copies the target's points.
- The nearest update replaces the best point, extra, distance and marker only when the new distance is strictly below the stored one.
- A NaN distance does not replace.

## Capabilities

### New Capabilities

- `morph-controller`: the controller's setup and nearest point.

### Modified Capabilities

## Impact

- `exa_assets::morph_controller`, from `setup_morph` and `nearest_morph`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 0% to 1%.
- Evidence: the 100-byte nearest setup: the target, the point count from the owner's output, the two arrays and the copy. The 66-byte nearest callback: the distance call, the comparison with the stored distance, and the four stores. No new corpus files.
- Out of scope: the distance, allocation and copy bodies.

## Phase

Phase 5, characters. The morph controller's setup and nearest point.

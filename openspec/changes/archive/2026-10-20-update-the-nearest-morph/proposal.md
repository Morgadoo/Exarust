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
- Evidence: the 100-byte `0x1000db620`: the target at `0x38`, the count at `0x48` from `0x280 + 0xc`, the two arrays at `0x40` and `0x60`, and the copy `0x1000022c0`. The 66-byte `0x1000de0b0`: the distance call `0x100019ae0`, the comparison with `0x14`, and the stores at `0x8`, `0x10`, `0x14` and `0x18`. No new corpus files.
- Out of scope: the distance, allocation and copy bodies.

## Phase

Phase 5, characters. The morph controller's setup and nearest point.

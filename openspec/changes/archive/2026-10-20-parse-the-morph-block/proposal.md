# Proposal

## Why

The morph controller is built from a tagged block: an unknown word, a first count and its points, a run of the owner's points, a second count and, per entry, the first count's points and the owner's points. Exarust has the controller's behaviours but not the block parser.

## What Changes

- The tag `0xEDC0C589` is required.
- The unknown word, first count, first points, skipped owner points, second count and each entry's two point runs are read in order.
- A truncated or wrongly tagged block returns none.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the block parser.

## Impact

- `exa_assets::morph_controller`, from `parse_morph_block`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 8% to 11%.
- Evidence: the 412-byte `0x1000de470`: the tag `0xedc0c589` at `+0`, the unknown word, the first count at `+8`, its points at `+0xc`, the owner-point skip, the second count, the `0x4c` entries at `0x50`, each with pointers at `+0x18` and `+0x10`. No new corpus files.
- Out of scope: the controller's later apply calls.

## Phase

Phase 5, characters. The morph block parser.

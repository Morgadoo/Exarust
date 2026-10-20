# Proposal

## Why

The morph controller merges up to four source ids and weights into its own four slots: an existing id adds the scaled weight, a new id appends while a slot is free. Exarust has the blend and part walk but not the weight merge.

## What Changes

- A scale below the epsilon does nothing.
- Up to four source entries are read; the first id below one stops the merge.
- An id already present adds `weight × scale` to its slot.
- A new id appends to the highest used slot plus one when that leaves room.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the weight merge.

## Impact

- `exa_assets::morph_controller`, from `merge_morph_weights`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` moves from 6% to 7%.
- Evidence: the 228-byte `0x1000dbcd0`: the epsilon `0x2ab6c0`, the highest positive slot at `0x0`, the four source entries at `0x0`, the slot weights at `0x10`, the id compare, the scaled add and the append bounded at slot 3. No new corpus files.
- Out of scope: the caller.

## Phase

Phase 5, characters. The morph weight merge.

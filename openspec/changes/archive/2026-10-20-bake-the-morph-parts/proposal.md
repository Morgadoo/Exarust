# Proposal

## Why

The last morph pass finalises the imported parts: it gathers child positions by the two digits a node's name begins with, sorts the parts by their ordinal, makes every delta relative to the model, and folds three special parts onto part 5. Its gather helper is a 680-byte routine. Exarust has the cook and evaluators but not this bake.

## What Changes

- The gather indexes each child by `(first − '0')·10 + (second − '0')`, zero-fills gaps and reports the largest index.
- The bake selection-sorts the parts ascending by ordinal from a starting best of 100.
- Each delta becomes the part's position minus the model's position.
- With more than nine parts, part 5's vertices and deltas are subtracted from parts 6 through 8.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the part bake.

## Impact

- `exa_assets::morph_controller`, from `morph_child_index`, `gather_morph_children`, `BakePart` and `bake_morph_parts`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **62%** (10,563 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the gather helper (the node's child array, the two name digits less `'0'`, the 12-byte position copies) and the bake (the model gather, the per-part gather, the selection with the `0x64` start, the descriptor-to-`atoi` key, the relative subtraction, the `P>9` guard and the part-5 reference for 6 through 8). No new corpus files.
- Out of scope: the engine's name-string conversion and the reallocator.

## Phase

Phase 3, bodies. The morph part bake.

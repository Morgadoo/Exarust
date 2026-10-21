# Proposal

## Why

The morph controller's remap rebuild composes its vertex permutation and gathers every part's bytes through it, then refreshes the owner's arrays and indices. Exarust has the controller's setup, start, basis and apply, but not the rebuild.

## What Changes

- The map becomes `input[map[i]]`.
- Each part whose flags are clear is gathered by `input[j]` over its stride and count; a part with flags set is left alone.
- The arrays, indices and vertices are always rebuilt.
- With bit 0 set the rebuild also purges, marks the owner dirty, stamps the version and re-binds the bound object when there is one.

## Capabilities

### New Capabilities

### Modified Capabilities

- `morph-controller`: the remap rebuild.

## Impact

- `exa_assets::morph_controller`, from `rebuild_morph_remap`, `morph_element_size` and `morph_stride`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `RF_Morphing` becomes **27%** (4,686 of 17,099 bytes). The published headline stays 14.4% and 5.7%; the fresh sum stays about 15.5%.
- Evidence: the remap rebuild: the map count, the index block, the composition loop, the part array (0x18 bytes a part: flags, kind, count and bytes), the last part index, the stride helper (bits 20 through 23 give 8, 1 or 4 bytes, any other code 0; bits 16 through 19 give the component count), the flag-0 refresh calls (purge, arrays, index table, vertex data, dirty mark), the dirty stamp and the re-bind pair. No new corpus files.
- Out of scope: the 1,128-byte frame/apply routine and the 4,642-byte core.

## Phase

Phase 3, bodies. The morph remap rebuild.

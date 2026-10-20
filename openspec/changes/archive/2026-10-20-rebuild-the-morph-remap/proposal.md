# Proposal

## Why

The morph controller's remap rebuild (`0x1000da330`) composes its vertex permutation and gathers every part's bytes through it, then refreshes the owner's arrays and indices. Exarust has the controller's setup, start, basis and apply, but not the rebuild.

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
- Evidence: `0x1000da330`: the map count at `this+0x288`, the index block at `this+0x270`, the composition loop, the part array at `this+0x280` (0x18 bytes a part; flags at `+8`, kind at `+4`, count at `+0xc`, bytes at `+0x10`), the last part index at `this+0x278`, the stride helper `0x100066630` (bits 20 through 23 give 8, 1 or 4 bytes, any other code 0; bits 16 through 19 give the component count), the flag-0 refresh calls `0x1000640c0`, `0x100063e90`, `0x100067380`, `0x100068730` and `0x100051110`, the `0xe4` stamp and the `0x2e0` re-bind pair. No new corpus files.
- Out of scope: the 1,128-byte frame/apply routine `0x1000ddc40` and the 4,642-byte core.

## Phase

Phase 3, bodies. The morph remap rebuild.

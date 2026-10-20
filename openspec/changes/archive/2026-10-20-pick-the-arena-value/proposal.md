# Proposal

## Why

The arena picks one of three values by counting the positive entries, advancing a global seed with the `0x08088405` generator, scaling the seed by the count and indexing the table. Exarust has the arena record log but not the pick.

## What Changes

- The count is the number of positive entries among three.
- The seed becomes `seed × 0x08088405 + 1`.
- The index is the high half of `seed × count`.
- The value at that index is returned with the new seed.

## Capabilities

### New Capabilities

### Modified Capabilities

- `arena-records`: the weighted pick.

## Impact

- `exa_game::arena_log`, from `pick_arena_value`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `XA_Arena` stays about 34%.
- Evidence: the 138-byte `0x100184250`: the three dwords at `+8`, `+0xc` and `+0x10` counted with `jbe`, the seed `0x498590` multiplied by `0x8088405` and incremented, the high half of seed × count as the index, and the value read at `+8 + index × 4`. No new corpus files.
- Out of scope: the consumer call.

## Phase

Phase 6, game modes. The arena pick.

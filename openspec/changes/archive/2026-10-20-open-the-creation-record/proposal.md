# Proposal

## Why

The creation record opens by clearing five words and its tail, then fills three fixed eight-byte slots. Exarust has the later creation behaviours but not the opening.

## What Changes

- The five words at `0x64` through `0x84` and the tail at `0xf0` are zero, and the reset runs with size 8.
- Three slots hold the low word `0x61A8`, bytes 1 through 4, and a final byte of 8, 5 and 6.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the record opening.

## Impact

- `exa_game::create`, from `reset_creation_record` and `creation_slot_defaults`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 17% to 18%.
- Evidence: the 84-byte reset: its six clears and the record reset with size 8. The 99-byte slot seed: the three slots with `0x61a8` and the tail bytes 8, 5 and 6. No new corpus files.
- Out of scope: the reset body.

## Phase

Phase 5, character creation. The record opening.

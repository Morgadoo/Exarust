# Proposal

## Why

The creation screen clears six eight-byte slots: it finds the first used slot, clears it and empties the slots after it. Exarust has the record move and labels but not the slot clear.

## What Changes

- The first slot whose low word is non-zero is found and returned.
- Its low word and bytes 3 through 7 are zeroed; byte 2 is left.
- Every slot after it, up to the fifth, is zeroed whole.
- With no non-zero word nothing changes and none is returned.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the slots' clear.

## Impact

- `exa_game::create`, from `clear_creation_slots`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` stays about 2% (about 2.2%).
- Evidence: the 134-byte slot clear: the six slots with an eight-byte stride, the low-word test, the byte clear at offsets 0, 1 and 3 through 7, the loop bound at 5 and the memset of the later slots. No new corpus files.
- Out of scope: what the slots hold.

## Phase

Phase 5, character creation. The slots' clear.

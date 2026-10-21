# Proposal

## Why

A character creation record moves its appearance into a new record: four words, a count, a parts list, a tail and a second list. Exarust has no character-creation record.

## What Changes

- Moving copies the four words, the count, the tail and both lists into the destination.
- The two source lists are taken, and the source's count and tail are zeroed.
- The source's four words stay.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the appearance record's move.

## Impact

- `exa_game::create`, from `move_appearance`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 1% to 2%.
- Evidence: the 228-byte record move: the four qword copies, the dword, the first list's move and reset, the second dword and the second list's move and reset. No new corpus files.
- Out of scope: the constructors that own the lists.

## Phase

Phase 5, character creation. The appearance record's move.

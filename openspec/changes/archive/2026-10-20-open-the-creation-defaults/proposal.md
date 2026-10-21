# Proposal

## Why

The creation object opens with two fixed option entries, a count, a stride, a value and a flags word. Exarust has several creation behaviours but not the opening.

## What Changes

- 40 bytes at `0x198` are cleared.
- The count is 2 and the stride 16; two zeroed entries are made.
- The first entry gets `0x101` and `0x5FFF`, then `0x100000` is ORed into its first word.
- The second gets `0x201` and `0x5FFF`.
- The first and second entry words are stored, then 4.0 and `0x510000`.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the opening defaults.

## Impact

- `exa_game::create`, from `creation_defaults`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 9% to 10%.
- Evidence: the 209-byte defaults: the 40-byte clear, the count, the 16-byte allocation, the `0x101`/`0x5FFF` and `0x201`/`0x5FFF` entries, the `0x100000` OR, the two stored words, the 4.0 constant and the `0x510000` word. No new corpus files.
- Out of scope: the allocation routine.

## Phase

Phase 5, character creation. The opening defaults.

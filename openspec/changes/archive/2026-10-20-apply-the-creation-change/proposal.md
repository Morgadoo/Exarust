# Proposal

## Why

When the creation screen's sex option changes, it sets or clears bit 0 of the record word, takes the appearance label and refreshes the child, the global and two passes before the callback and its finalizer. Exarust has the label but not the change handler.

## What Changes

- A disabled change only finalizes.
- Otherwise bit 0 of the word is set for female and cleared otherwise, and the label is taken.
- The text set runs, the child call runs when the child object is present, the global call runs when the global flag is clear, both passes run and the callback runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the change handler.

## Impact

- `exa_game::create`, from `apply_creation_change`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 3% to 4%.
- Evidence: the 218-byte `0x10012d870`: the `0x40` guard on `0x1c`, the `0x2 == 2` sex test, bit 0 of the `0x158` word, the `0x10012d7a0` label call, `0x100008210`, `0x10012d650`, the `0x1002b3d60` global guard, `0x1000cf330`, the `0x1e5` flag around `0x10012cc50` and `0x10012cdf0`, the slot `0x1b8` callback and the `0x10012d850` finalizer. No new corpus files.
- Out of scope: the text, passes, callback and finalizer bodies.

## Phase

Phase 5, character creation. The change handler.

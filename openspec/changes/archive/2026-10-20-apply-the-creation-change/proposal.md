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
- Evidence: the 218-byte change handler: the `0x40` guard on the object's flag word, the sex test on the option word, bit 0 of the child's record word, the appearance-label call, the text set, the view refresh, the global guard and its update, the busy flag around the two rebuilds, the slot callback and the finalizer. No new corpus files.
- Out of scope: the text, passes, callback and finalizer bodies.

## Phase

Phase 5, character creation. The change handler.

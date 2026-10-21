# Proposal

## Why

The creation screen also copies a raw appearance value to its target, with the same two-unit redraw debounce as the settle. Exarust has the settle but not the copy.

## What Changes

- The copy stores the source at the target unless their distance is under the epsilon `0x33ffffd7`.
- A last stamp plus 2 above the clock sets the pending flag.
- Otherwise the stamp takes the clock and the callback runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the appearance copy.

## Impact

- `exa_game::create`, from `copy_appearance`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` stays about 3% (about 3.4%).
- Evidence: the 135-byte appearance copy: the source, the child's record target, the epsilon, the stamp, the pending byte, the two-unit guard and the slot callback. No new corpus files.
- Out of scope: the callback.

## Phase

Phase 5, character creation. The appearance copy.

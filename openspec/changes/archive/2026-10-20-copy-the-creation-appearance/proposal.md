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
- Evidence: the 135-byte `0x10012d9f0`: the source `0x38`, the target `0x158 + 0x38`, the epsilon at `0x1002ab6c0`, the stamp `0x1e0`, the pending byte `0x1e4`, the two-unit guard and the callback at slot `0x1b8`. No new corpus files.
- Out of scope: the callback.

## Phase

Phase 5, character creation. The appearance copy.

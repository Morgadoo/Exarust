# Proposal

## Why

The creation screen settles an appearance value toward a target and debounces the redraw callback by two clock units. Exarust has the record move, labels and slot clear but not the settle.

## What Changes

- The value is `(0.5 − source) × 2`.
- A distance from the target under the epsilon `0x33ffffd7` stores nothing.
- Otherwise the value is stored; a last stamp plus 2 above the clock sets the pending flag, and otherwise the stamp takes the clock and the callback runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `character-creation`: the appearance settle.

## Impact

- `exa_game::create`, from `settle_appearance`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `SG_Character_Create` moves from 2% to 3%.
- Evidence: the 154-byte appearance settle: `0.5`, `2.0`, the epsilon, the source, the child's record target, the stamp, the pending byte, the two-unit guard and the clock. No new corpus files.
- Out of scope: the callback at slot `0x1b8`.

## Phase

Phase 5, character creation. The appearance settle.

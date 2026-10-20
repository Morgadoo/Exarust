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
- Evidence: the 154-byte `0x10012d950`: `0.5` at `0x100315f00`, `2.0` at `0x100315f08`, the epsilon at `0x1002ab6c0`, the source `0x38`, the target `0x158 + 0x34`, the stamp `0x1e0`, the pending byte `0x1e4`, the two-unit guard and the clock `0x1004997b0`. No new corpus files.
- Out of scope: the callback at slot `0x1b8`.

## Phase

Phase 5, character creation. The appearance settle.

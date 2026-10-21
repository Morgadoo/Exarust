# Proposal

## Why

The 775-byte bevel method is slot 27 of `TGUIDrawSkin` and is inherited unchanged by `TGSLines`. `draw_bevel` already implements it, but the coverage ledger credited the line class, which does not own the method.

## What Changes

- Add `draw_skin_bevel`, which takes the parent method's two boolean arms rather than a state word, and keep `draw_bevel` as the shared implementation.
- Record the bevel against `TGUIDrawSkin`, where the method is owned, and leave `TGSLines` at its own overrides.
- State the arm precedence: the first boolean wins, else the second, else idle.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the parent bevel's arm inputs and its owner.

## Impact

- `exa_ui::skin`, from `draw_skin_bevel`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the parent's slot 27 address and the `TGSLines` slot table that inherits it. No new corpus files.
- Out of scope: the two boolean callers.

## Phase

Phase 7, the interface. The attribution, not new pixels.

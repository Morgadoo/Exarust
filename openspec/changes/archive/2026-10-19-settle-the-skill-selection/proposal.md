# Proposal

## Why

The skill selector stores the low 16 bits of its selection word, clamps above eight to zero, and on page 1 sets or clears bits 0x110 on the object. Exarust does not model it.

## What Changes

- The selection index is the low 16 bits, clamped to 0 above 8.
- On page 1 a positive index sets bits 0x110 and a zero index clears them.
- Any other page leaves the flags unchanged.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill selection.

## Impact

- `exa_ui::menu`, from `settle_skill_selection`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`.
- Evidence: the 108-byte selector method's mask, clamp and flag edits. No new corpus files.
- Out of scope: the base handler it calls first.

## Phase

Phase 7, the interface. The clamp and flags.

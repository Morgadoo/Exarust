# Proposal

## Why

`TSkillUI` places a skill node from its index, or centres it in the view when the index is not positive. Exarust has no `TSkillUI` behaviour.

## What Changes

- A positive index takes the layout point at the row in bits 8 through 15 and the column in the low 8 bits, rounded to the nearest even integer.
- A non-positive index takes the base position when the scaled flag is set, otherwise half the bounds' width and height.
- The centre rounds toward zero, as the original's sign-corrected shift does.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's node position.

## Impact

- `exa_ui::skill`, from `skill_position`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 0% to 1%.
- Evidence: the 186-byte node placement: the positive-index virtual, the row and column masks, `cvtss2si`, the scaled branch to the base method, and the form's half-width and half-height fields. No new corpus files.
- Out of scope: the layout virtual and the base method.

## Phase

Phase 4, UI. The skill screen's node position.

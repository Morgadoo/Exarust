# Proposal

## Why

The skill screen hit-tests a point against the form, the page-2 bottom band, the high node's left and right halves and the low node's nine horizontal slots. Exarust has the pointer input and node hit test but not the point hit.

## What Changes

- State bit 1 with a page below 2 returns 0; a foreign form or a refused base returns the base result.
- On page 2 a point in the bottom band returns `0x10000`, and a state byte of 4 returns the base result.
- A high node gives 1 or 2 for a horizontal offset between 13/4 units left or right within 4 units vertically.
- A low node gives the rounded slot from `4.5 + offset/10 units` when it is 1 through 8.
- Without selections the fallback runs.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill point hit test.

## Impact

- `exa_ui::skill`, from `hit_skill_point`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 20% to 25%.
- Evidence: the 857-byte point hit: the state and page, the base hit, the ownership check, the guard, the transform, the page-2 band from 16/8 units vertically and 6 units horizontally, `0x10000`, the state byte 4 return, the high and low nodes, the layout slot, the radius test with 41 units, the offsets 4 and 13 units, the 8-unit low band, the `4.5` over 10 units, and the fallback slot. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 4, UI. The skill point hit test.

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
- Evidence: the 857-byte `0x100225130`: the state `0x158`, the page `0x1c0`, the base `0x10002d270`, the ownership check, the guard `0x100028a20`, the transform `0x10002ce40`, the page-2 band from 16/8 units vertically and 6 units horizontally, `0x10000`, the state byte 4 return, the high `0x184` and low `0x180` nodes, the layout slot `0x1c8`, the radius `0x1002275a0` with 41 units, the offsets 4 and 13 units, the 8-unit low band, the `4.5` at `0x10033acf0` over 10 units, and the fallback slot `0x1c0`. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 4, UI. The skill point hit test.

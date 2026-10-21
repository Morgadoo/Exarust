# Proposal

## Why

The skill screen's input handler routes a point and mask to the base, clears or animates the screen, repages, selects a node and writes a grid slot. Exarust has the key handler but not the pointer input.

## What Changes

- An index below −1 goes to the base.
- Mask bit 1 leaves on a positive page; a closing word returns or finishes; otherwise the screen clears when state bit 1 is clear and returns 1.
- Index 0 with a selection hides for 200; `0x10000` sets the show deadline or the second deadline.
- A selected index repages, fills the grid or writes a slot, then hints and hides.
- The result is 16 unless the mask path returns 1 or the base replaces it.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's pointer input.

## Impact

- `exa_ui::skill`, from `handle_skill_input`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 16% to 20%.
- Evidence: the 826-byte click handler: the hit index, the point and its transform, mask bit 1, the page and closing words, the state word and the clear, the `0x10000` branch with its +500, the busy flag, the two selections, the repage, the fill, the hint, the two hides, the select helpers, the slot and the base handler. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 4, UI. The skill screen's pointer input.

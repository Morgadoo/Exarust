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
- Evidence: the 826-byte `0x1002241a0`: the index `0x6c`, the point `0x18`/`0x1c`, the transform `0x10002cd90`/`0x10002ce40`, mask bit 1, the page `0x1c0`, the closing `0x1c4`, the state `0x158`, the clear `0x10022a380`, the `0x10000` branch with `0x160`+500 and `0x164`, the flag `0x1ea`, the selections `0x180`/`0x184`, the repage `0x100223e30`, the fill `0x100223ac0`, the hint `0x100028a10`, the hides `0x159`/`0x15c`, the select `0x100112800`/`0x100223f00`, the slot at `0xc0` and the base `0x10002dbc0`. No new corpus files.
- Out of scope: the called bodies.

## Phase

Phase 4, UI. The skill screen's pointer input.

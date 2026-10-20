# Proposal

## Why

The skill screen's key handler moves the selection over the grid, routes base and hint keys, repages on key `0x100` and then runs the closing, cursor and clear steps. Exarust has the screen's hover, lists and hit test but not the key handler.

## What Changes

- Bit 0 moves the index: with a high selection back one, or forward while below two; otherwise a grid scan back from the index less two or forward from the index less one for the first zero.
- A positive moved index different from the old is published.
- A key with bit `0x100` calls the base; with bit `0x200` it hints, hides for 200 when not hidden and clears bit `0x40`.
- Key `0x100` stores the high slot, or the low slot plus the scanned index, publishes, hints, hides for 300 and sets bit `0x40`, then runs the closing, cursor and clear steps.
- The result is 0.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's key handler.

## Impact

- `exa_ui::skill`, from `skill_screen_key`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 12% to 16%.
- Evidence: the 864-byte `0x100224b90`: the `0x180`/`0x184` selections, the `0x6c` index, the `0xf001` masks `0x1001`/`0x2001`, the grid `0x198`, the publish slot `0x128`, the base `0x100224090`, the hint `0x100028a10`, the `0x159` hide and `0x15c` deadline, bit `0x40` of `0x60`, the page `0x1c0`, the scan `0x100223ac0`, the repage `0x100223e30`, the closing `0x1c4`, the cursor slot `0x130`, the transform `0x100019800`, the hit slot `0x158` and the clear `0x10022a380`. No new corpus files.
- Out of scope: the base, hint, cursor, hit and clear bodies.

## Phase

Phase 4, UI. The skill screen's key handler.

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
- Evidence: the 864-byte key handler: the two selections, the index, the `0xf001` masks `0x1001`/`0x2001`, the grid, the publish slot, the base, the hint, the hide and its deadline, bit `0x40` of the style word, the page, the scan, the repage, the closing word, the cursor slot, the transform, the hit slot and the clear. No new corpus files.
- Out of scope: the base, hint, cursor, hit and clear bodies.

## Phase

Phase 4, UI. The skill screen's key handler.

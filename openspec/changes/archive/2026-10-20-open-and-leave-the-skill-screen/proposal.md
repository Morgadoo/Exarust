# Proposal

## Why

Showing the skill screen marks it for redraw and stamps a deadline; leaving it either calls the base, waits out the page and its transition, or runs the clear call. Exarust has the node position only.

## What Changes

- Showing runs the relayout when the screen's open flag is set, sets bit 1 of the state and stores the clock plus the delay.
- Leaving calls the base when the closing flag is set; a non-zero page does nothing.
- With bit 0 set and a deadline more than 400 above the clock, leaving does nothing; with bit 1 set it does nothing; otherwise it runs the clear call.

## Capabilities

### New Capabilities

### Modified Capabilities

- `user-interface`: the skill screen's show and leave.

## Impact

- `exa_ui::skill`, from `show_skill_ui` and `leave_skill_ui`.
- `docs/PLAN.md` and `docs/COMPONENT-MAP.md`: `TSkillUI` moves from 1% to 2%.
- Evidence: the 63-byte `0x100224ef0` and the 116-byte `0x1002244f0`: the open bit `0x20` of field `0x60` and its call to `0x10022c950`, the state field `0x158`, the deadline `0x164`, the closing field `0x1c4`, the page `0x1c0`, the deadline `0x160`, the `0x190` subtraction, the clock `0x1004997d0` and the clear call `0x10022a380`. The delay global `0x1002ad5b0` is zero in the file and is a parameter. No new corpus files.
- Out of scope: the relayout, base and clear routines.

## Phase

Phase 4, UI. The skill screen's show and leave.
